//! Modello del nome dei file esportati: testo e token.
//!
//! Token: `{seq}`, `{seq:0Nd}` (N da 1 a 9), `{date}` (`AAAA-MM-GG`), `{time}`
//! (`HHMM`), `{time_full}` (`HHMMSS`), `{datetime}` (`AAAA-MM-GG_HHMMSS`),
//! `{tag}`, `{device}`, `{people}`, `{place}`, `{orig}`. Un token vuoto elimina
//! anche il separatore (`_`, `-`, spazio, `.`) che lo precede. I nomi sono
//! validi su Windows e Linux; i nomi uguali ricevono `-2`, `-3` in ordine.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Caratteri non ammessi nei nomi di file su Windows (Linux vieta solo `/`).
const RESERVED_CHARS: &str = "<>:\"/\\|?*";
/// Separatori che spariscono insieme a un token vuoto.
const SEPARATORS: &[char] = &['_', '-', ' ', '.'];
/// Lunghezza massima del nome (senza estensione), in caratteri.
const MAX_LEN: usize = 180;

pub const TOKENS: &[&str] = &[
    "seq",
    "seq:04d",
    "date",
    "time",
    "time_full",
    "datetime",
    "tag",
    "device",
    "people",
    "place",
    "orig",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Token {
    Seq { width: Option<u8> },
    Date,
    Time,
    TimeFull,
    DateTime,
    Tag,
    Device,
    People,
    Place,
    Orig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Part {
    Lit(String),
    Tok(Token),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    parts: Vec<Part>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum TemplateErrorKind {
    UnknownToken,
    InvalidChar,
    Unclosed,
    Empty,
}

/// Problema nel modello, con la posizione (in caratteri) da evidenziare.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct TemplateError {
    pub kind: TemplateErrorKind,
    pub start: u32,
    pub end: u32,
    pub text: String,
}

fn token(name: &str) -> Option<Token> {
    Some(match name {
        "seq" => Token::Seq { width: None },
        "date" => Token::Date,
        "time" => Token::Time,
        "time_full" => Token::TimeFull,
        "datetime" => Token::DateTime,
        "tag" => Token::Tag,
        "device" => Token::Device,
        "people" => Token::People,
        "place" => Token::Place,
        "orig" => Token::Orig,
        other => {
            // {seq:0Nd}
            let n = other.strip_prefix("seq:0")?.strip_suffix('d')?;
            let w: u8 = n.parse().ok()?;
            if !(1..=9).contains(&w) || n.len() != 1 {
                return None;
            }
            Token::Seq { width: Some(w) }
        }
    })
}

/// Analizza il modello; restituisce tutti i problemi trovati.
pub fn parse(template: &str) -> Result<Template, Vec<TemplateError>> {
    let chars: Vec<char> = template.chars().collect();
    let mut parts = Vec::new();
    let mut errors = Vec::new();
    let mut lit = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '{' {
            let Some(close) = chars[i..].iter().position(|x| *x == '}').map(|p| p + i) else {
                errors.push(TemplateError {
                    kind: TemplateErrorKind::Unclosed,
                    start: i as u32,
                    end: chars.len() as u32,
                    text: chars[i..].iter().collect(),
                });
                break;
            };
            let name: String = chars[i + 1..close].iter().collect();
            match token(name.trim()) {
                Some(t) => {
                    if !lit.is_empty() {
                        parts.push(Part::Lit(std::mem::take(&mut lit)));
                    }
                    parts.push(Part::Tok(t));
                }
                None => errors.push(TemplateError {
                    kind: TemplateErrorKind::UnknownToken,
                    start: i as u32,
                    end: close as u32 + 1,
                    text: format!("{{{name}}}"),
                }),
            }
            i = close + 1;
            continue;
        }
        if c.is_control() || RESERVED_CHARS.contains(c) || c == '}' {
            errors.push(TemplateError {
                kind: TemplateErrorKind::InvalidChar,
                start: i as u32,
                end: i as u32 + 1,
                text: c.to_string(),
            });
        } else {
            lit.push(c);
        }
        i += 1;
    }
    if !lit.is_empty() {
        parts.push(Part::Lit(lit));
    }
    if errors.is_empty() && parts.is_empty() {
        errors.push(TemplateError {
            kind: TemplateErrorKind::Empty,
            start: 0,
            end: 0,
            text: String::new(),
        });
    }
    if errors.is_empty() {
        Ok(Template { parts })
    } else {
        Err(errors)
    }
}

/// Dati di una foto per il nome.
#[derive(Debug, Clone, Default)]
pub struct NameInput<'a> {
    /// Progressivo da 1, in ordine di istante reale.
    pub seq: usize,
    /// Ora locale del gruppo di sorgente, `AAAA-MM-GG HH:MM:SS`.
    pub local: &'a str,
    pub tag: Option<&'a str>,
    pub device: Option<&'a str>,
    /// Nomi brevi delle identità presenti.
    pub people: &'a [String],
    pub place: Option<&'a str>,
    /// Nome originale senza estensione.
    pub orig: &'a str,
}

