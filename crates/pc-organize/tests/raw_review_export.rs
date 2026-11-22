//! Job `raw_review_export` (task 4.2): copia dei soli JPEG scelti, import della destinazione
//! e nuovo catalogo con valutazioni e tag, come un'unica attività.
use parking_lot::Mutex;
use pc_governor::MemoryGovernor;
use pc_ingest::import::{self, ImportDeps};
use pc_ingest::thumb::ThumbCache;
use pc_jobs::manager::{JobManager, JobSnapshot, ProgressSink};
use pc_organize::raw_export::{
    self, ExistingCatalog, ImportHook, ImportTarget, RawExportItem, RawExportOptions,
    RawExportReport, RawExportReports, JOB_KIND, PRESET_SET,
};
use pc_store::app_store::tags as sets;
use pc_store::model::NewSource;
use pc_store::{bulk, repo, AppStore, Store};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

#[derive(Default)]
struct Last(Mutex<BTreeMap<i64, JobSnapshot>>);
impl ProgressSink for Last {
    fn on_progress(&self, s: &JobSnapshot) {
        self.0.lock().insert(s.id, s.clone());
    }
}

/// JPEG con data di scatto EXIF.
fn jpeg(seed: u8, date: &str) -> Vec<u8> {
    let img = image::RgbImage::from_fn(64, 48, |x, y| {
        image::Rgb([seed, (x * 3 % 200) as u8, (y * 5 % 200) as u8])
    });
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 85)
        .encode_image(&img)
        .unwrap();
    let field = exif::Field {
        tag: exif::Tag::DateTimeOriginal,
        ifd_num: exif::In::PRIMARY,
        value: exif::Value::Ascii(vec![date.as_bytes().to_vec()]),
    };
    let mut w = exif::experimental::Writer::new();
    w.push_field(&field);
    let mut tiff = std::io::Cursor::new(Vec::new());
    w.write(&mut tiff, false).unwrap();
    let tiff = tiff.into_inner();
    let mut app1 = vec![0xFF, 0xE1];
    app1.extend_from_slice(&((tiff.len() + 8) as u16).to_be_bytes());
    app1.extend_from_slice(b"Exif\0\0");
    app1.extend_from_slice(&tiff);
    let mut res = out[..2].to_vec();
    res.extend(app1);
    res.extend_from_slice(&out[2..]);
    res
}

struct Env {
    tmp: tempfile::TempDir,
    store: Store,
    app: AppStore,
    mgr: JobManager,
    sink: Arc<Last>,
    reports: RawExportReports,
}

/// `cancel_in_import`: il job viene annullato quando comincia l'import.
fn env_with(cancel_in_import: bool) -> Env {
    let tmp = tempfile::tempdir().unwrap();
    let store = Store::open(&tmp.path().join("catalog.db"), Default::default()).unwrap();
    let app = AppStore::open_in_memory().unwrap();
    let sink = Arc::new(Last::default());
    let gov = MemoryGovernor::new(1024);
    let mgr = JobManager::new(store.clone(), gov.clone(), 1, sink.clone());
    let deps = ImportDeps {
        store: store.clone(),
        thumbs: ThumbCache::new(tmp.path().join("thumbs")),
        governor: gov,
        extra_stages: vec![],
        stage_factory: None,
        io_workers: 2,
        decode_workers: 2,
    };
    let phases = import::phases(&deps);
    let factory: pc_jobs::JobFactory = Arc::new(move |p| {
        let mut plan = import::plan(deps.clone(), p)?;
        if cancel_in_import {
            let run = plan.run;
            plan.run = Box::new(move |ctx| {
                ctx.control.cancel();
                run(ctx)
            });
        }
        Ok(plan)
    });
    let hook = ImportHook { phases, factory };
    let reports = RawExportReports::default();
    let (s, a, r) = (store.clone(), app.clone(), reports.clone());
    mgr.register(
        JOB_KIND,
        Arc::new(move |p| raw_export::plan(s.clone(), a.clone(), hook.clone(), r.clone(), p)),
    );
    Env {
        tmp,
        store,
        app,
        mgr,
        sink,
        reports,
    }
}

fn env() -> Env {
    env_with(false)
}

impl Env {
    fn path(&self, rel: &str) -> PathBuf {
        self.tmp.path().join(rel)
    }

