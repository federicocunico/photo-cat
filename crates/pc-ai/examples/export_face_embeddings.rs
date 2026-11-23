//! Comando di sviluppo: esporta in un file JSONL gli embedding dei volti delle persone con
//! nome di un catalogo, con soli id anonimi (identità, foto e volti rinumerati da 1), per il
//! banco di prova `cluster_bench` (design D9). Il catalogo si apre in sola lettura. Il file
//! contiene dati biometrici: resta sul computer e non va mai nel repository (il comando
//! rifiuta una destinazione dentro un repository git; `*.faces.jsonl` è escluso da git).
//!
//!   cargo run -p pc-ai --example export_face_embeddings -- <catalog.db> <uscita.faces.jsonl> [--unlabeled]
//!
//! Con `--unlabeled` aggiunge gli altri volti, senza identità (distrattori per il banco).

use std::path::{Path, PathBuf};

/// `true` se `path` sta dentro un repository git (una cartella antenata contiene `.git`).
fn inside_git_repo(path: &Path) -> bool {
    let abs = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    abs.ancestors().skip(1).any(|d| d.join(".git").exists())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    let (Some(catalog), Some(out)) = (positional.first(), positional.get(1)) else {
        eprintln!("uso: export_face_embeddings <catalog.db> <uscita.faces.jsonl> [--unlabeled]");
        std::process::exit(2);
    };
    let out = PathBuf::from(out);
    if inside_git_repo(&out) {
        eprintln!(
            "rifiutato: {} è dentro un repository git; scegli una cartella fuori dal repository",
            out.display()
        );
        std::process::exit(2);
    }
    let conn = rusqlite::Connection::open_with_flags(
        catalog,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .unwrap_or_else(|e| {
        eprintln!("apertura del catalogo non riuscita: {e}");
        std::process::exit(1);
    });
    let mut file = std::io::BufWriter::new(std::fs::File::create(&out).unwrap_or_else(|e| {
        eprintln!("creazione di {} non riuscita: {e}", out.display());
        std::process::exit(1);
    }));
    let unlabeled = args.iter().any(|a| a == "--unlabeled");
    match pc_ai::bench::export_named(&conn, &mut file, unlabeled) {
        Ok(n) => println!("{n} volti esportati in {}", out.display()),
        Err(e) => {
            eprintln!("esportazione non riuscita: {e}");
            std::process::exit(1);
        }
    }
}
