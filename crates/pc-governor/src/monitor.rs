//! Monitor periodico della RAM di sistema: sotto la soglia di pressione riduce
//! la capacità del governor; sotto la soglia critica mette in pausa le fasi AI.
//! Pubblica uno snapshot delle risorse per la UI.

use crate::governor::{GovernorSnapshot, MemoryGovernor};
use crate::probe::{SystemProbe, SystemSnapshot};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum PressureState {
    Normal,
    Pressure,
    Critical,
}

#[derive(Debug, Clone, Copy)]
pub struct Thresholds {
    /// Pressione: disponibile < max(pressure_pct × totale, pressure_min_mb)
    pub pressure_pct: f64,
    pub pressure_min_mb: u64,
    pub critical_pct: f64,
    pub critical_min_mb: u64,
    /// Capacità in pressione, come frazione del budget.
    pub pressure_capacity: f64,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            pressure_pct: 0.10,
            pressure_min_mb: 1024,
            critical_pct: 0.05,
            critical_min_mb: 512,
            pressure_capacity: 0.5,
        }
    }
}

impl Thresholds {
    pub fn classify(&self, s: &SystemSnapshot) -> PressureState {
        let total = s.total_mb as f64;
        let critical = (total * self.critical_pct).max(self.critical_min_mb as f64);
        let pressure = (total * self.pressure_pct).max(self.pressure_min_mb as f64);
        let avail = s.available_mb as f64;
        if avail < critical {
            PressureState::Critical
        } else if avail < pressure {
            PressureState::Pressure
        } else {
            PressureState::Normal
        }
    }
}

/// Payload dell'evento `resources://tick`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ResourceSnapshot {
    pub system: SystemSnapshot,
    pub governor: GovernorSnapshot,
    pub state: PressureState,
}

pub struct PressureMonitor {
    probe: Arc<dyn SystemProbe>,
    gov: MemoryGovernor,
    thresholds: Thresholds,
    state: Mutex<PressureState>,
}

impl PressureMonitor {
    pub fn new(probe: Arc<dyn SystemProbe>, gov: MemoryGovernor, thresholds: Thresholds) -> Self {
        Self {
            probe,
            gov,
            thresholds,
            state: Mutex::new(PressureState::Normal),
        }
    }

    /// Una misura: aggiorna capacità/pausa AI alle transizioni e restituisce lo snapshot.
    pub fn tick(&self) -> ResourceSnapshot {
        let sys = self.probe.snapshot();
        let next = self.thresholds.classify(&sys);
        let mut state = self.state.lock();
        if next != *state {
            let budget = self.gov.snapshot().budget_mb;
            match next {
                PressureState::Normal => {
                    self.gov.set_capacity(budget);
                    self.gov.set_ai_paused(false);
                    tracing::info!(
                        available_mb = sys.available_mb,
                        "memoria di sistema tornata disponibile"
                    );
                }
                PressureState::Pressure => {
                    let cap = (budget as f64 * self.thresholds.pressure_capacity) as u64;
                    self.gov.set_capacity(cap.max(1));
                    self.gov.set_ai_paused(false);
                    tracing::warn!(
                        code = "INSUFFICIENT_MEMORY",
                        available_mb = sys.available_mb,
                        capacity_mb = cap,
                        "pressione di memoria: concorrenza ridotta"
                    );
                }
                PressureState::Critical => {
                    let in_use = self.gov.snapshot().in_use_mb;
                    self.gov.set_capacity(in_use.max(1));
                    self.gov.set_ai_paused(true);
                    tracing::warn!(
                        code = "INSUFFICIENT_MEMORY",
                        available_mb = sys.available_mb,
                        "memoria di sistema critica: fasi AI in pausa"
                    );
                }
            }
            *state = next;
        }
        ResourceSnapshot {
            system: sys,
            governor: self.gov.snapshot(),
            state: next,
        }
    }

    pub fn state(&self) -> PressureState {
        *self.state.lock()
    }

