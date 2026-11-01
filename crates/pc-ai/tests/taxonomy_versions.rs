//! Voci e gruppi manuali: solo le modifiche che cambiano l'analisi automatica
//! rendono i file "da rianalizzare"; i gruppi creati dall'utente funzionano
//! con le correzioni manuali e con i cataloghi smart.

use pc_ai::taxonomy::{self, Category, Group, Taxonomy};
use pc_store::model::{CatalogKind, NewMedia};
use pc_store::query::{self, LibraryFilter};
use pc_store::{analysis, bulk, repo, Store};

const MODEL: &str = "clip-test";

fn manual(id: &str, label: &str) -> Category {
    Category {
        id: id.into(),
        label: label.into(),
        enabled: true,
        prompts: vec![],
        threshold: None,
    }
}

/// Libreria di `n` foto tutte analizzate con la tassonomia corrente.
fn analyzed_library(n: i64) -> (tempfile::TempDir, Store, Taxonomy) {
    let tmp = tempfile::tempdir().unwrap();
    let db = Store::open(&tmp.path().join("catalog.db"), Default::default()).unwrap();
    let tax = taxonomy::load(&db).unwrap();
    let ver = tax.analysis_version;
    db.write(move |c| {
        for i in 0..n {
            let id = repo::media::upsert(
                c,
                &NewMedia {
                    path: format!("/foto/{i}.jpg"),
                    name: format!("{i}.jpg"),
                    ext: "jpg".into(),
                    ..Default::default()
                },
            )?;
            repo::media::set_thumb(c, id, &format!("{:064x}", id))?;
            let mut v = vec![0.0f32; 512];
            v[(i % 512) as usize] = 1.0;
            analysis::classification(c, id, &["subject".into()], &[], &v, MODEL, ver)?;
        }
        Ok(())
    })
    .unwrap();
    (tmp, db, tax)
}

fn stale(db: &Store) -> i64 {
    let ver = taxonomy::load(db).unwrap().analysis_version;
    db.read(move |c| analysis::stale_count(c, "clip", MODEL, Some(ver)))
        .unwrap()
}

fn save(db: &Store, edit: impl FnOnce(&mut Taxonomy)) -> Taxonomy {
    let current = taxonomy::load(db).unwrap();
    let mut next = current.clone();
    edit(&mut next);
    next.version = current.version + 1;
    taxonomy::save(db, next, current.version).unwrap()
}

#[test]
fn manual_entry_and_rename_keep_the_library_analyzed() {
    let (_tmp, db, _) = analyzed_library(5);
    assert_eq!(stale(&db), 0);
    // Voce manuale aggiunta.
    save(&db, |t| {
        t.groups[0]
            .categories
            .push(manual("matrimonio_anna", "Matrimonio Anna"))
    });
    assert_eq!(stale(&db), 0);
    // Rinomina del nome visualizzato.
    let t = save(&db, |t| {
        let fiori = t.groups[0]
            .categories
            .iter_mut()
            .find(|c| c.id == "fiori")
            .unwrap();
        fiori.label = "Fiori e piante".into();
    });
    assert_eq!(stale(&db), 0);
    assert_eq!(t.version, 3);
    assert_eq!(t.analysis_version, 1);
    // Gruppo solo manuale.
    save(&db, |t| {
        t.groups.push(Group {
            id: "evento".into(),
            label: "Evento".into(),
            threshold: 0.25,
            categories: vec![
                manual("compleanno", "Compleanno"),
                manual("laurea", "Laurea"),
            ],
        })
    });
    assert_eq!(stale(&db), 0);
}

#[test]
fn changed_prompt_makes_the_library_stale() {
    let (_tmp, db, _) = analyzed_library(5);
    let t = save(&db, |t| {
        t.groups[0].categories[0].prompts = vec!["a close-up portrait of a person".into()];
    });
    assert_eq!(t.analysis_version, t.version);
    assert_eq!(stale(&db), 5);
}

#[test]
fn user_group_works_with_manual_labels_and_smart_catalogs() {
    let (_tmp, db, base) = analyzed_library(6);
    let gid = taxonomy::group_id(&base, "Evento");
    assert_eq!(gid, "evento");
    save(&db, |t| {
        t.groups.push(Group {
            id: gid.clone(),
            label: "Evento".into(),
            threshold: 0.25,
            categories: vec![
                manual("compleanno", "Compleanno"),
                manual("laurea", "Laurea"),
            ],
        })
    });
    let catalog = db
        .write(move |c| {
            // L'id generato è accettato dalla scrittura delle etichette.
            bulk::set_label(c, "evento", Some("compleanno"), &[1, 2, 3])?;
            bulk::set_label(c, "evento", Some("laurea"), &[4])?;
            repo::catalog::create(
                c,
                "Compleanni",
                CatalogKind::Smart,
                Some(&serde_json::json!({
                    "field": "label.evento", "cmp": "eq", "value": "compleanno"
                })),
            )
        })
        .unwrap();
    let ids = db
        .read(move |c| {
            query::ids(
                c,
                &LibraryFilter {
                    catalog_id: Some(catalog),
                    ..Default::default()
                },
            )
        })
        .unwrap();
    assert_eq!(ids, vec![1, 2, 3]);
    assert_eq!(stale(&db), 0, "le etichette manuali non toccano l'analisi");
}
