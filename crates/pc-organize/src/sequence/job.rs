//! Job `sequence_export`: copie rinominate della sequenza in ordine di istante
//! reale, cartelle per tag, manifest e archivi zip.
//!
//! Tutto viene scritto in `<destinazione>/.photo-cat-export-<job>/` e spostato
//! nella destinazione solo alla fine (rinomine sullo stesso volume). In caso
//! di annullamento o errore la cartella temporanea viene rimossa: nella
//! destinazione non resta nulla di parziale. Gli originali vengono solo letti.

use super::encode::{encode_verified, estimated_bytes, JpegSettings};
use super::metadata::{app_segments, CopyMeta};
use super::naming::{self, NameInput, TemplateError};
use pc_core::{AppError, AppResult, ErrorCode};
use pc_governor::MemoryGovernor;
use pc_jobs::{JobPlan, PhaseSpec, RunSummary};
use pc_store::sequence::{self, SequenceScope};
use pc_store::timeline::{local_text, offset_text};
use pc_store::{Store, StoreResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

pub const JOB_KIND: &str = "sequence_export";
pub const PHASE_PREFLIGHT: &str = "preparazione";
pub const PHASE_ENCODE: &str = "codifica";
pub const PHASE_PER_TAG: &str = "per_tag";
pub const PHASE_MANIFEST: &str = "manifest";
pub const PHASE_ZIP: &str = "archivi";

/// Nomi prodotti nella destinazione.
pub const OUTPUTS: &[&str] = &[
    "sequenza",
    "per-tag",
    "manifest.csv",
    "sequenza.zip",
    "per-tag.zip",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum ExportFormat {
    /// JPEG ricodificato e verificato.
    #[default]
    Jpeg,
    /// Copia dell'originale senza ricodifica.
    Original,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SequenceExportOptions {
    pub scope: SequenceScope,
    pub template: String,
    pub destination: String,
    #[serde(default)]
    pub format: ExportFormat,
    #[serde(default)]
    pub jpeg: JpegSettings,
    pub per_tag: bool,
    pub manifest: bool,
    /// Archivio non compresso di `sequenza/`.
    pub zip: bool,
    /// Archivio non compresso di `per-tag/` (con le cartelle per tag).
    #[serde(default)]
    pub zip_per_tag: bool,
    /// Metadati (data, fotocamera, GPS, parole chiave, identità) nelle copie.
    pub metadata: bool,
    /// Set (esclusivo) usato per `{tag}` e per le cartelle per tag.
    pub tag_set: Option<i64>,
}

/// Dati di `app.db` risolti dal chiamante: il job lavora solo sul catalogo.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportContext {
    /// Tag del set scelto, in ordine di tasto.
    pub set_tags: Vec<String>,
    /// Tag dei set che vanno nelle parole chiave XMP.
    pub keyword_tags: Vec<String>,
    /// uid identità → (nome, nome breve).
    pub identities: HashMap<String, (String, String)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct NamePreview {
    pub original: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ExportPreflight {
    pub count: u32,
    /// Foto senza tag del set scelto (solo in `sequenza/`).
    pub untagged: u32,
    pub required_bytes: u64,
    pub free_bytes: Option<u64>,
    pub hardlinks: bool,
    /// Nomi già presenti nella destinazione.
    pub existing: Vec<String>,
    pub template_errors: Vec<TemplateError>,
    pub preview: Vec<NamePreview>,
    pub can_start: bool,
    /// Motivo per cui l'avvio è bloccato.
    pub blocked: Option<String>,
}

/// Una foto della sequenza con i dati per nome, metadati e manifest.
#[derive(Debug, Clone)]
struct Item {
    path: String,
    stem: String,
    ext: String,
    size: i64,
    width: u32,
    height: u32,
    instant: i64,
    offset: i32,
    local: String,
    device: Option<String>,
    make: Option<String>,
    model: Option<String>,
    lens: Option<String>,
    gps: Option<(f64, f64)>,
    place: Option<String>,
    tags: Vec<String>,
    /// (nome, nome breve) delle persone presenti.
    people: Vec<(String, String)>,
}

fn load_items(
    store: &Store,
    scope: &SequenceScope,
    ctx: &ExportContext,
    limit: Option<usize>,
) -> AppResult<Vec<Item>> {
    let scope = scope.clone();
    let ctx = ctx.clone();
    Ok(store.read(move |c| {
        let mut ids = sequence::ordered_ids(c, &scope)?;
        if let Some(n) = limit {
            ids.truncate(n);
        }
        let mut q = c.prepare_cached(
            "SELECT m.path, m.name, m.ext, m.size, COALESCE(t.width, 0), COALESCE(t.height, 0),
                    d.instant_utc, COALESCE(d.local_offset_min, 0), d.device,
                    e.camera_make, e.camera_model, e.lens, e.gps_lat, e.gps_lon, lo.name
             FROM media_item m JOIN media_dates d ON d.media_id = m.id
             LEFT JOIN media_tech t ON t.media_id = m.id
             LEFT JOIN exif e ON e.media_id = m.id
             LEFT JOIN location lo ON lo.media_id = m.id
             WHERE m.id = ?1",
        )?;
        let mut people = c.prepare_cached(
            "SELECT DISTINCT p.identity_uid, p.name FROM face f JOIN person p ON p.id = f.person_id
             WHERE f.media_id = ?1 AND (p.name IS NOT NULL OR p.identity_uid IS NOT NULL) ORDER BY p.name",
        )?;
        ids.iter()
            .map(|id| -> StoreResult<Item> {
                let mut it = q.query_row([id], |r| {
                    let name: String = r.get(1)?;
                    let lat: Option<f64> = r.get(12)?;
                    let lon: Option<f64> = r.get(13)?;
                    let instant: i64 = r.get(6)?;
                    let offset: i32 = r.get(7)?;
                    Ok(Item {
                        path: r.get(0)?,
                        stem: Path::new(&name)
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or(name),
                        ext: r.get(2)?,
                        size: r.get(3)?,
                        width: r.get(4)?,
                        height: r.get(5)?,
                        instant,
                        offset,
                        local: local_text(instant, offset).unwrap_or_default(),
                        device: r.get(8)?,
                        make: r.get(9)?,
                        model: r.get(10)?,
                        lens: r.get(11)?,
                        gps: lat.zip(lon),
                        place: r.get(14)?,
                        tags: Vec::new(),
                        people: Vec::new(),
                    })
                })?;
                it.tags = pc_store::repo::tag::of_media(c, *id)?;
                let rows = people
                    .query_map([id], |r| Ok((r.get::<_, Option<String>>(0)?, r.get::<_, Option<String>>(1)?)))?
                    .collect::<Result<Vec<_>, _>>()?;
                for (uid, name) in rows {
                    let pair = uid
                        .and_then(|u| ctx.identities.get(&u).cloned())
                        .or_else(|| name.map(|n| (n.clone(), n.split_whitespace().next().unwrap_or("").to_string())));
                    if let Some(p) = pair {
                        if !it.people.contains(&p) {
                            it.people.push(p);
                        }
                    }
                }
                Ok(it)
            })
            .collect()
    })?)
}

fn set_tag<'a>(item: &'a Item, ctx: &'a ExportContext) -> Option<&'a str> {
    ctx.set_tags
        .iter()
        .find(|t| {
            item.tags
                .iter()
                .any(|x| x.to_lowercase() == t.to_lowercase())
        })
        .map(String::as_str)
}

fn set_tags_of(item: &Item, ctx: &ExportContext) -> Vec<String> {
    ctx.set_tags
        .iter()
        .filter(|t| {
            item.tags
                .iter()
                .any(|x| x.to_lowercase() == t.to_lowercase())
        })
        .cloned()
        .collect()
}

fn out_ext(opts: &SequenceExportOptions, item: &Item) -> String {
    match opts.format {
        ExportFormat::Jpeg => "jpg".into(),
        ExportFormat::Original => item.ext.to_lowercase(),
    }
}

/// Nomi finali (con estensione), unici, in ordine di sequenza.
fn names(
    template: &naming::Template,
    items: &[Item],
    opts: &SequenceExportOptions,
    ctx: &ExportContext,
) -> Vec<String> {
    let bases: Vec<String> = items
        .iter()
        .enumerate()
        .map(|(i, it)| {
            let shorts: Vec<String> = it.people.iter().map(|p| p.1.clone()).collect();
            template.render(&NameInput {
                seq: i + 1,
                local: &it.local,
                tag: set_tag(it, ctx),
                device: it.device.as_deref(),
                people: &shorts,
                place: it.place.as_deref(),
                orig: &it.stem,
            })
        })
        .collect();
    // L'unicità vale per nome completo, quindi per estensione.
    let mut out = vec![String::new(); items.len()];
    let mut by_ext: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, it) in items.iter().enumerate() {
        by_ext.entry(out_ext(opts, it)).or_default().push(i);
    }
    for (ext, idx) in by_ext {
        let group: Vec<String> = idx.iter().map(|i| bases[*i].clone()).collect();
        for (k, name) in naming::finalize(&group, &ext).into_iter().enumerate() {
            out[idx[k]] = name;
        }
    }
    out
}

/// Anteprima dei nomi delle prime `n` foto (mentre si scrive il modello).
pub fn preview_names(
    store: &Store,
    opts: &SequenceExportOptions,
    ctx: &ExportContext,
    n: usize,
) -> AppResult<Result<Vec<NamePreview>, Vec<TemplateError>>> {
    let template = match naming::parse(&opts.template) {
        Ok(t) => t,
        Err(e) => return Ok(Err(e)),
    };
    let items = load_items(store, &opts.scope, ctx, Some(n))?;
    let names = names(&template, &items, opts, ctx);
    Ok(Ok(items
        .iter()
        .zip(names)
        .map(|(it, name)| NamePreview {
            original: format!("{}.{}", it.stem, it.ext),
            name,
        })
        .collect()))
}

/// Spazio libero sul volume che contiene `path` (il punto di mount più lungo).
pub fn free_space(path: &Path) -> Option<u64> {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let target = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let key = |p: &Path| p.to_string_lossy().to_lowercase().replace("\\\\?\\", "");
    let t = key(&target);
    disks
        .list()
        .iter()
        .filter(|d| t.starts_with(&key(d.mount_point())))
        .max_by_key(|d| d.mount_point().as_os_str().len())
        .map(|d| d.available_space())
}

/// Antenato esistente più vicino (la destinazione può non esistere ancora).
fn existing_ancestor(path: &Path) -> Option<PathBuf> {
    let mut p = Some(path);
    while let Some(x) = p {
        if x.is_dir() {
            return Some(x.to_path_buf());
        }
        p = x.parent();
    }
    None
}

/// Prova a creare un hard link nella destinazione.
pub fn hardlinks_supported(dir: &Path) -> bool {
    let Some(base) = existing_ancestor(dir) else {
        return false;
    };
    let Ok(src) = tempfile::NamedTempFile::new_in(&base) else {
        return false;
    };
    let link = base.join(format!(".photo-cat-link-{}", std::process::id()));
    let ok = std::fs::hard_link(src.path(), &link).is_ok();
    let _ = std::fs::remove_file(&link);
    ok
}

fn gb(b: u64) -> String {
    format!("{:.1} GB", b as f64 / 1e9)
}

/// Verifica preliminare: modello, destinazione libera, spazio e hard link.
pub fn preflight(
    store: &Store,
    opts: &SequenceExportOptions,
    ctx: &ExportContext,
) -> AppResult<ExportPreflight> {
    let template_errors = naming::parse(&opts.template).err().unwrap_or_default();
    let items = load_items(store, &opts.scope, ctx, None)?;
    let preview = match naming::parse(&opts.template) {
        Ok(t) => {
            let head: Vec<Item> = items.iter().take(8).cloned().collect();
            let n = names(&t, &head, opts, ctx);
            head.iter()
                .zip(n)
                .map(|(it, name)| NamePreview {
                    original: format!("{}.{}", it.stem, it.ext),
                    name,
                })
                .collect()
        }
        Err(_) => Vec::new(),
    };
    let dest = PathBuf::from(&opts.destination);
    let existing: Vec<String> = OUTPUTS
        .iter()
        .filter(|n| dest.join(n).exists())
        .map(|s| s.to_string())
        .collect();
    let hardlinks = opts.per_tag && hardlinks_supported(&dest);
    let per_file = |it: &Item| -> u64 {
        match opts.format {
            ExportFormat::Jpeg => estimated_bytes(
                it.width.max(1),
                it.height.max(1),
                opts.jpeg.quality,
                opts.jpeg.chroma_444,
            ),
            ExportFormat::Original => it.size.max(0) as u64,
        }
    };
    let seq_bytes: u64 = items.iter().map(per_file).sum();
    let tagged: Vec<&Item> = items
        .iter()
        .filter(|it| set_tag(it, ctx).is_some())
        .collect();
    let tagged_bytes: u64 = tagged.iter().map(|it| per_file(it)).sum();
    let mut required = seq_bytes + 1024 * 1024;
    if opts.per_tag && !hardlinks {
        required += tagged_bytes;
    }
    if opts.zip {
        required += seq_bytes;
    }
    if opts.per_tag && opts.zip_per_tag {
        required += tagged_bytes;
    }
    let free = existing_ancestor(&dest).and_then(|p| free_space(&p));
    let blocked = if !template_errors.is_empty() {
        Some("Il modello del nome contiene errori.".to_string())
    } else if items.is_empty() {
        Some("Nessuna foto datata nell'ambito.".to_string())
    } else if !existing.is_empty() {
        Some(format!(
            "La destinazione contiene già {}: scegli un'altra cartella o una sottocartella nuova.",
            existing.join(", ")
        ))
    } else if opts.per_tag && ctx.set_tags.is_empty() {
        Some("Scegli un set di tag per le cartelle per tag.".to_string())
    } else if free.is_some_and(|f| f < required) {
        Some(format!(
            "Spazio insufficiente: servono {} e la destinazione ne ha {}.",
            gb(required),
            gb(free.unwrap_or(0))
        ))
    } else {
        None
    };
    Ok(ExportPreflight {
        count: items.len() as u32,
        untagged: (items.len() - tagged.len()) as u32,
        required_bytes: required,
        free_bytes: free,
        hardlinks,
        existing,
        template_errors,
        preview,
        can_start: blocked.is_none(),
        blocked,
    })
}

/// Esito per foto, per manifest e riepilogo.
#[derive(Debug, Clone)]
struct Outcome {
    quality: Option<u8>,
    ssim: Option<f64>,
    psnr: Option<f64>,
    below: bool,
}

/// Rimuove la cartella temporanea se il job non arriva in fondo.
struct TempDir {
    path: PathBuf,
    keep: bool,
}
impl Drop for TempDir {
    fn drop(&mut self) {
        if !self.keep {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

fn copy_meta(it: &Item, ctx: &ExportContext) -> CopyMeta {
    CopyMeta {
        local: (!it.local.is_empty()).then(|| it.local.clone()),
        offset_min: Some(it.offset),
        make: it.make.clone(),
        model: it.model.clone(),
        lens: it.lens.clone(),
        gps: it.gps,
        keywords: it
            .tags
            .iter()
            .filter(|t| {
                ctx.keyword_tags
                    .iter()
                    .any(|k| k.to_lowercase() == t.to_lowercase())
            })
            .cloned()
            .collect(),
        people: it.people.iter().map(|p| p.0.clone()).collect(),
    }
}

fn encode_one(
    it: &Item,
    target: &Path,
    opts: &SequenceExportOptions,
    ctx: &ExportContext,
    gov: &MemoryGovernor,
) -> AppResult<Outcome> {
    if opts.format == ExportFormat::Original {
        std::fs::copy(&it.path, target)?;
        return Ok(Outcome {
            quality: None,
            ssim: None,
            psnr: None,
            below: false,
        });
    }
    let img = pc_ingest::full::decode_full(Path::new(&it.path), &it.ext, gov)?.oriented();
    let rgb = img.to_rgb8();
    let segments = if opts.metadata {
        app_segments(&copy_meta(it, ctx))?
    } else {
        // Anche senza metadati l'orientamento va dichiarato "normale".
        app_segments(&CopyMeta::default())?
    };
    let out = encode_verified(&rgb, img.icc.as_deref(), &segments, &opts.jpeg)?;
    drop(rgb);
    let mut f = std::fs::File::create(target)?;
    f.write_all(&out.bytes)?;
    f.sync_all()?;
    Ok(Outcome {
        quality: Some(out.quality),
        ssim: out.ssim,
        psnr: out.psnr,
        below: out.below_threshold,
    })
}

fn csv(v: &str) -> String {
    if v.contains([';', '"', '\n', '\r']) {
        format!("\"{}\"", v.replace('"', "\"\""))
    } else {
        v.to_string()
    }
}

fn iso_utc(ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|t| t.format("%Y-%m-%dT%H:%M:%SZ").to_string())
        .unwrap_or_default()
}

fn write_manifest(
    path: &Path,
    rows: &[(String, &Item, Option<&Outcome>)],
    ctx: &ExportContext,
) -> AppResult<()> {
    let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
    // BOM e punto e virgola: si apre direttamente nei fogli di calcolo in italiano.
    f.write_all(b"\xef\xbb\xbf")?;
    writeln!(
        f,
        "nome;percorso_originale;istante_utc;ora_locale;dispositivo;tag;identita;qualita;ssim;psnr;esito"
    )?;
    for (name, it, out) in rows {
        let (q, s, p, esito) = match out {
            Some(o) => (
                o.quality.map(|q| q.to_string()).unwrap_or_default(),
                o.ssim.map(|v| format!("{v:.5}")).unwrap_or_default(),
                o.psnr
                    .map(|v| {
                        if v.is_finite() {
                            format!("{v:.2}")
                        } else {
                            "inf".into()
                        }
                    })
                    .unwrap_or_default(),
                if o.quality.is_none() {
                    "copia"
                } else if o.below {
                    "sotto_soglia"
                } else {
                    "ok"
                },
            ),
            None => (String::new(), String::new(), String::new(), "errore"),
        };
        let cells = [
            name.clone(),
            it.path.clone(),
            iso_utc(it.instant),
            format!("{} {}", it.local, offset_text(it.offset)),
            it.device.clone().unwrap_or_default(),
            set_tags_of(it, ctx).join(", "),
            it.people
                .iter()
                .map(|p| p.0.clone())
                .collect::<Vec<_>>()
                .join(", "),
            q,
            s,
            p,
            esito.to_string(),
        ];
        writeln!(
            f,
            "{}",
            cells.iter().map(|c| csv(c)).collect::<Vec<_>>().join(";")
        )?;
    }
    f.flush()?;
    Ok(())
}

/// Zip non compresso (con ZIP64) del contenuto di `dir`.
fn zip_dir(dir: &Path, target: &Path, cancelled: &dyn Fn() -> bool) -> AppResult<bool> {
    let file = std::fs::File::create(target)?;
    let mut zip = zip::ZipWriter::new(std::io::BufWriter::new(file));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .large_file(true);
    let mut stack = vec![dir.to_path_buf()];
    let mut entries = Vec::new();
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d)? {
            let p = e?.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                entries.push(p);
            }
        }
    }
    entries.sort();
    let zerr = |e: zip::result::ZipError| AppError::new(ErrorCode::Io, e.to_string());
    for p in entries {
        if cancelled() {
            return Ok(false);
        }
        let rel = p
            .strip_prefix(dir)
            .map_err(|e| AppError::internal(e.to_string()))?
            .to_string_lossy()
            .replace('\\', "/");
        zip.start_file(rel, options).map_err(zerr)?;
        std::io::copy(&mut std::fs::File::open(&p)?, &mut zip)?;
    }
    zip.finish().map_err(zerr)?.flush()?;
    Ok(true)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Params {
    options: SequenceExportOptions,
    #[serde(default)]
    context: ExportContext,
}

pub fn job_params(options: &SequenceExportOptions, context: &ExportContext) -> serde_json::Value {
    serde_json::json!({ "options": options, "context": context })
}

pub fn plan(store: Store, gov: MemoryGovernor, params: &serde_json::Value) -> AppResult<JobPlan> {
    let Params {
        options: opts,
        context: ctx,
    } = serde_json::from_value(params.clone())?;
    let template = naming::parse(&opts.template).map_err(|e| {
        AppError::invalid_input(format!(
            "modello del nome non valido: {}",
            e.iter()
                .map(|x| x.text.clone())
                .collect::<Vec<_>>()
                .join(", ")
        ))
    })?;
    Ok(JobPlan {
        phases: vec![
            PhaseSpec::new(PHASE_PREFLIGHT, 0.5),
            PhaseSpec::new(PHASE_ENCODE, 10.0),
            PhaseSpec::new(PHASE_PER_TAG, 0.5),
            PhaseSpec::new(PHASE_MANIFEST, 0.2),
            PhaseSpec::new(PHASE_ZIP, 2.0),
        ],
        min_memory_mb: 64,
        run: Box::new(move |jctx| {
            let cancelled = || RunSummary {
                cancelled: true,
                failed: 0,
            };
            let pre = jctx.progress.phase(PHASE_PREFLIGHT);
            pre.set_total(1);
            pre.start();
            let check = preflight(&store, &opts, &ctx)?;
            if let Some(why) = check.blocked {
                return Err(AppError::invalid_input(why));
            }
            let dest = crate::paths::writable_destination(&store, Path::new(&opts.destination))?;
            std::fs::create_dir_all(&dest)?;
            let tmp = TempDir {
                path: dest.join(format!(".photo-cat-export-{}", jctx.job_id)),
                keep: false,
            };
            let seq_dir = tmp.path.join("sequenza");
            std::fs::create_dir_all(&seq_dir)?;
            let items = load_items(&store, &opts.scope, &ctx, None)?;
            let names = names(&template, &items, &opts, &ctx);
            pre.inc_done(1);
            pre.finish();

            // Codifica in parallelo: i permessi del governor limitano la memoria.
            let enc = jctx.progress.phase(PHASE_ENCODE);
            enc.set_total(items.len() as i64);
            enc.start();
            let workers = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(2)
                .clamp(1, 4)
                .min(items.len().max(1));
            let next = AtomicUsize::new(0);
            let results: Mutex<Vec<Option<Outcome>>> = Mutex::new(vec![None; items.len()]);
            let failed = AtomicUsize::new(0);
            std::thread::scope(|s| {
                for _ in 0..workers {
                    s.spawn(|| loop {
                        if !jctx.control.checkpoint() {
                            break;
                        }
                        let i = next.fetch_add(1, Ordering::SeqCst);
                        if i >= items.len() {
                            break;
                        }
                        let it = &items[i];
                        match encode_one(it, &seq_dir.join(&names[i]), &opts, &ctx, &gov) {
                            Ok(o) => {
                                if o.below {
                                    tracing::warn!(
                                        job_id = jctx.job_id,
                                        phase = PHASE_ENCODE,
                                        path = %it.path,
                                        "soglia di qualità non raggiunta nemmeno a 100"
                                    );
                                }
                                results.lock().expect("risultati")[i] = Some(o);
                                enc.inc_done(1);
                            }
                            Err(e) => {
                                failed.fetch_add(1, Ordering::SeqCst);
                                enc.inc_failed(1);
                                tracing::error!(
                                    job_id = jctx.job_id,
                                    phase = PHASE_ENCODE,
                                    path = %it.path,
                                    code = e.code.as_str(),
                                    detail = %e.detail,
                                    "foto non esportata"
                                );
                            }
                        }
                    });
                }
            });
            if jctx.control.is_cancelled() {
                return Ok(cancelled());
            }
            enc.finish();
            let results = results.into_inner().expect("risultati");

            // Cartelle per tag: hard link ai file di sequenza/ se possibile.
            let per_tag_dir = tmp.path.join("per-tag");
            let pt = jctx.progress.phase(PHASE_PER_TAG);
            if opts.per_tag {
                pt.set_total(items.len() as i64);
                pt.start();
                for (i, it) in items.iter().enumerate() {
                    if jctx.control.is_cancelled() {
                        return Ok(cancelled());
                    }
                    if results[i].is_some() {
                        for tag in set_tags_of(it, &ctx) {
                            let folder =
                                per_tag_dir.join(naming::sanitize(&naming::clean_value(&tag)));
                            std::fs::create_dir_all(&folder)?;
                            let (src, dst) = (seq_dir.join(&names[i]), folder.join(&names[i]));
                            if std::fs::hard_link(&src, &dst).is_err() {
                                std::fs::copy(&src, &dst)?;
                            }
                        }
                    }
                    pt.inc_done(1);
                }
            }
            pt.finish();

            let mf = jctx.progress.phase(PHASE_MANIFEST);
            if opts.manifest {
                mf.set_total(1);
                mf.start();
                let rows: Vec<(String, &Item, Option<&Outcome>)> = items
                    .iter()
                    .enumerate()
                    .map(|(i, it)| (names[i].clone(), it, results[i].as_ref()))
                    .collect();
                write_manifest(&tmp.path.join("manifest.csv"), &rows, &ctx)?;
                mf.inc_done(1);
            }
            mf.finish();

            let zp = jctx.progress.phase(PHASE_ZIP);
            let zip_tags = opts.per_tag && opts.zip_per_tag;
            if opts.zip || zip_tags {
                zp.set_total(i64::from(opts.zip) + i64::from(zip_tags));
                zp.start();
                let stop = || jctx.control.is_cancelled();
                if opts.zip {
                    if !zip_dir(&seq_dir, &tmp.path.join("sequenza.zip"), &stop)? {
                        return Ok(cancelled());
                    }
                    zp.inc_done(1);
                }
                if zip_tags {
                    std::fs::create_dir_all(&per_tag_dir)?;
                    if !zip_dir(&per_tag_dir, &tmp.path.join("per-tag.zip"), &stop)? {
                        return Ok(cancelled());
                    }
                    zp.inc_done(1);
                }
            }
            zp.finish();
            if jctx.control.is_cancelled() {
                return Ok(cancelled());
            }

            // Spostamento finale: stesso volume, rinomine; nulla viene sovrascritto.
            let mut moved: Vec<PathBuf> = Vec::new();
            for name in OUTPUTS {
                let src = tmp.path.join(name);
                if !src.exists() {
                    continue;
                }
                let dst = dest.join(name);
                let res = if dst.exists() {
                    Err(AppError::new(
                        ErrorCode::Io,
                        format!("{} esiste già", dst.display()),
                    ))
                } else {
                    std::fs::rename(&src, &dst).map_err(AppError::from)
                };
                if let Err(e) = res {
                    // Si riportano indietro gli elementi già spostati.
                    for m in moved.iter().rev() {
                        let back = tmp.path.join(m.file_name().unwrap_or_default());
                        let _ = std::fs::rename(m, back);
                    }
                    return Err(e);
                }
                moved.push(dst);
            }
            drop(tmp);
            Ok(RunSummary {
                cancelled: false,
                failed: failed.into_inner() as i64,
            })
        }),
    })
}

/// Vero se `path` è una cartella temporanea di export (pulizia all'avvio).
pub fn is_temp_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with(".photo-cat-export-"))
}
