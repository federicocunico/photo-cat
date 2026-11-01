use pc_store::{analysis, migrate, people};
use rusqlite::Connection;
fn face(x: f32, axis: usize) -> people::DetectedFace {
    let mut embedding = vec![0.0; 128];
    embedding[axis] = 1.0;
    people::DetectedFace {
        bbox: [x, 0.1, 0.2, 0.2],
        quality: 0.99,
        embedding,
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
