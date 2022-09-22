use erg_common::config::ErgConfig;
use erg_common::error::MultiErrorDisplay;
use erg_common::traits::{Runnable, Stream};
use erg_common::Str;

use erg_parser::ast::AST;
use erg_parser::build_ast::ASTBuilder;

use crate::context::Context;
use crate::effectcheck::SideEffectChecker;
use crate::error::{CompileError, CompileErrors, TyCheckErrors};
use crate::hir::HIR;
use crate::lower::ASTLowerer;
use crate::mod_cache::SharedModuleCache;
use crate::ownercheck::OwnershipChecker;
use crate::reorder::Reorderer;

/// Summarize lowering, side-effect checking, and ownership checking
#[derive(Debug)]
pub struct HIRBuilder {
    lowerer: ASTLowerer,
    ownership_checker: OwnershipChecker,
}

impl Runnable for HIRBuilder {
    type Err = CompileError;
    type Errs = CompileErrors;
    const NAME: &'static str = "Erg HIR builder";

    fn new(cfg: ErgConfig) -> Self {
        HIRBuilder::new_with_cache(cfg, Str::ever("<module>"), SharedModuleCache::new())
    }

    #[inline]
    fn cfg(&self) -> &ErgConfig {
        self.lowerer.cfg()
    }

    #[inline]
    fn finish(&mut self) {}

    fn clear(&mut self) {}

    fn exec(&mut self) -> Result<(), Self::Errs> {
        let mut builder = ASTBuilder::new(self.cfg().copy());
        let ast = builder.build(self.input().read())?;
        let hir = self.check(ast, "exec")?;
        println!("{hir}");
        Ok(())
    }

    fn eval(&mut self, src: String) -> Result<String, Self::Errs> {
        let mut builder = ASTBuilder::new(self.cfg().copy());
        let ast = builder.build(src)?;
        let hir = self.check(ast, "eval")?;
        Ok(hir.to_string())
    }
}

impl HIRBuilder {
    pub fn new_with_cache<S: Into<Str>>(
        cfg: ErgConfig,
        mod_name: S,
        mod_cache: SharedModuleCache,
    ) -> Self {
        Self {
            lowerer: ASTLowerer::new_with_cache(cfg, mod_name, mod_cache),
            ownership_checker: OwnershipChecker::new(),
        }
    }

    fn convert(&self, errs: TyCheckErrors) -> CompileErrors {
        errs.into_iter()
            .map(|e| CompileError::new(e.core, self.input().clone(), e.caused_by))
            .collect::<Vec<_>>()
            .into()
    }

    pub fn check(&mut self, ast: AST, mode: &str) -> Result<HIR, CompileErrors> {
        let (hir, warns) = self
            .lowerer
            .lower(ast, mode)
            .map_err(|errs| self.convert(errs))?;
        if self.cfg().verbose >= 2 {
            let warns = self.convert(warns);
            warns.fmt_all_stderr();
        }
        let effect_checker = SideEffectChecker::new();
        let hir = effect_checker
            .check(hir)
            .map_err(|errs| self.convert(errs))?;
        let hir = self
            .ownership_checker
            .check(hir)
            .map_err(|errs| self.convert(errs))?;
        Ok(hir)
    }

    pub fn build(&mut self, src: String, mode: &str) -> Result<HIR, CompileErrors> {
        let mut ast_builder = ASTBuilder::new(self.cfg().copy());
        let ast = ast_builder.build(src)?;
        let ast = Reorderer::new()
            .reorder(ast)
            .map_err(|errs| self.convert(errs))?;
        let hir = self.check(ast, mode)?;
        Ok(hir)
    }

    pub fn pop_ctx(&mut self) -> Context {
        self.lowerer.ctx.pop()
    }
}
