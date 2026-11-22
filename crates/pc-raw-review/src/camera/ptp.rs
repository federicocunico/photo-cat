//! Transport-independent PTP primitives used by the Fujifilm connector.

use pc_core::error::{AppError, ErrorCode};
use std::fmt;

/// Standard PTP operation codes.
pub mod op {
    pub const GET_DEVICE_INFO: u16 = 0x1001;
    pub const OPEN_SESSION: u16 = 0x1002;
    pub const CLOSE_SESSION: u16 = 0x1003;
    pub const GET_OBJECT_HANDLES: u16 = 0x1007;
    pub const GET_OBJECT_INFO: u16 = 0x1008;
    pub const GET_OBJECT: u16 = 0x1009;
    pub const GET_THUMB: u16 = 0x100A;
    pub const DELETE_OBJECT: u16 = 0x100B;
    pub const GET_DEVICE_PROP_DESC: u16 = 0x1014;
    pub const GET_DEVICE_PROP_VALUE: u16 = 0x1015;
    pub const SET_DEVICE_PROP_VALUE: u16 = 0x1016;
    /// Fujifilm: SendObjectInfo variant used for RAW conversion uploads.
    pub const FUJI_SEND_OBJECT_INFO: u16 = 0x900C;
    /// Fujifilm: SendObject variant used for RAW conversion uploads.
    pub const FUJI_SEND_OBJECT: u16 = 0x900D;
}

/// Fujifilm device property codes.
pub mod prop {
    /// 5 = tether, 6 = USB RAW conversion, 8 = webcam.
    pub const USB_MODE: u16 = 0xD16E;
    pub const USB_MODE_RAW_CONV: u32 = 6;
    pub const START_RAW_CONVERSION: u16 = 0xD183;
    pub const RAW_CONVERSION_PROFILE: u16 = 0xD185;
    pub const CUSTOM_SETTING_SELECT: u16 = 0xD18C;
    pub const CUSTOM_SETTING_NAME: u16 = 0xD18D;
    pub const CUSTOM_SETTING_FIRST: u16 = 0xD18E;
    pub const CUSTOM_SETTING_LAST: u16 = 0xD1A4;
}

/// PTP response codes that matter to the connector.
pub mod rc {
    pub const OK: u16 = 0x2001;
    pub const GENERAL_ERROR: u16 = 0x2002;
    pub const SESSION_NOT_OPEN: u16 = 0x2003;
    pub const OPERATION_NOT_SUPPORTED: u16 = 0x2005;
    pub const DEVICE_PROP_NOT_SUPPORTED: u16 = 0x200A;
    pub const DEVICE_BUSY: u16 = 0x2019;
    pub const INVALID_DEVICE_PROP_VALUE: u16 = 0x201C;
    pub const SESSION_ALREADY_OPEN: u16 = 0x201E;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataPhase<'a> {
    None,
    /// Read the data phase sent by the device.
    Read,
    /// Send these bytes to the device.
    Write(&'a [u8]),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Response {
    pub code: u16,
    pub params: Vec<u32>,
    pub data: Vec<u8>,
}

impl Response {
    pub fn is_ok(&self) -> bool {
        self.code == rc::OK
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PtpError {
    /// The OS transport reported a failure (HRESULT or errno-like code).
    Transport {
        code: i32,
        message: String,
    },
    /// The camera answered with a non-OK response code.
    Response {
        op: u16,
        code: u16,
    },
    /// The camera is not connected or was removed.
    Disconnected,
    /// The camera is in use by another application.
    Busy,
    Timeout(String),
    Protocol(String),
}

impl fmt::Display for PtpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PtpError::Transport { code, message } => {
                write!(f, "errore di trasporto 0x{:08X}: {message}", *code as u32)
            }
            PtpError::Response { op, code } => write!(
                f,
                "la camera ha risposto 0x{code:04X} all'operazione 0x{op:04X}"
            ),
            PtpError::Disconnected => write!(f, "camera disconnessa"),
            PtpError::Busy => write!(f, "camera occupata da un altro programma"),
            PtpError::Timeout(what) => write!(f, "timeout: {what}"),
            PtpError::Protocol(what) => write!(f, "errore di protocollo: {what}"),
        }
    }
}

impl std::error::Error for PtpError {}

