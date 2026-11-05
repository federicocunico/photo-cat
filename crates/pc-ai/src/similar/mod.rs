//! Gruppi di foto quasi identiche (raffiche, scatti ripetuti).
//!
//! Per ogni foto da elaborare:
//! 1. candidati semantici (se c'è un modello CLIP): i vicini nell'embedding con coseno ≥ [`COSINE_MIN`];
//! 2. candidati globali: impronta percettiva a distanza di Hamming ≤ [`HAMMING_NEAR`], senza
//!    vincoli di data o cartella (copie ridimensionate, ricompresse, esportate);
//! 3. candidati temporali: stesso dispositivo (o, se ignoto, stessa cartella) entro
//!    ±[`BURST_WINDOW_MS`], con coseno ≥ [`COSINE_BURST`] oppure Hamming ≤ [`HAMMING_BURST`];
//!    una candidata non esatta con Hamming > [`HAMMING_REJECT`] è scartata prima della SSIM;
//! 4. copie esatte (stesso contenuto): sempre nello stesso gruppo, senza regole di Hamming;
//! 5. conferma con SSIM sulle miniature a 128 px; gli archi (anche sotto soglia, fino al
//!    pavimento) restano in `similar_edge`, così un cambio di soglia non riapre le miniature;
//! 6. gruppi con union-find e vincoli manuali ([`group`]).
//!
//! L'analisi è incrementale (`similar_done`) e annullabile; l'indice dei vicini usa memoria
//! concessa dal governor, altrimenti si scandiscono gli embedding a blocchi.

pub mod group;
pub mod measure;

use group::{build_groups, pair, Constraints, Link};
use measure::{exposure, sharpness, ssim, SsimThumb};
use pc_core::{AppError, AppResult};
use pc_governor::MemoryGovernor;
use pc_ingest::phash::hamming;
use pc_ingest::thumb::ThumbCache;
use pc_jobs::{JobControl, PhaseCounters};
use pc_store::model::MediaId;
use pc_store::{analysis, similar, Store};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Vicini semantici considerati per foto.
pub const NEIGHBOURS: usize = 12;
/// Coseno minimo tra gli embedding CLIP per un candidato semantico.
pub const COSINE_MIN: f32 = 0.92;
/// Coseno minimo per un candidato della stessa raffica.
pub const COSINE_BURST: f32 = 0.80;
/// Finestra di una raffica (stesso dispositivo).
pub const BURST_WINDOW_MS: i64 = 10_000;
/// Distanza di Hamming massima per un candidato globale.
pub const HAMMING_NEAR: u32 = 10;
/// Vicini globali tenuti per foto (le immagini uniformi hanno tutte impronta 0: evita O(k²) in SSIM).
pub const HAMMING_NEAR_MAX: usize = 32;
/// Distanza di Hamming massima perché una candidata della raffica passi senza CLIP.
pub const HAMMING_BURST: u32 = 22;
/// Oltre questa distanza una candidata non esatta è scartata prima della SSIM.
pub const HAMMING_REJECT: u32 = 40;
/// Oltre questo numero di foto nuove si costruisce l'indice HNSW; sotto, scansione esatta.
pub const INDEX_FROM: usize = 2_000;
/// Embedding letti per blocco nella scansione esatta.
const SCAN_BLOCK: i64 = 5_000;
/// Miniature a 128 px tenute in memoria.
const THUMB_CACHE: usize = 4_000;
/// Foto elaborate tra due scritture.
const WRITE_EVERY: usize = 200;

/// Chiave dei parametri del calcolo: cambiando modello o parametri le foto si rielaborano.
pub fn params_key(model: Option<&str>) -> String {
    let model = model.unwrap_or("none");
    format!(
        "v2;k{NEIGHBOURS};cos{COSINE_MIN};burst{COSINE_BURST}@{BURST_WINDOW_MS};h{HAMMING_NEAR}/{HAMMING_BURST}/{HAMMING_REJECT};{model}"
    )
}

