//! Dati completi di un elemento per il pannello di ispezione.

use crate::model::*;
use crate::repo::{catalog, media, meta, tag};
use crate::StoreResult;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct LabelView {
    pub group: String,
    pub value: String,
    pub score: f64,
    pub origin: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct MediaDetail {
    pub item: MediaItem,
    pub tech: Option<TechInfo>,
    pub dates: Option<MediaDates>,
    pub exif: Option<ExifInfo>,
    pub location: Option<Location>,
    pub labels: Vec<LabelView>,
    pub people_count: Option<i64>,
    pub people: Vec<String>,
    pub tags: Vec<String>,
    pub catalogs: Vec<String>,
    pub duplicates: Vec<MediaId>,
}

pub fn media_detail(c: &Connection, id: MediaId) -> StoreResult<MediaDetail> {
    let item = media::get(c, id)?;
    let mut st = c.prepare_cached(
        "SELECT grp, value, score, origin FROM label WHERE media_id = ?1 ORDER BY grp, score DESC",
    )?;
    let labels = st
        .query_map([id], |r| {
            Ok(LabelView {
                group: r.get(0)?,
                value: r.get(1)?,
                score: r.get(2)?,
                origin: r.get(3)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    let people_count = c
        .query_row(
            "SELECT count FROM media_people WHERE media_id = ?1",
            [id],
            |r| r.get(0),
        )
        .ok();
    let mut st = c.prepare_cached(
        "SELECT DISTINCT p.name FROM face f JOIN person p ON p.id = f.person_id
         WHERE f.media_id = ?1 AND p.name IS NOT NULL ORDER BY p.name",
    )?;
    let people = st
        .query_map(params![id], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    Ok(MediaDetail {
        tech: meta::tech(c, id)?,
        dates: meta::dates(c, id)?,
        exif: meta::exif(c, id)?.map(|mut e| {
            e.raw = None; // il JSON completo resta disponibile ma non serve all'ispettore
            e
        }),
        location: meta::location(c, id)?,
        labels,
        people_count,
        people,
        tags: tag::of_media(c, id)?,
        catalogs: catalog::catalogs_of(c, id)?,
        duplicates: media::duplicates_of(c, id)?,
        item,
    })
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SourceSummary {
    pub source: Source,
    pub items: i64,
    pub missing: i64,
}

pub fn source_summaries(c: &Connection) -> StoreResult<Vec<SourceSummary>> {
    let sources = crate::repo::source::list(c)?;
    let mut st = c.prepare_cached(
        "SELECT count(*), coalesce(sum(status = 'missing'), 0) FROM media_item WHERE source_id = ?1",
    )?;
    sources
        .into_iter()
        .map(|source| {
            let (items, missing) = st.query_row([source.id], |r| Ok((r.get(0)?, r.get(1)?)))?;
            Ok(SourceSummary {
                source,
                items,
                missing,
            })
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct CatalogSummary {
    pub catalog: Catalog,
    pub items: i64,
}

pub fn catalog_summaries(c: &Connection) -> StoreResult<Vec<CatalogSummary>> {
    catalog::list(c)?
        .into_iter()
        .map(|cat| {
            let f = crate::query::LibraryFilter {
                catalog_id: Some(cat.id),
                ..Default::default()
            };
            let items = crate::query::count(c, &f).unwrap_or(0);
            Ok(CatalogSummary {
                catalog: cat,
                items,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrate;

    #[test]
    fn detail_and_summaries() {
        let mut c = Connection::open_in_memory().unwrap();
        migrate::migrate(&mut c, None).unwrap();
        let sid = crate::repo::source::insert(
            &c,
            &NewSource {
                path: "D:\\Foto".into(),
                recursive: true,
                ..Default::default()
            },
        )
        .unwrap();
        let id = media::upsert(
            &c,
            &NewMedia {
                source_id: Some(sid),
                path: "D:\\Foto\\a.jpg".into(),
                name: "a.jpg".into(),
                ext: "jpg".into(),
                ..Default::default()
            },
        )
        .unwrap();
        c.execute("INSERT INTO label(media_id, grp, value, score, origin) VALUES (?1, 'subject', 'paesaggio', 0.82, 'auto')", [id]).unwrap();
        let t = tag::ensure(&c, "tramonto").unwrap();
        tag::add(&c, t, &[id]).unwrap();
        let d = media_detail(&c, id).unwrap();
        assert_eq!(d.labels[0].value, "paesaggio");
        assert_eq!(d.tags, vec!["tramonto"]);
        assert_eq!(d.people_count, None);

        let s = source_summaries(&c).unwrap();
        assert_eq!((s[0].items, s[0].missing), (1, 0));
        let k = catalog::create(&c, "Viaggio", CatalogKind::Manual, None).unwrap();
        catalog::add_items(&c, k, &[id]).unwrap();
        assert_eq!(catalog_summaries(&c).unwrap()[0].items, 1);
    }
}
