//! Eventi verso la UI. I nomi sono stabili e condivisi con `app/ui/src/lib/ipc.ts`.

use parking_lot::Mutex;
use pc_diag::{DiagSink, ErrorRecord, LogLine};
use pc_jobs::manager::{JobSnapshot, ProgressSink};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

pub const JOB_PROGRESS: &str = "job://progress";
pub const RESOURCES_TICK: &str = "resources://tick";
/// Righe di log raggruppate (al massimo ogni `LOG_FLUSH`).
pub const LOG_LINES: &str = "log://lines";
pub const ERROR_RECORDED: &str = "log://error";

const LOG_FLUSH: Duration = Duration::from_millis(300);
/// Righe inviate per raffica: oltre, si tengono le più recenti (il file di log ha tutto).
const LOG_BATCH_MAX: usize = 200;

/// Inoltra gli snapshot dei job alla UI.
pub struct TauriProgressSink(pub AppHandle);

impl ProgressSink for TauriProgressSink {
    fn on_progress(&self, snapshot: &JobSnapshot) {
        let _ = self.0.emit(JOB_PROGRESS, snapshot);
    }
}

/// Inoltra errori (subito) e righe di log (a gruppi) alla UI: un flusso di log
/// molto intenso non deve saturare l'IPC né rallentare l'interfaccia.
pub struct TauriDiagSink {
    app: AppHandle,
    pending: Arc<Mutex<Vec<LogLine>>>,
}

impl TauriDiagSink {
    pub fn new(app: AppHandle) -> Self {
        let pending: Arc<Mutex<Vec<LogLine>>> = Arc::default();
        let (p2, a2) = (pending.clone(), app.clone());
        std::thread::Builder::new()
            .name("pc-log-events".into())
            .spawn(move || loop {
                std::thread::sleep(LOG_FLUSH);
                let mut batch = std::mem::take(&mut *p2.lock());
                if batch.is_empty() {
                    continue;
                }
                if batch.len() > LOG_BATCH_MAX {
                    batch.drain(..batch.len() - LOG_BATCH_MAX);
                }
                let _ = a2.emit(LOG_LINES, &batch);
            })
            .expect("thread eventi di log");
        Self { app, pending }
    }
}

impl DiagSink for TauriDiagSink {
    fn on_error(&self, record: &ErrorRecord) {
        let _ = self.app.emit(ERROR_RECORDED, record);
    }
    fn on_log(&self, line: &LogLine) {
        // Solo INFO e superiori: il debug resta nei file.
        if matches!(line.level.as_str(), "INFO" | "WARN" | "ERROR") {
            let mut p = self.pending.lock();
            if p.len() < LOG_BATCH_MAX * 4 {
                p.push(line.clone());
            }
        }
    }
}