impl PtpError {
    /// Azione suggerita all'utente (spec camera-connection: ogni errore della
    /// camera riporta un messaggio comprensibile con il passo successivo).
    pub fn hint(&self) -> &'static str {
        match self {
            PtpError::Disconnected => "Controlla il cavo USB, accendi la fotocamera e riprova.",
            PtpError::Busy => {
                "Chiudi X RAW Studio e gli altri programmi che usano la fotocamera, poi riprova."
            }
            PtpError::Timeout(_) => "Spegni e riaccendi la fotocamera, poi riprova.",
            PtpError::Response {
                code: rc::DEVICE_BUSY,
                ..
            } => "La fotocamera è occupata: attendi qualche secondo e riprova.",
            PtpError::Response {
                code:
                    rc::OPERATION_NOT_SUPPORTED | rc::DEVICE_PROP_NOT_SUPPORTED | rc::SESSION_NOT_OPEN,
                ..
            } => {
                "Imposta la modalità USB RAW CONV./BACKUP RESTORE nel menu della fotocamera e riprova."
            }
            PtpError::Transport { .. } => {
                "Scollega e ricollega il cavo USB; se non basta, spegni e riaccendi la fotocamera."
            }
            PtpError::Response { .. } | PtpError::Protocol(_) => {
                "Spegni e riaccendi la fotocamera, poi riprova."
            }
        }
    }

    /// Messaggio per l'utente: causa e azione suggerita.
    pub fn user_message(&self) -> String {
        format!("{self}. {}", self.hint())
    }
}

impl From<PtpError> for AppError {
    fn from(error: PtpError) -> Self {
        let code = match &error {
            PtpError::Busy => ErrorCode::CameraBusy,
            PtpError::Disconnected => ErrorCode::CameraDisconnected,
            PtpError::Timeout(_) => ErrorCode::CameraUnresponsive,
            PtpError::Protocol(_) => ErrorCode::Internal,
            // Operazioni che la fotocamera offre solo in USB RAW CONV./BACKUP RESTORE.
            PtpError::Response {
                code:
                    rc::OPERATION_NOT_SUPPORTED | rc::DEVICE_PROP_NOT_SUPPORTED | rc::SESSION_NOT_OPEN,
                ..
            } => ErrorCode::CameraNotReady,
            PtpError::Transport { .. } | PtpError::Response { .. } => ErrorCode::Io,
        };
        let retriable = !matches!(error, PtpError::Protocol(_));
        AppError::new(code, error.user_message()).with_retriable(retriable)
    }
}

pub type PtpResult<T> = Result<T, PtpError>;

