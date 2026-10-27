# SymphonyScript Plan

The implementation runs in five milestones. The owner approves this plan once; the head then drives the milestones in order without stopping between them and reports each one in the head notes.

## How the work runs

### Roles

- **Owner:** decides direction.
- **Head:** the session that drives this plan. It briefs implementers and reviewers, sorts every gap, edits the documents, decides what the decision rules let it decide, and keeps the head notes in `.memory/symphonyscript/notes.md`. The head owns the decisions and the architecture, and the reviewers own the code's correctness against them.
- **Implementer:** one agent per milestone, Opus at effort `high`.
- **Reviewer:** a fresh agent per cold review, Opus at effort `xhigh` for M2, M2b, M2e, M2c, M2d, M3 and M5 and `high` for M1, M4a and M4b. In M2, M2b, M2e, M2c, M2d, M3 and M5 a second fresh reviewer, Fable at effort `xhigh`, reviews the hot path in the same cold review, and the Opus reviewer the rest of the diff: in M2, compile-time evaluation and its limits (SPEC §4.6, §6.3) and inference (SPEC §3.6, §4); in M2b, the types and capacities of arrays and states (SPEC §3.7, §6.6) and bounded evaluation (§6.3, §4.6); in M2e, Infer's capacity rules and the `for` expression's evaluation (SPEC §5.10, §7.9, §4.6); in M2c, the capacities and bindings of strings, records, sums and arrays with capacities at every level (SPEC §3.2, §3.3, §3.7, §3.8, §3.9, §4.4), maps (§3.10) and documents' budgets, typed reads and equality (§3.11); in M2d, pattern types and expansion (§6.4) and triggers (§6.6); in M3, Lower's constant folding, the runtime (SPEC §5's built-ins, §12) and reload (SPEC §13); in M5, the adapter (ARCHITECTURE §10) and the kernel move operations.

### Decisions

- A gap or contradiction with a single answer consistent with the documents: the head decides it, edits the documents and records it in the head notes.
- A question the documents and the owner's rulings answer, whose best-practice answer stands out among the alternatives and which is cheap to reverse: the head decides it and records it in the head notes, with how to reverse it. The owner reads the notes and may reverse any of them.
- A question that sets direction or has no clear winner: it goes to the owner as background, options with SymphonyScript examples, and a recommendation. Work that it does not block continues, and the question closes when the owner answers.
- The documents change first and the code after.
- Before a milestone starts, the head checks its documents for cross-cutting questions (nesting depth, size and cost, determinism, allocation, error paths and cascades) and settles them in the documents first.
- An implementer decides none of these. It records each gap in its progress file, keeps working on what the gap does not block, and hands back to the head when it is blocked or done. The head answers and resumes it.
- A gap is a behaviour the documents leave undefined or define twice in conflict, a statement in them that cannot be built as written, or a dependency this plan does not name. Implementation choices are not gaps and belong to the implementer: private code, module layout within the code conventions, algorithms and data structures, tests, and the names and shapes of public items ARCHITECTURE does not show, while every ARCHITECTURE example stays true. The implementer lists each public item it adds beyond ARCHITECTURE in its progress file, and the review checks them.
- Changes to synaptic-kernel need the owner's approval of their design before code (CLAUDE.md).

### Work and checks

- Commits go directly on `main` in the main checkout, one step at a time, each step passing the checks. There are no pull requests. At the end of each milestone the implementer pushes `main` to the branch `symphonyscript/progress` on `origin` with `git push origin main:symphonyscript/progress`, never forced, so no work is lost; the owner pushes `main`.
- One implementer commits on `main` at a time. Further implementers run beside it in git worktrees of their own, each committing on a branch that the head merges into `main` with a merge commit, where their files are disjoint from the others': M4a beside M3, and the parallel steps a milestone names; every other rule of this section binds them. The SPEC, ARCHITECTURE and this plan are edited and committed on the branch `docs`, in its worktree, at any time; the head merges `docs` into `main` with a merge commit before every launch, and a launch is refused while `docs` holds commits `main` lacks (`git merge-base --is-ancestor docs main`).
- The implementer keeps its progress in `.memory/symphonyscript/M<n>.md`: steps done, the gaps it found, and the next step. After a compaction it continues from the repository and that file.
- Checks, run by the implementer before every commit and by the reviewer at review: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and `cargo build -p symphonyscript-core --target thumbv7em-none-eabihf`.
- The implementer adds no dependency this plan does not name; another dependency is a gap.
- The workspace's `[profile.dev]` sets `opt-level = 1` and `debug = "line-tables-only"`, and `cargo test` runs the limit tests under it; an implementer or a reviewer that builds in a copy of the checkout deletes the copy's build directory when it hands back.
- A test that asserts a cost counts the work its pass does, queries, nodes or visits, never the clock.
- `packages/symphonyscript/rust-toolchain.toml` pins Rust 1.94.0 with clippy, rustfmt and the `thumbv7em-none-eabihf` target, so the checks give one result on every machine. A toolchain update is a commit of its own, which regenerates the compile-fail goldens and reads them.

### Reviews

- A reviewer reviews a whole milestone, never single commits: the milestone's diff against SPEC, ARCHITECTURE, this plan's exit criteria and CLAUDE.md, running the checks itself.
- A reviewer checks the ordinary paths and the common defects, then goes beyond them to every pattern `.claude/docs/REVIEWING.md` shows, each with its bad and good form.
- The head consolidates the findings with a verdict on each, has the implementer fix the accepted ones, and sorts the rest by the decision rules.
- The head briefs each fix as the property it must hold, with an adversarial test and a test over valid input beside the reported case, and checks a reviewer's proposed fix against the SPEC before briefing it.
- The implementer fixes every finding the head sends it, whoever wrote the code and whichever milestone it came from, and declines none. An item that cannot be built as the documents stand is a gap (§ Decisions), never a reason to skip it.
- A milestone gets at most two cold reviews: the first, and a second when the head judges the fixes need fresh eyes. Warm reviews, in which the reviewer that found an issue checks its fix, are unlimited.
- A milestone is done when its exit criteria pass and its last review's accepted findings are fixed and checked. The head then records it in the head notes and starts the next one.

### Carried findings

A finding, gap, limitation, review note or task that is not built in the round that found it is a carried item, and the ledger `.memory/symphonyscript/CARRY.md` is the one record of every carried item. Nothing else records a carry: a triage entry in the head notes, a brief, a progress file or a message names the ledger entry's id and nothing more.

- Entry. The head writes a carried item into the ledger in the same step as the triage that carries it, before the brief of that round launches, with the item's id, its source (the review or hand-back, the tip, the date), its severity, its full text copied from the report (the defect, the rule it breaks with the document section, the case, the fix the reviewer proposed), the head's triage line, and its state. A hand-back's open note, gap left unanswered or limitation recorded in a progress file is a carried item too, and the head enters it when it reads the hand-back. An item that is the owner's to decide is entered with the state `owner` and the question as asked.
- States. `open`, with the round or milestone it is carried to and the reason; `taken` by a brief, with the round; `done`, with the commit on main that built it and the test or document line that shows it, written when the round's hand-back is read; `cancelled`, with the ruling that cancels it cited by its date or the owner's words; `owner`, waiting for the owner's decision, with the date it was asked. A state changes only forward, in a `State` line that names the record it rests on; the newest `State` line is the entry's state.
- Briefs. Every fix-round brief ends with a section § Ledger listing every open entry of the ledger by id, each marked `taken` or `carried` with the reason and the round or milestone it goes to. A brief that lists no § Ledger does not launch. A review brief, cold or warm, lists the open entries whose case the reviewer re-runs on its tip, and the reviewer's report says for each whether the case still reproduces; an entry whose case no longer reproduces is marked `done` only with the commit found.
- Milestones. A milestone is done when every ledger entry opened before its last cold review is `done` or `cancelled`, so a minor found by the last cold review carries into the next milestone's first fix round and nothing else carries across a milestone. The next milestone's first fix round takes every open entry as its first items. The milestone's exit line here and its Milestone log line in the head notes state the ledger's open count at its close.
- Check. `packages/symphonyscript/tools/carry/check.py` reads the ledger, the newest brief under `work/` and main's log, and fails when an open or taken entry is missing from that brief's § Ledger, a `done` entry's commit is not on main, a `cancelled` entry cites no ruling, or a state moves backwards. The head runs it before every launch and before it writes a milestone done, and records its summary line in the head notes.
- Grounding. The head's Status in the head notes names the ledger and its open count, and the grounding note after every compaction lists the open entries' ids.
- Words. The head writes no "later", "carry", "queue", "defer", "follow-up", "after M<n>" or "when room allows" about a finding without the ledger entry's id beside it.

### Code conventions

- Rust edition 2024. Every crate forbids `unsafe` code, and a clippy warning fails the checks.
- One public entity per file, the file named after it; modules are folders.
- No comments in authored files (repository CLAUDE.md).
- Code follows `.claude/docs/CODING.md` § Good patterns. Where another rule of that file names another product's tools or conflicts with this plan, this plan decides.
- `Cargo.lock` is committed, as a workspace with binaries commits it.
- Dependencies: `libm` in core; `unicode-ident`, `unicode-normalization`, `unicode-width`, `rowan` and `num-bigint` in the compiler; `lsp-server` and `lsp-types` in the tool; the IntelliJ Platform Gradle Plugin for the JetBrains plugin; synaptic-kernel in the adapter. Dev-dependencies for tests are the implementer's choice.

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
8. The parser for every production, with error recovery (ARCHITECTURE §7.1). It takes the fixture vocabulary to tell wire placements from node placements.

Exit:
- A test for each error `build()` reports.
- The fixture vocabulary builds, and its stamp is identical across runs and targets and unchanged by a `.doc` edit.
- A hand-built Program dumps to a golden text.
- `symphonyscript-core` builds for `thumbv7em-none-eabihf`.
- A test extracts every `sym` and `sym-error` example from SPEC.md, and every `sym` example parses.
- Every production in SPEC.md has parse tests.
- Parsing any source and printing the tree reproduces it byte for byte.
- Broken sources still yield a tree, with diagnostics at the right spans.
- Recovery holds ARCHITECTURE §7.1's properties, shown by generated tests: every `sym` example and fixture file, re-flowed inside its brackets, parses unchanged; and in files built from independent single-fault items, each fault is reported exactly once at its span, in time linear in the file.
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
- Every resource limit is a field of `Limits` with its default in one constants file, and a test sets each limit below and above its default and sees the compiler follow it.

## M2b: Language additions

States and history reads, bounded recursion, variable arrays with joins, slices and `len`, `u8` and strings as bytes, native functions, `to` paths, and teaching diagnostics: lexing, parsing and the semantics through Check, and a lesson for every code. M3 builds their runtime.

Steps:
1. The lexer and the parser for `state`, history reads, `bounded`, `[]`, `T[..N]`, slices and `^`, `u8`, and `to` paths, with the tests over SPEC.md's examples and rule sentences passing again.
2. Collect and Resolve: states and their history reads, bounded functions, calls that read cells into recursive functions, the expansion of bounded calls, and `to` paths.
3. Infer: fixed and variable arrays, capacities, joins, slices, `len` and `^`, history reads, the types and capacities of states, bounded compile-time evaluation, `u8`, strings as bytes and `bytes`, variable-array attributes, and native functions: the builder's `native`, their signatures, and their compile-time calls.
4. Check: the errors of the new rules.
5. Diagnostics: a summary, a help and labelled related spans on every diagnostic, and a lesson for every code.

Exit:
- M2's exit criteria hold over the SPEC as it now reads: every rule that names an error has its test and its row, every `sym` example that declares its names passes Check, and every `sym-error` example is rejected.
- Each row of SPEC §3.7's capacity table and each state rule of §6.6 has tests whose capacities are derived by hand, and a state bounded by `[^1000000000..]` finds its capacity in work that does not depend on the bound.
- A bounded call expands in work linear in its levels times its body; a bounded function expanded 100,000 levels deep and a chain of 100,000 slices pass every stage through Check on a thread with a 1 MiB stack.
- Every code has a lesson, every lesson's `sym` examples pass Check and its `sym-error` examples draw its code, and every diagnostic the error-rule table's programs draw has a summary of one to three lines.
- The robustness corpus gains mutants of the new examples, and none draws internal-error or panics.
- A fixture vocabulary's native functions, one taking a string, are typed by their signatures, called at compile time with compile-time arguments, and refused with an argument that reads a cell where they take a string; a variable-array attribute accepts a value up to its capacity and refuses a larger one.

## M2e: Loops

The `for` expression, iterables with one or two loop names, the removal of `bounded`, and a warning beside every limit: lexing, parsing and the semantics through Check, with a lesson for every new code. M3 builds the `for` expression's runtime.

Steps:
1. The lexer and the parser: `into`, the `for` expression in its one-line and block forms, the loop head with one or two names in placement `for`s, bundles and `for` expressions, and `bounded` out of the grammar, with the tests over SPEC.md's examples and rule sentences passing again.
2. Resolve and Infer: iterables over arrays, maps and ranges with the bound rule (SPEC §7.9), and a slice's capacity from its bounds' maximum by the same rule (SPEC §3.7); the loop names' scope and the name rules of SPEC §6.5 over loop names, accumulators and a block form's local `let`s; the accumulator's type and every capacity in it (SPEC §5.10); a `for` expression evaluated at compile time when it reads no cell and one cell otherwise, its compile-time iterations counted toward SPEC §4.6's `for` iterations; its body inlined once and counted once; the removal of bounded expansion and its memo nesting; and the new help on a recursive call that reads a cell (SPEC §6.3).
3. The limits: a `<name>_warning` field beside every limit that draws an error, its default half the limit's in the constants file, the `Limits` fields that `[limits]` keys name, and the warning diagnostic, naming what the limit's error names (SPEC §4.6, ARCHITECTURE §7, §9).
4. Check, diagnostics and tests: the errors of the new rules, a summary and a help on each, a lesson for every new code, and a test for each rule of SPEC §5.10 and §7.9.

Exit:
- M2b's exit criteria hold over the SPEC as it now reads: every rule that names an error has its test and its row, every `sym` example that declares its names passes Check, and every `sym-error` example is rejected.
- Every `sym` example of SPEC §5.10 and §7.9 passes every stage through Check, and every `sym-error` example of them is rejected.
- For a `for` expression over a variable array, Check's time and Infer's node count grow linearly in the array's capacity, measured up to a capacity of 10,000.
- Every form of SPEC §7.9's bound rule has a test whose minimum and maximum are derived by hand, for a range and for a slice, and a slice whose bound has no maximum by its form takes the whole array's capacity.
- Each limit draws its warning at its warning value and its error at its limit.
- A program written with `bounded` is rejected with a help naming the `for` expression.
- A fault in a branch that a compile-time recursion passes over is reported in work linear in the recursion's depth, counted by the test, the property the removal of bounded expansion keeps.
- Every code has a lesson, every lesson's `sym` examples pass Check and its `sym-error` examples draw its code, and every diagnostic the error-rule table's programs draw has a summary of one to three lines.
- The robustness corpus gains mutants of the new examples, and none draws internal-error or panics.

## M2c: Language additions 2

Runtime strings, records, `none` and sums, aliases, arrays with capacities at every level, maps, documents, annotations on every binder, generics on functions, native functions and `type` declarations with their constraints, value parameters as compile-time constants, sizes and type arguments as expressions, function types with named parameters, lambdas, functions and built-ins passed as lambdas, compile-time `let`s holding a lambda or a function, native functions taking a lambda at compile time, and the vocabulary's `takes`, record, string, map and document attributes: lexing, parsing and the semantics through Check, with a lesson for every new code. M3 builds their runtime.

Steps:
1. The lexer and the parser: `type` in both forms, `str`, `none`, `|` and `?` in types, capacities at every level, `map`, `json` with the lexer's JSON mode, `without`, record, map and document literals, annotations on parameters, local `let`s, `for` variables and results, string and sum `match` labels, bracket parameters on functions and `type` declarations, a type's bracket arguments and its sizes as expressions, function types with named parameters, and lambdas as arguments and as a `let`'s value, with the tests over SPEC.md's examples and rule sentences passing again.
2. The vocabulary: `attr_str`, `attr_record`, `attr_record_any`, `attr_map`, `attr_json`, `attr_json_any`, `takes`, `type` in the prelude, native string signatures, native signatures with bracket parameters and function-type parameters, `build()`'s new errors and the stamp.
3. Core's `document` module: the RFC 8259 parser into a node pool within a budget, iterative, with kinds, views, equality and typed reads, built without `std` for the compiler and the runtime alike (ARCHITECTURE §4, §7).
4. Collect and Resolve: record types, aliases and sums with their cycles, the name rules of SPEC §6.5 over the new declarations, bracket names and lambda parameters, the new literals and operators, lambdas, functions and built-ins inlined at their parameters' calls and passed on as arguments (SPEC §6.7), a compile-time `let` holding a lambda or a function inlined at each use and call, a use reactive when its lambda's body reads a cell, the cycle through a `let`'s lambda (SPEC §6.2), and calls by a function type's parameter names.
5. Infer: strings' capacities, records as bindings, literals typed by their position, annotations as fixed types, sums (widening, `or`, `match` narrowing, `is`), capacities at every level, maps, documents (views and budgets, typed reads, literals, `typeof`, equality), their compile-time evaluation, SPEC §4.6's inline limit of 100,000, bracket parameters bound from the bare names in parameter types with their constraints and defaults, size expressions checked once bound, value parameters as compile-time constants, type arguments read by the parameters they fill and a record literal binding a generic type's parameters (SPEC §3.2, §6.8), lambdas' and functions' arity and types against function types or the parameter's calls, and lambdas and functions passed as arguments in the recursive-call memo (SPEC §4.6, §6.7), and a native call taking a lambda evaluated at compile time, running the lambda's body at each callback (SPEC §10.1).
6. Check and diagnostics: the binding each attribute takes, the string, record, map and document attribute rules, a `T?` at an attribute, the errors of the new rules, a summary and a help on each, and a lesson for every new code.

Steps 2 and 3 touch files the others do not and run in worktrees of their own beside step 1 (§ Work and checks), merged before step 4.

Exit:
- M2e's exit criteria hold over the SPEC as it now reads: every rule that names an error has its test and its row, every `sym` example that declares its names passes Check, and every `sym-error` example is rejected.
- Each row of SPEC §3.3's capacity table, each inner-level case of §3.7 and each fit rule of §3.11's typed read has tests whose values are derived by hand.
- Every rule of SPEC §3.9 has its test: widening in and no narrowing, `match` coverage and narrowing per arm, `or`, `is`, a record literal choosing its alternative, and a `T?` refused at an attribute.
- Every operation of SPEC §3.10 and every rule of §3.11 has its test, `is` between documents and between a document and each value that widens into one among them; core's `document` module accepts every valid document and refuses every invalid one of a conformance corpus with the byte position, refuses a document past its budget naming it, and parses 64 KiB in time linear in its length.
- Every rule of SPEC §6.7 and §6.8 has its test: a lambda's arity, annotations and scope, a function type refused outside a parameter and a compile-time `let`, a function type's named parameters called by name, a lambda passed on, a function and a built-in passed with and without a function type, a compile-time `let` holding a lambda or a function, called and passed, its use reactive when the lambda's body reads a cell, a function reference refused outside its positions, a lambda's body reading the scope it is written in, a `let` reaching itself through its lambda, bracket parameters bound and unified from arguments, a bracket parameter bare in no parameter type, a size expression checked at its call, a size out of range or a float, a value parameter in every position SPEC §6.8 lists, a type argument as an expression, constraints, the defaults of a flexible argument, a record literal binding a generic type's parameters, and a generic type named without its arguments.
- A fixture vocabulary's native function that takes a lambda is evaluated at compile time with compile-time arguments, running the lambda's body at each of its calls, and a generic native function binds its bracket parameters from its arguments.
- A fixture vocabulary with an attribute that takes an input, one that takes a constant, a record attribute of a prelude type, one taking any record, a runtime string attribute, a map attribute and a document attribute accepts what SPEC §10.2 allows, refuses the rest, and yields a Program whose bindings a host follows to the inputs it writes.
- Every code has a lesson, every lesson's `sym` examples pass Check and its `sym-error` examples draw its code, and every diagnostic the error-rule table's programs draw has a summary of one to three lines.
- The robustness corpus gains mutants of the new examples, and none draws internal-error or panics.
- A record type of 1,000 fields, a sum of 100 alternatives, an array with capacities at 8 levels and a document literal of 10,000 entries pass every stage through Check on a thread with a 1 MiB stack.

## M2d: Language additions 3

Pattern types, inputs and states in patterns, state triggers, the `input` and `const` parameter modifiers, the `children` parameters, wire patterns and pattern generics: lexing, parsing and the semantics through Check, with a lesson for every new code. M3 builds their runtime.

Steps:
1. The lexer and the parser: `const`, `children`, `on`, `node`, `wire`, placement types with counts and bundle entries, the `children` parameters and a use's body, the classification of patterns before bodies and wire patterns used as wires (SPEC §7.1), `input` and `state` lines in pattern bodies, and bracket parameters on patterns, with the tests over SPEC.md's examples and rule sentences passing again.
2. Collect, Expand and Resolve: a pattern's inputs and states under each use's path, a use's body partitioned among its `children` parameters, the parameter modifiers, the name rules of SPEC §6.5 over the new declarations, triggers, and a pattern's bracket parameters bound in Expand from a node reference or a placement (ARCHITECTURE §7).
3. Infer: pattern types inferred and checked against their declarations, triggers typed as reactive `let`s, and a pattern's other bracket parameters bound from its arguments (SPEC §6.8).
4. Check and diagnostics: the errors of the new rules, SPEC §10.2's checks on each use's expansion against the kind a type parameter binds, a summary and a help on each, and a lesson for every new code.

Exit:
- M2c's exit criteria hold over the SPEC as it now reads: every rule that names an error has its test and its row, every `sym` example that declares its names passes Check, and every `sym-error` example is rejected.
- Every pattern in the SPEC's examples has the type SPEC §6.4 gives it, inferred, and a declared type the body breaks is rejected naming the placement; a `children` parameter refuses a use's body outside its counts, and a wire pattern is used as a wire with its name, gate and `to`.
- Two uses of a pattern that declares an input and a state yield two inputs and two states named by the uses' paths, listed at the uses' positions.
- A pattern whose type parameter binds to a node kind expands per use, and each use's expansion is checked by SPEC §10.2 against the kind it binds; a pattern's value parameter binds from its arguments as a function's does.
- Every code has a lesson, every lesson's `sym` examples pass Check and its `sym-error` examples draw its code, and every diagnostic the error-rule table's programs draw has a summary of one to three lines.
- The robustness corpus gains mutants of the new examples, and none draws internal-error or panics.
- A pattern with `children` used 10,000 times, each with a body, passes every stage through Check on a thread with a 1 MiB stack.

## M3: Runtime and reload

Lowering, the runtime, `eval`, the `Consumer` trait and reload, for the language as M2, M2b, M2e, M2c and M2d leave it.

Steps:
1. Lower: constant folding with the canonical NaN, the cell graph in dependency order, paths, filled defaults, the string table, the Program in SPEC §11's order, the memory figure (SPEC §12.6), and the choice per non-recursive function between inlining and one shared body by Q6's measure over the resolved body instances (body nodes times call sites less one over `shared_body_threshold`; small and single-site bodies inlined; one shared body per distinct set of argument and result types; a call with only compile-time arguments folded; ARCHITECTURE §7), with a function table and a call node in the Program.
2. The runtime: validation in `Runtime::new`, commit and changes, the evaluator without recursion, states with their histories and triggers, variable arrays with joins and slices at every level, `For` nodes, each evaluating its body subtree once per element, strings stored one `u32` per character with UTF-8 at the boundary, records per leaf field, sums as a tag and slots, maps, documents parsed at the boundary by core's `document` module and read through views and typed reads, native functions at runtime with a callback for each lambda argument, taken from the vocabulary `Runtime::new` takes, whose stamp it checks (SPEC §13.1), typed attribute handles, the builder's `Attr<T>` read by `rt.attr` in the attribute's Rust type (ARCHITECTURE §2, §4), and writes (SPEC §12, ARCHITECTURE §4).
3. `eval` and the `Consumer` trait.
4. Reload: `reload_snapshot`, `prepare` and `install`, inputs (SPEC §13.2), matching (SPEC §13.3), the report (ARCHITECTURE §5).
5. Fixture pairs of projects, before and after an edit, for the reload tests.

Exit:
- Commit traces match golden traces for the fixture projects.
- A counting allocator shows `commit` allocates nothing.
- Golden values for float built-ins, saturation, division by zero, float `%`, the canonical NaN, clamped indexing and `rand` (SPEC §5.2, §5.5, §5.7, §12.5) match bit for bit.
- Change reports carry element indices for array attributes and follow the order of SPEC §12.3.
- A test for each rule of SPEC §13.
- Inserting one anonymous note into a playing clip reports exactly one addition.
- M2's deep sources lower, load, commit, reload, compare, clone, dump and drop on a thread with a 1 MiB stack.
- Compiling a fixture project repeatedly, with its sources in any order, gives equal Programs and equal dumps.
- A generated test mutates fixture Programs (ids out of range, cells out of order, mismatched operand types, picks with no candidates), and each mutant is refused by `Runtime::new` or commits and reloads without a panic.
- `Runtime::new` refuses a Program compiled against another vocabulary, and calls the vocabulary's native functions at runtime.
- A fixture consumer reads attributes through their `Attr<T>` handles in their Rust types, and a compile-fail test shows that renaming or retyping an attribute in the vocabulary stops a consumer that reads it from compiling.
- A reload prepared on another thread while the runtime keeps committing installs with the writes made meanwhile applied at the next commit.
- The state shared between the committing thread and a thread preparing a reload holds under loom at a preemption bound of 3 and under Miri with `-Zmiri-many-seeds`.
- The compiler reports each fixture Program's memory in bytes, equal to what `Runtime::new` allocates under the counting allocator, and refuses a program over `memory_bytes` with the figure in the error (SPEC §12.6).
- A benchmark over the fixture projects records compile time and Program size with and without shared bodies in the progress file.
- Reload matching follows SPEC §13.3's search bound, and a benchmark over generated edits measures Myers' cost against edit distance and sequence length, recorded in the progress file.
- Golden traces cover states and histories, triggers, variable arrays at every level, strings, records, sums, maps and documents; a string written past its capacity or in invalid UTF-8, and a document written past its budget or invalid, are refused with the capacity or the byte position in the error.
- Golden traces cover `for` expressions over arrays, maps and ranges, each evaluating its body once per live element, and a native function that calls a lambda at runtime; a counting allocator shows neither allocates in `commit`.

## M4a: Language server and editors

The tool's frame, the language server, the VS Code extension and the JetBrains plugin: the tooling that needs the compiler alone. It needs M2d and runs beside M3 (§ Order).

Steps:
1. The tool: `Tool::new`, framework commands, `Project`, `new`, `check` and single-file mode, the diagnostics' rendering with the summary, the help, the lesson under `── why ──`, the spans under `── where ──` and `--brief` (ARCHITECTURE §9), and the `lsp` command.
2. The language server on `lsp-server`.
3. The VS Code extension: the static grammar and the language client.
4. The JetBrains plugin: highlighting from the grammar or the server's semantic tokens, and the language client on the IntelliJ Platform's LSP API.

Exit:
- End-to-end tests for `new` and `check`, and for single-file mode.
- Language server tests for diagnostics, completion, hover, definition, references and rename on the fixture projects; hover on a pattern and on each of its uses shows the pattern's type (SPEC §6.4).
- The extension starts the language server for a sample project.
- The JetBrains plugin starts the language server for a sample project in IntelliJ IDEA, highlights its sources and shows its diagnostics.
- `check` prints each diagnostic as ARCHITECTURE §9 renders it, the lesson under `── why ──` and the spans under `── where ──`, and `--brief` leaves out the lesson.
- The language server handles M2's deep sources on a thread with a 1 MiB stack.
- `symphony.toml`'s `[limits]` and `--limit` reach the compiler and the language server, a flag overriding the file.

## M4b: Delivery

Generated Rust, `dump`, the watch loop, and what of the tool needs the runtime.

Steps:
1. `codegen::generate` and a fixture framework's `build.rs` that uses it, with a typed write method for each input whose variable-array inputs take core's bounded list type (ARCHITECTURE §8).
2. The tool's `dump`, and `check` printing the memory figure (SPEC §12.6).
3. The watch loop: recompile, prepare, consumer acceptance, install.

Exit:
- Every fixture project compiled through the library and through `codegen` gives equal Programs and equal commit traces.
- A generated Program builds for `thumbv7em-none-eabihf`.
- A write through a generated method longer than a variable-array input's capacity does not compile, shown by a compile-fail test.
- End-to-end tests for `dump`, and for `check` printing the memory figure.
- `codegen` handles M2's deep sources on a thread with a 1 MiB stack.
- `symphony.toml`'s `[limits]` and `--limit` reach the runtime, a flag overriding the file.
- `codegen` emits the Program's function table and call nodes, and a generated Program's shared bodies run as the library's do.
- A CI job on an x86_64 Linux runner asserts the pinned vocabulary stamp 0x35f8efc8552d30a7, so the stamp's encoding is machine-independent (SPEC §13.1).

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
- The adapter's publish and the consumer thread's reads hold under loom at a preemption bound of 3 and under Miri with `-Zmiri-many-seeds`.

## Order

M1, M2, M2b, M2e, M2c, M2d and M3 run in order. M4a needs M2d and runs beside M3. M4b and M5 each need M3 and run one after the other, M4b first.
