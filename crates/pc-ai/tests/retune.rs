//! Rivalutazione delle soglie dai risultati salvati, senza modelli (task 3.1–3.3).
use pc_ai::retune::{self, ClipInputs, RetuneSpec};
use pc_ai::taxonomy::{Category, Group, Taxonomy};
use pc_ai::text_cache::Concept;
use pc_ai::{classify, face_policy::FacePolicy};
use pc_core::AiThresholds;
use pc_governor::MemoryGovernor;
use pc_jobs::JobControl;
use pc_store::{analysis, Store};
use std::sync::Arc;

const MODEL: &str = "clip-image:v1";

fn category(id: &str) -> Category {
    Category {
        id: id.into(),
        label: id.to_uppercase(),
        enabled: true,
        prompts: vec![format!("a photo of {id}")],
        threshold: None,
    }
}

fn taxonomy() -> Taxonomy {
    Taxonomy {
        version: 1,
        groups: vec![Group {
            id: "subject".into(),
            label: "Tipo soggetto".into(),
            threshold: 0.25,
            categories: vec![category("a"), category("b"), category("c")],
        }],
    }
}

fn one_hot(i: usize) -> Vec<f32> {
    let mut v = vec![0.0; 512];
    v[i] = 1.0;
    v
}

fn concepts() -> Vec<Concept> {
    ["a", "b", "c"]
        .iter()
        .enumerate()
        .map(|(i, id)| Concept {
            group: "subject".into(),
            value: (*id).into(),
            vector: one_hot(i),
        })
        .collect()
}

fn image(raw: [f32; 3]) -> Vec<f32> {
    let n = raw.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mut v = vec![0.0; 512];
    for (i, x) in raw.iter().enumerate() {
        v[i] = x / n;
    }
    v
}

/// Libreria analizzata con la tassonomia v1: 1 = "a" netto, 2 = "a"+"b",
/// 3 = come 2 ma con "c" come correzione manuale.
fn library() -> (tempfile::TempDir, Store, Taxonomy) {
    let tmp = tempfile::tempdir().unwrap();
    let db = Store::open(&tmp.path().join("catalog.db"), Default::default()).unwrap();
    let tax = taxonomy();
    let concepts = concepts();
    let vectors = [
        image([1.2, 1.0, 1.0]),
        image([1.02, 1.01, 1.0]),
        image([1.02, 1.01, 1.0]),
    ];
    let t = tax.clone();
    db.write(move |c| {
        for (i, v) in vectors.iter().enumerate() {
            let id = i as i64 + 1;
            c.execute(
                "INSERT INTO media_item(id,path,name,ext,size,mtime,added_at,updated_at,thumb) VALUES (?1,?2,?2,'jpg',1,1,1,1,'t')",
                rusqlite::params![id, format!("f{id}.jpg")],
            )?;
            if id == 3 {
                c.execute("INSERT INTO label(media_id,grp,value,score,origin) VALUES (3,'subject','c',1,'manual')", [])?;
            }
            let labels = classify::labels(v, &t, &concepts);
            analysis::classification(c, id, &["subject".into()], &labels, v, MODEL, t.version)?;
        }
        Ok(())
    })
    .unwrap();
    (tmp, db, tax)
}