/// A PTP command channel to one device.
pub trait PtpTransport {
    fn execute(&mut self, op: u16, params: &[u32], data: DataPhase<'_>) -> PtpResult<Response>;

    /// Execute and fail unless the response code is OK.
    fn execute_ok(&mut self, op: u16, params: &[u32], data: DataPhase<'_>) -> PtpResult<Response> {
        let response = self.execute(op, params, data)?;
        if response.is_ok() {
            Ok(response)
        } else {
            Err(PtpError::Response {
                op,
                code: response.code,
            })
        }
    }
}

/// Little-endian reader for PTP datasets.
pub struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn take(&mut self, n: usize) -> PtpResult<&'a [u8]> {
        let end = self
            .pos
            .checked_add(n)
            .filter(|end| *end <= self.data.len());
        let end = end.ok_or_else(|| {
            PtpError::Protocol(format!("dataset troncato a {} byte", self.data.len()))
        })?;
        let slice = &self.data[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    pub fn u8(&mut self) -> PtpResult<u8> {
        Ok(self.take(1)?[0])
    }

    pub fn u16(&mut self) -> PtpResult<u16> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    pub fn u32(&mut self) -> PtpResult<u32> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn u16_array(&mut self) -> PtpResult<Vec<u16>> {
        let count = self.u32()? as usize;
        (0..count).map(|_| self.u16()).collect()
    }

    /// PTP string: u8 character count (including the terminator) then UCS-2.
    pub fn string(&mut self) -> PtpResult<String> {
        let chars = self.u8()? as usize;
        let units: Vec<u16> = (0..chars).map(|_| self.u16()).collect::<PtpResult<_>>()?;
        Ok(String::from_utf16_lossy(&units)
            .trim_end_matches('\0')
            .to_string())
    }
}

/// Encode a PTP string (u8 count including terminator, UCS-2 LE).
pub fn encode_string(value: &str) -> Vec<u8> {
    if value.is_empty() {
        return vec![0];
    }
    let mut units: Vec<u16> = value.encode_utf16().take(254).collect();
    units.push(0);
    let mut out = vec![units.len() as u8];
    for unit in units {
        out.extend_from_slice(&unit.to_le_bytes());
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    pub standard_version: u16,
    pub vendor_extension_id: u32,
    pub vendor_extension_version: u16,
    pub vendor_extension_desc: String,
    pub functional_mode: u16,
    pub operations: Vec<u16>,
    pub events: Vec<u16>,
    pub properties: Vec<u16>,
    pub capture_formats: Vec<u16>,
    pub image_formats: Vec<u16>,
    pub manufacturer: String,
    pub model: String,
    pub device_version: String,
    pub serial_number: String,
}

impl DeviceInfo {
    pub fn parse(data: &[u8]) -> PtpResult<Self> {
        let mut r = Reader::new(data);
        Ok(Self {
            standard_version: r.u16()?,
            vendor_extension_id: r.u32()?,
            vendor_extension_version: r.u16()?,
            vendor_extension_desc: r.string()?,
            functional_mode: r.u16()?,
            operations: r.u16_array()?,
            events: r.u16_array()?,
            properties: r.u16_array()?,
            capture_formats: r.u16_array()?,
            image_formats: r.u16_array()?,
            manufacturer: r.string()?,
            model: r.string()?,
            device_version: r.string()?,
            serial_number: r.string()?,
        })
    }

    pub fn supports_operation(&self, code: u16) -> bool {
        self.operations.contains(&code)
    }

    pub fn supports_property(&self, code: u16) -> bool {
        self.properties.contains(&code)
    }

    /// Operations and properties required for in-camera RAW conversion that
    /// the device does not advertise.
    pub fn missing_conversion_support(&self) -> Vec<String> {
        let mut missing = Vec::new();
        for code in [
            op::FUJI_SEND_OBJECT_INFO,
            op::FUJI_SEND_OBJECT,
            op::GET_OBJECT_HANDLES,
            op::GET_OBJECT,
            op::DELETE_OBJECT,
        ] {
            if !self.supports_operation(code) {
                missing.push(format!("op 0x{code:04X}"));
            }
        }
        for code in [prop::START_RAW_CONVERSION, prop::RAW_CONVERSION_PROFILE] {
            if !self.supports_property(code) {
                missing.push(format!("prop 0x{code:04X}"));
            }
        }
        missing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_device_info() -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(&100u16.to_le_bytes());
        d.extend_from_slice(&6u32.to_le_bytes());
        d.extend_from_slice(&100u16.to_le_bytes());
        d.extend(encode_string("fujifilm.co.jp: 1.0;"));
        d.extend_from_slice(&0u16.to_le_bytes());
        for list in [
            vec![0x1001u16, 0x900C, 0x900D, 0x1007, 0x1009, 0x100B],
            vec![0x4002],
            vec![0xD183, 0xD185],
            vec![],
            vec![0x3801],
        ] {
            d.extend_from_slice(&(list.len() as u32).to_le_bytes());
            for v in list {
                d.extend_from_slice(&v.to_le_bytes());
            }
        }
        for s in ["FUJIFILM", "X-T5", "4.20", "123"] {
            d.extend(encode_string(s));
        }
        d
    }

    #[test]
    fn parses_device_info() {
        let info = DeviceInfo::parse(&sample_device_info()).unwrap();
        assert_eq!(info.model, "X-T5");
        assert_eq!(info.manufacturer, "FUJIFILM");
        assert!(info.supports_operation(0x900C));
        assert!(info.missing_conversion_support().is_empty());
    }

    #[test]
    fn truncated_dataset_is_an_error() {
        let data = sample_device_info();
        assert!(DeviceInfo::parse(&data[..data.len() - 3]).is_err());
    }

    #[test]
    fn string_round_trip() {
        let encoded = encode_string("C1 Kodachrome");
        assert_eq!(Reader::new(&encoded).string().unwrap(), "C1 Kodachrome");
        assert_eq!(encode_string(""), vec![0]);
    }

    #[test]
    fn errors_carry_a_suggested_action() {
        assert!(PtpError::Busy.user_message().contains("X RAW Studio"));
        assert!(PtpError::Timeout("x".into())
            .user_message()
            .contains("Spegni e riaccendi"));
        assert!(PtpError::Disconnected.user_message().contains("cavo USB"));
        let unsupported = PtpError::Response {
            op: op::GET_DEVICE_PROP_VALUE,
            code: rc::DEVICE_PROP_NOT_SUPPORTED,
        };
        assert!(unsupported
            .user_message()
            .contains("USB RAW CONV./BACKUP RESTORE"));
    }

    #[test]
    fn converts_into_app_errors() {
        let busy: AppError = PtpError::Busy.into();
        assert_eq!(busy.code, ErrorCode::CameraBusy);
        assert!(busy.retriable);
        assert!(busy.detail.contains("occupata"));
        let gone: AppError = PtpError::Disconnected.into();
        assert_eq!(gone.code, ErrorCode::CameraDisconnected);
        let hung: AppError = PtpError::Timeout("x".into()).into();
        assert_eq!(hung.code, ErrorCode::CameraUnresponsive);
        let wrong_mode: AppError = PtpError::Response {
            op: op::GET_DEVICE_PROP_VALUE,
            code: rc::DEVICE_PROP_NOT_SUPPORTED,
        }
        .into();
        assert_eq!(wrong_mode.code, ErrorCode::CameraNotReady);
        let bad: AppError = PtpError::Protocol("x".into()).into();
        assert_eq!(bad.code, ErrorCode::Internal);
        assert!(!bad.retriable);
    }
}
