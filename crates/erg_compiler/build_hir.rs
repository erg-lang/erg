use erg_common::config::ErgConfig;
use erg_common::dict::Dict;
use erg_common::error::MultiErrorDisplay;
use erg_common::traits::{ExitStatus, Runnable, Stream};
use erg_common::Str;

use erg_parser::ast::{VarName, AST};
use erg_parser::build_ast::ASTBuilder;

use crate::artifact::{BuildRunnable, Buildable, CompleteArtifact, IncompleteArtifact};
use crate::context::{Context, ContextKind, ContextProvider, ModuleContext};
use crate::effectcheck::SideEffectChecker;
use crate::error::{CompileError, CompileErrors, LowerWarnings};
use crate::link_hir::HIRLinker;
use crate::lower::ASTLowerer;
use crate::module::SharedCompilerResource;
use crate::ownercheck::OwnershipChecker;
use crate::ty::VisibilityModifier;
use crate::varinfo::VarInfo;

/// Summarize lowering, side-effect checking, and ownership checking
///
/// NOTE: This does not perform dependency resolution, use `PackageBuilder` to build a package
#[derive(Debug)]
pub struct HIRBuilder {
    pub(crate) lowerer: ASTLowerer,
    ownership_checker: OwnershipChecker,
}

impl Default for HIRBuilder {
    fn default() -> Self {
        HIRBuilder::new(ErgConfig::default())
    }
}

impl Runnable for HIRBuilder {
    type Err = CompileError;
    type Errs = CompileErrors;
    const NAME: &'static str = "Erg HIR builder";

    fn new(cfg: ErgConfig) -> Self {
        HIRBuilder::new_with_cache(
            cfg.copy(),
            Str::ever("<module>"),
            SharedCompilerResource::new(cfg),
        )
    }

    #[inline]
    fn cfg(&self) -> &ErgConfig {
        self.lowerer.cfg()
    }
    #[inline]
    fn cfg_mut(&mut self) -> &mut ErgConfig {
        self.lowerer.cfg_mut()
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

    fn exec(&mut self) -> Result<ExitStatus, Self::Errs> {
        let mut builder = ASTBuilder::new(self.cfg().copy());
        let artifact = builder
            .build(self.cfg_mut().input.read())
            .map_err(|arti| arti.errors)?;
        artifact.warns.write_all_stderr();
        let artifact = self
            .check(artifact.ast, "exec")
            .map_err(|arti| arti.errors)?;
        artifact.warns.write_all_stderr();
        println!("{}", artifact.object);
        Ok(ExitStatus::compile_passed(artifact.warns.len()))
    }

    fn eval(&mut self, src: String) -> Result<String, Self::Errs> {
        let mut builder = ASTBuilder::new(self.cfg().copy());
        let artifact = builder.build(src).map_err(|arti| arti.errors)?;
        artifact.warns.write_all_stderr();
        let artifact = self
            .check(artifact.ast, "eval")
            .map_err(|arti| arti.errors)?;
        artifact.warns.write_all_stderr();
        Ok(artifact.object.to_string())
    }
}

impl Buildable for HIRBuilder {
    fn inherit(cfg: ErgConfig, shared: SharedCompilerResource) -> Self {
        let mod_name = Str::from(cfg.input.file_stem());
        Self::new_with_cache(cfg, mod_name, shared)
    }
    fn build(&mut self, src: String, mode: &str) -> Result<CompleteArtifact, IncompleteArtifact> {
        self.build(src, mode)
    }
    fn pop_context(&mut self) -> Option<ModuleContext> {
        self.pop_mod_ctx()
    }
    fn get_context(&self) -> Option<&ModuleContext> {
        Some(&self.lowerer.module)
    }
}

impl BuildRunnable for HIRBuilder {}

impl ContextProvider for HIRBuilder {
    fn dir(&self) -> Dict<&VarName, &VarInfo> {
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
        shared: SharedCompilerResource,
    ) -> Self {
        Self {
            lowerer: ASTLowerer::new_with_cache(cfg.copy(), mod_name, shared),
            ownership_checker: OwnershipChecker::new(cfg),
        }
    }

    pub fn new_with_ctx(mut mod_ctx: ModuleContext) -> Self {
        mod_ctx.context.grow(
            "<module>",
            ContextKind::Module,
            VisibilityModifier::Private,
            None,
        );
        Self {
            ownership_checker: OwnershipChecker::new(mod_ctx.get_top_cfg()),
            lowerer: ASTLowerer::new_with_ctx(mod_ctx),
        }
    }

    pub fn check(&mut self, ast: AST, mode: &str) -> Result<CompleteArtifact, IncompleteArtifact> {
        let mut artifact = self.lowerer.lower(ast, mode)?;
        let effect_checker = SideEffectChecker::new(self.cfg().clone());
        let hir = effect_checker
            .check(artifact.object)
            .map_err(|(hir, errs)| {
                self.lowerer.module.context.clear_invalid_vars();
                IncompleteArtifact::new(Some(hir), errs, artifact.warns.take_all().into())
            })?;
        let hir = self.ownership_checker.check(hir).map_err(|(hir, errs)| {
            self.lowerer.module.context.clear_invalid_vars();
            IncompleteArtifact::new(Some(hir), errs, artifact.warns.take_all().into())
        })?;
        Ok(CompleteArtifact::new(hir, artifact.warns))
    }

    pub fn build(
        &mut self,
        src: String,
        mode: &str,
    ) -> Result<CompleteArtifact, IncompleteArtifact> {
        let mut ast_builder = ASTBuilder::new(self.cfg().copy());
        let artifact = ast_builder
            .build(src)
            .map_err(|iart| IncompleteArtifact::new(None, iart.errors.into(), iart.warns.into()))?;
        self.lowerer
            .warns
            .extend(LowerWarnings::from(artifact.warns));
        self.check(artifact.ast, mode)
    }

    pub fn build_module(&mut self) -> Result<CompleteArtifact, IncompleteArtifact> {
        let src = self.cfg_mut().input.read();
        self.build(src, "exec")
    }

    pub fn build_linked_module(&mut self) -> Result<CompleteArtifact, IncompleteArtifact> {
        let artifact = self.build_module()?;
        let linker = HIRLinker::new(self.cfg(), self.lowerer.module.context.mod_cache());
        let hir = linker.link(artifact.object);
        Ok(CompleteArtifact::new(hir, artifact.warns))
    }

    pub fn pop_mod_ctx(&mut self) -> Option<ModuleContext> {
        self.lowerer.pop_mod_ctx()
    }

    pub fn dir(&mut self) -> Dict<&VarName, &VarInfo> {
        ContextProvider::dir(self)
    }

    pub fn get_receiver_ctx(&self, receiver_name: &str) -> Option<&Context> {
        ContextProvider::get_receiver_ctx(self, receiver_name)
    }

    pub fn get_var_info(&self, name: &str) -> Option<(&VarName, &VarInfo)> {
        ContextProvider::get_var_info(self, name)
    }

    pub fn current_ctx(&self) -> &Context {
        &self.lowerer.module.context
    }
}
