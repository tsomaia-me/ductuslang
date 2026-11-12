# Independent review: SymphonyScript's compiler at `6491d802` (branch `symphonyscript/r3`)

Reviewer: one agent on Fable at high effort, given the code, the three documents and the owner's two questions, with no hints. The text below is its report, word for word, saved by the peer session because the reviewer could not write files.

Every figure is from the code, the git history or a run of the reviewer's own, unless marked as the builders' record. "Verified" means it saw it in code or in a run; "inferred" means it reasons to it.

Code paths are under `/home/cdagent/cargo-targets/r3/wt/packages/symphonyscript/crates/compiler/`, written `…/compiler/` below. The new core is `…/compiler/src/semantics/pipeline/`. The old engine is `…/compiler/src/semantics/analysis/`.

---

## 1. Is the work of real quality?

**The compiler as a whole behaves well. The new engine inside it is built poorly, and its tests prove less than they appear to.**

- **Good, and I am sure of it.**
  - Ordinary programs get the right answer: 53 hand-written programs from the SPEC drew what the SPEC says.
  - The front end and the flat IR are clean and match the design.
  - The 59 integration test files under `…/compiler/tests/` state expected errors from the SPEC; they are the real proof of correctness.
  - Hygiene is good: 1,307 unit tests pass, formatting and lints are clean, there is no `unsafe` and no panic path, and a 100,000-deep expression compiles on a 1 MiB stack on both engines.
- **Good, fairly sure.**
  - On valid programs the new engine is 1.4 to 3.6 times faster than the old one on most shapes I timed, and 16 times on loop expansion.
  - It fixes old-engine defects (finding 11).
- **Poor, and I am sure of it.**
  - The new engine has wrong answers in unit handling that no project check caught: it rejects the valid line `Note | length=min(3, 2bars + 1)`.
  - It has two places where work grows with the square of the input.
  - It is one object with 253 fields that every stage changes.
  - The dense layout it was rebuilt for was fitted on day 5, not designed in.
  - It misses its own speed targets by 2.5 to 5 times.
  - About 80% of its own tests assert "the same as the old engine" and state no expected result. The plan deletes the old engine, and those tests lose their meaning that day.
  - The safety net between the engines is a 9,430-line list of exceptions decided by text matching.
- **How sure overall.** High for everything I ran. Medium for the design judgement: I read about a sixth of the new engine's 39,000 lines closely and sampled the rest by structure.

## 2. Why has the rebuild taken longer than the first build, and got less far?

**The fact is true.** From the git history:

| | Commits in `crates/` | Clock hours with a commit | Calendar | State |
|---|---|---|---|---|
| Old engine's semantics (M2 + M2b + M2e) | 377 | 132 | 24 Oct to 2 Nov | complete for the language |
| New front end (R1) | 3 | 2 | 3 Nov | done |
| New IR (R2) | 4 | 2 | 3 Nov | done |
| New semantic core (R3) | 615 | 146 | 3 Nov to 12 Nov | unfinished; recursion, the twin pass and the old engine's deletion not started |

**What is wrong is the target, not the builders' speed.** Six causes, the first three the heavy ones:

1. **The new engine must match the old one message for message, mostly on broken programs.** Of the project's 20,293 corpus programs, 727 compile without error. So most of the work is "which second and third message appears beside an error". Of R3's 624 commit titles, 434 speak of a diagnostic, a fault or a placeholder, and 267 name the old engine.
2. **The thing being matched keeps being overruled.** The old engine is called the oracle, but where the two disagree the new engine is often ruled right and an exception is written. There are now 117 exception classes and 323 listed programs, and 174 of R3's commits edit that list. In my run the new engine draws a different set from the old one on 8,765 of the 19,848 corpus programs it takes (44%). "Equal to the old engine" is no longer a target anyone can check.
3. **A program goes to the new engine whole or not at all.** Every construct must be right in every combination, or be detected and declined up front. The places in the code that decline a program rose on every day of work: 24, 70, 113, 124, 157, 172, 175, 176, 189. The tail is not converging.
4. **The plan's size was wrong by 2.5 times or more.** The builders' design estimated R3 at 14,000 to 16,000 lines in six or seven steps, and the whole rebuild at 4 to 9.5 days. R3 is 39,240 lines after nine days and passed 16,000 on its third day. By the builders' launch file it is on its 62nd builder session. The same design says to bring the owner the numbers at twice the estimate.
5. **The layout that justified the rebuild was added afterwards.** The core was first written with 79 hash maps keyed by node, then swapped for column wrappers on day 5. Allocation was removed on days 9 and 10 by a second builder working in the same files.
6. **The code's shape makes each next step dearer.** One 253-field object, files of 2,000 to 3,000 lines, a 938-line function, no comments by rule. Each fresh session must relearn it and is told never to read a file whole. Inferred from the builders' notes: recent sessions land one to three commits each.

