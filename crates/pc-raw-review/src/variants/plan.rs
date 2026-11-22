//! Piano dello sviluppo (porting di `plan.rs` di FujiView): quali varianti RAF × preset
//! esistono già, quali mancano e dove va ciascuna. La pianificazione non scrive mai su disco.

use super::estimate::{self, FreeSpace};
use super::types::{PlannedPreset, VariantPlan, VariantRequest};
use crate::camera::banks::{folder_name, Bank};
use pc_core::{AppError, AppResult, ErrorCode};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Cartella di output predefinita, accanto ai RAF: la stessa che la scansione della revisione
/// legge direttamente (`scan::VARIANTS_DIR_NAME`).
pub const DEFAULT_OUTPUT_DIR: &str = crate::scan::VARIANTS_DIR_NAME;
pub const AS_SHOT_ID: &str = "asshot";
pub const AS_SHOT_FOLDER: &str = "Come scattato";
pub const MANIFEST: &str = ".fujiview-preset.json";
pub const PART_SUFFIX: &str = ".part";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresetKind {
    AsShot,
    Bank(Bank),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresetDef {
    pub id: String,
    pub kind: PresetKind,
}

impl PresetDef {
    pub fn as_shot() -> Self {
        Self {
            id: AS_SHOT_ID.into(),
            kind: PresetKind::AsShot,
        }
    }

    pub fn bank(bank: Bank) -> Self {
        Self {
            id: format!("C{}", bank.index),
            kind: PresetKind::Bank(bank),
        }
    }

    pub fn base_folder_name(&self) -> String {
        match &self.kind {
            PresetKind::AsShot => AS_SHOT_FOLDER.into(),
            PresetKind::Bank(bank) => folder_name(bank.index, &bank.name),
        }
    }

    pub fn fingerprint(&self) -> String {
        match &self.kind {
            PresetKind::AsShot => AS_SHOT_ID.into(),
            PresetKind::Bank(bank) => bank.fingerprint(),
        }
    }

    pub fn manifest(&self) -> Manifest {
        Manifest {
            id: self.id.clone(),
            fingerprint: self.fingerprint(),
            bank: match &self.kind {
                PresetKind::AsShot => None,
                PresetKind::Bank(bank) => Some(bank.clone()),
            },
            generator: "photo-cat".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub id: String,
    pub fingerprint: String,
    pub bank: Option<Bank>,
    pub generator: String,
}

pub fn read_manifest(dir: &Path) -> Option<Manifest> {
    serde_json::from_str(&fs::read_to_string(dir.join(MANIFEST)).ok()?).ok()
}

#[derive(Debug, Clone)]
pub struct PresetTarget {
    pub preset: PresetDef,
    pub folder_name: String,
    pub dir: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Conversion {
    pub raf: usize,
    pub preset: usize,
}

#[derive(Debug, Clone)]
pub struct Plan {
    pub source_root: PathBuf,
    pub output_dir: PathBuf,
    pub rafs: Vec<PathBuf>,
    pub targets: Vec<PresetTarget>,
    pub pending: Vec<Conversion>,
    pub already_done: usize,
    pub warnings: Vec<String>,
}

impl Plan {
    pub fn total(&self) -> usize {
        self.rafs.len() * self.targets.len()
    }

    pub fn output_path(&self, conversion: Conversion) -> PathBuf {
        let stem = self.rafs[conversion.raf]
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        self.targets[conversion.preset]
            .dir
            .join(format!("{stem}.jpg"))
    }

    pub fn summary(&self) -> VariantPlan {
        self.summary_with(&(Arc::new(|p: &Path| estimate::free_bytes(p)) as FreeSpace))
    }

    /// Come [`Plan::summary`], con un altro modo di leggere lo spazio libero.
    pub fn summary_with(&self, free_space: &FreeSpace) -> VariantPlan {
        let free = free_space(&self.output_dir);
        let required =
            estimate::required_bytes(self.pending.len(), estimate::DEFAULT_VARIANT_BYTES);
        let raf_uploads = self
            .pending
            .iter()
            .map(|c| c.raf)
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        VariantPlan {
            source_root: self.source_root.to_string_lossy().into_owned(),
            output_dir: self.output_dir.to_string_lossy().into_owned(),
            raf_count: self.rafs.len(),
            presets: self
                .targets
                .iter()
                .map(|t| PlannedPreset {
                    id: t.preset.id.clone(),
                    folder_name: t.folder_name.clone(),
                })
                .collect(),
            total_conversions: self.total(),
            pending_conversions: self.pending.len(),
            already_done: self.already_done,
            estimated_bytes: required,
            free_bytes: free,
            safety_margin_bytes: estimate::SAFETY_MARGIN_BYTES,
            // Con lo spazio libero illeggibile non si blocca l'avvio.
            enough_space: free.is_none_or(|f| estimate::has_room(f, required)),
            estimated_seconds: (estimate::DEFAULT_SECONDS_PER_CONVERSION
                * self.pending.len() as f64) as u64
                + 2 * raf_uploads as u64,
            warnings: self.warnings.clone(),
        }
    }
}

fn is_raf(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("raf"))
}

/// Resolve request sources into a source root and a naturally sorted RAF list.
pub fn collect_rafs(sources: &[String]) -> Result<(PathBuf, Vec<PathBuf>), String> {
    let first = sources.first().ok_or("Nessuna sorgente selezionata.")?;
    let first = PathBuf::from(first);
    let (root, mut rafs) = if sources.len() == 1 && first.is_dir() {
        let entries = fs::read_dir(&first)
            .map_err(|e| format!("Impossibile leggere {}: {e}", first.display()))?;
        let rafs = entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| is_raf(p))
            .collect();
        (first, rafs)
    } else {
        let mut rafs = Vec::new();
        for source in sources {
            let path = PathBuf::from(source);
            if !is_raf(&path) {
                return Err(format!("{} non è un file RAF.", path.display()));
            }
            rafs.push(path);
        }
        let root = rafs[0]
            .parent()
            .map(Path::to_path_buf)
            .ok_or("Percorso RAF non valido.")?;
        (root, rafs)
    };
    rafs.sort_by(|a, b| natord::compare_ignore_case(&a.to_string_lossy(), &b.to_string_lossy()));
    rafs.dedup();
    if rafs.is_empty() {
        return Err("La cartella non contiene file RAF.".into());
    }
    Ok((root, rafs))
}

/// A variant is complete when the JPEG exists, is non-empty and ends with EOI.
/// Reads only the first and last bytes: planning checks hundreds of ~26 MB files.
pub fn variant_complete(path: &Path) -> bool {
    use std::io::{Read, Seek, SeekFrom};
    let check = || -> std::io::Result<bool> {
        let mut file = fs::File::open(path)?;
        let len = file.metadata()?.len();
        if len < 4 {
            return Ok(false);
        }
        let mut head = [0u8; 2];
        file.read_exact(&mut head)?;
        if head != [0xFF, 0xD8] {
            return Ok(false);
        }
        let tail_len = len.min(64);
        file.seek(SeekFrom::Start(len - tail_len))?;
        let mut tail = vec![0u8; tail_len as usize];
        file.read_exact(&mut tail)?;
        Ok(tail.windows(2).any(|w| w == [0xFF, 0xD9]))
    };
    check().unwrap_or(false)
}

fn has_variant_folders(dir: &Path) -> bool {
    fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .any(|e| e.path().join(MANIFEST).is_file())
        })
        .unwrap_or(false)
}

