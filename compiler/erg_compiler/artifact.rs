use crate::error::CompileErrors;
use crate::hir::HIR;

pub struct CompleteArtifact {
    pub hir: HIR,
    pub warns: CompileErrors,
}

impl CompleteArtifact {
    pub const fn new(hir: HIR, warns: CompileErrors) -> Self {
        Self { hir, warns }
    }
}

pub struct IncompleteArtifact {
    pub hir: Option<HIR>,
    pub errors: CompileErrors,
    pub warns: CompileErrors,
}

impl IncompleteArtifact {
    pub const fn new(hir: Option<HIR>, errors: CompileErrors, warns: CompileErrors) -> Self {
        Self { hir, errors, warns }
    }
}
