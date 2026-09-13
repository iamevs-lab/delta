use crate::parser::Program;
use crate::symbols::SymbolTable;
use delta_core::WorkMetrics;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub key: String,
    pub message: String,
    pub stmt: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticSet {
    pub items: Vec<Diagnostic>,
    pub work: WorkMetrics,
}

impl DiagnosticSet {
    pub fn semantic_eq(&self, other: &DiagnosticSet) -> bool {
        let mut a: Vec<_> = self.items.iter().map(|d| d.key.clone()).collect();
        let mut b: Vec<_> = other.items.iter().map(|d| d.key.clone()).collect();
        a.sort();
        b.sort();
        a == b
    }
}

pub fn diagnostics_full(program: &Program, symbols: &SymbolTable) -> DiagnosticSet {
    let mut items = Vec::new();
    for err in &program.errors {
        items.push(Diagnostic {
            key: "syntax".into(),
            message: err.clone(),
            stmt: 0,
        });
    }
    for r in &symbols.refs {
        if r.def.is_none() {
            items.push(Diagnostic {
                key: format!("undef:{}", r.name),
                message: format!("undefined `{}`", r.name),
                stmt: r.from_stmt,
            });
        }
    }
    for (name, sym) in &symbols.defs {
        let used = symbols.refs.iter().any(|r| r.def == Some(sym.id));
        if !used {
            items.push(Diagnostic {
                key: format!("unused:{name}"),
                message: format!("unused `{name}`"),
                stmt: sym.def_stmt,
            });
        }
    }
    let n = items.len();
    DiagnosticSet {
        items,
        work: WorkMetrics {
            items_recomputed: n,
            items_total: n,
            ..WorkMetrics::default()
        },
    }
}

pub fn diagnostics_incremental(
    program: &Program,
    symbols: &SymbolTable,
    previous: &DiagnosticSet,
    dirty_stmt_ids: &[u64],
) -> DiagnosticSet {
    let full = diagnostics_full(program, symbols);
    if dirty_stmt_ids.is_empty() && previous.semantic_eq(&full) {
        let mut d = previous.clone();
        d.work = WorkMetrics {
            items_reused: previous.items.len(),
            items_total: previous.items.len(),
            ..WorkMetrics::default()
        };
        return d;
    }

    let attempted = diagnostics_incremental_inner(program, symbols, previous, dirty_stmt_ids);
    if attempted.semantic_eq(&full) {
        return attempted;
    }
    full
}

fn diagnostics_incremental_inner(
    program: &Program,
    symbols: &SymbolTable,
    previous: &DiagnosticSet,
    dirty_stmt_ids: &[u64],
) -> DiagnosticSet {
    let dirty: std::collections::BTreeSet<u64> = dirty_stmt_ids.iter().copied().collect();
    let live: std::collections::BTreeSet<u64> = program.stmts.iter().map(|s| s.id().0).collect();
    let reused: Vec<Diagnostic> = previous
        .items
        .iter()
        .filter(|d| {
            d.key != "syntax" && !dirty.contains(&d.stmt) && (d.stmt == 0 || live.contains(&d.stmt))
        })
        .cloned()
        .collect();
    let full = diagnostics_full(program, symbols);
    let mut items: Vec<Diagnostic> = full
        .items
        .iter()
        .filter(|d| d.key == "syntax" || dirty.contains(&d.stmt) || d.stmt == 0)
        .cloned()
        .collect();
    items.extend(reused);
    items.sort_by(|a, b| a.stmt.cmp(&b.stmt).then(a.key.cmp(&b.key)));
    items.dedup_by(|a, b| a.key == b.key && a.stmt == b.stmt);
    let reused_n = items
        .iter()
        .filter(|d| !dirty.contains(&d.stmt) && d.key != "syntax")
        .count();
    let total = items.len();
    DiagnosticSet {
        items,
        work: WorkMetrics {
            items_reused: reused_n,
            items_recomputed: total.saturating_sub(reused_n),
            items_total: total,
            ..WorkMetrics::default()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_full;
    use crate::parser::parse_full;
    use crate::symbols::symbols_full;

    #[test]
    fn flags_undefined() {
        let ts = lex_full("let x = y;");
        let ast = parse_full(&ts);
        let sym = symbols_full(&ast);
        let d = diagnostics_full(&ast, &sym);
        assert!(d.items.iter().any(|i| i.key.starts_with("undef:")));
    }
}
