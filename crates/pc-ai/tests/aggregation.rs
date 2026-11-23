//! Raggruppamento in due passaggi (change face-identity-aggregation): vincoli della stessa
//! foto, primo passaggio deterministico e stabile, secondo passaggio con collegamento robusto,
//! gruppi piccoli verso le persone con nome, soglie, coda e misure.

use pc_ai::aggregate::{self, Face, Label, Mode, Params, People, QuestionKind};
use pc_ai::bench::{self, Rng};
use pc_ai::{cluster, face_policy::FacePolicy};
use pc_core::AiThresholds;
use pc_governor::MemoryGovernor;
use pc_jobs::JobControl;
use pc_store::merge_log::MergeOrigin;
use pc_store::{merge_log, people, review, Store};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

// ------------------------------------------------------------------ vettori di prova

fn normalize(v: &mut [f32]) {
    let n = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    v.iter_mut().for_each(|x| *x /= n);
}

fn unit(rng: &mut Rng) -> Vec<f32> {
    let mut v: Vec<f32> = (0..128).map(|_| rng.gauss() as f32).collect();
    normalize(&mut v);
    v
}

/// Vettore a coseno `rho` da `c`.
fn near(rng: &mut Rng, c: &[f32], rho: f32) -> Vec<f32> {
    let mut u = unit(rng);
    let d: f32 = u.iter().zip(c).map(|(a, b)| a * b).sum();
    u.iter_mut().zip(c).for_each(|(x, y)| *x -= d * y);
    normalize(&mut u);
    let s = (1.0 - rho * rho).sqrt();
    let mut v: Vec<f32> = c.iter().zip(&u).map(|(a, b)| rho * a + s * b).collect();
    normalize(&mut v);
    v
}

fn mix(a: &[f32], b: &[f32]) -> Vec<f32> {
    let mut v: Vec<f32> = a.iter().zip(b).map(|(x, y)| x + y).collect();
    normalize(&mut v);
    v
}

fn face(id: i64, media: i64, embedding: Vec<f32>) -> Face {
    Face {
        id,
        media,
        model: "sface".into(),
        embedding,
        quality: 0.9,
        ..Default::default()
    }
}

fn params() -> Params {
    Params::default()
}

fn run_pure(faces: &[Face], people: &People, mode: Mode) -> aggregate::Outcome {
    aggregate::aggregate(faces, people, params(), mode, &JobControl::new()).unwrap()
}

// ------------------------------------------------------------------ catalogo di prova

struct Lib {
    _tmp: tempfile::TempDir,
    db: Store,
}

fn library() -> Lib {
    let tmp = tempfile::tempdir().unwrap();
    let db = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
    Lib { _tmp: tmp, db }
}

/// Inserisce un volto (con la sua foto, se nuova) e restituisce l'id del volto.
fn insert_face(
    db: &Store,
    media: i64,
    embedding: &[f32],
    person: Option<i64>,
    manual: bool,
    captured: Option<&str>,
) -> i64 {
    let bytes: Vec<u8> = embedding.iter().flat_map(|x| x.to_le_bytes()).collect();
    let captured = captured.map(str::to_string);
    db.write(move |c| {
        c.execute(
            "INSERT OR IGNORE INTO media_item(id,path,name,ext,size,mtime,added_at,updated_at) VALUES (?1,?2,'t','jpg',1,1,1,1)",
            rusqlite::params![media, format!("m{media}.jpg")],
        )?;
        if let Some(at) = &captured {
            c.execute(
                "INSERT OR IGNORE INTO media_dates(media_id, captured_at) VALUES (?1, ?2)",
                rusqlite::params![media, at],
            )?;
        }
        let bbox = format!("[{},0.1,0.1,0.1]", 0.05 + 0.2 * (c.query_row("SELECT count(*) FROM face WHERE media_id=?1", [media], |r| r.get::<_, i64>(0))? as f32));
        c.execute(
            "INSERT INTO face(media_id,bbox,quality,sharpness,frontality,size_px,quality_score,embedding,person_id,assigned_by,model_ver)
             VALUES (?1,?2,0.97,0.9,0.95,120,0.9,?3,?4,?5,'sface')",
            rusqlite::params![media, bbox, bytes, person, if manual { "manual" } else { "auto" }],
        )?;
        Ok(c.last_insert_rowid())
    })
    .unwrap()
}

