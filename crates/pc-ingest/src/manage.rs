//! Gestione delle importazioni: rimozione completa di una sorgente (job,
//! catalogo, dati derivati), rimozione dei mancanti e pulizia delle miniature.
//! I file originali sul disco non vengono mai toccati.

use crate::thumb::ThumbCache;
use pc_core::{AppError, AppResult, ErrorCode};
use pc_jobs::JobManager;
use pc_store::model::MediaId;
use pc_store::sources::{self, RemovalSummary, Removed};
use pc_store::Store;
use std::collections::HashSet;
use std::time::Duration;

/// Tempo massimo di attesa per la fine dei job annullati prima della rimozione.
pub const CANCEL_TIMEOUT: Duration = Duration::from_secs(30);

/// Tipi di job che lavorano su una sorgente (parametro `sourceId`).
const SOURCE_JOBS: &[&str] = &[crate::import::JOB_KIND, "analysis", crate::relink::JOB_KIND];

/// Job non conclusi che riguardano la sorgente: con `sourceId` uguale, oppure
/// con `ids` che appartengono tutti alla sorgente.
pub fn source_jobs(jobs: &JobManager, store: &Store, source_id: i64) -> AppResult<Vec<i64>> {
    let active = jobs.active();
    let mut out = Vec::new();
    let mut members: Option<HashSet<MediaId>> = None;
    for (id, kind, params) in active {
        if params["sourceId"].as_i64() == Some(source_id) && SOURCE_JOBS.contains(&kind.as_str()) {
            out.push(id);
            continue;
        }
        let Some(ids) = params["ids"].as_array().filter(|a| !a.is_empty()) else {
            continue;
        };
        if members.is_none() {
            members = Some(store.read(|c| {
                Ok(pc_store::repo::media::by_source(c, source_id)?
                    .into_iter()
                    .map(|m| m.id)
                    .collect())
            })?);
        }
        let set = members.as_ref().expect("membri della sorgente");
        if ids
            .iter()
            .all(|v| v.as_i64().is_some_and(|i| set.contains(&i)))
        {
            out.push(id);
        }
    }
    Ok(out)
}

/// Riepilogo mostrato prima di rimuovere una sorgente.
pub fn removal_summary(
    jobs: &JobManager,
    store: &Store,
    source_id: i64,
) -> AppResult<RemovalSummary> {
    let mut s = store.read(|c| sources::removal_summary(c, source_id))?;
    s.jobs = source_jobs(jobs, store, source_id)?.len() as u32;
    Ok(s)
}

/// Rimuove una sorgente: annulla i suoi job e ne attende la fine, cancella in
/// una transazione catalogo e persone senza nome rimaste senza volti, poi le
/// miniature non più usate.
pub fn remove_source(
    jobs: &JobManager,
    store: &Store,
    thumbs: &ThumbCache,
    source_id: i64,
    timeout: Duration,
) -> AppResult<Removed> {
    let active = source_jobs(jobs, store, source_id)?;
    for id in &active {
        jobs.cancel(*id)?;
    }
    if !active.is_empty() {
        tracing::info!(
            source_id,
            jobs = active.len(),
            "job della sorgente annullati"
        );
    }
    if !jobs.wait_finished(&active, timeout) {
        return Err(AppError::new(
            ErrorCode::Internal,
            "i job della sorgente non si sono fermati in tempo: riprovare tra poco",
        ));
    }
    store.flush()?;
    let removed = store.write(move |c| sources::remove_source(c, source_id))?;
    let deleted = thumbs.remove(&removed.thumbs);
    tracing::info!(
        source_id,
        items = removed.items,
        thumbs = deleted,
        people = removed.people,
        "sorgente rimossa dalla libreria"
    );
    Ok(removed)
}

/// Rimuove elementi dalla libreria con i loro dati derivati.
pub fn remove_media(store: &Store, thumbs: &ThumbCache, ids: Vec<MediaId>) -> AppResult<Removed> {
    let removed = store.write(move |c| sources::remove_media(c, &ids))?;
    thumbs.remove(&removed.thumbs);
    Ok(removed)
}

/// Rimuove tutti i mancanti di una sorgente.
pub fn remove_missing(store: &Store, thumbs: &ThumbCache, source_id: i64) -> AppResult<Removed> {
    let removed = store.write(move |c| sources::remove_missing(c, source_id))?;
    thumbs.remove(&removed.thumbs);
    tracing::info!(
        source_id,
        items = removed.items,
        "mancanti rimossi dalla libreria"
    );
    Ok(removed)
}

/// Marcatore in `settings_kv`: miniature orfane già ripulite.
pub const THUMBS_PURGED: &str = "maintenance.orphan_thumbs_purged";

/// Pulizia una tantum delle miniature che nessun elemento usa (librerie
/// precedenti alla rimozione completa). Le miniature scritte dopo `started`
/// (avvio dell'app) restano: un import in corso potrebbe non averle ancora
/// registrate. `None` se la pulizia era già stata eseguita.
pub fn purge_orphan_thumbs_once(
    store: &Store,
    thumbs: &ThumbCache,
    started: std::time::SystemTime,
) -> AppResult<Option<usize>> {
    let done = store.read(|c| {
        Ok(c.query_row(
            "SELECT EXISTS(SELECT 1 FROM settings_kv WHERE key=?1)",
            [THUMBS_PURGED],
            |r| r.get::<_, bool>(0),
        )?)
    })?;
    if done {
        return Ok(None);
    }
    let used = store.read(sources::referenced_thumbs)?;
    let n = thumbs.purge_unreferenced(&used, started)?;
    store.write(|c| {
        c.execute(
            "INSERT INTO settings_kv(key,value) VALUES (?1,?2)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            rusqlite::params![THUMBS_PURGED, pc_store::now_ms().to_string()],
        )?;
        Ok(())
    })?;
    tracing::info!(count = n, "miniature orfane ripulite");
    Ok(Some(n))
}
