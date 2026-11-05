//! Hash percettivo a 64 bit (pHash su DCT) e ricerca veloce per distanza di Hamming.

use image::{imageops::FilterType, DynamicImage, GrayImage};
use std::sync::OnceLock;

const N: usize = 32;
const K: usize = 8;
/// Sotto questa ampiezza (DCT non normalizzata, pixel 0-255) gli AC sono rumore numerico.
const FLAT_EPS: f32 = 1.0;

/// Matrice del DCT-II 32x32, calcolata una volta sola.
fn cos_matrix() -> &'static [[f32; N]; N] {
    static M: OnceLock<[[f32; N]; N]> = OnceLock::new();
    M.get_or_init(|| {
        let mut m = [[0f32; N]; N];
        for (k, row) in m.iter_mut().enumerate() {
            for (n, v) in row.iter_mut().enumerate() {
                *v = (std::f64::consts::PI / N as f64 * (n as f64 + 0.5) * k as f64).cos() as f32;
            }
        }
        m
    })
}

/// pHash di un'immagine in toni di grigio: 32x32, DCT 2D, blocco 8x8 in alto a sinistra.
/// Il bit i (riga-maggiore sul blocco 8x8) è 1 se il coefficiente supera la mediana
/// dei 63 coefficienti AC. Il bit 0 (DC) è sempre 0: la luminosità media non conta.
pub fn phash_gray(img: &GrayImage) -> u64 {
    let small = image::imageops::resize(img, N as u32, N as u32, FilterType::Triangle);
    let m = cos_matrix();
    let mut px = [[0f32; N]; N];
    for (y, row) in px.iter_mut().enumerate() {
        for (x, v) in row.iter_mut().enumerate() {
            *v = small.get_pixel(x as u32, y as u32).0[0] as f32;
        }
    }
    // DCT separabile: righe (solo i primi K coefficienti), poi colonne.
    let mut tmp = [[0f32; K]; N];
    for y in 0..N {
        for (u, t) in tmp[y].iter_mut().enumerate() {
            *t = (0..N).map(|x| px[y][x] * m[u][x]).sum();
        }
    }
    let mut c = [0f32; K * K];
    for v in 0..K {
        for u in 0..K {
            c[v * K + u] = (0..N).map(|y| tmp[y][u] * m[v][y]).sum();
        }
    }
    let mut ac: Vec<f32> = c[1..].to_vec();
    // Immagine uniforme: gli AC sono solo rumore numerico. Impronta fissa (0), così due
    // immagini piatte risultano candidate e decide la SSIM.
    if ac.iter().all(|v| v.abs() < FLAT_EPS) {
        return 0;
    }
    ac.sort_by(|a, b| a.total_cmp(b));
    let median = ac[ac.len() / 2];
    let mut h = 0u64;
    for (i, &v) in c.iter().enumerate().skip(1) {
        if v > median {
            h |= 1 << i;
        }
    }
    h
}

pub fn phash(img: &DynamicImage) -> u64 {
    phash_gray(&img.to_luma8())
}

