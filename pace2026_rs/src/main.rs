mod tree;
mod state;
mod io;

use std::collections::{HashMap, HashSet};
use crate::tree::{Tree, Expansion, original_to_tree, collect_cherries, cut_leaf, offpath_candidates, get_all_leaves};
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
    
    let components = solve_anytime(t1, t2, instance.n_leaves, time_limit);
    
    for c in components { println!("{};", render_expansion(&c)); }
    Ok(())
}

fn solve_anytime(tree1: Arc<Tree>, tree2: Arc<Tree>, n_leaves: u32, limit_seconds: u64) -> Vec<Expansion> {
    let start_time = Instant::now();
    let deadline = start_time + Duration::from_secs(limit_seconds);
    
    let mut expansions = HashMap::new();
    for i in 1..=n_leaves { expansions.insert(i, Expansion::Leaf(i)); }
    
    let start_state = normalize_state(State {
        tree1, tree2, expansions,
        next_id: n_leaves + 1,
        cut_components: Vec::new(),
        cached_score: (0, 0, 0),
    });

    let initial_ans = greedy_rollout(&start_state, deadline).unwrap_or_else(|| {
        (1..=n_leaves).map(Expansion::Leaf).collect()
    });
    let best_ans = Arc::new(Mutex::new(initial_ans));
    let best_count = Arc::new(Mutex::new(best_ans.lock().unwrap().len()));

    let beam_deadline = start_time + Duration::from_secs(limit_seconds / 5);
    let beam_results = solve_beam_anytime(&start_state, beam_deadline, &best_ans, &best_count);

    (0..rayon::current_num_threads()).into_par_iter().for_each(|i| {
        let seed_state = if !beam_results.is_empty() {
            &beam_results[i % beam_results.len()]
        } else {
            &start_state
        };
        solve_sa_anytime(seed_state, deadline, &best_ans, &best_count);
    });

    let final_ans = best_ans.lock().unwrap().clone();
    final_ans
}

fn solve_beam_anytime(start_state: &State, deadline: Instant, best_ans: &Arc<Mutex<Vec<Expansion>>>, best_count: &Arc<Mutex<usize>>) -> Vec<State> {
    let mut beam = vec![start_state.clone()];
    let mut promising_states = Vec::new();

    while !beam.is_empty() && Instant::now() < deadline {
        let current_best_count = *best_count.lock().unwrap();
        let next_states: Vec<State> = beam.par_iter().flat_map(|state| {
            if state.tree1.is_leaf() && state.tree2.is_leaf() { return vec![]; }
            if state.cut_components.len() >= current_best_count { return vec![]; }
            
            get_candidates(state).into_iter().map(|(ids, exp)| {
                cut_and_normalize(state, &ids, exp)
            }).collect::<Vec<State>>()
        }).collect();
        
        for state in &next_states {
            if state.tree1.is_leaf() && state.tree2.is_leaf() {
                let mut ans = state.cut_components.clone();
                let root_id = state.tree1.leaf_id();
                ans.push(state.expansions.get(&root_id).cloned().unwrap_or(Expansion::Leaf(root_id)));
                
                let mut bc = best_count.lock().unwrap();
                if ans.len() < *bc {
                    *bc = ans.len();
                    *best_ans.lock().unwrap() = ans;
                }
            }
        }
        
        if next_states.is_empty() { break; }
        let mut sorted = next_states;
        sorted.sort_unstable_by_key(|s| s.cached_score);
        
        let mut unique = Vec::new();
        let mut seen = HashSet::new();
        for s in sorted {
            let key = (s.tree1.mask().clone(), s.tree2.mask().clone());
            if !seen.contains(&key) {
                seen.insert(key); unique.push(s);
            }
            if unique.len() >= 100 { break; }
        }
        beam = unique.clone();
        promising_states.extend(unique.into_iter().take(10));
        if promising_states.len() > 100 { promising_states.drain(0..10); }
    }
    promising_states
}

