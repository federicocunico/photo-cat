//! PTP transport over Windows Portable Devices, using the MTP extension
//! commands. This is the same path FUJIFILM X RAW STUDIO uses, so it works
//! with the in-box Windows driver and needs no WinUSB/libusb replacement.

// I parametri di uscita delle API COM sono dichiarati `*const` dai binding ma
// la chiamata li scrive: si passa `&mut` (come in FujiView) per non scrivere
// attraverso un riferimento condiviso.
#![allow(clippy::unnecessary_mut_passed)]

use super::ptp::{DataPhase, PtpError, PtpResult, PtpTransport, Response};
use std::ffi::c_void;
use windows::core::{GUID, HRESULT, PCWSTR, PWSTR};
use windows::Win32::Devices::PortableDevices::*;
use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_MULTITHREADED,
};

const SECURITY_IMPERSONATION: u32 = 0x0002_0000;
const DEFAULT_CHUNK: u32 = 1024 * 1024;
/// Fallback write chunk when the driver reports no optimal size (FUJIFILM
/// software on Windows 11 uses 511 x 512 bytes).
const WRITE_CHUNK: usize = 261_632;

/// Keeps COM initialised on the current thread for its lifetime.
pub struct ComGuard {
    owns: bool,
}

impl ComGuard {
    pub fn init() -> PtpResult<Self> {
        let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        // S_FALSE (already initialised) still requires a balancing CoUninitialize.
        if hr.is_ok() {
            Ok(Self { owns: true })
        } else if hr.0 as u32 == 0x8001_0106 {
            // RPC_E_CHANGED_MODE: another apartment model is active; COM is usable.
            Ok(Self { owns: false })
        } else {
            Err(transport_error(hr, "CoInitializeEx"))
        }
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.owns {
            unsafe { CoUninitialize() };
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WpdDevice {
    pub id: String,
    pub friendly_name: String,
    pub manufacturer: String,
    pub description: String,
}

impl WpdDevice {
    pub fn is_fujifilm(&self) -> bool {
        self.id
            .to_ascii_uppercase()
            .contains(super::FUJIFILM_USB_VID)
            || self.manufacturer.to_ascii_uppercase().contains("FUJIFILM")
    }
}

/// Traccia dei comandi PTP: attiva solo con il livello `trace` del target
/// `pc_raw_review::camera::wpd` (sostituisce `FUJIVIEW_PTP_TRACE`).
fn trace_enabled() -> bool {
    tracing::enabled!(tracing::Level::TRACE)
}

fn transport_error(hr: HRESULT, context: &str) -> PtpError {
    if trace_enabled() {
        tracing::trace!(
            "[ptp] {context}: HRESULT 0x{:08X} {}",
            hr.0 as u32,
            hr.message()
        );
    }
    match hr.0 as u32 {
        // ERROR_DEVICE_NOT_CONNECTED, ERROR_GEN_FAILURE, ERROR_NOT_READY, ERROR_FILE_NOT_FOUND
        0x8007_048F | 0x8007_001F | 0x8007_0015 | 0x8007_0002 => PtpError::Disconnected,
        // ERROR_BUSY, ERROR_SHARING_VIOLATION
        0x8007_00AA | 0x8007_0020 => PtpError::Busy,
        _ => PtpError::Transport {
            code: hr.0,
            message: format!("{context}: {}", hr.message()),
        },
    }
}

fn win_error(error: windows::core::Error, context: &str) -> PtpError {
    transport_error(error.code(), context)
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe fn take_pwstr(value: PWSTR) -> String {
    if value.is_null() {
        return String::new();
    }
    let text = value.to_string().unwrap_or_default();
    CoTaskMemFree(Some(value.0 as *const c_void));
    text
}

fn new_values() -> PtpResult<IPortableDeviceValues> {
    unsafe { CoCreateInstance(&PortableDeviceValues, None, CLSCTX_INPROC_SERVER) }
        .map_err(|e| win_error(e, "PortableDeviceValues"))
}

/// List WPD devices currently known to Windows.
pub fn list_devices() -> PtpResult<Vec<WpdDevice>> {
    unsafe {
        let manager: IPortableDeviceManager =
            CoCreateInstance(&PortableDeviceManager, None, CLSCTX_INPROC_SERVER)
                .map_err(|e| win_error(e, "PortableDeviceManager"))?;
        let _ = manager.RefreshDeviceList();
        let mut count = 0u32;
        manager
            .GetDevices(std::ptr::null_mut(), &mut count)
            .map_err(|e| win_error(e, "GetDevices"))?;
        let mut ids = vec![PWSTR::null(); count as usize];
        if count > 0 {
            manager
                .GetDevices(ids.as_mut_ptr(), &mut count)
                .map_err(|e| win_error(e, "GetDevices"))?;
        }
        let mut devices = Vec::new();
        for raw in ids.into_iter().take(count as usize) {
            let id = take_pwstr(raw);
            let id_w = wide(&id);
            let id_p = PCWSTR(id_w.as_ptr());
            let read = |kind: u8| -> String {
                let mut len = 0u32;
                let call = |buf: PWSTR, len: &mut u32| match kind {
                    0 => manager.GetDeviceFriendlyName(id_p, buf, len),
                    1 => manager.GetDeviceManufacturer(id_p, buf, len),
                    _ => manager.GetDeviceDescription(id_p, buf, len),
                };
                if call(PWSTR::null(), &mut len).is_err() || len == 0 {
                    return String::new();
                }
                let mut buf = vec![0u16; len as usize];
                if call(PWSTR(buf.as_mut_ptr()), &mut len).is_err() {
                    return String::new();
                }
                String::from_utf16_lossy(&buf)
                    .trim_end_matches('\0')
                    .to_string()
            };
            devices.push(WpdDevice {
                friendly_name: read(0),
                manufacturer: read(1),
                description: read(2),
                id,
            });
        }
        Ok(devices)
    }
}

pub fn find_fujifilm() -> PtpResult<Option<WpdDevice>> {
    Ok(list_devices()?.into_iter().find(WpdDevice::is_fujifilm))
}

pub struct WpdTransport {
    device: IPortableDevice,
    chunk: u32,
}

impl WpdTransport {
    pub fn open(device_id: &str) -> PtpResult<Self> {
        unsafe {
            let device: IPortableDevice =
                CoCreateInstance(&PortableDeviceFTM, None, CLSCTX_INPROC_SERVER)
                    .map_err(|e| win_error(e, "PortableDeviceFTM"))?;
            let client = new_values()?;
            let name = wide("photo-cat");
            client
                .SetStringValue(&WPD_CLIENT_NAME, PCWSTR(name.as_ptr()))
                .map_err(|e| win_error(e, "client name"))?;
            client
                .SetUnsignedIntegerValue(&WPD_CLIENT_MAJOR_VERSION, 1)
                .map_err(|e| win_error(e, "client version"))?;
            client
                .SetUnsignedIntegerValue(&WPD_CLIENT_MINOR_VERSION, 0)
                .map_err(|e| win_error(e, "client version"))?;
            client
                .SetUnsignedIntegerValue(&WPD_CLIENT_REVISION, 0)
                .map_err(|e| win_error(e, "client version"))?;
            client
                .SetUnsignedIntegerValue(
                    &WPD_CLIENT_SECURITY_QUALITY_OF_SERVICE,
                    SECURITY_IMPERSONATION,
                )
                .map_err(|e| win_error(e, "client QoS"))?;
            let id = wide(device_id);
            device
                .Open(PCWSTR(id.as_ptr()), &client)
                .map_err(|e| win_error(e, "IPortableDevice::Open"))?;
            Ok(Self {
                device,
                chunk: DEFAULT_CHUNK,
            })
        }
    }

    pub fn set_chunk_size(&mut self, bytes: u32) {
        self.chunk = bytes.max(64 * 1024);
    }

    fn command_values(command: &PROPERTYKEY) -> PtpResult<IPortableDeviceValues> {
        let values = new_values()?;
        unsafe {
            values
                .SetGuidValue(
                    &WPD_PROPERTY_COMMON_COMMAND_CATEGORY,
                    &command.fmtid as *const GUID,
                )
                .map_err(|e| win_error(e, "command category"))?;
            values
                .SetUnsignedIntegerValue(&WPD_PROPERTY_COMMON_COMMAND_ID, command.pid)
                .map_err(|e| win_error(e, "command id"))?;
        }
        Ok(values)
    }

    fn send(
        &self,
        values: &IPortableDeviceValues,
        context: &str,
    ) -> PtpResult<IPortableDeviceValues> {
        unsafe {
            let results = self
                .device
                .SendCommand(0, values)
                .map_err(|e| win_error(e, context))?;
            let hr = results
                .GetErrorValue(&WPD_PROPERTY_COMMON_HRESULT)
                .map_err(|e| win_error(e, context))?;
            if hr.is_err() {
                return Err(transport_error(hr, context));
            }
            Ok(results)
        }
    }

    fn operation_values(
        command: &PROPERTYKEY,
        op: u16,
        params: &[u32],
    ) -> PtpResult<IPortableDeviceValues> {
        let values = Self::command_values(command)?;
        unsafe {
            values
                .SetUnsignedIntegerValue(&WPD_PROPERTY_MTP_EXT_OPERATION_CODE, op as u32)
                .map_err(|e| win_error(e, "operation code"))?;
            let collection: IPortableDevicePropVariantCollection = CoCreateInstance(
                &PortableDevicePropVariantCollection,
                None,
                CLSCTX_INPROC_SERVER,
            )
            .map_err(|e| win_error(e, "params"))?;
            for param in params {
                let value = PROPVARIANT::from(*param);
                collection.Add(&value).map_err(|e| win_error(e, "params"))?;
            }
            values
                .SetIPortableDevicePropVariantCollectionValue(
                    &WPD_PROPERTY_MTP_EXT_OPERATION_PARAMS,
                    &collection,
                )
                .map_err(|e| win_error(e, "params"))?;
        }
        Ok(values)
    }

    fn read_response(results: &IPortableDeviceValues) -> PtpResult<(u16, Vec<u32>)> {
        unsafe {
            let code = results
                .GetUnsignedIntegerValue(&WPD_PROPERTY_MTP_EXT_RESPONSE_CODE)
                .map_err(|e| win_error(e, "response code"))? as u16;
            let mut params = Vec::new();
            if let Ok(collection) = results
                .GetIPortableDevicePropVariantCollectionValue(&WPD_PROPERTY_MTP_EXT_RESPONSE_PARAMS)
            {
                let mut count = 0u32;
                collection
                    .GetCount(&mut count)
                    .map_err(|e| win_error(e, "response params"))?;
                for index in 0..count {
                    let mut value = PROPVARIANT::default();
                    collection
                        .GetAt(index, &mut value)
                        .map_err(|e| win_error(e, "response params"))?;
                    params.push(u32::try_from(&value).unwrap_or_default());
                }
            }
            Ok((code, params))
        }
    }

    fn context_of(results: &IPortableDeviceValues) -> PtpResult<Vec<u16>> {
        unsafe {
            let context = results
                .GetStringValue(&WPD_PROPERTY_MTP_EXT_TRANSFER_CONTEXT)
                .map_err(|e| win_error(e, "transfer context"))?;
            Ok(wide(&take_pwstr(context)))
        }
    }

    fn end_transfer(&self, context: &[u16]) -> PtpResult<(u16, Vec<u32>)> {
        let values = Self::command_values(&WPD_COMMAND_MTP_EXT_END_DATA_TRANSFER)?;
        unsafe {
            values
                .SetStringValue(
                    &WPD_PROPERTY_MTP_EXT_TRANSFER_CONTEXT,
                    PCWSTR(context.as_ptr()),
                )
                .map_err(|e| win_error(e, "end transfer"))?;
        }
        let results = self.send(&values, "END_DATA_TRANSFER")?;
        Self::read_response(&results)
    }

    fn execute_read(&self, op: u16, params: &[u32]) -> PtpResult<Response> {
        let values = Self::operation_values(
            &WPD_COMMAND_MTP_EXT_EXECUTE_COMMAND_WITH_DATA_TO_READ,
            op,
            params,
        )?;
        let results = self.send(&values, "EXECUTE_COMMAND_WITH_DATA_TO_READ")?;
        let context = Self::context_of(&results)?;
        let total = unsafe {
            results.GetUnsignedLargeIntegerValue(&WPD_PROPERTY_MTP_EXT_TRANSFER_TOTAL_DATA_SIZE)
        }
        .unwrap_or(0);
        let chunk = unsafe {
            results.GetUnsignedIntegerValue(&WPD_PROPERTY_MTP_EXT_OPTIMAL_TRANSFER_BUFFER_SIZE)
        }
        .unwrap_or(self.chunk)
        .max(self.chunk);
        let mut data = Vec::with_capacity(total.min(512 * 1024 * 1024) as usize);
        let outcome = (|| -> PtpResult<()> {
            while (data.len() as u64) < total {
                let want = ((total - data.len() as u64).min(chunk as u64)) as u32;
                let request = Self::command_values(&WPD_COMMAND_MTP_EXT_READ_DATA)?;
                let buffer = vec![0u8; want as usize];
                unsafe {
                    request
                        .SetStringValue(
                            &WPD_PROPERTY_MTP_EXT_TRANSFER_CONTEXT,
                            PCWSTR(context.as_ptr()),
                        )
                        .map_err(|e| win_error(e, "read"))?;
                    request
                        .SetUnsignedIntegerValue(
                            &WPD_PROPERTY_MTP_EXT_TRANSFER_NUM_BYTES_TO_READ,
                            want,
                        )
                        .map_err(|e| win_error(e, "read"))?;
                    request
                        .SetBufferValue(&WPD_PROPERTY_MTP_EXT_TRANSFER_DATA, &buffer)
                        .map_err(|e| win_error(e, "read"))?;
                }
                let reply = self.send(&request, "READ_DATA")?;
                let mut ptr: *mut u8 = std::ptr::null_mut();
                let mut len = 0u32;
                unsafe {
                    reply
                        .GetBufferValue(&WPD_PROPERTY_MTP_EXT_TRANSFER_DATA, &mut ptr, &mut len)
                        .map_err(|e| win_error(e, "read"))?;
                    let read = reply
                        .GetUnsignedIntegerValue(&WPD_PROPERTY_MTP_EXT_TRANSFER_NUM_BYTES_READ)
                        .unwrap_or(len)
                        .min(len);
                    if !ptr.is_null() {
                        data.extend_from_slice(std::slice::from_raw_parts(ptr, read as usize));
                        CoTaskMemFree(Some(ptr as *const c_void));
                    }
                    if read == 0 {
                        break;
                    }
                }
            }
            Ok(())
        })();
        let end = self.end_transfer(&context);
        outcome?;
        let (code, params) = end?;
        Ok(Response { code, params, data })
    }

    fn execute_write(&self, op: u16, params: &[u32], payload: &[u8]) -> PtpResult<Response> {
        let values = Self::operation_values(
            &WPD_COMMAND_MTP_EXT_EXECUTE_COMMAND_WITH_DATA_TO_WRITE,
            op,
            params,
        )?;
        unsafe {
            values
                .SetUnsignedLargeIntegerValue(
                    &WPD_PROPERTY_MTP_EXT_TRANSFER_TOTAL_DATA_SIZE,
                    payload.len() as u64,
                )
                .map_err(|e| win_error(e, "write size"))?;
        }
        let results = self.send(&values, "EXECUTE_COMMAND_WITH_DATA_TO_WRITE")?;
        let context = Self::context_of(&results)?;
        let optimal = unsafe {
            results.GetUnsignedIntegerValue(&WPD_PROPERTY_MTP_EXT_OPTIMAL_TRANSFER_BUFFER_SIZE)
        }
        .ok();
        let chunk = std::env::var("PHOTOCAT_PTP_WRITE_CHUNK")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|&v| v > 0)
            .or(optimal.map(|v| v as usize).filter(|&v| v > 0))
            .unwrap_or(WRITE_CHUNK);
        if trace_enabled() {
            tracing::trace!("[ptp] write: chunk {chunk} B, ottimale driver {optimal:?}");
        }
        let started = std::time::Instant::now();
        let outcome = (|| -> PtpResult<()> {
            let mut sent = 0usize;
            for piece in payload.chunks(chunk) {
                let request = Self::command_values(&WPD_COMMAND_MTP_EXT_WRITE_DATA)?;
                unsafe {
                    request
                        .SetStringValue(
                            &WPD_PROPERTY_MTP_EXT_TRANSFER_CONTEXT,
                            PCWSTR(context.as_ptr()),
                        )
                        .map_err(|e| win_error(e, "write"))?;
                    request
                        .SetUnsignedIntegerValue(
                            &WPD_PROPERTY_MTP_EXT_TRANSFER_NUM_BYTES_TO_WRITE,
                            piece.len() as u32,
                        )
                        .map_err(|e| win_error(e, "write"))?;
                    request
                        .SetBufferValue(&WPD_PROPERTY_MTP_EXT_TRANSFER_DATA, piece)
                        .map_err(|e| win_error(e, "write"))?;
                }
                if let Err(e) = self.send(&request, "WRITE_DATA") {
                    if trace_enabled() {
                        tracing::trace!(
                            "[ptp] WRITE_DATA fallito dopo {sent} di {} byte ({} ms)",
                            payload.len(),
                            started.elapsed().as_millis()
                        );
                    }
                    return Err(e);
                }
                sent += piece.len();
            }
            Ok(())
        })();
        let end = self.end_transfer(&context);
        outcome?;
        let (code, params) = end?;
        Ok(Response {
            code,
            params,
            data: Vec::new(),
        })
    }

    fn execute_no_data(&self, op: u16, params: &[u32]) -> PtpResult<Response> {
        let values = Self::operation_values(
            &WPD_COMMAND_MTP_EXT_EXECUTE_COMMAND_WITHOUT_DATA_PHASE,
            op,
            params,
        )?;
        let results = self.send(&values, "EXECUTE_COMMAND_WITHOUT_DATA_PHASE")?;
        let (code, params) = Self::read_response(&results)?;
        Ok(Response {
            code,
            params,
            data: Vec::new(),
        })
    }

    /// Vendor operation codes the driver will pass through.
    pub fn supported_vendor_opcodes(&self) -> PtpResult<Vec<u16>> {
        let values = Self::command_values(&WPD_COMMAND_MTP_EXT_GET_SUPPORTED_VENDOR_OPCODES)?;
        let results = self.send(&values, "GET_SUPPORTED_VENDOR_OPCODES")?;
        let mut codes = Vec::new();
        unsafe {
            let collection = results
                .GetIPortableDevicePropVariantCollectionValue(
                    &WPD_PROPERTY_MTP_EXT_VENDOR_OPERATION_CODES,
                )
                .map_err(|e| win_error(e, "vendor opcodes"))?;
            let mut count = 0u32;
            collection
                .GetCount(&mut count)
                .map_err(|e| win_error(e, "vendor opcodes"))?;
            for index in 0..count {
                let mut value = PROPVARIANT::default();
                collection
                    .GetAt(index, &mut value)
                    .map_err(|e| win_error(e, "vendor opcodes"))?;
                codes.push(u32::try_from(&value).unwrap_or_default() as u16);
            }
        }
        Ok(codes)
    }
}

impl PtpTransport for WpdTransport {
    fn execute(&mut self, op: u16, params: &[u32], data: DataPhase<'_>) -> PtpResult<Response> {
        let trace = trace_enabled();
        let started = std::time::Instant::now();
        if trace {
            let phase = match &data {
                DataPhase::None => "none".to_string(),
                DataPhase::Read => "read".to_string(),
                DataPhase::Write(p) => format!("write {} B", p.len()),
            };
            tracing::trace!("[ptp] -> {op:04X} {params:X?} ({phase})");
        }
        let result = match data {
            DataPhase::None => self.execute_no_data(op, params),
            DataPhase::Read => self.execute_read(op, params),
            DataPhase::Write(payload) => self.execute_write(op, params, payload),
        };
        if trace {
            let ms = started.elapsed().as_millis();
            match &result {
                Ok(r) => tracing::trace!(
                    "[ptp] <- {:04X} {:X?} data {} B ({ms} ms)",
                    r.code,
                    r.params,
                    r.data.len()
                ),
                Err(e) => tracing::trace!("[ptp] <- errore: {e} ({ms} ms)"),
            }
        }
        result
    }
}

impl Drop for WpdTransport {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.Close();
        }
    }
}
