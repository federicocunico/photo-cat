use crate::state::AppState;
use pc_ai::retune::{self, ClipInputs, RetuneSpec, ThresholdImpact};
use pc_ai::taxonomy::{self, Taxonomy};
use pc_core::{AiThresholds, AppError, AppResult, Config};
use pc_jobs::{JobControl, JobManager};
use tauri::State;

/// Fasi di rivalutazione richieste da un cambio di impostazioni. La soglia
/// globale `confidence_threshold` è solo il predefinito dei gruppi nuovi: le
/// soglie dei gruppi della tassonomia non vengono mai riscritte da qui.
pub(crate) fn retune_after_settings(previous: &Config, next: &Config) -> RetuneSpec {
    RetuneSpec::between(&previous.ai.thresholds, &next.ai.thresholds)
}

/// Fasi di rivalutazione richieste da un salvataggio della tassonomia: solo se
/// cambia l'analisi automatica (soglie, voci automatiche attive e loro prompt).
/// Le catalogazioni si ricalcolano dagli embedding salvati, senza rieseguire il
/// modello immagine. Voci e gruppi manuali e nomi non richiedono nulla.
pub(crate) fn retune_after_taxonomy(previous: &Taxonomy, next: &Taxonomy) -> RetuneSpec {
    RetuneSpec {
        clip: previous.changes_analysis(next),
        ..Default::default()
    }
}

/// Accoda il job di rivalutazione se ci sono fasi da eseguire.
pub(crate) fn submit_retune(jobs: &JobManager, spec: RetuneSpec) -> AppResult<Option<i64>> {
    if spec.is_empty() {
        return Ok(None);
    }
    jobs.submit(retune::JOB_KIND, spec.params()).map(Some)
}

#[tauri::command]
#[specta::specta]
pub async fn save_settings(state: State<'_, AppState>, config: Config) -> AppResult<()> {
    config.validate()?;
    crate::open_with::validate_apps(&config)?;
    if let Some(path) = &config.data_dir {
        pc_organize::paths::writable_destination(&state.store, path)?;
    }
    let previous = state.config.read().clone();
    state.save_config(config.clone())?;
    // Dopo il salvataggio: il job legge le soglie nuove dalla configurazione.
    submit_retune(&state.jobs, retune_after_settings(&previous, &config))?;
    state.diagnostics.set_level(config.logging.level)?;
    if previous.ai.execution_provider != config.ai.execution_provider {
        let mut options = state.session_opts.lock().clone();
        options.preferred = config.ai.execution_provider;
        state.model_pool.set_options(options.clone());
        *state.session_opts.lock() = options;
    }
    if previous.resources != config.resources {
        state.model_pool.evict_idle("");
        pc_governor::apply_resources(&state.governor, state.probe.as_ref(), &config.resources);
    }
    Ok(())
}
#[tauri::command]
#[specta::specta]
pub async fn get_taxonomy(state: State<'_, AppState>) -> AppResult<Taxonomy> {
    let db = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || taxonomy::load(&db))
        .await
        .map_err(|e| AppError::internal(e.to_string()))?
}
#[tauri::command]
#[specta::specta]
pub async fn save_taxonomy(
    state: State<'_, AppState>,
    mut taxonomy: Taxonomy,
    expected_version: i64,
) -> AppResult<()> {
    taxonomy.version = expected_version
        .checked_add(1)
        .ok_or_else(|| AppError::invalid_input("versione tassonomia esaurita"))?;
    let db = state.store.clone();
    let spec = tauri::async_runtime::spawn_blocking(move || {
        let previous = taxonomy::load(&db)?;
        let spec = retune_after_taxonomy(&previous, &taxonomy);
        taxonomy::save(&db, taxonomy, expected_version)?;
        AppResult::Ok(spec)
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))??;
    submit_retune(&state.jobs, spec)?;
    Ok(())
}

/// Rianalisi dei file salvati con un minimo più alto delle soglie in vigore
/// (scenario "Soglia sotto il minimo salvato"). Restituisce l'id del job, o
/// `None` se non ci sono file da rianalizzare.
#[tauri::command]
#[specta::specta]
pub async fn reanalyze_below_floor(state: State<'_, AppState>) -> AppResult<Option<i64>> {
    let (db, t) = (state.store.clone(), state.config.read().ai.thresholds);
    let ids = tauri::async_runtime::spawn_blocking(move || retune::prepare_reanalysis(&db, &t))
        .await
        .map_err(|e| AppError::internal(e.to_string()))??;
    if ids.is_empty() {
        return Ok(None);
    }
    state
        .jobs
        .submit(pc_ai::analyze::JOB_KIND, serde_json::json!({ "ids": ids }))
        .map(Some)
}

