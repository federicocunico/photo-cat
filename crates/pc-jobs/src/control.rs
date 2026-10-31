//! Controllo di un job in esecuzione: annullamento e pausa cooperativi,
//! verificati dai worker tra un elemento e l'altro.

use parking_lot::{Condvar, Mutex};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Default)]
struct Flags {
    cancelled: bool,
    paused: bool,
}

#[derive(Debug, Clone, Default)]
pub struct JobControl {
    inner: Arc<(Mutex<Flags>, Condvar)>,
}

impl JobControl {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        let (lock, cv) = &*self.inner;
        lock.lock().cancelled = true;
        cv.notify_all();
    }

    pub fn pause(&self) {
        self.inner.0.lock().paused = true;
    }

    pub fn resume(&self) {
        let (lock, cv) = &*self.inner;
        lock.lock().paused = false;
        cv.notify_all();
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.0.lock().cancelled
    }

    pub fn is_paused(&self) -> bool {
        self.inner.0.lock().paused
    }

    /// Blocca mentre il job è in pausa. Restituisce `false` se il job è stato annullato.
    pub fn checkpoint(&self) -> bool {
        let (lock, cv) = &*self.inner;
        let mut f = lock.lock();
        while f.paused && !f.cancelled {
            cv.wait_for(&mut f, Duration::from_millis(500));
        }
        !f.cancelled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn checkpoint_blocks_while_paused_and_releases_on_resume_or_cancel() {
        let c = JobControl::new();
        assert!(c.checkpoint());
        c.pause();
        let c2 = c.clone();
        let t = std::thread::spawn(move || {
            let start = Instant::now();
            let ok = c2.checkpoint();
            (ok, start.elapsed())
        });
        std::thread::sleep(Duration::from_millis(80));
        c.resume();
        let (ok, waited) = t.join().unwrap();
        assert!(ok && waited >= Duration::from_millis(60));

        c.pause();
        let c3 = c.clone();
        let t = std::thread::spawn(move || c3.checkpoint());
        std::thread::sleep(Duration::from_millis(30));
        c.cancel();
        assert!(!t.join().unwrap());
    }
}
