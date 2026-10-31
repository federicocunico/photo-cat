//! Nomi delle librerie native distribuite negli asset, per piattaforma.
//!
//! Gli asset del repository tengono un runtime per piattaforma in `runtime/<cartella>/`;
//! la build distribuita li rimappa in `runtime/` (vedi `tauri.conf.json` e `tauri.linux.conf.json`).

/// Cartella degli asset nativi della piattaforma corrente (`None` se non distribuiti).
pub fn asset_dir() -> Option<&'static str> {
    if cfg!(all(windows, target_arch = "x86_64")) {
        Some("win-x64")
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some("linux-x64")
    } else {
        None
    }
}

/// Nome del file della libreria ONNX Runtime per la piattaforma corrente.
pub fn onnxruntime_file() -> &'static str {
    if cfg!(windows) {
        "onnxruntime.dll"
    } else if cfg!(target_os = "macos") {
        "libonnxruntime.dylib"
    } else {
        "libonnxruntime.so"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_match_the_current_platform() {
        #[cfg(all(windows, target_arch = "x86_64"))]
        assert_eq!(
            (asset_dir(), onnxruntime_file()),
            (Some("win-x64"), "onnxruntime.dll")
        );
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        assert_eq!(
            (asset_dir(), onnxruntime_file()),
            (Some("linux-x64"), "libonnxruntime.so")
        );
        #[cfg(target_os = "macos")]
        assert_eq!(
            (asset_dir(), onnxruntime_file()),
            (None, "libonnxruntime.dylib")
        );
    }
}
