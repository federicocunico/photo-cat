//! Layer `tracing` che trasforma gli eventi in record diagnostici:
//! - WARN/ERROR → [`ErrorRecord`] verso i sink registrati (registro errori su DB);
//! - ogni evento abilitato → [`LogLine`] nella coda in memoria e verso i sink (UI).
//!
//! I campi strutturati riconosciuti sono `code`, `job_id`, `phase`, `path`, `detail`.
//! Gli errori emessi prima che esista un sink vengono trattenuti (fino a un limite)
//! e consegnati al primo sink registrato.

use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fmt;
use std::sync::Arc;
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

const PENDING_LIMIT: usize = 1_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ErrorRecord {
    pub ts: DateTime<Utc>,
    /// `ERROR` o `WARN`.
    pub level: String,
    pub code: Option<String>,
    pub message: String,
    pub target: String,
    pub job_id: Option<i64>,
    pub phase: Option<String>,
    pub path: Option<String>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct LogLine {
    pub ts: DateTime<Utc>,
    pub level: String,
    pub target: String,
    pub message: String,
    pub fields: Vec<(String, String)>,
}

/// Destinazione dei record diagnostici (DB, UI, test).
pub trait DiagSink: Send + Sync + 'static {
    fn on_error(&self, record: &ErrorRecord);
    fn on_log(&self, _line: &LogLine) {}
}

#[derive(Default)]
struct Inner {
    sinks: Vec<Arc<dyn DiagSink>>,
    pending: VecDeque<ErrorRecord>,
    tail: VecDeque<LogLine>,
}

/// Punto di aggancio condiviso tra il layer e il resto dell'app.
#[derive(Clone)]
pub struct DiagHub {
    inner: Arc<Mutex<Inner>>,
    tail_capacity: usize,
}

impl DiagHub {
    pub fn new(tail_capacity: usize) -> Self {
        Self {
            inner: Arc::default(),
            tail_capacity: tail_capacity.max(1),
        }
    }

    /// Registra un sink e gli consegna gli errori trattenuti.
    pub fn add_sink(&self, sink: Arc<dyn DiagSink>) {
        let mut inner = self.inner.lock();
        for rec in inner.pending.drain(..) {
            sink.on_error(&rec);
        }
        inner.sinks.push(sink);
    }

    /// Ultime righe di log (per la coda nella vista Diagnostica).
    pub fn tail(&self) -> Vec<LogLine> {
        self.inner.lock().tail.iter().cloned().collect()
    }

    fn dispatch(&self, line: LogLine, error: Option<ErrorRecord>) {
        let mut inner = self.inner.lock();
        if inner.tail.len() == self.tail_capacity {
            inner.tail.pop_front();
        }
        inner.tail.push_back(line.clone());
        let sinks = inner.sinks.clone();
        if let Some(rec) = &error {
            if sinks.is_empty() {
                if inner.pending.len() == PENDING_LIMIT {
                    inner.pending.pop_front();
                }
                inner.pending.push_back(rec.clone());
            }
        }
        drop(inner);
        for sink in sinks {
            if let Some(rec) = &error {
                sink.on_error(rec);
            }
            sink.on_log(&line);
        }
    }
}

pub struct DiagLayer {
    hub: DiagHub,
}

impl DiagLayer {
    pub fn new(hub: DiagHub) -> Self {
        Self { hub }
    }
}

#[derive(Default)]
struct FieldCollector {
    message: Option<String>,
    code: Option<String>,
    job_id: Option<i64>,
    phase: Option<String>,
    path: Option<String>,
    detail: Option<String>,
    others: Vec<(String, String)>,
}

impl FieldCollector {
    fn set(&mut self, field: &Field, value: String) {
        match field.name() {
            "message" => self.message = Some(value),
            "code" => self.code = Some(value),
            "phase" => self.phase = Some(value),
            "path" => self.path = Some(value),
            "detail" => self.detail = Some(value),
            "job_id" => self.job_id = value.parse().ok(),
            name => self.others.push((name.to_string(), value)),
        }
    }
}

