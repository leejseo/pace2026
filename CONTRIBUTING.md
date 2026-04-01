# Contributing to PACE 2026 MAF Solver

## Engineering Standards
- **Correctness First**: Every change must be validated against `verify_maf.py`. No solution is acceptable unless it passes strict topology invariance.
- **Performance**: The solver must handle instances with 15,000+ leaves. Memory efficiency (using `Arc`, avoiding unnecessary clones) is critical.
- **Anytime Property**: The solver should provide a valid solution quickly and improve it as more time is allocated.
- **Deterministic Logic**: Algorithms should be deterministic where possible (e.g., canonical Newick rendering) to ensure reproducible verification.

## Development Workflow
1.  **Issue Documentation**: Before fixing a bug, document it in `docs/issues/` with a reproduction case.
2.  **Implementation**: Surgical updates to the Rust codebase.
3.  **Validation**: Run local benchmarks and the official validator.
4.  **Failure Logging**: If an idea or optimization fails or degrades performance, **DO NOT** commit the broken code to main. Instead, document the approach, why it failed, and what to avoid next time in `docs/failed_experiments/`. This serves as an anti-pattern reference for future development (and any assisting LLMs).
5.  **Upstream Sync**: Frequent commits to the main branch upon validated success to ensure progress visibility.
