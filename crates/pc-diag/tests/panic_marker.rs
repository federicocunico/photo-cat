//! Test in un binario separato: il panic hook è globale al processo.

use pc_core::config::LoggingConfig;
use std::sync::Arc;

#[test]
fn panic_is_logged_and_marker_survives_restart() {
    let tmp = tempfile::tempdir().unwrap();
    let (sub, diag) = pc_diag::build(tmp.path(), &LoggingConfig::default());
    let sink = Arc::new(pc_diag::MemorySink::default());
    diag.hub().add_sink(sink.clone());

    tracing::subscriber::with_default(sub, || {
        pc_diag::install_panic_hook(tmp.path());
        let result = std::panic::catch_unwind(|| panic!("boom di prova"));
        assert!(result.is_err());
    });
    let _ = std::panic::take_hook();

    let errors = sink.errors.lock();
    assert!(
        errors.iter().any(|e| e.message.contains("boom di prova")),
        "{errors:?}"
    );
    drop(errors);

    // "Riavvio": il marker viene letto una sola volta.
    let crash = pc_diag::take_previous_crash(tmp.path()).expect("marker presente");
    assert_eq!(crash.message, "boom di prova");
    assert!(crash.location.unwrap().contains("panic_marker.rs"));
    assert!(pc_diag::take_previous_crash(tmp.path()).is_none());
}
