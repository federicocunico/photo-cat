//! Job `time_backfill`: per le librerie importate prima dello schema v5 rilegge
//! solo l'EXIF (fuso dichiarato e dispositivo) e calcola l'istante reale.
//! Non tocca miniature, analisi AI né i file originali.

use crate::metadata::read_exif;
use pc_core::AppResult;
use pc_jobs::{JobManager, JobPlan, PhaseSpec, RunSummary};
use pc_store::{timeline, Store, StoreResult};
use rusqlite::OptionalExtension;
use std::path::Path;

pub const JOB_KIND: &str = "time_backfill";
pub const PHASE: &str = "orari";
/// Marcatore in `settings_kv`: job già accodato per questa libreria.
pub const QUEUED_KEY: &str = "time_backfill_queued";
const CHUNK: usize = 500;

/// File ancora senza istante reale.
pub fn pending(store: &Store) -> AppResult<Vec<i64>> {
    Ok(store.read(|c| {
        let mut q = c.prepare(
            "SELECT d.media_id FROM media_dates d JOIN media_item m ON m.id = d.media_id
             WHERE d.instant_utc IS NULL AND d.captured_at IS NOT NULL ORDER BY d.media_id",
        )?;
        let rows = q.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
        Ok(rows)
    })?)
}

/// Accoda il job una sola volta per libreria, se ci sono file da completare.
pub fn queue_once(store: &Store, jobs: &JobManager) -> AppResult<Option<i64>> {
    let queued: Option<String> = store.read(|c| {
        Ok(c.query_row(
            "SELECT value FROM settings_kv WHERE key = ?1",
            [QUEUED_KEY],
            |r| r.get(0),
        )
        .optional()?)
    })?;
    if queued.is_some() || pending(store)?.is_empty() {
        return Ok(None);
    }
    let id = jobs.submit(JOB_KIND, serde_json::json!({}))?;
    store.write(move |c| {
        c.execute(
            "INSERT INTO settings_kv(key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![QUEUED_KEY, id.to_string()],
        )?;
        Ok(())
    })?;
    Ok(Some(id))
}

