//! Abort nativi: eccezioni non gestite (Windows) e segnali fatali (Unix).
//!
//! Il panic hook copre solo i panic Rust. Un accesso non valido in una libreria nativa
//! (ONNX Runtime, libheif, WebView2...) o un `abort()` chiude il processo senza passare da lì.
//! Qui si installa un gestore minimo che, prima della chiusura:
//! - scrive una riga in `logs/crash/native.log`;
//! - lascia il marker dei crash (`last_session_crashed`) con il codice dell'eccezione;
//! - su Windows scrive un minidump in `logs/crash/`.
//!
//! Il gestore gira in un processo già compromesso: usa solo buffer preparati all'installazione
//! e chiamate di sistema dirette, senza allocazioni né lock.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Sottocartella dei log con i file degli abort nativi e i minidump.
pub const CRASH_DIR: &str = "crash";
pub const NATIVE_LOG: &str = "native.log";

/// Percorsi preparati all'installazione (terminati da zero, pronti per le API di sistema).
struct Paths {
    #[cfg(windows)]
    dump_prefix: Vec<u16>,
    #[cfg(windows)]
    log: Vec<u16>,
    #[cfg(windows)]
    marker: Vec<u16>,
    #[cfg(unix)]
    log: std::ffi::CString,
    #[cfg(unix)]
    marker: std::ffi::CString,
}

static PATHS: OnceLock<Paths> = OnceLock::new();

pub fn crash_dir(logs_dir: &Path) -> PathBuf {
    logs_dir.join(CRASH_DIR)
}

/// Installa il gestore degli abort nativi (una volta per processo).
pub fn install(logs_dir: &Path) {
    let dir = crash_dir(logs_dir);
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let marker = crate::crash::marker_path(logs_dir);
    let log = dir.join(NATIVE_LOG);
    #[cfg(windows)]
    let paths = Paths {
        dump_prefix: wide(&dir.join("native-")),
        log: wide(&log),
        marker: wide(&marker),
    };
    #[cfg(unix)]
    let paths = {
        use std::os::unix::ffi::OsStrExt;
        let c = |p: &Path| std::ffi::CString::new(p.as_os_str().as_bytes()).unwrap_or_default();
        Paths {
            log: c(&log),
            marker: c(&marker),
        }
    };
    #[cfg(not(any(windows, unix)))]
    let _ = (marker, log);
    #[cfg(any(windows, unix))]
    if PATHS.set(paths).is_ok() {
        platform::install();
    }
}

/// Righe registrate dagli abort nativi dopo `since` (lette al riavvio per il log principale).
pub fn recorded_since(logs_dir: &Path, since: std::time::SystemTime) -> Vec<String> {
    let path = crash_dir(logs_dir).join(NATIVE_LOG);
    let recent = std::fs::metadata(&path)
        .and_then(|m| m.modified())
        .is_ok_and(|t| t >= since);
    if !recent {
        return Vec::new();
    }
    std::fs::read_to_string(path)
        .map(|t| t.lines().rev().take(5).map(str::to_string).collect())
        .unwrap_or_default()
}

#[cfg(windows)]
fn wide(p: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    p.as_os_str().encode_wide().chain(Some(0)).collect()
}

/// Scrive `value` (32 bit) in esadecimale a 8 cifre; restituisce la nuova posizione.
fn put_hex(buf: &mut [u8], at: usize, value: u64) -> usize {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut n = at;
    for shift in (0..8).rev() {
        let d = ((value >> (shift * 4)) & 0xF) as usize;
        if n < buf.len() {
            buf[n] = HEX[d];
            n += 1;
        }
    }
    n
}

fn put(buf: &mut [u8], at: usize, s: &[u8]) -> usize {
    let end = (at + s.len()).min(buf.len());
    buf[at..end].copy_from_slice(&s[..end - at]);
    end
}

/// Messaggio e JSON del marker per un codice, in buffer sullo stack.
fn render(code: u64, label: &[u8], line: &mut [u8; 256], json: &mut [u8; 256]) -> (usize, usize) {
    let mut n = put(line, 0, b"abort nativo: ");
    n = put(line, n, label);
    n = put(line, n, b" 0x");
    n = put_hex(line, n, code);
    n = put(line, n, b"\n");
    let mut j = put(json, 0, b"{\"message\":\"abort nativo: ");
    j = put(json, j, label);
    j = put(json, j, b" 0x");
    j = put_hex(json, j, code);
    j = put(
        json,
        j,
        b"\",\"location\":null,\"thread\":null,\"kind\":\"native\"}",
    );
    (n, j)
}

