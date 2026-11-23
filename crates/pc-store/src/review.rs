//! Coda "Da confermare" (design D6) e misure del raggruppamento (design D8).
//!
//! Le domande si calcolano dopo ogni analisi (`pc-ai`) e si salvano in `review_question` già
//! ordinate. Le risposte si propagano: un "Sì" unisce i gruppi (le domande seguono il gruppo
//! unito e quelle ormai risolte spariscono), un "No" diventa un vincolo che toglie le domande
//! contraddette. Un "Sì" contro un vincolo esistente è un conflitto: non si unisce nulla finché
//! l'utente non sceglie di togliere il vincolo.

use crate::merge_log::{self, MergeOrigin};
use crate::people::{self, FaceMatch};
use crate::{bulk, now_ms, StoreError, StoreResult};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Tipo di domanda.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum QuestionKind {
    /// "È {persona}?": gruppo senza nome verso una persona con nome.
    GroupPerson,
    /// "Stessa persona?": due gruppi senza nome.
    GroupGroup,
    /// "È {A} o {B}?": due persone con nome quasi ugualmente simili.
    GroupChoice,
}

impl QuestionKind {
    fn as_str(self) -> &'static str {
        match self {
            QuestionKind::GroupPerson => "group_person",
            QuestionKind::GroupGroup => "group_group",
            QuestionKind::GroupChoice => "group_choice",
        }
    }
    fn parse(s: &str) -> Self {
        match s {
            "group_group" => QuestionKind::GroupGroup,
            "group_choice" => QuestionKind::GroupChoice,
            _ => QuestionKind::GroupPerson,
        }
    }
}

/// Domanda calcolata dall'analisi, già nell'ordine della coda.
#[derive(Debug, Clone, PartialEq)]
pub struct NewQuestion {
    pub kind: QuestionKind,
    pub a: i64,
    pub b: i64,
    pub c: Option<i64>,
    pub score: f32,
    pub probability: f32,
    pub faces: i64,
}

#[derive(Debug, Clone)]
struct Row {
    id: i64,
    kind: QuestionKind,
    a: i64,
    b: i64,
    c: Option<i64>,
}

