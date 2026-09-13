use serde::{Deserialize, Serialize};

/// How the engine chooses between incremental and full recomputation.
///
/// Incremental is a hypothesis. This policy exists because the hypothesis
/// is allowed to lose.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum RecomputePolicy {
    #[default]
    AlwaysIncremental,
    AlwaysFull,
    Adaptive(AdaptiveSettings),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AdaptiveSettings {
    /// If `affected / total` exceeds this, choose full.
    pub affected_ratio_threshold: f64,
    /// Extra cost charged per invalidated node (arbitrary units).
    pub traversal_cost_per_node: f64,
}

impl Default for AdaptiveSettings {
    fn default() -> Self {
        Self {
            affected_ratio_threshold: 0.60,
            traversal_cost_per_node: 0.05,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RecomputeDecision {
    Incremental {
        estimated_affected_ratio: f64,
    },
    Full {
        reason: String,
        estimated_affected_ratio: f64,
    },
}

impl RecomputeDecision {
    pub fn is_full(&self) -> bool {
        matches!(self, Self::Full { .. })
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Incremental { .. } => "incremental",
            Self::Full { .. } => "full",
        }
    }
}

impl RecomputePolicy {
    pub fn decide(&self, affected: usize, total: usize) -> RecomputeDecision {
        let ratio = if total == 0 {
            0.0
        } else {
            affected as f64 / total as f64
        };
        match self {
            Self::AlwaysIncremental => RecomputeDecision::Incremental {
                estimated_affected_ratio: ratio,
            },
            Self::AlwaysFull => RecomputeDecision::Full {
                reason: "policy=AlwaysFull".into(),
                estimated_affected_ratio: ratio,
            },
            Self::Adaptive(settings) => {
                if ratio > settings.affected_ratio_threshold {
                    RecomputeDecision::Full {
                        reason: format!(
                            "affected_ratio {ratio:.3} > threshold {:.3}",
                            settings.affected_ratio_threshold
                        ),
                        estimated_affected_ratio: ratio,
                    }
                } else {
                    RecomputeDecision::Incremental {
                        estimated_affected_ratio: ratio,
                    }
                }
            }
        }
    }

    pub fn estimate_costs(&self, affected: usize, total: usize) -> (f64, f64) {
        let inc = affected as f64;
        let full = total as f64;
        match self {
            Self::Adaptive(s) => (inc + s.traversal_cost_per_node * affected as f64, full),
            _ => (inc, full),
        }
    }
}
