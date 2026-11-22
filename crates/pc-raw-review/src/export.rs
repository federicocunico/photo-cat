//! Esportazione delle varianti scelte: selezione degli scatti da esportare.
//!
//! La copia dei file, l'import e il catalogo vivono nel job `raw_review_export` di
//! `pc-organize`, che lavora su un elenco di [`ExportItem`] già deciso qui. Questo
//! modulo non scrive nulla: legge l'indice della scansione e le decisioni della
//! sessione e applica la regola "solo JPEG scelti" (mai RAF, TIFF né l'anteprima RAW).

use crate::scan::{is_exportable_jpeg, ReviewIndex};
use crate::session::Session;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Filtri scelti nel dialogo di esportazione.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ExportFilter {
    /// Valutazione minima (0 = tutte le scelte, fino a 5).
    #[serde(default)]
    pub min_rating: u8,
    /// Solo le varianti scelte di questi preset; `None` = tutti.
    #[serde(default)]
    pub presets: Option<Vec<String>>,
}

/// Un file da esportare: la variante JPEG scelta di uno scatto.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ExportItem {
    /// Chiave dello scatto nella sessione.
    pub key: String,
    /// Percorso del JPEG scelto (mai modificato).
    pub source_path: String,
    /// Nome del file di partenza, con estensione (`DSCF0042.JPG`).
    pub orig_name: String,
    /// Preset della variante scelta.
    pub preset: String,
    /// Valutazione della revisione (0-5).
    pub rating: u8,
}

/// Esito della selezione, con i motivi di esclusione per il dialogo.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ExportSelection {
    /// Scatti da esportare (dopo valutazione minima e preset), in ordine di lista.
    pub items: Vec<ExportItem>,
    /// Scatti esportabili per valutazione 0-5 (indice = stelle), dopo il filtro per
    /// preset e prima della valutazione minima: il dialogo li mostra mentre si sceglie.
    pub by_rating: Vec<u32>,
    /// Scatti senza una variante scelta.
    pub no_choice: u32,
    /// Scatti la cui variante scelta non è un JPEG (per esempio un TIFF).
    pub not_jpeg: u32,
    /// Scatti la cui variante scelta non esiste più nell'indice o sul disco.
    pub missing: u32,
}

impl ExportSelection {
    /// Scatti che verranno esportati.
    pub fn total(&self) -> u32 {
        u32::try_from(self.items.len()).unwrap_or(u32::MAX)
    }
}

