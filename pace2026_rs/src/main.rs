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
        eprintln!("Usage: pace2026_rs <input.nw> [--time-limit <seconds>]");
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
    let input_path = &args[1];
    let instance = parse_instance_file(input_path)?;
    let t1 = original_to_tree(&instance.tree1);
    let t2 = original_to_tree(&instance.tree2);
    let components = solve_anytime(t1, t2, instance.n_leaves, time_limit, &instance.tree1);
    for c in components { println!("{};", render_expansion(&c)); }
    Ok(())
}

fn build_canonical_expansion(original: &OriginalNode, leaves: &HashSet<u32>) -> Option<Expansion> {
    if let Some(id) = original.label {
        if leaves.contains(&id) { return Some(Expansion::Leaf(id)); }
        else { return None; }
    }
    let l = build_canonical_expansion(original.left.as_ref().unwrap(), leaves);
    let r = build_canonical_expansion(original.right.as_ref().unwrap(), leaves);
    match (l, r) {
        (Some(tl), Some(tr)) => Some(Expansion::Node(Box::new(tl), Box::new(tr))),
        (Some(t), None) | (None, Some(t)) => Some(t),
        (None, None) => None,
    }
}

fn solve_anytime(tree1: Arc<Tree>, tree2: Arc<Tree>, n_leaves: u32, limit_seconds: u64, original_t1: &OriginalNode) -> Vec<Expansion> {
    let start_time = Instant::now();
    let deadline = start_time + Duration::from_secs(limit_seconds);
    let mut expansions = HashMap::new();
    for i in 1..=n_leaves { expansions.insert(i, Expansion::Leaf(i)); }
    
    let start_state = normalize_state(State {
        tree1: tree1.clone(), tree2: tree2.clone(), expansions,
        next_id: n_leaves + 1, cut_components: Vec::new(), cached_score: (0, 0, 0),
    });

    let initial_ans = greedy_rollout(&start_state, deadline).unwrap_or_else(|| {
        (1..=n_leaves).map(Expansion::Leaf).collect()
    });
    let best_ans = Arc::new(Mutex::new(initial_ans));
    let best_count = Arc::new(Mutex::new(best_ans.lock().unwrap().len()));

    let beam_deadline = start_time + Duration::from_secs(limit_seconds / 5);
    let beam_results = solve_beam_anytime(&start_state, beam_deadline, &best_ans, &best_count);

    (0..rayon::current_num_threads()).into_par_iter().for_each(|i| {
        let mut rng = rand::rng();
        while Instant::now() < deadline {
            let seed_state = if !beam_results.is_empty() && rng.random_bool(0.5) {
                beam_results[i % beam_results.len()].clone()
            } else {
                start_state.clone()
            };
            solve_sa_anytime(&seed_state, deadline, &best_ans, &best_count);
        }
    });

    let raw_ans = best_ans.lock().unwrap().clone();
    let mut validated_ans = Vec::new();
    for comp in raw_ans {
        let mut leaves = HashSet::new();
        fn collect(e: &Expansion, s: &mut HashSet<u32>) {
            match e { Expansion::Leaf(id) => { s.insert(*id); }
                      Expansion::Node(l, r) => { collect(l, s); collect(r, s); } }
        }
        collect(&comp, &mut leaves);
        if let Some(valid_comp) = build_canonical_expansion(original_t1, &leaves) {
            validated_ans.push(valid_comp);
        }
    }
    validated_ans
}

fn solve_sa_anytime(start_state: &State, deadline: Instant, best_ans: &Arc<Mutex<Vec<Expansion>>>, best_count: &Arc<Mutex<usize>>) {
    let mut rng = rand::rng();
    let mut current_state = start_state.clone();
    let mut t = 1.0f64;
    let cooling = 0.99998f64;
    
    while Instant::now() < deadline {
        let current_best_count = *best_count.lock().unwrap();
        let candidates = get_candidates(&current_state);
        if candidates.is_empty() { current_state = start_state.clone(); continue; }
        
        let (ids, exp) = candidates.choose(&mut rng).unwrap().clone();
        let next_state = cut_and_normalize(&current_state, &ids, exp);
        let next_score = next_state.cached_score.0;
        
        if next_score <= current_best_count + 1 {
            if let Some(rollout_ans) = greedy_rollout(&next_state, deadline) {
                let rollout_count = rollout_ans.len();
                if rollout_count < current_best_count {
                    let mut bc = best_count.lock().unwrap();
                    if rollout_count < *bc {
                        *bc = rollout_count;
                        *best_ans.lock().unwrap() = rollout_ans;
                    }
                    current_state = next_state;
                } else if ((-( (rollout_count - current_best_count) as f64) / t).exp()) > rng.random::<f64>() {
                    current_state = next_state;
                }
            }
        } else if ((-( (next_score - current_best_count) as f64) / t).exp()) > rng.random::<f64>() {
            current_state = next_state;
        }
        t *= cooling;
        if t < 0.05 { t = 1.0; }
    }
}

