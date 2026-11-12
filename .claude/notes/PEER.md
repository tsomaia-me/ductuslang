# Peer notes

## Now

- Compiler work is paused and no builder runs. The commit loop (`head-tools/commit-loop.sh`, pid 1260982 on `wb`) idles.
- Next: a fresh peer runs the design pass for the semantic core with the owner.
- The old records stay in `.memory/symphonyscript/`, an archive nothing in the harness reads; `work/peer-handoff.md` there says where the compiler stopped.

## Settled for the compiler

- A broken program is judged by the SPEC plus two rules: every fault draws its own message, and one fault draws one message. Equality with the old engine is no yardstick.
- The semantic core is rebuilt to a design. The lexer, the parser and the IR stay, code is salvaged where it fits, and both engines stay in the tree as reference, never as a gate.
- The design pass comes first and the owner approves it: the stages and what each owns, the error policy for broken programs, evaluation and inference, the fit for the runtime and the remaining milestones. It rewrites ARCHITECTURE's compiler section; the SPEC is the owner's and stays.
- Speed is proved first: the front end's floor on the benchmark programs, then a thin slice of the core, then a budget per stage kept as test rows.
- The work runs as loops shaped like the owner's plan and build loops, with a test loop before the build, a prober beside the reviewers, a round budget that stops a unit and goes to the owner, speed rows in the gate, and steps written as short lines.
- A test names the SPEC sentence it proves. Its author is not the builder and writes it first. A builder's commit that edits an expected value is refused. Each test is shown to fail once. Cases come from a rule's boundaries. No expected value is copied from the implementation. Progress is counted in rules passing.
- A builder reads the SPEC whole, its unit's design page, its task with its tests and the coding guidelines, and runs its unit's tests. The second host runs the full suite on every commit. The wave barrier runs everything and the robustness corpus.
- A ruling goes into the SPEC or a test. A finding is fixed in its stage or becomes a PLAN line of one line. A stage does not close with open entries.
- A probe that exposes a wrong answer becomes a stated test; a passing probe is thrown away.
- Sessions rotate, the peer's included. Agent definitions carry model and effort.

## Open

- Whether the speed targets are reachable: lexing, parsing and the IR alone take about the whole inlined-call target.
- How much machinery broken programs deserve; the owner sets that policy in the design pass.

## Owed

- Agent definitions: builder, reviewer, verifier, prober, test author.
- The loops written out: design, test, build.
- Before the old engine goes: expected answers written into the parity-only tests, about 350.
- The old records: sort the 137 open ledger entries into built, closed by the yardstick, real defect, PLAN line; distil the decisions record to one line a ruling; then close both.
- Defects the review found: the unit rule over `min`, `max`, `clamp` and `lerp`; two quadratic paths, in `locals.rs` and `coverage.rs`.
- To the PLAN: a builder wraps up at 400,000 tokens (`context-watch.mjs` still says 600,000); the builder-wait sentence goes, it is a preference; night hours 18:00Z to 02:00Z with light checks only; cargo runs at `nice -n 19` and 6 jobs.
- To SPEC §5.6: a read of an attribute in a unit family is a value of that family.
- The head and consultant skills and the agent files still name `.memory/` and `pnpm lesson`.
- The test baselines sit in `.memory/symphonyscript/baselines/`; they belong beside the review tool or nowhere.

## Read

- `.claude/reference/compiler-review.md`, the independent review of 2026-11-12; delete it once the design pass has used it.
- `.claude/worktrees/docs/packages/symphonyscript/docs/SPEC.md`, whole.
