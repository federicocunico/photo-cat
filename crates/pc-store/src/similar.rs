//! Gruppi di foto simili: archi confermati, vincoli dell'utente, gruppi e foto principale.
//!
//! Il calcolo dei gruppi (candidati, SSIM, union-find) sta in `pc-ai::similar`; qui ci sono
//! le letture e le scritture del catalogo. Nessuna funzione tocca i file su disco.

use crate::model::MediaId;
use crate::{now_ms, StoreResult};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Pavimento degli archi conservati: sotto non serve, nessuna soglia ammessa scende così in basso.
pub const EDGE_FLOOR: f32 = 0.6;

/// Archi confermati `(a, b, ssim)` con `a < b`.
pub fn edges(c: &Connection) -> StoreResult<Vec<(MediaId, MediaId, f32)>> {
    let mut q = c.prepare_cached("SELECT a, b, ssim FROM similar_edge")?;
    let rows = q
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get::<_, f64>(2)? as f32)))?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

/// Salva gli archi (sopra [`EDGE_FLOOR`]); un arco già presente viene aggiornato.
pub fn save_edges(c: &Connection, edges: &[(MediaId, MediaId, f32)]) -> StoreResult<()> {
    let mut st = c.prepare_cached(
        "INSERT INTO similar_edge(a, b, ssim) VALUES (?1, ?2, ?3)
         ON CONFLICT(a, b) DO UPDATE SET ssim = excluded.ssim",
    )?;
    for &(a, b, s) in edges {
        if s >= EDGE_FLOOR && a != b {
            let (a, b) = if a < b { (a, b) } else { (b, a) };
            st.execute(params![a, b, f64::from(s)])?;
        }
    }
    Ok(())
}

/// La foto è stata elaborata con questi parametri; misure della miniatura per la principale.
pub fn mark_done(
    c: &Connection,
    media: MediaId,
    params_key: &str,
    sharpness: f32,
    exposure: f32,
) -> StoreResult<()> {
    c.prepare_cached(
        "INSERT INTO similar_done(media_id, params, sharpness, exposure) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(media_id) DO UPDATE SET params = excluded.params,
           sharpness = excluded.sharpness, exposure = excluded.exposure",
    )?
    .execute(params![
        media,
        params_key,
        f64::from(sharpness),
        f64::from(exposure)
    ])?;
    Ok(())
}

