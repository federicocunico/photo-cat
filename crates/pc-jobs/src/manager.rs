//! Gestore dei job: registro dei tipi, coda con ammissione a budget,
//! esecuzione, pausa/ripresa/annullamento, snapshot periodici e ripresa
//! dei job interrotti.

use crate::control::JobControl;
use crate::persist::{self, JobState, StoreJournal};
use crate::pipeline::{Journal, RunSummary};
use crate::progress::{JobProgress, PhaseSnapshot, PhaseSpec};
use parking_lot::Mutex;
use pc_core::{AppError, AppResult, ErrorCode};
use pc_governor::MemoryGovernor;
use pc_store::Store;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Intervallo del ticker: limita gli aggiornamenti verso la UI a ~10/s per job.
pub const TICK: Duration = Duration::from_millis(100);
/// Ogni quanti tick si salvano i contatori nel DB.
const PERSIST_EVERY: u32 = 10;

/// Cosa serve per eseguire un job di un certo tipo.
pub struct JobPlan {
    pub phases: Vec<PhaseSpec>,
    /// Memoria minima richiesta per l'ammissione in coda.
    pub min_memory_mb: u64,
    pub run: Box<dyn FnOnce(JobContext) -> AppResult<RunSummary> + Send>,
}

pub type JobFactory = Arc<dyn Fn(&serde_json::Value) -> AppResult<JobPlan> + Send + Sync>;

/// Risorse a disposizione di un job in esecuzione.
pub struct JobContext {
    pub job_id: i64,
    pub params: serde_json::Value,
    pub progress: JobProgress,
    pub control: JobControl,
    pub journal: Arc<dyn Journal>,
    pub store: Store,
    pub governor: MemoryGovernor,
    /// Job da accodare quando questo termina senza errori fatali né annullamento
    /// (es. l'analisi AI rinviata a fine import).
    pub followup: Arc<Mutex<Option<(String, serde_json::Value)>>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct JobSnapshot {
    pub id: i64,
    pub kind: String,
    pub state: JobState,
    pub phases: Vec<PhaseSnapshot>,
    pub overall_percent: f64,
    pub eta_secs: Option<u64>,
    pub failed: i64,
    pub error: Option<String>,
}

pub trait ProgressSink: Send + Sync + 'static {
    fn on_progress(&self, snapshot: &JobSnapshot);
}

struct Entry {
    kind: String,
    state: JobState,
    progress: JobProgress,
    control: JobControl,
    min_memory_mb: u64,
    params: serde_json::Value,
    plan: Option<JobPlan>,
    error: Option<String>,
}

struct Shared {
    store: Store,
    governor: MemoryGovernor,
    sink: Arc<dyn ProgressSink>,
    factories: Mutex<BTreeMap<String, JobFactory>>,
    jobs: Mutex<BTreeMap<i64, Entry>>,
    queue: Mutex<VecDeque<i64>>,
    max_concurrent: usize,
    stop: AtomicBool,
    workers: Mutex<Vec<JoinHandle<()>>>,
}

#[derive(Clone)]
pub struct JobManager {
    shared: Arc<Shared>,
}

pub struct TickerHandle {
    shared: Arc<Shared>,
    handle: Option<JoinHandle<()>>,
}

impl Drop for TickerHandle {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

impl JobManager {
    pub fn new(
        store: Store,
        governor: MemoryGovernor,
        max_concurrent: usize,
        sink: Arc<dyn ProgressSink>,
    ) -> Self {
        Self {
            shared: Arc::new(Shared {
                store,
                governor,
                sink,
                factories: Mutex::new(BTreeMap::new()),
                jobs: Mutex::new(BTreeMap::new()),
                queue: Mutex::new(VecDeque::new()),
                max_concurrent: max_concurrent.max(1),
                stop: AtomicBool::new(false),
                workers: Mutex::new(Vec::new()),
            }),
        }
    }

    pub fn register(&self, kind: &str, factory: JobFactory) {
        self.shared
            .factories
            .lock()
            .insert(kind.to_string(), factory);
    }

