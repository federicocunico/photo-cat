//! Tipi di riga del catalogo, condivisi con ingestione, AI e UI.

use serde::{Deserialize, Serialize};

pub type MediaId = i64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum MediaKind {
    #[default]
    Photo,
}

impl MediaKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Photo => "photo",
        }
    }
    /// Tipi sconosciuti (scritti da versioni future) ricadono su `Photo`.
    pub fn parse(_s: &str) -> Self {
        Self::Photo
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum MediaStatus {
    #[default]
    Ok,
    Missing,
}

impl MediaStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Missing => "missing",
        }
    }
    pub fn parse(s: &str) -> Self {
        if s == "missing" {
            Self::Missing
        } else {
            Self::Ok
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Source {
    pub id: i64,
    pub path: String,
    pub recursive: bool,
    pub include_formats: Vec<String>,
    pub ignore_patterns: Vec<String>,
    pub watch: bool,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct NewSource {
    pub path: String,
    pub recursive: bool,
    pub include_formats: Vec<String>,
    pub ignore_patterns: Vec<String>,
    pub watch: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct NewMedia {
    pub kind: MediaKind,
    pub source_id: Option<i64>,
    pub path: String,
    pub name: String,
    pub ext: String,
    pub size: i64,
    pub mtime: i64,
    pub quick_hash: Option<String>,
    pub content_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct MediaItem {
    pub id: MediaId,
    pub kind: MediaKind,
    pub source_id: Option<i64>,
    pub path: String,
    pub name: String,
    pub ext: String,
    pub size: i64,
    pub mtime: i64,
    pub quick_hash: Option<String>,
    pub content_hash: Option<String>,
    pub status: MediaStatus,
    pub thumb: Option<String>,
    pub added_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct TechInfo {
    pub format: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub dpi_x: Option<f64>,
    pub dpi_y: Option<f64>,
    pub bit_depth: Option<u32>,
    pub color_space: Option<String>,
    pub orientation: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum DateSource {
    ExifOriginal,
    ExifCreate,
    FileSystem,
}

impl DateSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ExifOriginal => "exif_original",
            Self::ExifCreate => "exif_create",
            Self::FileSystem => "file_system",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "exif_original" => Self::ExifOriginal,
            "exif_create" => Self::ExifCreate,
            "file_system" => Self::FileSystem,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct MediaDates {
    /// `AAAA-MM-GG HH:MM:SS`
    pub captured_at: Option<String>,
    pub captured_source: Option<DateSource>,
    pub modified_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ExifInfo {
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub lens: Option<String>,
    pub exposure: Option<String>,
    pub f_number: Option<f64>,
    pub iso: Option<i64>,
    pub focal_length: Option<f64>,
    pub gps_lat: Option<f64>,
    pub gps_lon: Option<f64>,
    /// Tutti i tag letti, come oggetto JSON.
    pub raw: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum LocationSource {
    Gps,
    Directory,
    Manual,
}

impl LocationSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Gps => "gps",
            Self::Directory => "directory",
            Self::Manual => "manual",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "gps" => Self::Gps,
            "manual" => Self::Manual,
            _ => Self::Directory,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Location {
    pub name: Option<String>,
    pub country: Option<String>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub source: LocationSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum CatalogKind {
    Manual,
    Smart,
}

impl CatalogKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Smart => "smart",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Catalog {
    pub id: i64,
    pub name: String,
    pub kind: CatalogKind,
    pub rules: Option<serde_json::Value>,
    pub created_at: i64,
}
