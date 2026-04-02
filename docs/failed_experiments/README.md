# Failed Experiments & Anti-Patterns Log

This directory contains records of ideas and optimization attempts that were hypothesized to improve the MAF solver but either failed to maintain structural validity (e.g., topology mismatch), degraded performance, or were too memory-intensive.

**Instructions for LLM Assistants:**
When iterating on the solver:
1. Always review this log to prevent repeating known mistakes.
2. If an attempted improvement fails validation or performs worse than the baseline, **do not** commit the code. Instead, revert the changes, write a brief post-mortem file in this directory (e.g., `failed_fast_mcsr.md`), and proceed with a new strategy.
