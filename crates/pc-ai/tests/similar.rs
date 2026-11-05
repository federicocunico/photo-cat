//! Gruppi di foto simili su un catalogo sintetico: embedding, miniature e date costruiti qui.

use image::{imageops, RgbImage};
use pc_ai::similar::{self, compute_groups, params_key, preview_count, regroup};
use pc_governor::MemoryGovernor;
use pc_ingest::thumb::ThumbCache;
use pc_jobs::{JobControl, JobProgress, PhaseSpec};
use pc_store::model::MediaId;
use pc_store::{similar as db, Store};
use rusqlite::params;
use std::time::Duration;

const MODEL: &str = "clip-test:1";

/// Scena sintetica con forme ampie (`seed` cambia soggetto).
fn scene(w: u32, h: u32, seed: u32) -> RgbImage {
    RgbImage::from_fn(w, h, |x, y| {
        let (fx, fy) = (x as f32 / w as f32, y as f32 / h as f32);
        let v = match seed % 3 {
            0 => {
                let circle = ((fx - 0.4).powi(2) + (fy - 0.5).powi(2)).sqrt() < 0.25;
                let stripes = ((x / 24 + y / 36) % 2) as f32 * 40.0;
                if circle {
                    220.0 - stripes
                } else {
                    60.0 + 80.0 * fx + stripes
                }
            }
            1 => {
                let rect = fx > 0.55 && fx < 0.9 && fy > 0.1 && fy < 0.7;
                let checks = (((x / 44) + (y / 44)) % 2) as f32 * 90.0;
                if rect {
                    30.0 + checks
                } else {
                    200.0 - 120.0 * fy
                }
            }
            _ => 128.0 + 100.0 * ((fx * 23.0).sin() * (fy * 17.0).cos()),
        };
        let c = v.clamp(0.0, 255.0) as u8;
        image::Rgb([c, c.saturating_sub(10), c.saturating_add(10)])
    })
}

/// Fotogramma `i` di una raffica: la stessa scena spostata di qualche pixel.
fn burst_frame(seed: u32, i: u32) -> RgbImage {
    let big = scene(560, 420, seed);
    imageops::crop_imm(&big, i * 3, i * 2, 512, 384).to_image()
}

/// Embedding sintetico: direzione `base` più una piccola variazione `jitter`.
fn embedding(base: usize, jitter: f32) -> Vec<f32> {
    let mut v = vec![0.01f32; 64];
    v[base % 64] = 1.0;
    v[(base + 1) % 64] = jitter;
    v
}

struct Lib {
    _tmp: tempfile::TempDir,
    store: Store,
    thumbs: ThumbCache,
}

impl Lib {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
        let thumbs = ThumbCache::new(tmp.path().join("thumbs"));
        Self {
            _tmp: tmp,
            store,
            thumbs,
        }
    }

    /// Aggiunge una foto con miniatura, embedding e (facoltativi) dispositivo e istante.
    fn add(
        &self,
        id: MediaId,
        img: &RgbImage,
        emb: Vec<f32>,
        shot: Option<(&str, i64)>,
        hash: Option<&str>,
    ) {
        let key = format!("{id:016x}");
        self.thumbs.store(&key, img).unwrap();
        let blob: Vec<u8> = emb.iter().flat_map(|f| f.to_le_bytes()).collect();
        let (device, instant) = (shot.map(|s| s.0.to_string()), shot.map(|s| s.1));
        let hash = hash.map(str::to_string);
        self.store
            .write(move |c| {
                c.execute(
                    "INSERT INTO media_item(id,path,name,ext,size,mtime,added_at,updated_at,thumb,content_hash,status)
                     VALUES (?1, ?2, ?2, 'jpg', 1, 0, 0, 0, ?3, ?4, 'ok')",
                    params![id, format!("D:/Foto/{id}.jpg"), key, hash],
                )?;
                c.execute(
                    "INSERT INTO embedding(media_id, model_ver, vector) VALUES (?1, ?2, ?3)",
                    params![id, MODEL, blob],
                )?;
                c.execute(
                    "INSERT INTO media_dates(media_id, device, instant_utc) VALUES (?1, ?2, ?3)",
                    params![id, device, instant],
                )?;
                Ok(())
            })
            .unwrap();
    }

    fn run(&self, threshold: f32) -> similar::RunOutcome {
        similar::run(
            &self.store,
            &self.thumbs,
            &MemoryGovernor::new(2048),
            &JobControl::new(),
            Some(MODEL),
            threshold,
            None,
        )
        .unwrap()
    }

    fn group_of(&self, id: MediaId) -> Option<Vec<MediaId>> {
        self.store
            .read(move |c| db::members_with(c, id))
            .unwrap()
            .map(|g| g.1)
    }
}

