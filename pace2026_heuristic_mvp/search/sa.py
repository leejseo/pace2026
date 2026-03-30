import time
import math
import random
from typing import List
from ..core.tree import original_to_current, Expansion, leaf_ids_current
from ..core.state import State, normalize_state, is_solved, finalize_state, cut_block, cut_subtree, expansion_leaf_count, canon_expansion
from .beam import get_candidates, greedy_completion

def solve_sa(instance, time_limit_seconds: float = 5.0) -> List[Expansion]:
    start = normalize_state(State(
        tree1=original_to_current(instance.tree1),
        tree2=original_to_current(instance.tree2),
        expansions={i: i for i in range(1, instance.n_leaves + 1)},
        next_id=instance.n_leaves + 1,
        cut_components=tuple()
    ))
    deadline = time.monotonic() + time_limit_seconds
    
    # 1. Initial High-Quality Baseline
    best_components = greedy_completion(start, candidate_limit=5, deadline=deadline)
    best_cost = len(best_components)

    def adaptive_rollout(state: State, depth_limit: int = 10) -> List[Expansion]:
        curr = state
        for _ in range(depth_limit):
            if is_solved(curr) or time.monotonic() > deadline: break
            cands = get_candidates(curr, limit=3)
            if not cands: break
            # Softmax-like selection based on scores
            func, arg, _ = cands[0] if random.random() < 0.7 else random.choice(cands)
            curr = func(curr, arg)
        
        # If not solved, finish greedily
        if not is_solved(curr):
            return greedy_completion(curr, candidate_limit=3, deadline=deadline)
        return finalize_state(curr)

    current_state = start
    t_start = 1.0
    t = t_start
    cooling_rate = 0.98
    
    iteration = 0
    while time.monotonic() < deadline:
        iteration += 1
        
        # Reheating every 100 iterations
        if iteration % 100 == 0:
            t = t_start * 0.5
            current_state = start
            
        if is_solved(current_state):
            comp = finalize_state(current_state)
            if len(comp) < best_cost:
                best_components, best_cost = comp, len(comp)
            current_state = start
            continue

        cands = get_candidates(current_state, limit=10)
        if not cands:
            current_state = start
            continue
            
        func, arg, _ = random.choice(cands[:min(5, len(cands))])
        neighbor_state = func(current_state, arg)
        
        neighbor_sol = adaptive_rollout(neighbor_state)
        if not neighbor_sol: continue
        neighbor_cost = len(neighbor_sol)
        
        if neighbor_cost < best_cost:
            best_components, best_cost = neighbor_sol, neighbor_cost
            # Greedy descent from better neighbor
            current_state = neighbor_state
        else:
            cost_diff = neighbor_cost - best_cost
            if math.exp(-cost_diff / max(t, 1e-9)) > random.random():
                current_state = neighbor_state
        
        t *= cooling_rate
        if t < 0.001: t = 0.001

    return best_components
