//! Job `raw_review_develop`: sviluppo dalla fotocamera come attività di `pc-jobs` (D6).
//!
//! Un'unica fase (`sviluppo`) con un elemento per conversione. Il lavoro vero lo fa
//! [`runner::run_develop`](super::runner::run_develop) sul thread della fotocamera, che il job
//! tiene in esclusiva per tutta la sua durata. Il riepilogo (convertite, saltate, errori per RAF)
//! si conserva in memoria per il dialogo e arriva anche a `on_finished`, con cui l'app aggiorna la
//! sessione di revisione e avvisa la UI.
//!
//! Gli errori vanno nel registro errori con la fase `sviluppo_raw`.

use super::runner::{self, RunOptions, LOG_PHASE};
use super::service::CameraService;
use super::types::{DevelopReport, DevelopStop, VariantRequest};
use pc_core::{AppError, AppResult};
use pc_jobs::{JobPlan, PhaseSpec};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub const JOB_KIND: &str = "raw_review_develop";
pub use runner::PHASE as PHASE_DEVELOP;

/// Riepiloghi conservati in memoria per il dialogo.
const MAX_REPORTS: usize = 20;

/// Parametri del job (salvati con il job: bastano a ripianificarlo dopo un'interruzione).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevelopParams {
    /// Sessione di revisione da aggiornare alla fine.
    pub session_id: String,
    /// Percorsi dei RAF, dall'indice della sessione (mai dal chiamante).
    pub sources: Vec<String>,
    #[serde(default)]
    pub output_dir: Option<String>,
    pub preset_ids: Vec<String>,
    #[serde(default)]
    pub overwrite: bool,
}

/// Parametri del job per `submit`.
pub fn job_params(params: &DevelopParams) -> serde_json::Value {
    serde_json::to_value(params).unwrap_or(serde_json::Value::Null)
}

impl DevelopParams {
    fn request(&self) -> VariantRequest {
        VariantRequest {
            sources: self.sources.clone(),
            output_dir: self.output_dir.clone(),
            preset_ids: self.preset_ids.clone(),
            overwrite: self.overwrite,
        }
    }

    /// Controlli che non servono né la fotocamera né il disco.
    pub fn validate(&self) -> AppResult<()> {
        if self.sources.is_empty() {
            return Err(AppError::invalid_input("nessuno scatto da sviluppare"));
        }
        if self.preset_ids.is_empty() {
            return Err(AppError::invalid_input("nessun preset selezionato"));
        }
        for source in &self.sources {
            let is_raf = Path::new(source)
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("raf"));
            if !is_raf {
                return Err(AppError::invalid_input(format!(
                    "«{source}» non è un file RAF"
                )));
            }
        }
        Ok(())
    }
}

/// Riepiloghi degli ultimi sviluppi, per job (in memoria: servono al dialogo che li mostra).
#[derive(Clone, Default)]
pub struct DevelopReports(Arc<Mutex<BTreeMap<i64, DevelopReport>>>);

impl DevelopReports {
    pub fn get(&self, job_id: i64) -> Option<DevelopReport> {
        self.0.lock().ok()?.get(&job_id).cloned()
    }

    fn put(&self, report: DevelopReport) {
        let Ok(mut map) = self.0.lock() else { return };
        map.insert(report.job_id, report);
        while map.len() > MAX_REPORTS {
            let Some(first) = map.keys().next().copied() else {
                break;
            };
            map.remove(&first);
        }
    }
}

/// Cosa serve al job.
#[derive(Clone)]
pub struct DevelopDeps {
    /// Il servizio della fotocamera (uno solo, nello stato dell'app).
    pub service: Arc<CameraService>,
    pub reports: DevelopReports,
    /// Chiamata alla fine di ogni sviluppo, qualunque ne sia l'esito (anche annullato o fallito):
    /// l'app aggiorna la sessione per mostrare i nuovi preset e avvisa la UI.
    pub on_finished: Arc<dyn Fn(&DevelopReport) + Send + Sync>,
    pub options: RunOptions,
}

