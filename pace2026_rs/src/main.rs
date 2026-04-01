mod tree;
mod state;
mod io;

use std::collections::{HashSet, HashMap};
use crate::tree::{OriginalNode, original_to_tree, Tree, get_all_leaves, Expansion, get_hash_map, FastBitSet};
use crate::io::{parse_instance_file, render_expansion};
use anyhow::Result;
use std::time::{Instant, Duration};
use rayon::prelude::*;
use std::sync::{Arc, Mutex};
use rand::prelude::*;
use std::io::{Write, BufWriter, stdout};

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
    
    let t1 = original_to_tree(&instance.tree1, n_leaves);
    let t2 = original_to_tree(&instance.tree2, n_leaves);

    // Iteration 11: Compute Ancestral Sharing Weighting
    let (initial_clusters, ancestral_scores) = get_mcsr_clusters_and_scores(&t1, &t2, n_leaves, &labels);
    
    // Invert the score to act like discordance (lower is better for building).
    let max_score = ancestral_scores.values().max().unwrap_or(&0).clone();
    let mut discordance = HashMap::new();
    for (&l, &score) in &ancestral_scores {
        discordance.insert(l, max_score - score);
    }

    let partition = solve_maf_alns_sa_final(&t1, &t2, initial_clusters, time_limit, n_leaves, &discordance);
    
    let out = stdout();
    let mut writer = BufWriter::new(out.lock());
    for leaf_set in partition {
        let mut subset_mask = FastBitSet::new(n_leaves * 3);
        for &l in &leaf_set { subset_mask.set(l); }
        if let Some(exp) = build_induced_expansion_fast(&t1, &leaf_set, &subset_mask) {
            writer.write_all(render_expansion(&exp).as_bytes())?;
            writer.write_all(b";\n")?;
        }
    }
    writer.flush()?;
    Ok(())
}

fn get_labels(node: &OriginalNode, set: &mut HashSet<u32>) {
    if let Some(id) = node.label { set.insert(id); }
    if let Some(ref l) = node.left { get_labels(l, set); }
    if let Some(ref r) = node.right { get_labels(r, set); }
}

fn build_induced_expansion_fast(tree: &Arc<Tree>, leaves: &HashSet<u32>, subset_mask: &FastBitSet) -> Option<Expansion> {
    if !tree.mask().intersects(subset_mask) { return None; }
    match tree.as_ref() {
        Tree::Leaf(id, _) => if leaves.contains(id) { Some(Expansion::Leaf(*id)) } else { None },
        Tree::Node(l, r, _, _) => {
            let left_exp = build_induced_expansion_fast(l, leaves, subset_mask);
            let right_exp = build_induced_expansion_fast(r, leaves, subset_mask);
            match (left_exp, right_exp) {
                (Some(tl), Some(tr)) => Some(Expansion::new_node(tl, tr)),
                (Some(t), None) | (None, Some(t)) => Some(t),
                (None, None) => None
            }
        }
    }
}

fn get_mcsr_clusters_and_scores(t1: &Arc<Tree>, t2: &Arc<Tree>, n_leaves: u32, all_labels: &HashSet<u32>) -> (Vec<HashSet<u32>>, HashMap<u32, i32>) {
    let mut sub1 = HashMap::new(); get_hash_map(t1, &mut sub1);
    let mut sub2 = HashMap::new(); get_hash_map(t2, &mut sub2);
    let mut common = Vec::new();
    let mut ancestral_scores = HashMap::new();
    for &l in all_labels { ancestral_scores.insert(l, 0); }
    
    for (hash, node1) in sub1 {
        if sub2.contains_key(&hash) {
            let leaves = get_all_leaves(&node1);
            if leaves.len() > 1 {
                let s: HashSet<u32> = leaves.into_iter().collect();
                let mut subset_mask = FastBitSet::new(n_leaves * 3);
                for &l in &s { 
                    subset_mask.set(l); 
                    if let Some(count) = ancestral_scores.get_mut(&l) {
                        *count += 1;
                    }
                }
                if is_truly_isomorphic_fast(t1, t2, &s, &subset_mask) { common.push(s); }
            }
        }
    }
    common.sort_by_key(|c| c.len()); common.reverse();
    let mut used = HashSet::new();
    let mut result = Vec::new();
    for c in common {
        if c.iter().all(|l| !used.contains(l)) {
            for &l in &c { used.insert(l); }
            result.push(c);
        }
    }
    for &l in all_labels { if !used.contains(&l) { let mut s = HashSet::new(); s.insert(l); result.push(s); } }
    (result, ancestral_scores)
}

