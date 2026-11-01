//! Adattatori AI per la pipeline di import; ogni risultato arriva al writer unico.
use crate::{classify, persons, taxonomy::Taxonomy, text_cache::Concept};
use pc_core::{AppError, AppResult, ErrorCode};
use pc_governor::MemoryGovernor;
use pc_ingest::import::{ExtraWrite, ImportItem, ImportStage};
use pc_jobs::{PhaseSpec, StageCtx, StageOutcome};
use pc_models::{pool::ModelPool, store::ModelStore, Entry, Role};
use pc_store::analysis::{self, AutoLabel};
use std::sync::Arc;

pub fn model_key(entry: &Entry) -> String {
    format!("{}:{}", entry.version, entry.sha256)
}

#[derive(Clone)]
pub struct AiResources {
    pub models: Arc<ModelStore>,
    pub pool: Arc<ModelPool>,
    pub governor: MemoryGovernor,
}

impl AiResources {
    pub fn ready(&self, ctx: &StageCtx) -> bool {
        while self.governor.ai_paused() {
            if !ctx.control.checkpoint() {
                return false;
            }
            self.governor
                .wait_ai_resumed(std::time::Duration::from_millis(100));
        }
        ctx.control.checkpoint()
    }
    pub fn entry(&self, role: Role) -> AppResult<&Entry> {
        self.models.manifest().by_role(role).ok_or_else(|| {
            AppError::new(
                ErrorCode::ModelUnavailable,
                format!("modello {role:?} mancante"),
            )
        })
    }
}

pub struct ClipStage {
    pub resources: AiResources,
    pub taxonomy: Arc<Taxonomy>,
    pub concepts: Arc<Vec<Concept>>,
}

struct ClassificationWrite {
    groups: Vec<String>,
    labels: Vec<AutoLabel>,
    embedding: Vec<f32>,
    model: String,
    taxonomy: i64,
}
impl ExtraWrite for ClassificationWrite {
    fn write(&self, c: &rusqlite::Connection, id: i64) -> pc_store::StoreResult<()> {
        analysis::classification(
            c,
            id,
            &self.groups,
            &self.labels,
            &self.embedding,
            &self.model,
            self.taxonomy,
        )
    }
}

impl ImportStage for ClipStage {
    fn prepare(&self) -> AppResult<()> {
        self.resources
            .pool
            .get(&self.resources.entry(Role::ClassificationImage)?.id)?;
        Ok(())
    }
    fn phase(&self) -> PhaseSpec {
        PhaseSpec::new("clip", 4.0)
    }
    fn workers(&self) -> usize {
        1
    }
    fn process(&self, item: &mut ImportItem, ctx: &StageCtx) -> StageOutcome {
        let Some(decoded) = &item.decoded else {
            return StageOutcome::Continue;
        };
        if !self.resources.ready(ctx) {
            return StageOutcome::Continue;
        }
        let run = || -> AppResult<ClassificationWrite> {
            let mut results = classify::images(
                &[&decoded.image],
                &self.taxonomy,
                &self.concepts,
                &self.resources.models,
                &self.resources.pool,
                &self.resources.governor,
            )?;
            let result = results.remove(0);
            Ok(ClassificationWrite {
                groups: self.taxonomy.groups.iter().map(|g| g.id.clone()).collect(),
                labels: result.labels,
                embedding: result.embedding,
                model: model_key(self.resources.entry(Role::ClassificationImage)?),
                taxonomy: self.taxonomy.version,
            })
        };
        match run() {
            Ok(result) => {
                item.extra.push(Box::new(result));
                StageOutcome::Continue
            }
            Err(e) => StageOutcome::ContinueWithError(e),
        }
    }
}

/// Rilevamento persone: salva i riquadri dal pavimento in su e conta quelli
/// sopra la soglia configurata.
pub struct PersonsStage {
    pub resources: AiResources,
    pub count_threshold: f32,
}
struct PersonsWrite {
    boxes: Vec<[f32; 5]>,
    model: String,
    count_threshold: f32,
}
impl ExtraWrite for PersonsWrite {
    fn write(&self, c: &rusqlite::Connection, id: i64) -> pc_store::StoreResult<()> {
        analysis::people_with(
            c,
            id,
            &self.boxes,
            &self.model,
            self.count_threshold,
            Some(crate::analyze::PERSON_FLOOR),
        )
    }
}

