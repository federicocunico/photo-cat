use pc_store::model::*;
use pc_store::query::{self, LabelFilter, LibraryFilter, Presence, SortKey};
use pc_store::repo;
use pc_store::{Store, StoreOptions};
use rusqlite::{params, Connection};
use serde_json::json;
use std::time::Instant;

fn store(dir: &std::path::Path) -> Store {
    Store::open(&dir.join("catalog.db"), StoreOptions::default()).unwrap()
}

/// Libreria sintetica: soggetti, luoghi, anni, persone e tag distribuiti in modo deterministico.
fn seed(c: &Connection, n: i64) {
    let subjects = [
        "ritratto",
        "paesaggio",
        "natura",
        "ambiente_urbano",
        "monumento",
    ];
    let places = ["Firenze", "Siena", "Roma", "Milano"];
    let tx = c;
    tx.execute(
        "INSERT INTO person(id, name, created_at) VALUES (1, 'Giulia', 0), (2, 'Marco', 0)",
        [],
    )
    .unwrap();
    tx.execute("INSERT INTO tag(id, name) VALUES (1, 'mare')", [])
        .unwrap();
    {
        let mut m = tx
            .prepare("INSERT INTO media_item(id, path, name, ext, size, mtime, added_at, updated_at) VALUES (?1, ?2, ?3, 'jpg', ?4, 0, ?1, ?1)")
            .unwrap();
        let mut d = tx
            .prepare("INSERT INTO media_dates(media_id, captured_at) VALUES (?1, ?2)")
            .unwrap();
        let mut l = tx
            .prepare("INSERT INTO label(media_id, grp, value, score, origin) VALUES (?1, 'subject', ?2, 0.8, 'auto')")
            .unwrap();
        let mut loc = tx
            .prepare("INSERT INTO location(media_id, name, source) VALUES (?1, ?2, 'gps')")
            .unwrap();
        let mut pc = tx
            .prepare("INSERT INTO media_people(media_id, count) VALUES (?1, ?2)")
            .unwrap();
        let mut face = tx
            .prepare(
                "INSERT INTO face(media_id, bbox, quality, person_id) VALUES (?1, '[]', 1, ?2)",
            )
            .unwrap();
        let mut tg = tx
            .prepare("INSERT INTO media_tag(media_id, tag_id) VALUES (?1, 1)")
            .unwrap();
        for i in 1..=n {
            let name = format!("IMG_{i:06}.JPG");
            m.execute(params![i, format!("D:\\Foto\\{name}"), name, i * 10])
                .unwrap();
            let year = 2018 + (i % 6);
            d.execute(params![
                i,
                format!("{year}-{:02}-{:02} 10:00:00", i % 12 + 1, i % 28 + 1)
            ])
            .unwrap();
            l.execute(params![i, subjects[(i % 5) as usize]]).unwrap();
            loc.execute(params![i, places[(i % 4) as usize]]).unwrap();
            let people = if i % 5 == 0 { 2 } else { 0 };
            pc.execute(params![i, people]).unwrap();
            if i % 5 == 0 {
                face.execute(params![i, if i % 10 == 0 { 1 } else { 2 }])
                    .unwrap();
            }
            if i % 7 == 0 {
                tg.execute(params![i]).unwrap();
            }
        }
    }
}

