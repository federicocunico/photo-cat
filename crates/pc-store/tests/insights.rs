use pc_store::analysis::{self, AutoLabel};
use pc_store::insights::{self, SizeBucket};
use pc_store::model::CatalogKind;
use pc_store::query::{LibraryFilter, Target};
use pc_store::{bulk, repo, Store, StoreOptions};
use rusqlite::{params, Connection};
use std::time::Instant;

fn store(dir: &std::path::Path) -> Store {
    Store::open(&dir.join("catalog.db"), StoreOptions::default()).unwrap()
}

fn photo(c: &Connection, id: i64, date: Option<&str>) {
    c.execute(
        "INSERT INTO media_item(id, path, name, ext, size, mtime, added_at, updated_at, thumb) VALUES (?1, ?2, ?2, 'jpg', 1, 0, ?1, ?1, 't')",
        params![id, format!("/p/{id}.jpg")],
    )
    .unwrap();
    if let Some(d) = date {
        c.execute(
            "INSERT INTO media_dates(media_id, captured_at) VALUES (?1, ?2)",
            params![id, d],
        )
        .unwrap();
    }
}

fn label(c: &Connection, id: i64, value: &str, score: f64, origin: &str) {
    c.execute(
        "INSERT INTO label(media_id, grp, value, score, origin, model_ver, taxonomy_ver) VALUES (?1, 'subject', ?2, ?3, ?4, 'm1', 3)",
        params![id, value, score, origin],
    )
    .unwrap();
}

