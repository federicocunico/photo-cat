//! Comandi dell'area AI: pulizia delle etichette automatiche e aggregati per i grafici.
use crate::state::blocking;
use pc_ai::analyze::{AnalysisParams, Scope};
use pc_core::AppResult;
use pc_store::coverage::{ActiveTargets, Stage, StageCoverage, StageKeys, Target as CovTarget};
use pc_store::insights::{self, Bin, MonthCount, SimilarStats};
use pc_store::query::{LibraryFilter, Target};
use tauri::AppHandle;

/// Quante etichette automatiche `grp = value` (con `score < maxScore` se dato) verrebbero rifiutate.
#[tauri::command]
#[specta::specta]
pub async fn ai_reject_preview(
    app: AppHandle,
    grp: String,
    value: String,
    max_score: Option<f64>,
    target: Target,
) -> AppResult<i64> {
    blocking(app, move |_app, state| {
        let mut target = target;
        crate::stage_keys::fill_target(state, &mut target);
        Ok(state
            .store
            .read(|c| pc_store::bulk::reject_preview(c, &grp, &value, max_score, &target))?)
    })
    .await
}

/// Rifiuta le etichette automatiche sul target (job `bulk`, annullabile con `undo_bulk`).
/// Restituisce l'id del job come `start_bulk`.
#[tauri::command]
#[specta::specta]
pub async fn ai_reject_labels(
    app: AppHandle,
    grp: String,
    value: String,
    max_score: Option<f64>,
    target: Target,
) -> AppResult<i64> {
    blocking(app, move |_app, state| {
        let mut target = target;
        crate::stage_keys::fill_target(state, &mut target);
        let operation = pc_jobs::bulk::BulkAction::RejectAutoLabels {
            grp,
            value,
            max_score,
        };
        state.jobs.submit(
            pc_jobs::bulk::JOB_KIND,
            serde_json::json!({"target":target,"operation":operation,"confirmed":false}),
        )
    })
    .await
}

/// `true` se l'ultima operazione annullabile è un rifiuto di etichette automatiche.
#[tauri::command]
#[specta::specta]
pub async fn ai_last_is_rejection(app: AppHandle) -> AppResult<bool> {
    blocking(app, move |_app, state| {
        Ok(state.store.read(pc_store::bulk::last_is_label_rejection)?)
    })
    .await
}

/// Annulla l'ultima operazione solo se è un rifiuto di etichette automatiche.
#[tauri::command]
#[specta::specta]
pub async fn ai_undo_rejection(
    state: tauri::State<'_, crate::state::AppState>,
) -> AppResult<Option<String>> {
    let (db, app) = (state.store.clone(), state.app_store.clone());
    tauri::async_runtime::spawn_blocking(move || {
        db.write(move |c| {
            if !pc_store::bulk::last_is_label_rejection(c)? {
                return Ok(None);
            }
            let Some((description, payloads)) = pc_store::bulk::undo_last_collect(c)? else {
                return Ok(None);
            };
            app.tx(|a| {
                payloads
                    .iter()
                    .try_for_each(|p| pc_store::app_store::tags::restore(a, p))
            })?;
            Ok(Some(description))
        })
        .map_err(Into::into)
    })
    .await
    .map_err(|e| pc_core::AppError::internal(e.to_string()))?
}

/// Istogramma delle confidenze (20 classi se `bins` è assente).
#[tauri::command]
#[specta::specta]
pub async fn ai_label_histogram(
    app: AppHandle,
    filter: LibraryFilter,
    grp: String,
    value: String,
    bins: Option<u32>,
) -> AppResult<Vec<Bin>> {
    blocking(app, move |_app, state| {
        let mut filter = filter;
        crate::stage_keys::fill_filter(state, &mut filter);
        Ok(state.store.read(|c| {
            insights::label_histogram(c, &filter, &grp, &value, bins.unwrap_or(20) as usize)
        })?)
    })
    .await
}