pub fn hamming(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

/// Conversione per la colonna INTEGER di SQLite (bit pattern invariato).
pub fn to_sql(h: u64) -> i64 {
    h as i64
}

pub fn from_sql(v: i64) -> u64 {
    v as u64
}

// `inline(always)`: dentro `near_popcnt` il ciclo va compilato con `popcnt` abilitato.
#[inline(always)]
fn near_portable(hashes: &[u64], query: u64, max: u32, out: &mut Vec<usize>) {
    for (i, &h) in hashes.iter().enumerate() {
        if hamming(h, query) <= max {
            out.push(i);
        }
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "popcnt")]
unsafe fn near_popcnt(hashes: &[u64], query: u64, max: u32, out: &mut Vec<usize>) {
    near_portable(hashes, query, max, out)
}

/// Aggiunge a `out` gli indici i con `hamming(hashes[i], query) <= max`.
pub fn near(hashes: &[u64], query: u64, max: u32, out: &mut Vec<usize>) {
    #[cfg(target_arch = "x86_64")]
    {
        static HAS: OnceLock<bool> = OnceLock::new();
        if *HAS.get_or_init(|| is_x86_feature_detected!("popcnt")) {
            // SAFETY: popcnt verificato a runtime.
            unsafe { near_popcnt(hashes, query, max, out) };
            return;
        }
    }
    near_portable(hashes, query, max, out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{codecs::jpeg::JpegEncoder, ImageEncoder, Rgb, RgbImage};

    /// Rumore deterministico (xorshift) in [0,1). Il seme viene mescolato: semi vicini
    /// darebbero altrimenti scene quasi uguali.
    fn rng(seed: u64) -> impl FnMut() -> f64 {
        let mut s = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
        move || {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            (s >> 11) as f64 / (1u64 << 53) as f64
        }
    }

    /// Immagine con gradienti, forme e grana (come una foto: la ricompressione la altera).
    fn scene(seed: u64) -> RgbImage {
        let mut r = rng(seed);
        let (w, h) = (800u32, 600u32);
        let (a, b, c) = (r(), r(), r());
        let mut grain = rng(seed ^ 0xabc);
        let mut img = RgbImage::from_fn(w, h, |x, y| {
            let fx = x as f64 / w as f64;
            let fy = y as f64 / h as f64;
            let g = (grain() - 0.5) * 40.0;
            let px = |v: f64| (255.0 * v + g).clamp(0.0, 255.0) as u8;
            Rgb([
                px(a * fx + (1.0 - a) * fy),
                px(b * fy + (1.0 - b) * (fx * 3.0).sin().abs()),
                px(c * (fx * fy) + (1.0 - c) * 0.3),
            ])
        });
        for _ in 0..6 {
            let (cx, cy, rad) = (r() * w as f64, r() * h as f64, 40.0 + r() * 110.0);
            let col = Rgb([
                (r() * 255.0) as u8,
                (r() * 255.0) as u8,
                (r() * 255.0) as u8,
            ]);
            for y in 0..h {
                for x in 0..w {
                    let (dx, dy) = (x as f64 - cx, y as f64 - cy);
                    if dx * dx + dy * dy < rad * rad {
                        img.put_pixel(x, y, col);
                    }
                }
            }
        }
        img
    }

    fn jpeg_roundtrip(img: &DynamicImage, q: u8) -> DynamicImage {
        let rgb = img.to_rgb8();
        let mut buf = Vec::new();
        JpegEncoder::new_with_quality(&mut buf, q)
            .write_image(
                rgb.as_raw(),
                rgb.width(),
                rgb.height(),
                image::ExtendedColorType::Rgb8,
            )
            .unwrap();
        image::load_from_memory(&buf).unwrap()
    }

    /// Impronta come all'import: miniatura 512 px salvata in JPEG (qualità della cache).
    fn thumb_phash(img: &DynamicImage) -> u64 {
        phash(&jpeg_roundtrip(
            &img.resize(512, 512, FilterType::Triangle),
            82,
        ))
    }

    #[test]
    fn same_image_distance_zero() {
        let img = DynamicImage::ImageRgb8(scene(1));
        assert_eq!(hamming(phash(&img), phash(&img)), 0);
    }

    #[test]
    fn recompressed_downscaled_copy_is_close() {
        for seed in [1, 2, 3] {
            let img = DynamicImage::ImageRgb8(scene(seed));
            let base = thumb_phash(&img);
            // Copia a metà lato, JPEG 70, poi la sua miniatura.
            let small = img.resize_exact(400, 300, FilterType::Lanczos3);
            let d = hamming(base, thumb_phash(&jpeg_roundtrip(&small, 70)));
            // Esposizione +10%.
            let mut bright = img.to_rgb8();
            for p in bright.pixels_mut() {
                for v in p.0.iter_mut() {
                    *v = (*v as f32 * 1.1).min(255.0) as u8;
                }
            }
            let e = hamming(base, thumb_phash(&DynamicImage::ImageRgb8(bright)));
            println!("seed {seed}: ricompressa {d}, esposizione {e}");
            assert!(d <= 6, "ricompressa: distanza {d}");
            assert!(e <= 6, "esposizione: distanza {e}");
        }
    }

    #[test]
    fn different_images_are_far() {
        // Le scene sintetiche condividono la struttura (gradienti): margine misurato,
        // non quello di foto reali scorrelate (~32).
        let hs: Vec<u64> = (1..=8)
            .map(|s| phash(&DynamicImage::ImageRgb8(scene(s))))
            .collect();
        let mut d: Vec<u32> = (0..hs.len())
            .flat_map(|i| (i + 1..hs.len()).map(move |j| (i, j)))
            .map(|(i, j)| hamming(hs[i], hs[j]))
            .collect();
        d.sort_unstable();
        println!("diverse: min {} mediana {}", d[0], d[d.len() / 2]);
        assert!(d[0] > 10, "coppia diversa entro HAMMING_NEAR: {d:?}");
        assert!(d[d.len() / 2] > 20, "mediana {d:?}");
    }

    #[test]
    fn uniform_images_hash_to_zero() {
        // Immagini uniformi: impronta fissa 0, il giudice finale è la SSIM.
        let h = |v: u8| {
            phash(&DynamicImage::ImageRgb8(RgbImage::from_pixel(
                64,
                64,
                Rgb([v, v, v]),
            )))
        };
        assert_eq!((h(30), h(220)), (0, 0));
    }

    #[test]
    fn sql_roundtrip() {
        for h in [0u64, 1, u64::MAX, 1 << 63, 0xdead_beef_cafe_f00d] {
            assert_eq!(from_sql(to_sql(h)), h);
        }
    }

    fn data(n: usize) -> Vec<u64> {
        let mut r = rng(42);
        (0..n)
            .map(|_| (r() * u64::MAX as f64) as u64 ^ (r() * 1e9) as u64)
            .collect()
    }

    #[test]
    fn near_matches_portable() {
        let hs = data(5000);
        for q in [0u64, hs[7], hs[4000], u64::MAX] {
            for max in [0, 3, 10, 28, 32] {
                let (mut a, mut b, mut c) = (vec![], vec![], vec![]);
                near_portable(&hs, q, max, &mut a);
                near(&hs, q, max, &mut b);
                #[cfg(target_arch = "x86_64")]
                if is_x86_feature_detected!("popcnt") {
                    unsafe { near_popcnt(&hs, q, max, &mut c) };
                    assert_eq!(a, c);
                }
                assert_eq!(a, b);
            }
        }
        let mut o = vec![];
        near(&hs, hs[7], 0, &mut o);
        assert!(o.contains(&7));
    }

    #[test]
    #[ignore]
    fn bench_near_100k() {
        let hs = data(100_000);
        let mut out = Vec::new();
        let t = std::time::Instant::now();
        for q in 0..200 {
            out.clear();
            near(&hs, hs[q * 13], 10, &mut out);
        }
        let el = t.elapsed();
        println!("near 100k x 200 query: {el:?}");
        assert!(el < std::time::Duration::from_secs(1));
    }
}
