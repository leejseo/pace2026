import os
import subprocess
import time
import glob
import re

def parse_newick_labels(text):
    # Only look at lines that don't start with # and end with ;
    labels = set()
    for line in text.splitlines():
        line = line.strip()
        if line and not line.startswith("#"):
            # Extract numbers that are not part of the tree structure syntax
            labels.update(re.findall(r'\d+', line))
    return labels

def validate_solution(inst_path, output_text):
    # 1. Get original labels from input file
    with open(inst_path, "r") as f:
        input_text = f.read()
    original_labels = parse_newick_labels(input_text)
    
    # 2. Get solution labels from solver output
    # Ignore any lines in output that start with #
    solution_lines = [l.strip() for l in output_text.strip().splitlines() if l.strip() and not l.startswith("#")]
    output_clean = "".join(solution_lines)
    
    components = [c.strip() for c in output_clean.split(';') if c.strip()]
    solution_labels = []
    for comp in components:
        solution_labels.extend(re.findall(r'\d+', comp))
    
    solution_set = set(solution_labels)
    
    # 3. Check if it's a valid partition
    if solution_set != original_labels:
        missing = original_labels - solution_set
        extra = solution_set - original_labels
        return False, f"Mismatch: missing {len(missing)}, extra {len(extra)}"
    
    if len(solution_labels) != len(solution_set):
        return False, "Duplicate labels found in solution"
    
    return True, len(components)

from verify_maf import verify_maf

def run_benchmark(cmd_prefix, time_limit=5.0):
    instances = sorted(glob.glob("instances/heuristic*.nw"))
    subset = instances[::10] # 00, 10, 20...
    
    results = []
    print(f"{'Instance':<15} | {'Leaves':<8} | {'Status':<10} | {'Comp':<6} | {'Time (s)':<8}")
    print("-" * 60)
    
    for inst_path in subset:
        # Get leaf count safely
        with open(inst_path, "r") as f:
            content = f.read()
            n_leaves = len(re.findall(r'\d+', content)) // 2 # Rough estimate
        
        start_time = time.monotonic()
        try:
            cmd = cmd_prefix + [inst_path, "--time-limit", str(int(time_limit))]
            result = subprocess.run(cmd, capture_output=True, text=True, timeout=time_limit + 10)
            elapsed = time.monotonic() - start_time
            
            if result.returncode == 0:
                is_valid, val_result = verify_maf(inst_path, result.stdout)
                if is_valid:
                    print(f"{os.path.basename(inst_path):<15} | {n_leaves:<8} | {'OK':<10} | {val_result:<6} | {elapsed:<8.2f}")
                    results.append((val_result, elapsed))
                else:
                    print(f"{os.path.basename(inst_path):<15} | {n_leaves:<8} | {'INVALID':<10} | {'-':<6} | {elapsed:<8.2f}")
                    print(f"  Reason: {val_result}")
            else:
                print(f"{os.path.basename(inst_path):<15} | {n_leaves:<8} | {'ERROR':<10} | {'-':<6} | {elapsed:<8.2f}")
        except subprocess.TimeoutExpired:
            print(f"{os.path.basename(inst_path):<15} | {n_leaves:<8} | {'TIMEOUT':<10} | {'-':<6} | {'-':<8}")
            
    if results:
        avg_comp = sum(r[0] for r in results) / len(results)
        avg_time = sum(r[1] for r in results) / len(results)
        print("-" * 60)
        print(f"SUMMARY: Avg Comp: {avg_comp:.2f}, Avg Time: {avg_time:.2f}s")
    return results

if __name__ == "__main__":
    import sys
    limit = 300.0 if "--long" in sys.argv else 10.0
    print(f"\n=== BENCHMARKING RUST SOLVER (ANYTIME) - LIMIT {limit}s ===")
    run_benchmark(["./pace2026_rs/target/release/pace2026_rs"], time_limit=limit)
