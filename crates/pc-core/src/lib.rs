//! Tipi condivisi da tutti i crate di photo-cat: errori con codici stabili,
//! directory dei dati e configurazione.

pub mod config;
pub mod error;
pub mod paths;

pub use config::Config;
pub use error::{AppError, AppResult, ErrorCode};
pub use paths::AppPaths;
