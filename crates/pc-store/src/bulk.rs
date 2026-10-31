//! Modifiche in blocco a tag, cataloghi e catalogazioni, con registrazione
//! dell'operazione inversa in `undo_log` per "Annulla".

use crate::model::MediaId;
use crate::repo::{catalog, fts, tag};
use crate::{now_ms, StoreError, StoreResult};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

/// Numero di operazioni annullabili conservate.
const UNDO_DEPTH: i64 = 20;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LabelRow {
    pub media_id: MediaId,
    pub value: String,
    pub score: f64,
    pub origin: String,
    pub model_ver: Option<String>,
    pub taxonomy_ver: Option<i64>,
}

/// Come ripristinare lo stato precedente.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UndoAction {
    Group {
        actions: Vec<UndoAction>,
    },
    TagsAdded {
        tag_id: i64,
        ids: Vec<MediaId>,
    },
    TagsRemoved {
        tag_id: i64,
        ids: Vec<MediaId>,
    },
    CatalogAdded {
        catalog_id: i64,
        ids: Vec<MediaId>,
    },
    CatalogRemoved {
        catalog_id: i64,
        ids: Vec<MediaId>,
    },
    LabelsReplaced {
        group: String,
        ids: Vec<MediaId>,
        previous: Vec<LabelRow>,
    },
}

