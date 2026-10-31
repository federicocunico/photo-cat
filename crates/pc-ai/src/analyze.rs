//! Collegamento dell'AI ai job: stadi inline nell'import quando i modelli stanno
//! insieme nel budget, altrimenti job "analisi" eseguito un modello alla volta
//! (model-major). Lo stesso job serve per la rianalisi.

use crate::geo::Gazetteer;
use crate::schedule::{run_model_major, ModelPass};
use crate::stages::{model_key, normalized_box, AiResources, ClipStage, GeoStage, PersonsStage};
use crate::{classify, persons, taxonomy, text_cache};
use parking_lot::{Mutex, RwLock};
use pc_core::{AppError, AppResult, Config, ErrorCode};
use pc_ingest::import::{StagePlan, ANALYSIS_SIZE};
use pc_ingest::thumb::{apply_orientation, decode_reduced, fit, Decoded};
use pc_jobs::{JobPlan, PhaseSpec, RunSummary};
use pc_models::Role;
use pc_store::analysis;
use pc_store::model::{MediaId, MediaStatus};
use pc_store::{repo, Store};
use std::sync::Arc;

pub const JOB_KIND: &str = "analysis";
pub const PHASE_CLIP: &str = "clip";
pub const PHASE_PERSONS: &str = "persons";
pub const PHASE_FACES: &str = "faces";
pub const PHASE_CLUSTER: &str = "cluster";
const PERSON_THRESHOLD: f32 = 0.4;

#[derive(Clone)]
pub struct AiDeps {
    pub resources: AiResources,
    pub store: Store,
    pub gazetteer: Arc<Mutex<Option<Arc<Gazetteer>>>>,
    pub config: Arc<RwLock<Config>>,
    pub face_policy: Arc<crate::face_policy::FacePolicy>,
    pub analysis_lock: Arc<Mutex<()>>,
}

impl AiDeps {
    /// Gazetteer caricato alla prima richiesta (se il file è verificato).
    pub fn gazetteer(&self) -> Option<Arc<Gazetteer>> {
        let mut slot = self.gazetteer.lock();
        if slot.is_none() {
            let entry = self.resources.models.manifest().by_role(Role::Gazetteer)?;
            let path = self.resources.models.verified_path(&entry.id)?;
            match Gazetteer::load(&path) {
                Ok(g) => *slot = Some(Arc::new(g)),
                Err(e) => tracing::warn!(
                    code = e.code.as_str(),
                    detail = e.detail.as_str(),
                    "gazetteer non caricato"
                ),
            }
        }
        slot.clone()
    }

    fn verified(&self, role: Role) -> Option<String> {
        let e = self.resources.models.manifest().by_role(role)?;
        self.resources
            .models
            .verified_path(&e.id)
            .map(|_| e.id.clone())
    }

    /// Modelli AI pronti: (classificazione immagine, rilevamento persone).
    pub fn ready_models(&self) -> (Option<String>, Option<String>) {
        let clip = self.verified(Role::ClassificationImage).filter(|_| {
            self.verified(Role::ClassificationText).is_some()
                && self.verified(Role::Tokenizer).is_some()
        });
        (clip, self.verified(Role::PersonDetection))
    }

    /// Embedding testuali della tassonomia (dalla cache o calcolati), poi scarica l'encoder di testo.
    pub fn concepts(&self) -> AppResult<(Arc<taxonomy::Taxonomy>, Arc<Vec<text_cache::Concept>>)> {
        let tax = taxonomy::load(&self.store)?;
        let res = text_cache::from_models(
            &self.store,
            &tax,
            &self.resources.models,
            &self.resources.pool,
        );
        Ok((Arc::new(tax), Arc::new(res?)))
    }

    pub fn ready_faces(&self) -> Option<Vec<String>> {
        if !self.config.read().ai.faces_enabled {
            return None;
        }
        Some(vec![
            self.verified(Role::FaceDetection)?,
            self.verified(Role::FaceEmbedding)?,
        ])
    }