#[test]
fn combined_filters_and_keyset_pagination_on_100k() {
    let tmp = tempfile::tempdir().unwrap();
    let s = store(tmp.path());
    s.write(|c| {
        seed(c, 100_000);
        Ok(())
    })
    .unwrap();

    let f = LibraryFilter {
        labels: vec![LabelFilter {
            group: "subject".into(),
            values: vec!["ritratto".into()],
        }],
        person_ids: vec![1],
        people: Some(Presence::Present),
        ..Default::default()
    };
    let started = Instant::now();
    let page = s
        .read(|c| query::page(c, &f, SortKey::CapturedDesc, None, 200))
        .unwrap();
    let elapsed = started.elapsed();
    assert!(elapsed.as_millis() < 1000, "prima pagina in {elapsed:?}");
    // ritratto ⇔ i%5==0, Giulia ⇔ i%10==0 → 10.000 risultati.
    assert_eq!(s.read(|c| query::count(c, &f)).unwrap(), 10_000);
    assert_eq!(page.items.len(), 200);

    // Senza filtri: prima pagina sulla libreria completa.
    let started = Instant::now();
    let all = s
        .read(|c| {
            query::page(
                c,
                &LibraryFilter::default(),
                SortKey::CapturedDesc,
                None,
                200,
            )
        })
        .unwrap();
    assert!(
        started.elapsed().as_millis() < 1000,
        "{:?}",
        started.elapsed()
    );

    // Paginazione keyset: nessuna sovrapposizione e ordine decrescente rispettato.
    let second = s
        .read(|c| {
            query::page(
                c,
                &LibraryFilter::default(),
                SortKey::CapturedDesc,
                all.next.as_ref(),
                200,
            )
        })
        .unwrap();
    let last_first = all.items.last().unwrap();
    let first_second = &second.items[0];
    assert!(
        (first_second.captured_at.clone(), first_second.id)
            < (last_first.captured_at.clone(), last_first.id)
    );
    let ids: std::collections::HashSet<_> = all
        .items
        .iter()
        .chain(&second.items)
        .map(|i| i.id)
        .collect();
    assert_eq!(ids.len(), 400);

    // Testo + date + tag.
    let f = LibraryFilter {
        text: Some("Siena".into()),
        tags: vec!["mare".into()],
        date_from: Some("2023-01-01".into()),
        date_to: Some("2023-12-31".into()),
        ..Default::default()
    };
    s.write(|c| {
        for id in [7_i64, 35, 49, 77] {
            repo::fts::refresh(c, id)?;
        }
        Ok(())
    })
    .unwrap();
    // Solo gli elementi con FTS aggiornata e che soddisfano tutto: i=77 → 2018+77%6=2023, Siena (77%4=1), tag (77%7=0)
    let ids = s.read(|c| query::ids(c, &f)).unwrap();
    assert_eq!(ids, vec![77]);

    let facets = s
        .read(|c| {
            query::facets(
                c,
                &LibraryFilter {
                    places: vec!["Roma".into()],
                    ..Default::default()
                },
            )
        })
        .unwrap();
    assert_eq!(
        facets.places,
        vec![query::FacetValue {
            value: "Roma".into(),
            count: 25_000
        }]
    );
    assert_eq!(facets.labels[0].0, "subject");
}

#[test]
fn smart_catalog_matches_rules_and_updates_automatically() {
    let tmp = tempfile::tempdir().unwrap();
    let s = store(tmp.path());
    s.write(|c| {
        seed(c, 100);
        Ok(())
    })
    .unwrap();
    let rules = json!({"op": "and", "rules": [
        {"field": "label.subject", "cmp": "eq", "value": "ritratto"},
        {"field": "year", "cmp": "gte", "value": 2022}
    ]});
    let cid = s
        .write(move |c| {
            repo::catalog::create(c, "Ritratti recenti", CatalogKind::Smart, Some(&rules))
        })
        .unwrap();
    let f = LibraryFilter {
        catalog_id: Some(cid),
        ..Default::default()
    };
    let expected: Vec<i64> = (1..=100)
        .filter(|i| i % 5 == 0 && 2018 + (i % 6) >= 2022)
        .collect();
    assert_eq!(s.read(|c| query::ids(c, &f)).unwrap(), expected);

    // Un nuovo file analizzato che soddisfa le regole compare senza azioni manuali.
    s.write(|c| {
        let id = repo::media::upsert(
            c,
            &NewMedia { path: "D:\\nuova.jpg".into(), name: "nuova.jpg".into(), ext: "jpg".into(), ..Default::default() },
        )?;
        repo::meta::set_dates(c, id, &MediaDates { captured_at: Some("2024-05-01 12:00:00".into()), ..Default::default() })?;
        c.execute("INSERT INTO label(media_id, grp, value, score, origin) VALUES (?1, 'subject', 'ritratto', 0.9, 'auto')", [id])?;
        Ok(())
    })
    .unwrap();
    assert_eq!(
        s.read(|c| query::count(c, &f)).unwrap(),
        expected.len() as i64 + 1
    );

    // Regole OR annidate con persona e tag.
    let f = LibraryFilter {
        rule: Some(json!({"op": "or", "rules": [
            {"field": "person", "cmp": "eq", "value": "giulia"},
            {"field": "tag", "cmp": "in", "value": ["mare"]}
        ]})),
        ..Default::default()
    };
    let expected: Vec<i64> = (1..=100).filter(|i| i % 10 == 0 || i % 7 == 0).collect();
    assert_eq!(s.read(|c| query::ids(c, &f)).unwrap(), expected);
}

