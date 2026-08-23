//! Commit engine: the six steps of SPEC §13.10.2, mirrored literally — dirty set →
//! evaluation order (static-order walk) → evaluate → advance_recurrents (explicitly
//! empty until M1) → publish → clear. Emits trace events through the trace_sink seam.
//!
//! [`Engine`] owns the whole host-facing surface (§13.14.2/.4/.6/.10) and
//! [`Transaction`] is §13.10.4's grouping sugar over the same staged writes;
//! the validation both write paths share lives in the folder-private
//! `write_check`.

// The file is named after the entity it holds (S1), and the entity is
// `Engine`, so `engine/engine.rs` stutters — the same trade `value/value.rs`
// makes. `expect` rather than `allow` so the annotation reports itself as
// stale if the layout ever changes.
#[expect(
    clippy::module_inception,
    reason = "S1 names files after their entity; `Engine` lives in `engine/`"
)]
pub mod engine;

pub mod transaction;

mod write_check;

#[cfg(test)]
mod test_support;

pub use self::engine::Engine;
pub use self::transaction::Transaction;
