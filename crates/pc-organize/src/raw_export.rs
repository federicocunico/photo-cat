//! Job `raw_review_export`: dalle scelte di una revisione RAW a un nuovo catalogo.
//!
//! Tre fasi di un'unica attività, con avanzamento e annullamento unici:
//!
//! 1. **copia** (`copia`): solo JPEG, byte per byte, in un file temporaneo nascosto poi
//!    rinominato; il file già presente viene saltato (salvo "Sovrascrivi", con backup e
//!    ripristino se qualcosa va storto); i sorgenti non vengono mai modificati; l'errore su un
//!    file non ferma gli altri;
//! 2. **import**: la destinazione dentro una sorgente della libreria aggiorna solo quella
//!    cartella, altrimenti diventa una nuova sorgente (`recursive=false`, `watch=false`). L'import
//!    gira dentro questa attività (stesso avanzamento e stesso annullamento) tramite
//!    [`ImportHook`];
//! 3. **catalogo** (`catalogo`): i media si risolvono *per percorso* (anche i file saltati perché
//!    già presenti), poi in un'unica transazione: catalogo manuale nuovo o esistente, valutazione
//!    della revisione e, se richiesto, tag del set esclusivo "Preset Fujifilm". Una sola voce di
//!    annullamento toglie catalogo, valutazioni e tag, non i file né la sorgente.
//!
//! Se l'attività è annullata dopo la copia, i file copiati restano e il catalogo non viene creato.
//! Il riepilogo (in memoria, per il dialogo) si scrive sempre, anche quando un errore ferma
//! l'attività: in quel caso ne riporta il motivo in `failure`.
//! Gli errori vanno nel registro errori con la fase `esportazione_raw`.

use crate::paths;
use crate::sequence::job::NamePreview;
use crate::sequence::naming::TemplateError;
use crate::sequence::review_names::{
    plan_review_names, preview_review_names, template_errors_message, ReviewNameInput,
};
use pc_core::{AppError, AppResult, ErrorCode};
use pc_jobs::{JobContext, JobFactory, JobPlan, PhaseSpec, RunSummary};
use pc_store::app_store::tags as sets;
use pc_store::bulk::{self, UndoAction};
use pc_store::model::{CatalogKind, MediaId};
use pc_store::sources;
use pc_store::{repo, AppStore, Store};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub const JOB_KIND: &str = "raw_review_export";
pub const PHASE_COPY: &str = "copia";
pub const PHASE_CATALOG: &str = "catalogo";
/// Fase delle voci nel registro errori.
pub const LOG_PHASE: &str = "esportazione_raw";
/// Set esclusivo che raccoglie i preset come tag.
pub const PRESET_SET: &str = "Preset Fujifilm";
/// Modello dei nomi quando si mantengono quelli originali.
const ORIGINAL_TEMPLATE: &str = "{orig}";
/// Tag di un set: tasti 1-9.
const MAX_SET_TAGS: usize = sets::MAX_SET_TAGS;
const CHUNK: usize = 1 << 20;
/// Rapporti conservati in memoria per il riepilogo.
const MAX_REPORTS: usize = 20;

// ------------------------------------------------------------------ tipi

/// Cosa fare se esiste già un catalogo con il nome indicato.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum ExistingCatalog {
    /// Aggiunge le foto al catalogo manuale esistente.
    Add,
    /// Crea un catalogo nuovo con un nome reso unico (`Nome (2)`).
    Rename,
}

/// Un JPEG da esportare (la variante scelta di uno scatto).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct RawExportItem {
    pub source_path: String,
    /// Nome del file di partenza, con estensione.
    pub orig_name: String,
    pub preset: String,
    /// Valutazione della revisione (0-5; 0 non modifica la valutazione in libreria).
    pub rating: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct RawExportOptions {
    pub destination: String,
    /// Modello dei nomi; `None` mantiene i nomi originali.
    #[serde(default)]
    pub template: Option<String>,
    pub catalog_name: String,
    #[serde(default)]
    pub existing_catalog: Option<ExistingCatalog>,
    #[serde(default)]
    pub overwrite: bool,
    #[serde(default)]
    pub preset_as_tag: bool,
}

/// Cosa farà la fase di import con la destinazione.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ImportTarget {
    /// La destinazione non è in libreria: diventa una nuova sorgente (senza sottocartelle né
    /// monitoraggio).
    NewSource { path: String },
    /// La destinazione è dentro una sorgente: si aggiorna solo quella cartella.
    UpdateFolder {
        source_id: i64,
        source_path: String,
        folder: String,
    },
}

/// Come l'import è inserito nell'attività: le sue fasi e il piano per una sorgente.
#[derive(Clone)]
pub struct ImportHook {
    /// Fasi del job di import (incluse quelle degli stadi AI), nell'ordine in cui girano.
    pub phases: Vec<PhaseSpec>,
    /// Fabbrica del piano di import (parametri `{sourceId, folder}`).
    pub factory: JobFactory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum FileStatus {
    Exported,
    Skipped,
    Failed,
}

/// Un file con errore e il suo motivo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct RawExportError {
    /// Nome previsto nella destinazione.
    pub name: String,
    pub source_path: String,
    /// Codice stabile (`errors.<codice>` per il testo localizzato).
    pub code: String,
    pub message_key: String,
    pub reason: String,
}

/// Il catalogo risultante.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct RawExportCatalog {
    pub id: i64,
    pub name: String,
    /// `true` se il catalogo è stato creato, `false` se si è aggiunto a uno esistente.
    pub created: bool,
    /// Foto messe nel catalogo.
    pub photos: u32,
}

