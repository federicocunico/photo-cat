//! Rivalutazione delle soglie senza eseguire i modelli ("retune") e anteprima
//! dell'effetto. Le catalogazioni CLIP si ricalcolano dagli embedding salvati
//! e dalla cache dei concetti testuali, i conteggi delle persone dai riquadri
//! salvati, i raggruppamenti dai volti salvati. Le correzioni manuali restano.

use crate::analyze::{AiDeps, LEGACY_FACE_FLOOR, LEGACY_PERSON_FLOOR};
use crate::stages::model_key;
use crate::{classify, taxonomy::Taxonomy, text_cache::Concept};
use pc_core::{AiThresholds, AppError, AppResult, ErrorCode};
use pc_jobs::{JobControl, JobPlan, PhaseCounters, PhaseSpec, RunSummary};
use pc_models::Role;
use pc_store::{analysis, people, Store};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const JOB_KIND: &str = "retune";
pub const PHASE_CLIP: &str = "clip";
pub const PHASE_PERSONS: &str = "persons";
pub const PHASE_FACES: &str = "faces";
/// Media rivalutati per blocco (embedding letti e scritti insieme).
const BLOCK: i64 = 500;

/// Fasi del retune da eseguire (parametri del job).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase", default)]
pub struct RetuneSpec {
    /// Soglie CLIP (tassonomia) cambiate.
    pub clip: bool,
    /// Soglia di rilevamento persone cambiata.
    pub persons: bool,
    /// Soglie di rilevamento o somiglianza dei volti cambiate.
    pub faces: bool,
}

impl RetuneSpec {
    /// Fasi necessarie per passare dalle soglie `prev` a `next` della
    /// configurazione (le soglie CLIP stanno nella tassonomia).
    pub fn between(prev: &AiThresholds, next: &AiThresholds) -> Self {
        Self {
            clip: false,
            persons: prev.person_detection != next.person_detection,
            faces: prev.face_detection != next.face_detection
                || prev.face_similarity != next.face_similarity,
        }
    }
    pub fn is_empty(&self) -> bool {
        !self.clip && !self.persons && !self.faces
    }
    pub fn merge(self, other: Self) -> Self {
        Self {
            clip: self.clip || other.clip,
            persons: self.persons || other.persons,
            faces: self.faces || other.faces,
        }
    }
    pub fn params(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }
}

/// Effetto di nuove soglie sulla libreria, calcolato dai risultati salvati.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ThresholdImpact {
    /// Foto le cui catalogazioni automatiche cambiano.
    pub media_labels_changed: i64,
    /// Foto il cui conteggio delle persone cambia.
    pub media_people_changed: i64,
    /// Volti esclusi dal riconoscimento con la nuova soglia di rilevamento.
    pub faces_excluded: i64,
    /// Stima delle persone senza nome che cambierebbero raggruppamento.
    pub persons_affected_estimate: i64,
    /// File salvati con un minimo di persone più alto della nuova soglia.
    pub reanalysis_persons: i64,
    /// File salvati con un minimo di volti più alto della nuova soglia.
    pub reanalysis_faces: i64,
}

/// Concetti testuali e modello immagine per rivalutare le catalogazioni.
pub struct ClipInputs<'a> {
    pub taxonomy: &'a Taxonomy,
    pub concepts: &'a [Concept],
    /// Versione del modello immagine con cui sono salvati gli embedding.
    pub model: &'a str,
}

fn store_err(e: pc_store::StoreError) -> AppError {
    e.into()
}

