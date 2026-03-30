use std::collections::{HashMap, HashSet};
use crate::tree::{ArenaTree, Expansion, collect_cherries, Node};
use num_bigint::BigUint;

#[derive(Clone, Debug)]
pub struct State {
    pub tree1: ArenaTree,
    pub tree2: ArenaTree,
    pub expansions: HashMap<u32, Expansion>,
    pub next_id: u32,
    pub cut_components: Vec<Expansion>,
}

impl State {
    pub fn leaf_count(&self) -> usize {
        self.tree1.nodes[self.tree1.root].size
    }

    pub fn shared_clusters(&self) -> usize {
        let mut m1 = HashSet::new();
        for node in &self.tree1.nodes {
            if node.cluster_mask != BigUint::from(0u32) {
                m1.insert(node.cluster_mask.clone());
            }
        }
        let mut count = 0;
        for node in &self.tree2.nodes {
            if node.cluster_mask != BigUint::from(0u32) && m1.contains(&node.cluster_mask) {
                count += 1;
            }
        }
        count
    }
}

pub fn normalize_state(mut state: State) -> State {
    loop {
        let mut c1 = HashSet::new();
        let mut c2 = HashSet::new();
        collect_cherries(&state.tree1, &mut c1);
        collect_cherries(&state.tree2, &mut c2);
        
        let common: Vec<_> = c1.intersection(&c2).collect();
        if common.is_empty() { break; }
        
        let (a, b) = **common.iter().min().unwrap();
        let new_id = state.next_id;
        state.next_id += 1;
        
        let exp_a = state.expansions.get(&a).unwrap().clone();
        let exp_b = state.expansions.get(&b).unwrap().clone();
        state.expansions.insert(new_id, Expansion::Node(Box::new(exp_a), Box::new(exp_b)));
        
        // Arena contraction is simplified: replace cherry with new leaf in-place or rebuild
        // For correctness in this iteration, we use a simple approach:
        contract_arena(&mut state.tree1, a, b, new_id);
        contract_arena(&mut state.tree2, a, b, new_id);
    }
    state
}

fn contract_arena(tree: &mut ArenaTree, a: u32, b: u32, new_id: u32) {
    let mut a_idx = 0;
    let mut b_idx = 0;
    for (i, n) in tree.nodes.iter().enumerate() {
        if n.is_leaf && n.leaf_id == a { a_idx = i; }
        if n.is_leaf && n.leaf_id == b { b_idx = i; }
    }
    if a_idx == 0 || b_idx == 0 { return; }
    
    let p_idx = tree.nodes[a_idx].parent;
    if p_idx == 0 || tree.nodes[b_idx].parent != p_idx { return; }
    
    // Replace parent with a new leaf
    tree.nodes[p_idx].is_leaf = true;
    tree.nodes[p_idx].leaf_id = new_id;
    tree.nodes[p_idx].left = 0;
    tree.nodes[p_idx].right = 0;
    tree.nodes[p_idx].cluster_mask = BigUint::from(1u32) << (new_id - 1);
    tree.nodes[p_idx].size = 1;
    
    // Update sizes and masks up to root
    let mut curr = tree.nodes[p_idx].parent;
    while curr != 0 {
        let l = tree.nodes[curr].left;
        let r = tree.nodes[curr].right;
        tree.nodes[curr].cluster_mask = &tree.nodes[l].cluster_mask | &tree.nodes[r].cluster_mask;
        tree.nodes[curr].size = tree.nodes[l].size + tree.nodes[r].size;
        curr = tree.nodes[curr].parent;
    }
}
