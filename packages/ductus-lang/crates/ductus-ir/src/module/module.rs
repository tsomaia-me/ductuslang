//! The `Module` struct: a `.duir` module's name and its three sections.

use super::behaviors_section::BehaviorsSection;
use super::graph_section::GraphSection;
use super::types_section::TypesSection;
use crate::diag::Span;

/// One parsed `.duir` module — the root of the text form's AST (§15.4.6).
///
/// `module NAME '{' types_section graph_section behaviors_section '}'`
/// (IR_GRAMMAR §3): all three sections are mandatory and their order is fixed,
/// so they are plain fields rather than a list to search.
///
/// The parser builds this DIRECTLY, with no concrete syntax tree in between
/// (R16). `.duir` is a serialization with one canonical formatting, so the
/// layout a CST would preserve has no customer.
///
/// Plain data: public fields, no constructor, no validation. Legality is the
/// loader's alone (F4) — duplicate paths, unknown references, slice checks —
/// so a `Module` that parses may still be an illegal program.
///
/// No `PartialEq` (R17). Two modules are the same when they emit the same
/// canonical text, and `text::emitter::emit` is that definition. A derived
/// equality would fold SPANS into the comparison, making the same module read
/// from two differently-formatted files compare unequal.
#[derive(Debug, Clone)]
pub struct Module {
    /// The NAME following the `module` keyword.
    pub name: String,
    /// The `module` keyword through the module's closing `}` (R17).
    pub span: Span,
    /// The `types` section.
    pub types: TypesSection,
    /// The `graph` section.
    pub graph: GraphSection,
    /// The `behaviors` section.
    pub behaviors: BehaviorsSection,
}

#[cfg(test)]
mod tests {
    use super::*;

    // The canonical minimal module. Its text and every byte offset below are
    // pinned by the tests in `text::parser::parse` and `text::emitter::emit`;
    // this file only checks that a `Module` carries them.
    #[test]
    fn a_module_carries_its_name_span_and_three_sections() {
        let module = Module {
            name: "App".to_string(),
            span: Span::new(0, 51),
            types: TypesSection {
                span: Span::new(15, 8),
            },
            graph: GraphSection {
                span: Span::new(26, 8),
            },
            behaviors: BehaviorsSection {
                span: Span::new(37, 12),
            },
        };

        let copy = module.clone();

        assert_eq!(copy.name, "App");
        assert_eq!(copy.span.end(), 51);
        assert_eq!(copy.types.span, Span::new(15, 8));
        assert_eq!(copy.graph.span, Span::new(26, 8));
        assert_eq!(copy.behaviors.span, Span::new(37, 12));
    }
}