/// The output folder actually used. When the chosen folder holds no variants
/// but its `FujiView varianti` subfolder does (the user picked the RAF folder
/// itself), use the subfolder so existing variants are found and skipped.
pub fn resolve_output_dir(chosen: &Path) -> (PathBuf, Option<String>) {
    let nested = chosen.join(DEFAULT_OUTPUT_DIR);
    if !has_variant_folders(chosen) && has_variant_folders(&nested) {
        let note = format!(
            "La cartella scelta contiene già \"{DEFAULT_OUTPUT_DIR}\" con varianti esistenti: uso quella."
        );
        (nested, Some(note))
    } else {
        (chosen.to_path_buf(), None)
    }
}

/// Pick the folder for a preset: the plain name, or `name (n)` when the plain
/// folder already holds variants made with different settings.
fn resolve_folder(output: &Path, preset: &PresetDef, taken: &[String]) -> (String, PathBuf) {
    let base = preset.base_folder_name();
    let fingerprint = preset.fingerprint();
    for n in 1.. {
        let name = if n == 1 {
            base.clone()
        } else {
            format!("{base} ({n})")
        };
        if taken.iter().any(|t| t.eq_ignore_ascii_case(&name)) {
            continue;
        }
        let dir = output.join(&name);
        let usable = match read_manifest(&dir) {
            Some(manifest) => manifest.fingerprint == fingerprint,
            // A folder we did not create: only reuse it when it is empty.
            None => {
                !dir.exists()
                    || fs::read_dir(&dir)
                        .map(|mut d| d.next().is_none())
                        .unwrap_or(false)
            }
        };
        if usable {
            return (name, dir);
        }
    }
    unreachable!()
}

