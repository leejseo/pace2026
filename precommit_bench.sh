#!/bin/bash
set -e
echo "Running Pre-commit Lightweight Benchmark..."
source $HOME/.cargo/env
cd pace2026_rs
cargo build --release
cd ..

# Run small test to ensure validity
python3 -c "
import subprocess
import verify_maf
import sys

def test(inst, limit):
    print(f'Testing {inst} ({limit}s)...', end=' ', flush=True)
    proc = subprocess.run(['./pace2026_rs/target/release/pace2026_rs', inst, '--time-limit', str(limit)], capture_output=True, text=True)
    valid, count = verify_maf.verify_maf(inst, proc.stdout)
    if not valid:
        print(f'FAILED: {count}')
        sys.exit(1)
    print(f'PASSED ({count} components)')

test('instances/heuristic01.nw', 5)
test('instances/heuristic05.nw', 10)
"
echo "All lightweight tests passed!"
