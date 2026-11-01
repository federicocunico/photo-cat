//! Correzione, ripristino e rinomina del luogo (task 4.2–4.4).
use pc_store::bulk::{self, PlaceInput, PlaceInputs};
use pc_store::model::{Location, LocationSource, NewMedia};
use pc_store::query::{self, LibraryFilter};
use pc_store::{migrate, repo};
use rusqlite::Connection;

fn lib() -> Connection {
    let mut c = Connection::open_in_memory().unwrap();
    migrate::migrate(&mut c, None).unwrap();
    c
}

fn photo(c: &Connection, path: &str, loc: Option<(&str, LocationSource)>) -> i64 {
    let id = repo::media::upsert(
        c,
        &NewMedia {
            path: path.into(),
            name: path.rsplit('\\').next().unwrap().into(),
            ext: "jpg".into(),
            ..Default::default()
        },
    )
    .unwrap();
    if let Some((name, source)) = loc {
        repo::meta::set_location(
            c,
            id,
            &Location {
                name: Some(name.into()),
                country: None,
                lat: None,
                lon: None,
                source,
            },
        )
        .unwrap();
    }
    repo::fts::refresh(c, id).unwrap();
    id
}

/// Risolutore di prova: la cartella che contiene il file.
fn by_folder(i: &PlaceInputs) -> Option<Location> {
    let parent = i.path.rsplit_once('\\')?.0;
    Some(Location {
        name: Some(parent.rsplit('\\').next()?.to_string()),
        country: None,
        lat: None,
        lon: None,
        source: LocationSource::Directory,
    })
}

fn place(c: &Connection, id: i64) -> Option<(String, LocationSource)> {
    repo::meta::location(c, id)
        .unwrap()
        .map(|l| (l.name.unwrap_or_default(), l.source))
}

fn input(name: &str) -> PlaceInput {
    PlaceInput {
        name: name.into(),
        country: None,
        lat: None,
        lon: None,
    }
}

fn search(c: &Connection, text: &str) -> Vec<i64> {
    query::ids(
        c,
        &LibraryFilter {
            text: Some(text.into()),
            ..Default::default()
        },
    )
    .unwrap()
}

#[test]
fn set_reset_and_undo_place() {
    let c = lib();
    let dir = LocationSource::Directory;
    let a = photo(&c, r"D:\Foto\Viaggio\D7000\a.jpg", Some(("D7000", dir)));
    let b = photo(&c, r"D:\Foto\Viaggio\D7000\b.jpg", Some(("D7000", dir)));
    let other = photo(&c, r"D:\Foto\Roma\c.jpg", None);

    // Impostazione manuale in blocco.
    bulk::set_place(&c, &[a, b], Some(&input("Giappone")), &by_folder).unwrap();
    assert_eq!(
        place(&c, a),
        Some(("Giappone".into(), LocationSource::Manual))
    );
    assert_eq!(search(&c, "giappone"), vec![a, b], "ricerca sul nuovo nome");
    // Un'analisi successiva non tocca il luogo manuale.
    assert!(!repo::meta::set_location(
        &c,
        a,
        &Location {
            name: Some("Altro".into()),
            country: None,
            lat: None,
            lon: None,
            source: LocationSource::Directory
        }
    )
    .unwrap());

    // "Annulla luogo": luogo e fonte precedenti.
    assert_eq!(
        bulk::undo_last(&c).unwrap().as_deref(),
        Some("Imposta luogo Giappone")
    );
    assert_eq!(place(&c, a), Some(("D7000".into(), dir)));
    assert!(search(&c, "giappone").is_empty());

    // Ripristino del luogo automatico (ricalcolato) e del luogo assente.
    bulk::set_place(&c, &[a, other], Some(&input("Kyoto")), &by_folder).unwrap();
    bulk::set_place(&c, &[a, other], None, &by_folder).unwrap();
    assert_eq!(place(&c, a), Some(("D7000".into(), dir)));
    assert_eq!(place(&c, other), Some(("Roma".into(), dir)));
    bulk::undo_last(&c).unwrap();
    assert_eq!(
        place(&c, other),
        Some(("Kyoto".into(), LocationSource::Manual))
    );
    assert!(bulk::set_place(&c, &[a], Some(&input("  ")), &by_folder).is_err());
}

