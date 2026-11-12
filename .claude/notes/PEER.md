# Peer notes

## Now

- Compiler work is paused and no builder runs.
- Next: a fresh peer runs the design pass with the owner.
- The old process's commit loop still runs on `wb` (pid 1260982) with nothing to check; stop it on the owner's word.

## Settled

- A broken program is judged by the SPEC plus two rules: every fault draws its own message, and one fault draws one message. Another implementation is never the yardstick.
- A design pass comes first and the owner approves it. The SPEC is the owner's and stays. ARCHITECTURE was written by agents and is rewritten.
- Speed is proved first: the floor of the stages that exist, then a thin slice of the design, then a budget per stage kept as test rows.
- The work runs as loops shaped like the owner's plan and build loops, with a test loop before the build, a prober beside the reviewers, a round budget that stops a work unit and goes to the owner, speed rows in the gate, and steps written as short lines.
- A test names the SPEC sentence it proves. Its author is not the builder and writes it first. A builder's commit that edits an expected value is refused. Each test is shown to fail once. Cases come from a rule's boundaries. No expected value is copied from the implementation. Progress is counted in rules passing.
- A builder reads the SPEC whole, its work unit's design page, its task with its tests and the coding guidelines, and runs its work unit's tests. The second host runs the full suite on every commit. The wave barrier runs everything and the robustness corpus.
- A ruling goes into the SPEC or a test. A finding is fixed in its stage or becomes one PLAN line. A stage does not close with open entries.
- A probe that exposes a wrong answer becomes a stated test; a passing probe is thrown away.
- Sessions rotate, the peer's included. Agent definitions carry model and effort.

## Open

- How much is rebuilt: the whole compiler, or the semantic stages with the lexer, the parser and the IR kept. The review rated those three clean; the owner has not ruled.
- Whether the speed targets are reachable: lexing, parsing and the IR alone take about the whole inlined-call target.
- How much machinery broken programs deserve; the owner sets that policy in the design pass.
- What is taken from the old code and tests, as cases and never as a yardstick.

## Owed

- Agent definitions: builder, reviewer, verifier, prober, test author.
- The loops written out: design, test, build.
- To the SPEC: §5.6, a read of an attribute in a unit family is a value of that family (ruled). A plain label on a `match` over a unit value is accepted today and reads as refused (not ruled).
- A later job, not the design pass's: read the old ledger and decisions record once for places where the SPEC needed a ruling, and settle each in the SPEC.
- Host rules for any build here: cargo at `nice -n 19` and 6 jobs; nothing heavy from 18:00Z to 02:00Z.
- Harness debts: `context-watch.mjs` tells a subagent to wrap up at 600,000 tokens and the owner's last figure was 400,000; the head and consultant skills, the agent files and the root `CLAUDE.MD` describe the old layout; one test of `settings.test.mjs` and 22 of `memory-push.test.mjs` fail; the status line names a missing `usage-status.mjs`.

## Read

- `.claude/worktrees/docs/packages/symphonyscript/docs/SPEC.md`, whole.
- `.claude/reference/compiler-review.md`: what went wrong in the last build and what it measured.