fn cluster_with(db: &Store, t: &AiThresholds) {
    cluster::run(
        db,
        &MemoryGovernor::new(1024),
        &JobControl::new(),
        Arc::new(FacePolicy::new(true)),
        &Default::default(),
        t,
    )
    .unwrap();
}

fn person_of(db: &Store) -> HashMap<i64, Option<i64>> {
    db.read(|c| {
        Ok(people::records(c, None)?
            .into_iter()
            .map(|f| (f.id, f.person_id))
            .collect())
    })
    .unwrap()
}

// ------------------------------------------------------------------ 2.1 stessa foto

#[test]
fn gemelli_nella_stessa_foto_restano_separati_e_non_sono_proposti() {
    // Scenario "Gemelli nella stessa foto": due volti identici (oltre "unisci da solo")
    // nella stessa foto.
    let mut rng = Rng::new(1);
    let c = unit(&mut rng);
    let mut faces = vec![face(1, 10, c.clone()), face(2, 10, c.clone())];
    // Altri volti dei due gemelli in foto diverse, tutti molto simili.
    for i in 0..6 {
        faces.push(face(3 + i, 20 + i, near(&mut rng, &c, 0.95)));
    }
    let out = run_pure(&faces, &People::default(), Mode::Full);
    assert_ne!(out.labels[0], out.labels[1], "mai nello stesso gruppo");
    for q in &out.questions {
        let pair = HashSet::from([q.a, q.b]);
        assert!(
            !(pair.contains(&out.labels[0]) && pair.contains(&out.labels[1])),
            "mai proposti come la stessa persona"
        );
    }
    // Nessuna unione li riunisce.
    for m in &out.merges {
        assert!(!(m.faces.contains(&0) && out.labels[1] == m.into));
        assert!(!(m.faces.contains(&1) && out.labels[0] == m.into));
    }
}

#[test]
fn suggerimenti_mai_tra_gruppi_con_volti_della_stessa_foto() {
    let lib = library();
    let mut rng = Rng::new(2);
    let c = unit(&mut rng);
    // Due gruppi da 3 volti identici, con una foto in comune.
    let (ga, gb) = lib
        .db
        .write(|c| {
            Ok((
                people::create_person(c, None)?,
                people::create_person(c, None)?,
            ))
        })
        .unwrap();
    for (i, g) in [(1, ga), (2, ga), (3, ga), (4, gb), (5, gb)] {
        insert_face(&lib.db, i, &c, Some(g), true, None);
    }
    insert_face(&lib.db, 1, &c, Some(gb), true, None); // stessa foto del primo di A
    let m = people::default_face_match();
    let sug = lib
        .db
        .write(move |c| {
            people::refresh_centroids_with(c, &m)?;
            people::refresh_merge_suggestions(c, &m)
        })
        .unwrap();
    assert!(
        sug.is_empty(),
        "gruppi con una foto in comune: persone diverse"
    );
}

// ------------------------------------------------------------------ 2.2 determinismo e stabilità

#[test]
fn stesso_input_stesso_risultato() {
    let samples = bench::synthetic(&bench::SynthConfig {
        identities: 40,
        largest: 60,
        ..Default::default()
    });
    let confirmed = bench::confirmed_mask(&samples, 5, 0.3, 1);
    let (_, a) = bench::run_new(&samples, &confirmed, params());
    let (_, b) = bench::run_new(&samples, &confirmed, params());
    assert_eq!(a.labels, b.labels);
    assert_eq!(a.merges, b.merges);
    assert_eq!(a.questions, b.questions);
}

