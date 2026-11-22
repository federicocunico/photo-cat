//! Esecuzione di un [`Plan`] con la fotocamera, dentro un job di `pc-jobs` (porting di
//! `runner.rs` di FujiView, convertito secondo D6).
//!
//! Regole di sicurezza:
//! - i RAF si **leggono soltanto**: non vengono mai scritti, spostati né cancellati;
//! - non si scrive nulla fuori dalle cartelle dei preset del piano (`OutputGuard`);
//! - ogni JPEG arriva su disco come `<nome>.jpg.part` poi rinominato: col nome finale non esiste
//!   mai un file parziale.
//!
//! Rispetto a FujiView il runner non ha stato né eventi propri. Il job ha una fase con un
//! elemento per conversione (chiave del journal `<raf>|<cartella del preset>`); la pausa per
//! fotocamera scollegata, occupata o disco quasi pieno usa la pausa dei job con un motivo
//! (`JobControl::pause_for`), e dopo "Riprendi" il lavoro continua dalla conversione interrotta.

use super::diagnostics::Cam;
use super::estimate::{self, FreeSpace, Meter};
use super::plan::{Conversion, Plan, PresetKind, MANIFEST, PART_SUFFIX};
use super::service::{status_error, Worker};
use super::types::{CameraState, DevelopFailure, DevelopReport, VariantRequest};
use crate::camera::banks::{apply_bank, as_shot};
use crate::camera::fuji::is_raf_rejected;
use crate::camera::profile::Profile;
use crate::camera::ptp::{rc, PtpError};
use pc_core::{AppError, AppResult, ErrorCode};
use pc_jobs::{ItemState, JobContext, PauseReason, PhaseCounters, RunSummary};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Fase del job (una voce per conversione).
pub const PHASE: &str = "sviluppo";
/// Fase delle voci nel registro errori.
pub const LOG_PHASE: &str = "sviluppo_raw";

/// Codici dei motivi di pausa (`jobs.pauseReasons.<codice>` nella UI).
pub mod pause {
    pub const CAMERA_DISCONNECTED: &str = "camera_disconnected";
    pub const CAMERA_BUSY: &str = "camera_busy";
    pub const CAMERA_NOT_READY: &str = "camera_not_ready";
    pub const DISK_SPACE: &str = "disk_space";
    pub const OUTPUT_WRITE: &str = "output_write";
}

#[derive(Clone)]
pub struct RunOptions {
    /// Ricarica il RAF prima di ogni conversione (ripiego quando la fotocamera accetta una sola
    /// conversione per caricamento). Si attiva da solo quando lo si rileva.
    pub upload_per_conversion: bool,
    pub busy_retries: usize,
    pub busy_backoff: Duration,
    pub disk_check_every: usize,
    /// Come si legge lo spazio libero (sostituibile nei test).
    pub free_space: FreeSpace,
}

impl Default for RunOptions {
    fn default() -> Self {
        Self {
            upload_per_conversion: false,
            busy_retries: 3,
            busy_backoff: Duration::from_millis(750),
            disk_check_every: estimate::DISK_CHECK_EVERY,
            free_space: Arc::new(|path: &Path| estimate::free_bytes(path)),
        }
    }
}

/// Radice di output canonica più i controlli che ogni scrittura resti dentro.
pub struct OutputGuard {
    root: PathBuf,
}

impl OutputGuard {
    pub fn new(root: &Path) -> Result<Self, String> {
        fs::create_dir_all(root)
            .map_err(|e| format!("Impossibile creare {}: {e}", root.display()))?;
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        Ok(Self { root })
    }

    /// Fallisce se la cartella che contiene `path` non sta dentro la radice (e non è la radice
    /// stessa: i file vanno nelle cartelle dei preset).
    pub fn check(&self, path: &Path) -> Result<(), String> {
        let name_ok = path.file_name().is_some_and(|n| n != ".." && n != ".");
        let parent = path.parent().ok_or("percorso senza cartella")?;
        let parent = parent
            .canonicalize()
            .map_err(|e| format!("{}: {e}", parent.display()))?;
        if name_ok && parent.starts_with(&self.root) && parent != self.root {
            Ok(())
        } else {
            Err(format!(
                "Scrittura rifiutata fuori dalla cartella di output: {}",
                path.display()
            ))
        }
    }
}

