use pc_store::{analysis, migrate, people};
use rusqlite::Connection;
fn face(x: f32, axis: usize) -> people::DetectedFace {
    let mut embedding = vec![0.0; 128];
    embedding[axis] = 1.0;
    people::DetectedFace {
        bbox: [x, 0.1, 0.2, 0.2],
        quality: 0.99,
        embedding,
        ..Default::default()
    }
}

#[test]
fn corrections_survive_reanalysis_and_removed_faces_stay_removed() {
    let mut c = Connection::open_in_memory().unwrap();
    migrate::migrate(&mut c, None).unwrap();
    c.execute("INSERT INTO media_item(id,path,name,ext,size,mtime,added_at,updated_at) VALUES (1,'synthetic.jpg','synthetic.jpg','jpg',1,1,1,1)",[]).unwrap();
    let found = vec![face(0.1, 0), face(0.5, 1)];
    people::save_faces(&c, 1, &found, "sface1", "det1+sface1", 0.363).unwrap();
    let initial = people::records(&c, Some(1)).unwrap();
    assert_eq!(initial.len(), 2);
    let alice = people::create_person(&c, None).unwrap();
    let bob = people::create_person(&c, None).unwrap();
    people::assign(&c, &[initial[0].id], alice).unwrap();
    people::assign(&c, &[initial[1].id], bob).unwrap();
    people::rename(&c, alice, "Alice").unwrap();
    people::save_faces(&c, 1, &found, "sface2", "det2+sface2", 0.363).unwrap();
    assert_eq!(
        people::records(&c, Some(1)).unwrap()[0].person_id,
        Some(alice)
    );
    let other = people::separate(&c, &[initial[0].id], alice).unwrap();
    people::save_faces(&c, 1, &found, "sface2", "det2+sface2", 0.363).unwrap();
    assert_eq!(
        people::records(&c, Some(1)).unwrap()[0].person_id,
        Some(other)
    );
    people::reject(&c, initial[1].id, bob).unwrap();
    people::save_faces(&c, 1, &found, "sface2", "det2+sface2", 0.363).unwrap();
    let after = people::records(&c, Some(1)).unwrap();
    assert_eq!(after[1].person_id, None);
    assert!(after[1].rejected.contains(&bob));
    people::merge(&c, other, bob).unwrap();
    assert_eq!(
        people::records(&c, Some(1)).unwrap()[0].person_id,
        Some(bob)
    );
    people::remove_face(&c, initial[0].id).unwrap();
    people::save_faces(&c, 1, &found, "sface2", "det2+sface2", 0.363).unwrap();
    let remaining = people::records(&c, Some(1)).unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, initial[1].id);
    analysis::people(&c, 1, &[[0.1, 0.1, 0.2, 0.2, 0.9]], "yolox").unwrap();
    people::clear(&c).unwrap();
    for table in ["face", "person", "face_exclusion", "person_centroid"] {
        let n: i64 = c
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0, "{table}");
    }
    assert_eq!(
        c.query_row("SELECT count FROM media_people", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn new_faces_match_named_centroids_and_respect_refusals() {
    let mut c = Connection::open_in_memory().unwrap();
    migrate::migrate(&mut c, None).unwrap();
    for id in 1..=2 {
        c.execute("INSERT INTO media_item(id,path,name,ext,size,mtime,added_at,updated_at) VALUES (?1,?2,'test','jpg',1,1,1,1)",rusqlite::params![id,format!("test{id}.jpg")]).unwrap();
    }
    people::save_faces(&c, 1, &[face(0.1, 0)], "sface", "det+sface", 0.363).unwrap();
    let known = people::records(&c, Some(1)).unwrap()[0].id;
    let p = people::create_person(&c, None).unwrap();
    people::assign(&c, &[known], p).unwrap();
    people::rename(&c, p, "Giulia").unwrap();
    people::save_faces(&c, 2, &[face(0.1, 0)], "sface", "det+sface", 0.363).unwrap();
    let fresh = people::records(&c, Some(2)).unwrap()[0].clone();
    assert_eq!(fresh.person_id, Some(p));
    people::reject(&c, fresh.id, p).unwrap();
    people::save_faces(&c, 2, &[face(0.1, 0)], "sface", "det+sface", 0.363).unwrap();
    assert_eq!(people::records(&c, Some(2)).unwrap()[0].person_id, None);
}

#[test]
fn clear_and_vacuum_remove_face_data_from_database_and_wal() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("catalog.db");
    let db = pc_store::Store::open(&path, Default::default()).unwrap();
    db.write(|c|{
        c.execute("INSERT INTO media_item(id,path,name,ext,size,mtime,added_at,updated_at) VALUES (1,'synthetic.jpg','synthetic','jpg',1,1,1,1)",[])?;
        let marker=b"BIOMETRIC-SECRET".repeat(32);
        c.execute("INSERT INTO person(id,name,centroid,created_at) VALUES (1,'Private',?1,1)",[&marker])?;
        c.execute("INSERT INTO face(media_id,bbox,quality,embedding,person_id) VALUES (1,'[0.1,0.1,0.2,0.2]',1,?1,1)",[&marker])?;
        c.execute("INSERT INTO person_centroid(person_id,model_ver,vector) VALUES (1,'test',?1)",[&marker])?;
        people::clear(c)
    }).unwrap();
    db.vacuum().unwrap();
    assert_eq!(
        db.read(|c| Ok(c.query_row("SELECT count(*) FROM face", [], |r| r.get::<_, i64>(0))?))
            .unwrap(),
        0
    );
    let content = std::fs::read(&path).unwrap();
    assert!(!content.windows(16).any(|b| b == b"BIOMETRIC-SECRET"));
    let wal = path.with_extension("db-wal");
    assert!(std::fs::metadata(wal).map(|m| m.len() == 0).unwrap_or(true));
}

fn library_with(n: i64) -> Connection {
    let mut c = Connection::open_in_memory().unwrap();
    migrate::migrate(&mut c, None).unwrap();
    for i in 1..=n {
        c.execute(
            "INSERT INTO media_item(id,path,name,ext,size,mtime,added_at,updated_at) VALUES (?1,?2,?2,'jpg',1,1,1,1)",
            rusqlite::params![i, format!("f{i}.jpg")],
        )
        .unwrap();
    }
    c
}

fn reference(app: &pc_store::AppStore, name: &str, axis: usize) -> String {
    use pc_store::app_store::identities as ids;
    app.with(|a| {
        let uid = ids::create(
            a,
            &ids::IdentityInput {
                name: name.into(),
                ..Default::default()
            },
        )?;
        ids::add_face(
            a,
            &uid,
            b"png",
            "sface",
            Some(&face(0.1, axis).embedding),
            "file:ref.jpg",
        )?;
        Ok(uid)
    })
    .unwrap()
}

#[test]
fn reference_faces_recognise_an_unlinked_identity_in_a_new_library() {
    let app = pc_store::AppStore::open_in_memory().unwrap();
    let giulia = reference(&app, "Giulia", 3);
    let refs = people::IdentityRefs::load(&app, "sface").unwrap();
    let c = library_with(2);
    // Nessuna persona in libreria: il volto simile al riferimento crea "Giulia" collegata.
    people::save_faces_with(&c, 1, &[face(0.1, 3)], "sface", "det+sface", 0.363, &refs).unwrap();
    let f = &people::records(&c, Some(1)).unwrap()[0];
    let person = f.person_id.expect("persona collegata");
    let list = people::list(&c).unwrap();
    assert_eq!(list[0].name.as_deref(), Some("Giulia"));
    assert_eq!(list[0].identity_uid.as_deref(), Some(giulia.as_str()));
    // Un secondo file con Giulia va nella stessa persona, senza duplicarla.
    people::save_faces_with(&c, 2, &[face(0.1, 3)], "sface", "det+sface", 0.363, &refs).unwrap();
    assert_eq!(
        people::records(&c, Some(2)).unwrap()[0].person_id,
        Some(person)
    );
    assert_eq!(people::list(&c).unwrap().len(), 1);
    // Un volto diverso resta senza persona; senza riferimenti nessuna associazione.
    people::save_faces_with(&c, 2, &[face(0.6, 9)], "sface", "det+sface", 0.363, &refs).unwrap();
    let fresh = people::records(&c, Some(2))
        .unwrap()
        .into_iter()
        .find(|f| f.bbox[0] == 0.6)
        .unwrap();
    assert_eq!(fresh.person_id, None);
    let plain = library_with(1);
    people::save_faces(&plain, 1, &[face(0.1, 3)], "sface", "det+sface", 0.363).unwrap();
    assert_eq!(people::records(&plain, Some(1)).unwrap()[0].person_id, None);
}

#[test]
fn not_this_person_is_respected_for_the_linked_identity() {
    let app = pc_store::AppStore::open_in_memory().unwrap();
    reference(&app, "Giulia", 3);
    let refs = people::IdentityRefs::load(&app, "sface").unwrap();
    let c = library_with(1);
    people::save_faces_with(&c, 1, &[face(0.1, 3)], "sface", "det+sface", 0.363, &refs).unwrap();
    let f = people::records(&c, Some(1)).unwrap().remove(0);
    let giulia = f.person_id.unwrap();
    people::reject(&c, f.id, giulia).unwrap();
    // Rianalisi: il riferimento dell'identità collegata alla persona esclusa non vale.
    people::save_faces_with(&c, 1, &[face(0.1, 3)], "sface", "det+sface", 0.363, &refs).unwrap();
    let after = people::records(&c, Some(1)).unwrap().remove(0);
    assert_eq!(after.person_id, None);
    assert!(after.rejected.contains(&giulia));
}

#[test]
fn purge_deletes_reference_faces_but_keeps_identities() {
    let app = pc_store::AppStore::open_in_memory().unwrap();
    reference(&app, "Giulia", 3);
    let c = library_with(1);
    people::save_faces(&c, 1, &[face(0.1, 3)], "sface", "det+sface", 0.363).unwrap();
    people::clear_with_references(&c, &app).unwrap();
    let ids = app.with(pc_store::app_store::identities::list).unwrap();
    assert_eq!(ids.len(), 1);
    assert_eq!(ids[0].faces, 0);
    assert!(people::records(&c, None).unwrap().is_empty());
}

#[test]
fn link_merge_and_sync_identity_names() {
    let c = library_with(2);
    people::save_faces(&c, 1, &[face(0.1, 3)], "sface", "det+sface", 0.363).unwrap();
    people::save_faces(&c, 2, &[face(0.1, 5)], "sface", "det+sface", 0.363).unwrap();
    let a = people::create_person(&c, None).unwrap();
    let b = people::create_person(&c, None).unwrap();
    people::assign(&c, &[people::records(&c, Some(1)).unwrap()[0].id], a).unwrap();
    people::assign(&c, &[people::records(&c, Some(2)).unwrap()[0].id], b).unwrap();
    // "Senza nome" collegato a "Marco Russo".
    people::link_identity(&c, a, Some(("uid-m", "Marco Russo"))).unwrap();
    assert_eq!(people::identity_photos(&c).unwrap()["uid-m"], 1);
    // Collegare un secondo gruppo alla stessa identità unisce le persone.
    let kept = people::link_identity(&c, b, Some(("uid-m", "Marco Russo"))).unwrap();
    assert_eq!(kept, a);
    assert_eq!(people::identity_photos(&c).unwrap()["uid-m"], 2);
    // Identità rinominata mentre la libreria era chiusa: il nome si riallinea.
    let names = std::collections::HashMap::from([("uid-m".to_string(), "Marco R.".to_string())]);
    assert_eq!(people::sync_identity_names(&c, &names).unwrap(), 1);
    assert_eq!(
        people::list(&c).unwrap()[0].name.as_deref(),
        Some("Marco R.")
    );
    // Identità eliminata: la persona resta con il nome ma scollegata.
    people::sync_identity_names(&c, &Default::default()).unwrap();
    let p = &people::list(&c).unwrap()[0];
    assert_eq!(
        (p.name.as_deref(), p.identity_uid.as_deref()),
        (Some("Marco R."), None)
    );
}

/// Scenario identity-registry "Collegamento": il gruppo esce da Da rivedere,
/// appare come Marco Russo in Persone, l'identità mostra il conteggio foto.
#[test]
fn link_moves_group_to_people_section() {
    use pc_store::app_store::identities as ids;
    let app = pc_store::AppStore::open_in_memory().unwrap();
    let uid = app
        .with(|a| {
            ids::create(
                a,
                &ids::IdentityInput {
                    name: "Marco Russo".into(),
                    ..Default::default()
                },
            )
        })
        .unwrap();
    let c = library_with(2);
    let m = people::default_face_match();
    for media in 1..=2 {
        people::save_faces(&c, media, &[face(0.1, 3)], "sface", "det+sface", 0.363).unwrap();
    }
    let face_ids: Vec<_> = (1..=2)
        .map(|media| people::records(&c, Some(media)).unwrap()[0].id)
        .collect();
    let group = people::create_person(&c, None).unwrap();
    people::assign(&c, &face_ids, group).unwrap();

    let before = people::people_sections(&c, &m).unwrap();
    assert_eq!(before.review, 1);
    assert_eq!(before.people, 0);

    people::link_identity(&c, group, Some((&uid, "Marco Russo"))).unwrap();

    let after = people::people_sections(&c, &m).unwrap();
    assert_eq!(after.people, 1);
    assert_eq!(after.review, 0);
    let review = people::list_groups(
        &c,
        people::PeopleSection::Review,
        None,
        people::GroupSort::PhotosDesc,
        None,
        50,
        &m,
    )
    .unwrap();
    assert!(
        review.items.iter().all(|r| r.id != group),
        "dopo il collegamento il gruppo non resta in Da rivedere"
    );
    let people_page = people::list_groups(
        &c,
        people::PeopleSection::People,
        None,
        people::GroupSort::NameAsc,
        None,
        50,
        &m,
    )
    .unwrap();
    assert_eq!(people_page.items.len(), 1);
    assert_eq!(people_page.items[0].id, group);
    assert_eq!(people_page.items[0].name.as_deref(), Some("Marco Russo"));
    assert_eq!(
        people_page.items[0].identity_uid.as_deref(),
        Some(uid.as_str())
    );
    assert_eq!(people::identity_photos(&c).unwrap()[&uid], 2);
}

/// Scenario identity-registry "Collegamento a un'identità già collegata":
/// due gruppi collegati alla stessa uid diventano una sola persona.
#[test]
fn link_to_already_linked_identity_merges() {
    use pc_store::app_store::identities as ids;
    let app = pc_store::AppStore::open_in_memory().unwrap();
    let uid = app
        .with(|a| {
            ids::create(
                a,
                &ids::IdentityInput {
                    name: "Marco Russo".into(),
                    ..Default::default()
                },
            )
        })
        .unwrap();
    let c = library_with(3);
    let m = people::default_face_match();
    for media in 1..=3 {
        people::save_faces(
            &c,
            media,
            &[face(0.1, media as usize)],
            "sface",
            "det+sface",
            0.363,
        )
        .unwrap();
    }
    let a = people::create_person(&c, None).unwrap();
    let b = people::create_person(&c, None).unwrap();
    people::assign(&c, &[people::records(&c, Some(1)).unwrap()[0].id], a).unwrap();
    people::assign(
        &c,
        &[
            people::records(&c, Some(2)).unwrap()[0].id,
            people::records(&c, Some(3)).unwrap()[0].id,
        ],
        b,
    )
    .unwrap();

    people::link_identity(&c, a, Some((&uid, "Marco Russo"))).unwrap();
    let kept = people::link_identity(&c, b, Some((&uid, "Marco Russo"))).unwrap();
    assert_eq!(kept, a);

    let persons = people::list(&c).unwrap();
    assert_eq!(persons.len(), 1);
    assert_eq!(persons[0].id, a);
    assert_eq!(persons[0].name.as_deref(), Some("Marco Russo"));
    assert_eq!(persons[0].identity_uid.as_deref(), Some(uid.as_str()));
    assert_eq!(persons[0].faces, 3);

    let sections = people::people_sections(&c, &m).unwrap();
    assert_eq!(sections.people, 1);
    assert_eq!(sections.review, 0);
    assert_eq!(people::identity_photos(&c).unwrap()[&uid], 3);

    let faces = people::records(&c, None).unwrap();
    assert_eq!(faces.len(), 3);
    assert!(faces.iter().all(|f| f.person_id == Some(a)));
}

/// `rename` senza identità non sposta in Persone: la sezione usa `identity_uid`.
#[test]
fn rename_without_identity_stays_in_review() {
    let c = library_with(1);
    let m = people::default_face_match();
    people::save_faces(&c, 1, &[face(0.1, 0)], "sface", "det+sface", 0.363).unwrap();
    let fid = people::records(&c, Some(1)).unwrap()[0].id;
    let g = people::create_person(&c, None).unwrap();
    people::assign(&c, &[fid], g).unwrap();
    people::rename(&c, g, "Marco").unwrap();

    let sections = people::people_sections(&c, &m).unwrap();
    assert_eq!(sections.people, 0);
    assert_eq!(sections.review, 1);
    let row = &people::list(&c).unwrap()[0];
    assert_eq!(row.name.as_deref(), Some("Marco"));
    assert!(row.identity_uid.is_none());
}

/// Dopo il flag `IDENTITIES_MIGRATED`, un rename successivo non crea identità.
#[test]
fn migrate_to_identities_runs_once() {
    use pc_store::app_store::identities as ids;
    let app = pc_store::AppStore::open_in_memory().unwrap();
    let c = library_with(2);
    people::save_faces(&c, 1, &[face(0.1, 0)], "sface", "det+sface", 0.363).unwrap();
    people::save_faces(&c, 2, &[face(0.1, 1)], "sface", "det+sface", 0.363).unwrap();
    let p1 = people::create_person(&c, None).unwrap();
    let p2 = people::create_person(&c, None).unwrap();
    people::assign(&c, &[people::records(&c, Some(1)).unwrap()[0].id], p1).unwrap();
    people::assign(&c, &[people::records(&c, Some(2)).unwrap()[0].id], p2).unwrap();
    people::rename(&c, p1, "Giulia").unwrap();

    let linked =
        people::migrate_to_identities(&c, |name| app.with(|a| ids::ensure_by_name(a, name)))
            .unwrap();
    assert_eq!(linked, 1);
    assert_eq!(app.with(ids::list).unwrap().len(), 1);

    people::rename(&c, p2, "Luca").unwrap();
    let again =
        people::migrate_to_identities(&c, |name| app.with(|a| ids::ensure_by_name(a, name)))
            .unwrap();
    assert_eq!(again, 0, "la migrazione non riparte");
    assert_eq!(
        app.with(ids::list).unwrap().len(),
        1,
        "nessuna identità implicita per rename post-migrazione"
    );
    let m = people::default_face_match();
    let sections = people::people_sections(&c, &m).unwrap();
    assert_eq!(sections.people, 1);
    assert_eq!(sections.review, 1);
}

fn lib_with(n: i64) -> Connection {
    let mut c = Connection::open_in_memory().unwrap();
    migrate::migrate(&mut c, None).unwrap();
    for id in 1..=n {
        c.execute("INSERT INTO media_item(id,path,name,ext,size,mtime,added_at,updated_at) VALUES (?1,?2,'t','jpg',1,1,1,1)",rusqlite::params![id,format!("t{id}.jpg")]).unwrap();
    }
    c
}

#[test]
fn faces_below_quality_are_kept_but_not_matched() {
    let c = lib_with(2);
    // Persona con nome dal primo volto.
    people::save_faces(&c, 1, &[face(0.1, 0)], "sface", "det+sface", 0.363).unwrap();
    let known = people::records(&c, Some(1)).unwrap()[0].id;
    let giulia = people::create_person(&c, None).unwrap();
    people::assign(&c, &[known], giulia).unwrap();
    people::rename(&c, giulia, "Giulia").unwrap();
    // Secondo volto uguale ma con confidenza 0,7, sotto la soglia 0,8.
    let mut low = face(0.1, 0);
    low.quality = 0.7;
    let m = people::FaceMatch {
        similarity: 0.363,
        min_quality: 0.8,
        floor: Some(0.6),
        ..Default::default()
    };
    people::save_faces_tuned(&c, 2, &[low], "sface", "det+sface", m, &Default::default()).unwrap();
    let f = people::records(&c, Some(2)).unwrap();
    assert_eq!(f.len(), 1, "il volto resta salvato");
    assert_eq!(f[0].person_id, None, "ma non viene associato");
    // Pavimento registrato nella versione; `pending` lo considera aggiornato.
    let ver: String = c
        .query_row(
            "SELECT model_ver FROM analysis_version WHERE media_id=2 AND stage='faces'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(analysis::saved_floor(&ver), Some(0.6));
    assert!(analysis::pending(&c, &[1, 2], "faces", "det+sface", None)
        .unwrap()
        .is_empty());
    // Un volto auto sotto soglia perde la persona; quello manuale la tiene.
    people::save_faces(&c, 2, &[face(0.1, 0)], "sface", "det+sface", 0.363).unwrap();
    c.execute("UPDATE face SET quality=0.7", []).unwrap();
    assert_eq!(people::unassign_low_quality(&c, 0.8, false).unwrap(), 1);
    assert_eq!(people::unassign_low_quality(&c, 0.8, true).unwrap(), 1);
    assert_eq!(
        people::records(&c, Some(1)).unwrap()[0].person_id,
        Some(giulia)
    );
    assert_eq!(people::records(&c, Some(2)).unwrap()[0].person_id, None);
    assert_eq!(people::faces_below(&c, 0.8).unwrap(), 2);
}

#[test]
fn saved_floor_tells_which_files_need_reanalysis() {
    let c = lib_with(3);
    // 1: salvato prima dei pavimenti (minimo 0,8); 2: pavimento 0,6; 3: non analizzato.
    analysis::version(&c, 1, "faces", "det+sface", None).unwrap();
    analysis::version(
        &c,
        2,
        "faces",
        &analysis::with_floor("det+sface", 0.6),
        None,
    )
    .unwrap();
    assert_eq!(
        analysis::below_floor(&c, "faces", 0.7, 0.8).unwrap(),
        vec![1]
    );
    assert_eq!(
        analysis::below_floor(&c, "faces", 0.5, 0.8).unwrap(),
        vec![1, 2]
    );
    assert!(analysis::below_floor(&c, "faces", 0.8, 0.8)
        .unwrap()
        .is_empty());
    // La versione con pavimento non rende il file "da rianalizzare".
    c.execute("UPDATE media_item SET thumb='x'", []).unwrap();
    assert_eq!(
        analysis::stale_count(&c, "faces", "det+sface", None).unwrap(),
        1
    );
    assert_eq!(
        analysis::stale_count(&c, "faces", "det+sface2", None).unwrap(),
        3
    );
}

#[test]
fn person_count_follows_threshold_without_reanalysis() {
    let c = lib_with(1);
    let boxes = [
        [0.1, 0.1, 0.1, 0.1, 0.9],
        [0.3, 0.1, 0.1, 0.1, 0.7],
        [0.5, 0.1, 0.1, 0.1, 0.45],
        [0.7, 0.1, 0.1, 0.1, 0.3],
    ];
    analysis::people_with(&c, 1, &boxes, "yolox", 0.4, Some(0.25)).unwrap();
    let count = |c: &Connection| {
        c.query_row("SELECT count FROM media_people", [], |r| r.get::<_, i64>(0))
            .unwrap()
    };
    assert_eq!(count(&c), 3);
    assert_eq!(analysis::recount_people(&c, 0.6, false).unwrap(), vec![1]);
    assert_eq!(count(&c), 3, "l'anteprima non scrive");
    analysis::recount_people(&c, 0.6, true).unwrap();
    assert_eq!(count(&c), 2);
    assert!(analysis::recount_people(&c, 0.6, true).unwrap().is_empty());
}

#[test]
fn people_pages_have_no_duplicates_or_gaps() {
    let c = lib_with(30);
    let m = people::default_face_match();
    // 12 gruppi da rivedere con un volto ciascuno a qualità decrescente.
    let mut group_ids = Vec::new();
    for i in 0..12 {
        let media = i + 1;
        let mut f = face(0.1, (i % 8) as usize);
        f.quality = 0.99 - (i as f32) * 0.01;
        f.quality_score = Some(f.quality);
        f.sharpness = Some(0.9);
        f.size_px = Some(120);
        people::save_faces(&c, media, &[f], "sface", "det+sface", 0.363).unwrap();
        let fid = people::records(&c, Some(media)).unwrap()[0].id;
        let g = people::create_person(&c, None).unwrap();
        people::assign(&c, &[fid], g).unwrap();
        group_ids.push(g);
    }
    let sections = people::people_sections(&c, &m).unwrap();
    assert_eq!(sections.review, 12);
    assert_eq!(sections.people, 0);
    assert_eq!(sections.ignored, 0);

    let mut seen = Vec::new();
    let mut after = None;
    loop {
        let page = people::list_groups(
            &c,
            people::PeopleSection::Review,
            None,
            people::GroupSort::PhotosDesc,
            after,
            5,
            &m,
        )
        .unwrap();
        assert!(page.items.len() <= 5);
        for row in &page.items {
            assert!(!seen.contains(&row.id), "duplicato {}", row.id);
            seen.push(row.id);
        }
        match page.next {
            Some(id) => after = Some(id),
            None => break,
        }
    }
    assert_eq!(seen.len(), 12, "nessun buco nella paginazione dei gruppi");

    let g0 = group_ids[0];
    // Aggiunge altri volti allo stesso gruppo per la paginazione dei volti.
    for i in 12..30 {
        let mut f = face(0.1 + (i as f32) * 0.01, 0);
        f.quality = 0.5 + (i as f32) * 0.01;
        f.quality_score = Some(f.quality);
        f.sharpness = Some(0.9);
        f.size_px = Some(120);
        people::save_faces(&c, i + 1, &[f], "sface", "det+sface", 0.363).unwrap();
        let fid = people::records(&c, Some(i + 1)).unwrap()[0].id;
        people::assign(&c, &[fid], g0).unwrap();
    }
    let mut face_ids = Vec::new();
    let mut cursor = None;
    loop {
        let page = people::group_faces(
            &c,
            people::FaceScope::Group { group: g0 },
            people::FaceSort::QualityDesc,
            cursor.as_ref(),
            7,
            &m,
        )
        .unwrap();
        for f in &page.items {
            assert!(!face_ids.contains(&f.id), "volto duplicato {}", f.id);
            if let Some(prev) = face_ids.last() {
                let prev_q = page
                    .items
                    .iter()
                    .find(|x| x.id == *prev)
                    .map(|x| x.quality_score);
                // Solo confronta con l'ultimo della pagina precedente tramite cursore.
                let _ = prev_q;
            }
            face_ids.push(f.id);
        }
        // Verifica ordine non crescente sulla pagina.
        for w in page.items.windows(2) {
            assert!(
                w[0].quality_score >= w[1].quality_score
                    || (w[0].quality_score == w[1].quality_score && w[0].id > w[1].id)
            );
        }
        match page.next {
            Some(next) => cursor = Some(next),
            None => break,
        }
    }
    let total = people::group_faces(
        &c,
        people::FaceScope::Group { group: g0 },
        people::FaceSort::QualityDesc,
        None,
        200,
        &m,
    )
    .unwrap()
    .total;
    assert_eq!(face_ids.len() as i64, total);
}

#[test]
fn ignored_person_leaves_photos_but_disappears_from_library_filter() {
    let c = lib_with(3);
    let m = people::default_face_match();
    for media in 1..=3 {
        let mut f = face(0.1, 0);
        f.sharpness = Some(0.9);
        f.size_px = Some(120);
        people::save_faces(&c, media, &[f], "sface", "det+sface", 0.363).unwrap();
    }
    let ids: Vec<_> = (1..=3)
        .map(|media| people::records(&c, Some(media)).unwrap()[0].id)
        .collect();
    let p = people::create_person(&c, None).unwrap();
    people::assign(&c, &ids, p).unwrap();
    people::rename(&c, p, "Sconosciuto").unwrap();
    people::link_identity(&c, p, Some(("uid-x", "Sconosciuto"))).unwrap();

    let before = people::people_sections(&c, &m).unwrap();
    assert_eq!(before.people, 1);
    assert_eq!(before.ignored, 0);
    let facets = pc_store::query::facets(&c, &pc_store::query::LibraryFilter::default()).unwrap();
    assert!(
        facets
            .people
            .values
            .iter()
            .any(|v| v.value == "Sconosciuto"),
        "prima dell'ignore compare nel filtro"
    );
    let media_count =
        pc_store::query::count(&c, &pc_store::query::LibraryFilter::default()).unwrap();
    assert_eq!(media_count, 3);

    people::ignore_person(&c, p).unwrap();
    let after = people::people_sections(&c, &m).unwrap();
    assert_eq!(after.people, 0);
    assert_eq!(after.ignored, 1);
    let review = people::list_groups(
        &c,
        people::PeopleSection::Review,
        None,
        people::GroupSort::PhotosDesc,
        None,
        50,
        &m,
    )
    .unwrap();
    assert!(review.items.is_empty());
    let ignored = people::list_groups(
        &c,
        people::PeopleSection::Ignored,
        None,
        people::GroupSort::PhotosDesc,
        None,
        50,
        &m,
    )
    .unwrap();
    assert_eq!(ignored.items.len(), 1);
    assert_eq!(ignored.items[0].id, p);

    let facets = pc_store::query::facets(&c, &pc_store::query::LibraryFilter::default()).unwrap();
    assert!(
        facets
            .people
            .values
            .iter()
            .all(|v| v.value != "Sconosciuto"),
        "dopo l'ignore assente dal filtro Persone"
    );
    let media_count =
        pc_store::query::count(&c, &pc_store::query::LibraryFilter::default()).unwrap();
    assert_eq!(media_count, 3, "le foto restano in libreria");
    let filtered = pc_store::query::count(
        &c,
        &pc_store::query::LibraryFilter {
            person_ids: vec![p],
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(filtered, 0, "il filtro per persona ignorata non trova foto");
}

#[test]
fn merge_many_unites_groups() {
    let c = lib_with(4);
    let mut face_ids = Vec::new();
    for media in 1..=4 {
        let mut f = face(0.1, media as usize % 4);
        f.sharpness = Some(0.9);
        f.size_px = Some(120);
        people::save_faces(&c, media, &[f], "sface", "det+sface", 0.363).unwrap();
        face_ids.push(people::records(&c, Some(media)).unwrap()[0].id);
    }
    let g1 = people::create_person(&c, None).unwrap();
    let g2 = people::create_person(&c, None).unwrap();
    let g3 = people::create_person(&c, None).unwrap();
    let named = people::create_person(&c, Some("Anna")).unwrap();
    people::assign(&c, &[face_ids[0]], g1).unwrap();
    people::assign(&c, &[face_ids[1]], g2).unwrap();
    people::assign(&c, &[face_ids[2]], g3).unwrap();
    people::assign(&c, &[face_ids[3]], named).unwrap();

    people::merge_many(&c, &[g1, g2, g3], named).unwrap();

    for fid in &face_ids {
        let person = c
            .query_row("SELECT person_id FROM face WHERE id=?1", [*fid], |r| {
                r.get::<_, Option<i64>>(0)
            })
            .unwrap();
        assert_eq!(person, Some(named));
    }
    for id in [g1, g2, g3] {
        let exists: bool = c
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM person WHERE id=?1)",
                [id],
                |r| r.get(0),
            )
            .unwrap();
        assert!(!exists, "gruppo {id} dovrebbe essere eliminato");
    }
    let exists: bool = c
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM person WHERE id=?1)",
            [named],
            |r| r.get(0),
        )
        .unwrap();
    assert!(exists);
}

