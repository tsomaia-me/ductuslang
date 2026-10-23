//! Conformance harness — the long-lived test product: witness runner support,
//! programmatic module builders, shared stub library, trace comparison.
//!
//! A witness is a Rust `#[test]` (R9) that loads its `corpus/duir/<milestone>/
//! *.duir` module and `.trace` golden through [`Witness`](runner::Witness),
//! registers the stubs it needs in a
//! [`StubRegistry`](stub_library::StubRegistry), drives the runtime with a
//! [`RecordingSink`](runner::RecordingSink) listening, and byte-compares what
//! it heard against the golden through
//! [`assert_trace()`](trace_compare::assert_trace()).
//!
//! It then makes the same claim a second time over the same script, from a
//! module built through [`module_builder`] instead of parsed — the
//! dual-construction equivalence of M0.md §6 / M1.md §6.6.
//!
//! The witness `#[test]`s themselves live in `tests/`, grouped per construct
//! (R25); this library is only what they share.
//!
//! # Goldens are hand-derived
//!
//! A `.trace` golden is derived from the semantics contract — SPEC §13.10.2 as
//! pinned in M0.md §4 — never captured from a run. A golden produced by
//! running the engine would assert that the engine does what the engine does.
//! When a hand derivation and the engine disagree, the disagreement is the
//! finding.

pub mod module_builder;
pub mod runner;
pub mod stub_library;
pub mod trace_compare;
