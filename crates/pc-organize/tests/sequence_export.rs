//! Job `sequence_export` su una sequenza di 10 immagini (task 7.4 e 7.5).
use pc_jobs::{JobManager, JobSnapshot, JobState, ProgressSink};
use pc_organize::sequence::job::{
    self, ExportContext, ExportFormat, SequenceExportOptions, JOB_KIND,
};
use pc_organize::sequence::{encode::JpegSettings, metadata::read_xmp_lists};
use pc_store::model::{DateSource, MediaDates, NewMedia, TechInfo};
use pc_store::sequence::SequenceScope;
use pc_store::{bulk, repo, Store};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

struct Sink;
impl ProgressSink for Sink {
    fn on_progress(&self, _: &JobSnapshot) {}
}

fn sys(_: chrono::NaiveDateTime) -> i32 {
    0
}

/// Libreria con `n` foto di due dispositivi, a un minuto l'una dall'altra.
fn library(root: &Path, n: usize, side: u32) -> (Store, Vec<PathBuf>) {
    let src = root.join("originali");
    std::fs::create_dir_all(&src).unwrap();
    let db = Store::open(&root.join("catalog.db"), Default::default()).unwrap();
    let mut files = Vec::new();
    for i in 0..n {
        let p = src.join(format!("IMG_{i:04}.jpg"));
        let img = image::RgbImage::from_fn(side, side * 3 / 4, |x, y| {
            image::Rgb([
                (x * 7 + i as u32 * 13) as u8,
                (y * 5) as u8,
                ((x ^ y) & 0xff) as u8,
            ])
        });
        img.save(&p).unwrap();
        files.push(p);
    }
    let paths = files.clone();
    db.write(move |c| {
        for (i, p) in paths.iter().enumerate() {
            let id = repo::media::upsert(
                c,
                &NewMedia {
                    path: p.display().to_string(),
                    name: p.file_name().unwrap().to_string_lossy().to_string(),
                    ext: "jpg".into(),
                    size: std::fs::metadata(p).unwrap().len() as i64,
                    ..Default::default()
                },
            )?;
            repo::meta::set_tech(
                c,
                id,
                &TechInfo {
                    width: Some(side),
                    height: Some(side * 3 / 4),
                    ..Default::default()
                },
            )?;
            repo::meta::set_dates_with(
                c,
                id,
                &MediaDates {
                    captured_at: Some(format!("2026-08-19 10:{i:02}:00")),
                    captured_source: Some(DateSource::ExifOriginal),
                    declared_offset_min: Some(-360),
                    device: Some(
                        if i % 2 == 0 {
                            "Fujifilm X-T5"
                        } else {
                            "Apple iPhone 17 Pro Max"
                        }
                        .into(),
                    ),
                    ..Default::default()
                },
                &sys,
            )?;
        }
        Ok(())
    })
    .unwrap();
    (db, files)
}

fn manager(db: &Store) -> JobManager {
    let gov = pc_governor::MemoryGovernor::new(1024);
    let jobs = JobManager::new(db.clone(), gov.clone(), 1, Arc::new(Sink));
    let st = db.clone();
    jobs.register(
        JOB_KIND,
        Arc::new(move |p| job::plan(st.clone(), gov.clone(), p)),
    );
    jobs
}

fn options(dest: &Path) -> SequenceExportOptions {
    SequenceExportOptions {
        scope: SequenceScope::Library,
        template: "{seq:04d}_{date}_{time}_{tag}".into(),
        destination: dest.display().to_string(),
        format: ExportFormat::Jpeg,
        jpeg: JpegSettings::default(),
        per_tag: true,
        manifest: true,
        zip: true,
        zip_per_tag: true,
        metadata: true,
        tag_set: Some(2),
    }
}

fn context() -> ExportContext {
    ExportContext {
        set_tags: vec!["Paesaggi".into(), "Città".into()],
        keyword_tags: vec!["Paesaggi".into(), "Città".into()],
        identities: HashMap::from([(
            "u1".to_string(),
            ("Giulia Bianchi".to_string(), "Giulia".to_string()),
        )]),
    }
}

