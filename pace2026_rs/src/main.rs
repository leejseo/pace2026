mod tree;
mod state;
mod io;

use std::collections::{HashSet, HashMap};
use crate::tree::{OriginalNode, original_to_tree, cut_leaf, offpath_candidates, Tree, get_all_leaves, Expansion};
use crate::io::{parse_instance_file, render_expansion};
use anyhow::Result;
use std::time::{Instant, Duration};
use rayon::prelude::*;
use std::sync::{Arc, Mutex};
use rand::prelude::*;

const EMPTY_SENTINEL: u32 = 2_000_000_000;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: pace2026_rs <input.nw> [--time-limit <s|300>]");
        std::process::exit(1);
    }
    let mut time_limit = 300;
    for i in 0..args.len() {
        if args[i] == "--time-limit" && i + 1 < args.len() {
            if let Ok(limit) = args[i+1].parse() { time_limit = limit; }
        }
    }
    let safe_limit = if time_limit > 5 { time_limit - 2 } else { time_limit };
    if let Err(e) = run(safe_limit) { eprintln!("Error: {}", e); }
    Ok(())
}

fn run(time_limit: u64) -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let instance = parse_instance_file(&args[1])?;
    let mut labels = HashSet::new(); get_labels(&instance.tree1, &mut labels);
    let n_leaves = labels.len() as u32;
    let partition = solve_maf_anytime(&instance.tree1, &instance.tree2, n_leaves, time_limit, &labels);
    let mut output = String::new();
    for leaf_set in partition {
        if let Some(exp) = build_induced_expansion(&instance.tree1, &leaf_set) {
            output.push_str(&render_expansion(&exp));
            output.push_str(";\n");
        }
    }
    print!("{}", output);
    Ok(())
}

fn get_labels(node: &OriginalNode, set: &mut HashSet<u32>) {
    if let Some(id) = node.label { set.insert(id); }
    if let Some(ref l) = node.left { get_labels(l, set); }
    if let Some(ref r) = node.right { get_labels(r, set); }
}

fn build_induced_expansion(node: &OriginalNode, leaves: &HashSet<u32>) -> Option<Expansion> {
    if let Some(id) = node.label { return if leaves.contains(&id) { Some(Expansion::Leaf(id)) } else { None }; }
    let l = build_induced_expansion(node.left.as_ref().unwrap(), leaves);
    let r = build_induced_expansion(node.right.as_ref().unwrap(), leaves);
    match (l, r) { 
        (Some(tl), Some(tr)) => Some(Expansion::new_node(tl, tr)), 
        (Some(t), None) | (None, Some(t)) => Some(t), 
        (None, None) => None 
    }
}

fn solve_maf_anytime(tree1: &OriginalNode, tree2: &OriginalNode, n_leaves: u32, limit_seconds: u64, all_labels: &HashSet<u32>) -> Vec<HashSet<u32>> {
    let start_time = Instant::now();
    let deadline = start_time + Duration::from_secs(limit_seconds);
    
    let mut initial = Vec::new();
    for &l in all_labels {
        let mut s = HashSet::new(); s.insert(l);
        initial.push(s);
    }
    let best_partition = Arc::new(Mutex::new(initial));
    let best_count = Arc::new(Mutex::new(all_labels.len()));

    (0..rayon::current_num_threads()).into_par_iter().for_each(|_| {
        let mut rng = rand::rng();
        while Instant::now() < deadline {
            // Biased toward Local Search to ensure anytime improvement
            if rng.random_bool(0.05) {
                let current = grasp_step(tree1, tree2, n_leaves, deadline, all_labels, &mut rng);
                if !current.is_empty() {
                    let mut bc = best_count.lock().unwrap();
                    if current.len() < *bc {
                        *bc = current.len();
                        *best_partition.lock().unwrap() = current;
                        eprintln!("New best (GRASP): {}", *bc);
                    }
                }
            } else {
                let mut current = {
                    let bp = best_partition.lock().unwrap();
                    bp.clone()
                };
                if current.len() <= 1 { std::thread::sleep(Duration::from_millis(10)); continue; }
                
                // Attempt to merge a random pair of components
                let i = rng.random_range(0..current.len());
                let j = (i + rng.random_range(1..current.len())) % current.len();
                let mut merged = current[i].clone();
                merged.extend(&current[j]);
                
                if is_truly_isomorphic(tree1, tree2, &merged) {
                    let mut bp = best_partition.lock().unwrap();
                    if current.len() <= bp.len() { // Only update if still relevant
                        current.remove(if i > j { i } else { j });
                        current.remove(if i > j { j } else { i });
                        current.push(merged);
                        if current.len() < bp.len() {
                            *best_count.lock().unwrap() = current.len();
                            *bp = current;
                            eprintln!("New best (Merge): {}", bp.len());
                        }
                    }
                }
            }
        }
    });
    best_partition.lock().unwrap().clone()
}

