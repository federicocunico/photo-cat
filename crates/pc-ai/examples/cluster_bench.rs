//! Banco di prova del raggruppamento dei volti (design D9): confronta il raggruppamento di
//! prima (Chinese Whispers su kNN, una soglia) con quello in due passaggi, sui dati sintetici
//! e, se indicato, su un file JSONL di embedding anonimi esportato dalla libreria
//! (`export_face_embeddings`). Il file resta sul computer: non va mai nel repository.
//!
//!   cargo run -p pc-ai --example cluster_bench -- [opzioni]
//!
//! Opzioni:
//!   --seed N            seme dei dati sintetici (predefinito 42)
//!   --identities N      identità sintetiche (120)
//!   --largest N         volti dell'identità più grande (160)
//!   --jsonl FILE        anche il file JSONL indicato
//!   --no-synthetic      solo il file JSONL
//!   --ask X --auto Y --margin Z   soglie (predefinite 0,363 / 0,55 / 0,05)
//!   --named N           identità già con un nome: le N con più volti (12)
//!   --confirmed X       quota dei loro volti già confermati (0,3)

use pc_ai::aggregate::{self, Params};
use pc_ai::bench::{self, Metrics, Sample};

fn arg<T: std::str::FromStr>(args: &[String], name: &str) -> Option<T> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.replace(',', ".").parse().ok())
}

fn row(name: &str, m: &Metrics, extra: &str) {
    println!(
        "{name:<10} BCubed F {:.4} (P {:.4} R {:.4})  F coppie {:.4}  gruppi/identità {:.2}  \
         gruppi da 1 volto {:>5.1}%  gruppi {:>5}  unioni auto {:>4}  precisione unioni {:.4}{extra}",
        m.bcubed_f,
        m.bcubed_precision,
        m.bcubed_recall,
        m.pairwise_f,
        m.groups_per_identity,
        m.single_face_share * 100.0,
        m.groups,
        m.auto_merges,
        m.auto_precision,
    );
}

/// Quantili dei coseni tra volti della stessa identità (etichettati) e tra volti della stessa
/// foto (sempre persone diverse), per tarare le soglie e i dati sintetici.
fn distributions(samples: &[Sample]) {
    use pc_store::people::cosine;
    let mut genuine = Vec::new();
    let mut same_photo = Vec::new();
    for (i, a) in samples.iter().enumerate() {
        for b in &samples[i + 1..] {
            if a.photo == b.photo {
                same_photo.push(cosine(&a.embedding, &b.embedding));
            } else if a.identity.is_some() && a.identity == b.identity {
                genuine.push(cosine(&a.embedding, &b.embedding));
            }
        }
    }
    genuine.sort_by(|a, b| a.total_cmp(b));
    same_photo.sort_by(|a, b| a.total_cmp(b));
    let q = |v: &[f32], p: f64| pc_ai::calibrate::quantile(v, p).unwrap_or(f32::NAN);
    println!(
        "           coseni stessa identità ({} coppie): 10% {:.3} 25% {:.3} 50% {:.3} 75% {:.3} 90% {:.3}",
        genuine.len(),
        q(&genuine, 0.1),
        q(&genuine, 0.25),
        q(&genuine, 0.5),
        q(&genuine, 0.75),
        q(&genuine, 0.9)
    );
    println!(
        "           coseni stessa foto ({} coppie): 50% {:.3} 90% {:.3} 99% {:.3} 99,9% {:.3} max {:.3}",
        same_photo.len(),
        q(&same_photo, 0.5),
        q(&same_photo, 0.9),
        q(&same_photo, 0.99),
        q(&same_photo, 0.999),
        same_photo.last().copied().unwrap_or(f32::NAN)
    );
}

