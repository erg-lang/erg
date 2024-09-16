use std::fmt;
use std::thread::{current, JoinHandle, ThreadId};

use erg_common::consts::{DEBUG_MODE, SINGLE_THREAD};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    pub total: usize,
    pub running: usize,
    pub finished: usize,
}

#[derive(Debug, Clone, Default)]
pub struct SharedPromises {
    graph: SharedModuleGraph,
    pub(crate) root: NormalizedPathBuf,
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
    pub fn new(graph: SharedModuleGraph, root: NormalizedPathBuf) -> Self {
        Self {
            graph,
            root,
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

    pub fn mark_as_joined(&self, path: impl Into<NormalizedPathBuf>) {
        let path = path.into();
        self.promises.borrow_mut().insert(path, Promise::Joined);
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

    /// If the path is not registered, return `false`.
    pub fn is_joined(&self, path: &NormalizedPathBuf) -> bool {
        self.promises
            .borrow()
            .get(path)
            .is_some_and(|promise| promise.is_joined())
    }

    /// If the path is not registered, return `false`.
    pub fn is_finished(&self, path: &NormalizedPathBuf) -> bool {
        self.promises
            .borrow()
            .get(path)
            .is_some_and(|promise| promise.is_finished())
    }

    pub fn wait_until_finished(&self, path: &NormalizedPathBuf) {
        if self.promises.borrow().get(path).is_none() {
            panic!("not registered: {path}");
        }
        while !self.is_finished(path) {
            safe_yield();
        }
    }

    pub fn join(&self, path: &NormalizedPathBuf) -> std::thread::Result<()> {
        if !self.graph.entries().contains(path) {
            return Err(Box::new(format!("not registered: {path}")));
        }
        if self.graph.ancestors(path).contains(&self.root) {
            // cycle detected, `self.path` must not in the dependencies
            // Erg analysis processes never join ancestor threads (although joining ancestors itself is allowed in Rust)
            // self.wait_until_finished(path);
            return Ok(());
        }
        if SINGLE_THREAD {
            assert!(self.is_joined(path));
            return Ok(());
        }
        // Suppose A depends on B and C, and B depends on C.
        // In this case, B must join C before A joins C. Otherwise, a deadlock will occur.
        let children = self.graph.children(path);
        for child in children.iter() {
            if child == &self.root {
                continue;
            } else if self.graph.depends_on(&self.root, child) {
                self.wait_until_finished(path);
                return Ok(());
            }
        }
        while let Some(Promise::Joining) | None = self.promises.borrow().get(path) {
            safe_yield();
        }
        if self.is_joined(path) {
            return Ok(());
        }
        let promise = self.promises.borrow_mut().get_mut(path).unwrap().take();
        let Promise::Running { handle, .. } = promise else {
            *self.promises.borrow_mut().get_mut(path).unwrap() = promise;
            self.wait_until_finished(path);
            return Ok(());
        };
        if handle.thread().id() == current().id() {
            return Ok(());
        }
        let res = handle.join();
        *self.promises.borrow_mut().get_mut(path).unwrap() = Promise::Joined;
        res
    }

    pub fn join_children(&self) {
        let cur_id = std::thread::current().id();
        let mut paths = vec![];
        for (path, promise) in self.promises.borrow().iter() {
            if promise.parent_thread_id() != Some(cur_id) {
                continue;
            }
            paths.push(path.clone());
        }
        for path in paths {
            let _result = self.join(&path);
        }
    }

    pub fn join_all(&self) {
        let paths = self.promises.borrow().keys().cloned().collect::<Vec<_>>();
        for path in paths {
            let _result = self.join(&path);
        }
    }

    pub fn progress(&self) -> Progress {
        let mut total = 0;
        let mut running = 0;
        let mut finished = 0;
        for promise in self.promises.borrow().values() {
            match promise {
                Promise::Running { .. } => running += 1,
                Promise::Joining => finished += 1,
                Promise::Joined => finished += 1,
            }
            total += 1;
        }
        Progress {
            total,
            running,
            finished,
        }
    }
}
