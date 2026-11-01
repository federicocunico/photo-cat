//! Chinese Whispers su vicini kNN/HNSW, con identità manuali e rifiuti persistenti.
use crate::faces::SAME_PERSON;
use hnsw_rs::prelude::*;
use pc_core::{AppError, AppResult, ErrorCode};
use pc_governor::MemoryGovernor;
use pc_jobs::JobControl;
use pc_store::{
    people::{self, FaceRecord},
    Store,
};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Label {
    Person(i64),
    Seed(i64),
}

pub fn chinese_whispers(
    nodes: &[FaceRecord],
    named: &HashSet<i64>,
    threshold: f32,
    control: &JobControl,
) -> AppResult<Vec<Label>> {
    let mut protected = named.clone();
    for f in nodes {
        if f.manual {
            protected.extend(f.person_id);
        }
        protected.extend(f.rejected.iter().copied());
    }
    let fixed: Vec<_> = nodes
        .iter()
        .map(|f| f.person_id.filter(|p| protected.contains(p)))
        .collect();
    let mut labels: Vec<_> = nodes
        .iter()
        .zip(&fixed)
        .map(|(f, p)| p.map(Label::Person).unwrap_or(Label::Seed(f.id)))
        .collect();
    let mut groups: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (i, n) in nodes.iter().enumerate() {
        if n.embedding.is_some() {
            groups.entry(&n.model).or_default().push(i);
        }
    }
    let mut graph = vec![Vec::<(usize, f32)>::new(); nodes.len()];
    for ids in groups.values() {
        if !control.checkpoint() {
            return Err(AppError::cancelled());
        }
        let index = if ids.len() > 64 {
            let h = Hnsw::<f32, DistCosine>::new(16, ids.len(), 16, 100, DistCosine {});
            for i in ids {
                h.insert((
                    nodes[*i].embedding.as_ref().expect("embedding").as_slice(),
                    *i,
                ));
            }
            Some(h)
        } else {
            None
        };
        for i in ids {
            if !control.checkpoint() {
                return Err(AppError::cancelled());
            }
            let v = nodes[*i].embedding.as_ref().expect("embedding");
            let neighbours: Vec<_> = match &index {
                Some(h) => h
                    .search(v, 17.min(ids.len()), 50)
                    .into_iter()
                    .map(|n| n.d_id)
                    .collect(),
                None => ids.clone(),
            };
            let mut edges: Vec<_> = neighbours
                .into_iter()
                .filter(|j| j != i)
                .filter_map(|j| {
                    let score = people::cosine(v, nodes[j].embedding.as_ref()?);
                    (score >= threshold).then_some((j, score))
                })
                .collect();
            edges.sort_by(|a, b| b.1.total_cmp(&a.1));
            edges.truncate(16);
            graph[*i] = edges;
        }
    }
    for iteration in 0..30 {
        let mut changed = false;
        for step in 0..nodes.len() {
            if !control.checkpoint() {
                return Err(AppError::cancelled());
            }
            let i = if iteration % 2 == 0 {
                step
            } else {
                nodes.len() - 1 - step
            };
            if fixed[i].is_some() {
                continue;
            }
            let mut votes: BTreeMap<Label, f32> = BTreeMap::new();
            for (j, score) in &graph[i] {
                let label = labels[*j];
                if let Label::Person(p) = label {
                    if nodes[i].rejected.contains(&p) {
                        continue;
                    }
                }
                *votes.entry(label).or_default() += score;
            }
            let prior = labels[i];
            let selected = votes
                .into_iter()
                .max_by(|a, b| {
                    a.1.total_cmp(&b.1)
                        .then_with(|| (a.0 == prior).cmp(&(b.0 == prior)))
                })
                .map(|(l, _)| l)
                .unwrap_or(prior);
            changed |= selected != prior;
            labels[i] = selected;
        }
        if !changed {
            break;
        }
    }
    Ok(labels)
}

