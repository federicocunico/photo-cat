//! Semaforo pesato in MB: ogni operazione che consuma memoria chiede un
//! permesso pari al costo stimato e lo restituisce al termine. I modelli
//! caricati occupano una quota "residente" finché restano in memoria.
//!
//! La capacità può essere ridotta a runtime (pressione di memoria): i nuovi
//! permessi attendono finché l'uso non rientra.

use parking_lot::{Condvar, Mutex};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
struct State {
    budget_mb: u64,
    capacity_mb: u64,
    in_use_mb: u64,
    peak_mb: u64,
    residents: BTreeMap<String, u64>,
    ai_paused: bool,
}

#[derive(Debug, Clone)]
pub struct MemoryGovernor {
    inner: Arc<(Mutex<State>, Condvar)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct GovernorSnapshot {
    pub budget_mb: u64,
    pub capacity_mb: u64,
    pub in_use_mb: u64,
    pub peak_mb: u64,
    pub residents: Vec<(String, u64)>,
    pub ai_paused: bool,
}

/// Permesso di memoria: restituito automaticamente al drop.
#[derive(Debug)]
#[must_use = "il permesso va tenuto finché la memoria è in uso"]
pub struct Permit {
    gov: MemoryGovernor,
    mb: u64,
}

impl Permit {
    pub fn mb(&self) -> u64 {
        self.mb
    }
    /// Libera la quota di un buffer temporaneo dopo averlo ridotto.
    pub fn shrink_to(&mut self, mb: u64) {
        let next = mb.max(1).min(self.mb);
        self.gov.release(self.mb - next);
        self.mb = next;
    }
}

impl Drop for Permit {
    fn drop(&mut self) {
        self.gov.release(self.mb);
    }
}

/// Quota residente di un modello caricato.
#[derive(Debug)]
#[must_use]
pub struct Resident {
    gov: MemoryGovernor,
    name: String,
    mb: u64,
}

impl Resident {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn mb(&self) -> u64 {
        self.mb
    }
    /// Corregge la stima con il valore misurato dopo il caricamento.
    /// Restituisce false senza modificare la quota se il valore non entra più.
    pub fn adjust(&mut self, measured_mb: u64) -> bool {
        let (lock, cv) = &*self.gov.inner;
        let mut s = lock.lock();
        let Some(adjusted) = s.in_use_mb.saturating_sub(self.mb).checked_add(measured_mb) else {
            return false;
        };
        if adjusted > s.capacity_mb {
            return false;
        }
        s.in_use_mb = adjusted;
        s.peak_mb = s.peak_mb.max(s.in_use_mb);
        s.residents.insert(self.name.clone(), measured_mb);
        self.mb = measured_mb;
        cv.notify_all();
        true
    }
}

impl Drop for Resident {
    fn drop(&mut self) {
        let (lock, cv) = &*self.gov.inner;
        let mut s = lock.lock();
        s.in_use_mb = s.in_use_mb.saturating_sub(self.mb);
        s.residents.remove(&self.name);
        cv.notify_all();
    }
}

impl MemoryGovernor {
    pub fn new(budget_mb: u64) -> Self {
        let state = State {
            budget_mb,
            capacity_mb: budget_mb,
            ..Default::default()
        };
        Self {
            inner: Arc::new((Mutex::new(state), Condvar::new())),
        }
    }

    /// Un costo superiore alla capacità viene ridotto alla capacità, così
    /// un'operazione molto grande può comunque partire (da sola).
    fn clamp(s: &State, mb: u64) -> u64 {
        mb.max(1).min(s.capacity_mb.max(1))
    }

    /// Attende finché il costo entra nella capacità residua.
    pub fn acquire(&self, mb: u64) -> Permit {
        self.acquire_timeout(mb, None)
            .expect("attesa senza timeout")
    }

