//! Dati per la sezione Mappa: aggregazione a griglia delle foto con posizione, cataloghi
//! nell'area, luoghi senza coordinate e posizioni manuali dei luoghi (`place_geo`).
//!
//! Coordinate effettive di una foto: GPS (`location.source='gps'` con lat/lon) se presente,
//! altrimenti `place_geo` del nome del luogo (approssimata). Le costanti `EFF_*` sono
//! riutilizzabili da altri filtri (es. `LibraryFilter.geo_bbox`).

use crate::query::{self, LibraryFilter};
use crate::{rules, StoreError, StoreResult};
use rusqlite::types::Value;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Latitudine massima della proiezione Web Mercator (`atan(sinh(π))`: y = 0 esatto).
pub const MAX_LAT: f64 = 85.051_128_779_806_59;
/// Celle per lato a zoom 0 (la griglia ha `2^zoom * 8` celle per lato).
pub const CELLS_PER_TILE: f64 = 8.0;
pub const MAX_ZOOM: u32 = 24;
/// Limite di x/y normalizzati: 180° e -85° cadono nell'ultima cella.
const EDGE: f64 = 0.999_999_999_9;

/// Foto con GPS valido (alias `l` = `location`).
pub const EFF_IS_GPS: &str = "(l.source = 'gps' AND l.lat IS NOT NULL AND l.lon IS NOT NULL)";
/// Solo `l` (location) e `pg` (place_geo, per le foto senza GPS): per sottoquery su
/// `l.media_id`, senza `media_item`.
pub const EFF_LOC_FROM: &str = "FROM location l
    LEFT JOIN place_geo pg ON pg.name = l.name
        AND NOT (l.source = 'gps' AND l.lat IS NOT NULL AND l.lon IS NOT NULL)";
/// `FROM` con le coordinate effettive: parte da `m` (media_item), aggiunge `d` e `t` (come
/// il filtro della libreria), `l` (location) e `pg` (place_geo, solo per le foto senza GPS).
pub const EFF_FROM: &str = "FROM media_item m
    LEFT JOIN media_dates d ON d.media_id = m.id
    LEFT JOIN media_tech t ON t.media_id = m.id
    JOIN location l ON l.media_id = m.id
    LEFT JOIN place_geo pg ON pg.name = l.name
        AND NOT (l.source = 'gps' AND l.lat IS NOT NULL AND l.lon IS NOT NULL)";
/// Latitudine / longitudine effettive (NULL = foto senza posizione).
pub const EFF_LAT: &str = "(CASE WHEN l.source = 'gps' AND l.lat IS NOT NULL AND l.lon IS NOT NULL THEN l.lat ELSE pg.lat END)";
pub const EFF_LON: &str = "(CASE WHEN l.source = 'gps' AND l.lat IS NOT NULL AND l.lon IS NOT NULL THEN l.lon ELSE pg.lon END)";
/// 1 se la posizione effettiva è approssimata (da `place_geo`), 0 se GPS.
pub const EFF_APPROX: &str =
    "(CASE WHEN l.source = 'gps' AND l.lat IS NOT NULL AND l.lon IS NOT NULL THEN 0 ELSE 1 END)";

/// Area geografica (inclusiva). `west > east` = attraversa l'antimeridiano.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct GeoBbox {
    pub south: f64,
    pub west: f64,
    pub north: f64,
    pub east: f64,
}

impl GeoBbox {
    pub const WORLD: GeoBbox = GeoBbox {
        south: -90.0,
        west: -180.0,
        north: 90.0,
        east: 180.0,
    };
}

