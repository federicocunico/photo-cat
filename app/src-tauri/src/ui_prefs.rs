//! Copia delle preferenze dell'interfaccia (layout, ordinamento, pannelli) in
//! `ui-prefs.json` nella cartella dell'app, accanto ad `app.db`. Il profilo
//! WebView2 può essere cancellato dal disinstallatore ("Elimina i dati
//! dell'applicazione"); questa cartella no, quindi al riavvio la UI ripristina
//! da qui le preferenze mancanti.

use crate::state::AppState;
use pc_core::AppResult;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use tauri::State;

/// Solo le chiavi dei negozi persistiti della UI.
const PREFIX: &str = "photo-cat.";
/// Limite di sicurezza per un singolo valore.
const MAX_VALUE: usize = 256 * 1024;

pub type UiPrefs = BTreeMap<String, String>;

fn file(app_dir: &Path) -> PathBuf {
    app_dir.join("ui-prefs.json")
}

fn accepted(prefs: UiPrefs) -> UiPrefs {
    prefs
        .into_iter()
        .filter(|(k, v)| k.starts_with(PREFIX) && k.len() <= 64 && v.len() <= MAX_VALUE)
        .collect()
}

/// Preferenze salvate; file mancante o illeggibile = nessuna preferenza.
pub fn load(app_dir: &Path) -> UiPrefs {
    std::fs::read_to_string(file(app_dir))
        .ok()
        .and_then(|s| serde_json::from_str::<UiPrefs>(&s).ok())
        .map(accepted)
        .unwrap_or_default()
}

/// Scrittura atomica (file temporaneo nella stessa cartella, poi rinomina).
pub fn save(app_dir: &Path, prefs: UiPrefs) -> AppResult<()> {
    std::fs::create_dir_all(app_dir)?;
    let mut tmp = tempfile::NamedTempFile::new_in(app_dir)?;
    tmp.write_all(serde_json::to_string_pretty(&accepted(prefs))?.as_bytes())?;
    tmp.as_file().sync_all()?;
    tmp.persist(file(app_dir)).map_err(|e| e.error)?;
    Ok(())
}

/// Preferenze dell'interfaccia salvate nella cartella dell'app.
#[tauri::command]
#[specta::specta]
pub fn ui_prefs_load(state: State<'_, AppState>) -> UiPrefs {
    load(state.paths.app_dir())
}

/// Salva le preferenze dell'interfaccia nella cartella dell'app.
#[tauri::command]
#[specta::specta]
pub fn ui_prefs_save(state: State<'_, AppState>, prefs: UiPrefs) -> AppResult<()> {
    save(state.paths.app_dir(), prefs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_keeps_only_ui_keys() {
        let dir = tempfile::tempdir().unwrap();
        let app = dir.path().join("config");
        assert!(load(&app).is_empty());
        let prefs = UiPrefs::from([
            (
                "photo-cat.layout".into(),
                r#"{"state":{"leftWidth":320}}"#.into(),
            ),
            (
                "photo-cat.library".into(),
                r#"{"state":{"sort":"name_asc"}}"#.into(),
            ),
            ("altro".into(), "x".into()),
        ]);
        save(&app, prefs).unwrap();
        let back = load(&app);
        assert_eq!(back.len(), 2);
        assert_eq!(
            back["photo-cat.library"],
            r#"{"state":{"sort":"name_asc"}}"#
        );
        assert!(!back.contains_key("altro"));
        // Nessun file temporaneo lasciato nella cartella.
        assert_eq!(std::fs::read_dir(&app).unwrap().count(), 1);
    }

    #[test]
    fn corrupt_file_counts_as_empty() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("ui-prefs.json"), "{non json").unwrap();
        assert!(load(dir.path()).is_empty());
        save(
            dir.path(),
            UiPrefs::from([("photo-cat.floating".into(), "{}".into())]),
        )
        .unwrap();
        assert_eq!(load(dir.path()).len(), 1);
    }

    #[test]
    fn oversized_values_are_dropped() {
        let dir = tempfile::tempdir().unwrap();
        save(
            dir.path(),
            UiPrefs::from([("photo-cat.layout".into(), "x".repeat(MAX_VALUE + 1))]),
        )
        .unwrap();
        assert!(load(dir.path()).is_empty());
    }
}
