//! Data di scatto impostata a mano: scenari di `capture-timeline`.
use pc_store::bulk;
use pc_store::model::*;
use pc_store::repo;
use pc_store::sequence::{self, SequenceScope};
use pc_store::timeline::{self, GroupClock};
use pc_store::{Store, StoreOptions};
use rusqlite::Connection;

const UTC: &dyn Fn(chrono::NaiveDateTime) -> i32 = &|_| 0;

fn store(dir: &std::path::Path) -> Store {
    Store::open(&dir.join("catalog.db"), StoreOptions::default()).unwrap()
}

fn add(c: &Connection, name: &str, at: &str, src: DateSource) -> i64 {
    let id = repo::media::upsert(
        c,
        &NewMedia {
            path: format!("D:\\{name}"),
            name: name.into(),
            ext: "jpg".into(),
            ..Default::default()
        },
    )
    .unwrap();
    repo::meta::set_dates_with(
        c,
        id,
        &MediaDates {
            captured_at: Some(at.into()),
            captured_source: Some(src),
            ..Default::default()
        },
        UTC,
    )
    .unwrap();
    id
}

fn instant(c: &Connection, id: i64) -> Option<i64> {
    c.query_row(
        "SELECT instant_utc FROM media_dates WHERE media_id = ?1",
        [id],
        |r| r.get(0),
    )
    .unwrap()
}

fn day(s: &str) -> i64 {
    timeline::parse_local(s)
        .unwrap()
        .and_utc()
        .timestamp_millis()
}

#[test]
fn june_1984_on_a_scan_with_exif_2019() {
    let dir = tempfile::tempdir().unwrap();
    let s = store(dir.path());
    s.write(|c| {
        let id = add(
            c,
            "scan.jpg",
            "2019-03-02 10:00:00",
            DateSource::ExifOriginal,
        );
        bulk::set_manual_date_with(c, &[id], "1984-06", DatePrecision::Month, UTC)?;
        let d = repo::meta::dates(c, id)?.unwrap();
        assert_eq!(d.captured_source, Some(DateSource::Manual));
        assert_eq!(d.manual_at.as_deref(), Some("1984-06-01 00:00:00"));
        assert_eq!(d.manual_precision, Some(DatePrecision::Month));
        assert_eq!(d.captured_at.as_deref(), Some("2019-03-02 10:00:00"));
        assert_eq!(instant(c, id), Some(day("1984-06-01 00:00:00")));
        Ok(())
    })
    .unwrap();
}

#[test]
fn year_only_sorts_at_jan_1st_before_exif_photo_of_same_year() {
    let dir = tempfile::tempdir().unwrap();
    let s = store(dir.path());
    s.write(|c| {
        let exif = add(c, "a.jpg", "1984-06-10 14:00:00", DateSource::ExifOriginal);
        let scan = add(c, "b.jpg", "2020-01-01 09:00:00", DateSource::ExifOriginal);
        bulk::set_manual_date_with(c, &[scan], "1984", DatePrecision::Year, UTC)?;
        assert_eq!(instant(c, scan), Some(day("1984-01-01 00:00:00")));
        let order = sequence::ordered_ids(c, &SequenceScope::Library)?;
        assert_eq!(order, vec![scan, exif]);
        Ok(())
    })
    .unwrap();
}

#[test]
fn restore_returns_to_file_date_and_source() {
    let dir = tempfile::tempdir().unwrap();
    let s = store(dir.path());
    s.write(|c| {
        let id = add(c, "a.jpg", "2019-03-02 10:00:00", DateSource::ExifOriginal);
        let before = instant(c, id);
        bulk::set_manual_date_with(c, &[id], "1962", DatePrecision::Year, UTC)?;
        assert_ne!(instant(c, id), before);
        bulk::clear_manual_date_with(c, &[id], UTC)?;
        let d = repo::meta::dates(c, id)?.unwrap();
        assert_eq!(d.captured_source, Some(DateSource::ExifOriginal));
        assert_eq!(d.manual_at, None);
        assert_eq!(d.manual_precision, None);
        assert_eq!(instant(c, id), before);
        Ok(())
    })
    .unwrap();
}

#[test]
fn file_system_photo_with_manual_date_leaves_undated_area() {
    let dir = tempfile::tempdir().unwrap();
    let s = store(dir.path());
    s.write(|c| {
        let id = add(c, "a.jpg", "2024-01-01 00:00:00", DateSource::FileSystem);
        let other = add(c, "b.jpg", "2024-01-02 00:00:00", DateSource::FileSystem);
        let ids = |c: &Connection| -> Vec<i64> {
            sequence::undated(c, &SequenceScope::Library)
                .unwrap()
                .into_iter()
                .map(|u| u.id)
                .collect()
        };
        assert_eq!(ids(c), vec![id, other]);
        bulk::set_manual_date_with(c, &[id], "1990-05-17", DatePrecision::Day, UTC)?;
        assert_eq!(ids(c), vec![other]);
        let page = sequence::page(c, &SequenceScope::Library, 30, &[], None, 100)?;
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].id, id);
        bulk::clear_manual_date_with(c, &[id], UTC)?;
        assert_eq!(ids(c), vec![id, other]);
        Ok(())
    })
    .unwrap();
}

