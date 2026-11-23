//! Storico delle unioni, coda "Da confermare" e propagazione delle risposte
//! (change face-identity-aggregation, design D6–D8).

use pc_store::merge_log::{self, MergeOrigin};
use pc_store::review::{self, Answer, NewQuestion, QuestionKind, ReviewItem};
use pc_store::{bulk, migrate, people};
use rusqlite::Connection;

fn db() -> Connection {
    let mut c = Connection::open_in_memory().unwrap();
    migrate::migrate(&mut c, None).unwrap();
    c
}

/// Crea un gruppo di `n` volti, ciascuno in una foto propria.
fn group(c: &Connection, n: usize, name: Option<&str>) -> i64 {
    let p = people::create_person(c, name).unwrap();
    for _ in 0..n {
        let media: i64 = c
            .query_row("SELECT COALESCE(MAX(id),0)+1 FROM media_item", [], |r| {
                r.get(0)
            })
            .unwrap();
        c.execute(
            "INSERT INTO media_item(id,path,name,ext,size,mtime,added_at,updated_at) VALUES (?1,?2,'t','jpg',1,1,1,1)",
            rusqlite::params![media, format!("m{media}.jpg")],
        )
        .unwrap();
        let mut e = vec![0.0f32; 128];
        e[(media as usize) % 128] = 1.0;
        let bytes: Vec<u8> = e.iter().flat_map(|x| x.to_le_bytes()).collect();
        c.execute(
            "INSERT INTO face(media_id,bbox,quality,quality_score,embedding,person_id,assigned_by,model_ver)
             VALUES (?1,'[0.1,0.1,0.2,0.2]',0.95,0.9,?2,?3,'auto','sface')",
            rusqlite::params![media, bytes, p],
        )
        .unwrap();
    }
    p
}

