//! Preferenze dello sviluppo: i preset scelti l'ultima volta (spec camera-presets, "Scelta
//! ricordata"). Il file sta nei dati dell'app, mai nella cartella delle foto.

use super::types::{CameraPreset, CameraPresets};
use crate::session::write_json_atomic;
use pc_core::AppResult;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevelopPrefs {
    /// Id dei preset dell'ultimo sviluppo avviato.
    #[serde(default)]
    pub preset_ids: Vec<String>,
}

fn path(data_dir: &Path) -> PathBuf {
    data_dir.join("raw-review").join("develop-prefs.json")
}

/// Le preferenze salvate; un file mancante o illeggibile vale "nessuna scelta".
pub fn load(data_dir: &Path) -> DevelopPrefs {
    std::fs::read(path(data_dir))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

pub fn save(data_dir: &Path, prefs: &DevelopPrefs) -> AppResult<()> {
    write_json_atomic(&path(data_dir), prefs)
}

/// Ricorda i preset dell'ultimo sviluppo.
pub fn remember(data_dir: &Path, preset_ids: &[String]) -> AppResult<()> {
    save(
        data_dir,
        &DevelopPrefs {
            preset_ids: preset_ids.to_vec(),
        },
    )
}

/// Preset da preselezionare: l'ultima scelta (solo quelli che la fotocamera ha ancora); la prima
/// volta, o se non ne resta nessuno, tutti i banchi (e "Come scattato" se non ci sono banchi).
pub fn selection(prefs: &DevelopPrefs, presets: &[CameraPreset]) -> Vec<String> {
    let remembered: Vec<String> = presets
        .iter()
        .filter(|p| prefs.preset_ids.contains(&p.id))
        .map(|p| p.id.clone())
        .collect();
    if !remembered.is_empty() {
        return remembered;
    }
    let banks: Vec<String> = presets
        .iter()
        .filter(|p| p.bank.is_some())
        .map(|p| p.id.clone())
        .collect();
    if banks.is_empty() {
        presets.iter().map(|p| p.id.clone()).collect()
    } else {
        banks
    }
}

/// Elenco dei preset con la preselezione, per il dialogo.
pub fn with_selection(data_dir: &Path, presets: Vec<CameraPreset>) -> CameraPresets {
    let selected_ids = selection(&load(data_dir), &presets);
    CameraPresets {
        presets,
        selected_ids,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preset(id: &str, bank: Option<u8>) -> CameraPreset {
        CameraPreset {
            id: id.into(),
            bank,
            name: id.into(),
            folder_name: id.into(),
            film_simulation: None,
            summary: String::new(),
            unapplied_properties: Vec::new(),
        }
    }

    fn all() -> Vec<CameraPreset> {
        vec![
            preset("asshot", None),
            preset("C1", Some(1)),
            preset("C2", Some(2)),
            preset("C3", Some(3)),
        ]
    }

    #[test]
    fn first_time_every_bank_is_selected() {
        let data = tempfile::tempdir().unwrap();
        let view = with_selection(data.path(), all());
        assert_eq!(view.selected_ids, ["C1", "C2", "C3"]);
        assert_eq!(view.presets.len(), 4);
    }

    #[test]
    fn the_last_choice_is_remembered_and_unknown_ids_dropped() {
        let data = tempfile::tempdir().unwrap();
        remember(
            data.path(),
            &["C1".into(), "C3".into(), "asshot".into(), "C9".into()],
        )
        .unwrap();
        let view = with_selection(data.path(), all());
        assert_eq!(view.selected_ids, ["asshot", "C1", "C3"]);
        // Dopo che la fotocamera ha perso C1 e C3 resta solo "Come scattato".
        let few = vec![preset("asshot", None), preset("C2", Some(2))];
        assert_eq!(selection(&load(data.path()), &few), ["asshot"]);
        // Niente in comune: si torna a tutti i banchi.
        remember(data.path(), &["C7".into()]).unwrap();
        assert_eq!(selection(&load(data.path()), &few), ["C2"]);
    }

    #[test]
    fn without_banks_the_as_shot_variant_is_selected() {
        assert_eq!(
            selection(&DevelopPrefs::default(), &[preset("asshot", None)]),
            ["asshot"]
        );
    }

    #[test]
    fn a_broken_file_means_no_choice() {
        let data = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(data.path().join("raw-review")).unwrap();
        std::fs::write(path(data.path()), b"{not json").unwrap();
        assert_eq!(load(data.path()), DevelopPrefs::default());
    }
}
