//! Identità dell'app (persone note) e volti di riferimento in `app.db`.
//! Gli embedding non escono verso la UI: solo conteggi e ritagli su richiesta.

use crate::people::{cosine, vector};
use crate::{now_ms, StoreError, StoreResult};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

/// Soglia di somiglianza predefinita (coseno SFace, come `pc_ai::faces::SAME_PERSON`).
pub const DEFAULT_THRESHOLD: f32 = 0.363;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Identity {
    pub uid: String,
    pub name: String,
    pub short_name: String,
    pub group: Option<String>,
    /// Soglia propria; `None` = [`DEFAULT_THRESHOLD`].
    pub threshold: Option<f32>,
    pub aliases: Vec<String>,
    /// Numero di volti di riferimento.
    pub faces: i64,
    pub created_at: i64,
}

impl Identity {
    pub fn effective_threshold(&self) -> f32 {
        self.threshold.unwrap_or(DEFAULT_THRESHOLD)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct IdentityInput {
    pub name: String,
    /// Vuoto o assente: la prima parola del nome.
    pub short_name: Option<String>,
    pub group: Option<String>,
    pub threshold: Option<f32>,
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct RefFaceView {
    pub id: i64,
    pub uid: String,
    pub model_ver: String,
    /// "library:<face id>" oppure "file:<nome file>".
    pub source: String,
    pub created_at: i64,
}

/// Volto di riferimento completo (uso interno: riconoscimento ed esportazione).
#[derive(Debug, Clone, PartialEq)]
pub struct RefFace {
    pub id: i64,
    pub uid: String,
    pub crop: Vec<u8>,
    pub model_ver: String,
    pub embedding: Option<Vec<f32>>,
    pub source: String,
    pub created_at: i64,
}

/// Nome breve predefinito: la prima parola del nome.
pub fn default_short_name(name: &str) -> String {
    name.split_whitespace().next().unwrap_or("").to_string()
}

fn clean(s: &Option<String>) -> Option<String> {
    s.as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn validate(input: &IdentityInput) -> StoreResult<(String, String)> {
    let name = input.name.trim();
    if name.is_empty() || name.len() > 256 {
        return Err(StoreError::Invalid("nome dell'identità non valido".into()));
    }
    if let Some(t) = input.threshold {
        if !t.is_finite() || !(0.0..=1.0).contains(&t) {
            return Err(StoreError::Invalid("soglia fuori dai limiti (0–1)".into()));
        }
    }
    let short = clean(&input.short_name).unwrap_or_else(|| default_short_name(name));
    Ok((name.to_string(), short))
}

fn conflict(e: rusqlite::Error, name: &str) -> StoreError {
    match e {
        rusqlite::Error::SqliteFailure(f, _)
            if f.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            StoreError::Conflict(format!("esiste già un'identità \"{name}\""))
        }
        other => other.into(),
    }
}

/// UUID v4 dai byte casuali di SQLite.
fn new_uid(c: &Connection) -> StoreResult<String> {
    let mut b: Vec<u8> = c.query_row("SELECT randomblob(16)", [], |r| r.get(0))?;
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h: String = b.iter().map(|x| format!("{x:02x}")).collect();
    Ok(format!(
        "{}-{}-{}-{}-{}",
        &h[0..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32]
    ))
}

fn set_aliases(c: &Connection, uid: &str, name: &str, aliases: &[String]) -> StoreResult<()> {
    c.execute("DELETE FROM identity_alias WHERE uid = ?1", [uid])?;
    let mut st =
        c.prepare_cached("INSERT OR IGNORE INTO identity_alias (uid, alias) VALUES (?1, ?2)")?;
    for a in aliases {
        let a = a.trim();
        if !a.is_empty() && a.to_lowercase() != name.to_lowercase() {
            st.execute(params![uid, a])?;
        }
    }
    Ok(())
}

pub fn create(c: &Connection, input: &IdentityInput) -> StoreResult<String> {
    let (name, short) = validate(input)?;
    let uid = new_uid(c)?;
    insert(c, &uid, &name, &short, input, now_ms())?;
    Ok(uid)
}

/// Inserisce un'identità con un uid già noto (importazione).
pub(crate) fn insert(
    c: &Connection,
    uid: &str,
    name: &str,
    short: &str,
    input: &IdentityInput,
    created_at: i64,
) -> StoreResult<()> {
    c.execute(
        "INSERT INTO identity (uid, name, short_name, grp, threshold, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            uid,
            name,
            short,
            clean(&input.group),
            input.threshold,
            created_at
        ],
    )
    .map_err(|e| conflict(e, name))?;
    set_aliases(c, uid, name, &input.aliases)
}

pub fn update(c: &Connection, uid: &str, input: &IdentityInput) -> StoreResult<()> {
    let (name, short) = validate(input)?;
    let n = c
        .execute(
            "UPDATE identity SET name = ?2, short_name = ?3, grp = ?4, threshold = ?5 WHERE uid = ?1",
            params![uid, name, short, clean(&input.group), input.threshold],
        )
        .map_err(|e| conflict(e, &name))?;
    if n == 0 {
        return Err(StoreError::NotFound(format!("identità {uid}")));
    }
    set_aliases(c, uid, &name, &input.aliases)
}

pub fn delete(c: &Connection, uid: &str) -> StoreResult<()> {
    c.pragma_update(None, "secure_delete", true)?;
    if c.execute("DELETE FROM identity WHERE uid = ?1", [uid])? == 0 {
        return Err(StoreError::NotFound(format!("identità {uid}")));
    }
    Ok(())
}

fn aliases_of(c: &Connection, uid: &str) -> StoreResult<Vec<String>> {
    let mut st = c.prepare_cached(
        "SELECT alias FROM identity_alias WHERE uid = ?1 ORDER BY alias COLLATE NOCASE",
    )?;
    let rows = st
        .query_map([uid], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

fn read_rows(
    c: &Connection,
    filter: &str,
    args: &[&dyn rusqlite::ToSql],
) -> StoreResult<Vec<Identity>> {
    let mut st = c.prepare(&format!(
        "SELECT i.uid, i.name, i.short_name, i.grp, i.threshold, i.created_at,
                (SELECT count(*) FROM identity_face f WHERE f.uid = i.uid)
         FROM identity i {filter} ORDER BY i.name COLLATE NOCASE"
    ))?;
    let rows = st
        .query_map(args, |r| {
            Ok(Identity {
                uid: r.get(0)?,
                name: r.get(1)?,
                short_name: r.get(2)?,
                group: r.get(3)?,
                threshold: r.get(4)?,
                created_at: r.get(5)?,
                faces: r.get(6)?,
                aliases: Vec::new(),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    rows.into_iter()
        .map(|mut i| {
            i.aliases = aliases_of(c, &i.uid)?;
            Ok(i)
        })
        .collect()
}

pub fn list(c: &Connection) -> StoreResult<Vec<Identity>> {
    read_rows(c, "", &[])
}

pub fn get(c: &Connection, uid: &str) -> StoreResult<Identity> {
    read_rows(c, "WHERE i.uid = ?1", &[&uid])?
        .into_iter()
        .next()
        .ok_or_else(|| StoreError::NotFound(format!("identità {uid}")))
}

/// Identità con nome o alias che contengono `q` (senza maiuscole); `q` vuota = tutte.
pub fn search(c: &Connection, q: &str) -> StoreResult<Vec<Identity>> {
    let q = q.trim();
    if q.is_empty() {
        return list(c);
    }
    let like = format!(
        "%{}%",
        q.replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_")
    );
    read_rows(
        c,
        "WHERE i.name LIKE ?1 ESCAPE '\\'
            OR EXISTS (SELECT 1 FROM identity_alias a WHERE a.uid = i.uid AND a.alias LIKE ?1 ESCAPE '\\')",
        &[&like],
    )
}

/// Identità con questo nome esatto (senza maiuscole).
pub fn find_by_name(c: &Connection, name: &str) -> StoreResult<Option<String>> {
    Ok(c.query_row(
        "SELECT uid FROM identity WHERE name = ?1",
        [name.trim()],
        |r| r.get(0),
    )
    .optional()?)
}

/// Identità con questo nome, creata se non esiste.
///
/// **Solo per** `people::migrate_to_identities` (migrazione v5 una tantum).
/// Ogni altra creazione di identità passa da `create` / `save_identity` /
/// import esplicito dell'utente — non usare qui per collegamenti o riconoscimento.
pub fn ensure_by_name(c: &Connection, name: &str) -> StoreResult<String> {
    match find_by_name(c, name)? {
        Some(uid) => Ok(uid),
        None => create(
            c,
            &IdentityInput {
                name: name.into(),
                ..Default::default()
            },
        ),
    }
}

/// Unisce `from` in `into`: alias (incluso il nome di `from`) e volti di
/// riferimento passano a `into`, `from` viene eliminata. I collegamenti nelle
/// librerie vanno spostati da chi chiama (`people::relink_identity`).
pub fn merge(c: &Connection, from: &str, into: &str) -> StoreResult<()> {
    if from == into {
        return Err(StoreError::Invalid("identità coincidenti".into()));
    }
    let source = get(c, from)?;
    let target = get(c, into)?;
    c.execute(
        "UPDATE identity_face SET uid = ?2 WHERE uid = ?1",
        params![from, into],
    )?;
    let mut aliases = target.aliases.clone();
    aliases.extend(source.aliases);
    aliases.push(source.name);
    if target.group.is_none() && source.group.is_some() {
        c.execute(
            "UPDATE identity SET grp = ?2 WHERE uid = ?1",
            params![into, source.group],
        )?;
    }
    c.execute("DELETE FROM identity WHERE uid = ?1", [from])?;
    set_aliases(c, into, &target.name, &aliases)
}

/// Sorgenti effettive di un'unione in `into`: senza doppioni e senza `into` stessa, nell'ordine
/// dato.
pub fn merge_sources(from: &[String], into: &str) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    from.iter()
        .filter(|u| u.as_str() != into && seen.insert(u.as_str()))
        .cloned()
        .collect()
}

/// Unisce più identità in `into` con la stessa semantica di [`merge`] (alias, nome delle
/// sorgenti come alias, volti di riferimento, gruppo), in un'unica operazione tutto-o-niente:
/// se una sorgente o la destinazione non esiste, o un passo fallisce, nessuna identità cambia.
/// `into` tra le sorgenti viene ignorata; i doppioni contano una volta. Restituisce le sorgenti
/// effettivamente unite (da passare a `people::relink_identities`).
pub fn merge_many(c: &Connection, from: &[String], into: &str) -> StoreResult<Vec<String>> {
    let sources = merge_sources(from, into);
    if sources.is_empty() {
        return Err(StoreError::Invalid("nessuna identità da unire".into()));
    }
    // Tutto-o-niente anche se chi chiama non ha aperto una transazione.
    c.execute_batch("SAVEPOINT merge_identities")?;
    match merge_many_inner(c, &sources, into) {
        Ok(()) => {
            c.execute_batch("RELEASE merge_identities")?;
            Ok(sources)
        }
        Err(e) => {
            c.execute_batch("ROLLBACK TO merge_identities; RELEASE merge_identities")?;
            Err(e)
        }
    }
}

fn merge_many_inner(c: &Connection, sources: &[String], into: &str) -> StoreResult<()> {
    let target = get(c, into)?;
    let mut group = target.group.clone();
    let mut aliases = target.aliases.clone();
    for uid in sources {
        let source = get(c, uid)?;
        c.execute(
            "UPDATE identity_face SET uid = ?2 WHERE uid = ?1",
            params![uid, into],
        )?;
        aliases.extend(source.aliases);
        aliases.push(source.name);
        if group.is_none() {
            group = source.group;
        }
        c.execute("DELETE FROM identity WHERE uid = ?1", [uid])?;
    }
    if group != target.group {
        c.execute(
            "UPDATE identity SET grp = ?2 WHERE uid = ?1",
            params![into, group],
        )?;
    }
    set_aliases(c, into, &target.name, &aliases)
}

/// Quanto verrebbe unito da [`merge_many`] (per la conferma): identità sorgenti e loro volti di
/// riferimento. Una sorgente inesistente è un errore.
pub fn merge_counts(c: &Connection, from: &[String], into: &str) -> StoreResult<(i64, i64)> {
    get(c, into)?;
    let sources = merge_sources(from, into);
    let mut faces = 0;
    for uid in &sources {
        faces += get(c, uid)?.faces;
    }
    Ok((sources.len() as i64, faces))
}

/// Gruppi usati, in ordine alfabetico.
pub fn groups(c: &Connection) -> StoreResult<Vec<String>> {
    let mut st = c.prepare(
        "SELECT DISTINCT grp FROM identity WHERE grp IS NOT NULL ORDER BY grp COLLATE NOCASE",
    )?;
    let rows = st.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
    Ok(rows)
}

// ------------------------------------------------------------ volti di riferimento

fn embedding_bytes(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

pub fn add_face(
    c: &Connection,
    uid: &str,
    crop: &[u8],
    model_ver: &str,
    embedding: Option<&[f32]>,
    source: &str,
) -> StoreResult<i64> {
    get(c, uid)?;
    if crop.is_empty() {
        return Err(StoreError::Invalid("ritaglio del volto vuoto".into()));
    }
    if let Some(v) = embedding {
        vector(&embedding_bytes(v))?;
    }
    c.execute(
        "INSERT INTO identity_face (uid, crop, model_ver, embedding, source, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            uid,
            crop,
            model_ver,
            embedding.map(embedding_bytes),
            source,
            now_ms()
        ],
    )?;
    Ok(c.last_insert_rowid())
}

pub fn remove_face(c: &Connection, id: i64) -> StoreResult<()> {
    c.pragma_update(None, "secure_delete", true)?;
    c.execute("DELETE FROM identity_face WHERE id = ?1", [id])?;
    Ok(())
}

pub fn faces(c: &Connection, uid: &str) -> StoreResult<Vec<RefFaceView>> {
    let mut st = c.prepare(
        "SELECT id, uid, model_ver, source, created_at FROM identity_face WHERE uid = ?1 ORDER BY id",
    )?;
    let rows = st
        .query_map([uid], |r| {
            Ok(RefFaceView {
                id: r.get(0)?,
                uid: r.get(1)?,
                model_ver: r.get(2)?,
                source: r.get(3)?,
                created_at: r.get(4)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

pub fn face_crop(c: &Connection, id: i64) -> StoreResult<Vec<u8>> {
    Ok(
        c.query_row("SELECT crop FROM identity_face WHERE id = ?1", [id], |r| {
            r.get(0)
        })?,
    )
}

pub fn all_faces(c: &Connection) -> StoreResult<Vec<RefFace>> {
    let mut st = c.prepare(
        "SELECT id, uid, crop, model_ver, embedding, source, created_at FROM identity_face ORDER BY id",
    )?;
    let rows = st
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Vec<u8>>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, Option<Vec<u8>>>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, i64>(6)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    rows.into_iter()
        .map(|(id, uid, crop, model_ver, emb, source, created_at)| {
            Ok(RefFace {
                id,
                uid,
                crop,
                model_ver,
                embedding: emb.map(|b| vector(&b)).transpose()?,
                source,
                created_at,
            })
        })
        .collect()
}

/// Volti di riferimento senza embedding per `model_ver` (da ricalcolare dal ritaglio).
pub fn stale_faces(c: &Connection, model_ver: &str) -> StoreResult<Vec<RefFace>> {
    Ok(all_faces(c)?
        .into_iter()
        .filter(|f| f.embedding.is_none() || f.model_ver != model_ver)
        .collect())
}

pub fn set_face_embedding(
    c: &Connection,
    id: i64,
    model_ver: &str,
    embedding: &[f32],
) -> StoreResult<()> {
    let b = embedding_bytes(embedding);
    vector(&b)?;
    c.execute(
        "UPDATE identity_face SET model_ver = ?2, embedding = ?3 WHERE id = ?1",
        params![id, model_ver, b],
    )?;
    Ok(())
}

/// Riferimento di un'identità per il riconoscimento: centroide normalizzato
/// dei suoi volti per la versione di modello indicata.
#[derive(Debug, Clone, PartialEq)]
pub struct Reference {
    pub uid: String,
    pub threshold: f32,
    pub centroid: Vec<f32>,
}

pub fn references(c: &Connection, model_ver: &str) -> StoreResult<Vec<Reference>> {
    let ids = list(c)?;
    let faces = all_faces(c)?;
    let mut out = Vec::new();
    for i in ids {
        let mut sum = vec![0.0f32; 128];
        let mut n = 0;
        for f in faces
            .iter()
            .filter(|f| f.uid == i.uid && f.model_ver == model_ver)
        {
            if let Some(v) = &f.embedding {
                for (a, b) in sum.iter_mut().zip(v) {
                    *a += b;
                }
                n += 1;
            }
        }
        if n == 0 {
            continue;
        }
        let norm = sum.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            sum.iter_mut().for_each(|x| *x /= norm);
        }
        out.push(Reference {
            threshold: i.effective_threshold(),
            uid: i.uid,
            centroid: sum,
        });
    }
    Ok(out)
}

/// Identità più simile a `v` tra i riferimenti, sopra la soglia propria di ciascuna.
pub fn best_reference<'a>(
    refs: &'a [Reference],
    v: &[f32],
    excluded: &[String],
) -> Option<(&'a Reference, f32)> {
    refs.iter()
        .filter(|r| !excluded.contains(&r.uid))
        .map(|r| (r, cosine(v, &r.centroid)))
        .filter(|(r, s)| *s >= r.threshold)
        .max_by(|a, b| a.1.total_cmp(&b.1))
}

/// "Elimina tutti i dati dei volti": rimuove i volti di riferimento, tiene le identità.
pub fn clear_faces(c: &Connection) -> StoreResult<()> {
    c.pragma_update(None, "secure_delete", true)?;
    c.execute("DELETE FROM identity_face", [])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppStore;

    fn input(name: &str) -> IdentityInput {
        IdentityInput {
            name: name.into(),
            ..Default::default()
        }
    }

    fn axis(i: usize) -> Vec<f32> {
        let mut v = vec![0.0; 128];
        v[i] = 1.0;
        v
    }

    #[test]
    fn duplicate_name_is_refused() {
        let s = AppStore::open_in_memory().unwrap();
        s.with(|c| {
            create(c, &input("Giulia Bianchi"))?;
            let err = create(c, &input("giulia bianchi")).unwrap_err();
            assert!(matches!(err, StoreError::Conflict(_)), "{err:?}");
            let other = create(c, &input("Marco Russo"))?;
            assert!(matches!(
                update(c, &other, &input("Giulia Bianchi")),
                Err(StoreError::Conflict(_))
            ));
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn default_short_name_is_first_word() {
        let s = AppStore::open_in_memory().unwrap();
        s.with(|c| {
            let uid = create(c, &input("  Giulia Bianchi "))?;
            let i = get(c, &uid)?;
            assert_eq!(i.name, "Giulia Bianchi");
            assert_eq!(i.short_name, "Giulia");
            assert_eq!(i.effective_threshold(), DEFAULT_THRESHOLD);
            assert_eq!(uid.len(), 36);
            update(
                c,
                &uid,
                &IdentityInput {
                    name: "Giulia Bianchi".into(),
                    short_name: Some("GiuB".into()),
                    group: Some("Famiglia".into()),
                    threshold: Some(0.42),
                    aliases: vec![],
                },
            )?;
            let i = get(c, &uid)?;
            assert_eq!(
                (i.short_name.as_str(), i.group.as_deref()),
                ("GiuB", Some("Famiglia"))
            );
            assert_eq!(i.threshold, Some(0.42));
            assert!(update(
                c,
                &uid,
                &IdentityInput {
                    threshold: Some(1.5),
                    ..input("x")
                }
            )
            .is_err());
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn search_by_alias() {
        let s = AppStore::open_in_memory().unwrap();
        s.with(|c| {
            create(
                c,
                &IdentityInput {
                    name: "Rita Rossi".into(),
                    aliases: vec!["Nonna Rita".into(), "Rita Rossi".into()],
                    ..Default::default()
                },
            )?;
            create(c, &input("Giulia Bianchi"))?;
            let hits = search(c, "nonna rita")?;
            assert_eq!(hits.len(), 1);
            assert_eq!(hits[0].name, "Rita Rossi");
            // Il nome stesso non viene duplicato tra gli alias.
            assert_eq!(hits[0].aliases, vec!["Nonna Rita"]);
            assert_eq!(search(c, "giu")?.len(), 1);
            assert_eq!(search(c, "")?.len(), 2);
            assert!(search(c, "%")?.is_empty());
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn merge_moves_aliases_and_references() {
        let s = AppStore::open_in_memory().unwrap();
        s.with(|c| {
            let from = create(
                c,
                &IdentityInput {
                    name: "G. Bianchi".into(),
                    aliases: vec!["Giuly".into()],
                    group: Some("Amici".into()),
                    ..Default::default()
                },
            )?;
            let into = create(c, &input("Giulia Bianchi"))?;
            add_face(c, &from, b"png", "sface", Some(&axis(0)), "file:a.jpg")?;
            add_face(c, &into, b"png", "sface", Some(&axis(1)), "file:b.jpg")?;
            merge(c, &from, &into)?;
            assert!(get(c, &from).is_err());
            let g = get(c, &into)?;
            assert_eq!(g.faces, 2);
            assert_eq!(g.aliases, vec!["G. Bianchi", "Giuly"]);
            assert_eq!(g.group.as_deref(), Some("Amici"));
            assert_eq!(list(c)?.len(), 1);
            Ok(())
        })
        .unwrap();
    }

    fn snapshot(c: &Connection) -> Vec<String> {
        list(c)
            .unwrap()
            .into_iter()
            .map(|i| format!("{:?}", (i.uid, i.name, i.aliases, i.faces, i.group)))
            .collect()
    }

    #[test]
    fn merge_many_moves_hundreds_of_small_identities() {
        let s = AppStore::open_in_memory().unwrap();
        s.tx(|c| {
            let into = create(c, &input("Sconosciuti"))?;
            let keep = create(c, &input("Giulia Bianchi"))?;
            let mut from = Vec::new();
            for i in 0..340 {
                let uid = create(
                    c,
                    &IdentityInput {
                        name: format!("Persona {i}"),
                        aliases: vec![format!("P{i}")],
                        group: (i == 7).then(|| "Amici".to_string()),
                        ..Default::default()
                    },
                )?;
                add_face(c, &uid, b"png", "sface", Some(&axis(i % 128)), "file:a.jpg")?;
                if i % 2 == 0 {
                    add_face(c, &uid, b"png", "sface", Some(&axis(1)), "file:b.jpg")?;
                }
                from.push(uid);
            }
            add_face(c, &into, b"png", "sface", Some(&axis(2)), "file:c.jpg")?;
            let merged = merge_many(c, &from, &into)?;
            assert_eq!(merged, from);
            let all = list(c)?;
            assert_eq!(all.len(), 2, "restano solo la destinazione e un'altra");
            assert!(get(c, &keep).is_ok());
            let t = get(c, &into)?;
            assert_eq!(t.faces, 1 + 340 + 170);
            assert_eq!(t.aliases.len(), 680);
            assert!(t.aliases.contains(&"Persona 12".to_string()));
            assert!(t.aliases.contains(&"P339".to_string()));
            assert_eq!(t.group.as_deref(), Some("Amici"));
            assert!(from.iter().all(|u| get(c, u).is_err()));
            assert_eq!(
                search(c, "persona 200")?.len(),
                1,
                "cercando un alias si trova la destinazione"
            );
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn merge_many_ignores_the_destination_and_duplicates() {
        let s = AppStore::open_in_memory().unwrap();
        s.with(|c| {
            let g = create(c, &input("Giulia Bianchi"))?;
            let a = create(c, &input("G. Bianchi"))?;
            let b = create(c, &input("Giulia B."))?;
            let merged = merge_many(c, &[a.clone(), g.clone(), b.clone(), a.clone()], &g)?;
            assert_eq!(merged, vec![a.clone(), b]);
            let all = list(c)?;
            assert_eq!(all.len(), 1);
            assert_eq!(all[0].uid, g);
            assert_eq!(all[0].aliases, vec!["G. Bianchi", "Giulia B."]);
            // Solo la destinazione: niente da unire.
            assert!(matches!(
                merge_many(c, std::slice::from_ref(&g), &g),
                Err(StoreError::Invalid(_))
            ));
            assert!(matches!(
                merge_many(c, &[], &g),
                Err(StoreError::Invalid(_))
            ));
            assert!(matches!(
                merge_many(c, &[a], "nessuna"),
                Err(StoreError::NotFound(_))
            ));
            assert_eq!(
                merge_sources(std::slice::from_ref(&g), &g),
                Vec::<String>::new()
            );
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn merge_many_error_midway_changes_nothing() {
        let s = AppStore::open_in_memory().unwrap();
        let (into, mut from) = s
            .with(|c| {
                let into = create(c, &input("Sconosciuti"))?;
                add_face(c, &into, b"png", "sface", Some(&axis(0)), "file:t.jpg")?;
                let mut from = Vec::new();
                for i in 0..50 {
                    let uid = create(
                        c,
                        &IdentityInput {
                            name: format!("Persona {i}"),
                            aliases: vec![format!("P{i}")],
                            group: Some("Gruppo".into()),
                            ..Default::default()
                        },
                    )?;
                    add_face(c, &uid, b"png", "sface", Some(&axis(i % 128)), "file:a.jpg")?;
                    from.push(uid);
                }
                Ok((into, from))
            })
            .unwrap();
        // La trentesima non esiste: le prime 29 sono già state spostate quando si scopre.
        from[29] = "inesistente".into();
        let before = s.with(|c| Ok(snapshot(c))).unwrap();
        // Sia con una transazione esterna (come il comando) sia senza.
        let err = s.tx(|c| merge_many(c, &from, &into)).unwrap_err();
        assert!(matches!(err, StoreError::NotFound(_)), "{err:?}");
        assert_eq!(s.with(|c| Ok(snapshot(c))).unwrap(), before);
        let err = s.with(|c| merge_many(c, &from, &into)).unwrap_err();
        assert!(matches!(err, StoreError::NotFound(_)), "{err:?}");
        assert_eq!(s.with(|c| Ok(snapshot(c))).unwrap(), before);
        // Il collegamento resta utilizzabile: senza l'errore l'unione riesce.
        from.remove(29);
        s.with(|c| merge_many(c, &from, &into)).unwrap();
        assert_eq!(
            s.with(list).unwrap().len(),
            2,
            "resta la trentesima, mai inclusa"
        );
        assert_eq!(s.with(|c| get(c, &into)).unwrap().faces, 50);
    }

    #[test]
    fn merge_counts_report_identities_and_reference_faces() {
        let s = AppStore::open_in_memory().unwrap();
        s.with(|c| {
            let into = create(c, &input("Dest"))?;
            let a = create(c, &input("A"))?;
            let b = create(c, &input("B"))?;
            add_face(c, &into, b"png", "sface", Some(&axis(0)), "file:t.jpg")?;
            add_face(c, &a, b"png", "sface", Some(&axis(1)), "file:a.jpg")?;
            add_face(c, &a, b"png", "sface", Some(&axis(2)), "file:a2.jpg")?;
            add_face(c, &b, b"png", "sface", Some(&axis(3)), "file:b.jpg")?;
            let counts = merge_counts(c, &[a.clone(), b, into.clone(), a.clone()], &into)?;
            assert_eq!(counts, (2, 3), "la destinazione e i doppioni non contano");
            assert!(merge_counts(c, &[a, "x".into()], &into).is_err());
            assert_eq!(list(c)?.len(), 3, "l'anteprima non modifica nulla");
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn references_per_model_with_own_threshold() {
        let s = AppStore::open_in_memory().unwrap();
        s.with(|c| {
            let g = create(
                c,
                &IdentityInput {
                    threshold: Some(0.9),
                    ..input("Giulia")
                },
            )?;
            let m = create(c, &input("Marco"))?;
            create(c, &input("Senza volti"))?;
            add_face(c, &g, b"png", "sface", Some(&axis(0)), "file:g.jpg")?;
            add_face(c, &m, b"png", "sface", Some(&axis(1)), "file:m.jpg")?;
            add_face(c, &m, b"png", "old", Some(&axis(2)), "file:m2.jpg")?;
            let refs = references(c, "sface")?;
            assert_eq!(refs.len(), 2);
            let mut probe = axis(0);
            probe[1] = 0.6;
            let n = probe.iter().map(|x| x * x).sum::<f32>().sqrt();
            probe.iter_mut().for_each(|x| *x /= n);
            // Con Giulia (soglia 0.9) il coseno ~0.857 non basta; Marco (0.363) sì (~0.514).
            let (best, _) = best_reference(&refs, &probe, &[]).unwrap();
            assert_eq!(best.uid, m);
            assert!(best_reference(&refs, &probe, std::slice::from_ref(&m)).is_none());
            assert_eq!(stale_faces(c, "sface")?.len(), 1);
            clear_faces(c)?;
            assert!(references(c, "sface")?.is_empty());
            assert_eq!(list(c)?.len(), 3, "le identità restano");
            Ok(())
        })
        .unwrap();
    }
}
