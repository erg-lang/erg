use std::fmt;
use std::thread::{current, JoinHandle, ThreadId};

use erg_common::consts::DEBUG_MODE;
use erg_common::dict::Dict;
use erg_common::pathutil::NormalizedPathBuf;
use erg_common::shared::Shared;
use erg_common::spawn::safe_yield;

use super::SharedModuleGraph;

/// transition:
/// Running(not finished) -> Running(finished) -> Joining -> Joined
#[derive(Debug)]
pub enum Promise {
    Running {
        parent: ThreadId,
        handle: JoinHandle<()>,
    },
    Joining,
    Joined,
}

impl fmt::Display for Promise {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Running { handle, .. } => {
                write!(f, "running on thread {:?}", handle.thread().id())
            }
            Self::Joining => write!(f, "joining"),
            Self::Joined => write!(f, "joined"),
        }
    }
}

impl Promise {
    pub fn running(handle: JoinHandle<()>) -> Self {
        Self::Running {
            parent: current().id(),
            handle,
        }
    }

    /// can be joined if `true`
    pub fn is_finished(&self) -> bool {
        match self {
            Self::Joined => true,
            Self::Joining => false,
            Self::Running { handle, .. } => handle.is_finished(),
        }
    }

    pub const fn is_joined(&self) -> bool {
        matches!(self, Self::Joined)
    }

    pub fn thread_id(&self) -> Option<ThreadId> {
        match self {
            Self::Joined | Self::Joining => None,
            Self::Running { handle, .. } => Some(handle.thread().id()),
        }
    }

    pub fn parent_thread_id(&self) -> Option<ThreadId> {
        match self {
            Self::Joined | Self::Joining => None,
            Self::Running { parent, .. } => Some(*parent),
        }
    }

    pub fn take(&mut self) -> Self {
        std::mem::replace(self, Self::Joining)
    }
}

#[derive(Debug, Clone, Default)]
pub struct SharedPromises {
    graph: SharedModuleGraph,
    pub(crate) path: NormalizedPathBuf,
    promises: Shared<Dict<NormalizedPathBuf, Promise>>,
}

impl fmt::Display for SharedPromises {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SharedPromises {{ ")?;
        for (path, promise) in self.promises.borrow().iter() {
            writeln!(f, "{}: {}, ", path.display(), promise)?;
        }
        write!(f, "}}")
    }
}

impl SharedPromises {
    pub fn new(graph: SharedModuleGraph, path: NormalizedPathBuf) -> Self {
        Self {
            graph,
            path,
            promises: Shared::new(Dict::new()),
        }
    }

    pub fn insert(&self, path: impl Into<NormalizedPathBuf>, handle: JoinHandle<()>) {
        let path = path.into();
        if self.is_registered(&path) {
            if DEBUG_MODE {
                panic!("already registered: {}", path.display());
            }
            return;
        }
        self.promises
            .borrow_mut()
            .insert(path, Promise::running(handle));
    }

    pub fn remove(&self, path: &NormalizedPathBuf) -> Option<Promise> {
        self.promises.borrow_mut().remove(path)
    }

    pub fn initialize(&self) {
        self.promises.borrow_mut().clear();
    }

    pub fn rename(&self, old: &NormalizedPathBuf, new: NormalizedPathBuf) {
        let Some(promise) = self.remove(old) else {
            return;
        };
        self.promises.borrow_mut().insert(new, promise);
    }

    pub fn is_registered(&self, path: &NormalizedPathBuf) -> bool {
        self.promises.borrow().get(path).is_some()
    }

    pub fn is_joined(&self, path: &NormalizedPathBuf) -> bool {
        self.promises
            .borrow()
            .get(path)
            .is_some_and(|promise| promise.is_joined())
    }

    pub fn can_be_joined(&self, path: &NormalizedPathBuf) -> bool {
        self.promises
            .borrow()
            .get(path)
            .is_some_and(|promise| promise.is_finished())
    }

    fn join_checked(&self, path: &NormalizedPathBuf, promise: Promise) -> std::thread::Result<()> {
        let Promise::Running { handle, parent } = promise else {
            *self.promises.borrow_mut().get_mut(path).unwrap() = promise;
            return Ok(());
        };
        if self.graph.ancestors(path).contains(&self.path) || handle.thread().id() == current().id()
        {
            // cycle detected, `self.path` must not in the dependencies
            // Erg analysis processes never join ancestor threads (although joining ancestors itself is allowed in Rust)
            *self.promises.borrow_mut().get_mut(path).unwrap() =
                Promise::Running { parent, handle };
            return Ok(());
        }
        // Suppose A depends on B and C, and B depends on C.
        // In this case, B must join C before A joins C. Otherwise, a deadlock will occur.
        let children = self.graph.children(path);
        for child in children.iter() {
            if child == &self.path {
                continue;
            } else if self.graph.depends_on(&self.path, child) {
                *self.promises.borrow_mut().get_mut(path).unwrap() =
                    Promise::Running { parent, handle };
                while self
                    .promises
                    .borrow()
                    .get(path)
                    .is_some_and(|p| !p.is_finished())
                {
                    safe_yield();
                }
                return Ok(());
            }
        }
        let res = handle.join();
        *self.promises.borrow_mut().get_mut(path).unwrap() = Promise::Joined;
        res
    }

    pub fn join(&self, path: &NormalizedPathBuf) -> std::thread::Result<()> {
        while let Some(Promise::Joining) | None = self.promises.borrow().get(path) {
            safe_yield();
        }
        if self.is_joined(path) {
            return Ok(());
        }
        let promise = self.promises.borrow_mut().get_mut(path).unwrap().take();
        self.join_checked(path, promise)
    }

    pub fn join_children(&self) {
        let cur_id = std::thread::current().id();
        let mut promises = vec![];
        for (path, promise) in self.promises.borrow_mut().iter_mut() {
            if promise.parent_thread_id() != Some(cur_id) {
                continue;
            }
            promises.push((path.clone(), promise.take()));
        }
        for (path, promise) in promises {
            let _result = self.join_checked(&path, promise);
        }
    }

    pub fn join_all(&self) {
        let mut promises = vec![];
        for (path, promise) in self.promises.borrow_mut().iter_mut() {
            promises.push((path.clone(), promise.take()));
        }
        for (path, promise) in promises {
            let _result = self.join_checked(&path, promise);
        }
    }
}
