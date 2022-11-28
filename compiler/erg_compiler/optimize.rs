use crate::artifact::CompleteArtifact;
use crate::error::CompileWarnings;
use crate::hir::*;
// use crate::erg_common::traits::Stream;

#[derive(Debug)]
pub struct HIROptimizer {}

impl HIROptimizer {
    pub fn optimize(hir: HIR) -> CompleteArtifact {
        let mut optimizer = HIROptimizer {};
        optimizer.eliminate_dead_code(hir)
    }

    fn _fold_constants(&mut self, mut _hir: HIR) -> HIR {
        todo!()
    }

    fn _eliminate_unused_variables(&mut self, mut _hir: HIR) -> (HIR, CompileWarnings) {
        todo!()
    }

    fn eliminate_dead_code(&mut self, hir: HIR) -> CompleteArtifact {
        CompleteArtifact::new(
            self.eliminate_discarded_variables(hir),
            CompileWarnings::empty(),
        )
    }

    /// ```erg
    /// _ = 1
    /// (a, _) = (1, True)
    /// ```
    /// ↓
    /// ```erg
    /// a = 1
    /// ```
    fn eliminate_discarded_variables(&mut self, mut _hir: HIR) -> HIR {
        todo!()
    }
}
