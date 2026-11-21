//! Anteprime JPEG ridotte e cache dedicata.
//!
//! Le anteprime sono generate una sola volta per (file, dimensione) e conservate in
//! `<data_dir>/raw-review/previews/`. Un indice SQLite proprio (file separato dal database
//! della libreria) tiene dimensione e ultimo accesso di ogni file, così la cache resta sotto
//! il limite configurato eliminando le anteprime usate meno di recente.
//!
//! Decodifica: la stessa strada delle miniature della libreria (`pc_ingest::thumb`), cioè
//! scaling DCT del decoder JPEG (1/2, 1/4, 1/8) e, per i RAF, il JPEG incorporato. Con la
//! feature `simd-jpeg` i JPEG si decodificano invece con turbojpeg, come in FujiView.
//! Il percorso senza feature è quello predefinito della release.
//!
//! L'orientamento EXIF viene applicato ai pixel: i file in cache non hanno EXIF.

use blake3::Hasher;
use pc_core::{AppError, AppResult, ErrorCode};
use pc_governor::MemoryGovernor;
use pc_ingest::thumb::{apply_orientation, decode_reduced, decode_reduced_with, embedded_preview};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{BufReader, Cursor};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

/// Limite predefinito della cache: 20 GB.
pub const DEFAULT_CACHE_LIMIT_BYTES: u64 = 20 * 1024 * 1024 * 1024;
/// Lato lungo minimo richiedibile.
pub const MIN_EDGE: u32 = 384;
/// Lato lungo massimo richiedibile.
pub const MAX_EDGE: u32 = 8192;
/// Lato lungo dell'anteprima grande, riusata per ricavare le miniature.
pub const LARGE_EDGE: u32 = 2048;
/// Qualità JPEG delle anteprime in cache.
const JPEG_QUALITY: u8 = 90;
/// Filtro di riduzione: buon compromesso tra nitidezza e velocità.
const RESIZE_ALG: fast_image_resize::ResizeAlg =
    fast_image_resize::ResizeAlg::Convolution(fast_image_resize::FilterType::CatmullRom);
/// Cambia quando cambia il modo di generare le anteprime: invalida tutta la cache.
const CACHE_VERSION: &str = "raw-review-preview-v1";
/// Memoria concessa alle decodifiche in parallelo (MB).
const DECODE_BUDGET_MB: u64 = 2048;

/// Anteprima pronta in cache.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct PreviewResult {
    pub cache_path: String,
    pub width: u32,
    pub height: u32,
    /// `true` se l'anteprima era già in cache.
    pub cached: bool,
}

/// Cache delle anteprime. Si può condividere tra thread (`Arc<PreviewCache>`).
pub struct PreviewCache {
    dir: PathBuf,
    limit_bytes: u64,
    db: Mutex<Connection>,
    governor: MemoryGovernor,
    /// Una serratura per anteprima in corso: le richieste doppie aspettano la prima.
    in_flight: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

fn db_error(e: rusqlite::Error) -> AppError {
    AppError::new(ErrorCode::Database, format!("indice anteprime: {e}"))
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    // Un thread andato in panic non deve bloccare la cache: i dati protetti restano coerenti.
    m.lock().unwrap_or_else(|p| p.into_inner())
}

impl PreviewCache {
    /// Apre (o crea) la cache in `<data_dir>/raw-review/previews/` con il limite in byte.
    pub fn open(data_dir: &Path, limit_bytes: u64) -> AppResult<Self> {
        let dir = data_dir.join("raw-review").join("previews");
        fs::create_dir_all(&dir)?;
        let db = Connection::open(dir.join("index.sqlite")).map_err(db_error)?;
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS cache_entries(
                key TEXT PRIMARY KEY,
                path TEXT NOT NULL,
                bytes INTEGER NOT NULL,
                last_access INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS cache_entries_access ON cache_entries(last_access);",
        )
        .map_err(db_error)?;
        Ok(Self {
            dir,
            limit_bytes,
            db: Mutex::new(db),
            governor: MemoryGovernor::new(DECODE_BUDGET_MB),
            in_flight: Mutex::new(HashMap::new()),
        })
    }

