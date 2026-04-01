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
        let idx = (bit / 64) as usize;
        if idx < self.words.len() { self.words[idx] |= 1 << (bit % 64); }
    }
    pub fn get(&self, bit: u32) -> bool {
        let idx = (bit / 64) as usize;
        if idx < self.words.len() { (self.words[idx] & (1 << (bit % 64))) != 0 } else { false }
    }
    pub fn or(&self, other: &Self) -> Self {
        let mut res = self.words.clone();
        for i in 0..res.len().min(other.words.len()) { res[i] |= other.words[i]; }
        Self { words: res }
    }
}

#[derive(Clone, Debug)]
pub struct OriginalNode {
    pub left: Option<Box<OriginalNode>>,
    pub right: Option<Box<OriginalNode>>,
    pub label: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum Expansion {
    Leaf(u32),
    Node(Arc<Expansion>, Arc<Expansion>, u64),
}

impl Expansion {
    pub fn hash_val(&self) -> u64 {
        match self { Expansion::Leaf(id) => *id as u64, Expansion::Node(_, _, h) => *h }
    }
    pub fn new_node(l: Expansion, r: Expansion) -> Self {
        let h1 = l.hash_val();
        let h2 = r.hash_val();
        let (min_h, max_h) = if h1 < h2 { (h1, h2) } else { (h2, h1) };
        // Truly deterministic hash: combine min and max in a fixed order
        let hfinal = min_h.wrapping_mul(6364136223846793005)
            .wrapping_add(max_h)
            .wrapping_add(1442695040888963407);
        if h1 < h2 { Expansion::Node(Arc::new(l), Arc::new(r), hfinal) }
        else { Expansion::Node(Arc::new(r), Arc::new(l), hfinal) }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Tree {
    Leaf(u32, Arc<FastBitSet>),
    Node(Arc<Tree>, Arc<Tree>, Arc<FastBitSet>, usize),
}

impl Tree {
    pub fn mask(&self) -> &Arc<FastBitSet> { match self { Tree::Leaf(_, m) | Tree::Node(_, _, m, _) => m } }
    pub fn size(&self) -> usize { match self { Tree::Leaf(_, _) => 1, Tree::Node(_, _, _, s) => *s } }
    pub fn is_leaf(&self) -> bool { matches!(self, Tree::Leaf(_, _)) }
    pub fn leaf_id(&self) -> u32 { match self { Tree::Leaf(id, _) => *id, _ => 0 } }
}

pub fn original_to_tree(node: &OriginalNode, n_leaves: u32) -> Arc<Tree> {
    let bitset_size = 3 * n_leaves;
    fn build(node: &OriginalNode, bitset_size: u32) -> Arc<Tree> {
        if let Some(id) = node.label {
            let mut m = FastBitSet::new(bitset_size); m.set(id);
            Arc::new(Tree::Leaf(id, Arc::new(m)))
        } else {
            let l = build(node.left.as_ref().unwrap(), bitset_size);
            let r = build(node.right.as_ref().unwrap(), bitset_size);
            let m = l.mask().or(r.mask());
            let s = l.size() + r.size();
            Arc::new(Tree::Node(l, r, Arc::new(m), s))
        }
    }
    build(node, bitset_size)
}

pub fn cut_leaf(tree: &Arc<Tree>, leaf_id: u32) -> Option<Arc<Tree>> {
    match tree.as_ref() {
        Tree::Leaf(id, _) => if *id == leaf_id { None } else { Some(tree.clone()) },
        Tree::Node(l, r, _, _) => {
            let nl = cut_leaf(l, leaf_id);
            let nr = cut_leaf(r, leaf_id);
            match (nl, nr) {
                (None, None) => None,
                (Some(t), None) | (None, Some(t)) => Some(t),
                (Some(tl), Some(tr)) => {
                    if Arc::ptr_eq(&tl, l) && Arc::ptr_eq(&tr, r) { return Some(tree.clone()); }
                    let m = tl.mask().or(tr.mask());
                    let sz = tl.size() + tr.size();
                    Some(Arc::new(Tree::Node(tl, tr, Arc::new(m), sz)))
                }
            }
        }
    }
}

pub fn path_to_leaf(tree: &Arc<Tree>, target_leaf: u32) -> Vec<(Arc<Tree>, usize)> {
    let mut path = Vec::new();
    let mut curr = tree.clone();
    while let Tree::Node(l, r, _, _) = curr.as_ref() {
        if l.mask().get(target_leaf) {
            path.push((curr.clone(), 0)); curr = l.clone();
        } else if r.mask().get(target_leaf) {
            path.push((curr.clone(), 1)); curr = r.clone();
        } else { break; }
    }
    path
}

pub fn offpath_candidates(tree: &Arc<Tree>, a: u32, b: u32) -> Vec<Arc<Tree>> {
    let path_a = path_to_leaf(tree, a);
    let path_b = path_to_leaf(tree, b);
    let mut lca_idx = 0;
    while lca_idx + 1 < path_a.len() && lca_idx + 1 < path_b.len() && Arc::ptr_eq(&path_a[lca_idx + 1].0, &path_b[lca_idx + 1].0) {
        lca_idx += 1;
    }
    let mut candidates = Vec::new();
    for i in lca_idx..path_a.len() {
        if let Tree::Node(l, r, _, _) = path_a[i].0.as_ref() {
            let side = path_a[i].1;
            candidates.push(if side == 0 { r.clone() } else { l.clone() });
        }
    }
    for i in lca_idx..path_b.len() {
        if let Tree::Node(l, r, _, _) = path_b[i].0.as_ref() {
            let side = path_b[i].1;
            candidates.push(if side == 0 { r.clone() } else { l.clone() });
        }
    }
    candidates
}

pub fn get_all_leaves(tree: &Arc<Tree>) -> Vec<u32> {
    let mut leaves = Vec::new();
    let mut stack = vec![tree.clone()];
    while let Some(node) = stack.pop() {
        match node.as_ref() {
            Tree::Leaf(id, _) => leaves.push(*id),
            Tree::Node(l, r, _, _) => { stack.push(l.clone()); stack.push(r.clone()); }
        }
    }
    leaves
}
