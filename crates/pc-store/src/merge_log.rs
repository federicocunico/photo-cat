//! Storico delle unioni di gruppi di volti (design D7): ogni unione, manuale, automatica o da
//! una risposta della coda, lascia una voce con i volti spostati e lo stato dei gruppi d'origine.
//! "Annulla" riporta i volti nei gruppi di prima, ricreandoli con lo stesso id se serve; per le
//! unioni automatiche aggiunge il vincolo "Persone diverse", così l'unione non si ripete.

use crate::people::{self, PersonSnapshot};
use crate::{bulk, now_ms, StoreError, StoreResult};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Giorni di storico consultabili in "Unioni recenti"; le voci più vecchie si eliminano.
pub const RETENTION_DAYS: i64 = 90;
const DAY_MS: i64 = 24 * 60 * 60 * 1000;

/// Origine di un'unione.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum MergeOrigin {
    Manual,
    Auto,
    Queue,
}

impl MergeOrigin {
    pub fn as_str(self) -> &'static str {
        match self {
            MergeOrigin::Manual => "manual",
            MergeOrigin::Auto => "auto",
            MergeOrigin::Queue => "queue",
        }
    }
    fn parse(s: &str) -> Self {
        match s {
            "auto" => MergeOrigin::Auto,
            "queue" => MergeOrigin::Queue,
            _ => MergeOrigin::Manual,
        }
    }
}

/// Stato di un volto prima dell'unione.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FaceBefore {
    pub id: i64,
    pub person: Option<i64>,
    pub assigned_by: Option<String>,
    /// JSON dei gruppi rifiutati, come salvato.
    pub rejected: String,
}

/// Gruppo d'origine di un'unione, con quanto serve per ricrearlo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceGroup {
    pub person: i64,
    pub name: Option<String>,
    pub identity_uid: Option<String>,
    pub ignored_at: Option<i64>,
    pub ignored_faces: i64,
    /// Copertina scelta dall'utente (ripristinata con il gruppo).
    pub cover_face_id: Option<i64>,
    /// Copertina mostrata nello storico: quella scelta o il volto migliore.
    pub cover: Option<i64>,
    pub faces: Vec<FaceBefore>,
}

/// Istantanea salvata in `merge_log.snapshot`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MergeSnapshot {
    pub sources: Vec<SourceGroup>,
    /// Nome e identità della destinazione prima dell'unione (possono arrivare da un'origine).
    pub target_name: Option<String>,
    pub target_identity: Option<String>,
}

/// Gruppo d'origine come lo mostra "Unioni recenti".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct MergeSourceView {
    pub person: i64,
    pub name: Option<String>,
    pub faces: i64,
    /// Volto di copertina (`face://<id>`).
    pub cover: Option<i64>,
}

/// Voce di "Unioni recenti".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct MergeEntry {
    pub id: i64,
    pub at: i64,
    pub origin: MergeOrigin,
    pub target: i64,
    pub target_name: Option<String>,
    pub target_cover: Option<i64>,
    pub sources: Vec<MergeSourceView>,
    pub faces: i64,
    pub undone_at: Option<i64>,
    /// `false` se già annullata o se nessun volto è più nella destinazione.
    pub undoable: bool,
}

fn best_face(c: &Connection, person: i64) -> StoreResult<Option<i64>> {
    Ok(c.query_row(
        "SELECT id FROM face WHERE person_id=?1 \
         ORDER BY COALESCE(quality_score, quality) DESC, id LIMIT 1",
        [person],
        |r| r.get(0),
    )
    .optional()?)
}

/// Copertina di un gruppo: quella scelta, altrimenti il volto migliore.
pub fn cover_of(c: &Connection, person: i64) -> StoreResult<Option<i64>> {
    let chosen: Option<i64> = c
        .query_row(
            "SELECT p.cover_face_id FROM person p JOIN face f ON f.id = p.cover_face_id \
             AND f.person_id = p.id WHERE p.id=?1",
            [person],
            |r| r.get(0),
        )
        .optional()?;
    match chosen {
        Some(f) => Ok(Some(f)),
        None => best_face(c, person),
    }
}

