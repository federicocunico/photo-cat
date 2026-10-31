//! Rilevamento persone con YOLOX (classe COCO 0 = persona).

use crate::tensor::{bgr_raw, letterbox, nms, ort_err, BBox};
use image::RgbImage;
use ort::session::Session;
use ort::value::Tensor;
use pc_core::AppResult;

pub const INPUT: u32 = 416;
const STRIDES: [usize; 3] = [8, 16, 32];

#[derive(Debug, Clone, PartialEq)]
pub struct Person {
    pub bbox: BBox,
    pub score: f32,
}

pub fn detect(session: &mut Session, img: &RgbImage, threshold: f32) -> AppResult<Vec<Person>> {
    let lb = letterbox(img, INPUT, 114);
    let input = Tensor::from_array(bgr_raw(&lb.image)).map_err(ort_err)?;
    let out = session
        .run(ort::inputs!["images" => input])
        .map_err(ort_err)?;
    let (_, data) = out["output"].try_extract_tensor::<f32>().map_err(ort_err)?;
    let mut cands = Vec::new();
    let mut idx = 0usize;
    for s in STRIDES {
        let g = INPUT as usize / s;
        for gy in 0..g {
            for gx in 0..g {
                let row = &data[idx * 85..idx * 85 + 85];
                idx += 1;
                let score = row[4] * row[5]; // oggettività × probabilità "persona"
                if score < threshold {
                    continue;
                }
                let cx = (row[0] + gx as f32) * s as f32;
                let cy = (row[1] + gy as f32) * s as f32;
                let w = row[2].exp() * s as f32;
                let h = row[3].exp() * s as f32;
                let bbox = BBox {
                    x: (cx - w / 2.0) / lb.scale,
                    y: (cy - h / 2.0) / lb.scale,
                    w: w / lb.scale,
                    h: h / lb.scale,
                };
                cands.push((bbox, score));
            }
        }
    }
    Ok(nms(&cands, 0.45)
        .into_iter()
        .map(|i| Person {
            bbox: cands[i].0,
            score: cands[i].1,
        })
        .collect())
}
