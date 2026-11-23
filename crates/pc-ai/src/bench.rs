//! Banco di prova del raggruppamento dei volti (design D9).
//!
//! - Dati etichettati: sintetici ([`synthetic`]: identità come nuvole sull'ipersfera a 128
//!   dimensioni, con frammenti lontani nel tempo, gemelli nella stessa foto, volti ponte e
//!   volti di qualità bassa) o un file JSONL di embedding anonimi ([`read_jsonl`], esportato
//!   dalla libreria dell'utente con [`export_named`]).
//! - Metriche ([`Metrics`]): F a coppie, BCubed F (la metrica di IJB-B: ogni volto pesa uguale),
//!   gruppi per identità, quota di gruppi da un volto, precisione delle unioni automatiche.
//! - Il raggruppamento di prima ([`legacy_cluster`]) e quello nuovo ([`run_new`]) sugli
//!   stessi dati, e l'utente perfetto che risponde alla coda ([`questions_to_reach`]).
//!
//! Nessuna dipendenza nuova: il generatore pseudo-casuale è uno SplitMix64.

use crate::aggregate::{self, Face, Label, Mode, Outcome, Params, People, Question, QuestionKind};
use hnsw_rs::prelude::*;
use pc_jobs::JobControl;
use pc_store::people::cosine;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{BufRead, Write};

/// Generatore pseudo-casuale SplitMix64 (riproducibile, senza dipendenze).
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniforme in [0, 1).
    pub fn f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    /// Normale standard (Box-Muller).
    pub fn gauss(&mut self) -> f64 {
        let u1 = self.f64().max(1e-300);
        let u2 = self.f64();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.f64() * n as f64) as usize % n.max(1)
    }
}

/// Un volto etichettato. È anche il formato di una riga del file JSONL: solo id anonimi,
/// qualità ed embedding (l'istante esiste solo nei dati sintetici).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sample {
    pub face: i64,
    /// Identità (anonima); `None` = volto senza etichetta, escluso dalle metriche.
    pub identity: Option<i64>,
    pub photo: i64,
    pub quality: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<i64>,
    pub embedding: Vec<f32>,
}

/// Parametri dei dati sintetici.
#[derive(Debug, Clone)]
pub struct SynthConfig {
    pub seed: u64,
    pub identities: usize,
    /// Volti dell'identità più grande; le altre seguono una legge di Zipf.
    pub largest: usize,
    pub zipf: f64,
    /// Coppie di gemelli (identità molto simili, spesso nella stessa foto).
    pub twins: usize,
    /// Volti ponte: a metà tra due identità.
    pub bridges: usize,
    /// Quota di volti di qualità bassa (più rumore).
    pub low_quality: f64,
    /// Quota di volti in foto di gruppo.
    pub group_photos: f64,
    /// Coseno medio tra i centri di due identità: gli embedding reali non sono isotropi
    /// (sulla libreria di prova i volti della stessa foto hanno coseno mediano 0,14).
    pub shared: f32,
    /// Rumore per dimensione di un volto di qualità 1.
    pub noise: f64,
    /// Coseno tra il centro di un'identità e i suoi frammenti lontani nel tempo (min, max).
    pub far: (f32, f32),
}

impl Default for SynthConfig {
    fn default() -> Self {
        Self {
            seed: 42,
            identities: 120,
            largest: 160,
            zipf: 0.9,
            twins: 3,
            bridges: 8,
            low_quality: 0.12,
            group_photos: 0.35,
            shared: 0.25,
            noise: 0.068,
            far: (0.78, 0.9),
        }
    }
}

const DIM: usize = 128;
const YEAR_MS: i64 = 365 * 24 * 60 * 60 * 1000;

fn normalize(v: &mut [f32]) {
    let n = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if n > 0.0 {
        v.iter_mut().for_each(|x| *x /= n);
    }
}

fn random_unit(rng: &mut Rng) -> Vec<f32> {
    let mut v: Vec<f32> = (0..DIM).map(|_| rng.gauss() as f32).collect();
    normalize(&mut v);
    v
}

/// Direzione a coseno `rho` da `c` (vettore unitario).
fn at_cosine(rng: &mut Rng, c: &[f32], rho: f32) -> Vec<f32> {
    let mut u = random_unit(rng);
    let d = cosine(&u, c);
    u.iter_mut().zip(c).for_each(|(x, y)| *x -= d * y);
    normalize(&mut u);
    let s = (1.0 - rho * rho).max(0.0).sqrt();
    let mut v: Vec<f32> = c.iter().zip(&u).map(|(a, b)| rho * a + s * b).collect();
    normalize(&mut v);
    v
}

fn face_from(rng: &mut Rng, mode: &[f32], noise: f64, quality: f32) -> Vec<f32> {
    let sigma = (noise / f64::from(quality).sqrt()) as f32;
    let mut v: Vec<f32> = mode
        .iter()
        .map(|x| x + sigma * rng.gauss() as f32)
        .collect();
    normalize(&mut v);
    v
}

