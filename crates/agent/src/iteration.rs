use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub struct IterationBudget {
    max_total: usize,
    used: Arc<AtomicUsize>,
}

impl IterationBudget {
    pub fn new(max_total: usize) -> Self {
        Self {
            max_total,
            used: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn consume(&self) -> bool {
        let current = self.used.load(Ordering::Relaxed);
        if current >= self.max_total {
            return false;
        }
        self.used.store(current + 1, Ordering::Relaxed);
        true
    }

    pub fn refund(&self) {
        let current = self.used.load(Ordering::Relaxed);
        if current > 0 {
            self.used.store(current - 1, Ordering::Relaxed);
        }
    }

    pub fn remaining(&self) -> usize {
        let used = self.used.load(Ordering::Relaxed);
        self.max_total.saturating_sub(used)
    }
}
