use crate::tree::{Tree, Expansion};

#[derive(Clone, Debug)]
pub struct State {
    pub tree1: Arc<Tree>,
    pub tree2: Arc<Tree>,
    pub components: Vec<Vec<u32>>,
}

use std::sync::Arc;
