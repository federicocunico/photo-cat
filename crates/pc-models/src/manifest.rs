//! Manifest dei modelli: cosa serve, dove scaricarlo, come verificarlo.

use pc_core::{AppError, AppResult, ErrorCode};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Component, Path};

/// Manifest distribuito con l'app (copia di `assets/models/manifest.json`).
pub const BUILTIN: &str = include_str!("../../../assets/models/manifest.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum Role {
    ClassificationImage,
    ClassificationText,
    Tokenizer,
    PersonDetection,
    FaceDetection,
    FaceEmbedding,
    Gazetteer,
}

/// Contratti supportati dagli adattatori Rust. Una combinazione incompatibile
/// viene rifiutata prima di caricare il modello.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum Preprocessing {
    ClipRgb224,
    ClipTokens77,
    ClipBpe,
    YoloxBgr416,
    YunetBgr640,
    SfaceAlignedRgb112,
    GeonamesTsv,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum Postprocessing {
    NormalizedEmbedding512,
    TokenIds,
    PersonNms,
    FaceLandmarksNms,
    NormalizedEmbedding128,
    CityIndex,
}

impl Role {
    /// Il modello è eseguito da ONNX Runtime (tokenizer e gazetteer no).
    pub fn uses_onnx(self) -> bool {
        !matches!(self, Self::Tokenizer | Self::Gazetteer)
    }

    pub fn processing(self) -> (Preprocessing, Postprocessing) {
        use Postprocessing as Post;
        use Preprocessing as Pre;
        match self {
            Self::ClassificationImage => (Pre::ClipRgb224, Post::NormalizedEmbedding512),
            Self::ClassificationText => (Pre::ClipTokens77, Post::NormalizedEmbedding512),
            Self::Tokenizer => (Pre::ClipBpe, Post::TokenIds),
            Self::PersonDetection => (Pre::YoloxBgr416, Post::PersonNms),
            Self::FaceDetection => (Pre::YunetBgr640, Post::FaceLandmarksNms),
            Self::FaceEmbedding => (Pre::SfaceAlignedRgb112, Post::NormalizedEmbedding128),
            Self::Gazetteer => (Pre::GeonamesTsv, Post::CityIndex),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
    pub role: Role,
    pub name: String,
    pub version: String,
    /// Percorso relativo alla cartella dei modelli.
    pub file: String,
    pub size_bytes: u64,
    pub sha256: String,
    /// URL di download in ordine di preferenza (il primo è il principale, gli altri mirror).
    pub urls: Vec<String>,
    /// Memoria stimata a runtime, corretta dopo il caricamento.
    pub memory_mb: u64,
    pub preprocessing: Preprocessing,
    pub postprocessing: Postprocessing,
    /// Può usare la GPU (falso per modelli noti per non funzionare con DirectML).
    #[serde(default = "yes")]
    pub gpu: bool,
    pub license: String,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub version: u32,
    pub entries: Vec<Entry>,
}

impl Manifest {
    pub fn parse(text: &str) -> AppResult<Self> {
        let m: Manifest = serde_json::from_str(text).map_err(|e| {
            AppError::new(
                ErrorCode::InvalidConfig,
                format!("manifest non valido: {e}"),
            )
        })?;
        m.validate()?;
        Ok(m)
    }

    pub fn builtin() -> Self {
        Self::parse(BUILTIN).expect("manifest incluso valido")
    }

    pub fn validate(&self) -> AppResult<()> {
        let bad = |m: String| Err(AppError::new(ErrorCode::InvalidConfig, m));
        let mut ids = HashSet::new();
        let mut files = HashSet::new();
        if self.version == 0 || self.entries.is_empty() {
            return bad("versione o modelli del manifest mancanti".into());
        }
        for e in &self.entries {
            if e.id.is_empty()
                || !e
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                || e.name.trim().is_empty()
                || e.version.trim().is_empty()
                || e.license.trim().is_empty()
                || e.memory_mb == 0
            {
                return bad(format!(
                    "{}: metadati del modello incompleti o non validi",
                    e.id
                ));
            }
            if !ids.insert(&e.id) {
                return bad(format!("id duplicato: {}", e.id));
            }
            if e.sha256.len() != 64 || !e.sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
                return bad(format!("{}: sha256 non valido", e.id));
            }
            if e.size_bytes == 0 {
                return bad(format!("{}: dimensione mancante", e.id));
            }
            let p = Path::new(&e.file);
            if e.file.is_empty()
                || e.file.contains(['\\', ':'])
                || p.is_absolute()
                || p.components().any(|c| !matches!(c, Component::Normal(_)))
                || e.file
                    .split('/')
                    .any(|s| s.is_empty() || s == "." || s == "..")
            {
                return bad(format!("{}: percorso non consentito: {}", e.id, e.file));
            }
            if !files.insert(e.file.to_ascii_lowercase()) {
                return bad(format!("{}: percorso duplicato: {}", e.id, e.file));
            }
            if e.urls.iter().any(|u| {
                reqwest::Url::parse(u)
                    .map_or(true, |u| u.scheme() != "https" || u.host_str().is_none())
            }) {
                return bad(format!("{}: solo URL https", e.id));
            }
            if (e.preprocessing, e.postprocessing) != e.role.processing() {
                return bad(format!(
                    "{}: pre/post-processing incompatibile con il ruolo",
                    e.id
                ));
            }
        }
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.id == id)
    }

