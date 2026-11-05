use pc_store::geo_map::{self, GeoBbox};
use pc_store::query::LibraryFilter;
use pc_store::{Store, StoreOptions};
use rusqlite::{params, Connection};
use std::time::Instant;

fn store(dir: &std::path::Path) -> Store {
    Store::open(&dir.join("catalog.db"), StoreOptions::default()).unwrap()
}

fn photo(c: &Connection, id: i64, name: Option<&str>, source: &str, pos: Option<(f64, f64)>) {
    c.execute(
        "INSERT INTO media_item(id, path, name, ext, size, mtime, added_at, updated_at)
         VALUES (?1, ?2, ?3, 'jpg', 1, 0, ?1, ?1)",
        params![id, format!("/p/{id}.jpg"), format!("{id}.jpg")],
    )
    .unwrap();
    c.execute(
        "INSERT INTO location(media_id, name, lat, lon, source) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, name, pos.map(|p| p.0), pos.map(|p| p.1), source],
    )
    .unwrap();
}

fn total(cells: &[geo_map::Cell]) -> i64 {
    cells.iter().map(|c| c.count).sum()
}

fn seed_mixed(c: &Connection) {
    // 3 GPS a Roma, 2 GPS a Tokyo, 2 per nome (Lisbona collocata), 1 per nome non collocato.
    for (i, (la, lo)) in [(41.89, 12.49), (41.90, 12.50), (41.88, 12.48)]
        .iter()
        .enumerate()
    {
        photo(c, 1 + i as i64, Some("Roma"), "gps", Some((*la, *lo)));
    }
    photo(c, 4, Some("Tokyo"), "gps", Some((35.68, 139.69)));
    photo(c, 5, Some("Tokyo"), "gps", Some((35.69, 139.70)));
    photo(c, 6, Some("Lisbona"), "directory", None);
    photo(c, 7, Some("Lisbona"), "directory", None);
    photo(c, 8, Some("Atlantide"), "directory", None);
    geo_map::map_place_set(c, "Lisbona", 38.72, -9.14).unwrap();
}

#[test]
fn cells_sum_invariant_across_zooms_and_approx() {
    let d = tempfile::tempdir().unwrap();
    let s = store(d.path());
    s.write(|c| {
        seed_mixed(c);
        Ok(())
    })
    .unwrap();
    let f = LibraryFilter::default();
    s.read(|c| {
        for z in [0, 3, 8, 14] {
            let cells = geo_map::map_cells(c, &f, &GeoBbox::WORLD, z, true)?;
            assert_eq!(total(&cells), 7, "zoom {z}");
            let gps = geo_map::map_cells(c, &f, &GeoBbox::WORLD, z, false)?;
            assert_eq!(total(&gps), 5, "solo GPS, zoom {z}");
            assert!(gps.iter().all(|x| x.approx_count == 0));
        }
        let world = geo_map::map_cells(c, &f, &GeoBbox::WORLD, 0, true)?;
        assert_eq!(world.iter().map(|x| x.approx_count).sum::<i64>(), 2);
        let deep = geo_map::map_cells(c, &f, &GeoBbox::WORLD, 14, true)?;
        assert!(deep.len() >= 4);
        let rome = deep
            .iter()
            .find(|x| x.place.as_deref() == Some("Roma"))
            .unwrap();
        assert!(rome.sample_ids.len() <= 4 && !rome.sample_ids.is_empty());
        assert!((rome.lat - 41.89).abs() < 0.05);
        Ok(())
    })
    .unwrap();
}

