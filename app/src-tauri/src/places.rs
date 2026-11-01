//! Comandi del luogo: rinomina, suggerimenti e ricalcolo dei luoghi dedotti
//! dalle cartelle. Impostazione e ripristino passano dal job bulk (`SetPlace`).
use crate::state::AppState;
use pc_core::{AppError, AppResult};
use pc_store::bulk::{self, BulkOutcome};
use serde::Serialize;
use tauri::State;

/// Luogo proposto durante la digitazione.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlaceSuggestion {
    pub name: String,
    pub country: Option<String>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    /// Già presente in libreria (altrimenti dal dataset geografico).
    pub in_library: bool,
}

/// Suggerimenti: prima i luoghi della libreria, poi le località del dataset
/// geografico offline non già presenti.
pub(crate) fn suggestions(
    db: &pc_store::Store,
    gaz: Option<&pc_ai::geo::Gazetteer>,
    prefix: &str,
    limit: usize,
) -> AppResult<Vec<PlaceSuggestion>> {
    let p = prefix.to_string();
    let mut out: Vec<PlaceSuggestion> = db
        .read(move |c| bulk::suggest_places(c, &p, limit))?
        .into_iter()
        .map(|name| PlaceSuggestion {
            name,
            country: None,
            lat: None,
            lon: None,
            in_library: true,
        })
        .collect();
    if let Some(g) = gaz {
        for city in g.by_prefix(prefix, limit) {
            if out.len() >= limit {
                break;
            }
            if out
                .iter()
                .any(|s| pc_ai::geo::fold(&s.name) == pc_ai::geo::fold(&city.name))
            {
                continue;
            }
            out.push(PlaceSuggestion {
                name: city.name.clone(),
                country: Some(city.country.clone()),
                lat: Some(city.lat),
                lon: Some(city.lon),
                in_library: false,
            });
        }
    }
    Ok(out)
}

#[tauri::command]
#[specta::specta]
pub async fn suggest_places(
    state: State<'_, AppState>,
    prefix: String,
) -> AppResult<Vec<PlaceSuggestion>> {
    let (db, deps) = (state.store.clone(), state.ai_deps.clone());
    tauri::async_runtime::spawn_blocking(move || {
        if prefix.trim().is_empty() {
            return suggestions(&db, None, &prefix, 8);
        }
        suggestions(&db, deps.gazetteer().as_deref(), &prefix, 8)
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))?
}

/// Rinomina un luogo per tutte le foto che lo hanno (annullabile).
#[tauri::command]
#[specta::specta]
pub async fn rename_place(
    state: State<'_, AppState>,
    from: String,
    to: String,
) -> AppResult<BulkOutcome> {
    let db = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.write(move |c| bulk::rename_place(c, &from, &to))
            .map_err(Into::into)
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))?
}

/// Ricalcola i luoghi dedotti dalle cartelle con le cartelle da ignorare
/// correnti; manuali e GPS restano. Restituisce le foto cambiate.
#[tauri::command]
#[specta::specta]
pub async fn recompute_places(state: State<'_, AppState>) -> AppResult<u32> {
    let (db, deps) = (state.store.clone(), state.ai_deps.clone());
    tauri::async_runtime::spawn_blocking(move || {
        let resolve = deps.place_resolver();
        db.write(move |c| bulk::recompute_directory_places(c, &resolve))
            .map(|n| n as u32)
            .map_err(Into::into)
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use pc_store::model::{Location, LocationSource, NewMedia};
    use pc_store::{repo, Store};

    #[test]
    fn library_places_come_first_then_gazetteer() {
        let tmp = tempfile::tempdir().unwrap();
        let db = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
        db.write(|c| {
            let id = repo::media::upsert(
                c,
                &NewMedia {
                    path: "D:/a.jpg".into(),
                    name: "a.jpg".into(),
                    ext: "jpg".into(),
                    ..Default::default()
                },
            )?;
            repo::meta::set_location(
                c,
                id,
                &Location {
                    name: Some("Siena centro".into()),
                    country: None,
                    lat: None,
                    lon: None,
                    source: LocationSource::Manual,
                },
            )?;
            Ok(())
        })
        .unwrap();
        let g = pc_ai::geo::Gazetteer::parse(
            "1\tSiena\tSiena\t43.3\t11.3\tIT\t16\t53901\n2\tSiena centro\t\t43.3\t11.3\tIT\t16\t1\n",
        )
        .unwrap();
        let s = suggestions(&db, Some(&g), "sie", 8).unwrap();
        let names: Vec<(&str, bool)> = s.iter().map(|s| (s.name.as_str(), s.in_library)).collect();
        assert_eq!(names, vec![("Siena centro", true), ("Siena", false)]);
        assert_eq!(s[1].country.as_deref(), Some("IT"));
    }
}
