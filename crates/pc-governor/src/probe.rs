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
    pub logical_cores: usize,
    pub physical_cores: usize,
}

/// Fonte delle misure: reale (`SysinfoProbe`) o simulata nei test.
pub trait SystemProbe: Send + Sync + 'static {
    fn snapshot(&self) -> SystemSnapshot;
}

pub struct SysinfoProbe {
    sys: Mutex<System>,
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
        SystemSnapshot {
            total_mb: sys.total_memory() / MB,
            available_mb: sys.available_memory() / MB,
            process_mb,
            logical_cores: self.logical,
            physical_cores: self.physical,
        }
    }
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
                logical_cores: cores,
                physical_cores: cores.div_ceil(2).max(1),
            }),
        }
    }
    pub fn set_available(&self, mb: u64) {
        self.state.lock().available_mb = mb;
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
}
