//! Azioni di catalogo in blocchi, con un solo annullamento e diagnostica per file.
use crate::{JobPlan, PhaseSpec, RunSummary};
use pc_core::{AppError, AppResult};
use pc_store::{bulk, people, repo, Store, StoreResult};
use serde::{Deserialize, Serialize};

pub const JOB_KIND: &str = "bulk";
pub const JOB_THRESHOLD: usize = 200;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum BulkAction {
    AddTag {
        name: String,
        /// Altri tag del set esclusivo di `name`: vengono tolti dagli stessi file.
        #[serde(default)]
        exclusive: Vec<String>,
    },
    RemoveTag {
        name: String,
    },
    /// Rimuove più tag insieme (tasto 0: tutti i tag del set attivo).
    RemoveTags {
        names: Vec<String>,
    },
    AddCatalog {
        catalog: i64,
    },
    RemoveCatalog {
        catalog: i64,
    },
    SetLabel {
        group: String,
        value: Option<String>,
    },
    AssignPerson {
        person: i64,
    },
    RemoveLibrary,
}
impl BulkAction {
    pub fn destructive(&self) -> bool {
        matches!(self, Self::RemoveCatalog { .. } | Self::RemoveLibrary)
    }
    fn apply(&self, c: &rusqlite::Connection, ids: &[i64]) -> StoreResult<usize> {
        Ok(match self {
            Self::AddTag { name, exclusive } => bulk::add_tag(c, name, ids, exclusive)?.changed,
            Self::RemoveTag { name } => bulk::remove_tag(c, name, ids)?.changed,
            Self::RemoveTags { names } => bulk::remove_tags(c, names, ids)?.changed,
            Self::AddCatalog { catalog } => {
                if repo::catalog::get(c, *catalog)?.kind != pc_store::model::CatalogKind::Manual {
                    return Err(pc_store::StoreError::Invalid(
                        "catalogo smart non modificabile".into(),
                    ));
                }
                bulk::add_to_catalog(c, *catalog, ids)?.changed
            }
            Self::RemoveCatalog { catalog } => bulk::remove_from_catalog(c, *catalog, ids)?.changed,
            Self::SetLabel { group, value } => {
                bulk::set_label(c, group, value.as_deref(), ids)?.changed
            }
            Self::AssignPerson { person } => {
                let mut faces = Vec::new();
                for id in ids {
                    faces.extend(people::records(c, Some(*id))?.iter().map(|f| f.id));
                }
                people::assign(c, &faces, *person)?;
                ids.len()
            }
            Self::RemoveLibrary => repo::media::delete(c, ids)?,
        })
    }
}
pub fn plan(store: Store, params: &serde_json::Value) -> AppResult<JobPlan> {
    let ids: Vec<i64> = serde_json::from_value(params["ids"].clone())?;
    let action: BulkAction = serde_json::from_value(params["operation"].clone())?;
    if action.destructive() && params["confirmed"] != true {
        return Err(AppError::invalid_input("conferma richiesta"));
    }
    Ok(JobPlan {
        phases: vec![PhaseSpec::new("modifica", 1.0)],
        min_memory_mb: 16,
        run: Box::new(move |ctx| {
            let phase = ctx.progress.phase("modifica");
            phase.set_total(ids.len() as i64);
            phase.start();
            if !ctx.control.checkpoint() {
                return Ok(RunSummary {
                    cancelled: true,
                    failed: 0,
                });
            }
            let control = ctx.control.clone();
            let mut failed = 0;
            store.write(move|c|bulk::with_undo_group(c,"Modifica selezione",||{
            for chunk in ids.chunks(JOB_THRESHOLD){
                // Non attendere in pausa mentre si occupa il writer SQLite.
                if control.is_cancelled(){break;}
                let mut valid=Vec::new();for id in chunk {
                    match repo::media::get(c,*id){Ok(_)=>valid.push(*id),Err(e)=>{failed+=1;phase.inc_failed(1);tracing::error!(job_id=ctx.job_id,phase="modifica",path=format!("media:{id}"),code="NOT_FOUND",detail=%e,"elemento non trovato");}}
                }
                if !valid.is_empty(){action.apply(c,&valid)?;phase.inc_done(valid.len() as i64);}
            }
            phase.finish();Ok(failed)
        })).map(|failed|RunSummary{cancelled:ctx.control.is_cancelled(),failed}).map_err(Into::into)
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::JobManager;
    use std::{
        sync::Arc,
        time::{Duration, Instant},
    };
    struct Sink;
    impl crate::ProgressSink for Sink {
        fn on_progress(&self, _: &crate::JobSnapshot) {}
    }
    #[test]
    fn five_thousand_items_one_undo_and_partial_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let db = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
        db.write(|c| {
            for id in 1..=5000 {
                repo::media::upsert(
                    c,
                    &pc_store::model::NewMedia {
                        path: format!("fixture{id}.jpg"),
                        name: format!("fixture{id}.jpg"),
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
        jobs.register(JOB_KIND, Arc::new(move |p| plan(st.clone(), p)));
        let id=jobs.submit(JOB_KIND,serde_json::json!({"ids":(1..=5001).collect::<Vec<_>>(),"operation":{"action":"add_tag","name":"Viaggio"}})).unwrap();
        let until = Instant::now() + Duration::from_secs(30);
        loop {
            let s = jobs.snapshots().into_iter().find(|s| s.id == id).unwrap();
            if s.state.is_terminal() {
                assert_eq!(s.failed, 1);
                break;
            }
            assert!(Instant::now() < until);
            std::thread::sleep(Duration::from_millis(20));
        }
        db.write(|c| {
            assert_eq!(
                c.query_row("SELECT count(*) FROM media_tag", [], |r| r.get::<_, i64>(0))?,
                5000
            );
            bulk::undo_last(c)?;
            assert_eq!(
                c.query_row("SELECT count(*) FROM media_tag", [], |r| r.get::<_, i64>(0))?,
                0
            );
            Ok(())
        })
        .unwrap();
    }

    fn manager(db: &Store) -> JobManager {
        let jobs = JobManager::new(
            db.clone(),
            pc_governor::MemoryGovernor::new(256),
            1,
            Arc::new(Sink),
        );
        let st = db.clone();
        jobs.register(JOB_KIND, Arc::new(move |p| plan(st.clone(), p)));
        jobs
    }

    fn run(jobs: &JobManager, params: serde_json::Value) -> crate::JobSnapshot {
        let id = jobs.submit(JOB_KIND, params).unwrap();
        assert!(jobs.wait_idle(Duration::from_secs(30)));
        jobs.snapshots().into_iter().find(|s| s.id == id).unwrap()
    }

    fn media(c: &rusqlite::Connection, path: &str) -> StoreResult<i64> {
        repo::media::upsert(
            c,
            &pc_store::model::NewMedia {
                path: path.into(),
                name: path.into(),
                ext: "jpg".into(),
                ..Default::default()
            },
        )
    }

    #[test]
    fn assign_person_applies_only_to_faces_of_selection() {
        let tmp = tempfile::tempdir().unwrap();
        let db = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
        let person = db
            .write(|c| {
                for (i, axis) in [(1, 0), (2, 1), (3, 2)] {
                    let id = media(c, &format!("volti{i}.jpg"))?;
                    let mut embedding = vec![0.0; 128];
                    embedding[axis] = 1.0;
                    people::save_faces(
                        c,
                        id,
                        &[people::DetectedFace {
                            bbox: [0.1, 0.1, 0.3, 0.3],
                            quality: 0.99,
                            embedding,
                        }],
                        "sface",
                        "yunet+sface",
                        0.363,
                    )?;
                }
                people::create_person(c, Some("Giulia"))
            })
            .unwrap();
        let jobs = manager(&db);
        let s = run(
            &jobs,
            serde_json::json!({"ids":[1,2],"operation":{"action":"assign_person","person":person}}),
        );
        assert_eq!(s.state, crate::JobState::Completed, "{s:?}");
        let faces = db.read(|c| people::records(c, None)).unwrap();
        let assigned: Vec<(i64, Option<i64>)> =
            faces.iter().map(|f| (f.media_id, f.person_id)).collect();
        assert_eq!(
            assigned,
            vec![(1, Some(person)), (2, Some(person)), (3, None)]
        );
        // Assegnazione persistita: resta dopo la riapertura del catalogo.
        drop(jobs);
        drop(db);
        let db = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
        let again = db.read(|c| people::records(c, None)).unwrap();
        assert_eq!(again[0].person_id, Some(person));
        assert_eq!(again[1].person_id, Some(person));
    }

    #[test]
    fn remove_from_library_needs_confirmation_and_keeps_originals_on_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("Foto");
        std::fs::create_dir_all(&root).unwrap();
        let files: Vec<std::path::PathBuf> = (0..4usize)
            .map(|i| {
                let p = root.join(format!("IMG_{i}.jpg"));
                std::fs::write(&p, vec![i as u8; 1000 + i]).unwrap();
                p
            })
            .collect();
        let fingerprint = |p: &std::path::PathBuf| {
            (
                std::fs::read(p).unwrap(),
                std::fs::metadata(p).unwrap().modified().unwrap(),
            )
        };
        let before: Vec<_> = files.iter().map(fingerprint).collect();
        let db = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
        let paths: Vec<String> = files.iter().map(|p| p.display().to_string()).collect();
        let ids = db
            .write(move |c| {
                paths
                    .iter()
                    .map(|p| media(c, p))
                    .collect::<StoreResult<Vec<_>>>()
            })
            .unwrap();
        let jobs = manager(&db);
        let op = serde_json::json!({"action":"remove_library"});
        // Senza conferma il job non parte.
        let err = jobs
            .submit(
                JOB_KIND,
                serde_json::json!({"ids":&ids[..3],"operation":op}),
            )
            .unwrap_err();
        assert_eq!(err.code, pc_core::ErrorCode::InvalidInput);
        assert_eq!(db.read(repo::media::count).unwrap(), 4);

        let s = run(
            &jobs,
            serde_json::json!({"ids":&ids[..3],"operation":op,"confirmed":true}),
        );
        assert_eq!(s.state, crate::JobState::Completed, "{s:?}");
        assert_eq!(db.read(repo::media::count).unwrap(), 1);
        let kept = ids[3];
        assert!(db.read(move |c| repo::media::get(c, kept)).is_ok());
        // Gli originali restano sul disco, identici (contenuto e data di modifica).
        let after: Vec<_> = files.iter().map(fingerprint).collect();
        assert_eq!(before, after);
    }
}
