//! Comandi IPC esposti alla UI. I tipi TypeScript sono generati in
//! `app/ui/src/bindings.ts` (vedi `bindings::builder`).

use crate::state::AppState;
use pc_core::config::LogLevel;
use pc_core::{AppError, AppResult, Config};
use pc_diag::{CrashInfo, LogLine};
use pc_governor::ResourceSnapshot;
use pc_jobs::manager::JobSnapshot;
use pc_jobs::persist::JobRow;
use pc_store::errorlog::{self, ErrorEntry, ErrorQuery};
use serde::Serialize;
use tauri::State;

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub data_dir: String,
    pub logs_dir: String,
    pub previous_crash: Option<CrashInfo>,
}

#[tauri::command]
#[specta::specta]
pub fn app_info(state: State<'_, AppState>) -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION").into(),
        data_dir: state.paths.root().display().to_string(),
        logs_dir: state.paths.logs().display().to_string(),
        previous_crash: state.previous_crash.lock().clone(),
    }
}

/// Segna come letto l'avviso di chiusura anomala.
#[tauri::command]
#[specta::specta]
pub fn dismiss_crash_notice(state: State<'_, AppState>) {
    state.previous_crash.lock().take();
}

/// Terminazione dell'interfaccia da segnalare dopo la ricarica (una volta sola).
#[tauri::command]
#[specta::specta]
pub fn take_ui_recovery(state: State<'_, AppState>) -> Option<crate::recovery::UiRecovery> {
    state.recovery.take_pending()
}

/// Long task dell'interfaccia (≥ 50 ms) degli ultimi secondi, registrati in modalità
/// diagnostica (livello debug) per il test di carico.
#[tauri::command]
#[specta::specta]
pub fn report_long_tasks(count: u32, max_ms: f64, total_ms: f64) {
    tracing::debug!(
        count,
        max_ms = max_ms.round() as u64,
        total_ms = total_ms.round() as u64,
        "long task dell'interfaccia"
    );
}

/// Solo sviluppo: provoca un abort nativo per verificare log, minidump e avviso al riavvio.
#[tauri::command]
#[specta::specta]
pub fn dev_native_crash() -> AppResult<()> {
    #[cfg(debug_assertions)]
    {
        tracing::warn!("abort nativo provocato dal comando di sviluppo");
        pc_diag::native::crash_for_test();
    }
    #[cfg(not(debug_assertions))]
    Err(AppError::invalid_input("disponibile solo in sviluppo"))
}

#[tauri::command]
#[specta::specta]
pub fn get_config(state: State<'_, AppState>) -> Config {
    state.config.read().clone()
}

#[tauri::command]
#[specta::specta]
pub fn get_resources(state: State<'_, AppState>) -> ResourceSnapshot {
    if let Some(s) = state.last_resources.read().clone() {
        return s;
    }
    state.monitor.tick()
}

#[tauri::command]
#[specta::specta]
pub fn list_jobs(state: State<'_, AppState>) -> Vec<JobSnapshot> {
    let mut jobs = state.jobs.snapshots();
    jobs.sort_by_key(|j| std::cmp::Reverse(j.id));
    jobs
}

#[tauri::command]
#[specta::specta]
pub async fn pause_job(app: tauri::AppHandle, id: i64) -> AppResult<()> {
    crate::state::blocking(app, move |_app, state| state.jobs.pause(id)).await
}

#[tauri::command]
#[specta::specta]
pub async fn resume_job(app: tauri::AppHandle, id: i64) -> AppResult<()> {
    crate::state::blocking(app, move |_app, state| state.jobs.resume(id)).await
}

#[tauri::command]
#[specta::specta]
pub async fn cancel_job(app: tauri::AppHandle, id: i64) -> AppResult<()> {
    crate::state::blocking(app, move |_app, state| state.jobs.cancel(id)).await
}

#[tauri::command]
#[specta::specta]
pub async fn resume_interrupted_job(app: tauri::AppHandle, id: i64) -> AppResult<()> {
    crate::state::blocking(app, move |_app, state| state.jobs.resume_interrupted(id)).await
}

/// Job dimostrativo (solo build di sviluppo) per verificare la schermata Job.
#[tauri::command]
#[specta::specta]
pub async fn start_demo_job(app: tauri::AppHandle, items: u32, fail_every: u32) -> AppResult<i64> {
    crate::state::blocking(app, move |_app, state| {
        if !cfg!(debug_assertions) {
            return Err(AppError::invalid_input(
                "job dimostrativo disponibile solo in sviluppo",
            ));
        }
        state.jobs.submit(
            "demo",
            serde_json::json!({ "items": items, "failEvery": fail_every }),
        )
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn query_errors(app: tauri::AppHandle, query: ErrorQuery) -> AppResult<Vec<ErrorEntry>> {
    crate::state::blocking(app, move |_app, state| {
        Ok(state.store.read(|c| errorlog::query(c, &query))?)
    })
    .await
}

/// Errori raggruppati per job, fase e codice (vista "per codice" della Diagnostica).
#[tauri::command]
#[specta::specta]
pub async fn error_groups(
    state: State<'_, AppState>,
    query: ErrorQuery,
) -> AppResult<Vec<errorlog::ErrorGroup>> {
    let db = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || Ok(db.read(|c| errorlog::groups(c, &query))?))
        .await
        .map_err(|e| AppError::internal(e.to_string()))?
}

#[tauri::command]
#[specta::specta]
pub fn log_tail(state: State<'_, AppState>) -> Vec<LogLine> {
    state.diagnostics.hub().tail()
}

#[tauri::command]
#[specta::specta]
pub fn set_log_level(state: State<'_, AppState>, level: LogLevel) -> AppResult<()> {
    state.diagnostics.set_level(level)?;
    let mut cfg = state.config.read().clone();
    cfg.logging.level = level;
    state.save_config(cfg)?;
    tracing::info!(level = level.as_str(), "livello di log aggiornato");
    Ok(())
}

/// Storico dei job dal catalogo (inclusi quelli interrotti nella sessione precedente).
#[tauri::command]
#[specta::specta]
pub async fn list_job_history(app: tauri::AppHandle, limit: u32) -> AppResult<Vec<JobRow>> {
    crate::state::blocking(app, move |_app, state| {
        Ok(pc_jobs::persist::recent(&state.store, limit.min(500))?)
    })
    .await
}
