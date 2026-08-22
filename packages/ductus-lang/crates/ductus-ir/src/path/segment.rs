//! The `Segment` enum: one step of a cell path.

/// One step of a cell path — IR_GRAMMAR §2's `PathSegment` (§15.4.1.1).
///
/// ```text
/// PathSegment ::= NAME (':' INT)?
///               | KeySegment
/// KeySegment  ::= BareKey | QuotedKey
/// ```
///
/// Two variants for the production's two alternatives. A name segment may carry
/// an ORDINAL suffix — the zero-based declaration-order index among siblings of
/// the same type at the same nesting depth, which is how an anonymous or
/// duplicated placement is named (`App.print:0`). The suffix attaches to ANY
/// segment, not just the tail: `App.print:0.text` puts it on the middle
/// component. A key segment carries no ordinal — the grammar does not admit one
/// (033-243).
///
/// The ordinal is a STRUCTURAL number (R57): it indexes siblings rather than
/// holding a value of some cell's type, so the parser converts it to `u64` at
/// the position where its type is known, and it never travels as text.
///
/// # Which variant a parsed segment lands in
///
/// A `BareKey` is the NAME charset exactly (R41), so it lexes as a NAME and is
/// indistinguishable from a name segment in isolation. The parser therefore
/// reads every word-shaped segment as [`Segment::Name`] and only a quoted one
/// as [`Segment::Key`]. Nothing is lost: the graph structure decides which path
/// positions are keyed-scope positions, so the loader needs no distinction
/// (033-243).
///
/// A [`Segment::Key`] holds the key's UNESCAPED content — what the quotes
/// delimited, not how it was spelled. Rendering re-derives the spelling:
/// [`CellPath::render`](crate::path::CellPath::render) writes a key bare iff it
/// is NAME-shaped AND is not one of the five reserved spellings, and
/// single-quoted otherwise, which is the canonical-quoting law (R41, 002-39).
/// An empty key is not NAME-shaped, so it renders as `''` rather than as
/// nothing; a key spelled `null` is NAME-shaped but reserved, so it renders
/// `'null'` and reads back as the key rather than as the keyword.
///
/// # Ordering
///
/// Derived `Ord` follows VARIANT DECLARATION ORDER, so every `Name` sorts
/// before every `Key`, and within a `Name` the text is compared before the
/// ordinal. A new variant declared before `Name` would silently reorder every
/// existing path; new variants go last, or `Ord` gets hand-written.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Segment {
    /// A declared identifier, with the ordinal suffix it may carry — the
    /// `gain` in `app.mixer.gain`, the `print:0` in `App.print:0.text`.
    Name {
        /// The NAME itself, without the `:N` suffix.
        text: String,
        /// The `:N` suffix's index, absent when the segment carries none.
        ordinal: Option<u64>,
    },
    /// A `repeat` scope's key, unescaped — the `42` in `App.rows.'42'.row`.
    Key(String),
}
