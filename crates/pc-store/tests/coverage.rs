use pc_store::coverage::{coverage, ActiveTargets, Stage, StageCoverage, StageKeys, Target};
use pc_store::{Store, StoreOptions};
use rusqlite::Connection;
use std::time::Instant;

fn store(dir: &std::path::Path) -> Store {
    Store::open(&dir.join("catalog.db"), StoreOptions::default()).unwrap()
}

/// `n` foto analizzabili (id 1..=n).
fn seed(c: &Connection, n: i64) {
    {
        let mut m = c
            .prepare("INSERT INTO media_item(id, path, name, ext, size, mtime, added_at, updated_at, thumb) VALUES (?1, ?2, ?3, 'jpg', 1, 0, ?1, ?1, 't')")
            .unwrap();
        for i in 1..=n {
            m.execute(rusqlite::params![
                i,
                format!("/p/{i}.jpg"),
                format!("{i}.jpg")
            ])
            .unwrap();
        }
    }
}

fn mark(
    c: &Connection,
    ids: std::ops::RangeInclusive<i64>,
    stage: &str,
    ver: &str,
    tax: Option<i64>,
) {
    let mut q = c
        .prepare("INSERT INTO analysis_version(media_id,stage,model_ver,taxonomy_ver) VALUES (?1,?2,?3,?4)")
        .unwrap();
    for i in ids {
        q.execute(rusqlite::params![i, stage, ver, tax]).unwrap();
    }
}

fn keys() -> StageKeys {
    StageKeys {
        classification: Some(("clip-b".into(), 3)),
        persons: Some("det-1".into()),
        faces: Some("face-1".into()),
        similar: Some("sim-1".into()),
    }
}

fn of(v: &[StageCoverage], s: Stage) -> (i64, i64, i64, i64, i64) {
    let c = v.iter().find(|x| x.stage == s).unwrap();
    assert_eq!(c.current + c.stale + c.queued + c.never, c.total);
    (c.current, c.stale, c.queued, c.never, c.total)
}

/// 10.000 foto, 7.000 classificate di cui 500 con un modello precedente.
fn classified(c: &Connection) {
    seed(c, 10_000);
    mark(c, 1..=6_500, "clip", "clip-b", Some(3));
    mark(c, 6_501..=7_000, "clip", "clip-a", Some(3));
}

#[test]
fn conta_correnti_vecchie_mai() {
    let d = tempfile::tempdir().unwrap();
    let s = store(d.path());
    let r = s
        .write(move |c| {
            classified(c);
            coverage(c, &keys(), &ActiveTargets::default())
        })
        .unwrap();
    assert_eq!(
        of(&r, Stage::Classification),
        (6_500, 500, 0, 3_000, 10_000)
    );
    assert_eq!(of(&r, Stage::Persons), (0, 0, 0, 10_000, 10_000));
}

#[test]
fn pavimento_nella_versione_e_tassonomia() {
    let d = tempfile::tempdir().unwrap();
    let s = store(d.path());
    let r = s
        .write(move |c| {
            seed(c, 4);
            mark(c, 1..=1, "persons", "det-1|floor=0.25", None);
            mark(c, 2..=2, "persons", "det-10", None);
            // classificazione con zero etichette: conta come corrente
            mark(c, 1..=1, "clip", "clip-b", Some(3));
            mark(c, 2..=2, "clip", "clip-b", Some(2));
            coverage(c, &keys(), &ActiveTargets::default())
        })
        .unwrap();
    assert_eq!(of(&r, Stage::Persons), (1, 1, 0, 2, 4));
    assert_eq!(of(&r, Stage::Classification), (1, 1, 0, 2, 4));
}

#[test]
fn job_attivo_su_id_toglie_dai_mai() {
    let d = tempfile::tempdir().unwrap();
    let s = store(d.path());
    let act = ActiveTargets {
        classification: Target::Ids((9_001..=9_100).collect()),
        ..Default::default()
    };
    let r = s
        .write(move |c| {
            classified(c);
            coverage(c, &keys(), &act)
        })
        .unwrap();
    assert_eq!(
        of(&r, Stage::Classification),
        (6_500, 500, 100, 2_900, 10_000)
    );
}

#[test]
fn job_su_tutta_la_libreria() {
    let d = tempfile::tempdir().unwrap();
    let s = store(d.path());
    let act = ActiveTargets {
        classification: Target::Library,
        ..Default::default()
    };
    let r = s
        .write(move |c| {
            classified(c);
            coverage(c, &keys(), &act)
        })
        .unwrap();
    assert_eq!(of(&r, Stage::Classification), (6_500, 0, 3_500, 0, 10_000));
}

#[test]
fn chiave_assente_nessuna_corrente() {
    let d = tempfile::tempdir().unwrap();
    let s = store(d.path());
    let r = s
        .write(move |c| {
            classified(c);
            coverage(c, &StageKeys::default(), &ActiveTargets::default())
        })
        .unwrap();
    assert_eq!(of(&r, Stage::Classification), (0, 7_000, 0, 3_000, 10_000));
}

