//! Pool dei modelli caricati: ogni sessione occupa una quota residente del
//! governor (stimata dal manifest, poi corretta con la memoria misurata).
//! Se un modello non entra, si scaricano quelli inattivi; se ancora non
//! entra, la richiesta fallisce con `INSUFFICIENT_MEMORY` e il chiamante
//! può procedere in sequenza (modalità "model-major").

use crate::runtime::{self, Provider, SessionOptions};
use crate::store::ModelStore;
use parking_lot::Mutex;
use pc_core::{AppError, AppResult, ErrorCode};
use pc_governor::{MemoryGovernor, Resident, SystemProbe};
use std::collections::BTreeMap;
use std::sync::Arc;

pub struct LoadedModel {
    pub id: String,
    pub provider: Provider,
    pub session: Mutex<ort::session::Session>,
    resident: Mutex<Resident>,
}

impl LoadedModel {
    pub fn resident_mb(&self) -> u64 {
        self.resident.lock().mb()
    }
}

pub struct ModelPool {
    store: Arc<ModelStore>,
    gov: MemoryGovernor,
    probe: Arc<dyn SystemProbe>,
    opts: Mutex<SessionOptions>,
    loaded: Mutex<BTreeMap<String, Arc<LoadedModel>>>,
    loading: Mutex<()>,
}

impl ModelPool {
    pub fn new(
        store: Arc<ModelStore>,
        gov: MemoryGovernor,
        probe: Arc<dyn SystemProbe>,
        opts: SessionOptions,
    ) -> Self {
        Self {
            store,
            gov,
            probe,
            opts: Mutex::new(opts),
            loaded: Mutex::new(BTreeMap::new()),
            loading: Mutex::new(()),
        }
    }

    pub fn set_options(&self, opts: SessionOptions) {
        let _loading = self.loading.lock();
        *self.opts.lock() = opts;
        self.loaded.lock().clear();
    }

    /// Memoria stimata dal manifest.
    pub fn estimate_mb(&self, id: &str) -> u64 {
        self.store
            .manifest()
            .get(id)
            .map(|e| e.memory_mb)
            .unwrap_or(0)
    }

    /// I modelli indicati entrano insieme nella capacità attuale?
    pub fn fit_together(&self, ids: &[&str]) -> bool {
        let unique: std::collections::BTreeSet<_> = ids.iter().copied().collect();
        unique
            .iter()
            .try_fold(0u64, |total, id| {
                total.checked_add(self.store.manifest().get(id)?.memory_mb)
            })
            .is_some_and(|need| need <= self.gov.capacity_mb())
    }

    pub fn is_loaded(&self, id: &str) -> bool {
        self.loaded.lock().contains_key(id)
    }

    pub fn loaded_ids(&self) -> Vec<String> {
        self.loaded.lock().keys().cloned().collect()
    }

    /// Scarica i modelli non in uso (nessun riferimento esterno), tranne `keep`.
    pub fn evict_idle(&self, keep: &str) -> usize {
        let mut loaded = self.loaded.lock();
        let idle: Vec<String> = loaded
            .iter()
            .filter(|(id, m)| id.as_str() != keep && Arc::strong_count(m) == 1)
            .map(|(id, _)| id.clone())
            .collect();
        for id in &idle {
            loaded.remove(id);
            tracing::info!(model = id.as_str(), "modello scaricato dalla memoria");
        }
        idle.len()
    }

    pub fn unload(&self, id: &str) {
        let _loading = self.loading.lock();
        if self.loaded.lock().remove(id).is_some() {
            tracing::info!(model = id, "modello scaricato dalla memoria");
        }
    }

    pub fn unload_all(&self) {
        let _loading = self.loading.lock();
        self.loaded.lock().clear();
    }

    /// Restituisce il modello caricato, caricandolo se serve.
    pub fn get(&self, id: &str) -> AppResult<Arc<LoadedModel>> {
        // Serializza la misura RSS e impedisce due sessioni/quota residente con lo stesso id.
        let _loading = self.loading.lock();
        let path = self.store.verified_path(id).ok_or_else(|| {
            AppError::new(
                ErrorCode::ModelUnavailable,
                format!("modello {id} non verificato"),
            )
        })?;
        if let Some(m) = self.loaded.lock().get(id) {
            return Ok(m.clone());
        }
        let est = self.estimate_mb(id);
        let resident = match self.gov.try_reserve_resident(id, est) {
            Some(r) => r,
            None => {
                self.evict_idle(id);
                self.gov.try_reserve_resident(id, est).ok_or_else(|| {
                    AppError::new(
                        ErrorCode::InsufficientMemory,
                        format!("memoria insufficiente per caricare {id} (~{est} MB)"),
                    )
                })?
            }
        };
        let before = self.probe.snapshot().process_mb;
        let mut opts = self.opts.lock().clone();
        if !self.store.allow_gpu(id) {
            opts.preferred = pc_core::config::ExecutionProvider::Cpu;
        }
        let (session, provider) = runtime::load(&path, &opts)?; // in errore il permesso residente viene rilasciato
        let after = self.probe.snapshot().process_mb;
        let model = Arc::new(LoadedModel {
            id: id.to_string(),
            provider,
            session: Mutex::new(session),
            resident: Mutex::new(resident),
        });
        let measured = after.saturating_sub(before);
        if measured > 0 {
            // La misura del processo è rumorosa: non scendere sotto metà della stima.
            let actual = measured.max(est / 2);
            if !model.resident.lock().adjust(actual) {
                self.evict_idle(id);
                if !model.resident.lock().adjust(actual) {
                    return Err(AppError::new(
                        ErrorCode::InsufficientMemory,
                        format!("{id}: costo misurato {actual} MB superiore al budget residuo"),
                    ));
                }
            }
        }
        tracing::info!(
            model = id,
            ?provider,
            estimate_mb = est,
            measured_mb = measured,
            "modello caricato"
        );
        self.loaded.lock().insert(id.to_string(), model.clone());
        Ok(model)
    }
}

