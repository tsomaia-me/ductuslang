//! The `NoopSink` struct: the production sink that discards every event.

use super::trace_event::TraceEvent;
use super::trace_sink::TraceSink;

/// The production sink, until a real consumer exists.
///
/// The trace is COMPUTED only in the rendering a sink performs — an event
/// carries structure, never text — so discarding costs nothing per event
/// beyond the dispatch: no line is built, no path is joined, no float is
/// spelled.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoopSink;

impl TraceSink for NoopSink {
    fn event(&mut self, _event: &TraceEvent) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    // Nothing to observe IS the contract; the test is that a commit's worth of
    // events flows through the trait object, exactly as the engine emits it.
    #[test]
    fn discards_every_event_through_the_trait_object() {
        let mut noop = NoopSink;
        let sink: &mut dyn TraceSink = &mut noop;

        sink.event(&TraceEvent::CommitBegin);
        sink.event(&TraceEvent::Publish);
        sink.event(&TraceEvent::CommitEnd);
    }
}