/// Dati sintetici etichettati (design D9).
pub fn synthetic(cfg: &SynthConfig) -> Vec<Sample> {
    let mut rng = Rng::new(cfg.seed);
    // Centri delle identità: una componente comune a tutti i volti più una propria.
    let common = random_unit(&mut rng);
    let (a, b) = (cfg.shared.sqrt(), (1.0 - cfg.shared).max(0.0).sqrt());
    let mut centers: Vec<Vec<f32>> = (0..cfg.identities)
        .map(|_| {
            let own = random_unit(&mut rng);
            let mut v: Vec<f32> = common
                .iter()
                .zip(&own)
                .map(|(x, y)| a * x + b * y)
                .collect();
            normalize(&mut v);
            v
        })
        .collect();
    // Gemelli: identità vicine alle più grandi (coseno tra i centri 0,6).
    let mut twin_of: BTreeMap<usize, usize> = BTreeMap::new();
    for t in 0..cfg.twins.min(cfg.identities / 4) {
        let (a, b) = (2 + 2 * t, cfg.identities - 1 - t);
        centers[b] = at_cosine(&mut rng, &centers[a].clone(), 0.6);
        twin_of.insert(b, a);
    }
    let mut out: Vec<Sample> = Vec::new();
    for (k, center) in centers.iter().enumerate() {
        let size = ((cfg.largest as f64 / ((k + 1) as f64).powf(cfg.zipf)).round() as usize).max(1);
        // Modi: il principale e, per le identità con abbastanza foto, frammenti lontani nel
        // tempo (età, aspetto) a coseno 0,78–0,9 dal centro.
        let mut modes: Vec<(Vec<f32>, f64, i64)> = vec![(center.clone(), 0.6, 2015)];
        if size >= 6 {
            let extra =
                1 + usize::from(rng.f64() < 0.5) + usize::from(size >= 30 && rng.f64() < 0.4);
            for e in 0..extra {
                let rho = cfg.far.0 + (cfg.far.1 - cfg.far.0) * rng.f64() as f32;
                let year = 2015
                    + if e % 2 == 0 {
                        -4 - rng.below(4) as i64
                    } else {
                        4 + rng.below(4) as i64
                    };
                modes.push((at_cosine(&mut rng, center, rho), 0.4 / extra as f64, year));
            }
        }
        let total: f64 = modes.iter().map(|m| m.1).sum();
        for _ in 0..size {
            let mut pick = rng.f64() * total;
            let mut mi = 0;
            for (i, m) in modes.iter().enumerate() {
                if pick < m.1 {
                    mi = i;
                    break;
                }
                pick -= m.1;
                mi = i;
            }
            let low = rng.f64() < cfg.low_quality;
            let quality = if low {
                0.2 + 0.18 * rng.f64() as f32
            } else {
                0.4 + 0.6 * rng.f64() as f32
            };
            let (mode, _, year) = &modes[mi];
            let time = (*year - 1970) * YEAR_MS + (rng.f64() * YEAR_MS as f64) as i64;
            out.push(Sample {
                face: out.len() as i64 + 1,
                identity: Some(k as i64 + 1),
                photo: 0,
                quality,
                time: Some(time),
                embedding: face_from(&mut rng, mode, cfg.noise, quality),
            });
        }
    }
    // Volti ponte: di un'identità, ma a metà strada verso un'altra.
    let big: Vec<usize> = (0..cfg.identities.min(10)).collect();
    for b in 0..cfg.bridges {
        let (x, y) = (big[b % big.len()], big[(b * 3 + 1) % big.len()]);
        if x == y {
            continue;
        }
        let mut mid: Vec<f32> = centers[x]
            .iter()
            .zip(&centers[y])
            .map(|(p, q)| p + q)
            .collect();
        normalize(&mut mid);
        out.push(Sample {
            face: out.len() as i64 + 1,
            identity: Some(x as i64 + 1),
            photo: 0,
            quality: 0.8,
            time: Some((2015 - 1970) * YEAR_MS),
            embedding: face_from(&mut rng, &mid, cfg.noise, 0.8),
        });
    }
    // Foto: di gruppo (2–4 volti di identità diverse) o singole; i gemelli spesso insieme.
    let mut order: Vec<usize> = (0..out.len()).collect();
    for i in (1..order.len()).rev() {
        let j = rng.below(i + 1);
        order.swap(i, j);
    }
    let mut next_photo = 1i64;
    let mut open: Vec<(i64, Vec<i64>)> = Vec::new();
    let by_identity: HashMap<i64, Vec<usize>> =
        out.iter()
            .enumerate()
            .fold(HashMap::new(), |mut m, (i, s)| {
                m.entry(s.identity.unwrap()).or_default().push(i);
                m
            });
    for (b, a) in &twin_of {
        let (fb, fa) = (
            &by_identity[&(*b as i64 + 1)],
            &by_identity[&(*a as i64 + 1)],
        );
        for (x, y) in fb.iter().zip(fa.iter()).take(fb.len().div_ceil(2)) {
            out[*x].photo = next_photo;
            out[*y].photo = next_photo;
            next_photo += 1;
        }
    }
    for i in order {
        if out[i].photo != 0 {
            continue;
        }
        let id = out[i].identity.unwrap();
        if rng.f64() < cfg.group_photos {
            if let Some(slot) = open
                .iter_mut()
                .find(|(_, ids)| ids.len() < 4 && !ids.contains(&id))
            {
                slot.1.push(id);
                out[i].photo = slot.0;
                continue;
            }
            open.push((next_photo, vec![id]));
            out[i].photo = next_photo;
            next_photo += 1;
            if open.len() > 8 {
                open.remove(0);
            }
        } else {
            out[i].photo = next_photo;
            next_photo += 1;
        }
    }
    out
}

