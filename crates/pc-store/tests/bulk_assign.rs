//! Assegnazione in blocco dei gruppi, filtro per numero di foto e riallineamento delle
//! persone dopo l'unione di più identità (change `bulk-merge-people`, issue #51).
use pc_store::bulk;
use pc_store::people::{self, AssignTarget, FaceState, GroupSort, PeopleSection};
use pc_store::{migrate, AppStore};
use rusqlite::{params, Connection};
use std::collections::BTreeMap;
use std::time::Instant;

fn empty_library() -> Connection {
    let mut c = Connection::open_in_memory().unwrap();
    migrate::migrate(&mut c, None).unwrap();
    c
}

/// Aggiunge una foto con un volto (buono) assegnato a `person`; restituisce l'id del volto.
fn add_face(c: &Connection, media: i64, person: Option<i64>, axis: usize) -> i64 {
    c.execute(
        "INSERT INTO media_item(id,path,name,ext,size,mtime,added_at,updated_at) \
         VALUES (?1,?2,'t','jpg',1,1,1,1)",
        params![media, format!("t{media}.jpg")],
    )
    .unwrap();
    let mut v = vec![0.0f32; 128];
    v[axis % 128] = 1.0;
    let blob: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
    c.execute(
        "INSERT INTO face(media_id,bbox,quality,embedding,person_id,assigned_by,model_ver, \
                          sharpness,size_px,quality_score) \
         VALUES (?1,'[0.1,0.1,0.2,0.2]',0.99,?2,?3,'auto','sface',0.9,120,0.99)",
        params![media, blob, person],
    )
    .unwrap();
    c.last_insert_rowid()
}

/// Libreria con un gruppo (persona) per ogni elemento di `layout`, con quel numero di foto
/// (un volto per foto). Restituisce gli id dei gruppi e dei volti per gruppo.
fn library(layout: &[usize]) -> (Connection, Vec<i64>, Vec<Vec<i64>>) {
    let c = empty_library();
    let mut media = 0;
    let mut groups = Vec::new();
    let mut faces = Vec::new();
    for (g, n) in layout.iter().enumerate() {
        let person = people::create_person(&c, None).unwrap();
        let mut ids = Vec::new();
        for _ in 0..*n {
            media += 1;
            ids.push(add_face(&c, media, Some(person), g));
        }
        groups.push(person);
        faces.push(ids);
    }
    people::refresh_centroids(&c).unwrap();
    (c, groups, faces)
}

fn count(c: &Connection, sql: &str) -> i64 {
    c.query_row(sql, [], |r| r.get(0)).unwrap()
}

