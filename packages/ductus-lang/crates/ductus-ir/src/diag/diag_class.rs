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
/// nothing may read one into it — which is also why new variants APPEND at the
/// end rather than sitting beside their family: reordering would silently
/// change every stored sort, for a grouping the order never meant.
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
    /// One of the five reserved spellings stands where a `NAME` is required
    /// (002-39, 033-259).
    ///
    /// `null`, `true`, `false`, `nan` and `inf` are reserved language-wide, so
    /// no declaration of any kind may bear one as its name — a module name, a
    /// type id, a field name, a variant name, a binding name and a path's name
    /// segment all reject them. Its own class rather than `UnexpectedToken`
    /// because the token IS word-shaped and the position DOES want a word: what
    /// is wrong is the spelling, and the message a reader needs says so.
    ///
    /// A quoted KEY segment is untouched — `App.rows.'null'.render_row` is data
    /// and stays legal (033-243).
    ReservedWord,
    /// The derived dependency graph contains a cycle.
    ///
    /// A load error (R21) — nothing in M0 exists to break a cycle, so the
    /// loader rejects the module, reporting the cycle's member paths in the
    /// message.
    DerivedCycle,
    /// A cell's `init` literal does not convert under the cell's declared type.
    ///
    /// The loader converts TYPED values under the position's type (R57), and
    /// this is that conversion's failure. The write-time sibling is
    /// `type_mismatched_write`.
    TypeMismatchedInit,
    /// Two scope entries declare one path.
    ///
    /// ILL-FORMED IR: a conforming compiler cannot produce it, because a scope
    /// is one region of the tree and the graph section names it once. The
    /// SECOND declaration is the one rejected — the first stands, the repeat
    /// is the error. The cell-side sibling is `duplicate_cell_path`.
    DuplicateScopePath,
    /// An entry's path does not extend its enclosing scope's path as a proper
    /// prefix.
    ///
    /// The block structure and the paths carry ONE hierarchy, not two: a cell
    /// written inside `scope App.panel` lives at `App.panel.…`, and a path that
    /// wanders elsewhere would make the braces and the dots disagree about
    /// where the cell is. A `gate` entry answers to the same law — its path
    /// joins the one namespace, so it names a place in the same tree.
    ///
    /// Intermediate segments need no declared scope — depth is FREE, so
    /// `scope App { input App.print.text }` is legal with no `App.print` scope
    /// anywhere. Only the prefix is law.
    CellOutsideScope,
    /// A scope path's first segment is not the module's name.
    ///
    /// Every path in a module is anchored at the module root (§15.4.1.1), so
    /// `module App` holds `App`, `App.panel`, `App.panel.row` — never `Foo`.
    /// Cells inherit the anchor from their scope through `cell_outside_scope`
    /// and need no check of their own.
    UnanchoredScopePath,
    /// One path names both a scope and a cell.
    ///
    /// Scope paths and cell paths share ONE identity namespace: a path is a
    /// place in the tree, and it is either a region or a cell, never both. The
    /// SECOND declaration is the one rejected, whichever kind it is.
    ScopeCellCollision,
    /// A history depth means nothing at runtime: `depth 0`, a `lookback` of
    /// `0`, or a `fallbacks` key outside `1 ..= depth`.
    ///
    /// `depth` counts a recurrent's history SLOTS and a deferred's latency in
    /// commits, and both are positive by construction: a zero-slot ring names
    /// no parameter and allocates nothing, and a zero-commit latency is not a
    /// delay (M1.md §4.9, C-7/C-8/C-24). 016-282's "`derived T` IS the N = 0
    /// case" is a statement about the language's type ladder, not a licence for
    /// a `.duir` line that means nothing at runtime. A key past the ring's
    /// depth is the same error seen from the other end: it addresses a slot the
    /// cell does not have.
    InvalidHistoryDepth,
    /// A `recurrent` carries BOTH `init` and `fallbacks`.
    ///
    /// They are one clause in two spellings — `init v` says every depth falls
    /// back to `v`, `fallbacks` says them one by one — so carrying both would
    /// be two answers to one question (033-268, R71: one meaning, one
    /// spelling).
    ConflictingHistoryFallbacks,
    /// A `fallbacks` map names one depth twice.
    ///
    /// The clause is a MAP from depth `k` to the value `.past(k, …)` yields
    /// before `k` advances exist; a repeated key would give one slot two seeds.
    DuplicateFallbackKey,
    /// A `recurrent`'s history has a depth no fallback answers for.
    ///
    /// Every `k` in `1 ..= depth` is read by the behavior's `pastK` parameter
    /// before the ring has advanced that far, so every `k` needs a seed —
    /// spelled one by one with `fallbacks` or uniformly with `init`. A map that
    /// misses a depth (a `fallbacks` clause with a hole, or a recurrent
    /// carrying neither clause) leaves a read with no value to answer with, and
    /// the runtime does not invent one.
    MissingFallbackKey,
    /// A `lookback` key names a cell that is not among the reading cell's own
    /// `inputs`.
    ///
    /// The clause declares per-INPUT history depths, and the behavior ABI's
    /// argument order is defined over the `inputs` list (R71, §15.4.1's
    /// `input_lookback_map`): a key outside that list names history for a cell
    /// the recurrent does not read. A key naming no cell at all is
    /// `unknown_input_path` instead — the same diagnostic any unknown path
    /// gets.
    LookbackNotAnInput,
    /// A `deferred` cell names itself among its own `inputs`.
    ///
    /// §13.2.12 is direct: "its expression may not read the cell it declares".
    /// A `deferred` is a LATENCY, never a cycle device (§13.11.4) — cycle
    /// breaking remains the recurrent's role — so the delay does not make the
    /// self-read well-founded. Construction settles it: a deferred's startup
    /// value comes from a synchronous evaluation like a derived's (016-298),
    /// and a self-reading one would read the slot it is at that moment
    /// computing.
    DeferredSelfReference,
    /// Two gate entries declare one path.
    ///
    /// The scope/cell sibling of `duplicate_scope_path` and
    /// `duplicate_cell_path`: gate paths join the SAME identity namespace
    /// (§15.4.1's one tree), so a path names one gate and the SECOND
    /// declaration is the one rejected.
    DuplicateGatePath,
    /// A gate's `guards` entry names neither a declared cell nor a declared
    /// gate.
    ///
    /// §15.4.1: a gate carries "a `guards` list naming the gated instances it
    /// controls (each by path)". What a gate may guard in M1's slice is a cell
    /// or another gate, so a path naming neither guards nothing.
    UnknownGuardPath,
    /// A gate's `gate_parent` names no declared gate.
    ///
    /// §15.4.1: "`gate_parent` — the `id` of the nearest enclosing gate, or
    /// `null` if none". The clause is the nesting link effective activation is
    /// composed along (§13.9.7), so it must name a gate — a cell path or an
    /// undeclared one leaves the chain with nothing to climb.
    UnknownGateParent,
    /// The `gate_parent` chain closes a loop.
    ///
    /// Effective activation is "its own gate AND every ancestor gate on the
    /// path from the root" (§13.9.7), which is only computable on a chain that
    /// terminates. The corpus states no acyclicity rule (C-47); the loader
    /// enforces one, because a cycle has no root to compose from.
    GateParentCycle,
    /// A cell's `gate PATH` clause names no declared gate.
    ///
    /// IR_GRAMMAR §3's `('gate' PATH)?` is a back-reference to a `gate` entry.
    /// A cell pointing at a cell — or at nothing — declares no activation
    /// condition, so the clause has nothing to mean.
    UnknownGateReference,
    /// A gate's `guards` list and the constructs' own gate clauses disagree.
    ///
    /// §15.4.1 calls the `guards` list "the materialized inverse" of the
    /// construct's own clause (`gate PATH` on a cell, `gate_parent PATH` on a
    /// gate). An inverse is a RESTATEMENT, so the two must name exactly the same
    /// set, and a mismatch in EITHER direction is this class: a construct listed
    /// in `guards` whose own clause names another gate or none, and a construct
    /// whose own clause names a gate that does not list it. Two gates both
    /// listing one construct is the same failure seen once per gate — at most
    /// one of them can be the construct's own answer, and §13.9.7's effective
    /// activation is defined over exactly one.
    GuardsInverseMismatch,
    /// A `recurrent` cell names itself among its own `inputs`.
    ///
    /// 016-51: prior self-values are readable only through `.previous`/`.past`,
    /// which in the IR ride `depth` and `fallbacks` — never an `inputs` entry.
    /// So a recurrent listing its own path among `inputs` asks for its own
    /// current VALUE, which no source expression can spell: §13.2.4.1 denies
    /// that value even to the cell's own history accessors — *"the value being
    /// computed in the current commit is not visible through these accessors
    /// during that same commit"*.
    ///
    /// The sibling of `deferred_self_reference`. Left standing, the self-edge
    /// also makes the cell its own staleness trigger under the advance-root law:
    /// its advance roots the next commit, which re-evaluates it, which advances
    /// it again — once per commit forever, with no host activity at all.
    RecurrentSelfReference,
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
            DiagClass::ReservedWord => "reserved_word",
            DiagClass::DerivedCycle => "derived_cycle",
            DiagClass::TypeMismatchedInit => "type_mismatched_init",
            DiagClass::DuplicateScopePath => "duplicate_scope_path",
            DiagClass::CellOutsideScope => "cell_outside_scope",
            DiagClass::UnanchoredScopePath => "unanchored_scope_path",
            DiagClass::ScopeCellCollision => "scope_cell_collision",
            DiagClass::InvalidHistoryDepth => "invalid_history_depth",
            DiagClass::ConflictingHistoryFallbacks => "conflicting_history_fallbacks",
            DiagClass::DuplicateFallbackKey => "duplicate_fallback_key",
            DiagClass::MissingFallbackKey => "missing_fallback_key",
            DiagClass::LookbackNotAnInput => "lookback_not_an_input",
            DiagClass::DeferredSelfReference => "deferred_self_reference",
            DiagClass::DuplicateGatePath => "duplicate_gate_path",
            DiagClass::UnknownGuardPath => "unknown_guard_path",
            DiagClass::UnknownGateParent => "unknown_gate_parent",
            DiagClass::GateParentCycle => "gate_parent_cycle",
            DiagClass::UnknownGateReference => "unknown_gate_reference",
            DiagClass::GuardsInverseMismatch => "guards_inverse_mismatch",
            DiagClass::RecurrentSelfReference => "recurrent_self_reference",
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
        assert_eq!(DiagClass::ReservedWord.id(), "reserved_word");
        assert_eq!(DiagClass::DerivedCycle.id(), "derived_cycle");
        assert_eq!(DiagClass::TypeMismatchedInit.id(), "type_mismatched_init");
        assert_eq!(DiagClass::DuplicateScopePath.id(), "duplicate_scope_path");
        assert_eq!(DiagClass::CellOutsideScope.id(), "cell_outside_scope");
        assert_eq!(DiagClass::UnanchoredScopePath.id(), "unanchored_scope_path");
        assert_eq!(DiagClass::ScopeCellCollision.id(), "scope_cell_collision");
        assert_eq!(DiagClass::InvalidHistoryDepth.id(), "invalid_history_depth");
        assert_eq!(
            DiagClass::ConflictingHistoryFallbacks.id(),
            "conflicting_history_fallbacks"
        );
        assert_eq!(
            DiagClass::DuplicateFallbackKey.id(),
            "duplicate_fallback_key"
        );
        assert_eq!(DiagClass::MissingFallbackKey.id(), "missing_fallback_key");
        assert_eq!(DiagClass::LookbackNotAnInput.id(), "lookback_not_an_input");
        assert_eq!(
            DiagClass::DeferredSelfReference.id(),
            "deferred_self_reference"
        );
        assert_eq!(DiagClass::DuplicateGatePath.id(), "duplicate_gate_path");
        assert_eq!(DiagClass::UnknownGuardPath.id(), "unknown_guard_path");
        assert_eq!(DiagClass::UnknownGateParent.id(), "unknown_gate_parent");
        assert_eq!(DiagClass::GateParentCycle.id(), "gate_parent_cycle");
        assert_eq!(
            DiagClass::UnknownGateReference.id(),
            "unknown_gate_reference"
        );
        assert_eq!(
            DiagClass::GuardsInverseMismatch.id(),
            "guards_inverse_mismatch"
        );
        assert_eq!(
            DiagClass::RecurrentSelfReference.id(),
            "recurrent_self_reference"
        );
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