fn fingerprint(files: &[PathBuf]) -> Vec<(Vec<u8>, std::time::SystemTime)> {
    files
        .iter()
        .map(|p| {
            (
                std::fs::read(p).unwrap(),
                std::fs::metadata(p).unwrap().modified().unwrap(),
            )
        })
        .collect()
}

fn list(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    v.sort();
    v
}

#[test]
fn ten_photos_with_every_option() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, files) = library(tmp.path(), 10, 320);
    db.write(|c| {
        bulk::add_tag(c, "Paesaggi", &[1, 2, 3, 4], &[])?;
        bulk::add_tag(c, "Città", &[5, 6, 7, 8], &[])?;
        bulk::add_tag(c, "Altro", &[1], &[])?;
        let p = pc_store::people::create_person(c, Some("Giulia"))?;
        c.execute("UPDATE person SET identity_uid = 'u1' WHERE id = ?1", [p])?;
        c.execute(
            "INSERT INTO face(media_id, bbox, quality, person_id) VALUES (1, '[0.1,0.1,0.2,0.2]', 0.9, ?1)",
            [p],
        )?;
        Ok(())
    })
    .unwrap();
    let before = fingerprint(&files);
    let out = tmp.path().join("export");
    let (opts, ctx) = (options(&out), context());

    let pre = job::preflight(&db, &opts, &ctx).unwrap();
    assert!(pre.can_start, "{:?}", pre.blocked);
    assert_eq!((pre.count, pre.untagged), (10, 2));
    assert_eq!(pre.preview[0].name, "0001_2026-08-19_1000_Paesaggi.jpg");
    assert_eq!(pre.preview[0].original, "IMG_0000.jpg");
    assert!(pre.required_bytes > 0);

    let jobs = manager(&db);
    let id = jobs.submit(JOB_KIND, job::job_params(&opts, &ctx)).unwrap();
    assert!(jobs.wait_idle(Duration::from_secs(120)));
    let s = jobs.snapshot(id).unwrap();
    assert_eq!(s.state, JobState::Completed, "{s:?}");

    assert_eq!(
        list(&out),
        vec![
            "manifest.csv",
            "per-tag",
            "per-tag.zip",
            "sequenza",
            "sequenza.zip"
        ]
    );
    let seq = list(&out.join("sequenza"));
    assert_eq!(seq.len(), 10);
    assert_eq!(seq[0], "0001_2026-08-19_1000_Paesaggi.jpg");
    assert_eq!(seq[4], "0005_2026-08-19_1004_Città.jpg");
    assert_eq!(seq[8], "0009_2026-08-19_1008.jpg");
    assert_eq!(list(&out.join("per-tag")), vec!["Città", "Paesaggi"]);
    assert_eq!(
        list(&out.join("per-tag/Paesaggi")).len() + list(&out.join("per-tag/Città")).len(),
        8
    );

    // Manifest: intestazione e una riga per foto.
    let manifest = std::fs::read_to_string(out.join("manifest.csv")).unwrap();
    let lines: Vec<&str> = manifest.trim_start_matches('\u{feff}').lines().collect();
    assert_eq!(lines.len(), 11);
    assert!(lines[0].starts_with("nome;percorso_originale;istante_utc;ora_locale"));
    let first: Vec<&str> = lines[1].split(';').collect();
    assert_eq!(first[0], "0001_2026-08-19_1000_Paesaggi.jpg");
    assert_eq!(first[2], "2026-08-19T16:00:00Z");
    assert_eq!(first[3], "2026-08-19 10:00:00 -06:00");
    assert_eq!(first[4], "Fujifilm X-T5");
    assert_eq!(first[5], "Paesaggi");
    assert_eq!(first[6], "Giulia Bianchi");
    assert!(first[7].parse::<u8>().unwrap() >= 97);
    assert_eq!(first[10], "ok");

    // Zip leggibili, non compressi.
    for (zip_name, count) in [("sequenza.zip", 10), ("per-tag.zip", 8)] {
        let mut z = zip::ZipArchive::new(std::fs::File::open(out.join(zip_name)).unwrap()).unwrap();
        assert_eq!(z.len(), count, "{zip_name}");
        let mut e = z.by_index(0).unwrap();
        assert_eq!(e.compression(), zip::CompressionMethod::Stored);
        let mut buf = Vec::new();
        e.read_to_end(&mut buf).unwrap();
        assert!(image::load_from_memory(&buf).is_ok());
    }
    let mut z =
        zip::ZipArchive::new(std::fs::File::open(out.join("per-tag.zip")).unwrap()).unwrap();
    assert!(z
        .by_name("Paesaggi/0001_2026-08-19_1000_Paesaggi.jpg")
        .is_ok());

    // Metadati della copia: data e fuso, parole chiave del set e identità.
    let copy = out.join("sequenza").join(&seq[0]);
    let e = pc_ingest::metadata::read_exif(&copy).unwrap().unwrap();
    assert_eq!(e.date_original.as_deref(), Some("2026-08-19 10:00:00"));
    assert_eq!(e.offset_original, Some(-360));
    assert_eq!(e.orientation, Some(1));
    let (keywords, people) = read_xmp_lists(&std::fs::read(&copy).unwrap());
    assert_eq!(
        keywords,
        vec!["Paesaggi"],
        "solo i tag dei set con parole chiave"
    );
    assert_eq!(people, vec!["Giulia Bianchi"]);
    let decoded = image::open(&copy).unwrap();
    assert_eq!((decoded.width(), decoded.height()), (320, 240));

    // Originali intatti, nessuna cartella temporanea.
    assert_eq!(fingerprint(&files), before);
    assert!(list(&out)
        .iter()
        .all(|n| !n.starts_with(".photo-cat-export")));

    // Destinazione già usata: l'avvio è bloccato.
    let again = job::preflight(&db, &opts, &ctx).unwrap();
    assert!(!again.can_start);
    assert!(again.existing.contains(&"sequenza".to_string()));
    assert!(again.blocked.unwrap().contains("sequenza"));
}

