//! Nomi dei file esportati dalle Varianti RAW (file fuori libreria).
//!
//! Usa il motore di [`super::naming`] nel contesto [`NameContext::Review`]: solo
//! i token ricavabili dai file più `{preset}`. L'ordine di `{seq}` è quello di
//! FujiView (`sequence_names`): data e ora di scatto (`DateTimeOriginal` con i
//! sottosecondi, altrimenti `DateTime`), poi nome in ordine naturale; i file
//! senza data vanno in fondo, ordinati per nome. L'estensione resta quella del
//! file; i nomi uguali ricevono `-2`, `-3` come nella Sequenza.
//!
//! La lettura della data di scatto vive qui (e non in `pc-raw-review`) perché
//! `pc-organize` ha già `kamadak-exif` e `pc-ingest`, e così il crate delle
//! Varianti RAW non entra nelle dipendenze dell'export della Sequenza.

use super::job::NamePreview;
use super::naming::{self, NameContext, NameInput, TemplateError};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Un file da rinominare. Costruito una volta (la lettura EXIF costa) e poi
/// riusato per l'anteprima a ogni modifica del modello.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReviewNameInput {
    /// Nome del file di partenza, con estensione (`DSCF0042.JPG`).
    pub orig_name: String,
    /// Nome del preset della variante (`{preset}`).
    pub preset: Option<String>,
    /// Istante di scatto `AAAA:MM:GG HH:MM:SS[.sss]` (come EXIF). Valori non
    /// validi equivalgono a "senza data".
    pub capture: Option<String>,
    /// Dispositivo leggibile (`{device}`), es. "Fujifilm X-T5".
    pub device: Option<String>,
}

impl ReviewNameInput {
    /// Legge data di scatto e dispositivo dall'EXIF del file (mai scritto).
    /// Un file illeggibile o senza EXIF produce un ingresso senza data.
    /// La lettura passa da una cache per (percorso, dimensione, data di modifica):
    /// il dialogo dell'export ricalcola i nomi a ogni modifica delle opzioni.
    pub fn from_file(path: &Path, orig_name: &str, preset: Option<&str>) -> Self {
        let (capture, device) = cached_file_facts(path);
        Self {
            orig_name: orig_name.to_string(),
            preset: preset.map(str::to_string),
            capture,
            device,
        }
    }
}

/// Voci massime della cache dei dati EXIF (oltre si svuota e riparte).
const FACTS_CACHE_LIMIT: usize = 20_000;

type Facts = (Option<String>, Option<String>);
type FactsKey = (std::path::PathBuf, u64, Option<std::time::SystemTime>);

fn facts_cache() -> &'static std::sync::Mutex<HashMap<FactsKey, Facts>> {
    static CACHE: std::sync::OnceLock<std::sync::Mutex<HashMap<FactsKey, Facts>>> =
        std::sync::OnceLock::new();
    CACHE.get_or_init(Default::default)
}

/// Come [`read_file_facts`], ma rilegge il file solo se dimensione o data di
/// modifica sono cambiate. Un file che non si apre non entra nella cache.
pub fn cached_file_facts(path: &Path) -> Facts {
    let Ok(meta) = std::fs::metadata(path) else {
        return (None, None);
    };
    let key: FactsKey = (path.to_path_buf(), meta.len(), meta.modified().ok());
    if let Some(hit) = facts_cache().lock().ok().and_then(|c| c.get(&key).cloned()) {
        return hit;
    }
    let facts = read_file_facts(path);
    if let Ok(mut cache) = facts_cache().lock() {
        if cache.len() >= FACTS_CACHE_LIMIT {
            cache.clear();
        }
        cache.insert(key, facts.clone());
    }
    facts
}

/// Chiave di scatto ordinabile e dispositivo letti dall'EXIF di `path`.
pub fn read_file_facts(path: &Path) -> (Option<String>, Option<String>) {
    let Ok(file) = File::open(path) else {
        return (None, None);
    };
    let Ok(data) = exif::Reader::new().read_from_container(&mut BufReader::new(file)) else {
        return (None, None);
    };
    let text = |tag: exif::Tag| {
        data.get_field(tag, exif::In::PRIMARY)
            .and_then(|f| match &f.value {
                exif::Value::Ascii(parts) => parts
                    .first()
                    .map(|p| {
                        String::from_utf8_lossy(p)
                            .trim_matches(['\0', ' '])
                            .to_string()
                    })
                    .filter(|v| !v.is_empty()),
                _ => None,
            })
    };
    let capture = text(exif::Tag::DateTimeOriginal)
        .or_else(|| text(exif::Tag::DateTime))
        .map(|base| {
            let sub = text(exif::Tag::SubSecTimeOriginal).unwrap_or_default();
            format!("{base}.{sub}")
        });
    let device = pc_ingest::metadata::normalize_device(
        text(exif::Tag::Make).as_deref(),
        text(exif::Tag::Model).as_deref(),
    );
    (capture, device)
}

