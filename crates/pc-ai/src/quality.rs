//! Misure di qualità dell'immagine e dei ritagli dei volti: nitidezza
//! (varianza del laplaciano sul canale di luminanza), frontalità dai 5 punti
//! di riferimento ed esposizione.

use crate::faces::ALIGNED;
use crate::tensor::BBox;
use image::{imageops, RgbImage};

/// Varianza di riferimento del laplaciano su un ritaglio 112×112 per la curva
/// di normalizzazione in `0..=1`: `sharpness = sqrt(var / (var + REF_VAR))`.
pub const SHARPNESS_REF_VAR: f32 = 250.0;

/// Margine interno (in pixel su 112×112) escluso dal calcolo del laplaciano per
/// evitare i bordi neri introdotti dal campionamento fuori immagine.
const BORDER_MARGIN: u32 = 6;

#[inline]
fn luma(rgb: [u8; 3]) -> f32 {
    0.299 * rgb[0] as f32 + 0.587 * rgb[1] as f32 + 0.114 * rgb[2] as f32
}

/// Varianza del laplaciano 3×3 (`[0,1,0; 1,-4,1; 0,1,0]`) sul canale di
/// luminanza (`0..255`), escludendo una cornice di `margin` pixel.
pub fn laplacian_variance_cropped(img: &RgbImage, margin: u32) -> f32 {
    let (w, h) = img.dimensions();
    let m = margin.max(1);
    if w <= 2 * m || h <= 2 * m {
        return 0.0;
    }
    let mut sum = 0.0f64;
    let mut sum_sq = 0.0f64;
    let mut count = 0u64;
    for y in m..(h - m) {
        for x in m..(w - m) {
            let c = luma(img.get_pixel(x, y).0);
            let lap = luma(img.get_pixel(x - 1, y).0)
                + luma(img.get_pixel(x + 1, y).0)
                + luma(img.get_pixel(x, y - 1).0)
                + luma(img.get_pixel(x, y + 1).0)
                - 4.0 * c;
            let v = lap as f64;
            sum += v;
            sum_sq += v * v;
            count += 1;
        }
    }
    if count == 0 {
        return 0.0;
    }
    let mean = sum / count as f64;
    let var = (sum_sq / count as f64 - mean * mean).max(0.0);
    var as f32
}

/// Varianza del laplaciano sull'intera immagine (usata anche per confrontare
/// miniature in `similar-photo-stacks`).
pub fn laplacian_variance(img: &RgbImage) -> f32 {
    laplacian_variance_cropped(img, 1)
}

/// Nitidezza normalizzata in `0..=1` su un ritaglio del volto. Se il ritaglio
/// non è già `112×112`, viene riportato a `112×112` prima della misura così che
/// la scala del laplaciano sia invariante rispetto alla risoluzione di partenza.
pub fn sharpness(crop: &RgbImage) -> f32 {
    let std_crop;
    let target = if crop.dimensions() == (ALIGNED, ALIGNED) {
        crop
    } else {
        std_crop = imageops::resize(crop, ALIGNED, ALIGNED, imageops::FilterType::Triangle);
        &std_crop
    };
    let var = laplacian_variance_cropped(target, BORDER_MARGIN);
    if !var.is_finite() || var <= 0.0 {
        return 0.0;
    }
    (var / (var + SHARPNESS_REF_VAR)).sqrt().clamp(0.0, 1.0)
}

/// Nitidezza calcolata da un ritaglio `face_crop` (che include un margine del
/// 35% attorno al riquadro del volto): estrae la regione centrale del volto
/// prima di normalizzarla a `112×112`.
pub fn sharpness_from_display_crop(crop: &RgbImage) -> f32 {
    let (w, h) = crop.dimensions();
    // In `face_crop::crop` il lato totale è `1 + 2 * 0.35 = 1.70` volte il volto.
    let inset_x = ((w as f32) * (0.35 / 1.70)).round() as u32;
    let inset_y = ((h as f32) * (0.35 / 1.70)).round() as u32;
    if w > 2 * inset_x + 16 && h > 2 * inset_y + 16 {
        let inner =
            imageops::crop_imm(crop, inset_x, inset_y, w - 2 * inset_x, h - 2 * inset_y).to_image();
        sharpness(&inner)
    } else {
        sharpness(crop)
    }
}

/// Frontalità del volto in `0..=1` dai 5 punti di riferimento
/// (`[occhio_dx, occhio_sx, naso, bocca_dx, bocca_sx]`) e dal riquadro:
/// combina la simmetria orizzontale del naso rispetto agli occhi e il rapporto
/// tra distanza degli occhi e larghezza del riquadro.
pub fn frontality(bbox: &BBox, landmarks: &[(f32, f32); 5]) -> f32 {
    let (l0, l1, nose) = (landmarks[0], landmarks[1], landmarks[2]);
    let (dx, dy) = (l1.0 - l0.0, l1.1 - l0.1);
    let eye_dist = (dx * dx + dy * dy).sqrt();
    if !eye_dist.is_finite() || eye_dist <= 1e-3 || bbox.w <= 1e-3 {
        return 0.0;
    }
    let eye_ratio = (eye_dist / (bbox.w * 0.32)).clamp(0.0, 1.0);
    // Proiezione del naso sull'asse che unisce i due occhi (0.5 = centrato).
    let t = ((nose.0 - l0.0) * dx + (nose.1 - l0.1) * dy) / (eye_dist * eye_dist);
    let asym = ((t - 0.5).abs() * 2.0).clamp(0.0, 1.5);
    let sym = (1.0 - 0.75 * asym).clamp(0.0, 1.0);
    (sym * eye_ratio).clamp(0.0, 1.0)
}

