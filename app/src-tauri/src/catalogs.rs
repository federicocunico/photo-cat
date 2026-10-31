use crate::state::AppState;
use pc_core::{AppError, AppResult};
use pc_store::{bulk, model::CatalogKind, repo};
use tauri::State;

#[tauri::command]
#[specta::specta]
pub async fn save_catalog(
    state: State<'_, AppState>,
    id: Option<i64>,
    name: String,
    kind: CatalogKind,
    rules: Option<serde_json::Value>,
) -> AppResult<i64> {
    if kind == CatalogKind::Smart {
        pc_store::rules::parse(
            rules
                .as_ref()
                .ok_or_else(|| AppError::invalid_input("regole richieste"))?,
        )?;
    }
    let db = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.write(move |c| {
            if let Some(id) = id {
                repo::catalog::get(c, id)?;
                repo::catalog::rename(c, id, &name)?;
                c.execute(
                    "UPDATE catalog SET kind=?2,rules=?3 WHERE id=?1",
                    rusqlite::params![id, kind.as_str(), rules.map(|r| r.to_string())],
                )?;
                Ok(id)
            } else {
                repo::catalog::create(c, &name, kind, rules.as_ref())
            }
        })
        .map_err(Into::into)
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))?
}
#[tauri::command]
#[specta::specta]
pub async fn delete_catalog(state: State<'_, AppState>, id: i64, confirmed: bool) -> AppResult<()> {
    if !confirmed {
        return Err(AppError::invalid_input("conferma richiesta"));
    }
    let db = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.write(move |c| repo::catalog::delete(c, id))
            .map_err(Into::into)
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))?
}
#[tauri::command]
#[specta::specta]
pub fn start_bulk(
    state: State<'_, AppState>,
    ids: Vec<i64>,
    operation: pc_jobs::bulk::BulkAction,
    confirmed: bool,
) -> AppResult<i64> {
    state.jobs.submit(
        pc_jobs::bulk::JOB_KIND,
        serde_json::json!({"ids":ids,"operation":operation,"confirmed":confirmed}),
    )
}
#[tauri::command]
#[specta::specta]
pub async fn undo_bulk(state: State<'_, AppState>) -> AppResult<Option<String>> {
    let db = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || db.write(bulk::undo_last).map_err(Into::into))
        .await
        .map_err(|e| AppError::internal(e.to_string()))?
}
#[tauri::command]
#[specta::specta]
pub fn last_bulk_undo(state: State<'_, AppState>) -> AppResult<Option<String>> {
    Ok(state.store.read(bulk::last_undoable)?)
}
