//! `cargo test -p pc-ai --features model-tests --test selftest`
#![cfg(all(feature = "model-tests", windows))]

use pc_ai::selftest::{self_test, SelfTestEnv};
use pc_core::config::ExecutionProvider;
use pc_models::runtime::{self, SessionOptions};
use pc_models::{Manifest, Role};
use std::path::PathBuf;

fn assets() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn env_for(manifest: &Manifest) -> (PathBuf, impl Fn(Role) -> Option<PathBuf> + '_) {
    runtime::init(&runtime::dev_dll_path()).unwrap();
    let models = assets().join("models");
    let m2 = models.clone();
    (models, move |r: Role| {
        manifest.by_role(r).map(|e| m2.join(&e.file))
    })
}

#[test]
fn every_bundled_model_passes_its_self_test() {
    let manifest = Manifest::builtin();
    let (models, resolve) = env_for(&manifest);
    let env = SelfTestEnv {
        images: assets().join("selftest"),
        opts: SessionOptions {
            preferred: ExecutionProvider::Cpu,
            intra_threads: 4,
            ..Default::default()
        },
        resolve: &resolve,
    };
    for e in &manifest.entries {
        let r = self_test(e, &models.join(&e.file), &env);
        eprintln!(
            "{:<15} {:>8.1} ms  {}  {}",
            e.id,
            r.avg_ms,
            if r.passed { "OK" } else { "FALLITO" },
            r.message
        );
        assert!(r.passed, "{}: {}", e.id, r.message);
    }
}

#[test]
fn altered_model_fails_self_test() {
    let manifest = Manifest::builtin();
    let (models, resolve) = env_for(&manifest);
    let env = SelfTestEnv {
        images: assets().join("selftest"),
        opts: SessionOptions {
            preferred: ExecutionProvider::Cpu,
            intra_threads: 4,
            ..Default::default()
        },
        resolve: &resolve,
    };
    let entry = manifest.get("yolox-tiny").unwrap();
    let mut bytes = std::fs::read(models.join(&entry.file)).unwrap();
    // Altera una parte consistente dei pesi lasciando intatta la struttura del file.
    let (start, end) = (bytes.len() / 4, bytes.len() / 4 * 3);
    for b in &mut bytes[start..end] {
        *b = b.wrapping_add(37);
    }
    let tmp = tempfile::tempdir().unwrap();
    let altered = tmp.path().join("yolox_tiny_altered.onnx");
    std::fs::write(&altered, bytes).unwrap();
    let r = self_test(entry, &altered, &env);
    eprintln!("modello alterato: {}", r.message);
    assert!(!r.passed);
}

#[test]
fn bundled_models_restore_and_self_test_as_a_background_job() {
    use pc_ai::models_job::{self, ModelsDeps};
    use pc_jobs::{JobManager, JobSnapshot, JobState, ProgressSink};
    use pc_models::store::{ModelState, ModelStore};
    use std::sync::Arc;

    struct Sink;
    impl ProgressSink for Sink {
        fn on_progress(&self, _: &JobSnapshot) {}
    }

    runtime::init(&runtime::dev_dll_path()).unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let store = Arc::new(ModelStore::new(
        Manifest::builtin(),
        tmp.path().join("models"),
        Some(assets().join("models")),
    ));
    let governor = pc_governor::MemoryGovernor::new(512);
    let deps = ModelsDeps {
        store: store.clone(),
        images: assets().join("selftest"),
        opts: Arc::new(parking_lot::Mutex::new(SessionOptions {
            preferred: ExecutionProvider::Cpu,
            intra_threads: 4,
            ..Default::default()
        })),
    };
    let db = pc_store::Store::open(
        &tmp.path().join("catalog.db"),
        pc_store::StoreOptions::default(),
    )
    .unwrap();
    let jobs = JobManager::new(db, governor.clone(), 1, Arc::new(Sink));
    jobs.register(
        models_job::JOB_KIND,
        Arc::new(move |params| models_job::plan(deps.clone(), params)),
    );
    let id = jobs
        .submit(models_job::JOB_KIND, serde_json::json!({}))
        .unwrap();
    assert!(jobs.wait_idle(std::time::Duration::from_secs(60)));
    let snapshot = jobs.snapshot(id).unwrap();
    assert_eq!(snapshot.state, JobState::Completed, "{snapshot:?}");
    assert_eq!(snapshot.overall_percent, 100.0);
    assert!(
        store
            .statuses()
            .iter()
            .all(|s| s.state == ModelState::Verified
                && s.self_test.as_ref().is_some_and(|r| r.passed))
    );
    assert_eq!(governor.snapshot().in_use_mb, 0);
}