    /// Avvia il monitor su un thread: `on_tick` riceve ogni snapshot.
    pub fn spawn(
        self: Arc<Self>,
        interval: Duration,
        on_tick: impl Fn(ResourceSnapshot) + Send + 'static,
    ) -> MonitorHandle {
        let stop = Arc::new(AtomicBool::new(false));
        let stop2 = stop.clone();
        let handle = std::thread::Builder::new()
            .name("pc-governor-monitor".into())
            .spawn(move || {
                while !stop2.load(Ordering::Relaxed) {
                    on_tick(self.tick());
                    std::thread::park_timeout(interval);
                }
            })
            .expect("avvio thread monitor");
        MonitorHandle {
            stop,
            handle: Some(handle),
        }
    }
}

pub struct MonitorHandle {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Drop for MonitorHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            h.thread().unpark();
            let _ = h.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probe::FakeProbe;
    use std::time::Instant;

    fn setup(available: u64) -> (Arc<FakeProbe>, MemoryGovernor, PressureMonitor) {
        let probe = Arc::new(FakeProbe::new(16384, available, 8));
        let gov = MemoryGovernor::new(4000);
        let mon = PressureMonitor::new(probe.clone(), gov.clone(), Thresholds::default());
        (probe, gov, mon)
    }

    #[test]
    fn transitions_reduce_capacity_and_pause_ai() {
        let (probe, gov, mon) = setup(10_000);
        assert_eq!(mon.tick().state, PressureState::Normal);
        assert_eq!(gov.capacity_mb(), 4000);

        // 16 GB: pressione sotto ~1638 MB, critico sotto ~819 MB.
        probe.set_available(1500);
        assert_eq!(mon.tick().state, PressureState::Pressure);
        assert_eq!(gov.capacity_mb(), 2000);
        assert!(!gov.ai_paused());

        let _p = gov.acquire(700);
        probe.set_available(600);
        assert_eq!(mon.tick().state, PressureState::Critical);
        assert!(gov.ai_paused());
        assert_eq!(gov.capacity_mb(), 700);

        probe.set_available(9000);
        let snap = mon.tick();
        assert_eq!(snap.state, PressureState::Normal);
        assert!(!gov.ai_paused());
        assert_eq!(gov.capacity_mb(), 4000);
        assert_eq!(snap.governor.in_use_mb, 700);
    }

    #[test]
    fn transitions_are_logged() {
        let (probe, _gov, mon) = setup(10_000);
        let tmp = tempfile::tempdir().unwrap();
        let (sub, diag) = pc_diag::build(tmp.path(), &pc_core::config::LoggingConfig::default());
        let sink = Arc::new(pc_diag::MemorySink::default());
        diag.hub().add_sink(sink.clone());
        tracing::subscriber::with_default(sub, || {
            probe.set_available(500);
            mon.tick();
            mon.tick(); // nessuna nuova transizione → nessun nuovo log
        });
        assert_eq!(sink.errors.lock().len(), 1);
        assert_eq!(
            sink.errors.lock()[0].code.as_deref(),
            Some("INSUFFICIENT_MEMORY")
        );
    }

    #[test]
    fn snapshot_payload_and_frequency() {
        let (_probe, gov, mon) = setup(10_000);
        let _r = gov.try_reserve_resident("clip", 600).unwrap();
        let ticks = Arc::new(Mutex::new(Vec::<(Instant, ResourceSnapshot)>::new()));
        let t2 = ticks.clone();
        let handle = Arc::new(mon).spawn(Duration::from_millis(50), move |s| {
            t2.lock().push((Instant::now(), s))
        });
        std::thread::sleep(Duration::from_millis(330));
        drop(handle);
        let ticks = ticks.lock();
        assert!((5..=9).contains(&ticks.len()), "{} tick", ticks.len());
        let (_, s) = &ticks[0];
        assert_eq!(s.system.total_mb, 16384);
        assert_eq!(s.governor.budget_mb, 4000);
        assert_eq!(s.governor.residents, vec![("clip".to_string(), 600)]);
        let json = serde_json::to_value(s).unwrap();
        assert!(json["system"]["availableMb"].is_number());
        assert!(json["governor"]["inUseMb"].is_number());
        assert_eq!(json["state"], "normal");
    }
}
