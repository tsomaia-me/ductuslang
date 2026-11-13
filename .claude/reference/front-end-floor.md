# The speed floor of the lexer, the parser and the IR build, at `6491d802`

Measured by the peer session on 2026-11-13 and checked cold by one agent on Opus. The bench, its inputs and the raw rows are under `/home/cdagent/cargo-targets/peer-syntax-review/floor/` (`packages/symphonyscript/crates/compiler/examples/floor.rs`, `src/syntax/floor.rs`, `make_inputs.py`, `run.sh`, `inputs/`, `results/`, `check/`); the build folders are deleted.

## Conditions

- Host: Intel Core Ultra 5 125H. Turbo is off (`intel_pstate/no_turbo` is 1, governor `powersave`, preference `balance_power`); the checker timed the measured core at 1.98 GHz against a rated 4.5 GHz. Every time below is for that state. One nanosecond here is about two cycles. A run with turbo on was not made.
- Each run is one process pinned to cpu 6. Its sibling thread, cpu 7, was not kept free, and two reviewers ran at `nice 19` on cpus 8 to 17. The smallest of the repeated runs is taken, over three processes for release. Load moves the figures up only; the rounds differ by 0 to 3 % on most rows and up to 14 % on the corpus rows.
- `perf` counters are closed (`perf_event_paranoid` is 4), so no instruction counts.
- Release is the workspace's release profile. `dev` is its `opt-level = 1`.
- The stages: `lex` is the lexer alone to its columns; `parse` is `syntax::parse`, the lexer and the parser together, the result kept and dropped outside the clock; `ir` is `build_flat_file` over trees parsed outside the clock, into an empty IR with no prelude layer, the tree's lazily derived columns included.

## Figures, release

| Input | Size | `lex` | `parse` | `ir` | `parse` + `ir` |
|---|---|---|---|---|---|
| The large fixture project | 31 files, 319 KB, 10,413 lines, 64,918 syntax nodes | 5.0 ms, 77 ns a node, 35 ns a token | 16.6 to 16.8 ms, 256 to 259 ns a node | 13.8 to 15.3 ms, 213 to 236 ns a node | about 31 ms, 470 to 490 ns a node |
| The corpus programs that parse clean | 4,782 files, 2.6 MB, 592,769 nodes | 45 ms, 76 ns a node | 144 ms, 244 ns a node | 137 ms, 230 ns a node | 474 ns a node |
| The corpus programs with a syntax fault | 15,511 files, 9.3 MB, 1,599,127 nodes | 42 ns a token | 557 ms, 348 ns a node | 508 ms, 318 ns a node | 666 ns a node |
| The last plan's inlined-call program, 1,000,000 calls written out | one file, 13.6 MB, 10 tokens and 6 nodes a call | 0.26 to 0.31 µs a call | 1.14 to 1.24 µs a call | 0.90 to 1.02 µs a call | 2.04 to 2.26 µs a call |

- Where a range is given, the lower figure is with glibc kept from mapping and unmapping its large blocks on every run (`GLIBC_TUNABLES`), which matters for one very large file and not for many small ones.
- Against the last plan's targets, on this host as it runs: `parse` and `ir` together take about 0.48 µs of its 1 µs a syntax node for a whole compile, and 2.0 to 2.3 µs against its 1.5 µs an inlined call; `parse` alone takes 0.26 µs a node and 1.1 to 1.2 µs a call.
- Generated shapes at 80,000 lines, `parse` in ns a node: placements 192, blocks 201, document literals 211, expressions 227, functions 241 to 253, strings 251, non-ASCII identifiers 317. The lexer runs at 44 to 82 MB/s on ASCII and 38 MB/s on non-ASCII identifiers. These single large files carry the allocator's cost above, 5 to 27 % depending on the stage.
- `dev` against release: `parse` 1.3 to 1.5 times, `lex` 2.2 times, `ir` 1.2 to 1.3 times.
- Memory: the tree peaks at 48 to 82 B a syntax node with its derived columns; the IR adds 60 to 100 B a node. The lexer allocates 17 to 21 times a small file and about 5,000 times for the 319 KB fixture as one file.

## What the check found, and what follows

- The work of `lex` and `parse` is linear in the input across four doubling sizes of every shape, within 5 %. The IR build is not shown linear: its cost a byte grows 9 to 23 % over an eightfold input with the allocator's effect removed.
- The compiler parses, builds and drops one file at a time; the bench parses every file and then builds. In the compiler's order `parse` is 7 to 11 % slower on many-file inputs and `ir` 0 to 3 % faster.
- The compiler's IR starts after the prelude's and adapts each file for the old engine; neither is in `ir`.
- The IR builds placeholders for annotated functions and document literals, so their `ir` rows are not the cost of building those constructs and are left out above.
- The rows `scan_newlines` and `chars_pass` in the raw files are no machine floor and are not used.
- The checker recomputed five figures by hand from the raw rows; all matched.
