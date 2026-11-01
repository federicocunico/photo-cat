//! Decodifica ridotta e miniature.
//!
//! Per i JPEG si usa lo scaling DCT del decoder (1/2, 1/4, 1/8): l'immagine
//! viene decodificata direttamente alla dimensione utile, con memoria ridotta
//! di 4–64 volte rispetto alla piena risoluzione. Ogni decodifica chiede prima
//! al governor un permesso pari al costo stimato.

use crate::formats::{support, MetaOnlyKind, Support};
use image::{imageops, DynamicImage, RgbImage};
use pc_core::{AppError, AppResult, ErrorCode};
use pc_governor::budget::decode_cost_mb;
use pc_governor::{MemoryGovernor, Permit};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// Lato lungo delle miniature in cache.
pub const THUMB_SIZE: u32 = 512;

/// Immagine decodificata (già ridotta) con il permesso di memoria che la copre.
pub struct Decoded {
    pub image: RgbImage,
    /// Dimensioni originali del file.
    pub source_size: (u32, u32),
    pub permit: Permit,
}

/// Decodifica `path` in modo che il lato lungo sia almeno `min_long_side` (se l'originale lo consente).
pub fn decode_reduced(
    path: &Path,
    ext: &str,
    min_long_side: u32,
    gov: &MemoryGovernor,
) -> AppResult<Decoded> {
    match support(ext) {
        Some(Support::Decodable(image::ImageFormat::Jpeg)) => decode_jpeg(path, min_long_side, gov),
        Some(Support::Decodable(format)) => decode_generic(path, format, min_long_side, gov),
        Some(Support::MetadataOnly(MetaOnlyKind::Raw)) => {
            let preview = embedded_preview(path).ok_or_else(|| {
                AppError::new(
                    ErrorCode::UnsupportedFormat,
                    "RAW senza anteprima incorporata",
                )
            })?;
            decode_jpeg_bytes(&preview, min_long_side, gov)
        }
        #[cfg(feature = "heif")]
        Some(Support::MetadataOnly(MetaOnlyKind::Heif)) => {
            crate::heif::decode(path, min_long_side, gov)
        }
        #[cfg(not(feature = "heif"))]
        Some(Support::MetadataOnly(MetaOnlyKind::Heif)) => Err(AppError::new(
            ErrorCode::UnsupportedFormat,
            format!("decodifica {} non disponibile", ext.to_ascii_uppercase()),
        )),
        None => Err(AppError::new(
            ErrorCode::UnsupportedFormat,
            format!("estensione .{ext}"),
        )),
    }
}

fn decode_jpeg(path: &Path, min_long_side: u32, gov: &MemoryGovernor) -> AppResult<Decoded> {
    let file = BufReader::new(File::open(path)?);
    decode_jpeg_reader(file, min_long_side, gov)
}

fn decode_jpeg_bytes(bytes: &[u8], min_long_side: u32, gov: &MemoryGovernor) -> AppResult<Decoded> {
    decode_jpeg_reader(std::io::Cursor::new(bytes), min_long_side, gov)
}

