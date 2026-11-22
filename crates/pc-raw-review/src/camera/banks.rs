//! Camera custom setting banks (C1..C7) and their translation into
//! conversion-profile cells.

use super::profile::{cell, Profile, FILE_TYPE_JPEG, WB_SOURCE_PROFILE};
use super::ptp::PtpResult;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Bank property → profile cell, as observed in X RAW Studio conversions
/// (fujifilm-ptp-reference, doc 07). Must be validated on the X-T5 (task 5.3).
pub const BANK_TO_CELL: &[(u16, usize, &str)] = &[
    (0xD190, cell::DYNAMIC_RANGE, "gamma dinamica"),
    (0xD191, cell::DR_PRIORITY, "priorità gamma dinamica"),
    (0xD192, cell::FILM_SIMULATION, "simulazione pellicola"),
    (0xD193, cell::MONO_WARM_COOL, "monocromatico caldo/freddo"),
    (
        0xD194,
        cell::MONO_MAGENTA_GREEN,
        "monocromatico magenta/verde",
    ),
    (0xD195, cell::GRAIN, "grana"),
    (0xD196, cell::COLOR_CHROME, "color chrome"),
    (0xD197, cell::COLOR_CHROME_FX_BLUE, "color chrome FX blu"),
    (0xD198, cell::SMOOTH_SKIN, "pelle levigata"),
    (0xD199, cell::WB_MODE, "bilanciamento del bianco"),
    (0xD19A, cell::WB_SHIFT_R, "scostamento WB rosso"),
    (0xD19B, cell::WB_SHIFT_B, "scostamento WB blu"),
    (0xD19C, cell::WB_KELVIN, "temperatura colore"),
    (0xD19D, cell::HIGHLIGHT_TONE, "toni alti"),
    (0xD19E, cell::SHADOW_TONE, "toni bassi"),
    (0xD19F, cell::COLOR, "colore"),
    (0xD1A0, cell::SHARPNESS, "nitidezza"),
    (0xD1A1, cell::NOISE_REDUCTION, "riduzione rumore"),
    (0xD1A2, cell::CLARITY, "chiarezza"),
    (0xD1A4, cell::COLOR_SPACE, "spazio colore"),
];

/// Bank properties with no known profile cell.
pub fn property_label(code: u16) -> String {
    match code {
        0xD18E => "dimensione immagine (0xD18E)".into(),
        0xD18F => "qualità immagine (0xD18F)".into(),
        0xD1A3 => "proprietà 0xD1A3 (non mappata)".into(),
        _ => BANK_TO_CELL
            .iter()
            .find(|(c, _, _)| *c == code)
            .map(|(_, _, name)| format!("{name} (0x{code:04X})"))
            .unwrap_or_else(|| format!("proprietà 0x{code:04X}")),
    }
}

/// Properties that are deliberately taken from the RAF, not the bank.
const KEEP_AS_SHOT: &[u16] = &[0xD18E, 0xD18F];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bank {
    pub index: u8,
    pub name: String,
    /// Raw property values, sign-extended from their PTP data.
    pub props: BTreeMap<u16, i32>,
}

impl Bank {
    pub fn film_simulation(&self) -> Option<i32> {
        self.props.get(&0xD192).copied()
    }

    /// Stable fingerprint of the bank settings, used to keep variants made
    /// with different settings out of the same folder.
    pub fn fingerprint(&self) -> String {
        let text: Vec<String> = self
            .props
            .iter()
            .map(|(k, v)| format!("{k:04X}={v}"))
            .collect();
        blake3::hash(format!("{}|{}", self.name, text.join(";")).as_bytes()).to_hex()[..16]
            .to_string()
    }

    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        if let Some(film) = self.film_simulation() {
            parts.push(
                film_simulation_name(film)
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("simulazione {film}")),
            );
        }
        if let Some(dr) = self.props.get(&0xD190) {
            if matches!(dr, 100 | 200 | 400) {
                parts.push(format!("DR{dr}"));
            }
        }
        if parts.is_empty() {
            "impostazioni del banco".into()
        } else {
            parts.join(" · ")
        }
    }
}

/// Decode a PTP property value (u8/i16/i32 payloads) as a signed integer.
pub fn decode_value(data: &[u8]) -> Option<i32> {
    match data.len() {
        1 => Some(data[0] as i32),
        2 => Some(i16::from_le_bytes([data[0], data[1]]) as i32),
        4.. => Some(i32::from_le_bytes([data[0], data[1], data[2], data[3]])),
        _ => None,
    }
}