/// Il percorso con l'antenato esistente più vicino reso canonico (collegamenti risolti, maiuscole
/// reali su Windows) e il resto aggiunto così com'è: confrontabile anche se non esiste ancora.
fn normalized(path: &Path) -> PathBuf {
    use std::path::Component;
    // Prima `.` e `..` (lessicalmente: la parte che non esiste non si può risolvere).
    let mut lexical = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                lexical.pop();
            }
            other => lexical.push(other.as_os_str()),
        }
    }
    let path = lexical.as_path();
    let mut rest: Vec<&std::ffi::OsStr> = Vec::new();
    let mut current = path;
    loop {
        if let Ok(canonical) = current.canonicalize() {
            return rest
                .iter()
                .rev()
                .fold(canonical, |acc, part| acc.join(part));
        }
        match (current.parent(), current.file_name()) {
            (Some(parent), Some(name)) => {
                rest.push(name);
                current = parent;
            }
            _ => return path.to_path_buf(),
        }
    }
}

/// Cartelle di sistema in cui lo sviluppo non scrive mai (né dentro).
fn system_dirs() -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        [
            "SystemRoot",
            "windir",
            "ProgramFiles",
            "ProgramFiles(x86)",
            "ProgramW6432",
            "ProgramData",
        ]
        .iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .collect()
    }
    #[cfg(not(windows))]
    {
        [
            "/bin",
            "/boot",
            "/dev",
            "/etc",
            "/lib",
            "/lib32",
            "/lib64",
            "/libx32",
            "/proc",
            "/run",
            "/sbin",
            "/snap",
            "/sys",
            "/usr",
            "/System",
            "/Library",
            "/Applications",
            "/private/etc",
        ]
        .iter()
        .map(PathBuf::from)
        .collect()
    }
}

/// Cartelle che contengono le cartelle degli utenti o dei dischi: vietate come cartella di
/// output, ma non le loro sottocartelle.
fn container_dirs() -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("SystemDrive")
            .map(|drive| vec![PathBuf::from(format!(r"{}\Users", drive.to_string_lossy()))])
            .unwrap_or_default()
    }
    #[cfg(not(windows))]
    {
        ["/home", "/Users", "/Volumes", "/media", "/mnt"]
            .iter()
            .map(PathBuf::from)
            .collect()
    }
}

