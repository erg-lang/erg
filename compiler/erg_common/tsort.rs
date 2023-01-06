use crate::dict::Dict;
use crate::set;
use crate::set::Set;

use std::hash::Hash;

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
) -> Result<(), ()> {
    used.insert(v.clone());
    for node_id in g.iter().find(|n| n.id == v).unwrap().depends_on.iter() {
        // detecting cycles
        if used.contains(node_id) && !idx.contains(node_id) {
            return Err(());
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
pub fn tsort<T: Eq + Hash + Clone, U>(g: Graph<T, U>) -> Result<Graph<T, U>, ()> {
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

fn _test() -> Result<(), ()> {
    let v = vec!["e", "d", "b", "a", "c"];
    let idx = vec![3, 2, 4, 1, 0];
    assert_eq!(vec!["a", "b", "c", "d", "e"], _reorder_by_idx(v, idx));

    let en_0 = Node::new("even n", (), set!["odd n (decl)", "odd 0"]);
    let o0_1 = Node::new("odd 0", (), set![]);
    let on_2 = Node::new("odd n", (), set!["even 0", "even n"]);
    let e0_3 = Node::new("even 0", (), set![]);
    let ond_4 = Node::new("odd n (decl)", (), set![]);
    let sorted = vec![
        ond_4.clone(),
        o0_1.clone(),
        en_0.clone(),
        e0_3.clone(),
        on_2.clone(),
    ];
    let dag = vec![en_0, o0_1, on_2, e0_3, ond_4];
    assert_eq!(sorted, tsort(dag)?);
    Ok(())
}
