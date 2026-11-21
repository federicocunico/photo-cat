//! Metadati di uno scatto per il pannello laterale: dati di scatto EXIF, GPS,
//! informazioni sul file e ricetta Fujifilm (simulazione pellicola, gamma
//! dinamica, bilanciamento del bianco, toni...) letta dal MakerNote.
//!
//! Porting di `metadata.rs` di FujiView (a798987), senza Tauri: la lettura è
//! la funzione pura [`read_metadata`]. Le tabelle dei codici sono verificate su
//! MakerNote reali di X-T5 (`tests/fixtures/xt5_makernote_matrix.json`).
//!
//! # Etichette e valori (differenza voluta rispetto a FujiView)
//!
//! FujiView scriveva le etichette in italiano. Qui `MetaSection::title` e
//! `MetaRow::label` sono **chiavi stabili** (per esempio `shot`, `exposureTime`,
//! `filmSimulation`) che la UI traduce con i18n; i valori sono già formattati.
//!
//! I valori enumerati da tradurre (modi di esposizione, bilanciamento del
//! bianco, effetti, "Monocromatico"...) portano anche una [`MetaValueKey`]
//! (`MetaRow::value_key`, `Recipe::film_key`, `Recipe::white_balance_key`):
//! la UI li traduce con `rawReviewMeta.values.<chiave>` e, se la chiave manca,
//! mostra `value`, che resta il testo italiano di FujiView. Numeri, sigle e
//! nomi propri (`1/250 s`, `+4`, `DR200`, `Classic Chrome`) non hanno chiave.
//! `Recipe::summary` è la riga italiana già composta: la UI la ricompone da
//! `film`, `dynamic_range` e `white_balance` con le loro chiavi.
//!
//! Limite noto: i decimali usano la virgola (`f/2,8`, `0,5 s`), come in FujiView
//! (i suoi test lo fissano).
//!
//! # RAF
//!
//! `kamadak-exif` non legge il contenitore RAF: per la pseudo-variante
//! `raw_preview` l'EXIF e il MakerNote si leggono dal JPEG incorporato nel RAF
//! (offset e lunghezza nell'intestazione). Se non c'è o non è leggibile, i
//! metadati contengono solo la sezione `file`, senza errore. Le dimensioni
//! (`dimensions`) di un RAF sono omesse: quelle del JPEG incorporato non sono
//! quelle del RAW.

use pc_core::error::AppResult;
use serde::Serialize;
use std::{
    fs,
    io::{BufReader, Cursor, Read, Seek, SeekFrom},
    path::Path,
};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct MetaRow {
    /// Chiave stabile da tradurre (per esempio `exposureTime`).
    pub label: String,
    /// Valore già formattato (testo italiano per i valori enumerati).
    pub value: String,
    /// Chiave di traduzione del valore, solo per i valori enumerati da tradurre.
    pub value_key: Option<MetaValueKey>,
}

/// Valori enumerati traducibili dalla UI (`rawReviewMeta.values.<chiave>`).
///
/// Il nome serializzato è la chiave i18n. L'elenco è chiuso: nei binding
/// TypeScript diventa `MetaValueKey`, e il typecheck controlla che ogni chiave
/// abbia la traduzione in italiano e in inglese.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum MetaValueKey {
    ProgramManual,
    ProgramNormal,
    ProgramAperture,
    ProgramShutter,
    MeteringAverage,
    MeteringCenterWeighted,
    MeteringSpot,
    MeteringMultiSpot,
    MeteringMultiSegment,
    MeteringPartial,
    FlashFired,
    FlashNotFired,
    ShutterMechanical,
    ShutterElectronic,
    ShutterElectronicLong,
    ShutterElectronicFront,
    FocusAuto,
    FocusManual,
    FilmMonochrome,
    FilmMonochromeR,
    FilmMonochromeYe,
    FilmMonochromeG,
    FilmSepia,
    WbAuto,
    WbAutoWhite,
    WbAutoAmbience,
    WbDaylight,
    WbShade,
    WbFluorescent1,
    WbFluorescent2,
    WbFluorescent3,
    WbIncandescent,
    WbFlash,
    WbUnderwater,
    WbCustom,
    WbKelvin,
    EffectOff,
    EffectWeak,
    EffectStrong,
    GrainWeakSmall,
    GrainWeakLarge,
    GrainStrongSmall,
    GrainStrongLarge,
}

