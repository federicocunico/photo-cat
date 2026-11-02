//! Recupero dalla terminazione del processo dell'interfaccia (WebView).
//!
//! Il motore (catalogo, job) vive nel processo dell'app: se la WebView termina, i job
//! continuano. Si registra l'evento con memoria e job attivi, si ricarica l'interfaccia e,
//! al rimontaggio, l'UI mostra l'avviso. Dopo [`MAX_TERMINATIONS`] terminazioni in
//! [`WINDOW`] si smette di ricaricare: job in pausa e finestra di errore con il percorso dei log.

use crate::state::AppState;
use parking_lot::Mutex;
use serde::Serialize;
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

/// Evento inviato all'UI ricaricata (che lo legge anche da `app_info` al montaggio).
pub const UI_RECOVERED: &str = "app://ui-recovered";
/// Richiesta all'UI di liberare le cache (memoria della WebView oltre la quota).
pub const TRIM_CACHES: &str = "app://trim-caches";

pub const MAX_TERMINATIONS: usize = 3;
pub const WINDOW: Duration = Duration::from_secs(5 * 60);

/// Ultima terminazione dell'interfaccia, mostrata dall'UI dopo la ricarica.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UiRecovery {
    pub ts: chrono::DateTime<chrono::Utc>,
    /// Processo terminato e motivo, come li riporta la WebView.
    pub reason: String,
    /// Terminazioni negli ultimi 5 minuti.
    pub count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Reload,
    GiveUp,
}

#[derive(Default)]
pub struct Recovery {
    times: Mutex<VecDeque<Instant>>,
    pending: Mutex<Option<UiRecovery>>,
}

impl Recovery {
    /// Registra una terminazione all'istante `now` e decide cosa fare.
    pub fn record(&self, now: Instant) -> (Action, usize) {
        let mut times = self.times.lock();
        while times
            .front()
            .is_some_and(|t| now.duration_since(*t) > WINDOW)
        {
            times.pop_front();
        }
        times.push_back(now);
        let n = times.len();
        let action = if n >= MAX_TERMINATIONS {
            Action::GiveUp
        } else {
            Action::Reload
        };
        (action, n)
    }

    /// Avviso da mostrare all'UI ricaricata (una volta sola).
    pub fn take_pending(&self) -> Option<UiRecovery> {
        self.pending.lock().take()
    }
}

/// La WebView è terminata (`reason` dalla piattaforma). `reload` ricarica la pagina.
pub fn on_ui_terminated(app: &AppHandle, reason: String, reload: impl FnOnce()) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let (action, count) = state.recovery.record(Instant::now());
    let resources = crate::commands::get_resources(state.clone());
    let active: Vec<String> = crate::active_job_labels(&state.jobs);
    tracing::error!(
        code = "INTERNAL",
        reason = reason.as_str(),
        count,
        available_mb = resources.system.available_mb,
        app_mb = resources.system.process_mb,
        webview_mb = resources.system.webview_mb,
        jobs = ?active,
        "interfaccia terminata (processo della WebView)"
    );
    match action {
        Action::Reload => {
            *state.recovery.pending.lock() = Some(UiRecovery {
                ts: chrono::Utc::now(),
                reason,
                count: count as u32,
            });
            reload();
            let _ = app.emit(UI_RECOVERED, ());
        }
        Action::GiveUp => {
            for job in state.jobs.snapshots() {
                if job.state == pc_jobs::JobState::Running {
                    let _ = state.jobs.pause(job.id);
                }
            }
            tracing::error!(
                code = "INTERNAL",
                count,
                "interfaccia terminata più volte: job in pausa, ricarica automatica sospesa"
            );
            let logs = state.paths.logs().display().to_string();
            show_fatal_dialog(app, &logs);
        }
    }
}

