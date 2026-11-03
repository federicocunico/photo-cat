//! Volti e correzioni persistenti. Nessuna operazione modifica i file originali.
use crate::{analysis, now_ms, repo, StoreError, StoreResult};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, Default)]
pub struct DetectedFace {
    pub bbox: [f32; 4],
    pub quality: f32,
    pub sharpness: Option<f32>,
    pub frontality: Option<f32>,
    pub size_px: Option<u32>,
    pub quality_score: Option<f32>,
    pub embedding: Vec<f32>,
}

/// Motivo per cui un volto è scartato per qualità dal raggruppamento e dai suggerimenti.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum DiscardReason {
    Blurred,
    TooSmall,
    NonFrontal,
    LowConfidence,
}

/// Frontalità minima per non scartare un volto come "poco frontale".
pub const MIN_FRONTALITY: f32 = 0.22;

/// Qualità combinata: `min(1, conf^0.5 · sharp^0.8 · front^0.5 · size_factor)`,
/// dove `size_factor` satura a 1 sopra 96 px e frontalità/nitidezza assenti
/// valgono 1 (qualità sconosciuta sui volti pre-migrazione).
pub fn combined_quality(
    conf: f32,
    sharpness: Option<f32>,
    frontality: Option<f32>,
    size_px: Option<u32>,
) -> f32 {
    let c = conf.clamp(0.0, 1.0);
    let Some(s) = sharpness else {
        return c;
    };
    let s = s.clamp(0.0, 1.0).powf(0.8);
    let f = frontality.unwrap_or(1.0).clamp(0.0, 1.0).powf(0.5);
    let sz = size_px
        .map(|px| (px as f32 / 96.0).clamp(0.0, 1.0))
        .unwrap_or(1.0);
    (c.powf(0.5) * s * f * sz).clamp(0.0, 1.0)
}

/// Motivo dello scarto di un volto rispetto alle soglie correnti (`None` se il
/// volto è utilizzabile, assegnato a mano o recuperato dall'utente).
pub fn discard_reason(
    conf: f32,
    sharpness: Option<f32>,
    frontality: Option<f32>,
    size_px: Option<u32>,
    manual: bool,
    quality_override: bool,
    m: &FaceMatch,
) -> Option<DiscardReason> {
    if manual || quality_override {
        return None;
    }
    if conf < m.min_quality {
        return Some(DiscardReason::LowConfidence);
    }
    if size_px.is_some_and(|px| px < m.min_size_px) {
        return Some(DiscardReason::TooSmall);
    }
    if sharpness.is_some_and(|s| s < m.min_sharpness) {
        return Some(DiscardReason::Blurred);
    }
    if frontality.is_some_and(|f| f < MIN_FRONTALITY) {
        return Some(DiscardReason::NonFrontal);
    }
    None
}

#[derive(Debug, Clone, Default)]
pub struct FaceRecord {
    pub id: i64,
    pub media_id: i64,
    pub bbox: [f32; 4],
    pub quality: f32,
    pub sharpness: Option<f32>,
    pub frontality: Option<f32>,
    pub size_px: Option<u32>,
    pub quality_score: f32,
    pub quality_override: bool,
    pub embedding: Option<Vec<f32>>,
    pub person_id: Option<i64>,
    pub manual: bool,
    pub rejected: Vec<i64>,
    pub model: String,
}

impl FaceRecord {
    pub fn discard_reason(&self, m: &FaceMatch) -> Option<DiscardReason> {
        discard_reason(
            self.quality,
            self.sharpness,
            self.frontality,
            self.size_px,
            self.manual,
            self.quality_override,
            m,
        )
    }

    pub fn is_discarded(&self, m: &FaceMatch) -> bool {
        self.discard_reason(m).is_some()
    }
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct FaceView {
    pub id: i64,
    pub media_id: i64,
    pub bbox: [f32; 4],
    pub quality: f32,
    pub quality_score: f32,
    pub sharpness: Option<f32>,
    pub frontality: Option<f32>,
    pub size_px: Option<u32>,
    pub discard_reason: Option<DiscardReason>,
    pub year: Option<i32>,
    pub thumb: Option<String>,
    pub person_id: Option<i64>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct PersonView {
    pub id: i64,
    pub name: Option<String>,
    /// Identità dell'app collegata (UUID in `app.db`).
    pub identity_uid: Option<String>,
    pub photos: i64,
    pub faces: i64,
    pub cover: Option<FaceView>,
}

/// Sezione del modulo Persone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum PeopleSection {
    #[default]
    Review,
    People,
    Ignored,
}

/// Ordinamento dei gruppi nel modulo Persone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum GroupSort {
    #[default]
    PhotosDesc,
    NameAsc,
    FacesDesc,
}

/// Ordinamento dei volti in `group_faces`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum FaceSort {
    #[default]
    QualityDesc,
    CapturedDesc,
}