#[test]
fn restore_ignored_person() {
    let c = lib_with(2);
    let m = people::default_face_match();
    for media in 1..=2 {
        let mut f = face(0.1, 0);
        f.sharpness = Some(0.9);
        f.size_px = Some(120);
        people::save_faces(&c, media, &[f], "sface", "det+sface", 0.363).unwrap();
    }
    let ids: Vec<_> = (1..=2)
        .map(|media| people::records(&c, Some(media)).unwrap()[0].id)
        .collect();
    let p = people::create_person(&c, None).unwrap();
    people::assign(&c, &ids, p).unwrap();
    people::rename(&c, p, "Marco").unwrap();
    people::link_identity(&c, p, Some(("uid-m", "Marco"))).unwrap();

    people::ignore_person(&c, p).unwrap();
    let ignored = people::people_sections(&c, &m).unwrap();
    assert_eq!(ignored.people, 0);
    assert_eq!(ignored.ignored, 1);

    people::restore_person(&c, p).unwrap();
    let after = people::people_sections(&c, &m).unwrap();
    assert_eq!(after.people, 1);
    assert_eq!(after.ignored, 0);
    let ignored_at: Option<i64> = c
        .query_row("SELECT ignored_at FROM person WHERE id=?1", [p], |r| {
            r.get(0)
        })
        .unwrap();
    assert!(ignored_at.is_none());
    let people = people::list_groups(
        &c,
        people::PeopleSection::People,
        None,
        people::GroupSort::PhotosDesc,
        None,
        50,
        &m,
    )
    .unwrap();
    assert_eq!(people.items.len(), 1);
    assert_eq!(people.items[0].id, p);
}

