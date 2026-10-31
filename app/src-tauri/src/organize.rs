use crate::state::AppState;
use pc_core::{AppError, AppResult};
use pc_organize::{export, organize};
use tauri::State;
#[tauri::command]
#[specta::specta]
pub fn export_fields() -> Vec<String> {
    export::FIELDS.iter().map(|s| s.to_string()).collect()
}
#[tauri::command]
#[specta::specta]
pub fn start_export(state: State<'_, AppState>, options: export::Options) -> AppResult<i64> {
    state
        .jobs
        .submit(export::JOB_KIND, serde_json::to_value(options)?)
}
#[tauri::command]
#[specta::specta]
pub async fn organize_preview(
    state: State<'_, AppState>,
    options: organize::Options,
) -> AppResult<organize::Preview> {
    let db = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || organize::preview(&db, &options))
        .await
        .map_err(|e| AppError::internal(e.to_string()))?
}
#[tauri::command]
#[specta::specta]
pub async fn organize_preflight(
    state: State<'_, AppState>,
    options: organize::Options,
) -> AppResult<organize::Preflight> {
    let db = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || organize::preflight(&db, &options))
        .await
        .map_err(|e| AppError::internal(e.to_string()))?
}
#[tauri::command]
#[specta::specta]
pub fn start_organize(state: State<'_, AppState>, options: organize::Options) -> AppResult<i64> {
    state
        .jobs
        .submit(organize::JOB_KIND, serde_json::to_value(options)?)
}
#[derive(serde::Serialize, specta::Type)]
pub struct Organization {
    pub id: i64,
    pub dest: String,
    pub method: String,
    pub state: String,
    pub entries: i64,
}
#[tauri::command]
#[specta::specta]
pub fn list_organizations(state: State<'_, AppState>) -> AppResult<Vec<Organization>> {
    Ok(state.store.read(|c|{
    let mut q=c.prepare("SELECT o.id,o.dest,o.method,o.state,count(e.created) FROM organize_op o LEFT JOIN organize_entry e ON e.op_id=o.id GROUP BY o.id ORDER BY o.id DESC LIMIT 50")?;
    let v=q.query_map([],|r|Ok(Organization{id:r.get(0)?,dest:r.get(1)?,method:r.get(2)?,state:r.get(3)?,entries:r.get(4)?}))?.collect::<Result<_,_>>()?;Ok(v)
})?)
}
#[tauri::command]
#[specta::specta]
pub async fn undo_organization(
    state: State<'_, AppState>,
    id: i64,
    confirmed: bool,
) -> AppResult<(u64, u64)> {
    if !confirmed {
        return Err(AppError::invalid_input("conferma richiesta"));
    }
    let db = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || organize::undo(&db, id))
        .await
        .map_err(|e| AppError::internal(e.to_string()))?
}
