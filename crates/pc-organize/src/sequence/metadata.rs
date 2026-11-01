//! Metadati scritti nelle copie esportate (mai negli originali): blocco EXIF
//! con data e fuso corretti, fotocamera, obiettivo, GPS e orientamento 1
//! (la rotazione è già applicata ai pixel), e pacchetto XMP con i tag come
//! parole chiave (`dc:subject`) e le identità (`Iptc4xmpExt:PersonInImage`).

use exif::experimental::Writer;
use exif::{Field, In, Rational, Tag, Value};
use pc_core::{AppError, AppResult, ErrorCode};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CopyMeta {
    /// Ora locale corretta, `AAAA-MM-GG HH:MM:SS`.
    pub local: Option<String>,
    /// Fuso dell'ora locale, in minuti.
    pub offset_min: Option<i32>,
    pub make: Option<String>,
    pub model: Option<String>,
    pub lens: Option<String>,
    pub gps: Option<(f64, f64)>,
    pub keywords: Vec<String>,
    pub people: Vec<String>,
}

const XMP_HEADER: &[u8] = b"http://ns.adobe.com/xap/1.0/\0";

fn ascii(tag: Tag, s: &str) -> Field {
    Field {
        tag,
        ifd_num: In::PRIMARY,
        value: Value::Ascii(vec![s.as_bytes().to_vec()]),
    }
}

fn offset_text(min: i32) -> String {
    let sign = if min < 0 { '-' } else { '+' };
    let m = min.abs();
    format!("{sign}{:02}:{:02}", m / 60, m % 60)
}

/// Gradi decimali → gradi, minuti, secondi (millesimi) come razionali EXIF.
fn dms(v: f64) -> Vec<Rational> {
    let v = v.abs();
    let deg = v.trunc();
    let min = ((v - deg) * 60.0).trunc();
    let sec = ((v - deg) * 60.0 - min) * 60.0;
    vec![
        Rational {
            num: deg as u32,
            denom: 1,
        },
        Rational {
            num: min as u32,
            denom: 1,
        },
        Rational {
            num: (sec * 1000.0).round() as u32,
            denom: 1000,
        },
    ]
}

/// Segmento APP1 EXIF (`Exif\0\0` + TIFF).
pub fn exif_segment(m: &CopyMeta) -> AppResult<Vec<u8>> {
    let mut fields: Vec<Field> = vec![Field {
        tag: Tag::Orientation,
        ifd_num: In::PRIMARY,
        value: Value::Short(vec![1]),
    }];
    if let Some(local) = &m.local {
        let exif_date = local.replacen('-', ":", 2);
        fields.push(ascii(Tag::DateTimeOriginal, &exif_date));
        fields.push(ascii(Tag::DateTimeDigitized, &exif_date));
        fields.push(ascii(Tag::DateTime, &exif_date));
        if let Some(off) = m.offset_min {
            let t = offset_text(off);
            fields.push(ascii(Tag::OffsetTimeOriginal, &t));
            fields.push(ascii(Tag::OffsetTimeDigitized, &t));
            fields.push(ascii(Tag::OffsetTime, &t));
        }
    }
    for (tag, v) in [
        (Tag::Make, &m.make),
        (Tag::Model, &m.model),
        (Tag::LensModel, &m.lens),
    ] {
        if let Some(s) = v.as_deref().filter(|s| !s.trim().is_empty()) {
            fields.push(ascii(tag, s.trim()));
        }
    }
    if let Some((lat, lon)) = m.gps.filter(|(a, b)| a.is_finite() && b.is_finite()) {
        fields.push(Field {
            tag: Tag::GPSVersionID,
            ifd_num: In::PRIMARY,
            value: Value::Byte(vec![2, 3, 0, 0]),
        });
        fields.push(ascii(
            Tag::GPSLatitudeRef,
            if lat < 0.0 { "S" } else { "N" },
        ));
        fields.push(Field {
            tag: Tag::GPSLatitude,
            ifd_num: In::PRIMARY,
            value: Value::Rational(dms(lat)),
        });
        fields.push(ascii(
            Tag::GPSLongitudeRef,
            if lon < 0.0 { "W" } else { "E" },
        ));
        fields.push(Field {
            tag: Tag::GPSLongitude,
            ifd_num: In::PRIMARY,
            value: Value::Rational(dms(lon)),
        });
    }
    let mut writer = Writer::new();
    for f in &fields {
        writer.push_field(f);
    }
    let mut tiff = std::io::Cursor::new(Vec::new());
    writer
        .write(&mut tiff, false)
        .map_err(|e| AppError::new(ErrorCode::Internal, format!("scrittura EXIF: {e}")))?;
    let mut out = b"Exif\0\0".to_vec();
    out.extend_from_slice(&tiff.into_inner());
    Ok(out)
}

fn xml_escape(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control())
        .map(|c| match c {
            '&' => "&amp;".to_string(),
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '"' => "&quot;".to_string(),
            '\'' => "&apos;".to_string(),
            c => c.to_string(),
        })
        .collect()
}

fn bag(name: &str, items: &[String]) -> String {
    if items.is_empty() {
        return String::new();
    }
    let li: String = items
        .iter()
        .map(|i| format!("<rdf:li>{}</rdf:li>", xml_escape(i)))
        .collect();
    format!("<{name}><rdf:Bag>{li}</rdf:Bag></{name}>")
}