#[test]
fn import_di_venti_volti_non_cambia_i_gruppi_non_toccati() {
    // Scenario "Import di poche foto" (D1, D5).
    let lib = library();
    let mut rng = Rng::new(5);
    let centers: Vec<Vec<f32>> = (0..10).map(|_| unit(&mut rng)).collect();
    let mut media = 0;
    for c in &centers {
        for _ in 0..15 {
            media += 1;
            insert_face(&lib.db, media, &near(&mut rng, c, 0.85), None, false, None);
        }
    }
    let t = AiThresholds::default();
    cluster_with(&lib.db, &t);
    let before = person_of(&lib.db);
    let rows_before: Vec<(i64, Option<i64>, i64)> = lib
        .db
        .read(|c| {
            let mut q = c.prepare(
                "SELECT p.id, p.cover_face_id, (SELECT count(*) FROM face f WHERE f.person_id=p.id) \
                 FROM person p ORDER BY p.id",
            )?;
            let v = q
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
                .collect::<Result<_, _>>()?;
            Ok(v)
        })
        .unwrap();
    // 20 volti nuovi: 10 di una persona nuova e 10 di due persone esistenti.
    let newcomer = unit(&mut rng);
    let mut fresh = Vec::new();
    for i in 0..20 {
        media += 1;
        let e = match i {
            0..10 => near(&mut rng, &newcomer, 0.85),
            10..15 => near(&mut rng, &centers[0], 0.85),
            _ => near(&mut rng, &centers[1], 0.85),
        };
        fresh.push(insert_face(&lib.db, media, &e, None, false, None));
    }
    cluster_with(&lib.db, &t);
    let after = person_of(&lib.db);
    for (face, p) in &before {
        assert_eq!(after[face], *p, "il volto {face} non cambia gruppo");
    }
    // I nuovi volti entrano nei gruppi esistenti o in un gruppo nuovo.
    let joined_0 = after[&fresh[10]];
    assert!(fresh[10..15].iter().all(|f| after[f] == joined_0));
    assert!(
        before.values().any(|p| *p == joined_0),
        "nel gruppo esistente"
    );
    let new_group = after[&fresh[0]];
    assert!(fresh[..10].iter().all(|f| after[f] == new_group));
    assert!(!before.values().any(|p| *p == new_group), "gruppo nuovo");
    // Gli altri gruppi: stesso id, copertina e numero di volti (stessa posizione).
    let touched: HashSet<Option<i64>> = fresh.iter().map(|f| after[f]).collect();
    let rows_after: HashMap<i64, (Option<i64>, i64)> = lib
        .db
        .read(|c| {
            let mut q = c.prepare(
                "SELECT p.id, p.cover_face_id, (SELECT count(*) FROM face f WHERE f.person_id=p.id) FROM person p",
            )?;
            let v = q
                .query_map([], |r| Ok((r.get(0)?, (r.get(1)?, r.get(2)?))))?
                .collect::<Result<_, _>>()?;
            Ok(v)
        })
        .unwrap();
    for (id, cover, n) in rows_before {
        if !touched.contains(&Some(id)) {
            assert_eq!(rows_after[&id], (cover, n), "gruppo {id} invariato");
        }
    }
    // Un'analisi senza volti nuovi non cambia nulla.
    cluster_with(&lib.db, &t);
    assert_eq!(person_of(&lib.db), after);
}

// ------------------------------------------------------------------ 2.3 identità dei gruppi

#[test]
fn ricalcolo_completo_conserva_id_e_copertina_dei_gruppi() {
    let lib = library();
    let mut rng = Rng::new(9);
    let mut media = 0;
    for _ in 0..3 {
        let c = unit(&mut rng);
        for _ in 0..8 {
            media += 1;
            insert_face(&lib.db, media, &near(&mut rng, &c, 0.85), None, false, None);
        }
    }
    cluster_with(&lib.db, &AiThresholds::default());
    let before = person_of(&lib.db);
    let group = before[&1].unwrap();
    let cover: i64 = lib
        .db
        .read(move |c| {
            Ok(c.query_row(
                "SELECT max(id) FROM face WHERE person_id=?1",
                [group],
                |r| r.get(0),
            )?)
        })
        .unwrap();
    lib.db
        .write(move |c| people::set_cover(c, group, cover))
        .unwrap();
    // Una sensibilità diversa forza il ricalcolo completo (etichette ripartite da zero).
    cluster_with(
        &lib.db,
        &AiThresholds {
            face_merge_sensitivity: 0.2,
            ..Default::default()
        },
    );
    assert_eq!(person_of(&lib.db), before, "stessi id per gli stessi volti");
    assert_eq!(
        lib.db
            .read(move |c| people::cover_face_id(c, group))
            .unwrap(),
        Some(cover)
    );
}

// ------------------------------------------------------------------ 3.1 volto ponte

