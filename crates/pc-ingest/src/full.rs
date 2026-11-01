//! Decodifica a piena risoluzione per l'export: pixel a 8 o 16 bit (con
//! eventuale alpha), profilo ICC incorporato e orientamento EXIF.
//!
//! JPEG, PNG, TIFF e WebP passano dal crate `image`; HEIC da libheif (che
//! applica già rotazione e specchiatura del contenitore); RAF dal JPEG a
//! piena dimensione incorporato nel contenitore (niente sviluppo del RAW).

use crate::formats::{support, MetaOnlyKind, Support};
use image::{DynamicImage, ImageDecoder, RgbImage};
use pc_core::{AppError, AppResult, ErrorCode};
use pc_governor::{MemoryGovernor, Permit};
use std::io::BufReader;
use std::path::Path;

pub struct FullImage {
    pub image: DynamicImage,
    /// Profilo colore ICC incorporato nell'originale.
    pub icc: Option<Vec<u8>>,
    /// Orientamento EXIF (1–8) ancora da applicare ai pixel; `None` se già applicato.
    pub orientation: Option<u32>,
    pub permit: Permit,
}

impl FullImage {
    pub fn dimensions(&self) -> (u32, u32) {
        (self.image.width(), self.image.height())
    }

    /// Applica l'orientamento ai pixel: dopo, `orientation` è `None`.
    pub fn oriented(mut self) -> Self {
        let img = std::mem::take(&mut self.image);
        self.image = match self.orientation.unwrap_or(1) {
            2 => img.fliph(),
            3 => img.rotate180(),
            4 => img.flipv(),
            5 => img.rotate90().fliph(),
            6 => img.rotate90(),
            7 => img.rotate270().fliph(),
            8 => img.rotate270(),
            _ => img,
        };
        self.orientation = None;
        self
    }

    /// RGB a 8 bit; l'eventuale trasparenza è composta su bianco.
    pub fn to_rgb8(&self) -> RgbImage {
        if !self.image.color().has_alpha() {
            return self.image.to_rgb8();
        }
        let rgba = self.image.to_rgba8();
        RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
            let p = rgba.get_pixel(x, y).0;
            let a = u32::from(p[3]);
            let mix = |c: u8| ((u32::from(c) * a + 255 * (255 - a)) / 255) as u8;
            image::Rgb([mix(p[0]), mix(p[1]), mix(p[2])])
        })
    }
}

fn cost_mb(w: u32, h: u32, bytes_per_pixel: u64) -> u64 {
    (u64::from(w) * u64::from(h) * bytes_per_pixel)
        .div_ceil(1024 * 1024)
        .max(1)
}

fn permit(gov: &MemoryGovernor, mb: u64) -> AppResult<Permit> {
    if mb > gov.capacity_mb() {
        return Err(AppError::new(
            ErrorCode::InsufficientMemory,
            format!("immagine troppo grande per il budget di memoria ({mb} MB)"),
        ));
    }
    Ok(gov.acquire(mb))
}

fn exif_orientation(path: &Path) -> Option<u32> {
    crate::metadata::read_exif(path)
        .ok()
        .flatten()
        .and_then(|m| m.orientation)
}

fn decode_with_image<R: std::io::BufRead + std::io::Seek>(
    reader: image::ImageReader<R>,
    gov: &MemoryGovernor,
) -> AppResult<(DynamicImage, Option<Vec<u8>>, Permit)> {
    let bad = |e: image::ImageError| AppError::new(ErrorCode::DecodeFailed, e.to_string());
    let mut reader = reader.with_guessed_format()?;
    reader.no_limits();
    let mut dec = reader.into_decoder().map_err(bad)?;
    let icc = dec.icc_profile().ok().flatten().filter(|p| !p.is_empty());
    let (w, h) = dec.dimensions();
    // Pixel decodificati più una copia di lavoro (orientamento, conversione).
    let bpp = u64::from(dec.color_type().bytes_per_pixel());
    let permit = permit(gov, cost_mb(w, h, bpp * 2 + 3))?;
    let image = DynamicImage::from_decoder(dec).map_err(bad)?;
    Ok((image, icc, permit))
}

