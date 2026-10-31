use crate::state::AppState;
use pc_ai::taxonomy::{self, Taxonomy};
use pc_core::{AppError, AppResult, Config};
use tauri::State;
#[tauri::command]
#[specta::specta]
pub async fn save_settings(state: State<'_, AppState>, config: Config) -> AppResult<()> {
    config.validate()?;
    if let Some(path) = &config.data_dir {
        pc_organize::paths::writable_destination(&state.store, path)?;
    }
    let previous = state.config.read().clone();
    // Le soglie fanno parte della versione dei risultati, così i file diventano rianalizzabili.
    if previous.ai.confidence_threshold != config.ai.confidence_threshold {
        let db = state.store.clone();
        let threshold = config.ai.confidence_threshold;
        tauri::async_runtime::spawn_blocking(move || {
            let mut tax = taxonomy::load(&db)?;
            let version = tax.version;
            tax.version += 1;
            for g in &mut tax.groups {
                g.threshold = threshold;
            }
            taxonomy::save(&db, tax, version)
        })
        .await
        .map_err(|e| AppError::internal(e.to_string()))??;
    }
    state.save_config(config.clone())?;
    state.diagnostics.set_level(config.logging.level)?;
    if previous.ai.execution_provider != config.ai.execution_provider {
        let mut options = state.session_opts.lock().clone();
        options.preferred = config.ai.execution_provider;
        state.model_pool.set_options(options.clone());
        *state.session_opts.lock() = options;
    }
    if previous.resources != config.resources {
        state.model_pool.evict_idle("");
        pc_governor::apply_resources(&state.governor, state.probe.as_ref(), &config.resources);
    }
    Ok(())
}
#[tauri::command]
#[specta::specta]
pub async fn get_taxonomy(state: State<'_, AppState>) -> AppResult<Taxonomy> {
    let db = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || taxonomy::load(&db))
        .await
        .map_err(|e| AppError::internal(e.to_string()))?
}
#[tauri::command]
#[specta::specta]
pub async fn save_taxonomy(
    state: State<'_, AppState>,
    mut taxonomy: Taxonomy,
    expected_version: i64,
) -> AppResult<()> {
    taxonomy.version = expected_version
        .checked_add(1)
        .ok_or_else(|| AppError::invalid_input("versione tassonomia esaurita"))?;
    let db = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || taxonomy::save(&db, taxonomy, expected_version))
        .await
        .map_err(|e| AppError::internal(e.to_string()))?
}
#[tauri::command]
#[specta::specta]
pub fn onboarding_done(state: State<'_, AppState>) -> AppResult<bool> {
    Ok(state.store.read(|c| {
        Ok(c.query_row(
            "SELECT EXISTS(SELECT 1 FROM settings_kv WHERE key='onboarding.done' AND value='true')",
            [],
            |r| r.get(0),
        )?)
    })?)
}
#[tauri::command]
#[specta::specta]
pub fn complete_onboarding(state: State<'_, AppState>) -> AppResult<()> {
    state.store.write(|c|{c.execute("INSERT INTO settings_kv(key,value) VALUES ('onboarding.done','true') ON CONFLICT(key) DO UPDATE SET value='true'",[])?;Ok(())})?;
    Ok(())
}
