//! Comandi di sorgenti, import e libreria; protocollo `thumb://` per le miniature.

use crate::state::AppState;
use pc_core::{AppError, AppResult, ErrorCode};
use pc_ingest::import;
use pc_ingest::watch::SourceWatcher;
use pc_store::detail::{self, CatalogSummary, MediaDetail, SourceSummary};
use pc_store::model::{MediaId, NewSource, Source};
use pc_store::query::{self, Cursor, Facets, LibraryFilter, Page, SortKey};
use pc_store::repo;
use std::path::Path;
use std::time::Duration;
use tauri::{AppHandle, Manager, State};

#[tauri::command]
#[specta::specta]
pub fn list_sources(state: State<'_, AppState>) -> AppResult<Vec<SourceSummary>> {
    Ok(state.store.read(detail::source_summaries)?)
}

#[tauri::command]
#[specta::specta]
pub fn add_source(app: AppHandle, state: State<'_, AppState>, source: NewSource) -> AppResult<i64> {
    if !Path::new(&source.path).is_dir() {
        return Err(AppError::new(
            ErrorCode::NotFound,
            format!("cartella non trovata: {}", source.path),
        ));
    }
    let mut source = source;
    source.path = dunce_path(Path::new(&source.path));
    let id = state
        .store
        .write(move |c| repo::source::insert(c, &source))?;
    tracing::info!(source_id = id, "sorgente aggiunta");
    restart_watcher(&app);
    Ok(id)
}

#[tauri::command]
#[specta::specta]
pub fn update_source(app: AppHandle, state: State<'_, AppState>, source: Source) -> AppResult<()> {
    state
        .store
        .write(move |c| repo::source::update(c, &source))?;
    restart_watcher(&app);
    Ok(())
}

/// Rimuove la sorgente e i suoi elementi dalla libreria (mai i file dal disco).
#[tauri::command]
#[specta::specta]
pub fn remove_source(app: AppHandle, state: State<'_, AppState>, id: i64) -> AppResult<()> {
    state.store.write(move |c| repo::source::delete(c, id))?;
    tracing::info!(source_id = id, "sorgente rimossa dalla libreria");
    restart_watcher(&app);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn start_import(state: State<'_, AppState>, source_id: i64) -> AppResult<i64> {
    state.jobs.submit(
        import::JOB_KIND,
        serde_json::json!({ "sourceId": source_id }),
    )
}

#[tauri::command]
#[specta::specta]
pub fn library_page(
    state: State<'_, AppState>,
    filter: LibraryFilter,
    sort: SortKey,
    after: Option<Cursor>,
    limit: u32,
) -> AppResult<Page> {
    Ok(state
        .store
        .read(|c| query::page(c, &filter, sort, after.as_ref(), limit))?)
}

#[tauri::command]
#[specta::specta]
pub fn library_count(state: State<'_, AppState>, filter: LibraryFilter) -> AppResult<i64> {
    Ok(state.store.read(|c| query::count(c, &filter))?)
}

#[tauri::command]
#[specta::specta]
pub fn library_facets(state: State<'_, AppState>, filter: LibraryFilter) -> AppResult<Facets> {
    Ok(state.store.read(|c| query::facets(c, &filter))?)
}

/// Tutti gli id del filtro corrente ("Seleziona tutto").
#[tauri::command]
#[specta::specta]
pub fn library_ids(state: State<'_, AppState>, filter: LibraryFilter) -> AppResult<Vec<MediaId>> {
    Ok(state.store.read(|c| query::ids(c, &filter))?)
}

/// Sottocartelle di una sorgente (o di `parent`), un livello alla volta.
#[tauri::command]
#[specta::specta]
pub async fn list_folders(
    state: State<'_, AppState>,
    source_id: i64,
    parent: Option<String>,
) -> AppResult<Vec<query::FolderNode>> {
    let db = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.read(|c| query::list_folders(c, source_id, parent.as_deref()))
            .map_err(Into::into)
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))?
}

#[tauri::command]
#[specta::specta]
pub fn media_detail(state: State<'_, AppState>, id: MediaId) -> AppResult<MediaDetail> {
    Ok(state.store.read(|c| detail::media_detail(c, id))?)
}

#[tauri::command]
#[specta::specta]
pub fn list_catalogs(state: State<'_, AppState>) -> AppResult<Vec<CatalogSummary>> {
    Ok(state.store.read(detail::catalog_summaries)?)
}

/// Percorso assoluto senza prefisso `\\?\` (come lo mostra Esplora risorse).
fn dunce_path(p: &Path) -> String {
    let abs = std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let s = abs.display().to_string();
    s.strip_prefix(r"\\?\").map(str::to_string).unwrap_or(s)
}

/// (Ri)avvia il monitoraggio delle sorgenti con `watch` attivo: alle modifiche
/// accoda un import incrementale se non ce n'è già uno per quella sorgente.
pub fn restart_watcher(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let sources = match state.store.read(repo::source::list) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(code = "DATABASE", detail = %e, "lettura sorgenti per il monitoraggio");
            return;
        }
    };
    let watched: Vec<_> = sources
        .into_iter()
        .filter(|s| s.watch)
        .map(|s| (s.id, std::path::PathBuf::from(s.path), s.recursive))
        .collect();
    let mut slot = state.watcher.lock();
    *slot = None;
    if watched.is_empty() {
        return;
    }
    let handle = app.clone();
    match SourceWatcher::start(watched, Duration::from_secs(2), move |id| {
        let Some(st) = handle.try_state::<AppState>() else {
            return;
        };
        let busy = st
            .jobs
            .has_active(import::JOB_KIND, |p| p["sourceId"].as_i64() == Some(id));
        if !busy {
            let _ = st
                .jobs
                .submit(import::JOB_KIND, serde_json::json!({ "sourceId": id }));
        }
    }) {
        Ok(w) => *slot = Some(w),
        Err(e) => tracing::warn!(
            code = e.code.as_str(),
            detail = e.detail.as_str(),
            "monitoraggio non avviato"
        ),
    }
}

/// Serve `thumb://localhost/<chiave>` dalla cache delle miniature.
pub fn thumb_protocol(
    ctx: tauri::UriSchemeContext<'_, tauri::Wry>,
    request: tauri::http::Request<Vec<u8>>,
) -> tauri::http::Response<Vec<u8>> {
    let not_found = || {
        tauri::http::Response::builder()
            .status(404)
            .body(Vec::new())
            .expect("risposta 404")
    };
    let Some(state) = ctx.app_handle().try_state::<AppState>() else {
        return not_found();
    };
    let key = request.uri().path().trim_start_matches('/');
    let Some(path) = state.thumbs.path_of(key) else {
        return not_found();
    };
    match std::fs::read(path) {
        Ok(bytes) => tauri::http::Response::builder()
            .header("Content-Type", "image/jpeg")
            .header("Cache-Control", "max-age=31536000, immutable")
            .header("Access-Control-Allow-Origin", "*")
            .body(bytes)
            .expect("risposta miniatura"),
        Err(_) => not_found(),
    }
}
