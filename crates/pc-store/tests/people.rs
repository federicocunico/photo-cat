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
