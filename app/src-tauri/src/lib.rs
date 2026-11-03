//! Shell Tauri di photo-cat: avvio dei servizi, comandi IPC, eventi verso la UI.

mod boot;
mod catalogs;
mod commands;
mod demo;
#[cfg(any(debug_assertions, feature = "e2e"))]
mod dev;
mod diagnostics;
mod entries;
mod events;
mod identities;
mod library;
mod models;
mod open_with;
mod organize;
mod people;
mod places;
mod recovery;
mod sequence;
mod settings;
mod similar;
mod state;
mod tags;
mod timeline;
mod ui_prefs;
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

use state::AppState;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tauri::{Emitter, Manager};
use tauri_specta::{collect_commands, Builder, ErrorHandlingMode};

/// Intervallo di aggiornamento delle risorse verso la UI.
const RESOURCES_INTERVAL: Duration = Duration::from_secs(1);
/// Ogni quanti tick delle risorse si aggiorna il marcatore di sessione (30 s).
const SESSION_TOUCH_TICKS: u64 = 30;

/// Job attivi come "id tipo" (marcatore di sessione e log dei crash).
pub(crate) fn active_job_labels(jobs: &JobManager) -> Vec<String> {
    jobs.snapshots()
        .into_iter()
        .filter(|j| {
            matches!(
                j.state,
                pc_jobs::JobState::Running | pc_jobs::JobState::Paused | pc_jobs::JobState::Queued
            )
        })
        .map(|j| format!("{} {}", j.id, j.kind))
        .collect()
}

fn memory_summary(s: &pc_governor::SystemSnapshot) -> pc_diag::MemorySummary {
    pc_diag::MemorySummary {
        app_mb: s.process_mb,
        webview_mb: s.webview_mb,
        available_mb: s.available_mb,
        total_mb: s.total_mb,
    }
}

/// Builder dei binding: comandi più i tipi dei payload degli eventi.
pub fn bindings() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            boot::boot_status,
            boot::boot_retry,
            boot::boot_choose_folder,
            commands::app_info,
            commands::dismiss_crash_notice,
            commands::take_ui_recovery,
            commands::report_long_tasks,
            commands::dev_native_crash,
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
            commands::error_groups,
            commands::log_tail,
            commands::set_log_level,
            library::list_sources,
            library::add_source,
            library::update_source,
            library::remove_source,
            library::start_import,
            library::source_removal_summary,
            library::relink_missing,
            library::remove_missing,
            library::move_source,
            library::missing_folder,
            library::import_updates,
            library::last_relink,
            library::library_page,
            library::library_count,
            library::library_grid_count,
            library::library_facets,
            library::library_facet_values,
            library::library_selection,
            library::list_folders,
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
            people::people_sections,
            people::list_groups,
            people::get_group,
            people::group_faces,
            people::locate_face,
            people::face_count,
            people::person_faces,
            people::correct_person,
            people::undo_people,
            people::purge_faces,
            catalogs::save_catalog,
            catalogs::delete_catalog,
            catalogs::collect_duplicates,
            catalogs::start_photo_quality,
            catalogs::start_bulk,
            catalogs::trash_impact,
            catalogs::undo_bulk,
            catalogs::last_bulk_undo,
            tags::tag_overview,
            tags::rename_tag,
            tags::merge_tag,
            tags::delete_tag,
            tags::set_tag_color,
            tags::save_tag_set,
            tags::duplicate_tag_set,
            tags::delete_tag_set,
            tags::reorder_tag_sets,
            tags::add_tag_to_set,
            tags::remove_tag_from_set,
            tags::set_active_tag_set,
            tags::set_tag_set_export_keywords,
            tags::set_tag_set_exclusive,
            tags::tag_set_conflicts,
            identities::list_identities,
            identities::save_identity,
            identities::delete_identity,
            identities::merge_identities,
            identities::identity_faces,
            identities::identity_face_image,
            identities::remove_identity_face,
            identities::add_identity_reference_file,
            identities::add_identity_reference_face,
            identities::link_person_identity,
            identities::suggest_identities,
            identities::export_identities,
            identities::preview_identity_import,
            identities::import_identities,
            timeline::sequence_overview,
            timeline::sequence_page,
            timeline::sequence_session_ids,
            timeline::sequence_ids,
            timeline::set_session_gap,
            timeline::set_group_display,
            timeline::set_group_shift,
            timeline::preview_group_clock,
            timeline::propose_alignment,
            timeline::place_undated,
            timeline::ignore_tz_suggestion,
            timeline::list_timezones,
            sequence::sequence_export_defaults,
            sequence::sequence_export_preview,
            sequence::sequence_export_preflight,
            sequence::start_sequence_export,
            similar::similar_group,
            similar::similar_remove,
            similar::similar_group_together,
            similar::similar_dissolve,
            similar::similar_set_cover,
            similar::similar_group_members,
            similar::similar_non_covers,
            similar::similar_preview,
            similar::similar_pending,
            similar::start_similar_analysis,
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
            entries::add_category,
            entries::category_usage,
            entries::delete_category,
            settings::preview_thresholds,
            settings::reanalyze_below_floor,
            places::suggest_places,
            places::rename_place,
            places::recompute_places,
            open_with::open_with,
            settings::onboarding_done,
            ui_prefs::ui_prefs_load,
            ui_prefs::ui_prefs_save,
            settings::complete_onboarding,
            diagnostics::export_diagnostics,
            diagnostics::open_logs,
            diagnostics::retry_failed,
        ])
        .typ::<pc_jobs::JobSnapshot>()
        .typ::<pc_governor::ResourceSnapshot>()
        .typ::<pc_diag::LogLine>()
        .typ::<pc_diag::ErrorRecord>()
        .typ::<pc_diag::ErrorBatch>()
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
    let paths = AppPaths::platform_default();
    let primary_logs = paths.as_ref().ok().map(AppPaths::logs);
    let fallback_logs = std::env::temp_dir().join("photo-cat-boot-logs");
    let logs = primary_logs
        .clone()
        .filter(|p| std::fs::create_dir_all(p).is_ok())
        .unwrap_or(fallback_logs);
    let logging = paths
        .as_ref()
        .ok()
        .and_then(|p| Config::load(&p.config_file()).ok())
        .map(|c| c.logging)
        .unwrap_or_default();
    let diagnostics = Arc::new(pc_diag::init(&logs, &logging).expect("inizializzazione log"));
    pc_diag::native::install(&logs);
    let (session, previous_session) =
        pc_diag::SessionMarker::start(&logs, env!("CARGO_PKG_VERSION"));
    let session = Arc::new(session);
    let previous_crash =
        pc_diag::CrashInfo::combine(pc_diag::take_previous_crash(&logs), previous_session);
    if let Some(c) = &previous_crash {
        let mem = c.memory.unwrap_or_default();
        tracing::warn!(
            kind = ?c.kind,
            detail = c.message.as_str(),
            last_activity = ?c.last_activity,
            app_mb = mem.app_mb,
            webview_mb = mem.webview_mb,
            available_mb = mem.available_mb,
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

    let setup_session = session.clone();
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .register_asynchronous_uri_scheme_protocol("thumb", library::thumb_protocol)
        .register_asynchronous_uri_scheme_protocol("face", people::face_protocol)
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);
            app.manage(boot::Boot::new(
                paths,
                diagnostics,
                logs,
                previous_crash,
                setup_session,
            ));
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
            }
            boot::start(app.handle().clone());
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("errore durante l'avvio di photo-cat");
    app.run(move |app, event| match event {
        // La finestra viene ricreata dopo la terminazione della WebView: non è un'uscita.
        tauri::RunEvent::ExitRequested { api, .. }
            if app
                .try_state::<AppState>()
                .is_some_and(|s| *s.recreating.lock()) =>
        {
            api.prevent_exit();
        }
        tauri::RunEvent::Exit => {
            session.finish();
            tracing::info!("photo-cat chiuso regolarmente");
        }
        _ => {}
    });
}

