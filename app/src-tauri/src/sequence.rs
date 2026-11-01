//! Comandi dell'export della sequenza: anteprima dei nomi, verifica
//! preliminare e avvio del job.
use crate::state::AppState;
use pc_core::{AppError, AppResult};
use pc_organize::sequence::job::{
    self, ExportContext, ExportPreflight, NamePreview, SequenceExportOptions,
};
use pc_organize::sequence::naming::TemplateError;
use pc_store::app_store::{identities, tags as sets};
use pc_store::AppStore;
use serde::{Deserialize, Serialize};
use tauri::State;

const TEMPLATE_KEY: &str = "sequence_export_template";
pub const DEFAULT_TEMPLATE: &str = "{seq:04d}_{date}_{time}_{tag}";

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NamePreviewResult {
    pub names: Vec<NamePreview>,
    pub errors: Vec<TemplateError>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ExportDefaults {
    pub template: String,
    /// Set proposto per `{tag}` e per-tag: il primo esclusivo, altrimenti quello attivo.
    pub tag_set: Option<i64>,
}

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?
}

/// Dati di `app.db` per il job: tag del set scelto, parole chiave, identità.
pub fn context(app: &AppStore, tag_set: Option<i64>) -> AppResult<ExportContext> {
    Ok(app.with(|c| {
        let all = sets::list_sets(c)?;
        let set_tags = tag_set
            .and_then(|id| all.iter().find(|s| s.id == id))
            .map(|s| s.tag_names())
            .unwrap_or_default();
        let mut keyword_tags: Vec<String> = all
            .iter()
            .filter(|s| s.export_keywords)
            .flat_map(|s| s.tag_names())
            .collect();
        keyword_tags.sort_by_key(|t| t.to_lowercase());
        keyword_tags.dedup_by_key(|t| t.to_lowercase());
        let identities = identities::list(c)?
            .into_iter()
            .map(|i| (i.uid, (i.name, i.short_name)))
            .collect();
        Ok(ExportContext {
            set_tags,
            keyword_tags,
            identities,
        })
    })?)
}

#[tauri::command]
#[specta::specta]
pub fn sequence_export_defaults(state: State<'_, AppState>) -> AppResult<ExportDefaults> {
    let app = &state.app_store;
    let template = app
        .kv_get(TEMPLATE_KEY)?
        .unwrap_or_else(|| DEFAULT_TEMPLATE.to_string());
    let tag_set = app.with(|c| {
        let all = sets::list_sets(c)?;
        Ok(all
            .iter()
            .find(|s| s.exclusive && !s.tags.is_empty())
            .map(|s| s.id)
            .or(sets::active_set(c)?))
    })?;
    Ok(ExportDefaults { template, tag_set })
}

/// Nomi delle prime foto mentre si scrive il modello.
#[tauri::command]
#[specta::specta]
pub async fn sequence_export_preview(
    state: State<'_, AppState>,
    options: SequenceExportOptions,
    limit: u32,
) -> AppResult<NamePreviewResult> {
    let (db, app) = (state.store.clone(), state.app_store.clone());
    blocking(move || {
        let ctx = context(&app, options.tag_set)?;
        Ok(
            match job::preview_names(&db, &options, &ctx, limit.clamp(1, 50) as usize)? {
                Ok(names) => NamePreviewResult {
                    names,
                    errors: Vec::new(),
                },
                Err(errors) => NamePreviewResult {
                    names: Vec::new(),
                    errors,
                },
            },
        )
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn sequence_export_preflight(
    state: State<'_, AppState>,
    options: SequenceExportOptions,
) -> AppResult<ExportPreflight> {
    let (db, app) = (state.store.clone(), state.app_store.clone());
    blocking(move || {
        let ctx = context(&app, options.tag_set)?;
        job::preflight(&db, &options, &ctx)
    })
    .await
}

/// Avvia l'export come job annullabile e ricorda il modello usato.
#[tauri::command]
#[specta::specta]
pub async fn start_sequence_export(
    state: State<'_, AppState>,
    options: SequenceExportOptions,
) -> AppResult<i64> {
    let (db, app) = (state.store.clone(), state.app_store.clone());
    let pre_options = options.clone();
    let pre = blocking(move || {
        let ctx = context(&app, pre_options.tag_set)?;
        Ok((job::preflight(&db, &pre_options, &ctx)?, ctx))
    })
    .await?;
    let (check, ctx) = pre;
    if let Some(why) = check.blocked {
        return Err(AppError::invalid_input(why));
    }
    state.app_store.kv_set(TEMPLATE_KEY, &options.template)?;
    state
        .jobs
        .submit(job::JOB_KIND, job::job_params(&options, &ctx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_resolves_set_keywords_and_identities() {
        let app = AppStore::open_in_memory().unwrap();
        let usa = app
            .with(|c| {
                let id = sets::create_set(c, "USA", &["Paesaggi".into(), "Città".into()])?;
                sets::set_exclusive(c, id, true)?;
                sets::set_export_keywords(c, id, true)?;
                identities::create(
                    c,
                    &identities::IdentityInput {
                        name: "Giulia Bianchi".into(),
                        ..Default::default()
                    },
                )?;
                Ok(id)
            })
            .unwrap();
        let ctx = context(&app, Some(usa)).unwrap();
        assert_eq!(ctx.set_tags, vec!["Paesaggi", "Città"]);
        assert_eq!(ctx.keyword_tags, vec!["Città", "Paesaggi"]);
        let (name, short) = ctx.identities.values().next().unwrap();
        assert_eq!(
            (name.as_str(), short.as_str()),
            ("Giulia Bianchi", "Giulia")
        );
        assert!(context(&app, None).unwrap().set_tags.is_empty());
    }
}