#[test]
fn malicious_rules_are_rejected_and_tables_survive() {
    let tmp = tempfile::tempdir().unwrap();
    let s = store(tmp.path());
    let f = LibraryFilter {
        rule: Some(json!({"field": "name", "cmp": "eq", "value": "x'; DROP TABLE media_item; --"})),
        ..Default::default()
    };
    assert_eq!(s.read(|c| query::count(c, &f)).unwrap(), 0);
    let f = LibraryFilter {
        rule: Some(json!({"field": "1=1 --", "cmp": "eq", "value": 1})),
        ..Default::default()
    };
    assert!(s.read(|c| query::count(c, &f)).is_err());
    let f = LibraryFilter {
        text: Some("\" OR * NEAR(".into()),
        ..Default::default()
    };
    assert_eq!(s.read(|c| query::count(c, &f)).unwrap(), 0);
    assert_eq!(s.read(repo::media::count).unwrap(), 0);
}

#[test]
fn grid_items_carry_their_tags() {
    let tmp = tempfile::tempdir().unwrap();
    let db = store(tmp.path());
    db.write(|c| {
        seed(c, 14);
        c.execute("INSERT INTO tag(id, name) VALUES (2, 'Alba')", [])?;
        c.execute("INSERT INTO media_tag(media_id, tag_id) VALUES (7, 2)", [])?;
        Ok(())
    })
    .unwrap();
    let page = db
        .read(|c| query::page(c, &LibraryFilter::default(), SortKey::NameAsc, None, 50))
        .unwrap();
    let tags = |id: i64| page.items.iter().find(|i| i.id == id).unwrap().tags.clone();
    assert_eq!(tags(7), vec!["Alba".to_string(), "mare".to_string()]);
    assert_eq!(tags(14), vec!["mare".to_string()]);
    assert!(tags(1).is_empty());
}

mod folders {
    use super::*;
    use pc_store::query::{FolderFilter, FolderNode};

    /// Sorgente `D:\Foto` con un viaggio e le sottocartelle dei dispositivi.
    fn library() -> (Connection, i64, Vec<i64>) {
        let mut c = Connection::open_in_memory().unwrap();
        pc_store::migrate::migrate(&mut c, None).unwrap();
        let src = repo::source::insert(
            &c,
            &NewSource {
                path: r"D:\Foto".into(),
                recursive: true,
                ..Default::default()
            },
        )
        .unwrap();
        let files = [
            r"D:\Foto\Viaggio Giappone\copertina.jpg",
            r"D:\Foto\Viaggio Giappone\D7000\a.jpg",
            r"D:\Foto\Viaggio Giappone\D7000\b.jpg",
            r"D:\Foto\Viaggio Giappone\D7000\Scarti\c.jpg",
            r"D:\Foto\Viaggio Giappone\Telefono\d.jpg",
            r"D:\Foto\Viaggio Giappone 2\e.jpg",
            r"D:\Foto\50%_sconto\f.jpg",
            r"D:\Foto\50x_sconto\g.jpg",
            r"D:\Foto\radice.jpg",
        ];
        let ids = files
            .iter()
            .map(|p| {
                repo::media::upsert(
                    &c,
                    &NewMedia {
                        source_id: Some(src),
                        path: p.to_string(),
                        name: p.rsplit('\\').next().unwrap().into(),
                        ext: "jpg".into(),
                        ..Default::default()
                    },
                )
                .unwrap()
            })
            .collect();
        (c, src, ids)
    }

