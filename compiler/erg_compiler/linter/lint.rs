use erg_common::Str;

use crate::hir::HIR;
use crate::error::CompileWarnings;

#[derive(Debug, Default)]
pub struct Linter {
    _used: Vec<Str>,
}

impl Linter {
    pub fn new() -> Self {
        Self {
            _used: Vec::new(),
        }
    }

    pub fn lint(&mut self, _hir: &HIR) -> CompileWarnings {
        CompileWarnings::empty()
    }
}
