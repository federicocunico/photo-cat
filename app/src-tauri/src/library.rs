//! Comandi di sorgenti, import e libreria; protocollo `thumb://` per le miniature.

use crate::state::AppState;
use pc_core::{AppError, AppResult, ErrorCode};
use pc_ingest::watch::SourceWatcher;
use pc_ingest::{import, manage, relink};
use pc_store::detail::{self, CatalogSummary, MediaDetail, SourceSummary};
use pc_store::model::{MediaId, NewSource, Source};
use pc_store::query::{self, Facets, LibraryFilter, Page, PageAt, SortKey};
use pc_store::repo;
use pc_store::sources::{self, RemovalSummary, UpdateSummary};
use std::path::Path;
use std::time::Duration;
use tauri::{AppHandle, Manager, State};

#[tauri::command]
#[specta::specta]
pub async fn list_sources(app: tauri::AppHandle) -> AppResult<Vec<SourceSummary>> {
    crate::state::blocking(app, move |_app, state| {
        Ok(state.store.read(detail::source_summaries)?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn add_source(app: tauri::AppHandle, source: NewSource) -> AppResult<i64> {
    crate::state::blocking(app, move |app, state| {
        if !Path::new(&source.path).is_dir() {
            return Err(AppError::new(
                ErrorCode::NotFound,
                format!("cartella non trovata: {}", source.path),
            ));
        }
        let mut source = source;
        source.path = dunce_path(Path::new(&source.path));
        let id = state.store.write(move |c| {
            let id = repo::source::insert(c, &source)?;
            // File ricollegati in questa cartella prima che fosse una sorgente.
            sources::adopt_strays(c, id)?;
            Ok(id)
        })?;
        tracing::info!(source_id = id, "sorgente aggiunta");
        restart_watcher(app);
        Ok(id)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn update_source(app: tauri::AppHandle, source: Source) -> AppResult<()> {
    crate::state::blocking(app, move |app, state| {
        state
            .store
            .write(move |c| repo::source::update(c, &source))?;
        restart_watcher(app);
        Ok(())
    })
    .await
}

/// Cosa comporta la rimozione di una sorgente (mostrato prima della conferma).
#[tauri::command]
#[specta::specta]
pub async fn source_removal_summary(app: tauri::AppHandle, id: i64) -> AppResult<RemovalSummary> {
    crate::state::blocking(app, move |_app, state| {
        manage::removal_summary(&state.jobs, &state.store, id)
    })
    .await
}

/// Rimuove la sorgente e i suoi elementi dalla libreria (mai i file dal disco):
/// annulla prima i job della sorgente, poi cancella miniature e dati derivati.
#[tauri::command]
#[specta::specta]
pub async fn remove_source(app: AppHandle, state: State<'_, AppState>, id: i64) -> AppResult<()> {
    let (jobs, store, thumbs) = (
        state.jobs.clone(),
        state.store.clone(),
        state.thumbs.clone(),
    );
    tauri::async_runtime::spawn_blocking(move || {
        manage::remove_source(&jobs, &store, &thumbs, id, manage::CANCEL_TIMEOUT)
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))??;
    restart_watcher(&app);
    Ok(())
}

/// Aggiorna la sorgente o solo una sua cartella (`folder`).
#[tauri::command]
#[specta::specta]
pub async fn start_import(
    app: tauri::AppHandle,
    source_id: i64,
    folder: Option<String>,
) -> AppResult<i64> {
    crate::state::blocking(app, move |_app, state| {
        let mut params = serde_json::json!({ "sourceId": source_id });
        if let Some(f) = folder.filter(|f| !f.trim().is_empty()) {
            params["folder"] = f.into();
        }
        state.jobs.submit(import::JOB_KIND, params)
    })
    .await
}

/// Riepiloghi dell'ultimo aggiornamento di ogni sorgente.
#[tauri::command]
#[specta::specta]
pub async fn import_updates(app: tauri::AppHandle) -> AppResult<Vec<UpdateSummary>> {
    crate::state::blocking(app, move |_app, state| {
        Ok(state.store.read(sources::updates)?)
    })
    .await
}

/// Cartella comune più profonda dei mancanti (proposta per "Ricollega").
#[tauri::command]
#[specta::specta]
pub async fn missing_folder(app: tauri::AppHandle, source_id: i64) -> AppResult<Option<String>> {
    crate::state::blocking(app, move |_app, state| {
        let paths: Vec<String> = state
            .store
            .read(|c| sources::missing(c, source_id, None))?
            .into_iter()
            .map(|(_, p)| p)
            .collect();
        Ok(sources::common_folder(&paths))
    })
    .await
}

/// Ricollega i mancanti sotto `old_folder` cercandoli in `new_folder` (job).
#[tauri::command]
#[specta::specta]
pub async fn relink_missing(
    app: tauri::AppHandle,
    source_id: i64,
    old_folder: String,
    new_folder: String,
) -> AppResult<i64> {
    crate::state::blocking(app, move |_app, state| {
        state.jobs.submit(
            relink::JOB_KIND,
            serde_json::json!({ "sourceId": source_id, "oldFolder": old_folder, "newFolder": new_folder }),
        )
    })
    .await
}

/// Esito dell'ultimo ricollegamento della sorgente.
#[tauri::command]
#[specta::specta]
pub async fn last_relink(
    app: tauri::AppHandle,
    source_id: i64,
) -> AppResult<Option<relink::RelinkSummary>> {
    crate::state::blocking(app, move |_app, state| {
        relink::last(&state.store, source_id)
    })
    .await
}

/// Rimuove dalla libreria tutti i mancanti della sorgente; restituisce quanti.
#[tauri::command]
#[specta::specta]
pub async fn remove_missing(
    app: tauri::AppHandle,
    source_id: i64,
    confirmed: bool,
) -> AppResult<u32> {
    crate::state::blocking(app, move |_app, state| {
        if !confirmed {
            return Err(AppError::invalid_input("conferma richiesta"));
        }
        let removed = manage::remove_missing(&state.store, &state.thumbs, source_id)?;
        Ok(removed.items as u32)
    })
    .await
}

/// Sposta la sorgente su una nuova cartella (dopo un ricollegamento completo).
#[tauri::command]
#[specta::specta]
pub async fn move_source(app: tauri::AppHandle, id: i64, path: String) -> AppResult<()> {
    crate::state::blocking(app, move |app, state| {
        if !Path::new(&path).is_dir() {
            return Err(AppError::new(
                ErrorCode::NotFound,
                format!("cartella non trovata: {path}"),
            ));
        }
        let path = dunce_path(Path::new(&path));
        state.store.write(move |c| {
            sources::set_source_path(c, id, &path)?;
            sources::adopt_strays(c, id)?;
            Ok(())
        })?;
        tracing::info!(source_id = id, "cartella della sorgente cambiata");
        restart_watcher(app);
        Ok(())
    })
    .await
}

/// Pulizia una tantum delle miniature orfane, in background.
pub fn purge_orphan_thumbs(app: &AppHandle, started: std::time::SystemTime) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let (store, thumbs) = (state.store.clone(), state.thumbs.clone());
    let spawned = std::thread::Builder::new()
        .name("pulizia-miniature".into())
        .spawn(move || {
            if let Err(e) = manage::purge_orphan_thumbs_once(&store, &thumbs, started) {
                tracing::warn!(
                    code = e.code.as_str(),
                    detail = e.detail.as_str(),
                    "pulizia delle miniature orfane"
                );
            }
        });
    if let Err(e) = spawned {
        tracing::warn!(detail = %e, "pulizia delle miniature non avviata");
    }
}

#[tauri::command]
#[specta::specta]
pub async fn library_page(
    app: tauri::AppHandle,
    filter: LibraryFilter,
    sort: SortKey,
    at: Option<PageAt>,
    limit: u32,
) -> AppResult<Page> {
    crate::state::blocking(app, move |_app, state| {
        Ok(state
            .store
            .read(|c| query::page_at(c, &filter, sort, at.as_ref(), limit))?)
    })
    .await
}

/// Etichetta di una richiesta della UI: le richieste più vecchie dello stesso canale vengono
/// scartate o interrotte (filtro cambiato mentre il motore calcolava).
#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RequestTag {
    pub channel: String,
    pub id: u32,
}

/// Esegue la lettura, o la scarta se `request` è stata superata (`None`).
fn gated<T>(
    state: &AppState,
    request: Option<&RequestTag>,
    f: impl FnOnce(&rusqlite::Connection) -> Result<T, pc_store::StoreError>,
) -> AppResult<Option<T>> {
    Ok(match request {
        Some(r) => state
            .gates
            .run(&state.store, &r.channel, u64::from(r.id), f)?,
        None => Some(state.store.read(f)?),
    })
}

/// Numero di elementi del filtro; `None` se la richiesta è stata superata.
#[tauri::command]
#[specta::specta]
pub async fn library_count(
    app: tauri::AppHandle,
    filter: LibraryFilter,
    request: Option<RequestTag>,
) -> AppResult<Option<i64>> {
    crate::state::blocking(app, move |_app, state| {
        gated(state, request.as_ref(), |c| query::count(c, &filter))
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn library_facets(
    app: tauri::AppHandle,
    filter: LibraryFilter,
    request: Option<RequestTag>,
) -> AppResult<Option<Facets>> {
    crate::state::blocking(app, move |_app, state| {
        gated(state, request.as_ref(), |c| query::facets(c, &filter))
    })
    .await
}

/// Elenco completo dei valori di una lista di filtro (ricerca, ordinamento, pagine).
#[tauri::command]
#[specta::specta]
pub async fn library_facet_values(
    state: State<'_, AppState>,
    filter: LibraryFilter,
    facet: query::FacetKey,
    search: Option<String>,
    sort: query::FacetSort,
    offset: u32,
    limit: u32,
) -> AppResult<query::FacetPage> {
    let db = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.read(|c| query::facet_values(c, &filter, &facet, search.as_deref(), sort, offset, limit))
            .map_err(Into::into)
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))?
}

/// Base di "Seleziona tutto": numero di risultati e id massimo in quell'istante. La selezione
/// resta "filtro più eccezioni" e non porta mai nell'interfaccia l'elenco di tutti gli id.
#[tauri::command]
#[specta::specta]
pub async fn library_selection(
    app: tauri::AppHandle,
    filter: LibraryFilter,
) -> AppResult<query::SelectionBase> {
    crate::state::blocking(app, move |_app, state| {
        Ok(state.store.read(|c| query::selection_base(c, &filter))?)
    })
    .await
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
pub async fn media_detail(app: tauri::AppHandle, id: MediaId) -> AppResult<MediaDetail> {
    crate::state::blocking(app, move |_app, state| {
        Ok(state.store.read(|c| detail::media_detail(c, id))?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn list_catalogs(app: tauri::AppHandle) -> AppResult<Vec<CatalogSummary>> {
    crate::state::blocking(app, move |_app, state| {
        Ok(state.store.read(detail::catalog_summaries)?)
    })
    .await
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

/// Cache della WebView per le miniature: un giorno, senza `immutable` (la WebView può
/// liberarle sotto pressione e una miniatura rigenerata viene riletta).
pub const THUMB_CACHE_CONTROL: &str = "max-age=86400";

/// Risposta di `thumb://` per il file di `path` (letto qui: chiamare fuori dal thread principale).
pub fn thumb_response(path: Option<std::path::PathBuf>) -> tauri::http::Response<Vec<u8>> {
    match path.map(std::fs::read) {
        Some(Ok(bytes)) => tauri::http::Response::builder()
            .header("Content-Type", "image/jpeg")
            .header("Cache-Control", THUMB_CACHE_CONTROL)
            .header("Access-Control-Allow-Origin", "*")
            .body(bytes),
        _ => tauri::http::Response::builder()
            .status(404)
            .body(Vec::new()),
    }
    .expect("risposta miniatura")
}

/// Serve `thumb://localhost/<chiave>` dalla cache delle miniature, leggendo il file su un
/// thread del pool di I/O: il thread principale (che ospita la WebView) non attende mai il disco.
pub fn thumb_protocol(
    ctx: tauri::UriSchemeContext<'_, tauri::Wry>,
    request: tauri::http::Request<Vec<u8>>,
    responder: tauri::UriSchemeResponder,
) {
    let path = ctx.app_handle().try_state::<AppState>().and_then(|state| {
        let key = request.uri().path().trim_start_matches('/');
        state.thumbs.path_of(key)
    });
    tauri::async_runtime::spawn_blocking(move || responder.respond(thumb_response(path)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumbnails_are_cached_without_immutable() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("t.jpg");
        std::fs::write(&p, b"jpeg").unwrap();
        let r = thumb_response(Some(p));
        assert_eq!(r.status(), 200);
        let cc = r.headers()["Cache-Control"].to_str().unwrap();
        assert!(!cc.contains("immutable"), "{cc}");
        assert_eq!(cc, "max-age=86400");
        assert_eq!(r.body(), b"jpeg");
        assert_eq!(
            thumb_response(Some(tmp.path().join("assente"))).status(),
            404
        );
        assert_eq!(thumb_response(None).status(), 404);
    }
}
