//! Comandi dei gruppi di foto simili: Inspector, correzioni, espansione nella griglia e
//! "Seleziona tutte tranne la principale". Nessun comando tocca i file su disco.

use crate::state::blocking;
use pc_core::AppResult;
use pc_store::model::MediaId;
use pc_store::query::{self, LibraryFilter, Page, SortKey};
use pc_store::similar::{self, SimilarGroupView};
use tauri::AppHandle;

fn regroup(state: &crate::state::AppState) -> AppResult<()> {
    let threshold = state.config.read().ai.thresholds.similar_photos;
    pc_ai::similar::regroup(&state.store, threshold, "manual")?;
    Ok(())
}

/// Gruppo della foto (Inspector), se c'è.
#[tauri::command]
#[specta::specta]
pub async fn similar_group(app: AppHandle, media: MediaId) -> AppResult<Option<SimilarGroupView>> {
    blocking(app, move |_app, state| {
        Ok(state.store.read(|c| similar::group_view(c, media))?)
    })
    .await
}

/// "Togli dal gruppo": la foto non torna nel gruppo nelle analisi successive.
#[tauri::command]
#[specta::specta]
pub async fn similar_remove(app: AppHandle, media: MediaId) -> AppResult<()> {
    blocking(app, move |_app, state| {
        state
            .store
            .write(move |c| similar::remove_from_group(c, media))?;
        regroup(state)
    })
    .await
}

/// "Raggruppa come simili": le foto restano unite anche dopo nuove analisi.
#[tauri::command]
#[specta::specta]
pub async fn similar_group_together(app: AppHandle, ids: Vec<MediaId>) -> AppResult<()> {
    if ids.len() < 2 {
        return Err(pc_core::AppError::invalid_input("servono almeno due foto"));
    }
    blocking(app, move |_app, state| {
        state
            .store
            .write(move |c| similar::group_together(c, &ids))?;
        regroup(state)
    })
    .await
}

/// "Sciogli gruppo": le foto tornano singole e non si raggruppano più tra loro da sole.
#[tauri::command]
#[specta::specta]
pub async fn similar_dissolve(app: AppHandle, group: i64) -> AppResult<()> {
    blocking(app, move |_app, state| {
        state
            .store
            .write(move |c| similar::dissolve(c, group).map(|_| ()))?;
        regroup(state)
    })
    .await
}

/// Imposta la foto principale del gruppo (prevale sulle analisi successive).
#[tauri::command]
#[specta::specta]
pub async fn similar_set_cover(app: AppHandle, group: i64, media: MediaId) -> AppResult<()> {
    blocking(app, move |_app, state| {
        Ok(state
            .store
            .write(move |c| similar::set_cover(c, group, media))?)
    })
    .await
}

/// Foto del gruppo presenti nei risultati del filtro (espansione nella griglia).
#[tauri::command]
#[specta::specta]
pub async fn similar_group_members(
    app: AppHandle,
    group: i64,
    filter: LibraryFilter,
    sort: SortKey,
) -> AppResult<Page> {
    blocking(app, move |_app, state| {
        Ok(state.store.read(|c| {
            let members = similar::group_members(c, group)?;
            let f = LibraryFilter {
                ids: Some(match &filter.ids {
                    Some(ids) => members.into_iter().filter(|m| ids.contains(m)).collect(),
                    None => members,
                }),
                group_similar: false,
                ..filter
            };
            query::page_at(c, &f, sort, None, 1000)
        })?)
    })
    .await
}

/// Foto da selezionare con "Seleziona tutte tranne la principale": i membri dei gruppi presenti
/// nei risultati, esclusa la principale di ciascun gruppo (o la migliore, se la principale non
/// è tra i risultati).
#[tauri::command]
#[specta::specta]
pub async fn similar_non_covers(app: AppHandle, filter: LibraryFilter) -> AppResult<Vec<MediaId>> {
    blocking(app, move |_app, state| {
        Ok(state.store.read(|c| similar::non_covers(c, &filter))?)
    })
    .await
}

