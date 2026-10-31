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
