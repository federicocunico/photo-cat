//! Correzioni orarie dei gruppi di sorgente (task 5.3).
use pc_store::model::{DateSource, MediaDates, NewMedia};
use pc_store::timeline::{self, GroupClock};
use pc_store::{bulk, migrate, repo};
use rusqlite::Connection;
use std::time::Instant;

fn sys(_: chrono::NaiveDateTime) -> i32 {
    60
}

fn lib() -> Connection {
    let mut c = Connection::open_in_memory().unwrap();
    migrate::migrate(&mut c, None).unwrap();
    c
}

fn photo(
    c: &Connection,
    name: &str,
    at: &str,
    off: Option<i32>,
    device: Option<&str>,
    src: DateSource,
) -> i64 {
    let id = repo::media::upsert(
        c,
        &NewMedia {
            path: name.into(),
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
            declared_offset_min: off,
            device: device.map(str::to_string),
            ..Default::default()
        },
        &sys,
    )
    .unwrap();
    id
}

const XT5: Option<&str> = Some("Fujifilm X-T5");
const IPHONE: Option<&str> = Some("Apple iPhone 17 Pro Max");
const EXIF: DateSource = DateSource::ExifOriginal;

fn shown(c: &Connection, id: i64) -> String {
    let (u, l): (i64, i32) = c
        .query_row(
            "SELECT instant_utc, local_offset_min FROM media_dates WHERE media_id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    timeline::local_text(u, l).unwrap()
}

fn order(c: &Connection) -> Vec<i64> {
    let mut q = c
        .prepare("SELECT media_id FROM media_dates ORDER BY instant_utc, media_id")
        .unwrap();
    q.query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

fn group(c: &Connection, id: i64) -> i64 {
    timeline::instant_of(c, id).unwrap().1
}

#[test]
fn local_time_changes_display_only_and_undo_restores_it() {
    let c = lib();
    let x = photo(&c, "x.raf", "2026-08-19 10:43:00", Some(120), XT5, EXIF);
    let i = photo(
        &c,
        "i.heic",
        "2026-08-19 01:44:00",
        Some(-420),
        IPHONE,
        EXIF,
    );
    let before = order(&c);
    let g = group(&c, x);
    timeline::set_display(&c, g, None, Some(-420), &sys).unwrap();
    assert_eq!(shown(&c, x), "2026-08-19 01:43:00");
    assert_eq!(order(&c), before, "la posizione non cambia");
    assert_eq!(order(&c), vec![x, i]);
    bulk::undo_last(&c).unwrap();
    assert_eq!(shown(&c, x), "2026-08-19 10:43:00");
    // Fuso IANA del gruppo.
    timeline::set_display(&c, g, Some("America/Denver".into()), None, &sys).unwrap();
    assert_eq!(shown(&c, x), "2026-08-19 02:43:00");
    assert!(timeline::set_display(&c, g, Some("Marte/Olympus".into()), None, &sys).is_err());
}

#[test]
fn shift_to_the_second_reorders_and_undo_restores() {
    let c = lib();
    let x = photo(&c, "x.raf", "2026-08-19 10:41:50", Some(-360), XT5, EXIF);
    let i = photo(
        &c,
        "i.heic",
        "2026-08-19 10:43:00",
        Some(-360),
        IPHONE,
        EXIF,
    );
    assert_eq!(order(&c), vec![x, i]);
    let g = group(&c, x);
    let clock = GroupClock {
        shift_s: 130,
        ..Default::default()
    };
    let preview = timeline::preview_clock(&c, g, &clock, 5, &sys).unwrap();
    assert_eq!(preview[0].before.as_deref(), Some("2026-08-19 10:41:50"));
    assert_eq!(preview[0].after.as_deref(), Some("2026-08-19 10:44:00"));
    timeline::set_shift(&c, g, 130, &sys).unwrap();
    assert_eq!(shown(&c, x), "2026-08-19 10:44:00");
    assert_eq!(order(&c), vec![i, x], "la sequenza si riordina");
    assert_eq!(
        bulk::last_undoable(&c).unwrap().as_deref(),
        Some("Scostamento del gruppo: +2 min 10 s")
    );
    bulk::undo_last(&c).unwrap();
    assert_eq!(shown(&c, x), "2026-08-19 10:41:50");
    assert_eq!(order(&c), vec![x, i]);
}

#[test]
fn aligning_two_photos_proposes_the_shift() {
    let c = lib();
    let i = photo(
        &c,
        "i.heic",
        "2026-08-19 18:30:05",
        Some(-360),
        IPHONE,
        EXIF,
    );
    let x = photo(&c, "x.raf", "2026-08-19 18:27:40", Some(-360), XT5, EXIF);
    let a = timeline::propose_alignment(&c, i, x).unwrap();
    assert_eq!(a.delta_s, 145);
    assert_eq!(timeline::shift_text(a.delta_s), "+2 min 25 s");
    assert_eq!(a.group, group(&c, x));
    timeline::set_shift(&c, a.group, a.proposed_shift_s, &sys).unwrap();
    assert_eq!(
        timeline::instant_of(&c, x).unwrap().0,
        timeline::instant_of(&c, i).unwrap().0
    );
    bulk::undo_last(&c).unwrap();
    assert_eq!(timeline::propose_alignment(&c, i, x).unwrap().delta_s, 145);
    let same = photo(
        &c,
        "i2.heic",
        "2026-08-19 18:31:00",
        Some(-360),
        IPHONE,
        EXIF,
    );
    assert!(timeline::propose_alignment(&c, i, same).is_err());
}

#[test]
fn undated_photo_is_placed_between_neighbours_and_undo_removes_it() {
    let c = lib();
    let a = photo(&c, "a.jpg", "2026-08-19 18:30:00", Some(-360), IPHONE, EXIF);
    let b = photo(&c, "b.jpg", "2026-08-19 18:34:00", Some(-360), IPHONE, EXIF);
    let u = photo(
        &c,
        "ricevuta.jpg",
        "2026-10-01 09:00:00",
        None,
        None,
        DateSource::FileSystem,
    );
    let utc = timeline::place_after(&c, u, a, &sys).unwrap();
    let ia = timeline::instant_of(&c, a).unwrap().0;
    let ib = timeline::instant_of(&c, b).unwrap().0;
    assert!(ia < utc && utc < ib);
    assert_eq!(shown(&c, u), "2026-08-19 18:32:00");
    assert_eq!(order(&c), vec![a, u, b]);
    // Ricalcoli del gruppo non spostano la foto collocata.
    timeline::set_shift(&c, group(&c, u), 3600, &sys).unwrap();
    assert_eq!(order(&c), vec![a, u, b]);
    bulk::undo_last(&c).unwrap();
    bulk::undo_last(&c).unwrap();
    let placed: i64 = c
        .query_row("SELECT count(*) FROM media_time_placement", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(placed, 0);
    assert_eq!(shown(&c, u), "2026-10-01 09:00:00");
    // Dopo l'ultima foto: +1 s.
    let last = timeline::place_after(&c, u, b, &sys).unwrap();
    assert_eq!(last, ib + 1000);
}

#[test]
fn shift_on_100k_rows_is_under_one_second() {
    let c = lib();
    let g = timeline::ensure_group(&c, XT5, Some(120)).unwrap();
    c.execute_batch("BEGIN").unwrap();
    {
        let mut m = c
            .prepare(
                "INSERT INTO media_item(id,path,name,ext,size,mtime,added_at,updated_at)
                 VALUES (?1,?2,?2,'raf',1,1,1,1)",
            )
            .unwrap();
        let mut d = c
            .prepare(
                "INSERT INTO media_dates(media_id,captured_at,captured_source,declared_offset_min,device,instant_utc,local_offset_min,group_id)
                 VALUES (?1,'2026-08-19 10:43:00','exif_original',120,'Fujifilm X-T5',?2,120,?3)",
            )
            .unwrap();
        for i in 1..=100_000i64 {
            m.execute(rusqlite::params![i, format!("f{i}.raf")])
                .unwrap();
            d.execute(rusqlite::params![i, 1_787_000_000_000i64 + i * 1000, g])
                .unwrap();
        }
    }
    c.execute_batch("COMMIT").unwrap();
    let t = Instant::now();
    c.execute_batch("BEGIN").unwrap();
    let n = timeline::set_shift(&c, g, 130, &sys).unwrap();
    c.execute_batch("COMMIT").unwrap();
    let elapsed = t.elapsed();
    assert_eq!(n, 100_000);
    assert!(elapsed.as_millis() < 1000, "{elapsed:?}");
    let first: i64 = c
        .query_row(
            "SELECT instant_utc FROM media_dates WHERE media_id = 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(first, 1_787_000_000_000 + 1000 + 130_000);
}

/// 64 scatti X-T5 che dichiarano +02:00 (orologio rimasto sull'ora italiana)
/// e foto iPhone con GPS negli stessi istanti, con fuso `iphone_offset`.
fn trip(c: &Connection, iphone_offset: i32) {
    for k in 0..64i64 {
        let utc = chrono::NaiveDate::from_ymd_opt(2026, 8, 19)
            .unwrap()
            .and_hms_opt(15, 0, 0)
            .unwrap()
            + chrono::Duration::minutes(3 * k);
        let at = |off: i32| {
            (utc + chrono::Duration::minutes(off as i64))
                .format(timeline::LOCAL_FMT)
                .to_string()
        };
        photo(c, &format!("x{k}.raf"), &at(120), Some(120), XT5, EXIF);
        let id = photo(
            c,
            &format!("i{k}.heic"),
            &at(iphone_offset),
            Some(iphone_offset),
            IPHONE,
            EXIF,
        );
        c.execute(
            "INSERT INTO exif(media_id, gps_lat, gps_lon) VALUES (?1, 33.45, -112.07)",
            [id],
        )
        .unwrap();
    }
}

fn phoenix(_: f64, _: f64) -> Option<String> {
    Some("America/Phoenix".to_string())
}

#[test]
fn gps_of_another_source_suggests_the_local_time() {
    let c = lib();
    trip(&c, -420);
    let s = timeline::suggest_timezones(&c, None, &phoenix).unwrap();
    assert_eq!(s.len(), 1, "{s:?}");
    assert_eq!(s[0].device.as_deref(), XT5);
    assert_eq!(s[0].offset_min, -420);
    assert_eq!(timeline::offset_text(s[0].offset_min), "-07:00");
    assert_eq!(s[0].tz.as_deref(), Some("America/Phoenix"));
    assert_eq!(s[0].current_offset_min, 120);
    assert_eq!(s[0].count, 64);
    // Accettato: l'ora locale del gruppo diventa quella proposta e il suggerimento sparisce.
    timeline::set_display(&c, s[0].group, s[0].tz.clone(), None, &sys).unwrap();
    assert!(timeline::suggest_timezones(&c, None, &phoenix)
        .unwrap()
        .is_empty());
    // Ignorato: non viene più proposto.
    bulk::undo_last(&c).unwrap();
    timeline::ignore_suggestion(&c, s[0].group, -420).unwrap();
    assert!(timeline::suggest_timezones(&c, None, &phoenix)
        .unwrap()
        .is_empty());
}

#[test]
fn no_suggestion_when_time_zones_agree() {
    let c = lib();
    trip(&c, 120);
    let rome = |_: f64, _: f64| Some("Europe/Rome".to_string());
    assert!(timeline::suggest_timezones(&c, None, &rome)
        .unwrap()
        .is_empty());
}

#[test]
fn library_sorts_by_real_instant_and_filters_on_corrected_local_day() {
    use pc_store::query::{self, LibraryFilter, SortKey};
    let c = lib();
    let x1 = photo(&c, "x1.raf", "2026-08-19 10:41:00", Some(-360), XT5, EXIF);
    let i1 = photo(
        &c,
        "i1.heic",
        "2026-08-19 10:42:00",
        Some(-360),
        IPHONE,
        EXIF,
    );
    let x2 = photo(&c, "x2.raf", "2026-08-19 23:59:00", Some(-360), XT5, EXIF);
    let ids = |sort| -> Vec<i64> {
        query::page(&c, &LibraryFilter::default(), sort, None, 50)
            .unwrap()
            .items
            .iter()
            .map(|i| i.id)
            .collect()
    };
    assert_eq!(ids(SortKey::CapturedAsc), vec![x1, i1, x2]);
    // +2 minuti alla X-T5: x1 passa dopo i1 nell'ordinamento per data di scatto.
    timeline::set_shift(&c, group(&c, x1), 120, &sys).unwrap();
    assert_eq!(ids(SortKey::CapturedAsc), vec![i1, x1, x2]);
    assert_eq!(ids(SortKey::CapturedDesc), vec![x2, x1, i1]);
    // x2 corretta cade il 20 agosto: il filtro per data usa l'ora locale corretta.
    let day = |d: &str| LibraryFilter {
        date_from: Some(d.into()),
        date_to: Some(d.into()),
        ..Default::default()
    };
    assert_eq!(query::ids(&c, &day("2026-08-20")).unwrap(), vec![x2]);
    assert_eq!(query::ids(&c, &day("2026-08-19")).unwrap(), vec![x1, i1]);
    // Il CSV di catalog-export continua a usare l'ora letta dal file.
    let d = repo::meta::dates(&c, x2).unwrap().unwrap();
    assert_eq!(d.captured_at.as_deref(), Some("2026-08-19 23:59:00"));
}
