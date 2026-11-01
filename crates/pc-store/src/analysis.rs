//! Risultati automatici idempotenti. Le correzioni manuali prevalgono per gruppo.
use crate::{repo, StoreError, StoreResult};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutoLabel {
    pub group: String,
    pub value: String,
    pub score: f64,
}

pub fn version(
    c: &Connection,
    id: i64,
    stage: &str,
    model: &str,
    taxonomy: Option<i64>,
) -> StoreResult<()> {
    c.execute("INSERT INTO analysis_version(media_id,stage,model_ver,taxonomy_ver) VALUES (?1,?2,?3,?4)
        ON CONFLICT(media_id,stage) DO UPDATE SET model_ver=excluded.model_ver,taxonomy_ver=excluded.taxonomy_ver", params![id,stage,model,taxonomy])?;
    Ok(())
}

pub fn classification(
    c: &Connection,
    id: i64,
    groups: &[String],
    labels: &[AutoLabel],
    embedding: &[f32],
    model: &str,
    taxonomy: i64,
) -> StoreResult<()> {
    if embedding.len() != 512
        || embedding.iter().any(|x| !x.is_finite())
        || labels.iter().any(|l| {
            !l.score.is_finite() || !(0.0..=1.0).contains(&l.score) || !groups.contains(&l.group)
        })
    {
        return Err(StoreError::Invalid(
            "risultati di classificazione non validi".into(),
        ));
    }
    for group in groups {
        let manual: bool = c.query_row(
            "SELECT EXISTS(SELECT 1 FROM label WHERE media_id=?1 AND grp=?2 AND origin='manual')",
            params![id, group],
            |r| r.get(0),
        )?;
        if manual {
            continue;
        }
        c.execute(
            "DELETE FROM label WHERE media_id=?1 AND grp=?2 AND origin='auto'",
            params![id, group],
        )?;
        for l in labels.iter().filter(|l| &l.group == group) {
            c.execute("INSERT INTO label(media_id,grp,value,score,origin,model_ver,taxonomy_ver) VALUES (?1,?2,?3,?4,'auto',?5,?6)",params![id,l.group,l.value,l.score,model,taxonomy])?;
        }
    }
    let bytes: Vec<u8> = embedding.iter().flat_map(|x| x.to_le_bytes()).collect();
    c.execute("INSERT INTO embedding(media_id,model_ver,vector) VALUES (?1,?2,?3) ON CONFLICT(media_id,model_ver) DO UPDATE SET vector=excluded.vector",params![id,model,bytes])?;
    version(c, id, "clip", model, Some(taxonomy))?;
    repo::fts::refresh(c, id)?;
    Ok(())
}

/// Separatore del pavimento di salvataggio nella versione di un risultato:
/// `<modello>|floor=0.25`. Le versioni senza suffisso sono state salvate prima
/// dei pavimenti, con la soglia di allora come minimo.
const FLOOR_SEP: &str = "|floor=";

/// Versione di un risultato salvato con il pavimento `floor`.
pub fn with_floor(model: &str, floor: f32) -> String {
    format!("{model}{FLOOR_SEP}{floor}")
}

/// Pavimento registrato in una versione; `None` per le versioni precedenti.
pub fn saved_floor(model_ver: &str) -> Option<f32> {
    model_ver
        .split_once(FLOOR_SEP)
        .and_then(|(_, f)| f.parse().ok())
}

/// Condizione SQL "la versione `col` corrisponde al modello `param`", con o
/// senza pavimento.
fn same_model(col: &str, param: &str) -> String {
    format!(
        "({col}={param} OR substr({col},1,length({param})+{n})={param}||'{FLOOR_SEP}')",
        n = FLOOR_SEP.len()
    )
}

fn persons_count(boxes: &[[f32; 5]], threshold: f32) -> i64 {
    boxes.iter().filter(|b| b[4] >= threshold).count() as i64
}

/// Riquadri persone senza soglia di conteggio (tutti contati).
pub fn people(c: &Connection, id: i64, boxes: &[[f32; 5]], model: &str) -> StoreResult<()> {
    people_with(c, id, boxes, model, 0.0, None)
}

/// Salva tutti i riquadri (sopra il pavimento `floor` usato dal rilevatore) e
/// conta solo quelli con confidenza `>= threshold`.
pub fn people_with(
    c: &Connection,
    id: i64,
    boxes: &[[f32; 5]],
    model: &str,
    threshold: f32,
    floor: Option<f32>,
) -> StoreResult<()> {
    if boxes
        .iter()
        .flatten()
        .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
    {
        return Err(StoreError::Invalid("riquadri persone non validi".into()));
    }
    c.execute("INSERT INTO media_people(media_id,count,boxes,model_ver) VALUES (?1,?2,?3,?4) ON CONFLICT(media_id) DO UPDATE SET count=excluded.count,boxes=excluded.boxes,model_ver=excluded.model_ver",params![id,persons_count(boxes, threshold),serde_json::to_string(boxes)?,model])?;
    let ver = floor.map_or_else(|| model.to_string(), |f| with_floor(model, f));
    version(c, id, "persons", &ver, None)
}

/// Ricalcola i conteggi delle persone dai riquadri salvati con una nuova
/// soglia. Con `write` falso non scrive (anteprima). Restituisce i media il
/// cui conteggio cambia.
pub fn recount_people(c: &Connection, threshold: f32, write: bool) -> StoreResult<Vec<i64>> {
    let mut q = c.prepare("SELECT media_id,count,boxes FROM media_people ORDER BY media_id")?;
    let rows = q
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut changed = Vec::new();
    for (id, count, boxes) in rows {
        let boxes: Vec<[f32; 5]> = serde_json::from_str(&boxes).unwrap_or_default();
        let n = persons_count(&boxes, threshold);
        if n != count {
            if write {
                c.execute(
                    "UPDATE media_people SET count=?2 WHERE media_id=?1",
                    params![id, n],
                )?;
            }
            changed.push(id);
        }
    }
    Ok(changed)
}

/// Rende "da rianalizzare" lo stadio `stage` dei media indicati (i risultati
/// restano finché la rianalisi non li sostituisce).
pub fn forget(c: &Connection, ids: &[i64], stage: &str) -> StoreResult<()> {
    let mut q = c.prepare_cached("DELETE FROM analysis_version WHERE media_id=?1 AND stage=?2")?;
    for id in ids {
        q.execute(params![id, stage])?;
    }
    Ok(())
}

/// Media (presenti) il cui risultato `stage` è stato salvato con un minimo più
/// alto di `threshold`: per loro abbassare la soglia richiede una rianalisi.
/// `legacy_floor` è il minimo dei risultati salvati senza pavimento.
pub fn below_floor(
    c: &Connection,
    stage: &str,
    threshold: f32,
    legacy_floor: f32,
) -> StoreResult<Vec<i64>> {
    let mut q = c.prepare(
        "SELECT a.media_id,a.model_ver FROM analysis_version a JOIN media_item m ON m.id=a.media_id
         WHERE a.stage=?1 AND m.status='ok' ORDER BY a.media_id",
    )?;
    let rows = q
        .query_map([stage], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows
        .into_iter()
        .filter(|(_, v)| saved_floor(v).unwrap_or(legacy_floor) > threshold + 1e-6)
        .map(|(id, _)| id)
        .collect())
}

pub fn pending(
    c: &Connection,
    ids: &[i64],
    stage: &str,
    model: &str,
    taxonomy: Option<i64>,
) -> StoreResult<Vec<i64>> {
    let mut q = c.prepare_cached(&format!(
        "SELECT EXISTS(SELECT 1 FROM analysis_version WHERE media_id=?1 AND stage=?2 AND {} AND taxonomy_ver IS ?4)",
        same_model("model_ver", "?3")
    ))?;
    let mut out = Vec::new();
    for id in ids {
        if !q.query_row(params![id, stage, model, taxonomy], |r| r.get::<_, bool>(0))? {
            out.push(*id);
        }
    }
    Ok(out)
}

pub fn stale_count(
    c: &Connection,
    stage: &str,
    model: &str,
    taxonomy: Option<i64>,
) -> StoreResult<i64> {
    Ok(c.query_row(&format!("SELECT count(*) FROM media_item m WHERE m.status='ok' AND m.thumb IS NOT NULL
        AND NOT EXISTS(SELECT 1 FROM analysis_version a WHERE a.media_id=m.id AND a.stage=?1 AND {} AND a.taxonomy_ver IS ?3)", same_model("a.model_ver", "?2")),params![stage,model,taxonomy],|r|r.get(0))?)
}

/// Nuove etichette automatiche dalla rivalutazione delle soglie: sostituisce
/// le etichette `auto` dei gruppi senza correzione manuale e marca il media
/// con la versione `taxonomy` della tassonomia, senza toccare l'embedding.
/// Restituisce vero se le etichette del media sono cambiate.
pub fn relabel(
    c: &Connection,
    id: i64,
    groups: &[String],
    labels: &[AutoLabel],
    model: &str,
    taxonomy: i64,
    write: bool,
) -> StoreResult<bool> {
    let mut changed = false;
    let mut q = c.prepare_cached(
        "SELECT value FROM label WHERE media_id=?1 AND grp=?2 AND origin='auto' ORDER BY value",
    )?;
    let mut manual = c.prepare_cached(
        "SELECT EXISTS(SELECT 1 FROM label WHERE media_id=?1 AND grp=?2 AND origin='manual')",
    )?;
    for group in groups {
        if manual.query_row(params![id, group], |r| r.get::<_, bool>(0))? {
            continue;
        }
        let old: Vec<String> = q
            .query_map(params![id, group], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        let mut new: Vec<&AutoLabel> = labels.iter().filter(|l| &l.group == group).collect();
        new.sort_by(|a, b| a.value.cmp(&b.value));
        let same = old.len() == new.len() && old.iter().zip(&new).all(|(o, n)| *o == n.value);
        if same {
            if write {
                // Punteggi invariati nei valori, versione della tassonomia aggiornata.
                c.execute(
                    "UPDATE label SET taxonomy_ver=?3 WHERE media_id=?1 AND grp=?2 AND origin='auto'",
                    params![id, group, taxonomy],
                )?;
            }
            continue;
        }
        changed = true;
        if write {
            c.execute(
                "DELETE FROM label WHERE media_id=?1 AND grp=?2 AND origin='auto'",
                params![id, group],
            )?;
            for l in new {
                c.execute("INSERT INTO label(media_id,grp,value,score,origin,model_ver,taxonomy_ver) VALUES (?1,?2,?3,?4,'auto',?5,?6)",params![id,l.group,l.value,l.score,model,taxonomy])?;
            }
        }
    }
    if write {
        version(c, id, "clip", model, Some(taxonomy))?;
        if changed {
            repo::fts::refresh(c, id)?;
        }
    }
    Ok(changed)
}

/// Embedding CLIP salvati per `model` (id del media, vettore), a pagine di
/// `limit` media con id maggiore di `after`.
pub fn embeddings(
    c: &Connection,
    model: &str,
    after: i64,
    limit: i64,
) -> StoreResult<Vec<(i64, Vec<f32>)>> {
    let mut q = c.prepare_cached(
        "SELECT e.media_id,e.vector FROM embedding e JOIN media_item m ON m.id=e.media_id
         WHERE e.model_ver=?1 AND m.status='ok' AND e.media_id>?2 ORDER BY e.media_id LIMIT ?3",
    )?;
    let rows = q
        .query_map(params![model, after, limit], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, Vec<u8>>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows
        .into_iter()
        .filter(|(_, b)| b.len() % 4 == 0)
        .map(|(id, b)| {
            (
                id,
                b.as_chunks::<4>()
                    .0
                    .iter()
                    .map(|x| f32::from_le_bytes(*x))
                    .collect(),
            )
        })
        .collect())
}

/// Numero di media con embedding per `model`.
pub fn embedding_count(c: &Connection, model: &str) -> StoreResult<i64> {
    Ok(c.query_row(
        "SELECT count(*) FROM embedding e JOIN media_item m ON m.id=e.media_id
         WHERE e.model_ver=?1 AND m.status='ok'",
        [model],
        |r| r.get(0),
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reanalysis_preserves_manual_groups_and_tracks_current_versions() {
        let mut c = Connection::open_in_memory().unwrap();
        crate::migrate::migrate(&mut c, None).unwrap();
        c.execute("INSERT INTO media_item(id,path,name,ext,size,mtime,added_at,updated_at) VALUES (1,'a.jpg','a.jpg','jpg',1,1,1,1)",[]).unwrap();
        c.execute("INSERT INTO label(media_id,grp,value,score,origin) VALUES (1,'subject','opera_arte',1,'manual')",[]).unwrap();
        let groups = vec!["subject".into(), "environment".into()];
        let labels = vec![
            AutoLabel {
                group: "subject".into(),
                value: "natura".into(),
                score: 0.9,
            },
            AutoLabel {
                group: "environment".into(),
                value: "montagna".into(),
                score: 0.8,
            },
        ];
        classification(&c, 1, &groups, &labels, &vec![0.0; 512], "v1", 1).unwrap();
        classification(&c, 1, &groups, &labels, &vec![0.0; 512], "v2", 2).unwrap();
        let subject: String = c
            .query_row("SELECT value FROM label WHERE grp='subject'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(subject, "opera_arte");
        assert!(pending(&c, &[1], "clip", "v2", Some(2)).unwrap().is_empty());
        assert_eq!(pending(&c, &[1], "clip", "v3", Some(2)).unwrap(), vec![1]);
        assert_eq!(pending(&c, &[1], "clip", "v2", Some(3)).unwrap(), vec![1]);
        c.execute("UPDATE media_item SET thumb='feedfeed' WHERE id=1", [])
            .unwrap();
        assert_eq!(stale_count(&c, "clip", "v2", Some(2)).unwrap(), 0);
        assert_eq!(stale_count(&c, "clip", "v3", Some(2)).unwrap(), 1);
        people(&c, 1, &[[0.1, 0.1, 0.2, 0.3, 0.9]], "person-v1").unwrap();
        people(&c, 1, &[], "person-v2").unwrap();
        assert_eq!(
            c.query_row("SELECT count FROM media_people", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
    }

    #[test]
    fn reanalysis_leaves_user_tags_unchanged_and_separate_from_ai_labels() {
        let mut c = Connection::open_in_memory().unwrap();
        crate::migrate::migrate(&mut c, None).unwrap();
        c.execute("INSERT INTO media_item(id,path,name,ext,size,mtime,added_at,updated_at) VALUES (1,'a.jpg','a.jpg','jpg',1,1,1,1)",[]).unwrap();
        // Un tag utente con lo stesso testo di un'etichetta AI resta comunque un tag.
        crate::bulk::add_tag(&c, "natura", &[1], &[]).unwrap();
        crate::bulk::add_tag(&c, "Vacanze 2023", &[1], &[]).unwrap();
        let groups = vec!["subject".into(), "environment".into()];
        let run = |value: &str, model: &str, taxonomy: i64| {
            let labels = vec![
                AutoLabel {
                    group: "subject".into(),
                    value: value.into(),
                    score: 0.9,
                },
                AutoLabel {
                    group: "environment".into(),
                    value: "montagna".into(),
                    score: 0.8,
                },
            ];
            classification(&c, 1, &groups, &labels, &[0.0; 512], model, taxonomy).unwrap();
        };
        run("natura", "v1", 1);
        // Rianalisi con un modello nuovo e un risultato diverso.
        run("ritratto", "v2", 2);
        let detail = crate::detail::media_detail(&c, 1).unwrap();
        assert_eq!(detail.tags, vec!["natura", "Vacanze 2023"]);
        let labels: Vec<(String, String)> = detail
            .labels
            .iter()
            .map(|l| (l.group.clone(), l.origin.clone()))
            .collect();
        assert_eq!(
            labels,
            vec![
                ("environment".to_string(), "auto".to_string()),
                ("subject".to_string(), "auto".to_string())
            ]
        );
        assert!(detail.labels.iter().all(|l| l.value != "Vacanze 2023"));
        assert_eq!(
            detail
                .labels
                .iter()
                .find(|l| l.group == "subject")
                .unwrap()
                .value,
            "ritratto"
        );
    }
}
