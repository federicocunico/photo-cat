//! Fujifilm RAW-conversion protocol on top of any [`PtpTransport`].

use super::banks::{decode_bank_value, Bank};
use super::profile::Profile;
use super::ptp::{
    encode_string, op, prop, rc, DataPhase, DeviceInfo, PtpError, PtpResult, PtpTransport, Reader,
    Response,
};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

/// Object format code Fujifilm uses for RAF uploads.
pub const RAF_OBJECT_FORMAT: u16 = 0xF802;
pub const UPLOAD_FILE_NAME: &str = "FUP_FILE.dat";
pub const MAX_BANKS: u8 = 7;

impl<T: PtpTransport + ?Sized> PtpTransport for Box<T> {
    fn execute(&mut self, op: u16, params: &[u32], data: DataPhase<'_>) -> PtpResult<Response> {
        (**self).execute(op, params, data)
    }
}

/// True when the camera refused the uploaded RAF (other body, corrupt file).
pub fn is_raf_rejected(error: &PtpError) -> bool {
    matches!(
        error,
        PtpError::Response {
            op: op::GET_DEVICE_PROP_VALUE,
            code: rc::GENERAL_ERROR
        }
    )
}

pub struct FujiCamera<T: PtpTransport> {
    transport: T,
    pub poll_interval: Duration,
    pub conversion_timeout: Duration,
}

