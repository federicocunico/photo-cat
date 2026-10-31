//! Job di import di una sorgente: scansione → metadati → miniature → [stadi
//! aggiuntivi, es. AI] → scrittura nel catalogo.
//!
//! È incrementale: i file con stessa dimensione e data di modifica già in
//! libreria vengono saltati; i file scomparsi diventano "mancanti"; un file
//! spostato viene riconosciuto dall'impronta e ricollegato.

use crate::formats::{support, Support};
use crate::hash::{content_hash, quick_hash};
use crate::metadata::{read_exif, read_tech, resolve_dates, ExifMeta};
use crate::scan::{FoundFile, ScanRules};
use crate::thumb::{apply_orientation, decode_reduced, fit, Decoded, ThumbCache, THUMB_SIZE};
use parking_lot::Mutex;
use pc_core::{AppError, AppResult, ErrorCode};
use pc_governor::MemoryGovernor;
use pc_jobs::{JobContext, JobPlan, PhaseSpec, Pipeline, RunSummary, StageCtx, StageOutcome};
use pc_store::model::{MediaDates, MediaId, MediaItem, MediaKind, MediaStatus, NewMedia, TechInfo};
use pc_store::{repo, Store};
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

pub const JOB_KIND: &str = "import";
pub const PHASE_SCAN: &str = "scansione";
pub const PHASE_META: &str = "metadati";
pub const PHASE_THUMB: &str = "miniature";
pub const PHASE_WRITE: &str = "scrittura";

/// Lato lungo dell'immagine decodificata conservata per gli stadi successivi (AI).
pub const ANALYSIS_SIZE: u32 = 640;

/// Un file in transito nella pipeline di import.
pub struct ImportItem {
    pub found: FoundFile,
    pub source_id: i64,
    pub ext: String,
    pub existing: Option<MediaItem>,
    /// Elemento mancante con la stessa impronta: il file è stato spostato.
    pub moved_from: Option<MediaId>,
    pub quick_hash: Option<String>,
    pub content_hash: Option<String>,
    pub tech: Option<TechInfo>,
    pub exif: Option<ExifMeta>,
    pub dates: Option<MediaDates>,
    pub thumb_key: Option<String>,
    /// Immagine ridotta e orientata (con permesso di memoria), per gli stadi AI.
    pub decoded: Option<Decoded>,
    /// Risultati degli stadi aggiuntivi, scritti dallo stadio finale.
    pub extra: Vec<Box<dyn ExtraWrite>>,
}

impl ImportItem {
    pub fn path_str(&self) -> String {
        self.found.path.display().to_string()
    }
}

/// Scrittura nel catalogo prodotta da uno stadio aggiuntivo.
pub trait ExtraWrite: Send {
    fn write(&self, c: &rusqlite::Connection, media_id: MediaId) -> pc_store::StoreResult<()>;
}

/// Stadio aggiuntivo inserito tra miniature e scrittura (es. classificazione AI).
pub trait ImportStage: Send + Sync {
    fn phase(&self) -> PhaseSpec;
    fn workers(&self) -> usize;
    /// Carica le risorse sul worker, prima che la pipeline occupi memoria.
    fn prepare(&self) -> AppResult<()> {
        Ok(())
    }
    /// Lo stadio usa l'immagine decodificata (altrimenti non serve conservarla).
    fn needs_image(&self) -> bool {
        true
    }
    fn process(&self, item: &mut ImportItem, ctx: &StageCtx) -> StageOutcome;
}

/// Stadi aggiuntivi decisi all'avvio di un import e job da accodare al termine.
#[derive(Default)]
pub struct StagePlan {
    pub stages: Vec<Arc<dyn ImportStage>>,
    pub followup: Option<(String, serde_json::Value)>,
}

#[derive(Clone)]
pub struct StageFactory {
    pub phases: Vec<PhaseSpec>,
    pub build: Arc<dyn Fn(i64) -> StagePlan + Send + Sync>,
}

#[derive(Clone)]
pub struct ImportDeps {
    pub store: Store,
    pub thumbs: ThumbCache,
    pub governor: MemoryGovernor,
    pub extra_stages: Vec<Arc<dyn ImportStage>>,
    /// Stadi calcolati a ogni import (modelli e memoria disponibili cambiano nel tempo).
    pub stage_factory: Option<StageFactory>,
    /// Worker per metadati e miniature (dal governor).
    pub io_workers: usize,
    pub decode_workers: usize,
}