/// Legge un file JSONL di [`Sample`].
pub fn read_jsonl(path: &std::path::Path) -> std::io::Result<Vec<Sample>> {
    let f = std::io::BufReader::new(std::fs::File::open(path)?);
    let mut out = Vec::new();
    for line in f.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        out.push(
            serde_json::from_str(&line)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?,
        );
    }
    Ok(out)
}

/// Volti "già confermati" dall'utente: per le `named` identità con più volti, la quota
/// `share` dei volti del periodo con più foto (anno di scatto più frequente; tutti i volti se
/// le date mancano), scelti in modo riproducibile. Simula le persone a cui l'utente ha già dato
/// un nome partendo dalle foto più numerose: gli altri periodi restano frammenti da scoprire.
pub fn confirmed_mask(samples: &[Sample], named: usize, share: f64, seed: u64) -> Vec<bool> {
    let mut rng = Rng::new(seed ^ 0x00C0_FFEE);
    let mut count: BTreeMap<i64, usize> = BTreeMap::new();
    let mut years: BTreeMap<i64, BTreeMap<i64, usize>> = BTreeMap::new();
    let year = |s: &Sample| s.time.map(|t| t.div_euclid(YEAR_MS));
    for s in samples {
        if let Some(id) = s.identity {
            *count.entry(id).or_default() += 1;
            if let Some(y) = year(s) {
                *years.entry(id).or_default().entry(y).or_default() += 1;
            }
        }
    }
    let main_year: BTreeMap<i64, i64> = years
        .into_iter()
        .filter_map(|(id, ys)| {
            ys.into_iter()
                .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))
                .map(|(y, _)| (id, y))
        })
        .collect();
    let mut ranked: Vec<(i64, usize)> = count.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let chosen: HashSet<i64> = ranked.iter().take(named).map(|x| x.0).collect();
    samples
        .iter()
        .map(|s| {
            let in_period = match (s.identity.and_then(|id| main_year.get(&id)), year(s)) {
                (Some(m), Some(y)) => *m == y,
                _ => true,
            };
            s.identity.is_some_and(|id| chosen.contains(&id)) && in_period && rng.f64() < share
        })
        .collect()
}

const NAMED_BASE: i64 = 1_000_000;

fn named_person(identity: i64) -> i64 {
    NAMED_BASE + identity
}

/// Volti e persone per il raggruppamento: i volti confermati sono nella persona con nome
/// della loro identità; gli altri sono nuovi.
pub fn to_faces(samples: &[Sample], confirmed: &[bool]) -> (Vec<Face>, People) {
    let mut people = People::default();
    let faces = samples
        .iter()
        .zip(confirmed)
        .map(|(s, c)| {
            let person = if *c {
                let p = named_person(s.identity.unwrap_or_default());
                people.protected.insert(p);
                people.named.insert(p);
                Some(p)
            } else {
                None
            };
            Face {
                id: s.face,
                media: s.photo,
                model: "sface".into(),
                embedding: s.embedding.clone(),
                quality: s.quality,
                time: s.time,
                person,
                rejected: Vec::new(),
            }
        })
        .collect();
    (faces, people)
}

fn centroids(faces: &[Face]) -> BTreeMap<i64, Vec<f32>> {
    let mut sums: BTreeMap<i64, Vec<f32>> = BTreeMap::new();
    for f in faces {
        if let Some(p) = f.person {
            let s = sums.entry(p).or_insert_with(|| vec![0.0; DIM]);
            s.iter_mut().zip(&f.embedding).for_each(|(a, b)| *a += b);
        }
    }
    for v in sums.values_mut() {
        normalize(v);
    }
    sums
}

