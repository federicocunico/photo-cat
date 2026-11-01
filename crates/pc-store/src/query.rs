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
    /// Cartella (con o senza sottocartelle).
    pub folder: Option<FolderFilter>,
    /// Solo questi elementi (collegamenti dal riepilogo di un aggiornamento).
    pub ids: Option<Vec<MediaId>>,
}

/// Filtro per cartella sul percorso dei file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct FolderFilter {
    pub path: String,
    /// Includere i file delle sottocartelle.
    pub recursive: bool,
}

/// Cartella dell'albero di una sorgente.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct FolderNode {
    pub path: String,
    pub name: String,
    /// File contenuti, sottocartelle incluse.
    pub items: i64,
    pub has_children: bool,
}

/// Confronto dei percorsi senza maiuscole (Windows) o esatto (macOS/Linux).
const PATHS_IGNORE_CASE: bool = cfg!(windows);

/// Separatore e prefisso normalizzato (senza separatori finali) di una cartella.
fn folder_prefix(path: &str) -> (char, String) {
    let sep = if path.contains('\\') { '\\' } else { '/' };
    (sep, path.trim_end_matches(['\\', '/']).to_string())
}

/// Condizione "il file sta sotto `prefix`" che usa l'indice univoco su
/// `media_item.path` (intervallo) e, senza maiuscole, un confronto esatto
/// `NOCASE` sull'inizio del percorso.
fn under_folder(prefix: &str, sep: char, args: &mut Vec<Value>) -> String {
    let next = char::from_u32(sep as u32 + 1).expect("separatore ASCII");
    let mut sql = String::from("m.path >= ? AND m.path < ?");
    if PATHS_IGNORE_CASE {
        args.push(Value::Text(format!("{}{sep}", prefix.to_uppercase())));
        args.push(Value::Text(format!("{}{next}", prefix.to_lowercase())));
        sql.push_str(" AND substr(m.path, 1, ?) = ? COLLATE NOCASE");
        args.push(Value::Integer(prefix.chars().count() as i64 + 1));
        args.push(Value::Text(format!("{prefix}{sep}")));
    } else {
        args.push(Value::Text(format!("{prefix}{sep}")));
        args.push(Value::Text(format!("{prefix}{next}")));
    }
    sql
}

/// Sottocartelle di `parent` (o della radice della sorgente) un livello alla
/// volta, con il numero di file contenuti (sottocartelle incluse).
pub fn list_folders(
    c: &Connection,
    source_id: i64,
    parent: Option<&str>,
) -> StoreResult<Vec<FolderNode>> {
    let root = match parent {
        Some(p) => p.to_string(),
        None => repo::source::get(c, source_id)?.path,
    };
    let (sep, prefix) = folder_prefix(&root);
    let mut args = Vec::new();
    let cond = under_folder(&prefix, sep, &mut args);
    let n = prefix.chars().count() as i64;
    let group = if PATHS_IGNORE_CASE {
        "GROUP BY seg COLLATE NOCASE ORDER BY seg COLLATE NOCASE"
    } else {
        "GROUP BY seg ORDER BY seg"
    };
    let sql = format!(
        "SELECT min(seg), count(*), max(instr(substr(rest, length(seg) + 2), ?) > 0) FROM (
            SELECT substr(rest, 1, instr(rest, ?) - 1) AS seg, rest FROM (
                SELECT substr(m.path, {start}) AS rest FROM media_item m
                WHERE m.source_id = ? AND {cond}
            ) WHERE instr(rest, ?) > 0
        ) {group}",
        start = n + 2,
    );
    let mut all: Vec<Value> = vec![
        Value::Text(sep.to_string()),
        Value::Text(sep.to_string()),
        Value::Integer(source_id),
    ];
    all.extend(args);
    all.push(Value::Text(sep.to_string()));
    let mut q = c.prepare(&sql)?;
    let rows = q
        .query_map(rusqlite::params_from_iter(all), |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, bool>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows
        .into_iter()
        .map(|(name, items, has_children)| FolderNode {
            path: format!("{prefix}{sep}{name}"),
            name,
            items,
            has_children,
        })
        .collect())
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

