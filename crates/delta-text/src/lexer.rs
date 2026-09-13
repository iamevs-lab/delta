use delta_core::{Delta, Result, WorkMetrics};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TokenId(pub u64);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TokenKind {
    Let,
    Fn,
    Ident,
    Number,
    Eq,
    Plus,
    Minus,
    Semi,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Token {
    pub id: TokenId,
    pub kind: TokenKind,
    pub start: usize,
    pub end: usize,
    pub lexeme: String,
}

impl Token {
    pub fn semantic_eq(&self, other: &Token) -> bool {
        self.kind == other.kind && self.lexeme == other.lexeme && self.start == other.start
    }
}

/// Byte span of an applied text edit, in both old and new coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditSpan {
    pub old_start: usize,
    pub old_end: usize,
    pub new_start: usize,
    pub new_end: usize,
}

impl EditSpan {
    pub fn shift(self) -> isize {
        self.new_end as isize - self.old_end as isize
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TokenStream {
    pub tokens: Vec<Token>,
    pub next_id: u64,
    pub work: WorkMetrics,
    pub edit: Option<EditSpan>,
}

impl TokenStream {
    pub fn semantic_eq(&self, other: &TokenStream) -> bool {
        if self.tokens.len() != other.tokens.len() {
            return false;
        }
        self.tokens
            .iter()
            .zip(&other.tokens)
            .all(|(a, b)| a.kind == b.kind && a.lexeme == b.lexeme && a.start == b.start)
    }
}

pub fn lex_full(text: &str) -> TokenStream {
    let mut next_id = 1u64;
    let (tokens, scanned) = scan(text, 0, &mut next_id);
    let n = tokens.len();
    TokenStream {
        tokens,
        next_id,
        work: WorkMetrics {
            bytes_processed: scanned,
            items_recomputed: n,
            items_total: n,
            items_reused: 0,
            ..WorkMetrics::default()
        },
        edit: None,
    }
}

/// Incremental lexer. Falls back to full scan if there is no previous stream
/// or the delta is not a simple text op we can localize.
pub fn lex_incremental(text: &str, previous: &TokenStream, delta: &Delta) -> Result<TokenStream> {
    let attempted = lex_incremental_inner(text, previous, delta)?;
    let full = lex_full(text);
    if attempted.semantic_eq(&full) {
        Ok(attempted)
    } else {
        Ok(full)
    }
}

fn lex_incremental_inner(text: &str, previous: &TokenStream, delta: &Delta) -> Result<TokenStream> {
    let Some((edit_start, old_end, new_end)) = delta_span(delta, previous_end(previous, text))
    else {
        return Ok(lex_full(text));
    };
    let edit = EditSpan {
        old_start: edit_start,
        old_end,
        new_start: edit_start,
        new_end,
    };
    let shift = edit.shift();

    let rescan_from = previous
        .tokens
        .iter()
        .rposition(|t| t.start <= edit_start)
        .map(|i| previous.tokens[i].start)
        .unwrap_or(0);

    let prefix: Vec<Token> = previous
        .tokens
        .iter()
        .filter(|t| t.end <= rescan_from)
        .cloned()
        .collect();

    let mut next_id = previous.next_id;
    let (mut scanned, bytes) = scan(text, rescan_from, &mut next_id);

    let mut reused_suffix = Vec::new();
    let mut cut = scanned.len();
    for (i, tok) in scanned.iter().enumerate() {
        if tok.start >= new_end {
            if let Some(old) = previous.tokens.iter().find(|old| {
                old.start as isize + shift == tok.start as isize
                    && old.kind == tok.kind
                    && old.lexeme == tok.lexeme
            }) {
                cut = i;
                reused_suffix = previous
                    .tokens
                    .iter()
                    .skip_while(|t| t.id != old.id)
                    .map(|t| Token {
                        id: t.id,
                        kind: t.kind.clone(),
                        start: (t.start as isize + shift) as usize,
                        end: (t.end as isize + shift) as usize,
                        lexeme: t.lexeme.clone(),
                    })
                    .collect();
                next_id = previous.next_id;
                break;
            }
        }
    }
    scanned.truncate(cut);

    let recomputed = scanned.len();
    let mut tokens = prefix;
    tokens.extend(scanned);
    let reused = tokens.len().saturating_sub(recomputed) + reused_suffix.len();
    tokens.extend(reused_suffix);

    let max_id = tokens.iter().map(|t| t.id.0).max().unwrap_or(0);
    let next_id = next_id.max(max_id.saturating_add(1));

    let total = tokens.len();
    Ok(TokenStream {
        tokens,
        next_id,
        work: WorkMetrics {
            bytes_processed: bytes,
            items_recomputed: recomputed,
            items_reused: reused,
            items_total: total,
            ..WorkMetrics::default()
        },
        edit: Some(edit),
    })
}

fn previous_end(previous: &TokenStream, text: &str) -> usize {
    previous
        .tokens
        .last()
        .map(|t| t.end)
        .unwrap_or(text.len())
        .max(text.len())
}

/// Returns (edit_start, old_end, new_end) in byte offsets.
fn delta_span(delta: &Delta, _hint_len: usize) -> Option<(usize, usize, usize)> {
    match delta {
        Delta::Insert { offset, text } => Some((*offset, *offset, *offset + text.len())),
        Delta::Delete { start, end } => Some((*start, *end, *start)),
        Delta::Replace { start, end, text } => Some((*start, *end, *start + text.len())),
        Delta::Composite { ops } => {
            // Sequential: track a running coordinate transform.
            let mut start = usize::MAX;
            let mut old_end = 0usize;
            let mut new_end = 0usize;
            let mut shift: isize = 0;
            let mut any = false;
            for op in ops {
                let (s, o, n) = delta_span(op, 0)?;
                let s = (s as isize + shift).max(0) as usize;
                let o = (o as isize + shift).max(0) as usize;
                let n = (n as isize + shift).max(0) as usize;
                if !any {
                    start = s;
                    any = true;
                }
                start = start.min(s);
                old_end = old_end.max(o);
                new_end = new_end.max(n);
                shift += n as isize - o as isize;
            }
            if any {
                Some((start, old_end, new_end))
            } else {
                None
            }
        }
        Delta::Move { .. } | Delta::Custom { .. } => None,
    }
}

fn scan(text: &str, from: usize, next_id: &mut u64) -> (Vec<Token>, usize) {
    let bytes = text.as_bytes();
    let mut i = from;
    let mut tokens = Vec::new();
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        let (kind, end) = if is_ident_start(c) {
            i += 1;
            while i < bytes.len() && is_ident_continue(bytes[i]) {
                i += 1;
            }
            let lex = &text[start..i];
            let kind = match lex {
                "let" => TokenKind::Let,
                "fn" => TokenKind::Fn,
                _ => TokenKind::Ident,
            };
            (kind, i)
        } else if c.is_ascii_digit() {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            (TokenKind::Number, i)
        } else {
            i += 1;
            let kind = match c {
                b'=' => TokenKind::Eq,
                b'+' => TokenKind::Plus,
                b'-' => TokenKind::Minus,
                b';' => TokenKind::Semi,
                b'(' => TokenKind::LParen,
                b')' => TokenKind::RParen,
                b'{' => TokenKind::LBrace,
                b'}' => TokenKind::RBrace,
                _ => TokenKind::Unknown,
            };
            (kind, i)
        };
        let id = TokenId(*next_id);
        *next_id += 1;
        tokens.push(Token {
            id,
            kind,
            start,
            end,
            lexeme: text[start..end].to_string(),
        });
    }
    (tokens, text.len() - from)
}

fn is_ident_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_'
}

fn is_ident_continue(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexes_lets() {
        let ts = lex_full("let foo = 10;");
        let kinds: Vec<_> = ts.tokens.iter().map(|t| t.kind.clone()).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Let,
                TokenKind::Ident,
                TokenKind::Eq,
                TokenKind::Number,
                TokenKind::Semi
            ]
        );
    }

    #[test]
    fn incremental_rename_matches_full() {
        let src = "let foo = 10;\nlet bar = 20;\n";
        let prev = lex_full(src);
        let delta = Delta::insert(7, "bar"); // foo -> foobar
        let mut text = src.to_string();
        delta_core::apply_delta_mut(&mut text, &delta).unwrap();
        let inc = lex_incremental(&text, &prev, &delta).unwrap();
        let full = lex_full(&text);
        assert!(inc.semantic_eq(&full), "{inc:?} vs {full:?}");
        assert!(inc.work.items_reused > 0);
    }
}