/// Abbinamento dei volti nuovi alle persone con nome, come al salvataggio dei volti. Prima:
/// coseno con il centroide sopra la soglia. Dopo (`margin`): centroide come filtro rapido,
/// poi collegamento robusto con i volti migliori della persona sopra la soglia e margine
/// verso la seconda (`pc_store::people::named_match`).
fn match_named(faces: &mut [Face], threshold: f32, margin: Option<f32>) {
    let cents = centroids(faces);
    let mut members: BTreeMap<i64, Vec<usize>> = BTreeMap::new();
    for (i, f) in faces.iter().enumerate() {
        if let Some(p) = f.person {
            members.entry(p).or_default().push(i);
        }
    }
    let samples: BTreeMap<i64, Vec<Vec<f32>>> = members
        .iter()
        .map(|(p, m)| {
            let mut m = m.clone();
            m.sort_by(|a, b| {
                faces[*b]
                    .quality
                    .total_cmp(&faces[*a].quality)
                    .then(faces[*a].id.cmp(&faces[*b].id))
            });
            m.truncate(pc_store::people::LINK_SAMPLES);
            (*p, m.iter().map(|i| faces[*i].embedding.clone()).collect())
        })
        .collect();
    for f in faces.iter_mut().filter(|f| f.person.is_none()) {
        let mut scored: Vec<(i64, f32)> = cents
            .iter()
            .filter_map(|(p, c)| {
                let centroid = cosine(&f.embedding, c);
                if margin.is_none() {
                    return (centroid >= threshold).then_some((*p, centroid));
                }
                if centroid < threshold * 0.8 {
                    return None;
                }
                let refs: Vec<&[f32]> = samples[p].iter().map(|s| s.as_slice()).collect();
                pc_store::people::robust_linkage(&[&f.embedding], &refs)
                    .filter(|s| *s >= threshold)
                    .map(|s| (*p, s))
            })
            .collect();
        scored.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        let Some(&(p, s)) = scored.first() else {
            continue;
        };
        if let (Some(m), Some(&(_, s2))) = (margin, scored.get(1)) {
            if s - s2 < m {
                continue;
            }
        }
        f.person = Some(p);
    }
}

/// Il raggruppamento di prima della change: abbinamento ai centroidi delle persone con nome
/// (soglia di somiglianza, senza margine), poi Chinese Whispers su kNN con le etichette con
/// nome che si propagano, partenza da etichette proprie, ordine alternato, nessun vincolo
/// della stessa foto e nessun secondo passaggio.
pub fn legacy_cluster(samples: &[Sample], confirmed: &[bool], threshold: f32) -> Vec<Label> {
    let (mut faces, people) = to_faces(samples, confirmed);
    match_named(&mut faces, threshold, None);
    let n = faces.len();
    let fixed: Vec<Option<i64>> = faces
        .iter()
        .map(|f| f.person.filter(|p| people.protected.contains(p)))
        .collect();
    let mut labels: Vec<Label> = faces
        .iter()
        .zip(&fixed)
        .map(|(f, p)| p.map(Label::Person).unwrap_or(Label::Seed(f.id)))
        .collect();
    let ids: Vec<usize> = (0..n).collect();
    let index = (n > 64).then(|| {
        let h = Hnsw::<f32, DistCosine>::new(16, n, 16, 100, DistCosine {});
        for i in &ids {
            h.insert((faces[*i].embedding.as_slice(), *i));
        }
        h
    });
    let graph: Vec<Vec<(usize, f32)>> = ids
        .iter()
        .map(|&i| {
            let v = &faces[i].embedding;
            let neighbours: Vec<usize> = match &index {
                Some(h) => h
                    .search(v, 17.min(n), 50)
                    .into_iter()
                    .map(|x| x.d_id)
                    .collect(),
                None => ids.clone(),
            };
            let mut edges: Vec<(usize, f32)> = neighbours
                .into_iter()
                .filter(|j| *j != i)
                .filter_map(|j| {
                    let s = cosine(v, &faces[j].embedding);
                    (s >= threshold).then_some((j, s))
                })
                .collect();
            edges.sort_by(|a, b| b.1.total_cmp(&a.1));
            edges.truncate(16);
            edges
        })
        .collect();
    for iteration in 0..30 {
        let mut changed = false;
        for step in 0..n {
            let i = if iteration % 2 == 0 {
                step
            } else {
                n - 1 - step
            };
            if fixed[i].is_some() {
                continue;
            }
            let mut votes: BTreeMap<Label, f32> = BTreeMap::new();
            for (j, s) in &graph[i] {
                *votes.entry(labels[*j]).or_default() += s;
            }
            let prior = labels[i];
            let selected = votes
                .into_iter()
                .max_by(|a, b| {
                    a.1.total_cmp(&b.1)
                        .then_with(|| (a.0 == prior).cmp(&(b.0 == prior)))
                })
                .map(|(l, _)| l)
                .unwrap_or(prior);
            changed |= selected != prior;
            labels[i] = selected;
        }
        if !changed {
            break;
        }
    }
    labels
}

/// Il raggruppamento nuovo sugli stessi dati: abbinamento con margine ai centroidi delle
/// persone con nome, poi [`aggregate::aggregate`] in ricalcolo completo.
pub fn run_new(samples: &[Sample], confirmed: &[bool], params: Params) -> (Vec<Face>, Outcome) {
    let (mut faces, people) = to_faces(samples, confirmed);
    match_named(&mut faces, params.ask, Some(params.margin));
    let out = aggregate::aggregate(&faces, &people, params, Mode::Full, &JobControl::new())
        .expect("raggruppamento");
    (faces, out)
}

