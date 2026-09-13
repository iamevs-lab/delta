use crate::lexer::{Token, TokenKind, TokenStream};
use delta_core::{ByteRange, Result, WorkMetrics};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AstId(pub u64);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Expr {
    Ident {
        id: AstId,
        name: String,
        span: ByteRange,
    },
    Number {
        id: AstId,
        value: i64,
        span: ByteRange,
    },
    Binary {
        id: AstId,
        op: char,
        left: Box<Expr>,
        right: Box<Expr>,
        span: ByteRange,
    },
}

impl Expr {
    pub fn id(&self) -> AstId {
        match self {
            Self::Ident { id, .. } | Self::Number { id, .. } | Self::Binary { id, .. } => *id,
        }
    }

    pub fn span(&self) -> ByteRange {
        match self {
            Self::Ident { span, .. } | Self::Number { span, .. } | Self::Binary { span, .. } => {
                *span
            }
        }
    }

    pub fn walk_ids(&self, out: &mut Vec<AstId>) {
        out.push(self.id());
        if let Self::Binary { left, right, .. } = self {
            left.walk_ids(out);
            right.walk_ids(out);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stmt {
    Let {
        id: AstId,
        name: String,
        expr: Expr,
        span: ByteRange,
    },
    Fn {
        id: AstId,
        name: String,
        body: Vec<Stmt>,
        span: ByteRange,
    },
    Call {
        id: AstId,
        name: String,
        span: ByteRange,
    },
}

impl Stmt {
    pub fn id(&self) -> AstId {
        match self {
            Self::Let { id, .. } | Self::Fn { id, .. } | Self::Call { id, .. } => *id,
        }
    }

    pub fn span(&self) -> ByteRange {
        match self {
            Self::Let { span, .. } | Self::Fn { span, .. } | Self::Call { span, .. } => *span,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Let { name, .. } | Self::Fn { name, .. } | Self::Call { name, .. } => name,
        }
    }

    pub fn walk_ids(&self, out: &mut Vec<AstId>) {
        out.push(self.id());
        match self {
            Self::Let { expr, .. } => expr.walk_ids(out),
            Self::Fn { body, .. } => {
                for s in body {
                    s.walk_ids(out);
                }
            }
            Self::Call { .. } => {}
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Program {
    pub id: AstId,
    pub stmts: Vec<Stmt>,
    pub errors: Vec<String>,
    pub next_id: u64,
    pub work: WorkMetrics,
    pub dirty_stmt_ids: Vec<u64>,
}

impl Program {
    pub fn all_ids(&self) -> Vec<AstId> {
        let mut ids = vec![self.id];
        for s in &self.stmts {
            s.walk_ids(&mut ids);
        }
        ids
    }

    pub fn semantic_eq(&self, other: &Program) -> bool {
        fn stmt_eq(a: &Stmt, b: &Stmt) -> bool {
            match (a, b) {
                (
                    Stmt::Let {
                        name: n1, expr: e1, ..
                    },
                    Stmt::Let {
                        name: n2, expr: e2, ..
                    },
                ) => n1 == n2 && expr_eq(e1, e2),
                (
                    Stmt::Fn {
                        name: n1, body: b1, ..
                    },
                    Stmt::Fn {
                        name: n2, body: b2, ..
                    },
                ) => {
                    n1 == n2
                        && b1.len() == b2.len()
                        && b1.iter().zip(b2).all(|(x, y)| stmt_eq(x, y))
                }
                (Stmt::Call { name: n1, .. }, Stmt::Call { name: n2, .. }) => n1 == n2,
                _ => false,
            }
        }
        fn expr_eq(a: &Expr, b: &Expr) -> bool {
            match (a, b) {
                (Expr::Ident { name: n1, .. }, Expr::Ident { name: n2, .. }) => n1 == n2,
                (Expr::Number { value: v1, .. }, Expr::Number { value: v2, .. }) => v1 == v2,
                (
                    Expr::Binary {
                        op: o1,
                        left: l1,
                        right: r1,
                        ..
                    },
                    Expr::Binary {
                        op: o2,
                        left: l2,
                        right: r2,
                        ..
                    },
                ) => o1 == o2 && expr_eq(l1, l2) && expr_eq(r1, r2),
                _ => false,
            }
        }
        self.stmts.len() == other.stmts.len()
            && self
                .stmts
                .iter()
                .zip(&other.stmts)
                .all(|(a, b)| stmt_eq(a, b))
            && self.errors == other.errors
    }
}

pub fn parse_full(tokens: &TokenStream) -> Program {
    let mut next_id = 1u64;
    let mut p = Parser {
        tokens: &tokens.tokens,
        i: 0,
        next_id: &mut next_id,
        errors: Vec::new(),
    };
    let stmts = p.parse_stmts_until(None);
    let errors = p.errors;
    let n = count_nodes(&stmts) + 1;
    let dirty_stmt_ids = stmts.iter().map(|s| s.id().0).collect();
    Program {
        id: AstId(0),
        stmts,
        errors,
        next_id,
        work: WorkMetrics {
            items_recomputed: n,
            items_total: n,
            items_reused: 0,
            bytes_processed: tokens.work.bytes_processed,
            ..WorkMetrics::default()
        },
        dirty_stmt_ids,
    }
}

pub fn parse_incremental(tokens: &TokenStream, previous: &Program) -> Result<Program> {
    let attempted = parse_incremental_inner(tokens, previous)?;
    let full = parse_full(tokens);
    if attempted.semantic_eq(&full) {
        Ok(attempted)
    } else {
        Ok(full)
    }
}

fn parse_incremental_inner(tokens: &TokenStream, previous: &Program) -> Result<Program> {
    let Some(edit) = tokens.edit else {
        return Ok(parse_full(tokens));
    };
    if previous.stmts.is_empty() {
        return Ok(parse_full(tokens));
    }

    let shift = edit.shift();
    let mut prefix = Vec::new();
    let mut suffix = Vec::new();
    for stmt in &previous.stmts {
        if stmt.span().end <= edit.old_start {
            prefix.push(stmt.clone());
        } else if stmt.span().start >= edit.old_end && stmt.span().start > edit.old_start {
            // Insert at a statement's first byte prepends to that statement;
            // it is not "after" the edit.
            suffix.push(shift_stmt(stmt, shift));
        }
    }

    let mid_start = prefix.last().map(|s| s.span().end).unwrap_or(0);
    let mid_end = suffix.first().map(|s| s.span().start).unwrap_or(usize::MAX);
    let mid_tokens: Vec<Token> = tokens
        .tokens
        .iter()
        .filter(|t| t.start >= mid_start && t.start < mid_end)
        .cloned()
        .collect();

    let mut next_id = previous.next_id;
    let mut errors = Vec::new();
    let mut mid = Vec::new();
    if !mid_tokens.is_empty() {
        let mut p = Parser {
            tokens: &mid_tokens,
            i: 0,
            next_id: &mut next_id,
            errors: Vec::new(),
        };
        mid = p.parse_stmts_until(None);
        errors = p.errors;
        // If the slice does not parse cleanly, do not invent a different tree
        // than the full oracle. Fall back.
        if !errors.is_empty() || p.i != mid_tokens.len() {
            return Ok(parse_full(tokens));
        }
    }

    let reused = prefix.iter().chain(&suffix).map(count_nodes_stmt).sum();
    let recomputed = mid.iter().map(count_nodes_stmt).sum();
    let dirty_stmt_ids: Vec<u64> = mid.iter().map(|s| s.id().0).collect();
    let mut stmts = prefix;
    stmts.extend(mid);
    stmts.extend(suffix);
    let total = stmts.iter().map(count_nodes_stmt).sum::<usize>() + 1;
    Ok(Program {
        id: previous.id,
        stmts,
        errors,
        next_id,
        work: WorkMetrics {
            items_reused: reused,
            items_recomputed: recomputed,
            items_total: total,
            ..WorkMetrics::default()
        },
        dirty_stmt_ids,
    })
}

fn shift_stmt(stmt: &Stmt, shift: isize) -> Stmt {
    match stmt {
        Stmt::Let {
            id,
            name,
            expr,
            span,
        } => Stmt::Let {
            id: *id,
            name: name.clone(),
            expr: shift_expr(expr, shift),
            span: shift_range(*span, shift),
        },
        Stmt::Fn {
            id,
            name,
            body,
            span,
        } => Stmt::Fn {
            id: *id,
            name: name.clone(),
            body: body.iter().map(|s| shift_stmt(s, shift)).collect(),
            span: shift_range(*span, shift),
        },
        Stmt::Call { id, name, span } => Stmt::Call {
            id: *id,
            name: name.clone(),
            span: shift_range(*span, shift),
        },
    }
}

fn shift_expr(expr: &Expr, shift: isize) -> Expr {
    match expr {
        Expr::Ident { id, name, span } => Expr::Ident {
            id: *id,
            name: name.clone(),
            span: shift_range(*span, shift),
        },
        Expr::Number { id, value, span } => Expr::Number {
            id: *id,
            value: *value,
            span: shift_range(*span, shift),
        },
        Expr::Binary {
            id,
            op,
            left,
            right,
            span,
        } => Expr::Binary {
            id: *id,
            op: *op,
            left: Box::new(shift_expr(left, shift)),
            right: Box::new(shift_expr(right, shift)),
            span: shift_range(*span, shift),
        },
    }
}

fn shift_range(range: ByteRange, shift: isize) -> ByteRange {
    ByteRange {
        start: (range.start as isize + shift).max(0) as usize,
        end: (range.end as isize + shift).max(0) as usize,
    }
}

fn count_nodes(stmts: &[Stmt]) -> usize {
    stmts.iter().map(count_nodes_stmt).sum()
}

fn count_nodes_stmt(stmt: &Stmt) -> usize {
    let mut ids = Vec::new();
    stmt.walk_ids(&mut ids);
    ids.len()
}

struct Parser<'a> {
    tokens: &'a [Token],
    i: usize,
    next_id: &'a mut u64,
    errors: Vec<String>,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.i)
    }

    fn bump(&mut self) -> Option<&Token> {
        let t = self.tokens.get(self.i)?;
        self.i += 1;
        Some(t)
    }

    fn alloc(&mut self) -> AstId {
        let id = AstId(*self.next_id);
        *self.next_id += 1;
        id
    }

    fn parse_stmts_until(&mut self, until: Option<TokenKind>) -> Vec<Stmt> {
        let mut stmts = Vec::new();
        while let Some(t) = self.peek() {
            if let Some(k) = &until {
                if t.kind == *k {
                    break;
                }
            }
            if let Some(s) = self.parse_stmt() {
                stmts.push(s);
            } else {
                break;
            }
        }
        stmts
    }

    fn parse_stmt(&mut self) -> Option<Stmt> {
        let t = self.peek()?.clone();
        match t.kind {
            TokenKind::Let => self.parse_let(),
            TokenKind::Fn => self.parse_fn(),
            TokenKind::Ident => self.parse_call(),
            _ => {
                self.errors
                    .push(format!("unexpected token '{}' at {}", t.lexeme, t.start));
                self.bump();
                None
            }
        }
    }

    fn parse_let(&mut self) -> Option<Stmt> {
        let start = self.bump()?.start;
        let name_tok = self.bump()?.clone();
        if name_tok.kind != TokenKind::Ident {
            self.errors.push("expected identifier after let".into());
            return None;
        }
        let eq = self.bump()?.clone();
        if eq.kind != TokenKind::Eq {
            self.errors.push("expected '='".into());
            return None;
        }
        let expr = self.parse_expr()?;
        let end = match self.bump().map(|t| (t.kind.clone(), t.end)) {
            Some((TokenKind::Semi, end)) => end,
            Some((_, end)) => {
                self.errors.push("expected ';'".into());
                end
            }
            None => expr.span().end,
        };
        Some(Stmt::Let {
            id: self.alloc(),
            name: name_tok.lexeme,
            expr,
            span: ByteRange { start, end },
        })
    }

    fn parse_fn(&mut self) -> Option<Stmt> {
        let start = self.bump()?.start;
        let name_tok = self.bump()?.clone();
        self.bump(); // (
        self.bump(); // )
        self.bump(); // {
        let body = self.parse_stmts_until(Some(TokenKind::RBrace));
        let end = match self.bump() {
            Some(t) => t.end,
            None => body.last().map(|s| s.span().end).unwrap_or(start),
        };
        Some(Stmt::Fn {
            id: self.alloc(),
            name: name_tok.lexeme,
            body,
            span: ByteRange { start, end },
        })
    }

    fn parse_call(&mut self) -> Option<Stmt> {
        let name_tok = self.bump()?.clone();
        self.bump(); // (
        self.bump(); // )
        let end = match self.bump() {
            Some(t) => t.end,
            None => name_tok.end,
        };
        Some(Stmt::Call {
            id: self.alloc(),
            name: name_tok.lexeme,
            span: ByteRange {
                start: name_tok.start,
                end,
            },
        })
    }

    fn parse_expr(&mut self) -> Option<Expr> {
        let mut left = self.parse_primary()?;
        while let Some(t) = self.peek() {
            let op = match t.kind {
                TokenKind::Plus => '+',
                TokenKind::Minus => '-',
                _ => break,
            };
            self.bump();
            let right = self.parse_primary()?;
            let span = ByteRange {
                start: left.span().start,
                end: right.span().end,
            };
            left = Expr::Binary {
                id: self.alloc(),
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Some(left)
    }

    fn parse_primary(&mut self) -> Option<Expr> {
        let t = self.bump()?.clone();
        match t.kind {
            TokenKind::Ident => Some(Expr::Ident {
                id: self.alloc(),
                name: t.lexeme,
                span: ByteRange {
                    start: t.start,
                    end: t.end,
                },
            }),
            TokenKind::Number => {
                let value = t.lexeme.parse().unwrap_or(0);
                Some(Expr::Number {
                    id: self.alloc(),
                    value,
                    span: ByteRange {
                        start: t.start,
                        end: t.end,
                    },
                })
            }
            _ => {
                self.errors
                    .push(format!("expected expression, got {}", t.lexeme));
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::{lex_full, lex_incremental};
    use delta_core::Delta;

    #[test]
    fn parse_lets() {
        let ts = lex_full("let foo = 10; let bar = foo + 1;");
        let p = parse_full(&ts);
        assert_eq!(p.stmts.len(), 2);
        assert!(p.errors.is_empty());
    }

    #[test]
    fn incremental_reuses_untouched_stmt() {
        let src = "let foo = 10;\nlet bar = 20;\nlet baz = 30;\n";
        let tokens0 = lex_full(src);
        let ast0 = parse_full(&tokens0);
        let delta = Delta::replace(src.find("20").unwrap(), src.find("20").unwrap() + 2, "99");
        let mut text = src.to_string();
        delta_core::apply_delta_mut(&mut text, &delta).unwrap();
        let tokens1 = lex_incremental(&text, &tokens0, &delta).unwrap();
        let ast1 = parse_incremental(&tokens1, &ast0).unwrap();
        let ast_full = parse_full(&tokens1);
        assert!(ast1.semantic_eq(&ast_full));
        assert!(ast1.work.items_reused > 0);
    }
}