/// Ora locale corretta (`AAAA-MM-GG HH:MM:SS`): istante reale più il fuso
/// dell'ora mostrata; per i file non ancora elaborati, l'ora letta dal file.
pub const LOCAL_TIME_SQL: &str = "COALESCE(datetime(d.instant_utc / 1000 + COALESCE(d.local_offset_min, 0) * 60, 'unixepoch'), d.captured_at)";

/// Anno dell'ora locale corretta.
pub const LOCAL_YEAR_SQL: &str = "CAST(substr(COALESCE(datetime(d.instant_utc / 1000 + COALESCE(d.local_offset_min, 0) * 60, 'unixepoch'), d.captured_at), 1, 4) AS INTEGER)";

/// Chiave di ordinamento per istante reale (ms UTC); senza istante si usa
/// l'ora letta dal file, senza data in fondo.
const INSTANT_SORT_SQL: &str = "COALESCE(d.instant_utc, CAST(strftime('%s', d.captured_at) AS INTEGER) * 1000, -9223372036854775807)";

impl SortKey {
    fn expr(self) -> &'static str {
        match self {
            Self::CapturedDesc | Self::CapturedAsc => INSTANT_SORT_SQL,
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
    /// Tag utente del file (per i chip colorati della griglia).
    pub tags: Vec<String>,
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

    if let Some(ids) = &f.ids {
        parts.push("m.id IN (SELECT value FROM json_each(?))".into());
        args.push(Value::Text(serde_json::to_string(ids)?));
    }
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
    if let Some(folder) = f.folder.as_ref().filter(|d| !d.path.trim().is_empty()) {
        let (sep, prefix) = folder_prefix(&folder.path);
        parts.push(under_folder(&prefix, sep, &mut args));
        if !folder.recursive {
            parts.push(format!(
                "instr(substr(m.path, {}), ?) = 0",
                prefix.chars().count() + 2
            ));
            args.push(Value::Text(sep.to_string()));
        }
    }
    if !f.places.is_empty() {
        parts.push(format!(
            "EXISTS (SELECT 1 FROM location lo WHERE lo.media_id = m.id AND lo.name IN ({}))",
            placeholders(f.places.len())
        ));
        args.extend(f.places.iter().map(|v| Value::Text(v.clone())));
    }
    if let Some(from) = &f.date_from {
        parts.push(format!("{LOCAL_TIME_SQL} >= ?"));
        args.push(Value::Text(from.clone()));
    }
    if let Some(to) = &f.date_to {
        parts.push(format!("{LOCAL_TIME_SQL} <= ?"));
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
                {key},
                (SELECT group_concat(tg.name, char(31)) FROM media_tag mt JOIN tag tg ON tg.id = mt.tag_id
                 WHERE mt.media_id = m.id)
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
                tags: split_tags(r.get(11)?),
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

/// Valori di una lista di filtro: i principali (più quelli selezionati) e il
/// numero totale di voci.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct FacetList {
    pub values: Vec<FacetValue>,
    pub total: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Facets {
    /// gruppo → valori
    pub labels: Vec<(String, FacetList)>,
    pub places: FacetList,
    pub years: FacetList,
    pub formats: FacetList,
    pub people: FacetList,
    pub tags: FacetList,
}

/// Valori restituiti per lista nel riepilogo dei filtri (l'elenco completo
/// si chiede con [`facet_values`]).
pub const FACET_TOP: usize = 50;

/// Una lista di filtro.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum FacetKey {
    Label { group: String },
    Place,
    Year,
    Format,
    Tag,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum FacetSort {
    /// Per numero di foto (decrescente).
    #[default]
    Count,
    /// Alfabetico, senza maiuscole e accenti.
    Name,
}

/// Una pagina dell'elenco completo dei valori di una lista.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct FacetPage {
    pub values: Vec<FacetValue>,
    /// Voci che corrispondono alla ricerca.
    pub total: i64,
}

impl FacetKey {
    /// Il filtro senza la selezione di questa lista (`None` se la lista non ha
    /// selezione): i suoi conteggi applicano tutti gli altri filtri, così le
    /// altre voci della lista restano selezionabili.
    fn without_own(&self, f: &LibraryFilter) -> Option<LibraryFilter> {
        let mut g = f.clone();
        let changed = match self {
            Self::Label { group } => {
                let before = g.labels.len();
                g.labels
                    .retain(|l| &l.group != group || l.values.is_empty());
                g.labels.len() != before
            }
            Self::Place => !std::mem::take(&mut g.places).is_empty(),
            Self::Year => g.date_from.take().is_some() | g.date_to.take().is_some(),
            Self::Format => !std::mem::take(&mut g.formats).is_empty(),
            Self::Tag => !std::mem::take(&mut g.tags).is_empty(),
        };
        changed.then_some(g)
    }

    /// Valori selezionati nel filtro per questa lista.
    fn selected(&self, f: &LibraryFilter) -> Vec<String> {
        match self {
            Self::Label { group } => f
                .labels
                .iter()
                .filter(|l| &l.group == group)
                .flat_map(|l| l.values.clone())
                .collect(),
            Self::Place => f.places.clone(),
            Self::Year => f
                .date_from
                .as_deref()
                .and_then(|d| d.get(..4))
                .map(|y| vec![y.to_string()])
                .unwrap_or_default(),
            Self::Format => f.formats.clone(),
            Self::Tag => f.tags.clone(),
        }
    }

    /// Query `(valore, conteggio)` sulle foto del CTE `f`, ordinate per conteggio.
    fn select(&self) -> String {
        match self {
            Self::Label { .. } => "SELECT l.value, count(*) FROM label l JOIN f ON f.id = l.media_id \
                 WHERE l.grp = ? GROUP BY l.value ORDER BY 2 DESC, 1"
                .into(),
            Self::Place => "SELECT lo.name, count(*) FROM location lo JOIN f ON f.id = lo.media_id \
                 WHERE lo.name IS NOT NULL GROUP BY lo.name ORDER BY 2 DESC, 1"
                .into(),
            // Gli anni restano in ordine cronologico inverso.
            Self::Year => format!(
                "SELECT substr({LOCAL_TIME_SQL}, 1, 4), count(*) FROM media_dates d JOIN f ON f.id = d.media_id \
                 WHERE d.captured_at IS NOT NULL GROUP BY 1 ORDER BY 1 DESC"
            ),
            Self::Format => "SELECT m.ext, count(*) FROM media_item m JOIN f ON f.id = m.id \
                 GROUP BY m.ext ORDER BY 2 DESC, 1"
                .into(),
            Self::Tag => "SELECT tg.name, count(*) FROM media_tag mt JOIN f ON f.id = mt.media_id \
                 JOIN tag tg ON tg.id = mt.tag_id GROUP BY tg.id ORDER BY 2 DESC, 1"
                .into(),
        }
    }
}

/// Tutti i valori di una lista (senza limiti), con il filtro già compilato.
fn facet_rows(c: &Connection, w: &SqlFragment, key: &FacetKey) -> StoreResult<Vec<FacetValue>> {
    let mut args = w.args.clone();
    if let FacetKey::Label { group } = key {
        args.push(Value::Text(group.clone()));
    }
    let sql = format!(
        "WITH f AS (SELECT m.id {FROM} WHERE {}) {}",
        w.sql,
        key.select()
    );
    let mut st = c.prepare(&sql)?;
    let rows = st
        .query_map(rusqlite::params_from_iter(args.iter()), |r| {
            Ok(FacetValue {
                value: r.get::<_, Option<String>>(0)?.unwrap_or_default(),
                count: r.get(1)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

/// Valori di una lista con il filtro senza la sua selezione (si riusa il
/// filtro di base già compilato se la lista non ha selezione).
fn rows_for(
    c: &Connection,
    f: &LibraryFilter,
    base: &SqlFragment,
    key: &FacetKey,
) -> StoreResult<Vec<FacetValue>> {
    match key.without_own(f) {
        Some(g) => facet_rows(c, &compile_filter(c, &g)?, key),
        None => facet_rows(c, base, key),
    }
}

/// Riepilogo di una lista: i primi [`FACET_TOP`] valori più quelli selezionati
/// che non ne fanno parte (anche con 0 foto, per poterli deselezionare).
fn summarize(rows: Vec<FacetValue>, selected: &[String]) -> FacetList {
    let total = rows.len() as i64;
    let mut values: Vec<FacetValue> = rows.iter().take(FACET_TOP).cloned().collect();
    for s in selected {
        if !values.iter().any(|v| &v.value == s) {
            let count = rows
                .iter()
                .find(|v| &v.value == s)
                .map(|v| v.count)
                .unwrap_or(0);
            values.push(FacetValue {
                value: s.clone(),
                count,
            });
        }
    }
    FacetList { values, total }
}

/// Conteggi per i filtri del pannello laterale. Ogni lista applica tutti i
/// filtri tranne la propria selezione.
pub fn facets(c: &Connection, f: &LibraryFilter) -> StoreResult<Facets> {
    let base = compile_filter(c, f)?;
    let list = |key: FacetKey| -> StoreResult<FacetList> {
        Ok(summarize(rows_for(c, f, &base, &key)?, &key.selected(f)))
    };
    let column = |sql: &str| -> StoreResult<Vec<(String, i64)>> {
        let mut st = c.prepare(&format!(
            "WITH f AS (SELECT m.id {FROM} WHERE {}) {sql}",
            base.sql
        ))?;
        let rows = st
            .query_map(rusqlite::params_from_iter(base.args.iter()), |r| {
                Ok((r.get(0)?, r.get(1)?))
            })?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    };
    let mut out = Facets::default();
    // Gruppi di catalogazione presenti nei risultati o con una selezione.
    let mut groups: Vec<String> =
        column("SELECT l.grp, count(*) FROM label l JOIN f ON f.id = l.media_id GROUP BY l.grp")?
            .into_iter()
            .map(|(g, _)| g)
            .collect();
    for l in f.labels.iter().filter(|l| !l.values.is_empty()) {
        if !groups.contains(&l.group) {
            groups.push(l.group.clone());
        }
    }
    groups.sort();
    for group in groups {
        let values = list(FacetKey::Label {
            group: group.clone(),
        })?;
        out.labels.push((group, values));
    }
    out.places = list(FacetKey::Place)?;
    out.years = list(FacetKey::Year)?;
    out.formats = list(FacetKey::Format)?;
    out.tags = list(FacetKey::Tag)?;
    // Persone: non mostrate nella barra laterale, con il filtro completo.
    let people = column(
        "SELECT p.name, count(DISTINCT fc.media_id) FROM face fc JOIN f ON f.id = fc.media_id \
         JOIN person p ON p.id = fc.person_id WHERE p.name IS NOT NULL GROUP BY p.id ORDER BY 2 DESC, 1",
    )?;
    out.people = summarize(
        people
            .into_iter()
            .map(|(value, count)| FacetValue { value, count })
            .collect(),
        &[],
    );
    Ok(out)
}

/// Elenco completo dei valori di una lista, con ricerca (senza maiuscole e
/// accenti), ordinamento e paginazione. Applica il filtro senza la selezione
/// della lista stessa, come [`facets`].
pub fn facet_values(
    c: &Connection,
    f: &LibraryFilter,
    key: &FacetKey,
    search: Option<&str>,
    sort: FacetSort,
    offset: u32,
    limit: u32,
) -> StoreResult<FacetPage> {
    let base = compile_filter(c, f)?;
    let mut rows = rows_for(c, f, &base, key)?;
    if let Some(q) = search.map(str::trim).filter(|q| !q.is_empty()) {
        let q = pc_core::text::fold(q);
        rows.retain(|v| pc_core::text::fold(&v.value).contains(&q));
    }
    if sort == FacetSort::Name {
        let mut keyed: Vec<(String, FacetValue)> = rows
            .into_iter()
            .map(|v| (pc_core::text::fold(&v.value), v))
            .collect();
        keyed.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.value.cmp(&b.1.value)));
        rows = keyed.into_iter().map(|(_, v)| v).collect();
    }
    let total = rows.len() as i64;
    let values = rows
        .into_iter()
        .skip(offset as usize)
        .take(limit.clamp(1, 1000) as usize)
        .collect();
    Ok(FacetPage { values, total })
}

/// Tag concatenati con il separatore unità (U+001F), in ordine alfabetico.
fn split_tags(joined: Option<String>) -> Vec<String> {
    let mut tags: Vec<String> = joined
        .map(|s| s.split(char::from(0x1f)).map(str::to_string).collect())
        .unwrap_or_default();
    tags.sort_by_key(|t| t.to_lowercase());
    tags
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
