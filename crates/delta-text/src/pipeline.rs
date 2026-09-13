use crate::diagnostics::{diagnostics_full, diagnostics_incremental, DiagnosticSet};
use crate::lexer::{lex_full, lex_incremental, TokenStream};
use crate::parser::{parse_full, parse_incremental, Program};
use crate::symbols::{symbols_full, symbols_incremental, SymbolTable};
use delta_core::{apply_delta_mut, Delta, DurationNanos, Result, WorkMetrics};
use delta_runtime::{Computation, ComputeCtx, ComputeOutput, Engine, UpdateReport};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Standalone pipeline used by benches, replay, and Neovim.
/// The generic engine is also wired via [`install_language_graph`].
#[derive(Clone, Debug)]
pub struct LanguagePipeline {
    pub source: String,
    pub tokens: TokenStream,
    pub program: Program,
    pub symbols: SymbolTable,
    pub diagnostics: DiagnosticSet,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PipelineSnapshot {
    pub inserted_chars: i64,
    pub deleted_chars: i64,
    pub tokens_total: usize,
    pub tokens_reused: usize,
    pub tokens_recomputed: usize,
    pub ast_total: usize,
    pub ast_reused: usize,
    pub ast_recomputed: usize,
    pub symbols_total: usize,
    pub symbols_reused: usize,
    pub symbols_recomputed: usize,
    pub diagnostics_total: usize,
    pub diagnostics_reused: usize,
    pub diagnostics_recomputed: usize,
    pub diagnostics_before: usize,
    pub diagnostics_after: usize,
    pub bytes_processed: usize,
    pub elapsed_nanos: u128,
    pub source_len: usize,
    pub mode: String,
}

impl LanguagePipeline {
    pub fn from_text(text: impl Into<String>) -> Self {
        let source = text.into();
        let tokens = lex_full(&source);
        let program = parse_full(&tokens);
        let symbols = symbols_full(&program);
        let diagnostics = diagnostics_full(&program, &symbols);
        Self {
            source,
            tokens,
            program,
            symbols,
            diagnostics,
        }
    }

    pub fn apply_incremental(&mut self, delta: &Delta) -> Result<PipelineSnapshot> {
        let t0 = Instant::now();
        let before_diag = self.diagnostics.items.len();
        let (ins, del) = delta.net_char_delta();
        let stats = apply_delta_mut(&mut self.source, delta)?;
        self.tokens = lex_incremental(&self.source, &self.tokens, delta)?;
        self.program = parse_incremental(&self.tokens, &self.program)?;
        let dirty = dirty_stmt_ids(&self.program);
        self.symbols = symbols_incremental(&self.program, &self.symbols, &dirty);
        let dirty = expand_symbol_fanout(&self.symbols, &dirty);
        self.diagnostics =
            diagnostics_incremental(&self.program, &self.symbols, &self.diagnostics, &dirty);
        Ok(self.snapshot(
            "incremental",
            ins,
            del,
            stats.bytes_processed,
            before_diag,
            t0,
        ))
    }

    pub fn apply_full(&mut self, delta: &Delta) -> Result<PipelineSnapshot> {
        let t0 = Instant::now();
        let before_diag = self.diagnostics.items.len();
        let (ins, del) = delta.net_char_delta();
        let stats = apply_delta_mut(&mut self.source, delta)?;
        self.rebuild_full();
        Ok(self.snapshot("full", ins, del, stats.bytes_processed, before_diag, t0))
    }

    pub fn rebuild_full(&mut self) {
        self.tokens = lex_full(&self.source);
        self.program = parse_full(&self.tokens);
        self.symbols = symbols_full(&self.program);
        self.diagnostics = diagnostics_full(&self.program, &self.symbols);
    }