/// Stato completo da confrontare prima e dopo un annullamento.
fn dump(c: &Connection) -> (Vec<FaceState>, Vec<String>, Vec<String>) {
    let ids: Vec<i64> = c
        .prepare("SELECT id FROM face ORDER BY id")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let rows = |sql: &str| -> Vec<String> {
        let mut q = c.prepare(sql).unwrap();
        let n = q.column_count();
        q.query_map([], |r| {
            Ok((0..n)
                .map(|i| format!("{:?}", r.get::<_, rusqlite::types::Value>(i).unwrap()))
                .collect::<Vec<_>>()
                .join("|"))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
    };
    (
        people::face_states(c, &ids).unwrap(),
        rows("SELECT id,name,identity_uid,ignored_at,ignored_faces,cover_face_id FROM person ORDER BY id"),
        rows("SELECT a,b,created_at FROM person_not_same ORDER BY a,b"),
    )
}

fn centroids(c: &Connection) -> BTreeMap<i64, Vec<u8>> {
    let mut q = c
        .prepare("SELECT person_id,vector FROM person_centroid ORDER BY person_id")
        .unwrap();
    q.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn person_of(c: &Connection, face: i64) -> Option<i64> {
    c.query_row("SELECT person_id FROM face WHERE id=?1", [face], |r| {
        r.get(0)
    })
    .unwrap()
}

fn exists(c: &Connection, person: i64) -> bool {
    c.query_row(
        "SELECT EXISTS(SELECT 1 FROM person WHERE id=?1)",
        [person],
        |r| r.get(0),
    )
    .unwrap()
}

#[test]
fn assign_420_groups_to_a_person_with_a_single_undo() {
    // "Nonno Piero" ha già 3 volti; 420 gruppi da una foto.
    let mut layout = vec![3];
    layout.extend(std::iter::repeat_n(1, 420));
    let (c, groups, faces) = library(&layout);
    let piero = groups[0];
    people::rename(&c, piero, "Nonno Piero").unwrap();
    let before = dump(&c);
    let before_centroids = centroids(&c);
    let small = &groups[1..];

    let out = people::assign_groups(&c, small, &AssignTarget::Person { id: piero }).unwrap();

    assert_eq!(out.person, Some(piero));
    assert!(!out.created);
    assert_eq!((out.groups, out.faces, out.skipped_faces), (420, 420, 0));
    assert_eq!(count(&c, "SELECT count(*) FROM person"), 1);
    for ids in &faces {
        for f in ids {
            assert_eq!(person_of(&c, *f), Some(piero));
        }
    }
    // Un solo annullamento, riconosciuto come correzione delle persone.
    assert_eq!(count(&c, "SELECT count(*) FROM undo_log"), 1);
    assert_eq!(
        bulk::undo_last_people(&c).unwrap().as_deref(),
        Some("Assegna gruppi")
    );
    assert_eq!(count(&c, "SELECT count(*) FROM undo_log"), 0);
    assert_eq!(
        dump(&c),
        before,
        "gruppi, volti, rifiuti e vincoli come prima"
    );
    assert_eq!(centroids(&c), before_centroids);
    assert_eq!(count(&c, "SELECT count(*) FROM person"), 421);
    // I gruppi sono di nuovo in "Da rivedere" e Piero ha di nuovo solo i suoi 3 volti.
    let m = people::default_face_match();
    let page = people::list_groups(
        &c,
        PeopleSection::Review,
        None,
        GroupSort::PhotosDesc,
        None,
        5,
        &m,
    )
    .unwrap();
    assert_eq!(page.total, 421);
    assert_eq!(
        count(
            &c,
            &format!("SELECT count(*) FROM face WHERE person_id={piero}")
        ),
        3
    );
}

#[test]
fn assign_to_an_unlinked_identity_creates_one_linked_person_and_reuses_it() {
    let (c, groups, faces) = library(&[1; 12]);
    let target = AssignTarget::Identity {
        uid: "uid-anna".into(),
        name: "Zia Anna".into(),
    };
    let before = dump(&c);

    let out = people::assign_groups(&c, &groups[..12], &target).unwrap();

    assert!(out.created);
    assert_eq!((out.groups, out.faces), (12, 12));
    let person = out.person.unwrap();
    assert_eq!(count(&c, "SELECT count(*) FROM person"), 1);
    let (name, uid): (Option<String>, Option<String>) = c
        .query_row(
            "SELECT name,identity_uid FROM person WHERE id=?1",
            [person],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(name.as_deref(), Some("Zia Anna"));
    assert_eq!(uid.as_deref(), Some("uid-anna"));
    assert_eq!(
        people::person_for_identity(&c, "uid-anna").unwrap(),
        Some(person)
    );
    assert!(faces
        .iter()
        .flatten()
        .all(|f| person_of(&c, *f) == Some(person)));
    // Compare in "Persone", non più in "Da rivedere".
    let m = people::default_face_match();
    let p = people::list_groups(
        &c,
        PeopleSection::People,
        None,
        GroupSort::PhotosDesc,
        None,
        5,
        &m,
    )
    .unwrap();
    assert_eq!((p.total, p.items[0].photos), (1, 12));

    // Annulla: i 12 gruppi tornano e la persona creata sparisce.
    bulk::undo_last_people(&c).unwrap();
    assert_eq!(dump(&c), before);
    assert_eq!(people::person_for_identity(&c, "uid-anna").unwrap(), None);

    // Con la persona già collegata viene riusata, non ne nasce un'altra.
    people::assign_groups(&c, &groups[..5], &target).unwrap();
    let again = people::assign_groups(&c, &groups[5..], &target).unwrap();
    assert!(!again.created);
    assert_eq!(count(&c, "SELECT count(*) FROM person"), 1);
}

#[test]
fn assign_to_a_new_named_person() {
    let (c, groups, faces) = library(&[1, 2, 1, 1, 3]);
    let before = dump(&c);
    let out = people::assign_groups(
        &c,
        &groups,
        &AssignTarget::NewPerson {
            name: "  Vicino di casa ".into(),
        },
    )
    .unwrap();
    let person = out.person.unwrap();
    assert!(out.created);
    assert_eq!((out.groups, out.faces), (5, 8));
    assert_eq!(count(&c, "SELECT count(*) FROM person"), 1);
    let name: String = c
        .query_row("SELECT name FROM person WHERE id=?1", [person], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(name, "Vicino di casa");
    assert!(faces
        .iter()
        .flatten()
        .all(|f| person_of(&c, *f) == Some(person)));
    bulk::undo_last_people(&c).unwrap();
    assert_eq!(dump(&c), before);
    assert!(!exists(&c, person));

    // Nome vuoto: errore e nessuna modifica.
    let err = people::assign_groups(&c, &groups, &AssignTarget::NewPerson { name: "  ".into() });
    assert!(err.is_err());
    assert_eq!(dump(&c), before);
    assert_eq!(count(&c, "SELECT count(*) FROM undo_log"), 0);
}

#[test]
fn refused_faces_are_not_assigned_and_stay_in_review() {
    // 0 = destinazione; 1 = gruppo misto (3 volti, 1 rifiutato); 2 = gruppo tutto rifiutato;
    // 3 = gruppo normale.
    let (c, groups, faces) = library(&[2, 3, 2, 1]);
    let (target, mixed, refused, plain) = (groups[0], groups[1], groups[2], groups[3]);
    for f in [faces[1][0], faces[2][0], faces[2][1]] {
        c.execute(
            "UPDATE face SET rejected=?2 WHERE id=?1",
            params![f, format!("[{target}]")],
        )
        .unwrap();
    }
    // La copertina del gruppo misto è proprio un volto che verrà spostato.
    people::set_cover(&c, mixed, faces[1][1]).unwrap();
    let before = dump(&c);

    let out = people::assign_groups(
        &c,
        &[mixed, refused, plain],
        &AssignTarget::Person { id: target },
    )
    .unwrap();

    assert_eq!(out.person, Some(target));
    assert_eq!((out.groups, out.faces, out.skipped_faces), (1, 3, 3));
    // Rifiutati: restano dove erano, nei gruppi da rivedere.
    assert_eq!(person_of(&c, faces[1][0]), Some(mixed));
    assert_eq!(person_of(&c, faces[2][0]), Some(refused));
    assert_eq!(person_of(&c, faces[2][1]), Some(refused));
    // Gli altri sono nella destinazione.
    assert_eq!(person_of(&c, faces[1][1]), Some(target));
    assert_eq!(person_of(&c, faces[1][2]), Some(target));
    assert_eq!(person_of(&c, faces[3][0]), Some(target));
    assert!(exists(&c, mixed) && exists(&c, refused) && !exists(&c, plain));
    // Il rifiuto per la destinazione è ancora registrato.
    assert!(people::records(&c, None)
        .unwrap()
        .iter()
        .find(|f| f.id == faces[1][0])
        .unwrap()
        .rejected
        .contains(&target));
    // La copertina non punta più a un volto spostato.
    assert_eq!(people::cover_face_id(&c, mixed).unwrap(), None);
    let m = people::default_face_match();
    let review = people::list_groups(
        &c,
        PeopleSection::Review,
        None,
        GroupSort::PhotosDesc,
        None,
        10,
        &m,
    )
    .unwrap();
    let ids: Vec<i64> = review.items.iter().map(|g| g.id).collect();
    assert!(ids.contains(&mixed) && ids.contains(&refused));
    assert_eq!(
        review.items.iter().find(|g| g.id == mixed).unwrap().faces,
        1
    );

    bulk::undo_last_people(&c).unwrap();
    assert_eq!(dump(&c), before);
    assert_eq!(people::cover_face_id(&c, mixed).unwrap(), Some(faces[1][1]));

    // Nessun volto assegnabile: nessuna modifica e nessuna voce di annullamento.
    let log = count(&c, "SELECT count(*) FROM undo_log");
    let out = people::assign_groups(&c, &[refused], &AssignTarget::Person { id: target }).unwrap();
    assert_eq!((out.groups, out.faces, out.skipped_faces), (0, 0, 2));
    assert_eq!(count(&c, "SELECT count(*) FROM undo_log"), log);
    assert_eq!(dump(&c), before);
}

#[test]
fn rejection_for_a_new_identity_person_does_not_apply() {
    // Una destinazione appena creata non può essere stata rifiutata da nessun volto.
    let (c, groups, _) = library(&[1, 1]);
    let out = people::assign_groups(
        &c,
        &groups,
        &AssignTarget::Identity {
            uid: "u".into(),
            name: "Nuova".into(),
        },
    )
    .unwrap();
    assert_eq!((out.groups, out.skipped_faces), (2, 0));
}

#[test]
fn destination_among_groups_duplicates_and_unknown_groups() {
    let (c, groups, faces) = library(&[2, 1, 1]);
    let before = dump(&c);
    // La destinazione tra i gruppi e un gruppo doppio si ignorano.
    let out = people::assign_groups(
        &c,
        &[groups[0], groups[1], groups[1], groups[2]],
        &AssignTarget::Person { id: groups[0] },
    )
    .unwrap();
    assert_eq!((out.groups, out.faces), (2, 2));
    assert!(faces
        .iter()
        .flatten()
        .all(|f| person_of(&c, *f) == Some(groups[0])));
    bulk::undo_last_people(&c).unwrap();
    assert_eq!(dump(&c), before);
    // Gruppo o destinazione inesistenti: errore e nessuna modifica.
    assert!(people::assign_groups(
        &c,
        &[groups[1], 99_999],
        &AssignTarget::Person { id: groups[0] }
    )
    .is_err());
    assert!(people::assign_groups(&c, &[groups[1]], &AssignTarget::Person { id: 99_999 }).is_err());
    assert!(people::assign_groups(&c, &[], &AssignTarget::Person { id: groups[0] }).is_err());
    assert_eq!(dump(&c), before);
    assert_eq!(count(&c, "SELECT count(*) FROM undo_log"), 0);
}

#[test]
fn assigning_keeps_not_same_constraints_and_restores_them() {
    let (c, groups, _) = library(&[1, 1, 1, 1]);
    // 1 e 2 sono "persone diverse"; 2 e 3 pure; 1 e la destinazione 0 pure.
    people::mark_not_same(&c, groups[1], groups[2]).unwrap();
    people::mark_not_same(&c, groups[2], groups[3]).unwrap();
    people::mark_not_same(&c, groups[1], groups[0]).unwrap();
    let before = dump(&c);
    people::assign_groups(&c, &groups[1..3], &AssignTarget::Person { id: groups[0] }).unwrap();
    // Il vincolo con il gruppo 3 passa alla destinazione; quelli tra gruppi uniti spariscono.
    let pairs = people::not_same_pairs(&c).unwrap();
    let (lo, hi) = (groups[0].min(groups[3]), groups[0].max(groups[3]));
    assert!(pairs.contains(&(lo, hi)));
    assert_eq!(count(&c, "SELECT count(*) FROM person_not_same"), 1);
    bulk::undo_last_people(&c).unwrap();
    assert_eq!(dump(&c), before);
}

#[test]
fn photo_count_filter_on_groups_and_selection_ids() {
    // 5 gruppi da 1 foto, 4 da 2, 3 da 3, 1 da 7.
    let mut layout = vec![1; 5];
    layout.extend([2; 4]);
    layout.extend([3; 3]);
    layout.push(7);
    let (c, groups, _) = library(&layout);
    let m = people::default_face_match();
    let list = |max: Option<u32>, query: Option<&str>| {
        people::list_groups_filtered(
            &c,
            PeopleSection::Review,
            query,
            max,
            GroupSort::PhotosDesc,
            None,
            200,
            &m,
        )
        .unwrap()
    };
    let one = list(Some(1), None);
    assert_eq!(one.total, 5);
    assert!(one.items.iter().all(|g| g.photos == 1));
    let two = list(Some(2), None);
    assert_eq!(two.total, 9);
    assert!(two.items.iter().all(|g| g.photos <= 2));
    assert_eq!(list(Some(3), None).total, 12);
    assert_eq!(list(None, None).total, 13);
    assert_eq!(list(Some(0), None).total, 0);
    // Senza filtro l'elenco è quello di sempre.
    let plain = people::list_groups(
        &c,
        PeopleSection::Review,
        None,
        GroupSort::PhotosDesc,
        None,
        200,
        &m,
    )
    .unwrap();
    assert_eq!(plain.items, list(None, None).items);

    // Il cursore funziona dentro l'elenco filtrato.
    let first = people::list_groups_filtered(
        &c,
        PeopleSection::Review,
        None,
        Some(2),
        GroupSort::PhotosDesc,
        None,
        4,
        &m,
    )
    .unwrap();
    let rest = people::list_groups_filtered(
        &c,
        PeopleSection::Review,
        None,
        Some(2),
        GroupSort::PhotosDesc,
        first.next,
        200,
        &m,
    )
    .unwrap();
    assert_eq!((first.items.len(), rest.items.len(), rest.total), (4, 5, 9));

    // "Seleziona tutto": tutti gli id dell'elenco filtrato, non solo la prima pagina.
    let mut all =
        people::group_selection_ids(&c, PeopleSection::Review, None, Some(2), &m).unwrap();
    let mut listed: Vec<i64> = first
        .items
        .iter()
        .chain(&rest.items)
        .map(|g| g.id)
        .collect();
    all.sort();
    listed.sort();
    assert_eq!(all, listed);
    assert_eq!(
        people::group_selection_ids(&c, PeopleSection::Review, None, Some(1), &m).unwrap(),
        {
            let mut v: Vec<i64> = groups[..5].to_vec();
            v.reverse(); // foto uguali: id decrescente
            v
        }
    );
    assert_eq!(
        people::group_selection_ids(&c, PeopleSection::Review, None, None, &m)
            .unwrap()
            .len(),
        13
    );
    // Nessun gruppo con identità: "Persone" è vuota.
    assert!(
        people::group_selection_ids(&c, PeopleSection::People, None, Some(2), &m)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn photo_count_filter_counts_photos_not_faces() {
    // Un gruppo con due volti nella stessa foto ha 1 foto.
    let c = empty_library();
    let p = people::create_person(&c, None).unwrap();
    add_face(&c, 1, Some(p), 0);
    c.execute(
        "INSERT INTO face(media_id,bbox,quality,embedding,person_id,assigned_by,model_ver, \
                          sharpness,size_px,quality_score) \
         SELECT media_id,'[0.5,0.5,0.2,0.2]',0.99,embedding,person_id,'auto','sface',0.9,120,0.99 \
         FROM face WHERE person_id=?1",
        [p],
    )
    .unwrap();
    let m = people::default_face_match();
    let page = people::list_groups_filtered(
        &c,
        PeopleSection::Review,
        None,
        Some(1),
        GroupSort::PhotosDesc,
        None,
        10,
        &m,
    )
    .unwrap();
    assert_eq!(
        (page.total, page.items[0].faces, page.items[0].photos),
        (1, 2, 1)
    );
}

#[test]
fn relink_identities_follows_the_destination_person() {
    let c = empty_library();
    let mut media = 0;
    let mut linked = |uid: &str, n: usize, axis: usize| -> i64 {
        let p = people::create_person(&c, Some(uid)).unwrap();
        c.execute(
            "UPDATE person SET identity_uid=?2 WHERE id=?1",
            params![p, uid],
        )
        .unwrap();
        for _ in 0..n {
            media += 1;
            add_face(&c, media, Some(p), axis);
        }
        p
    };
    let (pa, pb, pc, pd) = (
        linked("a", 2, 0),
        linked("b", 1, 1),
        linked("c", 3, 2),
        linked("dest", 4, 3),
    );
    let (faces_before, _) = (count(&c, "SELECT count(*) FROM face"), ());
    // Destinazione già collegata: le altre vi si uniscono ("e" non ha persona).
    let out = people::relink_identities(
        &c,
        &[
            "a".into(),
            "b".into(),
            "e".into(),
            "dest".into(),
            "a".into(),
        ],
        "dest",
        "Dest",
    )
    .unwrap();
    assert_eq!(out, Some(pd));
    assert!(!exists(&c, pa) && !exists(&c, pb) && exists(&c, pc) && exists(&c, pd));
    assert_eq!(
        count(
            &c,
            &format!("SELECT count(*) FROM face WHERE person_id={pd}")
        ),
        7
    );
    assert_eq!(count(&c, "SELECT count(*) FROM face"), faces_before);
    assert_eq!(people::person_for_identity(&c, "a").unwrap(), None);
    // Destinazione non collegata: la prima persona diventa la sua, le altre vi si uniscono.
    let out =
        people::relink_identities(&c, &["c".into(), "dest".into()], "nuova", "Nuova").unwrap();
    assert_eq!(out, Some(pc));
    let (uid, name): (String, String) = c
        .query_row(
            "SELECT identity_uid,name FROM person WHERE id=?1",
            [pc],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!((uid.as_str(), name.as_str()), ("nuova", "Nuova"));
    assert!(!exists(&c, pd), "dest era tra le sorgenti: passa a nuova");
    let out =
        people::relink_identities(&c, &["c".into(), "dest".into()], "nuova", "Nuova").unwrap();
    assert_eq!(out, Some(pc));
}

#[test]
fn relink_identities_with_unlinked_sources_changes_nothing() {
    let c = empty_library();
    let p = people::create_person(&c, Some("x")).unwrap();
    c.execute("UPDATE person SET identity_uid='dest' WHERE id=?1", [p])
        .unwrap();
    assert_eq!(
        people::relink_identities(&c, &["a".into(), "b".into()], "dest", "x").unwrap(),
        Some(p)
    );
    assert_eq!(
        people::relink_identities(&c, &["a".into()], "zzz", "z").unwrap(),
        None
    );
    assert_eq!(count(&c, "SELECT count(*) FROM person"), 1);
}

/// Misura (con `--nocapture`) l'assegnazione di 420 gruppi in una libreria grande.
#[test]
fn bulk_assignment_is_fast_on_a_large_library() {
    // 420 gruppi da una foto, 40 gruppi grandi da 100 foto (4000 volti) e la destinazione.
    let mut layout = vec![1; 420];
    layout.extend([100; 40]);
    layout.push(50);
    let (c, groups, _) = library(&layout);
    let target = *groups.last().unwrap();
    let small = &groups[..420];
    let m = people::default_face_match();

    let t = Instant::now();
    let ids = people::group_selection_ids(&c, PeopleSection::Review, None, Some(1), &m).unwrap();
    let t_ids = t.elapsed();
    assert_eq!(ids.len(), 420);

    let t = Instant::now();
    let out = people::assign_groups(&c, small, &AssignTarget::Person { id: target }).unwrap();
    let t_assign = t.elapsed();
    assert_eq!((out.groups, out.faces), (420, 420));

    let t = Instant::now();
    bulk::undo_last_people(&c).unwrap();
    let t_undo = t.elapsed();
    assert_eq!(count(&c, "SELECT count(*) FROM person"), 461);
    eprintln!(
        "PERF 420 gruppi su 4470 volti: selezione {t_ids:?}, assegnazione {t_assign:?}, annulla {t_undo:?}"
    );
    assert!(t_assign.as_secs() < 10 && t_undo.as_secs() < 10);
}

#[test]
fn identity_merge_then_relink_is_consistent_with_the_app_store() {
    // Il flusso di `merge_identities_many`: archivio identità + persone della libreria.
    use pc_store::app_store::identities as ids;
    let app = AppStore::open_in_memory().unwrap();
    let c = empty_library();
    let uids = app
        .with(|a| {
            ["Dest", "A", "B"]
                .iter()
                .map(|n| {
                    ids::create(
                        a,
                        &ids::IdentityInput {
                            name: (*n).into(),
                            ..Default::default()
                        },
                    )
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .unwrap();
    let pa = people::ensure_identity_person(&c, &uids[1], "A").unwrap();
    add_face(&c, 1, Some(pa), 0);
    let pb = people::ensure_identity_person(&c, &uids[2], "B").unwrap();
    add_face(&c, 2, Some(pb), 1);
    let merged = app
        .tx(|a| ids::merge_many(a, &uids[1..], &uids[0]))
        .unwrap();
    let person = people::relink_identities(&c, &merged, &uids[0], "Dest")
        .unwrap()
        .unwrap();
    assert_eq!(
        person, pa,
        "la destinazione non era collegata: la prima persona è sua"
    );
    assert!(!exists(&c, pb));
    assert_eq!(
        count(
            &c,
            &format!("SELECT count(*) FROM face WHERE person_id={person}")
        ),
        2
    );
    assert_eq!(people::photos_of_identities(&c, &uids).unwrap(), 2);
}
