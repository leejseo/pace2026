mod tree;
mod state;
mod io;

use std::collections::{HashSet};
use crate::tree::{OriginalNode, Expansion};
use crate::io::{parse_instance_file, render_expansion};
use anyhow::Result;
use std::time::{Instant, Duration};
use rayon::prelude::*;
use std::sync::{Arc, Mutex};
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
            if let Ok(limit) = args[i+1].parse() { time_limit = limit; }
        }
    }
    // Safety buffer: Finish 10 seconds early to ensure output is printed
    let safe_limit = if time_limit > 15 { time_limit - 10 } else { time_limit };
    if let Err(e) = run(safe_limit) { eprintln!("Error: {}", e); }
    Ok(())
}

fn run(time_limit: u64) -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let instance = parse_instance_file(&args[1])?;
    let mut labels = HashSet::new(); get_labels(&instance.tree1, &mut labels);
    let n_leaves = labels.len() as u32;
    let partition = solve_maf_lns(&instance.tree1, &instance.tree2, n_leaves, time_limit, &labels);
    
    // Final output of the best found partition
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

fn solve_maf_lns(tree1: &OriginalNode, tree2: &OriginalNode, _n_leaves: u32, limit_seconds: u64, all_labels: &HashSet<u32>) -> Vec<HashSet<u32>> {
    let start_time = Instant::now();
    let deadline = start_time + Duration::from_secs(limit_seconds);
    
    let initial_partition = build_greedy_forest(tree1, tree2, all_labels, &mut rand::rng());
    let best_partition = Arc::new(Mutex::new(initial_partition));
    let best_count = Arc::new(Mutex::new(best_partition.lock().unwrap().len()));

    (0..rayon::current_num_threads()).into_par_iter().for_each(|_| {
        let mut rng = rand::rng();
        while Instant::now() < deadline {
            let mut current = {
                let bp = best_partition.lock().unwrap();
                bp.clone()
            };

            let destroy_count = (current.len() as f64 * rng.random_range(0.1..0.3)) as usize;
            let mut removed_labels = HashSet::new();
            for _ in 0..destroy_count.max(1) {
                if current.is_empty() { break; }
                let idx = rng.random_range(0..current.len());
                removed_labels.extend(current.remove(idx));
            }

            if !removed_labels.is_empty() {
                let repaired = build_greedy_forest(tree1, tree2, &removed_labels, &mut rng);
                current.extend(repaired);
            }

            current = merge_components(tree1, tree2, current, &mut rng, deadline);

            let count = current.len();
            let mut bc = best_count.lock().unwrap();
            if count < *bc {
                *bc = count;
                *best_partition.lock().unwrap() = current;
                eprintln!("New best (LNS): {}", count);
            }
        }
    });
    
    let res = best_partition.lock().unwrap().clone();
    res
}

fn build_greedy_forest(t1: &OriginalNode, t2: &OriginalNode, labels: &HashSet<u32>, rng: &mut ThreadRng) -> Vec<HashSet<u32>> {
    let mut current_labels = labels.clone();
    let mut forest = Vec::new();
    while !current_labels.is_empty() {
        let mut comp = HashSet::new();
        let mut candidates: Vec<u32> = current_labels.iter().cloned().collect();
        candidates.shuffle(rng);
        for l in candidates {
            comp.insert(l);
            if !is_truly_isomorphic(t1, t2, &comp) { comp.remove(&l); }
            if comp.len() >= 100 { break; }
        }
        if comp.is_empty() { let first = *current_labels.iter().next().unwrap(); comp.insert(first); }
        for &l in &comp { current_labels.remove(&l); }
        forest.push(comp);
    }
    forest
}

fn merge_components(t1: &OriginalNode, t2: &OriginalNode, mut forest: Vec<HashSet<u32>>, rng: &mut ThreadRng, deadline: Instant) -> Vec<HashSet<u32>> {
    let mut changed = true;
    let mut iter = 0;
    while changed && Instant::now() < deadline && iter < 100 {
        changed = false;
        iter += 1;
        if forest.len() <= 1 { break; }
        let i = rng.random_range(0..forest.len());
        let j = (i + rng.random_range(1..forest.len())) % forest.len();
        let mut merged = forest[i].clone();
        merged.extend(&forest[j]);
        if is_truly_isomorphic(t1, t2, &merged) {
            forest.remove(if i > j { i } else { j });
            forest.remove(if i > j { j } else { i });
            forest.push(merged);
            changed = true;
        }
    }
    forest
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
