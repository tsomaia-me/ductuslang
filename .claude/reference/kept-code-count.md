# The settled lines the kept syntax code breaks, at `6491d802`

Counted once by the peer session on 2026-11-14, read-only, after the code standard's five topics were settled. No fix is proposed and no size is estimated. It sits beside the trial's result when the owner chooses repair or rewrite. Paths are under `crates/compiler/src/syntax/` of `/home/cdagent/cargo-targets/r3/wt/packages/symphonyscript/`. "Searched" means a pattern counted the places; "read" means each place was opened; "from the sizing" means `syntax-repair-size.md` gives the place and it was not opened again.

## The parts

| Part | Files | Lines | What it holds |
|---|---|---|---|
| Lexer | 12 | 1,566 | `lexer/`, `lex.rs`, `lexed.rs`, `token.rs`, `nfc.rs` |
| Grammar | 19 | 4,979 | `parser/` but its tests and the tree builder, `parse.rs` |
| Tree | 14 | 1,370 | `parser/tree_builder.rs`, `parsed.rs`, `syntax_kind.rs`, the pools, walks and views of nodes and tokens |
| Views | 108 | 3,009 | `ast/` but its tests |
| Old tests | 12 | 4,218 | `ast/tests.rs`, `cost_tests.rs`, `front_end_goldens.rs`, `lexer/tests.rs`, `parser/tests/` |

## The counts

