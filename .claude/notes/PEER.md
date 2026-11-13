# Peer notes

## Now

- Compiler work is paused and no builder runs.
- The design pass runs since the owner's go of 2026-11-13. Nothing is designed yet.
- The two cold reviews of the lexer and the parser and the speed floor are done (2026-11-13) and saved: `.claude/reference/syntax-conformance-review.md`, `syntax-recovery-review.md`, `front-end-floor.md`. The reviewers' probes, cases and raw rows stay under `/home/cdagent/cargo-targets/peer-syntax-review/`; every `target` there is deleted. No agent runs.
- What they found, in short:
  - The foundations hold: layout but one sentence, the lexical rules, precedence, JSON mode, a lossless tree, and no panic, hang or stack growth on about 700,000 broken inputs.
  - The parser refuses twelve families of valid programs (one a SPEC `sym` example), accepts two invalid forms, and five typed views answer wrong. They cluster in what was never built after the parser: qualified names, bracket arguments, patterns that stand in for wires, keyword attribute names, `json` as a member.
  - Two paths cost time that grows with the square of the input: lines that each leave a nested group open (4,000 lines take 12 s), and one `{` left open a line, valid input included.
  - The two message rules break in recurring shapes: a second fault on a line is silent, a mis-indented line or a tab draws two messages, a missing `let` or `fn` draws one a line of its block, and a block under any faulty line vanishes silently with its declarations.
  - The SPEC has thirteen gaps or contradictions for a parser, chiefly: a wire-pattern line and a type's first bracket list need knowledge a parser of one file lacks, and §5.3 contradicts §5.1 on the right operand of `is`.
  - The module has no test that states a tree, a layout sequence or a precedence.
  - Speed, on this host as it runs, turbo off at about 2 GHz: lexing and parsing 0.26 µs a syntax node and 1.1 to 1.2 µs a written call; the IR build 0.21 to 0.24 µs a node and 0.9 to 1.0 µs a call; against the last plan's 1 µs a node and 1.5 µs a call for everything. The IR build is not shown linear.
- Running since 03:20Z on 2026-11-13: the floor's re-run on wb2 (`ssh wb2`, `~/cargo-targets/peer-floor/`, `run-wb2.sh`, rows under `results-wb2/`, `DONE` when it ends; it deletes its `target`), and one agent on Opus sizing the parser's repair finding by finding, returning text.
- Next from the peer: the SPEC rulings as one batch (the gaps of the conformance review and the rules for a syntax fault), then the repair's size, then the owner chooses repair or rewrite.
- The old process's commit loop still runs on `wb` (pid 1260982, alive on 2026-11-12 at 17:53Z) with nothing to check; stop it on the owner's word.

## Settled

- A broken program is judged by the SPEC plus two rules: every fault draws its own message, and one fault draws one message. Another implementation is never the yardstick.
- The lexer and the parser stay. The IR is the design pass's to keep, reshape or replace, with its reason. Everything after the IR is rebuilt to the design. The lexer and the parser reopen if they alone exceed their share of the speed budget, or if the error policy for broken programs changes their recovery rules.
- A design pass comes first and the owner approves it. The SPEC is the owner's and stays. ARCHITECTURE was written by agents and is rewritten.
- Speed is proved first: the floor of the stages that exist, then a thin slice of the design, then a budget per stage kept as test rows.
- The work runs as loops shaped like the owner's plan and build loops, with a test loop before the build, a prober beside the reviewers, a round budget that stops a work unit and goes to the owner, speed rows in the gate, and steps written as short lines.
- A test names the SPEC sentence it proves. Its author is not the builder and writes it first. A builder's commit that edits an expected value is refused. Each test is shown to fail once. Cases come from a rule's boundaries. No expected value is copied from the implementation. Progress is counted in rules passing.
- A builder reads the SPEC whole, its work unit's design page, its task with its tests and the coding guidelines, and runs its work unit's tests. The second host runs the full suite on every commit. The wave barrier runs everything and the robustness corpus.
- A ruling goes into the SPEC or a test. A finding is fixed in its stage or becomes one PLAN line. A stage does not close with open entries.
- A probe that exposes a wrong answer becomes a stated test; a passing probe is thrown away.
- Sessions rotate, the peer's included. Agent definitions carry model and effort.
- Broken programs (owner, 2026-11-13): a fault leaves a placeholder with no type and no value; a check that needs a fact the placeholder holds draws nothing, and a fault that does not read it still draws. No type is kept behind a fault and no value is tried. The SPEC gains this rule.
- Scope (owner, 2026-11-13): the design covers the whole SPEC, source to Program, and is built in slices.
- The front end (owner, 2026-11-13): ids are local to each file, and the parser needs no other file. Nothing else about reuse or side-by-side parsing is chosen.
- Speed (owner, 2026-11-13): the promise is stated for the second host, wb2, where the gate's speed rows run. A time taken on this host is never compared with a target. The last plan's two targets disagree, 1 µs a syntax node against 1.5 µs for a call of five or six nodes, and neither stands. The promise is restated as what a user feels, a project of a stated size checked in under N milliseconds on wb2, and the budgets a node derive from it; N is a product call still to make, after the thin slice. The front end stays inside the judged figure.
- The parser (owner, 2026-11-13): no choice between repair and rewrite yet. First the SPEC's gaps and the rules for a syntax fault are settled, then the repair is sized, each finding mapped to the code it touches, with the lexer and layout counted apart from the parser, the recovery and the views. The lexer and layout are kept either way. If the repairs touch more than about half of the rest, that part is rewritten against stated trees, and the 700,000 broken inputs stay as its robustness net.