fn solve_maf_alns_sa_final(t1: &Arc<Tree>, t2: &Arc<Tree>, initial: Vec<HashSet<u32>>, limit_seconds: u64, n_leaves: u32, discordance: &HashMap<u32, i32>) -> Vec<HashSet<u32>> {
    let start_time = Instant::now();
    let deadline = start_time + Duration::from_secs(limit_seconds);
    let best_partition_shared = Arc::new(Mutex::new(initial.clone()));
    let best_count_shared = Arc::new(Mutex::new(initial.len()));

    (0..rayon::current_num_threads()).into_par_iter().for_each(|thread_id| {
        let mut rng = rand::rng();
        let mut current = initial.clone();
        let mut current_count = current.len();

        while Instant::now() < deadline {
            let mut next = current.clone();
            
            let elapsed = start_time.elapsed().as_secs_f64();
            let progress = elapsed / limit_seconds as f64;

            // Iteration 10: Multi-Phase Anytime Execution
            let (phase_destroy_base, phase_destroy_max, allow_sa) = if progress < 0.6 {
                (0.05, 0.25, true)
            } else if progress < 0.9 {
                (0.01, 0.10, true)
            } else {
                (0.01, 0.05, false) 
            };

            let destroy_base = phase_destroy_base + (thread_id as f64 * 0.01);
            let destroy_max = destroy_base + phase_destroy_max;
            let t_mult = 5.0 + (thread_id as f64 * 5.0);
            let dynamic_temp = 1.0 * (1.0 - progress).max(0.001);

            // ADAPTIVE DESTROY
            let strategy = rng.random_range(0..3);
            let removed = match strategy {
                0 => {
                    let count = (next.len() as f64 * rng.random_range(destroy_base..destroy_max)) as usize;
                    let mut pool = HashSet::new();
                    for _ in 0..count.max(1) { if !next.is_empty() { pool.extend(next.swap_remove(rng.random_range(0..next.len()))); } }
                    pool
                }
                1 => {
                    next.sort_by_key(|s| s.len());
                    let mut pool = HashSet::new();
                    for _ in 0..(next.len() / 4).max(1) { if !next.is_empty() { pool.extend(next.remove(0)); } }
                    pool
                }
                _ => {
                    next.sort_by_key(|s| std::cmp::Reverse(s.len()));
                    let mut pool = HashSet::new();
                    for _ in 0..(rng.random_range(1..=2)).min(next.len()) {
                        if !next.is_empty() { pool.extend(next.remove(0)); }
                    }
                    pool
                }
            };

            // REPAIR: Discordance-biased expand
            if !removed.is_empty() {
                let mut pool_vec: Vec<u32> = removed.iter().cloned().collect();
                pool_vec.shuffle(&mut rng);
                pool_vec.sort_by_key(|l| *discordance.get(l).unwrap_or(&0));
                pool_vec.reverse();
                
                while !pool_vec.is_empty() {
                    let mut comp = HashSet::new(); comp.insert(pool_vec.pop().unwrap());
                    let mut subset_mask = FastBitSet::new(n_leaves * 3);
                    for &l in &comp { subset_mask.set(l); }
                    
                    let mut i = 0;
                    while i < pool_vec.len() {
                        comp.insert(pool_vec[i]);
                        subset_mask.set(pool_vec[i]);
                        
                        if is_truly_isomorphic_fast(t1, t2, &comp, &subset_mask) { 
                            pool_vec.remove(i); 
                        } else { 
                            comp.remove(&pool_vec[i]); 
                            subset_mask = FastBitSet::new(n_leaves * 3);
                            for &l in &comp { subset_mask.set(l); }
                            i += 1; 
                        }
                        if comp.len() > 500 { break; }
                    }
                    next.push(comp);
                }
            }

            // LOCAL MERGE
            if dynamic_temp < 0.05 {
                // Iteration 8: Exhaustive Pairwise Merging at Low Temperatures
                let mut changed = true;
                while changed && Instant::now() < deadline {
                    changed = false;
                    let mut i = 0;
                    while i < next.len() && Instant::now() < deadline {
                        let mut j = i + 1;
                        let mut merged_flag = false;
                        while j < next.len() {
                            let mut merged = next[i].clone(); merged.extend(&next[j]);
                            let mut subset_mask = FastBitSet::new(n_leaves * 3);
                            for &l in &merged { subset_mask.set(l); }
                            
                            if is_truly_isomorphic_fast(t1, t2, &merged, &subset_mask) {
                                next.remove(j);
                                next[i] = merged;
                                changed = true;
                                merged_flag = true;
                                break;
                            }
                            j += 1;
                        }
                        if !merged_flag { i += 1; }
                    }
                }
            } else {
                for _ in 0..100 {
                    if next.len() <= 1 { break; }
                    let i = rng.random_range(0..next.len());
                    let j = (i + rng.random_range(1..next.len())) % next.len();
                    let mut merged = next[i].clone(); merged.extend(&next[j]);
                    let mut subset_mask = FastBitSet::new(n_leaves * 3);
                    for &l in &merged { subset_mask.set(l); }
                    
                    if is_truly_isomorphic_fast(t1, t2, &merged, &subset_mask) {
                        let (f, s) = if i > j { (i, j) } else { (j, i) };
                        next.remove(f); next.remove(s); next.push(merged);
                    }
                }
            }

            let next_count = next.len();
            let delta = next_count as f64 - current_count as f64;
            
            if delta <= 0.0 || (allow_sa && rng.random_bool((-delta / (dynamic_temp * t_mult)).exp().clamp(0.0, 1.0))) {
                current = next; current_count = next_count;
                let mut bc = best_count_shared.lock().unwrap();
                if current_count < *bc {
                    *bc = current_count;
                    *best_partition_shared.lock().unwrap() = current.clone();
                    eprintln!("New best (ALNS-SA): {}", *bc);
                }
            }
            if rng.random_bool(0.01) {
                current = best_partition_shared.lock().unwrap().clone();
                current_count = current.len();
            }
        }
    });
    best_partition_shared.lock().unwrap().clone()
}

fn is_truly_isomorphic_fast(t1: &Arc<Tree>, t2: &Arc<Tree>, leaves: &HashSet<u32>, subset_mask: &FastBitSet) -> bool {
    if leaves.len() <= 1 { return true; }
    let exp1 = build_induced_expansion_fast(t1, leaves, subset_mask);
    let exp2 = build_induced_expansion_fast(t2, leaves, subset_mask);
    match (exp1, exp2) {
        (Some(e1), Some(e2)) => e1 == e2,
        _ => false,
    }
}
