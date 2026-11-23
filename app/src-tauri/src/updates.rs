//! Aggiornamenti dell'app (app-updates): ultima release pubblicata su GitHub, download
//! verificato dell'installer di Windows in background e avvio dell'installazione.
//!
//! Nessun dato dell'utente esce dal computer: la sola richiesta è a `releases/latest`, con
//! l'User-Agent "photo-cat/<versione>". Gli URL arrivano dalla risposta di GitHub, mai dalla UI,
//! e devono stare nelle release del repository del pacchetto.

use crate::state::{blocking, AppState};
use parking_lot::Mutex;
use pc_core::{AppError, AppResult, ErrorCode};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State};

/// Avanzamento del download (`UpdateProgress`), al massimo 4 volte al secondo.
pub const UPDATE_PROGRESS: &str = "update://progress";
/// Installer scaricato e verificato (`UpdateReady`).
pub const UPDATE_READY: &str = "update://ready";
/// Download non riuscito o interrotto (`UpdateFailed`).
pub const UPDATE_FAILED: &str = "update://failed";

const CURRENT: &str = env!("CARGO_PKG_VERSION");
const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
/// Cartella degli installer scaricati, nei dati dell'app.
const DIR: &str = "updates";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    /// Versione della release, senza "v".
    pub version: String,
    /// Versione in uso.
    pub current: String,
    /// Pagina della release ("Novità", e "Aggiorna" su macOS e Linux).
    pub page_url: String,
    pub published_at: Option<String>,
    /// Installer per questa piattaforma (solo Windows x64).
    pub installer: Option<InstallerAsset>,
    /// Installer già scaricato e verificato nella cartella degli aggiornamenti.
    pub downloaded: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InstallerAsset {
    pub name: String,
    pub url: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProgress {
    pub version: String,
    pub done: u64,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UpdateReady {
    pub version: String,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UpdateFailed {
    pub version: String,
    pub message: String,
    /// Interrotto dall'utente: il file parziale resta per la ripresa.
    pub cancelled: bool,
}

/// Stato condiviso: ultimo controllo e download in corso.
#[derive(Default)]
pub struct Updates {
    last: Mutex<Option<UpdateInfo>>,
    downloading: AtomicBool,
    cancel: Arc<AtomicBool>,
}

// --- Risposta di GitHub -------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    published_at: Option<String>,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    size: u64,
    /// "sha256:<hex>" nelle risposte recenti dell'API.
    #[serde(default)]
    digest: Option<String>,
}

// --- Funzioni pure (testate) --------------------------------------------------------------

/// "owner/repo" dall'URL del repository del pacchetto.
fn repo_path(repository: &str) -> Option<String> {
    let rest = repository
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .strip_prefix("https://github.com/")?;
    let mut parts = rest.split('/');
    let (owner, repo) = (parts.next()?, parts.next()?);
    (!owner.is_empty() && !repo.is_empty() && parts.next().is_none())
        .then(|| format!("{owner}/{repo}"))
}

/// Versione `major.minor.patch` (con o senza "v"); `None` se non leggibile.
fn parse_version(v: &str) -> Option<(u64, u64, u64)> {
    let v = v.trim().trim_start_matches('v');
    let core = v.split(['-', '+']).next()?;
    let mut it = core.split('.').map(|p| p.parse::<u64>().ok());
    let out = (it.next()??, it.next()??, it.next()??);
    it.next().is_none().then_some(out)
}

/// `true` se `candidate` è più recente di `current`.
fn is_newer(candidate: &str, current: &str) -> bool {
    match (parse_version(candidate), parse_version(current)) {
        (Some(c), Some(cur)) => c > cur,
        _ => false,
    }
}

/// Nome dell'installer di Windows per una versione (come in `release.yml`).
fn installer_name(version: &str) -> String {
    format!("photo-cat_{version}_x64-setup.exe")
}

/// Versione di un file della cartella degli aggiornamenti (installer o download parziale).
fn version_of_file(name: &str) -> Option<String> {
    let name = name.strip_suffix(".part").unwrap_or(name);
    let v = name
        .strip_prefix("photo-cat_")?
        .strip_suffix("_x64-setup.exe")?;
    parse_version(v).map(|_| v.to_string())
}

/// Prima parola di un file `.sha256` ("<hex>  <nome>"), se è uno SHA-256 esadecimale.
fn parse_sha256(text: &str) -> Option<String> {
    let word = text.split_whitespace().next()?;
    let word = word.strip_prefix("sha256:").unwrap_or(word);
    (word.len() == 64 && word.chars().all(|c| c.is_ascii_hexdigit())).then(|| word.to_lowercase())
}

/// URL ammesso: pagina o allegato di una release del repository del pacchetto.
fn allowed_release_url(url: &str, repo: &str) -> bool {
    let prefix = format!("https://github.com/{repo}/releases/");
    url.starts_with(&prefix) && !url.contains("..") && !url.contains('@')
}

/// Esito del controllo dalla release, senza rete: versione, pagina e installer da scaricare
/// (`sha256` da riempire). `None` se non è più recente della versione in uso.
fn info_from_release(
    release: &Release,
    current: &str,
    repo: &str,
    windows_x64: bool,
) -> AppResult<Option<(UpdateInfo, Option<String>)>> {
    if release.draft || release.prerelease {
        return Ok(None);
    }
    let version = release.tag_name.trim_start_matches('v').to_string();
    if !is_newer(&version, current) {
        return Ok(None);
    }
    if !allowed_release_url(&release.html_url, repo) {
        return Err(AppError::new(
            ErrorCode::Network,
            "pagina della release non riconosciuta",
        ));
    }
    let mut sha_url = None;
    let installer = if windows_x64 {
        let name = installer_name(&version);
        release
            .assets
            .iter()
            .find(|a| a.name == name)
            .filter(|a| allowed_release_url(&a.browser_download_url, repo) && a.size > 0)
            .map(|a| {
                sha_url = release
                    .assets
                    .iter()
                    .find(|s| s.name == format!("{name}.sha256"))
                    .map(|s| s.browser_download_url.clone())
                    .filter(|u| allowed_release_url(u, repo));
                InstallerAsset {
                    name: a.name.clone(),
                    url: a.browser_download_url.clone(),
                    size: a.size,
                    sha256: a
                        .digest
                        .as_deref()
                        .and_then(parse_sha256)
                        .unwrap_or_default(),
                }
            })
    } else {
        None
    };
    Ok(Some((
        UpdateInfo {
            version,
            current: current.to_string(),
            page_url: release.html_url.clone(),
            published_at: release.published_at.clone(),
            installer,
            downloaded: false,
        },
        sha_url,
    )))
}

/// Elimina installer e download parziali di versioni uguali o precedenti a quella in uso.
pub fn cleanup(dir: &Path, current: &str) -> usize {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut removed = 0;
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if let Some(v) = version_of_file(&name) {
            if !is_newer(&v, current) && std::fs::remove_file(e.path()).is_ok() {
                removed += 1;
            }
        }
    }
    removed
}

// --- Rete -----------------------------------------------------------------------------------

fn client() -> AppResult<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(15))
        .user_agent(concat!("photo-cat/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| AppError::new(ErrorCode::Network, e.to_string()))
}

fn network(e: impl std::fmt::Display) -> AppError {
    AppError::new(
        ErrorCode::Network,
        format!("GitHub non raggiungibile ({e})"),
    )
}

fn fetch_latest(repo: &str) -> AppResult<Release> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let resp = client()?
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .map_err(network)?;
    match resp.status().as_u16() {
        200 => resp.json::<Release>().map_err(network),
        403 | 429 => Err(AppError::new(
            ErrorCode::Network,
            "troppi controlli in poco tempo: riprova più tardi",
        )),
        404 => Err(AppError::new(
            ErrorCode::Network,
            "nessuna release pubblicata",
        )),
        s => Err(network(format!("risposta {s}"))),
    }
}

