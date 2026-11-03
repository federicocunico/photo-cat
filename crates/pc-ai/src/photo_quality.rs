//! Job `photo_quality` (change `system-catalogs`, issue #20): misura nitidezza e
//! risoluzione delle foto senza misura e sposta nel catalogo di sistema "Qualità
//! insufficiente" quelle molto sfocate o troppo piccole. La nitidezza si calcola
//! sulla miniatura già in cache, senza riaprire gli originali.

use crate::quality::{laplacian_variance_cropped, SHARPNESS_REF_VAR};
use image::RgbImage;
use pc_core::AppResult;
use pc_ingest::thumb::ThumbCache;
use pc_jobs::{JobPlan, PhaseSpec, RunSummary};
use pc_store::model::SYSTEM_LOW_QUALITY;
use pc_store::query::HIDDEN_BY_SYSTEM;
use pc_store::Store;

pub const JOB_KIND: &str = "photo_quality";
pub const PHASE: &str = "qualita_foto";
/// Nitidezza normalizzata sotto cui una foto è "molto sfocata" (scala di `quality::sharpness`).
pub const MIN_SHARPNESS: f32 = 0.2;
/// Risoluzione minima: sotto 0,3 megapixel la foto è troppo piccola ("sgranata").
pub const MIN_MEGAPIXELS: f32 = 0.3;
const CHUNK: usize = 128;

/// Nitidezza della foto in `0..=1` dalla varianza del laplaciano sulla miniatura.
pub fn photo_sharpness(img: &RgbImage) -> f32 {
    let var = laplacian_variance_cropped(img, 2);
    if !var.is_finite() || var <= 0.0 {
        return 0.0;
    }
    (var / (var + SHARPNESS_REF_VAR)).sqrt().clamp(0.0, 1.0)
}

/// Esito della misura: `true` se la foto va in "Qualità insufficiente".
pub fn is_insufficient(sharpness: Option<f32>, megapixels: Option<f32>) -> bool {
    sharpness.is_some_and(|s| s < MIN_SHARPNESS) || megapixels.is_some_and(|m| m < MIN_MEGAPIXELS)
}

struct Pending {
    id: i64,
    thumb: Option<String>,
    megapixels: Option<f32>,
}