    fn plan(&self, kind: &str, params: &serde_json::Value) -> AppResult<JobPlan> {
        let factory = self
            .shared
            .factories
            .lock()
            .get(kind)
            .cloned()
            .ok_or_else(|| AppError::invalid_input(format!("tipo di job sconosciuto: {kind}")))?;
        factory(params)
    }

    /// Accoda un nuovo job e restituisce il suo id.
    pub fn submit(&self, kind: &str, params: serde_json::Value) -> AppResult<i64> {
        let plan = self.plan(kind, &params)?;
        let id = persist::create(&self.shared.store, kind, &params, &plan.phases)?;
        tracing::info!(job_id = id, kind, "job accodato");
        self.enqueue(id, kind, params, plan);
        Ok(id)
    }

    /// Riprende un job interrotto: elabora solo gli elementi non ancora conclusi.
    pub fn resume_interrupted(&self, id: i64) -> AppResult<()> {
        let row = persist::get(&self.shared.store, id)?
            .ok_or_else(|| AppError::new(ErrorCode::NotFound, format!("job {id}")))?;
        if row.state != JobState::Interrupted {
            return Err(AppError::invalid_input(format!(
                "il job {id} non è interrotto"
            )));
        }
        let plan = self.plan(&row.kind, &row.params)?;
        persist::reset_phases(&self.shared.store, id, &plan.phases)?;
        persist::set_state(&self.shared.store, id, JobState::Queued)?;
        tracing::info!(
            job_id = id,
            kind = row.kind.as_str(),
            "ripresa di un job interrotto"
        );
        self.enqueue(id, &row.kind, row.params, plan);
        Ok(())
    }

    /// Job rimasti attivi nella sessione precedente, ora marcati "interrotti".
    pub fn recover_interrupted(&self) -> AppResult<Vec<persist::JobRow>> {
        Ok(persist::mark_interrupted(&self.shared.store)?)
    }

    fn enqueue(&self, id: i64, kind: &str, params: serde_json::Value, plan: JobPlan) {
        let entry = Entry {
            kind: kind.to_string(),
            state: JobState::Queued,
            progress: JobProgress::new(&plan.phases),
            control: JobControl::new(),
            min_memory_mb: plan.min_memory_mb,
            params,
            plan: Some(plan),
            error: None,
        };
        let snap = snapshot_of(id, &entry, Instant::now());
        self.shared.jobs.lock().insert(id, entry);
        self.shared.queue.lock().push_back(id);
        self.shared.sink.on_progress(&snap);
        self.try_start();
    }

    pub fn pause(&self, id: i64) -> AppResult<()> {
        self.transition(id, |e| match e.state {
            JobState::Running => {
                e.control.pause();
                Ok(JobState::Paused)
            }
            s => Err(AppError::invalid_input(format!(
                "impossibile mettere in pausa un job {}",
                s.as_str()
            ))),
        })
    }

    pub fn resume(&self, id: i64) -> AppResult<()> {
        self.transition(id, |e| match e.state {
            JobState::Paused => {
                e.control.resume();
                Ok(JobState::Running)
            }
            s => Err(AppError::invalid_input(format!(
                "impossibile riprendere un job {}",
                s.as_str()
            ))),
        })
    }

    pub fn cancel(&self, id: i64) -> AppResult<()> {
        let was_queued = self.shared.queue.lock().iter().any(|q| *q == id);
        if was_queued {
            self.shared.queue.lock().retain(|q| *q != id);
            self.transition(id, |e| {
                e.plan = None;
                Ok(JobState::Cancelled)
            })?;
            self.try_start();
            return Ok(());
        }
        let jobs = self.shared.jobs.lock();
        let e = jobs
            .get(&id)
            .ok_or_else(|| AppError::new(ErrorCode::NotFound, format!("job {id}")))?;
        if !e.state.is_terminal() {
            e.control.cancel();
            tracing::info!(job_id = id, "annullamento richiesto");
        }
        Ok(())
    }

