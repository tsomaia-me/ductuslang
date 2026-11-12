# Peer notes

## Now

- Compiler work is paused and no builder runs.
- The design pass is open. This peer has read the four documents and the front end's interfaces and waits on the owner's go; nothing is designed, nothing is measured and no agent runs.
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

## Open

- Before the owner since 2026-11-12, unanswered:
  - The policy for broken programs: a placeholder that silences whatever reads it, a placeholder that keeps what the fault leaves known, or the values a placeholder could hold tried as before. The peer's pick is the first.
  - What the design covers and where it ends: the whole SPEC through the Program, the whole SPEC through the checks, or only the language the last build carried. The peer's pick is the first, built in slices.
  - Which moment the speed promise covers: every compile cold and whole, cold with files parsed side by side, or the edit loop with an unchanged file's tokens, tree and IR kept. The peer's pick is the third.
- Whether the speed targets are reachable: lexing, parsing and the IR alone take about the whole inlined-call target. No test times the front end by the clock; its cost tests count work. The lexer and the parser have no stated share of any budget.
- What is taken from the old code and tests, as cases and never as a yardstick.
- Whether the parser reopens for patterns that stand in for wires. It tells a wire placement by the vocabulary's wire kinds alone (`syntax/parser/mod.rs`, `at_wire_kind`) and parses one file with no other file's declarations, so a use of such a pattern (SPEC §7.1) parses as a node placement with a body, and a destination that is a path, an `if`, an indexed array or a `match` has no parse there. Read from the code, not run.
- What "keep the IR" keeps. The flat IR draws `unbuilt-construct` for about forty constructs: every type but a scalar and an array of them, `type` declarations, records, sums, maps, documents, lambdas, bracket parameters, clocks, parameter markers and annotations, cells in patterns, `=>` and the package items. 25 SPEC examples are listed pending. A second, tree-shaped IR (`Hir`) is built from the flat one for the old engine.
- Whether ARCHITECTURE §3 to §6 stand as what the compiler hands the runtime. Core holds a `Program` and no runtime.

## Owed

- Before the design relies on them: one cold review of the lexer and the parser against the SPEC's grammar and layout rules; the last review read one file of them. The peer proposes one reviewer on Fable at high effort for the grammar, the layout, the tree and the tests, and one on Opus at high effort for what recovery must hold under the settled rules; where recovery cuts waits on the policy for broken programs.
- The speed floor of the lexer, the parser and the IR build, by the clock, on one pinned fast core (the host's CPU has fast and slow cores), release and `dev`, over the large fixture project, the valid corpus programs, generated shapes at doubling sizes and broken input.
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
- The last plan's speed targets: `docs/PLAN.md` § Reviews in the same tree, the table alone.
