//! Commit engine: the six steps of SPEC §13.10.2, mirrored literally — dirty set →
//! evaluation order (static-order walk) → evaluate → advance_recurrents (explicitly
//! empty until M1) → publish → clear. Emits trace events through the trace_sink seam.
//! Lands in increment 12.
