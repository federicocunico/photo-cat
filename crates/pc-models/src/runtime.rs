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

/// Carica `onnxruntime.dll` (una sola volta per processo). Restituisce la versione.
pub fn init(dll: &Path) -> AppResult<String> {
    let res = INIT.get_or_init(|| {
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

/// Provider disponibili su questa macchina (la CPU c'è sempre).
pub fn available_providers() -> Vec<Provider> {
    let mut v = vec![Provider::Cpu];
    if is_initialized()
        && ep::DirectML::default().supported_by_platform()
        && ep::DirectML::default().is_available().unwrap_or(false)
    {
        v.push(Provider::DirectMl);
    }
    if is_initialized()
        && ep::CUDA::default().supported_by_platform()
        && ep::CUDA::default().is_available().unwrap_or(false)
    {
        v.push(Provider::Cuda);
    }
    if is_initialized()
        && ep::CoreML::default().supported_by_platform()
        && ep::CoreML::default().is_available().unwrap_or(false)
    {
        v.push(Provider::CoreMl);
    }
    v
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

/// Percorso della DLL negli asset del repository (test e sviluppo).
pub fn dev_dll_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/runtime/win-x64/onnxruntime.dll")
}

pub fn dev_models_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/models")
}

#[cfg(all(test, feature = "model-tests", windows))]
mod tests {
    use super::*;

    fn yunet() -> std::path::PathBuf {
        dev_models_dir().join("yunet/face_detection_yunet_2023mar.onnx")
    }

    #[test]
    fn loads_runtime_and_runs_on_cpu() {
        let v = init(&dev_dll_path()).unwrap();
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
        init(&dev_dll_path()).unwrap();
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
        init(&dev_dll_path()).unwrap();
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

    #[test]
    fn missing_model_is_an_error() {
        init(&dev_dll_path()).unwrap();
        let err = load(Path::new("nope.onnx"), &SessionOptions::default()).unwrap_err();
        assert_eq!(err.code, ErrorCode::ModelUnavailable);
    }
}