/// Foto totali e analizzate per mese di scatto.
#[tauri::command]
#[specta::specta]
pub async fn ai_month_coverage(
    app: AppHandle,
    filter: LibraryFilter,
) -> AppResult<Vec<MonthCount>> {
    blocking(app, move |_app, state| {
        let mut filter = filter;
        crate::stage_keys::fill_filter(state, &mut filter);
        Ok(state.store.read(|c| insights::month_coverage(c, &filter))?)
    })
    .await
}

/// Statistiche dei gruppi di simili, con le foto ancora da elaborare.
#[tauri::command]
#[specta::specta]
pub async fn ai_similar_stats(
    app: AppHandle,
    filter: Option<LibraryFilter>,
) -> AppResult<SimilarStats> {
    blocking(app, move |_app, state| {
        let mut filter = filter;
        if let Some(f) = filter.as_mut() {
            crate::stage_keys::fill_filter(state, f);
        }
        let mut st = state
            .store
            .read(|c| insights::similar_stats(c, filter.as_ref()))?;
        st.pending = pc_ai::similar::pending_count(&state.ai_deps)? as i64;
        Ok(st)
    })
    .await
}

// ---- Panoramica e rianalisi per ambito ----

/// Copertura di ogni fase (chiavi correnti dei modelli + job attivi come "in coda").
#[derive(Debug, Clone, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AiOverview {
    pub stages: Vec<StageCoverage>,
}

/// Ambito di una rianalisi: tutta la libreria, cataloghi o un target (selezione/filtro).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
// Valore di breve durata (parametro di comando): la differenza di dimensione è irrilevante.
#[allow(clippy::large_enum_variant)]
pub enum ReprocessScope {
    Library,
    #[serde(rename_all = "camelCase")]
    Catalogs {
        ids: Vec<i64>,
    },
    Target {
        target: Target,
    },
}

/// Foto che la fase rielaborerebbe; `available = false` se manca il modello.
#[derive(Debug, Clone, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct StageCount {
    pub stage: Stage,
    pub count: i64,
    pub available: bool,
}

/// Nome della fase nei parametri del job `analysis`.
fn stage_name(s: Stage) -> &'static str {
    match s {
        Stage::Classification => "clip",
        Stage::Persons => "persons",
        Stage::Faces => "faces",
        Stage::Similar => "similar",
        Stage::Phash => "phash",
    }
}

/// Unione di due bersagli: la libreria prevale, gli id si fondono.
fn merge(a: CovTarget, b: CovTarget) -> CovTarget {
    match (a, b) {
        (CovTarget::Library, _) | (_, CovTarget::Library) => CovTarget::Library,
        (CovTarget::None, x) | (x, CovTarget::None) => x,
        (CovTarget::Ids(mut x), CovTarget::Ids(y)) => {
            x.extend(y);
            x.sort_unstable();
            x.dedup();
            CovTarget::Ids(x)
        }
    }
}

fn add_target(at: &mut ActiveTargets, stage: Stage, t: &CovTarget) {
    let slot = match stage {
        Stage::Classification => &mut at.classification,
        Stage::Persons => &mut at.persons,
        Stage::Faces => &mut at.faces,
        Stage::Similar => &mut at.similar,
        Stage::Phash => &mut at.phash,
    };
    *slot = merge(std::mem::take(slot), t.clone());
}

