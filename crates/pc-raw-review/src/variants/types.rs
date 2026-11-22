//! Tipi condivisi con l'interfaccia (porting di `variants/types.rs` di FujiView).
//!
//! Lo stato e l'avanzamento dello sviluppo non sono più qui: passano dagli eventi delle attività
//! (`pc-jobs`). Restano lo stato della fotocamera, i preset, il piano e il riepilogo finale.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum CameraState {
    Disconnected,
    NotReady,
    Busy,
    Ready,
    Error,
    /// Una chiamata alla fotocamera è bloccata (fotocamera congelata): si attende che torni.
    Unresponsive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct CameraStatus {
    /// `false` dove lo sviluppo dalla fotocamera non esiste (Linux e macOS, senza simulatore):
    /// `message` dice "Disponibile solo su Windows".
    pub available: bool,
    pub state: CameraState,
    pub model: Option<String>,
    pub firmware: Option<String>,
    pub message: Option<String>,
    pub simulated: bool,
}

impl CameraStatus {
    pub fn new(state: CameraState, message: impl Into<Option<String>>) -> Self {
        Self {
            available: true,
            state,
            model: None,
            firmware: None,
            message: message.into(),
            simulated: false,
        }
    }

    /// Stato di una piattaforma senza fotocamera.
    pub fn unavailable(reason: &str) -> Self {
        Self {
            available: false,
            ..Self::new(CameraState::Disconnected, Some(reason.to_string()))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct CameraPreset {
    /// `asshot` oppure `C1`..`C7`.
    pub id: String,
    pub bank: Option<u8>,
    pub name: String,
    pub folder_name: String,
    pub film_simulation: Option<String>,
    pub summary: String,
    /// Impostazioni del banco che la conversione in fotocamera non può applicare.
    pub unapplied_properties: Vec<String>,
}

/// Preset della fotocamera e quelli da preselezionare (l'ultima scelta, o tutti i banchi).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct CameraPresets {
    pub presets: Vec<CameraPreset>,
    pub selected_ids: Vec<String>,
}

/// Cosa sviluppare (interno: le sorgenti sono percorsi di RAF già risolti dalla sessione).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariantRequest {
    pub sources: Vec<String>,
    pub output_dir: Option<String>,
    pub preset_ids: Vec<String>,
    pub overwrite: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct PlannedPreset {
    pub id: String,
    pub folder_name: String,
}

/// Quello che farà lo sviluppo: conteggi, spazio e tempo stimati.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct VariantPlan {
    pub source_root: String,
    pub output_dir: String,
    pub raf_count: usize,
    pub presets: Vec<PlannedPreset>,
    /// RAF × preset.
    pub total_conversions: usize,
    /// Conversioni ancora da fare.
    pub pending_conversions: usize,
    /// Varianti già presenti e complete (saltate).
    pub already_done: usize,
    pub estimated_bytes: u64,
    /// `None` se non si riesce a leggere lo spazio libero (non blocca l'avvio).
    pub free_bytes: Option<u64>,
    /// Margine che deve restare libero dopo lo sviluppo.
    pub safety_margin_bytes: u64,
    pub enough_space: bool,
    pub estimated_seconds: u64,
    pub warnings: Vec<String>,
}

/// Un RAF che la fotocamera ha rifiutato o che non si è potuto leggere.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct DevelopFailure {
    /// Nome del file RAF.
    pub raf: String,
    /// Cartelle dei preset non sviluppate per questo RAF.
    pub presets: Vec<String>,
    /// Codice stabile (`errors.<codice>` per il testo localizzato).
    pub code: String,
    pub message_key: String,
    pub reason: String,
}

/// Perché lo sviluppo si è fermato prima della fine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct DevelopStop {
    pub code: String,
    pub message_key: String,
    pub reason: String,
}

/// Riepilogo finale di uno sviluppo (in memoria, per il dialogo).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct DevelopReport {
    pub job_id: i64,
    pub session_id: String,
    pub output_dir: String,
    /// Conversioni scritte in questa esecuzione.
    pub converted: u32,
    /// Varianti già presenti e complete, o già concluse in un'esecuzione precedente.
    pub skipped: u32,
    /// Conversioni non riuscite (un RAF rifiutato conta una per preset).
    pub failed: u32,
    pub failures: Vec<DevelopFailure>,
    pub cancelled: bool,
    /// Errore che ha fermato l'attività (fotocamera bloccata, scrittura rifiutata…).
    pub stopped: Option<DevelopStop>,
}

impl DevelopReport {
    pub fn new(job_id: i64, session_id: &str) -> Self {
        Self {
            job_id,
            session_id: session_id.to_string(),
            output_dir: String::new(),
            converted: 0,
            skipped: 0,
            failed: 0,
            failures: Vec::new(),
            cancelled: false,
            stopped: None,
        }
    }
}