#[test]
fn recover_face_sets_quality_override() {
    let c = lib_with(1);
    let m = people::default_face_match();
    let mut f = face(0.1, 0);
    f.quality = 0.95;
    f.sharpness = Some(0.1); // sotto la soglia predefinita 0,35
    f.size_px = Some(120);
    f.quality_score = Some(0.2);
    people::save_faces(&c, 1, &[f], "sface", "det+sface", 0.363).unwrap();
    let fid = people::records(&c, Some(1)).unwrap()[0].id;
    let rec = people::records(&c, Some(1)).unwrap()[0].clone();
    assert_eq!(rec.discard_reason(&m), Some(people::DiscardReason::Blurred));

    people::recover_faces(&c, &[fid]).unwrap();
    let rec = people::records(&c, Some(1)).unwrap()[0].clone();
    assert!(rec.quality_override);
    assert_eq!(rec.discard_reason(&m), None);
}

#[test]
fn people_corrections_are_undoable() {
    use pc_store::bulk::{self, UndoAction};
    let c = lib_with(3);
    let m = people::default_face_match();
    for media in 1..=3 {
        let mut f = face(0.1, media as usize);
        f.sharpness = Some(0.9);
        f.size_px = Some(120);
        people::save_faces(&c, media, &[f], "sface", "det+sface", 0.363).unwrap();
    }
    let fids: Vec<_> = (1..=3)
        .map(|media| people::records(&c, Some(media)).unwrap()[0].id)
        .collect();
    let a = people::create_person(&c, None).unwrap();
    let b = people::create_person(&c, Some("B")).unwrap();
    people::assign(&c, &[fids[0]], a).unwrap();
    people::assign(&c, &[fids[1]], b).unwrap();

    // Recupero + annulla.
    c.execute(
        "UPDATE face SET sharpness=0.1, quality_override=0 WHERE id=?1",
        [fids[2]],
    )
    .unwrap();
    people::recover_faces(&c, &[fids[2]]).unwrap();
    bulk::record_undo(
        &c,
        "Recupera volti",
        &UndoAction::FacesRecovered { ids: vec![fids[2]] },
    )
    .unwrap();
    assert_eq!(
        bulk::undo_last(&c).unwrap().as_deref(),
        Some("Recupera volti")
    );
    let ov: i64 = c
        .query_row(
            "SELECT quality_override FROM face WHERE id=?1",
            [fids[2]],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(ov, 0);

    // Ignora / ripristina + annulla.
    let (prev_at, prev_faces) = people::ignore_person(&c, b).unwrap();
    bulk::record_undo(
        &c,
        "Ignora persona",
        &UndoAction::PersonIgnored {
            id: b,
            prev_ignored_at: prev_at,
            prev_ignored_faces: prev_faces,
        },
    )
    .unwrap();
    assert_eq!(people::people_sections(&c, &m).unwrap().ignored, 1);
    assert_eq!(
        bulk::undo_last(&c).unwrap().as_deref(),
        Some("Ignora persona")
    );
    assert_eq!(people::people_sections(&c, &m).unwrap().ignored, 0);

    // Unione + annulla ripristina il gruppo sorgente.
    let snap = people::person_snapshot(&c, a).unwrap();
    let target = people::person_snapshot(&c, b).unwrap();
    let faces = people::faces_of_persons(&c, &[a]).unwrap();
    let states = people::face_states(&c, &people::faces_touching(&c, &[a, b]).unwrap()).unwrap();
    let not_same = people::not_same_rows(&c, &[a, b]).unwrap();
    people::merge(&c, a, b).unwrap();
    bulk::record_undo(
        &c,
        "Unisci persone",
        &UndoAction::PersonsMerged {
            from: vec![snap],
            faces,
            to: b,
            states: states.clone(),
            target: Some(target),
            not_same,
        },
    )
    .unwrap();
    assert_eq!(
        bulk::undo_last(&c).unwrap().as_deref(),
        Some("Unisci persone")
    );
    let exists: bool = c
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM person WHERE id=?1)",
            [a],
            |r| r.get(0),
        )
        .unwrap();
    assert!(exists);
    let person: Option<i64> = c
        .query_row("SELECT person_id FROM face WHERE id=?1", [fids[0]], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(person, Some(a));
    // Origine dell assegnazione e rifiuti tornano com erano, non solo la persona.
    let after = people::face_states(&c, &states.iter().map(|s| s.id).collect::<Vec<_>>()).unwrap();
    assert_eq!(after, states);
}

