//! Job di manutenzione `face_quality`: per i volti importati prima dello schema
//! v7 (senza nitidezza né dimensione salvate) ricalcola nitidezza, dimensione e
//! `quality_score` a partire dai ritagli dei volti, senza eseguire modelli AI.

use crate::analyze::load_oriented;
use crate::face_crop::{self, CROP_SIZE};
use crate::quality::sharpness_from_display_crop;
use pc_core::AppResult;
use pc_governor::MemoryGovernor;
use pc_jobs::{JobManager, JobPlan, PhaseSpec, RunSummary};
use pc_store::{people::combined_quality, Store, StoreResult};
use rusqlite::OptionalExtension;
use std::path::{Path, PathBuf};

pub const JOB_KIND: &str = "face_quality";
pub const PHASE: &str = "qualita_volti";
/// Marcatore in `settings_kv`: job già accodato per questa libreria.
pub const QUEUED_KEY: &str = "face_quality_queued";
const CHUNK: usize = 64;

#[derive(Debug, Clone)]
pub struct PendingFace {
    pub id: i64,
    pub media_id: i64,
    pub bbox_raw: String,
    pub bbox: [f32; 4],
    pub quality: f32,
    pub frontality: Option<f32>,
}

/// Volti ancora privi della misura di nitidezza.
pub fn pending(store: &Store) -> AppResult<Vec<PendingFace>> {
    Ok(store.read(|c| {
        let mut q = c.prepare(
            "SELECT f.id, f.media_id, f.bbox, f.quality, f.frontality
             FROM face f JOIN media_item m ON m.id = f.media_id
             WHERE f.sharpness IS NULL AND m.status = 'ok'
             ORDER BY f.media_id, f.id",
        )?;
        let rows = q
            .query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, f32>(3)?,
                    r.get::<_, Option<f32>>(4)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        rows.into_iter()
            .map(|(id, media_id, bbox_raw, quality, frontality)| {
                let bbox: [f32; 4] = serde_json::from_str(&bbox_raw)?;
                Ok(PendingFace {
                    id,
                    media_id,
                    bbox_raw,
                    bbox,
                    quality,
                    frontality,
                })
            })
            .collect::<StoreResult<Vec<_>>>()
    })?)
}

/// Accoda il job una sola volta per libreria, se ci sono volti senza qualità.
pub fn queue_once(store: &Store, jobs: &JobManager) -> AppResult<Option<i64>> {
    let queued: Option<String> = store.read(|c| {
        Ok(c.query_row(
            "SELECT value FROM settings_kv WHERE key = ?1",
            [QUEUED_KEY],
            |r| r.get(0),
        )
        .optional()?)
    })?;
    if queued.is_some() || pending(store)?.is_empty() {
        return Ok(None);
    }
    let id = jobs.submit(JOB_KIND, serde_json::json!({}))?;
    store.write(move |c| {
        c.execute(
            "INSERT INTO settings_kv(key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![QUEUED_KEY, id.to_string()],
        )?;
        Ok(())
    })?;
    Ok(Some(id))
}

fn measure_one(
    store: &Store,
    governor: &MemoryGovernor,
    crops_dir: Option<&Path>,
    f: &PendingFace,
) -> (f32, u32, f32) {
    // Dimensione originale del media (se salvata in media_tech).
    let tech_dims: Option<(u32, u32)> = store
        .read({
            let media_id = f.media_id;
            move |c| {
                Ok(pc_store::repo::meta::tech(c, media_id)?
                    .and_then(|t| Some((t.width?, t.height?))))
            }
        })
        .ok()
        .flatten();
    // Se il ritaglio è già in cache, non serve riaprire la foto se conosciamo le dimensioni.
    if let Some(dir) = crops_dir {
        let cached = face_crop::cache_path(dir, f.id, &f.bbox_raw);
        if let Ok(bytes) = std::fs::read(&cached) {
            if let Ok(img) = image::load_from_memory(&bytes) {
                let rgb = img.to_rgb8();
                let sharp = sharpness_from_display_crop(&rgb);
                let size_px = tech_dims
                    .map(|(w, h)| {
                        ((f.bbox[2] * w as f32).min(f.bbox[3] * h as f32))
                            .round()
                            .max(1.0) as u32
                    })
                    .unwrap_or(96);
                let qscore = combined_quality(f.quality, Some(sharp), f.frontality, Some(size_px));
                return (sharp, size_px, qscore);
            }
        }
    }
    let face_side = f.bbox[2].max(f.bbox[3]).max(0.01);
    let need = ((CROP_SIZE as f32 * 1.70 / face_side).ceil() as u32).clamp(512, 4096);
    if let Ok(decoded) = load_oriented(store, governor, f.media_id, need) {
        let (sw, sh) = tech_dims.unwrap_or(decoded.source_size);
        let size_px = ((f.bbox[2] * sw as f32).min(f.bbox[3] * sh as f32))
            .round()
            .max(1.0) as u32;
        let cropped = face_crop::crop(&decoded.image, f.bbox);
        drop(decoded);
        let sharp = sharpness_from_display_crop(&cropped);
        let qscore = combined_quality(f.quality, Some(sharp), f.frontality, Some(size_px));
        return (sharp, size_px, qscore);
    }
    // File non decodificabile: usa valori neutri così non resta bloccato.
    let size_px = tech_dims
        .map(|(w, h)| {
            ((f.bbox[2] * w as f32).min(f.bbox[3] * h as f32))
                .round()
                .max(1.0) as u32
        })
        .unwrap_or(96);
    let qscore = combined_quality(f.quality, Some(0.5), f.frontality, Some(size_px));
    (0.5, size_px, qscore)
}

