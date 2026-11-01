//! Caratteristiche tecniche, EXIF e date di un file.

use crate::formats::{format_name, support, Support};
use chrono::{DateTime, Local, NaiveDateTime};
use exif::{In, Tag, Value};
use pc_core::{AppError, AppResult, ErrorCode};
use pc_store::model::{DateSource, ExifInfo, MediaDates, TechInfo};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

/// EXIF letto da un file, già interpretato.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExifMeta {
    pub info: ExifInfo,
    pub orientation: Option<u32>,
    pub date_original: Option<String>,
    pub date_create: Option<String>,
    pub dpi: Option<(f64, f64)>,
    pub pixel_size: Option<(u32, u32)>,
    pub color_space: Option<String>,
    /// Fuso dichiarato per `date_original` (OffsetTimeOriginal), in minuti.
    pub offset_original: Option<i32>,
    /// Fuso dichiarato per `date_create` (OffsetTimeDigitized, poi OffsetTime).
    pub offset_create: Option<i32>,
    /// Dispositivo normalizzato ("Apple iPhone 17 Pro Max", "Fujifilm X-T5").
    pub device: Option<String>,
}

/// `Ok(None)` = nessun EXIF; `Err(CORRUPT_EXIF)` = EXIF presente ma illeggibile.
pub fn read_exif(path: &Path) -> AppResult<Option<ExifMeta>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let parsed = if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("raf"))
    {
        match crate::thumb::embedded_preview(path) {
            Some(bytes) => {
                exif::Reader::new().read_from_container(&mut std::io::Cursor::new(bytes))
            }
            None => return Ok(None),
        }
    } else {
        exif::Reader::new().read_from_container(&mut reader)
    };
    let exif = match parsed {
        Ok(e) => e,
        Err(exif::Error::NotFound(_)) => return Ok(None),
        Err(exif::Error::Io(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
            return Err(AppError::new(ErrorCode::CorruptExif, "EXIF troncato"))
        }
        Err(exif::Error::Io(e)) => return Err(e.into()),
        Err(e) => return Err(AppError::new(ErrorCode::CorruptExif, e.to_string())),
    };
    let text = |tag: Tag| -> Option<String> {
        let f = exif.get_field(tag, In::PRIMARY)?;
        match &f.value {
            Value::Ascii(v) => v
                .first()
                .map(|b| {
                    String::from_utf8_lossy(b)
                        .trim_matches(char::from(0))
                        .trim()
                        .to_string()
                })
                .filter(|s| !s.is_empty()),
            _ => Some(f.display_value().to_string()),
        }
    };
    let rational = |tag: Tag| -> Option<f64> {
        match &exif.get_field(tag, In::PRIMARY)?.value {
            Value::Rational(v) => v.first().map(|r| r.to_f64()).filter(|x| x.is_finite()),
            Value::SRational(v) => v.first().map(|r| r.to_f64()).filter(|x| x.is_finite()),
            _ => None,
        }
    };
    let uint = |tag: Tag| -> Option<u32> { exif.get_field(tag, In::PRIMARY)?.value.get_uint(0) };

    let dpi = match (rational(Tag::XResolution), rational(Tag::YResolution)) {
        (Some(x), Some(y)) if x > 0.0 && y > 0.0 => {
            let factor = if uint(Tag::ResolutionUnit) == Some(3) {
                2.54
            } else {
                1.0
            };
            Some((x * factor, y * factor))
        }
        _ => None,
    };
    let gps = |tag: Tag, reference: Tag, negative: u8| -> Option<f64> {
        let f = exif.get_field(tag, In::PRIMARY)?;
        let Value::Rational(v) = &f.value else {
            return None;
        };
        if v.len() < 3 {
            return None;
        }
        let deg = v[0].to_f64() + v[1].to_f64() / 60.0 + v[2].to_f64() / 3600.0;
        let neg = match &exif.get_field(reference, In::PRIMARY)?.value {
            Value::Ascii(r) => r.first().and_then(|b| b.first()).copied() == Some(negative),
            _ => false,
        };
        deg.is_finite().then_some(if neg { -deg } else { deg })
    };
    let pixel_size = match (uint(Tag::PixelXDimension), uint(Tag::PixelYDimension)) {
        (Some(w), Some(h)) if w > 0 && h > 0 => Some((w, h)),
        _ => match (uint(Tag::ImageWidth), uint(Tag::ImageLength)) {
            (Some(w), Some(h)) if w > 0 && h > 0 => Some((w, h)),
            _ => None,
        },
    };
    let mut raw = serde_json::Map::new();
    for f in exif.fields() {
        if f.ifd_num != In::PRIMARY || matches!(f.tag, Tag::MakerNote | Tag::UserComment) {
            continue;
        }
        let v = f.display_value().with_unit(&exif).to_string();
        if v.len() <= 256 {
            raw.insert(f.tag.to_string(), serde_json::Value::String(v));
        }
    }
    let info = ExifInfo {
        camera_make: text(Tag::Make),
        camera_model: text(Tag::Model),
        lens: text(Tag::LensModel),
        exposure: exif
            .get_field(Tag::ExposureTime, In::PRIMARY)
            .map(|f| f.display_value().to_string()),
        f_number: rational(Tag::FNumber),
        iso: uint(Tag::PhotographicSensitivity).map(i64::from),
        focal_length: rational(Tag::FocalLength),
        gps_lat: gps(Tag::GPSLatitude, Tag::GPSLatitudeRef, b'S'),
        gps_lon: gps(Tag::GPSLongitude, Tag::GPSLongitudeRef, b'W'),
        raw: Some(serde_json::Value::Object(raw)),
    };
    let digitized = text(Tag::DateTimeDigitized).and_then(|s| normalize_exif_date(&s));
    let offset_create = if digitized.is_some() {
        text(Tag::OffsetTimeDigitized).and_then(|s| parse_offset(&s))
    } else {
        text(Tag::OffsetTime).and_then(|s| parse_offset(&s))
    };
    let device = normalize_device(info.camera_make.as_deref(), info.camera_model.as_deref());
    Ok(Some(ExifMeta {
        device,
        offset_original: text(Tag::OffsetTimeOriginal).and_then(|s| parse_offset(&s)),
        offset_create,
        info,
        orientation: uint(Tag::Orientation).filter(|o| (1..=8).contains(o)),
        date_original: text(Tag::DateTimeOriginal).and_then(|s| normalize_exif_date(&s)),
        date_create: digitized
            .or_else(|| text(Tag::DateTime).and_then(|s| normalize_exif_date(&s))),
        dpi,
        pixel_size,
        color_space: match uint(Tag::ColorSpace) {
            Some(1) => Some("sRGB".into()),
            Some(2) => Some("Adobe RGB".into()),
            _ => None,
        },
    }))
}