#[test]
fn locate_face_in_large_person_group() {
    let mut c = Connection::open_in_memory().unwrap();
    migrate::migrate(&mut c, None).unwrap();
    let m = people::default_face_match();
    c.execute(
        "INSERT INTO media_item(id,path,name,ext,size,mtime,added_at,updated_at) \
         VALUES (1,'big.jpg','big.jpg','jpg',1,1,1,1)",
        [],
    )
    .unwrap();
    let person = people::create_person(&c, Some("Giulia")).unwrap();
    people::link_identity(&c, person, Some(("uid-g", "Giulia"))).unwrap();

    let tx = c.transaction().unwrap();
    {
        let mut stmt = tx
            .prepare(
                "INSERT INTO face(id,media_id,bbox,quality,sharpness,size_px,quality_score,\
                 quality_override,person_id,rejected) \
                 VALUES (?1,1,'[0.1,0.1,0.2,0.2]',0.95,0.9,120,?2,0,?3,'[]')",
            )
            .unwrap();
        for i in 1..=400 {
            // qualità decrescente: id 1 = migliore (indice 0), id 400 = peggiore (indice 399)
            let score = 1.0 - (i as f32) * 0.001;
            stmt.execute(rusqlite::params![i, score, person]).unwrap();
        }
    }
    tx.commit().unwrap();
    c.execute(
        "INSERT INTO settings_kv(key,value) VALUES ('identity.next.face','400') \
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        [],
    )
    .unwrap();

    let target = 380_i64; // vicino alla fine dell'ordine per qualità
    let expected_index = target - 1; // id 1 → 0, id 380 → 379
    let located =
        people::locate_face(&c, &people::LocateFaceRequest::Face { face_id: target }, &m).unwrap();
    assert_eq!(located.face_id, target);
    assert_eq!(located.media_id, 1);
    assert_eq!(located.media_name, "big.jpg");
    assert_eq!(located.section, people::PeopleSection::People);
    assert_eq!(located.group_id, Some(person));
    assert!(!located.singles);
    assert!(!located.discarded);
    assert_eq!(located.index, expected_index);

    // Stesso risultato via media + person_id.
    let via_media = people::locate_face(
        &c,
        &people::LocateFaceRequest::Media {
            media_id: 1,
            person_id: Some(person),
        },
        &m,
    )
    .unwrap();
    // Preferisce non-scartati; con qualità diverse prende il migliore (indice 0).
    assert_eq!(via_media.section, people::PeopleSection::People);
    assert_eq!(via_media.group_id, Some(person));
    assert_eq!(via_media.index, 0);
}

