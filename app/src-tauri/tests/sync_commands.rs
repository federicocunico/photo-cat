//! I comandi sincroni di Tauri girano sul thread principale, che ospita la WebView: una query
//! lenta o un writer occupato (import in corso) bloccherebbero l'interfaccia. Questo test elenca
//! i comandi sincroni ammessi e fallisce se un comando sincrono usa il catalogo o i job.
//! Un comando nuovo che legge il catalogo va scritto `async` con `crate::state::blocking`.

use std::path::Path;

/// Comandi sincroni ammessi: nessuno legge il catalogo, l'archivio dell'app o i job persistiti.
const ALLOWED_SYNC: &[&str] = &[
    "app_info",
    "boot_retry",
    "boot_status",
    "dismiss_crash_notice",
    "take_ui_recovery",
    "report_long_tasks",
    "dev_native_crash",
    "get_config",
    "get_resources",
    "list_jobs",
    "log_tail",
    "set_log_level",
    "open_logs",
    "system_info",
    "set_execution_provider",
    "set_resource_profile",
    "export_fields",
    "list_timezones",
    "ui_prefs_load",
    "ui_prefs_save",
];

/// Uso del catalogo nel corpo di un comando.
fn touches_catalog(body: &str) -> bool {
    body.contains("store")
        || body.contains("jobs.submit")
        || body.contains("jobs.pause")
        || body.contains("jobs.resume")
        || body.contains("jobs.cancel")
}

/// (nome, async, corpo) di ogni `#[tauri::command]` del file.
fn commands(src: &str) -> Vec<(String, bool, String)> {
    let mut out = Vec::new();
    let mut rest = src;
    while let Some(i) = rest.find("#[tauri::command") {
        rest = &rest[i..];
        let Some(f) = rest.find("pub ") else { break };
        let head = &rest[f..];
        let is_async = head.starts_with("pub async fn");
        let name_start = head.find("fn ").unwrap() + 3;
        let name: String = head[name_start..]
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        // Corpo: dalla prima graffa dopo la firma alla sua chiusura.
        let open = head.find('{').unwrap();
        let mut depth = 0;
        let mut end = open;
        for (k, ch) in head[open..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = open + k;
                        break;
                    }
                }
                _ => {}
            }
        }
        out.push((name, is_async, head[open..=end].to_string()));
        rest = &head[end..];
    }
    out
}

#[test]
fn synchronous_commands_never_touch_the_catalog() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut sync = Vec::new();
    let mut offending = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "rs") {
            continue;
        }
        let src = std::fs::read_to_string(&path).unwrap();
        for (name, is_async, body) in commands(&src) {
            if is_async {
                continue;
            }
            if touches_catalog(&body) {
                offending.push(name.clone());
            }
            sync.push(name);
        }
    }
    assert!(
        offending.is_empty(),
        "comandi sincroni che usano il catalogo (vanno resi async con state::blocking): {offending:?}"
    );
    sync.sort();
    let mut allowed: Vec<String> = ALLOWED_SYNC.iter().map(|s| s.to_string()).collect();
    allowed.sort();
    assert_eq!(
        sync, allowed,
        "elenco dei comandi sincroni cambiato: se il nuovo comando non legge il catalogo, aggiungilo ad ALLOWED_SYNC"
    );
}
