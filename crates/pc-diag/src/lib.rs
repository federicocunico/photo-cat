//! Diagnostica di photo-cat: `tracing` è l'unica fonte per log su file,
//! registro errori e coda log della UI.

pub mod bundle;
pub mod crash;
pub mod layer;
pub mod rolling;

pub use crash::{install_panic_hook, take_previous_crash, CrashInfo};
pub use layer::{DiagHub, DiagLayer, DiagSink, ErrorRecord, LogLine, MemorySink};

use pc_core::config::{LogLevel, LoggingConfig};
use pc_core::{AppError, AppResult};
use rolling::{RollingFileWriter, SharedRollingWriter};
use std::path::Path;
use std::sync::Arc;
use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::filter::Targets;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::reload;
use tracing_subscriber::{Layer, Registry};

pub const LOG_PREFIX: &str = "photo-cat";
const TAIL_LINES: usize = 500;

type LevelHandle = reload::Handle<Targets, Registry>;

/// Librerie molto verbose: il loro livello non scende mai sotto WARN.
const NOISY: &[&str] = &[
    "ort",
    "ort_sys",
    "tokenizers",
    "hyper",
    "reqwest",
    "rustls",
    "notify",
];

/// Filtro per target: il livello scelto per photo-cat, WARN per le librerie verbose.
/// Il log interno di ONNX Runtime arriva solo da ERROR: i suoi WARN (es. "Initializer ...
/// appears in graph inputs" a ogni caricamento di SFace) sono innocui e riempirebbero il registro errori.
pub fn targets(level: LogLevel) -> Targets {
    let base = to_filter(level);
    let noisy = base.min(LevelFilter::WARN);
    NOISY
        .iter()
        .fold(Targets::new().with_default(base), |t, n| {
            t.with_target(*n, noisy)
        })
        .with_target("ort::logging", base.min(LevelFilter::ERROR))
}

/// Mantiene vivo il writer non bloccante e permette di cambiare livello a runtime.
pub struct Diagnostics {
    hub: DiagHub,
    level: LevelHandle,
    _guard: Option<WorkerGuard>,
}

impl Diagnostics {
    pub fn hub(&self) -> &DiagHub {
        &self.hub
    }

    /// Cambia il livello di log con effetto immediato.
    pub fn set_level(&self, level: LogLevel) -> AppResult<()> {
        self.level
            .reload(targets(level))
            .map_err(|e| AppError::internal(format!("cambio livello log fallito: {e}")))
    }

    pub fn level(&self) -> Option<LevelFilter> {
        self.level
            .clone_current()
            .map(|t| t.default_level().unwrap_or(LevelFilter::OFF))
    }
}

pub fn to_filter(level: LogLevel) -> LevelFilter {
    match level {
        LogLevel::Error => LevelFilter::ERROR,
        LogLevel::Warn => LevelFilter::WARN,
        LogLevel::Info => LevelFilter::INFO,
        LogLevel::Debug => LevelFilter::DEBUG,
        LogLevel::Trace => LevelFilter::TRACE,
    }
}

/// Costruisce il subscriber senza installarlo (usato da [`init`] e dai test).
pub fn build(
    logs_dir: &Path,
    cfg: &LoggingConfig,
) -> (impl tracing::Subscriber + Send + Sync, Diagnostics) {
    let writer = Arc::new(RollingFileWriter::new(
        logs_dir,
        LOG_PREFIX,
        cfg.max_file_mb * 1024 * 1024,
        cfg.max_files,
    ));
    let (non_blocking, guard) = tracing_appender::non_blocking(SharedRollingWriter(writer));
    let (filter, level) = reload::Layer::new(targets(cfg.level));
    let hub = DiagHub::new(TAIL_LINES);

    let file_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_current_span(true)
        .with_thread_names(true)
        .with_writer(non_blocking);

    let subscriber = tracing_subscriber::registry()
        .with(filter)
        .with(file_layer.with_filter(LevelFilter::TRACE))
        .with(DiagLayer::new(hub.clone()));
    (
        subscriber,
        Diagnostics {
            hub,
            level,
            _guard: Some(guard),
        },
    )
}

/// Installa il subscriber globale e il panic hook.
pub fn init(logs_dir: &Path, cfg: &LoggingConfig) -> AppResult<Diagnostics> {
    std::fs::create_dir_all(logs_dir)?;
    let (subscriber, diag) = build(logs_dir, cfg);
    tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| AppError::internal(format!("logger già inizializzato: {e}")))?;
    install_panic_hook(logs_dir);
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        "diagnostica inizializzata"
    );
    Ok(diag)
}

/// File di log presenti, dal più vecchio al più recente.
pub fn log_files(logs_dir: &Path) -> AppResult<Vec<std::path::PathBuf>> {
    Ok(rolling::list_log_files(logs_dir, LOG_PREFIX)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_level_change_enables_debug() {
        let tmp = tempfile::tempdir().unwrap();
        let (sub, diag) = build(tmp.path(), &LoggingConfig::default());
        let sink = Arc::new(MemorySink::default());
        diag.hub().add_sink(sink.clone());
        tracing::subscriber::with_default(sub, || {
            tracing::debug!("nascosto");
            diag.set_level(LogLevel::Debug).unwrap();
            tracing::debug!("visibile");
        });
        let lines: Vec<_> = sink
            .lines
            .lock()
            .iter()
            .map(|l| l.message.clone())
            .collect();
        assert_eq!(lines, vec!["visibile".to_string()]);
    }

    #[test]
    fn noisy_libraries_are_capped_at_warn() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = LoggingConfig {
            level: LogLevel::Debug,
            ..Default::default()
        };
        let (sub, diag) = build(tmp.path(), &cfg);
        let sink = Arc::new(MemorySink::default());
        diag.hub().add_sink(sink.clone());
        tracing::subscriber::with_default(sub, || {
            tracing::info!(target: "ort::session", "rumore");
            tracing::warn!(target: "ort::session", "avviso utile");
            tracing::warn!(target: "ort::logging", "Initializer x appears in graph inputs");
            tracing::error!(target: "ort::logging", "errore runtime");
            tracing::debug!(target: "pc_ingest::scan", "nostro debug");
        });
        let msgs: Vec<_> = sink
            .lines
            .lock()
            .iter()
            .map(|l| l.message.clone())
            .collect();
        assert_eq!(
            msgs,
            vec![
                "avviso utile".to_string(),
                "errore runtime".to_string(),
                "nostro debug".to_string()
            ]
        );
    }

    #[test]
    fn writes_json_lines_to_file() {
        let tmp = tempfile::tempdir().unwrap();
        {
            let (sub, diag) = build(tmp.path(), &LoggingConfig::default());
            tracing::subscriber::with_default(sub, || {
                tracing::info!(phase = "scansione", "fase completata");
            });
            drop(diag); // flush del writer non bloccante
        }
        let files = log_files(tmp.path()).unwrap();
        assert_eq!(files.len(), 1);
        let text = std::fs::read_to_string(&files[0]).unwrap();
        let v: serde_json::Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
        assert_eq!(v["fields"]["message"], "fase completata");
        assert_eq!(v["fields"]["phase"], "scansione");
    }
}
