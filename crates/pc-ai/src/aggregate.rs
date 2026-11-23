//! Raggruppamento dei volti in due passaggi (design D1–D3 e D6), senza accesso al catalogo:
//! - grafo kNN con il vincolo della stessa foto;
//! - primo passaggio: Chinese Whispers deterministico, avviato dalle etichette precedenti;
//! - secondo passaggio: unione dei gruppi con collegamento robusto (mediana tra campioni),
//!   vicinanza reciproca, vincoli e margine per le date lontane;
//! - abbinamento dei gruppi piccoli alle persone con nome, con margine verso la seconda;
//! - domande per la coda "Da confermare", nell'ordine che risolve più volti per risposta.
//!
//! È la stessa funzione usata da `cluster::run` sul catalogo e dal banco di prova
//! (`examples/cluster_bench.rs`) sui dati etichettati.

use hnsw_rs::prelude::*;
use pc_core::{AppError, AppResult};
use pc_jobs::JobControl;
use pc_store::people::{cosine, robust_linkage, LINK_SAMPLES};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BinaryHeap, HashMap, HashSet};

/// Vicini per volto nel grafo kNN.
pub const NEIGHBOURS: usize = 16;
/// Gruppi candidati per gruppo nel secondo passaggio: quelli con più archi in comune.
pub const CANDIDATES: usize = 8;
/// Vicinanza reciproca: B tra i 3 gruppi più vicini di A e viceversa.
pub const RECIPROCAL: usize = 3;
/// Gruppi piccoli (design D3): al massimo 3 volti.
pub const SMALL_GROUP: usize = 3;
/// Qualità combinata sotto la quale un volto è "di qualità bassa": un gruppo di soli volti
/// così non si unisce mai da solo a una persona con nome (design D3).
pub const LOW_QUALITY: f32 = 0.4;
/// Distanza oltre la quale l'unione automatica chiede un margine in più (design D2).
pub const FAR_MS: i64 = 3 * 365 * 24 * 60 * 60 * 1000 + 24 * 60 * 60 * 1000;
pub const FAR_MARGIN: f32 = 0.05;
/// Fino a quanti volti per modello il grafo è esatto (oltre, HNSW con seme fisso).
pub const EXACT_LIMIT: usize = 2048;
const ITERATIONS: usize = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Label {
    Person(i64),
    Seed(i64),
}

/// Un volto da raggruppare.
#[derive(Debug, Clone, Default)]
pub struct Face {
    pub id: i64,
    pub media: i64,
    pub model: String,
    pub embedding: Vec<f32>,
    /// Qualità combinata (0..1).
    pub quality: f32,
    /// Istante di scatto in millisecondi, se noto.
    pub time: Option<i64>,
    /// Persona attuale.
    pub person: Option<i64>,
    /// Persone rifiutate ("Non è questa persona").
    pub rejected: Vec<i64>,
}

/// Stato delle persone che il raggruppamento deve rispettare.
#[derive(Debug, Clone, Default)]
pub struct People {
    /// Persone con etichetta fissa: con nome o identità, ignorate, con volti confermati,
    /// rifiutate o con vincoli "Persone diverse". I loro volti non si spostano.
    pub protected: HashSet<i64>,
    /// Persone con nome o identità.
    pub named: HashSet<i64>,
    /// Gruppi ignorati: attraggono i volti simili ma non generano domande.
    pub ignored: HashSet<i64>,
    /// Coppie "Persone diverse" (in entrambi i versi).
    pub not_same: HashSet<(i64, i64)>,
}

/// Soglie del raggruppamento (design D4).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Params {
    /// "Chiedi": soglia degli archi del grafo e delle domande.
    pub ask: f32,
    /// "Unisci da solo".
    pub auto: f32,
    /// Margine verso la seconda persona candidata.
    pub margin: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            ask: pc_core::AiThresholds::DEFAULT_FACE_SIMILARITY,
            auto: pc_core::AiThresholds::DEFAULT_FACE_AUTO_MERGE,
            margin: pc_core::AiThresholds::DEFAULT_FACE_MERGE_MARGIN,
        }
    }
}

/// Ricalcolo completo (soglie cambiate, prima analisi dopo l'aggiornamento) o incrementale
/// (nuovi volti: cambiano solo i gruppi che toccano).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Full,
    Incremental,
}