pub fn run(
    store: &Store,
    governor: &MemoryGovernor,
    control: &JobControl,
    policy: std::sync::Arc<crate::face_policy::FacePolicy>,
    refs: &people::IdentityRefs,
) -> AppResult<()> {
    let Some(generation) = policy.token() else {
        return Ok(());
    };
    let count: i64 = store.read(|c| {
        Ok(c.query_row(
            "SELECT count(*) FROM face WHERE embedding IS NOT NULL",
            [],
            |r| r.get(0),
        )?)
    })?;
    if count == 0 {
        return Ok(());
    }
    let cost = (count as u64)
        .saturating_mul(4096)
        .div_ceil(1024 * 1024)
        .saturating_add(16);
    if cost > governor.capacity_mb() {
        return Err(AppError::new(
            ErrorCode::InsufficientMemory,
            "memoria insufficiente per il raggruppamento volti",
        ));
    }
    let _permit = governor.try_acquire(cost).ok_or_else(|| {
        AppError::new(
            ErrorCode::InsufficientMemory,
            "attendere memoria libera per raggruppare i volti",
        )
    })?;
    let (nodes, named) = store.read(|c| {
        let mut nodes: Vec<_> = people::records(c, None)?
            .into_iter()
            .filter(|f| f.embedding.is_some())
            .collect();
        for f in &mut nodes {
            if !f.manual {
                // Lettura: un'identità non collegata non crea persone qui (lo fa il salvataggio).
                if let Some(p) = people::named_match_refs(
                    c,
                    f.embedding.as_ref().expect("embedding"),
                    &f.model,
                    &f.rejected,
                    SAME_PERSON,
                    refs,
                    false,
                )? {
                    f.person_id = Some(p);
                }
            }
        }
        Ok((nodes, people::named_ids(c)?))
    })?;
    let labels = chinese_whispers(&nodes, &named, SAME_PERSON, control)?;
    if !control.checkpoint() {
        return Err(AppError::cancelled());
    }
    store.write(move |c| {
        if !policy.is_current(generation) {
            return Ok(());
        }
        let mut assignments: HashMap<Label, Vec<usize>> = HashMap::new();
        for (i, label) in labels.iter().enumerate() {
            assignments.entry(*label).or_default().push(i);
        }
        let mut used: HashSet<i64> = labels
            .iter()
            .filter_map(|l| {
                if let Label::Person(p) = l {
                    Some(*p)
                } else {
                    None
                }
            })
            .collect();
        let mut ordered: Vec<_> = assignments.into_iter().collect();
        ordered.sort_by_key(|(label, _)| *label);
        for (label, members) in ordered {
            let person = match label {
                Label::Person(id) => id,
                Label::Seed(_) => {
                    let mut candidates: BTreeMap<i64, usize> = BTreeMap::new();
                    for i in &members {
                        if let Some(p) = nodes[*i].person_id {
                            if !used.contains(&p)
                                && members.iter().all(|j| !nodes[*j].rejected.contains(&p))
                            {
                                *candidates.entry(p).or_default() += 1;
                            }
                        }
                    }
                    match candidates.into_iter().max_by_key(|(_, n)| *n) {
                        Some((p, _)) => p,
                        None => people::create_person(c, None)?,
                    }
                }
            };
            used.insert(person);
            for i in members {
                let f = &nodes[i];
                if !f.manual {
                    c.execute(
                        "UPDATE face SET person_id=?2,assigned_by='auto' WHERE id=?1",
                        rusqlite::params![f.id, person],
                    )?;
                }
            }
        }
        people::refresh_centroids(c)?;
        for id in nodes.iter().map(|f| f.media_id).collect::<HashSet<_>>() {
            pc_store::repo::fts::refresh(c, id)?;
        }
        Ok(())
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn node(
        id: i64,
        person: Option<i64>,
        manual: bool,
        rejected: Vec<i64>,
        axis: usize,
    ) -> FaceRecord {
        let mut v = vec![0.0; 128];
        v[axis] = 1.0;
        FaceRecord {
            id,
            media_id: id,
            bbox: [0.1, 0.1, 0.2, 0.2],
            quality: 0.99,
            embedding: Some(v),
            person_id: person,
            manual,
            rejected,
            model: "sface".into(),
        }
    }
    #[test]
    fn identities_and_refusals_survive_reclustering() {
        let nodes = vec![
            node(1, Some(10), true, vec![], 0),
            node(2, None, false, vec![], 0),
            node(3, None, false, vec![10], 0),
            node(4, Some(20), true, vec![], 0),
            node(5, None, false, vec![], 1),
            node(6, None, false, vec![], 1),
        ];
        let labels = chinese_whispers(
            &nodes,
            &HashSet::from([10]),
            SAME_PERSON,
            &JobControl::new(),
        )
        .unwrap();
        assert_eq!(labels[0], Label::Person(10));
        assert_eq!(labels[3], Label::Person(20));
        assert_ne!(labels[2], Label::Person(10));
        assert_eq!(labels[4], labels[5]);
        assert_ne!(labels[4], labels[0]);
    }
    #[test]
    fn persisted_corrections_survive_multiple_clustering_runs() {
        let tmp = tempfile::tempdir().unwrap();
        let db = Store::open(&tmp.path().join("test.db"), Default::default()).unwrap();
        db.write(|c|{for id in 1..=4{
            c.execute("INSERT INTO media_item(id,path,name,ext,size,mtime,added_at,updated_at) VALUES (?1,?2,'test','jpg',1,1,1,1)",rusqlite::params![id,format!("test{id}.jpg")])?;
            let mut embedding=vec![0.0;128];embedding[0]=1.0;
            people::save_faces(c,id,&[people::DetectedFace{bbox:[0.1,0.1,0.2,0.2],quality:1.0,embedding}],"sface","det+sface",SAME_PERSON)?;
        }Ok(())}).unwrap();
        let policy = std::sync::Arc::new(crate::face_policy::FacePolicy::new(true));
        let gov = MemoryGovernor::new(256);
        run(
            &db,
            &gov,
            &JobControl::new(),
            policy.clone(),
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
}
