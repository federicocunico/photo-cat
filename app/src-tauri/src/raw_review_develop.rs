//! Comandi dello sviluppo dalla fotocamera (Varianti RAW): stato e preset della fotocamera,
//! verifica, piano con stime e avvio dell'attività `raw_review_develop`.
//!
//! Su Windows parlano con la fotocamera reale. Su Linux e macOS rispondono "Disponibile solo su
//! Windows" (lo stato ha `available: false`), salvo con la feature `fake-camera`, che usa la
//! fotocamera simulata per prove e sviluppo.
//!
//! I RAF da sviluppare vengono sempre dall'indice della sessione aperta, mai da percorsi dati dal
//! chiamante; l'attività li legge soltanto e scrive solo nelle cartelle dei preset.

use crate::raw_review::{OpenedReview, ReviewState};
use crate::state::AppState;
use pc_core::{AppError, AppResult};
use pc_raw_review::variants::diagnostics::{CameraDiagnostics, CameraLogLine};
use pc_raw_review::variants::job::{self, DevelopParams};
use pc_raw_review::variants::plan::{build_plan, PresetDef};
use pc_raw_review::variants::prefs;
use pc_raw_review::variants::service::status_error;
use pc_raw_review::variants::types::{
    CameraPresets, CameraStatus, DevelopReport, DevelopStop, VariantPlan, VariantRequest,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::{Emitter, State};

/// Stato della fotocamera cambiato (collegata, scollegata, occupata, pronta…).
pub const CAMERA_STATUS: &str = "raw-review://camera-status";
/// Riga del registro della fotocamera.
pub const CAMERA_LOG: &str = "raw-review://camera-log";
/// Avanzamento o esito della verifica della fotocamera.
pub const CAMERA_DIAGNOSTICS: &str = "raw-review://camera-diagnostics";
/// Uno sviluppo è terminato (anche annullato o fallito) e la sessione è stata aggiornata.
pub const DEVELOPED: &str = "raw-review://developed";

/// Cosa sviluppare, scelto nel dialogo.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RawReviewDevelopRequest {
    pub session_id: String,
    /// Solo questi scatti; assente = tutti gli scatti con un RAF.
    #[serde(default)]
    pub shot_keys: Option<Vec<String>>,
    /// Preset scelti: `asshot` e `C1`..`C7`.
    pub preset_ids: Vec<String>,
    /// Cartella di output; assente = quella predefinita della sessione.
    #[serde(default)]
    pub output_dir: Option<String>,
    /// Rigenera anche le varianti già presenti e complete.
    #[serde(default)]
    pub overwrite: bool,
}

/// Evento di fine sviluppo: riepilogo e sessione aggiornata con i nuovi preset.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RawReviewDeveloped {
    pub job_id: i64,
    pub session_id: String,
    pub converted: u32,
    pub skipped: u32,
    pub failed: u32,
    pub cancelled: bool,
    /// Perché lo sviluppo si è fermato prima della fine (fotocamera bloccata, cartella rifiutata…).
    pub stopped: Option<DevelopStop>,
    /// La sessione riletta dal disco (scatti e preset aggiornati, scelte intatte). `None` se la
    /// sessione è stata chiusa nel frattempo o la rilettura non è riuscita (vedi `refreshError`).
    pub opened: Option<OpenedReview>,
    pub refresh_error: Option<String>,
}

/// Dopo uno sviluppo: aggiorna la sessione e avvisa la UI.
pub fn on_develop_finished(app: &tauri::AppHandle, state: &AppState, report: &DevelopReport) {
    let refreshed = state.raw_review.refresh_from_disk(
        state.paths.root(),
        &report.session_id,
        Some(&report.output_dir),
    );
    let (opened, refresh_error) = match refreshed {
        Ok(opened) => (Some(opened), None),
        Err(e) => {
            tracing::warn!(
                code = e.code.as_str(),
                detail = e.detail.as_str(),
                "sessione non aggiornata dopo lo sviluppo"
            );
            (None, Some(e.detail))
        }
    };
    let _ = app.emit(
        DEVELOPED,
        &RawReviewDeveloped {
            job_id: report.job_id,
            session_id: report.session_id.clone(),
            converted: report.converted,
            skipped: report.skipped,
            failed: report.failed,
            cancelled: report.cancelled,
            stopped: report.stopped.clone(),
            opened,
            refresh_error,
        },
    );
}

