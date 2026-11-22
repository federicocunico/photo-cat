//! Verifica della fotocamera (porting di `diagnostics.rs` di FujiView): un elenco di controlli
//! passo per passo, un registro circolare delle righe di log della fotocamera e il modo di
//! aprirla (reale, simulata o fornita da chi usa il crate).
//!
//! L'elenco gira sul thread della fotocamera. Una chiamata WPD bloccata non si può annullare:
//! il servizio guarda l'ultimo avanzamento condiviso e segnala "la fotocamera non risponde"
//! da fuori (vedi `service.rs`).

use crate::camera::fuji::FujiCamera;
use crate::camera::ptp::{prop, DeviceInfo, PtpError, PtpTransport};
#[cfg(any(test, feature = "fake-camera"))]
use crate::camera::sim::SimulatedCamera;
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::Arc as StdArc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const LOG_CAPACITY: usize = 200;
/// A step slower than this is reported as "camera not responding".
pub const STEP_TIMEOUT: Duration = Duration::from_secs(12);
pub const FROZEN_HINT: &str = "Spegni e riaccendi la camera (se serve scollega e ricollega il cavo USB), poi premi Ricontrolla.";

pub type Cam = FujiCamera<Box<dyn PtpTransport>>;

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------- log

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct CameraLogLine {
    /// Milliseconds since the Unix epoch.
    pub at: u64,
    /// "info" | "warn" | "error"
    pub level: String,
    pub message: String,
}

#[derive(Clone)]
pub struct CameraLog {
    lines: Arc<Mutex<VecDeque<CameraLogLine>>>,
    // A closure (not an AppHandle) keeps the tauri runtime out of unit tests.
    sink: Option<LogSink>,
}

pub type LogSink = Arc<dyn Fn(&CameraLogLine) + Send + Sync>;

impl CameraLog {
    pub fn new(sink: Option<LogSink>) -> Self {
        Self {
            lines: Arc::new(Mutex::new(VecDeque::new())),
            sink,
        }
    }

    pub fn push(&self, level: &str, message: impl Into<String>) {
        let line = CameraLogLine {
            at: now_ms(),
            level: level.to_string(),
            message: message.into(),
        };
        {
            let mut lines = self.lines.lock().unwrap();
            if lines.len() >= LOG_CAPACITY {
                lines.pop_front();
            }
            lines.push_back(line.clone());
        }
        if let Some(sink) = &self.sink {
            sink(&line);
        }
    }

    pub fn info(&self, message: impl Into<String>) {
        self.push("info", message);
    }
    pub fn warn(&self, message: impl Into<String>) {
        self.push("warn", message);
    }
    pub fn error(&self, message: impl Into<String>) {
        self.push("error", message);
    }

    pub fn snapshot(&self) -> Vec<CameraLogLine> {
        self.lines.lock().unwrap().iter().cloned().collect()
    }
}

// ---------------------------------------------------------------- model

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum CheckStatus {
    Ok,
    Warning,
    Error,
    Pending,
    Skipped,
    Info,
}