fn decode_jpeg_reader<R: std::io::Read>(
    reader: R,
    min_long_side: u32,
    gov: &MemoryGovernor,
) -> AppResult<Decoded> {
    let bad = |e: jpeg_decoder::Error| AppError::new(ErrorCode::DecodeFailed, e.to_string());
    let mut dec = jpeg_decoder::Decoder::new(reader);
    dec.read_info().map_err(bad)?;
    let info = dec
        .info()
        .ok_or_else(|| AppError::new(ErrorCode::DecodeFailed, "intestazione JPEG mancante"))?;
    let (w, h) = (u32::from(info.width), u32::from(info.height));
    let long = w.max(h).max(1);
    let target = min_long_side.min(long);
    let req_w = (u64::from(w) * u64::from(target)).div_ceil(u64::from(long)) as u16;
    let req_h = (u64::from(h) * u64::from(target)).div_ceil(u64::from(long)) as u16;
    let (sw, sh) = dec.scale(req_w.max(1), req_h.max(1)).map_err(bad)?;
    let scale = (w / u32::from(sw).max(1)).max(1);
    let permit = gov.acquire(decode_cost_mb(w, h, scale));
    let pixels = dec.decode().map_err(bad)?;
    let info = dec.info().expect("info dopo decode");
    let (ow, oh) = (u32::from(info.width), u32::from(info.height));
    debug_assert_eq!((ow, oh), (u32::from(sw), u32::from(sh)));
    let image = match info.pixel_format {
        jpeg_decoder::PixelFormat::RGB24 => RgbImage::from_raw(ow, oh, pixels),
        jpeg_decoder::PixelFormat::L8 => image::GrayImage::from_raw(ow, oh, pixels)
            .map(|g| DynamicImage::ImageLuma8(g).to_rgb8()),
        jpeg_decoder::PixelFormat::L16 => {
            let v: Vec<u16> = pixels
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| u16::from_be_bytes(*c))
                .collect();
            image::ImageBuffer::<image::Luma<u16>, _>::from_raw(ow, oh, v)
                .map(|g| DynamicImage::ImageLuma16(g).to_rgb8())
        }
        jpeg_decoder::PixelFormat::CMYK32 => {
            let rgb: Vec<u8> = pixels
                .as_chunks::<4>()
                .0
                .iter()
                .flat_map(|p| {
                    // I JPEG CMYK di Adobe sono invertiti.
                    let k = 255 - u32::from(p[3]);
                    [p[0], p[1], p[2]].map(|c| ((255 - u32::from(c)) * k / 255) as u8)
                })
                .collect();
            RgbImage::from_raw(ow, oh, rgb)
        }
    }
    .ok_or_else(|| AppError::new(ErrorCode::DecodeFailed, "buffer JPEG incoerente"))?;
    Ok(Decoded {
        image,
        source_size: (w, h),
        permit,
    })
}

fn decode_generic(
    path: &Path,
    format: image::ImageFormat,
    min_long_side: u32,
    gov: &MemoryGovernor,
) -> AppResult<Decoded> {
    let reader = image::ImageReader::with_format(BufReader::new(File::open(path)?), format);
    let reader = {
        let mut r = reader;
        r.no_limits();
        r
    };
    let (w, h) = image::ImageReader::open(path)?
        .with_guessed_format()?
        .into_dimensions()
        .map_err(|e| AppError::new(ErrorCode::DecodeFailed, e.to_string()))?;
    let permit = gov.acquire(decode_cost_mb(w, h, 1));
    let img = reader
        .decode()
        .map_err(|e| AppError::new(ErrorCode::DecodeFailed, e.to_string()))?;
    let long = w.max(h).max(1);
    let rgb = if long > min_long_side.saturating_mul(2) {
        let f = f64::from(min_long_side) / f64::from(long);
        let (nw, nh) = (
            ((f64::from(w) * f).round() as u32).max(1),
            ((f64::from(h) * f).round() as u32).max(1),
        );
        imageops::thumbnail(&img.to_rgb8(), nw, nh)
    } else {
        img.to_rgb8()
    };
    Ok(Decoded {
        image: rgb,
        source_size: (w, h),
        permit,
    })
}

/// Anteprima JPEG incorporata nei RAW basati su TIFF (IFD1).
pub fn embedded_preview(path: &Path) -> Option<Vec<u8>> {
    let mut file = File::open(path).ok()?;
    let mut header = [0u8; 96];
    let n = file.read(&mut header).ok()?;
    if header[..n].starts_with(b"FUJIFILMCCD-RAW") {
        if n < 92 {
            return None;
        }
        let off = u32::from_be_bytes(header[84..88].try_into().ok()?) as u64;
        let len = u32::from_be_bytes(header[88..92].try_into().ok()?) as u64;
        if off < 96
            || !(4..=64 * 1024 * 1024).contains(&len)
            || off.checked_add(len)? > file.metadata().ok()?.len()
        {
            return None;
        }
        file.seek(SeekFrom::Start(off)).ok()?;
        let mut bytes = vec![0; len as usize];
        file.read_exact(&mut bytes).ok()?;
        return bytes.starts_with(&[0xff, 0xd8]).then_some(bytes);
    }
    file.rewind().ok()?;
    let mut reader = BufReader::new(file);
    let exif = exif::Reader::new().read_from_container(&mut reader).ok()?;
    let off = exif
        .get_field(exif::Tag::JPEGInterchangeFormat, exif::In::THUMBNAIL)?
        .value
        .get_uint(0)? as usize;
    let len = exif
        .get_field(exif::Tag::JPEGInterchangeFormatLength, exif::In::THUMBNAIL)?
        .value
        .get_uint(0)? as usize;
    exif.buf()
        .get(off..off.checked_add(len)?)
        .map(<[u8]>::to_vec)
}

