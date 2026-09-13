use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Nanoseconds, so reports stay machine-readable without `Duration` quirks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DurationNanos(pub u128);

impl DurationNanos {
    pub fn from_duration(d: Duration) -> Self {
        Self(d.as_nanos())
    }

    pub fn as_millis_f64(self) -> f64 {
        self.0 as f64 / 1_000_000.0
    }
}

/// Work performed by one computation or one edit apply.
///
/// These counters are how we study `Δ / N`. They are not marketing.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkMetrics {
    pub bytes_processed: usize,
    pub allocations_hint: u64,
    pub items_reused: usize,
    pub items_recomputed: usize,
    pub items_invalidated: usize,
    pub items_total: usize,
    pub hashing_nanos: DurationNanos,
}

impl WorkMetrics {
    pub fn merge(&mut self, other: &WorkMetrics) {
        self.bytes_processed += other.bytes_processed;
        self.allocations_hint += other.allocations_hint;
        self.items_reused += other.items_reused;
        self.items_recomputed += other.items_recomputed;
        self.items_invalidated += other.items_invalidated;
        self.items_total += other.items_total;
        self.hashing_nanos.0 += other.hashing_nanos.0;
    }

    pub fn affected_ratio(&self) -> Option<f64> {
        if self.items_total == 0 {
            None
        } else {
            Some(self.items_recomputed as f64 / self.items_total as f64)
        }
    }
}
