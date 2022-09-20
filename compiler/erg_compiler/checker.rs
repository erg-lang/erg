use erg_common::config::ErgConfig;
use erg_common::error::MultiErrorDisplay;
use erg_common::traits::Runnable;

use erg_parser::ast::AST;
use erg_parser::builder::ASTBuilder;

use crate::effectcheck::SideEffectChecker;
use crate::error::{TyCheckError, TyCheckErrors};
use crate::hir::HIR;
use crate::lower::ASTLowerer;
use crate::ownercheck::OwnershipChecker;

/// Summarize lowering, side-effect checking, and ownership checking
#[derive(Debug)]
pub struct Checker {
    cfg: ErgConfig,
    lowerer: ASTLowerer,
    ownership_checker: OwnershipChecker,
}

impl Runnable for Checker {
    type Err = TyCheckError;
    type Errs = TyCheckErrors;
    const NAME: &'static str = "Erg type-checker";

    fn new(cfg: ErgConfig) -> Self {
        Self {
            ownership_checker: OwnershipChecker::new(),
            lowerer: ASTLowerer::new(),
            cfg,
        }
    }

    #[inline]
    fn cfg(&self) -> &ErgConfig {
        &self.cfg
    }

    #[inline]
    fn finish(&mut self) {}

    fn clear(&mut self) {}

    fn exec(&mut self) -> Result<(), Self::Errs> {
        let mut builder = ASTBuilder::new(self.cfg.copy());
        let ast = builder.build()?;
        let hir = self.check(ast, "exec")?;
        println!("{hir}");
        Ok(())
    }

    fn eval(&mut self, src: String) -> Result<String, TyCheckErrors> {
        let mut builder = ASTBuilder::new(self.cfg.copy());
        let ast = builder.build_with_input(src)?;
        let hir = self.check(ast, "eval")?;
        Ok(hir.to_string())
    }
}

impl Checker {
    pub fn check(&mut self, ast: AST, mode: &str) -> Result<HIR, TyCheckErrors> {
        let (hir, warns) = self.lowerer.lower(ast, mode)?;
        if self.cfg.verbose >= 2 {
            warns.fmt_all_stderr();
        }
        let effect_checker = SideEffectChecker::new();
        let hir = effect_checker.check(hir)?;
        let hir = self.ownership_checker.check(hir)?;
        Ok(hir)
    }
}
