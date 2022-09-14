//! defines `Compiler`.
//!
//! コンパイラーを定義する
use std::path::Path;

use erg_common::config::{ErgConfig, Input};
use erg_common::error::MultiErrorDisplay;
use erg_common::log;
use erg_common::traits::{Runnable, Stream};
use erg_type::codeobj::CodeObj;

use erg_parser::ParserRunner;

use crate::codegen::CodeGenerator;
use crate::effectcheck::SideEffectChecker;
use crate::error::{CompileError, CompileErrors, TyCheckErrors};
use crate::link::Linker;
use crate::lower::ASTLowerer;
use crate::ownercheck::OwnershipChecker;

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

/// Generates a `CodeObj` from an `AST`.
/// The input AST is not typed, so it's typed by `ASTLowerer` according to the cfg.opt_level.
#[derive(Debug)]
pub struct Compiler {
    cfg: ErgConfig,
    lowerer: ASTLowerer,
    ownership_checker: OwnershipChecker,
    code_generator: CodeGenerator,
}

impl Runnable for Compiler {
    type Err = CompileError;
    type Errs = CompileErrors;
    const NAME: &'static str = "Erg compiler";

    fn new(cfg: ErgConfig) -> Self {
        Self {
            code_generator: CodeGenerator::new(cfg.copy()),
            ownership_checker: OwnershipChecker::new(),
            lowerer: ASTLowerer::new(),
            cfg,
        }
    }

    #[inline]
    fn input(&self) -> &Input {
        &self.cfg.input
    }

    #[inline]
    fn finish(&mut self) {}

    fn clear(&mut self) {
        self.code_generator.clear();
    }

    fn exec(&mut self) -> Result<(), Self::Errs> {
        let path = self.input().filename().replace(".er", ".pyc");
        let src = self.input().read();
        self.compile_and_dump_as_pyc(src, path, "exec")
    }

    fn eval(&mut self, src: String) -> Result<String, CompileErrors> {
        let codeobj = self.compile(src, "eval")?;
        Ok(codeobj.code_info())
    }
}

impl Compiler {
    fn convert(&self, errs: TyCheckErrors) -> CompileErrors {
        errs.into_iter()
            .map(|e| CompileError::new(e.core, self.input().clone(), e.caused_by))
            .collect::<Vec<_>>()
            .into()
    }

    pub fn compile_and_dump_as_pyc<P: AsRef<Path>>(
        &mut self,
        src: String,
        path: P,
        mode: &str,
    ) -> Result<(), CompileErrors> {
        let code = self.compile(src, mode)?;
        code.dump_as_pyc(path, self.cfg.python_ver)
            .expect("failed to dump a .pyc file");
        Ok(())
    }

    pub fn compile(&mut self, src: String, mode: &str) -> Result<CodeObj, CompileErrors> {
        log!(info "the compiling process has started.");
        let mut cfg = self.cfg.copy();
        cfg.input = Input::Str(src);
        let mut parser = ParserRunner::new(cfg);
        let ast = parser.parse()?;
        let linker = Linker::new();
        let ast = linker.link(ast).map_err(|errs| self.convert(errs))?;
        let (hir, warns) = self
            .lowerer
            .lower(ast, mode)
            .map_err(|errs| self.convert(errs))?;
        if self.cfg.verbose >= 2 {
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
        let codeobj = self.code_generator.codegen(hir);
        log!(info "code object:\n{}", codeobj.code_info());
        log!(
            info "the compiling process has completed, found errors: {}",
            self.code_generator.errs.len()
        );
        if self.code_generator.errs.is_empty() {
            Ok(codeobj)
        } else {
            Err(self.code_generator.errs.flush())
        }
    }
}