/// Ambito dei volti richiesti con `group_faces`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FaceScope {
    Group { group: i64 },
    Singles,
    Discarded { group: Option<i64> },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct FaceCursor {
    pub quality_score: f32,
    pub id: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct FacePage {
    pub items: Vec<FaceView>,
    pub next: Option<FaceCursor>,
    pub total: i64,
    pub good_count: i64,
    pub discarded_count: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct PeopleSections {
    pub review: i64,
    pub people: i64,
    pub ignored: i64,
    pub singles: i64,
    pub discarded: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct NameHint {
    pub uid: String,
    pub name: String,
    pub short_name: String,
    pub group: Option<String>,
    pub score: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct MergeHint {
    pub other_id: i64,
    pub other_name: Option<String>,
    pub other_faces: i64,
    pub other_photos: i64,
    pub other_cover: Option<FaceView>,
    pub score: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct GroupRow {
    pub id: i64,
    pub name: Option<String>,
    pub identity_uid: Option<String>,
    pub section: PeopleSection,
    pub faces: i64,
    pub good_faces: i64,
    pub discarded_faces: i64,
    pub photos: i64,
    pub cover: Option<FaceView>,
    pub ignored_at: Option<i64>,
    pub new_faces: i64,
    pub name_hint: Option<NameHint>,
    pub merge_hint: Option<MergeHint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct GroupPage {
    pub items: Vec<GroupRow>,
    pub next: Option<i64>,
    pub total: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LocateFaceRequest {
    Face {
        face_id: i64,
    },
    Media {
        media_id: i64,
        person_id: Option<i64>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct LocatedFace {
    pub face_id: i64,
    pub media_id: i64,
    pub media_name: String,
    pub section: PeopleSection,
    pub group_id: Option<i64>,
    pub singles: bool,
    pub discarded: bool,
    pub index: i64,
}

pub fn vector(bytes: &[u8]) -> StoreResult<Vec<f32>> {
    if bytes.len() != 128 * 4 {
        return Err(StoreError::Invalid("embedding volto non valido".into()));
    }
    let v: Vec<_> = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| f32::from_le_bytes(*b))
        .collect();
    if v.iter().any(|x| !x.is_finite()) {
        return Err(StoreError::Invalid("embedding volto non finito".into()));
    }
    Ok(v)
}
fn bytes(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
fn normalize(v: &mut [f32]) {
    let n = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if n > 0.0 {
        for x in v {
            *x /= n;
        }
    }
}
pub fn iou(a: [f32; 4], b: [f32; 4]) -> f32 {
    let area = (a[0] + a[2]).min(b[0] + b[2]) - (a[0].max(b[0]));
    let height = (a[1] + a[3]).min(b[1] + b[3]) - (a[1].max(b[1]));
    let intersection = area.max(0.0) * height.max(0.0);
    intersection / (a[2] * a[3] + b[2] * b[3] - intersection).max(1e-8)
}
fn next_id(c: &Connection, table: &str) -> StoreResult<i64> {
    let key = format!("identity.next.{table}");
    let previous: Option<String> = c
        .query_row("SELECT value FROM settings_kv WHERE key=?1", [&key], |r| {
            r.get(0)
        })
        .optional()?;
    let max: i64 = c.query_row(
        &format!("SELECT COALESCE(MAX(id),0) FROM {table}"),
        [],
        |r| r.get(0),
    )?;
    let id = max
        .max(previous.and_then(|s| s.parse().ok()).unwrap_or(0))
        .checked_add(1)
        .ok_or_else(|| StoreError::Invalid("identificativi esauriti".into()))?;
    c.execute("INSERT INTO settings_kv(key,value) VALUES (?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",params![key,id.to_string()])?;
    Ok(id)
}
pub fn create_person(c: &Connection, name: Option<&str>) -> StoreResult<i64> {
    let id = next_id(c, "person")?;
    c.execute(
        "INSERT INTO person(id,name,created_at) VALUES (?1,?2,?3)",
        params![id, name, now_ms()],
    )?;
    Ok(id)
}
pub fn records(c: &Connection, media: Option<i64>) -> StoreResult<Vec<FaceRecord>> {
    let mut q = c.prepare(
        "SELECT id,media_id,bbox,quality,sharpness,frontality,size_px,COALESCE(quality_score,quality),quality_override,embedding,person_id,assigned_by,rejected,COALESCE(model_ver,'')
         FROM face WHERE (?1 IS NULL OR media_id=?1) ORDER BY id",
    )?;
    let rows = q
        .query_map([media], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, f32>(3)?,
                r.get::<_, Option<f32>>(4)?,
                r.get::<_, Option<f32>>(5)?,
                r.get::<_, Option<u32>>(6)?,
                r.get::<_, f32>(7)?,
                r.get::<_, i64>(8)?,
                r.get::<_, Option<Vec<u8>>>(9)?,
                r.get::<_, Option<i64>>(10)?,
                r.get::<_, Option<String>>(11)?,
                r.get::<_, String>(12)?,
                r.get::<_, String>(13)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    rows.into_iter()
        .map(
            |(
                id,
                media_id,
                bbox,
                quality,
                sharpness,
                frontality,
                size_px,
                quality_score,
                quality_override,
                embed,
                person_id,
                assigned,
                reject,
                model,
            )| {
                Ok(FaceRecord {
                    id,
                    media_id,
                    bbox: serde_json::from_str(&bbox)?,
                    quality,
                    sharpness,
                    frontality,
                    size_px,
                    quality_score,
                    quality_override: quality_override != 0,
                    embedding: embed.map(|b| vector(&b)).transpose()?,
                    person_id,
                    manual: assigned.as_deref() == Some("manual"),
                    rejected: serde_json::from_str(&reject)?,
                    model,
                })
            },
        )
        .collect()
}
/// Soglie di qualità usate dalle query di lettura quando non ne viene passata una.
pub fn default_face_match() -> FaceMatch {
    FaceMatch {
        similarity: pc_core::AiThresholds::DEFAULT_FACE_SIMILARITY,
        min_quality: pc_core::AiThresholds::DEFAULT_FACE_DETECTION,
        min_sharpness: pc_core::AiThresholds::DEFAULT_FACE_MIN_SHARPNESS,
        min_size_px: 40,
        floor: None,
    }
}

/// Predicato SQL (alias `f`) per i volti scartati rispetto alle soglie `m`.
fn discarded_sql(m: &FaceMatch) -> String {
    format!(
        "(COALESCE(f.assigned_by,'') != 'manual' AND f.quality_override = 0 AND (\
            f.quality < {q} OR \
            (f.size_px IS NOT NULL AND f.size_px < {sz}) OR \
            (f.sharpness IS NOT NULL AND f.sharpness < {sh}) OR \
            (f.frontality IS NOT NULL AND f.frontality < {fr})\
        ))",
        q = m.min_quality,
        sz = m.min_size_px,
        sh = m.min_sharpness,
        fr = MIN_FRONTALITY,
    )
}

const FACE_VIEW_COLS: &str = "f.id, f.media_id, f.bbox, f.quality, \
    COALESCE(f.quality_score, f.quality), f.sharpness, f.frontality, f.size_px, \
    f.quality_override, f.assigned_by, m.thumb, f.person_id, \
    CAST(substr(COALESCE(\
        datetime(d.instant_utc / 1000 + COALESCE(d.local_offset_min, 0) * 60, 'unixepoch'), \
        d.captured_at), 1, 4) AS INTEGER)";

#[allow(clippy::too_many_arguments)]
fn map_face_view(
    id: i64,
    media_id: i64,
    bbox: String,
    quality: f32,
    quality_score: f32,
    sharpness: Option<f32>,
    frontality: Option<f32>,
    size_px: Option<u32>,
    quality_override: i64,
    assigned_by: Option<String>,
    thumb: Option<String>,
    person_id: Option<i64>,
    year: Option<i32>,
    m: &FaceMatch,
) -> StoreResult<FaceView> {
    let manual = assigned_by.as_deref() == Some("manual");
    Ok(FaceView {
        id,
        media_id,
        bbox: serde_json::from_str(&bbox)?,
        quality,
        quality_score,
        sharpness,
        frontality,
        size_px,
        discard_reason: discard_reason(
            quality,
            sharpness,
            frontality,
            size_px,
            manual,
            quality_override != 0,
            m,
        ),
        year,
        thumb,
        person_id,
    })
}

pub fn views(c: &Connection, person: Option<i64>) -> StoreResult<Vec<FaceView>> {
    views_with(c, person, &default_face_match())
}

pub fn views_with(
    c: &Connection,
    person: Option<i64>,
    m: &FaceMatch,
) -> StoreResult<Vec<FaceView>> {
    let sql = format!(
        "SELECT {FACE_VIEW_COLS} FROM face f \
         JOIN media_item m ON m.id = f.media_id \
         LEFT JOIN media_dates d ON d.media_id = f.media_id \
         WHERE (?1 IS NULL OR f.person_id = ?1) \
         ORDER BY COALESCE(f.quality_score, f.quality) DESC, f.id"
    );
    let mut q = c.prepare(&sql)?;
    let rows = q
        .query_map([person], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get::<_, String>(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
                r.get(7)?,
                r.get(8)?,
                r.get(9)?,
                r.get(10)?,
                r.get(11)?,
                r.get(12)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    rows.into_iter()
        .map(
            |(
                id,
                media_id,
                bbox,
                quality,
                quality_score,
                sharpness,
                frontality,
                size_px,
                quality_override,
                assigned_by,
                thumb,
                person_id,
                year,
            )| {
                map_face_view(
                    id,
                    media_id,
                    bbox,
                    quality,
                    quality_score,
                    sharpness,
                    frontality,
                    size_px,
                    quality_override,
                    assigned_by,
                    thumb,
                    person_id,
                    year,
                    m,
                )
            },
        )
        .collect()
}

/// Volti di una foto per l'Inspector (persona, ignorato, scartato).
pub fn media_faces(
    c: &Connection,
    media_id: i64,
) -> StoreResult<Vec<crate::detail::MediaFaceView>> {
    media_faces_with(c, media_id, &default_face_match())
}

pub fn media_faces_with(
    c: &Connection,
    media_id: i64,
    m: &FaceMatch,
) -> StoreResult<Vec<crate::detail::MediaFaceView>> {
    let mut q = c.prepare(
        "SELECT f.id, f.bbox, f.quality, f.sharpness, f.frontality, f.size_px, f.quality_override, \
                f.assigned_by, f.person_id, p.name, p.identity_uid, p.ignored_at \
         FROM face f LEFT JOIN person p ON p.id = f.person_id \
         WHERE f.media_id = ?1 ORDER BY f.id",
    )?;
    let rows = q
        .query_map([media_id], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, f32>(2)?,
                r.get::<_, Option<f32>>(3)?,
                r.get::<_, Option<f32>>(4)?,
                r.get::<_, Option<u32>>(5)?,
                r.get::<_, i64>(6)?,
                r.get::<_, Option<String>>(7)?,
                r.get::<_, Option<i64>>(8)?,
                r.get::<_, Option<String>>(9)?,
                r.get::<_, Option<String>>(10)?,
                r.get::<_, Option<i64>>(11)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    rows.into_iter()
        .map(
            |(
                id,
                bbox,
                quality,
                sharpness,
                frontality,
                size_px,
                quality_override,
                assigned_by,
                person_id,
                person_name,
                identity_uid,
                ignored_at,
            )| {
                let manual = assigned_by.as_deref() == Some("manual");
                Ok(crate::detail::MediaFaceView {
                    id,
                    bbox: serde_json::from_str(&bbox)?,
                    person_id,
                    person_name,
                    identity_uid,
                    ignored: ignored_at.is_some(),
                    discarded: discard_reason(
                        quality,
                        sharpness,
                        frontality,
                        size_px,
                        manual,
                        quality_override != 0,
                        m,
                    )
                    .is_some(),
                })
            },
        )
        .collect()
}

pub fn list(c: &Connection) -> StoreResult<Vec<PersonView>> {
    // Le persone con nome restano anche senza volti (es. dopo la rimozione di una sorgente).
    let mut q=c.prepare("SELECT p.id,p.name,count(DISTINCT f.media_id),count(f.id),p.identity_uid FROM person p LEFT JOIN face f ON f.person_id=p.id WHERE p.ignored_at IS NULL GROUP BY p.id HAVING count(f.id)>0 OR p.name IS NOT NULL ORDER BY p.name IS NULL,p.name,p.id")?;
    let rows = q
        .query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    rows.into_iter()
        .map(|(id, name, photos, faces, identity_uid)| {
            Ok(PersonView {
                id,
                name,
                identity_uid,
                photos,
                faces,
                cover: views(c, Some(id))?.into_iter().next(),
            })
        })
        .collect()
}

fn clamp_limit(limit: i64) -> i64 {
    limit.clamp(1, 200)
}

fn section_where(section: PeopleSection) -> &'static str {
    match section {
        PeopleSection::Review => "p.ignored_at IS NULL AND p.identity_uid IS NULL",
        PeopleSection::People => "p.ignored_at IS NULL AND p.identity_uid IS NOT NULL",
        PeopleSection::Ignored => "p.ignored_at IS NOT NULL",
    }
}

/// Conteggi delle sezioni del modulo Persone.
pub fn people_sections(c: &Connection, m: &FaceMatch) -> StoreResult<PeopleSections> {
    let discarded = discarded_sql(m);
    let review: i64 = c.query_row(
        "SELECT count(*) FROM person p WHERE p.ignored_at IS NULL AND p.identity_uid IS NULL \
         AND EXISTS(SELECT 1 FROM face f WHERE f.person_id = p.id)",
        [],
        |r| r.get(0),
    )?;
    let people: i64 = c.query_row(
        "SELECT count(*) FROM person p WHERE p.ignored_at IS NULL AND p.identity_uid IS NOT NULL",
        [],
        |r| r.get(0),
    )?;
    let ignored: i64 = c.query_row(
        "SELECT count(*) FROM person p WHERE p.ignored_at IS NOT NULL",
        [],
        |r| r.get(0),
    )?;
    let singles: i64 = c.query_row(
        &format!("SELECT count(*) FROM face f WHERE f.person_id IS NULL AND NOT {discarded}"),
        [],
        |r| r.get(0),
    )?;
    let discarded_n: i64 = c.query_row(
        &format!("SELECT count(*) FROM face f WHERE {discarded}"),
        [],
        |r| r.get(0),
    )?;
    Ok(PeopleSections {
        review,
        people,
        ignored,
        singles,
        discarded: discarded_n,
    })
}

fn cover_face(
    c: &Connection,
    person: i64,
    cover_id: Option<i64>,
    m: &FaceMatch,
) -> StoreResult<Option<FaceView>> {
    if let Some(id) = cover_id {
        let sql = format!(
            "SELECT {FACE_VIEW_COLS} FROM face f \
             JOIN media_item m ON m.id = f.media_id \
             LEFT JOIN media_dates d ON d.media_id = f.media_id \
             WHERE f.id = ?1"
        );
        let mut q = c.prepare(&sql)?;
        let row = q
            .query_map([id], |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get::<_, String>(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                    r.get(7)?,
                    r.get(8)?,
                    r.get(9)?,
                    r.get(10)?,
                    r.get(11)?,
                    r.get(12)?,
                ))
            })?
            .next()
            .transpose()?;
        if let Some(row) = row {
            return Ok(Some(map_face_view(
                row.0, row.1, row.2, row.3, row.4, row.5, row.6, row.7, row.8, row.9, row.10,
                row.11, row.12, m,
            )?));
        }
    }
    Ok(views_with(c, Some(person), m)?.into_iter().next())
}

fn merge_hint_for(c: &Connection, id: i64, m: &FaceMatch) -> StoreResult<Option<MergeHint>> {
    let row: Option<(i64, i64, f32)> = c
        .query_row(
            "SELECT a, b, score FROM person_suggestion WHERE a=?1 OR b=?1 ORDER BY score DESC LIMIT 1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let Some((a, b, score)) = row else {
        return Ok(None);
    };
    let other = if a == id { b } else { a };
    let (name, faces, photos, cover_id): (Option<String>, i64, i64, Option<i64>) = c.query_row(
        "SELECT p.name, \
            (SELECT count(*) FROM face WHERE person_id = p.id), \
            (SELECT count(DISTINCT media_id) FROM face WHERE person_id = p.id), \
            p.cover_face_id \
         FROM person p WHERE p.id = ?1",
        [other],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )?;
    Ok(Some(MergeHint {
        other_id: other,
        other_name: name,
        other_faces: faces,
        other_photos: photos,
        other_cover: cover_face(c, other, cover_id, m)?,
        score,
    }))
}

/// Elenco paginato dei gruppi di una sezione del modulo Persone.
pub fn list_groups(
    c: &Connection,
    section: PeopleSection,
    query: Option<&str>,
    sort: GroupSort,
    after: Option<i64>,
    limit: i64,
    m: &FaceMatch,
) -> StoreResult<GroupPage> {
    let limit = clamp_limit(limit);
    let discarded = discarded_sql(m);
    let section_sql = section_where(section);
    let qnorm = query
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_lowercase());
    let search_sql = if qnorm.is_some() {
        "AND (lower(COALESCE(p.name,'')) LIKE '%' || ?1 || '%' \
              OR lower(COALESCE(p.identity_uid,'')) LIKE '%' || ?1 || '%')"
    } else {
        ""
    };
    let having = match section {
        PeopleSection::People => {
            "HAVING faces > 0 OR p.name IS NOT NULL OR p.identity_uid IS NOT NULL"
        }
        PeopleSection::Ignored => "HAVING faces > 0 OR p.ignored_at IS NOT NULL",
        PeopleSection::Review => "HAVING faces > 0",
    };
    let sql = format!(
        "SELECT p.id, p.name, p.identity_uid, p.ignored_at, p.ignored_faces, p.cover_face_id, \
                count(f.id) AS faces, \
                count(DISTINCT f.media_id) AS photos, \
                COALESCE(sum(CASE WHEN {discarded} THEN 1 ELSE 0 END), 0) AS discarded_faces \
         FROM person p \
         LEFT JOIN face f ON f.person_id = p.id \
         WHERE {section_sql} {search_sql} \
         GROUP BY p.id {having}"
    );
    let mut all = Vec::new();
    if let Some(ref q) = qnorm {
        let mut st = c.prepare(&sql)?;
        let mapped = st.query_map([q.as_str()], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, Option<i64>>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, Option<i64>>(5)?,
                r.get::<_, i64>(6)?,
                r.get::<_, i64>(7)?,
                r.get::<_, i64>(8)?,
            ))
        })?;
        for row in mapped {
            all.push(row?);
        }
    } else {
        let mut st = c.prepare(&sql)?;
        let mapped = st.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, Option<i64>>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, Option<i64>>(5)?,
                r.get::<_, i64>(6)?,
                r.get::<_, i64>(7)?,
                r.get::<_, i64>(8)?,
            ))
        })?;
        for row in mapped {
            all.push(row?);
        }
    }
    let total = all.len() as i64;
    all.sort_by(|a, b| match sort {
        GroupSort::PhotosDesc => b.7.cmp(&a.7).then(b.0.cmp(&a.0)),
        GroupSort::FacesDesc => b.6.cmp(&a.6).then(b.0.cmp(&a.0)),
        GroupSort::NameAsc => match (&a.1, &b.1) {
            (None, None) => a.0.cmp(&b.0),
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (Some(_), None) => std::cmp::Ordering::Less,
            (Some(an), Some(bn)) => an
                .to_lowercase()
                .cmp(&bn.to_lowercase())
                .then(a.0.cmp(&b.0)),
        },
    });
    let start = after
        .and_then(|id| all.iter().position(|r| r.0 == id).map(|i| i + 1))
        .unwrap_or(0);
    let page: Vec<_> = all.into_iter().skip(start).take(limit as usize).collect();
    let mut items = Vec::with_capacity(page.len());
    for (
        id,
        name,
        identity_uid,
        ignored_at,
        ignored_faces,
        cover_id,
        faces,
        photos,
        discarded_faces,
    ) in page
    {
        let good_faces = (faces - discarded_faces).max(0);
        let new_faces = if ignored_at.is_some() {
            (faces - ignored_faces).max(0)
        } else {
            0
        };
        items.push(GroupRow {
            id,
            name,
            identity_uid,
            section,
            faces,
            good_faces,
            discarded_faces,
            photos,
            cover: cover_face(c, id, cover_id, m)?,
            ignored_at,
            new_faces,
            name_hint: None,
            merge_hint: merge_hint_for(c, id, m)?,
        });
    }
    let next = if items.len() == limit as usize {
        items.last().map(|r| r.id)
    } else {
        None
    };
    Ok(GroupPage { items, next, total })
}

/// Pagina di volti per gruppo, singoli o scartati.
pub fn group_faces(
    c: &Connection,
    scope: FaceScope,
    sort: FaceSort,
    after: Option<&FaceCursor>,
    limit: i64,
    m: &FaceMatch,
) -> StoreResult<FacePage> {
    let limit = clamp_limit(limit);
    let discarded = discarded_sql(m);
    let (filter_sql, bind_person, good_count, discarded_count, total) = match &scope {
        FaceScope::Group { group } => {
            let good: i64 = c.query_row(
                &format!("SELECT count(*) FROM face f WHERE f.person_id = ?1 AND NOT {discarded}"),
                [*group],
                |r| r.get(0),
            )?;
            let bad: i64 = c.query_row(
                &format!("SELECT count(*) FROM face f WHERE f.person_id = ?1 AND {discarded}"),
                [*group],
                |r| r.get(0),
            )?;
            (
                format!("f.person_id = ?1 AND NOT {discarded}"),
                Some(*group),
                good,
                bad,
                good,
            )
        }
        FaceScope::Singles => {
            let n: i64 = c.query_row(
                &format!(
                    "SELECT count(*) FROM face f WHERE f.person_id IS NULL AND NOT {discarded}"
                ),
                [],
                |r| r.get(0),
            )?;
            (
                format!("f.person_id IS NULL AND NOT {discarded}"),
                None,
                n,
                0,
                n,
            )
        }
        FaceScope::Discarded { group } => {
            if let Some(g) = *group {
                let n: i64 = c.query_row(
                    &format!("SELECT count(*) FROM face f WHERE f.person_id = ?1 AND {discarded}"),
                    [g],
                    |r| r.get(0),
                )?;
                (
                    format!("f.person_id = ?1 AND {discarded}"),
                    Some(g),
                    0,
                    n,
                    n,
                )
            } else {
                let n: i64 = c.query_row(
                    &format!("SELECT count(*) FROM face f WHERE {discarded}"),
                    [],
                    |r| r.get(0),
                )?;
                (discarded.clone(), None, 0, n, n)
            }
        }
    };

    let instant_sql = "COALESCE(d.instant_utc, CAST(strftime('%s', d.captured_at) AS INTEGER) * 1000, -9223372036854775807)";
    let (order_sql, after_q, after_id, after_t) = match sort {
        FaceSort::QualityDesc => match after {
            Some(cur) => (
                "COALESCE(f.quality_score, f.quality) DESC, f.id DESC".to_string(),
                Some(cur.quality_score),
                Some(cur.id),
                None::<i64>,
            ),
            None => (
                "COALESCE(f.quality_score, f.quality) DESC, f.id DESC".to_string(),
                None,
                None,
                None,
            ),
        },
        FaceSort::CapturedDesc => match after {
            Some(cur) => {
                let t: i64 = c
                    .query_row(
                        &format!(
                            "SELECT {instant_sql} FROM face f \
                             LEFT JOIN media_dates d ON d.media_id = f.media_id \
                             WHERE f.id = ?1"
                        ),
                        [cur.id],
                        |r| r.get(0),
                    )
                    .unwrap_or(i64::MIN);
                (
                    format!("{instant_sql} DESC, f.id DESC"),
                    None,
                    Some(cur.id),
                    Some(t),
                )
            }
            None => (format!("{instant_sql} DESC, f.id DESC"), None, None, None),
        },
    };

    let after_clause = match sort {
        FaceSort::QualityDesc if after_q.is_some() => {
            "AND (COALESCE(f.quality_score, f.quality) < ?aq \
              OR (COALESCE(f.quality_score, f.quality) = ?aq AND f.id < ?aid))"
        }
        FaceSort::CapturedDesc if after_t.is_some() => {
            "AND ({instant} < ?at OR ({instant} = ?at AND f.id < ?aid))"
        }
        _ => "",
    };
    let after_clause = after_clause.replace("{instant}", instant_sql);

    let mut sql = format!(
        "SELECT {FACE_VIEW_COLS}, {instant_sql} FROM face f \
         JOIN media_item m ON m.id = f.media_id \
         LEFT JOIN media_dates d ON d.media_id = f.media_id \
         WHERE {filter_sql}"
    );
    let mut binds: Vec<rusqlite::types::Value> = Vec::new();
    if let Some(pid) = bind_person {
        binds.push(pid.into());
    }
    let mut clause = after_clause;
    if after_q.is_some() || after_t.is_some() {
        let aq_i = binds.len() + 1;
        let aid_i = binds.len() + 2;
        if let Some(aq) = after_q {
            clause = clause.replace("?aq", &format!("?{aq_i}"));
            clause = clause.replace("?aid", &format!("?{aid_i}"));
            binds.push(aq.into());
            binds.push(after_id.unwrap().into());
        } else if let Some(at) = after_t {
            clause = clause.replace("?at", &format!("?{aq_i}"));
            clause = clause.replace("?aid", &format!("?{aid_i}"));
            binds.push(at.into());
            binds.push(after_id.unwrap().into());
        }
        sql.push(' ');
        sql.push_str(&clause);
    }
    let lim_i = binds.len() + 1;
    sql.push_str(&format!(" ORDER BY {order_sql} LIMIT ?{lim_i}"));
    binds.push(limit.into());

    let mut stmt = c.prepare(&sql)?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(binds), face_page_row)?
        .collect::<Result<Vec<_>, _>>()?;

    let mut items = Vec::with_capacity(rows.len());
    let mut last_instant = 0_i64;
    for row in rows {
        last_instant = row.13;
        items.push(map_face_view(
            row.0, row.1, row.2, row.3, row.4, row.5, row.6, row.7, row.8, row.9, row.10, row.11,
            row.12, m,
        )?);
    }
    let next = if items.len() == limit as usize {
        items.last().map(|f| match sort {
            FaceSort::QualityDesc => FaceCursor {
                quality_score: f.quality_score,
                id: f.id,
            },
            FaceSort::CapturedDesc => FaceCursor {
                quality_score: last_instant as f32,
                id: f.id,
            },
        })
    } else {
        None
    };
    Ok(FacePage {
        items,
        next,
        total,
        good_count,
        discarded_count,
    })
}

/// Posizione di un volto (o del volto scelto su una foto) nell'ordinamento
/// predefinito della relativa vista Persone, così la UI apre la pagina giusta.
pub fn locate_face(
    c: &Connection,
    req: &LocateFaceRequest,
    m: &FaceMatch,
) -> StoreResult<LocatedFace> {
    let face = resolve_locate_target(c, req, m)?;
    let person_id = face.person_id;
    let section = if let Some(pid) = person_id {
        let (ignored_at, identity_uid): (Option<i64>, Option<String>) = c
            .query_row(
                "SELECT ignored_at, identity_uid FROM person WHERE id=?1",
                [pid],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|_| StoreError::NotFound("persona".into()))?;
        if ignored_at.is_some() {
            PeopleSection::Ignored
        } else if identity_uid.is_some() {
            PeopleSection::People
        } else {
            PeopleSection::Review
        }
    } else {
        PeopleSection::Review
    };
    let discarded = face.is_discarded(m);
    let singles = person_id.is_none() && !discarded;
    let index = face_index_in_scope(c, &face, discarded, m)?;
    let media_name: String = c
        .query_row(
            "SELECT name FROM media_item WHERE id=?1",
            [face.media_id],
            |r| r.get(0),
        )
        .map_err(|_| StoreError::NotFound("media".into()))?;
    Ok(LocatedFace {
        face_id: face.id,
        media_id: face.media_id,
        media_name,
        section,
        group_id: person_id,
        singles,
        discarded,
        index,
    })
}

fn resolve_locate_target(
    c: &Connection,
    req: &LocateFaceRequest,
    m: &FaceMatch,
) -> StoreResult<FaceRecord> {
    match *req {
        LocateFaceRequest::Face { face_id } => face_record(c, face_id),
        LocateFaceRequest::Media {
            media_id,
            person_id: Some(pid),
        } => {
            let faces: Vec<_> = records(c, Some(media_id))?
                .into_iter()
                .filter(|f| f.person_id == Some(pid))
                .collect();
            faces
                .iter()
                .find(|f| !f.is_discarded(m))
                .or_else(|| faces.first())
                .cloned()
                .ok_or_else(|| StoreError::NotFound("volto".into()))
        }
        LocateFaceRequest::Media {
            media_id,
            person_id: None,
        } => {
            let faces = records(c, Some(media_id))?;
            faces
                .iter()
                .find(|f| f.person_id.is_some() && !f.is_discarded(m))
                .or_else(|| faces.first())
                .cloned()
                .ok_or_else(|| StoreError::NotFound("volto".into()))
        }
    }
}

fn face_record(c: &Connection, id: i64) -> StoreResult<FaceRecord> {
    let mut q = c.prepare(
        "SELECT id,media_id,bbox,quality,sharpness,frontality,size_px,COALESCE(quality_score,quality),quality_override,embedding,person_id,assigned_by,rejected,COALESCE(model_ver,'')
         FROM face WHERE id=?1",
    )?;
    let row = q
        .query_row([id], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, f32>(3)?,
                r.get::<_, Option<f32>>(4)?,
                r.get::<_, Option<f32>>(5)?,
                r.get::<_, Option<u32>>(6)?,
                r.get::<_, f32>(7)?,
                r.get::<_, i64>(8)?,
                r.get::<_, Option<Vec<u8>>>(9)?,
                r.get::<_, Option<i64>>(10)?,
                r.get::<_, Option<String>>(11)?,
                r.get::<_, String>(12)?,
                r.get::<_, String>(13)?,
            ))
        })
        .optional()?
        .ok_or_else(|| StoreError::NotFound("volto".into()))?;
    let (
        id,
        media_id,
        bbox,
        quality,
        sharpness,
        frontality,
        size_px,
        quality_score,
        quality_override,
        embed,
        person_id,
        assigned,
        reject,
        model,
    ) = row;
    Ok(FaceRecord {
        id,
        media_id,
        bbox: serde_json::from_str(&bbox)?,
        quality,
        sharpness,
        frontality,
        size_px,
        quality_score,
        quality_override: quality_override != 0,
        embedding: embed.map(|b| vector(&b)).transpose()?,
        person_id,
        manual: assigned.as_deref() == Some("manual"),
        rejected: serde_json::from_str(&reject)?,
        model,
    })
}

