//! Eventi verso la UI. I nomi sono stabili e condivisi con `app/ui/src/lib/ipc.ts`.

use parking_lot::Mutex;
use pc_diag::{DiagSink, ErrorRecord, LogLine};
use pc_jobs::manager::{JobSnapshot, ProgressSink};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

pub const JOB_PROGRESS: &str = "job://progress";
pub const RESOURCES_TICK: &str = "resources://tick";
/// Righe di log raggruppate (al massimo ogni `LOG_FLUSH`).
pub const LOG_LINES: &str = "log://lines";
/// Errori per file raggruppati (al massimo ogni `pc_diag::batch::BATCH_INTERVAL`).
pub const ERROR_BATCH: &str = "errors://batch";
/// Volti o persone cambiati (fine delle fasi volti/raggruppamento, correzioni): l'UI
/// aggiorna le liste su questo evento invece di interrogare a intervalli.
pub const FACES_CHANGED: &str = "faces://changed";

pub fn faces_changed(app: &AppHandle) {
    let _ = app.emit(FACES_CHANGED, ());
}

/// Fasi dopo le quali volti e persone sono cambiati.
const FACE_PHASES: &[&str] = &["faces", "cluster"];

/// Riconosce le fasi dei volti appena concluse e i job che modificano le persone.
#[derive(Default)]
pub struct FacesWatch {
    done: Mutex<std::collections::HashSet<(i64, String)>>,
}

impl FacesWatch {
    /// `true` se dopo questo snapshot volti o persone sono cambiati.
    pub fn changed(&self, s: &JobSnapshot) -> bool {
        let mut done = self.done.lock();
        let mut changed = false;
        for p in &s.phases {
            if FACE_PHASES.contains(&p.name.as_str())
                && p.state == pc_jobs::PhaseState::Done
                && done.insert((s.id, p.name.clone()))
            {
                changed = true;
            }
        }
        // Assegnazioni di persone in bulk.
        if s.kind == pc_jobs::bulk::JOB_KIND
            && s.state.is_terminal()
            && done.insert((s.id, String::new()))
        {
            changed = true;
        }
        changed
    }
}

const LOG_FLUSH: Duration = Duration::from_millis(300);
/// Righe inviate per raffica: oltre, si tengono le più recenti (il file di log ha tutto).
const LOG_BATCH_MAX: usize = 200;

/// Intervallo minimo tra due eventi di progresso dello stesso job (i cambi di stato passano subito).
pub const PROGRESS_EVERY: Duration = Duration::from_millis(250);

/// Ultimo invio per job: istante e stato.
type LastSent = HashMap<i64, (Instant, pc_jobs::JobState)>;

/// Limita gli snapshot di progresso a uno ogni [`PROGRESS_EVERY`] per job; l'ultimo snapshot
/// trattenuto viene inviato dal thread di svuotamento, così lo stato finale arriva sempre.
#[derive(Default)]
pub struct ProgressThrottle {
    inner: Mutex<(LastSent, HashMap<i64, JobSnapshot>)>,
}

impl ProgressThrottle {
    /// `true` se lo snapshot va inviato subito; altrimenti resta in attesa.
    pub fn offer(&self, s: &JobSnapshot, now: Instant) -> bool {
        let mut g = self.inner.lock();
        let due = match g.0.get(&s.id) {
            Some((at, state)) => *state != s.state || now.duration_since(*at) >= PROGRESS_EVERY,
            None => true,
        };
        if due {
            g.0.insert(s.id, (now, s.state));
            g.1.remove(&s.id);
        } else {
            g.1.insert(s.id, s.clone());
        }
        due
    }

    /// Snapshot trattenuti il cui intervallo è trascorso.
    pub fn due(&self, now: Instant) -> Vec<JobSnapshot> {
        let mut g = self.inner.lock();
        let ready: Vec<i64> =
            g.1.keys()
                .copied()
                .filter(|id| {
                    g.0.get(id)
                        .is_none_or(|(at, _)| now.duration_since(*at) >= PROGRESS_EVERY)
                })
                .collect();
        let mut out = Vec::new();
        for id in ready {
            if let Some(s) = g.1.remove(&id) {
                g.0.insert(id, (now, s.state));
                out.push(s);
            }
        }
        out
    }
}

/// Inoltra gli snapshot dei job alla UI, limitati da [`ProgressThrottle`].
pub struct TauriProgressSink {
    app: AppHandle,
    throttle: Arc<ProgressThrottle>,
    faces: FacesWatch,
}

impl TauriProgressSink {
    pub fn new(app: AppHandle) -> Self {
        let throttle = Arc::new(ProgressThrottle::default());
        let (t2, a2) = (Arc::downgrade(&throttle), app.clone());
        std::thread::Builder::new()
            .name("pc-job-events".into())
            .spawn(move || loop {
                std::thread::sleep(PROGRESS_EVERY);
                let Some(t) = t2.upgrade() else {
                    return;
                };
                for s in t.due(Instant::now()) {
                    let _ = a2.emit(JOB_PROGRESS, &s);
                }
            })
            .expect("thread eventi dei job");
        Self {
            app,
            throttle,
            faces: FacesWatch::default(),
        }
    }
}

