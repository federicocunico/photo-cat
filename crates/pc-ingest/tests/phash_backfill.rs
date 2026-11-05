//! Job `phash_backfill`: calcolo per le foto senza impronta, ripresa dopo
//! un'interruzione e accodamento una sola volta.

use image::{Rgb, RgbImage};
use pc_ingest::phash_backfill::{self, DONE_KEY, JOB_KIND, PHASE};
use pc_ingest::thumb::ThumbCache;
use pc_jobs::{
    persist, JobControl, JobManager, JobProgress, JobSnapshot, JobState, PhaseSpec, ProgressSink,
};
use pc_store::model::NewMedia;
use pc_store::{repo, Store};
use std::sync::Arc;
use std::time::Duration;

const N: i64 = 1200;

struct Sink;
impl ProgressSink for Sink {
    fn on_progress(&self, _: &JobSnapshot) {}
}

fn key(i: i64) -> String {
    format!("{:016x}", 0x1000 + i)
}

/// Miniatura sintetica 64x64 con motivo diverso per ogni `i`.
fn synthetic(i: i64) -> RgbImage {
    let i = i as u32;
    RgbImage::from_fn(64, 64, |x, y| {
        let v = ((x * (i % 7 + 1)) ^ (y * (i % 5 + 2)) ^ i) % 256;
        Rgb([v as u8, (v + i) as u8, (255 - v) as u8])
    })
}

struct Lib {
    _tmp: tempfile::TempDir,
    store: Store,
    thumbs: ThumbCache,
}

/// `n` foto con miniatura in cache e senza impronta; la numero `broken` (se c'è) senza file.
fn lib(n: i64, broken: Option<i64>) -> Lib {
    let tmp = tempfile::tempdir().unwrap();
    let thumbs = ThumbCache::new(tmp.path().join("thumbs"));
    let store = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
    for i in 1..=n {
        if Some(i) != broken {
            thumbs.store(&key(i), &synthetic(i)).unwrap();
        }
    }
    store
        .write(move |c| {
            for i in 1..=n {
                let id = repo::media::upsert(
                    c,
                    &NewMedia {
                        path: format!("D:/Foto/{i}.jpg"),
                        name: format!("{i}.jpg"),
                        ..Default::default()
                    },
                )?;
                repo::media::set_thumb(c, id, &key(i))?;
            }
            Ok(())
        })
        .unwrap();
    Lib {
        _tmp: tmp,
        store,
        thumbs,
    }
}

fn with_phash(store: &Store) -> i64 {
    store
        .read(|c| {
            Ok(c.query_row(
                "SELECT count(*) FROM media_item WHERE phash IS NOT NULL",
                [],
                |r| r.get(0),
            )?)
        })
        .unwrap()
}

fn manager(l: &Lib) -> JobManager {
    let jobs = JobManager::new(
        l.store.clone(),
        pc_governor::MemoryGovernor::new(256),
        1,
        Arc::new(Sink),
    );
    let (st, th) = (l.store.clone(), l.thumbs.clone());
    jobs.register(
        JOB_KIND,
        Arc::new(move |p| phash_backfill::plan(st.clone(), th.clone(), p)),
    );
    jobs
}

#[test]
fn all_photos_get_a_hash_and_it_matches_the_thumbnail() {
    let l = lib(N, None);
    assert_eq!(phash_backfill::pending(&l.store).unwrap().len() as i64, N);
    let jobs = manager(&l);
    assert!(phash_backfill::queue_once(&l.store, &jobs)
        .unwrap()
        .is_some());
    assert!(jobs.wait_idle(Duration::from_secs(120)));
    assert_eq!(with_phash(&l.store), N);
    assert!(phash_backfill::pending(&l.store).unwrap().is_empty());
    let h: i64 = l
        .store
        .read(|c| {
            Ok(
                c.query_row("SELECT phash FROM media_item WHERE id = 1", [], |r| {
                    r.get(0)
                })?,
            )
        })
        .unwrap();
    let img = image::open(l.thumbs.path_of(&key(1)).unwrap()).unwrap();
    assert_eq!(h, pc_ingest::phash::to_sql(pc_ingest::phash::phash(&img)));
}

#[test]
fn interrupted_run_resumes_with_only_the_missing_ones() {
    let l = lib(N, None);
    let progress = JobProgress::new(&[PhaseSpec::new(PHASE, 1.0)]);
    let phase = progress.phase(PHASE);
    let control = JobControl::new();
    let (c2, p2) = (control.clone(), phase.clone());
    let watcher = std::thread::spawn(move || {
        while p2.done() < 300 {
            std::thread::sleep(Duration::from_millis(1));
        }
        c2.cancel();
    });
    let first = phash_backfill::run(&l.store, &l.thumbs, 1, &control, &phase).unwrap();
    watcher.join().unwrap();
    let done = with_phash(&l.store);
    assert!(first.cancelled, "interrotto");
    assert!((300..N).contains(&done), "calcolate {done}");
    assert_eq!(phase.done(), done);

    let rest = N - done;
    assert_eq!(
        phash_backfill::pending(&l.store).unwrap().len() as i64,
        rest
    );
    let progress = JobProgress::new(&[PhaseSpec::new(PHASE, 1.0)]);
    let phase = progress.phase(PHASE);
    let second = phash_backfill::run(&l.store, &l.thumbs, 1, &JobControl::new(), &phase).unwrap();
    assert!(!second.cancelled);
    assert_eq!(phase.total(), rest, "solo le mancanti");
    assert_eq!(phase.done(), rest);
    assert_eq!(with_phash(&l.store), N);
}