/// Un gruppo del primo passaggio finito in un altro gruppo.
#[derive(Debug, Clone, PartialEq)]
pub struct Merge {
    /// Etichetta del gruppo del primo passaggio.
    pub from: Label,
    /// Etichetta del gruppo finale che lo contiene.
    pub into: Label,
    /// Volti (indici) del gruppo del primo passaggio.
    pub faces: Vec<usize>,
    /// Collegamento al momento dell'unione.
    pub score: f32,
    /// `true` se unito come gruppo piccolo a una persona con nome (design D3).
    pub small: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QuestionKind {
    GroupPerson,
    GroupGroup,
    GroupChoice,
}

/// Una domanda per la coda: `a` è il gruppo senza nome, `b` la persona o il gruppo proposto,
/// `c` la seconda persona di "È A o B?".
#[derive(Debug, Clone, PartialEq)]
pub struct Question {
    pub kind: QuestionKind,
    pub a: Label,
    pub b: Label,
    pub c: Option<Label>,
    pub score: f32,
    pub probability: f32,
    /// Volti risolti da un "Sì".
    pub faces: usize,
}

#[derive(Debug, Clone, Default)]
pub struct Outcome {
    /// Gruppo finale di ogni volto.
    pub labels: Vec<Label>,
    pub merges: Vec<Merge>,
    pub questions: Vec<Question>,
}

impl Outcome {
    /// Unioni che cambiano qualcosa per l'utente: i volti del gruppo d'origine non stavano già
    /// tutti nella persona in cui sono finiti (un ricalcolo completo che ricompone un gruppo
    /// già unito non è una unione nuova).
    pub fn visible_merges<'a>(&'a self, faces: &'a [Face]) -> impl Iterator<Item = &'a Merge> {
        let mut members: HashMap<Label, Vec<usize>> = HashMap::new();
        for (i, l) in self.labels.iter().enumerate() {
            members.entry(*l).or_default().push(i);
        }
        self.merges.iter().filter(move |m| {
            let from: HashSet<usize> = m.faces.iter().copied().collect();
            let rest: Vec<usize> = members
                .get(&m.into)
                .map(|v| v.iter().copied().filter(|i| !from.contains(i)).collect())
                .unwrap_or_default();
            let target = match m.into {
                Label::Person(p) => Some(p),
                Label::Seed(_) => majority_person(faces, &rest),
            };
            !(target.is_some() && m.faces.iter().all(|i| faces[*i].person == target))
        })
    }
}

/// Persona da cui proviene almeno metà dei volti (con una persona) dell'insieme.
pub fn majority_person(faces: &[Face], members: &[usize]) -> Option<i64> {
    let mut counts: BTreeMap<i64, usize> = BTreeMap::new();
    let mut known = 0;
    for i in members {
        if let Some(p) = faces[*i].person {
            *counts.entry(p).or_default() += 1;
            known += 1;
        }
    }
    let (p, n) = counts
        .into_iter()
        .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))?;
    (n * 2 >= known).then_some(p)
}

fn cancelled() -> AppError {
    AppError::cancelled()
}

/// Grafo kNN: per ogni volto al massimo [`NEIGHBOURS`] archi verso volti dello stesso modello
/// con coseno ≥ `threshold`, mai verso volti della stessa foto né tra persone "diverse".
/// Ordine degli archi: punteggio decrescente, poi indice: lo stesso input dà lo stesso grafo
/// (l'HNSW usa un seme fisso e inserimenti in ordine).
pub fn knn_graph(
    faces: &[Face],
    people: &People,
    threshold: f32,
    control: &JobControl,
) -> AppResult<Vec<Vec<(usize, f32)>>> {
    let mut by_model: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (i, f) in faces.iter().enumerate() {
        if !f.embedding.is_empty() {
            by_model.entry(&f.model).or_default().push(i);
        }
    }
    let mut graph = vec![Vec::<(usize, f32)>::new(); faces.len()];
    let allowed = |i: usize, j: usize| {
        j != i
            && faces[i].media != faces[j].media
            && !matches!(
                (faces[i].person, faces[j].person),
                (Some(a), Some(b)) if people.not_same.contains(&(a, b))
            )
    };
    for ids in by_model.values() {
        if !control.checkpoint() {
            return Err(cancelled());
        }
        if ids.len() <= EXACT_LIMIT {
            for &i in ids {
                if !control.checkpoint() {
                    return Err(cancelled());
                }
                let v = &faces[i].embedding;
                let mut edges: Vec<(usize, f32)> = ids
                    .iter()
                    .filter(|j| allowed(i, **j))
                    .filter_map(|&j| {
                        let s = cosine(v, &faces[j].embedding);
                        (s >= threshold).then_some((j, s))
                    })
                    .collect();
                edges.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
                edges.truncate(NEIGHBOURS);
                graph[i] = edges;
            }
            continue;
        }
        let h = Hnsw::<f32, DistCosine>::new(16, ids.len(), 16, 100, DistCosine {});
        for &i in ids {
            if !control.checkpoint() {
                return Err(cancelled());
            }
            h.insert((faces[i].embedding.as_slice(), i));
        }
        for &i in ids {
            if !control.checkpoint() {
                return Err(cancelled());
            }
            let v = &faces[i].embedding;
            // Qualche vicino in più: i volti della stessa foto vengono scartati.
            let mut edges: Vec<(usize, f32)> = h
                .search(v, (NEIGHBOURS + 5).min(ids.len()), 50)
                .into_iter()
                .map(|n| n.d_id)
                .filter(|j| allowed(i, *j))
                .filter_map(|j| {
                    let s = cosine(v, &faces[j].embedding);
                    (s >= threshold).then_some((j, s))
                })
                .collect();
            edges.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
            edges.dedup_by_key(|e| e.0);
            edges.truncate(NEIGHBOURS);
            graph[i] = edges;
        }
    }
    Ok(graph)
}

