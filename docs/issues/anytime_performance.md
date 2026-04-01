# Issue: Poor Anytime Performance

## Description
The component count does not decrease significantly as time increases from 1 minute to 5 minutes on large instances.

## Symptoms
- Benchmark results show static or slowly improving component counts.
- High variance between parallel runs but limited progress within a single thread's exploration.

## Potential Causes
1.  **Branching Bias**: The current 3-way branching weights might be suboptimal, favoring "macro-cuts" that add many components too aggressively.
2.  **Lack of Local Search**: No hill-climbing or simulated annealing is currently applied to the forest partition.
3.  **Contraction Overhead**: Re-calculating common cherries and contracting them might be too slow on 15k-leaf trees, limiting the number of iterations.

## Goal
Implement a strategy where `heuristic28.nw` shows clear improvement (e.g., >10% reduction in components) between 60s and 300s.