thread_local! { static GROUPING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }
/// Un'unica operazione annullabile anche quando un job lavora in blocchi.
pub fn with_undo_group<T>(
    c: &Connection,
    description: &str,
    work: impl FnOnce() -> StoreResult<T>,
) -> StoreResult<T> {
    let first: i64 = c.query_row("SELECT COALESCE(MAX(id),0) FROM undo_log", [], |r| r.get(0))?;
    struct Reset(bool);
    impl Drop for Reset {
        fn drop(&mut self) {
            GROUPING.set(self.0);
        }
    }
    let prior = GROUPING.replace(true);
    let reset = Reset(prior);
    let result = work();
    drop(reset);
    let mut q = c.prepare("SELECT payload FROM undo_log WHERE id>?1 ORDER BY id")?;
    let payloads = q
        .query_map([first], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    let actions = payloads
        .iter()
        .map(|p| serde_json::from_str(p))
        .collect::<Result<Vec<_>, _>>()?;
    c.execute("DELETE FROM undo_log WHERE id>?1", [first])?;
    if !actions.is_empty() {
        record(c, description, &UndoAction::Group { actions })?;
    }
    result
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct BulkOutcome {
    /// Elementi effettivamente modificati.
    pub changed: usize,
    /// Elementi richiesti.
    pub requested: usize,
}

fn record(c: &Connection, description: &str, action: &UndoAction) -> StoreResult<()> {
    c.execute(
        "INSERT INTO undo_log(ts, description, payload) VALUES (?1, ?2, ?3)",
        params![now_ms(), description, serde_json::to_string(action)?],
    )?;
    if !GROUPING.get() {
        c.execute(
        "DELETE FROM undo_log WHERE id NOT IN (SELECT id FROM undo_log ORDER BY id DESC LIMIT ?1)",
        [UNDO_DEPTH],
    )?;
    }
    Ok(())
}

fn refresh_all(c: &Connection, ids: &[MediaId]) -> StoreResult<()> {
    ids.iter().try_for_each(|id| fts::refresh(c, *id))
}

pub fn add_tag(c: &Connection, name: &str, ids: &[MediaId]) -> StoreResult<BulkOutcome> {
    let tag_id = tag::ensure(c, name)?;
    let added = tag::add(c, tag_id, ids)?;
    refresh_all(c, &added)?;
    record(
        c,
        &format!("Aggiungi tag \"{name}\""),
        &UndoAction::TagsAdded {
            tag_id,
            ids: added.clone(),
        },
    )?;
    Ok(BulkOutcome {
        changed: added.len(),
        requested: ids.len(),
    })
}

pub fn remove_tag(c: &Connection, name: &str, ids: &[MediaId]) -> StoreResult<BulkOutcome> {
    let Some(tag_id) = c
        .query_row("SELECT id FROM tag WHERE name = ?1", [name], |r| {
            r.get::<_, i64>(0)
        })
        .optional()?
    else {
        return Ok(BulkOutcome {
            changed: 0,
            requested: ids.len(),
        });
    };
    let removed = tag::remove(c, tag_id, ids)?;
    refresh_all(c, &removed)?;
    record(
        c,
        &format!("Rimuovi tag \"{name}\""),
        &UndoAction::TagsRemoved {
            tag_id,
            ids: removed.clone(),
        },
    )?;
    Ok(BulkOutcome {
        changed: removed.len(),
        requested: ids.len(),
    })
}

pub fn add_to_catalog(
    c: &Connection,
    catalog_id: i64,
    ids: &[MediaId],
) -> StoreResult<BulkOutcome> {
    let name = catalog::get(c, catalog_id)?.name;
    let added = catalog::add_items(c, catalog_id, ids)?;
    record(
        c,
        &format!("Aggiungi a \"{name}\""),
        &UndoAction::CatalogAdded {
            catalog_id,
            ids: added.clone(),
        },
    )?;
    Ok(BulkOutcome {
        changed: added.len(),
        requested: ids.len(),
    })
}

pub fn remove_from_catalog(
    c: &Connection,
    catalog_id: i64,
    ids: &[MediaId],
) -> StoreResult<BulkOutcome> {
    let name = catalog::get(c, catalog_id)?.name;
    let removed = catalog::remove_items(c, catalog_id, ids)?;
    record(
        c,
        &format!("Rimuovi da \"{name}\""),
        &UndoAction::CatalogRemoved {
            catalog_id,
            ids: removed.clone(),
        },
    )?;
    Ok(BulkOutcome {
        changed: removed.len(),
        requested: ids.len(),
    })
}

fn labels_of_group(c: &Connection, group: &str, ids: &[MediaId]) -> StoreResult<Vec<LabelRow>> {
    let mut st = c.prepare_cached(
        "SELECT media_id, value, score, origin, model_ver, taxonomy_ver FROM label WHERE media_id = ?1 AND grp = ?2",
    )?;
    let mut out = Vec::new();
    for id in ids {
        let rows = st.query_map(params![id, group], |r| {
            Ok(LabelRow {
                media_id: r.get(0)?,
                value: r.get(1)?,
                score: r.get(2)?,
                origin: r.get(3)?,
                model_ver: r.get(4)?,
                taxonomy_ver: r.get(5)?,
            })
        })?;
        for row in rows {
            out.push(row?);
        }
    }
    Ok(out)
}

/// Imposta come correzione manuale il valore di un gruppo di catalogazione
/// (sostituendo i valori precedenti del gruppo); `None` cancella il gruppo.
pub fn set_label(
    c: &Connection,
    group: &str,
    value: Option<&str>,
    ids: &[MediaId],
) -> StoreResult<BulkOutcome> {
    if group.is_empty() || !group.chars().all(|ch| ch.is_ascii_lowercase() || ch == '_') {
        return Err(StoreError::Invalid(format!("gruppo non valido: {group}")));
    }
    let previous = labels_of_group(c, group, ids)?;
    let mut del = c.prepare_cached("DELETE FROM label WHERE media_id = ?1 AND grp = ?2")?;
    let mut ins = c.prepare_cached(
        "INSERT INTO label(media_id, grp, value, score, origin) VALUES (?1, ?2, ?3, 1.0, 'manual')",
    )?;
    for id in ids {
        del.execute(params![id, group])?;
        if let Some(v) = value {
            ins.execute(params![id, group, v])?;
        }
    }
    refresh_all(c, ids)?;
    record(
        c,
        &format!("Imposta {group} = {}", value.unwrap_or("—")),
        &UndoAction::LabelsReplaced {
            group: group.into(),
            ids: ids.to_vec(),
            previous,
        },
    )?;
    Ok(BulkOutcome {
        changed: ids.len(),
        requested: ids.len(),
    })
}

/// Descrizione dell'ultima operazione annullabile.
pub fn last_undoable(c: &Connection) -> StoreResult<Option<String>> {
    Ok(c.query_row(
        "SELECT description FROM undo_log ORDER BY id DESC LIMIT 1",
        [],
        |r| r.get(0),
    )
    .optional()?)
}

/// Annulla l'ultima operazione registrata; restituisce la sua descrizione.
pub fn undo_last(c: &Connection) -> StoreResult<Option<String>> {
    let Some((id, description, payload)) = c
        .query_row(
            "SELECT id, description, payload FROM undo_log ORDER BY id DESC LIMIT 1",
            [],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            },
        )
        .optional()?
    else {
        return Ok(None);
    };
    let action: UndoAction = serde_json::from_str(&payload)?;
    undo_action(c, action)?;
    c.execute("DELETE FROM undo_log WHERE id = ?1", [id])?;
    Ok(Some(description))
}
fn undo_action(c: &Connection, action: UndoAction) -> StoreResult<()> {
    match action {
        UndoAction::Group { actions } => {
            for action in actions.into_iter().rev() {
                undo_action(c, action)?;
            }
        }
        UndoAction::TagsAdded { tag_id, ids } => {
            tag::remove(c, tag_id, &ids)?;
            refresh_all(c, &ids)?;
        }
        UndoAction::TagsRemoved { tag_id, ids } => {
            tag::add(c, tag_id, &ids)?;
            refresh_all(c, &ids)?;
        }
        UndoAction::CatalogAdded { catalog_id, ids } => {
            catalog::remove_items(c, catalog_id, &ids)?;
        }
        UndoAction::CatalogRemoved { catalog_id, ids } => {
            catalog::add_items(c, catalog_id, &ids)?;
        }
        UndoAction::LabelsReplaced {
            group,
            ids,
            previous,
        } => {
            let mut del = c.prepare_cached("DELETE FROM label WHERE media_id = ?1 AND grp = ?2")?;
            for id in &ids {
                del.execute(params![id, group])?;
            }
            let mut ins = c.prepare_cached(
                "INSERT INTO label(media_id, grp, value, score, origin, model_ver, taxonomy_ver) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )?;
            for r in previous {
                ins.execute(params![
                    r.media_id,
                    group,
                    r.value,
                    r.score,
                    r.origin,
                    r.model_ver,
                    r.taxonomy_ver
                ])?;
            }
            refresh_all(c, &ids)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CatalogKind, NewMedia};
    use crate::{migrate, repo};

    fn db_with(n: usize) -> (Connection, Vec<MediaId>) {
        let mut c = Connection::open_in_memory().unwrap();
        migrate::migrate(&mut c, None).unwrap();
        let ids = (0..n)
            .map(|i| {
                repo::media::upsert(
                    &c,
                    &NewMedia {
                        path: format!("D:\\{i}.jpg"),
                        name: format!("{i}.jpg"),
                        ext: "jpg".into(),
                        ..Default::default()
                    },
                )
                .unwrap()
            })
            .collect();
        (c, ids)
    }

    fn tagged(c: &Connection, name: &str) -> i64 {
        c.query_row(
            "SELECT count(*) FROM media_tag mt JOIN tag t ON t.id = mt.tag_id WHERE t.name = ?1",
            [name],
            |r| r.get(0),
        )
        .unwrap()
    }

    #[test]
    fn undo_tag_on_200_files() {
        let (c, ids) = db_with(200);
        add_tag(&c, "Estate", &ids[..10]).unwrap();
        let out = add_tag(&c, "Matrimonio", &ids).unwrap();
        assert_eq!(
            out,
            BulkOutcome {
                changed: 200,
                requested: 200
            }
        );
        assert_eq!(tagged(&c, "Matrimonio"), 200);
        assert_eq!(
            undo_last(&c).unwrap().as_deref(),
            Some("Aggiungi tag \"Matrimonio\"")
        );
        assert_eq!(tagged(&c, "Matrimonio"), 0);
        // Gli altri dati restano invariati.
        assert_eq!(tagged(&c, "Estate"), 10);
    }

    #[test]
    fn undo_only_reverts_actually_changed_items() {
        let (c, ids) = db_with(5);
        add_tag(&c, "mare", &ids[..2]).unwrap();
        add_tag(&c, "mare", &ids).unwrap(); // aggiunge solo agli altri 3
        undo_last(&c).unwrap();
        assert_eq!(tagged(&c, "mare"), 2);
    }

    #[test]
    fn undo_catalog_and_labels() {
        let (c, ids) = db_with(4);
        let k = repo::catalog::create(&c, "Teatro", CatalogKind::Manual, None).unwrap();
        add_to_catalog(&c, k, &ids).unwrap();
        remove_from_catalog(&c, k, &ids[..2]).unwrap();
        undo_last(&c).unwrap();
        assert_eq!(repo::catalog::manual_items(&c, k).unwrap().len(), 4);

        c.execute(
            "INSERT INTO label(media_id, grp, value, score, origin, model_ver) VALUES (?1, 'location', 'esterno_natura', 0.7, 'auto', 'clip-1')",
            [ids[0]],
        )
        .unwrap();
        set_label(&c, "location", Some("interno_teatro"), &ids).unwrap();
        let v: String = c
            .query_row(
                "SELECT value || ':' || origin FROM label WHERE media_id = ?1 AND grp = 'location'",
                [ids[0]],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(v, "interno_teatro:manual");
        undo_last(&c).unwrap();
        let v: String = c
            .query_row(
                "SELECT value || ':' || origin FROM label WHERE media_id = ?1 AND grp = 'location'",
                [ids[0]],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(v, "esterno_natura:auto");
        let n: i64 = c
            .query_row(
                "SELECT count(*) FROM label WHERE grp = 'location'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1);
        assert!(set_label(&c, "x; DROP", Some("a"), &ids).is_err());
    }

    #[test]
    fn undo_history_is_bounded_and_empty_undo_is_noop() {
        let (c, ids) = db_with(1);
        assert_eq!(undo_last(&c).unwrap(), None);
        for i in 0..30 {
            add_tag(&c, &format!("t{i}"), &ids).unwrap();
        }
        let n: i64 = c
            .query_row("SELECT count(*) FROM undo_log", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, UNDO_DEPTH);
        assert_eq!(
            last_undoable(&c).unwrap().as_deref(),
            Some("Aggiungi tag \"t29\"")
        );
    }
}
