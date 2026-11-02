//! Misure sulle miniature per i gruppi di foto simili: somiglianza strutturale (SSIM),
//! nitidezza ed esposizione per proporre la foto principale.
//!
//! La nitidezza è la varianza del laplaciano sulla luminanza, come quella dei ritagli dei volti
//! (`people-faces-workspace`): quando i due change saranno entrambi in `main`, le due funzioni
//! vanno riunite in `pc-ai/src/quality.rs`.

use image::{imageops, GrayImage, RgbImage};

/// Lato lungo delle miniature confrontate con SSIM.
pub const SSIM_SIDE: u32 = 128;
/// Differenza massima di proporzioni tra due foto confrontabili (5%).
pub const MAX_ASPECT_DIFF: f32 = 0.05;

const WINDOW: u32 = 8;
const STEP: u32 = 4;
const C1: f64 = (0.01 * 255.0) * (0.01 * 255.0);
const C2: f64 = (0.03 * 255.0) * (0.03 * 255.0);

/// Miniatura in scala di grigi a [`SSIM_SIDE`] di lato lungo, con le proporzioni originali.
#[derive(Debug, Clone)]
pub struct SsimThumb {
    pub image: GrayImage,
    /// Larghezza / altezza della foto.
    pub aspect: f32,
}

impl SsimThumb {
    pub fn from_rgb(img: &RgbImage) -> Self {
        let (w, h) = img.dimensions();
        let aspect = w.max(1) as f32 / h.max(1) as f32;
        let gray = imageops::grayscale(img);
        let long = w.max(h).max(1);
        let image = if long > SSIM_SIDE {
            let f = SSIM_SIDE as f32 / long as f32;
            imageops::resize(
                &gray,
                ((w as f32 * f).round() as u32).max(1),
                ((h as f32 * f).round() as u32).max(1),
                imageops::FilterType::Triangle,
            )
        } else {
            gray
        };
        // Leggera sfocatura: piccoli spostamenti tra gli scatti di una raffica non contano.
        let image = imageops::blur(&image, 1.0);
        Self { image, aspect }
    }
}

/// SSIM medio su finestre 8×8 (passo 4). `None` se le proporzioni differiscono oltre il 5%:
/// due inquadrature con proporzioni diverse non sono la stessa foto ripetuta.
pub fn ssim(a: &SsimThumb, b: &SsimThumb) -> Option<f32> {
    if (a.aspect / b.aspect - 1.0).abs() > MAX_ASPECT_DIFF {
        return None;
    }
    // Stesse dimensioni: si porta la più grande alla più piccola.
    let (w, h) = (
        a.image.width().min(b.image.width()),
        a.image.height().min(b.image.height()),
    );
    let fit = |g: &GrayImage| {
        if g.dimensions() == (w, h) {
            g.clone()
        } else {
            imageops::resize(g, w, h, imageops::FilterType::Triangle)
        }
    };
    let (x, y) = (fit(&a.image), fit(&b.image));
    if w < WINDOW || h < WINDOW {
        return Some(if x == y { 1.0 } else { 0.0 });
    }
    let mut total = 0.0f64;
    let mut n = 0u32;
    let mut oy = 0;
    while oy + WINDOW <= h {
        let mut ox = 0;
        while ox + WINDOW <= w {
            total += window_ssim(&x, &y, ox, oy);
            n += 1;
            ox += STEP;
        }
        oy += STEP;
    }
    Some((total / f64::from(n.max(1))) as f32)
}

fn window_ssim(x: &GrayImage, y: &GrayImage, ox: u32, oy: u32) -> f64 {
    let count = f64::from(WINDOW * WINDOW);
    let (mut sx, mut sy, mut sxx, mut syy, mut sxy) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for j in oy..oy + WINDOW {
        for i in ox..ox + WINDOW {
            let a = f64::from(x.get_pixel(i, j).0[0]);
            let b = f64::from(y.get_pixel(i, j).0[0]);
            sx += a;
            sy += b;
            sxx += a * a;
            syy += b * b;
            sxy += a * b;
        }
    }
    let (mx, my) = (sx / count, sy / count);
    let vx = sxx / count - mx * mx;
    let vy = syy / count - my * my;
    let cov = sxy / count - mx * my;
    ((2.0 * mx * my + C1) * (2.0 * cov + C2)) / ((mx * mx + my * my + C1) * (vx + vy + C2))
}

/// Varianza del laplaciano 3×3 sulla luminanza: più alta = più nitida.
pub fn sharpness(img: &RgbImage) -> f32 {
    let g = imageops::grayscale(img);
    let (w, h) = g.dimensions();
    if w < 3 || h < 3 {
        return 0.0;
    }
    let px = |x: u32, y: u32| f64::from(g.get_pixel(x, y).0[0]);
    let (mut sum, mut sq, mut n) = (0.0f64, 0.0f64, 0.0f64);
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let l = px(x - 1, y) + px(x + 1, y) + px(x, y - 1) + px(x, y + 1) - 4.0 * px(x, y);
            sum += l;
            sq += l * l;
            n += 1.0;
        }
    }
    let mean = sum / n;
    (sq / n - mean * mean) as f32
}

