use pc_ai::geo::Gazetteer;
use pc_ai::place_geo::run_with;
use pc_jobs::{JobControl, JobProgress, PhaseSpec};
use pc_store::{Store, StoreOptions};

const TSV: &str = "2267057\tLisbon\tLisbona\t38.71667\t-9.13333\tPT\t14\t517802\n\
3169070\tRome\tRoma\t41.89193\t12.51133\tIT\t07\t2318895\n";

fn setup() -> (tempfile::TempDir, Store) {
    let d = tempfile::tempdir().unwrap();
    let s = Store::open(&d.path().join("c.db"), StoreOptions::default()).unwrap();
    s.write(|c| {
        for (i, (name, src)) in [
            ("Lisbona", "directory"),
            ("Atlantide", "directory"),
            ("Roma", "manual"),
            ("Parigi", "gps"),
        ]
        .iter()
        .enumerate()
        {
            let id = i as i64 + 1;
            c.execute(
                "INSERT INTO media_item(id, path, name, ext, size, mtime, added_at, updated_at)
                 VALUES (?1, ?2, 'a.jpg', 'jpg', 1, 0, 0, 0)",
                rusqlite::params![id, format!("/p/{id}.jpg")],
            )?;
            c.execute(
                "INSERT INTO location(media_id, name, source) VALUES (?1, ?2, ?3)",
                rusqlite::params![id, name, src],
            )?;
        }
        Ok(())
    })
    .unwrap();
    (d, s)
}

fn run(s: &Store, g: Option<&Gazetteer>) -> usize {
    let phase = JobProgress::new(&[PhaseSpec::new("luoghi", 1.0)]).phase("luoghi");
    run_with(s, g, &JobControl::new(), &phase).unwrap().0
}

fn row(s: &Store, name: &str) -> Option<(f64, f64, String)> {
    s.read(|c| {
        Ok(c.query_row(
            "SELECT lat, lon, source FROM place_geo WHERE name = ?1",
            [name],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .ok())
    })
    .unwrap()
}

#[test]
fn known_name_gets_coordinates_unknown_and_gps_do_not() {
    let (_d, s) = setup();
    let g = Gazetteer::parse(TSV).unwrap();
    assert_eq!(run(&s, Some(&g)), 2);
    let (lat, lon, src) = row(&s, "Lisbona").unwrap();
    assert!((lat - 38.71667).abs() < 1e-6 && (lon + 9.13333).abs() < 1e-6);
    assert_eq!(src, "gazetteer");
    assert!(row(&s, "Atlantide").is_none());
    assert!(
        row(&s, "Parigi").is_none(),
        "le foto GPS non si geocodificano"
    );
    // Seconda esecuzione: niente da fare.
    assert_eq!(run(&s, Some(&g)), 0);
}

#[test]
fn manual_position_is_never_overwritten() {
    let (_d, s) = setup();
    s.write(|c| pc_store::geo_map::map_place_set(c, "Roma", 1.0, 2.0))
        .unwrap();
    let g = Gazetteer::parse(TSV).unwrap();
    run(&s, Some(&g));
    assert_eq!(row(&s, "Roma").unwrap(), (1.0, 2.0, "manual".into()));
}

#[test]
fn without_gazetteer_is_a_noop() {
    let (_d, s) = setup();
    assert_eq!(run(&s, None), 0);
    assert!(row(&s, "Lisbona").is_none());
}