    /// Modelli pronti che entrano nel budget di memoria. Una fase il cui modello (o gruppo di
    /// modelli caricati insieme) supera il budget viene disattivata con un avviso
    /// `INSUFFICIENT_MEMORY`; le altre fasi e l'import proseguono.
    pub fn runnable_models(&self) -> RunnableModels {
        let (clip, persons) = self.ready_models();
        let faces = self.ready_faces();
        let mut over_budget = false;
        let mut fits = |phase: &str, sets: Vec<Vec<String>>| {
            let need = sets
                .iter()
                .map(|s| {
                    s.iter()
                        .map(|id| self.resources.pool.estimate_mb(id))
                        .sum::<u64>()
                })
                .max()
                .unwrap_or(0);
            let budget = self.resources.governor.snapshot().budget_mb;
            if need <= budget {
                return true;
            }
            over_budget = true;
            tracing::warn!(
                code = ErrorCode::InsufficientMemory.as_str(),
                phase,
                detail = format!("servono ~{need} MB, budget {budget} MB").as_str(),
                "memoria insufficiente per la fase AI: fase disattivata"
            );
            false
        };
        // CLIP: encoder di testo (concetti) e di immagine sono caricati uno alla volta.
        let clip = clip.filter(|id| {
            let text = self.verified(Role::ClassificationText).unwrap_or_default();
            fits(PHASE_CLIP, vec![vec![id.clone()], vec![text]])
        });
        let persons = persons.filter(|id| fits(PHASE_PERSONS, vec![vec![id.clone()]]));
        // Volti: rilevamento ed embedding restano caricati insieme nella stessa passata.
        let faces = faces.filter(|ids| fits(PHASE_FACES, vec![ids.clone()]));
        RunnableModels {
            clip,
            persons,
            faces,
            over_budget,
        }
    }
}

/// Esito di [`AiDeps::runnable_models`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunnableModels {
    pub clip: Option<String>,
    pub persons: Option<String>,
    pub faces: Option<Vec<String>>,
    /// Almeno una fase pronta è stata disattivata per memoria insufficiente.
    pub over_budget: bool,
}

impl RunnableModels {
    pub fn is_empty(&self) -> bool {
        self.clip.is_none() && self.persons.is_none() && self.faces.is_none()
    }
}

pub fn import_factory(deps: AiDeps) -> pc_ingest::import::StageFactory {
    pc_ingest::import::StageFactory {
        phases: vec![
            PhaseSpec::new("geo", 0.5),
            PhaseSpec::new(PHASE_CLIP, 4.0),
            PhaseSpec::new(PHASE_PERSONS, 2.0),
            PhaseSpec::new(PHASE_FACES, 4.0),
        ],
        build: Arc::new(move |source| import_stages(&deps, source)),
    }
}

/// Stadi dell'import: geolocalizzazione sempre; CLIP e persone inline se i modelli
/// stanno insieme nel budget, altrimenti analisi rinviata a un job model-major.
pub fn import_stages(deps: &AiDeps, source_id: i64) -> StagePlan {
    let mut plan = StagePlan::default();
    plan.stages.push(Arc::new(GeoStage {
        gazetteer: deps.gazetteer(),
    }));
    let runnable = deps.runnable_models();
    let RunnableModels {
        clip,
        persons,
        faces,
        ..
    } = &runnable;
    let ids: Vec<&str> = clip
        .iter()
        .chain(persons.iter())
        .chain(faces.iter().flatten())
        .map(String::as_str)
        .collect();
    if ids.is_empty() && (runnable.over_budget || !deps.config.read().ai.auto_download_models) {
        tracing::info!(source_id, "modelli AI non pronti: import senza analisi");
        return plan;
    }
    let memory: u64 = ids
        .iter()
        .map(|id| deps.resources.pool.estimate_mb(id))
        .sum();
    // Dopo l'import completa anche modelli appena installati e clustering.
    plan.followup = Some((JOB_KIND.into(), serde_json::json!({"sourceId":source_id})));
    if ids.is_empty()
        || !deps.resources.pool.fit_together(&ids)
        || memory.saturating_add(384) > deps.resources.governor.capacity_mb()
    {
        tracing::info!(
            source_id,
            "modelli AI non contemporanei nel budget: analisi dopo l'import, un modello alla volta"
        );
        plan.followup = Some((
            JOB_KIND.into(),
            serde_json::json!({ "sourceId": source_id }),
        ));
        return plan;
    }
    if clip.is_some() {
        match deps.concepts() {
            Ok((taxonomy, concepts)) => plan.stages.push(Arc::new(ClipStage {
                resources: deps.resources.clone(),
                taxonomy,
                concepts,
            })),
            Err(e) => tracing::error!(
                code = e.code.as_str(),
                detail = e.detail.as_str(),
                "tassonomia non pronta: classificazione saltata"
            ),
        }
    }
    if persons.is_some() {
        plan.stages.push(Arc::new(PersonsStage {
            resources: deps.resources.clone(),
            threshold: PERSON_THRESHOLD,
        }));
    }
    if faces.is_some() {
        plan.stages.push(Arc::new(crate::face_analysis::FaceStage {
            resources: deps.resources.clone(),
            min_pixels: deps.config.read().ai.face_min_size_px,
            threshold: 0.8,
            policy: deps.face_policy.clone(),
        }));
    }
    plan
}