/// Chiave ordinabile `AAAA:MM:GG HH:MM:SS.sss` e ora locale `AAAA-MM-GG HH:MM:SS`
/// da un valore EXIF; `None` se la data non è valida (anche `0000:00:00`).
fn normalize_capture(raw: &str) -> Option<(String, String)> {
    let (base, sub) = match raw.split_once('.') {
        Some((b, s)) => (b, s),
        None => (raw, ""),
    };
    let (date, time) = base.trim().split_once(' ')?;
    let num = |s: &str, len: usize| -> Option<u32> {
        (s.len() == len && s.bytes().all(|b| b.is_ascii_digit()))
            .then(|| s.parse().ok())
            .flatten()
    };
    let mut d = date.split(':');
    let (y, mo, da) = (d.next()?, d.next()?, d.next()?);
    let mut t = time.split(':');
    let (h, mi, se) = (t.next()?, t.next()?, t.next()?);
    if d.next().is_some() || t.next().is_some() {
        return None;
    }
    let (yn, mon, day) = (num(y, 4)?, num(mo, 2)?, num(da, 2)?);
    let (hn, min, sec) = (num(h, 2)?, num(mi, 2)?, num(se, 2)?);
    if yn == 0 || !(1..=12).contains(&mon) || !(1..=31).contains(&day) {
        return None;
    }
    if hn > 23 || min > 59 || sec > 60 {
        return None;
    }
    let sub: String = sub.chars().filter(char::is_ascii_digit).collect();
    let key = format!("{y}:{mo}:{da} {h}:{mi}:{se}.{sub:0<3}");
    let local = format!("{y}-{mo}-{da} {h}:{mi}:{se}");
    Some((key, local))
}

fn stem_and_ext(name: &str) -> (String, String) {
    let p = Path::new(name);
    let stem = p
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ext = p
        .extension()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    (stem, ext)
}

/// Indici degli ingressi in ordine di `{seq}`: scatto, poi nome naturale; i
/// file senza data in fondo, per nome. A parità l'ordine di ingresso resta.
fn sequence_order(items: &[ReviewNameInput], keys: &[Option<(String, String)>]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..items.len()).collect();
    order.sort_by(|&x, &y| {
        let by_name = || natord::compare(&items[x].orig_name, &items[y].orig_name);
        match (&keys[x], &keys[y]) {
            (Some(a), Some(b)) => a.0.cmp(&b.0).then_with(by_name),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => by_name(),
        }
    });
    order
}

/// Nomi finali (con estensione) nello stesso ordine di `items`, più l'ordine
/// di sequenza degli indici.
fn plan(items: &[ReviewNameInput], template: &naming::Template) -> (Vec<String>, Vec<usize>) {
    let keys: Vec<Option<(String, String)>> = items
        .iter()
        .map(|i| i.capture.as_deref().and_then(normalize_capture))
        .collect();
    let order = sequence_order(items, &keys);
    let split: Vec<(String, String)> = items.iter().map(|i| stem_and_ext(&i.orig_name)).collect();
    let mut bases = vec![String::new(); items.len()];
    for (pos, &i) in order.iter().enumerate() {
        bases[i] = template.render(&NameInput {
            seq: pos + 1,
            local: keys[i].as_ref().map_or("", |k| k.1.as_str()),
            tag: None,
            device: items[i].device.as_deref(),
            people: &[],
            place: None,
            orig: &split[i].0,
            preset: items[i].preset.as_deref(),
        });
    }
    // L'unicità vale per nome completo (senza distinguere le maiuscole, anche
    // nell'estensione); l'estensione scritta è quella originale del file.
    let mut by_ext: HashMap<String, Vec<usize>> = HashMap::new();
    for &i in &order {
        by_ext.entry(split[i].1.to_lowercase()).or_default().push(i);
    }
    let mut out = vec![String::new(); items.len()];
    for (ext, idx) in by_ext {
        let group: Vec<String> = idx.iter().map(|i| bases[*i].clone()).collect();
        for (k, name) in naming::finalize(&group, &ext).into_iter().enumerate() {
            let i = idx[k];
            let stem = name
                .strip_suffix(&format!(".{ext}"))
                .unwrap_or(&name)
                .to_string();
            out[i] = if split[i].1.is_empty() {
                stem
            } else {
                format!("{stem}.{}", split[i].1)
            };
        }
    }
    (out, order)
}

/// Nomi finali di tutti i file, nello stesso ordine di `items`. Errore con i
/// problemi del modello (token sconosciuti o non disponibili, caratteri non
/// validi) senza produrre alcun nome.
pub fn plan_review_names(
    items: &[ReviewNameInput],
    template: &str,
) -> Result<Vec<String>, Vec<TemplateError>> {
    let t = naming::parse_for(template, NameContext::Review)?;
    Ok(plan(items, &t).0)
}

