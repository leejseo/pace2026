use std::collections::{HashMap, HashSet};
use crate::tree::{Tree, Expansion, collect_cherries, contract_cherry, get_cluster_masks};
use num_bigint::BigUint;

#[derive(Clone, Debug)]
pub struct State {
    pub tree1: Tree,
    pub tree2: Tree,
    pub expansions: HashMap<u32, Expansion>,
    pub next_id: u32,
    pub cut_components: Vec<Expansion>,
}

impl State {
    pub fn leaf_count(&self) -> usize {
        self.tree1.size()
    }

    pub fn shared_clusters(&self) -> usize {
        let mut m1 = HashSet::new();
        get_cluster_masks(&self.tree1, &mut m1);
        let mut m2 = HashSet::new();
        get_cluster_masks(&self.tree2, &mut m2);
        m1.intersection(&m2).count()
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
        
        let (a, b) = **common.iter().min().unwrap();
        let new_id = state.next_id;
        state.next_id += 1;
        
        let exp_a = state.expansions.get(&a).unwrap().clone();
        let exp_b = state.expansions.get(&b).unwrap().clone();
        state.expansions.insert(new_id, Expansion::Node(Box::new(exp_a), Box::new(exp_b)));
        
        state.tree1 = contract_cherry(&state.tree1, a, b, new_id);
        state.tree2 = contract_cherry(&state.tree2, a, b, new_id);
    }
    state
}
