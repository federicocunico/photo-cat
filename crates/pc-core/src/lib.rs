//! Tipi condivisi da tutti i crate di photo-cat: errori con codici stabili,
//! directory dei dati e configurazione.

pub mod config;
pub mod error;
pub mod paths;
pub mod platform;
pub mod text;

pub use config::{AiThresholds, Config, ExternalApp};
pub use error::{AppError, AppResult, ErrorCode};
pub use paths::AppPaths;
