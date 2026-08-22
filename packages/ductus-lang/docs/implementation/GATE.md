# Execution gate — the file-state half of enforcement mechanism #1 (M0.md §8 R12)

The hook (mechanism #1's harness half) blocks all writes under `crates/**` and `corpus/**`
unless this file's phase line reads `phase: build`. Phase transitions are made ONLY on the
owner's literal instruction, quoted below the state block. Every transition is an auditable
file edit.

increment: Chunk B (the runtime kernel: loader, store, engine, trace_sink, witness corpus + goldens)
phase: build

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
- 2026-08-02 — "approved, committed and merged. continue." → increment 2 CLOSED (owner
  commit d5aa03e). Gate moves to increment 3 (text foundations), phase: planning.
- 2026-08-02 — "proceed." → FND execution batch authorized (R35–R45 into
  DECISION_LOG/IR_GRAMMAR/SPEC, LOG-first; docs are ungated — logged here for the
  audit trail). Lexer increment plan follows the batch.
- 2026-08-02 — "confirmed." → D2 NEWLINE scoping confirmed (braced-body entries terminate
  after their closing brace; section-final same-line entry legal); FND batch cleared for
  commit. Next: the lexer increment plan (beat 2) for increment 3.
- 2026-08-02 — "confirmed" → increment 3 plan confirmed (R46–R48 + TokenKind/raw-payload/
  -inf/CRLF details as presented); scope adjusted per soft-keyword ruling: complete lexer,
  no parser/emitter content. phase: build.
- 2026-08-02 — increment 3 build + review + remediation complete (79 tests, four gates,
  2 reviewers, FND-13/14/15 ruled → corpus rider 033-249-amended + 033-254; orchestrator
  end-to-end read done). phase: review — awaiting owner hand review + "approved" → commit.
- 2026-08-02 — "Verdict: HARD REJECT." (file-name≠entity-name; multiple public functions
  per file) → R52 file-naming law recorded, STYLE.md S1 rewritten; phase: build for the
  same-increment rework (renames + scan/ split). Beat 4 re-runs after.
- 2026-08-02 — R52 rework complete (diag_class.rs, token_kind.rs, lex.rs, scan/ folder
  with one recognizer per file + folder-private chars/payload/test_support; 79 tests
  held; four gates green; orchestrator verified tree + seam files). phase: review —
  beat 4 re-runs.
- 2026-08-02 — "approved. committed and pushed. continue." → increment 3 CLOSED (owner
  commit; first HARD REJECT survived — R52 rework accepted on second review). Gate moves
  to increment 4, phase: planning.
- 2026-08-02 — "confirmed." → increment 4 plan confirmed (R53/R54/R55 + stated details:
  expect_word soft-keyword helpers, module-level newline tolerance, `types {}` empty-form
  canon, one trailing \n, no behavior-body parsing in M0). phase: build.
- 2026-08-02 — increment 4 build + 2-reviewer pass + 15-fix remediation complete (98
  tests, four gates; first corpus witness module_header.duir + conformance test; parse
  takes Vec<Token> moving payloads; covering() hardened; property test live). FND-16/17
  filed. phase: review — beat 4: owner hand review + "approved" → commit.
- 2026-08-02 — "approved, committed and pushed. FND-17: (c)" → increment 4 CLOSED (owner
  commit); FND-17 ruled (c), R54's §5-oracle clause struck in M0.md. Gate moves to
  increment 5 (types-table vertical), phase: planning.
- 2026-08-02 — "confirm." → increment 5 plan confirmed (R56/R57/R58 + terminator helper,
  canon details, types_table witness). phase: build.
- 2026-08-02 — increment 5 build + 2-reviewer pass (~70 adversarial probes) + remediation
  complete: FND-18 ruled (a) → 033-255 nesting bound in all three corpus docs + depth
  guard with the law's own counting (leaves free, bracketed constructors cost); producer
  contract extended to every name/arity position; emitter renders into the sink; shared
  test fixtures; 127 tests, four gates; types_table.duir witness; orchestrator e2e read
  done. phase: review — beat 4: owner hand review + "approved" → commit.
- 2026-08-02 — "approved" → increment 5 CLOSED (owner commit a7c7122). Rows 6+7 merged
  per R61; gate moves to the scope vertical, phase: planning. FND-16 due in this round.
- 2026-08-02 — "proceed and take it from here" → Chunk A (complete .duir text form)
  authorized under the LEARNINGS #35 cadence: clear-winner calls autonomous with
  end-of-milestone disclosure; genuine direction/architecture forks still owner-ruled.
  FND-16 executes (b) unless struck. phase: build.
- 2026-08-22 — Chunk A COMPLETE: two build waves + 2-reviewer pass + remediation +
  FND-21 execution (owner-ruled mid-chunk: R62 language-wide reservation, R63 key
  escapes = STRING family). 221 tests, six gates green (rustdoc gate added).
  Orchestrator end-to-end read done: both captured diffs in full, all 26 untracked
  files direct, plus the six meta-file diffs neither capture held (LEARNINGS,
  Cargo.toml, FINDINGS/GATE/M0/STYLE). phase: review (mechanical transition) —
  beat 4: disclosure report + owner hand review + "approved" → commit.
- 2026-08-22 — "Approved here and in the future EXCEPT genuine design decisions:
  unless they block continuation - proceed and raise them when finished, only
  interrupt if you are genuinely blocked by design call required from me. Proceed."
  → Chunk A APPROVED and committed (this commit); STANDING approval recorded as
  LEARNINGS #36 (per-chunk approval gates end; genuine non-blocking design calls
  proceed on best-grounded defaults and are raised at finish, veto standing;
  interrupt only on genuine blockage). Gate moves to Chunk B (runtime kernel),
  phase: build. R20's probe-#1 ruling beat is superseded by the standing
  instruction: corpus verified on disk first, then the near-single-path answer
  executes with a FINDINGS entry and veto standing, raised at finish.
