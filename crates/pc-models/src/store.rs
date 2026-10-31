//! Cartella dei modelli installati: verifica, ripristino dagli asset inclusi
//! nell'app, download in background e installazione da file locale.

use crate::download::{self, sha256_file, RetryPolicy};
use crate::manifest::{Entry, Manifest};
use parking_lot::Mutex;
use pc_core::{AppError, AppResult, ErrorCode};
use pc_jobs::{JobContext, JobPlan, PhaseSpec, RunSummary};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::UNIX_EPOCH;

pub const JOB_KIND: &str = "models";
pub const PHASE_VERIFY: &str = "verifica";
pub const PHASE_DOWNLOAD: &str = "download";
const CACHE_FILE: &str = ".verified.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ModelState {
    Missing,
    Downloading {
        done_bytes: u64,
        total_bytes: u64,
        #[serde(default)]
        bytes_per_second: u64,
    },
    Verified,
    Error {
        code: String,
        message: String,
    },
    SelfTestFailed {
        message: String,
    },
    UpdateAvailable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SelfTestResult {
    pub passed: bool,
    pub avg_ms: f64,
    pub message: String,
    /// ms dall'epoca.
    pub at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ModelStatus {
    pub entry: Entry,
    pub state: ModelState,
    pub self_test: Option<SelfTestResult>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct CacheRow {
    #[serde(default)]
    file: String,
    size: u64,
    mtime: u64,
    sha256: String,
    version: String,
    /// Ultimo self-test del file con questo hash.
    #[serde(default)]
    self_test: Option<SelfTestResult>,
}

pub type StatusSink = Arc<dyn Fn(&[ModelStatus]) + Send + Sync>;

pub struct ModelStore {
    manifest: Manifest,
    dir: PathBuf,
    bundled: Option<PathBuf>,
    state: Mutex<BTreeMap<String, (ModelState, Option<SelfTestResult>)>>,
    cache: Mutex<BTreeMap<String, CacheRow>>,
    sink: Mutex<Option<StatusSink>>,
    operation: Mutex<()>,
    pub retry: RetryPolicy,
}

fn mtime_of(meta: &std::fs::Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

impl ModelStore {
    /// `dir` = modelli installati (dati utente); `bundled` = asset inclusi nell'app.
    pub fn new(manifest: Manifest, dir: impl Into<PathBuf>, bundled: Option<PathBuf>) -> Self {
        let dir = dir.into();
        let cache = std::fs::read(dir.join(CACHE_FILE))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let state = manifest
            .entries
            .iter()
            .map(|e| (e.id.clone(), (ModelState::Missing, None)))
            .collect();
        Self {
            manifest,
            dir,
            bundled,
            state: Mutex::new(state),
            cache: Mutex::new(cache),
            sink: Mutex::new(None),
            operation: Mutex::new(()),
            retry: RetryPolicy::default(),
        }
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    pub fn set_sink(&self, sink: StatusSink) {
        *self.sink.lock() = Some(sink);
    }

    pub fn file_path(&self, e: &Entry) -> PathBuf {
        self.dir.join(&e.file)
    }

    fn gpu_trial_marker(&self, id: &str) -> PathBuf {
        self.dir.join(".gpu-trial").join(id)
    }

    fn gpu_blocked_file(&self) -> PathBuf {
        self.dir.join(".gpu-blocked.json")
    }

    fn gpu_blocked(&self) -> Vec<String> {
        std::fs::read(self.gpu_blocked_file())
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    /// Il modello può girare su GPU: lo consente il manifest e non ha mai causato un arresto.
    pub fn allow_gpu(&self, id: &str) -> bool {
        self.manifest.get(id).is_some_and(|e| e.gpu) && !self.gpu_blocked().iter().any(|b| b == id)
    }

    /// Da chiamare prima del primo uso su GPU: se il processo termina durante la prova,
    /// al riavvio il marker è ancora presente e la GPU viene disattivata per quel modello.
    pub fn begin_gpu_trial(&self, id: &str) {
        let m = self.gpu_trial_marker(id);
        let _ = std::fs::create_dir_all(m.parent().expect("cartella marker"));
        let _ = std::fs::write(m, b"in corso");
    }

    pub fn end_gpu_trial(&self, id: &str) {
        let _ = std::fs::remove_file(self.gpu_trial_marker(id));
    }

    /// All'avvio: le prove GPU rimaste aperte indicano un arresto anomalo durante l'inferenza.
    pub fn recover_gpu_trials(&self) -> Vec<String> {
        let Ok(entries) = std::fs::read_dir(self.dir.join(".gpu-trial")) else {
            return Vec::new();
        };
        let crashed: Vec<String> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        if crashed.is_empty() {
            return crashed;
        }
        let mut blocked = self.gpu_blocked();
        for id in &crashed {
            tracing::warn!(
                code = "MODEL_UNAVAILABLE",
                model = id.as_str(),
                "il modello si è arrestato sulla GPU nella sessione precedente: userà la CPU"
            );
            if !blocked.contains(id) {
                blocked.push(id.clone());
            }
            self.end_gpu_trial(id);
        }
        let _ = std::fs::create_dir_all(&self.dir);
        let _ = std::fs::write(
            self.gpu_blocked_file(),
            serde_json::to_vec(&blocked).unwrap_or_default(),
        );
        // Il self-test va ripetuto sulla CPU.
        for id in &crashed {
            if let Some(row) = self.cache.lock().get_mut(id) {
                row.self_test = None;
            }
        }
        self.save_cache();
        crashed
    }

    /// Modelli verificati senza un self-test registrato per il file attuale.
    pub fn needing_self_test(&self) -> Vec<String> {
        let st = self.state.lock();
        self.manifest
            .entries
            .iter()
            .filter(|e| matches!(st.get(&e.id), Some((ModelState::Verified, None))))
            .map(|e| e.id.clone())
            .collect()
    }

    /// Percorso del file anche se non (ancora) verificato: per eseguire il self-test.
    pub fn installed_path(&self, id: &str) -> Option<PathBuf> {
        self.manifest
            .get(id)
            .map(|e| self.file_path(e))
            .filter(|p| p.is_file())
    }

    /// Percorso del modello solo se verificato.
    pub fn verified_path(&self, id: &str) -> Option<PathBuf> {
        let e = self.manifest.get(id)?;
        matches!(self.state.lock().get(id), Some((ModelState::Verified, _)))
            .then(|| self.file_path(e))
    }

    pub fn statuses(&self) -> Vec<ModelStatus> {
        let st = self.state.lock();
        self.manifest
            .entries
            .iter()
            .map(|e| {
                let (state, test) = st
                    .get(&e.id)
                    .cloned()
                    .unwrap_or((ModelState::Missing, None));
                ModelStatus {
                    entry: e.clone(),
                    state,
                    self_test: test,
                }
            })
            .collect()
    }

    fn set_state(&self, id: &str, state: ModelState) {
        let test = if state == ModelState::Verified {
            self.cache.lock().get(id).and_then(|c| c.self_test.clone())
        } else {
            None
        };
        let state = match &test {
            Some(t) if !t.passed => ModelState::SelfTestFailed {
                message: t.message.clone(),
            },
            _ => state,
        };
        self.state.lock().insert(id.to_string(), (state, test));
        self.notify();
    }

    pub fn set_self_test(&self, id: &str, r: SelfTestResult) {
        if let Some(row) = self.cache.lock().get_mut(id) {
            row.self_test = Some(r.clone());
        }
        self.save_cache();
        {
            let mut st = self.state.lock();
            let slot = st
                .entry(id.to_string())
                .or_insert((ModelState::Missing, None));
            if !r.passed {
                slot.0 = ModelState::SelfTestFailed {
                    message: r.message.clone(),
                };
            } else if matches!(slot.0, ModelState::SelfTestFailed { .. }) {
                slot.0 = ModelState::Verified;
            }
            slot.1 = Some(r);
        }
        self.notify();
    }

    fn notify(&self) {
        let sink = self.sink.lock().clone();
        if let Some(s) = sink {
            s(&self.statuses());
        }
    }

    fn save_cache(&self) {
        let _ = std::fs::create_dir_all(&self.dir);
        if let Ok(json) = serde_json::to_vec_pretty(&*self.cache.lock()) {
            let _ = std::fs::write(self.dir.join(CACHE_FILE), json);
        }
    }

    /// Verifica un file (hash ricalcolato solo se dimensione o data sono cambiate).
    fn verify_entry(&self, e: &Entry) -> ModelState {
        let path = self.file_path(e);
        let Ok(meta) = std::fs::metadata(&path) else {
            return ModelState::Missing;
        };
        let (size, mtime) = (meta.len(), mtime_of(&meta));
        let cached = self.cache.lock().get(&e.id).cloned();
        let sha = match cached {
            Some(c) if c.file == e.file && c.size == size && c.mtime == mtime => c.sha256,
            _ => match sha256_file(&path) {
                Ok(s) => s,
                Err(err) => {
                    return ModelState::Error {
                        code: err.code.as_str().into(),
                        message: err.detail,
                    }
                }
            },
        };
        if size == e.size_bytes && sha.eq_ignore_ascii_case(&e.sha256) {
            let mut cache = self.cache.lock();
            let prev = cache
                .get(&e.id)
                .filter(|c| {
                    c.file == e.file
                        && c.version == e.version
                        && c.sha256.eq_ignore_ascii_case(&sha)
                })
                .and_then(|c| c.self_test.clone());
            cache.insert(
                e.id.clone(),
                CacheRow {
                    file: e.file.clone(),
                    size,
                    mtime,
                    sha256: sha,
                    version: e.version.clone(),
                    self_test: prev,
                },
            );
            return ModelState::Verified;
        }
        let old_version = self.cache.lock().get(&e.id).map(|c| c.version.clone());
        match old_version {
            Some(v) if v != e.version => ModelState::UpdateAvailable,
            _ => {
                tracing::warn!(code = "CHECKSUM_MISMATCH", path = %path.display(), model = e.id.as_str(), "modello su disco corrotto");
                ModelState::Error {
                    code: ErrorCode::ChecksumMismatch.as_str().into(),
                    message: "SHA-256 non corrispondente".into(),
                }
            }
        }
    }

    /// Verifica tutti i modelli; restituisce gli id da ripristinare.
    pub fn check_all(&self) -> Vec<String> {
        let _operation = self.operation.lock();
        let mut todo = Vec::new();
        for e in &self.manifest.entries {
            let mut s = self.verify_entry(e);
            if s != ModelState::Verified {
                todo.push(e.id.clone());
            }
            let test = (s == ModelState::Verified)
                .then(|| {
                    self.cache
                        .lock()
                        .get(&e.id)
                        .filter(|c| {
                            c.file == e.file
                                && c.version == e.version
                                && c.sha256.eq_ignore_ascii_case(&e.sha256)
                        })
                        .and_then(|c| c.self_test.clone())
                })
                .flatten();
            if s == ModelState::Verified {
                if let Some(t) = test.as_ref().filter(|t| !t.passed) {
                    s = ModelState::SelfTestFailed {
                        message: t.message.clone(),
                    };
                }
            }
            self.state.lock().insert(e.id.clone(), (s, test));
        }
        self.save_cache();
        self.notify();
        todo
    }

    /// Copia dagli asset inclusi nell'app, se presenti e integri.
    fn restore_bundled(&self, e: &Entry) -> bool {
        let Some(root) = &self.bundled else {
            return false;
        };
        let src = root.join(&e.file);
        let Ok(sha) = sha256_file(&src) else {
            return false;
        };
        if !sha.eq_ignore_ascii_case(&e.sha256) {
            tracing::warn!(code = "CHECKSUM_MISMATCH", path = %src.display(), "asset incluso non integro");
            return false;
        }
        let dest = self.file_path(e);
        let tmp = download::part_path(&dest);
        let ok = dest
            .parent()
            .map(std::fs::create_dir_all)
            .transpose()
            .is_ok()
            && std::fs::copy(&src, &tmp).is_ok()
            && std::fs::rename(&tmp, &dest).is_ok();
        if ok {
            tracing::info!(
                model = e.id.as_str(),
                "modello ripristinato dagli asset dell'app"
            );
        }
        ok
    }

    /// Installa da un file scelto dall'utente: accettato solo se lo SHA-256 corrisponde.
    pub fn install_local(&self, id: &str, file: &Path) -> AppResult<()> {
        let _operation = self.operation.lock();
        let e = self
            .manifest
            .get(id)
            .ok_or_else(|| AppError::new(ErrorCode::NotFound, format!("modello {id}")))?;
        let sha = sha256_file(file)?;
        if !sha.eq_ignore_ascii_case(&e.sha256) {
            return Err(AppError::new(
                ErrorCode::ChecksumMismatch,
                format!("il file non corrisponde al modello {id} (SHA-256 {sha})"),
            ));
        }
        let dest = self.file_path(e);
        std::fs::create_dir_all(dest.parent().expect("cartella"))?;
        let tmp = download::part_path(&dest);
        std::fs::copy(file, &tmp)?;
        std::fs::rename(&tmp, &dest)?;
        let s = self.verify_entry(e);
        self.save_cache();
        self.set_state(id, s);
        tracing::info!(model = id, "modello installato da file locale");
        Ok(())
    }

    /// Ripristina (asset inclusi → download) i modelli indicati, aggiornando progresso e stato.
    pub fn restore(
        &self,
        ids: &[String],
        ctx: Option<&JobContext>,
        cancel: &AtomicBool,
    ) -> RunSummary {
        let _operation = self.operation.lock();
        let verify = ctx.map(|c| c.progress.phase(PHASE_VERIFY));
        let dl = ctx.map(|c| c.progress.phase(PHASE_DOWNLOAD));
        let entries: Vec<Entry> = ids
            .iter()
            .filter_map(|id| self.manifest.get(id).cloned())
            .collect();
        if let Some(v) = &verify {
            v.set_total(entries.len() as i64);
        }
        let mut failed = 0;
        let checkpoint =
            || !cancel.load(Ordering::Relaxed) && ctx.is_none_or(|c| c.control.checkpoint());
        let mut to_download = Vec::new();
        for e in &entries {
            if !checkpoint() {
                return RunSummary {
                    cancelled: true,
                    failed,
                };
            }
            if self.verify_entry(e) == ModelState::Verified
                || (self.restore_bundled(e) && self.verify_entry(e) == ModelState::Verified)
            {
                self.set_state(&e.id, ModelState::Verified);
            } else {
                to_download.push(e.clone());
            }
            if let Some(v) = &verify {
                v.inc_done(1);
            }
        }
        if let Some(v) = &verify {
            v.finish();
        }
        let total: u64 = to_download.iter().map(|e| e.size_bytes).sum();
        if let Some(d) = &dl {
            d.set_total(total as i64);
        }
        let mut base = 0u64;
        for e in to_download {
            if !checkpoint() {
                return RunSummary {
                    cancelled: true,
                    failed,
                };
            }
            let dest = self.file_path(&e);
            self.set_state(
                &e.id,
                ModelState::Downloading {
                    done_bytes: 0,
                    total_bytes: e.size_bytes,
                    bytes_per_second: 0,
                },
            );
            let last_pct = Mutex::new(0u64);
            let first_progress = Mutex::new(None::<(std::time::Instant, u64)>);
            let progress = |bytes: u64| {
                let now = std::time::Instant::now();
                let mut first = first_progress.lock();
                let (start, initial_bytes) = *first.get_or_insert((now, bytes));
                let bytes_per_second = (bytes.saturating_sub(initial_bytes) as f64
                    / now.duration_since(start).as_secs_f64().max(0.001))
                    as u64;
                drop(first);
                if let Some(d) = &dl {
                    d.restore(
                        ((base + bytes) as i64).min(total as i64),
                        0,
                        total as i64,
                        true,
                    );
                    d.start();
                }
                let pct = bytes * 100 / e.size_bytes.max(1);
                let mut lp = last_pct.lock();
                if pct != *lp {
                    *lp = pct;
                    drop(lp);
                    self.set_state(
                        &e.id,
                        ModelState::Downloading {
                            done_bytes: bytes,
                            total_bytes: e.size_bytes,
                            bytes_per_second,
                        },
                    );
                }
            };
            match download::download_with_control(
                &e.urls,
                &dest,
                e.size_bytes,
                &e.sha256,
                &self.retry,
                &progress,
                &checkpoint,
            ) {
                Ok(()) => {
                    let s = self.verify_entry(&e);
                    self.set_state(&e.id, s);
                }
                Err(err) if err.code == ErrorCode::Cancelled => {
                    self.set_state(&e.id, ModelState::Missing);
                    return RunSummary {
                        cancelled: true,
                        failed,
                    };
                }
                Err(err) => {
                    failed += 1;
                    tracing::error!(
                        code = err.code.as_str(),
                        phase = PHASE_DOWNLOAD,
                        path = %dest.display(),
                        detail = err.detail.as_str(),
                        model = e.id.as_str(),
                        "download del modello non riuscito"
                    );
                    self.set_state(
                        &e.id,
                        ModelState::Error {
                            code: err.code.as_str().into(),
                            message: err.detail,
                        },
                    );
                }
            }
            base += e.size_bytes;
        }
        if let Some(d) = &dl {
            d.finish();
        }
        self.save_cache();
        RunSummary {
            cancelled: false,
            failed,
        }
    }
}

/// Job di verifica e ripristino dei modelli (parametri: `{ "ids": [...] }`, vuoto = tutti i non verificati).
pub fn plan(store: Arc<ModelStore>, params: &serde_json::Value) -> AppResult<JobPlan> {
    let ids: Vec<String> = params["ids"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    Ok(JobPlan {
        phases: vec![
            PhaseSpec::new(PHASE_VERIFY, 1.0),
            PhaseSpec::new(PHASE_DOWNLOAD, 9.0),
        ],
        min_memory_mb: 0,
        run: Box::new(move |ctx| {
            let ids = if ids.is_empty() {
                store.check_all()
            } else {
                ids
            };
            let cancel = AtomicBool::new(false);
            Ok(store.restore(&ids, Some(&ctx), &cancel))
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::download::tests::{serve, sha, Mode};

    fn entry(id: &str, file: &str, data: &[u8], urls: Vec<String>) -> Entry {
        Entry {
            id: id.into(),
            role: crate::Role::FaceDetection,
            name: id.into(),
            version: "1".into(),
            file: file.into(),
            size_bytes: data.len() as u64,
            sha256: sha(data),
            urls,
            memory_mb: 10,
            preprocessing: crate::manifest::Preprocessing::YunetBgr640,
            postprocessing: crate::manifest::Postprocessing::FaceLandmarksNms,
            gpu: true,
            license: "MIT".into(),
        }
    }

    fn quick(store: &mut ModelStore) {
        store.retry = RetryPolicy {
            attempts_per_url: 2,
            first_backoff: std::time::Duration::from_millis(5),
        };
    }

    #[test]
    fn missing_model_is_restored_from_bundle_or_downloaded() {
        let tmp = tempfile::tempdir().unwrap();
        let a: Vec<u8> = vec![1; 50_000];
        let b: Vec<u8> = vec![2; 80_000];
        // "a" è negli asset dell'app, "b" solo online.
        let bundled = tmp.path().join("bundled");
        std::fs::create_dir_all(bundled.join("x")).unwrap();
        std::fs::write(bundled.join("x/a.onnx"), &a).unwrap();
        let (url, _) = serve(b.clone(), Mode::Ok);
        let m = Manifest {
            version: 1,
            entries: vec![
                entry("a", "x/a.onnx", &a, vec![]),
                entry("b", "y/b.onnx", &b, vec![url]),
            ],
        };
        let mut store = ModelStore::new(m, tmp.path().join("models"), Some(bundled));
        quick(&mut store);
        let seen = Arc::new(Mutex::new(Vec::<Vec<ModelState>>::new()));
        let s2 = seen.clone();
        store.set_sink(Arc::new(move |st| {
            s2.lock().push(st.iter().map(|s| s.state.clone()).collect())
        }));

        assert_eq!(store.check_all(), vec!["a".to_string(), "b".to_string()]);
        assert_eq!(store.statuses()[0].state, ModelState::Missing);
        let r = store.restore(&["a".into(), "b".into()], None, &AtomicBool::new(false));
        assert_eq!(r.failed, 0);
        assert!(store.verified_path("a").is_some() && store.verified_path("b").is_some());
        assert!(seen
            .lock()
            .iter()
            .any(|v| matches!(v[1], ModelState::Downloading { .. })));

        // Riavvio: tutto verificato senza ricalcolare (cache dimensione+data).
        let again = ModelStore::new(store.manifest().clone(), tmp.path().join("models"), None);
        assert!(again.check_all().is_empty());
    }

    #[test]
    fn corrupt_file_on_disk_is_detected_and_replaced() {
        let tmp = tempfile::tempdir().unwrap();
        let data: Vec<u8> = (0..100_000u32).map(|i| i as u8).collect();
        let (url, _) = serve(data.clone(), Mode::Ok);
        let m = Manifest {
            version: 1,
            entries: vec![entry("m", "m.onnx", &data, vec![url])],
        };
        let dir = tmp.path().join("models");
        std::fs::create_dir_all(&dir).unwrap();
        let mut bad = data.clone();
        bad[10] ^= 1;
        std::fs::write(dir.join("m.onnx"), &bad).unwrap();
        let mut store = ModelStore::new(m, &dir, None);
        quick(&mut store);
        assert_eq!(store.check_all(), vec!["m".to_string()]);
        assert!(
            matches!(store.statuses()[0].state, ModelState::Error { ref code, .. } if code == "CHECKSUM_MISMATCH")
        );
        store.restore(&["m".into()], None, &AtomicBool::new(false));
        assert_eq!(std::fs::read(dir.join("m.onnx")).unwrap(), data);
        assert_eq!(store.statuses()[0].state, ModelState::Verified);
    }

    #[test]
    fn failed_download_marks_error_and_local_install_checks_hash() {
        let tmp = tempfile::tempdir().unwrap();
        let data = vec![7u8; 40_000];
        let (url, _) = serve(data.clone(), Mode::NotFound);
        let m = Manifest {
            version: 1,
            entries: vec![entry("m", "m.onnx", &data, vec![url])],
        };
        let mut store = ModelStore::new(m, tmp.path().join("models"), None);
        quick(&mut store);
        store.check_all();
        let r = store.restore(&["m".into()], None, &AtomicBool::new(false));
        assert_eq!(r.failed, 1);
        assert!(matches!(
            store.statuses()[0].state,
            ModelState::Error { .. }
        ));
        assert!(store.verified_path("m").is_none());

        let wrong = tmp.path().join("wrong.onnx");
        std::fs::write(&wrong, b"altro").unwrap();
        assert_eq!(
            store.install_local("m", &wrong).unwrap_err().code,
            ErrorCode::ChecksumMismatch
        );
        let right = tmp.path().join("right.onnx");
        std::fs::write(&right, &data).unwrap();
        store.install_local("m", &right).unwrap();
        assert_eq!(store.statuses()[0].state, ModelState::Verified);
    }

    #[test]
    fn self_test_result_survives_restart_and_failure_blocks_use() {
        let tmp = tempfile::tempdir().unwrap();
        let data = vec![3u8; 500];
        let dir = tmp.path().join("models");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("m.onnx"), &data).unwrap();
        let m = Manifest {
            version: 1,
            entries: vec![entry("m", "m.onnx", &data, vec![])],
        };
        let s = ModelStore::new(m.clone(), &dir, None);
        s.check_all();
        assert_eq!(s.needing_self_test(), vec!["m".to_string()]);
        s.set_self_test(
            "m",
            SelfTestResult {
                passed: false,
                avg_ms: 0.0,
                message: "fuori soglia".into(),
                at: 1,
            },
        );
        assert!(
            s.verified_path("m").is_none(),
            "test fallito: modello non usabile"
        );
        let again = ModelStore::new(m, &dir, None);
        again.check_all();
        assert!(matches!(
            again.statuses()[0].state,
            ModelState::SelfTestFailed { .. }
        ));
        assert!(again.needing_self_test().is_empty());
        again.set_self_test(
            "m",
            SelfTestResult {
                passed: true,
                avg_ms: 2.0,
                message: "ok".into(),
                at: 2,
            },
        );
        assert!(again.verified_path("m").is_some());
    }

    #[test]
    fn crash_during_gpu_trial_disables_gpu_for_that_model() {
        let tmp = tempfile::tempdir().unwrap();
        let data = vec![1u8; 10];
        let m = Manifest {
            version: 1,
            entries: vec![entry("m", "m.onnx", &data, vec![])],
        };
        let s = ModelStore::new(m.clone(), tmp.path(), None);
        assert!(s.allow_gpu("m"));
        s.begin_gpu_trial("m");
        // ... il processo termina qui senza end_gpu_trial ...
        let restarted = ModelStore::new(m, tmp.path(), None);
        assert_eq!(restarted.recover_gpu_trials(), vec!["m".to_string()]);
        assert!(!restarted.allow_gpu("m"));
        assert!(restarted.recover_gpu_trials().is_empty(), "marker rimosso");
    }

    #[test]
    fn new_manifest_version_shows_update_available() {
        let tmp = tempfile::tempdir().unwrap();
        let old = vec![1u8; 1000];
        let new = vec![2u8; 1000];
        let dir = tmp.path().join("models");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("m.onnx"), &old).unwrap();
        let s1 = ModelStore::new(
            Manifest {
                version: 1,
                entries: vec![entry("m", "m.onnx", &old, vec![])],
            },
            &dir,
            None,
        );
        assert!(s1.check_all().is_empty());
        let mut e2 = entry("m", "m.onnx", &new, vec![]);
        e2.version = "2".into();
        let s2 = ModelStore::new(
            Manifest {
                version: 2,
                entries: vec![e2],
            },
            &dir,
            None,
        );
        assert_eq!(s2.check_all(), vec!["m".to_string()]);
        assert_eq!(s2.statuses()[0].state, ModelState::UpdateAvailable);
        assert!(s2.statuses()[0].self_test.is_none());
    }

    #[test]
    fn a_new_manifest_path_cannot_reuse_another_files_cached_hash() {
        let tmp = tempfile::tempdir().unwrap();
        let good = vec![1; 100];
        let mut e = entry("m", "old.onnx", &good, vec![]);
        std::fs::write(tmp.path().join(&e.file), &good).unwrap();
        let first = ModelStore::new(
            Manifest {
                version: 1,
                entries: vec![e.clone()],
            },
            tmp.path(),
            None,
        );
        first.check_all();
        first.set_self_test(
            "m",
            SelfTestResult {
                passed: true,
                avg_ms: 1.0,
                message: "ok".into(),
                at: 1,
            },
        );
        e.file = "new.onnx".into();
        let path = tmp.path().join(&e.file);
        std::fs::write(&path, vec![2; 100]).unwrap();
        // Stesse dimensioni e data del file precedente: il percorso deve invalidare la cache.
        let original = std::fs::metadata(tmp.path().join("old.onnx"))
            .unwrap()
            .modified()
            .unwrap();
        std::fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(original)
            .unwrap();
        let next = ModelStore::new(
            Manifest {
                version: 1,
                entries: vec![e],
            },
            tmp.path(),
            None,
        );
        assert_eq!(next.check_all(), vec!["m"]);
        assert!(next.verified_path("m").is_none());
        assert!(next.statuses()[0].self_test.is_none());
    }

    #[test]
    fn restoring_verified_bytes_does_not_clear_a_failed_self_test() {
        let tmp = tempfile::tempdir().unwrap();
        let data = vec![1; 100];
        let e = entry("m", "m.onnx", &data, vec![]);
        std::fs::write(tmp.path().join(&e.file), &data).unwrap();
        let s = ModelStore::new(
            Manifest {
                version: 1,
                entries: vec![e],
            },
            tmp.path(),
            None,
        );
        s.check_all();
        s.set_self_test(
            "m",
            SelfTestResult {
                passed: false,
                avg_ms: 0.0,
                message: "fuori soglia".into(),
                at: 1,
            },
        );
        let result = s.restore(&["m".into()], None, &AtomicBool::new(false));
        assert_eq!(result.failed, 0);
        assert!(matches!(
            s.statuses()[0].state,
            ModelState::SelfTestFailed { .. }
        ));
        assert!(s.verified_path("m").is_none());
    }

    #[test]
    fn corrupt_file_does_not_show_a_previous_successful_test() {
        let tmp = tempfile::tempdir().unwrap();
        let data = vec![1; 100];
        let e = entry("m", "m.onnx", &data, vec![]);
        let file = tmp.path().join(&e.file);
        std::fs::write(&file, &data).unwrap();
        let s = ModelStore::new(
            Manifest {
                version: 1,
                entries: vec![e],
            },
            tmp.path(),
            None,
        );
        s.check_all();
        s.set_self_test(
            "m",
            SelfTestResult {
                passed: true,
                avg_ms: 1.0,
                message: "ok".into(),
                at: 1,
            },
        );
        std::fs::write(&file, vec![2; 100]).unwrap();
        std::fs::File::options()
            .write(true)
            .open(&file)
            .unwrap()
            .set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(1))
            .unwrap();
        assert_eq!(s.check_all(), vec!["m"]);
        assert!(s.statuses()[0].self_test.is_none());
        assert!(s.verified_path("m").is_none());
    }
}