/// Riepilogo finale dell'esportazione.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct RawExportReport {
    pub job_id: i64,
    pub exported: u32,
    pub skipped: u32,
    pub failed: u32,
    pub errors: Vec<RawExportError>,
    /// L'attività è stata annullata (i file già copiati restano, senza catalogo).
    pub cancelled: bool,
    pub import: Option<ImportTarget>,
    pub catalog: Option<RawExportCatalog>,
    /// File della destinazione che non si sono trovati in libreria dopo l'import.
    pub unresolved: u32,
    /// Errore che ha fermato l'attività (destinazione non scrivibile, import non riuscito…):
    /// i conteggi dicono fin dove si è arrivati, il catalogo non è stato creato.
    pub failure: Option<RawExportFailure>,
}

/// Motivo per cui l'attività si è fermata prima della fine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct RawExportFailure {
    pub code: String,
    pub message_key: String,
    pub reason: String,
}

impl RawExportReport {
    fn new(job_id: i64) -> Self {
        Self {
            job_id,
            exported: 0,
            skipped: 0,
            failed: 0,
            errors: Vec::new(),
            cancelled: false,
            import: None,
            catalog: None,
            unresolved: 0,
            failure: None,
        }
    }

    fn push_error(&mut self, name: &str, source_path: &str, e: &AppError) {
        self.errors.push(RawExportError {
            name: name.to_string(),
            source_path: source_path.to_string(),
            code: e.code.as_str().to_string(),
            message_key: e.message_key.clone(),
            reason: e.detail.clone(),
        });
    }
}

/// Riepiloghi degli ultimi export, per job (in memoria: servono al dialogo che li mostra).
#[derive(Clone, Default)]
pub struct RawExportReports(Arc<Mutex<BTreeMap<i64, RawExportReport>>>);

impl RawExportReports {
    pub fn get(&self, job_id: i64) -> Option<RawExportReport> {
        self.0.lock().ok()?.get(&job_id).cloned()
    }

    fn put(&self, report: RawExportReport) {
        let Ok(mut map) = self.0.lock() else { return };
        map.insert(report.job_id, report);
        while map.len() > MAX_REPORTS {
            let Some(first) = map.keys().next().copied() else {
                break;
            };
            map.remove(&first);
        }
    }
}

/// Catalogo di destinazione risolto prima della copia.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogTarget {
    Create { name: String },
    Existing { id: i64, name: String },
}

/// Conflitto di nome con un catalogo esistente (per il dialogo).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct CatalogConflict {
    pub id: i64,
    pub name: String,
    /// Se si può aggiungere (catalogo manuale e non di sistema).
    pub can_add: bool,
}

// ------------------------------------------------------------------ percorsi

/// Toglie il prefisso `\\?\` che `canonicalize` aggiunge su Windows (`\\?\UNC\server\x` diventa
/// `\\server\x`).
fn plain(path: PathBuf) -> PathBuf {
    let text = path.display().to_string();
    PathBuf::from(strip_verbatim(&text))
}

fn strip_verbatim(text: &str) -> String {
    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{rest}");
    }
    text.strip_prefix(r"\\?\").unwrap_or(text).to_string()
}

/// Chiave di confronto di due percorsi della libreria: senza prefisso esteso, con separatori
/// uniformi, senza separatore finale e, su Windows, senza distinguere le maiuscole.
fn path_key(path: &str) -> String {
    let p = strip_verbatim(path);
    let p = if cfg!(windows) {
        p.replace('/', "\\").to_lowercase()
    } else {
        p
    };
    p.trim_end_matches(['\\', '/']).to_string()
}

/// La destinazione (anche se non esiste ancora) coincide con la cartella di una delle varianti
/// scelte: copiare lì vorrebbe dire scrivere tra i file della revisione.
fn destination_holds_sources(dest: &Path, items: &[RawExportItem]) -> bool {
    let Ok(dest) = std::fs::canonicalize(dest) else {
        return false;
    };
    let parents: std::collections::HashSet<&Path> = items
        .iter()
        .filter_map(|i| Path::new(&i.source_path).parent())
        .collect();
    parents
        .into_iter()
        .filter_map(|p| std::fs::canonicalize(p).ok())
        .any(|p| paths::same(&p, &dest))
}

/// Destinazione assoluta, normalizzata e senza prefisso esteso (anche se non esiste ancora).
pub fn resolve_destination(destination: &str) -> AppResult<PathBuf> {
    if destination.trim().is_empty() {
        return Err(AppError::invalid_input("destinazione mancante"));
    }
    Ok(plain(paths::resolved(Path::new(destination.trim()))?))
}

/// Cosa farà l'import per questa destinazione (già risolta con [`resolve_destination`]).
pub fn import_target(store: &Store, destination: &Path) -> AppResult<ImportTarget> {
    let dest = destination.display().to_string();
    let list = store.read(repo::source::list)?;
    let best = list
        .iter()
        // Una sorgente senza sottocartelle copre solo la propria cartella.
        .filter(|s| {
            sources::path_in_folder(&dest, &s.path)
                && (s.recursive || sources::path_in_folder(&s.path, &dest))
        })
        .max_by_key(|s| s.path.len());
    Ok(match best {
        Some(source) => {
            let root = source.path.trim_end_matches(['\\', '/']);
            // Si riparte dal percorso della sorgente: i percorsi in libreria hanno le sue
            // maiuscole, e `{cartella}` + nome deve coincidere con quelli dell'import.
            let folder = match dest.get(root.len()..) {
                Some(rest) if sources::path_in_folder(&dest, root) => format!("{root}{rest}"),
                _ => dest.clone(),
            };
            ImportTarget::UpdateFolder {
                source_id: source.id,
                source_path: source.path.clone(),
                folder,
            }
        }
        None => ImportTarget::NewSource { path: dest },
    })
}

