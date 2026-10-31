//! Repository: funzioni su `&Connection` (letture dal pool, scritture dal writer).

use crate::model::*;
use crate::{now_ms, StoreError, StoreResult};
use rusqlite::{params, Connection, OptionalExtension, Row};

// ---------------------------------------------------------------- sorgenti

pub mod source {
    use super::*;

    fn from_row(r: &Row<'_>) -> rusqlite::Result<Source> {
        let formats: String = r.get(3)?;
        let ignore: String = r.get(4)?;
        Ok(Source {
            id: r.get(0)?,
            path: r.get(1)?,
            recursive: r.get(2)?,
            include_formats: serde_json::from_str(&formats).unwrap_or_default(),
            ignore_patterns: serde_json::from_str(&ignore).unwrap_or_default(),
            watch: r.get(5)?,
            created_at: r.get(6)?,
        })
    }

    const COLS: &str = "id, path, recursive, include_formats, ignore_patterns, watch, created_at";

    pub fn insert(c: &Connection, s: &NewSource) -> StoreResult<i64> {
        c.execute(
            "INSERT INTO source(path, recursive, include_formats, ignore_patterns, watch, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                s.path,
                s.recursive,
                serde_json::to_string(&s.include_formats)?,
                serde_json::to_string(&s.ignore_patterns)?,
                s.watch,
                now_ms()
            ],
        )?;
        Ok(c.last_insert_rowid())
    }

    pub fn update(c: &Connection, s: &Source) -> StoreResult<()> {
        let n = c.execute(
            "UPDATE source SET recursive=?2, include_formats=?3, ignore_patterns=?4, watch=?5 WHERE id=?1",
            params![
                s.id,
                s.recursive,
                serde_json::to_string(&s.include_formats)?,
                serde_json::to_string(&s.ignore_patterns)?,
                s.watch
            ],
        )?;
        if n == 0 {
            return Err(StoreError::NotFound(format!("sorgente {}", s.id)));
        }
        Ok(())
    }

    /// Rimuove la sorgente e, a cascata, i suoi elementi dalla libreria.
    pub fn delete(c: &Connection, id: i64) -> StoreResult<()> {
        c.execute(
            "DELETE FROM media_fts WHERE rowid IN (SELECT id FROM media_item WHERE source_id=?1)",
            [id],
        )?;
        c.execute("DELETE FROM source WHERE id=?1", [id])?;
        Ok(())
    }

    pub fn get(c: &Connection, id: i64) -> StoreResult<Source> {
        Ok(c.query_row(
            &format!("SELECT {COLS} FROM source WHERE id=?1"),
            [id],
            from_row,
        )?)
    }

    pub fn list(c: &Connection) -> StoreResult<Vec<Source>> {
        let mut st = c.prepare(&format!("SELECT {COLS} FROM source ORDER BY path"))?;
        let rows = st.query_map([], from_row)?.collect::<Result<_, _>>()?;
        Ok(rows)
    }
}

// ---------------------------------------------------------------- media

pub mod media {
    use super::*;

    const COLS: &str = "id, kind, source_id, path, name, ext, size, mtime, quick_hash, content_hash, status, thumb, added_at, updated_at";

    fn from_row(r: &Row<'_>) -> rusqlite::Result<MediaItem> {
        Ok(MediaItem {
            id: r.get(0)?,
            kind: MediaKind::parse(&r.get::<_, String>(1)?),
            source_id: r.get(2)?,
            path: r.get(3)?,
            name: r.get(4)?,
            ext: r.get(5)?,
            size: r.get(6)?,
            mtime: r.get(7)?,
            quick_hash: r.get(8)?,
            content_hash: r.get(9)?,
            status: MediaStatus::parse(&r.get::<_, String>(10)?),
            thumb: r.get(11)?,
            added_at: r.get(12)?,
            updated_at: r.get(13)?,
        })
    }