/// Bersagli dei job attivi (`(kind, params)` in coda/avviati/in pausa). `resolve`
/// trasforma un ambito di analisi in bersaglio di copertura (cataloghi e sorgenti
/// diventano id). Mappatura: analisi = suo ambito sulle sue fasi (predefinite:
/// tutte tranne impronta); `similar` = libreria su Simili; `phash_backfill` =
/// libreria su Impronta; import con `sourceId` = foto di quella sorgente su
/// tutte le fasi, senza = libreria.
pub(crate) fn active_targets(
    jobs: &[(String, serde_json::Value)],
    mut resolve: impl FnMut(&Scope) -> CovTarget,
) -> ActiveTargets {
    let mut at = ActiveTargets::default();
    for (kind, params) in jobs {
        match kind.as_str() {
            k if k == pc_ai::analyze::JOB_KIND => {
                let Ok(p) = AnalysisParams::parse(params) else {
                    continue;
                };
                let t = resolve(&p.scope);
                for s in Stage::ALL {
                    if p.wants(s) {
                        add_target(&mut at, s, &t);
                    }
                }
            }
            k if k == pc_ai::similar::JOB_KIND => {
                add_target(&mut at, Stage::Similar, &CovTarget::Library)
            }
            k if k == pc_ingest::phash_backfill::JOB_KIND => {
                add_target(&mut at, Stage::Phash, &CovTarget::Library)
            }
            k if k == pc_ingest::import::JOB_KIND => {
                let t = match params["sourceId"].as_i64() {
                    Some(id) => resolve(&Scope::Source(id)),
                    None => CovTarget::Library,
                };
                for s in Stage::ALL {
                    add_target(&mut at, s, &t);
                }
            }
            _ => {}
        }
    }
    at
}

/// Panoramica AI: copertura per fase.
#[tauri::command]
#[specta::specta]
pub async fn ai_overview(app: AppHandle) -> AppResult<AiOverview> {
    blocking(app, move |_app, state| {
        let keys = crate::stage_keys::stage_keys(state);
        let jobs: Vec<(String, serde_json::Value)> = state.store.read(|c| {
            let mut st = c.prepare(
                "SELECT kind, params FROM job WHERE state IN ('queued','running','paused')",
            )?;
            let rows = st
                .query_map([], |r| {
                    let p: String = r.get(1)?;
                    Ok((r.get(0)?, serde_json::from_str(&p).unwrap_or_default()))
                })?
                .collect::<Result<_, _>>()?;
            Ok(rows)
        })?;
        let active = active_targets(&jobs, |scope| match scope {
            Scope::Library => CovTarget::Library,
            // se la risoluzione fallisce non si dichiara nulla in coda
            s => pc_ai::analyze::resolve_scope(&state.store, s)
                .map(CovTarget::Ids)
                .unwrap_or_default(),
        });
        let stages = state
            .store
            .read(|c| pc_store::coverage::coverage(c, &keys, &active))?;
        Ok(AiOverview { stages })
    })
    .await
}

/// Foto dell'ambito (eleggibili) che ogni fase rielaborerebbe. `ids = None` = libreria.
fn count_stages(
    c: &rusqlite::Connection,
    keys: &StageKeys,
    ids: Option<&[i64]>,
    stages: &[Stage],
    force: bool,
) -> pc_store::StoreResult<Vec<StageCount>> {
    use rusqlite::types::Value;
    let mut out = Vec::new();
    for &stage in stages {
        let available = match stage {
            Stage::Classification => keys.classification.is_some(),
            Stage::Persons => keys.persons.is_some(),
            Stage::Faces => keys.faces.is_some(),
            Stage::Similar | Stage::Phash => true,
        };
        if !available {
            out.push(StageCount {
                stage,
                count: 0,
                available,
            });
            continue;
        }
        let mut params: Vec<Value> = Vec::new();
        let mut cond = String::new();
        if let Some(ids) = ids {
            params.push(Value::Text(serde_json::to_string(ids)?));
            cond.push_str(" AND m.id IN (SELECT value FROM json_each(?1))");
        }
        if !force {
            let cur = pc_store::coverage::current(stage, keys, params.len());
            cond.push_str(&format!(" AND NOT ({})", cur.sql));
            params.extend(cur.params);
        }
        let sql = format!(
            "SELECT count(*) FROM media_item m WHERE m.status='ok' AND m.thumb IS NOT NULL AND {}{cond}",
            pc_store::query::HIDDEN_BY_SYSTEM
        );
        let count = c.query_row(&sql, rusqlite::params_from_iter(params), |r| r.get(0))?;
        out.push(StageCount {
            stage,
            count,
            available,
        });
    }
    Ok(out)
}

