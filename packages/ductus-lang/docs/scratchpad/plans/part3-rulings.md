# Part 3 — Ruling Record

*Authority for Part 3 ruling-queue executions. Consent batch authority: part3-consent-approved.md. Evidence: part3-survey-report.md.*

## Batch 1 — Piping, Interpretation & Dispatch (owner, 2026-07-20)

- **I-01: Option A.** The interpretation bootstrap is a plain call — `render(song)` — the effect-kind method on the LEFT, dispatching on the Subject-typed argument. `|>`'s third dispatch case RETIRES: two dispatch cases remain (operator call, effect call); node-reference LHS is illegal. Sites: 029-65 (third case removed), 029-106 (third alternative removed), SPEC §13.17.7 Case 3 (rewritten to a short illegality note + call-form pointer), the garbled Case-3 comment (dies), 005-114/120/121 "`|>` bootstrap" phrasing → effects:-clause call form, 017-273 example stands as-is. Liveness unaffected (node-as-effect-argument is already a reach-edge).
- **I-24: Subject-less effect-kind methods ALLOWED** (owner overrules the mandatory recommendation): an effect-kind method may take a Subject-typed first parameter (interpretation obligation — the interpreter pattern) or omit it (trait-associated effect). Emerging design questions (satisfaction gate, invocation form, effects-local boundary) to be pinned before execution — presented as Q1–Q3.
- **I-24 rider: α.** 005-51 conforms to SPEC practice: "effect-specific" means invocation form (invoked as an effect, never a value call), NOT satisfied-only-by-effect-kinds. The replacement satisfaction gate is Q1's answer. (Disclosure on record: 005-51 vs SPEC §3.1.1.1 + `fulfill Renderable for Song` was a live LOG↔SPEC contradiction found during Batch 1.)
- **I-57: REJECT** — containment-acyclicity law: a type's transitive type-emitted containment may not include itself; compile error, named diagnostic class (default `type_emission_cycle`), catching direct and transitive cycles. Loops live in topology (Circularity wires), not containment.
- **I-57 rider: ADOPTED** — `subject` becomes the fourth admissible self-sourced wire destination form (own child / module-level instance / WeakHandle attr / subject): static, counts against the type's own incoming budget, self-edge subject to the Circularity cycle check. Makes self-loop wires type-intrinsic (`Loops/500ms: subject`).
- **DP-13: (a)** — a runtime `match` in `Type[…]` position is legal iff every arm resolves to one concrete type; otherwise compile error directing to `given` (or a compile-time scrutinee). No runtime type tags; no-erasure stands.

## Batch 1 design completion — Q rulings (owner, 2026-07-20)

