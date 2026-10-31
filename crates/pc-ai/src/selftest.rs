//! Self-test dei modelli su immagini di riferimento incluse nell'app
//! (`assets/selftest`): verifica che l'output sia nei limiti attesi e misura
//! il tempo medio di inferenza.

use crate::geo::Gazetteer;
use crate::{clip, faces, persons, tensor};
use pc_core::{AppError, AppResult};
use pc_models::runtime::{self, SessionOptions};
use pc_models::store::SelfTestResult;
use pc_models::{Entry, Role};
use std::path::{Path, PathBuf};
use std::time::Instant;

pub const LANDSCAPE: &str = "landscape_mountain.jpg";
pub const GROUP4: &str = "group_4_people.jpg";
const RUNS: u32 = 3;

/// Punti di riferimento di due volti di `group_4_people.jpg` (1280×1600), da YuNet.
const FACE_A: [(f32, f32); 5] = [
    (643.0, 332.0),
    (688.0, 333.0),
    (666.0, 359.0),
    (647.0, 380.0),
    (686.0, 381.0),
];
const FACE_B: [(f32, f32); 5] = [
    (329.0, 453.0),
    (380.0, 452.0),
    (356.0, 480.0),
    (332.0, 510.0),
    (377.0, 505.0),
];

pub struct SelfTestEnv<'a> {
    /// Cartella delle immagini di riferimento.
    pub images: PathBuf,
    pub opts: SessionOptions,
    /// Percorso verificato di un modello per ruolo (es. il tokenizer per l'encoder di testo).
    pub resolve: &'a dyn Fn(Role) -> Option<PathBuf>,
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn img(env: &SelfTestEnv, name: &str) -> AppResult<image::RgbImage> {
    Ok(image::open(env.images.join(name))
        .map_err(|e| AppError::internal(format!("immagine di riferimento {name}: {e}")))?
        .to_rgb8())
}

/// Esegue `f` più volte e restituisce l'ultimo risultato con il tempo medio.
fn timed<T>(mut f: impl FnMut() -> AppResult<T>) -> AppResult<(T, f64)> {
    let start = Instant::now();
    let mut last = f()?;
    for _ in 1..RUNS {
        last = f()?;
    }
    Ok((last, start.elapsed().as_secs_f64() * 1000.0 / RUNS as f64))
}

fn check(cond: bool, msg: String) -> AppResult<()> {
    if cond {
        Ok(())
    } else {
        Err(AppError::new(pc_core::ErrorCode::SelfTestFailed, msg))
    }
}

fn finite(v: &[f32]) -> bool {
    !v.is_empty()
        && v.iter().all(|x| x.is_finite())
        && (v.iter().map(|x| x * x).sum::<f32>() - 1.0).abs() < 0.01
}