    /// Cartella dei file in cache.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Chiave della cache: cambia se il file sorgente cambia (percorso, dimensione, data di
    /// modifica) o se cambia la dimensione richiesta.
    pub fn key(source: &Path, max_edge: u32) -> AppResult<String> {
        let meta = fs::metadata(source)?;
        let modified = meta
            .modified()
            .unwrap_or(UNIX_EPOCH)
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let mut hasher = Hasher::new();
        hasher.update(
            format!(
                "{CACHE_VERSION}|{}|{}|{modified}|{max_edge}",
                source.to_string_lossy(),
                meta.len()
            )
            .as_bytes(),
        );
        Ok(hasher.finalize().to_hex().to_string())
    }

    fn file_for(&self, key: &str) -> PathBuf {
        self.dir.join(format!("{key}.jpg"))
    }

    /// Anteprima di `source` con lato lungo al più `max_edge` (limitato a 384–8192).
    /// Se esiste già in cache la restituisce; altrimenti la genera, una sola volta anche con
    /// richieste doppie contemporanee.
    pub fn get(&self, source: &Path, max_edge: u32) -> AppResult<PreviewResult> {
        let max_edge = max_edge.clamp(MIN_EDGE, MAX_EDGE);
        let key = Self::key(source, max_edge)?;
        let dest = self.file_for(&key);
        if let Some(hit) = self.hit(&key, &dest) {
            return Ok(hit);
        }
        let gate = Arc::clone(lock(&self.in_flight).entry(key.clone()).or_default());
        let result = {
            let _turn = lock(&gate);
            // Un'altra richiesta può aver finito mentre questa aspettava.
            match self.hit(&key, &dest) {
                Some(hit) => Ok(hit),
                None => self.build_and_register(source, &key, &dest, max_edge),
            }
        };
        let mut map = lock(&self.in_flight);
        // Restano solo la mappa e questo thread: nessun altro in attesa, la voce si può togliere.
        if Arc::strong_count(&gate) <= 2 {
            map.remove(&key);
        }
        result
    }

    fn hit(&self, key: &str, dest: &Path) -> Option<PreviewResult> {
        if !dest.is_file() {
            return None;
        }
        let (width, height) = image::image_dimensions(dest).ok()?;
        self.touch(key, dest);
        Some(PreviewResult {
            cache_path: dest.to_string_lossy().into_owned(),
            width,
            height,
            cached: true,
        })
    }

    fn build_and_register(
        &self,
        source: &Path,
        key: &str,
        dest: &Path,
        max_edge: u32,
    ) -> AppResult<PreviewResult> {
        // Per le miniature si parte dall'anteprima grande già in cache, se c'è.
        let larger = (max_edge < LARGE_EDGE)
            .then(|| Self::key(source, LARGE_EDGE).ok())
            .flatten()
            .map(|k| self.file_for(&k))
            .filter(|p| p.is_file());
        let (width, height) = match larger {
            Some(big) => build_preview(&big, dest, max_edge, &self.governor, true)
                .or_else(|_| build_preview(source, dest, max_edge, &self.governor, false))?,
            None => build_preview(source, dest, max_edge, &self.governor, false)?,
        };
        self.touch(key, dest);
        Ok(PreviewResult {
            cache_path: dest.to_string_lossy().into_owned(),
            width,
            height,
            cached: false,
        })
    }