/// Indice 0-based nell'ordinamento `quality_score DESC, id DESC` dello scope.
fn face_index_in_scope(
    c: &Connection,
    face: &FaceRecord,
    discarded: bool,
    m: &FaceMatch,
) -> StoreResult<i64> {
    let discarded_pred = discarded_sql(m);
    let q = "COALESCE(f.quality_score, f.quality)";
    let scope = if discarded {
        match face.person_id {
            Some(_) => format!("f.person_id = ?1 AND {discarded_pred}"),
            None => discarded_pred.clone(),
        }
    } else if face.person_id.is_none() {
        format!("f.person_id IS NULL AND NOT {discarded_pred}")
    } else {
        format!("f.person_id = ?1 AND NOT {discarded_pred}")
    };
    let mut binds: Vec<rusqlite::types::Value> = Vec::new();
    if let Some(pid) = face.person_id {
        binds.push(pid.into());
    }
    let aq = binds.len() + 1;
    let aid = binds.len() + 2;
    binds.push(face.quality_score.into());
    binds.push(face.id.into());
    let sql = format!(
        "SELECT count(*) FROM face f WHERE {scope} \
         AND ({q} > ?{aq} OR ({q} = ?{aq} AND f.id > ?{aid}))"
    );
    Ok(c.query_row(&sql, rusqlite::params_from_iter(binds), |r| r.get(0))?)
}

