//! The `LoadedGraph` struct: a validated module, ready for the engine.

use std::collections::HashMap;

use ductus_ir::path::CellPath;

use super::cell_index::CellIndex;
use super::loaded_cell::LoadedCell;
use super::loaded_gate::LoadedGate;

/// A validated, linked, ordered graph — what [`load`](super::load::load) hands
/// the engine.
///
/// This is the module with every load-time question answered: legality checked
/// (R21), behaviors resolved (D5), inputs linked to dense indices, and the
/// static evaluation order computed. The engine allocates the store from
/// `cells`, looks paths up through `index`, walks `static_order` at startup
/// (§13.14.1 step 3) and again at commit time (§13.10.2 step 2), and runs each
/// of `gates`' predicates in step 1a (R73) — it never re-derives any of the
/// four.
///
/// The fields are CRATE-VISIBLE for the engine wave: the engine reads them
/// directly, and the loader stays the only writer — which is what makes R21's
/// "the engine checks nothing" true rather than merely intended. The four
/// fields are four views of one graph and only agree because one builder wrote
/// them all; a host that could push a cell without touching the index would
/// hand the engine a desynchronized set. Outside
/// the crate the contract is by path, through
/// [`Snapshot::read`](crate::store::Snapshot::read).
#[derive(Debug, Clone)]
pub struct LoadedGraph {
    /// Every cell, in RANK order — scope-major, then body order (FND-22/R64) —
    /// so `cells[i]` is the cell [`CellIndex`]`(i)` names.
    pub(crate) cells: Vec<LoadedCell>,
    /// Path → dense index, over every cell. Cell identity is BY PATH from day
    /// one (M0.md §2), and this map is that identity's lookup.
    pub(crate) index: HashMap<CellPath, CellIndex>,
    /// THE static evaluation order — §13.14.1 step 3's construction walk and
    /// §13.10.2 step 2's per-commit walk, which are ONE vector (R80 item 4).
    ///
    /// The nodes are every COMPUTED cell, `deferred`s included. An `input` is
    /// absent because it is written, never evaluated. A `deferred` is present
    /// because construction evaluates it synchronously, like a derived (C-23,
    /// 016-298); §13.10.2's "the cell's own body is never a step-2 DAG node" is
    /// honoured by step 3 WALKING PAST it, not by a second vector — at commit
    /// time its value arrives as a landing at step 1 and its body dispatches at
    /// step 4½.
    ///
    /// The edges are every `inputs` read of a computed cell, a read of a
    /// `recurrent` included: a reader follows what it reads, in construction and
    /// at commit alike, so no reader ever sees a derived's allocation
    /// placeholder. What is NOT an edge is self-history and `lookback` — neither
    /// is an `inputs` read to begin with. A recurrent read is dropped from the
    /// order only where honouring it would deadlock, which is §13.11.3's
    /// previous-committed reading applied to the schedule and is what lets a
    /// recurrent close a reactive loop and still take a place here; the released
    /// reader gets that recurrent's depth-1 FALLBACK, which is what the engine
    /// seeds a recurrent's slot with.
    pub(crate) static_order: Vec<CellIndex>,
    /// Every `gate` entry, in the order the same Pass-A walk met them — so
    /// `gates[i]` is the gate [`GateIndex`](super::gate_index::GateIndex)`(i)`
    /// names, and each carries its own graph-section
    /// [`entry_rank`](LoadedGate::entry_rank) for step 1's interleaved root
    /// walk (C-45, R76).
    ///
    /// Gates are in NEITHER evaluation order, and that is the design rather
    /// than an omission: a predicate is not a step-2 DAG node (R73), it runs in
    /// step 1a's pre-pass against prior-commit values, and its boolean occupies
    /// no graph cell (C-50). Nothing about a gate is readable through
    /// [`Snapshot`](crate::store::Snapshot) — gate state is observable only
    /// through the trace's `gate <path> open|close` lines.
    pub(crate) gates: Vec<LoadedGate>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::cell_role::CellRole;
    use crate::value::{Value, ValueType};
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

    // The struct is plain data; the loader's own tests are where the invariants
    // (rank order, index totality, topological order) are pinned against real
    // modules. This one checks only that the four fields hold together.
    #[test]
    fn a_loaded_graph_carries_cells_index_order_and_gates() {
        let count = path(&["App", "count"]);
        let cells = vec![LoadedCell {
            path: count.clone(),
            entry_rank: 0,
            value_type: ValueType::I32,
            gate: None,
            role: CellRole::Input {
                init: Value::I32(0),
            },
        }];
        let mut index = HashMap::new();
        index.insert(count.clone(), CellIndex(0));

        let graph = LoadedGraph {
            cells,
            index,
            static_order: Vec::new(),
            gates: Vec::new(),
        };

        assert_eq!(graph.cells.len(), 1);
        assert_eq!(graph.index.get(&count), Some(&CellIndex(0)));
        assert!(graph.static_order.is_empty());
        assert!(graph.gates.is_empty());
    }
}