use CheckStatus as S;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticCheck {
    pub id: String,
    pub label: String,
    pub status: CheckStatus,
    pub detail: Option<String>,
    pub hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct CameraDiagnostics {
    pub checks: Vec<DiagnosticCheck>,
    pub model: Option<String>,
    pub firmware: Option<String>,
    pub simulated: bool,
    pub checked_at: u64,
    /// Nessun controllo è fallito o saltato: lo sviluppo può partire.
    pub ready: bool,
    /// A step hung or failed in a way that points to a frozen camera.
    pub unresponsive: bool,
    /// Still running (some checks pending).
    pub running: bool,
}

const STEPS: [(&str, &str); 10] = [
    ("device", "Camera Fujifilm vista da Windows"),
    ("software", "Nessun programma in conflitto"),
    ("connection", "Connessione alla camera"),
    ("response", "La camera risponde"),
    ("model", "Modello supportato"),
    ("usbMode", "Modalità USB: RAW CONV."),
    ("driver", "Driver e funzioni di conversione"),
    ("banks", "Preset personalizzati leggibili"),
    ("autoPowerOff", "Spegnimento automatico"),
    ("power", "Alimentazione"),
];

impl CameraDiagnostics {
    pub fn new(simulated: bool) -> Self {
        let checks = STEPS
            .iter()
            .map(|(id, label)| DiagnosticCheck {
                id: id.to_string(),
                label: label.to_string(),
                status: S::Pending,
                detail: None,
                hint: None,
            })
            .collect();
        Self {
            checks,
            model: None,
            firmware: None,
            simulated,
            checked_at: now_ms(),
            ready: false,
            unresponsive: false,
            running: true,
        }
    }

    pub fn check(&self, id: &str) -> Option<&DiagnosticCheck> {
        self.checks.iter().find(|c| c.id == id)
    }

    fn set(&mut self, id: &str, status: CheckStatus, detail: Option<String>, hint: Option<&str>) {
        if let Some(check) = self.checks.iter_mut().find(|c| c.id == id) {
            check.status = status;
            check.detail = detail;
            check.hint = hint.map(str::to_string);
        }
        self.checked_at = now_ms();
    }

    /// Pending checks become skipped; computes `ready`.
    fn finish(&mut self, skip_reason: &str) {
        for check in &mut self.checks {
            if check.status == S::Pending {
                check.status = S::Skipped;
                check.detail = Some(skip_reason.to_string());
            }
        }
        self.running = false;
        self.checked_at = now_ms();
        self.ready = !self
            .checks
            .iter()
            .any(|c| matches!(c.status, S::Error | S::Skipped));
    }

    /// Build the result for a camera that stopped answering while `self` (a
    /// partial snapshot) was being produced.
    pub fn into_unresponsive(mut self, waited: Duration) -> Self {
        let detail = format!("nessuna risposta dalla camera da {} s", waited.as_secs());
        let target = self
            .checks
            .iter()
            .find(|c| c.status == S::Pending)
            .map(|c| c.id.clone());
        if let Some(id) = target {
            self.set(&id, S::Error, Some(detail), Some(FROZEN_HINT));
        }
        self.unresponsive = true;
        self.finish("Non eseguito: la camera non risponde.");
        self
    }

    /// Result when the worker is stuck in an earlier operation and no
    /// checklist progress exists.
    pub fn stuck(simulated: bool, what: &str, waited: Duration) -> Self {
        let mut d = Self::new(simulated);
        for id in ["device", "software", "connection"] {
            d.set(id, S::Skipped, Some("Non verificato".into()), None);
        }
        d.set(
            "response",
            S::Pending,
            Some(format!("operazione «{what}» in sospeso")),
            None,
        );
        d.into_unresponsive(waited)
    }
}

// ---------------------------------------------------------------- errors

fn is_frozen_hresult(code: i32) -> bool {
    matches!(code as u32, 0x802A_0006 | 0x8007_001F)
}

/// True when the error looks like a frozen (not merely absent) camera.
pub fn looks_frozen(error: &PtpError, elapsed: Duration) -> bool {
    elapsed >= STEP_TIMEOUT
        || match error {
            PtpError::Timeout(_) => true,
            PtpError::Transport { code, .. } => is_frozen_hresult(*code),
            _ => false,
        }
}

/// Returns (detail, hint, frozen) for a failed step.
pub fn describe_error(error: &PtpError, elapsed: Duration) -> (String, &'static str, bool) {
    let frozen = looks_frozen(error, elapsed);
    if frozen {
        return (
            format!("la camera non risponde ({error})"),
            FROZEN_HINT,
            true,
        );
    }
    let hint = match error {
        PtpError::Busy => "Chiudi FUJIFILM X RAW Studio e le altre app Fujifilm, poi premi Ricontrolla.",
        PtpError::Disconnected => {
            "Controlla il cavo e che la camera sia accesa. Se lo è e non risponde, spegnila e riaccendila."
        }
        _ => "Premi Ricontrolla. Se il problema resta, spegni e riaccendi la camera e copia il log camera per l'assistenza.",
    };
    (error.to_string(), hint, false)
}

pub fn usb_mode_label(mode: u32) -> String {
    match mode {
        5 => "Tether".into(),
        6 => "RAW CONV./BACKUP RESTORE".into(),
        8 => "Webcam".into(),
        other => format!("modalità {other}"),
    }
}

/// Little-endian integer from a PTP property payload (1, 2 or 4 bytes).
pub fn le_value(data: &[u8]) -> Option<u32> {
    match data.len() {
        0 => None,
        1 => Some(data[0] as u32),
        2 => Some(u16::from_le_bytes([data[0], data[1]]) as u32),
        _ => Some(u32::from_le_bytes([data[0], data[1], data[2], data[3]])),
    }
}

/// Pick conflicting programs out of `tasklist /FO CSV /NH` output.
pub fn find_conflicts(tasklist: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for line in tasklist.lines() {
        let name = line
            .trim()
            .trim_start_matches('"')
            .split("\",")
            .next()
            .unwrap_or("")
            .trim_end_matches('"')
            .to_string();
        let upper = name.to_ascii_uppercase();
        if upper.is_empty() || upper.contains("FUJIVIEW") {
            continue;
        }
        let hit = upper.contains("RAW STUDIO")
            || upper.contains("RAWSTUDIO")
            || upper.contains("X ACQUIRE")
            || upper.contains("XACQUIRE")
            || upper.contains("FUJIFILM")
            || upper.contains("TETHER");
        if hit && !found.iter().any(|f| f.eq_ignore_ascii_case(&name)) {
            found.push(name);
        }
    }
    found
}

#[cfg(windows)]
fn running_conflicts() -> Vec<String> {
    use std::os::windows::process::CommandExt;
    let output = std::process::Command::new("tasklist")
        .args(["/FO", "CSV", "/NH"])
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .output();
    match output {
        Ok(out) => find_conflicts(&String::from_utf8_lossy(&out.stdout)),
        Err(_) => Vec::new(),
    }
}

// ---------------------------------------------------------------- backend

/// Difetto simulato dalla fotocamera finta (prove manuali e test).
#[cfg(any(test, feature = "fake-camera"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimFault {
    None,
    Frozen,
    WrongMode,
    Absent,
}

