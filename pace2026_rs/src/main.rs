mod tree;
mod state;
mod io;

use std::collections::{HashMap, HashSet};
use crate::tree::{Tree, Expansion, original_to_tree, collect_cherries, cut_leaf, offpath_candidates, get_all_leaves, OriginalNode};
use crate::state::{State, normalize_state};
use crate::io::{parse_instance_file, render_expansion};
use anyhow::Result;
use std::time::{Instant, Duration};
use rayon::prelude::*;
use std::sync::{Arc, Mutex};
use num_traits::Zero;
use rand::prelude::*;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: pace2026_rs <input.nw> [--time-limit <s|300>]");
        std::process::exit(1);
    }
    let mut time_limit = 300;
    for i in 0..args.len() {
        if args[i] == "--time-limit" && i + 1 < args.len() { 
            time_limit = args[i+1].parse().unwrap_or(300);
        }
    }
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(move || { if let Err(e) = run(time_limit) { eprintln!("Error: {}", e); } })?
        .join()
        .expect("Thread failed");
    Ok(())
}

fn run(time_limit: u64) -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let instance = parse_instance_file(&args[1])?;
    let t1 = original_to_tree(&instance.tree1);
    let t2 = original_to_tree(&instance.tree2);
    let partition = solve_partition(t1, t2, instance.n_leaves, time_limit, &instance.tree1, &instance.tree2);
    for leaf_set in partition {
        if let Some(exp) = build_induced_expansion(&instance.tree1, &leaf_set) {
            println!("{};", render_expansion(&exp));
        }
    }
    Ok(())
}

fn are_isomorphic(n1: &Expansion, n2: &Expansion) -> bool {
    match (n1, n2) {
        (Expansion::Leaf(id1), Expansion::Leaf(id2)) => id1 == id2,
        (Expansion::Node(l1, r1), Expansion::Node(l2, r2)) => {
            (are_isomorphic(l1, l2) && are_isomorphic(r1, r2)) || 
            (are_isomorphic(l1, r2) && are_isomorphic(r1, l2))
        }
        _ => false,
    }
}

fn build_induced_expansion(node: &OriginalNode, leaves: &HashSet<u32>) -> Option<Expansion> {
    if let Some(id) = node.label {
        return if leaves.contains(&id) { Some(Expansion::Leaf(id)) } else { None };
    }
    let l = build_induced_expansion(node.left.as_ref().unwrap(), leaves);
    let r = build_induced_expansion(node.right.as_ref().unwrap(), leaves);
    match (l, r) {
        (Some(tl), Some(tr)) => Some(Expansion::Node(Box::new(tl), Box::new(tr))),
        (Some(t), None) | (None, Some(t)) => Some(t),
        (None, None) => None,
    }
}

fn get_conflict_candidates(state: &State) -> Vec<u32> {
    let mut c1 = HashSet::new(); collect_cherries(&state.tree1, &mut c1);
    let mut c2 = HashSet::new(); collect_cherries(&state.tree2, &mut c2);
    let mut candidates = HashSet::new();
    let mut rng = rand::rng();
    let diff1: Vec<_> = c1.difference(&c2).collect();
    let diff2: Vec<_> = c2.difference(&c1).collect();
    if let Some(&(a, b)) = diff1.choose(&mut rng).or(diff2.choose(&mut rng)) {
        candidates.insert(*a); candidates.insert(*b);
    }
    if candidates.is_empty() { return get_all_leaves(&state.tree1); }
    candidates.into_iter().collect()
}

