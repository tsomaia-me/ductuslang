//! The `CellRole` enum: what a loaded cell is, in executable form.

use std::fmt;

use super::cell_index::CellIndex;
use crate::provider::Behavior;
use crate::value::Value;

/// What a loaded cell IS — the M0 slice's two cell kinds (M0.md §2), carried in
/// the form the engine runs rather than the form the text spelled.
///
/// An `input` cell's `init` literal has already been converted under the cell's
/// declared type (R57), so the engine seeds the store with a [`Value`] and
/// never re-reads text. A `derived` cell's `uses` handle has already been
/// resolved through the provider (D5), and its `inputs` paths have already been
/// linked to dense indices — load-time work done once, at load time.
///
/// `Clone` clones the `Rc` inside a [`Behavior`], so two clones of a derived
/// role share one executable — the sharing [`Behavior`]'s contract promises.
#[derive(Clone)]
pub enum CellRole {
    /// An `input` cell: host-written, seeded by its converted `init` value.
    Input {
        /// The `init` literal, converted under the cell's declared type (R57).
        init: Value,
    },
    /// A `derived` cell: computed by its resolved behavior from its input
    /// cells.
    Derived {
        /// The resolved executable (D5: module table ∪ provider registry).
        behavior: Behavior,
        /// The input cells as dense indices, in `inputs` list order. May be
        /// empty — a constant derived is legal and evaluates once at startup.
        inputs: Vec<CellIndex>,
    },
}

/// Hand-written because [`Behavior`] is an `Rc<dyn Fn…>` and a closure has no
/// `Debug` — nor any rendering that would mean anything. The behavior therefore
/// prints as the opaque marker `<behavior>`, and everything else prints as the
/// derive would have printed it.
impl fmt::Debug for CellRole {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CellRole::Input { init } => out.debug_struct("Input").field("init", init).finish(),
            CellRole::Derived { inputs, .. } => out
                .debug_struct("Derived")
                .field("behavior", &format_args!("<behavior>"))
                .field("inputs", inputs)
                .finish(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    #[test]
    fn an_input_role_debugs_its_converted_value() {
        let role = CellRole::Input {
            init: Value::I32(7),
        };

        assert_eq!(format!("{role:?}"), "Input { init: I32(7) }");
    }

    // The marker is the point: the closure itself has no rendering, and the
    // indices — the part a reader can act on — print in full.
    #[test]
    fn a_derived_role_debugs_its_behavior_as_an_opaque_marker() {
        let role = CellRole::Derived {
            behavior: Rc::new(|_| Value::Bool(true)),
            inputs: vec![CellIndex(0), CellIndex(2)],
        };

        assert_eq!(
            format!("{role:?}"),
            "Derived { behavior: <behavior>, inputs: [CellIndex(0), CellIndex(2)] }"
        );
    }

    // Cloning a derived role clones the `Rc`, so the clone runs the SAME
    // executable — shared, not copied.
    #[test]
    fn cloning_a_derived_role_shares_its_executable() {
        let behavior: Behavior = Rc::new(|_| Value::I64(9));
        let role = CellRole::Derived {
            behavior: Rc::clone(&behavior),
            inputs: Vec::new(),
        };
        let copy = role.clone();

        let CellRole::Derived {
            behavior: shared, ..
        } = copy
        else {
            panic!("the clone is a derived role");
        };

        assert!(Rc::ptr_eq(&behavior, &shared));
    }
}
