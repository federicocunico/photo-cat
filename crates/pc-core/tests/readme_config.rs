//! Gli esempi di `config.toml` del README (luogo, "Apri con", soglie AI) si
//! caricano senza errori (task 7.3).
use pc_core::Config;

#[test]
fn readme_config_examples_load() {
    let readme = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../README.md"))
        .unwrap()
        .replace("\r\n", "\n");
    let start = readme
        .find("## Luogo, cartelle, \"Apri con\" e soglie AI")
        .expect("sezione del README");
    let section = &readme[start..];
    let section = &section[..section[3..].find("\n## ").map_or(section.len(), |i| i + 3)];
    let blocks: Vec<&str> = section
        .split("```toml\n")
        .skip(1)
        .map(|b| b.split("```").next().unwrap())
        .collect();
    assert!(blocks.len() >= 4, "esempi trovati: {}", blocks.len());
    let tmp = tempfile::tempdir().unwrap();
    for (i, block) in blocks.iter().enumerate() {
        let p = tmp.path().join(format!("config{i}.toml"));
        std::fs::write(&p, block).unwrap();
        let cfg = Config::load(&p).unwrap_or_else(|e| panic!("esempio {i}: {e:?}\n{block}"));
        if block.contains("[[external_apps]]") {
            assert!(!cfg.external_apps.is_empty());
        }
    }
}
