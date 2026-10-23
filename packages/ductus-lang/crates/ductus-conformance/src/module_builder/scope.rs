//! The `scope` function: one `Scope` holding entries in body order.

use ductus_ir::diag::Span;
use ductus_ir::graph::{Entry, Scope};

use super::path::path;

/// The scope at `names` holding `entries` in body order, with the empty clause
/// sets the implemented slice requires — no `exposes`, no `effects`, no
/// scope-level `reset_on_reopen`.
///
/// `entries` is a mixed list on purpose. Gates and cells share ONE entry-rank
/// sequence (C-45), so a gate sits BETWEEN two cells here rather than in a list
/// of its own — which is what makes the rank interleaving of `gate` and `dirty`
/// lines expressible at all (R76).
#[must_use]
pub fn scope(names: &[&str], entries: Vec<Entry>) -> Scope {
    Scope {
        path: path(names),
        span: Span::new(0, 0),
        exposes: Vec::new(),
        effects: Vec::new(),
        reset_on_reopen: Vec::new(),
        entries,
    }
}

#[cfg(test)]
mod tests {
    use ductus_ir::types::Prim;

    use super::super::cell::cell;
    use ductus_ir::graph::CellKind;

    use super::*;

    #[test]
    fn keeps_its_entries_in_the_order_they_were_given() {
        let built = scope(
            &["App"],
            vec![
                Entry::Cell(cell(CellKind::Input, &["App", "x"], Prim::I64)),
                Entry::Cell(cell(CellKind::Derived, &["App", "d"], Prim::I64)),
            ],
        );

        assert_eq!(built.path.render(), "App");
        assert_eq!(built.entries.len(), 2);
    }

    #[test]
    fn carries_no_scope_level_clause() {
        let built = scope(&["App"], Vec::new());

        assert!(built.exposes.is_empty());
        assert!(built.effects.is_empty());
        assert!(built.reset_on_reopen.is_empty());
    }
}