fn grasp_step(tree1: &OriginalNode, tree2: &OriginalNode, n_leaves: u32, deadline: Instant, all_labels: &HashSet<u32>, rng: &mut ThreadRng) -> Vec<HashSet<u32>> {
    let mut t1 = original_to_tree(tree1, n_leaves);
    let mut t2 = original_to_tree(tree2, n_leaves);
    let mut components: Vec<HashSet<u32>> = Vec::new();
    let mut virtual_map: HashMap<u32, HashSet<u32>> = HashMap::new();
    for &l in all_labels {
        let mut s = HashSet::new(); s.insert(l);
        virtual_map.insert(l, s);
    }
    let mut next_virtual_id = n_leaves + 1;

    loop {
        if Instant::now() > deadline { break; }
        let mut contracted = true;
        while contracted {
            contracted = false;
            let mut c1 = HashSet::new(); collect_cherries_internal(&t1, &mut c1);
            let mut c2 = HashSet::new(); collect_cherries_internal(&t2, &mut c2);
            let common: Vec<_> = c1.intersection(&c2).cloned().collect();
            if !common.is_empty() {
                contracted = true;
                for (a, b) in common {
                    let new_id = next_virtual_id; next_virtual_id += 1;
                    if let (Some(mut s1), Some(s2)) = (virtual_map.remove(&a), virtual_map.remove(&b)) {
                        s1.extend(s2); virtual_map.insert(new_id, s1);
                        t1 = contract_cherry_internal(&t1, a, b, new_id, n_leaves);
                        t2 = contract_cherry_internal(&t2, a, b, new_id, n_leaves);
                    }
                }
            }
        }
        if t1.size() <= 1 { break; }
        let mut c1 = Vec::new(); collect_cherries_vec(&t1, &mut c1);
        let mut c2_set = HashSet::new(); collect_cherries_internal(&t2, &mut c2_set);
        let mut conflict = None;
        c1.shuffle(rng);
        for (a, b) in c1 { if !c2_set.contains(&(a, b)) { conflict = Some((a, b)); break; } }

        if let Some((a, b)) = conflict {
            let ops = offpath_candidates(&t2, a, b);
            let choice = if ops.is_empty() { rng.random_range(0..2) } 
                         else { if rng.random_bool(0.7) { rng.random_range(0..2) } else { 2 } };
            match choice {
                0 | 1 => {
                    let to_cut = if choice == 0 { a } else { b };
                    if let Some(orig_labels) = virtual_map.remove(&to_cut) { components.push(orig_labels); }
                    t1 = cut_leaf(&t1, to_cut).unwrap_or_else(|| Arc::new(Tree::Leaf(EMPTY_SENTINEL, Arc::new(tree::FastBitSet::new(n_leaves*3)))));
                    t2 = cut_leaf(&t2, to_cut).unwrap_or_else(|| Arc::new(Tree::Leaf(EMPTY_SENTINEL, Arc::new(tree::FastBitSet::new(n_leaves*3)))));
                },
                _ => {
                    for op in ops {
                        let v_leaves = get_all_leaves(&op);
                        let mut combined_orig = HashSet::new();
                        for v in v_leaves {
                            if v != EMPTY_SENTINEL {
                                if let Some(orig) = virtual_map.remove(&v) { combined_orig.extend(orig); }
                                t1 = cut_leaf(&t1, v).unwrap_or_else(|| Arc::new(Tree::Leaf(EMPTY_SENTINEL, Arc::new(tree::FastBitSet::new(n_leaves*3)))));
                                t2 = cut_leaf(&t2, v).unwrap_or_else(|| Arc::new(Tree::Leaf(EMPTY_SENTINEL, Arc::new(tree::FastBitSet::new(n_leaves*3)))));
                            }
                        }
                        if !combined_orig.is_empty() {
                            if is_truly_isomorphic(tree1, tree2, &combined_orig) { components.push(combined_orig); }
                            else { for l in combined_orig { let mut s = HashSet::new(); s.insert(l); components.push(s); } }
                        }
                    }
                }
            }
        } else { break; }
    }
    for (_, orig_labels) in virtual_map {
        if !orig_labels.is_empty() {
            if is_truly_isomorphic(tree1, tree2, &orig_labels) { components.push(orig_labels); }
            else { for l in orig_labels { let mut s = HashSet::new(); s.insert(l); components.push(s); } }
        }
    }
    if components.iter().map(|s| s.len()).sum::<usize>() == all_labels.len() { components } else { Vec::new() }
}

