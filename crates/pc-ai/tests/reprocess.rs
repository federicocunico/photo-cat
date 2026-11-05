//! Rianalisi per ambito (ai-workspace D3): parametri, ambiti, azzeramento dei marcatori e
//! correzioni manuali preservate. Senza modelli: logica pura e controlli sullo store.
use pc_ai::analyze::{AnalysisParams, Scope};
use pc_store::analysis::{self, AutoLabel};
use pc_store::coverage::Stage;
use pc_store::model::{CatalogKind, NewMedia};
use pc_store::people::{self, DetectedFace, FaceMatch, IdentityRefs};
use pc_store::{repo, reprocess, Store};
use serde_json::json;

const KEY: &str = "clip-image:v1";

fn store_with(n: i64) -> (tempfile::TempDir, Store) {
    let tmp = tempfile::tempdir().unwrap();
    let store = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
    store
        .write(move |c| {
            for i in 1..=n {
                repo::media::upsert(
                    c,
                    &NewMedia {
                        path: format!("IMG_{i}.jpg"),
                        name: format!("IMG_{i}.jpg"),
                        ext: "jpg".into(),
                        ..Default::default()
                    },
                )?;
            }
            c.execute("UPDATE media_item SET thumb = 'm', phash = 7", [])?;
            Ok(())
        })
        .unwrap();
    (tmp, store)
}

fn emb() -> Vec<f32> {
    let mut v = vec![0.0; 512];
    v[0] = 1.0;
    v
}

fn count(store: &Store, sql: &'static str) -> i64 {
    store
        .read(|c| Ok(c.query_row(sql, [], |r| r.get(0))?))
        .unwrap()
}

fn stage_count(store: &Store, stage: &'static str) -> i64 {
    store
        .read(|c| {
            Ok(c.query_row(
                "SELECT count(*) FROM analysis_version WHERE stage = ?1",
                [stage],
                |r| r.get(0),
            )?)
        })
        .unwrap()
}

#[test]
fn legacy_and_new_params_parse() {
    let p = AnalysisParams::parse(&json!({"ids": [2, 4]})).unwrap();
    assert_eq!(p.scope, Scope::Ids(vec![2, 4]));
    assert!(!p.force && p.wants(Stage::Similar) && !p.wants(Stage::Phash));
    let p = AnalysisParams::parse(&json!({"sourceId": 3})).unwrap();
    assert_eq!(p.scope, Scope::Source(3));
    assert_eq!(
        AnalysisParams::parse(&json!({})).unwrap().scope,
        Scope::Library
    );
    let p = AnalysisParams::parse(&json!({
        "scope": {"catalogIds": [1, 2]}, "stages": ["persons", "faces", "phash"], "force": true
    }))
    .unwrap();
    assert_eq!(p.scope, Scope::Catalogs(vec![1, 2]));
    assert!(p.force && p.wants(Stage::Faces) && p.wants(Stage::Phash));
    assert!(!p.wants(Stage::Classification) && !p.wants(Stage::Similar));
    let p = AnalysisParams::parse(&json!({"scope": {"ids": [9]}})).unwrap();
    assert_eq!(p.scope, Scope::Ids(vec![9]));
    assert_eq!(
        AnalysisParams::parse(&json!({"scope": "library"}))
            .unwrap()
            .scope,
        Scope::Library
    );
    assert!(AnalysisParams::parse(&json!({"stages": ["boh"]})).is_err());
}

#[test]
fn two_catalogs_only_missing_selects_non_current_photos_of_both() {
    let (_t, store) = store_with(30);
    let cats = store
        .write(move |c| {
            let a = repo::catalog::create(c, "Vacanze", CatalogKind::Manual, None)?;
            let b = repo::catalog::create(c, "Matrimonio", CatalogKind::Manual, None)?;
            repo::catalog::add_items(c, a, &(1..=10).collect::<Vec<_>>())?;
            repo::catalog::add_items(c, b, &(8..=15).collect::<Vec<_>>())?;
            // 1..3 e 9 già correnti, 4 con tassonomia vecchia.
            for id in [1, 2, 3, 9] {
                analysis::version(c, id, "clip", KEY, Some(2))?;
            }
            analysis::version(c, 4, "clip", KEY, Some(1))?;
            Ok(vec![a, b])
        })
        .unwrap();
    let p = AnalysisParams::parse(&json!({"scope": {"catalogIds": cats}})).unwrap();
    let ids = pc_ai::analyze::resolve_scope(&store, &p.scope).unwrap();
    assert_eq!(ids, (1..=15).collect::<Vec<_>>());
    let todo = store
        .read(|c| analysis::pending(c, &ids, "clip", KEY, Some(2)))
        .unwrap();
    assert_eq!(todo, vec![4, 5, 6, 7, 8, 10, 11, 12, 13, 14, 15]);
}

