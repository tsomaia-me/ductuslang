//! Witness tests for the graph section — scopes, and the five entry kinds a
//! scope body holds (corpus/README.md, R25).
//!
//! One `#[test]` fn per witness, named after the witness's file stem, so a
//! witness is grep-able by one word across the corpus and the tests.
//!
//! Each witness asserts what a pre-goldens witness can assert: the file lexes,
//! parses, and emits back to its own bytes. Because the emitter IS the
//! canonicalizer (R54), byte equality is also the claim that the hand-written
//! file is CANONICAL — a corpus line that spaces a clause differently from the
//! emitter fails here rather than quietly teaching a second spelling.

use std::fs;

use ductus_ir::text::emitter::emit;
use ductus_ir::text::lexer::lex;
use ductus_ir::text::parser::parse;

/// Resolved against the CRATE MANIFEST, never the process cwd
/// (corpus/README.md).
const SCOPES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../corpus/duir/m0/scopes.duir"
);

/// Resolved against the CRATE MANIFEST, never the process cwd
/// (corpus/README.md).
const CELLS: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../corpus/duir/m0/cells.duir"
);

/// Resolved against the CRATE MANIFEST, never the process cwd
/// (corpus/README.md).
const GRAPH_ENTRIES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../corpus/duir/m0/graph_entries.duir"
);

/// Resolved against the CRATE MANIFEST, never the process cwd
/// (corpus/README.md).
const RECURRENT_CLAUSES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../corpus/duir/m0/recurrent_clauses.duir"
);

/// Scopes: flat under the section, nesting carried by their PATHs.
///
/// `App`, `App.mixer` and `App.panel` are three siblings here and a tree by
/// name (§15.4.1.1). The first carries non-empty `exposes` / `effects` and a
/// `reset_on_reopen` set — the payload-carrying position of that keyword
/// (033-215) — the second an empty pair of mandatory sets written `[]`, and
/// the third an empty BODY, which closes up on its own line.
#[test]
fn scopes() {
    round_trips(SCOPES);
}

/// Cells: all five kinds, and every clause the `cell` production admits, in
/// the production's order.
///
/// The `init` values cover an integer, a float and a string carrying an
/// escape; the `fold` row carries a `gate` clause, a `combiner`, a negative
/// `else`, and a `members` set naming all three membership drivers — three of
/// them through quoted keys, which is where the whole key-quoting law is
/// witnessed: `'42'` because a numeric key is not NAME-shaped (R41), `'it\'s'`
/// because a key's escape set is the string form's (R63), and `'null'` because
/// a key spelled by one of the five reserved words must stay quoted or it would
/// read back as the keyword (002-39, 033-243).
#[test]
fn cells() {
    round_trips(CELLS);
}

/// The other four entry kinds: `gate`, `wire`, `effect` and `stream`.
///
/// A wire to a PATH and a wire to `null`; `attrs` bindings carrying both
/// target kinds with all three `@`-provenance markers glued on (033-252) — a
/// value target under `@bridge` and under `@constant_wrap`, a path target
/// under `@bound`; an effect with
/// `desired` and `observed`; and two streams, one per policy, the second
/// carrying a `history` and a `lookback` map whose first key bears an ordinal
/// — the shape R43's last-`':' INT` rule exists for.
#[test]
fn graph_entries() {
    round_trips(GRAPH_ENTRIES);
}

/// The two history clauses a `recurrent` cell carries — `lookback` and
/// `fallbacks` (033-266 / 033-267).
///
/// `App.total` is the corpus accumulator; `App.fib` is the two-slot recurrence
/// with a distinct seed per slot, and carries NO `inputs` — the empty set is
/// the clause's absence, so `inputs []` is a spelling this file may not use.
/// `App.avg` reads three commits back into one input while keeping its own
/// depth 2, and shows the two clauses standing on the same line either side of
/// `reset_on_reopen`. `App.blend` is the ordinal-bearing lookback key — the
/// shape R43's last-`':' INT` rule exists for — beside `init nan`, which is
/// the uniform spelling of `fallbacks` (033-268).
///
/// It is a GRAMMAR witness: the behaviors it names and the gate it points at
/// are undeclared, and the file's only claim is that the text form reads and
/// writes back.
#[test]
fn recurrent_clauses() {
    round_trips(RECURRENT_CLAUSES);
}

/// The witness assertion: the file lexes, parses, and emits back to its own
/// bytes.
fn round_trips(witness: &str) {
    let source = fs::read_to_string(witness).expect("the witness file is readable");
    let tokens = lex(&source).expect("the witness lexes");
    let module = parse(tokens).expect("the witness parses");

    assert_eq!(emit(&module), source);
}
