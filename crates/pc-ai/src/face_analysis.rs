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
use pc_store::people::{DetectedFace, IdentityRefs};
use pc_store::AppStore;
use std::sync::Arc;

/// Volto rilevato in un'immagine, candidato a diventare volto di riferimento.
#[derive(Debug, Clone)]
pub struct RefCandidate {
    /// Box normalizzato `[x, y, w, h]`.
    pub bbox: [f32; 4],
    /// Ritaglio allineato 112×112 in PNG (quello usato da SFace).
    pub crop_png: Vec<u8>,
    pub embedding: Vec<f32>,
    pub model_ver: String,
}

fn png(img: &RgbImage) -> AppResult<Vec<u8>> {
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png)
        .map_err(|e| AppError::new(ErrorCode::Internal, e.to_string()))?;
    Ok(out.into_inner())
}

/// Volti utilizzabili di un'immagine, con ritaglio allineato ed embedding.
pub fn reference_candidates(
    resources: &AiResources,
    img: &RgbImage,
    source_size: (u32, u32),
    min_pixels: u32,
) -> AppResult<Vec<RefCandidate>> {
    let det = resources
        .pool
        .get(&resources.entry(Role::FaceDetection)?.id)?;
    let rec = resources
        .pool
        .get(&resources.entry(Role::FaceEmbedding)?.id)?;
    let (model_ver, _) = versions(resources)?;
    let scale = ((source_size.0 as f64 * source_size.1 as f64)
        / (img.width().max(1) as f64 * img.height().max(1) as f64))
        .sqrt()
        .max(1.0) as f32;
    let mut out = Vec::new();
    for face in faces::detect_usable(&mut det.session.lock(), img, 0.8, min_pixels as f32 / scale)?
    {
        let Some(b) = normalized_box(&face.bbox, face.score, img.width(), img.height()) else {
            continue;
        };
        let aligned = faces::align(img, &face.landmarks);
        let embedding = faces::embed(&mut rec.session.lock(), &aligned)?;
        out.push(RefCandidate {
            bbox: [b[0], b[1], b[2], b[3]],
            crop_png: png(&aligned)?,
            embedding,
            model_ver: model_ver.clone(),
        });
    }
    Ok(out)
}

/// Lato massimo della decodifica per cercare volti di riferimento.
const REFERENCE_SIDE: u32 = 2048;

/// Volti di un file immagine scelto dall'utente ("aggiungi riferimento da file").
/// Il file viene solo letto: nell'archivio finiscono ritaglio ed embedding.
pub fn candidates_from_file(
    resources: &AiResources,
    governor: &pc_governor::MemoryGovernor,
    path: &std::path::Path,
    min_pixels: u32,
) -> AppResult<Vec<RefCandidate>> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let d = pc_ingest::thumb::decode_reduced(path, &ext, REFERENCE_SIDE, governor)?;
    let native = matches!(ext.as_str(), "heic" | "heif");
    let orientation = if native {
        None
    } else {
        pc_ingest::metadata::read_exif(path)
            .ok()
            .flatten()
            .and_then(|m| m.orientation)
    };
    let img = pc_ingest::thumb::apply_orientation(d.image, orientation);
    reference_candidates(resources, &img, d.source_size, min_pixels)
}

/// Volto di riferimento da un volto della libreria: si rianalizza l'originale
/// e si sceglie il volto rilevato più sovrapposto al box salvato.
pub fn candidate_from_library_face(
    resources: &AiResources,
    store: &pc_store::Store,
    governor: &pc_governor::MemoryGovernor,
    face_id: i64,
) -> AppResult<RefCandidate> {
    let (media_id, raw): (i64, String) = store.read(|c| {
        Ok(c.query_row(
            "SELECT media_id, bbox FROM face WHERE id = ?1",
            [face_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?)
    })?;
    let bbox: [f32; 4] = serde_json::from_str(&raw).map_err(|e| {
        AppError::new(
            ErrorCode::Internal,
            format!("box del volto non valido: {e}"),
        )
    })?;
    let d = crate::analyze::load_oriented(store, governor, media_id, REFERENCE_SIDE)?;
    reference_candidates(resources, &d.image, d.source_size, 1)?
        .into_iter()
        .map(|c| (pc_store::people::iou(c.bbox, bbox), c))
        .filter(|(o, _)| *o >= 0.3)
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, c)| c)
        .ok_or_else(|| AppError::new(ErrorCode::NotFound, "volto non ritrovato nell'originale"))
}

/// Ricalcola dai ritagli gli embedding dei volti di riferimento creati con un
/// altro modello (o senza embedding). Restituisce quanti sono stati aggiornati.
pub fn refresh_references(resources: &AiResources, app: &AppStore) -> AppResult<usize> {
    let (model_ver, _) = versions(resources)?;
    let stale = app.with(|c| pc_store::app_store::identities::stale_faces(c, &model_ver))?;
    if stale.is_empty() {
        return Ok(0);
    }
    let rec = resources
        .pool
        .get(&resources.entry(Role::FaceEmbedding)?.id)?;
    let mut done = 0;
    for f in stale {
        let img = match image::load_from_memory(&f.crop) {
            Ok(i) => i.to_rgb8(),
            Err(e) => {
                tracing::warn!(detail = %e, "ritaglio di riferimento non leggibile");
                continue;
            }
        };
        let img = if img.dimensions() == (faces::ALIGNED, faces::ALIGNED) {
            img
        } else {
            image::imageops::resize(
                &img,
                faces::ALIGNED,
                faces::ALIGNED,
                image::imageops::FilterType::CatmullRom,
            )
        };
        let v = faces::embed(&mut rec.session.lock(), &img)?;
        app.with(|c| pc_store::app_store::identities::set_face_embedding(c, f.id, &model_ver, &v))?;
        done += 1;
    }
    tracing::info!(
        count = done,
        model = model_ver.as_str(),
        "embedding dei volti di riferimento ricalcolati"
    );
    Ok(done)
}

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
    /// Volti di riferimento delle identità dell'app.
    pub refs: Arc<IdentityRefs>,
}
struct FaceWrite {
    faces: Vec<DetectedFace>,
    embedding_model: String,
    analysis_model: String,
    policy: Arc<crate::face_policy::FacePolicy>,
    generation: u64,
    refs: Arc<IdentityRefs>,
}
impl ExtraWrite for FaceWrite {
    fn write(&self, c: &rusqlite::Connection, id: i64) -> pc_store::StoreResult<()> {
        if !self.policy.is_current(self.generation) {
            return Ok(());
        }
        pc_store::people::save_faces_with(
            c,
            id,
            &self.faces,
            &self.embedding_model,
            &self.analysis_model,
            faces::SAME_PERSON,
            &self.refs,
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
                refs: self.refs.clone(),
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
            refs: Arc::default(),
        };
        policy.invalidate();
        pc_store::people::clear(&c).unwrap();
        policy.set_enabled(true);
        queued.write(&c, 1).unwrap();
        assert!(pc_store::people::records(&c, None).unwrap().is_empty());
    }
}
