use std::fmt;

use erg_common::consts::DEBUG_MODE;
use erg_common::dict::Dict;
use erg_common::pathutil::NormalizedPathBuf;
use erg_common::set;
use erg_common::set::Set;
use erg_common::shared::{MappedRwLockReadGuard, RwLockReadGuard, Shared};
use erg_common::tsort::{tsort, Graph, Node, TopoSortError};

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
pub struct ModuleGraph {
    graph: Graph<NormalizedPathBuf, ()>,
    index: Dict<NormalizedPathBuf, usize>,
}

impl fmt::Display for ModuleGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "ModuleGraph {{")?;
        for node in self.graph.iter() {
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
    type Item = Node<NormalizedPathBuf, ()>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.graph.into_iter()
    }
}

impl ModuleGraph {
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
            index: Dict::new(),
        }
    }

    pub fn get_node(&self, path: &NormalizedPathBuf) -> Option<&Node<NormalizedPathBuf, ()>> {
        self.index.get(path).map(|&i| &self.graph[i])
    }

    pub fn get_mut_node(
        &mut self,
        path: &NormalizedPathBuf,
    ) -> Option<&mut Node<NormalizedPathBuf, ()>> {
        self.index.get(path).map(|&i| &mut self.graph[i])
    }

    /// if `path` directly depends on `target`, returns `true`, else `false`.
    /// if `path` not found, returns `false`.
    /// O(1)
    pub fn depends_on(&self, path: &NormalizedPathBuf, target: &NormalizedPathBuf) -> bool {
        let path = NormalizedPathBuf::new(path.to_path_buf());
        let target = NormalizedPathBuf::new(target.to_path_buf());
        self.get_node(&path)
            .map(|n| n.depends_on.contains(&target))
            .unwrap_or(false)
    }

    /// if `path` depends on `target`, returns `true`, else `false`.
    /// if `path` not found, returns `false`.
    pub fn deep_depends_on(&self, path: &NormalizedPathBuf, target: &NormalizedPathBuf) -> bool {
        let path = NormalizedPathBuf::new(path.to_path_buf());
        let target = NormalizedPathBuf::new(target.to_path_buf());
        let mut visited = set! {};
        self.deep_depends_on_(&path, &target, &mut visited)
    }

    fn deep_depends_on_<'p>(
        &'p self,
        path: &'p NormalizedPathBuf,
        target: &NormalizedPathBuf,
        visited: &mut Set<&'p NormalizedPathBuf>,
    ) -> bool {
        if !visited.insert(path) {
            return false;
        }
        self.get_node(path)
            .map(|n| {
                n.depends_on.contains(target)
                    || n.depends_on
                        .iter()
                        .any(|p| self.deep_depends_on_(p, target, visited))
            })
            .unwrap_or(false)
    }

    /// (usually) `path` is not contained.
    /// O(N)
    pub fn children<'p>(
        &'p self,
        path: &'p NormalizedPathBuf,
    ) -> impl Iterator<Item = NormalizedPathBuf> + 'p {
        self.graph
            .iter()
            .filter(|n| n.depends_on.contains(path))
            .map(|n| n.id.clone())
    }

    /// (usually) `path` is not contained.
    /// O(1)
    pub fn parents(&self, path: &NormalizedPathBuf) -> Option<&Set<NormalizedPathBuf>> {
        self.get_node(path).map(|n| &n.depends_on)
    }

    /// ```erg
    /// # a.er
    /// b = import "b"
    /// # -> a: child, b: parent
    /// # b.er
    /// c = import "c"
    /// # -> ancestors(a) == {b, c}
    /// ```
    /// O(N)
    pub fn ancestors<'p>(&'p self, path: &'p NormalizedPathBuf) -> Set<&'p NormalizedPathBuf> {
        let mut ancestors = set! {};
        let mut visited = set! {};
        self.ancestors_(path, &mut ancestors, &mut visited);
        ancestors
    }

    fn ancestors_<'p>(
        &'p self,
        path: &'p NormalizedPathBuf,
        ancestors: &mut Set<&'p NormalizedPathBuf>,
        visited: &mut Set<&'p NormalizedPathBuf>,
    ) {
        if !visited.insert(path) {
            return;
        }
        if let Some(parents) = self.parents(path) {
            for parent in parents.iter() {
                if ancestors.insert(parent) {
                    self.ancestors_(parent, ancestors, visited);
                }
            }
        }
    }

    pub fn add_node_if_none(&mut self, path: &NormalizedPathBuf) {
        if path.is_dir() {
            if DEBUG_MODE {
                panic!("path is a directory: {path}");
            }
            return;
        }
        if self.index.get(path).is_none() {
            let node = Node::new(path.clone(), (), set! {});
            self.graph.push(node);
            self.index.insert(path.clone(), self.graph.len() - 1);
        }
    }

    /// returns Err (and do nothing) if this operation makes a cycle
    pub fn inc_ref(
        &mut self,
        referrer: &NormalizedPathBuf,
        depends_on: NormalizedPathBuf,
    ) -> Result<(), IncRefError> {
        if depends_on.is_dir() {
            if DEBUG_MODE {
                panic!("path is a directory: {depends_on}");
            }
            return Ok(());
        }
        self.add_node_if_none(referrer);
        if referrer == &depends_on {
            return Ok(());
        }
        if self.deep_depends_on(&depends_on, referrer) {
            return Err(IncRefError::CycleDetected);
        }
        if let Some(node) = self.get_mut_node(referrer) {
            node.push_dep(depends_on);
        } else {
            unreachable!("node not found: {}", referrer.display());
        }
        Ok(())
    }

    pub fn iter(&self) -> impl Iterator<Item = &Node<NormalizedPathBuf, ()>> {
        self.graph.iter()
    }

    pub fn sorted(self) -> Result<Self, TopoSortError> {
        tsort(self.graph).map(|graph| {
            let index = graph
                .iter()
                .map(|n| n.id.clone())
                .enumerate()
                .map(|(i, path)| (path, i))
                .collect();
            Self { graph, index }
        })
    }

    #[allow(clippy::result_unit_err)]
    pub fn sort(&mut self) -> Result<(), TopoSortError> {
        *self = std::mem::take(self).sorted()?;
        Ok(())
    }

    /// Do not erase relationships with modules that depend on `path`.
    /// O(N)
    pub fn remove(&mut self, path: &NormalizedPathBuf) {
        if let Some(&i) = self.index.get(path) {
            self.graph.remove(i);
            self.index.remove(path);
            // shift indices
            for index in self.index.values_mut() {
                if *index > i {
                    *index -= 1;
                }
            }
        }
        for node in self.graph.iter_mut() {
            node.depends_on.retain(|p| p != path);
        }
    }

    /// O(N)
    pub fn rename_path(&mut self, old: &NormalizedPathBuf, new: NormalizedPathBuf) {
        for node in self.graph.iter_mut() {
            if &node.id == old {
                node.id = new.clone();
            }
            if node.depends_on.contains(old) {
                node.depends_on.insert(new.clone());
            }
            node.depends_on.retain(|p| p != old);
        }
    }

    pub fn initialize(&mut self) {
        self.graph.clear();
        self.index.clear();
    }

    pub fn display_parents(
        &self,
        lev: usize,
        id: &NormalizedPathBuf,
        appeared: &mut Set<NormalizedPathBuf>,
    ) -> String {
        let mut s = String::new();
        let Some(parents) = self.parents(id) else {
            return s;
        };
        for parent in parents.iter() {
            s.push_str(&format!("{}-> {}\n", "    ".repeat(lev), parent.display()));
            if appeared.contains(parent) {
                continue;
            }
            s.push_str(&self.display_parents(lev + 1, parent, appeared));
            appeared.insert(parent.clone());
        }
        s
    }

    pub fn display(&self) -> String {
        let mut s = String::new();
        let mut appeared = set! {};
        for node in self.graph.iter() {
            let mut children = self.children(&node.id);
            if children.next().is_some() || appeared.contains(&node.id) {
                continue;
            }
            s.push_str(&format!("{}\n", node.id.display()));
            s.push_str(&self.display_parents(1, &node.id, &mut appeared));
            appeared.insert(node.id.clone());
        }
        s
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
    type Item = Node<NormalizedPathBuf, ()>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_inner().into_iter()
    }
}

