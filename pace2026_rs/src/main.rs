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
    let safe_limit = if time_limit > 5 { time_limit - 2 } else { time_limit };
    if let Err(e) = run(safe_limit) { eprintln!("Error: {}", e); }
    Ok(())
}

fn run(time_limit: u64) -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let instance = parse_instance_file(&args[1])?;
    let mut labels = HashSet::new(); get_labels(&instance.tree1, &mut labels);
    let n_leaves = labels.len() as u32;
    let partition = solve_maf_safe(&instance.tree1, &instance.tree2, n_leaves, time_limit, &labels);
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

fn solve_maf_safe(tree1: &OriginalNode, tree2: &OriginalNode, _n_leaves: u32, limit_seconds: u64, all_labels: &HashSet<u32>) -> Vec<HashSet<u32>> {
    let start_time = Instant::now();
    let deadline = start_time + Duration::from_secs(limit_seconds);
    let best_partition = Arc::new(Mutex::new(all_labels.iter().map(|&l| { let mut s = HashSet::new(); s.insert(l); s }).collect::<Vec<_>>()));
    let best_count = Arc::new(Mutex::new(all_labels.len()));

    (0..rayon::current_num_threads()).into_par_iter().for_each(|_| {
        let mut rng = rand::rng();
        while Instant::now() < deadline {
            let mut current_labels = all_labels.clone();
            let mut components = Vec::new();
            
            while !current_labels.is_empty() {
                if Instant::now() > deadline { break; }
                let mut candidates: Vec<u32> = current_labels.iter().cloned().collect();
                candidates.shuffle(&mut rng);
                
                let mut comp = HashSet::new();
                for l in candidates {
                    comp.insert(l);
                    if !is_truly_isomorphic(tree1, tree2, &comp) { comp.remove(&l); }
                    if comp.len() >= 100 { break; }
                }
                
                if comp.is_empty() {
                    let first = *current_labels.iter().next().unwrap();
                    comp.insert(first);
                }
                
                for &l in &comp { current_labels.remove(&l); }
                components.push(comp);
            }

            if current_labels.is_empty() {
                // Local Search: Merge
                let mut refined = components;
                let mut local_improved = true;
                while local_improved && Instant::now() < deadline {
                    local_improved = false;
                    if refined.len() <= 1 { break; }
                    let i = rng.random_range(0..refined.len());
                    let j = (i + rng.random_range(1..refined.len())) % refined.len();
                    let mut merged = refined[i].clone();
                    merged.extend(&refined[j]);
                    if is_truly_isomorphic(tree1, tree2, &merged) {
                        refined.remove(if i > j { i } else { j });
                        refined.remove(if i > j { j } else { i });
                        refined.push(merged);
                        local_improved = true;
                    }
                }

                let count = refined.len();
                let mut bc = best_count.lock().unwrap();
                if count < *bc {
                    *bc = count;
                    *best_partition.lock().unwrap() = refined;
                    eprintln!("New best: {}", count);
                }
            }
        }
    });
    best_partition.lock().unwrap().clone()
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