    fn snapshot(
        &self,
        mode: &str,
        ins: isize,
        del: isize,
        bytes: usize,
        before_diag: usize,
        t0: Instant,
    ) -> PipelineSnapshot {
        PipelineSnapshot {
            inserted_chars: ins as i64,
            deleted_chars: del as i64,
            tokens_total: self.tokens.tokens.len(),
            tokens_reused: self.tokens.work.items_reused,
            tokens_recomputed: self.tokens.work.items_recomputed,
            ast_total: self.program.work.items_total,
            ast_reused: self.program.work.items_reused,
            ast_recomputed: self.program.work.items_recomputed,
            symbols_total: self.symbols.work.items_total,
            symbols_reused: self.symbols.work.items_reused,
            symbols_recomputed: self.symbols.work.items_recomputed,
            diagnostics_total: self.diagnostics.work.items_total,
            diagnostics_reused: self.diagnostics.work.items_reused,
            diagnostics_recomputed: self.diagnostics.work.items_recomputed,
            diagnostics_before: before_diag,
            diagnostics_after: self.diagnostics.items.len(),
            bytes_processed: bytes
                + self.tokens.work.bytes_processed
                + self.program.work.bytes_processed,
            elapsed_nanos: t0.elapsed().as_nanos(),
            source_len: self.source.len(),
            mode: mode.into(),
        }
    }
}

fn dirty_stmt_ids(program: &Program) -> Vec<u64> {
    program.dirty_stmt_ids.clone()
}

fn expand_symbol_fanout(symbols: &SymbolTable, dirty: &[u64]) -> Vec<u64> {
    let dirty_set: std::collections::BTreeSet<u64> = dirty.iter().copied().collect();
    let dirty_names: std::collections::BTreeSet<String> = symbols
        .defs
        .values()
        .filter(|s| dirty_set.contains(&s.def_stmt))
        .map(|s| s.name.clone())
        .collect();
    let mut out = dirty_set;
    for r in &symbols.refs {
        if dirty_names.contains(&r.name) {
            out.insert(r.from_stmt);
        }
    }
    out.into_iter().collect()
}

#[derive(Clone, Copy, Debug)]
pub struct LanguageHandles {
    pub source: delta_core::NodeId,
    pub tokens: delta_core::NodeId,
    pub ast: delta_core::NodeId,
    pub symbols: delta_core::NodeId,
    pub diagnostics: delta_core::NodeId,
}

pub fn install_language_graph(engine: &mut Engine, text: String) -> Result<LanguageHandles> {
    let source = engine.register_input("SOURCE", "text", Box::new(text))?;
    let tokens = engine.register(Box::new(LexComp), &[source])?;
    let ast = engine.register(Box::new(ParseComp), &[tokens])?;
    let symbols = engine.register(Box::new(SymbolComp), &[ast])?;
    let diagnostics = engine.register(Box::new(DiagComp), &[ast, symbols])?;
    engine.update()?;
    Ok(LanguageHandles {
        source,
        tokens,
        ast,
        symbols,
        diagnostics,
    })
}

struct LexComp;
struct ParseComp;
struct SymbolComp;
struct DiagComp;

impl Computation for LexComp {
    fn name(&self) -> &str {
        "TOKENS"
    }
    fn kind(&self) -> &str {
        "lexer"
    }
    fn compute(&self, ctx: ComputeCtx<'_>) -> Result<ComputeOutput> {
        let text: &String = ctx.input(0)?;
        let ts = lex_full(text);
        let fp = Some(delta_core::Fingerprint::of_bytes(
            ts.tokens
                .iter()
                .flat_map(|t| t.lexeme.as_bytes())
                .copied()
                .collect::<Vec<_>>()
                .as_slice(),
        ));
        Ok(ComputeOutput {
            work: ts.work.clone(),
            value: Box::new(ts),
            fingerprint: fp,
        })
    }
    fn compute_incremental(&self, ctx: ComputeCtx<'_>) -> Result<Option<ComputeOutput>> {
        let text: &String = ctx.input(0)?;
        let Some(prev) = ctx.previous_as::<TokenStream>() else {
            return Ok(None);
        };
        let Some(delta) = ctx.delta else {
            return Ok(None);
        };
        let ts = lex_incremental(text, prev, delta)?;
        Ok(Some(ComputeOutput {
            work: ts.work.clone(),
            value: Box::new(ts),
            fingerprint: None,
        }))
    }
}

impl Computation for ParseComp {
    fn name(&self) -> &str {
        "AST"
    }
    fn kind(&self) -> &str {
        "parser"
    }
    fn compute(&self, ctx: ComputeCtx<'_>) -> Result<ComputeOutput> {
        let tokens: &TokenStream = ctx.input(0)?;
        let p = parse_full(tokens);
        Ok(ComputeOutput {
            work: p.work.clone(),
            value: Box::new(p),
            fingerprint: None,
        })
    }
    fn compute_incremental(&self, ctx: ComputeCtx<'_>) -> Result<Option<ComputeOutput>> {
        let tokens: &TokenStream = ctx.input(0)?;
        let Some(prev) = ctx.previous_as::<Program>() else {
            return Ok(None);
        };
        let p = parse_incremental(tokens, prev)?;
        Ok(Some(ComputeOutput {
            work: p.work.clone(),
            value: Box::new(p),
            fingerprint: None,
        }))
    }
}

impl Computation for SymbolComp {
    fn name(&self) -> &str {
        "SYMBOLS"
    }
    fn kind(&self) -> &str {
        "symbols"
    }
    fn compute(&self, ctx: ComputeCtx<'_>) -> Result<ComputeOutput> {
        let program: &Program = ctx.input(0)?;
        let s = symbols_full(program);
        Ok(ComputeOutput {
            work: s.work.clone(),
            value: Box::new(s),
            fingerprint: None,
        })
    }
    fn compute_incremental(&self, ctx: ComputeCtx<'_>) -> Result<Option<ComputeOutput>> {
        let program: &Program = ctx.input(0)?;
        let Some(prev) = ctx.previous_as::<SymbolTable>() else {
            return Ok(None);
        };
        let dirty = dirty_stmt_ids(program);
        let s = symbols_incremental(program, prev, &dirty);
        Ok(Some(ComputeOutput {
            work: s.work.clone(),
            value: Box::new(s),
            fingerprint: None,
        }))
    }
}

impl Computation for DiagComp {
    fn name(&self) -> &str {
        "DIAGNOSTICS"
    }
    fn kind(&self) -> &str {
        "diagnostics"
    }
    fn compute(&self, ctx: ComputeCtx<'_>) -> Result<ComputeOutput> {
        let program: &Program = ctx.input(0)?;
        let symbols: &SymbolTable = ctx.input(1)?;
        let d = diagnostics_full(program, symbols);
        Ok(ComputeOutput {
            work: d.work.clone(),
            value: Box::new(d),
            fingerprint: None,
        })
    }
    fn compute_incremental(&self, ctx: ComputeCtx<'_>) -> Result<Option<ComputeOutput>> {
        let program: &Program = ctx.input(0)?;
        let symbols: &SymbolTable = ctx.input(1)?;
        let Some(prev) = ctx.previous_as::<DiagnosticSet>() else {
            return Ok(None);
        };
        let dirty = expand_symbol_fanout(symbols, &dirty_stmt_ids(program));
        let d = diagnostics_incremental(program, symbols, prev, &dirty);
        Ok(Some(ComputeOutput {
            work: d.work.clone(),
            value: Box::new(d),
            fingerprint: None,
        }))
    }
}

pub fn report_from_snapshot(
    snap: &PipelineSnapshot,
    engine: Option<&UpdateReport>,
) -> UpdateReport {
    let mut report = engine.cloned().unwrap_or_default();
    report.change = Some(delta_runtime::ChangeSummary {
        inserted_chars: snap.inserted_chars,
        deleted_chars: snap.deleted_chars,
        inserted_bytes: 0,
        deleted_bytes: 0,
    });
    report.work.merge(&WorkMetrics {
        bytes_processed: snap.bytes_processed,
        items_reused: snap.tokens_reused
            + snap.ast_reused
            + snap.symbols_reused
            + snap.diagnostics_reused,
        items_recomputed: snap.tokens_recomputed
            + snap.ast_recomputed
            + snap.symbols_recomputed
            + snap.diagnostics_recomputed,
        items_total: snap.tokens_total
            + snap.ast_total
            + snap.symbols_total
            + snap.diagnostics_total,
        hashing_nanos: DurationNanos(0),
        ..WorkMetrics::default()
    });
    report.execution_nanos = DurationNanos(snap.elapsed_nanos);
    report.finalize_percents();
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use delta_core::Delta;

    #[test]
    fn incremental_matches_full_oracle() {
        let src = "let foo = 10;\nlet bar = 20;\nlet baz = foo + bar;\n";
        let mut inc = LanguagePipeline::from_text(src);
        let mut full = LanguagePipeline::from_text(src);
        let delta = Delta::insert(src.find("foo =").unwrap() + 1, "x");
        inc.apply_incremental(&delta).unwrap();
        full.apply_full(&delta).unwrap();
        assert_eq!(inc.source, full.source);
        assert!(inc.tokens.semantic_eq(&full.tokens));
        assert!(inc.program.semantic_eq(&full.program));
        assert!(inc.symbols.semantic_eq(&full.symbols));
        assert!(inc.diagnostics.semantic_eq(&full.diagnostics));
    }

    #[test]
    fn engine_graph_runs() {
        let mut engine = Engine::new();
        let h = install_language_graph(&mut engine, "let foo = 1;".into()).unwrap();
        let ts = engine.get::<TokenStream>(h.tokens).unwrap();
        assert!(!ts.tokens.is_empty());
        engine
            .apply_delta(h.source, Delta::insert(11, "0"))
            .unwrap();
        let src = engine.get::<String>(h.source).unwrap();
        assert_eq!(src, "let foo = 10;");
    }
}