fn solve_partition(tree1: Arc<Tree>, tree2: Arc<Tree>, n_leaves: u32, limit_seconds: u64, ot1: &OriginalNode, ot2: &OriginalNode) -> Vec<HashSet<u32>> {
    let start_time = Instant::now();
    let deadline = start_time + Duration::from_secs(limit_seconds);
    let mut expansions = HashMap::new();
    for i in 1..=n_leaves { expansions.insert(i, Expansion::Leaf(i)); }
    
    let start_state = normalize_state(State {
        tree1, tree2, expansions,
        next_id: n_leaves + 1, cut_components: Vec::new(), cached_score: (0, 0, 0),
    });

    let best_partition = Arc::new(Mutex::new(vec![]));
    let best_count = Arc::new(Mutex::new(n_leaves as usize + 1));

    (0..rayon::current_num_threads()).into_par_iter().for_each(|_| {
        let mut rng = rand::rng();
        while Instant::now() < deadline {
            let mut curr = start_state.clone();
            
            // Iterative Refinement: occasionally shake the current best
            let mut p = if rng.random_bool(0.4) {
                let current_best = best_partition.lock().unwrap().clone();
                if !current_best.is_empty() && current_best.len() > 3 {
                    // Start from a state that partially contains the best solution
                    let shake_count = rng.random_range(1..current_best.len() / 2 + 1);
                    let mut shaken = current_best;
                    shaken.shuffle(&mut rng);
                    // Combine the first shake_count partitions into a "remaining to solve" state
                    let mut combined_leaves = HashSet::new();
                    for s in shaken.drain(0..shake_count) { combined_leaves.extend(s); }
                    
                    // Filter start_state to only contain combined_leaves
                    let mut next_t1 = start_state.tree1.clone();
                    let mut next_t2 = start_state.tree2.clone();
                    for i in 1..=n_leaves {
                        if !combined_leaves.contains(&i) {
                            if let Some(t) = cut_leaf(&next_t1, i) { next_t1 = t; }
                            if let Some(t) = cut_leaf(&next_t2, i) { next_t2 = t; }
                        }
                    }
                    curr = normalize_state(State {
                        tree1: next_t1, tree2: next_t2, expansions: start_state.expansions.clone(),
                        next_id: start_state.next_id, cut_components: Vec::new(), cached_score: (0, 0, 0)
                    });
                    shaken // These are already "fixed" agreement subtrees
                } else { Vec::new() }
            } else { Vec::new() };

            while !curr.tree1.is_leaf() && Instant::now() < deadline {
                let candidates = get_conflict_candidates(&curr);
                let &leaf_id = candidates.choose(&mut rng).unwrap();
                let next_t1 = cut_leaf(&curr.tree1, leaf_id);
                let next_t2 = cut_leaf(&curr.tree2, leaf_id);
                let mut next_nc = curr.cut_components.clone();
                next_nc.push(curr.expansions.get(&leaf_id).cloned().unwrap_or(Expansion::Leaf(leaf_id)));
                curr = normalize_state(State {
                    tree1: next_t1.unwrap_or(Arc::new(Tree::Leaf(0, Zero::zero()))),
                    tree2: next_t2.unwrap_or(Arc::new(Tree::Leaf(0, Zero::zero()))),
                    expansions: curr.expansions.clone(),
                    next_id: curr.next_id,
                    cut_components: next_nc,
                    cached_score: (0, 0, 0),
                });
            }
            
            if curr.tree1.is_leaf() {
                for comp in &curr.cut_components {
                    let mut s = HashSet::new(); collect_leaves(comp, &mut s);
                    p.push(s);
                }
                let mut s = HashSet::new();
                let rid = curr.tree1.leaf_id();
                let last_exp = curr.expansions.get(&rid).cloned().unwrap_or(Expansion::Leaf(rid));
                collect_leaves(&last_exp, &mut s);
                p.push(s);
                
                p = merge_partitions(p, ot1, ot2);
                let mut bc = best_count.lock().unwrap();
                if p.len() < *bc { *bc = p.len(); *best_partition.lock().unwrap() = p; }
            }
        }
    });

    let res = best_partition.lock().unwrap().clone();
    if res.is_empty() { (1..=n_leaves).map(|i| { let mut s = HashSet::new(); s.insert(i); s }).collect() } else { res }
}

fn merge_partitions(mut p: Vec<HashSet<u32>>, ot1: &OriginalNode, ot2: &OriginalNode) -> Vec<HashSet<u32>> {
    let mut changed = true;
    while changed {
        changed = false;
        let mut i = 0;
        while i < p.len() {
            let mut j = i + 1;
            while j < p.len() {
                let mut merged = p[i].clone();
                merged.extend(&p[j]);
                if let (Some(e1), Some(e2)) = (build_induced_expansion(ot1, &merged), build_induced_expansion(ot2, &merged)) {
                    if are_isomorphic(&e1, &e2) {
                        p[i] = merged; p.remove(j);
                        changed = true; continue;
                    }
                }
                j += 1;
            }
            i += 1;
        }
    }
    p
}

fn collect_leaves(exp: &Expansion, set: &mut HashSet<u32>) {
    match exp {
        Expansion::Leaf(id) => { set.insert(*id); }
        Expansion::Node(l, r) => { collect_leaves(l, set); collect_leaves(r, set); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_isomorphism() {
        let t1 = Expansion::Node(Box::new(Expansion::Leaf(1)), Box::new(Expansion::Leaf(2)));
        let t2 = Expansion::Node(Box::new(Expansion::Leaf(2)), Box::new(Expansion::Leaf(1)));
        assert!(are_isomorphic(&t1, &t2));

        let t3 = Expansion::Node(
            Box::new(Expansion::Leaf(1)),
            Box::new(Expansion::Node(Box::new(Expansion::Leaf(2)), Box::new(Expansion::Leaf(3))))
        );
        let t4 = Expansion::Node(
            Box::new(Expansion::Node(Box::new(Expansion::Leaf(2)), Box::new(Expansion::Leaf(3)))),
            Box::new(Expansion::Leaf(1))
        );
        assert!(are_isomorphic(&t3, &t4));
    }

    #[test]
    fn test_merge_partitions_basic() {
        let mut p = Vec::new();
        let mut s1 = HashSet::new(); s1.insert(1);
        let mut s2 = HashSet::new(); s2.insert(2);
        p.push(s1);
        p.push(s2);

        let ot1 = OriginalNode {
            left: Some(Box::new(OriginalNode { left: None, right: None, label: Some(1) })),
            right: Some(Box::new(OriginalNode { left: None, right: None, label: Some(2) })),
            label: None,
        };
        let ot2 = ot1.clone();

        let merged = merge_partitions(p, &ot1, &ot2);
        assert_eq!(merged.len(), 1);
        assert!(merged[0].contains(&1));
        assert!(merged[0].contains(&2));
    }
}
