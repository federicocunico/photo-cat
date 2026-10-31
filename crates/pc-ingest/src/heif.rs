//! HEIF: metadati leggeri e decoder libheif opzionale caricato dinamicamente.
use std::{fs::File, io::Read, path::Path};

pub fn header_dimensions(path: &Path) -> Option<(u32, u32)> {
    let mut bytes = vec![0u8; 256 * 1024];
    let n = File::open(path).ok()?.read(&mut bytes).ok()?;
    bytes.truncate(n);
    if bytes.get(4..8)? != b"ftyp" {
        return None;
    }
    for i in 4..bytes.len().saturating_sub(16) {
        if &bytes[i..i + 4] != b"ispe" {
            continue;
        }
        let size = u32::from_be_bytes(bytes[i - 4..i].try_into().ok()?) as usize;
        if size < 20 || i - 4 + size > bytes.len() {
            continue;
        }
        let w = u32::from_be_bytes(bytes[i + 8..i + 12].try_into().ok()?);
        let h = u32::from_be_bytes(bytes[i + 12..i + 16].try_into().ok()?);
        if w > 0 && h > 0 {
            return Some((w, h));
        }
    }
    None
}

#[cfg(feature = "heif")]
mod native {
    use super::*;
    use crate::thumb::{fit, Decoded};
    use libloading::Library;
    use parking_lot::Mutex;
    use pc_core::{AppError, AppResult, ErrorCode};
    use pc_governor::{budget::decode_cost_mb, MemoryGovernor};
    use std::{
        ffi::{c_char, c_void, CStr},
        io::{Seek, SeekFrom},
        path::PathBuf,
        sync::OnceLock,
    };

    static LIBRARY_PATH: OnceLock<PathBuf> = OnceLock::new();
    pub fn set_library_path(path: PathBuf) {
        let _ = LIBRARY_PATH.set(path);
    }
    pub fn library_path() -> PathBuf {
        if let Some(p) = LIBRARY_PATH.get() {
            return p.clone();
        }
        if let Some(p) = std::env::var_os("PHOTOCAT_HEIF_LIBRARY") {
            return p.into();
        }
        if cfg!(windows) {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/runtime/win-x64/heif.dll")
        } else if cfg!(target_os = "macos") {
            PathBuf::from("libheif.dylib")
        } else {
            PathBuf::from("libheif.so.1")
        }
    }
    /// Versione minima della libheif di sistema: `heif_image_get_plane_readonly2` è della 1.20.
    pub const MIN_SYSTEM_VERSION: &str = "1.20";

    /// Fuori da Windows libheif non è distribuita con l'app: si suggerisce come installarla.
    fn with_install_hint(detail: String) -> String {
        if cfg!(windows) {
            detail
        } else {
            format!(
                "{detail}. Per le foto HEIC installare libheif {MIN_SYSTEM_VERSION} o successiva \
                 con il decoder HEVC (su Debian/Ubuntu: libheif1 e libheif-plugin-libde265)"
            )
        }
    }

    #[repr(C)]
    struct Error {
        code: i32,
        _subcode: i32,
        message: *const c_char,
    }
    impl Error {
        fn check(self) -> AppResult<()> {
            if self.code == 0 {
                return Ok(());
            }
            let detail = if self.message.is_null() {
                "errore libheif".into()
            } else {
                unsafe { CStr::from_ptr(self.message) }
                    .to_string_lossy()
                    .into_owned()
            };
            let (code, detail) = match self.code {
                3 => (ErrorCode::UnsupportedFormat, detail),
                // Anche "nessun plugin di decodifica": su Linux manca il plugin HEVC di sistema.
                4 => (ErrorCode::UnsupportedFormat, with_install_hint(detail)),
                6 => (ErrorCode::InsufficientMemory, detail),
                _ => (ErrorCode::DecodeFailed, detail),
            };
            Err(AppError::new(code, detail))
        }
    }
    type Release = unsafe extern "C" fn(*mut c_void);
    struct Guard {
        p: *mut c_void,
        release: Release,
    }
    impl Drop for Guard {
        fn drop(&mut self) {
            unsafe { (self.release)(self.p) }
        }
    }

    #[repr(C)]
    struct Reader {
        version: i32,
        position: unsafe extern "C" fn(*mut c_void) -> i64,
        read: unsafe extern "C" fn(*mut c_void, usize, *mut c_void) -> i32,
        seek: unsafe extern "C" fn(i64, *mut c_void) -> i32,
        wait: unsafe extern "C" fn(i64, *mut c_void) -> i32,
        extensions: [Option<unsafe extern "C" fn()>; 4],
    }
    unsafe extern "C" fn position(data: *mut c_void) -> i64 {
        let file = unsafe { &*(data as *const Mutex<File>) };
        file.lock()
            .stream_position()
            .ok()
            .and_then(|x| x.try_into().ok())
            .unwrap_or(-1)
    }
    unsafe extern "C" fn read(out: *mut c_void, size: usize, data: *mut c_void) -> i32 {
        if size == 0 {
            return 0;
        }
        if out.is_null() || size > isize::MAX as usize {
            return 1;
        }
        let file = unsafe { &*(data as *const Mutex<File>) };
        let bytes = unsafe { std::slice::from_raw_parts_mut(out.cast::<u8>(), size) };
        i32::from(file.lock().read_exact(bytes).is_err())
    }
    unsafe extern "C" fn seek(pos: i64, data: *mut c_void) -> i32 {
        if pos < 0 {
            return 1;
        }
        let file = unsafe { &*(data as *const Mutex<File>) };
        i32::from(file.lock().seek(SeekFrom::Start(pos as u64)).is_err())
    }
    unsafe extern "C" fn wait(size: i64, data: *mut c_void) -> i32 {
        let file = unsafe { &*(data as *const Mutex<File>) };
        if size >= 0 && file.lock().metadata().is_ok_and(|m| m.len() >= size as u64) {
            0
        } else {
            2
        }
    }

