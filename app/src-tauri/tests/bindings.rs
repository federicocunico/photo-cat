//! Rigenera i binding TypeScript; in CI si verifica poi che il file non cambi
//! (`git diff --exit-code app/ui/src/bindings.ts`).

#[test]
fn export_bindings() {
    let path = photo_cat_lib::bindings_path();
    photo_cat_lib::export_bindings(&photo_cat_lib::bindings(), &path);
    let text = std::fs::read_to_string(path).unwrap();
    for name in [
        "listJobs",
        "queryErrors",
        "JobSnapshot",
        "ResourceSnapshot",
        "LogLine",
    ] {
        assert!(text.contains(name), "{name} mancante nei binding");
    }
}