type FacePageRow = (
    i64,
    i64,
    String,
    f32,
    f32,
    Option<f32>,
    Option<f32>,
    Option<u32>,
    i64,
    Option<String>,
    Option<String>,
    Option<i64>,
    Option<i32>,
    i64,
);

fn face_page_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<FacePageRow> {
    Ok((
        r.get(0)?,
        r.get(1)?,
        r.get(2)?,
        r.get(3)?,
        r.get(4)?,
        r.get(5)?,
        r.get(6)?,
        r.get(7)?,
        r.get(8)?,
        r.get(9)?,
        r.get(10)?,
        r.get(11)?,
        r.get(12)?,
        r.get(13)?,
    ))
}

pub fn refresh_centroids(c: &Connection) -> StoreResult<()> {
    refresh_centroids_with(
        c,
        &FaceMatch {
            min_sharpness: pc_core::AiThresholds::DEFAULT_FACE_MIN_SHARPNESS,
            ..Default::default()
        },
    )
}

pub fn refresh_centroids_with(c: &Connection, m: &FaceMatch) -> StoreResult<()> {
    let all = records(c, None)?;
    let mut groups: BTreeMap<(i64, String), (Vec<f32>, usize)> = BTreeMap::new();
    for f in &all {
        if f.is_discarded(m) {
            continue;
        }
        if let (Some(person), Some(v)) = (f.person_id, &f.embedding) {
            let (sum, n) = groups
                .entry((person, f.model.clone()))
                .or_insert((vec![0.0; 128], 0));
            for (a, b) in sum.iter_mut().zip(v) {
                *a += b;
            }
            *n += 1;
        }
    }
    c.execute("DELETE FROM person_centroid", [])?;
    c.execute("UPDATE person SET centroid=NULL", [])?;
    for ((id, model), (mut sum, _)) in groups {
        normalize(&mut sum);
        c.execute(
            "INSERT INTO person_centroid(person_id,model_ver,vector) VALUES (?1,?2,?3)",
            params![id, model, bytes(&sum)],
        )?;
        c.execute(
            "UPDATE person SET centroid=?2 WHERE id=?1",
            params![id, bytes(&sum)],
        )?;
    }
    Ok(())
}
/// Un suggerimento di unione tra due gruppi non ignorati (`a < b`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct MergeSuggestion {
    pub a: i64,
    pub b: i64,
    pub score: f32,
}

/// Ricalcola i suggerimenti di unione dai centroidi dei gruppi non ignorati:
/// - ogni gruppo deve avere almeno 3 volti non scartati;
/// - coseno tra centroidi ≥ `similarity * 0.8`;
/// - almeno 3 coppie di volti non scartati tra i due gruppi con coseno ≥ `similarity`;
/// - esclusione delle coppie in `person_not_same`;
/// - limite di al massimo 1 suggerimento per gruppo (scelto per punteggio decrescente).
pub fn refresh_merge_suggestions(
    c: &Connection,
    m: &FaceMatch,
) -> StoreResult<Vec<MergeSuggestion>> {
    let similarity = m.similarity;
    let centroid_min = similarity * 0.8;
    let not_same = not_same_pairs(c)?;
    let ignored: HashSet<i64> = {
        let mut q = c.prepare("SELECT id FROM person WHERE ignored_at IS NOT NULL")?;
        let v = q.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
        v
    };
    let all = records(c, None)?;
    // Raggruppa i volti validi per `(person_id, model)`.
    let mut by_group: BTreeMap<(i64, String), Vec<Vec<f32>>> = BTreeMap::new();
    for f in &all {
        if f.is_discarded(m) {
            continue;
        }
        let Some(pid) = f.person_id.filter(|p| !ignored.contains(p)) else {
            continue;
        };
        let Some(emb) = &f.embedding else {
            continue;
        };
        by_group
            .entry((pid, f.model.clone()))
            .or_default()
            .push(emb.clone());
    }
    let mut centroids: BTreeMap<String, Vec<(i64, Vec<f32>)>> = BTreeMap::new();
    {
        let mut q = c.prepare(
            "SELECT pc.person_id, pc.model_ver, pc.vector
             FROM person_centroid pc JOIN person p ON p.id = pc.person_id
             WHERE p.ignored_at IS NULL ORDER BY pc.person_id",
        )?;
        let rows = q
            .query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Vec<u8>>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        for (pid, model, raw) in rows {
            if by_group
                .get(&(pid, model.clone()))
                .is_some_and(|faces| faces.len() >= 3)
            {
                centroids
                    .entry(model)
                    .or_default()
                    .push((pid, vector(&raw)?));
            }
        }
    }
    let mut candidates: BTreeMap<(i64, i64), f32> = BTreeMap::new();
    for (model, group_centroids) in &centroids {
        for (i, (pa, ca)) in group_centroids.iter().enumerate() {
            for (pb, cb) in group_centroids.iter().skip(i + 1) {
                let (lo, hi) = ((*pa).min(*pb), (*pa).max(*pb));
                if not_same.contains(&(lo, hi)) {
                    continue;
                }
                let c_score = cosine(ca, cb);
                if c_score < centroid_min {
                    continue;
                }
                let Some(fa) = by_group.get(&(*pa, model.clone())) else {
                    continue;
                };
                let Some(fb) = by_group.get(&(*pb, model.clone())) else {
                    continue;
                };
                let mut pairs = 0usize;
                'outer: for va in fa {
                    for vb in fb {
                        if cosine(va, vb) >= similarity {
                            pairs += 1;
                            if pairs >= 3 {
                                break 'outer;
                            }
                        }
                    }
                }
                if pairs >= 3 {
                    let entry = candidates.entry((lo, hi)).or_insert(c_score);
                    if c_score > *entry {
                        *entry = c_score;
                    }
                }
            }
        }
    }
    let mut sorted: Vec<((i64, i64), f32)> = candidates.into_iter().collect();
    sorted.sort_by(|x, y| {
        y.1.total_cmp(&x.1)
            .then_with(|| x.0 .0.cmp(&y.0 .0))
            .then_with(|| x.0 .1.cmp(&y.0 .1))
    });
    let mut matched = HashSet::new();
    let mut chosen = Vec::new();
    for ((a, b), score) in sorted {
        if matched.contains(&a) || matched.contains(&b) {
            continue;
        }
        matched.insert(a);
        matched.insert(b);
        chosen.push(MergeSuggestion { a, b, score });
    }
    c.execute("DELETE FROM person_suggestion", [])?;
    let ts = now_ms();
    for s in &chosen {
        c.execute(
            "INSERT INTO person_suggestion(a, b, score, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![s.a, s.b, s.score, ts],
        )?;
    }
    Ok(chosen)
}

