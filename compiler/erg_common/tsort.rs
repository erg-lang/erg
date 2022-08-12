use crate::dict::Dict;

type NodeIdx = usize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node<T> {
    _id: T,
    depends_on: Vec<NodeIdx>,
}

impl<T> Node<T> {
    pub const fn new(id: T, depends_on: Vec<NodeIdx>) -> Self {
        Node {
            _id: id,
            depends_on,
        }
    }
}

pub type Graph<T> = Vec<Node<T>>;

fn reorder_by_idx<T>(mut v: Vec<T>, idx: Vec<NodeIdx>) -> Vec<T> {
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

fn dfs<T>(g: &Graph<T>, v: NodeIdx, used: &mut Vec<bool>, idx: &mut Vec<NodeIdx>) {
    used[v] = true;
    for &node_id in g[v].depends_on.iter() {
        if !used[node_id] {
            dfs(g, node_id, used, idx);
        }
    }
    idx.push(v);
}

/// perform topological sort on a graph
pub fn tsort<T>(g: Graph<T>) -> Graph<T> {
    let n = g.len();
    let mut idx = Vec::with_capacity(n);
    let mut used = vec![false; n];
    for v in 0..n {
        if !used[v] {
            dfs(&g, v, &mut used, &mut idx);
        }
    }
    reorder_by_idx(g, idx)
}

fn _test() {
    let v = vec!["e", "d", "b", "a", "c"];
    let idx = vec![3, 2, 4, 1, 0];
    assert_eq!(vec!["a", "b", "c", "d", "e"], reorder_by_idx(v, idx));

    let en_0 = Node::new("even n", vec![4, 1]);
    let o0_1 = Node::new("odd 0", vec![]);
    let on_2 = Node::new("odd n", vec![3, 0]);
    let e0_3 = Node::new("even 0", vec![]);
    let ond_4 = Node::new("odd n (decl)", vec![]);
    let sorted = vec![
        ond_4.clone(),
        o0_1.clone(),
        en_0.clone(),
        e0_3.clone(),
        on_2.clone(),
    ];
    let dag = vec![en_0, o0_1, on_2, e0_3, ond_4];
    assert_eq!(sorted, tsort(dag));
}
