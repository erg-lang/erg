use std::fmt;
use std::thread::{current, JoinHandle, ThreadId};

use erg_common::config::ErgConfig;
use erg_common::consts::{DEBUG_MODE, PARALLEL};
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
    Joined,
}

impl fmt::Display for Promise {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Running { handle, .. } => {
                write!(f, "running on thread {:?}", handle.thread().id())
            }
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
            Self::Joined { .. } => true,
            Self::Running { handle, .. } => handle.is_finished(),
        }
    }

    pub const fn is_joined(&self) -> bool {
        matches!(self, Self::Joined { .. })
    }

    pub fn thread_id(&self) -> Option<ThreadId> {
        match self {
            Self::Joined => None,
            Self::Running { handle, .. } => Some(handle.thread().id()),
        }
    }

    pub fn parent_thread_id(&self) -> Option<ThreadId> {
        match self {
            Self::Joined { .. } => None,
            Self::Running { parent, .. } => Some(*parent),
        }
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
        if !self.graph.entries().contains(path) {
            panic!("not registered: {path}");
        }
        while !self.is_finished(path) {
            safe_yield();
        }
    }

    /// waits for the promise to be finished, and then marks it as joined
    pub fn join(&self, path: &NormalizedPathBuf, cfg: &ErgConfig) {
        if !self.graph.entries().contains(path) {
            panic!("not registered: {path}");
        }
        let current = self.current_path();
        if self.graph.ancestors(path).contains(&current) || path == &current {
            // cycle detected, `current` must not in the dependencies
            // Erg analysis processes never join themselves / ancestor threads
            // self.wait_until_finished(path);
            return;
        }
        if !cfg.mode.is_language_server() && !self.graph.deep_depends_on(&current, path) {
            // no relation, so no need to join
            if DEBUG_MODE {
                panic!("not depends on: {current} -> {path}");
            }
            return;
        }
        if !PARALLEL {
            assert!(self.is_joined(path));
            return;
        }
        while self.promises.borrow().get(path).is_none() {
            safe_yield();
        }
        if self.is_joined(path) {
            return;
        }
        while !self.is_finished(path) {
            safe_yield();
        }
        *self.promises.borrow_mut().get_mut(path).unwrap() = Promise::Joined;
    }

    pub fn join_children(&self, cfg: &ErgConfig) {
        let cur_id = std::thread::current().id();
        let mut paths = vec![];
        for (path, promise) in self.promises.borrow().iter() {
            if promise.parent_thread_id() != Some(cur_id) {
                continue;
            }
            paths.push(path.clone());
        }
        for path in paths {
            self.join(&path, cfg);
        }
    }

    pub fn join_all(&self, cfg: &ErgConfig) {
        let paths = self.promises.borrow().keys().cloned().collect::<Vec<_>>();
        for path in paths {
            self.join(&path, cfg);
        }
    }

    pub fn thread_id(&self, path: &NormalizedPathBuf) -> Option<ThreadId> {
        self.promises
            .borrow()
            .get(path)
            .and_then(|promise| promise.thread_id())
    }

    pub fn current_path(&self) -> NormalizedPathBuf {
        let cur_id = current().id();
        for (path, promise) in self.promises.borrow().iter() {
            if promise.thread_id() == Some(cur_id) {
                return path.clone();
            }
        }
        self.root.clone()
    }

    pub fn progress(&self) -> Progress {
        let mut total = 0;
        let mut running = 0;
        let mut finished = 0;
        for promise in self.promises.borrow().values() {
            match promise {
                Promise::Running { .. } => running += 1,
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