fn collect_cherries_internal(tree: &Arc<Tree>, cherries: &mut HashSet<(u32, u32)>) {
    let mut stack = vec![tree.clone()];
    while let Some(node) = stack.pop() {
        if let Tree::Node(l, r, _, _) = node.as_ref() {
            if l.is_leaf() && r.is_leaf() {
                let (a, b) = (l.leaf_id(), r.leaf_id());
                if a != EMPTY_SENTINEL && b != EMPTY_SENTINEL { cherries.insert(if a < b { (a, b) } else { (b, a) }); }
            }
            stack.push(l.clone()); stack.push(r.clone());
        }
    }
}

fn collect_cherries_vec(tree: &Arc<Tree>, cherries: &mut Vec<(u32, u32)>) {
    let mut stack = vec![tree.clone()];
    while let Some(node) = stack.pop() {
        if let Tree::Node(l, r, _, _) = node.as_ref() {
            if l.is_leaf() && r.is_leaf() {
                let (a, b) = (l.leaf_id(), r.leaf_id());
                if a != EMPTY_SENTINEL && b != EMPTY_SENTINEL { cherries.push(if a < b { (a, b) } else { (b, a) }); }
            }
            stack.push(l.clone()); stack.push(r.clone());
        }
    }
}

fn contract_cherry_internal(tree: &Arc<Tree>, a: u32, b: u32, new_id: u32, n_leaves: u32) -> Arc<Tree> {
    match tree.as_ref() {
        Tree::Leaf(id, _) => if *id == a || *id == b { 
            let mut m = tree::FastBitSet::new(n_leaves*3); m.set(new_id);
            Arc::new(Tree::Leaf(new_id, Arc::new(m)))
        } else { tree.clone() },
        Tree::Node(l, r, _, _) => {
            if l.is_leaf() && r.is_leaf() {
                let (id_l, id_r) = (l.leaf_id(), r.leaf_id());
                if (id_l == a && id_r == b) || (id_l == b && id_r == a) {
                    let mut m = tree::FastBitSet::new(n_leaves*3); m.set(new_id);
                    return Arc::new(Tree::Leaf(new_id, Arc::new(m)));
                }
            }
            let nl = contract_cherry_internal(l, a, b, new_id, n_leaves);
            let nr = contract_cherry_internal(r, a, b, new_id, n_leaves);
            if Arc::ptr_eq(&nl, l) && Arc::ptr_eq(&nr, r) { return tree.clone(); }
            let m = nl.mask().or(nr.mask());
            let sz = nl.size() + nr.size();
            Arc::new(Tree::Node(nl, nr, Arc::new(m), sz))
        }
    }
}

fn is_truly_isomorphic(t1: &OriginalNode, t2: &OriginalNode, leaves: &HashSet<u32>) -> bool {
    if leaves.len() <= 1 { return true; }
    let exp1 = build_induced_expansion(t1, leaves);
    let exp2 = build_induced_expansion(t2, leaves);
    match (exp1, exp2) {
        (Some(e1), Some(e2)) => render_expansion(&e1) == render_expansion(&e2),
        _ => false,
    }
}
