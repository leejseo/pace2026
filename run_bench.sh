#!/bin/bash
INSTANCES=("instances/heuristic28.nw" "instances/heuristic26.nw")
LIMITS=(60 120 300)
SOLVER="./pace2026_rs/target/release/pace2026_rs"
VALIDATOR="python3 verify_maf.py"

echo "| Instance | Limit (s) | Components | Valid |"
echo "|----------|-----------|------------|-------|"

for inst in "${INSTANCES[@]}"; do
    for limit in "${LIMITS[@]}"; do
        sol="sol_$(basename $inst)_${limit}.txt"
        $SOLVER "$inst" --time-limit "$limit" > "$sol" 2> /dev/null
        res=$($VALIDATOR "$inst" "$sol")
        valid=$(echo "$res" | grep -o "True" || echo "False")
        count=$(echo "$res" | grep -oE "[0-9]+" | head -n 1 || echo "N/A")
        echo "| $(basename $inst) | ${limit} | $count | $valid |"
    done
done
