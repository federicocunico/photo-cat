//! Errori raggruppati per l'interfaccia: invece di un evento per ogni file fallito,
//! un lotto ogni [`BATCH_INTERVAL`] con i conteggi per `(job, fase, codice)` e gli ultimi record.

use crate::ErrorRecord;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{Arc, Weak};
use std::time::Duration;

pub const BATCH_INTERVAL: Duration = Duration::from_millis(500);
/// Record completi inviati per lotto (i più recenti).
pub const BATCH_RECENT: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ErrorCount {
    pub job_id: Option<i64>,
    pub phase: Option<String>,
    pub code: Option<String>,
    pub level: String,
    pub count: u32,
}

/// Payload dell'evento `errors://batch`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ErrorBatch {
    pub counts: Vec<ErrorCount>,
    /// Ultimi record completi (al massimo [`BATCH_RECENT`]).
    pub recent: Vec<ErrorRecord>,
}

impl ErrorBatch {
    pub fn total(&self) -> u32 {
        self.counts.iter().map(|c| c.count).sum()
    }
}

type Key = (Option<i64>, Option<String>, Option<String>, String);

#[derive(Default)]
pub struct ErrorAggregator {
    inner: Mutex<(BTreeMap<Key, u32>, Vec<ErrorRecord>)>,
}

impl ErrorAggregator {
    pub fn push(&self, r: &ErrorRecord) {
        let mut g = self.inner.lock();
        let key = (r.job_id, r.phase.clone(), r.code.clone(), r.level.clone());
        *g.0.entry(key).or_default() += 1;
        if g.1.len() >= BATCH_RECENT {
            g.1.remove(0);
        }
        g.1.push(r.clone());
    }

    /// Il lotto accumulato, se c'è qualcosa da inviare.
    pub fn take(&self) -> Option<ErrorBatch> {
        let (counts, recent) = std::mem::take(&mut *self.inner.lock());
        if counts.is_empty() {
            return None;
        }
        Some(ErrorBatch {
            counts: counts
                .into_iter()
                .map(|((job_id, phase, code, level), count)| ErrorCount {
                    job_id,
                    phase,
                    code,
                    level,
                    count,
                })
                .collect(),
            recent,
        })
    }

    /// Invia i lotti con `emit` ogni `interval`, finché l'aggregatore esiste.
    pub fn spawn(self: &Arc<Self>, interval: Duration, emit: impl Fn(ErrorBatch) + Send + 'static) {
        let weak: Weak<Self> = Arc::downgrade(self);
        let _ = std::thread::Builder::new()
            .name("pc-error-events".into())
            .spawn(move || loop {
                std::thread::sleep(interval);
                let Some(agg) = weak.upgrade() else {
                    return;
                };
                if let Some(b) = agg.take() {
                    emit(b);
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn rec(i: usize, code: &str) -> ErrorRecord {
        ErrorRecord {
            ts: chrono::Utc::now(),
            level: "ERROR".into(),
            code: Some(code.into()),
            message: format!("errore {i}"),
            target: "pc_ingest".into(),
            job_id: Some(7),
            phase: Some("miniature".into()),
            path: Some(format!("D:\\Foto\\{i}.jpg")),
            detail: None,
        }
    }

    #[test]
    fn ten_thousand_errors_arrive_in_few_batches_with_exact_counts() {
        let agg = Arc::new(ErrorAggregator::default());
        let batches: Arc<Mutex<Vec<(Instant, ErrorBatch)>>> = Arc::default();
        let b2 = batches.clone();
        agg.spawn(Duration::from_millis(500), move |b| {
            b2.lock().push((Instant::now(), b))
        });
        let start = Instant::now();
        for i in 0..10_000 {
            agg.push(&rec(
                i,
                if i % 10 == 0 {
                    "UNSUPPORTED_FORMAT"
                } else {
                    "DECODE_FAILED"
                },
            ));
            if i % 1000 == 0 {
                std::thread::sleep(Duration::from_millis(100));
            }
        }
        std::thread::sleep(Duration::from_millis(1200));
        let elapsed = start.elapsed();
        let batches = batches.lock();
        let max = (elapsed.as_millis() / 500) as usize + 1;
        assert!(
            batches.len() <= max,
            "{} lotti in {elapsed:?}",
            batches.len()
        );
        for w in batches.windows(2) {
            assert!(
                w[1].0 - w[0].0 >= Duration::from_millis(450),
                "al massimo un evento ogni 500 ms"
            );
        }
        let total: u32 = batches.iter().map(|(_, b)| b.total()).sum();
        assert_eq!(total, 10_000);
        let decode: u32 = batches
            .iter()
            .flat_map(|(_, b)| &b.counts)
            .filter(|c| c.code.as_deref() == Some("DECODE_FAILED"))
            .map(|c| c.count)
            .sum();
        assert_eq!(decode, 9_000);
        assert!(batches.iter().all(|(_, b)| b.recent.len() <= BATCH_RECENT));
    }
}
