//! defines `Compiler`.
//!
//! コンパイラーを定義する
use std::path::Path;

use erg_common::config::ErgConfig;
use erg_common::dict::Dict;
use erg_common::error::MultiErrorDisplay;
use erg_common::log;
use erg_common::traits::{ExitStatus, New, Runnable, Stream};
use erg_parser::ast::{VarName, AST};

use crate::artifact::{Buildable, CompleteArtifact, ErrorArtifact};
use crate::build_package::PackageBuilder;
use crate::codegen::PyCodeGenerator;
use crate::context::{Context, ContextProvider};
use crate::desugar_hir::HIRDesugarer;
use crate::error::{CompileError, CompileErrors, CompileWarnings};
use crate::hir::Expr;
use crate::link_hir::HIRLinker;
use crate::module::SharedCompilerResource;
use crate::optimize::HIROptimizer;
use crate::ty::codeobj::CodeObj;
use crate::varinfo::VarInfo;

/// * registered as global -> Global
/// * defined in the toplevel scope (and called in the inner scope) -> Global
/// * defined and called in the toplevel scope -> Local
/// * not defined in the toplevel and called in the inner scope -> Deref
/// * defined and called in the current scope (except the toplevel) -> Fast
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreLoadKind {
    Local,
    LocalConst,
    Global,
    GlobalConst,
    Deref,
    DerefConst,
    Fast,
    FastConst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Name {
    pub kind: StoreLoadKind,
    pub idx: usize,
}

impl Name {
    pub const fn new(kind: StoreLoadKind, idx: usize) -> Self {
        Self { kind, idx }
    }

    pub const fn local(idx: usize) -> Self {
        Self {
            kind: StoreLoadKind::Local,
            idx,
        }
    }
    pub const fn global(idx: usize) -> Self {
        Self {
            kind: StoreLoadKind::Global,
            idx,
        }
    }
    pub const fn deref(idx: usize) -> Self {
        Self {
            kind: StoreLoadKind::Deref,
            idx,
        }
    }
    pub const fn fast(idx: usize) -> Self {
        Self {
            kind: StoreLoadKind::Fast,
            idx,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessKind {
    Name,
    /// class/module attr
    /// e.g. `Str.center`
    UnboundAttr,
    /// method/instance attr
    /// e.g. `"aaa".center`
    ///
    /// can also access class/module attrs
    BoundAttr,
}

impl AccessKind {
    pub const fn is_local(&self) -> bool {
        matches!(self, Self::Name)
    }
    pub const fn is_unbound_attr(&self) -> bool {
        matches!(self, Self::UnboundAttr)
    }
    pub const fn is_bound_attr(&self) -> bool {
        matches!(self, Self::BoundAttr)
    }
    pub fn matches(&self, vi: &VarInfo) -> bool {
        match self {
            Self::Name | Self::BoundAttr => true,
            Self::UnboundAttr => !vi.kind.is_instance_attr(),
        }
    }
}

/// Generates a `CodeObj` from an String or other File inputs.
#[derive(Debug)]
pub struct Compiler {
    pub cfg: ErgConfig,
    builder: PackageBuilder,
    shared: SharedCompilerResource,
    code_generator: PyCodeGenerator,
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new(ErgConfig::default())
    }
}

impl New for Compiler {
    fn new(cfg: ErgConfig) -> Self {
        let shared = SharedCompilerResource::new(cfg.copy());
        Self {
            shared: shared.clone(),
            builder: PackageBuilder::new(cfg.copy(), shared),
            code_generator: PyCodeGenerator::new(cfg.copy()),
            cfg,
        }
    }
}

impl Runnable for Compiler {
    type Err = CompileError;
    type Errs = CompileErrors;
    const NAME: &'static str = "Erg compiler";

    #[inline]
    fn cfg(&self) -> &ErgConfig {
        &self.cfg
    }
    #[inline]
    fn cfg_mut(&mut self) -> &mut ErgConfig {
        &mut self.cfg
    }

    #[inline]
    fn finish(&mut self) {}

    fn initialize(&mut self) {
        self.builder.initialize();
        self.code_generator.clear();
        // .mod_cache will be initialized in .builder
    }

    fn clear(&mut self) {
        self.builder.clear();
        self.code_generator.clear();
    }

    fn set_input(&mut self, input: erg_common::io::Input) {
        self.cfg.input = input;
        self.builder.set_input(self.cfg.input.clone());
        self.code_generator.set_input(self.cfg.input.clone());
    }

    fn exec(&mut self) -> Result<ExitStatus, Self::Errs> {
        let path = self.cfg.dump_pyc_path();
        let src = self.cfg.input.read();
        let warns = self
            .compile_and_dump_as_pyc(path, src, "exec")
            .map_err(|eart| {
                eart.warns.write_all_stderr();
                eart.errors
            })?;
        warns.write_all_stderr();
        Ok(ExitStatus::compile_passed(warns.len()))
    }

    fn eval(&mut self, src: String) -> Result<String, CompileErrors> {
        let arti = self.compile(src, "eval").map_err(|eart| {
            eart.warns.write_all_stderr();
            eart.errors
        })?;
        arti.warns.write_all_stderr();
        Ok(arti.object.code_info(Some(self.code_generator.py_version)))
    }
}

impl ContextProvider for Compiler {
    fn dir(&self) -> Dict<&VarName, &VarInfo> {
        self.builder.dir()
    }

    fn get_receiver_ctx(&self, receiver_name: &str) -> Option<&Context> {
        self.builder.get_receiver_ctx(receiver_name)
    }

    fn get_var_info(&self, name: &str) -> Option<(&VarName, &VarInfo)> {
        self.builder.get_var_info(name)
    }
}

impl Compiler {
    pub fn new(cfg: ErgConfig) -> Self {
        New::new(cfg)
    }

    pub fn compile_and_dump_as_pyc<P: AsRef<Path>>(
        &mut self,
        pyc_path: P,
        src: String,
        mode: &str,
    ) -> Result<CompileWarnings, ErrorArtifact> {
        let arti = self.compile(src, mode)?;
        arti.object
            .dump_as_pyc(pyc_path, self.cfg.py_magic_num)
            .expect("failed to dump a .pyc file (maybe permission denied)");
        Ok(arti.warns)
    }

    pub fn eval_compile_and_dump_as_pyc<P: AsRef<Path>>(
        &mut self,
        pyc_path: P,
        src: String,
        mode: &str,
    ) -> Result<CompleteArtifact<Option<Expr>>, ErrorArtifact> {
        let arti = self.eval_compile(src, mode)?;
        let (code, last) = arti.object;
        code.dump_as_pyc(pyc_path, self.cfg.py_magic_num)
            .expect("failed to dump a .pyc file (maybe permission denied)");
        Ok(CompleteArtifact::new(last, arti.warns))
    }

    pub fn compile(
        &mut self,
        src: String,
        mode: &str,
    ) -> Result<CompleteArtifact<CodeObj>, ErrorArtifact> {
        log!(info "the compiling process has started.");
        let arti = self.build_link_desugar_optimize(src, mode)?;
        let codeobj = self.code_generator.emit(arti.object);
        log!(info "code object:\n{}", codeobj.code_info(Some(self.code_generator.py_version)));
        log!(info "the compiling process has completed");
        Ok(CompleteArtifact::new(codeobj, arti.warns))
    }

    pub fn compile_ast(
        &mut self,
        ast: AST,
        mode: &str,
    ) -> Result<CompleteArtifact<CodeObj>, ErrorArtifact> {
        log!(info "the compiling process has started.");
        let arti = self.build_link_desugar_optimize_ast(ast, mode)?;
        let codeobj = self.code_generator.emit(arti.object);
        log!(info "code object:\n{}", codeobj.code_info(Some(self.code_generator.py_version)));
        log!(info "the compiling process has completed");
        Ok(CompleteArtifact::new(codeobj, arti.warns))
    }

    pub fn compile_module(&mut self) -> Result<CompleteArtifact<CodeObj>, ErrorArtifact> {
        let src = self.cfg.input.read();
        self.compile(src, "exec")
    }

    pub fn eval_compile(
        &mut self,
        src: String,
        mode: &str,
    ) -> Result<CompleteArtifact<(CodeObj, Option<Expr>)>, ErrorArtifact> {
        log!(info "the compiling process has started.");
        let arti = self.build_link_desugar_optimize(src, mode)?;
        let last = arti.object.module.last().cloned();
        let codeobj = self.code_generator.emit(arti.object);
        log!(info "code object:\n{}", codeobj.code_info(Some(self.code_generator.py_version)));
        log!(info "the compiling process has completed");
        Ok(CompleteArtifact::new((codeobj, last), arti.warns))
    }

    fn build_link_desugar_optimize(
        &mut self,
        src: String,
        mode: &str,
    ) -> Result<CompleteArtifact, ErrorArtifact> {
        let artifact = self.builder.build(src, mode)?;
        let linker = HIRLinker::new(&self.cfg, &self.shared.mod_cache);
        let hir = linker.link(artifact.object);
        let hir = HIRDesugarer::desugar(hir);
        let hir = HIROptimizer::optimize(self.cfg.clone(), self.shared.clone(), hir);
        Ok(CompleteArtifact::new(hir, artifact.warns))
    }

    fn build_link_desugar_optimize_ast(
        &mut self,
        ast: AST,
        mode: &str,
    ) -> Result<CompleteArtifact, ErrorArtifact> {
        let artifact = self.builder.build_from_ast(ast, mode)?;
        let linker = HIRLinker::new(&self.cfg, &self.shared.mod_cache);
        let hir = linker.link(artifact.object);
        let hir = HIRDesugarer::desugar(hir);
        let hir = HIROptimizer::optimize(self.cfg.clone(), self.shared.clone(), hir);
        Ok(CompleteArtifact::new(hir, artifact.warns))
    }

    pub fn initialize_generator(&mut self) {
        self.code_generator.initialize();
    }
}
