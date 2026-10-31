//! `PHOTOCAT_HEIF_LIBRARY` sostituisce la libheif predefinita. File separato: la variabile
//! d'ambiente vale per tutto il processo e non deve influenzare gli altri test HEIF.
#![cfg(feature = "heif")]

use pc_core::ErrorCode;
use pc_governor::MemoryGovernor;
use std::path::PathBuf;

#[test]
fn library_from_environment_is_used_and_missing_one_suggests_installing() {
    std::env::set_var("PHOTOCAT_HEIF_LIBRARY", "percorso/inesistente/libheif.so.1");
    assert_eq!(
        pc_ingest::heif::library_path(),
        PathBuf::from("percorso/inesistente/libheif.so.1")
    );
    let heic =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/selftest/codec_pattern.heic");
    let error = pc_ingest::thumb::decode_reduced(&heic, "heic", 32, &MemoryGovernor::new(64))
        .err()
        .unwrap();
    assert_eq!(error.code, ErrorCode::UnsupportedFormat);
    assert!(error.detail.contains("inesistente"), "{}", error.detail);
    if !cfg!(windows) {
        assert!(error.detail.contains("libheif1"), "{}", error.detail);
    }
}
