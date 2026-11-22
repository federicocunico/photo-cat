//! A simulated X-T5 speaking the same PTP dialect as the real camera.
//! Used by unit tests and, with the `fake-camera` feature, for development and demos.
//! Its "conversion" returns the JPEG preview embedded in the uploaded RAF.

use super::banks::Bank;
use super::profile::{cell, Profile};
use super::ptp::{
    encode_string, op, prop, rc, DataPhase, PtpError, PtpResult, PtpTransport, Response,
};
use std::collections::BTreeMap;
use std::time::Duration;

const SIM_IOP: &str = "FF179502";
const TINY_JPEG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xD9];

/// Camera model string stored at offset 0x1C of a RAF header.
pub fn raf_model(raf: &[u8]) -> Option<String> {
    let bytes = raf.get(0x1C..0x3C)?;
    if !raf.starts_with(b"FUJIFILMCCD-RAW") {
        return None;
    }
    Some(
        String::from_utf8_lossy(bytes)
            .trim_end_matches('\0')
            .trim()
            .to_string(),
    )
}

/// JPEG preview embedded in a RAF (offset/length are big-endian at 0x54/0x58).
pub fn raf_embedded_jpeg(raf: &[u8]) -> Option<&[u8]> {
    let be = |at: usize| {
        raf.get(at..at + 4)
            .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as usize)
    };
    let (offset, length) = (be(0x54)?, be(0x58)?);
    let jpeg = raf.get(offset..offset.checked_add(length)?)?;
    jpeg.starts_with(&[0xFF, 0xD8]).then_some(jpeg)
}

/// Minimal RAF-like file for tests.
pub fn synthetic_raf(model: &str) -> Vec<u8> {
    let mut raf = vec![0u8; 0x100];
    raf[..16].copy_from_slice(b"FUJIFILMCCD-RAW ");
    raf[0x1C..0x1C + model.len()].copy_from_slice(model.as_bytes());
    raf[0x54..0x58].copy_from_slice(&0x100u32.to_be_bytes());
    raf[0x58..0x5C].copy_from_slice(&(TINY_JPEG.len() as u32).to_be_bytes());
    raf.extend_from_slice(TINY_JPEG);
    raf
}

fn sim_banks() -> Vec<Bank> {
    let films = [11, 17, 2, 19, 12, 16, 20];
    (1..=7u8)
        .map(|index| {
            let mut props = BTreeMap::new();
            for code in prop::CUSTOM_SETTING_FIRST..=prop::CUSTOM_SETTING_LAST {
                props.insert(code, 0);
            }
            props.insert(0xD190, 100);
            props.insert(0xD192, films[index as usize - 1]);
            props.insert(0xD195, (index % 3) as i32);
            Bank {
                index,
                name: format!("Sim C{index}"),
                props,
            }
        })
        .collect()
}

pub struct SimulatedCamera {
    pub model: String,
    pub banks: Vec<Bank>,
    pub selected_bank: u16,
    pub bank_writes: usize,
    pub objects: BTreeMap<u32, Vec<u8>>,
    pub conversions: usize,
    pub uploads: usize,
    pub conversion_delay: Duration,
    /// Never produce an output (timeout tests).
    pub hang_conversions: bool,
    /// Answer 0x2002 when this property is read (failure tests).
    pub fail_prop: Option<u16>,
    /// Answer 0x2019 to the next N conversion starts.
    pub busy_starts: usize,
    /// Report a disconnection once this many conversions have completed.
    pub disconnect_after: Option<usize>,
    /// Reject a second conversion after one upload (D4 fallback tests).
    pub single_conversion_per_upload: bool,
    /// Value answered for the USB mode property (6 = RAW CONV.).
    pub usb_mode: u32,
    /// When false the DeviceInfo hides the conversion operations/properties.
    pub advertise_conversion: bool,
    disconnected: bool,
    raf: Option<Vec<u8>>,
    profile: Option<Profile>,
    converted_since_upload: usize,
    next_handle: u32,
}

impl Default for SimulatedCamera {
    fn default() -> Self {
        Self::new()
    }
}

