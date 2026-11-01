//! Set di tag e colori in `app.db`. I tag sono riferiti per nome (senza
//! maiuscole): così valgono in ogni libreria e vengono creati al primo uso.

use super::{kv_get, kv_set};
use crate::{StoreError, StoreResult};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Tasti rapidi disponibili per set (1–9).
pub const MAX_SET_TAGS: usize = 9;
/// Palette fissa dei tag (chiavi stabili; i colori stanno nei token della UI).
pub const PALETTE: &[&str] = &["blue", "teal", "yellow", "coral", "purple", "gray"];
const ACTIVE_KEY: &str = "active_tag_set";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SetTag {
    /// Tasto rapido (1–9).
    pub key: u8,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct TagSet {
    pub id: i64,
    pub name: String,
    pub exclusive: bool,
    pub is_base: bool,
    pub export_keywords: bool,
    pub tags: Vec<SetTag>,
}

impl TagSet {
    pub fn tag_names(&self) -> Vec<String> {
        self.tags.iter().map(|t| t.name.clone()).collect()
    }
}

/// Colore predefinito di un tag senza colore scelto: deterministico dal nome.
pub fn default_color(name: &str) -> &'static str {
    let h = name
        .to_lowercase()
        .bytes()
        .fold(0u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32));
    PALETTE[(h as usize) % (PALETTE.len() - 1)]
}

fn clean_name(name: &str, what: &str) -> StoreResult<String> {
    let n = name.trim();
    if n.is_empty() {
        return Err(StoreError::Invalid(format!("nome {what} vuoto")));
    }
    Ok(n.to_string())
}

fn unique_conflict(e: rusqlite::Error, what: String) -> StoreError {
    match e {
        rusqlite::Error::SqliteFailure(f, _)
            if f.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            StoreError::Conflict(what)
        }
        other => other.into(),
    }
}

