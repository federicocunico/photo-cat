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

/// Informazioni sull'app installata (app-about): versione, build, sistema, autori e licenza.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    /// Commit abbreviato della build; `None` per una build senza git (archivio dei sorgenti).
    pub commit: Option<String>,
    /// Data della build, "AAAA-MM-GG".
    pub build_date: String,
    /// Configurazione della build ("release", "debug").
    pub profile: String,
    /// Sistema operativo con la versione, ad esempio "Windows 11 Pro".
    pub os: String,
    pub arch: String,
    /// Versione del motore della finestra (WebView2, WebKitGTK, WKWebView), se nota.
    pub webview: Option<String>,
    /// Acceleratore dell'analisi AI secondo la preferenza e i provider disponibili.
    pub accelerator: String,
    pub authors: String,
    pub repository: Option<String>,
    pub license: String,
    pub data_dir: String,
    pub logs_dir: String,
    pub previous_crash: Option<CrashInfo>,
}

#[tauri::command]
#[specta::specta]
pub fn app_info(state: State<'_, AppState>) -> AppInfo {
    let preferred = state.config.read().ai.execution_provider;
    AppInfo {
        data_dir: state.paths.root().display().to_string(),
        logs_dir: state.paths.logs().display().to_string(),
        previous_crash: state.previous_crash.lock().clone(),
        ..about(
            preferred,
            &pc_models::runtime::available_providers(),
            tauri::webview_version().ok(),
        )
    }
}

/// Parte di [`AppInfo`] che non dipende dallo stato dell'app.
pub fn about(
    preferred: pc_core::config::ExecutionProvider,
    available: &[pc_models::runtime::Provider],
    webview: Option<String>,
) -> AppInfo {
    let non_empty = |s: &str| (!s.is_empty()).then(|| s.to_string());
    AppInfo {
        version: env!("CARGO_PKG_VERSION").into(),
        commit: non_empty(env!("PC_BUILD_COMMIT")),
        build_date: env!("PC_BUILD_DATE").into(),
        profile: env!("PC_BUILD_PROFILE").into(),
        os: sysinfo::System::long_os_version().unwrap_or_else(|| std::env::consts::OS.to_string()),
        arch: std::env::consts::ARCH.into(),
        webview,
        accelerator: accelerator(preferred, available).into(),
        authors: env!("CARGO_PKG_AUTHORS").replace(':', ", "),
        repository: non_empty(env!("CARGO_PKG_REPOSITORY")),
        license: env!("CARGO_PKG_LICENSE").into(),
        data_dir: String::new(),
        logs_dir: String::new(),
        previous_crash: None,
    }
}

/// Nome dell'acceleratore che l'analisi usa, con la stessa scelta di `runtime::load`.
fn accelerator(
    preferred: pc_core::config::ExecutionProvider,
    available: &[pc_models::runtime::Provider],
) -> &'static str {
    use pc_core::config::ExecutionProvider as Pref;
    use pc_models::runtime::Provider as P;
    let gpu = |p: P| available.contains(&p).then_some(p);
    let chosen = match preferred {
        Pref::Cpu => None,
        Pref::Auto => available.iter().copied().find(|p| *p != P::Cpu),
        Pref::DirectMl => gpu(P::DirectMl),
        Pref::Cuda => gpu(P::Cuda),
        Pref::CoreMl => gpu(P::CoreMl),
    };
    match chosen.unwrap_or(P::Cpu) {
        P::Cpu => "CPU",
        P::DirectMl => "DirectML",
        P::Cuda => "CUDA",
        P::CoreMl => "Core ML",
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

/// Esito di un'archiviazione/ripristino: voci toccate e nuovo valore dell'indicatore globale.
#[derive(Debug, Clone, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveOutcome {
    pub affected: u32,
    /// Errori (ERROR) non archiviati: sostituisce `errorCount` nello store live.
    pub active_count: i64,
}

/// Archivia voci singole, un gruppo o tutto il filtro corrente. Non cancella nulla.
#[tauri::command]
#[specta::specta]
pub async fn archive_errors(
    state: State<'_, AppState>,
    target: errorlog::ArchiveTarget,
) -> AppResult<ArchiveOutcome> {
    let db = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.write(move |c| {
            let affected = errorlog::archive_errors(c, &target)? as u32;
            let active_count = errorlog::active_error_count(c)?;
            Ok(ArchiveOutcome {
                affected,
                active_count,
            })
        })
        .map_err(Into::into)
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))?
}

/// Ripristina voci archiviate.
#[tauri::command]
#[specta::specta]
pub async fn unarchive_errors(
    state: State<'_, AppState>,
    ids: Vec<i64>,
) -> AppResult<ArchiveOutcome> {
    let db = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.write(move |c| {
            let affected = errorlog::unarchive_errors(c, &ids)? as u32;
            let active_count = errorlog::active_error_count(c)?;
            Ok(ArchiveOutcome {
                affected,
                active_count,
            })
        })
        .map_err(Into::into)
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))?
}

/// Conteggio iniziale dell'indicatore globale: errori non archiviati.
#[tauri::command]
#[specta::specta]
pub async fn error_count(state: State<'_, AppState>) -> AppResult<i64> {
    let db = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || Ok(db.read(errorlog::active_error_count)?))
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

#[cfg(test)]
mod about_tests {
    use super::*;
    use pc_core::config::ExecutionProvider as Pref;
    use pc_models::runtime::Provider as P;

    #[test]
    fn informazioni_su_versione_build_e_licenza() {
        let info = about(Pref::Cpu, &[P::Cpu], Some("131.0".into()));
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
        // Commit abbreviato, oppure nessuno per una build senza git: mai una stringa vuota.
        assert_ne!(info.commit.as_deref(), Some(""));
        let date = info.build_date.as_bytes();
        assert_eq!(date.len(), 10, "data {}", info.build_date);
        assert!(date.iter().enumerate().all(|(i, c)| if i == 4 || i == 7 {
            *c == b'-'
        } else {
            c.is_ascii_digit()
        }));
        assert!(["debug", "release"].contains(&info.profile.as_str()));
        assert_eq!(info.license, "MIT OR Apache-2.0");
        assert_eq!(info.authors, "photo-cat contributors");
        assert_eq!(
            info.repository.as_deref(),
            Some("https://github.com/federicocunico/photo-cat")
        );
        assert_eq!(info.webview.as_deref(), Some("131.0"));
        assert!(!info.os.is_empty() && !info.arch.is_empty());
    }

    #[test]
    fn acceleratore_secondo_preferenza_e_disponibilita() {
        assert_eq!(
            about(Pref::Cpu, &[P::Cpu, P::DirectMl], None).accelerator,
            "CPU"
        );
        assert_eq!(
            about(Pref::Auto, &[P::Cpu, P::DirectMl], None).accelerator,
            "DirectML"
        );
        assert_eq!(about(Pref::Auto, &[P::Cpu], None).accelerator, "CPU");
        // Provider scelto ma non disponibile: l'analisi usa la CPU.
        assert_eq!(
            about(Pref::Cuda, &[P::Cpu, P::DirectMl], None).accelerator,
            "CPU"
        );
    }
}
