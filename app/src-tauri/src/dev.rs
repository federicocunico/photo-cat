//! Aiuti per lo sviluppo e le prove end-to-end (build di debug o feature `e2e`), attivati da variabili d'ambiente:
//! - `PHOTO_CAT_DEMO=1`: avvia un job dimostrativo;
//! - `PHOTO_CAT_DEV_IMPORT=<cartella>[;<cartella>...]`: aggiunge le cartelle come sorgenti e le importa;
//! - `PHOTO_CAT_DEV_RESUME=1`: riprende i job interrotti (prova di ripresa dopo kill) senza nuovi import;
//! - `PHOTO_CAT_DEV_SEED=<n>`: con catalogo vuoto, inserisce `n` elementi sintetici (prove di scorrimento).
//!
//! Da usare con `PHOTO_CAT_DATA_DIR` per non toccare il catalogo reale.

use pc_ingest::import;
use pc_jobs::JobManager;
use pc_store::model::NewSource;
use pc_store::{repo, Store};

pub fn on_startup(store: &Store, jobs: &JobManager) {
    if std::env::var_os("PHOTO_CAT_DEV_PANIC").is_some() {
        panic!("panic di verifica richiesto in debug");
    }
    if std::env::var_os("PHOTO_CAT_DEMO").is_some() {
        let _ = jobs.submit(
            "demo",
            serde_json::json!({ "items": 3000, "failEvery": 400 }),
        );
    }
    if std::env::var_os("PHOTO_CAT_DEV_RESUME").is_some() {
        let interrupted = pc_jobs::persist::recent(store, 100).unwrap_or_default();
        for job in interrupted
            .into_iter()
            .filter(|j| j.state == pc_jobs::persist::JobState::Interrupted)
        {
            let _ = jobs.resume_interrupted(job.id);
        }
        return;
    }
    let dirs = std::env::var("PHOTO_CAT_DEV_IMPORT").unwrap_or_default();
    for path in dirs.split(';').map(str::trim).filter(|p| !p.is_empty()) {
        let path = path.to_string();
        let res = store.write(move |c| {
            let existing = repo::source::list(c)?.into_iter().find(|s| s.path == path);
            match existing {
                Some(s) => Ok(s.id),
                None => repo::source::insert(
                    c,
                    &NewSource {
                        path,
                        recursive: true,
                        ..Default::default()
                    },
                ),
            }
        });
        if let Ok(id) = res {
            let _ = jobs.submit(import::JOB_KIND, serde_json::json!({ "sourceId": id }));
        }
    }
    if let Some(n) = std::env::var("PHOTO_CAT_DEV_SEED")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
    {
        let _ = store.write(move |c| {
            if repo::media::count(c)? > 0 {
                return Ok(());
            }
            let mut ins = c.prepare(
                "INSERT INTO media_item(path, name, ext, size, mtime, added_at, updated_at) VALUES (?1, ?2, 'jpg', ?3, 0, ?4, ?4)",
            )?;
            let mut dates = c.prepare("INSERT INTO media_dates(media_id, captured_at) VALUES (?1, ?2)")?;
            for i in 1..=n {
                let name = format!("SYN_{i:06}.JPG");
                ins.execute(rusqlite::params![format!("S:\\sintetici\\{name}"), name, 1_000_000 + i, i])?;
                let id = c.last_insert_rowid();
                dates.execute(rusqlite::params![id, format!("20{:02}-{:02}-{:02} 12:00:00", 10 + i % 14, 1 + i % 12, 1 + i % 28)])?;
            }
            tracing::info!(count = n, "elementi sintetici inseriti");
            Ok(())
        });
    }
}
