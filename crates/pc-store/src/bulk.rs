//! Modifiche in blocco a tag, cataloghi e catalogazioni, con registrazione
//! dell'operazione inversa in `undo_log` per "Annulla".

use crate::model::{Location, LocationSource, MediaId};
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
    /// Tag rinominato: si torna al nome precedente.
    TagRenamed {
        tag_id: i64,
        from: String,
        to: String,
    },
    /// Tag sorgente unito nel tag di destinazione ed eliminato.
    TagMerged {
        source_id: i64,
        source_name: String,
        target_id: i64,
        /// File che avevano il tag sorgente.
        source_ids: Vec<MediaId>,
        /// File che hanno ricevuto il tag di destinazione con l'unione.
        added: Vec<MediaId>,
    },
    /// Tag eliminato con le sue assegnazioni.
    TagDeleted {
        tag_id: i64,
        name: String,
        ids: Vec<MediaId>,
    },
    /// Stato precedente di `app.db` (set e colori dei tag): `pc-store` non lo
    /// interpreta, lo restituisce a chi annulla con [`undo_last_collect`].
    AppStore {
        payload: serde_json::Value,
    },
    /// Correzioni di un gruppo di sorgente prima della modifica.
    GroupClock {
        group: i64,
        previous: crate::timeline::GroupClock,
    },
    /// Collocazione manuale precedente di una foto senza data (`None` = nessuna).
    Placement {
        media_id: MediaId,
        previous: Option<(i64, Option<i32>)>,
    },
    /// Luoghi precedenti (riga completa o assenza) di un gruppo di file.
    LocationReplaced {
        previous: Vec<(MediaId, Option<Location>)>,
    },
}

/// Luogo scelto dall'utente (nome libero o località del dataset geografico).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct PlaceInput {
    pub name: String,
    #[serde(default)]
    pub country: Option<String>,
    #[serde(default)]
    pub lat: Option<f64>,
    #[serde(default)]
    pub lon: Option<f64>,
}

/// Dati salvati da cui si ricalcola il luogo automatico di un file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlaceInputs {
    pub path: String,
    pub gps: Option<(f64, f64)>,
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
}

/// Calcolo del luogo automatico (GPS o cartella), fornito da chi conosce il
/// dataset geografico e le cartelle da ignorare.
pub type PlaceResolver<'a> = &'a dyn Fn(&PlaceInputs) -> Option<Location>;

/// Percorso, GPS e fotocamera di un file, dal catalogo.
pub fn place_inputs(c: &Connection, id: MediaId) -> StoreResult<PlaceInputs> {
    let path: String = c.query_row("SELECT path FROM media_item WHERE id=?1", [id], |r| {
        r.get(0)
    })?;
    let exif = c
        .query_row(
            "SELECT camera_make, camera_model, gps_lat, gps_lon FROM exif WHERE media_id=?1",
            [id],
            |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<f64>>(2)?,
                    r.get::<_, Option<f64>>(3)?,
                ))
            },
        )
        .optional()?;
    let (camera_make, camera_model, lat, lon) = exif.unwrap_or_default();
    Ok(PlaceInputs {
        path,
        gps: lat.zip(lon),
        camera_make,
        camera_model,
    })
}

fn locations_of(c: &Connection, ids: &[MediaId]) -> StoreResult<Vec<(MediaId, Option<Location>)>> {
    ids.iter()
        .map(|id| Ok((*id, crate::repo::meta::location(c, *id)?)))
        .collect()
}

fn write_location(c: &Connection, id: MediaId, l: Option<&Location>) -> StoreResult<()> {
    c.execute("DELETE FROM location WHERE media_id=?1", [id])?;
    if let Some(l) = l {
        c.execute(
            "INSERT INTO location(media_id, name, country, lat, lon, source) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, l.name, l.country, l.lat, l.lon, l.source.as_str()],
        )?;
    }
    Ok(())
}

