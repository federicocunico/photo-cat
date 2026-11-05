//! Job `place_geo` (change `photo-map`): coordinate approssimate dei luoghi da cartella o
//! manuali, cercate per nome nel gazetteer offline. Senza gazetteer non fa nulla. Una
//! posizione `manual` non viene mai sovrascritta.

use crate::analyze::AiDeps;
use crate::geo::Gazetteer;
use pc_core::AppResult;
use pc_jobs::{JobControl, JobPlan, PhaseCounters, PhaseSpec, RunSummary};
use pc_store::{geo_map, Store};
use rusqlite::OptionalExtension;

pub const JOB_KIND: &str = "place_geo";
pub const PHASE: &str = "luoghi";
/// Marcatore in `settings_kv`: primo passaggio concluso con il gazetteer disponibile.
pub const DONE_KEY: &str = "place_geo_done";
const CHUNK: usize = 200;

/// Geocodifica i nomi senza riga; restituisce quanti ne ha collocati.
pub fn run_with(
    store: &Store,
    gaz: Option<&Gazetteer>,
    control: &JobControl,
    phase: &PhaseCounters,
) -> AppResult<(usize, bool)> {
    let Some(gaz) = gaz else {
        return Ok((0, false));
    };
    let names = store.read(geo_map::names_to_geocode)?;
    phase.set_total(names.len() as i64);
    phase.start();
    let mut placed = 0;
    for chunk in names.chunks(CHUNK) {
        if !control.checkpoint() {
            return Ok((placed, true));
        }
        let found: Vec<(String, f64, f64, String)> = chunk
            .iter()
            .filter_map(|n| {
                gaz.by_name(n)
                    .map(|c| (n.clone(), c.lat, c.lon, c.country.clone()))
            })
            .collect();
        placed += store.write(move |c| {
            let mut n = 0;
            for (name, lat, lon, country) in &found {
                let cc = Some(country.as_str()).filter(|s| !s.is_empty());
                if geo_map::place_geo_insert_auto(c, name, *lat, *lon, cc)? {
                    n += 1;
                }
            }
            Ok(n)
        })?;
        phase.inc_done(chunk.len() as i64);
    }
    phase.finish();
    Ok((placed, false))
}

/// Passaggio sincrono (senza avanzamento), per i punti in cui un job sarebbe eccessivo.
pub fn run_inline(deps: &AiDeps) -> AppResult<usize> {
    // Dopo ogni job bulk/import: senza nomi nuovi non si carica nemmeno il gazetteer.
    if deps.store.read(geo_map::names_to_geocode)?.is_empty() {
        return Ok(0);
    }
    let gaz = deps.gazetteer();
    let ctl = JobControl::new();
    let phase = pc_jobs::JobProgress::new(&[PhaseSpec::new(PHASE, 1.0)]).phase(PHASE);
    Ok(run_with(&deps.store, gaz.as_deref(), &ctl, &phase)?.0)
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

/// Accoda il job una volta per libreria (marcatore) se ci sono nomi da collocare e non c'è
/// già un `place_geo` attivo.
pub fn queue_once(store: &Store, jobs: &pc_jobs::JobManager) -> AppResult<Option<i64>> {
    let done: Option<String> = store.read(|c| {
        Ok(c.query_row(
            "SELECT value FROM settings_kv WHERE key = ?1",
            [DONE_KEY],
            |r| r.get(0),
        )
        .optional()?)
    })?;
    if done.is_some() || store.read(geo_map::names_to_geocode)?.is_empty() {
        return Ok(None);
    }
    queue(store, jobs)
}

/// Accoda il job (dopo una rinomina dei luoghi) se non ce n'è già uno in coda o in corso.
pub fn queue(store: &Store, jobs: &pc_jobs::JobManager) -> AppResult<Option<i64>> {
    if pc_jobs::persist::any_in_state(store, &[JOB_KIND], &["queued", "running", "paused"])? {
        return Ok(None);
    }
    Ok(Some(jobs.submit(JOB_KIND, serde_json::json!({}))?))
}

pub fn plan(deps: AiDeps, _params: &serde_json::Value) -> AppResult<JobPlan> {
    Ok(JobPlan {
        phases: vec![PhaseSpec::new(PHASE, 1.0)],
        min_memory_mb: 32,
        run: Box::new(move |ctx| {
            let gaz = deps.gazetteer();
            let phase = ctx.progress.phase(PHASE);
            let (_, cancelled) = run_with(&deps.store, gaz.as_deref(), &ctx.control, &phase)?;
            // Marcatore solo con gazetteer: se si installa dopo, il passaggio riparte.
            if !cancelled && gaz.is_some() {
                mark_done(&deps.store)?;
            }
            Ok(RunSummary {
                cancelled,
                failed: 0,
            })
        }),
    })
}

/// Avvolge un job (import): a lavoro concluso colloca i nuovi luoghi in modo sincrono
/// (costo trascurabile; lo slot del followup è già usato da `similar`/`analysis`).
pub fn with_place_geo_after(deps: AiDeps, plan: JobPlan) -> JobPlan {
    let inner = plan.run;
    JobPlan {
        run: Box::new(move |ctx| {
            let summary = inner(ctx)?;
            if !summary.cancelled {
                if let Err(e) = run_inline(&deps) {
                    tracing::warn!(
                        code = e.code.as_str(),
                        detail = e.detail.as_str(),
                        "place_geo dopo l'import"
                    );
                }
            }
            Ok(summary)
        }),
        ..plan
    }
}