impl<T: PtpTransport> FujiCamera<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            poll_interval: Duration::from_millis(50),
            conversion_timeout: Duration::from_secs(60),
        }
    }

    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    pub fn device_info(&mut self) -> PtpResult<DeviceInfo> {
        let response = self
            .transport
            .execute_ok(op::GET_DEVICE_INFO, &[], DataPhase::Read)?;
        DeviceInfo::parse(&response.data)
    }

    pub fn get_prop(&mut self, code: u16) -> PtpResult<Vec<u8>> {
        Ok(self
            .transport
            .execute_ok(op::GET_DEVICE_PROP_VALUE, &[code as u32], DataPhase::Read)?
            .data)
    }

    pub fn set_prop(&mut self, code: u16, value: &[u8]) -> PtpResult<()> {
        self.transport.execute_ok(
            op::SET_DEVICE_PROP_VALUE,
            &[code as u32],
            DataPhase::Write(value),
        )?;
        Ok(())
    }

    fn set_u16(&mut self, code: u16, value: u16) -> PtpResult<()> {
        self.set_prop(code, &value.to_le_bytes())
    }

    fn read_bank_settings(&mut self, index: u8) -> PtpResult<Bank> {
        self.set_u16(prop::CUSTOM_SETTING_SELECT, index as u16)?;
        let name = self
            .get_prop(prop::CUSTOM_SETTING_NAME)
            .and_then(|data| Reader::new(&data).string())
            .unwrap_or_default();
        let mut props = BTreeMap::new();
        for code in prop::CUSTOM_SETTING_FIRST..=prop::CUSTOM_SETTING_LAST {
            match self.get_prop(code) {
                Ok(data) => {
                    if let Some(value) = decode_bank_value(code, &data) {
                        props.insert(code, value);
                    }
                }
                // Property absent on this body: skip it.
                Err(PtpError::Response {
                    code: rc::DEVICE_PROP_NOT_SUPPORTED,
                    ..
                }) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(Bank { index, name, props })
    }

    /// Read every custom bank. The bank selected on the camera is restored
    /// afterwards, even when a read fails. Banks are never written.
    pub fn read_banks(&mut self) -> PtpResult<Vec<Bank>> {
        let original = self.get_prop(prop::CUSTOM_SETTING_SELECT)?;
        let mut banks = Vec::new();
        let mut outcome = Ok(());
        for index in 1..=MAX_BANKS {
            match self.read_bank_settings(index) {
                Ok(bank) => banks.push(bank),
                // Fewer banks than MAX_BANKS on this body.
                Err(PtpError::Response {
                    op: op::SET_DEVICE_PROP_VALUE,
                    code: rc::INVALID_DEVICE_PROP_VALUE,
                }) if index > 1 => break,
                Err(error) => {
                    outcome = Err(error);
                    break;
                }
            }
        }
        let restore = self.set_prop(prop::CUSTOM_SETTING_SELECT, &original);
        outcome?;
        restore?;
        Ok(banks)
    }

    pub fn object_handles(&mut self) -> PtpResult<Vec<u32>> {
        let response = self.transport.execute_ok(
            op::GET_OBJECT_HANDLES,
            &[0xFFFF_FFFF, 0, 0],
            DataPhase::Read,
        )?;
        let mut reader = Reader::new(&response.data);
        let count = reader.u32()?;
        (0..count).map(|_| reader.u32()).collect()
    }

    /// Delete leftover conversion outputs so a new job never picks them up.
    pub fn clear_objects(&mut self) -> PtpResult<usize> {
        let handles = self.object_handles()?;
        for handle in &handles {
            self.transport
                .execute_ok(op::DELETE_OBJECT, &[*handle], DataPhase::None)?;
        }
        Ok(handles.len())
    }

    pub fn upload_raf(&mut self, raf: &[u8]) -> PtpResult<()> {
        let info = object_info(raf.len() as u32);
        self.transport.execute_ok(
            op::FUJI_SEND_OBJECT_INFO,
            &[0, 0, 0],
            DataPhase::Write(&info),
        )?;
        self.transport
            .execute_ok(op::FUJI_SEND_OBJECT, &[], DataPhase::Write(raf))?;
        Ok(())
    }

    /// The as-shot profile of the RAF just uploaded. A 0x2002 answer means the
    /// camera cannot convert this RAF (see [`is_raf_rejected`]).
    pub fn read_profile(&mut self) -> PtpResult<Profile> {
        Profile::parse(&self.get_prop(prop::RAW_CONVERSION_PROFILE)?)
    }

    pub fn write_profile(&mut self, profile: &Profile) -> PtpResult<()> {
        self.set_prop(prop::RAW_CONVERSION_PROFILE, &profile.to_bytes())
    }

    /// Convert the uploaded RAF with `profile` at full resolution and return
    /// the JPEG. The output object is deleted from the camera afterwards.
    pub fn convert(&mut self, profile: &Profile) -> PtpResult<Vec<u8>> {
        let before = self.object_handles()?;
        self.write_profile(profile)?;
        self.set_u16(prop::START_RAW_CONVERSION, 1)?;
        let started = Instant::now();
        let handle = loop {
            if let Some(handle) = self
                .object_handles()?
                .into_iter()
                .find(|h| !before.contains(h))
            {
                break handle;
            }
            if started.elapsed() > self.conversion_timeout {
                return Err(PtpError::Timeout(
                    "la camera non ha prodotto il JPEG".into(),
                ));
            }
            std::thread::sleep(self.poll_interval);
        };
        let data = self
            .transport
            .execute_ok(op::GET_OBJECT, &[handle], DataPhase::Read)
            .map(|r| r.data);
        let deleted = self
            .transport
            .execute_ok(op::DELETE_OBJECT, &[handle], DataPhase::None);
        let data = data?;
        deleted?;
        if data.len() < 4 || data[0] != 0xFF || data[1] != 0xD8 {
            return Err(PtpError::Protocol(
                "l'output della camera non è un JPEG".into(),
            ));
        }
        Ok(data)
    }
}

/// PTP ObjectInfo dataset announcing a RAF upload.
pub fn object_info(size: u32) -> Vec<u8> {
    let mut d = Vec::with_capacity(64);
    d.extend_from_slice(&0u32.to_le_bytes()); // StorageID
    d.extend_from_slice(&RAF_OBJECT_FORMAT.to_le_bytes());
    d.extend_from_slice(&0u16.to_le_bytes()); // ProtectionStatus
    d.extend_from_slice(&size.to_le_bytes()); // CompressedSize
    d.extend_from_slice(&0u16.to_le_bytes()); // ThumbFormat
                                              // ThumbCompressedSize, ThumbPixWidth, ThumbPixHeight,
                                              // ImagePixWidth, ImagePixHeight, ImageBitDepth
    for _ in 0..6 {
        d.extend_from_slice(&0u32.to_le_bytes());
    }
    d.extend_from_slice(&0u32.to_le_bytes()); // ParentObject
    d.extend_from_slice(&0u16.to_le_bytes()); // AssociationType
    d.extend_from_slice(&0u32.to_le_bytes()); // AssociationDesc
    d.extend_from_slice(&0u32.to_le_bytes()); // SequenceNumber
    d.extend(encode_string(UPLOAD_FILE_NAME));
    d.extend([0, 0, 0]); // CaptureDate, ModificationDate, Keywords
    d
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::sim::SimulatedCamera;

    fn raf() -> Vec<u8> {
        crate::camera::sim::synthetic_raf("X-T5")
    }

    #[test]
    fn reads_banks_and_restores_selection() {
        let mut sim = SimulatedCamera::new();
        sim.selected_bank = 4;
        let mut camera = FujiCamera::new(sim);
        let banks = camera.read_banks().unwrap();
        assert_eq!(banks.len(), 7);
        assert_eq!(banks[0].name, "Sim C1");
        assert_eq!(camera.transport_mut().selected_bank, 4);
        assert!(camera.transport_mut().bank_writes == 0);
    }

    #[test]
    fn restores_selection_when_a_read_fails() {
        let mut sim = SimulatedCamera::new();
        sim.selected_bank = 2;
        sim.fail_prop = Some(0xD195);
        let mut camera = FujiCamera::new(sim);
        assert!(camera.read_banks().is_err());
        assert_eq!(camera.transport_mut().selected_bank, 2);
    }

    #[test]
    fn converts_and_deletes_output() {
        let mut camera = FujiCamera::new(SimulatedCamera::new());
        camera.upload_raf(&raf()).unwrap();
        let profile = camera.read_profile().unwrap();
        let jpeg = camera.convert(&profile).unwrap();
        assert_eq!(&jpeg[..2], &[0xFF, 0xD8]);
        assert!(camera.object_handles().unwrap().is_empty());
        // Several conversions after a single upload.
        camera.convert(&profile).unwrap();
        assert_eq!(camera.transport_mut().conversions, 2);
    }

    #[test]
    fn foreign_raf_is_rejected_at_profile_read() {
        let mut camera = FujiCamera::new(SimulatedCamera::new());
        camera
            .upload_raf(&crate::camera::sim::synthetic_raf("X100VI"))
            .unwrap();
        let error = camera.read_profile().unwrap_err();
        assert!(is_raf_rejected(&error), "{error:?}");
    }

    #[test]
    fn conversion_times_out() {
        let mut sim = SimulatedCamera::new();
        sim.hang_conversions = true;
        let mut camera = FujiCamera::new(sim);
        camera.conversion_timeout = Duration::from_millis(30);
        camera.poll_interval = Duration::from_millis(5);
        camera.upload_raf(&raf()).unwrap();
        let profile = camera.read_profile().unwrap();
        assert!(matches!(
            camera.convert(&profile),
            Err(PtpError::Timeout(_))
        ));
    }

    #[test]
    fn clears_leftover_objects() {
        let mut sim = SimulatedCamera::new();
        sim.objects.insert(99, vec![0xFF, 0xD8, 0xFF, 0xD9]);
        let mut camera = FujiCamera::new(sim);
        assert_eq!(camera.clear_objects().unwrap(), 1);
        assert!(camera.object_handles().unwrap().is_empty());
    }

    #[test]
    fn object_info_names_the_upload() {
        let info = object_info(1234);
        assert_eq!(u16::from_le_bytes([info[4], info[5]]), RAF_OBJECT_FORMAT);
        assert_eq!(
            u32::from_le_bytes([info[8], info[9], info[10], info[11]]),
            1234
        );
        // 52 fixed bytes (ISO 15740 ObjectInfo) + "FUP_FILE.dat" (1 + 13 * 2) + 3 empty strings.
        assert_eq!(info.len(), 52 + 27 + 3);
        assert_eq!(
            info[52], 13,
            "filename must start right after the fixed fields"
        );
    }
}