#[test]
fn rename_merges_and_undoes_in_one_step() {
    let c = lib();
    let dir = LocationSource::Directory;
    let japan: Vec<i64> = (0..3)
        .map(|i| {
            photo(
                &c,
                &format!(r"D:\G\{i}.jpg"),
                Some(("Giappone", LocationSource::Gps)),
            )
        })
        .collect();
    let d7000: Vec<i64> = (0..5)
        .map(|i| photo(&c, &format!(r"D:\G\D7000\{i}.jpg"), Some(("D7000", dir))))
        .collect();
    let facet = |c: &Connection| -> Vec<(String, i64)> {
        query::facets(c, &LibraryFilter::default())
            .unwrap()
            .places
            .values
            .into_iter()
            .map(|f| (f.value, f.count))
            .collect()
    };
    assert_eq!(
        bulk::place_ids(&c, "D7000").unwrap().len(),
        5,
        "conteggio per la conferma"
    );
    // Scenario "Unione con luogo esistente".
    let out = bulk::rename_place(&c, "D7000", "Giappone").unwrap();
    assert_eq!(out.changed, 5);
    assert_eq!(facet(&c), vec![("Giappone".to_string(), 8)]);
    assert_eq!(
        place(&c, d7000[0]),
        Some(("Giappone".into(), LocationSource::Manual))
    );
    assert_eq!(
        place(&c, japan[0]),
        Some(("Giappone".into(), LocationSource::Gps))
    );
    // Scenario "Annulla rinomina".
    bulk::undo_last(&c).unwrap();
    assert_eq!(place(&c, d7000[4]), Some(("D7000".into(), dir)));
    let mut f = facet(&c);
    f.sort();
    assert_eq!(
        f,
        vec![("D7000".to_string(), 5), ("Giappone".to_string(), 3)]
    );
    assert_eq!(bulk::rename_place(&c, "Nessuno", "X").unwrap().changed, 0);
}

#[test]
fn recompute_touches_only_directory_places_and_suggests_names() {
    let c = lib();
    let dir = photo(
        &c,
        r"D:\Foto\Sardegna\Selezionate\a.jpg",
        Some(("Selezionate", LocationSource::Directory)),
    );
    let manual = photo(
        &c,
        r"D:\Foto\Sardegna\Selezionate\b.jpg",
        Some(("Cagliari", LocationSource::Manual)),
    );
    let gps = photo(
        &c,
        r"D:\Foto\Sardegna\Selezionate\c.jpg",
        Some(("Olbia", LocationSource::Gps)),
    );
    let none = photo(&c, r"D:\Foto\Sardegna\Selezionate\d.jpg", None);
    // Nuovo risolutore: "Selezionate" ignorata, si risale a "Sardegna".
    let resolve = |i: &PlaceInputs| {
        let folders: Vec<&str> = i.path.split('\\').collect();
        let name = folders[..folders.len() - 1]
            .iter()
            .rev()
            .find(|s| **s != "Selezionate")?;
        Some(Location {
            name: Some(name.to_string()),
            country: None,
            lat: None,
            lon: None,
            source: LocationSource::Directory,
        })
    };
    assert_eq!(bulk::recompute_directory_places(&c, &resolve).unwrap(), 2);
    assert_eq!(
        place(&c, dir),
        Some(("Sardegna".into(), LocationSource::Directory))
    );
    assert_eq!(
        place(&c, none),
        Some(("Sardegna".into(), LocationSource::Directory))
    );
    assert_eq!(
        place(&c, manual),
        Some(("Cagliari".into(), LocationSource::Manual))
    );
    assert_eq!(place(&c, gps), Some(("Olbia".into(), LocationSource::Gps)));
    assert_eq!(bulk::recompute_directory_places(&c, &resolve).unwrap(), 0);
    assert_eq!(
        bulk::suggest_places(&c, "sar", 10).unwrap(),
        vec!["Sardegna"]
    );
    assert_eq!(
        bulk::suggest_places(&c, "", 2).unwrap(),
        vec!["Sardegna", "Cagliari"]
    );
    assert!(bulk::suggest_places(&c, "%", 10).unwrap().is_empty());
}
