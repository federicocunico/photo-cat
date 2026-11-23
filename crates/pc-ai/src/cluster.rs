//! Raggruppamento dei volti sul catalogo (design D1–D8): legge volti, vincoli e soglie,
//! esegue il raggruppamento in due passaggi ([`crate::aggregate`]) e scrive persone, unioni
//! automatiche nello storico, domande della coda "Da confermare" e misure.

use crate::aggregate::{self, Face, Mode, Outcome, People, QuestionKind};
use crate::calibrate::{self, Calibration};
use pc_core::{AppError, AppResult, ErrorCode};
use pc_governor::MemoryGovernor;
use pc_jobs::JobControl;
use pc_store::merge_log::{self, MergeOrigin};
use pc_store::review::{self, NewQuestion, ThresholdsInUse};
use pc_store::{people, Store};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

pub use crate::aggregate::Label;

/// Versione dell'algoritmo: cambiandola, la prima analisi rifà il raggruppamento completo.
pub const ALGORITHM: u32 = 2;
const RUN_KEY: &str = "faces.cluster_run";

/// Impostazioni che, se cambiano, richiedono un ricalcolo completo.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct RunKey {
    algorithm: u32,
    face_detection: f32,
    face_min_sharpness: f32,
    face_similarity: f32,
    face_merge_sensitivity: f32,
}

impl RunKey {
    fn of(t: &pc_core::AiThresholds) -> Self {
        Self {
            algorithm: ALGORITHM,
            face_detection: t.face_detection,
            face_min_sharpness: t.face_min_sharpness,
            face_similarity: t.face_similarity,
            face_merge_sensitivity: t.face_merge_sensitivity,
        }
    }
}

/// Volti e stato delle persone letti dal catalogo.
struct Loaded {
    faces: Vec<Face>,
    /// Volti confermati dall'utente (`assigned_by = 'manual'`).
    confirmed: Vec<bool>,
    /// Persona salvata di ogni volto (prima dell'abbinamento ai nomi in lettura).
    stored: Vec<Option<i64>>,
    people: People,
}

fn capture_times(c: &Connection) -> pc_store::StoreResult<HashMap<i64, i64>> {
    let mut q = c.prepare(
        "SELECT f.id, COALESCE(d.instant_utc, CAST(strftime('%s', d.captured_at) AS INTEGER) * 1000) \
         FROM face f JOIN media_dates d ON d.media_id = f.media_id WHERE f.embedding IS NOT NULL",
    )?;
    let rows = q
        .query_map([], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, Option<i64>>(1)?))
        })?
        .filter_map(|r| r.ok())
        .filter_map(|(id, t)| t.map(|t| (id, t)))
        .collect();
    Ok(rows)
}

/// Legge i volti utilizzabili e lo stato delle persone. I volti senza persona vengono prima
/// abbinati alle persone con nome come al salvataggio (centroide sopra `ask`, con margine
/// verso la seconda; identità dell'app con la loro soglia).
fn load(
    c: &Connection,
    matching: &people::FaceMatch,
    ask: f32,
    refs: &people::IdentityRefs,
) -> pc_store::StoreResult<Loaded> {
    let not_same = people::not_same_pairs(c)?;
    let times = capture_times(c)?;
    let records: Vec<people::FaceRecord> = people::records(c, None)?
        .into_iter()
        .filter(|f| f.embedding.is_some() && !f.is_discarded(matching))
        .collect();
    let mut protected = people::named_ids(c)?;
    for f in &records {
        if f.manual {
            protected.extend(f.person_id);
        }
        protected.extend(f.rejected.iter().copied());
    }
    let mut named = HashSet::new();
    let mut ignored = HashSet::new();
    {
        let mut q = c.prepare(
            "SELECT id, (name IS NOT NULL OR identity_uid IS NOT NULL), ignored_at IS NOT NULL \
             FROM person",
        )?;
        for row in q.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, bool>(1)?,
                r.get::<_, bool>(2)?,
            ))
        })? {
            let (id, n, i) = row?;
            if n {
                named.insert(id);
            }
            if i {
                ignored.insert(id);
            }
        }
    }
    let mut faces = Vec::with_capacity(records.len());
    let mut confirmed = Vec::with_capacity(records.len());
    let mut stored = Vec::with_capacity(records.len());
    for f in records {
        let embedding = f.embedding.expect("embedding");
        let mut person = f.person_id;
        if person.is_none() && !f.manual {
            // Lettura: un'identità non collegata non crea persone qui (lo fa il salvataggio).
            if let Some(p) =
                people::named_match_refs(c, &embedding, &f.model, &f.rejected, ask, refs, false)?
            {
                person = Some(p);
            }
        }
        confirmed.push(f.manual);
        stored.push(f.person_id);
        faces.push(Face {
            id: f.id,
            media: f.media_id,
            model: f.model,
            embedding,
            quality: f.quality_score,
            time: times.get(&f.id).copied(),
            person,
            rejected: f.rejected,
        });
    }
    Ok(Loaded {
        faces,
        confirmed,
        stored,
        people: People {
            protected,
            named,
            ignored,
            not_same,
        },
    })
}