fn prepare_targets(plan: &Plan, guard: &OutputGuard) -> Result<(), String> {
    for target in &plan.targets {
        fs::create_dir_all(&target.dir)
            .map_err(|e| format!("Impossibile creare {}: {e}", target.dir.display()))?;
        let manifest = target.dir.join(MANIFEST);
        guard.check(&manifest)?;
        if !manifest.exists() {
            let text = serde_json::to_string_pretty(&target.preset.manifest())
                .map_err(|e| e.to_string())?;
            fs::write(&manifest, text).map_err(|e| e.to_string())?;
        }
        // Residui di un'esecuzione interrotta: solo i `*.part`, solo direttamente dentro le
        // nostre cartelle dei preset.
        for entry in fs::read_dir(&target.dir)
            .map_err(|e| e.to_string())?
            .flatten()
        {
            let path = entry.path();
            if path.is_file()
                && path.to_string_lossy().ends_with(PART_SUFFIX)
                && guard.check(&path).is_ok()
            {
                let _ = fs::remove_file(&path);
            }
        }
    }
    Ok(())
}

/// Scrive `bytes` in `path` passando da un `.part`, sync e rinomina.
pub fn write_atomic(guard: &OutputGuard, path: &Path, bytes: &[u8]) -> Result<(), String> {
    guard.check(path)?;
    let part = PathBuf::from(format!("{}{PART_SUFFIX}", path.display()));
    let result = (|| -> std::io::Result<()> {
        let mut file = fs::File::create(&part)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&part, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&part);
    }
    result.map_err(|e| format!("Scrittura di {} non riuscita: {e}", path.display()))
}

fn spawn_read(path: PathBuf) -> JoinHandle<std::io::Result<Vec<u8>>> {
    // Sola lettura: il RAF non si apre mai in scrittura.
    std::thread::spawn(move || fs::read(path))
}

/// Perché un passo non può proseguire.
enum Stop {
    /// Si mette in pausa con un motivo e si riprova dopo "Riprendi".
    Pause(PauseReason),
    /// Ferma il job con questo errore.
    Fail(AppError),
}

fn pause_reason(code: &str, detail: impl Into<String>) -> Stop {
    Stop::Pause(PauseReason {
        code: code.to_string(),
        detail: detail.into(),
    })
}

/// Errori della fotocamera che fermano il lavoro (pausa o errore); `None` = solo questa
/// conversione è fallita.
fn classify(error: &PtpError) -> Option<Stop> {
    match error {
        PtpError::Disconnected => Some(pause_reason(
            pause::CAMERA_DISCONNECTED,
            "Fotocamera scollegata: ricollega il cavo USB, accendi la fotocamera e premi Riprendi.",
        )),
        PtpError::Transport { .. } => Some(pause_reason(
            pause::CAMERA_DISCONNECTED,
            format!(
                "Comunicazione con la fotocamera interrotta ({error}). Ricollegala e premi Riprendi."
            ),
        )),
        PtpError::Busy => Some(pause_reason(
            pause::CAMERA_BUSY,
            "La fotocamera è usata da un altro programma: chiudi X RAW Studio e premi Riprendi.",
        )),
        PtpError::Timeout(_) => Some(Stop::Fail(AppError::new(
            ErrorCode::CameraUnresponsive,
            "La fotocamera non risponde: spegnila e riaccendila, poi riavvia lo sviluppo (le varianti già pronte si saltano).",
        ))),
        PtpError::Response {
            code: rc::DEVICE_BUSY,
            ..
        } => Some(Stop::Fail(AppError::new(
            ErrorCode::CameraUnresponsive,
            "La fotocamera resta occupata (0x2019): spegnila e riaccendila, poi riavvia lo sviluppo (le varianti già pronte si saltano).",
        ))),
        _ => None,
    }
}

fn stop_of(error: &PtpError) -> Stop {
    classify(error)
        .unwrap_or_else(|| pause_reason(pause::CAMERA_DISCONNECTED, error.user_message()))
}

fn raf_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Chiave del journal di una conversione: `<raf>|<cartella del preset>`.
pub fn journal_key(plan: &Plan, conversion: Conversion) -> String {
    format!(
        "{}|{}",
        raf_name(&plan.rafs[conversion.raf]),
        plan.targets[conversion.preset].folder_name
    )
}

/// Ripete l'azione se la fotocamera risponde "occupata" (0x2019), con attese crescenti.
fn retry_busy<R>(
    worker: &mut Worker,
    options: &RunOptions,
    mut action: impl FnMut(&mut Cam) -> Result<R, PtpError>,
) -> Result<R, PtpError> {
    let mut attempt = 0;
    loop {
        let camera = worker.camera().ok_or(PtpError::Disconnected)?;
        match action(camera) {
            Err(PtpError::Response {
                code: rc::DEVICE_BUSY,
                ..
            }) if attempt < options.busy_retries => {
                attempt += 1;
                std::thread::sleep(options.busy_backoff * attempt as u32);
            }
            other => return other,
        }
    }
}

