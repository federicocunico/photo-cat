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

#[cfg(feature = "heif")]
#[test]
fn native_decoder_handles_heic_and_releases_memory_without_touching_source() {
    let p = fixture();
    // Fuori da Windows si usa la libheif di sistema: se non è installata il test non ha senso.
    if !cfg!(windows) {
        if let Err(e) = pc_ingest::heif::dimensions(&p) {
            assert!(e.detail.contains("libheif non disponibile"), "{}", e.detail);
            eprintln!("libheif di sistema assente, test saltato: {}", e.detail);
            return;
        }
    }
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
    if !cfg!(windows) {
        assert!(
            error.detail.contains("libheif-plugin-libde265"),
            "{}",
            error.detail
        );
        assert!(error.detail.contains(pc_ingest::heif::MIN_SYSTEM_VERSION));
    }
}

#[cfg(feature = "heif")]
#[test]
fn heic_full_decode_keeps_full_size() {
    let p = fixture();
    if !cfg!(windows) && pc_ingest::heif::dimensions(&p).is_err() {
        return;
    }
    let f = pc_ingest::full::decode_full(&p, "heic", &MemoryGovernor::new(64)).unwrap();
    assert_eq!(f.dimensions(), (64, 64));
    assert_eq!(f.orientation, None, "libheif applica già la rotazione");
    // Il profilo, se c'è, è un ICC valido.
    if let Some(icc) = &f.icc {
        assert_eq!(&icc[36..40], b"acsp");
    }
}