/// Il servizio emette stato, log e verifica come eventi verso la UI.
pub fn camera_event_sink(app: tauri::AppHandle) -> pc_raw_review::variants::service::EventSink {
    use pc_raw_review::variants::service::CameraEvent;
    std::sync::Arc::new(move |event| {
        let _ = match &event {
            CameraEvent::Status(status) => app.emit(CAMERA_STATUS, status),
            CameraEvent::Log(line) => app.emit(CAMERA_LOG, line),
            CameraEvent::Diagnostics(diagnostics) => app.emit(CAMERA_DIAGNOSTICS, diagnostics),
        };
    })
}

/// Preset dalla fotocamera per il piano: l'istantanea già letta o, se manca, una lettura nuova;
/// senza fotocamera resta "Come scattato" (gli altri id si segnalano come non disponibili).
fn presets_for_plan(state: &AppState) -> Vec<PresetDef> {
    let cached = state.camera.cached_presets();
    if !cached.is_empty() || state.camera.is_running() {
        return cached;
    }
    state
        .camera
        .presets()
        .unwrap_or_else(|_| vec![PresetDef::as_shot()])
}

struct Resolved {
    request: VariantRequest,
    params: DevelopParams,
}

fn resolve(raw_review: &ReviewState, request: &RawReviewDevelopRequest) -> AppResult<Resolved> {
    let sources = raw_review.develop_sources(&request.session_id, request.shot_keys.as_deref())?;
    let output_dir = match request
        .output_dir
        .as_deref()
        .map(str::trim)
        .filter(|d| !d.is_empty())
    {
        Some(dir) => {
            if !Path::new(dir).is_absolute() {
                return Err(AppError::invalid_input(
                    "la cartella di output deve essere un percorso assoluto",
                ));
            }
            dir.to_string()
        }
        None => sources.default_output_dir,
    };
    let params = DevelopParams {
        session_id: request.session_id.clone(),
        sources: sources.rafs,
        output_dir: Some(output_dir),
        preset_ids: request.preset_ids.clone(),
        overwrite: request.overwrite,
    };
    params.validate()?;
    Ok(Resolved {
        request: VariantRequest {
            sources: params.sources.clone(),
            output_dir: params.output_dir.clone(),
            preset_ids: params.preset_ids.clone(),
            overwrite: params.overwrite,
        },
        params,
    })
}

fn plan_of(resolved: &Resolved, presets: &[PresetDef]) -> AppResult<VariantPlan> {
    build_plan(&resolved.request, presets).map(|plan| plan.summary())
}

/// Stato della fotocamera, senza bloccarsi su una fotocamera appesa. Con `available: false` (Linux
/// e macOS) `message` dice "Disponibile solo su Windows".
#[tauri::command]
#[specta::specta]
pub async fn raw_review_camera_status(app: tauri::AppHandle) -> AppResult<CameraStatus> {
    crate::state::blocking(app, |_app, state| Ok(state.camera.status())).await
}

/// Banchi C1-C7 e "Come scattato" con riepilogo e impostazioni non applicabili, più i preset da
/// preselezionare (l'ultima scelta, o tutti i banchi la prima volta).
#[tauri::command]
#[specta::specta]
pub async fn raw_review_camera_presets(app: tauri::AppHandle) -> AppResult<CameraPresets> {
    crate::state::blocking(app, |_app, state| {
        let presets = state.camera.presets_view()?;
        Ok(prefs::with_selection(state.paths.root(), presets))
    })
    .await
}

/// Verifica della fotocamera: l'esito di ogni controllo con il suggerimento. `ready` è vero solo
/// se nessun controllo è fallito: solo allora "Avvia" è possibile.
#[tauri::command]
#[specta::specta]
pub async fn raw_review_camera_diagnostics(app: tauri::AppHandle) -> AppResult<CameraDiagnostics> {
    crate::state::blocking(app, |_app, state| state.camera.diagnostics()).await
}

/// Registro recente della fotocamera (per l'assistenza).
#[tauri::command]
#[specta::specta]
pub async fn raw_review_camera_log(state: State<'_, AppState>) -> AppResult<Vec<CameraLogLine>> {
    Ok(state.camera.log.snapshot())
}

/// Piano dello sviluppo: RAF, conversioni totali, già presenti (saltate), da fare, spazio e tempo
/// stimati, avvisi. Non scrive nulla e non occupa la fotocamera se i preset sono già noti.
#[tauri::command]
#[specta::specta]
pub async fn raw_review_develop_plan(
    app: tauri::AppHandle,
    request: RawReviewDevelopRequest,
) -> AppResult<VariantPlan> {
    crate::state::blocking(app, move |_app, state| {
        let resolved = resolve(&state.raw_review, &request)?;
        plan_of(&resolved, &presets_for_plan(state))
    })
    .await
}

