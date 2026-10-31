use crate::{commands, state::AppState};
use pc_core::{AppError, AppResult};
use pc_store::{
    errorlog::{self, ErrorQuery},
    repo,
};
use tauri::State;
#[tauri::command]
#[specta::specta]
pub async fn export_diagnostics(state: State<'_, AppState>, destination: String) -> AppResult<()> {
    let dest =
        pc_organize::paths::writable_destination(&state.store, std::path::Path::new(&destination))?;
    let config = state.config.read().clone();
    let logs = state.diagnostics.hub().tail();
    let models = state.model_store.statuses();
    let system = commands::get_resources(state.clone());
    let db = state.store.clone();
    tauri::async_runtime::spawn_blocking(move||{
        let errors=db.read(|c|errorlog::query(c,&ErrorQuery{limit:Some(5000),..Default::default()}))?;
        let jobs=pc_jobs::persist::recent(&db,500)?;
        // I parametri dei job non entrano: possono contenere selezioni o correzioni private.
        let jobs:Vec<_>=jobs.into_iter().map(|j|serde_json::json!({"id":j.id,"kind":j.kind,"state":j.state,"created_at":j.created_at})).collect();
        pc_diag::bundle::write(&dest,pc_diag::bundle::Bundle{config,logs,models:serde_json::to_value(models)?,system:serde_json::to_value(system)?,jobs:serde_json::json!(jobs),errors:serde_json::to_value(errors)?})
    }).await.map_err(|e|AppError::internal(e.to_string()))?
}
#[tauri::command]
#[specta::specta]
pub fn open_logs(state: State<'_, AppState>) -> AppResult<()> {
    let dir = state.paths.logs();
    let unavailable = |why: String| {
        AppError::new(
            pc_core::ErrorCode::Io,
            format!(
                "Impossibile aprire la cartella dei log ({why}). Si trova in: {}",
                dir.display()
            ),
        )
    };
    let opener = if cfg!(windows) {
        "explorer.exe"
    } else if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(unix) {
        "xdg-open"
    } else {
        return Err(unavailable("sistema operativo non supportato".into()));
    };
    let mut child = std::process::Command::new(opener)
        .arg(&dir)
        .spawn()
        .map_err(|e| unavailable(format!("{opener}: {e}")))?;
    // explorer.exe esce con codice 1 anche quando apre la cartella: si controlla solo altrove.
    // L'attesa avviene fuori dal thread del comando e un fallimento finisce in diagnostica.
    if !cfg!(windows) {
        std::thread::spawn(move || match child.wait() {
            Ok(status) if !status.success() => tracing::warn!(
                code = "IO",
                path = %dir.display(),
                ?status,
                "{opener} non ha aperto la cartella dei log: nessun file manager associato?"
            ),
            Err(e) => tracing::warn!(code = "IO", detail = %e, "attesa di {opener} fallita"),
            Ok(_) => {}
        });
    }
    Ok(())
}
/// Seleziona solo i percorsi falliti del job/fase, senza riscan completo.
#[tauri::command]
#[specta::specta]
pub async fn retry_failed(
    state: State<'_, AppState>,
    job_id: Option<i64>,
    phase: Option<String>,
) -> AppResult<Vec<i64>> {
    let db = state.store.clone();
    let groups = tauri::async_runtime::spawn_blocking(move || {
        db.read(|c| {
            let mut groups = Vec::new();
            for job in errorlog::failed_jobs(c, job_id, phase.as_deref())? {
                groups.push((job, errorlog::failed_paths(c, job, phase.as_deref())?));
            }
            Ok(groups)
        })
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))??;
    let mut created = Vec::new();
    for (job, paths) in groups {
        let Some(row) = pc_jobs::persist::get(&state.store, job)? else {
            continue;
        };
        let resolve = |path: &str| {
            state
                .store
                .read(|c| repo::media::get_by_path(c, path))
                .ok()
                .flatten()
                .map(|m| m.id)
        };
        if let Some(params) = retry_params(&row.kind, row.params, &paths, resolve) {
            created.push(state.jobs.submit(&row.kind, params)?);
        }
    }
    Ok(created)
}

/// Parametri del nuovo tentativo: il job originale ristretto ai soli file falliti.
/// `None` se il tipo di job non è ripetibile o nessun file è risolvibile.
fn retry_params(
    kind: &str,
    mut params: serde_json::Value,
    paths: &[String],
    resolve: impl Fn(&str) -> Option<i64>,
) -> Option<serde_json::Value> {
    if kind == pc_ingest::import::JOB_KIND {
        params["paths"] = serde_json::json!(paths);
        return Some(params);
    }
    let ids: Vec<i64> = paths
        .iter()
        .filter_map(|p| {
            p.strip_prefix("media:")
                .and_then(|s| s.parse().ok())
                .or_else(|| resolve(p))
        })
        .collect();
    if ids.is_empty() {
        return None;
    }
    match kind {
        pc_ai::analyze::JOB_KIND | pc_jobs::bulk::JOB_KIND => {
            params["ids"] = serde_json::json!(ids)
        }
        pc_organize::organize::JOB_KIND => {
            params["scope"] = serde_json::json!({"scope":"selection","ids":ids})
        }
        _ => return None,
    }
    Some(params)
}

#[cfg(test)]
mod tests {
    use super::retry_params;
    use serde_json::json;

    #[test]
    fn retry_restricts_each_job_kind_to_failed_files() {
        let paths = vec![
            r"D:\Foto\a.jpg".to_string(),
            "media:42".to_string(),
            r"D:\Foto\sparito.jpg".to_string(),
        ];
        let resolve = |p: &str| (p == r"D:\Foto\a.jpg").then_some(7);
        let import = retry_params(
            pc_ingest::import::JOB_KIND,
            json!({"sourceId":3}),
            &paths,
            resolve,
        )
        .unwrap();
        assert_eq!(import, json!({"sourceId":3,"paths":paths}));
        let ai = retry_params(pc_ai::analyze::JOB_KIND, json!({}), &paths, resolve).unwrap();
        assert_eq!(ai["ids"], json!([7, 42]));
        let org = retry_params(
            pc_organize::organize::JOB_KIND,
            json!({"method":"copy"}),
            &paths,
            resolve,
        )
        .unwrap();
        assert_eq!(org["scope"], json!({"scope":"selection","ids":[7,42]}));
        assert_eq!(org["method"], "copy");
        assert!(retry_params("demo", json!({}), &paths, resolve).is_none());
        assert!(retry_params(
            pc_ai::analyze::JOB_KIND,
            json!({}),
            &[r"D:\x.jpg".to_string()],
            |_| None
        )
        .is_none());
    }
}
