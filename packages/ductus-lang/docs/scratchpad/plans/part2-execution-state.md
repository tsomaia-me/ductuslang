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
9. **Structure batch (D-09/D-14/D-16, D-24 gate)** — committed fd3cf60. WeakHandle in every
   dynamic-handle context; StringifiableKey = the canonical key set shared with Map; bundle
   empty-literal zero rows + offsets=rows+1 + count-is-row-count carve-out.
10. **Streams batch (D-19 DRAIN + granularity)** — committed 69caff7 (commit MESSAGE says
    barrel plan; CONTENT is the streams batch — mismatch disclosed and recorded).
11. **Streams-rework amendment (kind-homogeneous observe + arrow crossings)** — committed
    (this commit). §016 → 295 Rules: observe's kind = its triggers' kind (016-247 rework);
    mixed signal+stream trigger sets = compile error (016-292); banned quadrants spelled
    (016-293 stream→value = ->latest only; 016-294 signal→stream = `on sig->changes as e:`);
    value-kind lockstep, no chaining (016-289/290); stream-kind per-event in commit order
    (016-287); sampling law (016-295). §030 → 273 Rules: crossings as language forms
    (030-269..273) — `sig->changes` / `stream->latest(fallback)`; razor "dot projects, arrow
    mints"; `->` third positionally-disjoint role; CrossingName closed list, NOT reserved.
    to_signal + member-.changes RETIRED everywhere (16+13 sites); accumulate/event_count/
    any/all = scan+`->latest` compositions, zero privileged operators. GRAMMAR: §5.2
    ArrowAccess/CrossingName; §5.20 observe productions conformed (OnTarget gains
    ArrowAccess?, missing as-binder slot added, stale citations refreshed to
    016-243/244/259/261 + 292/294 — all verified on disk). Executed defaults disclosed:
    multi-trigger set elements each admit a crossing (`on (a->changes, b->changes)` parses;
    homogeneity is post-parse) — no entry restricts crossings to single-trigger arms;
    030-266 one-bullet respell per plan §3.B. Pre-existing nit: §13.19.2-area
    `ws.inbound |> count` should be event_count (NOT fixed — out of scope, flag standing).

## IN FLIGHT at time of writing

**Barrel-visibility batch launching now** (plan APPROVED by owner 2026-07-18; standing
authorization). Policies batch landed + committed (D-05 six items, D-06/07/08/10/11/28/29;
new entries 002-31..35, 027-121..123, 031-159; open flags from its report: suspend unplaced
in hook order — protocol-complete question OUT to owner; 'policy keyword' 3 pre-existing LOG
hits 030-19/030-244/031-32 — owner to rule; GRAMMAR/IR_GRAMMAR fences left bare, SPEC-scoped
convention — owner to rule; dyn class deferred in 002-34 — one ruling away). Riders
elaborated protocol-complete 2026-07-18 (SPEC §3.3 AssocTypeBinding '=' is the lone deviation,
LOG 005-92 + GRAMMAR §7.12 + all worked examples say 'is'; SPEC §13.3.7 at-most-once list has
4 members vs 017-226's 7 — children:/incoming:/outgoing: missing) + 001 philosophy entry
(next free 001-39; principle verified absent from corpus; nearest kin 005-22/005-172).
Standing owner Q&A: handle stays a bracket TYPE; no `handle T` kind.

## QUEUE (in order)

1. **Policies batch**: execute D-06 (repoint policy — largely applied as-we-went; sweep),
   D-07 residual consolidations, D-08 (hook order rules NEVER YET WRITTEN into LOG/SPEC:
   post-publish inside blocking commit(), teardown→create→resume→update, initial create =
   first post-first-commit pass, reload fires the tail; exec block at plan line ~485),
   D-10 (Model 2 live membership), D-11 (fence tags: text/ductus-ir/ebnf/rust), D-28
   (immutability scoped to within-the-program), D-29 (std::vec::new()). PLUS the six
   APPROVED D-05 items (consent-list DISPOSED — see ledger): ownership class own/move; kind
   class with cell; declaration-modifier class with sealed; Model-A keyword assignments
   (enum/requires/wraps/effects/given/observe) + SPEC keyword-paragraph re-sync; FULL sweep
   reconciling LOG keyword classes with GRAMMAR §2.4 as ONE taxonomy; satisfies+main freed
   silently (zero-mention verify). Visibility-class item stays PARKED to the barrel batch.
2. **Barrel-visibility amendment** (owner-settled 2026-07-16, "that settles it"): plan doc =
   barrel-visibility-plan.md, ALL rulings locked (in-source public+shared retire; private sole
   in-source keyword; public.duc barrel at package root with `export` entries, no glob;
   no-barrel = nothing public; leak rule = compile error; member model (i); one barrel per
   package; `type(private) Email:` hosts constructor visibility; `export` reserved, never
   `expose`). Runs AFTER the policies batch. D-05's visibility-class item resolves here.
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

## STANDING AUTHORIZATION (owner, 2026-07-16, verbatim intent)
"and you commit. don't wait for me. as soon as structure batch finishes: verify, commit, start
the streams batch. IF there will surface any items that need my calling, surface them, of
course." — So: after each batch passes the full verification chain (greps + reviewer
adjudication + my end-to-end diff read), I COMMIT it myself (descriptive message, house style,
no push unless told) and IMMEDIATELY launch the next fully-specified batch in the queue.
Owner-flag items still get surfaced; genuinely blocking items still halt the chain.

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
