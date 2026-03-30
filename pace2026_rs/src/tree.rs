use num_bigint::BigUint;
use num_traits::Zero;
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct OriginalNode {
    pub left: Option<Box<OriginalNode>>,
    pub right: Option<Box<OriginalNode>>,
    pub label: Option<u32>,
}

#[derive(Clone, Debug)]
pub enum Expansion {
    Leaf(u32),
    Node(Box<Expansion>, Box<Expansion>),
}

#[derive(Clone, Debug, Default)]
pub struct Node {
    pub left: usize,
    pub right: usize,
    pub parent: usize,
    pub leaf_id: u32,
    pub cluster_mask: BigUint,
    pub is_leaf: bool,
    pub size: usize,
}

#[derive(Clone, Debug)]
pub struct ArenaTree {
    pub nodes: Vec<Node>,
    pub root: usize,
}

impl ArenaTree {
    pub fn new() -> Self {
        Self { nodes: vec![Node::default()], root: 0 }
    }

    pub fn add_node(&mut self, node: Node) -> usize {
        let idx = self.nodes.len();
        self.nodes.push(node);
        idx
    }
}

pub fn original_to_arena(node: &OriginalNode, tree: &mut ArenaTree) -> usize {
    if let Some(id) = node.label {
        let n = Node {
            left: 0, right: 0, parent: 0,
            leaf_id: id,
            cluster_mask: BigUint::from(1u32) << (id - 1),
            is_leaf: true,
            size: 1,
        };
        tree.add_node(n)
    } else {
        let l = original_to_arena(node.left.as_ref().unwrap(), tree);
        let r = original_to_arena(node.right.as_ref().unwrap(), tree);
        let mask = &tree.nodes[l].cluster_mask | &tree.nodes[r].cluster_mask;
        let size = tree.nodes[l].size + tree.nodes[r].size;
        let n_idx = tree.nodes.len();
        tree.nodes.push(Node {
            left: l, right: r, parent: 0,
            leaf_id: 0,
            cluster_mask: mask,
            is_leaf: false,
            size,
        });
        tree.nodes[l].parent = n_idx;
        tree.nodes[r].parent = n_idx;
        n_idx
    }
}

pub fn collect_cherries(tree: &ArenaTree, cherries: &mut HashSet<(u32, u32)>) {
    if tree.root == 0 { return; }
    for (i, node) in tree.nodes.iter().enumerate() {
        if i == 0 || node.is_leaf || node.left == 0 || node.right == 0 { continue; }
        let l = &tree.nodes[node.left];
        let r = &tree.nodes[node.right];
        if l.is_leaf && r.is_leaf {
            let a = l.leaf_id;
            let b = r.leaf_id;
            cherries.insert(if a < b { (a, b) } else { (b, a) });
        }
    }
}

pub fn cut_leaf_arena(tree: &mut ArenaTree, leaf_id: u32) {
    if tree.root == 0 { return; }
    let target_mask = BigUint::from(1u32) << (leaf_id - 1);
    if (&tree.nodes[tree.root].cluster_mask & &target_mask).is_zero() { return; }
    
    let mut leaf_idx = 0;
    for (i, n) in tree.nodes.iter().enumerate() {
        if n.is_leaf && n.leaf_id == leaf_id {
            leaf_idx = i; break;
        }
    }
    if leaf_idx == 0 { return; }
    
    let parent_idx = tree.nodes[leaf_idx].parent;
    if parent_idx == 0 {
        tree.root = 0;
        return;
    }
    
    let sibling_idx = if tree.nodes[parent_idx].left == leaf_idx {
        tree.nodes[parent_idx].right
    } else {
        tree.nodes[parent_idx].left
    };
    
    let gp_idx = tree.nodes[parent_idx].parent;
    if gp_idx == 0 {
        tree.root = sibling_idx;
        tree.nodes[sibling_idx].parent = 0;
    } else {
        if tree.nodes[gp_idx].left == parent_idx {
            tree.nodes[gp_idx].left = sibling_idx;
        } else {
            tree.nodes[gp_idx].right = sibling_idx;
        }
        tree.nodes[sibling_idx].parent = gp_idx;
        
        let mut curr = gp_idx;
        while curr != 0 {
            let l = tree.nodes[curr].left;
            let r = tree.nodes[curr].right;
            tree.nodes[curr].cluster_mask = &tree.nodes[l].cluster_mask | &tree.nodes[r].cluster_mask;
            tree.nodes[curr].size = tree.nodes[l].size + tree.nodes[r].size;
            curr = tree.nodes[curr].parent;
        }
    }
}

pub fn get_all_meta_leaves(node: &Node) -> Vec<u32> {
    let mut leaves = Vec::new();
    let mut mask = node.cluster_mask.clone();
    let mut idx = 1;
    while !mask.is_zero() {
        if (&mask & BigUint::from(1u32)) == BigUint::from(1u32) {
            leaves.push(idx);
        }
        mask >>= 1;
        idx += 1;
    }
    leaves
}