    /// `n` JPEG in `revisione/C1`, con scatti a un minuto di distanza (ordine dei nomi = ordine
    /// di scatto), preset alternati e valutazione `i % 6`.
    fn shots(&self, n: usize) -> Vec<RawExportItem> {
        let dir = self.path("revisione/C1");
        std::fs::create_dir_all(&dir).unwrap();
        (0..n)
            .map(|i| {
                let name = format!("DSCF{:04}.JPG", i + 1);
                let file = dir.join(&name);
                std::fs::write(&file, jpeg(i as u8, &format!("2026:08:19 10:{i:02}:00"))).unwrap();
                RawExportItem {
                    source_path: file.display().to_string(),
                    orig_name: name,
                    preset: if i % 2 == 0 {
                        "C1 Kodachrome 64".into()
                    } else {
                        "C2 Velvia".into()
                    },
                    rating: (i % 6) as u8,
                }
            })
            .collect()
    }

    fn options(&self, dest: &str, catalog: &str) -> RawExportOptions {
        RawExportOptions {
            destination: self.path(dest).display().to_string(),
            template: None,
            catalog_name: catalog.into(),
            existing_catalog: None,
            overwrite: false,
            preset_as_tag: true,
        }
    }

    /// Avvia, attende e restituisce lo stato finale e il riepilogo.
    fn run(
        &self,
        items: &[RawExportItem],
        options: &RawExportOptions,
    ) -> (JobSnapshot, RawExportReport) {
        let id = self
            .mgr
            .submit(JOB_KIND, raw_export::job_params(items, options))
            .unwrap();
        assert!(self.mgr.wait_idle(Duration::from_secs(60)));
        (
            self.sink.0.lock()[&id].clone(),
            self.reports.get(id).expect("riepilogo"),
        )
    }

    fn catalog(&self, name: &str) -> Option<pc_store::model::Catalog> {
        let name = name.to_string();
        self.store
            .read(move |c| repo::catalog::by_name(c, &name))
            .unwrap()
    }

    fn catalog_paths(&self, id: i64) -> Vec<String> {
        let mut paths: Vec<String> = self
            .store
            .read(move |c| {
                let mut st = c.prepare(
                    "SELECT m.path FROM catalog_item ci JOIN media_item m ON m.id = ci.media_id
                     WHERE ci.catalog_id = ?1",
                )?;
                let rows = st
                    .query_map([id], |r| r.get(0))?
                    .collect::<Result<_, _>>()?;
                Ok(rows)
            })
            .unwrap();
        paths.sort();
        paths
    }

    fn sources(&self) -> Vec<pc_store::model::Source> {
        self.store.read(repo::source::list).unwrap()
    }

    fn rating_of(&self, path: &Path) -> u8 {
        let p = path.display().to_string();
        self.store
            .read(move |c| {
                Ok(
                    c.query_row("SELECT rating FROM media_item WHERE path = ?1", [p], |r| {
                        r.get::<_, u8>(0)
                    })?,
                )
            })
            .unwrap()
    }

    fn tags_of(&self, path: &Path) -> Vec<String> {
        let p = path.display().to_string();
        self.store
            .read(move |c| {
                let mut st = c.prepare(
                    "SELECT t.name FROM media_tag mt JOIN tag t ON t.id = mt.tag_id
                     JOIN media_item m ON m.id = mt.media_id WHERE m.path = ?1 ORDER BY t.name",
                )?;
                let rows = st.query_map([p], |r| r.get(0))?.collect::<Result<_, _>>()?;
                Ok(rows)
            })
            .unwrap()
    }

    fn count(&self, sql: &'static str) -> i64 {
        self.store
            .read(move |c| Ok(c.query_row(sql, [], |r| r.get(0))?))
            .unwrap()
    }
}

