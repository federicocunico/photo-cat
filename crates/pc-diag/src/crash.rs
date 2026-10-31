//! Panic hook: registra il panic nei log e lascia un marker su disco, così al
//! riavvio l'app può avvisare che la sessione precedente si è chiusa male.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const CRASH_MARKER: &str = "last_session_crashed";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct CrashInfo {
    pub ts: DateTime<Utc>,
    pub message: String,
    pub location: Option<String>,
    pub thread: Option<String>,
}

pub fn marker_path(logs_dir: &Path) -> PathBuf {
    logs_dir.join(CRASH_MARKER)
}

/// Installa il panic hook (mantenendo quello precedente in coda).
pub fn install_panic_hook(logs_dir: &Path) {
    let marker = marker_path(logs_dir);
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let message = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "panic senza messaggio".to_string()
        };
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()));
        let thread = std::thread::current().name().map(str::to_string);
        tracing::error!(
            code = "INTERNAL",
            detail = location.as_deref().unwrap_or("?"),
            thread = thread.as_deref().unwrap_or("?"),
            "panic: {message}"
        );
        let crash = CrashInfo {
            ts: Utc::now(),
            message,
            location,
            thread,
        };
        if let Ok(json) = serde_json::to_vec_pretty(&crash) {
            let _ = std::fs::write(&marker, json);
        }
        previous(info);
    }));
}

/// Legge e rimuove il marker della sessione precedente, se presente.
pub fn take_previous_crash(logs_dir: &Path) -> Option<CrashInfo> {
    let marker = marker_path(logs_dir);
    let bytes = std::fs::read(&marker).ok()?;
    let _ = std::fs::remove_file(&marker);
    Some(
        serde_json::from_slice(&bytes).unwrap_or_else(|_| CrashInfo {
            ts: Utc::now(),
            message: String::from_utf8_lossy(&bytes).into_owned(),
            location: None,
            thread: None,
        }),
    )
}
