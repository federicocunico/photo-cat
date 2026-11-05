//! Comandi della sezione Mappa: gruppi a griglia, cataloghi nell'area, luoghi senza
//! coordinate e posizioni manuali dei luoghi. Solo lettura/scrittura di `place_geo`.

use crate::state::blocking;
use pc_core::AppResult;
use pc_store::geo_map::{self, CatalogCount, Cell, GeoBbox, PlaceCount, PlacePosition};
use pc_store::query::LibraryFilter;
use tauri::AppHandle;

#[tauri::command]
#[specta::specta]
pub async fn map_cells(
    app: AppHandle,
    filter: LibraryFilter,
    bbox: GeoBbox,
    zoom: u32,
    approx: bool,
) -> AppResult<Vec<Cell>> {
    blocking(app, move |_app, state| {
        let mut filter = filter;
        crate::stage_keys::fill_filter(state, &mut filter);
        Ok(state
            .store
            .read(move |c| geo_map::map_cells(c, &filter, &bbox, zoom, approx))?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn map_catalogs(
    app: AppHandle,
    filter: LibraryFilter,
    bbox: GeoBbox,
    approx: bool,
) -> AppResult<Vec<CatalogCount>> {
    blocking(app, move |_app, state| {
        let mut filter = filter;
        crate::stage_keys::fill_filter(state, &mut filter);
        Ok(state
            .store
            .read(move |c| geo_map::map_catalogs(c, &filter, &bbox, approx))?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn map_places_unplaced(
    app: AppHandle,
    filter: LibraryFilter,
) -> AppResult<Vec<PlaceCount>> {
    blocking(app, move |_app, state| {
        let mut filter = filter;
        crate::stage_keys::fill_filter(state, &mut filter);
        Ok(state
            .store
            .read(move |c| geo_map::map_places_unplaced(c, &filter))?)
    })
    .await
}

/// Luoghi con posizione manuale (modifica e "Rimuovi posizione").
#[tauri::command]
#[specta::specta]
pub async fn map_places_manual(
    app: AppHandle,
    filter: LibraryFilter,
) -> AppResult<Vec<PlacePosition>> {
    blocking(app, move |_app, state| {
        let mut filter = filter;
        crate::stage_keys::fill_filter(state, &mut filter);
        Ok(state
            .store
            .read(move |c| geo_map::map_places_manual(c, &filter))?)
    })
    .await
}

/// "Posiziona": posizione manuale del luogo.
#[tauri::command]
#[specta::specta]
pub async fn map_place_set(app: AppHandle, name: String, lat: f64, lon: f64) -> AppResult<()> {
    blocking(app, move |_app, state| {
        Ok(state
            .store
            .write(move |c| geo_map::map_place_set(c, &name, lat, lon))?)
    })
    .await
}

/// "Rimuovi posizione": toglie quella manuale e rimette l'eventuale risultato del gazetteer.
#[tauri::command]
#[specta::specta]
pub async fn map_place_clear(app: AppHandle, name: String) -> AppResult<()> {
    blocking(app, move |_app, state| {
        state
            .store
            .write(move |c| geo_map::map_place_clear(c, &name))?;
        // La rimozione è già fatta: un errore del gazetteer non la fa risultare fallita.
        if let Err(e) = pc_ai::place_geo::run_inline(&state.ai_deps) {
            tracing::warn!(
                code = e.code.as_str(),
                detail = e.detail.as_str(),
                "place_geo dopo Rimuovi posizione"
            );
        }
        Ok(())
    })
    .await
}