    /// Come [`acquire`](Self::acquire), con un tempo massimo di attesa.
    pub fn acquire_timeout(&self, mb: u64, timeout: Option<Duration>) -> Option<Permit> {
        let deadline = timeout.map(|t| Instant::now() + t);
        let (lock, cv) = &*self.inner;
        let mut s = lock.lock();
        loop {
            let need = Self::clamp(&s, mb);
            if s.in_use_mb + need <= s.capacity_mb || s.in_use_mb == 0 {
                s.in_use_mb += need;
                s.peak_mb = s.peak_mb.max(s.in_use_mb);
                return Some(Permit {
                    gov: self.clone(),
                    mb: need,
                });
            }
            match deadline {
                Some(d) => {
                    if cv.wait_until(&mut s, d).timed_out() {
                        return None;
                    }
                }
                None => cv.wait(&mut s),
            }
        }
    }

    pub fn try_acquire(&self, mb: u64) -> Option<Permit> {
        self.acquire_timeout(mb, Some(Duration::ZERO))
    }

    fn release(&self, mb: u64) {
        let (lock, cv) = &*self.inner;
        let mut s = lock.lock();
        s.in_use_mb = s.in_use_mb.saturating_sub(mb);
        cv.notify_all();
    }

    /// Riserva la memoria di un modello se entra nella capacità residua.
    pub fn try_reserve_resident(&self, name: &str, mb: u64) -> Option<Resident> {
        let (lock, cv) = &*self.inner;
        let mut s = lock.lock();
        if s.residents.contains_key(name) || s.in_use_mb + mb > s.capacity_mb {
            return None;
        }
        s.in_use_mb += mb;
        s.peak_mb = s.peak_mb.max(s.in_use_mb);
        s.residents.insert(name.to_string(), mb);
        cv.notify_all();
        Some(Resident {
            gov: self.clone(),
            name: name.to_string(),
            mb,
        })
    }

    /// Memoria residua disponibile ora.
    pub fn free_mb(&self) -> u64 {
        let s = self.inner.0.lock();
        s.capacity_mb.saturating_sub(s.in_use_mb)
    }

    pub fn capacity_mb(&self) -> u64 {
        self.inner.0.lock().capacity_mb
    }

    /// Nuovo budget (es. cambio profilo): la capacità torna pari al budget.
    pub fn set_budget(&self, budget_mb: u64) {
        let (lock, cv) = &*self.inner;
        let mut s = lock.lock();
        s.budget_mb = budget_mb;
        s.capacity_mb = budget_mb;
        cv.notify_all();
    }

    /// Capacità temporanea (pressione di memoria), mai sopra il budget.
    pub fn set_capacity(&self, capacity_mb: u64) {
        let (lock, cv) = &*self.inner;
        let mut s = lock.lock();
        s.capacity_mb = capacity_mb.min(s.budget_mb);
        cv.notify_all();
    }

    pub fn set_ai_paused(&self, paused: bool) {
        let (lock, cv) = &*self.inner;
        lock.lock().ai_paused = paused;
        cv.notify_all();
    }

    pub fn ai_paused(&self) -> bool {
        self.inner.0.lock().ai_paused
    }

    /// Blocca finché le fasi AI sono in pausa per pressione (o fino al timeout).
    pub fn wait_ai_resumed(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let (lock, cv) = &*self.inner;
        let mut s = lock.lock();
        while s.ai_paused {
            if cv.wait_until(&mut s, deadline).timed_out() {
                return !s.ai_paused;
            }
        }
        true
    }

    pub fn snapshot(&self) -> GovernorSnapshot {
        let s = self.inner.0.lock();
        GovernorSnapshot {
            budget_mb: s.budget_mb,
            capacity_mb: s.capacity_mb,
            in_use_mb: s.in_use_mb,
            peak_mb: s.peak_mb,
            residents: s.residents.iter().map(|(k, v)| (k.clone(), *v)).collect(),
            ai_paused: s.ai_paused,
        }
    }