/// Misure di un raggruppamento sui volti etichettati.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Metrics {
    pub pairwise_precision: f64,
    pub pairwise_recall: f64,
    pub pairwise_f: f64,
    pub bcubed_precision: f64,
    pub bcubed_recall: f64,
    pub bcubed_f: f64,
    /// Gruppi in cui finiscono i volti di un'identità, in media.
    pub groups_per_identity: f64,
    /// Quota dei gruppi con un solo volto.
    pub single_face_share: f64,
    pub groups: usize,
    pub auto_merges: usize,
    /// Quota di unioni automatiche corrette (1 se nessuna).
    pub auto_precision: f64,
}

fn f1(p: f64, r: f64) -> f64 {
    if p + r == 0.0 {
        0.0
    } else {
        2.0 * p * r / (p + r)
    }
}

/// Precisione, richiamo e F BCubed sui volti con identità (`clusters[i]` = gruppo del volto).
pub fn bcubed<K: std::hash::Hash + Eq + Copy>(
    identities: &[Option<i64>],
    clusters: &[K],
) -> (f64, f64, f64) {
    let mut joint: HashMap<(K, i64), f64> = HashMap::new();
    let mut by_cluster: HashMap<K, f64> = HashMap::new();
    let mut by_identity: HashMap<i64, f64> = HashMap::new();
    let mut n = 0.0;
    for (id, k) in identities.iter().zip(clusters) {
        if let Some(id) = id {
            *joint.entry((*k, *id)).or_default() += 1.0;
            *by_cluster.entry(*k).or_default() += 1.0;
            *by_identity.entry(*id).or_default() += 1.0;
            n += 1.0;
        }
    }
    if n == 0.0 {
        return (1.0, 1.0, 1.0);
    }
    let (mut p, mut r) = (0.0, 0.0);
    for ((k, id), c) in &joint {
        p += c * c / by_cluster[k];
        r += c * c / by_identity[id];
    }
    let (p, r) = (p / n, r / n);
    (p, r, f1(p, r))
}

/// Precisione, richiamo e F a coppie sui volti con identità.
pub fn pairwise<K: std::hash::Hash + Eq + Copy>(
    identities: &[Option<i64>],
    clusters: &[K],
) -> (f64, f64, f64) {
    let pairs = |c: f64| c * (c - 1.0) / 2.0;
    let mut joint: HashMap<(K, i64), f64> = HashMap::new();
    let mut by_cluster: HashMap<K, f64> = HashMap::new();
    let mut by_identity: HashMap<i64, f64> = HashMap::new();
    for (id, k) in identities.iter().zip(clusters) {
        if let Some(id) = id {
            *joint.entry((*k, *id)).or_default() += 1.0;
            *by_cluster.entry(*k).or_default() += 1.0;
            *by_identity.entry(*id).or_default() += 1.0;
        }
    }
    let tp: f64 = joint.values().map(|c| pairs(*c)).sum();
    let predicted: f64 = by_cluster.values().map(|c| pairs(*c)).sum();
    let truth: f64 = by_identity.values().map(|c| pairs(*c)).sum();
    let p = if predicted > 0.0 { tp / predicted } else { 1.0 };
    let r = if truth > 0.0 { tp / truth } else { 1.0 };
    (p, r, f1(p, r))
}

/// Gruppi per identità (media) e quota di gruppi da un volto (tra i gruppi con almeno un
/// volto etichettato).
pub fn fragmentation<K: std::hash::Hash + Eq + Copy + Ord>(
    identities: &[Option<i64>],
    clusters: &[K],
) -> (f64, f64, usize) {
    let mut per_identity: HashMap<i64, HashSet<K>> = HashMap::new();
    let mut size: BTreeMap<K, usize> = BTreeMap::new();
    for (id, k) in identities.iter().zip(clusters) {
        if let Some(id) = id {
            per_identity.entry(*id).or_default().insert(*k);
            *size.entry(*k).or_default() += 1;
        }
    }
    let gpi = if per_identity.is_empty() {
        0.0
    } else {
        per_identity.values().map(|s| s.len() as f64).sum::<f64>() / per_identity.len() as f64
    };
    let singles = size.values().filter(|n| **n == 1).count();
    let share = if size.is_empty() {
        0.0
    } else {
        singles as f64 / size.len() as f64
    };
    (gpi, share, size.len())
}

fn majority(identities: &[Option<i64>], members: &[usize]) -> Option<i64> {
    let mut c: BTreeMap<i64, usize> = BTreeMap::new();
    for i in members {
        if let Some(id) = identities[*i] {
            *c.entry(id).or_default() += 1;
        }
    }
    c.into_iter()
        .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))
        .map(|x| x.0)
}

