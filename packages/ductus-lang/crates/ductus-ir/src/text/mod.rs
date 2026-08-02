//! The `.duir` text form: token vocabulary, lexer, parser, emitter.
//! The token inventory and the complete lexer land in increment 3; the parser
//! and the emitter land at their own rows of the ladder.

pub mod emitter;
pub mod lexer;
pub mod parser;
pub mod token;
