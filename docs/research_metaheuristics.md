# Research: Metaheuristics for Combinatorial Optimization (MAF Focus)

## Core Concepts
Combinatorial optimization problems like Maximum Agreement Forest (MAF) require a balance between **Exploration** (discovering new areas of the search space) and **Exploitation** (refining known good solutions).

### 1. Large Neighborhood Search (LNS) / Adaptive LNS (ALNS)
LNS is currently our most successful framework.
- **Destroy & Repair**: The solver breaks a portion of the solution and re-builds it greedily. 
- **Adaptive Portfolio**: ALNS uses multiple destroy/repair operators and weights them based on their performance.
- **Rooted MAF Application**: 
    - **Destroy**: Random component removal, removing components near topological conflicts, or removing small "noise" components.
    - **Repair**: Greedy agreement subtree expansion (current approach) or FPT-based conflict resolution.

### 2. Simulated Annealing (SA)
SA provides a robust acceptance criterion to escape local optima.
- **Metropolis Criterion**: Always accept improvements; accept worse solutions with probability $P = e^{-\Delta E / T}$.
- **Cooling Schedule**: Start with high temperature (exploration) and gradually cool down (exploitation).

### 3. Evolutionary Algorithms (EA / Genetic)
Good for global exploration but computationally expensive due to validity repair.
- **Rooted MAF Challenge**: Standard crossover (combining two forests) almost always results in an invalid forest that requires costly repair.

---

## Conclusions & Implementation Strategy
To achieve single-digit component counts on large instances, we will evolve the current LNS into a **Conflict-Aware ALNS with SA Acceptance**.

### Phase 1: Conflict-Aware Destruction
Instead of random removal, target components that are "near" each other in $T_1$ but distant in $T_2$ (topological conflicts).

### Phase 2: SA Acceptance Engine
Replace the current "always accept better" rule with a Simulated Annealing engine to allow the solver to bypass local optima.

### Phase 3: Targeted Preprocessing
Aggressive maximal subtree reduction (MCSR) must be used to minimize the initial number of labels.
