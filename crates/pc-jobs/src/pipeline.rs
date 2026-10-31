//! Pipeline a stadi: una sorgente emette elementi che attraversano in ordine
//! gli stadi, ciascuno con i propri worker, collegati da canali limitati
//! (back-pressure). Un elemento può trovarsi in uno stadio mentre altri sono
//! in stadi diversi.
//!
//! Ogni stadio restituisce `Continue`, `Skip` (elemento concluso senza passare
//! agli stadi successivi) o `Fail` (errore registrato, il job prosegue).

use crate::control::JobControl;
use crate::progress::{JobProgress, PhaseCounters};
use crossbeam_channel::{bounded, Receiver, Sender};
use pc_core::AppError;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub enum StageOutcome {
    Continue,
    Skip,
    Fail(AppError),
    /// Errore registrato e contato come fallito in questa fase, ma l'elemento
    /// prosegue (es. miniatura non decodificabile: il file entra comunque in libreria).
    ContinueWithError(AppError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemState {
    Done,
    Failed,
}

impl ItemState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Done => "done",
            Self::Failed => "failed",
        }
    }
}

/// Registro per elemento e fase, usato per riprendere un job interrotto.
pub trait Journal: Send + Sync {
    /// L'elemento è già concluso (completato o fallito) in un'esecuzione precedente.
    fn is_finished(&self, key: &str) -> bool;
    fn record(&self, key: &str, phase: &str, state: ItemState);
}

pub type KeyFn<T> = Arc<dyn Fn(&T) -> String + Send + Sync>;
type StageFn<T> = Arc<dyn Fn(&mut T, &StageCtx) -> StageOutcome + Send + Sync>;
type SourceFn<T> = Box<dyn FnOnce(&Emitter<T>) -> Result<(), AppError> + Send>;

/// Contesto passato a ogni invocazione di uno stadio.
pub struct StageCtx {
    pub job_id: i64,
    pub phase: Arc<PhaseCounters>,
    pub control: JobControl,
}

struct Stage<T> {
    name: String,
    workers: usize,
    f: StageFn<T>,
}

/// Lato sorgente: emette elementi verso il primo stadio.
pub struct Emitter<T> {
    tx: Option<Sender<T>>,
    phase: Arc<PhaseCounters>,
    downstream: Vec<Arc<PhaseCounters>>,
    control: JobControl,
    journal: Option<Arc<dyn Journal>>,
    key: Option<KeyFn<T>>,
}

impl<T> Emitter<T> {
    /// Emette un elemento; `false` se il job è stato annullato.
    pub fn emit(&self, item: T) -> bool {
        if !self.control.checkpoint() {
            return false;
        }
        self.phase.add_total(1);
        self.phase.inc_done(1);
        for d in &self.downstream {
            d.add_total(1);
        }
        if let (Some(j), Some(k)) = (&self.journal, &self.key) {
            if j.is_finished(&k(&item)) {
                for d in &self.downstream {
                    d.inc_done(1);
                }
                return true;
            }
        }
        match &self.tx {
            Some(tx) => tx.send(item).is_ok(),
            None => true,
        }
    }

    pub fn phase(&self) -> &PhaseCounters {
        &self.phase
    }