impl SimulatedCamera {
    pub fn new() -> Self {
        Self {
            model: "X-T5".into(),
            banks: sim_banks(),
            selected_bank: 1,
            bank_writes: 0,
            objects: BTreeMap::new(),
            conversions: 0,
            uploads: 0,
            conversion_delay: Duration::ZERO,
            hang_conversions: false,
            fail_prop: None,
            busy_starts: 0,
            disconnect_after: None,
            single_conversion_per_upload: false,
            usb_mode: prop::USB_MODE_RAW_CONV,
            advertise_conversion: true,
            disconnected: false,
            raf: None,
            profile: None,
            converted_since_upload: 0,
            next_handle: 1,
        }
    }

    pub fn reconnect(&mut self) {
        self.disconnected = false;
        self.disconnect_after = None;
    }

    fn base_profile() -> Profile {
        let mut cells = vec![0i32; 29];
        cells[cell::FILE_TYPE] = 7;
        cells[cell::IMAGE_SIZE] = 1;
        cells[cell::DYNAMIC_RANGE] = 100;
        cells[cell::FILM_SIMULATION] = 1;
        cells[cell::WB_SOURCE] = 1;
        Profile::synthetic(SIM_IOP, &cells)
    }

    fn device_info(&self) -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(&100u16.to_le_bytes());
        d.extend_from_slice(&0x0000_000Eu32.to_le_bytes());
        d.extend_from_slice(&100u16.to_le_bytes());
        d.extend(encode_string("fujifilm.co.jp: 1.0;"));
        d.extend_from_slice(&0u16.to_le_bytes());
        let mut ops = vec![
            op::GET_DEVICE_INFO,
            op::GET_OBJECT_HANDLES,
            op::GET_OBJECT,
            op::DELETE_OBJECT,
            op::GET_DEVICE_PROP_VALUE,
            op::SET_DEVICE_PROP_VALUE,
            op::FUJI_SEND_OBJECT_INFO,
            op::FUJI_SEND_OBJECT,
        ];
        let mut props = vec![
            prop::START_RAW_CONVERSION,
            prop::RAW_CONVERSION_PROFILE,
            prop::CUSTOM_SETTING_SELECT,
            prop::CUSTOM_SETTING_NAME,
        ];
        props.extend(prop::CUSTOM_SETTING_FIRST..=prop::CUSTOM_SETTING_LAST);
        if !self.advertise_conversion {
            ops.retain(|o| *o != op::FUJI_SEND_OBJECT_INFO && *o != op::FUJI_SEND_OBJECT);
            props
                .retain(|p| *p != prop::START_RAW_CONVERSION && *p != prop::RAW_CONVERSION_PROFILE);
        }
        for list in [ops, vec![], props, vec![], vec![0x3801]] {
            d.extend_from_slice(&(list.len() as u32).to_le_bytes());
            for v in list {
                d.extend_from_slice(&v.to_le_bytes());
            }
        }
        for s in ["FUJIFILM", self.model.as_str(), "SIM", "0000"] {
            d.extend(encode_string(s));
        }
        d
    }

    fn code(code: u16) -> PtpResult<Response> {
        Ok(Response {
            code,
            ..Default::default()
        })
    }

    fn data(data: Vec<u8>) -> PtpResult<Response> {
        Ok(Response {
            code: rc::OK,
            params: Vec::new(),
            data,
        })
    }

    fn raf_accepted(&self) -> bool {
        self.raf
            .as_deref()
            .and_then(raf_model)
            .is_some_and(|m| m == self.model)
    }

    fn get_prop(&self, code: u16) -> PtpResult<Response> {
        if self.fail_prop == Some(code) {
            return Self::code(rc::GENERAL_ERROR);
        }
        let bank = self
            .banks
            .get((self.selected_bank as usize).wrapping_sub(1));
        match code {
            prop::USB_MODE => Self::data((self.usb_mode as u16).to_le_bytes().to_vec()),
            prop::CUSTOM_SETTING_SELECT => Self::data(self.selected_bank.to_le_bytes().to_vec()),
            prop::CUSTOM_SETTING_NAME => {
                Self::data(encode_string(bank.map(|b| b.name.as_str()).unwrap_or("")))
            }
            prop::CUSTOM_SETTING_FIRST..=prop::CUSTOM_SETTING_LAST => {
                match bank.and_then(|b| b.props.get(&code)) {
                    Some(value) => Self::data((*value as i16).to_le_bytes().to_vec()),
                    None => Self::code(rc::DEVICE_PROP_NOT_SUPPORTED),
                }
            }
            prop::RAW_CONVERSION_PROFILE if self.raf_accepted() => Self::data(
                self.profile
                    .clone()
                    .unwrap_or_else(Self::base_profile)
                    .to_bytes(),
            ),
            prop::RAW_CONVERSION_PROFILE => Self::code(rc::GENERAL_ERROR),
            _ => Self::code(rc::DEVICE_PROP_NOT_SUPPORTED),
        }
    }

    fn set_prop(&mut self, code: u16, value: &[u8]) -> PtpResult<Response> {
        let as_u16 = || value.get(..2).map(|b| u16::from_le_bytes([b[0], b[1]]));
        match code {
            prop::CUSTOM_SETTING_SELECT => match as_u16() {
                Some(index) if (1..=self.banks.len() as u16).contains(&index) => {
                    self.selected_bank = index;
                    Self::code(rc::OK)
                }
                _ => Self::code(rc::INVALID_DEVICE_PROP_VALUE),
            },
            prop::RAW_CONVERSION_PROFILE => {
                if !self.raf_accepted() {
                    return Self::code(rc::GENERAL_ERROR);
                }
                match Profile::parse(value) {
                    Ok(profile) => {
                        self.profile = Some(profile);
                        Self::code(rc::OK)
                    }
                    Err(_) => Self::code(rc::INVALID_DEVICE_PROP_VALUE),
                }
            }
            prop::START_RAW_CONVERSION => {
                if self.busy_starts > 0 {
                    self.busy_starts -= 1;
                    return Self::code(rc::DEVICE_BUSY);
                }
                if !self.raf_accepted()
                    || (self.single_conversion_per_upload && self.converted_since_upload > 0)
                {
                    return Self::code(rc::GENERAL_ERROR);
                }
                if self.hang_conversions {
                    return Self::code(rc::OK);
                }
                if !self.conversion_delay.is_zero() {
                    std::thread::sleep(self.conversion_delay);
                }
                let jpeg = self
                    .raf
                    .as_deref()
                    .and_then(raf_embedded_jpeg)
                    .unwrap_or(TINY_JPEG)
                    .to_vec();
                self.objects.insert(self.next_handle, jpeg);
                self.next_handle += 1;
                self.conversions += 1;
                self.converted_since_upload += 1;
                if self.disconnect_after.is_some_and(|n| self.conversions >= n) {
                    self.disconnected = true;
                }
                Self::code(rc::OK)
            }
            prop::CUSTOM_SETTING_NAME | prop::CUSTOM_SETTING_FIRST..=prop::CUSTOM_SETTING_LAST => {
                self.bank_writes += 1;
                Self::code(rc::OK)
            }
            _ => Self::code(rc::DEVICE_PROP_NOT_SUPPORTED),
        }
    }
}

