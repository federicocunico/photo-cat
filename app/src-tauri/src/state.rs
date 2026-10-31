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
use pc_store::Store;
use std::sync::Arc;

pub struct AppState {
    pub paths: AppPaths,
    pub config: Arc<RwLock<Config>>,
    pub face_policy: Arc<pc_ai::face_policy::FacePolicy>,
    pub diagnostics: Diagnostics,
    pub store: Store,
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