fn run(title: &str, samples: &[Sample], params: Params, share: f64, seed: u64) {
    let named: usize = arg(&std::env::args().collect::<Vec<_>>(), "--named").unwrap_or(12);
    let confirmed = bench::confirmed_mask(samples, named, share, seed);
    let labeled = samples.iter().filter(|s| s.identity.is_some()).count();
    let identities: std::collections::HashSet<i64> =
        samples.iter().filter_map(|s| s.identity).collect();
    println!(
        "\n== {title}: {} volti ({labeled} etichettati, {} identità), {} già confermati",
        samples.len(),
        identities.len(),
        confirmed.iter().filter(|c| **c).count()
    );
    distributions(samples);
    let t = std::time::Instant::now();
    let legacy = bench::legacy_cluster(samples, &confirmed, params.ask);
    let t_legacy = t.elapsed();
    row(
        "prima",
        &bench::measure(samples, &legacy, None),
        &format!("  [{:.2?}]", t_legacy),
    );
    let t = std::time::Instant::now();
    let (_, out) = bench::run_new(samples, &confirmed, params);
    let t_new = t.elapsed();
    let m = bench::measure(samples, &out.labels, Some(&out));
    let (gp, gg, gc) = bench::question_kinds(&out.questions);
    row(
        "dopo",
        &m,
        &format!(
            "  domande {} (persona {gp}, gruppi {gg}, scelta {gc})  [{:.2?}]",
            out.questions.len(),
            t_new
        ),
    );
    if std::env::args().any(|a| a == "--debug") {
        // Gruppi con più identità: dimensione per identità.
        let mut groups: std::collections::BTreeMap<_, std::collections::BTreeMap<i64, usize>> =
            Default::default();
        for (s, l) in samples.iter().zip(&out.labels) {
            if let Some(id) = s.identity {
                *groups.entry(*l).or_default().entry(id).or_default() += 1;
            }
        }
        let mut impure: Vec<_> = groups.iter().filter(|(_, m)| m.len() > 1).collect();
        impure.sort_by_key(|(_, m)| std::cmp::Reverse(m.values().sum::<usize>()));
        for (l, m) in impure.iter().take(12) {
            println!("           impuro {l:?}: {m:?}");
        }
        let ident = |l: &aggregate::Label| groups.get(l).cloned();
        for q in out.questions.iter().take(25) {
            println!(
                "           domanda {:?} volti {} p {:.2} punteggio {:.3}: {:?} → {:?}",
                q.kind,
                q.faces,
                q.probability,
                q.score,
                ident(&q.a),
                ident(&q.b).map(|m| m.into_iter().max_by_key(|x| x.1))
            );
        }
    }
    let mut by_prob = out.questions.clone();
    aggregate::order_by_probability(&mut by_prob);
    let mixed = bench::trajectory(samples, &out, &out.questions);
    let prob = bench::trajectory(samples, &out, &by_prob);
    let (start, end) = (mixed[0], *mixed.last().unwrap_or(&mixed[0]));
    let half = start + (end - start) / 2.0;
    let reach = |t: &[f64], target: f64| {
        t.iter()
            .position(|f| *f >= target)
            .map_or("non raggiunta".to_string(), |n| n.to_string())
    };
    println!(
        "           utente perfetto: BCubed F {start:.4} → {end:.4} rispondendo a tutte; \
         domande per 0,95: misto {} / per probabilità {}; per metà del guadagno ({half:.4}): \
         misto {} / per probabilità {}",
        reach(&mixed, 0.95),
        reach(&prob, 0.95),
        reach(&mixed, half),
        reach(&prob, half)
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let defaults = Params::default();
    let params = Params {
        ask: arg(&args, "--ask").unwrap_or(defaults.ask),
        auto: arg(&args, "--auto").unwrap_or(defaults.auto),
        margin: arg(&args, "--margin").unwrap_or(defaults.margin),
    };
    let share: f64 = arg(&args, "--confirmed").unwrap_or(0.3);
    let seed: u64 = arg(&args, "--seed").unwrap_or(42);
    println!(
        "soglie: chiedi {:.3}, unisci da solo {:.3}, margine {:.3}",
        params.ask, params.auto, params.margin
    );
    if !args.iter().any(|a| a == "--no-synthetic") {
        let cfg = bench::SynthConfig {
            seed,
            identities: arg(&args, "--identities").unwrap_or(120),
            largest: arg(&args, "--largest").unwrap_or(160),
            far: (
                arg(&args, "--far-min").unwrap_or(0.78),
                arg(&args, "--far-max").unwrap_or(0.9),
            ),
            ..Default::default()
        };
        let samples = bench::synthetic(&cfg);
        run(
            &format!("dati sintetici (seme {seed})"),
            &samples,
            params,
            share,
            seed,
        );
    }
    if let Some(path) = arg::<String>(&args, "--jsonl") {
        match bench::read_jsonl(std::path::Path::new(&path)) {
            Ok(samples) => run("file JSONL", &samples, params, share, seed),
            Err(e) => eprintln!("lettura di {path} non riuscita: {e}"),
        }
    }
}
