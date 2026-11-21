//! Sessione di revisione: valutazioni, varianti migliori e salvataggio atomico.
//!
//! Porting dello schema `Project` di FujiView (a798987: `types.ts`, `project.ts`
//! e `persist.rs`) come **versione 2** (design D4), con i campi aggiunti
//! `layout` e `variantsOutputDir`. I file v1 (anche i `.fujiview`, stesso JSON)
//! si leggono e si aggiornano in memoria.
//!
//! Le sessioni stanno nella cartella dati dell'app, mai nella cartella delle
//! foto: `<dati app>/raw-review/sessions/<blake3(root canonico)>.json`, più un
//! indice `<dati app>/raw-review/recent.json` delle sessioni recenti. Il
//! percorso base arriva come parametro (l'app passa la cartella di `AppPaths`).

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use pc_core::error::{AppError, AppResult, ErrorCode};
use serde::{Deserialize, Serialize};

use crate::scan::{Layout, ReviewIndex, RAW_PREVIEW_PRESET};

/// Versione corrente dello schema della sessione.
pub const SCHEMA_VERSION: u32 = 2;

/// Un pannello per preset, fino ai tasti numerici 1-9.
pub const MAX_PANELS: usize = 9;

/// Numero massimo di sessioni nell'indice delle recenti.
pub const MAX_RECENT: usize = 20;

/// Decisione su uno scatto.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ShotDecision {
    pub key: String,
    /// Valutazione da 0 a 5.
    pub rating: u8,
    /// Preset della variante migliore; mai `raw-preview`.
    pub winner_preset: Option<String>,
    pub reviewed: bool,
}

impl ShotDecision {
    fn empty(key: &str) -> Self {
        Self {
            key: key.to_string(),
            rating: 0,
            winner_preset: None,
            reviewed: false,
        }
    }
}

/// Sessione di revisione di una cartella (schema v2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub schema_version: u32,
    pub name: String,
    pub source_root: String,
    /// Forma della cartella (assente nei file v1 finché non si fa il merge).
    #[serde(default)]
    pub layout: Option<Layout>,
    /// Cartella di output scelta per le varianti (nuovo in v2).
    #[serde(default)]
    pub variants_output_dir: Option<String>,
    pub presets: Vec<String>,
    #[serde(default, deserialize_with = "empty_as_none")]
    pub guide_preset: Option<String>,
    pub panel_presets: Vec<String>,
    #[serde(default)]
    pub hidden_presets: Vec<String>,
    #[serde(default)]
    pub decisions: BTreeMap<String, ShotDecision>,
    #[serde(default)]
    pub last_shot_key: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Il preset guida v1 è una stringa (a volte vuota): la vuota diventa `None`.
fn empty_as_none<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    let value = Option::<String>::deserialize(d)?;
    Ok(value.filter(|text| !text.is_empty()))
}

/// Preset in ordine di pannello: la guida per prima, poi gli altri.
fn ordered_presets(index: &ReviewIndex) -> Vec<String> {
    let guide = index.guide_preset.as_deref();
    guide
        .map(str::to_string)
        .into_iter()
        .chain(
            index
                .presets
                .iter()
                .filter(|preset| Some(preset.as_str()) != guide)
                .cloned(),
        )
        .collect()
}

