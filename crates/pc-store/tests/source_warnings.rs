//! Avvisi chiudibili delle sorgenti: `missing_since` e riconoscimento.
use pc_store::detail;
use pc_store::model::*;
use pc_store::repo;
use pc_store::sources::{self, UpdateSummary};
use pc_store::{Store, StoreOptions};
use rusqlite::Connection;

fn store(dir: &std::path::Path) -> Store {
    Store::open(&dir.join("catalog.db"), StoreOptions::default()).unwrap()
}

fn source(c: &Connection) -> i64 {
    repo::source::insert(
        c,
        &NewSource {
            path: "D:\\Foto".into(),
            recursive: true,
            include_formats: vec![],
            ignore_patterns: vec![],
            watch: false,
        },
    )
    .unwrap()
}

fn add(c: &Connection, src: i64, n: usize) -> i64 {
    repo::media::upsert(
        c,
        &NewMedia {
            source_id: Some(src),
            path: format!("D:\\Foto\\{n}.jpg"),
            name: format!("{n}.jpg"),
            ext: "jpg".into(),
            ..Default::default()
        },
    )
    .unwrap()
}

fn counts(c: &Connection) -> (i64, i64, i64) {
    let s = detail::source_summaries(c).unwrap().remove(0);
    (s.missing, s.new_missing, s.failed_warning)
}

fn summary(src: i64, job_id: i64, failed: i64) -> UpdateSummary {
    UpdateSummary {
        source_id: src,
        job_id,
        failed,
        ..Default::default()
    }
}

#[test]
fn new_missing_counts_only_after_acknowledgement() {
    let dir = tempfile::tempdir().unwrap();
    let s = store(dir.path());
    s.write(|c| {
        let src = source(c);
        let ids: Vec<i64> = (0..8).map(|n| add(c, src, n)).collect();
        for id in &ids[..5] {
            repo::media::set_status(c, *id, MediaStatus::Missing)?;
        }
        assert_eq!(counts(c), (5, 5, 0));

        sources::acknowledge_warnings(c, src)?;
        // I mancanti restano, ma non sono più "nuovi".
        assert_eq!(counts(c), (5, 0, 0));

        for id in &ids[5..7] {
            repo::media::set_status(c, *id, MediaStatus::Missing)?;
        }
        // Il riconoscimento è nel passato: i nuovi mancanti sono posteriori.
        let at = sources::warning_ack(c, src)?.unwrap().at;
        c.execute(
            "UPDATE media_item SET missing_since = ?1 WHERE id IN (?2, ?3)",
            rusqlite::params![at + 1000, ids[5], ids[6]],
        )?;
        assert_eq!(counts(c), (7, 2, 0));

        // Un file tornato presente esce dal conteggio e azzera missing_since.
        repo::media::set_status(c, ids[5], MediaStatus::Ok)?;
        assert_eq!(counts(c), (6, 1, 0));
        let since: Option<i64> = c.query_row(
            "SELECT missing_since FROM media_item WHERE id=?1",
            [ids[5]],
            |r| r.get(0),
        )?;
        assert_eq!(since, None);
        Ok(())
    })
    .unwrap();
}

#[test]
fn failed_warning_clears_on_acknowledgement_and_on_clean_update() {
    let dir = tempfile::tempdir().unwrap();
    let s = store(dir.path());
    s.write(|c| {
        let src = source(c);
        add(c, src, 0);
        sources::save_update(c, &summary(src, 10, 3))?;
        assert_eq!(counts(c).2, 3);

        sources::acknowledge_warnings(c, src)?;
        assert_eq!(counts(c).2, 0);

        // Nuovo aggiornamento con errori: l'avviso ritorna.
        sources::save_update(c, &summary(src, 11, 2))?;
        assert_eq!(counts(c).2, 2);

        // Aggiornamento successivo senza errori: nessun avviso.
        sources::save_update(c, &summary(src, 12, 0))?;
        assert_eq!(counts(c).2, 0);
        Ok(())
    })
    .unwrap();
}