fn load(c: &Connection) -> StoreResult<Vec<Row>> {
    let mut q = c.prepare("SELECT id, kind, a, b, c FROM review_question ORDER BY rank, id")?;
    let rows = q
        .query_map([], |r| {
            Ok(Row {
                id: r.get(0)?,
                kind: QuestionKind::parse(&r.get::<_, String>(1)?),
                a: r.get(2)?,
                b: r.get(3)?,
                c: r.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Sostituisce le domande con quelle dell'ultima analisi (nell'ordine dato).
pub fn replace(c: &Connection, questions: &[NewQuestion]) -> StoreResult<()> {
    c.execute("DELETE FROM review_question", [])?;
    let ts = now_ms();
    let mut ins = c.prepare_cached(
        "INSERT INTO review_question(kind, a, b, c, score, probability, faces, rank, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    )?;
    for (rank, q) in questions.iter().enumerate() {
        ins.execute(params![
            q.kind.as_str(),
            q.a,
            q.b,
            q.c,
            q.score,
            q.probability,
            q.faces,
            rank as i64,
            ts
        ])?;
    }
    drop(ins);
    cleanup(c)
}

/// Le domande che riguardavano i gruppi `from` riguardano ora `to` (da chiamare prima di
/// eliminare i gruppi uniti), poi si tolgono quelle risolte o doppie.
pub fn after_merge(c: &Connection, from: &[i64], to: i64) -> StoreResult<()> {
    if from.is_empty() {
        return Ok(());
    }
    let list = serde_json::to_string(from)?;
    for col in ["a", "b", "c"] {
        c.execute(
            &format!(
                "UPDATE review_question SET {col}=?2 WHERE {col} IN (SELECT value FROM json_each(?1))"
            ),
            params![list, to],
        )?;
    }
    cleanup(c)
}

/// Stato delle persone usato per normalizzare le domande.
struct PeopleState {
    named: HashSet<i64>,
    ignored: HashSet<i64>,
    faces: HashMap<i64, i64>,
}

fn people_state(c: &Connection, ids: &HashSet<i64>) -> StoreResult<PeopleState> {
    let mut st = PeopleState {
        named: HashSet::new(),
        ignored: HashSet::new(),
        faces: HashMap::new(),
    };
    let mut q = c.prepare_cached(
        "SELECT (name IS NOT NULL OR identity_uid IS NOT NULL), ignored_at IS NOT NULL, \
                (SELECT count(*) FROM face WHERE person_id = person.id) \
         FROM person WHERE id=?1",
    )?;
    for id in ids {
        if let Some((named, ignored, faces)) = q
            .query_row([*id], |r| {
                Ok((
                    r.get::<_, bool>(0)?,
                    r.get::<_, bool>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            })
            .optional()?
        {
            if named {
                st.named.insert(*id);
            }
            if ignored {
                st.ignored.insert(*id);
            }
            st.faces.insert(*id, faces);
        }
    }
    Ok(st)
}

/// Normalizza la coda: toglie le domande risolte (stesso gruppo), contraddette da un vincolo,
/// verso gruppi vuoti o ignorati, tra due persone con nome e i doppioni (resta la prima per
/// ordine); una "È A o B?" con una sola candidata rimasta diventa "È A?".
pub fn cleanup(c: &Connection) -> StoreResult<()> {
    let rows = load(c)?;
    if rows.is_empty() {
        return Ok(());
    }
    let ids: HashSet<i64> = rows
        .iter()
        .flat_map(|r| [Some(r.a), Some(r.b), r.c])
        .flatten()
        .collect();
    let st = people_state(c, &ids)?;
    let mut blocked_cache: HashMap<(i64, i64), bool> = HashMap::new();
    let mut blocked = |x: i64, y: i64| -> StoreResult<bool> {
        let key = (x.min(y), x.max(y));
        if let Some(v) = blocked_cache.get(&key) {
            return Ok(*v);
        }
        let v = people::blocked(c, x, y)?;
        blocked_cache.insert(key, v);
        Ok(v)
    };
    let alive = |p: i64| st.faces.get(&p).is_some_and(|n| *n > 0) && !st.ignored.contains(&p);
    let mut seen: HashSet<(QuestionKind, i64, i64, Option<i64>)> = HashSet::new();
    let mut delete = Vec::new();
    let mut update = Vec::new();
    for r in rows {
        let (mut a, mut b) = (r.a, r.b);
        let mut kind = r.kind;
        let mut third = r.c;
        let mut keep = alive(a);
        if kind == QuestionKind::GroupChoice {
            // Restano le candidate valide; con una sola diventa "È A?".
            let mut options: Vec<i64> = Vec::new();
            for p in [Some(b), third].into_iter().flatten() {
                if p != a && alive(p) && !options.contains(&p) && !blocked(a, p)? {
                    options.push(p);
                }
            }
            match options.as_slice() {
                [x] => {
                    b = *x;
                    third = None;
                    kind = QuestionKind::GroupPerson;
                }
                [x, y] => {
                    b = *x;
                    third = Some(*y);
                }
                _ => keep = false,
            }
        } else {
            keep = keep && a != b && alive(b) && !blocked(a, b)?;
        }
        if keep && kind != QuestionKind::GroupChoice {
            match (st.named.contains(&a), st.named.contains(&b)) {
                (true, true) => keep = false,
                (true, false) => {
                    std::mem::swap(&mut a, &mut b);
                    kind = QuestionKind::GroupPerson;
                }
                (false, true) => kind = QuestionKind::GroupPerson,
                (false, false) => kind = QuestionKind::GroupGroup,
            }
        }
        if keep && kind == QuestionKind::GroupChoice && st.named.contains(&a) {
            keep = false;
        }
        if keep {
            let key = match kind {
                QuestionKind::GroupGroup => (kind, a.min(b), a.max(b), None),
                QuestionKind::GroupChoice => {
                    (kind, a, b.min(third.unwrap()), third.map(|t| t.max(b)))
                }
                QuestionKind::GroupPerson => (kind, a, b, None),
            };
            keep = seen.insert(key);
        }
        if !keep {
            delete.push(r.id);
        } else if (kind, a, b, third) != (r.kind, r.a, r.b, r.c) {
            update.push((r.id, kind, a, b, third));
        }
    }
    for id in delete {
        c.execute("DELETE FROM review_question WHERE id=?1", [id])?;
    }
    for (id, kind, a, b, third) in update {
        c.execute(
            "UPDATE review_question SET kind=?2, a=?3, b=?4, c=?5 WHERE id=?1",
            params![id, kind.as_str(), a, b, third],
        )?;
    }
    Ok(())
}

/// Gruppo come lo mostra una domanda.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct QuestionGroup {
    pub id: i64,
    pub name: Option<String>,
    pub faces: i64,
    pub photos: i64,
    /// Volto di copertina (`face://<id>`).
    pub cover: Option<i64>,
}

/// Una domanda della coda.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ReviewQuestion {
    pub id: i64,
    pub kind: QuestionKind,
    /// Il gruppo senza nome della domanda.
    pub group: QuestionGroup,
    /// La persona (o il gruppo) proposta.
    pub other: QuestionGroup,
    /// La seconda persona di "È A o B?".
    pub second: Option<QuestionGroup>,
    /// Probabilità stimata (0..1) che la risposta sia "Sì": serve solo all'ordine e alle
    /// etichette ("molto probabile"); non è una somiglianza.
    pub probability: f32,
}

/// Elemento della coda: una domanda singola o le domande verso la stessa persona, da
/// confermare in blocco ("12 gruppi probabilmente di Gioia").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReviewItem {
    Single {
        question: ReviewQuestion,
    },
    Batch {
        person: QuestionGroup,
        questions: Vec<ReviewQuestion>,
    },
}

/// La coda "Da confermare".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ReviewQueue {
    pub items: Vec<ReviewItem>,
    /// Domande aperte in tutto.
    pub open: i64,
    /// Gruppi senza nome in "Da rivedere" (per la coda vuota).
    pub unnamed_groups: i64,
}

fn question_group(
    c: &Connection,
    cache: &mut HashMap<i64, Option<QuestionGroup>>,
    id: i64,
) -> StoreResult<Option<QuestionGroup>> {
    if let Some(g) = cache.get(&id) {
        return Ok(g.clone());
    }
    let row: Option<(Option<String>, i64, i64)> = c
        .query_row(
            "SELECT p.name, count(f.id), count(DISTINCT f.media_id) FROM person p \
             LEFT JOIN face f ON f.person_id = p.id WHERE p.id=?1 GROUP BY p.id",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let g = match row {
        Some((name, faces, photos)) if faces > 0 => Some(QuestionGroup {
            id,
            name,
            faces,
            photos,
            cover: merge_log::cover_of(c, id)?,
        }),
        _ => None,
    };
    cache.insert(id, g.clone());
    Ok(g)
}

/// La coda in ordine, con le domande "È {persona}?" verso la stessa persona riunite in un
/// solo elemento nella posizione della prima; al massimo `limit` elementi.
pub fn queue(c: &Connection, m: &FaceMatch, limit: usize) -> StoreResult<ReviewQueue> {
    let mut q =
        c.prepare("SELECT id, kind, a, b, c, probability FROM review_question ORDER BY rank, id")?;
    let rows: Vec<(i64, String, i64, i64, Option<i64>, f32)> = q
        .query_map([], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        })?
        .collect::<Result<_, _>>()?;
    let mut cache = HashMap::new();
    let mut items: Vec<ReviewItem> = Vec::new();
    let mut batch_at: HashMap<i64, usize> = HashMap::new();
    let mut open = 0i64;
    for (id, kind, a, b, third, probability) in rows {
        let kind = QuestionKind::parse(&kind);
        let (Some(group), Some(other)) = (
            question_group(c, &mut cache, a)?,
            question_group(c, &mut cache, b)?,
        ) else {
            continue;
        };
        let second = match third {
            Some(t) => question_group(c, &mut cache, t)?,
            None => None,
        };
        open += 1;
        let question = ReviewQuestion {
            id,
            kind,
            group,
            other: other.clone(),
            second,
            probability,
        };
        if kind == QuestionKind::GroupPerson {
            if let Some(i) = batch_at.get(&b) {
                match &mut items[*i] {
                    ReviewItem::Batch { questions, .. } => questions.push(question),
                    ReviewItem::Single { question: first } => {
                        let first = first.clone();
                        items[*i] = ReviewItem::Batch {
                            person: other,
                            questions: vec![first, question],
                        };
                    }
                }
                continue;
            }
            if items.len() < limit {
                batch_at.insert(b, items.len());
            }
        }
        if items.len() < limit {
            items.push(ReviewItem::Single { question });
        }
    }
    Ok(ReviewQueue {
        items,
        open,
        unnamed_groups: people::people_sections(c, m)?.review,
    })
}

/// Risposta a una domanda.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Answer {
    /// Sì: stessa persona (per "È A o B?" si usa `Choose`).
    Yes,
    /// No: diventa un vincolo.
    No,
    /// "È A o B?": è questa persona.
    Choose { person: i64 },
    /// "È A o B?": nessuna delle due.
    Neither,
}

/// Una risposta in conflitto con un vincolo dato prima dall'utente.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct AnswerConflict {
    pub question: i64,
    /// Gruppo da unire.
    pub group: i64,
    /// Persona o gruppo con cui c'è il vincolo ("Non è questa persona", "Persone diverse").
    pub person: i64,
}

/// Esito di una risposta.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct AnswerOutcome {
    /// Gruppi uniti.
    pub merged_groups: i64,
    /// Volti spostati.
    pub faces: i64,
    /// Gruppi che hanno ricevuto un vincolo.
    pub constrained_groups: i64,
    /// Conflitto: nessuna modifica finché l'utente non sceglie di togliere il vincolo.
    pub conflict: Option<AnswerConflict>,
}

