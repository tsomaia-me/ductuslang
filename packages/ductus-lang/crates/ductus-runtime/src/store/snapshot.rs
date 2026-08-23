//! The `Snapshot` struct: a consistent read view of the committed state.

use std::collections::HashMap;

use ductus_ir::diag::{DiagClass, Diagnostic};
use ductus_ir::path::CellPath;

use crate::loader::CellIndex;
use crate::value::Value;

/// A consistent view of the latest committed state — `acquire_snapshot`'s
/// return (§13.14.6), rendered in M0 as a BORROW view (R22).
///
/// # What the borrow buys
///
/// §13.14.6 asks for three things: every cell read from one snapshot is
/// coherent with every other, reads are wait-free, and holding one never blocks
/// a commit. The first two fall out of the shape — a snapshot is one immutable
/// slice, so there is no torn read to have and no synchronization to wait on.
/// The third is where M0 differs, deliberately: the borrow checker makes
/// "commit while a snapshot is alive" a COMPILE error rather than something the
/// runtime has to make safe, because [`Engine::commit`] takes `&mut self` and
/// this view holds `&`. M0 is strictly sequential, so nothing loses a capability
/// it had; a later milestone's concurrent index-flip snapshot arrives behind
/// the same API name and lifts the restriction without renaming anything.
///
/// [`Engine::commit`]: crate::engine::Engine::commit
pub struct Snapshot<'a> {
    index: &'a HashMap<CellPath, CellIndex>,
    values: &'a [Value],
}

impl<'a> Snapshot<'a> {
    /// Builds the view over a graph's path index and the store's live buffer.
    pub(crate) fn new(
        index: &'a HashMap<CellPath, CellIndex>,
        values: &'a [Value],
    ) -> Snapshot<'a> {
        Snapshot { index, values }
    }

    /// Reads `path`'s committed value.
    ///
    /// An unknown path is `unknown_input_path`, carrying the path that was
    /// asked for. There is no span: a runtime read is not a position in source
    /// text, and every text-pointing diagnostic was already reported at load.
    ///
    /// # Errors
    ///
    /// [`DiagClass::UnknownInputPath`] when no cell of the loaded graph has
    /// this path.
    pub fn read(&self, path: &CellPath) -> Result<Value, Diagnostic> {
        let Some(cell) = self.index.get(path) else {
            return Err(Diagnostic {
                class: DiagClass::UnknownInputPath,
                span: None,
                path: Some(path.clone()),
                message: format!("no cell named `{}` exists", path.render()),
            });
        };

        Ok(self.values[cell.0])
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

    fn index() -> HashMap<CellPath, CellIndex> {
        let mut index = HashMap::new();

        index.insert(path(&["App", "x"]), CellIndex(0));
        index.insert(path(&["App", "flag"]), CellIndex(1));
        index.insert(path(&["App", "panel", "gain"]), CellIndex(2));

        index
    }

    // Every cell of the graph reads back through its path, whatever its type
    // and however deep its scope — one slice, so the three reads are coherent
    // with each other by construction.
    #[test]
    fn reads_every_cell_by_path() {
        let index = index();
        let values = [Value::I64(7), Value::Bool(true), Value::F64(-0.0)];
        let snapshot = Snapshot::new(&index, &values);

        assert!(
            snapshot
                .read(&path(&["App", "x"]))
                .expect("the cell exists")
                .bits_eq(&Value::I64(7))
        );
        assert!(
            snapshot
                .read(&path(&["App", "flag"]))
                .expect("the cell exists")
                .bits_eq(&Value::Bool(true))
        );
        assert!(
            snapshot
                .read(&path(&["App", "panel", "gain"]))
                .expect("the cell exists")
                .bits_eq(&Value::F64(-0.0))
        );
    }

    #[test]
    fn an_unknown_path_is_unknown_input_path() {
        let index = index();
        let values = [Value::I64(7), Value::Bool(true), Value::F64(-0.0)];
        let snapshot = Snapshot::new(&index, &values);
        let failure = snapshot
            .read(&path(&["App", "missing"]))
            .expect_err("the cell does not exist");

        assert_eq!(failure.class, DiagClass::UnknownInputPath);
        assert_eq!(
            failure.path.as_ref().map(CellPath::render),
            Some("App.missing".to_string())
        );
        assert!(failure.span.is_none());
    }
}
