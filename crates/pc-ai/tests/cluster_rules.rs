//! Test di raggruppamento dei volti: gruppi ignorati come attrattori,
//! vincoli "persone diverse", nessuna persona creata automaticamente ed
//! esclusione dei volti scartati per qualità.

use pc_ai::{cluster, face_policy::FacePolicy, faces::SAME_PERSON};
use pc_core::AiThresholds;
use pc_governor::MemoryGovernor;
use pc_jobs::JobControl;
use pc_store::{people, Store};
use std::sync::Arc;

fn emb(axis: usize, extra: Option<(usize, f32)>) -> Vec<f32> {
    let mut v = vec![0.0f32; 128];
    v[axis] = 1.0;
    if let Some((k, w)) = extra {
        v[k] = w;
    }
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        v.iter_mut().for_each(|x| *x /= norm);
    }
    v
}

fn face(axis: usize) -> people::DetectedFace {
    people::DetectedFace {
        bbox: [0.1, 0.1, 0.2, 0.2],
        quality: 0.96,
        sharpness: Some(0.80),
        frontality: Some(0.92),
        size_px: Some(120),
        quality_score: Some(0.88),
        embedding: emb(axis, None),
    }
}

fn run_cluster(db: &Store, t: &AiThresholds) {
    cluster::run(
        db,
        &MemoryGovernor::new(256),
        &JobControl::new(),
        Arc::new(FacePolicy::new(true)),
        &Default::default(),
        t,
    )
    .unwrap();
}