pub fn normalized_box(
    b: &crate::tensor::BBox,
    score: f32,
    width: u32,
    height: u32,
) -> Option<[f32; 5]> {
    if [b.x, b.y, b.w, b.h, score].iter().any(|x| !x.is_finite()) {
        return None;
    }
    let (w, h) = (width.max(1) as f32, height.max(1) as f32);
    let (x, y) = (b.x.clamp(0.0, w), b.y.clamp(0.0, h));
    let (right, bottom) = ((b.x + b.w).clamp(0.0, w), (b.y + b.h).clamp(0.0, h));
    (right > x && bottom > y).then(|| {
        [
            x / w,
            y / h,
            (right - x) / w,
            (bottom - y) / h,
            score.clamp(0.0, 1.0),
        ]
    })
}

impl ImportStage for PersonsStage {
    fn prepare(&self) -> AppResult<()> {
        self.resources
            .pool
            .get(&self.resources.entry(Role::PersonDetection)?.id)?;
        Ok(())
    }
    fn phase(&self) -> PhaseSpec {
        PhaseSpec::new("persons", 2.0)
    }
    fn workers(&self) -> usize {
        1
    }
    fn process(&self, item: &mut ImportItem, ctx: &StageCtx) -> StageOutcome {
        let Some(decoded) = &item.decoded else {
            return StageOutcome::Continue;
        };
        if !self.resources.ready(ctx) {
            return StageOutcome::Continue;
        }
        let run = || -> AppResult<PersonsWrite> {
            let entry = self.resources.entry(Role::PersonDetection)?;
            let model = self.resources.pool.get(&entry.id)?;
            let _permit = self.resources.governor.try_acquire(32).ok_or_else(|| {
                AppError::new(
                    ErrorCode::InsufficientMemory,
                    "memoria insufficiente per il rilevamento persone",
                )
            })?;
            let found = persons::detect(
                &mut model.session.lock(),
                &decoded.image,
                crate::analyze::PERSON_FLOOR,
            )?;
            let boxes = found
                .iter()
                .filter_map(|p| {
                    normalized_box(
                        &p.bbox,
                        p.score,
                        decoded.image.width(),
                        decoded.image.height(),
                    )
                })
                .collect();
            Ok(PersonsWrite {
                boxes,
                model: model_key(entry),
                count_threshold: self.count_threshold,
            })
        };
        match run() {
            Ok(result) => {
                item.extra.push(Box::new(result));
                StageOutcome::Continue
            }
            Err(e) => StageOutcome::ContinueWithError(e),
        }
    }
}

pub struct GeoStage {
    pub gazetteer: Option<Arc<crate::geo::Gazetteer>>,
}
struct LocationWrite(pc_store::model::Location);
impl ExtraWrite for LocationWrite {
    fn write(&self, c: &rusqlite::Connection, id: i64) -> pc_store::StoreResult<()> {
        pc_store::repo::meta::set_location(c, id, &self.0)?;
        pc_store::repo::fts::refresh(c, id)
    }
}
impl ImportStage for GeoStage {
    fn needs_image(&self) -> bool {
        false
    }
    fn phase(&self) -> PhaseSpec {
        PhaseSpec::new("geo", 0.5)
    }
    fn workers(&self) -> usize {
        1
    }
    fn process(&self, item: &mut ImportItem, _: &StageCtx) -> StageOutcome {
        use pc_store::model::{Location, LocationSource};
        let gps = item
            .exif
            .as_ref()
            .and_then(|e| Some((e.info.gps_lat?, e.info.gps_lon?)))
            .filter(|(lat, lon)| {
                lat.is_finite()
                    && lon.is_finite()
                    && (-90.0..=90.0).contains(lat)
                    && (-180.0..=180.0).contains(lon)
            });
        let location = if let Some((lat, lon)) = gps {
            let city = self
                .gazetteer
                .as_ref()
                .and_then(|g| g.nearest(lat, lon, 100.0))
                .map(|(c, _)| c);
            Some(Location {
                name: city.map(|c| c.name.clone()),
                country: city.map(|c| c.country.clone()),
                lat: Some(lat),
                lon: Some(lon),
                source: LocationSource::Gps,
            })
        } else {
            crate::geo::place_from_path(&item.found.path, self.gazetteer.as_deref()).map(|name| {
                Location {
                    name: Some(name),
                    country: None,
                    lat: None,
                    lon: None,
                    source: LocationSource::Directory,
                }
            })
        };
        if let Some(l) = location {
            item.extra.push(Box::new(LocationWrite(l)));
        }
        StageOutcome::Continue
    }
}