/// Bank properties whose 16-bit value is a code or bit field, not a signed
/// number. Verified on the X-T5: noise reduction 0x8000 reaches the
/// conversion profile as 32768, and white balance holds codes such as 0x8020
/// (auto, white priority) and 0x8021 (auto, ambience priority).
const UNSIGNED_BANK_PROPS: [u16; 3] = [0xD199, 0xD19C, 0xD1A1];

/// Decode a custom-bank property value with the signedness the conversion
/// profile expects.
pub fn decode_bank_value(code: u16, data: &[u8]) -> Option<i32> {
    if data.len() == 2 && UNSIGNED_BANK_PROPS.contains(&code) {
        return Some(u16::from_le_bytes([data[0], data[1]]) as i32);
    }
    decode_value(data)
}

pub fn film_simulation_name(value: i32) -> Option<&'static str> {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedBank {
    pub profile: Profile,
    pub unapplied: Vec<String>,
}

/// Apply a bank on top of the as-shot profile read after the RAF upload.
pub fn apply_bank(base: &Profile, bank: &Bank) -> PtpResult<AppliedBank> {
    let mut profile = base.clone();
    let mut unapplied = Vec::new();
    profile.set(cell::FILE_TYPE, FILE_TYPE_JPEG)?;
    for (code, value) in &bank.props {
        if KEEP_AS_SHOT.contains(code) {
            continue;
        }
        let Some((_, target, _)) = BANK_TO_CELL.iter().find(|(c, _, _)| c == code) else {
            if !is_inert(*code, *value) {
                unapplied.push(property_label(*code));
            }
            continue;
        };
        if *target >= profile.cell_count() {
            unapplied.push(property_label(*code));
            continue;
        }
        let mut value = *value;
        if *target == cell::DYNAMIC_RANGE {
            // A DR wider than the one the RAF was exposed for renders badly
            // (reported on the X-T50): never exceed the as-shot value.
            if let Some(shot) = base
                .get(cell::DYNAMIC_RANGE)
                .filter(|v| matches!(v, 100 | 200 | 400))
            {
                if matches!(value, 100 | 200 | 400) && value > shot {
                    unapplied.push(format!(
                        "gamma dinamica limitata a DR{shot} (valore dello scatto)"
                    ));
                    value = shot;
                }
            }
        }
        profile.set(*target, value)?;
    }
    if bank.props.contains_key(&0xD199) {
        profile.set(cell::WB_SOURCE, WB_SOURCE_PROFILE)?;
    }
    Ok(AppliedBank { profile, unapplied })
}

/// Unmapped bank properties that do not change the result: 0xD1A3 (unknown;
/// 1 on every X-T5 bank seen so far).
fn is_inert(code: u16, value: i32) -> bool {
    matches!((code, value), (0xD1A3, 1))
}

/// Profile for the "Come scattato" variant.
pub fn as_shot(base: &Profile) -> PtpResult<Profile> {
    let mut profile = base.clone();
    profile.set(cell::FILE_TYPE, FILE_TYPE_JPEG)?;
    Ok(profile)
}