#[test]
fn force_clears_only_selected_stages_for_target_ids() {
    let (_t, store) = store_with(60);
    store
        .write(move |c| {
            for id in 1..=60 {
                for st in ["clip", "persons", "faces"] {
                    analysis::version(c, id, st, KEY, None)?;
                }
                c.execute(
                    "INSERT INTO similar_done(media_id, params, sharpness, exposure) VALUES (?1,'k',0,0)",
                    [id],
                )?;
            }
            Ok(())
        })
        .unwrap();
    let p = AnalysisParams::parse(&json!({
        "scope": {"ids": (1..=40).collect::<Vec<i64>>()},
        "stages": ["persons", "faces"], "force": true
    }))
    .unwrap();
    let ids = pc_ai::analyze::resolve_scope(&store, &p.scope).unwrap();
    let runnable = [
        Stage::Classification,
        Stage::Persons,
        Stage::Faces,
        Stage::Similar,
    ];
    let forced = p.forced_stages(&runnable);
    assert_eq!(forced, vec![Stage::Persons, Stage::Faces]);
    store
        .write(move |c| reprocess::clear_markers(c, &ids, &forced))
        .unwrap();
    assert_eq!(stage_count(&store, "persons"), 20);
    assert_eq!(stage_count(&store, "faces"), 20);
    assert_eq!(stage_count(&store, "clip"), 60);
    assert_eq!(count(&store, "SELECT count(*) FROM similar_done"), 60);
    // Senza force nulla viene azzerato.
    let nf = AnalysisParams::parse(&json!({"stages": ["clip"]})).unwrap();
    assert!(nf.forced_stages(&runnable).is_empty());
}

#[test]
fn force_similar_and_phash_clear_only_their_markers() {
    let (_t, store) = store_with(6);
    store
        .write(move |c| {
            for id in 1..=6 {
                c.execute(
                    "INSERT INTO similar_done(media_id, params, sharpness, exposure) VALUES (?1,'k',0,0)",
                    [id],
                )?;
            }
            reprocess::clear_markers(c, &[1, 2], &[Stage::Similar, Stage::Phash])
        })
        .unwrap();
    assert_eq!(count(&store, "SELECT count(*) FROM similar_done"), 4);
    assert_eq!(
        count(
            &store,
            "SELECT count(*) FROM media_item WHERE phash IS NULL"
        ),
        2
    );
    assert!(store
        .read(|c| reprocess::any_without_phash(c, &[1, 5]))
        .unwrap());
    assert!(!store
        .read(|c| reprocess::any_without_phash(c, &[5, 6]))
        .unwrap());
}

#[test]
fn manual_subject_and_rejection_survive_forced_classification() {
    let (_t, store) = store_with(1);
    let groups = vec!["subject".to_string(), "scene".to_string()];
    let auto = |g: &str, v: &str| AutoLabel {
        group: g.into(),
        value: v.into(),
        score: 0.9,
    };
    store
        .write(move |c| {
            c.execute(
                "INSERT INTO label(media_id,grp,value,score,origin) VALUES (1,'subject','opera',1.0,'manual')",
                [],
            )?;
            analysis::reject(c, 1, "scene", "mare")?;
            analysis::classification(c, 1, &groups, &[auto("scene", "monti")], &emb(), KEY, 1)?;
            // Rianalisi forzata: marcatore azzerato, poi nuovi risultati diversi.
            reprocess::clear_markers(c, &[1], &[Stage::Classification])?;
            analysis::classification(
                c,
                1,
                &groups,
                &[
                    auto("subject", "paesaggio"),
                    auto("scene", "mare"),
                    auto("scene", "citta"),
                ],
                &emb(),
                KEY,
                1,
            )
        })
        .unwrap();
    let labels: Vec<(String, String, String)> = store
        .read(|c| {
            let mut q = c.prepare("SELECT grp,value,origin FROM label ORDER BY grp,value")?;
            let v = q
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
                .collect::<Result<_, _>>()?;
            Ok(v)
        })
        .unwrap();
    let s = |a: &str, b: &str, c: &str| (a.to_string(), b.to_string(), c.to_string());
    assert_eq!(
        labels,
        vec![s("scene", "citta", "auto"), s("subject", "opera", "manual")]
    );
}

#[test]
fn manual_face_assignment_survives_forced_faces() {
    let (_t, store) = store_with(1);
    let face = DetectedFace {
        bbox: [0.2, 0.2, 0.3, 0.3],
        quality: 0.99,
        sharpness: Some(0.9),
        frontality: Some(0.9),
        size_px: Some(200),
        quality_score: Some(0.9),
        embedding: (0..128).map(|i| (i as f32).sin()).collect(),
    };
    let m = FaceMatch::default();
    let refs = IdentityRefs::default();
    // Embedding diverso nella rianalisi: non deve spostare l'assegnazione manuale.
    let mut again = face.clone();
    again.embedding = (0..128).map(|i| (i as f32).cos()).collect();
    let person = store
        .write(move |c| {
            people::save_faces_tuned(c, 1, std::slice::from_ref(&face), "e1", "a1", m, &refs)?;
            let p = people::create_person(c, Some("Anna"))?;
            let fid = people::records(c, Some(1))?[0].id;
            people::assign(c, &[fid], p)?;
            reprocess::clear_markers(c, &[1], &[Stage::Faces])?;
            people::save_faces_tuned(c, 1, std::slice::from_ref(&again), "e1", "a2", m, &refs)?;
            Ok(p)
        })
        .unwrap();
    let rec = store.read(move |c| people::records(c, Some(1))).unwrap();
    assert_eq!(rec.len(), 1);
    assert_eq!(rec[0].person_id, Some(person));
    assert!(rec[0].manual);
}
