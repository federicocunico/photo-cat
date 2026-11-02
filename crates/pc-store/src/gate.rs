//! Richieste superate: conteggi e facet di un filtro che l'utente ha già cambiato non servono più.
//!
//! Ogni canale (es. "library-count") ricorda l'ultima richiesta arrivata. Una richiesta più
//! vecchia dell'ultima viene scartata prima di iniziare; se è già in corso, la sua query viene
//! interrotta con `sqlite3_interrupt` sulla connessione di lettura che la sta eseguendo.

use crate::{Store, StoreError};
use parking_lot::Mutex;
use rusqlite::{Connection, InterruptHandle};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Default)]
struct Channel {
    latest: u64,
    running: Vec<(u64, InterruptHandle)>,
}

/// Canali delle richieste (uno per tipo di query della UI).
#[derive(Default, Clone)]
pub struct Gates {
    channels: Arc<Mutex<HashMap<String, Channel>>>,
}

impl Gates {
    /// Esegue `f` come richiesta `id` del canale. `Ok(None)` se la richiesta è stata superata
    /// da una più recente (prima di iniziare o durante l'esecuzione).
    pub fn run<T>(
        &self,
        store: &Store,
        channel: &str,
        id: u64,
        f: impl FnOnce(&Connection) -> Result<T, StoreError>,
    ) -> Result<Option<T>, StoreError> {
        {
            let mut g = self.channels.lock();
            let ch = g.entry(channel.to_string()).or_default();
            if id < ch.latest {
                return Ok(None);
            }
            ch.latest = id;
            // Le richieste precedenti ancora in corso non servono più.
            ch.running.retain(|(rid, h)| {
                if *rid < id {
                    h.interrupt();
                    false
                } else {
                    true
                }
            });
        }
        store.read(|c| {
            {
                let mut g = self.channels.lock();
                let ch = g.entry(channel.to_string()).or_default();
                if id < ch.latest {
                    return Ok(None);
                }
                ch.running.push((id, c.get_interrupt_handle()));
            }
            let res = f(c);
            let superseded = {
                let mut g = self.channels.lock();
                let ch = g.entry(channel.to_string()).or_default();
                ch.running.retain(|(rid, _)| *rid != id);
                id < ch.latest
            };
            match res {
                _ if superseded => Ok(None),
                Ok(v) => Ok(Some(v)),
                Err(e) => Err(e),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StoreOptions;
    use std::time::{Duration, Instant};

    fn store() -> (tempfile::TempDir, Store) {
        let tmp = tempfile::tempdir().unwrap();
        let s = Store::open(&tmp.path().join("c.db"), StoreOptions::default()).unwrap();
        (tmp, s)
    }

    /// Query lenta (secondi) che si può interrompere.
    const SLOW: &str = "WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM n)
                        SELECT COUNT(*) FROM (SELECT x FROM n LIMIT 400000000)";

    #[test]
    fn newer_request_interrupts_the_running_one() {
        let (_t, s) = store();
        let gates = Gates::default();
        let (g2, s2) = (gates.clone(), s.clone());
        let start = Instant::now();
        let old = std::thread::spawn(move || {
            g2.run(&s2, "count", 1, |c| {
                Ok(c.query_row(SLOW, [], |r| r.get::<_, i64>(0))?)
            })
        });
        std::thread::sleep(Duration::from_millis(200));
        let new = gates
            .run(&s, "count", 2, |c| {
                Ok(c.query_row("SELECT 42", [], |r| r.get::<_, i64>(0))?)
            })
            .unwrap();
        assert_eq!(new, Some(42));
        let old = old.join().unwrap().unwrap();
        assert_eq!(old, None, "la richiesta superata non restituisce risultati");
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "query interrotta: {:?}",
            start.elapsed()
        );
    }

    #[test]
    fn late_old_request_is_discarded_and_channels_are_independent() {
        let (_t, s) = store();
        let gates = Gates::default();
        let one = |c: &Connection| Ok(c.query_row("SELECT 1", [], |r| r.get::<_, i64>(0))?);
        assert_eq!(gates.run(&s, "count", 5, one).unwrap(), Some(1));
        assert_eq!(
            gates.run(&s, "count", 4, one).unwrap(),
            None,
            "arrivata tardi"
        );
        assert_eq!(
            gates.run(&s, "facets", 1, one).unwrap(),
            Some(1),
            "altro canale"
        );
        // Dopo un'interruzione la connessione resta utilizzabile.
        assert_eq!(gates.run(&s, "count", 6, one).unwrap(), Some(1));
    }
}
