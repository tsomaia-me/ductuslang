//! The `TraceEvent` enum: one observable step of a commit.

use ductus_ir::path::CellPath;

use crate::value::Value;

/// One event of the commit trace — the runtime's observable contract (R24
/// minimal-semantic): values and order only, never implementation shape. No
/// timing, no pointers, no step boundaries, no propagation edges, no
/// suppressed evaluations — the dual-route equivalence law asserts trace
/// IDENTITY across construction routes, so nothing route-dependent may appear
/// here. Internal debugging uses ordinary logging, not the trace.
///
/// No `PartialEq`: the event is a composite (and [`Value`] carries none on
/// purpose), and equality of traces is TEXT equality of rendered lines — the
/// same R17 doctrine the AST follows, where the rendering is the definition of
/// "same".
#[derive(Debug, Clone)]
pub enum TraceEvent {
    /// `commit_begin` — a commit opened.
    CommitBegin,
    /// `dirty <path> <old> -> <new>` — a cell entered the dirty set.
    Dirty {
        /// The dirtied cell.
        path: CellPath,
        /// Its previously-committed value.
        old: Value,
        /// The staged value replacing it.
        new: Value,
    },
    /// `eval <path> -> <value>` — a behavior evaluation produced a value.
    Eval {
        /// The evaluated cell.
        path: CellPath,
        /// The value its behavior produced.
        value: Value,
    },
    /// `gate <path> open` / `gate <path> close` — a gate's EFFECTIVE
    /// activation changed.
    ///
    /// The one event kind M1 adds (R75), and it is added because no existing
    /// event can express it: a transition is observable — the whole guarded
    /// subtree's behavior changes — and it carries no value. A recurrent's
    /// advance and a deferred's landing needed no new kind for the opposite
    /// reason: they ARE a value production and a dirty root, so they reuse
    /// `eval` and `dirty`. An event naming which mechanism produced a value
    /// would record implementation shape, which R24 forbids.
    ///
    /// EFFECTIVE, not own: a nested gate whose own predicate flips under a
    /// closed ancestor changes nothing observable, so it emits nothing
    /// (C-39/C-40). One line per change of "own gate AND every ancestor".
    Gate {
        /// The gate whose effective activation changed.
        path: CellPath,
        /// The activation it changed TO — `true` renders `open`.
        open: bool,
    },
    /// `publish` — the atomic snapshot flip.
    Publish,
    /// `commit_end` — the commit closed.
    CommitEnd,
}

impl TraceEvent {
    /// The canonical trace line — ONE line per event, `'\n'`-terminated,
    /// exactly this grammar (R24's ruled schema):
    ///
    /// ```text
    /// commit_begin
    /// dirty <path> <old> -> <new>
    /// eval <path> -> <value>
    /// gate <path> open
    /// gate <path> close
    /// publish
    /// commit_end
    /// ```
    ///
    /// `<path>` is [`CellPath::render_into`] and the values are
    /// [`Value::render_into`]; single spaces exactly as shown, `->` with a
    /// space each side. The rendering is BYTE-EXACT: the golden files are
    /// byte-compared, so this rendering IS the trace format's canonical form —
    /// there is exactly one spelling of every event.
    #[must_use]
    pub fn render(&self) -> String {
        let mut rendered = String::new();

        self.render_into(&mut rendered);

        rendered
    }