/// Condizione SQL sul bbox, sulle coordinate effettive (con `EFF_FROM`) e, se `!approx`,
/// solo GPS. I parametri vanno aggiunti ad `args` nell'ordine dei segnaposto.
pub fn bbox_clause(bbox: &GeoBbox, approx: bool, args: &mut Vec<Value>) -> String {
    // Con le coordinate effettive non nulle (GPS oppure riga di place_geo).
    let mut s = if approx {
        format!("(pg.lat IS NOT NULL OR {EFF_IS_GPS})")
    } else {
        EFF_IS_GPS.to_string()
    };
    // Un lato che copre tutto il mondo non serve controllarlo.
    if bbox.south > -90.0 || bbox.north < 90.0 {
        s += &format!(" AND {EFF_LAT} BETWEEN ? AND ?");
        args.push(Value::Real(bbox.south));
        args.push(Value::Real(bbox.north));
    }
    if bbox.west > -180.0 || bbox.east < 180.0 {
        if bbox.west <= bbox.east {
            s += &format!(" AND {EFF_LON} BETWEEN ? AND ?");
        } else {
            s += &format!(" AND ({EFF_LON} >= ? OR {EFF_LON} <= ?)");
        }
        args.push(Value::Real(bbox.west));
        args.push(Value::Real(bbox.east));
    }
    s
}

/// y Mercator normalizzato (0 = nord, 1 = sud), latitudine limitata a ±`MAX_LAT`.
pub fn merc_y(lat: f64) -> f64 {
    let phi = lat.clamp(-MAX_LAT, MAX_LAT).to_radians();
    (1.0 - (phi.tan() + 1.0 / phi.cos()).ln() / std::f64::consts::PI) / 2.0
}

/// Celle per lato allo zoom dato.
fn grid(zoom: u32) -> f64 {
    (1u64 << zoom.min(MAX_ZOOM)) as f64 * CELLS_PER_TILE
}

/// Colonna della cella di una longitudine.
pub fn cell_x(lon: f64, zoom: u32) -> i64 {
    let n = grid(zoom);
    (((lon + 180.0) / 360.0).clamp(0.0, EDGE) * n) as i64
}

/// Riga della cella di una latitudine (0 = nord).
pub fn cell_y(lat: f64, zoom: u32) -> i64 {
    (merc_y(lat).clamp(0.0, EDGE) * grid(zoom)) as i64
}

/// Chiave intera con lo stesso ordine dei f64 (per la bisezione bit per bit).
fn fkey(v: f64) -> i128 {
    let b = v.to_bits() as i64;
    (if b < 0 { b ^ i64::MAX } else { b }) as i128
}

fn from_fkey(k: i128) -> f64 {
    let b = k as i64;
    f64::from_bits((if b < 0 { b ^ i64::MAX } else { b }) as u64)
}

/// Primo f64 in `(lo, hi]` con `pred` vero (`pred` monotono, falso in `lo`, vero in `hi`).
/// Prova prima una finestra stretta attorno a `guess`.
fn first_true(lo: f64, hi: f64, guess: f64, pred: impl Fn(f64) -> bool) -> f64 {
    let (mut a, mut b) = (fkey(lo), fkey(hi));
    let (gl, gh) = (guess - 1e-6, guess + 1e-6);
    if gl > lo && gh < hi && !pred(gl) && pred(gh) {
        (a, b) = (fkey(gl), fkey(gh));
    }
    while b - a > 1 {
        let m = a + (b - a) / 2;
        if pred(from_fkey(m)) {
            b = m;
        } else {
            a = m;
        }
    }
    from_fkey(b)
}

/// Più piccola longitudine della colonna `k` (`cell_x(v) >= k`), esatta in f64; `k` in 1..n.
fn lon_edge(k: i64, zoom: u32) -> f64 {
    let guess = k as f64 / grid(zoom) * 360.0 - 180.0;
    first_true(-180.0, 180.0, guess, |v| cell_x(v, zoom) >= k)
}

/// Più grande latitudine della riga `k` (`cell_y(v) >= k`), esatta in f64; `k` in 1..n.
fn lat_edge(k: i64, zoom: u32) -> f64 {
    let y = k as f64 / grid(zoom);
    let guess = (std::f64::consts::PI * (1.0 - 2.0 * y))
        .sinh()
        .atan()
        .to_degrees();
    first_true(-90.0, 90.0, guess, |v| cell_y(v, zoom) < k).next_down()
}

