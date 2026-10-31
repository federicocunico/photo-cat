//! Import end-to-end su una cartella di prova: metadati, duplicati, formati
//! non decodificabili, rescan incrementale, file spostati e mancanti, originali intatti.

use parking_lot::Mutex;
use pc_governor::MemoryGovernor;
use pc_ingest::import::{self, ImportDeps};
use pc_ingest::thumb::ThumbCache;
use pc_jobs::manager::{JobManager, JobSnapshot, ProgressSink};
use pc_jobs::JobState;
use pc_store::model::{DateSource, MediaStatus, NewSource};
use pc_store::{bulk, query, repo, Store, StoreOptions};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

#[derive(Default)]
struct Last(Mutex<BTreeMap<i64, JobSnapshot>>);
impl ProgressSink for Last {
    fn on_progress(&self, s: &JobSnapshot) {
        self.0.lock().insert(s.id, s.clone());
    }
}

fn jpeg(w: u32, h: u32, seed: u8, exif_date: Option<&str>) -> Vec<u8> {
    let img = image::RgbImage::from_fn(w, h, |x, y| {
        image::Rgb([seed, (x % 200) as u8, (y % 200) as u8])
    });
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 85)
        .encode_image(&img)
        .unwrap();
    let Some(date) = exif_date else { return out };
    let field = exif::Field {
        tag: exif::Tag::DateTimeOriginal,
        ifd_num: exif::In::PRIMARY,
        value: exif::Value::Ascii(vec![date.as_bytes().to_vec()]),
    };
    let mut w = exif::experimental::Writer::new();
    w.push_field(&field);
    let mut tiff = std::io::Cursor::new(Vec::new());
    w.write(&mut tiff, false).unwrap();
    let tiff = tiff.into_inner();
    let mut app1 = vec![0xFF, 0xE1];
    app1.extend_from_slice(&((tiff.len() + 8) as u16).to_be_bytes());
    app1.extend_from_slice(b"Exif\0\0");
    app1.extend_from_slice(&tiff);
    let mut res = out[..2].to_vec();
    res.extend(app1);
    res.extend_from_slice(&out[2..]);
    res
}

fn corrupt_exif_jpeg() -> Vec<u8> {
    let mut b = jpeg(80, 60, 9, Some("2022:01:01 10:00:00"));
    let i = b.windows(6).position(|w| w == b"Exif\0\0").unwrap() + 6;
    b[i] = b'X';
    b[i + 1] = b'X';
    b
}

struct Env {
    _tmp: tempfile::TempDir,
    root: PathBuf,
    store: Store,
    mgr: JobManager,
    sink: Arc<Last>,
    source_id: i64,
    thumbs: ThumbCache,
}

fn env() -> Env {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("Foto");
    let w = |rel: &str, bytes: &[u8]| {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, bytes).unwrap();
    };
    w(
        "2023/Toscana/IMG_0001.JPG",
        &jpeg(1200, 800, 1, Some("2023:07:14 10:22:01")),
    );
    w("2023/Toscana/IMG_0002.JPG", &jpeg(800, 1200, 2, None));
    w(
        "2023/Dolomiti/IMG_0003.jpg",
        &jpeg(640, 480, 3, Some("2023:08:01 09:00:00")),
    );
    w(
        "2023/Dolomiti/copia/IMG_0003.jpg",
        &jpeg(640, 480, 3, Some("2023:08:01 09:00:00")),
    );
    w("2022/scan.png", &{
        let mut v = Vec::new();
        image::RgbImage::from_pixel(300, 200, image::Rgb([5, 5, 5]))
            .write_to(&mut std::io::Cursor::new(&mut v), image::ImageFormat::Png)
            .unwrap();
        v
    });
    w(
        "2022/IMG_7701.HEIC",
        b"\0\0\0\x18ftypheic....non decodificabile",
    );
    w("2022/bad_exif.jpg", &corrupt_exif_jpeg());
    w("2022/rotto.jpg", b"\xFF\xD8\xFF\xE0 non un jpeg");
    w("note.txt", b"ignorato");

    let store = Store::open(&tmp.path().join("catalog.db"), StoreOptions::default()).unwrap();
    let source_id = store
        .write({
            let path = root.display().to_string();
            move |c| {
                repo::source::insert(
                    c,
                    &NewSource {
                        path,
                        recursive: true,
                        ..Default::default()
                    },
                )
            }
        })
        .unwrap();
    let sink = Arc::new(Last::default());
    let gov = MemoryGovernor::new(1024);
    let thumbs = ThumbCache::new(tmp.path().join("thumbs"));
    let mgr = JobManager::new(store.clone(), gov.clone(), 1, sink.clone());
    let deps = ImportDeps {
        store: store.clone(),
        thumbs: thumbs.clone(),
        governor: gov,
        extra_stages: vec![],
        stage_factory: None,
        io_workers: 3,
        decode_workers: 2,
    };
    mgr.register(
        import::JOB_KIND,
        Arc::new(move |p| import::plan(deps.clone(), p)),
    );
    Env {
        _tmp: tmp,
        root,
        store,
        mgr,
        sink,
        source_id,
        thumbs,
    }
}