impl ProgressSink for TauriProgressSink {
    fn on_progress(&self, snapshot: &JobSnapshot) {
        if self.throttle.offer(snapshot, Instant::now()) {
            let _ = self.app.emit(JOB_PROGRESS, snapshot);
        }
        if self.faces.changed(snapshot) {
            faces_changed(&self.app);
        }
    }
}

/// Inoltra errori e righe di log alla UI, a gruppi: un flusso di log
/// molto intenso non deve saturare l'IPC né rallentare l'interfaccia.
pub struct TauriDiagSink {
    errors: Arc<pc_diag::ErrorAggregator>,
    pending: Arc<Mutex<Vec<LogLine>>>,
}

impl TauriDiagSink {
    pub fn new(app: AppHandle) -> Self {
        let pending: Arc<Mutex<Vec<LogLine>>> = Arc::default();
        let errors = Arc::new(pc_diag::ErrorAggregator::default());
        let a3 = app.clone();
        errors.spawn(pc_diag::batch::BATCH_INTERVAL, move |b| {
            let _ = a3.emit(ERROR_BATCH, &b);
        });
        let (p2, a2) = (pending.clone(), app.clone());
        std::thread::Builder::new()
            .name("pc-log-events".into())
            .spawn(move || loop {
                std::thread::sleep(LOG_FLUSH);
                let mut batch = std::mem::take(&mut *p2.lock());
                if batch.is_empty() {
                    continue;
                }
                if batch.len() > LOG_BATCH_MAX {
                    batch.drain(..batch.len() - LOG_BATCH_MAX);
                }
                let _ = a2.emit(LOG_LINES, &batch);
            })
            .expect("thread eventi di log");
        Self { errors, pending }
    }
}

impl DiagSink for TauriDiagSink {
    fn on_error(&self, record: &ErrorRecord) {
        self.errors.push(record);
    }
    fn on_log(&self, line: &LogLine) {
        // Solo INFO e superiori: il debug resta nei file.
        if matches!(line.level.as_str(), "INFO" | "WARN" | "ERROR") {
            let mut p = self.pending.lock();
            if p.len() < LOG_BATCH_MAX * 4 {
                p.push(line.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pc_jobs::JobState;

    fn snap(id: i64, state: JobState, pct: f64) -> JobSnapshot {
        JobSnapshot {
            id,
            kind: "import".into(),
            state,
            phases: vec![],
            overall_percent: pct,
            eta_secs: None,
            failed: 0,
            error: None,
            memory_paused: false,
        }
    }

    #[test]
    fn progress_is_limited_per_job_but_state_changes_and_last_value_arrive() {
        let t = ProgressThrottle::default();
        let t0 = Instant::now();
        let ms = |n| t0 + Duration::from_millis(n);
        assert!(t.offer(&snap(1, JobState::Running, 1.0), ms(0)));
        // Ogni 100 ms per un secondo: passano solo quelli a distanza di 250 ms.
        let sent = (1..=10)
            .filter(|i| t.offer(&snap(1, JobState::Running, *i as f64), ms(i * 100)))
            .count();
        assert!(sent <= 4, "{sent}");
        // Un altro job non è limitato dal primo.
        assert!(t.offer(&snap(2, JobState::Running, 1.0), ms(1010)));
        // Il cambio di stato passa subito.
        assert!(t.offer(&snap(1, JobState::Paused, 50.0), ms(1020)));
        assert!(!t.offer(&snap(1, JobState::Paused, 51.0), ms(1030)));
        assert!(t.due(ms(1100)).is_empty(), "non ancora trascorso");
        let late = t.due(ms(1300));
        assert_eq!(late.len(), 1);
        assert_eq!(
            late[0].overall_percent, 51.0,
            "l'ultimo valore arriva comunque"
        );
    }
}

#[cfg(test)]
mod faces_tests {
    use super::*;
    use pc_jobs::{JobState, PhaseSnapshot, PhaseState};

    fn snap(state: JobState, faces: PhaseState, cluster: PhaseState) -> JobSnapshot {
        let phase = |name: &str, state| PhaseSnapshot {
            name: name.into(),
            state,
            done: 0,
            failed: 0,
            total: 0,
            total_known: true,
            percent: None,
            rate: None,
            eta_secs: None,
            memory_paused: false,
        };
        JobSnapshot {
            id: 3,
            kind: "analysis".into(),
            state,
            phases: vec![phase("faces", faces), phase("cluster", cluster)],
            overall_percent: 0.0,
            eta_secs: None,
            failed: 0,
            error: None,
            memory_paused: false,
        }
    }

    #[test]
    fn faces_changed_once_per_phase_end() {
        use PhaseState::*;
        let w = FacesWatch::default();
        assert!(!w.changed(&snap(JobState::Running, Running, Pending)));
        assert!(
            w.changed(&snap(JobState::Running, Done, Pending)),
            "fine fase volti"
        );
        assert!(
            !w.changed(&snap(JobState::Running, Done, Running)),
            "solo una volta"
        );
        assert!(
            w.changed(&snap(JobState::Completed, Done, Done)),
            "fine raggruppamento"
        );
        assert!(!w.changed(&snap(JobState::Completed, Done, Done)));
    }
}
