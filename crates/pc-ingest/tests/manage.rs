//! Gestione delle importazioni: rimozione completa, riepilogo dell'aggiornamento,
//! aggiornamento di una cartella, rianalisi dei file modificati, ricollegamento
//! e rimozione dei mancanti. I file originali non vengono mai toccati.

use parking_lot::Mutex;
use pc_core::ErrorCode;
use pc_governor::MemoryGovernor;
use pc_ingest::import::{self, ImportDeps, ImportItem, ImportStage};
use pc_ingest::thumb::ThumbCache;
use pc_ingest::{manage, relink};
use pc_jobs::manager::{JobManager, JobSnapshot, ProgressSink};
use pc_jobs::{JobState, PhaseSpec, StageCtx, StageOutcome};
use pc_store::model::{CatalogKind, MediaId, MediaStatus, NewSource};
use pc_store::query::{self, LibraryFilter};
use pc_store::sources::{self, UpdateSummary};
use pc_store::{bulk, repo, Store, StoreOptions};
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

/// JPEG piccolo con contenuto diverso per ogni `i`.
fn photo(i: u32) -> Vec<u8> {
    let img = image::RgbImage::from_fn(48, 32, |x, y| {
        image::Rgb([
            (i * 37 % 256) as u8,
            ((i * 91 + x * 5) % 256) as u8,
            ((i / 3 + y * 7) % 256) as u8,
        ])
    });
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 90)
        .encode_image(&img)
        .unwrap();
    out
}

fn put(root: &Path, rel: &str, bytes: &[u8]) -> PathBuf {
    let p = rel.split('/').fold(root.to_path_buf(), |a, s| a.join(s));
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, bytes).unwrap();
    p
}

fn path_of(root: &Path, rel: &str) -> PathBuf {
    rel.split('/').fold(root.to_path_buf(), |a, s| a.join(s))
}

/// Pausa per elemento: rende l'import abbastanza lungo da rimuovere la sorgente a metà.
struct Slow;
impl ImportStage for Slow {
    fn phase(&self) -> PhaseSpec {
        PhaseSpec::new("lento", 1.0)
    }
    fn workers(&self) -> usize {
        1
    }
    fn needs_image(&self) -> bool {
        false
    }
    fn process(&self, _: &mut ImportItem, _: &StageCtx) -> StageOutcome {
        std::thread::sleep(Duration::from_millis(40));
        StageOutcome::Continue
    }
}

struct Env {
    tmp: tempfile::TempDir,
    root: PathBuf,
    store: Store,
    mgr: JobManager,
    sink: Arc<Last>,
    source_id: i64,
    thumbs: ThumbCache,
}

fn env_with(files: &[(&str, u32)], slow: bool) -> Env {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("Foto");
    std::fs::create_dir_all(&root).unwrap();
    for (rel, i) in files {
        put(&root, rel, &photo(*i));
    }
    let store = Store::open(&tmp.path().join("catalog.db"), StoreOptions::default()).unwrap();
    let source_id = add_source(&store, &root);
    let sink = Arc::new(Last::default());
    let gov = MemoryGovernor::new(1024);
    let thumbs = ThumbCache::new(tmp.path().join("thumbs"));
    let mgr = JobManager::new(store.clone(), gov.clone(), 2, sink.clone());
    let deps = ImportDeps {
        store: store.clone(),
        thumbs: thumbs.clone(),
        governor: gov,
        extra_stages: if slow { vec![Arc::new(Slow)] } else { vec![] },
        stage_factory: None,
        io_workers: 2,
        decode_workers: 2,
    };
    mgr.register(
        import::JOB_KIND,
        Arc::new(move |p| import::plan(deps.clone(), p)),
    );
    let st = store.clone();
    mgr.register(
        relink::JOB_KIND,
        Arc::new(move |p| relink::plan(st.clone(), p)),
    );
    let (st, cache) = (store.clone(), thumbs.clone());
    mgr.register(
        pc_jobs::bulk::JOB_KIND,
        Arc::new(move |p| {
            let cache = cache.clone();
            let remover: pc_jobs::bulk::ThumbRemover = Arc::new(move |keys| {
                cache.remove(keys);
            });
            pc_jobs::bulk::plan_full(st.clone(), None, Some(remover), p)
        }),
    );
    Env {
        tmp,
        root,
        store,
        mgr,
        sink,
        source_id,
        thumbs,
    }
}

