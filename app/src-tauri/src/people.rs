//! Comandi per le correzioni delle identità; non espongono embedding alla UI.
use crate::events;
use crate::state::AppState;
use pc_core::{AppError, AppResult};
use pc_store::bulk::{self, UndoAction};
use pc_store::people::{
    self, FaceCursor, FacePage, FaceScope, FaceSort, FaceView, GroupPage, GroupRow, GroupSort,
    LocateFaceRequest, LocatedFace, PeopleSection, PeopleSections, PersonView,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

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

/// Un singolo gruppo, in qualunque sezione: `None` se non esiste più.
#[tauri::command]
#[specta::specta]
pub async fn get_group(state: State<'_, AppState>, id: i64) -> AppResult<Option<GroupRow>> {
    let (db, m) = (state.store.clone(), face_match(&state));
    tauri::async_runtime::spawn_blocking(move || {
        db.read(move |c| people::group_row(c, id, &m))
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
pub async fn face_count(app: tauri::AppHandle) -> AppResult<i64> {
    crate::state::blocking(app, move |_app, state| {
        Ok(state
            .store
            .read(|c| Ok(c.query_row("SELECT count(*) FROM face", [], |r| r.get(0))?))?)
    })
    .await
}
#[tauri::command]
#[specta::specta]
pub async fn person_faces(
    state: State<'_, AppState>,
    person: Option<i64>,
    limit: Option<u32>,
) -> AppResult<Vec<FaceView>> {
    let (db, m) = (state.store.clone(), face_match(&state));
    // Memoria limitata anche per le persone con migliaia di volti.
    let limit = i64::from(limit.unwrap_or(500).clamp(1, 2000));
    tauri::async_runtime::spawn_blocking(move || {
        db.read(move |c| people::views_limited(c, person, Some(limit), &m))
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
                Correction::Merge { from, to } => merge_with_undo(c, vec![from], to)?,
                Correction::Separate { person, faces } => {
                    let touched = people::faces_touching(c, &[person])?;
                    let states = people::face_states(c, &touched)?;
                    let created = people::separate(c, &faces, person)?;
                    bulk::record_undo(
                        c,
                        "Separa volti",
                        &UndoAction::FacesRestored {
                            states,
                            created: Some(created),
                        },
                    )?;
                }
                Correction::Assign { person, faces } => {
                    move_with_undo(c, &faces, person, |c| people::assign(c, &faces, person))?
                }
                Correction::Reject { person, face } => {
                    let states = people::face_states(c, &[face])?;
                    people::reject(c, face, person)?;
                    bulk::record_undo(
                        c,
                        "Non è questa persona",
                        &UndoAction::FacesRestored {
                            states,
                            created: None,
                        },
                    )?;
                }
                Correction::Remove { face } => people::remove_face(c, face)?,
                Correction::MergeMany { from, to } => {
                    let from: Vec<i64> = from.into_iter().filter(|id| *id != to).collect();
                    if from.is_empty() {
                        return Err(pc_store::StoreError::Invalid(
                            "nessun gruppo da unire".into(),
                        ));
                    }
                    merge_with_undo(c, from, to)?;
                }
                Correction::Move { faces, to } => {
                    move_with_undo(c, &faces, to, |c| people::move_faces(c, &faces, to))?
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
    events::faces_changed(&app);
    Ok(())
}
/// Unisce `from` in `to` registrando uno snapshot completo per l'annullamento: gruppi
/// eliminati, gruppo di destinazione, stato dei volti toccati e vincoli "persone diverse".
fn merge_with_undo(c: &rusqlite::Connection, from: Vec<i64>, to: i64) -> pc_store::StoreResult<()> {
    let mut snaps = Vec::with_capacity(from.len());
    for id in &from {
        snaps.push(people::person_snapshot(c, *id)?);
    }
    let target = people::person_snapshot(c, to)?;
    let faces = people::faces_of_persons(c, &from)?;
    let mut groups = from.clone();
    groups.push(to);
    let states = people::face_states(c, &people::faces_touching(c, &groups)?)?;
    let not_same = people::not_same_rows(c, &groups)?;
    people::merge_many(c, &from, to)?;
    bulk::record_undo(
        c,
        "Unisci persone",
        &UndoAction::PersonsMerged {
            from: snaps,
            faces,
            to,
            states,
            target: Some(target),
            not_same,
        },
    )
}

/// Sposta volti su un gruppo registrando lo stato completo dei volti per l'annullamento.
fn move_with_undo(
    c: &rusqlite::Connection,
    faces: &[i64],
    to: i64,
    apply: impl FnOnce(&rusqlite::Connection) -> pc_store::StoreResult<()>,
) -> pc_store::StoreResult<()> {
    let prev = people::face_person_ids(c, faces)?;
    let states = people::face_states(c, faces)?;
    apply(c)?;
    if !prev.is_empty() {
        bulk::record_undo(
            c,
            "Sposta volti",
            &UndoAction::FacesMoved {
                faces: prev,
                to,
                states,
            },
        )?;
    }
    Ok(())
}

/// Annulla l'ultima correzione delle persone (Ctrl+Z nel modulo Persone). Se l'ultima
/// operazione annullabile riguarda altro (tag, cataloghi…) non fa nulla e restituisce `None`.
#[tauri::command]
#[specta::specta]
pub async fn undo_people(app: AppHandle, state: State<'_, AppState>) -> AppResult<Option<String>> {
    let db = state.store.clone();
    let done = tauri::async_runtime::spawn_blocking(move || {
        db.write(pc_store::bulk::undo_last_people)
            .map_err(AppError::from)
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))??;
    if done.is_some() {
        events::faces_changed(&app);
    }
    Ok(done)
}

/// Disabilita prima di svuotare il writer; i risultati in volo vengono invalidati.
#[tauri::command]
#[specta::specta]
pub async fn purge_faces(app: tauri::AppHandle, state: State<'_, AppState>) -> AppResult<()> {
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
    .map_err(|e| AppError::internal(e.to_string()))??;
    crate::events::faces_changed(&app);
    Ok(())
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
    if handle.try_state::<AppState>().is_none() {
        responder.respond(
            tauri::http::Response::builder()
                .status(503)
                .header("Retry-After", "1")
                .body(Vec::new())
                .expect("risposta avvio"),
        );
        return;
    }
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
