//! Cataloghi di sistema (change `system-catalogs`): Cestino, Duplicati e Qualità
//! insufficiente. Le foto che ne fanno parte sono nascoste dalle viste della libreria.

use pc_store::{
    bulk,
    model::{NewMedia, SYSTEM_DUPLICATES, SYSTEM_LOW_QUALITY, SYSTEM_TRASH},
    query::{self, LibraryFilter},
    repo, Store,
};

fn media(c: &rusqlite::Connection, name: &str, hash: Option<&str>) -> i64 {
    repo::media::upsert(
        c,
        &NewMedia {
            path: format!("D:/Foto/{name}"),
            name: name.into(),
            content_hash: hash.map(str::to_string),
            ..Default::default()
        },
    )
    .unwrap()
}

fn all() -> LibraryFilter {
    LibraryFilter::default()
}

#[test]
fn system_catalogs_exist_and_are_protected() {
    let tmp = tempfile::tempdir().unwrap();
    let db = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
    db.write(|c| {
        for role in [SYSTEM_TRASH, SYSTEM_DUPLICATES, SYSTEM_LOW_QUALITY] {
            let id = repo::catalog::system_id(c, role)?;
            assert_eq!(repo::catalog::get(c, id)?.system.as_deref(), Some(role));
            assert!(
                repo::catalog::delete(c, id).is_err(),
                "{role} non eliminabile"
            );
            assert!(
                repo::catalog::rename(c, id, "Altro").is_err(),
                "{role} non rinominabile"
            );
        }
        // I cataloghi dell'utente vengono prima di quelli di sistema.
        let user = repo::catalog::create(c, "Zzz", pc_store::model::CatalogKind::Manual, None)?;
        assert_eq!(repo::catalog::list(c)?[0].id, user);
        Ok(())
    })
    .unwrap();
}

#[test]
fn trashed_photos_are_hidden_everywhere_but_the_trash_and_can_be_restored() {
    let tmp = tempfile::tempdir().unwrap();
    let db = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
    db.write(|c| {
        let a = media(c, "a.jpg", None);
        let b = media(c, "b.jpg", None);
        let trip = repo::catalog::create(c, "Viaggio", pc_store::model::CatalogKind::Manual, None)?;
        repo::catalog::add_items(c, trip, &[a, b])?;
        let trash = repo::catalog::system_id(c, SYSTEM_TRASH)?;

        bulk::add_to_catalog(c, trash, &[a])?;
        assert_eq!(query::count(c, &all())?, 1, "nascosta da Tutte le foto");
        let in_trip = LibraryFilter {
            catalog_id: Some(trip),
            ..all()
        };
        assert_eq!(
            query::count(c, &in_trip)?,
            1,
            "nascosta anche nei cataloghi dell'utente"
        );
        let in_trash = LibraryFilter {
            catalog_id: Some(trash),
            ..all()
        };
        assert_eq!(
            query::count(c, &in_trash)?,
            1,
            "visibile aprendo il Cestino"
        );

        // Ripristina: torna visibile con i suoi cataloghi; l'annullamento la rimette nel cestino.
        let out = bulk::restore_from_system(c, trash, &[a])?;
        assert_eq!(out.changed, 1);
        assert_eq!(query::count(c, &all())?, 2);
        assert_eq!(query::count(c, &in_trip)?, 2);
        bulk::undo_last(c)?;
        assert_eq!(query::count(c, &all())?, 1);
        // "Ripristina" non vale per i cataloghi dell'utente.
        assert!(bulk::restore_from_system(c, trip, &[b]).is_err());
        Ok(())
    })
    .unwrap();
}

#[test]
fn duplicates_keep_the_first_import_and_set_aside_the_others() {
    let tmp = tempfile::tempdir().unwrap();
    let db = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
    db.write(|c| {
        let first = media(c, "a.jpg", Some("h1"));
        let copy1 = media(c, "a (1).jpg", Some("h1"));
        let copy2 = media(c, "a (2).jpg", Some("h1"));
        let unique = media(c, "b.jpg", Some("h2"));
        let out = bulk::collect_duplicates(c)?;
        assert_eq!(out.changed, 2);
        let dup = repo::catalog::system_id(c, SYSTEM_DUPLICATES)?;
        let mut set_aside = repo::catalog::manual_items(c, dup)?;
        set_aside.sort();
        assert_eq!(set_aside, vec![copy1, copy2]);
        assert_eq!(query::count(c, &all())?, 2, "restano {first} e {unique}");
        // Una seconda passata non trova altro.
        assert_eq!(bulk::collect_duplicates(c)?.changed, 0);
        // Annullabile.
        bulk::undo_last(c)?;
        assert_eq!(query::count(c, &all())?, 4);
        Ok(())
    })
    .unwrap();
}