/// Stato attuale di un gruppo esistente e di tutti i suoi volti, prima di un'unione.
pub fn source_group(c: &Connection, person: i64) -> StoreResult<SourceGroup> {
    let snap = people::person_snapshot(c, person)?;
    let mut q = c.prepare_cached(
        "SELECT id, person_id, assigned_by, rejected FROM face WHERE person_id=?1 ORDER BY id",
    )?;
    let faces = q
        .query_map([person], |r| {
            Ok(FaceBefore {
                id: r.get(0)?,
                person: r.get(1)?,
                assigned_by: r.get(2)?,
                rejected: r.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SourceGroup {
        person,
        name: snap.name,
        identity_uid: snap.identity_uid,
        ignored_at: snap.ignored_at,
        ignored_faces: snap.ignored_faces,
        cover_face_id: snap.cover_face_id,
        cover: cover_of(c, person)?,
        faces,
    })
}

/// Stato dei volti indicati (volti di un gruppo che non esiste ancora, es. un gruppo del primo
/// passaggio unito subito dal secondo).
pub fn faces_before(c: &Connection, faces: &[i64]) -> StoreResult<Vec<FaceBefore>> {
    Ok(people::face_states(c, faces)?
        .into_iter()
        .map(|s| FaceBefore {
            id: s.id,
            person: s.person_id,
            assigned_by: s.assigned_by,
            rejected: s.rejected,
        })
        .collect())
}

/// Istantanea per unire i gruppi `from` in `to` (da prendere prima dell'unione).
pub fn snapshot_groups(c: &Connection, from: &[i64], to: i64) -> StoreResult<MergeSnapshot> {
    let target = people::person_snapshot(c, to)?;
    Ok(MergeSnapshot {
        sources: from
            .iter()
            .filter(|g| **g != to)
            .map(|g| source_group(c, *g))
            .collect::<StoreResult<_>>()?,
        target_name: target.name,
        target_identity: target.identity_uid,
    })
}

/// Registra un'unione già eseguita (o da eseguire nella stessa transazione) e restituisce l'id.
pub fn record(
    c: &Connection,
    origin: MergeOrigin,
    target: i64,
    snapshot: &MergeSnapshot,
) -> StoreResult<i64> {
    let faces: usize = snapshot.sources.iter().map(|s| s.faces.len()).sum();
    let at = now_ms();
    c.execute(
        "INSERT INTO merge_log(at, origin, target_person, faces, snapshot) VALUES (?1,?2,?3,?4,?5)",
        params![
            at,
            origin.as_str(),
            target,
            faces as i64,
            serde_json::to_string(snapshot)?
        ],
    )?;
    let id = c.last_insert_rowid();
    c.execute(
        "DELETE FROM merge_log WHERE at < ?1",
        [at - RETENTION_DAYS * DAY_MS],
    )?;
    Ok(id)
}

/// Segna come annullata una voce (annullamento con Ctrl+Z della correzione che l'ha creata).
pub fn mark_undone(c: &Connection, id: i64) -> StoreResult<()> {
    c.execute(
        "UPDATE merge_log SET undone_at=?2 WHERE id=?1 AND undone_at IS NULL",
        params![id, now_ms()],
    )?;
    Ok(())
}

fn faces_still_in(c: &Connection, ids: &[i64], target: i64) -> StoreResult<i64> {
    if ids.is_empty() {
        return Ok(0);
    }
    Ok(c.query_row(
        "SELECT count(*) FROM face WHERE person_id=?2 AND id IN (SELECT value FROM json_each(?1))",
        params![serde_json::to_string(ids)?, target],
        |r| r.get(0),
    )?)
}

/// Voci degli ultimi [`RETENTION_DAYS`] giorni, dalla più recente; con `person` solo quelle
/// che la riguardano come destinazione o come origine.
pub fn list(c: &Connection, person: Option<i64>, limit: i64) -> StoreResult<Vec<MergeEntry>> {
    let since = now_ms() - RETENTION_DAYS * DAY_MS;
    let mut q = c.prepare(
        "SELECT id, at, origin, target_person, faces, snapshot, undone_at FROM merge_log m \
         WHERE at >= ?1 AND (?2 IS NULL OR target_person = ?2 OR EXISTS ( \
             SELECT 1 FROM json_each(m.snapshot, '$.sources') s \
             WHERE json_extract(s.value, '$.person') = ?2)) \
         ORDER BY at DESC, id DESC LIMIT ?3",
    )?;
    let rows = q
        .query_map(params![since, person, limit.clamp(1, 1000)], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, Option<i64>>(6)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut out = Vec::with_capacity(rows.len());
    for (id, at, origin, target, faces, snapshot, undone_at) in rows {
        let snap: MergeSnapshot = serde_json::from_str(&snapshot)?;
        let ids: Vec<i64> = snap
            .sources
            .iter()
            .flat_map(|s| s.faces.iter().map(|f| f.id))
            .collect();
        let target_name: Option<String> = c
            .query_row("SELECT name FROM person WHERE id=?1", [target], |r| {
                r.get(0)
            })
            .optional()?
            .flatten()
            .or(snap.target_name.clone());
        out.push(MergeEntry {
            id,
            at,
            origin: MergeOrigin::parse(&origin),
            target,
            target_name,
            target_cover: cover_of(c, target)?,
            sources: snap
                .sources
                .iter()
                .map(|s| MergeSourceView {
                    person: s.person,
                    name: s.name.clone(),
                    faces: s.faces.len() as i64,
                    cover: s.cover,
                })
                .collect(),
            faces,
            undone_at,
            undoable: undone_at.is_none() && faces_still_in(c, &ids, target)? > 0,
        });
    }
    Ok(out)
}

/// Unioni automatiche degli ultimi [`RETENTION_DAYS`] giorni e quante sono state annullate.
pub fn auto_counts(c: &Connection) -> StoreResult<(i64, i64)> {
    let since = now_ms() - RETENTION_DAYS * DAY_MS;
    Ok(c.query_row(
        "SELECT count(*), COALESCE(sum(undone_at IS NOT NULL), 0) FROM merge_log \
         WHERE origin='auto' AND at >= ?1",
        [since],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?)
}

/// Annulla una voce dello storico. Se la correzione che l'ha creata è l'ultima operazione
/// annullabile, si annulla quella (stato completo: rifiuti, vincoli, nome); altrimenti si
/// riportano nei gruppi d'origine i volti ancora nella destinazione. Restituisce i volti
/// riportati.
pub fn undo(c: &Connection, id: i64) -> StoreResult<i64> {
    let (origin, target, snapshot, undone): (String, i64, String, Option<i64>) = c
        .query_row(
            "SELECT origin, target_person, snapshot, undone_at FROM merge_log WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?
        .ok_or_else(|| StoreError::NotFound("unione".into()))?;
    if undone.is_some() {
        return Err(StoreError::Invalid("unione già annullata".into()));
    }
    let origin = MergeOrigin::parse(&origin);
    let snap: MergeSnapshot = serde_json::from_str(&snapshot)?;
    let ids: Vec<i64> = snap
        .sources
        .iter()
        .flat_map(|s| s.faces.iter().map(|f| f.id))
        .collect();
    let still = faces_still_in(c, &ids, target)?;
    if still == 0 {
        return Err(StoreError::Invalid(
            "nessun volto dell'unione è ancora nella persona di destinazione".into(),
        ));
    }
    if bulk::last_undo_is_merge(c, id)? {
        bulk::undo_last(c)?;
        // L'annullamento segna la voce tramite `MergeLogged`; per sicurezza anche qui.
        mark_undone(c, id)?;
        return Ok(still);
    }
    bulk::forget_merge(c, id)?;
    let mut media = HashSet::new();
    let mut moved = 0i64;
    let mut q = c.prepare_cached("SELECT person_id, media_id FROM face WHERE id=?1")?;
    for s in &snap.sources {
        let mut back = Vec::new();
        for f in &s.faces {
            if let Some((person, media_id)) = q
                .query_row([f.id], |r| {
                    Ok((r.get::<_, Option<i64>>(0)?, r.get::<_, i64>(1)?))
                })
                .optional()?
            {
                if person == Some(target) {
                    back.push(f);
                    media.insert(media_id);
                }
            }
        }
        if back.is_empty() || s.person == target {
            continue;
        }
        let exists: bool = c.query_row(
            "SELECT EXISTS(SELECT 1 FROM person WHERE id=?1)",
            [s.person],
            |r| r.get(0),
        )?;
        if !exists {
            people::restore_person_row(
                c,
                &PersonSnapshot {
                    id: s.person,
                    name: s.name.clone(),
                    identity_uid: s.identity_uid.clone(),
                    ignored_at: s.ignored_at,
                    ignored_faces: s.ignored_faces,
                    cover_face_id: None,
                },
            )?;
        }
        for f in &back {
            c.execute(
                "UPDATE face SET person_id=?2, assigned_by=?3, rejected=?4 WHERE id=?1",
                params![f.id, s.person, f.assigned_by, f.rejected],
            )?;
        }
        if let Some(cover) = s.cover_face_id {
            c.execute(
                "UPDATE person SET cover_face_id=?2 WHERE id=?1 \
                 AND EXISTS(SELECT 1 FROM face WHERE id=?2 AND person_id=?1)",
                params![s.person, cover],
            )?;
        }
        if origin == MergeOrigin::Auto {
            // L'unione automatica annullata non si ripete: vincolo "Persone diverse".
            c.execute(
                "INSERT OR IGNORE INTO person_not_same(a, b, created_at) VALUES (?1, ?2, ?3)",
                params![s.person.min(target), s.person.max(target), now_ms()],
            )?;
        }
        // Nome e identità passati alla destinazione con l'unione tornano all'origine.
        if snap.target_name.is_none() && s.name.is_some() {
            c.execute(
                "UPDATE person SET name=NULL WHERE id=?1 AND name IS ?2",
                params![target, s.name],
            )?;
        }
        if snap.target_identity.is_none() && s.identity_uid.is_some() {
            c.execute(
                "UPDATE person SET identity_uid=NULL WHERE id=?1 AND identity_uid IS ?2",
                params![target, s.identity_uid],
            )?;
        }
        moved += back.len() as i64;
    }
    drop(q);
    c.execute(
        "UPDATE merge_log SET undone_at=?2 WHERE id=?1",
        params![id, now_ms()],
    )?;
    people::refresh_centroids(c)?;
    people::refresh_media_ids(c, &media)?;
    crate::review::cleanup(c)?;
    Ok(moved)
}