fn env(files: &[(&str, u32)]) -> Env {
    env_with(files, false)
}

fn add_source(store: &Store, root: &Path) -> i64 {
    let path = root.display().to_string();
    store
        .write(move |c| {
            repo::source::insert(
                c,
                &NewSource {
                    path,
                    recursive: true,
                    ..Default::default()
                },
            )
        })
        .unwrap()
}

impl Env {
    fn run(&self, kind: &str, params: serde_json::Value) -> JobSnapshot {
        let id = self.mgr.submit(kind, params).unwrap();
        assert!(self.mgr.wait_idle(Duration::from_secs(60)));
        let snap = self.sink.0.lock()[&id].clone();
        assert!(
            matches!(
                snap.state,
                JobState::Completed | JobState::CompletedWithErrors
            ),
            "{snap:?}"
        );
        snap
    }
    fn import(&self) -> UpdateSummary {
        self.import_source(self.source_id, None)
    }
    fn import_source(&self, source_id: i64, folder: Option<&Path>) -> UpdateSummary {
        let mut params = serde_json::json!({ "sourceId": source_id });
        if let Some(f) = folder {
            params["folder"] = f.display().to_string().into();
        }
        let snap = self.run(import::JOB_KIND, params);
        let s = self.store.read(sources::updates).unwrap();
        let s = s.into_iter().find(|u| u.source_id == source_id).unwrap();
        assert_eq!(s.job_id, snap.id);
        s
    }
    fn id(&self, rel: &str) -> MediaId {
        self.media(rel).id
    }
    fn media(&self, rel: &str) -> pc_store::model::MediaItem {
        let p = path_of(&self.root, rel).display().to_string();
        self.store
            .read(move |c| repo::media::get_by_path(c, &p))
            .unwrap()
            .unwrap_or_else(|| panic!("{rel} non in libreria"))
    }
    fn thumb_exists(&self, key: &str) -> bool {
        self.thumbs.contains(key)
    }
    fn count(&self, f: LibraryFilter) -> i64 {
        self.store.read(move |c| query::count(c, &f)).unwrap()
    }
    fn sql<T: rusqlite::types::FromSql + Send + 'static>(&self, q: &'static str) -> T {
        self.store
            .read(move |c| Ok(c.query_row(q, [], |r| r.get(0))?))
            .unwrap()
    }
}

fn fingerprint(root: &Path) -> BTreeMap<PathBuf, String> {
    walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .map(|e| {
            let h = blake3::hash(&std::fs::read(e.path()).unwrap())
                .to_hex()
                .to_string();
            (e.path().to_path_buf(), h)
        })
        .collect()
}

fn files(prefix: &str, from: u32, n: u32) -> Vec<(String, u32)> {
    (from..from + n)
        .map(|i| (format!("{prefix}/IMG_{i:04}.jpg"), i))
        .collect()
}

fn as_refs(v: &[(String, u32)]) -> Vec<(&str, u32)> {
    v.iter().map(|(p, i)| (p.as_str(), *i)).collect()
}

fn face(e: &Env, media: MediaId, person: Option<i64>) {
    e.store
        .write(move |c| {
            c.execute(
                "INSERT INTO face(media_id,bbox,quality,person_id,assigned_by) VALUES (?1,'[0.1,0.1,0.2,0.2]',0.9,?2,'auto')",
                rusqlite::params![media, person],
            )?;
            Ok(())
        })
        .unwrap();
}

// ------------------------------------------------------------------ 1. rimozione

#[test]
fn bulk_removal_deletes_thumbnails_and_keeps_originals() {
    let list = files("A", 1, 5);
    let e = env(&as_refs(&list));
    e.import();
    let before = fingerprint(&e.root);
    let gone: Vec<_> = list[..3].iter().map(|(p, _)| e.media(p)).collect();
    let kept = e.media(&list[3].0);
    assert!(gone
        .iter()
        .all(|m| e.thumb_exists(m.thumb.as_deref().unwrap())));
    e.run(
        pc_jobs::bulk::JOB_KIND,
        serde_json::json!({
            "ids": gone.iter().map(|m| m.id).collect::<Vec<_>>(),
            "operation": { "action": "remove_library" },
            "confirmed": true,
        }),
    );
    for m in &gone {
        assert!(!e.thumb_exists(m.thumb.as_deref().unwrap()), "{}", m.path);
    }
    assert!(e.thumb_exists(kept.thumb.as_deref().unwrap()));
    assert_eq!(e.count(LibraryFilter::default()), 2);
    assert_eq!(fingerprint(&e.root), before, "originali intatti");
}

