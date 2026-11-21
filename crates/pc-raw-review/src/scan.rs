//! Scansione della cartella di revisione e abbinamento degli scatti alle varianti.
//!
//! Porting di `scan_library_impl` di FujiView (a798987), esteso a tre forme di
//! cartella (design D2):
//!
//! - **RAF al primo livello** ([`Layout::RafOnly`]): ogni RAF è uno scatto.
//! - **RAF con le varianti generate** ([`Layout::RafWithVariants`]): come sopra,
//!   con i preset letti dalle sottocartelle di `FujiView varianti/` (o da una
//!   cartella di output scelta dall'utente).
//! - **Una sottocartella per preset** ([`Layout::Presets`]): il layout classico
//!   di FujiView, senza RAF.
//!
//! La scansione è in sola lettura: non crea e non modifica nulla nella cartella.
//!
//! # Regole comuni
//!
//! - Le varianti si abbinano per nome base in minuscolo (`DSCF0001.JPG` e
//!   `dscf0001.tif` sono lo stesso scatto).
//! - Elenchi di preset e di scatti seguono l'ordine naturale (`natord`, senza
//!   distinzione di maiuscole): `DSCF2` precede `DSCF10`.
//! - Un secondo file con lo stesso nome base nello stesso preset è ignorato con
//!   un avviso. Per essere deterministico, i file di una cartella si leggono in
//!   ordine naturale (FujiView usava l'ordine del file system).
//! - Nelle cartelle dei preset contano solo JPEG e TIFF del primo livello: RAF,
//!   sidecar FP2/FP3, altri file e sottocartelle annidate sono ignorati.
//!
//! # Preset guida (layout per preset)
//!
//! Come in FujiView, la guida è il preset con più nomi base distinti; a parità
//! vince il primo in ordine naturale (si sostituisce solo con un aumento
//! stretto). **Solo i nomi della guida definiscono il catalogo**: gli scatti
//! presenti unicamente in altri preset sono scartati. Con i RAF gli scatti sono
//! definiti dai RAF, quindi nessuno scatto è scartato; la guida (calcolata sui
//! soli nomi che corrispondono a un RAF) serve solo come preset predefinito.
//!
//! # Differenza voluta rispetto a FujiView
//!
//! Una cartella i cui preset non contengono nessun JPEG/TIFF (per esempio
//! solo file annidati) è un errore spiegato, come richiede la spec "Cartella
//! senza foto"; FujiView restituiva un indice vuoto.
//!
//! # Anteprima RAW
//!
//! Ogni scatto RAF ha sempre la pseudo-variante [`RAW_PREVIEW_PRESET`]
//! ("Anteprima RAW"), in prima posizione, con il percorso del RAF e
//! [`VariantKind::RawPreview`]. Non compare in [`ReviewIndex::presets`] (che
//! elenca solo cartelle reali) e non è esportabile né scegliibile come variante
//! migliore: la regola di scelta spetta alla sessione, che usa
//! [`Variant::is_selectable`]; l'export usa [`is_exportable_jpeg`].

use pc_core::error::{AppError, AppResult, ErrorCode};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

/// Nome della cartella delle varianti generate, accanto ai RAF.
pub const VARIANTS_DIR_NAME: &str = "FujiView varianti";

/// Identificatore del preset della pseudo-variante "Anteprima RAW".
pub const RAW_PREVIEW_PRESET: &str = "raw-preview";

/// Etichetta mostrata per la pseudo-variante (la UI la traduce con i18n).
pub const RAW_PREVIEW_LABEL: &str = "Anteprima RAW";

/// Forma della cartella aperta.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Layout {
    /// Solo RAF al primo livello, nessuna variante generata.
    RafOnly,
    /// RAF più sottocartelle di preset in `output_dir`.
    RafWithVariants { output_dir: String },
    /// Una sottocartella per preset con JPEG o TIFF, senza RAF.
    Presets,
}

