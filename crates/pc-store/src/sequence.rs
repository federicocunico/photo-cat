//! Vista Sequenza: un ambito (libreria, catalogo, selezione) in ordine di
//! istante reale, diviso in sessioni separate da pause oltre una soglia.
//! Le foto senza data affidabile (fonte "file system" e non collocate) stanno
//! in un'area a parte.

use crate::model::MediaId;
use crate::query::{compile_filter, LibraryFilter};
use crate::timeline::local_text;
use crate::StoreResult;
use rusqlite::types::Value;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

/// Soglia predefinita tra due sessioni: 2 ore.
pub const DEFAULT_SESSION_GAP_MIN: i64 = 120;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "scope", rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum SequenceScope {
    Library,
    Catalog { id: i64 },
    Selection { ids: Vec<MediaId> },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SequenceItem {
    pub id: MediaId,
    pub name: String,
    pub thumb: Option<String>,
    pub instant_utc: i64,
    /// Ora locale mostrata (`AAAA-MM-GG HH:MM:SS`).
    pub local: String,
    pub local_offset_min: i32,
    pub presumed: bool,
    pub group_id: Option<i64>,
    pub device: Option<String>,
    /// Indice della sessione (da 0).
    pub session: i64,
    /// Tag del file che appartengono al set indicato.
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SessionView {
    pub index: i64,
    pub start_local: String,
    pub end_local: String,
    pub start_utc: i64,
    pub end_utc: i64,
    /// Luogo più frequente della sessione.
    pub place: Option<String>,
    pub count: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SequenceCursor {
    pub instant_utc: i64,
    pub id: MediaId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SequencePage {
    pub items: Vec<SequenceItem>,
    pub next: Option<SequenceCursor>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct UndatedItem {
    pub id: MediaId,
    pub name: String,
    pub thumb: Option<String>,
    /// Data del file system (inaffidabile, solo indicativa).
    pub file_date: Option<String>,
}

/// Condizione SQL dell'ambito (su `m`/`d`).
fn scope_where(c: &Connection, scope: &SequenceScope) -> StoreResult<(String, Vec<Value>)> {
    Ok(match scope {
        SequenceScope::Library => ("1=1".into(), Vec::new()),
        SequenceScope::Catalog { id } => {
            let f = compile_filter(
                c,
                &LibraryFilter {
                    catalog_id: Some(*id),
                    ..Default::default()
                },
            )?;
            (f.sql, f.args)
        }
        SequenceScope::Selection { ids } => (
            "m.id IN (SELECT value FROM json_each(?))".into(),
            vec![Value::Text(serde_json::to_string(ids)?)],
        ),
    })
}

/// Foto datate (o collocate) dell'ambito.
const DATED: &str = "d.instant_utc IS NOT NULL
    AND (d.captured_source IS NULL OR d.captured_source <> 'file_system'
         OR d.media_id IN (SELECT media_id FROM media_time_placement))";

/// Sottoquery con l'indice di sessione di ogni foto (pause oltre `gap_ms`).
fn sessions_cte(scope_sql: &str) -> String {
    format!(
        "WITH s AS (
            SELECT m.id, d.instant_utc,
                   CASE WHEN d.instant_utc - LAG(d.instant_utc) OVER (ORDER BY d.instant_utc, m.id) > ?
                        THEN 1 ELSE 0 END AS brk
            FROM media_item m JOIN media_dates d ON d.media_id = m.id
            LEFT JOIN media_tech t ON t.media_id = m.id
            WHERE {DATED} AND {scope_sql}
         ), q AS (
            SELECT id, instant_utc, SUM(brk) OVER (ORDER BY instant_utc, id) AS session FROM s
         )"
    )
}

/// Una pagina della sequenza, dopo `after` (paginazione keyset su istante e id).
pub fn page(
    c: &Connection,
    scope: &SequenceScope,
    gap_min: i64,
    set_tags: &[String],
    after: Option<&SequenceCursor>,
    limit: u32,
) -> StoreResult<SequencePage> {
    let (scope_sql, scope_args) = scope_where(c, scope)?;
    let limit = limit.clamp(1, 5000);
    let mut args: Vec<Value> = vec![Value::Integer(gap_min.max(1) * 60_000)];
    args.extend(scope_args);
    // Parametri nell'ordine in cui compaiono nel testo SQL.
    args.push(Value::Text(serde_json::to_string(set_tags)?));
    let mut cond = String::from("1=1");
    if let Some(cur) = after {
        cond = "(q.instant_utc > ? OR (q.instant_utc = ? AND q.id > ?))".into();
        args.push(Value::Integer(cur.instant_utc));
        args.push(Value::Integer(cur.instant_utc));
        args.push(Value::Integer(cur.id));
    }
    args.push(Value::Integer(limit as i64 + 1));
    let sql = format!(
        "{} SELECT q.id, m.name, m.thumb, q.instant_utc, COALESCE(d.local_offset_min, 0), d.instant_presumed,
                  d.group_id, g.device, q.session,
                  (SELECT group_concat(tg.name, char(31)) FROM media_tag mt JOIN tag tg ON tg.id = mt.tag_id
                    WHERE mt.media_id = q.id AND tg.name IN (SELECT value FROM json_each(?)))
           FROM q JOIN media_item m ON m.id = q.id JOIN media_dates d ON d.media_id = q.id
           LEFT JOIN source_group g ON g.id = d.group_id
           WHERE {cond}
           ORDER BY q.instant_utc, q.id LIMIT ?",
        sessions_cte(&scope_sql),
    );
    let mut st = c.prepare(&sql)?;
    let mut items: Vec<SequenceItem> = st
        .query_map(rusqlite::params_from_iter(args.iter()), |r| {
            let utc: i64 = r.get(3)?;
            let off: i32 = r.get(4)?;
            let tags: Option<String> = r.get(9)?;
            Ok(SequenceItem {
                id: r.get(0)?,
                name: r.get(1)?,
                thumb: r.get(2)?,
                instant_utc: utc,
                local: local_text(utc, off).unwrap_or_default(),
                local_offset_min: off,
                presumed: r.get(5)?,
                group_id: r.get(6)?,
                device: r.get(7)?,
                session: r.get(8)?,
                tags: tags
                    .map(|s| s.split(char::from(0x1f)).map(str::to_string).collect())
                    .unwrap_or_default(),
            })
        })?
        .collect::<Result<_, _>>()?;
    let next = if items.len() > limit as usize {
        items.truncate(limit as usize);
        items.last().map(|i| SequenceCursor {
            instant_utc: i.instant_utc,
            id: i.id,
        })
    } else {
        None
    };
    Ok(SequencePage { items, next })
}

/// Riepilogo delle sessioni dell'ambito: date, intervallo, luogo e numero di foto.
pub fn sessions(
    c: &Connection,
    scope: &SequenceScope,
    gap_min: i64,
) -> StoreResult<Vec<SessionView>> {
    let (scope_sql, scope_args) = scope_where(c, scope)?;
    let mut args: Vec<Value> = vec![Value::Integer(gap_min.max(1) * 60_000)];
    args.extend(scope_args);
    let sql = format!(
        "{} , agg AS (
            SELECT q.session, MIN(q.instant_utc) AS a, MAX(q.instant_utc) AS b, count(*) AS n
            FROM q GROUP BY q.session
         ), places AS (
            SELECT q.session, lo.name, count(*) AS k,
                   ROW_NUMBER() OVER (PARTITION BY q.session ORDER BY count(*) DESC, lo.name) AS rk
            FROM q JOIN location lo ON lo.media_id = q.id WHERE lo.name IS NOT NULL
            GROUP BY q.session, lo.name
         )
         SELECT agg.session, agg.a, agg.b, agg.n, p.name,
                (SELECT COALESCE(d.local_offset_min, 0) FROM q JOIN media_dates d ON d.media_id = q.id
                  WHERE q.session = agg.session ORDER BY q.instant_utc LIMIT 1),
                (SELECT COALESCE(d.local_offset_min, 0) FROM q JOIN media_dates d ON d.media_id = q.id
                  WHERE q.session = agg.session ORDER BY q.instant_utc DESC LIMIT 1)
         FROM agg LEFT JOIN places p ON p.session = agg.session AND p.rk = 1
         ORDER BY agg.session",
        sessions_cte(&scope_sql)
    );
    let mut st = c.prepare(&sql)?;
    let rows = st
        .query_map(rusqlite::params_from_iter(args.iter()), |r| {
            let (a, b): (i64, i64) = (r.get(1)?, r.get(2)?);
            let (oa, ob): (i32, i32) = (r.get(5)?, r.get(6)?);
            Ok(SessionView {
                index: r.get(0)?,
                start_utc: a,
                end_utc: b,
                start_local: local_text(a, oa).unwrap_or_default(),
                end_local: local_text(b, ob).unwrap_or_default(),
                count: r.get(3)?,
                place: r.get(4)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

/// Foto senza data affidabile dell'ambito, non ancora collocate.
pub fn undated(c: &Connection, scope: &SequenceScope) -> StoreResult<Vec<UndatedItem>> {
    let (scope_sql, args) = scope_where(c, scope)?;
    let sql = format!(
        "SELECT m.id, m.name, m.thumb, d.captured_at
         FROM media_item m JOIN media_dates d ON d.media_id = m.id
         LEFT JOIN media_tech t ON t.media_id = m.id
         WHERE (d.captured_source = 'file_system' OR d.captured_at IS NULL)
           AND m.id NOT IN (SELECT media_id FROM media_time_placement)
           AND {scope_sql}
         ORDER BY m.name COLLATE NOCASE, m.id"
    );
    let mut st = c.prepare(&sql)?;
    let rows = st
        .query_map(rusqlite::params_from_iter(args.iter()), |r| {
            Ok(UndatedItem {
                id: r.get(0)?,
                name: r.get(1)?,
                thumb: r.get(2)?,
                file_date: r.get(3)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

/// Id di tutte le foto di una sessione (tasto A).
pub fn session_ids(
    c: &Connection,
    scope: &SequenceScope,
    gap_min: i64,
    session: i64,
) -> StoreResult<Vec<MediaId>> {
    let (scope_sql, scope_args) = scope_where(c, scope)?;
    let mut args: Vec<Value> = vec![Value::Integer(gap_min.max(1) * 60_000)];
    args.extend(scope_args);
    args.push(Value::Integer(session));
    let sql = format!(
        "{} SELECT id FROM q WHERE session = ? ORDER BY instant_utc, id",
        sessions_cte(&scope_sql)
    );
    let mut st = c.prepare(&sql)?;
    let rows = st
        .query_map(rusqlite::params_from_iter(args.iter()), |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

/// Tutti gli id della sequenza in ordine (export e selezione dell'ambito).
pub fn ordered_ids(c: &Connection, scope: &SequenceScope) -> StoreResult<Vec<MediaId>> {
    let (scope_sql, args) = scope_where(c, scope)?;
    let sql = format!(
        "SELECT m.id FROM media_item m JOIN media_dates d ON d.media_id = m.id
         LEFT JOIN media_tech t ON t.media_id = m.id
         WHERE {DATED} AND {scope_sql} ORDER BY d.instant_utc, m.id"
    );
    let mut st = c.prepare(&sql)?;
    let rows = st
        .query_map(rusqlite::params_from_iter(args.iter()), |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}
