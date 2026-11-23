fn main() {
    tauri_build::build();
    build_info();
    // tauri-build incorpora il manifest Windows (comctl32 v6) solo nei binari:
    // senza di esso i test d'integrazione non si avviano (STATUS_ENTRYPOINT_NOT_FOUND).
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        let out = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
        println!(
            "cargo:rustc-link-arg-tests={}",
            out.join("resource.lib").display()
        );
    }
}

/// Dati della build per la finestra Informazioni (app-about, "Dati della build"): commit
/// abbreviato (vuoto senza git, ad esempio da un archivio dei sorgenti), data della build e
/// configurazione. Con `SOURCE_DATE_EPOCH` la data è quella indicata, così la build è riproducibile.
fn build_info() {
    let commit = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    println!("cargo:rustc-env=PC_BUILD_COMMIT={commit}");

    let epoch = std::env::var("SOURCE_DATE_EPOCH")
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        });
    println!("cargo:rustc-env=PC_BUILD_DATE={}", iso_date(epoch));
    println!(
        "cargo:rustc-env=PC_BUILD_PROFILE={}",
        std::env::var("PROFILE").unwrap_or_default()
    );

    println!("cargo:rerun-if-env-changed=SOURCE_DATE_EPOCH");
    // Nuovo commit o cambio di branch: si aggiorna il commit.
    let git = std::path::Path::new("../../.git");
    for p in ["HEAD", "refs/heads"] {
        let path = git.join(p);
        if path.exists() {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
}

/// Data "AAAA-MM-GG" (UTC) da secondi Unix, senza dipendenze (algoritmo civil-from-days).
fn iso_date(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}
