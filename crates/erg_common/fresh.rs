use std::sync::atomic::AtomicUsize;

use crate::Str;

#[derive(Debug, Default)]
pub struct FreshNameGenerator {
    id: AtomicUsize,
    /// To avoid conflicts with variable names generated in another phase
    prefix: &'static str,
}

impl FreshNameGenerator {
    pub const fn new(prefix: &'static str) -> Self {
        Self {
            id: AtomicUsize::new(0),
            prefix,
        }
    }

    pub fn fresh_varname(&self) -> Str {
        self.id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let i = self.id.load(std::sync::atomic::Ordering::SeqCst);
        Str::from(format!("%v_{}_{i}", self.prefix))
    }

    pub fn fresh_param_name(&self) -> Str {
        self.id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let i = self.id.load(std::sync::atomic::Ordering::SeqCst);
        Str::from(format!("%p_{}_{i}", self.prefix))
    }
}

pub static FRESH_GEN: FreshNameGenerator = FreshNameGenerator::new("global");