/// Effetto di nuove soglie (e, se indicata, di una tassonomia con soglie
/// diverse) calcolato dai risultati salvati, senza scrivere.
#[tauri::command]
#[specta::specta]
pub async fn preview_thresholds(
    state: State<'_, AppState>,
    thresholds: AiThresholds,
    taxonomy: Option<Taxonomy>,
) -> AppResult<ThresholdImpact> {
    let mut check = state.config.read().clone();
    let current = check.ai.thresholds;
    check.ai.thresholds = thresholds;
    check.validate()?;
    if let Some(t) = &taxonomy {
        t.validate()?;
    }
    let deps = state.ai_deps.clone();
    tauri::async_runtime::spawn_blocking(move || {
        // I concetti testuali sono quelli correnti (dalla cache); della
        // tassonomia proposta contano solo le soglie.
        let ctx = match &taxonomy {
            Some(_) => retune::clip_context(&deps)?,
            None => None,
        };
        let inputs = match (&taxonomy, &ctx) {
            (Some(t), Some((_, concepts, model))) => Some(ClipInputs {
                taxonomy: t,
                concepts,
                model,
            }),
            _ => None,
        };
        retune::preview(
            &deps.store,
            inputs.as_ref(),
            &current,
            &thresholds,
            &JobControl::new(),
        )
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))?
}
#[tauri::command]
#[specta::specta]
pub fn onboarding_done(state: State<'_, AppState>) -> AppResult<bool> {
    Ok(state.store.read(|c| {
        Ok(c.query_row(
            "SELECT EXISTS(SELECT 1 FROM settings_kv WHERE key='onboarding.done' AND value='true')",
            [],
            |r| r.get(0),
        )?)
    })?)
}
#[tauri::command]
#[specta::specta]
pub fn complete_onboarding(state: State<'_, AppState>) -> AppResult<()> {
    state.store.write(|c|{c.execute("INSERT INTO settings_kv(key,value) VALUES ('onboarding.done','true') ON CONFLICT(key) DO UPDATE SET value='true'",[])?;Ok(())})?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pc_governor::MemoryGovernor;
    use pc_jobs::{JobPlan, JobSnapshot, PhaseSpec, ProgressSink, RunSummary};
    use pc_store::Store;
    use std::sync::Arc;

    struct Sink;
    impl ProgressSink for Sink {
        fn on_progress(&self, _: &JobSnapshot) {}
    }

    fn jobs(db: &Store) -> JobManager {
        let jobs = JobManager::new(db.clone(), MemoryGovernor::new(1024), 1, Arc::new(Sink));
        jobs.register(
            retune::JOB_KIND,
            Arc::new(|_| {
                Ok(JobPlan {
                    phases: vec![PhaseSpec::new("clip", 1.0)],
                    min_memory_mb: 1,
                    run: Box::new(|_| {
                        Ok(RunSummary {
                            cancelled: false,
                            failed: 0,
                        })
                    }),
                })
            }),
        );
        jobs
    }

    /// Scenario "Soglie di gruppo preservate".
    #[test]
    fn group_thresholds_survive_other_settings() {
        let tmp = tempfile::tempdir().unwrap();
        let db = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
        let mut tax = taxonomy::load(&db).unwrap();
        tax.version = 2;
        tax.groups[0].threshold = 0.6;
        tax.groups[1].threshold = 0.1;
        let tax = taxonomy::save(&db, tax, 1).unwrap();
        let previous = Config::default();
        let mut next = previous.clone();
        next.ai.confidence_threshold = 0.4;
        next.ui.theme = pc_core::config::Theme::Light;
        let jobs = jobs(&db);
        assert_eq!(
            submit_retune(&jobs, retune_after_settings(&previous, &next)).unwrap(),
            None
        );
        assert_eq!(taxonomy::load(&db).unwrap(), tax);
        assert!(jobs.snapshots().is_empty());
    }

    #[test]
    fn saving_a_threshold_queues_a_retune_job() {
        let tmp = tempfile::tempdir().unwrap();
        let db = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
        let jobs = jobs(&db);
        let previous = Config::default();
        let mut next = previous.clone();
        next.ai.thresholds.person_detection = 0.6;
        let spec = retune_after_settings(&previous, &next);
        assert!(spec.persons && !spec.faces && !spec.clip);
        let id = submit_retune(&jobs, spec).unwrap().expect("job accodato");
        let snap = jobs.snapshot(id).expect("job nella schermata Job");
        assert_eq!(snap.kind, retune::JOB_KIND);

        // Soglia di una categoria della tassonomia: fase CLIP.
        let before = Taxonomy::default();
        let mut after = before.clone();
        after.version += 1;
        assert!(retune_after_taxonomy(&before, &after).is_empty());
        after.groups[2].categories[0].threshold = Some(0.6);
        assert!(retune_after_taxonomy(&before, &after).clip);
        assert!(jobs.wait_idle(std::time::Duration::from_secs(10)));
    }
}