    fn transition(
        &self,
        id: i64,
        f: impl FnOnce(&mut Entry) -> AppResult<JobState>,
    ) -> AppResult<()> {
        let snap = {
            let mut jobs = self.shared.jobs.lock();
            let e = jobs
                .get_mut(&id)
                .ok_or_else(|| AppError::new(ErrorCode::NotFound, format!("job {id}")))?;
            let next = f(e)?;
            e.state = next;
            persist::set_state(&self.shared.store, id, next)?;
            snapshot_of(id, e, Instant::now())
        };
        self.shared.sink.on_progress(&snap);
        Ok(())
    }

    pub fn snapshot(&self, id: i64) -> Option<JobSnapshot> {
        self.shared
            .jobs
            .lock()
            .get(&id)
            .map(|e| snapshot_of(id, e, Instant::now()))
    }

    /// Esiste un job non concluso di questo tipo i cui parametri soddisfano `pred`.
    pub fn has_active(&self, kind: &str, pred: impl Fn(&serde_json::Value) -> bool) -> bool {
        self.shared
            .jobs
            .lock()
            .values()
            .any(|e| e.kind == kind && !e.state.is_terminal() && pred(&e.params))
    }

    pub fn snapshots(&self) -> Vec<JobSnapshot> {
        let now = Instant::now();
        self.shared
            .jobs
            .lock()
            .iter()
            .map(|(id, e)| snapshot_of(*id, e, now))
            .collect()
    }

    /// Avvia i job in coda finché c'è posto e la loro memoria minima entra nel budget.
    fn try_start(&self) {
        loop {
            let (id, plan, ctx) = {
                let mut jobs = self.shared.jobs.lock();
                let mut queue = self.shared.queue.lock();
                let (active, reserved) = jobs
                    .values()
                    .filter(|e| matches!(e.state, JobState::Running | JobState::Paused))
                    .fold((0usize, 0u64), |(n, mb), e| (n + 1, mb + e.min_memory_mb));
                if active >= self.shared.max_concurrent {
                    return;
                }
                let Some(&id) = queue.front() else { return };
                let e = jobs.get_mut(&id).expect("job in coda registrato");
                let budget = self.shared.governor.snapshot().budget_mb;
                if active > 0 && reserved + e.min_memory_mb > budget {
                    return; // FIFO: attende che si liberi memoria
                }
                queue.pop_front();
                let plan = e.plan.take().expect("piano del job");
                let journal: Arc<dyn Journal> = match plan.phases.last() {
                    Some(last) => {
                        match StoreJournal::load(self.shared.store.clone(), id, &last.name) {
                            Ok(j) => Arc::new(j),
                            Err(err) => {
                                tracing::error!(code = "DATABASE", job_id = id, detail = %err, "journal non disponibile");
                                Arc::new(NoJournal)
                            }
                        }
                    }
                    None => Arc::new(NoJournal),
                };
                e.state = JobState::Running;
                let ctx = JobContext {
                    job_id: id,
                    params: e.params.clone(),
                    progress: e.progress.clone(),
                    control: e.control.clone(),
                    journal,
                    store: self.shared.store.clone(),
                    governor: self.shared.governor.clone(),
                    followup: Arc::default(),
                };
                (id, plan, ctx)
            };
            let _ = persist::set_state(&self.shared.store, id, JobState::Running);
            tracing::info!(job_id = id, "job avviato");
            let mgr = self.clone();
            let handle = std::thread::Builder::new()
                .name(format!("job-{id}"))
                .spawn(move || mgr.execute(id, plan, ctx))
                .expect("avvio thread del job");
            self.shared.workers.lock().push(handle);
        }
    }

