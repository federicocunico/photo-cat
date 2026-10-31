//! Preparazione degli input dei modelli e utilità numeriche.

use image::{imageops, imageops::FilterType, RgbImage};
use ndarray::Array4;
use pc_core::{AppError, ErrorCode};

pub fn ort_err(e: impl std::fmt::Display) -> AppError {
    AppError::new(ErrorCode::ModelUnavailable, format!("inferenza: {e}"))
}

/// Immagine ridimensionata dentro un quadrato `size×size` (in alto a sinistra), con il fattore di scala.
pub struct Letterbox {
    pub image: RgbImage,
    pub scale: f32,
}

pub fn letterbox(img: &RgbImage, size: u32, pad: u8) -> Letterbox {
    let (w, h) = img.dimensions();
    let scale = (size as f32 / w as f32).min(size as f32 / h as f32);
    let (nw, nh) = (
        ((w as f32 * scale).round() as u32).max(1),
        ((h as f32 * scale).round() as u32).max(1),
    );
    let resized = imageops::resize(img, nw, nh, FilterType::Triangle);
    let mut canvas = RgbImage::from_pixel(size, size, image::Rgb([pad, pad, pad]));
    imageops::replace(&mut canvas, &resized, 0, 0);
    Letterbox {
        image: canvas,
        scale,
    }
}

/// Tensore NCHW con valori 0–255 in ordine BGR (modelli addestrati con OpenCV).
pub fn bgr_raw(img: &RgbImage) -> Array4<f32> {
    let (w, h) = img.dimensions();
    let mut t = Array4::<f32>::zeros((1, 3, h as usize, w as usize));
    for (x, y, p) in img.enumerate_pixels() {
        let (x, y) = (x as usize, y as usize);
        t[[0, 0, y, x]] = p[2] as f32;
        t[[0, 1, y, x]] = p[1] as f32;
        t[[0, 2, y, x]] = p[0] as f32;
    }
    t
}

/// Tensore NCHW con valori 0–255 in ordine RGB.
pub fn rgb_raw(img: &RgbImage) -> Array4<f32> {
    let (w, h) = img.dimensions();
    let mut t = Array4::<f32>::zeros((1, 3, h as usize, w as usize));
    for (x, y, p) in img.enumerate_pixels() {
        for c in 0..3 {
            t[[0, c, y as usize, x as usize]] = p[c] as f32;
        }
    }
    t
}

const CLIP_MEAN: [f32; 3] = [0.481_454_66, 0.457_827_5, 0.408_210_73];
const CLIP_STD: [f32; 3] = [0.268_629_54, 0.261_302_6, 0.275_777_1];
pub const CLIP_SIZE: u32 = 224;

/// Preprocessing CLIP: lato corto a 224, ritaglio centrale, normalizzazione.
pub fn clip_batch(imgs: &[&RgbImage]) -> Array4<f32> {
    let s = CLIP_SIZE as usize;
    let mut t = Array4::<f32>::zeros((imgs.len(), 3, s, s));
    for (n, img) in imgs.iter().enumerate() {
        let (w, h) = img.dimensions();
        let f = CLIP_SIZE as f32 / w.min(h) as f32;
        let (nw, nh) = (
            ((w as f32 * f).round() as u32).max(CLIP_SIZE),
            ((h as f32 * f).round() as u32).max(CLIP_SIZE),
        );
        let resized = imageops::resize(*img, nw, nh, FilterType::CatmullRom);
        let (ox, oy) = ((nw - CLIP_SIZE) / 2, (nh - CLIP_SIZE) / 2);
        let crop = imageops::crop_imm(&resized, ox, oy, CLIP_SIZE, CLIP_SIZE).to_image();
        for (x, y, p) in crop.enumerate_pixels() {
            for c in 0..3 {
                t[[n, c, y as usize, x as usize]] =
                    (p[c] as f32 / 255.0 - CLIP_MEAN[c]) / CLIP_STD[c];
            }
        }
    }
    t
}

pub fn l2_normalize(v: &mut [f32]) {
    let n = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if n > 0.0 {
        v.iter_mut().for_each(|x| *x /= n);
    }
}

pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na * nb)
    }
}

pub fn softmax(v: &[f32], temperature: f32) -> Vec<f32> {
    let m = v.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let e: Vec<f32> = v.iter().map(|x| ((x - m) * temperature).exp()).collect();
    let s: f32 = e.iter().sum();
    e.into_iter().map(|x| x / s).collect()
}

/// Riquadro in pixel dell'immagine originale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BBox {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl BBox {
    pub fn iou(&self, o: &BBox) -> f32 {
        let x1 = self.x.max(o.x);
        let y1 = self.y.max(o.y);
        let x2 = (self.x + self.w).min(o.x + o.w);
        let y2 = (self.y + self.h).min(o.y + o.h);
        let inter = (x2 - x1).max(0.0) * (y2 - y1).max(0.0);
        let union = self.w * self.h + o.w * o.h - inter;
        if union <= 0.0 {
            0.0
        } else {
            inter / union
        }
    }

    /// Normalizzato 0..1 rispetto alle dimensioni dell'immagine.
    pub fn normalized(&self, w: u32, h: u32) -> [f32; 4] {
        let (w, h) = (w as f32, h as f32);
        [
            (self.x / w).clamp(0.0, 1.0),
            (self.y / h).clamp(0.0, 1.0),
            (self.w / w).clamp(0.0, 1.0),
            (self.h / h).clamp(0.0, 1.0),
        ]
    }
}

/// Non-maximum suppression: indici da tenere, in ordine di punteggio decrescente.
pub fn nms(boxes: &[(BBox, f32)], iou: f32) -> Vec<usize> {
    let mut order: Vec<usize> = (0..boxes.len()).collect();
    order.sort_by(|a, b| boxes[*b].1.total_cmp(&boxes[*a].1));
    let mut keep: Vec<usize> = Vec::new();
    for i in order {
        if keep.iter().all(|&k| boxes[k].0.iou(&boxes[i].0) <= iou) {
            keep.push(i);
        }
    }
    keep
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letterbox_keeps_aspect_ratio() {
        let img = RgbImage::from_pixel(800, 400, image::Rgb([1, 2, 3]));
        let lb = letterbox(&img, 416, 114);
        assert_eq!(lb.image.dimensions(), (416, 416));
        assert!((lb.scale - 0.52).abs() < 1e-3);
        assert_eq!(lb.image.get_pixel(10, 300)[0], 114, "padding in basso");
    }

    #[test]
    fn clip_batch_shape_and_normalization() {
        let img = RgbImage::from_pixel(640, 480, image::Rgb([255, 255, 255]));
        let t = clip_batch(&[&img, &img]);
        assert_eq!(t.shape(), &[2, 3, 224, 224]);
        assert!((t[[0, 0, 0, 0]] - (1.0 - CLIP_MEAN[0]) / CLIP_STD[0]).abs() < 1e-4);
    }

    #[test]
    fn nms_and_math() {
        let b = |x: f32| BBox {
            x,
            y: 0.0,
            w: 10.0,
            h: 10.0,
        };
        let keep = nms(&[(b(0.0), 0.9), (b(1.0), 0.8), (b(50.0), 0.7)], 0.5);
        assert_eq!(keep, vec![0, 2]);
        assert!((cosine(&[1.0, 0.0], &[1.0, 0.0]) - 1.0).abs() < 1e-6);
        let p = softmax(&[1.0, 1.0], 100.0);
        assert!((p[0] - 0.5).abs() < 1e-6);
    }
}