/// Avvia lo sviluppo come attività. Rifiuta l'avvio se la fotocamera non è pronta, se non c'è
/// spazio sufficiente sul disco di output (con il margine), se non c'è niente da fare o se un
/// altro sviluppo è in corso. Ricorda i preset scelti. Restituisce l'id dell'attività.
#[tauri::command]
#[specta::specta]
pub async fn start_raw_review_develop(
    app: tauri::AppHandle,
    request: RawReviewDevelopRequest,
) -> AppResult<i64> {
    crate::state::blocking(app, move |_app, state| {
        if !state.camera.is_available() {
            return Err(status_error(&state.camera.status()));
        }
        let resolved = resolve(&state.raw_review, &request)?;
        if state.jobs.has_active(job::JOB_KIND, |_| true) {
            return Err(AppError::invalid_input("Uno sviluppo è già in corso."));
        }
        let status = state.camera.status();
        if status.state != pc_raw_review::variants::types::CameraState::Ready {
            return Err(status_error(&status));
        }
        // Piano con i banchi letti ora dalla fotocamera: è quello che si applicherà.
        let presets = state.camera.presets()?;
        let plan = plan_of(&resolved, &presets)?;
        job::check_space(&plan)?;
        if plan.pending_conversions == 0 {
            return Err(AppError::invalid_input(
                "Tutte le varianti scelte sono già presenti: attiva «Sovrascrivi» per rigenerarle.",
            ));
        }
        if let Err(e) = prefs::remember(state.paths.root(), &request.preset_ids) {
            tracing::warn!(
                code = e.code.as_str(),
                detail = e.detail.as_str(),
                "preset dello sviluppo non ricordati"
            );
        }
        // Le scelte vive si salvano prima che l'attività parta.
        if let Err(e) = state
            .raw_review
            .save(state.paths.root(), &request.session_id)
        {
            tracing::warn!(
                code = e.code.as_str(),
                detail = e.detail.as_str(),
                "sessione non salvata prima dello sviluppo"
            );
        }
        state
            .jobs
            .submit(job::JOB_KIND, job::job_params(&resolved.params))
    })
    .await
}