/// Elementi da analizzare: `ids` espliciti, una sorgente, o tutta la libreria.
fn target_ids(store: &Store, params: &serde_json::Value) -> AppResult<Vec<MediaId>> {
    if let Some(a) = params["ids"].as_array() {
        return Ok(a.iter().filter_map(|v| v.as_i64()).collect());
    }
    let source = params["sourceId"].as_i64();
    Ok(store.read(|c| {
        let mut st = c.prepare(
            "SELECT id FROM media_item WHERE status = 'ok' AND thumb IS NOT NULL AND (?1 IS NULL OR source_id = ?1) ORDER BY id",
        )?;
        let v = st.query_map([source], |r| r.get(0))?.collect::<Result<_, _>>()?;
        Ok(v)
    })?)
}

/// Decodifica ridotta e orientata per l'analisi.
fn load_image(deps: &AiDeps, id: MediaId) -> AppResult<Decoded> {
    load_oriented(&deps.store, &deps.resources.governor, id, ANALYSIS_SIZE)
}

/// Immagine orientata come la vede l'analisi, con lato lungo `max_side` (i box dei volti sono
/// normalizzati su questa immagine).
pub fn load_oriented(
    store: &Store,
    governor: &pc_governor::MemoryGovernor,
    id: MediaId,
    max_side: u32,
) -> AppResult<Decoded> {
    let (item, tech) = store.read(|c| Ok((repo::media::get(c, id)?, repo::meta::tech(c, id)?)))?;
    if item.status != MediaStatus::Ok {
        return Err(AppError::new(
            ErrorCode::NotFound,
            format!("file mancante: {}", item.path),
        ));
    }
    let mut d = decode_reduced(
        std::path::Path::new(&item.path),
        &item.ext,
        max_side,
        governor,
    )?;
    let native = matches!(item.ext.to_ascii_lowercase().as_str(), "heic" | "heif");
    let oriented = apply_orientation(
        d.image,
        if native {
            None
        } else {
            tech.as_ref().and_then(|t| t.orientation)
        },
    );
    d.image = fit(&oriented, max_side);
    drop(oriented);
    d.source_size = tech
        .and_then(|t| Some((t.width?, t.height?)))
        .unwrap_or(d.source_size);
    d.permit
        .shrink_to((d.image.as_raw().len() as u64).div_ceil(1024 * 1024).max(1));
    Ok(d)
}