#[test]
fn missing_thumbnail_is_a_non_blocking_per_file_failure() {
    let l = lib(10, Some(4));
    let progress = JobProgress::new(&[PhaseSpec::new(PHASE, 1.0)]);
    let phase = progress.phase(PHASE);
    let s = phash_backfill::run(&l.store, &l.thumbs, 1, &JobControl::new(), &phase).unwrap();
    assert!(!s.cancelled);
    assert_eq!((s.failed, phase.failed(), phase.done()), (1, 1, 9));
    assert_eq!(with_phash(&l.store), 9);
}

fn job_count(l: &Lib) -> i64 {
    l.store
        .read(|c| {
            Ok(c.query_row(
                "SELECT count(*) FROM job WHERE kind = ?1",
                [JOB_KIND],
                |r| r.get(0),
            )?)
        })
        .unwrap()
}

fn kv_count(l: &Lib, key: &'static str) -> i64 {
    l.store
        .read(move |c| {
            Ok(c.query_row(
                "SELECT count(*) FROM settings_kv WHERE key = ?1",
                [key],
                |r| r.get(0),
            )?)
        })
        .unwrap()
}

/// Riga `job` di un recupero precedente nello stato dato (come lasciata da una sessione passata).
fn past_job(l: &Lib, state: JobState) {
    let id = persist::create(&l.store, JOB_KIND, &serde_json::json!({}), &[]).unwrap();
    persist::set_state(&l.store, id, state).unwrap();
}

#[test]
fn not_queued_twice_while_one_is_active() {
    let l = lib(20, None);
    let jobs = manager(&l);
    assert!(phash_backfill::queue_once(&l.store, &jobs)
        .unwrap()
        .is_some());
    assert!(
        phash_backfill::queue_once(&l.store, &jobs)
            .unwrap()
            .is_none(),
        "già in coda o in corso"
    );
    assert!(jobs.wait_idle(Duration::from_secs(60)));
    assert_eq!(job_count(&l), 1);
    // Nessuna foto da completare: nemmeno su una libreria nuova.
    let empty = lib(0, None);
    assert!(phash_backfill::queue_once(&empty.store, &manager(&empty))
        .unwrap()
        .is_none());
}

#[test]
fn interrupted_job_blocks_queueing_but_a_cancelled_one_does_not() {
    let l = lib(20, None);
    past_job(&l, JobState::Interrupted);
    assert!(phash_backfill::queue_once(&l.store, &manager(&l))
        .unwrap()
        .is_none());
    let l = lib(20, None);
    past_job(&l, JobState::Cancelled);
    assert_eq!(kv_count(&l, DONE_KEY), 0, "annullato: nessun marcatore");
    let jobs = manager(&l);
    assert!(
        phash_backfill::queue_once(&l.store, &jobs)
            .unwrap()
            .is_some(),
        "riaccodato all'avvio dopo l'annullamento"
    );
    assert!(jobs.wait_idle(Duration::from_secs(60)));
}

#[test]
fn completion_writes_the_marker_even_with_failed_files_and_stops_queueing() {
    let l = lib(10, Some(4));
    let jobs = manager(&l);
    assert!(phash_backfill::queue_once(&l.store, &jobs)
        .unwrap()
        .is_some());
    assert!(jobs.wait_idle(Duration::from_secs(60)));
    assert_eq!(with_phash(&l.store), 9);
    assert_eq!(kv_count(&l, DONE_KEY), 1);
    // Una foto resta senza impronta, ma il recupero è concluso: non si riaccoda.
    assert_eq!(phash_backfill::pending(&l.store).unwrap().len(), 1);
    assert!(phash_backfill::queue_once(&l.store, &jobs)
        .unwrap()
        .is_none());
}

#[test]
fn legacy_queued_marker_is_not_treated_as_done() {
    let l = lib(5, None);
    l.store
        .write(|c| {
            c.execute(
                "INSERT INTO settings_kv(key, value) VALUES ('phash_backfill_queued', '7')",
                [],
            )?;
            Ok(())
        })
        .unwrap();
    let jobs = manager(&l);
    assert!(phash_backfill::queue_once(&l.store, &jobs)
        .unwrap()
        .is_some());
    assert_eq!(kv_count(&l, "phash_backfill_queued"), 0, "rimosso");
    assert!(jobs.wait_idle(Duration::from_secs(60)));
}