/// Foto da elaborare: con embedding `model` e miniatura, non ancora fatte con `params_key`.
pub fn pending(c: &Connection, model: &str, params_key: &str) -> StoreResult<Vec<MediaId>> {
    let mut q = c.prepare_cached(
        "SELECT m.id FROM media_item m JOIN embedding e ON e.media_id = m.id AND e.model_ver = ?1
         LEFT JOIN similar_done d ON d.media_id = m.id
         WHERE m.status = 'ok' AND m.thumb IS NOT NULL AND (d.params IS NULL OR d.params <> ?2)
         ORDER BY m.id",
    )?;
    let rows = q
        .query_map(params![model, params_key], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

/// Dati per i candidati: chiave della miniatura, impronta, dispositivo e istante.
#[derive(Debug, Clone, Default)]
pub struct MediaFacts {
    pub thumb: Option<String>,
    pub content_hash: Option<String>,
    pub device: Option<String>,
    pub instant_utc: Option<i64>,
}

pub fn facts(c: &Connection, ids: &[MediaId]) -> StoreResult<HashMap<MediaId, MediaFacts>> {
    let mut out = HashMap::new();
    let mut q = c.prepare_cached(
        "SELECT m.id, m.thumb, m.content_hash, d.device, d.instant_utc
         FROM media_item m LEFT JOIN media_dates d ON d.media_id = m.id WHERE m.id = ?1",
    )?;
    for id in ids {
        if let Some(f) = q
            .query_row([id], |r| {
                Ok(MediaFacts {
                    thumb: r.get(1)?,
                    content_hash: r.get(2)?,
                    device: r.get(3)?,
                    instant_utc: r.get(4)?,
                })
            })
            .optional()?
        {
            out.insert(*id, f);
        }
    }
    Ok(out)
}

/// Foto dello stesso dispositivo entro `window_ms` dall'istante (raffiche), esclusa `id`.
pub fn burst_neighbours(
    c: &Connection,
    id: MediaId,
    device: &str,
    instant: i64,
    window_ms: i64,
) -> StoreResult<Vec<MediaId>> {
    let mut q = c.prepare_cached(
        "SELECT d.media_id FROM media_dates d JOIN media_item m ON m.id = d.media_id
         WHERE d.device = ?1 AND d.instant_utc BETWEEN ?2 AND ?3 AND d.media_id <> ?4
           AND m.status = 'ok' AND m.thumb IS NOT NULL",
    )?;
    let rows = q
        .query_map(
            params![device, instant - window_ms, instant + window_ms, id],
            |r| r.get(0),
        )?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

/// Copie esatte (stessa impronta del contenuto), esclusa `id`.
pub fn exact_copies(c: &Connection, id: MediaId, hash: &str) -> StoreResult<Vec<MediaId>> {
    let mut q = c.prepare_cached(
        "SELECT id FROM media_item WHERE content_hash = ?1 AND id <> ?2 AND status = 'ok'",
    )?;
    let rows = q
        .query_map(params![hash, id], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

/// Embedding di alcune foto per `model`.
pub fn embeddings_of(
    c: &Connection,
    model: &str,
    ids: &[MediaId],
) -> StoreResult<HashMap<MediaId, Vec<f32>>> {
    let mut q = c
        .prepare_cached("SELECT vector FROM embedding WHERE media_id = ?1 AND model_ver = ?2")?;
    let mut out = HashMap::new();
    for id in ids {
        if let Some(b) = q
            .query_row(params![id, model], |r| r.get::<_, Vec<u8>>(0))
            .optional()?
        {
            out.insert(
                *id,
                b.as_chunks::<4>()
                    .0
                    .iter()
                    .map(|x| f32::from_le_bytes(*x))
                    .collect(),
            );
        }
    }
    Ok(out)
}

/// Vincoli: `(a, b, must, istante)` e gruppi sciolti `(foto, token)`.
pub type RawConstraints = (Vec<(MediaId, MediaId, bool, i64)>, Vec<(MediaId, i64)>);

pub fn constraints(c: &Connection) -> StoreResult<RawConstraints> {
    let links = c
        .prepare_cached("SELECT a, b, kind = 'must', created_at FROM similar_link")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
        .collect::<Result<_, _>>()?;
    let dissolved = c
        .prepare_cached("SELECT media_id, token FROM similar_dissolved")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    Ok((links, dissolved))
}

fn set_link(c: &Connection, a: MediaId, b: MediaId, must: bool, at: i64) -> StoreResult<()> {
    if a == b {
        return Ok(());
    }
    let (a, b) = if a < b { (a, b) } else { (b, a) };
    c.prepare_cached(
        "INSERT INTO similar_link(a, b, kind, created_at) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(a, b) DO UPDATE SET kind = excluded.kind, created_at = excluded.created_at",
    )?
    .execute(params![a, b, if must { "must" } else { "cannot" }, at])?;
    Ok(())
}

/// Gruppo della foto e membri, se c'è.
pub fn members_with(c: &Connection, media: MediaId) -> StoreResult<Option<(i64, Vec<MediaId>)>> {
    let Some(group) = c
        .query_row(
            "SELECT group_id FROM similar_member WHERE media_id = ?1",
            [media],
            |r| r.get::<_, i64>(0),
        )
        .optional()?
    else {
        return Ok(None);
    };
    Ok(Some((group, group_members(c, group)?)))
}

pub fn group_members(c: &Connection, group: i64) -> StoreResult<Vec<MediaId>> {
    let rows = c
        .prepare_cached("SELECT media_id FROM similar_member WHERE group_id = ?1 ORDER BY media_id")?
        .query_map([group], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

/// "Togli dal gruppo": la foto non si unisce più agli altri membri.
pub fn remove_from_group(c: &Connection, media: MediaId) -> StoreResult<()> {
    let Some((_, members)) = members_with(c, media)? else {
        return Ok(());
    };
    let at = now_ms();
    for m in members.iter().filter(|m| **m != media) {
        set_link(c, media, *m, false, at)?;
    }
    Ok(())
}

/// "Raggruppa come simili": le foto restano unite anche dopo nuove analisi.
pub fn group_together(c: &Connection, ids: &[MediaId]) -> StoreResult<()> {
    let at = now_ms();
    for w in ids.windows(2) {
        set_link(c, w[0], w[1], true, at)?;
    }
    Ok(())
}

/// "Sciogli gruppo": le foto non si raggruppano più tra loro automaticamente, e le unioni
/// manuali tra loro vengono annullate.
pub fn dissolve(c: &Connection, group: i64) -> StoreResult<Vec<MediaId>> {
    let members = group_members(c, group)?;
    let token: i64 = c.query_row(
        "SELECT COALESCE(MAX(token), 0) + 1 FROM similar_dissolved",
        [],
        |r| r.get(0),
    )?;
    let at = now_ms();
    let set: HashSet<MediaId> = members.iter().copied().collect();
    for m in &members {
        c.execute(
            "INSERT OR IGNORE INTO similar_dissolved(media_id, token, created_at) VALUES (?1, ?2, ?3)",
            params![m, token, at],
        )?;
    }
    let musts: Vec<(MediaId, MediaId)> = c
        .prepare("SELECT a, b FROM similar_link WHERE kind = 'must'")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    for (a, b) in musts {
        if set.contains(&a) && set.contains(&b) {
            c.execute("DELETE FROM similar_link WHERE a = ?1 AND b = ?2", params![a, b])?;
        }
    }
    Ok(members)
}

/// Imposta la principale scelta dall'utente (prevale sulle analisi successive).
pub fn set_cover(c: &Connection, group: i64, media: MediaId) -> StoreResult<()> {
    let n = c.execute(
        "UPDATE similar_group SET cover_media_id = ?2, cover_manual = 1, updated_at = ?3
         WHERE id = ?1 AND EXISTS (SELECT 1 FROM similar_member WHERE group_id = ?1 AND media_id = ?2)",
        params![group, media, now_ms()],
    )?;
    if n == 0 {
        return Err(crate::StoreError::Invalid(
            "la foto non fa parte del gruppo".into(),
        ));
    }
    Ok(())
}

/// Misure per la principale: (nitidezza, esposizione).
fn quality(c: &Connection, ids: &[MediaId]) -> StoreResult<HashMap<MediaId, (f32, f32)>> {
    let mut q = c.prepare_cached(
        "SELECT sharpness, exposure FROM similar_done WHERE media_id = ?1",
    )?;
    let mut out = HashMap::new();
    for id in ids {
        if let Some(v) = q
            .query_row([id], |r| {
                Ok((r.get::<_, f64>(0)? as f32, r.get::<_, f64>(1)? as f32))
            })
            .optional()?
        {
            out.insert(*id, v);
        }
    }
    Ok(out)
}

/// Punteggio per la principale: nitidezza normalizzata nel gruppo (×0,7) ed esposizione (×0,3).
pub fn cover_scores(c: &Connection, members: &[MediaId]) -> StoreResult<HashMap<MediaId, f32>> {
    let q = quality(c, members)?;
    let max = q.values().map(|v| v.0).fold(0.0f32, f32::max);
    Ok(members
        .iter()
        .map(|m| {
            let (s, e) = q.get(m).copied().unwrap_or((0.0, 1.0));
            let s = if max > 0.0 { (s / max).clamp(0.0, 1.0) } else { 0.0 };
            (*m, 0.7 * s + 0.3 * e.clamp(0.0, 1.0))
        })
        .collect())
}

/// Esito del confronto tra i gruppi calcolati e quelli salvati.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ApplyOutcome {
    pub unchanged: usize,
    pub updated: usize,
    pub created: usize,
    pub removed: usize,
}

/// Sostituisce i gruppi salvati con quelli calcolati, scrivendo solo quelli cambiati.
/// Un gruppo che conserva gran parte dei membri mantiene id e principale manuale.
pub fn apply_groups(
    c: &Connection,
    groups: &[Vec<MediaId>],
    params_key: &str,
) -> StoreResult<ApplyOutcome> {
    let mut existing: HashMap<i64, (Vec<MediaId>, Option<MediaId>, bool)> = HashMap::new();
    {
        let mut q = c.prepare(
            "SELECT g.id, g.cover_media_id, g.cover_manual, m.media_id
             FROM similar_group g LEFT JOIN similar_member m ON m.group_id = g.id ORDER BY m.media_id",
        )?;
        let rows = q.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, Option<MediaId>>(1)?,
                r.get::<_, bool>(2)?,
                r.get::<_, Option<MediaId>>(3)?,
            ))
        })?;
        for row in rows {
            let (g, cover, manual, m) = row?;
            let e = existing.entry(g).or_insert((Vec::new(), cover, manual));
            if let Some(m) = m {
                e.0.push(m);
            }
        }
    }
    let mut by_member: HashMap<MediaId, i64> = HashMap::new();
    for (g, (members, _, _)) in &existing {
        for m in members {
            by_member.insert(*m, *g);
        }
    }
    let now = now_ms();
    let mut out = ApplyOutcome::default();
    let mut kept: HashSet<i64> = HashSet::new();
    for members in groups {
        // Gruppo esistente con più membri in comune (non ancora usato).
        let mut overlap: HashMap<i64, usize> = HashMap::new();
        for m in members {
            if let Some(g) = by_member.get(m) {
                *overlap.entry(*g).or_default() += 1;
            }
        }
        let best = overlap
            .into_iter()
            .filter(|(g, _)| !kept.contains(g))
            .max_by_key(|(g, n)| (*n, std::cmp::Reverse(*g)))
            .map(|(g, _)| g);
        let scores = cover_scores(c, members)?;
        let auto_cover = members
            .iter()
            .copied()
            .max_by(|a, b| scores[a].total_cmp(&scores[b]).then(b.cmp(a)))
            .expect("gruppo non vuoto");
        match best {
            Some(g) if existing[&g].0 == *members => {
                kept.insert(g);
                let (_, cover, manual) = existing[&g];
                // Principale automatica aggiornata se le misure sono cambiate.
                if !manual && cover != Some(auto_cover) {
                    c.execute(
                        "UPDATE similar_group SET cover_media_id = ?2, updated_at = ?3 WHERE id = ?1",
                        params![g, auto_cover, now],
                    )?;
                    out.updated += 1;
                } else {
                    out.unchanged += 1;
                }
            }
            Some(g) => {
                kept.insert(g);
                let (_, cover, manual) = existing[&g];
                let keep_manual = manual && cover.is_some_and(|c| members.contains(&c));
                let cover = if keep_manual { cover } else { Some(auto_cover) };
                c.execute(
                    "UPDATE similar_group SET cover_media_id = ?2, cover_manual = ?3, params = ?4,
                       updated_at = ?5 WHERE id = ?1",
                    params![g, cover, keep_manual, params_key, now],
                )?;
                c.execute("DELETE FROM similar_member WHERE group_id = ?1", [g])?;
                insert_members(c, g, members, &scores)?;
                out.updated += 1;
            }
            None => {
                c.execute(
                    "INSERT INTO similar_group(cover_media_id, cover_manual, params, updated_at)
                     VALUES (?1, 0, ?2, ?3)",
                    params![auto_cover, params_key, now],
                )?;
                let g = c.last_insert_rowid();
                kept.insert(g);
                insert_members(c, g, members, &scores)?;
                out.created += 1;
            }
        }
    }
    for g in existing.keys().filter(|g| !kept.contains(g)) {
        c.execute("DELETE FROM similar_group WHERE id = ?1", [g])?;
        out.removed += 1;
    }
    Ok(out)
}

fn insert_members(
    c: &Connection,
    group: i64,
    members: &[MediaId],
    scores: &HashMap<MediaId, f32>,
) -> StoreResult<()> {
    let mut st = c.prepare_cached(
        "INSERT INTO similar_member(media_id, group_id, score) VALUES (?1, ?2, ?3)
         ON CONFLICT(media_id) DO UPDATE SET group_id = excluded.group_id, score = excluded.score",
    )?;
    for m in members {
        st.execute(params![
            m,
            group,
            f64::from(scores.get(m).copied().unwrap_or(0.0))
        ])?;
    }
    Ok(())
}

/// Foto simili mostrate nell'Inspector.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SimilarGroupView {
    pub id: i64,
    pub cover: Option<MediaId>,
    pub cover_manual: bool,
    pub members: Vec<SimilarMember>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SimilarMember {
    pub id: MediaId,
    pub name: String,
    pub thumb: Option<String>,
}

pub fn group_view(c: &Connection, media: MediaId) -> StoreResult<Option<SimilarGroupView>> {
    let Some((group, _)) = members_with(c, media)? else {
        return Ok(None);
    };
    let (cover, cover_manual) = c.query_row(
        "SELECT cover_media_id, cover_manual FROM similar_group WHERE id = ?1",
        [group],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let members = c
        .prepare_cached(
            "SELECT m.id, m.name, m.thumb FROM similar_member s JOIN media_item m ON m.id = s.media_id
             WHERE s.group_id = ?1 ORDER BY s.score DESC, m.id",
        )?
        .query_map([group], |r| {
            Ok(SimilarMember {
                id: r.get(0)?,
                name: r.get(1)?,
                thumb: r.get(2)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(Some(SimilarGroupView {
        id: group,
        cover,
        cover_manual,
        members,
    }))
}

/// Numero di gruppi salvati.
pub fn group_count(c: &Connection) -> StoreResult<i64> {
    Ok(c.query_row("SELECT count(*) FROM similar_group", [], |r| r.get(0))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db(n: i64) -> Connection {
        let mut c = Connection::open_in_memory().unwrap();
        crate::migrate::migrate(&mut c, None).unwrap();
        for i in 1..=n {
            c.execute(
                "INSERT INTO media_item(id,path,name,ext,size,mtime,added_at,updated_at)
                 VALUES (?1, ?2, ?2, 'jpg', 1, 0, 0, 0)",
                params![i, format!("{i}.jpg")],
            )
            .unwrap();
        }
        c
    }

    #[test]
    fn unchanged_groups_are_not_rewritten_and_manual_cover_survives() {
        let c = db(6);
        mark_done(&c, 1, "p", 10.0, 1.0).unwrap();
        mark_done(&c, 2, "p", 50.0, 1.0).unwrap();
        mark_done(&c, 3, "p", 20.0, 1.0).unwrap();
        let o = apply_groups(&c, &[vec![1, 2, 3], vec![4, 5]], "p").unwrap();
        assert_eq!(o.created, 2);
        let (g, _) = members_with(&c, 1).unwrap().unwrap();
        let v = group_view(&c, 1).unwrap().unwrap();
        assert_eq!(v.cover, Some(2), "la più nitida è la principale");
        set_cover(&c, g, 3).unwrap();
        let o = apply_groups(&c, &[vec![1, 2, 3], vec![4, 5]], "p").unwrap();
        assert_eq!(o.unchanged, 2, "nessuna scrittura per i gruppi invariati");
        // Un nuovo membro: stesso gruppo, principale manuale conservata.
        let o = apply_groups(&c, &[vec![1, 2, 3, 6], vec![4, 5]], "p").unwrap();
        assert_eq!((o.updated, o.unchanged), (1, 1));
        let v = group_view(&c, 6).unwrap().unwrap();
        assert_eq!(v.id, g);
        assert_eq!(v.cover, Some(3));
        assert!(v.cover_manual);
        // Gruppi scomparsi vengono rimossi.
        let o = apply_groups(&c, &[vec![1, 2, 3, 6]], "p").unwrap();
        assert_eq!(o.removed, 1);
        assert!(members_with(&c, 4).unwrap().is_none());
    }

    #[test]
    fn corrections_are_stored_as_constraints() {
        let c = db(5);
        apply_groups(&c, &[vec![1, 2, 3]], "p").unwrap();
        remove_from_group(&c, 3).unwrap();
        group_together(&c, &[4, 5]).unwrap();
        let (links, _) = constraints(&c).unwrap();
        assert!(links.contains(&(1, 3, false, links[0].3)) || links.iter().any(|l| l.0 == 1 && l.1 == 3 && !l.2));
        assert!(links.iter().any(|l| l.0 == 4 && l.1 == 5 && l.2));
        let (g, _) = members_with(&c, 1).unwrap().unwrap();
        group_together(&c, &[1, 2]).unwrap();
        let members = dissolve(&c, g).unwrap();
        assert_eq!(members, vec![1, 2, 3]);
        let (links, dissolved) = constraints(&c).unwrap();
        assert!(!links.iter().any(|l| l.0 == 1 && l.1 == 2 && l.2), "unione annullata");
        assert_eq!(dissolved.len(), 3);
        assert!(set_cover(&c, g, 5).is_err());
    }
}
