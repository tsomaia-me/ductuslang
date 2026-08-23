//! The trace seam: the commit trace is the runtime's observable contract (R24
//! minimal-semantic) — values and order only, no timing, no pointers, no
//! implementation shape — because the dual-route equivalence law asserts trace
//! identity across construction routes.

// The file is named after the entity it holds (S1), and the entity is
// `TraceSink`, so `trace_sink/trace_sink.rs` stutters — the same trade
// `value/value.rs` makes. `expect` rather than `allow` so the annotation
// reports itself as stale if the layout ever changes.
#[expect(
    clippy::module_inception,
    reason = "S1 names files after their entity; `TraceSink` lives in `trace_sink/`"
)]
pub mod trace_sink;

pub mod noop_sink;
pub mod trace_event;

pub use self::noop_sink::NoopSink;
pub use self::trace_event::TraceEvent;
pub use self::trace_sink::TraceSink;
