use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct InterruptFlag {
    flag: Arc<AtomicBool>,
}

impl InterruptFlag {
    pub fn new() -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn set(&self) {
        self.flag.store(true, Ordering::Relaxed);
    }

    pub fn check(&self) -> bool {
        self.flag.load(Ordering::Relaxed)
    }

    pub fn clear(&self) {
        self.flag.store(false, Ordering::Relaxed);
    }

    pub fn as_bool(&self) -> Arc<AtomicBool> {
        self.flag.clone()
    }
}

impl Default for InterruptFlag {
    fn default() -> Self {
        Self::new()
    }
}
