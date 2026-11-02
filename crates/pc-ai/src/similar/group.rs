//! Gruppi dagli archi confermati con SSIM e dalle correzioni dell'utente.
//!
//! 1. Vincoli `must` (unisci) e `cannot` (separa): tra due correzioni in conflitto vale la più
//!    recente. I `must` si applicano dal più recente; un `must` viene saltato se unirebbe due foto
//!    separate da un `cannot` più recente.
//! 2. Archi automatici sopra la soglia, saltando quelli che unirebbero foto separate da un
//!    `cannot` ancora valido o appartenenti allo stesso gruppo sciolto.
//! 3. Componenti oltre [`MAX_COMPONENT`] foto: divisi con collegamento completo sulla matrice
//!    SSIM interna, per evitare catene A≈B≈C con A e C diversi.

use pc_store::model::MediaId;
use std::collections::{HashMap, HashSet};

/// Componenti più grandi di così vengono divisi con collegamento completo.
pub const MAX_COMPONENT: usize = 50;

/// Correzione dell'utente tra due foto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Link {
    pub a: MediaId,
    pub b: MediaId,
    pub must: bool,
    /// Istante della correzione (ms): tra due correzioni in conflitto vale la più recente.
    pub at: i64,
}

#[derive(Default, Debug, Clone)]
pub struct Constraints {
    pub links: Vec<Link>,
    /// Gruppi sciolti: foto → token; foto con un token in comune non si uniscono più da sole.
    pub dissolved: HashMap<MediaId, Vec<i64>>,
}

impl Constraints {
    fn same_dissolved(&self, a: MediaId, b: MediaId) -> bool {
        match (self.dissolved.get(&a), self.dissolved.get(&b)) {
            (Some(x), Some(y)) => x.iter().any(|t| y.contains(t)),
            _ => false,
        }
    }
}

struct Dsu {
    parent: HashMap<MediaId, MediaId>,
    members: HashMap<MediaId, Vec<MediaId>>,
}

impl Dsu {
    fn new() -> Self {
        Self {
            parent: HashMap::new(),
            members: HashMap::new(),
        }
    }
    fn find(&mut self, x: MediaId) -> MediaId {
        let p = *self.parent.entry(x).or_insert(x);
        if p == x {
            self.members.entry(x).or_insert_with(|| vec![x]);
            return x;
        }
        let r = self.find(p);
        self.parent.insert(x, r);
        r
    }
    fn union(&mut self, a: MediaId, b: MediaId) -> bool {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra == rb {
            return false;
        }
        let (big, small) = if self.members[&ra].len() >= self.members[&rb].len() {
            (ra, rb)
        } else {
            (rb, ra)
        };
        let moved = self.members.remove(&small).unwrap_or_default();
        self.members.get_mut(&big).expect("radice").extend(moved);
        self.parent.insert(small, big);
        true
    }
    fn members_of(&mut self, x: MediaId) -> Vec<MediaId> {
        let r = self.find(x);
        self.members[&r].clone()
    }
}

/// Coppia ordinata.
pub fn pair(a: MediaId, b: MediaId) -> (MediaId, MediaId) {
    if a < b {
        (a, b)
    } else {
        (b, a)
    }
}

/// Gruppi (almeno 2 foto) dagli archi `(a, b, ssim)` e dalla soglia.
pub fn build_groups(
    edges: &[(MediaId, MediaId, f32)],
    threshold: f32,
    constraints: &Constraints,
) -> Vec<Vec<MediaId>> {
    let mut dsu = Dsu::new();
    let mut links = constraints.links.clone();
    links.sort_by_key(|l| std::cmp::Reverse(l.at));
    // `cannot` ancora valido: non superato da un `must` più recente che unisce le stesse foto.
    let cannots: Vec<Link> = links.iter().copied().filter(|l| !l.must).collect();
    for m in links.iter().filter(|l| l.must) {
        // Un `must` più vecchio di un `cannot` che verrebbe violato si salta.
        let mut a_side = dsu.members_of(m.a);
        let b_side = dsu.members_of(m.b);
        if a_side.contains(&m.b) {
            continue;
        }
        a_side.sort_unstable();
        let violates = cannots.iter().any(|c| {
            c.at > m.at
                && ((a_side.binary_search(&c.a).is_ok() && b_side.contains(&c.b))
                    || (a_side.binary_search(&c.b).is_ok() && b_side.contains(&c.a)))
        });
        if !violates {
            dsu.union(m.a, m.b);
        }
    }
    // `cannot` superati: le due foto sono già unite da `must` più recenti.
    let mut active: HashMap<MediaId, HashSet<MediaId>> = HashMap::new();
    for c in &cannots {
        if dsu.find(c.a) != dsu.find(c.b) {
            active.entry(c.a).or_default().insert(c.b);
            active.entry(c.b).or_default().insert(c.a);
        }
    }
    let mut strong: Vec<&(MediaId, MediaId, f32)> =
        edges.iter().filter(|e| e.2 >= threshold).collect();
    strong.sort_by(|x, y| y.2.total_cmp(&x.2));
    for &&(a, b, _) in &strong {
        if constraints.same_dissolved(a, b) {
            continue;
        }
        let (ra, rb) = (dsu.find(a), dsu.find(b));
        if ra == rb {
            continue;
        }
        let (ma, mb) = (dsu.members_of(a), dsu.members_of(b));
        let blocked = ma.iter().any(|x| {
            active
                .get(x)
                .is_some_and(|set| mb.iter().any(|y| set.contains(y)))
        });
        if !blocked {
            dsu.union(a, b);
        }
    }
    let ssim: HashMap<(MediaId, MediaId), f32> =
        edges.iter().map(|&(a, b, s)| (pair(a, b), s)).collect();
    let musts: HashSet<(MediaId, MediaId)> = links
        .iter()
        .filter(|l| l.must)
        .map(|l| pair(l.a, l.b))
        .collect();
    let mut out = Vec::new();
    let roots: Vec<MediaId> = dsu.members.keys().copied().collect();
    for r in roots {
        let mut members = dsu.members[&r].clone();
        if members.len() < 2 {
            continue;
        }
        members.sort_unstable();
        if members.len() > MAX_COMPONENT {
            out.extend(split_complete_linkage(&members, &ssim, &musts, threshold));
        } else {
            out.push(members);
        }
    }
    out.sort();
    out
}

