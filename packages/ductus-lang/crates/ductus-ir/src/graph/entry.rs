//! The `Entry` enum: one entry of a scope body.

use super::cell::Cell;
use super::effect::Effect;
use super::gate::Gate;
use super::stream::Stream;
use super::wire::Wire;

/// One entry inside a scope's braces — IR_GRAMMAR §3's `entry` (§15.4.6).
///
/// ```text
/// entry ::= cell | gate | wire | effect | stream
/// ```
///
/// Five alternatives, each distinguished by its FIRST word: one of the five
/// cell kinds, or `gate`, `wire`, `effect`, `stream`. A scope holds them in
/// FILE ORDER and nothing sorts them — the emitter writes them back in that
/// order, which is what makes the round trip byte-stable, and entry order is
/// also the candidate carrier of declaration order for the loader's tiebreak
/// (§10's probe #1, unruled — the parser only preserves what it read).
///
/// Each entry is terminated by a NEWLINE, or by the scope's closing `}` when it
/// shares that line (033-250).
///
/// No `PartialEq` (R17): a composite AST node, compared by emitted text.
#[derive(Debug, Clone)]
pub enum Entry {
    /// A `cell` line, led by one of the five kind words.
    Cell(Cell),
    /// A `gate` line.
    Gate(Gate),
    /// A `wire` line.
    Wire(Wire),
    /// An `effect` line.
    Effect(Effect),
    /// A `stream` line.
    Stream(Stream),
}
