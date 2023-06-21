use crate::effectcheck::SideEffectChecker;
use crate::hir::*;
use crate::module::SharedCompilerResource;
// use crate::erg_common::traits::Stream;

#[derive(Debug)]
pub struct HIROptimizer {
    shared: SharedCompilerResource,
}

impl HIROptimizer {
    pub fn optimize(shared: SharedCompilerResource, hir: HIR) -> HIR {
        let mut optimizer = HIROptimizer { shared };
        optimizer.eliminate_dead_code(hir)
    }

    fn _fold_constants(&mut self, mut _hir: HIR) -> HIR {
        todo!()
    }

    fn eliminate_unused_variables(&mut self, mut hir: HIR) -> HIR {
        for chunk in hir.module.iter_mut() {
            self.eliminate_unused_def(chunk);
        }
        hir
    }

    fn eliminate_unused_def(&mut self, expr: &mut Expr) {
        match expr {
            Expr::Def(def) => {
                if self
                    .shared
                    .index
                    .get_refs(&def.sig.ident().vi.def_loc)
                    .unwrap()
                    .referrers
                    .is_empty()
                    && SideEffectChecker::is_pure(expr)
                {
                    *expr = Expr::Dummy(Dummy::empty());
                }
            }
            Expr::Call(call) => {
                for arg in call.args.pos_args.iter_mut() {
                    self.eliminate_unused_def(&mut arg.expr);
                }
            }
            Expr::Code(block) | Expr::Compound(block) => {
                for chunk in block.iter_mut() {
                    self.eliminate_unused_def(chunk);
                }
            }
            Expr::Lambda(lambda) => {
                for chunk in lambda.body.iter_mut() {
                    self.eliminate_unused_def(chunk);
                }
            }
            _ => {}
        }
    }

    fn eliminate_dead_code(&mut self, hir: HIR) -> HIR {
        let hir = self.eliminate_discarded_variables(hir);
        self.eliminate_unused_variables(hir)
    }

    /// ```erg
    /// _ = 1
    /// (a, _) = (1, True)
    /// ```
    /// ↓
    /// ```erg
    /// a = 1
    /// ```
    fn eliminate_discarded_variables(&mut self, hir: HIR) -> HIR {
        hir
    }
}