/// Suggerimenti di unione attualmente salvati, ordinati per punteggio decrescente.
pub fn list_merge_suggestions(c: &Connection) -> StoreResult<Vec<MergeSuggestion>> {
    let mut q = c.prepare("SELECT a, b, score FROM person_suggestion ORDER BY score DESC, a, b")?;
    let rows = q
        .query_map([], |r| {
            Ok(MergeSuggestion {
                a: r.get(0)?,
                b: r.get(1)?,
                score: r.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn named_match(
    c: &Connection,
    v: &[f32],
    model: &str,
    rejected: &[i64],
    threshold: f32,
) -> StoreResult<Option<i64>> {
    let mut q = c.prepare(
        "SELECT p.id,pc.vector FROM person p JOIN person_centroid pc ON pc.person_id=p.id
         WHERE (p.name IS NOT NULL OR p.identity_uid IS NOT NULL OR p.ignored_at IS NOT NULL) AND pc.model_ver=?1",
    )?;
    let rows = q.query_map([model], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, Vec<u8>>(1)?))
    })?;
    let mut best = (None, threshold);
    for row in rows {
        let (id, b) = row?;
        if rejected.contains(&id) {
            continue;
        }
        let score = cosine(v, &vector(&b)?);
        if score >= best.1 {
            best = (Some(id), score);
        }
    }
    Ok(best.0)
}
/// Riferimenti delle identità dell'app per una versione di modello, con i
/// nomi per creare la persona collegata in una libreria che non la conosce.
#[derive(Debug, Clone, Default)]
pub struct IdentityRefs {
    pub refs: Vec<crate::app_store::identities::Reference>,
    pub names: std::collections::HashMap<String, String>,
}

impl IdentityRefs {
    /// Riferimenti per `model_ver` letti da `app.db`.
    pub fn load(app: &crate::AppStore, model_ver: &str) -> StoreResult<Self> {
        app.with(|a| {
            Ok(Self {
                refs: crate::app_store::identities::references(a, model_ver)?,
                names: crate::app_store::identities::list(a)?
                    .into_iter()
                    .map(|i| (i.uid, i.name))
                    .collect(),
            })
        })
    }
    pub fn is_empty(&self) -> bool {
        self.refs.is_empty()
    }
}

/// Come [`named_match`], confrontando anche i volti di riferimento delle
/// identità (ognuna con la propria soglia). Vince il punteggio più alto. Se
/// vince un'identità non ancora collegata in questa libreria e `create` è vero,
/// viene creata la persona collegata; con `create` falso (letture) il volto
/// resta senza persona. Le esclusioni ("non è questa persona") valgono anche
/// per l'identità collegata alla persona esclusa.
pub fn named_match_refs(
    c: &Connection,
    v: &[f32],
    model: &str,
    rejected: &[i64],
    threshold: f32,
    ids: &IdentityRefs,
    create: bool,
) -> StoreResult<Option<i64>> {
    let mut q = c.prepare(
        "SELECT p.id,pc.vector FROM person p JOIN person_centroid pc ON pc.person_id=p.id
         WHERE (p.name IS NOT NULL OR p.identity_uid IS NOT NULL OR p.ignored_at IS NOT NULL) AND pc.model_ver=?1",
    )?;
    let rows = q.query_map([model], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, Vec<u8>>(1)?))
    })?;
    let mut best: (Option<i64>, f32) = (None, threshold);
    for row in rows {
        let (id, b) = row?;
        if rejected.contains(&id) {
            continue;
        }
        let score = cosine(v, &vector(&b)?);
        if score >= best.1 {
            best = (Some(id), score);
        }
    }
    if ids.is_empty() {
        return Ok(best.0);
    }
    let mut excluded = Vec::new();
    for p in rejected {
        if let Some(uid) = c
            .query_row("SELECT identity_uid FROM person WHERE id=?1", [p], |r| {
                r.get::<_, Option<String>>(0)
            })
            .optional()?
            .flatten()
        {
            excluded.push(uid);
        }
    }
    let Some((r, score)) = crate::app_store::identities::best_reference(&ids.refs, v, &excluded)
    else {
        return Ok(best.0);
    };
    if best.0.is_some() && score < best.1 {
        return Ok(best.0);
    }
    match person_for_identity(c, &r.uid)? {
        Some(p) => Ok(Some(p)),
        None if create => {
            let name = ids.names.get(&r.uid).cloned().unwrap_or_default();
            Ok(Some(ensure_identity_person(c, &r.uid, &name)?))
        }
        None => Ok(best.0),
    }
}

pub fn save_faces(
    c: &Connection,
    media: i64,
    detected: &[DetectedFace],
    embedding_model: &str,
    analysis_model: &str,
    threshold: f32,
) -> StoreResult<()> {
    save_faces_with(
        c,
        media,
        detected,
        embedding_model,
        analysis_model,
        threshold,
        &IdentityRefs::default(),
    )
}

/// Salva i volti rilevati e li associa a persone con nome o identità dell'app.
pub fn save_faces_with(
    c: &Connection,
    media: i64,
    detected: &[DetectedFace],
    embedding_model: &str,
    analysis_model: &str,
    threshold: f32,
    ids: &IdentityRefs,
) -> StoreResult<()> {
    save_faces_tuned(
        c,
        media,
        detected,
        embedding_model,
        analysis_model,
        FaceMatch {
            similarity: threshold,
            ..Default::default()
        },
        ids,
    )
}

/// Soglie applicate al salvataggio dei volti.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaceMatch {
    /// Somiglianza minima per associare un volto a una persona.
    pub similarity: f32,
    /// Confidenza minima di rilevamento di un volto usato per il riconoscimento:
    /// i volti sotto restano salvati ma senza persona automatica.
    pub min_quality: f32,
    /// Nitidezza minima (0..1) di un volto usato per il riconoscimento.
    pub min_sharpness: f32,
    /// Lato minimo in pixel di un volto usato per il riconoscimento.
    pub min_size_px: u32,
    /// Pavimento del rilevatore, registrato nella versione del risultato.
    pub floor: Option<f32>,
}

impl Default for FaceMatch {
    fn default() -> Self {
        Self {
            similarity: 0.363,
            min_quality: 0.0,
            min_sharpness: 0.0,
            min_size_px: 0,
            floor: None,
        }
    }
}

/// Come [`save_faces_with`], con soglia di qualità e pavimento registrato.
pub fn save_faces_tuned(
    c: &Connection,
    media: i64,
    detected: &[DetectedFace],
    embedding_model: &str,
    analysis_model: &str,
    m: FaceMatch,
    ids: &IdentityRefs,
) -> StoreResult<()> {
    let threshold = m.similarity;
    for f in detected {
        if f.embedding.len() != 128
            || f.embedding.iter().any(|x| !x.is_finite())
            || f.bbox
                .iter()
                .any(|x| !x.is_finite() || !(0.0..=1.0).contains(x))
            || f.bbox[2] <= 0.0
            || f.bbox[3] <= 0.0
            || !f.quality.is_finite()
        {
            return Err(StoreError::Invalid("volto non valido".into()));
        }
    }
    let old = records(c, Some(media))?;
    let mut used = HashSet::new();
    let named = named_ids(c)?;
    let mut q = c.prepare("SELECT bbox FROM face_exclusion WHERE media_id=?1")?;
    let excluded = q
        .query_map([media], |r| r.get::<_, String>(0))?
        .map(|r| Ok(serde_json::from_str::<[f32; 4]>(&r?)?))
        .collect::<StoreResult<Vec<_>>>()?;
    for f in detected {
        if excluded.iter().any(|b| iou(*b, f.bbox) >= 0.6) {
            continue;
        }
        let qscore = f
            .quality_score
            .unwrap_or_else(|| combined_quality(f.quality, f.sharpness, f.frontality, f.size_px));
        let prior = old
            .iter()
            .filter(|x| !used.contains(&x.id) && iou(x.bbox, f.bbox) >= 0.5)
            .max_by(|a, b| iou(a.bbox, f.bbox).total_cmp(&iou(b.bbox, f.bbox)));
        let (id, overridden) = if let Some(prior) = prior {
            used.insert(prior.id);
            c.execute(
                "UPDATE face SET bbox=?2,quality=?3,sharpness=?4,frontality=?5,size_px=?6,quality_score=?7,embedding=?8,model_ver=?9 WHERE id=?1",
                params![
                    prior.id,
                    serde_json::to_string(&f.bbox)?,
                    f.quality,
                    f.sharpness,
                    f.frontality,
                    f.size_px,
                    qscore,
                    bytes(&f.embedding),
                    embedding_model
                ],
            )?;
            (prior.id, prior.quality_override)
        } else {
            let id = next_id(c, "face")?;
            c.execute(
                "INSERT INTO face(id,media_id,bbox,quality,sharpness,frontality,size_px,quality_score,embedding,model_ver) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                params![
                    id,
                    media,
                    serde_json::to_string(&f.bbox)?,
                    f.quality,
                    f.sharpness,
                    f.frontality,
                    f.size_px,
                    qscore,
                    bytes(&f.embedding),
                    embedding_model
                ],
            )?;
            (id, false)
        };
        let prior = prior.filter(|p| {
            p.person_id
                .is_some_and(|id| p.manual || named.contains(&id))
        });
        if prior.is_none() {
            let rejects = old
                .iter()
                .find(|x| x.id == id)
                .map(|x| x.rejected.as_slice())
                .unwrap_or(&[]);
            let discarded = discard_reason(
                f.quality,
                f.sharpness,
                f.frontality,
                f.size_px,
                false,
                overridden,
                &m,
            )
            .is_some();
            let person = if discarded {
                None
            } else {
                named_match_refs(
                    c,
                    &f.embedding,
                    embedding_model,
                    rejects,
                    threshold,
                    ids,
                    true,
                )?
            };
            c.execute(
                "UPDATE face SET person_id=?2,assigned_by='auto' WHERE id=?1",
                params![id, person],
            )?;
        }
    }
    for f in old.iter().filter(|f| !used.contains(&f.id)) {
        let keep = if let Some(p) = f.person_id {
            c.query_row(
                "SELECT (name IS NOT NULL OR identity_uid IS NOT NULL OR ignored_at IS NOT NULL) FROM person WHERE id=?1",
                [p],
                |r| r.get::<_, bool>(0),
            )
            .optional()?
            .unwrap_or(false)
        } else {
            false
        };
        if f.manual || keep {
            c.execute("UPDATE face SET embedding=NULL WHERE id=?1", [f.id])?;
        } else {
            c.execute("DELETE FROM face WHERE id=?1", [f.id])?;
        }
    }
    refresh_centroids_with(c, &m)?;
    let ver = m.floor.map_or_else(
        || analysis_model.to_string(),
        |f| analysis::with_floor(analysis_model, f),
    );
    analysis::version(c, media, "faces", &ver, None)?;
    repo::fts::refresh(c, media)?;
    Ok(())
}

