//! The `module` function: the module named `App` that every witness builds.

use ductus_ir::diag::Span;
use ductus_ir::graph::Scope;
use ductus_ir::module::{BehaviorsSection, GraphSection, Module, TypesSection};

/// A module named `App` holding `scopes`, with an EMPTY type table and an
/// EMPTY behavior table.
///
/// Both emptinesses are the slice's, not a convenience. No witness declares a
/// type — every cell is a `Prim` — and no witness declares a behavior body: the
/// provider registry is the whole resolvability answer while the module half of
/// the D5 seam stays empty.
#[must_use]
pub fn module(scopes: Vec<Scope>) -> Module {
    Module {
        name: "App".to_string(),
        span: Span::new(0, 0),
        types: TypesSection {
            span: Span::new(0, 0),
            type_defs: Vec::new(),
        },
        graph: GraphSection {
            span: Span::new(0, 0),
            scopes,
        },
        behaviors: BehaviorsSection {
            span: Span::new(0, 0),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::super::scope::scope;

    use super::*;

    #[test]
    fn names_the_module_app_and_carries_the_scopes_it_was_given() {
        let built = module(vec![scope(&["App"], Vec::new())]);

        assert_eq!(built.name, "App");
        assert_eq!(built.graph.scopes.len(), 1);
    }

    #[test]
    fn declares_no_type_and_no_behavior() {
        let built = module(Vec::new());

        assert!(built.types.type_defs.is_empty());
    }
}
