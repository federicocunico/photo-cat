//! Comandi dell'esportazione delle Varianti RAW in un nuovo catalogo: verifica preliminare per il
//! dialogo, anteprima dei nomi, avvio dell'attività `raw_review_export` e suo riepilogo.
//!
//! La scelta degli scatti (solo JPEG scelti, valutazione minima, preset) nasce qui dalla sessione
//! aperta; il job riceve già l'elenco dei file e non si fida di nient'altro.

use crate::sequence::NamePreviewResult;
use crate::state::AppState;
use pc_core::{AppError, AppResult};
use pc_organize::raw_export::{
    self, CatalogConflict, ExistingCatalog, ImportTarget, RawExportItem, RawExportOptions,
    RawExportReport,
};
use pc_organize::sequence::job::NamePreview;
use pc_organize::sequence::naming::TemplateError;
use pc_raw_review::export::{ExportFilter, ExportSelection};
use serde::{Deserialize, Serialize};
use tauri::State;

/// Anteprima massima dei nomi nel dialogo.
const PREVIEW_LIMIT: usize = 8;

/// Opzioni scelte nel dialogo di esportazione.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RawReviewExportRequest {
    pub session_id: String,
    pub destination: String,
    /// Valutazione minima (0-5).
    #[serde(default)]
    pub min_rating: u8,
    /// Solo i preset indicati; assente = tutti.
    #[serde(default)]
    pub presets: Option<Vec<String>>,
    /// Modello dei nomi; `null` mantiene i nomi originali.
    #[serde(default)]
    pub template: Option<String>,
    pub catalog_name: String,
    /// Se il catalogo esiste già: `add` aggiunge le foto, `rename` ne crea uno con nome unico.
    #[serde(default)]
    pub existing_catalog: Option<ExistingCatalog>,
    #[serde(default)]
    pub overwrite: bool,
    #[serde(default)]
    pub preset_as_tag: bool,
}

/// Perché l'esportazione non può partire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ExportBlock {
    /// Nessuno scatto da esportare con questi filtri.
    NoPhotos,
    /// Modello dei nomi non valido (vedi `templateErrors`).
    TemplateInvalid,
    /// Nome del catalogo mancante.
    CatalogNameEmpty,
    /// Il nome esiste già e non si è scelto cosa fare (vedi `catalogConflict`).
    CatalogNameTaken,
    /// Il catalogo esistente non accetta foto aggiunte a mano.
    CatalogNotAddable,
    /// Destinazione mancante o non utilizzabile.
    DestinationInvalid,
}

/// Dati del dialogo: conteggi, esclusioni, nomi, conflitti e cosa farà l'import.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RawReviewExportPreflight {
    /// Scatti che verranno esportati con i filtri attuali.
    pub total: u32,
    /// Scatti esportabili per valutazione (indice 0-5), prima della valutazione minima.
    pub by_rating: Vec<u32>,
    /// Esclusi perché senza una variante scelta.
    pub no_choice: u32,
    /// Esclusi perché la variante scelta non è un JPEG.
    pub not_jpeg: u32,
    /// Esclusi perché la variante scelta non esiste più.
    pub missing: u32,
    pub template_errors: Vec<TemplateError>,
    /// Primi nomi in ordine di scatto.
    pub preview: Vec<NamePreview>,
    /// File che hanno già il nome finale nella destinazione (saltati, o sovrascritti).
    pub existing_files: u32,
    pub catalog_conflict: Option<CatalogConflict>,
    /// Cosa farà l'import: nuova sorgente o aggiornamento di una cartella.
    pub import: Option<ImportTarget>,
    pub can_start: bool,
    pub block: Option<ExportBlock>,
}

fn selection_of(state: &AppState, request: &RawReviewExportRequest) -> AppResult<ExportSelection> {
    state.raw_review.export_selection(
        &request.session_id,
        &ExportFilter {
            min_rating: request.min_rating.min(5),
            presets: request.presets.clone(),
        },
    )
}

fn items_of(selection: &ExportSelection) -> Vec<RawExportItem> {
    selection
        .items
        .iter()
        .map(|i| RawExportItem {
            source_path: i.source_path.clone(),
            orig_name: i.orig_name.clone(),
            preset: i.preset.clone(),
            rating: i.rating,
        })
        .collect()
}

fn options_of(request: &RawReviewExportRequest) -> RawExportOptions {
    RawExportOptions {
        destination: request.destination.clone(),
        template: request.template.clone(),
        catalog_name: request.catalog_name.clone(),
        existing_catalog: request.existing_catalog,
        overwrite: request.overwrite,
        preset_as_tag: request.preset_as_tag,
    }
}