    /// Inserisce o aggiorna per percorso; restituisce l'id.
    pub fn upsert(c: &Connection, m: &NewMedia) -> StoreResult<MediaId> {
        let now = now_ms();
        c.execute(
            "INSERT INTO media_item(kind, source_id, path, name, ext, size, mtime, quick_hash, content_hash, status, added_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'ok', ?10, ?10)
             ON CONFLICT(path) DO UPDATE SET
                kind=excluded.kind, source_id=excluded.source_id, name=excluded.name, ext=excluded.ext,
                size=excluded.size, mtime=excluded.mtime, quick_hash=excluded.quick_hash,
                content_hash=COALESCE(excluded.content_hash, media_item.content_hash),
                status='ok', updated_at=excluded.updated_at",
            params![
                m.kind.as_str(),
                m.source_id,
                m.path,
                m.name,
                m.ext.to_ascii_lowercase(),
                m.size,
                m.mtime,
                m.quick_hash,
                m.content_hash,
                now
            ],
        )?;
        Ok(
            c.query_row("SELECT id FROM media_item WHERE path=?1", [&m.path], |r| {
                r.get(0)
            })?,
        )
    }

    pub fn get(c: &Connection, id: MediaId) -> StoreResult<MediaItem> {
        Ok(c.query_row(
            &format!("SELECT {COLS} FROM media_item WHERE id=?1"),
            [id],
            from_row,
        )?)
    }

    pub fn get_by_path(c: &Connection, path: &str) -> StoreResult<Option<MediaItem>> {
        Ok(c.query_row(
            &format!("SELECT {COLS} FROM media_item WHERE path=?1"),
            [path],
            from_row,
        )
        .optional()?)
    }

