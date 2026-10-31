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

pub struct SourceWatcher {
    _debouncer: Debouncer<RecommendedWatcher>,
    sources: Vec<(i64, PathBuf)>,
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
        for (id, path, recursive) in &sources {
            let mode = if *recursive {
                RecursiveMode::Recursive
            } else {
                RecursiveMode::NonRecursive
            };
            if let Err(e) = debouncer.watcher().watch(path, mode) {
                tracing::warn!(source_id = id, code = "IO", path = %path.display(), detail = %e, "impossibile monitorare la sorgente");
            }
        }
        Ok(Self {
            _debouncer: debouncer,
            sources: roots.to_vec(),
        })
    }

    pub fn watched(&self) -> Vec<i64> {
        self.sources.iter().map(|(id, _)| *id).collect()
    }
}
