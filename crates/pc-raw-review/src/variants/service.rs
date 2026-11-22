//! Thread della fotocamera e servizio che lo usa (porting di `service.rs` di FujiView).
//!
//! Un solo thread possiede COM e la connessione con la fotocamera; chi chiede (comandi della UI,
//! job di sviluppo) gli parla attraverso un canale. Mentre un job è in corso lo stato e i preset
//! si servono dalla cache: il job ha la sessione esclusiva per tutta la sua durata.
//!
//! Rispetto a FujiView il servizio non conosce Tauri: gli eventi (stato, log, verifica) escono da
//! un [`EventSink`], e lo sviluppo non è più una richiesta propria ma un compito
//! ([`CameraService::run_exclusive`]) eseguito dal thread della fotocamera per conto di un job di
//! `pc-jobs`.

use super::diagnostics::{
    run_diagnostics, Cam, CameraBackend, CameraDiagnostics, CameraLog, CameraLogLine, CheckStatus,
    DiagHost, STEP_TIMEOUT, UNAVAILABLE_MESSAGE,
};
use super::plan::{PresetDef, PresetKind};
use super::types::{CameraPreset, CameraState, CameraStatus};
use crate::camera::banks::{apply_bank, film_simulation_name, folder_name};
use crate::camera::profile::Profile;
use crate::camera::ptp::{DeviceInfo, PtpError};
use pc_core::{AppError, AppResult, ErrorCode};
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Cosa il servizio comunica all'esterno.
#[derive(Debug, Clone)]
pub enum CameraEvent {
    Status(CameraStatus),
    Log(CameraLogLine),
    /// Avanzamento (o esito) della verifica della fotocamera.
    Diagnostics(CameraDiagnostics),
}

pub type EventSink = Arc<dyn Fn(CameraEvent) + Send + Sync>;

type Reply<T> = Sender<AppResult<T>>;
type Task = Box<dyn FnOnce(&mut Worker) + Send>;

enum Request {
    Status(Sender<CameraStatus>),
    Presets(Reply<Vec<PresetDef>>),
    Diagnostics(Sender<CameraDiagnostics>),
    /// Compito di lunga durata con la sessione esclusiva (un job di sviluppo).
    Run(Task),
}

