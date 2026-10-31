//! Rilevamento, filtro qualità/dimensione, allineamento e embedding dei volti.
use crate::{
    faces,
    stages::{model_key, normalized_box, AiResources},
};
use image::RgbImage;
use pc_core::{AppError, AppResult, ErrorCode};
use pc_ingest::import::{ExtraWrite, ImportItem, ImportStage};
use pc_jobs::{PhaseSpec, StageCtx, StageOutcome};
use pc_models::Role;
use pc_store::people::DetectedFace;
use std::sync::Arc;

pub fn versions(resources: &AiResources) -> AppResult<(String, String)> {
    let det = model_key(resources.entry(Role::FaceDetection)?);
    let emb = model_key(resources.entry(Role::FaceEmbedding)?);
    Ok((emb.clone(), format!("{det}+{emb}")))
}
pub fn analyze(
    resources: &AiResources,
    img: &RgbImage,
    source_size: (u32, u32),
    min_pixels: u32,
    threshold: f32,
) -> AppResult<Vec<DetectedFace>> {
    let det = resources
        .pool
        .get(&resources.entry(Role::FaceDetection)?.id)?;
    let rec = resources
        .pool
        .get(&resources.entry(Role::FaceEmbedding)?.id)?;
    let _permit = resources.governor.try_acquire(32).ok_or_else(|| {
        AppError::new(
            ErrorCode::InsufficientMemory,
            "memoria insufficiente per analizzare i volti",
        )
    })?;
    let scale = ((source_size.0 as f64 * source_size.1 as f64)
        / (img.width().max(1) as f64 * img.height().max(1) as f64))
        .sqrt()
        .max(1.0) as f32;
    let detected = faces::detect_usable(
        &mut det.session.lock(),
        img,
        threshold,
        min_pixels as f32 / scale,
    )?;
    let mut result = Vec::new();
    for face in detected {
        let Some(b) = normalized_box(&face.bbox, face.score, img.width(), img.height()) else {
            continue;
        };
        let aligned = faces::align(img, &face.landmarks);
        let embedding = faces::embed(&mut rec.session.lock(), &aligned)?;
        result.push(DetectedFace {
            bbox: [b[0], b[1], b[2], b[3]],
            quality: face.score,
            embedding,
        });
    }
    Ok(result)
}

pub struct FaceStage {
    pub resources: AiResources,
    pub min_pixels: u32,
    pub threshold: f32,
    pub policy: Arc<crate::face_policy::FacePolicy>,
}
struct FaceWrite {
    faces: Vec<DetectedFace>,
    embedding_model: String,
    analysis_model: String,
    policy: Arc<crate::face_policy::FacePolicy>,
    generation: u64,
}
impl ExtraWrite for FaceWrite {
    fn write(&self, c: &rusqlite::Connection, id: i64) -> pc_store::StoreResult<()> {
        if !self.policy.is_current(self.generation) {
            return Ok(());
        }
        pc_store::people::save_faces(
            c,
            id,
            &self.faces,
            &self.embedding_model,
            &self.analysis_model,
            faces::SAME_PERSON,
        )
    }
}
impl ImportStage for FaceStage {
    fn prepare(&self) -> AppResult<()> {
        for role in [Role::FaceDetection, Role::FaceEmbedding] {
            self.resources.pool.get(&self.resources.entry(role)?.id)?;
        }
        Ok(())
    }
    fn phase(&self) -> PhaseSpec {
        PhaseSpec::new("faces", 4.0)
    }
    fn workers(&self) -> usize {
        1
    }
    fn process(&self, item: &mut ImportItem, ctx: &StageCtx) -> StageOutcome {
        let Some(d) = &item.decoded else {
            return StageOutcome::Continue;
        };
        let Some(generation) = self.policy.token() else {
            return StageOutcome::Continue;
        };
        if !self.resources.ready(ctx) {
            return StageOutcome::Continue;
        }
        let source = item
            .tech
            .as_ref()
            .and_then(|t| Some((t.width?, t.height?)))
            .unwrap_or(d.source_size);
        let run = || -> AppResult<FaceWrite> {
            let faces = analyze(
                &self.resources,
                &d.image,
                source,
                self.min_pixels,
                self.threshold,
            )?;
            let (embedding_model, analysis_model) = versions(&self.resources)?;
            Ok(FaceWrite {
                faces,
                embedding_model,
                analysis_model,
                policy: self.policy.clone(),
                generation,
            })
        };
        match run() {
            Ok(data) => {
                item.extra.push(Box::new(data));
                StageOutcome::Continue
            }
            Err(e) => StageOutcome::ContinueWithError(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn queued_biometric_results_cannot_reappear_after_purge() {
        let mut c = rusqlite::Connection::open_in_memory().unwrap();
        pc_store::migrate::migrate(&mut c, None).unwrap();
        c.execute("INSERT INTO media_item(id,path,name,ext,size,mtime,added_at,updated_at) VALUES (1,'test.jpg','test','jpg',1,1,1,1)",[]).unwrap();
        let policy = Arc::new(crate::face_policy::FacePolicy::new(true));
        let queued = FaceWrite {
            faces: vec![DetectedFace {
                bbox: [0.1, 0.1, 0.2, 0.2],
                quality: 1.0,
                embedding: {
                    let mut v = vec![0.0; 128];
                    v[0] = 1.0;
                    v
                },
            }],
            embedding_model: "sface".into(),
            analysis_model: "det+sface".into(),
            generation: policy.token().unwrap(),
            policy: policy.clone(),
        };
        policy.invalidate();
        pc_store::people::clear(&c).unwrap();
        policy.set_enabled(true);
        queued.write(&c, 1).unwrap();
        assert!(pc_store::people::records(&c, None).unwrap().is_empty());
    }
}
