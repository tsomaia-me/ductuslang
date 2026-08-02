//! The `.duir` emitter: the module AST back to text, in canonical form.
//!
//! [`emit()`] is the whole public surface. The emitter IS the canonicalizer
//! (R54/E1): a module has exactly ONE rendering, which makes `emit` the
//! definition of module equivalence (R17) and makes the round-trip gate
//! `emit(parse(text)) == text` a check that `text` itself was written
//! canonically.
//!
//! Emitting is total — it takes a `Module` and returns a `String`, with no
//! failure path. A module that cannot be rendered would be a module the parser
//! should never have produced.
//!
//! The grammar is IR_GRAMMAR §3 (§15.4.6).

pub mod emit;

pub use self::emit::emit;
