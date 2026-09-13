use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphLimits {
    pub max_nodes: usize,
    pub max_edges: usize,
    pub max_invalidation_nodes: usize,
    pub max_schedule_nodes: usize,
}

impl Default for GraphLimits {
    fn default() -> Self {
        Self {
            max_nodes: 100_000,
            max_edges: 1_000_000,
            max_invalidation_nodes: 100_000,
            max_schedule_nodes: 100_000,
        }
    }
}
