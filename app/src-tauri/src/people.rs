//! Comandi per le correzioni delle identità; non espongono embedding alla UI.
use crate::state::AppState;
use pc_core::{AppError, AppResult};
use pc_store::people::{self, FaceView, PersonView};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize, specta::Type)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Correction {
    Rename { person: i64, name: String },
    Merge { from: i64, to: i64 },
    Separate { person: i64, faces: Vec<i64> },
    Assign { person: i64, faces: Vec<i64> },
    Reject { person: i64, face: i64 },
    Remove { face: i64 },
}
#[tauri::command]
#[specta::specta]
pub async fn list_people(state: State<'_, AppState>) -> AppResult<Vec<PersonView>> {
    let db = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || db.read(people::list).map_err(Into::into))
        .await
        .map_err(|e| AppError::internal(e.to_string()))?
}
#[tauri::command]
#[specta::specta]
pub fn face_count(state: State<'_, AppState>) -> AppResult<i64> {
    Ok(state
        .store
        .read(|c| Ok(c.query_row("SELECT count(*) FROM face", [], |r| r.get(0))?))?)
}
#[tauri::command]
#[specta::specta]
pub async fn person_faces(
    state: State<'_, AppState>,
    person: Option<i64>,
) -> AppResult<Vec<FaceView>> {
    let db = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.read(|c| people::views(c, person)).map_err(Into::into)
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))?
}
#[tauri::command]
#[specta::specta]
pub async fn correct_person(state: State<'_, AppState>, correction: Correction) -> AppResult<()> {
    let db = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.write(move |c| {
            match correction {
                Correction::Rename { person, name } => people::rename(c, person, &name)?,
                Correction::Merge { from, to } => people::merge(c, from, to)?,
                Correction::Separate { person, faces } => {
                    people::separate(c, &faces, person)?;
                }
                Correction::Assign { person, faces } => people::assign(c, &faces, person)?,
                Correction::Reject { person, face } => people::reject(c, face, person)?,
                Correction::Remove { face } => people::remove_face(c, face)?,
            }
            Ok(())
        })
        .map_err(Into::into)
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))?
}
/// Disabilita prima di svuotare il writer; i risultati in volo vengono invalidati.
#[tauri::command]
#[specta::specta]
pub async fn purge_faces(state: State<'_, AppState>) -> AppResult<()> {
    let mut cfg = state.config.read().clone();
    cfg.ai.faces_enabled = false;
    state.save_config(cfg)?;
    let db = state.store.clone();
    let crops = state.paths.face_crops();
    tauri::async_runtime::spawn_blocking(move || {
        db.write(people::clear)?;
        // Anche i ritagli in cache sono dati dei volti.
        match std::fs::remove_dir_all(&crops) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e.into()),
            _ => {}
        }
        db.vacuum()?;
        Ok::<_, pc_core::AppError>(())
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))?
}

/// `face://<id>`: ritaglio del volto generato dall'originale (vedi `pc_ai::face_crop`),
/// fuori dal thread della UI.
pub fn face_protocol(
    ctx: tauri::UriSchemeContext<'_, tauri::Wry>,
    request: tauri::http::Request<Vec<u8>>,
    responder: tauri::UriSchemeResponder,
) {
    use tauri::Manager;
    let handle = ctx.app_handle().clone();
    let id = request
        .uri()
        .path()
        .trim_start_matches('/')
        .parse::<i64>()
        .ok();
    if id.is_none() {
        tracing::warn!(uri = %request.uri(), "richiesta ritaglio volto non valida");
    }
    tauri::async_runtime::spawn_blocking(move || {
        let bytes = (|| {
            let state = handle.try_state::<AppState>()?;
            let dir = state.paths.face_crops();
            pc_ai::face_crop::jpeg(&state.store, &state.governor, &dir, id?)
                .inspect_err(
                    |e| tracing::warn!(face_id = id, detail = %e, "ritaglio volto non disponibile"),
                )
                .ok()
        })();
        let response = match bytes {
            Some(bytes) => tauri::http::Response::builder()
                .header("Content-Type", "image/jpeg")
                .header("Cache-Control", "max-age=3600")
                .header("Access-Control-Allow-Origin", "*")
                .body(bytes),
            None => tauri::http::Response::builder()
                .status(404)
                .body(Vec::new()),
        };
        responder.respond(response.expect("risposta ritaglio volto"));
    });
}
