//! Witness-driving support: the corpus pair loader and the recording sink —
//! the plumbing every witness test shares (R9).
//!
//! A witness test is Rust: it loads its `.duir`/`.trace` pair through
//! [`Witness`], re-emits the source through [`parsed()`] to make its claim 1,
//! drives the module through [`traced()`] with a [`RecordingSink`] listening,
//! and byte-compares through
//! [`assert_trace()`](crate::trace_compare::assert_trace()).

pub mod parsed;
pub mod recording_sink;
pub mod traced;
pub mod witness;

pub use self::parsed::parsed;
pub use self::recording_sink::RecordingSink;
pub use self::traced::traced;
pub use self::witness::Witness;
