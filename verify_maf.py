import re
import sys

def parse_newick_to_clusters(text):
    """Parses a Newick string and returns a set of all clusters (leaf sets)."""
    text = text.strip().replace(" ", "")
    if not text: return set()
    
    pos = 0
    clusters = []
    
    def parse_rec():
        nonlocal pos
        if pos >= len(text): return set()
        
        if text[pos].isdigit():
            start = pos
            while pos < len(text) and text[pos].isdigit():
                pos += 1
            leaf = {int(text[start:pos])}
            # Single leaves are technically clusters but often ignored in comparison
            return leaf
        
        if text[pos] == '(':
            pos += 1 # skip (
            left_set = parse_rec()
            if pos < len(text) and text[pos] == ',':
                pos += 1 # skip ,
            right_set = parse_rec()
            if pos < len(text) and text[pos] == ')':
                pos += 1 # skip )
            
            full_set = left_set | right_set
            clusters.append(frozenset(full_set))
            return full_set
        return set()

    root_set = parse_rec()
    return set(clusters), root_set

def get_induced_clusters(original_clusters, target_leaves):
    """Filters original clusters to only those that contain a subset of target_leaves."""
    induced = []
    for c in original_clusters:
        intersection = c & target_leaves
        if len(intersection) > 1:
            induced.append(frozenset(intersection))
    return set(induced)

def verify_maf(inst_path, output_text):
    try:
        with open(inst_path, "r") as f:
            lines = [l.strip() for l in f if l.strip() and not l.startswith("#")]
        
        # 1. Parse Input Trees into Cluster Sets
        c1_all, l1_all = parse_newick_to_clusters(lines[0])
        c2_all, l2_all = parse_newick_to_clusters(lines[1])
        
        # 2. Parse Output Components
        comp_texts = [c.strip() for c in output_text.strip().split(';') if c.strip()]
        all_forest_leaves = set()
        
        for i, comp_text in enumerate(comp_texts):
            comp_clusters, comp_leaves = parse_newick_to_clusters(comp_text)
            
            # Check overlap
            if not comp_leaves.isdisjoint(all_forest_leaves):
                return False, f"Component {i} has overlapping leaves"
            all_forest_leaves |= comp_leaves
            
            # 3. Check Agreement: Cluster sets must match the induced original clusters
            # For rooted trees, isomorphism is equivalent to having identical cluster sets
            # when restricted to the same leaf set.
            induced1 = get_induced_clusters(c1_all, comp_leaves)
            induced2 = get_induced_clusters(c2_all, comp_leaves)
            
            if comp_clusters != induced1:
                return False, f"Component {i} structure mismatch with T1"
            if comp_clusters != induced2:
                return False, f"Component {i} structure mismatch with T2"
                
        # 4. Check Partition Completeness
        if all_forest_leaves != l1_all:
            return False, f"Leaf partition incomplete: missing {len(l1_all - all_forest_leaves)} labels"
            
        return True, len(comp_texts)
        
    except Exception as e:
        return False, f"Verifier Error: {str(e)}"

if __name__ == "__main__":
    if len(sys.argv) == 3:
        print(verify_maf(sys.argv[1], sys.argv[2]))