| Line | What was counted | Lexer | Grammar | Tree | Views |
|---|---|---|---|---|---|
| Comments; 3.3 | Files that open with their job, what they take and hand on, and their invariants | 0 of 12 | 0 of 19 | 0 of 14 | 0 of 108 |
| Comments; 3.3 | Comment lines; `// SPEC` marks (searched) | 0; 0 | 5; 0 | 0; 0 | 0; 0 |
| 1 | Interior mutability in a finished result (searched, read) | 0 | 0 | 3 fields | 0 |
| 1 | A result changed after it was written (from the sizing) | tokens cut and lexed again | 0 | token kinds renamed after the parse | 0 |
| 2 | Lines importing from `semantics` (searched) | 1 | 4 | 7 | 0 |
| 3, 5 | `bool` fields of the one object every method changes (read) | 4 of 28 fields | 3 of 19 fields | not read | none |
| 5 | Flags and side lists beside the message list (read, and from the sizing) | 3 side lists | 2 flags, 1 de-duplication, 2 side lists | 0 | 0 |
| 6 | `_ =>` arms on our own enums (searched, each read) | 2 of 20 | 24 of 26 | 0 of 3 | 10 of 10 |
| 6 | `matches!` naming a variant of our own enums (searched) | 3 of 9 | 9 of 15 | 1 of 1 | 2 of 5 |
| 6 | `if let` or `let … else` naming a variant of our own enums (searched) | 4 of 20 | 2 of 42 | 0 of 9 | 0 of 4 |
| 6 | Code that chooses among constructs and does their work | not judged | `step`, 194 lines over 121 task kinds; `primary`, 132 lines | 0 | 0 |
| 8 | `as` casts between integer types (searched) | 25 | 8 | 18 | 0 |
| 8 | Views that answer by position and answer wrong (from the conformance review, C1 to C5) | 0 | 0 | 0 | 5 |
| 9; 3.2 | Decisions from a fact the stage's input does not hold (read, and from the sizing) | 2 | 3 | 0 | 0 |
| 4; H | Lines that write a message's words in place (searched) | 27 | 177 | 0 | 0 |
| 4 | Message words written at two or more places (searched) | 21 strings over both | | 0 | 0 |
| 2.1 | Paths whose work grows with the square of the input (from the recovery review) | 1 | 1 | 0 | 0 |
| 2.2 | Allocation a token; a hash operation at a lookup | about 5,000 on the large fixture, not counted by site | 1 map read at lookahead | not measured | not measured |
| 2.4 | A function that calls itself on a depth the source sizes | none: 100,000 levels pass on 1 MiB (the recovery review's run) | none | none | not run |
| 2.5 | Loops that state their total | 0 | 0 | 0 | 0 |
| Panics | `unwrap`, `expect` of the standard library, `panic!`, `unreachable!`, an assertion (searched) | 0 | 0 | 0 | 0 |

## The places

- **Comments.** The five comment lines are `parser/loops.rs:152-154` and `parser/mod.rs:852-853`; the second cites ARCHITECTURE §7.1, the design being replaced.
- **Line 1.** `parsed.rs:23-25`: `parents`, `token_parents` and `child_pool` are `OnceLock` columns filled on first read. `parser/tree_builder.rs:33` with `parser/mod.rs:899-909` and `:138-142`: a keyword's token kind is renamed after the parse. `lexer/mod.rs:346-415`, called from `parser/mod.rs:450` and `:821`: the lexer cuts its tokens and lexes again.
- **Line 2.** `lexer/mod.rs` 1; `parser/mod.rs` 4; `parser/tree_builder.rs` 2, `node_view.rs` 4, `child_pool.rs` 1. They name `counted_walk`, `analysis::fast_hash` and `reporting::order`.
- **Lines 3 and 5.** Lexer: `line_has_token`, `clause_pending`, `finished`, `json_next` (written at `lexer/mod.rs:412`, `:577`, `:684`, `:688`, `:965`); side lists `faults`, `reach`, `cut_constructs` (`:528`, `:546`, `:824`). Parser: `line_error` (written at `parser/mod.rs:655`, `:776`, `:806`), `previous_line_error` (`:654`), `block_ended`; `same_fault_as_lexer` (`:788-802`); side lists `detached` (`:768`, `:875`) and `at_layout` (`:781`).
- **Line 6, `_ =>` on our own enums.** Lexer: `lexer/mod.rs:560`, `:1302`. Grammar: `parser/starts_line.rs:22`, `:78`, `:120`; `parser/expressions.rs:37`, `:339`, `:466`, `:606`; `parser/items.rs:61`; `parser/types.rs:169`; `parser/json.rs:69`; `parser/placements.rs:203`, `:381`; `parser/decls.rs:280`; `parser/blocks.rs:101`, `:343`; `parser/mod.rs:196`, `:520`, `:583`, `:599`, `:705`, `:1008`, `:1027`, `:1075`, `:1118`. Views: `ast/json_value.rs:22`, `ast/type.rs:20`, `ast/expr.rs:65`, `ast/member.rs:20`, `ast/flags.rs:25`, `ast/iterable.rs:16`, `ast/arg_list.rs:29`, `ast/body_item.rs:26`, `ast/type_ref.rs:57`, `ast/item.rs:44`. The other arms match a character, a byte, a string or an `Option`.
- **Line 6, the choosers.** `parser/mod.rs` `step` (194 lines) over `parser/task.rs`'s 121 task kinds; `parser/expressions.rs` `primary` (132 lines). The kind enum has 182 variants.
- **Line 8.** The five views: `ast/type_ref.rs:42`, `:47`; `ast/lambda.rs:8`; `ast/arg_list.rs:22` with `ast/named_arg.rs:25`; `ast/attr_entry.rs:29`; `ast/placement_entry.rs:11`.
- **Line 9.** Lexer: `json_next`, the word before decides JSON text (`lexer/mod.rs:964-966`); the keyword list that ends a bracket's region (`lexer/mod.rs:462-470`, `:1312-1335`). Grammar: `at_wire_kind` reads the vocabulary to choose wire or node (`parser/mod.rs:546`, used at `parser/items.rs:50`, `parser/placements.rs:229`, `parser/blocks.rs:96`); `is_kind` (`parser/starts_line.rs:54-59`); `parser/starts_line.rs` whole, 143 lines of line shapes that end a region. The lexer also reads unit names (`lexer/mod.rs:1094`); whether that is a breach waits on the page that lists the lexer's inputs.
- **Line 4.** Message words by file: `lexer/mod.rs` 27; `parser/items.rs` 33, `parser/placements.rs` 30, `parser/decls.rs` 28, `parser/blocks.rs` 21, `parser/loops.rs` 14, `parser/mod.rs` 13, `parser/expressions.rs` 12, `parser/types.rs` 10, `parser/json.rs` 8, `parser/lambdas.rs` 8.
- **2.1.** `parser/mod.rs:1094-1126` with `:451` and `:822`; `lexer/mod.rs:590-597` with `:545` and `:655-660`.
- **2.2.** `groups`, a map from a token position to a group's closer, a field of the parser, read at lookahead. The likely source of the lexer's allocations, `line_stack_snapshot` at `lexer/mod.rs:1337`, is read and not measured.

## The old tests

12 files, 4,218 lines, 23 tests of which 9 are ignored. None names a SPEC sentence, and every one is code and not a case file (4.8).

| File | Lines | Tests, ignored | Lines it breaks |
|---|---|---|---|
| `front_end_goldens.rs` | 406 | 11, 7 | 4.1: digests recorded from a run; 4.7; it imports `semantics::ir` |
| `parser/tests/line_shape.rs`, `generated_file.rs` | 190, 516 | support for the next three | 4.1: they restate the recovery rules and compute the expected messages from them |
| `parser/tests/fault_catalogue.rs`, `single_faults.rs`, `repeated_faults.rs` | 2,025, 187, 45 | 3, 1 | Written by hand to recovery rules the SPEC did not state; 4.7 |
| `parser/tests/valid_items.rs` | 164 | support | It checks only that no message is drawn |
| `ast/tests.rs` | 106 | 1, 0 | One tree built by hand, read through a few accessors |
| `cost_tests.rs`, `parser/tests/kind_reads.rs`, `lexer/tests.rs` | 383, 114, 43 | 8, 1 | Cost bounds; whether they state the design's cost or the day's (2.6) was not read; 4.7 |

## Not counted

- Fields and parameters that carry an id as a bare `u32` or `usize` (line 8); loops whose turn is not constant (2.5), since none states anything; branches without a source beyond the lists above (3.2), which needs each branch read against its sentence.
- The syntax tests under `crates/compiler/tests/` (`lexer.rs`, `lossless.rs`, `productions.rs`, `recovery.rs`, `bracket_line_breaks.rs`, `keywords.rs`, `views.rs`, 70 tests) are outside the kept code's own 4,218 lines.
