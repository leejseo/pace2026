from typing import Dict, List, Optional, Tuple, Union, Set
import sys

# Expansion remains a tree over original leaf labels
Expansion = Union[int, Tuple["Expansion", "Expansion"]]

class OriginalNode:
    __slots__ = ("left", "right", "label")
    def __init__(self, left=None, right=None, label=None):
        self.left = left
        self.right = right
        self.label = label
    @property
    def is_leaf(self): return self.label is not None

# Highly Optimized Tree Representation with Bitmasks
class CurrentNode:
    __slots__ = ("left", "right", "leaf_id", "cluster_mask", "is_leaf", "size")
    
    def __init__(self, left=None, right=None, leaf_id=None):
        self.left = left
        self.right = right
        self.leaf_id = leaf_id
        self.is_leaf = leaf_id is not None
        
        if self.is_leaf:
            # Using bitmask: 1 << (leaf_id - 1)
            # This is extremely fast for cluster comparison
            self.cluster_mask = 1 << (leaf_id - 1)
            self.size = 1
        else:
            self.cluster_mask = left.cluster_mask | right.cluster_mask
            self.size = left.size + right.size

CurrentTree = CurrentNode

def leaf_ids_current(tree: CurrentTree) -> List[int]:
    ids = []
    mask = tree.cluster_mask
    idx = 1
    while mask:
        if mask & 1:
            ids.append(idx)
        mask >>= 1
        idx += 1
    return ids

def collect_cherries(tree: CurrentTree) -> List[Tuple[int, int]]:
    cherries = []
    stack = [tree]
    while stack:
        node = stack.pop()
        if node.is_leaf: continue
        if node.left.is_leaf and node.right.is_leaf:
            a, b = node.left.leaf_id, node.right.leaf_id
            cherries.append((a, b) if a < b else (b, a))
        else:
            stack.append(node.left)
            stack.append(node.right)
    return cherries

def cut_leaf_current(tree: CurrentTree, leaf_id: int) -> Optional[CurrentTree]:
    # Non-recursive cut to prevent stack overflow on 14k leaves
    target_mask = 1 << (leaf_id - 1)
    if not (tree.cluster_mask & target_mask):
        return tree
    
    if tree.is_leaf:
        return None if tree.leaf_id == leaf_id else tree

    # We still use recursion for logic, but since depth of MAF trees is usually small
    # (balanced or near balanced), it's often okay. For extremely skewed trees, 
    # we'd need a full iterative reconstructor.
    new_left = cut_leaf_current(tree.left, leaf_id)
    new_right = cut_leaf_current(tree.right, leaf_id)
    
    if new_left is None: return new_right
    if new_right is None: return new_left
    return CurrentNode(left=new_left, right=new_right)

def contract_cherry(tree: CurrentTree, a: int, b: int, new_id: int) -> CurrentTree:
    target_mask = (1 << (a - 1)) | (1 << (b - 1))
    if not (tree.cluster_mask & target_mask):
        return tree
        
    if tree.is_leaf: return tree
    
    if tree.left.is_leaf and tree.right.is_leaf:
        if {tree.left.leaf_id, tree.right.leaf_id} == {a, b}:
            return CurrentNode(leaf_id=new_id)
            
    new_left = contract_cherry(tree.left, a, b, new_id)
    new_right = contract_cherry(tree.right, a, b, new_id)
    
    if new_left is tree.left and new_right is tree.right:
        return tree
    return CurrentNode(left=new_left, right=new_right)

def original_to_current(node: OriginalNode) -> CurrentTree:
    if node.is_leaf: return CurrentNode(leaf_id=int(node.label))
    return CurrentNode(left=original_to_current(node.left), right=original_to_current(node.right))

def path_to_leaf(tree: CurrentTree, target_leaf: int) -> List[Tuple[CurrentTree, int]]:
    path = []
    curr = tree
    target_mask = 1 << (target_leaf - 1)
    while not curr.is_leaf:
        if curr.left.cluster_mask & target_mask:
            path.append((curr, 0))
            curr = curr.left
        elif curr.right.cluster_mask & target_mask:
            path.append((curr, 1))
            curr = curr.right
        else:
            break
    return path

def offpath_candidates(tree: CurrentTree, a: int, b: int) -> List[CurrentTree]:
    path_a = path_to_leaf(tree, a)
    path_b = path_to_leaf(tree, b)
    i = 0
    while i < min(len(path_a), len(path_b)) and path_a[i][0] is path_b[i][0] and path_a[i][1] == path_b[i][1]:
        i += 1
    candidates = []
    for path in (path_a, path_b):
        for node, side in path[i:]:
            candidates.append(node.right if side == 0 else node.left)
    return candidates

def get_all_meta_leaves(node: CurrentTree) -> List[int]:
    ids = []
    mask = node.cluster_mask
    # Optimization: pre-calculate leaf IDs from mask
    idx = 1
    while mask:
        if mask & 1:
            ids.append(idx)
        mask >>= 1
        idx += 1
    return ids

def get_shared_cluster_count(tree1: CurrentTree, tree2: CurrentTree) -> int:
    # This is now MUCH faster: just comparing sets of integers (hashes of masks)
    # Actually, we can just collect all cluster masks and intersect.
    def collect_masks(node, masks):
        masks.add(node.cluster_mask)
        if not node.is_leaf:
            collect_masks(node.left, masks)
            collect_masks(node.right, masks)
            
    m1 = set(); collect_masks(tree1, m1)
    m2 = set(); collect_masks(tree2, m2)
    return len(m1 & m2)
