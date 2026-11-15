# The loops

How one unit of SymphonyScript's compiler is designed, tested, built and closed. The runner is the session that starts the agents and carries every line between them; agents never write to each other, and an agent's lines reach the runner in its hand-back. There are no tickets and no pull requests.

## A unit and its page

A unit is one piece of the design, built in one build loop on a branch `unit/<unit>` cut from the base branch. The base branch is `main`.

Its design page, `packages/symphonyscript/docs/design/<unit>.md`, is written by the peer and approved by the owner. The page states:

- what the unit builds, and the SPEC sections and productions it serves; the unit's rules are every rule whose place lies in them;
- the structure: each module, each item visible outside its module's folder with its name, fields and signature, the entry functions, the output types, and the modules each module may import;
- its invariants;
- the surface its cases observe: what is printed, and the diagnostic codes, spans and order the cases may compare;
- the one diagnostic for a construct the unit does not build yet;
- the files it may change, the dependencies and settings it may change, and the older tests and pending lines it retires, each with the sentence that retires it;
- the proven checks, as commands, and its cost cases;
- its speed rows on wb2;
- its budget: builder sessions before the first READY, and fix rounds.

Only the peer changes a page. A change that alters what the unit hands on is cold-reviewed and approved as the page was.

## The stated tests

`docs/TESTING.md` says where they live and what the harness enforces. In short: rules, cases and the harness are one tree on the base branch; `stated-pending.txt` lists the cases the code does not pass yet, so the suite is green at every commit and the list is the baseline; a rule with no case is untested and never counts as passing.

## The gate

A script on wb2 runs on every commit pushed to a `unit/`, `tests/` or `harness/` branch, the newest first, and writes each result where the agents read it (`docs/running.md`).

It refuses a commit, running nothing, when:

- the commit does not contain the base branch's tip;
- on a `unit/` branch, the stated tree differs from the base tip's, the pending list holds a line the base tip's lacks, or a test, a dependency file, a setting or a baseline changed that the unit's page does not list;
- on a `tests/` branch, a file changed outside `rules/`, `cases/` and the pending list;
- on a `harness/` branch, a file changed outside `harness/` and what its page lists.

Then it runs, each judged by its own exit code:

1. the format check;
2. the lints of `docs/CODING.md` and its scripts, against the baseline, whose counts never rise;
3. the whole suite, and beside it by name every check the pages built so far list as proven, the ones marked `#[ignore]` included; a test marked `#[ignore]` that no page lists is a breach;
4. the count of cases the stated binary ran against the count the tree holds;
5. the speed rows of the pages built so far;
6. the verdicts of every corpus program on the base tip and on the commit, and the list of those that differ.

Green is: not refused, and 1 to 5 pass. The result names every failing case and every differing verdict.

The runner moves the base branch only by fast-forward to the exact commit the gate marked green. On every new base tip the gate checks that it marked that commit green, and reports a tip it did not.

No server rule stops a push to the base branch: every agent shares one git identity. The files forbid it, and the gate reports it after the fact.

The gate's own tests show each refusal once: a unit commit that edits a case, one that adds a pending line, one that edits an unlisted test, one that drops the stated binary from the build, a tests commit that edits the harness, a commit that lacks the base tip, and a base tip the gate did not mark.

## Design loop

1. The peer writes the page.
2. One cold reviewer reads it against the SPEC.
3. The owner approves it. An open point comes to the owner; a ruling goes into the SPEC or a case.

## Harness

The harness is built once for each surface, as a unit with a page: a builder writes it on a `harness/` branch, the reviewer reads it, and it lands as a unit does. It is written against the public surface the page states and reads no private item.

## Test loop, before any build

1. The test author reads the SPEC whole and the page, and lists the rules: every sentence, grammar alternative and invariant of the page's sections is a row, or a line naming the unit that owns it.
2. It writes each rule's cases and commits them before the first run.
3. It runs them and lists every case that fails on the base. A value it changes after a run it reports with the words that decide it.
4. A cold test reviewer works in an export of the branch that holds no implementation source. It derives every expected value before reading the stated one, and checks the rules list against the sections. The runner then gives it the list, taken from git, of every value that changed after the first run.
5. The `tests/` branch lands when the test reviewer has derived every case, has no finding left, and the gate is green. From then the cases are frozen.

Every case that lands passes the test reviewer, whatever brought it: a first writing, a correction, a finding or a missing program.

Before the full repair of the parser starts, the stated trees of every production land, with their pending list as the baseline. Before the trial, those of SPEC §7 to §9 land.

## Build loop

