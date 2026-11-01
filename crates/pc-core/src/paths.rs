//! Directory dei dati dell'applicazione (DB, miniature, modelli, log, config).

use crate::error::{AppError, AppResult};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    root: PathBuf,
    /// Cartella dell'archivio dell'app (`app.db`): non segue `data_dir`.
    app_dir: PathBuf,
}

impl AppPaths {
    /// Directory dati standard dell'utente (es. `%APPDATA%\photo-cat\data` su Windows).
    /// La variabile d'ambiente `PHOTO_CAT_DATA_DIR` la sostituisce (installazioni portabili, prove).
    pub fn platform_default() -> AppResult<Self> {
        Self::resolve(std::env::var_os("PHOTO_CAT_DATA_DIR"))
    }

    fn resolve(env_dir: Option<std::ffi::OsString>) -> AppResult<Self> {
        if let Some(dir) = env_dir.filter(|d| !d.is_empty()) {
            return Ok(Self::at(dir));
        }
        let dirs = directories::ProjectDirs::from("com", "photocat", "photo-cat")
            .ok_or_else(|| AppError::internal("impossibile determinare la directory utente"))?;
        Self::with_app_dir(dirs.data_dir(), dirs.config_dir()).configured()
    }

    fn with_app_dir(root: impl AsRef<Path>, app_dir: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            app_dir: app_dir.as_ref().to_path_buf(),
        }
    }

    /// Segue le preferenze della directory dati senza spostare file esistenti.
    /// La cartella dell'app resta quella di partenza.
    pub fn configured(self) -> AppResult<Self> {
        let mut current = self;
        let mut visited = std::collections::HashSet::new();
        for _ in 0..8 {
            if !visited.insert(current.root.clone()) {
                return Err(AppError::invalid_input("ciclo nel percorso dati"));
            }
            let next = crate::Config::load(&current.config_file())
                .ok()
                .and_then(|c| c.data_dir);
            match next {
                Some(p) if p != current.root => current = current.with_root(p),
                _ => return Ok(current),
            }
        }
        Err(AppError::invalid_input(
            "troppi reindirizzamenti della directory dati",
        ))
    }

    /// Directory dati in un percorso scelto (impostazioni o test): l'archivio
    /// dell'app sta in `<radice>/app/`, così le prove restano isolate.
    pub fn at(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();
        Self {
            app_dir: root.join("app"),
            root,
        }
    }

    /// Stessa cartella dell'app, radice dati diversa.
    pub fn with_root(&self, root: impl AsRef<Path>) -> Self {
        Self::with_app_dir(root, &self.app_dir)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn database(&self) -> PathBuf {
        self.root.join("catalog.db")
    }
    pub fn thumbnails(&self) -> PathBuf {
        self.root.join("thumbs")
    }
    pub fn models(&self) -> PathBuf {
        self.root.join("models")
    }
    pub fn logs(&self) -> PathBuf {
        self.root.join("logs")
    }
    pub fn config_file(&self) -> PathBuf {
        self.root.join("config.toml")
    }
    /// Cache dei ritagli dei volti (dati biometrici: cancellata con "Elimina dati volti").
    pub fn face_crops(&self) -> PathBuf {
        self.root.join("faces")
    }
    pub fn backups(&self) -> PathBuf {
        self.root.join("backups")
    }
    /// Cartella dell'archivio dell'app (identità, set di tag): cartella di
    /// configurazione della piattaforma, indipendente da `data_dir`.
    pub fn app_dir(&self) -> &Path {
        &self.app_dir
    }
    /// Archivio dell'app `app.db`.
    pub fn app_store(&self) -> PathBuf {
        self.app_dir.join("app.db")
    }

    /// Crea tutte le sottodirectory necessarie.
    pub fn ensure(&self) -> AppResult<()> {
        for dir in [
            self.root.clone(),
            self.thumbnails(),
            self.models(),
            self.logs(),
            self.backups(),
            self.app_dir.clone(),
        ] {
            std::fs::create_dir_all(&dir)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_creates_layout() {
        let tmp = tempfile::tempdir().unwrap();
        let p = AppPaths::at(tmp.path().join("data"));
        p.ensure().unwrap();
        assert!(p.thumbnails().is_dir());
        assert!(p.models().is_dir());
        assert!(p.logs().is_dir());
        assert!(p.app_dir().is_dir());
        assert_eq!(p.database().file_name().unwrap(), "catalog.db");
    }

    #[test]
    fn platform_default_resolves() {
        assert!(AppPaths::platform_default().is_ok());
    }

    #[test]
    fn app_store_ignores_data_dir_redirect() {
        let tmp = tempfile::tempdir().unwrap();
        let first = tmp.path().join("data");
        let second = tmp.path().join("other");
        let config = tmp.path().join("config");
        std::fs::create_dir_all(&first).unwrap();
        let cfg = crate::Config {
            data_dir: Some(second.clone()),
            ..Default::default()
        };
        cfg.save(&first.join("config.toml")).unwrap();

        let p = AppPaths::with_app_dir(&first, &config)
            .configured()
            .unwrap();
        assert_eq!(p.root(), second);
        assert_eq!(p.app_store(), config.join("app.db"));
        assert_eq!(p.with_root(&first).app_store(), config.join("app.db"));
    }

    #[test]
    fn env_data_dir_keeps_app_store_under_root() {
        let tmp = tempfile::tempdir().unwrap();
        let p = AppPaths::resolve(Some(tmp.path().as_os_str().to_owned())).unwrap();
        assert!(p.app_store().starts_with(tmp.path()));
        assert_eq!(p.app_store(), tmp.path().join("app").join("app.db"));
    }
}