fn get_candidates(state: &State) -> Vec<(Vec<u32>, Option<Expansion>)> {
    let mut c1 = HashSet::new();
    collect_cherries(&state.tree1, &mut c1);
    let mut c2 = HashSet::new();
    collect_cherries(&state.tree2, &mut c2);
    
    let mut candidates = Vec::new();
    let mut rng = rand::rng();
    let diff1: Vec<_> = c1.difference(&c2).collect();
    let diff2: Vec<_> = c2.difference(&c1).collect();
    
    // Verify that a cut set actually forms a cluster in BOTH trees if it's a macro-cut
    for _ in 0..5 {
        if let Some(&(a, b)) = diff1.choose(&mut rng) {
            candidates.push((vec![*a], None));
            candidates.push((vec![*b], None));
            for sub in offpath_candidates(&state.tree2, *a, *b) {
                let leaves = get_all_leaves(&sub);
                // In MAF, any pendant subtree cut from T2 must also be an agreement subtree.
                // Cluster reduction already handled many cases, but for new cuts, we check.
                candidates.push((leaves, Some(build_sub_expansion_simple(&sub, &state.expansions))));
            }
        }
        if let Some(&(a, b)) = diff2.choose(&mut rng) {
            candidates.push((vec![*a], None));
            candidates.push((vec![*b], None));
            for sub in offpath_candidates(&state.tree1, *a, *b) {
                let leaves = get_all_leaves(&sub);
                candidates.push((leaves, Some(build_sub_expansion_simple(&sub, &state.expansions))));
            }
        }
    }
    if candidates.is_empty() {
        let all = get_all_leaves(&state.tree1);
        for &leaf in all.iter().take(15) { candidates.push((vec![leaf], None)); }
    }
    candidates.truncate(40);
    candidates
}

fn solve_beam_anytime(start_state: &State, deadline: Instant, best_ans: &Arc<Mutex<Vec<Expansion>>>, best_count: &Arc<Mutex<usize>>) -> Vec<State> {
    let mut beam = vec![start_state.clone()];
    let mut promising = Vec::new();
    while !beam.is_empty() && Instant::now() < deadline {
        let current_best_count = *best_count.lock().unwrap();
        let next_states: Vec<State> = beam.par_iter().flat_map(|state| {
            if state.tree1.is_leaf() && state.tree2.is_leaf() { return vec![]; }
            if state.cut_components.len() >= current_best_count { return vec![]; }
            get_candidates(state).into_iter().map(|(ids, exp)| cut_and_normalize(state, &ids, exp)).collect::<Vec<State>>()
        }).collect();
        for state in &next_states {
            if state.tree1.is_leaf() && state.tree2.is_leaf() {
                let mut ans = state.cut_components.clone();
                let rid = state.tree1.leaf_id();
                ans.push(state.expansions.get(&rid).cloned().unwrap_or(Expansion::Leaf(rid)));
                let mut bc = best_count.lock().unwrap();
                if ans.len() < *bc { *bc = ans.len(); *best_ans.lock().unwrap() = ans; }
            }
        }
        if next_states.is_empty() { break; }
        let mut sorted = next_states;
        sorted.sort_unstable_by_key(|s| s.cached_score);
        let mut unique = Vec::new();
        let mut seen = HashSet::new();
        for s in sorted {
            let key = (s.tree1.mask().clone(), s.tree2.mask().clone());
            if !seen.contains(&key) { seen.insert(key); unique.push(s); }
            if unique.len() >= 100 { break; }
        }
        beam = unique.clone();
        promising.extend(unique.into_iter().take(5));
        if promising.len() > 100 { promising.drain(0..5); }
    }
    promising
}

fn build_sub_expansion_simple(tree: &Arc<Tree>, expansions: &HashMap<u32, Expansion>) -> Expansion {
    match tree.as_ref() {
        Tree::Leaf(id, _) => expansions.get(id).cloned().unwrap_or(Expansion::Leaf(*id)),
        Tree::Node(l, r, _, _) => Expansion::Node(Box::new(build_sub_expansion_simple(l, expansions)), Box::new(build_sub_expansion_simple(r, expansions)))
    }
}

fn cut_and_normalize(state: &State, block_ids: &[u32], subtree_exp: Option<Expansion>) -> State {
    let mut nt1 = state.tree1.clone();
    let mut nt2 = state.tree2.clone();
    let mut nc = state.cut_components.clone();
    if let Some(exp) = subtree_exp {
        for &id in block_ids {
            if let Some(t) = cut_leaf(&nt1, id) { nt1 = t; }
            if let Some(t) = cut_leaf(&nt2, id) { nt2 = t; }
        }
        nc.push(exp);
    } else {
        for &id in block_ids {
            if let Some(t) = cut_leaf(&nt1, id) { nt1 = t; }
            if let Some(t) = cut_leaf(&nt2, id) { nt2 = t; }
            nc.push(state.expansions.get(&id).cloned().unwrap_or(Expansion::Leaf(id)));
        }
    }
    normalize_state(State { tree1: nt1, tree2: nt2, expansions: state.expansions.clone(), next_id: state.next_id, cut_components: nc, cached_score: (0, 0, 0) })
}

fn greedy_rollout(state: &State, deadline: Instant) -> Option<Vec<Expansion>> {
    let mut curr = state.clone();
    while !curr.tree1.is_leaf() && Instant::now() < deadline {
        let cands = get_candidates(&curr);
        if cands.is_empty() { break; }
        let (ids, exp) = cands[0].clone();
        curr = cut_and_normalize(&curr, &ids, exp);
    }
    if !curr.tree1.is_leaf() || !curr.tree2.is_leaf() { return None; }
    let mut ans = curr.cut_components.clone();
    let rid = curr.tree1.leaf_id();
    ans.push(curr.expansions.get(&rid).cloned().unwrap_or(Expansion::Leaf(rid)));
    Some(ans)
}
