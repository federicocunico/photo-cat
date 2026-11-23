//! Soglie del raggruppamento tarate sulla libreria, senza etichette (design D4).
//!
//! Due volti della stessa foto sono sempre persone diverse: i loro coseni stimano la coda alta
//! della distribuzione delle coppie di sconosciuti. Con almeno [`MIN_PAIRS`] coppie:
//! - "unisci da solo" è il quantile 1 − tasso obiettivo (10⁻³ con la sensibilità predefinita),
//!   mai sotto il valore predefinito del modello;
//! - "chiedi" è il quantile 1 − 10⁻², mai sotto la soglia di somiglianza impostata.
//!
//! Le coppie di volti confermati della stessa persona servono solo da controllo: se più del
//! 20% supera appena "chiedi" (resta sotto "unisci da solo"), Diagnostica segnala soglie
//! probabilmente troppo severe; le soglie non si abbassano da sole.

use crate::aggregate::{Face, Params};
use pc_core::AiThresholds;
use pc_store::people::cosine;
use std::collections::BTreeMap;

/// Coppie della stessa foto necessarie per tarare le soglie.
pub const MIN_PAIRS: usize = 200;
/// Tasso di coppie di sconosciuti sopra "chiedi".
pub const ASK_RATE: f64 = 1e-2;
/// Quota di coppie confermate appena sopra "chiedi" oltre cui le soglie sono troppo severe.
pub const TOO_STRICT_SHARE: f32 = 0.2;
/// Volti confermati per persona usati nel controllo (i primi per id: risultato stabile).
const CONFIRMED_PER_PERSON: usize = 40;
/// Distanza minima tra "chiedi" e "unisci da solo".
const ASK_GAP: f32 = 0.05;

/// Esito della taratura.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Calibration {
    pub params: Params,
    /// `true` se la libreria ha spostato almeno una soglia dal valore predefinito.
    pub calibrated: bool,
    pub same_photo_pairs: usize,
    pub confirmed_pairs: usize,
    pub too_strict: bool,
    pub sensitivity: f32,
}

/// Tasso obiettivo di unioni sbagliate per la sensibilità (0 → 10⁻⁴, 0,5 → 10⁻³, 1 → 10⁻²).
pub fn target_rate(sensitivity: f32) -> f64 {
    10f64.powf(-4.0 + 2.0 * f64::from(sensitivity.clamp(0.0, 1.0)))
}

/// Valore predefinito di "unisci da solo" per la sensibilità: 0,55 dal centro verso "Più unioni
/// automatiche" (mai più basso), fino a 0,65 verso "Più domande".
pub fn default_auto(sensitivity: f32) -> f32 {
    AiThresholds::DEFAULT_FACE_AUTO_MERGE + 0.2 * (0.5 - sensitivity.clamp(0.0, 1.0)).max(0.0)
}

/// Quantile `q` (0..1) di valori ordinati: il più piccolo valore con almeno `q·n` valori ≤.
pub fn quantile(sorted: &[f32], q: f64) -> Option<f32> {
    if sorted.is_empty() {
        return None;
    }
    let n = sorted.len();
    let rank = ((q * n as f64).ceil() as usize).clamp(1, n);
    Some(sorted[rank - 1])
}

/// Coseni di tutte le coppie di volti della stessa foto e dello stesso modello, ordinati.
pub fn same_photo_scores(faces: &[Face]) -> Vec<f32> {
    let mut by_photo: BTreeMap<(i64, &str), Vec<usize>> = BTreeMap::new();
    for (i, f) in faces.iter().enumerate() {
        if !f.embedding.is_empty() {
            by_photo.entry((f.media, &f.model)).or_default().push(i);
        }
    }
    let mut out = Vec::new();
    for ids in by_photo.values() {
        for (k, a) in ids.iter().enumerate() {
            for b in &ids[k + 1..] {
                out.push(cosine(&faces[*a].embedding, &faces[*b].embedding));
            }
        }
    }
    out.sort_by(|a, b| a.total_cmp(b));
    out
}

