//! Codifica JPEG verificata: qualità scelta, crominanza 4:4:4, profilo ICC e
//! segmenti APP (EXIF/XMP); confronto SSIM/PSNR con l'originale e ricodifica a
//! qualità crescente fino a 100 se le soglie non sono raggiunte.

use image::RgbImage;
use jpeg_encoder::{ColorType, Encoder, SamplingFactor};
use pc_core::{AppError, AppResult, ErrorCode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct JpegSettings {
    /// Qualità iniziale (1–100).
    pub quality: u8,
    /// Crominanza a piena risoluzione (4:4:4); altrimenti 4:2:0.
    pub chroma_444: bool,
    /// Confronto con l'originale e ricodifica se sotto soglia.
    pub verify: bool,
    pub min_ssim: f64,
    pub min_psnr: f64,
}

impl Default for JpegSettings {
    fn default() -> Self {
        Self {
            quality: 97,
            chroma_444: true,
            verify: true,
            min_ssim: 0.985,
            min_psnr: 42.0,
        }
    }
}

/// Esito della codifica di una foto.
#[derive(Debug, Clone, PartialEq)]
pub struct Encoded {
    pub bytes: Vec<u8>,
    pub quality: u8,
    pub ssim: Option<f64>,
    pub psnr: Option<f64>,
    /// Soglie non raggiunte nemmeno a qualità 100.
    pub below_threshold: bool,
}

fn err(e: impl std::fmt::Display) -> AppError {
    AppError::new(ErrorCode::Internal, format!("codifica JPEG: {e}"))
}

/// Un passaggio di codifica.
pub fn encode_jpeg(
    rgb: &RgbImage,
    quality: u8,
    chroma_444: bool,
    icc: Option<&[u8]>,
    app_segments: &[(u8, Vec<u8>)],
) -> AppResult<Vec<u8>> {
    let (w, h) = rgb.dimensions();
    if w > u32::from(u16::MAX) || h > u32::from(u16::MAX) {
        return Err(AppError::new(
            ErrorCode::UnsupportedFormat,
            format!("immagine {w}×{h} oltre il limite JPEG di 65535 pixel per lato"),
        ));
    }
    let mut out = Vec::with_capacity((w as usize * h as usize) / 2);
    let mut enc = Encoder::new(&mut out, quality.clamp(1, 100));
    enc.set_sampling_factor(if chroma_444 {
        SamplingFactor::F_1_1
    } else {
        SamplingFactor::F_2_2
    });
    enc.set_optimized_huffman_tables(true);
    for (nr, data) in app_segments {
        enc.add_app_segment(*nr, data).map_err(err)?;
    }
    if let Some(p) = icc {
        enc.add_icc_profile(p).map_err(err)?;
    }
    enc.encode(rgb.as_raw(), w as u16, h as u16, ColorType::Rgb)
        .map_err(err)?;
    Ok(out)
}

fn luma(p: &image::Rgb<u8>) -> f64 {
    0.299 * f64::from(p[0]) + 0.587 * f64::from(p[1]) + 0.114 * f64::from(p[2])
}

/// SSIM medio sulla luminanza, su finestre 8×8 non sovrapposte.
pub fn ssim(a: &RgbImage, b: &RgbImage) -> f64 {
    assert_eq!(a.dimensions(), b.dimensions(), "dimensioni diverse");
    let (w, h) = a.dimensions();
    const C1: f64 = (0.01 * 255.0) * (0.01 * 255.0);
    const C2: f64 = (0.03 * 255.0) * (0.03 * 255.0);
    let (mut total, mut windows) = (0.0, 0u64);
    let mut y = 0;
    while y < h {
        let wh = 8.min(h - y);
        let mut x = 0;
        while x < w {
            let ww = 8.min(w - x);
            let n = f64::from(ww * wh);
            let (mut sa, mut sb, mut saa, mut sbb, mut sab) = (0.0, 0.0, 0.0, 0.0, 0.0);
            for yy in y..y + wh {
                for xx in x..x + ww {
                    let (la, lb) = (luma(a.get_pixel(xx, yy)), luma(b.get_pixel(xx, yy)));
                    sa += la;
                    sb += lb;
                    saa += la * la;
                    sbb += lb * lb;
                    sab += la * lb;
                }
            }
            let (ma, mb) = (sa / n, sb / n);
            let va = (saa / n - ma * ma).max(0.0);
            let vb = (sbb / n - mb * mb).max(0.0);
            let cov = sab / n - ma * mb;
            total += ((2.0 * ma * mb + C1) * (2.0 * cov + C2))
                / ((ma * ma + mb * mb + C1) * (va + vb + C2));
            windows += 1;
            x += 8;
        }
        y += 8;
    }
    if windows == 0 {
        1.0
    } else {
        total / windows as f64
    }
}

/// PSNR su RGB in dB (`f64::INFINITY` per immagini identiche).
pub fn psnr(a: &RgbImage, b: &RgbImage) -> f64 {
    assert_eq!(a.dimensions(), b.dimensions(), "dimensioni diverse");
    let sum: u64 = a
        .as_raw()
        .iter()
        .zip(b.as_raw())
        .map(|(x, y)| {
            let d = i64::from(*x) - i64::from(*y);
            (d * d) as u64
        })
        .sum();
    if sum == 0 {
        return f64::INFINITY;
    }
    let mse = sum as f64 / a.as_raw().len() as f64;
    10.0 * (255.0 * 255.0 / mse).log10()
}

fn decode(bytes: &[u8]) -> AppResult<RgbImage> {
    Ok(
        image::load_from_memory_with_format(bytes, image::ImageFormat::Jpeg)
            .map_err(|e| AppError::new(ErrorCode::DecodeFailed, e.to_string()))?
            .to_rgb8(),
    )
}

/// Codifica con verifica: se SSIM o PSNR sono sotto soglia ricodifica a
/// qualità +1 fino a 100; a 100 l'esito resta, segnalato come sotto soglia.
pub fn encode_verified(
    rgb: &RgbImage,
    icc: Option<&[u8]>,
    app_segments: &[(u8, Vec<u8>)],
    s: &JpegSettings,
) -> AppResult<Encoded> {
    let mut quality = s.quality.clamp(1, 100);
    loop {
        let bytes = encode_jpeg(rgb, quality, s.chroma_444, icc, app_segments)?;
        if !s.verify {
            return Ok(Encoded {
                bytes,
                quality,
                ssim: None,
                psnr: None,
                below_threshold: false,
            });
        }
        let back = decode(&bytes)?;
        let (ss, ps) = (ssim(rgb, &back), psnr(rgb, &back));
        let ok = ss >= s.min_ssim && ps >= s.min_psnr;
        if ok || quality >= 100 {
            return Ok(Encoded {
                bytes,
                quality,
                ssim: Some(ss),
                psnr: Some(ps),
                below_threshold: !ok,
            });
        }
        quality += 1;
    }
}

/// Stima della dimensione del JPEG per la verifica preliminare dello spazio:
/// byte per pixel in funzione della qualità (calibrato sulle immagini di prova
/// a 4:4:4 con margine; foto reali tipicamente sotto questi valori).
pub fn estimated_bytes(width: u32, height: u32, quality: u8, chroma_444: bool) -> u64 {
    let q = f64::from(quality.clamp(1, 100));
    let mut bpp = match q as u8 {
        0..=80 => 0.45,
        81..=90 => 0.75,
        91..=95 => 1.1,
        96..=98 => 1.6,
        _ => 2.4,
    };
    if !chroma_444 {
        bpp *= 0.7;
    }
    (f64::from(width) * f64::from(height) * bpp) as u64 + 64 * 1024
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Immagine con dettagli (gradienti e bordi), più realistica di un colore piatto.
    fn sample(w: u32, h: u32) -> RgbImage {
        RgbImage::from_fn(w, h, |x, y| {
            let edge = if (x / 7 + y / 5) % 2 == 0 { 40 } else { 0 };
            image::Rgb([
                (x * 255 / w) as u8,
                (y * 255 / h) as u8,
                ((x + y) % 200) as u8 + edge,
            ])
        })
    }

    #[test]
    fn identical_images_have_ssim_one_and_infinite_psnr() {
        let a = sample(37, 29);
        assert!((ssim(&a, &a) - 1.0).abs() < 1e-12);
        assert_eq!(psnr(&a, &a), f64::INFINITY);
    }

    #[test]
    fn known_noise_gives_the_expected_psnr() {
        let a = sample(64, 48);
        // +4 su ogni canale (senza saturazione): MSE = 16.
        let a = RgbImage::from_fn(64, 48, |x, y| {
            let p = a.get_pixel(x, y).0;
            image::Rgb(p.map(|c| c.min(200)))
        });
        let b = RgbImage::from_fn(64, 48, |x, y| {
            image::Rgb(a.get_pixel(x, y).0.map(|c| c + 4))
        });
        let expected = 10.0 * (255.0f64 * 255.0 / 16.0).log10();
        assert!((psnr(&a, &b) - expected).abs() < 1e-9, "{}", psnr(&a, &b));
        assert!((expected - 36.0896).abs() < 1e-3);
        // Uno spostamento uniforme lascia la struttura: SSIM resta alto ma < 1.
        let s = ssim(&a, &b);
        assert!(s > 0.99 && s < 1.0, "{s}");
    }

    #[test]
    fn default_quality_is_444_with_icc_and_passes_on_detailed_image() {
        let img = sample(96, 64);
        let icc = vec![7u8; 300];
        let out = encode_verified(&img, Some(&icc), &[], &JpegSettings::default()).unwrap();
        assert!(out.quality >= 97);
        assert!(!out.below_threshold);
        assert!(out.ssim.unwrap() >= 0.985 && out.psnr.unwrap() >= 42.0);
        let decoded = image::load_from_memory(&out.bytes).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (96, 64));
        // Il profilo ICC è incorporato (APP2 "ICC_PROFILE").
        let mut dec =
            image::codecs::jpeg::JpegDecoder::new(std::io::Cursor::new(&out.bytes)).unwrap();
        use image::ImageDecoder;
        assert_eq!(dec.icc_profile().unwrap(), Some(icc));
        // 4:4:4: un solo fattore di campionamento 1×1 per tutti i componenti (SOF).
        let sof = out
            .bytes
            .windows(2)
            .position(|w| w == [0xFF, 0xC0] || w == [0xFF, 0xC2])
            .unwrap();
        let comps = &out.bytes[sof + 10..sof + 19];
        assert_eq!([comps[1], comps[4], comps[7]], [0x11, 0x11, 0x11]);
    }

    #[test]
    fn unreachable_threshold_stops_at_100_and_is_reported() {
        let img = sample(48, 32);
        let s = JpegSettings {
            quality: 95,
            min_ssim: 1.01,
            ..Default::default()
        };
        let out = encode_verified(&img, None, &[], &s).unwrap();
        assert_eq!(out.quality, 100);
        assert!(out.below_threshold);
        assert!(out.ssim.is_some());
        // Senza verifica: una sola codifica alla qualità scelta.
        let out = encode_verified(&img, None, &[], &JpegSettings { verify: false, ..s }).unwrap();
        assert_eq!((out.quality, out.ssim), (95, None));
    }

    #[test]
    fn size_estimate_covers_real_photos() {
        let dir =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/selftest");
        for name in [
            "group_3_faces.jpg",
            "group_4_people.jpg",
            "landscape_mountain.jpg",
        ] {
            let img = image::open(dir.join(name)).unwrap().to_rgb8();
            for q in [90u8, 97, 100] {
                let bytes = encode_jpeg(&img, q, true, None, &[]).unwrap();
                let est = estimated_bytes(img.width(), img.height(), q, true);
                let real = bytes.len() as u64;
                eprintln!(
                    "{name} q{q}: reale {real} stima {est} ({:.2} B/px)",
                    real as f64 / (img.width() * img.height()) as f64
                );
                assert!(real <= est, "{name} q{q}: {real} > {est}");
            }
        }
    }

    #[test]
    fn size_estimate_grows_with_quality() {
        assert!(estimated_bytes(6000, 4000, 97, true) > estimated_bytes(6000, 4000, 85, true));
        assert!(estimated_bytes(6000, 4000, 97, false) < estimated_bytes(6000, 4000, 97, true));
    }
}