#[test]
fn cestino_escluso_dal_totale() {
    let d = tempfile::tempdir().unwrap();
    let s = store(d.path());
    let r = s
        .write(move |c| {
            seed(c, 100);
            c.execute(
                "INSERT INTO catalog_item(catalog_id, media_id) SELECT c.id, m.id FROM catalog c, media_item m WHERE c.system='trash' AND m.id<=10",
                [],
            )
            .unwrap();
            c.execute("UPDATE media_item SET status='missing' WHERE id BETWEEN 11 AND 15", [])
                .unwrap();
            c.execute("UPDATE media_item SET thumb=NULL WHERE id BETWEEN 16 AND 20", [])
                .unwrap();
            coverage(c, &keys(), &ActiveTargets::default())
        })
        .unwrap();
    assert_eq!(of(&r, Stage::Faces).4, 80);
}

#[test]
fn similari_e_impronta() {
    let d = tempfile::tempdir().unwrap();
    let s = store(d.path());
    let r = s
        .write(move |c| {
            seed(c, 10);
            c.execute("UPDATE media_item SET phash=7 WHERE id<=6", []).unwrap();
            c.execute("INSERT INTO similar_done(media_id,params) SELECT id,'sim-1' FROM media_item WHERE id<=3", []).unwrap();
            c.execute("INSERT INTO similar_done(media_id,params) SELECT id,'sim-0' FROM media_item WHERE id BETWEEN 4 AND 5", []).unwrap();
            coverage(c, &keys(), &ActiveTargets::default())
        })
        .unwrap();
    assert_eq!(of(&r, Stage::Similar), (3, 2, 0, 5, 10));
    assert_eq!(of(&r, Stage::Phash), (6, 0, 0, 4, 10));
}

#[test]
#[ignore = "prestazioni: cargo test -p pc-store --release --test coverage -- --ignored --nocapture"]
fn perf_100k_cinque_fasi() {
    let d = tempfile::tempdir().unwrap();
    let s = store(d.path());
    s.write(move |c| {
        seed(c, 100_000);
        mark(c, 1..=70_000, "clip", "clip-b", Some(3));
        mark(c, 1..=60_000, "persons", "det-1|floor=0.25", None);
        mark(c, 1..=50_000, "faces", "face-1", None);
        c.execute("UPDATE media_item SET phash=1 WHERE id<=90000", [])?;
        c.execute("INSERT INTO similar_done(media_id,params) SELECT id,'sim-1' FROM media_item WHERE id<=40000", [])?;
        Ok(())
    })
    .unwrap();
    let act = ActiveTargets {
        faces: Target::Ids((50_001..=60_000).collect()),
        ..Default::default()
    };
    let t = Instant::now();
    let r = s.write(move |c| coverage(c, &keys(), &act)).unwrap();
    let ms = t.elapsed().as_millis();
    println!("copertura 100k x 5 fasi: {ms} ms");
    assert_eq!(r.len(), 5);
    assert_eq!(of(&r, Stage::Faces), (50_000, 0, 10_000, 40_000, 100_000));
    assert!(ms < 150, "troppo lento: {ms} ms");
}

/// "Dalla Panoramica alla Libreria": senza job attivi, il conteggio di ogni fase e stato
/// è uguale a quello della Libreria con il filtro `analysis` corrispondente, anche con
/// foto nel Cestino, file mancanti e foto senza miniatura che hanno già dei marcatori.
#[test]
fn conteggi_uguali_al_filtro_della_libreria() {
    use pc_store::query::{self, AnalysisFilter, AnalysisFilterState, LibraryFilter};
    let d = tempfile::tempdir().unwrap();
    let s = store(d.path());
    s.write(|c| {
        seed(c, 60);
        mark(c, 1..=20, "clip", "clip-b", Some(3));
        mark(c, 21..=30, "clip", "clip-a", Some(3));
        mark(c, 1..=15, "persons", "det-1|floor=0.25", None);
        mark(c, 16..=25, "persons", "det-0", None);
        mark(c, 5..=35, "faces", "face-1", None);
        c.execute("UPDATE media_item SET phash=1 WHERE id<=40", [])?;
        c.execute("INSERT INTO similar_done(media_id,params) SELECT id,'sim-1' FROM media_item WHERE id<=12", [])?;
        c.execute("INSERT INTO similar_done(media_id,params) SELECT id,'sim-0' FROM media_item WHERE id BETWEEN 13 AND 22", [])?;
        // Cestino (con e senza marcatori), file mancanti, senza miniatura.
        c.execute(
            "INSERT INTO catalog_item(catalog_id, media_id) SELECT c.id, m.id FROM catalog c, media_item m WHERE c.system='trash' AND (m.id IN (1,2,50,51))",
            [],
        )?;
        c.execute("UPDATE media_item SET status='missing' WHERE id IN (3,21,52)", [])?;
        c.execute("UPDATE media_item SET thumb=NULL WHERE id IN (4,22,53)", [])?;
        Ok(())
    })
    .unwrap();
    let cov = s
        .read(|c| coverage(c, &keys(), &ActiveTargets::default()))
        .unwrap();
    for stage in Stage::ALL {
        let (cur, stale, _, never, _) = of(&cov, stage);
        for (state, want) in [
            (AnalysisFilterState::Current, cur),
            (AnalysisFilterState::Stale, stale),
            (AnalysisFilterState::Never, never),
        ] {
            let f = LibraryFilter {
                analysis: Some(AnalysisFilter { stage, state }),
                stage_keys: Some(keys()),
                ..Default::default()
            };
            let got = s.read(|c| query::count(c, &f)).unwrap();
            assert_eq!(got, want, "{stage:?} {state:?}");
        }
    }
}