fn memory_permit(
    store: &Store,
    governor: &MemoryGovernor,
) -> AppResult<Option<pc_governor::Permit>> {
    let count: i64 = store.read(|c| {
        Ok(c.query_row(
            "SELECT count(*) FROM face WHERE embedding IS NOT NULL",
            [],
            |r| r.get(0),
        )?)
    })?;
    if count == 0 {
        return Ok(None);
    }
    // Embedding, grafo kNN e indice (≈ 4 KiB per volto) più gruppi e coppie del secondo
    // passaggio (≈ 1 KiB per volto).
    let cost = (count as u64)
        .saturating_mul(5120)
        .div_ceil(1024 * 1024)
        .saturating_add(16);
    if cost > governor.capacity_mb() {
        return Err(AppError::new(
            ErrorCode::InsufficientMemory,
            "memoria insufficiente per il raggruppamento volti",
        ));
    }
    governor.try_acquire(cost).map(Some).ok_or_else(|| {
        AppError::new(
            ErrorCode::InsufficientMemory,
            "attendere memoria libera per raggruppare i volti",
        )
    })
}

/// Raggruppa i volti salvati: ricalcolo completo se sono cambiate soglie o algoritmo,
/// altrimenti incrementale (cambiano solo i gruppi toccati dai volti nuovi).
pub fn run(
    store: &Store,
    governor: &MemoryGovernor,
    control: &JobControl,
    policy: std::sync::Arc<crate::face_policy::FacePolicy>,
    refs: &people::IdentityRefs,
    thresholds: &pc_core::AiThresholds,
) -> AppResult<()> {
    run_with(store, governor, control, policy, refs, thresholds, None)
}