/// Configurazione della fotocamera simulata.
#[cfg(any(test, feature = "fake-camera"))]
#[derive(Debug, Clone)]
pub struct SimConfig {
    pub fault: SimFault,
    /// Quanto dura la finta chiamata bloccata di `SimFault::Frozen`.
    pub freeze: Duration,
    /// Durata simulata di una conversione.
    pub conversion_delay: Duration,
}

#[cfg(any(test, feature = "fake-camera"))]
impl Default for SimConfig {
    fn default() -> Self {
        Self {
            fault: SimFault::None,
            freeze: Duration::from_millis(3000),
            conversion_delay: Duration::from_millis(1500),
        }
    }
}

#[cfg(any(test, feature = "fake-camera"))]
impl SimConfig {
    /// Dalle variabili `PHOTOCAT_FAKE_CAMERA_FAULT` (`frozen`, `wrongmode`, `absent`),
    /// `PHOTOCAT_FAKE_FREEZE_MS` e `PHOTOCAT_FAKE_DELAY_MS`.
    pub fn from_env() -> Self {
        let millis = |name: &str, default: Duration| {
            std::env::var(name)
                .ok()
                .and_then(|v| v.parse().ok())
                .map(Duration::from_millis)
                .unwrap_or(default)
        };
        let base = Self::default();
        let fault = match std::env::var("PHOTOCAT_FAKE_CAMERA_FAULT")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str()
        {
            "frozen" => SimFault::Frozen,
            "wrongmode" => SimFault::WrongMode,
            "absent" => SimFault::Absent,
            _ => SimFault::None,
        };
        Self {
            fault,
            freeze: millis("PHOTOCAT_FAKE_FREEZE_MS", base.freeze),
            conversion_delay: millis("PHOTOCAT_FAKE_DELAY_MS", base.conversion_delay),
        }
    }
}

/// Come si apre una fotocamera (sul thread della fotocamera, mai altrove).
pub type Opener = StdArc<dyn Fn() -> Result<Option<Cam>, PtpError> + Send + Sync>;