1. The builder reads the SPEC whole, the page, `docs/CODING.md`, the unit's rules and cases, and every file the page lists, whole, before it changes one.
2. It builds until no case of the unit's rules is listed. It commits no test, removes the line of any case its commit makes pass, and retires exactly the older tests the page lists. The gate runs on every commit, and the builder never waits for it.
3. When the stated tree moves on the base, the gate refuses the unit's next commit, and the builder merges the base at once.
4. On READY the runner makes two exports, the unit's merge-base with the base branch and its tip, each without git history and without the pending list. A cold reviewer reads the difference between them, told the page and nothing else. A prober, whose export also lacks the cases, writes programs around the unit's rules and runs them on the tip.
5. The verifier takes every finding before the builder sees it and answers each ADMIT, REJECT or ROUTE:
   - it quotes a SPEC sentence, a page invariant or a code rule, and the words are there and say what the finding claims;
   - it reproduces on the tip;
   - its fix lies inside the page; one that does not is routed.
6. An admitted finding that shows a wrong answer becomes a stated case, written from the rule's words, before the builder fixes it. A probe that passes is thrown away.
7. A fix round: the builder merges the base and fixes every admitted finding. The reviewer is resumed with the new exports and the verifier's rejections, the prober with the rules the fixes touch, and the verifier takes their findings again.
8. The unit closes when: no case of its rules is listed; none of its rules is untested, but for a GAP the owner holds; no admitted finding is open; every verdict the gate lists as differing is one a stated case requires; the gate is green on its tip. The runner then moves the base to that tip.

## Lines

A line reaches the runner in a hand-back. The builder hands back at once, after committing what is whole, for a DISPUTE, a QUESTION, a GAP or an OBJECTION; the runner resumes it with the answer.

| Line | From | Says | Goes to |
|---|---|---|---|
| DISPUTE | builder | a case contradicts the SPEC or the page: the case, the sentence quoted, the value it gives | the test author, who corrects the case or answers with the words that decide it |
| OBJECTION | builder | an admitted finding is wrong: the finding, the sentence quoted | the verifier, once more; then the peer |
| QUESTION | builder | the page cannot be built as written, leaves open a choice that changes what the unit hands on, or a rule of `docs/CODING.md` cannot be met | the peer, who answers from the page or amends it |
| GAP | anyone | the SPEC is silent or says two things | the owner, through the peer, with options and a pick |
| MISSING | builder | a program its rules cover that no case holds | the test author |
| INVARIANT | builder | a property its code now relies on that the page does not state | the peer, who puts it on the page, and the test author, who writes its case |

- A disputed case stays as it is until the test author answers. The builder builds to the sentence, the gate names the case as failing, and the unit cannot close while the dispute is open.
- A dispute the test author answers and the builder still holds goes to the peer once, and the peer's answer ends it unless it is a GAP.
- A reviewer's defect outside the diff, a verifier's defect seen in passing, and a routed finding the page does not hold each become one PLAN line, written by the runner.

## Rounds and the stop

- The page gives two budgets: builder sessions before the first READY, and fix rounds after it. The first READY, its review and its probes are not a fix round. A fix round is one set of admitted findings fixed and reviewed again.
- A round or a session lost to a wrong case or to a gap in the page is counted apart and spends neither budget.
- A unit that passes either budget stops and goes to the owner with its records: the sessions, the rounds, the findings, the lines, the tokens, the tool calls and the wall time.
- A finding is fixed in its unit or becomes one PLAN line. A unit does not close with an open entry.

## Roles

| Role | Model, effort | Works in | Never |
|---|---|---|---|
| Test author | Fable, high | a worktree of a `tests/` branch | runs the code to find an expected value |
| Test reviewer | Opus, high | an export with no implementation source | runs anything |
| Builder | Opus, high | a worktree of the unit's branch | changes a stated test, adds a pending line, changes what the page does not list |
| Reviewer | Opus, high | exports of the merge-base and the tip, no history, no pending list | changes the repository, learns what a test shows |
| Prober | Opus, high | an export of the tip, no history, no pending list, no cases | keeps a passing probe |
| Verifier | Opus, high | an export of the tip | fixes, rates, proposes a mechanism |

## The trial

- The unit: the neutral shape of a body line with a colon.
- Its base is `trial/base`, cut from `symphonyscript/r3`, on which the harness and the stated trees of SPEC §7 to §9 land with their pending list. That list is the baseline. Its branch is `unit/colon-line`, cut from `trial/base` and gated like any unit. Whether a trial that passes lands on `main` is the owner's call with the repair-or-rewrite choice.
- Budget: one builder session before the first READY, and two fix rounds.
- It passes when: the unit closes as a unit does; the tree reproduces the source over the corpus; the 700,000 broken inputs draw no panic; 100,000 levels fit a 1 MiB stack; the cost rows stay linear; the large fixture parses on wb2 within 5 % of 8.3 ms. Each of these is a proven check on its page, so the gate runs it on every commit.
- It sprawls when it touches the tree builder or the lexer, or passes a budget. The peer also measures the lines it changes against a figure no builder is told, and counts the baseline entries it touches.
- Recorded either way: tokens, tool calls, sessions, rounds, wall time, and what was counted apart.
- The full repair's budget is three fix rounds a unit.