    fn in_folder(c: &Connection, path: &str, recursive: bool) -> Vec<i64> {
        let mut v = query::ids(
            c,
            &LibraryFilter {
                folder: Some(FolderFilter {
                    path: path.into(),
                    recursive,
                }),
                ..Default::default()
            },
        )
        .unwrap();
        v.sort();
        v
    }

    #[test]
    fn folder_filter_with_and_without_subfolders() {
        let (c, _, ids) = library();
        // Scenario "Filtro per cartella con sottocartelle".
        assert_eq!(
            in_folder(&c, r"D:\Foto\Viaggio Giappone", true),
            ids[..5].to_vec()
        );
        // Separatore finale e prefisso di un'altra cartella ("Viaggio Giappone 2").
        assert_eq!(
            in_folder(&c, r"D:\Foto\Viaggio Giappone\", true),
            ids[..5].to_vec()
        );
        // Scenario "Filtro per cartella esatta".
        assert_eq!(
            in_folder(&c, r"D:\Foto\Viaggio Giappone\D7000", false),
            vec![ids[1], ids[2]]
        );
        // `%` e `_` sono caratteri normali.
        assert_eq!(in_folder(&c, r"D:\Foto\50%_sconto", true), vec![ids[6]]);
        // "Seleziona tutto" usa gli stessi id del filtro.
        let count = query::count(
            &c,
            &LibraryFilter {
                folder: Some(FolderFilter {
                    path: r"D:\Foto\Viaggio Giappone".into(),
                    recursive: true,
                }),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(count, 5);
        if cfg!(windows) {
            assert_eq!(
                in_folder(&c, r"d:\foto\viaggio giappone\d7000", false).len(),
                2
            );
        } else {
            assert!(in_folder(&c, r"d:\foto\viaggio giappone\d7000", false).is_empty());
        }
    }

    #[test]
    fn folder_tree_one_level_at_a_time_with_counts() {
        let (c, src, _) = library();
        let node = |n: &FolderNode| (n.name.clone(), n.items, n.has_children);
        let top: Vec<_> = query::list_folders(&c, src, None)
            .unwrap()
            .iter()
            .map(node)
            .collect();
        assert_eq!(
            top,
            vec![
                ("50%_sconto".to_string(), 1, false),
                ("50x_sconto".to_string(), 1, false),
                ("Viaggio Giappone".to_string(), 5, true),
                ("Viaggio Giappone 2".to_string(), 1, false),
            ]
        );
        let trip = query::list_folders(&c, src, Some(r"D:\Foto\Viaggio Giappone")).unwrap();
        assert_eq!(trip[0].path, r"D:\Foto\Viaggio Giappone\D7000");
        assert_eq!(
            trip.iter().map(node).collect::<Vec<_>>(),
            vec![
                ("D7000".to_string(), 3, true),
                ("Telefono".to_string(), 1, false),
            ]
        );
    }

    #[test]
    fn folder_filter_uses_the_path_index() {
        let (c, _, _) = library();
        let frag = query::compile_filter(
            &c,
            &LibraryFilter {
                folder: Some(FolderFilter {
                    path: r"D:\Foto\Viaggio Giappone".into(),
                    recursive: true,
                }),
                ..Default::default()
            },
        )
        .unwrap();
        let sql = format!(
            "EXPLAIN QUERY PLAN SELECT m.id FROM media_item m WHERE {}",
            frag.sql
        );
        let mut q = c.prepare(&sql).unwrap();
        let plan: Vec<String> = q
            .query_map(rusqlite::params_from_iter(frag.args), |r| {
                r.get::<_, String>(3)
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert!(
            plan.iter()
                .any(|p| p.contains("INDEX sqlite_autoindex_media_item") && p.contains("path>")),
            "{plan:?}"
        );
    }
}
