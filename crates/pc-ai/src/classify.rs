//! Classificazione CLIP con softmax per gruppo e batch dimensionati dal governor.
use crate::{clip, taxonomy::Taxonomy, tensor::cosine, text_cache::Concept};
use image::RgbImage;
use pc_core::{AppError, AppResult, ErrorCode};
use pc_governor::{budget::recommended_batch, MemoryGovernor};
use pc_models::{pool::ModelPool, store::ModelStore, Role};
use pc_store::analysis::AutoLabel;

pub struct Classified {
    pub embedding: Vec<f32>,
    pub labels: Vec<AutoLabel>,
}

pub fn labels(image: &[f32], taxonomy: &Taxonomy, concepts: &[Concept]) -> Vec<AutoLabel> {
    let mut result = Vec::new();
    for group in &taxonomy.groups {
        let candidates: Vec<_> = concepts.iter().filter(|c| c.group == group.id).collect();
        if candidates.is_empty() {
            continue;
        }
        let logits: Vec<_> = candidates
            .iter()
            .map(|c| cosine(image, &c.vector) * 100.0)
            .collect();
        let max = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let weights: Vec<_> = logits.iter().map(|x| (x - max).exp()).collect();
        let sum: f32 = weights.iter().sum();
        for (c, w) in candidates.iter().zip(weights) {
            let score = w / sum;
            if score.is_finite() && score >= group.threshold {
                result.push(AutoLabel {
                    group: group.id.clone(),
                    value: c.value.clone(),
                    score: score as f64,
                });
            }
        }
    }
    result
}

pub fn images(
    imgs: &[&RgbImage],
    taxonomy: &Taxonomy,
    concepts: &[Concept],
    models: &ModelStore,
    pool: &ModelPool,
    governor: &MemoryGovernor,
) -> AppResult<Vec<Classified>> {
    let entry = models
        .manifest()
        .by_role(Role::ClassificationImage)
        .ok_or_else(|| AppError::new(ErrorCode::ModelUnavailable, "modello immagine mancante"))?;
    let model = pool.get(&entry.id)?;
    let batch = recommended_batch(governor.free_mb(), 32, 8) as usize;
    let mut result = Vec::new();
    for chunk in imgs.chunks(batch) {
        let _permit = governor
            .try_acquire(32 * chunk.len() as u64)
            .ok_or_else(|| {
                AppError::new(
                    ErrorCode::InsufficientMemory,
                    "memoria insufficiente per il batch CLIP",
                )
            })?;
        for embedding in clip::embed_images(&mut model.session.lock(), chunk)? {
            let assigned = labels(&embedding, taxonomy, concepts);
            result.push(Classified {
                embedding,
                labels: assigned,
            });
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn confidence_is_per_group_and_respects_thresholds() {
        let mut t = Taxonomy::default();
        t.groups.truncate(1);
        t.groups[0].threshold = 0.6;
        let concepts = vec![
            Concept {
                group: "subject".into(),
                value: "one".into(),
                vector: vec![1.0, 0.0],
            },
            Concept {
                group: "subject".into(),
                value: "two".into(),
                vector: vec![0.0, 1.0],
            },
        ];
        assert_eq!(labels(&[1.0, 0.0], &t, &concepts)[0].value, "one");
        assert!(labels(&[0.5, 0.5], &t, &concepts).is_empty());
    }
}