/// Rivaluta le catalogazioni CLIP di tutti i media con embedding. Con `write`
/// scrive le nuove etichette e marca i media con la versione corrente della
/// tassonomia (saltando quelli già aggiornati, così il job è riprendibile);
/// senza `write` conta soltanto. Restituisce i media con etichette cambiate.
pub fn relabel_all(
    store: &Store,
    clip: &ClipInputs,
    write: bool,
    control: &JobControl,
    phase: Option<&PhaseCounters>,
) -> AppResult<i64> {
    let groups: Arc<Vec<String>> =
        Arc::new(clip.taxonomy.groups.iter().map(|g| g.id.clone()).collect());
    let version = clip.taxonomy.version;
    let model = clip.model.to_string();
    if let Some(p) = phase {
        let m = model.clone();
        p.set_total(store.read(move |c| analysis::embedding_count(c, &m))?);
        p.start();
    }
    let mut changed = 0i64;
    let mut after = 0i64;
    loop {
        if !control.checkpoint() {
            return Err(AppError::cancelled());
        }
        let m = model.clone();
        let page = store.read(move |c| analysis::embeddings(c, &m, after, BLOCK))?;
        let Some(last) = page.last().map(|(id, _)| *id) else {
            break;
        };
        after = last;
        let n = page.len() as i64;
        let ids: Vec<i64> = page.iter().map(|(id, _)| *id).collect();
        let todo: std::collections::HashSet<i64> = if write {
            let m = model.clone();
            store
                .read(move |c| analysis::pending(c, &ids, "clip", &m, Some(version)))?
                .into_iter()
                .collect()
        } else {
            ids.into_iter().collect()
        };
        let results: Vec<_> = page
            .into_iter()
            .filter(|(id, _)| todo.contains(id))
            .map(|(id, v)| (id, classify::labels(&v, clip.taxonomy, clip.concepts)))
            .collect();
        let (g, m) = (groups.clone(), model.clone());
        let relabel = move |c: &rusqlite::Connection| -> pc_store::StoreResult<i64> {
            let mut k = 0;
            for (id, labels) in &results {
                if analysis::relabel(c, *id, &g, labels, &m, version, write)? {
                    k += 1;
                }
            }
            Ok(k)
        };
        changed += if write {
            store.write(relabel)?
        } else {
            store.read(relabel)?
        };
        if let Some(p) = phase {
            p.inc_done(n);
        }
    }
    if let Some(p) = phase {
        p.finish();
    }
    Ok(changed)
}

/// Anteprima: effetto di `next` (e della tassonomia di `clip`, se presente)
/// rispetto alle soglie in vigore, senza scrivere.
pub fn preview(
    store: &Store,
    clip: Option<&ClipInputs>,
    current: &AiThresholds,
    next: &AiThresholds,
    control: &JobControl,
) -> AppResult<ThresholdImpact> {
    let media_labels_changed = match clip {
        Some(c) => relabel_all(store, c, false, control, None)?,
        None => 0,
    };
    let next = *next;
    let current = *current;
    let rest = store
        .read(move |c| {
            Ok((
                analysis::recount_people(c, next.person_detection, false)?.len() as i64,
                people::faces_below(c, next.face_detection)?,
                if next.face_similarity > current.face_similarity {
                    people::persons_below_similarity(c, next.face_similarity)?
                } else {
                    0
                },
                analysis::below_floor(c, "persons", next.person_detection, LEGACY_PERSON_FLOOR)?
                    .len() as i64,
                analysis::below_floor(c, "faces", next.face_detection, LEGACY_FACE_FLOOR)?.len()
                    as i64,
            ))
        })
        .map_err(store_err)?;
    Ok(ThresholdImpact {
        media_labels_changed,
        media_people_changed: rest.0,
        faces_excluded: rest.1,
        persons_affected_estimate: rest.2,
        reanalysis_persons: rest.3,
        reanalysis_faces: rest.4,
    })
}

/// File da rianalizzare perché salvati con un minimo più alto delle soglie
/// `t`: il loro stadio viene marcato "da rianalizzare" e se ne restituiscono
/// gli id, da passare al job di analisi.
pub fn prepare_reanalysis(store: &Store, t: &AiThresholds) -> AppResult<Vec<i64>> {
    let t = *t;
    Ok(store.write(move |c| {
        let persons = analysis::below_floor(c, "persons", t.person_detection, LEGACY_PERSON_FLOOR)?;
        let faces = analysis::below_floor(c, "faces", t.face_detection, LEGACY_FACE_FLOOR)?;
        analysis::forget(c, &persons, "persons")?;
        analysis::forget(c, &faces, "faces")?;
        let mut ids: Vec<i64> = persons.into_iter().chain(faces).collect();
        ids.sort_unstable();
        ids.dedup();
        Ok(ids)
    })?)
}

/// Esito di [`apply`] per fase (stessi conteggi dell'anteprima).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Applied {
    pub media_labels_changed: i64,
    pub media_people_changed: i64,
}

