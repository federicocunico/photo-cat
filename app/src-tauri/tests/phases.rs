//! Elenco dei nomi di fase emessi dal backend, per il test delle traduzioni dell'interfaccia
//! (ui-usability-audit, F4): ogni nome in `app/ui/src/test/fixtures/phases.json` deve avere
//! una voce in `jobs.phases` (it ed en).
//!
//! I nomi si raccolgono dai sorgenti di tutti i crate: costanti `*PHASE*: &str`, letterali in
//! `PhaseSpec::new("…")` e `.phase("…")` e campi `phase = "…"` dei log (le fasi del registro
//! errori). I moduli `#[cfg(test)]` in fondo ai file e le cartelle `tests` non contano.
//!
//! Se l'elenco cambia, il test riscrive il file e fallisce: rilanciare i test dell'interfaccia,
//! aggiungere le traduzioni mancanti e includere il file nel commit.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == "tests") {
                continue;
            }
            rust_sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Letterale tra virgolette che segue `prefix` nella riga, se c'è.
fn literals_after<'a>(line: &'a str, prefix: &str) -> Vec<&'a str> {
    let mut found = Vec::new();
    let mut rest = line;
    while let Some(at) = rest.find(prefix) {
        let after = &rest[at + prefix.len()..];
        if let Some(end) = after.find('"') {
            found.push(&after[..end]);
            rest = &after[end..];
        } else {
            break;
        }
    }
    found
}

fn phase_names(source: &str) -> Vec<String> {
    // I test in fondo al file usano nomi di comodo ("scan", "ai"…): non sono fasi reali.
    let code = source.split("#[cfg(test)]").next().unwrap_or_default();
    let mut names: Vec<String> = Vec::new();
    let mut push = |found: Vec<&str>| names.extend(found.into_iter().map(str::to_owned));
    for line in code.lines() {
        let line = line.trim_start();
        if line.starts_with("//") {
            continue;
        }
        if line.contains("const ") && line.contains("PHASE") {
            push(literals_after(line, ": &str = \""));
        }
        push(literals_after(line, "PhaseSpec::new(\""));
        push(literals_after(line, ".phase(\""));
        // Campi dei log (`phase = "…"`), non assegnamenti come `status.phase = "…"` (avvio).
        let fields = line.replace(".phase = \"", "");
        push(literals_after(&fields, "phase = \""));
        push(literals_after(&fields, "phase=\""));
    }
    names
}

#[test]
fn elenco_delle_fasi_per_le_traduzioni() {
    let root = repo_root();
    let mut files = Vec::new();
    rust_sources(&root.join("crates"), &mut files);
    rust_sources(&root.join("app/src-tauri/src"), &mut files);
    let mut names = BTreeSet::new();
    for file in files {
        let source = fs::read_to_string(&file).expect("sorgente leggibile");
        names.extend(phase_names(&source));
    }
    for expected in [
        "clip",
        "faces",
        "geo",
        "persons",
        "scansione",
        "sviluppo_raw",
    ] {
        assert!(
            names.contains(expected),
            "fase {expected} non trovata nei sorgenti"
        );
    }

    let json = format!(
        "[\n{}\n]\n",
        names
            .iter()
            .map(|n| format!("  \"{n}\""))
            .collect::<Vec<_>>()
            .join(",\n")
    );
    let target = root.join("app/ui/src/test/fixtures/phases.json");
    let current = fs::read_to_string(&target)
        .unwrap_or_default()
        .replace("\r\n", "\n");
    if current != json {
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, &json).unwrap();
        panic!(
            "elenco delle fasi aggiornato in {}: aggiungere le traduzioni in jobs.phases e \
             includere il file nel commit",
            target.display()
        );
    }
}

#[test]
fn i_moduli_di_test_non_contano() {
    let src = "const PHASE: &str = \"vera\";\n#[cfg(test)]\nmod tests { const PHASE_X: &str = \"finta\"; }";
    assert_eq!(phase_names(src), vec!["vera".to_owned()]);
}