/// Area esatta di una cella: con `bbox_clause` (estremi inclusi) seleziona esattamente le
/// foto che `map_cells` mette nella cella `(cx, cy)`. Le righe e colonne di bordo arrivano
/// a ±90° / ±180°.
pub fn cell_bbox(cx: i64, cy: i64, zoom: u32) -> GeoBbox {
    let last = grid(zoom) as i64 - 1;
    GeoBbox {
        west: if cx <= 0 { -180.0 } else { lon_edge(cx, zoom) },
        east: if cx >= last {
            180.0
        } else {
            lon_edge(cx + 1, zoom).next_down()
        },
        north: if cy <= 0 { 90.0 } else { lat_edge(cy, zoom) },
        south: if cy >= last {
            -90.0
        } else {
            lat_edge(cy + 1, zoom).next_up()
        },
    }
}

/// Allarga il bbox alle celle intere che tocca: una cella ai bordi della vista ha comunque
/// il conteggio completo (uguale a quello del filtro `cell_bbox` in Libreria).
pub fn snap_bbox(b: &GeoBbox, zoom: u32) -> GeoBbox {
    let mut o = *b;
    if (b.south > -90.0 || b.north < 90.0) && b.south <= b.north {
        o.north = cell_bbox(0, cell_y(b.north, zoom), zoom).north;
        o.south = cell_bbox(0, cell_y(b.south, zoom), zoom).south;
    }
    if b.west > -180.0 || b.east < 180.0 {
        let (w, e) = (cell_x(b.west, zoom), cell_x(b.east, zoom));
        if b.west > b.east && w <= e {
            // Antimeridiano con le due estremità nella stessa cella: tutto il giro.
            (o.west, o.east) = (-180.0, 180.0);
        } else {
            o.west = cell_bbox(w, 0, zoom).west;
            o.east = cell_bbox(e, 0, zoom).east;
        }
    }
    o
}

/// Cella della griglia.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Cell {
    /// Colonna e riga nella griglia dello zoom richiesto (celle adiacenti: ±1).
    pub cx: i64,
    pub cy: i64,
    /// Area esatta della cella, da usare nel filtro `geo_bbox` della Libreria (stesso
    /// conteggio della cella).
    pub bbox: GeoBbox,
    pub count: i64,
    pub approx_count: i64,
    pub lat: f64,
    pub lon: f64,
    /// Luogo più frequente nella cella.
    pub place: Option<String>,
    /// Il luogo prevalente ha una posizione manuale (`place_geo.source = 'manual'`).
    pub manual: bool,
    /// Fino a 4 foto di anteprima.
    pub sample_ids: Vec<i64>,
    /// Chiavi delle miniature, allineate a `sample_ids` (`None` = senza miniatura).
    pub sample_thumbs: Vec<Option<String>>,
}