#[test]
fn locate_face_review_ignored_and_discarded() {
    let c = lib_with(3);
    let m = people::default_face_match();

    // Gruppo da rivedere (senza identity).
    let mut f = face(0.1, 0);
    f.sharpness = Some(0.9);
    f.size_px = Some(120);
    f.quality_score = Some(0.9);
    people::save_faces(&c, 1, &[f], "sface", "det+sface", 0.363).unwrap();
    let review_face = people::records(&c, Some(1)).unwrap()[0].id;
    let review = people::create_person(&c, None).unwrap();
    people::assign(&c, &[review_face], review).unwrap();

    let located = people::locate_face(
        &c,
        &people::LocateFaceRequest::Face {
            face_id: review_face,
        },
        &m,
    )
    .unwrap();
    assert_eq!(located.section, people::PeopleSection::Review);
    assert_eq!(located.group_id, Some(review));
    assert!(!located.discarded);
    assert_eq!(located.index, 0);

    // Gruppo ignorato.
    let mut f = face(0.1, 1);
    f.sharpness = Some(0.9);
    f.size_px = Some(120);
    f.quality_score = Some(0.85);
    people::save_faces(&c, 2, &[f], "sface", "det+sface", 0.363).unwrap();
    let ignored_face = people::records(&c, Some(2)).unwrap()[0].id;
    let ignored = people::create_person(&c, Some("X")).unwrap();
    people::assign(&c, &[ignored_face], ignored).unwrap();
    people::link_identity(&c, ignored, Some(("uid-x", "X"))).unwrap();
    people::ignore_person(&c, ignored).unwrap();

    let located = people::locate_face(
        &c,
        &people::LocateFaceRequest::Face {
            face_id: ignored_face,
        },
        &m,
    )
    .unwrap();
    assert_eq!(located.section, people::PeopleSection::Ignored);
    assert_eq!(located.group_id, Some(ignored));
    assert!(!located.discarded);
    assert_eq!(located.index, 0);

    // Volto scartato (nitidezza sotto soglia) assegnato a un gruppo.
    let mut f = face(0.1, 2);
    f.quality = 0.95;
    f.sharpness = Some(0.1); // < default 0.35
    f.size_px = Some(120);
    f.quality_score = Some(0.2);
    people::save_faces(&c, 3, &[f], "sface", "det+sface", 0.363).unwrap();
    let discarded_face = people::records(&c, Some(3)).unwrap()[0].id;
    // save_faces non assegna i scartati: forza assegnazione + quality_override=0.
    c.execute(
        "UPDATE face SET person_id=?2, sharpness=0.1, quality_override=0, quality_score=0.2 WHERE id=?1",
        rusqlite::params![discarded_face, review],
    )
    .unwrap();
    // Secondo scartato migliore per verificare l'indice nello scope Discarded.
    c.execute(
        "INSERT INTO face(id,media_id,bbox,quality,sharpness,size_px,quality_score,\
         quality_override,person_id,rejected) \
         VALUES (900,3,'[0.5,0.1,0.2,0.2]',0.95,0.05,120,0.5,0,?1,'[]')",
        [review],
    )
    .unwrap();

    let located = people::locate_face(
        &c,
        &people::LocateFaceRequest::Face {
            face_id: discarded_face,
        },
        &m,
    )
    .unwrap();
    assert!(located.discarded);
    assert_eq!(located.group_id, Some(review));
    assert_eq!(located.section, people::PeopleSection::Review);
    assert!(!located.singles);
    // quality_score 0.2 < 0.5 → dopo il volto 900 nell'ordine DESC → index 1
    assert_eq!(located.index, 1);

    let page = people::group_faces(
        &c,
        people::FaceScope::Discarded {
            group: Some(review),
        },
        people::FaceSort::QualityDesc,
        None,
        50,
        &m,
    )
    .unwrap();
    assert_eq!(page.items[located.index as usize].id, discarded_face);
}

