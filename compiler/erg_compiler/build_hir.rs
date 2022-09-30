use erg_common::config::ErgConfig;
use erg_common::error::MultiErrorDisplay;
use erg_common::traits::Runnable;
use erg_common::Str;

use erg_parser::ast::AST;
use erg_parser::build_ast::ASTBuilder;

use crate::context::Context;
use crate::effectcheck::SideEffectChecker;
use crate::error::{CompileError, CompileErrors};
use crate::hir::HIR;
use crate::lower::ASTLowerer;
use crate::mod_cache::SharedModuleCache;
use crate::ownercheck::OwnershipChecker;

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
        HIRBuilder::new_with_cache(
            cfg,
            Str::ever("<module>"),
            SharedModuleCache::new(),
            SharedModuleCache::new(),
        )
    }

    #[inline]
    fn cfg(&self) -> &ErgConfig {
        self.lowerer.cfg()
    }

    #[inline]
    fn finish(&mut self) {}

    fn clear(&mut self) {}

    fn exec(&mut self) -> Result<i32, Self::Errs> {
        let mut builder = ASTBuilder::new(self.cfg().copy());
        let ast = builder.build(self.input().read())?;
        let hir = self.check(ast, "exec").map_err(|(_, errs)| errs)?;
        println!("{hir}");
        Ok(0)
    }

    fn eval(&mut self, src: String) -> Result<String, Self::Errs> {
        let mut builder = ASTBuilder::new(self.cfg().copy());
        let ast = builder.build(src)?;
        let hir = self.check(ast, "eval").map_err(|(_, errs)| errs)?;
        Ok(hir.to_string())
    }
}

impl HIRBuilder {
    pub fn new_with_cache<S: Into<Str>>(
        cfg: ErgConfig,
        mod_name: S,
        mod_cache: SharedModuleCache,
        py_mod_cache: SharedModuleCache,
    ) -> Self {
        Self {
            lowerer: ASTLowerer::new_with_cache(cfg.copy(), mod_name, mod_cache, py_mod_cache),
            ownership_checker: OwnershipChecker::new(cfg),
        }
    }

    pub fn check(&mut self, ast: AST, mode: &str) -> Result<HIR, (Option<HIR>, CompileErrors)> {
        let (hir, warns) = self.lowerer.lower(ast, mode)?;
        if self.cfg().verbose >= 2 {
            warns.fmt_all_stderr();
        }
        let effect_checker = SideEffectChecker::new(self.cfg().clone());
        let hir = effect_checker
            .check(hir)
            .map_err(|(hir, errs)| (Some(hir), errs))?;
        let hir = self
            .ownership_checker
            .check(hir)
            .map_err(|(hir, errs)| (Some(hir), errs))?;
        Ok(hir)
    }

    pub fn build(&mut self, src: String, mode: &str) -> Result<HIR, (Option<HIR>, CompileErrors)> {
        let mut ast_builder = ASTBuilder::new(self.cfg().copy());
        let ast = ast_builder
            .build(src)
            .map_err(|errs| (None, CompileErrors::from(errs)))?;
        let hir = self.check(ast, mode)?;
        Ok(hir)
    }

    pub fn pop_ctx(&mut self) -> Context {
        self.lowerer.ctx.pop()
    }
}