/// Imposta il luogo come correzione manuale (`Some`) oppure ripristina il
/// luogo automatico (`None`): la riga manuale viene tolta e il luogo
/// ricalcolato subito con `resolve` da GPS o cartella. Annullabile.
pub fn set_place(
    c: &Connection,
    ids: &[MediaId],
    place: Option<&PlaceInput>,
    resolve: PlaceResolver,
) -> StoreResult<BulkOutcome> {
    let manual = match place {
        Some(p) => {
            let name = p.name.trim();
            if name.is_empty() || name.chars().count() > 200 {
                return Err(StoreError::Invalid("nome del luogo non valido".into()));
            }
            Some(Location {
                name: Some(name.to_string()),
                country: p.country.clone().filter(|s| !s.trim().is_empty()),
                lat: p
                    .lat
                    .filter(|v| v.is_finite() && (-90.0..=90.0).contains(v)),
                lon: p
                    .lon
                    .filter(|v| v.is_finite() && (-180.0..=180.0).contains(v)),
                source: LocationSource::Manual,
            })
        }
        None => None,
    };
    let previous = locations_of(c, ids)?;
    for id in ids {
        let next = match &manual {
            Some(l) => Some(l.clone()),
            None => resolve(&place_inputs(c, *id)?).filter(|l| l.source != LocationSource::Manual),
        };
        write_location(c, *id, next.as_ref())?;
    }
    refresh_all(c, ids)?;
    let description = match &manual {
        Some(l) => format!("Imposta luogo {}", l.name.as_deref().unwrap_or_default()),
        None => "Ripristina luogo automatico".to_string(),
    };
    record(c, &description, &UndoAction::LocationReplaced { previous })?;
    Ok(BulkOutcome {
        changed: ids.len(),
        requested: ids.len(),
    })
}

