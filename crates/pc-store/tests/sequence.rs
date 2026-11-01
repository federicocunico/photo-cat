//! Query della vista Sequenza (task 6.1).
use pc_store::model::{DateSource, MediaDates, NewMedia};
use pc_store::sequence::{self, SequenceScope, DEFAULT_SESSION_GAP_MIN};
use pc_store::{bulk, migrate, repo, timeline};
use rusqlite::Connection;
use std::time::Instant;

fn sys(_: chrono::NaiveDateTime) -> i32 {
    0
}

fn lib() -> Connection {
    let mut c = Connection::open_in_memory().unwrap();
    migrate::migrate(&mut c, None).unwrap();
    c
}

fn photo(c: &Connection, name: &str, at: &str, device: &str, src: DateSource) -> i64 {
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
            declared_offset_min: Some(-360),
            device: Some(device.into()),
            ..Default::default()
        },
        &sys,
    )
    .unwrap();
    id
}

const EXIF: DateSource = DateSource::ExifOriginal;

/// Due uscite: mattina (3 foto) e sera dopo 6 ore (2 foto), con una pausa di
/// 1 h 30 nella mattina.
fn day(c: &Connection) -> Vec<i64> {
    let v = vec![
        photo(c, "a.jpg", "2026-08-19 08:00:00", "Fujifilm X-T5", EXIF),
        photo(
            c,
            "b.jpg",
            "2026-08-19 08:10:00",
            "Apple iPhone 17 Pro Max",
            EXIF,
        ),
        photo(c, "c.jpg", "2026-08-19 09:40:00", "Fujifilm X-T5", EXIF),
        photo(c, "d.jpg", "2026-08-19 15:40:00", "Fujifilm X-T5", EXIF),
        photo(
            c,
            "e.jpg",
            "2026-08-19 15:45:00",
            "Apple iPhone 17 Pro Max",
            EXIF,
        ),
    ];
    c.execute(
        "INSERT INTO location(media_id, name, source) VALUES (?1,'Moab','gps'), (?2,'Moab','gps'), (?3,'Arches','gps')",
        [v[0], v[1], v[2]],
    )
    .unwrap();
    v
}

#[test]
fn sessions_with_default_and_custom_gap() {
    let c = lib();
    let ids = day(&c);
    let s = sequence::sessions(&c, &SequenceScope::Library, DEFAULT_SESSION_GAP_MIN).unwrap();
    assert_eq!(s.len(), 2, "pausa di 6 ore oltre la soglia di 2 ore");
    assert_eq!(s[0].count, 3);
    assert_eq!(s[0].start_local, "2026-08-19 08:00:00");
    assert_eq!(s[0].end_local, "2026-08-19 09:40:00");
    assert_eq!(s[0].place.as_deref(), Some("Moab"));
    assert_eq!(s[1].count, 2);
    assert_eq!(s[1].place, None);
    // Soglia di 60 minuti: la pausa di 1 h 30 apre una terza sessione.
    let s = sequence::sessions(&c, &SequenceScope::Library, 60).unwrap();
    assert_eq!(s.iter().map(|x| x.count).collect::<Vec<_>>(), vec![2, 1, 2]);
    // Tasto A: tutte le foto di una sessione.
    assert_eq!(
        sequence::session_ids(&c, &SequenceScope::Library, DEFAULT_SESSION_GAP_MIN, 1).unwrap(),
        vec![ids[3], ids[4]]
    );
}