/// Archi in entrambi i versi, senza doppioni (per sapere chi è "toccato" da un cambio).
fn undirected(graph: &[Vec<(usize, f32)>]) -> Vec<Vec<usize>> {
    let mut adj = vec![Vec::new(); graph.len()];
    for (i, edges) in graph.iter().enumerate() {
        for (j, _) in edges {
            adj[i].push(*j);
            adj[*j].push(i);
        }
    }
    for a in &mut adj {
        a.sort_unstable();
        a.dedup();
    }
    adj
}

/// Etichette di partenza: fisse per le persone protette; per gli altri volti la persona
/// precedente (incrementale) o un'etichetta propria (completo). Un volto che con la sua
/// etichetta si troverebbe insieme a un altro volto della stessa foto riparte da solo.
/// Restituisce etichette, volti fissi e volti "nuovi" (da cui parte l'incrementale).
pub fn initial_labels(
    faces: &[Face],
    people: &People,
    mode: Mode,
) -> (Vec<Label>, Vec<bool>, Vec<bool>) {
    let mut labels = Vec::with_capacity(faces.len());
    let mut fixed = Vec::with_capacity(faces.len());
    let mut fresh = Vec::with_capacity(faces.len());
    let mut media_of: HashMap<Label, HashSet<i64>> = HashMap::new();
    for f in faces {
        let protected = f.person.filter(|p| people.protected.contains(p));
        let (mut label, is_fixed, mut is_fresh) = match (protected, f.person, mode) {
            (Some(p), _, _) => (Label::Person(p), true, false),
            (None, Some(p), Mode::Incremental) => (Label::Person(p), false, false),
            _ => (Label::Seed(f.id), false, true),
        };
        if !is_fixed && media_of.get(&label).is_some_and(|m| m.contains(&f.media)) {
            label = Label::Seed(f.id);
            is_fresh = true;
        }
        media_of.entry(label).or_default().insert(f.media);
        labels.push(label);
        fixed.push(is_fixed);
        fresh.push(is_fresh);
    }
    (labels, fixed, fresh)
}

/// Chinese Whispers deterministico (design D1): ordine di visita fisso, parità a favore
/// dell'etichetta attuale e poi della minore. Le etichette fisse non si propagano (le persone
/// protette ricevono volti solo dal secondo passaggio e dall'abbinamento dei gruppi piccoli);
/// un volto non prende un'etichetta che contiene già un altro volto della sua foto. Con
/// `active` si visitano solo quei volti e, a ogni cambio, i loro vicini.
pub fn chinese_whispers(
    faces: &[Face],
    graph: &[Vec<(usize, f32)>],
    fixed: &[bool],
    mut labels: Vec<Label>,
    active: Option<Vec<bool>>,
    control: &JobControl,
) -> AppResult<Vec<Label>> {
    let n = faces.len();
    let incremental = active.is_some();
    let mut active = active.unwrap_or_else(|| vec![true; n]);
    let adj = undirected(graph);
    let mut media_of: HashMap<Label, HashMap<i64, u32>> = HashMap::new();
    for (i, l) in labels.iter().enumerate() {
        *media_of
            .entry(*l)
            .or_default()
            .entry(faces[i].media)
            .or_default() += 1;
    }
    for _ in 0..ITERATIONS {
        let mut changed = false;
        let mut next = vec![false; n];
        for i in 0..n {
            if !control.checkpoint() {
                return Err(cancelled());
            }
            if fixed[i] || !active[i] {
                continue;
            }
            let prior = labels[i];
            let media = faces[i].media;
            let mut votes: BTreeMap<Label, f32> = BTreeMap::new();
            for &(j, s) in &graph[i] {
                if fixed[j] {
                    continue;
                }
                let l = labels[j];
                if l != prior
                    && media_of
                        .get(&l)
                        .and_then(|m| m.get(&media))
                        .is_some_and(|c| *c > 0)
                {
                    continue;
                }
                *votes.entry(l).or_default() += s;
            }
            let selected = votes
                .into_iter()
                .max_by(|a, b| {
                    a.1.total_cmp(&b.1)
                        .then_with(|| (a.0 == prior).cmp(&(b.0 == prior)))
                        .then_with(|| b.0.cmp(&a.0))
                })
                .map(|(l, _)| l)
                .unwrap_or(prior);
            if selected != prior {
                if let Some(m) = media_of.get_mut(&prior) {
                    if let Some(c) = m.get_mut(&media) {
                        *c -= 1;
                    }
                }
                *media_of
                    .entry(selected)
                    .or_default()
                    .entry(media)
                    .or_default() += 1;
                labels[i] = selected;
                changed = true;
                next[i] = true;
                for &j in &adj[i] {
                    next[j] = true;
                }
            }
        }
        if !changed {
            break;
        }
        if incremental {
            active = next;
        }
    }
    Ok(labels)
}