#[test]
fn un_volto_ponte_non_unisce_due_gruppi_da_quaranta() {
    // Scenario "Un volto ponte non basta".
    let mut rng = Rng::new(11);
    let (a, b) = (unit(&mut rng), unit(&mut rng));
    let mut faces = Vec::new();
    for i in 0..40 {
        faces.push(face(i, i, near(&mut rng, &a, 0.88)));
        faces.push(face(100 + i, 100 + i, near(&mut rng, &b, 0.88)));
    }
    let bridge = mix(&a, &b);
    faces.push(face(999, 999, bridge.clone()));
    let out = run_pure(&faces, &People::default(), Mode::Full);
    let la: HashSet<Label> = (0..40).map(|i| out.labels[2 * i]).collect();
    let lb: HashSet<Label> = (0..40).map(|i| out.labels[2 * i + 1]).collect();
    assert!(la.is_disjoint(&lb), "i due gruppi restano separati");
    // Il volto ponte è molto simile a entrambi: col massimo (collegamento singolo) li unirebbe,
    // con la mediana no.
    let ia: Vec<usize> = (0..40).map(|i| 2 * i).chain([80]).collect();
    let ib: Vec<usize> = (0..40).map(|i| 2 * i + 1).collect();
    let max = ib
        .iter()
        .map(|j| pc_store::people::cosine(&bridge, &faces[*j].embedding))
        .fold(f32::MIN, f32::max);
    assert!(max >= params().auto, "il ponte supera da solo la soglia");
    assert!(aggregate::linkage_of(&faces, &ia, &ib).unwrap() < params().auto);
}

// ------------------------------------------------------------------ 3.2 secondo passaggio

/// Un gruppo protetto di 30 volti e quattro frammenti di 1–2 volti molto vicini al gruppo ma
/// non tra loro (i loro vicini kNN sono tutti volti del gruppo grande).
fn fragments(lib: &Lib, rng: &mut Rng) -> (i64, Vec<i64>) {
    let c = unit(rng);
    let p = lib.db.write(|c| people::create_person(c, None)).unwrap();
    for m in 1..=30 {
        insert_face(&lib.db, m, &near(rng, &c, 0.97), Some(p), true, None);
    }
    let mut frag = Vec::new();
    let mut media = 100;
    for k in 0..4 {
        let dir = near(rng, &c, 0.8);
        for _ in 0..(1 + k % 2) {
            media += 1;
            frag.push(insert_face(
                &lib.db,
                media,
                &near(rng, &dir, 0.99),
                None,
                false,
                None,
            ));
        }
    }
    (p, frag)
}

#[test]
fn frammenti_uniti_al_secondo_passaggio() {
    // Scenario "Frammenti uniti al secondo passaggio".
    let lib = library();
    let mut rng = Rng::new(21);
    let (p, frag) = fragments(&lib, &mut rng);
    assert_eq!(frag.len(), 6);
    cluster_with(&lib.db, &AiThresholds::default());
    let after = person_of(&lib.db);
    assert!(frag.iter().all(|f| after[f] == Some(p)), "un solo gruppo");
    assert_eq!(after.values().filter(|x| **x == Some(p)).count(), 36);
    let log = lib.db.read(|c| merge_log::list(c, None, 50)).unwrap();
    assert_eq!(log.len(), 4, "le quattro unioni automatiche nello storico");
    assert!(log
        .iter()
        .all(|e| e.origin == MergeOrigin::Auto && e.target == p));
    // Prima riaggregazione dopo l'aggiornamento (c'erano già volti assegnati): riepilogo.
    let summary = lib
        .db
        .read(review::first_summary)
        .unwrap()
        .expect("riepilogo");
    assert_eq!(summary.auto_merges, 4);
    lib.db.write(review::dismiss_first_summary).unwrap();
    // Una nuova analisi non registra di nuovo le stesse unioni né un nuovo riepilogo.
    cluster_with(&lib.db, &AiThresholds::default());
    assert_eq!(
        lib.db.read(|c| merge_log::list(c, None, 50)).unwrap().len(),
        4
    );
    assert!(lib.db.read(review::first_summary).unwrap().is_none());
}

#[test]
fn unione_automatica_annullata_non_si_ripete() {
    // Scenario "Unione automatica annullata", attraverso l'analisi.
    let lib = library();
    let mut rng = Rng::new(22);
    let (p, frag) = fragments(&lib, &mut rng);
    cluster_with(&lib.db, &AiThresholds::default());
    let entry = lib
        .db
        .read(|c| merge_log::list(c, None, 50))
        .unwrap()
        .into_iter()
        .find(|e| e.faces == 2)
        .unwrap();
    lib.db.write(move |c| merge_log::undo(c, entry.id)).unwrap();
    let back = person_of(&lib.db);
    let src = entry.sources[0].person;
    let moved: Vec<&i64> = frag.iter().filter(|f| back[f] == Some(src)).collect();
    assert_eq!(moved.len(), 2, "il gruppo torna com'era");
    // Ricalcolo completo e incrementale: non si riunisce alla stessa persona.
    cluster_with(
        &lib.db,
        &AiThresholds {
            face_merge_sensitivity: 0.6,
            ..Default::default()
        },
    );
    let again = person_of(&lib.db);
    assert!(moved.iter().all(|f| again[f] != Some(p)));
}

