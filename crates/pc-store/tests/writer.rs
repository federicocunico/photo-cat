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
