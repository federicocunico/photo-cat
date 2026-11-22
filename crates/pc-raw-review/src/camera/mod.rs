//! Collegamento alla fotocamera Fujifilm: PTP sopra Windows Portable Devices
//! (porting di `src-tauri/src/camera/` di FujiView, commit a798987).
//!
//! Struttura e `cfg`:
//! - `ptp`, `profile`, `banks`, `fuji`: protocollo, profilo di conversione,
//!   banchi custom e flusso di conversione. Non usano API di sistema e si
//!   compilano e si testano su ogni piattaforma (Linux compreso).
//! - `wpd`: trasporto reale, solo `cfg(windows)`.
//! - `sim`: fotocamera simulata (sostituisce `FUJIVIEW_FAKE_CAMERA`); presente
//!   con la feature `fake-camera` su ogni piattaforma e nei test del crate,
//!   dove serve ai test di `fuji`.
//!
//! Su Linux e macOS non esiste quindi un trasporto reale: lo sviluppo dalla
//! fotocamera è disponibile solo con il simulatore (vedi spec camera-connection).

pub mod banks;
pub mod fuji;
pub mod profile;
pub mod ptp;
#[cfg(any(test, feature = "fake-camera"))]
pub mod sim;
#[cfg(windows)]
pub mod wpd;

/// Identificativo USB del produttore Fujifilm.
pub const FUJIFILM_USB_VID: &str = "VID_04CB";