#[test]
fn burst_is_one_group_but_same_place_from_another_angle_is_not() {
    let lib = Lib::new();
    // Raffica: 8 scatti in 3 secondi, stesso dispositivo.
    for i in 0..8 {
        lib.add(
            i + 1,
            &burst_frame(0, i as u32),
            embedding(1, 0.05 * i as f32),
            Some(("X-T4", 1_000_000 + i * 400)),
            None,
        );
    }
    // Stesso monumento, altra inquadratura: CLIP molto vicino, struttura diversa.
    lib.add(20, &scene(512, 384, 1), embedding(5, 0.0), None, None);
    lib.add(21, &scene(512, 384, 2), embedding(5, 0.02), None, None);
    let out = lib.run(0.80);
    assert_eq!(out.processed, 10);
    assert_eq!(lib.group_of(1), Some((1..=8).collect()), "raffica");
    assert_eq!(lib.group_of(20), None, "inquadrature diverse");
    assert_eq!(lib.group_of(21), None);
}

#[test]
fn photos_without_date_are_grouped_by_content_and_exact_copies_always() {
    let lib = Lib::new();
    lib.add(1, &burst_frame(2, 0), embedding(3, 0.0), None, None);
    lib.add(2, &burst_frame(2, 1), embedding(3, 0.01), None, None);
    // Copie esatte con embedding lontani (es. modello diverso): unite comunque.
    lib.add(3, &scene(512, 384, 1), embedding(9, 0.0), None, Some("abc"));
    lib.add(
        4,
        &scene(512, 384, 1),
        embedding(30, 0.0),
        None,
        Some("abc"),
    );
    lib.run(0.80);
    assert_eq!(lib.group_of(1), Some(vec![1, 2]), "senza data");
    assert_eq!(lib.group_of(3), Some(vec![3, 4]), "copie esatte");
}

#[test]
fn sharpest_photo_is_proposed_and_user_choice_survives_reanalysis() {
    let lib = Lib::new();
    let sharp = burst_frame(0, 0);
    let blurred = imageops::blur(&sharp, 1.2);
    lib.add(1, &blurred, embedding(1, 0.0), Some(("A", 0)), None);
    lib.add(2, &sharp, embedding(1, 0.01), Some(("A", 500)), None);
    lib.run(0.70);
    let v = lib
        .store
        .read(|c| db::group_view(c, 1))
        .unwrap()
        .expect("gruppo");
    assert_eq!(v.cover, Some(2), "principale proposta: la più nitida");
    let gid = v.id;
    lib.store.write(move |c| db::set_cover(c, gid, 1)).unwrap();
    // Nuova analisi (parametri invariati) e ricalcolo dei gruppi.
    lib.run(0.70);
    regroup(&lib.store, 0.70, &params_key(Some(MODEL))).unwrap();
    let v = lib.store.read(|c| db::group_view(c, 1)).unwrap().unwrap();
    assert_eq!(v.cover, Some(1), "scelta dell'utente");
}

