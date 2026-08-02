//! Witness tests for the module construct — the `module` header and its three
//! section shells (corpus/README.md, R25).
//!
//! One `#[test]` fn per witness, named after the witness's file stem, so a
//! witness is grep-able by one word across the corpus and the tests.

use std::fs;

use ductus_ir::text::emitter::emit;
use ductus_ir::text::lexer::lex;
use ductus_ir::text::parser::parse;

/// Resolved against the CRATE MANIFEST, never the process cwd
/// (corpus/README.md).
const MODULE_HEADER: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../corpus/duir/m0/module_header.duir"
);

/// Resolved against the CRATE MANIFEST, never the process cwd
/// (corpus/README.md).
const TYPES_TABLE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../corpus/duir/m0/types_table.duir"
);

/// The minimal module: a header, three empty sections, one trailing newline.
///
/// The trace format arrives at increment 12, so this witness carries its
/// `.duir` half alone and asserts what a pre-goldens witness can assert —
/// that the file lexes, parses, and emits back to its own bytes. Because the
/// emitter IS the canonicalizer (R54), byte equality here is also the claim
/// that the hand-written file is canonical: a corpus file that drifts out of
/// canonical form fails this test rather than quietly teaching a second
/// spelling.
#[test]
fn module_header() {
    let source = fs::read_to_string(MODULE_HEADER).expect("the witness file is readable");
    let tokens = lex(&source).expect("the witness lexes");
    let module = parse(tokens).expect("the witness parses");

    assert_eq!(emit(&module), source);
}

/// The `types` table: every `layout` and every `type_tag` alternative the
/// grammar admits, in one module.
///
/// The empty `graph {}` and `behaviors {}` are the same latitude the minimal
/// witness takes (FND-16 is open until the scope row), so this file exercises
/// the type table alone. Byte equality is again the claim that the file is
/// CANONICAL — a hand-written row that spaces a tag differently from the
/// emitter fails here rather than teaching a second spelling.
#[test]
fn types_table() {
    let source = fs::read_to_string(TYPES_TABLE).expect("the witness file is readable");
    let tokens = lex(&source).expect("the witness lexes");
    let module = parse(tokens).expect("the witness parses");

    assert_eq!(emit(&module), source);
}
