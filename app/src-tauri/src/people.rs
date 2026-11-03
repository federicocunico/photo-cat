//! Comandi per le correzioni delle identità; non espongono embedding alla UI.
use crate::events;
use crate::state::AppState;
use pc_core::{AppError, AppResult};
use pc_store::bulk::{self, UndoAction};
use pc_store::people::{
    self, FaceCursor, FacePage, FaceScope, FaceSort, FaceView, GroupPage, GroupSort,
    LocateFaceRequest, LocatedFace, PeopleSection, PeopleSections, PersonView,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Serialize, Deserialize, specta::Type)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Correction {
    Rename { person: i64, name: String },
    Merge { from: i64, to: i64 },
    Separate { person: i64, faces: Vec<i64> },
    Assign { person: i64, faces: Vec<i64> },
    Reject { person: i64, face: i64 },
    Remove { face: i64 },
    MergeMany { from: Vec<i64>, to: i64 },
    Move { faces: Vec<i64>, to: i64 },
    NotSame { a: i64, b: i64 },
    Ignore { group: i64 },
    Restore { group: i64 },
    Recover { faces: Vec<i64> },
    SetCover { group: i64, face: i64 },
}

fn face_match(state: &AppState) -> people::FaceMatch {
    let cfg = state.config.read();
    pc_ai::analyze::face_match_with_size(&cfg.ai.thresholds, cfg.ai.face_min_size_px)
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
pub async fn people_sections(state: State<'_, AppState>) -> AppResult<PeopleSections> {
    let (db, m) = (state.store.clone(), face_match(&state));
    tauri::async_runtime::spawn_blocking(move || {
        db.read(move |c| people::people_sections(c, &m))
            .map_err(Into::into)
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))?
}

#[tauri::command]
#[specta::specta]
pub async fn list_groups(
    state: State<'_, AppState>,
    section: PeopleSection,
    query: Option<String>,
    sort: GroupSort,
    after: Option<i64>,
    limit: Option<i64>,
) -> AppResult<GroupPage> {
    let (db, m) = (state.store.clone(), face_match(&state));
    tauri::async_runtime::spawn_blocking(move || {
        db.read(move |c| {
            people::list_groups(
                c,
                section,
                query.as_deref(),
                sort,
                after,
                limit.unwrap_or(50),
                &m,
            )
        })
        .map_err(Into::into)
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))?
}

#[tauri::command]
#[specta::specta]
pub async fn group_faces(
    state: State<'_, AppState>,
    scope: FaceScope,
    sort: FaceSort,
    after: Option<FaceCursor>,
    limit: Option<i64>,
) -> AppResult<FacePage> {
    let (db, m) = (state.store.clone(), face_match(&state));
    tauri::async_runtime::spawn_blocking(move || {
        db.read(move |c| {
            people::group_faces(c, scope, sort, after.as_ref(), limit.unwrap_or(100), &m)
        })
        .map_err(Into::into)
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))?
}

#[tauri::command]
#[specta::specta]
pub async fn locate_face(
    state: State<'_, AppState>,
    request: LocateFaceRequest,
) -> AppResult<LocatedFace> {
    let (db, m) = (state.store.clone(), face_match(&state));
    tauri::async_runtime::spawn_blocking(move || {
        db.read(move |c| people::locate_face(c, &request, &m))
            .map_err(Into::into)
    })
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
    let (db, m) = (state.store.clone(), face_match(&state));
    tauri::async_runtime::spawn_blocking(move || {
        db.read(move |c| people::views_with(c, person, &m))
            .map_err(Into::into)
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))?
}
#[tauri::command]
#[specta::specta]
pub async fn correct_person(
    app: AppHandle,
    state: State<'_, AppState>,
    correction: Correction,
) -> AppResult<()> {
    let db = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.write(move |c| {
            match correction {
                Correction::Rename { person, name } => people::rename(c, person, &name)?,
                Correction::Merge { from, to } => {
                    let snap = people::person_snapshot(c, from)?;
                    let faces = people::faces_of_persons(c, &[from])?;
                    people::merge(c, from, to)?;
                    bulk::record_undo(
                        c,
                        "Unisci persone",
                        &UndoAction::PersonsMerged {
                            from: vec![snap],
                            faces,
                            to,
                        },
                    )?;
                }
                Correction::Separate { person, faces } => {
                    people::separate(c, &faces, person)?;
                }
                Correction::Assign { person, faces } => {
                    let prev = people::face_person_ids(c, &faces)?;
                    people::assign(c, &faces, person)?;
                    if !prev.is_empty() {
                        bulk::record_undo(
                            c,
                            "Sposta volti",
                            &UndoAction::FacesMoved {
                                faces: prev,
                                to: person,
                            },
                        )?;
                    }
                }
                Correction::Reject { person, face } => people::reject(c, face, person)?,
                Correction::Remove { face } => people::remove_face(c, face)?,
                Correction::MergeMany { from, to } => {
                    let from: Vec<i64> = from.into_iter().filter(|id| *id != to).collect();
                    if from.is_empty() {
                        return Err(pc_store::StoreError::Invalid(
                            "nessun gruppo da unire".into(),
                        ));
                    }
                    let mut snaps = Vec::with_capacity(from.len());
                    for id in &from {
                        snaps.push(people::person_snapshot(c, *id)?);
                    }
                    let faces = people::faces_of_persons(c, &from)?;
                    people::merge_many(c, &from, to)?;
                    bulk::record_undo(
                        c,
                        "Unisci persone",
                        &UndoAction::PersonsMerged {
                            from: snaps,
                            faces,
                            to,
                        },
                    )?;
                }
                Correction::Move { faces, to } => {
                    let prev = people::face_person_ids(c, &faces)?;
                    people::move_faces(c, &faces, to)?;
                    if !prev.is_empty() {
                        bulk::record_undo(
                            c,
                            "Sposta volti",
                            &UndoAction::FacesMoved { faces: prev, to },
                        )?;
                    }
                }
                Correction::NotSame { a, b } => {
                    people::mark_not_same(c, a, b)?;
                    bulk::record_undo(c, "Persone diverse", &UndoAction::PersonNotSame { a, b })?;
                }
                Correction::Ignore { group } => {
                    let (prev_ignored_at, prev_ignored_faces) = people::ignore_person(c, group)?;
                    bulk::record_undo(
                        c,
                        "Ignora persona",
                        &UndoAction::PersonIgnored {
                            id: group,
                            prev_ignored_at,
                            prev_ignored_faces,
                        },
                    )?;
                }
                Correction::Restore { group } => {
                    let (prev_ignored_at, prev_ignored_faces) = people::restore_person(c, group)?;
                    bulk::record_undo(
                        c,
                        "Ripristina persona",
                        &UndoAction::PersonRestored {
                            id: group,
                            prev_ignored_at,
                            prev_ignored_faces,
                        },
                    )?;
                }
                Correction::Recover { faces } => {
                    if !faces.is_empty() {
                        people::recover_faces(c, &faces)?;
                        bulk::record_undo(
                            c,
                            "Recupera volti",
                            &UndoAction::FacesRecovered { ids: faces },
                        )?;
                    }
                }
                Correction::SetCover { group, face } => {
                    let prev = people::cover_face_id(c, group)?;
                    people::set_cover(c, group, face)?;
                    bulk::record_undo(
                        c,
                        "Imposta copertina",
                        &UndoAction::PersonCover { group, prev },
                    )?;
                }
            }
            Ok(())
        })
        .map_err(AppError::from)
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))??;
    // Evento UI: vedi commento su `events::FACES_CHANGED` (emit solo da correzione in 2.2).
    let _ = app.emit(events::FACES_CHANGED, ());
    Ok(())
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
    let app_store = state.app_store.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let app = app_store.clone();
        db.write(move |c| people::clear_with_references(c, &app))?;
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
