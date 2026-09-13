use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GraphStats {
    pub total_nodes: usize,
    pub dirty_nodes: usize,
    pub clean_nodes: usize,
    pub failed_nodes: usize,
    pub pending_nodes: usize,
    pub dependency_edges: usize,
    pub max_dependency_depth: usize,
    pub max_fanout: usize,
}
