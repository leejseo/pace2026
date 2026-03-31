import os
from verify_maf import verify_maf

def test_evaluator():
    # Setup a tiny instance for testing
    inst_path = "test_instance.nw"
    with open(inst_path, "w") as f:
        f.write("(1,(2,3));\n") # T1
        f.write("((1,2),3);\n") # T2
    
    print("Testing Evaluator Accuracy...")

    # Case 1: Valid partition (1; 2; 3;)
    valid_sol = "1; 2; 3;"
    ok, res = verify_maf(inst_path, valid_sol)
    assert ok == True and res == 3, f"Failed Case 1: {res}"
    print("✅ Case 1: Valid partition detected correctly.")

    # Case 2: Missing leaf (1; 2;)
    invalid_sol = "1; 2;"
    ok, res = verify_maf(inst_path, invalid_sol)
    assert ok == False and "Leaf set mismatch" in res, f"Failed Case 2: {res}"
    print("✅ Case 2: Missing leaf detected correctly.")

    # Case 3: Duplicate leaf (1; 2; 3; 1;)
    dup_sol = "1; 2; 3; 1;"
    ok, res = verify_maf(inst_path, dup_sol)
    assert ok == False and "Duplicate leaves" in res, f"Failed Case 3: {res}"
    print("✅ Case 3: Duplicate leaf detected correctly.")

    # Case 4: Non-agreement subtree
    # In T1, (2,3) is a cherry. In T2, (1,2) is a cherry. 
    # (2,3) is NOT an agreement subtree if 1 is present in the forest elsewhere.
    # Let's try: (2,3); 1;
    non_agree_sol = "(2,3); 1;"
    ok, res = verify_maf(inst_path, non_agree_sol)
    # Induced subtree of {2,3} in T2 is just 2 and 3 connected to root, but structure matches (2,3)
    # Wait, in rooted MAF, (2,3) in T1 induced in T2 must be (2,3).
    # In T2 = ((1,2),3), induced subtree of {2,3} is NOT (2,3), it's just 2 and 3 siblings of 1.
    # Actually, induced subtree suppresses degree-2 nodes. 
    # T2 induced {2,3} -> path 2-node-root-node-3 -> simplified to 2-3. Matches (2,3).
    # So (2,3); 1; IS a valid Agreement Forest.
    
    # Let's try an actually invalid one: 
    # T1 = (1,(2,3)), T2 = ((1,2),3)
    # The only way they don't agree is if we try to say the whole tree agrees.
    bad_agree_sol = "(1,(2,3));"
    ok, res = verify_maf(inst_path, bad_agree_sol)
    assert ok == False, "Failed Case 4: Should detected that T1 and T2 are not isomorphic"
    print("✅ Case 4: Non-isomorphic trees detected correctly.")

    os.remove(inst_path)
    print("\nALL EVALUATOR TESTS PASSED!")

if __name__ == "__main__":
    test_evaluator()
