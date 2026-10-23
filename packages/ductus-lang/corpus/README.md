# Corpus

Hand-written IR modules and their expected commit traces — the input side of
the conformance harness; the harness lives in `crates/ductus-conformance`.

## Witness convention

Each M0 witness is a pair of files plus one test function:

- `corpus/duir/m0/<name>.duir` — a hand-written `.duir` IR module.
- `corpus/duir/m0/<name>.trace` — the expected commit trace (golden).

Witness tests live in per-construct files, not one file per witness:
`crates/ductus-conformance/tests/<construct>.rs` — `cells.rs`, `commit.rs`,
and so on — each holding one `#[test]` fn per witness it covers.

The test makes five claims, in order:

1. `<name>.duir` lexes, parses and emits back to its own bytes —
   `emit(parse(source)) == source`. The emitter IS the canonicalizer (R54), so
   byte equality is also the claim that the hand-written file is canonical.
2. The module loads into the runtime under the stubs the witness registers.
3. A scripted sequence of writes and commits drives it.
4. The trace heard is byte-identical to `<name>.trace`.
5. The SAME script over the same module built PROGRAMMATICALLY through the
   `ductus-ir` Rust API — no text anywhere — produces a byte-identical trace
   (M0.md §6's dual-construction track).

`trap_abort` is the one exception: past an abort there is no trace to observe
(D8, §13.13.1), so it asserts the panic instead of claims 4 and 5.

Where a claim-2 stub lives follows its reach: a stub used by two or more
witnesses (or by the property tests) earns a file in the shared stub library,
`crates/ductus-conformance/src/stub_library/`; a single-use stub — a witness's
gating fixture, its trapping fixture — is written INLINE in the witness test,
beside the golden it explains. The library holds vocabulary, not every closure.

## Naming

`snake_case` throughout. A witness test's fn name equals its `.duir` file
stem, so a witness is grep-able by one word across corpus and tests. The rule
binds witness tests only; non-witness tests such as `smoke.rs` are exempt.

## Path resolution

Witness code resolves corpus files relative to the crate manifest —
`concat!(env!("CARGO_MANIFEST_DIR"), "/../../corpus/duir/m0/<name>.duir")` —
never relative to the process cwd.

## Status

The trace format is defined — R24's event schema, landed with the commit
engine — so the commit witnesses carry both halves: `single_derived`,
`empty_commit`, `chain`, `diamond`, `value_gating`, `tiebreak_order`,
`transaction_batch`, `islands` and `rank_interleaved` each pair a `.duir`
module with a `.trace` golden, byte-compared by
`crates/ductus-conformance/tests/commit.rs`.

Two kinds of witness carry the `.duir` half alone:

- the grammar witnesses written before the engine — `module_header`,
  `types_table`, `scopes`, `cells`, `graph_entries`, `recurrent_clauses` —
  which assert parsing and round-trip byte-stability only. A grammar witness
  is not required to LOAD: it may name a behavior nobody defines or a gate no
  entry declares, because the parser over-admits by design (F4, R21) and the
  file's whole claim is about the text form;
- `trap_abort`, whose commit ends in an abort (D8, §13.13.1): past a trap there
  is no trace to observe, so its test asserts the panic instead of a golden.

Goldens are HAND-DERIVED from the semantics contract (SPEC §13.10.2 as pinned
in M0.md §4), never captured from a run — a captured golden would assert only
that the engine does what the engine does. Each witness test's doc comment
carries the derivation, block by block.

### M1 — `corpus/duir/m1/`

M1's witnesses live in their own directory, one per milestone, resolved by the
same manifest-anchored rule. Everything above binds them unchanged: the pair
convention, the five claims, the naming rule, and the hand-derived discipline —
with the contract they are derived from being §13.10.2 as pinned in M1.md §4,
whose step list gains step 4½.

The `recurrent` family — `depth1_accumulator`, `first_evaluation`,
`lockstep_pair`, `self_cycle`, `depth3_shift`, `equal_value_advance`,
`skipped_commit`, `lookback_read`, `fallback_map` and `nan_history` — each pairs a `.duir`
module with a `.trace` golden, byte-compared by
`crates/ductus-conformance/tests/recurrent.rs`. Two things about those goldens
are worth knowing before reading one. A recurrent's FALLBACKS are read at
construction, which is untraced, so a fallback reaches the trace only after it
has ridden the ring into an evaluation — `first_evaluation` therefore asserts
the constructed value through the host read surface as well. And an advance is
written `eval <path> -> <value>` like any other value production (R75), so what
marks a recurrent in a block is what its numbers prove: the ring shifts exactly
once per evaluation and never otherwise (R72 + R77).

One block shape in that family surprises a cold reader and is not a mistake:
`self_cycle` ends with a commit that has `eval` lines and NO `dirty` line. A
recurrent's advance is a silent dirty root at the next commit (M1.md §4.12), so a
cycle closed through a recurrent keeps turning after the writes stop — a delay
loop with no input still ticks.

The `deferred` family — `latency1`, `latency3`, `in_flight_reads`,
`landing_dirty_root`, `pipeline_full_rate`, `landing_gating`,
`dispatch_gating` and `chained_deferred` — each pairs a `.duir` module with a
`.trace` golden, byte-compared by
`crates/ductus-conformance/tests/deferred.rs`. A deferred's two ends sit at
opposite ends of a commit, so its goldens read as `dirty` lines the host never
staged (the landing, R75/R76) and as an absence where the dispatch happened
(step 4½ publishes no value, so it records none).

The `gate` family — `closed_at_startup`, `open_snap`, `close_freeze`,
`live_inputs_frozen_derived`, `nested_gates`, `reopen_reset`,
`gate_recurrent_loss`, `gate_deferred_cancel`, `predicate_stable`,
`rank_interleaved_roots` and `self_conditional_gate` — each
pairs a `.duir` module with a `.trace` golden, byte-compared by
`crates/ductus-conformance/tests/gates.rs`. Two shapes recur in those goldens
and are worth knowing before reading one. A gate move costs TWO commit blocks,
because the predicate reads prior-commit values (R73, step 1a): the block
carrying the `dirty` line on the predicate's input is never the block carrying
the `gate <path> open|close` line. And a frozen subtree is recorded as an
ABSENCE — activation is checked before staleness (RW-9), so the assertion is
usually the `eval` line that is missing beside a `dirty` line that is not.

`self_conditional_gate` is the family's newest member and the one whose
existence was ruled rather than derived: its predicate reads a cell its own gate
guards, which the owner ruled LEGAL on 2026-08-24 (SPEC §13.9.6, 022-36..39).
Both shapes above are visible in it at once — the transition trails the guarded
change by one commit, and the gate then LATCHES, because the value its predicate
reads is the value it froze.
