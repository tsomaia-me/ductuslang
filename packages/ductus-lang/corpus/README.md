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

The test does four things, in order:

1. Loads `<name>.duir` into the runtime.
2. Registers the named stubs it needs from the shared stub library.
3. Drives the module: writes, then commits.
4. Byte-compares the emitted trace against `<name>.trace`.

## Naming

`snake_case` throughout. A witness test's fn name equals its `.duir` file
stem, so a witness is grep-able by one word across corpus and tests. The rule
binds witness tests only; non-witness tests such as `smoke.rs` are exempt.

## Path resolution

Witness code resolves corpus files relative to the crate manifest —
`concat!(env!("CARGO_MANIFEST_DIR"), "/../../corpus/duir/m0/<name>.duir")` —
never relative to the process cwd.

## Status

The trace format is defined at the commit-engine increment (increment 12).
Witnesses added before it carry the `.duir` half alone and assert parsing and
round-trip byte-stability only; goldens and byte-comparison start there.