## Open

- What is taken from the old code and tests, as cases and never as a yardstick.
- The edges the placeholder rule leaves for the SPEC, to bring as one batch with examples: a name a faulty declaration declares, a name declared twice, structure that depends on a placeholder (a range bound, a use's argument), the vocabulary's counts beside a faulty placement, a limit's stop, and warnings in a program that draws errors.
- What ids local to each file ask of the IR: today one id space and one symbol table run across the prelude and every file (`FlatIr::after`, `Symbols::layer`), so the IR is reshaped at least there.
- What "keep the IR" keeps. The flat IR draws `unbuilt-construct` for about forty constructs: every type but a scalar and an array of them, `type` declarations, records, sums, maps, documents, lambdas, bracket parameters, clocks, parameter markers and annotations, cells in patterns, `=>` and the package items. 25 SPEC examples are listed pending. A second, tree-shaped IR (`Hir`) is built from the flat one for the old engine.
- Whether ARCHITECTURE §3 to §6 stand as what the compiler hands the runtime. Core holds a `Program` and no runtime.

## Owed

- To the SPEC, the owner's to approve: the placeholder rule for broken programs, with its edges (see Open).
- To the SPEC, the owner's to rule: the thirteen gaps of `syntax-conformance-review.md` § E, as one batch with examples.
- Agent definitions: builder, reviewer, verifier, prober, test author.
- The loops written out: design, test, build.
- To the SPEC: §5.6, a read of an attribute in a unit family is a value of that family (ruled). A plain label on a `match` over a unit value is accepted today and reads as refused (not ruled).
- A later job, not the design pass's: read the old ledger and decisions record once for places where the SPEC needed a ruling, and settle each in the SPEC.
- Host rules for any build here: cargo at `nice -n 19` and 6 jobs; nothing heavy from 18:00Z to 02:00Z.
- The syntax module imports the old engine's `semantics::analysis::fast_hash` and `semantics::counted_walk`; both move before the old engine goes.
- `minus.txt` at the repository's root, untracked, is an old comparison run's output of 5 November; remove it on the owner's word.
- Harness debts: `context-watch.mjs` tells a subagent to wrap up at 600,000 tokens and the owner's last figure was 400,000; the head and consultant skills, the agent files and the root `CLAUDE.MD` describe the old layout; one test of `settings.test.mjs` and 22 of `memory-push.test.mjs` fail; the status line names a missing `usage-status.mjs`.

## Read

- `.claude/worktrees/docs/packages/symphonyscript/docs/SPEC.md`, whole.
- `.claude/reference/compiler-review.md`: what went wrong in the last build and what it measured.
- `.claude/worktrees/docs/packages/symphonyscript/docs/ARCHITECTURE.md`, whole, as the last design and no authority; its §3 to §6 state what the compiler hands the runtime.
- The lexer, the parser and the IR as they stand: `crates/compiler/src/syntax/` and `src/semantics/ir/` under `/home/cdagent/cargo-targets/r3/wt/packages/symphonyscript/`, by their public interfaces.
- `.claude/reference/syntax-conformance-review.md`, `syntax-recovery-review.md` and `front-end-floor.md`: the reviews and the measurement of 2026-11-13.
- The last plan's speed targets: `docs/PLAN.md` § Reviews in the same tree, the table alone.
