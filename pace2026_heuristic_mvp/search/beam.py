import time
import sys
from typing import List, Dict, Tuple, Callable, Any
from ..core.tree import (
    collect_cherries, leaf_ids_current, offpath_candidates, 
    original_to_current, Expansion, get_all_meta_leaves, 
    CurrentTree, get_shared_cluster_count
)
from ..core.state import State, normalize_state, is_solved, finalize_state, cut_block, cut_subtree, canon_expansion, expansion_leaf_count, state_signature

# A candidate is now (Function, Argument, ScoreKey)
Candidate = Tuple[Callable[[State, Any], State], Any, Tuple]

def get_score(child: State):
    sc = get_shared_cluster_count(child.tree1, child.tree2)
    # Priority:
    # 1. Minimize total estimated components (cut + current leaves)
    # 2. Maximize shared clusters (structural similarity)
    # 3. Minimize current leaf count
    return (len(child.cut_components) + child.leaf_count(), -sc, child.leaf_count())

def state_rank(state: State) -> Tuple[int, int, int]:
    return get_score(state)

def get_candidates(state: State, limit: int) -> List[Candidate]:
    cherries1 = set(collect_cherries(state.tree1))
    cherries2 = set(collect_cherries(state.tree2))
    
    candidates: List[Candidate] = []
    
    for a, b in cherries1 - cherries2:
        # Cut endpoints
        for bid in (a, b):
            child = cut_block(state, bid)
            candidates.append((cut_block, bid, get_score(child)))
        # Cut hanging subtrees
        for sub_root in offpath_candidates(state.tree2, a, b):
            child = cut_subtree(state, sub_root)
            candidates.append((cut_subtree, sub_root, get_score(child)))
            
    for a, b in cherries2 - cherries1:
        # Cut endpoints
        for bid in (a, b):
            child = cut_block(state, bid)
            candidates.append((cut_block, bid, get_score(child)))
        # Cut hanging subtrees
        for sub_root in offpath_candidates(state.tree1, a, b):
            child = cut_subtree(state, sub_root)
            candidates.append((cut_subtree, sub_root, get_score(child)))
            
    if not candidates:
        for bid in leaf_ids_current(state.tree1):
            child = cut_block(state, bid)
            candidates.append((cut_block, bid, get_score(child)))
    
    # Sort by score
    candidates.sort(key=lambda x: x[2])
    
    # Deduplicate by signature to keep search diverse
    unique_candidates = []
    seen_sigs = set()
    for func, arg, score in candidates:
        child = func(state, arg)
        sig = state_signature(child)
        if sig not in seen_sigs:
            seen_sigs.add(sig)
            unique_candidates.append((func, arg, score))
            if len(unique_candidates) >= limit: break
            
    return unique_candidates

def greedy_completion(start_state: State, candidate_limit: int, deadline: float) -> List[Expansion]:
    state = start_state
    while not is_solved(state):
        if time.monotonic() > deadline:
            candidates = leaf_ids_current(state.tree1)
            chosen_id = min(candidates, key=lambda b: (expansion_leaf_count(state.expansions[b]), canon_expansion(state.expansions[b])))
            state = cut_block(state, chosen_id)
            continue
        cands = get_candidates(state, limit=candidate_limit)
        if not cands: break
        func, arg, _ = cands[0]
        state = func(state, arg)
    return finalize_state(state)

def solve_beam(instance, beam_width: int = 8, candidate_limit: int = 12, time_limit_seconds: float = 5.0) -> List[Expansion]:
    actual_beam_width = max(beam_width, int(beam_width * (time_limit_seconds / 2.0)))
    # Cap beam width for performance
    actual_beam_width = min(actual_beam_width, 100)
    
    sys.setrecursionlimit(max(10_000, 8 * instance.n_leaves + 100))
    start = normalize_state(State(
        tree1=original_to_current(instance.tree1),
        tree2=original_to_current(instance.tree2),
        expansions={i: i for i in range(1, instance.n_leaves + 1)},
        next_id=instance.n_leaves + 1,
        cut_components=tuple()
    ))
    deadline = time.monotonic() + time_limit_seconds
    best_components = greedy_completion(start, candidate_limit, deadline)
    best_count = len(best_components)

    beam: List[State] = [start]
    while beam and time.monotonic() < deadline:
        next_states: Dict[Tuple[Tuple, Tuple], State] = {}
        for state in beam:
            if is_solved(state):
                components = finalize_state(state)
                if len(components) < best_count:
                    best_components, best_count = components, len(components)
                continue
            
            for func, arg, _ in get_candidates(state, limit=candidate_limit):
                if time.monotonic() >= deadline: break
                child = func(state, arg)
                if is_solved(child):
                    components = finalize_state(child)
                    if len(components) < best_count:
                        best_components, best_count = components, len(components)
                    continue
                if len(child.cut_components) >= best_count: continue
                sig = state_signature(child)
                if sig not in next_states or state_rank(child) < state_rank(next_states[sig]):
                    next_states[sig] = child
        if not next_states: break
        beam = sorted(next_states.values(), key=state_rank)[:actual_beam_width]
        
    return best_components