fn question(c: &Connection, id: i64) -> StoreResult<Row> {
    c.query_row(
        "SELECT id, kind, a, b, c FROM review_question WHERE id=?1",
        [id],
        |r| {
            Ok(Row {
                id: r.get(0)?,
                kind: QuestionKind::parse(&r.get::<_, String>(1)?),
                a: r.get(2)?,
                b: r.get(3)?,
                c: r.get(4)?,
            })
        },
    )
    .optional()?
    .ok_or_else(|| StoreError::NotFound("domanda".into()))
}

fn faces_count(c: &Connection, person: i64) -> StoreResult<i64> {
    Ok(c.query_row(
        "SELECT count(*) FROM face WHERE person_id=?1",
        [person],
        |r| r.get(0),
    )?)
}

/// "Non è questa persona" per tutti i volti del gruppo `group` verso `person`, più il vincolo
/// "Persone diverse" tra i due gruppi; i volti restano nel loro gruppo. Restituisce le azioni
/// per l'annullamento.
fn constrain(c: &Connection, group: i64, person: i64) -> StoreResult<Vec<bulk::UndoAction>> {
    let faces: Vec<(i64, String)> = {
        let mut q = c.prepare("SELECT id, rejected FROM face WHERE person_id=?1 ORDER BY id")?;
        let v = q
            .query_map([group], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        v
    };
    let ids: Vec<i64> = faces.iter().map(|f| f.0).collect();
    let states = people::face_states(c, &ids)?;
    for (id, rejected) in faces {
        let mut list: Vec<i64> = serde_json::from_str(&rejected)?;
        if !list.contains(&person) {
            list.push(person);
            c.execute(
                "UPDATE face SET rejected=?2 WHERE id=?1",
                params![id, serde_json::to_string(&list)?],
            )?;
        }
    }
    let had_pair: bool = c.query_row(
        "SELECT EXISTS(SELECT 1 FROM person_not_same WHERE a=?1 AND b=?2)",
        params![group.min(person), group.max(person)],
        |r| r.get(0),
    )?;
    people::mark_not_same(c, group, person)?;
    let mut actions = vec![bulk::UndoAction::FacesRestored {
        states,
        created: None,
    }];
    if !had_pair {
        actions.push(bulk::UndoAction::PersonNotSame {
            a: group,
            b: person,
        });
    }
    Ok(actions)
}

/// Unione da una risposta "Sì": i volti diventano confermati (`assigned_by = 'manual'`).
fn merge_yes(
    c: &Connection,
    question: i64,
    from: &[i64],
    to: i64,
    override_conflict: bool,
) -> StoreResult<AnswerOutcome> {
    for g in from {
        if people::blocked(c, *g, to)? && !override_conflict {
            return Ok(AnswerOutcome {
                conflict: Some(AnswerConflict {
                    question,
                    group: *g,
                    person: to,
                }),
                ..Default::default()
            });
        }
    }
    let mut faces = 0;
    for g in from {
        faces += faces_count(c, *g)?;
    }
    people::merge_logged(
        c,
        from,
        to,
        MergeOrigin::Queue,
        "Conferma dalla coda",
        override_conflict,
    )?;
    Ok(AnswerOutcome {
        merged_groups: from.len() as i64,
        faces,
        ..Default::default()
    })
}

/// Risponde a una domanda della coda e propaga la risposta. Con `override_conflict` un "Sì" in
/// conflitto con un vincolo precedente toglie quel vincolo e unisce.
pub fn answer(
    c: &Connection,
    id: i64,
    answer: &Answer,
    override_conflict: bool,
) -> StoreResult<AnswerOutcome> {
    let q = question(c, id)?;
    let out = match (q.kind, answer) {
        (QuestionKind::GroupChoice, Answer::Choose { person }) => {
            if *person != q.b && Some(*person) != q.c {
                return Err(StoreError::Invalid("persona non proposta".into()));
            }
            merge_yes(c, id, &[q.a], *person, override_conflict)?
        }
        (QuestionKind::GroupChoice, Answer::Neither) => {
            let mut actions = constrain(c, q.a, q.b)?;
            if let Some(third) = q.c {
                actions.extend(constrain(c, q.a, third)?);
            }
            bulk::record_undo(c, "Nessuna delle due", &bulk::UndoAction::Group { actions })?;
            AnswerOutcome {
                constrained_groups: 1,
                ..Default::default()
            }
        }
        (QuestionKind::GroupChoice, _) | (_, Answer::Choose { .. } | Answer::Neither) => {
            return Err(StoreError::Invalid(
                "risposta non valida per la domanda".into(),
            ));
        }
        (QuestionKind::GroupPerson, Answer::Yes) => {
            merge_yes(c, id, &[q.a], q.b, override_conflict)?
        }
        (QuestionKind::GroupGroup, Answer::Yes) => {
            // Il gruppo più piccolo entra nel più grande (a parità, in quello con id minore).
            let (fa, fb) = (faces_count(c, q.a)?, faces_count(c, q.b)?);
            let (from, to) = if (fa, -q.a) > (fb, -q.b) {
                (q.b, q.a)
            } else {
                (q.a, q.b)
            };
            merge_yes(c, id, &[from], to, override_conflict)?
        }
        (_, Answer::No) => {
            let actions = constrain(c, q.a, q.b)?;
            let label = if q.kind == QuestionKind::GroupGroup {
                "Persone diverse"
            } else {
                "Non è questa persona"
            };
            bulk::record_undo(c, label, &bulk::UndoAction::Group { actions })?;
            AnswerOutcome {
                constrained_groups: 1,
                ..Default::default()
            }
        }
    };
    if out.conflict.is_none() {
        c.execute("DELETE FROM review_question WHERE id=?1", [id])?;
        cleanup(c)?;
    }
    Ok(out)
}

/// "Conferma tutti" su un elemento in blocco verso `person`: le domande `confirm` passano a
/// `person` con un'unica unione annullabile; i gruppi delle domande `reject` (tolti dall'utente)
/// ricevono "Non è questa persona" verso `person`. Tutto in una sola operazione annullabile.
pub fn answer_batch(
    c: &Connection,
    person: i64,
    confirm: &[i64],
    reject: &[i64],
    override_conflict: bool,
) -> StoreResult<AnswerOutcome> {
    let mut from = Vec::new();
    let mut ids = Vec::new();
    for id in confirm {
        let q = question(c, *id)?;
        if q.kind != QuestionKind::GroupPerson || q.b != person {
            return Err(StoreError::Invalid("domanda non di questa persona".into()));
        }
        if !from.contains(&q.a) {
            from.push(q.a);
        }
        ids.push(q.id);
    }
    let mut refused = Vec::new();
    for id in reject {
        let q = question(c, *id)?;
        if q.kind != QuestionKind::GroupPerson || q.b != person {
            return Err(StoreError::Invalid("domanda non di questa persona".into()));
        }
        refused.push(q.a);
        ids.push(q.id);
    }
    let mut out = AnswerOutcome::default();
    bulk::with_undo_group(c, "Conferma in blocco", || {
        if !from.is_empty() {
            out = merge_yes(c, confirm[0], &from, person, override_conflict)?;
        }
        if out.conflict.is_none() {
            for g in &refused {
                let actions = constrain(c, *g, person)?;
                bulk::record_undo(
                    c,
                    "Non è questa persona",
                    &bulk::UndoAction::Group { actions },
                )?;
                out.constrained_groups += 1;
            }
        }
        Ok(())
    })?;
    if out.conflict.is_none() {
        for id in ids {
            c.execute("DELETE FROM review_question WHERE id=?1", [id])?;
        }
        cleanup(c)?;
    }
    Ok(out)
}

// ------------------------------------------------------------------ misure (design D8)

const THRESHOLDS_KEY: &str = "faces.thresholds";
const METRICS_KEY: &str = "faces.metrics";
const SUMMARY_KEY: &str = "faces.first_summary";

/// Soglie del raggruppamento in uso e loro origine.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ThresholdsInUse {
    /// Soglia "chiedi" (anche soglia del grafo).
    pub ask: f32,
    /// Soglia "unisci da solo".
    pub auto: f32,
    /// Margine verso la seconda candidata.
    pub margin: f32,
    /// `true` se tarate sulla libreria, `false` se predefinite.
    pub calibrated: bool,
    /// Coppie di volti della stessa foto usate per la taratura.
    pub same_photo_pairs: i64,
    /// Coppie di volti confermati della stessa persona usate per il controllo.
    pub confirmed_pairs: i64,
    /// Più del 20% delle coppie confermate supera appena "chiedi": soglie probabilmente
    /// troppo severe (le soglie non si abbassano da sole).
    pub too_strict: bool,
    /// Sensibilità impostata (0 = più domande, 1 = più unioni automatiche).
    pub sensitivity: f32,
}

