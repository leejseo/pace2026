use std::collections::{HashMap, HashSet};
use crate::tree::{Tree, Expansion, collect_cherries, contract_cherry, get_cluster_masks};
use std::sync::Arc;
use num_bigint::BigUint;

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
        // 1. Collect all cluster masks from both trees
        let mut m1 = HashMap::new();
        collect_all_clusters(&state.tree1, &mut m1);
        let mut m2 = HashMap::new();
        collect_all_clusters(&state.tree2, &mut m2);
        
        // 2. Find shared clusters that are not single leaves
        let mut common_clusters = Vec::new();
        for (mask, node) in m1.iter() {
            if node.size() > 1 && m2.contains_key(mask) {
                common_clusters.push((mask.clone(), node.size()));
            }
        }
        
        if common_clusters.is_empty() { break; }
        
        // 3. Pick the SMALLEST shared cluster to contract first (Bottom-up)
        common_clusters.sort_by_key(|x| x.1);
        let (target_mask, _) = &common_clusters[0];
        
        // Get all leaf IDs in this cluster
        let target_node = m1.get(target_mask).unwrap();
        let leaf_ids = crate::tree::get_all_leaves(target_node);
        
        let new_id = state.next_id;
        state.next_id += 1;
        
        // Build the expansion for this cluster root
        let cluster_exp = build_cluster_expansion(target_node, &state.expansions);
        state.expansions.insert(new_id, cluster_exp);
        
        // Contract the cluster in both trees
        state.tree1 = contract_general_cluster(&state.tree1, target_mask, new_id);
        state.tree2 = contract_general_cluster(&state.tree2, target_mask, new_id);
    }
    state.cached_score = state.compute_score();
    state
}

fn collect_all_clusters(tree: &Arc<Tree>, map: &mut HashMap<BigUint, Arc<Tree>>) {
    map.insert(tree.mask().clone(), tree.clone());
    if let Tree::Node(l, r, _, _) = tree.as_ref() {
        collect_all_clusters(l, map);
        collect_all_clusters(r, map);
    }
}

fn build_cluster_expansion(node: &Arc<Tree>, expansions: &HashMap<u32, Expansion>) -> Expansion {
    match node.as_ref() {
        Tree::Leaf(id, _) => expansions.get(id).cloned().unwrap_or(Expansion::Leaf(*id)),
        Tree::Node(l, r, _, _) => {
            Expansion::Node(Box::new(build_cluster_expansion(l, expansions)), 
                            Box::new(build_cluster_expansion(r, expansions)))
        }
    }
}

fn contract_general_cluster(tree: &Arc<Tree>, target_mask: &BigUint, new_id: u32) -> Arc<Tree> {
    if tree.mask() == target_mask {
        return Arc::new(Tree::Leaf(new_id, BigUint::from(1u32) << (new_id - 1)));
    }
    match tree.as_ref() {
        Tree::Leaf(_, _) => tree.clone(),
        Tree::Node(l, r, m, s) => {
            let nl = contract_general_cluster(l, target_mask, new_id);
            let nr = contract_general_cluster(r, target_mask, new_id);
            if Arc::ptr_eq(&nl, l) && Arc::ptr_eq(&nr, r) { return tree.clone(); }
            let nm = nl.mask() | nr.mask();
            let ns = nl.size() + nr.size();
            Arc::new(Tree::Node(nl, nr, nm, ns))
        }
    }
}
