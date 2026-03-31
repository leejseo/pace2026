use std::collections::{HashMap, HashSet};
use crate::tree::{Tree, Expansion, collect_cherries, cut_leaf, get_cluster_masks};
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
        let mut m1 = HashMap::new();
        collect_all_clusters(&state.tree1, &mut m1);
        let mut m2 = HashMap::new();
        collect_all_clusters(&state.tree2, &mut m2);
        
        let mut common = Vec::new();
        for (mask, node) in m1.iter() {
            if node.size() > 1 && m2.contains_key(mask) {
                // Ensure topologies are also isomorphic
                if are_topologies_same(node, m2.get(mask).unwrap(), &state.expansions) {
                    common.push((mask.clone(), node.size()));
                }
            }
        }
        
        if common.is_empty() { break; }
        
        common.sort_by_key(|x| x.1); // Smallest clusters first
        let (target_mask, _) = &common[0];
        let target_node = m1.get(target_mask).unwrap();
        
        let new_id = state.next_id;
        state.next_id += 1;
        
        let cluster_exp = build_cluster_expansion(target_node, &state.expansions);
        state.expansions.insert(new_id, cluster_exp);
        
        state.tree1 = contract_cluster(&state.tree1, target_mask, new_id);
        state.tree2 = contract_cluster(&state.tree2, target_mask, new_id);
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

fn are_topologies_same(n1: &Arc<Tree>, n2: &Arc<Tree>, _exps: &HashMap<u32, Expansion>) -> bool {
    if n1.is_leaf() && n2.is_leaf() { return n1.leaf_id() == n2.leaf_id(); }
    if n1.is_leaf() || n2.is_leaf() { return false; }
    if let (Tree::Node(l1, r1, _, _), Tree::Node(l2, r2, _, _)) = (n1.as_ref(), n2.as_ref()) {
        let case1 = (l1.mask() == l2.mask() && are_topologies_same(l1, l2, _exps)) &&
                    (r1.mask() == r2.mask() && are_topologies_same(r1, r2, _exps));
        if case1 { return true; }
        let case2 = (l1.mask() == r2.mask() && are_topologies_same(l1, r2, _exps)) &&
                    (r1.mask() == l2.mask() && are_topologies_same(r1, l2, _exps));
        return case2;
    }
    false
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

fn contract_cluster(tree: &Arc<Tree>, target_mask: &BigUint, new_id: u32) -> Arc<Tree> {
    if tree.mask() == target_mask {
        return Arc::new(Tree::Leaf(new_id, BigUint::from(1u32) << (new_id - 1)));
    }
    match tree.as_ref() {
        Tree::Leaf(_, _) => tree.clone(),
        Tree::Node(l, r, _, _) => {
            let nl = contract_cluster(l, target_mask, new_id);
            let nr = contract_cluster(r, target_mask, new_id);
            if Arc::ptr_eq(&nl, l) && Arc::ptr_eq(&nr, r) { return tree.clone(); }
            let nm = nl.mask() | nr.mask();
            let ns = nl.size() + nr.size();
            Arc::new(Tree::Node(nl, nr, nm, ns))
        }
    }
}