- **Q2: (i)** — Subject-less effect-kind methods invoke ONLY as path-qualified effects:-clause entries: `<TypePath>::<method>(args)` where TypePath is a concrete type or in-scope generic parameter; bare-name and trait-path-alone forms rejected. Subject-ful methods invoke as `method(subject_arg, ...)` per I-01-A, dispatching on the argument.
- **Q3: PINNED** — new entry: selecting which effect type a node instantiates via a generic parameter or a `Type[C]` slot does not violate effects-are-local; instantiation site, lifecycle, and ownership remain the local `effects:` clause. Both routes admitted.
- **Q4: confirmed** — observed:-contract projection is form-independent (contract cells only, both method forms).
- **Q5: confirmed** — fulfill mechanics unchanged for Subject-less bodies (default bodies, auto-satisfaction verbatim; only the Subject parameter absent).
- **Q1: CLOSED via Pins A+B** (owner-proposed design, confirmed 2026-07-20; executes as recorded unless vetoed):
  - **Pin A** — `Subject` in effect-kind methods ranges over ANY nominal type; NO fulfiller kind gate exists. The body's capabilities typecheck against the subject's actual kind (a node subject walks `.exposition`; a record subject reads fields; a `fulfill` whose body uses capabilities the subject's kind lacks fails ordinary type checking). Supersedes the i-open/i-graph fork; this is 005-51's replacement law. Subject-ful methods on records = trait-polymorphic effects over values; subject binds per standard effect-parameter rules.
  - **Pin B** — invocation closure: call form `method(subject_expr, …)` for all subjects (dispatch on the argument's type, per I-01-A); `x |> method` admissible exactly when `x` is a reactive cell — i.e. value subjects only, as ordinary Case 2 with trait-resolved callee (graph references are not cells, so graph subjects are call-form only; no third dispatch case returns); Subject-less methods are path-qualified effects:-entries per Q2-(i).
- **effects:-clause challenge (owner stress test): RETAINED, settled for now** — the clause survives as the discriminator for the body's only keywordless entry family AND the enforced world-boundary inventory; the per-entry-keyword rival was examined and rejected (spends the inventory guarantee to buy adjacency).

Execution note: Batch 1 executes as ONE batch (I-01 + I-24/α + Pins A/B + Q2/Q3 entries + I-57 both halves + DP-13), QUEUED BEHIND the consent batch (same files; orchestrators never run concurrently on the corpus). [Launched 2026-07-20 post-7785518.]

## Batch 3 — Commit Engine & Gates (owner, 2026-07-20)

- **I-04: (a)** — tiebreakers scoped: source-declaration-order governs SAME-INSTANCE same-level pairs only; cross-instance pairs are owned by placement-order-at-construction. Composed total order = (instance placement order, then intra-instance declaration order). Every pair matches exactly one rule. M0's deterministic trace is defined by this.
- **I-61: (1)** — the gate-transition dirty-root carve-out GENERALIZES: a gate transition (open or close) is a dirty root; the opening subtree's output-affecting deriveds join the SAME commit's DAG, evaluated after the flip in topological order; the gate predicate itself keeps reading prior-commit values. The fold/membership-descriptor carve-out becomes an instance of the general rule; 022-59's same-commit snap and the landed reopen hook order (re-snap, resume-then-update) now have their mechanism.
- **I-07: (B)** — dynamic-view membership is ACTIVATION-DRIVEN: a gated-off feeder (static or repeat-born) leaves the dynamic view's membership while effectively inactive and rejoins on gate-open (the same law as fold membership); static views keep counting frozen members (017-105 unchanged for static views). The line: static views count structure; dynamic membership follows activation.
- **I-06 residual: full anchor + sentence, §023 as owner** — the repeat iterate/diff/scope_obtain/scope_evaluate/scope_drop sequence is anchored to the numbered commit steps in the commit chapter (§023 family / SPEC §13.10.2), and the snapshot-visibility law is pinned: a scope dropped during a commit is absent from that commit's published snapshot; its final values are never published.

## Batch 1 ratifications + riders (owner, 2026-07-20)

- The four Batch-1 executed defaults are RATIFIED as landed: `divergent_runtime_type_arms`; the two `|>` rejection classes inside amended 029-106; §13.3.8's "pipe chain or constructor-call form" widening; the two extra DP-13 sites (016-242, §13.9.13).
- **Effect-method grammar gap: AUTHORIZED as a rider** on the next execution batch — SPEC §3.1 TraitBody BNF + §3.3 FulfillBody BNF + GRAMMAR §7.11/§7.12 gain the effect-kind method production (`EffectMethodSig`: `effect name(params):` with effect body; fulfill bodies may supply effect-kind method bodies), a mechanical conform to landed law (two forms per the Batch-1 amendments).

## Batch 5 — Placement/Lexing, Streams/Effects Surface, Groups/Folds (owner, 2026-07-20)

- **I-22: (A)** (owner rationale verbatim: "views are just for selection. children defines what is accepted and not and how much") — standalone-view cardinality is receiver-side selection, checked for satisfiability at compile time; caller obligations come from acceptance entries alone; the unscoped enforcement entries get scoped to acceptance-entry views.
- **I-34: (a)** — a letter glued onto a flag run (`Pin'!x`) is a LEX ERROR ("insert a space"); no whitespace-free instance naming.
- **I-11: (i)** — source-derived capacity inheritance governs desired:-block streams; the standalone 1024 leaves the effect-block sentence; §13.18.2's full defaulting rule is the single owner.
- **I-56: (B)** — 016-157 conforms to its SPEC anchor's scope: host-driven SIGNAL-CELL writes occur only through write_signal (directly or batched in a transaction); 016-286 remains the whole-surface two-channel law.
- **I-75: (a) with the addition** — error channels are author-opt-in, stated as law; the sanctioned place to MANDATE them is an effect-kind trait's observed: contract block (domain policy over language mechanism), stated alongside.
- **I-12: (1)** — one LOG equivalence sentence bridges the vocabularies pairwise (permanent=permanent, key-driven=keyed-template, activation-driven=gate-guarded); surface names for semantic prose, IR names for entry fields; neither renamed.
- **DP-17: (B)** (owner overrules the reference-only recommendation, rationale verbatim: "fold accepts something of type, not 'reference'... that's like asking what does 'fn' parameter accept — just variable references or inline values?") — the fold `<members>` slot is TYPED, not syntactically enumerated: it admits any expression of membership-structure type, including a name reference and an anonymous inline `collect:` block. Execution lands the header self-delimiting/parenthesization rule for the colon-bearing operand (cf. observe's parenthesize-or-newline convention); if that rule turns out gnarly, it comes back as a sub-ruling, never an invented default.

## Batches 2–4 ratification (owner, 2026-07-20): "no vetoes" — all seven executed defaults RATIFIED as landed (endpoint_read_requires_handle_form, stale_write_dropped, the canonical key-encoding rule, F1 wire-view inclusion, F2 verbatim prior-commit predicate scope, the I-51 riders, the window/step placements).

## Batch 2 — Wires & Endpoints (owner, 2026-07-20)

- **I-05: (A)+(A2a)** — a handle-typed endpoint derived (`derived target: WeakHandle[X] = handle to`) on a wire that has never been active holds a WeakHandle resolving `None` (the None-pin completes the always-defined-read law). (A2a): on a wire type whose destination can be unresolved from construction (WeakHandle destination), a NON-handle-typed derived reading `to`/`pair` is rejected at compile time — no lawful pre-activation value exists — with the diagnostic pointing at the handle form.
- **I-16: drop** — `Portal` comes out of SPEC §13.6.2's endpoint sentence; `WeakHandle` is the sole endpoint carrier, conforming SPEC to the LOG's sole-surface law and to Portal's non-graph kind-scoping.
- **DP-07: (A)** — the no-effect-beyond-opt-in entry gains the qualifier: satisfying Circularity also licenses the temporal-break/fixpoint semantics that cycles through the wire rely on (wake-gate cycles resolve by the prior-commit one-commit-delay fixpoint only when so guarded), citing the consent-batch-landed SPEC elaboration.

## Batch 4 — Reload (owner, 2026-07-20)

- **I-32: (a)** — a reload adds an explicit host step between diff and go-live: the runtime calls the host with the set of reconciler names required by the new program and not yet registered; any still missing when the window closes REJECTS the reload (old version keeps running, parallel to compile-failure reloads). Startup abort-on-missing unchanged.
- **I-49: (A)** — a value recurrent whose path+type match across reload preserves its VALUE AND its `.past` history buffer; a body (behavior) change is not retroactive — the next evaluation runs the new behavior over the preserved history. Rider (N-4, same root): reload step 8's recompute rule names recurrents explicitly with this no-retroactive-recompute semantics.
- **I-50: (C)** — queued host writes are VALIDATED at apply time: a write whose target path no longer exists after the reload/restart is dropped and counted in a runtime diagnostic, never applied to a wrong or dead target (generation-stamp philosophy at the API boundary). Quiesce rejected; blind apply rejected.
- **I-51: (b)** — a trap during reload's pre-live first evaluations ABANDONS THE RELOAD: added cells are discarded, the previous version keeps running, the trap surfaces as a reload-failure diagnostic. Stated consequence pinned with it: the pre-live first-evaluation phase is a sanctioned machinery-level containment boundary for traps — a deliberate carve-out from no-continuation-past-a-trap, which otherwise stands (startup first-eval traps still abort the process).
- **I-60: (a)** — the wire-format identity grammar gains `.<key>.` as a first-class multiplicity segment: the stringified StringifiableKey form of a repeat scope key. Execution flag: the text-format encoding needs an escaping/quoting rule for keys containing `.` — if nontrivial, comes back as a sub-ruling, never an invented default.
- **I-79: (a)** — per-instance restart governs operator-internal cell type changes; the reload-safe remove+add entry is scoped to non-operator-internal cells (LOG conforms to SPEC; no behavior change).
- **I-80: two-axis framing** (owner verbatim): class = which restart the change would require; disposition = restart-or-reject, implementation-defined for either class.
- **I-74 residual: (a)** — visibility is a source-language concept and does not cross the host boundary; one-line note at the host-API surface. No IR name-mangling.

## Batch 6 — Traits & Types, Ownership (owner, 2026-07-20)

- **I-23: pin the conjunction sentence** — a trait's fulfiller gate is the conjunction of each member's own requirement (required cells → node/wire; endpoints → wire; effect-kind methods → none). New 005-family entry; the old empty-intersection finding dissolved via Pin A.
- **I-28: (B), owner verbatim: "'::' period. T::Item must be the access, not T.Item. non-negotiable request regardless of how big of a change this is."** — associated-type access is `::` EVERYWHERE at type level: `T::Item`, `T::Iter::Item`, `Iterator::Item`; where-clause predicates respell (`where Iter::Source is Subject` — the rider-1 `is` boundary untouched, only the projection spelling flips; rider-1's exec sites re-swept per the superseding-ruling rule). 014-157 inverts; GRAMMAR AssocPath `('.' IDENT)+` → `('::' IDENT)+`; corpus-wide survey-then-sweep classifying type-level projections (flip) vs value member access (stays). Resulting razor: `.` is VALUE member access only; `::` reaches through types/namespaces. Executed as its own orchestrated sweep inside the Batch 6 execution (owner: "you have the workflow, use it").
- **I-30: any-scope** — suffix-family reservation binds in any scope; 005-236 conforms to its own cited section.
- **I-38: (a)** — the §1.3 referential-transparency floor defines inputs = arguments + provenance set (the cells the call transitively reads).
- **I-53: (i)** — canonical structural-exception enumeration: tuples, closure types, trait-constraint intersections, Map; narrower claims scope-qualified to their actual dimension (e.g. conformable-but-declaration-less = tuples + closures).
- **I-77: (b)** — normative warning-class lint when a bare call resolves to a trait impl while a same-named free function is in scope; path-qualification silences; coin the class name (flag as default).
- **DP-08: (B)** — `(r.handler)()` is THE invocation form for field/index-held closures (ordinary call postfix on a field read); bare `r.handler()` stays unknown-method with a hint pointing at the parenthesized form.
- **I-64: qualify both** — 013-153 AND SPEC §11.9.1 blanket Copy-storability statements gain the closure exception (no cell type admits a closure; fields/tuples/payloads stay legal).
- **I-29: add** — "indexed slot" joins 013-42's closed consuming enumeration.
- **I-68: (a)** — placement-attr positions consume an own-param RHS implicitly (category B); the category-A own/move law is scoped to call-argument positions.

## Batch 7 — Collections/Numerics, Modules/Scopes, IR, Directives (owner, 2026-07-20)

- **I-10: per-element pool index** — a fixed-extent array cell of a dynamic-size element type stores one word-size pool index per element; N×word satisfies the fit rule.
- **I-25: empty string** — reversed slice bounds yield the empty string per the range convention (start >= end → empty).
- **I-27: isize** — `.count` returns `isize`, family-consistent; all divergent example sites (usize/u32/i32) conform.
- **I-47: bracket** (owner verbatim: "a map key is a map key, regardless of whether its plain or composite value") — composite-map slot paths spell `<binding>['<key>']`; the SPEC dot-form site conforms.
- **I-48:** `v` evaluates once iff N ≥ 1; requires `T: Clone`; clones N−1 times; N=0 evaluates zero times.
- **I-54: normative** — string backing sharing is refcounted-shared as law; O(1) duplication binds.
- **I-67: rename** — `byte_len` → `byte_count`, conforming to the x_count naming law.
- **I-41: dedicated tier below the range tier** — `in` binds looser than `..`, tighter than `not`; both justification texts conform.
- **I-62: trap + const compile error** — shift count ≥ bit width traps; compile-time-known counts are compile errors.
- **I-63: each operator's defined semantics** — const evaluation mirrors each operator's runtime law; wrapping/saturating/checked results const-fold; only trapping-operator overflow rejects.
- **I-44: specificity wins** — selective import beats glob; local declaration beats glob; only symmetric glob-vs-glob errors.
- **I-55: declare the exception** — bare-type-rooted variant imports are the sanctioned exception to the absolute-path law (type itself in scope). EXTENSION QUESTION OPENED by owner (bare variant construction without variant imports) — under discussion, recorded when ruled.
- **I-65: defer** — §8.9 points at §13.13 as the single owner of the trap-construct list.
- **I-78: exception clause + phrase fix** — nearest-wins entries gain the body/module ambiguity-error exception; the "shadowed by a member" trap phrase is corrected (that case errors).
- **I-18: (B)** — open-base-plus-extensions copula at 033-43; a Handle-enumerating entry; grammar conformed including `yielded<T>` (Portal/arrays already landed via consent I-19).
- **R-6: (a)** — mixed own-line + inline directive stacks are legal; source order governs (own-line top-to-bottom, then inline left-to-right).

## I-55 extension — bare variant construction (owner, 2026-07-20): (β), (α) REJECTED on the evolution trap

Bare variant construction resolves from the EXPECTED TYPE — a variant placeholder resolved by §2.1's use-site machinery, exactly as pattern position already resolves bare variants from the scrutinee: `Ok(value)` legal wherever the expected type is a known enum (return positions, annotated bindings, field/attr positions), zero imports required; no expected type → compile error directing to annotation or qualification; qualified `Enum::Variant(...)` always works; variant imports remain for the no-expected-type minority. NO scope injection: importing a type never injects variant names into bare scope — adding a variant to a library enum is never a downstream breaking change. New law: 009-family entry + §2.1/§6.2 mirrors.

**THE PART 3 RULING QUEUE IS CLOSED.** Remaining work is execution only: Batch 5 + grammar rider (in flight) → Batch 6 (incl. the `::` associated-type sweep) + Batch 7 + I-55β as the final execution wave → verification chains → commits → consolidated Part 3 final report.