/// Misure del raggruppamento mostrate in Diagnostica.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ClusterMetrics {
    /// Istante del calcolo (ms).
    pub at: i64,
    /// Persone con nome (o collegate a un'identità), non ignorate.
    pub named_people: i64,
    /// Gruppi per persona con nome: 1 più i gruppi senza nome con una domanda aperta verso
    /// quella persona, in media.
    pub groups_per_named: f32,
    /// Gruppi senza nome (non ignorati) con almeno un volto.
    pub unnamed_groups: i64,
    /// Quota (0..1) dei gruppi senza nome con un solo volto.
    pub single_face_share: f32,
    /// Unioni automatiche degli ultimi 90 giorni e quante annullate.
    pub auto_merges: i64,
    pub auto_merges_undone: i64,
    /// Unioni automatiche dell'ultima analisi.
    pub last_auto_merges: i64,
    /// Domande aperte nella coda.
    pub questions: i64,
    pub thresholds: Option<ThresholdsInUse>,
}

fn kv_get<T: serde::de::DeserializeOwned>(c: &Connection, key: &str) -> StoreResult<Option<T>> {
    let raw: Option<String> = c
        .query_row("SELECT value FROM settings_kv WHERE key=?1", [key], |r| {
            r.get(0)
        })
        .optional()?;
    Ok(raw.and_then(|s| serde_json::from_str(&s).ok()))
}