pub fn list_sets(c: &Connection) -> StoreResult<Vec<TagSet>> {
    let mut st = c.prepare(
        "SELECT id, name, exclusive, is_base, export_keywords FROM tag_set ORDER BY position, id",
    )?;
    let mut sets = st
        .query_map([], |r| {
            Ok(TagSet {
                id: r.get(0)?,
                name: r.get(1)?,
                exclusive: r.get(2)?,
                is_base: r.get(3)?,
                export_keywords: r.get(4)?,
                tags: Vec::new(),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut items = c.prepare(
        "SELECT position, tag_name FROM tag_set_item WHERE set_id = ?1 ORDER BY position",
    )?;
    for s in &mut sets {
        s.tags = items
            .query_map([s.id], |r| {
                Ok(SetTag {
                    key: r.get(0)?,
                    name: r.get(1)?,
                })
            })?
            .collect::<Result<_, _>>()?;
    }
    Ok(sets)
}

pub fn get_set(c: &Connection, id: i64) -> StoreResult<TagSet> {
    list_sets(c)?
        .into_iter()
        .find(|s| s.id == id)
        .ok_or_else(|| StoreError::NotFound(format!("set {id}")))
}

/// Crea un set con i tag nell'ordine indicato (tasti 1..n).
pub fn create_set(c: &Connection, name: &str, tags: &[String]) -> StoreResult<i64> {
    let name = clean_name(name, "set")?;
    let position: i64 = c.query_row(
        "SELECT COALESCE(MAX(position), 0) + 1 FROM tag_set",
        [],
        |r| r.get(0),
    )?;
    c.execute(
        "INSERT INTO tag_set (name, position) VALUES (?1, ?2)",
        params![name, position],
    )
    .map_err(|e| unique_conflict(e, format!("set \"{name}\" già esistente")))?;
    let id = c.last_insert_rowid();
    set_tags(c, id, tags)?;
    Ok(id)
}

pub fn rename_set(c: &Connection, id: i64, name: &str) -> StoreResult<()> {
    let name = clean_name(name, "set")?;
    let n = c
        .execute(
            "UPDATE tag_set SET name = ?2 WHERE id = ?1",
            params![id, name],
        )
        .map_err(|e| unique_conflict(e, format!("set \"{name}\" già esistente")))?;
    if n == 0 {
        return Err(StoreError::NotFound(format!("set {id}")));
    }
    Ok(())
}

/// Copia un set (tag, esclusività, metadati) con un nome libero "<nome> (copia N)".
pub fn duplicate_set(c: &Connection, id: i64) -> StoreResult<i64> {
    let src = get_set(c, id)?;
    let mut name = format!("{} (copia)", src.name);
    let mut n = 2;
    while c
        .query_row("SELECT 1 FROM tag_set WHERE name = ?1", [&name], |_| Ok(()))
        .optional()?
        .is_some()
    {
        name = format!("{} (copia {n})", src.name);
        n += 1;
    }
    let new_id = create_set(c, &name, &[])?;
    c.execute(
        "UPDATE tag_set SET exclusive = ?2, export_keywords = ?3 WHERE id = ?1",
        params![new_id, src.exclusive, src.export_keywords],
    )?;
    c.execute(
        "INSERT INTO tag_set_item (set_id, position, tag_name)
         SELECT ?2, position, tag_name FROM tag_set_item WHERE set_id = ?1",
        params![id, new_id],
    )?;
    Ok(new_id)
}

/// Elimina un set: i tag e le loro assegnazioni restano. Il set Base non si elimina.
pub fn delete_set(c: &Connection, id: i64) -> StoreResult<()> {
    let set = get_set(c, id)?;
    if set.is_base {
        return Err(StoreError::Invalid(
            "il set Base non si può eliminare".into(),
        ));
    }
    c.execute("DELETE FROM tag_set WHERE id = ?1", [id])?;
    if active_set(c)? == Some(id) {
        let base: i64 = c.query_row(
            "SELECT id FROM tag_set ORDER BY is_base DESC, position LIMIT 1",
            [],
            |r| r.get(0),
        )?;
        set_active(c, base)?;
    }
    Ok(())
}

/// Nuovo ordine dei set (gli id non elencati restano in coda).
pub fn reorder_sets(c: &Connection, ids: &[i64]) -> StoreResult<()> {
    let mut st = c.prepare_cached("UPDATE tag_set SET position = ?2 WHERE id = ?1")?;
    for (i, id) in ids.iter().enumerate() {
        st.execute(params![id, i as i64])?;
    }
    c.execute(
        "UPDATE tag_set SET position = position + ?1 WHERE id NOT IN (SELECT value FROM json_each(?2))",
        params![ids.len() as i64, serde_json::to_string(ids)?],
    )?;
    Ok(())
}

pub fn set_exclusive(c: &Connection, id: i64, exclusive: bool) -> StoreResult<()> {
    update_flag(c, id, "exclusive", exclusive)
}

pub fn set_export_keywords(c: &Connection, id: i64, on: bool) -> StoreResult<()> {
    update_flag(c, id, "export_keywords", on)
}

fn update_flag(c: &Connection, id: i64, column: &str, on: bool) -> StoreResult<()> {
    let n = c.execute(
        &format!("UPDATE tag_set SET {column} = ?2 WHERE id = ?1"),
        params![id, on],
    )?;
    if n == 0 {
        return Err(StoreError::NotFound(format!("set {id}")));
    }
    Ok(())
}

/// Sostituisce i tag del set con quelli indicati, nell'ordine (tasti 1..n).
pub fn set_tags(c: &Connection, id: i64, tags: &[String]) -> StoreResult<()> {
    let mut names: Vec<String> = Vec::new();
    for t in tags {
        let t = clean_name(t, "tag")?;
        if !names.iter().any(|n| n.to_lowercase() == t.to_lowercase()) {
            names.push(t);
        }
    }
    if names.len() > MAX_SET_TAGS {
        return Err(StoreError::Invalid(format!(
            "un set può contenere al massimo {MAX_SET_TAGS} tag"
        )));
    }
    c.execute("DELETE FROM tag_set_item WHERE set_id = ?1", [id])?;
    let mut st = c.prepare_cached(
        "INSERT INTO tag_set_item (set_id, position, tag_name) VALUES (?1, ?2, ?3)",
    )?;
    for (i, n) in names.iter().enumerate() {
        st.execute(params![id, i as i64 + 1, n])?;
    }
    Ok(())
}

/// Aggiunge un tag nel primo tasto libero; restituisce il tasto.
pub fn add_to_set(c: &Connection, id: i64, tag: &str) -> StoreResult<u8> {
    let tag = clean_name(tag, "tag")?;
    let set = get_set(c, id)?;
    if let Some(t) = set
        .tags
        .iter()
        .find(|t| t.name.to_lowercase() == tag.to_lowercase())
    {
        return Ok(t.key);
    }
    let key = (1..=MAX_SET_TAGS as u8)
        .find(|k| !set.tags.iter().any(|t| t.key == *k))
        .ok_or_else(|| {
            StoreError::Invalid(format!(
                "un set può contenere al massimo {MAX_SET_TAGS} tag"
            ))
        })?;
    c.execute(
        "INSERT INTO tag_set_item (set_id, position, tag_name) VALUES (?1, ?2, ?3)",
        params![id, key, tag],
    )?;
    Ok(key)
}

pub fn remove_from_set(c: &Connection, id: i64, tag: &str) -> StoreResult<()> {
    c.execute(
        "DELETE FROM tag_set_item WHERE set_id = ?1 AND tag_name = ?2",
        params![id, tag.trim()],
    )?;
    Ok(())
}

pub fn active_set(c: &Connection) -> StoreResult<Option<i64>> {
    let id = kv_get(c, ACTIVE_KEY)?.and_then(|v| v.parse::<i64>().ok());
    match id {
        Some(id) if get_set(c, id).is_ok() => Ok(Some(id)),
        _ => Ok(c
            .query_row(
                "SELECT id FROM tag_set ORDER BY is_base DESC, position LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()?),
    }
}

pub fn set_active(c: &Connection, id: i64) -> StoreResult<()> {
    get_set(c, id)?;
    kv_set(c, ACTIVE_KEY, &id.to_string())
}

/// Altri tag dei set esclusivi che contengono `tag`: da togliere quando lo si assegna.
pub fn exclusive_siblings(c: &Connection, tag: &str) -> StoreResult<Vec<String>> {
    let mut st = c.prepare(
        "SELECT DISTINCT o.tag_name FROM tag_set_item i
         JOIN tag_set s ON s.id = i.set_id AND s.exclusive = 1
         JOIN tag_set_item o ON o.set_id = i.set_id AND o.tag_name <> i.tag_name
         WHERE i.tag_name = ?1 ORDER BY o.tag_name",
    )?;
    let rows = st
        .query_map([tag.trim()], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

pub fn set_color(c: &Connection, tag: &str, color: Option<&str>) -> StoreResult<()> {
    let tag = clean_name(tag, "tag")?;
    match color {
        Some(col) => {
            if !PALETTE.contains(&col) {
                return Err(StoreError::Invalid(format!("colore non valido: {col}")));
            }
            c.execute(
                "INSERT INTO tag_style (tag_name, color) VALUES (?1, ?2)
                 ON CONFLICT(tag_name) DO UPDATE SET color = excluded.color",
                params![tag, col],
            )?;
        }
        None => {
            c.execute("DELETE FROM tag_style WHERE tag_name = ?1", [tag])?;
        }
    }
    Ok(())
}

/// Colori scelti, per nome del tag in minuscolo.
pub fn colors(c: &Connection) -> StoreResult<HashMap<String, String>> {
    let mut st = c.prepare("SELECT tag_name, color FROM tag_style")?;
    let rows = st
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?.to_lowercase(),
                r.get::<_, String>(1)?,
            ))
        })?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

pub fn color_of(colors: &HashMap<String, String>, tag: &str) -> String {
    colors
        .get(&tag.to_lowercase())
        .cloned()
        .unwrap_or_else(|| default_color(tag).to_string())
}

// ------------------------------------------- propagazione delle modifiche ai tag

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ItemRow {
    set_id: i64,
    position: i64,
    tag_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct StyleRow {
    tag_name: String,
    color: String,
}

/// Stato di set e colori per i tag indicati, da ripristinare con [`restore`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Snapshot {
    names: Vec<String>,
    items: Vec<ItemRow>,
    styles: Vec<StyleRow>,
}

/// Fotografa le righe di `app.db` che riguardano i tag indicati (prima di
/// rinomina, unione o eliminazione), per l'annullamento.
pub fn snapshot(c: &Connection, names: &[&str]) -> StoreResult<serde_json::Value> {
    let mut items = Vec::new();
    let mut styles = Vec::new();
    let mut it = c.prepare(
        "SELECT set_id, position, tag_name FROM tag_set_item WHERE tag_name = ?1 ORDER BY set_id",
    )?;
    let mut sy = c.prepare("SELECT tag_name, color FROM tag_style WHERE tag_name = ?1")?;
    for n in names {
        for row in it.query_map([n.trim()], |r| {
            Ok(ItemRow {
                set_id: r.get(0)?,
                position: r.get(1)?,
                tag_name: r.get(2)?,
            })
        })? {
            items.push(row?);
        }
        for row in sy.query_map([n.trim()], |r| {
            Ok(StyleRow {
                tag_name: r.get(0)?,
                color: r.get(1)?,
            })
        })? {
            styles.push(row?);
        }
    }
    Ok(serde_json::to_value(Snapshot {
        names: names.iter().map(|n| n.trim().to_string()).collect(),
        items,
        styles,
    })?)
}

/// Ripristina lo stato fotografato da [`snapshot`].
pub fn restore(c: &Connection, payload: &serde_json::Value) -> StoreResult<()> {
    let snap: Snapshot = serde_json::from_value(payload.clone())?;
    for n in &snap.names {
        c.execute("DELETE FROM tag_set_item WHERE tag_name = ?1", [n])?;
        c.execute("DELETE FROM tag_style WHERE tag_name = ?1", [n])?;
    }
    for i in &snap.items {
        // Il set potrebbe essere stato eliminato nel frattempo: si salta.
        c.execute(
            "INSERT OR REPLACE INTO tag_set_item (set_id, position, tag_name)
             SELECT ?1, ?2, ?3 WHERE EXISTS (SELECT 1 FROM tag_set WHERE id = ?1)",
            params![i.set_id, i.position, i.tag_name],
        )?;
    }
    for s in &snap.styles {
        c.execute(
            "INSERT OR REPLACE INTO tag_style (tag_name, color) VALUES (?1, ?2)",
            params![s.tag_name, s.color],
        )?;
    }
    Ok(())
}

/// Rinomina il tag nei set e nei colori.
pub fn rename_tag(c: &Connection, from: &str, to: &str) -> StoreResult<()> {
    let to = clean_name(to, "tag")?;
    c.execute(
        "UPDATE tag_set_item SET tag_name = ?2 WHERE tag_name = ?1",
        params![from.trim(), to],
    )?;
    c.execute(
        "UPDATE OR REPLACE tag_style SET tag_name = ?2 WHERE tag_name = ?1",
        params![from.trim(), to],
    )?;
    Ok(())
}

/// Unisce `source` in `target` nei set: dove il set contiene già `target` la
/// voce di `source` sparisce, altrimenti prende il nome di `target` allo stesso
/// tasto. `target` tiene il proprio colore (o eredita quello di `source`).
pub fn merge_tag(c: &Connection, source: &str, target: &str) -> StoreResult<()> {
    let (source, target) = (source.trim(), clean_name(target, "tag")?);
    c.execute(
        "DELETE FROM tag_set_item WHERE tag_name = ?1
           AND set_id IN (SELECT set_id FROM tag_set_item WHERE tag_name = ?2)",
        params![source, target],
    )?;
    c.execute(
        "UPDATE tag_set_item SET tag_name = ?2 WHERE tag_name = ?1",
        params![source, target],
    )?;
    c.execute(
        "UPDATE OR IGNORE tag_style SET tag_name = ?2 WHERE tag_name = ?1",
        params![source, target],
    )?;
    c.execute("DELETE FROM tag_style WHERE tag_name = ?1", [source])?;
    Ok(())
}

/// Toglie il tag da tutti i set e ne dimentica il colore.
pub fn delete_tag(c: &Connection, name: &str) -> StoreResult<()> {
    c.execute(
        "DELETE FROM tag_set_item WHERE tag_name = ?1",
        [name.trim()],
    )?;
    c.execute("DELETE FROM tag_style WHERE tag_name = ?1", [name.trim()])?;
    Ok(())
}

/// Nomi dei set che contengono ciascun tag (chiave in minuscolo).
pub fn sets_by_tag(c: &Connection) -> StoreResult<HashMap<String, Vec<String>>> {
    let mut out: HashMap<String, Vec<String>> = HashMap::new();
    for s in list_sets(c)? {
        for t in s.tags {
            out.entry(t.name.to_lowercase())
                .or_default()
                .push(s.name.clone());
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppStore;

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn create_rename_duplicate_reorder_sets() {
        let s = AppStore::open_in_memory().unwrap();
        s.with(|c| {
            let usa = create_set(
                c,
                "Viaggio USA 2026",
                &names(&["Paesaggi", "Città", "Tramonti", "Persone", "Cibo"]),
            )?;
            let set = get_set(c, usa)?;
            let keys: Vec<(u8, &str)> = set.tags.iter().map(|t| (t.key, t.name.as_str())).collect();
            assert_eq!(
                keys,
                vec![
                    (1, "Paesaggi"),
                    (2, "Città"),
                    (3, "Tramonti"),
                    (4, "Persone"),
                    (5, "Cibo")
                ]
            );
            assert!(matches!(
                create_set(c, "viaggio usa 2026", &[]),
                Err(StoreError::Conflict(_))
            ));
            rename_set(c, usa, "USA")?;
            let copy = duplicate_set(c, usa)?;
            let dup = get_set(c, copy)?;
            assert_eq!(dup.name, "USA (copia)");
            assert_eq!(dup.tags, get_set(c, usa)?.tags);
            assert_eq!(get_set(c, duplicate_set(c, usa)?)?.name, "USA (copia 2)");
            reorder_sets(c, &[copy, usa])?;
            let order: Vec<i64> = list_sets(c)?.iter().map(|s| s.id).collect();
            assert_eq!(&order[..2], &[copy, usa]);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn nine_tag_limit() {
        let s = AppStore::open_in_memory().unwrap();
        s.with(|c| {
            let nine: Vec<String> = (1..=9).map(|i| format!("t{i}")).collect();
            let id = create_set(c, "Pieno", &nine)?;
            let err = add_to_set(c, id, "decimo").unwrap_err();
            assert!(err.to_string().contains("9"), "{err}");
            let ten: Vec<String> = (1..=10).map(|i| format!("t{i}")).collect();
            assert!(set_tags(c, id, &ten).is_err());
            // Un tag già presente non conta come nuovo.
            assert_eq!(add_to_set(c, id, "T3")?, 3);
            remove_from_set(c, id, "t4")?;
            assert_eq!(add_to_set(c, id, "nuovo")?, 4);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn deleting_set_keeps_tags_and_moves_active_to_base() {
        let s = AppStore::open_in_memory().unwrap();
        s.with(|c| {
            let id = create_set(c, "Eventi", &names(&["Matrimonio", "Compleanno"]))?;
            set_active(c, id)?;
            set_color(c, "Matrimonio", Some("yellow"))?;
            delete_set(c, id)?;
            assert_eq!(active_set(c)?, Some(1));
            // Il colore del tag (che vive con il nome) resta.
            assert_eq!(color_of(&colors(c)?, "matrimonio"), "yellow");
            assert!(delete_set(c, 1).is_err(), "il set Base resta");
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn exclusive_siblings_only_from_exclusive_sets() {
        let s = AppStore::open_in_memory().unwrap();
        s.with(|c| {
            let usa = create_set(c, "USA", &names(&["Paesaggi", "Città", "Tramonti"]))?;
            create_set(c, "Altro", &names(&["Tramonti", "Cibo"]))?;
            assert!(exclusive_siblings(c, "Tramonti")?.is_empty());
            set_exclusive(c, usa, true)?;
            assert_eq!(
                exclusive_siblings(c, "tramonti")?,
                names(&["Città", "Paesaggi"])
            );
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn rename_merge_delete_propagate_and_restore() {
        let s = AppStore::open_in_memory().unwrap();
        s.with(|c| {
            let usa = create_set(c, "USA", &names(&["Paesaggi", "Sunset", "Tramonti"]))?;
            let eventi = create_set(c, "Eventi", &names(&["Sunset", "Cena"]))?;
            set_color(c, "Sunset", Some("coral"))?;

            let snap = snapshot(c, &["Sunset", "Tramonti"])?;
            merge_tag(c, "Sunset", "Tramonti")?;
            // In USA "Tramonti" c'era già: la voce di Sunset sparisce.
            assert_eq!(
                get_set(c, usa)?.tag_names(),
                names(&["Paesaggi", "Tramonti"])
            );
            // In Eventi prende il tasto di Sunset.
            assert_eq!(
                get_set(c, eventi)?.tags[0],
                SetTag {
                    key: 1,
                    name: "Tramonti".into()
                }
            );
            assert_eq!(color_of(&colors(c)?, "Tramonti"), "coral");
            restore(c, &snap)?;
            assert_eq!(
                get_set(c, usa)?.tag_names(),
                names(&["Paesaggi", "Sunset", "Tramonti"])
            );
            assert_eq!(get_set(c, eventi)?.tag_names(), names(&["Sunset", "Cena"]));
            assert_eq!(color_of(&colors(c)?, "Sunset"), "coral");
            assert_eq!(colors(c)?.get("tramonti"), None);

            let snap = snapshot(c, &["Cena", "Cene"])?;
            rename_tag(c, "cena", "Cene")?;
            assert_eq!(get_set(c, eventi)?.tags[1].name, "Cene");
            restore(c, &snap)?;
            assert_eq!(get_set(c, eventi)?.tags[1].name, "Cena");

            let snap = snapshot(c, &["Sunset"])?;
            delete_tag(c, "Sunset")?;
            assert!(!sets_by_tag(c)?.contains_key("sunset"));
            restore(c, &snap)?;
            assert_eq!(sets_by_tag(c)?["sunset"], names(&["USA", "Eventi"]));
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn colors_from_fixed_palette_with_default() {
        let s = AppStore::open_in_memory().unwrap();
        s.with(|c| {
            assert!(set_color(c, "Tramonti", Some("#ff0000")).is_err());
            set_color(c, "Tramonti", Some("yellow"))?;
            let map = colors(c)?;
            assert_eq!(color_of(&map, "TRAMONTI"), "yellow");
            let d = color_of(&map, "Nuovo");
            assert!(PALETTE.contains(&d.as_str()));
            assert_eq!(d, color_of(&map, "nuovo"));
            set_color(c, "Tramonti", None)?;
            assert_eq!(color_of(&colors(c)?, "Tramonti"), default_color("Tramonti"));
            Ok(())
        })
        .unwrap();
    }
}
