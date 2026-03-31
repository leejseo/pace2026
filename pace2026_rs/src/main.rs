mod tree;
mod state;
mod io;

use std::collections::{HashMap, HashSet};
use crate::tree::{Tree, Expansion, original_to_tree, collect_cherries, cut_leaf, get_all_leaves, OriginalNode, FastBitSet};
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
    
    // Index all clusters in both trees for O(1) lookup
    let mut clusters1 = HashSet::new();
    collect_all_cluster_masks(&t1, &mut clusters1);
    let mut clusters2 = HashSet::new();
    collect_all_cluster_masks(&t2, &mut clusters2);

    let partition = solve_partition(t1, t2, instance.n_leaves, time_limit, &clusters1, &clusters2);
    
    for leaf_set in partition {
        if let Some(exp) = build_induced_expansion(&instance.tree1, &leaf_set) {
            println!("{};", render_expansion(&exp));
        }
    }
    Ok(())
}

fn collect_all_cluster_masks(tree: &Arc<Tree>, set: &mut HashSet<FastBitSet>) {
    let mut stack = vec![tree.clone()];
    while let Some(node) = stack.pop() {
        set.insert(node.mask().clone());
        if let Tree::Node(l, r, _, _, _) = node.as_ref() {
            stack.push(l.clone()); stack.push(r.clone());
        }
    }
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

fn get_conflict_batch(state: &State, size: usize) -> Vec<u32> {
    let mut c1 = HashSet::new(); collect_cherries(&state.tree1, &mut c1);
    let mut c2 = HashSet::new(); collect_cherries(&state.tree2, &mut c2);
    let mut combined_diff: Vec<_> = c1.difference(&c2).chain(c2.difference(&c1)).cloned().collect();
    let mut rng = rand::rng();
    combined_diff.shuffle(&mut rng);
    let mut batch = HashSet::new();
    for cherry in combined_diff.iter().take(size) { batch.insert(cherry.0); batch.insert(cherry.1); }
    if batch.is_empty() { return get_all_leaves(&state.tree1).into_iter().take(size).collect(); }
    batch.into_iter().collect()
}

fn solve_partition(t1: Arc<Tree>, t2: Arc<Tree>, n_leaves: u32, limit_seconds: u64, c1: &HashSet<FastBitSet>, c2: &HashSet<FastBitSet>) -> Vec<HashSet<u32>> {
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
            while !curr.tree1.is_leaf() && Instant::now() < deadline {
                let batch_size = if curr.tree1.size() > 1000 { 30 } else { 1 };
                let to_cut = get_conflict_batch(&curr, batch_size);
                let mut nt1 = curr.tree1.clone();
                let mut nt2 = curr.tree2.clone();
                let mut nnc = curr.cut_components.clone();
                for lid in to_cut {
                    if let Some(t) = cut_leaf(&nt1, lid) { nt1 = t; }
                    if let Some(t) = cut_leaf(&nt2, lid) { nt2 = t; }
                    nnc.push(curr.expansions.get(&lid).cloned().unwrap_or(Expansion::Leaf(lid)));
                }
                curr = normalize_state(State { tree1: nt1, tree2: nt2, expansions: curr.expansions.clone(), next_id: curr.next_id, cut_components: nnc, cached_score: (0, 0, 0) });
            }
            if curr.tree1.is_leaf() {
                let mut p = Vec::new();
                for comp in &curr.cut_components {
                    let mut s = HashSet::new(); collect_leaves(comp, &mut s);
                    let mut m = FastBitSet::new(n_leaves); for &id in &s { m.set(id); }
                    p.push((s, m));
                }
                let mut s = HashSet::new(); let rid = curr.tree1.leaf_id();
                collect_leaves(&curr.expansions.get(&rid).cloned().unwrap_or(Expansion::Leaf(rid)), &mut s);
                let mut m = FastBitSet::new(n_leaves); for &id in &s { m.set(id); }
                p.push((s, m));
                
                let merged_p = merge_partitions_fast(p, c1, c2);
                let mut bc = best_count.lock().unwrap();
                if merged_p.len() < *bc { *bc = merged_p.len(); *best_partition.lock().unwrap() = merged_p; }
            }
        }
    });
    best_partition.lock().unwrap().clone()
}

fn merge_partitions_fast(mut p: Vec<(HashSet<u32>, FastBitSet)>, c1: &HashSet<FastBitSet>, c2: &HashSet<FastBitSet>) -> Vec<HashSet<u32>> {
    let mut changed = true;
    while changed {
        changed = false;
        let mut i = 0;
        while i < p.len() {
            let mut j = i + 1;
            while j < p.len() {
                let merged_mask = p[i].1.or(&p[j].1);
                // O(1) Lookup: A merge is valid IF the combined leaf set is a cluster in BOTH trees
                if c1.contains(&merged_mask) && c2.contains(&merged_mask) {
                    let mut s = p[i].0.clone(); s.extend(p[j].0.iter());
                    p[i] = (s, merged_mask);
                    p.remove(j);
                    changed = true; continue;
                }
                j += 1;
            }
            i += 1;
        }
    }
    p.into_iter().map(|x| x.0).collect()
}

fn collect_leaves(exp: &Expansion, set: &mut HashSet<u32>) {
    let mut stack = vec![exp];
    while let Some(e) = stack.pop() {
        match e { Expansion::Leaf(id) => { set.insert(*id); } Expansion::Node(l, r, _) => { stack.push(l); stack.push(r); } }
    }
}
