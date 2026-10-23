//! The `LoadedGate` struct: one validated `gate` entry of a loaded graph.

use std::fmt;

use ductus_ir::path::CellPath;

use super::cell_index::CellIndex;
use super::gate_index::GateIndex;
use crate::provider::Behavior;

/// One `gate` entry of a [`LoadedGraph`](super::loaded_graph::LoadedGraph),
/// fully validated and ready for the engine (§13.9).
///
/// A gate gates PROPAGATION, never existence: everything it guards is
/// constructed unconditionally and, while the gate is closed, is **frozen** —
/// its cells hold their last committed values and resume when the gate reopens
/// (§13.9.7's Model B). So a gate is not a cell, occupies no slot in the store,
/// and appears in neither evaluation order. Its whole runtime footprint is the
/// two booleans the engine holds beside it (C-50) and the predicate below.
///
/// # What the loader has already answered
///
/// `guards` is GONE by the time one of these exists, and its absence is the
/// point: the BINDING is the guarded construct's own back-reference (`gate PATH`
/// on a cell, `gate_parent PATH` on a gate), and a `guards` list is its
/// materialized inverse (§15.4.1) — a restatement, checked for EQUALITY at load
/// (`guards_inverse_mismatch`, C-43) and then dropped. What survives here is the
/// direction the engine reads: each cell's own gate, and each gate's parent.
///
/// The fields are CRATE-VISIBLE like the graph's own: the loader is the only
/// writer (R21).
#[derive(Clone)]
pub struct LoadedGate {
    /// The gate's path — its identity in the one namespace cells and scopes
    /// share, and what the `gate <path> open|close` trace line renders.
    pub(crate) path: CellPath,
    /// The gate's rank among ALL graph-section entries, cells included (C-45,
    /// 033-260's letter).
    ///
    /// Step 1's root walk is one rank-ordered sequence with every root source
    /// interleaved (R76), and gate transitions are one of those sources — so a
    /// gate needs a rank on the same scale a cell's [`CellIndex`] is on, which
    /// is what this is.
    pub(crate) entry_rank: usize,
    /// `pred BID`, resolved through the provider (D5). It answers `bool` and
    /// anything else is a trap at the gate's path (§13.9.4).
    pub(crate) predicate: Behavior,
    /// `inputs path_set`, linked to dense indices in list order — the cells the
    /// predicate reads, and the whole of its argument list.
    ///
    /// They are read at PRIOR-COMMIT values (R73): the predicate pre-pass runs
    /// against the live buffer at commit start, so a staged write publishes at
    /// commit N and the predicate sees it at N+1.
    pub(crate) inputs: Vec<CellIndex>,
    /// The nearest enclosing gate, or `None` at the root of a chain — §15.4.1's
    /// `gate_parent` clause, verbatim: the enclosing gate's `guards` list must
    /// name this gate back, but it is the clause that binds.
    ///
    /// Effective activation climbs this chain: an instance is effectively
    /// active iff its own gate AND every ancestor is open (§13.9.7). The loader
    /// has proved the chain acyclic (`gate_parent_cycle`), so the climb
    /// terminates.
    pub(crate) parent: Option<GateIndex>,
}

/// Hand-written for the same reason [`CellRole`](super::cell_role::CellRole)'s
/// is: a [`Behavior`] is an `Rc<dyn Fn…>` and a closure has no rendering that
/// would mean anything, so it prints as the opaque marker `<behavior>`.
impl fmt::Debug for LoadedGate {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.debug_struct("LoadedGate")
            .field("path", &self.path)
            .field("entry_rank", &self.entry_rank)
            .field("predicate", &format_args!("<behavior>"))
            .field("inputs", &self.inputs)
            .field("parent", &self.parent)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    use ductus_ir::path::Segment;

    use crate::value::Value;

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

    fn gate() -> LoadedGate {
        LoadedGate {
            path: path(&["App", "g"]),
            entry_rank: 3,
            predicate: Rc::new(|_| Value::Bool(true)),
            inputs: vec![CellIndex(0)],
            parent: Some(GateIndex(0)),
        }
    }

    // The marker is the point: the closure has no rendering, and everything a
    // reader can act on — the path, the rank, the linked inputs, the parent —
    // prints in full.
    #[test]
    fn a_gate_debugs_its_predicate_as_an_opaque_marker() {
        let rendered = format!("{:?}", gate());

        assert!(rendered.contains("predicate: <behavior>"), "{rendered}");
        assert!(rendered.contains("entry_rank: 3"), "{rendered}");
        assert!(rendered.contains("inputs: [CellIndex(0)]"), "{rendered}");
        assert!(
            rendered.contains("parent: Some(GateIndex(0))"),
            "{rendered}"
        );
    }

    // Cloning clones the `Rc`, so the clone runs the SAME predicate — shared,
    // not copied, exactly as a cell role's behavior is.
    #[test]
    fn cloning_a_gate_shares_its_predicate() {
        let original = gate();
        let copy = original.clone();

        assert!(Rc::ptr_eq(&original.predicate, &copy.predicate));
    }
}