impl Session {
    /// Nuova sessione per l'indice (porting di `createProject`).
    pub fn create(index: &ReviewIndex) -> Self {
        let now = now_iso();
        let decisions = index
            .shots
            .iter()
            .map(|shot| (shot.key.clone(), ShotDecision::empty(&shot.key)))
            .collect();
        let name = Path::new(&index.root)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| "Nuova sessione".to_string());
        let mut panels = ordered_presets(index);
        panels.truncate(MAX_PANELS);
        Self {
            schema_version: SCHEMA_VERSION,
            name,
            source_root: index.root.clone(),
            layout: Some(index.layout.clone()),
            variants_output_dir: match &index.layout {
                Layout::RafWithVariants { output_dir } => Some(output_dir.clone()),
                _ => None,
            },
            presets: index.presets.clone(),
            guide_preset: index.guide_preset.clone(),
            panel_presets: panels,
            hidden_presets: Vec::new(),
            decisions,
            last_shot_key: index.shots.first().map(|s| s.key.clone()),
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// Legge una sessione v1 o v2 da JSON e la porta a v2 in memoria.
    pub fn from_json(bytes: &[u8]) -> AppResult<Self> {
        let invalid = |e: serde_json::Error| {
            AppError::new(
                ErrorCode::InvalidConfig,
                format!("File di sessione non valido: {e}"),
            )
        };
        let value: serde_json::Value = serde_json::from_slice(bytes).map_err(invalid)?;
        let version = value
            .get("schemaVersion")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(1);
        if version > u64::from(SCHEMA_VERSION) {
            return Err(AppError::new(
                ErrorCode::UnsupportedFormat,
                format!(
                    "La sessione ha versione {version}, più recente di quella supportata ({SCHEMA_VERSION}): aggiorna l'applicazione."
                ),
            ));
        }
        let mut session: Session = serde_json::from_value(value).map_err(invalid)?;
        session.schema_version = SCHEMA_VERSION;
        Ok(session)
    }

    /// Aggiunge scatti e preset nuovi senza perdere le scelte (porting di
    /// `mergeProject`). I preset nascosti restano nascosti, quelli nuovi si
    /// mostrano; l'ordine dei pannelli salvato è mantenuto.
    pub fn merge(&mut self, index: &ReviewIndex) {
        for shot in &index.shots {
            self.decisions
                .entry(shot.key.clone())
                .or_insert_with(|| ShotDecision::empty(&shot.key));
        }
        let mut hidden: Vec<String> = self
            .hidden_presets
            .iter()
            .filter(|p| index.presets.contains(p))
            .cloned()
            .collect();
        if hidden.len() >= index.presets.len() {
            hidden.retain(|p| Some(p) != index.guide_preset.as_ref());
        }
        let mut panels: Vec<String> = Vec::new();
        for preset in &self.panel_presets {
            if index.presets.contains(preset)
                && !hidden.contains(preset)
                && !panels.contains(preset)
            {
                panels.push(preset.clone());
            }
        }
        for preset in ordered_presets(index) {
            if !panels.contains(&preset) && !hidden.contains(&preset) {
                panels.push(preset);
            }
        }
        panels.truncate(MAX_PANELS);

        self.source_root = index.root.clone();
        self.layout = Some(index.layout.clone());
        if let Layout::RafWithVariants { output_dir } = &index.layout {
            self.variants_output_dir = Some(output_dir.clone());
        }
        self.presets = index.presets.clone();
        self.guide_preset = index.guide_preset.clone();
        self.panel_presets = panels;
        self.hidden_presets = hidden;
        // Una scelta su un preset sparito non è più valida.
        for decision in self.decisions.values_mut() {
            if decision
                .winner_preset
                .as_ref()
                .is_some_and(|p| !index.presets.contains(p))
            {
                decision.winner_preset = None;
            }
        }
        self.schema_version = SCHEMA_VERSION;
        self.touch();
    }

    fn touch(&mut self) {
        self.updated_at = now_iso();
    }

    fn decision_mut(&mut self, key: &str) -> AppResult<&mut ShotDecision> {
        self.decisions.get_mut(key).ok_or_else(|| {
            AppError::new(
                ErrorCode::NotFound,
                format!("Scatto «{key}» non trovato nella sessione."),
            )
        })
    }

    /// Imposta o toglie (`None`) la variante migliore. Rifiuta l'anteprima RAW
    /// (non selezionabile) e i preset che la sessione non conosce.
    pub fn set_winner(&mut self, key: &str, preset: Option<&str>) -> AppResult<()> {
        if let Some(preset) = preset {
            if preset == RAW_PREVIEW_PRESET {
                return Err(AppError::invalid_input(
                    "L'anteprima RAW non può essere scelta come variante migliore: è solo un riferimento.",
                ));
            }
            if !self.presets.iter().any(|p| p == preset) {
                return Err(AppError::invalid_input(format!(
                    "Il preset «{preset}» non esiste in questa sessione."
                )));
            }
        }
        let decision = self.decision_mut(key)?;
        decision.winner_preset = preset.map(str::to_string);
        self.touch();
        Ok(())
    }

    /// Imposta la valutazione (0-5).
    pub fn set_rating(&mut self, key: &str, rating: u8) -> AppResult<()> {
        if rating > 5 {
            return Err(AppError::invalid_input(format!(
                "Valutazione {rating} non valida: deve essere tra 0 e 5."
            )));
        }
        self.decision_mut(key)?.rating = rating;
        self.touch();
        Ok(())
    }

    pub fn set_reviewed(&mut self, key: &str, reviewed: bool) -> AppResult<()> {
        self.decision_mut(key)?.reviewed = reviewed;
        self.touch();
        Ok(())
    }

    /// Ricorda l'ultimo scatto visto.
    pub fn set_last_shot(&mut self, key: Option<&str>) {
        self.last_shot_key = key.map(str::to_string);
        self.touch();
    }

    /// Mostra o nasconde il pannello di un preset (porting di
    /// `togglePresetVisibility`). L'ultimo pannello visibile non si nasconde e
    /// i pannelli sono al massimo [`MAX_PANELS`]. Ritorna `true` se è cambiato.
    pub fn toggle_preset_visibility(&mut self, preset: &str) -> bool {
        let shown = self.panel_presets.iter().any(|p| p == preset);
        if shown {
            if self.panel_presets.len() <= 1 {
                return false;
            }
            self.panel_presets.retain(|p| p != preset);
            self.hidden_presets.retain(|p| p != preset);
            self.hidden_presets.push(preset.to_string());
        } else {
            if self.panel_presets.len() >= MAX_PANELS || !self.presets.iter().any(|p| p == preset) {
                return false;
            }
            self.panel_presets.push(preset.to_string());
            self.hidden_presets.retain(|p| p != preset);
        }
        self.touch();
        true
    }

    /// Mostra di nuovo tutti i preset (fino a [`MAX_PANELS`]).
    pub fn show_all_presets(&mut self) {
        let extra: Vec<String> = self
            .presets
            .iter()
            .filter(|p| !self.panel_presets.contains(p))
            .cloned()
            .collect();
        self.panel_presets.extend(extra);
        self.panel_presets.truncate(MAX_PANELS);
        self.hidden_presets.clear();
        self.touch();
    }

    /// Scatti con una variante scelta.
    pub fn winner_count(&self) -> usize {
        self.decisions
            .values()
            .filter(|d| d.winner_preset.is_some())
            .count()
    }
}

// ---------------------------------------------------------------------------
// Percorsi e persistenza
// ---------------------------------------------------------------------------

/// Voce dell'indice delle sessioni recenti.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct RecentSession {
    pub source_root: String,
    pub name: String,
    /// Ultimo uso, ISO 8601 UTC.
    pub last_used_at: String,
    pub shot_count: usize,
    /// Scatti con una variante scelta.
    pub choice_count: usize,
}

