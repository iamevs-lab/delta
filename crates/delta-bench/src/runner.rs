use crate::workload::Workload;
use delta_text::LanguagePipeline;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BenchCase {
    pub mode: String,
    pub elapsed_nanos: u128,
    pub elapsed_ms: f64,
    pub bytes_processed: usize,
    pub source_len: usize,
    pub tokens_reused: usize,
    pub tokens_recomputed: usize,
    pub ast_reused: usize,
    pub ast_recomputed: usize,
    pub correct: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BenchReport {
    pub workload: String,
    pub seed: u64,
    pub size_bytes: usize,
    pub opt: String,
    pub profile: String,
    pub full: BenchCase,
    pub incremental: BenchCase,
    pub incremental_faster: Option<bool>,
    pub delta_over_n_bytes: Option<f64>,
    pub notes: String,
}

pub fn run_compare(workload: &Workload) -> Result<BenchReport, String> {
    let mut full_pipe = LanguagePipeline::from_text(&workload.initial);
    let mut inc_pipe = LanguagePipeline::from_text(&workload.initial);

    let t_full = Instant::now();
    let mut full_bytes = 0usize;
    for d in &workload.deltas {
        let snap = full_pipe.apply_full(d).map_err(|e| e.to_string())?;
        full_bytes += snap.bytes_processed;
    }
    let full_elapsed = t_full.elapsed();

    let t_inc = Instant::now();
    let mut inc_snap = None;
    let mut inc_bytes = 0usize;
    for d in &workload.deltas {
        let snap = inc_pipe.apply_incremental(d).map_err(|e| e.to_string())?;
        inc_bytes += snap.bytes_processed;
        inc_snap = Some(snap);
    }
    let inc_elapsed = t_inc.elapsed();

    let correct = inc_pipe.source == full_pipe.source
        && inc_pipe.tokens.semantic_eq(&full_pipe.tokens)
        && inc_pipe.program.semantic_eq(&full_pipe.program)
        && inc_pipe.symbols.semantic_eq(&full_pipe.symbols)
        && inc_pipe.diagnostics.semantic_eq(&full_pipe.diagnostics);

    if !correct {
        return Err("incremental result != full oracle".into());
    }

    let inc = inc_snap.unwrap_or_default();
    let full_case = BenchCase {
        mode: "full".into(),
        elapsed_nanos: full_elapsed.as_nanos(),
        elapsed_ms: full_elapsed.as_secs_f64() * 1000.0,
        bytes_processed: full_bytes,
        source_len: full_pipe.source.len(),
        tokens_reused: 0,
        tokens_recomputed: full_pipe.tokens.tokens.len(),
        ast_reused: 0,
        ast_recomputed: full_pipe.program.work.items_total,
        correct: true,
    };
    let inc_case = BenchCase {
        mode: "incremental".into(),
        elapsed_nanos: inc_elapsed.as_nanos(),
        elapsed_ms: inc_elapsed.as_secs_f64() * 1000.0,
        bytes_processed: inc_bytes,
        source_len: inc_pipe.source.len(),
        tokens_reused: inc.tokens_reused,
        tokens_recomputed: inc.tokens_recomputed,
        ast_reused: inc.ast_reused,
        ast_recomputed: inc.ast_recomputed,
        correct: true,
    };

    let delta_over_n = if full_bytes == 0 {
        None
    } else {
        Some(inc_bytes as f64 / full_bytes as f64)
    };

    Ok(BenchReport {
        workload: workload.name.clone(),
        seed: workload.seed,
        size_bytes: workload.size_bytes,
        opt: opt_label(),
        profile: profile_label(),
        incremental_faster: Some(inc_elapsed < full_elapsed),
        delta_over_n_bytes: delta_over_n,
        notes: if inc_elapsed >= full_elapsed {
            "incremental was not faster on this run (allowed; see experiment 010)".into()
        } else {
            "incremental latency was lower on this run; do not generalize".into()
        },
        full: full_case,
        incremental: inc_case,
    })
}

fn opt_label() -> String {
    if cfg!(debug_assertions) {
        "debug".into()
    } else {
        "release".into()
    }
}

fn profile_label() -> String {
    std::env::var("PROFILE").unwrap_or_else(|_| opt_label())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workload::{generate, WorkloadKind};

    #[test]
    fn smoke_compare_is_correct() {
        let w = generate(WorkloadKind::TextSmoke, 2048, 1);
        let report = run_compare(&w).unwrap();
        assert!(report.full.correct);
        assert!(report.incremental.correct);
    }
}
