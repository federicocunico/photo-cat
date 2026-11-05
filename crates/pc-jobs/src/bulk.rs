//! Azioni di catalogo in blocchi, con un solo annullamento e diagnostica per file.
use crate::{JobPlan, PhaseSpec, RunSummary};
use pc_core::{AppError, AppResult};
use pc_store::query::Target;
use pc_store::{bulk, people, repo, Store, StoreResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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
    #[serde(alias = "remove_catalog")]
    ExcludeFromCatalog {
        catalog: i64,
    },
    RestoreToCatalog {
        catalog: i64,
    },
    /// "Ripristina" da un catalogo di sistema (Cestino, Duplicati, Qualità insufficiente).
    RestoreFromSystem {
        catalog: i64,
    },
    /// "Sposta nel cestino": toglie le foto anche dai cataloghi manuali.
    MoveToTrash,
    SetLabel {
        group: String,
        value: Option<String>,
    },
    AssignPerson {
        person: i64,
    },
    /// Luogo manuale (`Some`) o ripristino del luogo automatico (`None`).
    SetPlace {
        place: Option<bulk::PlaceInput>,
    },
    RemoveLibrary,
    /// Rifiuta l'etichetta automatica `grp = value` (con `score < max_score` se dato).
    RejectAutoLabels {
        grp: String,
        value: String,
        #[serde(default)]
        max_score: Option<f64>,
    },
}

/// Calcolo del luogo automatico per "Ripristina luogo automatico".
pub type PlaceResolver =
    Arc<dyn Fn(&bulk::PlaceInputs) -> Option<pc_store::model::Location> + Send + Sync>;

/// Cancella dalla cache le miniature non più usate dopo "Rimuovi dalla libreria".
pub type ThumbRemover = Arc<dyn Fn(&[String]) + Send + Sync>;

impl BulkAction {
    pub fn destructive(&self) -> bool {
        matches!(self, Self::RemoveLibrary)
    }
    fn apply(
        &self,
        c: &rusqlite::Connection,
        ids: &[i64],
        resolver: Option<&PlaceResolver>,
        removed_thumbs: &mut Vec<String>,
    ) -> StoreResult<usize> {
        Ok(match self {
            Self::SetPlace { place } => {
                let none = |_: &bulk::PlaceInputs| None;
                let resolve: &dyn Fn(&bulk::PlaceInputs) -> Option<pc_store::model::Location> =
                    match resolver {
                        Some(r) => r.as_ref(),
                        None => &none,
                    };
                bulk::set_place(c, ids, place.as_ref(), resolve)?.changed
            }
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
            Self::ExcludeFromCatalog { catalog } => {
                bulk::exclude_from_catalog(c, *catalog, ids)?.changed
            }
            Self::RestoreToCatalog { catalog } => {
                bulk::restore_to_catalog(c, *catalog, ids)?.changed
            }
            Self::RestoreFromSystem { catalog } => {
                bulk::restore_from_system(c, *catalog, ids)?.changed
            }
            Self::MoveToTrash => bulk::move_to_trash(c, ids)?.changed,
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
            Self::RejectAutoLabels {
                grp,
                value,
                max_score,
            } => bulk::reject_auto_labels(c, grp, value, *max_score, ids)?.changed,
            Self::RemoveLibrary => {
                let removed = pc_store::sources::remove_media(c, ids)?;
                removed_thumbs.extend(removed.thumbs);
                removed.items
            }
        })
    }
}
pub fn plan(store: Store, params: &serde_json::Value) -> AppResult<JobPlan> {
    plan_with(store, None, params)
}

/// Come [`plan`], con il calcolo del luogo automatico per `SetPlace`.
pub fn plan_with(
    store: Store,
    resolver: Option<PlaceResolver>,
    params: &serde_json::Value,
) -> AppResult<JobPlan> {
    plan_full(store, resolver, None, params)
}

