# The loops

How one unit of SymphonyScript's compiler is designed, tested, built and closed. The runner is the session that starts the agents and carries every line between them; agents never write to each other. There are no tickets and no pull requests.

## A unit

A unit is one piece of the design, built in one build loop. It has:

- a design page, `packages/symphonyscript/docs/design/<unit>.md`, written by the peer and approved by the owner;
- the rules it must make pass, named on its page by their ids;
- a branch, `unit/<unit>`, cut from the base branch, and a worktree of its own.

The base branch is the branch the work lands on: `main`.

## The stated tests

- Every stated test lives in one tree, `packages/symphonyscript/crates/compiler/tests/stated/`, on the base branch: the rules files, the cases and the harness.
- A rule is one SPEC sentence, one grammar alternative or one design-page invariant, quoted word for word with its place, under an id. A case belongs to one rule and holds a program and the expected value the rule's words give.
- `packages/symphonyscript/crates/compiler/tests/stated-pending.txt` lists the id of every case the code does not pass yet. The harness requires every listed case to fail and every other case to pass, so the suite is green at every commit and the list is the baseline: a case that passed at the start is one the list never held.
- Only the test loop changes the tree. Only the test loop adds a line to the list; a builder removes a line in the commit that makes its case pass.

## The gate

A script on wb2 runs on every commit pushed to a `unit/` or `tests/` branch, the newest first, and writes each result where the agents read it.

1. On a `unit/` branch it refuses the commit, running nothing, when the stated tree differs from the base branch's or the pending list holds a line the base branch's lacks. On a `tests/` branch it refuses a commit that changes a file outside the stated tree and the pending list.
2. It runs the format check, the lints, the whole suite and the speed rows of the pages built so far, each judged by its own exit code.
3. It confirms the stated-test binary ran as many cases as the tree holds, and counts the rules passing: a rule passes when none of its cases is listed.
4. Green is all of these. The runner moves the base branch forward only to a tip the gate marked green.

The gate's own tests show each refusal once: a commit that edits a case, one that adds a pending line, one that drops the stated-test binary from the build, one that regresses a passing case.

## Design loop

1. The peer writes the unit's page: what it builds; the SPEC sections it serves; what it hands on; its invariants; the surface its tests observe; the files it may change; the proven checks its builder runs; its speed rows on wb2; its budget in fix rounds.
2. One cold reviewer reads the page against the SPEC.
3. The owner approves the page. An open point comes to the owner; a ruling goes into the SPEC or a test.

## Test loop, before any build

1. The test author reads the SPEC whole and the page, and lists the rules: every sentence, grammar alternative and invariant of the sections the page names is a row or a line naming the unit that owns it.
2. It writes each rule's cases from the rule's boundaries, every expected value derived from the rule's words by hand, and commits them before the first run.
3. It runs them and lists every failing case as pending. A value it changes after a run is reported with the words that decide it.
4. Shown to fail once, by the test author, at this grain:
   - a case of new behaviour fails on arrival, which the harness proves at every commit while the case is listed;
   - a harness that compares whole values, a whole tree or a whole list of diagnostics, holds one deliberately wrong case that must fail, kept in the suite;
   - a case of kept behaviour with a weaker check is made a whole-value comparison, and where it cannot be, the test author breaks the code once in a scratch copy, sees the case fail and discards the copy.
5. A cold test reviewer, who opens no implementation file and runs nothing, derives every expected value from its rule before reading the stated one, and checks the rules list against the sections.
6. The runner lands the `tests/` branch on the base branch once the test reviewer has no finding left and the gate is green. From then the cases are frozen.

## Build loop

