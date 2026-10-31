//! Benchmark locale riproducibile, senza dati fotografici dell'utente.
//! cargo run -p pc-ai --example benchmark_detector -- yolox
//! cargo run -p pc-ai --example benchmark_detector -- rtdetr target/model-bench/rtdetr_r18vd.onnx
use pc_ai::{persons, tensor};
use pc_core::config::ExecutionProvider;
use pc_governor::{SysinfoProbe, SystemProbe};
use pc_models::runtime::{self, SessionOptions};
use std::{path::PathBuf, time::Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let detector = std::env::args().nth(1).unwrap_or_else(|| "yolox".into());
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    let path = if detector == "yolox" {
        root.join("models/yolox-tiny/yolox_tiny.onnx")
    } else {
        PathBuf::from(std::env::args().nth(2).expect("percorso ONNX RT-DETR"))
    };
    runtime::init(&runtime::dev_dll_path())?;
    let probe = SysinfoProbe::default();
    let baseline = probe.snapshot().process_mb;
    let (mut session, _) = runtime::load(
        &path,
        &SessionOptions {
            preferred: ExecutionProvider::Cpu,
            intra_threads: 4,
            ..Default::default()
        },
    )?;
    eprintln!("input: {:?}", session.inputs());
    let group = image::open(root.join("selftest/group_4_people.jpg"))?.to_rgb8();
    let land = image::open(root.join("selftest/landscape_mountain.jpg"))?.to_rgb8();
    let mut detect = |img: &image::RgbImage| -> Result<usize, Box<dyn std::error::Error>> {
        if detector == "yolox" {
            return Ok(persons::detect(&mut session, img, 0.4)?.len());
        }
        let resized = image::imageops::resize(img, 640, 640, image::imageops::FilterType::Triangle);
        let values = tensor::rgb_raw(&resized).mapv(|x| x / 255.0);
        let input = ort::value::Tensor::from_array(values)?;
        let out = session.run(ort::inputs!["pixel_values"=>input])?;
        let (shape, logits) = out["logits"].try_extract_tensor::<f32>()?;
        let classes = shape[2] as usize;
        Ok(logits
            .chunks_exact(classes)
            .filter(|row| {
                let best = row
                    .iter()
                    .enumerate()
                    .max_by(|a, b| a.1.total_cmp(b.1))
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                best == 0 && 1.0 / (1.0 + (-row[0]).exp()) >= 0.4
            })
            .count())
    };
    for _ in 0..2 {
        detect(&group)?;
    }
    let start = Instant::now();
    let mut group_count = 0;
    for _ in 0..5 {
        group_count = detect(&group)?;
    }
    let avg_ms = start.elapsed().as_secs_f64() * 1000.0 / 5.0;
    let land_count = detect(&land)?;
    println!(
        "{}",
        serde_json::json!({"detector":detector,"sha256":pc_models::download::sha256_file(&path)?,"bytes":std::fs::metadata(path)?.len(),"threads":4,"runs":5,"avg_ms":avg_ms,"group_count":group_count,"landscape_count":land_count,"rss_mb":probe.snapshot().process_mb,"baseline_mb":baseline})
    );
    Ok(())
}