/// Anteprima per le Impostazioni: gruppi con la soglia `threshold` (dagli archi salvati).
#[tauri::command]
#[specta::specta]
pub async fn similar_preview(app: AppHandle, threshold: f32) -> AppResult<u32> {
    blocking(app, move |_app, state| {
        Ok(pc_ai::similar::preview_count(&state.store, threshold)? as u32)
    })
    .await
}

/// Foto analizzate con CLIP ma non ancora elaborate per i gruppi (librerie esistenti).
#[tauri::command]
#[specta::specta]
pub async fn similar_pending(app: AppHandle) -> AppResult<u32> {
    blocking(app, move |_app, state| {
        let Ok(entry) = state
            .ai_deps
            .resources
            .entry(pc_models::Role::ClassificationImage)
        else {
            return Ok(0);
        };
        let model = pc_ai::stages::model_key(entry);
        let key = pc_ai::similar::params_key(&model);
        Ok(state
            .store
            .read(move |c| similar::pending(c, &model, &key))?
            .len() as u32)
    })
    .await
}

/// Accoda l'analisi dei gruppi (librerie esistenti: proposta, non automatica).
#[tauri::command]
#[specta::specta]
pub async fn start_similar_analysis(app: AppHandle) -> AppResult<i64> {
    blocking(app, move |_app, state| {
        state
            .jobs
            .submit(pc_ai::similar::JOB_KIND, serde_json::json!({}))
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{params, Connection};

    fn db() -> Connection {
        let mut c = Connection::open_in_memory().unwrap();
        pc_store::migrate::migrate(&mut c, None).unwrap();
        for i in 1..=6 {
            c.execute(
                "INSERT INTO media_item(id,path,name,ext,size,mtime,added_at,updated_at)
                 VALUES (?1, ?2, ?2, 'jpg', 1, 0, ?1, 0)",
                params![i, format!("{i}.jpg")],
            )
            .unwrap();
        }
        c
    }

    #[test]
    fn grouped_page_shows_one_item_per_group_with_the_count_of_filtered_photos() {
        let c = db();
        for (id, s) in [(1, 10.0), (2, 50.0), (3, 20.0), (4, 5.0)] {
            similar::mark_done(&c, id, "p", s, 1.0).unwrap();
        }
        similar::apply_groups(&c, &[vec![1, 2, 3, 4]], "p").unwrap();
        let all = LibraryFilter {
            group_similar: true,
            ..Default::default()
        };
        let p = query::page_at(&c, &all, SortKey::AddedDesc, None, 100).unwrap();
        let ids: Vec<_> = p.items.iter().map(|i| i.id).collect();
        assert_eq!(ids, vec![6, 5, 2], "gruppo chiuso: solo la principale");
        let cover = p.items.iter().find(|i| i.id == 2).unwrap();
        assert_eq!(cover.similar_count, Some(4));
        assert!(cover.similar_cover);
        assert_eq!(
            query::count(&c, &all).unwrap(),
            6,
            "il conteggio resta sulle foto"
        );
        assert_eq!(
            query::grid_count(&c, &all).unwrap(),
            3,
            "elementi della griglia"
        );
        // Filtro che include solo 3 foto del gruppo, principale esclusa.
        let partial = LibraryFilter {
            ids: Some(vec![1, 3, 4, 5]),
            ..all.clone()
        };
        let p = query::page_at(&c, &partial, SortKey::AddedDesc, None, 100).unwrap();
        let rep = p.items.iter().find(|i| i.similar_group.is_some()).unwrap();
        assert_eq!(rep.id, 3, "la migliore delle tre presenti");
        assert_eq!(rep.similar_count, Some(3));
        assert_eq!(p.items.len(), 2);
    }

    #[test]
    fn only_similar_filter_keeps_group_members() {
        let c = db();
        similar::apply_groups(&c, &[vec![1, 2], vec![4, 5]], "p").unwrap();
        let f = LibraryFilter {
            similar_only: true,
            ..Default::default()
        };
        assert_eq!(query::count(&c, &f).unwrap(), 4);
    }
}