#[test]
fn corrections_are_respected_by_later_analyses() {
    let lib = Lib::new();
    for i in 0..5 {
        lib.add(
            i + 1,
            &burst_frame(0, i as u32),
            embedding(1, 0.0),
            Some(("A", i * 300)),
            None,
        );
    }
    for i in 0..3 {
        lib.add(
            10 + i,
            &burst_frame(1, 0),
            embedding(40 + i as usize * 7, 0.0),
            None,
            None,
        );
    }
    lib.run(0.80);
    assert_eq!(lib.group_of(1).unwrap().len(), 5);
    let key = params_key(Some(MODEL));
    // Foto tolta da un gruppo.
    lib.store.write(|c| db::remove_from_group(c, 5)).unwrap();
    regroup(&lib.store, 0.80, &key).unwrap();
    assert_eq!(lib.group_of(5), None);
    // Unione manuale di 3 foto non raggruppate (embedding lontani).
    lib.store
        .write(|c| db::group_together(c, &[10, 11, 12]))
        .unwrap();
    regroup(&lib.store, 0.80, &key).unwrap();
    assert_eq!(lib.group_of(10), Some(vec![10, 11, 12]));
    // Gruppo sciolto.
    let (g, _) = lib.store.read(|c| db::members_with(c, 1)).unwrap().unwrap();
    lib.store
        .write(move |c| db::dissolve(c, g).map(|_| ()))
        .unwrap();
    regroup(&lib.store, 0.80, &key).unwrap();
    // Nuova analisi completa: le correzioni restano.
    lib.store
        .write(|c| Ok(c.execute("DELETE FROM similar_done", []).map(|_| ())?))
        .unwrap();
    lib.run(0.80);
    for id in 1..=5 {
        assert_eq!(lib.group_of(id), None, "foto {id} dopo lo scioglimento");
    }
    assert_eq!(lib.group_of(10), Some(vec![10, 11, 12]));
}

#[test]
fn new_import_compares_only_new_photos_and_keeps_other_groups() {
    let lib = Lib::new();
    for i in 0..4 {
        lib.add(
            i + 1,
            &burst_frame(0, i as u32),
            embedding(1, 0.0),
            Some(("A", i * 300)),
            None,
        );
    }
    for i in 0..3 {
        lib.add(
            10 + i,
            &burst_frame(2, i as u32),
            embedding(20, 0.0),
            Some(("B", i * 300)),
            None,
        );
    }
    let first = lib.run(0.80);
    assert_eq!(first.groups.created, 2);
    // Due foto nuove della prima raffica.
    for i in 4..6 {
        lib.add(
            i + 1,
            &burst_frame(0, i as u32),
            embedding(1, 0.0),
            Some(("A", i * 300)),
            None,
        );
    }
    let second = lib.run(0.80);
    assert_eq!(second.processed, 2, "solo le foto nuove");
    assert!(second.comparisons <= 2 * 12, "{}", second.comparisons);
    assert_eq!((second.groups.updated, second.groups.unchanged), (1, 1));
    assert_eq!(lib.group_of(1).unwrap().len(), 6);
}

#[test]
fn interrupted_analysis_resumes_with_the_remaining_photos() {
    let lib = Lib::new();
    let n = 60;
    for i in 0..n {
        lib.add(
            i + 1,
            &burst_frame(i as u32 % 3, (i % 7) as u32),
            embedding(i as usize, 0.0),
            None,
            None,
        );
    }
    let progress = JobProgress::new(&[PhaseSpec::new("similar", 1.0)]);
    let phase = progress.phase("similar");
    let control = JobControl::new();
    let (c2, p2) = (control.clone(), phase.clone());
    let watcher = std::thread::spawn(move || {
        while p2.done() < 15 {
            std::thread::sleep(Duration::from_millis(1));
        }
        c2.cancel();
    });
    let res = similar::run(
        &lib.store,
        &lib.thumbs,
        &MemoryGovernor::new(2048),
        &control,
        Some(MODEL),
        0.8,
        Some(&phase),
    );
    watcher.join().unwrap();
    assert!(res.is_err(), "interrotta");
    let key = params_key(Some(MODEL));
    let left = lib
        .store
        .read(move |c| db::pending(c, Some(MODEL), &key, &params_key(None)))
        .unwrap()
        .len() as i64;
    assert!(left < n && left > 0, "rimaste {left}");
    let resumed = lib.run(0.8);
    assert_eq!(
        resumed.processed as i64, left,
        "solo le foto non completate"
    );
}

#[test]
fn stricter_threshold_regroups_from_saved_edges_without_thumbnails() {
    let lib = Lib::new();
    for i in 0..6 {
        lib.add(
            i + 1,
            &burst_frame(0, i as u32),
            embedding(1, 0.0),
            Some(("A", i * 300)),
            None,
        );
    }
    lib.run(0.70);
    let loose = lib.group_of(1).unwrap().len();
    // Senza miniature: il ricalcolo usa solo gli archi salvati.
    std::fs::remove_dir_all(
        lib.thumbs
            .path_of("0000000000000001")
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap(),
    )
    .unwrap();
    let edges = lib.store.read(db::edges).unwrap();
    let strict = edges.iter().map(|e| e.2).fold(0.0f32, f32::max) - 0.001;
    let preview = preview_count(&lib.store, strict).unwrap();
    regroup(&lib.store, strict, &params_key(Some(MODEL))).unwrap();
    let after = compute_groups(&lib.store, strict).unwrap();
    assert_eq!(after.len(), preview, "anteprima = esito");
    let saved = lib.store.read(db::group_count).unwrap() as usize;
    assert_eq!(saved, preview);
    let total_after: usize = after.iter().map(Vec::len).sum();
    assert!(
        total_after < loose,
        "le foto meno simili escono: {total_after} < {loose}"
    );
}

