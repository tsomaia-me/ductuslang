# Execution gate — the file-state half of enforcement mechanism #1 (M0.md §8 R12)

The hook (mechanism #1's harness half) blocks all writes under `crates/**` and `corpus/**`
unless this file's phase line reads `phase: build`. Phase transitions are made ONLY on the
owner's literal instruction, quoted below the state block. Every transition is an auditable
file edit.

increment: 1 (workspace scaffold)
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