fn show_fatal_dialog(app: &AppHandle, logs: &str) {
    use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
    let text = format!(
        "L'interfaccia di photo-cat si è chiusa {MAX_TERMINATIONS} volte in pochi minuti.\n\
         I job in corso sono stati messi in pausa: il catalogo è al sicuro.\n\n\
         Log: {logs}\n\n\
         Riavvia photo-cat e crea il pacchetto diagnostico da Diagnostica › Esporta pacchetto.\n\n\
         The photo-cat interface closed {MAX_TERMINATIONS} times in a few minutes. Running jobs \
         were paused. Restart photo-cat and export the diagnostics bundle (Diagnostics › Export)."
    );
    app.dialog()
        .message(text)
        .title("photo-cat")
        .kind(MessageDialogKind::Error)
        .show(|_| {});
}

/// Si registra agli eventi di terminazione della WebView della finestra principale.
pub fn watch(app: &AppHandle) {
    let Some(window) = app.webview_windows().into_values().next() else {
        return;
    };
    let handle = app.clone();
    let res = window.with_webview(move |wv| platform::watch(handle, wv));
    if let Err(e) = res {
        tracing::warn!(detail = %e, "terminazioni della WebView non monitorate");
    }
}

/// Su pressione o memoria della WebView oltre la quota: chiede alla WebView di ridurre la
/// memoria (WebView2 `MemoryUsageTargetLevel`, se il runtime lo supporta).
pub fn set_low_memory(app: &AppHandle, low: bool) {
    #[cfg(windows)]
    for window in app.webview_windows().into_values() {
        let _ = window.with_webview(move |wv| platform::set_low_memory(wv, low));
    }
    #[cfg(not(windows))]
    let _ = (app, low);
}

#[cfg(windows)]
mod platform {
    use tauri::webview::PlatformWebview;
    use webview2_com::Microsoft::Web::WebView2::Win32::*;
    use webview2_com::ProcessFailedEventHandler;
    use windows_core::Interface;

