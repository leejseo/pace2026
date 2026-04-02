use std::collections::{HashSet, HashMap};
use crate::tree::{OriginalNode, Expansion};

pub fn build_expansion_map(node: &OriginalNode, map: &mut HashMap<Expansion, Vec<u32>>) -> Expansion {
    if let Some(id) = node.label {
        let exp = Expansion::Leaf(id);
        map.insert(exp.clone(), vec![id]);
        exp
    } else {
        let l_exp = build_expansion_map(node.left.as_ref().unwrap(), map);
        let r_exp = build_expansion_map(node.right.as_ref().unwrap(), map);
        
        let mut leaves = map.get(&l_exp).unwrap().clone();
        let mut r_leaves = map.get(&r_exp).unwrap().clone();
        leaves.append(&mut r_leaves);
        
        let exp = Expansion::new_node(l_exp, r_exp);
        map.insert(exp.clone(), leaves);
        exp
    }
}

pub fn reduce_tree(
    node: &OriginalNode, 
    common_exps: &HashSet<Expansion>, 
    exp_to_new_id: &HashMap<Expansion, u32>
) -> OriginalNode {
    fn recurse(n: &OriginalNode, common: &HashSet<Expansion>, mapping: &HashMap<Expansion, u32>) -> (OriginalNode, Expansion) {
        if let Some(id) = n.label {
            let exp = Expansion::Leaf(id);
            if common.contains(&exp) {
                if let Some(&new_id) = mapping.get(&exp) {
                    return (OriginalNode { left: None, right: None, label: Some(new_id) }, exp);
                }
            }
            return (n.clone(), exp);
        }
        
        let (l_node, l_exp) = recurse(n.left.as_ref().unwrap(), common, mapping);
        let (r_node, r_exp) = recurse(n.right.as_ref().unwrap(), common, mapping);
        let exp = Expansion::new_node(l_exp, r_exp);
        
        if common.contains(&exp) {
            if let Some(&new_id) = mapping.get(&exp) {
                return (OriginalNode { left: None, right: None, label: Some(new_id) }, exp);
            }
        }
        
        (OriginalNode { left: Some(Box::new(l_node)), right: Some(Box::new(r_node)), label: None }, exp)
    }
    
    recurse(node, common_exps, exp_to_new_id).0
}

pub fn exact_subtree_kernelization(orig1: &OriginalNode, orig2: &OriginalNode, n_leaves: u32) 
    -> (OriginalNode, OriginalNode, HashMap<u32, Vec<u32>>, u32) 
{
    let mut map1 = HashMap::new();
    let mut map2 = HashMap::new();
    
    build_expansion_map(orig1, &mut map1);
    build_expansion_map(orig2, &mut map2);
    
    let common_exps: HashSet<Expansion> = map1.keys().filter(|&k| map2.contains_key(k)).cloned().collect();
    
    let mut exp_to_new_id = HashMap::new();
    let mut new_id_to_old_leaves = HashMap::new();
    let mut next_id = n_leaves + 1;
    
    for exp in &common_exps {
        if map1.get(exp).unwrap().len() > 1 {
            exp_to_new_id.insert(exp.clone(), next_id);
            new_id_to_old_leaves.insert(next_id, map1.get(exp).unwrap().clone());
            next_id += 1;
        }
    }
    
    let k1 = reduce_tree(orig1, &common_exps, &exp_to_new_id);
    let k2 = reduce_tree(orig2, &common_exps, &exp_to_new_id);
    
    for i in 1..=n_leaves {
        let exp = Expansion::Leaf(i);
        if !exp_to_new_id.contains_key(&exp) {
            new_id_to_old_leaves.insert(i, vec![i]);
        }
    }
    
    (k1, k2, new_id_to_old_leaves, next_id)
}
