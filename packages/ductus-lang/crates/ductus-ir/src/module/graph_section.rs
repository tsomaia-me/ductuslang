//! The `GraphSection` struct: a module's `graph` section.

use crate::diag::Span;

/// The `graph` section: `'graph' '{' scope+ '}'` (IR_GRAMMAR §3, §15.4.6).
///
/// Empty at increment 4 — the section parses and emits as `graph {}`, and its
/// scopes arrive at increment 7, where this struct gains the field that holds
/// them. The entries INSIDE a scope (cells, gates, wires, effects, streams)
/// live in `crate::graph`, not here (R55).
///
/// Note the production's arity: `scope+` requires at least one scope, so the
/// empty `graph {}` this increment accepts is outside the grammar as written.
/// The scaffold admits it because no scope form exists yet; the arity is an
/// open obligation for increment 7 (FND-16), recorded here so it stays visible.
///
/// The span is here from the start (R17) so a load-time diagnostic about the
/// graph can point at the section even before the section has content.
#[derive(Debug, Clone)]
pub struct GraphSection {
    /// The `graph` keyword through the section's closing `}`.
    pub span: Span,
}