/// Luogo con posizione manuale e foto del filtro collocate lì (senza GPS).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct PlacePosition {
    pub name: String,
    pub count: i64,
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct CatalogCount {
    pub catalog_id: i64,
    pub name: String,
    pub count: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct PlaceCount {
    pub name: String,
    pub count: i64,
}

#[derive(Default)]
struct Acc {
    count: i64,
    approx: i64,
    lat: f64,
    lon: f64,
    names: HashMap<String, i64>,
    /// Anteprime: il minimo id per ciascuna classe `id % 4`.
    ids: [Option<i64>; 4],
}

/// Gruppi di foto con posizione per cella (lato = 1/(2^zoom·8) del mondo in Mercator).
/// Il filtro include l'esclusione dei cataloghi di sistema (come la libreria). Il bbox
/// viene allargato alle celle intere (`snap_bbox`).
pub fn map_cells(
    c: &Connection,
    filter: &LibraryFilter,
    bbox: &GeoBbox,
    zoom: u32,
    approx: bool,
) -> StoreResult<Vec<Cell>> {
    let zoom = zoom.min(MAX_ZOOM);
    let frag = query::compile_filter(c, filter)?;
    let mut args = frag.args;
    let bb = bbox_clause(&snap_bbox(bbox, zoom), approx, &mut args);
    let from = eff_from_for(&frag.sql);
    // Righe per foto e raggruppamento in Rust: più veloce di GROUP BY in SQLite (ordinamento
    // di 100k righe) e con le stesse `cell_x`/`cell_y` di `cell_bbox`.
    let sql = format!(
        "SELECT m.id, {EFF_LAT}, {EFF_LON}, {EFF_APPROX}, l.name {from} WHERE {} AND {bb}",
        frag.sql
    );
    let mut q = c.prepare(&sql)?;
    let mut cells: HashMap<(i64, i64), Acc> = HashMap::new();
    let mut rows = q.query(rusqlite::params_from_iter(args))?;
    while let Some(r) = rows.next()? {
        let id: i64 = r.get(0)?;
        let (lat, lon): (f64, f64) = (r.get(1)?, r.get(2)?);
        let a = cells
            .entry((cell_x(lon, zoom), cell_y(lat, zoom)))
            .or_default();
        a.count += 1;
        a.approx += r.get::<_, i64>(3)?;
        a.lat += lat;
        a.lon += lon;
        if let Some(name) = r
            .get_ref(4)?
            .as_str_or_null()
            .ok()
            .flatten()
            .filter(|s| !s.is_empty())
        {
            match a.names.get_mut(name) {
                Some(n) => *n += 1,
                None => {
                    a.names.insert(name.to_string(), 1);
                }
            }
        }
        let slot = &mut a.ids[id.rem_euclid(4) as usize];
        if slot.is_none_or(|v| id < v) {
            *slot = Some(id);
        }
    }
    let manual = manual_names(c)?;
    let mut out: Vec<Cell> = cells
        .into_iter()
        .map(|((cx, cy), a)| {
            // Luogo prevalente; a parità il primo in ordine alfabetico.
            let place = a
                .names
                .into_iter()
                .max_by(|x, y| x.1.cmp(&y.1).then_with(|| y.0.cmp(&x.0)))
                .map(|(n, _)| n);
            let mut ids: Vec<i64> = a.ids.into_iter().flatten().collect();
            ids.sort_unstable();
            Cell {
                cx,
                cy,
                bbox: cell_bbox(cx, cy, zoom),
                count: a.count,
                approx_count: a.approx,
                lat: a.lat / a.count as f64,
                lon: a.lon / a.count as f64,
                manual: place.as_ref().is_some_and(|p| manual.contains(p)),
                place,
                sample_thumbs: Vec::new(),
                sample_ids: ids,
            }
        })
        .collect();
    out.sort_by_key(|c| (c.cy, c.cx));
    // Miniature delle anteprime: una sola query per tutte le celle.
    let all: Vec<i64> = out
        .iter()
        .flat_map(|c| c.sample_ids.iter().copied())
        .collect();
    if !all.is_empty() {
        let mut q = c.prepare(
            "SELECT id, thumb FROM media_item WHERE id IN (SELECT value FROM json_each(?1))",
        )?;
        let thumbs: HashMap<i64, Option<String>> = q
            .query_map([serde_json::to_string(&all)?], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })?
            .collect::<Result<_, _>>()?;
        for cell in &mut out {
            cell.sample_thumbs = cell
                .sample_ids
                .iter()
                .map(|id| thumbs.get(id).cloned().flatten())
                .collect();
        }
    }
    Ok(out)
}

/// Nomi con posizione manuale.
fn manual_names(c: &Connection) -> StoreResult<HashSet<String>> {
    let mut q = c.prepare("SELECT name FROM place_geo WHERE source = 'manual'")?;
    let rows = q.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
    Ok(rows)
}