/// Id dell'ambito (`None` = libreria intera).
fn scope_ids(state: &crate::state::AppState, scope: ReprocessScope) -> AppResult<Option<Vec<i64>>> {
    Ok(match scope {
        ReprocessScope::Library => None,
        ReprocessScope::Catalogs { ids } => Some(
            state
                .store
                .read(|c| pc_store::reprocess::catalog_media_ids(c, &ids))?,
        ),
        ReprocessScope::Target { mut target } => {
            crate::stage_keys::fill_target(state, &mut target);
            Some(state.store.read(|c| target.resolve(c))?)
        }
    })
}

/// Anteprima della rianalisi: conteggio per fase.
#[tauri::command]
#[specta::specta]
pub async fn ai_reprocess_preview(
    app: AppHandle,
    scope: ReprocessScope,
    stages: Vec<Stage>,
    force: bool,
) -> AppResult<Vec<StageCount>> {
    blocking(app, move |_app, state| {
        let keys = crate::stage_keys::stage_keys(state);
        let ids = scope_ids(state, scope)?;
        Ok(state
            .store
            .read(|c| count_stages(c, &keys, ids.as_deref(), &stages, force))?)
    })
    .await
}

/// Parametri del job `analysis` per un ambito (puro, testabile). Libreria e
/// cataloghi restano simbolici; un target viaggia come id già risolti.
fn job_params(
    scope: &ReprocessScope,
    target_ids: Option<&[i64]>,
    stages: &[Stage],
    force: bool,
) -> serde_json::Value {
    let scope = match scope {
        ReprocessScope::Library => serde_json::json!("library"),
        ReprocessScope::Catalogs { ids } => serde_json::json!({ "catalogIds": ids }),
        ReprocessScope::Target { .. } => serde_json::json!({ "ids": target_ids.unwrap_or(&[]) }),
    };
    let stages: Vec<&str> = stages.iter().map(|s| stage_name(*s)).collect();
    serde_json::json!({ "scope": scope, "stages": stages, "force": force })
}