    /// The same canonical line, APPENDED to a caller's sink.
    ///
    /// A recording sink accumulates a whole trace into one buffer, so it pays
    /// no `String` per event.
    pub fn render_into(&self, out: &mut String) {
        match self {
            TraceEvent::CommitBegin => out.push_str("commit_begin"),
            TraceEvent::Dirty { path, old, new } => {
                out.push_str("dirty ");
                path.render_into(out);
                out.push(' ');
                old.render_into(out);
                out.push_str(" -> ");
                new.render_into(out);
            }
            TraceEvent::Eval { path, value } => {
                out.push_str("eval ");
                path.render_into(out);
                out.push_str(" -> ");
                value.render_into(out);
            }
            TraceEvent::Gate { path, open } => {
                out.push_str("gate ");
                path.render_into(out);
                out.push(' ');
                out.push_str(if *open { "open" } else { "close" });
            }
            TraceEvent::Publish => out.push_str("publish"),
            TraceEvent::CommitEnd => out.push_str("commit_end"),
        }

        out.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ductus_ir::path::Segment;

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

    #[test]
    fn renders_the_payload_free_events_as_bare_lines() {
        assert_eq!(TraceEvent::CommitBegin.render(), "commit_begin\n");
        assert_eq!(TraceEvent::Publish.render(), "publish\n");
        assert_eq!(TraceEvent::CommitEnd.render(), "commit_end\n");
    }

    #[test]
    fn renders_a_dirty_line_with_old_and_new() {
        let event = TraceEvent::Dirty {
            path: path(&["App", "count"]),
            old: Value::I64(0),
            new: Value::I64(1),
        };

        assert_eq!(event.render(), "dirty App.count 0 -> 1\n");
    }

    #[test]
    fn renders_an_eval_line() {
        let event = TraceEvent::Eval {
            path: path(&["App", "doubled"]),
            value: Value::I64(2),
        };

        assert_eq!(event.render(), "eval App.doubled -> 2\n");
    }

    // The one M1 event: two spellings, one per direction, and no value —
    // a transition produces none (R75).
    #[test]
    fn renders_a_gate_line_in_both_directions() {
        let opened = TraceEvent::Gate {
            path: path(&["App", "panel"]),
            open: true,
        };
        let closed = TraceEvent::Gate {
            path: path(&["App", "panel"]),
            open: false,
        };

        assert_eq!(opened.render(), "gate App.panel open\n");
        assert_eq!(closed.render(), "gate App.panel close\n");
    }

    // A float event carries the canonical value rendering — `-0.0` spells
    // `-0`, per R34's pinned canon.
    #[test]
    fn renders_float_values_by_the_canonical_rendering() {
        let event = TraceEvent::Eval {
            path: path(&["App", "gain"]),
            value: Value::F64(-0.0),
        };

        assert_eq!(event.render(), "eval App.gain -> -0\n");
    }

    // `<path>` is the path's OWN canon: an ordinal renders `:N`, a key renders
    // by the bare-iff-NAME-shaped rule — nothing here respells any of it.
    #[test]
    fn renders_paths_through_their_own_canon() {
        let ordinal = TraceEvent::Eval {
            path: CellPath::new(vec![
                Segment::Name {
                    text: "App".to_string(),
                    ordinal: None,
                },
                Segment::Name {
                    text: "print".to_string(),
                    ordinal: Some(0),
                },
            ]),
            value: Value::Bool(true),
        };

        assert_eq!(ordinal.render(), "eval App.print:0 -> true\n");

        let keyed = TraceEvent::Dirty {
            path: CellPath::new(vec![
                Segment::Name {
                    text: "App".to_string(),
                    ordinal: None,
                },
                Segment::Name {
                    text: "rows".to_string(),
                    ordinal: None,
                },
                Segment::Key("42".to_string()),
            ]),
            old: Value::I32(0),
            new: Value::I32(7),
        };

        assert_eq!(keyed.render(), "dirty App.rows.'42' 0 -> 7\n");
    }

    // `render_into` is the rendering and `render` is a wrapper around it, so
    // the two agree by construction — and the sink is APPENDED to, never
    // cleared, which is what lets one buffer hold a whole trace.
    #[test]
    fn renders_into_a_callers_sink_by_appending() {
        let mut trace = String::new();

        TraceEvent::CommitBegin.render_into(&mut trace);
        TraceEvent::Eval {
            path: path(&["App", "doubled"]),
            value: Value::I64(2),
        }
        .render_into(&mut trace);
        TraceEvent::CommitEnd.render_into(&mut trace);

        assert_eq!(trace, "commit_begin\neval App.doubled -> 2\ncommit_end\n");
    }
}