#[test]
fn gps_photos_ignore_place_geo() {
    let d = tempfile::tempdir().unwrap();
    let s = store(d.path());
    s.write(|c| {
        photo(c, 1, Some("Roma"), "gps", Some((41.89, 12.49)));
        // Il nome ha una posizione diversa (approssimata): la foto GPS non la usa.
        geo_map::map_place_set(c, "Roma", -33.0, 151.0).unwrap();
        Ok(())
    })
    .unwrap();
    s.read(|c| {
        let cells = geo_map::map_cells(c, &LibraryFilter::default(), &GeoBbox::WORLD, 6, true)?;
        assert_eq!(cells.len(), 1);
        assert_eq!(cells[0].approx_count, 0);
        assert!((cells[0].lat - 41.89).abs() < 1e-9);
        Ok(())
    })
    .unwrap();
}

#[test]
fn bbox_is_inclusive_and_wraps_antimeridian() {
    let d = tempfile::tempdir().unwrap();
    let s = store(d.path());
    s.write(|c| {
        seed_mixed(c);
        photo(c, 9, Some("Fiji"), "gps", Some((-17.7, 179.5)));
        photo(c, 10, Some("Samoa"), "gps", Some((-13.8, -171.8)));
        Ok(())
    })
    .unwrap();
    s.read(|c| {
        let f = LibraryFilter::default();
        let rome = GeoBbox {
            south: 41.88,
            west: 12.48,
            north: 41.90,
            east: 12.50,
        };
        assert_eq!(total(&geo_map::map_cells(c, &f, &rome, 5, true)?), 3);
        let pacific = GeoBbox {
            south: -30.0,
            west: 170.0,
            north: 0.0,
            east: -170.0,
        };
        assert_eq!(total(&geo_map::map_cells(c, &f, &pacific, 5, true)?), 2);
        Ok(())
    })
    .unwrap();
}

#[test]
fn catalogs_counts_in_area_and_unplaced_list() {
    let d = tempfile::tempdir().unwrap();
    let s = store(d.path());
    s.write(|c| {
        seed_mixed(c);
        c.execute_batch(
            "INSERT INTO catalog(id, name, kind, created_at) VALUES (100, 'Viaggio', 'manual', 0);
             INSERT INTO catalog_item VALUES (100, 1), (100, 2), (100, 4), (100, 8);",
        )
        .unwrap();
        Ok(())
    })
    .unwrap();
    s.read(|c| {
        let f = LibraryFilter::default();
        let rome = GeoBbox {
            south: 40.0,
            west: 10.0,
            north: 43.0,
            east: 14.0,
        };
        let cats = geo_map::map_catalogs(c, &f, &rome, true)?;
        assert_eq!(cats.len(), 1);
        assert_eq!((cats[0].name.as_str(), cats[0].count), ("Viaggio", 2));
        let all = geo_map::map_catalogs(c, &f, &GeoBbox::WORLD, true)?;
        assert_eq!(all[0].count, 3, "la foto 8 non ha posizione");
        let un = geo_map::map_places_unplaced(c, &f)?;
        assert_eq!(un.len(), 1);
        assert_eq!((un[0].name.as_str(), un[0].count), ("Atlantide", 1));
        Ok(())
    })
    .unwrap();
}

#[test]
fn place_set_clear_and_auto_never_overwrites_manual() {
    let d = tempfile::tempdir().unwrap();
    let s = store(d.path());
    s.write(|c| {
        photo(c, 1, Some("Siena"), "directory", None);
        assert_eq!(geo_map::names_to_geocode(c).unwrap(), vec!["Siena"]);
        geo_map::map_place_set(c, "Siena", 43.3, 11.3).unwrap();
        assert!(geo_map::names_to_geocode(c).unwrap().is_empty());
        assert!(!geo_map::place_geo_insert_auto(c, "Siena", 0.0, 0.0, None).unwrap());
        assert!(geo_map::map_place_set(c, "Siena", 200.0, 0.0).is_err());
        assert!(geo_map::map_place_clear(c, "Siena").unwrap());
        assert!(!geo_map::map_place_clear(c, "Siena").unwrap());
        assert_eq!(geo_map::names_to_geocode(c).unwrap().len(), 1);
        Ok(())
    })
    .unwrap();
}