#[test]
fn margine_per_le_date_lontane() {
    // Collegamento tra "unisci da solo" e "unisci da solo + 0,05": unione automatica solo se
    // le foto più vicine nel tempo dei due gruppi distano meno di 3 anni.
    let build = |years_apart: i64| {
        let mut rng = Rng::new(31);
        let c = unit(&mut rng);
        let year = 365 * 24 * 3600 * 1000i64;
        let people = People {
            protected: HashSet::from([7]),
            ..Default::default()
        };
        let mut faces = Vec::new();
        for i in 0..20 {
            let mut f = face(i, i, near(&mut rng, &c, 0.95));
            f.person = Some(7);
            f.time = Some(10 * year);
            faces.push(f);
        }
        // Un gruppo di 4 volti a collegamento ≈ 0,57 dal gruppo protetto.
        let dir = near(&mut rng, &c, 0.6);
        for i in 0..4 {
            let mut f = face(100 + i, 100 + i, near(&mut rng, &dir, 0.995));
            f.time = Some((10 + years_apart) * year);
            faces.push(f);
        }
        let out = run_pure(&faces, &people, Mode::Full);
        let link = aggregate::linkage_of(&faces, &(0..20).collect::<Vec<_>>(), &[20, 21, 22, 23]);
        (out, link.unwrap())
    };
    let (near_out, link) = build(1);
    assert!((0.55..0.60).contains(&link), "collegamento di prova {link}");
    assert_eq!(
        near_out.labels[20],
        Label::Person(7),
        "1 anno: unione automatica"
    );
    let (far_out, _) = build(4);
    assert_ne!(
        far_out.labels[20],
        Label::Person(7),
        "3 anni: margine in più"
    );
    assert!(
        far_out
            .questions
            .iter()
            .any(|q| q.a == far_out.labels[20] && q.b == Label::Person(7)),
        "la coppia diventa una domanda"
    );
}

// ------------------------------------------------------------------ 3.3 gruppi piccoli

#[test]
fn candidata_chiara_unita_e_registrata() {
    // Scenario "Candidata chiara".
    let lib = library();
    let mut rng = Rng::new(41);
    let (gioia_c, other_c) = (unit(&mut rng), unit(&mut rng));
    let (gioia, other, small) = lib
        .db
        .write(|c| {
            Ok((
                people::create_person(c, Some("Gioia"))?,
                people::create_person(c, Some("Paolo"))?,
                people::create_person(c, None)?,
            ))
        })
        .unwrap();
    for m in 1..=12 {
        insert_face(
            &lib.db,
            m,
            &near(&mut rng, &gioia_c, 0.95),
            Some(gioia),
            true,
            None,
        );
        insert_face(
            &lib.db,
            100 + m,
            &near(&mut rng, &other_c, 0.95),
            Some(other),
            true,
            None,
        );
    }
    let dir = near(&mut rng, &gioia_c, 0.75);
    let f1 = insert_face(
        &lib.db,
        201,
        &near(&mut rng, &dir, 0.99),
        Some(small),
        false,
        None,
    );
    let f2 = insert_face(
        &lib.db,
        202,
        &near(&mut rng, &dir, 0.99),
        Some(small),
        false,
        None,
    );
    cluster_with(&lib.db, &AiThresholds::default());
    let after = person_of(&lib.db);
    assert_eq!((after[&f1], after[&f2]), (Some(gioia), Some(gioia)));
    let log = lib
        .db
        .read(|c| merge_log::list(c, Some(gioia), 10))
        .unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].origin, MergeOrigin::Auto);
    assert_eq!(log[0].sources[0].person, small, "Gruppo di 2 volti → Gioia");
    assert_eq!(log[0].faces, 2);
}