fn review_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("raw-review")
}

fn recent_path(data_dir: &Path) -> PathBuf {
    review_dir(data_dir).join("recent.json")
}

/// Hash blake3 del percorso canonico della cartella (minuscolo su Windows,
/// dove il filesystem non distingue le maiuscole).
fn root_hash(root: &str) -> String {
    let canonical = fs::canonicalize(root)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| root.to_string());
    let normalized = if cfg!(windows) {
        canonical.to_lowercase()
    } else {
        canonical
    };
    blake3::hash(normalized.as_bytes()).to_hex().to_string()
}

/// File della sessione di una cartella.
pub fn session_path(data_dir: &Path, root: &str) -> PathBuf {
    review_dir(data_dir)
        .join("sessions")
        .join(format!("{}.json", root_hash(root)))
}

fn backup_path(target: &Path) -> PathBuf {
    let mut name = target.file_name().unwrap_or_default().to_os_string();
    name.push(".bak");
    target.with_file_name(name)
}

/// Scrive il JSON accanto alla destinazione e lo scambia, tenendo una copia
/// `.bak` della versione precedente (porting di `write_json_atomic`).
pub fn write_json_atomic<T: Serialize>(target: &Path, value: &T) -> AppResult<()> {
    let parent = target
        .parent()
        .ok_or_else(|| AppError::invalid_input("Percorso non valido"))?;
    fs::create_dir_all(parent)?;
    let mut temp_name = target.file_name().unwrap_or_default().to_os_string();
    temp_name.push(".tmp");
    let temp = target.with_file_name(temp_name);
    let backup = backup_path(target);
    let bytes = serde_json::to_vec_pretty(value)?;
    fs::write(&temp, bytes)?;
    if !target.exists() {
        return rename_retry(&temp, target).map_err(Into::into);
    }
    if backup.exists() {
        fs::remove_file(&backup)?;
    }
    rename_retry(target, &backup)?;
    if let Err(error) = rename_retry(&temp, target) {
        let _ = rename_retry(&backup, target);
        return Err(error.into());
    }
    Ok(())
}