/// Fasi del retune con le risorse già pronte: usato dal job e dai test.
#[allow(clippy::too_many_arguments)]
pub fn apply(
    store: &Store,
    governor: &pc_governor::MemoryGovernor,
    spec: RetuneSpec,
    clip: Option<&ClipInputs>,
    thresholds: &AiThresholds,
    policy: Arc<crate::face_policy::FacePolicy>,
    refs: &people::IdentityRefs,
    control: &JobControl,
    progress: Option<&pc_jobs::JobProgress>,
) -> AppResult<Applied> {
    let phase = |name: &str| progress.map(|p| p.phase(name));
    let mut out = Applied::default();
    match clip.filter(|_| spec.clip) {
        Some(c) => {
            let p = phase(PHASE_CLIP);
            out.media_labels_changed = relabel_all(store, c, true, control, p.as_deref())?;
        }
        None => finish_empty(phase(PHASE_CLIP)),
    }
    if spec.persons {
        let p = phase(PHASE_PERSONS);
        if let Some(p) = &p {
            p.set_total(1);
            p.start();
        }
        let t = thresholds.person_detection;
        out.media_people_changed = store
            .write(move |c| analysis::recount_people(c, t, true))?
            .len() as i64;
        if let Some(p) = p {
            p.inc_done(1);
            p.finish();
        }
    } else {
        finish_empty(phase(PHASE_PERSONS));
    }
    if spec.faces {
        let p = phase(PHASE_FACES);
        if let Some(p) = &p {
            p.set_total(1);
            p.start();
        }
        crate::cluster::run(store, governor, control, policy, refs, thresholds)?;
        if let Some(p) = p {
            p.inc_done(1);
            p.finish();
        }
    } else {
        finish_empty(phase(PHASE_FACES));
    }
    Ok(out)
}

fn finish_empty(p: Option<Arc<PhaseCounters>>) {
    if let Some(p) = p {
        p.set_total(0);
        p.finish();
    }
}

/// Tassonomia corrente, concetti testuali e versione del modello immagine.
pub type ClipContext = (Arc<Taxonomy>, Arc<Vec<Concept>>, String);

/// Concetti testuali e versione del modello immagine correnti, se CLIP è
/// installato. Con la cache valida l'encoder di testo non viene caricato;
/// il modello immagine non viene mai caricato.
pub fn clip_context(deps: &AiDeps) -> AppResult<Option<ClipContext>> {
    let (clip, _) = deps.ready_models();
    if clip.is_none() {
        return Ok(None);
    }
    let model = model_key(deps.resources.entry(Role::ClassificationImage)?);
    let (tax, concepts) = deps.concepts()?;
    Ok(Some((tax, concepts, model)))
}

/// Job "retune" (parametri: [`RetuneSpec`]).
pub fn plan(deps: AiDeps, params: &serde_json::Value) -> AppResult<JobPlan> {
    let spec: RetuneSpec = serde_json::from_value(params.clone())
        .map_err(|e| AppError::invalid_input(format!("parametri retune: {e}")))?;
    Ok(JobPlan {
        phases: vec![
            PhaseSpec::new(PHASE_CLIP, 3.0),
            PhaseSpec::new(PHASE_PERSONS, 0.5),
            PhaseSpec::new(PHASE_FACES, 1.5),
        ],
        min_memory_mb: 64,
        run: Box::new(move |ctx| {
            // Non si sovrappone a un'analisi che scrive gli stessi risultati.
            let _serial = loop {
                if let Some(g) = deps.analysis_lock.try_lock() {
                    break g;
                }
                if !ctx.control.checkpoint() {
                    return Ok(RunSummary {
                        cancelled: true,
                        failed: 0,
                    });
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            };
            let clip = if spec.clip {
                clip_context(&deps)?
            } else {
                None
            };
            let inputs = clip.as_ref().map(|(t, c, m)| ClipInputs {
                taxonomy: t,
                concepts: c,
                model: m,
            });
            let thresholds = deps.config.read().ai.thresholds;
            let refs = match (&deps.app_store, spec.faces) {
                (Some(app), true) => crate::face_analysis::versions(&deps.resources)
                    .ok()
                    .and_then(|(emb, _)| people::IdentityRefs::load(app, &emb).ok())
                    .unwrap_or_default(),
                _ => Default::default(),
            };
            let res = apply(
                &deps.store,
                &deps.resources.governor,
                spec,
                inputs.as_ref(),
                &thresholds,
                deps.face_policy.clone(),
                &refs,
                &ctx.control,
                Some(&ctx.progress),
            );
            match res {
                Ok(a) => {
                    tracing::info!(
                        labels = a.media_labels_changed,
                        people = a.media_people_changed,
                        "soglie applicate senza rianalisi"
                    );
                    Ok(RunSummary {
                        cancelled: false,
                        failed: 0,
                    })
                }
                Err(e) if e.code == ErrorCode::Cancelled => Ok(RunSummary {
                    cancelled: true,
                    failed: 0,
                }),
                Err(e) => Err(e),
            }
        }),
    })
}
