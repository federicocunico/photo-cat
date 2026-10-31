use parking_lot::Mutex;
use pc_core::{AppError, ErrorCode};
use pc_governor::MemoryGovernor;
use pc_jobs::manager::{JobManager, JobPlan, JobSnapshot, ProgressSink};
use pc_jobs::persist::{self, JobState};
use pc_jobs::{PhaseSpec, Pipeline, StageOutcome};
use pc_store::{Store, StoreOptions};
use serde_json::json;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Default)]
struct RecSink {
    snaps: Mutex<Vec<(Instant, JobSnapshot)>>,
}
impl ProgressSink for RecSink {
    fn on_progress(&self, s: &JobSnapshot) {
        self.snaps.lock().push((Instant::now(), s.clone()));
    }
}

struct Fixture {
    _tmp: tempfile::TempDir,
    store: Store,
    mgr: JobManager,
    _sink: Arc<RecSink>,
    processed: Arc<Mutex<Vec<u64>>>,
    gate: Arc<AtomicBool>,
    governor: MemoryGovernor,
}

/// Job di prova "count": elabora 0..n con una pausa per elemento.
/// `fail` = elementi che falliscono; se `gate` è falso il job si blocca dopo `block_after` elementi.
fn fixture(budget_mb: u64, max_concurrent: usize) -> Fixture {
    let tmp = tempfile::tempdir().unwrap();
    let store = Store::open(&tmp.path().join("c.db"), StoreOptions::default()).unwrap();
    fixture_on(tmp, store, budget_mb, max_concurrent)
}

fn fixture_on(
    tmp: tempfile::TempDir,
    store: Store,
    budget_mb: u64,
    max_concurrent: usize,
) -> Fixture {
    let sink = Arc::new(RecSink::default());
    let governor = MemoryGovernor::new(budget_mb);
    let mgr = JobManager::new(
        store.clone(),
        governor.clone(),
        max_concurrent,
        sink.clone(),
    );
    let processed = Arc::new(Mutex::new(Vec::new()));
    let gate = Arc::new(AtomicBool::new(true));
    let (p2, g2) = (processed.clone(), gate.clone());
    mgr.register(
        "count",
        Arc::new(move |params| {
            let n = params["n"].as_u64().unwrap_or(100);
            let sleep = Duration::from_micros(params["sleepUs"].as_u64().unwrap_or(0));
            let fail: HashSet<u64> = params["fail"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_u64()).collect())
                .unwrap_or_default();
            let block_after = params["blockAfter"].as_u64();
            let min = params["minMb"].as_u64().unwrap_or(0);
            let (p3, g3) = (p2.clone(), g2.clone());
            Ok(JobPlan {
                phases: vec![
                    PhaseSpec::new("scan", 1.0),
                    PhaseSpec::new("work", 4.0),
                    PhaseSpec::new("write", 1.0),
                ],
                min_memory_mb: min,
                run: Box::new(move |ctx| {
                    Pipeline::<u64>::new(ctx.job_id, ctx.progress, ctx.control)
                        .key(|x| x.to_string())
                        .journal(ctx.journal)
                        .source("scan", move |e| {
                            for i in 0..n {
                                if !e.emit(i) {
                                    break;
                                }
                            }
                            Ok(())
                        })
                        .stage("work", 2, move |x, _| {
                            std::thread::sleep(sleep);
                            if fail.contains(x) {
                                StageOutcome::Fail(AppError::new(
                                    ErrorCode::DecodeFailed,
                                    "file illeggibile",
                                ))
                            } else {
                                StageOutcome::Continue
                            }
                        })
                        .stage("write", 1, move |x, _| {
                            if let Some(b) = block_after {
                                if p3.lock().len() as u64 >= b {
                                    while !g3.load(Ordering::SeqCst) {
                                        std::thread::sleep(Duration::from_millis(5));
                                    }
                                }
                            }
                            p3.lock().push(*x);
                            StageOutcome::Continue
                        })
                        .run()
                }),
            })
        }),
    );
    Fixture {
        _tmp: tmp,
        store,
        mgr,
        _sink: sink,
        processed,
        gate,
        governor,
    }
}

fn state(f: &Fixture, id: i64) -> JobState {
    f.mgr.snapshot(id).unwrap().state
}

#[test]
fn completed_and_completed_with_errors() {
    let f = fixture(4000, 2);
    let ok = f.mgr.submit("count", json!({"n": 300})).unwrap();
    let ko = f
        .mgr
        .submit("count", json!({"n": 1000, "fail": [3, 999]}))
        .unwrap();
    assert!(f.mgr.wait_idle(Duration::from_secs(20)));
    assert_eq!(state(&f, ok), JobState::Completed);
    let s = f.mgr.snapshot(ko).unwrap();
    assert_eq!(s.state, JobState::CompletedWithErrors);
    assert_eq!(s.failed, 2);
    assert_eq!(s.overall_percent, 100.0);
    // Stato e contatori persistiti.
    assert_eq!(
        persist::get(&f.store, ko).unwrap().unwrap().state,
        JobState::CompletedWithErrors
    );
    let phases = persist::phases(&f.store, ko).unwrap();
    assert_eq!(phases[1], ("work".into(), 998, 2, 1000, true));
}