/// Come [`plan_with`], con la pulizia delle miniature dopo la rimozione.
pub fn plan_full(
    store: Store,
    resolver: Option<PlaceResolver>,
    remover: Option<ThumbRemover>,
    params: &serde_json::Value,
) -> AppResult<JobPlan> {
    // `target` (elenco o filtro con eccezioni); `ids` per i job salvati dalle versioni precedenti.
    let target: Target = match params.get("ids").filter(|v| !v.is_null()) {
        Some(ids) => Target::ids(serde_json::from_value(ids.clone())?),
        None => serde_json::from_value(params["target"].clone())?,
    };
    let action: BulkAction = serde_json::from_value(params["operation"].clone())?;
    if action.destructive() && params["confirmed"] != true {
        return Err(AppError::invalid_input("conferma richiesta"));
    }
    Ok(JobPlan {
        phases: vec![PhaseSpec::new("modifica", 1.0)],
        min_memory_mb: 16,
        run: Box::new(move |ctx| {
            let phase = ctx.progress.phase("modifica");
            let total = store.read(|c| target.count(c))?;
            phase.set_total(total);
            phase.start();
            if !ctx.control.checkpoint() {
                return Ok(RunSummary {
                    cancelled: true,
                    failed: 0,
                });
            }
            let control = ctx.control.clone();
            let mut failed = 0;
            let mut thumbs = Vec::new();
            let cancelled = ctx.control.clone();
            let result = store.write(move|c|bulk::with_undo_group(c,"Modifica selezione",||{
            // Il target si risolve a blocchi: anche "tutta la libreria" non diventa mai un elenco unico.
            target.for_each_chunk(c, JOB_THRESHOLD, |chunk| {
                // Non attendere in pausa mentre si occupa il writer SQLite.
                if control.is_cancelled(){return Ok(());}
                let mut valid=Vec::new();for id in chunk {
                    match repo::media::get(c,*id){Ok(_)=>valid.push(*id),Err(e)=>{failed+=1;phase.inc_failed(1);tracing::error!(job_id=ctx.job_id,phase="modifica",path=format!("media:{id}"),code="NOT_FOUND",detail=%e,"elemento non trovato");}}
                }
                if !valid.is_empty(){action.apply(c,&valid,resolver.as_ref(),&mut thumbs)?;phase.inc_done(valid.len() as i64);}
                Ok(())
            })?;
            phase.finish();Ok((failed,thumbs))
        }));
            let (failed, thumbs) = result?;
            if let Some(remove) = &remover {
                if !thumbs.is_empty() {
                    remove(&thumbs);
                }
            }
            Ok(RunSummary {
                cancelled: cancelled.is_cancelled(),
                failed,
            })
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
    fn move_to_trash_over_several_chunks_is_one_undo() {
        let tmp = tempfile::tempdir().unwrap();
        let db = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
        let (ids, cat, trash) = db
            .write(|c| {
                let ids = (0..450)
                    .map(|i| media(c, &format!("t{i}.jpg")))
                    .collect::<StoreResult<Vec<_>>>()?;
                let cat = repo::catalog::create(
                    c,
                    "Viaggio",
                    pc_store::model::CatalogKind::Manual,
                    None,
                )?;
                repo::catalog::add_items(c, cat, &ids)?;
                let trash = repo::catalog::system_id(c, pc_store::model::SYSTEM_TRASH)?;
                Ok((ids, cat, trash))
            })
            .unwrap();
        let jobs = manager(&db);
        let s = run(
            &jobs,
            serde_json::json!({"ids": ids, "operation": {"action": "move_to_trash"}}),
        );
        assert_eq!(s.failed, 0);
        db.write(move |c| {
            assert!(repo::catalog::manual_items(c, cat)?.is_empty());
            assert_eq!(repo::catalog::manual_items(c, trash)?.len(), 450);
            bulk::undo_last(c)?;
            assert_eq!(repo::catalog::manual_items(c, cat)?.len(), 450);
            assert!(repo::catalog::manual_items(c, trash)?.is_empty());
            Ok(())
        })
        .unwrap();
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
                            ..Default::default()
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

    /// Scenari "Luogo in bulk" e "Ripristino del luogo automatico".
    #[test]
    fn set_place_in_bulk_reports_failures_and_resets_with_resolver() {
        use pc_store::model::{Location, LocationSource};
        let tmp = tempfile::tempdir().unwrap();
        let db = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
        let ids = db
            .write(|c| {
                (0..300)
                    .map(|i| media(c, &format!("D:/Foto/Viaggio Giappone/D7000/{i}.jpg")))
                    .collect::<StoreResult<Vec<_>>>()
            })
            .unwrap();
        let jobs = JobManager::new(
            db.clone(),
            pc_governor::MemoryGovernor::new(256),
            1,
            Arc::new(Sink),
        );
        let st = db.clone();
        let resolver: PlaceResolver = Arc::new(|_| {
            Some(Location {
                name: Some("Viaggio Giappone".into()),
                country: None,
                lat: None,
                lon: None,
                source: LocationSource::Directory,
            })
        });
        jobs.register(
            JOB_KIND,
            Arc::new(move |p| plan_with(st.clone(), Some(resolver.clone()), p)),
        );
        let mut with_missing = ids.clone();
        with_missing.push(99_999);
        let s = run(
            &jobs,
            serde_json::json!({"ids":with_missing,"operation":{"action":"set_place","place":{"name":"Giappone"}}}),
        );
        assert_eq!(s.failed, 1, "il file inesistente è riportato come fallito");
        let manual = |db: &Store, name: &str, source: &str| {
            let (name, source) = (name.to_string(), source.to_string());
            db.read(move |c| {
                Ok(c.query_row(
                    "SELECT count(*) FROM location WHERE name=?1 AND source=?2",
                    [name, source],
                    |r| r.get::<_, i64>(0),
                )?)
            })
            .unwrap()
        };
        assert_eq!(manual(&db, "Giappone", "manual"), 300);
        let s = run(
            &jobs,
            serde_json::json!({"ids":&ids[..10],"operation":{"action":"set_place","place":null}}),
        );
        assert_eq!(s.state, crate::JobState::Completed, "{s:?}");
        assert_eq!(manual(&db, "Viaggio Giappone", "directory"), 10);
        assert_eq!(manual(&db, "Giappone", "manual"), 290);
        // Un solo annullamento per job.
        db.write(bulk::undo_last).unwrap();
        assert_eq!(manual(&db, "Giappone", "manual"), 300);
    }

    /// Libreria sintetica di `n` elementi (inserimento diretto, veloce anche con 100.000).
    fn seed(db: &Store, from: i64, to: i64) {
        db.write(move |c| {
            let mut st = c.prepare(
                "INSERT INTO media_item(id, path, name, ext, size, mtime, added_at, updated_at)
                 VALUES (?1, ?2, ?2, 'jpg', 1, 0, ?1, ?1)",
            )?;
            for id in from..=to {
                st.execute(rusqlite::params![id, format!("f{id}.jpg")])?;
            }
            Ok(())
        })
        .unwrap();
    }

    fn run_to_end(jobs: &JobManager, params: serde_json::Value) -> crate::JobSnapshot {
        let id = jobs.submit(JOB_KIND, params).unwrap();
        let until = Instant::now() + Duration::from_secs(120);
        loop {
            let s = jobs.snapshots().into_iter().find(|s| s.id == id).unwrap();
            if s.state.is_terminal() {
                return s;
            }
            assert!(Instant::now() < until);
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn tagged(db: &Store) -> i64 {
        db.read(|c| Ok(c.query_row("SELECT count(*) FROM media_tag", [], |r| r.get(0))?))
            .unwrap()
    }

    #[test]
    fn select_all_on_a_large_library_minus_three_tags_exactly_the_rest() {
        use pc_store::query::{self, LibraryFilter};
        let tmp = tempfile::tempdir().unwrap();
        let db = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
        seed(&db, 1, 100_000);
        let jobs = JobManager::new(
            db.clone(),
            pc_governor::MemoryGovernor::new(256),
            1,
            Arc::new(Sink),
        );
        let st = db.clone();
        jobs.register(JOB_KIND, Arc::new(move |p| plan(st.clone(), p)));
        // "Seleziona tutto": solo conteggio e id massimo, nessun elenco di id.
        let base = db
            .read(|c| query::selection_base(c, &LibraryFilter::default()))
            .unwrap();
        assert_eq!((base.count, base.max_id), (100_000, 100_000));
        let target = Target::Filter {
            filter: LibraryFilter::default(),
            max_id: base.max_id,
            except: vec![7, 50_000, 99_999],
        };
        assert_eq!(db.read(|c| target.count(c)).unwrap(), 99_997);
        let s = run_to_end(
            &jobs,
            serde_json::json!({"target": target, "operation": {"action": "add_tag", "name": "Mare"}}),
        );
        assert_eq!(s.failed, 0);
        assert_eq!(tagged(&db), 99_997);
        let excluded: i64 = db
            .read(|c| {
                Ok(c.query_row(
                    "SELECT count(*) FROM media_tag WHERE media_id IN (7, 50000, 99999)",
                    [],
                    |r| r.get(0),
                )?)
            })
            .unwrap();
        assert_eq!(excluded, 0);
    }

    #[test]
    fn items_imported_after_select_all_are_not_affected() {
        use pc_store::query::{self, LibraryFilter};
        let tmp = tempfile::tempdir().unwrap();
        let db = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
        seed(&db, 1, 500);
        let jobs = JobManager::new(
            db.clone(),
            pc_governor::MemoryGovernor::new(256),
            1,
            Arc::new(Sink),
        );
        let st = db.clone();
        jobs.register(JOB_KIND, Arc::new(move |p| plan(st.clone(), p)));
        let base = db
            .read(|c| query::selection_base(c, &LibraryFilter::default()))
            .unwrap();
        // Un import aggiunge altre foto che soddisfano il filtro.
        seed(&db, 501, 800);
        let target = Target::Filter {
            filter: LibraryFilter::default(),
            max_id: base.max_id,
            except: vec![],
        };
        run_to_end(
            &jobs,
            serde_json::json!({"target": target, "operation": {"action": "add_tag", "name": "X"}}),
        );
        assert_eq!(
            tagged(&db),
            500,
            "solo le foto presenti al momento della selezione"
        );
    }

    #[test]
    fn smart_exclusion_job_legacy_alias_and_undo() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("photo.jpg");
        std::fs::write(&file, b"original").unwrap();
        let db = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
        let path = file.to_string_lossy().to_string();
        let (id, cat) = db
            .write(move |c| {
                let id = media(c, &path)?;
                let cat = repo::catalog::create(
                    c,
                    "Smart",
                    pc_store::model::CatalogKind::Smart,
                    Some(&serde_json::json!({"field":"format","cmp":"eq","value":"jpg"})),
                )?;
                Ok((id, cat))
            })
            .unwrap();
        let legacy: BulkAction =
            serde_json::from_value(serde_json::json!({"action":"remove_catalog","catalog":cat}))
                .unwrap();
        assert!(matches!(legacy, BulkAction::ExcludeFromCatalog { .. }));
        assert!(!legacy.destructive());
        assert!(!BulkAction::RestoreToCatalog { catalog: cat }.destructive());
        let jobs = manager(&db);
        let outcome = run(
            &jobs,
            serde_json::json!({"ids":[id],"operation":{"action":"exclude_from_catalog","catalog":cat},"confirmed":false}),
        );
        assert_eq!(outcome.state, crate::JobState::Completed);
        db.write(move |c| {
            let filter = pc_store::query::LibraryFilter {
                catalog_id: Some(cat),
                ..Default::default()
            };
            assert_eq!(pc_store::query::count(c, &filter)?, 0);
            bulk::undo_last(c)?;
            assert_eq!(pc_store::query::count(c, &filter)?, 1);
            assert_eq!(pc_store::query::count(c, &Default::default())?, 1);
            Ok(())
        })
        .unwrap();
        assert_eq!(std::fs::read(file).unwrap(), b"original");
    }

    #[test]
    fn reject_auto_labels_over_several_chunks_is_one_undo() {
        let tmp = tempfile::tempdir().unwrap();
        let db = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
        let ids = db
            .write(|c| {
                let ids = (0..450)
                    .map(|i| media(c, &format!("r{i}.jpg")))
                    .collect::<StoreResult<Vec<_>>>()?;
                for (i, id) in ids.iter().enumerate() {
                    let score = if i % 2 == 0 { 0.3 } else { 0.8 };
                    c.execute(
                        "INSERT INTO label(media_id, grp, value, score, origin, model_ver, taxonomy_ver) VALUES (?1, 'subject', 'natura', ?2, 'auto', 'm', 3)",
                        rusqlite::params![id, score],
                    )?;
                }
                Ok(ids)
            })
            .unwrap();
        let jobs = manager(&db);
        let s = run(
            &jobs,
            serde_json::json!({"ids": ids, "operation": {"action": "reject_auto_labels", "grp": "subject", "value": "natura", "max_score": 0.5}}),
        );
        assert_eq!(s.failed, 0);
        let count = |c: &rusqlite::Connection, sql: &str| -> StoreResult<i64> {
            Ok(c.query_row(sql, [], |r| r.get(0))?)
        };
        db.write(move |c| {
            assert_eq!(count(c, "SELECT count(*) FROM label")?, 225);
            assert_eq!(count(c, "SELECT count(*) FROM label_rejection")?, 225);
            bulk::undo_last(c)?;
            assert_eq!(count(c, "SELECT count(*) FROM label WHERE score = 0.3 AND model_ver = 'm' AND taxonomy_ver = 3")?, 225);
            assert_eq!(count(c, "SELECT count(*) FROM label")?, 450);
            assert_eq!(count(c, "SELECT count(*) FROM label_rejection")?, 0);
            assert_eq!(count(c, "SELECT count(*) FROM undo_log")?, 0);
            Ok(())
        })
        .unwrap();
    }
}