#[cfg(all(test, feature = "model-tests", windows))]
mod tests {
    use super::*;
    use crate::manifest::Manifest;
    use pc_governor::FakeProbe;

    fn pool(budget: u64) -> (ModelPool, tempfile::TempDir) {
        runtime::init(&runtime::dev_dll_path()).unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let store = Arc::new(ModelStore::new(
            Manifest::builtin(),
            tmp.path(),
            Some(runtime::dev_models_dir()),
        ));
        let ids: Vec<String> = ["yunet", "sface"].iter().map(|s| s.to_string()).collect();
        store.check_all();
        store.restore(&ids, None, &std::sync::atomic::AtomicBool::new(false));
        let pool = ModelPool::new(
            store,
            MemoryGovernor::new(budget),
            Arc::new(FakeProbe::new(16384, 8000, 8)),
            SessionOptions {
                preferred: pc_core::config::ExecutionProvider::Cpu,
                ..Default::default()
            },
        );
        (pool, tmp)
    }

    #[test]
    fn models_that_do_not_fit_together_are_swapped() {
        // yunet ~30 MB + sface ~150 MB > 160 MB di budget.
        let (p, _tmp) = pool(160);
        assert!(!p.fit_together(&["yunet", "sface"]));
        let y = p.get("yunet").unwrap();
        assert_eq!(y.provider, Provider::Cpu);
        // Finché yunet è in uso, sface non entra.
        assert_eq!(
            p.get("sface").err().unwrap().code,
            ErrorCode::InsufficientMemory
        );
        drop(y);
        // Inattivo: viene scaricato per far posto.
        let s = p.get("sface").unwrap();
        assert_eq!(p.loaded_ids(), vec!["sface".to_string()]);
        assert_eq!(s.resident_mb(), 150);
    }

    #[test]
    fn models_that_fit_stay_resident_together() {
        let (p, _tmp) = pool(1000);
        assert!(p.fit_together(&["yunet", "sface"]));
        let _a = p.get("yunet").unwrap();
        let _b = p.get("sface").unwrap();
        assert_eq!(p.loaded_ids().len(), 2);
        // Seconda richiesta: stessa istanza.
        assert!(Arc::ptr_eq(&p.get("yunet").unwrap(), &_a));
    }

    #[test]
    fn concurrent_requests_share_a_single_session() {
        let (p, _tmp) = pool(1000);
        let p = Arc::new(p);
        let threads: Vec<_> = (0..4)
            .map(|_| {
                let p = p.clone();
                std::thread::spawn(move || p.get("yunet").unwrap())
            })
            .collect();
        let models: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
        assert!(models.iter().all(|m| Arc::ptr_eq(m, &models[0])));
        assert_eq!(p.gov.snapshot().residents, vec![("yunet".into(), 30)]);
        assert!(!p.fit_together(&["unknown"]));
    }

    #[test]
    fn failed_self_test_blocks_an_already_cached_session() {
        let (p, _tmp) = pool(1000);
        let _loaded = p.get("yunet").unwrap();
        p.store.set_self_test(
            "yunet",
            crate::store::SelfTestResult {
                passed: false,
                avg_ms: 0.0,
                message: "fuori soglia".into(),
                at: 1,
            },
        );
        assert_eq!(
            p.get("yunet").err().unwrap().code,
            ErrorCode::ModelUnavailable
        );
    }

    #[test]
    fn measured_cost_above_budget_releases_session_and_reservation() {
        struct GrowingProbe(std::sync::atomic::AtomicU64);
        impl SystemProbe for GrowingProbe {
            fn snapshot(&self) -> pc_governor::SystemSnapshot {
                let mut s = FakeProbe::new(16384, 8000, 8).snapshot();
                s.process_mb = self.0.fetch_add(500, std::sync::atomic::Ordering::SeqCst);
                s
            }
        }
        let (mut p, _tmp) = pool(160);
        p.probe = Arc::new(GrowingProbe(std::sync::atomic::AtomicU64::new(100)));
        assert_eq!(
            p.get("yunet").err().unwrap().code,
            ErrorCode::InsufficientMemory
        );
        assert!(p.loaded_ids().is_empty());
        assert_eq!(p.gov.snapshot().in_use_mb, 0);
    }
}