/// Anteprima dei primi `n` nomi in ordine di sequenza (quindi di scatto), per
/// il dialogo dell'export mentre si scrive il modello.
pub fn preview_review_names(
    items: &[ReviewNameInput],
    template: &str,
    n: usize,
) -> Result<Vec<NamePreview>, Vec<TemplateError>> {
    let t = naming::parse_for(template, NameContext::Review)?;
    let (names, order) = plan(items, &t);
    Ok(order
        .into_iter()
        .take(n)
        .map(|i| NamePreview {
            original: items[i].orig_name.clone(),
            name: names[i].clone(),
        })
        .collect())
}

/// Messaggio unico per i problemi del modello (errore di avvio del job).
pub fn template_errors_message(errors: &[TemplateError]) -> String {
    format!(
        "modello del nome non valido: {}",
        errors
            .iter()
            .map(|e| e.text.clone())
            .collect::<Vec<_>>()
            .join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::naming::TemplateErrorKind;
    use super::*;

    fn shot(name: &str, time: Option<&str>, preset: Option<&str>) -> ReviewNameInput {
        ReviewNameInput {
            orig_name: name.into(),
            preset: preset.map(str::to_string),
            capture: time.map(|t| format!("2026:08:19 {t}")),
            device: Some("Fujifilm X-T5".into()),
        }
    }

    #[test]
    fn numbering_follows_capture_time() {
        let items = vec![
            shot("DSCF0001.jpg", Some("10:02:00"), None),
            shot("DSCF0002.jpg", Some("09:15:00"), None),
            shot("DSCF0003.jpg", Some("11:40:00"), None),
        ];
        assert_eq!(
            plan_review_names(&items, "Dolomiti_{seq:04d}").unwrap(),
            vec![
                "Dolomiti_0002.jpg",
                "Dolomiti_0001.jpg",
                "Dolomiti_0003.jpg"
            ]
        );
        // L'anteprima segue l'ordine di scatto.
        let p = preview_review_names(&items, "Dolomiti_{seq:04d}", 2).unwrap();
        assert_eq!(
            p.iter().map(|x| x.name.as_str()).collect::<Vec<_>>(),
            vec!["Dolomiti_0001.jpg", "Dolomiti_0002.jpg"]
        );
        assert_eq!(p[0].original, "DSCF0002.jpg");
    }

    #[test]
    fn subseconds_then_natural_name_break_ties() {
        let mut a = shot("DSCF10.jpg", Some("10:00:00"), None);
        a.capture = Some("2026:08:19 10:00:00.5".into());
        let mut b = shot("DSCF2.jpg", Some("10:00:00"), None);
        b.capture = Some("2026:08:19 10:00:00.25".into());
        let c = shot("DSCF9.jpg", Some("10:00:00"), None);
        let d = shot("DSCF10.jpg", Some("10:00:00"), None);
        // ".000" < ".25" < ".5"; a parità di istante, ordine naturale (9 < 10).
        let names = plan_review_names(&[a, b, c, d], "{seq}").unwrap();
        assert_eq!(names, vec!["4.jpg", "3.jpg", "1.jpg", "2.jpg"]);
    }

    #[test]
    fn files_without_a_date_go_last_by_name() {
        let items = vec![
            shot("b.jpg", None, None),
            shot("z.jpg", Some("08:00:00"), None),
            shot("a.jpg", None, None),
            ReviewNameInput {
                capture: Some("0000:00:00 00:00:00".into()),
                ..shot("c.jpg", None, None)
            },
            ReviewNameInput {
                capture: Some("non una data".into()),
                ..shot("d.jpg", None, None)
            },
        ];
        assert_eq!(
            plan_review_names(&items, "{seq}").unwrap(),
            vec!["3.jpg", "1.jpg", "2.jpg", "4.jpg", "5.jpg"]
        );
        // Senza data, {date} è vuoto e porta via il separatore.
        assert_eq!(
            plan_review_names(&items[2..3], "x_{date}").unwrap(),
            vec!["x.jpg"]
        );
    }

    #[test]
    fn preset_is_sanitized_and_tokens_come_from_the_files() {
        let items = vec![shot(
            "DSCF0042.JPG",
            Some("09:15:07"),
            Some("C1 Kodachrome/64: \"v2\"?"),
        )];
        assert_eq!(
            plan_review_names(
                &items,
                "{datetime}_{time}_{time_full}_{device}_{orig}_{preset}"
            )
            .unwrap(),
            // Estensione conservata com'è (maiuscola).
            vec!["2026-08-19_091507_0915_091507_Fujifilm X-T5_DSCF0042_C1 Kodachrome-64- -v2.JPG"]
        );
        // Preset assente: il token vuoto toglie il separatore.
        let none = vec![shot("a.jpg", Some("09:15:07"), None)];
        assert_eq!(
            plan_review_names(&none, "{seq:03d}_{preset}").unwrap(),
            vec!["001.jpg"]
        );
    }

    #[test]
    fn equal_names_get_deterministic_suffixes() {
        let items = vec![
            shot("a.jpg", Some("09:00:00"), Some("Provia")),
            shot("b.jpg", Some("09:00:01"), Some("Provia")),
            shot("c.JPG", Some("09:00:02"), Some("Provia")),
            shot("d.jpeg", Some("09:00:03"), Some("Provia")),
        ];
        assert_eq!(
            plan_review_names(&items, "{preset}").unwrap(),
            vec!["Provia.jpg", "Provia-2.jpg", "Provia-3.JPG", "Provia.jpeg"]
        );
    }

    #[test]
    fn library_tokens_and_invalid_templates_are_rejected_before_start() {
        let items = vec![shot("a.jpg", Some("09:00:00"), None)];
        let e = plan_review_names(&items, "{tag}_{seq}").unwrap_err();
        assert_eq!(e[0].kind, TemplateErrorKind::UnavailableToken);
        assert_eq!(e[0].text, "{tag}");
        assert!(template_errors_message(&e).contains("{tag}"));
        for t in ["{people}", "{place}"] {
            assert_eq!(
                plan_review_names(&items, t).unwrap_err()[0].kind,
                TemplateErrorKind::UnavailableToken
            );
        }
        let e = plan_review_names(&items, "a/b:{seq}").unwrap_err();
        assert!(e.iter().all(|x| x.kind == TemplateErrorKind::InvalidChar));
        assert_eq!(e.len(), 2);
        assert_eq!(
            plan_review_names(&items, "{camera}").unwrap_err()[0].kind,
            TemplateErrorKind::UnknownToken
        );
        assert!(preview_review_names(&items, "{tag}", 5).is_err());
    }

    #[test]
    fn preset_is_rejected_in_the_library_context() {
        assert_eq!(
            naming::parse("{seq}_{preset}").unwrap_err()[0].kind,
            TemplateErrorKind::UnavailableToken
        );
    }

    #[test]
    fn reads_capture_time_and_device_from_exif_without_touching_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("DSCF0001.jpg");
        let meta = crate::sequence::metadata::CopyMeta {
            local: Some("2026-08-19 09:15:07".into()),
            make: Some("FUJIFILM".into()),
            model: Some("X-T5".into()),
            ..Default::default()
        };
        let segments = crate::sequence::metadata::app_segments(&meta).unwrap();
        let rgb = image::RgbImage::new(8, 8);
        let bytes = crate::sequence::encode::encode_jpeg(&rgb, 80, false, None, &segments).unwrap();
        std::fs::write(&path, &bytes).unwrap();
        let input = ReviewNameInput::from_file(&path, "DSCF0001.jpg", Some("Provia"));
        assert_eq!(input.capture.as_deref(), Some("2026:08:19 09:15:07."));
        assert_eq!(input.device.as_deref(), Some("Fujifilm X-T5"));
        assert_eq!(
            plan_review_names(&[input], "{date}_{time}_{device}").unwrap(),
            vec!["2026-08-19_0915_Fujifilm X-T5.jpg"]
        );
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        // File illeggibile: nessuna data, nessun errore.
        let missing = ReviewNameInput::from_file(&dir.path().join("nope.jpg"), "nope.jpg", None);
        assert_eq!((missing.capture, missing.device), (None, None));
    }

    #[test]
    fn cached_facts_follow_size_and_modification_time() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("DSCF0002.jpg");
        let write = |local: &str| {
            let meta = crate::sequence::metadata::CopyMeta {
                local: Some(local.into()),
                ..Default::default()
            };
            let segments = crate::sequence::metadata::app_segments(&meta).unwrap();
            let rgb = image::RgbImage::new(8, 8);
            let bytes =
                crate::sequence::encode::encode_jpeg(&rgb, 80, false, None, &segments).unwrap();
            std::fs::write(&path, bytes).unwrap();
        };
        write("2026-08-19 09:15:07");
        let first = cached_file_facts(&path).0;
        assert_eq!(first.as_deref(), Some("2026:08:19 09:15:07."));
        assert_eq!(cached_file_facts(&path).0, first, "dalla cache");
        // Stessa dimensione, altro contenuto e altra data di modifica: si rilegge.
        write("2026-08-19 11:40:00");
        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(10);
        std::fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(later)
            .unwrap();
        assert_eq!(
            cached_file_facts(&path).0.as_deref(),
            Some("2026:08:19 11:40:00.")
        );
    }
}
