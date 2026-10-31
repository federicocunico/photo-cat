//! Ritagli dei volti per la UI: generati dall'originale alla risoluzione necessaria
//! (la miniatura da 512 px è troppo piccola per i volti lontani) e salvati in cache locale.

use crate::analyze::load_oriented;
use image::{imageops, RgbImage};
use pc_core::{AppError, AppResult, ErrorCode};
use pc_governor::MemoryGovernor;
use pc_store::Store;
use std::path::{Path, PathBuf};

/// Lato del ritaglio in pixel (le card della UI lo mostrano a 112-150 px, anche su schermi HiDPI).
pub const CROP_SIZE: u32 = 192;
/// Margine attorno al box del volto, in proporzione al suo lato.
const MARGIN: f32 = 0.35;

/// Ritaglio quadrato centrato sul box normalizzato `[x, y, w, h]`, ridimensionato a `CROP_SIZE`.
pub fn crop(img: &RgbImage, bbox: [f32; 4]) -> RgbImage {
    let (iw, ih) = (img.width() as f32, img.height() as f32);
    let [x, y, w, h] = bbox;
    let (cx, cy) = ((x + w / 2.0) * iw, (y + h / 2.0) * ih);
    let side = ((w * iw).max(h * ih) * (1.0 + 2.0 * MARGIN)).clamp(1.0, iw.min(ih));
    let left = (cx - side / 2.0).clamp(0.0, iw - side);
    let top = (cy - side / 2.0).clamp(0.0, ih - side);
    let region =
        imageops::crop_imm(img, left as u32, top as u32, side as u32, side as u32).to_image();
    imageops::resize(
        &region,
        CROP_SIZE,
        CROP_SIZE,
        imageops::FilterType::CatmullRom,
    )
}

/// File di cache del ritaglio: il nome dipende dal box, così un volto rianalizzato non
/// riusa un ritaglio vecchio.
pub fn cache_path(dir: &Path, face_id: i64, bbox: &str) -> PathBuf {
    let key = blake3::hash(bbox.as_bytes()).to_hex();
    dir.join(format!("{face_id}-{}.jpg", &key[..12]))
}

/// JPEG del ritaglio del volto `face_id`, dalla cache o generato dall'originale.
pub fn jpeg(
    store: &Store,
    governor: &MemoryGovernor,
    dir: &Path,
    face_id: i64,
) -> AppResult<Vec<u8>> {
    let (media_id, raw): (i64, String) = store.read(|c| {
        Ok(c.query_row(
            "SELECT media_id, bbox FROM face WHERE id = ?1",
            [face_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?)
    })?;
    let path = cache_path(dir, face_id, &raw);
    if let Ok(bytes) = std::fs::read(&path) {
        return Ok(bytes);
    }
    let bbox: [f32; 4] = serde_json::from_str(&raw).map_err(|e| {
        AppError::new(
            ErrorCode::Internal,
            format!("box del volto non valido: {e}"),
        )
    })?;
    // Lato lungo tale che il volto risulti di almeno CROP_SIZE px (entro limiti ragionevoli).
    let face_side = bbox[2].max(bbox[3]).max(0.01);
    let need =
        ((CROP_SIZE as f32 * (1.0 + 2.0 * MARGIN) / face_side).ceil() as u32).clamp(512, 4096);
    let decoded = load_oriented(store, governor, media_id, need)?;
    let out = crop(&decoded.image, bbox);
    drop(decoded);
    let mut bytes = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 88)
        .encode_image(&out)
        .map_err(|e| AppError::new(ErrorCode::Internal, e.to_string()))?;
    std::fs::create_dir_all(dir)?;
    let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
    std::fs::write(&tmp, &bytes)?;
    if std::fs::rename(&tmp, &path).is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgb;

    #[test]
    fn crop_is_centered_on_the_face_and_clamped_at_borders() {
        // Immagine 1000x500 nera con un "volto" bianco 50x50 in (800, 100).
        let mut img = RgbImage::new(1000, 500);
        for y in 100..150 {
            for x in 800..850 {
                img.put_pixel(x, y, Rgb([255, 255, 255]));
            }
        }
        let out = crop(&img, [0.8, 0.2, 0.05, 0.1]);
        assert_eq!(out.dimensions(), (CROP_SIZE, CROP_SIZE));
        let c = CROP_SIZE / 2;
        assert_eq!(out.get_pixel(c, c)[0], 255, "centro sul volto");
        assert_eq!(out.get_pixel(2, 2)[0], 0, "margine attorno al volto");
        // Volto sul bordo: il ritaglio resta dentro l'immagine.
        let edge = crop(&img, [0.97, 0.9, 0.03, 0.1]);
        assert_eq!(edge.dimensions(), (CROP_SIZE, CROP_SIZE));
    }

    #[test]
    fn cache_name_changes_with_the_box() {
        let dir = Path::new("cache");
        assert_ne!(
            cache_path(dir, 7, "[0.1,0.1,0.2,0.2]"),
            cache_path(dir, 7, "[0.3,0.1,0.2,0.2]")
        );
        assert!(cache_path(dir, 7, "[0.1,0.1,0.2,0.2]").starts_with(dir));
    }
}
