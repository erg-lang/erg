use std::fmt;
use std::path::{Path, PathBuf};

use erg_common::set::Set;
use erg_common::shared::{MappedRwLockReadGuard, RwLockReadGuard, Shared};
use erg_common::tsort::{tsort, Graph, Node, TopoSortError};
use erg_common::{normalize_path, set};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IncRefError {
    CycleDetected,
}

impl IncRefError {
    pub const fn is_cycle_detected(&self) -> bool {
        matches!(self, Self::CycleDetected)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ModuleGraph(Graph<PathBuf, ()>);

impl fmt::Display for ModuleGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "ModuleGraph {{")?;
        for node in self.0.iter() {
            writeln!(f, "{} depends on {{", node.id.display())?;
            for dep in node.depends_on.iter() {
                writeln!(f, "{}, ", dep.display())?;
            }
            writeln!(f, "}}, ")?;
        }
        write!(f, "}}")
    }
}

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
        let path = normalize_path(path.to_path_buf());
        self.0.iter().find(|n| n.id == path)
    }

    fn parents(&self, path: &Path) -> Option<&Set<PathBuf>> {
        let path = normalize_path(path.to_path_buf());
        self.0.iter().find(|n| n.id == path).map(|n| &n.depends_on)
    }

    pub fn ancestors(&self, path: &Path) -> Set<PathBuf> {
        let path = normalize_path(path.to_path_buf());
        let mut ancestors = set! {};
        if let Some(parents) = self.parents(&path) {
            for parent in parents.iter() {
                ancestors.insert(parent.clone());
                ancestors.extend(self.ancestors(parent));
            }
        }
        ancestors
    }

    pub fn add_node_if_none(&mut self, path: &Path) {
        let path = normalize_path(path.to_path_buf());
        if self.0.iter().all(|n| n.id != path) {
            let node = Node::new(path, (), set! {});
            self.0.push(node);
        }
    }

    /// returns Err (and do nothing) if this operation makes a cycle
    pub fn inc_ref(&mut self, referrer: &Path, depends_on: PathBuf) -> Result<(), IncRefError> {
        let referrer = normalize_path(referrer.to_path_buf());
        let depends_on = normalize_path(depends_on);
        if self.ancestors(&depends_on).contains(&referrer) && referrer != depends_on {
            return Err(IncRefError::CycleDetected);
        }
        if let Some(node) = self.0.iter_mut().find(|n| n.id == referrer) {
            if referrer == depends_on {
                return Ok(());
            }
            node.push_dep(depends_on);
        } else {
            unreachable!("node not found: {}", referrer.display());
        }
        Ok(())
    }

    pub fn iter(&self) -> impl Iterator<Item = &Node<PathBuf, ()>> {
        self.0.iter()
    }

    #[allow(clippy::result_unit_err)]
    pub fn sorted(self) -> Result<Self, TopoSortError> {
        tsort(self.0).map(Self)
    }

    #[allow(clippy::result_unit_err)]
    pub fn sort(&mut self) -> Result<(), TopoSortError> {
        *self = std::mem::take(self).sorted()?;
        Ok(())
    }

    pub fn remove(&mut self, path: &Path) {
        let path = normalize_path(path.to_path_buf());
        self.0.retain(|n| n.id != path);
    }

    pub fn rename_path(&mut self, old: &Path, new: PathBuf) {
        let old = normalize_path(old.to_path_buf());
        let new = normalize_path(new);
        for node in self.0.iter_mut() {
            if node.id == old {
                node.id = new.clone();
            }
            if node.depends_on.contains(&old) {
                node.depends_on.insert(new.clone());
            }
            node.depends_on.retain(|p| *p != old);
        }
    }

    pub fn initialize(&mut self) {
        self.0.clear();
    }
}

#[derive(Debug, Clone, Default)]
pub struct SharedModuleGraph(Shared<ModuleGraph>);

impl fmt::Display for SharedModuleGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.borrow())
    }
}

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

    pub fn get_node(&self, path: &Path) -> Option<MappedRwLockReadGuard<Node<PathBuf, ()>>> {
        if self.0.borrow().get_node(path).is_some() {
            Some(RwLockReadGuard::map(self.0.borrow(), |graph| {
                graph.get_node(path).unwrap()
            }))
        } else {
            None
        }
    }

    pub fn ancestors(&self, path: &Path) -> Set<PathBuf> {
        self.0.borrow().ancestors(path)
    }

    pub fn add_node_if_none(&self, path: &Path) {
        self.0.borrow_mut().add_node_if_none(path);
    }

    pub fn inc_ref(&self, referrer: &Path, depends_on: PathBuf) -> Result<(), IncRefError> {
        self.0.borrow_mut().inc_ref(referrer, depends_on)
    }

    pub fn ref_inner(&self) -> RwLockReadGuard<ModuleGraph> {
        self.0.borrow()
    }

    pub fn remove(&self, path: &Path) {
        self.0.borrow_mut().remove(path);
    }

    pub fn rename_path(&self, old: &Path, new: PathBuf) {
        self.0.borrow_mut().rename_path(old, new);
    }

    #[allow(clippy::result_unit_err)]
    pub fn sort(&self) -> Result<(), TopoSortError> {
        self.0.borrow_mut().sort()
    }

    pub fn initialize(&self) {
        self.0.borrow_mut().initialize();
    }
}
