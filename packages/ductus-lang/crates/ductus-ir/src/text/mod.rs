//! The `.duir` text form: token vocabulary, lexer, parser, emitter.
//!
//! The token inventory and the complete lexer land in increment 3; the parser
//! and the emitter land in increment 4 with the module header, and each
//! construct row of the ladder extends the two of them together — a construct
//! is finished when it parses, emits, and round-trips.
//!
//! `keywords` is the crate-private spelling table both the parser and the
//! emitter read from (R58) — it sits at `text/` level for the same reason the
//! token vocabulary does: it is shared by the two halves of the text form and
//! owned by neither. `test_support` sits here for that same reason: the parser
//! and the emitter are tested against the same sources, so the sources belong
//! to the folder rather than to either file.

pub(crate) mod keywords;

#[cfg(test)]
mod test_support;

pub mod emitter;
pub mod lexer;
pub mod parser;
pub mod token;
