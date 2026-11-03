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
