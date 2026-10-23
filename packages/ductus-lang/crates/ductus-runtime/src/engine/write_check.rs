//! [`write_check`] — the validation both write paths share.

use ductus_ir::diag::{DiagClass, Diagnostic};
use ductus_ir::path::CellPath;

use crate::loader::{CellIndex, CellRole, LoadedGraph};
use crate::value::{Value, ValueType};

/// Validates one host write and answers with the target cell's index — the
/// shared front half of [`Engine::write_signal`] and [`Transaction::write_signal`]
/// (§13.14.2), so a transaction's write is checked by exactly the law a bare
/// write is.
///
/// Three questions, in this order:
///
/// 1. Does a cell of this path exist? No → `unknown_input_path`.
/// 2. Is it an `input` cell? No → `write_to_non_input`. §13.14.2 is explicit
///    that `write_signal` targets signals only; every other kind in the slice
///    is computed, never written — a `recurrent` included, whose control is
///    indirect by §13.2.4.2 ("the host cannot directly write a recurrent cell
///    at runtime … the host writes signals that the recurrent's expression
///    reads").
/// 3. Does the value carry the cell's declared type? No →
///    `type_mismatched_write`.
///
/// Every diagnostic carries the path and NO span: a runtime write is not a
/// position in source text, and the loader already reported everything that
/// pointed there (R21).
///
/// Nothing here mutates anything. A rejected write stages nothing — the caller
/// never reaches its store.
///
/// [`Engine::write_signal`]: super::Engine::write_signal
/// [`Transaction::write_signal`]: super::Transaction::write_signal
pub(super) fn write_check(
    graph: &LoadedGraph,
    path: &CellPath,
    value: &Value,
) -> Result<CellIndex, Diagnostic> {
    let Some(&cell_index) = graph.index.get(path) else {
        return Err(Diagnostic {
            class: DiagClass::UnknownInputPath,
            span: None,
            path: Some(path.clone()),
            message: format!("no cell named `{}` exists", path.render()),
        });
    };

    let cell = &graph.cells[cell_index.0];

    if !matches!(cell.role, CellRole::Input { .. }) {
        return Err(Diagnostic {
            class: DiagClass::WriteToNonInput,
            span: None,
            path: Some(path.clone()),
            message: format!("`{}` is a computed cell, not an input", path.render()),
        });
    }

    if !cell.value_type.matches(value) {
        return Err(Diagnostic {
            class: DiagClass::TypeMismatchedWrite,
            span: None,
            path: Some(path.clone()),
            message: format!(
                "`{}` is declared `{}`; the written value is `{}`",
                path.render(),
                cell.value_type.as_str(),
                ValueType::of(value).as_str()
            ),
        });
    }

    Ok(cell_index)
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{derived, input, module, path, provider, scope};
    use super::*;
    use ductus_ir::graph::ValueLiteral;
    use ductus_ir::types::Prim;

    use crate::loader::load;

    // One graph for every case: an `i64` input, a `bool` input, and a derived
    // over the first.
    fn graph() -> LoadedGraph {
        let module = module(vec![scope(
            &["App"],
            vec![
                input(&["App", "x"], Prim::I64, ValueLiteral::int("1".to_string())),
                input(&["App", "flag"], Prim::Bool, ValueLiteral::False),
                derived(&["App", "d"], Prim::I64, 1, &[&["App", "x"]]),
            ],
        )]);

        load(&module, &provider(&[1])).expect("the module loads")
    }

    #[test]
    fn an_input_of_the_declared_type_passes_and_names_its_index() {
        let graph = graph();

        assert_eq!(
            write_check(&graph, &path(&["App", "x"]), &Value::I64(7)).expect("a legal write"),
            CellIndex(0)
        );
        assert_eq!(
            write_check(&graph, &path(&["App", "flag"]), &Value::Bool(true))
                .expect("a legal write"),
            CellIndex(1)
        );
    }

    #[test]
    fn an_unknown_path_is_unknown_input_path() {
        let graph = graph();
        let failure =
            write_check(&graph, &path(&["App", "nope"]), &Value::I64(7)).expect_err("no such cell");

        assert_eq!(failure.class, DiagClass::UnknownInputPath);
        assert_eq!(
            failure.path.as_ref().map(CellPath::render),
            Some("App.nope".to_string())
        );
        assert!(failure.span.is_none());
    }

    #[test]
    fn a_derived_target_is_write_to_non_input() {
        let graph = graph();
        let failure =
            write_check(&graph, &path(&["App", "d"]), &Value::I64(7)).expect_err("not an input");

        assert_eq!(failure.class, DiagClass::WriteToNonInput);
        assert_eq!(
            failure.path.as_ref().map(CellPath::render),
            Some("App.d".to_string())
        );
        assert!(failure.span.is_none());
    }

    // The type check is `ValueType::matches`, so it is exact: a same-digit
    // value of the wrong width is as wrong as a bool under an integer.
    #[test]
    fn a_wrong_typed_value_is_type_mismatched_write() {
        let graph = graph();

        for value in [Value::I32(7), Value::Bool(true), Value::F64(7.0)] {
            let failure =
                write_check(&graph, &path(&["App", "x"]), &value).expect_err("the type is wrong");

            assert_eq!(failure.class, DiagClass::TypeMismatchedWrite);
            assert_eq!(
                failure.path.as_ref().map(CellPath::render),
                Some("App.x".to_string())
            );
            assert!(failure.span.is_none());
        }
    }

    // Order matters: an unknown path is reported as unknown even when the value
    // would also have been wrong, and a derived target is reported as a
    // non-input even when the value's type does not match either.
    #[test]
    fn the_checks_report_in_order() {
        let graph = graph();
        let unknown = write_check(&graph, &path(&["App", "nope"]), &Value::Bool(true))
            .expect_err("no such cell");
        let non_input = write_check(&graph, &path(&["App", "d"]), &Value::Bool(true))
            .expect_err("not an input");

        assert_eq!(unknown.class, DiagClass::UnknownInputPath);
        assert_eq!(non_input.class, DiagClass::WriteToNonInput);
    }
}