/// 100.000 foto con posizione: `map_cells` < 150 ms a zoom mondiale e regionale.
/// `cargo test -p pc-store --release --test geo_map -- --ignored --nocapture`
#[test]
#[ignore]
fn perf_map_cells_100k() {
    let d = tempfile::tempdir().unwrap();
    let s = store(d.path());
    s.write(|c| {
        let mut seed = 0x1234_5678_9abc_def0u64;
        let mut rnd = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed >> 11) as f64 / (1u64 << 53) as f64
        };
        for i in 1..=100_000i64 {
            // 70% GPS (cluster attorno a 40 centri), 30% per nome (500 luoghi).
            let name = format!("Luogo {}", i % 500);
            if i % 10 < 7 {
                let k = (i % 40) as f64;
                let (la, lo) = (
                    (-60.0 + k * 3.0) + rnd() * 2.0,
                    (-170.0 + k * 8.0) + rnd() * 2.0,
                );
                photo(c, i, Some(&name), "gps", Some((la, lo)));
            } else {
                photo(c, i, Some(&name), "directory", None);
            }
        }
        for n in 0..500 {
            geo_map::map_place_set(
                c,
                &format!("Luogo {n}"),
                -60.0 + (n % 120) as f64,
                -170.0 + (n % 340) as f64,
            )
            .unwrap();
        }
        Ok(())
    })
    .unwrap();
    let f = LibraryFilter::default();
    let regional = GeoBbox {
        south: -20.0,
        west: -60.0,
        north: 20.0,
        east: 60.0,
    };
    s.read(|c| {
        for (label, bbox, zoom) in [("mondo", GeoBbox::WORLD, 1), ("regione", regional, 5)] {
            // Un giro a vuoto (cache di pagine), poi la misura.
            geo_map::map_cells(c, &f, &bbox, zoom, true)?;
            let t = Instant::now();
            let cells = geo_map::map_cells(c, &f, &bbox, zoom, true)?;
            let ms = t.elapsed().as_millis();
            println!(
                "map_cells {label} zoom {zoom}: {ms} ms, {} celle",
                cells.len()
            );
            assert!(ms < 150, "{label}: {ms} ms");
        }
        Ok(())
    })
    .unwrap();
}

fn geo_filter(b: GeoBbox, approx: bool) -> LibraryFilter {
    LibraryFilter {
        geo_bbox: Some(pc_store::query::GeoBboxFilter {
            bbox: b,
            approx,
            label: "Area".into(),
        }),
        ..Default::default()
    }
}

#[test]
fn geo_bbox_filter_matches_cell_and_page_count_selection_agree() {
    use pc_store::query::{count, page, selection_base, SortKey};
    let d = tempfile::tempdir().unwrap();
    let s = store(d.path());
    s.write(|c| {
        seed_mixed(c);
        // Foto sui bordi delle celle (0°, 45°, ±180°) e oltre il limite Mercator.
        for (i, (la, lo)) in [
            (0.0, 0.0),
            (0.0, -0.0),
            (-0.000001, 45.0),
            (10.0, 180.0),
            (10.0, -180.0),
            (89.0, 10.0),
            (-89.0, 10.0),
            (66.513_260_443_111_86, 90.0),
            (41.0, 11.25),
            (41.0, 11.249_999_999),
        ]
        .iter()
        .enumerate()
        {
            photo(c, 20 + i as i64, Some("Bordo"), "gps", Some((*la, *lo)));
        }
        Ok(())
    })
    .unwrap();
    s.read(|c| {
        for z in [0, 1, 2, 5, 9, 14, 20] {
            let cells = geo_map::map_cells(c, &LibraryFilter::default(), &GeoBbox::WORLD, z, true)?;
            assert_eq!(total(&cells), 17, "zoom {z}");
            for cell in &cells {
                // Il bbox esatto della cella riproduce il conteggio della cella a ogni zoom.
                let f = geo_filter(cell.bbox, true);
                let n = count(c, &f)?;
                assert_eq!(n, cell.count, "zoom {z} cella {:?}", cell.bbox);
                assert_eq!(cell.bbox, geo_map::cell_bbox(cell.cx, cell.cy, z));
                let p = page(c, &f, SortKey::default(), None, 100)?;
                assert_eq!(p.items.len() as i64, n);
                assert_eq!(selection_base(c, &f)?.count, n);
                // Solo GPS: escluse le approssimate della cella.
                let g = count(c, &geo_filter(cell.bbox, false))?;
                assert_eq!(g, cell.count - cell.approx_count);
            }
        }
        // Antimeridiano (west > east): Tokyo è dentro solo se west <= 139.69.
        let anti = GeoBbox {
            south: 35.0,
            west: 170.0,
            north: 36.0,
            east: -170.0,
        };
        assert_eq!(count(c, &geo_filter(anti, true))?, 0);
        let anti = GeoBbox {
            south: 35.0,
            west: 139.0,
            north: 36.0,
            east: -170.0,
        };
        assert_eq!(count(c, &geo_filter(anti, true))?, 2);
        Ok(())
    })
    .unwrap();
}

