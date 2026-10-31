//! Verifica del runtime WebView2 prima di aprire la finestra.
//!
//! Il loader di WebView2 trova il runtime tramite il registro di EdgeUpdate. Se il registro
//! punta a una versione non più presente su disco (aggiornamento interrotto, Windows Sandbox
//! che condivide i file dell'host ma non il registro), l'app non si aprirebbe anche se un
//! runtime valido è installato: in quel caso lo si indica esplicitamente al loader.

use std::path::{Path, PathBuf};

const FOLDER_ENV: &str = "WEBVIEW2_BROWSER_EXECUTABLE_FOLDER";

/// Restituisce la cartella scelta quando il rilevamento standard fallisce (da registrare nei log).
pub fn ensure() -> Option<PathBuf> {
    if std::env::var_os(FOLDER_ENV).is_some() || standard_lookup_works() {
        return None;
    }
    let roots = [
        std::env::var_os("ProgramFiles(x86)").map(PathBuf::from),
        std::env::var_os("ProgramFiles").map(PathBuf::from),
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from),
    ];
    let folder = roots
        .into_iter()
        .flatten()
        .filter_map(|root| newest_runtime(&root.join(r"Microsoft\EdgeWebView\Application")))
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(_, dir)| dir)?;
    std::env::set_var(FOLDER_ENV, &folder);
    Some(folder)
}

fn standard_lookup_works() -> bool {
    let mut info = windows_core::PWSTR::null();
    // SAFETY: API del loader WebView2; `info` è un buffer allocato dal loader (non liberato:
    // pochi byte, una sola volta all'avvio).
    let res = unsafe {
        webview2_com::Microsoft::Web::WebView2::Win32::GetAvailableCoreWebView2BrowserVersionString(
            windows_core::PCWSTR::null(),
            &mut info,
        )
    };
    res.is_ok() && !info.is_null()
}

/// Cartella di versione più recente che contiene davvero `msedgewebview2.exe`.
pub fn newest_runtime(app_dir: &Path) -> Option<(Vec<u32>, PathBuf)> {
    std::fs::read_dir(app_dir)
        .ok()?
        .flatten()
        .filter_map(|e| {
            let version: Vec<u32> = e
                .file_name()
                .to_str()?
                .split('.')
                .map(|p| p.parse().ok())
                .collect::<Option<_>>()?;
            let dir = e.path();
            (version.len() == 4 && dir.join("msedgewebview2.exe").is_file())
                .then_some((version, dir))
        })
        .max_by(|a, b| a.0.cmp(&b.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_newest_complete_version_folder() {
        let tmp = tempfile::tempdir().unwrap();
        for (v, complete) in [
            ("155.0.4283.62", false),
            ("156.0.4314.50", true),
            ("99.0.1.2", true),
            ("SetupMetrics", false),
        ] {
            let d = tmp.path().join(v);
            std::fs::create_dir_all(&d).unwrap();
            if complete {
                std::fs::write(d.join("msedgewebview2.exe"), b"").unwrap();
            }
        }
        let (version, dir) = newest_runtime(tmp.path()).unwrap();
        assert_eq!(version, vec![156, 0, 4314, 50]);
        assert!(dir.ends_with("156.0.4314.50"));
        assert!(newest_runtime(&tmp.path().join("assente")).is_none());
    }
}
