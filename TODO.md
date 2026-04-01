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

## Iteration 6: Leaf Depth Discordance Calculation
- **Task**: Precompute the topological depth of every leaf in $T_1$ and $T_2$. Calculate the absolute difference (discordance).
- **Goal**: Provide a fast, $O(1)$ heuristic metric to identify which leaves are structurally consistent between the two trees.

## Iteration 7: Discordance-Biased ALNS Repair
- **Task**: When the ALNS repair operator builds new components, sort the pool of available leaves by their discordance score (ascending). 
- **Goal**: Force the greedy builder to construct the "core" agreement forest first using the most consistent leaves, leaving highly discordant leaves as singletons or small components.

## Iteration 8: Exhaustive Pairwise Merging at Low Temperatures
- **Task**: When the SA temperature drops below a certain threshold, switch the local search from random polling to an exhaustive $O(K^2)$ check of all component pairs.
- **Goal**: Ensure no trivial merges are missed at the end of the search, squeezing out the last few component reductions.

## Iteration 9: Targeted Destruction of Failed Merges
- **Task**: Track pairs of components that fail the isomorphism check during merging. Use these "near-misses" as targets for the destroy operator.
- **Goal**: Focus destruction on the boundaries between large components to facilitate their eventual union.

## Iteration 10: Multi-Phase Anytime Execution
- **Task**: Structure the 5-minute run into distinct phases (Broad Exploration -> Intense Exploitation -> Final Exhaustive Polish) using the time limit.
- **Goal**: Guarantee the absolute minimum component count before the process terminates.

## Iteration 11: Ancestral Sharing Weighting
- **Task**: Calculate exact shared ancestral cluster count for each leaf between $T_1$ and $T_2$. Prioritize leaves with highest shared ancestry during component building.
- **Goal**: Build highly stable core agreement subtrees early, drastically reducing fragmentation.

## Iteration 12: Continuous Exhaustive Merging
- **Task**: Upgrade the local search to perform exhaustive $O(K^2)$ merge attempts continuously rather than just during the final phase.
- **Goal**: Since $K \approx 200$, 40k pairs is fast enough. Maximizes the component reduction across all phases.

## Iteration 13: Leaf Shift / Relocation Operator
- **Task**: Introduce an ALNS operator that moves a single leaf from one component to another (if valid).
- **Goal**: Explore local neighborhoods that are unreachable by pure component merging.

## Iteration 14: Stubborn Component Destruction
- **Task**: Track components that consistently fail to merge or shift. Target these specific components for ALNS destruction.
- **Goal**: Break out of severe local optima caused by highly discordant but tightly bound sub-clusters.

## Iteration 15: Cross-Thread Best Partition Seeding
- **Task**: Allow threads to periodically adopt the *global* best partition rather than just their local best, simulating an evolutionary "migration" or swarm intelligence.
- **Goal**: Converge all parallel computational power onto the most promising structural backbones.
