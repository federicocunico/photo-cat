//! Liste dei filtri: conteggi che ignorano la propria selezione, totale delle
//! voci, elenco completo con ricerca, ordinamento e pagine.

use pc_store::query::{self, FacetKey, FacetSort, FacetValue, LabelFilter, LibraryFilter};
use pc_store::{Store, StoreOptions};
use rusqlite::{params, Connection};
use std::time::Instant;

fn store(dir: &std::path::Path) -> Store {
    Store::open(&dir.join("catalog.db"), StoreOptions::default()).unwrap()
}

/// Inserisce una foto con luogo, catalogazioni e tag.
fn photo(c: &Connection, id: i64, place: Option<&str>, labels: &[(&str, &str)], tags: &[&str]) {
    c.execute(
        "INSERT INTO media_item(id, path, name, ext, size, mtime, added_at, updated_at)
         VALUES (?1, ?2, ?3, 'jpg', 1, 0, 0, 0)",
        params![id, format!("/foto/{id}.jpg"), format!("{id}.jpg")],
    )
    .unwrap();
    c.execute(
        "INSERT INTO media_dates(media_id, captured_at) VALUES (?1, ?2)",
        params![id, format!("{}-05-01 10:00:00", 2020 + id % 4)],
    )
    .unwrap();
    if let Some(p) = place {
        c.execute(
            "INSERT INTO location(media_id, name, source) VALUES (?1, ?2, 'gps')",
            params![id, p],
        )
        .unwrap();
    }
    for (g, v) in labels {
        c.execute(
            "INSERT INTO label(media_id, grp, value, score, origin) VALUES (?1, ?2, ?3, 0.9, 'auto')",
            params![id, g, v],
        )
        .unwrap();
    }
    for t in tags {
        c.execute("INSERT OR IGNORE INTO tag(name) VALUES (?1)", [t])
            .unwrap();
        c.execute(
            "INSERT INTO media_tag(media_id, tag_id) SELECT ?1, id FROM tag WHERE name = ?2",
            params![id, t],
        )
        .unwrap();
    }
}

fn count_of(values: &[FacetValue], value: &str) -> Option<i64> {
    values.iter().find(|v| v.value == value).map(|v| v.count)
}

/// Siena 20 foto (10 esterne), Firenze 80 (60 esterne), Roma 5.
fn three_cities(c: &Connection) {
    let mut id = 0;
    let mut add = |n: i64, place: &str, outdoor: i64| {
        for i in 0..n {
            id += 1;
            let loc = if i < outdoor {
                "esterno_urbano"
            } else {
                "interno"
            };
            let subj = if i % 2 == 0 { "ritratto" } else { "paesaggio" };
            photo(
                c,
                id,
                Some(place),
                &[("location", loc), ("subject", subj)],
                &[],
            );
        }
    };
    add(20, "Siena", 10);
    add(80, "Firenze", 60);
    add(5, "Roma", 0);
}

#[test]
fn choosing_a_place_keeps_the_other_places_selectable() {
    let tmp = tempfile::tempdir().unwrap();
    let s = store(tmp.path());
    s.write(|c| {
        three_cities(c);
        Ok(())
    })
    .unwrap();
    let f = LibraryFilter {
        places: vec!["Siena".into()],
        ..Default::default()
    };
    let facets = s.read(move |c| query::facets(c, &f)).unwrap();
    assert_eq!(count_of(&facets.places.values, "Firenze"), Some(80));
    assert_eq!(count_of(&facets.places.values, "Siena"), Some(20));
    assert_eq!(facets.places.total, 3);
    // Le altre liste applicano la selezione del luogo.
    let location = &facets
        .labels
        .iter()
        .find(|(g, _)| g == "location")
        .unwrap()
        .1;
    assert_eq!(count_of(&location.values, "esterno_urbano"), Some(10));
    assert_eq!(count_of(&location.values, "interno"), Some(10));
}

