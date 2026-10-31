//! Volti: rilevamento con YuNet, allineamento sui 5 punti di riferimento,
//! embedding con SFace (128 dimensioni).

use crate::tensor::{bgr_raw, l2_normalize, letterbox, nms, ort_err, rgb_raw, BBox};
use image::{Rgb, RgbImage};
use ort::session::Session;
use ort::value::Tensor;
use pc_core::AppResult;

pub const DET_INPUT: u32 = 640;
pub const ALIGNED: u32 = 112;
const STRIDES: [usize; 3] = [8, 16, 32];
/// Posizioni di riferimento (occhi, naso, angoli della bocca) nel ritaglio 112×112.
const TEMPLATE: [(f32, f32); 5] = [
    (38.2946, 51.6963),
    (73.5318, 51.5014),
    (56.0252, 71.7366),
    (41.5493, 92.3655),
    (70.7299, 92.2041),
];
/// Soglia di somiglianza coseno di SFace per "stessa persona" (OpenCV).
pub const SAME_PERSON: f32 = 0.363;

#[derive(Debug, Clone, PartialEq)]
pub struct Face {
    pub bbox: BBox,
    pub score: f32,
    /// Occhio destro, occhio sinistro, naso, bocca destra, bocca sinistra (pixel originali).
    pub landmarks: [(f32, f32); 5],
}

impl Face {
    /// Lato minore del riquadro in pixel.
    pub fn size(&self) -> f32 {
        self.bbox.w.min(self.bbox.h)
    }
}

pub fn detect(session: &mut Session, img: &RgbImage, threshold: f32) -> AppResult<Vec<Face>> {
    let lb = letterbox(img, DET_INPUT, 0);
    let input = Tensor::from_array(bgr_raw(&lb.image)).map_err(ort_err)?;
    let out = session
        .run(ort::inputs!["input" => input])
        .map_err(ort_err)?;
    let mut cands: Vec<(BBox, f32)> = Vec::new();
    let mut marks: Vec<[(f32, f32); 5]> = Vec::new();
    for s in STRIDES {
        let get = |name: String| {
            out[name.as_str()]
                .try_extract_tensor::<f32>()
                .map(|(_, d)| d.to_vec())
        };
        let cls = get(format!("cls_{s}")).map_err(ort_err)?;
        let obj = get(format!("obj_{s}")).map_err(ort_err)?;
        let bbox = get(format!("bbox_{s}")).map_err(ort_err)?;
        let kps = get(format!("kps_{s}")).map_err(ort_err)?;
        let g = DET_INPUT as usize / s;
        for i in 0..g * g {
            let score = (cls[i].clamp(0.0, 1.0) * obj[i].clamp(0.0, 1.0)).sqrt();
            if score < threshold {
                continue;
            }
            let (c, r) = ((i % g) as f32, (i / g) as f32);
            let sf = s as f32;
            let cx = (c + bbox[i * 4]) * sf;
            let cy = (r + bbox[i * 4 + 1]) * sf;
            let w = bbox[i * 4 + 2].exp() * sf;
            let h = bbox[i * 4 + 3].exp() * sf;
            let b = BBox {
                x: (cx - w / 2.0) / lb.scale,
                y: (cy - h / 2.0) / lb.scale,
                w: w / lb.scale,
                h: h / lb.scale,
            };
            let mut lm = [(0.0, 0.0); 5];
            for (n, p) in lm.iter_mut().enumerate() {
                *p = (
                    (kps[i * 10 + 2 * n] + c) * sf / lb.scale,
                    (kps[i * 10 + 2 * n + 1] + r) * sf / lb.scale,
                );
            }
            cands.push((b, score));
            marks.push(lm);
        }
    }
    Ok(nms(&cands, 0.3)
        .into_iter()
        .map(|i| Face {
            bbox: cands[i].0,
            score: cands[i].1,
            landmarks: marks[i],
        })
        .collect())
}

pub fn detect_usable(
    session: &mut Session,
    img: &RgbImage,
    threshold: f32,
    min_size: f32,
) -> AppResult<Vec<Face>> {
    Ok(detect(session, img, threshold)?
        .into_iter()
        .filter(|f| f.size() >= min_size && f.score >= threshold)
        .collect())
}

