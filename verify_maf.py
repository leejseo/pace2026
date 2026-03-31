import re
import sys

def parse_newick_to_ancestry(text):
    """Parses a Newick string and returns a mapping: leaf -> set of all ancestors (as leaf sets)."""
    text = text.strip().replace(" ", "")
    if not text: return {}, set()
    
    pos = 0
    ancestry = {} # leaf_id -> list of frozensets (clusters it belongs to)
    
    def parse_rec():
        nonlocal pos
        if pos >= len(text): return set()
        
        if text[pos].isdigit():
            start = pos
            while pos < len(text) and text[pos].isdigit(): pos += 1
            leaf = int(text[start:pos])
            ancestry[leaf] = []
            return {leaf}
        
        if text[pos] == '(':
            pos += 1
            left_set = parse_rec()
            if pos < len(text) and text[pos] == ',': pos += 1
            right_set = parse_rec()
            if pos < len(text) and text[pos] == ')': pos += 1
            
            full_set = left_set | right_set
            fs = frozenset(full_set)
            for leaf in full_set:
                ancestry[leaf].append(fs)
            return full_set
        return set()

    root_set = parse_rec()
    return ancestry, root_set

def verify_maf(inst_path, output_text):
    """
    Robust MAF verification based on Ancestry Invariance.
    A component matches the original tree if for any two leaves x, y in the component,
    their lowest common ancestor in the component has the same leaf set (restricted to the component)
    as their lowest common ancestor in the original tree.
    """
    try:
        with open(inst_path, "r") as f:
            lines = [l.strip() for l in f if l.strip() and not l.startswith("#")]
        
        # 1. Parse Original Trees
        anc1, leaves1 = parse_newick_to_ancestry(lines[0])
        anc2, leaves2 = parse_newick_to_ancestry(lines[1])
        
        # 2. Parse Forest
        comp_texts = [c.strip() for c in output_text.strip().split(';') if c.strip()]
        all_forest_leaves = set()
        
        for i, comp_text in enumerate(comp_texts):
            comp_anc, comp_leaves = parse_newick_to_ancestry(comp_text)
            
            if not comp_leaves.isdisjoint(all_forest_leaves):
                return False, f"Overlap in component {i}"
            all_forest_leaves |= comp_leaves
            
            # 3. Validation: For each leaf in component, its cluster hierarchy (restricted to comp_leaves)
            # must be a subset of the original hierarchy.
            for leaf in comp_leaves:
                comp_clusters = [c for c in comp_anc[leaf] if len(c) > 1]
                
                # Check against T1
                orig1_clusters_restricted = [c & comp_leaves for c in anc1[leaf] if len(c & comp_leaves) > 1]
                if set(comp_clusters) != set(orig1_clusters_restricted):
                    return False, f"Component {i} topology mismatch with T1 at leaf {leaf}"
                
                # Check against T2
                orig2_clusters_restricted = [c & comp_leaves for c in anc2[leaf] if len(c & comp_leaves) > 1]
                if set(comp_clusters) != set(orig2_clusters_restricted):
                    return False, f"Component {i} topology mismatch with T2 at leaf {leaf}"
                    
        if all_forest_leaves != leaves1:
            return False, "Partition incomplete"
            
        return True, len(comp_texts)
    except Exception as e:
        return False, f"Error: {str(e)}"

if __name__ == "__main__":
    if len(sys.argv) == 3:
        print(verify_maf(sys.argv[1], sys.argv[2]))
