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
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

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
    let mut subset_mask = FastBitSet::new(n_leaves * 3);
    for leaf_set in partition {
        subset_mask.clear_all();
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

fn compute_induced_hash(tree: &Arc<Tree>, leaves: &HashSet<u32>, subset_mask: &FastBitSet) -> Option<u64> {
    if !tree.mask().intersects(subset_mask) { return None; }
    match tree.as_ref() {
        Tree::Leaf(id, _) => {
            if leaves.contains(id) {
                let mut hasher = DefaultHasher::new();
                id.hash(&mut hasher);
                Some(hasher.finish())
            } else {
                None
            }
        },
        Tree::Node(l, r, _, _) => {
            let left_h = compute_induced_hash(l, leaves, subset_mask);
            let right_h = compute_induced_hash(r, leaves, subset_mask);
            match (left_h, right_h) {
                (Some(h1), Some(h2)) => {
                    let mut hasher = DefaultHasher::new();
                    if h1 < h2 {
                        h1.hash(&mut hasher);
                        h2.hash(&mut hasher);
                    } else {
                        h2.hash(&mut hasher);
                        h1.hash(&mut hasher);
                    }
                    Some(hasher.finish())
                },
                (Some(h), None) | (None, Some(h)) => Some(h),
                (None, None) => None
            }
        }
    }
}

fn is_truly_isomorphic_fast(t1: &Arc<Tree>, t2: &Arc<Tree>, leaves: &HashSet<u32>, subset_mask: &FastBitSet) -> bool {
    if leaves.len() <= 1 { return true; }
    let h1 = compute_induced_hash(t1, leaves, subset_mask);
    let h2 = compute_induced_hash(t2, leaves, subset_mask);
    h1 == h2 && h1.is_some()
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
        
        let mut stubbornness = vec![0; current.len()];
        let mut age = vec![0; current.len()];
        let mut subset_mask = FastBitSet::new(n_leaves * 3);

        while Instant::now() < deadline {
            let mut next = current.clone();
            
            let elapsed = start_time.elapsed().as_secs_f64();
            let progress = elapsed / limit_seconds as f64;

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
            let strategy = rng.random_range(0..4);
            let removed = match strategy {
                0 => {
                    let count = (next.len() as f64 * rng.random_range(destroy_base..destroy_max)) as usize;
                    let mut pool = HashSet::new();
                    for _ in 0..count.max(1) { 
                        if !next.is_empty() { 
                            let idx = rng.random_range(0..next.len());
                            if age[idx] < 50 || next[idx].len() < 20 {
                                pool.extend(next.swap_remove(idx)); 
                                stubbornness.swap_remove(idx);
                                age.swap_remove(idx);
                            }
                        } 
                    }
                    pool
                }
                1 => {
                    let mut zipped: Vec<_> = next.into_iter().zip(stubbornness.into_iter()).zip(age.into_iter()).collect();
                    zipped.sort_by_key(|((s, _), _)| s.len());
                    next = zipped.into_iter().map(|((s, _), _)| s).collect();
                    stubbornness = vec![0; next.len()];
                    age = vec![0; next.len()];
                    let mut pool = HashSet::new();
                    let mut i = 0;
                    let target = (next.len() / 4).max(1);
                    let mut removed_count = 0;
                    while i < next.len() && removed_count < target {
                        if age[i] < 50 || next[i].len() < 20 {
                            pool.extend(next.remove(i));
                            stubbornness.remove(i);
                            age.remove(i);
                            removed_count += 1;
                        } else {
                            i += 1;
                        }
                    }
                    pool
                }
                2 => {
                    let mut zipped: Vec<_> = next.into_iter().zip(stubbornness.into_iter()).zip(age.into_iter()).collect();
                    zipped.sort_by_key(|((s, _), _)| std::cmp::Reverse(s.len()));
                    next = zipped.into_iter().map(|((s, _), _)| s).collect();
                    stubbornness = vec![0; next.len()];
                    age = vec![0; next.len()];
                    let mut pool = HashSet::new();
                    let mut i = 0;
                    let target = (rng.random_range(1..=2)).min(next.len());
                    let mut removed_count = 0;
                    while i < next.len() && removed_count < target {
                        if age[i] < 50 || next[i].len() < 20 {
                            pool.extend(next.remove(i));
                            stubbornness.remove(i);
                            age.remove(i);
                            removed_count += 1;
                        } else {
                            i += 1;
                        }
                    }
                    pool
                }
                _ => {
                    let mut zipped: Vec<_> = next.into_iter().zip(stubbornness.into_iter()).zip(age.into_iter()).collect();
                    zipped.sort_by_key(|((_, stub), _)| std::cmp::Reverse(*stub));
                    next = zipped.into_iter().map(|((s, _), _)| s).collect();
                    stubbornness = vec![0; next.len()];
                    age = vec![0; next.len()];
                    let count = (next.len() as f64 * rng.random_range(destroy_base..destroy_max)) as usize;
                    let mut pool = HashSet::new();
                    let mut i = 0;
                    let mut removed_count = 0;
                    while i < next.len() && removed_count < count.max(1) {
                        if age[i] < 50 || next[i].len() < 20 {
                            pool.extend(next.remove(i));
                            stubbornness.remove(i);
                            age.remove(i);
                            removed_count += 1;
                        } else {
                            i += 1;
                        }
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
                    subset_mask.clear_all();
                    for &l in &comp { subset_mask.set(l); }
                    
                    let mut i = 0;
                    while i < pool_vec.len() {
                        comp.insert(pool_vec[i]);
                        subset_mask.set(pool_vec[i]);
                        
                        if is_truly_isomorphic_fast(t1, t2, &comp, &subset_mask) { 
                            pool_vec.remove(i); 
                        } else { 
                            comp.remove(&pool_vec[i]); 
                            subset_mask.clear(pool_vec[i]);
                            i += 1; 
                        }
                        if comp.len() > 500 { break; }
                    }
                    next.push(comp);
                    stubbornness.push(0);
                    age.push(0);
                }
            }

            // SHIFT OPERATOR 
            if rng.random_bool(0.3) && next.len() > 1 {
                for _ in 0..50 {
                    let from_idx = rng.random_range(0..next.len());
                    if next[from_idx].len() <= 1 { continue; }
                    if stubbornness[from_idx] < 2 && rng.random_bool(0.8) { continue; }

                    let to_idx = rng.random_range(0..next.len());
                    if from_idx == to_idx { continue; }
                    
                    let leaves: Vec<u32> = next[from_idx].iter().cloned().collect();
                    let leaf_to_move = *leaves.choose(&mut rng).unwrap();
                    
                    let mut new_to = next[to_idx].clone();
                    new_to.insert(leaf_to_move);
                    
                    subset_mask.clear_all();
                    for &l in &new_to { subset_mask.set(l); }
                    
                    if is_truly_isomorphic_fast(t1, t2, &new_to, &subset_mask) {
                        let mut new_from = next[from_idx].clone();
                        new_from.remove(&leaf_to_move);
                        subset_mask.clear_all();
                        for &l in &new_from { subset_mask.set(l); }
                        
                        if is_truly_isomorphic_fast(t1, t2, &new_from, &subset_mask) {
                            next[to_idx] = new_to;
                            next[from_idx] = new_from;
                            stubbornness[to_idx] = 0;
                            stubbornness[from_idx] = 0;
                            age[to_idx] = 0;
                            age[from_idx] = 0;
                        }
                    }
                }
            }

            // LOCAL MERGE
            if next.len() < 500 || progress > 0.9 {
                let mut changed = true;
                while changed && Instant::now() < deadline {
                    changed = false;
                    let mut i = 0;
                    while i < next.len() && Instant::now() < deadline {
                        let mut j = i + 1;
                        let mut merged_flag = false;
                        while j < next.len() {
                            let mut merged = next[i].clone(); merged.extend(&next[j]);
                            subset_mask.clear_all();
                            for &l in &merged { subset_mask.set(l); }
                            
                            if is_truly_isomorphic_fast(t1, t2, &merged, &subset_mask) {
                                next.remove(j);
                                stubbornness.remove(j);
                                age.remove(j);
                                next[i] = merged;
                                stubbornness[i] = 0;
                                age[i] = 0;
                                changed = true;
                                merged_flag = true;
                                break;
                            } else {
                                stubbornness[i] += 1;
                                stubbornness[j] += 1;
                            }
                            j += 1;
                        }
                        if !merged_flag { i += 1; }
                    }
                }
            } else {
                for _ in 0..2000 {
                    if next.len() <= 1 { break; }
                    let i = rng.random_range(0..next.len());
                    let j = (i + rng.random_range(1..next.len())) % next.len();
                    if i == j { continue; }
                    
                    let mut merged = next[i].clone(); merged.extend(&next[j]);
                    subset_mask.clear_all();
                    for &l in &merged { subset_mask.set(l); }
                    
                    if is_truly_isomorphic_fast(t1, t2, &merged, &subset_mask) {
                        let (f, s) = if i > j { (i, j) } else { (j, i) };
                        next.remove(f); stubbornness.remove(f); age.remove(f);
                        next.remove(s); stubbornness.remove(s); age.remove(s);
                        next.push(merged); stubbornness.push(0); age.push(0);
                    } else {
                        stubbornness[i] += 1;
                        stubbornness[j] += 1;
                    }
                }
            }

            let next_count = next.len();
            let delta = next_count as f64 - current_count as f64;
            
            if delta <= 0.0 || (allow_sa && rng.random_bool((-delta / (dynamic_temp * t_mult)).exp().clamp(0.0, 1.0))) {
                for i in 0..next.len() {
                    if current.contains(&next[i]) {
                        age[i] += 1;
                    } else {
                        age[i] = 0;
                    }
                }
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
                stubbornness = vec![0; current_count];
                age = vec![0; current_count];
            }
        }
    });
    best_partition_shared.lock().unwrap().clone()
}
