//! Computation runtime.
//!
//! Full recomputation is the oracle. Incremental is the experiment.
//! Policy chooses. Decisions are visible.

mod compute;
mod engine;
mod log;
mod policy;
mod report;

pub use compute::{Computation, ComputeCtx, ComputeMode, ComputeOutput};
pub use engine::{Engine, EngineConfig};
pub use log::LogLevel;
pub use policy::{AdaptiveSettings, RecomputeDecision, RecomputePolicy};
pub use report::{ChangeSummary, UpdateReport};
