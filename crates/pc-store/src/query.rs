//! Query della libreria: filtri combinati, ricerca testuale, cataloghi
//! (manuali e smart), facet e paginazione keyset.

use crate::model::{CatalogKind, MediaId, MediaStatus};
use crate::rules::{self, SqlFragment};
use crate::{repo, StoreError, StoreResult};
use rusqlite::types::Value;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct LabelFilter {
    pub group: String,
    /// Valori ammessi (OR all'interno del gruppo).
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum Presence {
    Present,
    Absent,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct LibraryFilter {
    pub source_ids: Vec<i64>,
    pub catalog_id: Option<i64>,
    /// AND tra gruppi, OR tra i valori dello stesso gruppo.
    pub labels: Vec<LabelFilter>,
    /// Tutte le persone indicate devono essere presenti.
    pub person_ids: Vec<i64>,
    pub people: Option<Presence>,
    pub places: Vec<String>,
    /// `AAAA-MM-GG` inclusivo.
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub formats: Vec<String>,
    /// Tutti i tag indicati devono essere presenti.
    pub tags: Vec<String>,
    pub text: Option<String>,
    pub status: Option<MediaStatus>,
    pub duplicates_only: bool,
    /// Regola smart aggiuntiva (anteprima nell'editor dei cataloghi).
    pub rule: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum SortKey {
    #[default]
    CapturedDesc,
    CapturedAsc,
    NameAsc,
    SizeDesc,
    AddedDesc,
}

impl SortKey {
    fn expr(self) -> &'static str {
        match self {
            Self::CapturedDesc | Self::CapturedAsc => "COALESCE(d.captured_at, '')",
            Self::NameAsc => "m.name COLLATE NOCASE",
            Self::SizeDesc => "m.size",
            Self::AddedDesc => "m.added_at",
        }
    }
    fn desc(self) -> bool {
        matches!(self, Self::CapturedDesc | Self::SizeDesc | Self::AddedDesc)
    }
}

/// Posizione dopo l'ultimo elemento della pagina precedente.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Cursor {
    pub key: serde_json::Value,
    pub id: MediaId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct GridItem {
    pub id: MediaId,
    pub name: String,
    pub path: String,
    pub thumb: Option<String>,
    pub captured_at: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub status: MediaStatus,
    pub faces: i64,
    pub analyzed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Page {
    pub items: Vec<GridItem>,
    pub next: Option<Cursor>,
}

const FROM: &str = "FROM media_item m
    LEFT JOIN media_dates d ON d.media_id = m.id
    LEFT JOIN media_tech t ON t.media_id = m.id";

fn placeholders(n: usize) -> String {
    vec!["?"; n].join(", ")
}

/// Compila il filtro in una clausola `WHERE` (sempre presente).
pub fn compile_filter(c: &Connection, f: &LibraryFilter) -> StoreResult<SqlFragment> {
    let mut parts: Vec<String> = vec!["1=1".into()];
    let mut args: Vec<Value> = Vec::new();

    if !f.source_ids.is_empty() {
        parts.push(format!(
            "m.source_id IN ({})",
            placeholders(f.source_ids.len())
        ));
        args.extend(f.source_ids.iter().map(|v| Value::Integer(*v)));
    }
    if let Some(cid) = f.catalog_id {
        let cat = repo::catalog::get(c, cid)?;
        match cat.kind {
            CatalogKind::Manual => {
                parts.push(
                    "m.id IN (SELECT media_id FROM catalog_item WHERE catalog_id = ?)".into(),
                );
                args.push(Value::Integer(cid));
            }
            CatalogKind::Smart => {
                let json = cat.rules.ok_or_else(|| {
                    StoreError::Invalid(format!("catalogo smart {cid} senza regole"))
                })?;
                let frag = rules::compile(&rules::parse(&json)?)?;
                parts.push(frag.sql);
                args.extend(frag.args);
            }
        }
    }
    if let Some(json) = &f.rule {
        let frag = rules::compile(&rules::parse(json)?)?;
        parts.push(frag.sql);
        args.extend(frag.args);
    }
    for lf in f.labels.iter().filter(|l| !l.values.is_empty()) {
        parts.push(format!(
            "EXISTS (SELECT 1 FROM label l WHERE l.media_id = m.id AND l.grp = ? AND l.value IN ({}))",
            placeholders(lf.values.len())
        ));
        args.push(Value::Text(lf.group.clone()));
        args.extend(lf.values.iter().map(|v| Value::Text(v.clone())));
    }
    for pid in &f.person_ids {
        parts.push(
            "EXISTS (SELECT 1 FROM face fc WHERE fc.media_id = m.id AND fc.person_id = ?)".into(),
        );
        args.push(Value::Integer(*pid));
    }
    match f.people {
        Some(Presence::Present) => parts.push(
            "EXISTS (SELECT 1 FROM media_people mp WHERE mp.media_id = m.id AND mp.count > 0)"
                .into(),
        ),
        Some(Presence::Absent) => parts.push(
            "EXISTS (SELECT 1 FROM media_people mp WHERE mp.media_id = m.id AND mp.count = 0)"
                .into(),
        ),
        None => {}
    }
    if !f.places.is_empty() {
        parts.push(format!(
            "EXISTS (SELECT 1 FROM location lo WHERE lo.media_id = m.id AND lo.name IN ({}))",
            placeholders(f.places.len())
        ));
        args.extend(f.places.iter().map(|v| Value::Text(v.clone())));
    }
    if let Some(from) = &f.date_from {
        parts.push("d.captured_at >= ?".into());
        args.push(Value::Text(from.clone()));
    }
    if let Some(to) = &f.date_to {
        parts.push("d.captured_at <= ?".into());
        args.push(Value::Text(format!("{to} 99")));
    }
    if !f.formats.is_empty() {
        parts.push(format!("m.ext IN ({})", placeholders(f.formats.len())));
        args.extend(
            f.formats
                .iter()
                .map(|v| Value::Text(v.to_ascii_lowercase())),
        );
    }
    for tag in &f.tags {
        parts.push(
            "EXISTS (SELECT 1 FROM media_tag mt JOIN tag tg ON tg.id = mt.tag_id WHERE mt.media_id = m.id AND tg.name = ?)"
                .into(),
        );
        args.push(Value::Text(tag.clone()));
    }
    if let Some(text) = f.text.as_deref().filter(|t| !t.trim().is_empty()) {
        parts.push("m.id IN (SELECT rowid FROM media_fts WHERE media_fts MATCH ?)".into());
        args.push(Value::Text(rules::fts_query(text)));
    }
    if let Some(status) = f.status {
        parts.push("m.status = ?".into());
        args.push(Value::Text(status.as_str().into()));
    }
    if f.duplicates_only {
        parts.push(
            "m.content_hash IS NOT NULL AND EXISTS (SELECT 1 FROM media_item m2 WHERE m2.content_hash = m.content_hash AND m2.id <> m.id)"
                .into(),
        );
    }
    Ok(SqlFragment {
        sql: parts.join(" AND "),
        args,
    })
}

/// Una pagina di risultati ordinata, a partire da `after`.
pub fn page(
    c: &Connection,
    f: &LibraryFilter,
    sort: SortKey,
    after: Option<&Cursor>,
    limit: u32,
) -> StoreResult<Page> {
    let mut w = compile_filter(c, f)?;
    let key = sort.expr();
    let (op, dir) = if sort.desc() {
        ("<", "DESC")
    } else {
        (">", "ASC")
    };
    if let Some(cur) = after {
        w.sql.push_str(&format!(
            " AND ({key} {op} ? OR ({key} = ? AND m.id {op} ?))"
        ));
        let k = json_to_value(&cur.key);
        w.args.push(k.clone());
        w.args.push(k);
        w.args.push(Value::Integer(cur.id));
    }
    let limit = limit.clamp(1, 2000);
    let sql = format!(
        "SELECT m.id, m.name, m.path, m.thumb, d.captured_at, t.width, t.height, m.status,
                (SELECT count(*) FROM face fc WHERE fc.media_id = m.id),
                EXISTS (SELECT 1 FROM label l WHERE l.media_id = m.id AND l.origin = 'auto'),
                {key}
         {FROM} WHERE {} ORDER BY {key} {dir}, m.id {dir} LIMIT ?",
        w.sql
    );
    w.args.push(Value::Integer(limit as i64 + 1));
    let mut st = c.prepare(&sql)?;
    let mut keys: Vec<serde_json::Value> = Vec::new();
    let mut items: Vec<GridItem> = st
        .query_map(rusqlite::params_from_iter(w.args.iter()), |r| {
            let key: Value = r.get(10)?;
            keys.push(value_to_json(key));
            Ok(GridItem {
                id: r.get(0)?,
                name: r.get(1)?,
                path: r.get(2)?,
                thumb: r.get(3)?,
                captured_at: r.get(4)?,
                width: r.get(5)?,
                height: r.get(6)?,
                status: MediaStatus::parse(&r.get::<_, String>(7)?),
                faces: r.get(8)?,
                analyzed: r.get(9)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    let next = if items.len() > limit as usize {
        items.truncate(limit as usize);
        let last = items.last().expect("pagina non vuota");
        Some(Cursor {
            key: keys[limit as usize - 1].clone(),
            id: last.id,
        })
    } else {
        None
    };
    Ok(Page { items, next })
}

/// Tutti gli id che soddisfano il filtro (per "seleziona tutto" e azioni bulk).
pub fn ids(c: &Connection, f: &LibraryFilter) -> StoreResult<Vec<MediaId>> {
    let w = compile_filter(c, f)?;
    let mut st = c.prepare(&format!("SELECT m.id {FROM} WHERE {} ORDER BY m.id", w.sql))?;
    let rows = st
        .query_map(rusqlite::params_from_iter(w.args.iter()), |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

pub fn count(c: &Connection, f: &LibraryFilter) -> StoreResult<i64> {
    let w = compile_filter(c, f)?;
    Ok(c.query_row(
        &format!("SELECT count(*) {FROM} WHERE {}", w.sql),
        rusqlite::params_from_iter(w.args.iter()),
        |r| r.get(0),
    )?)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct FacetValue {
    pub value: String,
    pub count: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Facets {
    /// gruppo → valori
    pub labels: Vec<(String, Vec<FacetValue>)>,
    pub places: Vec<FacetValue>,
    pub years: Vec<FacetValue>,
    pub formats: Vec<FacetValue>,
    pub people: Vec<FacetValue>,
    pub tags: Vec<FacetValue>,
}

/// Conteggi per i filtri del pannello laterale, calcolati sui risultati correnti.
pub fn facets(c: &Connection, f: &LibraryFilter) -> StoreResult<Facets> {
    let w = compile_filter(c, f)?;
    let base = format!("WITH f AS (SELECT m.id {FROM} WHERE {})", w.sql);
    let run = |select: &str| -> StoreResult<Vec<(String, String, i64)>> {
        let mut st = c.prepare(&format!("{base} {select}"))?;
        let rows = st
            .query_map(rusqlite::params_from_iter(w.args.iter()), |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?.unwrap_or_default(),
                    r.get(1)?,
                    r.get(2)?,
                ))
            })?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    };
    let simple = |rows: Vec<(String, String, i64)>| -> Vec<FacetValue> {
        rows.into_iter()
            .map(|(_, value, count)| FacetValue { value, count })
            .collect()
    };
    let mut out = Facets::default();
    for (grp, value, count) in run(
        "SELECT l.grp, l.value, count(*) FROM label l JOIN f ON f.id = l.media_id
         GROUP BY l.grp, l.value ORDER BY l.grp, count(*) DESC",
    )? {
        match out.labels.last_mut() {
            Some((g, vals)) if *g == grp => vals.push(FacetValue { value, count }),
            _ => out.labels.push((grp, vec![FacetValue { value, count }])),
        }
    }
    out.places = simple(run(
        "SELECT NULL, lo.name, count(*) FROM location lo JOIN f ON f.id = lo.media_id
         WHERE lo.name IS NOT NULL GROUP BY lo.name ORDER BY count(*) DESC LIMIT 100",
    )?);
    out.years = simple(run(
        "SELECT NULL, substr(d.captured_at, 1, 4), count(*) FROM media_dates d JOIN f ON f.id = d.media_id
         WHERE d.captured_at IS NOT NULL GROUP BY 2 ORDER BY 2 DESC",
    )?);
    out.formats = simple(run(
        "SELECT NULL, m.ext, count(*) FROM media_item m JOIN f ON f.id = m.id GROUP BY m.ext ORDER BY count(*) DESC",
    )?);
    out.people = simple(run(
        "SELECT NULL, p.name, count(DISTINCT fc.media_id) FROM face fc JOIN f ON f.id = fc.media_id
         JOIN person p ON p.id = fc.person_id WHERE p.name IS NOT NULL GROUP BY p.id ORDER BY 3 DESC LIMIT 100",
    )?);
    out.tags = simple(run(
        "SELECT NULL, tg.name, count(*) FROM media_tag mt JOIN f ON f.id = mt.media_id
         JOIN tag tg ON tg.id = mt.tag_id GROUP BY tg.id ORDER BY 3 DESC LIMIT 100",
    )?);
    Ok(out)
}

fn json_to_value(v: &serde_json::Value) -> Value {
    match v {
        serde_json::Value::Number(n) if n.is_i64() => Value::Integer(n.as_i64().unwrap()),
        serde_json::Value::Number(n) => Value::Real(n.as_f64().unwrap_or_default()),
        serde_json::Value::String(s) => Value::Text(s.clone()),
        serde_json::Value::Null => Value::Null,
        other => Value::Text(other.to_string()),
    }
}

fn value_to_json(v: Value) -> serde_json::Value {
    match v {
        Value::Integer(i) => i.into(),
        Value::Real(f) => f.into(),
        Value::Text(s) => s.into(),
        _ => serde_json::Value::Null,
    }
}