#[test]
fn view_edge_cells_keep_full_count() {
    let d = tempfile::tempdir().unwrap();
    let s = store(d.path());
    s.write(|c| {
        seed_mixed(c);
        Ok(())
    })
    .unwrap();
    s.read(|c| {
        let f = LibraryFilter::default();
        // Vista che taglia la cella di Roma (zoom 2: celle di 11,25°) a metà delle foto.
        let half = GeoBbox {
            south: 41.885,
            west: 12.485,
            north: 60.0,
            east: 30.0,
        };
        let cells = geo_map::map_cells(c, &f, &half, 2, true)?;
        let rome = cells
            .iter()
            .find(|x| x.place.as_deref() == Some("Roma"))
            .unwrap();
        assert_eq!(rome.count, 3, "la cella resta intera");
        // Antimeridiano con le due estremità nella stessa cella: tutto il mondo.
        let wrap = GeoBbox {
            south: -90.0,
            west: 179.9,
            north: 90.0,
            east: 179.8,
        };
        assert_eq!(total(&geo_map::map_cells(c, &f, &wrap, 2, true)?), 7);
        Ok(())
    })
    .unwrap();
}

#[test]
fn cell_bbox_tiles_the_world() {
    for z in [0, 3, 10, 24] {
        let last = 8i64 << z;
        let b = geo_map::cell_bbox(0, 0, z);
        assert_eq!((b.west, b.north), (-180.0, 90.0));
        let b = geo_map::cell_bbox(last - 1, last - 1, z);
        assert_eq!((b.east, b.south), (180.0, -90.0));
        for k in [1, last / 3, last / 2, last - 1] {
            // Celle adiacenti: nessun buco e nessuna sovrapposizione in f64.
            let (a, b) = (
                geo_map::cell_bbox(k - 1, k - 1, z),
                geo_map::cell_bbox(k, k, z),
            );
            assert_eq!(a.east.next_up(), b.west, "zoom {z} k {k}");
            assert_eq!(a.south.next_down(), b.north, "zoom {z} k {k}");
            assert_eq!(geo_map::cell_x(b.west, z), k);
            assert_eq!(geo_map::cell_x(a.east, z), k - 1);
            assert_eq!(geo_map::cell_y(b.north, z), k);
            assert_eq!(geo_map::cell_y(a.south, z), k - 1);
        }
    }
    // Il bordo Mercator è y = 0 (nessuna riga vuota in cima a zoom alti).
    assert_eq!(geo_map::cell_y(geo_map::MAX_LAT, 24), 0);
    assert_eq!(geo_map::cell_y(-geo_map::MAX_LAT, 24), (8i64 << 24) - 1);
}