/// Luoghi con posizione manuale (per "Modifica" / "Rimuovi posizione"), con il numero di
/// foto del filtro senza GPS collocate lì. Anche quelli senza foto nel filtro (conteggio 0),
/// così una posizione rimasta orfana resta rimovibile.
pub fn map_places_manual(
    c: &Connection,
    filter: &LibraryFilter,
) -> StoreResult<Vec<PlacePosition>> {
    let frag = query::compile_filter(c, filter)?;
    let sql = format!(
        "SELECT l.name, count(*) {}JOIN location l ON l.media_id = m.id
         JOIN place_geo p ON p.name = l.name AND p.source = 'manual'
         WHERE {} AND NOT {EFF_IS_GPS} GROUP BY l.name",
        media_from_for(&frag.sql),
        frag.sql
    );
    let mut q = c.prepare(&sql)?;
    let counts: HashMap<String, i64> = q
        .query_map(rusqlite::params_from_iter(frag.args), |r| {
            Ok((r.get(0)?, r.get(1)?))
        })?
        .collect::<Result<_, _>>()?;
    let mut q = c.prepare("SELECT name, lat, lon FROM place_geo WHERE source = 'manual'")?;
    let mut out = q
        .query_map([], |r| {
            Ok(PlacePosition {
                name: r.get(0)?,
                count: 0,
                lat: r.get(1)?,
                lon: r.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for p in &mut out {
        p.count = counts.get(&p.name).copied().unwrap_or(0);
    }
    out.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));
    Ok(out)
}

/// Ids delle foto del filtro nell'area.
fn ids_in_area(
    c: &Connection,
    filter: &LibraryFilter,
    bbox: &GeoBbox,
    approx: bool,
) -> StoreResult<HashSet<i64>> {
    let frag = query::compile_filter(c, filter)?;
    let mut args = frag.args;
    let bb = bbox_clause(bbox, approx, &mut args);
    let sql = format!(
        "SELECT m.id {} WHERE {} AND {bb}",
        eff_from_for(&frag.sql),
        frag.sql
    );
    let mut q = c.prepare(&sql)?;
    let ids = q
        .query_map(rusqlite::params_from_iter(args), |r| r.get(0))?
        .collect::<Result<HashSet<i64>, _>>()?;
    Ok(ids)
}

/// Foto per catalogo (manuale o smart, non di sistema) nell'area; solo conteggi > 0.
pub fn map_catalogs(
    c: &Connection,
    filter: &LibraryFilter,
    bbox: &GeoBbox,
    approx: bool,
) -> StoreResult<Vec<CatalogCount>> {
    let ids = ids_in_area(c, filter, bbox, approx)?;
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let cats: Vec<(i64, String)> = {
        let mut q = c.prepare("SELECT id, name FROM catalog WHERE system IS NULL ORDER BY name")?;
        let rows = q
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        rows
    };
    let mut out = Vec::new();
    for (id, name) in cats {
        // Contenuto del catalogo intersecato in Rust con l'area: niente lista di id passata a
        // SQLite per ogni catalogo. Le regole smart possono usare `d.`/`t.`.
        let frag = rules::catalog_content(c, id, &[])?;
        let sql = format!(
            "SELECT m.id {} WHERE {}",
            media_from_for(&frag.sql),
            frag.sql
        );
        let mut q = c.prepare(&sql)?;
        let mut rows = q.query(rusqlite::params_from_iter(frag.args))?;
        let mut count = 0i64;
        while let Some(r) = rows.next()? {
            if ids.contains(&r.get::<_, i64>(0)?) {
                count += 1;
            }
        }
        if count > 0 {
            out.push(CatalogCount {
                catalog_id: id,
                name,
                count,
            });
        }
    }
    out.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));
    Ok(out)
}