/// Avvia la rianalisi delle fasi scelte sull'ambito; restituisce l'id del job.
#[tauri::command]
#[specta::specta]
pub async fn ai_reprocess(
    app: AppHandle,
    scope: ReprocessScope,
    stages: Vec<Stage>,
    force: bool,
) -> AppResult<i64> {
    blocking(app, move |_app, state| {
        let keys = crate::stage_keys::stage_keys(state);
        let ids = scope_ids(state, scope.clone())?;
        let total: i64 = state
            .store
            .read(|c| count_stages(c, &keys, ids.as_deref(), &stages, force))?
            .iter()
            .map(|s| s.count)
            .sum();
        if total == 0 {
            return Err(pc_core::AppError::invalid_input(
                "nessun elemento da rianalizzare",
            ));
        }
        let params = job_params(&scope, ids.as_deref(), &stages, force);
        state.jobs.submit(pc_ai::analyze::JOB_KIND, params)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use pc_store::{Store, StoreOptions};
    use serde_json::json;

    fn job(kind: &str, p: serde_json::Value) -> (String, serde_json::Value) {
        (kind.to_string(), p)
    }

    #[test]
    fn active_targets_mapping() {
        let jobs = vec![
            job(
                "analysis",
                json!({"scope":{"catalogIds":[1]},"stages":["clip"]}),
            ),
            job("analysis", json!({"scope":{"ids":[5,3]}})),
            job("similar", json!({})),
            job("phash_backfill", json!({})),
            job("import", json!({"sourceId":9})),
        ];
        let at = active_targets(&jobs, |s| match s {
            Scope::Catalogs(_) => CovTarget::Ids(vec![1, 2]),
            Scope::Source(_) => CovTarget::Ids(vec![7]),
            Scope::Ids(v) => CovTarget::Ids(v.clone()),
            Scope::Library => CovTarget::Library,
        });
        assert_eq!(at.classification, CovTarget::Ids(vec![1, 2, 3, 5, 7]));
        assert_eq!(at.persons, CovTarget::Ids(vec![3, 5, 7]));
        // simili e impronta: la libreria prevale
        assert_eq!(at.similar, CovTarget::Library);
        assert_eq!(at.phash, CovTarget::Library);
        let none = active_targets(&[job("retune", json!({}))], |_| CovTarget::Library);
        assert_eq!(none, ActiveTargets::default());
        let imp = active_targets(&[job("import", json!({}))], |_| CovTarget::None);
        assert_eq!(imp.faces, CovTarget::Library);
    }

    #[test]
    fn job_params_shapes() {
        let st = [Stage::Classification, Stage::Phash];
        let p = job_params(&ReprocessScope::Library, None, &st, true);
        assert_eq!(
            p,
            json!({"scope":"library","stages":["clip","phash"],"force":true})
        );
        let p = job_params(&ReprocessScope::Catalogs { ids: vec![2] }, None, &st, false);
        assert_eq!(p["scope"], json!({"catalogIds":[2]}));
        let t = ReprocessScope::Target {
            target: Target::Ids { ids: vec![1] },
        };
        let p = job_params(&t, Some(&[1, 4]), &st, false);
        assert_eq!(p["scope"], json!({"ids":[1,4]}));
        // il job la rilegge come previsto
        let parsed = AnalysisParams::parse(&p).unwrap();
        assert_eq!(parsed.scope, Scope::Ids(vec![1, 4]));
        assert_eq!(parsed.stages, st.to_vec());
    }

    #[test]
    fn count_stages_scope_force_and_availability() {
        let d = tempfile::tempdir().unwrap();
        let s = Store::open(&d.path().join("catalog.db"), StoreOptions::default()).unwrap();
        s.write(|c| {
            for id in 1..=3i64 {
                c.execute(
                    "INSERT INTO media_item(id, path, name, ext, size, mtime, added_at, updated_at, thumb) VALUES (?1, ?2, ?2, 'jpg', 1, 0, ?1, ?1, 't')",
                    rusqlite::params![id, format!("/p/{id}.jpg")],
                )?;
            }
            // 1: persone corrente; 2: modello vecchio; 1 e 2 hanno l'impronta
            c.execute("INSERT INTO analysis_version(media_id, stage, model_ver, taxonomy_ver) VALUES (1,'persons','pm',NULL),(2,'persons','old',NULL)", [])?;
            c.execute("UPDATE media_item SET phash=x'00' WHERE id IN (1,2)", [])?;
            Ok(())
        })
        .unwrap();
        let keys = StageKeys {
            persons: Some("pm".into()),
            ..Default::default()
        };
        let all = [Stage::Persons, Stage::Faces, Stage::Phash];
        let n = |ids: Option<&[i64]>, force| {
            s.read(|c| count_stages(c, &keys, ids, &all, force))
                .unwrap()
                .iter()
                .map(|x| (x.count, x.available))
                .collect::<Vec<_>>()
        };
        // senza force: persone da rifare 2 e 3, volti non disponibili, impronta solo la 3
        assert_eq!(n(None, false), vec![(2, true), (0, false), (1, true)]);
        // con force: tutta la libreria; i volti indisponibili restano 0
        assert_eq!(n(None, true), vec![(3, true), (0, false), (3, true)]);
        // ambito ristretto
        assert_eq!(
            n(Some(&[1, 3]), false),
            vec![(1, true), (0, false), (1, true)]
        );
        assert_eq!(n(Some(&[]), true), vec![(0, true), (0, false), (0, true)]);
    }
}
