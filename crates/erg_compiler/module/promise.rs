use std::path::{Path, PathBuf};
use std::thread::JoinHandle;

use erg_common::dict::Dict;
use erg_common::shared::Shared;

use crate::artifact::ErrorArtifact;

#[derive(Debug)]
pub enum Promise {
    Running(JoinHandle<Result<(), ErrorArtifact>>),
    Finished,
}

impl Promise {
    pub fn is_finished(&self) -> bool {
        match self {
            Self::Finished => true,
            Self::Running(handle) => handle.is_finished(),
        }
    }

    pub fn take(&mut self) -> Self {
        std::mem::replace(self, Self::Finished)
    }
}

#[derive(Debug, Clone, Default)]
pub struct SharedPromises(Shared<Dict<PathBuf, Promise>>);

impl SharedPromises {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, path: PathBuf, handle: JoinHandle<Result<(), ErrorArtifact>>) {
        self.0.borrow_mut().insert(path, Promise::Running(handle));
    }

    pub fn is_registered(&self, path: &Path) -> bool {
        self.0.borrow_mut().get(path).is_some()
    }

    pub fn is_finished(&self, path: &Path) -> bool {
        self.0
            .borrow_mut()
            .get(path)
            .is_some_and(|promise| promise.is_finished())
    }

    pub fn join(&self, path: &Path) -> Result<(), ErrorArtifact> {
        let promise = self.0.borrow_mut().get_mut(path).unwrap().take();
        let Promise::Running(handle) = promise else { todo!() };
        handle.join().unwrap()
    }
}