/// Regole della cartella di output dello sviluppo (spec variant-generation, "Protezione dei file
/// originali"). Le cartelle dei preset vanno dentro `output`, quindi `output` non può essere:
/// - un percorso relativo, o un file;
/// - la radice di un disco (`C:\`, `/`, `\\server\share`);
/// - la cartella dei RAF, né una cartella che la contiene: una cartella di preset potrebbe
///   coincidere con quella dei RAF o con una vicina, e la revisione leggerebbe come preset le
///   cartelle delle foto;
/// - una cartella di sistema o una sua sottocartella, o la cartella che contiene gli utenti.
///
/// Una sottocartella della cartella dei RAF (come la predefinita `FujiView varianti`), una
/// cartella accanto o su un altro disco vanno bene.
pub fn check_output_dir(source_root: &Path, output: &Path) -> AppResult<()> {
    let reject = |reason: &str| {
        Err(AppError::new(
            ErrorCode::OutputFolderRejected,
            format!(
                "{reason}: {}. Scegli un'altra cartella, per esempio «{DEFAULT_OUTPUT_DIR}» accanto ai RAF.",
                output.display()
            ),
        ))
    };
    if !output.is_absolute() {
        return reject("La cartella di output deve essere un percorso completo");
    }
    if output.exists() && !output.is_dir() {
        return reject("La cartella di output è un file");
    }
    let out = normalized(output);
    let source = normalized(source_root);
    if out.parent().is_none() {
        return reject("La cartella di output non può essere la radice di un disco");
    }
    if out == source {
        return reject("La cartella di output non può essere la cartella dei RAF");
    }
    if source.starts_with(&out) {
        return reject("La cartella di output non può contenere la cartella dei RAF");
    }
    if system_dirs()
        .iter()
        .any(|dir| out.starts_with(normalized(dir)))
        || container_dirs().iter().any(|dir| out == normalized(dir))
    {
        return reject("La cartella di output non può essere una cartella di sistema");
    }
    Ok(())
}

