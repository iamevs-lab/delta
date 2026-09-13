use crate::parser::{Expr, Program, Stmt};
use delta_core::WorkMetrics;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SymbolId(pub u64);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Symbol {
    pub id: SymbolId,
    pub name: String,
    pub def_stmt: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reference {
    pub name: String,
    pub from_stmt: u64,
    pub def: Option<SymbolId>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolTable {
    pub defs: BTreeMap<String, Symbol>,
    pub refs: Vec<Reference>,
    pub work: WorkMetrics,
}

impl SymbolTable {
    pub fn semantic_eq(&self, other: &SymbolTable) -> bool {
        if self.defs.keys().collect::<Vec<_>>() != other.defs.keys().collect::<Vec<_>>() {
            return false;
        }
        let mut a: Vec<_> = self
            .refs
            .iter()
            .map(|r| (r.name.clone(), r.def.is_some()))
            .collect();
        let mut b: Vec<_> = other
            .refs
            .iter()
            .map(|r| (r.name.clone(), r.def.is_some()))
            .collect();
        a.sort();
        b.sort();
        a == b
    }
}

pub fn symbols_full(program: &Program) -> SymbolTable {
    let mut table = SymbolTable::default();
    let mut next = 1u64;
    collect(program, &mut table, &mut next, true);
    let n = table.defs.len() + table.refs.len();
    table.work = WorkMetrics {
        items_recomputed: n,
        items_total: n,
        items_reused: 0,
        ..WorkMetrics::default()
    };
    table
}

pub fn symbols_incremental(
    program: &Program,
    previous: &SymbolTable,
    dirty_stmt_ids: &[u64],
) -> SymbolTable {
    let full = symbols_full(program);
    if dirty_stmt_ids.is_empty() && previous.semantic_eq(&full) {
        let mut t = previous.clone();
        t.work = WorkMetrics {
            items_reused: previous.defs.len() + previous.refs.len(),
            items_total: previous.defs.len() + previous.refs.len(),
            ..WorkMetrics::default()
        };
        return t;
    }

    let attempted = symbols_incremental_inner(program, previous, dirty_stmt_ids);
    if attempted.semantic_eq(&full) {
        return attempted;
    }
    // Incremental hypothesis lost. Keep the oracle.
    full
}

fn symbols_incremental_inner(
    program: &Program,
    previous: &SymbolTable,
    dirty_stmt_ids: &[u64],
) -> SymbolTable {
    let dirty: std::collections::BTreeSet<u64> = dirty_stmt_ids.iter().copied().collect();
    let live: std::collections::BTreeSet<u64> = program.stmts.iter().map(|s| s.id().0).collect();
    let mut table = SymbolTable {
        defs: previous
            .defs
            .iter()
            .filter(|(_, s)| live.contains(&s.def_stmt) && !dirty.contains(&s.def_stmt))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
        refs: previous
            .refs
            .iter()
            .filter(|r| live.contains(&r.from_stmt) && !dirty.contains(&r.from_stmt))
            .cloned()
            .collect(),
        work: WorkMetrics::default(),
    };

    let reused = table.defs.len() + table.refs.len();
    let mut next = previous.defs.values().map(|s| s.id.0).max().unwrap_or(0) + 1;

    collect_dirty(program, &mut table, &mut next, &dirty);
    rebind_refs(&mut table);

    let total = table.defs.len() + table.refs.len();
    table.work = WorkMetrics {
        items_reused: reused,
        items_recomputed: total.saturating_sub(reused),
        items_total: total,
        ..WorkMetrics::default()
    };
    table
}

fn collect(program: &Program, table: &mut SymbolTable, next: &mut u64, bind: bool) {
    for stmt in &program.stmts {
        collect_stmt(stmt, table, next);
    }
    if bind {
        rebind_refs(table);
    }
}

fn collect_dirty(
    program: &Program,
    table: &mut SymbolTable,
    next: &mut u64,
    dirty: &std::collections::BTreeSet<u64>,
) {
    for stmt in &program.stmts {
        walk_dirty(stmt, table, next, dirty);
    }
}

fn walk_dirty(
    stmt: &Stmt,
    table: &mut SymbolTable,
    next: &mut u64,
    dirty: &std::collections::BTreeSet<u64>,
) {
    if dirty.contains(&stmt.id().0) {
        collect_stmt(stmt, table, next);
    }
    if let Stmt::Fn { body, .. } = stmt {
        for s in body {
            walk_dirty(s, table, next, dirty);
        }
    }
}

fn collect_stmt(stmt: &Stmt, table: &mut SymbolTable, next: &mut u64) {
    match stmt {
        Stmt::Let { id, name, expr, .. } => {
            table.defs.insert(
                name.clone(),
                Symbol {
                    id: SymbolId(*next),
                    name: name.clone(),
                    def_stmt: id.0,
                },
            );
            *next += 1;
            collect_expr(expr, id.0, table);
        }
        Stmt::Fn { id, name, body, .. } => {
            table.defs.insert(
                name.clone(),
                Symbol {
                    id: SymbolId(*next),
                    name: name.clone(),
                    def_stmt: id.0,
                },
            );
            *next += 1;
            for s in body {
                collect_stmt(s, table, next);
            }
        }
        Stmt::Call { id, name, .. } => {
            table.refs.push(Reference {
                name: name.clone(),
                from_stmt: id.0,
                def: None,
            });
        }
    }
}

fn collect_expr(expr: &Expr, from_stmt: u64, table: &mut SymbolTable) {
    match expr {
        Expr::Ident { name, .. } => {
            table.refs.push(Reference {
                name: name.clone(),
                from_stmt,
                def: None,
            });
        }
        Expr::Number { .. } => {}
        Expr::Binary { left, right, .. } => {
            collect_expr(left, from_stmt, table);
            collect_expr(right, from_stmt, table);
        }
    }
}

fn rebind_refs(table: &mut SymbolTable) {
    for r in &mut table.refs {
        r.def = table.defs.get(&r.name).map(|s| s.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_full;
    use crate::parser::parse_full;

    #[test]
    fn tracks_def_and_refs() {
        let ts = lex_full("fn foo() {}\nfoo();\nfoo();\nfoo();\n");
        let ast = parse_full(&ts);
        let sym = symbols_full(&ast);
        assert!(sym.defs.contains_key("foo"));
        assert_eq!(sym.refs.len(), 3);
        assert!(sym.refs.iter().all(|r| r.def.is_some()));
    }
}
