use serde::{Deserialize, Serialize};

/// Lifecycle of a computation node.
///
/// `Clean` and "valid output" are the same state: the last computation
/// succeeded and no invalidation has occurred since.
///
/// `Dirty` covers both a local edit and an eager/lazy invalidation.
/// The reason is stored separately so we can inspect *why*.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeStatus {
    /// Registered, never successfully computed.
    Pending,
    /// Output matches current known inputs.
    Clean,
    /// Must recompute before the output is trusted.
    Dirty,
    /// Currently running (single-threaded today; reserved for parallel later).
    Computing,
    /// Last computation returned an error. Still dirty.
    Failed,
}

impl NodeStatus {
    pub fn is_usable(self) -> bool {
        matches!(self, Self::Clean)
    }

    pub fn needs_compute(self) -> bool {
        matches!(self, Self::Pending | Self::Dirty | Self::Failed)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvalidationReason {
    InputChanged,
    DependencyChanged { dependency: String },
    Manual,
    PolicyChoseFull,
    CycleBreak,
}

impl std::fmt::Display for InvalidationReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InputChanged => write!(f, "input changed"),
            Self::DependencyChanged { dependency } => {
                write!(f, "dependency {dependency} changed")
            }
            Self::Manual => write!(f, "manual"),
            Self::PolicyChoseFull => write!(f, "policy chose full recompute"),
            Self::CycleBreak => write!(f, "cycle break"),
        }
    }
}