/// Foto ancora senza misura (anche quelle nascoste: una foto ripristinata più tardi resta).
fn pending(store: &Store) -> AppResult<Vec<Pending>> {
    Ok(store.read(|c| {
        let mut q = c.prepare(
            "SELECT m.id, m.thumb, t.width, t.height
             FROM media_item m
             LEFT JOIN media_tech t ON t.media_id = m.id
             LEFT JOIN media_quality q ON q.media_id = m.id
             WHERE q.media_id IS NULL AND m.status = 'ok' AND m.kind = 'photo'
             ORDER BY m.id",
        )?;
        let rows = q
            .query_map([], |r| {
                let w: Option<i64> = r.get(2)?;
                let h: Option<i64> = r.get(3)?;
                Ok(Pending {
                    id: r.get(0)?,
                    thumb: r.get(1)?,
                    megapixels: match (w, h) {
                        (Some(w), Some(h)) if w > 0 && h > 0 => {
                            Some((w as f64 * h as f64 / 1_000_000.0) as f32)
                        }
                        _ => None,
                    },
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    })?)
}

fn measure(thumbs: &ThumbCache, p: &Pending) -> Option<f32> {
    let path = thumbs.path_of(p.thumb.as_deref()?)?;
    let img = image::open(path).ok()?.to_rgb8();
    Some(photo_sharpness(&img))
}

pub fn plan(store: Store, thumbs: ThumbCache, _params: &serde_json::Value) -> AppResult<JobPlan> {
    Ok(JobPlan {
        phases: vec![PhaseSpec::new(PHASE, 1.0)],
        min_memory_mb: 32,
        run: Box::new(move |ctx| {
            let phase = ctx.progress.phase(PHASE);
            let photos = pending(&store)?;
            phase.set_total(photos.len() as i64);
            phase.start();
            for chunk in photos.chunks(CHUNK) {
                if !ctx.control.checkpoint() {
                    return Ok(RunSummary {
                        cancelled: true,
                        failed: 0,
                    });
                }
                let measured: Vec<(i64, Option<f32>, Option<f32>)> = chunk
                    .iter()
                    .map(|p| (p.id, measure(&thumbs, p), p.megapixels))
                    .collect();
                let n = measured.len() as i64;
                store.write(move |c| {
                    let now = pc_store::now_ms();
                    let mut up = c.prepare_cached(
                        "INSERT OR REPLACE INTO media_quality(media_id, sharpness, megapixels, measured_at)
                         VALUES (?1, ?2, ?3, ?4)",
                    )?;
                    let mut bad = Vec::new();
                    for (id, sharp, mp) in &measured {
                        up.execute(rusqlite::params![id, sharp, mp, now])?;
                        if is_insufficient(*sharp, *mp) {
                            bad.push(*id);
                        }
                    }
                    drop(up);
                    if bad.is_empty() {
                        return Ok(());
                    }
                    // Solo le foto ancora visibili: quelle già nascoste restano dove sono.
                    let visible: Vec<i64> = {
                        let sql = format!(
                            "SELECT m.id FROM media_item m WHERE m.id IN (SELECT value FROM json_each(?1)) AND {HIDDEN_BY_SYSTEM}"
                        );
                        let mut st = c.prepare(&sql)?;
                        let rows = st
                            .query_map([serde_json::to_string(&bad)?], |r| r.get(0))?
                            .collect::<Result<_, _>>()?;
                        rows
                    };
                    let target = pc_store::repo::catalog::system_id(c, SYSTEM_LOW_QUALITY)?;
                    pc_store::repo::catalog::add_items(c, target, &visible)?;
                    Ok(())
                })?;
                phase.inc_done(n);
            }
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
    use image::{imageops, Rgb};

    /// Trama fine e irregolare, come il dettaglio di una foto a fuoco.
    fn textured(w: u32, h: u32) -> RgbImage {
        let mut seed = 12345u32;
        RgbImage::from_fn(w, h, |_, _| {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
            let v = (seed >> 16) as u8;
            Rgb([v, v, v])
        })
    }

    #[test]
    fn sharp_and_blurred_thumbnails_are_told_apart() {
        let sharp = textured(320, 240);
        let blurred = imageops::blur(&sharp, 6.0);
        let s = photo_sharpness(&sharp);
        let b = photo_sharpness(&blurred);
        assert!(s > MIN_SHARPNESS * 2.0, "nitida: {s}");
        assert!(b < MIN_SHARPNESS, "sfocata: {b}");
    }

    struct Sink;
    impl pc_jobs::ProgressSink for Sink {
        fn on_progress(&self, _: &pc_jobs::JobSnapshot) {}
    }

    #[test]
    fn the_job_moves_blurry_photos_and_respects_restores() {
        use pc_store::model::NewMedia;
        use std::sync::Arc;
        use std::time::Duration;
        let tmp = tempfile::tempdir().unwrap();
        let thumbs = ThumbCache::new(tmp.path().join("thumbs"));
        let sharp = textured(320, 240);
        let blurred = imageops::blur(&sharp, 6.0);
        for (key, img) in [("aaaaaaaa01", &sharp), ("bbbbbbbb02", &blurred)] {
            let p = thumbs.path_of(key).unwrap();
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            img.save(&p).unwrap();
        }
        let db = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
        let (good, bad) = db
            .write(|c| {
                let mut ids = Vec::new();
                for (name, key) in [("good.jpg", "aaaaaaaa01"), ("bad.jpg", "bbbbbbbb02")] {
                    let id = pc_store::repo::media::upsert(
                        c,
                        &NewMedia {
                            path: format!("D:/Foto/{name}"),
                            name: name.into(),
                            ..Default::default()
                        },
                    )?;
                    c.execute(
                        "UPDATE media_item SET thumb = ?2 WHERE id = ?1",
                        rusqlite::params![id, key],
                    )?;
                    c.execute(
                        "INSERT INTO media_tech(media_id, width, height) VALUES (?1, 4000, 3000)",
                        [id],
                    )?;
                    ids.push(id);
                }
                Ok((ids[0], ids[1]))
            })
            .unwrap();
        let gov = pc_governor::MemoryGovernor::new(256);
        let jobs = pc_jobs::JobManager::new(db.clone(), gov, 1, Arc::new(Sink));
        let (st, th) = (db.clone(), thumbs.clone());
        jobs.register(JOB_KIND, Arc::new(move |p| plan(st.clone(), th.clone(), p)));
        let low = db
            .read(|c| pc_store::repo::catalog::system_id(c, SYSTEM_LOW_QUALITY))
            .unwrap();
        jobs.submit(JOB_KIND, serde_json::json!({})).unwrap();
        assert!(jobs.wait_idle(Duration::from_secs(20)));
        let moved = db
            .read(|c| pc_store::repo::catalog::manual_items(c, low))
            .unwrap();
        assert_eq!(moved, vec![bad], "solo la foto sfocata: {good} resta");

        // L'utente la ripristina: un nuovo controllo la lascia in libreria.
        db.write(move |c| pc_store::bulk::restore_from_system(c, low, &[bad]).map(|_| ()))
            .unwrap();
        jobs.submit(JOB_KIND, serde_json::json!({})).unwrap();
        assert!(jobs.wait_idle(Duration::from_secs(20)));
        assert!(db
            .read(|c| pc_store::repo::catalog::manual_items(c, low))
            .unwrap()
            .is_empty());
    }

    #[test]
    fn small_or_blurry_photos_are_insufficient() {
        assert!(is_insufficient(Some(0.05), Some(12.0)));
        assert!(is_insufficient(Some(0.9), Some(0.2)));
        assert!(!is_insufficient(Some(0.9), Some(12.0)));
        // Misura mancante (miniatura assente): decide solo la risoluzione.
        assert!(!is_insufficient(None, None));
    }
}