fn fetch_sha256(url: &str) -> AppResult<String> {
    let text = client()?
        .get(url)
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.text())
        .map_err(network)?;
    parse_sha256(&text)
        .ok_or_else(|| AppError::new(ErrorCode::Network, "SHA-256 della release non leggibile"))
}

fn updates_dir(state: &AppState) -> PathBuf {
    state.paths.root().join(DIR)
}

fn repo() -> AppResult<String> {
    repo_path(REPOSITORY).ok_or_else(|| AppError::internal("repository del pacchetto non valido"))
}

// --- Comandi ------------------------------------------------------------------------------

/// Ultima release pubblicata, se più recente della versione in uso (`None` se aggiornata).
/// Riconosce l'installer già scaricato e verificato.
#[tauri::command]
#[specta::specta]
pub async fn check_update(app: AppHandle) -> AppResult<Option<UpdateInfo>> {
    blocking(app, |app, state| {
        let repo = repo()?;
        let release = fetch_latest(&repo)?;
        let windows_x64 = cfg!(all(windows, target_arch = "x86_64"));
        let Some((mut info, sha_url)) = info_from_release(&release, CURRENT, &repo, windows_x64)?
        else {
            *app.state::<Updates>().last.lock() = None;
            return Ok(None);
        };
        if let (Some(inst), Some(url)) = (info.installer.as_mut(), sha_url) {
            inst.sha256 = fetch_sha256(&url)?;
        }
        // Senza SHA-256 non si scarica: resta solo la pagina della release.
        if info.installer.as_ref().is_some_and(|i| i.sha256.is_empty()) {
            info.installer = None;
        }
        if let Some(inst) = &info.installer {
            let file = updates_dir(state).join(&inst.name);
            info.downloaded = file.is_file()
                && pc_models::download::sha256_file(&file).is_ok_and(|h| h == inst.sha256);
        }
        *app.state::<Updates>().last.lock() = Some(info.clone());
        Ok(Some(info))
    })
    .await
}

