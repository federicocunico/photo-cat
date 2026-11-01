//! Migrazioni versionate tramite `PRAGMA user_version`, con backup del DB
//! esistente prima di ogni aggiornamento di schema.

use crate::StoreError;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

/// Migrazioni in ordine: l'indice + 1 è la versione raggiunta.
const MIGRATIONS: &[&str] = &[
    include_str!("schema_v1.sql"),
    include_str!("schema_v2.sql"),
    include_str!("schema_v3.sql"),
    include_str!("schema_v4.sql"),
    include_str!("schema_v5.sql"),
];

pub const SCHEMA_VERSION: i64 = MIGRATIONS.len() as i64;

pub fn current_version(conn: &Connection) -> Result<i64, StoreError> {
    Ok(conn.query_row("PRAGMA user_version", [], |r| r.get(0))?)
}

/// Porta il DB all'ultima versione. Se il DB contiene già dati di una versione
/// precedente, ne crea prima una copia in `backup_dir`. Restituisce il percorso
/// del backup, se creato.
pub fn migrate(
    conn: &mut Connection,
    backup_dir: Option<&Path>,
) -> Result<Option<PathBuf>, StoreError> {
    let from = current_version(conn)?;
    if from > SCHEMA_VERSION {
        return Err(StoreError::Schema(format!(
            "il catalogo è alla versione {from}, più recente di quella supportata ({SCHEMA_VERSION})"
        )));
    }
    if from == SCHEMA_VERSION {
        return Ok(None);
    }
    let backup = match (from > 0, backup_dir) {
        (true, Some(dir)) => Some(backup(conn, dir, from)?),
        _ => None,
    };
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(from as usize) {
        let version = i as i64 + 1;
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", version)?;
        tx.commit()?;
        tracing::info!(version, "schema del catalogo migrato");
    }
    Ok(backup)
}

/// Versione di schema di un catalogo esistente, senza modificarlo (`None` se
/// il file non esiste o non è leggibile).
pub fn catalog_version(path: &Path) -> Option<i64> {
    if !path.is_file() {
        return None;
    }
    let conn =
        Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).ok()?;
    current_version(&conn).ok()
}

/// Prima di migrare un catalogo di una versione precedente, copia anche la
/// configurazione accanto al backup del catalogo: le versioni precedenti
/// rifiutano i campi nuovi di `config.toml`, quindi per tornare indietro
/// servono entrambe le copie. Restituisce il percorso della copia.
pub fn backup_config_before_migration(
    catalog: &Path,
    config: &Path,
    dir: &Path,
) -> Option<PathBuf> {
    let version = catalog_version(catalog)?;
    if version == 0 || version >= SCHEMA_VERSION || !config.is_file() {
        return None;
    }
    std::fs::create_dir_all(dir).ok()?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let path = dir.join(format!("config-v{version}-{stamp}.toml"));
    match std::fs::copy(config, &path) {
        Ok(_) => {
            tracing::info!(path = %path.display(), "backup della configurazione prima della migrazione");
            Some(path)
        }
        Err(e) => {
            tracing::warn!(detail = %e, "backup della configurazione non riuscito");
            None
        }
    }
}

fn backup(conn: &Connection, dir: &Path, version: i64) -> Result<PathBuf, StoreError> {
    std::fs::create_dir_all(dir).map_err(|e| StoreError::Schema(e.to_string()))?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let path = dir.join(format!("catalog-v{version}-{stamp}.db"));
    conn.execute("VACUUM INTO ?1", [path.to_string_lossy()])?;
    tracing::info!(path = %path.display(), "backup del catalogo prima della migrazione");
    Ok(path)
}