fn labels(c: &Connection, id: i64) -> Vec<(String, f64, String)> {
    let mut q = c
        .prepare("SELECT value, score, origin FROM label WHERE media_id=?1 ORDER BY value")
        .unwrap();
    q.query_map([id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

#[test]
fn histogram_bins_edges() {
    let d = tempfile::tempdir().unwrap();
    let s = store(d.path());
    s.write(|c| {
        for (i, sc) in [0.0, 0.05, 0.049, 0.99, 1.0, 0.5].iter().enumerate() {
            photo(c, i as i64 + 1, None);
            label(c, i as i64 + 1, "natura", *sc, "auto");
        }
        // manuale e altro valore: esclusi
        photo(c, 7, None);
        label(c, 7, "natura", 1.0, "manual");
        label(c, 1, "ritratto", 0.5, "auto");
        let h = insights::label_histogram(c, &LibraryFilter::default(), "subject", "natura", 20)?;
        assert_eq!(h.len(), 20);
        assert_eq!(h[0].count, 2); // 0.0 e 0.049
        assert_eq!(h[1].count, 1); // 0.05
        assert_eq!(h[10].count, 1); // 0.5
        assert_eq!(h[19].count, 2); // 0.99 e 1.0
        assert_eq!(h.iter().map(|b| b.count).sum::<i64>(), 6);
        assert!((h[1].from - 0.05).abs() < 1e-9 && (h[19].to - 1.0).abs() < 1e-9);
        Ok(())
    })
    .unwrap();
}

#[test]
fn histogram_respects_filter() {
    let d = tempfile::tempdir().unwrap();
    let s = store(d.path());
    s.write(|c| {
        for i in 1..=4 {
            photo(c, i, None);
            label(c, i, "natura", 0.8, "auto");
        }
        let cat = repo::catalog::create(c, "A", CatalogKind::Manual, None)?;
        repo::catalog::add_items(c, cat, &[1, 2])?;
        let f = LibraryFilter {
            catalog_id: Some(cat),
            ..Default::default()
        };
        let h = insights::label_histogram(c, &f, "subject", "natura", 10)?;
        assert_eq!(h[8].count, 2);
        Ok(())
    })
    .unwrap();
}

#[test]
fn month_coverage_with_undated() {
    let d = tempfile::tempdir().unwrap();
    let s = store(d.path());
    s.write(|c| {
        photo(c, 1, Some("2024-01-05 10:00:00"));
        photo(c, 2, Some("2024-01-20 10:00:00"));
        photo(c, 3, Some("2023-12-31 23:00:00"));
        photo(c, 4, None);
        photo(c, 5, None);
        for id in [1, 4] {
            c.execute(
                "INSERT INTO analysis_version(media_id, stage, model_ver, taxonomy_ver) VALUES (?1,'clip','v',1)",
                [id],
            )?;
        }
        let m = insights::month_coverage(c, &LibraryFilter::default())?;
        let got: Vec<_> = m
            .iter()
            .map(|x| (x.month.clone(), x.total, x.analyzed))
            .collect();
        assert_eq!(
            got,
            vec![
                (Some("2023-12".into()), 1, 0),
                (Some("2024-01".into()), 2, 1),
                (None, 2, 1)
            ]
        );
        Ok(())
    })
    .unwrap();
}

#[test]
fn similar_stats_buckets() {
    let d = tempfile::tempdir().unwrap();
    let s = store(d.path());
    s.write(|c| {
        let sizes = [2, 2, 4, 7, 12];
        let mut id = 0;
        for (g, n) in sizes.iter().enumerate() {
            c.execute(
                "INSERT INTO similar_group(id, params, updated_at) VALUES (?1,'p',0)",
                [g as i64 + 1],
            )?;
            for _ in 0..*n {
                id += 1;
                photo(c, id, None);
                c.execute(
                    "INSERT INTO similar_member(media_id, group_id, score) VALUES (?1,?2,1.0)",
                    params![id, g as i64 + 1],
                )?;
            }
        }
        let st = insights::similar_stats(c, None)?;
        assert_eq!((st.groups, st.grouped_photos), (5, 27));
        let b: Vec<_> = st
            .size_buckets
            .iter()
            .map(|b| (b.label.as_str(), b.groups, b.photos))
            .collect();
        assert_eq!(
            b,
            vec![("2", 2, 4), ("3-5", 1, 4), ("6-10", 1, 7), (">10", 1, 12)]
        );
        assert_eq!(SizeBucket::of(11), Some(SizeBucket::Over10));
        // filtro: foto 1..=3 (gruppo 1 intero, 1 sola del gruppo 2 che quindi non conta)
        let f = LibraryFilter {
            ids: Some((1..=3).collect()),
            ..Default::default()
        };
        let st = insights::similar_stats(c, Some(&f))?;
        assert_eq!((st.groups, st.grouped_photos), (1, 2));
        Ok(())
    })
    .unwrap();
}

fn reject_setup(c: &Connection) -> i64 {
    for i in 1..=4 {
        photo(c, i, None);
    }
    label(c, 1, "natura", 0.30, "auto");
    label(c, 2, "natura", 0.90, "auto");
    label(c, 3, "natura", 0.10, "auto"); // fuori catalogo
    label(c, 4, "natura", 1.0, "manual");
    label(c, 1, "ritratto", 0.2, "auto");
    let cat = repo::catalog::create(c, "A", CatalogKind::Manual, None).unwrap();
    repo::catalog::add_items(c, cat, &[1, 2, 4]).unwrap();
    cat
}

#[test]
fn reject_below_threshold_in_catalog_and_undo() {
    let d = tempfile::tempdir().unwrap();
    let s = store(d.path());
    s.write(|c| {
        let cat = reject_setup(c);
        let target = Target::Filter {
            filter: LibraryFilter {
                catalog_id: Some(cat),
                ..Default::default()
            },
            max_id: 100,
            except: vec![],
        };
        assert_eq!(
            bulk::reject_preview(c, "subject", "natura", Some(0.5), &target)?,
            1
        );
        assert_eq!(
            bulk::reject_preview(c, "subject", "natura", None, &target)?,
            2
        );
        let ids = target.resolve(c)?;
        let out = bulk::with_undo_group(c, "x", || {
            bulk::reject_auto_labels(c, "subject", "natura", Some(0.5), &ids)
        })?;
        assert_eq!(out.changed, 1);
        // 1: tolta solo natura; 2 (sopra soglia), 3 (fuori catalogo), 4 (manuale) intatte
        assert_eq!(labels(c, 1).len(), 1);
        assert_eq!(labels(c, 2).len(), 1);
        assert_eq!(labels(c, 3).len(), 1);
        assert_eq!(labels(c, 4), vec![("natura".into(), 1.0, "manual".into())]);
        assert_eq!(analysis::rejected_values(c, 1, "subject")?, vec!["natura"]);
        assert!(analysis::rejected_values(c, 3, "subject")?.is_empty());

        // una nuova classificazione non la rimette
        let groups = vec!["subject".to_string()];
        let al = [AutoLabel {
            group: "subject".into(),
            value: "natura".into(),
            score: 0.95,
        }];
        analysis::classification(c, 1, &groups, &al, &[0.1; 512], "m1", 3)?;
        assert!(labels(c, 1).iter().all(|l| l.0 != "natura"));

        // annulla: righe e rifiuti tornano
        assert!(bulk::undo_last(c)?.is_some());
        assert!(labels(c, 1).contains(&("natura".into(), 0.30, "auto".into())));
        let (mv, tv): (Option<String>, Option<i64>) = c.query_row(
            "SELECT model_ver, taxonomy_ver FROM label WHERE media_id=1 AND value='natura'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        assert_eq!((mv.as_deref(), tv), (Some("m1"), Some(3)));
        assert!(analysis::rejected_values(c, 1, "subject")?.is_empty());
        Ok(())
    })
    .unwrap();
}

#[test]
#[ignore]
fn perf_100k() {
    let d = tempfile::tempdir().unwrap();
    let s = store(d.path());
    s.write(|c| {
        for i in 1..=100_000i64 {
            let date = format!("{}-{:02}-01 00:00:00", 2010 + i % 15, 1 + i % 12);
            photo(c, i, (i % 20 != 0).then_some(date.as_str()));
            label(c, i, "natura", (i % 1000) as f64 / 1000.0, "auto");
            if i % 3 == 0 {
                c.execute(
                    "INSERT INTO analysis_version(media_id, stage, model_ver, taxonomy_ver) VALUES (?1,'clip','v',1)",
                    [i],
                )?;
            }
        }
        for g in 0..20_000i64 {
            c.execute(
                "INSERT INTO similar_group(id, params, updated_at) VALUES (?1,'p',0)",
                [g + 1],
            )?;
            for k in 0..3 {
                c.execute(
                    "INSERT INTO similar_member(media_id, group_id, score) VALUES (?1,?2,1.0)",
                    params![g * 3 + k + 1, g + 1],
                )?;
            }
        }
        let f = LibraryFilter::default();
        let t = Instant::now();
        insights::label_histogram(c, &f, "subject", "natura", 20)?;
        let (a, t) = (t.elapsed(), Instant::now());
        insights::month_coverage(c, &f)?;
        let (b, t) = (t.elapsed(), Instant::now());
        insights::similar_stats(c, None)?;
        let (d2, t) = (t.elapsed(), Instant::now());
        insights::similar_stats(c, Some(&f))?;
        let e = t.elapsed();
        eprintln!("histogram {a:?} months {b:?} similar {d2:?} similar(filter) {e:?}");
        for x in [a, b, d2, e] {
            assert!(x.as_millis() < 300, "{x:?}");
        }
        Ok(())
    })
    .unwrap();
}
