# Ductus Interpretation RFC — Amendment Plan v2

Target repo: `/Users/torniketsomaia/projects/@tsomaia.tech/ductuslang/packages/ductus-lang/docs/`
Files under edit: `DECISION_LOG.md` (4312 lines, sections 001–033) and `SPEC.md` (~1.0 MB).

---

# PART 0 — PROVENANCE AND HOW TO USE THIS PLAN

## 0.1 What this document is

This is the complete, self-contained execution plan for amending the Ductus DECISION_LOG
and SPEC with the Interpretation RFC design. It supersedes plan v1. It is written so that
a reader with **zero session context** can execute it: Part A serializes the full decision
record (nothing is elided); Part B is the complete LOG edit inventory for every existing
section 001–033; Part C specifies the two new LOG sections; Part D is the SPEC edit
inventory; Part E is the execution order; Part F is the acceptance checklist.

Sources of record, in precedence order:
1. The canonical decision record (design decisions C1–C18, rulings R1–R11, verification
   digest W2, editor constraints, rejected/deferred lists) — serialized in full in Part A.
2. Verification passes W1/W2 (file-fact checks against DECISION_LOG.md and SPEC.md; W2's
   corrections (a)–(n) BIND this plan and are serialized in A.4).
3. The gate review of plan v1 (18 findings), all of which are addressed in this v2; the
   finding→fix map appears in Part 0.4.

Execution model: **LOG first, then SPEC.** All LOG edits land per Part B/C in the wave
order of Part E; SPEC edits (Part D) follow, then the Part F acceptance checklist runs.

## 0.2 Legend — every code family used in this plan (read before Part A)

- **C1–C18** — the eighteen design decisions of the RFC (serialized in full in A.2).
- **R1–R11** — post-verification rulings, all locked (serialized in full in A.3).
- **W1/W2** — the two verification passes over the LOG/SPEC. W2's corrections are cited
  as **W2(a)…W2(n)** and are serialized in A.4.
- **INV1–INV3** — the LOG's own editorial invariants (serialized in A.1). They govern
  every edit in Part B/C.
- **Editor constraints EC1–EC8** — hard constraints on the editor (A.5, numbered).
- **⚛A–⚛D** — atomic edit groups: sets of edits that MUST land in one commit because they
  restate one rule at several sites. ⚛A = the reconciler-registration conditional (see
  B.2). ⚛B = the Cell-umbrella coordinated edit (six LOG entries + both SPEC sides, EC4).
  ⚛C = the SPEC §13.1 four dead host-claims (EC5). ⚛D = the two-axis StreamPolicy remodel
  (030-255/259/262 + SPEC §13.18.3).
- **Act column vocabulary** (Part B/D tables):
  - **RW** rewrite — entry text changes, entry survives under (possibly renumbered) id.
  - **SPL** split — one entry becomes two (its claims diverge); both new texts given.
  - **DEL** delete — entry removed; later entries in the section renumber (INV1).
  - **ADD** add — new entry inserted at a stated anchor point; later entries renumber.
  - **PR** protect — entry must survive **byte-identical** (HARD) or without meaning
    change (soft); listed in the Protect Register (B.1).
  - **EXT** extend — entry text gains a clause; original claim preserved.
  - **CON** contradicted — the entry's claim is reversed by the RFC; rewrite states the
    new truth.
  - **RES** respell — surface-only reword (naming, spelling, kind syntax); claim intact.
  - **ADJ** adjacent — entry is touched only as context: no text change, or a clarifying
    note that does not alter the rule. ADJ rows exist so coverage is auditable.
  - **SUP** superseded — entry's rule is replaced by a new entry; old entry rewritten to
    the surviving residue (or DEL if nothing survives — stated per row).
- **Anchor quote** — the entry's opening text as it exists in DECISION_LOG.md today,
  verified by grep during the writing of this plan (line numbers cited are pre-edit).
- **LOG entry id format** — `SSS-N.` at line start; sections are dense (INV1), so every
  ADD/DEL renumbers the section tail. Part B anchors always use **pre-edit ids**.

## 0.3 Terms of art (plain-language, defined once here)

- **Interpretation** — the host- or program-side act of walking a node's exposition and
  giving meaning to what it finds there. This RFC makes interpretation expressible IN
  Ductus (via effects walking `.exposition`) instead of host-only.
- **Entry sum** — `.exposition` becomes a typed, language-owned, CLOSED sum of five entry
  kinds: `Node / Connection / Bundle / DynamicView / Gated`. "Closed" = user code can
  never add a sixth kind; "language-owned" = the sum type is defined by the language, not
  the stdlib. Walking is `for entry` + `match` over the five variants.
- **Wire-following** — an interpreter reaching through a connection ("wire") to the
  node on its far end. The RFC's invariant: **wires ACTIVATE, never instantiate** — every
  render an interpretation could ever need is mounted once at startup (one render per
  (root, instance) pair, at stable paths); a wire arriving/leaving merely opens/closes a
  compiler-synthesized wake gate on the target's render. Why: instantiation-on-wire would
  make graph shape a runtime function of data, breaking the static-graph guarantee and
  hot-reload path identity.
- **Interpretation closure** — the static set of instances an interpretation root can
  reach = containment closure UNION wire-candidate envelopes (both static sets).
- **collect / yield** — `collect:` is a block expression that builds a living membership
  structure; each `yield <expr>` inside contributes one member-cell whose membership
  tracks whether its position is live. Not a coroutine: evaluated once.
- **fold form** — a language expression form combining a membership-varying group with a
  user combiner (`by:`) and an empty-membership result (`else:`). Semigroup + default; NO
  identity element; `Monoid` is deleted from the design.
- **yielded kind** — `yielded T`: the ordered, membership-varying group of cells a
  `collect` produces. Stores nothing (compile-time wire set + runtime on/off bits).
- **Kind sweep / storability razor** — C14's rule of thumb: storable values keep bracket
  types (`Handle[T]`, `Portal[T]`, `Type[C]`, `WeakHandle[T]`); reactive binding
  machinery becomes lowercase keyword KINDS in annotation positions (`signal i32`,
  `derived f32`, `stream ring[1024] Event`, `yielded f32[128]`, `cell T`, …).
- **observe as-binding** — `on <trigger> [where C] [as <binder>]:` — the R5 grammar
  giving observe arms access to the post-filter event value via an explicit binder.
- **Reconciliation contract** — the new SPEC §13.19 framing: every effect is a
  reconciliation contract — fulfilled by a host reconciler (leaf effect), by child
  effects (interior effect), or both.

## 0.4 Gate finding → fix map (all 18 findings of the v1 review)