#[test]
fn unknown_token_blocks_the_start() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _) = library(tmp.path(), 2, 64);
    let mut opts = options(&tmp.path().join("out"));
    opts.template = "{seq}_{camera}".into();
    let pre = job::preflight(&db, &opts, &context()).unwrap();
    assert!(!pre.can_start);
    assert_eq!(pre.template_errors[0].text, "{camera}");
    let jobs = manager(&db);
    assert!(jobs
        .submit(JOB_KIND, job::job_params(&opts, &context()))
        .is_err());
}

#[test]
fn cancelled_export_leaves_nothing_in_the_destination() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, files) = library(tmp.path(), 10, 1600);
    let before = fingerprint(&files);
    let out = tmp.path().join("export");
    std::fs::create_dir_all(&out).unwrap();
    let mut opts = options(&out);
    opts.per_tag = false;
    let jobs = manager(&db);
    let id = jobs
        .submit(JOB_KIND, job::job_params(&opts, &context()))
        .unwrap();
    // Attende che la codifica sia iniziata, poi annulla.
    let until = Instant::now() + Duration::from_secs(60);
    loop {
        let s = jobs.snapshot(id).unwrap();
        let encoding = s
            .phases
            .iter()
            .any(|p| p.name == job::PHASE_ENCODE && p.done >= 1);
        if encoding || s.state.is_terminal() {
            break;
        }
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(5));
    }
    jobs.cancel(id).unwrap();
    assert!(jobs.wait_idle(Duration::from_secs(120)));
    let s = jobs.snapshot(id).unwrap();
    assert_eq!(s.state, JobState::Cancelled, "{s:?}");
    assert!(list(&out).is_empty(), "{:?}", list(&out));
    assert_eq!(fingerprint(&files), before);
}
