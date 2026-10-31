//! Formati riconosciuti e cosa si può fare con ciascuno.

/// Capacità di elaborazione di un formato.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Support {
    /// Decodificabile: miniature e analisi AI.
    Decodable(image::ImageFormat),
    /// Importato con i soli metadati (decodifica non disponibile).
    MetadataOnly(MetaOnlyKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaOnlyKind {
    Heif,
    Raw,
}

/// Estensioni importate di default (minuscole, senza punto).
pub const DEFAULT_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "tif", "tiff", "webp", "bmp", "gif", "heic", "heif", "avif", "dng",
    "cr2", "cr3", "nef", "arw", "orf", "rw2", "raf", "pef", "srw",
];

pub fn support(ext: &str) -> Option<Support> {
    use image::ImageFormat as F;
    Some(match ext.to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" | "jpe" => Support::Decodable(F::Jpeg),
        "png" => Support::Decodable(F::Png),
        "tif" | "tiff" => Support::Decodable(F::Tiff),
        "webp" => Support::Decodable(F::WebP),
        "bmp" => Support::Decodable(F::Bmp),
        "gif" => Support::Decodable(F::Gif),
        "heic" | "heif" | "avif" => Support::MetadataOnly(MetaOnlyKind::Heif),
        "dng" | "cr2" | "cr3" | "nef" | "arw" | "orf" | "rw2" | "raf" | "pef" | "srw" => {
            Support::MetadataOnly(MetaOnlyKind::Raw)
        }
        _ => return None,
    })
}

/// Nome del formato mostrato all'utente.
pub fn format_name(ext: &str) -> String {
    match support(ext) {
        Some(Support::Decodable(f)) => match f {
            image::ImageFormat::Jpeg => "JPEG".into(),
            image::ImageFormat::Png => "PNG".into(),
            image::ImageFormat::Tiff => "TIFF".into(),
            image::ImageFormat::WebP => "WebP".into(),
            image::ImageFormat::Bmp => "BMP".into(),
            image::ImageFormat::Gif => "GIF".into(),
            other => format!("{other:?}").to_uppercase(),
        },
        Some(Support::MetadataOnly(MetaOnlyKind::Heif)) => ext.to_ascii_uppercase(),
        Some(Support::MetadataOnly(MetaOnlyKind::Raw)) => {
            format!("RAW ({})", ext.to_ascii_uppercase())
        }
        None => ext.to_ascii_uppercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification() {
        assert!(matches!(
            support("JPG"),
            Some(Support::Decodable(image::ImageFormat::Jpeg))
        ));
        assert_eq!(
            support("heic"),
            Some(Support::MetadataOnly(MetaOnlyKind::Heif))
        );
        assert_eq!(
            support("NEF"),
            Some(Support::MetadataOnly(MetaOnlyKind::Raw))
        );
        assert_eq!(support("txt"), None);
        assert_eq!(format_name("jpeg"), "JPEG");
        assert_eq!(format_name("cr2"), "RAW (CR2)");
        for e in DEFAULT_EXTENSIONS {
            assert!(support(e).is_some(), "{e}");
        }
    }
}
