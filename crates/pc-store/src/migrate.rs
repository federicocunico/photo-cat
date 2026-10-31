//! Migrazioni versionate tramite `PRAGMA user_version`, con backup del DB
//! esistente prima di ogni aggiornamento di schema.

use crate::StoreError;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

/// Migrazioni in ordine: l'indice + 1 è la versione raggiunta.
const MIGRATIONS: &[&str] = &[
    include_str!("schema_v1.sql"),
    include_str!("schema_v2.sql"),
    include_str!("schema_v3.sql"),
    include_str!("schema_v4.sql"),
];

pub const SCHEMA_VERSION: i64 = MIGRATIONS.len() as i64;

pub fn current_version(conn: &Connection) -> Result<i64, StoreError> {
    Ok(conn.query_row("PRAGMA user_version", [], |r| r.get(0))?)
}

/// Porta il DB all'ultima versione. Se il DB contiene già dati di una versione
/// precedente, ne crea prima una copia in `backup_dir`. Restituisce il percorso
/// del backup, se creato.
pub fn migrate(
    conn: &mut Connection,
    backup_dir: Option<&Path>,
) -> Result<Option<PathBuf>, StoreError> {
    let from = current_version(conn)?;
    if from > SCHEMA_VERSION {
        return Err(StoreError::Schema(format!(
            "il catalogo è alla versione {from}, più recente di quella supportata ({SCHEMA_VERSION})"
        )));
    }
    if from == SCHEMA_VERSION {
        return Ok(None);
    }
    let backup = match (from > 0, backup_dir) {
        (true, Some(dir)) => Some(backup(conn, dir, from)?),
        _ => None,
    };
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(from as usize) {
        let version = i as i64 + 1;
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", version)?;
        tx.commit()?;
        tracing::info!(version, "schema del catalogo migrato");
    }
    Ok(backup)
}

fn backup(conn: &Connection, dir: &Path, version: i64) -> Result<PathBuf, StoreError> {
    std::fs::create_dir_all(dir).map_err(|e| StoreError::Schema(e.to_string()))?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let path = dir.join(format!("catalog-v{version}-{stamp}.db"));
    conn.execute("VACUUM INTO ?1", [path.to_string_lossy()])?;
    tracing::info!(path = %path.display(), "backup del catalogo prima della migrazione");
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXPECTED_TABLES: &[&str] = &[
        "source",
        "media_item",
        "media_tech",
        "media_dates",
        "exif",
        "location",
        "label",
        "embedding",
        "media_people",
        "person",
        "face",
        "tag",
        "media_tag",
        "catalog",
        "catalog_item",
        "job",
        "job_phase",
        "item_stage",
        "error_log",
        "organize_op",
        "organize_entry",
        "undo_log",
        "settings_kv",
        "media_fts",
    ];

    #[test]
    fn empty_db_migrates_to_latest() {
        let mut conn = Connection::open_in_memory().unwrap();
        assert_eq!(migrate(&mut conn, None).unwrap(), None);
        assert_eq!(current_version(&conn).unwrap(), SCHEMA_VERSION);
        for t in EXPECTED_TABLES {
            let n: i64 = conn
                .query_row(
                    "SELECT count(*) FROM sqlite_master WHERE name = ?1",
                    [t],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(n, 1, "tabella {t} mancante");
        }
        // Idempotente.
        migrate(&mut conn, None).unwrap();
    }

    #[test]
    fn newer_schema_is_refused() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "user_version", SCHEMA_VERSION + 1)
            .unwrap();
        assert!(matches!(
            migrate(&mut conn, None),
            Err(StoreError::Schema(_))
        ));
    }

    #[test]
    fn backup_is_written_for_existing_older_db() {
        let tmp = tempfile::tempdir().unwrap();
        let mut conn = Connection::open(tmp.path().join("c.db")).unwrap();
        // Simula un DB "v0" con dati ma senza versione: forziamo la versione a 0
        // dopo aver creato una tabella fittizia, poi fingiamo versione 1 < target.
        conn.execute_batch("CREATE TABLE legacy(x); INSERT INTO legacy VALUES (1);")
            .unwrap();
        let path = backup(&conn, &tmp.path().join("bk"), 0).unwrap();
        assert!(path.exists());
        let copy = Connection::open(&path).unwrap();
        let n: i64 = copy
            .query_row("SELECT count(*) FROM legacy", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
        migrate(&mut conn, Some(&tmp.path().join("bk"))).unwrap();
    }
}