/// Cartella (nel formato dei percorsi in libreria) in cui cadranno i file importati.
fn import_folder(target: &ImportTarget) -> PathBuf {
    match target {
        ImportTarget::NewSource { path } => PathBuf::from(path),
        ImportTarget::UpdateFolder { folder, .. } => PathBuf::from(folder),
    }
}

// ------------------------------------------------------------------ nomi

fn name_inputs(items: &[RawExportItem]) -> Vec<ReviewNameInput> {
    items
        .iter()
        .map(|i| {
            ReviewNameInput::from_file(Path::new(&i.source_path), &i.orig_name, Some(&i.preset))
        })
        .collect()
}

/// Nomi finali nello stesso ordine di `items` (modello `None` = nomi originali).
pub fn final_names(
    items: &[RawExportItem],
    template: Option<&str>,
) -> Result<Vec<String>, Vec<TemplateError>> {
    plan_review_names(&name_inputs(items), template.unwrap_or(ORIGINAL_TEMPLATE))
}

/// Anteprima dei primi `limit` nomi in ordine di scatto.
pub fn names_preview(
    items: &[RawExportItem],
    template: Option<&str>,
    limit: usize,
) -> Result<Vec<NamePreview>, Vec<TemplateError>> {
    preview_review_names(
        &name_inputs(items),
        template.unwrap_or(ORIGINAL_TEMPLATE),
        limit,
    )
}

/// Nomi finali (nell'ordine di `items`) e anteprima dei primi `limit`, leggendo i file una volta.
pub fn names_with_preview(
    items: &[RawExportItem],
    template: Option<&str>,
    limit: usize,
) -> Result<(Vec<String>, Vec<NamePreview>), Vec<TemplateError>> {
    let inputs = name_inputs(items);
    let template = template.unwrap_or(ORIGINAL_TEMPLATE);
    Ok((
        plan_review_names(&inputs, template)?,
        preview_review_names(&inputs, template, limit)?,
    ))
}

/// Controlla il modello senza leggere file.
pub fn validate_template(template: Option<&str>) -> Result<(), Vec<TemplateError>> {
    use crate::sequence::naming::{parse_for, NameContext};
    match template {
        Some(t) => parse_for(t, NameContext::Review).map(|_| ()),
        None => Ok(()),
    }
}

// ------------------------------------------------------------------ catalogo

/// Il conflitto di nome, se il catalogo esiste già.
pub fn catalog_conflict(store: &Store, name: &str) -> AppResult<Option<CatalogConflict>> {
    let name = name.trim().to_string();
    Ok(store
        .read(move |c| repo::catalog::by_name(c, &name))?
        .map(|c| CatalogConflict {
            can_add: c.kind == CatalogKind::Manual && c.system.is_none(),
            id: c.id,
            name: c.name,
        }))
}

/// Risolve il catalogo di destinazione: errore se il nome esiste e non si è scelto di
/// aggiungere (o rinominare automaticamente).
pub fn resolve_catalog(
    store: &Store,
    name: &str,
    existing: Option<ExistingCatalog>,
) -> AppResult<CatalogTarget> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::invalid_input("nome del catalogo mancante"));
    }
    let Some(found) = catalog_conflict(store, name)? else {
        return Ok(CatalogTarget::Create {
            name: name.to_string(),
        });
    };
    match existing {
        Some(ExistingCatalog::Add) if found.can_add => Ok(CatalogTarget::Existing {
            id: found.id,
            name: found.name,
        }),
        Some(ExistingCatalog::Add) => Err(AppError::invalid_input(format!(
            "il catalogo «{}» non accetta foto aggiunte a mano",
            found.name
        ))),
        Some(ExistingCatalog::Rename) => {
            let base = name.to_string();
            let unique = store.read(move |c| {
                for n in 2.. {
                    let candidate = format!("{base} ({n})");
                    if repo::catalog::by_name(c, &candidate)?.is_none() {
                        return Ok(candidate);
                    }
                }
                unreachable!()
            })?;
            Ok(CatalogTarget::Create { name: unique })
        }
        None => Err(AppError::invalid_input(format!(
            "esiste già un catalogo chiamato «{}»: aggiungi le foto a quello o scegli un altro nome",
            found.name
        ))),
    }
}

/// Righe da scrivere in libreria per le foto risolte.
#[derive(Debug, Clone)]
struct CatalogRow {
    id: MediaId,
    rating: u8,
    preset: String,
}

#[derive(Debug)]
struct CatalogOutcome {
    id: i64,
    name: String,
    created: bool,
}

/// Tag del set dei preset dopo l'aggiunta, nell'ordine dei tasti, e se il set è esclusivo.
fn planned_set_tags(existing: Option<&sets::TagSet>, presets: &[String]) -> (Vec<String>, bool) {
    let mut tags: Vec<String> = existing.map(|s| s.tag_names()).unwrap_or_default();
    let exclusive = existing.is_none_or(|s| s.exclusive);
    for preset in presets {
        if tags.len() >= MAX_SET_TAGS {
            break;
        }
        if !tags.iter().any(|t| t.eq_ignore_ascii_case(preset)) {
            tags.push(preset.clone());
        }
    }
    (tags, exclusive)
}