fn preflight(
    state: &AppState,
    request: &RawReviewExportRequest,
) -> AppResult<RawReviewExportPreflight> {
    let selection = selection_of(state, request)?;
    let items = items_of(&selection);
    let mut out = RawReviewExportPreflight {
        total: selection.total(),
        by_rating: selection.by_rating.clone(),
        no_choice: selection.no_choice,
        not_jpeg: selection.not_jpeg,
        missing: selection.missing,
        template_errors: Vec::new(),
        preview: Vec::new(),
        existing_files: 0,
        catalog_conflict: None,
        import: None,
        can_start: false,
        block: None,
    };
    let mut block: Option<ExportBlock> = None;

    // Nomi: modello valido, anteprima e collisioni con la destinazione.
    let dest = raw_export::resolve_destination(&request.destination).ok();
    match raw_export::validate_template(request.template.as_deref()) {
        Err(errors) => {
            out.template_errors = errors;
            block = Some(ExportBlock::TemplateInvalid);
        }
        Ok(()) if !items.is_empty() => {
            if let Ok(preview) =
                raw_export::names_preview(&items, request.template.as_deref(), PREVIEW_LIMIT)
            {
                out.preview = preview;
            }
            if let (Some(dest), Ok(names)) = (
                &dest,
                raw_export::final_names(&items, request.template.as_deref()),
            ) {
                out.existing_files = names.iter().filter(|n| dest.join(n).exists()).count() as u32;
            }
        }
        Ok(()) => {}
    }

    // Catalogo: conflitto sul nome.
    if request.catalog_name.trim().is_empty() {
        block = block.or(Some(ExportBlock::CatalogNameEmpty));
    } else {
        out.catalog_conflict = raw_export::catalog_conflict(&state.store, &request.catalog_name)?;
        if let Some(conflict) = &out.catalog_conflict {
            match request.existing_catalog {
                None => block = block.or(Some(ExportBlock::CatalogNameTaken)),
                Some(ExistingCatalog::Add) if !conflict.can_add => {
                    block = block.or(Some(ExportBlock::CatalogNotAddable))
                }
                Some(_) => {}
            }
        }
    }

    // Destinazione: cosa farà l'import.
    match &dest {
        Some(dest) => out.import = Some(raw_export::import_target(&state.store, dest)?),
        None => block = block.or(Some(ExportBlock::DestinationInvalid)),
    }
    if out.total == 0 {
        block = block.or(Some(ExportBlock::NoPhotos));
    }
    out.can_start = block.is_none();
    out.block = block;
    Ok(out)
}

/// Conteggi, esclusioni, anteprima dei nomi e conflitti, mentre si cambiano i filtri.
#[tauri::command]
#[specta::specta]
pub async fn raw_review_export_preflight(
    app: tauri::AppHandle,
    request: RawReviewExportRequest,
) -> AppResult<RawReviewExportPreflight> {
    crate::state::blocking(app, move |_app, state| preflight(state, &request)).await
}

/// Nomi dei primi scatti da esportare mentre si scrive il modello (`null` = nomi originali).
#[tauri::command]
#[specta::specta]
pub async fn raw_review_preview_names(
    app: tauri::AppHandle,
    request: RawReviewExportRequest,
    limit: u32,
) -> AppResult<NamePreviewResult> {
    crate::state::blocking(app, move |_app, state| {
        let items = items_of(&selection_of(state, &request)?);
        let limit = (limit as usize).clamp(1, 50);
        Ok(
            match raw_export::names_preview(&items, request.template.as_deref(), limit) {
                Ok(names) => NamePreviewResult {
                    names,
                    errors: Vec::new(),
                },
                Err(errors) => NamePreviewResult {
                    names: Vec::new(),
                    errors,
                },
            },
        )
    })
    .await
}

/// Avvia l'esportazione come attività (copia, import, catalogo). Il conflitto sul nome del
/// catalogo e il modello dei nomi si controllano prima di copiare: in caso contrario l'avvio
/// fallisce senza toccare il disco. Restituisce l'id dell'attività.
#[tauri::command]
#[specta::specta]
pub async fn start_raw_review_export(
    app: tauri::AppHandle,
    request: RawReviewExportRequest,
) -> AppResult<i64> {
    crate::state::blocking(app, move |_app, state| {
        let selection = selection_of(state, &request)?;
        if selection.items.is_empty() {
            return Err(AppError::invalid_input("nessuno scatto da esportare"));
        }
        // Le scelte vive finiscono anche nei dati dell'app prima che l'attività parta.
        if let Err(e) = state
            .raw_review
            .save(state.paths.root(), &request.session_id)
        {
            tracing::warn!(
                code = e.code.as_str(),
                detail = e.detail.as_str(),
                "sessione non salvata prima dell'esportazione"
            );
        }
        state.jobs.submit(
            raw_export::JOB_KIND,
            raw_export::job_params(&items_of(&selection), &options_of(&request)),
        )
    })
    .await
}

/// Riepilogo di un'esportazione conclusa (esportati, saltati, errori con il motivo, catalogo).
/// `null` finché l'attività non ha finito o se l'app è stata riavviata.
#[tauri::command]
#[specta::specta]
pub async fn raw_review_export_report(
    state: State<'_, AppState>,
    job_id: i64,
) -> AppResult<Option<RawExportReport>> {
    Ok(state.raw_export_reports.get(job_id))
}
