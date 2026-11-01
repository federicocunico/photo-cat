//! Ricollegamento dei file mancanti dopo lo spostamento di una cartella.
//!
//! Per ogni mancante sotto `oldFolder` si cerca lo stesso percorso relativo in
//! `newFolder` (stessa dimensione e impronta rapida); i rimanenti si cercano
//! per impronta del contenuto tra i file di `newFolder`, solo con
//! corrispondenza univoca. Catalogazioni, tag, cataloghi e persone restano.

use crate::formats::support;
use crate::hash::{content_hash, quick_hash};
use crate::scan::mtime_ms;
use pc_core::{AppError, AppResult};
use pc_jobs::{JobPlan, PhaseSpec, RunSummary};
use pc_store::model::{MediaId, MediaItem};
use pc_store::sources::path_in_folder;
use pc_store::{now_ms, repo, sources, Store};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const JOB_KIND: &str = "relink";
pub const PHASE: &str = "ricollegamento";

/// Esito dell'ultimo ricollegamento di una sorgente.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct RelinkSummary {
    pub source_id: i64,
    pub job_id: i64,
    pub finished_at: i64,
    pub old_folder: String,
    pub new_folder: String,
    /// Ritrovati per percorso relativo.
    pub relinked: i64,
    /// Ritrovati per impronta del contenuto (rinominati o spostati).
    pub by_hash: i64,
    /// Restano mancanti (non trovati o ambigui).
    pub still_missing: i64,
    /// Di cui ambigui: più copie identiche.
    pub ambiguous: i64,
    /// La nuova cartella non sta in nessuna sorgente.
    pub outside_sources: bool,
    /// Nessun mancante resta nella sorgente: si può spostarla sulla nuova cartella.
    pub source_complete: bool,
}

fn key(source_id: i64) -> String {
    format!("import.last_relink.{source_id}")
}

pub fn last(store: &Store, source_id: i64) -> AppResult<Option<RelinkSummary>> {
    let raw: Option<String> = store.read(|c| {
        Ok(c.query_row(
            "SELECT value FROM settings_kv WHERE key=?1",
            [key(source_id)],
            |r| r.get(0),
        )
        .optional()?)
    })?;
    Ok(raw.and_then(|v| serde_json::from_str(&v).ok()))
}

/// Percorso di `path` relativo a `folder`, con `/` come separatore.
fn relative(path: &str, folder: &str) -> Option<String> {
    let folder = folder.trim_end_matches(['\\', '/']);
    if !path_in_folder(path, folder) || path.len() <= folder.len() {
        return None;
    }
    Some(path.get(folder.len() + 1..)?.replace('\\', "/"))
}

fn join(folder: &Path, rel: &str) -> PathBuf {
    rel.split('/').fold(folder.to_path_buf(), |p, s| p.join(s))
}

/// Stesso file: dimensione e impronta rapida coincidono.
fn same_quick(item: &MediaItem, path: &Path) -> Option<i64> {
    let meta = std::fs::metadata(path).ok().filter(|m| m.is_file())?;
    if meta.len() as i64 != item.size {
        return None;
    }
    let qh = quick_hash(path).ok()?;
    (item.quick_hash.as_deref() == Some(qh.as_str())).then(|| mtime_ms(&meta))
}

#[derive(Default)]
struct Found {
    /// (id, nuovo percorso, mtime, per impronta)
    links: Vec<(MediaId, String, i64, bool)>,
    ambiguous: i64,
}

fn find(
    missing: Vec<MediaItem>,
    old_folder: &str,
    new_folder: &Path,
    known: &dyn Fn(&str) -> bool,
) -> Found {
    let mut found = Found::default();
    let mut rest = Vec::new();
    for m in missing {
        let candidate = relative(&m.path, old_folder).map(|r| join(new_folder, &r));
        match candidate
            .as_ref()
            .and_then(|p| same_quick(&m, p).map(|t| (p, t)))
        {
            Some((p, mtime)) => found
                .links
                .push((m.id, p.display().to_string(), mtime, false)),
            None => rest.push(m),
        }
    }
    if rest.is_empty() {
        return found;
    }
    // Indice dei file non ancora in libreria per dimensione e impronta rapida.
    let mut index: HashMap<(i64, String), Vec<(PathBuf, i64)>> = HashMap::new();
    for entry in walkdir::WalkDir::new(new_folder)
        .follow_links(false)
        .into_iter()
        .flatten()
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        let path_str = path.display().to_string();
        if support(&ext).is_none() || known(&path_str) {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let Ok(qh) = quick_hash(path) else { continue };
        index
            .entry((meta.len() as i64, qh))
            .or_default()
            .push((path.to_path_buf(), mtime_ms(&meta)));
    }
    let mut claims: HashMap<(i64, String), usize> = HashMap::new();
    for m in &rest {
        if let Some(qh) = &m.quick_hash {
            *claims.entry((m.size, qh.clone())).or_default() += 1;
        }
    }
    for m in rest {
        let Some(qh) = m.quick_hash.clone() else {
            continue;
        };
        let k = (m.size, qh);
        let candidates = index.get(&k).map(Vec::as_slice).unwrap_or_default();
        if candidates.is_empty() {
            continue;
        }
        if candidates.len() > 1 || claims.get(&k).copied().unwrap_or(0) > 1 {
            found.ambiguous += 1;
            continue;
        }
        let (path, mtime) = &candidates[0];
        let verified = match (&m.content_hash, content_hash(path)) {
            (Some(expected), Ok(actual)) => *expected == actual,
            (None, Ok(_)) => true,
            _ => false,
        };
        if verified {
            found
                .links
                .push((m.id, path.display().to_string(), *mtime, true));
        }
    }
    found
}