/// Candidati globali di una foto: gli indici (in `ids`/`hashes`) a distanza ≤ [`HAMMING_NEAR`],
/// esclusa la foto stessa, i [`HAMMING_NEAR_MAX`] più vicini (a parità, id minore).
/// `scratch` è un buffer riusabile tra le chiamate.
pub fn global_candidates(
    ids: &[MediaId],
    hashes: &[u64],
    me: MediaId,
    hash: u64,
    scratch: &mut Vec<usize>,
) -> Vec<MediaId> {
    scratch.clear();
    pc_ingest::phash::near(hashes, hash, HAMMING_NEAR, scratch);
    let mut found: Vec<(u32, MediaId)> = scratch
        .iter()
        .filter(|&&i| ids[i] != me)
        .map(|&i| (pc_ingest::phash::hamming(hashes[i], hash), ids[i]))
        .collect();
    if found.len() > HAMMING_NEAR_MAX {
        found.select_nth_unstable(HAMMING_NEAR_MAX - 1);
        found.truncate(HAMMING_NEAR_MAX);
    }
    found.sort_unstable();
    found.into_iter().map(|(_, id)| id).collect()
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let (mut dot, mut na, mut nb) = (0.0f32, 0.0f32, 0.0f32);
    for (x, y) in a.iter().zip(b) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na.sqrt() * nb.sqrt())
    }
}

/// Vicini semantici delle foto `targets` (id → [(vicino, coseno)]). Con molte foto nuove usa un
/// indice HNSW (memoria chiesta al governor), altrimenti una scansione esatta a blocchi.
pub fn semantic_neighbours(
    store: &Store,
    gov: &MemoryGovernor,
    model: &str,
    targets: &[MediaId],
    control: &JobControl,
) -> AppResult<HashMap<MediaId, Vec<(MediaId, f32)>>> {
    let target_vecs = store.read(|c| similar::embeddings_of(c, model, targets))?;
    let total = store.read(|c| analysis::embedding_count(c, model))? as usize;
    let dims = target_vecs.values().next().map_or(512, Vec::len);
    // Indice in memoria per molte foto nuove (prima analisi), se il governor concede il budget.
    let index_mb = ((total * dims * 4) as u64 * 3 / 2)
        .div_ceil(1024 * 1024)
        .max(1);
    let permit = (targets.len() >= INDEX_FROM)
        .then(|| gov.acquire_timeout(index_mb, Some(std::time::Duration::from_secs(5))))
        .flatten();
    let mut out: HashMap<MediaId, Vec<(MediaId, f32)>> = HashMap::new();
    if permit.is_some() {
        use hnsw_rs::prelude::*;
        let mut all: Vec<(MediaId, Vec<f32>)> = Vec::with_capacity(total);
        let mut after = 0;
        loop {
            if !control.checkpoint() {
                return Err(AppError::cancelled());
            }
            let page = store.read(|c| analysis::embeddings(c, model, after, SCAN_BLOCK))?;
            let Some(last) = page.last().map(|p| p.0) else {
                break;
            };
            after = last;
            all.extend(page);
        }
        let h = Hnsw::<f32, DistCosine>::new(16, all.len().max(1), 16, 100, DistCosine {});
        for (i, (_, v)) in all.iter().enumerate() {
            h.insert((v.as_slice(), i));
        }
        for (id, v) in &target_vecs {
            if !control.checkpoint() {
                return Err(AppError::cancelled());
            }
            let found = h
                .search(v, (NEIGHBOURS + 1).min(all.len()), 64)
                .into_iter()
                .map(|n| (all[n.d_id].0, 1.0 - n.distance))
                .filter(|(n, s)| n != id && *s >= COSINE_MIN)
                .take(NEIGHBOURS)
                .collect();
            out.insert(*id, found);
        }
        return Ok(out);
    }
    // Scansione esatta a blocchi: memoria limitata a un blocco più i vicini migliori.
    let mut after = 0;
    loop {
        if !control.checkpoint() {
            return Err(AppError::cancelled());
        }
        let page = store.read(|c| analysis::embeddings(c, model, after, SCAN_BLOCK))?;
        let Some(last) = page.last().map(|p| p.0) else {
            break;
        };
        after = last;
        for (id, v) in &target_vecs {
            let best = out.entry(*id).or_default();
            for (other, w) in &page {
                if other == id {
                    continue;
                }
                let s = cosine(v, w);
                if s >= COSINE_MIN {
                    best.push((*other, s));
                }
            }
            if best.len() > NEIGHBOURS * 4 {
                best.sort_by(|a, b| b.1.total_cmp(&a.1));
                best.truncate(NEIGHBOURS);
            }
        }
    }
    for best in out.values_mut() {
        best.sort_by(|a, b| b.1.total_cmp(&a.1));
        best.truncate(NEIGHBOURS);
    }
    Ok(out)
}

/// Impronta di una foto dall'indice globale (ordinato per id).
fn phash_of(ids: &[MediaId], hashes: &[u64], id: MediaId) -> Option<u64> {
    ids.binary_search(&id).ok().map(|i| hashes[i])
}