    fn kind_name(k: COREWEBVIEW2_PROCESS_FAILED_KIND) -> &'static str {
        match k {
            COREWEBVIEW2_PROCESS_FAILED_KIND_BROWSER_PROCESS_EXITED => "browser",
            COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED => "render",
            COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_UNRESPONSIVE => "render (bloccato)",
            COREWEBVIEW2_PROCESS_FAILED_KIND_FRAME_RENDER_PROCESS_EXITED => "frame",
            COREWEBVIEW2_PROCESS_FAILED_KIND_GPU_PROCESS_EXITED => "gpu",
            COREWEBVIEW2_PROCESS_FAILED_KIND_UTILITY_PROCESS_EXITED => "utility",
            _ => "altro",
        }
    }

    pub fn watch(app: tauri::AppHandle, wv: PlatformWebview) {
        // SAFETY: API COM di WebView2 chiamate sul thread dell'interfaccia (with_webview).
        let core = match unsafe { wv.controller().CoreWebView2() } {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(detail = %e, "CoreWebView2 non disponibile");
                return;
            }
        };
        let handler = ProcessFailedEventHandler::create(Box::new(move |sender, args| {
            let Some(args) = args else {
                return Ok(());
            };
            let mut kind = COREWEBVIEW2_PROCESS_FAILED_KIND::default();
            // SAFETY: argomenti validi per la durata del callback.
            unsafe { args.ProcessFailedKind(&mut kind)? };
            let mut reason = format!("processo {}", kind_name(kind));
            if let Ok(a2) = args.cast::<ICoreWebView2ProcessFailedEventArgs2>() {
                let (mut r, mut code) = (COREWEBVIEW2_PROCESS_FAILED_REASON::default(), 0i32);
                // SAFETY: come sopra.
                unsafe {
                    let _ = a2.Reason(&mut r);
                    let _ = a2.ExitCode(&mut code);
                }
                reason.push_str(&format!(", motivo {}, codice di uscita {code:#x}", r.0));
            }
            // Un processo GPU o di servizio viene riavviato da WebView2 da solo: si registra.
            let fatal = matches!(
                kind,
                COREWEBVIEW2_PROCESS_FAILED_KIND_BROWSER_PROCESS_EXITED
                    | COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED
                    | COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_UNRESPONSIVE
            );
            if !fatal {
                tracing::warn!(
                    code = "INTERNAL",
                    reason = reason.as_str(),
                    "processo secondario della WebView terminato"
                );
                return Ok(());
            }
            let browser = kind == COREWEBVIEW2_PROCESS_FAILED_KIND_BROWSER_PROCESS_EXITED;
            let app2 = app.clone();
            super::on_ui_terminated(&app, reason, move || {
                if browser {
                    // Il processo browser non c'è più: la WebView va ricreata.
                    super::recreate_main_window(&app2);
                } else if let Some(core) = sender {
                    // SAFETY: COM sul thread dell'interfaccia.
                    let _ = unsafe { core.Reload() };
                }
            });
            Ok(())
        }));
        let mut token = 0i64;
        // SAFETY: registrazione dell'handler; il token non serve (vale per tutta la sessione).
        if let Err(e) = unsafe { core.add_ProcessFailed(&handler, &mut token) } {
            tracing::warn!(detail = %e, "registrazione ProcessFailed fallita");
        }
    }

    pub fn set_low_memory(wv: PlatformWebview, low: bool) {
        // SAFETY: API COM sul thread dell'interfaccia; ICoreWebView2_19 manca nei runtime vecchi.
        unsafe {
            let Ok(core) = wv.controller().CoreWebView2() else {
                return;
            };
            if let Ok(c19) = core.cast::<ICoreWebView2_19>() {
                let level = if low {
                    COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW
                } else {
                    COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL
                };
                let _ = c19.SetMemoryUsageTargetLevel(level);
            }
        }
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use tauri::webview::PlatformWebview;
    use webkit2gtk::{WebProcessTerminationReason, WebViewExt};

    pub fn watch(app: tauri::AppHandle, wv: PlatformWebview) {
        wv.inner().connect_web_process_terminated(move |view, why| {
            let reason = match why {
                WebProcessTerminationReason::Crashed => "processo web: crash".to_string(),
                WebProcessTerminationReason::ExceededMemoryLimit => {
                    "processo web: limite di memoria superato".to_string()
                }
                other => format!("processo web: {other:?}"),
            };
            super::on_ui_terminated(&app, reason, || view.reload());
        });
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
mod platform {
    // macOS: si usa solo il marcatore di sessione.
    pub fn watch(_app: tauri::AppHandle, _wv: tauri::webview::PlatformWebview) {}
}

/// Ricrea la finestra principale (dopo la terminazione del processo browser di WebView2).
#[cfg(windows)]
fn recreate_main_window(app: &AppHandle) {
    let app = app.clone();
    let main = app.clone();
    let _ = main.run_on_main_thread(move || {
        let Some(cfg) = app.config().app.windows.first().cloned() else {
            return;
        };
        if let Some(old) = app.get_webview_window(&cfg.label) {
            *app.state::<AppState>().recreating.lock() = true;
            let _ = old.destroy();
        }
        match tauri::WebviewWindowBuilder::from_config(&app, &cfg).and_then(|b| b.build()) {
            Ok(_) => watch(&app),
            Err(e) => tracing::error!(code = "INTERNAL", detail = %e, "finestra non ricreata"),
        }
        *app.state::<AppState>().recreating.lock() = false;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn third_termination_within_five_minutes_gives_up() {
        let r = Recovery::default();
        let t0 = Instant::now();
        assert_eq!(r.record(t0), (Action::Reload, 1));
        assert_eq!(r.record(t0 + Duration::from_secs(60)), (Action::Reload, 2));
        assert_eq!(r.record(t0 + Duration::from_secs(120)).0, Action::GiveUp);
    }

    #[test]
    fn terminations_far_apart_keep_reloading() {
        let r = Recovery::default();
        let t0 = Instant::now();
        for i in 0..5 {
            let (a, n) = r.record(t0 + Duration::from_secs(400 * i));
            assert_eq!(a, Action::Reload);
            assert_eq!(n, 1);
        }
    }
}