/// Punteggio di esposizione in `0..=1` (penalizza pixel bruciati o chiusi).
pub fn exposure(img: &RgbImage) -> f32 {
    let total = (img.width() as u64) * (img.height() as u64);
    if total == 0 {
        return 0.0;
    }
    let clipped = img
        .pixels()
        .filter(|p| {
            let y = luma(p.0);
            !(8.0..=247.0).contains(&y)
        })
        .count() as f32;
    (1.0 - (clipped / total as f32) * 2.0).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgb;
    use pc_store::people::{combined_quality, discard_reason, DiscardReason, FaceMatch};

    /// Ritaglio sintetico 112×112 con dettagli netti (scacchiera fine e tratti).
    fn synthetic_sharp_crop() -> RgbImage {
        RgbImage::from_fn(ALIGNED, ALIGNED, |x, y| {
            let checker = ((x / 4 + y / 4) % 2) as u8 * 160 + 40;
            let ring = {
                let (dx, dy) = (x as i32 - 56, y as i32 - 56);
                if (dx * dx + dy * dy - 900).abs() < 80 {
                    240u8
                } else {
                    checker
                }
            };
            Rgb([ring, ring, ring])
        })
    }

    /// Sfocatura con kernel gaussiano separabile (sigma ~ 3 px).
    fn gaussian_blur_crop(src: &RgbImage) -> RgbImage {
        imageops::blur(src, 3.2)
    }

    #[test]
    fn sharp_blurred_small_and_profile_synthetic_faces() {
        let sharp_img = synthetic_sharp_crop();
        let blurred_img = gaussian_blur_crop(&sharp_img);
        let s_sharp = sharpness(&sharp_img);
        let s_blur = sharpness(&blurred_img);
        assert!(
            s_sharp > 0.65,
            "il ritaglio nitido deve avere nitidezza alta: {s_sharp}"
        );
        assert!(
            s_blur < pc_core::AiThresholds::DEFAULT_FACE_MIN_SHARPNESS,
            "il ritaglio sfocato deve stare sotto la soglia predefinita: {s_blur}"
        );

        let bbox = BBox {
            x: 10.0,
            y: 10.0,
            w: 100.0,
            h: 120.0,
        };
        let frontal_lm = [
            (38.0, 52.0),
            (74.0, 52.0),
            (56.0, 72.0),
            (42.0, 92.0),
            (70.0, 92.0),
        ];
        let profile_lm = [
            (72.0, 52.0),
            (82.0, 52.0),
            (92.0, 72.0),
            (76.0, 92.0),
            (84.0, 92.0),
        ];
        let f_frontal = frontality(&bbox, &frontal_lm);
        let f_profile = frontality(&bbox, &profile_lm);
        assert!(f_frontal > 0.85, "volto frontale: {f_frontal}");
        assert!(f_profile < 0.22, "volto di profilo estremo: {f_profile}");

        let q_good = combined_quality(0.95, Some(s_sharp), Some(f_frontal), Some(120));
        let q_blur = combined_quality(0.95, Some(s_blur), Some(f_frontal), Some(120));
        let q_small = combined_quality(0.95, Some(s_sharp), Some(f_frontal), Some(24));
        let q_prof = combined_quality(0.95, Some(s_sharp), Some(f_profile), Some(120));
        assert!(q_good > q_blur && q_good > q_small && q_good > q_prof);

        let m = FaceMatch {
            similarity: 0.363,
            min_quality: 0.8,
            min_sharpness: 0.35,
            min_size_px: 40,
            floor: Some(0.6),
        };
        assert_eq!(
            discard_reason(
                0.95,
                Some(s_sharp),
                Some(f_frontal),
                Some(120),
                false,
                false,
                &m
            ),
            None
        );
        assert_eq!(
            discard_reason(
                0.95,
                Some(s_blur),
                Some(f_frontal),
                Some(120),
                false,
                false,
                &m
            ),
            Some(DiscardReason::Blurred)
        );
        assert_eq!(
            discard_reason(
                0.95,
                Some(s_sharp),
                Some(f_frontal),
                Some(28),
                false,
                false,
                &m
            ),
            Some(DiscardReason::TooSmall)
        );
        assert_eq!(
            discard_reason(
                0.95,
                Some(s_sharp),
                Some(f_profile),
                Some(120),
                false,
                false,
                &m
            ),
            Some(DiscardReason::NonFrontal)
        );
        assert_eq!(
            discard_reason(
                0.70,
                Some(s_sharp),
                Some(f_frontal),
                Some(120),
                false,
                false,
                &m
            ),
            Some(DiscardReason::LowConfidence)
        );
    }
}
