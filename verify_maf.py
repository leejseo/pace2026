import re
import sys

class Node:
    def __init__(self, left=None, right=None, label=None):
        self.left = left
        self.right = right
        self.label = label
    def is_leaf(self):
        return self.label is not None

def parse_newick(text):
    text = text.strip().replace(" ", "")
    pos = 0
    def parse_rec():
        nonlocal pos
        if text[pos].isdigit():
            start = pos
            while pos < len(text) and text[pos].isdigit():
                pos += 1
            return Node(label=int(text[start:pos]))
        if text[pos] == '(':
            pos += 1 # skip (
            left = parse_rec()
            pos += 1 # skip ,
            right = parse_rec()
            pos += 1 # skip )
            return Node(left=left, right=right)
    return parse_rec()

def get_induced_subtree(root, labels):
    """Returns a simplified tree containing only the given labels."""
    if root.is_leaf():
        return root if root.label in labels else None
    
    l = get_induced_subtree(root.left, labels)
    r = get_induced_subtree(root.right, labels)
    
    if l and r: return Node(left=l, right=r)
    return l or r

def are_isomorphic(n1, n2):
    """Checks if two trees have the same topology."""
    if n1.is_leaf() and n2.is_leaf():
        return n1.label == n2.label
    if n1.is_leaf() or n2.is_leaf():
        return False
    
    # Try both orientations (since children order might differ in agreement)
    case1 = are_isomorphic(n1.left, n2.left) and are_isomorphic(n1.right, n2.right)
    case2 = are_isomorphic(n1.left, n2.right) and are_isomorphic(n1.right, n2.left)
    return case1 or case2

def verify_maf(inst_path, output_text):
    """Comprehensive MAF verification."""
    with open(inst_path, "r") as f:
        lines = [l.strip() for l in f if l.strip() and not l.startswith("#")]
    
    T1 = parse_newick(lines[0])
    T2 = parse_newick(lines[1])
    
    # 1. Parse output components
    comp_texts = [c.strip() for c in output_text.strip().split(';') if c.strip()]
    forest = [parse_newick(c) for c in comp_texts]
    
    # 2. Check leaf partition
    all_original_labels = set(re.findall(r'\d+', "".join(lines)))
    all_forest_labels = []
    for comp in forest:
        def collect(n, l):
            if n.is_leaf(): l.append(str(n.label))
            else: collect(n.left, l); collect(n.right, l)
        collect(comp, all_forest_labels)
    
    if set(all_forest_labels) != all_original_labels:
        return False, "Leaf set mismatch"
    if len(all_forest_labels) != len(set(all_forest_labels)):
        return False, "Duplicate leaves in forest"

    # 3. Check if each component is an Agreement Subtree
    for i, comp in enumerate(forest):
        labels = set()
        def collect(n, l):
            if n.is_leaf(): l.add(n.label)
            else: collect(n.left, l); collect(n.right, l)
        collect(comp, labels)
        
        # Induced subtrees in original T1 and T2 must be isomorphic to the component
        sub1 = get_induced_subtree(T1, labels)
        sub2 = get_induced_subtree(T2, labels)
        
        if not (are_isomorphic(comp, sub1) and are_isomorphic(comp, sub2)):
            return False, f"Component {i} is not a common agreement subtree"

    return True, len(forest)

if __name__ == "__main__":
    # Unit tests for the verifier itself
    print("Self-testing verifier...")
    # Add logic here if run directly
