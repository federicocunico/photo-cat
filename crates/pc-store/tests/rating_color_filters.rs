//! Filtri, facet e regole smart per valutazione, colore e "Anno senza mese".
use pc_store::bulk;
use pc_store::model::*;
use pc_store::query::{self, FacetKey, FacetList, LibraryFilter, RatingFilter, RatingOp};
use pc_store::repo;
use pc_store::{Store, StoreOptions};
use rusqlite::Connection;
use serde_json::json;

const UTC: &dyn Fn(chrono::NaiveDateTime) -> i32 = &|_| 0;

fn store(dir: &std::path::Path) -> Store {
    Store::open(&dir.join("catalog.db"), StoreOptions::default()).unwrap()
}

fn add(c: &Connection, name: &str, ext: &str, at: &str) -> i64 {
    let id = repo::media::upsert(
        c,
        &NewMedia {
            path: format!("D:\\foto\\{name}.{ext}"),
            name: format!("{name}.{ext}"),
            ext: ext.into(),
            ..Default::default()
        },
    )
    .unwrap();
    repo::meta::set_dates_with(
        c,
        id,
        &MediaDates {
            captured_at: Some(at.into()),
            captured_source: Some(DateSource::ExifOriginal),
            ..Default::default()
        },
        UTC,
    )
    .unwrap();
    id
}

fn rating(op: RatingOp, value: u8) -> Option<RatingFilter> {
    Some(RatingFilter { op, value })
}

fn count(l: &FacetList, v: &str) -> i64 {
    l.values.iter().find(|x| x.value == v).unwrap().count
}

/// a(5,bw,jpg) b(4,sepia,jpg) c(4,-,png) d(2,bw,png) e(0,-,jpg)
fn seed(c: &Connection) -> Vec<i64> {
    let ids: Vec<i64> = [
        ("a", "jpg"),
        ("b", "jpg"),
        ("c", "png"),
        ("d", "png"),
        ("e", "jpg"),
    ]
    .iter()
    .map(|(n, e)| add(c, n, e, "2020-05-01 10:00:00"))
    .collect();
    bulk::set_rating(c, &[ids[0]], Some(5)).unwrap();
    bulk::set_rating(c, &[ids[1], ids[2]], Some(4)).unwrap();
    bulk::set_rating(c, &[ids[3]], Some(2)).unwrap();
    bulk::set_color_trait(c, &[ids[0], ids[3]], Some(ColorTrait::Bw)).unwrap();
    bulk::set_color_trait(c, &[ids[1]], Some(ColorTrait::Sepia)).unwrap();
    ids
}

#[test]
fn at_least_four_and_combination_with_format() {
    let dir = tempfile::tempdir().unwrap();
    let s = store(dir.path());
    let ids = s.write(|c| Ok(seed(c))).unwrap();
    let f = LibraryFilter {
        rating: rating(RatingOp::AtLeast, 4),
        ..Default::default()
    };
    let mut got = s.read(|c| query::ids(c, &f)).unwrap();
    got.sort();
    assert_eq!(got, vec![ids[0], ids[1], ids[2]]);
    let f = LibraryFilter {
        formats: vec!["png".into()],
        ..f
    };
    assert_eq!(s.read(|c| query::ids(c, &f)).unwrap(), vec![ids[2]]);
    let eq = LibraryFilter {
        rating: rating(RatingOp::Equal, 4),
        ..Default::default()
    };
    assert_eq!(s.read(|c| query::count(c, &eq)).unwrap(), 2);
    let most = LibraryFilter {
        rating: rating(RatingOp::AtMost, 2),
        ..Default::default()
    };
    assert_eq!(s.read(|c| query::count(c, &most)).unwrap(), 2);
}

#[test]
fn color_not_indicated_and_or() {
    let dir = tempfile::tempdir().unwrap();
    let s = store(dir.path());
    let ids = s.write(|c| Ok(seed(c))).unwrap();
    let none = LibraryFilter {
        color_trait: vec![None],
        ..Default::default()
    };
    let mut got = s.read(|c| query::ids(c, &none)).unwrap();
    got.sort();
    assert_eq!(got, vec![ids[2], ids[4]]);
    let or = LibraryFilter {
        color_trait: vec![None, Some(ColorTrait::Sepia)],
        ..Default::default()
    };
    assert_eq!(s.read(|c| query::count(c, &or)).unwrap(), 3);
    let both = LibraryFilter {
        color_trait: vec![Some(ColorTrait::Bw)],
        rating: rating(RatingOp::AtLeast, 3),
        ..Default::default()
    };
    assert_eq!(s.read(|c| query::ids(c, &both)).unwrap(), vec![ids[0]]);
}

