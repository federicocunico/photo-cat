//! `init` con una libreria non utilizzabile deve fallire subito: con ort 2.0.0-rc.12 un errore
//! di caricamento non intercettato blocca il thread per sempre. Ogni file di test è un processo
//! a sé, quindi il `OnceLock` globale del runtime non interferisce con gli altri test.

use pc_core::ErrorCode;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

fn init_within_timeout(path: PathBuf) -> pc_core::AppResult<String> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(pc_models::runtime::init(&path));
    });
    rx.recv_timeout(Duration::from_secs(10))
        .expect("init bloccato: il deadlock di ort non è intercettato")
}

#[test]
fn unusable_library_fails_without_blocking() {
    let dir = tempfile::tempdir().unwrap();
    let fake = dir.path().join("onnxruntime.dll");
    std::fs::write(&fake, b"non sono una libreria").unwrap();

    let err = init_within_timeout(fake).unwrap_err();
    assert_eq!(err.code, ErrorCode::ModelUnavailable);
    assert!(!pc_models::runtime::is_initialized());
}
