//! In-camera RAW conversion profile (device property 0xD185).
//!
//! Layout (fujifilm-ptp-reference): u16 parameter count, u8 IOPCode length,
//! UCS-2 IOPCode, zero padding up to 0x1FD, then little-endian i32 cells.
//! Cell `i` lives at `0x1FD + 4 * i`. Everything we do not understand is kept
//! byte-for-byte so a read → patch → write cycle only touches patched cells.

use super::ptp::{PtpError, PtpResult};

pub const CELLS_OFFSET: usize = 0x1FD;

/// Profile cell indexes (FilmKit / fujifilm-ptp-reference numbering).
pub mod cell {
    pub const FILE_TYPE: usize = 2;
    pub const IMAGE_SIZE: usize = 3;
    pub const DYNAMIC_RANGE: usize = 6;
    /// 0 off, 1 auto, 2 weak, 3 strong (verified via MakerNotes 0x1443/0x1444).
    pub const DR_PRIORITY: usize = 7;
    pub const FILM_SIMULATION: usize = 8;
    pub const GRAIN: usize = 9;
    pub const COLOR_CHROME: usize = 10;
    pub const WB_SOURCE: usize = 11;
    pub const WB_MODE: usize = 12;
    pub const WB_SHIFT_R: usize = 13;
    pub const WB_SHIFT_B: usize = 14;
    pub const WB_KELVIN: usize = 15;
    pub const HIGHLIGHT_TONE: usize = 16;
    pub const SHADOW_TONE: usize = 17;
    pub const COLOR: usize = 18;
    pub const SHARPNESS: usize = 19;
    pub const NOISE_REDUCTION: usize = 20;
    pub const COLOR_SPACE: usize = 22;
    pub const MONO_WARM_COOL: usize = 23;
    /// Not verified on output yet (all X-T5 banks tested had it off).
    pub const SMOOTH_SKIN: usize = 24;
    /// Verified on the X-T5 via MakerNotes ColorChromeFXBlue.
    pub const COLOR_CHROME_FX_BLUE: usize = 25;
    pub const MONO_MAGENTA_GREEN: usize = 26;
    pub const CLARITY: usize = 27;
}

pub const FILE_TYPE_JPEG: i32 = 0x7;
/// Cell 11 value meaning "use the white balance in cell 12".
pub const WB_SOURCE_PROFILE: i32 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    header: Vec<u8>,
    cells: Vec<i32>,
    trailing: Vec<u8>,
}

impl Profile {
    pub fn parse(data: &[u8]) -> PtpResult<Self> {
        if data.len() < CELLS_OFFSET + 4 * (cell::CLARITY + 1) {
            return Err(PtpError::Protocol(format!(
                "profilo di conversione troppo corto: {} byte",
                data.len()
            )));
        }
        let iop_chars = data[2] as usize;
        if 3 + iop_chars * 2 > CELLS_OFFSET {
            return Err(PtpError::Protocol("IOPCode del profilo non valido".into()));
        }
        let body = &data[CELLS_OFFSET..];
        let (words, trailing) = body.as_chunks::<4>();
        let cells = words.iter().map(|b| i32::from_le_bytes(*b)).collect();
        let trailing = trailing.to_vec();
        Ok(Self {
            header: data[..CELLS_OFFSET].to_vec(),
            cells,
            trailing,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = self.header.clone();
        for value in &self.cells {
            out.extend_from_slice(&value.to_le_bytes());
        }
        out.extend_from_slice(&self.trailing);
        out
    }

    pub fn declared_params(&self) -> u16 {
        u16::from_le_bytes([self.header[0], self.header[1]])
    }

    /// Image processor identifier, e.g. "FF179502" on the X-T5.
    pub fn iop_code(&self) -> String {
        let chars = self.header[2] as usize;
        let units: Vec<u16> = (0..chars)
            .map(|i| u16::from_le_bytes([self.header[3 + 2 * i], self.header[4 + 2 * i]]))
            .collect();
        String::from_utf16_lossy(&units)
            .trim_end_matches('\0')
            .to_string()
    }

    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    pub fn get(&self, index: usize) -> Option<i32> {
        self.cells.get(index).copied()
    }

    pub fn set(&mut self, index: usize, value: i32) -> PtpResult<()> {
        let slot = self
            .cells
            .get_mut(index)
            .ok_or_else(|| PtpError::Protocol(format!("cella {index} assente nel profilo")))?;
        *slot = value;
        Ok(())
    }

    pub fn cells(&self) -> &[i32] {
        &self.cells
    }

    /// Build a synthetic profile (tests and the simulated camera).
    pub fn synthetic(iop: &str, cells: &[i32]) -> Self {
        let units: Vec<u16> = iop.encode_utf16().chain(std::iter::once(0)).collect();
        let mut header = Vec::with_capacity(CELLS_OFFSET);
        header.extend_from_slice(&(cells.len() as u16).to_le_bytes());
        header.push(units.len() as u8);
        for unit in units {
            header.extend_from_slice(&unit.to_le_bytes());
        }
        header.resize(CELLS_OFFSET, 0);
        Self {
            header,
            cells: cells.to_vec(),
            trailing: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        let cells: Vec<i32> = (0..29).map(|i| i * 10 - 30).collect();
        Profile::synthetic("FF179502", &cells).to_bytes()
    }

    #[test]
    fn round_trip_is_byte_identical() {
        let bytes = sample();
        assert_eq!(bytes.len(), 625);
        let profile = Profile::parse(&bytes).unwrap();
        assert_eq!(profile.to_bytes(), bytes);
        assert_eq!(profile.iop_code(), "FF179502");
        assert_eq!(profile.cell_count(), 29);
        assert_eq!(profile.declared_params(), 29);
    }

    #[test]
    fn round_trip_keeps_trailing_bytes() {
        let mut bytes = sample();
        bytes.extend_from_slice(&[1, 2, 3]);
        assert_eq!(Profile::parse(&bytes).unwrap().to_bytes(), bytes);
    }

    #[test]
    fn real_fixture_round_trip_when_available() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/xt5_profile_asshot.bin"
        );
        if let Ok(bytes) = std::fs::read(path) {
            let profile = Profile::parse(&bytes).unwrap();
            assert_eq!(profile.to_bytes(), bytes);
        }
    }

    #[test]
    fn patch_changes_only_one_cell() {
        let bytes = sample();
        let mut profile = Profile::parse(&bytes).unwrap();
        profile.set(cell::FILM_SIMULATION, 11).unwrap();
        let patched = profile.to_bytes();
        let offset = CELLS_OFFSET + 4 * cell::FILM_SIMULATION;
        assert_eq!(&patched[offset..offset + 4], &11i32.to_le_bytes());
        let changed: Vec<usize> = (0..bytes.len())
            .filter(|i| bytes[*i] != patched[*i])
            .collect();
        assert!(changed.iter().all(|i| (offset..offset + 4).contains(i)));
    }

    #[test]
    fn rejects_short_or_invalid_profiles() {
        assert!(Profile::parse(&[0u8; 100]).is_err());
        let mut bytes = sample();
        bytes[2] = 255;
        assert!(Profile::parse(&bytes).is_err());
        let mut profile = Profile::parse(&sample()).unwrap();
        assert!(profile.set(99, 1).is_err());
    }
}