#[test]
fn pause_resume_and_cancel_keep_completed_results() {
    let f = fixture(4000, 2);
    let id = f
        .mgr
        .submit("count", json!({"n": 5000, "sleepUs": 300}))
        .unwrap();
    std::thread::sleep(Duration::from_millis(150));
    f.mgr.pause(id).unwrap();
    assert_eq!(state(&f, id), JobState::Paused);
    std::thread::sleep(Duration::from_millis(100)); // lascia terminare gli elementi in corso
    let before = f.processed.lock().len();
    std::thread::sleep(Duration::from_millis(200));
    let after = f.processed.lock().len();
    assert!(
        after <= before + 4,
        "in pausa non si elabora: {before} → {after}"
    );
    assert!(f.mgr.pause(id).is_err());

    f.mgr.resume(id).unwrap();
    std::thread::sleep(Duration::from_millis(150));
    assert!(f.processed.lock().len() > after);
    f.mgr.cancel(id).unwrap();
    assert!(f.mgr.wait_idle(Duration::from_secs(10)));
    let s = f.mgr.snapshot(id).unwrap();
    assert_eq!(s.state, JobState::Cancelled);
    let done = f.processed.lock().len();
    assert!(done > 0 && done < 5000);
    assert_eq!(
        persist::get(&f.store, id).unwrap().unwrap().state,
        JobState::Cancelled
    );
}

#[test]
fn queued_job_can_be_cancelled_and_budget_admission_is_fifo() {
    let f = fixture(1000, 2);
    f.gate.store(false, Ordering::SeqCst);
    let big = f
        .mgr
        .submit("count", json!({"n": 50, "minMb": 800, "blockAfter": 10}))
        .unwrap();
    let big2 = f
        .mgr
        .submit("count", json!({"n": 50, "minMb": 800}))
        .unwrap();
    let small = f
        .mgr
        .submit("count", json!({"n": 50, "minMb": 100}))
        .unwrap();
    std::thread::sleep(Duration::from_millis(150));
    assert_eq!(state(&f, big), JobState::Running);
    assert_eq!(
        state(&f, big2),
        JobState::Queued,
        "800 + 800 > 1000: attende"
    );
    assert_eq!(
        state(&f, small),
        JobState::Queued,
        "FIFO: dietro al job grande"
    );

    f.mgr.cancel(big2).unwrap();
    assert_eq!(state(&f, big2), JobState::Cancelled);
    std::thread::sleep(Duration::from_millis(100));
    assert_ne!(
        state(&f, small),
        JobState::Queued,
        "800 + 100 entra nel budget"
    );

    f.gate.store(true, Ordering::SeqCst);
    assert!(f.mgr.wait_idle(Duration::from_secs(10)));
    assert_eq!(state(&f, big), JobState::Completed);
    assert_eq!(state(&f, small), JobState::Completed);
}

#[test]
fn interrupted_job_resumes_only_pending_items() {
    let tmp = tempfile::tempdir().unwrap();
    let store = Store::open(&tmp.path().join("c.db"), StoreOptions::default()).unwrap();
    let f = fixture_on(tmp, store.clone(), 4000, 1);
    f.gate.store(false, Ordering::SeqCst);
    let id = f
        .mgr
        .submit("count", json!({"n": 200, "blockAfter": 80, "fail": [150]}))
        .unwrap();
    // Attende che il job sia bloccato a 80 elementi scritti, poi "uccide" il processo:
    // il vecchio gestore viene abbandonato con il job ancora "running" nel DB.
    let deadline = Instant::now() + Duration::from_secs(10);
    while f.processed.lock().len() < 80 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    store.flush().unwrap();
    assert_eq!(
        persist::get(&store, id).unwrap().unwrap().state,
        JobState::Running
    );
    let first_run: HashSet<u64> = f.processed.lock().iter().copied().collect();
    assert_eq!(first_run.len(), 80);

    // "Riavvio": nuovo gestore sullo stesso catalogo.
    let tmp2 = tempfile::tempdir().unwrap();
    let g = fixture_on(tmp2, store.clone(), 4000, 1);
    let interrupted = g.mgr.recover_interrupted().unwrap();
    assert_eq!(
        interrupted.iter().map(|r| r.id).collect::<Vec<_>>(),
        vec![id]
    );
    g.mgr.resume_interrupted(id).unwrap();
    assert!(g.mgr.wait_idle(Duration::from_secs(10)));
    let second_run: HashSet<u64> = g.processed.lock().iter().copied().collect();
    assert!(
        first_run.is_disjoint(&second_run),
        "nessun elemento rielaborato"
    );
    assert_eq!(first_run.len() + second_run.len(), 199, "200 − 1 fallito");
    let s = g.mgr.snapshot(id).unwrap();
    assert_eq!(s.state, JobState::CompletedWithErrors);
    assert_eq!(s.overall_percent, 100.0);
    f.gate.store(true, Ordering::SeqCst); // sblocca il thread abbandonato
    assert!(g.mgr.resume_interrupted(id).is_err());
}

