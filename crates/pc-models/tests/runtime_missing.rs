//! `init` con una libreria inesistente fallisce subito con un errore che indica il percorso.
//! File separato da `runtime_init.rs`: il risultato di `init` è memorizzato per processo.

use pc_core::ErrorCode;
use std::sync::mpsc;
use std::time::Duration;

#[test]
fn missing_library_fails_without_blocking() {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(pc_models::runtime::init(std::path::Path::new(
            "percorso/inesistente/onnxruntime.dll",
        )));
    });
    let err = rx
        .recv_timeout(Duration::from_secs(10))
        .expect("init bloccato")
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::ModelUnavailable);
    assert!(err.detail.contains("inesistente"), "{}", err.detail);
}