/// Misure di un raggruppamento: `labels` per volto e, se c'è, l'esito con le unioni
/// automatiche (corrette se l'identità prevalente dell'origine è quella della destinazione).
pub fn measure(samples: &[Sample], labels: &[Label], outcome: Option<&Outcome>) -> Metrics {
    let ids: Vec<Option<i64>> = samples.iter().map(|s| s.identity).collect();
    let (pp, pr, pf) = pairwise(&ids, labels);
    let (bp, br, bf) = bcubed(&ids, labels);
    let (gpi, single, groups) = fragmentation(&ids, labels);
    let (mut merges, mut correct) = (0usize, 0usize);
    if let Some(out) = outcome {
        // Nucleo di ogni gruppo finale: i volti del gruppo del primo passaggio che ha tenuto
        // l'etichetta (esclusi tutti i gruppi uniti dentro).
        let merged: HashSet<usize> = out
            .merges
            .iter()
            .flat_map(|m| m.faces.iter().copied())
            .collect();
        let mut core: HashMap<Label, Vec<usize>> = HashMap::new();
        for (i, l) in out.labels.iter().enumerate() {
            if !merged.contains(&i) {
                core.entry(*l).or_default().push(i);
            }
        }
        for m in &out.merges {
            let rest = core.get(&m.into).cloned().unwrap_or_default();
            let (a, b) = (majority(&ids, &m.faces), majority(&ids, &rest));
            if a.is_none() || b.is_none() {
                continue;
            }
            merges += 1;
            if a == b {
                correct += 1;
            }
        }
    }
    Metrics {
        pairwise_precision: pp,
        pairwise_recall: pr,
        pairwise_f: pf,
        bcubed_precision: bp,
        bcubed_recall: br,
        bcubed_f: bf,
        groups_per_identity: gpi,
        single_face_share: single,
        groups,
        auto_merges: merges,
        auto_precision: if merges == 0 {
            1.0
        } else {
            correct as f64 / merges as f64
        },
    }
}

struct Components {
    parent: Vec<usize>,
}

impl Components {
    fn find(&mut self, x: usize) -> usize {
        let mut r = x;
        while self.parent[r] != r {
            r = self.parent[r];
        }
        let mut y = x;
        while self.parent[y] != r {
            let n = self.parent[y];
            self.parent[y] = r;
            y = n;
        }
        r
    }
}

/// Domande a cui un utente perfetto deve rispondere, nell'ordine dato, perché la BCubed F
/// arrivi a `target` (`None` se non ci arriva). Un "Sì" unisce i due gruppi (union-find),
/// un "No" diventa un vincolo; le domande ormai risolte o contraddette non si contano.
pub fn questions_to_reach(
    samples: &[Sample],
    outcome: &Outcome,
    questions: &[Question],
    target: f64,
) -> Option<usize> {
    trajectory(samples, outcome, questions)
        .iter()
        .position(|f| *f >= target)
}

/// BCubed F prima di ogni risposta e dopo ciascuna delle domande contate (l'utente perfetto
/// di [`questions_to_reach`]): l'elemento `k` è la F dopo `k` risposte.
pub fn trajectory(samples: &[Sample], outcome: &Outcome, questions: &[Question]) -> Vec<f64> {
    let ids: Vec<Option<i64>> = samples.iter().map(|s| s.identity).collect();
    let mut keys: BTreeMap<Label, usize> = BTreeMap::new();
    for l in &outcome.labels {
        let n = keys.len();
        keys.entry(*l).or_insert(n);
    }
    let mut members: Vec<Vec<usize>> = vec![Vec::new(); keys.len()];
    for (i, l) in outcome.labels.iter().enumerate() {
        members[keys[l]].push(i);
    }
    let mut uf = Components {
        parent: (0..keys.len()).collect(),
    };
    let mut ident: Vec<Option<i64>> = members.iter().map(|m| majority(&ids, m)).collect();
    let mut nos: Vec<(usize, usize)> = Vec::new();
    let score = |uf: &mut Components| {
        let clusters: Vec<usize> = outcome.labels.iter().map(|l| uf.find(keys[l])).collect();
        bcubed(&ids, &clusters).2
    };
    let mut out = vec![score(&mut uf)];
    for q in questions {
        let (Some(&a), Some(&b)) = (keys.get(&q.a), keys.get(&q.b)) else {
            continue;
        };
        let ra = uf.find(a);
        let mut options = vec![b];
        if let Some(c) = q.c.and_then(|c| keys.get(&c).copied()) {
            options.push(c);
        }
        let roots: Vec<usize> = options.iter().map(|o| uf.find(*o)).collect();
        if roots.contains(&ra) {
            continue; // già risolta per transitività
        }
        let contradicted = roots.iter().all(|r| {
            nos.iter().any(|(x, y)| {
                let (x, y) = (uf.find(*x), uf.find(*y));
                (x, y) == (ra, *r) || (y, x) == (ra, *r)
            })
        });
        if contradicted {
            continue;
        }
        let right = roots
            .iter()
            .copied()
            .find(|r| ident[ra].is_some() && ident[*r] == ident[ra]);
        match right {
            Some(r) => {
                uf.parent[r] = ra;
                // Identità prevalente del gruppo unito.
                let merged: Vec<usize> = (0..members.len())
                    .filter(|g| uf.find(*g) == ra)
                    .flat_map(|g| members[g].iter().copied())
                    .collect();
                ident[ra] = majority(&ids, &merged);
            }
            None => {
                for r in roots {
                    nos.push((ra, r));
                }
            }
        }
        out.push(score(&mut uf));
    }
    out
}