/// Rinomina con qualche nuovo tentativo: su Windows antivirus e indicizzatore
/// possono tenere il file aperto per un istante (errore di condivisione).
fn rename_retry(from: &Path, to: &Path) -> std::io::Result<()> {
    let mut attempt = 0;
    loop {
        match fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(_) if attempt < 8 => {
                attempt += 1;
                std::thread::sleep(std::time::Duration::from_millis(20 * attempt));
            }
            Err(error) => return Err(error),
        }
    }
}

/// Legge un JSON usando il `.bak` se il file principale manca o è corrotto.
/// `None` se non esiste né l'uno né l'altro.
fn read_with_backup<T>(
    target: &Path,
    parse: impl Fn(&[u8]) -> AppResult<T>,
) -> AppResult<Option<T>> {
    let mut last_error = None;
    for candidate in [target.to_path_buf(), backup_path(target)] {
        match fs::read(&candidate) {
            Ok(bytes) => match parse(&bytes) {
                Ok(value) => return Ok(Some(value)),
                Err(error) => last_error = Some(error),
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => last_error = Some(error.into()),
        }
    }
    last_error.map_or(Ok(None), Err)
}

/// Carica la sessione salvata per la cartella, se c'è.
pub fn load(data_dir: &Path, root: &str) -> AppResult<Option<Session>> {
    read_with_backup(&session_path(data_dir, root), Session::from_json)
}

/// Legge un file di progetto FujiView (`.fujiview`, v1) senza salvarlo.
pub fn read_fujiview_file(path: &Path) -> AppResult<Session> {
    Session::from_json(&fs::read(path)?)
}

/// Apre la sessione della cartella: carica quella salvata (o ne crea una),
/// la fonde con l'indice e la salva.
pub fn open(data_dir: &Path, index: &ReviewIndex) -> AppResult<Session> {
    let session = match load(data_dir, &index.root)? {
        Some(mut saved) => {
            saved.merge(index);
            saved
        }
        None => Session::create(index),
    };
    save(data_dir, &session)?;
    Ok(session)
}

/// Salva la sessione e aggiorna l'indice delle recenti.
pub fn save(data_dir: &Path, session: &Session) -> AppResult<()> {
    write_json_atomic(&session_path(data_dir, &session.source_root), session)?;
    record_recent(data_dir, session, &now_iso())
}

/// Aggiorna `recent.json` con la sessione, usando `used_at` come ultimo uso.
fn record_recent(data_dir: &Path, session: &Session, used_at: &str) -> AppResult<()> {
    let mut recent = list_recent(data_dir);
    recent.retain(|r| r.source_root != session.source_root);
    recent.push(RecentSession {
        source_root: session.source_root.clone(),
        name: session.name.clone(),
        last_used_at: used_at.to_string(),
        shot_count: session.decisions.len(),
        choice_count: session.winner_count(),
    });
    recent.sort_by(|a, b| b.last_used_at.cmp(&a.last_used_at));
    recent.truncate(MAX_RECENT);
    write_json_atomic(&recent_path(data_dir), &recent)
}

/// Sessioni recenti, dalla più recente. Un indice illeggibile non blocca
/// l'app: si riparte da elenco vuoto.
pub fn list_recent(data_dir: &Path) -> Vec<RecentSession> {
    let parse =
        |bytes: &[u8]| -> AppResult<Vec<RecentSession>> { Ok(serde_json::from_slice(bytes)?) };
    let mut recent = read_with_backup(&recent_path(data_dir), parse)
        .ok()
        .flatten()
        .unwrap_or_default();
    recent.sort_by(|a, b| b.last_used_at.cmp(&a.last_used_at));
    recent.truncate(MAX_RECENT);
    recent
}

/// Toglie una cartella dall'elenco delle recenti (la sessione resta salvata).
pub fn forget_recent(data_dir: &Path, root: &str) -> AppResult<()> {
    let mut recent = list_recent(data_dir);
    recent.retain(|r| r.source_root != root);
    write_json_atomic(&recent_path(data_dir), &recent)
}

// ---------------------------------------------------------------------------
// Data
// ---------------------------------------------------------------------------

/// Istante corrente in ISO 8601 UTC con millisecondi.
fn now_iso() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    iso_from_millis(i64::try_from(millis).unwrap_or(0))
}

