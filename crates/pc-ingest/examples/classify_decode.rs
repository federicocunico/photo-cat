//! Classifica le decodifiche fallite di una cartella (in sola lettura), come il task 1.3 del
//! change large-library-stability: per ogni immagine prova `decode_reduced` alla dimensione
//! delle miniature e raggruppa gli errori per estensione e messaggio.
//!
//!   cargo run --release -p pc-ingest --features heif --example classify_decode -- D:\Foto [out.tsv]
//!
//! Non scrive nulla nella cartella analizzata. Il file `out.tsv` (facoltativo) elenca i file
//! falliti: percorso, estensione, codice e messaggio.

use pc_governor::MemoryGovernor;
use pc_ingest::formats::support;
use pc_ingest::thumb::{decode_reduced, THUMB_SIZE};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

/// (estensione, codice, causa) -> (file, esempi)
type Groups = BTreeMap<(String, String, String), (usize, Vec<String>)>;

fn main() {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().expect("cartella da analizzare"));
    let out = args.next().map(PathBuf::from);
    let files: Vec<(PathBuf, String)> = walkdir::WalkDir::new(&root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| {
            let ext = e.path().extension()?.to_str()?.to_ascii_lowercase();
            support(&ext).map(|_| (e.path().to_path_buf(), ext))
        })
        .collect();
    eprintln!("{} immagini in {}", files.len(), root.display());
    let gov = MemoryGovernor::new(4096);
    let next = AtomicUsize::new(0);
    let ok = AtomicUsize::new(0);
    // (estensione, codice, messaggio normalizzato) -> (conteggio, esempi)
    let failures: Mutex<Groups> = Mutex::default();
    let rows: Mutex<Vec<String>> = Mutex::default();
    let started = Instant::now();
    let threads = std::thread::available_parallelism()
        .map_or(4, |n| n.get())
        .min(8);
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some((path, ext)) = files.get(i) else {
                    break;
                };
                match decode_reduced(path, ext, THUMB_SIZE, &gov) {
                    Ok(_) => {
                        ok.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(e) => {
                        let msg = normalize(&e.detail);
                        let key = (ext.clone(), e.code.as_str().to_string(), msg);
                        let mut f = failures.lock().unwrap();
                        let entry = f.entry(key).or_default();
                        entry.0 += 1;
                        if entry.1.len() < 3 {
                            entry.1.push(path.display().to_string());
                        }
                        rows.lock().unwrap().push(format!(
                            "{}\t{}\t{}\t{}",
                            path.display(),
                            ext,
                            e.code.as_str(),
                            e.detail.replace(['\t', '\n'], " ")
                        ));
                    }
                }
                if i.is_multiple_of(2000) && i > 0 {
                    eprintln!("  {i}/{} in {:?}", files.len(), started.elapsed());
                }
            });
        }
    });
    let failed: usize = failures.lock().unwrap().values().map(|v| v.0).sum();
    println!(
        "Totale {} · decodificate {} · fallite {} · tempo {:?}",
        files.len(),
        ok.load(Ordering::Relaxed),
        failed,
        started.elapsed()
    );
    println!("| Estensione | Codice | Causa | File | Esempi |");
    println!("|---|---|---|---|---|");
    let mut groups: Vec<_> = failures.into_inner().unwrap().into_iter().collect();
    groups.sort_by_key(|g| std::cmp::Reverse(g.1 .0));
    for ((ext, code, msg), (n, examples)) in groups {
        println!(
            "| {ext} | {code} | {msg} | {n} | {} |",
            examples.join("<br>")
        );
    }
    if let Some(out) = out {
        let mut f = std::fs::File::create(out).expect("file di uscita");
        for r in rows.into_inner().unwrap() {
            let _ = writeln!(f, "{r}");
        }
    }
}

/// Messaggio senza numeri variabili (posizioni, dimensioni), per raggruppare le cause.
fn normalize(s: &str) -> String {
    let mut out = String::new();
    let mut digits = false;
    for c in s.chars() {
        if c.is_ascii_digit() {
            if !digits {
                out.push('N');
            }
            digits = true;
        } else {
            digits = false;
            out.push(c);
        }
    }
    out.chars().take(140).collect()
}
