//! Test semantici sui modelli reali e sulle immagini di `assets/selftest`.
//! `cargo test -p pc-ai --features model-tests`
#![cfg(feature = "model-tests")]

use pc_ai::{clip, faces, persons};
use pc_core::config::ExecutionProvider;
use pc_models::runtime::{self, SessionOptions};
use std::path::PathBuf;

fn asset(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets")
        .join(rel)
}

fn session(model: &str) -> ort::session::Session {
    runtime::init(&runtime::dev_runtime_path()).unwrap();
    let opts = SessionOptions {
        preferred: ExecutionProvider::Cpu,
        intra_threads: 4,
        ..Default::default()
    };
    runtime::load(&asset(&format!("models/{model}")), &opts)
        .unwrap()
        .0
}

fn img(name: &str) -> image::RgbImage {
    image::open(asset(&format!("selftest/{name}")))
        .unwrap()
        .to_rgb8()
}

#[test]
fn persons_counted_in_group_and_absent_in_landscape() {
    let mut s = session("yolox-tiny/yolox_tiny.onnx");
    let group = persons::detect(&mut s, &img("group_4_people.jpg"), 0.4).unwrap();
    let land = persons::detect(&mut s, &img("landscape_mountain.jpg"), 0.4).unwrap();
    eprintln!(
        "persone: gruppo {} {:?}",
        group.len(),
        group.iter().map(|p| p.score).collect::<Vec<_>>()
    );
    assert_eq!(group.len(), 4);
    assert_eq!(land.len(), 0);
}

#[test]
fn faces_detected_and_recognized() {
    let mut det = session("yunet/face_detection_yunet_2023mar.onnx");
    let mut rec = session("sface/face_recognition_sface_2021dec.onnx");
    let g4 = img("group_4_people.jpg");
    let found4 = faces::detect(&mut det, &g4, 0.7).unwrap();
    let found3 = faces::detect(&mut det, &img("group_3_faces.jpg"), 0.7).unwrap();
    let none = faces::detect(&mut det, &img("landscape_mountain.jpg"), 0.7).unwrap();
    eprintln!("volti: {} {} {}", found4.len(), found3.len(), none.len());
    assert_eq!(found4.len(), 4);
    assert_eq!(found3.len(), 3);
    assert!(none.is_empty());

    // Stesso volto in due scale diverse ≈ stessa persona; volti diversi no.
    let emb: Vec<Vec<f32>> = found4
        .iter()
        .map(|f| faces::embed(&mut rec, &faces::align(&g4, &f.landmarks)).unwrap())
        .collect();
    let small = image::imageops::resize(
        &g4,
        g4.width() / 2,
        g4.height() / 2,
        image::imageops::FilterType::Triangle,
    );
    let f_small = faces::detect(&mut det, &small, 0.7).unwrap();
    let first = &found4[0];
    let twin = f_small
        .iter()
        .min_by(|a, b| {
            let d = |f: &faces::Face| {
                (f.bbox.x * 2.0 - first.bbox.x).powi(2) + (f.bbox.y * 2.0 - first.bbox.y).powi(2)
            };
            d(a).total_cmp(&d(b))
        })
        .unwrap();
    let e_small = faces::embed(&mut rec, &faces::align(&small, &twin.landmarks)).unwrap();
    let same = pc_ai::tensor::cosine(&emb[0], &e_small);
    let others: Vec<f32> = emb[1..]
        .iter()
        .map(|e| pc_ai::tensor::cosine(&emb[0], e))
        .collect();
    eprintln!("stesso volto {same:.3}, altri {others:?}");
    assert!(same > faces::SAME_PERSON + 0.2);
    assert!(others.iter().all(|&c| c < faces::SAME_PERSON));
}

#[test]
fn clip_zero_shot_matches_scene() {
    let mut vis = session("clip-vit-b32/vision_model_quantized.onnx");
    let mut txt = session("clip-vit-b32/text_model_quantized.onnx");
    let tok = clip::ClipTokenizer::load(&asset("models/clip-vit-b32/tokenizer.json")).unwrap();
    let land = img("landscape_mountain.jpg");
    let group = img("group_4_people.jpg");
    let ie = clip::embed_images(&mut vis, &[&land, &group]).unwrap();
    let te = clip::embed_texts(
        &mut txt,
        &tok,
        &[
            "a photo of a snowy mountain landscape",
            "a portrait photo of a group of people",
        ],
    )
    .unwrap();
    let c = |i: usize, t: usize| pc_ai::tensor::cosine(&ie[i], &te[t]);
    eprintln!(
        "land: {:.3} {:.3} | group: {:.3} {:.3}",
        c(0, 0),
        c(0, 1),
        c(1, 0),
        c(1, 1)
    );
    assert!(c(0, 0) > c(0, 1));
    assert!(c(1, 1) > c(1, 0));
}