fn run_import(e: &Env) -> JobSnapshot {
    let id = e
        .mgr
        .submit(
            import::JOB_KIND,
            serde_json::json!({ "sourceId": e.source_id }),
        )
        .unwrap();
    assert!(e.mgr.wait_idle(Duration::from_secs(60)));
    e.sink.0.lock()[&id].clone()
}

fn phase<'a>(s: &'a JobSnapshot, name: &str) -> &'a pc_jobs::PhaseSnapshot {
    s.phases.iter().find(|p| p.name == name).unwrap()
}

fn item(e: &Env, rel: &str) -> pc_store::model::MediaItem {
    let p = rel
        .split('/')
        .fold(e.root.clone(), |acc, part| acc.join(part))
        .display()
        .to_string();
    e.store
        .read(move |c| repo::media::get_by_path(c, &p))
        .unwrap()
        .unwrap()
}

fn fingerprint(root: &Path) -> BTreeMap<PathBuf, (String, std::time::SystemTime)> {
    walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .map(|e| {
            let p = e.path().to_path_buf();
            let h = blake3::hash(&std::fs::read(&p).unwrap())
                .to_hex()
                .to_string();
            (
                p.clone(),
                (h, std::fs::metadata(&p).unwrap().modified().unwrap()),
            )
        })
        .collect()
}

