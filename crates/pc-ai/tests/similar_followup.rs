//! Senza alcun modello AI i gruppi partono da soli: dopo l'import e dopo il recupero delle
//! impronte, accodando `similar` una sola volta.

use image::{codecs::jpeg::JpegEncoder, imageops, ImageEncoder, Rgb, RgbImage};
use parking_lot::RwLock;
use pc_ai::analyze::{self, AiDeps};
use pc_ai::stages::AiResources;
use pc_governor::{FakeProbe, MemoryGovernor};
use pc_ingest::import::{self, ImportDeps};
use pc_ingest::phash_backfill;
use pc_ingest::thumb::ThumbCache;
use pc_jobs::manager::{JobManager, JobSnapshot, ProgressSink};
use pc_jobs::persist;
use pc_models::{pool::ModelPool, store::ModelStore, Manifest};
use pc_store::model::NewSource;
use pc_store::{repo, similar as db, Store, StoreOptions};
use std::sync::Arc;
use std::time::Duration;

struct Sink;
impl ProgressSink for Sink {
    fn on_progress(&self, _: &JobSnapshot) {}
}

/// Scena testurizzata deterministica (le immagini lisce hanno impronte instabili).
fn scene(seed: u32, w: u32, h: u32) -> RgbImage {
    RgbImage::from_fn(w, h, |x, y| {
        let (fx, fy) = (x as f32 / w as f32, y as f32 / h as f32);
        let s = seed as f32;
        let a = ((fx * (3.0 + s) * std::f32::consts::TAU).sin() * 0.5 + 0.5) * 255.0;
        let b = ((fy * (2.0 + s * 1.7) * std::f32::consts::TAU + fx * s).cos() * 0.5 + 0.5) * 255.0;
        let c =
            (((fx - 0.5).powi(2) + (fy - 0.5).powi(2)).sqrt() * (4.0 + s) * std::f32::consts::TAU)
                .sin()
                .abs()
                * 255.0;
        Rgb([a as u8, b as u8, c as u8])
    })
}

