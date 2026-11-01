//! "Nuova voce…" durante la correzione ed eliminazione di una voce usata.

use pc_ai::entries;
use pc_ai::taxonomy::{self, Category, Group};
use pc_core::ErrorCode;
use pc_jobs::manager::{JobManager, JobSnapshot, ProgressSink};
use pc_jobs::JobState;
use pc_store::model::NewMedia;
use pc_store::{bulk, repo, Store};
use std::sync::Arc;
use std::time::Duration;

struct Sink;
impl ProgressSink for Sink {
    fn on_progress(&self, _: &JobSnapshot) {}
}

fn library(n: i64) -> (tempfile::TempDir, Store, JobManager) {
    let tmp = tempfile::tempdir().unwrap();
    let db = Store::open(&tmp.path().join("catalog.db"), Default::default()).unwrap();
    db.write(move |c| {
        for i in 0..n {
            repo::media::upsert(
                c,
                &NewMedia {
                    path: format!("/foto/{i}.jpg"),
                    name: format!("{i}.jpg"),
                    ext: "jpg".into(),
                    ..Default::default()
                },
            )?;
        }
        Ok(())
    })
    .unwrap();
    let jobs = JobManager::new(
        db.clone(),
        pc_governor::MemoryGovernor::new(256),
        1,
        Arc::new(Sink),
    );
    let st = db.clone();
    jobs.register(
        pc_jobs::bulk::JOB_KIND,
        Arc::new(move |p| pc_jobs::bulk::plan(st.clone(), p)),
    );
    (tmp, db, jobs)
}

fn labels(db: &Store, group: &'static str, value: &'static str) -> i64 {
    db.read(move |c| {
        Ok(c.query_row(
            "SELECT count(*) FROM label WHERE grp=?1 AND value=?2 AND origin='manual'",
            [group, value],
            |r| r.get(0),
        )?)
    })
    .unwrap()
}

fn entries_named(db: &Store, group: &str, label: &str) -> usize {
    taxonomy::load(db)
        .unwrap()
        .group(group)
        .unwrap()
        .categories
        .iter()
        .filter(|c| taxonomy::same_name(&c.label, label))
        .count()
}

#[test]
fn new_entry_in_bulk_is_created_once_assigned_and_undoable() {
    let (_tmp, db, jobs) = library(60);
    let ids: Vec<i64> = (1..=60).collect();
    let before = taxonomy::load(&db).unwrap();
    let r = entries::add_and_assign(&db, &jobs, "subject", "Gita scolastica", vec![], Some(ids))
        .unwrap();
    assert!(r.created);
    assert_eq!(r.id, "gita_scolastica");
    assert_eq!(r.label, "Gita scolastica");
    assert!(jobs.wait_idle(Duration::from_secs(30)));
    let snap = jobs.snapshot(r.job.unwrap()).unwrap();
    assert_eq!(snap.state, JobState::Completed);
    assert_eq!(labels(&db, "subject", "gita_scolastica"), 60);
    let after = taxonomy::load(&db).unwrap();
    assert_eq!(after.version, before.version + 1);
    assert_eq!(
        after.analysis_version, before.analysis_version,
        "voce manuale"
    );
    let entry = after
        .group("subject")
        .unwrap()
        .categories
        .iter()
        .find(|c| c.id == "gita_scolastica")
        .unwrap();
    assert!(entry.is_manual());
    // Stesso nome di nuovo (anche da un'altra selezione): nessun doppione.
    let again =
        entries::add_and_assign(&db, &jobs, "subject", "gita  Scolastica", vec![], None).unwrap();
    assert!(!again.created);
    assert_eq!(again.id, "gita_scolastica");
    assert_eq!(entries_named(&db, "subject", "Gita scolastica"), 1);
    // L'annullamento toglie le etichette ma la voce resta.
    db.write(bulk::undo_last).unwrap();
    assert_eq!(labels(&db, "subject", "gita_scolastica"), 0);
    assert_eq!(entries_named(&db, "subject", "Gita scolastica"), 1);
}