fn files_in(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(Result::ok)
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

fn hash(path: &Path) -> String {
    blake3::hash(&std::fs::read(path).unwrap())
        .to_hex()
        .to_string()
}

#[test]
fn copies_only_jpeg_bytes_and_leaves_sources_untouched() {
    let e = env();
    let mut items = e.shots(12);
    // Un RAF accanto ai JPEG e uno scatto la cui variante scelta è un TIFF: mai copiati.
    std::fs::write(e.path("revisione/C1/DSCF0001.RAF"), b"raf").unwrap();
    std::fs::write(e.path("revisione/C1/DSCF0099.tif"), b"tiff").unwrap();
    items.push(RawExportItem {
        source_path: e.path("revisione/C1/DSCF0099.tif").display().to_string(),
        orig_name: "DSCF0099.tif".into(),
        preset: "C1 Kodachrome 64".into(),
        rating: 3,
    });
    let before: Vec<(String, std::time::SystemTime)> = files_in(&e.path("revisione/C1"))
        .into_iter()
        .map(|n| {
            let p = e.path("revisione/C1").join(&n);
            (hash(&p), std::fs::metadata(&p).unwrap().modified().unwrap())
        })
        .collect();
    let (snap, report) = e.run(&items, &e.options("export", "Dolomiti"));
    assert_eq!(report.exported, 12);
    assert_eq!(report.failed, 1, "{:?}", report.errors);
    assert_eq!(report.errors[0].code, "UNSUPPORTED_FORMAT");
    assert!(!snap.phases.is_empty());
    let out = files_in(&e.path("export"));
    assert_eq!(out.len(), 12, "{out:?}");
    assert!(out.iter().all(|n| n.ends_with(".JPG")), "{out:?}");
    for item in items.iter().take(12) {
        assert_eq!(
            hash(Path::new(&item.source_path)),
            hash(&e.path("export").join(&item.orig_name)),
            "{}",
            item.orig_name
        );
    }
    let after: Vec<(String, std::time::SystemTime)> = files_in(&e.path("revisione/C1"))
        .into_iter()
        .map(|n| {
            let p = e.path("revisione/C1").join(&n);
            (hash(&p), std::fs::metadata(&p).unwrap().modified().unwrap())
        })
        .collect();
    assert_eq!(before, after, "i file sorgente non vanno toccati");
}

#[test]
fn numbers_files_by_capture_time_and_rejects_bad_templates() {
    let e = env();
    let dir = e.path("revisione/C1");
    std::fs::create_dir_all(&dir).unwrap();
    let mk = |name: &str, time: &str, seed: u8| {
        let f = dir.join(name);
        std::fs::write(&f, jpeg(seed, &format!("2026:08:19 {time}"))).unwrap();
        RawExportItem {
            source_path: f.display().to_string(),
            orig_name: name.into(),
            preset: "C1".into(),
            rating: 0,
        }
    };
    let items = vec![
        mk("DSCF0001.JPG", "10:02:00", 1),
        mk("DSCF0002.JPG", "09:15:00", 2),
        mk("DSCF0003.JPG", "11:40:00", 3),
    ];
    let mut options = e.options("export", "Dolomiti");
    options.template = Some("Dolomiti_{seq:04d}".into());
    let (_, report) = e.run(&items, &options);
    assert_eq!(report.exported, 3);
    assert_eq!(
        files_in(&e.path("export")),
        vec![
            "Dolomiti_0001.JPG",
            "Dolomiti_0002.JPG",
            "Dolomiti_0003.JPG"
        ]
    );
    // 09:15 è DSCF0002: il primo nome ha il suo contenuto.
    assert_eq!(
        hash(&e.path("export/Dolomiti_0001.JPG")),
        hash(&dir.join("DSCF0002.JPG"))
    );

    // Token sconosciuto, carattere non valido, `{tag}` fuori dalla revisione: rifiutati prima di copiare.
    for bad in ["{nope}_{seq}", "a/b_{seq}", "{tag}_{seq}", ""] {
        let mut o = e.options("export2", "Altro");
        o.template = Some(bad.into());
        let err = e
            .mgr
            .submit(JOB_KIND, raw_export::job_params(&items, &o))
            .unwrap_err();
        assert_eq!(err.code.as_str(), "INVALID_INPUT", "{bad}");
    }
    assert!(!e.path("export2").exists(), "nulla copiato");
    assert!(e.catalog("Altro").is_none());
}

#[test]
fn preset_token_names_files() {
    let e = env();
    let items = e.shots(2);
    let mut options = e.options("export", "Preset");
    options.template = Some("{preset}_{seq:02d}".into());
    let (_, report) = e.run(&items, &options);
    assert_eq!(report.exported, 2);
    assert_eq!(
        files_in(&e.path("export")),
        vec!["C1 Kodachrome 64_01.JPG", "C2 Velvia_02.JPG"]
    );
}

#[test]
fn creates_catalog_with_new_source_for_destination() {
    let e = env();
    let items = e.shots(12);
    let (snap, report) = e.run(&items, &e.options("Foto/Dolomiti", "Dolomiti 2026"));
    assert_eq!(report.exported, 12);
    assert!(!report.cancelled);
    let catalog = report.catalog.clone().expect("catalogo");
    assert!(catalog.created);
    assert_eq!(
        (catalog.name.as_str(), catalog.photos),
        ("Dolomiti 2026", 12)
    );
    assert!(matches!(
        report.import,
        Some(ImportTarget::NewSource { .. })
    ));
    // Nuova sorgente: la sola cartella di destinazione, senza sottocartelle né monitoraggio.
    let sources = e.sources();
    assert_eq!(sources.len(), 1);
    assert!(!sources[0].recursive && !sources[0].watch);
    assert!(sources[0].path.ends_with("Dolomiti"));
    // Catalogo manuale con esattamente quei 12 file.
    let found = e.catalog("Dolomiti 2026").unwrap();
    assert_eq!(found.kind, pc_store::model::CatalogKind::Manual);
    let paths = e.catalog_paths(found.id);
    assert_eq!(paths.len(), 12);
    assert!(paths.iter().all(|p| p.contains("Dolomiti")));
    // Un'unica attività: copia, fasi dell'import e catalogo.
    let names: Vec<&str> = snap.phases.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names.first(), Some(&"copia"));
    assert_eq!(names.last(), Some(&"catalogo"));
    assert!(names.contains(&"scansione") && names.contains(&"scrittura"));
    assert_eq!(snap.state, pc_jobs::JobState::Completed);
}

