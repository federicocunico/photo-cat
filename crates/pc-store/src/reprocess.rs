//! Supporto alla rianalisi per ambito (change `ai-workspace`, D3): id dei cataloghi e
//! azzeramento dei marcatori di fase per i soli id indicati.

use crate::coverage::Stage;
use crate::model::MediaId;
use crate::query::{self, LibraryFilter};
use crate::StoreResult;
use rusqlite::{params, Connection};

/// Unione degli id dei cataloghi (manuali: elementi; smart: regole), senza le foto dei
/// cataloghi di sistema e solo con file presente e miniatura (come la libreria intera).
pub fn catalog_media_ids(c: &Connection, catalog_ids: &[i64]) -> StoreResult<Vec<MediaId>> {
    let mut out: Vec<MediaId> = Vec::new();
    for &cid in catalog_ids {
        let f = LibraryFilter {
            catalog_id: Some(cid),
            ..Default::default()
        };
        out.extend(query::ids(c, &f)?);
    }
    out.sort_unstable();
    out.dedup();
    let mut ok = c.prepare_cached(
        "SELECT EXISTS(SELECT 1 FROM media_item WHERE id = ?1 AND status = 'ok' AND thumb IS NOT NULL)",
    )?;
    let mut keep = Vec::with_capacity(out.len());
    for id in out {
        if ok.query_row([id], |r| r.get::<_, bool>(0))? {
            keep.push(id);
        }
    }
    Ok(keep)
}

/// Azzera i marcatori delle fasi scelte per `ids`, così la rianalisi le rifà. Non tocca
/// etichette, volti, gruppi di simili, collegamenti né correzioni manuali.
pub fn clear_markers(c: &Connection, ids: &[MediaId], stages: &[Stage]) -> StoreResult<()> {
    for &stage in stages {
        match stage {
            Stage::Classification => crate::analysis::forget(c, ids, "clip")?,
            Stage::Persons => crate::analysis::forget(c, ids, "persons")?,
            Stage::Faces => crate::analysis::forget(c, ids, "faces")?,
            Stage::Similar => {
                let mut q = c.prepare_cached("DELETE FROM similar_done WHERE media_id = ?1")?;
                for id in ids {
                    q.execute(params![id])?;
                }
            }
            Stage::Phash => {
                let mut q = c.prepare_cached("UPDATE media_item SET phash = NULL WHERE id = ?1")?;
                for id in ids {
                    q.execute(params![id])?;
                }
            }
        }
    }
    Ok(())
}

/// Vero se almeno una delle foto indicate è ancora senza impronta percettiva.
pub fn any_without_phash(c: &Connection, ids: &[MediaId]) -> StoreResult<bool> {
    let mut q = c.prepare_cached(
        "SELECT EXISTS(SELECT 1 FROM media_item WHERE id = ?1 AND thumb IS NOT NULL AND phash IS NULL)",
    )?;
    for id in ids {
        if q.query_row([id], |r| r.get::<_, bool>(0))? {
            return Ok(true);
        }
    }
    Ok(false)
}