// ------------------------------------------------------------------ secondo passaggio

#[derive(Debug, Clone)]
struct Part {
    label: Label,
    faces: Vec<usize>,
    score: Option<f32>,
    small: bool,
}

#[derive(Debug, Clone)]
struct Group {
    key: Label,
    members: Vec<usize>,
    person: Option<i64>,
    protected: bool,
    /// Bersaglio "con nome" dell'abbinamento: persona con nome o identità, o gruppo ignorato.
    target: bool,
    named: bool,
    ignored: bool,
    media: HashSet<i64>,
    rejects: HashSet<i64>,
    times: Vec<i64>,
    samples: Vec<usize>,
    parts: Vec<Part>,
    touched: bool,
    alive: bool,
    version: u32,
}

impl Group {
    fn small_free(&self) -> bool {
        !self.protected && self.members.len() <= SMALL_GROUP
    }
}

fn pick_samples(faces: &[Face], members: &[usize]) -> Vec<usize> {
    let mut v = members.to_vec();
    v.sort_by(|a, b| {
        faces[*b]
            .quality
            .total_cmp(&faces[*a].quality)
            .then(faces[*a].id.cmp(&faces[*b].id))
    });
    v.truncate(LINK_SAMPLES);
    v
}

/// Distanza minima nel tempo tra le foto di due gruppi (`None` se uno non ha date).
fn time_gap(a: &[i64], b: &[i64]) -> Option<i64> {
    if a.is_empty() || b.is_empty() {
        return None;
    }
    let (mut i, mut j) = (0, 0);
    let mut best = i64::MAX;
    while i < a.len() && j < b.len() {
        best = best.min((a[i] - b[j]).abs());
        if a[i] < b[j] {
            i += 1;
        } else {
            j += 1;
        }
    }
    Some(best)
}

/// Elemento della coda di priorità del secondo passaggio (dal collegamento più alto).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Entry {
    score: f32,
    a: usize,
    b: usize,
    va: u32,
    vb: u32,
}
impl Eq for Entry {}
impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Entry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score
            .total_cmp(&other.score)
            .then_with(|| other.a.cmp(&self.a))
            .then_with(|| other.b.cmp(&self.b))
    }
}

struct Aggregator<'a> {
    faces: &'a [Face],
    people: &'a People,
    params: Params,
    mode: Mode,
    groups: Vec<Group>,
    /// Archi del grafo tra gruppi (conteggio).
    adj: Vec<HashMap<usize, u32>>,
    cand: Vec<Vec<usize>>,
    cache: HashMap<(usize, usize, u32, u32), Option<f32>>,
}

impl<'a> Aggregator<'a> {
    fn new(
        faces: &'a [Face],
        people: &'a People,
        params: Params,
        mode: Mode,
        graph: &[Vec<(usize, f32)>],
        labels: &[Label],
        touched_faces: &[bool],
    ) -> Self {
        let mut index: BTreeMap<Label, usize> = BTreeMap::new();
        let mut groups: Vec<Group> = Vec::new();
        let mut owner = vec![0usize; faces.len()];
        let mut ordered: BTreeMap<Label, Vec<usize>> = BTreeMap::new();
        for (i, l) in labels.iter().enumerate() {
            ordered.entry(*l).or_default().push(i);
        }
        for (label, members) in ordered {
            let gi = groups.len();
            index.insert(label, gi);
            for m in &members {
                owner[*m] = gi;
            }
            let person = match label {
                Label::Person(p) => Some(p),
                Label::Seed(_) => None,
            };
            let protected = person.is_some_and(|p| people.protected.contains(&p));
            let named = person.is_some_and(|p| people.named.contains(&p));
            let ignored = person.is_some_and(|p| people.ignored.contains(&p));
            let mut times: Vec<i64> = members.iter().filter_map(|i| faces[*i].time).collect();
            times.sort_unstable();
            groups.push(Group {
                key: label,
                person,
                protected,
                target: protected && (named || ignored),
                named,
                ignored,
                media: members.iter().map(|i| faces[*i].media).collect(),
                rejects: members
                    .iter()
                    .flat_map(|i| faces[*i].rejected.iter().copied())
                    .collect(),
                times,
                samples: pick_samples(faces, &members),
                parts: vec![Part {
                    label,
                    faces: members.clone(),
                    score: None,
                    small: false,
                }],
                touched: mode == Mode::Full || members.iter().any(|i| touched_faces[*i]),
                alive: true,
                version: 0,
                members,
            });
        }
        let mut adj = vec![HashMap::new(); groups.len()];
        for (i, edges) in graph.iter().enumerate() {
            for (j, _) in edges {
                let (gi, gj) = (owner[i], owner[*j]);
                if gi != gj {
                    *adj[gi].entry(gj).or_insert(0) += 1;
                    *adj[gj].entry(gi).or_insert(0) += 1;
                }
            }
        }
        let mut me = Self {
            faces,
            people,
            params,
            mode,
            groups,
            adj,
            cand: Vec::new(),
            cache: HashMap::new(),
        };
        me.cand = (0..me.groups.len()).map(|g| me.top_candidates(g)).collect();
        me
    }

