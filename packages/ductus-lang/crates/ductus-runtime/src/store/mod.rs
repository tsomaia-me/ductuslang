//! The storage seam (M0.md §3, seam 3): the committed values of every cell,
//! double-buffered so the publish is a constant-time index flip (§13.12.4;
//! DECISION_LOG 025-61), plus the staged-write buffer §13.10.1's lazy writes
//! accumulate into. [`Snapshot`] is the read face — §13.14.6 rendered as a
//! borrow view (R22).
//!
//! [`Snapshot`] is the whole public surface. `CellStore` is the engine's own
//! storage — crate-visible so the engine can hold it, never part of the
//! runtime's contract with a host.

pub mod snapshot;

pub(crate) mod cell_store;

pub use self::snapshot::Snapshot;

pub(crate) use self::cell_store::CellStore;
