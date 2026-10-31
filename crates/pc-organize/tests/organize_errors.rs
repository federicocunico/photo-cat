//! Organizzazione come job: un file sorgente mancante non ferma il job; l'errore finisce
//! nel registro con il percorso e gli altri file vengono creati.
//! Binario separato: installa un subscriber globale (il job gira su un altro thread).

use pc_jobs::{JobManager, JobSnapshot, JobState, ProgressSink};
use pc_organize::{
    organize::{self, Method},
    Scope,
};
use pc_store::errorlog::{self, DbErrorSink, ErrorQuery};
use pc_store::{
    model::{NewMedia, NewSource},
    repo, Store,
};
use std::{sync::Arc, time::Duration};

struct Sink;
impl ProgressSink for Sink {
    fn on_progress(&self, _: &JobSnapshot) {}
}

#[test]
fn missing_source_file_completes_with_errors_logged_with_path() {
    let tmp = tempfile::tempdir().unwrap();
    let (sub, diag) = pc_diag::build(
        &tmp.path().join("logs"),
        &pc_core::config::LoggingConfig::default(),
    );
    tracing::subscriber::set_global_default(sub).unwrap();

    let source = tmp.path().join("source");
    std::fs::create_dir(&source).unwrap();
    let db = Store::open(&tmp.path().join("catalog.db"), Default::default()).unwrap();
    diag.hub().add_sink(Arc::new(DbErrorSink::new(db.clone())));
    let files: Vec<_> = (1..=3)
        .map(|n| {
            let p = source.join(format!("IMG_000{n}.JPG"));
            std::fs::write(&p, format!("photo {n}")).unwrap();
            p
        })
        .collect();
    let (src, paths) = (
        source.display().to_string(),
        files
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>(),
    );
    db.write(move |c| {
        let source_id = repo::source::insert(
            c,
            &NewSource {
                path: src,
                recursive: true,
                ..Default::default()
            },
        )?;
        for p in paths {
            repo::media::upsert(
                c,
                &NewMedia {
                    source_id: Some(source_id),
                    name: p.rsplit(std::path::MAIN_SEPARATOR).next().unwrap().into(),
                    path: p,
                    ext: "jpg".into(),
                    size: 7,
                    ..Default::default()
                },
            )?;
        }
        Ok(())
    })
    .unwrap();
    // Il secondo file sparisce dal disco dopo l'import.
    std::fs::remove_file(&files[1]).unwrap();

    let dest = tmp.path().join("Organizzate");
    let opts = organize::Options {
        scope: Scope::Library,
        destination: dest.clone(),
        levels: vec!["place".into()],
        method: Method::Copy,
    };
    let jobs = JobManager::new(
        db.clone(),
        pc_governor::MemoryGovernor::new(512),
        1,
        Arc::new(Sink),
    );
    let st = db.clone();
    jobs.register(
        organize::JOB_KIND,
        Arc::new(move |p| organize::plan(st.clone(), p)),
    );
    let id = jobs
        .submit(organize::JOB_KIND, serde_json::to_value(&opts).unwrap())
        .unwrap();
    assert!(jobs.wait_idle(Duration::from_secs(30)));
    let s = jobs.snapshot(id).unwrap();
    assert_eq!(s.state, JobState::CompletedWithErrors, "{s:?}");
    assert_eq!(s.failed, 1);

    // Gli altri file sono stati creati.
    for name in ["IMG_0001.JPG", "IMG_0003.JPG"] {
        assert!(dest.join("Senza luogo").join(name).is_file(), "{name}");
    }
    assert!(!dest.join("Senza luogo").join("IMG_0002.JPG").exists());

    // Errore nel registro, con job, fase e percorso del file.
    db.flush().unwrap();
    let errors = db
        .read(|c| {
            errorlog::query(
                c,
                &ErrorQuery {
                    level: Some("ERROR".into()),
                    job_id: Some(id),
                    ..Default::default()
                },
            )
        })
        .unwrap();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].phase.as_deref(), Some("organizzazione"));
    assert_eq!(
        errors[0].path.as_deref(),
        Some(files[1].display().to_string().as_str())
    );
    assert!(errors[0].code.is_some());
}
