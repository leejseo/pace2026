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

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: pace2026_rs <input.nw>");
        std::process::exit(1);
    }
    let input_path = &args[1];
    let instance = parse_instance_file(input_path)?;
    
    let t1 = original_to_tree(&instance.tree1);
    let t2 = original_to_tree(&instance.tree2);
    
    let components = solve_beam(t1, t2, instance.n_leaves, 5);
    for c in components {
        println!("{};", render_expansion(&c));
    }
    
    Ok(())
}

fn build_sub_expansion(tree: &Tree, expansions: &HashMap<u32, Expansion>) -> Expansion {
    match tree {
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
            let leaves = get_all_leaves(&sub);
            let exp = build_sub_expansion(&sub, &state.expansions);
            candidates.push((leaves, Some(exp)));
        }
    }
    for &(a, b) in c2.difference(&c1) {
        candidates.push((vec![a], None));
        candidates.push((vec![b], None));
        for sub in offpath_candidates(&state.tree1, a, b) {
            let leaves = get_all_leaves(&sub);
            let exp = build_sub_expansion(&sub, &state.expansions);
            candidates.push((leaves, Some(exp)));
        }
    }
    
    if candidates.is_empty() {
        let all = get_all_leaves(&state.tree1);
        for &leaf in all.iter().take(15) {
            candidates.push((vec![leaf], None));
        }
    }
    
    candidates.truncate(20);
    candidates
}

fn cut_and_normalize(state: &State, block_ids: &[u32], subtree_exp: Option<Expansion>) -> State {
    let mut next_t1 = state.tree1.clone();
    let mut next_t2 = state.tree2.clone();
    let mut next_comps = state.cut_components.clone();
    
    if let Some(exp) = subtree_exp {
        // Macro-cut: cut all leaves but add only ONE combined component
        for &id in block_ids {
            if let Some(t) = cut_leaf(&next_t1, id) { next_t1 = t; }
            if let Some(t) = cut_leaf(&next_t2, id) { next_t2 = t; }
        }
        next_comps.push(exp);
    } else {
        // Single block cut
        for &id in block_ids {
            if let Some(t) = cut_leaf(&next_t1, id) { next_t1 = t; }
            if let Some(t) = cut_leaf(&next_t2, id) { next_t2 = t; }
            if let Some(exp) = state.expansions.get(&id) {
                next_comps.push(exp.clone());
            }
        }
    }
    
    normalize_state(State {
        tree1: next_t1,
        tree2: next_t2,
        expansions: state.expansions.clone(),
        next_id: state.next_id,
        cut_components: next_comps,
    })
}

fn solve_beam(tree1: Tree, tree2: Tree, n_leaves: u32, limit_seconds: u64) -> Vec<Expansion> {
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
                if let Some(exp) = state.expansions.get(&root_id) {
                    ans.push(exp.clone());
                }
                if ans.len() < best_count {
                    best_count = ans.len();
                    best_ans = ans;
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
                seen.insert(key);
                unique.push(s);
            }
            if unique.len() >= 40 { break; }
        }
        beam = unique;
    }
    
    best_ans
}
