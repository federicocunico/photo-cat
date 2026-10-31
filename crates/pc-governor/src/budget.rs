//! Calcolo del budget di memoria dell'app e stime di costo delle operazioni.

use pc_core::config::ResourceProfile;
use serde::{Deserialize, Serialize};

/// Memoria minima lasciata al sistema operativo e alle altre applicazioni.
pub const OS_RESERVE_MB: u64 = 2048;
/// Sotto questa soglia l'app lavora comunque, ma in modo strettamente sequenziale.
pub const MIN_BUDGET_MB: u64 = 256;

pub fn profile_fraction(profile: ResourceProfile) -> f64 {
    match profile {
        ResourceProfile::Light => 0.25,
        ResourceProfile::Balanced => 0.40,
        ResourceProfile::Performance => 0.60,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Budget {
    pub budget_mb: u64,
    /// Quale vincolo ha determinato il budget.
    pub limited_by: BudgetLimit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum BudgetLimit {
    Profile,
    UserLimit,
    OsReserve,
    Minimum,
}

/// `min(limite utente, frazione profilo × RAM disponibile, RAM totale − riserva OS)`,
/// mai sotto [`MIN_BUDGET_MB`].
pub fn compute_budget(
    total_mb: u64,
    available_mb: u64,
    profile: ResourceProfile,
    user_limit_mb: Option<u64>,
) -> Budget {
    let by_profile = (available_mb as f64 * profile_fraction(profile)) as u64;
    let by_reserve = total_mb.saturating_sub(OS_RESERVE_MB);
    let mut best = (by_profile, BudgetLimit::Profile);
    if by_reserve < best.0 {
        best = (by_reserve, BudgetLimit::OsReserve);
    }
    if let Some(limit) = user_limit_mb {
        if limit < best.0 {
            best = (limit, BudgetLimit::UserLimit);
        }
    }
    if best.0 < MIN_BUDGET_MB {
        best = (MIN_BUDGET_MB, BudgetLimit::Minimum);
    }
    Budget {
        budget_mb: best.0,
        limited_by: best.1,
    }
}

/// Memoria per decodificare un'immagine `w×h` con riduzione `1/scale` (RGBA + margine 50%).
pub fn decode_cost_mb(width: u32, height: u32, scale: u32) -> u64 {
    let s = scale.max(1) as u64;
    let w = (width as u64).div_ceil(s);
    let h = (height as u64).div_ceil(s);
    (w * h * 4 * 3 / 2).div_ceil(1024 * 1024).max(1)
}

/// Memoria per un batch di inferenza: tensori di input `batch × c × h × w` in f32,
/// moltiplicati per un fattore per attivazioni intermedie.
pub fn inference_cost_mb(batch: u32, channels: u32, h: u32, w: u32, activation_factor: f64) -> u64 {
    let bytes = batch as f64 * channels as f64 * h as f64 * w as f64 * 4.0 * activation_factor;
    ((bytes / (1024.0 * 1024.0)).ceil() as u64).max(1)
}

/// Numero di worker per uno stadio: limitato dai core e da quanti costi entrano nel budget.
pub fn recommended_workers(capacity_mb: u64, cost_per_task_mb: u64, logical_cores: usize) -> usize {
    let by_cores = logical_cores.saturating_sub(1).max(1);
    let by_memory = (capacity_mb / cost_per_task_mb.max(1)).max(1) as usize;
    by_cores.min(by_memory)
}

/// Dimensione del batch di inferenza che resta entro `available_mb`.
pub fn recommended_batch(available_mb: u64, cost_per_item_mb: u64, max_batch: u32) -> u32 {
    ((available_mb / cost_per_item_mb.max(1)) as u32).clamp(1, max_batch.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ResourceProfile::*;

    #[test]
    fn budget_matrix() {
        // (totale, disponibile, profilo, limite) → budget atteso
        let cases = [
            (4096, 2000, Balanced, None, 800, BudgetLimit::Profile),
            (4096, 3500, Performance, None, 2048, BudgetLimit::OsReserve),
            (4096, 600, Light, None, MIN_BUDGET_MB, BudgetLimit::Minimum),
            (8192, 5000, Light, None, 1250, BudgetLimit::Profile),
            (8192, 5000, Balanced, None, 2000, BudgetLimit::Profile),
            (8192, 7000, Performance, None, 4200, BudgetLimit::Profile),
            (16384, 10000, Balanced, None, 4000, BudgetLimit::Profile),
            (
                16384,
                10000,
                Balanced,
                Some(4096),
                4000,
                BudgetLimit::Profile,
            ),
            (
                16384,
                12000,
                Performance,
                Some(4096),
                4096,
                BudgetLimit::UserLimit,
            ),
            (32768, 28000, Performance, None, 16800, BudgetLimit::Profile),
            (
                32768,
                28000,
                Balanced,
                Some(2048),
                2048,
                BudgetLimit::UserLimit,
            ),
        ];
        for (total, avail, profile, limit, expected, by) in cases {
            let b = compute_budget(total, avail, profile, limit);
            assert_eq!(
                (b.budget_mb, b.limited_by),
                (expected, by),
                "{total}/{avail} {profile:?} {limit:?}"
            );
        }
    }

    #[test]
    fn user_limit_always_caps() {
        for total in [4096, 8192, 16384, 32768] {
            for profile in [Light, Balanced, Performance] {
                let b = compute_budget(total, total, profile, Some(1024));
                assert!(b.budget_mb <= 1024);
            }
        }
    }

    #[test]
    fn decode_cost_scales_with_dct_reduction() {
        // 50 MP: ~286 MB a piena risoluzione, ~5 MB a 1/8.
        let full = decode_cost_mb(8660, 5774, 1);
        let eighth = decode_cost_mb(8660, 5774, 8);
        assert!((280..=300).contains(&full), "{full}");
        assert!(eighth <= 6, "{eighth}");
    }

    #[test]
    fn workers_and_batch_respect_limits() {
        assert_eq!(recommended_workers(2048, 300, 16), 6);
        assert_eq!(recommended_workers(100_000, 10, 8), 7);
        assert_eq!(recommended_workers(10, 300, 8), 1);
        assert_eq!(recommended_batch(500, 40, 32), 12);
        assert_eq!(recommended_batch(10, 40, 32), 1);
        assert_eq!(inference_cost_mb(1, 3, 224, 224, 1.0), 1);
    }
}
