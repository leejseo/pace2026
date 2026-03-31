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
use std::sync::Arc;
use num_traits::Zero;
use rand::prelude::*;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: pace2026_rs <input.nw> [--strategy sa|beam|hybrid] [--time-limit <seconds>]");
        std::process::exit(1);
    }
    
    std::thread::Builder::new()
        .stack_size(128 * 1024 * 1024) // 128MB for massive trees
        .spawn(|| { if let Err(e) = run() { eprintln!("Error: {}", e); } })?
        .join()
        .expect("Thread failed");
    Ok(())
}

fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let input_path = &args[1];
    
    let mut strategy = "hybrid";
    let mut time_limit = 10;
    
    for i in 0..args.len() {
        if args[i] == "--strategy" && i + 1 < args.len() { strategy = &args[i+1]; }
        if args[i] == "--time-limit" && i + 1 < args.len() { 
            time_limit = args[i+1].parse().unwrap_or(10);
        }
    }

    let instance = parse_instance_file(input_path)?;
    let t1 = original_to_tree(&instance.tree1);
    let t2 = original_to_tree(&instance.tree2);
    
    let components = match strategy {
        "beam" => solve_beam(t1, t2, instance.n_leaves, time_limit),
        "sa" => solve_sa(t1, t2, instance.n_leaves, time_limit),
        _ => solve_hybrid(t1, t2, instance.n_leaves, time_limit),
    };
    
    for c in components { println!("{};", render_expansion(&c)); }
    Ok(())
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
    let mut next_t1 = Some(state.tree1.clone());
    let mut next_t2 = Some(state.tree2.clone());
    let mut next_comps = state.cut_components.clone();
    
    if let Some(exp) = subtree_exp {
        for &id in block_ids {
            if let Some(t) = next_t1 { next_t1 = cut_leaf(&t, id); }
            if let Some(t) = next_t2 { next_t2 = cut_leaf(&t, id); }
        }
        next_comps.push(exp);
    } else {
        for &id in block_ids {
            if let Some(t) = next_t1 { next_t1 = cut_leaf(&t, id); }
            if let Some(t) = next_t2 { next_t2 = cut_leaf(&t, id); }
            let exp = state.expansions.get(&id).cloned().unwrap_or(Expansion::Leaf(id));
            next_comps.push(exp);
        }
    }
    
    normalize_state(State {
        tree1: next_t1.unwrap_or(Arc::new(Tree::Leaf(0, num_bigint::BigUint::zero()))),
        tree2: next_t2.unwrap_or(Arc::new(Tree::Leaf(0, num_bigint::BigUint::zero()))),
        expansions: state.expansions.clone(),
        next_id: state.next_id,
        cut_components: next_comps,
    })
}

fn solve_beam(tree1: Arc<Tree>, tree2: Arc<Tree>, n_leaves: u32, limit_seconds: u64) -> Vec<Expansion> {
    let start_time = Instant::now();
    let deadline = start_time + Duration::from_secs(limit_seconds);
    let mut expansions = HashMap::new();
    for i in 1..=n_leaves { expansions.insert(i, Expansion::Leaf(i)); }
    
    let start_state = normalize_state(State {
        tree1, tree2, expansions,
        next_id: n_leaves + 1,
        cut_components: Vec::new(),
    });
    
    let mut best_ans = Vec::new();
    for i in 1..=n_leaves { best_ans.push(Expansion::Leaf(i)); }
    let mut best_count = best_ans.len();

    let mut beam = vec![start_state];

    while !beam.is_empty() && Instant::now() < deadline {
        let next_states: Vec<State> = beam.par_iter().flat_map(|state| {
            if state.tree1.is_leaf() && state.tree2.is_leaf() { return vec![]; }
            if state.cut_components.len() >= best_count { return vec![]; }
            get_candidates(state).into_iter().map(|(ids, exp)| {
                cut_and_normalize(state, &ids, exp)
            }).collect::<Vec<State>>()
        }).collect();
        
        for state in &beam {
            if state.tree1.is_leaf() && state.tree2.is_leaf() {
                let mut ans = state.cut_components.clone();
                let root_id = state.tree1.leaf_id();
                ans.push(state.expansions.get(&root_id).cloned().unwrap_or(Expansion::Leaf(root_id)));
                if ans.len() < best_count {
                    best_count = ans.len(); best_ans = ans;
                }
            }
        }
        
        if next_states.is_empty() { break; }
        let mut sorted = next_states;
        sorted.sort_unstable_by_key(|s| {
            let sc = s.shared_clusters();
            (s.cut_components.len() + s.leaf_count(), -(sc as isize), s.leaf_count())
        });
        
        let mut unique = Vec::new();
        let mut seen = HashSet::new();
        for s in sorted {
            let key = (s.tree1.mask().clone(), s.tree2.mask().clone());
            if !seen.contains(&key) {
                seen.insert(key); unique.push(s);
            }
            if unique.len() >= 60 { break; }
        }
        beam = unique;
    }
    best_ans
}

