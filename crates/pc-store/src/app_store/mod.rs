//! Archivio dell'app `app.db`: identità e set di tag, validi per ogni libreria.
//!
//! Il traffico è basso (modifiche dall'interfaccia, letture all'avvio dei job),
//! quindi basta una sola connessione protetta da un mutex. Le migrazioni usano
//! lo stesso meccanismo `PRAGMA user_version` del catalogo.

use crate::{StoreError, StoreResult};
use parking_lot::Mutex;
use rusqlite::{Connection, OptionalExtension, Transaction};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

/// Migrazioni di `app.db`: l'indice + 1 è la versione raggiunta.
pub mod identities;
pub mod tags;
pub mod transfer;

const MIGRATIONS: &[&str] = &[include_str!("schema_v1.sql")];

pub const APP_SCHEMA_VERSION: i64 = MIGRATIONS.len() as i64;

#[derive(Clone)]
pub struct AppStore {
    conn: Arc<Mutex<Connection>>,
    path: Option<PathBuf>,
}

impl AppStore {
    pub fn open(path: &Path) -> StoreResult<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| StoreError::Schema(e.to_string()))?;
        }
        let mut conn = Connection::open(path)?;
        configure(&conn)?;
        migrate(&mut conn, path.parent())?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            path: Some(path.to_path_buf()),
        })
    }

    /// Archivio in memoria (test).
    pub fn open_in_memory() -> StoreResult<Self> {
        let mut conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        migrate(&mut conn, None)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            path: None,
        })
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Lettura o scrittura semplice con la connessione condivisa.
    pub fn with<T>(&self, f: impl FnOnce(&Connection) -> StoreResult<T>) -> StoreResult<T> {
        f(&self.conn.lock())
    }

    /// Scrittura in una transazione: un errore annulla tutto.
    pub fn tx<T>(&self, f: impl FnOnce(&Transaction) -> StoreResult<T>) -> StoreResult<T> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let out = f(&tx)?;
        tx.commit()?;
        Ok(out)
    }

    pub fn kv_get(&self, key: &str) -> StoreResult<Option<String>> {
        self.with(|c| kv_get(c, key))
    }

    pub fn kv_set(&self, key: &str, value: &str) -> StoreResult<()> {
        self.with(|c| kv_set(c, key, value))
    }

    /// Riepilogo per il bundle diagnostico: versione e conteggi, senza nomi,
    /// ritagli o embedding.
    pub fn diagnostic_summary(&self) -> StoreResult<serde_json::Value> {
        self.with(|c| {
            let count = |table: &str| -> StoreResult<i64> {
                Ok(c.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))?)
            };
            Ok(serde_json::json!({
                "schema_version": current_version(c)?,
                "identities": count("identity")?,
                "reference_faces": count("identity_face")?,
                "tag_sets": count("tag_set")?,
                "tag_set_items": count("tag_set_item")?,
                "tag_styles": count("tag_style")?,
            }))
        })
    }
}

pub fn kv_get(conn: &Connection, key: &str) -> StoreResult<Option<String>> {
    Ok(conn
        .query_row("SELECT value FROM app_kv WHERE key = ?1", [key], |r| {
            r.get(0)
        })
        .optional()?)
}

pub fn kv_set(conn: &Connection, key: &str, value: &str) -> StoreResult<()> {
    conn.execute(
        "INSERT INTO app_kv (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [key, value],
    )?;
    Ok(())
}

fn configure(conn: &Connection) -> StoreResult<()> {
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.busy_timeout(Duration::from_secs(10))?;
    Ok(())
}

pub fn current_version(conn: &Connection) -> StoreResult<i64> {
    Ok(conn.query_row("PRAGMA user_version", [], |r| r.get(0))?)
}

fn migrate(conn: &mut Connection, backup_dir: Option<&Path>) -> StoreResult<()> {
    let from = current_version(conn)?;
    if from > APP_SCHEMA_VERSION {
        return Err(StoreError::Schema(format!(
            "l'archivio dell'app è alla versione {from}, più recente di quella supportata ({APP_SCHEMA_VERSION})"
        )));
    }
    if from == APP_SCHEMA_VERSION {
        return Ok(());
    }
    if let (true, Some(dir)) = (from > 0, backup_dir) {
        let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        let path = dir.join(format!("app-v{from}-{stamp}.db"));
        conn.execute("VACUUM INTO ?1", [path.to_string_lossy()])?;
    }
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(from as usize) {
        let version = i as i64 + 1;
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", version)?;
        tx.commit()?;
        tracing::info!(version, "schema dell'archivio dell'app migrato");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLES: &[&str] = &[
        "identity",
        "identity_alias",
        "identity_face",
        "tag_set",
        "tag_set_item",
        "tag_style",
        "app_kv",
    ];

    fn base_sets(store: &AppStore) -> i64 {
        store
            .with(|c| {
                Ok(c.query_row(
                    "SELECT count(*) FROM tag_set WHERE is_base = 1 AND name = 'Base'",
                    [],
                    |r| r.get(0),
                )?)
            })
            .unwrap()
    }

    #[test]
    fn empty_app_db_gets_schema_and_base_set_once() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("app").join("app.db");
        let store = AppStore::open(&path).unwrap();
        store
            .with(|c| {
                assert_eq!(current_version(c)?, APP_SCHEMA_VERSION);
                for t in TABLES {
                    let n: i64 = c.query_row(
                        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                        [t],
                        |r| r.get(0),
                    )?;
                    assert_eq!(n, 1, "tabella {t} mancante");
                }
                Ok(())
            })
            .unwrap();
        assert_eq!(base_sets(&store), 1);
        drop(store);

        let again = AppStore::open(&path).unwrap();
        assert_eq!(base_sets(&again), 1);
        assert_eq!(
            again.kv_get("active_tag_set").unwrap().as_deref(),
            Some("1")
        );
    }

    #[test]
    fn newer_app_db_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("app.db");
        {
            let c = Connection::open(&path).unwrap();
            c.pragma_update(None, "user_version", APP_SCHEMA_VERSION + 1)
                .unwrap();
        }
        assert!(matches!(AppStore::open(&path), Err(StoreError::Schema(_))));
    }

    #[test]
    fn diagnostic_summary_has_counts_only() {
        let store = AppStore::open_in_memory().unwrap();
        store
            .with(|c| {
                c.execute(
                    "INSERT INTO identity (uid, name, short_name, created_at) VALUES ('u1', 'Giulia Bianchi', 'Giulia', 0)",
                    [],
                )?;
                c.execute(
                    "INSERT INTO identity_face (uid, crop, model_ver, embedding, source, created_at)
                     VALUES ('u1', x'DEADBEEF', 'sface-1', x'01020304', 'file', 0)",
                    [],
                )?;
                Ok(())
            })
            .unwrap();
        let s = store.diagnostic_summary().unwrap();
        assert_eq!(s["identities"], 1);
        assert_eq!(s["reference_faces"], 1);
        assert_eq!(s["tag_sets"], 1);
        let text = s.to_string();
        assert!(!text.contains("Giulia"));
        assert!(!text.to_lowercase().contains("deadbeef"));
    }
}