/// Coseni tra volti confermati della stessa persona (al massimo [`CONFIRMED_PER_PERSON`] volti
/// per persona), stesso modello.
pub fn confirmed_scores(faces: &[Face], confirmed: &[bool]) -> Vec<f32> {
    let mut by_person: BTreeMap<(i64, &str), Vec<usize>> = BTreeMap::new();
    for (i, f) in faces.iter().enumerate() {
        if let (Some(p), true) = (f.person, confirmed.get(i).copied().unwrap_or(false)) {
            let v = by_person.entry((p, &f.model)).or_default();
            if v.len() < CONFIRMED_PER_PERSON {
                v.push(i);
            }
        }
    }
    let mut out = Vec::new();
    for ids in by_person.values() {
        for (k, a) in ids.iter().enumerate() {
            for b in &ids[k + 1..] {
                out.push(cosine(&faces[*a].embedding, &faces[*b].embedding));
            }
        }
    }
    out
}

/// Soglie per la libreria: `similarity` è la soglia di somiglianza impostata (minimo di
/// "chiedi"), `sensitivity` la sensibilità dell'unione automatica.
pub fn calibrate(
    faces: &[Face],
    confirmed: &[bool],
    similarity: f32,
    sensitivity: f32,
) -> Calibration {
    let s = sensitivity.clamp(0.0, 1.0);
    let floor = default_auto(s);
    let pairs = same_photo_scores(faces);
    let (mut auto, ask, calibrated) = if pairs.len() >= MIN_PAIRS {
        let auto_q = quantile(&pairs, 1.0 - target_rate(s)).unwrap_or(floor);
        let ask_q = quantile(&pairs, 1.0 - ASK_RATE).unwrap_or(similarity);
        (
            floor.max(auto_q),
            similarity.max(ask_q),
            auto_q > floor || ask_q > similarity,
        )
    } else {
        (floor, similarity, false)
    };
    // "Chiedi" resta sotto "unisci da solo": se serve si alza la seconda, mai il contrario.
    auto = auto.max(ask + ASK_GAP);
    let conf = confirmed_scores(faces, confirmed);
    let barely = conf.iter().filter(|x| **x >= ask && **x < auto).count();
    let too_strict = !conf.is_empty() && barely as f32 / conf.len() as f32 > TOO_STRICT_SHARE;
    Calibration {
        params: Params {
            ask,
            auto,
            margin: AiThresholds::DEFAULT_FACE_MERGE_MARGIN,
        },
        calibrated,
        same_photo_pairs: pairs.len(),
        confirmed_pairs: conf.len(),
        too_strict,
        sensitivity: s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bench::Rng;

    fn unit(rng: &mut Rng) -> Vec<f32> {
        let mut v: Vec<f32> = (0..128).map(|_| rng.gauss() as f32).collect();
        let n = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        v.iter_mut().for_each(|x| *x /= n);
        v
    }

    /// `photos` foto con `per_photo` volti di persone diverse ciascuna. Con `spread` < 1 gli
    /// sconosciuti si somigliano di più (coseni più alti).
    fn photos(n: usize, per_photo: usize, spread: f32, seed: u64) -> Vec<Face> {
        let mut rng = Rng::new(seed);
        let common = unit(&mut rng);
        let mut out = Vec::new();
        for m in 0..n {
            for k in 0..per_photo {
                let own = unit(&mut rng);
                let mut e: Vec<f32> = own
                    .iter()
                    .zip(&common)
                    .map(|(a, b)| spread * a + (1.0 - spread) * b)
                    .collect();
                let norm = e.iter().map(|x| x * x).sum::<f32>().sqrt();
                e.iter_mut().for_each(|x| *x /= norm);
                out.push(Face {
                    id: (m * per_photo + k) as i64,
                    media: m as i64,
                    model: "sface".into(),
                    embedding: e,
                    quality: 0.9,
                    ..Default::default()
                });
            }
        }
        out
    }

    #[test]
    fn quantili_calcolati_a_mano() {
        let v: Vec<f32> = (1..=1000).map(|i| i as f32 / 1000.0).collect();
        assert_eq!(quantile(&v, 0.99), Some(0.99));
        assert_eq!(quantile(&v, 0.999), Some(0.999));
        assert_eq!(quantile(&v, 1.0), Some(1.0));
        assert_eq!(quantile(&[], 0.5), None);
        assert!((target_rate(0.5) - 1e-3).abs() < 1e-12);
        assert!((target_rate(0.0) - 1e-4).abs() < 1e-12);
        assert!((target_rate(1.0) - 1e-2).abs() < 1e-12);
        assert!((default_auto(0.5) - 0.55).abs() < 1e-6);
        assert!(
            (default_auto(1.0) - 0.55).abs() < 1e-6,
            "mai sotto il predefinito"
        );
        assert!((default_auto(0.0) - 0.65).abs() < 1e-6);
    }

    #[test]
    fn libreria_con_pochi_dati_resta_ai_valori_predefiniti() {
        // Scenario "Libreria con pochi dati": meno di 200 coppie nella stessa foto.
        let faces = photos(60, 3, 1.0, 7); // 60 × 3 = 180 coppie
        let c = calibrate(&faces, &vec![false; faces.len()], 0.363, 0.5);
        assert_eq!(c.same_photo_pairs, 180);
        assert!(!c.calibrated);
        assert_eq!(c.params.ask, 0.363);
        assert_eq!(c.params.auto, 0.55);
        assert_eq!(c.params.margin, 0.05);
    }

    #[test]
    fn sconosciuti_tipici_non_spostano_le_soglie_predefinite() {
        // Coseni di vettori casuali a 128 dimensioni: deviazione ≈ 0,09, sotto 0,363.
        let faces = photos(400, 3, 1.0, 11);
        let c = calibrate(&faces, &vec![false; faces.len()], 0.363, 0.5);
        assert_eq!(c.same_photo_pairs, 1200);
        assert!(!c.calibrated);
        assert_eq!(c.params.auto, 0.55);
    }

    #[test]
    fn sconosciuti_molto_simili_alzano_le_soglie() {
        // Libreria con molti parenti: gli sconosciuti si somigliano, le soglie salgono.
        let faces = photos(400, 3, 0.5, 13);
        let pairs = same_photo_scores(&faces);
        let c = calibrate(&faces, &vec![false; faces.len()], 0.363, 0.5);
        assert!(c.calibrated);
        let q999 = quantile(&pairs, 0.999).unwrap();
        let q99 = quantile(&pairs, 0.99).unwrap();
        assert!(q999 > 0.55, "dati di prova: {q999}");
        assert_eq!(c.params.auto, q999.max(c.params.ask + 0.05));
        assert_eq!(c.params.ask, q99.max(0.363));
        assert!(
            c.params.ask < c.params.auto,
            "chiedi resta sotto unisci da solo"
        );
        // Sensibilità ridotta: tasso 10⁻⁴, soglia più alta (mai più bassa).
        let strict = calibrate(&faces, &vec![false; faces.len()], 0.363, 0.0);
        assert!(strict.params.auto >= c.params.auto);
        // Più unioni automatiche: mai sotto il valore predefinito.
        let loose = calibrate(&faces, &vec![false; faces.len()], 0.363, 1.0);
        assert!(loose.params.auto >= 0.55);
        assert!(loose.params.auto <= c.params.auto);
    }

    #[test]
    fn sensibilita_ridotta_alza_unisci_da_solo_anche_senza_dati() {
        let c = calibrate(&[], &[], 0.363, 0.0);
        assert!((c.params.auto - 0.65).abs() < 1e-6);
        let c = calibrate(&[], &[], 0.363, 1.0);
        assert!((c.params.auto - 0.55).abs() < 1e-6);
    }

    #[test]
    fn controllo_sulle_coppie_confermate() {
        // Volti confermati della stessa persona con coseni tra "chiedi" e "unisci da solo":
        // Diagnostica segnala soglie troppo severe, ma le soglie non cambiano.
        let mut rng = Rng::new(3);
        let center = unit(&mut rng);
        let mut faces = Vec::new();
        for i in 0..20 {
            let noise = unit(&mut rng);
            let mut e: Vec<f32> = center
                .iter()
                .zip(&noise)
                .map(|(a, b)| 0.68 * a + 0.73 * b)
                .collect();
            let n = e.iter().map(|x| x * x).sum::<f32>().sqrt();
            e.iter_mut().for_each(|x| *x /= n);
            faces.push(Face {
                id: i,
                media: i,
                model: "sface".into(),
                embedding: e,
                person: Some(1),
                quality: 0.9,
                ..Default::default()
            });
        }
        let conf = vec![true; faces.len()];
        let c = calibrate(&faces, &conf, 0.363, 0.5);
        assert_eq!(c.confirmed_pairs, 190);
        assert!(c.too_strict);
        assert_eq!(c.params.auto, 0.55);
        let c = calibrate(&faces, &vec![false; faces.len()], 0.363, 0.5);
        assert!(!c.too_strict);
    }
}