#[test]
fn merging_a_named_person_into_an_unnamed_group_keeps_name_and_identity() {
    let c = lib_with(2);
    for media in 1..=2 {
        let mut f = face(0.1, media as usize);
        f.sharpness = Some(0.9);
        f.size_px = Some(120);
        people::save_faces(&c, media, &[f], "sface", "det+sface", 0.363).unwrap();
    }
    let fids: Vec<_> = (1..=2)
        .map(|media| people::records(&c, Some(media)).unwrap()[0].id)
        .collect();
    let unnamed = people::create_person(&c, None).unwrap();
    let giulia = people::create_person(&c, Some("Giulia")).unwrap();
    people::link_identity(&c, giulia, Some(("uid-g", "Giulia"))).unwrap();
    people::assign(&c, &[fids[0]], unnamed).unwrap();
    people::assign(&c, &[fids[1]], giulia).unwrap();

    // Dal banner dei suggerimenti: il gruppo senza nome è la destinazione.
    people::merge(&c, giulia, unnamed).unwrap();
    let (name, uid): (Option<String>, Option<String>) = c
        .query_row(
            "SELECT name, identity_uid FROM person WHERE id=?1",
            [unnamed],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(name.as_deref(), Some("Giulia"));
    assert_eq!(uid.as_deref(), Some("uid-g"));
}

#[test]
fn people_undo_restores_full_face_state_and_ignores_other_actions() {
    use pc_store::bulk::{self, UndoAction};
    let c = lib_with(3);
    for media in 1..=3 {
        let mut f = face(0.1, media as usize);
        f.sharpness = Some(0.9);
        f.size_px = Some(120);
        people::save_faces(&c, media, &[f], "sface", "det+sface", 0.363).unwrap();
    }
    let fids: Vec<_> = (1..=3)
        .map(|media| people::records(&c, Some(media)).unwrap()[0].id)
        .collect();
    let a = people::create_person(&c, None).unwrap();
    people::assign(&c, &fids, a).unwrap();
    c.execute("UPDATE face SET assigned_by='auto'", []).unwrap();
    let before = people::face_states(&c, &fids).unwrap();

    // "Non è questa persona": l'annullamento rimette persona, origine e rifiuti.
    let states = people::face_states(&c, &[fids[0]]).unwrap();
    people::reject(&c, fids[0], a).unwrap();
    bulk::record_undo(
        &c,
        "Non è questa persona",
        &UndoAction::FacesRestored {
            states,
            created: None,
        },
    )
    .unwrap();
    assert_eq!(
        bulk::undo_last_people(&c).unwrap().as_deref(),
        Some("Non è questa persona")
    );
    assert_eq!(people::face_states(&c, &fids).unwrap(), before);

    // "Separa": l'annullamento ricompone il gruppo ed elimina quello creato.
    let states = people::face_states(&c, &people::faces_touching(&c, &[a]).unwrap()).unwrap();
    let created = people::separate(&c, &[fids[2]], a).unwrap();
    bulk::record_undo(
        &c,
        "Separa volti",
        &UndoAction::FacesRestored {
            states,
            created: Some(created),
        },
    )
    .unwrap();
    bulk::undo_last_people(&c).unwrap();
    assert_eq!(people::face_states(&c, &fids).unwrap(), before);
    let exists: bool = c
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM person WHERE id=?1)",
            [created],
            |r| r.get(0),
        )
        .unwrap();
    assert!(!exists);

    // L'ultima operazione è un tag della Libreria: Ctrl+Z in Persone non la tocca.
    bulk::add_tag(&c, "Mare", &[1], &[]).unwrap();
    assert_eq!(bulk::undo_last_people(&c).unwrap(), None);
    let tagged: i64 = c
        .query_row("SELECT count(*) FROM media_tag", [], |r| r.get(0))
        .unwrap();
    assert_eq!(tagged, 1);
}