/// Fase 3: una transazione sola, con una voce di annullamento sola.
fn apply_catalog(
    store: &Store,
    app: &AppStore,
    target: CatalogTarget,
    rows: Vec<CatalogRow>,
    preset_as_tag: bool,
) -> AppResult<CatalogOutcome> {
    let app = app.clone();
    let description = match &target {
        CatalogTarget::Create { name } | CatalogTarget::Existing { name, .. } => {
            format!("Esporta in {name}")
        }
    };
    Ok(store.write(move |c| {
        let description = description.clone();
        bulk::with_undo_group(c, &description, || {
            let ids: Vec<MediaId> = rows.iter().map(|r| r.id).collect();
            let outcome = match &target {
                CatalogTarget::Create { name } => {
                    let (id, _) = bulk::create_catalog_with_items(c, name, &ids)?;
                    CatalogOutcome {
                        id,
                        name: name.clone(),
                        created: true,
                    }
                }
                CatalogTarget::Existing { id, name } => {
                    bulk::add_to_catalog(c, *id, &ids)?;
                    CatalogOutcome {
                        id: *id,
                        name: name.clone(),
                        created: false,
                    }
                }
            };
            // La valutazione della revisione prevale su quella in libreria; senza stelle in
            // revisione (0) non si cancella una valutazione già data in libreria.
            for stars in 1..=5u8 {
                let group: Vec<MediaId> = rows
                    .iter()
                    .filter(|r| r.rating == stars)
                    .map(|r| r.id)
                    .collect();
                if !group.is_empty() {
                    bulk::set_rating(c, &group, Some(stars))?;
                }
            }
            if preset_as_tag {
                apply_preset_tags(c, &app, &rows)?;
            }
            Ok(outcome)
        })
    })?)
}

fn apply_preset_tags(
    c: &rusqlite::Connection,
    app: &AppStore,
    rows: &[CatalogRow],
) -> pc_store::StoreResult<()> {
    let mut presets: Vec<String> = Vec::new();
    for r in rows {
        if !r.preset.trim().is_empty() && !presets.iter().any(|p| p == &r.preset) {
            presets.push(r.preset.clone());
        }
    }
    if presets.is_empty() {
        return Ok(());
    }
    let existing = app.with(|a| {
        Ok(sets::list_sets(a)?
            .into_iter()
            .find(|s| s.name.eq_ignore_ascii_case(PRESET_SET)))
    })?;
    let (set_tags, exclusive) = planned_set_tags(existing.as_ref(), &presets);
    // Tag sulle foto: gli altri preset del set (esclusivo) si tolgono dalle stesse foto.
    for preset in &presets {
        let ids: Vec<MediaId> = rows
            .iter()
            .filter(|r| &r.preset == preset)
            .map(|r| r.id)
            .collect();
        let in_set = set_tags.iter().any(|t| t.eq_ignore_ascii_case(preset));
        let siblings: Vec<String> = if exclusive && in_set {
            set_tags
                .iter()
                .filter(|t| !t.eq_ignore_ascii_case(preset))
                .cloned()
                .collect()
        } else {
            Vec::new()
        };
        bulk::add_tag(c, preset, &ids, &siblings)?;
    }
    // Set e tag in app.db per ultimi: se il catalogo fallisce prima, app.db non è toccato.
    let names: Vec<&str> = presets.iter().map(String::as_str).collect();
    let payload = app.tx(|a| {
        let (set_id, created) = match &existing {
            Some(s) => (s.id, Vec::new()),
            None => {
                let id = sets::create_set(a, PRESET_SET, &[])?;
                sets::set_exclusive(a, id, true)?;
                (id, vec![id])
            }
        };
        let payload = sets::snapshot_created(a, &names, &created)?;
        for preset in &presets {
            // Set pieno (9 tasti): il tag resta sulle foto senza entrare nel set.
            match sets::add_to_set(a, set_id, preset) {
                Ok(_) | Err(pc_store::StoreError::Invalid(_)) => {}
                Err(e) => return Err(e),
            }
        }
        Ok(payload)
    })?;
    bulk::record_undo(c, "Preset Fujifilm", &UndoAction::AppStore { payload })
}

// ------------------------------------------------------------------ copia

#[derive(Debug)]
enum CopyOutcome {
    Exported,
    Skipped,
    Cancelled,
    Failed(AppError),
}

fn is_jpeg_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "jpg" | "jpeg"))
}

fn hidden_sibling(target: &Path, kind: &str) -> PathBuf {
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    target.with_file_name(format!(".{name}.photo-cat-{kind}.tmp"))
}

