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

/// Sink diagnostico che persiste WARN/ERROR nel catalogo tramite il writer.
pub struct DbErrorSink {
    store: Store,
}

impl DbErrorSink {
    pub fn new(store: Store) -> Self {
        Self { store }
    }
}

impl DiagSink for DbErrorSink {
    fn on_error(&self, record: &ErrorRecord) {
        // Gli errori del writer stesso non tornano nel DB (eviterebbe un ciclo).
        if record.target == "pc_store::db" {
            return;
        }
        let rec = record.clone();
        let _ = self.store.submit(move |c| insert(c, &rec).map(|_| ()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StoreOptions;
    use std::sync::Arc;

    #[test]
    fn warn_with_file_and_phase_is_persisted() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::open(&tmp.path().join("c.db"), StoreOptions::default()).unwrap();
        let (sub, diag) = pc_diag::build(tmp.path(), &pc_core::config::LoggingConfig::default());
        diag.hub()
            .add_sink(Arc::new(DbErrorSink::new(store.clone())));
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
