//! Registro errori persistente, alimentato dal layer diagnostico.

use crate::{Store, StoreResult};
use pc_diag::{DiagSink, ErrorRecord};
use rusqlite::{params, Connection, ToSql};
use serde::{Deserialize, Serialize};

pub fn insert(c: &Connection, r: &ErrorRecord) -> StoreResult<i64> {
    c.execute(
        "INSERT INTO error_log(ts, level, code, message, target, job_id, phase, path, detail)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            r.ts.to_rfc3339(),
            r.level,
            r.code,
            r.message,
            r.target,
            r.job_id,
            r.phase,
            r.path,
            r.detail
        ],
    )?;
    Ok(c.last_insert_rowid())
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ErrorQuery {
    pub level: Option<String>,
    pub phase: Option<String>,
    pub job_id: Option<i64>,
    pub code: Option<String>,
    pub text: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ErrorEntry {
    pub id: i64,
    pub ts: String,
    pub level: String,
    pub code: Option<String>,
    pub message: String,
    pub target: String,
    pub job_id: Option<i64>,
    pub phase: Option<String>,
    pub path: Option<String>,
    pub detail: Option<String>,
}

pub fn query(c: &Connection, q: &ErrorQuery) -> StoreResult<Vec<ErrorEntry>> {
    let mut sql = String::from(
        "SELECT id, ts, level, code, message, target, job_id, phase, path, detail FROM error_log WHERE 1=1",
    );
    let mut args: Vec<Box<dyn ToSql>> = Vec::new();
    if let Some(v) = &q.level {
        sql.push_str(" AND level = ?");
        args.push(Box::new(v.to_ascii_uppercase()));
    }
    if let Some(v) = &q.phase {
        sql.push_str(" AND phase = ?");
        args.push(Box::new(v.clone()));
    }
    if let Some(v) = q.job_id {
        sql.push_str(" AND job_id = ?");
        args.push(Box::new(v));
    }
    if let Some(v) = &q.code {
        sql.push_str(" AND code = ?");
        args.push(Box::new(v.clone()));
    }
    if let Some(v) = q.text.as_deref().filter(|t| !t.trim().is_empty()) {
        sql.push_str(" AND (message LIKE ? ESCAPE '\\' OR path LIKE ? ESCAPE '\\' OR detail LIKE ? ESCAPE '\\')");
        let pat = format!(
            "%{}%",
            v.replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_")
        );
        for _ in 0..3 {
            args.push(Box::new(pat.clone()));
        }
    }
    sql.push_str(" ORDER BY id DESC LIMIT ? OFFSET ?");
    args.push(Box::new(q.limit.unwrap_or(200).min(5000)));
    args.push(Box::new(q.offset.unwrap_or(0)));
    let mut st = c.prepare(&sql)?;
    let refs: Vec<&dyn ToSql> = args.iter().map(|b| b.as_ref()).collect();
    let rows = st
        .query_map(refs.as_slice(), |r| {
            Ok(ErrorEntry {
                id: r.get(0)?,
                ts: r.get(1)?,
                level: r.get(2)?,
                code: r.get(3)?,
                message: r.get(4)?,
                target: r.get(5)?,
                job_id: r.get(6)?,
                phase: r.get(7)?,
                path: r.get(8)?,
                detail: r.get(9)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

/// Percorsi dei file falliti per un job (ed eventualmente una fase): base di "Riprova".
pub fn failed_paths(c: &Connection, job_id: i64, phase: Option<&str>) -> StoreResult<Vec<String>> {
    let mut st = c.prepare(
        "SELECT DISTINCT path FROM error_log
         WHERE job_id=?1 AND level='ERROR' AND path IS NOT NULL AND (?2 IS NULL OR phase=?2) ORDER BY path",
    )?;
    let rows = st
        .query_map(params![job_id, phase], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

/// Job con almeno un file fallito (filtrabili per job e fase): punto di partenza di "Riprova".
pub fn failed_jobs(
    c: &Connection,
    job_id: Option<i64>,
    phase: Option<&str>,
) -> StoreResult<Vec<i64>> {
    let mut st = c.prepare(
        "SELECT DISTINCT job_id FROM error_log
         WHERE level='ERROR' AND job_id IS NOT NULL AND path IS NOT NULL
           AND (?1 IS NULL OR job_id=?1) AND (?2 IS NULL OR phase=?2) ORDER BY job_id",
    )?;
    let rows = st
        .query_map(params![job_id, phase], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

/// Intervallo di scrittura a lotti degli errori.
pub const ERROR_FLUSH: std::time::Duration = std::time::Duration::from_millis(500);

/// Sink diagnostico che persiste WARN/ERROR nel catalogo tramite il writer, a lotti:
/// migliaia di errori per file diventano poche scritture, ognuna in una sola transazione.
pub struct DbErrorSink {
    store: Store,
    pending: std::sync::Arc<parking_lot::Mutex<Vec<ErrorRecord>>>,
}

impl DbErrorSink {
    pub fn new(store: Store) -> Self {
        let pending: std::sync::Arc<parking_lot::Mutex<Vec<ErrorRecord>>> = Default::default();
        let weak = std::sync::Arc::downgrade(&pending);
        let s2 = store.clone();
        let _ = std::thread::Builder::new()
            .name("pc-error-log".into())
            .spawn(move || loop {
                std::thread::sleep(ERROR_FLUSH);
                let Some(p) = weak.upgrade() else {
                    return;
                };
                write_batch(&s2, &p);
            });
        Self { store, pending }
    }

    /// Accoda subito gli errori in attesa (prima di una lettura nei test o alla chiusura).
    pub fn flush(&self) {
        write_batch(&self.store, &self.pending);
    }
}

fn write_batch(store: &Store, pending: &parking_lot::Mutex<Vec<ErrorRecord>>) {
    let batch = std::mem::take(&mut *pending.lock());
    if batch.is_empty() {
        return;
    }
    let _ = store.submit(move |c| {
        let mut st = c.prepare_cached(
            "INSERT INTO error_log(ts, level, code, message, target, job_id, phase, path, detail)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )?;
        for r in &batch {
            st.execute(params![
                r.ts.to_rfc3339(),
                r.level,
                r.code,
                r.message,
                r.target,
                r.job_id,
                r.phase,
                r.path,
                r.detail
            ])?;
        }
        Ok(())
    });
}

impl DiagSink for DbErrorSink {
    fn on_error(&self, record: &ErrorRecord) {
        // Gli errori del writer stesso non tornano nel DB (eviterebbe un ciclo).
        if record.target == "pc_store::db" {
            return;
        }
        self.pending.lock().push(record.clone());
    }
}

/// Errori raggruppati per job, fase e codice (vista "per codice" della Diagnostica).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ErrorGroup {
    pub job_id: Option<i64>,
    pub phase: Option<String>,
    pub code: Option<String>,
    pub level: String,
    /// Numero di righe (errori) del gruppo.
    pub count: i64,
    /// File distinti coinvolti.
    pub files: i64,
    pub last_ts: String,
    /// Messaggio dell'errore più recente del gruppo.
    pub message: String,
}

/// Riepilogo per `(job, fase, codice, livello)`, dal gruppo più recente, filtrato come [`query`]
/// (testo escluso).
pub fn groups(c: &Connection, q: &ErrorQuery) -> StoreResult<Vec<ErrorGroup>> {
    let mut sql = String::from(
        "SELECT job_id, phase, code, level, COUNT(*), COUNT(DISTINCT path), MAX(ts), MAX(id) AS last_id
         FROM error_log WHERE 1=1",
    );
    let mut args: Vec<Box<dyn ToSql>> = Vec::new();
    if let Some(v) = &q.level {
        sql.push_str(" AND level = ?");
        args.push(Box::new(v.to_ascii_uppercase()));
    }
    if let Some(v) = &q.phase {
        sql.push_str(" AND phase = ?");
        args.push(Box::new(v.clone()));
    }
    if let Some(v) = q.job_id {
        sql.push_str(" AND job_id = ?");
        args.push(Box::new(v));
    }
    if let Some(v) = &q.code {
        sql.push_str(" AND code = ?");
        args.push(Box::new(v.clone()));
    }
    sql.push_str(" GROUP BY job_id, phase, code, level");
    let sql = format!(
        "SELECT g.*, (SELECT message FROM error_log WHERE id = g.last_id)
         FROM ({sql}) g ORDER BY g.last_id DESC LIMIT ? OFFSET ?"
    );
    args.push(Box::new(q.limit.unwrap_or(200).min(5000)));
    args.push(Box::new(q.offset.unwrap_or(0)));
    let mut st = c.prepare(&sql)?;
    let refs: Vec<&dyn ToSql> = args.iter().map(|b| b.as_ref()).collect();
    let rows = st
        .query_map(refs.as_slice(), |r| {
            Ok(ErrorGroup {
                job_id: r.get(0)?,
                phase: r.get(1)?,
                code: r.get(2)?,
                level: r.get(3)?,
                count: r.get(4)?,
                files: r.get(5)?,
                last_ts: r.get(6)?,
                message: r.get::<_, Option<String>>(8)?.unwrap_or_default(),
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StoreOptions;
    use std::sync::Arc;

    fn record(i: usize, code: &str) -> ErrorRecord {
        ErrorRecord {
            ts: chrono::Utc::now(),
            level: "ERROR".into(),
            code: Some(code.into()),
            message: format!("decodifica fallita {i}"),
            target: "pc_ingest".into(),
            job_id: Some(3),
            phase: Some("miniature".into()),
            path: Some(format!("D:\\Foto\\{i}.jpg")),
            detail: None,
        }
    }

    #[test]
    fn ten_thousand_errors_are_written_in_batches_and_grouped_by_code() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::open(&tmp.path().join("c.db"), StoreOptions::default()).unwrap();
        let sink = DbErrorSink::new(store.clone());
        for i in 0..10_000 {
            let code = if i < 9_000 {
                "DECODE_FAILED"
            } else {
                "UNSUPPORTED_FORMAT"
            };
            sink.on_error(&record(i, code));
        }
        // Il thread di scrittura svuota il buffer entro ERROR_FLUSH.
        std::thread::sleep(ERROR_FLUSH * 3);
        store.flush().unwrap();
        let n: i64 = store
            .read(|c| Ok(c.query_row("SELECT COUNT(*) FROM error_log", [], |r| r.get(0))?))
            .unwrap();
        assert_eq!(n, 10_000, "ogni errore resta registrato singolarmente");
        let g = store.read(|c| groups(c, &ErrorQuery::default())).unwrap();
        assert_eq!(g.len(), 2, "{g:?}");
        let decode = g
            .iter()
            .find(|g| g.code.as_deref() == Some("DECODE_FAILED"))
            .unwrap();
        assert_eq!((decode.count, decode.files), (9_000, 9_000));
        assert_eq!(decode.phase.as_deref(), Some("miniature"));
        let unsupported = g
            .iter()
            .find(|g| g.code.as_deref() == Some("UNSUPPORTED_FORMAT"))
            .unwrap();
        assert_eq!(unsupported.count, 1_000);
        assert_eq!(unsupported.message, "decodifica fallita 9999");
        // Elenco dei file di un gruppo, paginato.
        let page = store
            .read(|c| {
                query(
                    c,
                    &ErrorQuery {
                        job_id: Some(3),
                        code: Some("DECODE_FAILED".into()),
                        limit: Some(50),
                        offset: Some(100),
                        ..Default::default()
                    },
                )
            })
            .unwrap();
        assert_eq!(page.len(), 50);
    }

    #[test]
    fn warn_with_file_and_phase_is_persisted() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::open(&tmp.path().join("c.db"), StoreOptions::default()).unwrap();
        let (sub, diag) = pc_diag::build(tmp.path(), &pc_core::config::LoggingConfig::default());
        let sink = Arc::new(DbErrorSink::new(store.clone()));
        diag.hub().add_sink(sink.clone());
        tracing::subscriber::with_default(sub, || {
            tracing::warn!(
                code = "CORRUPT_EXIF",
                job_id = 7_i64,
                phase = "metadati",
                path = "D:\\Foto\\IMG_1.JPG",
                "EXIF illeggibile"
            );
            tracing::error!(
                code = "DECODE_FAILED",
                job_id = 7_i64,
                phase = "miniature",
                path = "D:\\Foto\\IMG_2.HEIC",
                "decodifica fallita"
            );
            tracing::info!("non persistito");
        });
        sink.flush();
        store.flush().unwrap();
        let all = store.read(|c| query(c, &ErrorQuery::default())).unwrap();
        assert_eq!(all.len(), 2);
        let meta = store
            .read(|c| {
                query(
                    c,
                    &ErrorQuery {
                        phase: Some("metadati".into()),
                        ..Default::default()
                    },
                )
            })
            .unwrap();
        assert_eq!(meta.len(), 1);
        assert_eq!(meta[0].path.as_deref(), Some("D:\\Foto\\IMG_1.JPG"));
        assert_eq!(meta[0].code.as_deref(), Some("CORRUPT_EXIF"));
        let text = store
            .read(|c| {
                query(
                    c,
                    &ErrorQuery {
                        text: Some("heic".into()),
                        ..Default::default()
                    },
                )
            })
            .unwrap();
        assert_eq!(text.len(), 1);
        let failed = store.read(|c| failed_paths(c, 7, None)).unwrap();
        assert_eq!(failed, vec!["D:\\Foto\\IMG_2.HEIC".to_string()]);
        // Solo gli ERROR contano: il job 7 ha un WARN in "metadati" e un ERROR in "miniature".
        assert_eq!(store.read(|c| failed_jobs(c, None, None)).unwrap(), vec![7]);
        assert_eq!(
            store
                .read(|c| failed_jobs(c, Some(7), Some("miniature")))
                .unwrap(),
            vec![7]
        );
        assert!(store
            .read(|c| failed_jobs(c, Some(7), Some("metadati")))
            .unwrap()
            .is_empty());
        assert!(store
            .read(|c| failed_jobs(c, Some(8), None))
            .unwrap()
            .is_empty());
    }
}
