//! Archivio costruito da snapshot espliciti. Non visita directory né legge il DB.
use crate::LogLine;
use pc_core::{AppError, AppResult, Config};
use serde_json::{json, Value};
use std::{io::Write, path::Path};
pub struct Bundle {
    pub config: Config,
    pub system: Value,
    pub models: Value,
    pub jobs: Value,
    pub errors: Value,
    /// Riepilogo di `app.db` (solo conteggi: niente nomi né dati biometrici).
    pub app_store: Value,
    pub logs: Vec<LogLine>,
    /// Dump di crash e marcatore dell'ultima sessione anomala, salvati in `crash/`.
    pub attachments: Vec<crate::dumps::Attachment>,
}
pub const CONTENTS: &[&str] = &[
    "manifest.json",
    "config.json",
    "system.json",
    "models.json",
    "jobs.json",
    "errors.json",
    "app-store.json",
    "recent-log.json",
];
pub fn write(destination: &Path, bundle: Bundle) -> AppResult<()> {
    let parent = destination
        .parent()
        .ok_or_else(|| AppError::invalid_input("destinazione archivio"))?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
    let logs: Vec<_> = bundle
        .logs
        .into_iter()
        .map(|l| {
            let fields: Vec<_> = l
                .fields
                .into_iter()
                .filter(|(k, _)| {
                    [
                        "job_id",
                        "phase",
                        "code",
                        "path",
                        "duration_ms",
                        "count",
                        "done",
                        "failed",
                        "percent",
                        "budget_mb",
                        "model",
                        "provider",
                        "version",
                    ]
                    .contains(&k.as_str())
                })
                .collect();
            json!({"ts":l.ts,"level":l.level,"target":l.target,"message":l.message,"fields":fields})
        })
        .collect();
    let snapshots = [
        json!({"schema_version":1,"app_version":env!("CARGO_PKG_VERSION"),"generated_at":chrono::Utc::now().to_rfc3339(),"contents":CONTENTS,"crash":bundle.attachments.iter().map(|a| format!("crash/{}", a.name)).collect::<Vec<_>>()}),
        serde_json::to_value(bundle.config)?,
        bundle.system,
        bundle.models,
        bundle.jobs,
        bundle.errors,
        bundle.app_store,
        json!(logs),
    ];
    {
        let mut zip = zip::ZipWriter::new(tmp.as_file_mut());
        for (name, value) in CONTENTS.iter().zip(snapshots) {
            zip.start_file(
                *name,
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated),
            )
            .map_err(|e| AppError::internal(e.to_string()))?;
            zip.write_all(&serde_json::to_vec_pretty(&value)?)?;
        }
        for a in &bundle.attachments {
            zip.start_file(
                format!("crash/{}", a.name),
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated),
            )
            .map_err(|e| AppError::internal(e.to_string()))?;
            zip.write_all(&a.bytes)?;
        }
        zip.finish()
            .map_err(|e| AppError::internal(e.to_string()))?;
    }
    tmp.as_file().sync_all()?;
    tmp.persist_noclobber(destination)
        .map_err(|e| AppError::from(e.error))?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    #[test]
    fn explicit_allowlist_excludes_photos_database_crops_and_embedding_fields() {
        let tmp = tempfile::tempdir().unwrap();
        for file in ["photo.jpg", "face-crop.png", "catalog.db"] {
            std::fs::write(tmp.path().join(file), b"PRIVATE-BIOMETRIC-CANARY").unwrap();
        }
        let out = tmp.path().join("diagnostics.zip");
        write(
            &out,
            Bundle {
                config: Config::default(),
                system: json!({"os":"windows"}),
                models: json!([{ "id":"sface","version":"1"}]),
                jobs: json!([]),
                errors: json!([]),
                app_store: json!({"schema_version":1,"identities":0}),
                logs: vec![LogLine {
                    ts: chrono::Utc::now(),
                    level: "INFO".into(),
                    target: "pc_ai".into(),
                    message: "analisi completata".into(),
                    fields: vec![
                        ("embedding".into(), "PRIVATE-BIOMETRIC-CANARY".into()),
                        ("crop".into(), "photo.jpg".into()),
                        ("count".into(), "3".into()),
                    ],
                }],
                attachments: vec![],
            },
        )
        .unwrap();
        let mut zip = zip::ZipArchive::new(std::fs::File::open(out).unwrap()).unwrap();
        assert_eq!(zip.len(), CONTENTS.len());
        for i in 0..zip.len() {
            let mut file = zip.by_index(i).unwrap();
            assert!(CONTENTS.contains(&file.name()));
            let mut text = String::new();
            file.read_to_string(&mut text).unwrap();
            assert!(!text.contains("PRIVATE-BIOMETRIC-CANARY"));
            assert!(!text.contains("face-crop"));
        }
    }

    #[test]
    fn webview_crash_dumps_go_into_the_crash_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let logs = tmp.path().join("logs");
        std::fs::create_dir_all(&logs).unwrap();
        let reports = crate::dumps::crashpad_reports(&tmp.path().join("EBWebView"));
        std::fs::create_dir_all(&reports).unwrap();
        std::fs::write(reports.join("e8195465.dmp"), b"MDMP-webview").unwrap();
        let (_m, _) = crate::SessionMarker::start(&logs, "1");
        let (_m2, prev) = crate::SessionMarker::start(&logs, "1");
        assert!(prev.is_some(), "sessione anomala");
        let out = tmp.path().join("d.zip");
        write(
            &out,
            Bundle {
                config: Config::default(),
                system: json!({}),
                models: json!([]),
                jobs: json!([]),
                errors: json!([]),
                app_store: json!({}),
                logs: vec![],
                attachments: crate::dumps::collect(&logs, Some(&reports)),
            },
        )
        .unwrap();
        let mut zip = zip::ZipArchive::new(std::fs::File::open(out).unwrap()).unwrap();
        let mut dump = String::new();
        zip.by_name("crash/webview-e8195465.dmp")
            .unwrap()
            .read_to_string(&mut dump)
            .unwrap();
        assert_eq!(dump, "MDMP-webview");
        let mut session = String::new();
        zip.by_name("crash/session.json")
            .unwrap()
            .read_to_string(&mut session)
            .unwrap();
        assert!(session.contains("lastActivity"));
        let mut manifest = String::new();
        zip.by_name("manifest.json")
            .unwrap()
            .read_to_string(&mut manifest)
            .unwrap();
        assert!(manifest.contains("crash/webview-e8195465.dmp"));
    }
}