| # | Finding (class) | Fixed where in v2 |
|---|---|---|
| 1 | COVERAGE/BLOCKER — Part A missing | Part A authored in full (A.0–A.7) |
| 2 | COVERAGE/MAJOR — no 001–029 rows | Part B complete tables incl. explicit rows for 005-9, 005-58, 016-139, 016-267, 017-151, 017-236, 018-57, 018-109, 029-18, 029-41, 029-44, 029-45, 029-103 |
| 3 | COVERAGE/MINOR — 016-139/016-267 unaddressed | B §016 rows + Part D anchors (D.3) |
| 4 | PRECISION/MINOR — 029-74 quote missing "RHS" | Corrected: LOG 029-74 (L3421) reads "with a `fn` RHS"; SPEC L19526 reads "with a `fn`" (no RHS) — the two texts legitimately differ; protect checks key off the true texts (B §029, D.6) |
| 5 | PRECISION/MINOR — §13.1 line cites off | Corrected to L11355-56, L11370-71, L11377-81 (verified; D.1) |
| 6 | ONBOARDING/BLOCKER — no orientation/Part A | A.0 definitions + 0.2/0.3 legend |
| 7 | ONBOARDING/BLOCKER — 030 + 001–029 tables absent | Part B complete incl. full 030 table |
| 8 | ONBOARDING/BLOCKER — 017 "~+18, pin later" | Exact +18 → **326**, all 18 ADDs enumerated (B §017); deferral deleted |
| 9 | ONBOARDING/MAJOR — 017 budget discrepancy | Reconciled: cluster budget 2+2+3+1+7+3 = **18** (the reviewer's "sums to 15" was a miscount; the ambiguity finding stood and is resolved by full enumeration) |
| 10 | ONBOARDING/MAJOR — 030-123..131 premise unauditable | All nine rows shown with explicit Act; −4 (DEL 123/128/129/130) +2 (ADD .changes pair) → 260 (B §030) |
| 11 | ONBOARDING/MAJOR — ⚛A 5/6/7 site ambiguity | Canonical ⚛A = **6 core LOG sites** (015-39, 027-80, 027-81, 031-128, 031-143, 033-122); the record's "five LOG sites" label is superseded by its own authoritative enumeration (which lists six ids); support sites 031-119 + new 031-157 are NOT counted; 3 SPEC sites + 2 coordinated companions. Same list verbatim in B.2, E (Wave 3), F.24 |
| 12 | ONBOARDING/MAJOR — undefined code families | 0.2 legend |
| 13 | ONBOARDING/MAJOR — Part D not standalone | D.0 respell mapping table; every respell row cites it |
| 14 | ONBOARDING/MAJOR — entry-sum / wire-following undefined | 0.3 + A.0 |
| 15 | ONBOARDING/MINOR — protect-list drift | Single Protect Register (B.1); E.5 and F.17/F.20 reference it |
| 16 | ONBOARDING/MINOR — F.13 grep bound | Widened pattern + stated section-range assumption (F.13) |
| 17 | ONBOARDING/MINOR — F.22 not mechanical | 017 pinned at 326; F.22 fully mechanical |
| 18 | PRECISION note — "~" tolerance | All verified citations tightened to exact lines throughout |

---

# PART A — COMPLETE DECISION RECORD

This part is the single source of truth serialized in full. A reader who has ONLY this
plan can onboard from Part A alone. Nothing below is excerpted or summarized away.

## A.0 Plain-language orientation — what the amendment IS

The RFC makes **interpretation** a first-class, in-language activity. Today the LOG/SPEC
treat effects as leaf-only host contracts and interpretation as a host-side act. After
this amendment:

1. **Effects compose** (C1). An effect body may place child effects before its
   `desired:`/`observed:` blocks. `observed:` is the effect's public surface; child
   placements are private. An effect is a *reconciliation contract*: fulfilled by a host
   reconciler (leaf), by child effects (interior), or both. Host registration is required
   only iff `observed:` has host-written channels — one conditional, landed atomically
   (⚛A).
2. **Traits can declare effect-kind methods** (C2) with `observed:` contract blocks, so
   interpretation logic is written against traits (e.g. `effect render(value: Subject):`).
3. **Interpretation bootstraps as an ordinary effect call** in `effects:` (C3) — no new
   top-level construct. `render(song).audio |> audio_out`. The `|>` operator gains a
   third dispatch case (node-reference LHS, effect-kind trait-method RHS).
4. **`.exposition` becomes a typed closed sum of five entry kinds** (C4) —
   Node/Connection/Bundle/DynamicView/Gated — walked with `for` + `match`, with a new
   entry-match-only bounded pattern `Variant(name: Bound)` monomorphized like a generic
   parameter. Why closed: the language owns the exposition vocabulary, so interpreters
   are exhaustively checkable and statically monomorphizable.
5. **Bundle rows, gated arms, and repeats are walkable** (C5, C6, C7): `[B]` matches a
   whole bundle row; `Gated(arms)` exposes all arms at compile time with positional
   gate_parents; `repeat` lifts to expression position and gains a typed filtering
   binder `repeat (child: Bound)` over dynamic views.
6. **Living collections** (C8, C9, C10): `collect:`/`yield` build a `yielded T` —
   an ordered membership-varying group of cells — consumed by the new `fold` expression
   form (semigroup combiner + `else:` empty result; O(log n) recombine cost; new `fold`
   cell KIND in the IR; the six-primitives count is untouched).
7. **One uniform cell-argument rule** (C11): any expression in a cell position resolves
   by provenance (bare cell binds; static wraps as constant cell; reactive synthesizes a
   derived bridge). 029-108 is deleted.
8. **Conversions cleaned up** (C12, C13): `to_ring_stream`/`to_gate_stream` are deleted
   in favor of a synthesized per-use-site `sig.changes` stream member; `to_signal`
   becomes an ordinary user-written operator; observe arms gain `as` event binders (R5).
9. **Kind sweep** (C14): reactive binding machinery is spelled as lowercase kinds
   (`signal T`, `derived T`, `stream ring[N] T`, …); `Cell` becomes a KIND, resolving the
   sextupled type-vs-trait contradiction; two-axis stream policy model.
10. **Map becomes insertion-ordered; `.count` unifies element tallies** (C15). HashSet
    stays unordered — every co-occurrence entry is SPLIT, never wholesale rewritten.
11. **Wire-following interpretation** (C16, C17): connections may surface endpoint data
    via their own deriveds (opt-in); interpretation closure = containment UNION
    wire-candidate envelopes; one render per (root, instance) mounted at startup; wires
    ACTIVATE, never instantiate (compiler-synthesized wake gates, prior-commit reads,
    one-commit-delay fixpoint); interpreters read a target's declared incoming
    connection-views by NAME through the node reference; meaning stays extrinsic.
12. **Misc** (C18): Keyed/StringifiableKey become language-defined; WeakHandle AND
    Portal resolution are the two dynamic-dependency sources; slice-of-Copy
    materialization to owned `T[N]` params; interpreter-placed effect paths mirror node
    paths with `:N` ordinals.

## A.1 LOG invariants (govern all edits)

- **INV1** — dense positional numbering: inserting/removing an entry renumbers all later
  entries in that section.
- **INV2** — entries are atomic; cross-references between entries are FORBIDDEN; rules
  are restated locally wherever needed.
- **INV3** — LOG→SPEC one-way (SECTION) refs; SPEC never references LOG; every new entry
  needs a SPEC section ref.

## A.2 Design decisions C1–C18 (complete, every clause preserved)

**C1 COMPOSABLE EFFECTS.** Effect bodies may place child effects as top-of-body items
(`x = effect_expr`), BEFORE `desired:`/`observed:`. Lifecycle cascades
(mount/teardown/suspend/resume with parent). 031-134 lifted PARTIALLY: top-of-body
placement allowed; effect-instantiation as a desired-cell EXPRESSION stays an error.
017-255 amended (effect bodies join `effects:` clauses as instantiation sites). Body
order: body items → `desired:` → `observed:` LAST (supersedes 031-16 when body items
present; mirrors 017-211). `observed:` is the public surface of the effect; child
placements are PRIVATE. `observed:` ACCEPTS `derived` and `recurrent` as computed
outputs (VALUE recurrents; host-fed `recurrent[N]` STREAM in `observed:` stays rejected
— split, do not blanket-lift: 031-49/146/152, 030-117/118). `signal`/`stream` in
`observed:` remain the host-written channels; the two-channel host-WRITE count (016-283)
is PRESERVED. Reconciler registration required IFF `observed:` has host-written channels
— this conditional must land ATOMICALLY at five LOG sites (031-128, 031-143, 027-80,
027-81, 033-122, 015-39) and three SPEC sites (13.14.7 ~18713-16; 13.19.14 + 15.4.1
~22027-30; diagnostic ~22116-23). [Plan note: the label "five LOG sites" is superseded
by the authoritative enumeration in the same sentence, which lists SIX ids — see B.2.]
Placements orthogonal to host channels: MIXING ALLOWED. Framing for SPEC 13.19: every
effect is a reconciliation contract — fulfilled by a host reconciler (leaf), by child
effects (interior), or both.

**C2 TRAIT EFFECT METHODS.** Traits may declare effect-kind methods
(`effect render(value: Subject):`) with `observed:` contract blocks (required output
cells; contract = MINIMUM, fulfill may expose more; consumers projecting through the
trait see only contract cells) and default bodies. Waiver: traits where EVERY method
(incl. effect-kind) has a default body AND that declare no required cells allow
`fulfill` WITHOUT `satisfies`; `satisfies` stays for abstract-method and required-cell
traits. 005-67 rewrite must SEPARATE auto-satisfaction from this waiver. Effect-method
names join the method-name collision namespace (005-69/74/76/80). Effect-method traits
are effect-specific (005-51 contradicted).

**C3 BOOTSTRAP/LIVENESS/RECURSION.** No new top-level construct — interpretation
bootstraps as an effect call in `effects:` (`render(song).audio |> audio_out`;
projection explicit per 031-117). The `|>` gains a THIRD dispatch case: node-reference
LHS when RHS is an effect-kind trait method with Subject-typed first param (fix homed at
029-65/106; the fn-RHS error 029-74 stays — NO fn exemption). 021-140/141 reachability
closure gains: node references bound as effect arguments join the closure (as BORROWS,
not cell stores — 017-162 intact) AND wire-candidate envelopes. 004-45 gains carve-out:
compile-time interpretation expansion exempt from the polymorphic-recursion ban, bounded
by the finite interpretation closure (004-47 contradicted; termination leans on
017-26/27).

**C4 ENTRY SUM.** `.exposition` becomes a typed language-owned CLOSED sum of five kinds:
Node/Connection/Bundle/DynamicView/Gated (017-210 superseded). Walk = `for entry` +
`match`. New pattern `Variant(name: Bound)`, Bound = trait or concrete type, scoped to
ENTRY MATCHES ONLY (write as an entry-match-only carve-out, not a general pattern
shape). Bounded arms are filters, never count toward exhaustiveness; each variant needs
an unbounded arm (`Node(_)`) or the match a final `_` (009-84 wildcard substrate).
`Node(n)` = `Node(n: Node)` (intrinsic marker). Declaration order, first match wins;
broad-before-narrow dead-arm lint. Binding = generic-parameter semantics, monomorphized
(static entries at compile-time unroll, zero runtime tests; dynamic elements once at
mount via tag over the closed candidate envelope 023-56). Compiler can emit
per-interpretation-root participation/skip report. Trait-headed match over RECORDS stays
BANNED — this ban is free-floating and needs a NEW anchor entry (not 009-90).

**C5 BUNDLE PATTERN.** `[B]` in bundle-payload pattern position matches a whole row
whose every element satisfies B; binding = row slice `Handle[B][..]` (017-97 type).
Bracket trilogy: write (017-89) / accept (017-93) / match (new). NO `T[]` synonym for
`T[..]`. NO `[B ...]` partial-row matching in v1 (R7); 006-32 closed enumeration intact.

**C6 GATED ENTRY.** `Gated(arms)` binds a compile-time fixed-extent sequence of arm
bodies, EACH the same walkable type as `.exposition` (update 017-224 prose to say so —
R10). Interpreter walks ALL arms at compile time (Model B mirror). Positional guarding:
structural output while walking arm i gets the gate of arm i as gate_parent (033-65).
Value contributions join folds as ACTIVATION-DRIVEN members (present iff effectively
active 022-67) — a genuinely NEW fourth gate read-path vs 017-299/300 frozen reads.
Predicate/scrutinee never surfaced (022-14). Three existing gate read-paths unchanged,
documented as one system (022-56/57 connections do not deliver; 022-70/71 direct reads
frozen; 017-103 static views include frozen members).

**C7 REPEAT EXPR/BINDER.** `repeat` liftable to expression position (yields exactly what
`repeat ... as X` binds; `as` = sugar). Carried keys over dynamic views (018-65/78). New
binder: `repeat (child: Bound)` in `dyn:` — PARENTHESIZED typed binder (mirrors match
payload patterns; bound-colons always inside parens); legal ONLY over dynamic
views/connection-views (entity references, closed envelopes); FILTER semantics,
per-candidate monomorphization, no scope for non-matching members, no exhaustiveness
implication. Value collections keep bare/tuple binders only (bounds over values = record
type-case, banned).

**C8 COLLECT/YIELD.** NEW construct: `collect:` block expression (+ `collect as x:`
statement form; both legal). `yield <expr>` = one member-cell; ONE invariant meaning:
member while this position is live. Positional membership: static position = permanent;
inside `repeat` = key-driven; inside gated arm = activation-driven. Bare `yield`
directly under `collect` legal; `yield` outside `collect` = compile error. Not a
coroutine — `collect` evaluates once, builds a living membership structure. `yield`
accepts any expression: reactive provenance → its cell; static → degenerate constant
cell (029-20/31). RULING R4: `yield` under a value `if`/`match` inside `collect` is
LEGAL iff the condition/scrutinee is compile-time-known (conditional expansion, rule
shape of 021-46); COMPILE ERROR when reactive/runtime, diagnostic offering `when`/`given`
arms (membership switching) or a conditional VALUE yield (`yield if c: a else: b`).
022-5 (if/match never gate structure) preserved. New LOG section 034.

**C9 YIELDED KIND.** `yielded T` — ordered (walk-order), membership-varying group of
cells; stores nothing (compile-time wire set + runtime on/off bits; 023-19 machinery);
cell-KIND (membership changes propagate dirt). Consumption: the fold form; yielded-typed
params (operators AND fns); `repeat` over it. NO indexing, NO compile-time `for`
(structural `for` over Yielded = compile error, 014-71). Yielded is NOT a dynamic-view
consumer and does NOT join the dynamic-view consumer surface (017-307 and family are ADJ
— W2 correction). Fulfills Iterable at LANGUAGE level (like Keyed/StringifiableKey —
author as language-DEFINED fulfillment, cross-ref 005-22/011-40): Item = T member
values, walk order; runtime-loop contexts only (fn/derived/operator bodies). Synthesized
`count` member.

**C10 FOLD FORM.** Language EXPRESSION form (not operator, not method — behaviors opaque
032-107). Block: `fold <members>:` / `by: <fn(T,T)->T expression>` / `else: <T
expression>`. Newline-separated arms, no comma, both mandatory, `else` last. `else` =
EMPTY-MEMBERSHIP result, NEVER combined (semigroup + default; NO identity; Monoid
deleted from the design). Associativity asserted by using the form. Normative cost rule
(family of 014-148, 025-40, 012-107): O(log n) combines per member
value-change/join/leave; deterministic tree from member order; commutativity NOT
required. Scope: Yielded + reactive composites with uniform slot type (members = slots
in declared order; zero slots → else). NOT `Cell[Vec[T]]`. Joins operator-body items
(029-29; forces indented block 029-30) and derived RHS. RULING R6: IR lowering = a NEW
CELL KIND: 033-80 kind enum gains `fold` (input|derived|recurrent|fold); six-primitives
count (033-56) UNCHANGED — editors must not touch it; fold-kind cell carries combiner
behavior id, else value, member edges in member order with membership drivers
(permanent / keyed-template / gate-guarded). Surface unchanged: result lives in a
`derived` declaration; consumers see `derived T`. New LOG section 035 (or sibling home).

**C11 UNIFORM CELL ARGS.** 029-108 DELETED. One rule for placement attrs, operator args,
effect args, `|>` LHS: a cell position accepts ANY expression, resolved by provenance —
bare cell binds directly (029-22); static expression wraps as degenerate constant cell
(029-31); reactive expression synthesizes an implicit derived bridge (021-16 mechanism
extended to arguments; bridge path from call site per 029-33). 029-27 rewritten
(dichotomy incomplete). Instance identity unaffected. IR: `parameter_bindings` gains a
provenance MARKER (per 033-234 precedent), NOT a third binding case (R8).

**C12 CONVERSIONS.** Sampling rule STAYS (030-67, wall 030-88/90/91) — rationale
(emit-on-either fabricates phantom events; either-changes already exists as derived;
per-operand control) goes to SPEC 13.18.7 prose (new subsection). `sig.changes` =
synthesized per-USE-SITE stream member (synthesized-member precedent 030-52..60;
per-site identity 029-57). `to_ring_stream`/`to_gate_stream` DELETED (030-123/128/129/
130 + every example naming them, incl. 030-29/127/135/246, SPEC worked examples).
Policy/capacity of `.changes` resolve like PLACEHOLDERS (004-4 use-site list: consuming
stream declaration pins it — materializes AS the declared stream, one buffer;
parameter/return type pins; else ring[1024] default; unresolved display = erased
`stream T` per 030-35). Two textual `.changes` sites = two streams. `to_signal`
RECLASSIFIED ordinary user-written operator (observe + default arm = fallback; R5
grammar enables it). Policy conversions user-writable (030-181, 029-29). 030-121
reworded per NO-PRIVILEGE (map/filter/merge user-writable).

**C13 OBSERVE AS-BINDING.** Arms bind event values via `as`. RULING R5 grammar:
`on <trigger> [where C] [as <binder>]:` — `as` binds the POST-FILTER event; inside C the
bare stream name keeps its 030-158 event meaning; in the arm BODY only the binder
denotes the event (bare stream names in bodies NEVER denote events). Single stream:
`as e` binds bare E. Multi-trigger: `as (e1, e2)` or `as events` (binder = identifier |
tuple pattern; tuple access .0/.1). Multi-set typing: stream slots `Option[E]` (Some iff
that trigger fired this activation; both fired = both Some); signal slots bind plainly.
Value-cell context: once per commit, binding = LATEST event of the commit. Stream
context: per-event evaluation. combineLatest REJECTED for observe arms (lives in stream
expressions 030-70..72). Two-clock semantics 016-255 preserved (`as` rides the
arm-selection clock). Observe still lowers to a derived read (033-193; binder adds an
event input, no new IR primitive).

**C14 KIND SWEEP.** Storability razor — storable values keep brackets (`Type[C]`
compile-time-only type; `Handle[T]`; `WeakHandle[T]`; `Portal[T]`); binding machinery
becomes lowercase keyword KINDS in annotation positions: `signal i32`; `derived f32`;
`recurrent[2] i32` (N omitted = [1]); `stream ring[1024] Event`; `stream gate[256] W`;
`recurrent[3] stream ring[512] f32`; ERASED `stream T` (replaces policy-erased
`Stream[T]` 030-35); `yielded f32[128]`; `cell T` (umbrella); `dynamic view Voice`
(replaces `Cell[DynamicView[WeakHandle[Voice]]]`). Cell = KIND, resolving the SEXTUPLED
type-vs-trait contradiction: 016-61/162/165 (type side) vs 030-47/49/51 (trait side)
PLUS a membership disagreement (016-162 excludes Stream[T] from the umbrella; 016-61/165
include it) — ONE coordinated edit across all six entries + both SPEC sides (13.2.8 vs
13.18.5 incl. the code block and the Why-a-trait rationale ~20072-75). `attr` annotates
as `signal T` (016-163). Static views get NO kind form (values: `Handle[T]`
arrays/slices). Bounds never inline in kind annotations (016-178 DELETED; bounds in
generic param list or `where`). `const` needs no row; effects/nodes/connections annotate
by type name. TWO-AXIS streams: 030-255/259 sealed four-member StreamPolicy remodeled to
policy {Ring[N], Gate[N]} × history-depth parameter (default 0); RecurrentRing/
RecurrentGate fulfill/alias lines deleted; alias spellings survive as sugar (030-262).
Kind taxonomy consolidated in ONE place (values / graph entities / cells; storable
designators direct / Handle / Portal; 013-52/53, 016-176 etc. become corollaries).
Cell-as-value REFUSED; `Portal[Cell[T]]` is the sanctioned identity-as-data (017-207).

**C15 MAP/COUNT.** Map insertion-ordered at language level, JS semantics (update keeps
position; delete+reinsert appends). 012-106/111, 018-39, 018-83, 018-139 amended — Map
exits the unordered category; HashSet STAYS unordered (SPLIT each co-occurrence entry,
never wholesale rewrite). Ord-not-derived conclusion (012-114) survives at LOG level;
its unordered rationale is SPEC-only stale (9.5 ~7382; also 4.9.5 ~4131-34 via 007-238)
— reword to keys-are-not-positions. Underspecified ripples to pin: Map + merge
(012-105/120) and `m[k]=v` insert-vs-update (012-108) positional behavior under
insertion order. COUNT unification: bare `.count` = element tally on
arrays/slices/bundles/Map/Vec/HashSet/Yielded/dynamic views (012-91, 017-90/92 renamed;
examples 012-87/88, 017-67/193 respelled; 017-188 no-len clause contradicted — dynamic
views get `.count`). Strings exempt (byte_len/char_count 012-29/30); stream metrics
exempt (pending_count etc. 030-53..58, 028-50, 033-109). Naming rule recorded: bare
`count` = element tally; prefixed `x_count` = specialized tally. DynamicView vs
Cell[Map] stay distinct types.

**C16 WIRE-FOLLOWING.** Connections may surface endpoint data via their OWN deriveds
(`derived target: WeakHandle[Clip] = handle to`; opt-in; 019-58 gains the carve-out;
019-57 preserved). Instance-granular interpretation: interpretation closure =
containment UNION wire-candidate envelopes (static sets; extends 015-16, 024-17); ONE
render per (root, instance) mounted at startup at stable paths; wires ACTIVATE, never
instantiate. Wire lowering: (a) movable value read through handle resolution (023-47
unchanged); (b) compiler-SYNTHESIZED wake gate per target render — predicate: some live
wire currently resolves to me OR containment parent active — lowering to existing gate
objects (033-65); wake-gate reads of incoming views are compiler-internal (017-217/218
user restrictions unaffected). Reactive-scrutinee match in interpretation context lowers
to `given` semantics. Wire inside a gated-off arm activates nothing (022-45/57). CYCLES:
wake gates inherit 022-37 (prior-commit reads, one-commit-delay fixpoint); consistent
with 024-13; guarded by Circularity (019-75, 024-17; 024-26 reinforced). Per-wire
multiplicity = explicit `repeat` over the incoming connection-view.
Engagement/activation orthogonality preserved (017-254); presence-is-participation
preserved (017-243 CLOSED as ADJ — R9). Connection-engagement MEANING stays extrinsic
(017-247/248 SPLIT: node-interpretation half CON, connection-meaning half preserved).

**C17 CONNECTIONS-AS-DATA.** Interpreters read the declared incoming connection-views of
the target through the node reference (NAMED views — `value.mods` where `mods` is the
view name; multi-view same-type ambiguity is MOOT). Acceptance intrinsic and
meaning-free (trait selectors for openness); meaning extrinsic (in interpreters). No
parent-threading (rejected). Dynamic connections: target reads its own dynamic incoming
view (017-114). Connection ownership asymmetry (019-18) preserved; borrow-not-in-cell
(019-19/20) preserved.

**C18 MISC.** Keyed (018-69 CON, 018-68/72/73 reworded) and StringifiableKey (018-64 RES
— definitional list; stdlib framing lives in 018-68/69; 018-28/63 touched) reclassified
LANGUAGE-defined. 023-46 AND 025-6 (the missed parallel) rewritten: WeakHandle
resolution AND Portal resolution are the two dynamic-dependency sources (017-204 is the
ADJ anchor; 017-204/207 are NOT internally contradictory — resolution-read vs
inert-window split already encoded). `operator` terminology note (029 construct vs
007-187 operator traits vs 001-34 application). 004-82 clarified: declared fn references
const-eligible; only anonymous/capturing closures excluded. Slice materialization: slice
of Copy elements to owned `T[N]` param materializes element-wise copy (extends 018-52;
compile-time length required; enables `chord_voice[const N](notes: Handle[Note][N])`
from a `[B]` binding). Interpreter-placed effect paths rooted at interpretation site
mirroring node paths (028-4/5 EXT; 033-128..133 EXT; `:N` ordinals apply).

## A.3 Post-verification rulings R1–R11 (all locked)

- **R1**: stream operator `fold` (030-140 — NOT 030-139) RENAMED `accumulate` (running
  seed-based accumulation over events; pairs with `scan` 030-151).
- **R2**: stream operator `count` (030-139) RENAMED `event_count` (applies the C15
  naming rule to itself).
- **R3**: the 029-43 example operator `changes` REPLACED with a differently-named
  illustration (function subsumed by synthesized `.changes`; entry also lists dead
  `to_ring_stream`/`to_gate_stream` — full rewrite).
- **R4**: yield-under-value-conditional — see C8.
- **R5**: observe grammar — see C13.
- **R6**: fold IR — see C10.
- **R7**: no `[B ...]` partial-row.
- **R8**: `parameter_bindings` provenance marker, not third case.
- **R9**: 017-243 preserved, closed.
- **R10**: Gated payload = all arms; 017-224 prose updated.
- **R11**: `else` is two-sense only (loop-else 014-88 + fold-else; 022-101 uses
  `otherwise:`/`default:` — the three-way claim was refuted); acceptable under 002-27;
  note at 002-6.

## A.4 Verification corrections that BIND this plan (W2 digest, complete)

- **W2(a)** 017-307 is ADJ; strip every Yielded-twin/carve-out rationale from the
  dynamic-view consumer family (017-74/125/190/306/282) — only C14 respell + C15 count
  apply there.
- **W2(b)** 002-27/28 are context/ADJ (rule text unchanged; keyword set grows: `collect`,
  `yield`, `fold`, `by`, `cell`, `yielded`; 002-3/4/6/9 EXT).
- **W2(c)** 005-67 rewrite separates auto-satisfaction from the waiver; 005-105 is an
  ADD (RES). [Plan note: 005-105 already exists in the LOG (L446, auto-satisfaction
  text); this plan reads W2(c) as: the auto-satisfaction rule RESIDES at 005-105 as a
  RES-grade touch, while the C2 waiver gets its own NEW entries — see B §005.]
- **W2(d)** 008-56, 016-9 are ADJ-with-new-sibling; 016-283 preserved + clarifying note
  only.
- **W2(e)** 009-89 reason corrected (entry text = match selects one arm); 009-90
  re-attached (given-vs-match division); record-match ban gets a NEW anchor entry.
- **W2(f)** 006-27 extension written as entry-match-only carve-out.
- **W2(g)** 012-114 ADJ at LOG level (SPEC-only rationale fix); 013-199 ADJ (already
  keyword-spelled); 014-157 without the auto-deref claim; 015-4 ADJ (graph-building
  enumeration; canonical kind list is 016-1 — which is CON: six→seven kinds +
  declaration/annotation split rider).
- **W2(h)** 015-15 RES-grade reword (verb collision only); 015-36 CON-leaning (interior
  effects have no host interpreter).
- **W2(i)** 018-64 RES not CON.
- **W2(j)** 027-45 light RES (observed signals stay host-written).
- **W2(k)** 032-87 CON (positive false enumeration); 032-107 ADJ (cited as fold
  rationale).
- **W2(l)** 029-74 stays an error; the `|>` fix homes at 029-65/106.
- **W2(m)** Missed entries to include (W2 section 3): 004-3, 008-70, 005-58, 005-9,
  007-238, 009-84, 012-105/108/120, 014-65, 015-16, 016-11/139/179/267,
  017-151/236/282, 018-57/106/107/109, 019-18/19/20, 021-142, 022-3, 023-56, 024-22,
  025-6, 026-2/9, 028-33/68/73, 029-18/41/44/45/103, 030-90/91/246/247/248, 031-1/70,
  033-64/121, 020-8/23/37.
- **W2(n)** Blind-spot dispositions: 017-243 ADJ closed; 017-254 EXT orthogonality
  preserved; 019-18 ADJ; 024-26 EXT reinforced; 030-246 RES hard defect (dead names);
  016-255 EXT two-clock preserved; 033-193 EXT lowering preserved; 011-40 + 005-22 ADJ
  cross-refs. Sections 003 and 010 cleared (one authoring note for 003: `observed:` as
  export boundary — lands at new 031-156, NOT in 003); 020 and 026 have real EXT
  additions (020-8/23/37 reserved words + body-scope members; 026-2/9 trap sites gain
  fold `by:`/`else:` combiners and collect/yield behaviors).

## A.5 Editor constraints (hard), numbered

- **EC1** — DO NOT rewrite 017-244 (effects never exposition entries — verified
  airtight; the token sweep alarm was a false positive). Byte-identical.
- **EC2** — KEEP the six-primitives count (033-56) — fold is a cell KIND, not a seventh
  primitive. Byte-identical (LOG L4134; SPEC L23041).
- **EC3** — The Cell-umbrella fix is ONE coordinated edit: six entries (016-61, 016-162,
  016-165, 030-47, 030-49, 030-51) + both SPEC sides (§13.2.8, §13.18.5 incl. code block
  and Why-a-trait rationale ~L20072-75). This is atomic group ⚛B.
- **EC4** — The SPEC §13.1 four dead host-claims (L11355-56, L11370-71, L11377-81 — see
  D.1 for the four claims) rewrite together. Atomic group ⚛C.
- **EC5** — The reconciler-conditional lands atomically at all 6 core LOG sites + 3 SPEC
  sites (atomic group ⚛A, B.2).
- **EC6** — Map/HashSet co-occurrence entries are SPLIT, never wholesale rewritten
  (HashSet stays unordered).
- **EC7** — No fn-exemption on `|>` (029-74 stays an error).
- **EC8** — 031-134 lift is PARTIAL (see C1); recurrent-in-observed lift is SPLIT
  value-vs-stream (see C1).

## A.6 Rejected during design (recorded for onboarding — do NOT resurrect)

`interpret` keyword and interpret-X-with-T bootstrap; `each` loop; Monoid (and any
identity requirement on fold); fold as operator / fold-with-tuple / fold-by-else linear
/ `fold[op,id]` declaration forms; changes-prefix keyword form; `T[]` slice synonym;
trait-headed match over records; Cell-as-value; auto-emitting signals in stream
expressions (combineLatest in observe arms); for/repeat unification and the `each` loop;
membership markers on yield; parent-threaded connection context; `seed` naming for the
fold else-value; VoiceSet-style domain records as host wire-format; node-internal
effects as interpretation substitute.

## A.7 Deferred (out of scope for this amendment)

Context RFC (inherited attributes ride explicit params); primitive-floor enumeration
(stdlib design); performance validation pass.

---

# PART B — LOG EDIT INVENTORY (sections 001–033, complete)

Table columns: **Entry** (pre-edit id) | **Act** (0.2 vocabulary) | **Anchor** (opening
text as verified by grep against DECISION_LOG.md during plan authoring; truncated with
`…`) | **Change** (new-text gist) | **Refs** (decision/ruling/W2). Sections with no
edits carry an explicit clearance note. ADD rows state the insertion anchor; per INV1
all later entries in the section renumber — renumber deltas are summarized in B.3.

## B.1 PROTECT REGISTER (single source; referenced by E.5, F.17, F.20)

HARD = byte-identical; SOFT = no meaning change (respell of surroundings allowed).

| Entry / site | Grade | LOG/SPEC line (pre-edit) | Verified text (truncated) |
|---|---|---|---|
| 017-244 | HARD (EC1) | LOG L2376 | "Effects are never exposition entries: effects are declared only in the `effects:` clause, never in `expose:`." |
| 033-56 | HARD (EC2) | LOG L4134 | "The graph is built from exactly six primitives: `cell`, `connection`, `gate`, `stream`, `effect`, and `scope`." |
| SPEC six-primitives | HARD (EC2) | SPEC L23041 | "The graph is built from **six primitives** — `cell`, `connection`, `gate`, …" |
| 029-74 | SOFT (EC7) | LOG L3421 | "Using `|>` with a `fn` RHS is a compile error: `0.0 \|> some_fn` ✗." — note: contains "RHS" |
| SPEC fn-error | SOFT (EC7) | SPEC L19526 | "Using `|>` with a `fn` is a compile error:" — note: NO "RHS"; the two texts legitimately differ |
| 016-283 | SOFT (C1) | LOG L2129 | "The host-program write surface comprises exactly two channels: `runtime.write_signal` … and per-effect-instance …" — clarifying note only |
| 030-67 | SOFT (C12) | LOG L3542 | "A signal in a stream expression is sampled at each driving-stream event…" — sampling rule stays; rationale goes to SPEC §13.18.7 |
| 022-5 | SOFT (C8) | LOG L2858 | "`if`/`else`/`match` remain value selection everywhere they appear … never gate structure" |
| 019-57 | SOFT (C16) | LOG L2643 | "`from` and `to` are body-internal, visible only inside the connection type's own body." |
| 017-162 | SOFT (C3) | LOG L2294 | "A node reference cannot be stored in a cell, record field, enum payload, or tuple." |
| 006-32 | SOFT (C5) | LOG L610 | "The `...` rest token — three dots, distinct from the `..` range operator …" |
| 030-117/118 | SOFT (C1/EC8) | LOG L3592-93 | host-fed `recurrent[N] stream` in `observed:` stays rejected |
| 023-47 | SOFT (C16) | LOG L3023 | "A read through a `WeakHandle[T]` reaches whichever entity the handle currently resolves to…" |
| 030-158 | SOFT (C13) | LOG L3633 | "Inside `C`, the LHS stream's name denotes the current event…" |
| 014-88 | SOFT (R11) | LOG L1720 | loop-`else` semantics (one of the two `else` senses) |

## B.2 Atomic group ⚛A — the reconciler-registration conditional (canonical list)

New rule text (restated verbatim at every site, per INV2): *"Reconciler registration is
required if and only if the effect's `observed:` block declares host-written channels
(`signal`/`stream`); an interior effect whose contract is fulfilled entirely by child
effects requires no reconciler."*

- **Core LOG sites (exactly 6)**: 015-39, 027-80, 027-81, 031-128, 031-143, 033-122.
  (The record's label "five LOG sites" is superseded by its own authoritative
  enumeration, which lists these six ids. Part A carries this note; E and F reuse this
  list verbatim.)
- **Support sites (NOT counted in the 6; edited in the same commit)**: 031-119
  (registration keyed by type name — gains the conditional clause), new 031-157
  (mixing placements and host channels allowed; see B §031).
- **SPEC sites (exactly 3)**: §13.14.7 L18713-16; §13.19.14 + §15.4.1 L22027-30;
  diagnostic L22116-23. (All three verified: "Unregistered effect types." ×2 and the
  `error: effect type `fetch` has no registered reconciler` example.)
- **Coordinated companions (same commit, not conditional-restating)**: LOG 015-36
  rewrite (CON-leaning, W2(h)); SPEC §13.1 L11377-81 ("Adding a new effect type … the
  host registers a reconciler") — also a member of ⚛C.

## B.3 Per-section renumber roll-up (exact, auditable)

| Sec | Now | Δ | Final | Sec | Now | Δ | Final |
|---|---|---|---|---|---|---|---|
| 001 | 38 | 0 | 38 | 018 | 140 | +3 | 143 |
| 002 | 29 | 0 | 29 | 019 | 78 | +1 | 79 |
| 003 | 77 | 0 | 77 | 020 | 38 | 0 | 38 |
| 004 | 155 | 0 | 155 | 021 | 142 | 0 | 142 |
| 005 | 234 | +4 | 238 | 022 | 120 | +1 | 121 |
| 006 | 33 | 0 | 33 | 023 | 56 | 0 | 56 |
| 007 | 239 | 0 | 239 | 024 | 27 | 0 | 27 |
| 008 | 73 | +1 | 74 | 025 | 65 | 0 | 65 |
| 009 | 122 | +1 | 123 | 026 | 10 | 0 | 10 |
| 010 | 45 | 0 | 45 | 027 | 120 | 0 | 120 |
| 011 | 81 | 0 | 81 | 028 | 75 | 0 | 75 |
| 012 | 187 | +1 | 188 | 029 | 125 | −1 | 124 |
| 013 | 250 | 0 | 250 | 030 | 262 | −2 | 260 |
| 014 | 167 | 0 | 167 | 031 | 153 | +5 | 158 |
| 015 | 41 | 0 | 41 | 032 | 179 | 0 | 179 |
| 016 | 283 | +2 | 285 | 033 | 234 | +2 | 236 |
| 017 | 308 | **+18** | **326** | 034 | — | +13 | 13 (new) |
| — | | | | 035 | — | +10 | 10 (new) |

Old grand total 4186 entries; new grand total 4186 + 32 (existing-section net) + 23
(new sections) = **4241**. "Now" counts verified by `grep -c "^SSS-"` per section.

## B.4 Section tables

### Section 001 (38 → 38)

| Entry | Act | Anchor (L64) | Change | Refs |
|---|---|---|---|---|
| 001-34 | ADJ | "Operator application uses the `|>` token." | No text change; terminology triangle (001-34 application / 007-187 operator traits / 029 construct) is documented in the SPEC operator-terminology note (D.6), not here | C18 |

### Section 002 (29 → 29)

| Entry | Act | Anchor | Change | Refs |
|---|---|---|---|---|
| 002-3 (L74) | EXT | "The declaration keywords are `node`, `connection`, …" | List gains `collect` (statement form) and notes `fold` as expression-form keyword | W2(b) |
| 002-4 (L75) | EXT | "The clause keywords are `children`, `incoming`, …" | List gains `by`, `else` (fold arms) usage note; no removals | W2(b) |
| 002-6 (L77) | EXT | "The control-flow keywords are `if`, `else`, `match`, …" | Gains `yield`; carries the R11 note: `else` has exactly two senses (loop-else 014-88, fold-else); `when:`/`given` fallbacks are `otherwise:`/`default:` (022-101), so no third sense | W2(b), R11 |
| 002-9 (L80) | EXT | "`as` is the naming/alias keyword…" | List gains `collect as x` and observe `on … as <binder>` | W2(b), C13, C8 |
| 002-27 (L98) | ADJ | "Every keyword is reserved in every position…" | Rule text unchanged; the keyword SET grows: `collect`, `yield`, `fold`, `by`, `cell`, `yielded` | W2(b), R11 |
| 002-28 (L99) | ADJ | "A keyword in a field-like position (`from`, `to`, …)" | Unchanged; context row (new keywords obey it) | W2(b) |

### Section 003 (77 → 77) — CLEARED

No edits (W2(n)). The one authoring note arising from 003 review — `observed:` as the
effect's export boundary — lands at NEW entry 031-156, not in 003.

### Section 004 (155 → 155)

| Entry | Act | Anchor | Change | Refs |
|---|---|---|---|---|
| 004-3 (L186) | ADJ | "A placeholder is resolved to a concrete type at each use site…" | Unchanged; cited context for `.changes` placeholder-style resolution | W2(m), C12 |
| 004-4 (L187) | EXT | "Use sites include explicit type annotations, arguments…" | Gains clause: a consuming `stream` declaration is a use site that pins a `.changes` policy/capacity (materializes AS the declared stream, one buffer); parameter/return type pins; else ring[1024] default | C12 |
| 004-45 (L228) | EXT | "Polymorphic recursion — a recursive call requiring a different type instantiation — is rejected…" | Gains carve-out: compile-time interpretation expansion is exempt, bounded by the finite interpretation closure (termination leans on the self-recursion termination rules restated from 017-26/27) | C3 |
| 004-47 (L230) | CON | "The polymorphic-recursion ban is permanent…" | Rewritten: the ban is permanent for runtime dispatch; compile-time interpretation expansion is the sole carved-out static case | C3 |
| 004-82 (L265) | RW | "Heap-allocated stdlib collections (`Vec`), … types containing function ref…" | Clarified: declared `fn` references are const-eligible; only anonymous/capturing closures are excluded | C18 |

### Section 005 (234 → 238; +4 ADDs after 005-67)

| Entry | Act | Anchor | Change | Refs |
|---|---|---|---|---|
| 005-9 (L350) | ADJ | "Trait method signatures name their receiver parameter explicitly…" | Unchanged; effect-kind methods obey it (`value: Subject` first param) | W2(m), C2 |
| 005-22 (L363) | ADJ | "The associated-type borrow/own convention applies uniformly … no language-privileged path…" | Unchanged; cross-ref context for language-DEFINED fulfillments (Yielded/Keyed/StringifiableKey are language-defined, not language-privileged) | W2(n), C9, C18 |
| 005-51 (L392) | CON | "A trait that declares only methods and associated types is kind-agnostic…" | Rewritten: a trait declaring an effect-kind method is effect-specific; kind-agnosticism holds only for traits with no effect-kind methods | C2 |
| 005-58 (L399) | ADJ | "A trait carrying `from`/`to` endpoints … is a connection contract…" | Unchanged; context: trait selectors for connection openness (C17 acceptance) | W2(m), C17 |
| 005-67 (L408) | RW | "The satisfies/fulfill pairing requirement is waived for traits with no methods and traits whose methods all have default bodies; these are…" | Rewritten to state ONLY the fulfill-without-satisfies waiver, now with the C2 conditions (every method incl. effect-kind has a default body AND no required cells); the auto-satisfaction half moves OUT (it already lives at 005-105) | C2, W2(c) |
| ADD 005-67+1 | ADD | insert after 005-67 | New: traits may declare effect-kind methods `effect name(params):` whose first parameter is Subject-typed; an effect-kind method declares an interpretation obligation | C2 |
| ADD 005-67+2 | ADD | insert after previous | New: an effect-kind trait method may declare an `observed:` contract block listing required output cells; the contract is a MINIMUM — a fulfill may expose more | C2 |
| ADD 005-67+3 | ADD | insert after previous | New: consumers projecting through the trait see only the contract cells | C2 |
| ADD 005-67+4 | ADD | insert after previous | New: `satisfies` remains mandatory for abstract-method traits and required-cell traits (the waiver never applies to them) | C2 |
| 005-69 (L410) | EXT | "A type's `satisfies` set must not contain two distinct parent-trait identities whose method names overlap…" | Effect-method names join the method-name collision namespace | C2 |
| 005-74 (L415) | EXT | "Effective method-set computation begins from an empty set…" | Effect-kind methods enter the effective set like value methods | C2 |
| 005-76 (L417) | EXT | "Two effective-set entries sharing a method name but originating from different parent-trait identities…" | Applies unchanged to effect-kind methods (clause added) | C2 |
| 005-80 (L421) | EXT | "The effective-method-set collision check ranges over the type's effective fulfilled-trait set…" | Applies unchanged to effect-kind methods (clause added) | C2 |
| 005-105 (L446) | RES | "A trait is automatically satisfied when all of its `requires` are satisfied, every method it declares has a default body, and every assoc…" | Light reword: auto-satisfaction stated standalone (no longer entangled with the 005-67 waiver); effect-kind default bodies count | W2(c), C2 |

### Section 006 (33 → 33)

| Entry | Act | Anchor | Change | Refs |
|---|---|---|---|---|
| 006-27 (L605) | EXT | "Variant patterns may be positional or named, parallel to variant construction." | Gains entry-match-only carve-out sentence: in `.exposition` entry matches (and ONLY there) a variant payload may carry a typed bound `Variant(name: Bound)`; this is not a general pattern shape | C4, W2(f) |
| 006-32 (L610) | PR | "The `...` rest token — three dots, distinct from the `..` range operator…" | Protected: closed enumeration intact; no `[B ...]` partial-row (R7) | C5, R7 |

### Section 007 (239 → 239)

| Entry | Act | Anchor | Change | Refs |
|---|---|---|---|---|
| 007-187 (L801) | ADJ | "Each operator has its own fine-grained trait whose method name matches the conventional operator name, e.g. `Add::add`." | Unchanged; one corner of the operator-terminology triangle (D.6 note) | C18 |
| 007-238 (L852) | ADJ | "`Map[K, V]` does not fulfill `Index[Range[K]]`. …" | LOG text survives; its SPEC-side rationale (§4.9.5 L4131-34 "unordered keyed collection") is stale after C15 and is reworded SPEC-side to keys-are-not-positions (D.4) | C15, W2(m) |

### Section 008 (73 → 74; +1 ADD after 008-56)

| Entry | Act | Anchor | Change | Refs |
|---|---|---|---|---|
| 008-56 (L912) | ADJ | "`Type[C]` is the type of a type value — a value whose value is a type satisfying the constraint `C`." | Unchanged; gains new sibling | W2(d), C14 |
| ADD 008-56+1 | ADD | insert after 008-56 | New sibling: `Type[C]` keeps its bracket spelling under the storability razor — it is a storable compile-time-only value, not binding machinery; kind keywords never replace it | C14, W2(d) |
| 008-70 (L926) | ADJ | "A type value may be stored in persistent slots: record fields, attrs, and cells." | Unchanged; context for the razor (storable ⇒ brackets) | W2(m), C14 |

### Section 009 (122 → 123; +1 ADD after 009-90)

| Entry | Act | Anchor | Change | Refs |
|---|---|---|---|---|
| 009-84 (L1016) | ADJ | "The wildcard pattern `_` matches without binding." | Unchanged; the wildcard substrate that entry-match exhaustiveness leans on | C4 |
| 009-89 (L1021) | RW | "`match` is a value selector: it evaluates the scrutinee, selects one arm, evaluates that arm to a value, and discards the rest." | Reason corrected per W2(e): entry text already says match selects ONE arm; rewrite only adds that in interpretation context a reactive-scrutinee match lowers to `given` semantics (C16) — selection claim intact | W2(e), C16 |
| 009-90 (L1022) | RW | "Selecting which node/connection subtree is exposed and kept live is the role of `given` … not `match`." | Re-attached to the given-vs-match division; text updated to note the interpretation-context lowering (match over entries compiles to static unroll / mount-time tag; live-subtree selection remains `given`) | W2(e), C4, C16 |
| ADD 009-90+1 | ADD | insert after 009-90 | NEW ANCHOR entry: trait-headed match over RECORDS is banned; a bound in match-payload position is legal only in `.exposition` entry matches (restated locally per INV2) | C4, W2(e) |

### Section 010 (45 → 45) — CLEARED

No edits (W2(n)).

### Section 011 (81 → 81)

| Entry | Act | Anchor | Change | Refs |
|---|---|---|---|---|
| 011-40 (L1145) | ADJ | "The postfix `?` operator dispatches through the stdlib trait `Try`." | Unchanged; cross-ref context for language-defined fulfillment authoring style (Yielded:Iterable is authored like this family) | W2(n), C9 |

### Section 012 (187 → 188; +1 ADD after 012-91)

| Entry | Act | Anchor | Change | Refs |
|---|---|---|---|---|
| 012-29 (L1218) | PR | "`s.byte_len() -> isize` returns the length in bytes in O(1)." | Strings exempt from `.count` unification | C15 |
| 012-30 (L1219) | PR | "`s.char_count() -> isize` returns the number of Unicode scalars in O(n)." | Strings exempt | C15 |
| 012-87 (L1276) | RES | "Open-ended slice ranges are sugar at slice-index positions only: `arr[..n]` → `0..n`, `arr[k..]` → `k..arr.length`…" | Respell `arr.length` → `arr.count` in examples | C15 |
| 012-88 (L1277) | RES | "The prefix `^k` inside a slice index means `arr.length - k`…" | Respell `.length` → `.count` | C15 |
| 012-91 (L1280) | RW | "`.length` is the full-word slice/array length accessor — compile-time-known for `T[N]`…" | RENAMED: `.count` is the element-tally accessor on arrays/slices/bundles/Map/Vec/HashSet/Yielded/dynamic views; compile-time-known where extent is static | C15 |
| ADD 012-91+1 | ADD | insert after 012-91 | New: naming rule — bare `count` = element tally; prefixed `x_count` (`byte_len`/`char_count` for strings, `pending_count` etc. for stream metrics) = specialized tally, exempt from unification | C15 |
| 012-105 (L1294) | RW | "Maps merge with `+`: `x + y` produces a new Map where right-hand keys win…" | Pins positional behavior under insertion order: result order = left order, then unseen right keys in right order; a right-side collision keeps the LEFT position (update-keeps-position) | C15, W2(m) |
| 012-106 (L1295) | CON | "Map iteration is **unordered** — keys and `(K, V)` pairs are yielded in whatever order the underlying hash table emits…" | Rewritten: Map iteration is insertion-ordered at language level (JS semantics: update keeps position; delete+reinsert appends) | C15 |
| 012-107 (L1296) | ADJ | "Map cost model is normative at the language level: lookup, insert, and delete are O(1) average…" | Unchanged; one of the three cost-rule family precedents the fold cost rule joins | C10 |
| 012-108 (L1297) | RW | "Maps are mutable only inside function bodies on `mut` bindings: `m[k] = v` inserts or updates…" | Pins: `m[k]=v` on absent key appends (insert); on present key updates in place keeping position | C15, W2(m) |
| 012-111 (L1300) | CON | "`Map[K, V]` implements `Iterable` and `IntoIterable`, yielding `(K, V)` pairs; iteration order is unordered…" | Rewritten: yields pairs in insertion order (same commitment as 012-106 rewrite, restated locally per INV2) | C15 |
| 012-114 (L1303) | ADJ | "The compiler provides `Eq`, `Hash`, `Clone`, `Display`, and `Debug` for `Map[K, V]` structurally…" | LOG text unchanged (Ord still not derived — conclusion survives); the stale "unordered" rationale is SPEC-only (§9.5 L7382) and reworded there to keys-are-not-positions (D.4) | C15, W2(g) |
| 012-120 (L1309) | RW | "`Map[K, V]` fulfills the language-defined fine-grained `Add` operator trait … `x + y` produces a new `Map`…" | Same positional pin as 012-105 (restated locally per INV2) | C15, W2(m) |

### Section 013 (250 → 250)

| Entry | Act | Anchor | Change | Refs |
|---|---|---|---|---|
| 013-52 (L1431) | ADJ | "A borrow-equivalent alias cannot be stored in a record field, tuple component, enum payload, or indexed slot." | Unchanged; becomes a stated corollary of the consolidated kind taxonomy (016 ADDs) | C14 |
| 013-53 (L1432) | ADJ | "A borrow-equivalent alias cannot be written into a reactive cell (`signal.write`, `stream.emit`, …)." | Unchanged; taxonomy corollary | C14 |
| 013-199 (L1578) | ADJ | "A closure cannot be the value type of a `signal`, `attr`, `recurrent`, or `derived` cell." | Unchanged; already keyword-spelled (W2(g)) | W2(g), C14 |

### Section 014 (167 → 167)

| Entry | Act | Anchor | Change | Refs |
|---|---|---|---|---|
| 014-65 (L1697) | EXT | "Loops over variable-extent types run at runtime: `Vec[T]`, `SmallVec[T, N]`, … `Map[K, V]`, and rang…" | Runtime-loop list gains `yielded T` (Yielded is runtime-loop iterable only) | C9, W2(m) |
| 014-71 (L1703) | EXT | "A `for` in a node type's `expose:` block or in a placement body is auto-enforced compile-time…" | Gains: structural (compile-time) `for` over a `yielded` group is a compile error | C9 |
| 014-88 (L1720) | PR | "The `else:` expression is evaluated exactly when the loop completes naturally…" | Protected (SOFT); loop-else is one of the exactly-two `else` senses (R11) | R11 |
| 014-148 (L1780) | ADJ | "A conforming implementation must compile monomorphized loops over built-in iterables to machine code equivalent to hand-written…" | Unchanged; cost-rule family precedent for the fold cost rule | C10 |
| 014-157 (L1789) | RW | "Associated-type constraints use `.` member-access notation: `fn total[T: Iterable](source: T) -> T.Iter.Item`." | Rewritten WITHOUT the auto-deref claim (W2(g)); constraint notation itself unchanged | W2(g) |

### Section 015 (41 → 41)

| Entry | Act | Anchor | Change | Refs |
|---|---|---|---|---|
| 015-4 (L1806) | ADJ | "A reactive graph is built declaratively from `signal`, `attr`, `recurrent`, `derived`, `node`, and `connection` declarations." | Unchanged; graph-building enumeration, NOT the canonical kind list (that is 016-1) | W2(g) |
| 015-15 (L1817) | RES | "A function or cell can only yield a node that was already placed." | RES-grade reword: replace the verb "yield" (now a keyword) with "return"/"surface"; rule unchanged | W2(h) |
| 015-16 (L1818) | EXT | "Endpoint typing and topology-cycle analysis are performed on the candidate topology — every node type a `to` could resolve to…" | Gains: the candidate topology's wire-candidate envelopes also join the interpretation closure (containment UNION envelopes) | C16, W2(m) |
| 015-36 (L1838) | CON | "`effect` is the host-interpreted bridge for outside-world alignment, using the `desired:`/`observed:` reconciliation model." | CON-leaning rewrite: an effect is a reconciliation contract — fulfilled by a host reconciler (leaf), by child effects (interior), or both; interior effects have no host interpreter | C1, W2(h); ⚛A companion |
| 015-39 (L1841) | RW | "A new effect type is added by declaring an `effect`; the host registers a reconciler for that effect type." | ⚛A core site: gains the registration conditional (B.2 text) | C1; ⚛A |

### Section 016 (283 → 285; DEL 016-178; ADDs: sibling after 016-9, two taxonomy entries after 016-176)

| Entry | Act | Anchor | Change | Refs |
|---|---|---|---|---|
| 016-1 (L1847) | CON | "There are exactly six reactive declaration kinds — `signal`, `attr`, `recurrent`, `derived`, `const`, `stream` — distinguished by who…" | Rewritten: SEVEN kinds (gains `yielded`… via `collect`) + declaration/annotation split rider (declaration keywords vs kind annotations are two rows of one taxonomy) | W2(g), C9, C14 |
| 016-9 (L1855) | ADJ | "A signal may be declared only at module level or in an effect's `observed:` block…" | Unchanged; gains new sibling | W2(d), C1 |
| ADD 016-9+1 | ADD | insert after 016-9 | New sibling: an effect's `observed:` block also accepts `derived` and value-`recurrent` declarations as computed outputs; host-fed `recurrent[N] stream` remains rejected there (restated locally) | C1, EC8 |
| 016-11 (L1857) | ADJ | "Signal semantics — host-written, not source-assignable, reactive — are identical at both declaration sites…" | Unchanged | W2(m) |
| 016-61 (L1907) | RW | "`Cell[T]` is the typing umbrella over the reactive cell types `Signal[T]`, `Derived[T]`, `Recurrent[T, N]`, and `Stream[T]`." | ⚛B: `cell` is a KIND (umbrella annotation `cell T`), not a type or trait; membership = all reactive cells incl. streams and yielded | C14; ⚛B |
| 016-139 (L1985) | RES | "An init-time read of a `Signal[T]` cell returns the cell's value at the topological-init evaluation point…" | Respell `Signal[T]` → `signal T` kind spelling; rule intact | C14, W2(m) |
| 016-162 (L2008) | RW | "`Cell[T]` is the umbrella type over all reactive cells; the value cells are the concrete types `Signal[T]`, `Derived[T]`, and `Recurrent…" | ⚛B: same coordinated rewrite; the membership disagreement (this entry excluded Stream[T]; 016-61/165 included it) is resolved: streams ARE under the `cell` kind umbrella | C14; ⚛B |
| 016-163 (L2009) | RW | "An `attr` is a placement-written `Signal[T]`…" | `attr` annotates as `signal T` (kind spelling) | C14 |
| 016-165 (L2011) | RW | "`Cell[T]` is the umbrella over all reactive cells: `Signal[T]`, `Derived[T]`, `Recurrent[T, N]`, and `Stream[T]`." | ⚛B: coordinated rewrite (kind umbrella, restated locally) | C14; ⚛B |
| 016-176 (L2022) | ADJ | "A `Cell[T]` received as a parameter is read-only; no source-level form writes through a cell reference." | Respell to `cell T` param spelling; becomes taxonomy corollary; gains the two ADDs below as neighbors | C14 |
| ADD 016-176+1 | ADD | insert after 016-176 | New: THE consolidated kind taxonomy (single home): values / graph entities / cells; storable designators direct / `Handle` / `Portal`; kind annotations `signal T`, `derived T`, `recurrent[N] T`, `stream ring[N] T`, `stream gate[N] T`, `recurrent[N] stream …`, erased `stream T`, `yielded T`, `cell T`, `dynamic view X`; static views get no kind form; `const` needs no row; effects/nodes/connections annotate by type name | C14 |
| ADD 016-176+2 | ADD | insert after previous | New: the storability razor — storable values keep bracket types (`Type[C]`, `Handle[T]`, `WeakHandle[T]`, `Portal[T]`); binding machinery is spelled as lowercase kinds; cell-as-value refused; `Portal[Cell[T]]` is the sanctioned identity-as-data | C14 |
| 016-178 (L2024) | DEL | "Standard trait bounds apply to the value type: `Cell[T: Numeric]` requires `T` to satisfy `Numeric`." | DELETED: bounds never inline in kind annotations; bounds live in the generic param list or `where` | C14 |
| 016-179 (L2025) | RES | "Field access on a value cell holding a record (e.g. `Signal[Record]`) inside a derived projects the field…" | Respell `Signal[Record]` → `signal Record`; rule intact | C14, W2(m) |
| 016-255 (L2101) | ADJ | "An observe's value can change in a commit where no `on` trigger fires; arm selection and intra-arm reactivity are independent…" | Unchanged (two-clock semantics preserved); `as` rides the arm-selection clock — clarifying clause only | C13, W2(n) |
| 016-267 (L2113) | RW | "`observe` may be the RHS of a `derived`, `signal`, `recurrent`, `recurrent[N] stream`, or `stream` declaration." | Kind respell + `as`-binder coherence: the RHS list keeps its members in kind spelling; notes binder typing per context (value-cell = latest event per commit; stream = per event) | C13, C14, W2(m) |
| 016-283 (L2129) | PR | "The host-program write surface comprises exactly two channels: `runtime.write_signal` … per-effect-instance…" | Protected + clarifying note ONLY: computed `observed:` outputs (derived/value-recurrent) are program-written, so the host-WRITE count stays two | C1, W2(d) |

### Section 017 (308 → 326; exactly 18 ADDs, enumerated below; no deferrals)

The cluster budget is 2 + 2 + 3 + 1 + 7 + 3 = **18** (the gate reviewer's "sums to 15"
was a miscount; the underlying ambiguity finding is resolved here by full enumeration).
Final count pinned at **326**. Insertion anchors use pre-edit ids; each insertion
renumbers the section tail per INV1.

**Cluster 1 — [B] bundle-row match (2 ADDs after 017-90):**

| # | Insert after | New entry text (gist) | Refs |
|---|---|---|---|
| A1 | 017-90 (L2222 "Bundle access goes through the `Index` trait…") | `[B]` in bundle-payload pattern position matches a whole row whose every element satisfies B; the binding is the row slice `Handle[B][..]` (017-97 type); completes the bracket trilogy write/accept/match | C5 |
| A2 | after A1 | NO `T[]` synonym for `T[..]`; NO `[B ...]` partial-row matching in v1; the `...` closed enumeration is intact (restated locally) | C5, R7 |

**Cluster 2 — connections-as-data reads (2 ADDs after 017-114):**

| # | Insert after | New entry text (gist) | Refs |
|---|---|---|---|
| A3 | 017-114 (L2246 "A connection whose membership is not a static fact requires a `dynamic incoming` connection-view…") | Interpreters read the target's DECLARED incoming connection-views through the node reference by view NAME (`value.mods`); multi-view same-type ambiguity is moot because access is by name | C17 |
| A4 | after A3 | For dynamic connections the target reads its own dynamic incoming view; acceptance is intrinsic and meaning-free (trait selectors for openness); meaning is extrinsic, in interpreters; no parent-threading | C17 |

**Cluster 3 — count / dynamic-view surface (3 ADDs after 017-193):**

| # | Insert after | New entry text (gist) | Refs |
|---|---|---|---|
| A5 | 017-193 (L2325 "A compile-time `for … as` loop with N = 0 iterations is legal…") | Dynamic views expose `.count` (element tally), contradicting the former no-length clause of 017-188; `.count` is the unified bare tally spelling (restated locally) | C15 |
| A6 | after A5 | The dynamic-view consumer surface is UNCHANGED: operators and `repeat` remain the only consumers; `yielded` groups are NOT dynamic-view consumers | W2(a), C9 |
| A7 | after A6 | `repeat (child: Bound)` — the parenthesized typed binder — is legal over dynamic views and connection-views only (entity references, closed envelopes); FILTER semantics; per-candidate monomorphization; no scope for non-matching members; no exhaustiveness implication (restated locally; declaration home is section 018) | C7 |

**Cluster 4 — Portal/WeakHandle split anchor (1 ADD after 017-204):**

| # | Insert after | New entry text (gist) | Refs |
|---|---|---|---|
| A8 | 017-204 (L2336 "A portal resolution read is reactive…") | WeakHandle resolution AND Portal resolution are the two dynamic-dependency sources; the resolution-read vs inert-window split (017-204/207) is one coherent design, not a contradiction | C18, W2(n) |

**Cluster 5 — entry sum (7 ADDs replacing/expanding at 017-210):**

017-210 itself is SUP (see table below); the seven new entries land immediately after it.

| # | Insert after | New entry text (gist) | Refs |
|---|---|---|---|
| A9 | 017-210 | `.exposition` is a typed, language-owned CLOSED sum of five entry kinds: Node, Connection, Bundle, DynamicView, Gated | C4 |
| A10 | after A9 | The walk is `for entry` over `.exposition` plus `match` over the five variants | C4 |
| A11 | after A10 | Entry-match-only bounded pattern `Variant(name: Bound)`; Bound = trait or concrete type; a carve-out scoped to entry matches, not a general pattern shape; trait-headed match over records stays banned (restated locally) | C4 |
| A12 | after A11 | Bounded arms are filters and never count toward exhaustiveness; each variant needs an unbounded arm (`Node(_)`) or the match a final `_`; `Node(n)` = `Node(n: Node)` intrinsic marker | C4 |
| A13 | after A12 | Arms apply in declaration order, first match wins; broad-before-narrow is a dead-arm lint | C4 |
| A14 | after A13 | Binding = generic-parameter semantics, monomorphized: static entries compile-time unroll with zero runtime tests; dynamic elements are tagged once at mount over the closed candidate envelope | C4 |
| A15 | after A14 | The compiler can emit a per-interpretation-root participation/skip report | C4 |

**Cluster 6 — interpretation closure / bootstrap (3 ADDs after 017-255):**

| # | Insert after | New entry text (gist) | Refs |
|---|---|---|---|
| A16 | 017-255 (L2387, rewritten — see table) | The interpretation closure = containment closure UNION wire-candidate envelopes (both static sets); ONE render per (root, instance), mounted at startup at stable paths | C16 |
| A17 | after A16 | Wires ACTIVATE, never instantiate: each target render gets a compiler-synthesized wake gate (predicate: some live wire currently resolves to me OR containment parent active), lowering to existing gate objects; wake-gate reads of incoming views are compiler-internal | C16 |
| A18 | after A17 | Interpretation bootstraps as an effect call in `effects:` (`render(song).audio \|> audio_out`); projection is explicit; no new top-level construct | C3 |

**017 rewrite/protect rows (non-ADD):**

| Entry | Act | Anchor | Change | Refs |
|---|---|---|---|---|
| 017-26/27 (L2158-59) | ADJ | "Self-recursion is legal…" / "Self-recursive placements terminate." | Unchanged; termination substrate for the 004-45 carve-out | C3 |
| 017-67 (L2199) | RES | "`for … as <name>` binds `<name>` to `[<name>::entry; N]`…" | Respell length spellings to `.count` in examples | C15 |
| 017-74 (L2206) | ADJ | "A dynamic view is consumed in exactly two ways: operators for values, `repeat` for structure." | Unchanged (W2(a): NO Yielded-twin rationale added) | W2(a) |
| 017-89 (L2221) | ADJ | "A `Bundle[T]` is a homogeneous co-placement … written `[...]` at placement…" | Unchanged; bracket-trilogy "write" corner | C5 |
| 017-90 (L2222) | RW | "Bundle access goes through the `Index` trait (§4.9.5): `bundle[g]` returns a row slice … `bundle[g]…" | Rename length spellings to `.count`; Index behavior intact; gains cluster-1 ADDs after it | C15, C5 |
| 017-92 (L2224) | RW | "With a single matching bundle its members are reached `bundle[i]` and `bundle.length`…" | `.length` → `.count` rename | C15 |
| 017-93 (L2225) | ADJ | "For a bundle view the inner cardinality … is part of the match predicate, a filter…" | Unchanged; bracket-trilogy "accept" corner | C5 |
| 017-97 (L2229) | ADJ | "`as`-naming a bundle: `[n1 n2] as pair` binds `pair` to a borrow of the row slice form — `Handle[T][..N]`…" | Unchanged; supplies the `[B]`-binding type | C5 |
| 017-103 (L2235) | ADJ | "Views count gated-but-frozen children and reads return their frozen values…" | Unchanged; one of the three existing gate read-paths, documented as one system in SPEC (D.5) | C6 |
| 017-114 (L2246) | ADJ | "A connection whose membership is not a static fact requires a `dynamic incoming` connection-view…" | Unchanged; gains cluster-2 ADDs after it | C17 |
| 017-125 (L2257) | ADJ | "A `dynamic` connection-view reads as a reactive cell … consumed only by operators and `repeat`…" | Unchanged (W2(a)) | W2(a) |
| 017-151 (L2283) | ADJ | "A caller-supplied (outgoing) connection engages only where the type names its connection-view in `expose:`…" | Unchanged; engagement substrate for C16 | W2(m), C16 |
| 017-162 (L2294) | PR | "A node reference cannot be stored in a cell, record field, enum payload, or tuple." | Protected: node refs bound as effect arguments join the closure as BORROWS, not cell stores | C3 |
| 017-188 (L2320) | CON | "`DynamicView[T]` is a language-level non-array iterator-shaped type with `Item = WeakHandle[T]`…" | The no-len clause is contradicted: dynamic views get `.count`; respell type to `dynamic view T` kind form | C15, C14 |
| 017-190 (L2322) | ADJ | "`DynamicView[T]` reactivity surface: the containing `Cell` is what declares the dependency edge…" | Respell only (kind form); NO Yielded rationale (W2(a)) | W2(a), C14 |
| 017-193 (L2325) | RES | "A compile-time `for … as` loop with N = 0 iterations is legal…" | Respell examples (`.count`); gains cluster-3 ADDs after it | C15 |
| 017-204 (L2336) | ADJ | "A portal resolution read is reactive: it joins the reader's provenance and re-fires when the slot drops or is rebound…" | Unchanged; the ADJ anchor for the 023-46/025-6 rewrites; gains cluster-4 ADD | C18, W2(n) |
| 017-207 (L2339) | ADJ | "`Cell[T]` and `Portal[T]` are orthogonal…" | Kind respell only (`cell T`); the inert-window half of the A8 split; `Portal[Cell[T]]` stays the sanctioned identity-as-data | C14, C18 |
| 017-210 (L2342) | SUP | "Exposition entries are node placements and connection placements; their order is semantic…" | SUPERSEDED by A9–A15; rewritten to surviving residue: entry order remains semantic (traversal and engagement order) — the kind enumeration moves to the new sum entries | C4 |
| 017-211 (L2343) | ADJ | "Node-body members — cells, `view`/`dynamic view` selection declarations, the acceptance clauses…" | Unchanged; the body-order precedent C1 mirrors | C1 |
| 017-217 (L2349) | PR | "A bare incoming connection-view name as an exposition entry is a compile error." | Protected: user restriction unaffected by compiler-internal wake-gate reads | C16 |
| 017-218 (L2350) | PR | "The bare incoming-connection-view exposition-entry error points at the expression-position idioms…" | Protected, same reason | C16 |
| 017-224 (L2356) | RW | "An exposition entry may be a `when` block or a `given` block; each arm body is a list of exposition entries, and the runtime exposes the…" | R10 prose update: each arm body is the same walkable type as `.exposition`; `Gated(arms)` binds ALL arms (compile-time fixed extent), not only the active one | C6, R10 |
| 017-236 (L2368) | ADJ | "The current exposition varies at runtime only through declared constructs: gate arms flipping, `repeat` scopes mounting…" | Unchanged; consistent with Gated walking all arms (walk is compile-time; runtime variation is activation) | W2(m), C6 |
| 017-243 (L2375) | ADJ | "Presence is participation: an exposition entry's connection is reactively live whether or not traversal has reached it." | CLOSED as ADJ per R9 — no text change | R9, W2(n) |
| 017-244 (L2376) | PR-HARD | "Effects are never exposition entries…" | EC1: byte-identical | EC1 |
| 017-246 (L2378) | ADJ | "A connection entry is engaged: the runtime takes up the connection and begins interpreting the connected subgraph." | Unchanged; anchor context for cluster 6 | C16 |
| 017-247 (L2379) | SPL | "What engagement means is domain semantics carried by the specific connection type and interpreted by the runtime/host…" | SPLIT: connection-meaning half preserved (meaning stays extrinsic); node-interpretation half CON (interpretation is now expressible in-language via effects) | C16 |
| 017-248 (L2380) | SPL | "One connection type may carry await-like meaning and another parallel meaning; the host interprets each connection type…" | Same split: per-type meaning survives; "the host interprets" generalizes to "the interpreter (host or in-language)" | C16 |
| 017-254 (L2386) | EXT | "Terminology is fixed: activation refers to gate-state, engagement to traversal reaching an entry…" | Orthogonality preserved; gains clause covering wake-gate activation (a render can be mounted yet inactive) | C16, W2(n) |
| 017-255 (L2387) | RW | "The `effects:` clause is the sole site where an effect may be instantiated; module level, operator bodies, connection bodies, and functi…" | AMENDED: effect bodies join `effects:` clauses as instantiation sites (child placements top-of-body); all other sites stay banned; gains cluster-6 ADDs after it | C1 |
| 017-282 (L2414) | ADJ | "A `dynamic` view is a reactive cell consumed via operators or `repeat`." | Unchanged (W2(a)) | W2(a), W2(m) |
| 017-299/300 (L2431-32) | ADJ | "When a function called from a reactive expression iterates a view…" / "When any one contributing child cell changes…" | Unchanged; the frozen-read baseline the NEW fourth gate read-path (022 ADD) is contrasted against | C6 |
| 017-306 (L2438) | ADJ | "A `dynamic` view is not `for`-iterable, not indexable, and not key-addressable from the receiving body." | Unchanged (W2(a)) | W2(a) |
| 017-307 (L2439) | ADJ | "Operators and `repeat` are the only consumers of a `dynamic` view." | Unchanged — Yielded does NOT join this surface (W2(a) correction) | W2(a) |

### Section 018 (140 → 143; +3 ADDs)

| Entry | Act | Anchor | Change | Refs |
|---|---|---|---|---|
| 018-28 (L2471) | RES | "Scope keys are required to be stringifiable primitives; for `repeat` the bound is the `StringifiableKey` trait." | Touched: `StringifiableKey` reframed as language-defined (wording only) | C18 |
| 018-39 (L2482) | SPL | "The standard library fulfills `Iterable` for `Vec[T]`, `T[N]` … and `HashSet[T]`. The language-level `Map[K, V]` (§9.5) ful…" | SPLIT per EC6: HashSet half unchanged (unordered); Map half rewritten to insertion-ordered iteration | C15, EC6 |
| 018-52 (L2495) | EXT | "Persisting a non-`Copy` bind into category B/D storage materializes a copy at that point…" | Extended: a slice of Copy elements passed to an owned `T[N]` param materializes an element-wise copy; compile-time length required; enables `chord_voice[const N](notes: Handle[Note][N])` from a `[B]` binding | C18 |
| 018-57 (L2500) | ADJ | "In an `effects:` clause, the `repeat` body is a list of effect entries instead of placements." | Unchanged; substrate for per-wire multiplicity (explicit repeat over the incoming connection-view) | W2(m), C16 |
| 018-63 (L2506) | RES | "An explicit `keyed by` result must be a `StringifiableKey`." | Wording touch (language-defined framing) | C18 |
| 018-64 (L2507) | RES | "`StringifiableKey` comprises `i8`–`i64`, `u8`–`u64`, `bool`, `char`, and `string`." | RES not CON (W2(i)): definitional list survives verbatim; only classification (language-defined) moves — and that framing lives in 018-68/69, not here | C18, W2(i) |
| 018-65 (L2508) | RW | "Key path 2 (carried key): when the source is a `dynamic` namespace cell, each element arrives with the key its supplier derived…" | Carried keys extended over dynamic views (repeat-expression sources) | C7 |
| 018-68 (L2511) | RW | "Key path 3 (`Keyed` trait): if the element type fulfills the stdlib `Keyed` trait…" | "stdlib" → language-defined framing | C18 |
| 018-69 (L2512) | CON | "`Keyed` is a stdlib trait of shape `type Key: StringifiableKey` plus `fn key(value: Subject) -> Key`." | Rewritten: `Keyed` is a LANGUAGE-defined trait (shape unchanged) | C18 |
| 018-72 (L2515) | RES | "When no key-derivation path applies, compilation fails with an error directing the user to fulfill `Keyed`…" | Reworded (language-defined framing) | C18 |
| 018-73 (L2516) | RES | "The `Keyed` path always wins over the stringifiable-element path when both apply…" | Reworded (framing only) | C18 |
| 018-78 (L2521) | RW | "Keys in `old − new` are dropped: `scope_drop(key)` releases the per-key cells." | Extended to repeat-expression / dynamic-view sources (carried keys) | C7 |
| 018-83 (L2526) | SPL | "Unordered iterables (`HashSet[T]`, `Map[K, V]`) are diffed by key identity; iteration order is whatever the underlying iterator emits…" | SPLIT per EC6: HashSet stays as-is; Map moves out (insertion-ordered; still diffed by key identity) | C15, EC6 |
| ADD 018-83+1 | ADD | insert after 018-83 | New: `repeat` is liftable to expression position — the expression yields exactly what `repeat … as X` binds; `as` is sugar | C7 |
| ADD 018-83+2 | ADD | insert after previous | New: the parenthesized typed binder `repeat (child: Bound)` in `dyn:`; mirrors match payload patterns; bound-colons always inside parens; legal only over dynamic views/connection-views | C7 |
| ADD 018-83+3 | ADD | insert after previous | New: the typed binder has FILTER semantics with per-candidate monomorphization; non-matching members get no scope; no exhaustiveness implication; value collections keep bare/tuple binders only (bounds over values = record type-case, banned) | C7 |
| 018-106 (L2549) | ADJ | "`repeat` is admitted in `effects:` clauses: each scope materializes one effect-bearing instance per element." | Unchanged; C16 multiplicity substrate | W2(m), C16 |
| 018-107 (L2550) | ADJ | "Per-element effects from an `effects:`-clause `repeat` suspend, resume, and tear down with the element key." | Unchanged; lifecycle precedent C1 child effects mirror | W2(m), C1 |
| 018-109 (L2552) | ADJ | "`repeat` is rejected in both of an effect's blocks (`observed:` and `desired:`)." | Unchanged — C1's child placements are top-of-BODY items, not block items; this ban stands | W2(m), C1 |
| 018-139 (L2582) | SPL | "`at <index>` on an unordered `repeat` source (`Map`, `HashSet`, …) emits a **normative diagnostic** of class `unstable_positional_iter…" | SPLIT per EC6: HashSet keeps the diagnostic; Map exits the unordered category (indexed access over insertion order is stable — diagnostic no longer applies to Map) | C15, EC6 |

### Section 019 (78 → 79; +1 ADD after 019-58)

| Entry | Act | Anchor | Change | Refs |
|---|---|---|---|---|
| 019-18 (L2604) | ADJ | "A connection's exposition entry belongs to the source's exposition, yet the connection is owned by no single endpoint." | Unchanged (ownership asymmetry preserved) | C17, W2(n) |
| 019-19 (L2605) | PR | "A node cannot store a direct reference to another node instance: `attr target: SomeNode` ✗." | Protected | C17 |
| 019-20 (L2606) | PR | "A node-instance reference is a borrow, and a borrow may not be stored in a cell." | Protected | C17 |
| 019-57 (L2643) | PR | "`from` and `to` are body-internal, visible only inside the connection type's own body." | Protected (SOFT) | C16 |
| 019-58 (L2644) | RW | "A connection does not surface its endpoints or its activation as readable fields." | Gains the opt-in carve-out: a connection MAY surface endpoint data via its OWN deriveds; activation stays unsurfaced | C16 |
| ADD 019-58+1 | ADD | insert after 019-58 | New: the endpoint-derived form — `derived target: WeakHandle[Clip] = handle to` — is the sole sanctioned endpoint surface; opt-in, per connection type | C16 |
| 019-75 (L2661) | ADJ | "Every topology cycle in the construction-time node graph must traverse at least one connection whose type satisfies `Circularity`." | Unchanged; guards wake-gate cycles | C16 |

### Section 020 (38 → 38)

| Entry | Act | Anchor | Change | Refs |
|---|---|---|---|---|
| 020-8 (L2675) | EXT | "The instance body scope comprises cells (`const`/`attr`/`recurrent`/`derived`/`stream`), `view` declarations, named acceptance entries…" | Cell list gains `collect as x` bindings and `yielded` groups | W2(n) |
| 020-23 (L2690) | EXT | "Reserved words cannot be declared as cell names." | Unchanged rule; reserved set grows (`collect`, `yield`, `fold`, `by`, `cell`, `yielded`) — stated here | W2(n), W2(b) |
| 020-37 (L2704) | EXT | "Every named identifier in an instance body scope — cells of all kinds…" | Kind list gains `yielded`; collision rule covers `collect as` names | W2(n) |

### Section 021 (142 → 142)

| Entry | Act | Anchor | Change | Refs |
|---|---|---|---|---|
| 021-16 (L2724) | EXT | "A placement attr RHS that references reactive cells … creates an implicit derived bridging the sourc…" | Mechanism EXTENDED to arguments: reactive expressions in any cell position (operator args, effect args, `\|>` LHS) synthesize the same implicit derived bridge | C11 |
| 021-46 (L2754) | ADJ | "A placement-body `for` whose iterable is not compile-time-known is a compile error pointing at the iterable…" | Unchanged; the rule SHAPE R4's yield-conditional legality mirrors | C8, R4 |
| 021-140 (L2848) | RW | "A top-level node instance that is not the entry-point and is not reachable from the entry-point's transitive closure…" | Closure gains: node references bound as effect arguments (as borrows) AND wire-candidate envelopes | C3, C16 |
| 021-141 (L2849) | RW | "The entry-point's transitive closure defines the live graph: cells are allocated, instances are mounted…" | Same closure extension restated locally (INV2); interpretation renders mount at startup within this closure | C3, C16 |
| 021-142 (L2850) | ADJ | "The static-vs-reactive distinction at placement is a normative per-placement choice (not per-attr-declaration)…" | Unchanged; provenance-resolution substrate for C11 | W2(m), C11 |

### Section 022 (120 → 121; +1 ADD after 022-71)

| Entry | Act | Anchor | Change | Refs |
|---|---|---|---|---|
| 022-3 (L2856) | ADJ | "A value conditional evaluates its scrutinee, selects one arm, discards the rest, and yields a value." | Unchanged | W2(m) |
| 022-5 (L2858) | PR | "`if`/`else`/`match` remain value selection everywhere they appear … never gate structure." | Protected (SOFT): R4 preserves it — yield-under-conditional is legal only when the conditional is compile-time (expansion, not gating) | C8, R4 |
| 022-14 (L2867) | PR | "Activation is never surfaced as a readable cell or field; source code cannot read gate state as a value." | Protected: Gated entries never surface predicate/scrutinee | C6 |
| 022-37 (L2890) | EXT | "A cyclically self-referential gate predicate evaluates against the gated cells' previously-committed values from the prior commit." | Wake gates INHERIT this rule (prior-commit reads, one-commit-delay fixpoint) — clause added | C16 |
| 022-45 (L2898) | ADJ | "An unresolved dynamic destination behaves as gate-false, and resolution returning `Some` behaves as the false-to-true flip…" | Unchanged; wire-inside-gated-off-arm activates nothing | C16 |
| 022-56 (L2909) | ADJ | "A gated node's outputs do not propagate; its outgoing connections do not deliver to their destinations." | Unchanged; read-path 1 of the documented gate system (D.5) | C6 |
| 022-57 (L2910) | ADJ | "A gated connection does not propagate at all; its destination receives nothing through that connection." | Unchanged; read-path 1 companion | C6, C16 |
| 022-67 (L2920) | ADJ | "An instance is effectively active iff its own gate and every ancestor gate on the path from the root are open." | Unchanged; defines "effectively active" for the new fourth read-path | C6 |
| 022-70 (L2923) | ADJ | "Cell reads on gated subgraphs always return a defined value of type `T`, never `Option[T]`." | Unchanged; read-path 2 | C6 |
| 022-71 (L2924) | ADJ | "Reads on a gated instance return frozen values: the last value committed during an active period…" | Unchanged; read-path 3; gains the ADD below | C6 |
| ADD 022-71+1 | ADD | insert after 022-71 | NEW fourth gate read-path: value contributions from gated positions join folds as ACTIVATION-DRIVEN members — present iff effectively active — unlike frozen direct reads; the three existing read-paths are unchanged | C6 |
| 022-101 (L2954) | ADJ | "In a `when:` or `given` block the fallback arm — `otherwise:` for `when:`, `default:` for `given:` — must be the last arm…" | Unchanged; evidence for R11 (no third `else` sense) | R11 |

### Section 023 (56 → 56)

| Entry | Act | Anchor | Change | Refs |
|---|---|---|---|---|
| 023-19 (L2995) | ADJ | "A dependency edge whose gate predicate evaluates false does not propagate to destination outputs." | Unchanged; the on/off-bit machinery yielded membership reuses | C9 |
| 023-46 (L3022) | RW | "`WeakHandle[T]` resolution — individually or in its collective dynamic-namespace form — is the only source of dynamic dependency…" | Rewritten: WeakHandle resolution AND Portal resolution are the TWO dynamic-dependency sources (coordinated with 025-6; ADJ anchor 017-204) | C18 |
| 023-47 (L3023) | PR | "A read through a `WeakHandle[T]` reaches whichever entity the handle currently resolves to, and the dependency flips on re-point…" | Protected: wire lowering path (a) uses exactly this, unchanged | C16 |
| 023-56 (L3032) | ADJ | "The candidate set of possible handle targets is statically known." | Unchanged; closed candidate envelope for mount-time tagging | C4, W2(m) |

### Section 024 (27 → 27)

| Entry | Act | Anchor | Change | Refs |
|---|---|---|---|---|
| 024-13 (L3048) | ADJ | "A recurrent cell on a cycle behaves as a one-commit delay element…" | Unchanged; wake-gate fixpoint is consistent with it | C16 |
| 024-17 (L3052) | EXT | "A connection whose destination is a dynamic reference contributes candidate edges to each node type the reference's static type admits…" | Extended: candidate envelopes also feed the interpretation closure (containment UNION envelopes) | C16 |
| 024-22 (L3057) | ADJ | "Every node type in a connection's candidate envelope must declare a `dynamic incoming` connection-view for that connection type…" | Unchanged; guarantees interpreters can read incoming views on every candidate | W2(m), C17 |
| 024-26 (L3061) | EXT | "Connection types that imply simultaneous source-destination activation should not satisfy `Circularity`." | REINFORCED: clause added noting wake-gate cycles make this guidance load-bearing | C16, W2(n) |

### Section 025 (65 → 65)

| Entry | Act | Anchor | Change | Refs |
|---|---|---|---|---|
| 025-6 (L3071) | RW | "A provenance set is normally static; the sole exception is a read through `WeakHandle[T]` resolution…" | The missed parallel of 023-46: rewritten to name BOTH dynamic-dependency sources (WeakHandle resolution AND Portal resolution) | C18, W2(m) |
| 025-40 (L3105) | ADJ | "The dynamic-collection cost model is normative: implementations must achieve the documented complexity bounds." | Unchanged; cost-rule family precedent for fold | C10 |

### Section 026 (10 → 10)

| Entry | Act | Anchor | Change | Refs |
|---|---|---|---|---|
| 026-2 (L3135) | EXT | "A derived or recurrent expression that traps during evaluation — arithmetic overflow … division by zero…" | Trap-site list gains fold `by:` combiner evaluation and collect/yield member expressions | W2(n) |
| 026-9 (L3142) | EXT | "The reactive evaluation context does not modify trap semantics: a behavior that traps aborts the process…" | Same extension restated locally (fold `else:` arms and combiners are behaviors; trap = abort) | W2(n) |

### Section 027 (120 → 120)

| Entry | Act | Anchor | Change | Refs |
|---|---|---|---|---|
| 027-45 (L3191) | RES | "The per-instance `write_signal` form targets an effect's `observed:` signal on one effect instance…" | Light RES (W2(j)): observed SIGNALS stay host-written; wording gains "host-written channels" phrasing to align with the C1 split (computed observed outputs are not written this way) | C1, W2(j) |
| 027-80 (L3226) | RW | "Generic-effect instantiations without a registered reconciler are detected at startup and cause the runtime to refuse…" | ⚛A core site: conditional added (B.2 text; generic instantiations of effects WITH host-written observed channels) | C1; ⚛A |
| 027-81 (L3227) | RW | "If the program declares an effect type with no registered reconciler, startup fails with a diagnostic naming the effect type…" | ⚛A core site: conditional added | C1; ⚛A |

### Section 028 (75 → 75)

| Entry | Act | Anchor | Change | Refs |
|---|---|---|---|---|
| 028-4 (L3273) | EXT | "Reactive cells are identified across reloads by fully-qualified declaration path — module path, instance name, cell name…" | Extended: interpreter-placed effect paths are rooted at the interpretation site, mirroring node paths | C18 |
| 028-5 (L3274) | EXT | "Anonymous or duplicated sibling placements get an ordinal suffix `:N`…" | Extended: `:N` ordinals apply to interpreter-placed effects | C18 |
| 028-33 (L3302) | ADJ | "Stream policy changes (`ring` ↔ `gate`) require per-instance restart of the affected stream cells and their consumers." | Unchanged; survives the two-axis remodel (policy axis intact) | W2(m), C14 |
| 028-50 (L3319) | ADJ | "The synthesized observation cells (`pending_count`, `pressure`, `is_full`, …) survive…" | Unchanged; `x_count` names exempt from the count rule | C15 |
| 028-68 (L3337) | ADJ | "Effect instances map to graph-IR `effect` primitives: groupings of standard cells plus a reconciler reference." | Unchanged text; note: reconciler reference becomes OPTIONAL in IR semantics via 033-122 (the ⚛A site) — no restatement here | W2(m), C1 |
| 028-73 (L3342) | ADJ | "Host-registered reconcilers are dispatched at the commit boundary; `suspend`/`resume` fire there on gate-close/gate-open…" | Unchanged; child-effect lifecycle cascades ride the same boundary (031 ADDs state it) | W2(m), C1 |

### Section 029 (125 → 124; DEL 029-108)

| Entry | Act | Anchor | Change | Refs |
|---|---|---|---|---|
| 029-18 (L3365) | ADJ | "Default values on `Cell[T]` parameters are not allowed in v1; a stdlib helper constructing a constant cell substitutes…" | Unchanged rule; kind respell (`cell T` params); the constant-cell helper is subsumed contextually by C11's static-wrap but the v1 no-defaults rule stands | W2(m), C11, C14 |
| 029-20 (L3367) | ADJ | "A static value passed to a `Cell[T]` parameter is wrapped as a degenerate `Derived[T]` cell…" | Unchanged mechanism (C8/C11 cite it); kind respell | C8, C11 |
| 029-22 (L3369) | ADJ | "A cell passed to a `Cell[T]` parameter binds directly." | Unchanged; leg 1 of the uniform rule | C11 |
| 029-27 (L3374) | RW | "Auto-deref does not apply where the context expects `Cell[T]` directly — operator parameters, `Cell[T]`-typed function parameters, `\|>`…" | Rewritten: the bare-cell-vs-value dichotomy was incomplete — cell positions accept ANY expression, resolved by provenance (bind / constant-wrap / derived-bridge) | C11 |
| 029-29 (L3376) | EXT | "The permitted operator body items are `recurrent` declarations …, `derived` declarations, `stream` declarati…" | Item list gains the `fold` expression form (and `collect`) | C10, C12 |
| 029-30 (L3377) | EXT | "An operator body that is just its final expression (no declarations) may be written inline after the colon…" | Clause added: a `fold` form forces the indented block (never inline) | C10 |
| 029-31 (L3378) | ADJ | "A static value (literal, `const`, or expression evaluated entirely from compile-time constants) in a `Cell[T]` position is wrapped…" | Unchanged; leg 2 of the uniform rule | C11 |
| 029-33 (L3380) | ADJ | "An operator-body `let` binding behaves as a synthesized derived for dependency-tracking purposes." | Unchanged; bridge-path precedent (bridge is attributed to the call site) | C11 |
| 029-41 (L3388) | ADJ | "A final expression that is already a `Cell[T]` (a named recurrent, derived, or stream in the body) is the output directly…" | Unchanged; kind respell only | W2(m), C14 |
| 029-43 (L3390) | RW | "Event-producing operators (e.g. `to_ring_stream`, `to_gate_stream`, `filter`, `merge`) return `Stream[T]`: `operator changes[T](source: C…" | R3 FULL REWRITE: the example operator `changes` is replaced with a differently-named illustration (e.g. `rising_edges`), because its function is subsumed by synthesized `.changes`; the dead `to_ring_stream`/`to_gate_stream` names are removed | R3, C12 |
| 029-44 (L3391) | ADJ | "Multiple outputs are returned as a record or tuple — either a plain record/tuple (form 1, exposed as a single `Derived`)…" | Unchanged; kind respell only | W2(m), C14 |
| 029-45 (L3392) | ADJ | "An operator's value-typed output is implicitly wrapped into a reactive cell via the `Cell[T]`≅`T` relationship…" | Kind respell (`cell T`≅`T` under the ⚛B rewrite); rule intact | W2(m), C14 |
| 029-57 (L3404) | ADJ | "An operator instance is identified by its enclosing scope, the operator name, and its argument bindings." | Unchanged; per-site identity precedent for `.changes` | C12 |
| 029-65 (L3412) | RW | "`|>` connects a source cell on the left to a destination on the right; the connection kind dispatches on the RHS kind: operator call or e…" | THIRD dispatch case added: node-reference LHS when RHS is an effect-kind trait method with Subject-typed first param | C3, W2(l) |
| 029-74 (L3421) | PR | "Using `\|>` with a `fn` RHS is a compile error: `0.0 \|> some_fn` ✗." | Protected (SOFT, EC7): stays an error; NO fn exemption; note the LOG text contains "RHS" (the SPEC parallel L19526 does not — both correct as-is) | EC7, W2(l) |
| 029-103 (L3450) | ADJ | "Operators share the composition surface — `\|>` pipe form, instance identity, parameter rules, generics, visibility — with effects." | Unchanged; the shared surface is why the third dispatch case homes in 029 | W2(m), C3 |
| 029-106 (L3453) | RW | "A normative diagnostic class rejects `\|>` with a non-operator, non-effect right-hand side." | Rewritten to admit the third case: rejects non-operator, non-effect, non-effect-kind-trait-method RHS | C3, W2(l) |
| 029-108 (L3455) | DEL | "A normative diagnostic class rejects passing a non-cell, non-literal value expression to a `Cell[T]` parameter." | DELETED — replaced by the uniform provenance rule (C11); section tail renumbers | C11 |

### Section 030 (262 → 260; DEL ×4, ADD ×2 — the −4/+2 math, fully shown)

The nine entries 030-123..131 (LOG L3598-3606, all texts fetched and verified):

| Entry | Act | Anchor (verified) | Change | Refs |
|---|---|---|---|---|
| 030-123 (L3598) | DEL | "`to_ring_stream`/`to_gate_stream` are the explicit signal-to-stream constructors: `to_ring_stream[T, …" | DELETED (constructors abolished) | C12 |
| 030-124 (L3599) | RW | "At stream creation, the constructor emits the source's current value as event 0." | Re-homed to `.changes` creation semantics: at materialization, `.changes` emits the source's current value as event 0 | C12 |
| 030-125 (L3600) | RW | "After creation, the constructor appends one event per commit of a new source value." | Re-homed to `.changes` (one event per commit of a new source value) | C12 |
| 030-126 (L3601) | RW | "Same-value commits of the source emit no event." | Re-homed to `.changes` (text survives with `.changes` subject) | C12 |
| 030-127 (L3602) | RW | "Capacity defaults to `1024`: `some_signal \|> to_ring_stream` is `RingStream[T, 1024]`." | Re-homed: unpinned `.changes` defaults to `ring[1024]`; example respelled (`some_signal.changes` is `stream ring[1024] T`) | C12, C14 |
| 030-128 (L3603) | DEL | "Capacity is overridable via turbofish: `some_signal \|> to_ring_stream::[Url, 2048]`." | DELETED (turbofish override dies with the constructor; capacity now pins via the consuming declaration or param/return type) | C12 |
| 030-129 (L3604) | DEL | "`to_ring_stream` produces `ring` policy; `to_gate_stream` produces `gate`." | DELETED | C12 |
| 030-130 (L3605) | DEL | "A gate stream from a signal is obtained directly via `signal \|> to_gate_stream` (no post-hoc policy c…" | DELETED | C12 |
| 030-131 (L3606) | RW | "There is no implicit signal-to-stream conversion in reactive expressions; a signal is converted expli…" | Respelled: no implicit conversion; a signal becomes an event source only via its synthesized `.changes` member | C12 |

Net for this block: −4. The two ADDs (below) land after 030-131: net −4 +2 = **−2**;
section 262 → **260**.

| # | Insert after | New entry text (gist) | Refs |
|---|---|---|---|
| ADD 030-131+1 | 030-131 | New: `sig.changes` is a SYNTHESIZED per-use-site stream member (synthesized-member precedent: the 030-52..60 observation cells; per-site identity: operator-instance identity rules). Two textual `.changes` sites = two streams | C12 |
| ADD 030-131+2 | after previous | New: `.changes` policy/capacity resolve like PLACEHOLDERS at use sites: a consuming `stream` declaration pins it (materializes AS the declared stream, one buffer); a parameter/return type pins it; otherwise `ring[1024]`; unresolved display = erased `stream T` | C12 |

Remaining 030 rows:

| Entry | Act | Anchor | Change | Refs |
|---|---|---|---|---|
| 030-29 (L3504) | RW | "A signal-only expression is not a stream source; convert the signal explicitly: `stream ring url_events: Url = current_url \|> to_ring_str…" | Example respelled to `.changes` (`current_url.changes`) | C12 |
| 030-35 (L3510) | RW | "The erased supertype `Stream[T]` carries only the element type — no policy…" | Respelled: the erased spelling is `stream T` (kind form), replacing `Stream[T]` | C14, C12 |
| 030-47 (L3522) | RW | "`Cell[T]` is the umbrella trait over reactive cells carrying `T`, implemented by `Signal[T]`, `Derived[T]`, …" | ⚛B: trait-side rewrite — `cell` is a KIND (coordinated with 016-61/162/165, 030-49/51 + SPEC both sides) | C14; ⚛B |
| 030-49 (L3524) | RW | "`Cell[T]` has no required methods at the language level; it is a marker trait…" | ⚛B: rewritten (kind, not marker trait) | C14; ⚛B |
| 030-51 (L3526) | RW | "The common abstraction over heterogeneous reactive cells is the trait `Cell[T]`." | ⚛B: rewritten (the common abstraction is the `cell` KIND) | C14; ⚛B |
| 030-52 (L3527) | ADJ | "Every bounded stream automatically exposes derived signal cells describing its state…" | Unchanged; the synthesized-member precedent `.changes` cites | C12 |
| 030-60 (L3535) | ADJ | "Observation cells are never declared in user code; the compiler synthesizes them…" | Unchanged; precedent | C12 |
| 030-67 (L3542) | PR | "A signal in a stream expression is sampled at each driving-stream event; it contributes no events of its own…" | Protected (SOFT): sampling rule STAYS; the rationale (phantom events; either-changes exists as derived; per-operand control) moves to NEW SPEC §13.18.7 prose subsection (D.5) | C12 |
| 030-70 (L3545) | PR | "With multiple reactive inputs the default combining behavior is combine_latest…" | Protected: combineLatest lives HERE (stream expressions), not in observe arms | C13 |
| 030-72 (L3547) | PR | "Combining behaviors other than combine_latest (`zip`, `sample`, `merge`, etc.) require explicit stdlib operators." | Protected; wording aligned with no-privilege (user-writable) via 030-121 rewrite | C13, C12 |
| 030-88 (L3563) | RW | "A `stream` declaration whose RHS contains no streams (signals only) is a compile error; convert a signal explicitly (`to_ring_stream`/`to…" | Wall stays; conversion spelling → `.changes` | C12 |
| 030-90 (L3565) | RW | "There is no implicit conversion; a signal becomes an event source only via explicit `to_ring_stream`/`to…" | Wall stays; respell to `.changes` | C12, W2(m) |
| 030-91 (L3566) | RW | "There is no implicit signal→stream conversion: a `stream` source must contain at least one stream input; a signal is converted explicit…" | Wall stays; respell to `.changes` | C12, W2(m) |
| 030-117 (L3592) | PR | "`recurrent[N] stream` is invalid in effect `observed:` blocks." | Protected: the stream half of the split lift stays rejected (EC8) | C1, EC8 |
| 030-118 (L3593) | PR | "Effects needing history-aware behavior must compute it in the host's reconciler." | RW-light within PR intent: "in the host's reconciler" gains "or in child effects" (interior effects exist now) — meaning-preserving extension, flagged for review | C1, EC8 |
| 030-121 (L3596) | RW | "Stream-producing, stream-transforming, and stream-consuming operators are stdlib primitives." | Reworded per NO-PRIVILEGE: map/filter/merge are user-writable; nothing about them is compiler-privileged | C12 |
| 030-135 (L3610) | RW | "`skip_first` is equivalent to `skip(1)`: `current_url \|> to_ring_stream \|> skip_first` drops the initial-value event." | Example respelled to `.changes` | C12 |
| 030-139 (L3614) | RW | "`count[T](source: Stream[T]) -> Derived[i64]` is the running count of observed events, starting at `0`." | R2: RENAMED `event_count` (applies the C15 naming rule to itself); signature respelled to kind forms | R2, C15 |
| 030-140 (L3615) | RW | "`fold[T, A](source: Stream[T], init: A, f: fn(A, T) -> A) -> Derived[A]` is a running accumulator whose initial value is `init`." | R1: RENAMED `accumulate` (frees `fold` for the expression form; pairs with `scan`); NOTE: 030-140 is the operator to rename, NOT 030-139 | R1, C10 |
| 030-151 (L3626) | ADJ | "`scan[T, A, P: StreamPolicy](source: Stream[T, P], init: A, f: fn(A, T) -> A) -> Stream[A, P]` emits `f(state, event)`…" | Unchanged semantics; signature respell under two-axis policy + kind forms | R1, C14 |
| 030-158 (L3633) | PR | "Inside `C`, the LHS stream's name denotes the current event: `A.field` reads the event's field…" | Protected: inside `where C` the bare stream name keeps its event meaning; the R5 binder governs arm BODIES only | C13 |
| 030-181 (L3656) | ADJ | "Mixed-policy reactive expressions are permitted; the output's policy is the LHS-declared policy…" | Unchanged; policy conversions user-writable context | C12 |
| 030-246 (L3721) | RES | "A signal never satisfies a `Stream[T]` parameter or a stream binding — there is no implicit conversion; convert explicitly (`signal \|>…" | HARD DEFECT fix (W2(n)): dead constructor names replaced with `.changes`; rule itself intact | C12, W2(n) |
| 030-247 (L3722) | ADJ | "Assigning a stream-valued expression to a signal binding is a normative error class…" | Unchanged; kind respell in example only | W2(m), C14 |
| 030-248 (L3723) | ADJ | "Reading a bare stream as a value is a normative error class: `derived latest: Event = events` ✗…" | Unchanged | W2(m) |
| 030-255 (L3730) | RW | "The recurrent policies are `RecurrentRing[B, H]` / `RecurrentGate[B, H]` (buffer `B`, `H`-deep self-history as separate allocations)…" | ⚛D: remodeled to TWO-AXIS — policy {Ring[N], Gate[N]} × history-depth parameter (default 0); RecurrentRing/RecurrentGate cease to be policy members | C14; ⚛D |
| 030-259 (L3734) | RW | "`StreamPolicy` is a sealed (language-closed) trait…; its members are exactly `Ring[N]`, `Gate[N]`, `RecurrentRing…" | ⚛D: sealed set becomes exactly {Ring[N], Gate[N]}; the RecurrentRing/RecurrentGate fulfill/alias lines are deleted from the entry | C14; ⚛D |
| 030-262 (L3737) | RW | "The stdlib provides transparent generic aliases for the common stream spellings: `RingStream[T, N]` = `Stream[T, Ring[N]]`, `GateStream[…" | ⚛D: alias spellings survive as sugar over the two-axis model | C14; ⚛D |

### Section 031 (153 → 158; +5 ADDs at section tail, after 031-153)

| Entry | Act | Anchor | Change | Refs |
|---|---|---|---|---|
| 031-1 (L3741) | ADJ | "An effect is a reusable, cell-allocating reactive construct describing a desired alignment between program state and external reality…" | Unchanged; the reconciliation-contract framing generalizes it without contradiction (interior effects align via children) | W2(m), C1 |
| 031-16 (L3756) | RW | "The `desired:` and `observed:` blocks may be declared in either order; the canonical order is `desired:` first…" | SUPERSEDED-when-body-items-present: with child placements, order is body items → `desired:` → `observed:` LAST; without body items the old freedom stands | C1 |
| 031-49 (L3789) | RW | "`recurrent[N] stream` is not valid in `observed:` blocks." | SPLIT lift (EC8): value `recurrent` (and `derived`) become VALID in `observed:` as computed outputs; `recurrent[N] STREAM` stays invalid — entry restates both halves | C1, EC8 |
| 031-70 (L3810) | ADJ | "Effect hot-reload identity follows the operator scheme, with tolerance for positional moves within the same clause." | Unchanged; child placements get paths per 028-4/5 EXT | W2(m), C18 |
| 031-117 (L3857) | PR | "An effect instance never auto-projects to a single cell in pipe-out position." | Protected: bootstrap projection is EXPLICIT (`render(song).audio`) | C3 |
| 031-119 (L3859) | RW | "The host registers a reconciler for each effect type via the host API, keyed by the effect's type name." | ⚛A SUPPORT site (not core-counted): gains the conditional clause | C1; ⚛A-support |
| 031-128 (L3868) | RW | "An effect type appearing in the graph with no registered reconciler triggers a startup diagnostic, and the runtime refuses…" | ⚛A core site: conditional (B.2 text) | C1; ⚛A |
| 031-134 (L3874) | RW | "An effect's `desired:` block may not contain another effect's instantiation: effects do not compose from effects." | PARTIAL lift (EC8): top-of-body child placements ARE allowed; effect-instantiation as a desired-cell EXPRESSION stays an error; "effects do not compose" clause deleted | C1, EC8 |
| 031-143 (L3883) | RW | "A normative diagnostic class covers an effect type with no registered reconciler, raised at runtime startup." | ⚛A core site: conditional | C1; ⚛A |
| 031-146 (L3886) | RW | "A normative diagnostic class covers a non-role-keyword declaration inside an effect's blocks, and `recurrent` in `observed:`; `recurrent…" | SPLIT lift: `derived`/value-`recurrent` in `observed:` no longer diagnosed; `recurrent[N] stream` in `observed:` still diagnosed | C1, EC8 |
| 031-152 (L3892) | RW | "`recurrent` is allowed in an effect's `desired:` block but forbidden in `observed:`." | SPLIT lift: value-`recurrent` now allowed in `observed:` too; stream-recurrent forbidden (restated locally) | C1, EC8 |
| 031-153 (L3893) | ADJ | "A `desired:` event-output stream's `= source` may read the effect's own `observed:` cells (feedback); such a cycle must pass a delay…" | Unchanged; feedback rule coexists with computed observed outputs | C1 |
| ADD 031-154 | ADD | after 031-153 | New: effect bodies may place child effects as top-of-body items (`x = effect_expr`) BEFORE `desired:`/`observed:`; placements are graph placements, not cell expressions | C1 |
| ADD 031-155 | ADD | after previous | New: child effects lifecycle-cascade with the parent — mount/teardown/suspend/resume | C1 |
| ADD 031-156 | ADD | after previous | New (the 003 authoring note's home): `observed:` is the effect's PUBLIC surface / export boundary; child placements are PRIVATE | C1, W2(n) |
| ADD 031-157 | ADD | after previous | New (⚛A SUPPORT, not core-counted): mixing child placements and host-written channels is allowed; registration requirement follows the host-written-channel conditional (restated locally) | C1; ⚛A-support |
| ADD 031-158 | ADD | after previous | New: every effect is a reconciliation contract — fulfilled by a host reconciler (leaf), by child effects (interior), or both (the SPEC §13.19 framing) | C1 |

### Section 032 (179 → 179)

| Entry | Act | Anchor | Change | Refs |
|---|---|---|---|---|
| 032-87 (L3983) | CON | "Cells declared inside an `effect`'s `desired:` and `observed:` blocks (§13.19.4, §13.19.5) are reactive state — ordinary Signal or St…" | CON (W2(k)): the positive enumeration ("Signal or Stream") is now false — observed blocks may hold derived/value-recurrent cells; rewrite enumerates all admitted kinds | C1, W2(k) |
| 032-107 (L4003) | ADJ | "The runtime reaches a behavior only through the behavior ABI; it never reaches inside." | Unchanged; cited as fold-form rationale (behaviors opaque ⇒ fold cannot be a method) | C10, W2(k) |

### Section 033 (234 → 236; +2 ADDs after 033-80)

| Entry | Act | Anchor | Change | Refs |
|---|---|---|---|---|
| 033-56 (L4134) | PR-HARD | "The graph is built from exactly six primitives: `cell`, `connection`, `gate`, `stream`, `effect`, and `scope`." | EC2: byte-identical — fold is a cell KIND, not a seventh primitive | EC2, R6 |
| 033-64 (L4142) | ADJ | "An effect-position `\|>` (binding the effect's pipe-target parameter) lowers to the effect entry's `parameter_bindings`…" | Unchanged; the third dispatch case lowers through the same `parameter_bindings` path | W2(m), C3 |
| 033-65 (L4143) | EXT | "Gating is encoded in the graph IR as first-class gate objects — `{id, pred (behavior handle + input cell IDs), guards…" | Extended: compiler-synthesized wake gates AND per-arm gate_parents of Gated walks lower to these SAME gate objects (no new IR construct) | C6, C16 |
| 033-80 (L4158) | RW | "A cell entry carries a `kind` — `input` (a stored, externally-written cell), `derived`, or `recurrent` — classifying it…" | R6: kind enum gains `fold` → input \| derived \| recurrent \| fold | R6, C10 |
| ADD 033-80+1 | ADD | after 033-80 | New: a fold-kind cell carries a combiner behavior id, an else value, and member edges in member order, each edge tagged with its membership driver: permanent / keyed-template / gate-guarded | R6, C10 |
| ADD 033-80+2 | ADD | after previous | New: fold surface is unchanged — the result lives in a `derived` declaration; consumers see `derived T`; membership changes propagate dirt like value changes | R6, C10, C9 |
| 033-109 (L4187) | ADJ | "A stream entry's `observation_cell_ids` name the synthesized observation cells: `pending_count`, `pressure`, …" | Unchanged; `x_count` exemption context | C15 |
| 033-121 (L4199) | ADJ | "An effect entry's `observed_cell_ids` name the cells declared in the instance's `observed:` block." | Unchanged; now may include derived/value-recurrent cell ids (no text change needed — it says "cells declared") | W2(m), C1 |
| 033-122 (L4200) | RW | "Reconciler dependencies are `(effect_type_name, [concrete_type_parameters])` pairs the host must register via `runtime.register_reconcil…" | ⚛A core site: "must register" gains the conditional (only effect types whose `observed:` has host-written channels appear in the dependency list) | C1; ⚛A |
| 033-128 (L4206) | EXT | "A cell ID is a dot-separated identifier path from module root through enclosing instances to the cell name: `audio.synth_a.osc_1.frequen…" | Extended: interpreter-placed effect paths root at the interpretation site, mirroring node paths; `:N` ordinals apply | C18 |
| 033-133 (L4211) | EXT | "Cross-implementation hot reload at the same source version yields matching cell IDs by construction." | Extended to cover interpreter-placed effect ids (same construction) | C18 |
| 033-193 (L4271) | EXT | "An `observe` expression lowers to a derived read." | Preserved + extended: the R5 `as` binder adds an event INPUT to the derived read; no new IR primitive | C13, W2(n) |
| 033-234 (L4312) | ADJ | "The IR representation of an attr placement encodes its provenance category (B static / C reactive) per placement…" | Unchanged; the PRECEDENT for R8 — `parameter_bindings` gains a provenance MARKER (not a third binding case), specified as a new clause on the parameter_bindings entry nearest 033-64 during execution | R8, C11 |

---

# PART C — NEW LOG SECTIONS

## C.1 Section 034 — collect / yield / yielded (13 entries)

All entries carry SPEC refs into the new §13.20 (collect/yield) authored in D.7.

| Id | Entry text (gist) | Refs |
|---|---|---|
| 034-1 | `collect:` is a block expression; `collect as x:` is the statement form; both legal | C8 |
| 034-2 | `yield <expr>` contributes ONE member-cell; single invariant meaning: member while this position is live | C8 |
| 034-3 | Positional membership: static position = permanent; inside `repeat` = key-driven; inside gated arm = activation-driven | C8, C6 |
| 034-4 | Bare `yield` directly under `collect` is legal; `yield` outside `collect` is a compile error | C8 |
| 034-5 | `collect` is not a coroutine: it evaluates once and builds a living membership structure | C8 |
| 034-6 | `yield` accepts any expression: reactive provenance → its cell; static → degenerate constant cell | C8, C11 |
| 034-7 | R4: `yield` under a value `if`/`match` inside `collect` is legal iff the condition/scrutinee is compile-time-known (conditional expansion); reactive/runtime = compile error with diagnostic offering `when`/`given` arms or a conditional VALUE yield (`yield if c: a else: b`); if/match never gate structure | C8, R4 |
| 034-8 | `yielded T` = the ordered (walk-order), membership-varying group of cells a `collect` produces | C9 |
| 034-9 | A `yielded` group stores nothing: compile-time wire set + runtime on/off bits | C9 |
| 034-10 | `yielded` is a cell-KIND: membership changes propagate dirt | C9 |
| 034-11 | Consumption: the fold form; yielded-typed params (operators AND fns); `repeat` over it; NO indexing; structural `for` over a yielded group = compile error | C9 |
| 034-12 | `yielded T` fulfills `Iterable` at LANGUAGE level (authored like Keyed/StringifiableKey); Item = T member values in walk order; runtime-loop contexts only; NOT a dynamic-view consumer | C9, W2(a) |
| 034-13 | A `yielded` group has a synthesized `count` member (element tally; bare-count naming rule) | C9, C15 |

## C.2 Section 035 — the fold form (10 entries)

SPEC refs into the new §13.21 (fold) authored in D.7.

| Id | Entry text (gist) | Refs |
|---|---|---|
| 035-1 | `fold` is a language EXPRESSION form — not an operator, not a method (behaviors are opaque) | C10 |
| 035-2 | Block shape: `fold <members>:` / `by: <expr>` / `else: <expr>`; newline-separated arms, no comma; both arms mandatory; `else` last | C10 |
| 035-3 | `by:` takes a `fn(T,T)->T` combiner expression; associativity is asserted by using the form; commutativity NOT required | C10 |
| 035-4 | `else:` is the EMPTY-MEMBERSHIP result, NEVER combined into non-empty results (semigroup + default; no identity; Monoid does not exist in the design) | C10 |
| 035-5 | Normative cost rule (family of the loop/collection cost rules): O(log n) combines per member value-change/join/leave; deterministic combine tree from member order | C10 |
| 035-6 | Scope: `yielded` groups + reactive composites with uniform slot type; members = slots in declared order; zero slots → else | C10 |
| 035-7 | `Cell[Vec[T]]`-shaped values are NOT foldable by this form (fold is over membership structures, not stored collections) | C10 |
| 035-8 | `fold` joins operator-body items and derived RHS; a fold in an operator body forces the indented block form | C10 |
| 035-9 | IR lowering: NEW cell kind `fold` (kind enum input\|derived\|recurrent\|fold); the six-primitives count is UNCHANGED | R6, EC2 |
| 035-10 | A fold-kind cell carries combiner behavior id, else value, member edges in member order with membership drivers (permanent / keyed-template / gate-guarded); surface = `derived T` | R6 |

## C.3 Homes for relocated/new content (cross-linked to Part A definitions)

| Content | Home | See |
|---|---|---|
| Entry-sum cluster (closed five-kind sum; A.0 item 4; 0.3 "Entry sum") | 017 ADDs A9–A15 | B §017 |
| Record-match ban anchor | 009 ADD 009-90+1 | B §009 |
| Kind taxonomy + storability razor (0.3 "Kind sweep") | 016 ADDs 016-176+1/+2 | B §016 |
| `.changes` synthesized member | 030 ADDs 030-131+1/+2 | B §030 |
| Effect composition + export boundary + contract framing (0.3 "Reconciliation contract") | 031 ADDs 031-154..158 | B §031 |
| fold IR payload | 033 ADDs 033-80+1/+2 | B §033 |
| bare-count naming rule | 012 ADD 012-91+1 | B §012 |
| repeat expression + typed binder | 018 ADDs 018-83+1..+3 | B §018 |
| connection endpoint deriveds (0.3 "Wire-following") | 019 ADD 019-58+1 | B §019 |
| fourth gate read-path | 022 ADD 022-71+1 | B §022 |
| Type[C] razor sibling | 008 ADD 008-56+1 | B §008 |
| observed: accepts computed outputs | 016 ADD 016-9+1 | B §016 |

---

# PART D — SPEC EDIT INVENTORY

Every row is standalone-executable: respell rows inline their mapping or cite the D.0
table by row id — no reliance on unexcerpted LOG text. Line numbers are pre-edit and
verified unless marked (~) approximate.

## D.0 Kind respell mapping table (cited by respell rows as D0-1 … D0-10)

| # | Old spelling | New spelling |
|---|---|---|
| D0-1 | `Signal[T]` | `signal T` |
| D0-2 | `Derived[T]` | `derived T` |
| D0-3 | `Recurrent[T, N]` | `recurrent[N] T` (N omitted = `[1]`) |
| D0-4 | `Stream[T, Ring[N]]` / `RingStream[T, N]` | `stream ring[N] T` (aliases survive as sugar) |
| D0-5 | `Stream[T, Gate[N]]` / `GateStream[T, N]` | `stream gate[N] T` |
| D0-6 | policy-erased `Stream[T]` | erased `stream T` |
| D0-7 | `Cell[T]` (umbrella) | `cell T` (KIND) |
| D0-8 | `Cell[DynamicView[WeakHandle[V]]]` / `DynamicView[V]` | `dynamic view V` |
| D0-9 | recurrent stream forms | `recurrent[H] stream ring[N] T` (two-axis: policy × history depth, default 0) |
| D0-10 | (new) yielded groups | `yielded T` / `yielded f32[128]` |

Storables KEEP brackets: `Type[C]`, `Handle[T]`, `WeakHandle[T]`, `Portal[T]`. `Cell[T]`
in a bound position (016-178 style) is deleted, not respelled: bounds move to the
generic param list or `where`.

## D.1 §13.1 — the four dead host-claims (atomic group ⚛C; verified lines)

These four claims assume interpretation is host-only and effects are leaf-only. They
rewrite together in one commit:

| Site (verified) | Current text | Rewrite |
|---|---|---|
| L11355-56 | "…reconciliation model. The host-interpreted bridge between program state and the runtime environment." | "…reconciliation model. A reconciliation contract between program state and external reality — fulfilled by a host reconciler (leaf), by child effects (interior), or both." |
| L11370-71 | "hosts are free to interpret nodes of specific types (DSP node graphs, UI children, music clips), but are not obliged…" | Interpretation is performable by hosts OR in-language by effect-kind trait methods walking `.exposition`; obligation unchanged |
| L11377-78 (within L11377-81) | "Adding a new effect type is achieved by declaring a new `effect`; the host registers a reconciler for that effect type." | Gains the ⚛A conditional (registration iff host-written observed channels) — this site is also the ⚛A "coordinated companion" |
| L11378-81 | "Adding a new topological participant is achieved by declaring a new `node`; the host extends its interpreter for traversed node types…" | "…the host extends its interpreter — or an in-language interpreter gains a match arm — for traversed node types" |

## D.2 §13.3 — exposition, entry sum, bundles, gating

| Section | Edit | Refs |
|---|---|---|
| §13.3.7 (exposition) | New subsection: the five-kind closed entry sum; `for entry` + `match` walk; `Variant(name: Bound)` entry-match-only pattern; filter-arms/exhaustiveness rule; first-match-wins + dead-arm lint; monomorphization (static unroll; mount-time tag over closed envelope); participation/skip report | C4 (A9–A15) |
| §13.3.7.5 | 017-244's SPEC text untouched (EC1 perimeter) | EC1 |
| §13.3.7.6 | Engagement prose: split per 017-247/248 (meaning extrinsic survives; host-only interpretation generalized) | C16 |
| §13.3.3.4 / §13.4 | Dynamic-view surface: `.count` added; consumer surface unchanged (operators + `repeat`); kind respell D0-8 | C15, W2(a) |
| Bundle §§ (13.3 bundle rows) | `[B]` row-match subsection (write/accept/match trilogy completed); no `T[]`, no `[B ...]` | C5, R7 |
| §13.3.7 `when`/`given` entries | R10 prose: each arm body is the same walkable type as `.exposition`; Gated binds ALL arms; positional gate_parents | C6, R10 |

## D.3 §13.2 — kinds, cells, observe

| Section | Edit | Refs |
|---|---|---|
| §13.2.8 | ⚛B type-side rewrite: `cell T` is a KIND umbrella (membership incl. streams and yielded); D0-7; 016-178 deletion ripple (bounds via generics/`where`) | C14, EC3 |
| §13.2.6 (~L init-read) | 016-139 companion: "a `Signal[T]` cell" respells per D0-1 — YES, the respell applies (verified: the SPEC text names `Signal[T]`) | C14 |
| §13.2.11 (observe) | R5 grammar `on <trigger> [where C] [as <binder>]:`; post-filter binding; bare-name-in-C keeps event meaning (030-158); bodies use binder only; multi-trigger tuple binders; `Option[E]` slot typing; value-cell = latest-of-commit, stream = per-event; combineLatest rejected for arms | C13, R5 |
| §13.2.11.7 (~L, 016-267 twin) | RHS list respelled to kind forms (D0-1..D0-6, D0-9) + binder-typing coherence note | C13, C14 |
| §13.2 kind table | New consolidated annotation table mirroring 016-176+1/+2 (taxonomy + razor) | C14 |

## D.4 §9 / §4.9.5 — Map order and count

| Section | Edit | Refs |
|---|---|---|
| §9.5 | Map is insertion-ordered (JS semantics: update keeps position; delete+reinsert appends); merge (`+`) and `m[k]=v` positional pins; iteration/Iterable text updated; HashSet text untouched | C15, EC6 |
| §9.5.8 + L7382 | "`Ord` is **not** derived: `Map` iteration is unordered…" → conclusion survives; rationale reworded to keys-are-not-positions (verified: L7382 carries the stale rationale) | C15, W2(g) |
| §4.9.5 L4131-34 | "range-slicing is unmeaningful on an unordered keyed collection" → reworded to keys-are-not-positions (007-238 LOG side untouched) | C15, W2(m) |
| §9.3.7 & all `.length` sites | `.length` → `.count` rename (arrays/slices/bundles/Map/Vec/HashSet/Yielded/dynamic views); strings exempt (`byte_len`/`char_count`); stream metrics exempt (`pending_count` etc.) | C15 |

## D.5 §13.18 / §13.9 — streams, conversions, gates

| Section | Edit | Refs |
|---|---|---|
| §13.18.9 | Constructor deletion: every `to_ring_stream`/`to_gate_stream` occurrence incl. worked examples → `.changes`; `.changes` synthesized per-use-site member + placeholder-style policy/capacity resolution (mirrors 030-131+1/+2); `to_signal` presented as ordinary user-written operator (observe + default arm) | C12, R3 |
| §13.18.7 | NEW prose subsection: sampling-rule rationale (emit-on-either fabricates phantom events; either-changes already exists as derived; per-operand control) | C12 |
| §13.18.3 / §13.18.5 | ⚛D two-axis policy remodel ({Ring[N], Gate[N]} × history depth, default 0; aliases as sugar) + ⚛B trait-side rewrite incl. code block and "Why a trait, not a union type" rationale (verified L20072-75) — that rationale is REPLACED by the kind explanation | C14, EC3 |
| §13.18.9 operator list | R1 `fold`→`accumulate` (pairs with `scan`); R2 `count`→`event_count`; no-privilege wording for map/filter/merge | R1, R2, C12 |
| §13.9.7 | Gate read-path system subsection: the three existing paths (no-deliver; defined-value; frozen reads) + NEW fourth path (activation-driven fold membership); predicate never surfaced | C6 |
| §13.18.16 | 030-246/247/248 examples: dead names → `.changes`; kind respells D0-1..D0-6 | C12, W2(n) |

## D.6 §13.17 / §13.19 / §15.4 — operators, effects, IR

| Section | Edit | Refs |
|---|---|---|
| §13.17.7 (L19526) | `|>` third dispatch case (node-reference LHS, effect-kind trait-method RHS); the fn-error text at L19526 stays — note it reads "Using `|>` with a `fn` is a compile error:" (NO "RHS", unlike LOG 029-74 — both correct as-is, protect keys off each true text) | C3, EC7 |
| §13.17.12 | Diagnostic list: drop the 029-108 class; admit the third dispatch case in the `|>` RHS diagnostic | C11, C3 |
| §13.17.3/4 | Uniform cell-argument provenance rule (bind / constant-wrap / derived-bridge from call site); fold/collect join operator body items | C11, C10 |
| §13.19 | Reconciliation-contract framing (leaf/interior/both); child placements (form, order body→desired→observed, lifecycle cascade, private vs `observed:` public); split observed-lift (derived/value-recurrent OK; host-fed recurrent[N] stream rejected); two-channel host-write count preserved | C1 |
| §13.19.14 L22027-30 + §13.14.7 L18713-16 + diagnostic L22116-23 | ⚛A: the three SPEC sites get the registration conditional (B.2 text) | C1, EC5 |
| §3.1 (traits) | Effect-kind trait methods; `observed:` contract blocks (minimum semantics; trait projection); waiver conditions; collision namespace; 005-51 reversal | C2 |
| §15.4.1 | fold cell kind (enum + payload); `parameter_bindings` provenance MARKER per 033-234 precedent (R8); wake gates + Gated arm gate_parents lower to existing gate objects; `observed_cell_ids` may include computed cells; six-primitives text at L23041 byte-identical (EC2) | R6, R8, C16, EC2 |
| §13.6 | Connection endpoint deriveds carve-out (`derived target: WeakHandle[Clip] = handle to`); named incoming connection-view reads (`value.mods`); acceptance intrinsic/meaning extrinsic | C16, C17 |
| §13.5 | repeat expression position; typed binder `repeat (child: Bound)`; carried keys over dynamic views; Keyed/StringifiableKey language-defined framing | C7, C18 |
| §13.10/§13.12 | 023-46/025-6 twin rewrite ripple (WeakHandle AND Portal as the two dynamic-dependency sources) | C18 |
| §13.13/§13.15/§15.4.1.1 | Interpreter-placed effect paths mirror node paths; `:N` ordinals; hot-reload id construction extended | C18 |
| §2.3.3 | Polymorphic-recursion carve-out for compile-time interpretation expansion (bounded by finite closure); permanence claim scoped to runtime dispatch | C3 |
| operator-terminology note (§4.9/§13.17 preamble) | One note distinguishing: operator application `|>` (001-34) / operator traits `Add::add` (007-187) / the `operator` construct (029) | C18 |

## D.7 New SPEC sections

| Section | Content | Refs |
|---|---|---|
| §13.20 (collect/yield) | Full construct spec mirroring 034-1..13: forms, membership semantics, R4 legality, yielded kind, Iterable fulfillment, count member | C8, C9 |
| §13.21 (fold) | Full form spec mirroring 035-1..10: grammar, semigroup+else semantics, cost rule, scope, IR note | C10, R6 |

---

# PART E — EXECUTION ORDER

All edits are applied to DECISION_LOG.md first (Waves 1–7), then SPEC.md (Wave 8), then
verification (Wave 9). Within a wave, sections are edited high-to-low entry id so
renumbering never invalidates pending anchors.

- **Wave 1 — protects pinned.** Record byte-hashes of every B.1 HARD row (017-244,
  033-56, SPEC L23041) and the SOFT-row texts; these hashes gate every later wave.
- **Wave 2 — deletions.** 016-178, 029-108, 030-123/128/129/130. Renumber tails; update
  the wave-local id map.
- **Wave 3 — atomic groups.** ⚛A at the canonical 6 core LOG sites (015-39, 027-80,
  027-81, 031-128, 031-143, 033-122) + support sites (031-119; 031-157 lands with Wave
  5's 031 ADDs — its conditional text is staged here) + companion 015-36; ⚛B six LOG
  entries; ⚛D 030-255/259/262. One commit per group.
- **Wave 4 — rewrites/splits/extends.** All remaining RW/SPL/EXT/CON/RES rows of Part B,
  section by section (001→033), high-to-low within each section.
- **Wave 5 — ADDs.** All ADD rows: 005 (+4), 008 (+1), 009 (+1), 012 (+1), 016 (+3
  gross: 016-9+1, 016-176+1/+2), 018 (+3), 019 (+1), 022 (+1), 030 (+2), 031 (+5), 033
  (+2). Renumber tails per INV1.
- **Wave 6 — section 017.** The exact 18-insert schedule of B §017, in cluster order
  6→5→4→3→2→1 (highest anchor first so earlier anchors stay valid): A16–A18 after
  017-255, A9–A15 after 017-210, A8 after 017-204, A5–A7 after 017-193, A3–A4 after
  017-114, A1–A2 after 017-90. Then the 017 RW/SPL/PR rows. Final count MUST equal 326.
  (No deferral: the schedule above is the pin.)
- **Wave 7 — new sections.** Append 034 (13 entries) and 035 (10 entries).
- **Wave 8 — SPEC.** Part D in order D.1→D.7; ⚛A SPEC sites and ⚛C land as single
  commits; ⚛B/⚛D SPEC sides land in the same commit as their Wave-3 LOG halves ONLY if
  the repo process allows cross-file commits — otherwise immediately after, flagged
  pending until closed.
- **Wave 9 — acceptance.** Run Part F fully. E.3-style spot checklist: every Part B row
  ticked; every ADD present; renumber roll-up B.3 reproduced by fresh `grep -c`.
- **E.5 — protect verification**: re-hash every Protect Register row (B.1); HARD rows
  byte-identical, SOFT rows meaning-reviewed. (This replaces v1's inline protect list.)

---

# PART F — ACCEPTANCE CHECKLIST

- **F.1** Every C1–C18 clause of A.2 maps to ≥1 executed Part B/C/D row (trace table
  produced during Wave 9).
- **F.2** Every Protect Register row (B.1) passes its grade check (HARD: byte-identical
  hash; SOFT: no meaning change). References B.1 only — no second list exists.
- **F.3** R1: `accumulate` exists; stream-operator `fold` gone; `scan` untouched
  semantically. R2: `event_count` exists; bare stream `count` operator gone.
- **F.4** R3: 029-43 successor names neither `to_ring_stream`, `to_gate_stream`, nor
  `changes` as its example operator.
- **F.5** `grep -n "to_ring_stream\|to_gate_stream"` over LOG and SPEC returns ZERO hits.
- **F.6** `grep -n "Monoid"` over LOG and SPEC returns zero hits in normative text.
- **F.7** ⚛A: all 6 core LOG sites + 031-119 + 031-157 + all 3 SPEC sites carry the
  conditional; `grep -c` for the conditional's key phrase ("host-written channels")
  equals the expected site count.
- **F.8** ⚛B: no entry or SPEC line states `Cell[T]` is a type OR a trait; the kind
  statement appears at all six LOG sites + §13.2.8 + §13.18.5.
- **F.9** ⚛C: the four §13.1 claims match D.1's rewrites; none of the old four texts
  remain.
- **F.10** ⚛D: `RecurrentRing`/`RecurrentGate` appear only as sugar-alias mentions (if
  at all), never as StreamPolicy members.
- **F.11** Map/HashSet: every former co-occurrence entry (018-39, 018-83, 018-139,
  012-106/111 context) shows a SPLIT — HashSet still described unordered;
  `grep -n "unordered" `on Map-describing lines returns zero.
- **F.12** `.length` survives nowhere as the collection tally (`grep -nw "\.length"`
  hits only string/stream-exempt or historical-note contexts); `.count` present on
  arrays/slices/bundles/Map/Vec/HashSet/Yielded/dynamic views.
- **F.13** Dangling-id sweep: `grep -nE "\b0[0-9]{2}-[0-9]+\b"` over LOG (pattern
  assumes section ids 001–099; sections top out at 035, so the pattern over-covers
  safely) — every match resolves to an existing post-renumber entry; INV2 check: no
  entry text references another entry id.
- **F.14** INV3: every new/rewritten entry carries a `(§…)` SPEC ref; new 034/035
  entries point into §13.20/§13.21.
- **F.15** 029-74 LOG text still contains "RHS"; SPEC L19526-successor still lacks it.
- **F.16** R11: exactly two `else` senses documented (loop 014-88, fold 035-4); 022-101
  fallbacks still `otherwise:`/`default:`.
- **F.17** EC1: 017-244 hash unchanged (via B.1).
- **F.18** R4: 034-7 present; 022-5 text unchanged.
- **F.19** R7: no `[B ...]` pattern anywhere; F.19b: no `T[]` synonym.
- **F.20** EC2: 033-56 and SPEC L23041-successor hashes unchanged (via B.1); kind enum
  at 033-80 lists exactly input|derived|recurrent|fold.
- **F.21** W2(a): none of 017-74/125/190/282/306/307 mention Yielded.
- **F.22** Section counts (fully mechanical): `grep -c "^SSS-"` per section equals B.3's
  Final column exactly — 017 = **326**, 030 = 260, 031 = 158, 016 = 285, 005 = 238,
  033 = 236, 029 = 124, 018 = 143, 012 = 188, 009 = 123, 008 = 74, 019 = 79, 022 = 121,
  034 = 13, 035 = 10; all others unchanged per B.3. Grand total 4241.
- **F.23** Keyword growth: 002-3/4/6/9 list `collect`/`yield`/`fold`/`by`/`cell`/
  `yielded` among them; 020-23's reserved-set statement matches.
- **F.24** ⚛A grep keys off the FIXED 6-site core list (015-39, 027-80, 027-81,
  031-128, 031-143, 033-122 — post-renumber ids resolved via the Wave-2/5 id map).
- **F.25** Anchor-vs-post-renumber footnote: all Part B anchors are PRE-edit ids/lines;
  Wave 9 checks run against POST-renumber ids using the id map maintained from Wave 2
  onward. Any check that cannot resolve an id via the map FAILS the run.

*End of amendment plan v2.*





