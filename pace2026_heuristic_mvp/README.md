# PACE 2026 Heuristic-track MVP (Refactored)

This is a **modular, multi-heuristic solver** for the PACE 2026 heuristic track on two rooted binary trees (Maximum Agreement Forest).

## Package Structure

The MVP has been refactored into a full Python package for easy experimentation and algorithmic fine-tuning:

```
pace2026_heuristic_mvp/
├── __main__.py          # CLI Entry point
├── core/                # Core domain logic
│   ├── io.py            # Newick parsing and output rendering
│   ├── state.py         # Search state representation and normalizations
│   └── tree.py          # Tree data structures and operations
└── search/              # Search heuristics
    ├── beam.py          # Greedy + Beam Search (Baseline)
    ├── local_search.py  # Randomized Multi-start Local Search (GRASP)
    ├── sa.py            # Simulated Annealing over cut sequences
    └── lp.py            # LP-Relaxation Inspired Vertex Cover approach
```

## Available Heuristics

1. **Beam Search (`beam`)**: The default baseline. Explores a small beam of promising cuts at each conflict, prioritizing cuts that minimize the remaining tree size.
2. **Local Search (`local`)**: A GRASP-style randomized greedy algorithm. It introduces noise into candidate selection to escape local optima and repeats the search until the time limit.
3. **Simulated Annealing (`sa`)**: Performs a simulated annealing random walk over the sequence of cuts. Accepts worse cut sequences with a decaying probability to explore broader regions of the MAF state space.
4. **LP-Relaxation / Vertex Cover (`lp`)**: Builds a conflict graph from mismatched cherries and computes a minimum-weight vertex cover to resolve multiple structural conflicts in a single batch.

## Usage

Run the solver as a Python module, specifying the heuristic strategy:

```bash
# Run the default Beam Search
python3 -m pace2026_heuristic_mvp instance.gr

# Run Simulated Annealing with a larger time budget
python3 -m pace2026_heuristic_mvp instance.gr --heuristic sa --time-limit-seconds 10.0

# Run LP-Relaxation Vertex Cover
python3 -m pace2026_heuristic_mvp instance.gr --heuristic lp
```

## Performance Observations & Next Steps

Current evaluation shows that all algorithms tend to produce a large number of components. This is due to a fundamental limitation in the current state representation:

- **The Single-Leaf Cut Plateau**: The solver currently cuts *single meta-leaves* (blocks) instead of entire pendant subtrees. When a large pendant subtree obstructs a cherry, the solver must chip away at it leaf-by-leaf, resulting in extreme fragmentation.
- **Future Upgrade Required**: To significantly lower the component count (e.g., from 190 down to 10~20), the core `cut_block` logic must be upgraded to a `cut_subtree` or `split_tree` operation. This would allow the heuristics to detach entire clades in a single move, drastically improving the quality of the Agreement Forest.
