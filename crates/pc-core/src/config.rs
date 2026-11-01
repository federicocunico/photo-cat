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
    /// Applicazioni esterne per "Apri con" e doppio clic.
    pub external_apps: Vec<ExternalApp>,
}

/// Applicazione esterna configurata dall'utente. Negli argomenti `{file}`
/// indica il percorso della foto; senza `{file}` i percorsi vengono accodati.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ExternalApp {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub args: Vec<String>,
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
    /// Nomi di cartella da ignorare nella deduzione del luogo dal percorso.
    pub ignored_folder_names: Vec<String>,
    pub thresholds: AiThresholds,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            execution_provider: ExecutionProvider::Cpu,
            confidence_threshold: 0.25,
            faces_enabled: true,
            face_min_size_px: 40,
            auto_download_models: true,
            ignored_folder_names: Vec::new(),
            thresholds: AiThresholds::default(),
        }
    }
}

/// Soglie dei modelli applicate ai risultati salvati (0..1).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct AiThresholds {
    /// Confidenza minima di una rilevazione di persona (YOLO).
    pub person_detection: f32,
    /// Confidenza minima di un volto usato per il riconoscimento.
    pub face_detection: f32,
    /// Somiglianza coseno minima tra due volti della stessa persona.
    pub face_similarity: f32,
}

impl AiThresholds {
    pub const DEFAULT_PERSON_DETECTION: f32 = 0.4;
    pub const DEFAULT_FACE_DETECTION: f32 = 0.8;
    pub const DEFAULT_FACE_SIMILARITY: f32 = 0.363;
}

impl Default for AiThresholds {
    fn default() -> Self {
        Self {
            person_detection: Self::DEFAULT_PERSON_DETECTION,
            face_detection: Self::DEFAULT_FACE_DETECTION,
            face_similarity: Self::DEFAULT_FACE_SIMILARITY,
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

/// Azione del doppio clic su una miniatura: `system` (app predefinita),
/// `none` oppure l'`id` di un'applicazione esterna.
pub const DOUBLE_CLICK_SYSTEM: &str = "system";
pub const DOUBLE_CLICK_NONE: &str = "none";

fn default_double_click() -> String {
    DOUBLE_CLICK_SYSTEM.into()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct UiConfig {
    pub theme: Theme,
    pub language: Language,
    /// `system`, `none` o l'id di un'applicazione esterna.
    pub double_click: String,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            language: Language::default(),
            double_click: default_double_click(),
        }
    }
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
        let t = &self.ai.thresholds;
        for (name, v) in [
            ("person_detection", t.person_detection),
            ("face_detection", t.face_detection),
            ("face_similarity", t.face_similarity),
        ] {
            if !(0.0..=1.0).contains(&v) {
                return bad(format!(
                    "ai.thresholds.{name} deve essere tra 0 e 1 (trovato {v})"
                ));
            }
        }
        let mut ids = std::collections::HashSet::new();
        for app in &self.external_apps {
            let id = app.id.trim();
            if id.is_empty() || app.name.trim().is_empty() {
                return bad("external_apps: id e nome sono obbligatori".into());
            }
            if id == DOUBLE_CLICK_SYSTEM || id == DOUBLE_CLICK_NONE {
                return bad(format!("external_apps: id riservato \"{id}\""));
            }
            if !ids.insert(id) {
                return bad(format!("external_apps: id duplicato \"{id}\""));
            }
        }
        let dc = self.ui.double_click.as_str();
        if dc != DOUBLE_CLICK_SYSTEM && dc != DOUBLE_CLICK_NONE && !ids.contains(dc) {
            return bad(format!(
                "ui.double_click punta a un'applicazione inesistente \"{dc}\""
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
            "[ai.thresholds]\nperson_detection = 1.5\n",
            "[ai.thresholds]\nface_similarity = -0.1\n",
            "[ui]\ndouble_click = \"lightroom\"\n",
            "[[external_apps]]\nid = \"a\"\nname = \"A\"\npath = \"/a\"\n[[external_apps]]\nid = \"a\"\nname = \"B\"\npath = \"/b\"\n",
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
    fn old_config_loads_with_new_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("config.toml");
        std::fs::write(
            &p,
            "[ai]\nconfidence_threshold = 0.3\nfaces_enabled = true\n[ui]\ntheme = \"light\"\n",
        )
        .unwrap();
        let cfg = Config::load(&p).unwrap();
        assert_eq!(cfg.ai.thresholds, AiThresholds::default());
        assert_eq!(cfg.ai.thresholds.person_detection, 0.4);
        assert_eq!(cfg.ai.thresholds.face_detection, 0.8);
        assert_eq!(cfg.ai.thresholds.face_similarity, 0.363);
        assert!(cfg.ai.ignored_folder_names.is_empty());
        assert!(cfg.external_apps.is_empty());
        assert_eq!(cfg.ui.double_click, "system");
    }

    #[test]
    fn external_apps_and_double_click_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("config.toml");
        std::fs::write(
            &p,
            "[ai]\nignored_folder_names = [\"Selezionate\"]\n[ai.thresholds]\nface_detection = 0.7\n\
             [ui]\ndouble_click = \"lr\"\n\
             [[external_apps]]\nid = \"lr\"\nname = \"Lightroom\"\npath = \"C:/LR/Lightroom.exe\"\nargs = [\"--edit\", \"{file}\"]\n",
        )
        .unwrap();
        let cfg = Config::load(&p).unwrap();
        assert_eq!(cfg.external_apps[0].args, vec!["--edit", "{file}"]);
        assert_eq!(cfg.ai.thresholds.face_detection, 0.7);
        assert_eq!(cfg.ai.ignored_folder_names, vec!["Selezionate"]);
        cfg.save(&p).unwrap();
        assert_eq!(Config::load(&p).unwrap(), cfg);
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