    pub fn control(&self) -> &JobControl {
        &self.control
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunSummary {
    pub cancelled: bool,
    pub failed: i64,
}

pub struct Pipeline<T> {
    job_id: i64,
    progress: JobProgress,
    control: JobControl,
    source_phase: String,
    source: Option<SourceFn<T>>,
    stages: Vec<Stage<T>>,
    channel_capacity: usize,
    journal: Option<Arc<dyn Journal>>,
    key: Option<KeyFn<T>>,
}

impl<T: Send + 'static> Pipeline<T> {
    pub fn new(job_id: i64, progress: JobProgress, control: JobControl) -> Self {
        Self {
            job_id,
            progress,
            control,
            source_phase: String::new(),
            source: None,
            stages: Vec::new(),
            channel_capacity: 64,
            journal: None,
            key: None,
        }
    }

    pub fn source(
        mut self,
        phase: &str,
        f: impl FnOnce(&Emitter<T>) -> Result<(), AppError> + Send + 'static,
    ) -> Self {
        self.source_phase = phase.into();
        self.source = Some(Box::new(f));
        self
    }

    pub fn stage(
        mut self,
        phase: &str,
        workers: usize,
        f: impl Fn(&mut T, &StageCtx) -> StageOutcome + Send + Sync + 'static,
    ) -> Self {
        self.stages.push(Stage {
            name: phase.into(),
            workers: workers.max(1),
            f: Arc::new(f),
        });
        self
    }

    pub fn channel_capacity(mut self, n: usize) -> Self {
        self.channel_capacity = n.max(1);
        self
    }

    /// Chiave dell'elemento (es. percorso) per journal e messaggi d'errore.
    pub fn key(mut self, f: impl Fn(&T) -> String + Send + Sync + 'static) -> Self {
        self.key = Some(Arc::new(f));
        self
    }

    pub fn journal(mut self, j: Arc<dyn Journal>) -> Self {
        self.journal = Some(j);
        self
    }

    /// Esegue la pipeline fino al termine (o all'annullamento).
    pub fn run(mut self) -> Result<RunSummary, AppError> {
        let source = self
            .source
            .take()
            .ok_or_else(|| AppError::internal("pipeline senza sorgente"))?;
        let source_phase = self.progress.phase(&self.source_phase);
        let stage_phases: Vec<_> = self
            .stages
            .iter()
            .map(|s| self.progress.phase(&s.name))
            .collect();

        // Canali: sorgente → stadio 0 → stadio 1 → ...
        let mut senders: Vec<Sender<T>> = Vec::new();
        let mut receivers: Vec<Receiver<T>> = Vec::new();
        for _ in 0..self.stages.len() {
            let (tx, rx) = bounded::<T>(self.channel_capacity);
            senders.push(tx);
            receivers.push(rx);
        }

        let mut handles = Vec::new();
        for (i, stage) in self.stages.iter().enumerate() {
            let rx = receivers[i].clone();
            let next: Option<Sender<T>> = senders.get(i + 1).cloned();
            let phase = stage_phases[i].clone();
            let downstream: Arc<Vec<_>> = Arc::new(stage_phases[i + 1..].to_vec());
            let alive = Arc::new(AtomicUsize::new(stage.workers));
            for w in 0..stage.workers {
                let rx = rx.clone();
                let next = next.clone();
                let ctx = StageCtx {
                    job_id: self.job_id,
                    phase: phase.clone(),
                    control: self.control.clone(),
                };
                let f = stage.f.clone();
                let downstream = downstream.clone();
                let alive = alive.clone();
                let journal = self.journal.clone();
                let key = self.key.clone();
                let name = stage.name.clone();
                let job_id = self.job_id;
                let last = i + 1 == self.stages.len();
                let handle = std::thread::Builder::new()
                    .name(format!("job{job_id}-{name}-{w}"))
                    .spawn(move || {
                        while let Ok(mut item) = rx.recv() {
                            if !ctx.control.checkpoint() {
                                break;
                            }
                            ctx.phase.start();
                            let outcome = catch_unwind(AssertUnwindSafe(|| f(&mut item, &ctx)))
                                .unwrap_or_else(|p| {
                                    let msg = p
                                        .downcast_ref::<&str>()
                                        .map(|s| s.to_string())
                                        .or_else(|| p.downcast_ref::<String>().cloned())
                                        .unwrap_or_else(|| "panic".into());
                                    StageOutcome::Fail(AppError::internal(format!(
                                        "panic nello stadio: {msg}"
                                    )))
                                });
                            let k = key.as_ref().map(|k| k(&item));
                            let outcome = match outcome {
                                StageOutcome::ContinueWithError(e) => {
                                    ctx.phase.inc_failed(1);
                                    log_item_error(job_id, &name, k.as_deref(), &e);
                                    // Prosegue come Continue senza contare due volte la fase.
                                    ctx.phase.inc_done(-1);
                                    StageOutcome::Continue
                                }
                                o => o,
                            };
                            match outcome {
                                StageOutcome::Continue => {
                                    ctx.phase.inc_done(1);
                                    if let (Some(j), Some(k)) = (&journal, &k) {
                                        j.record(k, &name, ItemState::Done);
                                    }
                                    if let Some(next) = &next {
                                        if next.send(item).is_err() {
                                            break;
                                        }
                                    } else if last {
                                        drop(item);
                                    }
                                }
                                StageOutcome::Skip => {
                                    ctx.phase.inc_done(1);
                                    for d in downstream.iter() {
                                        d.add_total(-1);
                                    }
                                    if let (Some(j), Some(k)) = (&journal, &k) {
                                        j.record(k, &name, ItemState::Done);
                                        if !last {
                                            j.record(k, "*", ItemState::Done);
                                        }
                                    }
                                }
                                StageOutcome::Fail(e) => {
                                    ctx.phase.inc_failed(1);
                                    for d in downstream.iter() {
                                        d.add_total(-1);
                                    }
                                    log_item_error(job_id, &name, k.as_deref(), &e);
                                    if let (Some(j), Some(k)) = (&journal, &k) {
                                        j.record(k, &name, ItemState::Failed);
                                    }
                                }
                                StageOutcome::ContinueWithError(_) => {
                                    unreachable!("convertito sopra")
                                }
                            }
                        }
                        // L'ultimo worker chiude la fase (solo se non annullata).
                        if alive.fetch_sub(1, Ordering::AcqRel) == 1 && !ctx.control.is_cancelled()
                        {
                            ctx.phase.finish();
                            log_phase_end(job_id, &ctx.phase);
                        }
                        drop(next);
                    })
                    .map_err(|e| AppError::internal(e.to_string()))?;
                handles.push(handle);
            }
        }
        drop(receivers);
        let first_tx = if senders.is_empty() {
            None
        } else {
            Some(senders.remove(0))
        };
        drop(senders); // gli stadi successivi tengono i propri sender

        source_phase.start();
        log_phase_start(self.job_id, &source_phase);
        for p in &stage_phases {
            log_phase_start(self.job_id, p);
        }
        let emitter = Emitter {
            tx: first_tx,
            phase: source_phase.clone(),
            downstream: stage_phases.clone(),
            control: self.control.clone(),
            journal: self.journal.clone(),
            key: self.key.clone(),
        };
        let source_result = catch_unwind(AssertUnwindSafe(|| source(&emitter)))
            .unwrap_or_else(|_| Err(AppError::internal("panic nella sorgente")));
        let cancelled_during_source = self.control.is_cancelled();
        if !cancelled_during_source {
            source_phase.finish();
            log_phase_end(self.job_id, &source_phase);
            for p in &stage_phases {
                p.mark_total_known();
            }
        }
        drop(emitter); // chiude il primo canale: gli stadi si svuotano e terminano

        for h in handles {
            let _ = h.join();
        }
        source_result?;
        Ok(RunSummary {
            cancelled: self.control.is_cancelled(),
            failed: self.progress.failed(),
        })
    }
}

fn log_item_error(job_id: i64, phase: &str, path: Option<&str>, e: &AppError) {
    tracing::error!(
        code = e.code.as_str(),
        job_id,
        phase,
        path = path.unwrap_or(""),
        detail = e.detail.as_str(),
        "{}",
        e.message_key
    );
}

fn log_phase_start(job_id: i64, phase: &PhaseCounters) {
    tracing::info!(job_id, phase = phase.name(), "fase avviata");
}

fn log_phase_end(job_id: i64, phase: &PhaseCounters) {
    let percent = phase
        .fraction()
        .map(|f| (f * 1000.0).round() / 10.0)
        .unwrap_or(0.0);
    tracing::info!(
        job_id,
        phase = phase.name(),
        duration_ms = phase.duration().map(|d| d.as_millis() as u64).unwrap_or(0),
        done = phase.done(),
        failed = phase.failed(),
        total = phase.total(),
        percent,
        "fase completata"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::PhaseSpec;
    use parking_lot::Mutex;
    use pc_core::ErrorCode;
    use std::collections::HashSet;
    use std::time::Duration;

    fn specs() -> Vec<PhaseSpec> {
        ["scan", "meta", "ai", "write"]
            .iter()
            .map(|n| PhaseSpec::new(n, 1.0))
            .collect()
    }

    #[test]
    fn processes_10k_items_through_all_stages() {
        let progress = JobProgress::new(&specs());
        let written = Arc::new(Mutex::new(Vec::new()));
        let w2 = written.clone();
        let summary = Pipeline::<u64>::new(1, progress.clone(), JobControl::new())
            .source("scan", |e| {
                for i in 0..10_000 {
                    if !e.emit(i) {
                        break;
                    }
                }
                Ok(())
            })
            .stage("meta", 4, |x, _| {
                *x *= 2;
                StageOutcome::Continue
            })
            .stage("ai", 3, |_, _| StageOutcome::Continue)
            .stage("write", 1, move |x, _| {
                w2.lock().push(*x);
                StageOutcome::Continue
            })
            .run()
            .unwrap();
        assert_eq!(
            summary,
            RunSummary {
                cancelled: false,
                failed: 0
            }
        );
        let mut v = written.lock().clone();
        v.sort_unstable();
        assert_eq!(v.len(), 10_000);
        assert_eq!(v[9_999], 19_998);
        assert_eq!(progress.overall_percent(), 100.0);
        for p in progress.phases() {
            assert_eq!(p.done(), 10_000, "{}", p.name());
            assert_eq!(p.total(), 10_000, "{}", p.name());
        }
    }

    #[test]
    fn next_stage_starts_before_the_source_has_finished() {
        // La sorgente emette il primo elemento e poi attende un segnale dallo stadio
        // successivo: termina solo se le fasi procedono sovrapposte.
        let progress = JobProgress::new(&specs());
        let (tx, rx) = std::sync::mpsc::channel::<u64>();
        let tx = Mutex::new(Some(tx));
        let overlapped = Arc::new(Mutex::new(None));
        let seen = overlapped.clone();
        let summary = Pipeline::<u64>::new(1, progress.clone(), JobControl::new())
            .source("scan", move |e| {
                e.emit(0);
                *seen.lock() = Some(rx.recv_timeout(Duration::from_secs(10)).is_ok());
                for i in 1..100 {
                    e.emit(i);
                }
                Ok(())
            })
            .stage("meta", 1, move |x, _| {
                if *x == 0 {
                    if let Some(tx) = tx.lock().take() {
                        tx.send(*x).unwrap();
                    }
                }
                StageOutcome::Continue
            })
            .stage("ai", 1, |_, _| StageOutcome::Continue)
            .stage("write", 1, |_, _| StageOutcome::Continue)
            .run()
            .unwrap();
        assert_eq!(
            *overlapped.lock(),
            Some(true),
            "lo stadio \"meta\" non ha elaborato il primo elemento mentre la scansione era in corso"
        );
        assert!(!summary.cancelled);
        assert_eq!(progress.phase("write").done(), 100);
    }

    #[test]
    fn failures_and_skips_are_counted_and_do_not_stop_the_job() {
        let progress = JobProgress::new(&specs());
        let summary = Pipeline::<u64>::new(2, progress.clone(), JobControl::new())
            .key(|x| format!("file-{x}"))
            .source("scan", |e| {
                (0..1000).for_each(|i| {
                    e.emit(i);
                });
                Ok(())
            })
            .stage("meta", 2, |x, _| match *x {
                7 => StageOutcome::Fail(AppError::new(ErrorCode::AccessDenied, "accesso negato")),
                x if x % 100 == 0 => StageOutcome::Skip,
                _ => StageOutcome::Continue,
            })
            .stage("ai", 2, |x, _| {
                if *x == 501 {
                    panic!("modello esploso");
                }
                StageOutcome::Continue
            })
            .stage("write", 1, |_, _| StageOutcome::Continue)
            .run()
            .unwrap();
        assert_eq!(summary.failed, 2);
        let meta = progress.phase("meta");
        assert_eq!((meta.done(), meta.failed(), meta.total()), (999, 1, 1000));
        let ai = progress.phase("ai");
        // 1000 − 1 fallito − 10 saltati = 989 elementi arrivati all'AI.
        assert_eq!((ai.done(), ai.failed(), ai.total()), (988, 1, 989));
        let write = progress.phase("write");
        assert_eq!((write.done(), write.total()), (988, 988));
        assert_eq!(progress.overall_percent(), 100.0);
    }

    #[test]
    fn soft_errors_count_as_failed_but_item_continues() {
        let progress = JobProgress::new(&specs());
        let written = Arc::new(AtomicUsize::new(0));
        let w2 = written.clone();
        let summary = Pipeline::<u64>::new(5, progress.clone(), JobControl::new())
            .source("scan", |e| {
                (0..100).for_each(|i| {
                    e.emit(i);
                });
                Ok(())
            })
            .stage("meta", 2, |_, _| StageOutcome::Continue)
            .stage("ai", 2, |x, _| {
                if *x % 10 == 0 {
                    StageOutcome::ContinueWithError(AppError::new(
                        ErrorCode::DecodeFailed,
                        "miniatura",
                    ))
                } else {
                    StageOutcome::Continue
                }
            })
            .stage("write", 1, move |_, _| {
                w2.fetch_add(1, Ordering::SeqCst);
                StageOutcome::Continue
            })
            .run()
            .unwrap();
        assert_eq!(written.load(Ordering::SeqCst), 100, "tutti scritti");
        let ai = progress.phase("ai");
        assert_eq!((ai.done(), ai.failed(), ai.total()), (90, 10, 100));
        assert_eq!(summary.failed, 10);
        assert_eq!(progress.overall_percent(), 100.0);
    }

    #[test]
    fn cancel_stops_promptly_and_keeps_completed_results() {
        let progress = JobProgress::new(&specs());
        let control = JobControl::new();
        let c2 = control.clone();
        let written = Arc::new(Mutex::new(HashSet::new()));
        let w2 = written.clone();
        let summary = Pipeline::<u64>::new(3, progress.clone(), control)
            .source("scan", move |e| {
                for i in 0..100_000 {
                    if i == 200 {
                        c2.cancel();
                    }
                    if !e.emit(i) {
                        break;
                    }
                }
                Ok(())
            })
            .stage("meta", 2, |_, _| {
                std::thread::sleep(Duration::from_micros(50));
                StageOutcome::Continue
            })
            .stage("ai", 1, |_, _| StageOutcome::Continue)
            .stage("write", 1, move |x, _| {
                w2.lock().insert(*x);
                StageOutcome::Continue
            })
            .run()
            .unwrap();
        assert!(summary.cancelled);
        let written = written.lock();
        assert!(written.len() <= 200 && written.len() < 100_000);
        // Le fasi non risultano completate dopo un annullamento.
        assert!(progress.phase("write").fraction().unwrap_or(0.0) < 1.0 || written.len() == 200);
    }

    #[test]
    fn journal_skips_finished_items_on_resume() {
        #[derive(Default)]
        struct MemJournal {
            finished: HashSet<String>,
            records: Mutex<Vec<(String, String, ItemState)>>,
        }
        impl Journal for MemJournal {
            fn is_finished(&self, key: &str) -> bool {
                self.finished.contains(key)
            }
            fn record(&self, key: &str, phase: &str, state: ItemState) {
                self.records.lock().push((key.into(), phase.into(), state));
            }
        }
        let journal = Arc::new(MemJournal {
            finished: (0..60).map(|i| i.to_string()).collect(),
            ..Default::default()
        });
        let processed = Arc::new(AtomicUsize::new(0));
        let p2 = processed.clone();
        let progress = JobProgress::new(&specs());
        Pipeline::<u64>::new(4, progress.clone(), JobControl::new())
            .key(|x| x.to_string())
            .journal(journal.clone())
            .source("scan", |e| {
                (0..100).for_each(|i| {
                    e.emit(i);
                });
                Ok(())
            })
            .stage("meta", 2, move |_, _| {
                p2.fetch_add(1, Ordering::SeqCst);
                StageOutcome::Continue
            })
            .stage("ai", 1, |_, _| StageOutcome::Continue)
            .stage("write", 1, |_, _| StageOutcome::Continue)
            .run()
            .unwrap();
        assert_eq!(processed.load(Ordering::SeqCst), 40);
        assert_eq!(progress.phase("write").done(), 100);
        let recs = journal.records.lock();
        assert_eq!(recs.iter().filter(|r| r.1 == "write").count(), 40);
    }
}
