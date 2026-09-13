//! Explicit dependency graph.
//!
//! Every node knows its upstream dependencies and downstream dependents.
//! Invalidation is iterative. Cycles are rejected at edge registration.

mod graph;
mod invalidate;
mod limits;
mod stats;
mod topo;

pub use graph::{Graph, NodeInspect, NodeRecord};
pub use invalidate::{InvalidationMode, InvalidationReport};
pub use limits::GraphLimits;
pub use stats::GraphStats;
pub use topo::Schedule;