/// Applica l'orientamento EXIF (1–8).
pub fn apply_orientation(img: RgbImage, orientation: Option<u32>) -> RgbImage {
    match orientation.unwrap_or(1) {
        2 => imageops::flip_horizontal(&img),
        3 => imageops::rotate180(&img),
        4 => imageops::flip_vertical(&img),
        5 => imageops::flip_horizontal(&imageops::rotate90(&img)),
        6 => imageops::rotate90(&img),
        7 => imageops::flip_horizontal(&imageops::rotate270(&img)),
        8 => imageops::rotate270(&img),
        _ => img,
    }
}

/// Riduce al lato lungo indicato (mai ingrandisce).
pub fn fit(img: &RgbImage, max_side: u32) -> RgbImage {
    let (w, h) = img.dimensions();
    let long = w.max(h);
    if long <= max_side {
        return img.clone();
    }
    let f = f64::from(max_side) / f64::from(long);
    imageops::thumbnail(
        img,
        ((f64::from(w) * f).round() as u32).max(1),
        ((f64::from(h) * f).round() as u32).max(1),
    )
}

/// Cache delle miniature su disco: `<radice>/<ab>/<chiave>.jpg`.
#[derive(Debug, Clone)]
pub struct ThumbCache {
    root: PathBuf,
}

impl ThumbCache {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Percorso della miniatura; `None` se la chiave non è valida (solo esadecimale).
    pub fn path_of(&self, key: &str) -> Option<PathBuf> {
        if key.len() < 8 || !key.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        Some(self.root.join(&key[..2]).join(format!("{key}.jpg")))
    }

    pub fn contains(&self, key: &str) -> bool {
        self.path_of(key).is_some_and(|p| p.is_file())
    }

    /// Cancella le miniature indicate. Gli errori di file system vengono solo
    /// registrati in diagnostica: la rimozione dalla libreria è già avvenuta.
    pub fn remove(&self, keys: &[String]) -> usize {
        let mut n = 0;
        for key in keys {
            let Some(path) = self.path_of(key) else {
                continue;
            };
            match std::fs::remove_file(&path) {
                Ok(()) => n += 1,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => tracing::warn!(
                    code = "IO",
                    path = %path.display(),
                    detail = %e,
                    "miniatura non cancellata"
                ),
            }
        }
        n
    }

    /// Cancella le miniature che nessun elemento usa (chiavi assenti da `used`)
    /// scritte prima di `before`.
    pub fn purge_unreferenced(
        &self,
        used: &std::collections::HashSet<String>,
        before: std::time::SystemTime,
    ) -> AppResult<usize> {
        if !self.root.is_dir() {
            return Ok(0);
        }
        let mut stale = Vec::new();
        for entry in walkdir::WalkDir::new(&self.root).min_depth(2).max_depth(2) {
            let entry = entry.map_err(|e| AppError::new(ErrorCode::Io, e.to_string()))?;
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "jpg") {
                continue;
            }
            let recent = entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .is_none_or(|t| t >= before);
            if recent {
                continue;
            }
            if let Some(key) = path.file_stem().and_then(|s| s.to_str()) {
                if self.path_of(key).is_some() && !used.contains(key) {
                    stale.push(key.to_string());
                }
            }
        }
        Ok(self.remove(&stale))
    }

    /// Scrive la miniatura in modo atomico.
    pub fn store(&self, key: &str, img: &RgbImage) -> AppResult<PathBuf> {
        let path = self
            .path_of(key)
            .ok_or_else(|| AppError::invalid_input("chiave miniatura non valida"))?;
        std::fs::create_dir_all(path.parent().expect("sottocartella"))?;
        // File temporaneo unico: file identici (duplicati) possono arrivare in parallelo.
        static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let tmp = path.with_extension(format!("{}-{n}.tmp", std::process::id()));
        let written = (|| -> AppResult<()> {
            let mut out = std::io::BufWriter::new(File::create(&tmp)?);
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 82)
                .encode_image(img)
                .map_err(|e| AppError::new(ErrorCode::Io, e.to_string()))?;
            Ok(())
        })();
        if let Err(e) = written {
            let _ = std::fs::remove_file(&tmp);
            return Err(e);
        }
        if let Err(e) = std::fs::rename(&tmp, &path) {
            let _ = std::fs::remove_file(&tmp);
            // Un altro worker ha già scritto la stessa miniatura: va bene così.
            if !path.is_file() {
                return Err(e.into());
            }
        }
        Ok(path)
    }
}

