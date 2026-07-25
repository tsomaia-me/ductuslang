# Part 3 — Re-anchor Survey Plan

*2026-07-20 · Initiated by owner ("Initiate Part 3"). Read-only survey; no corpus edits. Baseline commit: d719517 (wire+channel, Part 2 closed).*

## Scope

1. **I-01..I-80** in `audit-plan-3-individual-decisions.md` (§Decisions) — one decision per item, severity-ordered.
2. **DP-01..DP-17** (§Dismiss-or-pin) — verifier splits; DP-02 and the I-item at doc line ~1131 carry SETTLED annotations (record as SETTLED, do not re-litigate).
3. **Part-2 residue** (per ledger "PART-2 RESIDUE FOLDED INTO PART-3 SURVEY" + deferred notes):
   - R-1 duplicate-keyed-by residue
   - R-2 strict-kind vocabulary sweep (optional per ledger)
   - R-3 Portal[WeakHandle[T]] LOG-mirror nicety
   - R-4 LOG 033-207/208/209 vs IR_GRAMMAR InstrSep divergence
   - R-5 GRAMMAR 005-30 stale-cite check (TraitDecl re-anchor landed in 4ffb4a6 — verify closure)
   - R-6 mixed own-line + inline directive stacks unpinned

## Why re-anchor

The audit doc predates: streams rework, satisfies retirement, policies, barrel visibility, cleanup, main removal, wire+channel. All line numbers are stale; all vocabulary predates connection→wire, wiring-type→channel, main→@root, public/shared→barrel. Items must be re-located by CONTENT and re-read through the renames.

## Classification (one per item, with current-corpus evidence)

- **DISSOLVED** — finding no longer exists; cite the current text that closes it and (when known) the commit.
- **STANDS-MECHANICAL** — finding present; recommendation coherent; no real tradeoff → consent-list candidate (one-line restated rec).
- **STANDS-RULING** — finding present; genuine options → ruling queue (facts + current quotes; owner presentation authored by main session).
- **CHANGED** — finding mutated under Part 1/2; describe present state; propose classification of the mutated finding.
- **SETTLED** — pre-annotated closed; verify the annotation's referenced fix is on disk, then record.

Evidence discipline: every claim carries current file + line + quote (LEARNINGS #3/#8). /usr/bin/grep only (rtk corruption).

## Protocol

One Fable orchestrator (background) → synchronous Opus menials batched by topic (~10 items each), haiku for pure greps. Read-only: the ONLY file written is `part3-survey-report.md`. No corpus edits, no plan-doc edits, no commits. Main session verifies (sample re-check + classification review) before anything is presented to the owner.

## Deliverable

`part3-survey-report.md`: per-item table (id · title · classification · one-line evidence) + full evidence appendix + drafted consent-list + drafted ruling queue. Then: owner consent-list + protocol-complete ruling presentations (main session), rulings, execution batches per the standing orchestration protocol.
