import time
import random
from typing import List
from ..core.tree import original_to_current, Expansion
from ..core.state import State, normalize_state, finalize_state
from .beam import get_candidates, greedy_completion, is_solved

def solve_local_search(instance, time_limit_seconds: float = 5.0) -> List[Expansion]:
    """
    GRASP-style Randomized Local Search.
    """
    start = normalize_state(State(
        tree1=original_to_current(instance.tree1),
        tree2=original_to_current(instance.tree2),
        expansions={i: i for i in range(1, instance.n_leaves + 1)},
        next_id=instance.n_leaves + 1,
        cut_components=tuple()
    ))
    deadline = time.monotonic() + time_limit_seconds
    best_components = greedy_completion(start, candidate_limit=12, deadline=deadline)
    
    while time.monotonic() < deadline:
        state = start
        while not is_solved(state):
            if time.monotonic() > deadline:
                break
            candidates = get_candidates(state, limit=8)
            if not candidates:
                break
            chosen_func, chosen_arg, _ = random.choice(candidates[:min(3, len(candidates))])
            state = chosen_func(state, chosen_arg)
        
        if is_solved(state):
            comp = finalize_state(state)
            if len(comp) < len(best_components):
                best_components = comp
                
    return best_components
