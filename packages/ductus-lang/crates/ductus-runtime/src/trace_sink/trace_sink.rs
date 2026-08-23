//! The `TraceSink` trait: the seam through which the engine emits the trace.

use super::trace_event::TraceEvent;

/// The seam through which the commit engine emits trace events (M0.md §3,
/// seam 2).
///
/// The engine emits; the sink decides what hearing an event means. Tests
/// capture rendered lines and byte-compare them against the goldens — the real
/// recording sink lives in `ductus-conformance` — and production uses
/// [`NoopSink`](super::NoopSink) until a real consumer exists.
///
/// Object-safe on purpose: the engine holds `&mut dyn TraceSink`.
pub trait TraceSink {
    /// Hears one event, in commit order.
    fn event(&mut self, event: &TraceEvent);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;
    use ductus_ir::path::{CellPath, Segment};

    // The shape of the real recording sink — which lives in ductus-conformance
    // — reduced to what proving the seam needs: hear, render, accumulate.
    struct RecordingSink {
        trace: String,
    }

    impl TraceSink for RecordingSink {
        fn event(&mut self, event: &TraceEvent) {
            event.render_into(&mut self.trace);
        }
    }

    // Exercised THROUGH `&mut dyn TraceSink`, exactly as the engine holds it —
    // the test is the object-safety proof as much as the contract's.
    #[test]
    fn hears_events_in_commit_order_through_the_trait_object() {
        let mut recording = RecordingSink {
            trace: String::new(),
        };
        let sink: &mut dyn TraceSink = &mut recording;

        sink.event(&TraceEvent::CommitBegin);
        sink.event(&TraceEvent::Eval {
            path: CellPath::new(vec![
                Segment::Name {
                    text: "App".to_string(),
                    ordinal: None,
                },
                Segment::Name {
                    text: "doubled".to_string(),
                    ordinal: None,
                },
            ]),
            value: Value::I64(2),
        });
        sink.event(&TraceEvent::CommitEnd);

        assert_eq!(
            recording.trace,
            "commit_begin\neval App.doubled -> 2\ncommit_end\n"
        );
    }
}
