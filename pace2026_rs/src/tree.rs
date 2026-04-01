use std::collections::{HashSet, HashMap};
use std::sync::Arc;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct FastBitSet {
    pub words: Vec<u64>,
}

impl FastBitSet {
    pub fn new(n_leaves: u32) -> Self {
        let size = ((n_leaves + 64) / 64) as usize;
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
    pub fn and(&self, other: &Self) -> Self {
        let mut res = self.words.clone();
        for i in 0..res.len().min(other.words.len()) { res[i] &= other.words[i]; }
        Self { words: res }
    }
    pub fn and_not(&self, other: &Self) -> Self {
        let mut res = self.words.clone();
        for i in 0..res.len().min(other.words.len()) { res[i] &= !other.words[i]; }
        Self { words: res }
    }
    pub fn count_ones(&self) -> u32 {
        self.words.iter().map(|w| w.count_ones()).sum()
    }
    pub fn get_set_bits(&self) -> Vec<u32> {
        let mut res = Vec::new();
        for (i, &w) in self.words.iter().enumerate() {
            let mut w = w;
            while w != 0 {
                let tz = w.trailing_zeros();
                res.push((i * 64) as u32 + tz);
                w &= w - 1;
            }
        }
        res
    }
    pub fn clear(&mut self, bit: u32) {
        let idx = (bit / 64) as usize;
        if idx < self.words.len() { self.words[idx] &= !(1 << (bit % 64)); }
    }
    pub fn intersects(&self, other: &Self) -> bool {
        for i in 0..self.words.len().min(other.words.len()) {
            if (self.words[i] & other.words[i]) != 0 { return true; }
        }
        false
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

#[derive(Clone, Debug, Eq, Hash)]
pub enum Expansion {
    Leaf(u32),
    Node(Arc<Expansion>, Arc<Expansion>, u64),
}

impl PartialEq for Expansion {
    fn eq(&self, other: &Self) -> bool {
        if self.hash_val() != other.hash_val() { return false; }
        match (self, other) {
            (Expansion::Leaf(a), Expansion::Leaf(b)) => a == b,
            (Expansion::Node(l1, r1, _), Expansion::Node(l2, r2, _)) => l1 == l2 && r1 == r2,
            _ => false,
        }
    }
}

impl Expansion {
    pub fn hash_val(&self) -> u64 {
        match self { 
            Expansion::Leaf(id) => (*id as u64).wrapping_mul(0x9E3779B97F4A7C15), 
            Expansion::Node(_, _, h) => *h 
        }
    }
    pub fn new_node(l: Expansion, r: Expansion) -> Self {
        let h1 = l.hash_val();
        let h2 = r.hash_val();
        let (min_h, max_h) = if h1 < h2 { (h1, h2) } else { (h2, h1) };
        let hfinal = min_h.wrapping_mul(6364136223846793005)
            .wrapping_add(max_h.wrapping_mul(1442695040888963407))
            .wrapping_add(0x1234567890ABCDEF);
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

pub fn get_hash_map(tree: &Arc<Tree>, map: &mut HashMap<u64, Arc<Tree>>) -> u64 {
    match tree.as_ref() {
        Tree::Leaf(id, _) => {
            let h = (*id as u64).wrapping_mul(0x9E3779B97F4A7C15);
            map.insert(h, tree.clone());
            h
        },
        Tree::Node(l, r, _, _) => {
            let h1 = get_hash_map(l, map);
            let h2 = get_hash_map(r, map);
            let (min_h, max_h) = if h1 < h2 { (h1, h2) } else { (h2, h1) };
            let hfinal = min_h.wrapping_mul(6364136223846793005)
                .wrapping_add(max_h.wrapping_mul(1442695040888963407))
                .wrapping_add(0x1234567890ABCDEF);
            map.insert(hfinal, tree.clone());
            hfinal
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(id: u32, n: u32) -> Arc<Tree> {
        let mut m = FastBitSet::new(n * 3); m.set(id);
        Arc::new(Tree::Leaf(id, Arc::new(m)))
    }

    fn node(l: Arc<Tree>, r: Arc<Tree>) -> Arc<Tree> {
        let m = l.mask().or(r.mask());
        let s = l.size() + r.size();
        Arc::new(Tree::Node(l, r, Arc::new(m), s))
    }

    #[test]
    fn test_mask_integrity() {
        let l1 = leaf(1, 10);
        let l2 = leaf(2, 10);
        let n1 = node(l1, l2);
        assert!(n1.mask().get(1));
        assert!(n1.mask().get(2));
        assert!(!n1.mask().get(3));
    }

    #[test]
    fn test_deterministic_hashing() {
        let e1 = Expansion::new_node(Expansion::Leaf(1), Expansion::Leaf(2));
        let e2 = Expansion::new_node(Expansion::Leaf(2), Expansion::Leaf(1));
        assert_eq!(e1.hash_val(), e2.hash_val());
    }
}
