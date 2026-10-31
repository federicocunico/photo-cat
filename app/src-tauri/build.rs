fn main() {
    tauri_build::build();
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
