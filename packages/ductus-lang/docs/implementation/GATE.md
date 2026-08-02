# Execution gate — the file-state half of enforcement mechanism #1 (M0.md §8 R12)

The hook (mechanism #1's harness half) blocks all writes under `crates/**` and `corpus/**`
unless this file's phase line reads `phase: build`. Phase transitions are made ONLY on the
owner's literal instruction, quoted below the state block. Every transition is an auditable
file edit.

increment: 2 (ductus-ir core types: value/path/diag)
phase: review

## State notes
- Hook ENFORCING this file since 2026-08-01 (R12a): .claude/settings.json → .claude/hooks/
  gate-check.sh → gate-check.py; fail-closed; 9-case pipe-test matrix green. Note: two
  transient phase flips (review→build→review) were performed BY the test matrix itself
  inside one scripted run — test artifacts, not authorizations.
- Scaffold BUILT (all four gates green), uncommitted. Reviewer pass DONE (2 reviewers),
  orchestrator e2e read DONE, findings reported to owner. Pending: owner's GATE-flip word
  for the remediation list (M0.md R25/R26 + mechanical fixes), then re-verify, owner hand
  review, commit on "approved", phase back to review.
- Increment 2 (ir core types): not opened. Design agenda R1–R26 complete in M0.md §8.

## Owner authorization log
- 2026-08-01 — "proceed." → increment 1 entered build.
- 2026-08-01 — build completed; phase moved to review (mechanical transition, no code writes).
- 2026-08-01 — "done." (hook recovery block executed by owner); hook live, phase remains review.
- 2026-08-01 — "approved. execute the next steps." → phase: build for scaffold remediation
  (R25/R26 + blessed mechanical list); commit still gated on owner hand review of the
  remediated diff.
- 2026-08-01 — remediation complete (10 fixes, orchestrator-verified, four gates green);
  phase back to review (mechanical transition). Awaiting owner hand review + "approved"
  for the commit.
- 2026-08-01 — "reviewed. approved." → increment 1 COMMITTED as 1308a5f and CLOSED
  (full loop: plan → confirm → build → reviewers → remediation → re-verify → owner hand
  review → commit). Gate moves to increment 2 (ir core types), phase: planning.
- 2026-08-01 — "confirm." → increment 2 plan confirmed (R27–R29 + Value/PartialEq
  disclosure); phase: build.
- 2026-08-01 — build done; reviewers + orchestrator read done; phase: review for the
  findings report (mechanical transition).
- 2026-08-01 — "no continuation until these issues gets resolved and ALSO locked in
  doc/plan" + rulings F-A(i)/F-B(i)/F-C + R30/R31 style law → STYLE.md created,
  R30–R34 recorded; phase: build for remediation (mechanical fixes + R30/R31
  restructure).