fn iso_from_millis(millis: i64) -> String {
    let secs = millis.div_euclid(1000);
    let ms = millis.rem_euclid(1000);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    // Giorni dal 1970-01-01 a data civile (algoritmo di Howard Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{ms:03}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::{Shot, Variant, VariantKind};

    fn variant(preset: &str, key: &str) -> Variant {
        Variant {
            preset: preset.into(),
            path: format!("/x/{preset}/{key}.jpg"),
            file_name: format!("{key}.jpg"),
            extension: "jpg".into(),
            bytes: 1,
            kind: VariantKind::Image,
        }
    }

    fn index(root: &str, presets: &[&str], shots: usize) -> ReviewIndex {
        ReviewIndex {
            root: root.into(),
            layout: Layout::Presets,
            presets: presets.iter().map(|p| p.to_string()).collect(),
            guide_preset: presets.first().map(|p| p.to_string()),
            shots: (0..shots)
                .map(|i| {
                    let key = format!("dscf{i:04}");
                    Shot {
                        display_name: key.to_uppercase(),
                        variants: presets.iter().map(|p| variant(p, &key)).collect(),
                        key,
                    }
                })
                .collect(),
            warnings: vec![],
        }
    }

    fn key(i: usize) -> String {
        format!("dscf{i:04}")
    }

    #[test]
    fn iso_da_millisecondi() {
        assert_eq!(iso_from_millis(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(
            iso_from_millis(1_709_164_800_123),
            "2024-02-29T00:00:00.123Z"
        );
    }

    #[test]
    fn riapertura_con_80_scelte_su_160() {
        let data = tempfile::tempdir().unwrap();
        let photos = tempfile::tempdir().unwrap();
        let root = photos.path().to_string_lossy().into_owned();
        let idx = index(&root, &["C1", "C2", "C3"], 160);
        let mut session = open(data.path(), &idx).unwrap();
        for i in 0..80 {
            session.set_winner(&key(i), Some("C2")).unwrap();
            session.set_rating(&key(i), 4).unwrap();
        }
        session.set_last_shot(Some(&key(79)));
        save(data.path(), &session).unwrap();

        let reopened = open(data.path(), &idx).unwrap();
        assert_eq!(reopened.decisions.len(), 160);
        assert_eq!(reopened.winner_count(), 80);
        assert_eq!(reopened.decisions[&key(10)].rating, 4);
        assert_eq!(reopened.decisions[&key(100)].winner_preset, None);
        assert_eq!(reopened.last_shot_key.as_deref(), Some("dscf0079"));
        assert_eq!(reopened.schema_version, 2);
        let recent = list_recent(data.path());
        assert_eq!(recent[0].shot_count, 160);
        assert_eq!(recent[0].choice_count, 80);
    }

    #[test]
    fn nuovo_preset_non_tocca_le_scelte_e_compare_nei_pannelli() {
        let data = tempfile::tempdir().unwrap();
        let photos = tempfile::tempdir().unwrap();
        let root = photos.path().to_string_lossy().into_owned();
        let mut session = open(data.path(), &index(&root, &["C1", "C2"], 4)).unwrap();
        session.set_winner(&key(0), Some("C2")).unwrap();
        session.toggle_preset_visibility("C1");
        assert_eq!(session.hidden_presets, vec!["C1"]);
        save(data.path(), &session).unwrap();

        let reopened = open(data.path(), &index(&root, &["C1", "C2", "C5"], 4)).unwrap();
        assert_eq!(
            reopened.decisions[&key(0)].winner_preset.as_deref(),
            Some("C2")
        );
        assert_eq!(reopened.panel_presets, vec!["C2", "C5"]);
        assert_eq!(reopened.hidden_presets, vec!["C1"]);
    }

    #[test]
    fn usa_il_bak_se_il_file_principale_e_corrotto() {
        let data = tempfile::tempdir().unwrap();
        let photos = tempfile::tempdir().unwrap();
        let root = photos.path().to_string_lossy().into_owned();
        let idx = index(&root, &["C1", "C2"], 3);
        let mut session = open(data.path(), &idx).unwrap();
        session.set_winner(&key(1), Some("C1")).unwrap();
        save(data.path(), &session).unwrap(); // ora esiste il .bak
        let path = session_path(data.path(), &root);
        assert!(backup_path(&path).exists());
        fs::write(&path, b"{ corrotto").unwrap();
        let loaded = load(data.path(), &root).unwrap().unwrap();
        assert_eq!(loaded.decisions.len(), 3);
        // Se sono corrotti entrambi, errore e non una sessione vuota.
        fs::write(backup_path(&path), b"zzz").unwrap();
        assert!(load(data.path(), &root).is_err());
    }

    const FUJIVIEW_V1: &str = r#"{
  "schemaVersion": 1,
  "name": "Roma 2024",
  "sourceRoot": "D:\\Foto\\Roma",
  "presets": ["C1", "C2", "C3"],
  "guidePreset": "C1",
  "panelPresets": ["C1", "C2", "C3"],
  "decisions": {
    "dscf0001": { "key": "dscf0001", "rating": 3, "winnerPreset": "C2", "reviewed": true },
    "dscf0002": { "key": "dscf0002", "rating": 0, "winnerPreset": null, "reviewed": false }
  },
  "lastShotKey": "dscf0002",
  "createdAt": "2025-01-10T09:00:00.000Z",
  "updatedAt": "2025-01-11T10:30:00.000Z"
}"#;

    #[test]
    fn legge_un_fujiview_v1_e_lo_porta_a_v2() {
        let data = tempfile::tempdir().unwrap();
        let file = data.path().join("roma.fujiview");
        fs::write(&file, FUJIVIEW_V1).unwrap();
        let mut session = read_fujiview_file(&file).unwrap();
        assert_eq!(session.schema_version, 2);
        assert_eq!(session.layout, None);
        assert!(session.hidden_presets.is_empty());
        assert_eq!(
            session.decisions["dscf0001"].winner_preset.as_deref(),
            Some("C2")
        );
        assert_eq!(session.created_at, "2025-01-10T09:00:00.000Z");

        let idx = index("D:\\Foto\\Roma", &["C1", "C2", "C3", "C4"], 3);
        session.merge(&idx);
        assert_eq!(session.layout, Some(Layout::Presets));
        assert_eq!(session.panel_presets, vec!["C1", "C2", "C3", "C4"]);
        assert_eq!(session.decisions.len(), 3);
        assert_eq!(session.decisions["dscf0001"].rating, 3);
    }

    #[test]
    fn rifiuta_versioni_future_e_json_non_valido() {
        assert!(Session::from_json(br#"{"schemaVersion": 9}"#).is_err());
        assert!(Session::from_json(b"non json").is_err());
    }

    #[test]
    fn set_winner_rifiuta_anteprima_raw_e_preset_inesistenti() {
        let photos = tempfile::tempdir().unwrap();
        let root = photos.path().to_string_lossy().into_owned();
        let mut session = Session::create(&index(&root, &["C1"], 2));
        let err = session
            .set_winner(&key(0), Some(RAW_PREVIEW_PRESET))
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidInput);
        assert!(session.set_winner(&key(0), Some("C9")).is_err());
        assert!(session.set_winner("nessuno", Some("C1")).is_err());
        assert_eq!(session.decisions[&key(0)].winner_preset, None);
        session.set_winner(&key(0), Some("C1")).unwrap();
        session.set_winner(&key(0), None).unwrap();
        assert_eq!(session.decisions[&key(0)].winner_preset, None);
    }

    #[test]
    fn valutazione_oltre_cinque_rifiutata() {
        let mut session = Session::create(&index("/r", &["C1"], 1));
        assert!(session.set_rating(&key(0), 6).is_err());
        session.set_rating(&key(0), 5).unwrap();
        assert_eq!(session.decisions[&key(0)].rating, 5);
    }

    #[test]
    fn nessun_file_nella_cartella_delle_foto() {
        let data = tempfile::tempdir().unwrap();
        let photos = tempfile::tempdir().unwrap();
        fs::write(photos.path().join("a.txt"), b"x").unwrap();
        let root = photos.path().to_string_lossy().into_owned();
        let mut session = open(data.path(), &index(&root, &["C1"], 2)).unwrap();
        session.set_rating(&key(0), 2).unwrap();
        save(data.path(), &session).unwrap();
        assert_eq!(fs::read_dir(photos.path()).unwrap().count(), 1);
        assert!(session_path(data.path(), &root).starts_with(data.path()));
    }

    #[test]
    fn recenti_ordinati_e_limitati() {
        let data = tempfile::tempdir().unwrap();
        for i in 0..25 {
            let session = Session::create(&index(&format!("/foto/{i}"), &["C1"], 1));
            record_recent(
                data.path(),
                &session,
                &iso_from_millis(1_000_000 + i * 1000),
            )
            .unwrap();
        }
        // Riuso di una vecchia: torna in testa, senza duplicati.
        let session = Session::create(&index("/foto/10", &["C1"], 1));
        record_recent(data.path(), &session, &iso_from_millis(9_000_000)).unwrap();
        let recent = list_recent(data.path());
        assert_eq!(recent.len(), MAX_RECENT);
        assert_eq!(recent[0].source_root, "/foto/10");
        assert_eq!(
            recent
                .iter()
                .filter(|r| r.source_root == "/foto/10")
                .count(),
            1
        );
        assert_eq!(recent[1].source_root, "/foto/24");
        forget_recent(data.path(), "/foto/10").unwrap();
        assert_eq!(list_recent(data.path()).len(), MAX_RECENT - 1);
    }

    #[test]
    fn al_massimo_nove_pannelli() {
        let presets: Vec<String> = (1..=12).map(|i| format!("P{i:02}")).collect();
        let refs: Vec<&str> = presets.iter().map(String::as_str).collect();
        let mut session = Session::create(&index("/r", &refs, 1));
        assert_eq!(session.panel_presets.len(), MAX_PANELS);
        assert!(!session.toggle_preset_visibility("P10"));
        session.show_all_presets();
        assert_eq!(session.panel_presets.len(), MAX_PANELS);
        assert!(session.toggle_preset_visibility("P03"));
        assert!(session.toggle_preset_visibility("P10"));
        assert_eq!(session.panel_presets.len(), MAX_PANELS);
        session.merge(&index("/r", &refs, 1));
        assert_eq!(session.panel_presets.len(), MAX_PANELS);
        // L'ultimo pannello visibile non si nasconde.
        let mut one = Session::create(&index("/r", &["C1"], 1));
        assert!(!one.toggle_preset_visibility("C1"));
    }

    #[test]
    fn forma_json_v2() {
        let session = Session::create(&index("/r", &["C1"], 1));
        let v = serde_json::to_value(&session).unwrap();
        assert_eq!(v["schemaVersion"], 2);
        assert_eq!(v["layout"]["kind"], "presets");
        assert!(v["variantsOutputDir"].is_null());
        assert!(v["decisions"]["dscf0000"]["winnerPreset"].is_null());
    }
}