    fn top_candidates(&self, g: usize) -> Vec<usize> {
        let mut v: Vec<(usize, u32)> = self.adj[g].iter().map(|(k, c)| (*k, *c)).collect();
        v.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then_with(|| self.groups[a.0].key.cmp(&self.groups[b.0].key))
        });
        v.truncate(CANDIDATES);
        v.into_iter().map(|(k, _)| k).collect()
    }

    /// Gruppi vicini di `g`: i suoi candidati e quelli che hanno `g` tra i propri.
    fn neighbourhood(&self, g: usize) -> Vec<usize> {
        let mut v: Vec<usize> = self.adj[g]
            .keys()
            .copied()
            .filter(|x| {
                self.groups[*x].alive && (self.cand[g].contains(x) || self.cand[*x].contains(&g))
            })
            .collect();
        v.sort_by_key(|x| self.groups[*x].key);
        v
    }

    fn linkage(&mut self, a: usize, b: usize) -> Option<f32> {
        let (lo, hi) = (a.min(b), a.max(b));
        let key = (lo, hi, self.groups[lo].version, self.groups[hi].version);
        if let Some(v) = self.cache.get(&key) {
            return *v;
        }
        let (ga, gb) = (&self.groups[lo], &self.groups[hi]);
        // Solo coppie dello stesso modello di embedding.
        type Sides<'x> = (Vec<&'x [f32]>, Vec<&'x [f32]>);
        let mut by_model: BTreeMap<&str, Sides> = BTreeMap::new();
        for i in &ga.samples {
            by_model
                .entry(&self.faces[*i].model)
                .or_default()
                .0
                .push(&self.faces[*i].embedding);
        }
        for i in &gb.samples {
            by_model
                .entry(&self.faces[*i].model)
                .or_default()
                .1
                .push(&self.faces[*i].embedding);
        }
        let mut scores = Vec::new();
        for (x, y) in by_model.values() {
            for u in x {
                for v in y {
                    scores.push(cosine(u, v));
                }
            }
        }
        let v = pc_store::people::median(&mut scores);
        self.cache.insert(key, v);
        v
    }

    /// Un vincolo dell'utente o la stessa foto separano i due gruppi.
    fn hard_blocked(&self, a: usize, b: usize) -> bool {
        let (ga, gb) = (&self.groups[a], &self.groups[b]);
        let (small, big) = if ga.media.len() <= gb.media.len() {
            (ga, gb)
        } else {
            (gb, ga)
        };
        if small.media.iter().any(|m| big.media.contains(m)) {
            return true;
        }
        if let (Some(pa), Some(pb)) = (ga.person, gb.person) {
            if self.people.not_same.contains(&(pa, pb)) {
                return true;
            }
        }
        ga.person.is_some_and(|p| gb.rejects.contains(&p))
            || gb.person.is_some_and(|p| ga.rejects.contains(&p))
    }

    /// I due gruppi possono essere confrontati per un'unione automatica.
    fn mergeable(&self, a: usize, b: usize) -> bool {
        (!self.groups[a].protected || !self.groups[b].protected) && !self.hard_blocked(a, b)
    }

    fn far_margin(&self, a: usize, b: usize) -> f32 {
        match time_gap(&self.groups[a].times, &self.groups[b].times) {
            Some(gap) if gap > FAR_MS => FAR_MARGIN,
            _ => 0.0,
        }
    }

    /// I [`RECIPROCAL`] gruppi più vicini di `g` tra quelli confrontabili.
    fn nearest(&mut self, g: usize) -> Vec<usize> {
        let mut scored: Vec<(usize, f32)> = Vec::new();
        for x in self.neighbourhood(g) {
            if !self.mergeable(g, x) {
                continue;
            }
            if let Some(s) = self.linkage(g, x) {
                scored.push((x, s));
            }
        }
        scored.sort_by(|a, b| {
            b.1.total_cmp(&a.1)
                .then_with(|| self.groups[a.0].key.cmp(&self.groups[b.0].key))
        });
        scored.truncate(RECIPROCAL);
        scored.into_iter().map(|x| x.0).collect()
    }

    /// Miglior collegamento di `g` verso un bersaglio con nome diverso da `except`.
    fn second_target(&mut self, g: usize, except: usize) -> Option<f32> {
        let others: Vec<usize> = self.adj[g]
            .keys()
            .copied()
            .filter(|x| *x != except && self.groups[*x].alive && self.groups[*x].target)
            .collect();
        let mut best: Option<f32> = None;
        for x in others {
            if self.hard_blocked(g, x) {
                continue;
            }
            if let Some(s) = self.linkage(g, x) {
                best = Some(best.map_or(s, |b: f32| b.max(s)));
            }
        }
        best
    }

    fn eligible(&mut self, a: usize, b: usize) -> Option<f32> {
        if !self.groups[a].alive || !self.groups[b].alive || !self.mergeable(a, b) {
            return None;
        }
        if self.mode == Mode::Incremental && !self.groups[a].touched && !self.groups[b].touched {
            return None;
        }
        let (ta, tb) = (self.groups[a].target, self.groups[b].target);
        // I gruppi piccoli verso le persone con nome seguono la regola del margine (D3).
        if (ta && self.groups[b].small_free()) || (tb && self.groups[a].small_free()) {
            return None;
        }
        let s = self.linkage(a, b)?;
        if s < self.params.auto + self.far_margin(a, b) {
            return None;
        }
        if !self.nearest(a).contains(&b) || !self.nearest(b).contains(&a) {
            return None;
        }
        if ta || tb {
            let (free, target) = if ta { (b, a) } else { (a, b) };
            if self
                .second_target(free, target)
                .is_some_and(|second| s - second < self.params.margin)
            {
                return None;
            }
        }
        Some(s)
    }

    /// Unisce `b` in `a` (o viceversa secondo la direzione) e restituisce il gruppo rimasto.
    fn merge(&mut self, a: usize, b: usize, score: f32, small: bool) -> usize {
        let (into, from) = {
            let (ga, gb) = (&self.groups[a], &self.groups[b]);
            if ga.protected != gb.protected {
                if ga.protected {
                    (a, b)
                } else {
                    (b, a)
                }
            } else if (ga.members.len(), std::cmp::Reverse(ga.key))
                >= (gb.members.len(), std::cmp::Reverse(gb.key))
            {
                (a, b)
            } else {
                (b, a)
            }
        };
        let mut moved = std::mem::take(&mut self.groups[from].parts);
        for p in &mut moved {
            if p.score.is_none() {
                p.score = Some(score);
                p.small = small;
            }
        }
        let members = std::mem::take(&mut self.groups[from].members);
        let media = std::mem::take(&mut self.groups[from].media);
        let rejects = std::mem::take(&mut self.groups[from].rejects);
        let times = std::mem::take(&mut self.groups[from].times);
        let samples = std::mem::take(&mut self.groups[from].samples);
        let touched = self.groups[from].touched;
        self.groups[from].alive = false;
        {
            let g = &mut self.groups[into];
            g.parts.extend(moved);
            g.members.extend(members);
            g.media.extend(media);
            g.rejects.extend(rejects);
            g.times.extend(times);
            g.times.sort_unstable();
            let mut s = std::mem::take(&mut g.samples);
            s.extend(samples);
            g.touched |= touched;
            g.version += 1;
            g.samples = s;
        }
        let s = self.groups[into].samples.clone();
        self.groups[into].samples = pick_samples(self.faces, &s);
        // Archi tra gruppi: quelli di `from` passano a `into`.
        let edges: Vec<(usize, u32)> = self.adj[from].drain().collect();
        for (x, cnt) in edges {
            self.adj[x].remove(&from);
            if x == into {
                continue;
            }
            *self.adj[into].entry(x).or_insert(0) += cnt;
            *self.adj[x].entry(into).or_insert(0) += cnt;
        }
        self.adj[into].remove(&from);
        let touched: Vec<usize> = self.adj[into].keys().copied().collect();
        self.cand[into] = self.top_candidates(into);
        for x in touched {
            self.cand[x] = self.top_candidates(x);
        }
        into
    }

    fn push_pairs(&mut self, heap: &mut BinaryHeap<Entry>, g: usize) {
        for x in self.neighbourhood(g) {
            if !self.mergeable(g, x) {
                continue;
            }
            if let Some(s) = self.linkage(g, x) {
                if s >= self.params.auto {
                    let (a, b) = (g.min(x), g.max(x));
                    heap.push(Entry {
                        score: s,
                        a,
                        b,
                        va: self.groups[a].version,
                        vb: self.groups[b].version,
                    });
                }
            }
        }
    }

    /// Secondo passaggio (design D2): unione agglomerativa dalla coppia migliore.
    fn second_pass(&mut self, control: &JobControl) -> AppResult<()> {
        let mut heap = BinaryHeap::new();
        for g in 0..self.groups.len() {
            if !control.checkpoint() {
                return Err(cancelled());
            }
            self.push_pairs(&mut heap, g);
        }
        while let Some(e) = heap.pop() {
            if !control.checkpoint() {
                return Err(cancelled());
            }
            let (ga, gb) = (&self.groups[e.a], &self.groups[e.b]);
            if !ga.alive || !gb.alive || ga.version != e.va || gb.version != e.vb {
                continue;
            }
            let Some(s) = self.eligible(e.a, e.b) else {
                continue;
            };
            let into = self.merge(e.a, e.b, s, false);
            // Il gruppo nuovo e i suoi vicini (la cui vicinanza può essere cambiata).
            self.push_pairs(&mut heap, into);
            for x in self.neighbourhood(into) {
                self.push_pairs(&mut heap, x);
            }
        }
        Ok(())
    }

    fn all_low_quality(&self, g: usize) -> bool {
        self.groups[g]
            .members
            .iter()
            .all(|i| self.faces[*i].quality < LOW_QUALITY)
    }

    /// Persone con nome raggiungibili nel grafo da `g`, con il collegamento, dal migliore.
    fn targets(&mut self, g: usize) -> Vec<(usize, f32)> {
        let mut xs: Vec<usize> = self.adj[g]
            .keys()
            .copied()
            .filter(|x| self.groups[*x].alive && self.groups[*x].target)
            .collect();
        xs.sort_by_key(|x| self.groups[*x].key);
        let mut out = Vec::new();
        for x in xs {
            if self.hard_blocked(g, x) {
                continue;
            }
            if let Some(s) = self.linkage(g, x) {
                out.push((x, s));
            }
        }
        out.sort_by(|a, b| {
            b.1.total_cmp(&a.1)
                .then_with(|| self.groups[a.0].key.cmp(&self.groups[b.0].key))
        });
        out
    }

    /// Abbinamento dei gruppi piccoli alle persone con nome (design D3).
    fn small_groups(&mut self, control: &JobControl) -> AppResult<()> {
        let mut order: Vec<usize> = (0..self.groups.len())
            .filter(|g| self.groups[*g].alive && self.groups[*g].small_free())
            .collect();
        order.sort_by_key(|g| self.groups[*g].key);
        for g in order {
            if !control.checkpoint() {
                return Err(cancelled());
            }
            if !self.groups[g].alive || !self.groups[g].small_free() {
                continue;
            }
            if self.mode == Mode::Incremental && !self.groups[g].touched {
                continue;
            }
            if self.all_low_quality(g) {
                continue;
            }
            let t = self.targets(g);
            let Some(&(best, s)) = t.first() else {
                continue;
            };
            let second = t.get(1).map(|x| x.1);
            if s >= self.params.auto + self.far_margin(g, best)
                && second.is_none_or(|x| s - x >= self.params.margin)
            {
                self.merge(best, g, s, true);
            }
        }
        Ok(())
    }

    fn probability(&self, score: f32) -> f32 {
        let span = self.params.auto - self.params.ask;
        if span <= 0.0 {
            return 1.0;
        }
        ((score - self.params.ask) / span).clamp(0.0, 1.0)
    }

    /// Domande per la coda (design D6), già ordinate.
    fn questions(&mut self) -> Vec<Question> {
        let ask = self.params.ask;
        let mut out = Vec::new();
        let mut pairs: HashSet<(Label, Label)> = HashSet::new();
        let mut order: Vec<usize> = (0..self.groups.len())
            .filter(|g| {
                let x = &self.groups[*g];
                x.alive && !x.named && !x.ignored
            })
            .collect();
        order.sort_by_key(|g| self.groups[*g].key);
        for g in order {
            let mut xs: Vec<usize> = self.adj[g]
                .keys()
                .copied()
                .filter(|x| self.groups[*x].alive && !self.groups[*x].ignored)
                .collect();
            xs.sort_by_key(|x| self.groups[*x].key);
            let mut named: Vec<(usize, f32)> = Vec::new();
            let mut unnamed: Vec<(usize, f32)> = Vec::new();
            for x in xs {
                if self.hard_blocked(g, x) {
                    continue;
                }
                let Some(s) = self.linkage(g, x) else {
                    continue;
                };
                if s < ask {
                    continue;
                }
                if self.groups[x].named {
                    named.push((x, s));
                } else {
                    unnamed.push((x, s));
                }
            }
            let by_score = |v: &mut Vec<(usize, f32)>, groups: &Vec<Group>| {
                v.sort_by(|a, b| {
                    b.1.total_cmp(&a.1)
                        .then_with(|| groups[a.0].key.cmp(&groups[b.0].key))
                })
            };
            by_score(&mut named, &self.groups);
            by_score(&mut unnamed, &self.groups);
            let size = self.groups[g].members.len();
            if let Some(&(p, s)) = named.first() {
                let second = named.get(1).copied();
                let (kind, third) = match second {
                    Some((q, s2)) if s - s2 < self.params.margin => {
                        (QuestionKind::GroupChoice, Some(self.groups[q].key))
                    }
                    _ => (QuestionKind::GroupPerson, None),
                };
                out.push(Question {
                    kind,
                    a: self.groups[g].key,
                    b: self.groups[p].key,
                    c: third,
                    score: s,
                    probability: self.probability(s),
                    faces: size,
                });
            }
            if let Some(&(x, s)) = unnamed.first() {
                let (a, b) = if size <= self.groups[x].members.len() {
                    (g, x)
                } else {
                    (x, g)
                };
                let key = (self.groups[a].key, self.groups[b].key);
                let rev = (key.1, key.0);
                if !pairs.contains(&key) && !pairs.contains(&rev) {
                    pairs.insert(key);
                    out.push(Question {
                        kind: QuestionKind::GroupGroup,
                        a: self.groups[a].key,
                        b: self.groups[b].key,
                        c: None,
                        score: s,
                        probability: self.probability(s),
                        faces: self.groups[a].members.len(),
                    });
                }
            }
        }
        order_questions(&mut out);
        out
    }

    fn outcome(self, questions: Vec<Question>) -> Outcome {
        let mut labels = vec![Label::Seed(0); self.faces.len()];
        let mut merges = Vec::new();
        for g in self.groups.iter().filter(|g| g.alive) {
            for m in &g.members {
                labels[*m] = g.key;
            }
            for p in &g.parts {
                if let Some(score) = p.score {
                    merges.push(Merge {
                        from: p.label,
                        into: g.key,
                        faces: p.faces.clone(),
                        score,
                        small: p.small,
                    });
                }
            }
        }
        merges.sort_by_key(|m| m.from);
        Outcome {
            labels,
            merges,
            questions,
        }
    }
}