/// Da dove viene la fotocamera.
#[derive(Clone)]
pub enum CameraBackend {
    /// Nessuna fotocamera su questa piattaforma (Linux e macOS senza simulatore).
    Unavailable,
    /// Fotocamera reale, tramite Windows Portable Devices.
    #[cfg(windows)]
    Real,
    /// Fotocamera simulata (feature `fake-camera`).
    #[cfg(any(test, feature = "fake-camera"))]
    Simulated(SimConfig),
    /// Fotocamera fornita da chi usa il crate (test con un simulatore condiviso).
    #[cfg(any(test, feature = "fake-camera"))]
    Custom(Opener),
}

pub const UNAVAILABLE_MESSAGE: &str = "Disponibile solo su Windows";

impl CameraBackend {
    /// Il backend adatto a questa build: la fotocamera reale su Windows; con la feature
    /// `fake-camera` quella simulata (su Windows solo con `PHOTOCAT_FAKE_CAMERA=1`, altrove
    /// sempre); altrimenti nessuna.
    pub fn for_platform() -> Self {
        #[cfg(any(test, feature = "fake-camera"))]
        {
            let wanted = std::env::var("PHOTOCAT_FAKE_CAMERA")
                .is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
            if wanted || !cfg!(windows) {
                return Self::Simulated(SimConfig::from_env());
            }
        }
        #[cfg(windows)]
        {
            Self::Real
        }
        #[cfg(not(windows))]
        {
            Self::Unavailable
        }
    }

    /// Esiste una fotocamera (reale o simulata) con cui dialogare.
    pub fn is_available(&self) -> bool {
        !matches!(self, Self::Unavailable)
    }

    pub fn is_simulated(&self) -> bool {
        match self {
            #[cfg(any(test, feature = "fake-camera"))]
            Self::Simulated(_) | Self::Custom(_) => true,
            _ => false,
        }
    }

    /// Apre la connessione senza leggere nulla dalla fotocamera.
    pub fn open(&self) -> Result<Option<Cam>, PtpError> {
        match self {
            Self::Unavailable => Ok(None),
            #[cfg(windows)]
            Self::Real => open_real(),
            #[cfg(any(test, feature = "fake-camera"))]
            Self::Simulated(config) => open_sim(config),
            #[cfg(any(test, feature = "fake-camera"))]
            Self::Custom(opener) => opener(),
        }
    }

    /// Nome con cui il sistema vede la fotocamera Fujifilm.
    fn find_device(&self) -> Result<Option<String>, PtpError> {
        match self {
            Self::Unavailable => Ok(None),
            #[cfg(windows)]
            Self::Real => Ok(crate::camera::wpd::list_devices()?
                .into_iter()
                .find(|d| d.is_fujifilm())
                .map(|d| d.friendly_name)),
            #[cfg(any(test, feature = "fake-camera"))]
            Self::Simulated(config) => {
                Ok((config.fault != SimFault::Absent).then(|| "X-T5 (simulata)".to_string()))
            }
            #[cfg(any(test, feature = "fake-camera"))]
            Self::Custom(_) => Ok(Some("X-T5 (simulata)".to_string())),
        }
    }

    fn conflicting_software(&self) -> Vec<String> {
        match self {
            #[cfg(windows)]
            Self::Real => running_conflicts(),
            _ => Vec::new(),
        }
    }
}

#[cfg(any(test, feature = "fake-camera"))]
fn frozen_error() -> PtpError {
    PtpError::Transport {
        code: 0x802A_0006u32 as i32,
        message: "IPortableDevice::Open: timeout (simulato)".into(),
    }
}

#[cfg(any(test, feature = "fake-camera"))]
fn open_sim(config: &SimConfig) -> Result<Option<Cam>, PtpError> {
    match config.fault {
        SimFault::Absent => Ok(None),
        SimFault::Frozen => {
            std::thread::sleep(config.freeze);
            Err(frozen_error())
        }
        _ => {
            let mut sim = SimulatedCamera::new();
            sim.conversion_delay = config.conversion_delay;
            if config.fault == SimFault::WrongMode {
                sim.usb_mode = 5;
                sim.advertise_conversion = false;
            }
            Ok(Some(FujiCamera::new(Box::new(sim))))
        }
    }
}