/// Copia `source` in `target` byte per byte: file temporaneo nascosto, `sync`, rinomina. Con
/// `overwrite` il file esistente passa prima in un backup, ripristinato se la rinomina fallisce.
/// Il nome finale non esiste mai con un contenuto parziale.
fn copy_one(
    source: &Path,
    target: &Path,
    overwrite: bool,
    cancelled: &dyn Fn() -> bool,
) -> CopyOutcome {
    // Whitelist: solo JPEG, qualunque cosa chieda il chiamante.
    if !is_jpeg_path(source) || !is_jpeg_path(target) {
        return CopyOutcome::Failed(AppError::new(
            ErrorCode::UnsupportedFormat,
            "si esportano solo file JPEG",
        ));
    }
    let meta = match std::fs::metadata(source) {
        Ok(m) if m.is_file() => m,
        Ok(_) => {
            return CopyOutcome::Failed(AppError::new(
                ErrorCode::NotFound,
                "il file scelto non è un file",
            ))
        }
        Err(e) => return CopyOutcome::Failed(e.into()),
    };
    let backup = hidden_sibling(target, "old");
    // Un'interruzione brusca durante una sostituzione può aver lasciato il file precedente solo
    // nel backup: torna al suo nome prima di qualsiasi altra cosa.
    if !target.exists() && backup.is_file() {
        let _ = std::fs::rename(&backup, target);
    }
    let had_target = target.exists();
    if had_target {
        if let (Ok(a), Ok(b)) = (std::fs::canonicalize(source), std::fs::canonicalize(target)) {
            if paths::same(&a, &b) {
                return CopyOutcome::Failed(AppError::invalid_input(
                    "la destinazione coincide con il file di partenza",
                ));
            }
        }
        if !overwrite {
            return CopyOutcome::Skipped;
        }
        if !target.is_file() {
            return CopyOutcome::Failed(AppError::invalid_input(
                "nella destinazione c'è una cartella con lo stesso nome",
            ));
        }
    }
    let tmp = hidden_sibling(target, "export");
    let _ = std::fs::remove_file(&tmp);
    match write_temp(source, &tmp, meta.len(), cancelled) {
        Ok(true) => {}
        Ok(false) => {
            let _ = std::fs::remove_file(&tmp);
            return CopyOutcome::Cancelled;
        }
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            return CopyOutcome::Failed(e);
        }
    }
    if had_target {
        remove_leftover(&backup);
        // Su Windows fallisce se il file è aperto da un altro programma: resta com'è.
        if let Err(e) = std::fs::rename(target, &backup) {
            let _ = std::fs::remove_file(&tmp);
            return CopyOutcome::Failed(e.into());
        }
    }
    if let Err(e) = std::fs::rename(&tmp, target) {
        if had_target {
            let _ = std::fs::rename(&backup, target);
        }
        let _ = std::fs::remove_file(&tmp);
        return CopyOutcome::Failed(e.into());
    }
    if had_target {
        remove_leftover(&backup);
    }
    CopyOutcome::Exported
}

/// Elimina un temporaneo o un backup, anche se è in sola lettura (il file sostituito lo era).
fn remove_leftover(path: &Path) {
    if std::fs::remove_file(path).is_ok() || !path.is_file() {
        return;
    }
    if let Ok(meta) = std::fs::metadata(path) {
        let mut perms = meta.permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        perms.set_readonly(false);
        let _ = std::fs::set_permissions(path, perms);
    }
    let _ = std::fs::remove_file(path);
}

/// Porta su disco le rinomine della cartella (best effort, dove il sistema lo consente).
fn sync_dir(dir: &Path) {
    #[cfg(unix)]
    if let Ok(d) = std::fs::File::open(dir) {
        let _ = d.sync_all();
    }
    #[cfg(not(unix))]
    let _ = dir;
}

/// Scrive `tmp` come copia di `source`; `Ok(false)` se annullata a metà.
fn write_temp(
    source: &Path,
    tmp: &Path,
    expected: u64,
    cancelled: &dyn Fn() -> bool,
) -> AppResult<bool> {
    let mut input = std::fs::File::open(source)?;
    let mut output = std::fs::File::create(tmp)?;
    let mut buffer = vec![0u8; CHUNK];
    let mut total = 0u64;
    loop {
        if cancelled() {
            return Ok(false);
        }
        let n = input.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        output.write_all(&buffer[..n])?;
        total += n as u64;
    }
    if total != expected {
        return Err(AppError::new(
            ErrorCode::ChecksumMismatch,
            format!("copiati {total} byte su {expected}"),
        ));
    }
    // Stessa data di modifica dell'originale (best effort), poi su disco prima della rinomina.
    if let Ok(modified) = input.metadata().and_then(|m| m.modified()) {
        let _ = output.set_modified(modified);
    }
    output.sync_all()?;
    Ok(true)
}

fn log_error(job_id: i64, path: &Path, e: &AppError) {
    tracing::error!(
        code = e.code.as_str(),
        job_id,
        phase = LOG_PHASE,
        path = %path.display(),
        detail = e.detail.as_str(),
        "{}",
        e.message_key
    );
}

// ------------------------------------------------------------------ job

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Params {
    items: Vec<RawExportItem>,
    options: RawExportOptions,
}

/// Parametri del job per `submit`.
pub fn job_params(items: &[RawExportItem], options: &RawExportOptions) -> serde_json::Value {
    serde_json::json!({ "items": items, "options": options })
}

/// Fasi del job: copia, quelle dell'import, catalogo.
fn phases(hook: &ImportHook) -> Vec<PhaseSpec> {
    let mut all = vec![PhaseSpec::new(PHASE_COPY, 4.0)];
    all.extend(hook.phases.iter().cloned());
    all.push(PhaseSpec::new(PHASE_CATALOG, 0.5));
    all
}

/// Controlli che devono passare prima di copiare qualsiasi file.
fn validate(store: &Store, params: &Params) -> AppResult<CatalogTarget> {
    if params.items.is_empty() {
        return Err(AppError::invalid_input("nessuno scatto da esportare"));
    }
    if let Err(errors) = validate_template(params.options.template.as_deref()) {
        return Err(AppError::invalid_input(template_errors_message(&errors)));
    }
    let dest = resolve_destination(&params.options.destination)?;
    if destination_holds_sources(&dest, &params.items) {
        return Err(AppError::invalid_input(
            "la destinazione è la cartella delle varianti scelte: scegline un'altra",
        ));
    }
    resolve_catalog(
        store,
        &params.options.catalog_name,
        params.options.existing_catalog,
    )
}