impl SharedModuleGraph {
    pub fn new() -> Self {
        Self(Shared::new(ModuleGraph::new()))
    }

    pub fn get_node(
        &self,
        path: &NormalizedPathBuf,
    ) -> Option<MappedRwLockReadGuard<Node<NormalizedPathBuf, ()>>> {
        RwLockReadGuard::try_map(self.0.borrow(), |graph| graph.get_node(path)).ok()
    }

    /// if `path` directly depends on `target`, returns `true`, else `false`.
    /// if `path` not found, returns `false`.
    /// O(1)
    pub fn depends_on(&self, path: &NormalizedPathBuf, target: &NormalizedPathBuf) -> bool {
        self.0.borrow().depends_on(path, target)
    }

    pub fn deep_depends_on(&self, path: &NormalizedPathBuf, target: &NormalizedPathBuf) -> bool {
        self.0.borrow().deep_depends_on(path, target)
    }

    /// (usually) `path` is not contained.
    /// O(N)
    pub fn children(&self, path: &NormalizedPathBuf) -> Set<NormalizedPathBuf> {
        self.0.borrow().children(path).collect()
    }

    /// (usually) `path` is not contained.
    /// O(N)
    pub fn ancestors(&self, path: &NormalizedPathBuf) -> Set<NormalizedPathBuf> {
        self.0.borrow().ancestors(path).cloned()
    }

    pub fn add_node_if_none(&self, path: &NormalizedPathBuf) {
        self.0.borrow_mut().add_node_if_none(path);
    }

    pub fn inc_ref(
        &self,
        referrer: &NormalizedPathBuf,
        depends_on: NormalizedPathBuf,
    ) -> Result<(), IncRefError> {
        self.0.borrow_mut().inc_ref(referrer, depends_on)
    }

    pub fn ref_inner(&self) -> RwLockReadGuard<ModuleGraph> {
        self.0.borrow()
    }

    pub fn remove(&self, path: &NormalizedPathBuf) {
        self.0.borrow_mut().remove(path);
    }

    pub fn rename_path(&self, old: &NormalizedPathBuf, new: NormalizedPathBuf) {
        self.0.borrow_mut().rename_path(old, new);
    }

    #[allow(clippy::result_unit_err)]
    pub fn sort(&self) -> Result<(), TopoSortError> {
        self.0.borrow_mut().sort()
    }

    pub fn entries(&self) -> Set<NormalizedPathBuf> {
        self.0.borrow().iter().map(|n| n.id.clone()).collect()
    }

    pub fn initialize(&self) {
        self.0.borrow_mut().initialize();
    }

    pub fn clone_inner(&self) -> ModuleGraph {
        self.0.borrow().clone()
    }

    pub fn display(&self) -> String {
        self.0.borrow().display()
    }
}