#[test]
fn existing_name_reuses_the_entry() {
    let (_tmp, db, jobs) = library(1);
    let version = taxonomy::load(&db).unwrap().version;
    let r = entries::add_and_assign(&db, &jobs, "subject", "fiori", vec![], Some(vec![1])).unwrap();
    assert!(!r.created);
    assert_eq!(r.id, "fiori");
    assert_eq!(r.label, "Fiori");
    assert_eq!(
        taxonomy::load(&db).unwrap().version,
        version,
        "nessuna modifica"
    );
    assert!(jobs.wait_idle(Duration::from_secs(30)));
    assert_eq!(labels(&db, "subject", "fiori"), 1);
    // Anche gli accenti non contano.
    let (c, created) = entries::ensure(&db, "subject", "Opera d'àrte", vec![]).unwrap();
    assert!(!created);
    assert_eq!(c.id, "opera_arte");
}

#[test]
fn empty_name_creates_nothing() {
    let (_tmp, db, jobs) = library(1);
    let before = taxonomy::load(&db).unwrap();
    let err =
        entries::add_and_assign(&db, &jobs, "subject", "   ", vec![], Some(vec![1])).unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidInput);
    assert_eq!(taxonomy::load(&db).unwrap(), before);
    assert!(jobs.snapshots().is_empty(), "nessuna assegnazione");
    let err = entries::ensure(&db, "nessuno", "Barche", vec![]).unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidInput);
}

#[test]
fn entry_with_description_is_automatic() {
    let (_tmp, db, _jobs) = library(0);
    let before = taxonomy::load(&db).unwrap();
    let (c, created) = entries::ensure(
        &db,
        "subject",
        "Barche",
        vec!["a photo of boats".into(), " ".into()],
    )
    .unwrap();
    assert!(created && !c.is_manual());
    assert_eq!(c.prompts, vec!["a photo of boats".to_string()]);
    let after = taxonomy::load(&db).unwrap();
    assert!(after.analysis_version > before.analysis_version);
}

#[test]
fn deleting_a_used_entry_removes_it_from_25_photos_and_can_be_undone() {
    let (_tmp, db, _jobs) = library(30);
    let current = taxonomy::load(&db).unwrap();
    let mut next = current.clone();
    let manual = |id: &str, label: &str| Category {
        id: id.into(),
        label: label.into(),
        enabled: true,
        prompts: vec![],
        threshold: None,
    };
    next.groups.push(Group {
        id: "evento".into(),
        label: "Evento".into(),
        threshold: 0.25,
        categories: vec![
            manual("compleanno", "Compleanno"),
            manual("laurea", "Laurea"),
        ],
    });
    next.version += 1;
    taxonomy::save(&db, next, current.version).unwrap();
    db.write(|c| {
        let ids: Vec<i64> = (1..=25).collect();
        bulk::set_label(c, "evento", Some("laurea"), &ids)?;
        bulk::set_label(c, "evento", Some("compleanno"), &[26, 27])?;
        Ok(())
    })
    .unwrap();
    assert_eq!(entries::usage(&db, "evento", "laurea").unwrap(), 25);
    let before = taxonomy::load(&db).unwrap();
    assert_eq!(entries::delete(&db, "evento", "laurea").unwrap(), 25);
    assert_eq!(labels(&db, "evento", "laurea"), 0);
    assert_eq!(
        labels(&db, "evento", "compleanno"),
        2,
        "le altre voci restano"
    );
    let after = taxonomy::load(&db).unwrap();
    assert_eq!(entries_named(&db, "evento", "Laurea"), 0);
    assert_eq!(
        after.analysis_version, before.analysis_version,
        "voce manuale"
    );
    // Annullamento: etichette e voce tornano.
    let undone = db.write(bulk::undo_last).unwrap();
    assert_eq!(undone.as_deref(), Some("Elimina voce evento = laurea"));
    assert_eq!(labels(&db, "evento", "laurea"), 25);
    let restored = taxonomy::load(&db).unwrap();
    assert_eq!(entries_named(&db, "evento", "Laurea"), 1);
    assert_eq!(restored.version, after.version + 1);
    // Una voce senza foto si elimina (e si ripristina) lo stesso.
    assert_eq!(entries::delete(&db, "subject", "cibo").unwrap(), 0);
    let t = taxonomy::load(&db).unwrap();
    assert!(
        t.analysis_version > restored.analysis_version,
        "voce automatica"
    );
    db.write(bulk::undo_last).unwrap();
    assert_eq!(entries_named(&db, "subject", "Cibo"), 1);
    // Sconosciuta o ultima voce del gruppo: errore.
    assert!(entries::delete(&db, "subject", "nessuna").is_err());
}