/// Valore enumerato: chiave di traduzione e testo italiano di FujiView.
type Entry = (MetaValueKey, &'static str);

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct MetaSection {
    /// Chiave stabile della sezione: `shot`, `camera`, `place` o `file`.
    pub title: String,
    pub rows: Vec<MetaRow>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct GpsPoint {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Recipe {
    /// Riga breve, per esempio "Classic Chrome · DR200 · Auto".
    pub summary: String,
    /// Simulazione pellicola, per l'intestazione del pannello.
    pub film: Option<String>,
    /// Chiave di traduzione di `film` (solo monocromatico e seppia: gli altri
    /// sono nomi propri).
    pub film_key: Option<MetaValueKey>,
    /// Gamma dinamica (`DR200`).
    pub dynamic_range: Option<String>,
    /// Bilanciamento del bianco.
    pub white_balance: Option<String>,
    /// Chiave di traduzione di `white_balance`.
    pub white_balance_key: Option<MetaValueKey>,
    pub rows: Vec<MetaRow>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ImageMetadata {
    pub sections: Vec<MetaSection>,
    pub gps: Option<GpsPoint>,
    pub recipe: Option<Recipe>,
}

fn row(rows: &mut Vec<MetaRow>, label: &str, value: Option<String>) {
    if let Some(value) = value.filter(|v| !v.trim().is_empty()) {
        rows.push(MetaRow {
            label: label.into(),
            value,
            value_key: None,
        });
    }
}

/// Riga con un valore enumerato: testo italiano più chiave di traduzione.
fn enum_row(rows: &mut Vec<MetaRow>, label: &str, entry: Option<Entry>) {
    if let Some((key, text)) = entry {
        rows.push(MetaRow {
            label: label.into(),
            value: text.into(),
            value_key: Some(key),
        });
    }
}

/// Estrae il JPEG incorporato in un RAF Fujifilm (offset e lunghezza big-endian
/// a 0x54 e 0x58). `None` se il file non è un RAF o l'intervallo non è valido.
fn raf_embedded_jpeg(path: &Path) -> Option<Vec<u8>> {
    let mut file = fs::File::open(path).ok()?;
    let total = file.metadata().ok()?.len();
    let mut header = [0u8; 0x5C];
    file.read_exact(&mut header).ok()?;
    if !header.starts_with(b"FUJIFILMCCD-RAW") {
        return None;
    }
    let be =
        |o: usize| u32::from_be_bytes([header[o], header[o + 1], header[o + 2], header[o + 3]]);
    let (offset, len) = (u64::from(be(0x54)), u64::from(be(0x58)));
    if len == 0 || offset.checked_add(len)? > total {
        return None;
    }
    file.seek(SeekFrom::Start(offset)).ok()?;
    let mut jpeg = vec![0u8; usize::try_from(len).ok()?];
    file.read_exact(&mut jpeg).ok()?;
    Some(jpeg)
}

fn read_exif(path: &Path) -> Option<exif::Exif> {
    let direct = fs::File::open(path).ok().and_then(|file| {
        exif::Reader::new()
            .read_from_container(&mut BufReader::new(file))
            .ok()
    });
    direct.or_else(|| {
        let jpeg = raf_embedded_jpeg(path)?;
        exif::Reader::new()
            .read_from_container(&mut Cursor::new(jpeg))
            .ok()
    })
}

fn is_raf(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("raf"))
}

/// Legge metadati e ricetta di uno scatto (JPEG, TIFF o RAF). Non scrive nulla.
/// Un EXIF assente o illeggibile non è un errore: restano le informazioni sul
/// file. Errore solo se il file non è accessibile.
pub fn read_metadata(path: &Path) -> AppResult<ImageMetadata> {
    let file_meta = fs::metadata(path)?;
    let exif = read_exif(path);

    let mut sections = Vec::new();
    let mut gps = None;
    let mut recipe = None;
    let mut maker = MakerNote::default();

    if let Some(exif) = &exif {
        if let Some(exif::Value::Undefined(bytes, _)) =
            field(exif, exif::Tag::MakerNote).map(|f| &f.value)
        {
            maker = MakerNote::parse(bytes).unwrap_or_default();
        }

        let mut shot = Vec::new();
        row(&mut shot, "dateTime", date_time(exif));
        row(
            &mut shot,
            "exposureTime",
            rational(exif, exif::Tag::ExposureTime).map(format_exposure),
        );
        row(
            &mut shot,
            "aperture",
            rational(exif, exif::Tag::FNumber).map(|f| format!("f/{}", trim_float(f, 1))),
        );
        row(
            &mut shot,
            "iso",
            uint(exif, exif::Tag::PhotographicSensitivity).map(|v| v.to_string()),
        );
        row(
            &mut shot,
            "exposureBias",
            srational(exif, exif::Tag::ExposureBiasValue).map(format_bias),
        );
        row(&mut shot, "focalLength", focal_length(exif));
        enum_row(
            &mut shot,
            "exposureProgram",
            uint(exif, exif::Tag::ExposureProgram).and_then(exposure_program),
        );
        enum_row(
            &mut shot,
            "meteringMode",
            uint(exif, exif::Tag::MeteringMode).and_then(metering_mode),
        );
        enum_row(
            &mut shot,
            "flash",
            uint(exif, exif::Tag::Flash).map(|v| {
                if v & 1 == 1 {
                    (MetaValueKey::FlashFired, "Scattato")
                } else {
                    (MetaValueKey::FlashNotFired, "Non scattato")
                }
            }),
        );
        enum_row(
            &mut shot,
            "shutterType",
            maker.get(0x1050).and_then(shutter_type),
        );
        enum_row(
            &mut shot,
            "focusMode",
            maker.get(0x1021).and_then(focus_mode),
        );
        if !shot.is_empty() {
            sections.push(MetaSection {
                title: "shot".into(),
                rows: shot,
            });
        }

        let mut camera = Vec::new();
        row(&mut camera, "camera", camera_name(exif));
        row(&mut camera, "lens", lens_name(exif));
        row(&mut camera, "firmware", text(exif, exif::Tag::Software));
        if !camera.is_empty() {
            sections.push(MetaSection {
                title: "camera".into(),
                rows: camera,
            });
        }

        gps = gps_point(exif);
        if let Some(point) = &gps {
            let mut place = vec![MetaRow {
                label: "coordinates".into(),
                value: format!("{:.6}, {:.6}", point.latitude, point.longitude),
                value_key: None,
            }];
            row(
                &mut place,
                "altitude",
                point.altitude.map(|a| format!("{a:.0} m")),
            );
            sections.push(MetaSection {
                title: "place".into(),
                rows: place,
            });
        }

        recipe = maker.recipe();
    }

    let mut file = Vec::new();
    row(
        &mut file,
        "name",
        path.file_name().map(|n| n.to_string_lossy().into_owned()),
    );
    let dims = exif.as_ref().filter(|_| !is_raf(path)).and_then(|e| {
        Some((
            uint(e, exif::Tag::PixelXDimension)?,
            uint(e, exif::Tag::PixelYDimension)?,
        ))
    });
    row(
        &mut file,
        "dimensions",
        dims.map(|(w, h)| {
            format!("{w} × {h} px ({:.1} MP)", (w as f64 * h as f64) / 1e6).replace('.', ",")
        }),
    );
    row(
        &mut file,
        "size",
        Some(format!("{:.1} MB", file_meta.len() as f64 / 1_048_576.0).replace('.', ",")),
    );
    row(
        &mut file,
        "folder",
        path.parent().map(|p| p.display().to_string()),
    );
    sections.push(MetaSection {
        title: "file".into(),
        rows: file,
    });

    Ok(ImageMetadata {
        sections,
        gps,
        recipe,
    })
}

// ---------- EXIF helpers ----------

fn field(exif: &exif::Exif, tag: exif::Tag) -> Option<&exif::Field> {
    exif.get_field(tag, exif::In::PRIMARY)
}

fn text(exif: &exif::Exif, tag: exif::Tag) -> Option<String> {
    match &field(exif, tag)?.value {
        exif::Value::Ascii(parts) => parts
            .first()
            .map(|p| {
                String::from_utf8_lossy(p)
                    .trim_matches(char::from(0))
                    .trim()
                    .to_string()
            })
            .filter(|s| !s.is_empty()),
        _ => None,
    }
}

fn uint(exif: &exif::Exif, tag: exif::Tag) -> Option<u32> {
    field(exif, tag)?.value.get_uint(0)
}

fn rational(exif: &exif::Exif, tag: exif::Tag) -> Option<f64> {
    match &field(exif, tag)?.value {
        exif::Value::Rational(v) => v.first().filter(|r| r.denom != 0).map(|r| r.to_f64()),
        _ => None,
    }
}

fn srational(exif: &exif::Exif, tag: exif::Tag) -> Option<f64> {
    match &field(exif, tag)?.value {
        exif::Value::SRational(v) => v.first().filter(|r| r.denom != 0).map(|r| r.to_f64()),
        exif::Value::Rational(v) => v.first().filter(|r| r.denom != 0).map(|r| r.to_f64()),
        _ => None,
    }
}

fn trim_float(value: f64, decimals: usize) -> String {
    let s = format!("{value:.decimals$}");
    let s = if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    };
    s.replace('.', ",")
}

/// "2026:11:12 15:30:01" (+ "+01:00") → "12/11/2026 15:30:01 (UTC+01:00)".
fn date_time(exif: &exif::Exif) -> Option<String> {
    let raw =
        text(exif, exif::Tag::DateTimeOriginal).or_else(|| text(exif, exif::Tag::DateTime))?;
    let (date, time) = raw.split_once(' ')?;
    let mut parts = date.split(':');
    let (y, m, d) = (parts.next()?, parts.next()?, parts.next()?);
    let mut out = format!("{d}/{m}/{y} {time}");
    if let Some(offset) = text(exif, exif::Tag::OffsetTimeOriginal) {
        out.push_str(&format!(" (UTC{offset})"));
    }
    Some(out)
}

pub fn format_exposure(seconds: f64) -> String {
    if seconds <= 0.0 {
        return "—".into();
    }
    if seconds < 0.3 {
        format!("1/{} s", (1.0 / seconds).round() as u64)
    } else {
        format!("{} s", trim_float(seconds, 1))
    }
}

pub fn format_bias(ev: f64) -> String {
    if ev.abs() < 0.01 {
        "0 EV".into()
    } else {
        format!(
            "{}{} EV",
            if ev > 0.0 { "+" } else { "−" },
            trim_float(ev.abs(), 1)
        )
    }
}

fn focal_length(exif: &exif::Exif) -> Option<String> {
    let focal = rational(exif, exif::Tag::FocalLength)?;
    let mut out = format!("{} mm", trim_float(focal, 1));
    if let Some(eq) = uint(exif, exif::Tag::FocalLengthIn35mmFilm).filter(|v| *v > 0) {
        out.push_str(&format!(" ({eq} mm equiv.)"));
    }
    Some(out)
}

fn camera_name(exif: &exif::Exif) -> Option<String> {
    let model = text(exif, exif::Tag::Model)?;
    Some(match text(exif, exif::Tag::Make) {
        Some(make) if !model.to_lowercase().starts_with(&make.to_lowercase()) => {
            format!("{make} {model}")
        }
        _ => model,
    })
}

fn lens_name(exif: &exif::Exif) -> Option<String> {
    let model = text(exif, exif::Tag::LensModel)?;
    Some(match text(exif, exif::Tag::LensMake) {
        Some(make) if !model.to_lowercase().contains(&make.to_lowercase()) => {
            format!("{make} {model}")
        }
        _ => model,
    })
}

fn gps_point(exif: &exif::Exif) -> Option<GpsPoint> {
    let coord = |tag, reference| -> Option<f64> {
        let exif::Value::Rational(v) = &field(exif, tag)?.value else {
            return None;
        };
        if v.len() < 3 || v.iter().any(|r| r.denom == 0) {
            return None;
        }
        let value = v[0].to_f64() + v[1].to_f64() / 60.0 + v[2].to_f64() / 3600.0;
        let negative = matches!(text(exif, reference).as_deref(), Some("S") | Some("W"));
        Some(if negative { -value } else { value })
    };
    let latitude = coord(exif::Tag::GPSLatitude, exif::Tag::GPSLatitudeRef)?;
    let longitude = coord(exif::Tag::GPSLongitude, exif::Tag::GPSLongitudeRef)?;
    if latitude == 0.0 && longitude == 0.0 {
        return None;
    }
    let altitude = rational(exif, exif::Tag::GPSAltitude).map(|a| {
        if uint(exif, exif::Tag::GPSAltitudeRef) == Some(1) {
            -a
        } else {
            a
        }
    });
    Some(GpsPoint {
        latitude,
        longitude,
        altitude,
    })
}

fn exposure_program(v: u32) -> Option<Entry> {
    use MetaValueKey as K;
    Some(match v {
        1 => (K::ProgramManual, "Manuale"),
        2 => (K::ProgramNormal, "Programma"),
        3 => (K::ProgramAperture, "Priorità diaframma"),
        4 => (K::ProgramShutter, "Priorità tempi"),
        _ => return None,
    })
}

fn metering_mode(v: u32) -> Option<Entry> {
    use MetaValueKey as K;
    Some(match v {
        1 => (K::MeteringAverage, "Media"),
        2 => (K::MeteringCenterWeighted, "Media ponderata al centro"),
        3 => (K::MeteringSpot, "Spot"),
        4 => (K::MeteringMultiSpot, "Multi-spot"),
        5 => (K::MeteringMultiSegment, "Multi-zona"),
        6 => (K::MeteringPartial, "Parziale"),
        _ => return None,
    })
}

fn shutter_type(v: i64) -> Option<Entry> {
    use MetaValueKey as K;
    Some(match v {
        0 => (K::ShutterMechanical, "Meccanico"),
        1 => (K::ShutterElectronic, "Elettronico"),
        2 => (K::ShutterElectronicLong, "Elettronico (posa lunga)"),
        3 => (K::ShutterElectronicFront, "Prima tendina elettronica"),
        _ => return None,
    })
}

fn focus_mode(v: i64) -> Option<Entry> {
    use MetaValueKey as K;
    Some(match v {
        0 => (K::FocusAuto, "Automatica"),
        1 => (K::FocusManual, "Manuale"),
        _ => return None,
    })
}

// ---------- Fujifilm MakerNote ----------

/// Integer tags of a FUJIFILM MakerNote ("FUJIFILM" + LE IFD offset; offsets
/// are relative to the MakerNote start).
#[derive(Debug, Default, Clone)]
pub struct MakerNote {
    tags: std::collections::BTreeMap<u16, i64>,
}

impl MakerNote {
    pub fn parse(mn: &[u8]) -> Option<Self> {
        if mn.len() < 12 || &mn[..8] != b"FUJIFILM" {
            return None;
        }
        let u16_at = |o: usize| mn.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
        let u32_at = |o: usize| {
            mn.get(o..o + 4)
                .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        };
        let ifd = u32_at(8)? as usize;
        let count = u16_at(ifd)? as usize;
        let mut tags = std::collections::BTreeMap::new();
        for i in 0..count {
            let e = ifd + 2 + 12 * i;
            let (Some(tag), Some(typ), Some(n)) = (u16_at(e), u16_at(e + 2), u32_at(e + 4)) else {
                break;
            };
            if n != 1 {
                continue;
            }
            let value = match typ {
                1 => mn.get(e + 8).map(|b| *b as i64),
                3 => u16_at(e + 8).map(i64::from),
                8 => u16_at(e + 8).map(|v| v as i16 as i64),
                4 => u32_at(e + 8).map(i64::from),
                9 => u32_at(e + 8).map(|v| v as i32 as i64),
                _ => None,
            };
            if let Some(value) = value {
                tags.insert(tag, value);
            }
        }
        Some(Self { tags })
    }

    pub fn get(&self, tag: u16) -> Option<i64> {
        self.tags.get(&tag).copied()
    }

    pub fn recipe(&self) -> Option<Recipe> {
        let (film, film_key) = film_entry(self.get(0x1401), self.get(0x1003))
            .map_or((None, None), |(text, key)| (Some(text), key));
        let mono = self.get(0x1401).is_none()
            && self.get(0x1003).is_some_and(|s| s >= 0x300 && s != 0x8000);
        let dr = self
            .get(0x1403)
            .filter(|v| *v > 0)
            .map(|v| format!("DR{v}"));
        let wb_entry = self.get(0x1002).and_then(white_balance_entry);
        let wb = wb_entry.map(|(_, text)| text.to_string());
        let mut rows = Vec::new();
        if let Some(text) = &film {
            rows.push(MetaRow {
                label: "filmSimulation".into(),
                value: text.clone(),
                value_key: film_key,
            });
        }
        row(&mut rows, "dynamicRange", dr.clone());
        enum_row(&mut rows, "whiteBalance", wb_entry);
        row(&mut rows, "highlights", self.get(0x1041).map(tone));
        row(&mut rows, "shadows", self.get(0x1040).map(tone));
        if !mono {
            row(&mut rows, "color", self.get(0x1003).and_then(color));
        }
        row(&mut rows, "sharpness", self.get(0x1001).and_then(sharpness));
        row(
            &mut rows,
            "noiseReduction",
            self.get(0x100E).and_then(noise_reduction),
        );
        row(
            &mut rows,
            "clarity",
            self.get(0x100F).map(|v| signed(v as f64 / 1000.0)),
        );
        enum_row(
            &mut rows,
            "grain",
            grain(self.get(0x1047), self.get(0x104C)),
        );
        enum_row(&mut rows, "colorChrome", self.get(0x1048).and_then(effect));
        enum_row(
            &mut rows,
            "colorChromeFxBlue",
            self.get(0x104E).and_then(effect),
        );
        if rows.is_empty() {
            return None;
        }
        let summary = [&film, &dr, &wb]
            .into_iter()
            .flatten()
            .cloned()
            .collect::<Vec<_>>()
            .join(" · ");
        Some(Recipe {
            summary,
            film,
            film_key,
            dynamic_range: dr,
            white_balance: wb,
            white_balance_key: wb_entry.map(|(key, _)| key),
            rows,
        })
    }
}

/// FilmMode (0x1401) for colour; monochrome has no FilmMode and is encoded in
/// Saturation (0x1003).
pub fn film_name(film_mode: Option<i64>, saturation: Option<i64>) -> Option<String> {
    film_entry(film_mode, saturation).map(|(text, _)| text)
}

/// Nome della simulazione e, per monocromatico e seppia, la chiave di
/// traduzione (gli altri sono nomi propri, uguali in ogni lingua).
fn film_entry(
    film_mode: Option<i64>,
    saturation: Option<i64>,
) -> Option<(String, Option<MetaValueKey>)> {
    use MetaValueKey as K;
    let name = match film_mode {
        Some(mode) => match mode {
            0x000 => "Provia",
            0x120 => "Astia",
            0x200 | 0x400 => "Velvia",
            0x500 => "Pro Neg. Std",
            0x501 => "Pro Neg. Hi",
            0x600 => "Classic Chrome",
            0x700 => "Eterna",
            0x800 => "Classic Neg.",
            0x900 => "Eterna Bleach Bypass",
            0xA00 => "Nostalgic Neg.",
            0xB00 => "Reala Ace",
            _ => return Some((format!("Simulazione 0x{mode:X}"), None)),
        },
        None => {
            let (name, key) = match saturation? {
                0x300 => ("Monocromatico", Some(K::FilmMonochrome)),
                0x301 => ("Monocromatico + R", Some(K::FilmMonochromeR)),
                0x302 => ("Monocromatico + Ye", Some(K::FilmMonochromeYe)),
                0x303 => ("Monocromatico + G", Some(K::FilmMonochromeG)),
                0x310 => ("Seppia", Some(K::FilmSepia)),
                0x500 => ("Acros", None),
                0x501 => ("Acros + R", None),
                0x502 => ("Acros + Ye", None),
                0x503 => ("Acros + G", None),
                _ => return None,
            };
            return Some((name.into(), key));
        }
    };
    Some((name.into(), None))
}

pub fn white_balance(v: i64) -> Option<String> {
    white_balance_entry(v).map(|(_, text)| text.into())
}

fn white_balance_entry(v: i64) -> Option<Entry> {
    use MetaValueKey as K;
    Some(match v {
        0x000 => (K::WbAuto, "Auto"),
        0x001 => (K::WbAutoWhite, "Auto (priorità bianco)"),
        0x002 => (K::WbAutoAmbience, "Auto (priorità ambiente)"),
        0x100 => (K::WbDaylight, "Luce diurna"),
        0x200 => (K::WbShade, "Ombra"),
        0x300 => (K::WbFluorescent1, "Fluorescente 1"),
        0x301 => (K::WbFluorescent2, "Fluorescente 2"),
        0x302 => (K::WbFluorescent3, "Fluorescente 3"),
        0x400 => (K::WbIncandescent, "Incandescenza"),
        0x500 => (K::WbFlash, "Flash"),
        0x600 => (K::WbUnderwater, "Subacqueo"),
        0xF00..=0xF0F => (K::WbCustom, "Personalizzato"),
        0xFF0 => (K::WbKelvin, "Temperatura colore (K)"),
        _ => return None,
    })
}

fn signed(v: f64) -> String {
    if v.abs() < 0.01 {
        "0".into()
    } else {
        format!(
            "{}{}",
            if v > 0.0 { "+" } else { "−" },
            trim_float(v.abs(), 1)
        )
    }
}

/// Highlight/shadow tone: −16 per step, so −64 = +4 and 16 = −1.
pub fn tone(v: i64) -> String {
    signed(-(v as f64) / 16.0)
}

pub fn color(v: i64) -> Option<String> {
    Some(
        match v {
            0x000 => "0",
            0x080 => "+1",
            0x100 => "+2",
            0x0C0 => "+3",
            0x0E0 => "+4",
            0x180 => "−1",
            0x200 => "−2",
            0x4C0 => "−3",
            0x4E0 => "−4",
            _ => return None,
        }
        .into(),
    )
}

pub fn sharpness(v: i64) -> Option<String> {
    Some(
        match v {
            0x00 => "−4",
            0x01 => "−3",
            0x02 => "−2",
            0x82 => "−1",
            0x03 => "0",
            0x84 => "+1",
            0x04 => "+2",
            0x05 => "+3",
            0x06 => "+4",
            _ => return None,
        }
        .into(),
    )
}

pub fn noise_reduction(v: i64) -> Option<String> {
    Some(
        match v {
            0x000 => "0",
            0x100 => "+2",
            0x180 => "+1",
            0x1C0 => "+3",
            0x1E0 => "+4",
            0x200 => "−2",
            0x280 => "−1",
            0x2C0 => "−3",
            0x2E0 => "−4",
            _ => return None,
        }
        .into(),
    )
}

fn effect(v: i64) -> Option<Entry> {
    use MetaValueKey as K;
    Some(match v {
        0 => (K::EffectOff, "Off"),
        32 => (K::EffectWeak, "Debole"),
        64 => (K::EffectStrong, "Forte"),
        _ => return None,
    })
}

/// Grana: intensità (0x1047) e dimensione (0x104C), per esempio "Debole · piccola".
fn grain(roughness: Option<i64>, size: Option<i64>) -> Option<Entry> {
    use MetaValueKey as K;
    let strength = effect(roughness?)?;
    Some(match (strength.0, size) {
        (K::EffectWeak, Some(16)) => (K::GrainWeakSmall, "Debole · piccola"),
        (K::EffectWeak, Some(32)) => (K::GrainWeakLarge, "Debole · grande"),
        (K::EffectStrong, Some(16)) => (K::GrainStrongSmall, "Forte · piccola"),
        (K::EffectStrong, Some(32)) => (K::GrainStrongLarge, "Forte · grande"),
        // Off, oppure dimensione assente o sconosciuta: solo l'intensità.
        _ => strength,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn makernote(tags: &[(u16, u16, u32)]) -> Vec<u8> {
        let mut mn = b"FUJIFILM".to_vec();
        mn.extend_from_slice(&12u32.to_le_bytes());
        mn.extend_from_slice(&(tags.len() as u16).to_le_bytes());
        for (tag, typ, value) in tags {
            mn.extend_from_slice(&tag.to_le_bytes());
            mn.extend_from_slice(&typ.to_le_bytes());
            mn.extend_from_slice(&1u32.to_le_bytes());
            mn.extend_from_slice(&value.to_le_bytes());
        }
        mn
    }

    /// Valori della proprietà D192 (FilmSimulation) della fotocamera.
    ///
    /// TODO(task 5.1): copia locale di `camera::banks::film_simulation_name` di
    /// FujiView (a798987, `src-tauri/src/camera/banks.rs`), qui non ancora
    /// portato perché `banks` dipende da `camera::profile` e `camera::ptp`. Con
    /// il porting di `camera/` sostituire la chiamata qui sotto con
    /// `crate::camera::banks::film_simulation_name(film as i32)`, come nel test
    /// originale, e togliere questa funzione.
    fn film_simulation_prop_name(value: i64) -> Option<&'static str> {
        Some(match value {
            1 => "Provia",
            2 => "Velvia",
            3 => "Astia",
            4 => "Pro Neg. Hi",
            5 => "Pro Neg. Std",
            6 => "Monocromatico",
            7 => "Monocromatico + Ye",
            8 => "Monocromatico + R",
            9 => "Monocromatico + G",
            10 => "Seppia",
            11 => "Classic Chrome",
            12 => "Acros",
            13 => "Acros + Ye",
            14 => "Acros + R",
            15 => "Acros + G",
            16 => "Eterna",
            17 => "Classic Neg.",
            18 => "Eterna Bleach Bypass",
            19 => "Nostalgic Neg.",
            20 => "Reala Ace",
            _ => return None,
        })
    }

    #[test]
    fn parses_makernote_and_builds_recipe() {
        let mn = makernote(&[
            (0x1401, 3, 0x600),
            (0x1403, 3, 200),
            (0x1002, 3, 0),
            (0x1041, 9, (-64i32) as u32),
            (0x1040, 9, 32),
            (0x1047, 4, 32),
            (0x104C, 4, 16),
        ]);
        let recipe = MakerNote::parse(&mn).unwrap().recipe().unwrap();
        assert_eq!(recipe.summary, "Classic Chrome · DR200 · Auto");
        let value = |label: &str| {
            recipe
                .rows
                .iter()
                .find(|r| r.label == label)
                .map(|r| r.value.clone())
        };
        assert_eq!(value("highlights").as_deref(), Some("+4"));
        assert_eq!(value("shadows").as_deref(), Some("−2"));
        assert_eq!(value("grain").as_deref(), Some("Debole · piccola"));
        assert!(MakerNote::parse(b"NOTFUJI").is_none());
    }

    #[test]
    fn enumerated_values_carry_a_translation_key() {
        let mn = makernote(&[
            (0x1003, 3, 0x310), // seppia (monocromatico: niente FilmMode)
            (0x1002, 3, 0x100),
            (0x1047, 4, 64),
            (0x104C, 4, 32),
            (0x1048, 4, 0),
            (0x1001, 3, 0x84),
        ]);
        let recipe = MakerNote::parse(&mn).unwrap().recipe().unwrap();
        // Il testo italiano resta come fallback (e come in FujiView).
        assert_eq!(recipe.summary, "Seppia · Luce diurna");
        assert_eq!(recipe.film_key, Some(MetaValueKey::FilmSepia));
        assert_eq!(recipe.white_balance_key, Some(MetaValueKey::WbDaylight));
        let key = |label: &str| {
            let r = recipe.rows.iter().find(|r| r.label == label).unwrap();
            (r.value.as_str(), r.value_key)
        };
        assert_eq!(
            key("filmSimulation"),
            ("Seppia", Some(MetaValueKey::FilmSepia))
        );
        assert_eq!(
            key("grain"),
            ("Forte · grande", Some(MetaValueKey::GrainStrongLarge))
        );
        assert_eq!(key("colorChrome"), ("Off", Some(MetaValueKey::EffectOff)));
        // Numeri e sigle non si traducono.
        assert_eq!(key("sharpness"), ("+1", None));
        // Nomi propri: nessuna chiave.
        let proper = MakerNote::parse(&makernote(&[(0x1401, 3, 0x600)]))
            .unwrap()
            .recipe()
            .unwrap();
        assert_eq!(proper.film.as_deref(), Some("Classic Chrome"));
        assert_eq!(proper.film_key, None);
        // Grana con dimensione sconosciuta: solo l'intensità.
        assert_eq!(
            grain(Some(32), None),
            Some((MetaValueKey::EffectWeak, "Debole"))
        );
    }

    #[test]
    fn every_enumerated_table_entry_has_a_distinct_key_per_text() {
        // Stessa chiave => stesso testo: la UI può tradurre senza ambiguità.
        let mut seen: std::collections::BTreeMap<String, &str> = Default::default();
        let mut check = |entry: Option<Entry>| {
            if let Some((key, text)) = entry {
                let name = serde_json::to_value(key)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_owned();
                let previous = seen.insert(name.clone(), text);
                assert!(previous.is_none_or(|p| p == text), "{name}");
            }
        };
        for v in 0..8 {
            check(exposure_program(v));
            check(metering_mode(v));
        }
        for v in 0..4 {
            check(shutter_type(v));
            check(focus_mode(v));
        }
        for v in 0..0x1000 {
            check(white_balance_entry(v));
        }
        for v in [0, 32, 64] {
            check(effect(v));
            for size in [None, Some(16), Some(32)] {
                check(grain(Some(v), size));
            }
        }
        assert!(seen.len() >= 35, "{}", seen.len());
        // Serializzazione camelCase: è la chiave i18n `rawReviewMeta.values.<chiave>`.
        assert_eq!(
            serde_json::to_value(MetaValueKey::WbFluorescent1).unwrap(),
            "wbFluorescent1"
        );
    }

    /// Every case of the real X-T5 matrix decodes to the settings that produced it.
    #[test]
    fn decodes_real_xt5_makernotes() {
        let text = include_str!("../tests/fixtures/xt5_makernote_matrix.json");
        let matrix: Value = serde_json::from_str(text).unwrap();
        let wb_codes = [
            (2, 0x000),
            (4, 0x100),
            (6, 0x400),
            (0x8001, 0x300),
            (0x8002, 0x301),
            (0x8003, 0x302),
            (0x8006, 0x200),
            (0x8020, 0x001),
            (0x8021, 0x002),
        ];
        for case in matrix["cases"].as_array().unwrap() {
            let mn = &case["makernote"];
            let get = |tag: &str| mn[tag].as_i64();
            let props = &case["props"];
            if let Some(film) = props["D192"].as_i64() {
                let expected = film_simulation_prop_name(film).unwrap();
                let got = film_name(get("0x1401"), get("0x1003")).unwrap();
                assert_eq!(
                    got.replace("Monocromatico", "Mono"),
                    expected.replace("Monocromatico", "Mono"),
                    "{}",
                    case["name"]
                );
            }
            if let Some(wb) = props["D199"].as_i64() {
                if let Some((_, code)) = wb_codes.iter().find(|(prop, _)| *prop == wb) {
                    assert_eq!(get("0x1002"), Some(*code), "{}", case["name"]);
                    assert!(white_balance(*code).is_some());
                }
            }
            for tag in ["0x1002", "0x1001", "0x100E"] {
                if let Some(v) = get(tag) {
                    let decoded = match tag {
                        "0x1002" => white_balance(v),
                        "0x1001" => sharpness(v),
                        _ => noise_reduction(v),
                    };
                    assert!(
                        decoded.is_some(),
                        "{} {tag}={v:#x} not decoded",
                        case["name"]
                    );
                }
            }
        }
        let case = |name: &str| {
            matrix["cases"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["name"] == name)
                .unwrap()["makernote"]
                .clone()
        };
        let tones = case("tone_h40_s-20");
        assert_eq!(tone(tones["0x1041"].as_i64().unwrap()), "+4");
        assert_eq!(tone(tones["0x1040"].as_i64().unwrap()), "−2");
        assert_eq!(
            color(case("color-40_clar-50")["0x1003"].as_i64().unwrap()).as_deref(),
            Some("−4")
        );
        assert_eq!(
            sharpness(case("sharp_20")["0x1001"].as_i64().unwrap()).as_deref(),
            Some("+2")
        );
        assert_eq!(
            sharpness(case("sharp_-40")["0x1001"].as_i64().unwrap()).as_deref(),
            Some("−4")
        );
    }

    #[test]
    fn formats_exposure_values() {
        assert_eq!(format_exposure(1.0 / 250.0), "1/250 s");
        assert_eq!(format_exposure(2.0), "2 s");
        assert_eq!(format_exposure(0.5), "0,5 s");
        assert_eq!(format_bias(1.0 / 3.0), "+0,3 EV");
        assert_eq!(format_bias(-2.0 / 3.0), "−0,7 EV");
        assert_eq!(format_bias(0.0), "0 EV");
    }
}

#[cfg(test)]
mod file_tests {
    use super::*;
    use exif::{experimental::Writer, Field, In, Tag, Value};
    use pc_core::error::ErrorCode;
    use std::io::Cursor;

    /// TIFF con EXIF di prova e un MakerNote Fujifilm (Classic Chrome, DR200).
    fn exif_tiff() -> Vec<u8> {
        let mut mn = b"FUJIFILM".to_vec();
        mn.extend_from_slice(&12u32.to_le_bytes());
        mn.extend_from_slice(&2u16.to_le_bytes());
        for (tag, value) in [(0x1401u16, 0x600u32), (0x1403, 200)] {
            mn.extend_from_slice(&tag.to_le_bytes());
            mn.extend_from_slice(&3u16.to_le_bytes());
            mn.extend_from_slice(&1u32.to_le_bytes());
            mn.extend_from_slice(&value.to_le_bytes());
        }
        let field = |tag, value| Field {
            tag,
            ifd_num: In::PRIMARY,
            value,
        };
        let fields = [
            field(Tag::Make, Value::Ascii(vec![b"FUJIFILM".to_vec()])),
            field(Tag::Model, Value::Ascii(vec![b"X-T5".to_vec()])),
            field(
                Tag::DateTimeOriginal,
                Value::Ascii(vec![b"2026:11:12 15:30:01".to_vec()]),
            ),
            field(
                Tag::ExposureTime,
                Value::Rational(vec![exif::Rational { num: 1, denom: 250 }]),
            ),
            field(Tag::MakerNote, Value::Undefined(mn, 0)),
        ];
        let mut writer = Writer::new();
        for f in &fields {
            writer.push_field(f);
        }
        let mut out = Cursor::new(Vec::new());
        writer.write(&mut out, true).unwrap();
        out.into_inner()
    }

    fn jpeg_with_exif() -> Vec<u8> {
        let tiff = exif_tiff();
        let mut app1 = b"Exif\0\0".to_vec();
        app1.extend_from_slice(&tiff);
        let mut jpeg = vec![0xFF, 0xD8, 0xFF, 0xE1];
        jpeg.extend_from_slice(&((app1.len() + 2) as u16).to_be_bytes());
        jpeg.extend_from_slice(&app1);
        jpeg.extend_from_slice(&[0xFF, 0xD9]);
        jpeg
    }

    fn raf_with(jpeg: Option<&[u8]>) -> Vec<u8> {
        let mut raf = vec![0u8; 0x5C];
        raf[..16].copy_from_slice(b"FUJIFILMCCD-RAW ");
        let mut body = vec![0u8; 0x100 - 0x5C];
        if let Some(jpeg) = jpeg {
            raf[0x54..0x58].copy_from_slice(&0x100u32.to_be_bytes());
            raf[0x58..0x5C].copy_from_slice(&(jpeg.len() as u32).to_be_bytes());
            body.extend_from_slice(jpeg);
        }
        raf.extend_from_slice(&body);
        raf
    }

    fn value<'a>(rows: &'a [MetaRow], label: &str) -> Option<&'a str> {
        rows.iter()
            .find(|r| r.label == label)
            .map(|r| r.value.as_str())
    }

    fn section<'a>(meta: &'a ImageMetadata, title: &str) -> Option<&'a MetaSection> {
        meta.sections.iter().find(|s| s.title == title)
    }

    fn check_full(meta: &ImageMetadata) {
        let shot = section(meta, "shot").unwrap();
        assert_eq!(value(&shot.rows, "dateTime"), Some("12/11/2026 15:30:01"));
        assert_eq!(value(&shot.rows, "exposureTime"), Some("1/250 s"));
        let camera = section(meta, "camera").unwrap();
        assert_eq!(value(&camera.rows, "camera"), Some("FUJIFILM X-T5"));
        let recipe = meta.recipe.as_ref().unwrap();
        assert_eq!(recipe.summary, "Classic Chrome · DR200");
        assert_eq!(recipe.film.as_deref(), Some("Classic Chrome"));
        assert_eq!(recipe.dynamic_range.as_deref(), Some("DR200"));
        assert_eq!(recipe.white_balance, None);
        assert!(section(meta, "file").is_some());
    }

    #[test]
    fn reads_tiff() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.tif");
        fs::write(&path, exif_tiff()).unwrap();
        check_full(&read_metadata(&path).unwrap());
    }

    #[test]
    fn reads_jpeg() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.jpg");
        fs::write(&path, jpeg_with_exif()).unwrap();
        check_full(&read_metadata(&path).unwrap());
    }

    #[test]
    fn reads_exif_from_raf_embedded_jpeg() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("DSCF0001.RAF");
        fs::write(&path, raf_with(Some(&jpeg_with_exif()))).unwrap();
        let meta = read_metadata(&path).unwrap();
        check_full(&meta);
        let file = section(&meta, "file").unwrap();
        assert_eq!(value(&file.rows, "name"), Some("DSCF0001.RAF"));
        assert_eq!(value(&file.rows, "dimensions"), None);
    }

    #[test]
    fn raf_without_readable_exif_gives_file_section_only() {
        let dir = tempfile::tempdir().unwrap();
        for (name, bytes) in [
            ("senza.RAF", raf_with(None)),
            ("rotto.RAF", raf_with(Some(b"non e un jpeg"))),
            ("troncato.RAF", b"FUJIFILMCCD-RAW 0201".to_vec()),
        ] {
            let path = dir.path().join(name);
            fs::write(&path, bytes).unwrap();
            let meta = read_metadata(&path).unwrap();
            assert_eq!(meta.sections.len(), 1, "{name}");
            assert_eq!(meta.sections[0].title, "file");
            assert!(meta.recipe.is_none() && meta.gps.is_none());
        }
    }

    #[test]
    fn missing_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let err = read_metadata(&dir.path().join("manca.jpg")).unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
    }

    #[test]
    fn json_shape_is_camel_case() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.jpg");
        fs::write(&path, jpeg_with_exif()).unwrap();
        let json = serde_json::to_value(read_metadata(&path).unwrap()).unwrap();
        assert_eq!(json["sections"][0]["title"], "shot");
        assert_eq!(json["recipe"]["dynamicRange"], "DR200");
        assert!(json["recipe"]["filmKey"].is_null());
        assert!(json["sections"][0]["rows"][0]["valueKey"].is_null());
        assert!(json["gps"].is_null());
    }
}
