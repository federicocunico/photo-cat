use pc_store::{Store, StoreError, StoreOptions};
use std::time::Duration;

fn open(dir: &std::path::Path) -> Store {
    Store::open(&dir.join("catalog.db"), StoreOptions::default()).unwrap()
}

#[test]
fn concurrent_producers_never_hit_busy() {
    let tmp = tempfile::tempdir().unwrap();
    let store = open(tmp.path());
    let producers: Vec<_> = (0..8)
        .map(|p| {
            let store = store.clone();
            std::thread::spawn(move || {
                for i in 0..500 {
                    let tag = format!("t{p}-{i}");
                    store
                        .submit(move |c| {
                            c.execute("INSERT INTO tag(name) VALUES (?1)", [tag])?;
                            Ok(())
                        })
                        .unwrap();
                }
            })
        })
        .collect();
    // Letture concorrenti mentre si scrive.
    for _ in 0..20 {
        store
            .read(|c| Ok(c.query_row("SELECT count(*) FROM tag", [], |r| r.get::<_, i64>(0))?))
            .unwrap();
        std::thread::sleep(Duration::from_millis(5));
    }
    for p in producers {
        p.join().unwrap();
    }
    store.flush().unwrap();
    let n: i64 = store
        .read(|c| Ok(c.query_row("SELECT count(*) FROM tag", [], |r| r.get(0))?))
        .unwrap();
    assert_eq!(n, 4000);
}

#[test]
fn write_returns_value_and_failed_op_rolls_back_alone() {
    let tmp = tempfile::tempdir().unwrap();
    let store = open(tmp.path());
    let id: i64 = store
        .write(|c| {
            c.execute("INSERT INTO tag(name) VALUES ('mare')", [])?;
            Ok(c.last_insert_rowid())
        })
        .unwrap();
    assert!(id > 0);

    // Duplicato: fallisce solo questa operazione, il resto del batch resta.
    store
        .submit(|c| {
            c.execute("INSERT INTO tag(name) VALUES ('monti')", [])?;
            Ok(())
        })
        .unwrap();
    let err = store
        .write(|c| {
            c.execute("INSERT INTO tag(name) VALUES ('lago')", [])?;
            c.execute("INSERT INTO tag(name) VALUES ('MARE')", [])?; // UNIQUE NOCASE
            Ok(())
        })
        .unwrap_err();
    assert!(matches!(err, StoreError::Sqlite(_)));
    let names: Vec<String> = store
        .read(|c| {
            let mut st = c.prepare("SELECT name FROM tag ORDER BY name")?;
            let v = st.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
            Ok(v)
        })
        .unwrap();
    assert_eq!(names, vec!["mare", "monti"]);
}

#[test]
fn data_persists_after_reopen() {
    let tmp = tempfile::tempdir().unwrap();
    {
        let store = open(tmp.path());
        store
            .submit(|c| {
                c.execute("INSERT INTO tag(name) VALUES ('persistente')", [])?;
                Ok(())
            })
            .unwrap();
        store.close();
    }
    let store = open(tmp.path());
    let n: i64 = store
        .read(|c| Ok(c.query_row("SELECT count(*) FROM tag", [], |r| r.get(0))?))
        .unwrap();
    assert_eq!(n, 1);
}

/// Import sintetico di 20.000 elementi con un lettore che interroga di continuo (l'interfaccia):
/// il file WAL resta sotto la soglia grazie ai checkpoint periodici.
#[test]
fn wal_stays_bounded_during_a_large_import() {
    use std::sync::atomic::{AtomicBool, Ordering};
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("catalog.db");
    let store = pc_store::Store::open(&db, pc_store::StoreOptions::default()).unwrap();
    let wal = tmp.path().join("catalog.db-wal");
    let stop = std::sync::Arc::new(AtomicBool::new(false));
    let (s2, stop2) = (store.clone(), stop.clone());
    let reader = std::thread::spawn(move || {
        while !stop2.load(Ordering::Relaxed) {
            let _ = s2.read(|c| {
                Ok(c.query_row("SELECT count(*) FROM media_item", [], |r| {
                    r.get::<_, i64>(0)
                })?)
            });
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    });
    let padding = "x".repeat(1500);
    let mut max_wal = 0u64;
    for i in 0..20_000i64 {
        let path = format!("/foto/{padding}/IMG_{i:06}.JPG");
        store
            .submit(move |c| {
                c.execute(
                    "INSERT INTO media_item(path, name, ext, size, mtime, added_at, updated_at)
                     VALUES (?1, 'IMG.JPG', 'jpg', 1, 0, 0, 0)",
                    [&path],
                )?;
                Ok(())
            })
            .unwrap();
        if i % 1000 == 999 {
            store.flush().unwrap();
            max_wal = max_wal.max(std::fs::metadata(&wal).map(|m| m.len()).unwrap_or(0));
        }
    }
    store.flush().unwrap();
    stop.store(true, Ordering::Relaxed);
    reader.join().unwrap();
    let limit = pc_store::db::WAL_SIZE_LIMIT as u64;
    assert!(max_wal > 0, "il WAL è stato usato");
    assert!(
        max_wal <= limit,
        "WAL arrivato a {max_wal} byte (soglia {limit})"
    );
    let n: i64 = store
        .read(|c| Ok(c.query_row("SELECT count(*) FROM media_item", [], |r| r.get(0))?))
        .unwrap();
    assert_eq!(n, 20_000);
}
