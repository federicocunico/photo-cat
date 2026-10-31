use pc_jobs::{JobManager, JobSnapshot, JobState, ProgressSink};
use pc_organize::{
    export,
    organize::{self, Method},
    paths, Scope,
};
use pc_store::{
    model::{NewMedia, NewSource},
    repo, Store,
};
use std::{
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};
struct Sink;
impl ProgressSink for Sink {
    fn on_progress(&self, _: &JobSnapshot) {}
}
fn fixture(root: &Path) -> Store {
    let source = root.join("source");
    std::fs::create_dir(&source).unwrap();
    let db = Store::open(&root.join("catalog.db"), Default::default()).unwrap();
    for n in 1..=2 {
        let dir = source.join(n.to_string());
        std::fs::create_dir(&dir).unwrap();
        std::fs::write(dir.join("IMG_0001.JPG"), format!("photo {n}")).unwrap();
    }
    let src = source.clone();
    db.write(move |c| {
        let source_id = repo::source::insert(
            c,
            &NewSource {
                path: src.to_string_lossy().into_owned(),
                recursive: true,
                ..Default::default()
            },
        )?;
        for n in 1..=2 {
            let id = repo::media::upsert(
                c,
                &NewMedia {
                    source_id: Some(source_id),
                    path: src
                        .join(n.to_string())
                        .join("IMG_0001.JPG")
                        .to_string_lossy()
                        .into_owned(),
                    name: "IMG_0001.JPG".into(),
                    ext: "jpg".into(),
                    size: 7,
                    ..Default::default()
                },
            )?;
            pc_store::bulk::add_tag(c, "Città; \"Firenze\"", &[id])?;
            pc_store::bulk::add_tag(c, "Viaggio", &[id])?;
        }
        Ok(())
    })
    .unwrap();
    db
}
fn run(db: &Store, kind: &str, params: serde_json::Value, cancel: bool) -> JobSnapshot {
    let jobs = JobManager::new(
        db.clone(),
        pc_governor::MemoryGovernor::new(512),
        1,
        Arc::new(Sink),
    );
    let st = db.clone();
    jobs.register(
        export::JOB_KIND,
        Arc::new(move |p| export::plan(st.clone(), p)),
    );
    let st = db.clone();
    jobs.register(
        organize::JOB_KIND,
        Arc::new(move |p| organize::plan(st.clone(), p)),
    );
    let id = jobs.submit(kind, params).unwrap();
    if cancel {
        jobs.cancel(id).unwrap();
    }
    let until = Instant::now() + Duration::from_secs(20);
    loop {
        let s = jobs.snapshot(id).unwrap();
        if s.state.is_terminal() {
            return s;
        }
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(10));
    }
}
#[test]
fn csv_and_json_export_multivalues_accents_and_scope() {
    let tmp = tempfile::tempdir().unwrap();
    let db = fixture(tmp.path());
    let opts = export::Options {
        scope: Scope::Selection { ids: vec![1] },
        destination: tmp.path().join("out.csv"),
        format: export::Format::Csv,
        separator: ';',
        bom: true,
        fields: vec!["path".into(), "tags".into(), "people".into()],
    };
    assert_eq!(
        run(
            &db,
            export::JOB_KIND,
            serde_json::to_value(&opts).unwrap(),
            false
        )
        .state,
        JobState::Completed
    );
    let text = std::fs::read_to_string(&opts.destination).unwrap();
    assert!(text.starts_with("\u{feff}schema_version;path;tags;people"));
    assert!(text.contains("Città;"));
    assert!(text.contains("\"\"Viaggio\"\""));
    assert_eq!(text.lines().count(), 2);
    let opts = export::Options {
        destination: tmp.path().join("out.json"),
        format: export::Format::Json,
        scope: Scope::Library,
        ..opts
    };
    assert_eq!(
        run(
            &db,
            export::JOB_KIND,
            serde_json::to_value(&opts).unwrap(),
            false
        )
        .state,
        JobState::Completed
    );
    let json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&opts.destination).unwrap()).unwrap();
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["records"].as_array().unwrap().len(), 2);
    assert_eq!(json["records"][0]["tags"][0], "Città; \"Firenze\"");
    assert_eq!(json["records"][0].as_object().unwrap().len(), 3);
    // Non sovrascrive un file già presente.
    assert_eq!(
        run(
            &db,
            export::JOB_KIND,
            serde_json::to_value(&opts).unwrap(),
            false
        )
        .state,
        JobState::Failed
    );
}
#[test]
fn cancelled_export_leaves_no_partial_or_destination() {
    let tmp = tempfile::tempdir().unwrap();
    let db = fixture(tmp.path());
    let dest = tmp.path().join("cancelled.json");
    let opts = export::Options {
        scope: Scope::Library,
        destination: dest.clone(),
        format: export::Format::Json,
        separator: ',',
        bom: false,
        fields: vec!["path".into()],
    };
    let result = run(
        &db,
        export::JOB_KIND,
        serde_json::to_value(opts).unwrap(),
        true,
    );
    assert_eq!(result.state, JobState::Cancelled);
    assert!(!dest.exists());
    assert!(!std::fs::read_dir(tmp.path()).unwrap().any(|e| e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".tmp")));
}
#[test]
fn pure_preview_sanitizes_reserved_names_and_conflicts() {
    for (raw, expected) in [
        ("CON", "_CON"),
        ("NUL.txt", "_NUL.txt"),
        ("Firenze:centro", "Firenze_centro"),
        ("..", "Senza valore"),
        ("LPT9", "_LPT9"),
    ] {
        assert_eq!(organize::sanitize(raw), expected);
    }
    let tmp = tempfile::tempdir().unwrap();
    let db = fixture(tmp.path());
    let dest = tmp.path().join("absent");
    let opts = organize::Options {
        scope: Scope::Library,
        destination: dest.clone(),
        levels: vec!["place".into()],
        method: Method::Copy,
    };
    let preview = organize::preview(&db, &opts).unwrap();
    assert!(!dest.exists());
    assert_eq!(preview.files, 2);
    assert_eq!(preview.conflicts, 1);
    assert_eq!(
        preview.entries[0].relative,
        Path::new("Senza luogo").join("IMG_0001.JPG")
    );
    assert_eq!(
        preview.entries[1].relative,
        Path::new("Senza luogo").join("IMG_0001 (2).JPG")
    );
}
#[test]
fn organize_undo_preserves_originals_existing_files_and_modified_outputs() {
    let tmp = tempfile::tempdir().unwrap();
    let db = fixture(tmp.path());
    let dest = tmp.path().join("organized");
    std::fs::create_dir_all(dest.join("Senza luogo")).unwrap();
    let existing = dest.join("Senza luogo/IMG_0001.JPG");
    std::fs::write(&existing, b"keep existing").unwrap();
    let opts = organize::Options {
        scope: Scope::Library,
        destination: dest.clone(),
        levels: vec!["place".into()],
        method: Method::Copy,
    };
    assert!(organize::preflight(&db, &opts).unwrap().available);
    assert_eq!(
        run(
            &db,
            organize::JOB_KIND,
            serde_json::to_value(opts).unwrap(),
            false
        )
        .state,
        JobState::Completed
    );
    assert_eq!(std::fs::read(&existing).unwrap(), b"keep existing");
    let modified = dest.join("Senza luogo/IMG_0001 (2).JPG");
    std::fs::write(&modified, b"user edit").unwrap();
    let op = db
        .read(|c| Ok(c.query_row("SELECT MAX(id) FROM organize_op", [], |r| r.get(0))?))
        .unwrap();
    let (removed, kept) = organize::undo(&db, op).unwrap();
    assert_eq!((removed, kept), (1, 1));
    assert_eq!(std::fs::read(&modified).unwrap(), b"user edit");
    for n in 1..=2 {
        assert_eq!(
            std::fs::read_to_string(tmp.path().join(format!("source/{n}/IMG_0001.JPG"))).unwrap(),
            format!("photo {n}")
        );
    }
    assert!(paths::writable_destination(&db, &tmp.path().join("source/forbidden.json")).is_err());
}
#[test]
fn link_preflight_and_undo_never_remove_link_targets() {
    let tmp = tempfile::tempdir().unwrap();
    let db = fixture(tmp.path());
    for method in [Method::Hardlink, Method::Symlink] {
        let dest = tmp.path().join(format!("{method:?}"));
        let opts = organize::Options {
            scope: Scope::Library,
            destination: dest.clone(),
            levels: vec!["year".into()],
            method,
        };
        let check = organize::preflight(&db, &opts).unwrap();
        if !check.available {
            assert_eq!(
                check.alternative,
                if method == Method::Symlink {
                    Method::Hardlink
                } else {
                    Method::Copy
                }
            );
            continue;
        }
        assert_eq!(
            run(
                &db,
                organize::JOB_KIND,
                serde_json::to_value(opts).unwrap(),
                false
            )
            .state,
            JobState::Completed
        );
        let op = db
            .read(|c| Ok(c.query_row("SELECT MAX(id) FROM organize_op", [], |r| r.get(0))?))
            .unwrap();
        assert_eq!(organize::undo(&db, op).unwrap().1, 0, "{method:?}");
        assert!(!dest.exists());
    }
    assert!(tmp.path().join("source/1/IMG_0001.JPG").is_file());
}
#[test]
fn preflight_decision_with_and_without_symlink_privilege() {
    use organize::{decide, LinkProbe};
    // Senza privilegio symlink e senza Modalità sviluppatore: propone hard link se riesce.
    let denied = decide(
        Method::Symlink,
        &LinkProbe {
            symlink: Some(false),
            hardlink: Some(true),
            cross_volume: false,
            developer_mode: Some(false),
        },
    );
    assert!(!denied.available);
    assert_eq!(denied.alternative, Method::Hardlink);
    assert!(
        denied.message.contains("Modalità sviluppatore"),
        "{}",
        denied.message
    );
    assert_eq!(denied.reason.as_deref(), Some("symlink_privilege"));
    // Con privilegio (Modalità sviluppatore attiva): disponibile.
    let allowed = decide(
        Method::Symlink,
        &LinkProbe {
            symlink: Some(true),
            hardlink: Some(true),
            cross_volume: false,
            developer_mode: Some(true),
        },
    );
    assert!(allowed.available);
    assert_eq!(allowed.alternative, Method::Symlink);
    assert!(allowed.developer_mode);
    // Symlink negato e sorgenti su un altro volume: niente hard link, si ripiega sulla copia.
    let other = decide(
        Method::Symlink,
        &LinkProbe {
            symlink: Some(false),
            hardlink: Some(false),
            cross_volume: true,
            developer_mode: Some(false),
        },
    );
    assert_eq!(other.alternative, Method::Copy);
    // Hard link verso un volume diverso: motivo esplicito.
    let cross = decide(
        Method::Hardlink,
        &LinkProbe {
            symlink: None,
            hardlink: Some(false),
            cross_volume: true,
            developer_mode: Some(true),
        },
    );
    assert!(!cross.available);
    assert_eq!(cross.alternative, Method::Copy);
    assert!(cross.message.contains("stesso volume"), "{}", cross.message);
    assert_eq!(cross.reason.as_deref(), Some("cross_volume"));
    assert_eq!(allowed.reason, None);
    assert!(decide(Method::Copy, &LinkProbe::default()).available);
    // Fuori da Windows (nessuna Modalità sviluppatore): il motivo riguarda la destinazione.
    let unix = decide(
        Method::Symlink,
        &LinkProbe {
            symlink: Some(false),
            hardlink: Some(false),
            cross_volume: false,
            developer_mode: None,
        },
    );
    assert!(!unix.available);
    assert_eq!(unix.reason.as_deref(), Some("symlink_denied"));
    assert!(!unix.message.contains("Windows"), "{}", unix.message);
    assert_eq!(unix.alternative, Method::Copy);
}
/// Arricchisce un elemento del fixture con tutti i dati esportabili.
fn enrich(db: &Store, id: i64) {
    use pc_store::model::{DateSource, Location, LocationSource, MediaDates, TechInfo};
    db.write(move |c| {
        repo::meta::set_tech(
            c,
            id,
            &TechInfo {
                format: Some("JPEG".into()),
                width: Some(4000),
                height: Some(3000),
                dpi_x: Some(300.0),
                dpi_y: Some(240.0),
                bit_depth: Some(8),
                ..Default::default()
            },
        )?;
        repo::meta::set_dates(
            c,
            id,
            &MediaDates {
                captured_at: Some("2023-07-14 10:22:01".into()),
                captured_source: Some(DateSource::ExifOriginal),
                modified_at: Some("2023-07-15 08:00:00".into()),
            },
        )?;
        repo::meta::set_location(
            c,
            id,
            &Location {
                name: Some("Firenze".into()),
                country: Some("IT".into()),
                lat: Some(43.77),
                lon: Some(11.25),
                source: LocationSource::Gps,
            },
        )?;
        let groups: Vec<String> = ["subject", "location", "environment"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let label = |group: &str, value: &str, score: f64| pc_store::analysis::AutoLabel {
            group: group.into(),
            value: value.into(),
            score,
        };
        pc_store::analysis::classification(
            c,
            id,
            &groups,
            &[
                label("subject", "ritratto", 0.87),
                label("location", "interno", 0.66),
                label("environment", "urbano", 0.55),
            ],
            &[0.01; 512],
            "clip:1",
            1,
        )?;
        pc_store::analysis::people(c, id, &[[0.1, 0.1, 0.3, 0.3, 0.9]], "yolox")?;
        let mut embedding = vec![0.0; 128];
        embedding[0] = 1.0;
        pc_store::people::save_faces(
            c,
            id,
            &[pc_store::people::DetectedFace {
                bbox: [0.1, 0.1, 0.2, 0.2],
                quality: 0.99,
                embedding,
            }],
            "sface",
            "yunet+sface",
            0.363,
        )?;
        let giulia = pc_store::people::create_person(c, Some("Giulia"))?;
        let face = pc_store::people::records(c, Some(id))?[0].id;
        pc_store::people::assign(c, &[face], giulia)?;
        let catalog = repo::catalog::create(
            c,
            "Ritratti 2023",
            pc_store::model::CatalogKind::Manual,
            None,
        )?;
        repo::catalog::add_items(c, catalog, &[id])?;
        Ok(())
    })
    .unwrap();
}
fn ends_with_file(path: &serde_json::Value, folder: &str) -> bool {
    path.as_str().unwrap().ends_with(&format!(
        "{folder}{}IMG_0001.JPG",
        std::path::MAIN_SEPARATOR
    ))
}
#[test]
fn library_export_has_every_required_column_with_default_fields() {
    let tmp = tempfile::tempdir().unwrap();
    let db = fixture(tmp.path());
    enrich(&db, 1);
    let fields: Vec<String> = export::FIELDS.iter().map(|f| f.to_string()).collect();
    // Colonne richieste dal requisito, con i nomi usati dall'export.
    let required = [
        "path",
        "name",
        "format",
        "width",
        "height",
        "dpi_x",
        "dpi_y",
        "bit_depth",
        "size",
        "captured_at",
        "mtime",
        "location",
        "location_source",
        "people_present",
        "people_count",
        "people",
        "subject",
        "location_type",
        "tags",
        "catalogs",
    ];
    let csv = export::Options {
        scope: Scope::Library,
        destination: tmp.path().join("tutto.csv"),
        format: export::Format::Csv,
        separator: ';',
        bom: false,
        fields: fields.clone(),
    };
    assert_eq!(
        run(
            &db,
            export::JOB_KIND,
            serde_json::to_value(&csv).unwrap(),
            false
        )
        .state,
        JobState::Completed
    );
    let text = std::fs::read_to_string(&csv.destination).unwrap();
    let header: Vec<&str> = text.lines().next().unwrap().split(';').collect();
    for f in required {
        assert!(header.contains(&f), "colonna {f} mancante in {header:?}");
    }
    assert_eq!(text.lines().count(), 3, "una riga per file della libreria");

    let json = export::Options {
        destination: tmp.path().join("tutto.json"),
        format: export::Format::Json,
        ..csv
    };
    assert_eq!(
        run(
            &db,
            export::JOB_KIND,
            serde_json::to_value(&json).unwrap(),
            false
        )
        .state,
        JobState::Completed
    );
    let doc: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&json.destination).unwrap()).unwrap();
    let records = doc["records"].as_array().unwrap();
    assert_eq!(records.len(), 2);
    let r = records
        .iter()
        .find(|r| ends_with_file(&r["path"], "1"))
        .unwrap();
    for f in required {
        assert!(!r[f].is_null(), "campo {f} vuoto: {r}");
    }
    assert_eq!(r["name"], "IMG_0001.JPG");
    assert_eq!(
        (r["width"].as_u64(), r["height"].as_u64()),
        (Some(4000), Some(3000))
    );
    assert_eq!(
        (r["dpi_x"].as_f64(), r["dpi_y"].as_f64()),
        (Some(300.0), Some(240.0))
    );
    assert_eq!(r["bit_depth"], 8);
    assert_eq!(r["size"], 7);
    assert_eq!(r["captured_at"], "2023-07-14 10:22:01");
    assert_eq!(r["location"], "Firenze");
    assert_eq!(r["location_source"], "gps");
    assert_eq!(r["people_present"], true);
    assert_eq!(r["people_count"], 1);
    assert_eq!(r["people"], serde_json::json!(["Giulia"]));
    assert_eq!(r["subject"], serde_json::json!(["ritratto"]));
    assert_eq!(r["location_type"], serde_json::json!(["interno"]));
    assert_eq!(r["catalogs"], serde_json::json!(["Ritratti 2023"]));
    assert_eq!(r["tags"].as_array().unwrap().len(), 2);
}
#[test]
fn catalog_scope_exports_only_catalog_files_and_selected_fields_with_confidence() {
    let tmp = tempfile::tempdir().unwrap();
    let db = fixture(tmp.path());
    enrich(&db, 2);
    let catalog: i64 = db
        .read(|c| {
            Ok(c.query_row(
                "SELECT id FROM catalog WHERE name = 'Ritratti 2023'",
                [],
                |r| r.get(0),
            )?)
        })
        .unwrap();
    let opts = export::Options {
        scope: Scope::Catalog { id: catalog },
        destination: tmp.path().join("ritratti.json"),
        format: export::Format::Json,
        separator: ',',
        bom: false,
        fields: vec![
            "path".into(),
            "captured_at".into(),
            "people".into(),
            "labels".into(),
        ],
    };
    assert_eq!(
        run(
            &db,
            export::JOB_KIND,
            serde_json::to_value(&opts).unwrap(),
            false
        )
        .state,
        JobState::Completed
    );
    let doc: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&opts.destination).unwrap()).unwrap();
    let records = doc["records"].as_array().unwrap();
    assert_eq!(records.len(), 1, "solo i file del catalogo");
    let r = records[0].as_object().unwrap();
    let mut keys: Vec<&str> = r.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, vec!["captured_at", "labels", "path", "people"]);
    assert!(ends_with_file(&r["path"], "2"));
    assert_eq!(r["people"], serde_json::json!(["Giulia"]));
    // Confidenza delle catalogazioni AI inclusa.
    let subject = r["labels"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["group"] == "subject")
        .unwrap();
    assert_eq!(subject["value"], "ritratto");
    assert!((subject["score"].as_f64().unwrap() - 0.87).abs() < 1e-9);
    assert_eq!(subject["origin"], "auto");
}
#[test]
fn preview_by_place_then_year_builds_place_year_folders() {
    use pc_store::model::{Location, LocationSource, MediaDates};
    let tmp = tempfile::tempdir().unwrap();
    let db = fixture(tmp.path());
    db.write(|c| {
        let place = |name: &str| Location {
            name: Some(name.into()),
            country: Some("IT".into()),
            lat: None,
            lon: None,
            source: LocationSource::Manual,
        };
        repo::meta::set_location(c, 1, &place("Firenze"))?;
        repo::meta::set_dates(
            c,
            1,
            &MediaDates {
                captured_at: Some("2023-07-14 10:22:01".into()),
                ..Default::default()
            },
        )?;
        // Il secondo file ha un luogo ma nessuna data.
        repo::meta::set_location(c, 2, &place("Roma"))?;
        Ok(())
    })
    .unwrap();
    let opts = organize::Options {
        scope: Scope::Library,
        destination: tmp.path().join("Organizzate"),
        levels: vec!["place".into(), "year".into()],
        method: Method::Copy,
    };
    let preview = organize::preview(&db, &opts).unwrap();
    assert_eq!(preview.files, 2);
    assert_eq!(preview.conflicts, 0);
    let mut relative: Vec<_> = preview.entries.iter().map(|e| e.relative.clone()).collect();
    relative.sort();
    assert_eq!(
        relative,
        vec![
            Path::new("Firenze").join("2023").join("IMG_0001.JPG"),
            Path::new("Roma").join("Senza anno").join("IMG_0001.JPG"),
        ]
    );
    assert!(!opts.destination.exists(), "l'anteprima non crea nulla");
}