fn kv_set<T: Serialize>(c: &Connection, key: &str, value: &T) -> StoreResult<()> {
    c.execute(
        "INSERT INTO settings_kv(key,value) VALUES (?1,?2) \
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![key, serde_json::to_string(value)?],
    )?;
    Ok(())
}

pub fn save_thresholds(c: &Connection, t: &ThresholdsInUse) -> StoreResult<()> {
    kv_set(c, THRESHOLDS_KEY, t)
}

pub fn thresholds(c: &Connection) -> StoreResult<Option<ThresholdsInUse>> {
    kv_get(c, THRESHOLDS_KEY)
}

/// Misure calcolate ora dal catalogo (le soglie sono quelle salvate dall'ultima analisi).
pub fn metrics(c: &Connection) -> StoreResult<ClusterMetrics> {
    let named: Vec<i64> = {
        let mut q = c.prepare(
            "SELECT p.id FROM person p WHERE (p.name IS NOT NULL OR p.identity_uid IS NOT NULL) \
             AND p.ignored_at IS NULL AND EXISTS(SELECT 1 FROM face f WHERE f.person_id = p.id)",
        )?;
        let v = q.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
        v
    };
    let mut asked: HashMap<i64, HashSet<i64>> = HashMap::new();
    {
        let mut q = c.prepare("SELECT a, b, c FROM review_question")?;
        for row in q.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, Option<i64>>(2)?,
            ))
        })? {
            let (a, b, third) = row?;
            for p in [Some(b), third].into_iter().flatten() {
                asked.entry(p).or_default().insert(a);
            }
        }
    }
    let groups_per_named = if named.is_empty() {
        0.0
    } else {
        let total: usize = named
            .iter()
            .map(|p| 1 + asked.get(p).map_or(0, |s| s.len()))
            .sum();
        total as f32 / named.len() as f32
    };
    let (unnamed, singles): (i64, i64) = c.query_row(
        "SELECT count(*), COALESCE(sum(n = 1), 0) FROM ( \
           SELECT count(f.id) AS n FROM person p JOIN face f ON f.person_id = p.id \
           WHERE p.name IS NULL AND p.identity_uid IS NULL AND p.ignored_at IS NULL \
           GROUP BY p.id)",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let (auto, undone) = merge_log::auto_counts(c)?;
    let questions: i64 = c.query_row("SELECT count(*) FROM review_question", [], |r| r.get(0))?;
    let last: Option<ClusterMetrics> = kv_get(c, METRICS_KEY)?;
    Ok(ClusterMetrics {
        at: now_ms(),
        named_people: named.len() as i64,
        groups_per_named,
        unnamed_groups: unnamed,
        single_face_share: if unnamed > 0 {
            singles as f32 / unnamed as f32
        } else {
            0.0
        },
        auto_merges: auto,
        auto_merges_undone: undone,
        last_auto_merges: last.map_or(0, |m| m.last_auto_merges),
        questions,
        thresholds: thresholds(c)?,
    })
}