#[test]
fn due_candidate_vicine_diventano_una_domanda() {
    // Scenario "Due candidate vicine".
    let lib = library();
    let mut rng = Rng::new(42);
    let (anna_c, marta_c) = (unit(&mut rng), unit(&mut rng));
    let (anna, marta) = lib
        .db
        .write(|c| {
            Ok((
                people::create_person(c, Some("Anna"))?,
                people::create_person(c, Some("Marta"))?,
            ))
        })
        .unwrap();
    for m in 1..=12 {
        insert_face(
            &lib.db,
            m,
            &near(&mut rng, &anna_c, 0.97),
            Some(anna),
            true,
            None,
        );
        insert_face(
            &lib.db,
            100 + m,
            &near(&mut rng, &marta_c, 0.97),
            Some(marta),
            true,
            None,
        );
    }
    let lone = insert_face(&lib.db, 300, &mix(&anna_c, &marta_c), None, false, None);
    cluster_with(&lib.db, &AiThresholds::default());
    let after = person_of(&lib.db);
    assert!(
        after[&lone] != Some(anna) && after[&lone] != Some(marta),
        "non assegnato"
    );
    let queue = lib
        .db
        .read(|c| review::queue(c, &people::default_face_match(), 50))
        .unwrap();
    let choice = queue.items.iter().find_map(|i| match i {
        review::ReviewItem::Single { question }
            if question.kind == review::QuestionKind::GroupChoice =>
        {
            Some(question.clone())
        }
        _ => None,
    });
    let q = choice.expect("domanda \"È Anna o Marta?\"");
    let options = HashSet::from([q.other.id, q.second.as_ref().unwrap().id]);
    assert_eq!(options, HashSet::from([anna, marta]));
    assert_eq!(Some(q.group.id), after[&lone]);
}

#[test]
fn gruppo_di_soli_volti_di_qualita_bassa_non_si_unisce_da_solo() {
    let mut rng = Rng::new(43);
    let c = unit(&mut rng);
    let people = People {
        protected: HashSet::from([5]),
        named: HashSet::from([5]),
        ..Default::default()
    };
    let mut faces: Vec<Face> = (0..12)
        .map(|i| {
            let mut f = face(i, i, near(&mut rng, &c, 0.95));
            f.person = Some(5);
            f
        })
        .collect();
    let dir = near(&mut rng, &c, 0.8);
    for i in 0..2 {
        let mut f = face(50 + i, 50 + i, near(&mut rng, &dir, 0.99));
        f.quality = 0.3;
        faces.push(f);
    }
    let out = run_pure(&faces, &people, Mode::Full);
    assert_ne!(
        out.labels[12],
        Label::Person(5),
        "qualità bassa: niente unione automatica"
    );
    assert!(out.questions.iter().any(|q| q.a == out.labels[12]
        && q.b == Label::Person(5)
        && q.kind == QuestionKind::GroupPerson));
    // Lo stesso gruppo di buona qualità si unisce.
    for f in &mut faces[12..] {
        f.quality = 0.9;
    }
    let out = run_pure(&faces, &people, Mode::Full);
    assert_eq!(out.labels[12], Label::Person(5));
}

// ------------------------------------------------------------------ 3.4 banco di prova

#[test]
fn banco_di_prova_migliora_la_bcubed_senza_unioni_sbagliate() {
    // Regressione (D9): dati sintetici con le distribuzioni misurate sulla libreria di prova.
    let samples = bench::synthetic(&bench::SynthConfig::default());
    let confirmed = bench::confirmed_mask(&samples, 12, 0.3, 42);
    let legacy = bench::measure(
        &samples,
        &bench::legacy_cluster(&samples, &confirmed, params().ask),
        None,
    );
    let (_, out) = bench::run_new(&samples, &confirmed, params());
    let new = bench::measure(&samples, &out.labels, Some(&out));
    assert!(
        new.bcubed_f > legacy.bcubed_f + 0.05,
        "BCubed F: prima {:.4}, dopo {:.4}",
        legacy.bcubed_f,
        new.bcubed_f
    );
    assert!(new.auto_precision >= 0.99);
    // Con una soglia "unisci da solo" più bassa le unioni automatiche ci sono e restano giuste.
    let low = Params {
        auto: 0.38,
        ..params()
    };
    let (_, out) = bench::run_new(&samples, &confirmed, low);
    let m = bench::measure(&samples, &out.labels, Some(&out));
    assert!(m.auto_merges > 0);
    assert!(m.auto_precision >= 0.99, "precisione {}", m.auto_precision);
}

// ------------------------------------------------------------------ 3.5 prestazioni

#[test]
fn raggruppamento_interrompibile() {
    let samples = bench::synthetic(&bench::SynthConfig {
        identities: 60,
        largest: 80,
        ..Default::default()
    });
    let (faces, people) = bench::to_faces(&samples, &vec![false; samples.len()]);
    let control = JobControl::new();
    control.cancel();
    let err = aggregate::aggregate(&faces, &people, params(), Mode::Full, &control).unwrap_err();
    assert_eq!(err.code, pc_core::ErrorCode::Cancelled);
}