    pub fn by_role(&self, role: Role) -> Option<&Entry> {
        self.entries.iter().find(|e| e.role == role)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_manifest_is_valid_and_complete() {
        let m = Manifest::builtin();
        for role in [
            Role::ClassificationImage,
            Role::ClassificationText,
            Role::Tokenizer,
            Role::PersonDetection,
            Role::FaceDetection,
            Role::FaceEmbedding,
            Role::Gazetteer,
        ] {
            assert!(m.by_role(role).is_some(), "{role:?} mancante");
        }
        // Ogni file sta sotto il limite GitHub di 100 MB.
        assert!(m.entries.iter().all(|e| e.size_bytes < 100 * 1024 * 1024));
    }

    #[test]
    fn invalid_manifests_are_rejected() {
        let base = Manifest::builtin();
        let mut m = base.clone();
        m.entries[1].id = m.entries[0].id.clone();
        assert!(m.validate().is_err());
        let mut m = base.clone();
        m.entries[0].sha256 = "abc".into();
        assert!(m.validate().is_err());
        let mut m = base.clone();
        m.entries[0].file = "../../etc/passwd".into();
        assert!(m.validate().is_err());
        let mut m = base.clone();
        m.entries[0].urls = vec!["http://insicuro".into()];
        assert!(m.validate().is_err());
        assert!(Manifest::parse("{").is_err());
        for file in [
            "",
            "C:/model.onnx",
            "..\\model.onnx",
            "x/../model.onnx",
            "./m.onnx",
            "x//m.onnx",
        ] {
            let mut m = base.clone();
            m.entries[0].file = file.into();
            assert!(m.validate().is_err(), "{file}");
        }
        let mut m = base.clone();
        m.entries[1].file = m.entries[0].file.to_uppercase();
        assert!(m.validate().is_err());
        let mut m = base.clone();
        m.entries[0].postprocessing = Postprocessing::TokenIds;
        assert!(m.validate().is_err());
        let mut m = base.clone();
        m.entries[0].memory_mb = 0;
        assert!(m.validate().is_err());
        let mut m = base.clone();
        m.entries[0].id = "../marker".into();
        assert!(m.validate().is_err());
        let mut m = base.clone();
        m.version = 0;
        assert!(m.validate().is_err());
        let text = serde_json::to_string(&base).unwrap();
        assert_eq!(Manifest::parse(&text).unwrap(), base);
    }
}