/// Condizione SQL per i volti automatici scartati per qualità.
fn discarded_auto_sql() -> &'static str {
    "COALESCE(assigned_by,'auto') <> 'manual' AND quality_override = 0
     AND (quality < ?1
          OR (sharpness IS NOT NULL AND sharpness < ?2)
          OR (size_px IS NOT NULL AND size_px < ?3)
          OR (frontality IS NOT NULL AND frontality < ?4))"
}

/// Toglie la persona automatica ai volti scartati secondo le soglie `m`
/// (le assegnazioni manuali e i volti recuperati restano). Con `write` falso
/// conta soltanto. Restituisce il numero di volti interessati.
pub fn unassign_discarded(c: &Connection, m: &FaceMatch, write: bool) -> StoreResult<i64> {
    let filter = format!("person_id IS NOT NULL AND {}", discarded_auto_sql());
    let args = params![
        m.min_quality,
        m.min_sharpness,
        m.min_size_px,
        MIN_FRONTALITY
    ];
    let ids: Vec<i64> = {
        let mut q = c.prepare(&format!(
            "SELECT DISTINCT media_id FROM face WHERE {filter}"
        ))?;
        let v = q.query_map(args, |r| r.get(0))?.collect::<Result<_, _>>()?;
        v
    };
    let n: i64 = c.query_row(
        &format!("SELECT count(*) FROM face WHERE {filter}"),
        args,
        |r| r.get(0),
    )?;
    if write && n > 0 {
        c.execute(
            &format!("UPDATE face SET person_id=NULL,assigned_by='auto' WHERE {filter}"),
            args,
        )?;
        refresh_centroids_with(c, m)?;
        for id in ids {
            repo::fts::refresh(c, id)?;
        }
    }
    Ok(n)
}

/// Toglie la persona automatica ai volti sotto la confidenza `min_quality`
/// (le assegnazioni manuali restano). Con `write` falso conta soltanto.
/// Restituisce il numero di volti interessati.
pub fn unassign_low_quality(c: &Connection, min_quality: f32, write: bool) -> StoreResult<i64> {
    unassign_discarded(
        c,
        &FaceMatch {
            min_quality,
            ..Default::default()
        },
        write,
    )
}

/// Volti con embedding esclusi dal riconoscimento con la soglia `min_quality`.
pub fn faces_below(c: &Connection, min_quality: f32) -> StoreResult<i64> {
    Ok(c.query_row(
        "SELECT count(*) FROM face WHERE quality < ?1 AND embedding IS NOT NULL AND quality_override = 0",
        [min_quality],
        |r| r.get(0),
    )?)
}

/// Volti con embedding scartati per nitidezza sotto `min_sharpness`.
pub fn faces_below_sharpness(c: &Connection, min_sharpness: f32) -> StoreResult<i64> {
    Ok(c.query_row(
        "SELECT count(*) FROM face WHERE sharpness IS NOT NULL AND sharpness < ?1 AND embedding IS NOT NULL AND quality_override = 0",
        [min_sharpness],
        |r| r.get(0),
    )?)
}

/// Stima delle persone senza nome che cambierebbero raggruppamento con la
/// somiglianza `similarity`: quelle con almeno un volto automatico meno simile
/// del valore al centroide della persona. Le persone con nome sono protette.
pub fn persons_below_similarity(c: &Connection, similarity: f32) -> StoreResult<i64> {
    let mut q = c.prepare(
        "SELECT f.person_id,f.embedding,pc.vector FROM face f
         JOIN person p ON p.id=f.person_id AND p.name IS NULL
         JOIN person_centroid pc ON pc.person_id=f.person_id AND pc.model_ver=f.model_ver
         WHERE f.embedding IS NOT NULL AND COALESCE(f.assigned_by,'auto') <> 'manual'",
    )?;
    let rows = q
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, Vec<u8>>(1)?,
                r.get::<_, Vec<u8>>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut persons = HashSet::new();
    for (p, face, centroid) in rows {
        if cosine(&vector(&face)?, &vector(&centroid)?) < similarity {
            persons.insert(p);
        }
    }
    Ok(persons.len() as i64)
}
pub fn named_ids(c: &Connection) -> StoreResult<HashSet<i64>> {
    let mut q = c.prepare(
        "SELECT id FROM person WHERE name IS NOT NULL OR identity_uid IS NOT NULL OR ignored_at IS NOT NULL",
    )?;
    let mut ids: HashSet<i64> = q.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
    let mut ns = c.prepare("SELECT a, b FROM person_not_same")?;
    for row in ns.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))? {
        let (a, b) = row?;
        ids.insert(a);
        ids.insert(b);
    }
    Ok(ids)
}

/// Coppie `(a, b)` e `(b, a)` marcate dall'utente come "persone diverse".
pub fn not_same_pairs(c: &Connection) -> StoreResult<HashSet<(i64, i64)>> {
    let mut q = c.prepare("SELECT a, b FROM person_not_same")?;
    let mut out = HashSet::new();
    for row in q.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))? {
        let (a, b) = row?;
        out.insert((a, b));
        out.insert((b, a));
    }
    Ok(out)
}

/// Registra che i gruppi `a` e `b` sono persone diverse e rimuove il suggerimento.
pub fn mark_not_same(c: &Connection, a: i64, b: i64) -> StoreResult<()> {
    if a == b {
        return Err(StoreError::Invalid("persone coincidenti".into()));
    }
    for id in [a, b] {
        if !c.query_row(
            "SELECT EXISTS(SELECT 1 FROM person WHERE id=?1)",
            [id],
            |r| r.get::<_, bool>(0),
        )? {
            return Err(StoreError::NotFound("persona".into()));
        }
    }
    let (lo, hi) = (a.min(b), a.max(b));
    c.execute(
        "INSERT OR IGNORE INTO person_not_same(a, b, created_at) VALUES (?1, ?2, ?3)",
        params![lo, hi, now_ms()],
    )?;
    c.execute(
        "DELETE FROM person_suggestion WHERE a=?1 AND b=?2",
        params![lo, hi],
    )?;
    Ok(())
}

pub fn remove_not_same(c: &Connection, a: i64, b: i64) -> StoreResult<()> {
    let (lo, hi) = (a.min(b), a.max(b));
    c.execute(
        "DELETE FROM person_not_same WHERE a=?1 AND b=?2",
        params![lo, hi],
    )?;
    Ok(())
}