#[test]
fn facet_counts_ignore_own_selection() {
    let dir = tempfile::tempdir().unwrap();
    let s = store(dir.path());
    s.write(|c| Ok(seed(c))).unwrap();
    let f = LibraryFilter {
        rating: rating(RatingOp::AtLeast, 4),
        color_trait: vec![Some(ColorTrait::Bw)],
        ..Default::default()
    };
    let fa = s.read(|c| query::facets(c, &f)).unwrap();
    // Le valutazioni ignorano la propria selezione ma applicano il colore (bw: a=5, d=2).
    assert_eq!(count(&fa.ratings, "5"), 1);
    assert_eq!(count(&fa.ratings, "2"), 1);
    assert_eq!(count(&fa.ratings, "4"), 0);
    assert_eq!(fa.ratings.values.len(), 6);
    // I colori applicano la valutazione >= 4 (a bw, b sepia, c non indicato).
    assert_eq!(count(&fa.colors, "bw"), 1);
    assert_eq!(count(&fa.colors, "sepia"), 1);
    assert_eq!(count(&fa.colors, "none"), 1);
    assert_eq!(count(&fa.colors, "faded"), 0);
    let page = s
        .read(|c| {
            query::facet_values(
                c,
                &f,
                &FacetKey::Color,
                None,
                query::FacetSort::Count,
                0,
                10,
            )
        })
        .unwrap();
    assert_eq!(page.total, 5);
}

#[test]
fn smart_catalog_follows_bulk_changes() {
    let dir = tempfile::tempdir().unwrap();
    let s = store(dir.path());
    let ids = s.write(|c| Ok(seed(c))).unwrap();
    let rules = json!({"op": "and", "rules": [
        {"field": "rating", "cmp": "gte", "value": 4},
        {"field": "color_trait", "cmp": "neq", "value": "bw"}
    ]});
    let cid = s
        .write(move |c| repo::catalog::create(c, "Belle", CatalogKind::Smart, Some(&rules)))
        .unwrap();
    let f = LibraryFilter {
        catalog_id: Some(cid),
        ..Default::default()
    };
    let mut got = s.read(|c| query::ids(c, &f)).unwrap();
    got.sort();
    assert_eq!(got, vec![ids[1], ids[2]]);
    let (i1, i3) = (ids[1], ids[3]);
    s.write(move |c| {
        bulk::set_rating(c, &[i3], Some(5))?;
        bulk::set_color_trait(c, &[i3], None)?;
        bulk::set_rating(c, &[i1], Some(1))?;
        Ok(())
    })
    .unwrap();
    let mut got = s.read(|c| query::ids(c, &f)).unwrap();
    got.sort();
    assert_eq!(got, vec![ids[2], ids[3]]);
    let none = json!({"field": "color_trait", "cmp": "eq", "value": "none"});
    let f = LibraryFilter {
        rule: Some(none),
        ..Default::default()
    };
    assert_eq!(s.read(|c| query::count(c, &f)).unwrap(), 3);
    assert!(pc_store::rules::parse(&json!({"field": "rating", "cmp": "gte", "value": 9})).is_err());
    assert!(pc_store::rules::parse(
        &json!({"field": "color_trait", "cmp": "eq", "value": "rosso"})
    )
    .is_err());
}

#[test]
fn year_only_dates_live_in_year_without_month() {
    let dir = tempfile::tempdir().unwrap();
    let s = store(dir.path());
    let (y, m, d, plain) = s
        .write(|c| {
            let y = add(c, "y", "jpg", "2019-03-02 10:00:00");
            let m = add(c, "m", "jpg", "2019-03-02 10:00:00");
            let d = add(c, "d", "jpg", "2019-03-02 10:00:00");
            let plain = add(c, "p", "jpg", "1984-01-15 10:00:00");
            bulk::set_manual_date_with(c, &[y], "1984", DatePrecision::Year, UTC)?;
            bulk::set_manual_date_with(c, &[m], "1984-01", DatePrecision::Month, UTC)?;
            bulk::set_manual_date_with(c, &[d], "1984-01-15", DatePrecision::Day, UTC)?;
            Ok((y, m, d, plain))
        })
        .unwrap();
    let year = LibraryFilter {
        date_from: Some("1984-01-01".into()),
        date_to: Some("1984-12-31".into()),
        ..Default::default()
    };
    let fa = s.read(|c| query::facets(c, &year)).unwrap();
    let months = fa.months.unwrap();
    assert_eq!(count(&months, "1984-00"), 1);
    assert_eq!(count(&months, "1984-01"), 3);
    // Mese scelto: la foto "solo anno" non c'è, quella "mese" sì ma non tra i giorni.
    let jan = LibraryFilter {
        date_from: Some("1984-01-01".into()),
        date_to: Some("1984-01-31".into()),
        ..Default::default()
    };
    let mut got = s.read(|c| query::ids(c, &jan)).unwrap();
    got.sort();
    assert_eq!(got, vec![m, d, plain]);
    let days = s.read(|c| query::facets(c, &jan)).unwrap().days.unwrap();
    assert_eq!(days.values.len(), 1);
    assert_eq!(count(&days, "1984-01-15"), 2);
    let day = LibraryFilter {
        date_from: Some("1984-01-15".into()),
        date_to: Some("1984-01-15".into()),
        ..Default::default()
    };
    let mut got = s.read(|c| query::ids(c, &day)).unwrap();
    got.sort();
    assert_eq!(got, vec![d, plain]);
    // Nodo "Anno senza mese".
    let nomonth = LibraryFilter {
        date_no_month: true,
        ..year
    };
    assert_eq!(s.read(|c| query::ids(c, &nomonth)).unwrap(), vec![y]);
    let fa = s.read(|c| query::facets(c, &nomonth)).unwrap();
    let months = fa.months.unwrap();
    assert_eq!(count(&months, "1984-00"), 1);
    assert_eq!(fa.days, None);
}

