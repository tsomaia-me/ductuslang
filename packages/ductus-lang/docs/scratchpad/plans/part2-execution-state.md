# Part-2 Execution State — post-compaction onboarding document

*Maintained by the assistant; updated 2026-07-16 before the structure-batch launch. Read this
FIRST after any compaction, then: the rulings ledger at the top of
audit-plan-2-master-decisions.md (authority on every ruling), LEARNINGS.md at repo root
(all 26 items — binding working rules), and the plan docs named below. Never trust recall of
LOG/SPEC content — verify on disk (LEARNINGS #3); verification = full diff read, not greps
(#23); ruling requests protocol-complete but TERSE per the #26 calibration.*

## Where everything stands (chronological, all owner-committed unless noted)

1. **Part 1 (mechanical M-01..M-30)** — done, committed long since.
2. **Part 2 Phase A (D-01/D-02/D-03)** — Model B kinds, four-group taxonomy, kind-position rule.
3. **Stream/group-class batch** — D-18/D-22 evictions, yielded 7th declaration keyword
   (`yielded name: T = collect:`), collect-as retired, P026, GROUP-SNAPSHOT (auto-materialize
   in value positions; groups NOT Iterable; loss law; O(n)/O(log n) cost note), D-15 (views
   structural-only), CONSTANT-WRAP (derived T), D-31(b) (IR_GRAMMAR normative, GRAMMAR not),
   D-25 (stream production both grammar docs), NORMALIZATION.
4. **D-04 phase** — fold as derived (4-tag enum, no yielded tag), membership-descriptor cell
   (`derived`-kind, typed `yielded<T>`, no `uses`), L1 derived liveness, leave-is-dirty-root,
   walk-order sequence vs birth-coordinate identity split, `keyed_by_on_yielded_group_rejected`.
5. **D-20/21/26/30 batch** — match unrolls/never freezes (+§6.2.5 subsection), walk order
   (034-18) / member order (035-12) / coordinate identity (034-19) definitions, coordinate
   respells, per-declaration desired cells (whole-record demoted to reconciler view),
   effect-instance identity = (scope, type, bindings) with keyed-repeat scope term, :N
   demoted to addressing.
6. **Stream-type retirement + sealed amendment** — `Stream[T,P]`/`RingStream`/`GateStream`
   retired as types; `stream[P] T` generic spelling, bracket form legal concretely, word form
   sugar; wiring-types two-level vocabulary (keyword = kind; applied annotation = wiring TYPE,
   unstorable); sealed de-magicked (user-writable, module rule); stdlib-vs-language boundary
   (operators stdlib; to_signal stdlib — expressible via `observe`, whose result kind is
   context-determined per 016-247); numeric conversions language-provided; bracket form in all
   declaration heads; 'policy keyword' prose retired.
7. **Traits batch (D-12/D-13)** — observed cells never block the (then-)waiver, precise
   auto-sat rule ("does the default body cover the contract?"), effect methods excluded from
   ordinary dispatch as algorithm step 2 + diagnostic, collision membership kept, three call
   forms value-only.
8. **Satisfies-retirement amendment (LARGEST; committed)** — `satisfies` clause fully retired;
   conformance IS the fulfill block; bodiless fulfill (marker claim / conditional via where /
   nothing-to-override / optional pin); single-fulfill coherence (005-66, coverage-harmonized:
   legal sites = orphan-admissible modules incl. local generic-argument and tuple-element
   coverage per 010-20/012-64; >1 writes = error naming all sites); grounded auto-satisfaction
   (005-246: empty traits never auto-satisfy — the claim is load-bearing); sealed uniform over
   nominal kinds (`sealed type/enum/node/connection`; tuples/closures conformable via coverage
   but NOT sealable — no declaration); `sealed_type_fulfillment_outside_module`; the
   clause/verb razor (backticked `satisfies` = clause = retired; unbackticked verb survives).
   Plan doc: satisfies-retirement-plan.md. Section 005 now 246 rules.

## IN FLIGHT at time of writing

**Structure batch launching now** (owner: "proceed with this batch", 2026-07-16). Scope:
- D-09 Model A relabels (Handle→WeakHandle in every dynamic context) + BOTH defaults confirmed
  (P020 mislabel reading: 017-111/017-147/SPEC §13.3.4; P021 relabel: 017-136 third form).
  Sites (stale lines, locate by content): SPEC §13.8.4, §13.8.5.1, §13.6.2 area, §13.3.4,
  §13.3.4.2 wheel example; LOG 019-49, 021-57. Class floor: any Handle with
  Option-read/freeze/re-point/dynamize behavior → WeakHandle.
- D-14 Model A: 018-64 widens StringifiableKey to i8–i128,u8–u128,isize,usize,bool,char,string,
  duration,instant; defined as THE SAME canonical set as the Map key bound (one list, both
  reference it; reconcile 012-100); SPEC §13.5.4.1 enumeration + §9.5.3 cross-ref; verify-reads
  018-63/69/71/42/44/58/127/142/83; NO coercion rule added.
- D-16 Model A: 017-89 (zero rows; offsets length = rows + 1 invariant pinned), 017-90 +
  012-90 (bundle.count = ROW count, intentional element-tally exception); SPEC §13.3.3.5.
- D-24 as a GATE only (grep: no package-worded orphan phrasing survives).
- NOT included (owner never nodded despite two asks): the two riders — SPEC §3.3
  AssocTypeBinding `'='`→`'is'` (LOG 005-92 + GRAMMAR say 'is'); SPEC §13.3.7 at-most-once
  list missing the acceptance clauses 017-226 lists. STILL QUEUED, ask again.
- Owner Q&A recorded: handle stays a bracket TYPE (storable value; razor's positive case; the
  `handle`/`handle!` keywords are the guarded acquisition operations — named-place-only, no
  constructor, 017-187 proof for handle!). No `handle T` kind exists or is wanted.

## QUEUE after the structure batch (in order)

1. **Streams batch**: D-19 (DRAIN completion semantics — ruled with sub-answers per item) +
   the observe-per-event gap (UNRULED: does an observe arm with a stream trigger evaluate once
   per EVENT or per COMMIT? Irrelevant to to_signal, load-bearing for accumulate's every-event
   fold — present protocol-complete with the batch).
2. **Policies batch**: execute D-06 (repoint policy — largely applied as-we-went; sweep),
   D-07 residual consolidations, D-08 (hook order rules NEVER YET WRITTEN into LOG/SPEC:
   post-publish inside blocking commit(), teardown→create→resume→update, initial create =
   first post-first-commit pass, reload fires the tail; exec block at plan line ~485),
   D-10 (Model 2 live membership), D-11 (fence tags: text/ductus-ir/ebnf/rust), D-28
   (immutability scoped to within-the-program), D-29 (std::vec::new()). PLUS the D-05
   residual rulings as the FIRST CONSENT-LIST (keyword classes for own/move/public/private;
   `cell` reserved?; sweep scope; `sealed` class; freed `satisfies`+`main` identifiers).
3. **Main-removal amendment**: plan doc main-removal-plan.md parked on EIGHT confirms
   (§3.1-3.8) + Phase-0 fold decision + two additions from later discussions: the
   reference-reachability closure pin (does a live-code reference pull a dead placement into
   the root closure?) and the unwired-instance ownership note (typecheck is liveness-blind;
   ownership rule fires textually).
4. **Part 3 re-anchor survey** (owner-agreed protocol): classify all ~80 items of
   audit-plan-3-individual-decisions.md as DISSOLVED (killed by Part-2 rulings — Cell[T],
   stream types, main, satisfies, view-Iterable all gone) / EXECUTED / STALE / LIVE; then
   LIVE items in themed consent-lists (one-liners: problem → recommendation → why; owner
   approves wholesale, strikes to escalate; only struck items get full treatment).
5. **Small standing flags**: proposed 001 philosophy entry ("privileged in performance,
   never in capability") awaiting nod; satisfies-flag name kept (verb-derived, noted);
   IR behavior-grammar ret/operand gap (own item); IR_GRAMMAR §2/§6 citation-drift sweep
   (033-166/167/168 offsets); duplicate keyed-by keys on value collections (LOW);
   optional strict-kind vocabulary sweep (016-167/030-48/49/029-124 loose "kind" usage).

## Working protocol (hard-won, do not relearn)

- Architecture per owner: I orchestrate via ONE Fable-5 orchestrator agent (general-purpose,
  model "fable") per batch; it spawns SYNCHRONOUS opus menials (haiku for greps), NEVER fable,
  never background children, never idle-waits (600s watchdog kills); same-file editors
  sequential; 3 scoped blind reviewers for big diffs, 1 for small; per-file greps (rtk proxy
  truncates multi-file); no commits ever — the owner commits.
- My verification chain before reporting done: independent greps → reviewer adjudication →
  MY end-to-end read of the full diff (LEARNINGS #23). Then report with owner flags
  protocol-complete-but-terse (#24, #26).
- Approval literalism: only explicit launch words from the owner; rulings ≠ launch.
- Orchestrator crash recovery: verify disk state via git diff entry-id inventory, relay to a
  resumed/fresh orchestrator with a do-not-re-edit landed list. Subagent SendMessage to
  orchestrators often fails — completed-child reports may arrive in the main session; verify
  claims on disk, then relay via SendMessage to the orchestrator id.
- Ledger updates: I patch audit-plan-2's ledger via python replace (assert count==1) after
  every ruling, BEFORE launching work that depends on it.

## Naming sidebar (paused, owner-driven)

Ductus is a placeholder (name taken). Sid rejected (Debian/Oracle/C64/Sidamo collisions).
Method: name the channel, not the flow; mine specialist lexicons; screen registries.
leat = usable-with-asterisk (crates.io reserved by a same-niche audio-graph project,
codeberg.org/Azorlogh/leat; GitHub user squatted; PyPI free; search clean).
Candidates offered: Runnel, Runlet (run+let!), Millrace, Swale, Adit, Anabranch, Rhei
(panta rhei), Qanat, Swash, Thalweg, Ductor, Rheon, Alvea, Arkhi (არხი — Georgian for
channel; owner is Georgian). Owner liked leat; sweep of favorites on request.
