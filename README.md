# PACE 2026 Heuristic Track Solver (Maximum Agreement Forest)

This repository contains a high-performance solver for the Maximum Agreement Forest (MAF) problem on two rooted binary trees, specifically developed for the PACE 2026 challenge.

## 🚀 Main Solver: Rust (High Performance)
The core logic has been ported to **Rust** to achieve maximum execution speed and parallel exploration density. It is the **primary solver** for this project.

### Key Algorithms
- **Parallel Wide Beam Search**: Explores multiple promising cut sequences in parallel using `Rayon`.
- **Anytime Simulated Annealing**: Uses the remaining time budget to iteratively improve the best solution found by the beam search.
- **Macro-cut Strategy**: Identifies and cuts entire pendant subtrees in a single operation, drastically reducing the number of components.
- **Score Caching & Bitmasking**: Uses `num-bigint` for arbitrary-precision bitmasking to accelerate structural similarity checks.

### Usage
```bash
cd pace2026_rs
cargo build --release
./target/release/pace2026_rs ../instances/heuristic00.nw --time-limit 300
```

## 🐍 Legacy/Prototype: Python
Located in `pace2026_heuristic_mvp/`, the Python implementation serves as a reference and rapid prototyping environment. It originally established the "Macro-cut" logic that now powers the Rust solver.

## 📊 Evaluation & Visualization
- `benchmark.py`: Validates and compares Python vs. Rust solvers.
- `compare_times.py`: Tests the "Anytime" property by running benchmarks at 30s, 60s, and 120s.
- `results/`: Contains timestamped logs and a `summary.md` of all runs.

## ⚙️ Development Environment (macOS)
### Python
We recommend using `pyenv` + `venv`:
```bash
python3 -m venv .venv
source .venv/bin/activate
```
### Rust
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

## ⚖️ License
This project is developed for the PACE 2026 competition.
