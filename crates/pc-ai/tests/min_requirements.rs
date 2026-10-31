//! Requisiti minimi: con un budget inferiore al modello più piccolo le fasi AI vengono
//! disattivate con un avviso `INSUFFICIENT_MEMORY` nel registro, mentre l'import termina
//! con metadati, miniature e libreria popolati.
//! Binario separato: installa un subscriber globale (i job girano su altri thread).

use parking_lot::{Mutex, RwLock};
use pc_ai::analyze::{self, AiDeps};
use pc_ai::stages::AiResources;
use pc_governor::{FakeProbe, MemoryGovernor};
use pc_ingest::import::{self, ImportDeps};
use pc_ingest::thumb::ThumbCache;
use pc_jobs::manager::{JobManager, JobSnapshot, ProgressSink};
use pc_jobs::JobState;
use pc_models::{pool::ModelPool, store::ModelStore, Manifest};
use pc_store::errorlog::{self, DbErrorSink, ErrorQuery};
use pc_store::model::NewSource;
use pc_store::{repo, Store, StoreOptions};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

#[derive(Default)]
struct Last(Mutex<BTreeMap<i64, JobSnapshot>>);
impl ProgressSink for Last {
    fn on_progress(&self, s: &JobSnapshot) {
        self.0.lock().insert(s.id, s.clone());
    }
}

fn asset(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets")
        .join(rel)
}

fn jpeg(seed: u8) -> Vec<u8> {
    let img = image::RgbImage::from_fn(320, 240, |x, y| {
        image::Rgb([seed, (x % 200) as u8, (y % 200) as u8])
    });
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 85)
        .encode_image(&img)
        .unwrap();
    out
}

