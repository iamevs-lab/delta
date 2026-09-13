//! First domain crate: text and a toy language.
//!
//! The engine remains generic. This crate is a client.

pub mod buffer;
pub mod diagnostics;
pub mod lexer;
pub mod parser;
pub mod pipeline;
pub mod symbols;

pub use buffer::TextBuffer;
pub use diagnostics::{Diagnostic, DiagnosticSet};
pub use lexer::{lex_full, lex_incremental, EditSpan, Token, TokenKind, TokenStream};
pub use parser::{parse_full, parse_incremental, AstId, Expr, Program, Stmt};
pub use pipeline::{install_language_graph, LanguageHandles, LanguagePipeline, PipelineSnapshot};
pub use symbols::{Reference, SymbolTable};

#[cfg(test)]
mod proptests;