fn warn_item(job_id: i64, phase: &str, path: &str, e: &AppError) {
    tracing::warn!(
        code = e.code.as_str(),
        job_id,
        phase,
        path,
        detail = e.detail.as_str(),
        "{}",
        e.message_key
    );
}

/// Fase metadati: salta i file invariati, riconosce gli spostati, legge EXIF, date e impronte.
fn stage_metadata(
    store: &Store,
    item: &mut ImportItem,
    ctx: &StageCtx,
    force: bool,
) -> StageOutcome {
    let path = item.path_str();
    item.existing = match store.read(|c| repo::media::get_by_path(c, &path)) {
        Ok(e) => e,
        Err(e) => return StageOutcome::Fail(e.into()),
    };
    if let Some(ex) = &item.existing {
        let unchanged = ex.status == MediaStatus::Ok
            && ex.size == item.found.size as i64
            && ex.mtime == item.found.mtime;
        if unchanged && !force {
            return StageOutcome::Skip;
        }
    }
    let qh = match quick_hash(&item.found.path) {
        Ok(h) => h,
        Err(e) => return StageOutcome::Fail(e),
    };
    if item.existing.is_none() {
        // Spostato: stessa impronta e dimensione di un elemento il cui file non esiste più.
        let size = item.found.size as i64;
        if let Ok(candidates) = store.read(|c| repo::media::by_quick_hash(c, &qh)) {
            item.moved_from = candidates
                .into_iter()
                .find(|m| m.size == size && !Path::new(&m.path).exists())
                .map(|m| m.id);
        }
    }
    item.quick_hash = Some(qh);
    item.exif = match read_exif(&item.found.path) {
        Ok(e) => e,
        Err(e) if e.code == ErrorCode::CorruptExif => {
            warn_item(ctx.job_id, PHASE_META, &path, &e);
            None
        }
        Err(e) => return StageOutcome::Fail(e),
    };
    item.tech = match read_tech(&item.found.path, &item.ext, item.exif.as_ref()) {
        Ok(t) => Some(t),
        Err(e) => return StageOutcome::Fail(e),
    };
    match std::fs::metadata(&item.found.path) {
        Ok(m) => item.dates = Some(resolve_dates(&m, item.exif.as_ref())),
        Err(e) => return StageOutcome::Fail(e.into()),
    }
    match content_hash(&item.found.path) {
        Ok(h) => item.content_hash = Some(h),
        Err(e) => return StageOutcome::Fail(e),
    }
    StageOutcome::Continue
}

/// Fase miniature: un'unica decodifica ridotta alimenta miniatura e (se servono) gli stadi AI.
fn stage_thumbnail(
    deps: &ImportDeps,
    keep_decoded: bool,
    item: &mut ImportItem,
    ctx: &StageCtx,
) -> StageOutcome {
    let Some(key) = item.content_hash.clone() else {
        return StageOutcome::Continue;
    };
    let have_thumb = deps.thumbs.contains(&key);
    if have_thumb && !keep_decoded {
        item.thumb_key = Some(key);
        return StageOutcome::Continue;
    }
    if let Some(Support::MetadataOnly(kind)) = support(&item.ext) {
        let supported_heif = kind == crate::formats::MetaOnlyKind::Heif && cfg!(feature = "heif");
        if !supported_heif && crate::thumb::embedded_preview(&item.found.path).is_none() {
            // Avviso, non errore: il file entra in libreria con i soli metadati.
            let e = AppError::new(
                ErrorCode::UnsupportedFormat,
                format!(
                    "{}: importato con i soli metadati",
                    item.ext.to_ascii_uppercase()
                ),
            );
            warn_item(ctx.job_id, PHASE_THUMB, &item.path_str(), &e);
            return StageOutcome::Continue;
        }
    }
    // libheif applica già le trasformazioni del contenitore HEIF.
    let native_orientation = matches!(
        support(&item.ext),
        Some(Support::MetadataOnly(crate::formats::MetaOnlyKind::Heif))
    ) && cfg!(feature = "heif");
    let orientation = if native_orientation {
        None
    } else {
        item.exif.as_ref().and_then(|e| e.orientation)
    };
    let target = if keep_decoded {
        ANALYSIS_SIZE.max(THUMB_SIZE)
    } else {
        THUMB_SIZE
    };
    let decoded = match decode_reduced(&item.found.path, &item.ext, target, &deps.governor) {
        Ok(d) => d,
        Err(e) if matches!(support(&item.ext), Some(Support::MetadataOnly(_))) => {
            warn_item(ctx.job_id, PHASE_THUMB, &item.path_str(), &e);
            return StageOutcome::Continue;
        }
        Err(e) => return StageOutcome::ContinueWithError(e),
    };
    let Decoded {
        image,
        source_size,
        mut permit,
    } = decoded;
    let oriented = apply_orientation(image, orientation);
    if !have_thumb {
        if let Err(e) = deps.thumbs.store(&key, &fit(&oriented, THUMB_SIZE)) {
            return StageOutcome::ContinueWithError(e);
        }
    }
    item.thumb_key = Some(key);
    if keep_decoded {
        let image = fit(&oriented, ANALYSIS_SIZE);
        drop(oriented);
        permit.shrink_to((image.as_raw().len() as u64).div_ceil(1024 * 1024).max(1));
        item.decoded = Some(Decoded {
            image,
            source_size,
            permit,
        });
    }
    StageOutcome::Continue
}

