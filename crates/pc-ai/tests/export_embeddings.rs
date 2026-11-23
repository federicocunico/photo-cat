//! Esportazione degli embedding per il banco di prova (task 1.4): il file contiene solo
//! embedding e id anonimi, mai nomi, percorsi, date o id del catalogo.

use pc_store::{people, Store};
use std::collections::HashSet;

#[test]
fn il_file_esportato_contiene_solo_embedding_e_id_anonimi() {
    let tmp = tempfile::tempdir().unwrap();
    let db = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
    db.write(|c| {
        let gioia = people::create_person(c, Some("Gioia Rossi"))?;
        let marco = people::create_person(c, None)?;
        c.execute(
            "UPDATE person SET identity_uid='uid-segreto-marco' WHERE id=?1",
            [marco],
        )?;
        let unnamed = people::create_person(c, None)?;
        let ignored = people::create_person(c, Some("Sconosciuto"))?;
        c.execute(
            "UPDATE person SET ignored_at=1 WHERE id=?1",
            [ignored],
        )?;
        for (media, person) in [
            (17, gioia),
            (17, marco),
            (23, gioia),
            (31, unnamed),
            (40, ignored),
        ] {
            c.execute(
                "INSERT OR IGNORE INTO media_item(id,path,name,ext,size,mtime,added_at,updated_at)
                 VALUES (?1,?2,?3,'jpg',1,1,1,1)",
                rusqlite::params![
                    media,
                    format!("D:\\Foto\\Gioia Rossi\\vacanza-{media}.jpg"),
                    format!("vacanza-{media}.jpg")
                ],
            )?;
            c.execute(
                "INSERT OR IGNORE INTO media_dates(media_id, captured_at) VALUES (?1, '2020-07-01 10:00:00')",
                [media],
            )?;
            let e: Vec<u8> = (0..128)
                .flat_map(|i| ((i as f32 + media as f32) / 1000.0).to_le_bytes())
                .collect();
            c.execute(
                "INSERT INTO face(id,media_id,bbox,quality,quality_score,embedding,person_id,assigned_by,model_ver)
                 VALUES (?1,?2,'[0.1,0.1,0.2,0.2]',0.9,0.8,?3,?4,'manual','sface')",
                rusqlite::params![1000 + media * 10 + person, media, e, person],
            )?;
        }
        Ok(())
    })
    .unwrap();

    let export = |unlabeled: bool| {
        let mut out = Vec::new();
        db.read(move |c| Ok(pc_ai::bench::export_named(c, &mut out, unlabeled).map(|n| (n, out))))
            .unwrap()
            .unwrap()
    };
    let (n, bytes) = export(false);
    assert_eq!(
        n, 3,
        "solo i volti delle persone con nome o identità, non ignorate"
    );
    let text = String::from_utf8(bytes).unwrap();
    for secret in [
        "Gioia",
        "Rossi",
        "uid-segreto",
        "Foto",
        "vacanza",
        "2020",
        "jpg",
        "Sconosciuto",
    ] {
        assert!(!text.contains(secret), "{secret} nel file esportato");
    }
    let allowed: HashSet<&str> = ["face", "identity", "photo", "quality", "embedding"].into();
    let mut faces = Vec::new();
    let mut identities = HashSet::new();
    let mut photos = HashSet::new();
    for line in text.lines() {
        let v: serde_json::Value = serde_json::from_str(line).unwrap();
        let obj = v.as_object().unwrap();
        for key in obj.keys() {
            assert!(allowed.contains(key.as_str()), "campo {key}");
        }
        assert_eq!(obj["embedding"].as_array().unwrap().len(), 128);
        assert!(obj["embedding"]
            .as_array()
            .unwrap()
            .iter()
            .all(|x| x.is_number()));
        faces.push(obj["face"].as_i64().unwrap());
        identities.insert(obj["identity"].as_i64().unwrap());
        photos.insert(obj["photo"].as_i64().unwrap());
    }
    // Id rinumerati da 1: nessun id del catalogo.
    assert_eq!(faces, vec![1, 2, 3]);
    assert_eq!(identities, HashSet::from([1, 2]));
    assert_eq!(photos, HashSet::from([1, 2]));
    // Si rilegge come dati del banco di prova.
    let path = tmp.path().join("e.faces.jsonl");
    std::fs::write(&path, &text).unwrap();
    let samples = pc_ai::bench::read_jsonl(&path).unwrap();
    assert_eq!(samples.len(), 3);
    assert!(samples.iter().all(|s| s.time.is_none()));

    // Con i volti senza etichetta: gli altri volti, senza identità.
    let (n, bytes) = export(true);
    assert_eq!(n, 5);
    let text = String::from_utf8(bytes).unwrap();
    let unlabeled = text
        .lines()
        .filter(|l| serde_json::from_str::<serde_json::Value>(l).unwrap()["identity"].is_null())
        .count();
    assert_eq!(unlabeled, 2);
    assert!(!text.contains("Sconosciuto"));
}