/// Ordine della coda (design D6): prima le domande dei gruppi grandi (più di
/// [`SMALL_GROUP`] volti) per volti risolti attesi (probabilità × volti), poi quelle dei gruppi
/// piccoli per probabilità decrescente.
pub fn order_questions(qs: &mut [Question]) {
    qs.sort_by(|x, y| {
        let tier = |q: &Question| usize::from(q.faces <= SMALL_GROUP);
        tier(x).cmp(&tier(y)).then_with(|| {
            let key = |q: &Question| {
                if q.faces > SMALL_GROUP {
                    q.probability * q.faces as f32
                } else {
                    q.probability
                }
            };
            key(y)
                .total_cmp(&key(x))
                .then_with(|| y.score.total_cmp(&x.score))
                .then_with(|| x.a.cmp(&y.a))
                .then_with(|| x.b.cmp(&y.b))
        })
    });
}

/// Ordine per sola probabilità (confronto del banco di prova).
pub fn order_by_probability(qs: &mut [Question]) {
    qs.sort_by(|x, y| {
        y.probability
            .total_cmp(&x.probability)
            .then_with(|| y.score.total_cmp(&x.score))
            .then_with(|| x.a.cmp(&y.a))
            .then_with(|| x.b.cmp(&y.b))
    });
}

