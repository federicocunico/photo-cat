//! Stato applicativo condiviso dai comandi: catalogo, diagnostica, governor e job.

use parking_lot::{Mutex, RwLock};
use pc_core::{AppPaths, AppResult, Config};
use pc_diag::{CrashInfo, Diagnostics};
use pc_governor::monitor::MonitorHandle;
use pc_governor::{MemoryGovernor, PressureMonitor, ResourceSnapshot, SystemProbe};
use pc_ingest::thumb::ThumbCache;
use pc_ingest::watch::SourceWatcher;
use pc_jobs::manager::TickerHandle;
use pc_jobs::JobManager;
use pc_store::{AppStore, Store};
use std::sync::Arc;

pub struct AppState {
    pub paths: AppPaths,
    pub config: Arc<RwLock<Config>>,
    pub face_policy: Arc<pc_ai::face_policy::FacePolicy>,
    pub diagnostics: Diagnostics,
    pub store: Store,
    /// Archivio dell'app (`app.db`): identità e set di tag, indipendente dalla libreria.
    pub app_store: AppStore,
    /// Dipendenze delle analisi AI (gazetteer per i suggerimenti di fuso).
    pub ai_deps: pc_ai::analyze::AiDeps,
    #[allow(dead_code)] // usato dai comandi di import e impostazioni (gruppi successivi)
    pub governor: MemoryGovernor,
    #[allow(dead_code)]
    pub probe: Arc<dyn SystemProbe>,
    pub monitor: Arc<PressureMonitor>,
    pub jobs: JobManager,
    pub thumbs: ThumbCache,
    pub model_store: Arc<pc_models::store::ModelStore>,
    pub model_pool: Arc<pc_models::pool::ModelPool>,
    pub session_opts: Arc<Mutex<pc_models::runtime::SessionOptions>>,
    /// Versione del runtime ONNX caricato (`None` = analisi AI non disponibili).
    pub onnx_runtime: Option<String>,
    #[allow(dead_code)] // usato dagli stadi AI dell'import (gruppo 9)
    pub assets: crate::models::AssetPaths,
    pub watcher: Mutex<Option<SourceWatcher>>,
    pub previous_crash: Mutex<Option<CrashInfo>>,
    /// Marcatore della sessione in corso (aggiornato dal monitor, cancellato all'uscita).
    pub session: Arc<pc_diag::SessionMarker>,
    /// Terminazioni della WebView in questa sessione.
    pub recovery: crate::recovery::Recovery,
    /// La finestra principale viene ricreata: la sua chiusura non è un'uscita.
    pub recreating: Mutex<bool>,
    /// Richieste di conteggi e facet della UI (scarto e interruzione di quelle superate).
    pub gates: pc_store::gate::Gates,
    pub last_resources: RwLock<Option<ResourceSnapshot>>,
    pub(crate) _monitor_handle: Mutex<Option<MonitorHandle>>,
    pub(crate) _ticker: Mutex<Option<TickerHandle>>,
}

impl AppState {
    pub fn save_config(&self, cfg: Config) -> AppResult<()> {
        cfg.save(&self.paths.config_file())?;
        self.face_policy.set_enabled(cfg.ai.faces_enabled);
        *self.config.write() = cfg;
        Ok(())
    }
}

/// Esegue `f` su un thread del pool bloccante: i comandi che leggono il catalogo non devono
/// mai occupare il thread principale, che ospita la WebView.
pub async fn blocking<T, F>(app: tauri::AppHandle, f: F) -> AppResult<T>
where
    T: Send + 'static,
    F: FnOnce(&tauri::AppHandle, &AppState) -> AppResult<T> + Send + 'static,
{
    use tauri::Manager;
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        f(&app, &state)
    })
    .await
    .map_err(|e| pc_core::AppError::internal(e.to_string()))?
}