/// Fase scrittura: un'unica operazione (in coda al writer) per tutti i dati del file.
fn stage_write(store: &Store, item: &mut ImportItem) -> StageOutcome {
    item.decoded = None; // libera la memoria dell'immagine
    let new = NewMedia {
        kind: MediaKind::Photo,
        source_id: Some(item.source_id),
        path: item.path_str(),
        name: item
            .found
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        ext: item.ext.clone(),
        size: item.found.size as i64,
        mtime: item.found.mtime,
        quick_hash: item.quick_hash.clone(),
        content_hash: item.content_hash.clone(),
    };
    let moved_from = item.moved_from;
    let tech = item.tech.take();
    let exif = item.exif.take();
    let dates = item.dates.take();
    let thumb = item.thumb_key.take();
    let extra = std::mem::take(&mut item.extra);
    let res = store.submit(move |c| {
        let id = match moved_from {
            Some(old) => {
                repo::media::relink(c, old, &new.path, &new.name, new.mtime)?;
                c.execute(
                    "UPDATE media_item SET source_id = ?2, size = ?3, quick_hash = ?4, content_hash = ?5 WHERE id = ?1",
                    rusqlite::params![old, new.source_id, new.size, new.quick_hash, new.content_hash],
                )?;
                tracing::info!(media_id = old, path = new.path.as_str(), "file spostato ricollegato");
                old
            }
            None => repo::media::upsert(c, &new)?,
        };
        if let Some(t) = &tech {
            repo::meta::set_tech(c, id, t)?;
        }
        if let Some(d) = &dates {
            repo::meta::set_dates(c, id, d)?;
        }
        if let Some(e) = &exif {
            repo::meta::set_exif(c, id, &e.info)?;
        }
        if let Some(k) = &thumb {
            repo::media::set_thumb(c, id, k)?;
        }
        for x in &extra {
            x.write(c, id)?;
        }
        repo::fts::refresh(c, id)?;
        Ok(())
    });
    match res {
        Ok(()) => StageOutcome::Continue,
        Err(e) => StageOutcome::Fail(e.into()),
    }
}

/// Fasi del job di import (incluse quelle degli stadi aggiuntivi).
pub fn phases(deps: &ImportDeps) -> Vec<PhaseSpec> {
    let mut p = vec![
        PhaseSpec::new(PHASE_SCAN, 0.5),
        PhaseSpec::new(PHASE_META, 1.5),
        PhaseSpec::new(PHASE_THUMB, 2.0),
    ];
    p.extend(deps.extra_stages.iter().map(|s| s.phase()));
    if let Some(factory) = &deps.stage_factory {
        p.extend(factory.phases.clone());
    }
    p.push(PhaseSpec::new(PHASE_WRITE, 0.5));
    p
}

/// Piano del job: parametri `{ "sourceId": <id> }`.
pub fn plan(deps: ImportDeps, params: &serde_json::Value) -> AppResult<JobPlan> {
    let source_id = params["sourceId"]
        .as_i64()
        .ok_or_else(|| AppError::invalid_input("sourceId mancante"))?;
    let source = deps.store.read(|c| repo::source::get(c, source_id))?;
    let rules = ScanRules::from_source(&source)?;
    let mut deps = deps;
    let only_paths: Option<Vec<String>> = params
        .get("paths")
        .map(|v| serde_json::from_value(v.clone()))
        .transpose()?;
    Ok(JobPlan {
        phases: phases(&deps),
        min_memory_mb: 256,
        run: Box::new(move |ctx| {
            if let Some(factory) = deps.stage_factory.take() {
                let extra = (factory.build)(source_id);
                deps.extra_stages.extend(extra.stages);
                *ctx.followup.lock() = extra.followup;
                for phase in factory.phases {
                    if !deps
                        .extra_stages
                        .iter()
                        .any(|s| s.phase().name == phase.name)
                    {
                        let p = ctx.progress.phase(&phase.name);
                        p.set_total(0);
                        p.finish();
                    }
                }
            }
            deps.extra_stages.retain(|s| match s.prepare() {
                Ok(()) => true,
                Err(e) => {
                    let p = ctx.progress.phase(&s.phase().name);
                    p.inc_failed(1);
                    p.finish();
                    tracing::error!(
                        job_id = ctx.job_id,
                        phase = s.phase().name,
                        code = e.code.as_str(),
                        detail = e.detail,
                        "preparazione stadio AI fallita"
                    );
                    false
                }
            });
            run(
                deps,
                source_id,
                rules,
                ctx,
                only_paths.map(|v| v.into_iter().collect()),
            )
        }),
    })
}