#[test]
fn page_has_device_sessions_and_tags_of_the_active_set() {
    let c = lib();
    let ids = day(&c);
    bulk::add_tag(&c, "Paesaggi", &ids[..2], &[]).unwrap();
    bulk::add_tag(&c, "Fuori set", &ids[..1], &[]).unwrap();
    let set = vec!["Paesaggi".to_string(), "Città".to_string()];
    let p = sequence::page(
        &c,
        &SequenceScope::Library,
        DEFAULT_SESSION_GAP_MIN,
        &set,
        None,
        3,
    )
    .unwrap();
    assert_eq!(p.items.iter().map(|i| i.id).collect::<Vec<_>>(), ids[..3]);
    assert_eq!(p.items[0].tags, vec!["Paesaggi"]);
    assert_eq!(
        p.items[1].device.as_deref(),
        Some("Apple iPhone 17 Pro Max")
    );
    assert_eq!(p.items[0].local, "2026-08-19 08:00:00");
    let next = sequence::page(
        &c,
        &SequenceScope::Library,
        DEFAULT_SESSION_GAP_MIN,
        &set,
        p.next.as_ref(),
        3,
    )
    .unwrap();
    assert_eq!(
        next.items
            .iter()
            .map(|i| (i.id, i.session))
            .collect::<Vec<_>>(),
        vec![(ids[3], 1), (ids[4], 1)]
    );
    assert!(next.next.is_none());
    // Ambito selezione.
    let sel = SequenceScope::Selection {
        ids: vec![ids[4], ids[0]],
    };
    let p = sequence::page(&c, &sel, DEFAULT_SESSION_GAP_MIN, &set, None, 10).unwrap();
    assert_eq!(
        p.items.iter().map(|i| i.id).collect::<Vec<_>>(),
        vec![ids[0], ids[4]]
    );
    assert_eq!(
        sequence::ordered_ids(&c, &sel).unwrap(),
        vec![ids[0], ids[4]]
    );
}

#[test]
fn undated_photos_stay_apart_until_placed() {
    let c = lib();
    let ids = day(&c);
    let u = photo(
        &c,
        "ricevuta.jpg",
        "2026-10-01 09:00:00",
        "",
        DateSource::FileSystem,
    );
    let und = sequence::undated(&c, &SequenceScope::Library).unwrap();
    assert_eq!(und.iter().map(|x| x.id).collect::<Vec<_>>(), vec![u]);
    let all = sequence::ordered_ids(&c, &SequenceScope::Library).unwrap();
    assert!(!all.contains(&u));
    timeline::place_after(&c, u, ids[0], &sys).unwrap();
    assert!(sequence::undated(&c, &SequenceScope::Library)
        .unwrap()
        .is_empty());
    let all = sequence::ordered_ids(&c, &SequenceScope::Library).unwrap();
    assert_eq!(&all[..3], &[ids[0], u, ids[1]]);
}

#[test]
fn first_page_of_100k_within_one_second() {
    let c = lib();
    let g = timeline::ensure_group(&c, Some("Fujifilm X-T5"), Some(120)).unwrap();
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
                "INSERT INTO media_dates(media_id,captured_at,captured_source,instant_utc,local_offset_min,group_id)
                 VALUES (?1,'2026-08-19 10:43:00','exif_original',?2,120,?3)",
            )
            .unwrap();
        for i in 1..=100_000i64 {
            m.execute(rusqlite::params![i, format!("f{i}.raf")])
                .unwrap();
            // Una pausa lunga ogni 500 scatti.
            let t = 1_787_000_000_000i64 + i * 5_000 + (i / 500) * 4 * 3_600_000;
            d.execute(rusqlite::params![i, t, g]).unwrap();
        }
    }
    c.execute_batch("COMMIT").unwrap();
    let started = Instant::now();
    let p = sequence::page(
        &c,
        &SequenceScope::Library,
        DEFAULT_SESSION_GAP_MIN,
        &[],
        None,
        500,
    )
    .unwrap();
    let elapsed = started.elapsed();
    assert_eq!(p.items.len(), 500);
    assert!(elapsed.as_millis() < 1000, "{elapsed:?}");
    let s = sequence::sessions(&c, &SequenceScope::Library, DEFAULT_SESSION_GAP_MIN).unwrap();
    assert_eq!(s.len(), 201);
}
