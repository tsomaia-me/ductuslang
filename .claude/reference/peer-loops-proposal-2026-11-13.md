# The peer's proposal of 2026-11-13: the loops, the roles and the trial

The three sections of the peer's message as the owner read them, word for word. The owner approved them with the gaps and fixes in `owner-brief-2026-11-13.md` items 4 and 5, which stand over this text: the verifier's job and model change, a dispute line and the meaning of "shown to fail once" are added, the stated trees for §7 to §9 come before the trial and the rest after it, a baseline is recorded first, and the 320-line stop is in no builder's brief.

---

## The loops

**Design loop, per work unit**
1. The peer writes the unit's design page: what it builds, the SPEC sections it serves, the invariants it must keep, its speed rows, its budget in rounds.
2. One cold reviewer reads the page against the SPEC.
3. You approve the page. An open point comes to you; a ruling goes into the SPEC or a test.

**Test loop, before any build**
1. The test author reads the SPEC whole and the design page.
2. It lists the unit's rules, one row per SPEC sentence or grammar alternative, each quoted.
3. It writes each rule's tests from the rule's boundaries, with expected values derived from the SPEC and never from a run.
4. Each test is shown to fail once: a new behaviour fails on arrival; a kept behaviour is broken once in the code.
5. A cold test reviewer checks every expected value against its quoted sentence.
6. The tests are frozen. The gate refuses a builder commit that touches them.

**Build loop**
1. The builder reads the SPEC whole, the design page, its task with its tests and the coding guidelines.
2. It builds until its unit's tests pass, running only those and the proven checks its page names.
3. On every commit, wb2 runs the full suite and the speed rows.
4. On READY: a cold reviewer reads the diff; a prober writes programs around the unit's rules and runs them.
5. A probe that exposes a wrong answer becomes a stated test, written by the test author, before the fix.
6. The unit stops at its round budget and comes to you with the records.
7. A unit closes with no open entries, and its rules passing are counted.

**Roles**

| Role | Model, effort | Reads | May not |
|---|---|---|---|
| Test author | Fable, high | SPEC whole, design page | run the code to find an expected value |
| Test reviewer | Opus, high | the tests, the quoted sentences | see the builder's code |
| Builder | Opus, high | SPEC whole, page, task, tests, guidelines | edit a frozen test |
| Reviewer | Opus, high | the diff, the page, the guidelines | change code |
| Prober | Opus, high | the SPEC sections, the built unit | keep a passing probe |
| Verifier | Sonnet, medium | the gate's records | judge code; it counts rules passing |

For the trial, this peer session starts the agents. Whether a head session runs the full repair is a later call.

## The trial

- **The unit:** the neutral shape for a body line with a colon (`Soft: wash`).
- **Before it:**
  1. A design page for what the parser hands on: which productions are nodes, and how the three name-dependent forms are shaped. Without it a tree cannot be stated from the SPEC, only copied from the code.
  2. The stated trees, one per grammar alternative, frozen. That is your first condition, and it is also the test loop's first run.
- **Budget:** one builder session and at most two fix rounds.
- **It lands clean when all of these hold:**
  - its stated tests pass;
  - every earlier stated tree still passes;
  - the tree still reproduces the source byte for byte over the corpus;
  - the 700,000 broken inputs still draw no panic;
  - 100,000 levels still fit a 1 MiB stack;
  - the cost rows stay linear;
  - parsing the large fixture on wb2 stays within 5% of 8.3 ms.
- **It sprawls when** it changes more than 320 lines (twice the sizing's upper estimate), touches the tree builder or the lexer, or needs a third fix round.
- **Recorded either way:** tokens, tool calls, rounds and wall time, since that cost is the estimate for the rest.
- **The full repair's budget:** three fix rounds per unit, then a stop that comes to you. I set the number of units after the trial.