/// Mette il job in pausa con un motivo e attende "Riprendi". `false` se è stato annullato.
fn pause_and_wait(ctx: &JobContext, reason: PauseReason) -> bool {
    tracing::warn!(
        code = reason.code.as_str(),
        job_id = ctx.job_id,
        phase = LOG_PHASE,
        detail = reason.detail.as_str(),
        "sviluppo in pausa"
    );
    ctx.control.pause_for(reason);
    ctx.control.checkpoint()
}

/// Lo stato della fotocamera mostrato dall'app segue il motivo della pausa (mentre il job ha la
/// sessione esclusiva lo stato non si rilegge dalla fotocamera).
fn publish_pause(worker: &Worker, reason: &PauseReason) {
    let state = match reason.code.as_str() {
        pause::CAMERA_DISCONNECTED => CameraState::Disconnected,
        pause::CAMERA_BUSY => CameraState::Busy,
        pause::CAMERA_NOT_READY => CameraState::NotReady,
        _ => return,
    };
    worker.set_state(state, Some(reason.detail.clone()));
}

/// Apre una sessione nuova (quella eventualmente rimasta aperta si chiude prima) e controlla che
/// la fotocamera sia pronta; ripulisce gli oggetti rimasti sulla fotocamera da una conversione
/// precedente.
fn open_session(worker: &mut Worker, options: &RunOptions) -> Result<(), Stop> {
    worker.close_session();
    let status = worker.refresh();
    match status.state {
        CameraState::Ready => {}
        CameraState::Disconnected => {
            return Err(pause_reason(
                pause::CAMERA_DISCONNECTED,
                "Nessuna fotocamera collegata: collega il cavo USB, accendila e premi Riprendi.",
            ))
        }
        CameraState::Busy => return Err(pause_reason(
            pause::CAMERA_BUSY,
            "La fotocamera è usata da un altro programma: chiudi X RAW Studio e premi Riprendi.",
        )),
        CameraState::NotReady => {
            return Err(pause_reason(
                pause::CAMERA_NOT_READY,
                status
                    .message
                    .unwrap_or_else(|| "La fotocamera non è pronta per la conversione.".into()),
            ))
        }
        CameraState::Unresponsive | CameraState::Error => {
            return Err(Stop::Fail(status_error(&status)))
        }
    }
    retry_busy(worker, options, |camera| camera.clear_objects())
        .map(|_| ())
        .map_err(|e| stop_of(&e))
}

struct Runner<'a> {
    worker: &'a mut Worker,
    ctx: &'a JobContext,
    plan: &'a Plan,
    guard: OutputGuard,
    options: RunOptions,
    phase: Arc<PhaseCounters>,
    meter: Meter,
    report: &'a mut DevelopReport,
    /// Conversioni ancora da fare (per stimare lo spazio che serve).
    remaining: usize,
    since_disk_check: usize,
}

