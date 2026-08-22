//! The `GraphSection` struct: a module's `graph` section.

use crate::diag::Span;
use crate::graph::Scope;

/// The `graph` section: `'graph' '{' scope* '}'` (IR_GRAMMAR §3, §15.4.6).
///
/// The arity is `scope*`, ruled (b) on FND-16 and pinned as 033-256: an empty
/// `graph {}` is structurally legal — a module of pure types — and any
/// minimum-content requirement is a load-time semantic rule rather than a
/// grammar one. The scaffold's earlier `scope+` reading is retired with it.
///
/// The scopes are held in FILE ORDER and nothing sorts them: the emitter writes
/// them back in that order, so order is what makes the round trip byte-stable.
/// Scopes are FLAT here — a scope never holds another scope — because the
/// nesting a program has is carried by each scope's PATH (§15.4.1.1).
///
/// The entries INSIDE a scope (cells, gates, wires, effects, streams) live in
/// `crate::graph`, not here (R55).
///
/// The span is the section's own (R17), so a load-time diagnostic about the
/// graph can point at the section even when the section is empty.
#[derive(Debug, Clone)]
pub struct GraphSection {
    /// The `graph` keyword through the section's closing `}`.
    pub span: Span,
    /// The section's scopes, in file order.
    pub scopes: Vec<Scope>,
}
