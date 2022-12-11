use erg_common::config::ErgConfig;
use erg_common::error::MultiErrorDisplay;
use erg_common::traits::Runnable;
use erg_common::Str;

use erg_parser::ast::{VarName, AST};
use erg_parser::build_ast::ASTBuilder;

use crate::artifact::{BuildRunnable, Buildable, CompleteArtifact, IncompleteArtifact};
use crate::context::{Context, ContextProvider};
use crate::effectcheck::SideEffectChecker;
use crate::error::{CompileError, CompileErrors};
use crate::lower::ASTLowerer;
use crate::mod_cache::SharedModuleCache;
use crate::ownercheck::OwnershipChecker;
use crate::varinfo::VarInfo;

/// Summarize lowering, side-effect checking, and ownership checking
#[derive(Debug, Default)]
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

    fn initialize(&mut self) {
        self.lowerer.initialize();
        self.ownership_checker = OwnershipChecker::new(self.cfg().copy());
    }

    fn clear(&mut self) {
        self.lowerer.clear();
        // don't initialize the ownership checker
    }

    fn exec(&mut self) -> Result<i32, Self::Errs> {
        let mut builder = ASTBuilder::new(self.cfg().copy());
        let ast = builder.build(self.input().read())?;
        let artifact = self.check(ast, "exec").map_err(|arti| arti.errors)?;
        artifact.warns.fmt_all_stderr();
        println!("{}", artifact.object);
        Ok(0)
    }

    fn eval(&mut self, src: String) -> Result<String, Self::Errs> {
        let mut builder = ASTBuilder::new(self.cfg().copy());
        let ast = builder.build(src)?;
        let artifact = self.check(ast, "eval").map_err(|arti| arti.errors)?;
        artifact.warns.fmt_all_stderr();
        Ok(artifact.object.to_string())
    }
}

impl Buildable for HIRBuilder {
    fn build(&mut self, src: String, mode: &str) -> Result<CompleteArtifact, IncompleteArtifact> {
        self.build(src, mode)
    }
    fn pop_context(&mut self) -> Option<Context> {
        Some(self.pop_mod_ctx())
    }
    fn get_context(&self) -> Option<&Context> {
        Some(&self.lowerer.ctx)
    }
}

impl BuildRunnable for HIRBuilder {}

impl ContextProvider for HIRBuilder {
    fn dir(&self) -> Vec<(&VarName, &VarInfo)> {
        self.lowerer.dir()
    }

    fn get_receiver_ctx(&self, receiver_name: &str) -> Option<&Context> {
        self.lowerer.get_receiver_ctx(receiver_name)
    }

    fn get_var_info(&self, name: &str) -> Option<(&VarName, &VarInfo)> {
        self.lowerer.get_var_info(name)
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

    pub fn check(&mut self, ast: AST, mode: &str) -> Result<CompleteArtifact, IncompleteArtifact> {
        let artifact = self.lowerer.lower(ast, mode)?;
        let effect_checker = SideEffectChecker::new(self.cfg().clone());
        let hir = effect_checker
            .check(artifact.object)
            .map_err(|(hir, errs)| {
                self.lowerer.ctx.clear_all_vars();
                IncompleteArtifact::new(Some(hir), errs, artifact.warns.clone())
            })?;
        let hir = self.ownership_checker.check(hir).map_err(|(hir, errs)| {
            self.lowerer.ctx.clear_all_vars();
            IncompleteArtifact::new(Some(hir), errs, artifact.warns.clone())
        })?;
        Ok(CompleteArtifact::new(hir, artifact.warns))
    }

    pub fn build(
        &mut self,
        src: String,
        mode: &str,
    ) -> Result<CompleteArtifact, IncompleteArtifact> {
        let mut ast_builder = ASTBuilder::new(self.cfg().copy());
        let ast = ast_builder.build(src).map_err(|errs| {
            IncompleteArtifact::new(None, CompileErrors::from(errs), CompileErrors::empty())
        })?;
        self.check(ast, mode)
    }

    pub fn pop_mod_ctx(&mut self) -> Context {
        self.lowerer.pop_mod_ctx()
    }

    pub fn dir(&mut self) -> Vec<(&VarName, &VarInfo)> {
        ContextProvider::dir(self)
    }

    pub fn get_receiver_ctx(&self, receiver_name: &str) -> Option<&Context> {
        ContextProvider::get_receiver_ctx(self, receiver_name)
    }

    pub fn get_var_info(&self, name: &str) -> Option<(&VarName, &VarInfo)> {
        ContextProvider::get_var_info(self, name)
    }
}