/// Riepilogo di uno sviluppo concluso (convertite, saltate, errori per RAF con il motivo, o il
/// motivo per cui si è fermato). `null` finché l'attività non ha finito o dopo un riavvio.
#[tauri::command]
#[specta::specta]
pub async fn raw_review_develop_report(
    state: State<'_, AppState>,
    job_id: i64,
) -> AppResult<Option<DevelopReport>> {
    Ok(state.develop_reports.get(job_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raw_review::Workers;
    use pc_raw_review::scan::{Layout, VARIANTS_DIR_NAME};
    use std::fs;
    use std::path::PathBuf;

    const NO_WORKERS: Workers = Workers {
        interactive: 1,
        prefetch: 0,
        thumbnails: 0,
    };

    fn jpeg(path: &Path) {
        let img =
            image::RgbImage::from_fn(64, 48, |x, y| image::Rgb([(x % 251) as u8, y as u8, 90]));
        img.save_with_format(path, image::ImageFormat::Jpeg)
            .unwrap();
    }

    struct Fixture {
        tmp: tempfile::TempDir,
        photos: PathBuf,
        data: PathBuf,
        state: ReviewState,
    }

    /// Cartella con 3 RAF (contenuto finto: la scansione guarda solo i nomi), un JPG accanto e le
    /// varianti già sviluppate del preset "C1 Sim" per i primi due scatti.
    fn fixture() -> Fixture {
        let tmp = tempfile::tempdir().unwrap();
        let photos = tmp.path().join("foto");
        fs::create_dir_all(&photos).unwrap();
        for n in 1..=3 {
            fs::write(photos.join(format!("DSCF{n:04}.RAF")), format!("raf {n}")).unwrap();
        }
        jpeg(&photos.join("DSCF0001.JPG"));
        let c1 = photos.join(VARIANTS_DIR_NAME).join("C1 Sim");
        fs::create_dir_all(&c1).unwrap();
        for n in 1..=2 {
            jpeg(&c1.join(format!("DSCF{n:04}.jpg")));
        }
        let data = tmp.path().join("dati");
        let state = ReviewState::new(
            &data,
            1 << 30,
            pc_governor::MemoryGovernor::new(2048),
            NO_WORKERS,
        );
        Fixture {
            tmp,
            photos,
            data,
            state,
        }
    }

    fn tree(dir: &Path) -> Vec<(PathBuf, u64)> {
        let mut out = Vec::new();
        let mut stack = vec![dir.to_path_buf()];
        while let Some(d) = stack.pop() {
            for e in fs::read_dir(d).unwrap().flatten() {
                let meta = e.metadata().unwrap();
                if meta.is_dir() {
                    stack.push(e.path());
                } else {
                    out.push((e.path(), meta.len()));
                }
            }
        }
        out.sort();
        out
    }

    fn request(session_id: &str, shots: Option<&[&str]>) -> RawReviewDevelopRequest {
        RawReviewDevelopRequest {
            session_id: session_id.into(),
            shot_keys: shots.map(|s| s.iter().map(|k| k.to_string()).collect()),
            preset_ids: vec!["asshot".into(), "C1".into()],
            output_dir: None,
            overwrite: false,
        }
    }

    #[test]
    fn the_rafs_come_from_the_session_index_only() {
        let f = fixture();
        let before = tree(&f.photos);
        let opened = f.state.open_folder(&f.data, &f.photos).unwrap();
        let all = f.state.develop_sources(&opened.session_id, None).unwrap();
        assert_eq!(all.rafs.len(), 3);
        assert!(all.rafs.iter().all(|r| r.to_lowercase().ends_with(".raf")));
        assert!(all.default_output_dir.ends_with(VARIANTS_DIR_NAME));
        let some = f
            .state
            .develop_sources(
                &opened.session_id,
                Some(&["dscf0003".to_string(), "dscf0001".to_string()]),
            )
            .unwrap();
        assert_eq!(some.rafs.len(), 2);
        assert!(some.rafs[0].ends_with("DSCF0003.RAF"), "{:?}", some.rafs);
        // Uno scatto sconosciuto o una sessione chiusa non danno percorsi.
        assert!(f
            .state
            .develop_sources(&opened.session_id, Some(&["../../etc/passwd".to_string()]))
            .is_err());
        assert!(f.state.develop_sources("nessuna", None).is_err());
        assert_eq!(
            tree(&f.photos),
            before,
            "nessuna scrittura nella cartella delle foto"
        );
    }

    #[test]
    fn a_folder_without_rafs_cannot_be_developed() {
        let tmp = tempfile::tempdir().unwrap();
        let photos = tmp.path().join("foto");
        for preset in ["Velvia", "Acros"] {
            fs::create_dir_all(photos.join(preset)).unwrap();
            jpeg(&photos.join(preset).join("DSCF0001.jpg"));
        }
        let data = tmp.path().join("dati");
        let state = ReviewState::new(
            &data,
            1 << 30,
            pc_governor::MemoryGovernor::new(2048),
            NO_WORKERS,
        );
        let opened = state.open_folder(&data, &photos).unwrap();
        assert!(state.develop_sources(&opened.session_id, None).is_err());
    }

    #[test]
    fn the_request_is_resolved_into_job_params() {
        let f = fixture();
        let opened = f.state.open_folder(&f.data, &f.photos).unwrap();
        let resolved =
            resolve(&f.state, &request(&opened.session_id, Some(&["dscf0002"]))).unwrap();
        assert_eq!(resolved.params.sources.len(), 1);
        assert!(resolved
            .params
            .output_dir
            .as_deref()
            .unwrap()
            .ends_with(VARIANTS_DIR_NAME));
        assert_eq!(resolved.request.preset_ids, ["asshot", "C1"]);
        // Il percorso di output deve essere assoluto; un elenco di preset vuoto si rifiuta.
        let mut relative = request(&opened.session_id, None);
        relative.output_dir = Some("varianti".into());
        assert!(resolve(&f.state, &relative).is_err());
        let mut none = request(&opened.session_id, None);
        none.preset_ids.clear();
        assert!(resolve(&f.state, &none).is_err());
        let mut elsewhere = request(&opened.session_id, None);
        elsewhere.output_dir = Some(f.data.join("altrove").display().to_string());
        let resolved = resolve(&f.state, &elsewhere).unwrap();
        assert!(resolved.params.output_dir.unwrap().ends_with("altrove"));
    }

    #[test]
    fn the_plan_ignores_presets_the_camera_does_not_have() {
        let f = fixture();
        let opened = f.state.open_folder(&f.data, &f.photos).unwrap();
        let resolved = resolve(&f.state, &request(&opened.session_id, None)).unwrap();
        // Senza fotocamera resta "Come scattato": C1 si segnala come non disponibile.
        let plan = plan_of(&resolved, &[PresetDef::as_shot()]).unwrap();
        assert_eq!(plan.raf_count, 3);
        assert_eq!(plan.total_conversions, 3);
        assert_eq!(plan.pending_conversions, 3);
        assert!(
            plan.warnings.iter().any(|w| w.contains("C1")),
            "{:?}",
            plan.warnings
        );
    }

    #[test]
    fn a_session_refresh_after_development_shows_the_new_presets_and_keeps_the_choices() {
        let f = fixture();
        let before = tree(&f.photos);
        let opened = f.state.open_folder(&f.data, &f.photos).unwrap();
        assert_eq!(opened.presets, ["C1 Sim"]);
        let sid = opened.session_id.clone();
        f.state
            .set_winner(&sid, "dscf0001", Some("C1 Sim"))
            .unwrap();
        f.state.set_rating(&sid, "dscf0002", 4).unwrap();
        let old = f.state.image_keys(&sid, "dscf0001", 1024).unwrap();
        assert_eq!(tree(&f.photos), before);

        // Lo sviluppo scrive un nuovo preset e completa il vecchio.
        let out = f.photos.join(VARIANTS_DIR_NAME);
        for (preset, shots) in [("C2 Sim", 3), ("C1 Sim", 3)] {
            fs::create_dir_all(out.join(preset)).unwrap();
            for n in 1..=shots {
                jpeg(&out.join(preset).join(format!("DSCF{n:04}.jpg")));
            }
        }
        let refreshed = f
            .state
            .refresh_from_disk(&f.data, &sid, Some(&out.display().to_string()))
            .unwrap();
        assert_eq!(
            refreshed.session_id, sid,
            "stesso id: la UI non riapre la cartella"
        );
        assert_eq!(refreshed.presets, ["C1 Sim", "C2 Sim"]);
        assert!(refreshed
            .session
            .panel_presets
            .contains(&"C2 Sim".to_string()));
        // Scelte e valutazioni sono intatte.
        let d1 = &refreshed.session.decisions["dscf0001"];
        assert_eq!(d1.winner_preset.as_deref(), Some("C1 Sim"));
        assert_eq!(refreshed.session.decisions["dscf0002"].rating, 4);
        let shot3 = refreshed
            .shots
            .iter()
            .find(|s| s.key == "dscf0003")
            .unwrap();
        assert_eq!(shot3.variants.len(), 3, "anteprima RAW + C1 + C2");
        // Le chiavi già date restano valide e quelle nuove si servono.
        let c1 = old.iter().find(|k| k.preset == "C1 Sim").unwrap();
        assert_eq!(f.state.serve(&c1.key).status(), 200);
        let keys = f.state.image_keys(&sid, "dscf0003", 1024).unwrap();
        assert_eq!(keys.len(), 3);
        let c2 = keys.iter().find(|k| k.preset == "C2 Sim").unwrap();
        assert_eq!(f.state.serve(&c2.key).status(), 200);
        // La sessione aggiornata è salvata nei dati dell'app, non nella cartella delle foto.
        let saved = pc_raw_review::session::load(&f.data, &refreshed.session.source_root)
            .unwrap()
            .unwrap();
        assert!(saved.presets.contains(&"C2 Sim".to_string()));
        assert!(matches!(saved.layout, Some(Layout::RafWithVariants { .. })));
        // Fuori dalle cartelle dei preset la cartella delle foto non è cambiata.
        let outside = |files: Vec<(PathBuf, u64)>| -> Vec<(PathBuf, u64)> {
            files
                .into_iter()
                .filter(|(p, _)| !p.starts_with(&out))
                .collect()
        };
        assert_eq!(outside(tree(&f.photos)), outside(before));
    }

    #[test]
    fn an_external_output_folder_becomes_the_session_default() {
        let f = fixture();
        let opened = f.state.open_folder(&f.data, &f.photos).unwrap();
        let sid = opened.session_id.clone();
        let external = f.tmp.path().join("disco-esterno");
        fs::create_dir_all(external.join("C5 Sim")).unwrap();
        for n in 1..=3 {
            jpeg(&external.join("C5 Sim").join(format!("DSCF{n:04}.jpg")));
        }
        let refreshed = f
            .state
            .refresh_from_disk(&f.data, &sid, Some(&external.display().to_string()))
            .unwrap();
        assert_eq!(refreshed.presets, ["C5 Sim"]);
        let again = f.state.develop_sources(&sid, None).unwrap();
        assert_eq!(PathBuf::from(&again.default_output_dir), external);
        // Una sessione sconosciuta non si aggiorna.
        assert!(f.state.refresh_from_disk(&f.data, "nessuna", None).is_err());
    }
}
