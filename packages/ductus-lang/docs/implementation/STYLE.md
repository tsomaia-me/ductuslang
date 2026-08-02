# Code style — binding on every implementer, reviewer, and remediation brief

*Owner-ruled 2026-08-01 (M0.md R30/R31). Every implementer brief cites this file; reviewers
verify against it. rustfmt does NOT enforce these — review does.*

## S1. File organization: one pub entity per file; mod.rs is a barrel (R30)

- A file holds ONE primary `pub` entity — the type/enum/trait, its `impl`s, and its
  `#[cfg(test)]` tests. Small PRIVATE helpers used only by that entity live with it.
- Similar entities group under a folder (separation of concerns — a folder scan tells you
  what lives where). Do not nest deeper than the concern needs; current depth
  (`crate/src/<concern>/<entity>.rs`) is the model.
- `mod.rs` contains ONLY module declarations and re-exports (barrel entries) — zero code,
  zero declarations. The module doc comment (`//!`) stays in the mod.rs.

```
src/diag/
  mod.rs         // //! doc + pub mod span; pub mod class; pub mod diagnostic; + pub use …
  span.rs        // pub struct Span + impl + tests
  class.rs       // pub enum DiagClass + impl + tests
  diagnostic.rs  // pub struct Diagnostic + tests
```

Rationale (owner): many pub entities in one file get harder to read, reason about, and
refactor as they grow.

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
