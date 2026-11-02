//! Dump di crash da allegare al pacchetto diagnostico.
//!
//! - dump di WebView2 (`EBWebView/Crashpad/reports/*.dmp`) generati dopo l'ultima uscita regolare;
//! - minidump degli abort nativi e `native.log` in `logs/crash/`;
//! - marcatore dell'ultima sessione anomala (memoria e job attivi prima del crash).
//!
//! Limiti: al massimo [`MAX_DUMPS`] dump per fonte, in tutto al massimo [`MAX_TOTAL_BYTES`].

use crate::native::{crash_dir, NATIVE_LOG};
use crate::session::{last_clean_exit, LAST_ABNORMAL};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub const MAX_DUMPS: usize = 3;
pub const MAX_TOTAL_BYTES: u64 = 20 * 1024 * 1024;

/// File allegato al pacchetto, salvato come `crash/<name>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attachment {
    pub name: String,
    pub bytes: Vec<u8>,
}

/// Cartella dei dump di WebView2 dato il percorso dei dati della WebView (`EBWebView`).
pub fn crashpad_reports(webview_data: &Path) -> PathBuf {
    webview_data.join("Crashpad").join("reports")
}

/// Raccoglie gli allegati di crash: `crashpad` è la cartella `reports` di WebView2 (se esiste).
pub fn collect(logs_dir: &Path, crashpad: Option<&Path>) -> Vec<Attachment> {
    let since = last_clean_exit(logs_dir).map(SystemTime::from);
    let mut budget = MAX_TOTAL_BYTES;
    let mut out = Vec::new();
    // Piccoli file di testo per primi: spiegano i dump.
    for (path, name) in [
        (logs_dir.join(LAST_ABNORMAL), "session.json".to_string()),
        (crash_dir(logs_dir).join(NATIVE_LOG), NATIVE_LOG.to_string()),
    ] {
        push(&mut out, &mut budget, &path, name);
    }
    if let Some(dir) = crashpad {
        for path in recent_dumps(dir, since) {
            let name = format!("webview-{}", file_name(&path));
            push(&mut out, &mut budget, &path, name);
        }
    }
    for path in recent_dumps(&crash_dir(logs_dir), since) {
        let name = file_name(&path);
        push(&mut out, &mut budget, &path, name);
    }
    out
}

fn file_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn push(out: &mut Vec<Attachment>, budget: &mut u64, path: &Path, name: String) {
    let Ok(meta) = std::fs::metadata(path) else {
        return;
    };
    if !meta.is_file() || meta.len() > *budget {
        return;
    }
    if let Ok(bytes) = std::fs::read(path) {
        *budget -= bytes.len() as u64;
        out.push(Attachment { name, bytes });
    }
}

/// I `.dmp` più recenti di `dir` scritti dopo `since`, dal più recente.
fn recent_dumps(dir: &Path, since: Option<SystemTime>) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut dumps: Vec<(SystemTime, PathBuf)> = entries
        .flatten()
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("dmp"))
        })
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .filter(|(t, _)| since.is_none_or(|s| *t > s))
        .collect();
    dumps.sort_by(|a, b| b.0.cmp(&a.0));
    dumps.into_iter().take(MAX_DUMPS).map(|(_, p)| p).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::SessionMarker;
    use std::time::Duration;

    fn set_mtime(p: &Path, t: SystemTime) {
        std::fs::File::options()
            .write(true)
            .open(p)
            .unwrap()
            .set_modified(t)
            .unwrap();
    }

    #[test]
    fn webview_dumps_after_last_clean_exit_within_limits() {
        let tmp = tempfile::tempdir().unwrap();
        let logs = tmp.path().join("logs");
        std::fs::create_dir_all(&logs).unwrap();
        let reports = crashpad_reports(&tmp.path().join("EBWebView"));
        std::fs::create_dir_all(&reports).unwrap();

        // Uscita regolare un'ora fa, poi una sessione anomala.
        let (m, _) = SessionMarker::start(&logs, "1");
        m.finish();
        let clean = SystemTime::now() - Duration::from_secs(3600);
        set_mtime(&logs.join(crate::session::LAST_CLEAN_EXIT), clean);
        std::fs::write(
            logs.join(crate::session::LAST_CLEAN_EXIT),
            chrono::DateTime::<chrono::Utc>::from(clean).to_rfc3339(),
        )
        .unwrap();
        let (_m, _) = SessionMarker::start(&logs, "1");
        let (_m2, prev) = SessionMarker::start(&logs, "1");
        assert!(prev.is_some());

        let old = reports.join("old.dmp");
        std::fs::write(&old, b"vecchio").unwrap();
        set_mtime(&old, clean - Duration::from_secs(60));
        for i in 0..5 {
            let p = reports.join(format!("r{i}.dmp"));
            std::fs::write(&p, vec![b'x'; 10]).unwrap();
            set_mtime(&p, SystemTime::now() - Duration::from_secs(100 - i));
        }
        let huge = reports.join("huge.dmp");
        std::fs::write(&huge, vec![0u8; (MAX_TOTAL_BYTES + 1) as usize]).unwrap();
        set_mtime(&huge, SystemTime::now() - Duration::from_secs(200));
        std::fs::write(reports.join("note.txt"), b"x").unwrap();

        let got = collect(&logs, Some(&reports));
        let names: Vec<_> = got.iter().map(|a| a.name.as_str()).collect();
        assert!(names.contains(&"session.json"), "{names:?}");
        let webview: Vec<_> = names.iter().filter(|n| n.starts_with("webview-")).collect();
        assert_eq!(
            webview,
            vec![&"webview-r4.dmp", &"webview-r3.dmp", &"webview-r2.dmp"],
            "i 3 più recenti"
        );
        assert!(!names
            .iter()
            .any(|n| n.contains("old") || n.contains("huge")));
        let total: usize = got.iter().map(|a| a.bytes.len()).sum();
        assert!(total as u64 <= MAX_TOTAL_BYTES);
    }

    #[test]
    fn native_minidumps_are_included_and_missing_crashpad_is_fine() {
        let tmp = tempfile::tempdir().unwrap();
        let logs = tmp.path().to_path_buf();
        let crash = crash_dir(&logs);
        std::fs::create_dir_all(&crash).unwrap();
        std::fs::write(crash.join("native-0001.dmp"), b"MDMP").unwrap();
        std::fs::write(
            crash.join(NATIVE_LOG),
            b"abort nativo: SIGSEGV 0x0000000B\n",
        )
        .unwrap();
        let got = collect(&logs, Some(&tmp.path().join("assente")));
        let names: Vec<_> = got.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, vec!["native.log", "native-0001.dmp"]);
    }
}
