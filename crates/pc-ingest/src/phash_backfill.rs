//! Job `phash_backfill` (change `perceptual-hash-similarity`): per le foto già in
//! libreria calcola l'impronta percettiva dalla miniatura in cache. Non riapre gli
//! originali né tocca miniature e analisi. È riprendibile: `pending` rilegge le foto
//! ancora senza impronta.

use crate::phash;
use crate::thumb::ThumbCache;
use pc_core::AppResult;
use pc_jobs::{JobControl, JobPlan, PhaseCounters, PhaseSpec, RunSummary};
use pc_store::{Store, StoreResult};
use rusqlite::OptionalExtension;

pub const JOB_KIND: &str = "phash_backfill";
pub const PHASE: &str = "impronte";
/// Marcatore in `settings_kv`: recupero concluso per questa libreria (anche con errori per
/// file). Si scrive solo a fine job: un recupero annullato o interrotto riparte.
pub const DONE_KEY: &str = "phash_backfill_done";
/// Marcatore delle versioni precedenti (scritto all'accodamento): non vale come "fatto".
const LEGACY_QUEUED_KEY: &str = "phash_backfill_queued";
const CHUNK: usize = 500;

/// Foto con miniatura ma ancora senza impronta.
pub fn pending(store: &Store) -> AppResult<Vec<i64>> {
    Ok(store.read(|c| {
        let mut q = c.prepare(
            "SELECT id FROM media_item
             WHERE status = 'ok' AND thumb IS NOT NULL AND phash IS NULL ORDER BY id",
        )?;
        let rows = q.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
        Ok(rows)
    })?)
}

/// Accoda il job se ci sono foto da completare, il recupero non è già concluso e non c'è un
/// job `phash_backfill` in coda, in corso, in pausa o interrotto.
pub fn queue_once(store: &Store, jobs: &pc_jobs::JobManager) -> AppResult<Option<i64>> {
    let done: Option<String> = store.write(|c| {
        c.execute(
            "DELETE FROM settings_kv WHERE key = ?1",
            [LEGACY_QUEUED_KEY],
        )?;
        Ok(c.query_row(
            "SELECT value FROM settings_kv WHERE key = ?1",
            [DONE_KEY],
            |r| r.get(0),
        )
        .optional()?)
    })?;
    if done.is_some() || pending(store)?.is_empty() {
        return Ok(None);
    }
    if pc_jobs::persist::any_in_state(
        store,
        &[JOB_KIND],
        &["queued", "running", "paused", "interrupted"],
    )? {
        return Ok(None);
    }
    Ok(Some(jobs.submit(JOB_KIND, serde_json::json!({}))?))
}

fn mark_done(store: &Store) -> AppResult<()> {
    store.write(|c| {
        c.execute(
            "INSERT INTO settings_kv(key, value) VALUES (?1, '1')
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [DONE_KEY],
        )?;
        Ok(())
    })?;
    Ok(())
}

/// Scrive solo se la foto ha ancora la stessa miniatura e nessuna impronta: un import
/// concorrente (contenuto cambiato) ha già scritto la sua e non va sovrascritto.
fn write(store: &Store, rows: Vec<(i64, String, i64)>) -> AppResult<()> {
    if rows.is_empty() {
        return Ok(());
    }
    store.write(move |c| {
        let mut up = c.prepare_cached(
            "UPDATE media_item SET phash = ?3 WHERE id = ?1 AND thumb = ?2 AND phash IS NULL",
        )?;
        for (id, key, h) in &rows {
            up.execute(rusqlite::params![id, key, h])?;
        }
        Ok(())
    })?;
    Ok(())
}

/// Esegue il job: restituisce il riepilogo (annullato se `control` lo ha fermato).
pub fn run(
    store: &Store,
    thumbs: &ThumbCache,
    job_id: i64,
    control: &JobControl,
    phase: &PhaseCounters,
) -> AppResult<RunSummary> {
    let ids = pending(store)?;
    phase.set_total(ids.len() as i64);
    phase.start();
    let mut failed = 0;
    for chunk in ids.chunks(CHUNK) {
        if !control.checkpoint() {
            return Ok(RunSummary {
                cancelled: true,
                failed,
            });
        }
        let keys: Vec<(i64, Option<String>)> = {
            let chunk = chunk.to_vec();
            store.read(move |c| {
                let mut q = c.prepare_cached("SELECT thumb FROM media_item WHERE id = ?1")?;
                chunk
                    .iter()
                    .map(|id| Ok((*id, q.query_row([id], |r| r.get(0))?)))
                    .collect::<StoreResult<Vec<_>>>()
            })?
        };
        let mut done = Vec::with_capacity(keys.len());
        let mut cancelled = false;
        for (id, key) in keys {
            if !control.checkpoint() {
                cancelled = true;
                break;
            }
            let img = key
                .as_deref()
                .and_then(|k| Some((k, thumbs.path_of(k)?)))
                .ok_or("miniatura assente")
                .and_then(|(k, p)| {
                    image::open(p)
                        .map(|img| (k, img))
                        .map_err(|_| "miniatura illeggibile")
                });
            match img {
                Ok((k, img)) => done.push((id, k.to_string(), phash::to_sql(phash::phash(&img)))),
                Err(why) => {
                    // Per-file e non bloccante: resta senza impronta.
                    tracing::debug!(
                        job_id,
                        media_id = id,
                        why,
                        "impronta percettiva non calcolata"
                    );
                    failed += 1;
                    phase.inc_failed(1);
                }
            }
        }
        let n = done.len() as i64;
        write(store, done)?;
        phase.inc_done(n);
        if cancelled {
            return Ok(RunSummary {
                cancelled: true,
                failed,
            });
        }
    }
    phase.finish();
    Ok(RunSummary {
        cancelled: false,
        failed,
    })
}

pub fn plan(store: Store, thumbs: ThumbCache, _params: &serde_json::Value) -> AppResult<JobPlan> {
    Ok(JobPlan {
        phases: vec![PhaseSpec::new(PHASE, 1.0)],
        min_memory_mb: 32,
        run: Box::new(move |ctx| {
            let s = run(
                &store,
                &thumbs,
                ctx.job_id,
                &ctx.control,
                &ctx.progress.phase(PHASE),
            )?;
            // Marcatore solo a job concluso (anche con file falliti, per non riaccodare a ogni avvio).
            if !s.cancelled {
                mark_done(&store)?;
            }
            Ok(s)
        }),
    })
}