fn labels_of(db: &Store, id: i64) -> Vec<(String, String)> {
    db.read(move |c| {
        let mut q = c.prepare(
            "SELECT value,origin FROM label WHERE media_id=?1 AND grp='subject' ORDER BY value",
        )?;
        let v = q
            .query_map([id], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(v)
    })
    .unwrap()
}

fn pair(v: &str, o: &str) -> (String, String) {
    (v.into(), o.into())
}

fn apply(
    db: &Store,
    spec: RetuneSpec,
    clip: Option<&ClipInputs>,
    t: &AiThresholds,
) -> retune::Applied {
    retune::apply(
        db,
        &MemoryGovernor::new(256),
        spec,
        clip,
        t,
        Arc::new(FacePolicy::new(true)),
        &Default::default(),
        &JobControl::new(),
        None,
    )
    .unwrap()
}

#[test]
fn raising_a_category_threshold_relabels_from_saved_embeddings() {
    let (_t, db, tax) = library();
    assert_eq!(
        labels_of(&db, 2),
        vec![pair("a", "auto"), pair("b", "auto")]
    );
    // "b" passa a soglia propria 0,5: la tassonomia sale alla versione 2.
    let mut tuned = tax.clone();
    tuned.version = 2;
    tuned.groups[0].categories[1].threshold = Some(0.5);
    let concepts = concepts();
    let clip = ClipInputs {
        taxonomy: &tuned,
        concepts: &concepts,
        model: MODEL,
    };
    let stale = |db: &Store| {
        db.read(|c| analysis::stale_count(c, "clip", MODEL, Some(2)))
            .unwrap()
    };
    assert_eq!(stale(&db), 3);
    let t = AiThresholds::default();
    let preview = retune::preview(&db, Some(&clip), &t, &t, &JobControl::new()).unwrap();
    assert_eq!(
        preview.media_labels_changed, 1,
        "solo la foto 2 perde \"b\""
    );
    assert_eq!(labels_of(&db, 2).len(), 2, "l'anteprima non scrive");

    // Il retune non riceve alcun modello: lavora solo su embedding e concetti.
    let spec = RetuneSpec {
        clip: true,
        ..Default::default()
    };
    let done = apply(&db, spec, Some(&clip), &t);
    assert_eq!(done.media_labels_changed, preview.media_labels_changed);
    assert_eq!(labels_of(&db, 1), vec![pair("a", "auto")]);
    assert_eq!(labels_of(&db, 2), vec![pair("a", "auto")]);
    // Scenario "Correzione manuale preservata".
    assert_eq!(labels_of(&db, 3), vec![pair("c", "manual")]);
    assert_eq!(stale(&db), 0, "nessuna foto risulta da rianalizzare");
    // Ripresa: un secondo passaggio non trova nulla da rifare.
    assert_eq!(apply(&db, spec, Some(&clip), &t).media_labels_changed, 0);
}

#[test]
fn person_count_preview_matches_retune_without_reanalysis() {
    let (_t, db, _) = library();
    db.write(|c| {
        analysis::people_with(
            c,
            1,
            &[
                [0.1, 0.1, 0.1, 0.1, 0.9],
                [0.3, 0.1, 0.1, 0.1, 0.7],
                [0.5, 0.1, 0.1, 0.1, 0.45],
                [0.7, 0.1, 0.1, 0.1, 0.3],
            ],
            "yolox",
            0.4,
            Some(pc_ai::analyze::PERSON_FLOOR),
        )?;
        analysis::people_with(
            c,
            2,
            &[[0.1, 0.1, 0.1, 0.1, 0.95]],
            "yolox",
            0.4,
            Some(0.25),
        )
    })
    .unwrap();
    let count = |db: &Store| {
        db.read(|c| {
            Ok(
                c.query_row("SELECT count FROM media_people WHERE media_id=1", [], |r| {
                    r.get::<_, i64>(0)
                })?,
            )
        })
        .unwrap()
    };
    assert_eq!(count(&db), 3, "soglia 0,4: 0,9 / 0,7 / 0,45");
    let current = AiThresholds::default();
    let next = AiThresholds {
        person_detection: 0.6,
        ..current
    };
    let preview = retune::preview(&db, None, &current, &next, &JobControl::new()).unwrap();
    assert_eq!(preview.media_people_changed, 1);
    assert_eq!(preview.reanalysis_persons, 0, "0,6 sopra il pavimento 0,25");
    let spec = RetuneSpec::between(&current, &next);
    assert!(spec.persons && !spec.faces && !spec.clip);
    let done = apply(&db, spec, None, &next);
    assert_eq!(done.media_people_changed, preview.media_people_changed);
    assert_eq!(count(&db), 2, "soglia 0,6: 0,9 / 0,7");
}

#[test]
fn threshold_below_saved_floor_reports_files_to_reanalyse() {
    let (_t, db, _) = library();
    // Volti della foto 1 salvati prima dei pavimenti (minimo 0,8), foto 2 con pavimento 0,6.
    db.write(|c| {
        analysis::version(c, 1, "faces", "det+sface", None)?;
        analysis::version(c, 2, "faces", &analysis::with_floor("det+sface", 0.6), None)
    })
    .unwrap();
    let current = AiThresholds::default();
    let next = AiThresholds {
        face_detection: 0.7,
        ..current
    };
    let impact = retune::preview(&db, None, &current, &next, &JobControl::new()).unwrap();
    assert_eq!(impact.reanalysis_faces, 1, "solo la foto salvata a 0,8");
    let lower = AiThresholds {
        face_detection: 0.5,
        ..current
    };
    let impact = retune::preview(&db, None, &current, &lower, &JobControl::new()).unwrap();
    assert_eq!(impact.reanalysis_faces, 2);
}

#[test]
fn reanalysis_below_floor_marks_only_affected_files_as_stale() {
    let (_t, db, _) = library();
    db.write(|c| {
        analysis::version(c, 1, "faces", "det+sface", None)?;
        analysis::version(c, 2, "faces", &analysis::with_floor("det+sface", 0.6), None)?;
        analysis::version(c, 3, "persons", "yolox", None)
    })
    .unwrap();
    let t = AiThresholds {
        face_detection: 0.7,
        ..Default::default()
    };
    // Persone a 0,4 (= minimo storico): nessuna rianalisi; volti: solo la foto 1.
    assert_eq!(retune::prepare_reanalysis(&db, &t).unwrap(), vec![1]);
    let pending = db
        .read(|c| analysis::pending(c, &[1, 2], "faces", "det+sface", None))
        .unwrap();
    assert_eq!(pending, vec![1], "la foto 1 torna nel job di analisi");
}