/// Fuso EXIF `±HH:MM` in minuti (`None` se assente o malformato).
pub fn parse_offset(s: &str) -> Option<i32> {
    let s = s.trim();
    let (sign, rest) = match s.as_bytes().first()? {
        b'+' => (1, &s[1..]),
        b'-' => (-1, &s[1..]),
        _ => return None,
    };
    let (h, m) = rest.split_once(':')?;
    if h.len() != 2 || m.len() != 2 {
        return None;
    }
    let (h, m): (i32, i32) = (h.parse().ok()?, m.parse().ok()?);
    if h > 14 || m > 59 || (h == 14 && m > 0) {
        return None;
    }
    Some(sign * (h * 60 + m))
}

/// Marca e modello in un nome di dispositivo leggibile: suffissi societari
/// tolti, maiuscole sistemate, marca non ripetuta se il modello la contiene già.
pub fn normalize_device(make: Option<&str>, model: Option<&str>) -> Option<String> {
    let clean = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    let make = make.map(clean).filter(|s| !s.is_empty()).map(|m| {
        let mut words: Vec<&str> = m.split(' ').collect();
        while words.len() > 1
            && matches!(
                words
                    .last()
                    .map(|w| w.trim_end_matches(['.', ',']).to_ascii_uppercase())
                    .as_deref(),
                Some(
                    "CORPORATION"
                        | "CORP"
                        | "CO"
                        | "LTD"
                        | "INC"
                        | "IMAGING"
                        | "COMPANY"
                        | "ELECTRONICS"
                )
            )
        {
            words.pop();
        }
        let joined = words.join(" ");
        let has_lower = joined.chars().any(|c| c.is_lowercase());
        let has_upper = joined.chars().any(|c| c.is_uppercase());
        if has_lower && has_upper {
            joined
        } else {
            // "FUJIFILM" → "Fujifilm", "samsung" → "Samsung".
            joined
                .split(' ')
                .map(|w| {
                    let mut c = w.chars();
                    match c.next() {
                        Some(f) => f
                            .to_uppercase()
                            .chain(c.flat_map(|x| x.to_lowercase()))
                            .collect(),
                        None => String::new(),
                    }
                })
                .collect::<Vec<String>>()
                .join(" ")
        }
    });
    let model = model.map(clean).filter(|s| !s.is_empty());
    match (make, model) {
        (Some(mk), Some(md)) => {
            if md.to_lowercase().starts_with(&mk.to_lowercase()) {
                let rest = md[mk.len()..].trim();
                Some(if rest.is_empty() {
                    mk
                } else {
                    format!("{mk} {rest}")
                })
            } else {
                Some(format!("{mk} {md}"))
            }
        }
        (Some(mk), None) => Some(mk),
        (None, Some(md)) => Some(md),
        (None, None) => None,
    }
}

