//! Picco di memoria della miniatura di un JPEG da 50 MP, misurato con un
//! allocatore che conta i byte (binario separato: l'allocatore è globale).

use pc_governor::MemoryGovernor;
use pc_ingest::thumb::{make_thumbnail, ThumbCache};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Counting;
static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = System.alloc(l);
        if !p.is_null() {
            let now = CURRENT.fetch_add(l.size(), Ordering::SeqCst) + l.size();
            PEAK.fetch_max(now, Ordering::SeqCst);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        System.dealloc(p, l);
        CURRENT.fetch_sub(l.size(), Ordering::SeqCst);
    }
}

#[global_allocator]
static A: Counting = Counting;

const MB: usize = 1024 * 1024;

#[test]
fn thumbnail_of_50mp_jpeg_stays_small() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("50mp.jpg");
    {
        let (w, h) = (8660u32, 5774u32);
        let img = image::RgbImage::from_fn(w, h, |x, y| {
            image::Rgb([(x / 34) as u8, (y / 23) as u8, ((x ^ y) & 0xFF) as u8])
        });
        let mut out = std::io::BufWriter::new(std::fs::File::create(&path).unwrap());
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 80)
            .encode_image(&img)
            .unwrap();
    }
    let baseline = CURRENT.load(Ordering::SeqCst);
    PEAK.store(baseline, Ordering::SeqCst);

    let gov = MemoryGovernor::new(2048);
    let cache = ThumbCache::new(tmp.path().join("thumbs"));
    let (w, h) = make_thumbnail(&path, "jpg", None, "00112233445566778899", &cache, &gov).unwrap();
    assert_eq!(w.max(h), 512);

    let peak_mb = (PEAK.load(Ordering::SeqCst) - baseline) / MB;
    // A piena risoluzione servirebbero ~143 MB solo per i pixel RGB (8660×5774×3).
    eprintln!("picco memoria miniatura 50 MP: {peak_mb} MB");
    assert!(peak_mb < 40, "picco {peak_mb} MB");
    assert!(
        gov.snapshot().peak_mb <= 10,
        "costo stimato {} MB",
        gov.snapshot().peak_mb
    );
}