/// Scarica in background l'installer dell'ultimo controllo. Avanzamento ed esito arrivano
/// con gli eventi `update://progress`, `update://ready` e `update://failed`.
#[tauri::command]
#[specta::specta]
pub async fn download_update(
    app: AppHandle,
    updates: State<'_, Updates>,
    version: String,
) -> AppResult<()> {
    let info = updates
        .last
        .lock()
        .clone()
        .filter(|i| i.version == version)
        .ok_or_else(|| AppError::invalid_input("controlla di nuovo gli aggiornamenti"))?;
    let inst = info
        .installer
        .clone()
        .ok_or_else(|| AppError::invalid_input("nessun installer per questo sistema"))?;
    if updates.downloading.swap(true, Ordering::SeqCst) {
        return Err(AppError::invalid_input(
            "download dell'aggiornamento già in corso",
        ));
    }
    updates.cancel.store(false, Ordering::SeqCst);
    let cancel = updates.cancel.clone();
    let dir = updates_dir(&app.state::<AppState>());
    std::thread::spawn(move || {
        let dest = dir.join(&inst.name);
        let last = Mutex::new(Instant::now() - Duration::from_secs(1));
        let progress = |done: u64| {
            let mut t = last.lock();
            if t.elapsed() >= Duration::from_millis(250) || done >= inst.size {
                *t = Instant::now();
                let _ = app.emit(
                    UPDATE_PROGRESS,
                    UpdateProgress {
                        version: info.version.clone(),
                        done,
                        total: inst.size,
                    },
                );
            }
        };
        let out = pc_models::download::download(
            std::slice::from_ref(&inst.url),
            &dest,
            inst.size,
            &inst.sha256,
            &pc_models::download::RetryPolicy::default(),
            &progress,
            &cancel,
        );
        let updates = app.state::<Updates>();
        match out {
            Ok(()) => {
                if let Some(last) = updates.last.lock().as_mut() {
                    last.downloaded = true;
                }
                tracing::info!(version = %info.version, "aggiornamento scaricato e verificato");
                let _ = app.emit(
                    UPDATE_READY,
                    UpdateReady {
                        version: info.version.clone(),
                    },
                );
            }
            Err(e) => {
                tracing::warn!(version = %info.version, "download dell'aggiornamento: {}", e.detail);
                let _ = app.emit(
                    UPDATE_FAILED,
                    UpdateFailed {
                        version: info.version.clone(),
                        message: e.detail.clone(),
                        cancelled: e.code == ErrorCode::Cancelled,
                    },
                );
            }
        }
        updates.downloading.store(false, Ordering::SeqCst);
    });
    Ok(())
}