#[test]
fn move_to_trash_removes_from_manual_catalogs_and_undoes_in_one_step() {
    use pc_store::model::CatalogKind;
    let tmp = tempfile::tempdir().unwrap();
    let db = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
    db.write(|c| {
        let a = media(c, "a.jpg", None);
        let b = media(c, "b.jpg", None);
        let trip = repo::catalog::create(c, "Viaggio", CatalogKind::Manual, None)?;
        let fam = repo::catalog::create(c, "Famiglia", CatalogKind::Manual, None)?;
        let smart = repo::catalog::create(
            c,
            "Smart",
            CatalogKind::Smart,
            Some(&serde_json::json!({"all":[]})),
        )?;
        repo::catalog::add_items(c, trip, &[a, b])?;
        repo::catalog::add_items(c, fam, &[a])?;
        let trash = repo::catalog::system_id(c, SYSTEM_TRASH)?;

        let out = bulk::move_to_trash(c, &[a])?;
        assert_eq!((out.changed, out.requested), (1, 1));
        assert_eq!(repo::catalog::manual_items(c, trip)?, vec![b]);
        assert!(repo::catalog::manual_items(c, fam)?.is_empty());
        assert_eq!(repo::catalog::manual_items(c, trash)?, vec![a]);
        let _ = smart;

        assert_eq!(bulk::undo_last(c)?.as_deref(), Some("Sposta nel cestino"));
        assert_eq!(repo::catalog::manual_items(c, trip)?, vec![a, b]);
        assert_eq!(repo::catalog::manual_items(c, fam)?, vec![a]);
        assert!(repo::catalog::manual_items(c, trash)?.is_empty());
        Ok(())
    })
    .unwrap();
}

#[test]
fn trash_impact_counts_manual_catalogs_for_ids_and_filters() {
    use pc_store::{model::CatalogKind, query::Target};
    let tmp = tempfile::tempdir().unwrap();
    let db = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
    db.write(|c| {
        let a = media(c, "a.jpg", None);
        let b = media(c, "b.jpg", None);
        let d = media(c, "d.jpg", None);
        let zeta = repo::catalog::create(c, "zeta", CatalogKind::Manual, None)?;
        let alfa = repo::catalog::create(c, "Alfa", CatalogKind::Manual, None)?;
        let empty = repo::catalog::create(c, "Vuoto", CatalogKind::Manual, None)?;
        repo::catalog::add_items(c, zeta, &[a, b])?;
        repo::catalog::add_items(c, alfa, &[b, d])?;
        let _ = empty;
        let loss = |l: Vec<pc_store::sources::CatalogLoss>| {
            l.into_iter().map(|x| (x.name, x.items)).collect::<Vec<_>>()
        };
        assert_eq!(
            loss(bulk::trash_impact(c, &Target::ids(vec![b]))?),
            vec![("Alfa".to_string(), 1), ("zeta".to_string(), 1)]
        );
        assert!(bulk::trash_impact(c, &Target::ids(vec![]))?.is_empty());
        let filter = Target::Filter {
            filter: all(),
            max_id: d,
            except: vec![a],
        };
        assert_eq!(
            loss(bulk::trash_impact(c, &filter)?),
            vec![("Alfa".to_string(), 2), ("zeta".to_string(), 1)]
        );
        Ok(())
    })
    .unwrap();
}

/// Eliminazione di più cataloghi (unified-selection): un'unica operazione, le foto restano in
/// libreria, i cataloghi di sistema vengono saltati; un id inesistente annulla tutto.
#[test]
fn delete_many_skips_system_catalogs_and_keeps_photos() {
    use pc_store::model::CatalogKind;
    let tmp = tempfile::tempdir().unwrap();
    let db = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
    let (a, b, smart, trash, photo) = db
        .write(|c| {
            let photo = media(c, "a.jpg", None);
            let a = repo::catalog::create(c, "Viaggio Toscana", CatalogKind::Manual, None)?;
            let b = repo::catalog::create(c, "Matrimonio", CatalogKind::Manual, None)?;
            let smart = repo::catalog::create(
                c,
                "Recenti",
                CatalogKind::Smart,
                Some(&serde_json::json!({"op": "and", "rules": []})),
            )?;
            repo::catalog::add_items(c, a, &[photo])?;
            Ok((
                a,
                b,
                smart,
                repo::catalog::system_id(c, SYSTEM_TRASH)?,
                photo,
            ))
        })
        .unwrap();

    // Un id inesistente: nessun catalogo eliminato.
    assert!(db
        .write(move |c| repo::catalog::delete_many(c, &[a, 999_999]))
        .is_err());
    assert!(db.read(move |c| repo::catalog::get(c, a)).is_ok());

    let out = db
        .write(move |c| repo::catalog::delete_many(c, &[a, b, trash, smart, a]))
        .unwrap();
    assert_eq!(out.deleted, vec![a, b, smart]);
    assert_eq!(out.skipped, vec![trash]);
    db.read(move |c| {
        assert!(repo::catalog::get(c, a).is_err());
        assert!(repo::catalog::get(c, trash).is_ok(), "il Cestino resta");
        assert_eq!(repo::media::count(c)?, 1, "la foto resta in libreria");
        assert!(repo::media::get(c, photo).is_ok());
        Ok(())
    })
    .unwrap();
}
