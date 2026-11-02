//! Decodifica ridotta e miniature.
//!
//! Per i JPEG si usa lo scaling DCT del decoder (1/2, 1/4, 1/8): l'immagine
//! viene decodificata direttamente alla dimensione utile, con memoria ridotta
//! di 4–64 volte rispetto alla piena risoluzione. Ogni decodifica chiede prima
//! al governor un permesso pari al costo stimato.
//!
//! Se il decodificatore JPEG principale non legge una variante del formato, si riprova con
//! quello di `image` (zune-jpeg). Quando la decodifica fallisce, l'errore indica la causa
//! riconosciuta ([`DecodeCause`]): file troncato, danneggiato, variante non supportata,
//! anteprima RAW assente o immagine oltre i limiti.

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

/// Oltre questa dimensione un'immagine non viene decodificata (100 megapixel).
pub const MAX_PIXELS: u64 = 100_000_000;

/// Immagine decodificata (già ridotta) con il permesso di memoria che la copre.
pub struct Decoded {
    pub image: RgbImage,
    /// Dimensioni originali del file.
    pub source_size: (u32, u32),
    pub permit: Permit,
}

/// Causa riconosciuta di una decodifica fallita. Il codice d'errore resta `DECODE_FAILED`
/// (o `UNSUPPORTED_FORMAT`) per compatibilità; la causa è all'inizio del dettaglio
/// (`"<chiave>: <descrizione> — <messaggio del decodificatore>"`) e l'interfaccia la traduce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeCause {
    /// File interrotto prima della fine (copia incompleta, scheda rimossa).
    Truncated,
    /// Dati non validi.
    Corrupt,
    /// Variante del formato non supportata (es. JPEG aritmetico, 12 bit).
    UnsupportedVariant(String),
    /// RAW senza anteprima JPEG incorporata.
    NoRawPreview,
    /// Immagine oltre [`MAX_PIXELS`] o oltre la memoria concessa.
    TooLarge,
    /// Errore di lettura del file.
    Io,
}

impl DecodeCause {
    pub fn key(&self) -> &'static str {
        match self {
            Self::Truncated => "truncated",
            Self::Corrupt => "corrupt",
            Self::UnsupportedVariant(_) => "unsupported_variant",
            Self::NoRawPreview => "no_raw_preview",
            Self::TooLarge => "too_large",
            Self::Io => "io",
        }
    }

    pub fn describe(&self) -> &'static str {
        match self {
            Self::Truncated => "file danneggiato o incompleto",
            Self::Corrupt => "file danneggiato",
            Self::UnsupportedVariant(_) => "variante del formato non supportata",
            Self::NoRawPreview => "RAW senza anteprima incorporata",
            Self::TooLarge => "immagine oltre i limiti",
            Self::Io => "errore di lettura",
        }
    }

    pub fn code(&self) -> ErrorCode {
        match self {
            Self::NoRawPreview => ErrorCode::UnsupportedFormat,
            Self::Io => ErrorCode::Io,
            _ => ErrorCode::DecodeFailed,
        }
    }

    /// Causa di un errore prodotto da [`cause_error`], se riconoscibile.
    pub fn of(err: &AppError) -> Option<&'static str> {
        const KEYS: [&str; 6] = [
            "truncated",
            "corrupt",
            "unsupported_variant",
            "no_raw_preview",
            "too_large",
            "io",
        ];
        let head = err.detail.split(':').next()?;
        KEYS.into_iter().find(|k| *k == head)
    }

    /// Una variante non supportata o dati anomali possono essere letti dal decodificatore di
    /// riserva; un file troncato no (resterebbe un'immagine parziale senza errore).
    fn worth_fallback(&self) -> bool {
        matches!(self, Self::UnsupportedVariant(_) | Self::Corrupt)
    }
}

/// Errore di decodifica con la causa riconosciuta all'inizio del dettaglio.
pub fn cause_error(cause: DecodeCause, detail: impl std::fmt::Display) -> AppError {
    let variant = match &cause {
        DecodeCause::UnsupportedVariant(v) if !v.is_empty() => format!(" ({v})"),
        _ => String::new(),
    };
    AppError::new(
        cause.code(),
        format!("{}: {}{variant} — {detail}", cause.key(), cause.describe()),
    )
}