#[test]
fn destination_inside_library_source_updates_only_that_folder() {
    let e = env();
    let items = e.shots(4);
    let root = e.path("Foto");
    std::fs::create_dir_all(root.join("Altro")).unwrap();
    std::fs::write(root.join("Altro/gia.jpg"), jpeg(200, "2020:01:01 10:00:00")).unwrap();
    let source_id = e
        .store
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
    let (_, report) = e.run(&items, &e.options("Foto/Dolomiti", "Dolomiti"));
    assert_eq!(report.exported, 4);
    match report.import.as_ref().unwrap() {
        ImportTarget::UpdateFolder {
            source_id: id,
            folder,
            ..
        } => {
            assert_eq!(*id, source_id);
            assert!(folder.ends_with("Dolomiti"));
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(e.sources().len(), 1, "nessuna nuova sorgente");
    let catalog = e.catalog("Dolomiti").unwrap();
    assert_eq!(e.catalog_paths(catalog.id).len(), 4);
    // Aggiornata solo la cartella di destinazione: l'altra non è stata importata.
    let other = root.join("Altro/gia.jpg").display().to_string();
    let found = e
        .store
        .read(move |c| repo::media::get_by_path(c, &other))
        .unwrap();
    assert!(found.is_none(), "l'import non deve uscire dalla cartella");
}

#[test]
fn name_conflict_is_resolved_before_copying() {
    let e = env();
    let items = e.shots(3);
    let (_, first) = e.run(&items, &e.options("export", "Dolomiti 2026"));
    assert_eq!(first.exported, 3);
    let first_catalog = e.catalog("Dolomiti 2026").unwrap();

    // Stesso nome, nessuna scelta: errore all'avvio e nessun file copiato.
    let more = e.shots(5);
    let err = e
        .mgr
        .submit(
            JOB_KIND,
            raw_export::job_params(&more, &e.options("export_b", "dolomiti 2026")),
        )
        .unwrap_err();
    assert_eq!(err.code.as_str(), "INVALID_INPUT");
    assert!(err.detail.contains("Dolomiti 2026"), "{}", err.detail);
    assert!(!e.path("export_b").exists());

    // "Aggiungi all'esistente": le nuove foto entrano nel catalogo, che non viene duplicato.
    let mut add = e.options("export_b", "Dolomiti 2026");
    add.existing_catalog = Some(ExistingCatalog::Add);
    let (_, report) = e.run(&more, &add);
    let catalog = report.catalog.unwrap();
    assert!(!catalog.created);
    assert_eq!(catalog.id, first_catalog.id);
    assert_eq!(e.catalog_paths(first_catalog.id).len(), 3 + 5);
    assert_eq!(
        e.count("SELECT count(*) FROM catalog WHERE name LIKE 'Dolomiti%'"),
        1
    );

    // "Cambia nome" automatico: nuovo catalogo con nome unico.
    let mut rename = e.options("export_c", "Dolomiti 2026");
    rename.existing_catalog = Some(ExistingCatalog::Rename);
    let (_, report) = e.run(&e.shots(2), &rename);
    let catalog = report.catalog.unwrap();
    assert!(catalog.created);
    assert_eq!(catalog.name, "Dolomiti 2026 (2)");
}

#[test]
fn repeated_export_skips_files_and_still_catalogs_them() {
    let e = env();
    let items = e.shots(12);
    // 3 file già nella destinazione, con un altro contenuto: vanno saltati, non sovrascritti.
    std::fs::create_dir_all(e.path("export")).unwrap();
    for item in items.iter().take(3) {
        std::fs::write(
            e.path("export").join(&item.orig_name),
            jpeg(250, "2026:08:19 08:00:00"),
        )
        .unwrap();
    }
    let kept = hash(&e.path("export").join(&items[0].orig_name));
    let (_, report) = e.run(&items, &e.options("export", "Ripetuto"));
    assert_eq!((report.exported, report.skipped, report.failed), (9, 3, 0));
    assert_eq!(hash(&e.path("export").join(&items[0].orig_name)), kept);
    let catalog = report.catalog.unwrap();
    assert_eq!(catalog.photos, 12, "anche i saltati entrano nel catalogo");
    assert_eq!(e.catalog_paths(catalog.id).len(), 12);

    // Ripetizione completa senza "Sovrascrivi": tutto saltato.
    let mut again = e.options("export", "Ripetuto");
    again.existing_catalog = Some(ExistingCatalog::Add);
    let (_, report) = e.run(&items, &again);
    assert_eq!((report.exported, report.skipped), (0, 12));
    assert_eq!(e.catalog_paths(catalog.id).len(), 12);

    // Con "Sovrascrivi" il contenuto diventa quello della variante scelta, senza residui.
    let mut over = e.options("export", "Ripetuto");
    over.existing_catalog = Some(ExistingCatalog::Add);
    over.overwrite = true;
    let (_, report) = e.run(&items, &over);
    assert_eq!((report.exported, report.skipped), (12, 0));
    assert_eq!(
        hash(&e.path("export").join(&items[0].orig_name)),
        hash(Path::new(&items[0].source_path))
    );
    assert!(
        files_in(&e.path("export"))
            .iter()
            .all(|n| !n.starts_with('.')),
        "nessun temporaneo o backup rimasto"
    );
}

#[test]
fn review_ratings_and_preset_tags_reach_the_library() {
    let e = env();
    let items = e.shots(6);
    // Una foto è già in libreria con una valutazione diversa: prevale quella della revisione.
    let (_, first) = e.run(&items[5..], &e.options("export_pre", "Pre"));
    assert_eq!(first.exported, 1);
    let p5 = e.path("export_pre").join(&items[5].orig_name);
    e.store
        .write({
            let p = p5.display().to_string();
            move |c| {
                c.execute("UPDATE media_item SET rating = 1 WHERE path = ?1", [p])?;
                Ok(())
            }
        })
        .unwrap();
    // Stessa destinazione per la foto 5 (rating 5 in revisione).
    let mut opts = e.options("export_pre", "Pre");
    opts.existing_catalog = Some(ExistingCatalog::Add);
    let (_, _) = e.run(&items[5..], &opts);
    assert_eq!(e.rating_of(&p5), 5);

    let (_, report) = e.run(&items[..5], &e.options("export", "Valutate"));
    assert_eq!(report.exported, 5);
    for item in &items[..5] {
        let target = e.path("export").join(&item.orig_name);
        assert_eq!(e.rating_of(&target), item.rating, "{}", item.orig_name);
        assert_eq!(e.tags_of(&target), vec![item.preset.clone()]);
    }
    // Set esclusivo creato con i tag dei preset.
    let found = e
        .app
        .with(|c| {
            Ok(sets::list_sets(c)?
                .into_iter()
                .find(|s| s.name == PRESET_SET))
        })
        .unwrap()
        .expect("set Preset Fujifilm");
    assert!(found.exclusive);
    let mut tags = found.tag_names();
    tags.sort();
    assert_eq!(tags, vec!["C1 Kodachrome 64", "C2 Velvia"]);
}

#[test]
fn preset_tag_option_off_adds_ratings_only() {
    let e = env();
    let items = e.shots(4);
    let mut options = e.options("export", "SenzaTag");
    options.preset_as_tag = false;
    let (_, report) = e.run(&items, &options);
    assert_eq!(report.exported, 4);
    for item in &items {
        let target = e.path("export").join(&item.orig_name);
        assert_eq!(e.rating_of(&target), item.rating);
        assert!(e.tags_of(&target).is_empty());
    }
    assert!(e
        .app
        .with(|c| Ok(sets::list_sets(c)?
            .into_iter()
            .all(|s| s.name != PRESET_SET)))
        .unwrap());
}

#[test]
fn exclusive_set_replaces_the_previous_preset_tag() {
    let e = env();
    let items = e.shots(2);
    let (_, _) = e.run(&items, &e.options("export", "Uno"));
    let first = e.path("export").join(&items[0].orig_name);
    assert_eq!(e.tags_of(&first), vec!["C1 Kodachrome 64"]);
    // Nuova revisione dello stesso file con un altro preset: sostituisce il tag del set.
    let mut changed = items[..1].to_vec();
    changed[0].preset = "C2 Velvia".into();
    let mut again = e.options("export", "Uno");
    again.existing_catalog = Some(ExistingCatalog::Add);
    let (_, _) = e.run(&changed, &again);
    assert_eq!(e.tags_of(&first), vec!["C2 Velvia"]);
}

#[test]
fn error_on_one_file_does_not_stop_the_others() {
    let e = env();
    let items = e.shots(12);
    std::fs::remove_file(&items[4].source_path).unwrap();
    let (snap, report) = e.run(&items, &e.options("export", "Undici"));
    assert_eq!((report.exported, report.failed), (11, 1));
    assert_eq!(report.errors.len(), 1);
    assert_eq!(report.errors[0].name, items[4].orig_name);
    assert_eq!(report.errors[0].code, "NOT_FOUND");
    assert!(!report.errors[0].reason.is_empty());
    assert_eq!(report.catalog.unwrap().photos, 11);
    assert_eq!(snap.state, pc_jobs::JobState::CompletedWithErrors);
    assert!(!e.path("export").join(&items[4].orig_name).exists());
}

#[test]
fn cancel_after_copy_keeps_files_and_creates_no_catalog() {
    let e = env_with(true);
    let items = e.shots(5);
    let (snap, report) = e.run(&items, &e.options("export", "Annullato"));
    assert_eq!(snap.state, pc_jobs::JobState::Cancelled);
    assert!(report.cancelled);
    assert_eq!(report.exported, 5);
    assert_eq!(
        files_in(&e.path("export")).len(),
        5,
        "i file copiati restano"
    );
    assert!(e.catalog("Annullato").is_none(), "nessun catalogo");
    assert_eq!(e.count("SELECT count(*) FROM undo_log"), 0);
}

#[test]
fn single_undo_removes_catalog_ratings_and_tags() {
    let e = env();
    let items = e.shots(6);
    let baseline = e.count("SELECT count(*) FROM undo_log");
    let (_, report) = e.run(&items, &e.options("export", "Annullabile"));
    assert_eq!(report.exported, 6);
    let paths: Vec<PathBuf> = items
        .iter()
        .map(|i| e.path("export").join(&i.orig_name))
        .collect();
    assert!(paths.iter().skip(1).any(|p| e.rating_of(p) > 0));
    assert_eq!(
        e.count("SELECT count(*) FROM undo_log"),
        baseline + 1,
        "una sola voce di annullamento"
    );
    let app = e.app.clone();
    e.store
        .write(move |c| {
            let (description, payloads) = bulk::undo_last_collect(c)?.expect("annullabile");
            assert!(description.contains("Annullabile"), "{description}");
            app.tx(|a| payloads.iter().try_for_each(|p| sets::restore(a, p)))?;
            Ok(())
        })
        .unwrap();
    assert!(e.catalog("Annullabile").is_none(), "catalogo tolto");
    for p in &paths {
        assert_eq!(e.rating_of(p), 0, "valutazione tolta");
        assert!(e.tags_of(p).is_empty(), "tag tolti");
    }
    assert!(e
        .app
        .with(|c| Ok(sets::list_sets(c)?
            .into_iter()
            .all(|s| s.name != PRESET_SET)))
        .unwrap());
    // I file e la sorgente restano.
    assert_eq!(files_in(&e.path("export")).len(), 6);
    assert_eq!(e.sources().len(), 1);
}

#[test]
fn destination_equal_to_source_folder_never_overwrites_the_source() {
    let e = env();
    let items = e.shots(2);
    let mut options = e.options("revisione/C1", "Stessa");
    options.overwrite = true;
    let before = hash(Path::new(&items[0].source_path));
    let (_, report) = e.run(&items, &options);
    assert_eq!(report.failed, 2);
    assert_eq!(hash(Path::new(&items[0].source_path)), before);
}