fn solve_sa_anytime(start_state: &State, deadline: Instant, best_ans: &Arc<Mutex<Vec<Expansion>>>, best_count: &Arc<Mutex<usize>>) {
    let mut rng = rand::rng();
    let mut current_state = start_state.clone();
    let mut t = 1.0f64;
    let cooling = 0.99995f64;
    
    while Instant::now() < deadline {
        let current_best_count = *best_count.lock().unwrap();
        let candidates = get_candidates(&current_state);
        if candidates.is_empty() { 
            current_state = start_state.clone(); 
            continue; 
        }
        
        let (ids, exp) = candidates.choose(&mut rng).unwrap().clone();
        let next_state = cut_and_normalize(&current_state, &ids, exp);
        
        let next_score = next_state.cached_score.0;
        
        if next_score <= current_best_count + 2 {
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
        if t < 0.01 { 
            t = 1.0; 
            current_state = start_state.clone(); 
        }
    }
}

fn build_sub_expansion(tree: &Arc<Tree>, expansions: &HashMap<u32, Expansion>) -> Expansion {
    match tree.as_ref() {
        Tree::Leaf(id, _) => expansions.get(id).cloned().unwrap_or(Expansion::Leaf(*id)),
        Tree::Node(l, r, _, _) => {
            Expansion::Node(Box::new(build_sub_expansion(l, expansions)), 
                            Box::new(build_sub_expansion(r, expansions)))
        }
    }
}

fn get_candidates(state: &State) -> Vec<(Vec<u32>, Option<Expansion>)> {
    let mut c1 = HashSet::new();
    collect_cherries(&state.tree1, &mut c1);
    let mut c2 = HashSet::new();
    collect_cherries(&state.tree2, &mut c2);
    
    let mut candidates = Vec::new();
    for &(a, b) in c1.difference(&c2) {
        candidates.push((vec![a], None));
        candidates.push((vec![b], None));
        for sub in offpath_candidates(&state.tree2, a, b) {
            candidates.push((get_all_leaves(&sub), Some(build_sub_expansion(&sub, &state.expansions))));
        }
    }
    for &(a, b) in c2.difference(&c1) {
        candidates.push((vec![a], None));
        candidates.push((vec![b], None));
        for sub in offpath_candidates(&state.tree1, a, b) {
            candidates.push((get_all_leaves(&sub), Some(build_sub_expansion(&sub, &state.expansions))));
        }
    }
    if candidates.is_empty() {
        let all = get_all_leaves(&state.tree1);
        for &leaf in all.iter().take(15) { candidates.push((vec![leaf], None)); }
    }
    candidates.truncate(30);
    candidates
}

fn cut_and_normalize(state: &State, block_ids: &[u32], subtree_exp: Option<Expansion>) -> State {
    let mut next_t1 = state.tree1.clone();
    let mut next_t2 = state.tree2.clone();
    let mut next_comps = state.cut_components.clone();
    
    if let Some(exp) = subtree_exp {
        for &id in block_ids {
            if let Some(t) = cut_leaf(&next_t1, id) { next_t1 = t; }
            if let Some(t) = cut_leaf(&next_t2, id) { next_t2 = t; }
        }
        next_comps.push(exp);
    } else {
        for &id in block_ids {
            if let Some(t) = cut_leaf(&next_t1, id) { next_t1 = t; }
            if let Some(t) = cut_leaf(&next_t2, id) { next_t2 = t; }
            if let Some(exp) = state.expansions.get(&id) { next_comps.push(exp.clone()); }
            else { next_comps.push(Expansion::Leaf(id)); }
        }
    }
    normalize_state(State {
        tree1: next_t1,
        tree2: next_t2,
        expansions: state.expansions.clone(),
        next_id: state.next_id,
        cut_components: next_comps,
        cached_score: (0, 0, 0),
    })
}

fn greedy_rollout(state: &State, deadline: Instant) -> Option<Vec<Expansion>> {
    let mut curr = state.clone();
    while !curr.tree1.is_leaf() && Instant::now() < deadline {
        let cands = get_candidates(&curr);
        if cands.is_empty() { break; }
        let (ids, exp) = cands[0].clone();
        curr = cut_and_normalize(&curr, &ids, exp);
    }
    
    if !curr.tree1.is_leaf() || !curr.tree2.is_leaf() {
        return None;
    }
    
    let mut ans = curr.cut_components.clone();
    let root_id = curr.tree1.leaf_id();
    ans.push(curr.expansions.get(&root_id).cloned().unwrap_or(Expansion::Leaf(root_id)));
    Some(ans)
}
