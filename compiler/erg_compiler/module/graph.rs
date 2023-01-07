use std::path::{Path, PathBuf};

use erg_common::shared::Shared;
use erg_common::tsort::{tsort, Graph, Node};
use erg_common::{normalize_path, set};

#[derive(Debug, Clone, Default)]
pub struct ModuleGraph(Graph<PathBuf, ()>);

impl IntoIterator for ModuleGraph {
    type Item = Node<PathBuf, ()>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl ModuleGraph {
    pub fn new() -> Self {
        Self(Graph::new())
    }

    pub fn get_node(&self, path: &Path) -> Option<&Node<PathBuf, ()>> {
        self.0.iter().find(|n| n.id == path)
    }

    pub fn add_node_if_none(&mut self, path: &Path) {
        let path = normalize_path(path.to_path_buf());
        if self.0.iter().all(|n| n.id != path) {
            let node = Node::new(path, (), set! {});
            self.0.push(node);
        }
    }

    pub fn inc_ref(&mut self, referrer: &Path, depends_on: PathBuf) {
        let depends_on = normalize_path(depends_on);
        if let Some(node) = self.0.iter_mut().find(|n| n.id == referrer) {
            if referrer == depends_on {
                return;
            }
            node.push_dep(depends_on);
        } else {
            unreachable!("node not found: {}", referrer.display());
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Node<PathBuf, ()>> {
        self.0.iter()
    }

    #[allow(clippy::result_unit_err)]
    pub fn sorted(self) -> Result<Self, ()> {
        tsort(self.0).map(Self)
    }

    #[allow(clippy::result_unit_err)]
    pub fn sort(&mut self) -> Result<(), ()> {
        *self = std::mem::take(self).sorted()?;
        Ok(())
    }

    pub fn initialize(&mut self) {
        self.0.clear();
    }
}

#[derive(Debug, Clone, Default)]
pub struct SharedModuleGraph(Shared<ModuleGraph>);

impl IntoIterator for SharedModuleGraph {
    type Item = Node<PathBuf, ()>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_inner().into_iter()
    }
}

impl SharedModuleGraph {
    pub fn new() -> Self {
        Self(Shared::new(ModuleGraph::new()))
    }

    /// SAFETY: don't hold this reference before sorting
    pub fn get_node(&self, path: &Path) -> Option<&Node<PathBuf, ()>> {
        let ref_graph = unsafe { self.0.as_ptr().as_ref().unwrap() };
        ref_graph.get_node(path)
    }

    pub fn add_node_if_none(&self, path: &Path) {
        self.0.borrow_mut().add_node_if_none(path);
    }

    pub fn inc_ref(&self, referrer: &Path, depends_on: PathBuf) {
        self.0.borrow_mut().inc_ref(referrer, depends_on);
    }

    /// SAFETY: don't hold this iterator before sorting
    pub fn iter(&self) -> impl Iterator<Item = &Node<PathBuf, ()>> {
        let ref_graph = unsafe { self.0.as_ptr().as_ref().unwrap() };
        ref_graph.iter()
    }

    #[allow(clippy::result_unit_err)]
    pub fn sort(&self) -> Result<(), ()> {
        self.0.borrow_mut().sort()
    }

    pub fn initialize(&self) {
        self.0.borrow_mut().initialize();
    }
}