    fn execute(&self, id: i64, plan: JobPlan, ctx: JobContext) {
        let started = Instant::now();
        let progress = ctx.progress.clone();
        let followup = ctx.followup.clone();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (plan.run)(ctx)))
            .unwrap_or_else(|_| Err(AppError::internal("panic nel job")));
        let (state, error) = match &result {
            Ok(RunSummary {
                cancelled: true, ..
            }) => (JobState::Cancelled, None),
            Ok(s) if s.failed > 0 => (JobState::CompletedWithErrors, None),
            Ok(_) => (JobState::Completed, None),
            Err(e) => {
                tracing::error!(
                    code = e.code.as_str(),
                    job_id = id,
                    detail = e.detail.as_str(),
                    "job fallito"
                );
                (JobState::Failed, Some(e.detail.clone()))
            }
        };
        let _ = persist::save_progress(&self.shared.store, id, &progress);
        let _ = persist::set_state(&self.shared.store, id, state);
        tracing::info!(
            job_id = id,
            state = state.as_str(),
            duration_ms = started.elapsed().as_millis() as u64,
            failed = progress.failed(),
            percent = progress.overall_percent(),
            "job terminato"
        );
        let snap = {
            let mut jobs = self.shared.jobs.lock();
            let e = jobs.get_mut(&id).expect("job registrato");
            e.state = state;
            e.error = error;
            snapshot_of(id, e, Instant::now())
        };
        self.shared.sink.on_progress(&snap);
        let next = followup.lock().take();
        if let (Some((kind, params)), JobState::Completed | JobState::CompletedWithErrors) =
            (next, state)
        {
            match self.submit(&kind, params) {
                Ok(next_id) => {
                    tracing::info!(job_id = id, next_job = next_id, kind = %kind, "job successivo accodato")
                }
                Err(e) => tracing::error!(
                    code = e.code.as_str(),
                    job_id = id,
                    detail = e.detail.as_str(),
                    "job successivo non accodato"
                ),
            }
        }
        self.try_start();
    }

    /// Avvia il ticker che pubblica gli snapshot dei job attivi e ne salva i contatori.
    pub fn start_ticker(&self) -> TickerHandle {
        let shared = self.shared.clone();
        let mgr = self.clone();
        let handle = std::thread::Builder::new()
            .name("pc-jobs-ticker".into())
            .spawn(move || {
                let mut n: u32 = 0;
                while !mgr.shared.stop.load(Ordering::Relaxed) {
                    std::thread::sleep(TICK);
                    n = n.wrapping_add(1);
                    let now = Instant::now();
                    let active: Vec<(i64, JobSnapshot, JobProgress)> = mgr
                        .shared
                        .jobs
                        .lock()
                        .iter()
                        .filter(|(_, e)| matches!(e.state, JobState::Running | JobState::Paused))
                        .map(|(id, e)| (*id, snapshot_of(*id, e, now), e.progress.clone()))
                        .collect();
                    for (id, snap, progress) in active {
                        mgr.shared.sink.on_progress(&snap);
                        if n.is_multiple_of(PERSIST_EVERY) {
                            let _ = persist::save_progress(&mgr.shared.store, id, &progress);
                        }
                    }
                    mgr.try_start();
                }
            })
            .expect("avvio ticker");
        TickerHandle {
            shared,
            handle: Some(handle),
        }
    }

    /// Attende la fine di tutti i job avviati finora (test e chiusura ordinata).
    pub fn wait_idle(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            let busy = self
                .shared
                .jobs
                .lock()
                .values()
                .any(|e| !e.state.is_terminal());
            if !busy {
                let handles: Vec<_> = self.shared.workers.lock().drain(..).collect();
                for h in handles {
                    let _ = h.join();
                }
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

struct NoJournal;
impl Journal for NoJournal {
    fn is_finished(&self, _key: &str) -> bool {
        false
    }
    fn record(&self, _key: &str, _phase: &str, _state: crate::pipeline::ItemState) {}
}

fn snapshot_of(id: i64, e: &Entry, now: Instant) -> JobSnapshot {
    let (phases, overall, eta) = e.progress.sample(now);
    JobSnapshot {
        id,
        kind: e.kind.clone(),
        state: e.state,
        phases,
        overall_percent: overall,
        eta_secs: if e.state == JobState::Running {
            eta
        } else {
            None
        },
        failed: e.progress.failed(),
        error: e.error.clone(),
    }
}