    struct Api {
        _library: Library,
        _decoder: Option<Library>,
        deinit: unsafe extern "C" fn(),
        alloc: unsafe extern "C" fn() -> *mut c_void,
        free: Release,
        read: unsafe extern "C" fn(*mut c_void, *const Reader, *mut c_void, *const c_void) -> Error,
        primary: unsafe extern "C" fn(*mut c_void, *mut *mut c_void) -> Error,
        handle_free: Release,
        width: unsafe extern "C" fn(*const c_void) -> i32,
        height: unsafe extern "C" fn(*const c_void) -> i32,
        decode:
            unsafe extern "C" fn(*const c_void, *mut *mut c_void, i32, i32, *const c_void) -> Error,
        plane: unsafe extern "C" fn(*const c_void, i32, *mut usize) -> *const u8,
        plane_width: unsafe extern "C" fn(*const c_void, i32) -> i32,
        plane_height: unsafe extern "C" fn(*const c_void, i32) -> i32,
        image_free: Release,
    }
    impl Drop for Api {
        fn drop(&mut self) {
            unsafe { (self.deinit)() };
        }
    }
    impl Api {
        fn load(path: &Path) -> AppResult<Self> {
            let bad = |e: libloading::Error| {
                AppError::new(
                    ErrorCode::UnsupportedFormat,
                    with_install_hint(format!("libheif non disponibile: {e}")),
                )
            };
            // C API di libheif: su Windows la versione distribuita (1.23.7), altrove quella di
            // sistema, almeno MIN_SYSTEM_VERSION.
            unsafe {
                let dependency = path
                    .parent()
                    .map(|p| p.join("libde265.dll"))
                    .filter(|p| p.is_file());
                let decoder = dependency
                    .map(|p| Library::new(p))
                    .transpose()
                    .map_err(bad)?;
                let lib = Library::new(path).map_err(bad)?;
                macro_rules! sym {
                    ($name:literal,$ty:ty) => {
                        *lib.get::<$ty>(concat!($name, "\0").as_bytes())
                            .map_err(bad)?
                    };
                }
                let init = sym!("heif_init", unsafe extern "C" fn(*const c_void) -> Error);
                let api = Self {
                    deinit: sym!("heif_deinit", unsafe extern "C" fn()),
                    alloc: sym!("heif_context_alloc", unsafe extern "C" fn() -> *mut c_void),
                    free: sym!("heif_context_free", Release),
                    read: sym!(
                        "heif_context_read_from_reader",
                        unsafe extern "C" fn(
                            *mut c_void,
                            *const Reader,
                            *mut c_void,
                            *const c_void,
                        ) -> Error
                    ),
                    primary: sym!(
                        "heif_context_get_primary_image_handle",
                        unsafe extern "C" fn(*mut c_void, *mut *mut c_void) -> Error
                    ),
                    handle_free: sym!("heif_image_handle_release", Release),
                    width: sym!(
                        "heif_image_handle_get_width",
                        unsafe extern "C" fn(*const c_void) -> i32
                    ),
                    height: sym!(
                        "heif_image_handle_get_height",
                        unsafe extern "C" fn(*const c_void) -> i32
                    ),
                    decode: sym!(
                        "heif_decode_image",
                        unsafe extern "C" fn(
                            *const c_void,
                            *mut *mut c_void,
                            i32,
                            i32,
                            *const c_void,
                        ) -> Error
                    ),
                    plane: sym!(
                        "heif_image_get_plane_readonly2",
                        unsafe extern "C" fn(*const c_void, i32, *mut usize) -> *const u8
                    ),
                    plane_width: sym!(
                        "heif_image_get_width",
                        unsafe extern "C" fn(*const c_void, i32) -> i32
                    ),
                    plane_height: sym!(
                        "heif_image_get_height",
                        unsafe extern "C" fn(*const c_void, i32) -> i32
                    ),
                    image_free: sym!("heif_image_release", Release),
                    _library: lib,
                    _decoder: decoder,
                };
                init(std::ptr::null()).check()?;
                Ok(api)
            }
        }
    }
    fn with_handle<T>(
        path: &Path,
        library: &Path,
        f: impl FnOnce(&Api, *mut c_void) -> AppResult<T>,
    ) -> AppResult<T> {
        let api = Api::load(library)?;
        let file = Box::new(Mutex::new(File::open(path)?));
        let reader = Reader {
            version: 1,
            position,
            read,
            seek,
            wait,
            extensions: [None; 4],
        };
        let raw = unsafe { (api.alloc)() };
        if raw.is_null() {
            return Err(AppError::new(
                ErrorCode::DecodeFailed,
                "allocazione contesto HEIF fallita",
            ));
        }
        let ctx = Guard {
            p: raw,
            release: api.free,
        };
        let user = (&*file as *const Mutex<File>).cast_mut().cast::<c_void>();
        unsafe { (api.read)(ctx.p, &reader, user, std::ptr::null()) }.check()?;
        let mut raw_handle = std::ptr::null_mut();
        unsafe { (api.primary)(ctx.p, &mut raw_handle) }.check()?;
        if raw_handle.is_null() {
            return Err(AppError::new(
                ErrorCode::DecodeFailed,
                "immagine HEIF primaria mancante",
            ));
        }
        let handle = Guard {
            p: raw_handle,
            release: api.handle_free,
        };
        f(&api, handle.p)
    }
    pub fn dimensions(path: &Path) -> AppResult<(u32, u32)> {
        with_handle(path, &library_path(), |api, h| {
            let (w, h) = unsafe { ((api.width)(h), (api.height)(h)) };
            if w <= 0 || h <= 0 {
                return Err(AppError::new(
                    ErrorCode::DecodeFailed,
                    "dimensioni HEIF non valide",
                ));
            }
            Ok((w as u32, h as u32))
        })
    }
    pub fn decode_with_library(
        path: &Path,
        library: &Path,
        target: u32,
        gov: &MemoryGovernor,
    ) -> AppResult<Decoded> {
        let mut decoded = with_handle(path, library, |api, h| {
            let (width, height) = unsafe { ((api.width)(h), (api.height)(h)) };
            if width <= 0 || height <= 0 {
                return Err(AppError::new(
                    ErrorCode::DecodeFailed,
                    "dimensioni HEIF non valide",
                ));
            }
            let cost = (width as u64)
                .saturating_mul(height as u64)
                .saturating_mul(12)
                .saturating_add(std::fs::metadata(path)?.len())
                .div_ceil(1024 * 1024)
                .max(1);
            if cost > gov.capacity_mb() {
                return Err(AppError::new(
                    ErrorCode::InsufficientMemory,
                    "immagine HEIF superiore al budget di decodifica",
                ));
            }
            let permit = gov.acquire(cost);
            let mut raw = std::ptr::null_mut();
            unsafe { (api.decode)(h, &mut raw, 1, 10, std::ptr::null()) }.check()?;
            if raw.is_null() {
                return Err(AppError::new(
                    ErrorCode::DecodeFailed,
                    "buffer HEIF mancante",
                ));
            }
            let img = Guard {
                p: raw,
                release: api.image_free,
            };
            let (w, h) = unsafe { ((api.plane_width)(img.p, 10), (api.plane_height)(img.p, 10)) };
            if w <= 0
                || h <= 0
                || (w as u64).saturating_mul(h as u64) > (width as u64) * (height as u64)
            {
                return Err(AppError::new(
                    ErrorCode::DecodeFailed,
                    "piano HEIF non coerente",
                ));
            }
            let row = (w as usize).checked_mul(3).ok_or_else(|| {
                AppError::new(ErrorCode::DecodeFailed, "dimensione HEIF eccessiva")
            })?;
            let mut stride = 0usize;
            let data = unsafe { (api.plane)(img.p, 10, &mut stride) };
            let size = stride
                .checked_mul(h as usize)
                .filter(|n| *n <= isize::MAX as usize);
            if data.is_null() || stride < row || size.is_none() {
                return Err(AppError::new(
                    ErrorCode::DecodeFailed,
                    "piano RGB HEIF non valido",
                ));
            }
            let mut rgb = image::RgbImage::new(w as u32, h as u32);
            for y in 0..h as usize {
                let src = unsafe { std::slice::from_raw_parts(data.add(y * stride), row) };
                let pixels: &mut [u8] = rgb.as_mut();
                pixels[y * row..(y + 1) * row].copy_from_slice(src);
            }
            let image = if w.max(h) as u32 > target {
                fit(&rgb, target)
            } else {
                rgb
            };
            Ok(Decoded {
                image,
                source_size: (width as u32, height as u32),
                permit,
            })
        })?;
        decoded.permit.shrink_to(decode_cost_mb(
            decoded.image.width(),
            decoded.image.height(),
            1,
        ));
        Ok(decoded)
    }
    pub fn decode(path: &Path, target: u32, gov: &MemoryGovernor) -> AppResult<Decoded> {
        decode_with_library(path, &library_path(), target, gov)
    }
}
#[cfg(feature = "heif")]
pub use native::{
    decode, decode_with_library, dimensions, library_path, set_library_path, MIN_SYSTEM_VERSION,
};