pub fn plan(store: Store, _params: &serde_json::Value) -> AppResult<JobPlan> {
    Ok(JobPlan {
        phases: vec![PhaseSpec::new(PHASE, 1.0)],
        min_memory_mb: 16,
        run: Box::new(move |ctx| {
            let phase = ctx.progress.phase(PHASE);
            let ids = pending(&store)?;
            phase.set_total(ids.len() as i64);
            phase.start();
            for chunk in ids.chunks(CHUNK) {
                if !ctx.control.checkpoint() {
                    return Ok(RunSummary {
                        cancelled: true,
                        failed: 0,
                    });
                }
                // Lettura EXIF fuori dal writer.
                let files: Vec<(i64, String)> = {
                    let chunk = chunk.to_vec();
                    store.read(move |c| {
                        let mut q =
                            c.prepare_cached("SELECT path FROM media_item WHERE id = ?1")?;
                        chunk
                            .iter()
                            .map(|id| Ok((*id, q.query_row([id], |r| r.get(0))?)))
                            .collect::<StoreResult<Vec<_>>>()
                    })?
                };
                let read: Vec<(i64, Option<i32>, Option<String>)> = files
                    .into_iter()
                    .map(|(id, path)| match read_exif(Path::new(&path)) {
                        Ok(Some(e)) => {
                            // Il fuso che accompagna la data usata dall'importazione.
                            let offset = if e.date_original.is_some() {
                                e.offset_original
                            } else {
                                e.offset_create
                            };
                            (id, offset, e.device)
                        }
                        Ok(None) => (id, None, None),
                        Err(err) => {
                            // File mancante o EXIF illeggibile: istante col fuso di sistema.
                            tracing::debug!(job_id = ctx.job_id, path = %path, detail = %err, "EXIF non riletto");
                            (id, None, None)
                        }
                    })
                    .collect();
                let n = read.len() as i64;
                store.write(move |c| {
                    let mut up = c.prepare_cached(
                        "UPDATE media_dates SET declared_offset_min = ?2, device = ?3 WHERE media_id = ?1",
                    )?;
                    for (id, offset, device) in &read {
                        up.execute(rusqlite::params![id, offset, device])?;
                        timeline::refresh_media(c, *id, &timeline::system_offset)?;
                    }
                    Ok(())
                })?;
                phase.inc_done(n);
            }
            phase.finish();
            Ok(RunSummary {
                cancelled: false,
                failed: 0,
            })
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::tests::{ascii, jpeg_with_exif};
    use exif::Tag;
    use std::sync::Arc;
    use std::time::Duration;

    struct Sink;
    impl pc_jobs::ProgressSink for Sink {
        fn on_progress(&self, _: &pc_jobs::JobSnapshot) {}
    }

    #[test]
    fn old_library_gets_instants_without_touching_thumbs_or_analysis() {
        let tmp = tempfile::tempdir().unwrap();
        let files = [
            ("a.jpg", Some("+02:00"), Some(("FUJIFILM", "X-T5"))),
            ("b.jpg", None, Some(("Apple", "iPhone 17 Pro Max"))),
            ("c.jpg", None, None),
        ];
        for (name, off, dev) in files {
            let mut fields = vec![ascii(Tag::DateTimeOriginal, "2026:08:19 10:43:00")];
            if let Some(o) = off {
                fields.push(ascii(Tag::OffsetTimeOriginal, o));
            }
            if let Some((make, model)) = dev {
                fields.push(ascii(Tag::Make, make));
                fields.push(ascii(Tag::Model, model));
            }
            std::fs::write(tmp.path().join(name), jpeg_with_exif(16, 16, &fields)).unwrap();
        }
        let db = Store::open(&tmp.path().join("catalog.db"), Default::default()).unwrap();
        let root = tmp.path().to_path_buf();
        // Come una libreria v4: date senza istante, miniature e analisi già presenti.
        db.write(move |c| {
            for (i, (name, _, _)) in files.iter().enumerate() {
                let id = i as i64 + 1;
                c.execute(
                    "INSERT INTO media_item(id,path,name,ext,size,mtime,added_at,updated_at,thumb)
                     VALUES (?1,?2,?3,'jpg',1,1,1,1,?4)",
                    rusqlite::params![id, root.join(name).display().to_string(), name, format!("thumb{id}")],
                )?;
                c.execute(
                    "INSERT INTO media_dates(media_id,captured_at,captured_source) VALUES (?1,'2026-08-19 10:43:00','exif_original')",
                    [id],
                )?;
                c.execute(
                    "INSERT INTO analysis_version(media_id,stage,model_ver,taxonomy_ver) VALUES (?1,'clip','clip-1',7)",
                    [id],
                )?;
            }
            Ok(())
        })
        .unwrap();
        let snapshot = |db: &Store| -> Vec<(String, i64)> {
            db.read(|c| {
                let mut q = c.prepare(
                    "SELECT m.thumb, a.taxonomy_ver FROM media_item m JOIN analysis_version a ON a.media_id = m.id ORDER BY m.id",
                )?;
                let r = q.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<Result<_, _>>()?;
                Ok(r)
            })
            .unwrap()
        };
        let before = snapshot(&db);
        let jobs = JobManager::new(
            db.clone(),
            pc_governor::MemoryGovernor::new(256),
            1,
            Arc::new(Sink),
        );
        let st = db.clone();
        jobs.register(JOB_KIND, Arc::new(move |p| plan(st.clone(), p)));

        assert!(queue_once(&db, &jobs).unwrap().is_some());
        assert!(jobs.wait_idle(Duration::from_secs(30)));
        assert!(
            queue_once(&db, &jobs).unwrap().is_none(),
            "accodato una volta sola"
        );

        type Row = (Option<i64>, Option<i32>, Option<String>, bool);
        let rows: Vec<Row> = db
            .read(|c| {
                let mut q = c.prepare(
                    "SELECT instant_utc, declared_offset_min, device, instant_presumed FROM media_dates ORDER BY media_id",
                )?;
                let r = q
                    .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
                    .collect::<Result<_, _>>()?;
                Ok(r)
            })
            .unwrap();
        assert!(rows.iter().all(|r| r.0.is_some()), "{rows:?}");
        assert_eq!(rows[0].1, Some(120));
        assert_eq!(rows[0].2.as_deref(), Some("Fujifilm X-T5"));
        assert!(!rows[0].3);
        assert_eq!(
            timeline::local_text(rows[0].0.unwrap(), 0).as_deref(),
            Some("2026-08-19 08:43:00")
        );
        assert_eq!(rows[1].2.as_deref(), Some("Apple iPhone 17 Pro Max"));
        assert!(rows[1].3 && rows[2].3, "senza fuso: presunto");
        assert_eq!(snapshot(&db), before, "miniature e analisi invariate");
    }
}