    /// Registra l'accesso e, se la cache supera il limite, elimina le anteprime meno recenti
    /// (mai quella appena usata).
    fn touch(&self, key: &str, path: &Path) {
        let Ok(meta) = fs::metadata(path) else {
            return;
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as i64;
        let db = lock(&self.db);
        let upsert = db.execute(
            "INSERT INTO cache_entries(key,path,bytes,last_access) VALUES(?1,?2,?3,?4)
             ON CONFLICT(key) DO UPDATE SET last_access=?4, bytes=?3, path=?2",
            params![key, path.to_string_lossy(), meta.len() as i64, now],
        );
        // Se l'indice non si aggiorna la cache funziona comunque: si salta l'eviction.
        if upsert.is_ok() {
            self.evict(&db, key);
        }
    }

    fn evict(&self, db: &Connection, keep: &str) {
        let limit = i64::try_from(self.limit_bytes).unwrap_or(i64::MAX);
        let total: i64 = db
            .query_row(
                "SELECT COALESCE(SUM(bytes),0) FROM cache_entries",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if total <= limit {
            return;
        }
        let victims: Vec<(String, String, i64)> = db
            .prepare(
                "SELECT key,path,bytes FROM cache_entries WHERE key<>?1
                 ORDER BY last_access ASC, rowid ASC",
            )
            .and_then(|mut st| {
                st.query_map([keep], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
                    .collect::<Result<Vec<_>, _>>()
            })
            .unwrap_or_default();
        let mut remaining = total;
        for (k, p, bytes) in victims {
            if remaining <= limit {
                break;
            }
            let _ = fs::remove_file(&p);
            if db
                .execute("DELETE FROM cache_entries WHERE key=?1", [&k])
                .is_ok()
            {
                remaining -= bytes;
            }
        }
    }

    /// Dimensione totale registrata nell'indice (byte).
    pub fn total_bytes(&self) -> u64 {
        let db = lock(&self.db);
        db.query_row(
            "SELECT COALESCE(SUM(bytes),0) FROM cache_entries",
            [],
            |r| r.get::<_, i64>(0),
        )
        .map(|v| v.max(0) as u64)
        .unwrap_or(0)
    }
}

fn extension_of(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

/// Orientamento EXIF da un file o da un buffer JPEG (1 se assente).
fn orientation_of<R: std::io::BufRead + std::io::Seek>(reader: &mut R) -> u32 {
    exif::Reader::new()
        .read_from_container(reader)
        .ok()
        .and_then(|d| {
            d.get_field(exif::Tag::Orientation, exif::In::PRIMARY)
                .and_then(|f| f.value.get_uint(0))
        })
        .unwrap_or(1)
}

/// Decodifica ridotta di `source` (non ancora orientata) con lato lungo almeno `max_edge`, più
/// l'orientamento EXIF da applicare. `baked`: sorgente già orientata (anteprima in cache).
fn decode_raw(
    source: &Path,
    max_edge: u32,
    gov: &MemoryGovernor,
    baked: bool,
) -> AppResult<(image::RgbImage, u32)> {
    let ext = extension_of(source);
    let (rgb, orientation) = if ext == "raf" {
        // Scatto RAF: l'anteprima è il JPEG incorporato.
        let jpeg = embedded_preview(source).ok_or_else(|| {
            AppError::new(
                ErrorCode::DecodeFailed,
                format!("anteprima incorporata assente: {}", source.display()),
            )
        })?;
        let orientation = orientation_of(&mut Cursor::new(&jpeg));
        let d = decode_reduced_with(source, &ext, Some(jpeg), max_edge, gov)?;
        (d.image, orientation)
    } else {
        let orientation = if baked {
            1
        } else {
            fs::File::open(source)
                .map(|f| orientation_of(&mut BufReader::new(f)))
                .unwrap_or(1)
        };
        (decode_plain(source, &ext, max_edge, gov)?, orientation)
    };
    Ok((rgb, orientation))
}

#[cfg(not(feature = "simd-jpeg"))]
fn decode_plain(
    source: &Path,
    ext: &str,
    max_edge: u32,
    gov: &MemoryGovernor,
) -> AppResult<image::RgbImage> {
    Ok(decode_reduced(source, ext, max_edge, gov)?.image)
}

/// Con `simd-jpeg` i JPEG si decodificano con turbojpeg (scaling DCT SIMD); gli altri
/// formati restano sul percorso comune.
#[cfg(feature = "simd-jpeg")]
fn decode_plain(
    source: &Path,
    ext: &str,
    max_edge: u32,
    gov: &MemoryGovernor,
) -> AppResult<image::RgbImage> {
    if !matches!(ext, "jpg" | "jpeg") {
        return Ok(decode_reduced(source, ext, max_edge, gov)?.image);
    }
    use turbojpeg::{Decompressor, Image, PixelFormat, ScalingFactor};
    let fail = |e: String| AppError::new(ErrorCode::DecodeFailed, format!("turbojpeg: {e}"));
    let jpeg = fs::read(source)?;
    let mut dec = Decompressor::new().map_err(|e| fail(e.to_string()))?;
    let header = dec.read_header(&jpeg).map_err(|e| fail(e.to_string()))?;
    let longest = header.width.max(header.height);
    let target = max_edge as usize;
    let target = target.min(longest);
    let scaling = if header.is_lossless {
        ScalingFactor::ONE
    } else {
        Decompressor::supported_scaling_factors()
            .into_iter()
            .filter(|f| f.scale(longest) >= target && f.scale(longest) <= longest)
            .min_by_key(|f| f.scale(longest))
            .unwrap_or(ScalingFactor::ONE)
    };
    dec.set_scaling_factor(scaling)
        .map_err(|e| fail(e.to_string()))?;
    dec.set_fast_upsample(true)
        .map_err(|e| fail(e.to_string()))?;
    let scaled = header.scaled(scaling);
    let pitch = scaled.width * 3;
    let mut out = Image {
        pixels: vec![0u8; pitch * scaled.height],
        width: scaled.width,
        pitch,
        height: scaled.height,
        format: PixelFormat::RGB,
    };
    dec.decompress(&jpeg, out.as_deref_mut())
        .map_err(|e| fail(e.to_string()))?;
    image::RgbImage::from_raw(scaled.width as u32, scaled.height as u32, out.pixels)
        .ok_or_else(|| fail("buffer non valido".into()))
}

/// Tempi delle fasi di una generazione (per il benchmark).
#[derive(Debug, Default, Clone, Copy)]
struct Phases {
    decode: std::time::Duration,
    resize: std::time::Duration,
    encode: std::time::Duration,
}

/// Riduce `rgb` a un lato lungo di `max_edge` con fast_image_resize (SIMD, parallelo).
fn shrink(rgb: image::RgbImage, max_edge: u32) -> AppResult<image::RgbImage> {
    use fast_image_resize as fir;
    let (w, h) = rgb.dimensions();
    let long = w.max(h);
    if long <= max_edge {
        return Ok(rgb);
    }
    let ratio = f64::from(max_edge) / f64::from(long);
    let nw = ((f64::from(w) * ratio).round() as u32).max(1);
    let nh = ((f64::from(h) * ratio).round() as u32).max(1);
    let fail = |e: String| AppError::new(ErrorCode::Internal, format!("ridimensionamento: {e}"));
    let src = fir::images::Image::from_vec_u8(w, h, rgb.into_raw(), fir::PixelType::U8x3)
        .map_err(|e| fail(e.to_string()))?;
    let mut dst = fir::images::Image::new(nw, nh, fir::PixelType::U8x3);
    fir::Resizer::new()
        .resize(
            &src,
            &mut dst,
            Some(&fir::ResizeOptions::new().resize_alg(RESIZE_ALG)),
        )
        .map_err(|e| fail(e.to_string()))?;
    image::RgbImage::from_raw(nw, nh, dst.into_vec())
        .ok_or_else(|| fail("buffer non valido".into()))
}

/// Genera l'anteprima in `dest` (scrittura su file temporaneo e rinomina). Restituisce le
/// dimensioni finali.
fn build_preview(
    source: &Path,
    dest: &Path,
    max_edge: u32,
    gov: &MemoryGovernor,
    baked: bool,
) -> AppResult<(u32, u32)> {
    build_preview_timed(source, dest, max_edge, gov, baked).map(|(size, _)| size)
}

fn build_preview_timed(
    source: &Path,
    dest: &Path,
    max_edge: u32,
    gov: &MemoryGovernor,
    baked: bool,
) -> AppResult<((u32, u32), Phases)> {
    let mut phases = Phases::default();
    let t = std::time::Instant::now();
    let (rgb, orientation) = decode_raw(source, max_edge, gov, baked)?;
    phases.decode = t.elapsed();
    let t = std::time::Instant::now();
    // Prima si riduce, poi si ruota: l'orientamento costa meno su meno pixel.
    let rgb = apply_orientation(shrink(rgb, max_edge)?, Some(orientation));
    phases.resize = t.elapsed();
    let t = std::time::Instant::now();
    let (ow, oh) = rgb.dimensions();
    let tmp = dest.with_extension(format!("{}.tmp", std::process::id()));
    let mut out = std::io::BufWriter::new(fs::File::create(&tmp)?);
    let encoded = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY)
        .encode_image(&rgb);
    let flushed = std::io::Write::flush(&mut out);
    drop(out);
    let done = match encoded {
        Err(e) => Err(AppError::new(
            ErrorCode::Io,
            format!("scrittura anteprima: {e}"),
        )),
        Ok(()) => flushed
            .map_err(AppError::from)
            .and_then(|()| fs::rename(&tmp, dest).map_err(AppError::from)),
    };
    if let Err(e) = done {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }
    phases.encode = t.elapsed();
    Ok(((ow, oh), phases))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    /// JPEG sintetico con gradiente e un po' di rumore (per tempi di decodifica realistici).
    fn jpeg_bytes(w: u32, h: u32, orientation: Option<u16>) -> Vec<u8> {
        let img = image::RgbImage::from_fn(w, h, |x, y| {
            let n = (x.wrapping_mul(2654435761) ^ y.wrapping_mul(40503)) >> 28;
            image::Rgb([
                ((x * 255 / w.max(1)) as u8).wrapping_add(n as u8),
                ((y * 255 / h.max(1)) as u8).wrapping_add(n as u8),
                ((x + y) % 256) as u8,
            ])
        });
        let mut out = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 85)
            .encode_image(&img)
            .unwrap();
        let Some(o) = orientation else {
            return out;
        };
        // APP1 EXIF minimale (big-endian) con il solo tag Orientation.
        let mut app1 = b"Exif\0\0MM\0*\0\0\0\x08\0\x01".to_vec();
        app1.extend_from_slice(&[0x01, 0x12, 0, 3, 0, 0, 0, 1]);
        app1.extend_from_slice(&o.to_be_bytes());
        app1.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
        let len = (app1.len() + 2) as u16;
        let mut v = out[..2].to_vec();
        v.extend_from_slice(&[0xff, 0xe1]);
        v.extend_from_slice(&len.to_be_bytes());
        v.extend_from_slice(&app1);
        v.extend_from_slice(&out[2..]);
        v
    }

    fn write_jpeg(dir: &Path, name: &str, w: u32, h: u32, o: Option<u16>) -> PathBuf {
        let p = dir.join(name);
        fs::write(&p, jpeg_bytes(w, h, o)).unwrap();
        p
    }

    fn cache(dir: &Path, limit: u64) -> PreviewCache {
        PreviewCache::open(dir, limit).unwrap()
    }

    #[test]
    fn preview_is_limited_and_keeps_aspect_ratio() {
        let t = tempfile::tempdir().unwrap();
        let src = write_jpeg(t.path(), "a.jpg", 3000, 2000, None);
        let c = cache(t.path(), DEFAULT_CACHE_LIMIT_BYTES);
        let r = c.get(&src, 1200).unwrap();
        assert!(!r.cached);
        assert_eq!((r.width, r.height), (1200, 800));
        assert_eq!(image::image_dimensions(&r.cache_path).unwrap(), (1200, 800));
        let again = c.get(&src, 1200).unwrap();
        assert!(again.cached);
        assert_eq!(again.cache_path, r.cache_path);
        // Mai ingrandita oltre l'originale.
        let small = write_jpeg(t.path(), "s.jpg", 500, 400, None);
        let r = c.get(&small, 2048).unwrap();
        assert_eq!((r.width, r.height), (500, 400));
    }

    #[test]
    fn exif_orientation_is_applied() {
        let t = tempfile::tempdir().unwrap();
        let src = write_jpeg(t.path(), "r.jpg", 900, 600, Some(6));
        let c = cache(t.path(), DEFAULT_CACHE_LIMIT_BYTES);
        let r = c.get(&src, 600).unwrap();
        // Ruotata di 90 gradi: da orizzontale a verticale.
        assert_eq!((r.width, r.height), (400, 600));
    }

    #[test]
    fn eviction_removes_least_recently_used() {
        let t = tempfile::tempdir().unwrap();
        let a = write_jpeg(t.path(), "a.jpg", 800, 600, None);
        let b = write_jpeg(t.path(), "b.jpg", 800, 600, None);
        let d = write_jpeg(t.path(), "d.jpg", 800, 600, None);
        // Cache senza vincoli per misurare la dimensione di un'anteprima.
        let probe_dir = tempfile::tempdir().unwrap();
        let probe = cache(probe_dir.path(), u64::MAX);
        let size = fs::metadata(probe.get(&a, 600).unwrap().cache_path)
            .unwrap()
            .len();
        let limit = size * 2 + size / 2;
        let c = cache(t.path(), limit);
        let ra = c.get(&a, 600).unwrap();
        std::thread::sleep(Duration::from_millis(5));
        let rb = c.get(&b, 600).unwrap();
        std::thread::sleep(Duration::from_millis(5));
        // `a` torna recente: `b` diventa la meno usata.
        assert!(c.get(&a, 600).unwrap().cached);
        std::thread::sleep(Duration::from_millis(5));
        let rd = c.get(&d, 600).unwrap();
        assert!(Path::new(&ra.cache_path).exists());
        assert!(!Path::new(&rb.cache_path).exists(), "b andava eliminata");
        assert!(Path::new(&rd.cache_path).exists());
        assert!(c.total_bytes() <= limit);
    }

    #[test]
    fn key_changes_when_source_changes() {
        let t = tempfile::tempdir().unwrap();
        let src = write_jpeg(t.path(), "a.jpg", 800, 600, None);
        let k1 = PreviewCache::key(&src, 600).unwrap();
        assert_eq!(k1, PreviewCache::key(&src, 600).unwrap());
        assert_ne!(k1, PreviewCache::key(&src, 800).unwrap());
        let c = cache(t.path(), DEFAULT_CACHE_LIMIT_BYTES);
        assert_eq!(c.get(&src, 600).unwrap().width, 600);
        fs::write(&src, jpeg_bytes(1000, 400, None)).unwrap();
        let k2 = PreviewCache::key(&src, 600).unwrap();
        assert_ne!(k1, k2);
        // La nuova versione produce una nuova anteprima, non quella vecchia.
        let r = c.get(&src, 600).unwrap();
        assert!(!r.cached);
        assert_eq!((r.width, r.height), (600, 240));
    }

    fn raf_with(jpeg: &[u8]) -> Vec<u8> {
        let mut raf = vec![0u8; 100];
        raf[..15].copy_from_slice(b"FUJIFILMCCD-RAW");
        raf[0x54..0x58].copy_from_slice(&100u32.to_be_bytes());
        raf[0x58..0x5C].copy_from_slice(&(jpeg.len() as u32).to_be_bytes());
        raf.extend_from_slice(jpeg);
        raf
    }

    #[test]
    fn preview_from_raf_embedded_jpeg_with_orientation() {
        let t = tempfile::tempdir().unwrap();
        let src = t.path().join("DSCF0001.RAF");
        fs::write(&src, raf_with(&jpeg_bytes(1500, 1000, Some(6)))).unwrap();
        let c = cache(t.path(), DEFAULT_CACHE_LIMIT_BYTES);
        let r = c.get(&src, 600).unwrap();
        assert_eq!((r.width, r.height), (400, 600));
        let no_preview = t.path().join("DSCF0002.RAF");
        fs::write(&no_preview, vec![0u8; 200]).unwrap();
        assert!(c.get(&no_preview, 600).is_err());
    }

    #[test]
    fn concurrent_duplicate_requests_build_one_file() {
        let t = tempfile::tempdir().unwrap();
        let src = write_jpeg(t.path(), "a.jpg", 3000, 2000, None);
        let c = Arc::new(cache(t.path(), DEFAULT_CACHE_LIMIT_BYTES));
        let fresh = AtomicUsize::new(0);
        std::thread::scope(|s| {
            for _ in 0..6 {
                s.spawn(|| {
                    let r = c.get(&src, 1000).unwrap();
                    if !r.cached {
                        fresh.fetch_add(1, Ordering::SeqCst);
                    }
                });
            }
        });
        assert_eq!(fresh.load(Ordering::SeqCst), 1, "una sola generazione");
        let files = fs::read_dir(c.dir())
            .unwrap()
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "jpg"))
            .count();
        assert_eq!(files, 1);
        assert!(lock(&c.in_flight).is_empty());
    }

