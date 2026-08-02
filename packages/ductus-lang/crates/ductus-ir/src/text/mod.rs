//! The `.duir` text form: token vocabulary, lexer, parser, emitter.
//!
//! The token inventory and the complete lexer land in increment 3; the parser
//! and the emitter land in increment 4 with the module header, and each
//! construct row of the ladder extends the two of them together — a construct
//! is finished when it parses, emits, and round-trips.

pub mod emitter;
pub mod lexer;
pub mod parser;
pub mod token;