The first build was different work: it grew feature by feature, each with tests stating the right answer. The rebuild chases "no difference" against about 108,000 programs at once.

## 3. Recommendation: change course. Do not stop, and do not continue as it is going.

**Keep** the new front end, the IR and the new core. They work, they are faster, and 98% of the corpus already runs on the new core.

**Change four things:**

1. **Change the yardstick.**
   - For a valid program: the same verdict and warnings as the old engine, with no exceptions.
   - For a broken program: still rejected, with the same first error at each faulty place. Stop matching the secondary messages.
   - *Cost:* some broken programs show different extra messages than today. The owner accepts that once instead of ruling on it case by case.
   - *Gain:* most of the exception list and most of the planned "faulty programs" step disappear.
2. **Turn the parity tests into stated expectations before the old engine goes.**
   - Record what each of the roughly 350 parity-only tests expects, as codes and spans, while the old engine can still answer.
   - *Cost:* about a day, mechanical (inferred).
   - *Without it:* deleting the old engine silently deletes the proof.
3. **Give the core a shape before recursion is added to it.**
   - Split the 253-field object by stage, replace the mode switches with arguments, and write one map of what each field means. No behaviour changes, so today's tests judge it.
   - *Cost:* one to two days with nothing new to show (inferred).
   - *Without it:* recursion and the twin pass land on the same object. By the builders' design these are the parts that took the most rounds in the first build.
4. **Give recursion (R4a) a budget and a stop rule the owner holds.** Port the old recursion engine; do not rewrite it. Name a line count and a day count up front, and stop at twice either.

Also fix the unit bugs and the two quadratic paths below; each is small.

**What the other choices cost:**