    pub fn reset_peak(&self) {
        let mut s = self.inner.0.lock();
        s.peak_mb = s.in_use_mb;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::budget::decode_cost_mb;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[test]
    fn reduced_buffer_releases_temporary_memory_quota() {
        let gov = MemoryGovernor::new(100);
        let mut permit = gov.acquire(80);
        permit.shrink_to(5);
        assert_eq!(gov.snapshot().in_use_mb, 5);
        let remaining = gov.try_acquire(95).unwrap();
        drop(permit);
        drop(remaining);
        assert_eq!(gov.snapshot().in_use_mb, 0);
    }

    #[test]
    fn concurrent_estimated_use_never_exceeds_budget() {
        let gov = MemoryGovernor::new(2048);
        let live = Arc::new(AtomicU64::new(0));
        let max_seen = Arc::new(AtomicU64::new(0));
        let handles: Vec<_> = (0..16)
            .map(|i| {
                let gov = gov.clone();
                let live = live.clone();
                let max_seen = max_seen.clone();
                std::thread::spawn(move || {
                    for j in 0..30 {
                        // Immagini da 12 a 50 MP decodificate a piena risoluzione.
                        let mp = 12 + ((i * 7 + j * 3) % 39) as u32;
                        let cost = decode_cost_mb(mp * 1000, 1000, 1);
                        let p = gov.acquire(cost);
                        let now = live.fetch_add(p.mb(), Ordering::SeqCst) + p.mb();
                        max_seen.fetch_max(now, Ordering::SeqCst);
                        std::thread::sleep(Duration::from_micros(200));
                        live.fetch_sub(p.mb(), Ordering::SeqCst);
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        assert!(
            max_seen.load(Ordering::SeqCst) <= 2048,
            "{}",
            max_seen.load(Ordering::SeqCst)
        );
        assert!(gov.snapshot().peak_mb <= 2048);
        assert_eq!(gov.snapshot().in_use_mb, 0);
    }

    #[test]
    fn oversized_cost_runs_alone() {
        let gov = MemoryGovernor::new(100);
        let big = gov.acquire(5000);
        assert_eq!(big.mb(), 100);
        assert!(gov.try_acquire(1).is_none());
        drop(big);
        assert!(gov.try_acquire(1).is_some());
    }

    #[test]
    fn residents_count_against_capacity() {
        let gov = MemoryGovernor::new(1000);
        let mut clip = gov.try_reserve_resident("clip", 600).unwrap();
        assert!(gov.try_reserve_resident("faces", 500).is_none());
        assert!(clip.adjust(450));
        let faces = gov.try_reserve_resident("faces", 500).unwrap();
        assert_eq!(
            gov.snapshot().residents,
            vec![("clip".into(), 450), ("faces".into(), 500)]
        );
        assert!(gov.try_acquire(100).is_none());
        drop(clip);
        assert!(gov.try_acquire(100).is_some());
        drop(faces);
        assert_eq!(gov.snapshot().in_use_mb, 0);
    }

    #[test]
    fn measured_resident_cost_cannot_exceed_capacity() {
        let gov = MemoryGovernor::new(100);
        let mut resident = gov.try_reserve_resident("model", 40).unwrap();
        let work = gov.acquire(30);
        assert!(!resident.adjust(80));
        assert_eq!(resident.mb(), 40);
        assert_eq!(gov.snapshot().in_use_mb, 70);
        drop(work);
        assert!(resident.adjust(80));
        drop(resident);
        assert_eq!(gov.snapshot().in_use_mb, 0);
    }

    #[test]
    fn reduced_capacity_blocks_new_permits_until_released() {
        let gov = MemoryGovernor::new(1000);
        let a = gov.acquire(400);
        gov.set_capacity(300);
        assert!(gov.try_acquire(10).is_none());
        let g2 = gov.clone();
        let waiter = std::thread::spawn(move || {
            g2.acquire_timeout(100, Some(Duration::from_secs(5)))
                .is_some()
        });
        std::thread::sleep(Duration::from_millis(50));
        drop(a);
        assert!(waiter.join().unwrap());
        gov.set_capacity(5000);
        assert_eq!(gov.capacity_mb(), 1000, "mai sopra il budget");
    }
}
