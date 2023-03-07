//! defines `Compiler`.
//!
//! コンパイラーを定義する
use std::path::Path;

use erg_common::config::ErgConfig;
use erg_common::dict::Dict;
use erg_common::error::MultiErrorDisplay;
use erg_common::log;
use erg_common::traits::{Runnable, Stream};
use erg_parser::ast::VarName;

use crate::artifact::{CompleteArtifact, ErrorArtifact};
use crate::context::{Context, ContextProvider};
use crate::ty::codeobj::CodeObj;

use crate::build_hir::HIRBuilder;
use crate::codegen::PyCodeGenerator;
use crate::desugar_hir::HIRDesugarer;
use crate::error::{CompileError, CompileErrors, CompileWarnings};
use crate::hir::Expr;
use crate::link_hir::HIRLinker;
use crate::module::{SharedCompilerResource, SharedModuleCache};
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
    Attr,
    Method,
}

impl AccessKind {
    pub const fn is_local(&self) -> bool {
        matches!(self, Self::Name)
    }
    pub const fn is_attr(&self) -> bool {
        matches!(self, Self::Attr)
    }
    pub const fn is_method(&self) -> bool {
        matches!(self, Self::Method)
    }
}

/// Generates a `CodeObj` from an String or other File inputs.
#[derive(Debug)]
pub struct Compiler {
    pub cfg: ErgConfig,
    builder: HIRBuilder,
    mod_cache: SharedModuleCache,
    code_generator: PyCodeGenerator,
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new(ErgConfig::default())
    }
}

impl Runnable for Compiler {
    type Err = CompileError;
    type Errs = CompileErrors;
    const NAME: &'static str = "Erg compiler";

    fn new(cfg: ErgConfig) -> Self {
        let shared = SharedCompilerResource::new(cfg.copy());
        Self {
            mod_cache: shared.mod_cache.clone(),
            builder: HIRBuilder::new_with_cache(cfg.copy(), "<module>", shared),
            code_generator: PyCodeGenerator::new(cfg.copy()),
            cfg,
        }
    }

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

    fn exec(&mut self) -> Result<i32, Self::Errs> {
        let path = self.cfg.dump_pyc_path();
        let src = self.cfg.input.read();
        let warns = self
            .compile_and_dump_as_pyc(path, src, "exec")
            .map_err(|eart| {
                eart.warns.fmt_all_stderr();
                eart.errors
            })?;
        warns.fmt_all_stderr();
        Ok(0)
    }

    fn eval(&mut self, src: String) -> Result<String, CompileErrors> {
        let arti = self.compile(src, "eval").map_err(|eart| {
            eart.warns.fmt_all_stderr();
            eart.errors
        })?;
        arti.warns.fmt_all_stderr();
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
        let arti = self.build_link_desugar(src, mode)?;
        let codeobj = self.code_generator.emit(arti.object);
        log!(info "code object:\n{}", codeobj.code_info(Some(self.code_generator.py_version)));
        log!(info "the compiling process has completed");
        Ok(CompleteArtifact::new(codeobj, arti.warns))
    }

    pub fn eval_compile(
        &mut self,
        src: String,
        mode: &str,
    ) -> Result<CompleteArtifact<(CodeObj, Option<Expr>)>, ErrorArtifact> {
        log!(info "the compiling process has started.");
        let arti = self.build_link_desugar(src, mode)?;
        let last = arti.object.module.last().cloned();
        let codeobj = self.code_generator.emit(arti.object);
        log!(info "code object:\n{}", codeobj.code_info(Some(self.code_generator.py_version)));
        log!(info "the compiling process has completed");
        Ok(CompleteArtifact::new((codeobj, last), arti.warns))
    }

    fn build_link_desugar(
        &mut self,
        src: String,
        mode: &str,
    ) -> Result<CompleteArtifact, ErrorArtifact> {
        let artifact = self.builder.build(src, mode)?;
        let linker = HIRLinker::new(&self.cfg, &self.mod_cache);
        let hir = linker.link(artifact.object);
        let desugared = HIRDesugarer::desugar(hir);
        Ok(CompleteArtifact::new(desugared, artifact.warns))
    }
}