/// Raggruppamento completo: grafo, primo passaggio, secondo passaggio, gruppi piccoli e
/// domande. Non scrive nulla.
pub fn aggregate(
    faces: &[Face],
    people: &People,
    params: Params,
    mode: Mode,
    control: &JobControl,
) -> AppResult<Outcome> {
    let graph = knn_graph(faces, people, params.ask, control)?;
    let (labels, fixed, fresh) = initial_labels(faces, people, mode);
    let active = match mode {
        Mode::Full => None,
        Mode::Incremental => {
            let adj = undirected(&graph);
            let mut active = fresh.clone();
            for (i, f) in fresh.iter().enumerate() {
                if *f {
                    for j in &adj[i] {
                        active[*j] = true;
                    }
                }
            }
            Some(active)
        }
    };
    let first = chinese_whispers(faces, &graph, &fixed, labels.clone(), active, control)?;
    // Volti toccati: nuovi o con l'etichetta cambiata dal primo passaggio.
    let touched: Vec<bool> = (0..faces.len())
        .map(|i| fresh[i] || first[i] != labels[i])
        .collect();
    let mut agg = Aggregator::new(faces, people, params, mode, &graph, &first, &touched);
    agg.second_pass(control)?;
    agg.small_groups(control)?;
    let questions = agg.questions();
    Ok(agg.outcome(questions))
}

/// Collegamento robusto tra due insiemi di volti (per i test e il banco di prova).
pub fn linkage_of(faces: &[Face], a: &[usize], b: &[usize]) -> Option<f32> {
    let pick = |m: &[usize]| -> Vec<&[f32]> {
        pick_samples(faces, m)
            .into_iter()
            .map(|i| faces[i].embedding.as_slice())
            .collect()
    };
    robust_linkage(&pick(a), &pick(b))
}
