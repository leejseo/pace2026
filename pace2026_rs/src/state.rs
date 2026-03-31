use std::collections::{HashMap, HashSet};
use crate::tree::{Tree, Expansion, collect_cherries, contract_cherry, get_cluster_masks, FastBitSet};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct State {
    pub tree1: Arc<Tree>,
    pub tree2: Arc<Tree>,
    pub expansions: HashMap<u32, Expansion>,
    pub next_id: u32,
    pub cut_components: Vec<Expansion>,
    pub cached_score: (usize, isize, usize),
}

impl State {
    pub fn leaf_count(&self) -> usize { self.tree1.size() }

    pub fn compute_score(&self) -> (usize, isize, usize) {
        let mut m1 = HashSet::new();
        get_cluster_masks(&self.tree1, &mut m1);
        let mut m2 = HashSet::new();
        get_cluster_masks(&self.tree2, &mut m2);
        let sc = m1.intersection(&m2).count();
        (self.cut_components.len() + self.leaf_count(), -(sc as isize), self.leaf_count())
    }
}

pub fn normalize_state(mut state: State) -> State {
    loop {
        let mut c1 = HashSet::new();
        collect_cherries(&state.tree1, &mut c1);
        let mut c2 = HashSet::new();
        collect_cherries(&state.tree2, &mut c2);
        
        let common: Vec<_> = c1.intersection(&c2).collect();
        if common.is_empty() { break; }
        
        let mut sorted_common = common;
        sorted_common.sort();
        let &(a, b) = sorted_common[0];
        
        let new_id = state.next_id;
        state.next_id += 1;
        
        let exp_a = state.expansions.get(&a).cloned().unwrap_or(Expansion::Leaf(a));
        let exp_b = state.expansions.get(&b).cloned().unwrap_or(Expansion::Leaf(b));
        state.expansions.insert(new_id, Expansion::Node(Box::new(exp_a), Box::new(exp_b)));
        
        state.tree1 = contract_cherry(&state.tree1, a, b, new_id);
        state.tree2 = contract_cherry(&state.tree2, a, b, new_id);
    }
    state.cached_score = state.compute_score();
    state
}
