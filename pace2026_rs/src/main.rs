mod tree;
mod state;
mod io;

use std::collections::{HashMap, HashSet};
use crate::tree::{Tree, Expansion, original_to_tree, collect_cherries, cut_leaf, get_all_leaves};
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
        tree1: tree1.clone(), tree2: tree2.clone(), expansions,
        next_id: n_leaves + 1, cut_components: Vec::new(), cached_score: (0, 0, 0),
    });

    let initial_ans = (1..=n_leaves).map(Expansion::Leaf).collect();
    let best_ans = Arc::new(Mutex::new(initial_ans));
    let best_count = Arc::new(Mutex::new(n_leaves as usize));

    (0..rayon::current_num_threads()).into_par_iter().for_each(|_| {
        let mut rng = rand::rng();
        while Instant::now() < deadline {
            solve_sa_anytime(&start_state, deadline, &best_ans, &best_count);
        }
    });

    best_ans.lock().unwrap().clone()
}

fn solve_sa_anytime(start_state: &State, deadline: Instant, best_ans: &Arc<Mutex<Vec<Expansion>>>, best_count: &Arc<Mutex<usize>>) {
    let mut rng = rand::rng();
    let mut curr = start_state.clone();
    let mut t = 1.0f64;
    
    while Instant::now() < deadline && !curr.tree1.is_leaf() {
        let current_best = *best_count.lock().unwrap();
        let leaves = get_all_leaves(&curr.tree1);
        if leaves.is_empty() { break; }
        
        let &leaf_id = leaves.choose(&mut rng).unwrap();
        let mut next_t1 = cut_leaf(&curr.tree1, leaf_id);
        let mut next_t2 = cut_leaf(&curr.tree2, leaf_id);
        let mut next_comps = curr.cut_components.clone();
        next_comps.push(curr.expansions.get(&leaf_id).cloned().unwrap_or(Expansion::Leaf(leaf_id)));
        
        let next_state = normalize_state(State {
            tree1: next_t1.unwrap_or(Arc::new(Tree::Leaf(0, Zero::zero()))),
            tree2: next_t2.unwrap_or(Arc::new(Tree::Leaf(0, Zero::zero()))),
            expansions: curr.expansions.clone(),
            next_id: curr.next_id,
            cut_components: next_comps,
            cached_score: (0, 0, 0),
        });

        let score = next_state.cut_components.len() + next_state.tree1.size();
        if score < current_best + 5 {
            if next_state.tree1.is_leaf() && next_state.tree2.is_leaf() {
                let mut ans = next_state.cut_components.clone();
                let rid = next_state.tree1.leaf_id();
                ans.push(next_state.expansions.get(&rid).cloned().unwrap_or(Expansion::Leaf(rid)));
                let mut bc = best_count.lock().unwrap();
                if ans.len() < *bc { *bc = ans.len(); *best_ans.lock().unwrap() = ans; }
            }
            curr = next_state;
        }
        t *= 0.9999;
    }
}