/// Come [`run`], con la modalità imposta (`None` = scelta automatica).
pub fn run_with(
    store: &Store,
    governor: &MemoryGovernor,
    control: &JobControl,
    policy: std::sync::Arc<crate::face_policy::FacePolicy>,
    refs: &people::IdentityRefs,
    thresholds: &pc_core::AiThresholds,
    mode: Option<Mode>,
) -> AppResult<()> {
    let Some(generation) = policy.token() else {
        return Ok(());
    };
    let matching = crate::analyze::face_match(thresholds);
    // I volti scartati per qualità restano salvati ma non sono usati per il
    // raggruppamento (né tengono una persona automatica).
    store.write(move |c| people::unassign_discarded(c, &matching, true).map(|_| ()))?;
    let Some(_permit) = memory_permit(store, governor)? else {
        return Ok(());
    };
    let key = RunKey::of(thresholds);
    let similarity = thresholds.face_similarity;
    let sensitivity = thresholds.face_merge_sensitivity;
    let (loaded, calibration, previous) = store.read(|c| {
        // Prima la taratura (sui volti con la persona attuale), poi l'abbinamento ai nomi.
        let base = load(c, &matching, similarity, &Default::default())?;
        let cal = calibrate::calibrate(&base.faces, &base.confirmed, similarity, sensitivity);
        let loaded = if refs.is_empty() && cal.params.ask == similarity {
            base
        } else {
            load(c, &matching, cal.params.ask, refs)?
        };
        let previous: Option<RunKey> = c
            .query_row(
                "SELECT value FROM settings_kv WHERE key=?1",
                [RUN_KEY],
                |r| r.get::<_, String>(0),
            )
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok());
        Ok((loaded, cal, previous))
    })?;
    let mode = mode.unwrap_or(if previous == Some(key) {
        Mode::Incremental
    } else {
        Mode::Full
    });
    let first_after_update = previous.is_none_or(|p| p.algorithm != ALGORITHM)
        && loaded.faces.iter().any(|f| f.person.is_some());
    let outcome = aggregate::aggregate(
        &loaded.faces,
        &loaded.people,
        calibration.params,
        mode,
        control,
    )?;
    if !control.checkpoint() {
        return Err(AppError::cancelled());
    }
    let ask_match = people::FaceMatch {
        similarity: calibration.params.ask,
        ..matching
    };
    store.write(move |c| {
        if !policy.is_current(generation) {
            return Ok(());
        }
        let auto = write_outcome(c, &loaded, &outcome)?;
        save_run(c, key, &calibration)?;
        if first_after_update {
            review::save_first_summary(c, auto)?;
        }
        people::refresh_centroids_with(c, &matching)?;
        people::refresh_merge_suggestions(c, &ask_match)?;
        let m = review::save_metrics(c, auto)?;
        tracing::info!(
            mode = ?mode,
            faces = loaded.faces.len(),
            auto_merges = auto,
            questions = m.questions,
            groups_per_named = m.groups_per_named,
            single_face_share = m.single_face_share,
            auto_merges_total = m.auto_merges,
            auto_merges_undone = m.auto_merges_undone,
            ask = calibration.params.ask,
            auto = calibration.params.auto,
            margin = calibration.params.margin,
            calibrated = calibration.calibrated,
            same_photo_pairs = calibration.same_photo_pairs,
            "raggruppamento dei volti: misure"
        );
        Ok(())
    })?;
    Ok(())
}

fn save_run(c: &Connection, key: RunKey, cal: &Calibration) -> pc_store::StoreResult<()> {
    c.execute(
        "INSERT INTO settings_kv(key,value) VALUES (?1,?2) \
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        rusqlite::params![RUN_KEY, serde_json::to_string(&key)?],
    )?;
    review::save_thresholds(
        c,
        &ThresholdsInUse {
            ask: cal.params.ask,
            auto: cal.params.auto,
            margin: cal.params.margin,
            calibrated: cal.calibrated,
            same_photo_pairs: cal.same_photo_pairs as i64,
            confirmed_pairs: cal.confirmed_pairs as i64,
            too_strict: cal.too_strict,
            sensitivity: cal.sensitivity,
        },
    )
}

/// Persona di ogni gruppo finale (design D5): le persone protette restano; un gruppo riprende
/// l'id della persona da cui proviene almeno metà dei suoi volti già assegnati (con nome e
/// copertina); gli altri ricevono una persona nuova.
fn persons_for_groups(
    c: &Connection,
    loaded: &Loaded,
    out: &Outcome,
) -> pc_store::StoreResult<BTreeMap<Label, i64>> {
    let mut members: BTreeMap<Label, Vec<usize>> = BTreeMap::new();
    for (i, l) in out.labels.iter().enumerate() {
        members.entry(*l).or_default().push(i);
    }
    let mut person_of: BTreeMap<Label, i64> = BTreeMap::new();
    let mut used: HashSet<i64> = HashSet::new();
    for label in members.keys() {
        if let Label::Person(p) = label {
            if loaded.people.protected.contains(p) {
                person_of.insert(*label, *p);
                used.insert(*p);
            }
        }
    }
    let mut candidates: Vec<(usize, Label, i64)> = Vec::new();
    for (label, m) in &members {
        if person_of.contains_key(label) {
            continue;
        }
        if let Some(p) = aggregate::majority_person(&loaded.faces, m) {
            let n = m
                .iter()
                .filter(|i| loaded.faces[**i].person == Some(p))
                .count();
            candidates.push((n, *label, p));
        }
    }
    candidates.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    for (_, label, p) in candidates {
        if !used.contains(&p) && !person_of.contains_key(&label) {
            person_of.insert(label, p);
            used.insert(p);
        }
    }
    for label in members.keys() {
        if !person_of.contains_key(label) {
            person_of.insert(*label, people::create_person(c, None)?);
        }
    }
    Ok(person_of)
}