#[cfg(windows)]
fn open_real() -> Result<Option<Cam>, PtpError> {
    use crate::camera::wpd::{find_fujifilm, WpdTransport};
    let Some(device) = find_fujifilm()? else {
        return Ok(None);
    };
    let transport = WpdTransport::open(&device.id)?;
    Ok(Some(FujiCamera::new(Box::new(transport))))
}

// ---------------------------------------------------------------- host

/// Ciò che l'elenco dei controlli chiede al mondo esterno.
pub trait DiagnosticsHost {
    fn simulated(&self) -> bool;
    /// Nome con cui il sistema vede la fotocamera Fujifilm.
    fn find_device(&mut self) -> Result<Option<String>, PtpError>;
    fn conflicting_software(&mut self) -> Vec<String>;
    fn open(&mut self) -> Result<(), PtpError>;
    fn device_info(&mut self) -> Result<DeviceInfo, PtpError>;
    fn usb_mode(&mut self) -> Result<u32, PtpError>;
    fn bank_count(&mut self) -> Result<usize, PtpError>;
}

/// Host sopra una connessione (già aperta o no) del backend.
pub struct DiagHost {
    pub camera: Option<Cam>,
    pub info: Option<DeviceInfo>,
    backend: CameraBackend,
}

impl DiagHost {
    pub fn new(camera: Option<Cam>, backend: CameraBackend) -> Self {
        Self {
            camera,
            info: None,
            backend,
        }
    }

    /// La connessione viva, solo se ha risposto a `device_info` in questa verifica.
    pub fn into_connected(self) -> Option<(Cam, DeviceInfo)> {
        Some((self.camera?, self.info?))
    }

    fn camera(&mut self) -> Result<&mut Cam, PtpError> {
        self.camera.as_mut().ok_or(PtpError::Disconnected)
    }

    fn guard<T>(&mut self, result: Result<T, PtpError>) -> Result<T, PtpError> {
        if matches!(
            result,
            Err(PtpError::Disconnected | PtpError::Transport { .. } | PtpError::Timeout(_))
        ) {
            self.camera = None;
        }
        result
    }
}

impl DiagnosticsHost for DiagHost {
    fn simulated(&self) -> bool {
        self.backend.is_simulated()
    }

    fn find_device(&mut self) -> Result<Option<String>, PtpError> {
        self.backend.find_device()
    }

    fn conflicting_software(&mut self) -> Vec<String> {
        self.backend.conflicting_software()
    }

    fn open(&mut self) -> Result<(), PtpError> {
        if self.camera.is_some() {
            return Ok(());
        }
        match self.backend.open()? {
            Some(camera) => {
                self.camera = Some(camera);
                Ok(())
            }
            None => Err(PtpError::Disconnected),
        }
    }

    fn device_info(&mut self) -> Result<DeviceInfo, PtpError> {
        let result = self.camera().and_then(|c| c.device_info());
        let info = self.guard(result)?;
        self.info = Some(info.clone());
        Ok(info)
    }

    fn usb_mode(&mut self) -> Result<u32, PtpError> {
        let result = self.camera().and_then(|c| c.get_prop(prop::USB_MODE));
        let data = self.guard(result)?;
        le_value(&data).ok_or_else(|| PtpError::Protocol("valore modalità USB vuoto".into()))
    }

    fn bank_count(&mut self) -> Result<usize, PtpError> {
        let result = self.camera().and_then(|c| c.read_banks());
        Ok(self.guard(result)?.len())
    }
}

// ---------------------------------------------------------------- runner