fn looks_truncated(msg: &str) -> bool {
    let m = msg.to_ascii_lowercase();
    [
        "eof",
        "end of file",
        "premature",
        "unexpected end",
        "truncated",
        "not enough data",
    ]
    .iter()
    .any(|k| m.contains(k))
}

fn io_cause(e: &std::io::Error) -> DecodeCause {
    if e.kind() == std::io::ErrorKind::UnexpectedEof {
        DecodeCause::Truncated
    } else {
        DecodeCause::Io
    }
}

fn jpeg_cause(e: &jpeg_decoder::Error) -> DecodeCause {
    match e {
        jpeg_decoder::Error::Unsupported(f) => DecodeCause::UnsupportedVariant(format!("{f:?}")),
        jpeg_decoder::Error::Io(io) => io_cause(io),
        jpeg_decoder::Error::Format(m) if looks_truncated(m) => DecodeCause::Truncated,
        _ => DecodeCause::Corrupt,
    }
}

fn image_cause(e: &image::ImageError) -> DecodeCause {
    match e {
        image::ImageError::Unsupported(u) => DecodeCause::UnsupportedVariant(u.to_string()),
        image::ImageError::Limits(_) => DecodeCause::TooLarge,
        image::ImageError::IoError(io) => io_cause(io),
        other if looks_truncated(&other.to_string()) => DecodeCause::Truncated,
        _ => DecodeCause::Corrupt,
    }
}

fn decode_error(e: image::ImageError) -> AppError {
    cause_error(image_cause(&e), e)
}

/// Decodifica `path` in modo che il lato lungo sia almeno `min_long_side` (se l'originale lo consente).
pub fn decode_reduced(
    path: &Path,
    ext: &str,
    min_long_side: u32,
    gov: &MemoryGovernor,
) -> AppResult<Decoded> {
    decode_reduced_with(path, ext, None, min_long_side, gov)
}

