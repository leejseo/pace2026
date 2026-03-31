import os
import subprocess
import time
from datetime import datetime
from verify_maf import verify_maf
import threading

def run_test():
    # 3 Difficult instances based on leaf count
    instances = ["instances/heuristic28.nw", "instances/heuristic26.nw", "instances/heuristic98.nw"]
    durations = [60, 120, 180, 300]
    
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    log_file = f"results/hard_benchmark_{timestamp}.txt"
    
    print(f"Starting HARD benchmark. Results will be saved to {log_file}")
    
    header = f"{'Instance':<20} | {'1m Comp':<8} | {'2m Comp':<8} | {'3m Comp':<8} | {'5m Comp':<8}"
    separator = "-" * 75
    
    with open(log_file, "w") as f:
        f.write(f"Hard Benchmark Run: {timestamp}\n")
        f.write(separator + "\n")
        f.write(header + "\n")
        f.write(separator + "\n")
        
        print(header)
        print(separator)
        
        for inst in instances:
            if not os.path.exists(inst):
                print(f"File not found: {inst}")
                continue
                
            row_data = [os.path.basename(inst)]
            for d in durations:
                print(f"  Running {os.path.basename(inst)} for {d}s...", end="", flush=True)
                
                stop_dots = False
                def print_dots():
                    while not stop_dots:
                        print(".", end="", flush=True)
                        time.sleep(15)
                dot_thread = threading.Thread(target=print_dots)
                dot_thread.start()
                
                try:
                    result = subprocess.run(
                        ["./pace2026_rs/target/release/pace2026_rs", inst, "--time-limit", str(d)],
                        capture_output=True, text=True, timeout=d + 30
                    )
                    stop_dots = True
                    dot_thread.join()
                    
                    if result.returncode == 0:
                        is_valid, comp_or_err = verify_maf(inst, result.stdout)
                        if is_valid:
                            row_data.append(str(comp_or_err))
                            print(f" OK ({comp_or_err})")
                        else:
                            row_data.append("INV")
                            print(f" INV: {comp_or_err}")
                    else:
                        row_data.append("ERR")
                        print(f" ERR: {result.stderr[:50]}")
                except subprocess.TimeoutExpired:
                    stop_dots = True
                    dot_thread.join()
                    row_data.append("TO")
                    print(" TO")
                except Exception as e:
                    stop_dots = True
                    dot_thread.join()
                    row_data.append("EXC")
                    print(f" EXC: {e}")
            
            row_str = f"{row_data[0]:<20} | {row_data[1]:<8} | {row_data[2]:<8} | {row_data[3]:<8} | {row_data[4]:<8}"
            print(row_str)
            f.write(row_str + "\n")

    # Update summary
    with open("results/summary.md", "a") as f:
        f.write(f"\n### Hard Benchmark Run {timestamp}\n")
        f.write("| Instance | 1m | 2m | 3m | 5m |\n")
        f.write("| --- | --- | --- | --- | --- |\n")
        with open(log_file, "r") as lf:
            lines = lf.readlines()[4:]
            for line in lines:
                parts = [p.strip() for p in line.split("|")]
                if len(parts) >= 5:
                    f.write(f"| {' | '.join(parts)} |\n")

if __name__ == "__main__":
    print("Ensuring Rust solver is built...")
    subprocess.run("source $HOME/.cargo/env && cargo build --release", cwd="pace2026_rs", shell=True, executable="/bin/bash")
    run_test()
