//! Gruppi di foto simili senza alcun modello AI: solo impronta percettiva, data e SSIM.

use image::{codecs::jpeg::JpegEncoder, imageops, ImageEncoder, Rgb, RgbImage};
use pc_ai::similar::{self, global_candidates, params_key, regroup, HAMMING_NEAR_MAX};
use pc_governor::MemoryGovernor;
use pc_ingest::phash::{phash, to_sql};
use pc_ingest::thumb::ThumbCache;
use pc_jobs::JobControl;
use pc_store::model::MediaId;
use pc_store::{similar as db, Store};
use rusqlite::params;

/// Rumore deterministico (xorshift) in [0,1).
fn rng(seed: u64) -> impl FnMut() -> f64 {
    let mut s = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    move || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        (s >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Scena testurizzata (gradienti, cerchi, grana lieve): le immagini lisce hanno impronte instabili.
fn scene(seed: u64, w: u32, h: u32) -> RgbImage {
    let mut r = rng(seed);
    let (a, b, c) = (r(), r(), r());
    let mut grain = rng(seed ^ 0xabc);
    let mut img = RgbImage::from_fn(w, h, |x, y| {
        let (fx, fy) = (x as f64 / w as f64, y as f64 / h as f64);
        let g = (grain() - 0.5) * 8.0;
        let px = |v: f64| (255.0 * v + g).clamp(0.0, 255.0) as u8;
        Rgb([
            px(a * fx + (1.0 - a) * fy),
            px(b * fy + (1.0 - b) * (fx * 3.0).sin().abs()),
            px(c * (fx * fy) + (1.0 - c) * 0.3),
        ])
    });
    for _ in 0..6 {
        let (cx, cy, rad) = (r() * w as f64, r() * h as f64, 20.0 + r() * 60.0);
        let col = Rgb([
            (r() * 255.0) as u8,
            (r() * 255.0) as u8,
            (r() * 255.0) as u8,
        ]);
        for y in 0..h {
            for x in 0..w {
                let (dx, dy) = (x as f64 - cx, y as f64 - cy);
                if dx * dx + dy * dy < rad * rad {
                    img.put_pixel(x, y, col);
                }
            }
        }
    }
    img
}

/// Scatto `i` di una raffica: la stessa scena spostata di pochi pixel.
fn burst_frame(seed: u64, i: u32) -> RgbImage {
    let big = scene(seed, 336, 252);
    imageops::crop_imm(&big, i * 2, i, 320, 240).to_image()
}

/// Copia ridimensionata e ricompressa (come un'esportazione).
fn recompressed(img: &RgbImage) -> RgbImage {
    let small = imageops::resize(img, 240, 180, imageops::FilterType::Triangle);
    let mut buf = Vec::new();
    JpegEncoder::new_with_quality(&mut buf, 70)
        .write_image(
            small.as_raw(),
            small.width(),
            small.height(),
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();
    image::load_from_memory(&buf).unwrap().to_rgb8()
}

/// Dispositivo (se noto) e istante di uno scatto.
type Shot<'a> = Option<(Option<&'a str>, i64)>;

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

    /// Foto con miniatura e impronta (come dopo l'import), senza embedding.
    fn add(&self, id: MediaId, folder: &str, img: &RgbImage, shot: Shot) {
        let key = format!("{id:016x}");
        self.thumbs.store(&key, img).unwrap();
        let h = to_sql(phash(&image::DynamicImage::ImageRgb8(img.clone())));
        let device = shot.and_then(|s| s.0).map(str::to_string);
        let instant = shot.map(|s| s.1);
        let path = format!("D:\\Foto\\{folder}\\{id}.jpg");
        self.store
            .write(move |c| {
                c.execute(
                    "INSERT INTO media_item(id,path,name,ext,size,mtime,added_at,updated_at,thumb,phash,status)
                     VALUES (?1, ?2, ?2, 'jpg', 1, 0, 0, 0, ?3, ?4, 'ok')",
                    params![id, path, key, h],
                )?;
                c.execute(
                    "INSERT INTO media_dates(media_id, device, instant_utc) VALUES (?1, ?2, ?3)",
                    params![id, device, instant],
                )?;
                Ok(())
            })
            .unwrap();
    }

    fn run(&self) -> similar::RunOutcome {
        similar::run(
            &self.store,
            &self.thumbs,
            &MemoryGovernor::new(2048),
            &JobControl::new(),
            None,
            0.80,
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
fn burst_of_five_shots_is_one_group_without_any_model() {
    let lib = Lib::new();
    for i in 0..5 {
        let t = 1_000_000 + i as i64 * 600;
        lib.add(
            i as i64 + 1,
            "A",
            &burst_frame(1, i),
            Some((Some("X-T4"), t)),
        );
    }
    let out = lib.run();
    assert_eq!(out.processed, 5);
    assert_eq!(lib.group_of(1), Some((1..=5).collect()));
}

#[test]
fn different_scenes_two_seconds_apart_are_not_grouped() {
    let lib = Lib::new();
    lib.add(1, "A", &scene(10, 320, 240), Some((Some("X-T4"), 5_000)));
    lib.add(2, "A", &scene(11, 320, 240), Some((Some("X-T4"), 7_000)));
    let out = lib.run();
    assert_eq!(out.processed, 2);
    assert_eq!(lib.group_of(1), None);
    assert_eq!(lib.group_of(2), None);
}

#[test]
fn resized_recompressed_copy_without_date_in_another_folder_joins_the_group() {
    let lib = Lib::new();
    let original = scene(7, 320, 240);
    lib.add(1, "Vacanze", &original, None);
    lib.add(2, "Esportate\\web", &recompressed(&original), None);
    lib.add(3, "Altro", &scene(8, 320, 240), None);
    lib.run();
    assert_eq!(lib.group_of(1), Some(vec![1, 2]), "via impronta globale");
    assert_eq!(lib.group_of(3), None);
}

#[test]
fn shots_without_a_device_use_the_same_folder_and_time_window() {
    let lib = Lib::new();
    // Spostamento di 7 passi: impronta oltre 10 bit ma entro 22 (non è un candidato globale).
    lib.add(1, "A", &burst_frame(3, 0), Some((None, 100)));
    lib.add(2, "A", &burst_frame(3, 7), Some((None, 3_100)));
    lib.run();
    let ham = |a: MediaId, b: MediaId| -> u32 {
        let h = |id: MediaId| {
            lib.store
                .read(move |c| db::facts(c, &[id]))
                .unwrap()
                .remove(&id)
                .unwrap()
                .phash
                .unwrap()
        };
        (h(a) ^ h(b)).count_ones()
    };
    let d = ham(1, 2);
    assert!((11..=22).contains(&d), "{d}");
    assert!(
        lib.group_of(1).is_some_and(|g| g.contains(&2)),
        "stessa cartella (d = {d})"
    );
}

#[test]
fn user_corrections_survive_a_recompute_with_a_new_key() {
    let lib = Lib::new();
    for i in 0..5 {
        lib.add(
            i as i64 + 1,
            "A",
            &burst_frame(1, i),
            Some((Some("X"), i as i64 * 500)),
        );
    }
    lib.add(10, "B", &scene(20, 320, 240), None);
    lib.add(11, "B", &scene(21, 320, 240), None);
    lib.run();
    assert_eq!(lib.group_of(1).unwrap().len(), 5);
    let key = params_key(None);
    lib.store.write(|c| db::remove_from_group(c, 5)).unwrap();
    lib.store
        .write(|c| db::group_together(c, &[10, 11]))
        .unwrap();
    regroup(&lib.store, 0.80, &key).unwrap();
    let (g, _) = lib.store.read(|c| db::members_with(c, 1)).unwrap().unwrap();
    lib.store.write(move |c| db::set_cover(c, g, 3)).unwrap();
    // Nuova chiave: tutte le foto tornano in pending e si rielaborano.
    lib.store
        .write(|c| {
            Ok(
                c.execute("UPDATE similar_done SET params = 'v1;vecchia'", [])
                    .map(|_| ())?,
            )
        })
        .unwrap();
    let out = lib.run();
    assert_eq!(out.processed, 7);
    assert_eq!(lib.group_of(5), None, "tolta dal gruppo");
    assert_eq!(lib.group_of(1), Some(vec![1, 2, 3, 4]));
    assert_eq!(lib.group_of(10), Some(vec![10, 11]), "unione manuale");
    let v = lib.store.read(|c| db::group_view(c, 1)).unwrap().unwrap();
    assert_eq!((v.cover, v.cover_manual), (Some(3), true));
    // Gruppo sciolto: resta sciolto dopo un'altra rielaborazione completa.
    lib.store
        .write(move |c| db::dissolve(c, g).map(|_| ()))
        .unwrap();
    lib.store
        .write(|c| Ok(c.execute("DELETE FROM similar_done", []).map(|_| ())?))
        .unwrap();
    lib.run();
    for id in 1..=4 {
        assert_eq!(lib.group_of(id), None, "foto {id}");
    }
}

#[test]
fn global_candidates_exclude_self_and_keep_the_nearest_ones() {
    // 100 impronte identiche (immagini uniformi), una a distanza 3 e una lontana.
    let mut ids: Vec<MediaId> = (1..=100).collect();
    let mut hashes = vec![0u64; 100];
    ids.push(101);
    hashes.push(0b111);
    ids.push(102);
    hashes.push(u64::MAX);
    let mut scratch = Vec::new();
    let found = global_candidates(&ids, &hashes, 1, 0, &mut scratch);
    assert_eq!(found.len(), HAMMING_NEAR_MAX);
    assert!(!found.contains(&1) && !found.contains(&102));
    assert_eq!(found[0], 2, "a parità di distanza, id minore");
    let found = global_candidates(&ids, &hashes, 101, 0b111, &mut scratch);
    assert_eq!(found.len(), HAMMING_NEAR_MAX);
    assert!(!found.contains(&101));
}

/// Prestazioni: ricerca globale per 200 foto nuove su 100.000 impronte nel catalogo.
/// Eseguire con `cargo test -p pc-ai --release --test similar_phash -- --ignored --nocapture`.
/// Misurato (release, x86_64): vedi `PERF_NOTE` stampata; misura: caricamento 100k impronte ~15 ms, 200 ricerche ~4 ms (< 1 s).
#[test]
#[ignore = "prestazioni"]
fn perf_global_search_200_photos_over_100k_hashes_under_one_second() {
    let lib = Lib::new();
    let mut r = rng(99);
    let n: i64 = 100_000;
    lib.store
        .write(move |c| {
            let mut m = c.prepare(
                "INSERT INTO media_item(id,path,name,ext,size,mtime,added_at,updated_at,thumb,phash,status)
                 VALUES (?1, ?1, ?1, 'jpg', 1, 0, 0, 0, 'aaaaaaaa', ?2, 'ok')",
            )?;
            for id in 1..=n {
                let h = (r() * u64::MAX as f64) as u64;
                m.execute(params![id, to_sql(h)])?;
            }
            Ok(())
        })
        .unwrap();
    let started = std::time::Instant::now();
    let (ids, hashes) = lib.store.read(db::phashes).unwrap();
    let loaded = started.elapsed();
    assert_eq!(ids.len(), n as usize);
    let search = std::time::Instant::now();
    let mut scratch = Vec::new();
    let mut total = 0;
    for q in 0..200usize {
        total += global_candidates(
            &ids,
            &hashes,
            1_000_000 + q as i64,
            hashes[q * 400],
            &mut scratch,
        )
        .len();
    }
    let elapsed = search.elapsed();
    println!(
        "PERF_NOTE: caricamento 100k impronte {loaded:?}, 200 ricerche {elapsed:?}, candidati {total}"
    );
    assert!(total >= 200, "ogni ricerca trova almeno la foto stessa");
    assert!(elapsed < std::time::Duration::from_secs(1), "{elapsed:?}");
}