#[cfg(windows)]
mod platform {
    use super::{render, PATHS};
    use windows_sys::Win32::Foundation::{CloseHandle, GENERIC_WRITE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, SetFilePointer, WriteFile, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, FILE_END,
        FILE_SHARE_READ, OPEN_ALWAYS,
    };
    use windows_sys::Win32::System::Diagnostics::Debug::{
        MiniDumpWithThreadInfo, MiniDumpWriteDump, SetUnhandledExceptionFilter, EXCEPTION_POINTERS,
        MINIDUMP_EXCEPTION_INFORMATION,
    };
    use windows_sys::Win32::System::SystemInformation::GetTickCount;
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcess, GetCurrentProcessId, GetCurrentThreadId,
    };

    const EXCEPTION_CONTINUE_SEARCH: i32 = 0;

    pub fn install() {
        // SAFETY: il filtro è una funzione `extern "system"` valida per tutta la vita del processo.
        unsafe {
            SetUnhandledExceptionFilter(Some(filter));
        }
        // `abort()` del CRT passa da SIGABRT prima di chiudere il processo.
        // SAFETY: gestore `extern "C"` senza stato.
        unsafe {
            libc::signal(libc::SIGABRT, on_abort as *const () as libc::sighandler_t);
        }
    }

    extern "C" fn on_abort(_sig: libc::c_int) {
        record(0x16, b"SIGABRT", std::ptr::null_mut());
    }

    unsafe extern "system" fn filter(info: *const EXCEPTION_POINTERS) -> i32 {
        let code = if info.is_null() || unsafe { (*info).ExceptionRecord }.is_null() {
            0
        } else {
            unsafe { (*(*info).ExceptionRecord).ExceptionCode as u32 as u64 }
        };
        record(code, b"eccezione", info as *mut EXCEPTION_POINTERS);
        EXCEPTION_CONTINUE_SEARCH
    }

    fn write_file(path: &[u16], data: &[u8], append: bool) {
        // SAFETY: percorso terminato da zero preparato all'installazione; buffer validi.
        unsafe {
            let h = CreateFileW(
                path.as_ptr(),
                GENERIC_WRITE,
                FILE_SHARE_READ,
                std::ptr::null(),
                if append { OPEN_ALWAYS } else { CREATE_ALWAYS },
                FILE_ATTRIBUTE_NORMAL,
                std::ptr::null_mut(),
            );
            if h == INVALID_HANDLE_VALUE {
                return;
            }
            if append {
                SetFilePointer(h, 0, std::ptr::null_mut(), FILE_END);
            }
            let mut written = 0u32;
            WriteFile(
                h,
                data.as_ptr(),
                data.len() as u32,
                &mut written,
                std::ptr::null_mut(),
            );
            CloseHandle(h);
        }
    }

    fn record(code: u64, label: &[u8], info: *mut EXCEPTION_POINTERS) {
        let Some(p) = PATHS.get() else {
            return;
        };
        let (mut line, mut json) = ([0u8; 256], [0u8; 256]);
        let (n, j) = render(code, label, &mut line, &mut json);
        write_file(&p.log, &line[..n], true);
        write_file(&p.marker, &json[..j], false);

        // Minidump: <logs>/crash/native-<tick>.dmp
        let mut path = [0u16; 1024];
        let prefix = &p.dump_prefix[..p.dump_prefix.len() - 1];
        if prefix.len() + 16 >= path.len() {
            return;
        }
        path[..prefix.len()].copy_from_slice(prefix);
        let mut i = prefix.len();
        // SAFETY: chiamata senza argomenti.
        let tick = unsafe { GetTickCount() };
        for shift in (0..8).rev() {
            path[i] = b"0123456789abcdef"[((tick >> (shift * 4)) & 0xF) as usize] as u16;
            i += 1;
        }
        for c in ".dmp".encode_utf16() {
            path[i] = c;
            i += 1;
        }
        path[i] = 0;
        // SAFETY: percorso terminato da zero sullo stack; handle chiuso subito dopo.
        unsafe {
            let h = CreateFileW(
                path.as_ptr(),
                GENERIC_WRITE,
                0,
                std::ptr::null(),
                CREATE_ALWAYS,
                FILE_ATTRIBUTE_NORMAL,
                std::ptr::null_mut(),
            );
            if h == INVALID_HANDLE_VALUE {
                return;
            }
            let exc = MINIDUMP_EXCEPTION_INFORMATION {
                ThreadId: GetCurrentThreadId(),
                ExceptionPointers: info,
                ClientPointers: 0,
            };
            MiniDumpWriteDump(
                GetCurrentProcess(),
                GetCurrentProcessId(),
                h,
                MiniDumpWithThreadInfo,
                if info.is_null() {
                    std::ptr::null()
                } else {
                    &exc
                },
                std::ptr::null(),
                std::ptr::null(),
            );
            CloseHandle(h);
        }
    }
}

