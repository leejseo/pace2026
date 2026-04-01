mod tree;
mod state;
mod io;

use std::collections::{HashSet, HashMap};
use crate::tree::{OriginalNode, original_to_tree, get_hash_map, Expansion};
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
    let safe_limit = if time_limit > 15 { time_limit - 10 } else { time_limit };
    if let Err(e) = run(safe_limit) { eprintln!("Error: {}", e); }
    Ok(())
}

fn run(time_limit: u64) -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let instance = parse_instance_file(&args[1])?;
    let mut labels = HashSet::new(); get_labels(&instance.tree1, &mut labels);
    let n_leaves = labels.len() as u32;
    
    let partition = solve_maf_alns_sa_final(&instance.tree1, &instance.tree2, get_initial_partition(&instance.tree1, &instance.tree2, n_leaves, &labels), time_limit);
    
    let out = stdout();
    let mut writer = BufWriter::new(out.lock());
    for leaf_set in partition {
        if let Some(exp) = build_induced_expansion(&instance.tree1, &leaf_set) {
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

fn build_induced_expansion(node: &OriginalNode, leaves: &HashSet<u32>) -> Option<Expansion> {
    if let Some(id) = node.label { 
        return if leaves.contains(&id) { Some(Expansion::Leaf(id)) } else { None }; 
    }
    let l = build_induced_expansion(node.left.as_ref().unwrap(), leaves);
    let r = build_induced_expansion(node.right.as_ref().unwrap(), leaves);
    match (l, r) { 
        (Some(tl), Some(tr)) => Some(Expansion::new_node(tl, tr)), 
        (Some(t), None) | (None, Some(t)) => Some(t), 
        (None, None) => None 
    }
}

fn get_initial_partition(orig1: &OriginalNode, orig2: &OriginalNode, n_leaves: u32, all_labels: &HashSet<u32>) -> Vec<HashSet<u32>> {
    let t1 = original_to_tree(orig1, n_leaves);
    let t2 = original_to_tree(orig2, n_leaves);
    let mut sub1 = HashMap::new(); get_hash_map(&t1, &mut sub1);
    let mut sub2 = HashMap::new(); get_hash_map(&t2, &mut sub2);
    let mut common = Vec::new();
    for (hash, node1) in sub1 {
        if sub2.contains_key(&hash) {
            let leaves = crate::tree::get_all_leaves(&node1);
            if leaves.len() > 1 {
                let s: HashSet<u32> = leaves.into_iter().collect();
                if is_truly_isomorphic(orig1, orig2, &s) { common.push(s); }
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
    result
}

fn solve_maf_alns_sa_final(orig1: &OriginalNode, orig2: &OriginalNode, initial: Vec<HashSet<u32>>, limit_seconds: u64) -> Vec<HashSet<u32>> {
    let start_time = Instant::now();
    let deadline = start_time + Duration::from_secs(limit_seconds);
    let best_partition_shared = Arc::new(Mutex::new(initial.clone()));
    let best_count_shared = Arc::new(Mutex::new(initial.len()));

    (0..rayon::current_num_threads()).into_par_iter().for_each(|_| {
        let mut rng = rand::rng();
        let mut current = initial.clone();
        let mut current_count = current.len();
        let mut temp = 1.0;

        while Instant::now() < deadline {
            let mut next = current.clone();
            let removed = if rng.random_bool(0.5) {
                let count = (next.len() as f64 * rng.random_range(0.05..0.20)) as usize;
                let mut pool = HashSet::new();
                for _ in 0..count.max(1) { if !next.is_empty() { pool.extend(next.swap_remove(rng.random_range(0..next.len()))); } }
                pool
            } else {
                next.sort_by_key(|s| s.len());
                let mut pool = HashSet::new();
                for _ in 0..(next.len() / 4).max(1) { if !next.is_empty() { pool.extend(next.remove(0)); } }
                pool
            };

            if !removed.is_empty() {
                let mut pool_vec: Vec<u32> = removed.iter().cloned().collect();
                pool_vec.shuffle(&mut rng);
                while !pool_vec.is_empty() {
                    let mut comp = HashSet::new(); comp.insert(pool_vec.pop().unwrap());
                    let mut i = 0;
                    while i < pool_vec.len() {
                        comp.insert(pool_vec[i]);
                        if is_truly_isomorphic(orig1, orig2, &comp) { pool_vec.swap_remove(i); }
                        else { comp.remove(&pool_vec[i]); i += 1; }
                        if comp.len() > 500 { break; }
                    }
                    next.push(comp);
                }
            }

            for _ in 0..100 {
                if next.len() <= 1 { break; }
                let i = rng.random_range(0..next.len());
                let j = (i + rng.random_range(1..next.len())) % next.len();
                let mut merged = next[i].clone(); merged.extend(&next[j]);
                if is_truly_isomorphic(orig1, orig2, &merged) {
                    let (f, s) = if i > j { (i, j) } else { (j, i) };
                    next.remove(f); next.remove(s); next.push(merged);
                }
            }

            let next_count = next.len();
            let delta = next_count as f64 - current_count as f64;
            if delta < 0.0 || rng.random_bool((-delta / temp).exp().min(1.0)) {
                current = next; current_count = next_count;
                let mut bc = best_count_shared.lock().unwrap();
                if current_count < *bc {
                    *bc = current_count;
                    *best_partition_shared.lock().unwrap() = current.clone();
                    eprintln!("New best (ALNS): {}", *bc);
                }
            }
            temp *= 0.9997;
            if rng.random_bool(0.005) {
                current = best_partition_shared.lock().unwrap().clone();
                current_count = current.len();
                temp = 1.0;
            }
        }
    });
    best_partition_shared.lock().unwrap().clone()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(id: u32) -> Box<OriginalNode> {
        Box::new(OriginalNode { left: None, right: None, label: Some(id) })
    }

    fn node(l: Box<OriginalNode>, r: Box<OriginalNode>) -> Box<OriginalNode> {
        Box::new(OriginalNode { left: Some(l), right: Some(r), label: None })
    }

    #[test]
    fn test_is_truly_isomorphic_simple() {
        let t1 = node(leaf(1), node(leaf(2), leaf(3))); // (1, (2, 3))
        let t2 = node(node(leaf(1), leaf(2)), leaf(3)); // ((1, 2), 3)
        
        let mut s = HashSet::new(); s.insert(2); s.insert(3);
        assert!(is_truly_isomorphic(&t1, &t2, &s));
        
        let mut s = HashSet::new(); s.insert(1); s.insert(2); s.insert(3);
        assert!(!is_truly_isomorphic(&t1, &t2, &s));
    }

    #[test]
    fn test_build_induced_expansion_suppression() {
        let t1 = node(leaf(1), node(leaf(2), leaf(3))); // (1, (2, 3))
        let mut s = HashSet::new(); s.insert(1); s.insert(3);
        let exp = build_induced_expansion(&t1, &s).unwrap();
        
        if let Expansion::Node(l, r, _) = exp {
            let mut ids = vec![l.hash_val(), r.hash_val()];
            ids.sort();
            assert_eq!(ids, vec![1, 3]);
        } else {
            panic!("Expected node expansion");
        }
    }
}
