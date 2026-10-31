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
pub fn pause_job(state: State<'_, AppState>, id: i64) -> AppResult<()> {
    state.jobs.pause(id)
}

#[tauri::command]
#[specta::specta]
pub fn resume_job(state: State<'_, AppState>, id: i64) -> AppResult<()> {
    state.jobs.resume(id)
}

#[tauri::command]
#[specta::specta]
pub fn cancel_job(state: State<'_, AppState>, id: i64) -> AppResult<()> {
    state.jobs.cancel(id)
}

#[tauri::command]
#[specta::specta]
pub fn resume_interrupted_job(state: State<'_, AppState>, id: i64) -> AppResult<()> {
    state.jobs.resume_interrupted(id)
}

/// Job dimostrativo (solo build di sviluppo) per verificare la schermata Job.
#[tauri::command]
#[specta::specta]
pub fn start_demo_job(state: State<'_, AppState>, items: u32, fail_every: u32) -> AppResult<i64> {
    if !cfg!(debug_assertions) {
        return Err(AppError::invalid_input(
            "job dimostrativo disponibile solo in sviluppo",
        ));
    }
    state.jobs.submit(
        "demo",
        serde_json::json!({ "items": items, "failEvery": fail_every }),
    )
}

#[tauri::command]
#[specta::specta]
pub fn query_errors(state: State<'_, AppState>, query: ErrorQuery) -> AppResult<Vec<ErrorEntry>> {
    Ok(state.store.read(|c| errorlog::query(c, &query))?)
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
pub fn list_job_history(state: State<'_, AppState>, limit: u32) -> AppResult<Vec<JobRow>> {
    Ok(pc_jobs::persist::recent(&state.store, limit.min(500))?)
}
