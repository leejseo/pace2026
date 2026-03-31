import gurobipy as gp
from gurobipy import GRB
import networkx as nx
import re
import sys

def parse_newick_to_nx(text):
    """Simple parser to convert Newick string to NetworkX graph."""
    text = text.strip().replace(";", "")
    
    def parse_rec(s, graph, next_node):
        if "(" not in s:
            # Leaf
            label = s.strip()
            graph.add_node(label, leaf=True)
            return label, next_node
        
        # Internal node
        # Find the comma that separates the two children at this level
        depth = 0
        comma_pos = -1
        for i, char in enumerate(s):
            if char == "(": depth += 1
            elif char == ")": depth -= 1
            elif char == "," and depth == 1:
                comma_pos = i
                break
        
        left_str = s[1:comma_pos]
        right_str = s[comma_pos+1:-1]
        
        node_id = f"int_{next_node}"
        next_node += 1
        graph.add_node(node_id, leaf=False)
        
        l_child, next_node = parse_rec(left_str, graph, next_node)
        r_child, next_node = parse_rec(right_str, graph, next_node)
        
        graph.add_edge(node_id, l_child)
        graph.add_edge(node_id, r_child)
        
        return node_id, next_node

    G = nx.Graph()
    root, _ = parse_rec(text, G, 0)
    return G, root

def solve_maf_ilp(inst_path):
    """
    Experimental ILP for MAF.
    This is a simplified version for small instances.
    Goal: Min edges to cut such that remaining components are identical subtrees.
    """
    try:
        with open(inst_path, "r") as f:
            lines = [l.strip() for l in f if l.strip() and not l.startswith("#")]
        
        T1, r1 = parse_newick_to_nx(lines[0])
        T2, r2 = parse_newick_to_nx(lines[1])
        
        leaves = [n for n, d in T1.nodes(data=True) if d.get('leaf')]
        
        model = gp.Model("MAF_ILP")
        
        # Decision variables: x[e] = 1 if edge e is cut in T1
        # Simplified: We only model cutting T1 and check if components are agreement subtrees.
        # This is a placeholder for a more complex cycle-breaking / subtree-matching ILP.
        
        print(f"Gurobi model initialized for {inst_path}")
        print(f"Leaves: {len(leaves)}")
        print("Note: Full MAF ILP is computationally expensive and requires complex path constraints.")
        
        # For demonstration of anytime quality vs optimal, we recommend using 
        # small 'exact' track instances from PACE.
        
    except Exception as e:
        print(f"Error initializing Gurobi: {e}")

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python3 gurobi_solver.py <instance.nw>")
    else:
        solve_maf_ilp(sys.argv[1])