    pub fn by_source(c: &Connection, source_id: i64) -> StoreResult<Vec<MediaItem>> {
        let mut st = c.prepare(&format!("SELECT {COLS} FROM media_item WHERE source_id=?1"))?;
        let rows = st
            .query_map([source_id], from_row)?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    /// Elementi mancanti con la stessa impronta rapida (candidati "spostati").
    pub fn missing_by_quick_hash(c: &Connection, quick_hash: &str) -> StoreResult<Vec<MediaItem>> {
        let mut st = c.prepare(&format!(
            "SELECT {COLS} FROM media_item WHERE quick_hash=?1 AND status='missing'"
        ))?;
        let rows = st
            .query_map([quick_hash], from_row)?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    /// Tutti gli elementi con la stessa impronta rapida.
    pub fn by_quick_hash(c: &Connection, quick_hash: &str) -> StoreResult<Vec<MediaItem>> {
        let mut st = c.prepare(&format!(
            "SELECT {COLS} FROM media_item WHERE quick_hash=?1"
        ))?;
        let rows = st
            .query_map([quick_hash], from_row)?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    /// Altri elementi con lo stesso contenuto (duplicati esatti).
    pub fn duplicates_of(c: &Connection, id: MediaId) -> StoreResult<Vec<MediaId>> {
        let mut st = c.prepare(
            "SELECT b.id FROM media_item a JOIN media_item b
               ON a.content_hash = b.content_hash AND a.id <> b.id
             WHERE a.id = ?1 AND a.content_hash IS NOT NULL ORDER BY b.id",
        )?;
        let rows = st
            .query_map([id], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    pub fn set_status(c: &Connection, id: MediaId, status: MediaStatus) -> StoreResult<()> {
        c.execute(
            "UPDATE media_item SET status=?2, updated_at=?3 WHERE id=?1",
            params![id, status.as_str(), now_ms()],
        )?;
        Ok(())
    }

    pub fn set_content_hash(c: &Connection, id: MediaId, hash: &str) -> StoreResult<()> {
        c.execute(
            "UPDATE media_item SET content_hash=?2 WHERE id=?1",
            params![id, hash],
        )?;
        Ok(())
    }

    pub fn set_thumb(c: &Connection, id: MediaId, thumb: &str) -> StoreResult<()> {
        c.execute(
            "UPDATE media_item SET thumb=?2 WHERE id=?1",
            params![id, thumb],
        )?;
        Ok(())
    }

    /// Ricollega un elemento a un nuovo percorso conservandone catalogazioni e cataloghi.
    pub fn relink(
        c: &Connection,
        id: MediaId,
        path: &str,
        name: &str,
        mtime: i64,
    ) -> StoreResult<()> {
        c.execute(
            "UPDATE media_item SET path=?2, name=?3, mtime=?4, status='ok', updated_at=?5 WHERE id=?1",
            params![id, path, name, mtime, now_ms()],
        )?;
        Ok(())
    }

    /// Rimuove dalla libreria (mai dal disco).
    pub fn delete(c: &Connection, ids: &[MediaId]) -> StoreResult<usize> {
        let mut del = c.prepare_cached("DELETE FROM media_item WHERE id=?1")?;
        let mut fts = c.prepare_cached("DELETE FROM media_fts WHERE rowid=?1")?;
        let mut n = 0;
        for id in ids {
            fts.execute([id])?;
            n += del.execute([id])?;
        }
        Ok(n)
    }

    pub fn count(c: &Connection) -> StoreResult<i64> {
        Ok(c.query_row("SELECT count(*) FROM media_item", [], |r| r.get(0))?)
    }
}

// ---------------------------------------------------------------- metadati

pub mod meta {
    use super::*;

    pub fn set_tech(c: &Connection, id: MediaId, t: &TechInfo) -> StoreResult<()> {
        c.execute(
            "INSERT OR REPLACE INTO media_tech(media_id, format, width, height, dpi_x, dpi_y, bit_depth, color_space, orientation)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![id, t.format, t.width, t.height, t.dpi_x, t.dpi_y, t.bit_depth, t.color_space, t.orientation],
        )?;
        Ok(())
    }

    pub fn tech(c: &Connection, id: MediaId) -> StoreResult<Option<TechInfo>> {
        Ok(c.query_row(
            "SELECT format, width, height, dpi_x, dpi_y, bit_depth, color_space, orientation
                 FROM media_tech WHERE media_id=?1",
            [id],
            |r| {
                Ok(TechInfo {
                    format: r.get(0)?,
                    width: r.get(1)?,
                    height: r.get(2)?,
                    dpi_x: r.get(3)?,
                    dpi_y: r.get(4)?,
                    bit_depth: r.get(5)?,
                    color_space: r.get(6)?,
                    orientation: r.get(7)?,
                })
            },
        )
        .optional()?)
    }

    pub fn set_dates(c: &Connection, id: MediaId, d: &MediaDates) -> StoreResult<()> {
        c.execute(
            "INSERT OR REPLACE INTO media_dates(media_id, captured_at, captured_source, modified_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![id, d.captured_at, d.captured_source.map(DateSource::as_str), d.modified_at],
        )?;
        Ok(())
    }

    pub fn dates(c: &Connection, id: MediaId) -> StoreResult<Option<MediaDates>> {
        Ok(c.query_row(
            "SELECT captured_at, captured_source, modified_at FROM media_dates WHERE media_id=?1",
            [id],
            |r| {
                let src: Option<String> = r.get(1)?;
                Ok(MediaDates {
                    captured_at: r.get(0)?,
                    captured_source: src.as_deref().and_then(DateSource::parse),
                    modified_at: r.get(2)?,
                })
            },
        )
        .optional()?)
    }

    pub fn set_exif(c: &Connection, id: MediaId, e: &ExifInfo) -> StoreResult<()> {
        c.execute(
            "INSERT OR REPLACE INTO exif(media_id, camera_make, camera_model, lens, exposure, f_number, iso, focal_length, gps_lat, gps_lon, raw)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                id, e.camera_make, e.camera_model, e.lens, e.exposure, e.f_number, e.iso,
                e.focal_length, e.gps_lat, e.gps_lon,
                e.raw.as_ref().map(|v| v.to_string())
            ],
        )?;
        Ok(())
    }

    pub fn exif(c: &Connection, id: MediaId) -> StoreResult<Option<ExifInfo>> {
        Ok(c
            .query_row(
                "SELECT camera_make, camera_model, lens, exposure, f_number, iso, focal_length, gps_lat, gps_lon, raw
                 FROM exif WHERE media_id=?1",
                [id],
                |r| {
                    let raw: Option<String> = r.get(9)?;
                    Ok(ExifInfo {
                        camera_make: r.get(0)?,
                        camera_model: r.get(1)?,
                        lens: r.get(2)?,
                        exposure: r.get(3)?,
                        f_number: r.get(4)?,
                        iso: r.get(5)?,
                        focal_length: r.get(6)?,
                        gps_lat: r.get(7)?,
                        gps_lon: r.get(8)?,
                        raw: raw.and_then(|s| serde_json::from_str(&s).ok()),
                    })
                },
            )
            .optional()?)
    }

    /// Imposta il luogo; una posizione manuale non viene sovrascritta da quelle automatiche.
    pub fn set_location(c: &Connection, id: MediaId, l: &Location) -> StoreResult<bool> {
        if l.source != LocationSource::Manual {
            let manual: Option<String> = c
                .query_row(
                    "SELECT source FROM location WHERE media_id=?1 AND source='manual'",
                    [id],
                    |r| r.get(0),
                )
                .optional()?;
            if manual.is_some() {
                return Ok(false);
            }
        }
        c.execute(
            "INSERT OR REPLACE INTO location(media_id, name, country, lat, lon, source) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, l.name, l.country, l.lat, l.lon, l.source.as_str()],
        )?;
        Ok(true)
    }

    pub fn location(c: &Connection, id: MediaId) -> StoreResult<Option<Location>> {
        Ok(c.query_row(
            "SELECT name, country, lat, lon, source FROM location WHERE media_id=?1",
            [id],
            |r| {
                Ok(Location {
                    name: r.get(0)?,
                    country: r.get(1)?,
                    lat: r.get(2)?,
                    lon: r.get(3)?,
                    source: LocationSource::parse(&r.get::<_, String>(4)?),
                })
            },
        )
        .optional()?)
    }
}

// ---------------------------------------------------------------- tag

pub mod tag {
    use super::*;

    /// Id del tag (creato se non esiste, confronto senza maiuscole).
    pub fn ensure(c: &Connection, name: &str) -> StoreResult<i64> {
        let name = name.trim();
        if name.is_empty() {
            return Err(StoreError::Invalid("nome tag vuoto".into()));
        }
        c.execute("INSERT OR IGNORE INTO tag(name) VALUES (?1)", [name])?;
        Ok(c.query_row("SELECT id FROM tag WHERE name=?1", [name], |r| r.get(0))?)
    }

    /// Aggiunge il tag; restituisce gli id che non lo avevano già.
    pub fn add(c: &Connection, tag_id: i64, ids: &[MediaId]) -> StoreResult<Vec<MediaId>> {
        let mut st =
            c.prepare_cached("INSERT OR IGNORE INTO media_tag(media_id, tag_id) VALUES (?1, ?2)")?;
        let mut added = Vec::new();
        for id in ids {
            if st.execute(params![id, tag_id])? > 0 {
                added.push(*id);
            }
        }
        Ok(added)
    }

    /// Rimuove il tag; restituisce gli id che lo avevano.
    pub fn remove(c: &Connection, tag_id: i64, ids: &[MediaId]) -> StoreResult<Vec<MediaId>> {
        let mut st = c.prepare_cached("DELETE FROM media_tag WHERE media_id=?1 AND tag_id=?2")?;
        let mut removed = Vec::new();
        for id in ids {
            if st.execute(params![id, tag_id])? > 0 {
                removed.push(*id);
            }
        }
        Ok(removed)
    }

    pub fn of_media(c: &Connection, id: MediaId) -> StoreResult<Vec<String>> {
        let mut st = c.prepare_cached(
            "SELECT t.name FROM media_tag mt JOIN tag t ON t.id=mt.tag_id WHERE mt.media_id=?1 ORDER BY t.name",
        )?;
        let rows = st
            .query_map([id], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    pub fn list(c: &Connection) -> StoreResult<Vec<(i64, String, i64)>> {
        let mut st = c.prepare(
            "SELECT t.id, t.name, count(mt.media_id) FROM tag t LEFT JOIN media_tag mt ON mt.tag_id=t.id
             GROUP BY t.id ORDER BY t.name",
        )?;
        let rows = st
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }
}

// ---------------------------------------------------------------- cataloghi

pub mod catalog {
    use super::*;

    fn from_row(r: &Row<'_>) -> rusqlite::Result<Catalog> {
        let kind: String = r.get(2)?;
        let rules: Option<String> = r.get(3)?;
        Ok(Catalog {
            id: r.get(0)?,
            name: r.get(1)?,
            kind: if kind == "smart" {
                CatalogKind::Smart
            } else {
                CatalogKind::Manual
            },
            rules: rules.and_then(|s| serde_json::from_str(&s).ok()),
            created_at: r.get(4)?,
        })
    }

    pub fn create(
        c: &Connection,
        name: &str,
        kind: CatalogKind,
        rules: Option<&serde_json::Value>,
    ) -> StoreResult<i64> {
        let name = name.trim();
        if name.is_empty() {
            return Err(StoreError::Invalid("nome catalogo vuoto".into()));
        }
        c.execute(
            "INSERT INTO catalog(name, kind, rules, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![name, kind.as_str(), rules.map(|r| r.to_string()), now_ms()],
        )?;
        Ok(c.last_insert_rowid())
    }

    pub fn rename(c: &Connection, id: i64, name: &str) -> StoreResult<()> {
        let name = name.trim();
        if name.is_empty() {
            return Err(StoreError::Invalid("nome catalogo vuoto".into()));
        }
        if c.execute("UPDATE catalog SET name=?2 WHERE id=?1", params![id, name])? == 0 {
            return Err(StoreError::NotFound(format!("catalogo {id}")));
        }
        Ok(())
    }

    pub fn set_rules(c: &Connection, id: i64, rules: &serde_json::Value) -> StoreResult<()> {
        c.execute(
            "UPDATE catalog SET rules=?2 WHERE id=?1",
            params![id, rules.to_string()],
        )?;
        Ok(())
    }

    /// Elimina il catalogo; i file restano in libreria.
    pub fn delete(c: &Connection, id: i64) -> StoreResult<()> {
        c.execute("DELETE FROM catalog WHERE id=?1", [id])?;
        Ok(())
    }

    pub fn get(c: &Connection, id: i64) -> StoreResult<Catalog> {
        Ok(c.query_row(
            "SELECT id, name, kind, rules, created_at FROM catalog WHERE id=?1",
            [id],
            from_row,
        )?)
    }

    pub fn list(c: &Connection) -> StoreResult<Vec<Catalog>> {
        let mut st = c.prepare(
            "SELECT id, name, kind, rules, created_at FROM catalog ORDER BY name COLLATE NOCASE",
        )?;
        let rows = st.query_map([], from_row)?.collect::<Result<_, _>>()?;
        Ok(rows)
    }

    /// Aggiunge elementi a un catalogo manuale; restituisce quelli effettivamente aggiunti.
    pub fn add_items(c: &Connection, id: i64, ids: &[MediaId]) -> StoreResult<Vec<MediaId>> {
        let mut st = c.prepare_cached(
            "INSERT OR IGNORE INTO catalog_item(catalog_id, media_id) VALUES (?1, ?2)",
        )?;
        let mut added = Vec::new();
        for m in ids {
            if st.execute(params![id, m])? > 0 {
                added.push(*m);
            }
        }
        Ok(added)
    }

    pub fn remove_items(c: &Connection, id: i64, ids: &[MediaId]) -> StoreResult<Vec<MediaId>> {
        let mut st =
            c.prepare_cached("DELETE FROM catalog_item WHERE catalog_id=?1 AND media_id=?2")?;
        let mut removed = Vec::new();
        for m in ids {
            if st.execute(params![id, m])? > 0 {
                removed.push(*m);
            }
        }
        Ok(removed)
    }

    pub fn manual_items(c: &Connection, id: i64) -> StoreResult<Vec<MediaId>> {
        let mut st =
            c.prepare("SELECT media_id FROM catalog_item WHERE catalog_id=?1 ORDER BY media_id")?;
        let rows = st
            .query_map([id], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    pub fn catalogs_of(c: &Connection, media: MediaId) -> StoreResult<Vec<String>> {
        let mut st = c.prepare_cached(
            "SELECT k.name FROM catalog_item ci JOIN catalog k ON k.id=ci.catalog_id WHERE ci.media_id=?1 ORDER BY k.name",
        )?;
        let rows = st
            .query_map([media], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }
}

// ---------------------------------------------------------------- ricerca testuale

pub mod fts {
    use super::*;

    /// Ricalcola la riga di ricerca di un elemento da nome, percorso, luogo, persone e tag.
    pub fn refresh(c: &Connection, id: MediaId) -> StoreResult<()> {
        let row: Option<(String, String)> = c
            .query_row("SELECT name, path FROM media_item WHERE id=?1", [id], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .optional()?;
        c.execute("DELETE FROM media_fts WHERE rowid=?1", [id])?;
        let Some((name, path)) = row else {
            return Ok(());
        };
        let place: Option<String> = c
            .query_row("SELECT name FROM location WHERE media_id=?1", [id], |r| {
                r.get(0)
            })
            .optional()?
            .flatten();
        let people: String = c.query_row(
            "SELECT coalesce(group_concat(DISTINCT p.name), '') FROM face f JOIN person p ON p.id=f.person_id
             WHERE f.media_id=?1 AND p.name IS NOT NULL",
            [id],
            |r| r.get(0),
        )?;
        let tags = tag::of_media(c, id)?.join(" ");
        c.execute(
            "INSERT INTO media_fts(rowid, name, path, place, people, tags) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, name, path, place.unwrap_or_default(), people, tags],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrate;

    fn db() -> Connection {
        let mut c = Connection::open_in_memory().unwrap();
        c.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrate::migrate(&mut c, None).unwrap();
        c
    }

    fn new_media(path: &str) -> NewMedia {
        NewMedia {
            path: path.into(),
            name: path.rsplit('\\').next().unwrap().into(),
            ext: "JPG".into(),
            size: 1000,
            mtime: 1,
            ..Default::default()
        }
    }

    #[test]
    fn source_crud_and_cascade() {
        let c = db();
        let sid = source::insert(
            &c,
            &NewSource {
                path: "D:\\Foto".into(),
                recursive: true,
                include_formats: vec!["jpg".into()],
                ignore_patterns: vec!["**/.thumbs/**".into()],
                watch: false,
            },
        )
        .unwrap();
        let mut s = source::get(&c, sid).unwrap();
        assert_eq!(s.include_formats, vec!["jpg"]);
        s.watch = true;
        source::update(&c, &s).unwrap();
        assert!(source::list(&c).unwrap()[0].watch);

        let mut m = new_media("D:\\Foto\\a.jpg");
        m.source_id = Some(sid);
        media::upsert(&c, &m).unwrap();
        source::delete(&c, sid).unwrap();
        assert_eq!(media::count(&c).unwrap(), 0);
    }

    #[test]
    fn media_upsert_is_idempotent_by_path() {
        let c = db();
        let id1 = media::upsert(&c, &new_media("D:\\a.jpg")).unwrap();
        let mut m = new_media("D:\\a.jpg");
        m.size = 2000;
        let id2 = media::upsert(&c, &m).unwrap();
        assert_eq!(id1, id2);
        let item = media::get(&c, id1).unwrap();
        assert_eq!(item.size, 2000);
        assert_eq!(item.ext, "jpg");
        assert_eq!(item.status, MediaStatus::Ok);
    }

    #[test]
    fn duplicates_and_relink() {
        let c = db();
        let mut a = new_media("D:\\a.jpg");
        a.content_hash = Some("h1".into());
        a.quick_hash = Some("q1".into());
        let mut b = new_media("D:\\b.jpg");
        b.content_hash = Some("h1".into());
        let ia = media::upsert(&c, &a).unwrap();
        let ib = media::upsert(&c, &b).unwrap();
        assert_eq!(media::duplicates_of(&c, ia).unwrap(), vec![ib]);

        media::set_status(&c, ia, MediaStatus::Missing).unwrap();
        assert_eq!(media::missing_by_quick_hash(&c, "q1").unwrap()[0].id, ia);
        media::relink(&c, ia, "D:\\nuova\\a.jpg", "a.jpg", 5).unwrap();
        let moved = media::get(&c, ia).unwrap();
        assert_eq!(moved.path, "D:\\nuova\\a.jpg");
        assert_eq!(moved.status, MediaStatus::Ok);
    }

    #[test]
    fn metadata_roundtrip() {
        let c = db();
        let id = media::upsert(&c, &new_media("D:\\a.jpg")).unwrap();
        let t = TechInfo {
            format: Some("JPEG".into()),
            width: Some(6000),
            height: Some(4000),
            dpi_x: Some(300.0),
            dpi_y: Some(300.0),
            bit_depth: Some(8),
            ..Default::default()
        };
        meta::set_tech(&c, id, &t).unwrap();
        assert_eq!(meta::tech(&c, id).unwrap().unwrap(), t);

        let d = MediaDates {
            captured_at: Some("2023-07-14 10:22:01".into()),
            captured_source: Some(DateSource::ExifOriginal),
            modified_at: Some("2023-07-15 08:00:00".into()),
        };
        meta::set_dates(&c, id, &d).unwrap();
        assert_eq!(meta::dates(&c, id).unwrap().unwrap(), d);

        let e = ExifInfo {
            camera_make: Some("Canon".into()),
            iso: Some(200),
            gps_lat: Some(43.77),
            gps_lon: Some(11.25),
            raw: Some(serde_json::json!({"Make": "Canon"})),
            ..Default::default()
        };
        meta::set_exif(&c, id, &e).unwrap();
        assert_eq!(meta::exif(&c, id).unwrap().unwrap(), e);
    }

    #[test]
    fn manual_location_is_not_overwritten() {
        let c = db();
        let id = media::upsert(&c, &new_media("D:\\a.jpg")).unwrap();
        let manual = Location {
            name: Some("Siena".into()),
            country: None,
            lat: None,
            lon: None,
            source: LocationSource::Manual,
        };
        assert!(meta::set_location(&c, id, &manual).unwrap());
        let auto = Location {
            name: Some("Firenze".into()),
            source: LocationSource::Gps,
            ..manual.clone()
        };
        assert!(!meta::set_location(&c, id, &auto).unwrap());
        assert_eq!(
            meta::location(&c, id).unwrap().unwrap().name.as_deref(),
            Some("Siena")
        );
    }

    #[test]
    fn tags_and_catalogs() {
        let c = db();
        let ids: Vec<_> = (0..3)
            .map(|i| media::upsert(&c, &new_media(&format!("D:\\{i}.jpg"))).unwrap())
            .collect();
        let t = tag::ensure(&c, "Mare").unwrap();
        assert_eq!(tag::ensure(&c, "mare").unwrap(), t);
        assert_eq!(tag::add(&c, t, &ids[..2]).unwrap(), ids[..2].to_vec());
        assert_eq!(tag::add(&c, t, &ids).unwrap(), vec![ids[2]]);
        assert_eq!(tag::of_media(&c, ids[0]).unwrap(), vec!["Mare"]);
        assert_eq!(tag::remove(&c, t, &ids[..1]).unwrap(), vec![ids[0]]);

        let k = catalog::create(&c, "Viaggio Toscana", CatalogKind::Manual, None).unwrap();
        catalog::add_items(&c, k, &ids).unwrap();
        assert_eq!(catalog::manual_items(&c, k).unwrap().len(), 3);
        let k2 = catalog::create(&c, "Altro", CatalogKind::Manual, None).unwrap();
        catalog::add_items(&c, k2, &ids[..1]).unwrap();
        catalog::rename(&c, k, "Toscana").unwrap();
        catalog::delete(&c, k).unwrap();
        // I file restano in libreria e negli altri cataloghi.
        assert_eq!(media::count(&c).unwrap(), 3);
        assert_eq!(catalog::catalogs_of(&c, ids[0]).unwrap(), vec!["Altro"]);
        assert!(catalog::create(&c, "  ", CatalogKind::Manual, None).is_err());
    }

    #[test]
    fn fts_finds_by_place_and_tag() {
        let c = db();
        let id = media::upsert(&c, &new_media("D:\\Foto\\IMG_4821.JPG")).unwrap();
        meta::set_location(
            &c,
            id,
            &Location {
                name: Some("Città di Siena".into()),
                country: None,
                lat: None,
                lon: None,
                source: LocationSource::Directory,
            },
        )
        .unwrap();
        let t = tag::ensure(&c, "Palio").unwrap();
        tag::add(&c, t, &[id]).unwrap();
        fts::refresh(&c, id).unwrap();
        for q in ["siena", "citta", "palio", "IMG_4821*"] {
            let n: i64 = c
                .query_row(
                    "SELECT count(*) FROM media_fts WHERE media_fts MATCH ?1",
                    [q],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(n, 1, "{q}");
        }
    }
}