#[test]
fn default_taxonomy_classifies_mountain_and_persists_embedding() {
    use pc_ai::{classify, taxonomy::Taxonomy, text_cache};
    use pc_models::{pool::ModelPool, store::ModelStore, Manifest};
    use std::sync::Arc;
    runtime::init(&runtime::dev_runtime_path()).unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let db = pc_store::Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
    let store = Arc::new(ModelStore::new(
        Manifest::builtin(),
        tmp.path().join("models"),
        Some(asset("models")),
    ));
    let missing = store.check_all();
    store.restore(&missing, None, &std::sync::atomic::AtomicBool::new(false));
    let gov = pc_governor::MemoryGovernor::new(1024);
    let pool = ModelPool::new(
        store.clone(),
        gov.clone(),
        Arc::new(pc_governor::FakeProbe::new(8192, 6000, 8)),
        SessionOptions {
            preferred: ExecutionProvider::Cpu,
            intra_threads: 4,
            ..Default::default()
        },
    );
    let taxonomy = Taxonomy::default();
    let concepts = text_cache::from_models(&db, &taxonomy, &store, &pool).unwrap();
    assert!(!pool.is_loaded("clip-text"));
    let land = img("landscape_mountain.jpg");
    let result = classify::images(&[&land], &taxonomy, &concepts, &store, &pool, &gov)
        .unwrap()
        .remove(0);
    eprintln!("etichette: {:?}", result.labels);
    assert!(result
        .labels
        .iter()
        .any(|l| l.group == "location" && l.value == "esterno_natura"));
    assert!(result
        .labels
        .iter()
        .any(|l| l.group == "environment" && l.value == "montagna"));
    let groups: Vec<String> = taxonomy.groups.iter().map(|g| g.id.clone()).collect();
    db.write(move|c|{
        c.execute("INSERT INTO media_item(id,path,name,ext,size,mtime,added_at,updated_at) VALUES (1,'a.jpg','a.jpg','jpg',1,1,1,1)",[])?;
        pc_store::analysis::classification(c,1,&groups,&result.labels,&result.embedding,"clip-v1",taxonomy.version)?;
        Ok(())
    }).unwrap();
    let size: i64 = db
        .read(|c| {
            Ok(c.query_row(
                "SELECT length(vector) FROM embedding WHERE media_id=1",
                [],
                |r| r.get(0),
            )?)
        })
        .unwrap();
    assert_eq!(size, 512 * 4);
}

#[test]
fn model_major_releases_each_session_before_loading_next_pass() {
    use pc_ai::schedule::{self, ModelEvent, ModelPass};
    use pc_models::{pool::ModelPool, store::ModelStore, Manifest};
    use std::sync::Arc;
    runtime::init(&runtime::dev_runtime_path()).unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let mut manifest = Manifest::builtin();
    manifest
        .entries
        .retain(|e| e.id == "yunet" || e.id == "sface");
    let store = Arc::new(ModelStore::new(manifest, tmp.path(), Some(asset("models"))));
    store.restore(
        &store.check_all(),
        None,
        &std::sync::atomic::AtomicBool::new(false),
    );
    let gov = pc_governor::MemoryGovernor::new(160);
    let pool = ModelPool::new(
        store,
        gov.clone(),
        Arc::new(pc_governor::FakeProbe::new(8192, 6000, 8)),
        SessionOptions::default(),
    );
    let passes = vec![
        ModelPass {
            phase: "detection".into(),
            ids: vec!["yunet".into()],
        },
        ModelPass {
            phase: "embedding".into(),
            ids: vec!["sface".into()],
        },
    ];
    assert!(schedule::needs_model_major(&pool, &passes));
    let mut events = Vec::new();
    schedule::run_model_major(
        &pool,
        &gov,
        &passes,
        &pc_jobs::JobControl::new(),
        |pass, models| {
            assert_eq!(models.len(), 1);
            assert_eq!(pool.loaded_ids(), pass.ids);
            assert!(gov.snapshot().in_use_mb <= 160);
            Ok(())
        },
        |e| events.push(e),
    )
    .unwrap();
    assert_eq!(
        events,
        vec![
            ModelEvent::Loaded("yunet".into()),
            ModelEvent::Unloaded("yunet".into()),
            ModelEvent::Loaded("sface".into()),
            ModelEvent::Unloaded("sface".into())
        ]
    );
    assert_eq!(gov.snapshot().in_use_mb, 0);
}

