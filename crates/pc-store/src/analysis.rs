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

pub fn people(c: &Connection, id: i64, boxes: &[[f32; 5]], model: &str) -> StoreResult<()> {
    if boxes
        .iter()
        .flatten()
        .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
    {
        return Err(StoreError::Invalid("riquadri persone non validi".into()));
    }
    c.execute("INSERT INTO media_people(media_id,count,boxes,model_ver) VALUES (?1,?2,?3,?4) ON CONFLICT(media_id) DO UPDATE SET count=excluded.count,boxes=excluded.boxes,model_ver=excluded.model_ver",params![id,boxes.len() as i64,serde_json::to_string(boxes)?,model])?;
    version(c, id, "persons", model, None)
}

pub fn pending(
    c: &Connection,
    ids: &[i64],
    stage: &str,
    model: &str,
    taxonomy: Option<i64>,
) -> StoreResult<Vec<i64>> {
    let mut q=c.prepare_cached("SELECT EXISTS(SELECT 1 FROM analysis_version WHERE media_id=?1 AND stage=?2 AND model_ver=?3 AND taxonomy_ver IS ?4)")?;
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
    Ok(c.query_row("SELECT count(*) FROM media_item m WHERE m.status='ok' AND m.thumb IS NOT NULL
        AND NOT EXISTS(SELECT 1 FROM analysis_version a WHERE a.media_id=m.id AND a.stage=?1 AND a.model_ver=?2 AND a.taxonomy_ver IS ?3)",params![stage,model,taxonomy],|r|r.get(0))?)
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
        crate::bulk::add_tag(&c, "natura", &[1]).unwrap();
        crate::bulk::add_tag(&c, "Vacanze 2023", &[1]).unwrap();
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