/// Segmento APP1 XMP; `None` se non c'è nulla da scrivere.
pub fn xmp_segment(m: &CopyMeta) -> Option<Vec<u8>> {
    let date = match (&m.local, m.offset_min) {
        (Some(l), Some(o)) => format!(
            "<photoshop:DateCreated>{}{}</photoshop:DateCreated>",
            l.replace(' ', "T"),
            offset_text(o)
        ),
        (Some(l), None) => format!(
            "<photoshop:DateCreated>{}</photoshop:DateCreated>",
            l.replace(' ', "T")
        ),
        _ => String::new(),
    };
    if m.keywords.is_empty() && m.people.is_empty() && date.is_empty() {
        return None;
    }
    let packet = format!(
        "<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\
<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\
<rdf:Description rdf:about=\"\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\" \
xmlns:Iptc4xmpExt=\"http://iptc.org/std/Iptc4xmpExt/2008-02-29/\" \
xmlns:photoshop=\"http://ns.adobe.com/photoshop/1.0/\">{}{}{}</rdf:Description></rdf:RDF></x:xmpmeta>\
<?xpacket end=\"w\"?>",
        bag("dc:subject", &m.keywords),
        bag("Iptc4xmpExt:PersonInImage", &m.people),
        date,
    );
    let mut out = XMP_HEADER.to_vec();
    out.extend_from_slice(packet.as_bytes());
    // Un segmento APP non può superare 64 KB: in quel caso si rinuncia all'XMP.
    (out.len() <= 65533).then_some(out)
}

/// Segmenti APP1 (EXIF, poi XMP) da passare all'encoder.
pub fn app_segments(m: &CopyMeta) -> AppResult<Vec<(u8, Vec<u8>)>> {
    let mut v = vec![(1u8, exif_segment(m)?)];
    if let Some(x) = xmp_segment(m) {
        v.push((1, x));
    }
    Ok(v)
}

/// Elenchi `dc:subject` e `PersonInImage` di un JPEG (verifica delle copie).
pub fn read_xmp_lists(jpeg: &[u8]) -> (Vec<String>, Vec<String>) {
    let Some(start) = jpeg.windows(XMP_HEADER.len()).position(|w| w == XMP_HEADER) else {
        return (Vec::new(), Vec::new());
    };
    let text = String::from_utf8_lossy(&jpeg[start + XMP_HEADER.len()..]);
    let end = text.find("<?xpacket end").unwrap_or(text.len());
    let text = &text[..end];
    let list = |name: &str| -> Vec<String> {
        let open = format!("<{name}>");
        let close = format!("</{name}>");
        let Some(a) = text.find(&open) else {
            return Vec::new();
        };
        let Some(b) = text[a..].find(&close) else {
            return Vec::new();
        };
        text[a..a + b]
            .split("<rdf:li>")
            .skip(1)
            .filter_map(|s| s.split("</rdf:li>").next())
            .map(|s| {
                s.replace("&lt;", "<")
                    .replace("&gt;", ">")
                    .replace("&quot;", "\"")
                    .replace("&apos;", "'")
                    .replace("&amp;", "&")
            })
            .collect()
    };
    (list("dc:subject"), list("Iptc4xmpExt:PersonInImage"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sequence::encode::{encode_verified, JpegSettings};

    fn meta() -> CopyMeta {
        CopyMeta {
            local: Some("2026-08-19 10:44:00".into()),
            offset_min: Some(-360),
            make: Some("FUJIFILM".into()),
            model: Some("X-T5".into()),
            lens: Some("XF16-55mmF2.8 R LM WR".into()),
            gps: Some((38.5733, -109.5498)),
            keywords: vec!["Paesaggi".into(), "Rock & Roll <live>".into()],
            people: vec!["Giulia Bianchi".into()],
        }
    }

    #[test]
    fn copy_metadata_round_trips_through_the_ingest_reader() {
        let tmp = tempfile::tempdir().unwrap();
        let img =
            image::RgbImage::from_fn(40, 30, |x, y| image::Rgb([x as u8 * 6, y as u8 * 8, 90]));
        let segs = app_segments(&meta()).unwrap();
        let out = encode_verified(&img, None, &segs, &JpegSettings::default()).unwrap();
        let p = tmp.path().join("0001.jpg");
        std::fs::write(&p, &out.bytes).unwrap();

        let e = pc_ingest::metadata::read_exif(&p).unwrap().unwrap();
        assert_eq!(e.date_original.as_deref(), Some("2026-08-19 10:44:00"));
        assert_eq!(e.offset_original, Some(-360));
        assert_eq!(e.orientation, Some(1));
        assert_eq!(e.device.as_deref(), Some("Fujifilm X-T5"));
        assert_eq!(e.info.lens.as_deref(), Some("XF16-55mmF2.8 R LM WR"));
        assert!((e.info.gps_lat.unwrap() - 38.5733).abs() < 1e-4);
        assert!((e.info.gps_lon.unwrap() + 109.5498).abs() < 1e-4);

        let (keywords, people) = read_xmp_lists(&out.bytes);
        assert_eq!(keywords, vec!["Paesaggi", "Rock & Roll <live>"]);
        assert_eq!(people, vec!["Giulia Bianchi"]);
        // I pixel restano decodificabili con i nuovi segmenti.
        let d = image::load_from_memory(&out.bytes).unwrap();
        assert_eq!((d.width(), d.height()), (40, 30));
    }

    #[test]
    fn minimal_metadata_has_orientation_and_no_xmp() {
        let m = CopyMeta::default();
        let segs = app_segments(&m).unwrap();
        assert_eq!(segs.len(), 1);
        assert!(segs[0].1.starts_with(b"Exif\0\0"));
        assert!(xmp_segment(&m).is_none());
    }
}