/// Cosa sta facendo il thread in questo momento (nome, da quando).
type Activity = Arc<Mutex<Option<(&'static str, Instant)>>>;
/// Ultimo avanzamento di una verifica in corso e quando è stato scritto.
type DiagProgress = Arc<Mutex<Option<(CameraDiagnostics, Instant)>>>;

const STATUS_WAIT: Duration = Duration::from_secs(5);
const PRESETS_WAIT: Duration = Duration::from_secs(45);
/// Operazioni che durano legittimamente a lungo: non si segnalano come fotocamera bloccata.
const SLOW_OPS: [&str; 2] = ["diagnostica", "preset"];

struct Connected {
    camera: Cam,
    info: DeviceInfo,
    simulated: bool,
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

fn status_of(connected: &Connected) -> CameraStatus {
    let missing = connected.info.missing_conversion_support();
    let (state, message) = if missing.is_empty() {
        (CameraState::Ready, None)
    } else {
        (
            CameraState::NotReady,
            Some("La fotocamera non è in modalità conversione RAW: imposta USB RAW CONV./BACKUP RESTORE nel menu della fotocamera.".to_string()),
        )
    };
    CameraStatus {
        available: true,
        state,
        model: Some(connected.info.model.clone()),
        firmware: Some(connected.info.device_version.clone()),
        message,
        simulated: connected.simulated,
    }
}

fn status_from_error(error: &PtpError) -> CameraStatus {
    match error {
        PtpError::Disconnected => CameraStatus::new(CameraState::Disconnected, None),
        PtpError::Busy => CameraStatus::new(
            CameraState::Busy,
            Some("Fotocamera occupata: chiudi X RAW Studio.".to_string()),
        ),
        other => CameraStatus::new(CameraState::Error, Some(other.to_string())),
    }
}

/// L'errore che dice perché la fotocamera non è pronta.
pub fn status_error(status: &CameraStatus) -> AppError {
    let message = |fallback: &str| {
        status
            .message
            .clone()
            .unwrap_or_else(|| fallback.to_string())
    };
    match status.state {
        _ if !status.available => AppError::invalid_input(UNAVAILABLE_MESSAGE),
        CameraState::Disconnected => AppError::new(
            ErrorCode::CameraDisconnected,
            "Nessuna fotocamera collegata. Controlla il cavo USB e accendi la fotocamera.",
        ),
        CameraState::Busy => AppError::new(
            ErrorCode::CameraBusy,
            message("Fotocamera occupata: chiudi X RAW Studio e gli altri programmi Fujifilm."),
        ),
        CameraState::Unresponsive => AppError::new(
            ErrorCode::CameraUnresponsive,
            message("La fotocamera non risponde: spegnila e riaccendila."),
        ),
        CameraState::NotReady | CameraState::Error | CameraState::Ready => AppError::new(
            ErrorCode::Io,
            message("Nessuna fotocamera pronta per la conversione."),
        ),
    }
}

/// Un preset come lo vede la UI: nome, cartella, riepilogo e impostazioni non applicabili.
pub fn preset_view(preset: &PresetDef) -> CameraPreset {
    match &preset.kind {
        PresetKind::AsShot => CameraPreset {
            id: preset.id.clone(),
            bank: None,
            name: "Come scattato".into(),
            folder_name: preset.base_folder_name(),
            film_simulation: None,
            summary: "Impostazioni registrate nel RAF".into(),
            unapplied_properties: Vec::new(),
        },
        PresetKind::Bank(bank) => {
            // Le stesse regole della conversione, su un profilo di base neutro.
            let neutral = Profile::synthetic("", &[0; 29]);
            let unapplied = apply_bank(&neutral, bank)
                .map(|a| a.unapplied)
                .unwrap_or_default();
            let unapplied = unapplied
                .into_iter()
                .filter(|u| !u.starts_with("gamma dinamica limitata"))
                .collect();
            CameraPreset {
                id: preset.id.clone(),
                bank: Some(bank.index),
                name: if bank.name.is_empty() {
                    format!("C{}", bank.index)
                } else {
                    bank.name.clone()
                },
                folder_name: folder_name(bank.index, &bank.name),
                film_simulation: bank
                    .film_simulation()
                    .and_then(film_simulation_name)
                    .map(str::to_string),
                summary: bank.summary(),
                unapplied_properties: unapplied,
            }
        }
    }
}

fn status_after_failed_diagnostics(result: &CameraDiagnostics) -> CameraStatus {
    if result.unresponsive {
        return unresponsive_status(None);
    }
    if result
        .check("device")
        .is_some_and(|c| c.status == CheckStatus::Error)
    {
        return CameraStatus::new(CameraState::Disconnected, None);
    }
    let message = result
        .checks
        .iter()
        .find(|c| c.status == CheckStatus::Error)
        .and_then(|c| c.detail.clone());
    let mut status = CameraStatus::new(CameraState::NotReady, message);
    status.model = result.model.clone();
    status.firmware = result.firmware.clone();
    status.simulated = result.simulated;
    status
}

fn unresponsive_status(previous: Option<CameraStatus>) -> CameraStatus {
    let mut status = CameraStatus::new(
        CameraState::Unresponsive,
        Some(
            "In attesa della fotocamera… Se non risponde, spegni e riaccendi la fotocamera."
                .to_string(),
        ),
    );
    if let Some(previous) = previous {
        status.model = previous.model;
        status.firmware = previous.firmware;
        status.simulated = previous.simulated;
    }
    status
}

/// Il thread della fotocamera. Passato ai compiti di [`CameraService::run_exclusive`].
pub struct Worker {
    backend: CameraBackend,
    sink: EventSink,
    status: Arc<Mutex<CameraStatus>>,
    presets: Arc<Mutex<Vec<PresetDef>>>,
    connected: Option<Connected>,
    pub log: CameraLog,
    activity: Activity,
    diag_progress: DiagProgress,
}

impl Worker {
    fn publish(&self, status: CameraStatus) -> CameraStatus {
        let changed = *lock(&self.status) != status;
        *lock(&self.status) = status.clone();
        if changed {
            let detail = status.message.as_deref().unwrap_or("");
            let line = format!("Stato fotocamera: {:?} {detail}", status.state);
            match status.state {
                CameraState::Error | CameraState::Unresponsive => self.log.error(line),
                CameraState::Ready => self.log.info(format!(
                    "Fotocamera pronta: {} fw {}",
                    status.model.as_deref().unwrap_or("?"),
                    status.firmware.as_deref().unwrap_or("?")
                )),
                _ => self.log.warn(line),
            }
            (self.sink)(CameraEvent::Status(status.clone()));
        }
        status
    }

    /// Aggiorna lo stato mostrato senza toccare la fotocamera (per esempio dopo che un job ha
    /// scoperto che è stata scollegata).
    pub fn set_state(&self, state: CameraState, message: Option<String>) {
        let mut status = CameraStatus::new(state, message);
        status.simulated = self.backend.is_simulated();
        self.publish(status);
    }

    /// Chiude la sessione con la fotocamera (la connessione viene lasciata cadere).
    pub fn close_session(&mut self) {
        self.connected = None;
    }

    /// La connessione aperta, se c'è.
    pub fn camera(&mut self) -> Option<&mut Cam> {
        self.connected.as_mut().map(|c| &mut c.camera)
    }

    /// Si assicura che esista una connessione viva e restituisce lo stato attuale.
    pub fn refresh(&mut self) -> CameraStatus {
        if let Some(connected) = self.connected.as_mut() {
            match connected.camera.device_info() {
                Ok(info) => connected.info = info,
                Err(error) => {
                    self.log.warn(format!("Connessione persa: {error}"));
                    self.connected = None;
                }
            }
        }
        if self.connected.is_none() {
            match self.backend.open().and_then(|camera| match camera {
                Some(mut camera) => camera.device_info().map(|info| Connected {
                    camera,
                    info,
                    simulated: self.backend.is_simulated(),
                }),
                None => Err(PtpError::Disconnected),
            }) {
                Ok(connected) => self.connected = Some(connected),
                Err(error) => return self.publish(status_from_error(&error)),
            }
        }
        let status = match self.connected.as_ref() {
            Some(connected) => status_of(connected),
            None => CameraStatus::new(CameraState::Disconnected, None),
        };
        self.publish(status)
    }

    /// Legge i banchi dalla fotocamera pronta e li tiene come istantanea: è quella che si applica.
    /// Un errore della fotocamera chiude la sessione (si riapre alla prossima richiesta).
    pub fn read_presets(&mut self) -> Result<Vec<PresetDef>, PtpError> {
        let Some(connected) = self.connected.as_mut() else {
            return Err(PtpError::Disconnected);
        };
        let banks = match connected.camera.read_banks() {
            Ok(banks) => banks,
            Err(error) => {
                self.connected = None;
                return Err(error);
            }
        };
        let mut presets = vec![PresetDef::as_shot()];
        presets.extend(banks.into_iter().map(PresetDef::bank));
        *lock(&self.presets) = presets.clone();
        Ok(presets)
    }

    /// Stato + lettura dei preset per la richiesta della UI.
    fn presets_for_ui(&mut self) -> AppResult<Vec<PresetDef>> {
        let status = self.refresh();
        if status.state != CameraState::Ready {
            return Err(status_error(&status));
        }
        self.read_presets().map_err(|error| {
            self.log
                .error(format!("Lettura dei preset non riuscita: {error}"));
            AppError::from(error)
        })
    }

    fn diagnose(&mut self) -> CameraDiagnostics {
        self.log.info("Verifica della fotocamera avviata");
        let existing = self.connected.take();
        let simulated = self.backend.is_simulated();
        let mut host = DiagHost::new(existing.map(|c| c.camera), self.backend.clone());
        let progress = self.diag_progress.clone();
        let sink = self.sink.clone();
        *lock(&progress) = Some((CameraDiagnostics::new(simulated), Instant::now()));
        let result = run_diagnostics(&mut host, &mut |snapshot| {
            *lock(&progress) = Some((snapshot.clone(), Instant::now()));
            sink(CameraEvent::Diagnostics(snapshot.clone()));
        });
        *lock(&progress) = None;
        for check in &result.checks {
            let detail = check.detail.as_deref().unwrap_or("");
            match check.status {
                CheckStatus::Error => self.log.error(format!("{}: {detail}", check.label)),
                CheckStatus::Warning => self.log.warn(format!("{}: {detail}", check.label)),
                _ => {}
            }
        }
        self.log.info(if result.ready {
            "Verifica completata: tutto in ordine"
        } else {
            "Verifica completata: problemi rilevati"
        });
        self.connected = host.into_connected().map(|(camera, info)| Connected {
            camera,
            info,
            simulated,
        });
        let status = match &self.connected {
            Some(connected) => status_of(connected),
            None => status_after_failed_diagnostics(&result),
        };
        self.publish(status);
        result
    }

    fn serve(mut self, rx: Receiver<Request>) {
        #[cfg(windows)]
        let _com = crate::camera::wpd::ComGuard::init();
        for request in rx {
            let what = match &request {
                Request::Status(_) => "stato",
                Request::Presets(_) => "preset",
                Request::Run(_) => "sviluppo",
                Request::Diagnostics(_) => "diagnostica",
            };
            *lock(&self.activity) = Some((what, Instant::now()));
            match request {
                Request::Status(reply) => {
                    let _ = reply.send(self.refresh());
                }
                Request::Presets(reply) => {
                    let _ = reply.send(self.presets_for_ui());
                }
                Request::Run(task) => task(&mut self),
                Request::Diagnostics(reply) => {
                    let _ = reply.send(self.diagnose());
                }
            }
            *lock(&self.activity) = None;
        }
    }
}

/// Segnala che la sessione esclusiva è in uso; si libera quando la guardia cade.
struct Exclusive(Arc<AtomicBool>);

impl Drop for Exclusive {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

/// Il servizio della fotocamera: uno solo per app, nello stato di Tauri.
pub struct CameraService {
    backend: CameraBackend,
    tx: Mutex<Option<Sender<Request>>>,
    status: Arc<Mutex<CameraStatus>>,
    presets: Arc<Mutex<Vec<PresetDef>>>,
    running: Arc<AtomicBool>,
    pub log: CameraLog,
    activity: Activity,
    diag_progress: DiagProgress,
    last_diag: Mutex<Option<CameraDiagnostics>>,
    sink: EventSink,
}

impl CameraService {
    pub fn start(backend: CameraBackend, sink: EventSink) -> Self {
        let initial = if backend.is_available() {
            CameraStatus::new(CameraState::Disconnected, None)
        } else {
            CameraStatus::unavailable(UNAVAILABLE_MESSAGE)
        };
        let status = Arc::new(Mutex::new(initial));
        let presets = Arc::new(Mutex::new(Vec::new()));
        let log = CameraLog::new(Some({
            let sink = sink.clone();
            Arc::new(move |line: &CameraLogLine| sink(CameraEvent::Log(line.clone())))
        }));
        let activity: Activity = Arc::new(Mutex::new(None));
        let diag_progress: DiagProgress = Arc::new(Mutex::new(None));
        let tx = backend.is_available().then(|| {
            let (tx, rx) = channel();
            let (backend, sink, status, presets) = (
                backend.clone(),
                sink.clone(),
                status.clone(),
                presets.clone(),
            );
            let (log, activity, diag_progress) =
                (log.clone(), activity.clone(), diag_progress.clone());
            // La connessione nasce sul thread della fotocamera e non lo lascia mai: il `Worker`
            // (che la possiede) si costruisce qui dentro.
            std::thread::Builder::new()
                .name("raw-review-camera".into())
                .spawn(move || {
                    Worker {
                        backend,
                        sink,
                        status,
                        presets,
                        connected: None,
                        log,
                        activity,
                        diag_progress,
                    }
                    .serve(rx)
                })
                .expect("thread della fotocamera");
            tx
        });
        Self {
            backend,
            tx: Mutex::new(tx),
            status,
            presets,
            running: Arc::new(AtomicBool::new(false)),
            log,
            activity,
            diag_progress,
            last_diag: Mutex::new(None),
            sink,
        }
    }

    /// Esiste una fotocamera (reale o simulata) con cui dialogare su questa piattaforma.
    pub fn is_available(&self) -> bool {
        self.backend.is_available()
    }

    fn unavailable_error() -> AppError {
        AppError::invalid_input(UNAVAILABLE_MESSAGE)
    }

    /// Manda una richiesta al thread e attende al massimo `wait` la risposta.
    fn ask<T: Send + 'static>(
        &self,
        wait: Duration,
        make: impl FnOnce(Sender<T>) -> Request,
    ) -> AppResult<T> {
        let (reply, rx) = channel();
        lock(&self.tx)
            .as_ref()
            .ok_or_else(Self::unavailable_error)?
            .send(make(reply))
            .map_err(|_| AppError::internal("servizio della fotocamera non disponibile"))?;
        rx.recv_timeout(wait).map_err(|error| match error {
            RecvTimeoutError::Timeout => AppError::new(
                ErrorCode::CameraUnresponsive,
                "La fotocamera non risponde. Spegnila e riaccendila, poi riprova.",
            ),
            RecvTimeoutError::Disconnected => {
                AppError::internal("servizio della fotocamera non disponibile")
            }
        })
    }

    /// Da quanto il thread è dentro l'operazione in corso, se ce n'è una.
    fn busy_for(&self) -> Option<(&'static str, Duration)> {
        (*lock(&self.activity)).map(|(what, since)| (what, since.elapsed()))
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn cached_status(&self) -> CameraStatus {
        lock(&self.status).clone()
    }

    /// Stato rapido che non resta mai bloccato su una fotocamera appesa.
    pub fn status(&self) -> CameraStatus {
        if !self.is_available() {
            return CameraStatus::unavailable(UNAVAILABLE_MESSAGE);
        }
        if self.is_running() {
            return self.cached_status();
        }
        if let Some((what, busy)) = self.busy_for() {
            if busy > Duration::from_secs(3) {
                return if SLOW_OPS.contains(&what) && busy < PRESETS_WAIT {
                    self.cached_status()
                } else {
                    unresponsive_status(Some(self.cached_status()))
                };
            }
        }
        match self.ask(STATUS_WAIT, Request::Status) {
            Ok(status) => status,
            Err(_) => {
                self.log
                    .error("La fotocamera non risponde alla richiesta di stato");
                unresponsive_status(Some(self.cached_status()))
            }
        }
    }

    /// I preset della fotocamera (banchi C1-C7 più "Come scattato"). Durante uno sviluppo si
    /// serve l'istantanea letta all'avvio.
    pub fn presets(&self) -> AppResult<Vec<PresetDef>> {
        if !self.is_available() {
            return Err(Self::unavailable_error());
        }
        if self.is_running() {
            return Ok(lock(&self.presets).clone());
        }
        self.ask(PRESETS_WAIT, Request::Presets)?
    }

    /// L'ultima istantanea dei preset (vuota se non sono mai stati letti).
    pub fn cached_presets(&self) -> Vec<PresetDef> {
        lock(&self.presets).clone()
    }

    pub fn presets_view(&self) -> AppResult<Vec<CameraPreset>> {
        Ok(self.presets()?.iter().map(preset_view).collect())
    }

    fn finish_diagnostics(&self, result: CameraDiagnostics) -> CameraDiagnostics {
        (self.sink)(CameraEvent::Diagnostics(result.clone()));
        *lock(&self.last_diag) = Some(result.clone());
        result
    }

    /// La verifica completa. Non blocca per più di `STEP_TIMEOUT` oltre l'ultimo passo concluso,
    /// nemmeno se la fotocamera appende il thread.
    pub fn diagnostics(&self) -> AppResult<CameraDiagnostics> {
        if !self.is_available() {
            return Err(Self::unavailable_error());
        }
        if self.is_running() {
            return lock(&self.last_diag).clone().ok_or_else(|| {
                AppError::invalid_input(
                    "Sviluppo in corso: la verifica completa non è disponibile.",
                )
            });
        }
        let simulated = self.backend.is_simulated();
        if let Some((what, busy)) = self.busy_for() {
            if busy > STEP_TIMEOUT {
                let snapshot = lock(&self.diag_progress).clone();
                let result = match snapshot {
                    Some((partial, at)) if what == "diagnostica" => {
                        partial.into_unresponsive(at.elapsed())
                    }
                    _ => CameraDiagnostics::stuck(simulated, what, busy),
                };
                return Ok(self.finish_diagnostics(result));
            }
        }
        *lock(&self.diag_progress) = None;
        let (reply, rx) = channel();
        lock(&self.tx)
            .as_ref()
            .ok_or_else(Self::unavailable_error)?
            .send(Request::Diagnostics(reply))
            .map_err(|_| AppError::internal("servizio della fotocamera non disponibile"))?;
        let started = Instant::now();
        loop {
            match rx.recv_timeout(Duration::from_millis(200)) {
                Ok(result) => return Ok(self.finish_diagnostics(result)),
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(AppError::internal(
                        "servizio della fotocamera non disponibile",
                    ))
                }
                Err(RecvTimeoutError::Timeout) => {
                    let snapshot = lock(&self.diag_progress).clone();
                    let (waited, partial) = match snapshot {
                        Some((partial, at)) => (at.elapsed(), Some(partial)),
                        None => (started.elapsed(), None),
                    };
                    if waited > STEP_TIMEOUT {
                        self.log.error(format!(
                            "La fotocamera non risponde da {} s: verifica interrotta",
                            waited.as_secs()
                        ));
                        let result = match partial {
                            Some(partial) => partial.into_unresponsive(waited),
                            None => CameraDiagnostics::stuck(simulated, "in coda", waited),
                        };
                        return Ok(self.finish_diagnostics(result));
                    }
                }
            }
        }
    }

    /// Esegue `task` sul thread della fotocamera con la sessione esclusiva e ne attende la fine.
    /// Un solo compito alla volta: un secondo tentativo fallisce subito. Se il compito va in
    /// panico, il thread della fotocamera resta in vita e l'errore arriva a chi aspetta.
    pub fn run_exclusive<R: Send + 'static>(
        &self,
        task: impl FnOnce(&mut Worker) -> R + Send + 'static,
    ) -> AppResult<R> {
        if !self.is_available() {
            return Err(Self::unavailable_error());
        }
        if self.running.swap(true, Ordering::SeqCst) {
            return Err(AppError::invalid_input("Uno sviluppo è già in corso."));
        }
        let _exclusive = Exclusive(self.running.clone());
        let (done, finished) = channel();
        let wrapped: Task = Box::new(move |worker| {
            let outcome = std::panic::catch_unwind(AssertUnwindSafe(|| task(worker)));
            let _ = done.send(outcome);
        });
        lock(&self.tx)
            .as_ref()
            .ok_or_else(Self::unavailable_error)?
            .send(Request::Run(wrapped))
            .map_err(|_| AppError::internal("servizio della fotocamera non disponibile"))?;
        match finished.recv() {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(_)) => Err(AppError::internal("panico nello sviluppo dalla fotocamera")),
            Err(_) => Err(AppError::internal(
                "servizio della fotocamera non disponibile",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::fuji::FujiCamera;
    use crate::camera::sim::SimulatedCamera;
    use crate::variants::diagnostics::Opener;

    fn sink() -> EventSink {
        Arc::new(|_| {})
    }

    #[test]
    fn without_a_camera_everything_says_windows_only() {
        let service = CameraService::start(CameraBackend::Unavailable, sink());
        assert!(!service.is_available());
        let status = service.status();
        assert!(!status.available);
        assert_eq!(status.state, CameraState::Disconnected);
        assert_eq!(
            status.message.as_deref(),
            Some("Disponibile solo su Windows")
        );
        assert_eq!(service.cached_status(), status);
        for error in [
            service.presets().unwrap_err(),
            service.diagnostics().unwrap_err(),
            service.run_exclusive(|_| ()).unwrap_err(),
        ] {
            assert_eq!(error.detail, "Disponibile solo su Windows");
        }
        assert_eq!(status_error(&status).detail, "Disponibile solo su Windows");
    }

    fn simulated() -> CameraService {
        let opener: Opener =
            Arc::new(|| Ok(Some(FujiCamera::new(Box::new(SimulatedCamera::new())))));
        CameraService::start(CameraBackend::Custom(opener), sink())
    }

    #[test]
    fn a_ready_camera_lists_its_banks_with_the_as_shot_variant() {
        let service = simulated();
        let status = service.status();
        assert!(status.available && status.simulated);
        assert_eq!(status.state, CameraState::Ready);
        assert_eq!(status.model.as_deref(), Some("X-T5"));
        let presets = service.presets_view().unwrap();
        let ids: Vec<&str> = presets.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["asshot", "C1", "C2", "C3", "C4", "C5", "C6", "C7"]);
        assert_eq!(presets[0].name, "Come scattato");
        assert_eq!(presets[1].folder_name, "C1 Sim C1");
        assert!(presets[1].film_simulation.is_some());
        assert!(!presets[1].summary.is_empty());
        assert_eq!(service.cached_presets().len(), 8);
        // La verifica passa e dice che si può avviare.
        let diagnostics = service.diagnostics().unwrap();
        assert!(diagnostics.ready, "{diagnostics:?}");
    }

    #[test]
    fn a_missing_or_busy_camera_gets_a_clear_error() {
        let gone: Opener = Arc::new(|| Ok(None));
        let service = CameraService::start(CameraBackend::Custom(gone), sink());
        assert_eq!(service.status().state, CameraState::Disconnected);
        let error = service.presets().unwrap_err();
        assert_eq!(error.code, ErrorCode::CameraDisconnected);
        let busy: Opener = Arc::new(|| Err(PtpError::Busy));
        let service = CameraService::start(CameraBackend::Custom(busy), sink());
        let status = service.status();
        assert_eq!(status.state, CameraState::Busy);
        assert!(status.message.unwrap().contains("X RAW Studio"));
        assert_eq!(service.presets().unwrap_err().code, ErrorCode::CameraBusy);
        // La verifica segnala il problema e non permette di avviare.
        let diagnostics = service.diagnostics().unwrap();
        assert!(!diagnostics.ready);
    }

    #[test]
    fn a_panicking_task_does_not_kill_the_camera_thread() {
        let service = simulated();
        let error = service
            .run_exclusive(|_| -> () { panic!("prova") })
            .unwrap_err();
        assert!(error.detail.contains("panico"), "{}", error.detail);
        assert!(!service.is_running());
        assert_eq!(service.status().state, CameraState::Ready);
        assert_eq!(service.run_exclusive(|_| 5).unwrap(), 5);
    }
}
