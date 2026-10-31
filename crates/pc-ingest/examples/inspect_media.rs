//! Legge metadati e decodifica in memoria, senza scrivere né modificare la sorgente.
use std::path::PathBuf;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from(std::env::args().nth(1).expect("percorso sorgente"));
    let ext = path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    let before = std::fs::metadata(&path)?;
    let hash_before = pc_ingest::hash::content_hash(&path)?;
    let exif = pc_ingest::metadata::read_exif(&path)?;
    let tech = pc_ingest::metadata::read_tech(&path, &ext, exif.as_ref())?;
    let decoded =
        pc_ingest::thumb::decode_reduced(&path, &ext, 640, &pc_governor::MemoryGovernor::new(512))?;
    let after = std::fs::metadata(&path)?;
    assert_eq!(before.modified()?, after.modified()?);
    assert_eq!(before.len(), after.len());
    assert_eq!(hash_before, pc_ingest::hash::content_hash(&path)?);
    println!(
        "{}",
        serde_json::json!({"tech":tech,"camera":exif.as_ref().map(|e|&e.info.camera_model),"preview_size":decoded.image.dimensions(),"original_mtime_unchanged":true,"original_hash_unchanged":true})
    );
    Ok(())
}