/// 100.000 volti sintetici: tempo e memoria del raggruppamento. Ignorato di default:
///   cargo test -p pc-ai --test aggregation -- --ignored --nocapture centomila
#[test]
#[ignore]
fn centomila_volti_entro_il_budget() {
    use pc_governor::{SysinfoProbe, SystemProbe};
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    // Σ k^-0,5 per 10.000 identità ≈ 200: circa 104.000 volti.
    let cfg = bench::SynthConfig {
        identities: 10_000,
        largest: 520,
        zipf: 0.5,
        ..Default::default()
    };
    let samples = bench::synthetic(&cfg);
    let n = samples.len();
    println!("volti sintetici: {n}");
    assert!(n >= 100_000, "{n}");
    let (faces, people) = bench::to_faces(&samples, &vec![false; n]);
    drop(samples);
    // Stima del governatore usata da `cluster::run` (5 KiB per volto + 16 MB).
    let budget_mb = (n as u64 * 5120).div_ceil(1024 * 1024) + 16;
    let gov = MemoryGovernor::new(budget_mb + 64);
    let permit = gov.try_acquire(budget_mb).expect("budget del governatore");
    // Memoria del processo: picco campionato durante il raggruppamento, oltre alla base.
    let probe = Arc::new(SysinfoProbe::default());
    let base = probe.snapshot().process_mb;
    let peak = Arc::new(AtomicU64::new(base));
    let done = Arc::new(AtomicBool::new(false));
    let sampler = {
        let (probe, peak, done) = (probe.clone(), peak.clone(), done.clone());
        std::thread::spawn(move || {
            while !done.load(Ordering::Relaxed) {
                peak.fetch_max(probe.snapshot().process_mb, Ordering::Relaxed);
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        })
    };
    let t = std::time::Instant::now();
    let out =
        aggregate::aggregate(&faces, &people, params(), Mode::Full, &JobControl::new()).unwrap();
    let elapsed = t.elapsed();
    done.store(true, Ordering::Relaxed);
    sampler.join().unwrap();
    drop(permit);
    let used = peak.load(Ordering::Relaxed).saturating_sub(base);
    let groups: HashSet<_> = out.labels.iter().collect();
    println!(
        "100k: {:.1?}, {} gruppi, {} unioni automatiche, {} domande; memoria in più al picco \
         {used} MB (base {base} MB), stima del governatore {budget_mb} MB",
        elapsed,
        groups.len(),
        out.merges.len(),
        out.questions.len(),
    );
    assert!(
        used <= budget_mb,
        "picco {used} MB oltre la stima {budget_mb} MB"
    );
    // Interrompibile: dopo l'annullamento torna entro pochi secondi.
    let control = JobControl::new();
    let stopper = {
        let control = control.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(2));
            control.cancel();
            std::time::Instant::now()
        })
    };
    let err = aggregate::aggregate(&faces, &people, params(), Mode::Full, &control).unwrap_err();
    let stopped = stopper.join().unwrap().elapsed();
    println!("annullamento: risposta in {stopped:.2?}");
    assert_eq!(err.code, pc_core::ErrorCode::Cancelled);
    assert!(stopped < std::time::Duration::from_secs(5));
}

// ------------------------------------------------------------------ 4.x soglie

#[test]
fn sensibilita_ridotta_chiede_invece_di_unire() {
    // Scenario "Sensibilità ridotta": un gruppo a collegamento ≈ 0,6 da Gioia si unisce con la
    // sensibilità predefinita e diventa una domanda verso "Più domande".
    let setup = || {
        let lib = library();
        let mut rng = Rng::new(51);
        let c = unit(&mut rng);
        let gioia = lib
            .db
            .write(|c| people::create_person(c, Some("Gioia")))
            .unwrap();
        for m in 1..=12 {
            insert_face(
                &lib.db,
                m,
                &near(&mut rng, &c, 0.97),
                Some(gioia),
                true,
                None,
            );
        }
        let dir = near(&mut rng, &c, 0.64);
        let small = lib.db.write(|c| people::create_person(c, None)).unwrap();
        let f = insert_face(
            &lib.db,
            99,
            &near(&mut rng, &dir, 0.99),
            Some(small),
            false,
            None,
        );
        insert_face(
            &lib.db,
            98,
            &near(&mut rng, &dir, 0.99),
            Some(small),
            false,
            None,
        );
        (lib, gioia, f)
    };
    let (lib, gioia, f) = setup();
    cluster_with(&lib.db, &AiThresholds::default());
    assert_eq!(
        person_of(&lib.db)[&f],
        Some(gioia),
        "sensibilità predefinita: unione"
    );
    let (lib, gioia, f) = setup();
    cluster_with(
        &lib.db,
        &AiThresholds {
            face_merge_sensitivity: 0.0,
            ..Default::default()
        },
    );
    assert_ne!(
        person_of(&lib.db)[&f],
        Some(gioia),
        "più domande: niente unione"
    );
    let queue = lib
        .db
        .read(|c| review::queue(c, &people::default_face_match(), 50))
        .unwrap();
    assert_eq!(queue.open, 1, "la coppia esclusa compare come domanda");
    let used = lib.db.read(review::thresholds).unwrap().unwrap();
    assert!((used.auto - 0.65).abs() < 1e-6);
    assert!((used.sensitivity - 0.0).abs() < 1e-6);
}

