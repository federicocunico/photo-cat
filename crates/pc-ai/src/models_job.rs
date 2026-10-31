//! Job "modelli": verifica all'avvio, ripristino dagli asset o download, self-test.
//!
//! Parametri: `{ "restore": false }` salta verifica e download; `{ "ids": [..] }` limita i modelli da ripristinare (vuoto = tutti i non
//! verificati); `{ "selfTest": "all" | ["id", ..] }` forza il self-test (altrimenti
//! solo i modelli senza un risultato per il file attuale).

use crate::selftest::{self_test, SelfTestEnv};
use parking_lot::Mutex;
use pc_core::config::ExecutionProvider;
use pc_core::{AppError, AppResult, ErrorCode};
use pc_jobs::{JobPlan, PhaseSpec, RunSummary};
use pc_models::runtime::{self, SessionOptions};
use pc_models::store::{self, ModelStore};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub const PHASE_SELFTEST: &str = "self_test";
pub use pc_models::store::JOB_KIND;

#[derive(Clone)]
pub struct ModelsDeps {
    pub store: Arc<ModelStore>,
    /// Immagini di riferimento per i self-test.
    pub images: PathBuf,
    pub opts: Arc<Mutex<SessionOptions>>,
}

fn ids(v: &serde_json::Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

pub fn plan(deps: ModelsDeps, params: &serde_json::Value) -> AppResult<JobPlan> {
    let restore_ids = ids(&params["ids"]);
    let skip_restore = params["restore"].as_bool() == Some(false);
    let forced = match &params["selfTest"] {
        serde_json::Value::String(s) if s == "all" => Some(Vec::new()),
        v @ serde_json::Value::Array(_) => Some(ids(v)),
        _ => None,
    };
    for id in restore_ids.iter().chain(forced.iter().flatten()) {
        if deps.store.manifest().get(id).is_none() {
            return Err(AppError::new(ErrorCode::NotFound, format!("modello {id}")));
        }
    }
    Ok(JobPlan {
        phases: vec![
            PhaseSpec::new(store::PHASE_VERIFY, 1.0),
            PhaseSpec::new(store::PHASE_DOWNLOAD, 8.0),
            PhaseSpec::new(PHASE_SELFTEST, 1.0),
        ],
        min_memory_mb: 512,
        run: Box::new(move |ctx| {
            let st = &deps.store;
            let cancel = AtomicBool::new(false);
            let mut summary = if skip_restore {
                ctx.progress.phase(store::PHASE_VERIFY).finish();
                ctx.progress.phase(store::PHASE_DOWNLOAD).finish();
                RunSummary {
                    cancelled: false,
                    failed: 0,
                }
            } else {
                let todo = if restore_ids.is_empty() {
                    st.check_all()
                } else {
                    restore_ids
                };
                st.restore(&todo, Some(&ctx), &cancel)
            };
            if summary.cancelled {
                return Ok(summary);
            }
            let to_test: Vec<String> = match forced {
                Some(v) if v.is_empty() => {
                    st.manifest().entries.iter().map(|e| e.id.clone()).collect()
                }
                Some(v) => v,
                None => st.needing_self_test(),
            };
            let phase = ctx.progress.phase(PHASE_SELFTEST);
            phase.set_total(to_test.len() as i64);
            let base_opts = deps.opts.lock().clone();
            let resolve = |r: pc_models::Role| {
                st.manifest()
                    .by_role(r)
                    .and_then(|e| st.verified_path(&e.id))
            };
            for id in to_test {
                if !ctx.control.checkpoint() {
                    return Ok(RunSummary {
                        cancelled: true,
                        failed: summary.failed,
                    });
                }
                let (Some(entry), Some(path)) =
                    (st.manifest().get(&id).cloned(), st.installed_path(&id))
                else {
                    phase.inc_failed(1);
                    summary.failed += 1;
                    tracing::warn!(
                        code = "MODEL_UNAVAILABLE",
                        job_id = ctx.job_id,
                        phase = PHASE_SELFTEST,
                        model = id.as_str(),
                        "modello non installato per il self-test"
                    );
                    continue;
                };
                // Senza runtime l'esito direbbe solo "runtime assente": non si registra, così il
                // modello resta da testare e non appare "Test fallito" quando il runtime torna.
                if entry.role.uses_onnx() && !runtime::is_initialized() {
                    phase.inc_failed(1);
                    summary.failed += 1;
                    tracing::warn!(
                        code = "MODEL_UNAVAILABLE",
                        job_id = ctx.job_id,
                        phase = PHASE_SELFTEST,
                        model = id.as_str(),
                        "ONNX Runtime non disponibile: self-test rimandato"
                    );
                    continue;
                }
                // Il self-test carica sessioni temporanee: anche queste consumano il budget.
                let cost = entry.memory_mb.saturating_add(64);
                let Some(_resident) = ctx
                    .governor
                    .try_reserve_resident(&format!("selftest-{}-{id}", ctx.job_id), cost)
                else {
                    phase.inc_failed(1);
                    summary.failed += 1;
                    tracing::warn!(
                        code = "INSUFFICIENT_MEMORY",
                        job_id = ctx.job_id,
                        phase = PHASE_SELFTEST,
                        model = id.as_str(),
                        cost_mb = cost,
                        "memoria insufficiente per il self-test: riprovare quando c'è spazio"
                    );
                    continue;
                };
                // Primo uso su GPU protetto da marker: un arresto disattiva la GPU per questo modello.
                let gpu = st.allow_gpu(&id) && base_opts.preferred != ExecutionProvider::Cpu;
                let mut opts = base_opts.clone();
                if !gpu {
                    opts.preferred = ExecutionProvider::Cpu;
                }
                let env = SelfTestEnv {
                    images: deps.images.clone(),
                    opts,
                    resolve: &resolve,
                };
                if gpu {
                    st.begin_gpu_trial(&id);
                }
                let r = self_test(&entry, &path, &env);
                if gpu {
                    st.end_gpu_trial(&id);
                }
                if r.passed {
                    phase.inc_done(1);
                } else {
                    phase.inc_failed(1);
                    summary.failed += 1;
                }
                st.set_self_test(&id, r);
            }
            phase.finish();
            Ok(summary)
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pc_governor::MemoryGovernor;
    use pc_jobs::{JobManager, JobSnapshot, JobState, ProgressSink};
    use pc_store::Store;

    struct Sink;
    impl ProgressSink for Sink {
        fn on_progress(&self, _: &JobSnapshot) {}
    }

    #[test]
    fn unknown_model_is_rejected_and_missing_self_test_is_a_job_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let deps = ModelsDeps {
            store: Arc::new(ModelStore::new(
                pc_models::Manifest::builtin(),
                tmp.path().join("models"),
                None,
            )),
            images: tmp.path().join("images"),
            opts: Arc::new(Mutex::new(SessionOptions::default())),
        };
        assert!(plan(deps.clone(), &serde_json::json!({"ids": ["unknown"]})).is_err());
        assert!(plan(deps.clone(), &serde_json::json!({"selfTest": ["unknown"]})).is_err());
        let db = Store::open(
            &tmp.path().join("catalog.db"),
            pc_store::StoreOptions::default(),
        )
        .unwrap();
        let jobs = JobManager::new(db, MemoryGovernor::new(1024), 1, Arc::new(Sink));
        jobs.register(JOB_KIND, Arc::new(move |params| plan(deps.clone(), params)));
        let id = jobs
            .submit(
                JOB_KIND,
                serde_json::json!({"restore": false, "selfTest": ["yunet"]}),
            )
            .unwrap();
        assert!(jobs.wait_idle(std::time::Duration::from_secs(5)));
        let snapshot = jobs.snapshot(id).unwrap();
        assert_eq!(snapshot.state, JobState::CompletedWithErrors);
        assert_eq!(snapshot.failed, 1);
    }

    #[test]
    fn without_runtime_onnx_self_test_is_postponed_not_recorded() {
        // In questo binario di test il runtime ONNX non viene mai inizializzato.
        assert!(!runtime::is_initialized());
        let tmp = tempfile::tempdir().unwrap();
        let bundle = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/models");
        let model_store = Arc::new(ModelStore::new(
            pc_models::Manifest::builtin(),
            tmp.path().join("models"),
            Some(bundle),
        ));
        let deps = ModelsDeps {
            store: model_store.clone(),
            images: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/selftest"),
            opts: Arc::new(Mutex::new(SessionOptions::default())),
        };
        let db = Store::open(
            &tmp.path().join("catalog.db"),
            pc_store::StoreOptions::default(),
        )
        .unwrap();
        let jobs = JobManager::new(db, MemoryGovernor::new(1024), 1, Arc::new(Sink));
        jobs.register(JOB_KIND, Arc::new(move |params| plan(deps.clone(), params)));
        let id = jobs
            .submit(
                JOB_KIND,
                serde_json::json!({"ids": ["yunet"], "selfTest": ["yunet"]}),
            )
            .unwrap();
        assert!(jobs.wait_idle(std::time::Duration::from_secs(30)));
        assert_eq!(jobs.snapshot(id).unwrap().failed, 1);
        assert!(model_store.installed_path("yunet").is_some());
        assert!(model_store
            .needing_self_test()
            .contains(&"yunet".to_string()));
    }
}
