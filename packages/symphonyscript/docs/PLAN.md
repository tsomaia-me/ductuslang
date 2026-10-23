# SymphonyScript Plan

The implementation proceeds in phases. A phase ends when its exit criteria pass and the owner approves the result. Each phase opens with a plan of its increments, which the owner approves before any code is written.

## Phase 1: Workspace and core data

The workspace, `symphonyscript-core`'s data types, and the fixtures.

- Cargo workspace with the five crates of ARCHITECTURE §1.
- `Value`, `Ty`, handles, and the vocabulary builder with `build()` validation and the stamp.
- The Program types and a `dump` printer.
- `symphonyscript-fixtures` with the example vocabulary of SPEC §1.3.

Exit:
- A test for each error `build()` reports.
- The fixture vocabulary builds, and its stamp is identical across runs and targets.
- A hand-built Program dumps to a golden text.
- `symphonyscript-core` builds for a `no_std` target.

## Phase 2: Lexer and parser

- Layout, literals, keywords and unit suffixes (SPEC §2).
- The lossless syntax tree with its typed view.
- Error recovery.

Exit:
- A test extracts every example from SPEC.md and parses it.
- Every production in SPEC.md has parse tests.
- Parsing any source and printing the tree reproduces it byte for byte.
- Broken sources still yield a tree, with diagnostics at the right spans.

## Phase 3: Semantics

Collect, expand, resolve, infer and check (ARCHITECTURE §7, stages 3 to 7).

Exit:
- For every rule in SPEC.md that names an error, a test rejects a program with that error's stable code.
- Every SPEC example that declares all the names it uses compiles against the fixture vocabulary.
- Diagnostics for expanded patterns point at the use and the pattern line.

## Phase 4: Lowering and runtime

Lowering, the runtime, `eval`, and the `Consumer` trait.

Exit:
- Commit traces match golden traces for the fixture projects.
- A counting allocator shows `commit` allocates nothing.
- Golden values for float built-ins, saturation, division by zero, clamped indexing and `rand` match bit for bit.
- Change reports carry element indices for array attributes.

## Phase 5: Reload

Exit:
- A test for each rule of SPEC §13.
- Inserting one anonymous note into a playing clip reports exactly one addition.

## Phase 6: Generated Rust

Exit:
- Every fixture project compiled through the library and through `codegen` gives equal Programs and equal commit traces.
- A generated Program builds for a `no_std` target.

## Phase 7: Tooling

The tool, the watch loop, the language server and the VS Code extension.

Exit:
- End-to-end tests for `new`, `check` and `dump`, and for single-file mode.
- Language server tests for diagnostics, completion, hover, definition, references and rename on the fixture projects.
- The extension starts the language server for a sample project.

## Phase 8: Kernel adapter

Exit:
- A fixture project loads into synaptic-kernel. A consumer thread entering at the sentinel reaches every instance and wire.
- An input write and a commit become visible to the consumer thread, and the adapter publishes only when a meta-plane value changed.
- A reload keeps the serials of matched instances.

## Order

Phases 1 to 5 run in order. Phases 6, 7 and 8 each need phase 5 and are independent of each other.
