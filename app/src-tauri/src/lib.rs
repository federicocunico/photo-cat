//! Shell Tauri di photo-cat: avvio dei servizi, comandi IPC, eventi verso la UI.

mod catalogs;
mod commands;
mod demo;
#[cfg(any(debug_assertions, feature = "e2e"))]
mod dev;
mod diagnostics;
mod events;
mod library;
mod models;
mod organize;
mod people;
mod settings;
mod state;
#[cfg(windows)]
mod webview_runtime;

use parking_lot::{Mutex, RwLock};
use pc_ai::models_job::{self, ModelsDeps};
use pc_core::{AppPaths, Config};
use pc_governor::budget::recommended_workers;
use pc_governor::{MemoryGovernor, PressureMonitor, SysinfoProbe, SystemProbe, Thresholds};
use pc_ingest::import::{self, ImportDeps};
use pc_ingest::thumb::ThumbCache;
use pc_jobs::JobManager;
use pc_models::pool::ModelPool;
use pc_models::runtime::SessionOptions;
use pc_models::store::ModelStore;
use pc_models::Manifest;
use pc_store::errorlog::DbErrorSink;
use pc_store::{Store, StoreOptions};
use state::AppState;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tauri::{Emitter, Manager};
use tauri_specta::{collect_commands, Builder, ErrorHandlingMode};

/// Intervallo di aggiornamento delle risorse verso la UI.
const RESOURCES_INTERVAL: Duration = Duration::from_secs(1);

/// Builder dei binding: comandi più i tipi dei payload degli eventi.
pub fn bindings() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            commands::app_info,
            commands::dismiss_crash_notice,
            commands::get_config,
            commands::get_resources,
            commands::list_jobs,
            commands::pause_job,
            commands::resume_job,
            commands::cancel_job,
            commands::resume_interrupted_job,
            commands::list_job_history,
            commands::start_demo_job,
            commands::query_errors,
            commands::log_tail,
            commands::set_log_level,
            library::list_sources,
            library::add_source,
            library::update_source,
            library::remove_source,
            library::start_import,
            library::library_page,
            library::library_count,
            library::library_facets,
            library::library_ids,
            library::media_detail,
            library::list_catalogs,
            models::system_info,
            models::list_models,
            models::verify_models,
            models::download_model,
            models::self_test_models,
            models::install_model_file,
            models::set_execution_provider,
            models::reanalyze,
            models::set_resource_profile,
            people::list_people,
            people::face_count,
            people::person_faces,
            people::correct_person,
            people::purge_faces,
            catalogs::save_catalog,
            catalogs::delete_catalog,
            catalogs::start_bulk,
            catalogs::undo_bulk,
            catalogs::last_bulk_undo,
            organize::export_fields,
            organize::start_export,
            organize::organize_preview,
            organize::organize_preflight,
            organize::start_organize,
            organize::list_organizations,
            organize::undo_organization,
            settings::save_settings,
            settings::get_taxonomy,
            settings::save_taxonomy,
            settings::onboarding_done,
            settings::complete_onboarding,
            diagnostics::export_diagnostics,
            diagnostics::open_logs,
            diagnostics::retry_failed,
        ])
        .typ::<pc_jobs::JobSnapshot>()
        .typ::<pc_governor::ResourceSnapshot>()
        .typ::<pc_diag::LogLine>()
        .typ::<pc_diag::ErrorRecord>()
        .typ::<pc_models::store::ModelStatus>()
        .error_handling(ErrorHandlingMode::Throw)
        .dangerously_cast_bigints_to_number()
        .disable_serde_phases()
}

/// Percorso del file di binding nel frontend.
pub fn bindings_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../ui/src/bindings.ts")
}

