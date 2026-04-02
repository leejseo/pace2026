# Failed Experiment: Exact Subtree Kernelization

## What we tried
We attempted to implement "Rule 1: Subtree Reduction" (True Topological Subtree Kernelization). The idea was to physically collapse maximal common subtrees in $T_1$ and $T_2$ into single "meta-leaves" before running the ALNS metaheuristic. This would theoretically shrink $N$ from 15,000 to a much smaller kernel size (e.g. $<1000$).

## Why it failed
1. **Low Baseline Similarity:** In the heuristic instances provided (like `heuristic26` and `heuristic28`), the trees are highly disorganized at the bottom levels. The kernelization only reduced $N$ from 15,000 down to $\approx 12,000$.
2. **Meta-leaf Rigidity:** By treating these large maximal common subtrees as single indivisible meta-leaves, the ALNS solver lost the ability to break them apart. Sometimes, breaking a common subtree to resolve a higher-level conflict is globally optimal.
3. **Performance Degradation:** Because $N$ was still very large (12,000), the ALNS solver didn't gain a massive speedup, but it was restricted in its search space. The result was a severe performance degradation (producing $\approx 4,000$ components instead of the $<250$ components found without kernelization).

## Conclusion for LLMs
Do not attempt to pre-collapse or freeze subtrees rigidly on these specific hard heuristic instances. The ALNS-SA with a greedy builder performs much better when it has the freedom to dynamically build and destroy components at the individual leaf level.