/// Il piano del job a partire dai parametri.
pub fn plan(deps: DevelopDeps, params: &serde_json::Value) -> AppResult<JobPlan> {
    let params: DevelopParams = serde_json::from_value(params.clone())?;
    params.validate()?;
    Ok(JobPlan {
        phases: vec![PhaseSpec::new(PHASE_DEVELOP, 1.0)],
        min_memory_mb: 64,
        run: Box::new(move |ctx| {
            let job_id = ctx.job_id;
            let request = params.request();
            let options = deps.options.clone();
            let session_id = params.session_id.clone();
            let task_session = session_id.clone();
            let control = ctx.control.clone();
            let grace = options.cancel_grace;
            // Con la fotocamera bloccata dentro una chiamata l'annullamento non arriverebbe mai al
            // runner: dopo `cancel_grace` il job smette di aspettarlo (spec variant-generation,
            // "l'annullamento termina entro la conversione in corso").
            let outcome = deps.service.run_exclusive_until(
                move |worker| {
                    let mut report = DevelopReport::new(job_id, &task_session);
                    let result = runner::run_develop(worker, &ctx, &request, options, &mut report);
                    (result, report)
                },
                move || control.is_cancelled(),
                grace,
            );
            // Se la sessione esclusiva non si è nemmeno ottenuta, il riepilogo è vuoto.
            let (result, mut report) = match outcome {
                Ok(done) => done,
                Err(e) => (Err(e), DevelopReport::new(job_id, &session_id)),
            };
            match &result {
                Ok(summary) => report.cancelled = summary.cancelled,
                Err(e) => {
                    tracing::error!(
                        code = e.code.as_str(),
                        job_id,
                        phase = LOG_PHASE,
                        detail = e.detail.as_str(),
                        "{}",
                        e.message_key
                    );
                    report.stopped = Some(DevelopStop {
                        code: e.code.as_str().to_string(),
                        message_key: e.message_key.clone(),
                        reason: e.detail.clone(),
                    });
                }
            }
            deps.reports.put(report.clone());
            (deps.on_finished)(&report);
            result
        }),
    })
}

