//! Comandi delle voci di catalogazione create dall'utente.

use crate::settings::{retune_after_taxonomy, submit_retune};
use crate::state::AppState;
use pc_ai::entries::{self, CategoryRef};
use pc_ai::taxonomy;
use pc_core::{AppError, AppResult};
use pc_store::model::MediaId;
use tauri::State;

/// "Nuova voce…": crea nel gruppo una voce con questo nome (o riusa quella
/// con lo stesso nome) e, con `assign_to`, la assegna come correzione manuale.
#[tauri::command]
#[specta::specta]
pub async fn add_category(
    state: State<'_, AppState>,
    group: String,
    label: String,
    prompts: Vec<String>,
    assign_to: Option<pc_store::query::Target>,
) -> AppResult<CategoryRef> {
    let (db, jobs) = (state.store.clone(), state.jobs.clone());
    tauri::async_runtime::spawn_blocking(move || {
        let assign_to: Option<Vec<MediaId>> = match assign_to {
            Some(t) => Some(db.read(|c| t.resolve(c))?),
            None => None,
        };
        entries::add_and_assign(&db, &jobs, &group, &label, prompts, assign_to)
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))?
}

/// Foto a cui è assegnata la voce (per la conferma dell'eliminazione).
#[tauri::command]
#[specta::specta]
pub async fn category_usage(app: tauri::AppHandle, group: String, id: String) -> AppResult<i64> {
    crate::state::blocking(app, move |_app, state| {
        entries::usage(&state.store, &group, &id)
    })
    .await
}

/// Elimina la voce e la toglie dalle foto (annullabile). Se era automatica,
/// le catalogazioni si ricalcolano dagli embedding. Restituisce le foto toccate.
#[tauri::command]
#[specta::specta]
pub async fn delete_category(
    state: State<'_, AppState>,
    group: String,
    id: String,
) -> AppResult<u32> {
    let db = state.store.clone();
    let (n, spec) = tauri::async_runtime::spawn_blocking(move || {
        let before = taxonomy::load(&db)?;
        let n = entries::delete(&db, &group, &id)?;
        let after = taxonomy::load(&db)?;
        AppResult::Ok((n, retune_after_taxonomy(&before, &after)))
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))??;
    submit_retune(&state.jobs, spec)?;
    Ok(n as u32)
}