impl PtpTransport for SimulatedCamera {
    fn execute(&mut self, code: u16, params: &[u32], data: DataPhase<'_>) -> PtpResult<Response> {
        if self.disconnected {
            return Err(PtpError::Disconnected);
        }
        let param = |i: usize| params.get(i).copied().unwrap_or(0);
        match (code, data) {
            (op::GET_DEVICE_INFO, _) => Self::data(self.device_info()),
            (op::GET_DEVICE_PROP_VALUE, _) => self.get_prop(param(0) as u16),
            (op::SET_DEVICE_PROP_VALUE, DataPhase::Write(value)) => {
                self.set_prop(param(0) as u16, value)
            }
            (op::FUJI_SEND_OBJECT_INFO, DataPhase::Write(_)) => {
                self.raf = None;
                Self::code(rc::OK)
            }
            (op::FUJI_SEND_OBJECT, DataPhase::Write(bytes)) => {
                self.raf = Some(bytes.to_vec());
                self.profile = None;
                self.converted_since_upload = 0;
                self.uploads += 1;
                Self::code(rc::OK)
            }
            (op::GET_OBJECT_HANDLES, _) => {
                let mut d = (self.objects.len() as u32).to_le_bytes().to_vec();
                for handle in self.objects.keys() {
                    d.extend_from_slice(&handle.to_le_bytes());
                }
                Self::data(d)
            }
            (op::GET_OBJECT, _) => match self.objects.get(&param(0)) {
                Some(bytes) => Self::data(bytes.clone()),
                None => Self::code(0x2009),
            },
            (op::DELETE_OBJECT, _) => match self.objects.remove(&param(0)) {
                Some(_) => Self::code(rc::OK),
                None => Self::code(0x2009),
            },
            _ => Self::code(rc::OPERATION_NOT_SUPPORTED),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_model_and_preview_from_raf() {
        let raf = synthetic_raf("X-T5");
        assert_eq!(raf_model(&raf).as_deref(), Some("X-T5"));
        assert_eq!(raf_embedded_jpeg(&raf), Some(TINY_JPEG));
        assert_eq!(raf_model(b"not a raf"), None);
    }

    #[test]
    fn real_raf_header_when_available() {
        if let Some(Ok(raf)) = std::env::var_os("PHOTOCAT_TEST_RAF").map(std::fs::read) {
            assert_eq!(raf_model(&raf).as_deref(), Some("X-T5"));
            assert!(raf_embedded_jpeg(&raf).is_some());
        }
    }

    #[test]
    fn answers_like_an_xt5_in_raw_conversion_mode() {
        use super::super::ptp::DeviceInfo;
        let mut sim = SimulatedCamera::new();
        let info = DeviceInfo::parse(
            &sim.execute_ok(op::GET_DEVICE_INFO, &[], DataPhase::Read)
                .unwrap()
                .data,
        )
        .unwrap();
        assert_eq!(info.model, "X-T5");
        assert!(info.missing_conversion_support().is_empty());
        let mode = sim
            .execute_ok(
                op::GET_DEVICE_PROP_VALUE,
                &[prop::USB_MODE as u32],
                DataPhase::Read,
            )
            .unwrap();
        assert_eq!(mode.data, (prop::USB_MODE_RAW_CONV as u16).to_le_bytes());
        // Operazione sconosciuta: la camera risponde "non supportata".
        let unknown = sim.execute(0x9999, &[], DataPhase::None).unwrap();
        assert_eq!(unknown.code, rc::OPERATION_NOT_SUPPORTED);
    }

    #[test]
    fn hides_conversion_support_when_not_advertised() {
        use super::super::ptp::DeviceInfo;
        let mut sim = SimulatedCamera::new();
        sim.advertise_conversion = false;
        let data = sim
            .execute_ok(op::GET_DEVICE_INFO, &[], DataPhase::Read)
            .unwrap()
            .data;
        let info = DeviceInfo::parse(&data).unwrap();
        assert!(!info.missing_conversion_support().is_empty());
    }

    #[test]
    fn simulates_busy_and_disconnection() {
        let mut sim = SimulatedCamera::new();
        sim.busy_starts = 1;
        let start = |sim: &mut SimulatedCamera| {
            sim.execute(
                op::SET_DEVICE_PROP_VALUE,
                &[prop::START_RAW_CONVERSION as u32],
                DataPhase::Write(&1u16.to_le_bytes()),
            )
        };
        let upload = |sim: &mut SimulatedCamera| {
            sim.execute(op::FUJI_SEND_OBJECT_INFO, &[0, 0, 0], DataPhase::Write(&[]))
                .unwrap();
            sim.execute(
                op::FUJI_SEND_OBJECT,
                &[],
                DataPhase::Write(&synthetic_raf("X-T5")),
            )
            .unwrap();
        };
        upload(&mut sim);
        assert_eq!(start(&mut sim).unwrap().code, rc::DEVICE_BUSY);
        sim.disconnect_after = Some(1);
        assert_eq!(start(&mut sim).unwrap().code, rc::OK);
        assert_eq!(start(&mut sim), Err(PtpError::Disconnected));
        sim.reconnect();
        assert!(start(&mut sim).is_ok());
    }

    #[test]
    fn never_writes_banks_when_reading_them() {
        let mut sim = SimulatedCamera::new();
        let banks = sim_banks();
        assert_eq!(banks.len(), 7);
        assert_eq!(banks[2].name, "Sim C3");
        assert_eq!(sim.bank_writes, 0);
        // Selezionare un banco inesistente è rifiutato come sulla camera reale.
        let r = sim
            .execute(
                op::SET_DEVICE_PROP_VALUE,
                &[prop::CUSTOM_SETTING_SELECT as u32],
                DataPhase::Write(&9u16.to_le_bytes()),
            )
            .unwrap();
        assert_eq!(r.code, rc::INVALID_DEVICE_PROP_VALUE);
    }
}
