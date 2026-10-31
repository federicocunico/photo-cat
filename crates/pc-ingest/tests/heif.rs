use pc_core::ErrorCode;
use pc_governor::MemoryGovernor;
use std::path::PathBuf;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/selftest/codec_pattern.heic")
}

#[cfg(not(feature = "heif"))]
#[test]
fn without_decoder_heic_keeps_metadata_and_reports_unsupported_decode() {
    let p = fixture();
    let tech = pc_ingest::metadata::read_tech(&p, "heic", None).unwrap();
    assert_eq!((tech.width, tech.height), (Some(64), Some(64)));
    let error = pc_ingest::thumb::decode_reduced(&p, "heic", 32, &MemoryGovernor::new(64))
        .err()
        .unwrap();
    assert_eq!(error.code, ErrorCode::UnsupportedFormat);
}

#[cfg(all(feature = "heif", windows))]
#[test]
fn native_decoder_handles_heic_and_releases_memory_without_touching_source() {
    let p = fixture();
    let original = std::fs::read(&p).unwrap();
    let mtime = std::fs::metadata(&p).unwrap().modified().unwrap();
    let gov = MemoryGovernor::new(64);
    let decoded = pc_ingest::thumb::decode_reduced(&p, "heic", 32, &gov).unwrap();
    assert_eq!(decoded.source_size, (64, 64));
    assert_eq!(decoded.image.dimensions(), (32, 32));
    assert_eq!(gov.snapshot().in_use_mb, decoded.permit.mb());
    drop(decoded);
    assert_eq!(gov.snapshot().in_use_mb, 0);
    assert_eq!(std::fs::read(&p).unwrap(), original);
    assert_eq!(std::fs::metadata(&p).unwrap().modified().unwrap(), mtime);
    let tech = pc_ingest::metadata::read_tech(&p, "heic", None).unwrap();
    assert_eq!((tech.width, tech.height), (Some(64), Some(64)));
    let missing = pc_ingest::heif::decode_with_library(
        &p,
        std::path::Path::new("missing-heif.dll"),
        32,
        &gov,
    )
    .err()
    .unwrap();
    assert_eq!(missing.code, ErrorCode::UnsupportedFormat);
    let small = pc_ingest::thumb::decode_reduced(&p, "heic", 32, &MemoryGovernor::new(0))
        .err()
        .unwrap();
    assert_eq!(small.code, ErrorCode::InsufficientMemory);
}

#[cfg(feature = "heif")]
#[test]
fn missing_decoder_library_reports_unsupported_format() {
    let error = pc_ingest::heif::decode_with_library(
        &fixture(),
        std::path::Path::new("percorso/inesistente/libheif.so.1"),
        32,
        &MemoryGovernor::new(64),
    )
    .err()
    .unwrap();
    assert_eq!(error.code, ErrorCode::UnsupportedFormat);
}