#[test]
fn anteprima_delle_unioni_automatiche() {
    let lib = library();
    let mut rng = Rng::new(23);
    fragments(&lib, &mut rng);
    let p = cluster::preview(&lib.db, &AiThresholds::default(), &JobControl::new()).unwrap();
    assert_eq!(p.auto_merges, 4);
    // L'anteprima non scrive.
    assert!(lib
        .db
        .read(|c| merge_log::list(c, None, 10))
        .unwrap()
        .is_empty());
}

// ------------------------------------------------------------------ 6.1 ordine della coda

#[test]
fn ordine_misto_riduce_le_domande_rispetto_alla_probabilita() {
    // D6: sui dati sintetici l'ordine misto (gruppi grandi per volti risolti attesi, poi i
    // piccoli per probabilità) arriva alla BCubed F 0,95 con meno domande dell'ordine per sola
    // probabilità, e non ne richiede mai di più per arrivare a metà del guadagno possibile.
    let run = |seed: u64, far: (f32, f32)| {
        let samples = bench::synthetic(&bench::SynthConfig {
            seed,
            far,
            ..Default::default()
        });
        let confirmed = bench::confirmed_mask(&samples, 12, 0.3, seed);
        let (_, out) = bench::run_new(&samples, &confirmed, params());
        let mut by_prob = out.questions.clone();
        aggregate::order_by_probability(&mut by_prob);
        (
            bench::trajectory(&samples, &out, &out.questions),
            bench::trajectory(&samples, &out, &by_prob),
        )
    };
    let reach = |t: &[f64], target: f64| t.iter().position(|f| *f >= target);
    let (mixed, prob) = run(1, (0.78, 0.9));
    let (m95, p95) = (reach(&mixed, 0.95).unwrap(), reach(&prob, 0.95).unwrap());
    assert!(
        m95 < p95,
        "BCubed F 0,95: misto {m95}, per probabilità {p95}"
    );
    for (seed, far) in [(1, (0.68, 0.8)), (2, (0.68, 0.8)), (42, (0.62, 0.72))] {
        let (mixed, prob) = run(seed, far);
        let half = mixed[0] + (mixed.last().unwrap() - mixed[0]) / 2.0;
        let (m, p) = (reach(&mixed, half).unwrap(), reach(&prob, half).unwrap());
        assert!(
            m < p,
            "seme {seed}: metà del guadagno con {m} domande contro {p}"
        );
    }
}

// ------------------------------------------------------------------ 7.1 misure

#[test]
fn misure_salvate_dopo_ogni_analisi_e_soglie_predefinite_con_pochi_dati() {
    // Scenario "Libreria con pochi dati" e misure (D8).
    let lib = library();
    let mut rng = Rng::new(61);
    let c = unit(&mut rng);
    for m in 1..=6 {
        insert_face(&lib.db, m, &near(&mut rng, &c, 0.9), None, false, None);
    }
    insert_face(&lib.db, 50, &unit(&mut rng), None, false, None);
    cluster_with(&lib.db, &AiThresholds::default());
    let saved = lib.db.read(review::saved_metrics).unwrap().expect("misure");
    let t = saved.thresholds.expect("soglie");
    assert!(!t.calibrated, "meno di 200 coppie: soglie predefinite");
    assert_eq!(t.same_photo_pairs, 0);
    assert!((t.ask - 0.363).abs() < 1e-6 && (t.auto - 0.55).abs() < 1e-6);
    assert_eq!(saved.unnamed_groups, 2);
    assert!((saved.single_face_share - 0.5).abs() < 1e-6);
    assert_eq!(saved.auto_merges, 0);
}