fn faces_of(c: &Connection, p: i64) -> Vec<i64> {
    let mut q = c
        .prepare("SELECT id FROM face WHERE person_id=?1 ORDER BY id")
        .unwrap();
    q.query_map([p], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn exists(c: &Connection, p: i64) -> bool {
    c.query_row(
        "SELECT EXISTS(SELECT 1 FROM person WHERE id=?1)",
        [p],
        |r| r.get(0),
    )
    .unwrap()
}

#[test]
fn gruppo_unito_a_mano_si_ritrova_nelle_unioni_recenti_e_si_annulla() {
    // Scenario "Ritrovare un gruppo unito".
    let c = db();
    let gioia = group(&c, 5, Some("Gioia"));
    let small = group(&c, 3, None);
    let small_faces = faces_of(&c, small);
    people::set_cover(&c, small, small_faces[1]).unwrap();
    people::merge_logged(
        &c,
        &[small],
        gioia,
        MergeOrigin::Manual,
        "Unisci persone",
        false,
    )
    .unwrap();
    assert!(!exists(&c, small), "il gruppo non è più in Da rivedere");
    let log = merge_log::list(&c, None, 50).unwrap();
    assert_eq!(log.len(), 1);
    let e = &log[0];
    assert_eq!(e.origin, MergeOrigin::Manual);
    assert_eq!(e.target, gioia);
    assert_eq!(e.target_name.as_deref(), Some("Gioia"));
    assert_eq!(e.faces, 3);
    assert_eq!(e.sources.len(), 1);
    assert_eq!(e.sources[0].faces, 3, "Gruppo di 3 volti → Gioia");
    assert_eq!(e.sources[0].cover, Some(small_faces[1]), "con la copertina");
    assert!(e.undoable);
    // Filtro per persona: la voce riguarda Gioia e il gruppo d'origine.
    assert_eq!(merge_log::list(&c, Some(gioia), 50).unwrap().len(), 1);
    assert_eq!(merge_log::list(&c, Some(small), 50).unwrap().len(), 1);
    let other = group(&c, 1, None);
    assert!(merge_log::list(&c, Some(other), 50).unwrap().is_empty());

    // "Annulla" dallo storico: è l'ultima operazione, si annulla con lo stato completo.
    assert_eq!(merge_log::undo(&c, e.id).unwrap(), 3);
    assert!(exists(&c, small), "il gruppo torna con lo stesso id");
    assert_eq!(faces_of(&c, small), small_faces);
    assert_eq!(
        people::cover_face_id(&c, small).unwrap(),
        Some(small_faces[1])
    );
    let after = merge_log::list(&c, None, 50).unwrap();
    assert!(after[0].undone_at.is_some());
    assert!(!after[0].undoable);
    assert!(
        bulk::undo_last_people(&c).unwrap().is_none(),
        "Ctrl+Z non riapplica un'unione già annullata"
    );
    // Un manuale annullato non aggiunge vincoli.
    assert!(!people::blocked(&c, small, gioia).unwrap());
}

#[test]
fn ctrl_z_annulla_l_unione_e_la_segna_nello_storico() {
    let c = db();
    let a = group(&c, 2, Some("Anna"));
    let b = group(&c, 2, None);
    people::merge_logged(&c, &[b], a, MergeOrigin::Manual, "Unisci persone", false).unwrap();
    assert_eq!(
        bulk::undo_last_people(&c).unwrap().as_deref(),
        Some("Unisci persone")
    );
    assert!(exists(&c, b));
    let log = merge_log::list(&c, None, 10).unwrap();
    assert!(log[0].undone_at.is_some(), "storico coerente con Ctrl+Z");
}

#[test]
fn annullare_dallo_storico_un_unione_non_recente_riporta_solo_i_volti_ancora_li() {
    let c = db();
    let gioia = group(&c, 4, Some("Gioia"));
    let g1 = group(&c, 2, None);
    let g2 = group(&c, 2, None);
    let g1_faces = faces_of(&c, g1);
    let first =
        people::merge_logged(&c, &[g1], gioia, MergeOrigin::Manual, "Unisci", false).unwrap();
    people::merge_logged(&c, &[g2], gioia, MergeOrigin::Manual, "Unisci", false).unwrap();
    // Uno dei volti di g1 viene poi spostato altrove: resta dov'è.
    let elsewhere = group(&c, 1, None);
    people::assign(&c, &[g1_faces[0]], elsewhere).unwrap();
    assert_eq!(merge_log::undo(&c, first).unwrap(), 1);
    assert_eq!(faces_of(&c, g1), vec![g1_faces[1]]);
    // L'operazione della prima unione esce dal registro di Ctrl+Z; resta quella di g2.
    assert_eq!(
        bulk::undo_last_people(&c).unwrap().as_deref(),
        Some("Unisci")
    );
    assert!(exists(&c, g2));
    assert!(bulk::undo_last_people(&c).unwrap().is_none());
}

#[test]
fn unione_automatica_annullata_diventa_un_vincolo() {
    // Scenario "Unione automatica annullata": il gruppo torna com'era e le analisi
    // successive non lo uniscono di nuovo alla stessa persona.
    let c = db();
    let gioia = group(&c, 4, Some("Gioia"));
    // Gruppo del primo passaggio unito subito dal secondo: id riservato, mai creato.
    let src = people::reserve_person_id(&c).unwrap();
    let moved: Vec<i64> = faces_of(&c, group(&c, 2, None));
    let before = merge_log::faces_before(&c, &moved).unwrap();
    for f in &moved {
        c.execute(
            "UPDATE face SET person_id=?2, assigned_by='auto' WHERE id=?1",
            rusqlite::params![f, gioia],
        )
        .unwrap();
    }
    let snap = merge_log::MergeSnapshot {
        sources: vec![merge_log::SourceGroup {
            person: src,
            name: None,
            identity_uid: None,
            ignored_at: None,
            ignored_faces: 0,
            cover_face_id: None,
            cover: Some(moved[0]),
            faces: before,
        }],
        target_name: Some("Gioia".into()),
        target_identity: None,
    };
    let id = merge_log::record(&c, MergeOrigin::Auto, gioia, &snap).unwrap();
    assert_eq!(merge_log::auto_counts(&c).unwrap(), (1, 0));
    assert_eq!(merge_log::undo(&c, id).unwrap(), 2);
    assert!(exists(&c, src), "ricreato con l'id dello storico");
    assert_eq!(faces_of(&c, src), moved);
    assert!(
        people::not_same_pairs(&c).unwrap().contains(&(src, gioia)),
        "vincolo Persone diverse: l'unione non si ripete"
    );
    assert_eq!(merge_log::auto_counts(&c).unwrap(), (1, 1));
    assert!(merge_log::undo(&c, id).is_err(), "già annullata");
}

#[test]
fn assegnazione_in_blocco_registra_una_sola_voce() {
    let c = db();
    let g1 = group(&c, 1, None);
    let g2 = group(&c, 2, None);
    let out = people::assign_groups(
        &c,
        &[g1, g2],
        &people::AssignTarget::NewPerson {
            name: "Marco".into(),
        },
    )
    .unwrap();
    let to = out.person.unwrap();
    let log = merge_log::list(&c, None, 10).unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].target, to);
    assert_eq!(log[0].sources.len(), 2);
    assert_eq!(log[0].faces, 3);
    bulk::undo_last_people(&c).unwrap();
    assert!(merge_log::list(&c, None, 10).unwrap()[0]
        .undone_at
        .is_some());
}