#[test]
fn removal_keeps_thumbnail_shared_with_a_duplicate() {
    let e = env(&[("A/uno.jpg", 1), ("B/copia.jpg", 1)]);
    e.import();
    let (a, b) = (e.media("A/uno.jpg"), e.media("B/copia.jpg"));
    assert_eq!(a.thumb, b.thumb);
    manage::remove_media(&e.store, &e.thumbs, vec![a.id]).unwrap();
    assert!(e.thumb_exists(b.thumb.as_deref().unwrap()));
}

#[test]
fn removal_summary_counts_photos_catalogs_and_orphaned_people() {
    let list = files("A", 1, 6);
    let e = env(&as_refs(&list));
    let other = e.tmp.path().join("Altro");
    put(&other, "x.jpg", &photo(50));
    put(&other, "y.jpg", &photo(51));
    let other_id = add_source(&e.store, &other);
    e.import();
    e.import_source(other_id, None);
    let ids: Vec<MediaId> = list.iter().map(|(p, _)| e.id(p)).collect();
    let x = e
        .store
        .read({
            let p = other.join("x.jpg").display().to_string();
            move |c| repo::media::get_by_path(c, &p)
        })
        .unwrap()
        .unwrap()
        .id;
    let (giulia, marco, unnamed) = e
        .store
        .write({
            let ids = ids.clone();
            move |c| {
                bulk::set_label(c, "subject", Some("ritratto"), &ids[..1])?;
                bulk::add_tag(c, "Mare", &ids[..2], &[])?;
                let cat = repo::catalog::create(c, "Vacanze", CatalogKind::Manual, None)?;
                bulk::add_to_catalog(c, cat, &ids[..3])?;
                bulk::add_to_catalog(c, cat, &[x])?;
                let other = repo::catalog::create(c, "Lavoro", CatalogKind::Manual, None)?;
                bulk::add_to_catalog(c, other, &[x])?;
                Ok((
                    pc_store::people::create_person(c, Some("Giulia"))?,
                    pc_store::people::create_person(c, Some("Marco"))?,
                    pc_store::people::create_person(c, None)?,
                ))
            }
        })
        .unwrap();
    face(&e, ids[0], Some(giulia));
    face(&e, ids[1], Some(giulia));
    face(&e, ids[2], Some(marco));
    face(&e, x, Some(marco));
    face(&e, ids[3], Some(unnamed));
    face(&e, ids[4], Some(unnamed));

    let s = manage::removal_summary(&e.mgr, &e.store, e.source_id).unwrap();
    assert_eq!(s.items, 6);
    assert_eq!(
        s.edited, 2,
        "etichetta manuale + tag (una foto ha entrambi)"
    );
    assert_eq!(
        s.catalogs,
        vec![sources::CatalogLoss {
            name: "Vacanze".into(),
            items: 3
        }]
    );
    assert_eq!(s.orphaned_people, vec!["Giulia".to_string()]);
    assert_eq!(s.jobs, 0);

    let before = fingerprint(&e.root);
    let keys: Vec<String> = list
        .iter()
        .map(|(p, _)| e.media(p).thumb.unwrap())
        .collect();
    let removed = manage::remove_source(
        &e.mgr,
        &e.store,
        &e.thumbs,
        e.source_id,
        manage::CANCEL_TIMEOUT,
    )
    .unwrap();
    assert_eq!(removed.items, 6);
    assert_eq!(removed.people, 1, "il cluster senza nome sparisce");
    assert!(
        keys.iter().all(|k| !e.thumb_exists(k)),
        "miniature cancellate"
    );
    assert_eq!(fingerprint(&e.root), before, "originali intatti");
    assert_eq!(
        e.count(LibraryFilter::default()),
        2,
        "restano le foto di Altro"
    );
    let people = e.store.read(pc_store::people::list).unwrap();
    let names: Vec<_> = people.iter().map(|p| (p.name.clone(), p.photos)).collect();
    assert!(names.contains(&(Some("Giulia".into()), 0)), "{names:?}");
    assert!(names.contains(&(Some("Marco".into()), 1)), "{names:?}");
    assert_eq!(people.len(), 2);
    assert!(e
        .store
        .read(sources::updates)
        .unwrap()
        .iter()
        .all(|u| u.source_id != e.source_id));
}