fn solve_sa(tree1: Arc<Tree>, tree2: Arc<Tree>, n_leaves: u32, limit_seconds: u64) -> Vec<Expansion> {
    let start_time = Instant::now();
    let deadline = start_time + Duration::from_secs(limit_seconds);
    let mut expansions = HashMap::new();
    for i in 1..=n_leaves { expansions.insert(i, Expansion::Leaf(i)); }
    
    let start_state = normalize_state(State {
        tree1, tree2, expansions,
        next_id: n_leaves + 1,
        cut_components: Vec::new(),
    });
    
    let mut best_ans = solve_beam(start_state.tree1.clone(), start_state.tree2.clone(), n_leaves, 1);
    let mut best_count = best_ans.len();
    
    let mut rng = rand::rng();
    let mut current_state = start_state.clone();
    let mut t = 1.0f64;
    let cooling = 0.999f64;
    
    while Instant::now() < deadline && t > 0.01 {
        let candidates = get_candidates(&current_state);
        if candidates.is_empty() { break; }
        
        let (ids, exp) = candidates.choose(&mut rng).unwrap().clone();
        let next_state = cut_and_normalize(&current_state, &ids, exp);
        
        let rollout_ans = greedy_rollout(&next_state, deadline);
        let rollout_count = rollout_ans.len();
        
        if rollout_count < best_count {
            best_count = rollout_count;
            best_ans = rollout_ans;
            current_state = next_state;
        } else {
            let diff = (rollout_count - best_count) as f64;
            if ((-diff / t).exp()) > rng.random::<f64>() {
                current_state = next_state;
            }
        }
        t *= cooling;
        if current_state.tree1.is_leaf() { current_state = start_state.clone(); }
    }
    
    best_ans
}

fn greedy_rollout(state: &State, deadline: Instant) -> Vec<Expansion> {
    let mut curr = state.clone();
    while !curr.tree1.is_leaf() && Instant::now() < deadline {
        let cands = get_candidates(&curr);
        if cands.is_empty() { break; }
        let (ids, exp) = cands[0].clone(); // Best according to get_candidates sorting
        curr = cut_and_normalize(&curr, &ids, exp);
    }
    let mut ans = curr.cut_components.clone();
    let root_id = curr.tree1.leaf_id();
    ans.push(curr.expansions.get(&root_id).cloned().unwrap_or(Expansion::Leaf(root_id)));
    ans
}

fn solve_hybrid(tree1: Arc<Tree>, tree2: Arc<Tree>, n_leaves: u32, limit_seconds: u64) -> Vec<Expansion> {
    // 70% Beam, 30% SA
    let beam_time = (limit_seconds as f64 * 0.7) as u64;
    let sa_time = limit_seconds - beam_time;
    
    let beam_ans = solve_beam(tree1.clone(), tree2.clone(), n_leaves, beam_time);
    // SA could start from beam results or fresh
    let sa_ans = solve_sa(tree1, tree2, n_leaves, sa_time);
    
    if beam_ans.len() < sa_ans.len() { beam_ans } else { sa_ans }
}