impl Runner<'_> {
    fn cancelled(&self) -> bool {
        self.ctx.control.is_cancelled()
    }

    fn current(&self, raf: &str, folder: Option<&str>) {
        self.phase.set_current(Some(match folder {
            Some(folder) => format!("{raf} · {folder}"),
            None => raf.to_string(),
        }));
    }

    /// Un RAF (o una sua conversione) non riuscito: nel riepilogo, nel registro errori e nei
    /// contatori. Non ferma il job.
    fn fail(&mut self, raf: usize, presets: &[usize], error: &AppError) {
        let name = raf_name(&self.plan.rafs[raf]);
        tracing::error!(
            code = error.code.as_str(),
            job_id = self.ctx.job_id,
            phase = LOG_PHASE,
            path = %self.plan.rafs[raf].display(),
            detail = error.detail.as_str(),
            "{}",
            error.message_key
        );
        for &p in presets {
            let key = journal_key(self.plan, Conversion { raf, preset: p });
            self.ctx.journal.record(&key, PHASE, ItemState::Failed);
        }
        self.phase.inc_failed(presets.len() as i64);
        self.report.failed += presets.len() as u32;
        self.remaining = self.remaining.saturating_sub(presets.len());
        self.report.failures.push(DevelopFailure {
            raf: name,
            presets: presets
                .iter()
                .map(|&p| self.plan.targets[p].folder_name.clone())
                .collect(),
            code: error.code.as_str().to_string(),
            message_key: error.message_key.clone(),
            reason: error.detail.clone(),
        });
    }

    fn done(&mut self, raf: usize, preset: usize) {
        let key = journal_key(self.plan, Conversion { raf, preset });
        self.ctx.journal.record(&key, PHASE, ItemState::Done);
        self.phase.inc_done(1);
        self.report.converted += 1;
        self.remaining = self.remaining.saturating_sub(1);
        self.since_disk_check += 1;
    }

    fn upload(&mut self, raf: &[u8]) -> Result<Profile, PtpError> {
        retry_busy(self.worker, &self.options, |camera| camera.upload_raf(raf))?;
        retry_busy(self.worker, &self.options, |camera| camera.read_profile())
    }

    fn profile_for(&self, base: &Profile, preset: usize) -> Result<Profile, PtpError> {
        match &self.plan.targets[preset].preset.kind {
            PresetKind::AsShot => as_shot(base),
            PresetKind::Bank(bank) => Ok(apply_bank(base, bank)?.profile),
        }
    }

    /// Controlla lo spazio; sotto il margine il job si mette in pausa finché non si libera.
    /// `false` se nel frattempo è stato annullato.
    fn ensure_room(&mut self) -> bool {
        loop {
            let Some(free) = (self.options.free_space)(&self.plan.output_dir) else {
                return true;
            };
            let need = estimate::required_bytes(self.remaining, self.meter.average_bytes());
            if estimate::has_room(free, need) {
                return true;
            }
            let gb = |bytes: u64| bytes as f64 / 1e9;
            let detail = format!(
                "Spazio su disco insufficiente: servono {:.1} GB più {:.0} GB di margine, liberi {:.1} GB. Libera spazio e premi Riprendi.",
                gb(need),
                gb(estimate::SAFETY_MARGIN_BYTES),
                gb(free)
            );
            if !pause_and_wait(
                self.ctx,
                PauseReason {
                    code: pause::DISK_SPACE.into(),
                    detail,
                },
            ) {
                return false;
            }
        }
    }

    /// Converte i preset indicati di un RAF. I preset conclusi (riusciti o falliti) escono da
    /// `presets`; con `Err` quelli rimasti si riprovano dopo la pausa.
    fn convert_raf(
        &mut self,
        raf: usize,
        presets: &mut Vec<usize>,
        bytes: &[u8],
    ) -> Result<(), Stop> {
        let name = raf_name(&self.plan.rafs[raf]);
        self.current(&name, None);
        let mut base = match self.upload(bytes) {
            Ok(base) => base,
            Err(error) if is_raf_rejected(&error) => {
                let rejected = AppError::new(
                    ErrorCode::RawCameraRejected,
                    "RAF non scattato da questa fotocamera o non convertibile",
                );
                let all = std::mem::take(presets);
                self.fail(raf, &all, &rejected);
                return Ok(());
            }
            Err(error) => return Err(stop_of(&error)),
        };
        let mut fresh_upload = true;
        while let Some(&preset) = presets.first() {
            if !self.ctx.control.checkpoint() || self.cancelled() {
                return Ok(());
            }
            let folder = self.plan.targets[preset].folder_name.clone();
            self.current(&name, Some(&folder));
            if self.options.upload_per_conversion && !fresh_upload {
                base = self.upload(bytes).map_err(|e| stop_of(&e))?;
            }
            let started = Instant::now();
            let mut outcome = self.profile_for(&base, preset).and_then(|profile| {
                retry_busy(self.worker, &self.options, |c| c.convert(&profile))
            });
            if let Err(PtpError::Response {
                code: rc::GENERAL_ERROR,
                ..
            }) = outcome
            {
                if !fresh_upload && !self.options.upload_per_conversion {
                    // La fotocamera rifiuta una seconda conversione per caricamento: da qui in poi
                    // si ricarica il RAF a ogni conversione.
                    self.options.upload_per_conversion = true;
                    outcome = self
                        .upload(bytes)
                        .and_then(|fresh| {
                            base = fresh;
                            self.profile_for(&base, preset)
                        })
                        .and_then(|profile| {
                            retry_busy(self.worker, &self.options, |c| c.convert(&profile))
                        });
                }
            }
            fresh_upload = false;
            match outcome {
                Ok(jpeg) => {
                    let path = self.plan.output_path(Conversion { raf, preset });
                    write_atomic(&self.guard, &path, &jpeg)
                        .map_err(|e| pause_reason(pause::OUTPUT_WRITE, e))?;
                    self.meter
                        .record(started.elapsed().as_secs_f64(), jpeg.len() as u64);
                    presets.remove(0);
                    self.done(raf, preset);
                }
                Err(error) => match classify(&error) {
                    Some(stop) => return Err(stop),
                    None => {
                        presets.remove(0);
                        self.fail(raf, &[preset], &AppError::from(error));
                    }
                },
            }
            if self.since_disk_check >= self.options.disk_check_every {
                self.since_disk_check = 0;
                if !self.ensure_room() {
                    return Ok(());
                }
            }
        }
        self.current(&name, None);
        Ok(())
    }

    /// Un RAF con i suoi preset: dopo una pausa riapre la sessione e riprende da dove era.
    fn convert_with_pauses(
        &mut self,
        raf: usize,
        presets: &mut Vec<usize>,
        bytes: &[u8],
    ) -> Result<(), AppError> {
        loop {
            match self.convert_raf(raf, presets, bytes) {
                Ok(()) => return Ok(()),
                Err(Stop::Fail(error)) => return Err(error),
                Err(Stop::Pause(reason)) => {
                    // La connessione non è più affidabile: si chiude e si riapre dopo la pausa.
                    self.worker.close_session();
                    publish_pause(self.worker, &reason);
                    if !self.wait_and_reopen(reason)? {
                        return Ok(());
                    }
                }
            }
        }
    }

    /// Pausa con motivo, poi riapre la sessione (altra pausa se non è ancora pronta).
    /// `Ok(false)` se il job è stato annullato.
    fn wait_and_reopen(&mut self, mut reason: PauseReason) -> Result<bool, AppError> {
        loop {
            if !pause_and_wait(self.ctx, reason) {
                return Ok(false);
            }
            match open_session(self.worker, &self.options) {
                Ok(()) => return Ok(true),
                Err(Stop::Pause(next)) => {
                    publish_pause(self.worker, &next);
                    reason = next;
                }
                Err(Stop::Fail(error)) => return Err(error),
            }
        }
    }
}

