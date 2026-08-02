# Code style — binding on every implementer, reviewer, and remediation brief

*Owner-ruled 2026-08-01 (M0.md R30/R31). Every implementer brief cites this file; reviewers
verify against it. rustfmt does NOT enforce these — review does.*

## S1. File organization: one public entity per file, file named after it; mod.rs is a barrel (R30, sharpened by R52 — owner HARD REJECT of increment 3's first build, 2026-08-02)

- A file holds ONE public entity — a type/enum/trait OR a file-level function — with its
  `impl`s and its `#[cfg(test)]` tests. This binds functions exactly as it binds types:
  a file with several public file-level functions is the same violation as one with
  several public types. No exceptions — except PRIVATE functions and entities, which live
  with their user (or, when shared by sibling files, in a folder-private support file).
- THE FILE NAME IS THE ENTITY NAME, lower_snake_cased, otherwise identical: `DiagClass` →
  `diag_class.rs`, `TokenKind` → `token_kind.rs`, `fn lex` → `lex.rs`. Never a role name
  (`driver.rs`, `class.rs`, `kind.rs` are all violations — owner: "I hate when file name
  is different than entity name").
- Same-concern files group under a folder module. Do not nest deeper than the concern
  needs. `mod.rs` contains ONLY module declarations and re-exports (barrel entries) plus
  the `//!` doc — zero code.
- Stutter (`value/value.rs`, `token/token.rs`) is accepted where the entity IS the
  concern, with the `#[expect(clippy::module_inception)]` pattern.
- Stated reading of the private-exception (owner may strike): "private" means not visible
  outside the concern folder — module-private and `pub(super)`-within-the-folder plumbing
  is exempt and may share a support file; anything visible beyond the folder follows the
  one-entity-one-file law.

```
src/diag/
  mod.rs          // //! doc + pub mod …; + pub use …
  span.rs         // pub struct Span
  diag_class.rs   // pub enum DiagClass
  diagnostic.rs   // pub struct Diagnostic
src/text/lexer/scan/
  mod.rs          // barrel
  ident.rs        // pub(crate) fn ident
  number.rs       // pub(crate) fn number  … one public recognizer per file
```

Rationale (owner): many pub entities in one file get harder to read, reason about, and
refactor as they grow; a file name that differs from its entity name defeats scanning.

## S2. Blank lines around block statements (R31)

A blank line BEFORE and AFTER every block statement — `if`, `for`, `while`, `loop`,
`match`, and block expressions used as statements — always. Stated interpretation: the
blank line separates the block from ADJACENT statements; no blank line is required
between a block and the `{`/`}` of its enclosing scope when it is the first or last
statement there.

```rust
let mut rendered = String::new();

for (index, segment) in self.0.iter().enumerate() {
    …
}

rendered
```

rustfmt preserves these blank lines (blank_lines_upper_bound default) but will not ADD
them — implementers write them, reviewers check them.

## S3. Practiced conventions (inherited from ruled increments)

- Doc comments on public items; inline comments only for non-obvious constraints.
- Spec-anchor comments (`§x.y.z`) on semantics-bearing functions where a corpus anchor
  exists (C1 practice).
- Unit tests inline in the entity's own file (`#[cfg(test)] mod tests`); witness tests in
  ductus-conformance per corpus/README.md (C2 practice, R25).
- No `unsafe` (C3 lean; unruled as a lint, honored as practice).
- Comment density low; no comments narrating what the next line does.
