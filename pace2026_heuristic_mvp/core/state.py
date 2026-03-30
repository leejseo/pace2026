import dataclasses
from functools import lru_cache
from typing import Dict, List, Tuple, Optional
from .tree import (
    CurrentNode, CurrentTree, Expansion, collect_cherries, 
    contract_cherry, cut_leaf_current, leaf_ids_current
)

@lru_cache(maxsize=None)
def canon_expansion(exp: Expansion) -> Tuple:
    if isinstance(exp, int): return ("L", exp)
    a, b = canon_expansion(exp[0]), canon_expansion(exp[1])
    return ("N", a, b) if a <= b else ("N", b, a)

@lru_cache(maxsize=None)
def expansion_leaf_count(exp: Expansion) -> int:
    if isinstance(exp, int): return 1
    return expansion_leaf_count(exp[0]) + expansion_leaf_count(exp[1])

def make_expansion_pair(a: Expansion, b: Expansion) -> Expansion:
    ca, cb = canon_expansion(a), canon_expansion(b)
    return (a, b) if ca <= cb else (b, a)

def canon_current(node: CurrentNode, expansions: Dict[int, Expansion]) -> Tuple:
    if node.is_leaf: 
        return ("B", canon_expansion(expansions[node.leaf_id]))
    a, b = canon_current(node.left, expansions), canon_current(node.right, expansions)
    return ("N", a, b) if a <= b else ("N", b, a)

def expand_current(node: CurrentNode, expansions: Dict[int, Expansion]) -> Expansion:
    if node.is_leaf: return expansions[node.leaf_id]
    return make_expansion_pair(expand_current(node.left, expansions), expand_current(node.right, expansions))

@dataclasses.dataclass(frozen=True)
class State:
    tree1: CurrentNode
    tree2: CurrentNode
    expansions: Dict[int, Expansion]
    next_id: int
    cut_components: Tuple[Expansion, ...]

    def leaf_count(self) -> int:
        # bit_count() is very fast in Python 3.10+
        mask = self.tree1.cluster_mask
        if hasattr(mask, "bit_count"):
            return mask.bit_count()
        return bin(mask).count('1')

def normalize_state(state: State) -> State:
    tree1, tree2 = state.tree1, state.tree2
    expansions, next_id = dict(state.expansions), state.next_id

    while True:
        c1 = set(collect_cherries(tree1))
        c2 = set(collect_cherries(tree2))
        common = c1 & c2
        if not common: break

        def cherry_key(pair: Tuple[int, int]) -> Tuple:
            ea, eb = canon_expansion(expansions[pair[0]]), canon_expansion(expansions[pair[1]])
            return (ea, eb) if ea <= eb else (eb, ea)

        a, b = min(common, key=cherry_key)
        new_id = next_id
        next_id += 1
        expansions[new_id] = make_expansion_pair(expansions[a], expansions[b])
        tree1 = contract_cherry(tree1, a, b, new_id)
        tree2 = contract_cherry(tree2, a, b, new_id)

    return State(tree1=tree1, tree2=tree2, expansions=expansions, next_id=next_id, cut_components=state.cut_components)

def state_signature(state: State) -> Tuple[Tuple, Tuple]:
    return (canon_current(state.tree1, state.expansions), canon_current(state.tree2, state.expansions))

def is_solved(state: State) -> bool:
    # Quick bitmask check before full canonical signature
    if state.tree1.cluster_mask != state.tree2.cluster_mask: return False
    sig1, sig2 = state_signature(state)
    return sig1 == sig2

def finalize_state(state: State) -> List[Expansion]:
    return list(state.cut_components) + [expand_current(state.tree1, state.expansions)]

def cut_subtree(state: State, subtree_root: CurrentNode) -> State:
    from .tree import get_all_meta_leaves
    block_ids = get_all_meta_leaves(subtree_root)
    new_component = expand_current(subtree_root, state.expansions)
    
    tree1, tree2 = state.tree1, state.tree2
    for bid in block_ids:
        tree1 = cut_leaf_current(tree1, bid)
        tree2 = cut_leaf_current(tree2, bid)
    
    return normalize_state(State(
        tree1=tree1, tree2=tree2,
        expansions=dict(state.expansions),
        next_id=state.next_id,
        cut_components=state.cut_components + (new_component,),
    ))

def cut_block(state: State, block_id: int) -> State:
    t1 = cut_leaf_current(state.tree1, block_id)
    t2 = cut_leaf_current(state.tree2, block_id)
    return normalize_state(State(
        tree1=t1, tree2=t2,
        expansions=dict(state.expansions),
        next_id=state.next_id,
        cut_components=state.cut_components + (state.expansions[block_id],),
    ))

def cut_blocks(state: State, block_ids: List[int]) -> State:
    curr = state
    for bid in block_ids:
        curr = cut_block(curr, bid)
    return curr