/// Sceglie gli scatti da esportare. La variante scelta deve appartenere all'indice della
/// sessione (la scelta salvata può riferirsi a un preset sparito, per esempio un disco di
/// varianti scollegato), esistere sul disco ed essere un JPEG.
pub fn select(index: &ReviewIndex, session: &Session, filter: &ExportFilter) -> ExportSelection {
    let mut out = ExportSelection {
        by_rating: vec![0; 6],
        ..Default::default()
    };
    for shot in &index.shots {
        let decision = session.decisions.get(&shot.key);
        let Some(winner) = decision.and_then(|d| d.winner_preset.as_deref()) else {
            out.no_choice += 1;
            continue;
        };
        let Some(variant) = shot.variants.iter().find(|v| v.preset == winner) else {
            out.missing += 1;
            continue;
        };
        if !is_exportable_jpeg(variant) {
            out.not_jpeg += 1;
            continue;
        }
        if !Path::new(&variant.path).is_file() {
            out.missing += 1;
            continue;
        }
        if let Some(presets) = &filter.presets {
            if !presets.iter().any(|p| p == winner) {
                continue;
            }
        }
        let rating = decision.map_or(0, |d| d.rating.min(5));
        out.by_rating[usize::from(rating)] += 1;
        if rating >= filter.min_rating {
            out.items.push(ExportItem {
                key: shot.key.clone(),
                source_path: variant.path.clone(),
                orig_name: variant.file_name.clone(),
                preset: variant.preset.clone(),
                rating,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::scan;
    use std::fs;

    /// Cartella per preset: `<preset>/<scatto>.<ext>` per ogni coppia.
    fn build(root: &Path, files: &[(&str, &str)]) -> ReviewIndex {
        for (preset, name) in files {
            fs::create_dir_all(root.join(preset)).unwrap();
            fs::write(root.join(preset).join(name), format!("{preset}/{name}")).unwrap();
        }
        scan(root, None).unwrap()
    }

    fn decide(session: &mut Session, key: &str, preset: Option<&str>, rating: u8) {
        if let Some(p) = preset {
            session.set_winner(key, Some(p)).unwrap();
        }
        session.set_rating(key, rating).unwrap();
    }

    #[test]
    fn min_rating_keeps_only_matching_chosen_shots() {
        let tmp = tempfile::tempdir().unwrap();
        let names: Vec<String> = (1..=40).map(|i| format!("DSCF{i:04}.JPG")).collect();
        let files: Vec<(&str, &str)> = names.iter().map(|n| ("C1", n.as_str())).collect();
        let index = build(tmp.path(), &files);
        let mut session = Session::create(&index);
        for (i, shot) in index.shots.iter().enumerate() {
            // 12 scatti con 4 o 5 stelle, gli altri 28 con meno.
            let rating = if i < 12 {
                4 + (i % 2) as u8
            } else {
                (i % 4) as u8
            };
            decide(&mut session, &shot.key, Some("C1"), rating);
        }
        let all = select(&index, &session, &ExportFilter::default());
        assert_eq!(all.total(), 40);
        let top = select(
            &index,
            &session,
            &ExportFilter {
                min_rating: 4,
                presets: None,
            },
        );
        assert_eq!(top.total(), 12);
        assert_eq!(top.by_rating.iter().sum::<u32>(), 40);
        assert_eq!(top.by_rating[4] + top.by_rating[5], 12);
    }

    #[test]
    fn rated_shots_without_choice_are_excluded_and_counted() {
        let tmp = tempfile::tempdir().unwrap();
        let names: Vec<String> = (1..=30).map(|i| format!("DSCF{i:04}.JPG")).collect();
        let files: Vec<(&str, &str)> = names.iter().map(|n| ("C1", n.as_str())).collect();
        let index = build(tmp.path(), &files);
        let mut session = Session::create(&index);
        for (i, shot) in index.shots.iter().enumerate() {
            // I primi 20 hanno solo una valutazione, gli ultimi 10 anche la scelta.
            decide(&mut session, &shot.key, (i >= 20).then_some("C1"), 3);
        }
        let selection = select(&index, &session, &ExportFilter::default());
        assert_eq!(selection.total(), 10);
        assert_eq!(selection.no_choice, 20);
    }

    #[test]
    fn tiff_choices_are_excluded_and_counted() {
        let tmp = tempfile::tempdir().unwrap();
        let index = build(
            tmp.path(),
            &[
                ("C1", "DSCF0001.JPG"),
                ("C1", "DSCF0002.JPG"),
                ("C1", "DSCF0003.JPG"),
                ("C1", "DSCF0004.JPG"),
                ("Velvia", "DSCF0001.tif"),
                ("Velvia", "DSCF0002.tif"),
                ("Velvia", "DSCF0003.tiff"),
            ],
        );
        let mut session = Session::create(&index);
        for shot in &index.shots {
            let preset = if shot.key == "dscf0004" {
                "C1"
            } else {
                "Velvia"
            };
            decide(&mut session, &shot.key, Some(preset), 2);
        }
        let selection = select(&index, &session, &ExportFilter::default());
        assert_eq!(selection.not_jpeg, 3);
        assert_eq!(selection.total(), 1);
        assert_eq!(selection.items[0].orig_name, "DSCF0004.JPG");
        assert!(selection.items.iter().all(|i| {
            let ext = Path::new(&i.source_path).extension().unwrap();
            ext.eq_ignore_ascii_case("jpg") || ext.eq_ignore_ascii_case("jpeg")
        }));
    }

    #[test]
    fn raf_and_raw_preview_are_never_selected() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("DSCF0001.RAF"), b"raf").unwrap();
        fs::create_dir_all(tmp.path().join("FujiView varianti").join("C1")).unwrap();
        fs::write(
            tmp.path()
                .join("FujiView varianti")
                .join("C1")
                .join("DSCF0001.JPG"),
            b"jpg",
        )
        .unwrap();
        let index = scan(tmp.path(), None).unwrap();
        let mut session = Session::create(&index);
        decide(&mut session, "dscf0001", Some("C1"), 5);
        let selection = select(&index, &session, &ExportFilter::default());
        assert_eq!(selection.total(), 1);
        assert!(selection.items[0]
            .source_path
            .to_lowercase()
            .ends_with("dscf0001.jpg"));
    }

    #[test]
    fn choice_on_a_vanished_preset_or_file_is_excluded() {
        let tmp = tempfile::tempdir().unwrap();
        let index = build(
            tmp.path(),
            &[("C1", "DSCF0001.JPG"), ("C1", "DSCF0002.JPG")],
        );
        let mut session = Session::create(&index);
        decide(&mut session, "dscf0001", Some("C1"), 3);
        decide(&mut session, "dscf0002", Some("C1"), 3);
        // Preset non più nell'indice (disco scollegato) e file sparito dal disco.
        session.decisions.get_mut("dscf0001").unwrap().winner_preset = Some("C9".into());
        fs::remove_file(tmp.path().join("C1").join("DSCF0002.JPG")).unwrap();
        let selection = select(&index, &session, &ExportFilter::default());
        assert_eq!(selection.total(), 0);
        assert_eq!(selection.missing, 2);
    }

    #[test]
    fn preset_filter_limits_selection_and_counts() {
        let tmp = tempfile::tempdir().unwrap();
        let index = build(
            tmp.path(),
            &[
                ("C1", "DSCF0001.JPG"),
                ("C2", "DSCF0001.JPG"),
                ("C1", "DSCF0002.JPG"),
                ("C2", "DSCF0002.JPG"),
            ],
        );
        let mut session = Session::create(&index);
        decide(&mut session, "dscf0001", Some("C1"), 5);
        decide(&mut session, "dscf0002", Some("C2"), 4);
        let selection = select(
            &index,
            &session,
            &ExportFilter {
                min_rating: 0,
                presets: Some(vec!["C2".into()]),
            },
        );
        assert_eq!(selection.total(), 1);
        assert_eq!(selection.items[0].preset, "C2");
        assert_eq!(selection.by_rating[4], 1);
        assert_eq!(selection.by_rating[5], 0);
    }
}