#[test]
fn new_photos_of_ignored_person_join_ignored_group() {
    // Scenario "Nuove foto della persona ignorata".
    let tmp = tempfile::tempdir().unwrap();
    let db = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
    let thresholds = AiThresholds::default();
    db.write(|c| {
        for id in 1..=3 {
            c.execute(
                "INSERT INTO media_item(id,path,name,ext,size,mtime,added_at,updated_at) VALUES (?1,?2,'t','jpg',1,1,1,1)",
                rusqlite::params![id, format!("p{id}.jpg")],
            )?;
            people::save_faces(c, id, &[face(2)], "sface", "det+sface", SAME_PERSON)?;
        }
        Ok(())
    })
    .unwrap();
    run_cluster(&db, &thresholds);
    let ignored_group = db
        .write(|c| {
            let pid = people::records(c, Some(1))?[0]
                .person_id
                .expect("raggruppato");
            people::ignore_person(c, pid)?;
            Ok(pid)
        })
        .unwrap();

    // Importazione di 6 nuove foto con lo stesso sconosciuto.
    db.write(|c| {
        for id in 4..=9 {
            c.execute(
                "INSERT INTO media_item(id,path,name,ext,size,mtime,added_at,updated_at) VALUES (?1,?2,'t','jpg',1,1,1,1)",
                rusqlite::params![id, format!("p{id}.jpg")],
            )?;
            people::save_faces(c, id, &[face(2)], "sface", "det+sface", SAME_PERSON)?;
        }
        Ok(())
    })
    .unwrap();
    run_cluster(&db, &thresholds);

    db.read(move |c| {
        let recs = people::records(c, None)?;
        assert_eq!(recs.len(), 9);
        assert!(
            recs.iter().all(|f| f.person_id == Some(ignored_group)),
            "tutti i 6 nuovi volti finiscono nel gruppo ignorato"
        );
        let (ignored_at, base_faces, total_faces): (Option<i64>, i64, i64) = c.query_row(
            "SELECT p.ignored_at, p.ignored_faces, (SELECT count(*) FROM face f WHERE f.person_id=p.id)
             FROM person p WHERE p.id=?1",
            [ignored_group],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        assert!(ignored_at.is_some());
        assert_eq!(base_faces, 3);
        assert_eq!(total_faces - base_faces, 6, "+6 volti nuovi");
        Ok(())
    })
    .unwrap();
}

#[test]
fn not_same_groups_are_never_merged_by_reclustering() {
    // Scenario "Persone diverse": due gruppi simili marcati come persone diverse
    // restano separati nei raggruppamenti successivi.
    let tmp = tempfile::tempdir().unwrap();
    let db = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
    let thresholds = AiThresholds::default();
    let (g1, g2) = db
        .write(|c| {
            let g1 = people::create_person(c, None)?;
            let g2 = people::create_person(c, None)?;
            for id in 1..=6 {
                c.execute(
                    "INSERT INTO media_item(id,path,name,ext,size,mtime,added_at,updated_at) VALUES (?1,?2,'t','jpg',1,1,1,1)",
                    rusqlite::params![id, format!("p{id}.jpg")],
                )?;
                let mut f = face(0);
                // Tutti molto simili (coseno > 0.95): senza vincolo finirebbero insieme.
                f.embedding = emb(0, Some((1, if id <= 3 { 0.05 } else { -0.05 })));
                people::save_faces(c, id, &[f], "sface", "det+sface", SAME_PERSON)?;
                let fid = people::records(c, Some(id))?[0].id;
                let target = if id <= 3 { g1 } else { g2 };
                c.execute(
                    "UPDATE face SET person_id=?2, assigned_by='auto' WHERE id=?1",
                    rusqlite::params![fid, target],
                )?;
            }
            people::refresh_centroids(c)?;
            people::mark_not_same(c, g1, g2)?;
            Ok((g1, g2))
        })
        .unwrap();

    run_cluster(&db, &thresholds);

    db.read(move |c| {
        let recs = people::records(c, None)?;
        for r in &recs {
            let expected = if r.media_id <= 3 { g1 } else { g2 };
            assert_eq!(r.person_id, Some(expected), "media {}", r.media_id);
        }
        Ok(())
    })
    .unwrap();
}

#[test]
fn automatic_clustering_never_creates_named_person_or_identity() {
    // Scenario "Nessuna persona creata automaticamente" + esclusione scartati.
    let tmp = tempfile::tempdir().unwrap();
    let db = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
    let thresholds = AiThresholds::default();
    db.write(move |c| {
        for id in 1..=12 {
            c.execute(
                "INSERT INTO media_item(id,path,name,ext,size,mtime,added_at,updated_at) VALUES (?1,?2,'t','jpg',1,1,1,1)",
                rusqlite::params![id, format!("p{id}.jpg")],
            )?;
            let mut f = face(4);
            if id == 12 {
                // Volto mosso sotto la soglia di nitidezza: scartato dal raggruppamento.
                f.sharpness = Some(0.10);
                f.quality_score = Some(0.20);
            }
            people::save_faces_tuned(
                c,
                id,
                &[f],
                "sface",
                "det+sface",
                pc_ai::analyze::face_match(&thresholds),
                &Default::default(),
            )?;
        }
        Ok(())
    })
    .unwrap();

    run_cluster(&db, &thresholds);

    db.read(|c| {
        let named_or_linked: i64 = c.query_row(
            "SELECT count(*) FROM person WHERE name IS NOT NULL OR identity_uid IS NOT NULL",
            [],
            |r| r.get(0),
        )?;
        assert_eq!(
            named_or_linked, 0,
            "nessuna persona creata automaticamente senza scelta dell'utente"
        );
        let recs = people::records(c, None)?;
        let clustered: Vec<_> = recs.iter().filter(|r| r.media_id <= 11).collect();
        let group = clustered[0].person_id.expect("gruppo da rivedere");
        assert!(clustered.iter().all(|r| r.person_id == Some(group)));
        let blurred = recs.iter().find(|r| r.media_id == 12).unwrap();
        assert_eq!(
            blurred.person_id, None,
            "il volto scartato per qualità non entra nel gruppo"
        );
        Ok(())
    })
    .unwrap();
}

#[test]
fn merge_suggestions_find_similar_groups_respect_not_same_and_limit_one_per_group() {
    // Scenari "Gruppi probabilmente uguali", esclusione "persone diverse" e limite di 1 suggerimento per gruppo.
    let tmp = tempfile::tempdir().unwrap();
    let db = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
    let thresholds = AiThresholds::default();
    let matching = pc_ai::analyze::face_match(&thresholds);

    let (ga, gb, gc) = db
        .write(move |c| {
            let ga = people::create_person(c, None)?;
            let gb = people::create_person(c, None)?;
            let gc = people::create_person(c, None)?;
            let mut media_id = 0i64;
            let mut insert_group = |pid: i64, vecs: [Vec<f32>; 3]| -> pc_store::StoreResult<()> {
                for v in vecs {
                    media_id += 1;
                    c.execute(
                        "INSERT INTO media_item(id,path,name,ext,size,mtime,added_at,updated_at) VALUES (?1,?2,'t','jpg',1,1,1,1)",
                        rusqlite::params![media_id, format!("m{media_id}.jpg")],
                    )?;
                    let mut f = face(0);
                    f.embedding = v;
                    people::save_faces_tuned(
                        c,
                        media_id,
                        &[f],
                        "sface",
                        "det+sface",
                        matching,
                        &Default::default(),
                    )?;
                    let fid = people::records(c, Some(media_id))?[0].id;
                    c.execute(
                        "UPDATE face SET person_id=?2, assigned_by='manual' WHERE id=?1",
                        rusqlite::params![fid, pid],
                    )?;
                }
                Ok(())
            };
            // ga e gb: molto simili (coseno ~0.95); gc: pure simile a ga ma leggermente meno (~0.85).
            insert_group(
                ga,
                [
                    emb(0, Some((1, 0.1))),
                    emb(0, Some((1, 0.2))),
                    emb(0, Some((1, 0.15))),
                ],
            )?;
            insert_group(
                gb,
                [
                    emb(0, Some((1, 0.25))),
                    emb(0, Some((1, 0.30))),
                    emb(0, Some((1, 0.20))),
                ],
            )?;
            insert_group(
                gc,
                [
                    emb(0, Some((2, 0.55))),
                    emb(0, Some((2, 0.60))),
                    emb(0, Some((2, 0.50))),
                ],
            )?;
            people::refresh_centroids_with(c, &matching)?;
            let sug = people::refresh_merge_suggestions(c, &matching)?;
            // Limite di 1 suggerimento per gruppo: ga si accoppia con gb (punteggio più alto),
            // quindi né ga né gb possono apparire in una seconda coppia con gc.
            assert_eq!(sug.len(), 1);
            assert_eq!((sug[0].a, sug[0].b), (ga.min(gb), ga.max(gb)));
            assert!(sug[0].score >= thresholds.face_similarity * 0.8);

            // Se l'utente indica che ga e gb sono "persone diverse", la coppia sparisce
            // e al ricalcolo successivo non viene più proposta (ga si accoppia invece con gc).
            people::mark_not_same(c, ga, gb)?;
            assert!(people::list_merge_suggestions(c)?.is_empty());
            let after = people::refresh_merge_suggestions(c, &matching)?;
            assert_eq!(after.len(), 1);
            assert_eq!((after[0].a, after[0].b), (ga.min(gc), ga.max(gc)));
            Ok((ga, gb, gc))
        })
        .unwrap();
    let _ = (ga, gb, gc);
}