#[test]
fn color_rule_in_and_values_checked() {
    let dir = tempfile::tempdir().unwrap();
    let s = store(dir.path());
    let ids = s.write(|c| Ok(seed(c))).unwrap();
    let ids_of = |rule: serde_json::Value| -> Vec<i64> {
        let f = LibraryFilter {
            rule: Some(rule),
            ..Default::default()
        };
        let mut got = s.read(|c| query::ids(c, &f)).unwrap();
        got.sort();
        got
    };
    assert_eq!(
        ids_of(json!({"field": "color_trait", "cmp": "in", "value": ["sepia", "none"]})),
        vec![ids[1], ids[2], ids[4]]
    );
    assert_eq!(
        ids_of(json!({"field": "color_trait", "cmp": "eq", "value": "bw"})),
        vec![ids[0], ids[3]]
    );
    // "diverso da" include le foto non indicate.
    assert_eq!(
        ids_of(json!({"field": "color_trait", "cmp": "neq", "value": "none"})),
        vec![ids[0], ids[1], ids[3]]
    );
    for bad in [
        json!({"field": "color_trait", "cmp": "in", "value": ["sepia", "rosso"]}),
        json!({"field": "rating", "cmp": "in", "value": [4, 9]}),
    ] {
        assert!(pc_store::rules::parse(&bad).is_err(), "{bad}");
    }
}

/// I percorsi caldi usano gli indici `media_rating` e `media_color_trait`
/// invece di scandire la tabella (100.000 foto).
#[test]
fn rating_and_color_filters_use_their_indexes() {
    let dir = tempfile::tempdir().unwrap();
    let s = store(dir.path());
    s.write(|c| Ok(seed(c))).unwrap();
    let plan = |f: LibraryFilter| -> String {
        s.read(move |c| {
            let w = query::compile_filter(c, &f)?;
            let sql = format!(
                "EXPLAIN QUERY PLAN SELECT m.id FROM media_item m \
                 LEFT JOIN media_dates d ON d.media_id = m.id WHERE {}",
                w.sql
            );
            let mut st = c.prepare(&sql)?;
            let rows: Vec<String> = st
                .query_map(rusqlite::params_from_iter(w.args.iter()), |r| r.get(3))?
                .collect::<Result<_, _>>()?;
            Ok(rows.join("\n"))
        })
        .unwrap()
    };
    let cases = [
        (
            LibraryFilter {
                rating: rating(RatingOp::AtLeast, 4),
                ..Default::default()
            },
            "media_rating (rating>?)",
        ),
        (
            LibraryFilter {
                color_trait: vec![Some(ColorTrait::Sepia)],
                ..Default::default()
            },
            "media_color_trait (color_trait=?)",
        ),
        (
            LibraryFilter {
                rule: Some(json!({"field": "rating", "cmp": "gte", "value": 4})),
                ..Default::default()
            },
            "media_rating (rating>?)",
        ),
        (
            LibraryFilter {
                rule: Some(json!({"field": "color_trait", "cmp": "eq", "value": "bw"})),
                ..Default::default()
            },
            "media_color_trait (color_trait=?)",
        ),
        (
            LibraryFilter {
                rule: Some(json!({"field": "color_trait", "cmp": "eq", "value": "none"})),
                ..Default::default()
            },
            "media_color_trait (color_trait=?)",
        ),
    ];
    for (f, expected) in cases {
        let p = plan(f);
        assert!(p.contains(expected), "{expected} non in:\n{p}");
    }
}

#[test]
fn manual_date_without_file_date_appears_in_year_list() {
    let dir = tempfile::tempdir().unwrap();
    let s = store(dir.path());
    s.write(|c| {
        let id = repo::media::upsert(
            c,
            &NewMedia {
                path: r"D:\foto\senza.jpg".into(),
                name: "senza.jpg".into(),
                ext: "jpg".into(),
                ..Default::default()
            },
        )?;
        bulk::set_manual_date_with(c, &[id], "1962", DatePrecision::Year, UTC)?;
        Ok(())
    })
    .unwrap();
    let fa = s
        .read(|c| query::facets(c, &LibraryFilter::default()))
        .unwrap();
    assert_eq!(count(&fa.years, "1962"), 1);
}
