//! Test-only fixtures shared by `parser::parse` and `emitter::emit`.
//!
//! The two halves of the text form are tested against the SAME sources — the
//! parser asserts what a source parses to, the emitter asserts what it renders
//! back — so the sources belong to neither file. Keeping one copy is also what
//! keeps the hand-counted byte offsets below trustworthy: two copies of a
//! fixture are two chances for the counts to describe the other one.
//!
//! Test code is private, so S1's one-public-entity law does not reach it —
//! `text::lexer::scan::test_support` is the same exemption one folder down.

use crate::module::Module;
use crate::text::lexer::lex;
use crate::text::parser::parse;

/// The minimal module in canonical form.
///
/// Byte offsets, counted by hand: `module` 0..6, `App` 7..10, `{` 11,
/// `types` 15..20, `{}` 21..23, `graph` 26..31, `{}` 32..34,
/// `behaviors` 37..46, `{}` 47..49, `}` 50, trailing newline 51, end 52.
pub(super) const CANONICAL: &str = "module App {\n  types {}\n  graph {}\n  behaviors {}\n}\n";

/// The same module with no newline anywhere, not even a trailing one.
pub(super) const ONE_LINE: &str = "module App { types {} graph {} behaviors {} }";

/// The same module again, blank lines everywhere they are legal.
pub(super) const BLANK_LINES: &str =
    "\n\nmodule App {\n\n  types {}\n\n\n  graph {}\n\n  behaviors {}\n\n}\n\n";

/// One type row, canonical — what [`with_types`] builds for that entry.
///
/// Byte offsets counted by hand: line 1 is 0..13, `  types {` is 13..22 with
/// `types` at 15 and `{` at 21, and the entry line starts at 23 — four spaces,
/// then `%` at 27. Inside the entry: `Request` 28..35, `=` 36, `record` 38..44,
/// `{` 45, `text` 47..51, `:` 51, `str` 53..56, `}` 57, `size` 59..63, `8` 64,
/// `align` 66..71, `8` 72, newline 73. Then `  }` closes the section with `}`
/// at 76, `  graph {}` runs 78..88 with `graph` at 80, `  behaviors {}` runs
/// 89..103 with `behaviors` at 91, and the module's `}` is at 104 with the
/// trailing newline at 105.
pub(super) const WITH_TYPES: &str = "module App {\n  types {\n    %Request = record { text: str } size 8 align 8\n  }\n  graph {}\n  behaviors {}\n}\n";

/// Where a [`with_types`] entry's first byte lands: the four-space indent on
/// the section's first entry line, which starts at 23.
pub(super) const ENTRY: usize = 27;

/// A module whose `types` section holds exactly `entry`, on its own line.
///
/// Error spans are written as `ENTRY + <offset within the entry>`, which is the
/// same hand count as above with the boilerplate factored out.
pub(super) fn with_types(entry: &str) -> String {
    format!("module App {{\n  types {{\n    {entry}\n  }}\n  graph {{}}\n  behaviors {{}}\n}}\n")
}

/// `source` through the lexer and the parser, both of which must succeed.
pub(super) fn parsed(source: &str) -> Module {
    let tokens = lex(source).expect("source lexes");

    parse(tokens).expect("source parses")
}
