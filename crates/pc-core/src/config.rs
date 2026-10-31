//! Configurazione persistita in `config.toml`.
//!
//! Ogni campo ha un default compilato: un file mancante o parziale è valido.
//! Valori fuori dai limiti sono un errore `INVALID_CONFIG`.

use crate::error::{AppError, AppResult, ErrorCode};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Config {
    pub resources: ResourcesConfig,
    pub ai: AiConfig,
    pub logging: LoggingConfig,
    pub ui: UiConfig,
    /// Percorso dati alternativo scelto dall'utente.
    pub data_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum ResourceProfile {
    Light,
    #[default]
    Balanced,
    Performance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ResourcesConfig {
    pub profile: ResourceProfile,
    /// Limite massimo manuale di RAM per l'app, in MB.
    pub ram_limit_mb: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum ExecutionProvider {
    Auto,
    #[default]
    Cpu,
    DirectMl,
    Cuda,
    CoreMl,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct AiConfig {
    pub execution_provider: ExecutionProvider,
    /// Soglia di confidenza predefinita per le catalogazioni (0..1).
    pub confidence_threshold: f32,
    pub faces_enabled: bool,
    /// Lato minimo in pixel di un volto utilizzabile per il riconoscimento.
    pub face_min_size_px: u32,
    pub auto_download_models: bool,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            execution_provider: ExecutionProvider::Cpu,
            confidence_threshold: 0.25,
            faces_enabled: true,
            face_min_size_px: 40,
            auto_download_models: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum LogLevel {
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
            Self::Trace => "trace",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct LoggingConfig {
    pub level: LogLevel,
    /// Numero massimo di file di log conservati.
    pub max_files: usize,
    /// Dimensione massima di un singolo file di log, in MB.
    pub max_file_mb: u64,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            max_files: 14,
            max_file_mb: 20,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum Language {
    #[default]
    It,
    En,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct UiConfig {
    pub theme: Theme,
    pub language: Language,
}

impl Config {
    /// Carica la configurazione; un file assente restituisce i default.
    pub fn load(path: &Path) -> AppResult<Self> {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e.into()),
        };
        let cfg: Config = toml::from_str(&text)?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Salva in modo atomico (file temporaneo + rename).
    pub fn save(&self, path: &Path) -> AppResult<()> {
        self.validate()?;
        let text = toml::to_string_pretty(self)
            .map_err(|e| AppError::new(ErrorCode::InvalidConfig, e.to_string()))?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, text)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    pub fn validate(&self) -> AppResult<()> {
        let bad = |m: String| Err(AppError::new(ErrorCode::InvalidConfig, m));
        if self.data_dir.as_ref().is_some_and(|p| !p.is_absolute()) {
            return bad("data_dir deve essere assoluto".into());
        }
        if let Some(limit) = self.resources.ram_limit_mb {
            if limit < 512 {
                return bad(format!(
                    "resources.ram_limit_mb deve essere >= 512 (trovato {limit})"
                ));
            }
        }
        if !(0.0..=1.0).contains(&self.ai.confidence_threshold) {
            return bad(format!(
                "ai.confidence_threshold deve essere tra 0 e 1 (trovato {})",
                self.ai.confidence_threshold
            ));
        }
        if self.ai.face_min_size_px < 8 {
            return bad("ai.face_min_size_px deve essere >= 8".into());
        }
        if self.logging.max_files == 0 || self.logging.max_file_mb == 0 {
            return bad("logging.max_files e logging.max_file_mb devono essere > 0".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_yields_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = Config::load(&tmp.path().join("config.toml")).unwrap();
        assert_eq!(cfg, Config::default());
        assert_eq!(cfg.resources.profile, ResourceProfile::Balanced);
    }

    #[test]
    fn partial_file_fills_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("config.toml");
        std::fs::write(
            &p,
            "[resources]\nram_limit_mb = 4096\n[ui]\nlanguage = \"en\"\n",
        )
        .unwrap();
        let cfg = Config::load(&p).unwrap();
        assert_eq!(cfg.resources.ram_limit_mb, Some(4096));
        assert_eq!(cfg.resources.profile, ResourceProfile::Balanced);
        assert_eq!(cfg.ui.language, Language::En);
        assert_eq!(cfg.logging, LoggingConfig::default());
    }

    #[test]
    fn invalid_values_are_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("config.toml");
        for bad in [
            "[ai]\nconfidence_threshold = 1.5\n",
            "[resources]\nram_limit_mb = 10\n",
            "[resources]\nprofile = \"turbo\"\n",
            "not toml at all = = =",
            "[unknown]\nx = 1\n",
        ] {
            std::fs::write(&p, bad).unwrap();
            assert_eq!(
                Config::load(&p).unwrap_err().code,
                ErrorCode::InvalidConfig,
                "{bad}"
            );
        }
    }

    #[test]
    fn save_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("sub").join("config.toml");
        let mut cfg = Config::default();
        cfg.resources.profile = ResourceProfile::Light;
        cfg.logging.level = LogLevel::Debug;
        cfg.save(&p).unwrap();
        assert_eq!(Config::load(&p).unwrap(), cfg);
    }
}
