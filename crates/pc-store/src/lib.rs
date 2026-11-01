//! Persistenza del catalogo di photo-cat su SQLite.

pub mod analysis;
pub mod app_store;
pub mod bulk;
pub mod db;
pub mod detail;
pub mod errorlog;
pub mod migrate;
pub mod model;
pub mod people;
pub mod query;
pub mod repo;
pub mod rules;
pub mod sequence;
pub mod sources;
pub mod timeline;

pub use app_store::AppStore;
pub use db::{Store, StoreOptions};

use pc_core::{AppError, ErrorCode};

#[derive(Debug, Clone, thiserror::Error)]
pub enum StoreError {
    #[error("errore SQLite: {0}")]
    Sqlite(String),
    #[error("schema del catalogo: {0}")]
    Schema(String),
    #[error("dato non valido: {0}")]
    Invalid(String),
    #[error("conflitto: {0}")]
    Conflict(String),
    #[error("elemento non trovato: {0}")]
    NotFound(String),
    #[error("commit del batch fallito")]
    CommitFailed,
    #[error("catalogo chiuso")]
    Closed,
}

impl From<rusqlite::Error> for StoreError {
    fn from(e: rusqlite::Error) -> Self {
        match e {
            rusqlite::Error::QueryReturnedNoRows => StoreError::NotFound("nessuna riga".into()),
            other => StoreError::Sqlite(other.to_string()),
        }
    }
}

impl From<serde_json::Error> for StoreError {
    fn from(e: serde_json::Error) -> Self {
        StoreError::Invalid(e.to_string())
    }
}

impl From<StoreError> for AppError {
    fn from(e: StoreError) -> Self {
        let code = match &e {
            StoreError::Invalid(_) | StoreError::Conflict(_) => ErrorCode::InvalidInput,
            StoreError::NotFound(_) => ErrorCode::NotFound,
            _ => ErrorCode::Database,
        };
        AppError::new(code, e.to_string())
    }
}

pub type StoreResult<T> = Result<T, StoreError>;

/// Millisecondi dall'epoca Unix (timestamp interni).
pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}
