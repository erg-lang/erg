//! defines `Compiler`.
//!
//! コンパイラーを定義する
use std::path::Path;

use erg_common::Str;
use erg_common::{log};
use erg_common::codeobj::{CodeObj, CodeObjFlags};
use erg_common::color::{GREEN, RESET};
use erg_common::config::{Input, ErgConfig, SEMVER, BUILD_INFO};
use erg_common::error::MultiErrorDisplay;
use erg_common::traits::{Runnable, Stream};

use erg_parser::ParserRunner;

use crate::codegen::CodeGenerator;
use crate::effectcheck::SideEffectChecker;
use crate::error::{TyCheckErrors, CompileError, CompileErrors};
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
    pub const fn new(kind: StoreLoadKind, idx: usize) -> Self { Self{ kind, idx } }

    pub const fn local(idx: usize) -> Self { Self{ kind: StoreLoadKind::Local, idx } }
    pub const fn global(idx: usize) -> Self { Self{ kind: StoreLoadKind::Global, idx } }
    pub const fn deref(idx: usize) -> Self { Self{ kind: StoreLoadKind::Deref, idx } }
    pub const fn fast(idx: usize) -> Self { Self{ kind: StoreLoadKind::Fast, idx } }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessKind {
    Name,
    Attr,
    Method,
}

impl AccessKind {
    pub const fn is_local(&self) -> bool { matches!(self, Self::Name) }
    pub const fn is_attr(&self) -> bool { matches!(self, Self::Attr) }
    pub const fn is_method(&self) -> bool { matches!(self, Self::Method) }
}

/// Generates a `CodeObj` from an `AST`.
/// The input AST is not typed, so it's typed by `ASTLowerer` according to the cfg.opt_level.
#[derive(Debug)]
pub struct Compiler {
    cfg: ErgConfig,
    lowerer: ASTLowerer,
    code_generator: CodeGenerator,
}

impl Runnable for Compiler {
    type Err = CompileError;
    type Errs = CompileErrors;

    fn new(cfg: ErgConfig) -> Self {
        Self {
            code_generator: CodeGenerator::new(cfg.copy()),
            lowerer: ASTLowerer::new(),
            cfg,
        }
    }

    #[inline]
    fn input(&self) -> &Input { &self.cfg.input }

    #[inline]
    fn start_message(&self) -> String { format!("Erg compiler {} {}\n", SEMVER, &*BUILD_INFO) }

    #[inline]
    fn finish(&mut self) {}

    fn clear(&mut self) {
        self.code_generator.clear();
    }

    fn eval(&mut self, src: Str) -> Result<String, CompileErrors> {
        let codeobj = self.compile(src, "eval")?;
        Ok(codeobj.code_info())
    }
}

impl Compiler {
    fn convert(&self, errs: TyCheckErrors) -> CompileErrors {
        errs.into_iter().map(|e| CompileError::new(e.core, self.input().clone(), e.caused_by)).collect::<Vec<_>>().into()
    }

    pub fn compile_and_dump_as_pyc<P: AsRef<Path>>(&mut self, src: Str, path: P) -> Result<(), CompileErrors> {
        let code = self.compile(src, "exec")?;
        code.dump_as_pyc(path, self.cfg.python_ver).expect("failed to dump a .pyc file");
        Ok(())
    }

    pub fn compile(&mut self, src: Str, mode: &str) -> Result<CodeObj, CompileErrors> {
        log!("{GREEN}[DEBUG] the compiling process has started.{RESET}");
        let mut dynamic = true;
        let mut parser = ParserRunner::new(self.cfg.copy());
        let ast = parser.parse_from_str(src)?;
        if ast.is_empty() {
            return Ok(CodeObj::empty(vec![], Str::rc(self.input().enclosed_name()), "<module>", 1))
        }
        let (hir, warns) = self.lowerer.lower(ast, mode).map_err(|errs| self.convert(errs))?;
        if warns.is_empty() { dynamic = false; }
        if self.cfg.verbose >= 2 {
            let warns = self.convert(warns);
            warns.fmt_all_stderr();
        }
        let effect_checker = SideEffectChecker::new();
        let hir = effect_checker.check(hir).map_err(|errs| self.convert(errs))?;
        let ownership_checker = OwnershipChecker::new();
        let hir = ownership_checker.check(hir).map_err(|errs| self.convert(errs))?;
        let mut codeobj = self.code_generator.codegen(hir);
        if dynamic {
            codeobj.flags += CodeObjFlags::EvmDynamic as u32;
        }
        log!("{GREEN}code object:\n{}", codeobj.code_info());
        log!("[DEBUG] the compiling process has completed, found errors: {}{RESET}", self.code_generator.errs.len());
        if self.code_generator.errs.is_empty() {
            Ok(codeobj)
        } else {
            Err(self.code_generator.errs.flush())
        }
    }
}
