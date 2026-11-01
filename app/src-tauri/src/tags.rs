//! Comandi della schermata Tag: tag della libreria (in `catalog.db`) e set,
//! colori e set attivo (in `app.db`).
use crate::state::AppState;
use pc_core::{AppError, AppResult};
use pc_store::app_store::tags::{self as sets, TagSet};
use pc_store::bulk::{self, BulkOutcome};
use pc_store::{repo, AppStore, Store, StoreError};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TagInfo {
    pub name: String,
    /// File della libreria con questo tag (0 se il tag esiste solo in un set).
    pub count: i64,
    pub color: String,
    /// Set che contengono il tag.
    pub sets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TagSetView {
    #[serde(flatten)]
    pub set: TagSet,
    /// File della libreria con almeno un tag del set.
    pub tagged: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TagOverview {
    pub tags: Vec<TagInfo>,
    pub sets: Vec<TagSetView>,
    pub active_set: Option<i64>,
    pub palette: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub enum RenameOutcome {
    Renamed,
    /// Esiste già un tag con quel nome: va confermata l'unione.
    NeedsMerge {
        target: String,
    },
}

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?
}

/// Riepilogo di tag e set per la libreria aperta.
pub fn overview(db: &Store, app: &AppStore) -> AppResult<TagOverview> {
    let (all_sets, active, colors, by_tag) = app.with(|c| {
        Ok((
            sets::list_sets(c)?,
            sets::active_set(c)?,
            sets::colors(c)?,
            sets::sets_by_tag(c)?,
        ))
    })?;
    let lib = db.read(repo::tag::list)?;
    let mut tags: Vec<TagInfo> = lib
        .iter()
        .map(|(_, name, count)| TagInfo {
            name: name.clone(),
            count: *count,
            color: sets::color_of(&colors, name),
            sets: by_tag
                .get(&name.to_lowercase())
                .cloned()
                .unwrap_or_default(),
        })
        .collect();
    // I tag dei set non ancora usati in questa libreria compaiono con 0 file.
    for s in &all_sets {
        for t in &s.tags {
            if !tags
                .iter()
                .any(|x| x.name.to_lowercase() == t.name.to_lowercase())
            {
                tags.push(TagInfo {
                    name: t.name.clone(),
                    count: 0,
                    color: sets::color_of(&colors, &t.name),
                    sets: by_tag
                        .get(&t.name.to_lowercase())
                        .cloned()
                        .unwrap_or_default(),
                });
            }
        }
    }
    tags.sort_by_key(|t| t.name.to_lowercase());
    let mut views = Vec::with_capacity(all_sets.len());
    for set in all_sets {
        let names = set.tag_names();
        let tagged = db.read(move |c| {
            Ok(c.query_row(
                "SELECT count(DISTINCT mt.media_id) FROM media_tag mt JOIN tag t ON t.id = mt.tag_id
                 WHERE t.name IN (SELECT value FROM json_each(?1))",
                [serde_json::to_string(&names)?],
                |r| r.get(0),
            )?)
        })?;
        views.push(TagSetView { set, tagged });
    }
    Ok(TagOverview {
        tags,
        sets: views,
        active_set: active,
        palette: sets::PALETTE.iter().map(|s| s.to_string()).collect(),
    })
}

#[tauri::command]
#[specta::specta]
pub async fn tag_overview(state: State<'_, AppState>) -> AppResult<TagOverview> {
    let (db, app) = (state.store.clone(), state.app_store.clone());
    blocking(move || overview(&db, &app)).await
}

#[tauri::command]
#[specta::specta]
pub async fn rename_tag(
    state: State<'_, AppState>,
    from: String,
    to: String,
) -> AppResult<RenameOutcome> {
    let (db, app) = (state.store.clone(), state.app_store.clone());
    blocking(move || {
        let target = to.trim().to_string();
        let res = db.write(move |c| {
            let snap = app.with(|a| sets::snapshot(a, &[&from, &to]))?;
            bulk::rename_tag(c, &from, &to, Some(snap))?;
            // Dentro la scrittura: se app.db fallisce, anche il catalogo torna indietro.
            app.with(|a| sets::rename_tag(a, &from, &to))
        });
        match res {
            Ok(()) => Ok(RenameOutcome::Renamed),
            Err(StoreError::Conflict(_)) => Ok(RenameOutcome::NeedsMerge { target }),
            Err(e) => Err(e.into()),
        }
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn merge_tag(
    state: State<'_, AppState>,
    source: String,
    target: String,
) -> AppResult<BulkOutcome> {
    let (db, app) = (state.store.clone(), state.app_store.clone());
    blocking(move || {
        db.write(move |c| {
            let snap = app.with(|a| sets::snapshot(a, &[&source, &target]))?;
            let out = bulk::merge_tag(c, &source, &target, Some(snap))?;
            app.with(|a| sets::merge_tag(a, &source, &target))?;
            Ok(out)
        })
        .map_err(Into::into)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_tag(
    state: State<'_, AppState>,
    name: String,
    confirmed: bool,
) -> AppResult<BulkOutcome> {
    if !confirmed {
        return Err(AppError::invalid_input("conferma richiesta"));
    }
    let (db, app) = (state.store.clone(), state.app_store.clone());
    blocking(move || {
        db.write(move |c| {
            let snap = app.with(|a| sets::snapshot(a, &[&name]))?;
            let out = bulk::delete_tag(c, &name, Some(snap))?;
            app.with(|a| sets::delete_tag(a, &name))?;
            Ok(out)
        })
        .map_err(Into::into)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub fn set_tag_color(
    state: State<'_, AppState>,
    name: String,
    color: Option<String>,
) -> AppResult<()> {
    Ok(state
        .app_store
        .with(|c| sets::set_color(c, &name, color.as_deref()))?)
}

/// Crea (senza `id`) o aggiorna un set: nome e tag in ordine di tasto.
#[tauri::command]
#[specta::specta]
pub fn save_tag_set(
    state: State<'_, AppState>,
    id: Option<i64>,
    name: String,
    tags: Vec<String>,
) -> AppResult<i64> {
    Ok(state.app_store.tx(|c| match id {
        Some(id) => {
            sets::rename_set(c, id, &name)?;
            sets::set_tags(c, id, &tags)?;
            Ok(id)
        }
        None => sets::create_set(c, &name, &tags),
    })?)
}

#[tauri::command]
#[specta::specta]
pub fn duplicate_tag_set(state: State<'_, AppState>, id: i64) -> AppResult<i64> {
    Ok(state.app_store.tx(|c| sets::duplicate_set(c, id))?)
}

#[tauri::command]
#[specta::specta]
pub fn delete_tag_set(state: State<'_, AppState>, id: i64) -> AppResult<()> {
    Ok(state.app_store.tx(|c| sets::delete_set(c, id))?)
}

#[tauri::command]
#[specta::specta]
pub fn reorder_tag_sets(state: State<'_, AppState>, ids: Vec<i64>) -> AppResult<()> {
    Ok(state.app_store.tx(|c| sets::reorder_sets(c, &ids))?)
}

#[tauri::command]
#[specta::specta]
pub fn add_tag_to_set(state: State<'_, AppState>, id: i64, tag: String) -> AppResult<u8> {
    Ok(state.app_store.with(|c| sets::add_to_set(c, id, &tag))?)
}

#[tauri::command]
#[specta::specta]
pub fn remove_tag_from_set(state: State<'_, AppState>, id: i64, tag: String) -> AppResult<()> {
    Ok(state
        .app_store
        .with(|c| sets::remove_from_set(c, id, &tag))?)
}

#[tauri::command]
#[specta::specta]
pub fn set_active_tag_set(state: State<'_, AppState>, id: i64) -> AppResult<()> {
    Ok(state.app_store.with(|c| sets::set_active(c, id))?)
}

#[tauri::command]
#[specta::specta]
pub fn set_tag_set_export_keywords(state: State<'_, AppState>, id: i64, on: bool) -> AppResult<()> {
    Ok(state
        .app_store
        .with(|c| sets::set_export_keywords(c, id, on))?)
}

/// File con due o più tag del set (da sistemare a mano se il set è esclusivo).
#[tauri::command]
#[specta::specta]
pub async fn tag_set_conflicts(state: State<'_, AppState>, id: i64) -> AppResult<Vec<i64>> {
    let (db, app) = (state.store.clone(), state.app_store.clone());
    blocking(move || {
        let names = app.with(|c| sets::get_set(c, id))?.tag_names();
        Ok(db.read(move |c| repo::tag::conflicts(c, &names))?)
    })
    .await
}

/// Rende il set esclusivo (o no). Non rimuove tag: restituisce i file in conflitto.
#[tauri::command]
#[specta::specta]
pub async fn set_tag_set_exclusive(
    state: State<'_, AppState>,
    id: i64,
    exclusive: bool,
) -> AppResult<Vec<i64>> {
    state
        .app_store
        .with(|c| sets::set_exclusive(c, id, exclusive))?;
    if !exclusive {
        return Ok(Vec::new());
    }
    tag_set_conflicts(state, id).await
}

/// Completa un'azione bulk con i dati dei set: per "aggiungi tag" gli altri
/// tag dei set esclusivi che lo contengono.
pub fn with_set_rules(
    app: &AppStore,
    operation: pc_jobs::bulk::BulkAction,
) -> AppResult<pc_jobs::bulk::BulkAction> {
    use pc_jobs::bulk::BulkAction;
    Ok(match operation {
        BulkAction::AddTag { name, .. } => {
            let exclusive = app.with(|c| sets::exclusive_siblings(c, &name))?;
            BulkAction::AddTag { name, exclusive }
        }
        other => other,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pc_store::model::NewMedia;

    fn stores() -> (tempfile::TempDir, Store, AppStore, Vec<i64>) {
        let tmp = tempfile::tempdir().unwrap();
        let db = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
        let app = AppStore::open(&tmp.path().join("app.db")).unwrap();
        let ids = db
            .write(|c| {
                (0..4)
                    .map(|i| {
                        repo::media::upsert(
                            c,
                            &NewMedia {
                                path: format!("D:\\{i}.jpg"),
                                name: format!("{i}.jpg"),
                                ext: "jpg".into(),
                                ..Default::default()
                            },
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .unwrap();
        (tmp, db, app, ids)
    }

    #[test]
    fn overview_lists_library_and_set_tags() {
        let (_t, db, app, ids) = stores();
        let usa = app
            .with(|c| sets::create_set(c, "USA", &["Tramonti".into(), "Cibo".into()]))
            .unwrap();
        let some = ids[..3].to_vec();
        db.write(move |c| bulk::add_tag(c, "Tramonti", &some, &[]).map(|_| ()))
            .unwrap();
        let o = overview(&db, &app).unwrap();
        let tramonti = o.tags.iter().find(|t| t.name == "Tramonti").unwrap();
        assert_eq!(tramonti.count, 3);
        assert_eq!(tramonti.sets, vec!["USA"]);
        // Tag del set Base non ancora usati in libreria: presenti con 0 file.
        assert_eq!(
            o.tags.iter().find(|t| t.name == "Preferite").unwrap().count,
            0
        );
        let set = o.sets.iter().find(|s| s.set.id == usa).unwrap();
        assert_eq!(set.tagged, 3);
        assert_eq!(o.active_set, Some(1));
    }

    #[test]
    fn exclusive_rule_is_added_to_bulk_action() {
        let (_t, _db, app, _) = stores();
        app.with(|c| {
            let id = sets::create_set(c, "USA", &["Paesaggi".into(), "Città".into()])?;
            sets::set_exclusive(c, id, true)
        })
        .unwrap();
        let op = with_set_rules(
            &app,
            pc_jobs::bulk::BulkAction::AddTag {
                name: "Città".into(),
                exclusive: vec![],
            },
        )
        .unwrap();
        match op {
            pc_jobs::bulk::BulkAction::AddTag { exclusive, .. } => {
                assert_eq!(exclusive, vec!["Paesaggi".to_string()])
            }
            _ => unreachable!(),
        }
    }
}