fn run(entry: &Entry, path: &Path, env: &SelfTestEnv) -> AppResult<(f64, String)> {
    check(
        (entry.preprocessing, entry.postprocessing) == entry.role.processing(),
        "pre/post-processing incompatibile con il ruolo".into(),
    )?;
    check(
        pc_models::download::sha256_file(path)?.eq_ignore_ascii_case(&entry.sha256),
        "SHA-256 del modello non corrispondente al manifest".into(),
    )?;
    let mut opts = env.opts.clone();
    if !entry.gpu {
        opts.preferred = pc_core::config::ExecutionProvider::Cpu;
    }
    let session = || runtime::load(path, &opts).map(|(s, _)| s);
    match entry.role {
        Role::ClassificationImage => {
            let mut s = session()?;
            let (land, group) = (img(env, LANDSCAPE)?, img(env, GROUP4)?);
            let (e, ms) = timed(|| clip::embed_images(&mut s, &[&land, &group]))?;
            check(
                e.len() == 2 && e.iter().all(|v| v.len() == clip::DIM && finite(v)),
                "embedding non validi".into(),
            )?;
            let c = tensor::cosine(&e[0], &e[1]);
            check(c < 0.95, format!("immagini diverse troppo simili ({c:.3})"))?;
            Ok((ms, format!("embedding {} dimensioni", clip::DIM)))
        }
        Role::ClassificationText => {
            let tok_path = (env.resolve)(Role::Tokenizer).ok_or_else(|| {
                AppError::new(
                    pc_core::ErrorCode::ModelUnavailable,
                    "tokenizer CLIP mancante",
                )
            })?;
            let tok = clip::ClipTokenizer::load(&tok_path)?;
            let mut s = session()?;
            let texts = [
                "a photo of a snowy mountain",
                "a portrait of a group of people",
            ];
            let (e, ms) = timed(|| clip::embed_texts(&mut s, &tok, &texts))?;
            check(
                e.len() == texts.len() && e.iter().all(|v| v.len() == clip::DIM && finite(v)),
                "embedding non validi".into(),
            )?;
            let c = tensor::cosine(&e[0], &e[1]);
            check(c < 0.98, format!("testi diversi troppo simili ({c:.3})"))?;
            Ok((ms, format!("embedding {} dimensioni", clip::DIM)))
        }
        Role::Tokenizer => {
            let start = Instant::now();
            let tok = clip::ClipTokenizer::load(path)?;
            let ids = tok.encode("a photo of a mountain")?;
            check(ids.len() >= 4, format!("tokenizzazione inattesa: {ids:?}"))?;
            Ok((
                start.elapsed().as_secs_f64() * 1000.0,
                format!("{} token", ids.len()),
            ))
        }
        Role::PersonDetection => {
            let mut s = session()?;
            let (land, group) = (img(env, LANDSCAPE)?, img(env, GROUP4)?);
            let (n, ms) = timed(|| persons::detect(&mut s, &group, 0.4).map(|v| v.len()))?;
            let none = persons::detect(&mut s, &land, 0.4)?.len();
            check(
                (3..=5).contains(&n) && none == 0,
                format!("persone rilevate {n} (attese 4) e {none} (attese 0)"),
            )?;
            Ok((ms, format!("{n} persone rilevate")))
        }
        Role::FaceDetection => {
            let mut s = session()?;
            let (land, group) = (img(env, LANDSCAPE)?, img(env, GROUP4)?);
            let (n, ms) = timed(|| faces::detect(&mut s, &group, 0.7).map(|v| v.len()))?;
            let none = faces::detect(&mut s, &land, 0.7)?.len();
            check(
                (3..=5).contains(&n) && none == 0,
                format!("volti rilevati {n} (attesi 4) e {none} (attesi 0)"),
            )?;
            Ok((ms, format!("{n} volti rilevati")))
        }
        Role::FaceEmbedding => {
            let mut s = session()?;
            let group = img(env, GROUP4)?;
            let (a, b) = (faces::align(&group, &FACE_A), faces::align(&group, &FACE_B));
            let small =
                image::imageops::resize(&group, 640, 800, image::imageops::FilterType::Triangle);
            let half: [(f32, f32); 5] = FACE_A.map(|(x, y)| (x / 2.0, y / 2.0));
            let a2 = faces::align(&small, &half);
            let ((ea, eb, ea2), ms) = timed(|| {
                Ok((
                    faces::embed(&mut s, &a)?,
                    faces::embed(&mut s, &b)?,
                    faces::embed(&mut s, &a2)?,
                ))
            })?;
            let same = tensor::cosine(&ea, &ea2);
            let diff = tensor::cosine(&ea, &eb);
            check(
                [ea.as_slice(), eb.as_slice(), ea2.as_slice()]
                    .iter()
                    .all(|v| v.len() == 128 && finite(v))
                    && same > faces::SAME_PERSON
                    && diff < faces::SAME_PERSON,
                format!("somiglianza stesso volto {same:.2}, volti diversi {diff:.2}"),
            )?;
            Ok((
                ms,
                format!("stesso volto {same:.2} · volti diversi {diff:.2}"),
            ))
        }
        Role::Gazetteer => {
            let start = Instant::now();
            let g = Gazetteer::load(path)?;
            let near = g.nearest(43.77, 11.25, 30.0).map(|(c, _)| c.name.clone());
            check(
                g.len() > 10_000 && near.as_deref() == Some("Firenze"),
                format!("{} città, Firenze → {near:?}", g.len()),
            )?;
            Ok((
                start.elapsed().as_secs_f64() * 1000.0,
                format!("{} città", g.len()),
            ))
        }
    }
}

/// Self-test di un modello: non va mai in panico, l'errore diventa un test fallito.
pub fn self_test(entry: &Entry, path: &Path, env: &SelfTestEnv) -> SelfTestResult {
    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(entry, path, env)))
        .unwrap_or_else(|_| Err(AppError::internal("panic durante il self-test")));
    match res {
        Ok((avg_ms, message)) => {
            tracing::info!(
                model = entry.id.as_str(),
                avg_ms,
                message = message.as_str(),
                "self-test superato"
            );
            SelfTestResult {
                passed: true,
                avg_ms,
                message,
                at: now_ms(),
            }
        }
        Err(e) => {
            tracing::error!(
                code = "SELF_TEST_FAILED",
                phase = "modelli",
                path = %path.display(),
                detail = e.detail.as_str(),
                model = entry.id.as_str(),
                "self-test del modello non superato"
            );
            SelfTestResult {
                passed: false,
                avg_ms: 0.0,
                message: e.detail,
                at: now_ms(),
            }
        }
    }
}
