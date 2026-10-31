//! Contatori di avanzamento per fase, percentuali (per fase e complessiva
//! pesata), velocità su finestra scorrevole ed ETA smorzata.

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Dopo quanto tempo di attività di una fase si mostra l'ETA.
pub const ETA_AFTER: Duration = Duration::from_secs(10);
/// Finestra su cui si misura la velocità: abbastanza lunga da assorbire elementi lenti isolati.
const RATE_WINDOW: Duration = Duration::from_secs(30);
/// Distanza minima tra due punti della finestra (il campionamento avviene ogni ~100 ms).
const RATE_STEP: Duration = Duration::from_millis(500);
/// Costante di tempo con cui l'ETA mostrata converge verso la stima corrente:
/// tra un campione e l'altro l'ETA scende col tempo trascorso invece di oscillare.
const ETA_SMOOTHING_SECS: f64 = 15.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhaseSpec {
    pub name: String,
    /// Peso relativo nella percentuale complessiva (costo medio per elemento).
    pub weight: f64,
}

impl PhaseSpec {
    pub fn new(name: &str, weight: f64) -> Self {
        Self {
            name: name.into(),
            weight,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum PhaseState {
    Pending,
    Running,
    Done,
}

#[derive(Debug)]
struct Timing {
    started: Option<Instant>,
    finished: Option<Instant>,
    /// Punti (istante, elaborati) degli ultimi `RATE_WINDOW`.
    window: VecDeque<(Instant, i64)>,
    rate: f64,
    /// Ultima ETA mostrata (secondi) e quando è stata calcolata.
    eta: Option<(Instant, f64)>,
}

impl Timing {
    /// Aggiorna la finestra e la velocità media su di essa.
    fn update_rate(&mut self, now: Instant, processed: i64) {
        if self
            .window
            .back()
            .is_none_or(|(at, _)| now.saturating_duration_since(*at) >= RATE_STEP)
        {
            self.window.push_back((now, processed));
        }
        // Tiene un punto appena fuori finestra, così la finestra copre sempre RATE_WINDOW.
        while self.window.len() > 2
            && now.saturating_duration_since(self.window[1].0) >= RATE_WINDOW
        {
            self.window.pop_front();
        }
        if let (Some(&(t0, p0)), Some(&(t1, p1))) = (self.window.front(), self.window.back()) {
            let dt = t1.saturating_duration_since(t0).as_secs_f64();
            if dt >= RATE_STEP.as_secs_f64() {
                self.rate = (p1 - p0).max(0) as f64 / dt;
            }
        }
    }

    /// ETA smorzata: la precedente, scalata del tempo trascorso, converge verso `raw`.
    fn smooth_eta(&mut self, now: Instant, raw: f64) -> f64 {
        let next = match self.eta {
            Some((at, shown)) => {
                let dt = now.saturating_duration_since(at).as_secs_f64();
                let predicted = (shown - dt).max(0.0);
                let k = 1.0 - (-dt / ETA_SMOOTHING_SECS).exp();
                predicted + k * (raw - predicted)
            }
            None => raw,
        };
        self.eta = Some((now, next));
        next
    }
}

/// Contatori di una fase, condivisi tra i worker.
#[derive(Debug)]
pub struct PhaseCounters {
    pub spec: PhaseSpec,
    done: AtomicI64,
    failed: AtomicI64,
    total: AtomicI64,
    total_known: AtomicBool,
    timing: Mutex<Timing>,
}

impl PhaseCounters {
    fn new(spec: PhaseSpec) -> Self {
        Self {
            spec,
            done: AtomicI64::new(0),
            failed: AtomicI64::new(0),
            total: AtomicI64::new(0),
            total_known: AtomicBool::new(false),
            timing: Mutex::new(Timing {
                started: None,
                finished: None,
                window: VecDeque::new(),
                rate: 0.0,
                eta: None,
            }),
        }
    }

    pub fn name(&self) -> &str {
        &self.spec.name
    }

    pub fn start(&self) {
        let mut t = self.timing.lock();
        if t.started.is_none() {
            t.started = Some(Instant::now());
        }
    }

    pub fn finish(&self) {
        let mut t = self.timing.lock();
        t.started.get_or_insert_with(Instant::now);
        t.finished.get_or_insert_with(Instant::now);
        self.total_known.store(true, Ordering::Release);
    }

    pub fn inc_done(&self, n: i64) {
        self.start();
        self.done.fetch_add(n, Ordering::AcqRel);
    }

    pub fn inc_failed(&self, n: i64) {
        self.start();
        self.failed.fetch_add(n, Ordering::AcqRel);
    }

    pub fn add_total(&self, n: i64) {
        self.total.fetch_add(n, Ordering::AcqRel);
    }

    pub fn set_total(&self, n: i64) {
        self.total.store(n, Ordering::Release);
        self.total_known.store(true, Ordering::Release);
    }

    pub fn mark_total_known(&self) {
        self.total_known.store(true, Ordering::Release);
    }

    /// Ripristina i contatori (ripresa di un job persistito).
    pub fn restore(&self, done: i64, failed: i64, total: i64, total_known: bool) {
        self.done.store(done, Ordering::Release);
        self.failed.store(failed, Ordering::Release);
        self.total.store(total, Ordering::Release);
        self.total_known.store(total_known, Ordering::Release);
    }

    pub fn done(&self) -> i64 {
        self.done.load(Ordering::Acquire)
    }
    pub fn failed(&self) -> i64 {
        self.failed.load(Ordering::Acquire)
    }
    pub fn total(&self) -> i64 {
        self.total.load(Ordering::Acquire)
    }
    pub fn total_known(&self) -> bool {
        self.total_known.load(Ordering::Acquire)
    }

    pub fn duration(&self) -> Option<Duration> {
        let t = self.timing.lock();
        Some(t.finished.unwrap_or_else(Instant::now) - t.started?)
    }

    /// Frazione completata (0..1); `None` se il totale non è ancora noto.
    pub fn fraction(&self) -> Option<f64> {
        let processed = (self.done() + self.failed()) as f64;
        let total = self.total() as f64;
        if self.timing.lock().finished.is_some() {
            return Some(1.0);
        }
        if !self.total_known() {
            return None;
        }
        Some(if total <= 0.0 {
            1.0
        } else {
            (processed / total).clamp(0.0, 1.0)
        })
    }

    fn sample(&self, now: Instant) -> PhaseSnapshot {
        let done = self.done();
        let failed = self.failed();
        let total = self.total();
        let total_known = self.total_known();
        let fraction = self.fraction();
        let mut t = self.timing.lock();
        let processed = done + failed;
        if t.started.is_some() && t.finished.is_none() {
            t.update_rate(now, processed);
        }
        let state = match (t.started, t.finished) {
            (_, Some(_)) => PhaseState::Done,
            (Some(_), None) => PhaseState::Running,
            _ => PhaseState::Pending,
        };
        let active = t
            .started
            .map(|s| now.saturating_duration_since(s))
            .unwrap_or_default();
        let remaining = (total - processed).max(0) as f64;
        let eta_secs =
            if state == PhaseState::Running && total_known && active >= ETA_AFTER && t.rate > 0.0 {
                let raw = remaining / t.rate;
                Some(t.smooth_eta(now, raw).round() as u64)
            } else {
                None
            };
        PhaseSnapshot {
            name: self.spec.name.clone(),
            state,
            done,
            failed,
            total,
            total_known,
            percent: fraction.map(|f| (f * 1000.0).round() / 10.0),
            rate: (state == PhaseState::Running).then_some((t.rate * 10.0).round() / 10.0),
            eta_secs,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct PhaseSnapshot {
    pub name: String,
    pub state: PhaseState,
    pub done: i64,
    pub failed: i64,
    pub total: i64,
    pub total_known: bool,
    /// `None` = indeterminato (totale ancora sconosciuto).
    pub percent: Option<f64>,
    /// Elementi al secondo.
    pub rate: Option<f64>,
    pub eta_secs: Option<u64>,
}

/// Progresso di un job: una serie ordinata di fasi.
#[derive(Debug, Clone)]
pub struct JobProgress {
    phases: Arc<Vec<Arc<PhaseCounters>>>,
}

impl JobProgress {
    pub fn new(specs: &[PhaseSpec]) -> Self {
        Self {
            phases: Arc::new(
                specs
                    .iter()
                    .cloned()
                    .map(|s| Arc::new(PhaseCounters::new(s)))
                    .collect(),
            ),
        }
    }

    pub fn phase(&self, name: &str) -> Arc<PhaseCounters> {
        self.phases
            .iter()
            .find(|p| p.name() == name)
            .cloned()
            .unwrap_or_else(|| panic!("fase sconosciuta: {name}"))
    }

    pub fn phases(&self) -> &[Arc<PhaseCounters>] {
        &self.phases
    }

    /// Percentuale complessiva pesata; le fasi indeterminate contano 0.
    pub fn overall_percent(&self) -> f64 {
        let total_weight: f64 = self.phases.iter().map(|p| p.spec.weight.max(0.0)).sum();
        if total_weight <= 0.0 {
            return 0.0;
        }
        let acc: f64 = self
            .phases
            .iter()
            .map(|p| p.spec.weight.max(0.0) * p.fraction().unwrap_or(0.0))
            .sum();
        ((acc / total_weight) * 1000.0).round() / 10.0
    }

    pub fn failed(&self) -> i64 {
        self.phases.iter().map(|p| p.failed()).sum()
    }

    pub fn sample(&self, now: Instant) -> (Vec<PhaseSnapshot>, f64, Option<u64>) {
        let snaps: Vec<_> = self.phases.iter().map(|p| p.sample(now)).collect();
        // ETA del job: la più lunga tra le fasi attive o in attesa con ETA nota.
        let eta = snaps.iter().filter_map(|s| s.eta_secs).max();
        (snaps, self.overall_percent(), eta)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn progress() -> JobProgress {
        JobProgress::new(&[
            PhaseSpec::new("scansione", 1.0),
            PhaseSpec::new("analisi", 3.0),
        ])
    }

    #[test]
    fn phase_percent_and_indeterminate_total() {
        let p = progress();
        let scan = p.phase("scansione");
        scan.add_total(500);
        scan.inc_done(200);
        let (snaps, _, _) = p.sample(Instant::now());
        assert_eq!(snaps[0].percent, None, "totale non ancora noto");
        assert_eq!(snaps[0].done, 200);
        assert_eq!(snaps[0].state, PhaseState::Running);

        let ai = p.phase("analisi");
        ai.set_total(1000);
        ai.inc_done(600);
        ai.inc_failed(20);
        let (snaps, _, _) = p.sample(Instant::now());
        assert_eq!(snaps[1].percent, Some(62.0));
    }

    #[test]
    fn overall_is_weighted() {
        let p = progress();
        p.phase("scansione").finish();
        let ai = p.phase("analisi");
        ai.set_total(100);
        ai.inc_done(50);
        // (1×1 + 3×0.5) / 4 = 62.5%
        assert_eq!(p.overall_percent(), 62.5);
    }

    #[test]
    fn empty_total_is_complete() {
        let p = progress();
        p.phase("analisi").set_total(0);
        assert_eq!(p.phase("analisi").fraction(), Some(1.0));
    }

    #[test]
    fn rate_and_eta_after_ten_seconds() {
        let p = progress();
        let ai = p.phase("analisi");
        ai.set_total(10_000);
        ai.inc_done(0);
        let t0 = Instant::now();
        p.sample(t0);
        // 100 elementi/s per 12 secondi simulati.
        let mut eta = None;
        for s in 1..=12u64 {
            ai.inc_done(100);
            let (snaps, _, job_eta) = p.sample(t0 + Duration::from_secs(s));
            let rate = snaps[1].rate.unwrap();
            assert!((rate - 100.0).abs() < 1.0, "rate {rate}");
            // L'ETA compare solo dopo 10 s di attività.
            if s < 9 {
                assert_eq!(snaps[1].eta_secs, None, "s = {s}");
            }
            eta = job_eta;
        }
        // 10.000 − 1.200 = 8.800 rimanenti a 100/s → 88 s
        assert_eq!(eta, Some(88));
    }

    #[test]
    fn eta_is_stable_with_bursty_items_and_follows_a_real_slowdown() {
        let p = progress();
        let ai = p.phase("analisi");
        ai.set_total(2000);
        ai.inc_done(0);
        let t0 = Instant::now();
        let mut done = 0i64;
        let mut last: Option<u64> = None;
        let mut max_rise = 0i64;
        // Campioni ogni 100 ms come il ticker; un elemento ogni 400 ms (2,5/s), a raffiche.
        for tick in 0..1200u64 {
            if tick % 4 == 0 {
                ai.inc_done(1);
                done += 1;
            }
            let (snaps, _, eta) = p.sample(t0 + Duration::from_millis(tick * 100));
            if let (Some(prev), Some(cur)) = (last, eta) {
                max_rise = max_rise.max(cur as i64 - prev as i64);
            }
            last = eta.or(last);
            if tick == 1199 {
                let rate = snaps[1].rate.unwrap();
                assert!((rate - 2.5).abs() < 0.1, "rate {rate}");
            }
        }
        assert!(max_rise <= 1, "l'ETA non deve risalire: +{max_rise} s");
        let expected = (2000 - done) as f64 / 2.5;
        let eta = last.unwrap() as f64;
        assert!(
            (eta - expected).abs() / expected < 0.05,
            "eta {eta} vs {expected}"
        );

        // Rallentamento reale a 1/s: l'ETA converge verso il nuovo valore in meno di un minuto.
        for tick in 1200..2100u64 {
            if tick % 10 == 0 {
                ai.inc_done(1);
                done += 1;
            }
            last = p.sample(t0 + Duration::from_millis(tick * 100)).2.or(last);
        }
        let expected = (2000 - done) as f64;
        let eta = last.unwrap() as f64;
        assert!(
            (eta - expected).abs() / expected < 0.1,
            "eta {eta} vs {expected}"
        );
    }

    #[test]
    fn eta_hidden_before_ten_seconds() {
        let p = progress();
        let ai = p.phase("analisi");
        ai.set_total(1000);
        ai.inc_done(10);
        let t0 = Instant::now();
        p.sample(t0);
        ai.inc_done(10);
        let (snaps, _, eta) = p.sample(t0 + Duration::from_secs(2));
        assert!(snaps[1].rate.is_some());
        assert_eq!(snaps[1].eta_secs, None);
        assert_eq!(eta, None);
    }
}
