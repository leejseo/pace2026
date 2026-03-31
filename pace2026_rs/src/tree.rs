use num_bigint::BigUint;
use num_traits::Zero;
use std::collections::HashSet;
use std::sync::Arc;

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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Tree {
    Leaf(u32, BigUint),
    Node(Arc<Tree>, Arc<Tree>, BigUint, usize),
}

impl Tree {
    pub fn mask(&self) -> &BigUint {
        match self { Tree::Leaf(_, m) | Tree::Node(_, _, m, _) => m }
    }
    pub fn size(&self) -> usize {
        match self { Tree::Leaf(_, _) => 1, Tree::Node(_, _, _, s) => *s }
    }
    pub fn is_leaf(&self) -> bool { matches!(self, Tree::Leaf(_, _)) }
    pub fn leaf_id(&self) -> u32 {
        match self { Tree::Leaf(id, _) => *id, _ => 0 }
    }
}

pub fn original_to_tree(node: &OriginalNode) -> Arc<Tree> {
    if let Some(id) = node.label {
        Arc::new(Tree::Leaf(id, BigUint::from(1u32) << (id - 1)))
    } else {
        let l = original_to_tree(node.left.as_ref().unwrap());
        let r = original_to_tree(node.right.as_ref().unwrap());
        let m = l.mask() | r.mask();
        let s = l.size() + r.size();
        Arc::new(Tree::Node(l, r, m, s))
    }
}

pub fn collect_cherries(tree: &Arc<Tree>, cherries: &mut HashSet<(u32, u32)>) {
    if let Tree::Node(l, r, _, _) = tree.as_ref() {
        if l.is_leaf() && r.is_leaf() {
            let a = l.leaf_id(); let b = r.leaf_id();
            cherries.insert(if a < b { (a, b) } else { (b, a) });
        }
        collect_cherries(l, cherries);
        collect_cherries(r, cherries);
    }
}

pub fn cut_leaf(tree: &Arc<Tree>, leaf_id: u32) -> Option<Arc<Tree>> {
    let target_mask = BigUint::from(1u32) << (leaf_id - 1);
    if (tree.mask() & &target_mask).is_zero() { return Some(tree.clone()); }
    
    match tree.as_ref() {
        Tree::Leaf(id, _) => if *id == leaf_id { None } else { Some(tree.clone()) },
        Tree::Node(l, r, _, _) => {
            let nl = cut_leaf(l, leaf_id);
            let nr = cut_leaf(r, leaf_id);
            match (nl, nr) {
                (None, None) => None,
                (Some(t), None) | (None, Some(t)) => Some(t),
                (Some(tl), Some(tr)) => {
                    let m = tl.mask() | tr.mask();
                    let s = tl.size() + tr.size();
                    Some(Arc::new(Tree::Node(tl, tr, m, s)))
                }
            }
        }
    }
}

pub fn path_to_leaf(tree: &Arc<Tree>, target_leaf: u32) -> Vec<(Arc<Tree>, usize)> {
    let mut path = Vec::new();
    let mut curr = tree.clone();
    let target_mask = BigUint::from(1u32) << (target_leaf - 1);
    while let Tree::Node(l, r, _, _) = curr.as_ref() {
        if !(l.mask() & &target_mask).is_zero() {
            path.push((curr.clone(), 0)); curr = l.clone();
        } else if !(r.mask() & &target_mask).is_zero() {
            path.push((curr.clone(), 1)); curr = r.clone();
        } else { break; }
    }
    path
}

pub fn offpath_candidates(tree: &Arc<Tree>, a: u32, b: u32) -> Vec<Arc<Tree>> {
    let path_a = path_to_leaf(tree, a);
    let path_b = path_to_leaf(tree, b);
    let mut i = 0;
    while i < path_a.len() && i < path_b.len() {
        if Arc::ptr_eq(&path_a[i].0, &path_b[i].0) && path_a[i].1 == path_b[i].1 { i += 1; }
        else { break; }
    }
    let mut candidates = Vec::new();
    for (p, side) in path_a.iter().skip(i) {
        if let Tree::Node(l, r, _, _) = p.as_ref() {
            candidates.push(if *side == 0 { r.clone() } else { l.clone() });
        }
    }
    for (p, side) in path_b.iter().skip(i) {
        if let Tree::Node(l, r, _, _) = p.as_ref() {
            candidates.push(if *side == 0 { r.clone() } else { l.clone() });
        }
    }
    candidates
}

pub fn get_all_leaves(tree: &Arc<Tree>) -> Vec<u32> {
    let mut leaves = Vec::new();
    let mut mask = tree.mask().clone();
    let mut idx = 1;
    while !mask.is_zero() {
        if (&mask & BigUint::from(1u32)) == BigUint::from(1u32) { leaves.push(idx); }
        mask >>= 1; idx += 1;
    }
    leaves
}

pub fn get_cluster_masks(tree: &Arc<Tree>, masks: &mut HashSet<BigUint>) {
    masks.insert(tree.mask().clone());
    if let Tree::Node(l, r, _, _) = tree.as_ref() {
        get_cluster_masks(l, masks);
        get_cluster_masks(r, masks);
    }
}
