use std::fmt;

use erg_common::config::ErgConfig;
use erg_common::traits::{Runnable, Stream};
use erg_common::Str;
use erg_parser::ast::AST;

use crate::context::ModuleContext;
use crate::error::CompileErrors;
use crate::hir::HIR;
use crate::module::SharedCompilerResource;

#[derive(Debug)]
pub struct CompleteArtifact<Inner = HIR> {
    pub object: Inner,
    pub warns: CompileErrors,
}

impl<Inner> CompleteArtifact<Inner> {
    pub const fn new(object: Inner, warns: CompileErrors) -> Self {
        Self { object, warns }
    }
}

#[derive(Debug)]
pub struct IncompleteArtifact<Inner = HIR> {
    pub object: Option<Inner>,
    pub errors: CompileErrors,
    pub warns: CompileErrors,
}

impl<I> fmt::Display for IncompleteArtifact<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.warns.is_empty() {
            writeln!(f, "{}", self.warns)?;
        }
        write!(f, "{}", self.errors)
    }
}

impl<I: fmt::Debug> std::error::Error for IncompleteArtifact<I> {}

impl<Inner> From<CompleteArtifact<Inner>> for IncompleteArtifact<Inner> {
    fn from(artifact: CompleteArtifact<Inner>) -> Self {
        Self {
            object: Some(artifact.object),
            errors: CompileErrors::empty(),
            warns: artifact.warns,
        }
    }
}

impl<Inner> IncompleteArtifact<Inner> {
    pub const fn new(object: Option<Inner>, errors: CompileErrors, warns: CompileErrors) -> Self {
        Self {
            object,
            errors,
            warns,
        }
    }
}

#[derive(Debug)]
pub struct ErrorArtifact {
    pub errors: CompileErrors,
    pub warns: CompileErrors,
}

impl fmt::Display for ErrorArtifact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.warns.is_empty() {
            writeln!(f, "{}", self.warns)?;
        }
        write!(f, "{}", self.errors)
    }
}

impl std::error::Error for ErrorArtifact {}

impl<Inner> From<IncompleteArtifact<Inner>> for ErrorArtifact {
    fn from(artifact: IncompleteArtifact<Inner>) -> Self {
        Self {
            errors: artifact.errors,
            warns: artifact.warns,
        }
    }
}

impl From<CompileErrors> for ErrorArtifact {
    fn from(errors: CompileErrors) -> Self {
        Self {
            errors,
            warns: CompileErrors::empty(),
        }
    }
}

impl ErrorArtifact {
    pub const fn new(errors: CompileErrors, warns: CompileErrors) -> Self {
        Self { errors, warns }
    }

    pub fn clear(&mut self) {
        self.errors.clear();
        self.warns.clear();
    }
}

pub trait Buildable<T = HIR> {
    fn inherit(cfg: ErgConfig, shared: SharedCompilerResource) -> Self
    where
        Self: Sized;
    fn inherit_with_name(cfg: ErgConfig, mod_name: Str, shared: SharedCompilerResource) -> Self
    where
        Self: Sized;
    fn build(
        &mut self,
        src: String,
        mode: &str,
    ) -> Result<CompleteArtifact<T>, IncompleteArtifact<T>>;
    fn build_from_ast(
        &mut self,
        ast: AST,
        mode: &str,
    ) -> Result<CompleteArtifact<T>, IncompleteArtifact<T>>;
    fn pop_context(&mut self) -> Option<ModuleContext>;
    fn get_context(&self) -> Option<&ModuleContext>;
}

pub trait BuildRunnable<T = HIR>: Buildable<T> + Runnable + 'static {
    fn build_module(&mut self) -> Result<CompleteArtifact<T>, IncompleteArtifact<T>> {
        let src = self.cfg_mut().input.read();
        self.build(src, "exec")
    }
}