/// Trasformazione di similitudine (scala, rotazione, traslazione) ai minimi quadrati: `src → dst`.
/// Restituisce `[a, b, tx, ty]` con `x' = a·x − b·y + tx`, `y' = b·x + a·y + ty`.
pub fn similarity(src: &[(f32, f32)], dst: &[(f32, f32)]) -> [f32; 4] {
    let n = src.len() as f32;
    let (sx, sy) = src
        .iter()
        .fold((0.0, 0.0), |a, p| (a.0 + p.0 / n, a.1 + p.1 / n));
    let (dx, dy) = dst
        .iter()
        .fold((0.0, 0.0), |a, p| (a.0 + p.0 / n, a.1 + p.1 / n));
    let (mut num_a, mut num_b, mut den) = (0.0f32, 0.0f32, 0.0f32);
    for (s, d) in src.iter().zip(dst) {
        let (x, y) = (s.0 - sx, s.1 - sy);
        let (u, v) = (d.0 - dx, d.1 - dy);
        num_a += x * u + y * v;
        num_b += x * v - y * u;
        den += x * x + y * y;
    }
    let a = num_a / den.max(1e-6);
    let b = num_b / den.max(1e-6);
    [a, b, dx - (a * sx - b * sy), dy - (b * sx + a * sy)]
}

/// Ritaglio 112×112 allineato (campionamento bilineare con la trasformazione inversa).
pub fn align(img: &RgbImage, landmarks: &[(f32, f32); 5]) -> RgbImage {
    let [a, b, tx, ty] = similarity(landmarks, &TEMPLATE);
    let det = a * a + b * b;
    let (w, h) = img.dimensions();
    let sample = |x: f32, y: f32| -> Rgb<u8> {
        if x < 0.0 || y < 0.0 || x > (w - 1) as f32 || y > (h - 1) as f32 {
            return Rgb([0, 0, 0]);
        }
        let (x0, y0) = (x.floor() as u32, y.floor() as u32);
        let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
        let (fx, fy) = (x - x0 as f32, y - y0 as f32);
        let p = |xx, yy| img.get_pixel(xx, yy).0;
        let mut out = [0u8; 3];
        for (c, o) in out.iter_mut().enumerate() {
            let top = p(x0, y0)[c] as f32 * (1.0 - fx) + p(x1, y0)[c] as f32 * fx;
            let bot = p(x0, y1)[c] as f32 * (1.0 - fx) + p(x1, y1)[c] as f32 * fx;
            *o = (top * (1.0 - fy) + bot * fy).round() as u8;
        }
        Rgb(out)
    };
    RgbImage::from_fn(ALIGNED, ALIGNED, |u, v| {
        // Inversa di [a −b; b a]·p + t
        let (u, v) = (u as f32 - tx, v as f32 - ty);
        let x = (a * u + b * v) / det;
        let y = (-b * u + a * v) / det;
        sample(x, y)
    })
}

/// Embedding normalizzato di un volto allineato.
pub fn embed(session: &mut Session, aligned: &RgbImage) -> AppResult<Vec<f32>> {
    let input = Tensor::from_array(rgb_raw(aligned)).map_err(ort_err)?;
    let out = session
        .run(ort::inputs!["data" => input])
        .map_err(ort_err)?;
    let (_, data) = out["fc1"].try_extract_tensor::<f32>().map_err(ort_err)?;
    let mut v = data.to_vec();
    l2_normalize(&mut v);
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn similarity_recovers_known_transform() {
        // Rotazione di 30°, scala 2, traslazione (5, −3).
        let (c, s) = (
            30f32.to_radians().cos() * 2.0,
            30f32.to_radians().sin() * 2.0,
        );
        let src: Vec<(f32, f32)> = TEMPLATE.to_vec();
        let dst: Vec<(f32, f32)> = src
            .iter()
            .map(|&(x, y)| (c * x - s * y + 5.0, s * x + c * y - 3.0))
            .collect();
        let [a, b, tx, ty] = similarity(&src, &dst);
        assert!((a - c).abs() < 1e-3 && (b - s).abs() < 1e-3);
        assert!((tx - 5.0).abs() < 1e-2 && (ty + 3.0).abs() < 1e-2);
    }

    #[test]
    fn align_maps_template_points_onto_themselves() {
        let img = RgbImage::from_fn(112, 112, |x, y| Rgb([x as u8, y as u8, 0]));
        let out = align(&img, &TEMPLATE);
        assert_eq!(out.get_pixel(50, 60).0[0], 50);
        assert_eq!(out.get_pixel(50, 60).0[1], 60);
    }
}