pub fn build_plan(request: &VariantRequest, presets: &[PresetDef]) -> AppResult<Plan> {
    let (source_root, rafs) = collect_rafs(&request.sources).map_err(AppError::invalid_input)?;
    let mut warnings = Vec::new();
    let output_dir = match request
        .output_dir
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(dir) => {
            let (dir, note) = resolve_output_dir(Path::new(dir));
            warnings.extend(note);
            dir
        }
        None => source_root.join(DEFAULT_OUTPUT_DIR),
    };
    check_output_dir(&source_root, &output_dir)?;
    let mut targets: Vec<PresetTarget> = Vec::new();
    for id in &request.preset_ids {
        let Some(preset) = presets.iter().find(|p| &p.id == id) else {
            warnings.push(format!(
                "Preset {id} non disponibile sulla fotocamera: ignorato."
            ));
            continue;
        };
        if targets.iter().any(|t| t.preset.id == preset.id) {
            continue;
        }
        let taken: Vec<String> = targets.iter().map(|t| t.folder_name.clone()).collect();
        let (folder_name, dir) = resolve_folder(&output_dir, preset, &taken);
        if folder_name != preset.base_folder_name() {
            warnings.push(format!("{} ha impostazioni diverse dalle varianti esistenti: uso la cartella \"{folder_name}\".", preset.base_folder_name()));
        }
        targets.push(PresetTarget {
            preset: preset.clone(),
            folder_name,
            dir,
        });
    }
    if targets.is_empty() {
        return Err(AppError::invalid_input("Nessun preset selezionato."));
    }
    let mut plan = Plan {
        source_root,
        output_dir,
        rafs,
        targets,
        pending: Vec::new(),
        already_done: 0,
        warnings,
    };
    for raf in 0..plan.rafs.len() {
        for preset in 0..plan.targets.len() {
            let conversion = Conversion { raf, preset };
            if !request.overwrite && variant_complete(&plan.output_path(conversion)) {
                plan.already_done += 1;
            } else {
                plan.pending.push(conversion);
            }
        }
    }
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    const JPEG: &[u8] = &[0xFF, 0xD8, 1, 2, 3, 0xFF, 0xD9];

    fn bank(index: u8, name: &str, film: i32) -> PresetDef {
        PresetDef::bank(Bank {
            index,
            name: name.into(),
            props: BTreeMap::from([(0xD192, film)]),
        })
    }

    fn presets() -> Vec<PresetDef> {
        vec![
            PresetDef::as_shot(),
            bank(1, "Kodachrome", 11),
            bank(2, "Neg/Hi: 2", 4),
        ]
    }

    fn setup(rafs: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for name in rafs {
            fs::write(dir.path().join(name), b"raf").unwrap();
        }
        fs::write(dir.path().join("DSCF0001.JPG"), JPEG).unwrap();
        dir
    }

    fn request(dir: &Path, ids: &[&str]) -> VariantRequest {
        VariantRequest {
            sources: vec![dir.to_string_lossy().into_owned()],
            output_dir: None,
            preset_ids: ids.iter().map(|s| s.to_string()).collect(),
            overwrite: false,
        }
    }

    #[test]
    fn plans_every_raf_for_every_preset() {
        let dir = setup(&["DSCF10.RAF", "DSCF2.raf", "DSCF0001.RAF"]);
        let plan = build_plan(&request(dir.path(), &["asshot", "C1", "C2"]), &presets()).unwrap();
        assert_eq!(plan.rafs.len(), 3);
        assert!(
            plan.rafs[1].ends_with("DSCF2.raf"),
            "natural order: {:?}",
            plan.rafs
        );
        assert_eq!(plan.total(), 9);
        assert_eq!(plan.pending.len(), 9);
        assert_eq!(plan.output_dir, dir.path().join(DEFAULT_OUTPUT_DIR));
        let names: Vec<_> = plan
            .targets
            .iter()
            .map(|t| t.folder_name.as_str())
            .collect();
        assert_eq!(names, ["Come scattato", "C1 Kodachrome", "C2 Neg-Hi- 2"]);
        assert_eq!(
            plan.output_path(Conversion { raf: 0, preset: 1 }),
            dir.path()
                .join(DEFAULT_OUTPUT_DIR)
                .join("C1 Kodachrome")
                .join("DSCF0001.jpg")
        );
    }

    #[test]
    fn skips_complete_variants_unless_overwriting() {
        let dir = setup(&["DSCF0001.RAF", "DSCF0002.RAF"]);
        let out = dir.path().join(DEFAULT_OUTPUT_DIR).join("C1 Kodachrome");
        fs::create_dir_all(&out).unwrap();
        fs::write(
            out.join(MANIFEST),
            serde_json::to_string(&presets()[1].manifest()).unwrap(),
        )
        .unwrap();
        fs::write(out.join("DSCF0001.jpg"), JPEG).unwrap();
        fs::write(out.join("DSCF0002.jpg"), &JPEG[..4]).unwrap(); // truncated: not complete
        let plan = build_plan(&request(dir.path(), &["C1"]), &presets()).unwrap();
        assert_eq!((plan.already_done, plan.pending.len()), (1, 1));
        // Choosing the RAF folder itself as output still finds the variants.
        let mut raf_folder = request(dir.path(), &["C1"]);
        raf_folder.output_dir = Some(dir.path().to_string_lossy().into_owned());
        let plan = build_plan(&raf_folder, &presets()).unwrap();
        assert_eq!(plan.output_dir, dir.path().join(DEFAULT_OUTPUT_DIR));
        assert_eq!((plan.already_done, plan.pending.len()), (1, 1));
        assert_eq!(plan.warnings.len(), 1, "{:?}", plan.warnings);
        // An unrelated empty output folder is used as is.
        let elsewhere = tempfile::tempdir().unwrap();
        let mut other = request(dir.path(), &["C1"]);
        other.output_dir = Some(elsewhere.path().to_string_lossy().into_owned());
        let plan = build_plan(&other, &presets()).unwrap();
        assert_eq!(plan.output_dir, elsewhere.path());
        assert_eq!(plan.already_done, 0);
        let mut again = request(dir.path(), &["C1"]);
        again.overwrite = true;
        assert_eq!(build_plan(&again, &presets()).unwrap().pending.len(), 2);
    }

    #[test]
    fn the_output_folder_is_validated() {
        let dir = setup(&["DSCF0001.RAF"]);
        let photos = dir.path().join("shooting");
        fs::create_dir_all(&photos).unwrap();
        fs::write(photos.join("DSCF0001.RAF"), b"raf").unwrap();
        let with_output = |output: &Path| {
            let mut req = request(&photos, &["asshot"]);
            req.output_dir = Some(output.to_string_lossy().into_owned());
            build_plan(&req, &presets())
        };
        let rejected = |output: &Path, why: &str| {
            let error = with_output(output).unwrap_err();
            assert_eq!(error.code, ErrorCode::OutputFolderRejected, "{output:?}");
            assert!(error.detail.contains(why), "{output:?}: {}", error.detail);
            assert!(!error.retriable);
        };
        // La cartella dei RAF (anche scritta in un altro modo) e chi la contiene.
        rejected(&photos, "cartella dei RAF");
        rejected(&photos.join("sub").join(".."), "cartella dei RAF");
        rejected(dir.path(), "contenere la cartella dei RAF");
        // La radice del disco.
        let root: PathBuf = photos.ancestors().last().unwrap().to_path_buf();
        rejected(&root, "radice");
        // Un percorso relativo o un file.
        rejected(Path::new("varianti"), "percorso completo");
        rejected(&photos.join("DSCF0001.RAF"), "è un file");
        // Le cartelle di sistema e le loro sottocartelle.
        #[cfg(windows)]
        let system = PathBuf::from(std::env::var_os("SystemRoot").unwrap()).join("Varianti");
        #[cfg(not(windows))]
        let system = PathBuf::from("/usr/share/varianti");
        rejected(&system, "cartella di sistema");
        // Vanno bene: la predefinita, una sottocartella qualsiasi, una cartella accanto, un
        // altro disco (qui: un'altra cartella temporanea), anche se non esiste ancora.
        assert!(with_output(&photos.join(DEFAULT_OUTPUT_DIR)).is_ok());
        assert!(with_output(&photos.join("a").join("b")).is_ok());
        assert!(with_output(&dir.path().join("varianti")).is_ok());
        let elsewhere = tempfile::tempdir().unwrap();
        assert!(with_output(&elsewhere.path().join("nuova")).is_ok());
        // Niente è stato scritto: la pianificazione non crea cartelle.
        assert!(!elsewhere.path().join("nuova").exists());
        assert!(!photos.join(DEFAULT_OUTPUT_DIR).exists());
    }

    #[test]
    fn changed_bank_settings_get_a_new_folder() {
        let dir = setup(&["DSCF0001.RAF"]);
        let out = dir.path().join(DEFAULT_OUTPUT_DIR).join("C1 Kodachrome");
        fs::create_dir_all(&out).unwrap();
        fs::write(
            out.join(MANIFEST),
            serde_json::to_string(&bank(1, "Kodachrome", 17).manifest()).unwrap(),
        )
        .unwrap();
        let plan = build_plan(&request(dir.path(), &["C1"]), &presets()).unwrap();
        assert_eq!(plan.targets[0].folder_name, "C1 Kodachrome (2)");
        assert_eq!(plan.warnings.len(), 1);
    }

    #[test]
    fn foreign_non_empty_folder_is_not_reused() {
        let dir = setup(&["DSCF0001.RAF"]);
        let out = dir.path().join(DEFAULT_OUTPUT_DIR).join("Come scattato");
        fs::create_dir_all(&out).unwrap();
        fs::write(out.join("mine.jpg"), JPEG).unwrap();
        let plan = build_plan(&request(dir.path(), &["asshot"]), &presets()).unwrap();
        assert_eq!(plan.targets[0].folder_name, "Come scattato (2)");
    }

    #[test]
    fn explicit_files_and_errors() {
        let dir = setup(&["DSCF0001.RAF", "DSCF0002.RAF"]);
        let file = dir
            .path()
            .join("DSCF0002.RAF")
            .to_string_lossy()
            .into_owned();
        let req = VariantRequest {
            sources: vec![file],
            output_dir: Some(dir.path().join("o").to_string_lossy().into_owned()),
            preset_ids: vec!["C1".into()],
            overwrite: false,
        };
        let plan = build_plan(&req, &presets()).unwrap();
        assert_eq!(plan.rafs.len(), 1);
        assert_eq!(plan.output_dir, dir.path().join("o"));
        let jpg = dir
            .path()
            .join("DSCF0001.JPG")
            .to_string_lossy()
            .into_owned();
        assert!(collect_rafs(&[jpg]).is_err());
        let empty = tempfile::tempdir().unwrap();
        assert!(collect_rafs(&[empty.path().to_string_lossy().into_owned()]).is_err());
        assert!(build_plan(&request(dir.path(), &["C9"]), &presets()).is_err());
    }

    #[test]
    fn summary_counts() {
        let dir = setup(&["DSCF0001.RAF"]);
        let summary = build_plan(&request(dir.path(), &["asshot", "C1"]), &presets())
            .unwrap()
            .summary();
        assert_eq!(
            (
                summary.raf_count,
                summary.total_conversions,
                summary.pending_conversions
            ),
            (1, 2, 2)
        );
        assert_eq!(summary.estimated_bytes, 2 * estimate::DEFAULT_VARIANT_BYTES);
    }

    /// Simulates the user's real X-T5 banks across runs: a completed job, then
    /// one bank edited and one renamed, then the edit reverted.
    #[test]
    fn real_banks_changed_between_runs() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/xt5_banks.json");
        let Ok(text) = fs::read_to_string(path) else {
            return;
        };
        let original: Vec<Bank> = serde_json::from_str(&text).unwrap();
        let dir = setup(&["DSCF0001.RAF", "DSCF0002.RAF"]);
        let ids: Vec<String> = std::iter::once("asshot".to_string())
            .chain(original.iter().map(|b| format!("C{}", b.index)))
            .collect();
        let ids: Vec<&str> = ids.iter().map(String::as_str).collect();
        let defs = |banks: &[Bank]| -> Vec<PresetDef> {
            std::iter::once(PresetDef::as_shot())
                .chain(banks.iter().cloned().map(PresetDef::bank))
                .collect()
        };
        // Write what a finished run leaves behind: manifest + complete JPEGs.
        let complete = |plan: &Plan| {
            for target in &plan.targets {
                fs::create_dir_all(&target.dir).unwrap();
                fs::write(
                    target.dir.join(MANIFEST),
                    serde_json::to_string(&target.preset.manifest()).unwrap(),
                )
                .unwrap();
            }
            for conversion in &plan.pending {
                fs::write(plan.output_path(*conversion), JPEG).unwrap();
            }
        };

        let first = build_plan(&request(dir.path(), &ids), &defs(&original)).unwrap();
        assert_eq!(first.pending.len(), 16);
        complete(&first);

        let mut edited = original.clone();
        let reggie = edited.iter_mut().find(|b| b.name == "REGGIE").unwrap();
        reggie.props.insert(0xD192, 16); // Eterna
        reggie.props.insert(0xD198, 3); // smooth skin strong
        let cuban = edited
            .iter_mut()
            .find(|b| b.name == "CLASSIC CUBAN")
            .unwrap();
        cuban.name = "CUBA 2".into();
        let second = build_plan(&request(dir.path(), &ids), &defs(&edited)).unwrap();
        let folders: Vec<&str> = second
            .targets
            .iter()
            .map(|t| t.folder_name.as_str())
            .collect();
        assert!(folders.contains(&"C5 REGGIE (2)"), "{folders:?}");
        assert!(folders.contains(&"C7 CUBA 2"), "{folders:?}");
        assert_eq!(second.already_done, 12, "6 unchanged presets x 2 RAF");
        assert_eq!(second.pending.len(), 4);
        // The old REGGIE variants are untouched.
        let old = dir.path().join(DEFAULT_OUTPUT_DIR).join("C5 REGGIE");
        assert!(old.join("DSCF0001.jpg").exists());
        complete(&second);

        // Reverting the edit reuses the original, already complete folder.
        let third = build_plan(&request(dir.path(), &ids), &defs(&original)).unwrap();
        let reggie = third
            .targets
            .iter()
            .find(|t| t.folder_name.starts_with("C5"))
            .unwrap();
        assert_eq!(reggie.folder_name, "C5 REGGIE");
        assert_eq!(third.pending.len(), 0);
    }
}
