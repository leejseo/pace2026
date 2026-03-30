import time
from typing import List, Set, Tuple
from ..core.tree import original_to_current, Expansion, collect_cherries
from ..core.state import State, normalize_state, is_solved, finalize_state, cut_blocks, expansion_leaf_count

def build_conflict_graph(state: State) -> Tuple[Set[int], Set[Tuple[int, int]]]:
    """
    Builds a conflict graph where edges represent pairs of leaves that are
    cherries in one tree but not the other.
    """
    cherries1 = set(collect_cherries(state.tree1))
    cherries2 = set(collect_cherries(state.tree2))
    
    nodes = set()
    edges = set()
    
    # Conflict: (a,b) is a cherry in T1, but they are not a cherry in T2.
    for a, b in cherries1 - cherries2:
        nodes.add(a); nodes.add(b)
        edges.add((min(a,b), max(a,b)))
        
    for a, b in cherries2 - cherries1:
        nodes.add(a); nodes.add(b)
        edges.add((min(a,b), max(a,b)))
        
    return nodes, edges

def greedy_vertex_cover(nodes: Set[int], edges: Set[Tuple[int, int]], state: State) -> List[int]:
    """
    Approximates minimum weight vertex cover. 
    """
    cover = []
    # degree of each node
    adj = {n: set() for n in nodes}
    for u, v in edges:
        adj[u].add(v)
        adj[v].add(u)
        
    while edges:
        def score(n):
            weight = expansion_leaf_count(state.expansions[n])
            return len(adj[n]) / (weight + 0.1)
            
        best_node = max(nodes, key=score)
        cover.append(best_node)
        
        edges_to_remove = [(u, v) for u, v in edges if u == best_node or v == best_node]
        for u, v in edges_to_remove:
            edges.remove((u, v))
            adj[u].discard(v)
            adj[v].discard(u)
        nodes.remove(best_node)
        
    return cover

def solve_lp_relaxation(instance, time_limit_seconds: float = 5.0) -> List[Expansion]:
    """
    LP-Relaxation Inspired Heuristic:
    Models the conflicts as a graph and uses Vertex Cover to resolve 
    multiple conflicts at once (Batch Cutting).
    """
    start = normalize_state(State(
        tree1=original_to_current(instance.tree1),
        tree2=original_to_current(instance.tree2),
        expansions={i: i for i in range(1, instance.n_leaves + 1)},
        next_id=instance.n_leaves + 1,
        cut_components=tuple()
    ))
    deadline = time.monotonic() + time_limit_seconds
    
    state = start
    while not is_solved(state):
        if time.monotonic() > deadline:
            break
            
        nodes, edges = build_conflict_graph(state)
        
        if not edges:
            if not is_solved(state):
                from ..core.tree import leaf_ids_current
                candidates = leaf_ids_current(state.tree1)
                smallest = min(candidates, key=lambda k: expansion_leaf_count(state.expansions[k]))
                state = cut_blocks(state, [smallest])
            continue
            
        # Cut the vertex cover batch
        cuts = greedy_vertex_cover(nodes, edges, state)
        if cuts:
            state = cut_blocks(state, cuts)
        else:
            break
                
    if is_solved(state):
        return finalize_state(state)
        
    from .beam import greedy_completion
    return greedy_completion(state, candidate_limit=5, deadline=deadline)
