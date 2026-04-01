# PACE 2026 Heuristic Improvement Action Items

## Ongoing Rust Port & Fine-tuning (5 Iterations)

- [x] **Iteration 11: Finish Rust Beam Search Core**
  - Done: Rust core parses CLI, initiates beam search.
- [x] **Iteration 12: Rust Newick IO**
  - Done: Fast custom parser implemented.
- [x] **Iteration 13: Parallel Candidate Generation (Rayon)**
  - Done: Rayon implemented in state branching.
- [ ] **Iteration 14: Implement Macro-Cut (Subtree Cut) in Rust**
  - To match Python's 3-component performance, Rust needs to cut whole pendant subtrees instead of just single leaves.
- [ ] **Iteration 15: Rust vs Python Benchmark & Mega-Instance Stress Test**
  - Evaluate Rust with macro-cuts on `heuristic00.nw` and `heuristic26.nw` (14k leaves) to ensure `no stack overflow` and `time_limit` adherence.
