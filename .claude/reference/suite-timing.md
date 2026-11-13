# What the test suite costs a builder, at `6491d802`

Measured by the peer session on 2026-11-13 on this host (turbo off, load about 3), in a private copy since deleted, `dev` profile (`opt-level = 1`), 6 jobs, `nice -n 19`, one run each. The raw rows are `/home/cdagent/cargo-targets/suite-timing-raw.txt`. No figure here is compared with a target.

## The suite

- 70 test binaries: the compiler's unit tests, 53 files under `crates/compiler/tests/` (41,188 lines of Rust, 680 tests), and the other crates'. 1,307 tests run and 293 are ignored: 166 marked `exhaustive`, 125 `stress`, 2 with no reason.
- A cold build of every test: 325 s.
- A run of everything built: 14.4 s, of which the compiler's unit tests take 5.6 s. The first run takes 45.3 s, because `tests/compile_fail.rs` (trybuild) makes a nested build of 16 s and then runs 31 s in all.

## After an edit of one statement in one parser function

| What is rebuilt | Seconds |
|---|---|
| The compiler crate alone | 6.8 |
| One small test file (34 lines) | 7.4 |
| One large test file (`typing.rs`, 6,751 lines) | 7.5 |
| The same, built and run (88 tests, 0.5 s) | 7.2 to 7.5 |
| All 53 test files | 14.5 |
| The crate's unit tests (the crate built again under `cfg(test)`) | 15.9 |

## After an edit of a test file alone

- The 34-line file: 0.7 s. The 6,751-line file: 5.2 s.

## The 53 files as one binary

- Each file made a module of one test binary (`#[path]`, `autotests = false`). After a one-line change of the crate (a comment): 17.7 s, against 11.2 s for the 53 separate binaries after the same change. The run of its 504 tests takes 12 s beside trybuild's nested build.

## What follows

- One link costs about 0.6 s; 53 links add about 7 s at 6 jobs. The link is not the main cost.
- The crate's own rebuild is the floor of every check, 6.8 s, and test code that compiles is the next cost: joined into one binary it compiles as one crate and is slower, and a case edit in a large file costs 5 s.
- A unit test inside the crate costs a second build of the whole crate, 15.9 s an edit.
- Not measured: the compiler split into a crate a stage; the stated tests as case files read by one harness.
