import verify_maf
import sys
if len(sys.argv) < 3:
    print("Usage: python3 test_diff.py <inst_path> <sol_path>")
    sys.exit(1)
inst_path = sys.argv[1]
sol_path = sys.argv[2]
output_text = open(sol_path).read()
res = verify_maf.verify_maf(inst_path, output_text)
print("Verification Result:", res)
f = open(inst_path, 'r')
lines = [l.strip() for l in f if l.strip() and not l.startswith('#')]
_, leaves1 = verify_maf.parse_newick_to_ancestry(lines[0])
comp_texts = [c.strip() for c in output_text.strip().split(';') if c.strip()]
all_forest_leaves = set()
for comp_text in comp_texts:
    _, comp_leaves = verify_maf.parse_newick_to_ancestry(comp_text)
    all_forest_leaves |= comp_leaves
print('Leaves1 size:', len(leaves1))
print('ForestLeaves size:', len(all_forest_leaves))
print('Diff (leaves1 - forest):', sorted(list(leaves1 - all_forest_leaves)))
print('Diff (forest - leaves1):', sorted(list(all_forest_leaves - leaves1)))
