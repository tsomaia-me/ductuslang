# SymphonyScript Plan

The implementation runs in five milestones. The owner approves this plan once; the head then drives the milestones in order without stopping between them and reports each one in the head notes.

## How the work runs

### Roles

- **Owner:** decides direction.
- **Head:** the session that drives this plan. It briefs implementers and reviewers, sorts every gap, edits the documents, decides what the decision rules let it decide, and keeps the head notes in `.memory/symphonyscript/notes.md`.
- **Implementer:** one agent per milestone, Opus at effort `high`.
- **Reviewer:** a fresh agent per cold review, Opus at effort `xhigh` for M2, M3 and M5 and `high` for M1 and M4. A Fable reviewer joins a review when the owner launches one.

### Decisions

- A gap or contradiction with a single answer consistent with the documents: the head decides it, edits the documents and records it in the head notes.
- A question the documents and the owner's rulings answer, whose best-practice answer stands out among the alternatives and which is cheap to reverse: the head decides it and records it in the head notes, with how to reverse it. The owner reads the notes and may reverse any of them.
- A question that sets direction or has no clear winner: it goes to the owner as background, options with SymphonyScript examples, and a recommendation. Work that it does not block continues, and the question closes when the owner answers.
- The documents change first and the code after.
- An implementer decides none of these. It records each gap in its progress file, keeps working on what the gap does not block, and hands back to the head when it is blocked or done. The head answers and resumes it.
- A gap is a behaviour the documents leave undefined or define twice in conflict, a statement in them that cannot be built as written, or a dependency this plan does not name. Implementation choices are not gaps and belong to the implementer: private code, module layout within the code conventions, algorithms and data structures, tests, and the names and shapes of public items ARCHITECTURE does not show, while every ARCHITECTURE example stays true. The implementer lists each public item it adds beyond ARCHITECTURE in its progress file, and the review checks them.
- Changes to synaptic-kernel need the owner's approval of their design before code (CLAUDE.md).

### Work and checks

- Commits go directly on `main` in the main checkout, one step at a time, each step passing the checks. There are no pull requests. At the end of each milestone the implementer pushes `main` to the branch `symphonyscript/progress` on `origin` with `git push origin main:symphonyscript/progress`, never forced, so no work is lost; the owner pushes `main`.
- One implementer runs at a time. The head commits its document edits on `main` only while no implementer runs.
- The implementer keeps its progress in `.memory/symphonyscript/M<n>.md`: steps done, the gaps it found, and the next step. After a compaction it continues from the repository and that file.
- Checks, run by the implementer before every commit and by the reviewer at review: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and `cargo build -p symphonyscript-core --target thumbv7em-none-eabihf`.
- The implementer adds no dependency this plan does not name; another dependency is a gap.
- `packages/symphonyscript/rust-toolchain.toml` pins Rust 1.94.0 with clippy, rustfmt and the `thumbv7em-none-eabihf` target, so the checks give one result on every machine. A toolchain update is a commit of its own, which regenerates the compile-fail goldens and reads them.

### Reviews

- A reviewer reviews a whole milestone, never single commits: the milestone's diff against SPEC, ARCHITECTURE, this plan's exit criteria and CLAUDE.md, running the checks itself.
- A reviewer checks the ordinary paths and the common defects, then goes beyond them to every pattern `.claude/docs/REVIEWING.md` shows, each with its bad and good form.
- The head consolidates the findings with a verdict on each, has the implementer fix the accepted ones, and sorts the rest by the decision rules.
- A milestone gets at most two cold reviews: the first, and a second when the head judges the fixes need fresh eyes. Warm reviews, in which the reviewer that found an issue checks its fix, are unlimited.
- A milestone is done when its exit criteria pass and its last review's accepted findings are fixed and checked. The head then records it in the head notes and starts the next one.

### Code conventions

- Rust edition 2024. Every crate forbids `unsafe` code, and a clippy warning fails the checks.
- One public entity per file, the file named after it; modules are folders.
- No comments in authored files (repository CLAUDE.md).
- Code follows `.claude/docs/CODING.md` § Good patterns. Where another rule of that file names another product's tools or conflicts with this plan, this plan decides.
- Dependencies: `libm` in core; `unicode-ident`, `unicode-normalization`, `unicode-width`, `rowan` and `num-bigint` in the compiler; `lsp-server` and `lsp-types` in the tool; synaptic-kernel in the adapter. Dev-dependencies for tests are the implementer's choice.

## M1: Front end

The workspace, the core data types, the fixtures, the lexer and the parser.

Steps:
1. The Cargo workspace with the five crates of ARCHITECTURE §1. `symphonyscript-kernel` starts empty and takes its synaptic-kernel dependency in M5.
2. Core types: `Value`, `Ty`, ids and handles, `Span`, `Path`.
3. The vocabulary builder, `build()` with every check of SPEC §10.1 except the prelude's, and the stamp (ARCHITECTURE §2).
4. The Program types (ARCHITECTURE §3) and a `dump` printer with a fixed text format.
5. `symphonyscript-fixtures`: the example vocabulary of SPEC §1.3; the fixture projects, each `sym` example of the SPEC that declares every name it uses, and a music project and a DSP project that together use every kind and wire kind of the fixture vocabulary.
6. The lexer (SPEC §2): layout, literals, unit suffixes from the vocabulary, interpolation, NFC comparison, trivia.
7. The lossless syntax tree on `rowan`, with its typed view.
8. The parser for every production, with error recovery (ARCHITECTURE §7). It takes the fixture vocabulary to tell wire placements from node placements.

