//! The `LoadedCell` struct: one validated cell of a loaded graph.

use ductus_ir::path::CellPath;

use super::cell_role::CellRole;
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
    /// The cell's declared type, mapped to the M0 word tier.
    pub(crate) value_type: ValueType,
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
    fn a_loaded_cell_carries_path_type_and_role() {
        let cell = LoadedCell {
            path: path(&["App", "count"]),
            value_type: ValueType::I32,
            role: CellRole::Input {
                init: Value::I32(0),
            },
        };
        let copy = cell.clone();

        assert_eq!(copy.path.render(), "App.count");
        assert_eq!(copy.value_type, ValueType::I32);
        assert!(matches!(copy.role, CellRole::Input { .. }));
    }
}
