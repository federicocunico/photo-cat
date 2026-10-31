//! Passate per modello quando tutte le sessioni non possono restare residenti.
use pc_core::{AppError, AppResult};
use pc_governor::MemoryGovernor;
use pc_jobs::JobControl;
use pc_models::pool::{LoadedModel, ModelPool};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ModelPass {
    pub phase: String,
    pub ids: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelEvent {
    Loaded(String),
    Unloaded(String),
}

pub fn needs_model_major(pool: &ModelPool, passes: &[ModelPass]) -> bool {
    !pool.fit_together(
        &passes
            .iter()
            .flat_map(|p| p.ids.iter().map(String::as_str))
            .collect::<Vec<_>>(),
    )
}

fn release(pool: &ModelPool, event: &mut impl FnMut(ModelEvent)) {
    let before = pool.loaded_ids();
    pool.evict_idle("");
    let after = pool.loaded_ids();
    for id in before {
        if !after.contains(&id) {
            event(ModelEvent::Unloaded(id));
        }
    }
}

pub fn run_model_major(
    pool: &ModelPool,
    governor: &MemoryGovernor,
    passes: &[ModelPass],
    control: &JobControl,
    mut work: impl FnMut(&ModelPass, &[Arc<LoadedModel>]) -> AppResult<()>,
    mut event: impl FnMut(ModelEvent),
) -> AppResult<()> {
    release(pool, &mut event);
    for pass in passes {
        if !control.checkpoint() {
            return Err(AppError::cancelled());
        }
        while governor.ai_paused() {
            if !control.checkpoint() {
                return Err(AppError::cancelled());
            }
            governor.wait_ai_resumed(std::time::Duration::from_millis(100));
        }
        let result = (|| {
            let mut loaded = Vec::new();
            for id in &pass.ids {
                loaded.push(pool.get(id)?);
                tracing::info!(
                    model = id.as_str(),
                    phase = pass.phase.as_str(),
                    "passata modello: sessione caricata"
                );
                event(ModelEvent::Loaded(id.clone()));
            }
            work(pass, &loaded)
        })();
        // I riferimenti della passata sono già rilasciati prima di liberare il budget.
        release(pool, &mut event);
        result?;
    }
    Ok(())
}
