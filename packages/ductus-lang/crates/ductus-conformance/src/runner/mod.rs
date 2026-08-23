//! Witness-driving support: the corpus pair loader and the recording sink —
//! the plumbing every witness test shares (R9).
//!
//! A witness test is Rust: it loads its `.duir`/`.trace` pair through
//! [`Witness`], hands a [`RecordingSink`] to the engine, drives writes and
//! commits, and byte-compares through
//! [`assert_trace()`](crate::trace_compare::assert_trace()).

pub mod recording_sink;
pub mod witness;

pub use self::recording_sink::RecordingSink;
pub use self::witness::Witness;