/// L'avvio è rifiutato se lo spazio libero sul disco di output non basta (con il margine).
pub fn check_space(plan: &super::types::VariantPlan) -> AppResult<()> {
    if plan.enough_space {
        return Ok(());
    }
    let gb = |bytes: u64| bytes as f64 / 1e9;
    Err(AppError::new(
        pc_core::ErrorCode::Io,
        format!(
            "Spazio insufficiente: servono {:.1} GB più {:.0} GB di margine, liberi {:.1} GB.",
            gb(plan.estimated_bytes),
            gb(plan.safety_margin_bytes),
            gb(plan.free_bytes.unwrap_or(0))
        ),
    )
    .with_retriable(false))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::fuji::FujiCamera;
    use crate::camera::ptp::{DataPhase, PtpResult, PtpTransport, Response};
    use crate::camera::sim::{synthetic_raf, SimulatedCamera};
    use crate::variants::diagnostics::{CameraBackend, Opener};
    use crate::variants::plan::{build_plan, PresetDef, DEFAULT_OUTPUT_DIR, MANIFEST};
    use crate::variants::types::CameraState;
    use pc_governor::MemoryGovernor;
    use pc_jobs::manager::{JobManager, JobSnapshot, ProgressSink};
    use pc_jobs::JobState;
    use pc_store::Store;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{Duration, Instant};

    const GB: u64 = 1_000_000_000;
    const JPEG: &[u8] = &[0xFF, 0xD8, 1, 2, 3, 0xFF, 0xD9];

    /// La fotocamera simulata condivisa col test, che può "ricollegarla".
    struct SharedSim(Arc<std::sync::Mutex<SimulatedCamera>>);

    impl PtpTransport for SharedSim {
        fn execute(&mut self, op: u16, params: &[u32], data: DataPhase<'_>) -> PtpResult<Response> {
            self.0.lock().unwrap().execute(op, params, data)
        }
    }

    #[derive(Default)]
    struct Last(std::sync::Mutex<BTreeMap<i64, JobSnapshot>>);
    impl ProgressSink for Last {
        fn on_progress(&self, s: &JobSnapshot) {
            self.0.lock().unwrap().insert(s.id, s.clone());
        }
    }

    struct Env {
        tmp: tempfile::TempDir,
        mgr: JobManager,
        sim: Arc<std::sync::Mutex<SimulatedCamera>>,
        service: Arc<CameraService>,
        reports: DevelopReports,
        finished: Arc<std::sync::Mutex<Vec<DevelopReport>>>,
        free: Arc<AtomicU64>,
        /// Lo spazio libero cala a questo valore dopo `shrink_after` conversioni.
        shrink: Arc<std::sync::Mutex<Option<(usize, u64)>>>,
    }

    fn env() -> Env {
        env_with(1, Duration::from_secs(30))
    }

    fn env_with(max_concurrent: usize, cancel_grace: Duration) -> Env {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::open(&tmp.path().join("catalog.db"), Default::default()).unwrap();
        let sink = Arc::new(Last::default());
        let mgr = JobManager::new(store, MemoryGovernor::new(1024), max_concurrent, sink);
        let sim = Arc::new(std::sync::Mutex::new(SimulatedCamera::new()));
        let opener: Opener = {
            let sim = sim.clone();
            Arc::new(move || Ok(Some(FujiCamera::new(Box::new(SharedSim(sim.clone()))))))
        };
        let service = Arc::new(CameraService::start(
            CameraBackend::Custom(opener),
            Arc::new(|_| {}),
        ));
        let free = Arc::new(AtomicU64::new(1000 * GB));
        let shrink: Arc<std::sync::Mutex<Option<(usize, u64)>>> = Arc::default();
        let free_space: super::super::estimate::FreeSpace = {
            let (free, sim, shrink) = (free.clone(), sim.clone(), shrink.clone());
            Arc::new(move |_| {
                if let Some((after, small)) = *shrink.lock().unwrap() {
                    if sim.lock().unwrap().conversions >= after {
                        return Some(small);
                    }
                }
                Some(free.load(Ordering::SeqCst))
            })
        };
        let reports = DevelopReports::default();
        let finished: Arc<std::sync::Mutex<Vec<DevelopReport>>> = Arc::default();
        let deps = DevelopDeps {
            service: service.clone(),
            reports: reports.clone(),
            on_finished: {
                let finished = finished.clone();
                Arc::new(move |r| finished.lock().unwrap().push(r.clone()))
            },
            options: RunOptions {
                busy_backoff: Duration::from_millis(1),
                disk_check_every: 2,
                free_space,
                pause_poll: Duration::from_millis(20),
                cancel_grace,
                ..RunOptions::default()
            },
        };
        mgr.register(JOB_KIND, Arc::new(move |p| plan(deps.clone(), p)));
        Env {
            tmp,
            mgr,
            sim,
            service,
            reports,
            finished,
            free,
            shrink,
        }
    }

    impl Env {
        fn dir(&self) -> PathBuf {
            self.tmp.path().join("raf")
        }

        fn output(&self) -> PathBuf {
            self.dir().join(DEFAULT_OUTPUT_DIR)
        }

        /// `n` RAF (`foreign` di un altro corpo macchina) in `raf/`, più un JPG accanto.
        fn rafs(&self, n: usize, foreign: &[usize]) -> Vec<String> {
            std::fs::create_dir_all(self.dir()).unwrap();
            std::fs::write(self.dir().join("DSCF0001.JPG"), JPEG).unwrap();
            (0..n)
                .map(|i| {
                    let model = if foreign.contains(&i) {
                        "X100VI"
                    } else {
                        "X-T5"
                    };
                    let path = self.dir().join(format!("DSCF{:04}.RAF", i + 1));
                    // Contenuto diverso per ogni RAF: l'hash dice se qualcuno lo ha toccato.
                    let mut raf = synthetic_raf(model);
                    raf.extend_from_slice(format!("scatto {i}").as_bytes());
                    std::fs::write(&path, raf).unwrap();
                    path.display().to_string()
                })
                .collect()
        }

        fn params(&self, sources: Vec<String>, presets: &[&str], overwrite: bool) -> DevelopParams {
            DevelopParams {
                session_id: "sessione".into(),
                sources,
                output_dir: None,
                preset_ids: presets.iter().map(|p| p.to_string()).collect(),
                overwrite,
            }
        }

        fn start(&self, params: &DevelopParams) -> i64 {
            self.mgr.submit(JOB_KIND, job_params(params)).unwrap()
        }

        fn run(&self, params: &DevelopParams) -> (JobSnapshot, DevelopReport) {
            let id = self.start(params);
            self.finish(id)
        }

        fn finish(&self, id: i64) -> (JobSnapshot, DevelopReport) {
            assert!(
                self.mgr.wait_finished(&[id], Duration::from_secs(60)),
                "il job non è terminato"
            );
            (
                self.mgr.snapshot(id).unwrap(),
                self.reports.get(id).unwrap(),
            )
        }

        /// Attende che il job soddisfi `cond`.
        fn wait_for(&self, id: i64, cond: impl Fn(&JobSnapshot) -> bool) -> JobSnapshot {
            let deadline = Instant::now() + Duration::from_secs(30);
            loop {
                let snap = self.mgr.snapshot(id).unwrap();
                if cond(&snap) {
                    return snap;
                }
                assert!(Instant::now() < deadline, "attesa scaduta: {snap:?}");
                std::thread::sleep(Duration::from_millis(5));
            }
        }
    }

    fn walk(dir: &Path) -> Vec<PathBuf> {
        let mut out = Vec::new();
        let Ok(entries) = std::fs::read_dir(dir) else {
            return out;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                out.extend(walk(&path));
            } else {
                out.push(path);
            }
        }
        out.sort();
        out
    }

    fn jpgs(dir: &Path) -> Vec<PathBuf> {
        walk(dir)
            .into_iter()
            .filter(|p| p.extension().is_some_and(|e| e == "jpg"))
            .collect()
    }

    fn no_part_files(dir: &Path) {
        for p in walk(dir) {
            assert!(
                !p.to_string_lossy().ends_with(".part"),
                "file parziale rimasto: {}",
                p.display()
            );
        }
    }

    /// Dimensione e hash di ogni file della cartella dei RAF (fuori dalle varianti).
    fn fingerprint(dir: &Path) -> Vec<(PathBuf, u64, String)> {
        walk(dir)
            .into_iter()
            .filter(|p| !p.starts_with(dir.join(DEFAULT_OUTPUT_DIR)))
            .map(|p| {
                let bytes = std::fs::read(&p).unwrap();
                (
                    p,
                    bytes.len() as u64,
                    blake3::hash(&bytes).to_hex().to_string(),
                )
            })
            .collect()
    }

    fn sim_presets() -> Vec<PresetDef> {
        let mut camera = FujiCamera::new(SimulatedCamera::new());
        let mut presets = vec![PresetDef::as_shot()];
        presets.extend(
            camera
                .read_banks()
                .unwrap()
                .into_iter()
                .map(PresetDef::bank),
        );
        presets
    }

    #[test]
    fn develops_everything_and_never_touches_the_rafs() {
        let e = env();
        let rafs = e.rafs(3, &[]);
        let before = fingerprint(&e.dir());
        let (snap, report) = e.run(&e.params(rafs, &["asshot", "C1", "C2"], false));
        assert_eq!(snap.state, JobState::Completed, "{snap:?}");
        assert_eq!((report.converted, report.skipped, report.failed), (9, 0, 0));
        assert!(!report.cancelled && report.stopped.is_none());
        assert_eq!(snap.phases[0].done, 9);
        assert_eq!(snap.phases[0].total, 9);
        let out = e.output();
        assert_eq!(jpgs(&out).len(), 9);
        assert!(out.join("Come scattato").join("DSCF0001.jpg").exists());
        assert!(out.join("C1 Sim C1").join("DSCF0003.jpg").exists());
        assert!(out.join("C1 Sim C1").join(MANIFEST).exists());
        no_part_files(&out);
        // RAF e JPG accanto ai RAF: stessi file, stessa dimensione, stesso hash.
        assert_eq!(fingerprint(&e.dir()), before);
        // Un solo caricamento per RAF; sessione chiusa e servizio libero alla fine.
        assert_eq!(e.sim.lock().unwrap().uploads, 3);
        assert!(!e.service.is_running());
        // L'app viene avvisata una volta, con il riepilogo.
        assert_eq!(e.finished.lock().unwrap().len(), 1);
    }

    /// Proporzioni dello scenario "400 su 1.120": 14 su 40 già presenti.
    #[test]
    fn resume_runs_only_the_missing_conversions() {
        let e = env();
        let rafs = e.rafs(10, &[]);
        let ids = ["asshot", "C1", "C2", "C3"];
        // Le prime 14 conversioni (nell'ordine del piano) sono già complete.
        let request = VariantRequest {
            sources: rafs.clone(),
            output_dir: None,
            preset_ids: ids.iter().map(|s| s.to_string()).collect(),
            overwrite: false,
        };
        let plan = build_plan(&request, &sim_presets()).unwrap();
        for target in &plan.targets {
            std::fs::create_dir_all(&target.dir).unwrap();
            std::fs::write(
                target.dir.join(MANIFEST),
                serde_json::to_string(&target.preset.manifest()).unwrap(),
            )
            .unwrap();
        }
        let existing: Vec<PathBuf> = plan.pending[..14]
            .iter()
            .map(|c| plan.output_path(*c))
            .collect();
        for path in &existing {
            std::fs::write(path, JPEG).unwrap();
        }
        let (snap, report) = e.run(&e.params(rafs, &ids, false));
        assert_eq!(snap.state, JobState::Completed, "{snap:?}");
        assert_eq!((report.converted, report.skipped), (26, 14), "{report:?}");
        assert_eq!(jpgs(&e.output()).len(), 40);
        // Le varianti già presenti non sono state rigenerate.
        for path in &existing {
            assert_eq!(std::fs::read(path).unwrap(), JPEG, "{}", path.display());
        }
        // Si caricano solo i RAF che hanno ancora conversioni da fare (7 su 10).
        assert_eq!(e.sim.lock().unwrap().uploads, 7);
        assert_eq!(snap.phases[0].done, 40);
        // Una seconda esecuzione identica non ha più niente da fare.
        let (_, again) = e.run(&e.params(
            plan.rafs.iter().map(|p| p.display().to_string()).collect(),
            &ids,
            false,
        ));
        assert_eq!((again.converted, again.skipped), (0, 40));
    }

    #[test]
    fn cancel_leaves_no_partial_files_and_the_rafs_intact() {
        let e = env();
        let rafs = e.rafs(8, &[]);
        let before = fingerprint(&e.dir());
        e.sim.lock().unwrap().conversion_delay = Duration::from_millis(25);
        let id = e.start(&e.params(rafs, &["asshot", "C1", "C2"], false));
        e.wait_for(id, |s| s.phases[0].done >= 3);
        e.mgr.cancel(id).unwrap();
        let (snap, report) = e.finish(id);
        assert_eq!(snap.state, JobState::Cancelled, "{snap:?}");
        assert!(report.cancelled);
        let count = jpgs(&e.output()).len();
        assert!((3..24).contains(&count), "{count}");
        assert_eq!(count as u32, report.converted);
        no_part_files(&e.output());
        for jpg in jpgs(&e.output()) {
            assert!(
                crate::variants::plan::variant_complete(&jpg),
                "{}",
                jpg.display()
            );
        }
        assert_eq!(fingerprint(&e.dir()), before);
        // Sessione chiusa: il servizio è di nuovo libero e si può rilanciare.
        assert!(!e.service.is_running());
        let (again, rerun) = e.run(
            &e.params(
                (0..8)
                    .map(|i| {
                        e.dir()
                            .join(format!("DSCF{:04}.RAF", i + 1))
                            .display()
                            .to_string()
                    })
                    .collect(),
                &["asshot", "C1", "C2"],
                false,
            ),
        );
        assert_eq!(again.state, JobState::Completed);
        assert_eq!(rerun.converted as usize + rerun.skipped as usize, 24);
        assert_eq!(rerun.skipped as usize, count);
    }

    #[test]
    fn a_raf_from_another_body_is_skipped_with_the_reason() {
        let e = env();
        let rafs = e.rafs(3, &[1]);
        let (snap, report) = e.run(&e.params(rafs, &["asshot", "C1", "C2"], false));
        assert_eq!(snap.state, JobState::CompletedWithErrors, "{snap:?}");
        assert_eq!((report.converted, report.failed), (6, 3));
        assert_eq!(snap.failed, 3);
        assert_eq!(report.failures.len(), 1);
        let failure = &report.failures[0];
        assert_eq!(failure.raf, "DSCF0002.RAF");
        assert_eq!(failure.presets.len(), 3);
        assert_eq!(failure.code, "RAW_CAMERA_REJECTED");
        assert_eq!(failure.message_key, "errors.raw_camera_rejected");
        assert!(failure.reason.contains("non scattato da questa fotocamera"));
        assert_eq!(jpgs(&e.output()).len(), 6);
        assert!(!e
            .output()
            .join("Come scattato")
            .join("DSCF0002.jpg")
            .exists());
    }

    #[test]
    fn low_disk_space_pauses_at_start_and_resumes_when_freed() {
        let e = env();
        let rafs = e.rafs(2, &[]);
        e.free.store(GB, Ordering::SeqCst);
        let id = e.start(&e.params(rafs, &["asshot", "C1"], false));
        let paused = e.wait_for(id, |s| s.state == JobState::Paused);
        let reason = paused.pause_reason.expect("motivo della pausa");
        assert_eq!(reason.code, "disk_space");
        assert!(reason.detail.contains("Spazio su disco insufficiente"));
        assert!(jpgs(&e.output()).is_empty());
        e.free.store(1000 * GB, Ordering::SeqCst);
        e.mgr.resume(id).unwrap();
        let (snap, report) = e.finish(id);
        assert_eq!(snap.state, JobState::Completed, "{snap:?}");
        assert!(snap.pause_reason.is_none());
        assert_eq!(report.converted, 4);
    }

    #[test]
    fn space_dropping_below_the_margin_pauses_mid_run() {
        let e = env();
        let rafs = e.rafs(4, &[]);
        *e.shrink.lock().unwrap() = Some((4, GB));
        let id = e.start(&e.params(rafs, &["asshot", "C1"], false));
        let paused = e.wait_for(id, |s| s.state == JobState::Paused);
        assert_eq!(paused.pause_reason.unwrap().code, "disk_space");
        let done = jpgs(&e.output()).len();
        assert!((4..8).contains(&done), "{done}");
        no_part_files(&e.output());
        *e.shrink.lock().unwrap() = None;
        e.mgr.resume(id).unwrap();
        let (snap, report) = e.finish(id);
        assert_eq!(snap.state, JobState::Completed);
        assert_eq!(report.converted, 8);
        assert_eq!(jpgs(&e.output()).len(), 8);
    }

    #[test]
    fn disconnection_pauses_with_a_reason_and_resume_completes() {
        let e = env();
        let rafs = e.rafs(3, &[]);
        let before = fingerprint(&e.dir());
        e.sim.lock().unwrap().disconnect_after = Some(4);
        let id = e.start(&e.params(rafs, &["asshot", "C1", "C2"], false));
        let paused = e.wait_for(id, |s| s.state == JobState::Paused);
        let reason = paused.pause_reason.clone().expect("motivo della pausa");
        assert_eq!(reason.code, "camera_disconnected");
        assert!(reason.detail.contains("scollegata"), "{}", reason.detail);
        // La quarta conversione era pronta ma non scaricata: nessun file parziale.
        assert_eq!(jpgs(&e.output()).len(), 3);
        no_part_files(&e.output());
        // Il servizio dice che la fotocamera non c'è, senza bloccarsi sul job in corso.
        assert_eq!(e.service.status().state, CameraState::Disconnected);
        e.sim.lock().unwrap().reconnect();
        e.mgr.resume(id).unwrap();
        let (snap, report) = e.finish(id);
        assert_eq!(snap.state, JobState::Completed, "{snap:?}");
        assert!(snap.pause_reason.is_none());
        assert_eq!((report.converted, report.failed), (9, 0));
        assert_eq!(jpgs(&e.output()).len(), 9);
        no_part_files(&e.output());
        assert_eq!(fingerprint(&e.dir()), before);
    }

    #[test]
    fn camera_busy_at_start_pauses_and_a_resume_with_the_camera_free_completes() {
        let e = env();
        let rafs = e.rafs(1, &[]);
        // Nessuna fotocamera: nessuna sessione, e il job aspetta invece di fallire.
        e.sim.lock().unwrap().disconnect_after = Some(0);
        let id = e.start(&e.params(rafs, &["asshot"], false));
        let paused = e.wait_for(id, |s| s.state == JobState::Paused);
        assert_eq!(paused.pause_reason.unwrap().code, "camera_disconnected");
        // Annullare da in pausa termina il job senza scrivere niente.
        e.mgr.cancel(id).unwrap();
        let (snap, report) = e.finish(id);
        assert_eq!(snap.state, JobState::Cancelled);
        assert!(report.cancelled);
        assert!(jpgs(&e.output()).is_empty());
    }

    #[test]
    fn overwrite_regenerates_existing_variants() {
        let e = env();
        let rafs = e.rafs(2, &[]);
        let (first, _) = e.run(&e.params(rafs.clone(), &["asshot"], false));
        assert_eq!(first.state, JobState::Completed);
        let target = e.output().join("Come scattato").join("DSCF0001.jpg");
        let generated = std::fs::read(&target).unwrap();
        // Una variante completa ma diversa da quella della fotocamera: senza "Sovrascrivi" resta.
        std::fs::write(&target, JPEG).unwrap();
        let (_, kept) = e.run(&e.params(rafs.clone(), &["asshot"], false));
        assert_eq!((kept.converted, kept.skipped), (0, 2));
        assert_eq!(std::fs::read(&target).unwrap(), JPEG);
        // Con "Sovrascrivi" tutto viene rigenerato.
        let (snap, regenerated) = e.run(&e.params(rafs, &["asshot"], true));
        assert_eq!(snap.state, JobState::Completed);
        assert_eq!((regenerated.converted, regenerated.skipped), (2, 0));
        assert_eq!(std::fs::read(&target).unwrap(), generated);
        no_part_files(&e.output());
    }

    #[test]
    fn a_session_left_open_is_closed_and_reopened() {
        let e = env();
        let rafs = e.rafs(1, &[]);
        // Il servizio ha già una sessione aperta (stato richiesto dalla UI).
        assert_eq!(e.service.status().state, CameraState::Ready);
        let (snap, report) = e.run(&e.params(rafs, &["asshot"], false));
        assert_eq!(snap.state, JobState::Completed, "{snap:?}");
        assert_eq!(report.converted, 1);
    }

    #[test]
    fn only_one_develop_at_a_time() {
        let e = env_with(2, Duration::from_secs(30));
        let rafs = e.rafs(6, &[]);
        e.sim.lock().unwrap().conversion_delay = Duration::from_millis(30);
        let id = e.start(&e.params(rafs.clone(), &["asshot", "C1"], false));
        e.wait_for(id, |s| s.phases[0].done >= 1);
        let second = e.start(&e.params(rafs, &["asshot"], false));
        // Il secondo job non ottiene la sessione esclusiva e fallisce con un motivo chiaro.
        let (snap, report) = e.finish(second);
        assert_eq!(snap.state, JobState::Failed, "{snap:?}");
        assert!(report.stopped.unwrap().reason.contains("già in corso"));
        e.mgr.cancel(id).unwrap();
        e.finish(id);
    }

    /// Journal di prova: ricorda le chiavi concluse in una esecuzione precedente e registra le nuove.
    #[derive(Default)]
    struct TestJournal {
        finished: std::sync::Mutex<Vec<String>>,
        recorded: std::sync::Mutex<Vec<(String, String, pc_jobs::ItemState)>>,
    }

    impl pc_jobs::Journal for TestJournal {
        fn is_finished(&self, key: &str) -> bool {
            self.finished.lock().unwrap().iter().any(|k| k == key)
        }
        fn record(&self, key: &str, phase: &str, state: pc_jobs::ItemState) {
            self.recorded
                .lock()
                .unwrap()
                .push((key.into(), phase.into(), state));
        }
    }

    #[test]
    fn the_journal_key_is_raf_and_preset_folder_and_a_resumed_job_skips_finished_items() {
        let e = env();
        let rafs = e.rafs(3, &[]);
        let journal = Arc::new(TestJournal::default());
        // Una esecuzione precedente aveva concluso queste due conversioni.
        journal.finished.lock().unwrap().extend([
            "DSCF0001.RAF|C1 Sim C1".to_string(),
            "DSCF0002.RAF|Come scattato".to_string(),
        ]);
        let params = e.params(rafs, &["asshot", "C1"], true);
        let tmp = tempfile::tempdir().unwrap();
        let ctx = pc_jobs::JobContext {
            job_id: 7,
            params: job_params(&params),
            progress: pc_jobs::JobProgress::new(&[pc_jobs::PhaseSpec::new(PHASE_DEVELOP, 1.0)]),
            control: pc_jobs::JobControl::new(),
            journal: journal.clone(),
            store: Store::open(&tmp.path().join("c.db"), Default::default()).unwrap(),
            governor: MemoryGovernor::new(1024),
            followup: Arc::default(),
        };
        let progress = ctx.progress.clone();
        let request = params.request();
        let options = RunOptions {
            busy_backoff: Duration::from_millis(1),
            ..RunOptions::default()
        };
        let (summary, report) = e
            .service
            .run_exclusive(move |worker| {
                let mut report = DevelopReport::new(7, "sessione");
                let summary = runner::run_develop(worker, &ctx, &request, options, &mut report);
                (summary, report)
            })
            .unwrap();
        assert!(!summary.unwrap().cancelled);
        // "Sovrascrivi" rigenera tutto, ma le due voci del journal non si rifanno.
        assert_eq!((report.converted, report.skipped), (4, 2), "{report:?}");
        assert!(!e.output().join("C1 Sim C1").join("DSCF0001.jpg").exists());
        assert!(e.output().join("C1 Sim C1").join("DSCF0002.jpg").exists());
        let recorded = journal.recorded.lock().unwrap();
        assert_eq!(recorded.len(), 4);
        assert!(recorded
            .iter()
            .all(|(_, phase, state)| phase == PHASE_DEVELOP && *state == pc_jobs::ItemState::Done));
        assert!(recorded.iter().any(|(k, ..)| k == "DSCF0003.RAF|C1 Sim C1"));
        let phase = progress.phase(PHASE_DEVELOP).done();
        assert_eq!(phase, 6, "le due già concluse contano come fatte");
    }

    #[test]
    fn params_are_validated_before_anything_runs() {
        let e = env();
        let rafs = e.rafs(1, &[]);
        let bad = |params: DevelopParams| e.mgr.submit(JOB_KIND, job_params(&params)).is_err();
        assert!(bad(e.params(vec![], &["asshot"], false)));
        assert!(bad(e.params(rafs.clone(), &[], false)));
        assert!(bad(e.params(
            vec![e.dir().join("DSCF0001.JPG").display().to_string()],
            &["asshot"],
            false
        )));
        assert!(!bad(e.params(rafs, &["asshot"], false)));
    }

    #[test]
    fn insufficient_space_is_refused_with_the_numbers() {
        let e = env();
        let rafs = e.rafs(2, &[]);
        let request = VariantRequest {
            sources: rafs,
            output_dir: None,
            preset_ids: vec!["asshot".into(), "C1".into()],
            overwrite: false,
        };
        let plan = build_plan(&request, &sim_presets()).unwrap();
        let small: super::super::estimate::FreeSpace = Arc::new(|_| Some(GB));
        let summary = plan.summary_with(&small);
        assert!(!summary.enough_space);
        let error = check_space(&summary).unwrap_err();
        assert!(
            error.detail.contains("Spazio insufficiente"),
            "{}",
            error.detail
        );
        assert!(error.detail.contains("1.0 GB"), "{}", error.detail);
        let big: super::super::estimate::FreeSpace = Arc::new(|_| Some(100 * GB));
        assert!(check_space(&plan.summary_with(&big)).is_ok());
    }

    /// Il job in pausa tiene la sessione esclusiva: è lui a ricontrollare la fotocamera, così lo
    /// stato mostrato (e "Riprendi" nel dialogo) segue il ricollegamento.
    #[test]
    fn while_paused_the_camera_status_follows_the_reconnection() {
        let e = env();
        let rafs = e.rafs(2, &[]);
        e.sim.lock().unwrap().disconnect_after = Some(1);
        let id = e.start(&e.params(rafs, &["asshot", "C1"], false));
        let paused = e.wait_for(id, |s| s.state == JobState::Paused);
        assert_eq!(paused.pause_reason.unwrap().code, "camera_disconnected");
        assert_eq!(e.service.status().state, CameraState::Disconnected);
        assert!(e.service.is_running(), "la sessione esclusiva resta al job");
        e.sim.lock().unwrap().reconnect();
        let deadline = Instant::now() + Duration::from_secs(10);
        while e.service.status().state != CameraState::Ready {
            assert!(Instant::now() < deadline, "lo stato non è tornato pronto");
            std::thread::sleep(Duration::from_millis(10));
        }
        // Ancora in pausa: si riprende solo con "Riprendi".
        assert_eq!(e.mgr.snapshot(id).unwrap().state, JobState::Paused);
        e.mgr.resume(id).unwrap();
        let (snap, report) = e.finish(id);
        assert_eq!(snap.state, JobState::Completed, "{snap:?}");
        assert_eq!(report.converted, 4);
        no_part_files(&e.output());
    }

    #[test]
    fn an_output_folder_that_is_the_raf_folder_is_refused_without_writing() {
        let e = env();
        let rafs = e.rafs(2, &[]);
        let before = walk(&e.dir());
        let mut params = e.params(rafs, &["asshot"], false);
        params.output_dir = Some(e.dir().display().to_string());
        let (snap, report) = e.run(&params);
        assert_eq!(snap.state, JobState::Failed, "{snap:?}");
        let stopped = report.stopped.expect("motivo");
        assert_eq!(stopped.code, "OUTPUT_FOLDER_REJECTED");
        assert_eq!(stopped.message_key, "errors.output_folder_rejected");
        assert!(
            stopped.reason.contains("cartella dei RAF"),
            "{}",
            stopped.reason
        );
        // Nessuna cartella di preset, nessun manifesto, nessun JPEG: la cartella è com'era.
        assert_eq!(walk(&e.dir()), before);
        assert!(!e.output().exists());
        assert_eq!(e.sim.lock().unwrap().uploads, 0);
        assert!(!e.service.is_running());
    }

    /// Fotocamera bloccata dentro una chiamata: l'annullamento non può raggiungere il runner, e
    /// dopo il tempo di grazia il job termina comunque con l'invito a spegnere e riaccendere.
    #[test]
    fn cancelling_with_a_frozen_camera_ends_the_job_after_the_grace_period() {
        let e = env_with(1, Duration::from_millis(150));
        let rafs = e.rafs(2, &[]);
        e.sim.lock().unwrap().conversion_delay = Duration::from_millis(2500);
        let id = e.start(&e.params(rafs, &["asshot"], false));
        // Si attende che la prima conversione sia partita (il simulatore è dentro la chiamata).
        e.wait_for(id, |s| {
            s.phases[0]
                .current
                .as_deref()
                .is_some_and(|c| c.contains('·'))
        });
        std::thread::sleep(Duration::from_millis(50));
        let cancelled_at = Instant::now();
        e.mgr.cancel(id).unwrap();
        assert!(e.mgr.wait_finished(&[id], Duration::from_secs(2)));
        assert!(cancelled_at.elapsed() < Duration::from_millis(2000));
        let snap = e.mgr.snapshot(id).unwrap();
        assert_eq!(snap.state, JobState::Failed, "{snap:?}");
        let report = e.reports.get(id).unwrap();
        let stopped = report.stopped.expect("motivo");
        assert_eq!(stopped.code, "CAMERA_UNRESPONSIVE");
        assert!(stopped.reason.contains("spegnila e riaccendila"));
        assert!(!e.service.is_running());
        // Quando la chiamata ritorna il runner si accorge dell'annullamento: nessun file parziale.
        std::thread::sleep(Duration::from_millis(2800));
        no_part_files(&e.output());
        assert!(jpgs(&e.output()).len() <= 1);
    }
}
