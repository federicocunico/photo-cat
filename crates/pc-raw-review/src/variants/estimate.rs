//! Stime di spazio su disco e di durata di uno sviluppo (porting di `estimate.rs` di FujiView).

use std::collections::VecDeque;
use std::path::Path;
use std::sync::Arc;

/// Stima iniziale di un JPEG Fine a piena risoluzione (40 MP).
pub const DEFAULT_VARIANT_BYTES: u64 = 25 * 1024 * 1024;
/// Spazio che deve restare libero dopo lo sviluppo.
pub const SAFETY_MARGIN_BYTES: u64 = 5 * 1024 * 1024 * 1024;
/// Ogni quante conversioni si ricontrolla lo spazio libero.
pub const DISK_CHECK_EVERY: usize = 20;
/// Stima iniziale, prima che una conversione sia stata cronometrata.
pub const DEFAULT_SECONDS_PER_CONVERSION: f64 = 4.0;
const WINDOW: usize = 20;

/// Come si legge lo spazio libero del volume di un percorso (sostituibile nei test).
pub type FreeSpace = Arc<dyn Fn(&Path) -> Option<u64> + Send + Sync>;

pub fn required_bytes(pending: usize, average_variant_bytes: u64) -> u64 {
    pending as u64 * average_variant_bytes
}

pub fn has_room(free: u64, required: u64) -> bool {
    free >= required.saturating_add(SAFETY_MARGIN_BYTES)
}

/// Byte liberi sul volume di `path` (o del suo antenato esistente più vicino).
pub fn free_bytes(path: &Path) -> Option<u64> {
    let existing = path.ancestors().find(|p| p.exists())?;
    platform_free_bytes(existing)
}

#[cfg(windows)]
fn platform_free_bytes(path: &Path) -> Option<u64> {
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let wide: Vec<u16> = path
        .as_os_str()
        .to_string_lossy()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let mut available = 0u64;
    unsafe { GetDiskFreeSpaceExW(PCWSTR(wide.as_ptr()), Some(&mut available), None, None) }.ok()?;
    Some(available)
}

/// Il volume con il punto di mount più lungo che contiene `path`.
#[cfg(not(windows))]
fn platform_free_bytes(path: &Path) -> Option<u64> {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let target = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    disks
        .list()
        .iter()
        .filter(|d| target.starts_with(d.mount_point()))
        .max_by_key(|d| d.mount_point().as_os_str().len())
        .map(|d| d.available_space())
}

/// Medie mobili della durata di una conversione e della dimensione dei file.
#[derive(Debug, Clone, Default)]
pub struct Meter {
    seconds: VecDeque<f64>,
    total_bytes: u64,
    files: u64,
}

impl Meter {
    pub fn record(&mut self, seconds: f64, bytes: u64) {
        if self.seconds.len() == WINDOW {
            self.seconds.pop_front();
        }
        self.seconds.push_back(seconds);
        self.total_bytes += bytes;
        self.files += 1;
    }

    pub fn seconds_per_conversion(&self) -> f64 {
        if self.seconds.is_empty() {
            DEFAULT_SECONDS_PER_CONVERSION
        } else {
            self.seconds.iter().sum::<f64>() / self.seconds.len() as f64
        }
    }

    pub fn average_bytes(&self) -> u64 {
        self.total_bytes
            .checked_div(self.files)
            .unwrap_or(DEFAULT_VARIANT_BYTES)
    }

    pub fn eta_seconds(&self, remaining: usize) -> u64 {
        (self.seconds_per_conversion() * remaining as f64).round() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn room_requires_safety_margin() {
        let need = required_bytes(1120, DEFAULT_VARIANT_BYTES);
        assert_eq!(need, 1120 * 25 * 1024 * 1024);
        assert!(!has_room(need, need));
        assert!(has_room(need + SAFETY_MARGIN_BYTES, need));
        assert!(!has_room(0, 1));
    }

    #[test]
    fn meter_uses_defaults_then_real_values() {
        let mut meter = Meter::default();
        assert_eq!(meter.average_bytes(), DEFAULT_VARIANT_BYTES);
        assert_eq!(meter.eta_seconds(10), 40);
        meter.record(2.0, 10);
        meter.record(4.0, 30);
        assert_eq!(meter.seconds_per_conversion(), 3.0);
        assert_eq!(meter.average_bytes(), 20);
        assert_eq!(meter.eta_seconds(100), 300);
    }

    #[test]
    fn meter_window_is_bounded() {
        let mut meter = Meter::default();
        for _ in 0..50 {
            meter.record(10.0, 1);
        }
        for _ in 0..WINDOW {
            meter.record(1.0, 1);
        }
        assert_eq!(meter.seconds_per_conversion(), 1.0);
    }

    #[test]
    fn free_bytes_on_existing_ancestor() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("a").join("b");
        // Su Windows lo spazio libero si legge sempre; altrove dipende dai dischi visibili.
        if cfg!(windows) {
            assert!(free_bytes(&missing).unwrap() > 0);
        }
    }
}
