//! The `BehaviorsSection` struct: a module's behavior table.

use crate::diag::Span;

/// The `behaviors` section: `'behaviors' '{' behavior* '}'` (IR_GRAMMAR §3,
/// §15.4.6).
///
/// Empty at increment 4 — the section parses and emits as `behaviors {}`, and
/// its `behavior` rows arrive at increment 6, where this struct gains the field
/// that holds them. A behavior's BODY is the separate instruction grammar of
/// IR_GRAMMAR §4 and lands with the behavior table, not before it.
///
/// The span is here from the start (R17) so a load-time diagnostic about an
/// unresolvable behavior can point at the table even before the table has
/// content.
#[derive(Debug, Clone)]
pub struct BehaviorsSection {
    /// The `behaviors` keyword through the section's closing `}`.
    pub span: Span,
}
