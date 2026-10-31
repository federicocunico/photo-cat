//! Governor delle risorse di photo-cat: misura RAM e CPU, calcola il budget
//! di memoria e regola concorrenza, batch e modelli residenti.

pub mod budget;
pub mod governor;
pub mod monitor;
pub mod probe;

pub use budget::{compute_budget, Budget};
pub use governor::{GovernorSnapshot, MemoryGovernor, Permit, Resident};
pub use monitor::{PressureMonitor, PressureState, ResourceSnapshot, Thresholds};
pub use probe::{FakeProbe, SysinfoProbe, SystemProbe, SystemSnapshot};

use pc_core::config::ResourcesConfig;

/// Rileva le risorse all'avvio, le registra nei log e calcola il budget.
pub fn detect(probe: &dyn SystemProbe, cfg: &ResourcesConfig) -> (SystemSnapshot, Budget) {
    let sys = probe.snapshot();
    let budget = compute_budget(
        sys.total_mb,
        sys.available_mb,
        cfg.profile,
        cfg.ram_limit_mb,
    );
    tracing::info!(
        total_mb = sys.total_mb,
        available_mb = sys.available_mb,
        logical_cores = sys.logical_cores,
        physical_cores = sys.physical_cores,
        profile = ?cfg.profile,
        budget_mb = budget.budget_mb,
        limited_by = ?budget.limited_by,
        "risorse di sistema rilevate"
    );
    (sys, budget)
}

/// Applica nuove impostazioni risorse (cambio profilo o limite RAM): i job ammessi da qui in
/// poi usano il nuovo budget. Una riduzione temporanea per pressione di memoria resta in vigore.
pub fn apply_resources(
    gov: &MemoryGovernor,
    probe: &dyn SystemProbe,
    cfg: &ResourcesConfig,
) -> Budget {
    let (_, budget) = detect(probe, cfg);
    let prior = gov.snapshot();
    gov.set_budget(budget.budget_mb);
    if prior.ai_paused || prior.capacity_mb < prior.budget_mb {
        gov.set_capacity(prior.capacity_mb.min(budget.budget_mb));
    }
    budget
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn detect_logs_system_resources() {
        let tmp = tempfile::tempdir().unwrap();
        let (sub, diag) = pc_diag::build(tmp.path(), &pc_core::config::LoggingConfig::default());
        let sink = Arc::new(pc_diag::MemorySink::default());
        diag.hub().add_sink(sink.clone());
        let (sys, budget) = tracing::subscriber::with_default(sub, || {
            detect(&SysinfoProbe::default(), &ResourcesConfig::default())
        });
        assert!(sys.total_mb > 0 && budget.budget_mb >= budget::MIN_BUDGET_MB);
        let lines = sink.lines.lock();
        let line = lines
            .iter()
            .find(|l| l.message == "risorse di sistema rilevate")
            .unwrap();
        assert!(line.fields.iter().any(|(k, _)| k == "total_mb"));
        assert!(line.fields.iter().any(|(k, _)| k == "budget_mb"));
    }
}
