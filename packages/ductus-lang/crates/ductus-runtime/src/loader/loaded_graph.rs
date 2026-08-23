//! The `LoadedGraph` struct: a validated module, ready for the engine.

use std::collections::HashMap;

use ductus_ir::path::CellPath;

use super::cell_index::CellIndex;
use super::loaded_cell::LoadedCell;

/// A validated, linked, ordered graph — what [`load`](super::load::load) hands
/// the engine.
///
/// This is the module with every load-time question answered: legality checked
/// (R21), behaviors resolved (D5), inputs linked to dense indices, and the
/// static evaluation order computed. The engine allocates the store from
/// `cells`, looks paths up through `index`, and walks `static_order` at commit
/// time (§13.10.2 step 2) — it never re-derives any of the three.
///
/// The fields are CRATE-VISIBLE for the engine wave: the engine reads them
/// directly, and the loader stays the only writer — which is what makes R21's
/// "the engine checks nothing" true rather than merely intended. `cells`,
/// `index` and `static_order` are three views of one graph and only agree
/// because one builder wrote all three; a host that could push a cell without
/// touching the index would hand the engine a desynchronized triple. Outside
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
    /// The static evaluation order: every DERIVED cell — inputs are written,
    /// never evaluated — topologically sorted over derived→derived edges, ties
    /// broken by rank (R64's same-level tiebreak). The engine's per-commit walk
    /// is this order filtered to the dirty set.
    pub(crate) static_order: Vec<CellIndex>,
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
    // modules. This one checks only that the three fields hold together.
    #[test]
    fn a_loaded_graph_carries_cells_index_and_order() {
        let count = path(&["App", "count"]);
        let cells = vec![LoadedCell {
            path: count.clone(),
            value_type: ValueType::I32,
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
        };

        assert_eq!(graph.cells.len(), 1);
        assert_eq!(graph.index.get(&count), Some(&CellIndex(0)));
        assert!(graph.static_order.is_empty());
    }
}