#[test]
fn place_counts_apply_the_other_filters() {
    let tmp = tempfile::tempdir().unwrap();
    let s = store(tmp.path());
    s.write(|c| {
        three_cities(c);
        Ok(())
    })
    .unwrap();
    let f = LibraryFilter {
        labels: vec![LabelFilter {
            group: "location".into(),
            values: vec!["esterno_urbano".into()],
        }],
        ..Default::default()
    };
    let facets = s.read(move |c| query::facets(c, &f)).unwrap();
    assert_eq!(count_of(&facets.places.values, "Siena"), Some(10));
    assert_eq!(count_of(&facets.places.values, "Firenze"), Some(60));
    assert_eq!(
        count_of(&facets.places.values, "Roma"),
        None,
        "nessuna foto esterna"
    );
    // Il gruppo selezionato non si riduce al valore scelto.
    let location = &facets
        .labels
        .iter()
        .find(|(g, _)| g == "location")
        .unwrap()
        .1;
    assert_eq!(count_of(&location.values, "interno"), Some(35));
    assert_eq!(count_of(&location.values, "esterno_urbano"), Some(70));
    // Un altro gruppo invece sì.
    let subject = &facets
        .labels
        .iter()
        .find(|(g, _)| g == "subject")
        .unwrap()
        .1;
    assert_eq!(subject.values.iter().map(|v| v.count).sum::<i64>(), 70);
}

#[test]
fn year_and_tag_lists_ignore_their_own_selection() {
    let tmp = tempfile::tempdir().unwrap();
    let s = store(tmp.path());
    s.write(|c| {
        for id in 1..=12 {
            let tags: &[&str] = if id % 3 == 0 {
                &["mare", "estate"]
            } else {
                &["montagna"]
            };
            photo(c, id, None, &[], tags);
        }
        Ok(())
    })
    .unwrap();
    let f = LibraryFilter {
        date_from: Some("2021-01-01".into()),
        date_to: Some("2021-12-31".into()),
        tags: vec!["mare".into()],
        ..Default::default()
    };
    let facets = s.read(move |c| query::facets(c, &f)).unwrap();
    // Anni: solo il filtro per tag (4 foto "mare": id 3, 6, 9, 12 → 2023, 2022, 2021, 2020).
    assert_eq!(facets.years.total, 4);
    // Tag: solo il filtro per anno (id 1, 5, 9 → 2021).
    assert_eq!(count_of(&facets.tags.values, "montagna"), Some(2));
    assert_eq!(count_of(&facets.tags.values, "mare"), Some(1));
}

#[test]
fn many_places_return_a_total_and_keep_the_selected_one() {
    let tmp = tempfile::tempdir().unwrap();
    let s = store(tmp.path());
    s.write(|c| {
        let mut id = 0;
        for p in 0..340 {
            // Luoghi con 1..=5 foto; "Luogo 339" ne ha una sola.
            for _ in 0..(5 - p % 5) {
                id += 1;
                photo(c, id, Some(&format!("Luogo {p}")), &[], &[]);
            }
        }
        Ok(())
    })
    .unwrap();
    let all = s
        .read(|c| query::facets(c, &LibraryFilter::default()))
        .unwrap();
    assert_eq!(all.places.total, 340);
    assert_eq!(all.places.values.len(), query::FACET_TOP);
    assert!(all
        .places
        .values
        .windows(2)
        .all(|w| w[0].count >= w[1].count));
    let f = LibraryFilter {
        places: vec!["Luogo 339".into()],
        ..Default::default()
    };
    let chosen = s.read(move |c| query::facets(c, &f)).unwrap();
    assert_eq!(chosen.places.values.len(), query::FACET_TOP + 1);
    assert_eq!(count_of(&chosen.places.values, "Luogo 339"), Some(1));
}

#[test]
fn full_list_search_sort_and_pages() {
    let tmp = tempfile::tempdir().unwrap();
    let s = store(tmp.path());
    s.write(|c| {
        let places = [
            ("Siena", 3),
            ("SIENA", 1),
            ("Città di Castello", 2),
            ("Firenze", 5),
            ("Assisi", 4),
            ("siena est", 1),
        ];
        let mut id = 0;
        for (p, n) in places {
            for _ in 0..n {
                id += 1;
                photo(c, id, Some(p), &[], &["zeta", "Àlfa", "beta"]);
            }
        }
        Ok(())
    })
    .unwrap();
    let page = |key: FacetKey, search: Option<&'static str>, sort: FacetSort, offset, limit| {
        s.read(move |c| {
            query::facet_values(
                c,
                &LibraryFilter::default(),
                &key,
                search,
                sort,
                offset,
                limit,
            )
        })
        .unwrap()
    };
    // Ricerca senza maiuscole e accenti.
    let found = page(FacetKey::Place, Some("sie"), FacetSort::Count, 0, 50);
    assert_eq!(found.total, 3);
    let names: Vec<_> = found.values.iter().map(|v| v.value.as_str()).collect();
    assert_eq!(names, vec!["Siena", "SIENA", "siena est"]);
    assert_eq!(
        page(FacetKey::Place, Some("CITTA"), FacetSort::Count, 0, 50).total,
        1
    );
    // Ordinamento alfabetico (anche dei tag), accenti ignorati.
    let tags = page(FacetKey::Tag, None, FacetSort::Name, 0, 50);
    let names: Vec<_> = tags.values.iter().map(|v| v.value.as_str()).collect();
    assert_eq!(names, vec!["Àlfa", "beta", "zeta"]);
    // Pagine senza duplicati né buchi.
    let full = page(FacetKey::Place, None, FacetSort::Name, 0, 100);
    assert_eq!(full.total, 6);
    let mut joined = Vec::new();
    for offset in (0..6).step_by(2) {
        let p = page(FacetKey::Place, None, FacetSort::Name, offset, 2);
        assert_eq!(p.total, 6);
        joined.extend(p.values);
    }
    assert_eq!(joined, full.values);
    // Per numero di foto.
    let by_count = page(FacetKey::Place, None, FacetSort::Count, 0, 2);
    assert_eq!(by_count.values[0].value, "Firenze");
    assert_eq!(by_count.values[1].value, "Assisi");
}

