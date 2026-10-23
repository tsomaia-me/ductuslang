//! The `cell` function: a `Cell` with every optional clause absent.

use ductus_ir::diag::Span;
use ductus_ir::graph::{Cell, CellKind};
use ductus_ir::types::{Prim, TypeTag};

use super::path::path;

/// A cell of `kind` at `names`, typed `prim`, with EVERY optional clause
/// absent.
///
/// This is the base every witness's kind builder starts from — an `input`
/// builder fills `init`, a `recurrent` builder fills `depth` and its seeds, a
/// `deferred` builder fills `depth` alone. Starting from one all-absent value
/// means a clause a builder does not mention is provably not set, which is what
/// makes "this witness carries no `gate` clause" a fact about the module rather
/// than about the builder that wrote it.
///
/// `span` is `Span::new(0, 0)`: a programmatically-built module has no source
/// text, and no claim any witness makes is about a span.
#[must_use]
pub fn cell(kind: CellKind, names: &[&str], prim: Prim) -> Cell {
    Cell {
        kind,
        path: path(names),
        span: Span::new(0, 0),
        tag: TypeTag::Prim(prim),
        uses: None,
        inputs: Vec::new(),
        depth: None,
        lookback: Vec::new(),
        reset_on_reopen: false,
        init: None,
        fallbacks: Vec::new(),
        gate: None,
        combiner: None,
        else_value: None,
        members: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carries_the_kind_the_path_and_the_type_it_was_given() {
        let built = cell(CellKind::Derived, &["App", "d"], Prim::I64);

        assert!(matches!(built.kind, CellKind::Derived));
        assert_eq!(built.path.render(), "App.d");
        assert!(matches!(built.tag, TypeTag::Prim(Prim::I64)));
    }

    #[test]
    fn leaves_every_optional_clause_absent() {
        let built = cell(CellKind::Recurrent, &["App", "r"], Prim::F64);

        assert!(built.uses.is_none());
        assert!(built.inputs.is_empty());
        assert!(built.depth.is_none());
        assert!(built.lookback.is_empty());
        assert!(!built.reset_on_reopen);
        assert!(built.init.is_none());
        assert!(built.fallbacks.is_empty());
        assert!(built.gate.is_none());
    }
}
