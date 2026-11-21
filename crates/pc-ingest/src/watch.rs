//! Monitoraggio delle sorgenti: alle modifiche di una cartella monitorata
//! segnala la sorgente da reimportare (in modo incrementale).

use crate::formats::support;
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, Debouncer};
use pc_core::{AppError, AppResult, ErrorCode};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

/// Stato del monitoraggio di una sorgente.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum WatchState {
    /// La cartella è sorvegliata.
    Active,
    /// La cartella non è raggiungibile (es. disco scollegato).
    Unreachable,
    /// Il sistema ha rifiutato la sorveglianza (vedi `message`).
    Error,
}

/// Stato del monitoraggio di una sorgente con l'eventuale messaggio d'errore.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct WatchStatus {
    pub source_id: i64,
    pub state: WatchState,
    pub message: Option<String>,
}

pub struct SourceWatcher {
    _debouncer: Debouncer<RecommendedWatcher>,
    sources: Vec<(i64, PathBuf)>,
    statuses: Vec<WatchStatus>,
}

impl SourceWatcher {
    /// Avvia il monitoraggio; `on_change(source_id)` è chiamato al più una volta
    /// per sorgente per ogni raffica di modifiche (dopo `debounce`).
    pub fn start(
        sources: Vec<(i64, PathBuf, bool)>,
        debounce: Duration,
        on_change: impl Fn(i64) + Send + Sync + 'static,
    ) -> AppResult<Self> {
        let roots: Arc<Vec<(i64, PathBuf)>> =
            Arc::new(sources.iter().map(|(id, p, _)| (*id, p.clone())).collect());
        let r2 = roots.clone();
        let mut debouncer = new_debouncer(debounce, move |res: DebounceEventResult| match res {
            Ok(events) => {
                let mut changed = BTreeSet::new();
                for ev in events {
                    let relevant = ev
                        .path
                        .extension()
                        .and_then(|e| e.to_str())
                        .is_some_and(|e| support(e).is_some())
                        || ev.path.extension().is_none(); // cartelle spostate o rinominate
                    if !relevant {
                        continue;
                    }
                    if let Some((id, _)) = r2.iter().find(|(_, root)| ev.path.starts_with(root)) {
                        changed.insert(*id);
                    }
                }
                for id in changed {
                    tracing::info!(
                        source_id = id,
                        "modifiche rilevate nella sorgente monitorata"
                    );
                    on_change(id);
                }
            }
            Err(e) => tracing::warn!(code = "IO", detail = %e, "errore del monitoraggio cartelle"),
        })
        .map_err(|e| AppError::new(ErrorCode::Io, e.to_string()))?;
        let mut statuses = Vec::with_capacity(sources.len());
        for (id, path, recursive) in &sources {
            // Una sorgente non raggiungibile non impedisce il monitoraggio delle altre.
            if !path.is_dir() {
                tracing::warn!(source_id = id, code = "IO", path = %path.display(), "cartella non raggiungibile, monitoraggio non avviato");
                statuses.push(WatchStatus {
                    source_id: *id,
                    state: WatchState::Unreachable,
                    message: None,
                });
                continue;
            }
            let mode = if *recursive {
                RecursiveMode::Recursive
            } else {
                RecursiveMode::NonRecursive
            };
            match debouncer.watcher().watch(path, mode) {
                Ok(()) => statuses.push(WatchStatus {
                    source_id: *id,
                    state: WatchState::Active,
                    message: None,
                }),
                Err(e) => {
                    tracing::warn!(source_id = id, code = "IO", path = %path.display(), detail = %e, "impossibile monitorare la sorgente");
                    statuses.push(WatchStatus {
                        source_id: *id,
                        state: WatchState::Error,
                        message: Some(e.to_string()),
                    });
                }
            }
        }
        Ok(Self {
            _debouncer: debouncer,
            sources: roots.to_vec(),
            statuses,
        })
    }

    /// Stato di ogni sorgente passata a `start`.
    pub fn statuses(&self) -> &[WatchStatus] {
        &self.statuses
    }

    pub fn watched(&self) -> Vec<i64> {
        self.sources.iter().map(|(id, _)| *id).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_folder_is_unreachable_without_blocking_others() {
        let tmp = tempfile::tempdir().unwrap();
        let ok = tmp.path().join("presente");
        std::fs::create_dir(&ok).unwrap();
        let w = SourceWatcher::start(
            vec![(1, tmp.path().join("assente"), true), (2, ok, true)],
            Duration::from_millis(100),
            |_| {},
        )
        .unwrap();
        let st = w.statuses();
        assert_eq!(st.len(), 2);
        assert_eq!(st[0].source_id, 1);
        assert_eq!(st[0].state, WatchState::Unreachable);
        assert_eq!(st[1].source_id, 2);
        assert_eq!(st[1].state, WatchState::Active);
    }
}