#[test]
fn full_list_ignores_its_own_selection() {
    let tmp = tempfile::tempdir().unwrap();
    let s = store(tmp.path());
    s.write(|c| {
        three_cities(c);
        Ok(())
    })
    .unwrap();
    let f = LibraryFilter {
        places: vec!["Siena".into()],
        labels: vec![LabelFilter {
            group: "subject".into(),
            values: vec!["ritratto".into()],
        }],
        ..Default::default()
    };
    let p = s
        .read(move |c| query::facet_values(c, &f, &FacetKey::Place, None, FacetSort::Count, 0, 10))
        .unwrap();
    assert_eq!(p.total, 3);
    assert_eq!(count_of(&p.values, "Firenze"), Some(40));
    let f = LibraryFilter {
        labels: vec![LabelFilter {
            group: "subject".into(),
            values: vec!["ritratto".into()],
        }],
        ..Default::default()
    };
    let subject = FacetKey::Label {
        group: "subject".into(),
    };
    let p = s
        .read(move |c| query::facet_values(c, &f, &subject, None, FacetSort::Name, 0, 10))
        .unwrap();
    let names: Vec<_> = p.values.iter().map(|v| v.value.as_str()).collect();
    assert_eq!(names, vec!["paesaggio", "ritratto"]);
}

/// 100.000 foto e 5.000 tag: la prima pagina dell'elenco completo dei tag entro
/// 1 s (misurato solo in release, come gli altri test di prestazioni).
#[test]
fn large_library_first_page_of_tags_is_fast() {
    let tmp = tempfile::tempdir().unwrap();
    let s = store(tmp.path());
    s.write(|c| {
        {
            let mut m = c.prepare(
                "INSERT INTO media_item(id, path, name, ext, size, mtime, added_at, updated_at)
                 VALUES (?1, ?2, ?2, 'jpg', 1, 0, 0, 0)",
            )?;
            let mut t = c.prepare("INSERT INTO tag(id, name) VALUES (?1, ?2)")?;
            let mut mt =
                c.prepare("INSERT OR IGNORE INTO media_tag(media_id, tag_id) VALUES (?1, ?2)")?;
            for tag in 1..=5_000i64 {
                t.execute(params![tag, format!("tag {tag:04}")])?;
            }
            for id in 1..=100_000i64 {
                m.execute(params![id, format!("/f/{id}.jpg")])?;
                mt.execute(params![id, id % 5_000 + 1])?;
                mt.execute(params![id, (id * 7) % 5_000 + 1])?;
            }
        }
        Ok(())
    })
    .unwrap();
    let started = Instant::now();
    let p = s
        .read(|c| {
            query::facet_values(
                c,
                &LibraryFilter::default(),
                &FacetKey::Tag,
                None,
                FacetSort::Count,
                0,
                100,
            )
        })
        .unwrap();
    let elapsed = started.elapsed();
    assert_eq!(p.total, 5_000);
    assert_eq!(p.values.len(), 100);
    let facets_started = Instant::now();
    let f = s
        .read(|c| query::facets(c, &LibraryFilter::default()))
        .unwrap();
    let facets_elapsed = facets_started.elapsed();
    assert_eq!(f.tags.total, 5_000);
    assert_eq!(f.tags.values.len(), query::FACET_TOP);
    if !cfg!(debug_assertions) {
        assert!(elapsed.as_millis() < 1_000, "prima pagina: {elapsed:?}");
        assert!(
            facets_elapsed.as_millis() < 1_000,
            "riepilogo: {facets_elapsed:?}"
        );
    }
}
