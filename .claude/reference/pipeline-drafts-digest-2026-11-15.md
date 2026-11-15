# The pushed agent-file drafts, digested

A cold reader's report of 2026-11-15 (a subagent on Opus, read-only), ordered by the peer for owed item 3 of the notes. It reads the pushed drafts, the root `CLAUDE.MD` and `.claude/agents/` (merged by the owner in 2083cd9a), against `STANDARD.md`, `FINDINGS.md`, the handover page and the earlier draft set `loops-draft/first/`. Every draft is unapproved. The peer checked three of its claims itself: no `CLAUDE.md` stands at the root; the root file's text did not load into the peer's session; the words "owner-approved" or "approved design" stand ten times across the root file and six agent files. Everything else is the reader's and is checked where it comes to carry weight. CM is the root `CLAUDE.MD`, A/x is `.claude/agents/x.md`, STD and FND are the standard and the findings file, F/ is `loops-draft/first/`; line numbers are at commit bd4803a0.

## The pipeline as the pushed drafts state it

1. A unit starts from an "owner-approved unit page" (CM:43, A/builder:11). No draft says who writes the page, where it lives or what form it has.
2. The test author, on a `tests/` branch, and the builder's first phase, on a `unit/` branch, start in parallel (CM:43). Who cuts the branches and who writes a brief is not said. The test author writes the rule list, the case files and the cost rows (A/test-author:17 to 46). The builder writes its breakdown and its structure proposal, hands them back and waits (A/builder:36), which stands against the preference "A builder never waits idle".
3. The test reviewer checks the tests (A/test-reviewer:18 to 38). Where its findings go is not said, and "test-reviewer sign-off" (A/test-author:41) is not defined.
4. The reviewer reads the proposal by FND 5.7 and the verifier gates its findings (A/reviewer:22 to 27, A/verifier:35). Nobody is named who declares a proposal accepted, and no repair of a first-phase finding is described.
5. "The runner" merges the frozen tests onto the builder's branch (A/builder:36). A/builder:11 calls the runner "the session that started you"; CM:5, 51 and 52 use the word for the Linux host. No pushed agent has a tool to start or message another agent, so starting, relaying and merging fall to a session no pushed file defines; CM:28's "orchestrators" is not defined either.
6. The second phase builds to the accepted proposal (A/builder:38 to 47). Review starts only on a tip the gate on `wb2` marked green (CM:45); what the gate runs and who reads its result is not said.
7. The reviewer and the prober read the tip; the reviewer has no shell, so how it sees the diff is not said. The verifier answers ADMIT, REJECT or ROUTE (A/verifier:25 to 36). Who carries an admitted finding to the builder is not said.
8. Not defined: the end of a round, the end of a unit, who merges to `main`, and what happens when a page's budget in rounds is spent. The earlier set stops the unit and goes to the owner there (F/LOOPS.md:81).
9. Of the builder's three lines only DISPUTE has a receiver, the test author. GAP and OBJECTION have none.

## Where the drafts and the standard put the owner into the work

- The owner-approved page: CM:24 and 43; A/builder:11; "approved design page" in A/test-author:3, 11 and 20, A/test-reviewer:23, A/reviewer:25 and 32, A/verifier:20.
- CM:28: orchestrators bring "language semantics, product, architecture, dependency, diagnostic catalogue, `unsafe`, and irreversible choices to the owner".
- CM:19 and 34: the kernel and `unsafe` need the owner's explicit approval.
- A/verifier:52: ROUTE to "Owner via peer (SPEC gap/conflict)".
- STD:16 (the page, approved by the owner), STD:59 (libraries, checked before the page was approved), STD:122 (two lines that cannot both be met), STD:128 (the owner approves catalogue entries with the page). FND names the owner nowhere.
- `.claude/OWNER_PREFERENCES.md`, last line: an entry for the message catalogue is written only after the owner approves it.

## Where the pushed drafts differ from the standard and the findings file

- DISPUTE is widened from "the SPEC sentence the test contradicts" (STD:137) to the SPEC or the page (A/builder:54, CM:26, A/test-author:51).
- A finding's sources are widened beyond FND:23: the root file's crate invariants and `ARCHITECTURE.md` (A/reviewer:35, A/verifier:28, CM:33).
- The verifier gains a third answer, ROUTE (A/verifier:35 to 36, CM:45); FND:48 knows admit and reject, and 5.8's note.
- CM:39 lets a page or a brief assign frozen tests and cost rows to a builder; STD:89 and STD:137 allow no exception.
- A/builder:33 puts each production's form in the proposal; STD:85 puts it on the page.
- A/reviewer:32 drops FND:16's "or the account is withdrawn".
- Citations that say something else at their source: CM:45 (4.3), A/test-author:52 (FND 5.4), A/test-reviewer:12 (4.1), A/explorer:21 (lesson P-12), A/reviewer:42 (the notes "for settled rulings").
- The test reviewer and the explorer are not among FND's four readers; "runner" and "orchestrators" stand in neither document.
- Paths that do not exist: `/CLAUDE.md` (named in A/explorer, A/reviewer, A/verifier; the root holds `CLAUDE.MD`), and the lessons files of the prober, the test author, the test reviewer and the verifier (each read "when it exists"). `ARCHITECTURE.md` line 9 still lists the deleted package `CLAUDE.md`; `PLAN.md` line 101 and `.claude/settings.json` line 22 still point to rules of the old root file.
- The root file does not load on `wb`: Claude Code looks for `CLAUDE.md`, the file is `CLAUDE.MD` and the filesystem tells them apart. By the reader's look at git history it has had that name since 2026-06-21, so the earlier version did not load either (unverified by the peer).
- The user-level folder `/home/cdagent/.claude/agents` holds another project's `builder`, `reviewer` and `verifier`, and the peer's session lists those. Whether the project's files of the same names win in a fresh session is unverified; a fresh session's agent list would settle it.

## The pushed set against the earlier set

- Only the earlier set has: "agents never write to each other"; a home for a page (`docs/design/<unit>.md`); the gate's jobs and how its results are read; the design loop; the criteria that close a unit; the lines' destinations and round cost; the budget and the stop; roles and models.
- Only the pushed set has: the first phase with the proposal, the breakdown and its review by FND 5.7; tests that reach the builder only after acceptance; OBJECTION; the account-first review; the class gate; 5.8's notes; the record of rejected findings; growth and clock rows.
- Where the earlier set breaks the standard: its builder reads the rules and cases before building (against STD:97); "it commits no test" (against STD:844); design-page invariants in the rule list (against STD:27); the prober's growth "with the work counted" (against FND:50); its verifier reads `CODING.md` and `REVIEWING.md` and accepts a code rule by its title (against FND:23).
- Where they disagree without the standard deciding: a deliberately wrong case kept in the suite (earlier) or none (pushed); the tests landing on `main` before the build (earlier) or merged onto the builder's branch (pushed); the test author on Fable (earlier) or Opus (pushed).

## What else those commits changed

- `.claude/lessons/BUILDER.md`, `HEAD.md`, `REVIEWER.md`: one line each, "Last ID: B-0", "H-0", "R-0".
- `.claude/OWNER_PREFERENCES.md`: "Lessons, preferences and notes are committed and pushed" became "Approved lessons and preferences, and notes, are committed and pushed"; two lines added, on how an open item is written for the owner and on text proposed for a file.
- The root file: all of the other project's layout removed (tickets, PRs, the head, deploy rules, "write no comments"); added a posture section, fences with an order of truth, crate invariants, the unit loop, host hygiene and documentation rules.
