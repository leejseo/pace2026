import os
import subprocess
import time
import glob
import re
from datetime import datetime

def get_comp_count(output):
    solution_lines = [l.strip() for l in output.strip().splitlines() if l.strip() and not l.startswith("#")]
    output_clean = "".join(solution_lines)
    return len([c for c in output_clean.split(';') if c.strip()])

def run_test():
    # Use 3 representative instances
    instances = ["instances/heuristic00.nw", "instances/heuristic10.nw", "instances/heuristic20.nw"]
    durations = [30, 60, 120]
    
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    log_file = f"results/benchmark_{timestamp}.txt"
    
    print(f"Starting benchmark. Results will be saved to {log_file}")
    
    header = f"{'Instance':<20} | {'30s Comp':<8} | {'60s Comp':<8} | {'120s Comp':<8}"
    separator = "-" * 55
    
    with open(log_file, "w") as f:
        f.write(f"Benchmark Run: {timestamp}\n")
        f.write(separator + "\n")
        f.write(header + "\n")
        f.write(separator + "\n")
        
        print(header)
        print(separator)
        
        for inst in instances:
            if not os.path.exists(inst):
                continue
                
            row_data = [os.path.basename(inst)]
            for d in durations:
                print(f"  Running {os.path.basename(inst)} for {d}s...", end="", flush=True)
                # For long runs, we might want to use a thread to print dots
                import threading
                stop_dots = False
                def print_dots():
                    while not stop_dots:
                        print(".", end="", flush=True)
                        time.sleep(10)
                dot_thread = threading.Thread(target=print_dots)
                dot_thread.start()
                
                try:
                    result = subprocess.run(
                        ["./pace2026_rs/target/release/pace2026_rs", inst, "--time-limit", str(d)],
                        capture_output=True, text=True, timeout=d + 15
                    )
                    stop_dots = True
                    dot_thread.join()
                    
                    if result.returncode == 0:
                        comp = get_comp_count(result.stdout)
                        row_data.append(str(comp))
                        print(f" Done ({comp} components)")
                    else:
                        row_data.append("ERR")
                        print(" Error")
                except subprocess.TimeoutExpired:
                    stop_dots = True
                    dot_thread.join()
                    row_data.append("TO")
                    print(" Timeout")
                except Exception as e:
                    stop_dots = True
                    dot_thread.join()
                    row_data.append("EXC")
                    print(f" Exception: {e}")
            
            row_str = f"{row_data[0]:<20} | {row_data[1]:<8} | {row_data[2]:<8} | {row_data[3]:<8}"
            print(row_str)
            f.write(row_str + "\n")

    # Append to summary
    with open("results/summary.md", "a") as f:
        f.write(f"\n### Run {timestamp}\n")
        f.write("| Instance | 30s | 60s | 120s |\n")
        f.write("| --- | --- | --- | --- |\n")
        # Read the last lines from log_file to append to summary
        with open(log_file, "r") as lf:
            lines = lf.readlines()[4:] # Skip headers
            for line in lines:
                parts = [p.strip() for p in line.split("|")]
                f.write(f"| {' | '.join(parts)} |\n")

if __name__ == "__main__":
    print("Compiling Rust solver...")
    subprocess.run("source $HOME/.cargo/env && cargo build --release", cwd="pace2026_rs", shell=True, executable="/bin/bash")
    run_test()
    
    # Git push
    print("Pushing results to Git...")
    subprocess.run("git add . && git commit -m 'Add benchmark results' && git push", shell=True)