pub fn plan(
    store: Store,
    app: AppStore,
    hook: ImportHook,
    reports: RawExportReports,
    params: &serde_json::Value,
) -> AppResult<JobPlan> {
    let params: Params = serde_json::from_value(params.clone())?;
    // Nome del catalogo e modello si controllano subito: il conflitto si risolve PRIMA della copia.
    validate(&store, &params)?;
    Ok(JobPlan {
        phases: phases(&hook),
        min_memory_mb: 64,
        run: Box::new(move |ctx| {
            // Il riepilogo si scrive sempre, anche se l'attività si ferma per un errore: il
            // dialogo che lo aspetta mostra fin dove si è arrivati e perché.
            let mut report = RawExportReport::new(ctx.job_id);
            let result = run(&store, &app, &hook, &params, &ctx, &mut report);
            match &result {
                Ok(summary) => report.cancelled = summary.cancelled,
                Err(e) => {
                    log_error(ctx.job_id, Path::new(&params.options.destination), e);
                    report.failure = Some(RawExportFailure {
                        code: e.code.as_str().to_string(),
                        message_key: e.message_key.clone(),
                        reason: e.detail.clone(),
                    });
                }
            }
            reports.put(report);
            result
        }),
    })
}

/// Media in libreria per percorso: prima il confronto esatto, poi (una volta sola) quello che
/// ignora prefisso esteso, separatori e, su Windows, maiuscole. Su Windows un file saltato perché
/// già presente come `dscf0001.jpg` ha in libreria il nome scritto sul disco, non `DSCF0001.JPG`.
struct MediaByPath<'a> {
    store: &'a Store,
    source_id: i64,
    folder: String,
    loose: Option<std::collections::HashMap<String, MediaId>>,
}

impl MediaByPath<'_> {
    fn find(&mut self, path: &str) -> AppResult<Option<MediaId>> {
        let exact = {
            let path = path.to_string();
            self.store
                .read(move |c| repo::media::get_by_path(c, &path))?
        };
        if let Some(media) = exact {
            return Ok(Some(media.id));
        }
        if self.loose.is_none() {
            let (source_id, folder) = (self.source_id, path_key(&self.folder));
            let all = self
                .store
                .read(move |c| repo::media::by_source(c, source_id))?;
            self.loose = Some(
                all.into_iter()
                    .filter_map(|m| {
                        let key = path_key(&m.path);
                        sources::path_in_folder(&key, &folder).then_some((key, m.id))
                    })
                    .collect(),
            );
        }
        Ok(self
            .loose
            .as_ref()
            .and_then(|m| m.get(&path_key(path)).copied()))
    }
}