#[test]
fn removal_during_import_cancels_it_first() {
    let list = files("A", 1, 60);
    let e = env_with(&as_refs(&list), true);
    let before = fingerprint(&e.root);
    let job = e
        .mgr
        .submit(
            import::JOB_KIND,
            serde_json::json!({ "sourceId": e.source_id }),
        )
        .unwrap();
    let until = std::time::Instant::now() + Duration::from_secs(30);
    while e.count(LibraryFilter::default()) == 0 {
        assert!(std::time::Instant::now() < until, "l'import non scrive");
        std::thread::sleep(Duration::from_millis(20));
    }
    let s = manage::removal_summary(&e.mgr, &e.store, e.source_id).unwrap();
    assert_eq!(s.jobs, 1);
    manage::remove_source(
        &e.mgr,
        &e.store,
        &e.thumbs,
        e.source_id,
        manage::CANCEL_TIMEOUT,
    )
    .unwrap();
    assert!(e.mgr.wait_idle(Duration::from_secs(30)));
    assert_eq!(e.sink.0.lock()[&job].state, JobState::Cancelled);
    assert_eq!(
        e.count(LibraryFilter::default()),
        0,
        "nessuna foto ricompare"
    );
    let used = e.store.read(sources::referenced_thumbs).unwrap();
    assert!(used.is_empty());
    // Nemmeno le miniature dei file in lavorazione al momento dell'annullamento.
    let left = walkdir::WalkDir::new(e.tmp.path().join("thumbs"))
        .into_iter()
        .filter_map(Result::ok)
        .filter(|x| x.file_type().is_file())
        .count();
    assert_eq!(left, 0, "miniature rimaste nella cache");
    assert_eq!(fingerprint(&e.root), before, "originali intatti");
}

#[test]
fn orphan_thumbnails_are_purged_once_at_startup() {
    let e = env(&[("A/uno.jpg", 1)]);
    e.import();
    let kept = e.media("A/uno.jpg").thumb.unwrap();
    let stale = "ab".repeat(32);
    let img = image::RgbImage::new(4, 4);
    e.thumbs.store(&stale, &img).unwrap();
    let later = std::time::SystemTime::now() + Duration::from_secs(5);
    assert_eq!(
        manage::purge_orphan_thumbs_once(&e.store, &e.thumbs, later).unwrap(),
        Some(1)
    );
    assert!(!e.thumb_exists(&stale));
    assert!(e.thumb_exists(&kept));
    // Seconda volta: già fatta, nulla viene toccato.
    e.thumbs.store(&stale, &img).unwrap();
    assert_eq!(
        manage::purge_orphan_thumbs_once(&e.store, &e.thumbs, later).unwrap(),
        None
    );
    assert!(e.thumb_exists(&stale));
}

#[test]
fn thumbnails_written_after_startup_are_not_purged() {
    let e = env(&[]);
    let started = std::time::SystemTime::now() - Duration::from_secs(5);
    let fresh = "cd".repeat(32);
    e.thumbs.store(&fresh, &image::RgbImage::new(4, 4)).unwrap();
    assert_eq!(
        manage::purge_orphan_thumbs_once(&e.store, &e.thumbs, started).unwrap(),
        Some(0)
    );
    assert!(e.thumb_exists(&fresh));
}

// ------------------------------------------------------------ 2. aggiornamento

#[test]
fn reorganized_folder_summary_and_library_links() {
    let list = files("A", 1, 20);
    let e = env(&as_refs(&list));
    let first = e.import();
    assert_eq!(first.new.count, 20);
    for (rel, i) in files("N", 100, 30) {
        put(&e.root, &rel, &photo(i));
    }
    let mut moved = Vec::new();
    for (rel, _) in &list[..12] {
        let id = e.id(rel);
        let to = path_of(&e.root, &rel.replacen("A/", "B/", 1));
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        std::fs::rename(path_of(&e.root, rel), to).unwrap();
        moved.push(id);
    }
    let mut deleted = Vec::new();
    for (rel, _) in &list[12..17] {
        deleted.push(e.id(rel));
        std::fs::remove_file(path_of(&e.root, rel)).unwrap();
    }
    let s = e.import();
    assert_eq!(s.new.count, 30);
    assert_eq!(s.moved.count, 12);
    assert_eq!(s.missing.count, 5);
    assert_eq!(s.modified.count, 0);
    assert_eq!(s.restored.count, 0);
    assert_eq!(s.unchanged, 3);
    assert!(!s.no_changes());
    let mut got = s.moved.ids.clone();
    got.sort();
    moved.sort();
    assert_eq!(got, moved, "gli spostati conservano il loro id");
    let mut missing = s.missing.ids.clone();
    missing.sort();
    deleted.sort();
    assert_eq!(missing, deleted);
    // "5 mancanti" apre la libreria su quei 5 file.
    let f = LibraryFilter {
        ids: Some(s.missing.ids.clone()),
        ..Default::default()
    };
    assert_eq!(e.count(f.clone()), 5);
    let page = e
        .store
        .read(move |c| query::page(c, &f, query::SortKey::default(), None, 50))
        .unwrap();
    let mut shown: Vec<_> = page.items.iter().map(|i| i.id).collect();
    shown.sort();
    assert_eq!(shown, deleted);
}