#[test]
fn group_row_reports_section_and_missing_groups() {
    let mut c = Connection::open_in_memory().unwrap();
    migrate::migrate(&mut c, None).unwrap();
    c.execute("INSERT INTO media_item(id,path,name,ext,size,mtime,added_at,updated_at) VALUES (1,'a.jpg','a.jpg','jpg',1,1,1,1)",[]).unwrap();
    people::save_faces(&c, 1, &[face(0.1, 0), face(0.5, 1)], "m", "d+m", 0.363).unwrap();
    let ids: Vec<i64> = people::records(&c, Some(1))
        .unwrap()
        .iter()
        .map(|f| f.id)
        .collect();
    let m = people::default_face_match();
    let a = people::create_person(&c, None).unwrap();
    let b = people::create_person(&c, None).unwrap();
    let keep = people::create_person(&c, None).unwrap();
    people::assign(&c, &[ids[0]], a).unwrap();
    people::assign(&c, &[ids[1]], b).unwrap();

    let row = people::group_row(&c, a, &m).unwrap().unwrap();
    assert_eq!(row.section, people::PeopleSection::Review);
    assert_eq!(row.faces, 1);
    assert_eq!(row.cover.map(|f| f.id), Some(ids[0]));
    assert!(people::group_row(&c, 9999, &m).unwrap().is_none());

    // Unione di più gruppi in un colpo solo: i volti passano tutti alla destinazione.
    people::merge_many(&c, &[a, b], keep).unwrap();
    assert!(people::group_row(&c, a, &m).unwrap().is_none());
    assert_eq!(people::group_row(&c, keep, &m).unwrap().unwrap().faces, 2);
}

#[test]
fn face_pages_use_the_rank_index() {
    let mut c = Connection::open_in_memory().unwrap();
    migrate::migrate(&mut c, None).unwrap();
    let plan: Vec<String> = c
        .prepare(
            "EXPLAIN QUERY PLAN SELECT f.id FROM face f WHERE f.person_id = 1 \
             ORDER BY COALESCE(f.quality_score, f.quality) DESC, f.id DESC LIMIT 1",
        )
        .unwrap()
        .query_map([], |r| r.get::<_, String>(3))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let plan = plan.join(" | ");
    assert!(plan.contains("face_person_rank"), "{plan}");
    assert!(!plan.contains("TEMP B-TREE"), "{plan}");
}