/// Miniature a 128 px in scala di grigi, con un limite di elementi in memoria.
struct ThumbLoader {
    cache: ThumbCache,
    keys: HashMap<MediaId, Option<String>>,
    loaded: HashMap<MediaId, Option<Arc<SsimThumb>>>,
    order: std::collections::VecDeque<MediaId>,
}

impl ThumbLoader {
    fn rgb(&self, id: MediaId) -> Option<image::RgbImage> {
        let key = self.keys.get(&id)?.as_ref()?;
        let path = self.cache.path_of(key)?;
        image::open(path).ok().map(|i| i.to_rgb8())
    }

    fn get(&mut self, store: &Store, id: MediaId) -> AppResult<Option<Arc<SsimThumb>>> {
        if let Some(t) = self.loaded.get(&id) {
            return Ok(t.clone());
        }
        if let std::collections::hash_map::Entry::Vacant(slot) = self.keys.entry(id) {
            let f = store.read(|c| similar::facts(c, &[id]))?;
            slot.insert(f.get(&id).and_then(|f| f.thumb.clone()));
        }
        let t = self.rgb(id).map(|i| Arc::new(SsimThumb::from_rgb(&i)));
        if self.order.len() >= THUMB_CACHE {
            if let Some(old) = self.order.pop_front() {
                self.loaded.remove(&old);
            }
        }
        self.order.push_back(id);
        self.loaded.insert(id, t.clone());
        Ok(t)
    }
}

/// Esito della fase `similar`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RunOutcome {
    pub processed: usize,
    pub comparisons: usize,
    pub groups: similar::ApplyOutcome,
}

