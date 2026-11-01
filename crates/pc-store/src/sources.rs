//! Gestione delle sorgenti: riepilogo e rimozione completa, dati derivati,
//! file mancanti e riepilogo dell'ultimo aggiornamento.
//!
//! Le funzioni lavorano solo sul database: i file derivati su disco (miniature)
//! vengono restituiti al chiamante, che li cancella fuori dalla transazione.

use crate::model::MediaId;
use crate::{now_ms, repo, StoreResult};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Esito di una rimozione dalla libreria.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Removed {
    pub items: usize,
    /// Miniature non più usate da alcun elemento: da cancellare dalla cache.
    pub thumbs: Vec<String>,
    /// Persone senza nome rimaste senza volti, eliminate.
    pub people: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct CatalogLoss {
    pub name: String,
    pub items: i64,
}

/// Cosa comporta la rimozione di una sorgente (mostrato prima della conferma).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct RemovalSummary {
    pub items: i64,
    /// Foto con un'etichetta manuale, un tag o un luogo manuale.
    pub edited: i64,
    /// Cataloghi manuali che perderanno foto.
    pub catalogs: Vec<CatalogLoss>,
    /// Persone con nome che resteranno senza volti.
    pub orphaned_people: Vec<String>,
    /// Job attivi o in coda che verranno annullati (calcolato dall'app).
    pub jobs: u32,
}

