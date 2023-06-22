use std::path::{Path, PathBuf};
use std::thread::{current, JoinHandle, ThreadId};

use erg_common::dict::Dict;
use erg_common::shared::Shared;

#[derive(Debug)]
pub enum Promise {
    Running {
        parent: ThreadId,
        handle: JoinHandle<()>,
    },
    Finished,
}

impl Promise {
    pub fn running(handle: JoinHandle<()>) -> Self {
        Self::Running {
            parent: current().id(),
            handle,
        }
    }

    pub fn is_finished(&self) -> bool {
        match self {
            Self::Finished => true,
            Self::Running { handle, .. } => handle.is_finished(),
        }
    }

    pub fn thread_id(&self) -> Option<ThreadId> {
        match self {
            Self::Finished => None,
            Self::Running { handle, .. } => Some(handle.thread().id()),
        }
    }

    pub fn parent_thread_id(&self) -> Option<ThreadId> {
        match self {
            Self::Finished => None,
            Self::Running { parent, .. } => Some(*parent),
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

    pub fn insert(&self, path: PathBuf, handle: JoinHandle<()>) {
        self.0.borrow_mut().insert(path, Promise::running(handle));
    }

    pub fn is_registered(&self, path: &Path) -> bool {
        self.0.borrow().get(path).is_some()
    }

    pub fn is_finished(&self, path: &Path) -> bool {
        self.0
            .borrow()
            .get(path)
            .is_some_and(|promise| promise.is_finished())
    }

    pub fn join(&self, path: &Path) -> std::thread::Result<()> {
        let promise = self.0.borrow_mut().get_mut(path).unwrap().take();
        let Promise::Running{ handle, .. } = promise else { todo!() };
        handle.join()
    }

    pub fn join_children(&self) {
        let cur_id = std::thread::current().id();
        let mut handles = vec![];
        for (_path, promise) in self.0.borrow_mut().iter_mut() {
            if promise.parent_thread_id() != Some(cur_id) {
                continue;
            }
            if let Promise::Running { handle, .. } = promise.take() {
                handles.push(handle);
            }
        }
        for handle in handles {
            let _result = handle.join();
        }
    }

    pub fn join_all(&self) {
        let mut handles = vec![];
        for (_path, promise) in self.0.borrow_mut().iter_mut() {
            if let Promise::Running { handle, .. } = promise.take() {
                handles.push(handle);
            }
        }
        for handle in handles {
            let _result = handle.join();
        }
    }
}