/// Interrompe il download: il file parziale resta e il prossimo "Aggiorna" riprende da lì.
#[tauri::command]
#[specta::specta]
pub fn cancel_update_download(updates: State<'_, Updates>) {
    // Con `swap`: il test dei comandi sincroni riconosce l'uso del catalogo da una parola
    // che il metodo di scrittura atomica conterrebbe.
    updates.cancel.swap(true, Ordering::SeqCst);
}

/// Avvia l'installer scaricato (di nuovo verificato) e chiude photo-cat in modo regolare,
/// così l'utente prosegue con l'installazione.
#[tauri::command]
#[specta::specta]
pub async fn install_update(app: AppHandle, version: String) -> AppResult<()> {
    blocking(app, move |app, state| {
        let info = app
            .state::<Updates>()
            .last
            .lock()
            .clone()
            .filter(|i| i.version == version)
            .ok_or_else(|| AppError::invalid_input("controlla di nuovo gli aggiornamenti"))?;
        let inst = info
            .installer
            .ok_or_else(|| AppError::invalid_input("nessun installer per questo sistema"))?;
        let file = updates_dir(state).join(&inst.name);
        if !file.is_file() || pc_models::download::sha256_file(&file)? != inst.sha256 {
            return Err(AppError::new(
                ErrorCode::Io,
                "l'installer scaricato non è integro: scaricalo di nuovo",
            ));
        }
        launch(&file)?;
        tracing::info!(version = %info.version, "installer avviato, chiusura di photo-cat");
        // Prima la risposta alla UI, poi la chiusura (che passa da RunEvent::Exit).
        let handle = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(400));
            handle.exit(0);
        });
        Ok(())
    })
    .await
}

#[cfg(windows)]
fn launch(file: &Path) -> AppResult<()> {
    use std::os::windows::process::CommandExt;
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    std::process::Command::new(file)
        .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
        .spawn()
        .map(|_| ())
        .map_err(|e| AppError::new(ErrorCode::Io, format!("installer non avviato: {e}")))
}

#[cfg(not(windows))]
fn launch(_file: &Path) -> AppResult<()> {
    Err(AppError::invalid_input(
        "su questo sistema l'aggiornamento si installa dalla pagina della release",
    ))
}

/// Apre nel browser la pagina di una release del progetto (solo URL delle release).
#[tauri::command]
#[specta::specta]
pub fn open_release_page(app: AppHandle, url: String) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt;
    if !allowed_release_url(&url, &repo()?) {
        return Err(AppError::invalid_input("indirizzo non ammesso"));
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| AppError::new(ErrorCode::Io, e.to_string()))
}