pub fn plan(
    store: Store,
    governor: MemoryGovernor,
    crops_dir: Option<PathBuf>,
    _params: &serde_json::Value,
) -> AppResult<JobPlan> {
    Ok(JobPlan {
        phases: vec![PhaseSpec::new(PHASE, 1.0)],
        min_memory_mb: 32,
        run: Box::new(move |ctx| {
            let phase = ctx.progress.phase(PHASE);
            let faces = pending(&store)?;
            phase.set_total(faces.len() as i64);
            phase.start();
            for chunk in faces.chunks(CHUNK) {
                if !ctx.control.checkpoint() {
                    return Ok(RunSummary {
                        cancelled: true,
                        failed: 0,
                    });
                }
                let mut measured = Vec::with_capacity(chunk.len());
                for f in chunk {
                    if !ctx.control.checkpoint() {
                        return Ok(RunSummary {
                            cancelled: true,
                            failed: 0,
                        });
                    }
                    let (sharp, size_px, qscore) =
                        measure_one(&store, &governor, crops_dir.as_deref(), f);
                    measured.push((f.id, sharp, size_px, qscore));
                }
                let n = measured.len() as i64;
                store.write(move |c| {
                    let mut up = c.prepare_cached(
                        "UPDATE face SET sharpness = ?2, size_px = ?3, quality_score = ?4 WHERE id = ?1",
                    )?;
                    for (id, sharp, size_px, qscore) in &measured {
                        up.execute(rusqlite::params![id, sharp, size_px, qscore])?;
                    }
                    Ok(())
                })?;
                phase.inc_done(n);
            }
            store.write(pc_store::people::refresh_centroids)?;
            phase.finish();
            Ok(RunSummary {
                cancelled: false,
                failed: 0,
            })
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};
    use std::sync::Arc;
    use std::time::Duration;

    struct Sink;
    impl pc_jobs::ProgressSink for Sink {
        fn on_progress(&self, _: &pc_jobs::JobSnapshot) {}
    }

    #[test]
    fn backfills_sharpness_and_size_for_existing_faces_without_models() {
        let tmp = tempfile::tempdir().unwrap();
        let img_path = tmp.path().join("photo.jpg");
        let img = RgbImage::from_fn(400, 400, |x, y| {
            let v = ((x / 4 + y / 4) % 2) as u8 * 180 + 30;
            Rgb([v, v, v])
        });
        img.save(&img_path).unwrap();

        let db = Store::open(&tmp.path().join("catalog.db"), Default::default()).unwrap();
        let p_str = img_path.display().to_string();
        db.write(move |c| {
            c.execute(
                "INSERT INTO media_item(id,path,name,ext,size,mtime,added_at,updated_at,status)
                 VALUES (1,?1,'photo.jpg','jpg',1000,1,1,1,'ok')",
                [&p_str],
            )?;
            c.execute(
                "INSERT INTO media_tech(media_id,width,height) VALUES (1,400,400)",
                [],
            )?;
            c.execute(
                "INSERT INTO face(id,media_id,bbox,quality,quality_score,model_ver)
                 VALUES (10,1,'[0.2,0.2,0.3,0.3]',0.92,0.92,'sface')",
                [],
            )?;
            Ok(())
        })
        .unwrap();

        let gov = MemoryGovernor::new(256);
        let jobs = JobManager::new(db.clone(), gov.clone(), 1, Arc::new(Sink));
        let st = db.clone();
        let gv = gov.clone();
        jobs.register(
            JOB_KIND,
            Arc::new(move |p| plan(st.clone(), gv.clone(), None, p)),
        );

        assert!(queue_once(&db, &jobs).unwrap().is_some());
        assert!(jobs.wait_idle(Duration::from_secs(20)));
        assert!(
            queue_once(&db, &jobs).unwrap().is_none(),
            "eseguito una volta sola"
        );

        let rec = &db.read(|c| pc_store::people::records(c, Some(1))).unwrap()[0];
        assert!(rec.sharpness.expect("nitidezza calcolata") > 0.5);
        assert_eq!(rec.size_px, Some(120));
        assert!(rec.quality_score > 0.5 && rec.quality_score <= 1.0);
    }
}