fn run(
    store: &Store,
    app: &AppStore,
    hook: &ImportHook,
    params: &Params,
    ctx: &JobContext,
    report: &mut RawExportReport,
) -> AppResult<RunSummary> {
    let Params { items, options } = params;
    let job_id = ctx.job_id;
    let cancelled = |failed: i64| {
        Ok(RunSummary {
            cancelled: true,
            failed,
        })
    };
    // Di nuovo al via: il nome del catalogo può essere stato preso mentre l'attività era in coda.
    let target = validate(store, params)?;
    let dest = resolve_destination(&options.destination)?;
    std::fs::create_dir_all(&dest)?;
    let dest = plain(std::fs::canonicalize(&dest)?);
    let names = final_names(items, options.template.as_deref())
        .map_err(|e| AppError::invalid_input(template_errors_message(&e)))?;

    // --- 1. copia
    let copy = ctx.progress.phase(PHASE_COPY);
    copy.set_total(items.len() as i64);
    copy.start();
    // (indice dell'elemento) dei file presenti nella destinazione: copiati o già lì.
    let mut present: Vec<usize> = Vec::new();
    for (i, item) in items.iter().enumerate() {
        if !ctx.control.checkpoint() {
            return cancelled(i64::from(report.failed));
        }
        let source = Path::new(&item.source_path);
        let target_path = dest.join(&names[i]);
        let cancel_flag = || ctx.control.is_cancelled();
        match copy_one(source, &target_path, options.overwrite, &cancel_flag) {
            CopyOutcome::Exported => {
                report.exported += 1;
                present.push(i);
                copy.inc_done(1);
            }
            CopyOutcome::Skipped => {
                report.skipped += 1;
                present.push(i);
                copy.inc_done(1);
            }
            CopyOutcome::Cancelled => return cancelled(i64::from(report.failed)),
            CopyOutcome::Failed(e) => {
                report.failed += 1;
                copy.inc_failed(1);
                log_error(job_id, source, &e);
                report.push_error(&names[i], &item.source_path, &e);
            }
        }
    }
    sync_dir(&dest);
    copy.finish();
    let mut failed = i64::from(report.failed);

    if ctx.control.is_cancelled() {
        return cancelled(failed);
    }
    if present.is_empty() {
        // Niente nella destinazione: né import né catalogo.
        for p in &hook.phases {
            let c = ctx.progress.phase(&p.name);
            c.set_total(0);
            c.finish();
        }
        let c = ctx.progress.phase(PHASE_CATALOG);
        c.set_total(0);
        c.finish();
        return Ok(RunSummary {
            cancelled: false,
            failed,
        });
    }

    // --- 2. import (dentro questa attività: stesso avanzamento e stesso annullamento)
    let import = import_target(store, &dest)?;
    report.import = Some(import.clone());
    let source_id = match &import {
        ImportTarget::UpdateFolder { source_id, .. } => *source_id,
        ImportTarget::NewSource { path } => {
            let path = path.clone();
            store.write(move |c| {
                // Un'altra attività potrebbe aver aggiunto nel frattempo una sorgente che la copre.
                if let Some(id) = sources::source_containing(c, &path)? {
                    return Ok(id);
                }
                let id = repo::source::insert(
                    c,
                    &pc_store::model::NewSource {
                        path,
                        recursive: false,
                        include_formats: Vec::new(),
                        ignore_patterns: Vec::new(),
                        watch: false,
                    },
                )?;
                sources::adopt_strays(c, id)?;
                Ok(id)
            })?
        }
    };
    let folder = import_folder(&import);
    let sub_params = serde_json::json!({
        "sourceId": source_id,
        "folder": folder.display().to_string(),
    });
    let sub = (hook.factory)(&sub_params)?;
    let sub_followup = Arc::new(parking_lot::Mutex::new(None));
    let sub_ctx = JobContext {
        job_id,
        params: sub_params,
        progress: ctx.progress.clone(),
        control: ctx.control.clone(),
        journal: ctx.journal.clone(),
        store: ctx.store.clone(),
        governor: ctx.governor.clone(),
        followup: sub_followup.clone(),
    };
    let imported = (sub.run)(sub_ctx)?;
    failed += imported.failed;
    if imported.cancelled || ctx.control.is_cancelled() {
        return cancelled(failed);
    }
    // Il seguito dell'import (analisi AI, foto simili) parte alla fine di questa attività.
    *ctx.followup.lock() = sub_followup.lock().take();

    // --- 3. catalogo: tutti i file attesi nella destinazione, anche quelli saltati
    let cat = ctx.progress.phase(PHASE_CATALOG);
    cat.set_total(1);
    cat.start();
    let mut lookup = MediaByPath {
        store,
        source_id,
        folder: folder.display().to_string(),
        loose: None,
    };
    let mut rows: Vec<CatalogRow> = Vec::new();
    for &i in &present {
        let path = folder.join(&names[i]).display().to_string();
        match lookup.find(&path)? {
            Some(id) => {
                if !rows.iter().any(|r| r.id == id) {
                    rows.push(CatalogRow {
                        id,
                        rating: items[i].rating,
                        preset: items[i].preset.clone(),
                    });
                }
            }
            None => {
                report.unresolved += 1;
                failed += 1;
                let e = AppError::new(
                    ErrorCode::NotFound,
                    "il file non è in libreria dopo l'importazione",
                );
                log_error(job_id, Path::new(&path), &e);
                report.push_error(&names[i], &items[i].source_path, &e);
            }
        }
    }
    // Ultimo punto in cui annullare: la scrittura del catalogo è una transazione sola.
    if !ctx.control.checkpoint() {
        return cancelled(failed);
    }
    if rows.is_empty() {
        cat.inc_failed(1);
        cat.finish();
        return Ok(RunSummary {
            cancelled: false,
            failed,
        });
    }
    let photos = rows.len() as u32;
    let outcome = apply_catalog(store, app, target, rows, options.preset_as_tag)?;
    report.catalog = Some(RawExportCatalog {
        id: outcome.id,
        name: outcome.name,
        created: outcome.created,
        photos,
    });
    cat.inc_done(1);
    cat.finish();
    Ok(RunSummary {
        cancelled: false,
        failed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn never() -> bool {
        false
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    }

    #[test]
    fn only_jpeg_is_copied() {
        let tmp = tempfile::tempdir().unwrap();
        for name in ["a.RAF", "a.tif", "a.tiff", "a.png", "a"] {
            let src = tmp.path().join(name);
            std::fs::write(&src, b"x").unwrap();
            let out = tmp.path().join("out").join(name);
            std::fs::create_dir_all(out.parent().unwrap()).unwrap();
            assert!(
                matches!(copy_one(&src, &out, false, &never), CopyOutcome::Failed(_)),
                "{name}"
            );
            assert!(!out.exists());
        }
        let src = tmp.path().join("ok.JPEG");
        std::fs::write(&src, b"jpeg").unwrap();
        // Anche un nome di destinazione non JPEG è rifiutato.
        let bad = tmp.path().join("out").join("ok.raf");
        assert!(matches!(
            copy_one(&src, &bad, false, &never),
            CopyOutcome::Failed(_)
        ));
        let good = tmp.path().join("out").join("ok.jpeg");
        assert!(matches!(
            copy_one(&src, &good, false, &never),
            CopyOutcome::Exported
        ));
        assert_eq!(std::fs::read(&good).unwrap(), b"jpeg");
    }

    #[test]
    fn interruption_leaves_no_partial_file_with_the_final_name() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("big.jpg");
        std::fs::write(&src, vec![7u8; CHUNK * 3 + 5]).unwrap();
        let out = tmp.path().join("out");
        std::fs::create_dir_all(&out).unwrap();
        let target = out.join("big.jpg");
        // Annullamento dopo il primo blocco copiato: file temporaneo parziale già scritto.
        let calls = Cell::new(0);
        let cancel = || {
            calls.set(calls.get() + 1);
            calls.get() > 2
        };
        assert!(matches!(
            copy_one(&src, &target, false, &cancel),
            CopyOutcome::Cancelled
        ));
        assert!(!target.exists());
        assert!(names(&out).is_empty(), "{:?}", names(&out));
        // Residuo di un'interruzione brusca (processo ucciso): mai col nome finale, e ripulito
        // alla prova successiva.
        std::fs::write(hidden_sibling(&target, "export"), b"parziale").unwrap();
        assert!(!target.exists());
        assert!(matches!(
            copy_one(&src, &target, false, &never),
            CopyOutcome::Exported
        ));
        assert_eq!(names(&out), vec!["big.jpg"]);
        assert_eq!(std::fs::read(&target).unwrap().len(), CHUNK * 3 + 5);
    }

    #[test]
    fn existing_target_is_skipped_or_replaced_with_backup_cleanup() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("new.jpg");
        std::fs::write(&src, b"nuovo").unwrap();
        let out = tmp.path().join("out");
        std::fs::create_dir_all(&out).unwrap();
        let target = out.join("new.jpg");
        std::fs::write(&target, b"vecchio").unwrap();
        assert!(matches!(
            copy_one(&src, &target, false, &never),
            CopyOutcome::Skipped
        ));
        assert_eq!(std::fs::read(&target).unwrap(), b"vecchio");
        assert!(matches!(
            copy_one(&src, &target, true, &never),
            CopyOutcome::Exported
        ));
        assert_eq!(std::fs::read(&target).unwrap(), b"nuovo");
        assert_eq!(names(&out), vec!["new.jpg"], "nessun backup rimasto");
        // La sorgente non coincide mai con la destinazione: nemmeno con "Sovrascrivi".
        assert!(matches!(
            copy_one(&target, &target, true, &never),
            CopyOutcome::Failed(_)
        ));
        assert_eq!(std::fs::read(&target).unwrap(), b"nuovo");
    }

    #[test]
    fn failed_replacement_restores_the_previous_file() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("new.jpg");
        std::fs::write(&src, b"nuovo").unwrap();
        let out = tmp.path().join("out");
        std::fs::create_dir_all(&out).unwrap();
        let target = out.join("new.jpg");
        std::fs::write(&target, b"vecchio").unwrap();
        // Il backup non si può creare (c'è una cartella al suo posto): il file resta com'era.
        std::fs::create_dir_all(hidden_sibling(&target, "old").join("x")).unwrap();
        assert!(matches!(
            copy_one(&src, &target, true, &never),
            CopyOutcome::Failed(_)
        ));
        assert_eq!(std::fs::read(&target).unwrap(), b"vecchio");
        assert!(!hidden_sibling(&target, "export").exists());
    }

    #[test]
    fn backup_left_by_a_crash_during_replacement_is_restored() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("new.jpg");
        std::fs::write(&src, b"nuovo").unwrap();
        let out = tmp.path().join("out");
        std::fs::create_dir_all(&out).unwrap();
        let target = out.join("new.jpg");
        // Processo ucciso tra "file vecchio nel backup" e "temporaneo al nome finale".
        std::fs::write(hidden_sibling(&target, "old"), b"vecchio").unwrap();
        std::fs::write(hidden_sibling(&target, "export"), b"nu").unwrap();
        // Senza "Sovrascrivi" il file precedente torna al suo posto e viene saltato.
        assert!(matches!(
            copy_one(&src, &target, false, &never),
            CopyOutcome::Skipped
        ));
        assert_eq!(std::fs::read(&target).unwrap(), b"vecchio");
        assert!(!hidden_sibling(&target, "old").exists());
    }

    #[test]
    fn read_only_file_is_replaced_without_leftovers() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("new.jpg");
        std::fs::write(&src, b"nuovo").unwrap();
        let out = tmp.path().join("out");
        std::fs::create_dir_all(&out).unwrap();
        let target = out.join("new.jpg");
        std::fs::write(&target, b"vecchio").unwrap();
        let mut perms = std::fs::metadata(&target).unwrap().permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(&target, perms).unwrap();
        assert!(matches!(
            copy_one(&src, &target, true, &never),
            CopyOutcome::Exported
        ));
        assert_eq!(std::fs::read(&target).unwrap(), b"nuovo");
        assert_eq!(names(&out), vec!["new.jpg"], "nessun backup rimasto");
    }

    #[test]
    fn path_keys_ignore_verbatim_prefix_separators_and_case_on_windows() {
        assert_eq!(strip_verbatim(r"\\?\D:\Foto"), r"D:\Foto");
        assert_eq!(strip_verbatim(r"\\?\UNC\nas\foto"), r"\\nas\foto");
        assert_eq!(strip_verbatim("/home/foto"), "/home/foto");
        if cfg!(windows) {
            assert_eq!(
                path_key(r"\\?\D:\Foto\Dolomiti\DSCF0001.JPG"),
                path_key("d:/foto/dolomiti/dscf0001.jpg")
            );
        } else {
            assert_ne!(path_key("/foto/A.jpg"), path_key("/foto/a.jpg"));
            assert_eq!(path_key("/foto/a/"), path_key("/foto/a"));
        }
    }

    #[test]
    fn destination_equal_to_a_variant_folder_is_detected() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("C1");
        std::fs::create_dir_all(&dir).unwrap();
        let item = RawExportItem {
            source_path: dir.join("a.jpg").display().to_string(),
            orig_name: "a.jpg".into(),
            preset: "C1".into(),
            rating: 0,
        };
        assert!(destination_holds_sources(&dir, std::slice::from_ref(&item)));
        assert!(!destination_holds_sources(
            &tmp.path().join("altrove"),
            std::slice::from_ref(&item)
        ));
        assert!(!destination_holds_sources(tmp.path(), &[item]));
    }

    #[test]
    fn set_plan_respects_the_nine_tag_limit_and_exclusivity() {
        let presets: Vec<String> = (1..=11).map(|i| format!("P{i}")).collect();
        let (tags, exclusive) = planned_set_tags(None, &presets);
        assert_eq!(tags.len(), 9);
        assert!(exclusive);
    }
}