/// Qualità dell'esposizione (0..1): 1 meno la frazione di pixel bruciati o chiusi.
pub fn exposure(img: &RgbImage) -> f32 {
    let g = imageops::grayscale(img);
    let total = g.pixels().len().max(1) as f32;
    let clipped = g.pixels().filter(|p| p.0[0] >= 250 || p.0[0] <= 5).count() as f32;
    1.0 - clipped / total
}

/// Punteggio per la foto principale: nitidezza normalizzata nel gruppo (×0,7) ed
/// esposizione (×0,3). `sharp_max` è la nitidezza massima tra i membri del gruppo.
pub fn cover_score(sharpness: f32, exposure: f32, sharp_max: f32) -> f32 {
    let s = if sharp_max > 0.0 {
        (sharpness / sharp_max).clamp(0.0, 1.0)
    } else {
        0.0
    };
    0.7 * s + 0.3 * exposure.clamp(0.0, 1.0)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Scena sintetica riconoscibile: forme e gradienti con dettaglio fine (`seed` cambia soggetto).
    pub fn scene(w: u32, h: u32, seed: u32) -> RgbImage {
        RgbImage::from_fn(w, h, |x, y| {
            let (fx, fy) = (x as f32 / w as f32, y as f32 / h as f32);
            let v = match seed % 3 {
                0 => {
                    let circle = ((fx - 0.4).powi(2) + (fy - 0.5).powi(2)).sqrt() < 0.25;
                    let stripes = ((x / 24 + y / 36) % 2) as f32 * 40.0;
                    if circle { 220.0 - stripes } else { 60.0 + 80.0 * fx + stripes }
                }
                1 => {
                    let rect = fx > 0.55 && fx < 0.9 && fy > 0.1 && fy < 0.7;
                    let checks = (((x / 44) + (y / 44)) % 2) as f32 * 90.0;
                    if rect { 30.0 + checks } else { 200.0 - 120.0 * fy }
                }
                _ => 128.0 + 100.0 * ((fx * 23.0).sin() * (fy * 17.0).cos()),
            };
            let n = ((x.wrapping_mul(7919) ^ y.wrapping_mul(104_729)) % 5) as f32;
            let c = (v + n).clamp(0.0, 255.0) as u8;
            image::Rgb([c, c.saturating_sub(10), c.saturating_add(10)])
        })
    }

    fn thumb(img: &RgbImage) -> SsimThumb {
        SsimThumb::from_rgb(img)
    }

    #[test]
    fn identical_photos_have_ssim_one() {
        let a = scene(512, 384, 0);
        assert!((ssim(&thumb(&a), &thumb(&a)).unwrap() - 1.0).abs() < 1e-4);
    }

    #[test]
    fn blurred_is_lower_but_shifted_burst_frame_stays_high() {
        let a = scene(512, 384, 0);
        let blurred = imageops::blur(&a, 6.0);
        let s_blur = ssim(&thumb(&a), &thumb(&blurred)).unwrap();
        assert!(s_blur < 0.95, "sfocata: {s_blur}");
        // Raffica: stessa scena spostata di qualche pixel.
        let big = scene(520, 390, 0);
        let shifted = imageops::crop_imm(&big, 4, 3, 512, 384).to_image();
        let base = imageops::crop_imm(&big, 0, 0, 512, 384).to_image();
        let s_shift = ssim(&thumb(&base), &thumb(&shifted)).unwrap();
        assert!(s_shift > 0.8, "spostata: {s_shift}");
    }

    #[test]
    fn different_subject_is_below_half_and_aspect_mismatch_is_skipped() {
        let a = scene(512, 384, 0);
        let b = scene(512, 384, 1);
        let s = ssim(&thumb(&a), &thumb(&b)).unwrap();
        assert!(s < 0.5, "soggetto diverso: {s}");
        let portrait = scene(384, 512, 0);
        assert_eq!(ssim(&thumb(&a), &thumb(&portrait)), None);
    }

    #[test]
    fn sharp_beats_blurred_and_clipping_lowers_exposure() {
        let a = scene(512, 384, 2);
        let blurred = imageops::blur(&a, 3.0);
        assert!(sharpness(&a) > sharpness(&blurred) * 2.0);
        let white = RgbImage::from_pixel(64, 64, image::Rgb([255, 255, 255]));
        assert!(exposure(&white) < 0.01);
        assert!(exposure(&a) > 0.9);
        let max = sharpness(&a);
        assert!(
            cover_score(sharpness(&a), exposure(&a), max)
                > cover_score(sharpness(&blurred), exposure(&blurred), max)
        );
    }
}
