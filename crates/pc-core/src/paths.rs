//! Directory dei dati dell'applicazione (DB, miniature, modelli, log, config).

use crate::error::{AppError, AppResult};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    root: PathBuf,
}

impl AppPaths {
    /// Directory dati standard dell'utente (es. `%APPDATA%\photo-cat\data` su Windows).
    /// La variabile d'ambiente `PHOTO_CAT_DATA_DIR` la sostituisce (installazioni portabili, prove).
    pub fn platform_default() -> AppResult<Self> {
        if let Some(dir) = std::env::var_os("PHOTO_CAT_DATA_DIR").filter(|d| !d.is_empty()) {
            return Ok(Self::at(dir));
        }
        let dirs = directories::ProjectDirs::from("com", "photocat", "photo-cat")
            .ok_or_else(|| AppError::internal("impossibile determinare la directory utente"))?;
        Self::at(dirs.data_dir()).configured()
    }

    /// Segue le preferenze della directory dati senza spostare file esistenti.
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
                Some(p) if p != current.root => current = Self::at(p),
                _ => return Ok(current),
            }
        }
        Err(AppError::invalid_input(
            "troppi reindirizzamenti della directory dati",
        ))
    }

    /// Directory dati in un percorso scelto (impostazioni o test).
    pub fn at(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
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

    /// Crea tutte le sottodirectory necessarie.
    pub fn ensure(&self) -> AppResult<()> {
        for dir in [
            self.root.clone(),
            self.thumbnails(),
            self.models(),
            self.logs(),
            self.backups(),
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
        assert_eq!(p.database().file_name().unwrap(), "catalog.db");
    }

    #[test]
    fn platform_default_resolves() {
        assert!(AppPaths::platform_default().is_ok());
    }
}