#[test]
fn group_shift_does_not_touch_manual_date() {
    let dir = tempfile::tempdir().unwrap();
    let s = store(dir.path());
    s.write(|c| {
        let manual = add(c, "a.jpg", "2019-03-02 10:00:00", DateSource::ExifOriginal);
        let plain = add(c, "b.jpg", "2019-03-02 11:00:00", DateSource::ExifOriginal);
        bulk::set_manual_date_with(c, &[manual], "1984-06-10 12:30", DatePrecision::Day, UTC)?;
        let (_, group) = timeline::instant_of(c, manual)?;
        let m0 = instant(c, manual);
        let p0 = instant(c, plain).unwrap();
        // Solo scostamento (percorso veloce).
        let clock = GroupClock {
            shift_s: 3600,
            ..Default::default()
        };
        timeline::set_clock(c, group, &clock, UTC)?;
        assert_eq!(instant(c, manual), m0);
        assert_eq!(instant(c, plain), Some(p0 + 3_600_000));
        // Cambio di fuso (ricalcolo completo del gruppo).
        let clock = GroupClock {
            display_offset_min: Some(120),
            shift_s: 3600,
            ..Default::default()
        };
        timeline::set_clock(c, group, &clock, UTC)?;
        assert_eq!(
            instant(c, manual),
            Some(day("1984-06-10 12:30:00") - 120 * 60_000)
        );
        Ok(())
    })
    .unwrap();
}

#[test]
fn undo_restores_previous_values_per_id() {
    let dir = tempfile::tempdir().unwrap();
    let s = store(dir.path());
    s.write(|c| {
        let a = add(c, "a.jpg", "2019-03-02 10:00:00", DateSource::ExifOriginal);
        let b = add(c, "b.jpg", "2019-04-02 10:00:00", DateSource::ExifOriginal);
        let (a0, b0) = (local(c, a), local(c, b));
        bulk::set_manual_date_with(c, &[a], "1970-02", DatePrecision::Month, UTC)?;
        bulk::set_manual_date_with(c, &[a, b], "1962", DatePrecision::Year, UTC)?;
        bulk::undo_last(c)?;
        let da = repo::meta::dates(c, a)?.unwrap();
        assert_eq!(da.manual_at.as_deref(), Some("1970-02-01 00:00:00"));
        assert_eq!(da.manual_precision, Some(DatePrecision::Month));
        assert_eq!(repo::meta::dates(c, b)?.unwrap().manual_at, None);
        bulk::undo_last(c)?;
        assert_eq!(repo::meta::dates(c, a)?.unwrap().manual_at, None);
        assert_eq!((local(c, a), local(c, b)), (a0, b0));
        // Annulla anche il ripristino.
        bulk::set_manual_date_with(c, &[a], "1970", DatePrecision::Year, UTC)?;
        bulk::clear_manual_date_with(c, &[a], UTC)?;
        bulk::undo_last(c)?;
        assert_eq!(
            repo::meta::dates(c, a)?.unwrap().manual_at.as_deref(),
            Some("1970-01-01 00:00:00")
        );
        Ok(())
    })
    .unwrap();
}

#[test]
fn invalid_input_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let s = store(dir.path());
    s.write(|c| {
        let id = add(c, "a.jpg", "2019-03-02 10:00:00", DateSource::ExifOriginal);
        for (v, p) in [
            ("1700", DatePrecision::Year),
            ("9999", DatePrecision::Year),
            ("1984-13", DatePrecision::Month),
            ("1984-00", DatePrecision::Month),
            ("1984-02-30", DatePrecision::Day),
            ("1984-02", DatePrecision::Day),
            ("1984-06", DatePrecision::Year),
            ("1984-06-01 10:00", DatePrecision::Month),
            ("abcd", DatePrecision::Year),
        ] {
            assert!(
                matches!(
                    bulk::set_manual_date_with(c, &[id], v, p, UTC),
                    Err(pc_store::StoreError::Invalid(_))
                ),
                "{v}"
            );
        }
        assert_eq!(repo::meta::dates(c, id)?.unwrap().manual_at, None);
        Ok(())
    })
    .unwrap();
}

/// Ora locale mostrata (indipendente dal fuso di sistema usato dall'annulla).
fn local(c: &Connection, id: i64) -> Option<String> {
    let (utc, off): (i64, i32) = c
        .query_row(
            "SELECT instant_utc, local_offset_min FROM media_dates WHERE media_id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    timeline::local_text(utc, off)
}
