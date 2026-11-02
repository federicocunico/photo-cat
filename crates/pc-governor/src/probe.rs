//! Lettura delle risorse di sistema (RAM, core, memoria del processo).

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sysinfo::{MemoryRefreshKind, Pid, ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};

const MB: u64 = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SystemSnapshot {
    pub total_mb: u64,
    pub available_mb: u64,
    /// Memoria residente del processo di photo-cat.
    pub process_mb: u64,
    /// Memoria residente dei processi dell'interfaccia (WebView2 o WebKitGTK) discendenti
    /// dell'app; 0 dove non misurabile (macOS) o nei test.
    #[serde(default)]
    pub webview_mb: u64,
    pub logical_cores: usize,
    pub physical_cores: usize,
}

/// Nomi dei processi dell'interfaccia (senza estensione, minuscoli).
const WEBVIEW_PROCESSES: &[&str] = &["msedgewebview2", "webkitwebprocess", "webkitnetworkprocess"];
/// Intervallo minimo tra due misure dei processi della WebView (enumerare i processi costa).
const WEBVIEW_EVERY: std::time::Duration = std::time::Duration::from_secs(5);

/// Fonte delle misure: reale (`SysinfoProbe`) o simulata nei test.
pub trait SystemProbe: Send + Sync + 'static {
    fn snapshot(&self) -> SystemSnapshot;
}

pub struct SysinfoProbe {
    sys: Mutex<System>,
    webview: Mutex<Option<(std::time::Instant, u64)>>,
    pid: Option<Pid>,
    logical: usize,
    physical: usize,
}

impl Default for SysinfoProbe {
    fn default() -> Self {
        let sys = System::new_with_specifics(
            RefreshKind::new().with_memory(MemoryRefreshKind::new().with_ram()),
        );
        let logical = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        let physical = sys.physical_core_count().unwrap_or(logical).max(1);
        Self {
            sys: Mutex::new(sys),
            webview: Mutex::new(None),
            pid: sysinfo::get_current_pid().ok(),
            logical,
            physical,
        }
    }
}

impl SystemProbe for SysinfoProbe {
    fn snapshot(&self) -> SystemSnapshot {
        let mut sys = self.sys.lock();
        sys.refresh_memory_specifics(MemoryRefreshKind::new().with_ram());
        let process_mb = match self.pid {
            Some(pid) => {
                sys.refresh_processes_specifics(
                    ProcessesToUpdate::Some(&[pid]),
                    true,
                    ProcessRefreshKind::new().with_memory(),
                );
                sys.process(pid).map(|p| p.memory() / MB).unwrap_or(0)
            }
            None => 0,
        };
        let webview_mb = self.webview_mb(&mut sys);
        SystemSnapshot {
            total_mb: sys.total_memory() / MB,
            available_mb: sys.available_memory() / MB,
            process_mb,
            webview_mb,
            logical_cores: self.logical,
            physical_cores: self.physical,
        }
    }
}

impl SysinfoProbe {
    /// Somma della memoria dei processi della WebView discendenti dell'app (misura in cache
    /// per [`WEBVIEW_EVERY`]).
    fn webview_mb(&self, sys: &mut System) -> u64 {
        let Some(root) = self.pid else {
            return 0;
        };
        let mut cached = self.webview.lock();
        if let Some((at, mb)) = *cached {
            if at.elapsed() < WEBVIEW_EVERY {
                return mb;
            }
        }
        sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::new().with_memory(),
        );
        let procs: Vec<(Pid, Option<Pid>, String, u64)> = sys
            .processes()
            .iter()
            .map(|(pid, p)| {
                (
                    *pid,
                    p.parent(),
                    p.name().to_string_lossy().to_ascii_lowercase(),
                    p.memory(),
                )
            })
            .collect();
        let mb = webview_bytes(root, &procs) / MB;
        *cached = Some((std::time::Instant::now(), mb));
        mb
    }
}

/// Memoria dei processi della WebView discendenti di `root` (pid, padre, nome, byte).
fn webview_bytes(root: Pid, procs: &[(Pid, Option<Pid>, String, u64)]) -> u64 {
    let mut frontier = vec![root];
    let mut seen = std::collections::HashSet::from([root]);
    let mut total = 0;
    while let Some(parent) = frontier.pop() {
        for (pid, ppid, name, bytes) in procs {
            if *ppid != Some(parent) || !seen.insert(*pid) {
                continue;
            }
            let stem = name.strip_suffix(".exe").unwrap_or(name);
            if WEBVIEW_PROCESSES.contains(&stem) {
                total += bytes;
            }
            frontier.push(*pid);
        }
    }
    total
}

/// Sonda con valori impostabili, per i test.
#[derive(Default)]
pub struct FakeProbe {
    pub state: Mutex<SystemSnapshot>,
}

impl FakeProbe {
    pub fn new(total_mb: u64, available_mb: u64, cores: usize) -> Self {
        Self {
            state: Mutex::new(SystemSnapshot {
                total_mb,
                available_mb,
                process_mb: 100,
                webview_mb: 0,
                logical_cores: cores,
                physical_cores: cores.div_ceil(2).max(1),
            }),
        }
    }
    pub fn set_available(&self, mb: u64) {
        self.state.lock().available_mb = mb;
    }
    pub fn set_webview(&self, mb: u64) {
        self.state.lock().webview_mb = mb;
    }
}

impl SystemProbe for FakeProbe {
    fn snapshot(&self) -> SystemSnapshot {
        *self.state.lock()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_probe_values_are_plausible() {
        let s = SysinfoProbe::default().snapshot();
        assert!(s.total_mb >= 512, "{s:?}");
        assert!(s.available_mb > 0 && s.available_mb <= s.total_mb, "{s:?}");
        assert!(s.logical_cores >= 1 && s.physical_cores >= 1);
        assert!(s.physical_cores <= s.logical_cores);
        assert!(s.process_mb > 0, "{s:?}");
    }

    #[test]
    fn webview_memory_sums_descendant_webview_processes_only() {
        let p = Pid::from_u32;
        let procs = vec![
            (p(10), Some(p(1)), "photo-cat.exe".to_string(), 500),
            (p(11), Some(p(10)), "msedgewebview2.exe".to_string(), 100),
            (p(12), Some(p(11)), "msedgewebview2.exe".to_string(), 200),
            (p(13), Some(p(11)), "msedgewebview2.exe".to_string(), 300),
            // WebView di un'altra app: non conta.
            (p(20), Some(p(1)), "msedgewebview2.exe".to_string(), 9000),
            // Processo figlio che non è la WebView: non conta, ma i suoi figli sì.
            (p(14), Some(p(10)), "helper".to_string(), 50),
            (p(15), Some(p(14)), "WebKitWebProcess".to_lowercase(), 7),
        ];
        assert_eq!(webview_bytes(p(10), &procs), 607);
    }
}