    #[test]
    fn thumbnail_reuses_large_preview() {
        let t = tempfile::tempdir().unwrap();
        let src = write_jpeg(t.path(), "a.jpg", 3000, 2000, Some(6));
        let c = cache(t.path(), DEFAULT_CACHE_LIMIT_BYTES);
        let big = c.get(&src, LARGE_EDGE).unwrap();
        assert_eq!((big.width, big.height), (1365, 2048));
        // Si sostituisce la sorgente con byte illeggibili ma stessa dimensione e stessa data
        // di modifica (stessa chiave): la miniatura riesce solo se parte dall'anteprima 2K.
        let modified = fs::metadata(&src).unwrap().modified().unwrap();
        let len = fs::metadata(&src).unwrap().len() as usize;
        fs::write(&src, vec![0u8; len]).unwrap();
        fs::OpenOptions::new()
            .write(true)
            .open(&src)
            .unwrap()
            .set_modified(modified)
            .unwrap();
        let thumb = c.get(&src, 384).unwrap();
        assert!(!thumb.cached);
        // L'orientamento non è applicato due volte: resta verticale.
        assert_eq!((thumb.width, thumb.height), (257, 384));
    }

    /// JPEG "fotografico" 6000x4000 a qualità 95: gradienti, dettagli a media frequenza e
    /// rumore lieve (molto meno entropico del rumore puro).
    fn realistic_jpeg(w: u32, h: u32) -> Vec<u8> {
        let img = image::RgbImage::from_fn(w, h, |x, y| {
            let (fx, fy) = (f64::from(x) / f64::from(w), f64::from(y) / f64::from(h));
            let base = 128.0 + 70.0 * (fx * 3.1).sin() * (fy * 2.3).cos();
            let detail = 18.0 * (f64::from(x) * 0.045).sin() * (f64::from(y) * 0.037).cos()
                + 9.0 * (f64::from(x + 2 * y) * 0.11).sin();
            let n =
                f64::from(((x.wrapping_mul(2654435761) ^ y.wrapping_mul(40503)) >> 29) as u8) - 3.5;
            let v = base + detail + n;
            image::Rgb([
                (v + 30.0 * fx).clamp(0.0, 255.0) as u8,
                (v - 10.0 * fy).clamp(0.0, 255.0) as u8,
                (v + 20.0 * (1.0 - fx)).clamp(0.0, 255.0) as u8,
            ])
        });
        let mut out = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 95)
            .encode_image(&img)
            .unwrap();
        out
    }