/// Ignora un gruppo o una persona: non compare più in "Da rivedere", nei filtri,
/// nell'Inspector né nei suggerimenti, e attrae i nuovi volti simili.
/// Restituisce `(ignored_at, ignored_faces)` precedenti per l'annullamento.
pub fn ignore_person(c: &Connection, id: i64) -> StoreResult<(Option<i64>, i64)> {
    let prev: (Option<i64>, i64) = c
        .query_row(
            "SELECT ignored_at, ignored_faces FROM person WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| StoreError::NotFound("persona".into()))?;
    let faces: i64 = c.query_row("SELECT count(*) FROM face WHERE person_id=?1", [id], |r| {
        r.get(0)
    })?;
    c.execute(
        "UPDATE person SET ignored_at=?2, ignored_faces=?3 WHERE id=?1",
        params![id, now_ms(), faces],
    )?;
    c.execute("DELETE FROM person_suggestion WHERE a=?1 OR b=?1", [id])?;
    refresh_centroids(c)?;
    refresh_media(c, &person_media(c, id)?)?;
    Ok(prev)
}

/// Ripristina un gruppo ignorato. Restituisce `(ignored_at, ignored_faces)` precedenti.
pub fn restore_person(c: &Connection, id: i64) -> StoreResult<(Option<i64>, i64)> {
    let prev: (Option<i64>, i64) = c
        .query_row(
            "SELECT ignored_at, ignored_faces FROM person WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| StoreError::NotFound("persona".into()))?;
    c.execute(
        "UPDATE person SET ignored_at=NULL, ignored_faces=0 WHERE id=?1",
        [id],
    )?;
    refresh_centroids(c)?;
    refresh_media(c, &person_media(c, id)?)?;
    Ok(prev)
}

/// Unisce più gruppi in `to` (salta eventuali `from == to`).
pub fn merge_many(c: &Connection, from: &[i64], to: i64) -> StoreResult<()> {
    if from.is_empty() {
        return Err(StoreError::Invalid("nessun gruppo da unire".into()));
    }
    for id in from {
        if *id == to {
            continue;
        }
        merge(c, *id, to)?;
    }
    Ok(())
}

/// Sposta i volti su un gruppo (assegnazione manuale).
pub fn move_faces(c: &Connection, faces: &[i64], to: i64) -> StoreResult<()> {
    assign(c, faces, to)
}

/// Recupera volti scartati per qualità (`quality_override = 1`).
pub fn recover_faces(c: &Connection, faces: &[i64]) -> StoreResult<()> {
    if faces.is_empty() {
        return Ok(());
    }
    let placeholders = faces.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!("UPDATE face SET quality_override=1 WHERE id IN ({placeholders})");
    c.execute(&sql, rusqlite::params_from_iter(faces.iter()))?;
    refresh_centroids(c)
}

/// Imposta la copertina del gruppo: il volto deve appartenere al gruppo.
pub fn set_cover(c: &Connection, group: i64, face: i64) -> StoreResult<()> {
    let person: Option<i64> = c
        .query_row("SELECT person_id FROM face WHERE id=?1", [face], |r| {
            r.get(0)
        })
        .optional()?
        .flatten();
    if person != Some(group) {
        return Err(StoreError::Invalid(
            "il volto non appartiene al gruppo".into(),
        ));
    }
    if c.execute(
        "UPDATE person SET cover_face_id=?2 WHERE id=?1",
        params![group, face],
    )? == 0
    {
        return Err(StoreError::NotFound("persona".into()));
    }
    Ok(())
}

/// Rimuove la copertina impostata a mano (torna alla scelta automatica).
pub fn clear_cover(c: &Connection, group: i64) -> StoreResult<()> {
    if c.execute("UPDATE person SET cover_face_id=NULL WHERE id=?1", [group])? == 0 {
        return Err(StoreError::NotFound("persona".into()));
    }
    Ok(())
}

/// Copertina attuale del gruppo (`None` = automatica).
pub fn cover_face_id(c: &Connection, group: i64) -> StoreResult<Option<i64>> {
    c.query_row(
        "SELECT cover_face_id FROM person WHERE id=?1",
        [group],
        |r| r.get(0),
    )
    .map_err(|_| StoreError::NotFound("persona".into()))
}

/// Snapshot minimale di una persona per l'annullamento di un'unione.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersonSnapshot {
    pub id: i64,
    pub name: Option<String>,
    pub identity_uid: Option<String>,
    pub ignored_at: Option<i64>,
    pub ignored_faces: i64,
    pub cover_face_id: Option<i64>,
}

pub fn person_snapshot(c: &Connection, id: i64) -> StoreResult<PersonSnapshot> {
    c.query_row(
        "SELECT id, name, identity_uid, ignored_at, ignored_faces, cover_face_id \
         FROM person WHERE id=?1",
        [id],
        |r| {
            Ok(PersonSnapshot {
                id: r.get(0)?,
                name: r.get(1)?,
                identity_uid: r.get(2)?,
                ignored_at: r.get(3)?,
                ignored_faces: r.get(4)?,
                cover_face_id: r.get(5)?,
            })
        },
    )
    .map_err(|_| StoreError::NotFound("persona".into()))
}

/// `(face_id, person_id)` per i volti dei gruppi indicati.
pub fn faces_of_persons(c: &Connection, persons: &[i64]) -> StoreResult<Vec<(i64, i64)>> {
    if persons.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = persons.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql =
        format!("SELECT id, person_id FROM face WHERE person_id IN ({placeholders}) ORDER BY id");
    let mut q = c.prepare(&sql)?;
    let rows = q
        .query_map(rusqlite::params_from_iter(persons.iter()), |r| {
            Ok((r.get(0)?, r.get(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Persona attuale di ogni volto (`None` se non assegnato).
pub fn face_person_ids(c: &Connection, faces: &[i64]) -> StoreResult<Vec<(i64, Option<i64>)>> {
    let mut out = Vec::with_capacity(faces.len());
    for id in faces {
        let person = c
            .query_row("SELECT person_id FROM face WHERE id=?1", [*id], |r| {
                r.get::<_, Option<i64>>(0)
            })
            .map_err(|_| StoreError::NotFound("volto".into()))?;
        out.push((*id, person));
    }
    Ok(out)
}

/// Ripristina le assegnazioni persona dei volti (senza aggiornare i rifiuti).
pub fn restore_face_persons(c: &Connection, faces: &[(i64, Option<i64>)]) -> StoreResult<()> {
    let mut media = HashSet::new();
    for (face, person) in faces {
        let media_id: i64 = c
            .query_row("SELECT media_id FROM face WHERE id=?1", [*face], |r| {
                r.get(0)
            })
            .map_err(|_| StoreError::NotFound("volto".into()))?;
        c.execute(
            "UPDATE face SET person_id=?2 WHERE id=?1",
            params![face, person],
        )?;
        media.insert(media_id);
    }
    refresh_centroids(c)?;
    refresh_media(c, &media)
}

/// Reinserisce una persona eliminata (es. annullamento di un'unione).
pub fn restore_person_row(c: &Connection, snap: &PersonSnapshot) -> StoreResult<()> {
    c.execute(
        "INSERT INTO person(id, name, identity_uid, ignored_at, ignored_faces, cover_face_id, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            snap.id,
            snap.name,
            snap.identity_uid,
            snap.ignored_at,
            snap.ignored_faces,
            snap.cover_face_id,
            now_ms(),
        ],
    )?;
    Ok(())
}

/// Stato di un volto per l'annullamento: persona, origine dell'assegnazione e rifiuti.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FaceState {
    pub id: i64,
    pub person_id: Option<i64>,
    pub assigned_by: Option<String>,
    /// JSON dei gruppi rifiutati, come salvato.
    pub rejected: String,
}

/// Volti assegnati a uno dei gruppi indicati o che li rifiutano.
pub fn faces_touching(c: &Connection, persons: &[i64]) -> StoreResult<Vec<i64>> {
    if persons.is_empty() {
        return Ok(Vec::new());
    }
    let list = persons.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT id FROM face WHERE person_id IN ({list}) \
         OR EXISTS (SELECT 1 FROM json_each(face.rejected) j WHERE j.value IN ({list})) \
         ORDER BY id"
    );
    let args: Vec<i64> = persons.iter().chain(persons.iter()).copied().collect();
    let mut q = c.prepare(&sql)?;
    let rows = q
        .query_map(rusqlite::params_from_iter(args), |r| r.get(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Stato attuale dei volti indicati (quelli inesistenti sono ignorati).
pub fn face_states(c: &Connection, faces: &[i64]) -> StoreResult<Vec<FaceState>> {
    let mut q =
        c.prepare_cached("SELECT person_id, assigned_by, rejected FROM face WHERE id=?1")?;
    let mut out = Vec::with_capacity(faces.len());
    for id in faces {
        if let Some((person_id, assigned_by, rejected)) = q
            .query_row([*id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .optional()?
        {
            out.push(FaceState {
                id: *id,
                person_id,
                assigned_by,
                rejected,
            });
        }
    }
    Ok(out)
}

/// Ripristina lo stato salvato dei volti, poi centroidi e persone delle foto.
pub fn restore_face_states(c: &Connection, states: &[FaceState]) -> StoreResult<()> {
    let mut media = HashSet::new();
    for s in states {
        let Some(media_id) = c
            .query_row("SELECT media_id FROM face WHERE id=?1", [s.id], |r| {
                r.get::<_, i64>(0)
            })
            .optional()?
        else {
            continue;
        };
        c.execute(
            "UPDATE face SET person_id=?2, assigned_by=?3, rejected=?4 WHERE id=?1",
            params![s.id, s.person_id, s.assigned_by, s.rejected],
        )?;
        media.insert(media_id);
    }
    refresh_centroids(c)?;
    refresh_media(c, &media)
}

/// Vincoli "persone diverse" che coinvolgono i gruppi indicati: `(a, b, created_at)`.
pub fn not_same_rows(c: &Connection, persons: &[i64]) -> StoreResult<Vec<(i64, i64, i64)>> {
    let mut out = Vec::new();
    let mut q = c.prepare_cached(
        "SELECT a, b, created_at FROM person_not_same WHERE a=?1 OR b=?1 ORDER BY a, b",
    )?;
    for p in persons {
        for row in q.query_map([*p], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))? {
            out.push(row?);
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

/// Rimette i vincoli "persone diverse" dei gruppi indicati com'erano.
pub fn restore_not_same(
    c: &Connection,
    persons: &[i64],
    rows: &[(i64, i64, i64)],
) -> StoreResult<()> {
    for p in persons {
        c.execute("DELETE FROM person_not_same WHERE a=?1 OR b=?1", [*p])?;
    }
    for (a, b, ts) in rows {
        c.execute(
            "INSERT OR IGNORE INTO person_not_same(a, b, created_at) VALUES (?1, ?2, ?3)",
            params![a, b, ts],
        )?;
    }
    Ok(())
}

/// Riporta nome, identità, stato "ignorato" e copertina di una persona esistente.
pub fn restore_person_fields(c: &Connection, snap: &PersonSnapshot) -> StoreResult<()> {
    c.execute(
        "UPDATE person SET name=?2, identity_uid=?3, ignored_at=?4, ignored_faces=?5, \
         cover_face_id=?6 WHERE id=?1",
        params![
            snap.id,
            snap.name,
            snap.identity_uid,
            snap.ignored_at,
            snap.ignored_faces,
            snap.cover_face_id
        ],
    )?;
    Ok(())
}

/// Ripristina ignored_at / ignored_faces (annullamento di ignora/ripristina).
pub fn set_ignored_state(
    c: &Connection,
    id: i64,
    ignored_at: Option<i64>,
    ignored_faces: i64,
) -> StoreResult<()> {
    c.execute(
        "UPDATE person SET ignored_at=?2, ignored_faces=?3 WHERE id=?1",
        params![id, ignored_at, ignored_faces],
    )?;
    refresh_centroids(c)?;
    refresh_media(c, &person_media(c, id)?)
}

/// Annulla un recupero: `quality_override = 0`.
pub fn unrecover_faces(c: &Connection, faces: &[i64]) -> StoreResult<()> {
    if faces.is_empty() {
        return Ok(());
    }
    let placeholders = faces.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!("UPDATE face SET quality_override=0 WHERE id IN ({placeholders})");
    c.execute(&sql, rusqlite::params_from_iter(faces.iter()))?;
    refresh_centroids(c)
}
fn refresh_media(c: &Connection, ids: &HashSet<i64>) -> StoreResult<()> {
    for id in ids {
        repo::fts::refresh(c, *id)?;
    }
    Ok(())
}
fn person_media(c: &Connection, person: i64) -> StoreResult<HashSet<i64>> {
    let mut q = c.prepare("SELECT DISTINCT media_id FROM face WHERE person_id=?1")?;
    let out = q
        .query_map([person], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    Ok(out)
}
pub fn rename(c: &Connection, id: i64, name: &str) -> StoreResult<()> {
    let name = name.trim();
    if name.is_empty() || name.len() > 256 {
        return Err(StoreError::Invalid("nome persona non valido".into()));
    }
    if c.execute("UPDATE person SET name=?2 WHERE id=?1", params![id, name])? == 0 {
        return Err(StoreError::NotFound("persona".into()));
    }
    refresh_centroids(c)?;
    refresh_media(c, &person_media(c, id)?)
}
pub fn assign(c: &Connection, face_ids: &[i64], person: i64) -> StoreResult<()> {
    if !c.query_row(
        "SELECT EXISTS(SELECT 1 FROM person WHERE id=?1)",
        [person],
        |r| r.get::<_, bool>(0),
    )? {
        return Err(StoreError::NotFound("persona".into()));
    }
    let records = records(c, None)?;
    let mut media = HashSet::new();
    for id in face_ids {
        let f = records
            .iter()
            .find(|f| f.id == *id)
            .ok_or_else(|| StoreError::NotFound("volto".into()))?;
        let mut reject = f.rejected.clone();
        reject.retain(|p| *p != person);
        if let Some(old) = f.person_id.filter(|p| *p != person) {
            if !reject.contains(&old) {
                reject.push(old);
            }
        }
        c.execute(
            "UPDATE face SET person_id=?2,assigned_by='manual',rejected=?3 WHERE id=?1",
            params![id, person, serde_json::to_string(&reject)?],
        )?;
        media.insert(f.media_id);
    }
    refresh_centroids(c)?;
    refresh_media(c, &media)
}
pub fn separate(c: &Connection, ids: &[i64], from: i64) -> StoreResult<i64> {
    if ids.is_empty() {
        return Err(StoreError::Invalid("nessun volto selezionato".into()));
    }
    let all = records(c, None)?;
    if ids
        .iter()
        .any(|id| !all.iter().any(|f| f.id == *id && f.person_id == Some(from)))
    {
        return Err(StoreError::Invalid(
            "volti non appartenenti alla persona".into(),
        ));
    }
    let target = create_person(c, None)?;
    c.execute(
        "UPDATE face SET assigned_by='manual' WHERE person_id=?1",
        [from],
    )?;
    assign(c, ids, target)?;
    Ok(target)
}
pub fn reject(c: &Connection, face: i64, person: i64) -> StoreResult<()> {
    let f = records(c, None)?
        .into_iter()
        .find(|f| f.id == face)
        .ok_or_else(|| StoreError::NotFound("volto".into()))?;
    let mut rejected = f.rejected;
    if !rejected.contains(&person) {
        rejected.push(person);
    }
    c.execute(
        "UPDATE face SET rejected=?2,person_id=NULL,assigned_by=NULL WHERE id=?1",
        params![face, serde_json::to_string(&rejected)?],
    )?;
    refresh_centroids(c)?;
    repo::fts::refresh(c, f.media_id)
}
pub fn merge(c: &Connection, from: i64, to: i64) -> StoreResult<()> {
    if from == to {
        return Err(StoreError::Invalid("persone coincidenti".into()));
    }
    for id in [from, to] {
        if !c.query_row(
            "SELECT EXISTS(SELECT 1 FROM person WHERE id=?1)",
            [id],
            |r| r.get::<_, bool>(0),
        )? {
            return Err(StoreError::NotFound("persona".into()));
        }
    }
    let mut media = person_media(c, from)?;
    media.extend(person_media(c, to)?);
    // Il nome e il collegamento all'identità sopravvivono all'unione: se solo il gruppo
    // eliminato li aveva, passano a quello di destinazione.
    let (from_name, from_uid): (Option<String>, Option<String>) = c.query_row(
        "SELECT name, identity_uid FROM person WHERE id=?1",
        [from],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    if from_name.is_some() || from_uid.is_some() {
        c.execute(
            "UPDATE person SET name = COALESCE(name, ?2), \
             identity_uid = COALESCE(identity_uid, ?3) WHERE id=?1",
            params![to, from_name, from_uid],
        )?;
    }
    c.execute(
        "UPDATE face SET person_id=?2,assigned_by='manual' WHERE person_id=?1",
        params![from, to],
    )?;
    // Solo i volti che rifiutano `from` o appartengono a `to` cambiano i rifiuti.
    for id in faces_touching(c, &[from, to])? {
        let (person_id, rejected): (Option<i64>, String) = c.query_row(
            "SELECT person_id, rejected FROM face WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let mut rejected: Vec<i64> = serde_json::from_str::<Vec<i64>>(&rejected)?
            .into_iter()
            .map(|p| if p == from { to } else { p })
            .collect();
        rejected.sort();
        rejected.dedup();
        if person_id == Some(to) {
            rejected.retain(|p| *p != to);
        }
        c.execute(
            "UPDATE face SET rejected=?2 WHERE id=?1",
            params![id, serde_json::to_string(&rejected)?],
        )?;
    }
    // Propaga i vincoli "persone diverse" al gruppo di destinazione.
    let mut q = c.prepare("SELECT a, b, created_at FROM person_not_same WHERE a=?1 OR b=?1")?;
    let ns_rows = q
        .query_map([from], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(q);
    c.execute("DELETE FROM person_not_same WHERE a=?1 OR b=?1", [from])?;
    for (a, b, ts) in ns_rows {
        let other = if a == from { b } else { a };
        if other != to {
            let (lo, hi) = (to.min(other), to.max(other));
            c.execute(
                "INSERT OR IGNORE INTO person_not_same(a, b, created_at) VALUES (?1, ?2, ?3)",
                params![lo, hi, ts],
            )?;
        }
    }
    c.execute("DELETE FROM person_suggestion WHERE a=?1 OR b=?1", [from])?;
    c.execute("DELETE FROM person WHERE id=?1", [from])?;
    refresh_centroids(c)?;
    refresh_media(c, &media)
}
pub fn remove_face(c: &Connection, id: i64) -> StoreResult<()> {
    let f = records(c, None)?
        .into_iter()
        .find(|f| f.id == id)
        .ok_or_else(|| StoreError::NotFound("volto".into()))?;
    c.execute(
        "INSERT INTO face_exclusion(media_id,bbox) VALUES (?1,?2)",
        params![f.media_id, serde_json::to_string(&f.bbox)?],
    )?;
    c.execute("DELETE FROM face WHERE id=?1", [id])?;
    refresh_centroids(c)?;
    repo::fts::refresh(c, f.media_id)
}
/// "Elimina tutti i dati dei volti": volti e persone della libreria e volti di
/// riferimento delle identità in `app.db` (le identità restano).
pub fn clear_with_references(c: &Connection, app: &crate::AppStore) -> StoreResult<()> {
    clear(c)?;
    app.with(crate::app_store::identities::clear_faces)
}

pub fn clear(c: &Connection) -> StoreResult<()> {
    c.pragma_update(None, "secure_delete", true)?;
    let ids: HashSet<_> = records(c, None)?.iter().map(|f| f.media_id).collect();
    c.execute("DELETE FROM face", [])?;
    c.execute("DELETE FROM person_centroid", [])?;
    c.execute("DELETE FROM person_not_same", [])?;
    c.execute("DELETE FROM person_suggestion", [])?;
    c.execute("DELETE FROM person", [])?;
    c.execute("DELETE FROM face_exclusion", [])?;
    c.execute("DELETE FROM analysis_version WHERE stage='faces'", [])?;
    refresh_media(c, &ids)
}

// ------------------------------------------------- collegamento con le identità

/// Marcatore in `settings_kv`: persone con nome già convertite in identità.
pub const IDENTITIES_MIGRATED: &str = "identities_migrated";

/// Migrazione una tantum: ogni persona con nome viene collegata all'identità
/// con lo stesso nome restituita da `ensure` (che la crea se manca). I
/// centroidi non diventano riferimenti. Restituisce le persone collegate.
pub fn migrate_to_identities(
    c: &Connection,
    mut ensure: impl FnMut(&str) -> StoreResult<String>,
) -> StoreResult<usize> {
    let done: Option<String> = c
        .query_row(
            "SELECT value FROM settings_kv WHERE key=?1",
            [IDENTITIES_MIGRATED],
            |r| r.get(0),
        )
        .optional()?;
    if done.is_some() {
        return Ok(0);
    }
    let mut q = c.prepare(
        "SELECT id,name FROM person WHERE name IS NOT NULL AND identity_uid IS NULL ORDER BY id",
    )?;
    let named = q
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    for (id, name) in &named {
        let uid = ensure(name)?;
        c.execute(
            "UPDATE person SET identity_uid=?2 WHERE id=?1",
            params![id, uid],
        )?;
    }
    c.execute(
        "INSERT INTO settings_kv(key,value) VALUES (?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![IDENTITIES_MIGRATED, now_ms().to_string()],
    )?;
    Ok(named.len())
}

/// Persona della libreria collegata all'identità, se esiste.
pub fn person_for_identity(c: &Connection, uid: &str) -> StoreResult<Option<i64>> {
    Ok(c.query_row(
        "SELECT id FROM person WHERE identity_uid=?1 ORDER BY id LIMIT 1",
        [uid],
        |r| r.get(0),
    )
    .optional()?)
}

/// Collega (o scollega, con `None`) una persona a un'identità. Il nome della
/// persona diventa quello dell'identità. Se un'altra persona era già collegata
/// alla stessa identità, le due persone vengono unite.
pub fn link_identity(
    c: &Connection,
    person: i64,
    identity: Option<(&str, &str)>,
) -> StoreResult<i64> {
    let Some((uid, name)) = identity else {
        if c.execute(
            "UPDATE person SET identity_uid=NULL,name=NULL WHERE id=?1",
            [person],
        )? == 0
        {
            return Err(StoreError::NotFound("persona".into()));
        }
        refresh_centroids(c)?;
        refresh_media(c, &person_media(c, person)?)?;
        return Ok(person);
    };
    if let Some(other) = person_for_identity(c, uid)?.filter(|o| *o != person) {
        merge(c, person, other)?;
        return Ok(other);
    }
    if c.execute(
        "UPDATE person SET identity_uid=?2,name=?3 WHERE id=?1",
        params![person, uid, name],
    )? == 0
    {
        return Err(StoreError::NotFound("persona".into()));
    }
    refresh_centroids(c)?;
    refresh_media(c, &person_media(c, person)?)?;
    Ok(person)
}

/// Persona collegata all'identità, creata se nella libreria non c'è ancora.
pub fn ensure_identity_person(c: &Connection, uid: &str, name: &str) -> StoreResult<i64> {
    if let Some(p) = person_for_identity(c, uid)? {
        return Ok(p);
    }
    let id = create_person(c, Some(name))?;
    c.execute(
        "UPDATE person SET identity_uid=?2 WHERE id=?1",
        params![id, uid],
    )?;
    Ok(id)
}

/// Riallinea i nomi delle persone collegate ai nomi delle identità
/// (`uid` → nome); le persone collegate a identità non più esistenti vengono
/// scollegate mantenendo il nome. Restituisce le persone modificate.
pub fn sync_identity_names(
    c: &Connection,
    names: &std::collections::HashMap<String, String>,
) -> StoreResult<usize> {
    let mut q = c.prepare(
        "SELECT id,identity_uid,COALESCE(name,'') FROM person WHERE identity_uid IS NOT NULL",
    )?;
    let rows = q
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut changed = 0;
    let mut media = HashSet::new();
    for (id, uid, name) in rows {
        match names.get(&uid) {
            Some(n) if *n == name => continue,
            Some(n) => {
                c.execute("UPDATE person SET name=?2 WHERE id=?1", params![id, n])?;
            }
            None => {
                c.execute("UPDATE person SET identity_uid=NULL WHERE id=?1", [id])?;
            }
        }
        changed += 1;
        media.extend(person_media(c, id)?);
    }
    if changed > 0 {
        refresh_media(c, &media)?;
    }
    Ok(changed)
}

/// Dopo l'unione di due identità: le persone collegate a `from` passano a `to`
/// (unendo le persone se `to` era già collegata in questa libreria).
pub fn relink_identity(c: &Connection, from: &str, to: &str, name: &str) -> StoreResult<()> {
    if let Some(p) = person_for_identity(c, from)? {
        link_identity(c, p, Some((to, name)))?;
    }
    Ok(())
}

/// Foto per identità collegata nella libreria aperta (`uid` → foto).
pub fn identity_photos(c: &Connection) -> StoreResult<std::collections::HashMap<String, i64>> {
    let mut q = c.prepare(
        "SELECT p.identity_uid,count(DISTINCT f.media_id) FROM person p JOIN face f ON f.person_id=p.id
         WHERE p.identity_uid IS NOT NULL GROUP BY p.identity_uid",
    )?;
    let rows = q
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}
