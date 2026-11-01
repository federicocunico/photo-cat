//! "Apri con": app predefinita del sistema, applicazioni esterne configurate
//! e "Mostra nella cartella". I percorsi sono sempre argomenti distinti, mai
//! interpretati da una shell; i file non vengono modificati.
use crate::state::AppState;
use pc_core::config::{DOUBLE_CLICK_NONE, DOUBLE_CLICK_SYSTEM};
use pc_core::{AppError, AppResult, Config, ErrorCode, ExternalApp};
use pc_store::model::{MediaId, MediaStatus};
use pc_store::{repo, Store};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

/// File oltre i quali "Apri con" chiede conferma.
pub const MAX_FILES: usize = 100;

/// Dove aprire i file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OpenTarget {
    /// Applicazione predefinita del sistema per il formato.
    System,
    /// Applicazione esterna configurata.
    App { id: String },
    /// File manager sulla cartella, con il file evidenziato dove possibile.
    Reveal,
    /// Azione del doppio clic scelta nelle impostazioni.
    DoubleClick,
}

/// Esito di [`open_with`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct OpenOutcome {
    /// File aperti.
    pub opened: u32,
    /// Troppi file: serve la conferma dell'utente.
    pub needs_confirmation: bool,
}

/// Argomenti per l'applicazione: `{file}` è sostituito da ogni percorso
/// (un argomento per percorso); senza `{file}` i percorsi sono accodati.
pub(crate) fn build_args(args: &[String], paths: &[String]) -> Vec<String> {
    if !args.iter().any(|a| a.contains("{file}")) {
        return args.iter().chain(paths).cloned().collect();
    }
    let mut out = Vec::new();
    for a in args {
        if a == "{file}" {
            out.extend(paths.iter().cloned());
        } else if a.contains("{file}") {
            out.extend(paths.iter().map(|p| a.replace("{file}", p)));
        } else {
            out.push(a.clone());
        }
    }
    out
}

/// Bundle `.app` di macOS (una directory che si avvia con `open -a`).
fn is_mac_bundle(path: &Path) -> bool {
    cfg!(target_os = "macos")
        && path.is_dir()
        && path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("app"))
}

/// Programma e argomenti da avviare per un'applicazione esterna.
pub(crate) fn command_for(app: &ExternalApp, paths: &[String]) -> (PathBuf, Vec<String>) {
    let args = build_args(&app.args, paths);
    if is_mac_bundle(&app.path) {
        let mut all = vec!["-a".to_string(), app.path.display().to_string()];
        if !args.is_empty() {
            all.push("--args".into());
            all.extend(args);
        }
        return (PathBuf::from("open"), all);
    }
    (app.path.clone(), args)
}

/// L'eseguibile di ogni applicazione esterna deve esistere (file, o bundle
/// `.app` su macOS).
pub(crate) fn validate_apps(config: &Config) -> AppResult<()> {
    for app in &config.external_apps {
        if !(app.path.is_file() || is_mac_bundle(&app.path)) {
            return Err(AppError::new(
                ErrorCode::InvalidConfig,
                format!(
                    "eseguibile di \"{}\" non trovato: {}",
                    app.name,
                    app.path.display()
                ),
            ));
        }
    }
    Ok(())
}

/// Percorsi dei file da aprire: rifiuta i file marcati mancanti o assenti dal disco.
pub(crate) fn existing_paths(db: &Store, ids: &[MediaId]) -> AppResult<Vec<String>> {
    let items = db.read(|c| {
        ids.iter()
            .map(|id| repo::media::get(c, *id))
            .collect::<Result<Vec<_>, _>>()
    })?;
    items
        .into_iter()
        .map(|m| {
            if m.status == MediaStatus::Missing || !Path::new(&m.path).is_file() {
                Err(AppError::new(
                    ErrorCode::NotFound,
                    format!("file non presente sul disco: {}", m.path),
                ))
            } else {
                Ok(m.path)
            }
        })
        .collect()
}

fn log_error(e: &AppError, target: &str) {
    tracing::error!(
        code = e.code.as_str(),
        target,
        detail = e.detail.as_str(),
        "apertura con applicazione esterna non riuscita"
    );
}

