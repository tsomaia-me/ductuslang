//! The `.duir` lexer: a hand-written direct scanner (R13) split in two (R14) —
//! `scan`'s pure recognizers, which are functions of their input alone, and the
//! single stateful driver in `lex`, which owns the cursor, the spans and the
//! fail-fast error path.
//!
//! [`lex()`] is the whole public surface. `scan` and `scan_error` are the
//! split's internal protocol — crate-visible so the two halves can talk, never
//! part of the lexer's contract with the parser.

pub mod lex;

pub(crate) mod scan;
pub(crate) mod scan_error;

pub use self::lex::lex;