/// Valore di un token reso sicuro come parte di un nome di file.
pub fn clean_value(s: &str) -> String {
    let out: String = s
        .chars()
        .map(|c| {
            if c.is_control() || RESERVED_CHARS.contains(c) {
                '-'
            } else {
                c
            }
        })
        .collect();
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

impl Template {
    fn value(&self, t: Token, n: &NameInput) -> String {
        let date = n.local.get(0..10).unwrap_or("");
        let hm: String = n.local.get(11..16).unwrap_or("").replace(':', "");
        let hms: String = n.local.get(11..19).unwrap_or("").replace(':', "");
        match t {
            Token::Seq { width: None } => n.seq.to_string(),
            Token::Seq { width: Some(w) } => format!("{:0width$}", n.seq, width = w as usize),
            Token::Date => date.to_string(),
            Token::Time => hm,
            Token::TimeFull => hms,
            Token::DateTime if date.is_empty() => String::new(),
            Token::DateTime => format!("{date}_{hms}"),
            Token::Tag => n.tag.map(clean_value).unwrap_or_default(),
            Token::Device => n.device.map(clean_value).unwrap_or_default(),
            Token::People => n
                .people
                .iter()
                .map(|p| clean_value(p))
                .filter(|p| !p.is_empty())
                .collect::<Vec<_>>()
                .join("-"),
            Token::Place => n.place.map(clean_value).unwrap_or_default(),
            Token::Orig => clean_value(n.orig),
        }
    }

    /// Nome senza estensione (non ancora reso univoco).
    pub fn render(&self, n: &NameInput) -> String {
        let mut out = String::new();
        // Lunghezza dell'ultimo letterale scritto in coda (per togliere il separatore).
        let mut lit_tail = 0usize;
        for p in &self.parts {
            match p {
                Part::Lit(s) => {
                    out.push_str(s);
                    lit_tail = s.len();
                }
                Part::Tok(t) => {
                    let v = self.value(*t, n);
                    if v.is_empty() {
                        if lit_tail > 0 && out.ends_with(SEPARATORS) {
                            out.pop();
                        }
                    } else {
                        out.push_str(&v);
                    }
                    lit_tail = 0;
                }
            }
        }
        sanitize(&out)
    }

    pub fn uses(&self, t: Token) -> bool {
        self.parts.contains(&Part::Tok(t))
    }
}

/// Nome valido su Windows e Linux: niente separatori o punti in testa/coda,
/// nomi riservati (`CON`, `COM1`...) evitati, lunghezza limitata.
pub fn sanitize(name: &str) -> String {
    let trimmed = name.trim_matches(|c: char| SEPARATORS.contains(&c));
    let mut s: String = trimmed.chars().take(MAX_LEN).collect();
    s = s.trim_end_matches(['.', ' ']).to_string();
    if s.is_empty() {
        s = "foto".into();
    }
    let stem = s.split('.').next().unwrap_or("").to_ascii_uppercase();
    let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.len() == 4
            && stem.as_bytes()[3].is_ascii_digit());
    if reserved {
        s.push('_');
    }
    s
}

