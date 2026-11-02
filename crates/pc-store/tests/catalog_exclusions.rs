use pc_store::{
    bulk, detail,
    model::{CatalogKind, NewMedia},
    query::{self, LibraryFilter, SortKey},
    repo, Store,
};
use serde_json::json;

#[test]
fn manual_exclusions_restore_exact_state_and_never_touch_files() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("photo.jpg");
    std::fs::write(&file, b"original photo").unwrap();
    let db = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
    let path = file.to_string_lossy().to_string();
    db.write(move |c| {
        let a = repo::media::upsert(
            c,
            &NewMedia {
                path,
                name: "photo.jpg".into(),
                ..Default::default()
            },
        )?;
        let b = repo::media::upsert(
            c,
            &NewMedia {
                path: "b.jpg".into(),
                name: "b.jpg".into(),
                ..Default::default()
            },
        )?;
        let cat = repo::catalog::create(c, "Trip", CatalogKind::Manual, None)?;
        repo::catalog::add_items(c, cat, &[a])?;
        bulk::exclude_from_catalog(c, cat, &[a, b])?;
        assert!(repo::catalog::manual_items(c, cat)?.is_empty());
        let counts = detail::catalog_summaries(c)?;
        assert_eq!((counts[0].items, counts[0].excluded), (0, 2));
        bulk::undo_last(c)?;
        assert_eq!(repo::catalog::manual_items(c, cat)?, vec![a]);
        assert_eq!(detail::catalog_summaries(c)?[0].excluded, 0);
        repo::catalog::exclude(c, cat, &[a])?;
        let old: i64 = c.query_row("SELECT excluded_at FROM catalog_exclusion", [], |r| {
            r.get(0)
        })?;
        bulk::restore_to_catalog(c, cat, &[a])?;
        assert_eq!(repo::catalog::manual_items(c, cat)?, vec![a]);
        bulk::undo_last(c)?;
        assert!(repo::catalog::manual_items(c, cat)?.is_empty());
        assert_eq!(
            c.query_row("SELECT excluded_at FROM catalog_exclusion", [], |r| r
                .get::<_, i64>(0))?,
            old
        );
        bulk::add_to_catalog(c, cat, &[a])?;
        assert_eq!(detail::catalog_summaries(c)?[0].excluded, 0);
        bulk::undo_last(c)?;
        assert_eq!(detail::catalog_summaries(c)?[0].excluded, 1);
        assert_eq!(query::count(c, &LibraryFilter::default())?, 2);
        Ok(())
    })
    .unwrap();
    assert_eq!(std::fs::read(file).unwrap(), b"original photo");
}

#[test]
fn smart_exclusions_survive_reclassification_and_resolve_membership() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("photo.jpg");
    std::fs::write(&file, b"unchanged").unwrap();
    let db = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
    let path = file.to_string_lossy().to_string();
    db.write(move |c| {
        let a = repo::media::upsert(
            c,
            &NewMedia {
                path,
                name: "a.jpg".into(),
                ext: "jpg".into(),
                ..Default::default()
            },
        )?;
        let cat = repo::catalog::create(
            c,
            "Smart",
            CatalogKind::Smart,
            Some(&json!({"field":"format","cmp":"eq","value":"jpg"})),
        )?;
        let dependent = repo::catalog::create(
            c,
            "Dependent",
            CatalogKind::Smart,
            Some(&json!({"field":"catalog","cmp":"eq","value":cat})),
        )?;
        let visible = LibraryFilter {
            catalog_id: Some(cat),
            ..Default::default()
        };
        let mut excluded = LibraryFilter {
            excluded_from: Some(cat),
            ..Default::default()
        };
        assert_eq!(query::count(c, &visible)?, 1);
        bulk::exclude_from_catalog(c, cat, &[a])?;
        assert_eq!(query::count(c, &visible)?, 0);
        assert_eq!(
            query::count(
                c,
                &LibraryFilter {
                    catalog_id: Some(dependent),
                    ..Default::default()
                }
            )?,
            0
        );
        assert_eq!(
            query::page(c, &excluded, SortKey::ExcludedAt, None, 1)?.items[0].id,
            a
        );
        assert!(
            query::page(c, &excluded, SortKey::ExcludedAt, None, 1)?.items[0]
                .excluded_at
                .is_some()
        );
        c.execute("UPDATE media_item SET ext='png' WHERE id=?1", [a])?;
        assert_eq!(query::count(c, &excluded)?, 0);
        excluded.matching_only = Some(false);
        assert_eq!(query::count(c, &excluded)?, 1);
        c.execute("UPDATE media_item SET ext='jpg' WHERE id=?1", [a])?;
        assert_eq!(query::count(c, &visible)?, 0);
        bulk::undo_last(c)?;
        assert_eq!(query::count(c, &visible)?, 1);
        assert_eq!(
            query::count(
                c,
                &LibraryFilter {
                    catalog_id: Some(dependent),
                    ..Default::default()
                }
            )?,
            1
        );
        assert!(repo::catalog::add_items(c, cat, &[a]).is_err());
        assert!(repo::catalog::manual_items(c, cat)?.is_empty());
        assert_eq!(query::count(c, &LibraryFilter::default())?, 1);
        let unknown = repo::catalog::create(
            c,
            "Senza dimensioni",
            CatalogKind::Smart,
            Some(&json!({"field":"width","cmp":"gt","value":100})),
        )?;
        let not_member = LibraryFilter {
            rule: Some(json!({"field":"catalog","cmp":"neq","value":unknown})),
            ..Default::default()
        };
        assert_eq!(query::count(c, &not_member)?, 1);
        repo::catalog::delete(c, unknown)?;
        assert_eq!(query::count(c, &not_member)?, 1);
        Ok(())
    })
    .unwrap();
    assert_eq!(std::fs::read(file).unwrap(), b"unchanged");
}
