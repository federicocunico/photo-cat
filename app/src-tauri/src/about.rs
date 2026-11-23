//! Licenze di terze parti per la finestra Informazioni (app-about). Il file
//! `licenses/third-party.json` è generato da `scripts/licenses.mjs` e installato con l'app.

use pc_core::{AppError, AppResult, ErrorCode};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use tauri::AppHandle;

/// Un componente distribuito con l'app.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LicenseComponent {
    pub name: String,
    pub version: String,
    /// Espressione SPDX (es. "MIT OR Apache-2.0").
    pub license: String,
    /// "model", "native", "ui" o "rust".
    pub kind: String,
    /// Chiave del testo in [`ThirdPartyLicenses::texts`].
    pub text: String,
    /// Sorgenti corrispondenti nell'installazione (componenti LGPL), relative a `root`.
    pub sources: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ThirdPartyLicenses {
    pub components: Vec<LicenseComponent>,
    /// Testi delle licenze, una volta sola per testo.
    pub texts: BTreeMap<String, String>,
    /// Cartella dell'installazione che contiene `sources` (riempita dal comando).
    #[serde(default)]
    pub root: String,
}

/// Legge l'elenco delle licenze; `root` è la cartella delle risorse dell'app.
pub fn read_licenses(root: &Path) -> AppResult<ThirdPartyLicenses> {
    let file = root.join("licenses").join("third-party.json");
    let raw = std::fs::read_to_string(&file).map_err(|e| {
        AppError::new(
            ErrorCode::Io,
            format!(
                "Elenco delle licenze non leggibile ({e}): {}",
                file.display()
            ),
        )
    })?;
    let mut list: ThirdPartyLicenses = serde_json::from_str(&raw).map_err(|e| {
        AppError::new(
            ErrorCode::Io,
            format!("Elenco delle licenze non valido ({e}): {}", file.display()),
        )
    })?;
    list.root = root.display().to_string();
    Ok(list)
}

#[tauri::command]
#[specta::specta]
pub async fn third_party_licenses(app: AppHandle) -> AppResult<ThirdPartyLicenses> {
    let root = crate::models::assets_dir(&app);
    tauri::async_runtime::spawn_blocking(move || read_licenses(&root))
        .await
        .map_err(|e| AppError::internal(e.to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legge_un_elenco_di_esempio() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("licenses")).unwrap();
        std::fs::write(
            tmp.path().join("licenses/third-party.json"),
            r#"{"components":[{"name":"libheif","version":"1.23.7","license":"LGPL-3.0-or-later",
               "kind":"native","text":"a1","sources":"native-sources/libheif-1.23.7.tar.gz"}],
               "texts":{"a1":"GNU LESSER GENERAL PUBLIC LICENSE"}}"#,
        )
        .unwrap();
        let list = read_licenses(tmp.path()).unwrap();
        assert_eq!(list.components.len(), 1);
        let c = &list.components[0];
        assert_eq!(c.license, "LGPL-3.0-or-later");
        assert_eq!(
            list.texts.get(&c.text).map(String::as_str),
            Some("GNU LESSER GENERAL PUBLIC LICENSE")
        );
        assert_eq!(list.root, tmp.path().display().to_string());
    }

    #[test]
    fn senza_file_un_errore_che_dice_dove_cercarlo() {
        let tmp = tempfile::tempdir().unwrap();
        let err = read_licenses(tmp.path()).unwrap_err();
        assert!(err.detail.contains("third-party.json"), "{}", err.detail);
    }

    /// Il file del repository (lo stesso dell'installer) si legge e copre tutti i tipi.
    #[test]
    fn il_file_del_repository_e_valido() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let list = read_licenses(&root).unwrap();
        for kind in ["model", "native", "ui", "rust"] {
            assert!(
                list.components.iter().any(|c| c.kind == kind),
                "manca {kind}"
            );
        }
        assert!(list
            .components
            .iter()
            .all(|c| list.texts.contains_key(&c.text)));
        let lgpl: Vec<_> = list
            .components
            .iter()
            .filter(|c| c.license.contains("LGPL") && c.kind == "native")
            .collect();
        assert!(lgpl.len() >= 2 && lgpl.iter().all(|c| c.sources.is_some()));
    }
}
