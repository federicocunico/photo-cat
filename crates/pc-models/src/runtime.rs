//! ONNX Runtime: caricamento dinamico della libreria distribuita con l'app,
//! rilevamento dei provider di esecuzione e creazione delle sessioni con
//! ripiego automatico sulla CPU.

use ort::ep::{self, ExecutionProvider};
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use pc_core::config::ExecutionProvider as Preferred;
use pc_core::{AppError, AppResult, ErrorCode};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    Cpu,
    DirectMl,
    Cuda,
    CoreMl,
}

static INIT: OnceLock<Result<String, String>> = OnceLock::new();

/// Carica la libreria ONNX Runtime (una sola volta per processo). Restituisce la versione.
pub fn init(dll: &Path) -> AppResult<String> {
    let res = INIT.get_or_init(|| {
        preload_vc_runtime(dll);
        if !dll.is_file() {
            return Err(format!(
                "libreria ONNX Runtime non trovata: {}",
                dll.display()
            ));
        }
        probe_library(dll)?;
        match ort::init_from(dll) {
            Ok(builder) => {
                builder.with_name("photo-cat").commit();
                let v = ort::info();
                tracing::info!(path = %dll.display(), info = %v, "ONNX Runtime caricato");
                Ok(v.to_string())
            }
            Err(e) => Err(e.to_string()),
        }
    });
    res.clone()
        .map_err(|e| AppError::new(ErrorCode::ModelUnavailable, e))
}

/// Ripete i controlli di `ort::init_from` prima di chiamarlo: in ort 2.0.0-rc.12 ogni
/// errore di caricamento costruisce un `ort::Error`, che rientra nel caricamento in corso
/// e blocca il thread per sempre (es. una DLL Windows su Linux, o una versione troppo vecchia).
/// Quando si aggiorna `ort`, verificare se il deadlock è stato corretto prima di toglierla.
fn probe_library(dll: &Path) -> Result<(), String> {
    let fail = |e: &dyn std::fmt::Display| format!("{}: {e}", dll.display());
    // SAFETY: è la stessa libreria che ort caricherà subito dopo.
    let lib = unsafe { libloading::Library::new(dll) }.map_err(|e| fail(&e))?;
    let get_base: libloading::Symbol<unsafe extern "C" fn() -> *const ort::sys::OrtApiBase> =
        unsafe { lib.get(b"OrtGetApiBase") }.map_err(|e| fail(&e))?;
    let base = unsafe { get_base() };
    if base.is_null() {
        return Err(fail(&"OrtGetApiBase ha restituito un puntatore nullo"));
    }
    let version = unsafe { std::ffi::CStr::from_ptr(((*base).GetVersionString)()) }
        .to_string_lossy()
        .into_owned();
    let minor = version
        .split('.')
        .nth(1)
        .and_then(|m| m.parse::<u32>().ok())
        .unwrap_or(0);
    if minor < ort::MINOR_VERSION {
        return Err(fail(&format!(
            "versione {version} non supportata, serve almeno 1.{}",
            ort::MINOR_VERSION
        )));
    }
    Ok(())
}

pub fn is_initialized() -> bool {
    matches!(INIT.get(), Some(Ok(_)))
}

/// Perché un provider GPU non è utilizzabile (mostrato all'utente accanto alla scelta).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum UnavailableReason {
    /// Il provider esiste solo su un altro sistema operativo (es. DirectML fuori da Windows).
    OtherPlatform,
    /// Il runtime distribuito con l'app non lo include.
    NotInRuntime,
    /// ONNX Runtime non è caricato: nessuna analisi AI disponibile.
    RuntimeMissing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct UnavailableProvider {
    pub provider: Provider,
    pub reason: UnavailableReason,
}

fn gpu_status(provider: Provider) -> Result<(), UnavailableReason> {
    let supported = match provider {
        Provider::DirectMl => ep::DirectML::default().supported_by_platform(),
        Provider::Cuda => ep::CUDA::default().supported_by_platform(),
        Provider::CoreMl => ep::CoreML::default().supported_by_platform(),
        Provider::Cpu => return Ok(()),
    };
    if !supported {
        return Err(UnavailableReason::OtherPlatform);
    }
    if !is_initialized() {
        return Err(UnavailableReason::RuntimeMissing);
    }
    // Interroga il runtime: va chiamato solo dopo un `init` riuscito.
    let available = match provider {
        Provider::DirectMl => ep::DirectML::default().is_available(),
        Provider::Cuda => ep::CUDA::default().is_available(),
        Provider::CoreMl => ep::CoreML::default().is_available(),
        Provider::Cpu => unreachable!(),
    };
    if available.unwrap_or(false) {
        Ok(())
    } else {
        Err(UnavailableReason::NotInRuntime)
    }
}