/// Porta un DB vuoto a una versione precedente (test delle migrazioni).
#[cfg(test)]
pub(crate) fn migrate_to(conn: &mut Connection, version: i64) {
    for (i, sql) in MIGRATIONS.iter().enumerate().take(version as usize) {
        let tx = conn.transaction().unwrap();
        tx.execute_batch(sql).unwrap();
        tx.pragma_update(None, "user_version", i as i64 + 1)
            .unwrap();
        tx.commit().unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v4_named_people_become_identities_once() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("catalog.db");
        {
            let mut c = Connection::open(&path).unwrap();
            migrate_to(&mut c, 4);
            c.execute_batch(
                "INSERT INTO person(id,name,created_at) VALUES (1,'Giulia',0),(2,'Luca',0),(3,NULL,0);",
            )
            .unwrap();
        }
        let app = crate::AppStore::open(&tmp.path().join("app.db")).unwrap();
        let bk = tmp.path().join("bk");
        for round in 0..2 {
            let db = crate::Store::open(
                &path,
                crate::StoreOptions {
                    backup_dir: Some(bk.clone()),
                    ..Default::default()
                },
            )
            .unwrap();
            let a = app.clone();
            let linked = db
                .write(move |c| {
                    crate::people::migrate_to_identities(c, |name| {
                        a.with(|ac| crate::app_store::identities::ensure_by_name(ac, name))
                    })
                })
                .unwrap();
            assert_eq!(linked, if round == 0 { 2 } else { 0 });
            let rows: Vec<(String, Option<String>)> = db
                .read(|c| {
                    let mut q =
                        c.prepare("SELECT COALESCE(name,''),identity_uid FROM person ORDER BY id")?;
                    let r = q
                        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(r)
                })
                .unwrap();
            let ids = app.with(crate::app_store::identities::list).unwrap();
            assert_eq!(ids.len(), 2, "nessun duplicato al giro {round}");
            let uid_of = |n: &str| ids.iter().find(|i| i.name == n).unwrap().uid.clone();
            assert_eq!(rows[0], ("Giulia".into(), Some(uid_of("Giulia"))));
            assert_eq!(rows[1], ("Luca".into(), Some(uid_of("Luca"))));
            assert_eq!(rows[2].1, None);
            db.close();
        }
        // La migrazione dello schema v4 → v5 ha lasciato un backup.
        assert_eq!(std::fs::read_dir(&bk).unwrap().count(), 1);
    }

    const EXPECTED_TABLES: &[&str] = &[
        "source",
        "media_item",
        "media_tech",
        "media_dates",
        "exif",
        "location",
        "label",
        "embedding",
        "media_people",
        "person",
        "face",
        "tag",
        "media_tag",
        "catalog",
        "catalog_item",
        "job",
        "job_phase",
        "item_stage",
        "error_log",
        "organize_op",
        "organize_entry",
        "undo_log",
        "settings_kv",
        "media_fts",
        "source_group",
        "media_time_placement",
    ];

    #[test]
    fn v4_library_migrates_to_v5_with_backup_and_keeps_dates() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("catalog.db");
        {
            let mut c = Connection::open(&path).unwrap();
            migrate_to(&mut c, 4);
            c.execute_batch(
                "INSERT INTO media_item(id,path,name,ext,size,mtime,added_at,updated_at) VALUES (1,'a.jpg','a.jpg','jpg',1,1,1,1);
                 INSERT INTO media_dates(media_id,captured_at,captured_source) VALUES (1,'2026-08-19 10:43:00','exif_original');",
            )
            .unwrap();
        }
        let mut c = Connection::open(&path).unwrap();
        let bk = tmp.path().join("bk");
        let backup = migrate(&mut c, Some(&bk)).unwrap().expect("backup");
        assert!(backup.exists());
        assert_eq!(current_version(&c).unwrap(), SCHEMA_VERSION);
        let (captured, instant, presumed): (String, Option<i64>, i64) = c
            .query_row(
                "SELECT captured_at, instant_utc, instant_presumed FROM media_dates WHERE media_id = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(captured, "2026-08-19 10:43:00");
        // L'istante arriva con il job time_backfill.
        assert_eq!((instant, presumed), (None, 0));
        let old = Connection::open(&backup).unwrap();
        assert_eq!(current_version(&old).unwrap(), 4);
    }

    #[test]
    fn import_computes_instant_and_source_group() {
        let mut c = Connection::open_in_memory().unwrap();
        migrate(&mut c, None).unwrap();
        let id = crate::repo::media::upsert(
            &c,
            &crate::model::NewMedia {
                path: "IMG_1.HEIC".into(),
                name: "IMG_1.HEIC".into(),
                ext: "heic".into(),
                ..Default::default()
            },
        )
        .unwrap();
        let d = crate::model::MediaDates {
            captured_at: Some("2026-08-19 10:44:12".into()),
            captured_source: Some(crate::model::DateSource::ExifOriginal),
            declared_offset_min: Some(-360),
            device: Some("Apple iPhone 17 Pro Max".into()),
            ..Default::default()
        };
        crate::repo::meta::set_dates_with(&c, id, &d, &|_| 60).unwrap();
        assert_eq!(crate::repo::meta::dates(&c, id).unwrap().unwrap(), d);
        let (instant, presumed, local, group): (i64, bool, i32, i64) = c
            .query_row(
                "SELECT instant_utc, instant_presumed, local_offset_min, group_id FROM media_dates WHERE media_id = ?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!(
            crate::timeline::local_text(instant, 0).as_deref(),
            Some("2026-08-19 16:44:12")
        );
        assert!(!presumed);
        assert_eq!(local, -360);
        let (dev, off): (String, i32) = c
            .query_row(
                "SELECT device, declared_offset_min FROM source_group WHERE id = ?1",
                [group],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!((dev.as_str(), off), ("Apple iPhone 17 Pro Max", -360));
        // Stesso dispositivo e fuso: stesso gruppo; senza dispositivo: "Altre sorgenti".
        assert_eq!(
            crate::timeline::ensure_group(&c, Some("Apple iPhone 17 Pro Max"), Some(-360)).unwrap(),
            group
        );
        let other = crate::timeline::ensure_group(&c, None, Some(120)).unwrap();
        assert_eq!(
            crate::timeline::ensure_group(&c, None, None).unwrap(),
            other
        );
    }

    #[test]
    fn empty_db_migrates_to_latest() {
        let mut conn = Connection::open_in_memory().unwrap();
        assert_eq!(migrate(&mut conn, None).unwrap(), None);
        assert_eq!(current_version(&conn).unwrap(), SCHEMA_VERSION);
        for t in EXPECTED_TABLES {
            let n: i64 = conn
                .query_row(
                    "SELECT count(*) FROM sqlite_master WHERE name = ?1",
                    [t],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(n, 1, "tabella {t} mancante");
        }
        // Idempotente.
        migrate(&mut conn, None).unwrap();
    }

    #[test]
    fn config_is_copied_only_before_migrating_an_older_catalog() {
        let tmp = tempfile::tempdir().unwrap();
        let (db, cfg, bk) = (
            tmp.path().join("catalog.db"),
            tmp.path().join("config.toml"),
            tmp.path().join("backups"),
        );
        std::fs::write(&cfg, "[ui]
theme = \"light\"
").unwrap();
        // Nessun catalogo (primo avvio): niente copia.
        assert_eq!(backup_config_before_migration(&db, &cfg, &bk), None);
        // Catalogo della versione precedente (v0.1.0 = schema 4).
        let mut conn = Connection::open(&db).unwrap();
        migrate_to(&mut conn, 4);
        drop(conn);
        let copy = backup_config_before_migration(&db, &cfg, &bk).expect("copia");
        assert!(copy.file_name().unwrap().to_string_lossy().starts_with("config-v4-"));
        assert_eq!(std::fs::read_to_string(&copy).unwrap(), std::fs::read_to_string(&cfg).unwrap());
        // Catalogo già aggiornato: niente copia.
        let mut conn = Connection::open(&db).unwrap();
        migrate(&mut conn, None).unwrap();
        drop(conn);
        assert_eq!(backup_config_before_migration(&db, &cfg, &bk), None);
    }

    #[test]
    fn newer_schema_is_refused() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "user_version", SCHEMA_VERSION + 1)
            .unwrap();
        assert!(matches!(
            migrate(&mut conn, None),
            Err(StoreError::Schema(_))
        ));
    }

    #[test]
    fn backup_is_written_for_existing_older_db() {
        let tmp = tempfile::tempdir().unwrap();
        let mut conn = Connection::open(tmp.path().join("c.db")).unwrap();
        // Simula un DB "v0" con dati ma senza versione: forziamo la versione a 0
        // dopo aver creato una tabella fittizia, poi fingiamo versione 1 < target.
        conn.execute_batch("CREATE TABLE legacy(x); INSERT INTO legacy VALUES (1);")
            .unwrap();
        let path = backup(&conn, &tmp.path().join("bk"), 0).unwrap();
        assert!(path.exists());
        let copy = Connection::open(&path).unwrap();
        let n: i64 = copy
            .query_row("SELECT count(*) FROM legacy", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
        migrate(&mut conn, Some(&tmp.path().join("bk"))).unwrap();
    }
}
