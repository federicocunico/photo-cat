//! Volti e correzioni persistenti. Nessuna operazione modifica i file originali.
use crate::{analysis, now_ms, repo, StoreError, StoreResult};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone)]
pub struct DetectedFace {
    pub bbox: [f32; 4],
    pub quality: f32,
    pub embedding: Vec<f32>,
}
#[derive(Debug, Clone)]
pub struct FaceRecord {
    pub id: i64,
    pub media_id: i64,
    pub bbox: [f32; 4],
    pub quality: f32,
    pub embedding: Option<Vec<f32>>,
    pub person_id: Option<i64>,
    pub manual: bool,
    pub rejected: Vec<i64>,
    pub model: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct FaceView {
    pub id: i64,
    pub media_id: i64,
    pub bbox: [f32; 4],
    pub quality: f32,
    pub thumb: Option<String>,
    pub person_id: Option<i64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct PersonView {
    pub id: i64,
    pub name: Option<String>,
    pub photos: i64,
    pub faces: i64,
    pub cover: Option<FaceView>,
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
    let mut q=c.prepare("SELECT id,media_id,bbox,quality,embedding,person_id,assigned_by,rejected,COALESCE(model_ver,'') FROM face WHERE (?1 IS NULL OR media_id=?1) ORDER BY id")?;
    let rows = q
        .query_map([media], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, f32>(3)?,
                r.get::<_, Option<Vec<u8>>>(4)?,
                r.get::<_, Option<i64>>(5)?,
                r.get::<_, Option<String>>(6)?,
                r.get::<_, String>(7)?,
                r.get::<_, String>(8)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    rows.into_iter()
        .map(
            |(id, media_id, bbox, quality, embed, person_id, assigned, reject, model)| {
                Ok(FaceRecord {
                    id,
                    media_id,
                    bbox: serde_json::from_str(&bbox)?,
                    quality,
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
pub fn views(c: &Connection, person: Option<i64>) -> StoreResult<Vec<FaceView>> {
    let mut q=c.prepare("SELECT f.id,f.media_id,f.bbox,f.quality,m.thumb,f.person_id FROM face f JOIN media_item m ON m.id=f.media_id WHERE (?1 IS NULL OR f.person_id=?1) ORDER BY f.quality DESC,f.id")?;
    let rows = q
        .query_map([person], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get::<_, String>(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    rows.into_iter()
        .map(|(id, media_id, bbox, quality, thumb, person_id)| {
            Ok(FaceView {
                id,
                media_id,
                bbox: serde_json::from_str(&bbox)?,
                quality,
                thumb,
                person_id,
            })
        })
        .collect()
}
pub fn list(c: &Connection) -> StoreResult<Vec<PersonView>> {
    let mut q=c.prepare("SELECT p.id,p.name,count(DISTINCT f.media_id),count(f.id) FROM person p JOIN face f ON f.person_id=p.id GROUP BY p.id ORDER BY p.name IS NULL,p.name,p.id")?;
    let rows = q
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    rows.into_iter()
        .map(|(id, name, photos, faces)| {
            Ok(PersonView {
                id,
                name,
                photos,
                faces,
                cover: views(c, Some(id))?.into_iter().next(),
            })
        })
        .collect()
}
pub fn refresh_centroids(c: &Connection) -> StoreResult<()> {
    let mut groups: BTreeMap<(i64, String), (Vec<f32>, usize)> = BTreeMap::new();
    for f in records(c, None)? {
        if let (Some(person), Some(v)) = (f.person_id, f.embedding) {
            let (sum, n) = groups
                .entry((person, f.model))
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
pub fn named_match(
    c: &Connection,
    v: &[f32],
    model: &str,
    rejected: &[i64],
    threshold: f32,
) -> StoreResult<Option<i64>> {
    let mut q=c.prepare("SELECT p.id,pc.vector FROM person p JOIN person_centroid pc ON pc.person_id=p.id WHERE p.name IS NOT NULL AND pc.model_ver=?1")?;
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
pub fn save_faces(
    c: &Connection,
    media: i64,
    detected: &[DetectedFace],
    embedding_model: &str,
    analysis_model: &str,
    threshold: f32,
) -> StoreResult<()> {
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
        let prior = old
            .iter()
            .filter(|x| !used.contains(&x.id) && iou(x.bbox, f.bbox) >= 0.5)
            .max_by(|a, b| iou(a.bbox, f.bbox).total_cmp(&iou(b.bbox, f.bbox)));
        let id = if let Some(prior) = prior {
            used.insert(prior.id);
            c.execute(
                "UPDATE face SET bbox=?2,quality=?3,embedding=?4,model_ver=?5 WHERE id=?1",
                params![
                    prior.id,
                    serde_json::to_string(&f.bbox)?,
                    f.quality,
                    bytes(&f.embedding),
                    embedding_model
                ],
            )?;
            prior.id
        } else {
            let id = next_id(c, "face")?;
            c.execute("INSERT INTO face(id,media_id,bbox,quality,embedding,model_ver) VALUES (?1,?2,?3,?4,?5,?6)",params![id,media,serde_json::to_string(&f.bbox)?,f.quality,bytes(&f.embedding),embedding_model])?;
            id
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
            let person = named_match(c, &f.embedding, embedding_model, rejects, threshold)?;
            c.execute(
                "UPDATE face SET person_id=?2,assigned_by='auto' WHERE id=?1",
                params![id, person],
            )?;
        }
    }
    for f in old.iter().filter(|f| !used.contains(&f.id)) {
        let named = if let Some(p) = f.person_id {
            c.query_row(
                "SELECT name IS NOT NULL FROM person WHERE id=?1",
                [p],
                |r| r.get::<_, bool>(0),
            )
            .optional()?
            .unwrap_or(false)
        } else {
            false
        };
        if f.manual || named {
            c.execute("UPDATE face SET embedding=NULL WHERE id=?1", [f.id])?;
        } else {
            c.execute("DELETE FROM face WHERE id=?1", [f.id])?;
        }
    }
    analysis::version(c, media, "faces", analysis_model, None)?;
    repo::fts::refresh(c, media)?;
    Ok(())
}
pub fn named_ids(c: &Connection) -> StoreResult<HashSet<i64>> {
    let mut q = c.prepare("SELECT id FROM person WHERE name IS NOT NULL")?;
    let ids = q.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
    Ok(ids)
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
    c.execute(
        "UPDATE face SET person_id=?2,assigned_by='manual' WHERE person_id=?1",
        params![from, to],
    )?;
    for f in records(c, None)? {
        let mut rejected: Vec<_> = f
            .rejected
            .into_iter()
            .map(|p| if p == from { to } else { p })
            .collect();
        rejected.sort();
        rejected.dedup();
        if f.person_id == Some(to) {
            rejected.retain(|p| *p != to);
        }
        c.execute(
            "UPDATE face SET rejected=?2 WHERE id=?1",
            params![f.id, serde_json::to_string(&rejected)?],
        )?;
    }
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
pub fn clear(c: &Connection) -> StoreResult<()> {
    c.pragma_update(None, "secure_delete", true)?;
    let ids: HashSet<_> = records(c, None)?.iter().map(|f| f.media_id).collect();
    c.execute("DELETE FROM face", [])?;
    c.execute("DELETE FROM person_centroid", [])?;
    c.execute("DELETE FROM person", [])?;
    c.execute("DELETE FROM face_exclusion", [])?;
    c.execute("DELETE FROM analysis_version WHERE stage='faces'", [])?;
    refresh_media(c, &ids)
}
