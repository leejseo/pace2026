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

## Iteration 16: True Topological Subtree Kernelization
- **Task**: Instead of just tracking common clusters, physically construct new reduced trees $T_1'$ and $T_2'$ where identical subtrees are collapsed into single "meta-leaves".
- **Goal**: Shrink the size of the tree ($N$) from 15,000+ to under 300 leaves, exponentially reducing bitmask allocation sizes and recursion depths.

## Iteration 17: Solve Kernel Tree MAF
- **Task**: Run the ALNS-SA / FPT solver entirely on the reduced kernel tree ($T_1', T_2'$).
- **Goal**: Solve the problem in the micro-space where the true rMAF distance $k$ dictates a tiny search boundary, reaching single-digit optimal components instantly.

## Iteration 18: Map Kernel Solution to Original Leaves
- **Task**: Take the resulting partition from the Kernel Tree and map the meta-leaves back to their original subsets of labels.
- **Goal**: Expand the optimized kernel solution back to a fully valid solution on the original 15k trees.

## Iteration 19: Strict Validation of Expanded Solution
- **Task**: Verify the expanded components with the `is_truly_isomorphic_fast` logic against the original $T_1$ and $T_2$.
- **Goal**: Ensure the kernel expansion did not introduce any subtle topology violations.

## Iteration 20: Kernelization Pre-computation Pipeline
- **Task**: Formalize Iterations 16-19 into a definitive pipeline that runs before any metaheuristic, defaulting to the original tree search only if kernelization fails to reduce $N$ significantly.
- **Goal**: The ultimate, robust structure for the 2026 PACE rMAF challenge.

## Iteration 21: Conflict Graph Formulation (ILP/LP Prep)
- **Task**: For a subset of discordant leaves or boundary clusters, construct a conflict graph where nodes are leaves and edges represent a topological contradiction between $T_1$ and $T_2$.
- **Goal**: Translate the rMAF problem locally into a Maximum Weight Independent Set (MWIS) or Vertex Cover problem to prepare for exact/LP solving.

## Iteration 22: Greedy Independent Set on Conflict Graph
- **Task**: Implement a fast $O(V+E)$ greedy solver on the constructed conflict graph to find an initial large independent set (a valid agreement forest).
- **Goal**: Serve as a highly optimized heuristic repair operator that outperforms random polling.

## Iteration 23: LP Relaxation using External Solver
- **Task**: Export the conflict graph to an LP format or interact with an external solver (like Gurobi or CBC) to solve the fractional relaxation of the MWIS.
- **Goal**: Obtain a theoretical upper bound on the maximum agreement forest size for the local neighborhood.

## Iteration 24: LP-Guided ALNS Construction
- **Task**: Use the fractional LP variables (e.g., $x_i = 0.8$) as selection probabilities in the ALNS greedy builder instead of pure random shuffles.
- **Goal**: Guide the stochastic search directly towards the mathematical global optimum.

## Iteration 25: Sub-component Conflict Graph
- **Task**: Instead of building a conflict graph of individual leaves, build it out of the $K \approx 200$ components currently found by the ALNS.
- **Goal**: Identify pairs or triplets of components that are "almost" mergeable and target their specific conflicting leaves.

## Iteration 26: Conflict Graph Generator
- **Task**: Build a rudimentary Conflict Graph Generator for subsets of components to identify specific leaf-level conflicts.
- **Goal**: Isolate and model the local constraints required to merge near-miss components.

## Iteration 27: Greedy Maximum Independent Set (MIS) Repair
- **Task**: Implement a Greedy MIS repair operator to replace the random ALNS repair for small neighborhoods.
- **Goal**: Serve as a highly optimized heuristic repair operator that outperforms random polling in dense conflict zones.

## Iteration 28: Edge-Cut ILP Formulation
- **Task**: Formalize the Edge-Cut ILP constraints for the MAF problem for local subproblems ($K \le 20$) and implement a basic solver logic.
- **Goal**: Obtain theoretical upper bounds and exact solutions for local neighborhood merges.

## Iteration 29: Local ILP Polish Integration
- **Task**: Integrate the "Local ILP Polish" phase into the final 30 seconds of the `main.rs` run loop.
- **Goal**: Squeeze out the last 5-10 component reductions to push the result towards single digits before termination.

## Iteration 30: Comprehensive Benchmark and Documentation
- **Task**: Run 5-minute benchmarks on all hard instances using the full ALNS-SA-ILP pipeline.
- **Goal**: Finalize the solver and push the ultimate results to upstream.