/// `2023:07:14 10:22:01` → `2023-07-14 10:22:01` (scarta date non valide).
pub fn normalize_exif_date(s: &str) -> Option<String> {
    let s = s.trim();
    let dt = NaiveDateTime::parse_from_str(s, "%Y:%m:%d %H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S"))
        .ok()?;
    Some(dt.format("%Y-%m-%d %H:%M:%S").to_string())
}

fn fs_time(t: std::io::Result<std::time::SystemTime>) -> Option<String> {
    t.ok().map(|t| {
        DateTime::<Local>::from(t)
            .format("%Y-%m-%d %H:%M:%S")
            .to_string()
    })
}

/// Data di acquisizione (EXIF originale → EXIF creazione → file system) e di ultima modifica.
pub fn resolve_dates(meta: &std::fs::Metadata, exif: Option<&ExifMeta>) -> MediaDates {
    let modified = fs_time(meta.modified());
    let (captured_at, src, offset) = match exif {
        Some(ExifMeta {
            date_original: Some(d),
            offset_original,
            ..
        }) => (Some(d.clone()), DateSource::ExifOriginal, *offset_original),
        Some(ExifMeta {
            date_create: Some(d),
            offset_create,
            ..
        }) => (Some(d.clone()), DateSource::ExifCreate, *offset_create),
        _ => {
            // La data di creazione del file cambia con le copie: si prende la più vecchia.
            let created = fs_time(meta.created());
            let oldest = match (&created, &modified) {
                (Some(c), Some(m)) => Some(c.min(m).clone()),
                (c, m) => c.clone().or_else(|| m.clone()),
            };
            (oldest, DateSource::FileSystem, None)
        }
    };
    MediaDates {
        captured_source: captured_at.as_ref().map(|_| src),
        captured_at,
        modified_at: modified,
        declared_offset_min: offset,
        device: exif.and_then(|e| e.device.clone()),
    }
}

/// Risoluzione dall'intestazione JFIF (JPEG) o pHYs (PNG).
fn header_dpi(path: &Path, format: image::ImageFormat) -> Option<(f64, f64)> {
    let mut head = vec![0u8; 64 * 1024];
    let n = File::open(path).ok()?.read(&mut head).ok()?;
    let head = &head[..n];
    match format {
        image::ImageFormat::Jpeg => {
            let i = head.windows(5).position(|w| w == b"JFIF\0")?;
            let d = head.get(i + 7..i + 12)?;
            let (unit, x, y) = (
                d[0],
                u16::from_be_bytes([d[1], d[2]]) as f64,
                u16::from_be_bytes([d[3], d[4]]) as f64,
            );
            match unit {
                1 if x > 0.0 => Some((x, y)),
                2 if x > 0.0 => Some((x * 2.54, y * 2.54)),
                _ => None,
            }
        }
        image::ImageFormat::Png => {
            let i = head.windows(4).position(|w| w == b"pHYs")?;
            let d = head.get(i + 4..i + 13)?;
            let x = u32::from_be_bytes([d[0], d[1], d[2], d[3]]) as f64;
            let y = u32::from_be_bytes([d[4], d[5], d[6], d[7]]) as f64;
            (d[8] == 1 && x > 0.0).then(|| ((x * 0.0254).round(), (y * 0.0254).round()))
        }
        _ => None,
    }
}

