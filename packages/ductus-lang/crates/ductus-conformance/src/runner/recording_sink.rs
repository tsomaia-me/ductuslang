//! The `RecordingSink` struct: the witness-side sink that keeps the trace.

use ductus_runtime::trace_sink::{TraceEvent, TraceSink};

/// The M0 witness sink: renders every event it hears into one owned buffer.
///
/// This is the OTHER half of the trace seam — [`NoopSink`] discards, this
/// records — and it is where the trace becomes text at all: an event carries
/// structure, and [`TraceEvent::render_into`] is the single canonical spelling
/// of it (R24 as amended by R34). Nothing here formats anything itself, so a
/// golden can never drift from the runtime's own rendering.
///
/// One buffer holds a whole run — every commit of a scripted witness, in
/// order — because a golden is the trace of the SCRIPT, not of one commit.
///
/// [`NoopSink`]: ductus_runtime::trace_sink::NoopSink
#[derive(Debug, Clone, Default)]
pub struct RecordingSink {
    trace: String,
}

impl RecordingSink {
    /// An empty sink, ready to hear a run.
    #[must_use]
    pub fn new() -> RecordingSink {
        RecordingSink::default()
    }

    /// Everything heard so far, as the trace text.
    ///
    /// Borrowing suits mid-run inspection and the dual-construction assertion,
    /// where two live sinks are compared against each other.
    #[must_use]
    pub fn trace(&self) -> &str {
        &self.trace
    }

    /// Everything heard, taking ownership of the buffer.
    ///
    /// The engine borrows its sink for its whole life, so a caller reaches
    /// this only after the engine is gone — which is exactly when a witness
    /// wants the trace by value.
    #[must_use]
    pub fn into_trace(self) -> String {
        self.trace
    }
}

impl TraceSink for RecordingSink {
    fn event(&mut self, event: &TraceEvent) {
        event.render_into(&mut self.trace);
    }
}

#[cfg(test)]
mod tests {
    use ductus_ir::path::{CellPath, Segment};
    use ductus_runtime::value::Value;

    use super::*;

    fn path(names: &[&str]) -> CellPath {
        CellPath::new(
            names
                .iter()
                .map(|text| Segment::Name {
                    text: (*text).to_string(),
                    ordinal: None,
                })
                .collect(),
        )
    }

    // Exercised THROUGH `&mut dyn TraceSink`, exactly as the engine holds it.
    #[test]
    fn accumulates_the_rendered_events_of_a_whole_run() {
        let mut recording = RecordingSink::new();

        {
            let sink: &mut dyn TraceSink = &mut recording;

            sink.event(&TraceEvent::CommitBegin);
            sink.event(&TraceEvent::Dirty {
                path: path(&["App", "count"]),
                old: Value::I32(0),
                new: Value::I32(7),
            });
            sink.event(&TraceEvent::Eval {
                path: path(&["App", "doubled"]),
                value: Value::I32(14),
            });
            sink.event(&TraceEvent::Publish);
            sink.event(&TraceEvent::CommitEnd);
        }

        assert_eq!(
            recording.trace(),
            "commit_begin\n\
             dirty App.count 0 -> 7\n\
             eval App.doubled -> 14\n\
             publish\n\
             commit_end\n"
        );
        assert_eq!(recording.clone().into_trace(), recording.trace());
    }

    // A fresh sink is empty, and a run with no events leaves it empty — the
    // trace of nothing is the empty string, never a blank line.
    #[test]
    fn starts_empty() {
        assert_eq!(RecordingSink::new().trace(), "");
        assert_eq!(RecordingSink::default().into_trace(), "");
    }
}
