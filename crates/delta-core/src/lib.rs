//! Core data model for EVS Delta.
//!
//! This crate is domain-agnostic. It does not lex, parse, or talk to editors.
//!
//! The questions it can answer:
//! - What is a change? (`Delta`)
//! - How is a computation identified? (`NodeId`)
//! - How do we know a result is current? (`Version`, `Fingerprint`, `NodeStatus`)

pub mod delta;
pub mod error;
pub mod fingerprint;
pub mod id;
pub mod metrics;
pub mod range;
pub mod status;
pub mod version;

pub use delta::{apply_delta, apply_delta_mut, ApplyStats, Delta};
pub use error::{Error, Result};
pub use fingerprint::{fingerprint_bytes, Fingerprint, FingerprintTimer};
pub use id::NodeId;
pub use metrics::{DurationNanos, WorkMetrics};
pub use range::ByteRange;
pub use status::{InvalidationReason, NodeStatus};
pub use version::Version;

#[cfg(test)]
mod proptests;
