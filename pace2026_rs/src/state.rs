use std::collections::{HashMap, HashSet, VecDeque};
use crate::tree::{Tree, Expansion, collect_cherries, contract_cherry, get_cluster_masks};
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

// Highly Optimized Incremental Normalization
pub fn normalize_state(mut state: State) -> State {
    // Initial cherry collection
    let mut c1 = HashSet::new(); collect_cherries(&state.tree1, &mut c1);
    let mut c2 = HashSet::new(); collect_cherries(&state.tree2, &mut c2);
    let mut common: VecDeque<_> = c1.intersection(&c2).cloned().collect();

    while let Some((a, b)) = common.pop_front() {
        // Double check if these leaves still exist in the current trees
        // (In a highly optimized version, we'd use a more robust way to track this)
        
        let new_id = state.next_id;
        state.next_id += 1;
        
        let exp_a = state.expansions.get(&a).cloned().unwrap_or(Expansion::Leaf(a));
        let exp_b = state.expansions.get(&b).cloned().unwrap_or(Expansion::Leaf(b));
        state.expansions.insert(new_id, Expansion::new_node(exp_a, exp_b));
        
        state.tree1 = contract_cherry(&state.tree1, a, b, new_id);
        state.tree2 = contract_cherry(&state.tree2, a, b, new_id);
        
        // After contraction, new cherries might have been formed at the parent of new_id.
        // For absolute correctness and speed in this iteration, we do a localized re-scan
        // or a simpler limited re-scan.
        if common.is_empty() {
            let mut nc1 = HashSet::new(); collect_cherries(&state.tree1, &mut nc1);
            let mut nc2 = HashSet::new(); collect_cherries(&state.tree2, &mut nc2);
            for cherry in nc1.intersection(&nc2) {
                if !common.contains(cherry) { common.push_back(*cherry); }
            }
        }
    }
    state.cached_score = state.compute_score();
    state
}