- **Continue as it is going.** Inferred: at least as long again as R3 has taken, likely more. What remains is recursion, the twin pass, about 7,000 declined broken programs (the builders' count) and a seven-reviewer cold review. The end state is a 50,000 to 60,000-line engine as hard to change as the old one, with the language's next milestones waiting throughout.
- **Stop and go back to the old engine.** Throws away nine days and a core that is 2 to 16 times faster. The old engine keeps its own defects (finding 11). Not justified.
- **Keep both engines for good.** Every later language feature is built twice or forces the fallback. Not justified.

---

## 4. Findings, most severe first

### 1. The new core rejects valid programs that use units (verified by run)

- **Where:** `…/compiler/src/semantics/pipeline/families.rs`, `plain_in_family` at line 585; the diagnostic is drawn at line 826. I did not find the exact faulty branch.
- **What:** `Clip c:` / `    Note | length=min(3, 2bars + 1)` draws `plain-in-family` on the new core and nothing on the old engine.
  - The same happens for `length=clamp(3, 1bars, 2bars + 1)`, `length=clamp(1, 2bars + 1, 3)`, and the same three with `hold=` and seconds.
  - `max(2bars + 1, 3)` is accepted, so the fault depends on argument position.
- **Rule:** SPEC §3.6 says `min`, `max` and `clamp` take operands of one family, and a plain number among them is read in the attribute's default unit. The program is valid.
- **Why it matters:** by the project's own scale (PLAN § Reviews) a valid program rejected is the worst class of defect. One such program, key `bc10ded75aa342a2`, is in the project's own corpus. I found no record of it in the ledger, the decisions record or the progress file.

### 2. The new core accepts programs the SPEC rejects, in the same area (verified by run)

- **Where:** same file and function.
- **What:** with `state span(1bars)`, the line `Note/60 when max(span, 1) > 2bars` draws nothing on the new core. The old engine draws `plain-in-family` at `1`.
  - The same holds for `min`, `clamp` and `lerp`.
  - Both engines reject the plain form `span + 1 > 2bars`, so the new core disagrees with itself.
- **Rule:** SPEC §3.6: where the family value ends in a comparison, the plain value is an error.
- **Why it matters:** a wrong program accepted. One such program, key `e56f4b3c3619d9aa`, is in the project's own corpus, also unrecorded. Of my 95 generated probes, 66 on units, 11 showed a wrong verdict from the new core. Units were the last slice built.

### 3. The check between the two engines is a growing list of exceptions decided by text matching (verified)

- **Where:** `…/compiler/tests/reviewed/mod.rs`, 9,430 lines.
  - 26 pinned programs from line 7.
  - 297 programs "carried to R4b" from line 232.
  - 111 exception classes from line 6504, and 6 more for the twin pass from line 7481.
- **How it decides:** each class is a function that looks at the program's text. One recognises a unit attribute by the four names `.hold`, `.at`, `.fade`, `.length` (line 4818), which belong to the example vocabulary.
- **Growth:** fewer than 10 classes on day 1, 117 at the tip, more on every day of work. 99 of the 111 sit under one ledger number, 476.
- **Where it runs:** the wide comparisons that use it are marked `#[ignore]` (`…/compiler/src/semantics/pipeline/differential_tests.rs` lines 179 to 193) and are not in the default suite. They skip any program the new core declines (line 119).
  - On the robustness corpus, run by me: the new core took 87,756 programs, and 4,866 were excused by a class.
  - That test took 234 s in the dev profile on the shared host. PLAN § Work and checks caps an exhaustive test at 30 s.
- **Gap:** by a search of `/home/cdagent/cargo-targets/r3/wt/packages/symphonyscript/tools/evolve/src/`, no tool compares the two engines over the 20,293-program corpus. Only `bin/evolve_scale.rs` line 45 selects the old engine.
- **Why it matters:** this is the rebuild's safety net, and findings 1 and 2 went through it.

### 4. Most of the new core's tests prove "same as the old engine", not "right" (verified; the split is my count by pattern, approximate)

- **Where:** `…/compiler/src/semantics/pipeline/flip_tests.rs` lines 18 to 39. The helper compiles each program on both engines and asserts the results are equal.
- **Split:** of 436 tests in `flip_tests.rs` and `differential_tests.rs`, about 348 assert only that, about 52 also state the expected codes, and about 30 state codes alone.
- **Why it matters:** the plan's last step deletes the old engine. Those tests then cannot run as written, and about 14,000 lines of evidence go with them. Until then they also treat every old-engine mistake as "expected".

### 5. Two places in the new core cost work that grows with the square of the input (verified by run and in code)

- **Local names:** `…/compiler/src/semantics/pipeline/locals.rs` line 199 compares each local name with every earlier one.

  | Local `let`s in one function | New core | Old engine |
  |---|---|---|
  | 5,000 | 0.15 s | 0.05 s |
  | 10,000 | 0.53 s | 0.10 s |
  | 20,000 | 2.0 s | 0.20 s |
  | 40,000 | 8.0 s | 0.45 s |

- **Variable-array annotations:** `…/compiler/src/semantics/pipeline/coverage.rs` line 280 searches a list once per annotation.

  | Cells declared `: i32[..10]` | New core | Old engine |
  |---|---|---|
  | 40,000 | 0.45 s | 0.29 s |
  | 80,000 | 1.4 s | 0.63 s |
  | 160,000 | 4.6 s | 1.3 s |

- **Why it matters:** ARCHITECTURE §7 promises work linear in the program, and the rebuild exists for speed. Neither shape is in the cost tests, and I found neither in the builders' records.

### 6. The new core is one object that every stage changes (verified)

- **Size:** `…/compiler/src/semantics/pipeline/core.rs` lines 34 to 295. `Core` has 253 public fields, and 49 files add about 1,100 functions to it.
- **Switches:** several fields change what other methods do: `inst` (line 84, set in 85 places), `owning` (51), `unchecked` (58), `fitting` (59), `driving` (78), `wire_now` (150), `choosing` (153).
- **One function of 938 lines:** `evaluate_frames` at `…/compiler/src/semantics/pipeline/values.rs` line 697. At line 794 one number means a `let` below a threshold and an attribute read above it.
- **Untyped IR reads:** `…/compiler/src/semantics/pipeline/coverage.rs` lines 238 to 240 read IR nodes as bare number triples whose meaning depends on the operation (`shape / 2`, `start + 2 * at + 1`).
- **Against the design:** the builders' design says a stage owns its output and later stages only read it. The code does not do that.
- **Why it matters:** this is the old engine's shape again (its `Engine` has 71 public fields and its memo about 87 maps), only larger.

### 7. The dense layout was fitted on afterwards, and partly in name only (verified)

- **Retrofit:** commit `3c29e6a8` of 7 November says "79 HashMap, HashSet, BTreeMap and BTreeSet fields of `core.rs` moved" to column types.
- **Not the described layout:** `…/compiler/src/semantics/pipeline/column.rs` lines 157 to 196. The two-key column is a sorted list per node, searched by halving and inserted in the middle (line 191). ARCHITECTURE §7.2 describes a block of columns per typed body.
- **Tests pin today's cost:** `…/compiler/src/semantics/pipeline/speed_cost_tests.rs` lines 51 and 53 allow 18 allocations per inlined call and 6 per expanded item. ARCHITECTURE §7.2 says the per-node loops allocate nothing.
- **Why it matters:** the tests guard the present cost, not the design's.

### 8. The new core is faster than the old engine but well short of its targets (verified by my timing; release build on a shared host, so read these as rough)

| Work | New core | Old engine | Target |
|---|---|---|---|
| An inlined call | 7.8 µs | 22.8 µs | 1.5 µs |
| An expanded item | 1.9 µs | 30.6 µs | 0.7 µs |
| Corpus program the new core takes, median | 394 µs | 510 µs | 380 µs |
| Same, slowest 1% | 7.8 ms | 7.8 ms | 1.5 ms |

- The builders' own figure for an inlined call on a quiet host is 4.18 µs.
- The slowest 1% are equal because broken programs still use the old engine's twin pass.
- Other valid shapes at 40,000 lines, new core against old: placements 2.3 times faster, pattern uses 3.7, wires 1.7, derived-cell chains 1.9, paths 2.0, cells with history 1.1.
- **Why it matters:** the gain is real, and it is a fraction of the 16 to 53 times the builders' design drew from its trial slice.

### 9. The scaffolding between the engines is large and still growing (verified)

- **Fallback:** `…/compiler/src/compiler.rs` lines 247 to 286. The new core runs first; if any stage declines, its work is thrown away and the old engine runs. In my run a declined program costs about 10% more than the old engine alone.
- **Decline rules:** 189 decline sites in the new core. `…/compiler/src/semantics/pipeline/coverage.rs` is 879 lines of rules for what may enter, and 139 of R3's commits edit it.
- **Preludes:** `…/compiler/src/semantics/pipeline/analyze.rs` line 55 sends every program of a vocabulary with a prelude to the old engine.
- **Old engine still in every compile:** `…/compiler/src/compiler.rs` line 329 builds the old engine's tree for every program, and line 308 runs its twin pass for every broken program.
- **Valid programs that still fall back without recursion:** `fn pick(big): if big then [60, 64, 67] else [60, 64]` read as `pick(true).size`, and a compile-time `match` whose arms differ in length.
- **Why it matters:** the new core cannot stand alone yet, and what is left is fine-grained.

### 10. The rebuild is far past its own estimate (verified in git; the estimate is the builders' design, `/home/cdagent/ductuslang/.memory/symphonyscript/work/layout-design.md` §7.1 and §7.2)

- **Estimate for R3:** 14,000 to 16,000 new lines.
- **Actual:** 39,240 lines of code and 28,026 of tests. Code by day, in thousands: 7.8, 13.8, 22.4, 28.5, 32.9, 34.2, 36.0, 38.0, 39.2.
- The new core is now the size of the whole old engine (39,114 lines), which includes recursion and the twin pass.
- **Why it matters:** growth slowed after day 5 while the decline sites kept rising. That is what a long tail looks like.

### 11. Good: the new core fixes real old-engine defects (verified by run)

- `state r = (r.past(1, []) + [n])[^4..]` read as `r.capacity` draws `internal-error` on the old engine and compiles on the new core.
- A 100,000-deep array literal takes 39 s on the old engine and 0.1 s on the new core.
- **Why it matters:** going back to the old engine is not free either.

### 12. Good: the foundations (verified)

- All 81 valid corpus programs that fall back reach a recursive function, as the plan says they should.
- No hash map left in the core is iterated, so output order does not depend on hashing.
- The new core has no `unsafe`, no `panic!` and no `#[allow]`. A lookup that misses returns an internal error (396 sites) instead of crashing.
- No panic on any of the 20,293 corpus programs. In the 8,765 where the engines differ, neither draws an internal error.

### 13. Minor (verified)

- The same message text is written out in several places. "this value has the wrong type here; `bool` is expected" appears four times in the new core: `…/compiler/src/semantics/pipeline/check.rs` 834, `references.rs` 222, `typing.rs` 988 and 993.
- `…/compiler/tests/error_rules.rs` line 20 holds a comment, which the project's rule forbids.

---

## 5. What I read, what I ran, what I did not cover

**Read.**

- **Documents:** the package `CLAUDE.md`; PLAN § How the work runs and M1 to M2r; ARCHITECTURE §7, §7.1, §7.2; SPEC §1, §3.6, §4, §6.1 to §6.3.
- **Code, whole or most:** `compiler.rs`, `selector.rs`, `lib.rs`; in the new core `analyze.rs`, `core.rs`, `reason.rs`, `coverage.rs`, `column.rs`.
- **Code, in part:** `locals.rs`, `values.rs`, `typing.rs`, `families.rs`, `shape.rs`, `speed_cost_tests.rs`, the heads of `flip_tests.rs` and `differential_tests.rs`, `tests/reviewed/mod.rs` by structure and samples, `tests/support/mod.rs`, samples of `tests/typing.rs` and `tests/error_rules.rs`, the old engine's `Engine` struct, the flat IR's main file, `syntax/parsed.rs`.
- **Git:** the whole branch's log, with counts by milestone, day and file.
- **Builders' records, for question 2 only and unverified:** `notes.md`, the end of `peer-notes.md`, `work/layout-design.md` §0 and §7, `work/flip-implementer-launch.md`, and searches of `CARRY.md`, `M2r.md` and `DECISIONS.md`.

**Ran**, all in the private copy, every cargo command under `nice -n 19` with 6 jobs:

| Command | Exit | Result |
|---|---|---|
| `git archive 6491d802 packages/symphonyscript \| tar -x` | 0 | |
| `cargo build --workspace --all-targets` | 0 | 5 min 29 s |
| `cargo nextest run --workspace --no-fail-fast` | 0 | 1,307 passed, 293 skipped (ignored), 45 s |
| `cargo fmt --check` | 0 | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | clean |
| `cargo build -p symphonyscript-core --target thumbv7em-none-eabihf` | 0 | |
| `cargo nextest run -p symphonyscript --lib --run-ignored ignored-only` over the two exhaustive differential tests of the SPEC, lesson, fixture and parser-test programs | 0 | 2 passed; the new core took 378 and 736 programs, 5 and 7 excused by a class |
| the same over `exhaustive_differential_the_robustness_corpus` | 0 | passed in 234 s; 87,756 taken, 4,866 excused |
| `cargo build -p symphonyscript --example probe`, dev and `--release` | 0 | a small program of my own, in the private copy only, that runs sources on both engines through the public `Compiler::select` and `check_by_engine` |
| `probe files` over 168 programs of my own (73 hand-written, 95 generated; the generated file run twice after I fixed a mistake of mine in it) | 0 | findings 1, 2, 9, 11 |
| `probe corpus` over the 20,293 programs of `tools/evolve/corpora/breakers.jsonl.gz`, dev once and release twice | 0 | 19,848 taken by the new core, 445 fell back, no panic; 8,765 differ from the old engine; 2 verdicts differ |
| `probe time` over 17 generated shapes at four doubling sizes, plus inlined calls and expanded items at three sizes, release | 0 | findings 5 and 8 |
| `probe time` over six deep-nesting programs under `ulimit -s 1024` | 0 | no overflow on either engine |

**Did not cover.**

- The `core` crate and the vocabulary builder, beyond their tests passing.
- The parser and lexer, beyond one file and their tests passing.
- The old engine's code, beyond its main struct.
- About five sixths of the new core line by line. I know `paths.rs`, `loops.rs`, `uses.rs`, `calls.rs`, `check.rs`, `cycles.rs`, `groups.rs`, `capacities.rs` and others only by size and by behaviour in runs.
- `tools/evolve` beyond one search, and `tools/carry`.
- The 290 other ignored tests, doc tests, and memory use.
- Programs in more than one file, vocabularies other than the example one, and the lessons.
- Whether each of the 117 exception classes is right by the SPEC. I judged only the two verdict changes I found.
- The builders' ledger and decisions, beyond searches.

---

## The same in plain words

- **The compiler gives right answers on normal programs.** I wrote programs from the manual; it accepted the good ones and refused the bad ones with the right message. Example: `Note/300000000000` is refused as too large for the pitch.
- **The new engine is faster, but not as fast as promised.** Example: a loop that places 200,000 lamps takes 0.4 s on the new engine and 6 s on the old one. The target was about 0.14 s.
- **The new engine has a few wrong answers nobody noticed.** Example: `length=min(3, 2bars + 1)` is a correct line and the new engine refuses it. The old engine accepts it.
- **The rebuild is slow because it copies the old engine's every message on broken programs.** Example: for one mistake the old engine may print two messages and the new one three. Someone then decides which is right, writes an exception and tests it. That has happened more than a hundred times.
- **The new engine's tests mostly say "same as the old engine".** Example: a test compiles a program on both and checks the answers match. Once the old engine is deleted, that test has nothing to compare with.
- **The new engine's code is one very large box.** Example: one object holds 253 named values and every step reaches into it, so a newcomer cannot tell which step owns which value.
- **What I would do.** Keep the new engine. Stop copying the extra messages on broken programs. Write down what each test expects before the old engine is removed. Split the big box into smaller ones. Then add recursion with a fixed budget, and stop if it doubles.
