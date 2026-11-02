//! Startup state is available before the catalog and the rest of AppState.
use parking_lot::Mutex;
use pc_core::{AppError, AppPaths, AppResult};
use pc_store::{AppStore, Store, StoreOptions};
use serde::Serialize;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tauri::{Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BootStatus {
    pub phase: String,
    pub ready: bool,
    pub error: Option<String>,
    pub logs_path: String,
}

pub struct Boot {
    pub status: Mutex<BootStatus>,
    pub paths: Mutex<AppResult<AppPaths>>,
    running: AtomicBool,
    pub diagnostics: Arc<pc_diag::Diagnostics>,
    pub previous_crash: Option<pc_diag::CrashInfo>,
}

impl Boot {
    pub fn new(
        paths: AppResult<AppPaths>,
        diagnostics: Arc<pc_diag::Diagnostics>,
        logs: std::path::PathBuf,
        previous_crash: Option<pc_diag::CrashInfo>,
    ) -> Self {
        Self {
            status: Mutex::new(BootStatus {
                phase: "opening".into(),
                ready: false,
                error: None,
                logs_path: logs.display().to_string(),
            }),
            paths: Mutex::new(paths),
            running: AtomicBool::new(false),
            diagnostics,
            previous_crash,
        }
    }

    pub fn phase(&self, handle: &tauri::AppHandle, phase: &str) {
        let status = {
            let mut s = self.status.lock();
            s.phase = phase.into();
            s.clone()
        };
        let _ = handle.emit("boot://phase", status);
    }
}

pub fn start(handle: tauri::AppHandle) {
    let boot = handle.state::<Boot>();
    if boot.status.lock().ready || boot.running.swap(true, Ordering::AcqRel) {
        return;
    }
    {
        let mut status = boot.status.lock();
        status.error = None;
        status.phase = "opening".into();
    }
    boot.phase(&handle, "opening");
    std::thread::spawn(move || {
        let boot = handle.state::<Boot>();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let paths = boot.paths.lock().clone()?;
            crate::initialize(&handle, paths, &boot)
        }))
        .unwrap_or_else(|_| Err(AppError::internal("startup worker panicked")));
        let status = {
            let mut status = boot.status.lock();
            match result {
                Ok(()) => {
                    status.ready = true;
                    status.phase = "ready".into();
                }
                Err(e) => {
                    tracing::error!(detail = %e, "avvio non riuscito");
                    status.error = Some(e.to_string());
                }
            }
            status.clone()
        };
        boot.running.store(false, Ordering::Release);
        let _ = handle.emit(
            if status.ready {
                "boot://ready"
            } else {
                "boot://phase"
            },
            status,
        );
    });
}

pub fn open_stores(paths: &AppPaths, mut phase: impl FnMut(&str)) -> AppResult<(Store, AppStore)> {
    phase("opening");
    paths.ensure()?;
    phase("migration");
    pc_store::migrate::backup_config_before_migration(
        &paths.database(),
        &paths.config_file(),
        &paths.backups(),
    );
    let store = Store::open(
        &paths.database(),
        StoreOptions {
            backup_dir: Some(paths.backups()),
            ..Default::default()
        },
    )?;
    phase("archive");
    let app_store = AppStore::open(&paths.app_store())?;
    Ok((store, app_store))
}

#[tauri::command]
#[specta::specta]
pub fn boot_status(boot: State<'_, Boot>) -> BootStatus {
    boot.status.lock().clone()
}

#[tauri::command]
#[specta::specta]
pub fn boot_retry(handle: tauri::AppHandle) {
    start(handle);
}

#[tauri::command]
#[specta::specta]
pub async fn boot_choose_folder(handle: tauri::AppHandle) -> AppResult<()> {
    tauri::async_runtime::spawn_blocking(move || {
        let chosen = handle.dialog().file().blocking_pick_folder();
        let Some(chosen) = chosen else {
            return Ok(());
        };
        let path = chosen
            .into_path()
            .map_err(|e| AppError::invalid_input(e.to_string()))?;
        let boot = handle.state::<Boot>();
        if boot.running.load(Ordering::Acquire) || boot.status.lock().ready {
            return Ok(());
        }
        let old = boot.paths.lock().clone().ok();
        let next = old
            .as_ref()
            .map_or_else(|| AppPaths::at(&path), |p| p.with_root(&path));
        next.ensure()?;
        // Remember the choice when the original configuration directory is writable.
        if let Some(old) = old.filter(|p| p.root() != next.root()) {
            let mut config = pc_core::Config::load(&old.config_file()).unwrap_or_default();
            config.data_dir = Some(path);
            if let Err(e) = config.save(&old.config_file()) {
                tracing::warn!(detail = %e, "percorso dati valido solo per questa sessione");
            }
        }
        *boot.paths.lock() = Ok(next);
        start(handle.clone());
        Ok(())
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn phases_cover_migration_and_app_archive_with_a_real_old_catalog() {
        let dir = tempfile::tempdir().unwrap();
        let paths = AppPaths::at(dir.path());
        paths.ensure().unwrap();
        let conn = rusqlite::Connection::open(paths.database()).unwrap();
        conn.execute_batch(include_str!("../../../crates/pc-store/src/schema_v1.sql"))
            .unwrap();
        conn.pragma_update(None, "user_version", 1).unwrap();
        drop(conn);
        pc_core::Config::default()
            .save(&paths.config_file())
            .unwrap();
        let mut phases = Vec::new();
        let (_store, _app_store) = open_stores(&paths, |p| phases.push(p.to_owned())).unwrap();
        assert_eq!(phases, ["opening", "migration", "archive"]);
        assert_eq!(
            pc_store::migrate::catalog_version(&paths.database()),
            Some(pc_store::migrate::SCHEMA_VERSION)
        );
        assert!(std::fs::read_dir(paths.backups()).unwrap().count() >= 2);
        assert!(paths.app_store().is_file());
    }
}
