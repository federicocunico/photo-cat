//! Marcatore di sessione in corso (`session.lock`).
//!
//! All'avvio si scrive il marcatore con PID, ora e versione, aggiornato periodicamente con
//! l'ora dell'ultima attività e un riepilogo della memoria; all'uscita regolare si cancella.
//! Se al riavvio il marcatore c'è ancora, la sessione precedente non si è chiusa regolarmente
//! (crash della WebView, abort nativo, chiusura forzata dal sistema...) anche se al momento
//! del crash non è stato possibile scrivere nulla.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const SESSION_LOCK: &str = "session.lock";
/// Copia del marcatore dell'ultima sessione anomala (per il pacchetto diagnostico).
pub const LAST_ABNORMAL: &str = "session.last-abnormal.json";
/// Ora dell'ultima uscita regolare: i dump di crash più recenti vanno nel pacchetto diagnostico.
pub const LAST_CLEAN_EXIT: &str = "last-clean-exit";

/// Memoria al momento dell'ultimo aggiornamento del marcatore.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct MemorySummary {
    /// Memoria residente del processo dell'app.
    pub app_mb: u64,
    /// Memoria residente dei processi dell'interfaccia (WebView).
    pub webview_mb: u64,
    /// RAM disponibile nel sistema.
    pub available_mb: u64,
    pub total_mb: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub pid: u32,
    pub version: String,
    pub started: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    #[serde(default)]
    pub memory: Option<MemorySummary>,
    /// Job attivi all'ultimo aggiornamento (id e tipo).
    #[serde(default)]
    pub active_jobs: Vec<String>,
}

/// Marcatore della sessione in corso. Va chiuso con [`SessionMarker::finish`] all'uscita regolare:
/// il `Drop` non lo cancella apposta (un panic che srotola non è un'uscita regolare).
pub struct SessionMarker {
    dir: PathBuf,
    info: parking_lot::Mutex<SessionInfo>,
}

impl SessionMarker {
    /// Legge il marcatore della sessione precedente (se c'è, la sessione non si è chiusa
    /// regolarmente: lo conserva come [`LAST_ABNORMAL`]) e scrive quello della sessione nuova.
    pub fn start(dir: &Path, version: &str) -> (Self, Option<SessionInfo>) {
        let previous = take_previous(dir);
        let now = Utc::now();
        let marker = Self {
            dir: dir.to_path_buf(),
            info: parking_lot::Mutex::new(SessionInfo {
                pid: std::process::id(),
                version: version.to_string(),
                started: now,
                last_activity: now,
                memory: None,
                active_jobs: Vec::new(),
            }),
        };
        marker.write();
        (marker, previous)
    }

    pub fn path(&self) -> PathBuf {
        self.dir.join(SESSION_LOCK)
    }

    /// Aggiorna ora dell'ultima attività, memoria e job attivi.
    pub fn touch(&self, memory: Option<MemorySummary>, active_jobs: Vec<String>) {
        {
            let mut info = self.info.lock();
            info.last_activity = Utc::now();
            if memory.is_some() {
                info.memory = memory;
            }
            info.active_jobs = active_jobs;
        }
        self.write();
    }

    pub fn info(&self) -> SessionInfo {
        self.info.lock().clone()
    }

    /// Uscita regolare: cancella il marcatore e annota l'ora.
    pub fn finish(&self) {
        let _ = std::fs::remove_file(self.path());
        let _ = std::fs::write(self.dir.join(LAST_CLEAN_EXIT), Utc::now().to_rfc3339());
    }

    fn write(&self) {
        let Ok(json) = serde_json::to_vec_pretty(&*self.info.lock()) else {
            return;
        };
        // Scrittura atomica: un crash a metà non lascia un marcatore illeggibile.
        let tmp = self.dir.join(format!("{SESSION_LOCK}.tmp"));
        if std::fs::write(&tmp, json).is_ok() {
            let _ = std::fs::rename(&tmp, self.path());
        }
    }
}

/// Marcatore lasciato da una sessione non chiusa regolarmente, se presente.
fn take_previous(dir: &Path) -> Option<SessionInfo> {
    let path = dir.join(SESSION_LOCK);
    let bytes = std::fs::read(&path).ok()?;
    let _ = std::fs::write(dir.join(LAST_ABNORMAL), &bytes);
    let _ = std::fs::remove_file(&path);
    match serde_json::from_slice::<SessionInfo>(&bytes) {
        Ok(info) => Some(info),
        // Marcatore illeggibile: la sessione resta comunque anomala, con l'ora del file.
        Err(_) => {
            let ts = std::fs::metadata(dir.join(LAST_ABNORMAL))
                .and_then(|m| m.modified())
                .map(DateTime::<Utc>::from)
                .unwrap_or_else(|_| Utc::now());
            Some(SessionInfo {
                pid: 0,
                version: String::new(),
                started: ts,
                last_activity: ts,
                memory: None,
                active_jobs: Vec::new(),
            })
        }
    }
}

/// Ora dell'ultima uscita regolare, se registrata.
pub fn last_clean_exit(dir: &Path) -> Option<DateTime<Utc>> {
    let text = std::fs::read_to_string(dir.join(LAST_CLEAN_EXIT)).ok()?;
    DateTime::parse_from_rfc3339(text.trim())
        .ok()
        .map(|t| t.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_exit_leaves_no_abnormal_session() {
        let tmp = tempfile::tempdir().unwrap();
        let (m, prev) = SessionMarker::start(tmp.path(), "1.0");
        assert!(prev.is_none());
        assert!(m.path().is_file());
        m.finish();
        assert!(!m.path().exists());
        assert!(last_clean_exit(tmp.path()).is_some());
        let (_m2, prev) = SessionMarker::start(tmp.path(), "1.0");
        assert!(prev.is_none(), "sessione precedente chiusa regolarmente");
    }

    #[test]
    fn marker_left_behind_reports_abnormal_session_with_last_activity_and_memory() {
        let tmp = tempfile::tempdir().unwrap();
        let (m, _) = SessionMarker::start(tmp.path(), "1.0");
        let mem = MemorySummary {
            app_mb: 900,
            webview_mb: 2100,
            available_mb: 925,
            total_mb: 16000,
        };
        m.touch(Some(mem), vec!["7 import".into()]);
        let last = m.info().last_activity;
        drop(m); // processo terminato senza finish()

        let (m2, prev) = SessionMarker::start(tmp.path(), "1.1");
        let prev = prev.expect("sessione anomala riconosciuta");
        assert_eq!(prev.last_activity, last);
        assert_eq!(prev.memory, Some(mem));
        assert_eq!(prev.active_jobs, vec!["7 import".to_string()]);
        assert!(tmp.path().join(LAST_ABNORMAL).is_file());
        assert_eq!(m2.info().version, "1.1", "nuovo marcatore scritto");
        assert!(m2.path().is_file());
    }

    #[test]
    fn unreadable_marker_still_counts_as_abnormal() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(SESSION_LOCK), b"{tronc").unwrap();
        let (_m, prev) = SessionMarker::start(tmp.path(), "1.0");
        assert!(prev.is_some());
    }
}
