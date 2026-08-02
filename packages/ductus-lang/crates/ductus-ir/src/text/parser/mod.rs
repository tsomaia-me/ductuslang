//! The `.duir` parser: recursive descent over the token stream, straight into
//! the module AST (R16 — no concrete syntax tree in between).
//!
//! [`parse()`] is the whole public surface. Inside it, ONE private method per
//! IR_GRAMMAR production, named after it 1:1 (R53): the grammar is the parser's
//! table of contents, and the correspondence is the review surface.
//!
//! Fail-fast (T6): the first token the grammar does not admit ends the parse
//! with one `unexpected_token` `Diagnostic`. That is the parser's ONLY
//! diagnostic class — it is a pure grammar machine, and every semantic
//! legality class belongs to the loader (F4).
//!
//! The grammar is IR_GRAMMAR §3 (§15.4.6).

pub mod parse;

pub use self::parse::parse;
