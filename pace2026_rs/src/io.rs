use crate::tree::{ArenaTree, OriginalNode};
use anyhow::{Result, bail};
use std::fs;

#[derive(Debug)]
pub struct Instance {
    pub n_leaves: u32,
    pub tree1: OriginalNode,
    pub tree2: OriginalNode,
}

pub fn parse_newick(text: &str) -> Result<OriginalNode> {
    let chars: Vec<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
    let mut pos = 0;
    
    fn parse_subtree(chars: &[char], pos: &mut usize) -> Result<OriginalNode> {
        if *pos >= chars.len() {
            bail!("Unexpected end of input");
        }
        let ch = chars[*pos];
        if ch.is_ascii_digit() {
            let start = *pos;
            while *pos < chars.len() && chars[*pos].is_ascii_digit() {
                *pos += 1;
            }
            let num_str: String = chars[start..*pos].iter().collect();
            let label = num_str.parse::<u32>()?;
            return Ok(OriginalNode { left: None, right: None, label: Some(label) });
        }
        
        if ch == '(' {
            *pos += 1; // skip '('
            let left = parse_subtree(chars, pos)?;
            if chars[*pos] != ',' {
                bail!("Expected ','");
            }
            *pos += 1; // skip ','
            let right = parse_subtree(chars, pos)?;
            if chars[*pos] != ')' {
                bail!("Expected ')'");
            }
            *pos += 1; // skip ')'
            return Ok(OriginalNode { left: Some(Box::new(left)), right: Some(Box::new(right)), label: None });
        }
        
        bail!("Unexpected character {}", ch);
    }
    
    let root = parse_subtree(&chars, &mut pos)?;
    if pos < chars.len() && chars[pos] == ';' {
        Ok(root)
    } else {
        bail!("Expected ';' at end")
    }
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
    match exp {
        crate::tree::Expansion::Leaf(id) => id.to_string(),
        crate::tree::Expansion::Node(l, r) => format!("({},{})", render_expansion(l), render_expansion(r)),
    }
}