/// Elabora le foto non ancora fatte con i parametri correnti, poi ricalcola i gruppi.
#[allow(clippy::too_many_arguments)]
pub fn run(
    store: &Store,
    thumbs: &ThumbCache,
    gov: &MemoryGovernor,
    control: &JobControl,
    model: Option<&str>,
    threshold: f32,
    phase: Option<&PhaseCounters>,
) -> AppResult<RunOutcome> {
    let key = params_key(model);
    // Chiave per le foto senza embedding del modello: non perdono i vicini CLIP futuri.
    let keys = (key.clone(), params_key(None));
    let (m, k) = (model.map(str::to_string), keys.clone());
    let todo = store.read(move |c| similar::pending(c, m.as_deref(), &k.0, &k.1))?;
    if let Some(p) = phase {
        p.set_total(todo.len() as i64);
        p.start();
    }
    let mut outcome = RunOutcome::default();
    if !todo.is_empty() {
        let neighbours = match model {
            Some(m) => semantic_neighbours(store, gov, m, &todo, control)?,
            None => HashMap::new(),
        };
        let (ph_ids, ph_hashes) = store.read(similar::phashes)?;
        let mut scratch = Vec::new();
        let facts = {
            let ids = todo.clone();
            store.read(move |c| similar::facts(c, &ids))?
        };
        let mut loader = ThumbLoader {
            cache: thumbs.clone(),
            keys: facts.iter().map(|(id, f)| (*id, f.thumb.clone())).collect(),
            loaded: HashMap::new(),
            order: Default::default(),
        };
        let mut edges: Vec<(MediaId, MediaId, f32)> = Vec::new();
        let mut done: Vec<(MediaId, f32, f32, bool)> = Vec::new();
        let mut seen: HashSet<(MediaId, MediaId)> = HashSet::new();
        for (i, id) in todo.iter().enumerate() {
            if !control.checkpoint() {
                flush(store, &keys, &mut edges, &mut done)?;
                return Err(AppError::cancelled());
            }
            let f = facts.get(id).cloned().unwrap_or_default();
            let mut candidates: Vec<MediaId> = neighbours
                .get(id)
                .map(|v| v.iter().map(|(n, _)| *n).collect())
                .unwrap_or_default();
            // Globale: impronta percettiva vicina, a prescindere da data e cartella.
            if let Some(h) = f.phash {
                candidates.extend(global_candidates(&ph_ids, &ph_hashes, *id, h, &mut scratch));
            }
            // Raffica: stesso dispositivo (o stessa cartella, se ignoto) entro la finestra;
            // passa con coseno permissivo oppure con impronta abbastanza vicina.
            if let Some(t) = f.instant_utc {
                let (dev, path, idc) = (f.device.clone(), f.path.clone(), *id);
                let near = store.read(move |c| {
                    similar::burst_neighbours(c, idc, dev.as_deref(), &path, t, BURST_WINDOW_MS)
                })?;
                if !near.is_empty() {
                    let vecs = match model {
                        Some(m) => {
                            let (m, mut ids) = (m.to_string(), near.clone());
                            ids.push(*id);
                            store.read(move |c| similar::embeddings_of(c, &m, &ids))?
                        }
                        None => HashMap::new(),
                    };
                    candidates.extend(near.into_iter().filter(|n| {
                        let by_clip = vecs
                            .get(id)
                            .zip(vecs.get(n))
                            .is_some_and(|(v, w)| cosine(v, w) >= COSINE_BURST);
                        by_clip
                            || f.phash
                                .zip(phash_of(&ph_ids, &ph_hashes, *n))
                                .is_some_and(|(a, b)| hamming(a, b) <= HAMMING_BURST)
                    }));
                }
            }
            // Scarto: impronte troppo lontane non si confrontano nemmeno (le copie esatte no).
            if let Some(h) = f.phash {
                candidates.retain(|n| {
                    phash_of(&ph_ids, &ph_hashes, *n)
                        .is_none_or(|b| hamming(h, b) <= HAMMING_REJECT)
                });
            }
            // Copie esatte: nello stesso gruppo senza altre verifiche.
            if let Some(h) = &f.content_hash {
                let (h, idc) = (h.clone(), *id);
                for copy in store.read(move |c| similar::exact_copies(c, idc, &h))? {
                    if seen.insert(pair(*id, copy)) {
                        edges.push((*id, copy, 1.0));
                    }
                }
            }
            let rgb = loader.rgb(*id);
            let (sharp, expo) = rgb
                .as_ref()
                .map_or((0.0, 1.0), |img| (sharpness(img), exposure(img)));
            if let Some(me) = loader.get(store, *id)? {
                candidates.sort_unstable();
                candidates.dedup();
                for other in candidates {
                    if !seen.insert(pair(*id, other)) {
                        continue;
                    }
                    let Some(them) = loader.get(store, other)? else {
                        continue;
                    };
                    outcome.comparisons += 1;
                    if let Some(s) = ssim(&me, &them) {
                        edges.push((*id, other, s));
                    }
                }
            }
            // Senza embedding: fatta con la chiave senza modello (ripresa quando arriva).
            done.push((
                *id,
                sharp,
                expo,
                model.is_none() || neighbours.contains_key(id),
            ));
            outcome.processed += 1;
            if let Some(p) = phase {
                p.inc_done(1);
            }
            if i % WRITE_EVERY == WRITE_EVERY - 1 {
                flush(store, &keys, &mut edges, &mut done)?;
            }
        }
        flush(store, &keys, &mut edges, &mut done)?;
    }
    outcome.groups = regroup(store, threshold, &key)?;
    if let Some(p) = phase {
        p.finish();
    }
    tracing::info!(
        processed = outcome.processed,
        comparisons = outcome.comparisons,
        created = outcome.groups.created,
        updated = outcome.groups.updated,
        removed = outcome.groups.removed,
        "gruppi di foto simili aggiornati"
    );
    Ok(outcome)
}

fn flush(
    store: &Store,
    keys: &(String, String),
    edges: &mut Vec<(MediaId, MediaId, f32)>,
    done: &mut Vec<(MediaId, f32, f32, bool)>,
) -> AppResult<()> {
    let (e, d, k) = (std::mem::take(edges), std::mem::take(done), keys.clone());
    store.write(move |c| {
        similar::save_edges(c, &e)?;
        for (id, s, x, full) in &d {
            let key = if *full { &k.0 } else { &k.1 };
            similar::mark_done(c, *id, key, *s, *x)?;
        }
        Ok(())
    })?;
    Ok(())
}

fn load_constraints(store: &Store) -> AppResult<Constraints> {
    let (links, dissolved) = store.read(similar::constraints)?;
    let mut c = Constraints {
        links: links
            .into_iter()
            .map(|(a, b, must, at)| Link { a, b, must, at })
            .collect(),
        ..Default::default()
    };
    for (m, t) in dissolved {
        c.dissolved.entry(m).or_default().push(t);
    }
    Ok(c)
}

/// Gruppi calcolati dagli archi salvati e dai vincoli (nessuna miniatura, nessun modello).
pub fn compute_groups(store: &Store, threshold: f32) -> AppResult<Vec<Vec<MediaId>>> {
    let edges = store.read(similar::edges)?;
    let constraints = load_constraints(store)?;
    Ok(build_groups(&edges, threshold, &constraints))
}