#[test]
fn geo_bbox_filter_combines_with_catalog() {
    use pc_store::query::count;
    let d = tempfile::tempdir().unwrap();
    let s = store(d.path());
    s.write(|c| {
        seed_mixed(c);
        c.execute_batch(
            "INSERT INTO catalog(id, name, kind, created_at) VALUES (100, 'Viaggio', 'manual', 0);
             INSERT INTO catalog_item VALUES (100, 1), (100, 4), (100, 6);",
        )
        .unwrap();
        Ok(())
    })
    .unwrap();
    s.read(|c| {
        let rome = GeoBbox {
            south: 41.0,
            west: 12.0,
            north: 42.0,
            east: 13.0,
        };
        let mut f = geo_filter(rome, true);
        f.catalog_id = Some(100);
        assert_eq!(count(c, &f)?, 1);
        let all = GeoBbox {
            south: -90.0,
            west: -180.0,
            north: 90.0,
            east: 180.0,
        };
        let mut f = geo_filter(all, true);
        f.catalog_id = Some(100);
        assert_eq!(count(c, &f)?, 3);
        f.geo_bbox.as_mut().unwrap().approx = false;
        assert_eq!(count(c, &f)?, 2);
        Ok(())
    })
    .unwrap();
}

#[test]
fn smart_catalog_with_tech_rule_is_counted() {
    use pc_store::model::CatalogKind;
    use pc_store::repo;
    let d = tempfile::tempdir().unwrap();
    let s = store(d.path());
    s.write(|c| {
        seed_mixed(c);
        c.execute_batch("INSERT INTO media_tech(media_id, width) VALUES (1, 4000), (2, 50);")
            .unwrap();
        // Regole su `t.width` e sull'anno (`d.`): richiedono i join della libreria.
        repo::catalog::create(
            c,
            "Grandi",
            CatalogKind::Smart,
            Some(&serde_json::json!({"field":"width","cmp":"gt","value":100})),
        )?;
        repo::catalog::create(
            c,
            "Recenti",
            CatalogKind::Smart,
            Some(&serde_json::json!({"field":"year","cmp":"gte","value":2000})),
        )?;
        Ok(())
    })
    .unwrap();
    s.read(|c| {
        let cats = geo_map::map_catalogs(c, &LibraryFilter::default(), &GeoBbox::WORLD, true)?;
        assert_eq!(cats.len(), 1);
        assert_eq!((cats[0].name.as_str(), cats[0].count), ("Grandi", 1));
        Ok(())
    })
    .unwrap();
}

#[test]
fn manual_places_flag_list_and_thumbs() {
    let d = tempfile::tempdir().unwrap();
    let s = store(d.path());
    s.write(|c| {
        seed_mixed(c);
        c.execute("UPDATE media_item SET thumb = 'k' || id WHERE id <> 7", [])
            .unwrap();
        geo_map::map_place_set(c, "Vuoto", 1.0, 1.0).unwrap();
        Ok(())
    })
    .unwrap();
    s.read(|c| {
        let f = LibraryFilter::default();
        let cells = geo_map::map_cells(c, &f, &GeoBbox::WORLD, 6, true)?;
        let lis = cells
            .iter()
            .find(|x| x.place.as_deref() == Some("Lisbona"))
            .unwrap();
        assert!(lis.manual);
        assert_eq!(lis.sample_ids, vec![6, 7]);
        assert_eq!(lis.sample_thumbs, vec![Some("k6".into()), None]);
        let rome = cells
            .iter()
            .find(|x| x.place.as_deref() == Some("Roma"))
            .unwrap();
        assert!(!rome.manual);
        assert_eq!(rome.sample_thumbs.len(), rome.sample_ids.len());
        let m = geo_map::map_places_manual(c, &f)?;
        let got: Vec<(&str, i64)> = m.iter().map(|p| (p.name.as_str(), p.count)).collect();
        assert_eq!(got, vec![("Lisbona", 2), ("Vuoto", 0)]);
        assert_eq!((m[0].lat, m[0].lon), (38.72, -9.14));
        Ok(())
    })
    .unwrap();
}
