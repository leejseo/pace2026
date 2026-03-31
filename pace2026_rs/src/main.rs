mod tree;
mod state;
mod io;

use std::collections::{HashMap, HashSet};
use crate::tree::{Tree, Expansion, original_to_tree, collect_cherries, cut_leaf, get_all_leaves, OriginalNode};
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
    
    // Stage 1: Get a base partition (Anytime search)
    let partition = solve_partition(t1, t2, instance.n_leaves, time_limit);
    
    // Stage 2: Post-process (Greedy Merge to fix over-cutting)
    // For now, we use the induced subtree of T1 for each leaf set in the partition.
    for leaf_set in partition {
        if let Some(exp) = build_induced_expansion(&instance.tree1, &leaf_set) {
            println!("{};", render_expansion(&exp));
        }
    }
    Ok(())
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

fn solve_partition(tree1: Arc<Tree>, tree2: Arc<Tree>, n_leaves: u32, limit_seconds: u64) -> Vec<HashSet<u32>> {
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
            while !curr.tree1.is_leaf() && Instant::now() < deadline {
                let leaves = get_all_leaves(&curr.tree1);
                let &leaf_id = leaves.choose(&mut rng).unwrap();
                
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
                let mut leaf_sets = Vec::new();
                for comp in &curr.cut_components {
                    let mut s = HashSet::new(); collect_leaves(comp, &mut s);
                    leaf_sets.push(s);
                }
                let mut s = HashSet::new();
                let rid = curr.tree1.leaf_id();
                let last_exp = curr.expansions.get(&rid).cloned().unwrap_or(Expansion::Leaf(rid));
                collect_leaves(&last_exp, &mut s);
                leaf_sets.push(s);
                
                let mut bc = best_count.lock().unwrap();
                if leaf_sets.len() < *bc {
                    *bc = leaf_sets.len();
                    *best_partition.lock().unwrap() = leaf_sets;
                }
            }
        }
    });

    let res = best_partition.lock().unwrap().clone();
    if res.is_empty() {
        (1..=n_leaves).map(|i| { let mut s = HashSet::new(); s.insert(i); s }).collect()
    } else {
        res
    }
}

fn collect_leaves(exp: &Expansion, set: &mut HashSet<u32>) {
    match exp {
        Expansion::Leaf(id) => { set.insert(*id); }
        Expansion::Node(l, r) => { collect_leaves(l, set); collect_leaves(r, set); }
    }
}