#[cfg(unix)]
mod platform {
    use super::{render, PATHS};

    const SIGNALS: [(libc::c_int, &[u8]); 5] = [
        (libc::SIGABRT, b"SIGABRT"),
        (libc::SIGSEGV, b"SIGSEGV"),
        (libc::SIGBUS, b"SIGBUS"),
        (libc::SIGILL, b"SIGILL"),
        (libc::SIGFPE, b"SIGFPE"),
    ];

    pub fn install() {
        for (sig, _) in SIGNALS {
            // SAFETY: `sigaction` con un gestore `extern "C"`; SA_RESETHAND ripristina il
            // comportamento predefinito, così il segnale chiude comunque il processo.
            unsafe {
                let mut action: libc::sigaction = std::mem::zeroed();
                action.sa_sigaction = handler as *const () as libc::sighandler_t;
                action.sa_flags = libc::SA_RESETHAND | libc::SA_ONSTACK;
                libc::sigemptyset(&mut action.sa_mask);
                libc::sigaction(sig, &action, std::ptr::null_mut());
            }
        }
    }

    extern "C" fn handler(sig: libc::c_int) {
        let Some(p) = PATHS.get() else {
            return;
        };
        let label = SIGNALS
            .iter()
            .find(|(s, _)| *s == sig)
            .map_or(&b"segnale"[..], |(_, l)| l);
        let (mut line, mut json) = ([0u8; 256], [0u8; 256]);
        let (n, j) = render(sig as u64, label, &mut line, &mut json);
        // SAFETY: solo chiamate async-signal-safe (open, write, close) su buffer preparati.
        unsafe {
            let fd = libc::open(
                p.log.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_APPEND,
                0o644,
            );
            if fd >= 0 {
                libc::write(fd, line.as_ptr().cast(), n);
                libc::close(fd);
            }
            let fd = libc::open(
                p.marker.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC,
                0o644,
            );
            if fd >= 0 {
                libc::write(fd, json.as_ptr().cast(), j);
                libc::close(fd);
            }
        }
    }
}

/// Solo sviluppo: provoca un accesso non valido per verificare il gestore.
#[cfg(debug_assertions)]
pub fn crash_for_test() -> ! {
    // SAFETY: nessuna; è proprio il crash che si vuole provocare.
    unsafe {
        std::ptr::write_volatile(std::ptr::null_mut::<u32>(), 1);
    }
    std::process::abort()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rendered_marker_is_valid_crash_json() {
        let (mut line, mut json) = ([0u8; 256], [0u8; 256]);
        let (n, j) = render(0xC000_0005, b"eccezione", &mut line, &mut json);
        assert_eq!(
            std::str::from_utf8(&line[..n]).unwrap(),
            "abort nativo: eccezione 0xC0000005\n"
        );
        // Il gestore non conosce l'ora: la lettura usa quella del file.
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(crate::crash::marker_path(tmp.path()), &json[..j]).unwrap();
        let c = crate::take_previous_crash(tmp.path()).unwrap();
        assert_eq!(c.kind, crate::CrashKind::Native);
        assert_eq!(c.message, "abort nativo: eccezione 0xC0000005");
        assert!((chrono::Utc::now() - c.ts).num_seconds() < 60);

        let (n, _) = render(6, b"SIGABRT", &mut line, &mut json);
        assert_eq!(
            std::str::from_utf8(&line[..n]).unwrap(),
            "abort nativo: SIGABRT 0x00000006\n"
        );
    }
}
