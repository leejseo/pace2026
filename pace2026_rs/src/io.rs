use crate::tree::{OriginalNode};
use anyhow::{Result, bail};
use std::fs;

#[derive(Debug)]
pub struct Instance {
    pub n_leaves: u32,
    pub tree1: OriginalNode,
    pub tree2: OriginalNode,
}

pub fn parse_newick(text: &str) -> Result<OriginalNode> {
    let mut stack: Vec<Vec<OriginalNode>> = Vec::new();
    let mut current_nodes = Vec::new();
    
    let chars: Vec<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
    let mut i = 0;
    
    while i < chars.len() {
        let ch = chars[i];
        if ch == '(' {
            stack.push(current_nodes);
            current_nodes = Vec::new();
            i += 1;
        } else if ch.is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let label: u32 = chars[start..i].iter().collect::<String>().parse()?;
            current_nodes.push(OriginalNode { left: None, right: None, label: Some(label) });
        } else if ch == ',' {
            i += 1;
        } else if ch == ')' {
            if current_nodes.len() != 2 {
                bail!("Expected exactly 2 children in Newick, found {}", current_nodes.len());
            }
            let right = current_nodes.pop().unwrap();
            let left = current_nodes.pop().unwrap();
            let node = OriginalNode { left: Some(Box::new(left)), right: Some(Box::new(right)), label: None };
            current_nodes = stack.pop().ok_or_else(|| anyhow::anyhow!("Unexpected ')'"))?;
            current_nodes.push(node);
            i += 1;
        } else if ch == ';' {
            break;
        } else {
            i += 1; // Ignore other characters or bail? Let's just skip for robustness
        }
    }
    
    if current_nodes.len() != 1 {
        bail!("Expected single root node, found {}", current_nodes.len());
    }
    Ok(current_nodes.pop().unwrap())
}

pub fn parse_instance_file(path: &str) -> Result<Instance> {
    let text = fs::read_to_string(path)?;
    let mut newicks = Vec::new();
    let mut n_leaves = 0;
    
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("#p") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 3 {
                n_leaves = parts[2].parse().unwrap_or(0);
            }
        } else if !trimmed.starts_with('#') && trimmed.ends_with(';') {
            newicks.push(trimmed.to_string());
        }
    }
    
    if newicks.len() < 2 {
        bail!("Could not find two Newick strings in file");
    }
    
    let tree1 = parse_newick(&newicks[0])?;
    let tree2 = parse_newick(&newicks[1])?;
    
    Ok(Instance { n_leaves, tree1, tree2 })
}

pub fn render_expansion(exp: &crate::tree::Expansion) -> String {
    let mut result = String::new();
    fn recurse(e: &crate::tree::Expansion, res: &mut String) {
        match e {
            crate::tree::Expansion::Leaf(id) => { res.push_str(&id.to_string()); }
            crate::tree::Expansion::Node(l, r, _) => {
                res.push('(');
                recurse(l, res);
                res.push(',');
                recurse(r, res);
                res.push(')');
            }
        }
    }
    recurse(exp, &mut result);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deep_newick_parser() {
        // Create a very deep tree: (((...(1,2),3),4),...N);
        let n = 10000;
        let mut s = String::new();
        for _ in 0..n-1 { s.push('('); }
        s.push_str("1,2)");
        for i in 3..=n {
            s.push_str(&format!(",{})", i));
        }
        s.push(';');
        
        let res = parse_newick(&s);
        assert!(res.is_ok());
        let root = res.unwrap();
        // Check if it's actually deep by going down the left side
        let mut curr = &root;
        let mut depth = 0;
        while let Some(ref l) = curr.left {
            curr = l;
            depth += 1;
        }
        assert_eq!(depth, n - 1);
    }
}
