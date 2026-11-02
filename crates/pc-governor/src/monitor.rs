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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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
    /// Quota di memoria dei processi dell'interfaccia (WebView): oltre, il sistema è in
    /// pressione e l'interfaccia deve liberare le cache.
    pub ui_quota_mb: u64,
}

/// Quota predefinita della WebView (design: da tarare con la prova a 100.000 foto).
pub const DEFAULT_UI_QUOTA_MB: u64 = 1536;

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            pressure_pct: 0.10,
            pressure_min_mb: 1024,
            critical_pct: 0.05,
            critical_min_mb: 512,
            pressure_capacity: 0.5,
            ui_quota_mb: DEFAULT_UI_QUOTA_MB,
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
    /// Solo al cambio: `Some(true)` quando la memoria della WebView supera la quota
    /// (l'interfaccia deve liberare le cache), `Some(false)` quando rientra.
    #[serde(default)]
    pub ui_trim: Option<bool>,
}

pub struct PressureMonitor {
    probe: Arc<dyn SystemProbe>,
    gov: MemoryGovernor,
    thresholds: Thresholds,
    state: Mutex<PressureState>,
    ui_over: Mutex<bool>,
}

impl PressureMonitor {
    pub fn new(probe: Arc<dyn SystemProbe>, gov: MemoryGovernor, thresholds: Thresholds) -> Self {
        Self {
            probe,
            gov,
            thresholds,
            state: Mutex::new(PressureState::Normal),
            ui_over: Mutex::new(false),
        }
    }

    /// Una misura: aggiorna capacità/pausa (AI e decodifica) alle transizioni e restituisce
    /// lo snapshot. La memoria della WebView oltre la quota conta come pressione.
    pub fn tick(&self) -> ResourceSnapshot {
        let sys = self.probe.snapshot();
        let over = sys.webview_mb > self.thresholds.ui_quota_mb;
        let ui_trim = {
            let mut prev = self.ui_over.lock();
            (over != *prev).then(|| {
                *prev = over;
                if over {
                    tracing::warn!(
                        code = "INSUFFICIENT_MEMORY",
                        webview_mb = sys.webview_mb,
                        quota_mb = self.thresholds.ui_quota_mb,
                        app_mb = sys.process_mb,
                        available_mb = sys.available_mb,
                        "memoria dell'interfaccia oltre la quota: concorrenza ridotta, cache da liberare"
                    );
                } else {
                    tracing::info!(
                        webview_mb = sys.webview_mb,
                        "memoria dell'interfaccia rientrata nella quota"
                    );
                }
                over
            })
        };
        let ui_state = if over {
            PressureState::Pressure
        } else {
            PressureState::Normal
        };
        let next = self.thresholds.classify(&sys).max(ui_state);
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
                        app_mb = sys.process_mb,
                        webview_mb = sys.webview_mb,
                        "memoria di sistema critica: fasi AI e decodifica in pausa"
                    );
                }
            }
            *state = next;
        }
        ResourceSnapshot {
            system: sys,
            governor: self.gov.snapshot(),
            state: next,
            ui_trim,
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
    fn growing_ui_reduces_capacity_and_asks_to_trim_caches() {
        let (probe, gov, mon) = setup(10_000);
        assert_eq!(mon.tick().ui_trim, None);
        probe.set_webview(DEFAULT_UI_QUOTA_MB + 400);
        let tmp = tempfile::tempdir().unwrap();
        let (sub, diag) = pc_diag::build(tmp.path(), &pc_core::config::LoggingConfig::default());
        let sink = Arc::new(pc_diag::MemorySink::default());
        diag.hub().add_sink(sink.clone());
        let snap = tracing::subscriber::with_default(sub, || mon.tick());
        assert_eq!(snap.ui_trim, Some(true), "richiesta di liberare le cache");
        assert_eq!(snap.state, PressureState::Pressure);
        assert_eq!(gov.capacity_mb(), 2000, "concorrenza dell'import ridotta");
        assert_eq!(
            sink.errors.lock().last().and_then(|e| e.code.clone()),
            Some("INSUFFICIENT_MEMORY".into())
        );
        let lines = sink.lines.lock();
        let line = lines
            .iter()
            .find(|l| l.message.contains("interfaccia oltre la quota"))
            .expect("evento registrato");
        assert!(
            line.fields
                .iter()
                .any(|(k, v)| k == "webview_mb" && v == &(DEFAULT_UI_QUOTA_MB + 400).to_string()),
            "{:?}",
            line.fields
        );
        drop(lines);
        assert_eq!(mon.tick().ui_trim, None, "solo al cambio");
        probe.set_webview(300);
        let snap = mon.tick();
        assert_eq!(snap.ui_trim, Some(false));
        assert_eq!(snap.state, PressureState::Normal);
        assert_eq!(gov.capacity_mb(), 4000);
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
