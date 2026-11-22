//! Controllo di un job in esecuzione: annullamento e pausa cooperativi,
//! verificati dai worker tra un elemento e l'altro.

use parking_lot::{Condvar, Mutex};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

/// Perché un job si è fermato da solo (la UI lo mostra accanto allo stato "In pausa").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct PauseReason {
    /// Codice stabile, tradotto dalla UI (`jobs.pauseReasons.<codice>`).
    pub code: String,
    /// Dettaglio per l'utente (causa e azione suggerita), già in italiano.
    pub detail: String,
}

#[derive(Debug, Default)]
struct Flags {
    cancelled: bool,
    paused: bool,
    reason: Option<PauseReason>,
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

    /// Pausa decisa dal job stesso (disco pieno, periferica scollegata…): resta in pausa finché
    /// l'utente non preme "Riprendi" o annulla, e la UI mostra il motivo.
    pub fn pause_for(&self, reason: PauseReason) {
        let mut f = self.inner.0.lock();
        f.paused = true;
        f.reason = Some(reason);
    }

    /// Il motivo della pausa, se è stato il job a fermarsi.
    pub fn pause_reason(&self) -> Option<PauseReason> {
        self.inner.0.lock().reason.clone()
    }

    pub fn resume(&self) {
        let (lock, cv) = &*self.inner;
        {
            let mut f = lock.lock();
            f.paused = false;
            f.reason = None;
        }
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

    #[test]
    fn pause_for_keeps_the_reason_until_resume() {
        let c = JobControl::new();
        assert!(c.pause_reason().is_none());
        c.pause_for(PauseReason {
            code: "disk_space".into(),
            detail: "Libera spazio".into(),
        });
        assert!(c.is_paused());
        assert_eq!(c.pause_reason().unwrap().code, "disk_space");
        c.resume();
        assert!(!c.is_paused());
        assert!(c.pause_reason().is_none());
        // Una pausa dell'utente non ha motivo.
        c.pause();
        assert!(c.pause_reason().is_none());
    }
}
