mod tree;
mod state;
mod io;

use std::collections::{HashMap, HashSet};
use crate::tree::{ArenaTree, Expansion, original_to_arena, collect_cherries, cut_leaf_arena, get_all_meta_leaves, Node};
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
    
    let mut t1 = ArenaTree::new();
    let r1 = original_to_arena(&instance.tree1, &mut t1);
    t1.root = r1;
    
    let mut t2 = ArenaTree::new();
    let r2 = original_to_arena(&instance.tree2, &mut t2);
    t2.root = r2;
    
    let components = solve_beam(t1, t2, instance.n_leaves, 5);
    for c in components {
        println!("{};", render_expansion(&c));
    }
    
    Ok(())
}

fn get_candidates(state: &State) -> Vec<Vec<u32>> {
    let mut c1 = HashSet::new();
    let mut c2 = HashSet::new();
    collect_cherries(&state.tree1, &mut c1);
    collect_cherries(&state.tree2, &mut c2);
    
    let mut candidates = HashSet::new();
    for &(a, b) in c1.difference(&c2) {
        candidates.insert(vec![a]);
        candidates.insert(vec![b]);
    }
    for &(a, b) in c2.difference(&c1) {
        candidates.insert(vec![a]);
        candidates.insert(vec![b]);
    }
    
    if candidates.is_empty() {
        let leaves = get_all_meta_leaves(&state.tree1.nodes[state.tree1.root]);
        for l in leaves.into_iter().take(10) {
            candidates.insert(vec![l]);
        }
    }
    
    let mut vec: Vec<(Vec<u32>, (usize, isize, usize))> = candidates.into_iter().filter_map(|ids| {
        let child = cut_blocks(state, &ids)?;
        let sc = child.shared_clusters();
        let score = (child.cut_components.len() + child.leaf_count(), -(sc as isize), child.leaf_count());
        Some((ids, score))
    }).collect();

    vec.sort_by_key(|x| x.1);
    vec.into_iter().map(|x| x.0).take(20).collect()
}

fn cut_blocks(state: &State, block_ids: &[u32]) -> Option<State> {
    let mut t1 = state.tree1.clone();
    let mut t2 = state.tree2.clone();
    let mut new_comps = state.cut_components.clone();
    
    for &id in block_ids {
        cut_leaf_arena(&mut t1, id);
        cut_leaf_arena(&mut t2, id);
        if let Some(exp) = state.expansions.get(&id) {
            new_comps.push(exp.clone());
        }
    }
    
    if t1.root == 0 || t2.root == 0 { return None; }
    
    Some(normalize_state(State {
        tree1: t1,
        tree2: t2,
        expansions: state.expansions.clone(),
        next_id: state.next_id,
        cut_components: new_comps,
    }))
}

fn greedy_completion(start_state: &State, deadline: Instant) -> Vec<Expansion> {
    let mut state = start_state.clone();
    while !(state.tree1.nodes[state.tree1.root].is_leaf && state.tree2.nodes[state.tree2.root].is_leaf) {
        if Instant::now() > deadline { break; }
        let cands = get_candidates(&state);
        if cands.is_empty() { break; }
        if let Some(child) = cut_blocks(&state, &cands[0]) {
            state = child;
        } else {
            break;
        }
    }
    
    let mut ans = state.cut_components.clone();
    let root_node = &state.tree1.nodes[state.tree1.root];
    if root_node.is_leaf {
        if let Some(exp) = state.expansions.get(&root_node.leaf_id) {
            ans.push(exp.clone());
        }
    }
    ans
}

fn solve_beam(tree1: ArenaTree, tree2: ArenaTree, n_leaves: u32, limit_seconds: u64) -> Vec<Expansion> {
    let start_time = Instant::now();
    let deadline = start_time + Duration::from_secs(limit_seconds);
    
    let mut expansions = HashMap::new();
    for i in 1..=n_leaves {
        expansions.insert(i, Expansion::Leaf(i));
    }
    
    let start_state = normalize_state(State {
        tree1,
        tree2,
        expansions,
        next_id: n_leaves + 1,
        cut_components: Vec::new(),
    });
    
    let mut best_ans = greedy_completion(&start_state, deadline);
    let mut best_count = best_ans.len();

    let mut beam = vec![start_state];

    while !beam.is_empty() && Instant::now() < deadline {
        let next_states: Vec<State> = beam.par_iter().flat_map(|state| {
            if state.tree1.nodes[state.tree1.root].is_leaf && state.tree2.nodes[state.tree2.root].is_leaf {
                return vec![];
            }
            let cands = get_candidates(state);
            cands.into_par_iter().filter_map(|block_ids| {
                cut_blocks(state, &block_ids)
            }).collect::<Vec<State>>()
        }).collect();
        
        for state in &beam {
            if state.tree1.nodes[state.tree1.root].is_leaf && state.tree2.nodes[state.tree2.root].is_leaf {
                let mut ans = state.cut_components.clone();
                let root_node = &state.tree1.nodes[state.tree1.root];
                if let Some(exp) = state.expansions.get(&root_node.leaf_id) {
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
            let hash = (s.tree1.nodes[s.tree1.root].cluster_mask.clone(), s.tree2.nodes[s.tree2.root].cluster_mask.clone());
            if !seen.contains(&hash) {
                seen.insert(hash);
                unique.push(s);
            }
        }
        unique.truncate(30);
        beam = unique;
    }
    
    best_ans
}