/// Collegamento completo: si uniscono due cluster solo se tutte le coppie tra loro sono sopra
/// soglia (le unioni manuali contano come somiglianza 1).
fn split_complete_linkage(
    members: &[MediaId],
    ssim: &HashMap<(MediaId, MediaId), f32>,
    musts: &HashSet<(MediaId, MediaId)>,
    threshold: f32,
) -> Vec<Vec<MediaId>> {
    let sim = |a: MediaId, b: MediaId| {
        let p = pair(a, b);
        if musts.contains(&p) {
            1.0
        } else {
            ssim.get(&p).copied().unwrap_or(0.0)
        }
    };
    let mut clusters: Vec<Vec<MediaId>> = members.iter().map(|m| vec![*m]).collect();
    loop {
        let mut best: Option<(usize, usize, f32)> = None;
        for i in 0..clusters.len() {
            for j in i + 1..clusters.len() {
                let link = clusters[i]
                    .iter()
                    .flat_map(|a| clusters[j].iter().map(move |b| (*a, *b)))
                    .map(|(a, b)| sim(a, b))
                    .fold(f32::INFINITY, f32::min);
                if link >= threshold && best.is_none_or(|(_, _, s)| link > s) {
                    best = Some((i, j, link));
                }
            }
        }
        let Some((i, j, _)) = best else { break };
        let moved = clusters.remove(j);
        clusters[i].extend(moved);
    }
    clusters
        .into_iter()
        .filter(|c| c.len() > 1)
        .map(|mut c| {
            c.sort_unstable();
            c
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link(a: MediaId, b: MediaId, must: bool, at: i64) -> Link {
        Link { a, b, must, at }
    }

    #[test]
    fn edges_above_threshold_form_groups() {
        let e = vec![(1, 2, 0.9), (2, 3, 0.85), (4, 5, 0.7), (6, 7, 0.95)];
        let g = build_groups(&e, 0.8, &Constraints::default());
        assert_eq!(g, vec![vec![1, 2, 3], vec![6, 7]]);
        // Soglia più bassa: entra anche 4–5.
        assert_eq!(build_groups(&e, 0.6, &Constraints::default()).len(), 3);
    }

    #[test]
    fn cannot_removes_a_photo_and_must_joins_photos() {
        let e = vec![(1, 2, 0.9), (2, 3, 0.9), (1, 3, 0.9)];
        let c = Constraints {
            links: vec![
                link(1, 3, false, 10),
                link(2, 3, false, 10),
                link(7, 8, true, 11),
                link(8, 9, true, 11),
            ],
            ..Default::default()
        };
        assert_eq!(build_groups(&e, 0.8, &c), vec![vec![1, 2], vec![7, 8, 9]]);
    }

    #[test]
    fn dissolved_group_does_not_reform_automatically() {
        let e = vec![(1, 2, 0.9), (2, 3, 0.9)];
        let c = Constraints {
            dissolved: [(1, vec![5]), (2, vec![5]), (3, vec![5])].into(),
            ..Default::default()
        };
        assert!(build_groups(&e, 0.8, &c).is_empty());
    }

    #[test]
    fn most_recent_correction_wins_a_conflict() {
        // A must B, B must C, poi A cannot C (più recente): la catena si interrompe.
        let c = Constraints {
            links: vec![
                link(1, 2, true, 1),
                link(2, 3, true, 2),
                link(1, 3, false, 3),
            ],
            ..Default::default()
        };
        let g = build_groups(&[], 0.8, &c);
        assert_eq!(g, vec![vec![2, 3]], "il must più vecchio cede");
        // Al contrario: il cannot è vecchio, il must più recente lo supera.
        let c = Constraints {
            links: vec![
                link(1, 3, false, 1),
                link(1, 2, true, 2),
                link(2, 3, true, 3),
            ],
            ..Default::default()
        };
        assert_eq!(build_groups(&[], 0.8, &c), vec![vec![1, 2, 3]]);
    }

    #[test]
    fn long_chains_are_split_with_complete_linkage() {
        // Catena di 60 foto: ogni foto è simile solo alle vicine.
        let mut e = Vec::new();
        for i in 1..60 {
            e.push((i, i + 1, 0.9));
        }
        // Due blocchi compatti in fondo alla catena.
        for a in 100..106 {
            for b in a + 1..106 {
                e.push((a, b, 0.95));
            }
        }
        let g = build_groups(&e, 0.8, &Constraints::default());
        assert!(g.iter().all(|c| c.len() <= 2 || c[0] >= 100), "{g:?}");
        assert!(g.contains(&(100..106).collect::<Vec<_>>()));
    }
}
