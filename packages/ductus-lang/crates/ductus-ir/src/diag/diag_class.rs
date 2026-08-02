//! The `DiagClass` registry: every way a Ductus program can be rejected.

/// The diagnostic registry: each way a Ductus program can be rejected, as a
/// stable class.
///
/// The ids are the contract. Conformance tests assert `id()` strings and never
/// message text, so wording can be improved freely (F3). The registry is open
/// by design (R28) — a newly discovered failure mode is an ordinary variant
/// addition, not a schema change.
///
/// `Hash` and `Ord` are derived so a class can key a map (counting, grouping)
/// and a diagnostic list can be sorted deterministically. Derived ordering is
/// *declaration order*: it carries no severity or importance meaning, and
/// nothing may read one into it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DiagClass {
    /// A behavior reference resolves to neither the module's behavior table nor
    /// the provider registry.
    UnresolvableBehavior,
    /// An input reference names a cell path that does not exist.
    UnknownInputPath,
    /// Two cells declare the same path.
    DuplicateCellPath,
    /// The construct is Ductus, but outside the slice M0 implements.
    OutsideM0Slice,
    /// A host write targets a cell that is not an input.
    WriteToNonInput,
    /// A host write arrives while a commit is in flight.
    WriteDuringCommit,
    /// A host write carries a value of the wrong type for its cell.
    TypeMismatchedWrite,
    /// A `"` string reaches end of input with no closing quote.
    UnterminatedString,
    /// A `'` path key reaches end of input with no closing quote.
    UnterminatedQuotedKey,
    /// A backslash escape names no escape the position admits.
    InvalidEscape,
    /// A character begins no token of the `.duir` inventory.
    InvalidCharacter,
    /// A `B@` handle's hexadecimal run is outside 1–8 digits.
    InvalidBid,
    /// A carriage return stands alone — not the `\r` of a `\r\n` pair.
    LoneCarriageReturn,
    /// A token is not what the grammar admits at its position.
    ///
    /// The single parse-error class — semantic legality classes belong to the
    /// loader (F4).
    UnexpectedToken,
    /// A STRUCTURAL integer literal does not fit the `u64` its position needs.
    ///
    /// Structural numbers — a type's `size` and `align`, an array's length,
    /// later a cell's `depth` and a stream's `capacity` — are converted at parse
    /// time, because their type is known from the position alone (R57). A
    /// negative or oversized spelling therefore fails HERE, at the literal,
    /// rather than surviving to the loader as text. TYPED values (`init`,
    /// `else`) are the other case and stay raw.
    IntOutOfRange,
    /// A type tag nests past the format's bound of 64 levels (033-255).
    ///
    /// Each bracketed constructor — a `pool_index`, `yielded`, `handle` or
    /// `portal` wrapper, a tuple position, an array element, a closure
    /// parameter or return — is one level. The bound is FORMAT LAW rather than
    /// implementation latitude: every conforming reader accepts every tag
    /// within it and rejects every tag beyond it, identically. So this is not a
    /// resource diagnostic that a larger stack would retire — it is a grammar
    /// error, reported at the constructor that exceeds the bound.
    NestingTooDeep,
}

impl DiagClass {
    /// The stable snake_case id, matching the corpus diagnostic-class style
    /// (R28). This — not the message — is what tests assert on.
    #[must_use]
    pub fn id(&self) -> &'static str {
        match self {
            DiagClass::UnresolvableBehavior => "unresolvable_behavior",
            DiagClass::UnknownInputPath => "unknown_input_path",
            DiagClass::DuplicateCellPath => "duplicate_cell_path",
            DiagClass::OutsideM0Slice => "outside_m0_slice",
            DiagClass::WriteToNonInput => "write_to_non_input",
            DiagClass::WriteDuringCommit => "write_during_commit",
            DiagClass::TypeMismatchedWrite => "type_mismatched_write",
            DiagClass::UnterminatedString => "unterminated_string",
            DiagClass::UnterminatedQuotedKey => "unterminated_quoted_key",
            DiagClass::InvalidEscape => "invalid_escape",
            DiagClass::InvalidCharacter => "invalid_character",
            DiagClass::InvalidBid => "invalid_bid",
            DiagClass::LoneCarriageReturn => "lone_carriage_return",
            DiagClass::UnexpectedToken => "unexpected_token",
            DiagClass::IntOutOfRange => "int_out_of_range",
            DiagClass::NestingTooDeep => "nesting_too_deep",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn every_class_reports_its_snake_case_id() {
        assert_eq!(
            DiagClass::UnresolvableBehavior.id(),
            "unresolvable_behavior"
        );
        assert_eq!(DiagClass::UnknownInputPath.id(), "unknown_input_path");
        assert_eq!(DiagClass::DuplicateCellPath.id(), "duplicate_cell_path");
        assert_eq!(DiagClass::OutsideM0Slice.id(), "outside_m0_slice");
        assert_eq!(DiagClass::WriteToNonInput.id(), "write_to_non_input");
        assert_eq!(DiagClass::WriteDuringCommit.id(), "write_during_commit");
        assert_eq!(DiagClass::TypeMismatchedWrite.id(), "type_mismatched_write");
        assert_eq!(DiagClass::UnterminatedString.id(), "unterminated_string");
        assert_eq!(
            DiagClass::UnterminatedQuotedKey.id(),
            "unterminated_quoted_key"
        );
        assert_eq!(DiagClass::InvalidEscape.id(), "invalid_escape");
        assert_eq!(DiagClass::InvalidCharacter.id(), "invalid_character");
        assert_eq!(DiagClass::InvalidBid.id(), "invalid_bid");
        assert_eq!(DiagClass::LoneCarriageReturn.id(), "lone_carriage_return");
        assert_eq!(DiagClass::UnexpectedToken.id(), "unexpected_token");
        assert_eq!(DiagClass::IntOutOfRange.id(), "int_out_of_range");
        assert_eq!(DiagClass::NestingTooDeep.id(), "nesting_too_deep");
    }

    // What the `Hash` and `Ord` derives are for: grouping by class, and a
    // deterministic sort. The sort order is declaration order, nothing more.
    #[test]
    fn classes_key_a_map_and_sort_deterministically() {
        let mut counts = HashMap::new();
        *counts.entry(DiagClass::DuplicateCellPath).or_insert(0) += 1;
        *counts.entry(DiagClass::DuplicateCellPath).or_insert(0) += 1;
        *counts.entry(DiagClass::OutsideM0Slice).or_insert(0) += 1;

        assert_eq!(counts.get(&DiagClass::DuplicateCellPath), Some(&2));
        assert_eq!(counts.get(&DiagClass::WriteToNonInput), None);

        let mut classes = [
            DiagClass::TypeMismatchedWrite,
            DiagClass::UnresolvableBehavior,
            DiagClass::OutsideM0Slice,
        ];
        classes.sort();
        let ids: Vec<&str> = classes.iter().map(DiagClass::id).collect();

        assert_eq!(
            ids,
            [
                "unresolvable_behavior",
                "outside_m0_slice",
                "type_mismatched_write"
            ]
        );
    }
}