/// Nome della cartella di un banco, `C<n> <nome>`, valido su Windows, Linux e
/// macOS: i caratteri vietati da Windows (`< > : " / \ | ? *`) e i caratteri di
/// controllo diventano `-`; spazi e punti finali (rifiutati da Windows) cadono;
/// senza nome resta `C<n>`. Il prefisso `C<n> ` evita anche i nomi riservati
/// (CON, NUL, ...).
pub fn folder_name(index: u8, name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_control() || r#"<>:"/\|?*"#.contains(c) {
                '-'
            } else {
                c
            }
        })
        .collect();
    let cleaned = cleaned.trim().trim_end_matches(['.', ' ']).to_string();
    if cleaned.is_empty() {
        format!("C{index}")
    } else {
        format!("C{index} {cleaned}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Profile {
        let mut cells = vec![0i32; 29];
        cells[cell::DYNAMIC_RANGE] = 200;
        cells[cell::FILM_SIMULATION] = 1;
        cells[cell::WB_SOURCE] = 1;
        Profile::synthetic("FF179502", &cells)
    }

    fn bank(props: &[(u16, i32)]) -> Bank {
        Bank {
            index: 1,
            name: "Kodachrome 64".into(),
            props: props.iter().copied().collect(),
        }
    }

    #[test]
    fn folder_names_are_windows_safe() {
        assert_eq!(folder_name(3, "Neg/Hi: 2"), "C3 Neg-Hi- 2");
        assert_eq!(folder_name(1, "Kodachrome 64"), "C1 Kodachrome 64");
        assert_eq!(folder_name(7, "   "), "C7");
        assert_eq!(folder_name(2, "Velvia."), "C2 Velvia");
    }

    #[test]
    fn folder_names_are_valid_on_every_platform() {
        // Ogni carattere vietato da Windows diventa `-`.
        assert_eq!(
            folder_name(4, r#"a<b>c:d"e/f\g|h?i*j"#),
            "C4 a-b-c-d-e-f-g-h-i-j"
        );
        // Caratteri di controllo, spazi e punti finali.
        assert_eq!(folder_name(5, "Cl\u{0}as\tsic. . "), "C5 Cl-as-sic");
        assert_eq!(folder_name(6, ".."), "C6");
        // Nomi riservati di Windows: il prefisso `C<n> ` li rende validi.
        assert_eq!(folder_name(1, "CON"), "C1 CON");
        // Unicode e spazi interni restano.
        assert_eq!(folder_name(2, "Pellicola è bella"), "C2 Pellicola è bella");
        // Nessun separatore di percorso in uscita, nemmeno con nomi di percorso.
        for name in ["../x", r"a\b", r"C:\x", "NUL"] {
            let folder = folder_name(1, name);
            assert!(!folder.contains('/') && !folder.contains('\\'), "{folder}");
            assert!(!folder.ends_with('.') && !folder.ends_with(' '), "{folder}");
        }
    }

    #[test]
    fn applies_mapped_cells_and_reports_the_rest() {
        let applied = apply_bank(
            &base(),
            &bank(&[
                (0xD192, 11),
                (0xD195, 2),
                (0xD199, 3),
                (0xD18E, 1),
                (0xD1A3, 5),
                (0xD191, 0),
            ]),
        )
        .unwrap();
        assert_eq!(applied.profile.get(cell::FILM_SIMULATION), Some(11));
        assert_eq!(applied.profile.get(cell::GRAIN), Some(2));
        assert_eq!(applied.profile.get(cell::DR_PRIORITY), Some(0));
        assert_eq!(applied.profile.get(cell::WB_MODE), Some(3));
        assert_eq!(
            applied.profile.get(cell::WB_SOURCE),
            Some(WB_SOURCE_PROFILE)
        );
        assert_eq!(applied.profile.get(cell::FILE_TYPE), Some(FILE_TYPE_JPEG));
        // 0xD1A3 = 5 is reported; D-range priority (0xD191) is mapped.
        assert_eq!(applied.unapplied.len(), 1, "{:?}", applied.unapplied);
        assert!(applied.unapplied[0].contains("0xD1A3"));
    }

    #[test]
    fn dynamic_range_never_exceeds_as_shot() {
        let applied = apply_bank(&base(), &bank(&[(0xD190, 400)])).unwrap();
        assert_eq!(applied.profile.get(cell::DYNAMIC_RANGE), Some(200));
        assert_eq!(applied.unapplied.len(), 1);
        let applied = apply_bank(&base(), &bank(&[(0xD190, 100)])).unwrap();
        assert_eq!(applied.profile.get(cell::DYNAMIC_RANGE), Some(100));
    }

    #[test]
    fn base_profile_is_not_modified() {
        let original = base();
        let _ = apply_bank(&original, &bank(&[(0xD192, 17)])).unwrap();
        assert_eq!(original.get(cell::FILM_SIMULATION), Some(1));
    }

    #[test]
    fn fingerprint_tracks_settings() {
        let a = bank(&[(0xD192, 11)]);
        let b = bank(&[(0xD192, 12)]);
        assert_ne!(a.fingerprint(), b.fingerprint());
        assert_eq!(a.fingerprint(), a.clone().fingerprint());
        assert_eq!(a.summary(), "Classic Chrome");
    }

    #[test]
    fn decodes_signed_values() {
        assert_eq!(decode_value(&(-20i16).to_le_bytes()), Some(-20));
        assert_eq!(decode_value(&400u16.to_le_bytes()), Some(400));
        assert_eq!(decode_value(&[]), None);
        assert_eq!(
            decode_bank_value(0xD1A1, &0x8000u16.to_le_bytes()),
            Some(32768)
        );
        assert_eq!(
            decode_bank_value(0xD199, &0x8020u16.to_le_bytes()),
            Some(0x8020)
        );
        assert_eq!(decode_bank_value(0xD19B, &(-5i16).to_le_bytes()), Some(-5));
    }

    #[test]
    fn real_banks_fixture_when_available() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/xt5_banks.json");
        if let Ok(text) = std::fs::read_to_string(path) {
            let banks: Vec<Bank> = serde_json::from_str(&text).unwrap();
            for bank in &banks {
                apply_bank(&base(), bank).unwrap();
            }
        }
    }

    /// Real X-T5 data (fw 4.31): as-shot profile + the user's C1..C7 banks.
    /// Expected cells were checked against the MakerNotes of the JPEGs the
    /// camera produced with each profile.
    #[test]
    fn real_xt5_banks_produce_verified_cells() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/");
        let (Ok(raw), Ok(text)) = (
            std::fs::read(format!("{dir}xt5_profile_asshot.bin")),
            std::fs::read_to_string(format!("{dir}xt5_banks.json")),
        ) else {
            return;
        };
        let base = Profile::parse(&raw).unwrap();
        let banks: Vec<Bank> = serde_json::from_str(&text).unwrap();
        let by_name = |name: &str| banks.iter().find(|b| b.name == name).unwrap();

        // C3 VIVID: Velvia, FX Blue weak, auto WB, H -1 / S -1, colour +4.
        let c3 = apply_bank(&base, by_name("VIVID")).unwrap();
        let get = |p: &Profile, c| p.get(c).unwrap();
        assert_eq!(get(&c3.profile, cell::FILM_SIMULATION), 2);
        assert_eq!(get(&c3.profile, cell::COLOR_CHROME_FX_BLUE), 2);
        assert_eq!(get(&c3.profile, cell::WB_MODE), 2);
        assert_eq!(get(&c3.profile, cell::HIGHLIGHT_TONE), -10);
        assert_eq!(get(&c3.profile, cell::SHADOW_TONE), -10);
        assert_eq!(get(&c3.profile, cell::COLOR), 40);
        assert!(c3.unapplied.is_empty(), "{:?}", c3.unapplied);

        // C1 DEFAULT: NR and WB codes must stay unsigned.
        let c1 = apply_bank(&base, by_name("DEFAULT")).unwrap();
        assert_eq!(get(&c1.profile, cell::NOISE_REDUCTION), 0x2000);
        assert_eq!(get(&c1.profile, cell::WB_MODE), 0x8020);

        // C6 VIVID FAST: DR400 on a DR200 shot is limited and reported.
        let c6 = apply_bank(&base, by_name("VIVID FAST")).unwrap();
        assert_eq!(get(&c6.profile, cell::DYNAMIC_RANGE), 200);
        assert_eq!(c6.unapplied.len(), 1, "{:?}", c6.unapplied);
    }

    /// Cases converted on a real X-T5 and checked against the MakerNotes of
    /// the resulting JPEGs (tests/fixtures/xt5_makernote_matrix.json): every
    /// bank property must land in the cell the camera actually honours.
    #[test]
    fn real_xt5_makernote_matrix() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/");
        let (Ok(raw), Ok(banks), Ok(matrix)) = (
            std::fs::read(format!("{dir}xt5_profile_asshot.bin")),
            std::fs::read_to_string(format!("{dir}xt5_banks.json")),
            std::fs::read_to_string(format!("{dir}xt5_makernote_matrix.json")),
        ) else {
            return;
        };
        let base = Profile::parse(&raw).unwrap();
        let banks: Vec<Bank> = serde_json::from_str(&banks).unwrap();
        let matrix: serde_json::Value = serde_json::from_str(&matrix).unwrap();
        let cases = matrix["cases"].as_array().unwrap();
        assert!(cases.len() >= 50);
        for case in cases {
            let name = case["name"].as_str().unwrap();
            let mut bank = banks
                .iter()
                .find(|b| b.name == case["bank"].as_str().unwrap())
                .unwrap()
                .clone();
            for (code, value) in case["props"].as_object().unwrap() {
                let code = u16::from_str_radix(code.trim_start_matches('D'), 16).unwrap() | 0xD000;
                bank.props.insert(code, value.as_i64().unwrap() as i32);
            }
            let applied = apply_bank(&base, &bank).unwrap();
            for (index, value) in case["expectCells"].as_object().unwrap() {
                let index: usize = index.parse().unwrap();
                assert_eq!(
                    applied.profile.get(index),
                    Some(value.as_i64().unwrap() as i32),
                    "{name}: cella {index}"
                );
            }
        }
    }
}