fn run(
    deps: ImportDeps,
    source_id: i64,
    rules: ScanRules,
    ctx: JobContext,
    only_paths: Option<HashSet<String>>,
) -> AppResult<RunSummary> {
    let job_id = ctx.job_id;
    let seen: Arc<Mutex<HashSet<String>>> = Arc::default();
    let seen2 = seen.clone();
    let keep_decoded = deps.extra_stages.iter().any(|s| s.needs_image());
    let (d_meta, d_thumb, d_write) = (deps.clone(), deps.clone(), deps.clone());
    let partial = only_paths.is_some();

    let mut pipeline = Pipeline::<ImportItem>::new(job_id, ctx.progress, ctx.control.clone())
        .key(|i| i.path_str())
        .journal(ctx.journal)
        .channel_capacity(32)
        .source(PHASE_SCAN, move |emit| {
            rules.walk(
                |found| {
                    if only_paths
                        .as_ref()
                        .is_some_and(|paths| !paths.contains(&found.path.display().to_string()))
                    {
                        return true;
                    }
                    let ext = found
                        .path
                        .extension()
                        .map(|e| e.to_string_lossy().to_ascii_lowercase())
                        .unwrap_or_default();
                    seen2.lock().insert(found.path.display().to_string());
                    emit.emit(ImportItem {
                        found,
                        source_id,
                        ext,
                        existing: None,
                        moved_from: None,
                        quick_hash: None,
                        content_hash: None,
                        tech: None,
                        exif: None,
                        dates: None,
                        thumb_key: None,
                        decoded: None,
                        extra: Vec::new(),
                    })
                },
                |path, e| {
                    emit.phase().inc_failed(1);
                    tracing::error!(
                        code = e.code.as_str(),
                        job_id,
                        phase = PHASE_SCAN,
                        path = %path.display(),
                        detail = e.detail.as_str(),
                        "{}",
                        e.message_key
                    );
                },
            )
        })
        .stage(PHASE_META, deps.io_workers.max(1), move |item, ctx| {
            stage_metadata(&d_meta.store, item, ctx, partial)
        })
        .stage(PHASE_THUMB, deps.decode_workers.max(1), move |item, ctx| {
            stage_thumbnail(&d_thumb, keep_decoded, item, ctx)
        });
    for stage in &deps.extra_stages {
        let s = stage.clone();
        let name = s.phase().name;
        pipeline = pipeline.stage(&name, s.workers().max(1), move |item, ctx| {
            s.process(item, ctx)
        });
    }
    let summary = pipeline
        .stage(PHASE_WRITE, 1, move |item, _| {
            stage_write(&d_write.store, item)
        })
        .run()?;

    if !summary.cancelled && !partial {
        mark_missing(&deps.store, source_id, &seen.lock())?;
        deps.store.optimize()?;
    }
    deps.store.flush()?;
    Ok(summary)
}

/// Gli elementi della sorgente non trovati nella scansione diventano "mancanti" (mai cancellati).
fn mark_missing(store: &Store, source_id: i64, seen: &HashSet<String>) -> AppResult<usize> {
    store.flush()?;
    let items = store.read(|c| repo::media::by_source(c, source_id))?;
    let missing: Vec<MediaId> = items
        .into_iter()
        .filter(|m| m.status == MediaStatus::Ok && !seen.contains(&m.path))
        .map(|m| m.id)
        .collect();
    let n = missing.len();
    if n > 0 {
        store.write(move |c| {
            for id in &missing {
                repo::media::set_status(c, *id, MediaStatus::Missing)?;
            }
            Ok(())
        })?;
        tracing::info!(
            source_id,
            count = n,
            "file non più presenti marcati come mancanti"
        );
    }
    Ok(n)
}