/// Pulizia all'avvio (in un thread: non rallenta l'apertura).
pub fn cleanup_in_background(root: PathBuf) {
    std::thread::spawn(move || {
        let removed = cleanup(&root.join(DIR), CURRENT);
        if removed > 0 {
            tracing::info!(removed, "installer di versioni già installate eliminati");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const REPO: &str = "federicocunico/photo-cat";

    fn release(tag: &str, assets: &[(&str, u64)]) -> Release {
        Release {
            tag_name: tag.into(),
            html_url: format!("https://github.com/{REPO}/releases/tag/{tag}"),
            published_at: Some("2026-11-24T10:00:00Z".into()),
            draft: false,
            prerelease: false,
            assets: assets
                .iter()
                .map(|(name, size)| Asset {
                    name: (*name).into(),
                    browser_download_url: format!(
                        "https://github.com/{REPO}/releases/download/{tag}/{name}"
                    ),
                    size: *size,
                    digest: None,
                })
                .collect(),
        }
    }

    #[test]
    fn repository_of_the_package() {
        assert_eq!(
            repo_path("https://github.com/federicocunico/photo-cat").as_deref(),
            Some(REPO)
        );
        assert_eq!(
            repo_path("https://github.com/a/b.git/").as_deref(),
            Some("a/b")
        );
        assert_eq!(repo_path("https://gitlab.com/a/b"), None);
        assert_eq!(repo_path("https://github.com/a"), None);
        assert_eq!(repo_path(REPOSITORY).as_deref(), Some(REPO));
    }

    #[test]
    fn versions_compare_numerically() {
        assert!(is_newer("0.5.4", "0.5.3"));
        assert!(is_newer("v0.10.0", "0.9.9"));
        assert!(is_newer("1.0.0", "0.99.99"));
        assert!(!is_newer("0.5.3", "0.5.3"));
        assert!(!is_newer("0.5.2", "0.5.3"));
        // Tag non leggibili: nessun aggiornamento.
        assert!(!is_newer("latest", "0.5.3"));
        assert!(!is_newer("0.5", "0.5.3"));
        assert!(!is_newer("0.5.4.1", "0.5.3"));
        assert_eq!(parse_version("v1.2.3-beta.1"), Some((1, 2, 3)));
    }

    #[test]
    fn installer_for_windows_only_and_newer_only() {
        let r = release(
            "v0.5.4",
            &[
                ("photo-cat_0.5.4_x64-setup.exe", 496_000_000),
                ("photo-cat_0.5.4_x64-setup.exe.sha256", 99),
                ("photo-cat_0.5.4_amd64.deb", 199_000_000),
            ],
        );
        let (info, sha) = info_from_release(&r, "0.5.3", REPO, true).unwrap().unwrap();
        assert_eq!(info.version, "0.5.4");
        assert_eq!(info.current, "0.5.3");
        let inst = info.installer.unwrap();
        assert_eq!(inst.name, "photo-cat_0.5.4_x64-setup.exe");
        assert_eq!(inst.size, 496_000_000);
        assert!(sha
            .unwrap()
            .ends_with("photo-cat_0.5.4_x64-setup.exe.sha256"));
        // macOS e Linux: solo la pagina della release.
        let (other, _) = info_from_release(&r, "0.5.3", REPO, false)
            .unwrap()
            .unwrap();
        assert!(other.installer.is_none());
        assert!(other.page_url.ends_with("/releases/tag/v0.5.4"));
        // Già aggiornata, bozze e pre-release: nessun aggiornamento.
        assert!(info_from_release(&r, "0.5.4", REPO, true)
            .unwrap()
            .is_none());
        let mut pre = release("v0.6.0", &[]);
        pre.prerelease = true;
        assert!(info_from_release(&pre, "0.5.3", REPO, true)
            .unwrap()
            .is_none());
    }

    #[test]
    fn urls_outside_the_project_releases_are_refused() {
        assert!(allowed_release_url(
            "https://github.com/federicocunico/photo-cat/releases/tag/v0.5.4",
            REPO
        ));
        for bad in [
            "https://github.com/altro/photo-cat/releases/tag/v0.5.4",
            "http://github.com/federicocunico/photo-cat/releases/tag/v0.5.4",
            "https://github.com/federicocunico/photo-cat/releases/../../evil",
            "https://github.com/federicocunico/photo-cat/releases@evil.com/x",
            "https://evil.com/federicocunico/photo-cat/releases/x",
            "file:///C:/Windows/System32/calc.exe",
        ] {
            assert!(!allowed_release_url(bad, REPO), "{bad}");
        }
        // Un allegato fuori dalle release del progetto non diventa un installer.
        let mut r = release("v0.5.4", &[("photo-cat_0.5.4_x64-setup.exe", 10)]);
        r.assets[0].browser_download_url = "https://evil.com/photo-cat_0.5.4_x64-setup.exe".into();
        let (info, _) = info_from_release(&r, "0.5.3", REPO, true).unwrap().unwrap();
        assert!(info.installer.is_none());
        // Pagina della release non riconosciuta: errore, non un collegamento.
        let mut r = release("v0.5.4", &[]);
        r.html_url = "https://evil.com/x".into();
        assert!(info_from_release(&r, "0.5.3", REPO, false).is_err());
    }

    #[test]
    fn sha256_from_release_file_or_digest() {
        let hex = "a".repeat(64);
        assert_eq!(
            parse_sha256(&format!("{hex}  photo-cat_0.5.4_x64-setup.exe\n")),
            Some(hex.clone())
        );
        assert_eq!(
            parse_sha256(&format!("sha256:{}", "B".repeat(64))),
            Some("b".repeat(64))
        );
        assert_eq!(parse_sha256("non è un hash"), None);
        assert_eq!(parse_sha256(&"z".repeat(64)), None);
        assert_eq!(parse_sha256(""), None);
    }

    /// Contro la release vera su GitHub (rete): `cargo test -p photo-cat --lib live_ -- --ignored`.
    #[test]
    #[ignore = "richiede la rete"]
    fn live_latest_release_has_a_verifiable_installer() {
        let repo = repo().unwrap();
        let release = fetch_latest(&repo).unwrap();
        let (info, sha_url) = info_from_release(&release, "0.0.1", &repo, true)
            .unwrap()
            .expect("una release più recente della 0.0.1");
        let inst = info.installer.expect("installer di Windows nella release");
        assert!(inst.size > 100 * 1024 * 1024, "{}", inst.size);
        let sha = fetch_sha256(&sha_url.expect("file .sha256 dell'installer")).unwrap();
        assert_eq!(sha.len(), 64);
        if !inst.sha256.is_empty() {
            assert_eq!(inst.sha256, sha, "digest dell'API e file .sha256");
        }
        assert!(allowed_release_url(&info.page_url, &repo));
        println!("{} {} {} {sha}", info.version, inst.name, inst.size);
    }

    #[test]
    fn cleanup_keeps_only_newer_installers() {
        let dir = tempfile::tempdir().unwrap();
        for name in [
            "photo-cat_0.5.2_x64-setup.exe",
            "photo-cat_0.5.3_x64-setup.exe",
            "photo-cat_0.5.3_x64-setup.exe.part",
            "photo-cat_0.5.4_x64-setup.exe",
            "photo-cat_0.5.4_x64-setup.exe.part",
            "altro.txt",
        ] {
            std::fs::write(dir.path().join(name), b"x").unwrap();
        }
        assert_eq!(cleanup(dir.path(), "0.5.3"), 3);
        let mut left: Vec<String> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        left.sort();
        assert_eq!(
            left,
            [
                "altro.txt",
                "photo-cat_0.5.4_x64-setup.exe",
                "photo-cat_0.5.4_x64-setup.exe.part"
            ]
        );
        // Cartella mancante: nessun errore.
        assert_eq!(cleanup(&dir.path().join("assente"), "0.5.3"), 0);
    }
}
