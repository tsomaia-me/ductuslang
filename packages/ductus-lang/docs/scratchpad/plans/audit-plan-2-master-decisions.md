# Audit Remediation — Part 2 of 3: Master decisions

*2026-07-07 · 26 decisions covering 126 findings. Read this document SECOND (approve Part 1's batch first or in parallel; Part 3 depends on this one).*

Each item below is one decision that settles a whole group of audit findings at once. They are ordered so that decisions which gate other decisions come first — the first three form a chain (what a cell is -> how many declaration kinds exist -> how the new yielded/fold constructs are implemented). Under each recommendation you'll find which findings resolve automatically and which leave a small named follow-up you can answer inline.

Every recommendation is a recommendation. Nothing here is decided until you decide it.

## Rulings ledger (2026-07-08, owner)

- D-01: RULED — Model B (lowercase kinds). Conform extends to GRAMMAR.md (it contradicts ruled canon). Residual defaults applied and disclosed: bracket type-hood entries REWORDED to kind claims (not deleted); 029-45 citation repointed to the operator-output section.
- D-02: RULED — adopt the SPEC's split (six declaration kinds; distinct cell-kind umbrella). Owner Q on recurrent stream answered: it is the stream declaration kind carrying the recurrent history modifier — named explicitly in the canonical membership line, not a seventh declaration kind. Residual RULED (2026-07-11): P026 — `yielded T` is admitted as the fourth operator-parameter kind (alongside cell-bound, value, function/operator-typed); executes in the stream/group-class batch.
- D-03: RULED — canonical module top-level set: signal, derived, recurrent, stream, const. No module-level let, no module-level attr.
- D-04: RULED — Model B group + Route 1 (fold as derived) + L1 (derived liveness) + leave-is-a-dirty-root. Residuals defaulted per item recommendation (walk-order-position repeat key; synthesized membership-descriptor cell) — owner may veto before Phase B executes.
- D-05: RULED — Model A (extend the closed lists). Residuals (class assignment for own/move/public/private; cell as reserved keyword; full-sweep scope) OPEN for owner before its phase.
- D-06: RULED — two-part policy (ancestor-umbrella valid; repoint by default, extend only when nothing elaborates).
- D-07: RULED — Option B (rider test). F131 consolidation BLOCKED on D-31 (grammar-doc status) since "no separate grammar document" is now factually false.
- D-08: RULED — Model B placement (all five hooks post-publish inside blocking commit()) + per-instance order teardown, create, resume, update. Residuals RULED (2026-07-11): F089 — the initial cohort's create fires as the first post-first-commit hook pass (no hook ever fires outside a commit boundary); F108 — the reload commit fires the ordinary reconciler tail (reload complete = external reality reconciled).
- D-09: RULED — Model A (WeakHandle dynamic/freezing; Handle static). Residual defaults per item recommendation (mislabel reading; enumeration third form rewritten WeakHandle).
- D-10: RULED — Model 2 (live membership; reactivity governs mid-pass change). F052 restated in existing vocabulary per recommendation.
- D-11: RULED — Option A (bare fence = Ductus; tag exceptions). Tag vocabulary defaulted to the minimal set (text, ductus-ir, ebnf, rust) — veto if you want other spellings.
- D-12: RULED — split: observed cells do NOT block the satisfies-waiver; they DO block auto-satisfaction. Residual RULED (2026-07-11): the precise rule — an observed contract blocks auto-satisfaction only when the default body does not itself expose every contract cell; a default body supplying the full observed block auto-satisfies (one compiler check: does the default body cover the contract?).
- D-13: RULED — Model B (effect methods excluded from ordinary dispatch, written as an explicit algorithm step). Residual defaults per item recommendation (stay in effective set for collision; three-call-forms carve-out).
- D-14: RULED — Model A (widen StringifiableKey to the full Map key set). Residual default: StringifiableKey and the Map key bound defined as the same set.
- D-15: SET ASIDE by owner — Model B is incoherent with the no-privileged-code rule (user-writable map could not consume the view). Needs redrafting; couples to the D-18/D-22 umbrella question.
- D-16: RULED — Model A (empty bundle = zero rows; offsets length = rows + 1). Bundle .count carved out as row-count exception per recommendation.
- D-17: PARTIALLY RULED — Option 1 (unify closure vocabulary) EXECUTES. Owner redesign captured, NOT executed here: no main placement, main keyword vanishes, unreachable instances are dead code (not a compile error), the root is the node the user starts traversal from, traversal happens inside Ductus. Needs its own amendment plan (see BACKLOG).
- D-18: RULED (2026-07-08, merged with D-22) — streams are their own kind, fully outside the cell umbrella; cell T never admits a stream; EXPLICIT stream<->cell conversion mechanisms remain the bridge (the owner's earlier phrasing meant an explicit-conversion requirement at those sites, not auto-conversion). The read-site-exclusion model is retired. Admission-rule and SPEC umbrella-section rework executes in a dedicated follow-up batch immediately after Phase A.
- D-19: RULED — DRAIN; complete = downstream-emission only. Sub-answers per item recommendation (merge emits until all arms complete; derived consumers see nothing special; observation cells never surface completion).
- D-20: RULED — Model B (match selects/unrolls; never freezes arms). Residuals RULED (2026-07-11): the surviving unroll/mount-tag lowering GETS a SPEC subsection (add it); the coined term 'interpretation context' is REPLACED with plain wording at both use sites (a match appearing inside reactive structure as it is built — exposition and placement bodies). RE-CONFIRMED by owner 2026-07-13 for the D-20/21/26/30 batch.
- D-21: RULED — Model B (member order as pivot; walk order and declared order as the per-source definitions). Walk-order homed in section 034 per recommendation; walk order explicitly distinct from exposition entry order. RE-CONFIRMED by owner 2026-07-13; scope note: the stream/group and D-04 phases added many new walk-order uses (018-37/74, 033-61, 034-11/12/14, SPEC repeat/keying text) — the defining entries must cover them; all are consistent with the pivot model.
- D-22: RULED — merged into D-18's stream ruling: streams (regular + recurrent) form one separate kind class. Resolved (2026-07-11): yielded evicted too — groups are their own kind class outside the umbrella (four-group taxonomy executed in Phase A).
- D-23: RULED — five-member closure (LOG side); SPEC conforms 3 to 5. Main-removal portion captured with D-17 in BACKLOG, not executed here.
- D-24: RULED — module granularity for the orphan rule.
- D-25: RULED — Option A, entry-alternative placement, applied in BOTH SPEC section 15.4.6 and IR_GRAMMAR.md.
- D-26: RULED — Model A (per-declaration desired cells). All three residuals CONFIRMED by owner 2026-07-13 (not vetoed): desired streams reuse the one stream cell form (D-25's landed production satisfies the dependency); the whole-record cell is retained as a reconciler-facing view assembled from the per-declaration cells, which own graph identity; intra-desired when/given lowers to per-arm cells behind their own gate, distinct from whole-effect suspend/resume, and the 'no activation input anywhere' claim narrows to whole-effect suspend/resume only.
- D-27: RULED — two sealed members (Ring[N], Gate[N]); a recurrent stream is not a policy. No recurrent alias names (residual resolved by the ruling's phrasing).
- D-28: RULED — Option B, owner framing: the immutability absolutes hold within the boundaries of the Ductus program; the program, within, is immutable. Scoping wording built from the owner's sentence.
- D-29: RULED — Option B with corrected rationale: Map is language-provided, stdlib is a separate library on top; the spec borrows Vec from stdlib's future spec for convenience. Constructor std::vec::new() per item recommendation — veto if another spelling is preferred.
- D-30: RULED (2026-07-11) — the operator identity rule is the single base: an effect instance's identity across reload = enclosing scope + effect type + argument bindings. The repeat element key enters through the scope term (each keyed repeat scope is the enclosing scope, so equal payloads under distinct keys stay distinct). Declaration paths are demoted to addressing only — derived after identity is decided, never the criterion. The positional-ordinal path rule retires. RE-CONFIRMED verbatim by owner 2026-07-13, plus F209 residual RULED: repoint — the cell-identity entry's effect clause repoints to the hot-reload-of-effects section that already elaborates the rule; interpreter-placed instances get one sentence (enclosing scope = the interpretation site; cell paths root there). Executes in the D-20/21/26/30 batch.
- D-31: RULED (2026-07-11) — option (b): IR_GRAMMAR.md is normative for the IR text form; GRAMMAR.md is a non-normative derived reformulation of already-normative rules. Precedence pinned: SPEC wins on conflict; grammar-doc divergence is a first-class defect (same contract as LOG<->SPEC). 001-3/002-29 rewrite to 'no separate NORMATIVE surface-grammar document'; unblocks D-07's F131 consolidation. Executes in the stream/group-class batch's grammar pass.

- KIND-POSITION: RULED (2026-07-11) — kind annotations are outermost-only (declaration/parameter/return); never inside a type constructor; sole exception Portal[cell T] (a cell designation, not a type). 017-189's Vec[cell ...] case is retired as ill-formed.

- CONSTANT-WRAP: RULED (post-Phase-A) — the static-value wrapper is a degenerate `derived T` (a nullary computation; no host-write carve-out). SPEC §13.17.3's 'compile-time-fixed signal T' phrasing is the defect side. Executes in the stream/group-class batch.
- §030-RESPELLS: RATIFIED — d01-c's 16 in-section value-cell conforms stand as D-01 work.
- NORMALIZATION: RULED — full sweep (Option A): kind-noun lowercasing in prose, typed→annotated where a kind is meant, and the informal SPEC:~23354 phrasing; rides the stream/group-class batch.
- D-15: RULED — Option 2, owner-grounded: a dynamic view cannot satisfy Iterable at all (its membership varies; Iterable's frozen-source contract has no mechanism for that). A view is wiring, consumed only by structural constructs: `repeat` materializes members; `collect`→`yielded` is the bridge into value-land. The Iterable/IntoIterable claims on views are retired; no `for`, no view-consuming operators; no privilege — constructs are syntax, and the no-privileged-code rule constrains library code.
- NEW OPEN (from D-15's grounding): D-10 tension — 'yielded fulfills Iterable' has the same membership-varying hole; owner to rule whether groups keep D-10's reactive-re-run carve-out or lose Iterable like views did. [Resolved: see GROUP-SNAPSHOT below — Iterable retires.]
- NEW OPEN: the declaration form of a named group — body statement `collect as x:` (current law) vs an annotated `yielded T x = collect:` form (requires amending the 'never a leading yielded' clause).
- MAIN-REMOVAL seeding: host-naming REJECTED by owner (the host must stay topology-blind; the runtime is fully general-purpose; users never write host code). Candidates: keep `main` (settle) vs a language-level `@root` directive on a top-level placement. Orchestrator recommends `@root`. Q2/Q3 derivative under the directive model.

- YIELDED-DECL-SYNTAX: RULED (2026-07-11) — a named group declares in house style: `yielded <name>: <MemberType> = collect:` + indented body (precedent: observe-as-RHS of a reactive declaration). Consequences: `yielded` becomes the SEVENTH declaration keyword (016-1 and the SPEC §13.2 header re-amend six→seven; `yielded` joins 002-3's declaration list via D-05's extension); group declarations are BODY-ONLY (module top-level five-kind set unchanged); the class taxonomy is untouched (groups remain their own class outside the cell umbrella). Sub-question RULED (2026-07-11): the `collect as x:` statement form is RETIRED; the annotated declaration `yielded <name>: <T> = collect:` is the single spelling.
- ROOT-DESIGNATION: RULED (2026-07-11) — the `main` keyword retires; a traversal root is designated in-language by the `@root` directive applied to a top-level placement. Host stays topology-blind; runtime fully general-purpose. Derivatives (defaults pending confirm in the amendment plan): multiple `@root` placements allowed, each rooting a closure, liveness = union; the live closure is statically computable again; unreachable instances are dead code surfaced as a lint, never a compile error.

- COLLECT-AS: RETIRED (2026-07-11) — single spelling per construct: named groups declare only as `yielded <name>: <MemberType> = collect:`; the `collect as x:` statement form retires (002-3's clause and all uses conform).
- D-10 Iterable follow-up: SUPERSEDED (2026-07-11) by GROUP-SNAPSHOT below — groups do NOT keep Iterable; the capability principle is honored by the auto-materialized snapshot instead. Historical record of the interim confirmation: groups KEEP their Iterable fulfillment. Settled by the owner's capability-vs-performance principle: fold's privilege is the maintained O(log n) combine tree (performance only); the same reduction must remain expressible in user code (a fn taking `yielded T`, iterating with `for`, recomputing in full). The Iterable law is refined rather than excepted: Iterable requires a within-pass-stable source of VALUES — groups qualify (members are values; pass atomicity + membership-change-as-reactive-update per D-10), dynamic views do not (members are wiring/entities, not values). D-15 unchanged.
- PROPOSED for owner nod (new 001 philosophy entry, future batch): "Language constructs may be privileged in performance, never in capability: any construct's observable result must be expressible, possibly less efficiently, in user-written code."

- GROUP-SNAPSHOT: RULED (2026-07-11, owner design) — no `.members()` method; a yielded group participates in the existing reactive-transparency machinery. In a VALUE position (a `T[..]`-shaped parameter, an arithmetic operand, a value-typed let/derived RHS), a group auto-materializes to a fresh read-only sequence of its current member values in walk order — the group parallel of value-cell auto-deref; the consumer re-evaluates whenever the group changes. In WIRING positions (the fold members operand, a repeat source, a `yielded T` parameter), the group passes as wiring — no materialization. Groups are NOT Iterable and never will be (D-10's Iterable fulfillment retires); `for` is legal only over the materialized snapshot value. The one law unifying all of this with the stream ruling: implicit wiring-to-value reads exist exactly where the value form is a LOSSLESS snapshot of current state (a cell's current value; a group's current members) and never where information would drop (a stream's events — explicit conversion stays mandatory). Defaults to confirm at execution: the canonical snapshot spelling (read-only `T[..]`-shaped sequence); `derived xs = gains` legality as a collection-valued derived; a cost-model note (snapshot consumers are O(n) per re-evaluation; `fold` is the O(log n) incremental form). Dynamic views stay structural-only (their members are instance references, not values; `repeat`/`collect` remain their surface).

- D-04 POST-PHASE CONFIRMS (owner, 2026-07-13): (1) descriptor kind spelling CONFIRMED — the membership-descriptor renders as a `derived`-kind cell typed `yielded<T>` with NO `uses` clause (structural evaluation; the grammar states the irregularity rather than minting a phantom behavior). (2) `keyed by` over a `yielded` group source RULED — compile error, named normative diagnostic class `keyed_by_on_yielded_group_rejected` (group members are keyed structurally; a written `keyed by` has nothing to key; precedent: the named rejection classes for unordered positional iteration and bundle-in-repeat). (3) FLOAT in the IR `value` production CONFIRMED kept (aligns `value` with the same grammar's `literal`; fold `else: 0.0` requires it). (4) the walk-order-position pin CONFIRMED by owner 2026-07-13 ('birth-certificate tag'): the walk-order-position pin — per-member scope identity = the structural coordinate (yield-site index, plus the producing repetition key for repeat-born members), never a flat ordinal; walk order = the per-commit sequence over live members (sites lexical, repeat members in key order) used by the fold tree and snapshots; the landed 018-37/033-61 "walk-order position" wording respells to the coordinate once confirmed.

- POST-BATCH RULINGS (owner, 2026-07-13): (1) COORDINATE-PATH: RULED (a) — a group member's scope path NESTS: the yield-site is its own path level (declaration-order index within the collect body, mirroring the anonymous-placement ordinal convention), and the producing repetition key sits under it as an ordinary key component (mixer.parts.1; mixer.parts.2.voice_7); each component stays a simple stringifiable value, so the existing scope-key rules hold unchanged; known accepted property: inserting a yield line above shifts later site numbers across a reload, like anonymous ordinals today. (2) 023-53: RULED (a) — rewrite in place in bridge vocabulary, keeping the two-kinds-of-dependency content (membership-cell edge + per-member-scope read edges, carried by repeat/collect, never by operators). (3) STREAM-SPELLING LINE: NOT ratified — owner challenged the "policy family = genuine types" framing on two grounds: (i) "stdlib" is a category error, these are LANGUAGE-BUILTIN types (never call language builtins stdlib; 030-261's own "stdlib provides" wording is a defect to conform); (ii) the retirement rationale review CONCLUDED and owner RULED (2026-07-13): option (a) — FINISH THE RETIREMENT. Stream[T, P], RingStream[T, N], GateStream[T, N] retire as types (a stream is wiring, same as derived and yielded; no cell of any class gets a type spelling). The policy-generic kind spelling is RULED: `stream[P] T` ("stream[P] T is the way, yes"). Ring[N] and Gate[N] remain language-provided const-generic marker types fulfilling the sealed trait StreamPolicy (per the existing sealed-trait and policy-type entries); `P: StreamPolicy` stays an ordinary trait bound, and the amendment must state that kind brackets admit parameters (const ints; policy types bounded by StreamPolicy) — distinct from the banned direction (kinds inside type constructors). Also: never "stdlib" for language builtins — 030-261's "The stdlib provides…" wording conforms to language-provided in the same amendment. Sub-choice RULED (2026-07-13): (a) — `stream[Ring[64]] f32` is LEGAL in concrete positions; the word form `stream ring[64] f32` is the idiomatic sugar for it. SEALED-DEMAGIC: RULED (owner, 2026-07-13, all four pins confirmed + fold into the retirement amendment): `sealed` is a USER-WRITABLE trait modifier — no language privilege. (1) The module unit is the same module unit as the orphan rule. (2) Sealing bars BOTH doors: `fulfill` blocks and bare `satisfies` marker claims outside the trait's declaring module. (3) Stated relation to the orphan rule: sealing disables the type-ownership arm — trait-module only. (4) `sealed` becomes a reserved word on the trait declaration (keyword-class assignment rides D-05's phase); diagnostic class `sealed_trait_fulfillment_outside_module`. Into/TryInto/StreamPolicy become ordinary sealed traits declared in the language core module. DOC REQUIREMENTS (owner): the sealed-traits SPEC section references the modules chapter (Ductus module semantics differ from e.g. JavaScript's — enable the jump); every sealed-trait site (Into, TryInto, StreamPolicy) carries a quick-reference pointer to the sealed-traits section.

- WIRING-TYPES vocabulary (owner clarification, 2026-07-13): the kind KEYWORD alone (`stream`, `derived`, `cell`, ...) is a kind and a keyword; the applied annotation (`stream T`, `derived T`, `stream[P] T`, `cell T`) is a TYPE — part of the type system, in a distinct class: wiring types, unstorable by nature, expressing wiring rather than values. Brackets-name-types stands; the refinement is that wiring annotations are types too, just never value types. CORPUS TENSION: owner ruled FOLD IT (2026-07-13) — the two-level rewording (kind keyword vs wiring-type annotation) rides the retirement amendment. Affected: entries phrasing the applied annotation itself as "a kind annotation, not a type" (the 016-62/163/166/178 family and SPEC §13.2.8's kind-not-type sentences) conflate the keyword level with the annotation level and need the two-level split.

- POST-RETIREMENT RULINGS (owner, 2026-07-15): (1) stream operator library (map, filter, merge, ...) = STDLIB, confirmed; to_signal RULED STDLIB too (owner, 2026-07-15): it is user-expressible via the `observe` construct — an observe arm takes a stream trigger with an `as` event binder, and an observe expression's concrete kind (value cell or stream) is determined by use context, so `observe: / on source as e: e / default: fallback` IS to_signal; the crossing primitive is the observe CONSTRUCT (syntax may be privileged; the no-privilege doctrine binds library code), and the loss law's naming of to_signal names the canonical stdlib spelling. FLAGGED GAP for the stream phase (D-19 neighborhood): whether an observe arm with a stream trigger evaluates once per EVENT or once per COMMIT is unpinned — irrelevant to to_signal (latest-event suffices) but load-bearing for accumulate's every-event fold. (2) Declaration heads ADMIT the bracket form: `stream[Ring[64]] events: f32 = source` is legal; GRAMMAR's StreamDecl and RequiredCellKind productions gain the bracket alternative. (3) SPEC §7.5's "provided by stdlib" for the built-in numeric From/TryFrom impls REWORDED to language-provided (the orphan rule forces it: stdlib declares neither From nor the numeric types). Items (2) and (3) EXECUTED (2026-07-15) and class-extended: the bracket-policy alternative landed in ALL eight stream production sites (GRAMMAR StreamDecl, RequiredCellKind, RecurrentDecl's recurrent-stream head, DesiredCellDecl's two stream heads, ObservedCellDecl, plus the pre-existing KindAnnotation pair) AND SPEC §3.1's RequiredCell (restoring the §3.1<->§7.11 mirror); §7.5 reworded to language-provided with the orphan-rule grounding; the word-form-only / policy-keyword prose conformed at §13.18.2, §13.19.4, §13.18.16 (error text + block header), GRAMMAR §7.15 + node-body comments. 'policy keyword' = 0 in SPEC. Lesson applied: the ruling was a CLASS; the first follow-up brief under-scoped it to two named sites and the editors' flags completed the class.

Execution phasing: Phase A (this batch) = D-01, D-02, D-03. Immediately next: the D-18/D-22 stream-class batch (admission rules, SPEC umbrella sections, conversion surface). Later phases proceed in dependency order as open residuals resolve.

#### D-01 — Is a cell written as the lowercase kind `cell T` or the bracket type `Cell[T]`? *(resolves 22 findings, 19 automatically)*

*Revision 3 (2026-07-11): scope pinned corpus-wide — the class-based sweep covers ALL LOG sections (a spot-verify found unassigned residue in 012, 018, 027, 028, 035). The STREAM-FAMILY spellings (Stream[T], Stream[T,P], RingStream[...], GateStream[...]) are explicitly DEFERRED to the stream-class batch and count as intentional in D-01's verification grep, alongside Handle/WeakHandle/Portal[T]/Type[C]/Map/Vec. GRAMMAR.md is deferred to the grammar pass.*

*Revision 2 (2026-07-08): a second halt found 016-173 also missing. The conform list is hereby a FLOOR, not a ceiling: the Model-B ruling determines the fix for EVERY entry carrying a model-A cell-bracket spelling; editors conform all such entries in their assigned sections whether listed or not, and the zero-residue verification grep is the authoritative completion criterion.*

*Revision (2026-07-08): a protocol halt during execution found three entries the conform list missed — 016-168, 016-169, 016-170 (the Signal[T]/Derived[T]/Recurrent[T,N] type-hood run in the same §13.2.8 region). Same defect class, fix fully determined by the Model-B ruling; added to the list.*

> **Post-merge status (2026-07-08, corpus at 1f204c473a):** What changed: the grammar session created a new document, GRAMMAR.md, that reintroduces the model-A bracket spelling D-01 wants retired. GRAMMAR.md:6049 lists `Cell[T]` as "The umbrella reactive-cell type; concrete kinds (signal/derived/recurrent/stream) refine it" — directly contradicting SPEC §13.2.8's "there is no `Cell[T]` type," and it also uses `Cell[T]`/`Derived[f32]`/`Cell[Map[...]]` in examples (GRAMMAR.md lines 3742, 3746, 3749, 3887, 5511, 6097). D-01's factual claim that "the LOG never got the same pass, and that is the whole conflict" is therefore no longer complete: model-A now also lives in a second, newly-created corpus document that is NOT in D-01's conform-list. What still stands: everything else. SPEC is still fully self-consistent on model B (verified "no `Cell[T]` type", "cell is a KIND ... never written with brackets", type-vs-trait "neither is correct"), and all 46 listed LOG ids still carry model-A spellings at their stated ids (030-48 still matches by content despite section 030's +1 renumbering). The recommendation (model B) and the entire conform-list are unaffected. What remains for the owner to decide: whether to extend the model-B conforming pass to GRAMMAR.md (at minimum line 6049's "umbrella reactive-cell type" claim, plus the example occurrences), and whether GRAMMAR.md is normative enough that its model-A usage counts as a defect on par with the LOG entries.


In Ductus, every reactive value lives in a cell: signals, deriveds, recurrents, streams, and yielded groups are all cells. Code has to name a cell in two places — when a parameter or return receives a cell, and when the taxonomy lists the cell forms. There are two rival spellings for that name, and the docs carry both at once.

Model A — bracket type. A cell is a first-class generic type, spelled with brackets like any other type:

```
operator passthrough[T](source: Cell[T]) -> Cell[T]
fn some_fn(s: Cell[T])          # binds the cell reference
signal X = init                 # declares a Signal[T]
```

Here `Cell[T]`, `Signal[T]`, `Derived[T]`, `Recurrent[T, N]`, `Stream[T]` are real types usable in parameter, return, and generic-argument positions.

Model B — lowercase kind. A cell is not a type at all; it is a KIND — a lowercase keyword that never takes brackets:

```
operator passthrough[T](source: cell T) -> cell T
fn some_fn(s: cell T)           # binds the cell reference
signal X = init                 # declares a signal T
```

Here `cell T` is an umbrella spanning `signal T`, `derived T`, `recurrent[N] T`, `stream T`, `yielded T`. There is no `Cell[T]` type and no `Cell` trait to implement. The two models also differ in one non-cosmetic spot — the recurrent history depth. Model A writes `Recurrent[T, N]` (depth after the value type, both bracketed); model B writes `recurrent[N] T` (depth bracketed, before the value type). An implementer cannot guess where `N` sits from one form given the other.

The amendment moved the SPEC wholesale to model B. The section that defines value-cell kinds now says outright that `cell` is a KIND not a type or trait, that it is never written with brackets, and that there is no `Cell[T]` type and no `Cell` trait to fulfill. A later SPEC passage explicitly records that the old type-vs-trait framing was wrong and retired it, and the taxonomy table even lists `dynamic view V` as "replacing" the old bracket spelling `Cell[DynamicView[WeakHandle[V]]]`. So the SPEC is internally consistent on model B and documents the switch as deliberate.

The LOG never got the same pass, and that is the whole conflict. Dozens of LOG entries still spell cells the model-A way: operator/function parameters typed `Cell[T]`, value cells declared as `Signal[T]`/`Derived[T]`/`Recurrent[T, N]`, the portal carrier written `Portal[Cell[T]]`, minted bindings written `Cell[Map[K,V]]`/`Cell[DynamicView[T]]`, and one entry even calling `Signal[T]` "a first-class type." Worse, the LOG contradicts *itself*: within one section it both says "a cell is refused as a value" and named "by a lowercase kind annotation" (model B) and, two entries away, types the same parameter `Cell[T]` and calls `Signal[T]` a first-class type (model A). One entry writes `Portal[Cell[T]]` in the very sentence that says binding machinery is spelled as lowercase kinds — it breaks its own rule mid-sentence.

**Recommended: model B (the lowercase kind `cell T`), because the amendment already chose it deliberately, the SPEC is fully self-consistent on it, and the SPEC records the bracket type-vs-trait framing as the older *incorrect* one and marks the bracket spellings "replaced." Adopting B means conforming the LOG to the SPEC; adopting A would mean reverting a deliberate, documented amendment across the entire cell type system — the more expensive and less-grounded direction.**

Consequences once model B is canonical:

- **Auto-conforming (19 findings).** Every finding that is "LOG says `Cell[T]`/`Signal[T]`/`Recurrent[T,N]`, SPEC says the lowercase kind" resolves by rewriting the LOG entry to the kind spelling. That covers the umbrella and parameter entries, the `Portal[Cell[T]]` carrier, the minted `Cell[Map]`/`Cell[DynamicView]` bindings, the dynamic-view `Cell[DynamicView[WeakHandle[T]]]` occurrences across the view section, the operator-section (029) bracket types, the recurrent depth-order spelling, the stream/derived definitional restatements, and the two internal §016 contradictions (they collapse the moment the bracket entries are rewritten to kinds). No residual sub-decision — the SPEC already fixes the target spelling.
- **One caveat inside the auto set — the "value cell only" outlier.** Separately from spelling, one parameter entry restricts an operator value parameter to *value* cells, while two other entries (and the SPEC) admit *any* cell and exclude a stream only at the read site. Rewriting spelling does not fix that; it is a distinct behavioral reconciliation tracked under the operator/parameter decision, not here. Flag so it is not assumed closed by this rename.
- **STILL — three findings carry a residual after the spelling is picked:**
  - **§016 identity-model cleanup (which bracket entries to delete vs reword):** picking model B retires the bracket entries, but a few assert type-hood claims with no kind-form restatement (e.g. "`Signal[T]` is a first-class type," the `Derived[T]`/`Recurrent[T,N]` "is the type produced by…" entries). Decide inline: **reword each to the kind claim** ("`signal T` is a first-class kind," "`derived` produces the kind `derived T`") **or delete as redundant** with the umbrella entry?
  - **first-class-type vs refused-as-value survivor:** the entry saying `Signal[T]` is a first-class *type* directly fights the entry saying a cell is refused as a value. Under model B the first must be reworded to "first-class *kind*." Confirm inline: **keep it reworded as a kind claim, or drop it** since the umbrella entry already states kinds are usable in parameter/return/generic positions?
  - **the ≅-wrap citation (029-45):** this entry cites the value-cell-kinds section for a `Cell[T]`≅`T` wrap, but that section contains neither the ≅ symbol nor any `Cell[T]`; the wrap language actually lives in the operator-output section written `cell T`≅`T`. Spelling fix alone leaves a wrong citation. Decide inline: **repoint the citation to the operator-output section that actually states the wrap** (and rewrite to `cell T`≅`T` / `derived f32`)?
- No plausible-only members in this set; all 22 were verified CONFIRMED.

> **For execution:** Canonical = model B (lowercase kind `cell T`; no `Cell[T]`/`Signal[T]`/`Derived[T]`/`Recurrent[T,N]`/`Stream[T]` types). SPEC basis unchanged and authoritative: SPEC.md §13.2.8 lines 12536-12542 (kind, never bracketed), 12594-12598 ("no `Cell[T]` type"), §13.2.8.1 lines 12649-12669 (taxonomy incl. `Portal[cell T]`, `dynamic view V` replaces bracket), §13.18.5 lines 20713-20717 (older type-vs-trait framing "neither is correct"); operator kind usage §13.17.4 (e.g. SPEC.md:19924 `cell T`≅`T`). Conform these LOG ids to the kind spelling: 013-10; 016-62, 016-63, 016-165, 016-167, 016-171, 016-172, 016-175, 016-179, 016-180, 016-246, 016-266, 016-270, 016-279, 016-280, 016-281, 016-282, 016-283; 017-23, 017-53, 017-71, 017-125, 017-193, 017-194, 017-215; 025-13, 025-59; 029-2, 029-6, 029-11, 029-12, 029-18, 029-20, 029-22, 029-31, 029-41, 029-45, 029-69, 029-75, 029-112, 029-122, 029-124; 030-48; 031-19, 031-25. Recurrent depth spelling `Recurrent[T,N]` -> `recurrent[N] T` (F158: 029-11, 029-69). Portal carrier `Portal[Cell[T]]` -> `Portal[cell T]`, `Option[&Cell[T]]` -> `Option[&cell T]` (F195/F159/F151: 016-179, 017-215). Findings resolved: F194, F195, F196, F197, F198, F183, F184, F187, F221, F059, F061, F146, F148, F150, F156, F158, F159, F149 auto; STILL residuals in F062 (delete-vs-reword bracket entries in §016), F147 (first-class-type entry 016-167 reword/drop), F157 (029-45 citation repoint to operator-output §13.17.x). Verification greps: after edits, `grep -nE 'Cell\[|Signal\[|Derived\[|Recurrent\[|Stream\[' DECISION_LOG.md` should return only intentional non-cell brackets (e.g. `Handle[T]`, `Portal[...]`, `Type[C]`, `Map[K,V]`, `WeakHandle`); `grep -n 'Portal\[Cell\[T\]\]\|Cell\[DynamicView' DECISION_LOG.md` should return zero; confirm SPEC still says `grep -n 'no .Cell\[T\]. type' SPEC.md` (line 12598).

#### D-02 — What is the authoritative set of declaration kinds, and does `yielded` count as one? *(resolves 6 findings, 4 automatically)*

Ductus builds its reactive layer out of a small fixed set of "kinds." A kind answers "what did this declaration create and who drives it." The problem is that the corpus uses the word "kind" for two different sets and never keeps them apart, so the same section counts the kinds three different ways: seven, six, and five. An implementer building a rule like "you cannot assign to these" has no way to know which set the rule ranges over.

The two sets that are being confused:

*Declaration kinds* — the top-level forms you write with a leading keyword, one declaration each:
```
signal x: i32 = 0        // a declaration kind
attr  y: i32 = 0         // a declaration kind
derived z = x + y        // a declaration kind
recurrent[2] w: i32 = z  // a declaration kind
stream  s: Event         // a declaration kind
const   C = 5            // a declaration kind (not reactive at all)
```
That is six.

*Cell kinds* — the members of the `cell T` reactive-cell umbrella. This set is different: `const` is NOT in it (a const has no cell), and `yielded` IS in it:
```
collect:
  yield voice          // produces a `yielded T` group — a cell-KIND,
                       // but you never write `yielded x = ...` as a declaration
```
A `yielded` group is born from `collect:`/`yield` inside a block, not from a leading `yielded` keyword. It stores nothing, yet membership changes propagate dirt like a value change, so it is a genuine reactive cell.

So the honest answer is not one number. It is: **declaration kinds = six** (signal, attr, recurrent, derived, stream, const), and **cell kinds = a different set** (signal, attr-as-signal, derived, recurrent, stream, yielded — and NOT const).

**What the corpus currently says, and where it fights itself.** The taxonomy-header entry claims "exactly seven reactive declaration kinds" and lists signal, attr, recurrent, derived, const, stream, yielded — folding both `const` and `yielded` into one "declaration kind" list. The same entry then admits in its own text that the taxonomy "has two rows" that "are not the same list," which is the real distinction leaking through. Meanwhile the no-source-write rule says it "applies uniformly to all six declaration kinds," and the entry right before it enumerates only five by name (signal, attr, recurrent, derived, const — no stream, no yielded). Three counts, one section. The SPEC is already consistent on the six-model: its reactive-declarations header lists exactly six (signal, attr, recurrent, derived, stream, const) and omits yielded, while a separate section makes `yielded T` a full member of the `cell` umbrella. So the SPEC already draws the line this decision needs; only the LOG conflates.

Note this decision **depends on the cell-model decision** (whether cells are lowercase kinds or bracket types). Settle that first, because "cell kind" only means something once "a cell is a kind, not a `Cell[T]` type" is fixed. Everything below assumes the kind-model was chosen.

**Recommended: adopt the SPEC's split — six declaration kinds (signal, attr, recurrent, derived, stream, const), a distinct cell-kind umbrella that excludes const and includes yielded, and each rule states which set it ranges over — because the SPEC is already internally consistent on exactly this split, the header entry itself concedes "two rows that are not the same list," and `yielded` provably is not a declaration form (it comes from `collect`/`yield`, never a leading `yielded` keyword) while provably is a cell (membership changes propagate dirt).**

Consequences:

- **Conforms automatically (4):** The finding that flags three counts (seven/six/five) in one section is resolved the moment the header is re-scoped to six declaration kinds and the enumerations name the same six. The finding that `const` is called "reactive" in the seven-header yet declared non-reactive elsewhere is resolved by dropping `const` from the *cell*-kind set while keeping it a *declaration* kind — const is a declaration kind that is not a cell kind, which is exactly the split. The finding that the `cell T` umbrella membership disagrees on `yielded` is resolved by fixing the umbrella to include `yielded` (matching the SPEC's cell-umbrella text). The redundant three-time restatement of the umbrella membership collapses to that one authoritative list.
- **`cell T` umbrella membership — residual sub-decision (from the yielded-membership finding and its low-severity duplicate):** the umbrella must be pinned to one member list. Does `attr` appear by name, or only via "attr annotates as `signal T`"? And is `yielded` in? Recommended list: signal, attr(-as-signal), derived, recurrent, every stream cell, yielded group. **You decide the exact spelling of the canonical membership line.**
- **Closure-cannot-be-a-cell-value rule — residual sub-decision (STILL, from the finding that this rule enumerates four kinds and omits stream):** the rule names signal/attr/recurrent/derived and silently omits stream, while a blanket rule elsewhere forbids closures in any reactive cell. Under the six/cell-kind split the clean fix is "a closure cannot be the value type of any cell kind." **Do you want the enumerated entry broadened to "any reactive cell kind," or left enumerated with a pointer to the blanket rule?**
- **Operator-parameter kinds vs `yielded` — residual sub-decision (STILL, addendum finding):** the authoritative operator-parameter rule enumerates a *closed* set of parameter kinds (cell-bound, value, function/operator-typed) that does not admit `yielded T`, yet another section requires operators to accept `yielded T` parameters. Making `yielded` a first-class cell kind sharpens rather than dissolves this: a first-class cell kind that the operator-parameter enumeration still rejects. **Do you add `yielded T` as a fourth permitted operator-parameter kind, or narrow the yielded-consumer surface to drop operator parameters?** This one is a genuine cross-section design call, not mechanical.

> **For execution:** Canonical side = SPEC's six-declaration-kinds + distinct cell-kind umbrella (SPEC §13.2 header at SPEC.md:11707-11713; §13.2.8 cell umbrella; §13.20.4 at SPEC.md:23221-23226 makes `yielded` a cell-KIND). Depends on K-CELL-MODEL (must resolve kind-vs-bracket first). LOG to conform: 016-1 (DECISION_LOG.md:1860 — re-scope "seven reactive declaration kinds" header; separate the declaration-keyword row from the cell-kind row; the entry already flags "two rows … not the same list"); 016-162 (:2021 — "six declaration kinds", align cardinal + members); 016-156 (:2015 — five-name enumeration, reconcile with the six); 016-4/016-115/016-118 (:1863/:1974/:1977 — const is a declaration kind but not a cell kind, keep non-reactive); 016-62/016-163/016-166 (:1921/:2022/:2025 — collapse `cell T` umbrella to one membership list, include yielded, decide attr spelling); 013-199 (:1591 — broaden to "any reactive cell kind" or cross-ref 025-55 at :3158); 029-12 (:3397) / 034-11 (:4375) — resolve yielded-as-operator-param (add fourth param kind vs narrow consumer surface). Findings: F153 (H, main report L32), F155 (M, L397), F154 (M, L413), F066 (L, L520), F049 (L, L485 — STILL), P026 (M, addendum L490 — STILL). Verify: `grep -n 'seven reactive declaration\|six declaration kinds\|all six declaration' DECISION_LOG.md SPEC.md` should return one consistent story; `grep -n '`cell T` is the umbrella' DECISION_LOG.md` should yield one authoritative membership; confirm no LOG entry lists `const` among cell-umbrella members and no entry omits `yielded` from it.

#### D-03 - What may a module's top level declare? *(resolves 2 findings, both automatically)*

> **Post-merge status (2026-07-08, corpus at 1f204c473a):** What changed: the grammar session added GRAMMAR.md section 7.15, a new normative surface grammar that already picks D-03's recommended set. Its ModuleReactiveDecl production admits exactly four reactive forms - signal / derived / recurrent / stream (anchored to LOG 003-31, 003-34) - plus TopLevelConstDecl for const. It explicitly bans module-level attr ("No attr at module level ... The ModuleReactiveDecl production has no 'attr' alternative") and provides no module-level let production (TopLevelDecl has no let alternative). So the surface grammar now normatively states the five-kind set the recommendation asks for. What still stands: the four target LOG entries (013-22, 013-23, 013-30, 020-9) and the split SPEC (11.2, 13.7.1) were NOT amended - they still contradict each other exactly as the item describes, and now also contradict GRAMMAR.md 7.15. The recommendation is unchanged and reinforced. What remains for the owner: (1) ratify the five-kind set (the grammar already commits to it, low risk); (2) note execution should also cross-reference GRAMMAR.md 7.15 as a fourth conforming surface - the exec block currently lists only LOG+SPEC targets; (3) the two residual sub-decisions (drop module-level let entirely; drop module-level attr) still need explicit owner sign-off since the entries and SPEC prose survive.


A Ductus module's top level is the outermost scope in a file, above any node or connection body. The question is exactly which declaration kinds are legal there: which reactive cells, whether an ordinary `let` value binding, and whether `attr` (a placement-written cell) belongs. Two LOG entries try to enumerate this set completely, and they disagree with each other and with two other entries. Because this set feeds the module-scope grammar and the scope-resolution chain, an implementer needs one closed list. **Decide the cell-kind set first (the item on how many cell kinds exist and what they are), because the module-scope set is a subset of it: you can only pin what module scope holds once you know what a cell kind is.**

The corpus offers two rival enumerations, plus a separate fault about `let`.

The stricter, cell-only reading - module scope holds reactive cells and consts, nothing else:

```ductus
signal   master_gain: f32 = 1.0     // ok
derived  scaled = master_gain * 2   // ok
const    MAX = 128                  // ok
let x = compute()                   // ERROR: no let at module scope
attr method: string = "GET"         // ERROR: attr needs an enclosing node
```

The permissive reading - module scope also admits a top-level `let` value binding:

```ductus
let base_url = "https://api"        // legal top-level value binding
```

Here is where the four entries land, and how they fight:

- One entry says `let` is a function-body construct only and module scope contains **no** `let` bindings.
- Another entry, listing every scope in the resolution chain, says the module top level comprises `signal`, `derived`, `recurrent`, `stream`, `const`, **and `let`**.
- A third entry, listing what carries an initial value, names 'top-level `let`/`const`' as if a top-level `let` exists.
- The direct module-scope enumeration says module scope contains `const`, `signal`, `attr`, `derived`, `recurrent` - it includes `attr` but omits `stream` and `let`.

So three faults overlap on one subject. (1) Does module-level `let` exist? One entry says no; two others assume yes. (2) Is `attr` a module-scope kind? One enumeration includes it. (3) Is `stream` a module-scope kind? One enumeration omits it, the other includes it. The SPEC is itself split: its binding-forms section says in one sentence that a top-level `let` carries an initial value, then three lines later says module scope does not contain `let` bindings and lists `const/signal/attr/derived/recurrent`; its scope-chain section lists `signal/derived/recurrent/stream/const/let`.

**Recommended: the canonical set is `signal`, `derived`, `recurrent`, `stream`, `const` (five kinds). No module-level `let`, no module-level `attr`.** Three grounded reasons. First, `attr` is defined as a cell that is per-instance of its enclosing node or connection type and is written only at placement time; a module top level has no enclosing node and no placement, so a module-level `attr` is incoherent by attr's own definition - the enumeration that lists it is the wrong one. Second, `stream` is confirmed a module-scope kind elsewhere: an entry states the module-level push form targets a top-level, module-scope stream cell, so omitting `stream` from module scope is the drift. Third, module-level `let` has no grammar, no example, and no substantive rule behind it - only the two enumerations assume it, while a dedicated entry categorically bans it and `mut` is already forbidden at module top level; the two positive mentions read as restatement drift, not a designed feature.

Consequences:

- Both member findings conform automatically once the five-kind set is pinned. The module-level-`let` contradiction and the enumeration-drift finding are both just restatements of this one set; fixing the set fixes both.
- **Residual sub-decision (module-level `let`):** confirm you want top-level `let` gone entirely, not kept as a compile-time value alias. If you instead want a top-level value binding, the right spelling is already `const` (which module scope has), so a separate top-level `let` looks redundant - but say so explicitly so the two enumerations and the SPEC sentence get struck rather than kept.
- **Residual sub-decision (module-level `attr`):** confirm `attr` is dropped from module scope on the strength of its per-instance/placement-written definition. If you actually intend some module-level attribute concept, that is a new feature and needs its own entry, not a quiet inclusion in an enumeration.

> **For execution:** Canonical side = five-kind set {`signal`, `derived`, `recurrent`, `stream`, `const`}; no module-level `let`, no module-level `attr`. Conform these entries to state it identically: 013-22 (LOG line 1414, keep 'module scope contains no let bindings'), 013-23 (line 1415, change to `const, signal, derived, recurrent, stream`; drop `attr`, add `stream`), 013-30 (line 1422, strike 'top-level `let`' - keep `let, mut, const, signal`), 020-9 (line 2713, drop `let`; result `signal, derived, recurrent, stream, const`). SPEC: section 11.2 (SPEC.md lines 8320 and 8344-8347 - strike the 'top-level `let`' phrase at 8320, correct the module-scope enumeration to add `stream` and drop `attr`), section 13.7.1 (SPEC.md lines 16348-16349 - drop `let` from the module-top-level list). Supporting facts: attr definition 016-14 (line 1873, per-instance/placement-written); module-scope stream 027-89 (line 3273); mut-forbidden-at-module-top-level 013-19 (line 1411). Findings: P007 (HIGH, module-level let contradiction), P008 (MED, enumeration drift). Depends-on: K-KIND-COUNT (cell-kind set). Verify: grep -n 'module scope contains|module top-level scope comprises|Module scope contains' DECISION_LOG.md should show one identical five-kind set; grep -n 'top-level .let.|module-level .*let' DECISION_LOG.md SPEC.md should return zero module-level-let mentions.

#### D-04 — How do `yielded` groups and `fold` actually get built and run in the IR? *(resolves 20 findings, 16 automatically)*

Ductus lets you gather reactive values into a group and fold them. You write `collect:` with `yield` inside it, producing a `yielded T` — an ordered, membership-varying group of cells. Members join and leave as the program runs: a plain `yield` is a permanent member, a `yield` in a `repeat` is one member per key, a `yield` in a gated arm is present only while that arm is effectively active. A `fold ... by: ... else: ...` combines the members into one value. The surface story is settled. What is broken is the machine underneath: how a yielded group and a fold become IR objects, how they enter the per-commit dirty-set and evaluation DAG, when a fold recomputes, and what a "liveness bit" even means. The amendment gave the fold a partial IR home but left the yielded group with none, and it asserts behaviors ("membership propagates dirt") that no rule in the commit engine can actually carry out.

This decision **depends on two earlier ones and cannot be settled before them**: the decision on how many cell kinds exist and the exact kind enum, and the decision on the cell model (whether a cell is a value/type or only a kind). Those two fix the vocabulary this decision slots into.

**Question 1 — What IR object is a standalone `yielded` group?** Take a named group not consumed by a fold:

```
collect as parts:            # 'parts' is a yielded Voice
  yield a
  repeat v in voices: yield v
# consumed by: repeat p in parts, or fn f(g: yielded Voice)
```

- **Model A — yielded is its own IR cell kind.** The kind enum grows a fifth tag `yielded`; the group is a cell entry holding its member edges plus liveness bits. Membership change is dirt like any cell's.
- **Model B — yielded exists only absorbed inside a fold.** A `collect` fed to a fold contributes member edges to that fold's cell payload; no separate group object is emitted. A standalone group (fed to `repeat` or a `yielded T` parameter) still needs *some* IR object, so under Model B you must additionally define that object — a synthesized cell, or a membership descriptor threaded as an ABI value.

**Question 2 — How does a fold enter the dirty-set and DAG, and when does it recompute?** The commit engine's dirty-set is defined only over writable cells; dirt propagates to deriveds and recurrents. A membership change is not a value change of any referenced cell — it is a change in the *set* of contributing edges. Two shapes:

```
derived total = fold parts: by: (x,y)=>x+y else: 0.0
# 'parts' gains a member this commit (repeat key added, or gate opens).
# Must 'total' recompute THIS commit? What marks it dirty?
```

- **Route 1 — fold is scheduled as a derived.** The fold cell rides the existing derived dependency-edge list and DAG node set. Its member edges are treated as derived inputs; a member join/leave dirties it exactly like an input value change. No new engine machinery.
- **Route 2 — fold is a distinct scheduled node with its own edge list.** Add a fold-dependency-edge list parallel to the derived and recurrent ones, add fold cells to the DAG node enumeration, and define an explicit "membership change is a dirty root" rule naming the root (the source signal's dirt, the repeat re-key diff, or the gate transition).

**Question 3 — liveness (the on/off bit).** The amendment says a yielded group "reuses the gate on/off-bit machinery," but the gate layer has no on/off bit — gates are predicate objects composed by walking `gate_parent`; activation is *derived*, not *stored*. So "runtime on/off bits" has no referent, and it sits uneasily with "stores nothing."

- **Model L1 — liveness is derived, not stored.** A member's presence is re-read each commit from its guarding gate's effective activation. The group truly stores nothing; there is no bit array. "On/off bit" is deleted from the wording.
- **Model L2 — liveness is a stored bit-vector the group owns.** Then "stores nothing" is false for the membership structure, and the bit representation must be defined and named where gates can share it.

**Question 4 — the deactivating-commit fold value (the sharp contradiction).** When a gated arm flips true→false, one rule says the member-leave dirties the fold and it recomputes *this* commit without the member (value = a+c); another rule says gate-close adds no DAG work this commit, so the fold still reflects the member (value = a+b+c). Both are currently mandatory and give different concrete numbers. You must pick: **leave is a first-class dirty root** (fold drops the member the same commit, symmetric with the gate-open snap) **or the fold reflects the departed member until the next commit** (gate-close stays cost-free).

**What the corpus currently says, and where it fights itself.** The IR section gives `fold` a real cell kind (`input | derived | recurrent | fold`) and a payload (combiner id, else value, tagged member edges). But: the yielded group has no lowering to any of the six primitives and no IR type, so a `repeat` over a group or a `yielded T` parameter has no IR object or ABI signature. The kind enum has no `yielded` tag even though the group is called "a distinct cell-KIND." The dirty-set and DAG node rules list only deriveds and recurrents — a fold cell fits nowhere, yet is required to recompute on membership change. The text-form rule for kind-led cells omits `fold`. The repeat-source rule admits only `Signal[Iterable]` or a dynamic collection cell — a yielded group is neither, yet `repeat` over a group is declared legal. And the two fold-value rules above are jointly unsatisfiable.

**Recommended: Model B for the group + Route 1 (fold scheduled as a derived) + Model L1 (liveness derived) + leave-is-a-dirty-root for Question 4, because that set is the smallest change that makes the amendment's own claims executable.** Concretely: (a) The amendment already says a fold cell's surface *is* a `derived` and consumers see `derived T` — scheduling it as a derived DAG node (Route 1) is what that sentence already promises, and it needs no new edge-list or node-type in the commit engine. (b) The amendment already says a yielded group "stores nothing" and that gates compute activation by *walking* `gate_parent`, not by reading a bit — so liveness-is-derived (L1) is what the rest of the corpus already does; the "on/off bit" phrase is a stray term with no home and should be struck, not homed. (c) Making the member-leave a real dirty root (Question 4) is the only answer consistent with "membership change propagates dirt just as a value change does," which is stated three times; the competing "gate-close adds no DAG work" clause is the outlier and must be narrowed to say gate-close still dirties folds that have a gate-guarded member. (d) Model B keeps the kind enum at four tags (matching the enum the IR section states twice), so no `yielded` tag is invented; the standalone-group object becomes a defined sub-decision (below) rather than a fifth kind. Route 1 + L1 + B together also dissolve the repeat-source conflict cleanly: a group consumed by `repeat` lowers to a dynamic scope keyed by walk-order position, admitted as a third repeat-source category.

**Consequences.**

Conform automatically once B + Route 1 + L1 + leave-is-a-dirty-root are adopted and the wording is reconciled: F114 (deactivating-commit value now single-valued), F123 and F246 and F247 (fold rides the derived edge-list and DAG node set), F167 (fold member/combiner reads walked as derived reads, so fold-mediated cycles are detected), F124 (membership dirt root named: the gate transition / repeat re-key), F213 (no `yielded` enum tag needed under B), F211+F171+F115 (the "on/off bit" term deleted, replaced by derived effective-activation), F248 (text-form enum gains `fold`), F242 (lowering-table pointer to which surface form yields kind=fold), F093 and F095 and F096 and F099 (standalone-group IR object and `.count` cell defined as the group sub-decision below), F094+F097+F164 (repeat-source rule widened to admit a yielded group).

Residual sub-decisions the user must answer inline:

- **F098 — repeat-over-yielded keying identity.** Once `repeat` over a group is admitted, what keys the per-member scopes for mount/unmount diffing? Permanent and gate-guarded members carry no repetition key. *Recommend: walk-order position as the key.* Confirm or pick member-cell path.
- **Standalone-group IR object (covers F093/F095/F096).** Under Model B the group still needs one IR object for the `repeat` and `yielded T`-parameter paths. *Recommend: a synthesized membership-descriptor cell (member edges + derived liveness), added to the IR type vocabulary as `yielded<T>` for ABI purposes only.* Confirm this vs. threading it as a non-cell ABI aggregate.

Plausible members to dismiss-or-pin (verifier crux in one line):

- (none of the members are PLAUSIBLE-only; all listed are CONFIRMED. F098 is CONFIRMED but downstream of the repeat-source decision, hence flagged as a residual above rather than auto-conforming.)

> **For execution:** Canonical side after this decision = the fold-as-derived IR model (Route 1) + Model B group + derived liveness (L1) + leave-is-a-dirty-root. DEPENDS ON: K-KIND-COUNT (kind enum + count), K-CELL-MODEL (cell = kind vs value). LOG to conform: 034-9 (4373: strike "on/off bits"/"reuses gate on/off-bit machinery"; restate liveness as derived effective-activation), 034-10 (4374: keep "propagates dirt"; drop "distinct cell-KIND" implying a 5th enum tag under Model B), 034-11 (4375) + 018-37 (2513) + 018-41 (2517): widen repeat-source rule to admit a yielded group as a third category; 034-12 (4376: Iterable-runtime-only unchanged), 034-13 (4377: `.count` = synthesized cell), 034-3 (4367: membership drivers unchanged). 033-58 (4177) + 033-59 (4178): add yielded/collect lowering row + fold-kind assignment trigger (a `derived` whose RHS is a `fold` carries kind=fold). 033-80 (4199) + 035-9 (4389): keep enum `input|derived|recurrent|fold`; add rule that a fold cell is scheduled as a derived DAG node. 033-81 (4200) + 033-82 (4201) + 035-10 (4390): fold payload unchanged; state member edges feed the derived dependency-edge list. 033-169 (4288): add `fold` to the kind-led text-form enumeration. 023-14 (3028): amend DAG-node enumeration to include fold cells (as derived nodes) OR state fold is scheduled-as-derived. 023-8/9/10/12 (3022–3026): add membership-change dirty-root rule tied to the gate transition / repeat re-key, respecting no-new-dirty-bits-after-step-1. 024-2 (3075) + 024-4 (3077): add fold combiner/member reads and reads-of-fold-cell to the cycle-graph walk. 035-5 (4385): cost rule unchanged. SPEC to conform: §13.20 / §13.20.4 (23221–23260: strike on/off-bit prose [23230], define standalone-group IR object, define `.count` cell [23235–23237]), §13.21.7 (23364–23375: state fold scheduled-as-derived), §15.4.1 (24261–24270 fold payload; 24264–24270 add member edges to derived edge-list); §15.4.6 (24744 + 24772–24782: text-form already lists fold — reconcile with LOG 033-169), §13.10.2 (18284–18305: add membership dirty-root + fold DAG node), §13.9.8 (17990–17991: narrow "Gate-close adds no DAG work" to exclude folds with a gate-guarded member), §13.9.7 (17962–17973 activation-driven membership), §13.11.1 (18471–18478: fold reads in cycle graph), §13.5.4.1 (15423–15432: admit yielded repeat source). Findings: F114, F123, F093, F248, F247, F246, F213, F211, F171, F115, F167, F124, F096, F095, F094, F097, F164, F242, F099, F098. Verification greps: `grep -n 'on/off bit' SPEC.md DECISION_LOG.md` must return 0 after edit; `grep -n 'input | derived | recurrent | fold\|input|derived|recurrent|fold' DECISION_LOG.md` confirms enum unchanged; `grep -n 'repeat.*source\|Signal\[I\]' DECISION_LOG.md` at 018-37 must name the yielded category; `grep -n 'fold' DECISION_LOG.md` at 023-14 and 024-2/4 must show fold in DAG/cycle rules.

#### D-05 — Should the closed keyword lists be extended to reserve every token the corpus already calls a keyword? *(resolves 10 findings, 9 automatically)*

Ductus has a section that is the lexical authority for the language: it lists every keyword, split into four closed classes — declaration keywords, clause keywords, control-flow keywords, and operator-context keywords — plus a few one-off keyword decisions. A rule right below those lists says every keyword is reserved everywhere, can never be an ordinary identifier, and no keyword is contextual. So the intent is clear: those lists are the complete reserved-word table, and an implementer builds the lexer straight from them.

The problem is the lists are not complete. All over the rest of the corpus, tokens are used as reserved surface syntax — and many are called "keyword" outright — yet appear in none of the four lists.

Tokens the corpus explicitly calls keywords but never enumerates:

```
requires Person      // "the requires keyword" — heads a super-trait clause
enum Option[T]:      // "the enum keyword" — heads a declaration body
fn f(own x)          // "the own keyword" — parameter mode
f(move x)            // "the move keyword" — parameter mode
public / private     // "explicit keywords" for visibility
```

Tokens used as reserved block/clause heads but never enumerated:

```
given mode:          // gates arms by variant — a first-class selector
observe:             // heads a recurrent-trigger block, parallel to collect
wraps i64            // heads the newtype underlying-type clause
effects:             // heads the effect-declaration clause
cell T               // "a lowercase keyword kind," peer to signal/derived
```

And a third, narrower slice: `collect`, `fold`, and `yield` ARE listed as keywords in the LOG, but the SPEC's parallel keyword paragraph drops all three — the SPEC reserves a strictly smaller set than the LOG.

Take any of these literally against the reservation rule and you get a contradiction. A lexer built from the closed lists would treat `given`, `own`, `enum`, `requires`, `wraps`, `observe`, `cell` as ordinary identifiers. So `let given = 3`, `let cell = 5`, `let enum = 1` all compile — and then `given mode:`, `cell T`, `enum Option:` collide with those bindings. The construct decisions that head blocks and clauses assume these words are reserved; the lexical floor that must reserve them is silent.

There are two ways to close the gap, and they are not exclusive:

Model A — extend the closed lists. Add each missing token to the class it belongs to: `enum` to declaration keywords, `requires`/`wraps`/`effects` to clause keywords, `given`/`observe` to a control-flow or selector-head class, `own`/`move`/`public`/`private` to a new parameter/visibility class, `cell` as a kind keyword, and add `collect`/`fold`/`yield` back to the SPEC paragraph. Every token becomes globally reserved, matching the existing rule.

```
// declaration keywords: ... main, collect, enum
// clause keywords: ... by, else, requires, wraps, effects
```

Model B — make some of them contextual keywords, reserved only in their slot. This would let `own` or `given` still be a legal identifier elsewhere. But the reservation rule currently forbids exactly this ("no keyword class is contextual"), so Model B is not just a list edit — it requires first changing that rule, which is a real semantic loosening.

What the corpus currently says: the LOG's construct sections call these tokens keywords and use them as reserved syntax; the LOG's lexical section and the SPEC's keyword paragraph both omit them. The two halves of each document disagree with each other, and on `collect`/`fold`/`yield` the LOG and SPEC disagree directly. Every member finding is confirmed. The task framing itself — confirm extending the lists — and the reservation rule both point one way.

**Recommended: Model A, extend the closed lists (and conform the SPEC paragraph), because the corpus already calls these tokens keywords and already mandates global reservation; the lists are simply incomplete, not wrong. Model B would require repealing the "no contextual keywords" rule — a deliberate semantic change nobody has asked for and the audit did not surface as intended.**

Consequences:

- Nine of ten members conform automatically once the lists are extended and the SPEC paragraph is synced: the SPEC-drops-three divergence, and the eight missing-token findings for `given` (twice), `observe`, `wraps`, `requires`, `enum`, `own`, and `cell`. They are all the same defect — a token used as a keyword but absent from the closed lists — and all resolve by the same edit.
- **Class assignment (residual):** the audit does not dictate which class each token joins, and some do not fit the existing four cleanly. `own`/`move` are parameter modes and `public`/`private` are visibility modifiers — neither maps to declaration/clause/control-flow/operator-context. Do you add them to an existing class, or introduce new classes (a parameter-mode class, a visibility class)? Answer inline: e.g. "new class for own/move/public/private" or "fold own/move into operator-context."
- **`cell` as keyword-kind (residual):** the SPEC calls `cell` a lowercase keyword kind peer to `signal`/`derived`, which ARE declaration keywords. But `cell` has no declaration form of its own — it appears only as the `cell T` annotation. Do you reserve it as a full keyword (blocking `let cell = 5`), or correct the SPEC to call it a non-reserved annotation form? Answer inline.
- **Coverage completeness (residual):** the missing set is likely wider than the ten members name. The charity checks also flag `move`, `dyn`, `public`, `private`, and `effects` as called-keyword-or-reserved-head yet unenumerated. Do you want a full sweep to reserve every such token in one pass, or only the named members? Answer inline.

> **For execution:** Canonical side = the LOG construct decisions (they define the keywords); the fix is to complete the closed lists and conform the SPEC. Edit LOG keyword enumerations 002-3 (declaration; add `enum`), 002-4 (clause; add `requires`, `wraps`, `effects`), 002-6 (control-flow; place `given`, `observe` or a new selector-head class), plus a class/decision for `own`, `move`, `public`, `private`, and a reservation for `cell` (reconcile with 016-1/016-62/016-178, SPEC §13.2.8 line 20685). Then conform SPEC §1.4 (SPEC.md:125-144): add `collect`, `fold` to the declaration list (line 126-128) and `yield` to the control-flow list (line 135-136) [F252], plus every token added on the LOG side [F251, F007, F141, F140, F139, F138, F137, F136, F060]. Do NOT touch 002-27 unless Model B is chosen. Findings: F251 (H, joint gap), F252 (M, SPEC drops collect/fold/yield), F060 (cell), F007+F138 (given), F141 (observe), F140 (wraps), F139 (requires), F137 (enum), F136 (own). Verify: `grep -n '^002-3\.\|^002-4\.\|^002-6\.' DECISION_LOG.md` and `sed -n '125,144p' SPEC.md`, then confirm each of given/observe/wraps/requires/enum/own/cell/collect/fold/yield appears in a closed list in both docs. Cross-check no token remains that the corpus calls "keyword" but the lists omit: `grep -nE '\`[a-z]+\`.*keyword' DECISION_LOG.md`.

#### D-06 — When a decision's SPEC pointer lands on a section that does not explain the claim, do we move the pointer or grow the section? *(resolves 9 findings, 3 automatically)*

Every decision in the log ends with a pointer like `(§8.4.1)` — a promise that the named SPEC section elaborates that decision. When someone wants to check what a decision really means, they follow the pointer. The pointer is a contract: land the reader on the text that explains the rule. Right now, nine decisions point at sections that do not explain the rule they carry. The rule lives somewhere else in the SPEC, or nowhere. This item picks the standing policy for fixing a broken pointer, and settles when pointing at a *parent* section (an "umbrella" cite) is fine and when it is a break.

There are two ways to fix any single broken pointer, and one prior question underneath them:

Option A — repoint the decision to the section that already explains it.
```
008-57. ...storability razor... (§5.7)      // §5.7 never mentions the razor
008-57. ...storability razor... (§13.2.8)   // §13.2.8 states it verbatim; sibling 016-179 already cites it
```
Move the pointer to where the text already lives. Cheap. No SPEC edit.

Option B — extend the cited section so its text grows to cover the claim.
```
017-190. ...expr? Try-propagation AND expr?.field optional chaining... (§8.4.1)
// §8.4.1 desugars only expr?; add an optional-chaining paragraph so §8.4.1 covers both
```
Grow the section under the existing pointer. Keeps the pointer stable but duplicates or relocates normative text.

The prior question: is pointing at a *parent* section acceptable when the real detail is a nested child?
```
005-68/69/70. effect-kind methods... (§3.1.1)   // detail is in §3.1.1.1, nested INSIDE §3.1.1
```
Here §3.1.1.1 sits strictly inside §3.1.1. A reader following `(§3.1.1)` reads the whole section including the child, and reaches the material. That is an *umbrella cite*, and it works. Contrast a distant target:
```
006-27. .exposition typed-bound carve-out... (§3.5.7)   // detail is in §13.3.7.7, a far-off section, NOT nested under §3.5.7
```
Here the pointer sends you to §3.5.7, and §13.3.7.7 is nowhere below it. Reading §3.5.7 never reaches the rule. That umbrella does not cover anything — it is a genuine break.

**What the corpus currently says, and where it disagrees with itself.** The log's own convention is a single section-ref per decision that must elaborate the claim. On the effect-kind method decisions, every related decision uniformly cites the parent §3.1.1 and *no* decision cites the child §3.1.1.1 — so the parent cite is the section's deliberate umbrella, not a slip. The audit verified §3.1.1.1 nests strictly between §3.1.1 (SPEC line 1192) and §3.1.2 (line 1255); that finding was refuted for exactly this reason. But seven other decisions cite sections that do *not* contain the claim and are *not* the parent of any section that does: the razor decision cites §5.7 while the razor text sits only at §13.2.8; two integer/bool/string Hash decisions cite §4.9.4 while Hash conformance lives one sibling over at §4.9.5; the exposition carve-out cites §3.5.7 while it is stated at §13.3.7.7; the trait-headed record-match ban cites §6.2.4 while the ban lives at §13.3.7.7 (which itself cites §6.2.4 back — a loop); the optional-chaining half of the `?` decision cites §8.4.1 which desugars only standalone `expr?`; the interpretation-root render clause cites §13.14.1 which never mentions renders or stable paths; the generalized implicit-derived-bridge cites §13.8.2.1 which covers only the attr-RHS case; and the portal-reload decision cites §15.4.6 (IR grammar) where the word "Portal" does not appear, while the real elaboration is at §13.3.6.3. So the log holds one valid umbrella pattern and seven broken pointers, and it does not state a rule telling them apart.

**Recommended: adopt a two-part policy. (1) Umbrella cites are valid only when the cited section is an ancestor of the section that elaborates the claim — parent-to-nested-child is fine, sibling or distant is a break. (2) The default fix for a break is repoint (Option A), not extend (Option B); extend only when no section elaborates the claim and the cited section is the natural home. Because** the single-ref-must-elaborate contract is what makes the pointer trustworthy, the corpus already uses parent-umbrella cites deliberately and consistently for the effect-kind methods, and repointing avoids duplicating normative text across two SPEC locations (the divergence risk the edit protocol exists to prevent). For most of the seven breaks the target section already exists and a sibling decision already cites it correctly, so repoint is both cheaper and lower-risk than growing a second copy.

**Consequences:**
- **Conforms automatically (3):** The effect-kind method umbrella cite (F012) is validated by part (1) — it is a parent-of-nested-child cite and needs no change; the finding was already refuted. The two razor/Hash-family cases each have a sibling decision that already points at the right section (016-179 → §13.2.8; 007-237 → §4.9.5), so the repoint target is fixed by precedent, not a judgment call — F040 and F036 resolve by copying the sibling's target with no design choice left open.
- **STILL — repoint-vs-extend, one call each:**
  - **F185 (optional chaining `?`):** repoint the optional-chaining half of the decision to the section that desugars `?.`/`?[]`/`?()` (§13.3.6.2 region), or extend §8.4.1 to cover optional chaining. Which?
  - **F034 (exposition typed-bound carve-out):** repoint to §13.3.7.7, or extend §3.5.7 to state the carve-out. Which?
  - **F016 (trait-headed record-match ban):** repoint to §13.3.7.7, or move the ban text into §6.2.4 (note §13.3.7.7 already cites §6.2.4 back, so a decision is needed to break the loop). Which?
  - **F207 (portal reload):** repoint from the IR-grammar section §15.4.6 to the portal hot-reload section §13.3.6.3, or add portal-reload text to §15.4.6. §13.3.6.3 already carries the elaboration — repoint is the low-risk default; confirm?
  - **F175 (interpretation-root renders mount at stable paths):** the clause has no elaborating section anywhere found. Repoint to the section that defines interpretation-root render mounting, or add that content to §13.14.1. Which section owns it?
  - **F174 (generalized implicit-derived-bridge):** extend §13.8.2.1 to cover operator args, effect args, and `|>` LHS, or repoint that half elsewhere. Which?
- **Plausible — dismiss-or-pin:** F012 is marked plausible/auto. Its charity check refuted it (parent-umbrella is valid). Crux to confirm before dismissing: verify §3.1.1.1 still nests strictly inside §3.1.1 (between the §3.1.1 and §3.1.2 headings) and that no decision cites §3.1.1.1 directly — if both hold, dismiss; if a later edit un-nested it, it reopens as a real break.

> **For execution:** Canonical side = LOG decisions (repoint the ref; the LOG is authoritative on which section owns a claim, per the edit protocol). Findings: F185, F040, F036, F034, F016, F012, F175, F174, F207.
> - **Auto-conform (3):** F012 (LOG 005-68/005-69/005-70 @ DECISION_LOG.md:410-412, cite §3.1.1) — validated, no edit; §3.1.1.1 nests at SPEC.md:1246 between §3.1.1 (1215) and §3.1.2 (1278). F040 (LOG 008-57 @ :919) — repoint (§5.7)→(§13.2.8) to match sibling 016-179 @ :2038; razor text at SPEC.md:12672. F036 (LOG 007-234/007-235 @ :854-855) — repoint (§4.9.4)→(§4.9.5) to match sibling 007-237 @ :857; Hash table at SPEC.md:4316-4324 (under the §4.9.5 "Built-in Hash conformance" subhead @ 4310).
> - **STILL (6):** F185 (LOG 017-190 @ :2337; §8.4.1 @ SPEC.md:6268; optional-chaining elaboration at §13.3.6.2 region, heading @ SPEC.md:14319, `?.`/`?[]`/`?()` semantics @ 14394). F034 (LOG 006-27 @ :610; §3.5.7 @ SPEC.md:2364; carve-out at §13.3.7.7, heading @ SPEC.md:14918, bounded pattern @ 14955). F016 (LOG 009-91 @ :1030; §6.2.4 @ SPEC.md:5307; ban at §13.3.7.7, back-cite @ SPEC.md:14967 — back-cites §6.2.4). F207 (LOG 033-113 @ :4232; §15.4.6 @ SPEC.md:24717-24820, zero "Portal" hits; real elaboration §13.3.6.3 @ SPEC.md:14499, hot-reload text @ 14576 — and sibling 028-12 @ :3319 already repoints portal-reload to §13.3.6.3 correctly). F175 (LOG 021-141 @ :2886; §13.14.1 @ SPEC.md:19042; no owning section identified). F174 (LOG 021-16 @ :2761; §13.8.2.1 @ SPEC.md:16678, attr-RHS only).
> - **Verification greps:** nesting check `grep -n '^#### 3\.1\.1 \|^##### 3\.1\.1\.1 \|^#### 3\.1\.2 ' SPEC.md`; per-repoint, confirm target string exists, e.g. `grep -n 'storability razor' SPEC.md` (§13.2.8, expect 12672), `grep -n 'Built-in Hash conformance' SPEC.md` (§4.9.5, expect 4310), `grep -n 'A portal preserves across reload' SPEC.md` (§13.3.6.3, expect 14576). After each repoint: re-grep the cited section body for the claim's keyword to confirm the new pointer lands.

#### D-07 — When the same rule appears in many places, which copies are legitimate and which are just duplicates? *(resolves 6 findings, 3 automatically)*

> **Post-merge status (2026-07-08, corpus at 1f204c473a):** Two premises shifted since the draft; the item's recommendation (rider test, six families, AUTOMATIC-vs-STILL split) still stands, but two factual anchors changed. (1) F131's deduped fact is now stale. GRAMMAR.md and IR_GRAMMAR.md now exist as separate grammar documents — GRAMMAR.md's own header calls itself "a reformulation of rules already established normatively in" SPEC/LOG, i.e. non-normative. Yet 001-3 (line 33, "there is no separate grammar document") and 002-29 (line 100, "no separate grammar document and no normative content is delegated to one") were NOT amended by the grammar session and now literally contradict the repo. The deduplication F131 flags still applies (both entries still state the identical fact), but the owner should reconcile the claim with the new docs (e.g. "no separate NORMATIVE grammar document") rather than merely collapse a now-inaccurate statement. (2) F191's punctuation outlier is already resolved. 015-39 (line 1854) now joins the two reconciler clauses with a semicolon, matching every other iff site (027-80/81, 031-119/128/143/157, 033-124); grep for the ", and" variant ('stream`), and') returns nothing, so the optional punctuation-normalization sub-task the item flags as F191 is already done. All other anchors are valid — only LOG line numbers drifted (see evidence), every entry located and confirmed by content.


The decision log is built on a rule it states about itself: every entry stands alone. A reader who needs one rule reads exactly one entry and nothing else. Entries never point at each other by number or name. The direct consequence, written into the log's own invariants, is that when a rule needs a piece of another rule to make sense, that piece is copied in, not linked. Cross-references are banned; local restatement is mandatory. So duplication in this log is not automatically a smell — much of it is the design working as intended.

The audit flagged six families where the same fact appears at multiple sites and asked: consolidate to one authoritative entry, or leave the copies. The apparent trap is that self-containment pulls one way (keep copies) and ordinary "don't repeat yourself" pulls the other (consolidate). But these two forces do not actually conflict — because the log's own rule only requires a copy in one specific case, and several of these families fall outside it.

Read the invariant carefully. It says: copy a rule locally only when the entry is incomplete without it. Under that test the flagged families split into two piles.

Pile one — a copy that carries its own distinct content:

```
005-185. A node/connection/effect type in value position is carried by Type[…].
016-223. A node/connection type is carried via Type[…], the mechanism for an
         attr template slot deferring which kind a receiver places.
```

The general fact is the same, but the second entry adds a real rider — how attr template slots use it. Trimming it to a bare pointer would strip content the attr-section reader needs. This is the invariant working as designed.

Pile two — a copy that carries nothing new:

```
001-25. …a borrow cannot be stored in a record field, tuple component,
        enum payload, or indexed slot.
013-52. A borrow-equivalent alias cannot be stored in a record field, tuple
        component, enum payload, or indexed slot.
```

Here the second entry is the first entry's trailing clause, verbatim, with no added content. The invariant does not require this copy, because 013-52 is complete on its own without anything from 001-25. It is a plain duplicate.

The two policies on offer:

Option A — one blanket rule: "duplication is a smell, consolidate everywhere." This is wrong for this log. It would force cross-references or force readers to chase two entries for one rule, breaking the self-containment the log is built on. It also cannot be applied — you cannot consolidate 016-223 into 005-185 without deleting the attr-template rider.

Option B — the rider test: keep a restatement only when the copy carries content the copied-from entry lacks; collapse a copy that is byte-for-byte redundant with no local rider. This matches the invariant's actual wording and treats the six families correctly instead of uniformly.

What the corpus says. The invariant text is explicit: "If a rule is incomplete without another rule's content, the rule is restated locally; cross-references between entries are forbidden." That word "incomplete" is the whole test. The audit's own verdicts already sort the families this way — three families were flagged as "each restatement carries a genuinely distinct rider" (the weaker, lower-confidence flags), and three as verbatim copies with nothing distinct. The log does not disagree with itself here; the audit found no contradictions in any of these families, only redundancy. So the choice is not about correctness, it is about which copies earn their keep.

**Recommended: Option B, the rider test, because it is literally what the log's self-containment invariant already commands — restate only when incomplete without the other entry — and it resolves each family on its own merits instead of applying one blunt rule that would break self-containment for the rider-bearing copies and leave the true duplicates in place.**

Consequences by family:

- Automatic — three families are pure verbatim duplicates with no distinct rider, so under the rider test they collapse to one entry with no judgment call needed. The four-slot borrow-storage list stated twice (the general first-class-citizen entry vs. the borrow-alias entry); the "no separate grammar document / single authoritative source" fact stated across three-to-four entries in two sections; and the two adjacent reconciler-registration entries that share an identical lead sentence and only differ in the trailing startup-failure clause (these three findings are one verified defect). Each drops to a single authoritative entry; the reconciler pair merges into one entry keeping both the generic-instantiation and the general startup-refusal behavior.

- **STILL — F135 (Type[…]-as-value family):** keep or trim? Each of the four per-construct copies carries a thin rider (attr template slot, Handle vs. storable designation, template slot, effect-vs-instance). Sub-decision: do these riders count as "distinct content" that earns a full restatement, or should the bare "carried via Type[…]" clause be dropped from the per-construct entries leaving only the rider? Recommend keeping them — the riders are real and the general entry does not cover them.

- **STILL — F133 (graph-member "held by borrow, never in cells or records" family):** keep or trim? The ownership rule appears three times with overlapping coverage (node+connection, connection+effect, effect), each with a small rider (Type[…] stand-in, effects: clause). Sub-decision: state the ownership rule once for all graph members and keep only the distinct riders per entry, or leave the overlapping copies as-is? Recommend trimming the overlap — the shared clause is identical and the riders are what differ.

- **STILL — F134 (entry-match bounded-pattern carve-out family):** keep or trim? The same carve-out ("Variant(name: Bound) legal only in .exposition/entry matches, banned over records") is stated three times across three sections, and unlike the other families the audit found no distinct rider — the three copies say the same thing. Sub-decision: does this belong in the automatic pile (collapse, since no rider), or does its placement in three genuinely different topic sections justify keeping self-contained copies for each section's reader? Recommend collapse-to-one unless you judge cross-section reachability more valuable than dedup here.

- **STILL — F092 (reconciler-iff nine-site restatement):** the iff clause is restated across roughly nine LOG and SPEC sites in genuinely different sections. This one is the clearest "invariant working as designed" case — a reader in the effects section, the runtime section, and the IR section each needs the rule, and self-containment mandates the copy. Sub-decision: leave all nine as required restatements (recommended), or additionally normalize their punctuation/wording so the nine copies read identically (the audit separately noted one site joins the two clauses with ", and" while the rest use a semicolon — a drift-fix, not a consolidation). Recommend leave the copies, and fold the punctuation-normalization into whatever wording pass touches these entries.

Plausible-but-not-blocking note: none of these six findings are contradictions — the audit rated every one LOW/design_smell and confirmed the copies agree. The verifier's crux for any "collapse" action is Invariant 1: the log uses dense positional numbering, so deleting an entry renumbers every later entry in its section. That means even the automatic collapses are not free edits — each deletion shifts ids below it in the same section. Do not treat "automatic" as "mechanical and safe to apply silently"; the renumbering ripple is why every one of these is surfaced rather than applied.

> **For execution:** Canonical side is the LOG's Invariant 2 (DECISION_LOG.md line 11): restate locally only when an entry is "incomplete without another rule's content." Apply the rider test per family. AUTOMATIC collapses (verbatim, no rider): F132 — merge the four-slot borrow-storage list, keep in 001-25 (line 55), drop the duplicate clause from 013-52 (line 1444) or scope 013-52 to the alias-specific case; F131 — consolidate the "no separate grammar document / single authoritative source" fact across 001-3, 001-4 (lines 33-34), 002-29 (line 100), partial 001-1 (line 31); F092+F190+F260 (one defect) — merge adjacent 027-80 and 027-81 (lines 3264-3265, §13.14.7) into one entry preserving both the generic-instantiation refusal and the general startup-refuse-with-diagnostic clause. STILL families (surface, user picks keep-vs-trim): F135 — 005-185 (line 527), 016-223 (line 2082), 017-167 (line 2314), 019-21 (line 2643), 031-6 (line 3782); SPEC §3.7.4, 13.2.10, 13.3.6.1, 13.6.0, 13.19. F133 — 016-224 (line 2083), 017-170 (line 2317), 031-5 (line 3781); SPEC §13.2.10, 13.3.6.1, 13.19. F134 — 006-27 (line 610), 009-91 (line 1030), 017-221 (line 2368); SPEC §3.5.7, 6.2.4, 13.3.7. F092 nine-site: 027-80, 027-81, 031-119, 031-128, 031-143, 031-157, 033-124, plus 015-39 (line 1854, the ", and" punctuation outlier flagged separately as F191) and SPEC §13.14.7, 13.19.14 — recommend leave-as-required-restatement, optionally normalize punctuation. Constraint: Invariant 1 dense positional numbering — every deletion renumbers later entries in the same section; do not apply collapses silently. Verification greps: `grep -n 'record field, tuple component, enum payload, or indexed slot' DECISION_LOG.md` (expect 001-25, 013-52); `grep -n 'no separate grammar document' DECISION_LOG.md` (expect 001-3, 002-29); `grep -n 'required if and only if' DECISION_LOG.md` (expect the ~7 reconciler sites); `grep -n 'carried .*via .*Type\[' DECISION_LOG.md`. Findings: F131, F132, F133, F134, F135, F092/F190/F260.

#### D-08 — When exactly do reconciler hooks fire around a commit, and in what order? *(resolves 6 findings, 0 automatically)*

An effect in Ductus talks to the outside world through a host-written reconciler with five hooks: `create`, `update`, `teardown`, `suspend`, `resume`. The runtime fires these hooks around each `commit`. `create` mints the reconciler's per-instance state and returns it; `update` receives that state to reconcile new desired values; `suspend`/`resume` release and re-acquire the external resource when a gate closes and reopens. For any of this to work, the runtime must pin two things: **where** hooks sit relative to the moment a commit publishes its new snapshot, and **in what order** hooks for one instance fire when several are eligible at the same commit. Right now the corpus pins neither cleanly, and it contradicts itself on the first.

The core conflict is about suspend/resume placement. Two models sit in the text at once.

Model A — suspend/resume are part of the commit body, firing *before* the snapshot is published:

```
commit():
  settle dirty cells
  advance recurrents
  fire suspend/resume         # before publish
  publish new snapshot
```

Model B — suspend/resume are reconciler hooks, firing *after* the snapshot is published, alongside create/update/teardown:

```
commit():
  settle dirty cells
  advance recurrents
  publish new snapshot
  fire ALL reconciler hooks   # create, update, teardown, suspend, resume — after publish
```

These are not cosmetic. A suspend/resume hook that runs before publish observes the pre-publish state; one that runs after observes the just-published snapshot. The snapshot a hook sees is program-visible, so the two models give different behavior.

What the corpus says, and where it fights itself: the commit-verb section describes commit as one that "settles dirty cells, advances recurrents, fires suspend/resume for effects whose activation changed, and publishes a new snapshot" — that is Model A, suspend/resume before publish. The steady-state commit entry and the hook-timing section say the opposite: all reconciler hooks fire "after the commit publishes its snapshot and before the next commit begins," and the lifecycle map explicitly lists suspend and resume as reconciler hooks. So suspend/resume are asserted to fire both before publish (as commit body) and after publish (as hooks). The SPEC mirrors the split verbatim across its two sections. This is the contradiction (F188).

Once placement is fixed, three ordering gaps and one gap-that-crosses-boundaries remain — all facets of "when several hooks are eligible for one instance at one commit boundary, what order do they fire in?"

Ordering problem 1 — create before update (F091). A `repeat` in an `effects:` clause adds a new element. In the same commit, that new instance both enters the live graph (eligible for `create`) and has dirty desired/param cells (eligible for `update`). `create` returns the instance state; `update` consumes it. If `update` runs first, there is no state to pass — an unsatisfiable call. The enumeration rule for update ("effect instances whose parameters or desired cells became dirty during that commit") carries no "existing" qualifier, so it would sweep in the just-created instance. Nothing orders create first.

Ordering problem 2 — initial-cohort create (F089). Startup enumerates load-IR, allocate, init cells, first commit — and never fires `create` for the effects live at startup. The runtime is "live" only after the first commit, and hooks fire after a commit publishes; so there is no named moment for the initial cohort's `create` to run and produce the state every later `update` needs.

Ordering problem 3 — resume before update at gate-open (F249). On a false→true gate flip, the desired-cell re-snap happens inside the flipping commit, so the effect's desired cells go dirty that commit. Both `resume` (gate went true) and `update` (desired dirty) are eligible at that one boundary. If `update` runs first, the reconciler reconciles fresh desired state against a resource `resume` has not yet re-acquired.

Reload commit (F108). Reload step 8 dirties effect desired cells; step 9 is "commit the reloaded state." The universal rule "every commit fires reconcilers against dirtied instances" is qualified "in steady state," and the reload commit runs under the reload lock, not steady state. So it is unstated whether the reload commit fires `update` (reconcile external reality at the reload instant) or defers to the next host commit.

Blocking question (F189, plausible). Does `commit()` block on hook execution? The verifier refuted the ambiguity: the commit entry says commit "settles... publishes... and then fires reconciler hooks," and a separate entry says commit "is synchronous and returns when the commit completes." Together those force one reading — hooks run inside the `commit()` call, so `commit()` blocks on them. No text puts hooks in a post-return host-driven window.

**Recommended: fix placement to Model B (all five hooks, including suspend/resume, are post-publish reconciler hooks running inside the `commit()` call), and pin a single per-instance hook order at each boundary: teardown, then create, then resume, then update; because Model B is the majority position (three entries plus the hook-timing SPEC section say post-publish, versus one commit-verb clause), it is the only placement under which suspend/resume observe the same just-published snapshot as every other hook, and a fixed create-before-update / resume-before-update order is the only order under which `update` always has the instance state and the re-acquired resource it consumes.** The blocking question needs no new decision — the corpus already forces "commit blocks on hooks"; only make the SPEC state it in one place so no reader re-derives it.

Consequences:

- **F188 (contradiction) — STILL, needs your ruling on placement.** Recommendation is Model B. Residual sub-decision: *Confirm suspend/resume move out of the commit body to post-publish reconciler-hook status (fix the commit-verb clause), or instead keep them commit-internal and demote them out of the reconciler-hook set (fix the two hook-timing entries and the SPEC hook-timing section)?* One side must yield; pick which.
- **F091 (create-before-update) — STILL.** Residual: *Adopt "for an instance newly entering the live graph this commit, `create` fires before any same-boundary `update` for it" — or instead exclude a just-created instance from that commit's update enumeration?* Both close the gap; they differ in whether a create-time desired change reconciles now or next commit.
- **F089 (initial-cohort create) — STILL.** Residual: *Fire the initial cohort's `create` during construction before the first commit, or as the first post-first-commit hook pass?* The recommended fixed order makes either work; choose which moment.
- **F249 (resume-before-update) — STILL.** Residual: *Pin `resume` strictly before any same-boundary `update` for a just-reopened instance — or coalesce them?* Recommendation is resume-first (matches the general "acquire before reconcile" rule); confirm.
- **F108 (reload commit tail) — STILL.** Residual: *Does reload step 9 run the ordinary reconciler-firing tail against every instance the reload dirtied, or defer all reconciliation to the first post-reload host commit?* Recommendation leans "fire it" for consistency with every other commit, but this one is genuinely open — deferring avoids reconciling under the reload lock.
- **F189 (does commit block) — PLAUSIBLE, dismiss-or-pin.** Verifier crux: the two commit entries already force "hooks run inside `commit()`, so it blocks"; the finding survives only as a documentation gap, not a real ambiguity. Dismiss as already-determined, or pin one explicit SPEC sentence to stop future re-derivation.

> **For execution:** Canonical side = Model B (all five hooks post-publish, inside the blocking `commit()` call) + fixed per-instance order teardown→create→resume→update.
> - **F188** (contradiction, placement): entries in conflict are 027-50 + SPEC §13.14.4 (SPEC.md ~19147-19151, Model A: suspend/resume before publish) vs 027-32, 027-91, 027-96 + SPEC §13.14.9 (SPEC.md ~19335-19336, Model B: after publish). To adopt Model B: rewrite 027-50 so `commit` does not itself fire suspend/resume before publish, and remove the "fires suspend/resume ... and publishes" clause from SPEC §13.14.4; keep 027-91/027-96 as the single source of truth. Conform SPEC §13.14.4 to §13.14.9.
> - **F091** (create-before-update): add an ordering rule near 027-69/027-70 (§13.14.7) and 027-91/027-96 (§13.14.9); reconcile with the update-enumeration 031-124 (§13.19.14, DECISION_LOG.md:3900) which currently lacks the "existing" qualifier that 027-70 carries.
> - **F089** (initial-cohort create): add a startup step in the 027-16..027-28 sequence (§13.14.1, DECISION_LOG.md:3200-3212) pinning when the initial cohort's `create` fires relative to 027-28 first-commit and 027-29 live-transition.
> - **F249** (resume-before-update at gate-open): add ordering in §13.14.9 or §13.19.14; anchors 022-59 (§13.9.7, DECISION_LOG.md:2949), 031-108 (§13.19.12), 031-124/125 (§13.19.14).
> - **F108** (reload commit tail): state in 028-23 / SPEC §13.15.3 (SPEC.md:19510, "9. Commit the reloaded state") whether step 9 runs the 031-124/125 reconciler tail or defers; note 027-32's "in steady state" qualifier is why the universal rule does not auto-apply.
> - **F189** (blocking): no normative change required (verifier refuted the ambiguity via 027-32 + 027-52, DECISION_LOG.md:3216 and 3236); optionally add one SPEC §13.14.9 sentence stating `commit()` blocks until hooks return.
> - Verification greps: `grep -n '^027-32\.\|^027-50\.\|^027-91\.\|^027-96\.\|^031-124\.\|^028-23\.' DECISION_LOG.md`; `grep -n 'suspend/resume\|publishes its snapshot\|Hook timing' SPEC.md`. After edits, confirm no SPEC passage still places suspend/resume before publish and no entry orders update ahead of create/resume for the same instance.

#### D-09 — Which handle kind freezes and re-points — `Handle` or `WeakHandle`? *(resolves 7 findings, 5 automatically)*

Ductus has two handle types for referring to a graph node you don't own outright. One is meant to be stable: the node it points at is fixed for the handle's whole life, so a read through it always succeeds. The other is meant to move: the node it points at can be dismounted or swapped at runtime, so a read can come back empty. Connections, the required kind of incoming-view, and freeze behavior all hinge on which of the two a destination uses. The problem is that the two names got swapped across a batch of entries, so the same word means opposite things depending on where you read.

The settled split reads like this:

```
handle! some_node    // -> Handle[T]      static:  node fixed, read is &T,        never empty, never freezes
handle some_node     // -> WeakHandle[T]  dynamic: node can move, read is Option[&T], empty -> freezes
```

Model A — the settled definition. The section that defines the two handle types is internally clean and complete. `Handle[T]` is the statically-placed form: it auto-derefs straight to `&T`, resolution is unnecessary, and it never freezes. `WeakHandle[T]` is the dynamically-placed form: it reads as `Option[&T]`, and a destination built on it freezes while that read is `None`. Bare keyword `handle` always produces `WeakHandle`; `handle!` produces `Handle` and must prove static placement.

```
// Model A applied to a connection destination:
attr wheel: WeakHandle[Drivable]   // dynamic -> reads Option, can freeze, membership varies
attr wheel: Handle[Drivable]       // static  -> reads &Drivable, never freezes, membership fixed
```

Model B — the swapped text. Several entries and SPEC sections attach the dynamic behavior (reads `Option[&N]`, freezes on `None`, re-points, dynamizes membership) to a bare `Handle` or a bracketed `Handle[N]`. That directly contradicts Model A: a `Handle[T]` cannot both auto-deref to `&T` and read `Option[&N]` and freeze. An implementer following the swapped text would build a resolving, freezing `Handle` destination and would require a `dynamic incoming` view for a node whose membership is actually static.

Where the corpus disagrees with itself. The definition section, the freeze-capable persist/re-point destination entries, the connection-semantics entries, and the self-sourced wiring entries all use `WeakHandle` for the dynamic role. But the placement elaboration in the SPEC pins the freeze-on-`None`, `Option`-reading destination as `Handle[N]`; the connection entry that says `to` follows a re-pointing target names a bare `Handle`; the membership entry says a `Handle` destination makes membership a runtime fact; and the worked example types `wheel` as `Handle[Drivable]` yet gives it a `dynamic incoming` view. The self-sourced enumeration lists the third destination form as a `Handle`-typed attr while its own three follow-up entries spell that same form `WeakHandle`. There is no passage anywhere that makes a static `Handle[T]` re-point, freeze, or dynamize membership — so Model A is the only self-consistent reading, and every Model-B mention is a swap, not a real second design.

**Recommended: Model A (WeakHandle is the dynamic, freeze-capable, membership-dynamizing carrier; Handle is static and never freezes), because the handle-definition section is internally consistent on exactly this split, the dynamic-role entries already use WeakHandle everywhere, and no passage grants a static Handle any dynamic behavior — so every conflicting mention is a mislabel to correct toward WeakHandle.**

Consequences:

- **Five findings conform automatically** once Model A is canonical — they are pure mislabels with no design left to decide. The SPEC placement elaboration that pins `Handle[N]` as the `Option`-reading freeze-on-`None` destination (the highest-severity one), the SPEC/LOG divergence on the same carrier, the connection entry whose static `Handle`'s `to` follows a re-pointing target, the terminology-drift entry naming `Handle` as the runtime-membership carrier, and the worked example typing `wheel: Handle[Drivable]` with a `dynamic incoming` view — all become "change `Handle` to `WeakHandle`" edits.
- **Membership-dynamizing entries still need one sub-decision (P020):** The two entries that say a bare `Handle` destination dynamizes membership and forces a `dynamic incoming` view — is bare `Handle` there a mislabel for `WeakHandle`, or is it the umbrella term covering both types? *If you say mislabel:* rewrite both to `WeakHandle` and the defect closes cleanly. *If you say umbrella:* you must also state that only the `WeakHandle` member dynamizes membership, and that a static `Handle[T]` attr keeps a static `incoming` view — otherwise the umbrella reading still forces a static handle into a dynamic view, which is unsatisfiable. Recommend the mislabel reading, because the dynamic role is `WeakHandle`'s everywhere else and the umbrella reading needs extra carve-out text to stay coherent.
- **Self-sourced enumeration still needs one sub-decision (P021):** The enumeration of the three admissible self-sourced destination forms names the third a `Handle`-typed attr, but its elaboration spells it `WeakHandle`. Is a static `Handle[T]` attr an admissible self-sourced destination at all? *If no:* rewrite the enumeration's third form to `WeakHandle`-typed and it matches its elaboration. *If yes:* add an entry stating a static `Handle[T]` attr's self-sourced behavior (static `incoming`, never freezes), because the current elaboration only covers the `WeakHandle` sub-case. Recommend rewriting to `WeakHandle` unless you specifically want static-handle self-sourcing as a feature.

> **For execution:** Canonical side = Model A, the settled §13.3.6.2 split (LOG 017-176, 017-179, 017-181, 017-182, 017-186: `Handle[T]` static / auto-derefs `&T` / never freezes; `WeakHandle[T]` dynamic / reads `Option[&T]` / freezes on `None`). AUTO fixes (relabel `Handle`→`WeakHandle`, SPEC-conforms-to-LOG): F228 [H] SPEC §13.8.5.1 lines ~17117-17120 → `WeakHandle[N]` (match LOG 021-82/021-83, 019-52/019-53); F172 [M] SPEC §13.8.4 lines ~16965-16968 and §13.8.5.1 lines ~17117-17120 → `WeakHandle[N]` (match LOG 021-56, 021-82, 021-83); F177 [M] LOG 019-49 (line 2671) → name only a re-pointing kind (`WeakHandle`/reactive selection), reconcile with 019-52 (2674), 019-17 (2639), SPEC §13.6.2 line ~16181 which already says `WeakHandle`; F176 [L] LOG 021-57 (line 2802) → `WeakHandle` (match 021-56/2801, 021-82/2827, 021-83/2828, 021-84/2829); F085 [M] SPEC §13.3.4.2 worked example lines ~14171-14190 → type `wheel: WeakHandle[Drivable]` (match LOG 017-139/2286, 017-141/2288). STILL: P020 [H] LOG 017-111 (line 2258) + 017-147 (line 2294) + SPEC §13.3.4 lines ~14052-14055 → user picks mislabel (→`WeakHandle`) vs umbrella (add carve-out that only `WeakHandle` dynamizes membership; static `Handle[T]` keeps static `incoming`); note SPEC 14054 self-refutingly cites §13.6.2 which per 019-52/53 makes `Handle[N]` static. P021 [M] LOG 017-136 (line 2283) third form → user picks relabel-to-`WeakHandle` vs admit static `Handle[T]` self-sourced form + add its static-membership entry; elaboration 017-139/140/141 (lines 2286-2288) already says `WeakHandle`. VERIFY: `grep -n 'Handle' packages/ductus-lang/docs/SPEC.md` around §13.8.4/§13.8.5.1/§13.6.2/§13.3.4/§13.3.4.2 shows no bracketed `Handle[N]` in an `Option[&N]`/freeze/re-point/dynamic-membership context; `grep -n '^019-49\.\|^021-57\.\|^017-111\.\|^017-136\.\|^017-147\.' packages/ductus-lang/docs/DECISION_LOG.md` shows the dynamic role spelled `WeakHandle` (or, for P020/P021, the chosen umbrella carve-out present).

#### D-10 — What iterator shape does a stores-nothing `yielded` group present, and what does a loop see when a member joins or leaves mid-pass? *(resolves 4 findings, 2 automatically)*

Decide the IR shape of a fold/yielded group first (the item on how a fold cell is serialized), because whether a `yielded` group can name a `Source` at all depends on whether the IR gives it any backing storage. This item assumes that item keeps the stores-nothing realization.

In Ductus a `yielded` group is the set of values produced by `yield` statements inside a `collect` block. It is a reactive cell-kind, but it stores nothing: no backing array, just a compile-time wire set plus runtime on/off bits saying which members are currently live. A member can appear or disappear at runtime — a `yield` inside a gated arm is present only while that arm is active. The amendment also declared this group is walkable by a runtime `for` loop: it "fulfills Iterable." That is the promise this item must make concrete.

The trouble is that the loop machinery was built for ordinary collections, and it needs two things a stores-nothing group cannot obviously supply.

First, an iterator has a `Source` — the value it reads from on each step. The machinery offers exactly two shapes:

```
// Shape A: self-contained. Iterator holds all its own state.
type Source = ()            // Range, Counter — next() reads nothing external

// Shape B: source-bearing. Iterator holds a cursor; the collection is the Source.
type Source = TheCollection // Vec, Map — next() reads through the backing collection
```

A `yielded` group has no backing collection, so shape B has nothing to name as `Source`. But its members are external, membership-varying cells, not a fixed internal buffer, so it is not honestly shape A either. On top of that, the trait forces `Source = Subject` — the source type must be the iterable value itself. So `Source` is forced to be a thing that stores nothing.

Second, the loop machinery freezes its source: the source is evaluated once at loop entry, held live, and may not be mutated while the iterator lives. A `yielded` group is designed to change membership at runtime. So the imported "the source won't change" contract lands on exactly the value whose point is to change.

Two coherent ways to resolve this:

**Model 1 — third Source pattern, membership snapshotted at loop entry.**

```
// A runtime `for` over a yielded group takes a snapshot of the live
// on/off bits at loop entry; walks that fixed member list to completion.
for v in group {        // members frozen at this line
    ...                 // a member joining/leaving mid-pass is NOT seen
}
```

The `yielded` group is declared a third, language-only Source pattern (neither `()` nor a backing collection): its `Source` is the group's on/off-bit view, and the "no mutation while live" freeze is honored by snapshotting membership at entry. This keeps the frozen-source invariant literally true — the walked set does not change under the loop.

**Model 2 — live membership, reactivity handles change across passes.**

```
// The loop walks live members. If membership changes mid-pass, the
// in-progress pass is atomic (unaffected); the change dirties the
// enclosing derived, which re-runs the whole loop on the next cycle.
derived total = collect { for v in group { ... } }
// member joins -> derived is dirty -> loop re-runs next cycle
```

Here nothing new is invented for the freeze: a mid-pass member join/leave is treated like any reactive update. The existing rule "reactive updates do not interrupt an in-progress loop iteration" makes the current pass atomic, and the rule "if the iterated collection is itself reactive, each update re-runs the loop" re-runs it next cycle. `Source = Subject` is satisfied because the group itself is the Source; its `next` reads the on/off bits.

**What the corpus currently says, and where it disagrees with itself.** The amendment asserts `yielded T` fulfills Iterable "in the style of Keyed/StringifiableKey" (a language-defined fulfillment) with `Item` = member values in walk order, usable in "runtime-loop contexts only." But those cited precedents (Keyed, StringifiableKey) are key-derivation traits — they do not bind a `Source` or define an iterator, so they are no template for the two unfilled slots. The Iterable section still says every iterator is shape A or shape B and forces `Source = Subject`; the lifetime section still forbids source mutation while the iterator is live. Nothing in the amendment states which Source shape the group uses, what `Iter.Source` is, what cluster the borrow-equivalent item is rooted in, or what a `for` sees when membership changes mid-pass. The reactivity section does say a reactive iterated collection re-runs the loop and an in-progress pass is not interrupted — but it never names membership change (only value updates), so it is close but not quite a stated answer.

**Recommended: Model 2 (live membership, reactivity governs change), because it invents the least and the corpus already leans this way.** The group is already a reactive cell-kind whose membership change "propagates dirt to consumers just as a value change does" — so a member join/leave is by design the same event as a value update, and the reactivity section already says such updates re-run the loop and never interrupt an in-progress pass. That makes the mid-pass answer (current pass atomic, re-run next cycle) fall out of existing rules rather than needing a new snapshot rule. It still requires one explicit statement: that the language-defined fulfillment's `Source` is the group itself (`Source = Subject`, backed by the on/off-bit view, not a collection) and its item is Copy/by-value, so the "no backing collection to root in" objection is answered by rooting the item nowhere — it is the copied member value, not a borrow into a collection.

**Consequences.**

- The two membership/undefined-term findings (the mid-iteration gap and the source-shape gap) conform automatically once Model 2's two explicit statements are added: the Source-is-Subject-by-value clause answers the "which shape / what root" gap, and the membership-change-is-a-reactive-update clause answers the mid-pass gap.
- The plausible finding (the ch12-side duplicate of the same two gaps) is **dismiss-or-pin**: its verifier already refuted it on the ground that the reactivity section covers mid-pass change and the language-defined-fulfillment clause is the Source boundary. Crux: if you accept that the reactivity rule's "reactive value" covers a membership change (not just a value change), the finding dissolves; if you require membership change to be named explicitly, it stands. Recommend pinning the explicit membership-change clause so it holds either way.
- **Still member — the "runtime-loop context" undefined-term finding:** the phrase "runtime-loop contexts" is used to scope the fulfillment but is defined nowhere; the loop machinery's own axis is compile-time-unrolled vs runtime, not "context." Residual sub-decision you must answer: **restate the scope using the existing compile-time-unrolled-vs-runtime vocabulary, or add a one-line definition of "runtime-loop context"?** Recommend restating in the existing vocabulary so no new term is introduced.

> **For execution:** Canonical side = the amendment's LOG (034-9, 034-10, 034-11, 034-12; §13.20.4.2). Conform SPEC §13.20.4.2 (SPEC ~23261-23269) to Model 2 by adding two clauses: (1) `Iter.Source = Subject` where Subject is the `yielded` group's on/off-bit view (a third, language-only Source pattern, neither `type Source = ()` per SPEC ~10893 nor `type Source = TheCollection` per SPEC ~10898), item is by-value/Copy so it is rooted nowhere (dissolving 014-29's borrow-equivalent-rooted default form for this fulfillment, LOG 1674); (2) a member join/leave is a reactive membership update per 034-10 (LOG 4374), so the in-progress pass is atomic per 014-161 (LOG 1806) and the loop re-runs next cycle per §12.13 (SPEC ~11452-11454) — reconcile with the source-freeze invariant §12.8.2 (SPEC ~11076-11078) by scoping freeze to within-a-pass. Resolves F051, F050 automatically; F044 [PLAUSIBLE] is the ch12-side duplicate — dismiss if clauses (1)+(2) land. F052 [STILL]: replace "runtime-loop contexts" in 034-12 (LOG 4376) and §13.20.4.2 (SPEC 23266) with the runtime-vs-compile-time-unrolled axis of 014-58 (LOG 1703). Depends on K-YIELDFOLD-IR (fold/yielded IR serialization, 033-80/035-9/035-10). Verify: `grep -n 'runtime-loop context' DECISION_LOG.md SPEC.md` should return zero after F052 fix; `grep -n 'Source = Subject\|language-defined fulfillment' SPEC.md` should show the new by-value Source clause at §13.20.4.2.

#### D-11 — What fence info-string convention marks a code block as non-Ductus so it is skipped by Ductus lexing? *(resolves 4 findings, 4 automatically)*

The SPEC teaches Ductus with fenced code blocks. Some of those blocks hold Ductus source. Many do not. Some show the compiler's internal IR text form. Some show its EBNF grammar. Some show compiler diagnostic output — error and hint lines. Some show host-runtime pseudocode, Rust-flavored calls like `runtime.commit();`. All of these legitimately use `;`. But Ductus itself makes a semicolon a lex error: a `;` anywhere in Ductus source is illegal. So if a tool reads every fence as Ductus and lexes it, all the non-Ductus blocks error out.

Today every fence in the SPEC is bare. It opens with three backticks and nothing after them — no info string, no language tag. So a mechanical reader has no way to tell Ductus source from IR text, grammar, diagnostics, or host code. The question is what label to write after the opening backticks so a tool knows which blocks to skip.

Option A — tag only the non-Ductus blocks; leave Ductus blocks bare. A bare fence means "this is Ductus source." A fence opened with a tag like `text` means "this is not Ductus, skip the lexer here." The Ductus IR text form gets a tag such as `ductus-ir`, the grammar gets `ebnf`, diagnostics get `text`, host pseudocode gets `rust` or `text`.

Option B — tag every block explicitly, including the Ductus ones. Ductus blocks open with `ductus`; non-Ductus blocks open with `text`, `ebnf`, `rust`, and so on. A bare fence becomes an error, because nothing should be untagged.

The SPEC's conventions section says flatly "Code examples use Ductus syntax." It gives no escape hatch — there is no notion in the spec of a block that is not Ductus. Yet the SPEC contains many blocks that are not Ductus and cannot be: IR text, EBNF, diagnostics, host pseudocode. So the SPEC contradicts itself. The stated convention claims every fence is Ductus, but the actual content includes fences that are deliberately not. Neither the decision-of-record nor the SPEC defines any info-string convention at all. Every one of the 1162 fence lines in the SPEC is bare.

**Recommended: Option A — bare-means-Ductus, tag the exceptions — because it matches how the SPEC is written today and needs the fewest edits.** All ~575 blocks in the SPEC are already bare, and the overwhelming majority of them are Ductus source. Option A keeps those correct with zero edits and only adds a tag to the handful that are not Ductus. Option B would force a `ductus` tag onto every single Ductus block — hundreds of edits — for no behavior gain over Option A. Either option makes "a fence marked non-Ductus is excluded from Ductus lex checks" mechanically enforceable, which is the actual goal.

Consequences: all four member findings are the same defect — a bare fence containing non-Ductus content that would lex-error if read as Ductus — and all four conform automatically once the convention exists and the flagged fences are tagged. The finding about the IR text form and the grammar, the finding about compiler-diagnostic hint prose, the finding about host loop/transaction pseudocode, and the finding about host write-signal pseudocode all resolve by tagging those specific blocks and adding the convention to the conventions section. No member has a residual sub-decision under Option A. The only open sub-choice, which the user can settle inline, is the exact tag vocabulary:

- **Tag vocabulary:** which info strings to standardize on. A minimal set is `text` (diagnostics and any prose-like non-Ductus), `ductus-ir` (the IR text form), `ebnf` (the grammar), and `rust` or `text` for host pseudocode. Recommend the smallest set that still lets a tool route each block correctly; the exact spellings are the user's call.

There are no plausible-only members to dismiss-or-pin; all four are CONFIRMED and low-severity.

> **For execution:** canonical side = Option A (bare fence = Ductus source; non-Ductus fences carry an info string that excludes them from Ductus lex checks). Add a new decision under topic 002 (lexing) stating the convention: bare fence info-string = Ductus source; a non-empty info string marks a non-Ductus block excluded from Ductus lexing rules including the semicolon-is-lex-error rule of LOG 002-16 (§1.4). Update SPEC §1.4 "Conventions" (line 115) to add this — its current sentence "Code examples use Ductus syntax." (line 117) is the contradicting text and must be amended to admit tagged non-Ductus blocks. Then tag the flagged fences: SPEC 24669/24725 (IR module text form §15.4.5 + IR module text grammar §15.4.6, evidence at 24670, 24726) plus the behavior-IR blocks 24530 (text form, evidence 24531) and 24605 (EBNF, evidence 24606); 1118 (diagnostic hint prose, evidence 1122) — NOTE the other two diagnostic fences originally cited as ~17905/~21402 no longer resolve to diagnostics at HEAD (those regions are now grammar-notation and stream source; re-locate against F144's evidence text, current diagnostic-prose fences include e.g. 18053 and 21578); 11680/19165 (host loop/transaction pseudocode, evidence 11682-11684, 19167-19168); 18268/18378 (host write-signal pseudocode, evidence 18270-18272, 18380-18381). Findings: F145, F144, F143, F142 (all LOW / stale_example / CONFIRMED; anchor LOG 002-16). Verify: `grep -c '^```$' SPEC.md` (currently 1178) should drop as bare non-Ductus fences get tagged; `grep -n '^```$' SPEC.md` remaining bare fences should all contain valid Ductus (no `;` outside comments/strings); confirm no bare fence contains a raw `;`.

#### D-12 — Do an effect method's `observed:` output cells count as "required cells" that block the satisfies-waiver and auto-satisfaction? *(resolves 4 findings, 3 automatically)*

In Ductus, a trait normally needs two things to be implemented for a type: a `satisfies` clause in the type body (the promise) and a `fulfill` block (the implementation). Two rules relax this. The **satisfies-waiver** drops the `satisfies` promise but keeps the `fulfill` block — meant for effect traits where you still write a `fulfill` to override a default effect body but have nothing abstract to promise. **Auto-satisfaction** drops *both* — the type gets the trait for free when every method already has a default body.

Both relaxations gate on "cells." But "cell" means two different things here, and that is the whole problem.

One meaning is a **required node/connection cell** — a structural member a trait can demand:

```
trait Draggable:
  attr enabled: bool = true    // a required cell — pins the trait to node/connection kinds
```

The other meaning is an **`observed:` contract cell** — an output channel an effect method's fulfillment must expose:

```
trait Renderable:
  effect render(value: Subject):
    observed:
      audio: stream ring[256] Sample   // an observed contract cell — a minimum output guarantee
```

These are unrelated axes. A required node cell forces the trait onto node/connection kinds. An observed cell forces nothing about kind; it is a minimum output contract on an effect method. The decision log defines "required cell" only in the first sense — the entry listing attr/const/derived/recurrent/stream plus endpoints. It calls observed cells "required *output* cells," a deliberately different phrase, and never ties them to either waiver.

**Where the two documents disagree.** The satisfies-waiver entry gates on "declares no required cells." Read on the log's own definition, that excludes observed cells — so a `Renderable` with only an observed contract still qualifies for the waiver. But the spec's waiver prose says the opposite outright: it folds "`observed:` contract cells that a fulfillment must supply" *into* the blocking "required cells" set. Same trait, two answers: the log-follower accepts `fulfill`-without-`satisfies`; the spec-follower demands `satisfies`. That is a compile-time acceptance divergence, and the spec must conform to the log.

The same split recurs one level up. The auto-satisfaction entry gates only on: requires satisfied, every method (including every effect method) defaulted, every associated type defaulted. It says nothing about observed cells. The spec's auto-satisfaction prose is likewise silent. But the spec's *waiver* reasoning treats observed cells as things "a fulfillment must supply" — apply that same logic to auto-satisfaction (which has no `fulfill` block at all) and a trait with an observed contract should *not* auto-satisfy, because nobody would supply those cells. The log never closes this.

So there is really one root question with two surfaces: **do observed cells block the waiver, and do they block auto-satisfaction?**

The two options for the waiver:

```
// Option A — observed cells do NOT block the waiver (the log's literal reading)
fulfill Renderable for Song:      // legal WITHOUT `satisfies Renderable`
  effect render(song):
    observed:
      audio: stream ring[256] Sample = ...   // the fulfill exposes the observed cells here

// Option B — observed cells DO block the waiver (the spec's current text)
Song:
  satisfies Renderable            // `satisfies` now MANDATORY because of the observed contract
fulfill Renderable for Song: ...
```

**Recommended: split the two rules — observed cells do NOT block the satisfies-waiver (Option A, reconcile the spec to the log), but they DO block auto-satisfaction. Because the waiver keeps the `fulfill` block, and the `fulfill` is exactly where observed cells get exposed and checked; waiving `satisfies` drops only a redundant promise, not the obligation.** The waiver's whole reason to exist is effect traits — the log/spec example that introduces it is `Renderable`, an effect trait *with* an observed contract. If observed cells block the waiver, the waiver fails on its own headline case and covers almost no real effect trait. Auto-satisfaction is the opposite: it drops the `fulfill` block, so there is no site that exposes the observed cells — letting such a trait auto-satisfy would leave the output contract unbacked. Blocking auto-satisfaction (but not the waiver) when an observed contract is present is the reading that keeps every contract cell backed by an actual `fulfill`.

**Consequences:**
- **F237 and F010 conform automatically.** Both are the same waiver divergence (one framed as differing program acceptance, one as an undefined term). Choosing Option A and editing the spec to stop folding observed cells into "required cells" closes both.
- **F014 conforms automatically.** It flags that the waiver has a cell-clause and auto-satisfaction does not. Once we state, in the log, that observed cells block auto-satisfaction but not the waiver, the two rules are consistent by construction and the flagged inconsistency is gone.
- **F238 — STILL. Residual sub-decision (answer inline):** *When an effect method's default body itself declares an `observed:` block that already exposes the contract cells, does that trait auto-satisfy, or is the presence of any observed contract an unconditional bar to auto-satisfaction regardless of what the default body exposes?* A default effect body can supply the observed cells, so a strict "any observed contract blocks auto-satisfaction" rule may reject cases that are genuinely self-backed. Pick the simple bar (any observed contract blocks) or the precise rule (blocks only if the default body does not expose every contract cell).
- No member is merely plausible/dismissable here; all four are CONFIRMED and land under this decision.

> **For execution:** Canonical side = LOG. Findings resolved: F237, F014, F010 (auto); F238 (still — residual on default-body-exposes-observed). Waiver: LOG 005-67 (waiver text) and 005-71 (mandatory-satisfies text) keep "required cell" in its node/connection sense per 005-49/005-50/005-59 (§3.1.7); edit SPEC §3.3.5 lines 1999-2013 to REMOVE the clause folding "`observed:` contract cells that a fulfillment must supply" into the waiver-blocking "no required cells" set (SPEC.md:2004-2005), leaving only the node/connection §3.1.7 members as blockers. Optionally add a one-line LOG note to 005-67 clarifying observed cells (005-69, "required output cells") do NOT count for the waiver. Auto-satisfaction: amend LOG 005-109 (§3.3.5) to add the observed-contract bar per the residual answer; mirror into SPEC §3.3.5 auto-satisfaction prose (SPEC.md:1972-1982), which is currently silent. Related but out of scope for this item: F013 (effect-method + node-cell jointly-unsatisfiable trait), F011 (first-param Subject modality), F015 (undefined "methodless-trait waiver"), F069/F070 (bootstrap `|>` Case-3) — do not resolve here. Verify: `grep -n 'observed:.*contract cells\|no required cells' packages/ductus-lang/docs/SPEC.md`; `grep -n '^005-67\.\|^005-71\.\|^005-109\.\|^005-69\.' packages/ductus-lang/docs/DECISION_LOG.md`; confirm the term "required cell" resolves to §3.1.7 node/connection members only in every waiver/auto-sat use.

#### D-13 — When you write `song.render()` and `render` is an effect, does ordinary dispatch grab it? *(resolves 3 findings, 1 automatically)*

In Ductus a trait can declare two kinds of methods. A `fn` method is an ordinary value method — you call it like `person.display()`. An `effect` method is different: it declares an interpretation obligation, and the language says it is invoked only as an effect — inside an `effects:` clause, or bootstrapped with the `|>` operator. So `render` on a `Renderable` trait is meant to run as an effect, never as a plain function call.

The trouble is a second machine in the language: the dispatch algorithm. When you write a bare call `f(x)` or a method call `x.f()`, the compiler searches every trait the receiver's type fulfills and collects a candidate for every trait method named `f`. That search does not look at whether `f` is a `fn` method or an `effect` method. So the two machines disagree about the same program.

Two things you could decide for `song.render()` where `render` is only an effect method:

```
// Model A — effect methods are ordinary dispatch candidates.
// x.f() collects them like any value method, so this resolves
// to Renderable::render and runs it as a plain call.
song.render()   // compiles, calls the effect method directly

// Model B — effect methods are excluded from ordinary dispatch.
// x.f() / f(x) never collect an effect method, so this finds
// no candidate and errors.
song.render()   // compile error: "render is an effect; invoke it
                //   in an effects: clause or via |>"
```

The corpus asserts both models and never reconciles them. On the model-A side, four decisions pull effect methods into ordinary dispatch: the effective-method-set rule says effect methods "enter the effective method set exactly as value methods do"; the collision rule says effect-set entries "resolve by the same rules as any other method"; the uniform-call-syntax rule says a trait method — with no carve-out — is callable in the three forms `person.display()` / `display(person)` / `Display::display(person)`; and the dispatch algorithm's step 1 collects a candidate for every trait method named `f`, then step 4 resolves it as an ordinary call. Nothing in the dispatch section even mentions the word `effect`.

On the model-B side, one line in the SPEC section on effect-kind methods says flatly that an effect method "is invoked as an effect ... not as an ordinary function call." That is the *only* restraining text, and it is a statement of intent, not an algorithm step. It supplies no dropped-candidate rule and no diagnostic. So an implementer writing the resolver has no rule that tells it to skip an effect candidate. The dispatch algorithm, followed literally, produces a legal ordinary call to `song.render()` — exactly what the intent line forbids.

**Recommended: Model B — effect methods are excluded from ordinary bare-name and method-call dispatch, and this exclusion is written as an explicit algorithm step, because the whole point of the `effect` keyword is that these methods run as interpreted effects with `observed:` contracts and effect bodies, not as plain calls; the SPEC already states that intent, and the amendment separately keeps effect methods in the *collision* namespace (so name clashes are still caught) without ever intending them to be ordinary call targets.** Model A would let any `song.render()` silently become a direct call that skips the entire effect machinery, which contradicts the reason effect methods exist.

Consequences:

- The finding that the dispatch algorithm has no step to exclude or diagnose an effect candidate conforms automatically under Model B: adding that step is exactly the fix it asks for.
- **STILL — the effective-set membership question:** Model B must decide whether an effect method stays in the effective method set (so name-collision checks still catch a `fn`/`effect` clash under one name) but is merely *unreachable* by ordinary dispatch, or whether it is removed from the set entirely. Recommended sub-answer: keep it in the set for collision purposes, exclude it only at the dispatch-candidate step, because the collision-namespace rule deliberately wants effect names to clash with value names. Confirm.
- **STILL — the `display(person)` conventional-call form:** The uniform-call-syntax decision names three call forms for "a trait method" with no carve-out. Under Model B this decision needs an explicit clause saying the three forms apply to value methods only; effect methods are reached through `effects:` / `|>`. Confirm this carve-out and where it lands.
- One of the three findings cites an example `render(song).audio |> audio_out` as a witness of the contradiction. The reviewer flagged that citation as misclassified: that example sits inside an `effects:` clause and *is* the permitted effect-invocation form, not an ordinary call. Dismiss that one citation; the contradiction still stands on the uniform-call-syntax-vs-intent half.

> **For execution:** Canonical side = Model B (effect methods excluded from ordinary dispatch). Conform these LOG entries: 005-78 (effective-method-set — qualify "enter exactly as value methods do" to mean set-membership for collision only, not dispatch candidacy), 005-114 (three call forms — carve out effect methods, value methods only), 005-120 and 005-121 (bare-name/method-call trait-impl search — add effect-method exclusion). SPEC: add an explicit exclusion/diagnostic step in §3.4.1 (currently lines 2060-2122, step 1 at 2066-2070, step 4 at 2085-2092); keep §3.1.1.1 (lines 1273-1276) as the intent statement and cross-reference it from §3.4.1; reconcile the collision-namespace bullet §3.1.1.1 lines 1268-1272 to say "collision namespace only, not a dispatch candidate." Findings: F068 (gap, auto-resolves), F033 (divergence), F067 (contradiction; drop its 017-273 citation per the charity note). Verify: grep -n 'effect' SPEC.md §3.4/§3.4.1 (lines 2028-2172) and DECISION_LOG 005-120..137 must now be non-empty; grep the new exclusion clause; confirm 005-114 carve-out text exists.

#### D-14 — When you loop over a collection, which key types may identify each item? *(resolves 3 findings, 1 automatically)*

Ductus's `repeat` construct makes one live scope per element of a collection. Each scope needs a stable key so the runtime can tell "the same item" apart across re-renders. That key also becomes part of a path string that identifies the scope (`<parent>.<key>.<field>`), so keys must be turnable into text. The language pins down which types may be a scope key with one bound called `StringifiableKey`. The problem: that bound is drawn narrower than the places that feed keys into it, so several programs the docs themselves tell you to write don't compile.

Here is the mismatch, concretely. A `Map[K, V]` may use a wide set of key types:

```
attr sessions: Map[usize, Session] = {}     // usize key — a legal Map
attr timers:   Map[instant, Timer]  = {}     // instant key — also legal
```

The rule for `Map` keys admits all integers (`i8`..`i128`, `u8`..`u128`, `isize`, `usize`), plus `bool`, `char`, `string`, `duration`, `instant`. But the `StringifiableKey` bound that gates every scope key admits only `i8`–`i64`, `u8`–`u64`, `bool`, `char`, `string`. Missing: `i128`, `u128`, `isize`, `usize`, `duration`, `instant`. So:

```
repeat (id, s) in sessions keyed by id:   // id is usize -> rejected: not a StringifiableKey
  SessionRow | info=s
```

There is no legal keying path for this loop. The explicit `keyed by` path requires a StringifiableKey; the map element is a `(K, V)` tuple, which is neither a StringifiableKey nor `Keyed`; the carried-key path is only for `dynamic` namespace sources. So the loop fails to compile with no workaround — yet another decision prescribes "project your stream into a `Map[K, V]` and repeat over it" as *the* canonical way to drive a repeat from a stream. Legal to build, impossible to loop over.

The same trap hits positional identity. The prescribed remedy for a stable positional key is to write `keyed by <index>`, and the `at <index>` binding is typed `usize`:

```
repeat item at i in items keyed by i:   // i is usize -> rejected: not a StringifiableKey
  Row | item=item
```

The one construct the docs offer for positional identity is rejected by the bound that gates it.

There are two ways to close the gap.

Model A — widen `StringifiableKey` to cover every Map-eligible, trivially-stringifiable type:

```
StringifiableKey = i8..i128, u8..u128, isize, usize, bool, char, string, duration, instant
```

The `keyed by id` and `keyed by <index>` programs above then compile as written. This matches the stated rationale for the bound — the key just has to be turnable into a path string — and `usize`, `i128`, `duration`, `instant` are all trivially stringifiable. The float exclusion stays (floats fail `Hash`/stringify because `NaN ≠ NaN`), so widening does not weaken the actual constraint.

Model B — keep `StringifiableKey` narrow and force explicit conversion at every wide-key loop:

```
repeat (id, s) in sessions keyed by id.to_u64():   // user narrows usize -> u64 by hand
```

This keeps the bound small but makes the canonical Map-repeat and positional-identity patterns require a manual cast the docs never mention, and it has no lossless target for `i128`/`u128`/`duration`/`instant` — so those Maps stay un-loopable. Model B does not actually fix the unsatisfiable cases; it only papers over the `usize` one.

What the corpus says today: it disagrees with itself. The Map key-bound rule and the SPEC's key-type list admit the wide set; the `StringifiableKey` decision and its SPEC line admit only the narrow set; and two separate decisions (canonical stream→Map→repeat, and `keyed by <index>` for positional identity) prescribe programs the narrow bound rejects. No widening or coercion rule exists anywhere to bridge them — the bound is a fixed enumerated set, and signed/unsigned crossings need explicit casts.

**Recommended: Model A — widen `StringifiableKey` to the full set of Map-eligible key types (`i8`..`i128`, `u8`..`u128`, `isize`, `usize`, `bool`, `char`, `string`, `duration`, `instant`), because the only stated reason for the bound is "the key must serialize into a path component," and every excluded type is trivially stringifiable — the narrow list reads as an oversight, not a design constraint. Widening makes the canonical Map-repeat path and the prescribed positional-identity remedy both compile, and it keeps the one real exclusion (floats, which fail `Hash`) intact.**

Consequences:
- **F166 (the Map-source wording that implies an automatic key path) conforms automatically.** Its concern was that `Map[K,V]` looks like it has an implicit key path when it doesn't; once `keyed by k` is legal for every Map key type, the wording is no longer misleading and the SPEC example (`keyed by sid`) is realizable for all key types. No separate edit needed beyond the bound change — but confirm during execution that 018-83's wording still reads correctly against the widened bound.
- **F229 (Map keys with no legal keying path) — residual sub-decision:** does widening cover *exactly* the Map key set, or a superset? If you want `StringifiableKey` and the Map key-bound to be defined as the same set (recommended, so this class of gap can't reopen), say so and we state one canonical list referenced by both. Answer inline: **same set, or independently maintained?**
- **F163 (`keyed by <index>` rejected because `usize` isn't stringifiable) — residual sub-decision:** widening admits `usize`, which fixes it directly. But confirm you don't instead want the index binding retyped (e.g. `at <index>` as `u64` rather than `usize`) — that's a narrower alternative fix touching the index type instead of the bound. Answer inline: **widen the bound (recommended), or retype the `at` index?**

> **For execution:** Canonical side = the wide Map key set; `StringifiableKey` must be widened to match it. LOG to amend: **018-64** (widen `StringifiableKey` enumeration from `i8`–`i64`, `u8`–`u64`, `bool`, `char`, `string` to `i8`–`i128`, `u8`–`u128`, `isize`, `usize`, `bool`, `char`, `string`, `duration`, `instant`); verify **018-63** (keyed-by result bound), **018-69** (`Keyed::Key: StringifiableKey`), **018-71** (stringifiable-element path) still read correctly against the widened set; verify **012-100** (Map key bound — LOG says "integers" broadly) and reconcile its wording with the widened `StringifiableKey` and with the SPEC list. Cross-check no-op cases: **018-42** (stream→Map→repeat canonical path), **018-58/018-127/018-142** (`at <index>` usize + `keyed by <index>` positional remedy), **018-44** (Map yields `(K,V)`), **018-83** (Map diffed by key identity — F166). SPEC to conform: **§13.5.4.1** (line ~15509, the `StringifiableKey` enumeration in the key-derivation precedence list) — must match amended 018-64; **§9.5.3** (line ~7443, Map key-type list) — cross-reference for the canonical set; **§15.4.1.1 / §13.5.4** path-component rationale (line ~15375) confirms the stringifiability rationale that grounds Model A. Findings: F229 [H], F163 [H], F166 [L/auto]. Verification greps: `grep -n 'StringifiableKey' SPEC.md` (all 8 sites must agree on the widened set), `grep -n '^018-6[3-4]\.\|^012-100\.' DECISION_LOG.md`, and confirm no coercion/widening rule was silently added — the fix is enumeration-widening, not a cast rule.

#### D-15 — When a dynamic view satisfies `Iterable`, does the user get a `for` loop over it — or only the operators and `repeat`? *(resolves 3 findings, 1 automatically)*

In Ductus, a dynamic view is the runtime-varying set of children a node receives when a parent feeds them in through a `repeat`. It is not an ordinary collection: its elements are weak references (`WeakHandle[T]`) to entities the caller owns, its membership changes at runtime, and its keys belong to the supplier, not the receiver. The language's general iteration rule is simple and strict: a `for` loop gets its iterator by dispatching through the `Iterable` trait, and *any* type that satisfies `Iterable` automatically gets a working `for` loop — there is no built-in per-type iteration, and no special privilege, not even for the stdlib.

The amendment declared the dynamic-view kind satisfies both `Iterable` and `IntoIterable`. It also forbids `for` over that same kind. Those two facts collide under the general rule. There are two ways out.

Model A — the view does NOT satisfy the user-surface iteration traits.
```
for x in view:        // rejected — view is simply not Iterable
view | map(...)       // but then: how do map / repeat get their elements?
```
This makes the `for` ban trivially true, but it breaks the consumers. The decision log says `repeat` reads its source and iterates it *via `Iterable::iterator`* — that is the exact mechanism `repeat` uses, and a `dynamic` collection cell is admitted as a `repeat` source directly. If the view is not `Iterable`, `repeat p in view:` has no iteration protocol to call, and the reactive operators (`map`/`filter`) lose theirs too. Model A only works if you also invent a *separate* internal iteration protocol for operators and `repeat` — new surface the corpus does not have.

Model B — the view satisfies the traits, but the traits are an *internal* consumption protocol; user-surface `for` is carved out.
```
for x in view:        // rejected — explicit carve-out for language-level kinds
view | map(...)       // works — map calls the satisfied Iterable impl
repeat p in view:     // works — repeat calls Iterable::iterator on the source
```
Here the trait satisfaction stays (so operators and `repeat` keep their mechanism), and a new rule says: a language-level kind may satisfy `Iterable`/`IntoIterable` for internal consumption while still rejecting surface `for`. This is a genuine exception to the "satisfying the trait auto-enables `for`" rule, and it must be written as one.

**Recommended: Model B (internal protocol plus an explicit surface-`for` carve-out), because the trait satisfaction is load-bearing — the decision log makes `repeat` iterate its source *through `Iterable::iterator`* and admits a `dynamic` cell as a `repeat` source, so dropping the trait would strand `repeat` and every reactive operator with no iteration mechanism, whereas the surface-`for` ban is a thin, separately motivated restriction (unstable positional identity, supplier-owned keys) that a one-line carve-out expresses cleanly.**

Consequences:
- **F255 (the core contradiction) is resolved by adopting Model B and writing the carve-out** into the general auto-dispatch rules. It does not conform automatically — it needs the new exception text.
- **F256 conforms automatically** once Model B is chosen: the false "`for` is compile-time" rationale gets replaced by the real reasons (weak-reference items, no stable positional identity, supplier-owned keys, consume-only membership) that Model B already rests on. No separate decision needed.
- **F257 — residual: does `dynamic view T` satisfy `IntoIterable` at all, and if so what does `for own` do?** Under Model B, surface `for own` is banned by the same carve-out, so this can collapse to "the traits are satisfied only as the internal protocol operators/`repeat` invoke; there is no user-surface `for`/`for own`." But you must still decide whether `IntoIterable` is even claimed: its contract moves *owned* elements out of a consumed source, and the view's items are non-owning `WeakHandle[T]` with nothing to move. Options: (a) keep both traits satisfied internally but state `for own` yields `WeakHandle[T]` by copy (weak handles are non-owning references, so "consuming" is a copy, not a move); or (b) drop the `IntoIterable` claim entirely and keep only `Iterable`, since operators and `repeat` use the non-consuming path. Pick one — the corpus underdetermines it.

> **For execution:** Canonical side = LOG (SPEC conforms). Adopt Model B. (1) Amend the general iteration rules — LOG 014-2 (line 1647), 014-125 (1770), 014-135 (1780), and 014-3 (1648) — to add an explicit carve-out: a language-level kind may satisfy `Iterable`/`IntoIterable` for consumption by operators/`repeat` while rejecting user-surface `for`/`for own`; keep 017-81 (2228) as that exception's instance. Cross-check 014-141 (1786: no stdlib privilege) — the carve-out is a language-kind exception, not a stdlib one, so 014-141 stays intact. (2) Retire the false rationale in SPEC §13.3.3.4 (lines 13771–13773, "`for` is compile-time, §12.3.7…"); replace with the weak-handle/no-positional-identity/supplier-key reasons already at 13774–13777; this closes F256 against 014-58 (1703)/014-65 (1710) and SPEC §12.3.7 (10465–10470). (3) Resolve F257: decide `IntoIterable` status for the view kind at 017-192 (2339) and SPEC §13.3.3.4 (13798), reconciling with 014-127 (1772)/014-130 (1775)/014-119 (1764); state `for own`'s behavior or drop the `IntoIterable` claim. Findings: F255 [H], F256 [M, auto], F257 [L]. Verify: `grep -n '^017-192\.\|^017-81\.\|^014-125\.' DECISION_LOG.md`; confirm SPEC §13.3.3.4 no longer cites "`for` is compile-time"; confirm the auto-dispatch entries carry the carve-out clause.

#### D-16 — Empty bundle `[]`: zero rows, or one zero-length row? *(resolves 3 findings, 2 automatically)*

A bundle is Ductus's jagged container: rows of elements, written `[...]` at placement. `chords` is a bundle of bundles — `chords.count` is the number of rows, and `chords[0].count` is the number of elements in the first row. Under the hood a bundle is stored the standard way: one flat backing buffer of all elements, plus an "offsets table" that marks where each row starts and ends. The question is what the empty literal `[]` means: does it have zero rows, or one row that happens to be empty? The two answers give different results for `[].count` and for `for row in []`.

Model A — empty bundle is **zero rows**:

```
[].count            // 0
for row in []: ...  // body runs 0 times
```

Model B — empty bundle is **one zero-length row**:

```
[].count            // 1  (one row, which is empty)
for row in []: ...  // body runs once, binding an empty row
```

The corpus asserts both at once and never pins the invariant that would decide it. The decision entry and the SPEC both say `[]` "has zero elements" (leans model A), but in the same breath say its offset table has "length 1 holding a single zero-length row" (leans model B). Separately, `bundle.count` is defined as the row count. The only thing that reconciles "zero elements" with a "length-1 offset table" is the standard CSR invariant — offsets length = rows + 1 — under which length 1 means zero rows and `[].count == 0`. But that invariant is stated nowhere: a grep of both documents finds no rule relating offset-table length to row count. So "a single zero-length row" reads literally as one row, and `[].count == 1` survives just as well. An implementer cannot tell which one compiles.

There is a second, adjacent conflict this item must clean up. The unified `.count` accessor entry says `.count` "reports the number of elements" everywhere, listing bundles among them. But the bundle-access entry defines `bundle.count` as the ROW count. For any non-empty bundle rows and elements differ, so these two LOG entries already disagree about what `bundle.count` returns — independent of the empty case. Whatever we decide for `[]`, `bundle.count` must be stated as an intentional row-count exception to the element-tally rule.

**Recommended: model A — empty bundle is zero rows, `[].count == 0`, and pin the offsets-length invariant as length = rows + 1. Because "zero elements" is the primary claim both documents lead with, model A is the only reading where `.count == 0` means "empty" (the universal expectation every branch on emptiness relies on), and length = rows + 1 is the standard CSR encoding that already makes a length-1 table mean zero rows — so it fixes the contradiction by keeping the storage sentence and rewording only the misleading "single zero-length row" gloss.**

- **F162 conforms automatically.** It only flags the surprise that `[].count` might be 1; model A makes it 0, and pinning the invariant removes the surprise. No residual.
- **F057 conforms automatically.** Its `for row in []` iteration count falls out of the row count: zero rows means zero iterations. Pinning `[].count == 0` settles it. No residual.
- **F086 — STILL: bundle `.count` vs the element-tally rule.** Model A pins the empty case, but F086's secondary observation (the unified `.count` entry says "elements," the bundle entry says "rows") is a live LOG-internal divergence for every bundle, not just `[]`. Resolve inline: should the unified-`.count` entry be amended to carve out bundles as row-counting (recommended — matches the bundle-access entry and the `chords.count // number of bundles` example), or should bundle `.count` be redefined to count elements (breaks the row-count example and the `for` unrolling)?

> **For execution:** Canonical side: model A (zero rows). LOG entries to conform: 017-89 (line 2236) — reword "an offset table of length 1 holding a single zero-length row" to "an empty offsets table of length 1, encoding zero rows (offsets length = rows + 1)"; 017-90 (line 2237) — add that `bundle.count` is the row count as an intentional exception to the element-tally unification. Resolve F086's residual by amending 012-90 (line 1292) to carve bundles out of "reports the number of elements" (bundle `.count` = rows). SPEC §13.3.3.5: lines 13835-13837 (reword "single zero-length row" to zero rows; keep "zero elements"); lines 13849-13850 (mark `bundle.count` = row count as the element-tally exception); lines 13863-13872 (state offsets-length = rows + 1 invariant explicitly, so length 1 = zero rows). Findings: F086, F057, F162. Verify: `grep -n 'zero-length row' SPEC.md DECISION_LOG.md` returns nothing after edit; `grep -n 'rows + 1\|offsets.*length' SPEC.md` finds the new invariant; confirm 012-90 no longer lists bundles under "number of elements" unqualified.

#### D-17 — What exactly is the set of instances a program reaches, and what do we call it? *(resolves 3 findings, 2 automatically)*

When a Ductus program starts, the runtime does not instantiate every node you wrote. It starts from one root instance and pulls in everything that root can reach: children, connection destinations, instances held through handles, and so on. Everything in that reachable set gets cells allocated, gets mounted, gets wired. Everything outside it is a compile error (an unused top-level instance). So this reachable set is load-bearing: it decides which instances are legal and which are dead code. The problem is that the documents describe this one set with two parallel vocabularies that are never reconciled, and three of the words in those vocabularies are used but never defined.

Here are the two vocabularies as they stand today.

Vocabulary A is the "transitive closure" of the "entry-point." This one is enumerated. The entry-point is the unique `main` placement. Its transitive closure is spelled out member by member:
```
transitive closure of entry-point =
    entry-point's own subtree
  + its connection destinations
  + its Handle/WeakHandle-reachable module-level instances
  + node references bound as effect arguments (reached as borrows)
  + its connections' wire-candidate envelopes
```
An instance not in this set is the compile error `unreachable_top_level_instance`.

Vocabulary B is the "interpretation closure" of an "interpretation root." It is defined by composition, on top of a base set that is itself never defined:
```
interpretation closure of interpretation root =
    containment closure        <- never given a definition anywhere
  + wire-candidate envelopes   <- never defined as distinct from "candidate envelope"
```
This is the set the compile-time interpretation walk recurses over; it terminates precisely because this static set is finite.

Both vocabularies describe the same idea: the static set of instances a root can reach. But nothing in either document states that they are the same set, and the two enumerations do not even match. The enumerated "transitive closure" lists five members. The prose in the spec's startup section and its reachability section lists only three (subtree, connection destinations, Handle-reachable) — it drops the effect-argument borrows and the wire-candidate envelopes. So even within Vocabulary A the membership is stated two different ways.

On top of that, three terms are used but never defined:
- "containment closure" — the base of the interpretation closure. No sentence anywhere says which instances it comprises. Is it the bare subtree? Subtree plus connection destinations plus Handle-reachable? An implementer cannot compute the interpretation closure without this.
- "wire-candidate envelope" — used in five log entries, never defined as distinct from the defined "candidate envelope" (which is: for a dynamic destination, every type the reference's static type admits; for a repeat-materialized connection, its destination's type). The charitable read is that a wire-candidate envelope is just the candidate envelope of a connection's wire endpoint, but that equivalence is never stated.
- "interpretation root" — described only by how it is *expressed* (piping a node reference into an effect-kind trait method), never by what it *is*. And its relation to the singular "entry-point" is left implicit: is a root the same thing as the entry-point, or can there be many roots under one entry-point?

There are two coherent ways to resolve this.

Option 1 — unify to one closure and one vocabulary. Declare that "interpretation closure" and "entry-point transitive closure" are the same set under two names, pick one canonical name, and give "containment closure" a definition as exactly the enumerated set minus the wire-candidate envelopes. Concretely:
```
containment closure of a root =
    root's own subtree
  + connection destinations
  + Handle/WeakHandle-reachable module-level instances
  + effect-argument borrows
interpretation closure = containment closure + wire-candidate envelopes
(and this equals the entry-point's "transitive closure")
```
Then fix the three-member spec enumerations to list all five. One set, one canonical name, every base term defined.

Option 2 — keep the two terms as genuinely distinct sets. Only viable if "containment closure" is meant to be narrower than the full reachable set (for example subtree-only, no handles), so interpretation closure and transitive closure really differ. But nothing in the corpus supports a narrower reading, and it would mean the interpretation walk and the reachability check operate on different sets — a subtle, unexplained split. This looks like accidental divergence, not design.

**Recommended: Option 1 — unify to one closure with one canonical name and a defined base, because the log itself already equates them.** The entry that defines the interpretation closure says it is "the static set of instances an interpretation root can reach," and the reachability entry says the transitive closure is what an instance must be inside to be legal — same set, same reachability semantics. The log's five-member enumeration is the authoritative membership; the spec's three-member prose is the drifted copy. "wire-candidate envelope" should be pinned as the candidate envelope of a connection's wire endpoint, matching how the log links "candidate edges" to "the same wire-candidate envelopes." "containment closure" should be defined as the reachable set minus those envelopes, so the composition `interpretation closure = containment closure + wire-candidate envelopes` holds by construction.

Consequences:
- **The two undefined-term findings (containment closure; wire-candidate envelope) conform automatically** once containment closure gets the subtree+destinations+Handle+borrows definition and wire-candidate envelope is pinned to the candidate envelope of a wire endpoint. No further sub-decision needed.
- **STILL — interpretation root vs entry-point (the third finding):** unifying the closures does not by itself settle whether one program has one root or many. The bootstrap form lets any node be piped into an effect-kind method, which reads like "many roots under one entry-point," but the reachability rule speaks of a single entry-point. **User sub-decision: is an interpretation root exactly the entry-point (one per program), or is every node bootstrapped in an `effects:` clause a root, so multiple roots live under one entry-point?** This changes what "one render mounted per (root, instance) pair" ranges over.
- **Pin-or-dismiss — the three-member vs five-member spec enumerations:** if Option 1 is taken, the spec startup section and reachability section must be edited from three members to five, or explicitly stated as a deliberate loose paraphrase. Verifier crux: does any legal instance reachable *only* via an effect-argument borrow or a wire-candidate envelope exist? If yes, the three-member spec text is a real soundness bug, not just wording.
- **Pin — the broken cite:** the spec's interpretation-closure sentence cites the Effects section for the closure definition; that section does not define it. Re-point to the reachability/interpretation-closure section.

> **For execution:** Canonical side = the DECISION_LOG five-member enumeration (021-140, 021-141). Give "containment closure" a definitional statement = subtree + connection destinations + Handle/WeakHandle-reachable module-level instances + effect-argument borrows (the reachable set minus wire-candidate envelopes), so that 017-271's `interpretation closure = containment closure ∪ wire-candidate envelopes` holds by construction and equals 021-140/141's transitive closure. Pin "wire-candidate envelope" ≡ the candidate envelope (017-119) of a connection's wire endpoint; confirm against 024-17's "candidate edges ... the same wire-candidate envelopes." Conform LOG 015-16, 017-271, 021-140, 021-141, 024-17 to one canonical closure name. Add/repair SPEC: §13.1 (near L11531-11551 context), §13.3.8 (017-271), §13.8.1 (SPEC L16613-16621 three-member enum → five, or mark as loose paraphrase; §13.14.1 L19050-19053 same), §13.11.5 (024-17), and the interpretation-closure sentence at SPEC L523-525 whose cite §13.19 is broken (re-point to §13.8.1 / §13.3.8). STILL sub-decision: define "interpretation root" and its relation to the singular "entry-point" (021-140 §13.8.1) — equal, or many-roots-under-one-entry-point; anchor SPEC L20087-20093 bootstrap form (§13.17.7 Case 3). Findings: F231 (containment closure, MED), F232 (wire-candidate envelope, MED), F234 (interpretation root, LOW). Verify: `grep -n 'containment closure' DECISION_LOG.md SPEC.md` must return a definitional statement, not only usages; `grep -n 'wire-candidate envelope' DECISION_LOG.md` sites must all resolve to the pinned definition; `grep -n 'transitive closure\|interpretation closure' SPEC.md` enumerations must all list five members or cite the canonical definition; confirm SPEC L525 no longer cites §13.19 for the closure.

#### D-18 — When a stream is passed to a value-reading operator parameter and read as a value, is it rejected at the read site or consumed one-event-at-a-time? *(resolves 2 findings, 1 automatically)*

An operator in Ductus can take a value-reading parameter. That parameter is annotated `cell T` — the umbrella kind that any value cell (a signal or a derived) satisfies. Inside the body the parameter is read for its current value, like `source * rate`. A stream also fits the umbrella spelling in some positions, but a stream has no "current value" — it is an append-only sequence of events. So the question: what happens when someone binds a stream to that value-reading parameter and the body reads it as a value?

Two models exist in the corpus, and they answer differently.

Model A — reject at the read site. The stream never becomes a legal value read. The body's attempt to read it as a value is a compile error.
```
operator smooth(source: cell f32, rate: f32) -> derived f32:
    derived out = source * rate   // `source` bound to a stream -> ERROR here
```
The diagnostic is the general "cannot read a `stream T` as a value" — streams have no current value; project to a signal first via `to_signal`.

Model B — consume per event. The body is treated as a reactive expression over a stream, so `source * rate` re-evaluates once per incoming event and yields one output event per input event.
```
operator smooth(source: cell f32, rate: f32) -> derived f32:
    derived out = source * rate   // re-fires per event; output is really a stream
```
Under this reading the operator's declared `derived f32` return is a lie — the body produced a stream, not a value cell.

What the corpus says, and where it fights itself. The value-cell-kind section states plainly, in three places, that a stream bound to a value-reading parameter "has no current value, so it is excluded at the read site rather than by the annotation" — that is Model A. But the stream-expression section says, for any reactive expression, "a stream contributes its events; the surrounding expression is re-evaluated once per event, producing one output event per input event" — that sounds like Model B, and the audit flagged the two passages as reading in opposite directions for `source * rate`. There are two more gaps on top of the clash. First, Model A is asserted but never made operable: the phrase "excluded at the read site" names no concrete rule or diagnostic for an operator body, and the one real "stream read as a value" diagnostic in the corpus targets a bare stream in a `derived` binding (`derived latest = events`), not a stream-bound `cell T` parameter read inside an operator. Second, if Model B were the intended reading, nobody has stated when the event is sampled for the derived-returning-operator case.

The clash is only apparent, and the tie-breaker is already in the corpus. The per-event model does not actually apply here. The rule that governs it says a reactive expression containing a stream "has stream result kind, and its surrounding declaration must be `stream` or `recurrent[N] stream`." The operator above returns `derived f32` — a value-cell context, not a `stream` declaration. So the per-event model refuses to license this body just as much as Model A does: there is no legal `stream`-typed home for the result. The two passages disagree only if you ignore the surrounding-declaration constraint; once you honor it, Model A is the sole coherent answer and Model B never gets off the ground.

**Recommended: Model A — reject at the read site, and make the rejection operable, because the corpus already states the exclusion three times and the per-event model self-excludes here (it requires a `stream`/`recurrent[N] stream` surrounding declaration, which a `derived`-returning operator does not provide).** The only real defect is that Model A is asserted without a mechanism. Pin it: reading a stream-bound `cell T` parameter as a value inside an operator body fires the "cannot read a `stream T` as a value" diagnostic, add that as a named operator-diagnostic class, and cross-reference it from the read-site-exclusion text. Choosing Model A also makes the sampling-moment question vacuous — there is no legal per-event read to time — which is exactly why the second finding dissolves automatically.

Consequences:
- **F129 conforms automatically (dissolves).** It asks where the sampling moment is specified for the stream-into-value-reading-operator case. Under Model A that construct is a compile error, so no legal program reaches a point where an event-consumption cadence must be defined. The audit's own verifier reached the same conclusion (charity: refute). Adopting Model A means nothing to specify here — the gap is vacuous.
- **F129 residual (plausible; dismiss-or-pin).** The finding is PLAUSIBLE, not confirmed. Verifier's crux: it presupposes the construct is legal; if the owner instead rules the construct legal (Model B), the sampling-moment gap becomes real and §13.18.7-vs-§13.17 must be reconciled. So: dismiss F129 iff Model A is adopted; it only survives under Model B.
- **F128 residual (still open under Model A):** *Which concrete mechanism carries the read-site rejection inside an operator body?* Two operable spellings — (a) reuse the existing general "cannot read a `stream T` as a value" diagnostic and have auto-deref reject a stream at a T-expecting position inside the body, or (b) add a dedicated operator-diagnostic class in the operator-diagnostics enumeration. Recommend (a) plus a cross-reference, because the general diagnostic already exists and states the right hint; a new class is only warranted if the operator-body message needs to differ. **Answer inline: (a) reuse-and-cross-reference, or (b) new operator-diagnostic class?**

> **For execution:** Canonical side = Model A (read-site exclusion), Model B does not apply because §13.18.7.3 requires a `stream`/`recurrent[N] stream` surrounding declaration. LOG entries that already state Model A and stand as-is: 029-124 (line 3509, §13.17.3), 016-283 (line 2142). Model B entries that stay scoped to `stream`-declaration contexts and do NOT need change (they already carry the surrounding-declaration constraint via 030-82): 030-64 (3576), 030-65 (3577), 030-66 (3578), 030-82 (3594, §13.18.7.3). 029-98 (3483, single-commit) unaffected. SPEC: read-site-exclusion text at §13.18.5 (SPEC 20723-20725) and the value-cell-kinds list (SPEC 12572-12575); per-event model at §13.18.7.1 (SPEC 20792-20796); surrounding-declaration rule at §13.18.7.3; existing diagnostic at SPEC 21919-21928; operator-diagnostics enumeration at §13.17.12 (SPEC 20297). Fix for F128: add/point a concrete diagnostic for reading a stream-bound `cell T` param as a value in an operator body (recommend reuse of SPEC 21922 "cannot read a `stream T` as a value" + cross-ref from §13.18.5 / 029-124), OR add a §13.17.12 class. Findings: F128 (MED/divergence/CONFIRMED) — pin mechanism, stays open pending F128 residual (a)/(b). F129 (MED/gap/PLAUSIBLE) — dismiss under Model A (construct rejected upstream). Verification greps: `grep -n 'excluded at the read site' SPEC.md` (expect 3 hits: ~20725, ~12575, LOG 029-124); `grep -n 'cannot read a `stream T` as a value' SPEC.md`; `grep -n 'surrounding declaration must be' SPEC.md` to confirm §13.18.7.3 constraint intact. Corpus pin: HEAD 1f204c473a.

#### D-19 — When a stream "completes" (after take), does the consumer keep draining or freeze the buffer, and what does downstream see? *(resolves 2 findings, 1 automatically)*

Ductus streams are bounded buffers read through per-consumer cursors. Each consumer keeps a cursor at the oldest event it has not yet read. For a `gate` stream, retention is slowest-cursor-driven: no event is evicted until every cursor has passed it, and when the buffer fills with events the slowest cursor has not read, the producer is rejected (`rejected_total` climbs, `is_full` becomes true). One operator, `take(n)`, introduces an idea the rest of the stream model never defines: after emitting `n` events its output stream is "complete and emits no more." Nothing says what "complete" means for the consumer's cursor on the source, for downstream operators, or for the observation cells.

The gap has two faces, and the narrow one is a special case of the broad one.

The narrow question (the gate case, F082). A `take(3)` consumer reads three events off a gate stream, then stops. Does its cursor on the source keep advancing?

```
stream gate writes = ...          // lossless gate stream
let sample = writes |> take(3)    // reads 3 events, then "complete"
// after event 3: does sample's cursor on `writes` keep moving, or stop at 3?
```

- Model FREEZE — the cursor stops at position 3. Since `writes` is a gate and retention is slowest-cursor-driven, the completed `take` cursor becomes the permanent slowest consumer. The buffer fills behind it and never drains: `rejected_total` climbs forever, `is_full` sticks true, every producer push is rejected for the life of the stream.
- Model DRAIN — the cursor keeps advancing past the events it no longer emits, releasing its retention hold. The gate buffer drains normally and producers are never blocked by a consumer that is done.

The broad question (F201). "Complete" is also undefined for everything else that touches a stream:

```
let merged = merge(a, b)     // if arm `a` completes, does merged end, or keep emitting `b`?
let n = sample |> event_count  // does a derived built on a completed stream freeze its value?
sample.is_full                 // does an observation cell reflect completion at all?
```

Merge (the operator that interleaves two arms in commit order) has no rule for a completed arm. Derived consumers like `event_count`, `to_signal`, `any`, `all` have no rule for a completed source. The observation cells (`pending_count`, `is_full`, `rejected_total`) have no "completion" reading.

**What the corpus currently says.** The stream-completion state exists in exactly one place — the `take` operator, in both LOG and SPEC — and its wording ("the output stream is complete after n events") describes only *output emission*. The consumer's cursor on the *source*, downstream operators, and observation cells are all silent. There is a related but distinct mechanism: freeze-and-backlog, which fires when a consumer's enclosing subtree is *gated off*. That model already spells out the FREEZE outcome for a gate consumer (it pins the buffer and back-pressures producers) and already provides the DRAIN escape hatch: `@reset_on_reopen` releases the buffer hold so a frozen gate consumer stops pinning. But gate-off is a different trigger from logical completion — a gated-off consumer is expected to *resume and drain its backlog*; a completed consumer never resumes. So the freeze rules do not cover completion, and the corpus does not contradict itself here; it simply leaves a hole.

**Recommended: DRAIN — a completed consumer's cursor keeps advancing and releases its retention hold, and "complete" means only "this stream emits no more events downstream," because** completion is permanent (a completed `take` cursor never resumes to drain a backlog, unlike a gated-off one), so applying FREEZE would let one finished `take(3)` silently and permanently jam every producer on a shared gate stream — `rejected_total` climbing forever with no live consumer to blame. The language already has the exact release mechanism it needs (`@reset_on_reopen`'s buffer-hold release), so DRAIN is the freeze model's escape hatch made automatic for the one case where the consumer is provably done. Making "complete" a purely downstream-emission property (not a special observation-cell state) keeps completion from leaking into the reactive graph as invisible frozen values.

**Consequences.**
- F082 (the gate case) conforms automatically once DRAIN is the rule: a completed consumer advances and releases, so producers are never permanently rejected.
- F201 is the STILL member — DRAIN answers the cursor-retention half, but three downstream sub-decisions remain open and must be pinned in the same rule:
  - **Merge arm completion:** when one merge arm completes, does `merge` end its whole output, or keep emitting the surviving arm(s) and end only when all arms complete? (Recommended default: keep emitting until all arms complete — matches interleave-in-commit-order intent.)
  - **Derived-consumer behavior:** do derived consumers built on a completed stream (`event_count`, `to_signal`, `any`, `all`, `accumulate`, `scan`) freeze their last value, or is completion invisible to them (they simply stop receiving events)? (Recommended: invisible — they hold their last committed value because no new events arrive, no special state.)
  - **Observation cells on completion:** do `pending_count`/`is_full`/etc. get a completion reading, or is completion never surfaced through observation cells? (Recommended: never surfaced — completion is not a buffer-pressure fact.)

> **For execution:** Canonical side = DRAIN + downstream-only "complete". Add a new LOG decision in §13.18.12 (consumer cursors) stating that a logically-completed subset consumer (`take`, and any operator that stops emitting while its source lives) continues to advance its source cursor and releases slowest-cursor retention — mirroring the `@reset_on_reopen` gate release (030-197) but automatic and permanent, and explicitly distinct from the gate-off freeze trigger (030-192/194, SPEC §13.18.12 lines 21660–21674). Add a companion LOG decision in §13.18.9 defining stream "complete" as a downstream-emission property only, with the three F201 sub-answers: merge arm completion (relates 030-143), derived-consumer completion (relates 030-130/137/139/140/149), observation-cell non-surfacing (relates 030-53/55/57). Update SPEC take operator (§13.18.9, lines 21244–21254) to cross-reference the new completion rule, and SPEC §13.18.12 to add the completed-consumer cursor paragraph. Anchor LOG: 030-134 (line 3646), 030-143 (3655), 030-189 (3701), 030-190 (3702), 030-192 (3704), 030-194 (3706), 030-197 (3709). Findings: F082 (report line 2405), F201 (report line 2502). Verify: `grep -niE 'stream is complete|is complete|completion' SPEC.md` should return a defined completion rule, not just the take comment at line 21246; `grep -n 'take\[' DECISION_LOG.md` cross-references the new rule.

#### D-20 — When a `match` runs inside reactive structure, does it freeze the arms it didn't pick, or throw them away? *(resolves 2 findings, 2 automatically)*

In Ductus, `match` and `given` look almost the same on the page — same arm shape, same exhaustiveness rule — but they do opposite things at runtime. `match` is the *value* selector: it picks one arm, produces that arm's value, and discards the others. `given` is the *structure* selector: it builds every arm and freezes the ones that aren't active, so an inactive subtree keeps its cells and can snap live later. That freeze-don't-discard behavior is the language's core runtime model (the runtime-semantics section calls it "Model B — frozen-when-gated, snap on activation"). The whole corpus repeats one boundary: `if`/`match` never gate structure; only `given`/`when` do.

The problem is one LOG entry that puts `match` on the wrong side of that boundary when a `match` sits inside reactive structure. Two models are in play.

Model A — match-freezes-like-given:
```
# the match-as-value-selector entry says:
# "in interpretation context a match on a reactive scrutinee
#  lowers to given semantics (it builds all arms and freezes unselected ones)"
```
Here a reactive-scrutinee `match` behaves exactly like `given`: every arm is constructed, the unpicked arms are frozen (not discarded), their cells persist and can snap live. `match` gains structure-gating power.

Model B — match-selects, structure-keeping stays `given`'s job:
```
# the very next entry says:
# "selecting which subtree is exposed and kept live is the role of given, not match;
#  a match over .exposition entries compiles to a static unroll for static entries
#  and a single mount-time tag over the closed candidate envelope for dynamic ones,
#  while live-subtree selection remains given"
```
Here `match` never freezes arms. A compile-time-known scrutinee is unrolled at build time — only the chosen arm ever exists. A dynamic one gets a single mount-time tag choosing among the closed set of candidates; keeping a subtree alive so it can snap back is left to `given`.

These two models cannot both hold, and they sit in adjacent LOG entries. Model A hands `match` the freeze-all-arms behavior; Model B explicitly denies `match` that behavior and reserves it for `given`. The overlap is real, not theoretical: exposition entries carry reactive kinds, so a `match` over exposition entries *is* a match on a reactive scrutinee — exactly the case Model A's unqualified rule claims. There is no text carving exposition out of the Model A rule.

The SPEC does not rescue either side. Both LOG entries cite the same SPEC section, but that section is the exhaustiveness-checking text, and it says only that `match` discards unselected arms and that build-all-and-freeze is `given`'s role, not `match`'s — it describes Model B's *boundary* but never elaborates either lowering. The phrase "interpretation context" appears nowhere in the SPEC. So Model A's freeze lowering has no SPEC elaboration at all, and Model B's static-unroll / mount-time-tag mechanism has no SPEC elaboration either. This is the second finding: the LOG claims a lowering the cited SPEC section flatly contradicts.

**Recommended: Model B (match selects/unrolls; it never freezes arms), because the entire rest of the corpus already commits to it and only the one parenthetical breaks ranks.** Three independent places say match never gates structure: the `collect`/`yield` rule states outright "`if`/`match` never gate structure"; the given-block section says match "returns a value and discards unselected arms" while given "builds all arms and freezes the inactive ones"; and the cited exhaustiveness section says the same. Model A's "match lowers to given semantics (builds all arms and freezes)" is the lone outlier — it would give `match` a structure-gating power the language spends three sections denying it. Fixing the outlier restores one story; keeping it forces edits to at least three other decisions.

Consequences:
- Both member findings conform automatically under Model B. The contradiction finding (F018) is resolved by deleting Model A's freeze parenthetical so only Model B's account of interpretation-context match survives. The LOG-SPEC divergence finding (F017) is resolved the same way — once the freeze parenthetical is gone, the LOG matches the cited SPEC section (match discards; given freezes), and no SPEC change is needed to reconcile them.
- **Residual — does Model B's static-unroll / mount-time-tag lowering need SPEC elaboration?** The recommendation deletes the wrong lowering but the *surviving* lowering (static unroll for static entries, single mount-time tag over the closed candidate envelope for dynamic ones) is stated only in the LOG; no SPEC section spells it out. Decide inline: (a) add a SPEC subsection under the exhaustiveness or exposition text elaborating the unroll/tag mechanism, or (b) leave it LOG-only for now as a known documentation gap. This is a scope call, not a semantics call.
- **Residual — keep or drop the "interpretation context" phrasing?** The term is load-bearing in these two entries but defined nowhere in the SPEC. Decide inline whether to (a) define it in words in the surviving entry ("when a match appears inside reactive structure that is being built, i.e. exposition/placement bodies") or (b) replace it with a plain description. Terminology only; does not change the recommended lowering.

> **For execution:** Canonical side = Model B (interpretation-context `match` selects/unrolls; it never freezes arms; structure-keeping is `given`). LOG: entry 009-89 (DECISION_LOG.md:1028) — delete the clause "in interpretation context a `match` on a reactive scrutinee lowers to `given` semantics (it builds all arms and freezes unselected ones), but the value-selection meaning is unchanged"; keep the value-selector definition (evaluates scrutinee, selects one arm, discards rest). LOG entry 009-90 (DECISION_LOG.md:1029) survives unchanged as the canonical interpretation-context lowering. Both cite §6.2.5; note §6.2.5 is actually the "Exhaustiveness checking" heading at SPEC.md:5377, with the match-vs-given text at SPEC.md:5395-5403 — verify the citation still resolves, or repoint to §6.2.4 (Pattern matching, SPEC.md:5307) if the intended target is the definition rather than exhaustiveness. Corroborating corpus (do not edit; they already match Model B): 034-7 (DECISION_LOG.md:4371) "`if`/`match` never gate structure"; §13.9.13 given-block (SPEC.md:18186-18187) "match returns a value and discards unselected arms; given builds all arms and freezes the inactive ones (Model B, §13.9.7)"; §13.9.7 runtime semantics (SPEC.md:17812). Findings resolved: F018 (contradiction, CONFIRMED), F017 (divergence, CONFIRMED). Verification greps: `grep -n 'lowers to .given. semantics\|builds all arms and freezes unselected' DECISION_LOG.md` (must return nothing after edit); `grep -rn 'interpretation context' SPEC.md` (currently empty — if residual (a) chosen, this becomes the anchor); `grep -n 'never gate structure' DECISION_LOG.md` (must still return 034-7).

#### D-21 — What is the one normative name for the order a fold's members are combined in? *(resolves 2 findings, 1 automatically)*

A `fold` combines a group of member values pairwise: the `by:` combiner runs on pairs, and the fold's cost rule promises O(log n) work per membership change by combining over a fixed binary tree. That tree is "fixed by member order," so the order the members sit in is load-bearing: it is what makes the tree deterministic across membership churn, and it is the order the values reach the combiner in. A fold's members come from two sources — a `yielded` group (produced by `collect`) or the slots of a reactive composite. Today the language names this single ordering three different ways depending on which source you look at, and never says the three names mean the same thing.

Here are the three terms as they appear now:

```
# yielded-group source — the group is "ordered, walk-order"
collect: yield a; yield b         # order called "walk order" (yield positions)

# the fold cost rule — the combine tree is "fixed by member order"
fold group: by: combine else: 0   # order called "member order"

# composite source — members are "the slots in declared order"
fold composite: by: ... else: ...  # order called "declared order"
```

All three describe the same thing: the sequence in which a fold's members get combined. But nothing states that walk order, member order, and declared order coincide. A reader trying to confirm the fold tree is deterministic hits a wall — the cost rule says "member order," but for a yielded source that order is called "walk order" and for a composite source it is called "declared order," and no entry bridges them. Worse, the one term the cost rule most depends on, "walk order," is never defined in the decision-of-record at all: its only definition ("the lexical/structural order of `yield` positions, with `repeat` members interleaved in key order") lives only in the SPEC. So the load-bearing order is spelled three ways, and one of those three spellings has its sole definition outside the LOG.

Two ways to fix this:

**Model A — one flat term, no hierarchy.** Pick a single name (say "member order"), define it once in the LOG, and rewrite the yielded and composite entries to use that one name. "Walk order" and "declared order" disappear as separate vocabulary.

```
member order := for a yielded group, the lexical order of yield positions
                (repeat members interleaved in key order);
                for a composite, the declared slot order.
fold ... : combines over a deterministic tree fixed by member order.
```

**Model B — one pivot term defined in terms of the two source-specific terms.** Keep "member order" as the term the fold cost rule leans on, and define it as: for a yielded source it *is* walk order, for a composite source it *is* declared slot order. Walk order and declared order survive as the per-source spellings; member order is the umbrella that unifies them.

```
walk order    := lexical order of yield positions (repeats in key order)   # yielded groups
declared order := the composite's slot declaration order                   # composites
member order  := walk order for a yielded fold, declared order for a composite fold
```

The current corpus does not choose. Three entries use "member order" for the determinism-fixing order (the cost rule, the fold-cell member-edge entry, and the fold-cell IR entry). One entry calls the yielded group's order "walk order," and one entry calls the composite's order "declared order." None of the five defines its term, and none states any two are equal. Note this depends on the fold/yielded IR shape being settled first (the decision that pins how a fold cell stores its member edges) — whatever that decision calls the ordered edge list must use the same term chosen here.

**Recommended: Model B (member order as the pivot, walk order and declared order kept as the two source-specific definitions), because the corpus already spells the two sources differently for a real reason — a yielded group's order comes from `yield` positions with repeat-key interleaving, a composite's order comes from slot declarations — and collapsing both into one flat definition (Model A) would either lose that source-specific detail or cram two unrelated derivations under one name. Model B lets the fold cost rule keep saying "member order" (its existing wording is untouched), while giving that term one home that resolves cleanly to walk order or declared order per source.**

Consequences:
- **The composite side conforms automatically.** "Declared order" already means the slots in declaration order; under Model B it simply becomes the composite branch of member order's definition. No rewrite of its meaning, only a stated equivalence.
- **STILL — where does "walk order" get its LOG home?** The term is used across the yielded-group definition and its `Iterable` fulfillment but defined nowhere in the LOG. Its definition must move into a LOG decision. Sub-decision to answer inline: **put the walk-order definition in the yielded-group section (034), or in the fold section (035) alongside member order?** Recommend the yielded section, since walk order is a property of the group, not of the fold that consumes it.
- **STILL — is "walk order" (yielded yield-positions) the same order as the exposition "entry/structural order" used for connection engagement?** The audit asks whether they coincide. They are different structures (yield positions inside a `collect` vs. exposition entries in a placement tree), so the safe answer is to state they are distinct orders that share the "structural source position" intuition, not to equate them. Sub-decision to answer inline: **explicitly state walk order ≠ exposition entry order (recommended), or investigate whether one can be defined via the other.**
- **F215 (plausible → pin):** the verifier's crux is that once member order is defined as walk-order-or-declared-order, the cost rule's "deterministic tree fixed by member order" has exactly one referent per source, and F215's three-terms-with-no-equivalence defect dissolves.

> **For execution:** Canonical side = Model B. LOG entries to conform: 035-5 (line 4385, "fixed by member order" — keep term, now defined), 035-6 (line 4386, "declared order" — restate as the composite branch of member order), 034-8 (line 4372, "walk-order" — add/point to a LOG definition), 034-12 (line 4376, "member values in walk order"), 035-10 (line 4390, "member edges in member order"), 033-81 (line 4200, "member edges listed in member order"). New LOG decision needed: define member order (pivot) and home walk order in section 034; check numbering places it by topic (034 for walk order, 035 for member order). SPEC sections: §13.20 (walk-order def currently at SPEC 23232-23234), §13.21 (fold/member order). Compare with 017-255 (line 2402) / 017-264 (line 2411) "entry order / structural (entry) order" — state distinct, do not equate. Findings: F212 [M/undefined_term, walk order defined only in SPEC], F215 [L/vague_term, three terms no equivalence]. Depends on: K-YIELDFOLD-IR (fold-cell member-edge shape must use the same term). Verify: `grep -n 'member order\|declared order\|walk order\|walk-order' DECISION_LOG.md` should show every hit resolving to the single defined term; `grep -n 'walk order' DECISION_LOG.md` must return a defining entry, not only 034-8/034-12.

#### D-22 — Do streams drop out of the `cell` umbrella at the signature or at the read site, and what does a bare stream name denote? *(resolves 2 findings, 2 automatically)*

In Ductus, `cell T` is the umbrella kind: any reactive cell carrying `T` fits it — signals, deriveds, recurrents, and, deliberately, streams. But a stream differs from the value cells in one hard way: it has no current value, only an append-only sequence of events. So every rule written against the umbrella owes an answer to "and what if the cell is a stream?" Two umbrella rules never answered it cleanly. The *binding* rule — what a `cell T` parameter admits — has one entry saying value cells only while two siblings say any cell, streams included. The *read* rule — what a bare cell-name in a reactive expression does — promises a current-value read for every cell-name, a promise a stream cannot keep.

The signature half. One operator-parameter entry says a value parameter binds "any reactive *value* cell" — under it, passing a stream is rejected at the signature. Its two sibling entries, and the umbrella section of the SPEC, say the opposite: the annotation admits any cell, and a stream falls out only at the read site.

```
operator monitor[T](source: cell T) -> derived bool
monitor(events)               // signature model: ERROR at the call — not a value cell
                              // read-site model: legal — monitor never reads a current value
```

The read-site model is the only one that lets `monitor` exist: an operator that never reads its parameter as a value works fine on a stream, and the SPEC's own umbrella example says exactly that. More to the point, this half is already decided: the stream-read-site decision earlier in this document (D-18) adopted the umbrella-at-signature / reject-at-the-read-site model as canonical, and the cell-spelling decision at the top of this document (D-01) explicitly flagged this "value cells only" entry as the one behavioral outlier its rename could not fix and deferred it here. So no options — this half collapses into that earlier decision, and the only work left is mechanical: rewrite the outlier entry to match its two siblings.

The read half, which the earlier decision does not touch. The umbrella read rule says every cell-name in a reactive expression resolves to two things: a provenance entry (a dependency edge for the enclosing expression) and an auto-deref'd current-value read at the name's position. Take that over the umbrella's full membership and it asserts a value read for a *stream* name. The corpus says otherwise, in three directions at once:

```
derived y = s + 1             // signal: dependency edge + current-value read — rule holds
derived latest = events       // stream in a value context: ERROR "cannot read a stream T as a value"
stream doubled = events * 2   // stream in a stream context: contributes events, re-fires per event
```

And in an observe arm body a bare stream name denotes nothing at all — only the `as` binder carries the event. So the rule's first half (dependency edge) is true for streams; its second half (value read) is never true for them. Two ways to fix the wording:

**Option A — narrow the rule's subject to value cells.** "A *value-cell* name in a reactive expression resolves to a provenance entry and an auto-deref'd read." Streams drop out of the rule's scope; their story lives only in the stream section. Cheap, but then no single place says what a bare stream name resolves to, and the narrowing reads as an omission next to the membership entry that says streams are cells.

**Option B — keep the umbrella subject, split the delivery by kind.** The name always resolves to a dependency edge; the inserted read is kind-specific:

```
# cell-name -> dependency edge for the enclosing expression   (all cell kinds)
# inserted read:
#   value cell -> auto-deref'd current-value read
#   stream     -> never a value read: events, when the surrounding declaration
#                 is stream; the "cannot read a stream T as a value" error
#                 anywhere a current value is required
```

**Recommended: confirm the read-site side for the signature half (it folds into the stream-read-site decision; conforming the outlier entry is mechanical), and Option B for the read half, because the dependency-edge half of the rule is genuinely umbrella-wide — a stream in a legal stream expression is a real dependency that re-fires the expression per event — and Option A would throw away that true half along with the false one. Option B also mirrors the carve-out sentence the parameter rule already carries in the same SPEC section ("a stream has no current value, so it is excluded at the read site, not by the annotation"), so both umbrella rules end up telling the story the same way.**

Consequences:
- **The signature finding conforms automatically (F243).** Rewrite the outlier entry (016-172) from "any reactive value cell" to its siblings' wording: binds any cell; a stream is excluded at the read site, not by the signature. This is a confirmation of the earlier stream-read-site decision, not a new choice, and it closes the caveat the cell-spelling decision left open. Do the spelling fix (`Cell[T]` to `cell T`, owned by that decision) and this rewording in one edit to avoid touching the entry twice.
- **The denotation finding conforms automatically under Option B (F245).** The umbrella read entry (016-64) gains the kind-split carve-out, cross-referencing the existing bare-stream-read error and the per-event stream-expression rule. No new semantics — every branch of the carve-out already exists in the corpus; the fix only stops the umbrella rule from over-claiming.
- **Caveat — do not flatten the observe rule.** The carve-out must say a stream name contributes events only in stream-result expression contexts. In an observe arm body a bare stream name never denotes the event (the binder does), and in the `where` filter it keeps its event meaning. Those two rules stand; the carve-out points at them, it does not restate them.
- **Editorial call, answer inline:** amend the umbrella read entry in place, or keep it one sentence and add a companion entry beside it for the stream branch? Recommend amend in place — the carve-out is one sentence, and a separate entry invites the same drift this decision is cleaning up.

> **For execution:** Canonical side = umbrella-at-signature / excluded-at-read-site (confirms D-18 Model A) + Option B kind-split on the umbrella read rule. LOG edits: 016-172 (line 2031, §13.2.8) — rewrite "binding to any reactive value cell" to the wording of 016-283 (2142) / 029-124 (3509): binds any reactive cell; a `stream T` is excluded at the read site, not by the signature; coordinate with D-01's spelling pass (its list already includes 016-172, 016-283, 030-48). 016-64 (line 1923, §13.17.3.1) — add the stream carve-out: dependency edge for all kinds; auto-deref'd cell-pool read for value cells only; a bare stream name is the 030-246 error (3758, §13.18.16) in value contexts and contributes events per 030-64/65/66 (3576-3578, §13.18.7/.1) under the surrounding-declaration constraint 030-82 (3594, §13.18.7.3). Stand as-is: 016-62 (1921), 016-165 (2024), 016-283 (2142), 029-124 (3509), 030-48 (3560, spelling only per D-01), 030-246 (3758). SPEC: parameter carve-out already correct at 12571-12575; add the matching carve-out sentence to the cell-name text at §13.2.8 (12606-12616, currently value-cell examples only); umbrella + monitor example §13.18.5 (20681-20710) and read-site exclusion (20723-20726) unchanged; diagnostic at 21919-21928; observe arm-body rule at 13089-13091 unchanged, cross-reference only. Depends on: D-18 (read-site model), D-01 (spelling of the same entries). Findings: F243 (MED/contradiction/CONFIRMED, report line 125) — auto after confirmation; F245 (LOW/divergence/CONFIRMED, report line 498) — auto under Option B. Verification greps: `grep -n 'any reactive value cell' DECISION_LOG.md` (zero after edit; currently exactly 016-172); `grep -c 'excluded at the read site' DECISION_LOG.md` (2 now, 3 after — 016-172 joins 016-283/029-124); `grep -n "auto-deref'd cell-pool read" DECISION_LOG.md` (016-64 must carry the carve-out); `grep -n 'never denotes an event value' SPEC.md` (observe rule intact). Corpus pin: HEAD 1f204c473a.


#### D-23 — Does the entry-point's live-graph closure have five members or three? *(resolves 2 findings, 2 automatically)*
When a Ductus program starts, the compiler works out which top-level node instances are actually alive. It starts from `main` and walks outward, collecting everything `main` can reach. That set is the transitive closure. It decides two things at once: which instances get built and wired into the runtime graph, and which unreachable instances are rejected with a compile error. Any instance the closure misses is either silently dropped from the live graph or flagged as an `unreachable_top_level_instance` error. So the exact membership of this set is the line between a legal program and a rejected one.

The two documents disagree on how wide the walk goes. The decision log names five kinds of reachable thing:

```
closure(main) =
    main's own subtree
  + destinations of connections in that subtree
  + module-level instances reachable through Handle / WeakHandle
  + node references passed as effect arguments   (reached as borrows, not cell stores)
  + the wire-candidate envelopes of its connections
```

The SPEC names only the first three:

```
closure(main) =
    main's own subtree
  + connection destinations
  + Handle / WeakHandle-reachable module-level instances
```

The two extra members in the log are real, defined concepts elsewhere in the language. An effect-argument-bound reference is a node handed to an effect as a borrowed argument, not stored in a cell — nothing "holds" it in a cell, yet the effect can touch it, so it must stay alive. A wire-candidate envelope is the type-level over-approximation of a dynamic connection: for a destination that can re-point, it is every node type the reference could resolve to. The section on dynamic references defines this same envelope and says it also feeds the interpretation closure, so it is already a first-class reachability concept.

The disagreement is not cosmetic. It is jointly unsatisfiable, and there is a concrete witness. Take a top-level instance that is reachable ONLY as an effect argument — nothing places it, no connection targets it, no Handle points at it, but `main`'s subtree passes it to an effect as a borrow. The log puts it in the closure, so it is live and legal. The SPEC's three-member closure misses it entirely, so the SPEC classifies it as `unreachable_top_level_instance` — a compile error. The same program is legal under the log and rejected under the SPEC. An implementer cannot honor both. (The effect-argument half of the witness is the strong one; the wire-candidate half is softer, since a dynamic reference resolves only to an already-placed node, so its envelope may not add instances the other members miss — but the effect-argument witness alone makes the pair unsatisfiable.)

The corpus is not actually torn here. The log's two closure entries both spell out all five members, in the same words, and are internally consistent. The SPEC's two closure passages — the reachability-scope paragraph and the startup-sequence step — both list only three. The one SPEC place that does mention wire-candidate envelopes is a different closure (the finite-interpretation closure), not the entry-point closure. So the SPEC omits both members in both of its entry-point-closure enumerations, and the edit protocol makes the SPEC the side that must conform.

**Recommended: the five-member closure (the log side), because both log entries state it consistently and in full, the two extra members are already defined reachability concepts the language relies on elsewhere, and the edit protocol makes the SPEC conform to the log. Dropping to three members would silently kill any instance reachable only as an effect argument — a live program turned into a compile error.**

Consequences:
- Both member findings conform automatically. Neither disputes the log's correctness; both flag only that the SPEC lists too few members. Adopting the five-member closure and editing the SPEC to match resolves both with no residual sub-decision.
- No STILL members. This is a one-directional divergence (SPEC under-specifies against a self-consistent log), not a genuine fork.
- One thing to confirm, not decide: whether the SPEC's three-member wording was an intended narrowing of the closure rather than an omission. The audit's charity check found no evidence it was intended — the SPEC never states a reason to exclude effect-arg or wire-candidate members — so the default is "omission, bring the SPEC up to five." If the user instead believes the closure should genuinely be three members, that is a change to the log, not the SPEC, and must be surfaced before editing.

> **For execution:** Canonical side = LOG (five-member closure). Conform SPEC to LOG 021-140 and 021-141 (both enumerate: subtree + connection destinations + Handle/WeakHandle-reachable module-level instances + effect-argument-bound node references [borrows, not cell stores] + connections' wire-candidate envelopes). Edit SPEC §13.8.1 reachability paragraph (SPEC.md:16612-16621, currently members (a)-(c)) and SPEC §13.14.1 startup step 3 (SPEC.md:19050-19053, currently "subtree plus everything reachable through connections and Handle/WeakHandle") to add the two missing members. Supporting definitions: wire-candidate envelope at LOG 024-17 and 015-16 (§13.11.5, §13.1). Do NOT touch SPEC.md:525 — that is the separate finite-interpretation closure (§13.19), which correctly already lists wire-candidate envelopes. Resolves findings F227 (HIGH, main report line 1325) and F173 (MED, main report line 1621) — same divergence, two severity views. Verify: `grep -n 'The closure includes' SPEC.md` and `grep -n 'transitive closure' SPEC.md` should, post-edit, both enumerate five members matching LOG 021-140. Confirm no other SPEC entry-point-closure enumeration remains at three.

#### D-24 — Does the orphan rule fire at package granularity or module granularity? *(resolves 1 finding, 1 automatically)*

In Ductus a `fulfill Trait for Type` block writes an implementation. To keep implementations from clashing, the language enforces coherence: for any (trait, type) pair there is exactly one implementation reachable through the module graph. The orphan rule is the mechanism that guarantees this. It says you may only write a `fulfill` if you "own" one side of it — the trait or the type. The open question is what "own" means: own it in your **package**, or own it in your **module**? A package is a whole distribution unit; each subfolder inside it is a separate, independent module. So the two readings are not the same, and they disagree about real programs.

Here is the exact program they disagree on. Package `myapp` has two sibling modules, `a` and `b`. Both `Foreign` (the trait) and `Widget` (the type) come from some other package.

```
// module myapp::a
fulfill Foreign for Widget: ...
```

Under **module granularity**, this is rejected: neither `Foreign` nor `Widget` is defined in module `a`, so both sides are foreign to `a`.

Under **package granularity**, whether it is legal depends only on whether either side is declared anywhere in package `myapp` — and here neither is, so it is also rejected. The two readings only split when one side lives in a *sibling* module of the same package:

```
// module myapp::b defines the type
node Widget: ...

// module myapp::a — writes the impl for a type it does not own locally
fulfill Foreign for Widget: ...
```

Now module granularity **rejects** this (`Widget` is foreign to `a`), while package granularity **accepts** it (`Widget` is declared in package `myapp`). Same source, opposite acceptance.

The corpus states the rule at both granularities and never reconciles them. Two LOG entries and one SPEC section say **package**: "the orphan rule operates on package-of-declaration... a `fulfill` is legal if the trait or the type is declared in the current package" (003-73), reinforced by 003-74 (a private type "still counts as in-package"), and SPEC §10.9 restates this verbatim. Two other LOG entries and a different SPEC section say **module**: "`fulfill Trait for Type` is permitted in module M iff `Trait` or `Type` is defined in M" (005-152), with 005-153 rejecting the both-foreign-to-module case, and SPEC §3.7.1 hard-coding "defined in M". The rule got restated in two different topic sections and the two copies drifted to different scopes.

**Recommended: module granularity, because it is the only reading that actually delivers the coherence the orphan rule exists to guarantee.** The SPEC's own rationale for the module rule spells out the mechanism: rejecting the both-foreign case "guarantees that no two independent modules can write conflicting implementations for the same (trait, type) pair: at least one of them would violate the orphan rule" (SPEC §3.7.1). Package granularity breaks exactly this. Under it, two sibling modules `a` and `b` in the same package could *both* write `fulfill Foreign for Widget` — both legal, both in-package — producing two implementations for one pair. That is a direct coherence violation (the "exactly one implementation" invariant, 005-150/005-87). The generic-coverage entries also assume module scope: they require a "concrete **local** type," and "local" is module-relative in these sections (005-155/156). So the module reading is the load-bearing one; the package reading is the drifted copy and is unsound.

**Consequences:**
- The single member finding conforms automatically. It only flags the contradiction and asks to pin one scope; picking module and conforming the package-side text resolves it with no residual sub-decision.
- No STILL members: this is a one-finding cluster with a clear technical winner.
- Note for whoever edits: 005-86 ("a `fulfill` block may live in any module subject to the orphan rule") is already phrased module-relative and needs no change. Only the two package-scope LOG entries and SPEC §10.9 move.

> **For execution:** Canonical side = **module-of-declaration**. Conform the package-scope statements to module scope. LOG edits: rewrite 003-73 and 003-74 to say "module" not "package" (a `fulfill` is legal iff the trait or type is declared in the current *module*; a private trait/type counts as in-module). Keep 005-152, 005-153, 005-154, 005-155, 005-156, 005-86 as-is (already module-scoped). SPEC edits: rewrite §10.9 (lines 8156–8168) to state package→module and drop "in the current package" (lines 8160, 8165); §3.7.1 (lines 2504–2521) already correct and is the reference text. Resolves finding P014 (addendum). Verify: `grep -n 'package-of-declaration\|in the current package\|in-package' DECISION_LOG.md SPEC.md` returns nothing in the orphan-rule context after edits; `grep -n 'defined in M\|current module' ` on 003-73/74 confirms conformance.

#### D-25 — Can the normative IR text form serialize a stream, or is one of the six graph primitives silently unserializable? *(resolves 1 finding, 0 automatically)*

> **Post-merge status (2026-07-08, corpus at 1f204c473a):** What changed: the grammar session created a new document, IR_GRAMMAR.md ('Ductus IR Text Grammar'), that declares itself normative -- 'It is normative for the IR text shape: a backend implementer should be able to write an IR loader from this document alone' -- and calls itself 'a standalone re-formulation of SPEC 15.4'. It did NOT close the stream-serialization gap; it duplicated it. IR_GRAMMAR.md:181 carries the identical `entry ::= cell | gate | connection | effect` with no `stream` alternative and no top-level `stream` production (its only 'stream' mentions are reopen_set comments at lines 176/233-234). So the totality gap D-25 names now exists in TWO normative places, not one: SPEC.md sec 15.4.6 (entry at SPEC.md:24739) and IR_GRAMMAR.md:181. What still stands: the core conflict is unresolved and the whole item -- both the problem statement and the Option A recommendation (entry-alternative, inside scope) -- holds unchanged; all six primitives are still named at 033-56, the lowering still targets stream at 033-68/028-65, the ten fields still exist at SPEC.md:24331-24360, and the three text-form-is-normative claims (033-53/033-232/033-237) still assume a stream can be written. What remains for the owner: apply Option A's fix in BOTH normative documents, not just SPEC -- add IR_GRAMMAR.md:181 as a second landing site for the `entry` alternative plus a `stream ::=` production in its module grammar (its sec 3), and add a `grep -n 'stream' IR_GRAMMAR.md` check to verification. Adjacent, out of D-25 scope but worth flagging: the 6 inserted 033 entries (033-220..231, sec 15.6/15.7 conformance) do not touch stream serialization and settle no part of this item; and LOG 001-3 and 002-29 still assert 'there is no separate grammar document' even though GRAMMAR.md and IR_GRAMMAR.md now exist -- a separate contradiction.


Ductus compiles a surface program down to a graph made of exactly six primitives: cell, connection, gate, stream, effect, and scope. The spec defines a normative text form for that graph — a human-readable grammar. That text form is the one serialization the spec calls the contract: a conformant compiler must render its IR in it, two implementations interoperate over it, and tests assert against it. The stream primitive is one of the six, and it has a complete data model: id, element type, ring-or-gate policy, capacity, source dependencies, six observation cells, two reset flags, output-history size, and an input-lookback map.

Here is the problem in one grep. The grammar's `entry` production — the list of things a scope can contain — reads:

```
entry ::= cell | gate | connection | effect
```

There is no `stream` alternative, and no top-level `stream` production anywhere else in the grammar. So a legal program like `stream ring[1024] events: LogEntry = source`, which lowers to the stream primitive, has no line the grammar can produce for it. A conformant compiler cannot render it, two implementations cannot exchange it, and no test can assert against it. One of the six primitives silently drops out of the one serialization the spec declares normative.

Two ways out, and they point in opposite directions.

Option A — add the production. Give stream its own line, parallel to the way the `effect` line renders, carrying the ten data-model fields:

```
entry  ::= cell | gate | connection | effect | stream
stream ::= 'stream' PATH ':' type_tag 'policy' ('ring'|'gate') 'capacity' INT
           ('source_deps' path_set)? ('observes' path_set)?
           ('reset_on_reload')? ('reset_on_reopen')?
           ('history' INT)? ('lookback' lookback_map)?
```

Now all six primitives serialize, and the text-form-is-normative claim stays true across the whole graph.

Option B — carve streams out. Leave the grammar as-is and amend the three claims that make the text form the total normative contract, so they read "every primitive except stream." Then say explicitly where a stream's serialization lives instead — since the spec already allows an implementation to never externalize the IR at all, streams could be declared implementation-defined for serialization.

Within Option A there is a shape sub-question: does the stream line go inside a scope as a fifth `entry` alternative (shown above), or as a separate top-level list in `graph_section` parallel to `scope+`? Streams are declared inside scopes in the surface language and the data model gives each a scoped path id, so the `entry`-alternative placement matches where streams actually live. A separate top-level section would divorce a stream from its enclosing scope for no stated reason.

What the corpus currently says, and where it disagrees with itself. The six-primitive count is normative and names stream. The lowering rule sends every surface stream to the stream primitive. The data model spells out all ten stream fields in full. And three separate rules make the text-form grammar the normative serialization, the interop contract, and the test-assertion target. Every one of those points assumes a stream can be written down. The grammar section alone contradicts them: its `entry` production stops at `effect`, and its prose walks through cell, gate, connection, and effect but never serializes a stream. This is not drift in wording — it is a totality gap. The machinery to describe a stream exists everywhere except in the one place that turns it into text.

**Recommended: Option A with the `entry`-alternative placement, because the spec has already committed everywhere else that stream is a first-class primitive with a full data model — the count says six, the lowering rule targets it, and ten fields define it — so the honest fix is to let it serialize, not to retract that commitment. Carving streams out (Option B) would mean the interop and test contract cannot cover a legal, fully-modeled feature, which is a strictly weaker spec.** Place it inside `scope` because the data model already scopes each stream by path.

Consequences:

- **No members conform automatically.** This key has a single member finding, and it stays open until the grammar is edited. Nothing else in the corpus resolves on its own.

- **P022 (STILL, HIGH) residual sub-decision:** the finding is confirmed and points to exactly this fix, but leaves two choices to you. First, Option A vs Option B — add the production, or carve streams out of the text-form-is-normative claims. Second, if Option A, the placement: fifth `entry` alternative vs a top-level `graph_section` list. Answer both inline to close it.

- **One coupling to watch, not a member here:** two sibling findings (the effect `desired:` block holding stream cells, and the per-cell-identity question for desired cells) also touch stream serialization. If Option A lands, the new `stream` production is the thing a resolved desired-stream lowering would render into — so pin the production first, then those resolve against it. This is a note, not a residual of this key.

> **For execution:** Canonical side -- Option A (add the production), `entry`-alternative placement, inside `scope`. Edit LOG-first per protocol. LOG entries to conform: 033-56 (six primitives, entry 4175), 033-68 (surface stream lowers to stream primitive, 4187), 033-106..118 (stream data-model fields, entries 4225-4237), 033-53 (text form normative, 4172), 033-168 (grammar covers graph section, 4287), 033-232 (compiler renders in text form, 4351), 033-237 (interop against text form, 4356), 028-65 (stream maps to graph-IR stream primitive, 3372). SPEC edits: sec 15.4.6 -- add `stream` to the `entry` production (SPEC.md:24739) and a new `stream ::=` production after the `effect` production (SPEC.md:24751-24752), covering the ten fields listed at sec 15.4.1 (SPEC.md:24331-24360); extend the sec 15.4.6 prose (SPEC.md:24769-24798) with a sentence serializing a stream and add a worked stream example. If Option B is chosen instead, amend 033-53/033-232/033-237 to exclude stream and add a carve-out citing 033-55 (need-not-externalize, 4174). Finding id: P022 (addendum). Verification greps: `grep -n 'stream ::=' SPEC.md` must return one hit; `grep -n 'entry *::=' SPEC.md` must show stream in the alternatives; confirm zero `stream ::=` at HEAD 1f204c473a before editing.

#### D-26 — Does an effect's `desired:` block lower to one whole-record cell, or to a set of per-declaration cells? *(resolves 3 findings, 0 automatically)*

An effect states what it *wants* the outside world to be in a `desired:` block. That block holds one or more declarations: plain `derived` fields (`derived request: Request = ...`), event-output streams (`stream ring[1024] outbound: Message = ...`) that the host's reconciler drains in order, and `when`/`given` arms whose fields freeze when their arm is inactive. The IR is the compiled form the runtime executes. The question is how the compiler represents that whole `desired:` block in the IR: as a single cell, or as many.

**Model A — a set of per-declaration cells.** Every declaration in the block becomes its own graph cell, exactly like cells anywhere else in the graph.

```
desired:
  derived request: Request = ...
  derived headers: Headers = ...
->
  derived  f:0.desired.request : Request  uses B@1 inputs [...]
  derived  f:0.desired.headers : Headers  uses B@2 inputs [...]
  effect f:0 ... desired [f:0.desired.request, f:0.desired.headers]
```

Each field has its own cell id, its own behavior, its own dependency edges. A stream field lowers to a stream cell (the same way `observed:` streams already do). A `when`/`given` arm lowers to per-arm cells behind a gate that freezes them.

**Model B — one whole-record cell.** The entire block collapses into a single cell holding one record, built by one pure "desired-builder" behavior that takes the block's inputs and returns the whole record. The runtime then scatters that record into per-field state for the reconciler.

```
desired:
  derived text: string = message
->
  derived  print:0.desired : pool_index<%TextRec>  uses B@d5 inputs [message]
  effect print:0 ... desired [print:0.desired]
```

One cell, one behavior, one record output. This is what the recent amendment added and what the only worked example in the IR-serialization section shows.

**Where the corpus disagrees with itself.** The amendment added Model B in two log entries — one saying the desired state is a single whole-record `pool_index<%T>` cell built by one desired-builder behavior, one giving that behavior's signature — plus a matching ABI table row and the single-field worked example. But Model A is what the rest of the corpus already assumes, in three independent places:

- The effect-entry data model says `desired_cell_ids` is a *plural list* of the block's cells. One record cell is not a list of the declared cells. (finding P024)
- Surface rules let program code read individual desired fields: `f.request` reads the `request` cell, and `derived spinner = f.in_flight` builds a dependency edge onto the `in_flight` cell. That edge needs `request`/`in_flight` to be real cells with stable ids. A field of an opaque record scattered "for the reconciler" is not a cell you can depend on. (finding P024)
- A desired stream (`stream ring[1024] outbound`) is a policy-buffered sequence the reconciler drains in order. A pure behavior returns exactly one value per call and a scalar record field cannot be a drained buffer, so a desired stream has no place in a single whole-record cell. (finding P023)
- A gated `when`/`given` arm must freeze independently — an inactive arm "neither goes dirty nor triggers an update," and reopening recomputes just that arm. One pure builder recomputes the whole record whenever any input changes; it cannot hold one arm's fields frozen while a sibling is live. And the entry asserting the desired computation has "no activation input anywhere" collides with a gate predicate selecting which arm is live. (finding P025)

So Model B, as written, only works for the trivial single-scalar-field case the worked example shows. Every richer feature the `desired:` block already declares legal — multiple readable fields, streams, gated arms — has no Model B representation.

**Recommended: Model A (a set of per-declaration cells), because it is what three separate parts of the corpus already require and Model B cannot represent the features the block already declares legal.** The plural `desired_cell_ids` list, the per-field program reads with real dependency edges, the drained streams, and the per-arm freeze are all established surface and data-model decisions with worked mechanics. Model B is a recent, narrow addition backed by a single-field example that never exercises any of them. Model A also matches the sibling `observed:` block, which already lowers its declarations to ordinary per-cell graph cells (computed observed cells "appear as an ordinary derived/recurrent cell in the Cells list"). The whole-record cell is best kept, if at all, as a reconciler-facing *view* assembled from the per-declaration cells — not as the graph representation.

**Decide first: the stream IR text form.** A desired stream can only lower to a graph stream cell if the IR serialization grammar has a text form for stream cells at all. That is the open question in the stream-IR-text item (K-STREAM-IR-TEXT). Settle that first; a desired stream cell reuses whatever cell form that item defines.

Consequences:

- **No member conforms automatically.** All three findings need the log/spec edits below; none is a free consequence of picking Model A.
- **P023 (desired streams) — residual:** confirm a desired stream lowers to the *same* graph stream cell form as an `observed:` stream (reusing K-STREAM-IR-TEXT's cell form), rather than a bespoke desired-only stream shape. Recommended yes, for one stream lowering across both blocks.
- **P024 (per-cell identity) — residual:** decide whether the whole-record cell is *deleted* or *retained as a reconciler-facing view* assembled from the per-declaration cells. Recommended: retain as a view (keeps the scatter-into-per-field-state mechanism the reconciler already relies on) but make the per-declaration cells the graph-level source of truth that `f.request` edges attach to.
- **P025 (gated arms) — residual:** confirm intra-desired `when`/`given` lowers to per-arm cells behind a gate distinct from the whole-effect suspend/resume gate, and reconcile the "no activation input anywhere" claim — it must be narrowed to mean "no activation input *for the whole effect's suspend/resume*," since an intra-desired gate predicate genuinely is an activation input for arm selection.

> **For execution:** Canonical side: Model A (per-declaration cells). Retract/rewrite the Model-B entries: **033-170** (LOG 4289, single whole-record `pool_index<%T>` cell) and **033-202** (LOG 4321, desired-builder behavior sig/output) — either delete or demote to a reconciler-facing view assembled from per-declaration cells. Conform to the plural model already in **033-122** (LOG 4241, `desired_cell_ids` = list). Keep legal: **031-30** (LOG 3806, `f.request` read), **017-278** (LOG 2425, `f.in_flight` flat access), **031-31/032/033/036** (LOG 3807-3812, desired event-output streams), **031-37/038/039/040/041** (LOG 3813-3817, per-arm freeze). Narrow **033-216** (LOG 4335, "no activation input anywhere") to whole-effect suspend/resume only; reconcile with **033-62** (LOG 4181, `when`/`given` lower to `gate`). SPEC: rewrite §15.4.5 (SPEC ~24646-24716, worked example + "no activation input" prose) and §15.4.4 desired-builder row in the Behavior ABI table (SPEC 24596); conform §15.4.1 `desired_cell_ids` prose (SPEC 24378-24379) and the graph text grammar in §15.4.6 to emit per-declaration desired cells (incl. stream cells per K-STREAM-IR-TEXT). Surface refs: §13.19.4 (desired streams + gated arms), §13.19.5 (observed lifted-computed precedent). Findings: P023, P024, P025 (addendum). Depends on: K-STREAM-IR-TEXT (stream cell text form). Verify greps: `grep -n '033-170\.\|033-202\.\|033-122\.\|033-216\.' DECISION_LOG.md`; `grep -n 'whole-record\|desired-builder\|desired_cell_ids\|no activation input' SPEC.md`.

## Late additions (drafted after the main pass)

#### D-27 — Does the sealed `StreamPolicy` trait have two members or four? *(resolves 2 findings, 1 automatically)*

Every Ductus stream carries a buffer policy that says what happens when the buffer is full. An amendment remodeled this surface: instead of a flat enumeration of four policies, the policy is two orthogonal axes — the buffer discipline, and a self-history depth. The stream section and the SPEC were rewritten to the two-axis model. One LOG entry was not, and it still counts four.

The two-axis model, as the SPEC states it:

```
trait StreamPolicy                       // sealed — users cannot add members
type Ring[const N: usize]                // overwrite oldest unconsumed event when full
type Gate[const N: usize]                // reject the write when full

fulfill StreamPolicy for Ring[N]
fulfill StreamPolicy for Gate[N]         // exactly two members — no third, no fourth
```

A recurrent stream is not a policy. It is an ordinary `ring`/`gate` stream carrying a positive history depth on the second axis:

```
recurrent[H] stream ring[N] T            // Ring[N] buffer + H steps of self-history
                                         // two axes on one stream, not a fourth policy member
```

What the corpus says, and where it fights itself. Five entries in the stream section fix the member set at exactly two, and one of them records the retirement in its own words — the recurrent policies "are no longer distinct policy members." The SPEC is emphatic and self-consistent: "The *only* two sealed `StreamPolicy` members," and "There is **no** `RecurrentRing` / `RecurrentGate` StreamPolicy member." Against all of that, one entry still says every stream value's policy is "one of the four `StreamPolicy` members." A sealed trait has one member count; an implementer cannot build both a two-way and a four-way discriminant. The four-member wording is a survivor of the pre-amendment model.

There is a second, quieter fault riding along: alias names. The stdlib provides transparent sugar for the common spellings — `RingStream[T, N]` = `Stream[T, Ring[N]]`, likewise `GateStream`. One entry goes further and names recurrent aliases too — `RecurrentRingStream[T, B, H]` / `RecurrentGateStream[T, B, H]` — while the SPEC affirmatively denies they exist and the sibling alias entry lists only the two. There is also a hard technical blocker: `Stream` takes two type parameters (element, policy), and the history depth is a stream parameter, never a policy parameter — so a three-parameter alias has no `Stream[...]` spelling to expand into. As written, the recurrent aliases cannot be transparent sugar over the two-axis model at all; they would need `Stream` to grow a third type parameter.

**Recommended: two members (`Ring[N]`, `Gate[N]`), because the two-axis model is the deliberate amendment — one entry records the retirement of the recurrent members in its own text — the SPEC elaborates it emphatically and consistently (its fulfill block lists exactly two), and the four-member wording survives in exactly one un-updated entry. Adopting four would mean reverting a documented amendment across five entries and the whole SPEC stream chapter.**

Consequences:

- **F199 (HIGH) conforms automatically.** The lone "one of the four" entry is rewritten to the two-member, two-axis phrasing. Nothing else moves; the five two-axis entries and the SPEC already state the target.
- **STILL — recurrent alias names (F193, MED):** even with the count fixed, whether `RecurrentRingStream`/`RecurrentGateStream` exist as stdlib sugar is its own call. Recommended: they do not — the SPEC already denies them, and they are inexpressible as transparent aliases while `Stream[T, P]` has no history slot. If you want them anyway, that is a real design change (a third type parameter on `Stream`) and the SPEC denial flips. Decide inline: **no recurrent aliases** (conform the one entry naming them, drop "recurrent" from the alias-list parenthetical, tighten the alias entry's vague "the recurrent forms" tail), **or add the aliases** (amend `Stream`'s type parameters plus the two SPEC denial passages)?
- Whichever way the alias call goes, keep the spelling consistent with the cell-model decision at the top of this document: the lowercase kind form is the declaration surface; the bracketed alias spellings are type-level stdlib sugar.

> **For execution:** Canonical side = two-axis model, exactly two sealed members (LOG 030-254 at DECISION_LOG.md:3766, 030-255 :3767, 030-258 :3770, 030-259 :3771, 030-261 :3773; SPEC §13.18.3 at SPEC.md:20584-20625, aliases 20627-20639; §13.18.8.6 at SPEC.md:21115-21131). Conform 030-37 (:3549): "one of the four `StreamPolicy` members" → "one of the two sealed `StreamPolicy` members (`Ring[N]` or `Gate[N]`)". F193 residual, if "no aliases" is confirmed: conform 030-116 (:3628) to stop naming `RecurrentRingStream[T,B,H]`/`RecurrentGateStream[T,B,H]` and restate per §13.18.8.6's actual text (produces a `Ring[B]`/`Gate[B]`-policy stream carrying history depth H); fix 030-36's (:3548) parenthetical "(likewise `GateStream`/recurrent)" → "(likewise `GateStream`)"; tighten 030-261's (:3773) tail so "the recurrent forms" cannot be read as alias names. SPEC unchanged. Related: K-CELL-MODEL (D-01) for kind-vs-bracket spellings. Findings: F199 (HIGH, main report L2280 — auto), F193 (MED, L2421 — STILL, alias existence). Verify: `grep -n 'four .StreamPolicy.\|RecurrentRingStream\|RecurrentGateStream' DECISION_LOG.md` returns zero; `grep -n 'RecurrentRing' SPEC.md` returns only the denial sentences (SPEC.md:20620, 20636-20637); `grep -n 'RecurrentRing' DECISION_LOG.md` returns only 030-254's retirement clause.

#### D-28 — When the philosophy says "no shared mutable state," does that cover the host-written reactive-cell store? *(resolves 2 findings, 1 automatically)*

The philosophy section makes two absolutes. External state — a list that names signals — "is always immutable." And, flatly: "There is no shared mutable state." The runtime chapter then defines the runtime as a transactional store of reactive cells that the host writes, every frame, through `write`/`push`. Both halves are load-bearing. Read literally, they collide on the very first signal a program declares:

```
signal master_gain: f32 = 1.0      // philosophy: "always immutable", "no shared mutable state"

# host side, every frame:
runtime.write(master_gain, 0.5)    # runtime chapter: a mutable store the host writes
```

The intended line is visible elsewhere in the corpus — it just never made it into the philosophy. The signal section says a signal is "a writable reactive cell," written "only through the host API; Ductus source has no syntactic form for assigning to a signal." And a threading entry states the scoped version outright: "no shared mutable state exists outside reactive cells, which the runtime coordinates" — an entry that concedes shared mutable state *does* exist, inside coordinated cells. So the corpus holds one absolute and one carve-out in different sections, neither acknowledging the other, and the SPEC mirrors both verbatim. The audit's verifier refiled this from an ambiguity to a LOG-internal divergence for exactly that reason.

The question: what are the absolutes actually statements about?

Option A — keep the absolutes literal and redefine the term:

```
# "shared mutable state" := unsynchronized shared memory
# the cell store is coordinated (snapshot isolation), so it does not count
```

This saves the sentence by definitional fiat. It needs a new definition no current text states, and it still has to rewrite the threading entry — which today concedes the store is the exception, not a non-instance.

Option B — scope the absolutes to Ductus source:

```
# In source: no assignment to shared state, ever.
# The reactive-cell store is the single sanctioned mutable channel —
# written only by the host, coordinated by the runtime.
```

**Recommended: option B, because the corpus already draws exactly this line in two places — the signal section (writes come only through the host API; source has no assignment form) and the philosophy's own "time-varying external behavior is expressed through the reactive system, not through assignment" — so B is one qualifier added to the absolutes, while A invents a definition nothing currently states and rewrites the threading entry anyway.**

Consequences:

- **F259 (LOW) conforms automatically.** Once the scope is pinned, "always immutable" for signals narrows mechanically to source-immutable: no source assignment form exists; the host writes through its API. The philosophy word and the signal section's "writable reactive cell" stop fighting because they describe two sides of one boundary.
- **STILL — where the scoping lives and its exact words (F258, MED):** the absolute entry and the threading entry contradict each other today; one of them must carry the reconciliation. Decide inline: **amend the absolute in place** ("no shared mutable state at the source level; the reactive-cell store, host-written and runtime-coordinated, is the single sanctioned mutable channel"), **or keep the absolute short and add a new philosophy entry** carrying the carve-out next to it? And pick the scope wording: "source-observable state" versus the threading entry's "outside reactive cells." The SPEC's philosophy sentence takes the same qualifier either way.

> **For execution:** Canonical side = source-scoped absolutes with the reactive-cell store named as the single host-written, runtime-coordinated mutable channel. LOG edits: 001-22 (DECISION_LOG.md:52) gains the scope qualifier (or a new 001 entry carries it — the F258 residual); 001-11 (:41) narrows "always immutable" for signals to source-immutable (no source assignment; host-writable per §13.14). Unchanged and now consistent: 001-15 (:45), 027-3 (:3187), 027-6 (:3190), 032-125 (:4062). SPEC edits: §1.3 "and no shared mutable state" (SPEC.md:62-65) and the "always immutable" sentence (SPEC.md:46-49) take the same qualifier; §13.2.1 (SPEC.md:11721-11724) and §14.6.4 (SPEC.md:23834-23836) already state the scoped version and stay. Findings: F258 (MED, main report L3250 — STILL, refiled by verifier as 001-22 vs 032-125 divergence), F259 (LOW, L3662 — auto). Verify: `grep -n 'no shared mutable state' DECISION_LOG.md SPEC.md` — every hit post-edit carries the scope (source-level / outside reactive cells); `grep -n 'always immutable' DECISION_LOG.md SPEC.md` — the signal mention is qualified source-immutable.

#### D-29 — Is `Vec[T]` the spec's own name, and how do examples construct one? *(resolves 3 findings, 3 automatically)*

Ductus gives dedicated language syntax only to fixed-size arrays. The growable vector is a stdlib type — no special powers, buildable by any user on the allocator intrinsic. One entry pushes that further: the vector's "name and syntax are outside the specification," and the SPEC hedges in the same spirit — "(`Vec[T]`, `Vector[T]`, or whatever stdlib chooses)." Meanwhile the rest of the corpus treats the name as fixed. The element-tally entry commits a language-level `.count` accessor on `Vec[T]` by name; the LOG names `Vec[` in some thirty entries; the SPEC spells out the module path `std::vec::Vec`; loop and trait entries name it normatively. You cannot commit `.count` on a type whose name you refuse to own.

There is a second fault riding on the same type: construction. Eleven SPEC fences and one LOG entry build a vector like this:

```
mut v = Vec::new()          // what 11 SPEC fences + 1 LOG entry write today
```

That is the type-qualified free-function call form, and the language forbids it outright — "No type-qualified free-function form exists" — with SPEC's own prose calling `Type::fn(x)` unsanctioned twice and prescribing a module path or method form instead. (Type-side dispatch does not rescue it: that form selects a fulfill-block method for a receiver argument, and `new()` has no receiver.) The form the language actually has:

```
mut v = std::vec::new()     // module-path free function — the sanctioned call shape
```

So, two questions. Ownership of the name:

Option A — the name really is outside the spec. Then every normative `Vec[T]` mention (dozens) must be rewritten to "a stdlib growable vector," and the `.count` unification cannot name its own subject.

Option B — the spec pins the name: `Vec[T]`, module `std::vec`, stdlib-owned. No dedicated syntax, no special powers — but the name is fixed so normative text can reference it, exactly as the corpus already does for `Map[K, V]` and `HashSet[T]`, stdlib collections named normatively in the very same `.count` entry.

And constructor spelling: the module-path free function `std::vec::new()`, matching the pattern SPEC's own prose prescribes for `std::duration::from_nanos` and `std::instant::now`.

**Recommended: option B plus `std::vec::new()`, because the corpus already treats the name as fixed in dozens of normative statements and even pins the module path `std::vec::Vec` — the "name outside the spec" sentence is the one-entry outlier — and the module-path constructor is the only spelling the language's own call rules permit; `Vec::new()` is flatly illegal under the forbidden-form entry.**

Consequences:

- **All three findings conform automatically** once the two spellings are pinned. F021: amend the "name and syntax are outside the specification" entry — keep stdlib ownership, pin the name — and the `.count` entry stands as written. F121 and F122: rewrite the two flagged fences to the sanctioned constructor. No residual sub-decision.
- **Note — the sweep is bigger than the findings.** The audit flagged two `Vec::new()` fences; there are eleven in the SPEC (closure-types, in-place-mutation, iterator, and reactive-structure examples) plus one LOG entry carrying the same form. Same defect class — fix all twelve sites in this pass, not just the two flagged ones.
- **Note — the constructor name is minted here.** No current text names the vector constructor; `std::vec::new()` follows the module-path pattern by analogy. If you prefer a different stdlib spelling (say, a from-literal form), say so before the sweep — it changes twelve edit sites.
- **Note — one edited entry anchors an unrelated open finding.** The LOG entry carrying `Vec::new()` also anchors the addendum's closure-field invocation ambiguity (how `r.handler()` resolves for a closure-typed field). This rename does not touch that question; do not mark it closed.

> **For execution:** Canonical side = `Vec[T]` spec-named / stdlib-owned, constructor `std::vec::new()`. LOG edits: 012-128 (DECISION_LOG.md:1330) — keep "stdlib concern, no dedicated language syntax," replace "the vector's name and syntax are outside the specification" with the pinned name (`Vec[T]` in `std::vec`; API surface beyond spec-referenced members is stdlib-owned); 013-200 (:1592) — `Vec::new()` → `std::vec::new()`. Unchanged: 012-91 (:1293), 011-55 (:1173), 005-129 (:471). SPEC edits: §9.3.6 (SPEC.md:7000-7004) — drop the "(`Vec[T]`, `Vector[T]`, or whatever stdlib chooses)" hedge, state the pinned name; replace `Vec::new()` at SPEC.md:9755 (§11.10.6 — F121), 9891 (§11.11.2 — F122), 9934, 11174, 11310, 13720, 15591, 15606, 15629, 15664, 15833. Supporting facts: forbidden form 011-55 + SPEC §8.7 (SPEC.md:6344-6350) + §9.4.1.4 (SPEC.md:7298-7302); module path `std::vec::Vec` at SPEC.md:7772/7776. Cross-note: 013-200 also anchors addendum P028 (closure-field call resolution) — untouched here. Findings: F122 (MED, main report L3300), F121 (MED, L3314), F021 (LOW, L3841) — all auto. Verify: `grep -n 'Vec::new' SPEC.md DECISION_LOG.md` returns zero; `grep -n 'whatever stdlib chooses' SPEC.md` returns zero; `grep -n '^012-128\.' DECISION_LOG.md` shows the pinned name.

#### D-30 — After a hot reload, when is an effect instance "the same instance"? *(resolves 3 findings, 2 automatically)*

Hot reload swaps source while the runtime keeps running. For every effect instance, the reload diff must answer one question: is this the same instance as one in the old source (preserve its state, fire `update`) or a different one (tear the old down, create the new)? The corpus currently answers with three different rules, and they disagree on real edits:

```
node Dashboard:
  effects:
    log_effect(target: sink_a)
    log_effect(target: sink_b)     // edit: swap these two lines, then reload
    repeat row in rows keyed by row.id:
      render_row(data: row)        // two rows, equal data payloads, different ids
```

Rule 1 — declaration path with positional ordinal. The cell-identity entry extends its path scheme to effect instances, and duplicated siblings get a zero-based `:N` ordinal by declaration order. Swap the two `log_effect` lines → each ordinal flips → each path changes → both are "different instances": teardown + create, state lost on a pure reorder.

Rule 2 — the operator identity rule. The hot-reload-of-effects entry keys an instance by (enclosing scope, effect type name, argument bindings), tolerating positional moves within the same scope. Swap the two lines → the arguments distinguish them → both preserved. But the two repeat rows with equal `data` payloads have identical bindings — they collide under this rule, while their distinct ids should keep them apart.

Rule 3 — the element key. The repeat-in-effects entries materialize one instance per element; instances suspend, resume, and tear down with the key; their cells live at `<node>.<row.id>.<cell>`. Correct for repeat — and silent about every non-repeat effect.

Where the corpus stands. Rules 2 and 3 each have full SPEC elaboration: the operator-identity section spells out the diff algorithm down to the tiebreak (indistinguishable identical calls match by syntactic order; an inserted third identical call is fresh, the existing two preserve state), and the repeat sections diff the key set. Rule 1's effect clause has none — the section it cites elaborates only cells and never mentions effect instances, which is its own finding. And rules 1 and 2 give opposite reconciler-hook sequences on the swap edit above; an implementer cannot satisfy both.

**Recommended: one rule — the operator identity rule — with the repeat key entering through its scope term, and paths demoted to addressing. Concretely: (a) base — effect-instance identity is (enclosing scope, effect type name, argument bindings), positional moves tolerated, indistinguishable duplicates matched by syntactic order; (b) repeat — a keyed repeat arm creates one enclosing scope per element key, so the key is the scope for its instance and equal payloads with distinct ids stay distinct (grounded: the repeat entries already say each per-element scope materializes one instance and root its cells at the key path — but no current text folds the key into the identity rule, so this clause must be stated, not assumed); (c) paths — the fully-qualified path with its `:N` ordinal remains how cells are addressed, and an effect instance's path is derived from its matched identity after the diff, never used as the identity criterion, so a preserved instance may get a new ordinal and keep its state. This is the only combination under which every behavior the SPEC already elaborates stays true — positional-move tolerance, syntactic-order matching, key-set diffing — and the only text it demotes is precisely the one claim with zero SPEC elaboration behind it.**

One dependency to name: this item decides *whether* a reload edit preserves or replaces an instance; the reconciler-hooks item earlier in this document decides *when* the resulting hooks fire and in what order. Assuming that item goes as recommended (all hooks post-publish, teardown before create, with the reload-commit tail as its own named residual), nothing here conflicts: an identity break at reload yields teardown then create, at whichever boundary that residual picks.

Consequences:

- **F205 (MED) conforms automatically.** Rewrite the cell-identity entry's effect clause and the ordinal entry's "applies equally to interpreter-placed effects" clause so effect instances key by the one rule and the ordinal is an addressing/tiebreak device mirroring the syntactic-order matching. The reorder contradiction (teardown/create vs preserve) dissolves.
- **F220 (MED) conforms automatically.** State the key-as-scope clause in the hot-reload-of-effects entry and its SPEC section, so the repeat contract and the operator-rule contract become one rule instead of two identity schemes for the same instances.
- **STILL — where the identity rule is elaborated (F209, LOW):** the cell-identity section is cited for the effect-instance claim but elaborates only cells. Decide inline: **repoint the effect clause to the hot-reload-of-effects section** (which already carries the elaboration — the low-risk default, same shape as the portal-reload repoint in the duplicate-copies item), **or add effect-instance text to the cell-identity section**? Either way, interpreter-placed instances need one sentence saying their enclosing scope is the interpretation site and their cell paths root there.

> **For execution:** Canonical side = operator identity rule as the single effect-instance rule (028-51 at DECISION_LOG.md:3358; SPEC §13.15.6 at SPEC.md:19620-19625; algorithm + tiebreak SPEC §13.17.10 at SPEC.md:20233-20249), plus keyed-repeat scope clause (018-109/018-110 :2585-2586; SPEC §13.5.4.3 at 15586-15600, §13.5.4.7 at 15753-15756), plus path-as-addressing demotion. LOG edits: 028-4 (:3311) second sentence — interpreter-placed effect instances follow the 028-51 rule; interpretation-site rooting applies to their cell paths, not their identity; 028-5 (:3312) — the ":N applies equally to interpreter-placed effects" clause becomes ordinal-as-address/tiebreak per §13.17.10's syntactic-order matching; 028-51 (:3358) — add the keyed-repeat clause (element key = per-element enclosing scope, superseding argument-binding equality; cross-ref §13.5.4.3/§13.5.4.7). Cells unchanged: 028-4 first sentence, 028-6 (:3313), SPEC §13.15.2 (19446-19471) remain the cell rule. Depends-on: K-HOOK-ORDER (D-08) for when the resulting teardown/create fire (its F108 reload-commit residual). F209 residual: repoint 028-4's effect clause §13.15.2→§13.15.6, or add effect text to §13.15.2. Findings: F205 (MED, main report L4656 — auto), F220 (MED, L4701 — auto), F209 (LOW, L4856 — STILL, repoint vs elaborate). Verify: `grep -n '^028-4\.\|^028-5\.\|^028-51\.' DECISION_LOG.md` — post-edit, no entry asserts path/ordinal as effect-instance identity; `grep -n 'element key' SPEC.md` — §13.15.6 states the key clause; confirm §13.17.10's tiebreak text (SPEC.md:20244-20249) is unchanged.
