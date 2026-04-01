import os
import subprocess
import time
from verify_maf import verify_maf

def run_bench():
    instances = [
        "instances/heuristic01.nw", # 80 leaves
        "instances/heuristic05.nw", # 350 leaves
        "instances/heuristic98.nw", # 10k leaves
        "instances/heuristic28.nw"  # 15k leaves
    ]
    time_limits = [30, 60, 120]
    
    print(f"{'Instance':<25} | {'Limit':<5} | {'Comp':<6} | {'Valid':<6} | {'Time':<6}")
    print("-" * 60)
    
    for inst in instances:
        if not os.path.exists(inst):
            continue
        for limit in time_limits:
            start = time.time()
            try:
                proc = subprocess.run(
                    ["./pace2026_rs/target/release/pace2026_rs", inst, "--time-limit", str(limit)],
                    capture_output=True, text=True, timeout=limit + 5
                )
                elapsed = time.time() - start
                output = proc.stdout.strip()
                
                if not output:
                    print(f"{os.path.basename(inst):<25} | {limit:<5}s | {'EMPTY':<6} | {'False':<6} | {elapsed:.1f}s")
                    continue

                valid, count = verify_maf(inst, output)
                print(f"{os.path.basename(inst):<25} | {limit:<5}s | {count if valid else 'ERR':<6} | {str(valid):<6} | {elapsed:.1f}s")
            except subprocess.TimeoutExpired:
                print(f"{os.path.basename(inst):<25} | {limit:<5}s | {'TO':<6} | {'False':<6} | {limit+5:.1f}s")
            except Exception as e:
                print(f"{os.path.basename(inst):<25} | {limit:<5}s | {'FAIL':<6} | {'False':<6} | {time.time()-start:.1f}s")

if __name__ == "__main__":
    print("Rebuilding Rust solver...")
    subprocess.run("source $HOME/.cargo/env && cd pace2026_rs && cargo build --release", shell=True, executable="/bin/bash")
    run_bench()