/// Decodifica `path` a piena risoluzione.
pub fn decode_full(path: &Path, ext: &str, gov: &MemoryGovernor) -> AppResult<FullImage> {
    match support(ext) {
        Some(Support::Decodable(_)) => {
            let reader = image::ImageReader::new(BufReader::new(std::fs::File::open(path)?));
            let (image, icc, permit) = decode_with_image(reader, gov)?;
            Ok(FullImage {
                image,
                icc,
                orientation: exif_orientation(path),
                permit,
            })
        }
        Some(Support::MetadataOnly(MetaOnlyKind::Raw)) => {
            let preview = crate::thumb::embedded_preview(path).ok_or_else(|| {
                AppError::new(
                    ErrorCode::UnsupportedFormat,
                    "RAW senza anteprima incorporata",
                )
            })?;
            let orientation = exif::Reader::new()
                .read_from_container(&mut std::io::Cursor::new(&preview))
                .ok()
                .and_then(|e| {
                    e.get_field(exif::Tag::Orientation, exif::In::PRIMARY)?
                        .value
                        .get_uint(0)
                })
                .filter(|o| (1..=8).contains(o))
                .or_else(|| exif_orientation(path));
            let reader = image::ImageReader::new(std::io::Cursor::new(preview));
            let (image, icc, permit) = decode_with_image(reader, gov)?;
            Ok(FullImage {
                image,
                icc,
                orientation,
                permit,
            })
        }
        #[cfg(feature = "heif")]
        Some(Support::MetadataOnly(MetaOnlyKind::Heif)) => {
            let d = crate::heif::decode(path, u32::MAX, gov)?;
            Ok(FullImage {
                image: DynamicImage::ImageRgb8(d.image),
                icc: crate::heif::icc_profile(path).ok().flatten(),
                orientation: None,
                permit: d.permit,
            })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::tests::{jpeg_with_exif, short};
    use exif::Tag;
    use image::ImageEncoder;

    /// PNG con un chunk iCCP (profilo zlib) inserito dopo IHDR.
    fn png_with_icc(w: u32, h: u32, profile: &[u8]) -> Vec<u8> {
        let img = image::RgbImage::from_fn(w, h, |x, y| image::Rgb([x as u8, y as u8, 7]));
        let mut png = Vec::new();
        let mut enc = image::codecs::png::PngEncoder::new(&mut png);
        enc.set_icc_profile(profile.to_vec()).unwrap();
        enc.write_image(img.as_raw(), w, h, image::ExtendedColorType::Rgb8)
            .unwrap();
        png
    }

    #[test]
    fn png_profile_is_returned_unchanged() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("p3.png");
        // Profilo fittizio ma con intestazione ICC plausibile (il contenuto non viene interpretato).
        let mut profile = vec![0u8; 512];
        profile[0..4].copy_from_slice(&512u32.to_be_bytes());
        profile[36..40].copy_from_slice(b"acsp");
        profile[100..120].copy_from_slice(b"Display P3 di prova!");
        std::fs::write(&p, png_with_icc(40, 30, &profile)).unwrap();
        let f = decode_full(&p, "png", &MemoryGovernor::new(64)).unwrap();
        assert_eq!(f.icc.as_deref(), Some(profile.as_slice()));
        assert_eq!(f.dimensions(), (40, 30));
        assert_eq!(f.image.color(), image::ColorType::Rgb8);
    }

    #[test]
    fn orientation_6_is_rotated() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("r.jpg");
        std::fs::write(&p, jpeg_with_exif(64, 32, &[short(Tag::Orientation, 6)])).unwrap();
        let f = decode_full(&p, "jpg", &MemoryGovernor::new(64)).unwrap();
        assert_eq!(f.orientation, Some(6));
        assert_eq!(f.dimensions(), (64, 32));
        let o = f.oriented();
        assert_eq!(o.dimensions(), (32, 64));
        assert_eq!(o.orientation, None);
    }

    #[test]
    fn sixteen_bit_png_keeps_depth_and_alpha_is_flattened_on_white() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("deep.png");
        let img = image::ImageBuffer::<image::Rgba<u16>, _>::from_fn(4, 4, |x, _| {
            image::Rgba([65535, 0, 0, if x < 2 { 0 } else { 65535 }])
        });
        DynamicImage::ImageRgba16(img).save(&p).unwrap();
        let f = decode_full(&p, "png", &MemoryGovernor::new(64)).unwrap();
        assert_eq!(f.image.color(), image::ColorType::Rgba16);
        let rgb = f.to_rgb8();
        assert_eq!(rgb.get_pixel(0, 0).0, [255, 255, 255]);
        assert_eq!(rgb.get_pixel(3, 0).0, [255, 0, 0]);
    }

    #[test]
    fn too_large_for_the_budget_is_an_error() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("big.png");
        image::RgbImage::new(1200, 1200).save(&p).unwrap();
        let err = decode_full(&p, "png", &MemoryGovernor::new(4))
            .err()
            .unwrap();
        assert_eq!(err.code, ErrorCode::InsufficientMemory);
    }
}
