#!/usr/bin/env python3
import sys
import argparse
from typing import Optional, Sequence

from .core.io import parse_instance, render_expansion
from .search.beam import solve_beam
from .search.local_search import solve_local_search
from .search.sa import solve_sa
from .search.lp import solve_lp_relaxation

def build_arg_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="PACE 2026 heuristic-track MVP solver (Refactored).")
    parser.add_argument("input", nargs="?", default="-", help="input file path or '-' for stdin")
    parser.add_argument("--heuristic", choices=["beam", "local", "sa", "lp"], default="beam", help="Search algorithm to use")
    parser.add_argument("--beam-width", type=int, default=8, help="Beam width (for beam search)")
    parser.add_argument("--candidate-limit", type=int, default=12, help="Candidate limit per state")
    parser.add_argument("--time-limit-seconds", type=float, default=5.0, help="Time limit in seconds")
    return parser

def main(argv: Optional[Sequence[str]] = None) -> int:
    parser = build_arg_parser()
    args = parser.parse_args(argv)
    
    if args.input == "-":
        text = sys.stdin.read()
    else:
        with open(args.input, "r", encoding="utf-8") as f:
            text = f.read()
            
    instance = parse_instance(text)
    
    if args.heuristic == "beam":
        components = solve_beam(instance, args.beam_width, args.candidate_limit, args.time_limit_seconds)
    elif args.heuristic == "local":
        components = solve_local_search(instance, args.time_limit_seconds)
    elif args.heuristic == "sa":
        components = solve_sa(instance, args.time_limit_seconds)
    elif args.heuristic == "lp":
        components = solve_lp_relaxation(instance, args.time_limit_seconds)
    else:
        components = []

    output = "\n".join(f"{render_expansion(comp)};" for comp in components) + "\n"
    sys.stdout.write(output)
    return 0

if __name__ == "__main__":
    sys.exit(main())