/// File con il luogo `name` (stesso confronto del facet "Luogo").
pub fn place_ids(c: &Connection, name: &str) -> StoreResult<Vec<MediaId>> {
    let mut q = c.prepare("SELECT media_id FROM location WHERE name = ?1 ORDER BY media_id")?;
    let ids = q
        .query_map([name], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    Ok(ids)
}

/// Rinomina un luogo per tutti i file che lo hanno, come correzione manuale
/// (coordinate e paese di ogni file restano). Rinominare verso un luogo già
/// esistente unisce i due luoghi. Un'unica operazione annullabile.
pub fn rename_place(c: &Connection, from: &str, to: &str) -> StoreResult<BulkOutcome> {
    let to = to.trim();
    if to.is_empty() || to.chars().count() > 200 {
        return Err(StoreError::Invalid("nome del luogo non valido".into()));
    }
    let ids = place_ids(c, from)?;
    if ids.is_empty() || from == to {
        return Ok(BulkOutcome {
            changed: 0,
            requested: ids.len(),
        });
    }
    let previous = locations_of(c, &ids)?;
    c.execute(
        "UPDATE location SET name = ?2, source = 'manual' WHERE name = ?1",
        params![from, to],
    )?;
    refresh_all(c, &ids)?;
    record(
        c,
        &format!("Rinomina luogo {from} → {to}"),
        &UndoAction::LocationReplaced { previous },
    )?;
    Ok(BulkOutcome {
        changed: ids.len(),
        requested: ids.len(),
    })
}

/// Ricalcola i luoghi dedotti dalle cartelle (e quelli dei file senza luogo né
/// GPS), ad esempio dopo un cambio delle cartelle da ignorare. I luoghi
/// manuali e quelli da GPS non vengono toccati. Restituisce i file cambiati.
pub fn recompute_directory_places(c: &Connection, resolve: PlaceResolver) -> StoreResult<usize> {
    let mut q = c.prepare(
        "SELECT m.id FROM media_item m LEFT JOIN location l ON l.media_id = m.id
         LEFT JOIN exif e ON e.media_id = m.id
         WHERE l.source = 'directory'
            OR (l.media_id IS NULL AND (e.gps_lat IS NULL OR e.gps_lon IS NULL))
         ORDER BY m.id",
    )?;
    let ids: Vec<MediaId> = q.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
    let mut changed = 0;
    for id in ids {
        let next = resolve(&place_inputs(c, id)?).filter(|l| l.source == LocationSource::Directory);
        let prev = crate::repo::meta::location(c, id)?;
        if prev != next {
            write_location(c, id, next.as_ref())?;
            fts::refresh(c, id)?;
            changed += 1;
        }
    }
    Ok(changed)
}

/// Luoghi della libreria che iniziano con `prefix` (senza maiuscole), dal più
/// frequente.
pub fn suggest_places(c: &Connection, prefix: &str, limit: usize) -> StoreResult<Vec<String>> {
    let escaped = prefix
        .trim()
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    let mut q = c.prepare(
        "SELECT name FROM location WHERE name IS NOT NULL AND name LIKE ?1 || '%' ESCAPE '\\'
         GROUP BY name ORDER BY count(*) DESC, name LIMIT ?2",
    )?;
    let v = q
        .query_map(params![escaped, limit as i64], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    Ok(v)
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

pub(crate) fn record(c: &Connection, description: &str, action: &UndoAction) -> StoreResult<()> {
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

/// Aggiunge un tag. `exclusive_siblings` sono gli altri tag del set esclusivo
/// a cui appartiene `name`: vengono tolti dagli stessi file nella stessa
/// operazione annullabile.
pub fn add_tag(
    c: &Connection,
    name: &str,
    ids: &[MediaId],
    exclusive_siblings: &[String],
) -> StoreResult<BulkOutcome> {
    let tag_id = tag::ensure(c, name)?;
    let mut actions = Vec::new();
    let mut touched: Vec<MediaId> = Vec::new();
    for sibling in exclusive_siblings {
        let Some(sib_id) = tag_id_of(c, sibling)? else {
            continue;
        };
        if sib_id == tag_id {
            continue;
        }
        let removed = tag::remove(c, sib_id, ids)?;
        if !removed.is_empty() {
            touched.extend(&removed);
            actions.push(UndoAction::TagsRemoved {
                tag_id: sib_id,
                ids: removed,
            });
        }
    }
    let added = tag::add(c, tag_id, ids)?;
    touched.extend(&added);
    touched.sort_unstable();
    touched.dedup();
    refresh_all(c, &touched)?;
    let changed = touched.len();
    let add_action = UndoAction::TagsAdded { tag_id, ids: added };
    let action = if actions.is_empty() {
        add_action
    } else {
        actions.push(add_action);
        UndoAction::Group { actions }
    };
    record(c, &format!("Aggiungi tag \"{name}\""), &action)?;
    Ok(BulkOutcome {
        changed,
        requested: ids.len(),
    })
}

fn tag_id_of(c: &Connection, name: &str) -> StoreResult<Option<i64>> {
    Ok(
        c.query_row("SELECT id FROM tag WHERE name = ?1", [name.trim()], |r| {
            r.get::<_, i64>(0)
        })
        .optional()?,
    )
}

fn tag_row(c: &Connection, name: &str) -> StoreResult<(i64, String)> {
    c.query_row(
        "SELECT id, name FROM tag WHERE name = ?1",
        [name.trim()],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .optional()?
    .ok_or_else(|| StoreError::NotFound(format!("tag \"{name}\"")))
}

fn media_with_tag(c: &Connection, tag_id: i64) -> StoreResult<Vec<MediaId>> {
    let mut st =
        c.prepare_cached("SELECT media_id FROM media_tag WHERE tag_id = ?1 ORDER BY media_id")?;
    let rows = st
        .query_map([tag_id], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

/// Registra l'azione e, se presente, lo stato precedente di `app.db` nella stessa voce.
fn record_with_app(
    c: &Connection,
    description: &str,
    action: UndoAction,
    app: Option<serde_json::Value>,
) -> StoreResult<()> {
    let action = match app {
        Some(payload) => UndoAction::Group {
            actions: vec![action, UndoAction::AppStore { payload }],
        },
        None => action,
    };
    record(c, description, &action)
}

/// Rinomina un tag mantenendo le assegnazioni. Se esiste già un altro tag con
/// il nuovo nome (senza maiuscole) restituisce [`StoreError::Conflict`]: va
/// proposta l'unione. `app` è lo stato dei set da ripristinare in annullamento.
pub fn rename_tag(
    c: &Connection,
    from: &str,
    to: &str,
    app: Option<serde_json::Value>,
) -> StoreResult<()> {
    let to = to.trim();
    if to.is_empty() {
        return Err(StoreError::Invalid("nome tag vuoto".into()));
    }
    let (tag_id, old) = tag_row(c, from)?;
    if let Some(other) = tag_id_of(c, to)? {
        if other != tag_id {
            return Err(StoreError::Conflict(format!("tag \"{to}\" già esistente")));
        }
    }
    if old == to {
        return Ok(());
    }
    c.execute(
        "UPDATE tag SET name = ?2 WHERE id = ?1",
        params![tag_id, to],
    )?;
    refresh_all(c, &media_with_tag(c, tag_id)?)?;
    record_with_app(
        c,
        &format!("Rinomina tag \"{old}\" in \"{to}\""),
        UndoAction::TagRenamed {
            tag_id,
            from: old,
            to: to.into(),
        },
        app,
    )
}

/// Unisce `source` in `target` (creato se non esiste): i file di `source`
/// ricevono `target` e `source` viene eliminato.
pub fn merge_tag(
    c: &Connection,
    source: &str,
    target: &str,
    app: Option<serde_json::Value>,
) -> StoreResult<BulkOutcome> {
    let (source_id, source_name) = tag_row(c, source)?;
    let target_id = tag::ensure(c, target)?;
    if target_id == source_id {
        return Err(StoreError::Invalid(
            "un tag non si unisce a se stesso".into(),
        ));
    }
    let target_name: String =
        c.query_row("SELECT name FROM tag WHERE id = ?1", [target_id], |r| {
            r.get(0)
        })?;
    let source_ids = media_with_tag(c, source_id)?;
    let added = tag::add(c, target_id, &source_ids)?;
    c.execute("DELETE FROM tag WHERE id = ?1", [source_id])?;
    refresh_all(c, &source_ids)?;
    let changed = added.len();
    let requested = source_ids.len();
    record_with_app(
        c,
        &format!("Unisci tag \"{source_name}\" in \"{target_name}\""),
        UndoAction::TagMerged {
            source_id,
            source_name,
            target_id,
            source_ids,
            added,
        },
        app,
    )?;
    Ok(BulkOutcome { changed, requested })
}

/// Elimina un tag e le sue assegnazioni (i file su disco non vengono toccati).
pub fn delete_tag(
    c: &Connection,
    name: &str,
    app: Option<serde_json::Value>,
) -> StoreResult<BulkOutcome> {
    let (tag_id, exact) = tag_row(c, name)?;
    let ids = media_with_tag(c, tag_id)?;
    c.execute("DELETE FROM tag WHERE id = ?1", [tag_id])?;
    refresh_all(c, &ids)?;
    let n = ids.len();
    record_with_app(
        c,
        &format!("Elimina tag \"{exact}\""),
        UndoAction::TagDeleted {
            tag_id,
            name: exact,
            ids,
        },
        app,
    )?;
    Ok(BulkOutcome {
        changed: n,
        requested: n,
    })
}

pub fn remove_tag(c: &Connection, name: &str, ids: &[MediaId]) -> StoreResult<BulkOutcome> {
    let Some(tag_id) = tag_id_of(c, name)? else {
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

/// Rimuove più tag dagli stessi file in un'unica operazione annullabile
/// (tasto 0: i tag del set attivo). `changed` conta i file distinti modificati.
pub fn remove_tags(c: &Connection, names: &[String], ids: &[MediaId]) -> StoreResult<BulkOutcome> {
    let mut actions = Vec::new();
    let mut touched: Vec<MediaId> = Vec::new();
    for name in names {
        let Some(tag_id) = tag_id_of(c, name)? else {
            continue;
        };
        let removed = tag::remove(c, tag_id, ids)?;
        if !removed.is_empty() {
            touched.extend(&removed);
            actions.push(UndoAction::TagsRemoved {
                tag_id,
                ids: removed,
            });
        }
    }
    touched.sort_unstable();
    touched.dedup();
    refresh_all(c, &touched)?;
    if !actions.is_empty() {
        record(c, "Rimuovi i tag del set", &UndoAction::Group { actions })?;
    }
    Ok(BulkOutcome {
        changed: touched.len(),
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
    Ok(undo_last_collect(c)?.map(|(d, _)| d))
}

/// Come [`undo_last`], restituendo anche gli stati di `app.db` da ripristinare
/// (nell'ordine in cui applicarli).
pub fn undo_last_collect(c: &Connection) -> StoreResult<Option<(String, Vec<serde_json::Value>)>> {
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
    let mut app = Vec::new();
    undo_action(c, action, &mut app)?;
    c.execute("DELETE FROM undo_log WHERE id = ?1", [id])?;
    Ok(Some((description, app)))
}
fn undo_action(
    c: &Connection,
    action: UndoAction,
    app: &mut Vec<serde_json::Value>,
) -> StoreResult<()> {
    match action {
        UndoAction::Group { actions } => {
            for action in actions.into_iter().rev() {
                undo_action(c, action, app)?;
            }
        }
        UndoAction::AppStore { payload } => app.push(payload),
        UndoAction::GroupClock { group, previous } => {
            crate::timeline::set_clock(c, group, &previous, &crate::timeline::system_offset)?;
        }
        UndoAction::Placement { media_id, previous } => {
            crate::timeline::set_placement(c, media_id, previous, &crate::timeline::system_offset)?;
        }
        UndoAction::LocationReplaced { previous } => {
            for (id, l) in &previous {
                write_location(c, *id, l.as_ref())?;
            }
            refresh_all(c, &previous.iter().map(|(id, _)| *id).collect::<Vec<_>>())?;
        }
        UndoAction::TagRenamed { tag_id, from, .. } => {
            c.execute(
                "UPDATE tag SET name = ?2 WHERE id = ?1",
                params![tag_id, from],
            )?;
            refresh_all(c, &media_with_tag(c, tag_id)?)?;
        }
        UndoAction::TagMerged {
            source_id,
            source_name,
            target_id,
            source_ids,
            added,
        } => {
            c.execute(
                "INSERT INTO tag(id, name) VALUES (?1, ?2)",
                params![source_id, source_name],
            )?;
            tag::add(c, source_id, &source_ids)?;
            tag::remove(c, target_id, &added)?;
            refresh_all(c, &source_ids)?;
        }
        UndoAction::TagDeleted { tag_id, name, ids } => {
            c.execute(
                "INSERT INTO tag(id, name) VALUES (?1, ?2)",
                params![tag_id, name],
            )?;
            tag::add(c, tag_id, &ids)?;
            refresh_all(c, &ids)?;
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
        add_tag(&c, "Estate", &ids[..10], &[]).unwrap();
        let out = add_tag(&c, "Matrimonio", &ids, &[]).unwrap();
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
        add_tag(&c, "mare", &ids[..2], &[]).unwrap();
        add_tag(&c, "mare", &ids, &[]).unwrap(); // aggiunge solo agli altri 3
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
            add_tag(&c, &format!("t{i}"), &ids, &[]).unwrap();
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

    fn tag_names(c: &Connection) -> Vec<String> {
        repo::tag::list(c)
            .unwrap()
            .into_iter()
            .map(|(_, n, _)| n)
            .collect()
    }

    #[test]
    fn rename_keeps_assignments_and_undo_restores_name() {
        let (c, ids) = db_with(5);
        add_tag(&c, "Tramonto", &ids[..3], &[]).unwrap();
        rename_tag(&c, "tramonto", "Tramonti", None).unwrap();
        assert_eq!(tagged(&c, "Tramonti"), 3);
        assert_eq!(tag_names(&c), vec!["Tramonti"]);
        // La ricerca testuale segue il nuovo nome.
        let hits: i64 = c
            .query_row(
                "SELECT count(*) FROM media_fts WHERE media_fts MATCH 'tags:Tramonti'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(hits, 3);
        assert_eq!(
            undo_last(&c).unwrap().as_deref(),
            Some("Rinomina tag \"Tramonto\" in \"Tramonti\"")
        );
        assert_eq!(tag_names(&c), vec!["Tramonto"]);
        assert_eq!(tagged(&c, "Tramonto"), 3);
    }

    #[test]
    fn rename_onto_existing_name_signals_merge() {
        let (c, ids) = db_with(2);
        add_tag(&c, "tramonto", &ids[..1], &[]).unwrap();
        add_tag(&c, "Tramonti", &ids[1..], &[]).unwrap();
        let err = rename_tag(&c, "tramonto", "TRAMONTI", None).unwrap_err();
        assert!(matches!(err, StoreError::Conflict(_)), "{err:?}");
        assert_eq!(tag_names(&c), vec!["Tramonti", "tramonto"]);
        // Cambiare solo le maiuscole dello stesso tag è una rinomina.
        rename_tag(&c, "tramonto", "Tramonto", None).unwrap();
        assert_eq!(tag_names(&c), vec!["Tramonti", "Tramonto"]);
    }

    #[test]
    fn merge_with_common_files_and_undo() {
        let (c, ids) = db_with(46);
        // "Tramonti" su 0..38, "Sunset" su 34..46: 4 in comune.
        add_tag(&c, "Tramonti", &ids[..38], &[]).unwrap();
        add_tag(&c, "Sunset", &ids[34..], &[]).unwrap();
        let app = serde_json::json!({"sets":[{"set_id":2,"position":3,"tag_name":"Sunset"}]});
        let out = merge_tag(&c, "Sunset", "Tramonti", Some(app.clone())).unwrap();
        assert_eq!(out.changed, 8);
        assert_eq!(out.requested, 12);
        assert_eq!(tagged(&c, "Tramonti"), 46);
        assert_eq!(tag_names(&c), vec!["Tramonti"]);
        let (desc, payloads) = undo_last_collect(&c).unwrap().unwrap();
        assert_eq!(desc, "Unisci tag \"Sunset\" in \"Tramonti\"");
        assert_eq!(payloads, vec![app]);
        assert_eq!(tagged(&c, "Sunset"), 12);
        assert_eq!(tagged(&c, "Tramonti"), 38);
    }

    #[test]
    fn delete_tag_and_undo() {
        let (c, ids) = db_with(10);
        add_tag(&c, "Scartate", &ids, &[]).unwrap();
        add_tag(&c, "Altro", &ids[..2], &[]).unwrap();
        let out = delete_tag(&c, "scartate", None).unwrap();
        assert_eq!(out.changed, 10);
        assert_eq!(tagged(&c, "Scartate"), 0);
        assert_eq!(tag_names(&c), vec!["Altro"]);
        assert_eq!(
            undo_last(&c).unwrap().as_deref(),
            Some("Elimina tag \"Scartate\"")
        );
        assert_eq!(tagged(&c, "Scartate"), 10);
        assert_eq!(tagged(&c, "Altro"), 2);
        assert!(matches!(
            delete_tag(&c, "inesistente", None),
            Err(StoreError::NotFound(_))
        ));
    }

    #[test]
    fn exclusive_set_replaces_sibling_and_undo_restores_it() {
        let (c, ids) = db_with(40);
        add_tag(&c, "Paesaggi", &ids, &[]).unwrap();
        add_tag(&c, "Fuori set", &ids[..5], &[]).unwrap();
        let siblings = vec!["Paesaggi".to_string(), "Città".to_string()];
        let out = add_tag(&c, "Città", &ids, &siblings).unwrap();
        assert_eq!(out.changed, 40);
        assert_eq!(tagged(&c, "Città"), 40);
        assert_eq!(tagged(&c, "Paesaggi"), 0);
        assert_eq!(tagged(&c, "Fuori set"), 5);
        undo_last(&c).unwrap();
        assert_eq!(tagged(&c, "Città"), 0);
        assert_eq!(tagged(&c, "Paesaggi"), 40);
        assert_eq!(tagged(&c, "Fuori set"), 5);
    }

    #[test]
    fn conflicts_when_set_becomes_exclusive() {
        let (c, ids) = db_with(10);
        add_tag(&c, "Paesaggi", &ids[..8], &[]).unwrap();
        add_tag(&c, "Città", &ids[2..], &[]).unwrap();
        add_tag(&c, "Altro", &ids, &[]).unwrap();
        let set = vec![
            "paesaggi".to_string(),
            "Città".to_string(),
            "Tramonti".to_string(),
        ];
        // 6 file hanno sia Paesaggi sia Città; nessun tag viene rimosso.
        assert_eq!(repo::tag::conflicts(&c, &set).unwrap(), ids[2..8].to_vec());
        assert_eq!(tagged(&c, "Paesaggi"), 8);
        assert!(repo::tag::conflicts(&c, &set[..1]).unwrap().is_empty());
    }

    #[test]
    fn remove_tags_of_set_is_one_undo() {
        let (c, ids) = db_with(6);
        add_tag(&c, "A", &ids[..3], &[]).unwrap();
        add_tag(&c, "B", &ids[2..], &[]).unwrap();
        add_tag(&c, "Altro", &ids, &[]).unwrap();
        let out = remove_tags(&c, &["A".into(), "B".into(), "Mai usato".into()], &ids).unwrap();
        assert_eq!(out.changed, 6);
        assert_eq!(tagged(&c, "A") + tagged(&c, "B"), 0);
        assert_eq!(tagged(&c, "Altro"), 6);
        undo_last(&c).unwrap();
        assert_eq!(tagged(&c, "A"), 3);
        assert_eq!(tagged(&c, "B"), 4);
    }
}
