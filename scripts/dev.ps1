$ErrorActionPreference = "Stop"
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p delta-cli -- benchmark --workload text-smoke --out benchmarks/results