fn q(kind: QuestionKind, a: i64, b: i64, third: Option<i64>, faces: i64) -> NewQuestion {
    NewQuestion {
        kind,
        a,
        b,
        c: third,
        score: 0.5,
        probability: 0.7,
        faces,
    }
}

#[test]
fn domande_risolte_per_transitivita() {
    // Scenario "Domande risolte per transitività".
    let c = db();
    let gioia = group(&c, 5, Some("Gioia"));
    let a = group(&c, 2, None);
    let b = group(&c, 1, None);
    review::replace(
        &c,
        &[
            q(QuestionKind::GroupPerson, a, gioia, None, 2),
            q(QuestionKind::GroupGroup, a, b, None, 1),
            q(QuestionKind::GroupPerson, b, gioia, None, 1),
        ],
    )
    .unwrap();
    let ids: Vec<i64> = {
        let mut st = c
            .prepare("SELECT id FROM review_question ORDER BY rank")
            .unwrap();
        st.query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    };
    let (a_is_gioia, a_is_b) = (ids[0], ids[1]);
    let (fa, fb) = (faces_of(&c, a), faces_of(&c, b));
    let out = review::answer(&c, a_is_b, &Answer::Yes, false).unwrap();
    assert_eq!(out.merged_groups, 1);
    // "B è Gioia?" è ora un doppione di "A è Gioia?": resta una sola domanda.
    let open: i64 = c
        .query_row("SELECT count(*) FROM review_question", [], |r| r.get(0))
        .unwrap();
    assert_eq!(open, 1);
    review::answer(&c, a_is_gioia, &Answer::Yes, false).unwrap();
    let mut all = faces_of(&c, gioia);
    for f in fa.iter().chain(&fb) {
        assert!(all.contains(f), "A e B passano entrambi a Gioia");
    }
    all.sort();
    let open: i64 = c
        .query_row("SELECT count(*) FROM review_question", [], |r| r.get(0))
        .unwrap();
    assert_eq!(open, 0, "\"B è Gioia?\" non viene più proposta");
    // I volti passati da una risposta sono confermati.
    let manual: i64 = c
        .query_row(
            "SELECT count(*) FROM face WHERE person_id=?1 AND assigned_by='manual'",
            [gioia],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(manual, 3);
    let log = merge_log::list(&c, None, 10).unwrap();
    assert!(log.iter().all(|e| e.origin == MergeOrigin::Queue));
}

#[test]
fn conflitto_con_un_vincolo_precedente() {
    // Scenario "Conflitto".
    let c = db();
    let gioia = group(&c, 5, Some("Gioia"));
    let cg = group(&c, 2, None);
    let d = group(&c, 2, None);
    // L'utente aveva indicato che C non è Gioia.
    let cf = faces_of(&c, cg);
    people::reject_faces(&c, &cf[..1], gioia).unwrap();
    c.execute(
        "UPDATE face SET person_id=?1 WHERE id=?2",
        rusqlite::params![cg, cf[0]],
    )
    .unwrap();
    review::replace(&c, &[q(QuestionKind::GroupGroup, cg, d, None, 2)]).unwrap();
    let qid: i64 = c
        .query_row("SELECT id FROM review_question", [], |r| r.get(0))
        .unwrap();
    // D era già stato unito a Gioia: la domanda segue l'unione.
    people::merge_logged(&c, &[d], gioia, MergeOrigin::Manual, "Unisci", false).unwrap();
    let (b, kind): (i64, String) = c
        .query_row(
            "SELECT b, kind FROM review_question WHERE id=?1",
            [qid],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap_or((0, String::new()));
    // Il vincolo esistente toglie la domanda dalla coda normalizzata…
    assert!(b == 0 || (b == gioia && kind == "group_person"));
    // …ma una risposta "Sì" in conflitto viene mostrata senza unire nulla.
    review::replace(&c, &[]).unwrap();
    c.execute(
        "INSERT INTO review_question(kind,a,b,score,probability,faces,rank,created_at) VALUES ('group_group',?1,?2,0.5,0.7,2,0,1)",
        rusqlite::params![cg, gioia],
    )
    .unwrap();
    let qid = c.last_insert_rowid();
    let before = faces_of(&c, gioia);
    let out = review::answer(&c, qid, &Answer::Yes, false).unwrap();
    let conflict = out.conflict.expect("conflitto");
    assert_eq!((conflict.group, conflict.person), (cg, gioia));
    assert_eq!(faces_of(&c, gioia), before, "niente unito");
    // L'utente sceglie di annullare il "Non è Gioia" precedente: ora si unisce.
    let out = review::answer(&c, qid, &Answer::Yes, true).unwrap();
    assert!(out.conflict.is_none());
    assert_eq!(out.merged_groups, 1);
    assert!(!exists(&c, cg));
    assert!(!people::blocked(&c, gioia, gioia).unwrap());
    // Ctrl+Z rimette anche il vincolo tolto.
    bulk::undo_last_people(&c).unwrap();
    assert!(exists(&c, cg));
    assert!(people::blocked(&c, cg, gioia).unwrap());
}

#[test]
fn conferma_in_blocco_con_gruppi_tolti() {
    // Scenario "Conferma in blocco".
    let c = db();
    let gioia = group(&c, 5, Some("Gioia"));
    let groups: Vec<i64> = (0..12).map(|i| group(&c, 1 + i % 2, None)).collect();
    review::replace(
        &c,
        &groups
            .iter()
            .map(|g| q(QuestionKind::GroupPerson, *g, gioia, None, 1))
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let m = people::default_face_match();
    let queue = review::queue(&c, &m, 50).unwrap();
    assert_eq!(queue.items.len(), 1, "un solo elemento: 12 gruppi di Gioia");
    let ReviewItem::Batch { person, questions } = &queue.items[0] else {
        panic!("atteso un blocco");
    };
    assert_eq!(person.id, gioia);
    assert_eq!(questions.len(), 12);
    let (removed, kept): (Vec<_>, Vec<_>) = questions.iter().enumerate().partition(|(i, _)| *i < 2);
    let confirm: Vec<i64> = kept.iter().map(|(_, q)| q.id).collect();
    let reject: Vec<i64> = removed.iter().map(|(_, q)| q.id).collect();
    let before = faces_of(&c, gioia).len();
    let out = review::answer_batch(&c, gioia, &confirm, &reject, false).unwrap();
    assert_eq!(out.merged_groups, 10);
    assert_eq!(out.constrained_groups, 2);
    let moved: usize = groups[2..]
        .iter()
        .map(|g| if exists(&c, *g) { 0 } else { 1 })
        .sum();
    assert_eq!(moved, 10, "i 10 gruppi rimasti passano a Gioia");
    assert!(faces_of(&c, gioia).len() > before);
    for g in &groups[..2] {
        assert!(exists(&c, *g));
        assert!(
            people::blocked(&c, *g, gioia).unwrap(),
            "i 2 gruppi tolti: Non è questa persona"
        );
        let rejected: String = c
            .query_row(
                "SELECT rejected FROM face WHERE person_id=?1 LIMIT 1",
                [g],
                |r| r.get(0),
            )
            .unwrap();
        assert!(rejected.contains(&gioia.to_string()));
    }
    let log = merge_log::list(&c, None, 10).unwrap();
    assert_eq!(log.len(), 1, "un'unica unione nello storico");
    assert_eq!(log[0].sources.len(), 10);
    assert_eq!(review::queue(&c, &m, 50).unwrap().open, 0);
    // Un'unica operazione annullabile.
    assert_eq!(
        bulk::undo_last_people(&c).unwrap().as_deref(),
        Some("Conferma in blocco")
    );
    assert!(groups.iter().all(|g| exists(&c, *g)));
    assert_eq!(faces_of(&c, gioia).len(), before);
    assert!(!people::blocked(&c, groups[0], gioia).unwrap());
}

#[test]
fn coda_vuota_indica_i_gruppi_senza_nome() {
    // Scenario "Coda vuota".
    let c = db();
    group(&c, 2, Some("Gioia"));
    group(&c, 1, None);
    group(&c, 3, None);
    let queue = review::queue(&c, &people::default_face_match(), 50).unwrap();
    assert!(queue.items.is_empty());
    assert_eq!(queue.open, 0);
    assert_eq!(queue.unnamed_groups, 3, "gruppi senza nome in Da rivedere");
}

#[test]
fn scelta_tra_due_persone_e_nessuna_delle_due() {
    let c = db();
    let anna = group(&c, 3, Some("Anna"));
    let marta = group(&c, 3, Some("Marta"));
    let x = group(&c, 1, None);
    let y = group(&c, 1, None);
    review::replace(
        &c,
        &[
            q(QuestionKind::GroupChoice, x, anna, Some(marta), 1),
            q(QuestionKind::GroupChoice, y, anna, Some(marta), 1),
        ],
    )
    .unwrap();
    let ids: Vec<i64> = {
        let mut st = c
            .prepare("SELECT id FROM review_question ORDER BY rank")
            .unwrap();
        st.query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    };
    assert!(review::answer(&c, ids[0], &Answer::Yes, false).is_err());
    review::answer(&c, ids[0], &Answer::Choose { person: marta }, false).unwrap();
    assert!(!exists(&c, x));
    assert_eq!(faces_of(&c, marta).len(), 4);
    review::answer(&c, ids[1], &Answer::Neither, false).unwrap();
    assert!(people::blocked(&c, y, anna).unwrap());
    assert!(people::blocked(&c, y, marta).unwrap());
}

#[test]
fn misure_del_raggruppamento() {
    let c = db();
    let gioia = group(&c, 5, Some("Gioia"));
    let marco = group(&c, 2, Some("Marco"));
    let a = group(&c, 1, None);
    let b = group(&c, 1, None);
    let _c3 = group(&c, 3, None);
    review::replace(
        &c,
        &[
            q(QuestionKind::GroupPerson, a, gioia, None, 1),
            q(QuestionKind::GroupPerson, b, gioia, None, 1),
        ],
    )
    .unwrap();
    review::save_thresholds(
        &c,
        &review::ThresholdsInUse {
            ask: 0.363,
            auto: 0.55,
            margin: 0.05,
            calibrated: false,
            same_photo_pairs: 12,
            confirmed_pairs: 0,
            too_strict: false,
            sensitivity: 0.5,
        },
    )
    .unwrap();
    let m = review::save_metrics(&c, 4).unwrap();
    assert_eq!(m.named_people, 2);
    // Gioia: 1 + 2 gruppi con domanda aperta; Marco: 1.
    assert!((m.groups_per_named - 2.0).abs() < 1e-6);
    assert_eq!(m.unnamed_groups, 3);
    assert!((m.single_face_share - 2.0 / 3.0).abs() < 1e-6);
    assert_eq!(m.questions, 2);
    assert_eq!(m.last_auto_merges, 4);
    let t = m.thresholds.unwrap();
    assert!(!t.calibrated);
    assert_eq!(
        review::saved_metrics(&c).unwrap().unwrap().last_auto_merges,
        4
    );
    let _ = marco;
}

#[test]
fn riepilogo_della_prima_riaggregazione_una_sola_volta() {
    let c = db();
    assert!(review::first_summary(&c).unwrap().is_none());
    review::save_first_summary(&c, 52).unwrap();
    assert_eq!(review::first_summary(&c).unwrap().unwrap().auto_merges, 52);
    review::dismiss_first_summary(&c).unwrap();
    assert!(review::first_summary(&c).unwrap().is_none());
}