/// Job "analisi" (parametri `{ "ids": [..] }`, `{ "sourceId": n }` o nessuno = tutta la libreria):
/// salta gli elementi già analizzati con le versioni correnti di modello e tassonomia.
pub fn plan(deps: AiDeps, params: &serde_json::Value) -> AppResult<JobPlan> {
    let params = params.clone();
    Ok(JobPlan {
        phases: vec![
            PhaseSpec::new(PHASE_CLIP, 4.0),
            PhaseSpec::new(PHASE_PERSONS, 2.0),
            PhaseSpec::new(PHASE_FACES, 4.0),
            PhaseSpec::new(PHASE_CLUSTER, 1.0),
        ],
        min_memory_mb: 256,
        run: Box::new(move |ctx| {
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
            let ids = target_ids(&deps.store, &params)?;
            let runnable = deps.runnable_models();
            if runnable.is_empty() && runnable.over_budget {
                // Requisiti minimi: nessuna fase AI entra nel budget, il job termina senza analisi.
                for p in [PHASE_CLIP, PHASE_PERSONS, PHASE_FACES, PHASE_CLUSTER] {
                    ctx.progress.phase(p).set_total(0);
                    ctx.progress.phase(p).finish();
                }
                return Ok(RunSummary {
                    cancelled: false,
                    failed: 0,
                });
            }
            let RunnableModels {
                clip,
                persons,
                faces,
                ..
            } = runnable;
            if clip.is_none() && persons.is_none() && faces.is_none() {
                return Err(AppError::new(
                    ErrorCode::ModelUnavailable,
                    "nessun modello AI verificato",
                ));
            }
            let mut passes = Vec::new();
            let mut clip_ctx = None;
            if let Some(id) = &clip {
                let (tax, concepts) = deps.concepts()?;
                passes.push(ModelPass {
                    phase: PHASE_CLIP.into(),
                    ids: vec![id.clone()],
                });
                clip_ctx = Some((tax, concepts));
            } else {
                ctx.progress.phase(PHASE_CLIP).finish();
            }
            if let Some(id) = &persons {
                passes.push(ModelPass {
                    phase: PHASE_PERSONS.into(),
                    ids: vec![id.clone()],
                });
            } else {
                ctx.progress.phase(PHASE_PERSONS).finish();
            }
            if let Some(ids) = faces {
                passes.push(ModelPass {
                    phase: PHASE_FACES.into(),
                    ids,
                });
            } else {
                ctx.progress.phase(PHASE_FACES).set_total(0);
                ctx.progress.phase(PHASE_FACES).finish();
            }
            let failed = std::cell::Cell::new(0i64);
            let res = run_model_major(
                &deps.resources.pool,
                &deps.resources.governor,
                &passes,
                &ctx.control,
                |pass, loaded| {
                    let phase = ctx.progress.phase(&pass.phase);
                    let key = if pass.phase == PHASE_FACES {
                        crate::face_analysis::versions(&deps.resources)?.1
                    } else {
                        model_key(deps.resources.entry(if pass.phase == PHASE_CLIP {
                            Role::ClassificationImage
                        } else {
                            Role::PersonDetection
                        })?)
                    };
                    let taxonomy_ver = clip_ctx.as_ref().map(|(t, _)| t.version);
                    let todo = deps.store.read(|c| {
                        analysis::pending(
                            c,
                            &ids,
                            &pass.phase,
                            &key,
                            if pass.phase == PHASE_CLIP {
                                taxonomy_ver
                            } else {
                                None
                            },
                        )
                    })?;
                    phase.set_total(todo.len() as i64);
                    phase.start();
                    for id in todo {
                        if !ctx.control.checkpoint() {
                            return Err(AppError::cancelled());
                        }
                        let outcome = (|| -> AppResult<()> {
                            let decoded = load_image(&deps, id)?;
                            let img = &decoded.image;
                            if pass.phase == PHASE_CLIP {
                                let (tax, concepts) = clip_ctx.as_ref().expect("contesto CLIP");
                                let mut r = classify::images(
                                    &[img],
                                    tax,
                                    concepts,
                                    &deps.resources.models,
                                    &deps.resources.pool,
                                    &deps.resources.governor,
                                )?;
                                let r = r.remove(0);
                                let groups: Vec<String> =
                                    tax.groups.iter().map(|g| g.id.clone()).collect();
                                let (k, v) = (key.clone(), tax.version);
                                deps.store.submit(move |c| {
                                    analysis::classification(
                                        c,
                                        id,
                                        &groups,
                                        &r.labels,
                                        &r.embedding,
                                        &k,
                                        v,
                                    )
                                })?;
                            } else if pass.phase == PHASE_FACES {
                                if let Some(generation) = deps.face_policy.token() {
                                    let found = crate::face_analysis::analyze(
                                        &deps.resources,
                                        img,
                                        decoded.source_size,
                                        deps.config.read().ai.face_min_size_px,
                                        0.8,
                                    )?;
                                    let (emb, analysis) =
                                        crate::face_analysis::versions(&deps.resources)?;
                                    let policy = deps.face_policy.clone();
                                    deps.store.submit(move |c| {
                                        if policy.is_current(generation) {
                                            pc_store::people::save_faces(
                                                c,
                                                id,
                                                &found,
                                                &emb,
                                                &analysis,
                                                crate::faces::SAME_PERSON,
                                            )?;
                                        }
                                        Ok(())
                                    })?;
                                }
                            } else {
                                let _permit =
                                    deps.resources.governor.try_acquire(32).ok_or_else(|| {
                                        AppError::new(
                                            ErrorCode::InsufficientMemory,
                                            "memoria per rilevamento persone",
                                        )
                                    })?;
                                let found = persons::detect(
                                    &mut loaded[0].session.lock(),
                                    img,
                                    PERSON_THRESHOLD,
                                )?;
                                let boxes: Vec<[f32; 5]> = found
                                    .iter()
                                    .filter_map(|p| {
                                        normalized_box(&p.bbox, p.score, img.width(), img.height())
                                    })
                                    .collect();
                                let k = key.clone();
                                deps.store
                                    .submit(move |c| analysis::people(c, id, &boxes, &k))?;
                            }
                            Ok(())
                        })();
                        match outcome {
                            Ok(()) => phase.inc_done(1),
                            Err(e) => {
                                phase.inc_failed(1);
                                failed.set(failed.get() + 1);
                                let path = deps
                                    .store
                                    .read(|c| repo::media::get(c, id))
                                    .map(|m| m.path)
                                    .unwrap_or_default();
                                tracing::error!(
                                    code = e.code.as_str(),
                                    job_id = ctx.job_id,
                                    phase = pass.phase.as_str(),
                                    path = path.as_str(),
                                    detail = e.detail.as_str(),
                                    "{}",
                                    e.message_key
                                );
                            }
                        }
                    }
                    deps.store.flush()?;
                    phase.finish();
                    Ok(())
                },
                |ev| tracing::debug!(?ev, "passata modello"),
            );
            if res.is_ok() {
                let p = ctx.progress.phase(PHASE_CLUSTER);
                p.set_total(1);
                p.start();
                crate::cluster::run(
                    &deps.store,
                    &deps.resources.governor,
                    &ctx.control,
                    deps.face_policy.clone(),
                )?;
                p.inc_done(1);
                p.finish();
            }
            match res {
                Ok(()) => Ok(RunSummary {
                    cancelled: false,
                    failed: failed.get(),
                }),
                Err(e) if e.code == ErrorCode::Cancelled => Ok(RunSummary {
                    cancelled: true,
                    failed: failed.get(),
                }),
                Err(e) => Err(e),
            }
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reanalysis_of_a_selection_targets_only_the_selected_ids() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
        store
            .write(|c| {
                for i in 1..=5 {
                    repo::media::upsert(
                        c,
                        &pc_store::model::NewMedia {
                            path: format!("IMG_{i}.jpg"),
                            name: format!("IMG_{i}.jpg"),
                            ext: "jpg".into(),
                            ..Default::default()
                        },
                    )?;
                }
                c.execute("UPDATE media_item SET thumb = 'miniatura'", [])?;
                Ok(())
            })
            .unwrap();
        // Parametri del comando "Rianalizza" su una selezione.
        let ids = target_ids(&store, &serde_json::json!({ "ids": [2, 4] })).unwrap();
        assert_eq!(ids, vec![2, 4]);
        // Senza selezione: tutta la libreria.
        let all = target_ids(&store, &serde_json::json!({})).unwrap();
        assert_eq!(all, vec![1, 2, 3, 4, 5]);
    }
}