/// Scrive assegnazioni, unioni automatiche (storico) e domande; restituisce le unioni
/// automatiche registrate.
fn write_outcome(c: &Connection, loaded: &Loaded, out: &Outcome) -> pc_store::StoreResult<i64> {
    let person_of = persons_for_groups(c, loaded, out)?;
    let faces = &loaded.faces;
    // Storico: prima delle scritture, con lo stato precedente dei volti.
    let mut logs = Vec::new();
    for m in out.visible_merges(faces) {
        let target = person_of[&m.into];
        let prev: HashSet<Option<i64>> = m.faces.iter().map(|i| faces[*i].person).collect();
        let source = match m.from {
            Label::Person(q) if q != target => q,
            _ => match prev.iter().next() {
                Some(Some(q)) if prev.len() == 1 && *q != target => *q,
                _ => people::reserve_person_id(c)?,
            },
        };
        let row = people::person_snapshot(c, source).ok();
        let ids: Vec<i64> = m.faces.iter().map(|i| faces[*i].id).collect();
        let cover = m
            .faces
            .iter()
            .max_by(|a, b| {
                faces[**a]
                    .quality
                    .total_cmp(&faces[**b].quality)
                    .then(faces[**b].id.cmp(&faces[**a].id))
            })
            .map(|i| faces[*i].id);
        logs.push((
            target,
            merge_log::SourceGroup {
                person: source,
                name: row.as_ref().and_then(|r| r.name.clone()),
                identity_uid: row.as_ref().and_then(|r| r.identity_uid.clone()),
                ignored_at: row.as_ref().and_then(|r| r.ignored_at),
                ignored_faces: row.as_ref().map_or(0, |r| r.ignored_faces),
                cover_face_id: row.as_ref().and_then(|r| r.cover_face_id),
                cover,
                faces: merge_log::faces_before(c, &ids)?,
            },
        ));
    }
    let mut media = HashSet::new();
    {
        let mut upd = c.prepare_cached(
            "UPDATE face SET person_id=?2, assigned_by='auto' WHERE id=?1 \
             AND COALESCE(assigned_by,'') <> 'manual'",
        )?;
        for (i, f) in faces.iter().enumerate() {
            let target = person_of[&out.labels[i]];
            // Anche i volti abbinati in lettura a una persona con nome vanno scritti.
            if loaded.stored[i] != Some(target) {
                upd.execute(rusqlite::params![f.id, target])?;
                media.insert(f.media);
            }
        }
    }
    let auto = logs.len() as i64;
    for (target, source) in logs {
        let target_row = people::person_snapshot(c, target).ok();
        merge_log::record(
            c,
            MergeOrigin::Auto,
            target,
            &merge_log::MergeSnapshot {
                sources: vec![source],
                target_name: target_row.as_ref().and_then(|t| t.name.clone()),
                target_identity: target_row.and_then(|t| t.identity_uid),
            },
        )?;
    }
    let questions: Vec<NewQuestion> = out
        .questions
        .iter()
        .map(|q| NewQuestion {
            kind: match q.kind {
                QuestionKind::GroupPerson => review::QuestionKind::GroupPerson,
                QuestionKind::GroupGroup => review::QuestionKind::GroupGroup,
                QuestionKind::GroupChoice => review::QuestionKind::GroupChoice,
            },
            a: person_of[&q.a],
            b: person_of[&q.b],
            c: q.c.map(|l| person_of[&l]),
            score: q.score,
            probability: q.probability,
            faces: q.faces as i64,
        })
        .collect();
    review::replace(c, &questions)?;
    people::refresh_media_ids(c, &media)?;
    Ok(auto)
}

