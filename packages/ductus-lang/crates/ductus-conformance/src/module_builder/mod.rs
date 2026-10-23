//! The programmatic half of a witness: `ductus-ir` values built in Rust,
//! with no text anywhere.
//!
//! Every witness makes the same claim twice — once from its `.duir` file and
//! once from a module built here — and the two traces must be byte-identical
//! (M0.md §6, M1.md §6.6). These are the pieces that claim shares: a path, a
//! clause-free cell, a scope, a module. What a witness does NOT share is its
//! KIND builders — an `input` at `i64`, a `recurrent` with per-depth seeds, a
//! `deferred` at `depth 2` — because those carry the clauses that witness is
//! about, and they live beside the golden they explain.

pub mod cell;
pub mod module;
pub mod path;
pub mod scope;

pub use self::cell::cell;
pub use self::module::module;
pub use self::path::path;
pub use self::scope::scope;