/// Decodifica, orienta, riduce e salva la miniatura.
pub fn make_thumbnail(
    path: &Path,
    ext: &str,
    orientation: Option<u32>,
    key: &str,
    cache: &ThumbCache,
    gov: &MemoryGovernor,
) -> AppResult<(u32, u32)> {
    let decoded = decode_reduced(path, ext, THUMB_SIZE, gov)?;
    let thumb = fit(&apply_orientation(decoded.image, orientation), THUMB_SIZE);
    cache.store(key, &thumb)?;
    Ok(thumb.dimensions())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::tests::{jpeg_with_exif, short};

    #[test]
    fn orientation_6_rotates_landscape_to_portrait() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("r.jpg");
        std::fs::write(
            &p,
            jpeg_with_exif(1200, 800, &[short(exif::Tag::Orientation, 6)]),
        )
        .unwrap();
        let cache = ThumbCache::new(tmp.path().join("thumbs"));
        let gov = MemoryGovernor::new(512);
        let (w, h) = make_thumbnail(&p, "jpg", Some(6), "abcdef0123456789", &cache, &gov).unwrap();
        assert!(h > w, "{w}x{h}");
        assert_eq!(h, 512);
        let stored = image::open(cache.path_of("abcdef0123456789").unwrap()).unwrap();
        assert_eq!((stored.width(), stored.height()), (w, h));
        assert_eq!(gov.snapshot().in_use_mb, 0, "permesso restituito");
    }

    #[test]
    fn jpeg_is_decoded_with_dct_reduction() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("big.jpg");
        std::fs::write(&p, jpeg_with_exif(4096, 2048, &[])).unwrap();
        let gov = MemoryGovernor::new(4096);
        let d = decode_reduced(&p, "jpg", 512, &gov).unwrap();
        assert_eq!(d.source_size, (4096, 2048));
        assert_eq!(d.image.dimensions(), (512, 256), "scala 1/8");
        assert_eq!(d.permit.mb(), decode_cost_mb(4096, 2048, 8));
    }

    #[test]
    fn invalid_heic_is_reported_and_raw_uses_embedded_preview() {
        let tmp = tempfile::tempdir().unwrap();
        let gov = MemoryGovernor::new(256);
        let heic = tmp.path().join("a.heic");
        std::fs::write(&heic, b"....ftypheic").unwrap();
        let code = decode_reduced(&heic, "heic", 512, &gov).err().unwrap().code;
        #[cfg(not(feature = "heif"))]
        assert_eq!(code, ErrorCode::UnsupportedFormat);
        #[cfg(feature = "heif")]
        assert!(matches!(
            code,
            ErrorCode::DecodeFailed | ErrorCode::UnsupportedFormat
        ));

        // "RAW" TIFF con anteprima JPEG nell'IFD1.
        let preview = jpeg_with_exif(160, 120, &[]);
        let make = crate::metadata::tests::ascii(exif::Tag::Make, "NIKON");
        let mut w = exif::experimental::Writer::new();
        w.push_field(&make);
        w.set_jpeg(&preview, exif::In::THUMBNAIL);
        let mut buf = std::io::Cursor::new(Vec::new());
        w.write(&mut buf, true).unwrap();
        let raw = tmp.path().join("b.nef");
        std::fs::write(&raw, buf.into_inner()).unwrap();
        assert_eq!(embedded_preview(&raw).unwrap(), preview);
        let d = decode_reduced(&raw, "nef", 512, &gov).unwrap();
        assert_eq!(d.image.dimensions(), (160, 120));
    }

    #[test]
    fn concurrent_store_of_same_key_succeeds() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = ThumbCache::new(tmp.path());
        let img = RgbImage::from_pixel(64, 64, image::Rgb([1, 2, 3]));
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let (c, i) = (cache.clone(), img.clone());
                std::thread::spawn(move || c.store("aabbccddeeff0011", &i).map(|_| ()))
            })
            .collect();
        for h in handles {
            h.join().unwrap().unwrap();
        }
        assert!(cache.contains("aabbccddeeff0011"));
        let leftovers = std::fs::read_dir(tmp.path().join("aa")).unwrap().count();
        assert_eq!(leftovers, 1, "nessun file temporaneo residuo");
    }

    #[test]
    fn cache_rejects_unsafe_keys() {
        let c = ThumbCache::new("x");
        assert!(c.path_of("../../etc/passwd").is_none());
        assert!(c.path_of("abc").is_none());
        assert!(c.path_of("0123456789abcdef").is_some());
    }
}
