use num_bigint::BigUint;
use num_traits::Zero;
use std::collections::{HashSet, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct FastBitSet {
    pub words: Vec<u64>,
}

impl FastBitSet {
    pub fn new(n_leaves: u32) -> Self {
        let size = ((n_leaves + 63) / 64) as usize;
        Self { words: vec![0; size] }
    }
    pub fn set(&mut self, bit: u32) {
        if bit == 0 { return; }
        let bit = bit - 1;
        let idx = (bit / 64) as usize;
        if idx < self.words.len() { self.words[idx] |= 1 << (bit % 64); }
    }
    pub fn or(&self, other: &Self) -> Self {
        let mut res = self.words.clone();
        for i in 0..res.len().min(other.words.len()) { res[i] |= other.words[i]; }
        Self { words: res }
    }
    pub fn and_is_zero(&self, other: &Self) -> bool {
        for i in 0..self.words.len().min(other.words.len()) { if (self.words[i] & other.words[i]) != 0 { return false; } }
        true
    }
}

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

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Tree {
    Leaf(u32, FastBitSet, u64),
    Node(Arc<Tree>, Arc<Tree>, FastBitSet, u64, usize),
}

impl Tree {
    pub fn mask(&self) -> &FastBitSet { match self { Tree::Leaf(_, m, _) | Tree::Node(_, _, m, _, _) => m } }
    pub fn mask_hash(&self) -> u64 { match self { Tree::Leaf(_, _, h) | Tree::Node(_, _, _, h, _) => *h } }
    pub fn size(&self) -> usize { match self { Tree::Leaf(_, _, _) => 1, Tree::Node(_, _, _, _, s) => *s } }
    pub fn is_leaf(&self) -> bool { matches!(self, Tree::Leaf(_, _, _)) }
    pub fn leaf_id(&self) -> u32 { match self { Tree::Leaf(id, _, _) => *id, _ => 0 } }
}

fn compute_mask_hash(m: &FastBitSet) -> u64 {
    let mut s = DefaultHasher::new();
    m.words.hash(&mut s);
    s.finish()
}

pub fn original_to_tree(node: &OriginalNode, n_leaves: u32) -> Arc<Tree> {
    if let Some(id) = node.label {
        let mut m = FastBitSet::new(n_leaves + 2000); m.set(id);
        let h = compute_mask_hash(&m);
        Arc::new(Tree::Leaf(id, m, h))
    } else {
        let l = original_to_tree(node.left.as_ref().unwrap(), n_leaves);
        let r = original_to_tree(node.right.as_ref().unwrap(), n_leaves);
        let m = l.mask().or(r.mask());
        let h = compute_mask_hash(&m);
        let s = l.size() + r.size();
        Arc::new(Tree::Node(l, r, m, h, s))
    }
}

pub fn collect_cherries(tree: &Arc<Tree>, cherries: &mut HashSet<(u32, u32)>) {
    let mut stack = vec![tree.clone()];
    while let Some(node) = stack.pop() {
        if let Tree::Node(l, r, _, _, _) = node.as_ref() {
            if l.is_leaf() && r.is_leaf() {
                let (a, b) = (l.leaf_id(), r.leaf_id());
                cherries.insert(if a < b { (a, b) } else { (b, a) });
            }
            stack.push(l.clone()); stack.push(r.clone());
        }
    }
}

pub fn cut_leaf(tree: &Arc<Tree>, leaf_id: u32) -> Option<Arc<Tree>> {
    let mut target = FastBitSet::new(tree.mask().words.len() as u32 * 64); target.set(leaf_id);
    if tree.mask().and_is_zero(&target) { return Some(tree.clone()); }
    match tree.as_ref() {
        Tree::Leaf(id, _, _) => if *id == leaf_id { None } else { Some(tree.clone()) },
        Tree::Node(l, r, _, _, _) => {
            let nl = cut_leaf(l, leaf_id);
            let nr = cut_leaf(r, leaf_id);
            match (nl, nr) {
                (None, None) => None,
                (Some(t), None) | (None, Some(t)) => Some(t),
                (Some(tl), Some(tr)) => {
                    let m = tl.mask().or(tr.mask());
                    let h = compute_mask_hash(&m);
                    let s = tl.size() + tr.size();
                    Some(Arc::new(Tree::Node(tl, tr, m, h, s)))
                }
            }
        }
    }
}

pub fn contract_cherry(tree: &Arc<Tree>, a: u32, b: u32, new_id: u32) -> Arc<Tree> {
    let mut target = FastBitSet::new(tree.mask().words.len() as u32 * 64); target.set(a); target.set(b);
    if tree.mask().and_is_zero(&target) { return tree.clone(); }
    match tree.as_ref() {
        Tree::Leaf(_, _, _) => tree.clone(),
        Tree::Node(l, r, _, _, _) => {
            if l.is_leaf() && r.is_leaf() {
                let (id_l, id_r) = (l.leaf_id(), r.leaf_id());
                if (id_l == a && id_r == b) || (id_l == b && id_r == a) {
                    let mut m = FastBitSet::new(tree.mask().words.len() as u32 * 64); m.set(new_id);
                    let h = compute_mask_hash(&m);
                    return Arc::new(Tree::Leaf(new_id, m, h));
                }
            }
            let nl = contract_cherry(l, a, b, new_id);
            let nr = contract_cherry(r, a, b, new_id);
            if Arc::ptr_eq(&nl, l) && Arc::ptr_eq(&nr, r) { return tree.clone(); }
            let m = nl.mask().or(nr.mask());
            let h = compute_mask_hash(&m);
            let s = nl.size() + nr.size();
            Arc::new(Tree::Node(nl, nr, m, h, s))
        }
    }
}

pub fn path_to_leaf(tree: &Arc<Tree>, target_leaf: u32) -> Vec<(Arc<Tree>, usize)> {
    let mut path = Vec::new();
    let mut curr = tree.clone();
    let mut target = FastBitSet::new(tree.mask().words.len() as u32 * 64); target.set(target_leaf);
    while let Tree::Node(l, r, _, _, _) = curr.as_ref() {
        if !l.mask().and_is_zero(&target) {
            path.push((curr.clone(), 0)); curr = l.clone();
        } else if !r.mask().and_is_zero(&target) {
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
        if let Tree::Node(l, r, _, _, _) = p.as_ref() {
            candidates.push(if *side == 0 { r.clone() } else { l.clone() });
        }
    }
    for (p, side) in path_b.iter().skip(i) {
        if let Tree::Node(l, r, _, _, _) = p.as_ref() {
            candidates.push(if *side == 0 { r.clone() } else { l.clone() });
        }
    }
    candidates
}

pub fn get_all_leaves(tree: &Arc<Tree>) -> Vec<u32> {
    let mut leaves = Vec::new();
    let mut stack = vec![tree.clone()];
    while let Some(node) = stack.pop() {
        match node.as_ref() {
            Tree::Leaf(id, _, _) => leaves.push(*id),
            Tree::Node(l, r, _, _, _) => { stack.push(l.clone()); stack.push(r.clone()); }
        }
    }
    leaves
}

pub fn get_cluster_masks(tree: &Arc<Tree>, masks: &mut HashSet<u64>) {
    let mut stack = vec![tree.clone()];
    while let Some(node) = stack.pop() {
        masks.insert(node.mask_hash());
        if let Tree::Node(l, r, _, _, _) = node.as_ref() {
            stack.push(l.clone()); stack.push(r.clone());
        }
    }
}
