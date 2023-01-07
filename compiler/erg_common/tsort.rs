//! Topological sort
use crate::dict::Dict;
use crate::set::Set;

use std::hash::Hash;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TopoSortError {
    CyclicReference,
    KeyNotFound,
}

impl std::fmt::Display for TopoSortError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for TopoSortError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node<T: Eq + Hash, U> {
    pub id: T,
    pub data: U,
    depends_on: Set<T>,
}

impl<T: Eq + Hash, U> Node<T, U> {
    pub const fn new(id: T, data: U, depends_on: Set<T>) -> Self {
        Node {
            id,
            data,
            depends_on,
        }
    }

    pub fn push_dep(&mut self, dep: T) {
        self.depends_on.insert(dep);
    }

    pub fn depends_on(&self, dep: &T) -> bool {
        self.depends_on.contains(dep)
    }
}

pub type Graph<T, U> = Vec<Node<T, U>>;

fn _reorder_by_idx<T>(mut v: Vec<T>, idx: Vec<usize>) -> Vec<T> {
    let mut swap_table = Dict::new();
    for (node_id, mut sort_i) in idx.into_iter().enumerate() {
        if node_id == sort_i {
            continue;
        }
        while let Some(moved_to) = swap_table.get(&sort_i) {
            sort_i = *moved_to;
        }
        v.swap(node_id, sort_i);
        swap_table.insert(node_id, sort_i);
    }
    v
}

fn reorder_by_key<T: Eq + Hash, U>(mut g: Graph<T, U>, idx: Vec<T>) -> Graph<T, U> {
    g.sort_by_key(|node| idx.iter().position(|k| k == &node.id).unwrap());
    g
}

fn dfs<T: Eq + Hash + Clone, U>(
    g: &Graph<T, U>,
    v: T,
    used: &mut Set<T>,
    idx: &mut Vec<T>,
) -> Result<(), TopoSortError> {
    used.insert(v.clone());
    let Some(vertex) = g.iter().find(|n| n.id == v) else {
        return Err(TopoSortError::KeyNotFound);
    };
    for node_id in vertex.depends_on.iter() {
        // detecting cycles
        if used.contains(node_id) && !idx.contains(node_id) {
            return Err(TopoSortError::CyclicReference);
        }
        if !used.contains(node_id) {
            dfs(g, node_id.clone(), used, idx)?;
        }
    }
    idx.push(v);
    Ok(())
}

/// perform topological sort on a graph
#[allow(clippy::result_unit_err)]
pub fn tsort<T: Eq + Hash + Clone, U>(g: Graph<T, U>) -> Result<Graph<T, U>, TopoSortError> {
    let n = g.len();
    let mut idx = Vec::with_capacity(n);
    let mut used = Set::new();
    for v in g.iter() {
        if !used.contains(&v.id) {
            dfs(&g, v.id.clone(), &mut used, &mut idx)?;
        }
    }
    Ok(reorder_by_key(g, idx))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::set;

    #[test]
    fn test_tsort() -> Result<(), TopoSortError> {
        let v = vec!["e", "d", "b", "a", "c"];
        let idx = vec![3, 2, 4, 1, 0];
        assert_eq!(vec!["a", "b", "c", "d", "e"], _reorder_by_idx(v, idx));

        // this is invalid, cause a cyclic reference exists
        // ```
        // odd 0 = False
        // odd n = even n - 1
        // even 0 = True
        // even n = odd n - 1
        // ```
        let even = Node::new("even n", (), set!["odd n", "True"]);
        let odd = Node::new("odd n", (), set!["even n", "False"]);
        let tru = Node::new("True", (), set![]);
        let fls = Node::new("False", (), set![]);
        let dag = vec![even, odd, tru.clone(), fls.clone()];
        assert!(tsort(dag).is_err());

        // this is valid, cause declaration exists
        // ```
        // odd: Nat -> Bool
        // odd 0 = False
        // odd n = even n - 1
        // even 0 = True
        // even n = odd n - 1 # this refers the declaration, not the definition
        // ```
        let even = Node::new("even n", (), set!["odd n (decl)", "True"]);
        let odd = Node::new("odd n", (), set!["even n", "False"]);
        let odd_decl = Node::new("odd n (decl)", (), set![]);
        let dag = vec![
            even,
            odd.clone(),
            odd_decl.clone(),
            fls.clone(),
            tru.clone(),
        ];
        let sorted = tsort(dag)?;
        assert!(sorted[0] == odd_decl || sorted[0] == fls || sorted[0] == tru);
        assert_eq!(sorted[4], odd);
        Ok(())
    }
}