#[test]
fn followup_job_is_queued_after_success_only() {
    let f = fixture(4000, 1);
    f.mgr.register(
        "chain",
        Arc::new(|params| {
            let next = params["next"].as_bool().unwrap_or(false);
            let cancel = params["cancel"].as_bool().unwrap_or(false);
            Ok(JobPlan {
                phases: vec![PhaseSpec::new("scan", 1.0)],
                min_memory_mb: 0,
                run: Box::new(move |ctx| {
                    if next {
                        *ctx.followup.lock() = Some(("count".into(), json!({"n": 5})));
                    }
                    if cancel {
                        ctx.control.cancel();
                    }
                    ctx.progress.phase("scan").finish();
                    Ok(pc_jobs::RunSummary {
                        cancelled: cancel,
                        failed: 0,
                    })
                }),
            })
        }),
    );
    f.mgr.submit("chain", json!({"next": true})).unwrap();
    assert!(f.mgr.wait_idle(Duration::from_secs(10)));
    assert!(f.mgr.wait_idle(Duration::from_secs(10)));
    let kinds: Vec<String> = f.mgr.snapshots().into_iter().map(|s| s.kind).collect();
    assert!(kinds.contains(&"count".to_string()), "{kinds:?}");
    assert_eq!(f.processed.lock().len(), 5);

    let g = fixture(4000, 1);
    g.mgr.register(
        "chain",
        Arc::new(|_| {
            Ok(JobPlan {
                phases: vec![PhaseSpec::new("scan", 1.0)],
                min_memory_mb: 0,
                run: Box::new(|ctx| {
                    *ctx.followup.lock() = Some(("count".into(), json!({"n": 5})));
                    ctx.control.cancel();
                    Ok(pc_jobs::RunSummary {
                        cancelled: true,
                        failed: 0,
                    })
                }),
            })
        }),
    );
    g.mgr.submit("chain", json!({})).unwrap();
    assert!(g.mgr.wait_idle(Duration::from_secs(10)));
    assert!(
        g.mgr.snapshots().iter().all(|s| s.kind != "count"),
        "nessun seguito dopo annullamento"
    );
}

#[test]
fn profile_change_sets_the_budget_of_later_jobs() {
    use pc_core::config::{ResourceProfile, ResourcesConfig};
    use pc_governor::{apply_resources, compute_budget, FakeProbe};
    let probe = FakeProbe::new(16_384, 12_000, 8);
    let budget = |profile| compute_budget(16_384, 12_000, profile, None).budget_mb;
    let (light, performance) = (
        budget(ResourceProfile::Light),
        budget(ResourceProfile::Performance),
    );
    // Due job da `min` MB: insieme entrano nel budget "Prestazioni" ma non in quello "Leggero".
    let min = light / 2 + 100;
    assert!(2 * min <= performance, "{light} / {performance}");

    let f = fixture(performance, 2);
    let profile = |p| ResourcesConfig {
        profile: p,
        ..Default::default()
    };
    f.gate.store(false, Ordering::SeqCst);
    let a = f
        .mgr
        .submit("count", json!({"n": 50, "minMb": min, "blockAfter": 5}))
        .unwrap();
    let b = f
        .mgr
        .submit("count", json!({"n": 50, "minMb": min, "blockAfter": 5}))
        .unwrap();
    std::thread::sleep(Duration::from_millis(150));
    assert_eq!(
        (state(&f, a), state(&f, b)),
        (JobState::Running, JobState::Running)
    );
    f.gate.store(true, Ordering::SeqCst);
    assert!(f.mgr.wait_idle(Duration::from_secs(10)));

    assert_eq!(
        apply_resources(&f.governor, &probe, &profile(ResourceProfile::Light)).budget_mb,
        light
    );
    assert_eq!(f.governor.snapshot().budget_mb, light);
    f.gate.store(false, Ordering::SeqCst);
    let c = f
        .mgr
        .submit("count", json!({"n": 50, "minMb": min, "blockAfter": 5}))
        .unwrap();
    let d = f
        .mgr
        .submit("count", json!({"n": 50, "minMb": min}))
        .unwrap();
    std::thread::sleep(Duration::from_millis(150));
    assert_eq!(state(&f, c), JobState::Running);
    assert_eq!(
        state(&f, d),
        JobState::Queued,
        "col profilo Leggero i due job non entrano insieme"
    );
    f.gate.store(true, Ordering::SeqCst);
    assert!(f.mgr.wait_idle(Duration::from_secs(10)));
    assert_eq!(state(&f, d), JobState::Completed);
}
