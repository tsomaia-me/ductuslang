//! The `LoadedCell` struct: one validated cell of a loaded graph.

use ductus_ir::path::CellPath;

use super::cell_role::CellRole;
use super::gate_index::GateIndex;
use crate::value::ValueType;

/// One cell of a [`LoadedGraph`](super::loaded_graph::LoadedGraph), fully
/// validated and ready for the engine.
///
/// Everything load-time is DONE by the time one of these exists: the path is
/// plain and unique, the type is word-tier, the clauses matched the kind, the
/// `init` converted, the behavior resolved and the inputs linked (R21 — the
/// loader owns all legality, so the engine checks nothing). What a cell does
/// NOT carry is a span: a loaded graph is a runtime object, not a text
/// position, and every text-pointing diagnostic was already reported at load.
///
/// The fields are CRATE-VISIBLE for the engine wave, like the graph's own: the
/// loader is the only writer, so what a cell says about itself is what the
/// loader checked.
#[derive(Debug, Clone)]
pub struct LoadedCell {
    /// The cell's path — its identity, from day one, so hot reload can match
    /// cells across loads without restructuring (M0.md §2).
    pub(crate) path: CellPath,
    /// The cell's rank among ALL graph-section entries, `gate` entries
    /// included (C-45).
    ///
    /// It is NOT the cell's [`CellIndex`](super::cell_index::CellIndex): that
    /// one is dense over cells alone, and this one counts every entry, so the
    /// two agree exactly when the graph declares no gates. Step 1's root walk
    /// interleaves gate transitions with cell roots by THIS number (R76).
    pub(crate) entry_rank: usize,
    /// The cell's declared type, mapped to the M0 word tier.
    pub(crate) value_type: ValueType,
    /// The gate that guards this cell, or `None` for an ungated one — the
    /// binding after the loader has folded the two spellings together (a
    /// `gate PATH` clause here, membership in some gate's `guards` list there).
    ///
    /// It is the cell's OWN gate, singular. Effective activation ANDs it with
    /// every ancestor along the gate's `parent` chain (§13.9.7), so the whole
    /// activation question for this cell is answered from this one link.
    pub(crate) gate: Option<GateIndex>,
    /// What the cell is: an input with its seed value, or a derived with its
    /// executable and linked inputs.
    pub(crate) role: CellRole,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;
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
    fn a_loaded_cell_carries_path_rank_type_gate_and_role() {
        let cell = LoadedCell {
            path: path(&["App", "count"]),
            entry_rank: 2,
            value_type: ValueType::I32,
            gate: Some(GateIndex(0)),
            role: CellRole::Input {
                init: Value::I32(0),
            },
        };
        let copy = cell.clone();

        assert_eq!(copy.path.render(), "App.count");
        assert_eq!(copy.entry_rank, 2);
        assert_eq!(copy.value_type, ValueType::I32);
        assert_eq!(copy.gate, Some(GateIndex(0)));
        assert!(matches!(copy.role, CellRole::Input { .. }));
    }

    // An ungated cell is the common case and says so: no gate, so no
    // activation question — it is eligible in every commit.
    #[test]
    fn an_ungated_cell_carries_no_gate() {
        let cell = LoadedCell {
            path: path(&["App", "count"]),
            entry_rank: 0,
            value_type: ValueType::I32,
            gate: None,
            role: CellRole::Input {
                init: Value::I32(0),
            },
        };

        assert_eq!(cell.gate, None);
    }
}
