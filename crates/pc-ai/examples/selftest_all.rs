//! Esegue i self-test di tutti i modelli: `cargo run -p pc-ai --example selftest_all -- [cpu|auto]`
use pc_ai::selftest::{self_test, SelfTestEnv};
use pc_core::config::ExecutionProvider;
use pc_models::runtime::{self, SessionOptions};
use pc_models::{Manifest, Role};
use std::path::PathBuf;

fn main() {
    let provider = match std::env::args().nth(1).as_deref() {
        Some("cpu") => ExecutionProvider::Cpu,
        _ => ExecutionProvider::Auto,
    };
    runtime::init(&runtime::dev_runtime_path()).unwrap();
    let assets = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    let manifest = Manifest::builtin();
    let models = assets.join("models");
    let handle = std::thread::spawn(move || {
        let resolve = |r: Role| manifest.by_role(r).map(|e| models.join(&e.file));
        let only = std::env::args().nth(2);
        for e in manifest
            .entries
            .iter()
            .filter(|e| only.as_deref().is_none_or(|o| o == e.id))
        {
            eprintln!("-> {}", e.id);
            let preferred = if e.gpu {
                provider
            } else {
                ExecutionProvider::Cpu
            };
            let env = SelfTestEnv {
                images: assets.join("selftest"),
                opts: SessionOptions {
                    preferred,
                    intra_threads: 4,
                    ..Default::default()
                },
                resolve: &resolve,
            };
            let r = self_test(e, &models.join(&e.file), &env);
            eprintln!("   {} {:.1} ms {}", r.passed, r.avg_ms, r.message);
        }
    });
    handle.join().unwrap();
}