/// Conteggio delle domande per tipo (per il riepilogo del banco di prova).
pub fn question_kinds(qs: &[Question]) -> (usize, usize, usize) {
    qs.iter().fold((0, 0, 0), |acc, q| match q.kind {
        QuestionKind::GroupPerson => (acc.0 + 1, acc.1, acc.2),
        QuestionKind::GroupGroup => (acc.0, acc.1 + 1, acc.2),
        QuestionKind::GroupChoice => (acc.0, acc.1, acc.2 + 1),
    })
}

/// Esporta in JSONL gli embedding dei volti delle persone con nome (o collegate a
/// un'identità, non ignorate) del catalogo `c`, con soli id anonimi: identità, foto e volti
/// rinumerati da 1, qualità ed embedding. Nessun nome, percorso, data o id del catalogo.
/// Con `unlabeled` aggiunge gli altri volti con embedding, senza identità. Restituisce le
/// righe scritte.
pub fn export_named(
    c: &rusqlite::Connection,
    out: &mut impl Write,
    unlabeled: bool,
) -> std::io::Result<usize> {
    let err = |e: rusqlite::Error| std::io::Error::other(e.to_string());
    let mut q = c
        .prepare(
            "SELECT f.id, f.media_id, COALESCE(f.quality_score, f.quality), f.embedding, \
                    CASE WHEN p.ignored_at IS NULL AND (p.name IS NOT NULL OR p.identity_uid IS NOT NULL) \
                         THEN p.id END \
             FROM face f LEFT JOIN person p ON p.id = f.person_id \
             WHERE f.embedding IS NOT NULL ORDER BY f.id",
        )
        .map_err(err)?;
    type Row = (i64, i64, f32, Vec<u8>, Option<i64>);
    let rows: Vec<Row> = q
        .query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })
        .map_err(err)?
        .collect::<Result<_, _>>()
        .map_err(err)?;
    let mut identities: BTreeMap<i64, i64> = BTreeMap::new();
    for (.., p) in &rows {
        if let Some(p) = p {
            identities.insert(*p, 0);
        }
    }
    for (i, v) in identities.values_mut().enumerate() {
        *v = i as i64 + 1;
    }
    let mut photos: HashMap<i64, i64> = HashMap::new();
    let mut written = 0usize;
    for (_, media, quality, raw, person) in rows {
        if person.is_none() && !unlabeled {
            continue;
        }
        let Ok(embedding) = pc_store::people::vector(&raw) else {
            continue;
        };
        let n = photos.len() as i64 + 1;
        let photo = *photos.entry(media).or_insert(n);
        written += 1;
        let line = Sample {
            face: written as i64,
            identity: person.map(|p| identities[&p]),
            photo,
            quality,
            time: None,
            embedding,
        };
        serde_json::to_writer(&mut *out, &line)?;
        out.write_all(b"\n")?;
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metriche_calcolate_a_mano() {
        // Identità: A A A B B; gruppi: 1 1 2 2 3.
        let ids = [Some(1), Some(1), Some(1), Some(2), Some(2)];
        let cl = [1, 1, 2, 2, 3];
        // Coppie: predette (1,1),(2,2) = 2, vere 3 + 1 = 4, corrette 1.
        let (p, r, f) = pairwise(&ids, &cl);
        assert!((p - 0.5).abs() < 1e-12);
        assert!((r - 0.25).abs() < 1e-12);
        assert!((f - 1.0 / 3.0).abs() < 1e-12);
        // BCubed: precisioni 1,1,1/2,1/2,1 → 4/5; richiami 2/3,2/3,1/3,1/2,1/2 → 8/15.
        let (bp, br, bf) = bcubed(&ids, &cl);
        assert!((bp - 0.8).abs() < 1e-12);
        assert!((br - 8.0 / 15.0).abs() < 1e-12);
        assert!((bf - 2.0 * 0.8 * (8.0 / 15.0) / (0.8 + 8.0 / 15.0)).abs() < 1e-12);
        // Gruppi per identità: A in 2, B in 2 → 2; gruppi da un volto: {3} su 3.
        let (gpi, single, groups) = fragmentation(&ids, &cl);
        assert!((gpi - 2.0).abs() < 1e-12);
        assert!((single - 1.0 / 3.0).abs() < 1e-12);
        assert_eq!(groups, 3);
        // Raggruppamento perfetto.
        let (_, _, f) = bcubed(&ids, &[7, 7, 7, 8, 8]);
        assert!((f - 1.0).abs() < 1e-12);
        // Volti senza etichetta esclusi.
        let (p, _, _) = bcubed(&[Some(1), None, Some(1)], &[1, 2, 1]);
        assert!((p - 1.0).abs() < 1e-12);
    }

    #[test]
    fn precisione_delle_unioni_automatiche() {
        let samples: Vec<Sample> = [1, 1, 2, 2]
            .iter()
            .enumerate()
            .map(|(i, id)| Sample {
                face: i as i64,
                identity: Some(*id),
                photo: i as i64,
                quality: 0.9,
                time: None,
                embedding: vec![0.0; 4],
            })
            .collect();
        let labels = vec![Label::Seed(0); 4];
        let out = Outcome {
            labels: labels.clone(),
            merges: vec![
                aggregate::Merge {
                    from: Label::Seed(1),
                    into: Label::Seed(0),
                    faces: vec![1],
                    score: 0.6,
                    small: false,
                },
                aggregate::Merge {
                    from: Label::Seed(2),
                    into: Label::Seed(0),
                    faces: vec![2, 3],
                    score: 0.6,
                    small: false,
                },
            ],
            questions: Vec::new(),
        };
        let m = measure(&samples, &labels, Some(&out));
        assert_eq!(m.auto_merges, 2);
        // Il volto 1 (A) va con A: giusta; il gruppo {2,3} (B) va con {0,1} (A): sbagliata.
        assert!((m.auto_precision - 0.5).abs() < 1e-12);
    }

    #[test]
    fn dati_sintetici_riproducibili_e_con_i_casi_difficili() {
        let cfg = SynthConfig {
            identities: 30,
            largest: 40,
            ..Default::default()
        };
        let a = synthetic(&cfg);
        let b = synthetic(&cfg);
        assert_eq!(a, b, "stesso seme, stessi dati");
        assert!(a.iter().all(|s| s.photo > 0));
        assert!(a.iter().all(|s| s.embedding.len() == 128));
        let norm = |v: &[f32]| v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(a.iter().all(|s| (norm(&s.embedding) - 1.0).abs() < 1e-4));
        // Foto di gruppo: volti di identità diverse nella stessa foto.
        let mut per_photo: HashMap<i64, Vec<i64>> = HashMap::new();
        for s in &a {
            per_photo
                .entry(s.photo)
                .or_default()
                .push(s.identity.unwrap());
        }
        assert!(per_photo.values().any(|v| v.len() > 1));
        for v in per_photo.values() {
            let set: HashSet<_> = v.iter().collect();
            assert_eq!(
                set.len(),
                v.len(),
                "mai due volti della stessa identità in una foto"
            );
        }
        assert!(a.iter().any(|s| s.quality < 0.4), "volti di qualità bassa");
        // Volti della stessa identità più simili di quelli di identità diverse.
        let same = cosine(&a[0].embedding, &a[1].embedding);
        let other = a.iter().find(|s| s.identity != a[0].identity).unwrap();
        assert!(same > cosine(&a[0].embedding, &other.embedding));
    }

    #[test]
    fn utente_perfetto_e_domande_risolte() {
        // Due gruppi della stessa identità e uno diverso: una domanda giusta basta.
        let samples: Vec<Sample> = [1, 1, 1, 1, 2]
            .iter()
            .enumerate()
            .map(|(i, id)| Sample {
                face: i as i64,
                identity: Some(*id),
                photo: i as i64,
                quality: 0.9,
                time: None,
                embedding: vec![0.0; 4],
            })
            .collect();
        let labels = vec![
            Label::Seed(0),
            Label::Seed(0),
            Label::Seed(2),
            Label::Seed(2),
            Label::Seed(4),
        ];
        let out = Outcome {
            labels,
            merges: Vec::new(),
            questions: Vec::new(),
        };
        let q = |a: i64, b: i64| Question {
            kind: QuestionKind::GroupGroup,
            a: Label::Seed(a),
            b: Label::Seed(b),
            c: None,
            score: 0.5,
            probability: 0.5,
            faces: 2,
        };
        // "No" (0 e 4) conta, poi "Sì" (0 e 2) porta la F a 1.
        assert_eq!(
            questions_to_reach(&samples, &out, &[q(0, 4), q(0, 2), q(2, 4)], 0.99),
            Some(2)
        );
        assert_eq!(questions_to_reach(&samples, &out, &[q(0, 4)], 0.99), None);
        // Quattro gruppi: 0 e 2 di A, 4 e 5 di B. Dopo "0 ≠ 4" e "0 = 2", la domanda
        // "2 = 4?" è contraddetta e non si conta; "2 = 0?" è già risolta.
        let mut more = samples.clone();
        more.push(Sample {
            face: 5,
            identity: Some(2),
            photo: 5,
            quality: 0.9,
            time: None,
            embedding: vec![0.0; 4],
        });
        let mut labels = out.labels.clone();
        labels.push(Label::Seed(5));
        let out = Outcome {
            labels,
            merges: Vec::new(),
            questions: Vec::new(),
        };
        assert_eq!(
            questions_to_reach(
                &more,
                &out,
                &[q(0, 4), q(0, 2), q(2, 4), q(2, 0), q(4, 5)],
                0.99
            ),
            Some(3)
        );
    }
}
