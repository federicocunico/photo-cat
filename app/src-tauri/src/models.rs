//! Modelli AI: avvio del runtime, stato dei modelli, comandi della schermata
//! "Modelli AI e sistema".

use crate::state::AppState;
use pc_ai::models_job;
use pc_core::config::{ExecutionProvider, ResourceProfile};
use pc_core::{AppError, AppResult, ErrorCode};
use pc_governor::{Budget, SystemSnapshot};
use pc_models::runtime::{self, Provider, UnavailableProvider};
use pc_models::store::ModelStatus;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager, State};

pub const MODELS_STATUS: &str = "models://status";

/// Cartella degli asset: risorse dell'installazione, o `assets/` del repository in sviluppo.
pub fn assets_dir(app: &AppHandle) -> PathBuf {
    if cfg!(debug_assertions) {
        return PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    }
    app.path()
        .resource_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
}

/// Percorsi degli asset nell'installazione (le risorse sono rimappate in tauri.conf.json).
pub struct AssetPaths {
    pub models: PathBuf,
    pub selftest: PathBuf,
    pub runtime_lib: PathBuf,
}

pub fn asset_paths(root: &Path) -> AssetPaths {
    if cfg!(debug_assertions) {
        AssetPaths {
            models: root.join("models"),
            selftest: root.join("selftest"),
            runtime_lib: root
                .join("runtime")
                .join(pc_core::platform::asset_dir().unwrap_or("non-distribuito"))
                .join(pc_core::platform::onnxruntime_file()),
        }
    } else {
        AssetPaths {
            models: root.join("models"),
            selftest: root.join("selftest"),
            runtime_lib: root
                .join("runtime")
                .join(pc_core::platform::onnxruntime_file()),
        }
    }
}

pub fn emit_status(app: &AppHandle, statuses: &[ModelStatus]) {
    let _ = app.emit(MODELS_STATUS, statuses);
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub system: SystemSnapshot,
    pub budget: Budget,
    pub providers: Vec<Provider>,
    /// Provider GPU non utilizzabili e perché, per spiegarlo accanto alla scelta.
    pub unavailable_providers: Vec<UnavailableProvider>,
    pub onnx_runtime: Option<String>,
    pub execution_provider: ExecutionProvider,
    pub resource_profile: ResourceProfile,
}

#[tauri::command]
#[specta::specta]
pub fn system_info(state: State<'_, AppState>) -> SystemInfo {
    let cfg = state.config.read().clone();
    let (system, mut budget) = pc_governor::detect(state.probe.as_ref(), &cfg.resources);
    budget.budget_mb = state.governor.snapshot().budget_mb;
    SystemInfo {
        system,
        budget,
        providers: runtime::available_providers(),
        unavailable_providers: runtime::unavailable_providers(),
        onnx_runtime: state.onnx_runtime.clone(),
        execution_provider: cfg.ai.execution_provider,
        resource_profile: cfg.resources.profile,
    }
}

#[tauri::command]
#[specta::specta]
pub fn list_models(state: State<'_, AppState>) -> Vec<ModelStatus> {
    state.model_store.statuses()
}

/// Verifica tutti i modelli e ripristina/scarica quelli mancanti o corrotti.
#[tauri::command]
#[specta::specta]
pub fn verify_models(state: State<'_, AppState>) -> AppResult<i64> {
    state
        .jobs
        .submit(models_job::JOB_KIND, serde_json::json!({}))
}

/// Riscarica (o ripristina) un modello e ne esegue il self-test.
#[tauri::command]
#[specta::specta]
pub fn download_model(state: State<'_, AppState>, id: String) -> AppResult<i64> {
    state.jobs.submit(
        models_job::JOB_KIND,
        serde_json::json!({ "ids": [id], "selfTest": [id] }),
    )
}

#[tauri::command]
#[specta::specta]
pub fn self_test_models(state: State<'_, AppState>, ids: Vec<String>) -> AppResult<i64> {
    let sel = if ids.is_empty() {
        serde_json::json!("all")
    } else {
        serde_json::json!(ids)
    };
    state.jobs.submit(
        models_job::JOB_KIND,
        serde_json::json!({ "restore": false, "selfTest": sel }),
    )
}

/// Installa un modello da un file locale (accettato solo se lo SHA-256 corrisponde).
#[tauri::command]
#[specta::specta]
pub async fn install_model_file(
    state: State<'_, AppState>,
    id: String,
    path: String,
) -> AppResult<i64> {
    let store = state.model_store.clone();
    let model_id = id.clone();
    tauri::async_runtime::spawn_blocking(move || store.install_local(&model_id, Path::new(&path)))
        .await
        .map_err(|e| AppError::internal(e.to_string()))??;
    state.model_pool.unload(&id);
    self_test_models(state, vec![id])
}

#[tauri::command]
#[specta::specta]
pub fn set_execution_provider(
    state: State<'_, AppState>,
    provider: ExecutionProvider,
) -> AppResult<()> {
    let selected = match provider {
        ExecutionProvider::Auto => None,
        ExecutionProvider::Cpu => Some(Provider::Cpu),
        ExecutionProvider::DirectMl => Some(Provider::DirectMl),
        ExecutionProvider::Cuda => Some(Provider::Cuda),
        ExecutionProvider::CoreMl => Some(Provider::CoreMl),
    };
    if selected.is_some_and(|p| !runtime::available_providers().contains(&p)) {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "provider non disponibile in questa installazione",
        ));
    }
    let mut cfg = state.config.read().clone();
    cfg.ai.execution_provider = provider;
    state.save_config(cfg)?;
    let mut opts = state.session_opts.lock().clone();
    opts.preferred = provider;
    *state.session_opts.lock() = opts.clone();
    state.model_pool.set_options(opts);
    state.model_pool.unload_all(); // le prossime sessioni usano il nuovo provider
    tracing::info!(?provider, "provider di esecuzione aggiornato");
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn set_resource_profile(state: State<'_, AppState>, profile: ResourceProfile) -> AppResult<()> {
    let mut cfg = state.config.read().clone();
    cfg.resources.profile = profile;
    state.save_config(cfg.clone())?;
    let (_, budget) = pc_governor::detect(state.probe.as_ref(), &cfg.resources);
    let previous = state.governor.snapshot();
    state.model_pool.evict_idle("");
    state.governor.set_budget(budget.budget_mb);
    // Un cambio di profilo non deve rimuovere una limitazione per pressione in corso.
    if previous.ai_paused || previous.capacity_mb < previous.budget_mb {
        state
            .governor
            .set_capacity(previous.capacity_mb.min(budget.budget_mb));
    }
    tracing::info!(
        ?profile,
        budget_mb = budget.budget_mb,
        "profilo risorse aggiornato"
    );
    Ok(())
}

/// Rianalizza con l'AI: `ids` vuoto = tutta la libreria (solo gli elementi non aggiornati).
#[tauri::command]
#[specta::specta]
pub fn reanalyze(state: State<'_, AppState>, ids: Vec<i64>) -> AppResult<i64> {
    let params = if ids.is_empty() {
        serde_json::json!({})
    } else {
        serde_json::json!({ "ids": ids })
    };
    state.jobs.submit(pc_ai::analyze::JOB_KIND, params)
}
