//! defines `Compiler`.
//!
//! コンパイラーを定義する
use std::path::Path;

use erg_common::config::ErgConfig;
use erg_common::log;
use erg_common::traits::{Runnable, Stream};

use crate::ty::codeobj::CodeObj;

use crate::build_hir::HIRBuilder;
use crate::codegen::CodeGenerator;
use crate::desugar_hir::HIRDesugarer;
use crate::error::{CompileError, CompileErrors};
use crate::hir::{Expr, HIR};
use crate::link::Linker;
use crate::mod_cache::SharedModuleCache;

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
    code_generator: CodeGenerator,
}

impl Runnable for Compiler {
    type Err = CompileError;
    type Errs = CompileErrors;
    const NAME: &'static str = "Erg compiler";

    fn new(cfg: ErgConfig) -> Self {
        let mod_cache = SharedModuleCache::new();
        let py_mod_cache = SharedModuleCache::new();
        Self {
            builder: HIRBuilder::new_with_cache(
                cfg.copy(),
                "<module>",
                mod_cache.clone(),
                py_mod_cache,
            ),
            code_generator: CodeGenerator::new(cfg.copy()),
            mod_cache,
            cfg,
        }
    }

    #[inline]
    fn cfg(&self) -> &ErgConfig {
        &self.cfg
    }

    #[inline]
    fn finish(&mut self) {}

    fn clear(&mut self) {
        self.code_generator.clear();
    }

    fn exec(&mut self) -> Result<i32, Self::Errs> {
        let path = self.input().filename().replace(".er", ".pyc");
        self.compile_and_dump_as_pyc(path, self.input().read(), "exec")?;
        Ok(0)
    }

    fn eval(&mut self, src: String) -> Result<String, CompileErrors> {
        let codeobj = self.compile(src, "eval")?;
        Ok(codeobj.code_info(Some(self.code_generator.py_version)))
    }
}

impl Compiler {
    pub fn compile_and_dump_as_pyc<P: AsRef<Path>>(
        &mut self,
        pyc_path: P,
        src: String,
        mode: &str,
    ) -> Result<(), CompileErrors> {
        let code = self.compile(src, mode)?;
        code.dump_as_pyc(pyc_path, self.cfg.py_magic_num)
            .expect("failed to dump a .pyc file (maybe permission denied)");
        Ok(())
    }

    pub fn eval_compile_and_dump_as_pyc<P: AsRef<Path>>(
        &mut self,
        pyc_path: P,
        src: String,
        mode: &str,
    ) -> Result<Option<Expr>, CompileErrors> {
        let (code, last) = self.eval_compile(src, mode)?;
        code.dump_as_pyc(pyc_path, self.cfg.py_magic_num)
            .expect("failed to dump a .pyc file (maybe permission denied)");
        Ok(last)
    }

    pub fn compile(&mut self, src: String, mode: &str) -> Result<CodeObj, CompileErrors> {
        log!(info "the compiling process has started.");
        let hir = self.build_link_desugar(src, mode)?;
        let codeobj = self.code_generator.emit(hir);
        log!(info "code object:\n{}", codeobj.code_info(Some(self.code_generator.py_version)));
        log!(info "the compiling process has completed");
        Ok(codeobj)
    }

    pub fn eval_compile(
        &mut self,
        src: String,
        mode: &str,
    ) -> Result<(CodeObj, Option<Expr>), CompileErrors> {
        log!(info "the compiling process has started.");
        let hir = self.build_link_desugar(src, mode)?;
        let last = hir.module.last().cloned();
        let codeobj = self.code_generator.emit(hir);
        log!(info "code object:\n{}", codeobj.code_info(Some(self.code_generator.py_version)));
        log!(info "the compiling process has completed");
        Ok((codeobj, last))
    }

    fn build_link_desugar(&mut self, src: String, mode: &str) -> Result<HIR, CompileErrors> {
        let artifact = self
            .builder
            .build(src, mode)
            .map_err(|artifact| artifact.errors)?;
        let linker = Linker::new(&self.cfg, &self.mod_cache);
        let hir = linker.link(artifact.hir);
        Ok(HIRDesugarer::desugar(hir))
    }
}
