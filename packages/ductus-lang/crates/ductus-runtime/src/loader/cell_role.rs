//! The `CellRole` enum: what a loaded cell is, in executable form.

use std::fmt;

use super::cell_index::CellIndex;
use super::lookback_link::LookbackLink;
use crate::provider::Behavior;
use crate::value::Value;

/// What a loaded cell IS — the implemented slice's cell kinds, carried in the
/// form the engine runs rather than the form the text spelled.
///
/// An `input` cell's `init` literal has already been converted under the cell's
/// declared type (R57), so the engine seeds the store with a [`Value`] and
/// never re-reads text. A `derived` cell's `uses` handle has already been
/// resolved through the provider (D5), and its `inputs` paths have already been
/// linked to dense indices — load-time work done once, at load time. A
/// `recurrent` carries the same two, plus everything its history needs: the
/// ring depth, the linked `lookback` entries, the per-`k` fallbacks already
/// converted under the declared type, and whether a gate reopen clears the
/// lot. A `deferred` carries the first two plus its latency depth — no
/// history, because a latency is not a memory.
///
/// What no role carries is the cell's GATE. Activation is not a property of
/// what a cell computes — an `input` can be gated and computes nothing — so it
/// rides on [`LoadedCell`](super::loaded_cell::LoadedCell) beside the role
/// rather than inside it.
///
/// `Clone` clones the `Rc` inside a [`Behavior`], so two clones of a computed
/// role share one executable — the sharing [`Behavior`]'s contract promises.
#[derive(Clone)]
pub enum CellRole {
    /// An `input` cell: host-written, seeded by its converted `init` value.
    Input {
        /// The `init` literal, converted under the cell's declared type (R57).
        init: Value,
    },
    /// A `derived` cell: computed by its resolved behavior from its input
    /// cells.
    Derived {
        /// The resolved executable (D5: module table ∪ provider registry).
        behavior: Behavior,
        /// The input cells as dense indices, in `inputs` list order. May be
        /// empty — a constant derived is legal and evaluates once at startup.
        inputs: Vec<CellIndex>,
    },
    /// A `recurrent` cell: computed from its input cells AND its own past
    /// (§13.2.4), advancing once per evaluation (R72 + R77).
    ///
    /// The four history fields are exactly what the behavior ABI's argument
    /// order is defined over (R71, §4.9): `inputs` in list order, then
    /// `past1 … pastDepth` ascending, then per `lookback` entry in list order
    /// `<input>_past1 … <input>_pastK` ascending. Arity is therefore
    /// `inputs.len() + depth + sum(lookback k)`, computable from this role
    /// alone.
    Recurrent {
        /// The resolved executable (D5: module table ∪ provider registry).
        behavior: Behavior,
        /// The VALUE input cells as dense indices, in `inputs` list order —
        /// the staleness triggers too: a recurrent evaluates when one of these
        /// is dirty, exactly like a derived (§4.2). Self-history is not among
        /// them; it rides `depth`.
        inputs: Vec<CellIndex>,
        /// The self-history ring's depth — `depth INT`, or 1 when the clause is
        /// absent (C-7/C-8). Always positive, and always equal to
        /// `fallbacks.len()`.
        depth: usize,
        /// The linked `lookback` entries in list order — per-INPUT history
        /// rings (R71). Empty when the clause is absent.
        lookback: Vec<LookbackLink>,
        /// The self-history fallback per depth, index `k - 1`: what
        /// `.past(k, …)` yields before `k` advances exist. Converted under the
        /// declared type at load (R57), from `fallbacks` or from `init`'s
        /// uniform spelling.
        fallbacks: Vec<Value>,
        /// The bare `reset_on_reopen` flag — whether an effective gate REOPEN
        /// clears this cell's history (§13.2.4, C-13).
        ///
        /// It lives on the recurrent alone because only a recurrent has
        /// history to clear (C-42): the flag's whole content is "drop the self
        /// ring and the `lookback` rings", so the first post-gap evaluation
        /// reads FALLBACKS rather than values from before the gap. The clearing
        /// happens in step 1a, before step 3's reads.
        reset_on_reopen: bool,
    },
    /// A `deferred` cell: computed from its input cells exactly as a derived
    /// is, but PUBLISHED `depth` commits after the commit that asked for it
    /// (§13.2.12).
    ///
    /// Its body is never a step-2 DAG node — §13.10.2 says so in those words —
    /// and that is honoured where the WALK happens, not by a second vector: it
    /// takes its place in the loaded graph's one `static_order` (R80 item 4),
    /// where construction evaluates it synchronously (C-23) and where the
    /// commit's step 3 walks past it. Its VALUE cell is an ordinary
    /// slot of the buffers, holding the last published value between landings,
    /// and its edges are ordinary edges: a deferred is a latency, never a cycle
    /// device (§13.11.4), so a loop closed through one is still a
    /// `derived_cycle`.
    Deferred {
        /// The resolved executable (D5: module table ∪ provider registry).
        behavior: Behavior,
        /// The input cells as dense indices, in `inputs` list order — the
        /// PROVENANCE whose dirtiness dispatches a run (C-28), and the
        /// behavior's whole argument list: a deferred has no history and its
        /// ABI is a derived's.
        inputs: Vec<CellIndex>,
        /// `depth INT` — the latency in COMMITS, at least 1 (C-7/C-8/C-24). A
        /// run dispatched at commit `c` lands at commit `c + depth`.
        depth: usize,
    },
}

