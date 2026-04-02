# Research: Exact Kernelization for Rooted MAF

## Core Concepts
While metaheuristics (ALNS, SA) are excellent for large combinatorial spaces, the Rooted Maximum Agreement Forest (rMAF) problem is Fixed-Parameter Tractable (FPT). This means if the optimal number of components (or rSPR distance, $k$) is small, the problem can be drastically reduced in size *before* solving.

This process is called **Kernelization**.

### 1. Subtree Reduction (Rule 1)
If a subtree $P$ appears identically in both $T_1$ and $T_2$ (same topology and leaf set), it can be replaced by a single "meta-leaf" in both trees.
- **Our Current State**: We identify these maximal common subtrees (MCSR) and group their labels into clusters, but we still perform our searches and bitmask operations on the original 15,000-leaf tree. 
- **The Upgrade**: We must physically build a new, smaller pair of trees ($T_1', T_2'$) where these subtrees are actually replaced by a single leaf node. If $k < 10$, theoretical bounds guarantee the reduced tree will have fewer than $\approx 300$ leaves!

### 2. Chain Reduction (Rule 2)
A "chain" is a sequence of leaves that form a caterpillar structure. If both trees contain an identical chain of length $n > 3$, it can be safely reduced to a chain of length exactly 3 without changing the optimal MAF cut strategy. 

### 3. FPT / ILP on the Kernel
Once $T_1$ and $T_2$ are reduced to $T_1'$ and $T_2'$ with $N' \ll N$ leaves, computing the MAF becomes exponentially faster.
- A bitset for 300 leaves takes only 5 `u64` words (vs 235 words for 15,000 leaves).
- We can apply an exact FPT branching algorithm or an exhaustive ALNS search on this micro-tree to guarantee finding the single-digit optimal solution within seconds.
- Finally, we map the meta-leaves back to their original leaf sets.

## Conclusions & Implementation Strategy (Iterations 16-20)
To shatter the 200-component plateau and reach single digits, we must transition from purely heuristic searches on the macro-tree to exact/heuristic searches on a physically reduced Kernel Tree.

- **Phase 1**: Implement True Topological Subtree Reduction (physically generating $T_1', T_2'$).
- **Phase 2**: Solve MAF on the Kernel Tree.
- **Phase 3**: Expand the Kernel Solution back to the original leaves.