Exit:
- A test for each error `build()` reports.
- The fixture vocabulary builds, and its stamp is identical across runs and targets and unchanged by a `.doc` edit.
- A hand-built Program dumps to a golden text.
- `symphonyscript-core` builds for `thumbv7em-none-eabihf`.
- A test extracts every `sym` and `sym-error` example from SPEC.md, and every `sym` example parses.
- Every production in SPEC.md has parse tests.
- Parsing any source and printing the tree reproduces it byte for byte.
- Broken sources still yield a tree, with diagnostics at the right spans.
- A source nested 100,000 levels deep in parentheses, brackets and array literals, and one nested 3,000 levels deep in bodies, each lex, parse, print and drop on a thread with a 1 MiB stack (ARCHITECTURE §7).

## M2: Semantics

The compiler's front half: `Compiler`, Collect, Expand, Resolve, Infer and Check (ARCHITECTURE §7, stages 3 to 7), and the diagnostic codes.

Steps:
1. `Compiler::new` with the prelude parsed and resolved once; Collect with the namespace, duplicates, prelude shadowing, the call graph and the pattern use graph.
2. Expand: pattern uses, `for` unrolling, structural values typed and evaluated on demand (SPEC §4.6), recursive calls with their memo and limits.
3. Resolve: scopes and shadowing (SPEC §6.5), paths, `to`, inlining of non-recursive calls, compile-time and reactive classification, declaration cycles.
4. Infer: flexible values, kinds and widths of cells, expected types, widening, input and `let` reads (SPEC §4), unit families (SPEC §3.6), bare variants.
5. Check: the vocabulary checks (SPEC §10.2), wire cycles, fits, the warnings.
6. The `Code` enum and the tests' table from each SPEC error rule to its code.

Exit:
- For every rule in SPEC.md that names an error, a test rejects a program with that error's stable code, and the tests' table maps each such rule to its code (ARCHITECTURE §7).
- Every `sym` example that declares all the names it uses passes Check against the fixture vocabulary, and every `sym-error` example is rejected.
- Diagnostics for expanded patterns point at the use and the pattern line.
- M1's deep sources, a 100,000-deep array type and a chain of 10,000 functions each calling the next pass every stage through Check on a thread with a 1 MiB stack.
- A chain of 40 calls of a function that uses its parameter twice, each call the argument of the next and the innermost reading an input, passes Check in time and memory linear in the chain.

## M3: Runtime and reload

Lowering, the runtime, `eval`, the `Consumer` trait and reload.

Steps:
1. Lower: constant folding with the canonical NaN, the cell graph in dependency order, paths, filled defaults, the string table, the Program in SPEC §11's order.
2. The runtime: validation in `Runtime::new`, commit and changes, the evaluator without recursion, writes (SPEC §12, ARCHITECTURE §4).
3. `eval` and the `Consumer` trait.
4. Reload: `prepare_reload` and `install`, inputs (SPEC §13.2), matching (SPEC §13.3), the report (ARCHITECTURE §5).
5. Fixture pairs of projects, before and after an edit, for the reload tests.

Exit:
- Commit traces match golden traces for the fixture projects.
- A counting allocator shows `commit` allocates nothing.
- Golden values for float built-ins, saturation, division by zero, float `%`, the canonical NaN, clamped indexing and `rand` (SPEC §5.2, §5.5, §5.7, §12.5) match bit for bit.
- Change reports carry element indices for array attributes and follow the order of SPEC §12.3.
- A test for each rule of SPEC §13.
- Inserting one anonymous note into a playing clip reports exactly one addition.
- M2's deep sources lower, load, commit, reload, compare, clone, dump and drop on a thread with a 1 MiB stack.

## M4: Delivery and tooling

Generated Rust, the tool, the watch loop, the language server and the VS Code extension.

Steps:
1. `codegen::generate` and a fixture framework's `build.rs` that uses it.
2. The tool: `Tool::new`, framework commands, `Project`, `new`, `check`, `dump`, single-file mode (ARCHITECTURE §9).
3. The watch loop: recompile, prepare, consumer acceptance, install.
4. The language server on `lsp-server`.
5. The VS Code extension: the static grammar and the language client.

Exit:
- Every fixture project compiled through the library and through `codegen` gives equal Programs and equal commit traces.
- A generated Program builds for `thumbv7em-none-eabihf`.
- End-to-end tests for `new`, `check` and `dump`, and for single-file mode.
- Language server tests for diagnostics, completion, hover, definition, references and rename on the fixture projects.
- The extension starts the language server for a sample project.
- `codegen` and the language server handle M2's deep sources on a thread with a 1 MiB stack.

## M5: Kernel adapter

`move_node_after` and `move_node_before` in synaptic-kernel, and the adapter of ARCHITECTURE §10.

Steps:
1. The design of the move operations, keeping a node's slot, synapses and cells and every kernel invariant, to the owner for approval. The adapter work that does not use them proceeds meanwhile.
2. The move operations, following synaptic-kernel's `CONVENTIONS.md`, after the owner's approval.
3. The adapter: structure, the layout table, the planes and the array region, strings in the LUT, apply and publish, capacity and reload. The array region's kernel mechanism, an entry store or a LUT, is chosen after reading synaptic-kernel's API, by the decision rules.

Exit:
- A fixture project loads into synaptic-kernel. A consumer thread entering at the sentinel reaches every instance and wire, and reads string attributes from the LUT.
- An input write and a commit become visible to the consumer thread, and the adapter publishes only when a meta-plane or array value changed. A vocabulary with a large array attribute leaves the stride of nodes without arrays unchanged.
- A reload keeps the slots and serials of matched instances, and moving a kept instance re-creates none of its synapses.
- The kernel's own test suite passes with the move operations added.

## Order

M1, M2 and M3 run in order. M4 and M5 each need M3 and run one after the other, M4 first.