pub fn export_bindings(builder: &Builder<tauri::Wry>, path: &std::path::Path) {
    builder
        .export(
            specta_typescript::Typescript::default()
                .header("// Generato da app/src-tauri (tauri-specta): non modificare a mano.\n/* eslint-disable */"),
            path,
        )
        .expect("export dei binding TypeScript");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let paths = AppPaths::platform_default().expect("directory dati");
    paths.ensure().expect("creazione directory dati");
    let (config, config_error) = match Config::load(&paths.config_file()) {
        Ok(c) => (c, None),
        Err(e) => (Config::default(), Some(e)),
    };
    let diagnostics = pc_diag::init(&paths.logs(), &config.logging).expect("inizializzazione log");
    if let Some(e) = config_error {
        tracing::warn!(
            code = e.code.as_str(),
            detail = e.detail.as_str(),
            "config.toml non valido: uso i valori predefiniti"
        );
    }
    let previous_crash = pc_diag::take_previous_crash(&paths.logs());
    if let Some(c) = &previous_crash {
        tracing::warn!(
            detail = c.message.as_str(),
            "la sessione precedente si è chiusa in modo anomalo"
        );
    }

    #[cfg(windows)]
    if let Some(dir) = webview_runtime::ensure() {
        tracing::warn!(
            folder = %dir.display(),
            "runtime WebView2 non trovato tramite il registro: uso la cartella installata più recente"
        );
    }

    let builder = bindings();
    #[cfg(debug_assertions)]
    export_bindings(&builder, &bindings_path());

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .register_uri_scheme_protocol("thumb", library::thumb_protocol)
        .register_asynchronous_uri_scheme_protocol("face", people::face_protocol)
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);
            let handle = app.handle().clone();

            let store = Store::open(
                &paths.database(),
                StoreOptions {
                    backup_dir: Some(paths.backups()),
                    ..Default::default()
                },
            )?;
            diagnostics
                .hub()
                .add_sink(Arc::new(DbErrorSink::new(store.clone())));
            diagnostics
                .hub()
                .add_sink(Arc::new(events::TauriDiagSink::new(handle.clone())));

            let probe: Arc<dyn SystemProbe> = Arc::new(SysinfoProbe::default());
            let (_sys, budget) = pc_governor::detect(probe.as_ref(), &config.resources);
            let governor = MemoryGovernor::new(budget.budget_mb);
            let monitor = Arc::new(PressureMonitor::new(
                probe.clone(),
                governor.clone(),
                Thresholds::default(),
            ));

            // Modelli AI: runtime ONNX, modelli installati (ripristinati dagli asset o scaricati), pool.
            let assets = models::asset_paths(&models::assets_dir(&handle));
            #[cfg(all(feature = "heif", windows))]
            pc_ingest::heif::set_library_path(assets.runtime_dll.with_file_name("heif.dll"));
            // Il caricamento avviene su un thread a parte: se la libreria non risponde (dipendenze
            // di sistema anomale) l'app si apre comunque, con l'analisi AI disattivata.
            let runtime_load = {
                let dll = assets.runtime_dll.clone();
                let (tx, rx) = std::sync::mpsc::channel();
                std::thread::spawn(move || {
                    let _ = tx.send(pc_models::runtime::init(&dll));
                });
                rx.recv_timeout(std::time::Duration::from_secs(30))
                    .unwrap_or_else(|_| {
                        Err(pc_core::AppError::new(
                            pc_core::ErrorCode::ModelUnavailable,
                            "ONNX Runtime non ha risposto entro 30 s",
                        ))
                    })
            };
            let onnx_runtime = match runtime_load {
                Ok(v) => Some(v),
                Err(e) => {
                    tracing::error!(
                        code = e.code.as_str(),
                        detail = e.detail.as_str(),
                        "ONNX Runtime non disponibile: analisi AI disattivate"
                    );
                    None
                }
            };
            let model_store = Arc::new(ModelStore::new(
                Manifest::builtin(),
                paths.models(),
                Some(assets.models.clone()),
            ));
            model_store.recover_gpu_trials();
            let sink_handle = handle.clone();
            model_store.set_sink(Arc::new(move |s| models::emit_status(&sink_handle, s)));
            let session_opts = Arc::new(Mutex::new(SessionOptions {
                preferred: config.ai.execution_provider,
                intra_threads: (_sys.physical_cores / 2).max(1),
                force_gpu_failure: false,
            }));
            let model_pool = Arc::new(ModelPool::new(
                model_store.clone(),
                governor.clone(),
                probe.clone(),
                session_opts.lock().clone(),
            ));
            let shared_config = Arc::new(RwLock::new(config.clone()));
            let face_policy =
                Arc::new(pc_ai::face_policy::FacePolicy::new(config.ai.faces_enabled));
            let ai_deps = pc_ai::analyze::AiDeps {
                resources: pc_ai::stages::AiResources {
                    models: model_store.clone(),
                    pool: model_pool.clone(),
                    governor: governor.clone(),
                },
                store: store.clone(),
                gazetteer: Arc::default(),
                config: shared_config.clone(),
                face_policy: face_policy.clone(),
                analysis_lock: Arc::default(),
            };

            let jobs = JobManager::new(
                store.clone(),
                governor.clone(),
                2,
                Arc::new(events::TauriProgressSink(handle.clone())),
            );
            jobs.register("demo", Arc::new(demo::plan));
            let export_store = store.clone();
            jobs.register(
                pc_organize::export::JOB_KIND,
                Arc::new(move |p| pc_organize::export::plan(export_store.clone(), p)),
            );
            let organize_store = store.clone();
            jobs.register(
                pc_organize::organize::JOB_KIND,
                Arc::new(move |p| pc_organize::organize::plan(organize_store.clone(), p)),
            );
            let bulk_store = store.clone();
            jobs.register(
                pc_jobs::bulk::JOB_KIND,
                Arc::new(move |p| pc_jobs::bulk::plan(bulk_store.clone(), p)),
            );
            let analysis_deps = ai_deps.clone();
            jobs.register(
                pc_ai::analyze::JOB_KIND,
                Arc::new(move |p| pc_ai::analyze::plan(analysis_deps.clone(), p)),
            );
            let models_deps = ModelsDeps {
                store: model_store.clone(),
                images: assets.selftest.clone(),
                opts: session_opts.clone(),
            };
            jobs.register(
                models_job::JOB_KIND,
                Arc::new(move |p| models_job::plan(models_deps.clone(), p)),
            );
            let thumbs = ThumbCache::new(paths.thumbnails());
            let cores = _sys.logical_cores;
            let import_deps = ImportDeps {
                store: store.clone(),
                thumbs: thumbs.clone(),
                governor: governor.clone(),
                extra_stages: Vec::new(),
                stage_factory: Some(pc_ai::analyze::import_factory(ai_deps)),
                io_workers: recommended_workers(budget.budget_mb, 32, cores).min(4),
                decode_workers: recommended_workers(budget.budget_mb, 160, cores),
            };
            jobs.register(
                import::JOB_KIND,
                Arc::new(move |p| import::plan(import_deps.clone(), p)),
            );
            match jobs.recover_interrupted() {
                Ok(rows) if !rows.is_empty() => {
                    tracing::warn!(
                        count = rows.len(),
                        "job interrotti disponibili per la ripresa"
                    )
                }
                Err(e) => tracing::error!(
                    code = e.code.as_str(),
                    detail = e.detail.as_str(),
                    "lettura job interrotti"
                ),
                _ => {}
            }
            let ticker = jobs.start_ticker();
            // Verifica dei modelli in background: la libreria resta utilizzabile.
            if config.ai.auto_download_models {
                let _ = jobs.submit(models_job::JOB_KIND, serde_json::json!({}));
            } else {
                let st = model_store.clone();
                std::thread::spawn(move || {
                    st.check_all();
                });
            }
            #[cfg(any(debug_assertions, feature = "e2e"))]
            dev::on_startup(&store, &jobs);

            app.manage(AppState {
                paths: paths.clone(),
                config: shared_config,
                face_policy,
                diagnostics,
                store,
                governor,
                probe,
                monitor: monitor.clone(),
                jobs,
                thumbs,
                watcher: Mutex::new(None),
                model_store,
                model_pool,
                session_opts,
                onnx_runtime,
                assets,
                previous_crash: Mutex::new(previous_crash.clone()),
                last_resources: RwLock::new(None),
                _monitor_handle: Mutex::new(None),
                _ticker: Mutex::new(Some(ticker)),
            });

            let emit_handle = handle.clone();
            let monitor_handle = monitor.spawn(RESOURCES_INTERVAL, move |snap| {
                if let Some(state) = emit_handle.try_state::<AppState>() {
                    *state.last_resources.write() = Some(snap.clone());
                }
                let _ = emit_handle.emit(events::RESOURCES_TICK, &snap);
            });
            *app.state::<AppState>()._monitor_handle.lock() = Some(monitor_handle);
            library::restart_watcher(&handle);
            tracing::info!("photo-cat avviato");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("errore durante l'avvio di photo-cat");
}
