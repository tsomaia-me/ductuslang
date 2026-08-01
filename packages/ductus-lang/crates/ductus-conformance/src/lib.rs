//! Conformance harness — the long-lived test product: witness runner support,
//! shared stub library, trace comparison. Witnesses are Rust `#[test]`s that
//! load `corpus/**/*.duir`, register stubs, drive the runtime, and assert
//! against `.trace` goldens.

pub mod runner;
pub mod stub_library;
pub mod trace_compare;
