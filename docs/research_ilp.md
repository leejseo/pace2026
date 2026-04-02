# Research: ILP and Conflict Graph Formulation for Rooted MAF

## Core Concepts
To reach the ultimate goal of single-digit components for massive instances (15k+ leaves), we must transition from pure ALNS heuristics to an exact or hybrid approach guided by Mathematical Optimization. 

### 1. Integer Linear Programming (ILP) via Edge Cuts
A standard ILP formulation for the MAF problem focuses on **cutting edges** in $T_1$.
- **Variables**: $x_e \in \{0, 1\}$ for each edge $e \in T_1$. $x_e = 1$ if edge $e$ is cut, $x_e = 0$ otherwise.
- **Objective**: Minimize $\sum x_e$ (the total number of edge cuts).
- **Constraints**: For any incompatible structure (e.g., a cherry $(a,b)$ in $T_1$ that does not exist or is contradicted in $T_2$), at least one edge on the path connecting the conflicting taxa must be cut.
$$\sum_{e \in P(conflict)} x_e \geq 1$$
This is essentially a **Hitting Set** problem.

### 2. Conflict Graph and Maximum Independent Set (MIS)
Alternatively, MAF can be modeled as a **Maximum Weight Independent Set (MWIS)** problem.
- **Vertices ($V$)**: Every valid "potential agreement component" (a subset of leaves whose induced subtrees in $T_1$ and $T_2$ are identical).
- **Edges ($E$)**: An edge exists between two vertices if they conflict (e.g., they share leaves, or their union creates a cycle/topology mismatch).
- **Goal**: Find the largest collection of non-conflicting components that partition the leaf set.

### 3. Hybrid Strategy for Large Instances
Building a full ILP or Conflict Graph for 15,000 leaves is computationally prohibitive (the number of potential components or edges is too large). 
Instead, we use a **Local ILP / Graph Repair**:
1. Run our ultra-fast ALNS to find a "good" partition (e.g., 200 components).
2. For small neighborhoods of these components (e.g., 5-10 adjacent components), build the exact ILP or Conflict Graph.
3. Solve the local problem to optimality to merge them into fewer components.
4. "Freeze" the merged components and repeat.

## Implementation Plan for Iterations 26-30
- **Iteration 26**: Build a rudimentary Conflict Graph Generator for subsets of components to identify specific leaf-level conflicts.
- **Iteration 27**: Implement a Greedy Maximum Independent Set (MIS) repair operator to replace the random ALNS repair for small neighborhoods.
- **Iteration 28**: Formalize the Edge-Cut ILP constraints for the MAF problem into a lightweight external format (LP file) or a simple internal solver for $K \le 20$.
- **Iteration 29**: Integrate the "Local ILP Polish" phase into the final 30 seconds of the `main.rs` run loop.
- **Iteration 30**: Final tuning, benchmark execution, and upstream documentation to deliver the ultimate solver for PACE 2026.