/// Calcola e salva le misure dopo un'analisi che ha fatto `auto_merges` unioni automatiche.
pub fn save_metrics(c: &Connection, auto_merges: i64) -> StoreResult<ClusterMetrics> {
    let mut m = metrics(c)?;
    m.last_auto_merges = auto_merges;
    kv_set(c, METRICS_KEY, &m)?;
    Ok(m)
}

/// Misure salvate dall'ultima analisi.
pub fn saved_metrics(c: &Connection) -> StoreResult<Option<ClusterMetrics>> {
    kv_get(c, METRICS_KEY)
}

/// Riepilogo della prima riaggregazione dopo l'aggiornamento ("52 gruppi uniti
/// automaticamente"), da mostrare una sola volta.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct AggregationSummary {
    pub auto_merges: i64,
    pub at: i64,
    #[serde(default)]
    pub shown: bool,
}

pub fn save_first_summary(c: &Connection, auto_merges: i64) -> StoreResult<()> {
    kv_set(
        c,
        SUMMARY_KEY,
        &AggregationSummary {
            auto_merges,
            at: now_ms(),
            shown: false,
        },
    )
}

/// Il riepilogo, se non è ancora stato mostrato.
pub fn first_summary(c: &Connection) -> StoreResult<Option<AggregationSummary>> {
    Ok(kv_get::<AggregationSummary>(c, SUMMARY_KEY)?.filter(|s| !s.shown))
}

/// Segna il riepilogo come mostrato: non compare più.
pub fn dismiss_first_summary(c: &Connection) -> StoreResult<()> {
    if let Some(mut s) = kv_get::<AggregationSummary>(c, SUMMARY_KEY)? {
        s.shown = true;
        kv_set(c, SUMMARY_KEY, &s)?;
    }
    Ok(())
}
