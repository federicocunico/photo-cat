//! Diagnostica: carica ONNX Runtime da un percorso e stampa ogni passo (uso: ort_probe <onnxruntime.dll>).
fn main() {
    let dll = std::path::PathBuf::from(std::env::args().nth(1).expect("percorso onnxruntime.dll"));
    println!("1. file presente: {}", dll.is_file());
    let t = std::time::Instant::now();
    match pc_models::runtime::init(&dll) {
        Ok(v) => println!("2. init ok in {:?}: {v}", t.elapsed()),
        Err(e) => println!("2. init errore in {:?}: {}", t.elapsed(), e.detail),
    }
    println!(
        "3. provider: {:?}",
        pc_models::runtime::available_providers()
    );
}