/// 100.000 embedding: l'indice dei vicini si costruisce entro il budget e la memoria torna libera.
#[test]
fn hundred_thousand_embeddings_within_the_governor_budget() {
    let lib = Lib::new();
    let n: i64 = 100_000;
    lib.store
        .write(move |c| {
            let mut m = c.prepare(
                "INSERT INTO media_item(id,path,name,ext,size,mtime,added_at,updated_at,thumb,status)
                 VALUES (?1, ?1, ?1, 'jpg', 1, 0, 0, 0, 'aaaaaaaaaaaaaaaa', 'ok')",
            )?;
            let mut e = c.prepare(
                "INSERT INTO embedding(media_id, model_ver, vector) VALUES (?1, ?2, ?3)",
            )?;
            for id in 1..=n {
                m.execute([id])?;
                // 32 dimensioni, raggruppate a 5 a 5 (vicini molto simili tra loro).
                let base = (id / 5) as usize;
                let v: Vec<f32> = (0..32)
                    .map(|d| if d == base % 32 || d == (base / 32) % 32 { 1.0 } else { 0.001 * (id % 5) as f32 })
                    .collect();
                let blob: Vec<u8> = v.iter().flat_map(|f| f.to_le_bytes()).collect();
                e.execute(params![id, MODEL, blob])?;
            }
            Ok(())
        })
        .unwrap();
    let gov = MemoryGovernor::new(4096);
    let targets: Vec<MediaId> = (1..=3_000).collect();
    let started = std::time::Instant::now();
    let found = similar::semantic_neighbours(&lib.store, &gov, MODEL, &targets, &JobControl::new())
        .unwrap();
    let elapsed = started.elapsed();
    assert_eq!(found.len(), targets.len());
    assert!(found.values().filter(|v| !v.is_empty()).count() > 2_500);
    assert!(gov.snapshot().peak_mb <= 4096, "entro il budget");
    assert_eq!(
        gov.snapshot().in_use_mb,
        0,
        "memoria dell'indice restituita"
    );
    assert!(elapsed < Duration::from_secs(180), "{elapsed:?}");
}

#[test]
fn photo_processed_before_its_embedding_is_reprocessed_when_it_gets_one() {
    let lib = Lib::new();
    lib.add(1, &burst_frame(2, 0), embedding(3, 0.0), None, None);
    lib.add(2, &burst_frame(2, 1), embedding(3, 0.01), None, None);
    // La 2 non ha ancora l'embedding; impronte a 20 bit: né candidato globale né scartato.
    lib.store
        .write(|c| {
            c.execute("DELETE FROM embedding WHERE media_id = 2", [])?;
            c.execute("UPDATE media_item SET phash = 0 WHERE id = 1", [])?;
            c.execute("UPDATE media_item SET phash = 1048575 WHERE id = 2", [])?;
            Ok(())
        })
        .unwrap();
    assert_eq!(lib.run(0.8).processed, 2);
    assert_eq!(
        lib.group_of(1),
        None,
        "senza embedding nessun vicino semantico"
    );
    // Arriva l'embedding (analisi successiva): la 2 torna da elaborare e trova la 1.
    let blob: Vec<u8> = embedding(3, 0.01)
        .iter()
        .flat_map(|f| f.to_le_bytes())
        .collect();
    lib.store
        .write(move |c| {
            c.execute(
                "INSERT INTO embedding(media_id, model_ver, vector) VALUES (2, ?1, ?2)",
                params![MODEL, blob],
            )?;
            Ok(())
        })
        .unwrap();
    assert_eq!(lib.run(0.8).processed, 1);
    assert_eq!(lib.group_of(1), Some(vec![1, 2]));
}