pub fn removal_summary(c: &Connection, source_id: i64) -> StoreResult<RemovalSummary> {
    let items = c.query_row(
        "SELECT count(*) FROM media_item WHERE source_id=?1",
        [source_id],
        |r| r.get(0),
    )?;
    let edited = c.query_row(
        "SELECT count(*) FROM media_item m WHERE m.source_id=?1 AND (
            EXISTS (SELECT 1 FROM label l WHERE l.media_id=m.id AND l.origin='manual')
         OR EXISTS (SELECT 1 FROM media_tag t WHERE t.media_id=m.id)
         OR EXISTS (SELECT 1 FROM location lo WHERE lo.media_id=m.id AND lo.source='manual'))",
        [source_id],
        |r| r.get(0),
    )?;
    let mut q = c.prepare(
        "SELECT c.name, count(*) FROM catalog_item ci
           JOIN catalog c ON c.id=ci.catalog_id
           JOIN media_item m ON m.id=ci.media_id
         WHERE m.source_id=?1 AND c.kind='manual'
         GROUP BY c.id ORDER BY c.name COLLATE NOCASE",
    )?;
    let catalogs = q
        .query_map([source_id], |r| {
            Ok(CatalogLoss {
                name: r.get(0)?,
                items: r.get(1)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    let mut q = c.prepare(
        "SELECT p.name FROM person p
         WHERE p.name IS NOT NULL
           AND EXISTS (SELECT 1 FROM face f JOIN media_item m ON m.id=f.media_id
                       WHERE f.person_id=p.id AND m.source_id=?1)
           AND NOT EXISTS (SELECT 1 FROM face f JOIN media_item m ON m.id=f.media_id
                           WHERE f.person_id=p.id AND m.source_id IS NOT ?1)
         ORDER BY p.name COLLATE NOCASE",
    )?;
    let orphaned_people = q
        .query_map([source_id], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    Ok(RemovalSummary {
        items,
        edited,
        catalogs,
        orphaned_people,
        jobs: 0,
    })
}

/// Miniature dei file indicati che nessun altro elemento usa (duplicati esatti
/// condividono la stessa miniatura).
fn orphan_thumbs(c: &Connection, keys: HashSet<String>) -> StoreResult<Vec<String>> {
    let mut used = c.prepare_cached("SELECT EXISTS(SELECT 1 FROM media_item WHERE thumb=?1)")?;
    let mut out = Vec::new();
    for k in keys {
        if !used.query_row([&k], |r| r.get::<_, bool>(0))? {
            out.push(k);
        }
    }
    out.sort();
    Ok(out)
}

/// Elimina le persone senza nome né identità che non hanno più volti.
pub fn prune_unnamed_people(c: &Connection) -> StoreResult<usize> {
    let n = c.execute(
        "DELETE FROM person WHERE name IS NULL AND identity_uid IS NULL
           AND NOT EXISTS (SELECT 1 FROM face f WHERE f.person_id=person.id)",
        [],
    )?;
    if n > 0 {
        crate::people::refresh_centroids(c)?;
    }
    Ok(n)
}

fn thumbs_of(c: &Connection, ids: &[MediaId]) -> StoreResult<HashSet<String>> {
    let mut q = c.prepare_cached("SELECT thumb FROM media_item WHERE id=?1")?;
    let mut keys = HashSet::new();
    for id in ids {
        if let Some(Some(k)) = q
            .query_row([id], |r| r.get::<_, Option<String>>(0))
            .optional()?
        {
            keys.insert(k);
        }
    }
    Ok(keys)
}

/// Rimuove elementi dalla libreria (mai dal disco) con i loro dati derivati.
pub fn remove_media(c: &Connection, ids: &[MediaId]) -> StoreResult<Removed> {
    let keys = thumbs_of(c, ids)?;
    let items = repo::media::delete(c, ids)?;
    let thumbs = orphan_thumbs(c, keys)?;
    let people = prune_unnamed_people(c)?;
    Ok(Removed {
        items,
        thumbs,
        people,
    })
}

/// Rimuove la sorgente con tutti i suoi elementi e i dati derivati.
pub fn remove_source(c: &Connection, source_id: i64) -> StoreResult<Removed> {
    let mut q = c.prepare(
        "SELECT DISTINCT thumb FROM media_item WHERE source_id=?1 AND thumb IS NOT NULL",
    )?;
    let keys: HashSet<String> = q
        .query_map([source_id], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let items: i64 = c.query_row(
        "SELECT count(*) FROM media_item WHERE source_id=?1",
        [source_id],
        |r| r.get(0),
    )?;
    repo::source::delete(c, source_id)?;
    forget_update(c, source_id)?;
    let thumbs = orphan_thumbs(c, keys)?;
    let people = prune_unnamed_people(c)?;
    Ok(Removed {
        items: items as usize,
        thumbs,
        people,
    })
}

/// Chiavi delle miniature usate da almeno un elemento.
pub fn referenced_thumbs(c: &Connection) -> StoreResult<HashSet<String>> {
    let mut q = c.prepare("SELECT DISTINCT thumb FROM media_item WHERE thumb IS NOT NULL")?;
    let keys = q.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
    Ok(keys)
}

/// Il percorso sta dentro `folder` (o coincide con essa); senza maiuscole su Windows.
pub fn path_in_folder(path: &str, folder: &str) -> bool {
    let folder = folder.trim_end_matches(['\\', '/']);
    let (p, f) = if cfg!(windows) {
        (path.to_lowercase(), folder.to_lowercase())
    } else {
        (path.to_string(), folder.to_string())
    };
    p == f
        || (p.len() > f.len() && p.starts_with(&f) && matches!(p.as_bytes()[f.len()], b'\\' | b'/'))
}

/// Elementi mancanti di una sorgente, eventualmente solo dentro `folder`.
pub fn missing(
    c: &Connection,
    source_id: i64,
    folder: Option<&str>,
) -> StoreResult<Vec<(MediaId, String)>> {
    let mut q = c.prepare(
        "SELECT id, path FROM media_item WHERE source_id=?1 AND status='missing' ORDER BY path",
    )?;
    let rows: Vec<(MediaId, String)> = q
        .query_map([source_id], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    Ok(rows
        .into_iter()
        .filter(|(_, p)| folder.is_none_or(|f| path_in_folder(p, f)))
        .collect())
}

/// Cartella comune più profonda dei percorsi indicati (proposta per "Ricollega").
pub fn common_folder(paths: &[String]) -> Option<String> {
    let first = paths.first()?;
    let sep = if first.contains('\\') { '\\' } else { '/' };
    let parent = |p: &str| p.rsplit_once(['\\', '/']).map(|(d, _)| d.to_string());
    let mut common = parent(first)?;
    for p in &paths[1..] {
        while !path_in_folder(p, &common) {
            common = parent(&common)?;
        }
    }
    if common.is_empty() {
        common.push(sep);
    }
    Some(common)
}

/// File modificato: si scarta l'analisi automatica e restano i dati manuali
/// (etichette manuali, tag, cataloghi, luogo manuale, volti assegnati a mano).
/// L'elemento torna "da analizzare".
pub fn reset_analysis(c: &Connection, id: MediaId) -> StoreResult<()> {
    c.execute("DELETE FROM analysis_version WHERE media_id=?1", [id])?;
    c.execute(
        "DELETE FROM label WHERE media_id=?1 AND origin<>'manual'",
        [id],
    )?;
    c.execute("DELETE FROM media_people WHERE media_id=?1", [id])?;
    c.execute("DELETE FROM embedding WHERE media_id=?1", [id])?;
    c.execute(
        "DELETE FROM face WHERE media_id=?1 AND assigned_by IS NOT 'manual'",
        [id],
    )?;
    c.execute("DELETE FROM face_exclusion WHERE media_id=?1", [id])?;
    prune_unnamed_people(c)?;
    Ok(())
}

/// Rimuove i mancanti di una sorgente.
pub fn remove_missing(c: &Connection, source_id: i64) -> StoreResult<Removed> {
    let ids: Vec<MediaId> = missing(c, source_id, None)?
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    remove_media(c, &ids)
}

// ------------------------------------------------------- riepilogo aggiornamento

/// Oltre questo numero di file per gruppo si conservano solo i contatori.
pub const MAX_SUMMARY_IDS: usize = 10_000;

/// Un gruppo del riepilogo: quanti file e (se non troppi) quali.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ChangeGroup {
    pub count: i64,
    /// Vuoto se `count` supera [`MAX_SUMMARY_IDS`].
    pub ids: Vec<MediaId>,
}

impl ChangeGroup {
    pub fn push(&mut self, id: MediaId) {
        self.count += 1;
        if self.ids.len() < MAX_SUMMARY_IDS {
            self.ids.push(id);
        }
    }
    /// Gli id sono completi (si può aprire la libreria su questo gruppo).
    pub fn complete(&self) -> bool {
        self.ids.len() as i64 == self.count
    }
    fn finish(&mut self) {
        if !self.complete() {
            self.ids.clear();
        }
    }
}

/// Riepilogo dell'ultimo aggiornamento di una sorgente.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct UpdateSummary {
    pub source_id: i64,
    pub job_id: i64,
    pub finished_at: i64,
    /// Sottocartella aggiornata (`None` = intera sorgente).
    pub folder: Option<String>,
    pub new: ChangeGroup,
    pub modified: ChangeGroup,
    pub moved: ChangeGroup,
    pub missing: ChangeGroup,
    pub restored: ChangeGroup,
    pub unchanged: i64,
    pub failed: i64,
}

impl UpdateSummary {
    pub fn no_changes(&self) -> bool {
        self.new.count
            + self.modified.count
            + self.moved.count
            + self.missing.count
            + self.restored.count
            == 0
    }
}

fn summary_key(source_id: i64) -> String {
    format!("import.last_update.{source_id}")
}

pub fn save_update(c: &Connection, s: &UpdateSummary) -> StoreResult<()> {
    let mut s = s.clone();
    for g in [
        &mut s.new,
        &mut s.modified,
        &mut s.moved,
        &mut s.missing,
        &mut s.restored,
    ] {
        g.finish();
    }
    if s.finished_at == 0 {
        s.finished_at = now_ms();
    }
    c.execute(
        "INSERT INTO settings_kv(key,value) VALUES (?1,?2)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![summary_key(s.source_id), serde_json::to_string(&s)?],
    )?;
    Ok(())
}

/// Riepiloghi salvati, uno per sorgente esistente.
pub fn updates(c: &Connection) -> StoreResult<Vec<UpdateSummary>> {
    let mut q = c.prepare(
        "SELECT kv.value FROM settings_kv kv JOIN source s
           ON kv.key = 'import.last_update.' || s.id ORDER BY s.id",
    )?;
    let rows: Vec<String> = q.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
    Ok(rows
        .iter()
        .filter_map(|v| serde_json::from_str(v).ok())
        .collect())
}

pub fn forget_update(c: &Connection, source_id: i64) -> StoreResult<()> {
    c.execute(
        "DELETE FROM settings_kv WHERE key=?1",
        [summary_key(source_id)],
    )?;
    Ok(())
}

/// Ricollega un mancante a un nuovo percorso (anche in un'altra sorgente),
/// conservando catalogazioni, tag, cataloghi e persone.
pub fn relink_item(
    c: &Connection,
    id: MediaId,
    path: &str,
    mtime: i64,
    source_id: Option<i64>,
) -> StoreResult<()> {
    let name = path.rsplit(['\\', '/']).next().unwrap_or(path).to_string();
    repo::media::relink(c, id, path, &name, mtime)?;
    c.execute(
        "UPDATE media_item SET source_id=?2 WHERE id=?1",
        params![id, source_id],
    )?;
    repo::fts::refresh(c, id)
}

/// Sorgente più specifica che contiene il percorso.
pub fn source_containing(c: &Connection, path: &str) -> StoreResult<Option<i64>> {
    Ok(repo::source::list(c)?
        .into_iter()
        .filter(|s| path_in_folder(path, &s.path))
        .max_by_key(|s| s.path.len())
        .map(|s| s.id))
}

/// Gli elementi fuori dalla cartella della propria sorgente e dentro `source_id`
/// passano a quella sorgente (dopo un ricollegamento verso una nuova sorgente).
pub fn adopt_strays(c: &Connection, source_id: i64) -> StoreResult<usize> {
    let target = repo::source::get(c, source_id)?;
    let sources: std::collections::HashMap<i64, String> = repo::source::list(c)?
        .into_iter()
        .map(|s| (s.id, s.path))
        .collect();
    let mut q =
        c.prepare("SELECT id, path, source_id FROM media_item WHERE source_id IS NOT ?1")?;
    let rows: Vec<(MediaId, String, Option<i64>)> = q
        .query_map([source_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    let mut n = 0;
    for (id, path, src) in rows {
        let outside_own = src
            .and_then(|s| sources.get(&s))
            .is_none_or(|own| !path_in_folder(&path, own));
        if outside_own && path_in_folder(&path, &target.path) {
            c.execute(
                "UPDATE media_item SET source_id=?2 WHERE id=?1",
                params![id, source_id],
            )?;
            n += 1;
        }
    }
    Ok(n)
}

/// Cambia la cartella di una sorgente (dopo averla spostata e ricollegata).
pub fn set_source_path(c: &Connection, source_id: i64, path: &str) -> StoreResult<()> {
    c.execute(
        "UPDATE source SET path=?2 WHERE id=?1",
        params![source_id, path],
    )?;
    Ok(())
}