/// Anteprima dell'effetto di nuove soglie sul raggruppamento, senza scrivere: unioni
/// automatiche e domande di un ricalcolo completo.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Preview {
    pub auto_merges: i64,
    pub questions: i64,
}

pub fn preview(
    store: &Store,
    thresholds: &pc_core::AiThresholds,
    control: &JobControl,
) -> AppResult<Preview> {
    let matching = crate::analyze::face_match(thresholds);
    let (loaded, cal) = store.read(|c| {
        let base = load(
            c,
            &matching,
            thresholds.face_similarity,
            &Default::default(),
        )?;
        let cal = calibrate::calibrate(
            &base.faces,
            &base.confirmed,
            thresholds.face_similarity,
            thresholds.face_merge_sensitivity,
        );
        Ok((base, cal))
    })?;
    if loaded.faces.is_empty() {
        return Ok(Preview::default());
    }
    let out = aggregate::aggregate(
        &loaded.faces,
        &loaded.people,
        cal.params,
        Mode::Full,
        control,
    )?;
    Ok(Preview {
        auto_merges: out.visible_merges(&loaded.faces).count() as i64,
        questions: out.questions.len() as i64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::faces::SAME_PERSON;

    fn node(id: i64, person: Option<i64>, axis: usize) -> Face {
        let mut v = vec![0.0; 128];
        v[axis] = 1.0;
        Face {
            id,
            media: id,
            model: "sface".into(),
            embedding: v,
            quality: 0.99,
            person,
            ..Default::default()
        }
    }

    #[test]
    fn identities_and_refusals_survive_reclustering() {
        let mut faces = vec![
            node(1, Some(10), 0),
            node(2, None, 0),
            node(3, None, 0),
            node(4, Some(20), 0),
            node(5, None, 1),
            node(6, None, 1),
        ];
        faces[2].rejected = vec![10];
        let people = People {
            protected: HashSet::from([10, 20]),
            named: HashSet::from([10]),
            ..Default::default()
        };
        let out = aggregate::aggregate(
            &faces,
            &people,
            aggregate::Params::default(),
            Mode::Full,
            &JobControl::new(),
        )
        .unwrap();
        let l = &out.labels;
        assert_eq!(l[0], Label::Person(10));
        assert_eq!(l[3], Label::Person(20));
        assert_ne!(l[2], Label::Person(10), "il rifiuto resta");
        assert_eq!(l[4], l[5]);
        assert_ne!(l[4], l[0]);
    }

    #[test]
    fn persisted_corrections_survive_multiple_clustering_runs() {
        let tmp = tempfile::tempdir().unwrap();
        let db = Store::open(&tmp.path().join("test.db"), Default::default()).unwrap();
        db.write(|c|{for id in 1..=4{
            c.execute("INSERT INTO media_item(id,path,name,ext,size,mtime,added_at,updated_at) VALUES (?1,?2,'test','jpg',1,1,1,1)",rusqlite::params![id,format!("test{id}.jpg")])?;
            let mut embedding=vec![0.0;128];embedding[0]=1.0;
            people::save_faces(c,id,&[people::DetectedFace{bbox:[0.1,0.1,0.2,0.2],quality:1.0,embedding,..Default::default()}],"sface","det+sface",SAME_PERSON)?;
        }Ok(())}).unwrap();
        let policy = std::sync::Arc::new(crate::face_policy::FacePolicy::new(true));
        let gov = MemoryGovernor::new(256);
        run(
            &db,
            &gov,
            &JobControl::new(),
            policy.clone(),
            &Default::default(),
            &Default::default(),
        )
        .unwrap();
        let (person, other, first, rejected) = db
            .write(|c| {
                let f = people::records(c, None)?;
                let person = f[0].person_id.unwrap();
                people::rename(c, person, "Alice")?;
                let other = people::separate(c, &[f[1].id], person)?;
                people::reject(c, f[2].id, person)?;
                Ok((person, other, f[0].id, f[2].id))
            })
            .unwrap();
        for _ in 0..2 {
            run(
                &db,
                &gov,
                &JobControl::new(),
                policy.clone(),
                &Default::default(),
                &Default::default(),
            )
            .unwrap();
        }
        db.read(|c| {
            let f = people::records(c, None)?;
            assert_eq!(
                f.iter().find(|f| f.id == first).unwrap().person_id,
                Some(person)
            );
            assert_eq!(f[1].person_id, Some(other));
            assert_ne!(
                f.iter().find(|f| f.id == rejected).unwrap().person_id,
                Some(person)
            );
            Ok(())
        })
        .unwrap();
    }

    /// Due volti con somiglianza 0,5 in due foto, con le confidenze indicate.
    fn two_faces(quality: [f32; 2]) -> (tempfile::TempDir, Store) {
        let tmp = tempfile::tempdir().unwrap();
        let db = Store::open(&tmp.path().join("t.db"), Default::default()).unwrap();
        db.write(move |c| {
            for (i, q) in quality.iter().enumerate() {
                let id = i as i64 + 1;
                c.execute("INSERT INTO media_item(id,path,name,ext,size,mtime,added_at,updated_at) VALUES (?1,?2,'t','jpg',1,1,1,1)",rusqlite::params![id,format!("t{id}.jpg")])?;
                let mut e = vec![0.0; 128];
                if i == 0 {
                    e[0] = 1.0;
                } else {
                    // cos(60°) = 0,5 con il primo volto.
                    e[0] = 0.5;
                    e[1] = 0.75f32.sqrt();
                }
                people::save_faces(
                    c,
                    id,
                    &[people::DetectedFace {
                        bbox: [0.1, 0.1, 0.2, 0.2],
                        quality: *q,
                        embedding: e,
                        ..Default::default()
                    }],
                    "sface",
                    "det+sface",
                    SAME_PERSON,
                )?;
            }
            Ok(())
        })
        .unwrap();
        (tmp, db)
    }

    fn persons_of(db: &Store) -> Vec<Option<i64>> {
        db.read(|c| {
            Ok(people::records(c, None)?
                .into_iter()
                .map(|f| f.person_id)
                .collect())
        })
        .unwrap()
    }

    fn cluster_with(db: &Store, t: pc_core::AiThresholds) {
        run(
            db,
            &MemoryGovernor::new(256),
            &JobControl::new(),
            std::sync::Arc::new(crate::face_policy::FacePolicy::new(true)),
            &Default::default(),
            &t,
        )
        .unwrap();
    }

    #[test]
    fn stricter_similarity_separates_faces_joined_by_default() {
        let (_t, db) = two_faces([0.99, 0.99]);
        cluster_with(&db, Default::default());
        let p = persons_of(&db);
        assert!(p[0].is_some());
        assert_eq!(p[0], p[1], "0,5 ≥ 0,363: stessa persona");
        cluster_with(
            &db,
            pc_core::AiThresholds {
                face_similarity: 0.6,
                ..Default::default()
            },
        );
        let p = persons_of(&db);
        assert!(p[0].is_some() && p[1].is_some());
        assert_ne!(p[0], p[1], "0,5 < 0,6: persone separate");
    }

    #[test]
    fn faces_below_detection_threshold_are_not_clustered() {
        // Scenario "Soglia di rilevamento alzata": volti a 0,95 e 0,85, soglia 0,9.
        let (_t, db) = two_faces([0.95, 0.85]);
        cluster_with(&db, Default::default());
        let p = persons_of(&db);
        assert_eq!(p[0], p[1]);
        cluster_with(
            &db,
            pc_core::AiThresholds {
                face_detection: 0.9,
                ..Default::default()
            },
        );
        let p = persons_of(&db);
        assert!(p[0].is_some());
        assert_eq!(
            p[1], None,
            "il volto a 0,85 non è usato per il raggruppamento"
        );
        // Il volto resta salvato.
        assert_eq!(p.len(), 2);
    }
}