/// Caratteristiche tecniche: dimensioni e profondità dall'intestazione (senza decodificare),
/// risoluzione da EXIF o intestazione, spazio colore da EXIF o profilo ICC.
pub fn read_tech(path: &Path, ext: &str, exif: Option<&ExifMeta>) -> AppResult<TechInfo> {
    let mut tech = TechInfo {
        format: Some(format_name(ext)),
        dpi_x: exif.and_then(|e| e.dpi).map(|d| d.0),
        dpi_y: exif.and_then(|e| e.dpi).map(|d| d.1),
        color_space: exif.and_then(|e| e.color_space.clone()),
        orientation: exif.and_then(|e| e.orientation),
        ..Default::default()
    };
    match support(ext) {
        Some(Support::Decodable(format)) => {
            use image::ImageDecoder;
            let reader = image::ImageReader::open(path)?.with_guessed_format()?;
            let mut dec = reader
                .into_decoder()
                .map_err(|e| AppError::new(ErrorCode::DecodeFailed, e.to_string()))?;
            let (w, h) = dec.dimensions();
            let ct = dec.color_type();
            tech.width = Some(w);
            tech.height = Some(h);
            tech.bit_depth =
                Some(u32::from(ct.bits_per_pixel()) / u32::from(ct.channel_count()).max(1));
            if tech.color_space.is_none() {
                if let Ok(Some(_)) = dec.icc_profile() {
                    tech.color_space = Some("ICC".into());
                }
            }
            if tech.dpi_x.is_none() {
                if let Some((x, y)) = header_dpi(path, format) {
                    tech.dpi_x = Some(x);
                    tech.dpi_y = Some(y);
                }
            }
        }
        Some(Support::MetadataOnly(_)) => {
            if let Some((w, h)) = exif.and_then(|e| e.pixel_size) {
                tech.width = Some(w);
                tech.height = Some(h);
            }
            if ext.eq_ignore_ascii_case("raf") {
                if let Some((w, h)) = crate::raf::dimensions(path) {
                    tech.width = Some(w);
                    tech.height = Some(h);
                }
            }
            if matches!(
                support(ext),
                Some(Support::MetadataOnly(crate::formats::MetaOnlyKind::Heif))
            ) {
                let dims = crate::heif::header_dimensions(path);
                #[cfg(feature = "heif")]
                let dims = crate::heif::dimensions(path).ok().or(dims);
                if let Some((w, h)) = dims {
                    tech.width = Some(w);
                    tech.height = Some(h);
                }
            }
        }
        None => {
            return Err(AppError::new(
                ErrorCode::UnsupportedFormat,
                format!("estensione .{ext}"),
            ))
        }
    }
    Ok(tech)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use exif::experimental::Writer;
    use exif::{Field, Rational};

    /// JPEG sintetico con EXIF (scritto con il writer di kamadak-exif) inserito come APP1.
    pub fn jpeg_with_exif(w: u32, h: u32, fields: &[Field]) -> Vec<u8> {
        let img = image::RgbImage::from_fn(w, h, |x, y| {
            image::Rgb([(x % 255) as u8, (y % 255) as u8, 90])
        });
        let mut jpeg = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 85)
            .encode_image(&img)
            .unwrap();
        if fields.is_empty() {
            return jpeg;
        }
        let mut writer = Writer::new();
        for f in fields {
            writer.push_field(f);
        }
        let mut tiff = std::io::Cursor::new(Vec::new());
        writer.write(&mut tiff, false).unwrap();
        let tiff = tiff.into_inner();
        let mut app1 = vec![0xFF, 0xE1];
        app1.extend_from_slice(&((tiff.len() + 8) as u16).to_be_bytes());
        app1.extend_from_slice(b"Exif\0\0");
        app1.extend_from_slice(&tiff);
        let mut out = jpeg[..2].to_vec();
        out.extend_from_slice(&app1);
        out.extend_from_slice(&jpeg[2..]);
        out
    }

    pub fn ascii(tag: Tag, s: &str) -> Field {
        Field {
            tag,
            ifd_num: In::PRIMARY,
            value: Value::Ascii(vec![s.as_bytes().to_vec()]),
        }
    }
    pub fn short(tag: Tag, v: u16) -> Field {
        Field {
            tag,
            ifd_num: In::PRIMARY,
            value: Value::Short(vec![v]),
        }
    }
    pub fn rational(tag: Tag, v: &[(u32, u32)]) -> Field {
        Field {
            tag,
            ifd_num: In::PRIMARY,
            value: Value::Rational(
                v.iter()
                    .map(|&(num, denom)| Rational { num, denom })
                    .collect(),
            ),
        }
    }

    #[test]
    fn exif_fields_dates_and_gps() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("IMG_1.JPG");
        let fields = [
            ascii(Tag::Make, "Canon"),
            ascii(Tag::Model, "EOS R6"),
            ascii(Tag::DateTimeOriginal, "2023:07:14 10:22:01"),
            ascii(Tag::DateTimeDigitized, "2023:07:14 10:22:05"),
            short(Tag::Orientation, 6),
            rational(Tag::XResolution, &[(300, 1)]),
            rational(Tag::YResolution, &[(300, 1)]),
            short(Tag::ResolutionUnit, 2),
            ascii(Tag::GPSLatitudeRef, "N"),
            rational(Tag::GPSLatitude, &[(43, 1), (46, 1), (12, 1)]),
            ascii(Tag::GPSLongitudeRef, "E"),
            rational(Tag::GPSLongitude, &[(11, 1), (15, 1), (0, 1)]),
        ];
        std::fs::write(&p, jpeg_with_exif(64, 48, &fields)).unwrap();
        let e = read_exif(&p).unwrap().unwrap();
        assert_eq!(e.info.camera_make.as_deref(), Some("Canon"));
        assert_eq!(e.date_original.as_deref(), Some("2023-07-14 10:22:01"));
        assert_eq!(e.orientation, Some(6));
        assert_eq!(e.dpi, Some((300.0, 300.0)));
        assert!((e.info.gps_lat.unwrap() - 43.77).abs() < 0.001);
        assert!((e.info.gps_lon.unwrap() - 11.25).abs() < 0.001);

        let meta = std::fs::metadata(&p).unwrap();
        let d = resolve_dates(&meta, Some(&e));
        assert_eq!(d.captured_at.as_deref(), Some("2023-07-14 10:22:01"));
        assert_eq!(d.captured_source, Some(DateSource::ExifOriginal));

        let t = read_tech(&p, "jpg", Some(&e)).unwrap();
        assert_eq!(
            (t.width, t.height, t.bit_depth),
            (Some(64), Some(48), Some(8))
        );
        assert_eq!((t.dpi_x, t.dpi_y), (Some(300.0), Some(300.0)));
        assert_eq!(t.format.as_deref(), Some("JPEG"));
    }

    #[test]
    fn missing_exif_falls_back_to_file_system() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("plain.jpg");
        std::fs::write(&p, jpeg_with_exif(32, 32, &[])).unwrap();
        assert_eq!(read_exif(&p).unwrap(), None);
        let d = resolve_dates(&std::fs::metadata(&p).unwrap(), None);
        assert_eq!(d.captured_source, Some(DateSource::FileSystem));
        assert!(d.captured_at.is_some() && d.modified_at.is_some());
    }

    #[test]
    fn declared_offset_and_device_are_read() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("IMG_0042.JPG");
        let fields = [
            ascii(Tag::Make, "Apple"),
            ascii(Tag::Model, "iPhone 17 Pro Max"),
            ascii(Tag::DateTimeOriginal, "2026:08:19 10:44:12"),
            ascii(Tag::OffsetTimeOriginal, "-06:00"),
        ];
        std::fs::write(&p, jpeg_with_exif(32, 32, &fields)).unwrap();
        let e = read_exif(&p).unwrap().unwrap();
        assert_eq!(e.offset_original, Some(-360));
        assert_eq!(e.device.as_deref(), Some("Apple iPhone 17 Pro Max"));
        let d = resolve_dates(&std::fs::metadata(&p).unwrap(), Some(&e));
        assert_eq!(d.captured_at.as_deref(), Some("2026-08-19 10:44:12"));
        assert_eq!(d.declared_offset_min, Some(-360));
        assert_eq!(d.device.as_deref(), Some("Apple iPhone 17 Pro Max"));
    }

    #[test]
    fn missing_or_malformed_offset_is_none() {
        let tmp = tempfile::tempdir().unwrap();
        let absent = tmp.path().join("a.jpg");
        std::fs::write(
            &absent,
            jpeg_with_exif(
                16,
                16,
                &[ascii(Tag::DateTimeOriginal, "2026:08:19 10:43:00")],
            ),
        )
        .unwrap();
        let e = read_exif(&absent).unwrap().unwrap();
        assert_eq!(e.offset_original, None);
        assert_eq!(e.device, None);
        let bad = tmp.path().join("b.jpg");
        std::fs::write(
            &bad,
            jpeg_with_exif(
                16,
                16,
                &[
                    ascii(Tag::DateTimeOriginal, "2026:08:19 10:43:00"),
                    ascii(Tag::OffsetTimeOriginal, "+2h"),
                ],
            ),
        )
        .unwrap();
        assert_eq!(read_exif(&bad).unwrap().unwrap().offset_original, None);
        // Senza DateTimeOriginal si usa la data di digitalizzazione con il suo fuso.
        let dig = tmp.path().join("c.jpg");
        std::fs::write(
            &dig,
            jpeg_with_exif(
                16,
                16,
                &[
                    ascii(Tag::DateTimeDigitized, "2026:08:19 10:43:00"),
                    ascii(Tag::OffsetTimeDigitized, "+02:00"),
                    ascii(Tag::OffsetTime, "+05:00"),
                ],
            ),
        )
        .unwrap();
        let e = read_exif(&dig).unwrap().unwrap();
        let d = resolve_dates(&std::fs::metadata(&dig).unwrap(), Some(&e));
        assert_eq!(d.captured_source, Some(DateSource::ExifCreate));
        assert_eq!(d.declared_offset_min, Some(120));
    }

    #[test]
    fn offsets_and_devices_are_normalized() {
        assert_eq!(parse_offset("+02:00"), Some(120));
        assert_eq!(parse_offset(" -07:00 "), Some(-420));
        assert_eq!(parse_offset("+05:45"), Some(345));
        assert_eq!(parse_offset("+14:00"), Some(840));
        for bad in [
            "", "02:00", "+2:00", "+15:00", "+02:60", "   :  ", "Z", "+02-00",
        ] {
            assert_eq!(parse_offset(bad), None, "{bad}");
        }
        let dev = |m: Option<&str>, n: Option<&str>| normalize_device(m, n);
        assert_eq!(
            dev(Some("FUJIFILM"), Some("X-T5")).as_deref(),
            Some("Fujifilm X-T5")
        );
        assert_eq!(
            dev(Some("Canon"), Some("Canon EOS R6")).as_deref(),
            Some("Canon EOS R6")
        );
        assert_eq!(
            dev(Some("NIKON CORPORATION"), Some("NIKON Z 6_2")).as_deref(),
            Some("Nikon Z 6_2")
        );
        assert_eq!(
            dev(Some("samsung"), Some("SM-S918B")).as_deref(),
            Some("Samsung SM-S918B")
        );
        assert_eq!(
            dev(Some("OLYMPUS IMAGING CORP.  "), Some("E-M10")).as_deref(),
            Some("Olympus E-M10")
        );
        assert_eq!(dev(None, Some("  Pixel 9  ")).as_deref(), Some("Pixel 9"));
        assert_eq!(dev(Some(" "), None), None);
    }

    #[test]
    fn selftest_images_have_no_declared_offset() {
        let dir =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/selftest");
        for name in [
            "group_3_faces.jpg",
            "group_4_people.jpg",
            "landscape_mountain.jpg",
        ] {
            let p = dir.join(name);
            let e = read_exif(&p).unwrap();
            let d = resolve_dates(&std::fs::metadata(&p).unwrap(), e.as_ref());
            assert_eq!(d.declared_offset_min, None, "{name}");
            assert!(d.captured_at.is_some(), "{name}");
        }
    }

    #[test]
    fn corrupt_exif_is_reported_with_code() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("bad.jpg");
        let mut bytes = jpeg_with_exif(32, 32, &[ascii(Tag::Make, "Nikon")]);
        // Rompe l'intestazione TIFF dentro l'APP1 (byte order non valido).
        let i = bytes.windows(6).position(|w| w == b"Exif\0\0").unwrap() + 6;
        bytes[i] = b'X';
        bytes[i + 1] = b'X';
        std::fs::write(&p, &bytes).unwrap();
        assert_eq!(read_exif(&p).unwrap_err().code, ErrorCode::CorruptExif);
        // Le caratteristiche tecniche restano leggibili.
        let t = read_tech(&p, "jpg", None).unwrap();
        assert_eq!(t.width, Some(32));
    }

    #[test]
    fn png_dpi_from_phys_and_bit_depth() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("a.png");
        let img =
            image::ImageBuffer::<image::Rgba<u16>, _>::from_pixel(10, 5, image::Rgba([1, 2, 3, 4]));
        img.save(&p).unwrap();
        // Inserisce un chunk pHYs a 300 dpi (11811 px/m) dopo IHDR.
        let mut bytes = std::fs::read(&p).unwrap();
        let mut chunk = Vec::new();
        chunk.extend_from_slice(&9u32.to_be_bytes());
        let mut body = b"pHYs".to_vec();
        body.extend_from_slice(&11811u32.to_be_bytes());
        body.extend_from_slice(&11811u32.to_be_bytes());
        body.push(1);
        chunk.extend_from_slice(&body);
        chunk.extend_from_slice(&crc32(&body).to_be_bytes());
        let ihdr_end = 8 + 8 + 13 + 4;
        bytes.splice(ihdr_end..ihdr_end, chunk);
        std::fs::write(&p, &bytes).unwrap();
        let t = read_tech(&p, "png", None).unwrap();
        assert_eq!(
            (t.width, t.height, t.bit_depth),
            (Some(10), Some(5), Some(16))
        );
        assert_eq!(t.dpi_x, Some(300.0));
    }

    fn crc32(data: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFFu32;
        for b in data {
            crc ^= *b as u32;
            for _ in 0..8 {
                crc = if crc & 1 != 0 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }

    #[test]
    fn tech_for_every_decodable_format() {
        let tmp = tempfile::tempdir().unwrap();
        let img = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            37,
            21,
            image::Rgb([9, 9, 9]),
        ));
        for (ext, fmt, name) in [
            ("tif", image::ImageFormat::Tiff, "TIFF"),
            ("webp", image::ImageFormat::WebP, "WebP"),
            ("bmp", image::ImageFormat::Bmp, "BMP"),
            ("gif", image::ImageFormat::Gif, "GIF"),
            ("png", image::ImageFormat::Png, "PNG"),
            ("jpg", image::ImageFormat::Jpeg, "JPEG"),
        ] {
            let p = tmp.path().join(format!("x.{ext}"));
            img.save_with_format(&p, fmt).unwrap();
            let t = read_tech(&p, ext, None).unwrap();
            assert_eq!((t.width, t.height), (Some(37), Some(21)), "{ext}");
            assert_eq!(t.format.as_deref(), Some(name));
            assert!(t.bit_depth.is_some(), "{ext}");
        }
    }

    #[test]
    fn date_normalization() {
        assert_eq!(
            normalize_exif_date("2023:07:14 10:22:01").as_deref(),
            Some("2023-07-14 10:22:01")
        );
        assert_eq!(normalize_exif_date("0000:00:00 00:00:00"), None);
        assert_eq!(normalize_exif_date("garbage"), None);
    }
}
