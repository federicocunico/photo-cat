//! Concorrenza dinamica: quanti worker di una fase lavorano davvero in questo momento.
//!
//! I thread di una fase sono creati all'avvio del job, ma ognuno passa da [`ConcurrencyGate`]
//! prima di ogni elemento: il limite si ricalcola dalla capacità attuale del governor, così in
//! pressione di memoria (capacità ridotta) lavorano meno worker, e tornano tutti al rientro.

use crate::budget::recommended_workers;
use crate::MemoryGovernor;
use parking_lot::{Condvar, Mutex};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct ConcurrencyGate {
    inner: Arc<(Mutex<usize>, Condvar)>,
    gov: MemoryGovernor,
    per_worker_mb: u64,
    cores: usize,
    max: usize,
}

/// Posto occupato nel cancello; si libera quando esce di scena.
pub struct Slot {
    inner: Arc<(Mutex<usize>, Condvar)>,
}

impl Drop for Slot {
    fn drop(&mut self) {
        let (active, cv) = &*self.inner;
        *active.lock() -= 1;
        cv.notify_all();
    }
}

impl ConcurrencyGate {
    /// `per_worker_mb`: memoria stimata di un worker; `max`: thread creati per la fase.
    pub fn new(gov: MemoryGovernor, per_worker_mb: u64, cores: usize, max: usize) -> Self {
        Self {
            inner: Arc::new((Mutex::new(0), Condvar::new())),
            gov,
            per_worker_mb,
            cores,
            max: max.max(1),
        }
    }

    /// Worker ammessi ora, dalla capacità attuale del governor.
    pub fn limit(&self) -> usize {
        recommended_workers(self.gov.capacity_mb(), self.per_worker_mb, self.cores)
            .clamp(1, self.max)
    }

    /// Worker al lavoro in questo momento.
    pub fn active(&self) -> usize {
        *self.inner.0.lock()
    }

    /// Attende un posto; `keep_waiting` permette di interrompere l'attesa (annullamento).
    pub fn enter(&self, mut keep_waiting: impl FnMut() -> bool) -> Option<Slot> {
        let (active, cv) = &*self.inner;
        let mut n = active.lock();
        while *n >= self.limit() {
            if !keep_waiting() {
                return None;
            }
            cv.wait_for(&mut n, Duration::from_millis(100));
        }
        *n += 1;
        Some(Slot {
            inner: self.inner.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn workers_follow_the_governor_capacity_during_the_job() {
        let gov = MemoryGovernor::new(1600);
        let gate = ConcurrencyGate::new(gov.clone(), 160, 16, 8);
        assert_eq!(gate.limit(), 8);
        // Pressione: capacità dimezzata → meno worker, senza riavviare il job.
        gov.set_capacity(320);
        assert_eq!(gate.limit(), 2);
        let peak = Arc::new(AtomicUsize::new(0));
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let (g, peak) = (gate.clone(), peak.clone());
                std::thread::spawn(move || {
                    for _ in 0..5 {
                        let _slot = g.enter(|| true).unwrap();
                        peak.fetch_max(g.active(), Ordering::SeqCst);
                        std::thread::sleep(Duration::from_millis(5));
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        assert!(
            peak.load(Ordering::SeqCst) <= 2,
            "{}",
            peak.load(Ordering::SeqCst)
        );
        gov.set_capacity(1600);
        assert_eq!(gate.limit(), 8, "al rientro tornano tutti");
    }

    #[test]
    fn waiting_can_be_cancelled() {
        let gov = MemoryGovernor::new(160);
        let gate = ConcurrencyGate::new(gov, 160, 4, 4);
        let _busy = gate.enter(|| true).unwrap();
        let mut tries = 0;
        assert!(gate
            .enter(|| {
                tries += 1;
                tries < 3
            })
            .is_none());
    }
}
