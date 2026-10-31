//! CLIP ViT-B/32: embedding di immagini e testi nello stesso spazio (512 dimensioni).

use crate::tensor::{clip_batch, l2_normalize, ort_err};
use image::RgbImage;
use ort::session::Session;
use ort::value::Tensor;
use pc_core::{AppError, AppResult, ErrorCode};
use std::path::Path;

pub const DIM: usize = 512;
const MAX_TOKENS: usize = 77;

/// Embedding normalizzati di un batch di immagini.
pub fn embed_images(session: &mut Session, imgs: &[&RgbImage]) -> AppResult<Vec<Vec<f32>>> {
    if imgs.is_empty() {
        return Ok(Vec::new());
    }
    let input = Tensor::from_array(clip_batch(imgs)).map_err(ort_err)?;
    let out = session
        .run(ort::inputs!["pixel_values" => input])
        .map_err(ort_err)?;
    let arr = out["image_embeds"]
        .try_extract_array::<f32>()
        .map_err(ort_err)?;
    let mut res = Vec::with_capacity(imgs.len());
    for row in arr.outer_iter() {
        let mut v: Vec<f32> = row.iter().copied().collect();
        l2_normalize(&mut v);
        res.push(v);
    }
    Ok(res)
}

pub struct ClipTokenizer {
    inner: tokenizers::Tokenizer,
    eos: i64,
}

impl ClipTokenizer {
    pub fn load(path: &Path) -> AppResult<Self> {
        let inner = tokenizers::Tokenizer::from_file(path).map_err(|e| {
            AppError::new(ErrorCode::ModelUnavailable, format!("tokenizer CLIP: {e}"))
        })?;
        let eos = inner.token_to_id("<|endoftext|>").unwrap_or(49407) as i64;
        Ok(Self { inner, eos })
    }

    /// Id dei token con inizio/fine testo, troncati a 77.
    pub fn encode(&self, text: &str) -> AppResult<Vec<i64>> {
        let enc = self
            .inner
            .encode(text.to_lowercase(), true)
            .map_err(|e| AppError::new(ErrorCode::InvalidInput, format!("tokenizzazione: {e}")))?;
        let mut ids: Vec<i64> = enc.get_ids().iter().map(|&i| i as i64).collect();
        if ids.len() > MAX_TOKENS {
            ids.truncate(MAX_TOKENS - 1);
            ids.push(self.eos);
        }
        if ids.last() != Some(&self.eos) {
            ids.push(self.eos);
        }
        Ok(ids)
    }
}

/// Embedding normalizzati dei testi (uno per volta: lunghezze diverse, nessun padding).
pub fn embed_texts(
    session: &mut Session,
    tok: &ClipTokenizer,
    texts: &[&str],
) -> AppResult<Vec<Vec<f32>>> {
    let mut res = Vec::with_capacity(texts.len());
    for t in texts {
        let ids = tok.encode(t)?;
        let input = Tensor::from_array(([1usize, ids.len()], ids)).map_err(ort_err)?;
        let out = session
            .run(ort::inputs!["input_ids" => input])
            .map_err(ort_err)?;
        let (_, data) = out["text_embeds"]
            .try_extract_tensor::<f32>()
            .map_err(ort_err)?;
        let mut v = data.to_vec();
        l2_normalize(&mut v);
        res.push(v);
    }
    Ok(res)
}

/// Embedding di un concetto come media di più formulazioni (prompt ensemble).
pub fn embed_concept(
    session: &mut Session,
    tok: &ClipTokenizer,
    prompts: &[String],
) -> AppResult<Vec<f32>> {
    let refs: Vec<&str> = prompts.iter().map(String::as_str).collect();
    let all = embed_texts(session, tok, &refs)?;
    let mut mean = vec![0.0f32; DIM];
    for v in &all {
        for (m, x) in mean.iter_mut().zip(v) {
            *m += x;
        }
    }
    l2_normalize(&mut mean);
    Ok(mean)
}
