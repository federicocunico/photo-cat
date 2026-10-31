//! Job dimostrativo: attraversa le stesse fasi di un import senza toccare file,
//! per verificare a mano la schermata Job (progresso, ETA, pausa, errori).

use pc_core::{AppError, ErrorCode};
use pc_jobs::{JobPlan, PhaseSpec, Pipeline, StageOutcome};
use std::time::Duration;

pub fn plan(params: &serde_json::Value) -> pc_core::AppResult<JobPlan> {
    let items = params["items"].as_u64().unwrap_or(2000);
    let fail_every = params["failEvery"].as_u64().unwrap_or(0);
    let phases = vec![
        PhaseSpec::new("scansione", 0.5),
        PhaseSpec::new("metadati", 1.0),
        PhaseSpec::new("miniature", 2.0),
        PhaseSpec::new("analisi_ai", 4.0),
        PhaseSpec::new("scrittura", 0.5),
    ];
    Ok(JobPlan {
        phases,
        min_memory_mb: 64,
        run: Box::new(move |ctx| {
            let gov = ctx.governor.clone();
            Pipeline::<u64>::new(ctx.job_id, ctx.progress, ctx.control)
                .key(|i| format!("D:\\Demo\\IMG_{i:05}.JPG"))
                .journal(ctx.journal)
                .source("scansione", move |e| {
                    for i in 0..items {
                        std::thread::sleep(Duration::from_micros(300));
                        if !e.emit(i) {
                            break;
                        }
                    }
                    Ok(())
                })
                .stage("metadati", 2, |_, _| {
                    std::thread::sleep(Duration::from_millis(2));
                    StageOutcome::Continue
                })
                .stage("miniature", 2, move |i, _| {
                    let _permit = gov.acquire(24);
                    std::thread::sleep(Duration::from_millis(6));
                    if fail_every > 0 && *i % fail_every == fail_every - 1 {
                        return StageOutcome::Fail(AppError::new(
                            ErrorCode::DecodeFailed,
                            format!("decodifica non riuscita per IMG_{i:05}.JPG (simulato)"),
                        ));
                    }
                    StageOutcome::Continue
                })
                .stage("analisi_ai", 2, |_, _| {
                    std::thread::sleep(Duration::from_millis(12));
                    StageOutcome::Continue
                })
                .stage("scrittura", 1, |_, _| StageOutcome::Continue)
                .run()
        }),
    })
}
