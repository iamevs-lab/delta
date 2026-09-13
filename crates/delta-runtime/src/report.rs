use crate::policy::RecomputeDecision;
use delta_core::{DurationNanos, WorkMetrics};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ChangeSummary {
    pub inserted_chars: i64,
    pub deleted_chars: i64,
    pub inserted_bytes: usize,
    pub deleted_bytes: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UpdateReport {
    pub decision: RecomputeDecision,
    pub estimated_incremental_cost: f64,
    pub estimated_full_cost: f64,
    pub actual_cost_ms: f64,
    pub invalidated_nodes: usize,
    pub recomputed_nodes: usize,
    pub reused_nodes: usize,
    pub dirty_nodes: usize,
    pub total_nodes: usize,
    pub failed_nodes: usize,
    pub dependency_edges: usize,
    pub invalidation_fanout: usize,
    pub max_dependency_depth: usize,
    pub execution_nanos: DurationNanos,
    pub invalidation_nanos: DurationNanos,
    pub hashing_nanos: DurationNanos,
    pub work: WorkMetrics,
    pub change: Option<ChangeSummary>,
    pub affected_percent: f64,
    pub reused_percent: f64,
    pub log: Vec<String>,
}

impl Default for UpdateReport {
    fn default() -> Self {
        Self {
            decision: RecomputeDecision::Incremental {
                estimated_affected_ratio: 0.0,
            },
            estimated_incremental_cost: 0.0,
            estimated_full_cost: 0.0,
            actual_cost_ms: 0.0,
            invalidated_nodes: 0,
            recomputed_nodes: 0,
            reused_nodes: 0,
            dirty_nodes: 0,
            total_nodes: 0,
            failed_nodes: 0,
            dependency_edges: 0,
            invalidation_fanout: 0,
            max_dependency_depth: 0,
            execution_nanos: DurationNanos(0),
            invalidation_nanos: DurationNanos(0),
            hashing_nanos: DurationNanos(0),
            work: WorkMetrics::default(),
            change: None,
            affected_percent: 0.0,
            reused_percent: 0.0,
            log: Vec::new(),
        }
    }
}

impl UpdateReport {
    pub fn finalize_percents(&mut self) {
        if self.total_nodes == 0 {
            self.affected_percent = 0.0;
            self.reused_percent = 0.0;
        } else {
            self.affected_percent =
                (self.recomputed_nodes as f64 / self.total_nodes as f64) * 100.0;
            self.reused_percent = (self.reused_nodes as f64 / self.total_nodes as f64) * 100.0;
        }
        self.actual_cost_ms = self.execution_nanos.as_millis_f64();
    }
}
