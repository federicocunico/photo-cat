//! Binario separato: installa un subscriber globale (i worker girano su altri thread).

use parking_lot::Mutex;
use pc_governor::MemoryGovernor;
use pc_jobs::manager::{JobManager, JobPlan, JobSnapshot, ProgressSink};
use pc_jobs::{JobState, PhaseSpec, Pipeline, StageOutcome};
use pc_store::{Store, StoreOptions};
use serde_json::json;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Default)]
struct RecSink {
    snaps: Mutex<Vec<JobSnapshot>>,
}
impl ProgressSink for RecSink {
    fn on_progress(&self, s: &JobSnapshot) {
        self.snaps.lock().push(s.clone());
    }
}

#[test]
fn ticker_rate_is_limited_and_phase_logs_are_written() {
    let logs = tempfile::tempdir().unwrap();
    let (sub, diag) = pc_diag::build(logs.path(), &pc_core::config::LoggingConfig::default());
    let mem = Arc::new(pc_diag::MemorySink::default());
    diag.hub().add_sink(mem.clone());
    tracing::subscriber::set_global_default(sub).unwrap();

    let tmp = tempfile::tempdir().unwrap();
    let store = Store::open(&tmp.path().join("c.db"), StoreOptions::default()).unwrap();
    let sink = Arc::new(RecSink::default());
    let mgr = JobManager::new(store, MemoryGovernor::new(4000), 1, sink.clone());
    mgr.register(
        "count",
        Arc::new(|params| {
            let n = params["n"].as_u64().unwrap();
            Ok(JobPlan {
                phases: vec![
                    PhaseSpec::new("scan", 1.0),
                    PhaseSpec::new("work", 4.0),
                    PhaseSpec::new("write", 1.0),
                ],
                min_memory_mb: 0,
                run: Box::new(move |ctx| {
                    Pipeline::<u64>::new(ctx.job_id, ctx.progress, ctx.control)
                        .source("scan", move |e| {
                            (0..n).for_each(|i| {
                                e.emit(i);
                            });
                            Ok(())
                        })
                        .stage("work", 2, |_, _| StageOutcome::Continue)
                        .stage("write", 1, |_, _| StageOutcome::Continue)
                        .run()
                }),
            })
        }),
    );

    let ticker = mgr.start_ticker();
    let started = Instant::now();
    let id = mgr.submit("count", json!({"n": 500_000})).unwrap();
    assert!(mgr.wait_idle(Duration::from_secs(60)));
    let elapsed = started.elapsed().as_secs_f64();
    drop(ticker);

    let snaps = sink.snaps.lock();
    let running = snaps
        .iter()
        .filter(|s| s.id == id && s.state == JobState::Running)
        .count();
    // ~10 aggiornamenti al secondo dal ticker.
    assert!(
        (running as f64) <= elapsed * 11.0 + 3.0,
        "{running} aggiornamenti in {elapsed:.2}s"
    );
    assert_eq!(snaps.last().unwrap().state, JobState::Completed);

    let lines = mem.lines.lock();
    let ends: Vec<_> = lines
        .iter()
        .filter(|l| l.message == "fase completata")
        .collect();
    assert_eq!(ends.len(), 3, "una riga per fase");
    for l in &ends {
        for k in [
            "duration_ms",
            "done",
            "failed",
            "total",
            "percent",
            "phase",
            "job_id",
        ] {
            assert!(
                l.fields.iter().any(|(f, _)| f == k),
                "campo {k} mancante in {l:?}"
            );
        }
    }
    let work = ends
        .iter()
        .find(|l| l.fields.iter().any(|(k, v)| k == "phase" && v == "work"))
        .unwrap();
    assert!(
        work.fields.contains(&("done".into(), "500000".into())),
        "{work:?}"
    );
    assert!(lines.iter().any(|l| l.message == "job terminato"));
}
