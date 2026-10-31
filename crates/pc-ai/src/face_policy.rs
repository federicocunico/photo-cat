//! I risultati in volo precedenti alla disattivazione/pulizia non possono essere risalvati.
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
pub struct FacePolicy {
    enabled: AtomicBool,
    generation: AtomicU64,
}
impl FacePolicy {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled: AtomicBool::new(enabled),
            generation: AtomicU64::new(0),
        }
    }
    pub fn set_enabled(&self, enabled: bool) {
        if enabled {
            self.enabled.store(true, Ordering::Release);
        } else {
            self.invalidate();
        }
    }
    pub fn invalidate(&self) {
        self.enabled.store(false, Ordering::Release);
        self.generation.fetch_add(1, Ordering::AcqRel);
    }
    pub fn token(&self) -> Option<u64> {
        self.enabled
            .load(Ordering::Acquire)
            .then(|| self.generation.load(Ordering::Acquire))
    }
    pub fn is_current(&self, token: u64) -> bool {
        self.enabled.load(Ordering::Acquire) && self.generation.load(Ordering::Acquire) == token
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clearing_invalidates_queued_results_even_after_reenable() {
        let p = FacePolicy::new(true);
        let token = p.token().unwrap();
        p.invalidate();
        assert!(!p.is_current(token));
        p.set_enabled(true);
        assert!(!p.is_current(token));
        assert!(p.is_current(p.token().unwrap()));
    }
}