    #[test]
    #[ignore = "benchmark manuale: FUJIVIEW_BENCH_IMAGE opzionale, altrimenti JPEG realistico 6000x4000 q95"]
    fn benchmark_real_preview_pipeline() {
        let t = tempfile::tempdir().unwrap();
        let source = match std::env::var("FUJIVIEW_BENCH_IMAGE") {
            Ok(p) => PathBuf::from(p),
            Err(_) => {
                let p = t.path().join("bench.jpg");
                fs::write(&p, realistic_jpeg(6000, 4000)).unwrap();
                p
            }
        };
        eprintln!("sorgente: {} byte", fs::metadata(&source).unwrap().len());
        let gov = MemoryGovernor::new(DECODE_BUDGET_MB);
        // Una passata di riscaldamento (pagine del file, thread pool).
        let warm = t.path().join("warm.jpg");
        build_preview(&source, &warm, 1024, &gov, false).unwrap();
        for (i, edge) in [LARGE_EDGE, 384, 4096].into_iter().enumerate() {
            let dest = t.path().join(format!("b{i}.jpg"));
            let started = Instant::now();
            let (size, p) = build_preview_timed(&source, &dest, edge, &gov, false).unwrap();
            eprintln!(
                "edge={edge} {size:?} totale={:?} decodifica={:?} resize+orienta={:?} encode={:?}",
                started.elapsed(),
                p.decode,
                p.resize,
                p.encode
            );
        }
        let c = cache(t.path(), DEFAULT_CACHE_LIMIT_BYTES);
        c.get(&source, LARGE_EDGE).unwrap();
        let started = Instant::now();
        let thumb = c.get(&source, 384).unwrap();
        eprintln!(
            "miniatura 384 dal 2K in cache: {:?} ({}x{})",
            started.elapsed(),
            thumb.width,
            thumb.height
        );
        let started = Instant::now();
        let r = c.get(&source, LARGE_EDGE).unwrap();
        eprintln!(
            "edge={LARGE_EDGE} da cache cached={} elapsed={:?}",
            r.cached,
            started.elapsed()
        );
    }
}