impl Visit for FieldCollector {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.set(field, value.to_string());
    }
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.set(field, value.to_string());
    }
    fn record_u64(&mut self, field: &Field, value: u64) {
        self.set(field, value.to_string());
    }
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.set(field, format!("{value:?}"));
    }
}

impl<S: Subscriber> Layer<S> for DiagLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let meta = event.metadata();
        let mut f = FieldCollector::default();
        event.record(&mut f);
        let ts = Utc::now();
        let level = meta.level().to_string();
        let message = f.message.clone().unwrap_or_default();

        let mut fields = f.others.clone();
        for (k, v) in [
            ("code", &f.code),
            ("phase", &f.phase),
            ("path", &f.path),
            ("detail", &f.detail),
        ] {
            if let Some(v) = v {
                fields.push((k.to_string(), v.clone()));
            }
        }
        if let Some(id) = f.job_id {
            fields.push(("job_id".into(), id.to_string()));
        }

        let error = (*meta.level() <= Level::WARN).then(|| ErrorRecord {
            ts,
            level: level.clone(),
            code: f.code.clone(),
            message: message.clone(),
            target: meta.target().to_string(),
            job_id: f.job_id,
            phase: f.phase.clone(),
            path: f.path.clone(),
            detail: f.detail.clone(),
        });
        let line = LogLine {
            ts,
            level,
            target: meta.target().to_string(),
            message,
            fields,
        };
        self.hub.dispatch(line, error);
    }
}

/// Sink in memoria, utile nei test e come buffer.
#[derive(Default)]
pub struct MemorySink {
    pub errors: Mutex<Vec<ErrorRecord>>,
    pub lines: Mutex<Vec<LogLine>>,
}

impl DiagSink for MemorySink {
    fn on_error(&self, record: &ErrorRecord) {
        self.errors.lock().push(record.clone());
    }
    fn on_log(&self, line: &LogLine) {
        self.lines.lock().push(line.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::layer::SubscriberExt;

    #[test]
    fn warn_with_fields_becomes_error_record() {
        let hub = DiagHub::new(10);
        let sink = Arc::new(MemorySink::default());
        hub.add_sink(sink.clone());
        let sub = tracing_subscriber::registry().with(DiagLayer::new(hub.clone()));
        tracing::subscriber::with_default(sub, || {
            tracing::info!("avvio");
            tracing::warn!(
                code = "CORRUPT_EXIF",
                job_id = 42_i64,
                phase = "metadati",
                path = "D:\\Foto\\IMG_1.JPG",
                "EXIF illeggibile"
            );
        });
        let errors = sink.errors.lock();
        assert_eq!(errors.len(), 1);
        let e = &errors[0];
        assert_eq!(e.level, "WARN");
        assert_eq!(e.code.as_deref(), Some("CORRUPT_EXIF"));
        assert_eq!(e.job_id, Some(42));
        assert_eq!(e.phase.as_deref(), Some("metadati"));
        assert_eq!(e.path.as_deref(), Some("D:\\Foto\\IMG_1.JPG"));
        assert_eq!(e.message, "EXIF illeggibile");
        assert_eq!(sink.lines.lock().len(), 2);
        assert_eq!(hub.tail().len(), 2);
    }

    #[test]
    fn errors_before_sink_are_delivered_later() {
        let hub = DiagHub::new(10);
        let sub = tracing_subscriber::registry().with(DiagLayer::new(hub.clone()));
        tracing::subscriber::with_default(sub, || tracing::error!(code = "IO", "prima del DB"));
        let sink = Arc::new(MemorySink::default());
        hub.add_sink(sink.clone());
        assert_eq!(sink.errors.lock()[0].message, "prima del DB");
    }

    #[test]
    fn tail_is_bounded() {
        let hub = DiagHub::new(3);
        let sub = tracing_subscriber::registry().with(DiagLayer::new(hub.clone()));
        tracing::subscriber::with_default(sub, || {
            for i in 0..5 {
                tracing::info!("riga {i}");
            }
        });
        let tail = hub.tail();
        assert_eq!(tail.len(), 3);
        assert_eq!(tail[0].message, "riga 2");
    }
}