#[test]
fn update_without_changes_reports_none() {
    let list = files("A", 1, 4);
    let e = env(&as_refs(&list));
    e.import();
    let s = e.import();
    assert!(s.no_changes(), "{s:?}");
    assert_eq!(s.unchanged, 4);
}

#[test]
fn subfolder_update_touches_only_that_folder() {
    let mut list = files("2024/Mare", 1, 3);
    list.extend(files("2024/Monti", 10, 3));
    let e = env(&as_refs(&list));
    e.import();
    let monti_gone = e.id(&list[3].0);
    std::fs::remove_file(path_of(&e.root, &list[3].0)).unwrap();
    for (rel, i) in files("2024/Mare", 50, 2) {
        put(&e.root, &rel, &photo(i));
    }
    let mare = path_of(&e.root, "2024/Mare");
    let s = e.import_source(e.source_id, Some(&mare));
    assert_eq!(s.new.count, 2);
    assert_eq!(
        s.missing.count, 0,
        "nessun file fuori da Mare diventa mancante"
    );
    assert_eq!(s.unchanged, 3, "solo i file di Mare sono stati esaminati");
    assert_eq!(
        s.folder.as_deref(),
        Some(mare.display().to_string().as_str())
    );
    let m = e
        .store
        .read(move |c| repo::media::get(c, monti_gone))
        .unwrap();
    assert_eq!(m.status, MediaStatus::Ok);
    // L'aggiornamento completo lo marca invece come mancante.
    let s = e.import();
    assert_eq!(s.missing.ids, vec![monti_gone]);
}

#[test]
fn folder_outside_the_source_is_rejected() {
    let e = env(&[("A/uno.jpg", 1)]);
    let outside = e.tmp.path().join("Altrove");
    std::fs::create_dir_all(&outside).unwrap();
    let err = e
        .mgr
        .submit(
            import::JOB_KIND,
            serde_json::json!({ "sourceId": e.source_id, "folder": outside.display().to_string() }),
        )
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidInput);
}

#[test]
fn retouched_photo_is_reanalyzed_and_keeps_manual_data() {
    let e = env(&[("A/ritratto.jpg", 1), ("A/altra.jpg", 2)]);
    e.import();
    let before = e.media("A/ritratto.jpg");
    let id = before.id;
    let old_thumb = before.thumb.clone().unwrap();
    e.store
        .write(move |c| {
            bulk::set_label(c, "subject", Some("ritratto"), &[id])?;
            bulk::add_tag(c, "Famiglia", &[id], &[])?;
            let cat = repo::catalog::create(c, "Album", CatalogKind::Manual, None)?;
            bulk::add_to_catalog(c, cat, &[id])?;
            c.execute(
                "INSERT INTO label(media_id,grp,value,score,origin) VALUES (?1,'scene','spiaggia',0.8,'auto')",
                [id],
            )?;
            c.execute(
                "INSERT INTO analysis_version(media_id,stage,model_ver) VALUES (?1,'clip','m1')",
                [id],
            )?;
            c.execute(
                "INSERT INTO embedding(media_id,model_ver,vector) VALUES (?1,'m1',x'00000000')",
                [id],
            )?;
            Ok(())
        })
        .unwrap();
    face(&e, id, None);
    // Ritaglio con un altro programma: contenuto e data cambiano.
    std::thread::sleep(Duration::from_millis(20));
    put(&e.root, "A/ritratto.jpg", &photo(77));
    let s = e.import();
    assert_eq!(s.modified.ids, vec![id]);
    assert_eq!(s.unchanged, 1);
    let after = e.media("A/ritratto.jpg");
    assert_eq!(after.id, id);
    assert_ne!(
        after.thumb.as_deref(),
        Some(old_thumb.as_str()),
        "nuova miniatura"
    );
    assert!(e.thumb_exists(after.thumb.as_deref().unwrap()));
    assert!(
        !e.thumb_exists(&old_thumb),
        "la vecchia miniatura è stata ripulita"
    );
    // Analisi automatica da rifare; dati manuali invariati.
    assert_eq!(e.sql::<i64>("SELECT count(*) FROM analysis_version"), 0);
    assert_eq!(e.sql::<i64>("SELECT count(*) FROM embedding"), 0);
    assert_eq!(e.sql::<i64>("SELECT count(*) FROM face"), 0);
    assert_eq!(
        e.sql::<i64>("SELECT count(*) FROM label WHERE origin='auto'"),
        0
    );
    assert_eq!(
        e.sql::<String>("SELECT value FROM label WHERE grp='subject' AND origin='manual'"),
        "ritratto"
    );
    assert_eq!(
        e.store.read(move |c| repo::tag::of_media(c, id)).unwrap(),
        vec!["Famiglia"]
    );
    assert_eq!(
        e.store
            .read(move |c| repo::catalog::catalogs_of(c, id))
            .unwrap(),
        vec!["Album"]
    );
}

