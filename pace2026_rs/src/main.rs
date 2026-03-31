mod tree;
mod state;
mod io;

use std::collections::{HashMap, HashSet};
use crate::tree::{Tree, Expansion, original_to_tree, collect_cherries, cut_leaf, offpath_candidates, get_all_leaves, OriginalNode, FastBitSet};
use crate::state::{State, normalize_state};
use crate::io::{parse_instance_file, render_expansion};
use anyhow::Result;
use std::time::{Instant, Duration};
use rayon::prelude::*;
use std::sync::{Arc, Mutex};
use num_traits::Zero;
use rand::prelude::*;
use std::hash::{Hash, Hasher};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: pace2026_rs <input.nw> [--time-limit <s|300>]");
        std::process::exit(1);
    }
    let mut time_limit = 300;
    for i in 0..args.len() {
        if args[i] == "--time-limit" && i + 1 < args.len() { time_limit = args[i+1].parse().unwrap_or(300); }
    }
    std::thread::Builder::new()
        .stack_size(512 * 1024 * 1024)
        .spawn(move || { if let Err(e) = run(time_limit) { eprintln!("Error: {}", e); } })?
        .join()
        .expect("Thread failed");
    Ok(())
}

fn run(time_limit: u64) -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let instance = parse_instance_file(&args[1])?;
    let t1 = original_to_tree(&instance.tree1, instance.n_leaves);
    let t2 = original_to_tree(&instance.tree2, instance.n_leaves);
    let partition = solve_partition(t1, t2, instance.n_leaves, time_limit, &instance.tree1, &instance.tree2);
    for leaf_set in partition {
        if let Some(exp) = build_induced_expansion(&instance.tree1, &leaf_set) {
            println!("{};", render_expansion(&exp));
        }
    }
    Ok(())
}

fn are_isomorphic(n1: &Expansion, n2: &Expansion) -> bool { n1 == n2 }

fn build_induced_expansion(node: &OriginalNode, leaves: &HashSet<u32>) -> Option<Expansion> {
    if let Some(id) = node.label { return if leaves.contains(&id) { Some(Expansion::Leaf(id)) } else { None }; }
    let l = build_induced_expansion(node.left.as_ref().unwrap(), leaves);
    let r = build_induced_expansion(node.right.as_ref().unwrap(), leaves);
    match (l, r) { (Some(tl), Some(tr)) => Some(Expansion::new_node(tl, tr)), (Some(t), None) | (None, Some(t)) => Some(t), (None, None) => None }
}

fn get_conflict_batch(state: &State, size: usize) -> Vec<u32> {
    let mut c1 = HashSet::new(); collect_cherries(&state.tree1, &mut c1);
    let mut c2 = HashSet::new(); collect_cherries(&state.tree2, &mut c2);
    let diff1: Vec<_> = c1.difference(&c2).collect();
    let diff2: Vec<_> = c2.difference(&c1).collect();
    let mut rng = rand::rng();
    let mut batch = HashSet::new();
    
    // Pick multiple conflicting cherries to speed up the process
    let mut combined_diff: Vec<_> = diff1.into_iter().chain(diff2.into_iter()).collect();
    combined_diff.shuffle(&mut rng);
    
    for cherry in combined_diff.iter().take(size) {
        batch.insert(cherry.0);
        batch.insert(cherry.1);
    }
    
    if batch.is_empty() { return get_all_leaves(&state.tree1).into_iter().take(size).collect(); }
    batch.into_iter().collect()
}

fn solve_partition(t1: Arc<Tree>, t2: Arc<Tree>, n_leaves: u32, limit_seconds: u64, ot1: &OriginalNode, ot2: &OriginalNode) -> Vec<HashSet<u32>> {
    let start_time = Instant::now();
    let deadline = start_time + Duration::from_secs(limit_seconds);
    let mut expansions = HashMap::new();
    for i in 1..=n_leaves { expansions.insert(i, Expansion::Leaf(i)); }
    let start_state = normalize_state(State { tree1: t1, tree2: t2, expansions, next_id: n_leaves + 1, cut_components: Vec::new(), cached_score: (0, 0, 0) });
    let best_partition = Arc::new(Mutex::new(vec![]));
    let best_count = Arc::new(Mutex::new(n_leaves as usize + 1));

    (0..rayon::current_num_threads()).into_par_iter().for_each(|_| {
        let mut rng = rand::rng();
        while Instant::now() < deadline {
            let mut curr = start_state.clone();
            let mut p = Vec::new();
            
            // Fast Anytime Loop with Batching
            while !curr.tree1.is_leaf() && Instant::now() < deadline {
                // Cut 10 leaves at once in large instances to speed up
                let batch_size = if curr.tree1.size() > 1000 { 20 } else { 1 };
                let to_cut = get_conflict_batch(&curr, batch_size);
                
                let mut nt1 = curr.tree1.clone();
                let mut nt2 = curr.tree2.clone();
                let mut nnc = curr.cut_components.clone();
                
                for leaf_id in to_cut {
                    if let Some(t) = cut_leaf(&nt1, leaf_id) { nt1 = t; }
                    if let Some(t) = cut_leaf(&nt2, leaf_id) { nt2 = t; }
                    nnc.push(curr.expansions.get(&leaf_id).cloned().unwrap_or(Expansion::Leaf(leaf_id)));
                }
                
                curr = normalize_state(State { tree1: nt1, tree2: nt2, expansions: curr.expansions.clone(), next_id: curr.next_id, cut_components: nnc, cached_score: (0, 0, 0) });
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
                
                let merged_p = merge_partitions(p, ot1, ot2);
                let mut bc = best_count.lock().unwrap();
                if merged_p.len() < *bc { *bc = merged_p.len(); *best_partition.lock().unwrap() = merged_p; }
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
                if p.len() > 200 && (p[i].len() > 100 || p[j].len() > 100) { j += 1; continue; }
                let mut merged = p[i].clone();
                merged.extend(&p[j]);
                if let (Some(e1), Some(e2)) = (build_induced_expansion(ot1, &merged), build_induced_expansion(ot2, &merged)) {
                    if e1 == e2 { p[i] = merged; p.remove(j); changed = true; continue; }
                }
                j += 1;
            }
            i += 1;
        }
    }
    p
}

fn collect_leaves(exp: &Expansion, set: &mut HashSet<u32>) {
    let mut stack = vec![exp];
    while let Some(e) = stack.pop() {
        match e { Expansion::Leaf(id) => { set.insert(*id); } Expansion::Node(l, r, _) => { stack.push(l); stack.push(r); } }
    }
}
