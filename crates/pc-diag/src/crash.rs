//! Panic hook: registra il panic nei log e lascia un marker su disco, così al
//! riavvio l'app può avvisare che la sessione precedente si è chiusa male.

use crate::session::{MemorySummary, SessionInfo};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const CRASH_MARKER: &str = "last_session_crashed";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct CrashInfo {
    /// Assente nel marker scritto dai gestori nativi: si usa l'ora del file.
    #[serde(default = "epoch")]
    pub ts: DateTime<Utc>,
    pub message: String,
    pub location: Option<String>,
    pub thread: Option<String>,
    #[serde(default)]
    pub kind: CrashKind,
    /// Ultima attività registrata dal marcatore di sessione.
    #[serde(default)]
    pub last_activity: Option<DateTime<Utc>>,
    /// Memoria all'ultimo aggiornamento del marcatore di sessione.
    #[serde(default)]
    pub memory: Option<MemorySummary>,
}

/// Causa della chiusura anomala.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum CrashKind {
    /// Panic Rust (messaggio e posizione nel log).
    #[default]
    Panic,
    /// Eccezione o segnale nativo (abort, accesso non valido).
    Native,
    /// Sessione non chiusa regolarmente senza altre informazioni (chiusura forzata,
    /// crash della WebView, memoria esaurita...).
    Unclean,
}

impl CrashInfo {
    pub fn new(kind: CrashKind, message: impl Into<String>) -> Self {
        Self {
            ts: Utc::now(),
            message: message.into(),
            location: None,
            thread: None,
            kind,
            last_activity: None,
            memory: None,
        }
    }

    /// Unisce il marker dei crash (panic, eccezione nativa) con il marcatore di sessione
    /// rimasto: la sessione è anomala se c'è almeno uno dei due.
    pub fn combine(crash: Option<CrashInfo>, session: Option<SessionInfo>) -> Option<CrashInfo> {
        match (crash, session) {
            (None, None) => None,
            (Some(c), None) => Some(c),
            (crash, Some(s)) => {
                let mut c = crash.unwrap_or_else(|| {
                    let mut c = CrashInfo::new(CrashKind::Unclean, "");
                    c.ts = s.last_activity;
                    c
                });
                c.last_activity = Some(s.last_activity);
                c.memory = s.memory;
                Some(c)
            }
        }
    }
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
            location,
            thread,
            ..CrashInfo::new(CrashKind::Panic, message)
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
    let written = std::fs::metadata(&marker)
        .and_then(|m| m.modified())
        .map(DateTime::<Utc>::from)
        .unwrap_or_else(|_| Utc::now());
    let _ = std::fs::remove_file(&marker);
    let mut crash: CrashInfo = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| CrashInfo::new(CrashKind::Panic, String::from_utf8_lossy(&bytes)));
    if crash.ts == epoch() {
        crash.ts = written;
    }
    Some(crash)
}

fn epoch() -> DateTime<Utc> {
    DateTime::<Utc>::UNIX_EPOCH
}

/// Scrive il marker dei crash (usato anche dai gestori nativi).
pub fn write_marker(logs_dir: &Path, crash: &CrashInfo) {
    if let Ok(json) = serde_json::to_vec_pretty(crash) {
        let _ = std::fs::write(marker_path(logs_dir), json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> SessionInfo {
        let t = Utc::now();
        SessionInfo {
            pid: 1,
            version: "1".into(),
            started: t,
            last_activity: t,
            memory: Some(MemorySummary {
                app_mb: 1,
                webview_mb: 2,
                available_mb: 3,
                total_mb: 4,
            }),
            active_jobs: vec![],
        }
    }

    #[test]
    fn session_marker_alone_is_an_unclean_session() {
        let s = session();
        let c = CrashInfo::combine(None, Some(s.clone())).unwrap();
        assert_eq!(c.kind, CrashKind::Unclean);
        assert_eq!(c.last_activity, Some(s.last_activity));
        assert_eq!(c.memory, s.memory);
        assert!(CrashInfo::combine(None, None).is_none());
    }

    #[test]
    fn panic_keeps_its_message_and_gains_session_details() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            marker_path(tmp.path()),
            r#"{"ts":"2026-11-02T08:25:53Z","message":"boom","location":"a.rs:1:1","thread":null}"#,
        )
        .unwrap();
        let panic = take_previous_crash(tmp.path()).unwrap();
        assert_eq!(
            panic.kind,
            CrashKind::Panic,
            "marker delle versioni precedenti"
        );
        let c = CrashInfo::combine(Some(panic), Some(session())).unwrap();
        assert_eq!(c.message, "boom");
        assert!(c.memory.is_some());
    }
}