/// Piano del job: parametri `{ "sourceId", "oldFolder", "newFolder" }`.
pub fn plan(store: Store, params: &serde_json::Value) -> AppResult<JobPlan> {
    let source_id = params["sourceId"]
        .as_i64()
        .ok_or_else(|| AppError::invalid_input("sourceId mancante"))?;
    let text = |k: &str| {
        params[k]
            .as_str()
            .map(|s| s.trim().trim_end_matches(['\\', '/']).to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| AppError::invalid_input(format!("{k} mancante")))
    };
    let old_folder = text("oldFolder")?;
    let new_folder = text("newFolder")?;
    if !Path::new(&new_folder).is_dir() {
        return Err(AppError::new(
            pc_core::ErrorCode::NotFound,
            format!("cartella non trovata: {new_folder}"),
        ));
    }
    store.read(|c| repo::source::get(c, source_id))?;
    Ok(JobPlan {
        phases: vec![PhaseSpec::new(PHASE, 1.0)],
        min_memory_mb: 32,
        run: Box::new(move |ctx| {
            let phase = ctx.progress.phase(PHASE);
            phase.start();
            let missing_ids: Vec<MediaId> = store
                .read(|c| sources::missing(c, source_id, Some(&old_folder)))?
                .into_iter()
                .map(|(id, _)| id)
                .collect();
            phase.set_total(missing_ids.len() as i64);
            let missing: Vec<MediaItem> = store.read(|c| {
                missing_ids
                    .iter()
                    .map(|id| repo::media::get(c, *id))
                    .collect()
            })?;
            let reader = store.clone();
            let known = move |p: &str| {
                reader
                    .read(|c| repo::media::get_by_path(c, p))
                    .ok()
                    .flatten()
                    .is_some()
            };
            if !ctx.control.checkpoint() {
                return Ok(RunSummary {
                    cancelled: true,
                    failed: 0,
                });
            }
            let found = find(missing, &old_folder, Path::new(&new_folder), &known);
            let total = missing_ids.len() as i64;
            let by_hash = found.links.iter().filter(|l| l.3).count() as i64;
            let relinked = found.links.len() as i64 - by_hash;
            let ambiguous = found.ambiguous;
            let links = found.links;
            let (old, new, job_id) = (old_folder.clone(), new_folder.clone(), ctx.job_id);
            let summary = store.write(move |c| {
                let target = sources::source_containing(c, &new)?;
                let n = links.len() as i64;
                for (id, path, mtime, _) in &links {
                    sources::relink_item(c, *id, path, *mtime, Some(target.unwrap_or(source_id)))?;
                }
                let remaining = sources::missing(c, source_id, None)?.len();
                let s = RelinkSummary {
                    source_id,
                    job_id,
                    finished_at: now_ms(),
                    old_folder: old,
                    new_folder: new,
                    relinked,
                    by_hash,
                    still_missing: total - n,
                    ambiguous,
                    outside_sources: target.is_none(),
                    source_complete: remaining == 0,
                };
                c.execute(
                    "INSERT INTO settings_kv(key,value) VALUES (?1,?2)
                     ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                    params![key(source_id), serde_json::to_string(&s)?],
                )?;
                Ok(s)
            })?;
            phase.inc_done(total);
            phase.finish();
            tracing::info!(
                source_id,
                relinked = summary.relinked,
                by_hash = summary.by_hash,
                still_missing = summary.still_missing,
                "mancanti ricollegati"
            );
            Ok(RunSummary {
                cancelled: false,
                failed: 0,
            })
        }),
    })
}
