//! Errore applicativo unico verso UI, log e registro errori.
//!
//! Ogni crate definisce i propri errori con `thiserror` e li converte in
//! [`AppError`], che porta un codice stabile, una chiave di messaggio
//! localizzabile (risolta dalla UI) e il dettaglio tecnico.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Codici errore stabili: compaiono nei log, nel registro errori e nell'export
/// diagnostico, quindi non vanno rinominati.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum ErrorCode {
    Internal,
    Io,
    NotFound,
    AccessDenied,
    InvalidConfig,
    Database,
    UnsupportedFormat,
    DecodeFailed,
    CorruptExif,
    Network,
    ChecksumMismatch,
    ModelUnavailable,
    SelfTestFailed,
    InsufficientMemory,
    Cancelled,
    InvalidInput,
    LinkUnsupported,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Internal => "INTERNAL",
            Self::Io => "IO",
            Self::NotFound => "NOT_FOUND",
            Self::AccessDenied => "ACCESS_DENIED",
            Self::InvalidConfig => "INVALID_CONFIG",
            Self::Database => "DATABASE",
            Self::UnsupportedFormat => "UNSUPPORTED_FORMAT",
            Self::DecodeFailed => "DECODE_FAILED",
            Self::CorruptExif => "CORRUPT_EXIF",
            Self::Network => "NETWORK",
            Self::ChecksumMismatch => "CHECKSUM_MISMATCH",
            Self::ModelUnavailable => "MODEL_UNAVAILABLE",
            Self::SelfTestFailed => "SELF_TEST_FAILED",
            Self::InsufficientMemory => "INSUFFICIENT_MEMORY",
            Self::Cancelled => "CANCELLED",
            Self::InvalidInput => "INVALID_INPUT",
            Self::LinkUnsupported => "LINK_UNSUPPORTED",
        }
    }

    /// Chiave i18n del messaggio per l'utente (`errors.<codice>`).
    pub fn message_key(self) -> String {
        format!("errors.{}", self.as_str().to_ascii_lowercase())
    }

    /// Se ha senso riprovare automaticamente o tramite "Riprova".
    pub fn default_retriable(self) -> bool {
        matches!(
            self,
            Self::Io | Self::Network | Self::Database | Self::InsufficientMemory
        )
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[error("[{code}] {detail}")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct AppError {
    pub code: ErrorCode,
    pub message_key: String,
    pub detail: String,
    pub retriable: bool,
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn new(code: ErrorCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            message_key: code.message_key(),
            detail: detail.into(),
            retriable: code.default_retriable(),
        }
    }

    pub fn with_retriable(mut self, retriable: bool) -> Self {
        self.retriable = retriable;
        self
    }

    pub fn internal(detail: impl Into<String>) -> Self {
        Self::new(ErrorCode::Internal, detail)
    }

    pub fn invalid_input(detail: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidInput, detail)
    }

    pub fn cancelled() -> Self {
        Self::new(ErrorCode::Cancelled, "operazione annullata")
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        use std::io::ErrorKind;
        let code = match e.kind() {
            ErrorKind::NotFound => ErrorCode::NotFound,
            ErrorKind::PermissionDenied => ErrorCode::AccessDenied,
            ErrorKind::OutOfMemory => ErrorCode::InsufficientMemory,
            _ => ErrorCode::Io,
        };
        Self::new(code, e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        Self::new(ErrorCode::InvalidInput, format!("JSON non valido: {e}"))
    }
}

impl From<toml::de::Error> for AppError {
    fn from(e: toml::de::Error) -> Self {
        Self::new(ErrorCode::InvalidConfig, e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn io_errors_map_to_specific_codes() {
        let e: AppError = io::Error::new(io::ErrorKind::PermissionDenied, "x").into();
        assert_eq!(e.code, ErrorCode::AccessDenied);
        assert_eq!(e.message_key, "errors.access_denied");
        assert!(!e.retriable);

        let e: AppError = io::Error::new(io::ErrorKind::NotFound, "x").into();
        assert_eq!(e.code, ErrorCode::NotFound);

        let e: AppError = io::Error::new(io::ErrorKind::TimedOut, "x").into();
        assert_eq!(e.code, ErrorCode::Io);
        assert!(e.retriable);
    }

    #[test]
    fn json_and_toml_errors_convert() {
        let e: AppError = serde_json::from_str::<u32>("nope").unwrap_err().into();
        assert_eq!(e.code, ErrorCode::InvalidInput);
        let e: AppError = toml::from_str::<toml::Value>("= =").unwrap_err().into();
        assert_eq!(e.code, ErrorCode::InvalidConfig);
    }

    #[test]
    fn crate_error_converts_through_from() {
        #[derive(Debug, thiserror::Error)]
        enum IngestError {
            #[error("decodifica fallita: {0}")]
            Decode(String),
        }
        impl From<IngestError> for AppError {
            fn from(e: IngestError) -> Self {
                AppError::new(ErrorCode::DecodeFailed, e.to_string())
            }
        }
        let e: AppError = IngestError::Decode("heic".into()).into();
        assert_eq!(e.code, ErrorCode::DecodeFailed);
        assert_eq!(e.to_string(), "[DECODE_FAILED] decodifica fallita: heic");
    }

    #[test]
    fn codes_serialize_as_stable_strings() {
        let s = serde_json::to_string(&ErrorCode::ChecksumMismatch).unwrap();
        assert_eq!(s, "\"CHECKSUM_MISMATCH\"");
        assert_eq!(ErrorCode::ChecksumMismatch.as_str(), "CHECKSUM_MISMATCH");
    }
}
