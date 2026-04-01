# Issue: Validity Failure on Large Instances

## Description
The solver returns `False` (Invalid) when running on large instances like `heuristic28.nw`.

## Symptoms
- `verify_maf.py` reports `False`.
- Small instances (e.g., caterpillar cases) pass, suggesting the core logic is sound but edge cases or scale-related bugs exist.

## Potential Causes
1.  **Sentinel Collision**: `EMPTY_SENTINEL` (2,000,000,000) might collide with internal node counts or large labels.
2.  **Virtual ID Management**: During cherry contraction, virtual IDs might not be handled correctly across $T_1$ and $T_2$.
3.  **Mask Inconsistency**: `FastBitSet` masks might not be updated perfectly after nested contractions.
4.  **Isomorphism False Positives**: `is_truly_isomorphic` might return true for subtrees that are not actually agreement subtrees in the original trees.

## Reproduction
```bash
./pace2026_rs/target/release/pace2026_rs instances/heuristic28.nw --time-limit 10 > fail.txt
python3 verify_maf.py instances/heuristic28.nw fail.txt
```
