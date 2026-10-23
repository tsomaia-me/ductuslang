# Execution gate — the file-state half of enforcement mechanism #1 (M0.md §8 R12)

The hook (mechanism #1's harness half) blocks all writes under `crates/**` and `corpus/**`
unless this file's phase line reads `phase: build`. Phase transitions are made ONLY on the
owner's literal instruction, quoted below the state block. Every transition is an auditable
file edit.

increment: M1 cut B (temporal + gates — scope + all nine forks owner-ruled; plan finalization + build)
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
- 2026-08-23 — Chunk B COMPLETE and committed under the standing approval
  (ultracode workflow: 34 agents — engine/store, harness, 9 witnesses with
  hand-derived goldens, property tests; 2 adversarial reviewers, 18 findings
  → skeptic-verified: 5 mechanical fixed, 9 design questions parked as
  FND-24..27 + riders, 4 refuted; batched remediation; final gates).
  R64/R65 corpus riders landed (033-260 entry-order carrier, 023-67 bitwise
  gating). 353 tests, six gates green, orchestrator e2e read done (all crate
  files, all corpus files with goldens independently re-derived, full docs
  diff). phase: review — owner rules FND-24..27 + reviews the landed chunk;
  M0 exit criteria met except criterion-4 closure on the open findings and
  the absent CI (raised in the finish report).
- 2026-08-23 — "Approved, continue" (on the FND-26 recommendation: reject all
  four scope-path shapes, now). FND-26 RULED as recommended. FND-24 (a),
  FND-25 (a), FND-27 (a) and the probe-3 resolvability touch-up were raised
  at finish with recommendations and not struck — executed under the #36
  cadence with veto standing. CI remains OPEN (owner has not chosen an
  infrastructure). phase: build for the M0 closure wave (corpus edits +
  loader checks + rank-interleaved witness).
- 2026-08-23 — "Temporal + gates" → M1 SCOPE RULED: cut B (recurrent +
  deferred + gate; wires and streams become a named successor milestone
  whose first half is corpus work — the unstruck rider stands). M1-DRAFT.md
  finalizes to M1.md after the fork rounds. Sequencing law: forks 3+6 ruled
  before any gate code, forks 1+2 before any recurrent code; forks 7+8
  (storage shape, host history reads) are implementation-latitude calls
  taken by cadence with veto standing. phase: planning.
- 2026-08-23 — "Fork 3 - (a) / Fork 6 - (a) / Fork 1 - (b) + (c) now
  (regardless of whatever further forks (c) opens) / Try again" → RULED:
  gate predicates evaluate in a step-1 pre-pass against prior-commit values
  (transitions then join the root set); step 1 is ONE rank-ordered walk with
  all root sources interleaved; the recurrent serialization gains BOTH the
  `lookback` clause on `cell` AND the fallback map — sub-forks (c) opens
  (e.g. non-literal fallbacks) are filed and raised, never blockers. Fork-3
  rider stands as unstruck default: a predicate reading cells inside its own
  guarded subtree REJECTS at load, named diagnostic (veto standing).
  "Try again" = next fork round.
- 2026-08-23 — "Fork 2 - (ii) per-advance / Fork 9 - (a) / Fork 5 - (c)" →
  RULED: history rings shift per-ADVANCE (a skipped commit consumes no
  slot); a recurrent that evaluates to a bit-identical value still advances
  (gating governs propagation only — ring shifts exactly once per
  evaluation); the M1 trace reuses `dirty` for landings and `eval` for
  advances and adds exactly one event kind, `gate <path> open|close`.
- 2026-08-23 — "Fork 4a - dispatch after step 4, before step 5 / Fork 4b -
  b), with (c)'s restraint about scope" → RULED: a deferred dispatches after
  step 4 and before step 5's flip (`publish` stays the last value-bearing
  line of a block); the readiness surface is `has_staged_work(&self) ->
  bool` — one query, `commit()`'s signature untouched, nothing more in M1.
  ALL NINE FORKS RULED. phase: build — M1.md finalization + corpus batch +
  the cut-B build run as one ultracode workflow; cadence defaults for the
  remaining B-findings (C-13, C-20/21, C-23, C-28, C-29, C-45, C-7/C-8
  depth defaults) execute veto-standing and are disclosed at finish.
- 2026-08-23 — M1 WAVE 2. NO OWNER WORDS EXIST FOR THIS WAVE; nothing below is
  quoted or owner-authorized. Orchestrator cadence actions under the standing
  instruction (LEARNINGS #36/#35), veto standing, all disclosed at the M1 finish
  report and recorded as M1.md R80: (1) the ADVANCE-ROOT LAW — a recurrent's
  advance is a dirty root at the NEXT commit, rendering no trace line, and sets no
  dirty bit inside the advancing commit; it completes corpus-silent semantics whose
  landed answer was nondeterministic by declaration order (FND-38) and lands
  LOG-first as the advance-root entry in the 023 family; (3) an omitted `depth` on a
  `deferred` defaults to 1 — code conforms to landed corpus law (016-301); (4)
  construction order = static order, only self-history and lookback edges dropped;
  (5) C-43 conforms to the disclosed default — the guards/gate inverse is validated
  strictly, `guards_inverse_mismatch` replacing `conflicting_gate_binding`; (6) the
  input-history reset fallback (construction-era value) is KEPT and ledgered in
  M1.md §4.7 + FND-28's open sub-fork; (7) witness-helper dedup into
  ductus-conformance's library (STYLE S1). PUT TO THE OWNER, not decided here:
  R73's fork-3 RIDER — wave-1 code enforces the load-time rejection
  (`gate_predicate_reads_guarded`) while the plan recorded it blocked, and SPEC
  §13.9.6 + the 022 gate family permit the pattern; the enforcement stays exactly as
  landed, FND-36 stays OPEN, and no document states the rider as ruled in either
  direction. phase unchanged (build).
- 2026-08-24 — "(a) continue." → the fork-3 RIDER is RULED (a) BY THE OWNER: the
  load-time rejection of a predicate reading its own guarded cells is REMOVED —
  the pattern is legal per SPEC §13.9.6 and the 022 gate family (self-conditional
  gates; prior-commit reads define it). `gate_predicate_reads_guarded` and its
  check/tests are deleted; FND-36 closes RULED (a). "continue" → the parked wave-2
  pipeline resumes (engine fix standalone, then the workflow from cache). phase
  unchanged (build).
- 2026-08-24 — "I understand now and yes, I accept your proposal regarding order.
  be careful." → MILESTONE ORDER RESHAPED (direction ruling, LEARNINGS #39): after
  M1 closes, the next milestone is a thin end-to-end vertical (small source
  subset → one behavior-compilation path → one drawing effect) so a Ductus
  program runs before any layer widens; widening milestones then implement the
  full spec construct by construct. Nothing is skipped — the owner rejected
  "unimplemented spec" outright; the widening schedule (M1b wires/streams and
  onward) stays explicit. STRATEGY.md is amended to record the reorder when M1
  closes. The vertical's plan and its forks come to the owner before any code.
  phase unchanged (build — M1 wave 2 still running).
