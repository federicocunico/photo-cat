//! Persistenza di job, fasi e stato per elemento nel catalogo.

use crate::pipeline::{ItemState, Journal};
use crate::progress::{JobProgress, PhaseSpec};
use pc_store::{now_ms, Store, StoreResult};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum JobState {
    Queued,
    Running,
    Paused,
    Completed,
    CompletedWithErrors,
    Cancelled,
    Interrupted,
    Failed,
}

impl JobState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::CompletedWithErrors => "completed_with_errors",
            Self::Cancelled => "cancelled",
            Self::Interrupted => "interrupted",
            Self::Failed => "failed",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "queued" => Self::Queued,
            "running" => Self::Running,
            "paused" => Self::Paused,
            "completed" => Self::Completed,
            "completed_with_errors" => Self::CompletedWithErrors,
            "cancelled" => Self::Cancelled,
            "failed" => Self::Failed,
            _ => Self::Interrupted,
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed
                | Self::CompletedWithErrors
                | Self::Cancelled
                | Self::Failed
                | Self::Interrupted
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct JobRow {
    pub id: i64,
    pub kind: String,
    pub state: JobState,
    pub params: serde_json::Value,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
}

pub fn create(
    store: &Store,
    kind: &str,
    params: &serde_json::Value,
    phases: &[PhaseSpec],
) -> StoreResult<i64> {
    let kind = kind.to_string();
    let params = params.to_string();
    let phases: Vec<String> = phases.iter().map(|p| p.name.clone()).collect();
    store.write(move |c| {
        c.execute(
            "INSERT INTO job(kind, state, params, created_at) VALUES (?1, 'queued', ?2, ?3)",
            params![kind, params, now_ms()],
        )?;
        let id = c.last_insert_rowid();
        for (i, p) in phases.iter().enumerate() {
            c.execute(
                "INSERT INTO job_phase(job_id, phase, ord) VALUES (?1, ?2, ?3)",
                params![id, p, i as i64],
            )?;
        }
        Ok(id)
    })
}

/// Reimposta le fasi di un job ripreso (contatori azzerati: il journal ricostruisce l'avanzamento).
pub fn reset_phases(store: &Store, id: i64, phases: &[PhaseSpec]) -> StoreResult<()> {
    let phases: Vec<String> = phases.iter().map(|p| p.name.clone()).collect();
    store.write(move |c| {
        c.execute("DELETE FROM job_phase WHERE job_id = ?1", [id])?;
        for (i, p) in phases.iter().enumerate() {
            c.execute(
                "INSERT INTO job_phase(job_id, phase, ord) VALUES (?1, ?2, ?3)",
                params![id, p, i as i64],
            )?;
        }
        Ok(())
    })
}

pub fn set_state(store: &Store, id: i64, state: JobState) -> StoreResult<()> {
    store.write(move |c| {
        let now = now_ms();
        match state {
            JobState::Running => c.execute(
                "UPDATE job SET state = ?2, started_at = COALESCE(started_at, ?3), finished_at = NULL WHERE id = ?1",
                params![id, state.as_str(), now],
            )?,
            s if s.is_terminal() => c.execute(
                "UPDATE job SET state = ?2, finished_at = ?3 WHERE id = ?1",
                params![id, state.as_str(), now],
            )?,
            _ => c.execute("UPDATE job SET state = ?2 WHERE id = ?1", params![id, state.as_str()])?,
        };
        Ok(())
    })
}

/// Salva i contatori delle fasi (scrittura in coda, non bloccante).
pub fn save_progress(store: &Store, id: i64, progress: &JobProgress) -> StoreResult<()> {
    let rows: Vec<_> = progress
        .phases()
        .iter()
        .map(|p| {
            (
                p.name().to_string(),
                p.done(),
                p.failed(),
                p.total(),
                p.total_known(),
            )
        })
        .collect();
    store.submit(move |c| {
        let mut st = c.prepare_cached(
            "UPDATE job_phase SET done = ?3, failed = ?4, total = ?5, total_known = ?6 WHERE job_id = ?1 AND phase = ?2",
        )?;
        for (name, done, failed, total, known) in rows {
            st.execute(params![id, name, done, failed, total, known])?;
        }
        Ok(())
    })
}

fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<JobRow> {
    let params: String = r.get(3)?;
    Ok(JobRow {
        id: r.get(0)?,
        kind: r.get(1)?,
        state: JobState::parse(&r.get::<_, String>(2)?),
        params: serde_json::from_str(&params).unwrap_or(serde_json::Value::Null),
        created_at: r.get(4)?,
        started_at: r.get(5)?,
        finished_at: r.get(6)?,
    })
}

const COLS: &str = "id, kind, state, params, created_at, started_at, finished_at";

pub fn get(store: &Store, id: i64) -> StoreResult<Option<JobRow>> {
    store.read(|c| {
        Ok(
            c.query_row(&format!("SELECT {COLS} FROM job WHERE id = ?1"), [id], row)
                .optional()?,
        )
    })
}

pub fn recent(store: &Store, limit: u32) -> StoreResult<Vec<JobRow>> {
    store.read(|c| {
        let mut st = c.prepare(&format!("SELECT {COLS} FROM job ORDER BY id DESC LIMIT ?1"))?;
        let rows = st.query_map([limit], row)?.collect::<Result<_, _>>()?;
        Ok(rows)
    })
}

pub type PhaseRow = (String, i64, i64, i64, bool);

/// Contatori persistiti delle fasi: (fase, fatti, falliti, totale, totale noto).
pub fn phases(store: &Store, id: i64) -> StoreResult<Vec<PhaseRow>> {
    store.read(|c| {
        let mut st = c.prepare(
            "SELECT phase, done, failed, total, total_known FROM job_phase WHERE job_id = ?1 ORDER BY ord",
        )?;
        let rows = st
            .query_map([id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    })
}

/// All'avvio: i job rimasti attivi (chiusura inattesa) diventano "interrotti".
pub fn mark_interrupted(store: &Store) -> StoreResult<Vec<JobRow>> {
    let ids: Vec<i64> = store.write(|c| {
        let mut st =
            c.prepare("SELECT id FROM job WHERE state IN ('queued', 'running', 'paused')")?;
        let ids: Vec<i64> = st.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
        c.execute(
            "UPDATE job SET state = 'interrupted' WHERE state IN ('queued', 'running', 'paused')",
            [],
        )?;
        Ok(ids)
    })?;
    let mut rows = Vec::new();
    for id in ids {
        if let Some(r) = get(store, id)? {
            tracing::warn!(
                job_id = id,
                kind = r.kind.as_str(),
                "job interrotto nella sessione precedente"
            );
            rows.push(r);
        }
    }
    Ok(rows)
}

/// Journal su `item_stage`: un elemento è concluso se è fallito in una fase,
/// se ha completato l'ultima fase o se è stato saltato (`*`).
pub struct StoreJournal {
    store: Store,
    job_id: i64,
    finished: HashSet<String>,
}

impl StoreJournal {
    pub fn load(store: Store, job_id: i64, last_phase: &str) -> StoreResult<Self> {
        let last = last_phase.to_string();
        let finished = store.read(move |c| {
            let mut st = c.prepare(
                "SELECT DISTINCT item FROM item_stage
                 WHERE job_id = ?1 AND (state = 'failed' OR phase = ?2 OR phase = '*')",
            )?;
            let set = st
                .query_map(params![job_id, last], |r| r.get(0))?
                .collect::<Result<_, _>>()?;
            Ok(set)
        })?;
        Ok(Self {
            store,
            job_id,
            finished,
        })
    }

    pub fn finished_count(&self) -> usize {
        self.finished.len()
    }
}

impl Journal for StoreJournal {
    fn is_finished(&self, key: &str) -> bool {
        self.finished.contains(key)
    }

    fn record(&self, key: &str, phase: &str, state: ItemState) {
        let (job, key, phase) = (self.job_id, key.to_string(), phase.to_string());
        let _ = self.store.submit(move |c| {
            c.execute(
                "INSERT OR REPLACE INTO item_stage(job_id, item, phase, state) VALUES (?1, ?2, ?3, ?4)",
                params![job, key, phase, state.as_str()],
            )?;
            Ok(())
        });
    }
}