fn open(
    handle: &AppHandle,
    config: &Config,
    paths: &[String],
    target: &OpenTarget,
) -> AppResult<()> {
    let target = match target {
        OpenTarget::DoubleClick => match config.ui.double_click.as_str() {
            DOUBLE_CLICK_NONE => return Ok(()),
            DOUBLE_CLICK_SYSTEM => OpenTarget::System,
            id => OpenTarget::App { id: id.into() },
        },
        t => t.clone(),
    };
    let io = |e: String| AppError::new(ErrorCode::Io, e);
    match target {
        OpenTarget::System => {
            for p in paths {
                handle
                    .opener()
                    .open_path(p, None::<&str>)
                    .map_err(|e| io(format!("{p}: {e}")))?;
            }
        }
        OpenTarget::Reveal => {
            if let Some(p) = paths.first() {
                handle
                    .opener()
                    .reveal_item_in_dir(p)
                    .map_err(|e| io(format!("{p}: {e}")))?;
            }
        }
        OpenTarget::App { id } => {
            let app = config
                .external_apps
                .iter()
                .find(|a| a.id == id)
                .ok_or_else(|| {
                    AppError::invalid_input(format!("applicazione sconosciuta: {id}"))
                })?;
            if !(app.path.is_file() || is_mac_bundle(&app.path)) {
                return Err(AppError::new(
                    ErrorCode::NotFound,
                    format!(
                        "applicazione \"{}\" non trovata: {}",
                        app.name,
                        app.path.display()
                    ),
                ));
            }
            let (program, args) = command_for(app, paths);
            std::process::Command::new(&program)
                .args(&args)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .map_err(|e| io(format!("avvio di {}: {e}", program.display())))?;
        }
        OpenTarget::DoubleClick => unreachable!("risolto sopra"),
    }
    Ok(())
}

/// Apre i file con l'app predefinita, un'applicazione esterna o il file
/// manager. Oltre [`MAX_FILES`] file serve `confirmed`.
#[tauri::command]
#[specta::specta]
pub async fn open_with(
    handle: AppHandle,
    state: State<'_, AppState>,
    ids: Vec<MediaId>,
    target: OpenTarget,
    confirmed: bool,
) -> AppResult<OpenOutcome> {
    if ids.is_empty() {
        return Ok(OpenOutcome {
            opened: 0,
            needs_confirmation: false,
        });
    }
    if ids.len() > MAX_FILES && !confirmed && target != OpenTarget::Reveal {
        return Ok(OpenOutcome {
            opened: 0,
            needs_confirmation: true,
        });
    }
    let (db, config) = (state.store.clone(), state.config.read().clone());
    let label = format!("{target:?}");
    tauri::async_runtime::spawn_blocking(move || {
        let run = || -> AppResult<u32> {
            let paths = existing_paths(&db, &ids)?;
            open(&handle, &config, &paths, &target)?;
            Ok(paths.len() as u32)
        };
        run()
            .map(|opened| OpenOutcome {
                opened,
                needs_confirmation: false,
            })
            .inspect_err(|e| log_error(e, &label))
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use pc_store::model::NewMedia;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn arguments_keep_each_path_whole_without_a_shell() {
        let tricky = s(&[r#"D:\Foto\Rock & Roll\"a" b.jpg"#, r"D:\Foto\c d.jpg"]);
        // Scenario "Argomenti personalizzati".
        assert_eq!(
            build_args(&s(&["--edit", "{file}"]), &tricky[..1]),
            s(&["--edit", r#"D:\Foto\Rock & Roll\"a" b.jpg"#])
        );
        // Senza `{file}` i percorsi sono accodati, uno per argomento.
        assert_eq!(
            build_args(&s(&["-n"]), &tricky),
            s(&["-n", r#"D:\Foto\Rock & Roll\"a" b.jpg"#, r"D:\Foto\c d.jpg"])
        );
        assert_eq!(build_args(&[], &tricky), tricky);
        // `{file}` dentro un argomento: un argomento per file.
        assert_eq!(
            build_args(&s(&["--open={file}", "-x"]), &tricky),
            s(&[
                r#"--open=D:\Foto\Rock & Roll\"a" b.jpg"#,
                r"--open=D:\Foto\c d.jpg",
                "-x"
            ])
        );
        let app = ExternalApp {
            id: "lr".into(),
            name: "Lightroom".into(),
            path: PathBuf::from(r"C:\LR\Lightroom.exe"),
            args: vec![],
        };
        let (program, args) = command_for(&app, &tricky);
        assert_eq!(program, PathBuf::from(r"C:\LR\Lightroom.exe"));
        assert_eq!(args, tricky);
    }

    #[test]
    fn missing_files_and_executables_are_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let db = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
        let present = tmp.path().join("a.jpg");
        std::fs::write(&present, b"x").unwrap();
        let gone = tmp.path().join("b.jpg");
        let (p1, p2) = (present.display().to_string(), gone.display().to_string());
        let ids = db
            .write(move |c| {
                [p1, p2]
                    .iter()
                    .map(|p| {
                        repo::media::upsert(
                            c,
                            &NewMedia {
                                path: p.clone(),
                                name: "x".into(),
                                ext: "jpg".into(),
                                ..Default::default()
                            },
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .unwrap();
        assert_eq!(existing_paths(&db, &ids[..1]).unwrap().len(), 1);
        // Scenario "File mancante".
        let err = existing_paths(&db, &ids).unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);

        // Scenario "Eseguibile inesistente".
        let mut cfg = Config::default();
        cfg.external_apps.push(ExternalApp {
            id: "x".into(),
            name: "Editor".into(),
            path: tmp.path().join("non-esiste.exe"),
            args: vec![],
        });
        let err = validate_apps(&cfg).unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidConfig);
        assert!(err.detail.contains("non-esiste.exe"), "{}", err.detail);
        cfg.external_apps[0].path = present;
        validate_apps(&cfg).unwrap();
    }
}