const GPU_PROVIDERS: [Provider; 3] = [Provider::DirectMl, Provider::Cuda, Provider::CoreMl];

/// Provider disponibili su questa macchina (la CPU c'è sempre).
pub fn available_providers() -> Vec<Provider> {
    let mut v = vec![Provider::Cpu];
    v.extend(GPU_PROVIDERS.into_iter().filter(|p| gpu_status(*p).is_ok()));
    v
}

/// Provider GPU non utilizzabili, con il motivo.
pub fn unavailable_providers() -> Vec<UnavailableProvider> {
    GPU_PROVIDERS
        .into_iter()
        .filter_map(|provider| {
            gpu_status(provider)
                .err()
                .map(|reason| UnavailableProvider { provider, reason })
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct SessionOptions {
    pub preferred: Preferred,
    pub intra_threads: usize,
    /// Solo test: forza il fallimento del provider GPU (dispositivo inesistente).
    pub force_gpu_failure: bool,
}

impl Default for SessionOptions {
    fn default() -> Self {
        Self {
            preferred: Preferred::Cpu,
            intra_threads: 1,
            force_gpu_failure: false,
        }
    }
}

fn base_builder(opts: &SessionOptions) -> AppResult<ort::session::builder::SessionBuilder> {
    let map = |e: ort::Error<_>| AppError::new(ErrorCode::ModelUnavailable, e.to_string());
    Session::builder()
        .map_err(|e| AppError::new(ErrorCode::ModelUnavailable, e.to_string()))?
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .map_err(map)?
        .with_intra_threads(opts.intra_threads.max(1))
        .map_err(map)?
        // Il pattern è disattivato qui; l'arena CPU è disattivata nell'EP sotto.
        .with_memory_pattern(false)
        .map_err(map)
}

/// Crea una sessione sul provider preferito; se fallisce ripiega sulla CPU e lo registra.
pub fn load(model: &Path, opts: &SessionOptions) -> AppResult<(Session, Provider)> {
    if !is_initialized() {
        return Err(AppError::new(
            ErrorCode::ModelUnavailable,
            "ONNX Runtime non inizializzato",
        ));
    }
    let available = available_providers();
    let requested = match opts.preferred {
        Preferred::Cpu => None,
        Preferred::Auto => available.iter().copied().find(|p| *p != Provider::Cpu),
        Preferred::DirectMl => Some(Provider::DirectMl),
        Preferred::Cuda => Some(Provider::Cuda),
        Preferred::CoreMl => Some(Provider::CoreMl),
    };
    let cpu = || ep::CPU::default().with_arena_allocator(false).build();
    if let Some(provider) = requested {
        let attempt = base_builder(opts).and_then(|b| {
            if opts.force_gpu_failure || !available.contains(&provider) {
                return Err(AppError::new(
                    ErrorCode::ModelUnavailable,
                    format!("provider {provider:?} non disponibile"),
                ));
            }
            let gpu = match provider {
                Provider::DirectMl => ep::DirectML::default().build(),
                Provider::Cuda => ep::CUDA::default().build(),
                Provider::CoreMl => ep::CoreML::default().build(),
                Provider::Cpu => unreachable!(),
            }
            .error_on_failure();
            b.with_parallel_execution(false)
                .map_err(|e| AppError::new(ErrorCode::ModelUnavailable, e.to_string()))?
                .with_execution_providers([gpu, cpu()])
                .map_err(|e| AppError::new(ErrorCode::ModelUnavailable, e.to_string()))?
                .commit_from_file(model)
                .map_err(|e| AppError::new(ErrorCode::ModelUnavailable, e.to_string()))
        });
        match attempt {
            Ok(s) => return Ok((s, provider)),
            Err(e) => tracing::warn!(
                code = "MODEL_UNAVAILABLE",
                path = %model.display(),
                detail = e.detail.as_str(),
                ?provider,
                "provider GPU non inizializzato: uso la CPU"
            ),
        }
    }
    let session = base_builder(opts)?
        .with_execution_providers([cpu()])
        .map_err(|e| AppError::new(ErrorCode::ModelUnavailable, e.to_string()))?
        .commit_from_file(model)
        .map_err(|e| {
            AppError::new(
                ErrorCode::ModelUnavailable,
                format!("{}: {e}", model.display()),
            )
        })?;
    Ok((session, Provider::Cpu))
}

/// Percorso del runtime negli asset del repository per la piattaforma corrente (test e sviluppo).
pub fn dev_runtime_path() -> std::path::PathBuf {
    let dir = pc_core::platform::asset_dir().unwrap_or("non-distribuito");
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/runtime")
        .join(dir)
        .join(pc_core::platform::onnxruntime_file())
}

pub fn dev_models_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/models")
}

/// Runtime Visual C++ richiesto da `onnxruntime.dll`, in ordine di dipendenza.
pub const VC_RUNTIME: &[&str] = &[
    "vcruntime140.dll",
    "vcruntime140_1.dll",
    "msvcp140.dll",
    "msvcp140_1.dll",
];

/// Carica le DLL del runtime Visual C++ distribuite accanto a `onnxruntime.dll` (Windows
/// senza Visual C++ Redistributable, es. Windows Sandbox o installazioni pulite): una volta
/// caricate nel processo, le dipendenze di ONNX Runtime si risolvono su queste copie.
/// Se il sistema ha già il runtime, Windows usa comunque quello caricato per primo per nome.
#[cfg(windows)]
fn preload_vc_runtime(dll: &Path) {
    let Some(dir) = dll.parent() else { return };
    for name in VC_RUNTIME {
        let path = dir.join(name);
        if !path.is_file() {
            continue;
        }
        // SAFETY: librerie di sistema Microsoft senza codice di inizializzazione dell'app;
        // restano caricate per tutta la vita del processo (come farebbe il loader).
        match unsafe { libloading::Library::new(&path) } {
            Ok(lib) => std::mem::forget(lib),
            Err(e) => {
                tracing::warn!(path = %path.display(), detail = %e, "runtime Visual C++ non caricato")
            }
        }
    }
}

#[cfg(not(windows))]
fn preload_vc_runtime(_dll: &Path) {}

#[cfg(test)]
mod probe_tests {
    use super::*;

    #[test]
    fn providers_of_other_platforms_are_unavailable_with_reason() {
        let other = |p| UnavailableProvider {
            provider: p,
            reason: UnavailableReason::OtherPlatform,
        };
        let list = unavailable_providers();
        if !cfg!(windows) {
            assert!(list.contains(&other(Provider::DirectMl)), "{list:?}");
        }
        if !cfg!(target_vendor = "apple") {
            assert!(list.contains(&other(Provider::CoreMl)), "{list:?}");
        }
    }

    #[test]
    fn probe_rejects_file_that_is_not_a_library() {
        let dir = tempfile::tempdir().unwrap();
        let fake = dir.path().join("onnxruntime.dll");
        std::fs::write(&fake, b"non sono una libreria").unwrap();
        let err = probe_library(&fake).unwrap_err();
        assert!(err.contains("onnxruntime.dll"), "{err}");
    }
}

#[cfg(all(test, feature = "model-tests"))]
mod tests {
    use super::*;

    fn yunet() -> std::path::PathBuf {
        dev_models_dir().join("yunet/face_detection_yunet_2023mar.onnx")
    }

    #[test]
    fn loads_runtime_and_runs_on_cpu() {
        let v = init(&dev_runtime_path()).unwrap();
        assert!(v.contains("ORT"), "{v}");
        let providers = available_providers();
        eprintln!("provider disponibili: {providers:?}");
        assert!(providers.contains(&Provider::Cpu));
        let (s, p) = load(
            &yunet(),
            &SessionOptions {
                preferred: Preferred::Cpu,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(p, Provider::Cpu);
        assert!(!s.inputs().is_empty());
    }

    #[test]
    fn auto_uses_directml_when_available() {
        init(&dev_runtime_path()).unwrap();
        let (_s, p) = load(
            &yunet(),
            &SessionOptions {
                preferred: Preferred::Auto,
                ..Default::default()
            },
        )
        .unwrap();
        let expected = available_providers()
            .into_iter()
            .find(|p| *p != Provider::Cpu)
            .unwrap_or(Provider::Cpu);
        assert_eq!(p, expected);
    }

    #[test]
    fn gpu_failure_falls_back_to_cpu() {
        init(&dev_runtime_path()).unwrap();
        for preferred in [Preferred::DirectMl, Preferred::Cuda, Preferred::CoreMl] {
            let opts = SessionOptions {
                preferred,
                force_gpu_failure: true,
                ..Default::default()
            };
            let (_s, p) = load(&yunet(), &opts).unwrap();
            assert_eq!(p, Provider::Cpu);
        }
    }

    #[cfg(not(windows))]
    #[test]
    fn directml_preference_falls_back_to_cpu_outside_windows() {
        init(&dev_runtime_path()).unwrap();
        assert!(!available_providers().contains(&Provider::DirectMl));
        let opts = SessionOptions {
            preferred: Preferred::DirectMl,
            ..Default::default()
        };
        let (_s, p) = load(&yunet(), &opts).unwrap();
        assert_eq!(p, Provider::Cpu);
    }

    #[test]
    fn missing_model_is_an_error() {
        init(&dev_runtime_path()).unwrap();
        let err = load(Path::new("nope.onnx"), &SessionOptions::default()).unwrap_err();
        assert_eq!(err.code, ErrorCode::ModelUnavailable);
    }
}