#[test]
fn full_import_incremental_rescan_moves_and_missing() {
    let e = env();
    let before = fingerprint(&e.root);

    // --- primo import
    let s = run_import(&e);
    assert_eq!(s.state, JobState::CompletedWithErrors, "{s:?}");
    assert_eq!(
        phase(&s, "scansione").done,
        8,
        "8 file supportati, .txt escluso"
    );
    // rotto.jpg fallisce nei metadati; nessun altro errore.
    assert_eq!(phase(&s, "metadati").failed, 1);
    assert_eq!(s.failed, 1);
    assert_eq!(e.store.read(repo::media::count).unwrap(), 7);

    // Caratteristiche tecniche e date.
    let a = item(&e, "2023/Toscana/IMG_0001.JPG");
    let tech = e
        .store
        .read(move |c| repo::meta::tech(c, a.id))
        .unwrap()
        .unwrap();
    assert_eq!(
        (tech.width, tech.height, tech.bit_depth),
        (Some(1200), Some(800), Some(8))
    );
    assert_eq!(tech.format.as_deref(), Some("JPEG"));
    let dates = e
        .store
        .read(move |c| repo::meta::dates(c, a.id))
        .unwrap()
        .unwrap();
    assert_eq!(dates.captured_at.as_deref(), Some("2023-07-14 10:22:01"));
    assert_eq!(dates.captured_source, Some(DateSource::ExifOriginal));
    let b = item(&e, "2023/Toscana/IMG_0002.JPG");
    let dates_b = e
        .store
        .read(move |c| repo::meta::dates(c, b.id))
        .unwrap()
        .unwrap();
    assert_eq!(dates_b.captured_source, Some(DateSource::FileSystem));
    assert!(e.thumbs.contains(a.thumb.as_deref().unwrap()));

    // EXIF corrotto: importato comunque. HEIC: solo metadati, senza miniatura.
    let bad = item(&e, "2022/bad_exif.jpg");
    assert!(bad.thumb.is_some());
    let heic = item(&e, "2022/IMG_7701.HEIC");
    assert!(heic.thumb.is_none());

    // Duplicati esatti.
    let d1 = item(&e, "2023/Dolomiti/IMG_0003.jpg");
    let d2 = item(&e, "2023/Dolomiti/copia/IMG_0003.jpg");
    assert_eq!(
        e.store
            .read(move |c| repo::media::duplicates_of(c, d1.id))
            .unwrap(),
        vec![d2.id]
    );
    let dups = query::LibraryFilter {
        duplicates_only: true,
        ..Default::default()
    };
    assert_eq!(e.store.read(move |c| query::count(c, &dups)).unwrap(), 2);

    // Originali intatti (contenuto e data di modifica).
    assert_eq!(
        fingerprint(&e.root),
        before,
        "gli originali non devono cambiare"
    );

    // --- rescan senza modifiche: nessun file rianalizzato
    let s2 = run_import(&e);
    assert_eq!(
        s2.state,
        JobState::CompletedWithErrors,
        "rotto.jpg fallisce di nuovo"
    );
    assert_eq!(phase(&s2, "miniature").total, 0, "{s2:?}");
    assert_eq!(phase(&s2, "scrittura").total, 0);

    // --- spostamento + cancellazione
    e.store
        .write(move |c| bulk::add_tag(c, "vacanze", &[a.id]).map(|_| ()))
        .unwrap();
    let from = e.root.join("2023/Toscana/IMG_0001.JPG");
    let to = e.root.join("2023/Archivio/IMG_0001.JPG");
    std::fs::create_dir_all(to.parent().unwrap()).unwrap();
    std::fs::rename(&from, &to).unwrap();
    std::fs::remove_file(e.root.join("2022/scan.png")).unwrap();
    let png_id = e.store.read(repo::media::count).unwrap(); // solo per forzare una lettura dopo le scritture
    assert_eq!(png_id, 7);

    let s3 = run_import(&e);
    assert_eq!(
        phase(&s3, "scrittura").total,
        1,
        "solo il file spostato viene rielaborato"
    );
    let moved = item(&e, "2023/Archivio/IMG_0001.JPG");
    assert_eq!(moved.id, a.id, "stesso elemento: catalogazioni conservate");
    assert_eq!(moved.status, MediaStatus::Ok);
    assert_eq!(
        e.store.read(move |c| repo::tag::of_media(c, a.id)).unwrap(),
        vec!["vacanze"]
    );
    let png = item(&e, "2022/scan.png");
    assert_eq!(
        png.status,
        MediaStatus::Missing,
        "file cancellato marcato mancante, non rimosso"
    );
    assert_eq!(e.store.read(repo::media::count).unwrap(), 7);
}

#[test]
fn watched_source_imports_new_files_automatically() {
    let e = env();
    run_import(&e);
    let before = e.store.read(repo::media::count).unwrap();

    let mgr = e.mgr.clone();
    let source_id = e.source_id;
    let _watcher = pc_ingest::watch::SourceWatcher::start(
        vec![(e.source_id, e.root.clone(), true)],
        Duration::from_millis(300),
        move |id| {
            assert_eq!(id, source_id);
            mgr.submit(import::JOB_KIND, serde_json::json!({ "sourceId": id }))
                .unwrap();
        },
    )
    .unwrap();
    std::thread::sleep(Duration::from_millis(200));
    std::fs::write(
        e.root.join("2023").join("nuova.jpg"),
        jpeg(320, 240, 77, None),
    )
    .unwrap();

    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    loop {
        e.mgr.wait_idle(Duration::from_secs(5));
        let p = e.root.join("2023").join("nuova.jpg").display().to_string();
        if e.store
            .read(move |c| repo::media::get_by_path(c, &p))
            .unwrap()
            .is_some()
        {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "il nuovo file non è stato importato"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
    assert_eq!(e.store.read(repo::media::count).unwrap(), before + 1);
}
