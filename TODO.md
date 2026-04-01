# PACE 2026 MAF Solver - Action Items (5 Iterations)

## Iteration 1: Enhance ALNS Destroy Operator
- **Task**: Implement a "Conflict-Proximity" destroy operator. Instead of purely random or size-based removal, identify nodes that have different Lowest Common Ancestors (LCAs) or high topological distance between $T_1$ and $T_2$, and target their containing components for destruction.
- **Goal**: More targeted resolution of topological conflicts.

## Iteration 2: Enhance Preprocessing with Safe Path Contraction
- **Task**: Proactively compress degree-2 nodes in the `OriginalNode` representation before running the solver loop.
- **Goal**: Save memory, reduce recursive depth, and speed up hashing / `is_truly_isomorphic` checks.

## Iteration 3: Refine ALNS Repair Operator
- **Task**: Implement a heuristic beam search within the repair phase instead of pure stochastic greedy. When repairing removed labels, keep top-k promising component expansions.
- **Goal**: Higher quality component building during the repair phase.

## Iteration 4: Dynamic Simulated Annealing Cooling
- **Task**: Make the SA cooling schedule reactive to the actual remaining time limit, ensuring it explores broadly at first and strictly exploits near the end of the time limit.
- **Goal**: Optimal use of the 5-minute (or given) time limit.

## Iteration 5: Parallel Local Search Diversification
- **Task**: Assign different ALNS parameters (destroy rate, cooling schedule aggressiveness) to different threads to diversify the search space exploration.
- **Goal**: Prevent all threads from getting stuck in the same local optima.