/// Run the checklist. `update` receives a snapshot before the first step and
/// after every step, so a watcher can detect a hung step.
pub fn run_diagnostics(
    host: &mut dyn DiagnosticsHost,
    update: &mut dyn FnMut(&CameraDiagnostics),
) -> CameraDiagnostics {
    let mut d = CameraDiagnostics::new(host.simulated());
    update(&d);
    let mut go = true;

    macro_rules! put {
        ($id:expr, $status:expr, $detail:expr, $hint:expr) => {{
            d.set($id, $status, $detail, $hint);
            update(&d);
        }};
    }
    macro_rules! fail {
        ($id:expr, $err:expr, $elapsed:expr) => {{
            let (detail, hint, frozen) = describe_error(&$err, $elapsed);
            d.unresponsive |= frozen;
            put!($id, S::Error, Some(detail), Some(hint));
        }};
    }

    // (a) device visible to Windows
    let t = Instant::now();
    match host.find_device() {
        Ok(Some(name)) => put!("device", S::Ok, Some(name), None),
        Ok(None) => {
            go = false;
            put!(
                "device",
                S::Error,
                Some("Windows non vede nessuna camera Fujifilm".into()),
                Some("Collega la camera con un cavo USB dati, accendila e imposta USB RAW CONV./BACKUP RESTORE nel menu della camera. Prova un'altra porta USB se non compare.")
            );
        }
        Err(e) => {
            go = false;
            fail!("device", e, t.elapsed());
        }
    }

    // (b) conflicting software (never blocks)
    let conflicts = host.conflicting_software();
    if conflicts.is_empty() {
        put!("software", S::Ok, None, None);
    } else {
        put!(
            "software",
            S::Warning,
            Some(format!("In esecuzione: {}", conflicts.join(", "))),
            Some("Chiudi FUJIFILM X RAW Studio e le altre app Fujifilm: tengono occupata la camera e la conversione può fallire.")
        );
    }

    // (c) open
    if go {
        let t = Instant::now();
        match host.open() {
            Ok(()) => put!("connection", S::Ok, None, None),
            Err(e) => {
                go = false;
                fail!("connection", e, t.elapsed());
            }
        }
    }

    // (d) answers
    let mut info = None;
    if go {
        let t = Instant::now();
        match host.device_info() {
            Ok(i) => {
                d.model = Some(i.model.clone());
                d.firmware = Some(i.device_version.clone());
                let elapsed = t.elapsed();
                if elapsed >= Duration::from_secs(5) {
                    put!(
                        "response",
                        S::Warning,
                        Some(format!("risposta lenta ({} s)", elapsed.as_secs())),
                        Some("Se la camera è lenta a rispondere, spegnila e riaccendila prima di generare.")
                    );
                } else {
                    put!(
                        "response",
                        S::Ok,
                        Some(format!("{} ms", elapsed.as_millis())),
                        None
                    );
                }
                info = Some(i);
            }
            Err(e) => {
                go = false;
                fail!("response", e, t.elapsed());
            }
        }
    }

    // (e) model
    if let Some(i) = &info {
        if i.model.to_ascii_uppercase().contains("X-T5") {
            put!("model", S::Ok, Some(i.model.clone()), None);
        } else {
            put!(
                "model",
                S::Warning,
                Some(format!("{}: modello non testato", i.model)),
                Some("La conversione in camera è stata verificata solo con la X-T5. Su altri modelli i risultati possono variare.")
            );
        }
    }

    // (f) USB mode
    let mut mode_ok = go;
    if go {
        let t = Instant::now();
        match host.usb_mode() {
            Ok(6) => put!("usbMode", S::Ok, Some(usb_mode_label(6)), None),
            Ok(mode) => {
                mode_ok = false;
                put!(
                    "usbMode",
                    S::Error,
                    Some(format!("impostata: {}", usb_mode_label(mode))),
                    Some("Imposta la modalità USB su USB RAW CONV./BACKUP RESTORE (menu della camera, Impostazioni connessione > Impostazione USB; il percorso esatto dipende dal firmware), poi ricollega il cavo e premi Ricontrolla.")
                );
            }
            Err(e) => {
                let elapsed = t.elapsed();
                if looks_frozen(&e, elapsed) {
                    mode_ok = false;
                    fail!("usbMode", e, elapsed);
                } else {
                    put!(
                        "usbMode",
                        S::Warning,
                        Some(format!("modalità USB non leggibile ({e})")),
                        Some("Verifica nel menu della camera che la modalità USB sia USB RAW CONV./BACKUP RESTORE.")
                    );
                }
            }
        }
    }

    // (g) driver passes vendor opcodes, conversion properties present
    if mode_ok {
        if let Some(i) = &info {
            let missing = i.missing_conversion_support();
            if missing.is_empty() {
                put!(
                    "driver",
                    S::Ok,
                    Some("operazioni 0x900C/0x900D disponibili".into()),
                    None
                );
            } else {
                mode_ok = false;
                put!(
                    "driver",
                    S::Error,
                    Some(format!("mancano: {}", missing.join(", "))),
                    Some("La camera non espone la conversione RAW: controlla la modalità USB RAW CONV., ricollega il cavo e riprova. Se resta così, il driver Windows potrebbe non passare i comandi Fujifilm.")
                );
            }
        }
    }

    // (h) custom banks
    if mode_ok {
        let t = Instant::now();
        match host.bank_count() {
            Ok(0) => put!(
                "banks",
                S::Warning,
                Some("nessun preset personalizzato trovato".into()),
                Some("Potrai comunque generare la variante «Come scattato».")
            ),
            Ok(n) => put!("banks", S::Ok, Some(format!("{n} preset C1-C{n}")), None),
            Err(e) => fail!("banks", e, t.elapsed()),
        }
    }

    // Practical reminders (always shown).
    put!(
        "autoPowerOff",
        S::Info,
        None,
        Some("Imposta lo spegnimento automatico su «Off» nel menu della camera: se si spegne durante la generazione il lavoro va in pausa.")
    );
    put!(
        "power",
        S::Info,
        None,
        Some(
            "Tieni la batteria carica o alimenta la camera: la conversione di molti file è lunga."
        )
    );

    d.finish("Non eseguito: un controllo precedente è fallito.");
    update(&d);
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host(fault: SimFault) -> DiagHost {
        let config = SimConfig {
            fault,
            freeze: Duration::from_millis(1),
            ..SimConfig::default()
        };
        DiagHost::new(None, CameraBackend::Simulated(config))
    }

    fn status(d: &CameraDiagnostics, id: &str) -> CheckStatus {
        d.check(id).unwrap().status
    }

    #[test]
    fn simulated_camera_is_all_green() {
        let mut h = host(SimFault::None);
        let mut updates = 0;
        let d = run_diagnostics(&mut h, &mut |_| updates += 1);
        assert!(d.ready, "{d:?}");
        assert!(!d.unresponsive && !d.running);
        assert_eq!(d.model.as_deref(), Some("X-T5"));
        for id in [
            "device",
            "software",
            "connection",
            "response",
            "model",
            "usbMode",
            "driver",
            "banks",
        ] {
            assert_eq!(status(&d, id), S::Ok, "{id}");
        }
        assert_eq!(
            d.check("banks").unwrap().detail.as_deref(),
            Some("7 preset C1-C7")
        );
        assert_eq!(status(&d, "power"), S::Info);
        assert!(updates >= 10);
        assert!(h.into_connected().is_some());
    }

    #[test]
    fn absent_camera_blocks_everything_after_device() {
        let d = run_diagnostics(&mut host(SimFault::Absent), &mut |_| {});
        assert!(!d.ready);
        assert_eq!(status(&d, "device"), S::Error);
        assert_eq!(status(&d, "software"), S::Ok);
        for id in [
            "connection",
            "response",
            "model",
            "usbMode",
            "driver",
            "banks",
        ] {
            assert_eq!(status(&d, id), S::Skipped, "{id}");
        }
    }

    #[test]
    fn wrong_mode_is_reported_with_hint_and_skips_driver() {
        let d = run_diagnostics(&mut host(SimFault::WrongMode), &mut |_| {});
        assert!(!d.ready);
        assert_eq!(status(&d, "response"), S::Ok);
        let mode = d.check("usbMode").unwrap();
        assert_eq!(mode.status, S::Error);
        assert!(mode.detail.as_deref().unwrap().contains("Tether"));
        assert!(mode.hint.as_deref().unwrap().contains("USB RAW CONV"));
        assert_eq!(status(&d, "driver"), S::Skipped);
        assert_eq!(status(&d, "banks"), S::Skipped);
    }

    #[test]
    fn frozen_camera_is_flagged_unresponsive() {
        let d = run_diagnostics(&mut host(SimFault::Frozen), &mut |_| {});
        assert!(d.unresponsive && !d.ready);
        let c = d.check("connection").unwrap();
        assert_eq!(c.status, S::Error);
        assert!(c.hint.as_deref().unwrap().contains("riaccendi"));
        assert_eq!(status(&d, "response"), S::Skipped);
    }

    #[test]
    fn non_xt5_model_is_a_warning_only() {
        struct Other(DiagHost);
        impl DiagnosticsHost for Other {
            fn simulated(&self) -> bool {
                true
            }
            fn find_device(&mut self) -> Result<Option<String>, PtpError> {
                self.0.find_device()
            }
            fn conflicting_software(&mut self) -> Vec<String> {
                vec!["FUJIFILM X RAW STUDIO.exe".into()]
            }
            fn open(&mut self) -> Result<(), PtpError> {
                self.0.open()
            }
            fn device_info(&mut self) -> Result<DeviceInfo, PtpError> {
                let mut i = self.0.device_info()?;
                i.model = "X-H2".into();
                Ok(i)
            }
            fn usb_mode(&mut self) -> Result<u32, PtpError> {
                self.0.usb_mode()
            }
            fn bank_count(&mut self) -> Result<usize, PtpError> {
                self.0.bank_count()
            }
        }
        let d = run_diagnostics(&mut Other(host(SimFault::None)), &mut |_| {});
        assert!(d.ready);
        assert_eq!(status(&d, "model"), S::Warning);
        assert_eq!(status(&d, "software"), S::Warning);
    }

    #[test]
    fn partial_snapshot_becomes_unresponsive_at_first_pending_step() {
        let mut d = CameraDiagnostics::new(false);
        d.set("device", S::Ok, None, None);
        d.set("software", S::Ok, None, None);
        let d = d.into_unresponsive(Duration::from_secs(13));
        assert_eq!(status(&d, "connection"), S::Error);
        assert!(d
            .check("connection")
            .unwrap()
            .detail
            .as_deref()
            .unwrap()
            .contains("13 s"));
        assert_eq!(status(&d, "response"), S::Skipped);
        assert!(d.unresponsive && !d.ready && !d.running);
        let s = CameraDiagnostics::stuck(false, "status", Duration::from_secs(20));
        assert_eq!(status(&s, "response"), S::Error);
        assert!(s.unresponsive);
    }

    #[test]
    fn error_classification() {
        let frozen = PtpError::Transport {
            code: 0x802A_0006u32 as i32,
            message: "x".into(),
        };
        assert!(looks_frozen(&frozen, Duration::ZERO));
        assert!(looks_frozen(
            &PtpError::Disconnected,
            Duration::from_secs(60)
        ));
        assert!(!looks_frozen(
            &PtpError::Disconnected,
            Duration::from_secs(1)
        ));
        let (_, hint, f) = describe_error(&PtpError::Busy, Duration::ZERO);
        assert!(!f && hint.contains("X RAW Studio"));
    }

    #[test]
    fn parses_values_and_processes() {
        assert_eq!(le_value(&[6, 0]), Some(6));
        assert_eq!(le_value(&[8]), Some(8));
        assert_eq!(le_value(&[]), None);
        let out = "\"explorer.exe\",\"1234\",\"Console\",\"1\",\"10 K\"\n\"FUJIFILM X RAW STUDIO.exe\",\"99\",\"Console\",\"1\",\"1 K\"\n\"fujiview.exe\",\"5\",\"Console\",\"1\",\"1 K\"\n";
        assert_eq!(
            find_conflicts(out),
            vec!["FUJIFILM X RAW STUDIO.exe".to_string()]
        );
    }

    #[test]
    fn log_is_a_bounded_ring() {
        let log = CameraLog::new(None);
        for i in 0..(LOG_CAPACITY + 25) {
            log.info(format!("riga {i}"));
        }
        let lines = log.snapshot();
        assert_eq!(lines.len(), LOG_CAPACITY);
        assert_eq!(lines[0].message, "riga 25");
        assert_eq!(
            lines.last().unwrap().message,
            format!("riga {}", LOG_CAPACITY + 24)
        );
    }
}