impl CellRole {
    /// The cells this one reads as VALUE inputs — its dependency edges, and the
    /// only reads that make it stale (§4.2).
    ///
    /// An `input` has none: it is written, never computed. A `recurrent`'s are
    /// its `inputs` list and nothing more — self-history and `lookback` reads
    /// resolve from the history rings at previous-committed values, so they are
    /// neither cycle edges nor staleness triggers (§13.11.3: "reads OF a
    /// recurrent cell … always return the previous-committed value. This is the
    /// rule that breaks reactive cycles"). A `deferred`'s are its `inputs` list
    /// too — the provenance whose dirtiness dispatches a run (C-28).
    pub(crate) fn inputs(&self) -> &[CellIndex] {
        match self {
            CellRole::Input { .. } => &[],
            CellRole::Derived { inputs, .. }
            | CellRole::Recurrent { inputs, .. }
            | CellRole::Deferred { inputs, .. } => inputs,
        }
    }
}

/// Hand-written because [`Behavior`] is an `Rc<dyn Fn…>` and a closure has no
/// `Debug` — nor any rendering that would mean anything. The behavior therefore
/// prints as the opaque marker `<behavior>`, and everything else prints as the
/// derive would have printed it.
impl fmt::Debug for CellRole {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CellRole::Input { init } => out.debug_struct("Input").field("init", init).finish(),
            CellRole::Derived { inputs, .. } => out
                .debug_struct("Derived")
                .field("behavior", &format_args!("<behavior>"))
                .field("inputs", inputs)
                .finish(),
            CellRole::Recurrent {
                inputs,
                depth,
                lookback,
                fallbacks,
                reset_on_reopen,
                ..
            } => out
                .debug_struct("Recurrent")
                .field("behavior", &format_args!("<behavior>"))
                .field("inputs", inputs)
                .field("depth", depth)
                .field("lookback", lookback)
                .field("fallbacks", fallbacks)
                .field("reset_on_reopen", reset_on_reopen)
                .finish(),
            CellRole::Deferred { inputs, depth, .. } => out
                .debug_struct("Deferred")
                .field("behavior", &format_args!("<behavior>"))
                .field("inputs", inputs)
                .field("depth", depth)
                .finish(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    #[test]
    fn an_input_role_debugs_its_converted_value() {
        let role = CellRole::Input {
            init: Value::I32(7),
        };

        assert_eq!(format!("{role:?}"), "Input { init: I32(7) }");
    }

    // The marker is the point: the closure itself has no rendering, and the
    // indices — the part a reader can act on — print in full.
    #[test]
    fn a_derived_role_debugs_its_behavior_as_an_opaque_marker() {
        let role = CellRole::Derived {
            behavior: Rc::new(|_| Value::Bool(true)),
            inputs: vec![CellIndex(0), CellIndex(2)],
        };

        assert_eq!(
            format!("{role:?}"),
            "Derived { behavior: <behavior>, inputs: [CellIndex(0), CellIndex(2)] }"
        );
    }

    // The history fields print in ABI order, which is the order a reader needs
    // them in to check an argument list against a behavior's parameters.
    #[test]
    fn a_recurrent_role_debugs_its_history_shape() {
        let role = CellRole::Recurrent {
            behavior: Rc::new(|_| Value::I64(0)),
            inputs: vec![CellIndex(1)],
            depth: 2,
            lookback: vec![LookbackLink {
                input: CellIndex(1),
                k: 3,
            }],
            fallbacks: vec![Value::I64(1), Value::I64(0)],
            reset_on_reopen: true,
        };

        assert_eq!(
            format!("{role:?}"),
            "Recurrent { behavior: <behavior>, inputs: [CellIndex(1)], depth: 2, \
             lookback: [LookbackLink { input: CellIndex(1), k: 3 }], \
             fallbacks: [I64(1), I64(0)], reset_on_reopen: true }"
        );
    }

    // A deferred debugs its latency and nothing else: a latency is not a
    // memory, so there is no ring, no lookback list and no fallback vector to
    // print.
    #[test]
    fn a_deferred_role_debugs_its_latency() {
        let role = CellRole::Deferred {
            behavior: Rc::new(|_| Value::I64(0)),
            inputs: vec![CellIndex(0)],
            depth: 3,
        };

        assert_eq!(
            format!("{role:?}"),
            "Deferred { behavior: <behavior>, inputs: [CellIndex(0)], depth: 3 }"
        );
    }

    // An input is written, never computed, so it reads nothing; the three
    // computed kinds answer with their `inputs` list — a recurrent's history
    // reads are NOT among them.
    #[test]
    fn only_the_computed_kinds_report_input_edges() {
        let input = CellRole::Input {
            init: Value::I64(0),
        };
        let derived = CellRole::Derived {
            behavior: Rc::new(|_| Value::I64(0)),
            inputs: vec![CellIndex(0)],
        };
        let recurrent = CellRole::Recurrent {
            behavior: Rc::new(|_| Value::I64(0)),
            inputs: vec![CellIndex(0), CellIndex(1)],
            depth: 1,
            lookback: vec![LookbackLink {
                input: CellIndex(0),
                k: 2,
            }],
            fallbacks: vec![Value::I64(0)],
            reset_on_reopen: false,
        };

        let deferred = CellRole::Deferred {
            behavior: Rc::new(|_| Value::I64(0)),
            inputs: vec![CellIndex(1)],
            depth: 2,
        };

        assert!(input.inputs().is_empty());
        assert_eq!(derived.inputs(), [CellIndex(0)]);
        assert_eq!(recurrent.inputs(), [CellIndex(0), CellIndex(1)]);
        assert_eq!(deferred.inputs(), [CellIndex(1)]);
    }

    // Cloning a derived role clones the `Rc`, so the clone runs the SAME
    // executable — shared, not copied.
    #[test]
    fn cloning_a_derived_role_shares_its_executable() {
        let behavior: Behavior = Rc::new(|_| Value::I64(9));
        let role = CellRole::Derived {
            behavior: Rc::clone(&behavior),
            inputs: Vec::new(),
        };
        let copy = role.clone();

        let CellRole::Derived {
            behavior: shared, ..
        } = copy
        else {
            panic!("the clone is a derived role");
        };

        assert!(Rc::ptr_eq(&behavior, &shared));
    }
}
