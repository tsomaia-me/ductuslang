//! Golden-trace comparison: byte equality, with a readable report when it
//! fails.
//!
//! The comparison is deliberately dumb — [`assert_trace()`] normalizes nothing —
//! because the trace format is a canonical form (R24 as amended by R34) and a
//! forgiving comparison would quietly admit a second spelling. All the
//! cleverness is in the FAILURE path: which line diverged, what both sides
//! said there, and both traces whole.

pub mod assert_trace;

pub use self::assert_trace::assert_trace;