/// Rende univoci i nomi (senza distinzione di maiuscole, come su Windows)
/// aggiungendo `-2`, `-3` nell'ordine dato, e aggiunge l'estensione.
pub fn finalize(names: &[String], ext: &str) -> Vec<String> {
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut taken: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut out = Vec::with_capacity(names.len());
    for n in names {
        let key = n.to_lowercase();
        let count = seen.entry(key.clone()).or_insert(0);
        *count += 1;
        let mut candidate = if *count == 1 {
            n.clone()
        } else {
            format!("{n}-{count}")
        };
        // Un nome con suffisso potrebbe coincidere con un nome già scritto.
        let mut k = *count;
        while !taken.insert(format!("{}.{ext}", candidate.to_lowercase())) {
            k += 1;
            candidate = format!("{n}-{k}");
        }
        out.push(format!("{candidate}.{ext}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input<'a>(seq: usize, local: &'a str, tag: Option<&'a str>) -> NameInput<'a> {
        NameInput {
            seq,
            local,
            tag,
            device: Some("Fujifilm X-T5"),
            people: &[],
            place: Some("Moab"),
            orig: "DSCF0042",
        }
    }

    #[test]
    fn sequence_date_time_and_tag() {
        let t = parse("{seq:04d}_{date}_{time}_{tag}").unwrap();
        let n = t.render(&input(1, "2026-08-19 10:43:12", Some("Paesaggi")));
        assert_eq!(
            finalize(&[n], "jpg"),
            vec!["0001_2026-08-19_1043_Paesaggi.jpg"]
        );
    }

    #[test]
    fn empty_token_drops_the_preceding_separator() {
        let t = parse("{seq:04d}_{date}_{time}_{tag}").unwrap();
        assert_eq!(
            t.render(&input(5, "2026-08-19 10:52:00", None)),
            "0005_2026-08-19_1052"
        );
        let t = parse("{tag}-{seq}").unwrap();
        assert_eq!(t.render(&input(7, "2026-08-19 10:52:00", None)), "7");
        let t = parse("{people} {orig}").unwrap();
        assert_eq!(t.render(&input(1, "", None)), "DSCF0042");
    }

    #[test]
    fn every_token() {
        let people = vec!["Giulia".to_string(), "Luca".to_string()];
        let n = NameInput {
            seq: 12,
            local: "2026-08-19 10:43:07",
            tag: Some("Città"),
            device: Some("Apple iPhone 17 Pro Max"),
            people: &people,
            place: Some("Moab"),
            orig: "IMG_0042",
        };
        let r = |s: &str| parse(s).unwrap().render(&n);
        assert_eq!(r("{seq}"), "12");
        assert_eq!(r("{seq:06d}"), "000012");
        assert_eq!(r("{date}"), "2026-08-19");
        assert_eq!(r("{time}"), "1043");
        assert_eq!(r("{time_full}"), "104307");
        assert_eq!(r("{datetime}"), "2026-08-19_104307");
        assert_eq!(r("{tag}"), "Città");
        assert_eq!(r("{device}"), "Apple iPhone 17 Pro Max");
        assert_eq!(r("{people}"), "Giulia-Luca");
        assert_eq!(r("{place}"), "Moab");
        assert_eq!(r("{orig}"), "IMG_0042");
    }

    #[test]
    fn unknown_tokens_and_invalid_characters_are_reported() {
        let e = parse("{seq}_{camera}_{date}").unwrap_err();
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].kind, TemplateErrorKind::UnknownToken);
        assert_eq!(e[0].text, "{camera}");
        assert_eq!((e[0].start, e[0].end), (6, 14));
        let e = parse("{seq}:{date}/x").unwrap_err();
        assert_eq!(
            e.iter()
                .map(|x| (x.kind, x.text.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (TemplateErrorKind::InvalidChar, ":"),
                (TemplateErrorKind::InvalidChar, "/")
            ]
        );
        assert_eq!(
            parse("{seq").unwrap_err()[0].kind,
            TemplateErrorKind::Unclosed
        );
        assert_eq!(parse("").unwrap_err()[0].kind, TemplateErrorKind::Empty);
        assert!(parse("{seq:010d}").is_err());
        assert!(parse("{seq:00d}").is_err());
    }

    #[test]
    fn values_and_names_are_safe_on_windows_and_linux() {
        let t = parse("{tag}").unwrap();
        assert_eq!(t.render(&input(1, "", Some("A/B: \"c\"?"))), "A-B- -c");
        assert_eq!(sanitize("CON"), "CON_");
        assert_eq!(sanitize("com3.foto"), "com3.foto_");
        assert_eq!(sanitize("COM10"), "COM10");
        assert_eq!(sanitize("nome. . "), "nome");
        assert_eq!(sanitize("__"), "foto");
        assert!(sanitize(&"x".repeat(400)).chars().count() <= MAX_LEN);
    }

    #[test]
    fn duplicate_names_get_deterministic_suffixes() {
        let names = vec![
            "2026-08-19_1043".to_string(),
            "2026-08-19_1043".to_string(),
            "2026-08-19_1044".to_string(),
            "2026-08-19_1043".to_string(),
            "2026-08-19_1043-2".to_string(),
        ];
        assert_eq!(
            finalize(&names, "jpg"),
            vec![
                "2026-08-19_1043.jpg",
                "2026-08-19_1043-2.jpg",
                "2026-08-19_1044.jpg",
                "2026-08-19_1043-3.jpg",
                "2026-08-19_1043-2-2.jpg",
            ]
        );
        // Maiuscole diverse: stesso nome su Windows.
        assert_eq!(
            finalize(&["Foto".into(), "foto".into()], "jpg"),
            vec!["Foto.jpg", "foto-2.jpg"]
        );
    }
}
