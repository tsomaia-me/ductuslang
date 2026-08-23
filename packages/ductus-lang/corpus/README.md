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
`transaction_batch` and `islands` each pair a `.duir` module with a `.trace`
golden, byte-compared by `crates/ductus-conformance/tests/commit.rs`.

Two kinds of witness carry the `.duir` half alone:

- the grammar witnesses written before the engine — `module_header`,
  `types_table`, `scopes`, `cells`, `graph_entries` — which assert parsing and
  round-trip byte-stability only;
- `trap_abort`, whose commit ends in an abort (D8, §13.13.1): past a trap there
  is no trace to observe, so its test asserts the panic instead of a golden.

Goldens are HAND-DERIVED from the semantics contract (SPEC §13.10.2 as pinned
in M0.md §4), never captured from a run — a captured golden would assert only
that the engine does what the engine does. Each witness test's doc comment
carries the derivation, block by block.