#[test]
fn missing_file_back_in_place_is_restored() {
    let e = env(&[("A/uno.jpg", 1), ("A/due.jpg", 2)]);
    e.import();
    let id = e.id("A/uno.jpg");
    e.store
        .write(move |c| bulk::add_tag(c, "Mare", &[id], &[]).map(|_| ()))
        .unwrap();
    let p = path_of(&e.root, "A/uno.jpg");
    let parked = e.tmp.path().join("uno.jpg");
    std::fs::rename(&p, &parked).unwrap();
    assert_eq!(e.import().missing.ids, vec![id]);
    std::fs::rename(&parked, &p).unwrap();
    let s = e.import();
    assert_eq!(s.restored.ids, vec![id]);
    assert_eq!(s.missing.count, 0);
    assert_eq!(e.media("A/uno.jpg").status, MediaStatus::Ok);
    assert_eq!(
        e.store.read(move |c| repo::tag::of_media(c, id)).unwrap(),
        vec!["Mare"]
    );
}

// ---------------------------------------------------------------- 3. mancanti

fn relink(e: &Env, old: &Path, new: &Path) -> relink::RelinkSummary {
    e.run(
        relink::JOB_KIND,
        serde_json::json!({
            "sourceId": e.source_id,
            "oldFolder": old.display().to_string(),
            "newFolder": new.display().to_string(),
        }),
    );
    relink::last(&e.store, e.source_id).unwrap().unwrap()
}

