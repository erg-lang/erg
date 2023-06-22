use erg_common::shared::Shared;
use erg_common::traits::Stream;

use crate::error::CompileErrors;

#[derive(Debug, Clone, Default)]
pub struct SharedCompileErrors(Shared<CompileErrors>);

impl SharedCompileErrors {
    pub fn new() -> Self {
        Self(Shared::new(CompileErrors::empty()))
    }

    pub fn extend(&self, errors: CompileErrors) {
        self.0.borrow_mut().extend(errors);
    }

    pub fn take(&self) -> CompileErrors {
        self.0.borrow_mut().take_all().into()
    }

    pub fn clear(&self) {
        self.0.borrow_mut().clear();
    }
}

pub type SharedCompileWarnings = SharedCompileErrors;