#[test]
fn budget_below_smallest_model_disables_ai_and_import_completes() {
    let tmp = tempfile::tempdir().unwrap();
    let (sub, diag) = pc_diag::build(
        &tmp.path().join("logs"),
        &pc_core::config::LoggingConfig::default(),
    );
    tracing::subscriber::set_global_default(sub).unwrap();

    let root = tmp.path().join("Foto");
    std::fs::create_dir_all(&root).unwrap();
    for i in 0..3u8 {
        std::fs::write(root.join(format!("IMG_{i}.jpg")), jpeg(i * 40)).unwrap();
    }
    let store = Store::open(&tmp.path().join("catalog.db"), StoreOptions::default()).unwrap();
    diag.hub()
        .add_sink(Arc::new(DbErrorSink::new(store.clone())));
    let source_id = store
        .write({
            let path = root.display().to_string();
            move |c| {
                repo::source::insert(
                    c,
                    &NewSource {
                        path,
                        recursive: true,
                        ..Default::default()
                    },
                )
            }
        })
        .unwrap();

    // Modelli veri e verificati (persone e volti), ma il budget non basta per nessuno.
    let mut manifest = Manifest::builtin();
    manifest
        .entries
        .retain(|e| ["yolox-tiny", "yunet", "sface"].contains(&e.id.as_str()));
    let smallest = manifest.entries.iter().map(|e| e.memory_mb).min().unwrap();
    let budget = smallest - 1;
    let models = Arc::new(ModelStore::new(
        manifest,
        tmp.path().join("models"),
        Some(asset("models")),
    ));
    models.restore(
        &models.check_all(),
        None,
        &std::sync::atomic::AtomicBool::new(false),
    );
    assert!(models.verified_path("yolox-tiny").is_some());
    let gov = MemoryGovernor::new(budget);
    let pool = Arc::new(ModelPool::new(
        models.clone(),
        gov.clone(),
        Arc::new(FakeProbe::new(2048, 300, 4)),
        Default::default(),
    ));
    let deps = AiDeps {
        resources: AiResources {
            models,
            pool: pool.clone(),
            governor: gov.clone(),
        },
        store: store.clone(),
        gazetteer: Arc::default(),
        config: Arc::new(RwLock::new(pc_core::Config::default())),
        face_policy: Arc::new(pc_ai::face_policy::FacePolicy::new(true)),
        analysis_lock: Arc::default(),
    };
    let r = deps.runnable_models();
    assert!(r.is_empty() && r.over_budget, "{r:?}");

    let sink = Arc::new(Last::default());
    // Il manager ha un budget proprio per l'ammissione dei job; quello AI è nel governor dei modelli.
    let mgr = JobManager::new(store.clone(), MemoryGovernor::new(1024), 1, sink.clone());
    let thumbs = ThumbCache::new(tmp.path().join("thumbs"));
    let import_deps = ImportDeps {
        store: store.clone(),
        thumbs: thumbs.clone(),
        governor: MemoryGovernor::new(1024),
        extra_stages: vec![],
        stage_factory: Some(analyze::import_factory(deps.clone())),
        io_workers: 2,
        decode_workers: 1,
    };
    mgr.register(
        import::JOB_KIND,
        Arc::new(move |p| import::plan(import_deps.clone(), p)),
    );
    let ai = deps.clone();
    mgr.register(
        analyze::JOB_KIND,
        Arc::new(move |p| analyze::plan(ai.clone(), p)),
    );

    let id = mgr
        .submit(
            import::JOB_KIND,
            serde_json::json!({ "sourceId": source_id }),
        )
        .unwrap();
    assert!(mgr.wait_idle(Duration::from_secs(60)));
    let s = sink.0.lock()[&id].clone();
    assert_eq!(s.state, JobState::Completed, "{s:?}");
    for p in [
        analyze::PHASE_CLIP,
        analyze::PHASE_PERSONS,
        analyze::PHASE_FACES,
    ] {
        let ph = s.phases.iter().find(|x| x.name == p).unwrap();
        assert_eq!(
            (ph.total, ph.done),
            (0, 0),
            "fase {p} non disattivata: {ph:?}"
        );
    }
    // Nessun job di analisi accodato dopo l'import.
    assert!(
        sink.0.lock().values().all(|j| j.kind != analyze::JOB_KIND),
        "analisi accodata nonostante la memoria insufficiente"
    );
    // Libreria popolata con metadati e miniature.
    assert_eq!(store.read(repo::media::count).unwrap(), 3);
    for i in 0..3u8 {
        let p = root.join(format!("IMG_{i}.jpg")).display().to_string();
        let item = store
            .read(move |c| repo::media::get_by_path(c, &p))
            .unwrap()
            .unwrap();
        assert!(thumbs.contains(item.thumb.as_deref().unwrap()));
        let tech = store
            .read(move |c| repo::meta::tech(c, item.id))
            .unwrap()
            .unwrap();
        assert_eq!((tech.width, tech.height), (Some(320), Some(240)));
    }
    // Nessun modello caricato né memoria trattenuta.
    assert!(pool.loaded_ids().is_empty());

    // Una rianalisi esplicita termina senza errori e senza caricare modelli.
    let job = mgr
        .submit(
            analyze::JOB_KIND,
            serde_json::json!({ "sourceId": source_id }),
        )
        .unwrap();
    assert!(mgr.wait_idle(Duration::from_secs(60)));
    assert_eq!(sink.0.lock()[&job].state, JobState::Completed);
    assert!(pool.loaded_ids().is_empty());

    // L'utente vede il motivo: avviso INSUFFICIENT_MEMORY nel registro, per fase.
    store.flush().unwrap();
    let warns = store
        .read(|c| {
            errorlog::query(
                c,
                &ErrorQuery {
                    code: Some("INSUFFICIENT_MEMORY".into()),
                    ..Default::default()
                },
            )
        })
        .unwrap();
    assert!(warns.iter().all(|w| w.level == "WARN"), "{warns:?}");
    for p in [analyze::PHASE_PERSONS, analyze::PHASE_FACES] {
        assert!(
            warns.iter().any(|w| w.phase.as_deref() == Some(p)),
            "avviso mancante per {p}: {warns:?}"
        );
    }
}