/// Esegue lo sviluppo sul thread della fotocamera (`CameraService::run_exclusive`).
///
/// Restituisce il riepilogo del job; il dettaglio (convertite, saltate, errori per RAF) è in
/// `report`. In ogni caso la sessione con la fotocamera è chiusa all'uscita.
pub fn run_develop(
    worker: &mut Worker,
    ctx: &JobContext,
    request: &VariantRequest,
    options: RunOptions,
    report: &mut DevelopReport,
) -> AppResult<RunSummary> {
    let result = develop(worker, ctx, request, options, report);
    // Spec camera-connection: la sessione si chiude al termine o all'annullamento.
    worker.close_session();
    ctx.progress.phase(PHASE).set_current(None);
    result
}

fn invalid(message: String) -> AppError {
    AppError::invalid_input(message)
}

fn develop(
    worker: &mut Worker,
    ctx: &JobContext,
    request: &VariantRequest,
    options: RunOptions,
    report: &mut DevelopReport,
) -> AppResult<RunSummary> {
    let phase = ctx.progress.phase(PHASE);
    let cancelled = |report: &DevelopReport| {
        Ok(RunSummary {
            cancelled: true,
            failed: i64::from(report.failed),
        })
    };

    // Sessione nuova con la fotocamera; un guasto qui mette in pausa invece di fallire.
    loop {
        match open_session(worker, &options) {
            Ok(()) => break,
            Err(Stop::Fail(error)) => return Err(error),
            Err(Stop::Pause(reason)) => {
                if !pause_and_wait(ctx, reason) {
                    return cancelled(report);
                }
            }
        }
    }
    // I banchi si leggono una volta all'avvio: questa istantanea è ciò che si applica.
    let presets = worker.read_presets().map_err(AppError::from)?;
    let plan = super::plan::build_plan(request, &presets).map_err(invalid)?;
    report.output_dir = plan.output_dir.to_string_lossy().into_owned();
    let guard = OutputGuard::new(&plan.output_dir)
        .and_then(|guard| prepare_targets(&plan, &guard).map(|()| guard))
        .map_err(|e| AppError::new(ErrorCode::Io, e))?;

    // Quanto è già concluso: varianti complete sul disco e voci del journal (ripresa di un job
    // interrotto). Il lavoro vero sono le altre.
    let mut todo: Vec<Conversion> = Vec::new();
    let mut journal_skipped = 0usize;
    for &conversion in &plan.pending {
        if ctx.journal.is_finished(&journal_key(&plan, conversion)) {
            journal_skipped += 1;
        } else {
            todo.push(conversion);
        }
    }
    report.skipped = (plan.already_done + journal_skipped) as u32;
    phase.restore(i64::from(report.skipped), 0, plan.total() as i64, true);
    phase.start();

    let mut runner = Runner {
        worker,
        ctx,
        plan: &plan,
        guard,
        options,
        phase: phase.clone(),
        meter: Meter::default(),
        report,
        remaining: todo.len(),
        since_disk_check: 0,
    };
    if !runner.ensure_room() {
        return cancelled(runner.report);
    }

    // Conversioni da fare, raggruppate per RAF nell'ordine del piano.
    let mut groups: Vec<(usize, Vec<usize>)> = Vec::new();
    for c in &todo {
        match groups.last_mut() {
            Some((raf, presets)) if *raf == c.raf => presets.push(c.preset),
            _ => groups.push((c.raf, vec![c.preset])),
        }
    }
    let mut next = groups
        .first()
        .map(|(raf, _)| spawn_read(plan.rafs[*raf].clone()));
    for index in 0..groups.len() {
        if !ctx.control.checkpoint() {
            return cancelled(runner.report);
        }
        let (raf, mut presets) = groups[index].clone();
        let bytes = next.take().map(|h| {
            h.join()
                .unwrap_or_else(|_| Err(std::io::Error::other("lettura interrotta")))
        });
        next = groups
            .get(index + 1)
            .map(|(r, _)| spawn_read(plan.rafs[*r].clone()));
        match bytes {
            Some(Ok(bytes)) => runner.convert_with_pauses(raf, &mut presets, &bytes)?,
            Some(Err(error)) => {
                let unreadable = AppError::from(error);
                runner.fail(raf, &presets, &unreadable);
            }
            None => {}
        }
        if runner.cancelled() {
            return cancelled(runner.report);
        }
    }
    phase.set_current(None);
    phase.finish();
    Ok(RunSummary {
        cancelled: false,
        failed: i64::from(runner.report.failed),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_rejects_paths_outside_output() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("out");
        let guard = OutputGuard::new(&root).unwrap();
        fs::create_dir_all(root.join("C1")).unwrap();
        assert!(guard.check(&root.join("C1").join("a.jpg")).is_ok());
        assert!(
            guard.check(&root.join("a.jpg")).is_err(),
            "root itself is not a preset folder"
        );
        assert!(guard
            .check(&root.join("C1").join("..").join("..").join("evil.jpg"))
            .is_err());
        assert!(guard.check(&dir.path().join("evil.jpg")).is_err());
        assert!(write_atomic(&guard, &dir.path().join("evil.jpg"), b"x").is_err());
        assert!(!dir.path().join("evil.jpg").exists());
        // Una cartella di preset che è un collegamento verso altrove si rifiuta.
        let elsewhere = dir.path().join("elsewhere");
        fs::create_dir_all(&elsewhere).unwrap();
        #[cfg(windows)]
        let linked = std::os::windows::fs::symlink_dir(&elsewhere, root.join("C2")).is_ok();
        #[cfg(not(windows))]
        let linked = std::os::unix::fs::symlink(&elsewhere, root.join("C2")).is_ok();
        if linked {
            assert!(guard.check(&root.join("C2").join("a.jpg")).is_err());
        }
    }

    #[test]
    fn atomic_write_leaves_no_part_file() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("out");
        let guard = OutputGuard::new(&root).unwrap();
        fs::create_dir_all(root.join("C1")).unwrap();
        let target = root.join("C1").join("a.jpg");
        write_atomic(&guard, &target, b"jpeg").unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"jpeg");
        assert!(!root.join("C1").join("a.jpg.part").exists());
    }

    #[test]
    fn camera_errors_are_classified() {
        assert!(matches!(
            classify(&PtpError::Disconnected),
            Some(Stop::Pause(r)) if r.code == pause::CAMERA_DISCONNECTED
        ));
        assert!(matches!(
            classify(&PtpError::Busy),
            Some(Stop::Pause(r)) if r.code == pause::CAMERA_BUSY
        ));
        assert!(matches!(
            classify(&PtpError::Timeout("x".into())),
            Some(Stop::Fail(e)) if e.code == ErrorCode::CameraUnresponsive
        ));
        assert!(classify(&PtpError::Protocol("x".into())).is_none());
    }
}