/// Natura di una variante.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum VariantKind {
    /// File immagine (JPEG o TIFF) di un preset.
    Image,
    /// Anteprima incorporata nel RAF ("Anteprima RAW").
    RawPreview,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Variant {
    pub preset: String,
    pub path: String,
    pub file_name: String,
    /// Estensione in minuscolo, senza punto.
    pub extension: String,
    pub bytes: u64,
    pub kind: VariantKind,
}

impl Variant {
    /// Se la variante può essere scelta come migliore (l'anteprima RAW no).
    pub fn is_selectable(&self) -> bool {
        self.kind == VariantKind::Image
    }
}

/// Vero solo per le varianti JPEG (`.jpg`/`.jpeg`, qualunque maiuscola): l'export
/// copia solo JPEG, quindi TIFF, RAF e l'anteprima RAW sono esclusi.
pub fn is_exportable_jpeg(variant: &Variant) -> bool {
    variant.kind == VariantKind::Image
        && matches!(
            variant.extension.to_ascii_lowercase().as_str(),
            "jpg" | "jpeg"
        )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Shot {
    /// Nome base in minuscolo: chiave di abbinamento.
    pub key: String,
    pub display_name: String,
    pub variants: Vec<Variant>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ReviewIndex {
    /// Percorso canonico della cartella aperta.
    pub root: String,
    pub layout: Layout,
    /// Cartelle di preset reali, in ordine naturale (senza "Anteprima RAW").
    pub presets: Vec<String>,
    /// Preset guida; `None` se non ci sono preset (solo RAF).
    pub guide_preset: Option<String>,
    pub shots: Vec<Shot>,
    pub warnings: Vec<String>,
}

fn unsupported_folder_error(root: &Path) -> AppError {
    AppError::new(
        ErrorCode::InvalidInput,
        format!(
            "La cartella «{}» non contiene foto da rivedere. Le forme supportate sono: \
             (1) file RAF al primo livello; \
             (2) file RAF con le varianti già generate in «{}/<preset>/»; \
             (3) una sottocartella per preset con file JPEG o TIFF.",
            root.display(),
            VARIANTS_DIR_NAME
        ),
    )
}

fn lower_extension(path: &Path) -> Option<String> {
    Some(path.extension()?.to_string_lossy().to_ascii_lowercase())
}

/// Estensione di un file immagine di preset (JPEG o TIFF), in minuscolo.
fn image_extension(path: &Path) -> Option<String> {
    lower_extension(path).filter(|ext| matches!(ext.as_str(), "jpg" | "jpeg" | "tif" | "tiff"))
}

fn is_raf(path: &Path) -> bool {
    lower_extension(path).as_deref() == Some("raf")
}

/// Elementi di una cartella in ordine naturale del nome (deterministico).
fn sorted_entries(dir: &Path) -> std::io::Result<Vec<fs::DirEntry>> {
    let mut entries = fs::read_dir(dir)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by(|a, b| {
        natord::compare_ignore_case(
            &a.file_name().to_string_lossy(),
            &b.file_name().to_string_lossy(),
        )
    });
    Ok(entries)
}

/// Sottocartelle dirette, in ordine naturale.
fn subdirectories(dir: &Path) -> AppResult<Vec<(String, PathBuf)>> {
    let mut dirs = Vec::new();
    for entry in sorted_entries(dir)? {
        if entry.file_type()?.is_dir() {
            dirs.push((
                entry.file_name().to_string_lossy().into_owned(),
                entry.path(),
            ));
        }
    }
    Ok(dirs)
}

/// Varianti raggruppate per nome base: chiave -> (nome originale, varianti).
type Grouped = BTreeMap<String, (String, Vec<Variant>)>;

struct Collected {
    grouped: Grouped,
    /// Nomi base presenti in ciascun preset (per la guida).
    preset_keys: BTreeMap<String, BTreeSet<String>>,
}

/// Legge JPEG e TIFF del primo livello di ogni cartella di preset.
fn collect_variants(preset_dirs: &[(String, PathBuf)], warnings: &mut Vec<String>) -> Collected {
    let mut grouped = Grouped::new();
    let mut preset_keys: BTreeMap<String, BTreeSet<String>> = preset_dirs
        .iter()
        .map(|(preset, _)| (preset.clone(), BTreeSet::new()))
        .collect();
    for (preset, directory) in preset_dirs {
        let entries = match sorted_entries(directory) {
            Ok(value) => value,
            Err(error) => {
                warnings.push(format!("{}: {}", directory.display(), error));
                continue;
            }
        };
        for entry in entries {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(extension) = image_extension(&path) else {
                continue;
            };
            let Some(stem) = path.file_stem().map(|v| v.to_string_lossy().into_owned()) else {
                continue;
            };
            let key = stem.to_lowercase();
            let variant = Variant {
                preset: preset.clone(),
                path: path.to_string_lossy().into_owned(),
                file_name: entry.file_name().to_string_lossy().into_owned(),
                extension,
                bytes: entry.metadata().map(|m| m.len()).unwrap_or(0),
                kind: VariantKind::Image,
            };
            let group = grouped
                .entry(key.clone())
                .or_insert_with(|| (stem, Vec::new()));
            if group.1.iter().any(|existing| existing.preset == *preset) {
                warnings.push(format!(
                    "Duplicato ignorato in {}: {}",
                    preset, variant.file_name
                ));
            } else {
                group.1.push(variant);
                if let Some(keys) = preset_keys.get_mut(preset) {
                    keys.insert(key);
                }
            }
        }
    }
    Collected {
        grouped,
        preset_keys,
    }
}

/// Preset con più nomi base; a parità il primo in ordine naturale.
fn pick_guide(presets: &[String], keys: &BTreeMap<String, BTreeSet<String>>) -> Option<String> {
    let mut guide = presets.first()?.clone();
    let mut guide_count = keys.get(&guide).map_or(0, BTreeSet::len);
    for preset in presets.iter().skip(1) {
        let count = keys.get(preset).map_or(0, BTreeSet::len);
        if count > guide_count {
            guide = preset.clone();
            guide_count = count;
        }
    }
    Some(guide)
}

fn sort_variants(variants: &mut [Variant]) {
    // L'anteprima RAW ("raw-preview") resta sempre per prima.
    variants.sort_by(|a, b| {
        (a.kind != VariantKind::RawPreview)
            .cmp(&(b.kind != VariantKind::RawPreview))
            .then_with(|| natord::compare_ignore_case(&a.preset, &b.preset))
    });
}

/// Scansiona `root` riconoscendo le tre forme di cartella.
///
/// `variants_dir` è la cartella di output delle varianti, se diversa da
/// `<root>/FujiView varianti` (la fornisce la sessione). Vale solo per le
/// cartelle con RAF. Non scrive nulla.
pub fn scan(root: &Path, variants_dir: Option<&Path>) -> AppResult<ReviewIndex> {
    if !root.is_dir() {
        return Err(AppError::new(
            ErrorCode::NotFound,
            "La cartella sorgente non esiste o non è accessibile",
        ));
    }
    let canonical = root.canonicalize()?;

    let mut rafs = Vec::new();
    for entry in sorted_entries(&canonical)? {
        let path = entry.path();
        if path.is_file() && is_raf(&path) {
            rafs.push((path, entry));
        }
    }

    if rafs.is_empty() {
        scan_presets(&canonical)
    } else {
        scan_rafs(&canonical, rafs, variants_dir)
    }
}

/// Layout classico di FujiView: una sottocartella per preset.
fn scan_presets(canonical: &Path) -> AppResult<ReviewIndex> {
    let preset_dirs = subdirectories(canonical)?;
    let mut warnings = Vec::new();
    let Collected {
        grouped,
        preset_keys,
    } = collect_variants(&preset_dirs, &mut warnings);
    if grouped.is_empty() {
        return Err(unsupported_folder_error(canonical));
    }
    let presets: Vec<String> = preset_dirs.into_iter().map(|(name, _)| name).collect();
    let guide_preset = pick_guide(&presets, &preset_keys);
    let guide_name = guide_preset.clone().unwrap_or_default();
    let guide_keys = preset_keys.get(&guide_name).cloned().unwrap_or_default();

    let mut shots: Vec<Shot> = grouped
        .into_iter()
        .filter(|(key, _)| guide_keys.contains(key))
        .map(|(key, (fallback_name, mut variants))| {
            sort_variants(&mut variants);
            let display_name = variants
                .iter()
                .find(|variant| variant.preset == guide_name)
                .and_then(|variant| Path::new(&variant.path).file_stem())
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or(fallback_name);
            Shot {
                key,
                display_name,
                variants,
            }
        })
        .collect();
    shots.sort_by(|a, b| natord::compare_ignore_case(&a.display_name, &b.display_name));
    Ok(ReviewIndex {
        root: canonical.to_string_lossy().into_owned(),
        layout: Layout::Presets,
        presets,
        guide_preset,
        shots,
        warnings,
    })
}

/// Cartella con RAF, con o senza varianti già generate.
fn scan_rafs(
    canonical: &Path,
    rafs: Vec<(PathBuf, fs::DirEntry)>,
    variants_dir: Option<&Path>,
) -> AppResult<ReviewIndex> {
    let mut warnings = Vec::new();
    let candidate =
        variants_dir.map_or_else(|| canonical.join(VARIANTS_DIR_NAME), Path::to_path_buf);
    let output_dir = if candidate.is_dir() {
        Some(candidate)
    } else {
        if variants_dir.is_some() {
            warnings.push(format!(
                "Cartella delle varianti non trovata: {}",
                candidate.display()
            ));
        }
        None
    };

    // Gli scatti sono definiti dai RAF.
    let mut shots_by_key: BTreeMap<String, Shot> = BTreeMap::new();
    for (path, entry) in rafs {
        let Some(stem) = path.file_stem().map(|v| v.to_string_lossy().into_owned()) else {
            continue;
        };
        let key = stem.to_lowercase();
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if shots_by_key.contains_key(&key) {
            warnings.push(format!("Duplicato ignorato tra i RAF: {file_name}"));
            continue;
        }
        let preview = Variant {
            preset: RAW_PREVIEW_PRESET.to_string(),
            path: path.to_string_lossy().into_owned(),
            file_name,
            extension: "raf".to_string(),
            bytes: entry.metadata().map(|m| m.len()).unwrap_or(0),
            kind: VariantKind::RawPreview,
        };
        shots_by_key.insert(
            key.clone(),
            Shot {
                key,
                display_name: stem,
                variants: vec![preview],
            },
        );
    }

    let mut presets = Vec::new();
    let mut guide_preset = None;
    if let Some(dir) = &output_dir {
        let preset_dirs = subdirectories(dir)?;
        let Collected {
            grouped,
            mut preset_keys,
        } = collect_variants(&preset_dirs, &mut warnings);
        let mut orphans: BTreeMap<String, usize> = BTreeMap::new();
        for (key, (_, variants)) in grouped {
            match shots_by_key.get_mut(&key) {
                Some(shot) => shot.variants.extend(variants),
                None => {
                    for variant in variants {
                        *orphans.entry(variant.preset).or_default() += 1;
                    }
                }
            }
        }
        for (preset, count) in orphans {
            warnings.push(format!(
                "{preset}: {count} file senza RAF corrispondente ignorati"
            ));
        }
        // La guida si calcola sui soli nomi che corrispondono a un RAF.
        for keys in preset_keys.values_mut() {
            keys.retain(|key| shots_by_key.contains_key(key));
        }
        presets = preset_dirs.into_iter().map(|(name, _)| name).collect();
        guide_preset = pick_guide(&presets, &preset_keys);
    }

    let mut shots: Vec<Shot> = shots_by_key.into_values().collect();
    for shot in &mut shots {
        sort_variants(&mut shot.variants);
    }
    shots.sort_by(|a, b| natord::compare_ignore_case(&a.display_name, &b.display_name));
    let layout = match output_dir {
        Some(dir) => Layout::RafWithVariants {
            output_dir: dir.to_string_lossy().into_owned(),
        },
        None => Layout::RafOnly,
    };
    Ok(ReviewIndex {
        root: canonical.to_string_lossy().into_owned(),
        layout,
        presets,
        guide_preset,
        shots,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(path: &Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"x").unwrap();
    }

    fn snapshot(root: &Path) -> Vec<(PathBuf, u64)> {
        fn walk(dir: &Path, out: &mut Vec<(PathBuf, u64)>) {
            for entry in fs::read_dir(dir).unwrap() {
                let entry = entry.unwrap();
                let meta = entry.metadata().unwrap();
                out.push((entry.path(), meta.len()));
                if meta.is_dir() {
                    walk(&entry.path(), out);
                }
            }
        }
        let mut out = Vec::new();
        walk(root, &mut out);
        out.sort();
        out
    }

    fn names(index: &ReviewIndex) -> Vec<&str> {
        index
            .shots
            .iter()
            .map(|s| s.display_name.as_str())
            .collect()
    }

    // --- Porting dei test di FujiView ---

    #[test]
    fn groups_extensions_and_case_by_stem() {
        let temp = tempfile::tempdir().unwrap();
        touch(&temp.path().join("Classic/DSC001.JPG"));
        touch(&temp.path().join("Velvia/dsc001.tif"));
        let result = scan(temp.path(), None).unwrap();
        assert_eq!(result.layout, Layout::Presets);
        assert_eq!(result.shots.len(), 1);
        assert_eq!(result.shots[0].variants.len(), 2);
    }

    #[test]
    fn ignores_root_files_and_nested_directories() {
        let temp = tempfile::tempdir().unwrap();
        touch(&temp.path().join("root.jpg"));
        touch(&temp.path().join("A/nested/hidden.jpg"));
        touch(&temp.path().join("A/DSCF1.jpg"));
        let result = scan(temp.path(), None).unwrap();
        assert_eq!(names(&result), ["DSCF1"]);
        // Senza altre foto non c'è nulla da rivedere: errore spiegato.
        let vuota = tempfile::tempdir().unwrap();
        touch(&vuota.path().join("root.jpg"));
        touch(&vuota.path().join("A/nested/hidden.jpg"));
        assert!(scan(vuota.path(), None).is_err());
    }

    #[test]
    fn ignores_fujifilm_raw_and_sidecar_files_in_preset_folders() {
        let temp = tempfile::tempdir().unwrap();
        let preset = temp.path().join("RAW");
        touch(&preset.join("DSCF0001.RAF"));
        touch(&preset.join("DSCF0001.FP2"));
        touch(&preset.join("DSCF0001.FP3"));
        touch(&preset.join("DSCF0002.jpg"));
        let result = scan(temp.path(), None).unwrap();
        assert_eq!(result.layout, Layout::Presets);
        assert_eq!(names(&result), ["DSCF0002"]);
        assert_eq!(result.shots[0].variants.len(), 1);
    }

    #[test]
    fn largest_preset_is_the_guide_and_shots_use_natural_order() {
        let temp = tempfile::tempdir().unwrap();
        for name in ["DSCF2.jpg", "DSCF10.jpg", "ONLY_SMALLER.jpg"] {
            touch(&temp.path().join("Preset 2").join(name));
        }
        for name in ["DSCF1.jpg", "DSCF2.jpg", "DSCF3.jpg", "DSCF10.jpg"] {
            touch(&temp.path().join("Preset 10").join(name));
        }
        let result = scan(temp.path(), None).unwrap();
        assert_eq!(result.guide_preset.as_deref(), Some("Preset 10"));
        assert_eq!(names(&result), ["DSCF1", "DSCF2", "DSCF3", "DSCF10"]);
        assert!(result.shots.iter().all(|shot| shot.key != "only_smaller"));
    }

    // --- Nuovi test ---

    #[test]
    fn guide_tie_picks_first_in_natural_order() {
        let temp = tempfile::tempdir().unwrap();
        touch(&temp.path().join("B/a.jpg"));
        touch(&temp.path().join("A/b.jpg"));
        let result = scan(temp.path(), None).unwrap();
        assert_eq!(result.guide_preset.as_deref(), Some("A"));
        assert_eq!(names(&result), ["b"]);
    }

    #[test]
    fn raf_only_folder() {
        let temp = tempfile::tempdir().unwrap();
        touch(&temp.path().join("DSCF0002.RAF"));
        touch(&temp.path().join("dscf0001.raf"));
        touch(&temp.path().join("nota.txt"));
        let result = scan(temp.path(), None).unwrap();
        assert_eq!(result.layout, Layout::RafOnly);
        assert!(result.presets.is_empty());
        assert_eq!(result.guide_preset, None);
        assert_eq!(names(&result), ["dscf0001", "DSCF0002"]);
        for shot in &result.shots {
            assert_eq!(shot.variants.len(), 1);
            let v = &shot.variants[0];
            assert_eq!(v.preset, RAW_PREVIEW_PRESET);
            assert_eq!(v.kind, VariantKind::RawPreview);
            assert!(!v.is_selectable());
            assert!(!is_exportable_jpeg(v));
            assert!(v.path.to_lowercase().ends_with(".raf"));
        }
    }

    #[test]
    fn raf_with_fujiview_variants() {
        let temp = tempfile::tempdir().unwrap();
        touch(&temp.path().join("DSCF0001.RAF"));
        let dir = temp.path().join(VARIANTS_DIR_NAME);
        touch(&dir.join("C1 Kodachrome/DSCF0001.jpg"));
        touch(&dir.join("Come scattato/DSCF0001.jpg"));
        touch(&dir.join("Come scattato/ALTRO.jpg"));
        let result = scan(temp.path(), None).unwrap();
        assert!(matches!(result.layout, Layout::RafWithVariants { .. }));
        assert_eq!(result.presets, ["C1 Kodachrome", "Come scattato"]);
        assert_eq!(names(&result), ["DSCF0001"]);
        let presets: Vec<&str> = result.shots[0]
            .variants
            .iter()
            .map(|v| v.preset.as_str())
            .collect();
        assert_eq!(
            presets,
            [RAW_PREVIEW_PRESET, "C1 Kodachrome", "Come scattato"]
        );
        assert!(result.shots[0].variants[1].is_selectable());
        assert!(is_exportable_jpeg(&result.shots[0].variants[1]));
        // La variante senza RAF è ignorata con un avviso.
        assert_eq!(result.guide_preset.as_deref(), Some("C1 Kodachrome"));
        assert!(result.warnings.iter().any(|w| w.contains("senza RAF")));
    }

    #[test]
    fn raf_with_explicit_output_dir() {
        let temp = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        touch(&temp.path().join("DSCF0001.RAF"));
        touch(&out.path().join("Velvia/DSCF0001.JPG"));
        let result = scan(temp.path(), Some(out.path())).unwrap();
        match &result.layout {
            Layout::RafWithVariants { output_dir } => {
                assert_eq!(Path::new(output_dir), out.path());
            }
            other => panic!("layout inatteso: {other:?}"),
        }
        assert_eq!(result.shots[0].variants.len(), 2);
        // Cartella esplicita inesistente: ripiega su solo RAF con un avviso.
        let missing = temp.path().join("non-esiste");
        let result = scan(temp.path(), Some(&missing)).unwrap();
        assert_eq!(result.layout, Layout::RafOnly);
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn missing_variant_keeps_the_shot() {
        let temp = tempfile::tempdir().unwrap();
        for name in ["DSCF2.jpg", "DSCF10.jpg"] {
            touch(&temp.path().join("Classic Chrome").join(name));
        }
        touch(&temp.path().join("Velvia/DSCF2.jpg"));
        let result = scan(temp.path(), None).unwrap();
        assert_eq!(names(&result), ["DSCF2", "DSCF10"]);
        assert_eq!(result.shots[0].variants.len(), 2);
        assert_eq!(result.shots[1].variants.len(), 1);
        assert_eq!(result.shots[1].variants[0].preset, "Classic Chrome");
    }

    #[test]
    fn missing_variant_in_raf_layout() {
        let temp = tempfile::tempdir().unwrap();
        touch(&temp.path().join("DSCF2.RAF"));
        touch(&temp.path().join("DSCF10.RAF"));
        let dir = temp.path().join(VARIANTS_DIR_NAME);
        touch(&dir.join("Velvia/DSCF2.jpg"));
        let result = scan(temp.path(), None).unwrap();
        assert_eq!(names(&result), ["DSCF2", "DSCF10"]);
        assert_eq!(result.shots[0].variants.len(), 2);
        assert_eq!(result.shots[1].variants.len(), 1);
    }

    #[test]
    fn duplicate_in_same_preset_is_ignored_with_warning() {
        let temp = tempfile::tempdir().unwrap();
        touch(&temp.path().join("Classic/DSCF1.jpg"));
        touch(&temp.path().join("Classic/DSCF1.tif"));
        let result = scan(temp.path(), None).unwrap();
        assert_eq!(result.shots.len(), 1);
        assert_eq!(result.shots[0].variants.len(), 1);
        assert_eq!(result.shots[0].variants[0].extension, "jpg");
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("Duplicato ignorato in Classic"));
    }

    #[test]
    fn empty_folder_gives_explained_error() {
        let temp = tempfile::tempdir().unwrap();
        touch(&temp.path().join("nota.txt"));
        fs::create_dir(temp.path().join("vuota")).unwrap();
        let error = scan(temp.path(), None).unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput);
        assert!(error.detail.contains("RAF"));
        assert!(error.detail.contains(VARIANTS_DIR_NAME));
        assert!(error.detail.contains("JPEG o TIFF"));
    }

    #[test]
    fn missing_folder_is_not_found() {
        let temp = tempfile::tempdir().unwrap();
        let error = scan(&temp.path().join("nessuna"), None).unwrap_err();
        assert_eq!(error.code, ErrorCode::NotFound);
    }

    #[test]
    fn natural_order_dscf2_before_dscf10() {
        let temp = tempfile::tempdir().unwrap();
        for name in ["DSCF10.RAF", "DSCF2.RAF", "DSCF1.RAF"] {
            touch(&temp.path().join(name));
        }
        let result = scan(temp.path(), None).unwrap();
        assert_eq!(names(&result), ["DSCF1", "DSCF2", "DSCF10"]);
    }

    #[test]
    fn scan_does_not_write_anything() {
        let temp = tempfile::tempdir().unwrap();
        touch(&temp.path().join("DSCF0001.RAF"));
        touch(
            &temp
                .path()
                .join(VARIANTS_DIR_NAME)
                .join("Velvia/DSCF0001.jpg"),
        );
        touch(&temp.path().join("presets-solo/Classic/DSCF0001.jpg"));
        let before = snapshot(temp.path());
        scan(temp.path(), None).unwrap();
        scan(&temp.path().join("presets-solo"), None).unwrap();
        assert_eq!(snapshot(temp.path()), before);
    }

    #[test]
    fn is_exportable_jpeg_only_for_jpeg() {
        let make = |ext: &str, kind| Variant {
            preset: "p".into(),
            path: format!("x.{ext}"),
            file_name: format!("x.{ext}"),
            extension: ext.into(),
            bytes: 1,
            kind,
        };
        assert!(is_exportable_jpeg(&make("jpg", VariantKind::Image)));
        assert!(is_exportable_jpeg(&make("jpeg", VariantKind::Image)));
        assert!(is_exportable_jpeg(&make("JPG", VariantKind::Image)));
        assert!(!is_exportable_jpeg(&make("tif", VariantKind::Image)));
        assert!(!is_exportable_jpeg(&make("tiff", VariantKind::Image)));
        assert!(!is_exportable_jpeg(&make("RAF", VariantKind::RawPreview)));
        assert!(!is_exportable_jpeg(&make("jpg", VariantKind::RawPreview)));
    }

    #[test]
    fn json_shape() {
        let temp = tempfile::tempdir().unwrap();
        touch(&temp.path().join("A.RAF"));
        let value = serde_json::to_value(scan(temp.path(), None).unwrap()).unwrap();
        assert_eq!(value["layout"]["kind"], "rafOnly");
        assert!(value["guidePreset"].is_null());
        assert_eq!(value["shots"][0]["displayName"], "A");
        assert_eq!(value["shots"][0]["variants"][0]["kind"], "raw_preview");
        assert_eq!(value["shots"][0]["variants"][0]["fileName"], "A.RAF");
    }
}
