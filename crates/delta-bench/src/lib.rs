//! Benchmark laboratory.
//!
//! Every result is produced by running both full and incremental on the
//! same seed and the same delta sequence. Numbers that do not exist yet
//! are not written.

mod replay;
mod runner;
mod workload;

pub use replay::{load_replay, save_replay, ReplayFile};
pub use runner::{run_compare, BenchCase, BenchReport};
pub use workload::{edit_at, generate, workload_names, Workload, WorkloadKind};
