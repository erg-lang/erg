use std::fmt;

use erg_common::traits::Stream;

use crate::error::CompileErrors;
use crate::hir::HIR;

#[derive(Debug)]
pub struct CompleteArtifact {
    pub hir: HIR,
    pub warns: CompileErrors,
}

impl CompleteArtifact {
    pub const fn new(hir: HIR, warns: CompileErrors) -> Self {
        Self { hir, warns }
    }
}

#[derive(Debug)]
pub struct IncompleteArtifact {
    pub hir: Option<HIR>,
    pub errors: CompileErrors,
    pub warns: CompileErrors,
}

impl fmt::Display for IncompleteArtifact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.warns.is_empty() {
            writeln!(f, "{}", self.warns)?;
        }
        write!(f, "{}", self.errors)
    }
}

impl std::error::Error for IncompleteArtifact {}

impl IncompleteArtifact {
    pub const fn new(hir: Option<HIR>, errors: CompileErrors, warns: CompileErrors) -> Self {
        Self { hir, errors, warns }
    }
}
