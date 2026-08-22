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
//! owned by neither. `is_name_shaped` sits here on the same grounds: the `NAME`
//! charset is §2's law, and the emitter's producer contract and the path
//! concern's segment invariant are both checks against it — one law, one
//! predicate, one file. `is_reserved_word` and `quote` sit here for the same
//! reason: the reservation (002-39, 033-259) is checked by the parser, the path
//! concern and the emitter alike, and the canonical spelling of a delimited
//! payload (033-249, 033-243) is written by the `STRING` emitter and the
//! `QuotedKey` emitter from one grammar. `test_support` sits here for that same
//! reason: the parser and the emitter are tested against the same sources, so
//! the sources belong to the folder rather than to either file.

// Each module is private and its one function is re-exported, so a predicate
// has exactly ONE path — `crate::text::is_name_shaped` — rather than a second
// one through a module of the same name.
mod is_name_shaped;
mod is_reserved_word;
mod quote;

pub(crate) mod keywords;

pub(crate) use self::is_name_shaped::is_name_shaped;
pub(crate) use self::is_reserved_word::is_reserved_word;
pub(crate) use self::quote::quote;

#[cfg(test)]
mod test_support;

pub mod emitter;
pub mod lexer;
pub mod parser;
pub mod token;
