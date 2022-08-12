use crate::error::CompileWarnings;
use crate::hir::HIR;

#[derive(Debug)]
pub struct HIROptimizer {}

impl HIROptimizer {
    pub fn fold_constants(&mut self, mut _hir: HIR) -> HIR {
        todo!()
    }

    pub fn eliminate_unused_variables(&mut self, mut _hir: HIR) -> (HIR, CompileWarnings) {
        todo!()
    }

    pub fn eliminate_dead_code(&mut self, mut _hir: HIR) -> (HIR, CompileWarnings) {
        todo!()
    }
}