/// Come [`decode_reduced`], riusando l'anteprima RAW già estratta (`raw_preview`).
pub fn decode_reduced_with(
    path: &Path,
    ext: &str,
    raw_preview: Option<Vec<u8>>,
    min_long_side: u32,
    gov: &MemoryGovernor,
) -> AppResult<Decoded> {
    match support(ext) {
        Some(Support::Decodable(image::ImageFormat::Jpeg)) => decode_jpeg(path, min_long_side, gov),
        Some(Support::Decodable(format)) => decode_generic(path, format, min_long_side, gov),
        Some(Support::MetadataOnly(MetaOnlyKind::Raw)) => {
            let preview = raw_preview
                .or_else(|| embedded_preview(path))
                .ok_or_else(|| cause_error(DecodeCause::NoRawPreview, ext.to_ascii_uppercase()))?;
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
    match decode_jpeg_reader(file, min_long_side, gov) {
        Ok(d) => Ok(d),
        Err((cause, e)) if cause.worth_fallback() => {
            // Decodificatore di riserva (zune-jpeg tramite `image`): legge varianti che
            // jpeg-decoder rifiuta. Se fallisce anche questo, vale la causa del primo.
            tracing::debug!(path = %path.display(), detail = %e, "JPEG: decodificatore di riserva");
            let file = BufReader::new(File::open(path)?);
            decode_generic_reader(file, image::ImageFormat::Jpeg, min_long_side, gov)
                .map_err(|_| cause_error(cause, e))
        }
        Err((cause, e)) => Err(cause_error(cause, e)),
    }
}

fn decode_jpeg_bytes(bytes: &[u8], min_long_side: u32, gov: &MemoryGovernor) -> AppResult<Decoded> {
    decode_jpeg_reader(std::io::Cursor::new(bytes), min_long_side, gov)
        .map_err(|(cause, e)| cause_error(cause, e))
}

fn decode_jpeg_reader<R: std::io::Read>(
    reader: R,
    min_long_side: u32,
    gov: &MemoryGovernor,
) -> Result<Decoded, (DecodeCause, String)> {
    let bad = |e: jpeg_decoder::Error| (jpeg_cause(&e), e.to_string());
    let mut dec = jpeg_decoder::Decoder::new(reader);
    dec.read_info().map_err(bad)?;
    let info = dec.info().ok_or_else(|| {
        (
            DecodeCause::Corrupt,
            "intestazione JPEG mancante".to_string(),
        )
    })?;
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
    .ok_or_else(|| (DecodeCause::Corrupt, "buffer JPEG incoerente".to_string()))?;
    Ok(Decoded {
        image,
        source_size: (w, h),
        permit,
    })
}

/// Memoria per decodificare un'immagine con `image`: il buffer nel formato del file, la sua
/// conversione in RGB e la miniatura (in MB).
pub fn generic_cost_mb(total_bytes: u64, w: u32, h: u32) -> u64 {
    let rgb = u64::from(w) * u64::from(h) * 3;
    (total_bytes + rgb).div_ceil(1024 * 1024) + 1
}

fn decode_generic(
    path: &Path,
    format: image::ImageFormat,
    min_long_side: u32,
    gov: &MemoryGovernor,
) -> AppResult<Decoded> {
    decode_generic_reader(
        BufReader::new(File::open(path)?),
        format,
        min_long_side,
        gov,
    )
}

fn decode_generic_reader<R: std::io::BufRead + std::io::Seek>(
    reader: R,
    format: image::ImageFormat,
    min_long_side: u32,
    gov: &MemoryGovernor,
) -> AppResult<Decoded> {
    use image::ImageDecoder;
    // Un solo accesso al file: prima l'intestazione (dimensioni e formato dei pixel), poi i dati.
    let mut decoder = image::ImageReader::with_format(reader, format)
        .into_decoder()
        .map_err(decode_error)?;
    let (w, h) = decoder.dimensions();
    if u64::from(w) * u64::from(h) > MAX_PIXELS {
        return Err(cause_error(
            DecodeCause::TooLarge,
            format!("{w}×{h} pixel, oltre {} megapixel", MAX_PIXELS / 1_000_000),
        ));
    }
    let permit = gov.acquire(generic_cost_mb(decoder.total_bytes(), w, h));
    // Il decodificatore non può allocare più di quanto il governor ha concesso.
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(permit.mb() * 1024 * 1024);
    decoder.set_limits(limits).map_err(decode_error)?;
    let img = DynamicImage::from_decoder(decoder).map_err(decode_error)?;
    // `into_rgb8` non copia se l'immagine è già RGB a 8 bit.
    let rgb = img.into_rgb8();
    let long = w.max(h).max(1);
    let rgb = if long > min_long_side.saturating_mul(2) {
        let f = f64::from(min_long_side) / f64::from(long);
        let (nw, nh) = (
            ((f64::from(w) * f).round() as u32).max(1),
            ((f64::from(h) * f).round() as u32).max(1),
        );
        imageops::thumbnail(&rgb, nw, nh)
    } else {
        rgb
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

    /// PNG valido nell'intestazione ma con dimensioni enormi (i dati non servono: si rifiuta prima).
    fn huge_png_header(w: u32, h: u32) -> Vec<u8> {
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&w.to_be_bytes());
        ihdr.extend_from_slice(&h.to_be_bytes());
        ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
        let mut chunk = b"IHDR".to_vec();
        chunk.extend_from_slice(&ihdr);
        let crc = crc32(&chunk);
        let mut out = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        out.extend_from_slice(&(ihdr.len() as u32).to_be_bytes());
        out.extend_from_slice(&chunk);
        out.extend_from_slice(&crc.to_be_bytes());
        // Primo blocco di dati (vuoto): il decodificatore legge l'intestazione fin qui.
        out.extend_from_slice(&0u32.to_be_bytes());
        out.extend_from_slice(b"IDAT");
        out.extend_from_slice(&crc32(b"IDAT").to_be_bytes());
        out
    }

    fn crc32(data: &[u8]) -> u32 {
        let mut c = 0xFFFF_FFFFu32;
        for b in data {
            c ^= u32::from(*b);
            for _ in 0..8 {
                c = if c & 1 != 0 {
                    0xEDB8_8320 ^ (c >> 1)
                } else {
                    c >> 1
                };
            }
        }
        !c
    }

    #[test]
    fn huge_image_is_rejected_without_decoding_or_memory() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("huge.png");
        std::fs::write(&p, huge_png_header(30_000, 20_000)).unwrap();
        let gov = MemoryGovernor::new(512);
        let e = decode_reduced(&p, "png", 512, &gov).err().unwrap();
        assert_eq!(e.code, ErrorCode::DecodeFailed);
        assert_eq!(DecodeCause::of(&e), Some("too_large"));
        assert!(e.detail.contains("immagine oltre i limiti"), "{}", e.detail);
        assert_eq!(gov.snapshot().peak_mb, 0, "nessun permesso richiesto");
    }

    #[test]
    fn png_permit_covers_the_buffer_and_the_rgb_copy() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("a.png");
        image::RgbaImage::from_pixel(2000, 1500, image::Rgba([1, 2, 3, 255]))
            .save(&p)
            .unwrap();
        let gov = MemoryGovernor::new(4096);
        let d = decode_reduced(&p, "png", 512, &gov).unwrap();
        let rgba = 2000 * 1500 * 4;
        let rgb = 2000 * 1500 * 3;
        assert!(
            d.permit.mb() * 1024 * 1024 >= rgba + rgb,
            "{} MB",
            d.permit.mb()
        );
        assert_eq!(d.source_size, (2000, 1500));
        assert!(d.image.width() <= 1024);
    }

    #[test]
    fn truncated_jpeg_is_reported_as_damaged_not_unsupported() {
        let tmp = tempfile::tempdir().unwrap();
        let full = jpeg_with_exif(1200, 800, &[]);
        let p = tmp.path().join("cut.jpg");
        std::fs::write(&p, &full[..full.len() / 2]).unwrap();
        let gov = MemoryGovernor::new(512);
        match decode_reduced(&p, "jpg", 512, &gov) {
            Err(e) => {
                assert_eq!(e.code, ErrorCode::DecodeFailed);
                assert_eq!(DecodeCause::of(&e), Some("truncated"), "{}", e.detail);
                assert!(e.detail.contains("file danneggiato o incompleto"));
            }
            Ok(_) => panic!("un JPEG troncato deve essere segnalato"),
        }
    }

    #[test]
    fn cmyk_jpeg_gets_a_thumbnail_without_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let (w, h) = (640u16, 480u16);
        let data: Vec<u8> = (0..u32::from(w) * u32::from(h))
            .flat_map(|i| [(i % 200) as u8, 40, 90, 10])
            .collect();
        let p = tmp.path().join("cmyk.jpg");
        jpeg_encoder::Encoder::new_file(&p, 85)
            .unwrap()
            .encode(&data, w, h, jpeg_encoder::ColorType::Cmyk)
            .unwrap();
        let cache = ThumbCache::new(tmp.path().join("t"));
        let gov = MemoryGovernor::new(512);
        let (tw, th) = make_thumbnail(&p, "jpg", None, "00112233aabbccdd", &cache, &gov).unwrap();
        assert_eq!((tw, th), (512, 384));
    }

    #[test]
    fn fallback_decoder_reads_jpeg_when_the_primary_refuses_it() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("ok.jpg");
        std::fs::write(&p, jpeg_with_exif(800, 600, &[])).unwrap();
        let gov = MemoryGovernor::new(512);
        let d = decode_generic(&p, image::ImageFormat::Jpeg, 512, &gov).unwrap();
        assert_eq!(d.source_size, (800, 600));
        assert!(DecodeCause::UnsupportedVariant("x".into()).worth_fallback());
        assert!(!DecodeCause::Truncated.worth_fallback());
    }

    #[test]
    fn raw_without_preview_names_the_cause() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("x.nef");
        std::fs::write(&p, [b'I', b'I', 42, 0, 1, 2, 3]).unwrap();
        let e = decode_reduced(&p, "nef", 512, &MemoryGovernor::new(64))
            .err()
            .unwrap();
        assert_eq!(e.code, ErrorCode::UnsupportedFormat);
        assert_eq!(DecodeCause::of(&e), Some("no_raw_preview"));
    }
}