/// Luoghi (con conteggio) delle foto del filtro senza GPS e senza `place_geo`.
pub fn map_places_unplaced(c: &Connection, filter: &LibraryFilter) -> StoreResult<Vec<PlaceCount>> {
    let frag = query::compile_filter(c, filter)?;
    let sql = format!(
        "SELECT l.name, count(*) {EFF_FROM}
         WHERE {} AND l.name IS NOT NULL AND l.name <> '' AND pg.name IS NULL
           AND NOT {EFF_IS_GPS}
         GROUP BY l.name ORDER BY count(*) DESC, l.name",
        frag.sql
    );
    let mut q = c.prepare(&sql)?;
    let rows = q
        .query_map(rusqlite::params_from_iter(frag.args), |r| {
            Ok(PlaceCount {
                name: r.get(0)?,
                count: r.get(1)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

/// Posizione manuale di un luogo (sovrascrive quella del gazetteer).
pub fn map_place_set(c: &Connection, name: &str, lat: f64, lon: f64) -> StoreResult<()> {
    let name = name.trim();
    if name.is_empty()
        || !lat.is_finite()
        || !lon.is_finite()
        || !(-90.0..=90.0).contains(&lat)
        || !(-180.0..=180.0).contains(&lon)
    {
        return Err(StoreError::Invalid("luogo o coordinate non valide".into()));
    }
    c.execute(
        "INSERT INTO place_geo(name, lat, lon, country, source, updated_at)
         VALUES (?1, ?2, ?3, NULL, 'manual', strftime('%s','now'))
         ON CONFLICT(name) DO UPDATE SET lat = excluded.lat, lon = excluded.lon,
            country = NULL, source = 'manual', updated_at = excluded.updated_at",
        rusqlite::params![name, lat, lon],
    )?;
    Ok(())
}

/// Rimuove la posizione manuale; il job `place_geo` può poi rimettere quella del
/// gazetteer. `true` se c'era una posizione manuale.
pub fn map_place_clear(c: &Connection, name: &str) -> StoreResult<bool> {
    let n = c.execute(
        "DELETE FROM place_geo WHERE name = ?1 AND source = 'manual'",
        [name.trim()],
    )?;
    Ok(n > 0)
}

/// Nomi (directory/manual) senza riga in `place_geo`: da geocodificare.
pub fn names_to_geocode(c: &Connection) -> StoreResult<Vec<String>> {
    let mut q = c.prepare(
        "SELECT DISTINCT l.name FROM location l
         WHERE l.source IN ('directory', 'manual') AND l.name IS NOT NULL AND l.name <> ''
           AND NOT EXISTS (SELECT 1 FROM place_geo pg WHERE pg.name = l.name)
         ORDER BY l.name",
    )?;
    let rows = q.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
    Ok(rows)
}

/// Inserisce la posizione del gazetteer senza toccare una riga esistente (mai la manuale).
pub fn place_geo_insert_auto(
    c: &Connection,
    name: &str,
    lat: f64,
    lon: f64,
    country: Option<&str>,
) -> StoreResult<bool> {
    let n = c.execute(
        "INSERT OR IGNORE INTO place_geo(name, lat, lon, country, source, updated_at)
         VALUES (?1, ?2, ?3, ?4, 'gazetteer', strftime('%s','now'))",
        rusqlite::params![name, lat, lon, country],
    )?;
    Ok(n > 0)
}

/// `FROM media_item m` con i join `d`/`t` solo se il filtro compilato li usa.
pub fn media_from_for(filter_sql: &str) -> String {
    let mut s = String::from("FROM media_item m\n");
    if filter_sql.contains("d.") {
        s += "LEFT JOIN media_dates d ON d.media_id = m.id\n";
    }
    if filter_sql.contains("t.") {
        s += "LEFT JOIN media_tech t ON t.media_id = m.id\n";
    }
    s
}

/// `EFF_FROM` senza i join `d`/`t` se il filtro compilato non li usa (più veloce).
pub fn eff_from_for(filter_sql: &str) -> String {
    media_from_for(filter_sql)
        + "JOIN location l ON l.media_id = m.id
    LEFT JOIN place_geo pg ON pg.name = l.name
        AND NOT (l.source = 'gps' AND l.lat IS NOT NULL AND l.lon IS NOT NULL)"
}