fn write_jpeg(path: &std::path::Path, img: &RgbImage, q: u8) {
    let mut buf = Vec::new();
    JpegEncoder::new_with_quality(&mut buf, q)
        .write_image(
            img.as_raw(),
            img.width(),
            img.height(),
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();
    std::fs::write(path, buf).unwrap();
}

struct Env {
    _tmp: tempfile::TempDir,
    store: Store,
    mgr: JobManager,
    source_id: i64,
}

/// Libreria con tre foto (due versioni della stessa scena e una diversa), nessun modello AI.
fn env() -> Env {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("Foto");
    std::fs::create_dir_all(&root).unwrap();
    let a = scene(1, 320, 240);
    write_jpeg(&root.join("a.jpg"), &a, 92);
    write_jpeg(
        &root.join("b.jpg"),
        &imageops::resize(&a, 240, 180, imageops::FilterType::Triangle),
        70,
    );
    write_jpeg(&root.join("c.jpg"), &scene(5, 320, 240), 92);
    let store = Store::open(&tmp.path().join("catalog.db"), StoreOptions::default()).unwrap();
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
    let mut cfg = pc_core::Config::default();
    cfg.ai.auto_download_models = false;
    let models = Arc::new(ModelStore::new(
        Manifest::builtin(),
        tmp.path().join("models"),
        None,
    ));
    let gov = MemoryGovernor::new(2048);
    let pool = Arc::new(ModelPool::new(
        models.clone(),
        gov.clone(),
        Arc::new(FakeProbe::new(2048, 300, 4)),
        Default::default(),
    ));
    let thumbs = ThumbCache::new(tmp.path().join("thumbs"));
    let deps = AiDeps {
        resources: AiResources {
            models,
            pool,
            governor: gov,
        },
        store: store.clone(),
        gazetteer: Arc::default(),
        config: Arc::new(RwLock::new(cfg)),
        face_policy: Arc::new(pc_ai::face_policy::FacePolicy::new(true)),
        analysis_lock: Arc::default(),
        app_store: None,
        thumbs: Some(thumbs.clone()),
    };
    assert!(pc_ai::similar::resolve_model(&deps.resources).is_none());
    let mgr = JobManager::new(store.clone(), MemoryGovernor::new(1024), 1, Arc::new(Sink));
    let import_deps = ImportDeps {
        store: store.clone(),
        thumbs: thumbs.clone(),
        governor: MemoryGovernor::new(1024),
        extra_stages: vec![],
        stage_factory: Some(analyze::import_factory(deps.clone())),
        io_workers: 2,
        decode_workers: 1,
    };
    let d = deps.clone();
    mgr.register(
        import::JOB_KIND,
        Arc::new(move |p| {
            let plan = import::plan(import_deps.clone(), p)?;
            Ok(pc_ai::similar::with_similar_followup(d.clone(), plan))
        }),
    );
    let (d, st, th) = (deps.clone(), store.clone(), thumbs);
    mgr.register(
        phash_backfill::JOB_KIND,
        Arc::new(move |p| {
            let plan = phash_backfill::plan(st.clone(), th.clone(), p)?;
            Ok(pc_ai::similar::with_similar_followup(d.clone(), plan))
        }),
    );
    let d = deps.clone();
    mgr.register(
        pc_ai::similar::JOB_KIND,
        Arc::new(move |p| pc_ai::similar::plan(d.clone(), p)),
    );
    let d = deps;
    mgr.register(
        analyze::JOB_KIND,
        Arc::new(move |p| analyze::plan(d.clone(), p)),
    );
    Env {
        _tmp: tmp,
        store,
        mgr,
        source_id,
    }
}

impl Env {
    fn jobs_of(&self, kind: &'static str) -> i64 {
        self.store
            .read(move |c| {
                Ok(
                    c.query_row("SELECT count(*) FROM job WHERE kind = ?1", [kind], |r| {
                        r.get(0)
                    })?,
                )
            })
            .unwrap()
    }

    fn import(&self) {
        self.mgr
            .submit(
                import::JOB_KIND,
                serde_json::json!({ "sourceId": self.source_id }),
            )
            .unwrap();
        assert!(self.mgr.wait_idle(Duration::from_secs(120)));
    }

    /// Azzera impronte, gruppi e marcatore: come una libreria precedente allo schema v11.
    fn reset_phashes(&self) {
        self.store
            .write(|c| {
                c.execute("UPDATE media_item SET phash = NULL", [])?;
                c.execute("DELETE FROM similar_done", [])?;
                // Senza archi né gruppi: li ricrea solo il `similar` accodato dal recupero.
                c.execute("DELETE FROM similar_edge", [])?;
                c.execute("DELETE FROM similar_group", [])?;
                c.execute(
                    "DELETE FROM settings_kv WHERE key = 'phash_backfill_done'",
                    [],
                )?;
                Ok(())
            })
            .unwrap();
    }

    /// Gruppi come liste ordinate di nomi file.
    fn groups(&self) -> Vec<Vec<String>> {
        self.store
            .read(|c| {
                let mut out = Vec::new();
                let ids: Vec<i64> = c
                    .prepare("SELECT id FROM media_item ORDER BY id")?
                    .query_map([], |r| r.get(0))?
                    .collect::<Result<_, _>>()?;
                let mut seen = std::collections::HashSet::new();
                for id in ids {
                    if let Some((_, members)) = db::members_with(c, id)? {
                        if seen.insert(members.clone()) {
                            let mut names: Vec<String> = members
                                .iter()
                                .map(|m| repo::media::get(c, *m).unwrap().name)
                                .collect();
                            names.sort();
                            out.push(names);
                        }
                    }
                }
                Ok(out)
            })
            .unwrap()
    }
}

fn ab() -> Vec<Vec<String>> {
    vec![vec!["a.jpg".to_string(), "b.jpg".to_string()]]
}

#[test]
fn import_without_models_groups_the_copies_with_one_similar_job() {
    let e = env();
    e.import();
    assert_eq!(e.jobs_of("similar"), 1);
    assert_eq!(e.jobs_of("analysis"), 0);
    assert_eq!(e.groups(), ab());
}

#[test]
fn an_active_similar_job_prevents_a_second_one_after_import() {
    let e = env();
    // Un `similar` già in coda (non avviato): l'import non ne accoda un altro.
    persist::create(&e.store, "similar", &serde_json::json!({}), &[]).unwrap();
    e.import();
    assert_eq!(e.jobs_of("similar"), 1);
}

#[test]
fn backfill_queues_similar_once_and_not_when_one_is_active() {
    let e = env();
    e.import();
    assert_eq!(e.jobs_of("similar"), 1);
    // Il recupero ricalcola le impronte e accoda `similar` (uno solo).
    e.reset_phashes();
    assert!(e.groups().is_empty());
    assert!(phash_backfill::queue_once(&e.store, &e.mgr)
        .unwrap()
        .is_some());
    assert!(e.mgr.wait_idle(Duration::from_secs(120)));
    assert_eq!(e.jobs_of("similar"), 2);
    assert_eq!(e.groups(), ab());

    // Con un `similar` già attivo il recupero non ne accoda altri.
    e.reset_phashes();
    persist::create(&e.store, "similar", &serde_json::json!({}), &[]).unwrap();
    assert!(phash_backfill::queue_once(&e.store, &e.mgr)
        .unwrap()
        .is_some());
    assert!(e.mgr.wait_idle(Duration::from_secs(120)));
    assert_eq!(e.jobs_of("similar"), 3, "solo quello creato a mano");
}

/// Avvio: il file del modello c'è ma `check_all` non è ancora girato (stato `Missing`):
/// il modello conta già come presente, così la chiave non passa da `none` e poi da CLIP.
#[test]
fn an_installed_but_unchecked_model_already_counts_as_present() {
    let tmp = tempfile::tempdir().unwrap();
    let manifest = Manifest::builtin();
    let file = manifest
        .by_role(pc_models::Role::ClassificationImage)
        .unwrap()
        .file
        .clone();
    let models = Arc::new(ModelStore::new(manifest, tmp.path().join("models"), None));
    let gov = MemoryGovernor::new(2048);
    let pool = Arc::new(ModelPool::new(
        models.clone(),
        gov.clone(),
        Arc::new(FakeProbe::new(2048, 300, 4)),
        Default::default(),
    ));
    let res = AiResources {
        models,
        pool,
        governor: gov,
    };
    assert!(pc_ai::similar::resolve_model(&res).is_none());
    let path = tmp.path().join("models").join(file);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, b"x").unwrap();
    assert!(pc_ai::similar::resolve_model(&res).is_some());
}
