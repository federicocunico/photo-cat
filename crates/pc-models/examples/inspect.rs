//! Stampa ingressi e uscite dei modelli del manifest: `cargo run -p pc-models --example inspect`
fn main() {
    pc_models::runtime::init(&pc_models::runtime::dev_dll_path()).unwrap();
    let dir = pc_models::runtime::dev_models_dir();
    for e in pc_models::Manifest::builtin().entries {
        if !e.file.ends_with(".onnx") {
            continue;
        }
        let opts = pc_models::runtime::SessionOptions {
            preferred: pc_core::config::ExecutionProvider::Cpu,
            ..Default::default()
        };
        let (s, _) = pc_models::runtime::load(&dir.join(&e.file), &opts).unwrap();
        println!("== {}", e.id);
        for i in s.inputs() {
            println!("  in  {} {:?}", i.name(), i.dtype());
        }
        for o in s.outputs() {
            println!("  out {} {:?}", o.name(), o.dtype());
        }
    }
}