1. The builder reads the SPEC whole, the page, the code rules, the unit's rules and cases, and the files the page names, whole, before it changes one.
2. It builds until no case of its rules is listed, running only those cases and the proven checks its page names. It commits no test. The gate runs everything else on every commit, and the builder never waits for it.
3. On READY a cold reviewer reads the diff, told the branch, the base, the tip and the page and nothing else, and a prober writes programs around the unit's rules and runs them on the tip. Neither is told what any test shows.
4. The verifier takes every finding before the builder sees it and answers each ADMIT, REJECT or ROUTE by its gates:
   - it quotes a SPEC sentence, a design-page invariant or a code rule, and the words are there and say what the finding claims;
   - it reproduces on the tip: the program gives what the finding reports, or the lines hold what it names;
   - its fix lies inside the page; one that does not is routed, a SPEC gap to the owner, a page gap to the peer, a defect outside the unit to one PLAN line.
5. An admitted finding that shows a wrong answer becomes a stated case, written by the test author from the rule's words and listed pending, before the builder fixes it. A probe that passes is thrown away.
6. A fix round: the builder fixes every admitted finding, the reviewer is resumed and checks each fix as a change of its own and reads all code the round changed, and the verifier takes its findings again.
7. The unit closes when no case of its rules is listed, no admitted finding is open and the gate is green on its tip. The runner then moves the base branch to that tip.

## Lines

Every line goes to the runner, and the agent keeps working on what the line does not block.

| Line | From | Says | Goes to | Costs a round |
|---|---|---|---|---|
| DISPUTE | builder | a case contradicts the SPEC or the page: the case, the sentence quoted, the value the sentence gives | the test author, who corrects the case on a `tests/` branch, or answers with the sentence that decides it; a sentence that decides neither way is a GAP | no |
| QUESTION | builder | the page cannot be built as written, or leaves open a choice that changes what the unit hands on | the peer, who answers from the page or amends it | no; a round lost to it is counted apart |
| GAP | anyone | the SPEC is silent or says two things | the owner, through the peer, with options and a pick | no |
| MISSING | builder | a program its rules cover that no case holds | the test author | no |

A corrected case is checked by the test reviewer before it lands. A dispute the test author answers and the builder still holds goes to the peer once.

## Rounds and the stop

- A unit has a budget in fix rounds, on its page. A round is one READY, its review, its probes and its admitted findings fixed.
- A round lost to a wrong case or to a gap in the page is counted apart and spends none of the budget.
- A unit that reaches its budget with a finding open stops and goes to the owner with its records: the rounds, the findings, the lines, the tokens, the tool calls and the wall time.
- A finding is fixed in its unit or becomes one PLAN line. A unit does not close with an open entry.

## Roles

| Role | Model, effort | Reads | Never |
|---|---|---|---|
| Test author | Fable, high | SPEC whole, the page, the test rules | runs the code to find an expected value |
| Test reviewer | Opus, high | SPEC whole, the page, the test rules, the rules and cases | opens an implementation file, runs anything |
| Builder | Opus, high | SPEC whole, the page, the code rules, its rules and cases, the page's files | changes a stated test, adds a pending line |
| Reviewer | Opus, high | SPEC whole, the page, the code rules, the defects, the diff and its files | changes a file, reads a test result |
| Prober | Opus, high | SPEC whole, the page, the unit's rules, the built code | keeps a passing probe, reads a case's expected value |
| Verifier | Opus, high | the findings, their sources, the tip | fixes, rates, proposes a mechanism |

## The trial

- The unit: the neutral shape of a body line with a colon. It runs on a throwaway branch.
- Before it: the design page for what the parser hands on, then the stated trees of SPEC §7 to §9, one a grammar alternative, landed with their pending list. That list is the baseline.
- Budget: one builder session and two fix rounds.
- It passes when: no case of its rules is listed; every case the baseline did not list still passes; the tree reproduces the source over the corpus; the 700,000 broken inputs draw no panic; 100,000 levels fit a 1 MiB stack; the cost rows stay linear; the large fixture parses on wb2 within 5 % of 8.3 ms.
- It sprawls when it touches the tree builder or the lexer or needs a third fix round. The peer also measures the lines it changes against a figure no builder is told.
- Recorded either way: tokens, tool calls, rounds, wall time, and the rounds counted apart.
- The full repair's budget is three fix rounds a unit.