fn initialize(
    handle: &tauri::AppHandle,
    paths: AppPaths,
    boot: &boot::Boot,
) -> pc_core::AppResult<()> {
    let started = std::time::SystemTime::now();

    let (store, app_store) = boot::open_stores(&paths, |phase| boot.phase(handle, phase))?;
    let config = Config::load(&paths.config_file()).unwrap_or_else(|e| {
        tracing::warn!(detail = %e, "config.toml non valido: uso i valori predefiniti");
        Config::default()
    });
    let diagnostics = boot.diagnostics.clone();
    diagnostics.set_level(config.logging.level)?;
    let previous_crash = &boot.previous_crash;
    let session = boot.session.clone();
    tracing::info!(path = %paths.app_store().display(), "archivio dell'app aperto");
    identities::on_library_open(&store, &app_store);
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
    boot.phase(handle, "runtime");
    let assets = models::asset_paths(&models::assets_dir(handle));
    #[cfg(all(feature = "heif", windows))]
    pc_ingest::heif::set_library_path(assets.runtime_lib.with_file_name("heif.dll"));
    // Il caricamento avviene su un thread a parte: se la libreria non risponde (dipendenze
    // di sistema anomale) l'app si apre comunque, con l'analisi AI disattivata.
    let runtime_load = {
        let lib = assets.runtime_lib.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(pc_models::runtime::init(&lib));
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
    boot.phase(handle, "services");
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
    let face_policy = Arc::new(pc_ai::face_policy::FacePolicy::new(config.ai.faces_enabled));
    let thumbs = ThumbCache::new(paths.thumbnails());
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
        app_store: Some(app_store.clone()),
        thumbs: Some(thumbs.clone()),
    };

    let jobs = JobManager::new(
        store.clone(),
        governor.clone(),
        2,
        Arc::new(events::TauriProgressSink::new(handle.clone())),
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
    let backfill_store = store.clone();
    jobs.register(
        pc_ingest::backfill::JOB_KIND,
        Arc::new(move |p| pc_ingest::backfill::plan(backfill_store.clone(), p)),
    );
    let fq_store = store.clone();
    let fq_gov = governor.clone();
    let fq_crops = paths.face_crops();
    jobs.register(
        pc_ai::face_quality::JOB_KIND,
        Arc::new(move |p| {
            pc_ai::face_quality::plan(fq_store.clone(), fq_gov.clone(), Some(fq_crops.clone()), p)
        }),
    );
    let pq_store = store.clone();
    let pq_thumbs = pc_ingest::thumb::ThumbCache::new(paths.thumbnails());
    jobs.register(
        pc_ai::photo_quality::JOB_KIND,
        Arc::new(move |p| pc_ai::photo_quality::plan(pq_store.clone(), pq_thumbs.clone(), p)),
    );
    let seq_store = store.clone();
    let seq_governor = governor.clone();
    jobs.register(
        pc_organize::sequence::job::JOB_KIND,
        Arc::new(move |p| {
            pc_organize::sequence::job::plan(seq_store.clone(), seq_governor.clone(), p)
        }),
    );
    let bulk_store = store.clone();
    let bulk_deps = ai_deps.clone();
    let bulk_thumbs = thumbs.clone();
    jobs.register(
        pc_jobs::bulk::JOB_KIND,
        Arc::new(move |p| {
            // Il gazetteer si carica solo se serve ("Ripristina luogo automatico").
            let deps = bulk_deps.clone();
            let resolver = std::sync::OnceLock::new();
            let r: pc_jobs::bulk::PlaceResolver = Arc::new(move |i| {
                let f = resolver.get_or_init(|| {
                    Box::new(deps.place_resolver())
                        as Box<
                            dyn Fn(
                                    &pc_store::bulk::PlaceInputs,
                                )
                                    -> Option<pc_store::model::Location>
                                + Send
                                + Sync,
                        >
                });
                f(i)
            });
            let cache = bulk_thumbs.clone();
            let remover: pc_jobs::bulk::ThumbRemover = Arc::new(move |keys| {
                cache.remove(keys);
            });
            pc_jobs::bulk::plan_full(bulk_store.clone(), Some(r), Some(remover), p)
        }),
    );
    let similar_deps = ai_deps.clone();
    jobs.register(
        pc_ai::similar::JOB_KIND,
        Arc::new(move |p| pc_ai::similar::plan(similar_deps.clone(), p)),
    );
    let retune_deps = ai_deps.clone();
    jobs.register(
        pc_ai::retune::JOB_KIND,
        Arc::new(move |p| pc_ai::retune::plan(retune_deps.clone(), p)),
    );
    let state_ai_deps = ai_deps.clone();
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
    let relink_store = store.clone();
    jobs.register(
        pc_ingest::relink::JOB_KIND,
        Arc::new(move |p| pc_ingest::relink::plan(relink_store.clone(), p)),
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
    // Librerie importate prima dello schema v5: fuso, dispositivo e istante reale.
    if let Err(e) = pc_ingest::backfill::queue_once(&store, &jobs) {
        tracing::error!(
            code = e.code.as_str(),
            detail = e.detail.as_str(),
            "accodamento time_backfill"
        );
    }
    // Librerie con volti analizzati prima dello schema v8: nitidezza e dimensione dei volti.
    if let Err(e) = pc_ai::face_quality::queue_once(&store, &jobs) {
        tracing::error!(
            code = e.code.as_str(),
            detail = e.detail.as_str(),
            "accodamento face_quality"
        );
    }
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

    handle.manage(AppState {
        paths: paths.clone(),
        config: shared_config,
        face_policy,
        diagnostics,
        store,
        app_store,
        ai_deps: state_ai_deps,
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
        session,
        recovery: recovery::Recovery::default(),
        recreating: Mutex::new(false),
        gates: Default::default(),
        last_resources: RwLock::new(None),
        _monitor_handle: Mutex::new(None),
        _ticker: Mutex::new(Some(ticker)),
    });

    let emit_handle = handle.clone();
    let ticks = std::sync::atomic::AtomicU64::new(0);
    let monitor_handle = monitor.spawn(RESOURCES_INTERVAL, move |snap| {
        if let Some(state) = emit_handle.try_state::<AppState>() {
            *state.last_resources.write() = Some(snap.clone());
            let n = ticks.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if n.is_multiple_of(SESSION_TOUCH_TICKS) {
                state.session.touch(
                    Some(memory_summary(&snap.system)),
                    active_job_labels(&state.jobs),
                );
            }
        }
        if let Some(low) = snap.ui_trim {
            // Memoria della WebView oltre la quota (o rientrata).
            recovery::set_low_memory(&emit_handle, low);
            if low {
                let _ = emit_handle.emit(recovery::TRIM_CACHES, ());
            }
        }
        let _ = emit_handle.emit(events::RESOURCES_TICK, &snap);
    });
    *handle.state::<AppState>()._monitor_handle.lock() = Some(monitor_handle);
    recovery::watch(handle);
    library::purge_orphan_thumbs(handle, started);
    library::restart_watcher(handle);
    tracing::info!("photo-cat avviato");
    Ok(())
}