/// Ricalcola e salva i gruppi (dopo l'analisi, una correzione o un cambio di soglia).
pub fn regroup(store: &Store, threshold: f32, key: &str) -> AppResult<similar::ApplyOutcome> {
    let groups = compute_groups(store, threshold)?;
    let k = key.to_string();
    Ok(store.write(move |c| similar::apply_groups(c, &groups, &k))?)
}

/// Anteprima per le Impostazioni: quanti gruppi si formerebbero con `threshold`.
pub fn preview_count(store: &Store, threshold: f32) -> AppResult<usize> {
    Ok(compute_groups(store, threshold)?.len())
}

/// Job autonomo "gruppi di foto simili": per le librerie già analizzate prima di questa
/// funzione (proposto all'utente, non avviato da solo).
pub const JOB_KIND: &str = "similar";

/// Modello CLIP per i gruppi: `Some(chiave)` se la voce di classificazione è presente su
/// disco e non è nota come guasta (errore di verifica o self-test fallito), altrimenti `None`
/// (solo impronte percettive). All'avvio lo stato è `Missing` finché non gira `check_all`:
/// contare solo i "verificati" darebbe una chiave `none` transitoria e un ricalcolo inutile.
/// Unico punto di risoluzione: la chiave dei parametri resta uguale tra conteggio, job
/// autonomo e fase dell'analisi.
pub fn resolve_model(resources: &crate::stages::AiResources) -> Option<String> {
    use pc_models::store::ModelState;
    let entry = resources.entry(pc_models::Role::ClassificationImage).ok()?;
    resources.models.installed_path(&entry.id)?;
    let broken = resources.models.statuses().iter().any(|s| {
        s.entry.id == entry.id
            && matches!(
                s.state,
                ModelState::Error { .. } | ModelState::SelfTestFailed { .. }
            )
    });
    (!broken).then(|| crate::stages::model_key(entry))
}

/// Foto ancora da elaborare per i gruppi con il modello corrente.
pub fn pending_count(deps: &crate::analyze::AiDeps) -> AppResult<usize> {
    let model = resolve_model(&deps.resources);
    let (key, phash_key) = (params_key(model.as_deref()), params_key(None));
    Ok(deps
        .store
        .read(move |c| similar::pending(c, model.as_deref(), &key, &phash_key))?
        .len())
}

/// Avvolge un job (import, recupero impronte): a lavoro concluso accoda `similar` se ci sono
/// foto in attesa e non c'è già un `similar`/`analysis` in coda, in corso o in pausa. Un
/// followup già deciso dal job (es. `analysis` dopo l'import, che include i gruppi) resta.
pub fn with_similar_followup(
    deps: crate::analyze::AiDeps,
    plan: pc_jobs::JobPlan,
) -> pc_jobs::JobPlan {
    let inner = plan.run;
    pc_jobs::JobPlan {
        run: Box::new(move |ctx| {
            let followup = ctx.followup.clone();
            let summary = inner(ctx)?;
            if !summary.cancelled && followup.lock().is_none() {
                let busy = pc_jobs::persist::any_in_state(
                    &deps.store,
                    &[JOB_KIND, crate::analyze::JOB_KIND],
                    &["queued", "running", "paused"],
                )?;
                if !busy && pending_count(&deps)? > 0 {
                    *followup.lock() = Some((JOB_KIND.into(), serde_json::json!({})));
                }
            }
            Ok(summary)
        }),
        ..plan
    }
}

pub fn plan(
    deps: crate::analyze::AiDeps,
    _params: &serde_json::Value,
) -> AppResult<pc_jobs::JobPlan> {
    Ok(pc_jobs::JobPlan {
        phases: vec![pc_jobs::PhaseSpec::new(crate::analyze::PHASE_SIMILAR, 1.0)],
        min_memory_mb: 128,
        run: Box::new(move |ctx| {
            let phase = ctx.progress.phase(crate::analyze::PHASE_SIMILAR);
            match crate::analyze::run_similar(&deps, &ctx.control, &phase) {
                Ok(()) => Ok(pc_jobs::RunSummary {
                    cancelled: false,
                    failed: 0,
                }),
                Err(e) if e.code == pc_core::ErrorCode::Cancelled => Ok(pc_jobs::RunSummary {
                    cancelled: true,
                    failed: 0,
                }),
                Err(e) => Err(e),
            }
        }),
    })
}