#[test]
fn usable_faces_are_filtered_embedded_and_saved() {
    use pc_models::{pool::ModelPool, store::ModelStore, Manifest};
    use std::sync::Arc;
    runtime::init(&runtime::dev_runtime_path()).unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let mut manifest = Manifest::builtin();
    manifest
        .entries
        .retain(|e| e.id == "yunet" || e.id == "sface");
    let store = Arc::new(ModelStore::new(
        manifest,
        tmp.path().join("models"),
        Some(asset("models")),
    ));
    store.restore(
        &store.check_all(),
        None,
        &std::sync::atomic::AtomicBool::new(false),
    );
    let gov = pc_governor::MemoryGovernor::new(512);
    let pool = Arc::new(ModelPool::new(
        store.clone(),
        gov.clone(),
        Arc::new(pc_governor::FakeProbe::new(8192, 6000, 8)),
        SessionOptions {
            intra_threads: 4,
            ..Default::default()
        },
    ));
    let resources = pc_ai::stages::AiResources {
        models: store,
        pool,
        governor: gov,
    };
    let group = img("group_3_faces.jpg");
    let source = group.dimensions();
    let found = pc_ai::face_analysis::analyze(&resources, &group, source, 40, 0.7).unwrap();
    assert_eq!(found.len(), 3);
    assert!(
        pc_ai::face_analysis::analyze(&resources, &group, source, 10000, 0.7)
            .unwrap()
            .is_empty()
    );
    assert!(found.iter().all(|f| f.embedding.len() == 128));
    let db = pc_store::Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
    db.write(move|c|{c.execute("INSERT INTO media_item(id,path,name,ext,size,mtime,added_at,updated_at) VALUES (1,'group.jpg','group.jpg','jpg',1,1,1,1)",[])?;pc_store::people::save_faces(c,1,&found,"sface","yunet+sface",0.363)}).unwrap();
    assert_eq!(
        db.read(|c| pc_store::people::records(c, Some(1)))
            .unwrap()
            .len(),
        3
    );
}

#[test]
fn disabled_face_recognition_saves_no_faces_but_still_counts_people() {
    use parking_lot::RwLock;
    use pc_ai::analyze::{self, AiDeps};
    use pc_jobs::manager::{JobManager, JobSnapshot, ProgressSink};
    use pc_models::{pool::ModelPool, store::ModelStore, Manifest};
    use std::sync::Arc;
    struct Sink;
    impl ProgressSink for Sink {
        fn on_progress(&self, _: &JobSnapshot) {}
    }
    runtime::init(&runtime::dev_runtime_path()).unwrap();
    // Riconoscimento disattivato nella configurazione e nella policy (come all'avvio dell'app),
    // e solo nella policy (disattivazione durante l'esecuzione).
    for config_enabled in [false, true] {
        let tmp = tempfile::tempdir().unwrap();
        let photo = tmp.path().join("gruppo.jpg");
        std::fs::copy(asset("selftest/group_4_people.jpg"), &photo).unwrap();
        let db = pc_store::Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
        let path = photo.display().to_string();
        db.write(move |c| {
            let id = pc_store::repo::media::upsert(
                c,
                &pc_store::model::NewMedia {
                    path,
                    name: "gruppo.jpg".into(),
                    ext: "jpg".into(),
                    ..Default::default()
                },
            )?;
            pc_store::repo::media::set_thumb(c, id, "miniatura")
        })
        .unwrap();
        let mut manifest = Manifest::builtin();
        manifest
            .entries
            .retain(|e| ["yolox-tiny", "yunet", "sface"].contains(&e.id.as_str()));
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
        let gov = pc_governor::MemoryGovernor::new(1024);
        let pool = Arc::new(ModelPool::new(
            models.clone(),
            gov.clone(),
            Arc::new(pc_governor::FakeProbe::new(8192, 6000, 8)),
            SessionOptions {
                preferred: ExecutionProvider::Cpu,
                intra_threads: 4,
                ..Default::default()
            },
        ));
        let mut config = pc_core::Config::default();
        config.ai.faces_enabled = config_enabled;
        let deps = AiDeps {
            resources: pc_ai::stages::AiResources {
                models,
                pool,
                governor: gov.clone(),
            },
            store: db.clone(),
            gazetteer: Arc::default(),
            config: Arc::new(RwLock::new(config)),
            face_policy: Arc::new(pc_ai::face_policy::FacePolicy::new(false)),
            analysis_lock: Arc::default(),
        };
        let jobs = JobManager::new(db.clone(), gov, 1, Arc::new(Sink));
        jobs.register(
            analyze::JOB_KIND,
            Arc::new(move |p| analyze::plan(deps.clone(), p)),
        );
        let id = jobs
            .submit(analyze::JOB_KIND, serde_json::json!({ "ids": [1] }))
            .unwrap();
        assert!(jobs.wait_idle(std::time::Duration::from_secs(120)));
        let s = jobs.snapshot(id).unwrap();
        assert_eq!(s.state, pc_jobs::JobState::Completed, "{s:?}");
        let count = |sql: &str| -> i64 {
            let sql = sql.to_string();
            db.read(move |c| Ok(c.query_row(&sql, [], |r| r.get(0))?))
                .unwrap()
        };
        assert_eq!(count("SELECT count(*) FROM face"), 0, "{config_enabled}");
        assert_eq!(count("SELECT count(*) FROM person"), 0);
        assert_eq!(
            count("SELECT count(*) FROM analysis_version WHERE stage = 'faces'"),
            0
        );
        assert_eq!(
            count("SELECT count FROM media_people WHERE media_id = 1"),
            4,
            "le persone vengono contate anche senza riconoscimento volti"
        );
    }
}
