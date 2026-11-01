//! Comandi della vista Sequenza: sessioni, gruppi di sorgente e correzioni orarie.
use crate::state::AppState;
use pc_core::{AppError, AppResult};
use pc_store::sequence::{
    self, SequenceCursor, SequencePage, SequenceScope, SessionView, UndatedItem,
    DEFAULT_SESSION_GAP_MIN,
};
use pc_store::timeline::{
    self, Alignment, ClockPreview, GroupClock, SourceGroupView, TzSuggestion,
};
use pc_store::{AppStore, Store};
use serde::{Deserialize, Serialize};
use tauri::State;

const GAP_KEY: &str = "sequence_session_gap_min";

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SequenceOverview {
    pub sessions: Vec<SessionView>,
    pub groups: Vec<SourceGroupView>,
    pub undated: Vec<UndatedItem>,
    pub suggestions: Vec<TzSuggestion>,
    pub total: i64,
    pub gap_min: i64,
}

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?
}

pub fn gap_min(app: &AppStore) -> i64 {
    app.kv_get(GAP_KEY)
        .ok()
        .flatten()
        .and_then(|v| v.parse::<i64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(DEFAULT_SESSION_GAP_MIN)
}

/// Tag del set attivo (per i chip della sequenza).
pub fn active_tags(app: &AppStore) -> AppResult<Vec<String>> {
    use pc_store::app_store::tags;
    Ok(app.with(|c| {
        let id = tags::active_set(c)?;
        Ok(match id {
            Some(id) => tags::get_set(c, id)?.tag_names(),
            None => Vec::new(),
        })
    })?)
}

fn scope_ids(db: &Store, scope: &SequenceScope) -> AppResult<Option<Vec<i64>>> {
    Ok(match scope {
        SequenceScope::Library => None,
        other => {
            let s = other.clone();
            Some(db.read(move |c| sequence::ordered_ids(c, &s))?)
        }
    })
}

pub fn overview(
    db: &Store,
    app: &AppStore,
    scope: SequenceScope,
    tz_at: &dyn Fn(f64, f64) -> Option<String>,
) -> AppResult<SequenceOverview> {
    let gap = gap_min(app);
    let ids = scope_ids(db, &scope)?;
    let s = scope.clone();
    let (sessions, undated) =
        db.read(move |c| Ok((sequence::sessions(c, &s, gap)?, sequence::undated(c, &s)?)))?;
    let groups = {
        let ids = ids.clone();
        db.read(move |c| timeline::list_groups(c, ids.as_deref()))?
    };
    let suggestions = db.read(move |c| timeline::suggest_timezones(c, ids.as_deref(), tz_at))?;
    Ok(SequenceOverview {
        total: sessions.iter().map(|s| s.count).sum(),
        sessions,
        groups,
        undated,
        suggestions,
        gap_min: gap,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn sequence_overview(
    state: State<'_, AppState>,
    scope: SequenceScope,
) -> AppResult<SequenceOverview> {
    let (db, app, ai) = (
        state.store.clone(),
        state.app_store.clone(),
        state.ai_deps.clone(),
    );
    blocking(move || {
        let gaz = ai.gazetteer();
        let tz_at = move |lat: f64, lon: f64| -> Option<String> {
            gaz.as_ref()?.timezone_at(lat, lon).map(str::to_string)
        };
        overview(&db, &app, scope, &tz_at)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn sequence_page(
    state: State<'_, AppState>,
    scope: SequenceScope,
    after: Option<SequenceCursor>,
    limit: u32,
) -> AppResult<SequencePage> {
    let (db, app) = (state.store.clone(), state.app_store.clone());
    blocking(move || {
        let (gap, tags) = (gap_min(&app), active_tags(&app)?);
        Ok(db.read(move |c| sequence::page(c, &scope, gap, &tags, after.as_ref(), limit))?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn sequence_session_ids(
    state: State<'_, AppState>,
    scope: SequenceScope,
    session: i64,
) -> AppResult<Vec<i64>> {
    let (db, app) = (state.store.clone(), state.app_store.clone());
    blocking(move || {
        let gap = gap_min(&app);
        Ok(db.read(move |c| sequence::session_ids(c, &scope, gap, session))?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn sequence_ids(state: State<'_, AppState>, scope: SequenceScope) -> AppResult<Vec<i64>> {
    let db = state.store.clone();
    blocking(move || Ok(db.read(move |c| sequence::ordered_ids(c, &scope))?)).await
}

#[tauri::command]
#[specta::specta]
pub fn set_session_gap(state: State<'_, AppState>, minutes: i64) -> AppResult<()> {
    if !(1..=7 * 24 * 60).contains(&minutes) {
        return Err(AppError::invalid_input(
            "soglia delle sessioni fuori dai limiti",
        ));
    }
    Ok(state.app_store.kv_set(GAP_KEY, &minutes.to_string())?)
}

#[tauri::command]
#[specta::specta]
pub async fn set_group_display(
    state: State<'_, AppState>,
    group: i64,
    tz: Option<String>,
    offset_min: Option<i32>,
) -> AppResult<usize> {
    let db = state.store.clone();
    blocking(move || {
        Ok(db.write(move |c| {
            timeline::set_display(c, group, tz, offset_min, &timeline::system_offset)
        })?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn set_group_shift(
    state: State<'_, AppState>,
    group: i64,
    shift_s: i64,
) -> AppResult<usize> {
    let db = state.store.clone();
    blocking(move || {
        Ok(db.write(move |c| timeline::set_shift(c, group, shift_s, &timeline::system_offset))?)
    })
    .await
}

/// Anteprima dell'ora prima e dopo una correzione del gruppo.
#[tauri::command]
#[specta::specta]
pub async fn preview_group_clock(
    state: State<'_, AppState>,
    group: i64,
    tz: Option<String>,
    offset_min: Option<i32>,
    shift_s: i64,
) -> AppResult<Vec<ClockPreview>> {
    let db = state.store.clone();
    blocking(move || {
        let clock = GroupClock {
            display_offset_min: if tz.is_some() { None } else { offset_min },
            display_tz: tz,
            shift_s,
        };
        Ok(db.read(move |c| {
            timeline::preview_clock(c, group, &clock, 6, &timeline::system_offset)
        })?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn propose_alignment(
    state: State<'_, AppState>,
    reference: i64,
    other: i64,
) -> AppResult<Alignment> {
    let db = state.store.clone();
    blocking(move || Ok(db.read(move |c| timeline::propose_alignment(c, reference, other))?)).await
}

#[tauri::command]
#[specta::specta]
pub async fn place_undated(state: State<'_, AppState>, media: i64, after: i64) -> AppResult<i64> {
    let db = state.store.clone();
    blocking(move || {
        Ok(db.write(move |c| timeline::place_after(c, media, after, &timeline::system_offset))?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub fn ignore_tz_suggestion(
    state: State<'_, AppState>,
    group: i64,
    offset_min: i32,
) -> AppResult<()> {
    Ok(state
        .store
        .write(move |c| timeline::ignore_suggestion(c, group, offset_min))?)
}

/// Nomi dei fusi IANA per il selettore dell'ora locale.
#[tauri::command]
#[specta::specta]
pub fn list_timezones() -> Vec<String> {
    let mut v: Vec<String> = chrono_tz::TZ_VARIANTS
        .iter()
        .map(|t| t.name().to_string())
        .filter(|n| n.contains('/') && !n.starts_with("Etc/"))
        .collect();
    v.sort();
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use pc_store::model::{DateSource, MediaDates, NewMedia};

    #[test]
    fn overview_with_custom_gap_and_active_set_tags() {
        let tmp = tempfile::tempdir().unwrap();
        let db = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
        let app = AppStore::open(&tmp.path().join("app.db")).unwrap();
        db.write(|c| {
            for (i, at) in ["2026-08-19 08:00:00", "2026-08-19 08:50:00"]
                .iter()
                .enumerate()
            {
                let id = pc_store::repo::media::upsert(
                    c,
                    &NewMedia {
                        path: format!("{i}.jpg"),
                        name: format!("{i}.jpg"),
                        ext: "jpg".into(),
                        ..Default::default()
                    },
                )?;
                pc_store::repo::meta::set_dates(
                    c,
                    id,
                    &MediaDates {
                        captured_at: Some(at.to_string()),
                        captured_source: Some(DateSource::ExifOriginal),
                        declared_offset_min: Some(0),
                        device: Some("X".into()),
                        ..Default::default()
                    },
                )?;
            }
            pc_store::bulk::add_tag(c, "Preferite", &[1], &[]).map(|_| ())
        })
        .unwrap();
        let none = |_: f64, _: f64| None;
        let o = overview(&db, &app, SequenceScope::Library, &none).unwrap();
        assert_eq!((o.sessions.len(), o.total, o.gap_min), (1, 2, 120));
        app.kv_set(GAP_KEY, "30").unwrap();
        let o = overview(&db, &app, SequenceScope::Library, &none).unwrap();
        assert_eq!(o.sessions.len(), 2);
        assert_eq!(o.groups.len(), 1);
        assert_eq!(active_tags(&app).unwrap()[0], "Preferite");
        let p = db
            .read(|c| {
                sequence::page(
                    c,
                    &SequenceScope::Library,
                    120,
                    &["Preferite".into()],
                    None,
                    10,
                )
            })
            .unwrap();
        assert_eq!(p.items[0].tags, vec!["Preferite"]);
        assert!(list_timezones().contains(&"Europe/Rome".to_string()));
    }
}
