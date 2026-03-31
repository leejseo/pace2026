use num_bigint::BigUint;
use num_traits::Zero;
use std::collections::{HashSet, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};
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

#[derive(Clone, Debug, Eq)]
pub enum Tree {
    Leaf(u32, BigUint, u64), // ID, Mask, MaskHash
    Node(Arc<Tree>, Arc<Tree>, BigUint, u64, usize), // L, R, Mask, MaskHash, Size
}

impl PartialEq for Tree {
    fn eq(&self, other: &Self) -> bool {
        self.mask_hash() == other.mask_hash() && self.mask() == other.mask()
    }
}

impl Hash for Tree {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.mask_hash().hash(state);
    }
}

impl Tree {
    pub fn mask(&self) -> &BigUint {
        match self { Tree::Leaf(_, m, _) | Tree::Node(_, _, m, _, _) => m }
    }
    pub fn mask_hash(&self) -> u64 {
        match self { Tree::Leaf(_, _, h) | Tree::Node(_, _, _, h, _) => *h }
    }
    pub fn size(&self) -> usize {
        match self { Tree::Leaf(_, _, _) => 1, Tree::Node(_, _, _, _, s) => *s }
    }
    pub fn is_leaf(&self) -> bool { matches!(self, Tree::Leaf(_, _, _)) }
    pub fn leaf_id(&self) -> u32 {
        match self { Tree::Leaf(id, _, _) => *id, _ => 0 }
    }
}

fn compute_hash(mask: &BigUint) -> u64 {
    let mut s = DefaultHasher::new();
    mask.hash(&mut s);
    s.finish()
}

pub fn original_to_tree(node: &OriginalNode) -> Arc<Tree> {
    if let Some(id) = node.label {
        let m = BigUint::from(1u32) << (id - 1);
        let h = compute_hash(&m);
        Arc::new(Tree::Leaf(id, m, h))
    } else {
        let l = original_to_tree(node.left.as_ref().unwrap());
        let r = original_to_tree(node.right.as_ref().unwrap());
        let m = l.mask() | r.mask();
        let h = compute_hash(&m);
        let s = l.size() + r.size();
        Arc::new(Tree::Node(l, r, m, h, s))
    }
}

pub fn collect_cherries(tree: &Arc<Tree>, cherries: &mut HashSet<(u32, u32)>) {
    let mut stack = vec![tree.clone()];
    while let Some(node) = stack.pop() {
        if let Tree::Node(l, r, _, _, _) = node.as_ref() {
            if l.is_leaf() && r.is_leaf() {
                let a = l.leaf_id(); let b = r.leaf_id();
                cherries.insert(if a < b { (a, b) } else { (b, a) });
            }
            stack.push(l.clone());
            stack.push(r.clone());
        }
    }
}

pub fn cut_leaf(tree: &Arc<Tree>, leaf_id: u32) -> Option<Arc<Tree>> {
    let target_mask = BigUint::from(1u32) << (leaf_id - 1);
    if (tree.mask() & &target_mask).is_zero() { return Some(tree.clone()); }
    
    match tree.as_ref() {
        Tree::Leaf(id, _, _) => if *id == leaf_id { None } else { Some(tree.clone()) },
        Tree::Node(l, r, _, _, _) => {
            let nl = cut_leaf(l, leaf_id);
            let nr = cut_leaf(r, leaf_id);
            match (nl, nr) {
                (None, None) => None,
                (Some(t), None) | (None, Some(t)) => Some(t),
                (Some(tl), Some(tr)) => {
                    let m = tl.mask() | tr.mask();
                    let h = compute_hash(&m);
                    let s = tl.size() + tr.size();
                    Some(Arc::new(Tree::Node(tl, tr, m, h, s)))
                }
            }
        }
    }
}

pub fn contract_cherry(tree: &Arc<Tree>, a: u32, b: u32, new_id: u32) -> Arc<Tree> {
    let target_mask = (BigUint::from(1u32) << (a - 1)) | (BigUint::from(1u32) << (b - 1));
    if (tree.mask() & &target_mask).is_zero() { return tree.clone(); }
    
    match tree.as_ref() {
        Tree::Leaf(_, _, _) => tree.clone(),
        Tree::Node(l, r, _, _, _) => {
            if l.is_leaf() && r.is_leaf() {
                let id_l = l.leaf_id(); let id_r = r.leaf_id();
                if (id_l == a && id_r == b) || (id_l == b && id_r == a) {
                    let m = BigUint::from(1u32) << (new_id - 1);
                    let h = compute_hash(&m);
                    return Arc::new(Tree::Leaf(new_id, m, h));
                }
            }
            let nl = contract_cherry(l, a, b, new_id);
            let nr = contract_cherry(r, a, b, new_id);
            if Arc::ptr_eq(&nl, l) && Arc::ptr_eq(&nr, r) { return tree.clone(); }
            let m = nl.mask() | nr.mask();
            let h = compute_hash(&m);
            let s = nl.size() + nr.size();
            Arc::new(Tree::Node(nl, nr, m, h, s))
        }
    }
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
            stack.push(l.clone());
            stack.push(r.clone());
        }
    }
}