#[test]
fn folder_moved_to_another_disk_is_relinked_with_its_cataloguing() {
    let mut list = files("Viaggio", 1, 6);
    list.extend(files("Viaggio/Giorno2", 20, 4));
    list.push(("Casa/altra.jpg".into(), 90));
    let e = env(&as_refs(&list));
    e.import();
    let ids: Vec<MediaId> = list[..10].iter().map(|(p, _)| e.id(p)).collect();
    let person = e
        .store
        .write({
            let ids = ids.clone();
            move |c| {
                bulk::add_tag(c, "Viaggio", &ids, &[])?;
                bulk::set_label(c, "subject", Some("paesaggio"), &ids[..3])?;
                let cat = repo::catalog::create(c, "Estate", CatalogKind::Manual, None)?;
                bulk::add_to_catalog(c, cat, &ids[..5])?;
                pc_store::people::create_person(c, Some("Giulia"))
            }
        })
        .unwrap();
    face(&e, ids[0], Some(person));
    // Spostata "su un altro disco" (fuori da ogni sorgente), due file rinominati.
    let old = e.root.join("Viaggio");
    let new = e.tmp.path().join("E").join("Archivio").join("Viaggio");
    std::fs::create_dir_all(new.parent().unwrap()).unwrap();
    std::fs::rename(&old, &new).unwrap();
    std::fs::rename(new.join("IMG_0001.jpg"), new.join("spiaggia.jpg")).unwrap();
    std::fs::rename(
        new.join("Giorno2").join("IMG_0020.jpg"),
        new.join("Giorno2").join("tramonto.jpg"),
    )
    .unwrap();
    let before = fingerprint(&new);
    assert_eq!(e.import().missing.count, 10);
    let proposed = e
        .store
        .read({
            let id = e.source_id;
            move |c| {
                let paths: Vec<String> = sources::missing(c, id, None)?
                    .into_iter()
                    .map(|(_, p)| p)
                    .collect();
                Ok(sources::common_folder(&paths))
            }
        })
        .unwrap();
    assert_eq!(
        proposed.as_deref(),
        Some(old.display().to_string().as_str())
    );

    let s = relink(&e, &old, &new);
    assert_eq!(
        (s.relinked, s.by_hash, s.still_missing, s.ambiguous),
        (8, 2, 0, 0)
    );
    assert!(s.outside_sources);
    assert!(s.source_complete);
    assert_eq!(
        e.count(LibraryFilter {
            status: Some(MediaStatus::Missing),
            ..Default::default()
        }),
        0
    );
    let first = e
        .store
        .read({
            let id = ids[0];
            move |c| repo::media::get(c, id)
        })
        .unwrap();
    assert_eq!(first.path, new.join("spiaggia.jpg").display().to_string());
    assert_eq!(first.name, "spiaggia.jpg");
    assert_eq!(first.status, MediaStatus::Ok);
    let tagged = e.count(LibraryFilter {
        tags: vec!["Viaggio".into()],
        ..Default::default()
    });
    assert_eq!(tagged, 10);
    assert_eq!(
        e.sql::<i64>("SELECT count(*) FROM label WHERE origin='manual'"),
        3
    );
    assert_eq!(e.sql::<i64>("SELECT count(*) FROM catalog_item"), 5);
    assert_eq!(
        e.sql::<i64>("SELECT count(*) FROM face WHERE person_id IS NOT NULL"),
        1
    );
    assert_eq!(fingerprint(&new), before, "i file non vengono toccati");
    // Aggiunta come sorgente: i file ricollegati passano alla nuova sorgente.
    let new_id = add_source(&e.store, &new);
    let adopted = e
        .store
        .write(move |c| sources::adopt_strays(c, new_id))
        .unwrap();
    assert_eq!(adopted, 10);
    assert_eq!(
        e.count(LibraryFilter {
            source_ids: vec![new_id],
            ..Default::default()
        }),
        10
    );
    assert_eq!(
        e.count(LibraryFilter {
            source_ids: vec![e.source_id],
            ..Default::default()
        }),
        1
    );
}

#[test]
fn identical_copies_stay_missing_as_ambiguous() {
    let e = env(&[
        ("Viaggio/a.jpg", 1),
        ("Viaggio/b.jpg", 1),
        ("Viaggio/c.jpg", 3),
    ]);
    e.import();
    let old = e.root.join("Viaggio");
    let new = e.tmp.path().join("Nuova");
    std::fs::rename(&old, &new).unwrap();
    std::fs::rename(new.join("a.jpg"), new.join("x.jpg")).unwrap();
    std::fs::rename(new.join("b.jpg"), new.join("y.jpg")).unwrap();
    std::fs::rename(new.join("c.jpg"), new.join("z.jpg")).unwrap();
    e.import();
    let s = relink(&e, &old, &new);
    assert_eq!(s.by_hash, 1, "solo c.jpg ha una corrispondenza univoca");
    assert_eq!(s.still_missing, 2);
    assert_eq!(s.ambiguous, 2);
    assert!(!s.source_complete);
}

#[test]
fn remove_missing_removes_only_the_missing_files() {
    let list = files("A", 1, 12);
    let e = env(&as_refs(&list));
    e.import();
    let mut gone = Vec::new();
    for (rel, _) in &list[..5] {
        gone.push(e.media(rel));
        std::fs::remove_file(path_of(&e.root, rel)).unwrap();
    }
    e.import();
    let before = fingerprint(&e.root);
    let removed = manage::remove_missing(&e.store, &e.thumbs, e.source_id).unwrap();
    assert_eq!(removed.items, 5);
    assert_eq!(e.count(LibraryFilter::default()), 7);
    assert_eq!(
        e.count(LibraryFilter {
            status: Some(MediaStatus::Ok),
            ..Default::default()
        }),
        7
    );
    for m in &gone {
        assert!(!e.thumb_exists(m.thumb.as_deref().unwrap()));
    }
    assert_eq!(fingerprint(&e.root), before);
}
