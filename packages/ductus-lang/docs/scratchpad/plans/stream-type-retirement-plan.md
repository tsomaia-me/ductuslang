# Stream-Type Retirement & Sealed De-Magic Amendment Plan

*2026-07-13 · Executes the owner rulings of 2026-07-13 recorded in the audit-plan-2 rulings ledger:
STREAM-TYPE-RETIREMENT (a), the `stream[P] T` spelling, WIRING-TYPES vocabulary fold,
SEALED-DEMAGIC (all four pins), the concrete-bracket-form ruling (a), plus two ruled riders
(COORDINATE-PATH (a), 023-53 (a)). Status: rulings LOCKED, plan approved for drafting;
execution launches on the owner's word.*

---

## 1. Background — why this amendment exists

The corpus already retired `Cell[T]`, `Signal[T]`, `Derived[T]`, `Recurrent[T, N]` in favor of
lowercase spellings. The reason, sharpened by the owner during review: **brackets name types,
types name values — and wiring has no values.** A node type is a genuine type because its
instances are genuine values (graph-owned, held by borrow). A cell is not a value at all, so it
gets no value-type spelling.

Three leftovers violate that principle, and this amendment removes all three:

1. **The stream type family survived.** `Stream[T, P]`, `RingStream[T, N]`, `GateStream[T, N]`
   still exist as bracket types. A stream is wiring — the same way `derived` and `yielded` are
   wiring. If `Stream[T, P]` is a type, `Vec[RingStream[f32, 64]]` is writable: a stored
   collection of wiring, exactly the absurdity that killed `Cell[T]`. The family survived only
   because the cell-spelling decision explicitly deferred it; the deferral ends here.

2. **The vocabulary conflates two levels.** Several entries say a kind annotation is
   "not a type." The owner's clarification: the kind *keyword* alone (`stream`, `derived`,
   `cell`) is a kind; the *applied annotation* (`stream T`, `derived T`, `stream[P] T`,
   `cell T`) IS a type — part of the type system, in a distinct class: **wiring types**,
   unstorable by nature, expressing wiring rather than values. The corpus needs that two-level
   split written in.

3. **Sealing is magic.** The whole sealing mechanism is one sentence: Into and TryInto are
   "never implementable by users." No user surface, no stated mechanism — a capability the
   language has and the user does not, which the no-privilege principle forbids. The owner's
   ruling converts it into ordinary module scoping that users get too.

## 2. The rulings (owner, 2026-07-13 — all LOCKED)

**R1 — Finish the retirement.** `Stream[T, P]`, `RingStream[T, N]`, `GateStream[T, N]` retire
as types. No cell of any class has a type spelling with value-type semantics.

**R2 — The policy-generic spelling is `stream[P] T`.** Concrete instantiation by substitution;
the bracket form is legal in concrete positions too (ruling (a)), with the word form as the
idiomatic sugar:

```
operator map[T, U, P: StreamPolicy](source: stream[P] T, f: fn(T) -> U) -> stream[P] U

stream[Ring[64]] f32      // legal concrete spelling — the expansion
stream ring[64] f32       // the idiomatic word-form sugar for the same thing
```

`Ring[const N: usize]` and `Gate[const N: usize]` stay exactly what they are: language-provided
const-generic marker types fulfilling the sealed trait `StreamPolicy`. Kind brackets ADMIT
parameters — const ints (`recurrent[N]`, capacities) and StreamPolicy-bounded policy types
(`stream[P]`). That is the legal direction, distinct from the banned one (wiring types never
appear inside value-type constructors).

**R3 — Wiring-types two-level vocabulary.** The kind keyword alone is a kind and a keyword; the
applied annotation is a wiring type: a member of the type system, unstorable by nature. Claims
of the shape "there is no `Cell[T]` type" (bracket type-hood denials) stay true and stand; claims
of the shape "the annotation is a kind, not a type" get the two-level split.

**R4 — Sealed de-magic, four pins confirmed.** `sealed` is a user-writable trait modifier:

```
sealed trait ColorSpace          // any module may declare one

type Rgb
fulfill ColorSpace for Rgb       // ✓ same module as the trait

// elsewhere:
fulfill ColorSpace for MyThing   // ✗ sealed_trait_fulfillment_outside_module
type T2: satisfies ColorSpace    // ✗ same error — bare marker claims are the second door
```

Pins: (1) the module unit is the same unit as the orphan rule; (2) sealing bars BOTH doors —
`fulfill` blocks and bare `satisfies` marker claims; (3) stated relation: sealing disables the
orphan rule's type-ownership arm — trait-module only; (4) `sealed` becomes a reserved word on
the trait declaration (keyword-class assignment rides the keyword-classes phase); the diagnostic
class is `sealed_trait_fulfillment_outside_module`. `Into`, `TryInto`, and `StreamPolicy` become
ordinary sealed traits declared in the language core module — no privileged mechanism remains.

**Doc requirements (owner):** the sealed-traits section references the modules chapter (a Ductus
module is a folder of `.duc` files — §10.2 — with semantics unlike e.g. JavaScript modules;
readers need the jump), and every sealed-trait site (Into, TryInto, StreamPolicy) carries a
quick-reference pointer to the sealed-traits section.

**R5 — Rider: coordinate-path rule (a).** A group member's scope path NESTS: the yield-site is
its own path level (declaration-order index within the `collect` body, mirroring the anonymous-
placement ordinal convention), and the producing repetition key sits under it as an ordinary key
component. Each component stays a simple stringifiable value, so the existing scope-key rules
hold unchanged.

```
mixer.parts.1              // the permanent member from yield-site 1
mixer.parts.2.voice_7      // the repeat-born member: site 2, key voice_7
```

Accepted property (owner-acknowledged): inserting a `yield` line above shifts later site
numbers across a reload, like anonymous `:N` ordinals today.

**R6 — Rider: 023-53 rewrite (a).** In place, bridge vocabulary, keeping the two-kinds-of-
dependency content:

> 023-53. Dynamic views and connection-views are the collective form of handle resolution: a
> `repeat` over a view — including the `repeat` inside a `collect` block — declares a dependency
> edge into the membership cell itself and, per member scope, into the member cells that scope's
> body reads. (§13.10.5)

## 3. Defect tally riding this amendment (all caught during the ruling review)

- **030-37**: says "one of the **four** `StreamPolicy` members" — the pre-D-27 count — and
  "every stream **value**" — wiring-vocabulary violation. Rewrite: every stream has a concrete
  policy (one of the two fulfilling types) fixed at compile time.
- **030-258**: "its **members** are exactly Ring[N] and Gate[N]" — traits have no members;
  conform to "the types fulfilling it are exactly …" and state it as a `sealed trait` per R4.
- **stdlib-for-builtin wordings**: 030-261 ("The **stdlib** provides … aliases") and SPEC's
  "standard library provides" alias prose — these die with the aliases; any survivor rewords to
  "the language provides." (Genuine stdlib items — Vec fulfillments, `alias type byte=u8` — are
  NOT in scope; the language/stdlib boundary follows the Map-vs-Vec precedent.)
- **§10.9 orphan-rule wording**: says the orphan rule keys on *package*-of-declaration while
  §3.7.1 says *module M* — a pre-existing contradiction. The module-granularity ruling (D-24)
  settles it: module. Sealing's pin (1) depends on this, so the one-passage conform folds in
  here rather than waiting for the structure phase.
- **The satisfies-route loophole**: the marker-trait rule (005-65 exception 1) lets a type claim
  a marker trait with no `fulfill` block; nothing said sealing bars that door. Pin (2) closes it.

## 4. Phased execution — LOG first

Editors: fresh reads, verbatim old_strings, no entry-number citations in entry text or briefs,
dense numbering per section with header-count updates, tail-append for new entries (ratified
convention), flag-don't-halt on same-class extras, per-file greps only.

### Phase A — LOG, sealing (sections 010, 005-region, 003-region)

- **010-9** rewrite: `sealed` is a trait-declaration modifier available to all code; a sealed
  trait is usable in trait bounds and method dispatch everywhere, but fulfillment claims — both
  `fulfill Trait for Type` blocks and bare `satisfies Trait` marker claims — are legal only in
  the trait's own declaring module; the diagnostic is `sealed_trait_fulfillment_outside_module`;
  `Into` and `TryInto` are sealed traits declared by the language. Pointer stays (§7.2) plus the
  new sealed-traits section.
- **010-12/010-13** verify/conform: the "coherence rule, not stdlib privilege" framing becomes
  literally true — reword to reference the sealed mechanism where they restate the ban.
- **NEW entries at the traits section tail** (topic home: trait fulfillment rules; next free
  numbers, verify tail at execution): (1) the sealed modifier and its module rule (both doors,
  restated locally); (2) sealing's relation to the orphan rule (disables the type-ownership arm;
  trait-module only; same module unit); (3) the reserved-word status of `sealed` (class
  assignment deferred to the keyword-classes phase).
- **Orphan-rule granularity conform**: the LOG-side orphan entries verify as module-worded (the
  module-granularity ruling); flag any package-worded stragglers.

### Phase B — LOG, stream de-typing (sections 030, 031, 004, 016-region)

- **030-36** rewrite: a stream annotation is the wiring type `stream[P] T` where
  `P: StreamPolicy`; the word forms `stream ring[N] T` / `stream gate[N] T` are the idiomatic
  sugar for `stream[Ring[N]] T` / `stream[Gate[N]] T`; the alias types are RETIRED.
- **030-37** rewrite per defect tally. **030-258** rewrite per defect tally (+ sealed-section
  pointer). **030-261** rewrite: the two-axis model and the sugar relationship in the new
  spellings; no alias types; "the language provides."
- **Signature respells** (class mandate — the listed ids are a floor): 030-39, 030-41, 030-44,
  030-45, 030-46 (widening: `stream[P] T` widens to erased `stream T` — restate in wiring-type
  family terms, drop "supertype" if it implies value subtyping), 030-132, 030-134, 030-141,
  030-142, 030-143 (merge: `stream[Ring[A + B]] T`, call-site override intact), 030-145,
  030-149, 030-151, 030-163, 030-177, 030-178, 030-179; 031-27 (effect stream params respell to
  the new forms).
- **§004 const-generic examples** (004-121, 004-124, 004-129, 004-133, 004-134, 004-138):
  respell `RingStream[T, N*2]`-style examples to `stream[Ring[N * 2]] T` forms. Execution note:
  the const arithmetic moves INSIDE the policy type's bracket; add one bridging clause where
  §004 first shows it — const-generic inference and canonicalization apply to kind-bracket
  parameters identically (the machinery is unchanged; only the host spelling moved).
- **Wiring-types vocabulary split** (016-62, 016-163, 016-166, 016-178, 016-180, 017-189,
  030-47, 030-51): rewrite "kind annotation, not a type" phrasings to the two-level form — the
  keyword is a kind; the applied annotation is a wiring type (a type-system member, unstorable
  by nature, never inside a value-type constructor); bracket-type-hood denials ("there is no
  `Cell[T]` type") stand unchanged.

### Phase C — LOG, riders (sections 018, 023)

- **NEW 018 entry** (tail): the group-member scope-path rule (R5): nested components —
  yield-site index level, then the producing repetition key for repeat-born members — each
  component a simple stringifiable value; the site index follows the anonymous-ordinal
  convention and shifts the same way across source edits. 018-28/018-29/018-88 verify
  unchanged (the rule composes with them; nothing contradicts).
- **023-53** rewrite in place with the ruled text (R6, exact text in section 2).

### Phase D — SPEC conformance

- **§3.7.x**: new sub-subsection "Sealed traits" adjacent to the strict orphan rule (§3.7.1):
  the modifier, the module rule (both doors), the disabled type-ownership arm, the diagnostic,
  a worked example, and the modules-chapter reference (§10.2) per the doc requirement.
- **§10.9**: the package-worded orphan sentence conforms to module (D-24).
- **§7.2**: rewrite the sealed passage — Into/TryInto are `sealed trait` declarations in the
  language core module under the general mechanism; quick-reference pointer to §3.7.x; the
  From/TryFrom auto-derive content is untouched.
- **§13.18.3** (20726 region): the two-axes text keeps its content; the `trait StreamPolicy`
  block gains the `sealed trait` spelling + §3.7.x pointer; the convenience-alias block
  (~20769–20781) is REPLACED by the sugar-relationship statement (word form ⇄ bracket form);
  the "erased supertype" passage restates as: erased `stream T` is the policy-erased wiring
  type every `stream[P] T` widens to (zero-cost, view-only).
- **§13.18.9 / §13.18.10.3 / §13.18.11 / §13.19.3**: operator-signature and policy-as-type
  respells (the surveyed line clusters; re-grep at execution).
- **§13.18.1** (20627), **§2.3.6 / §2.5.3–2.5.7** (the const-generic chapter): example respells
  per Phase B's §004 note, plus the one bridging clause at first use.
- **§13.2.8 / §13.2.8.1 / §13.18.5 / §15.4.1**: the two-level vocabulary split at the surveyed
  kind-not-type sentences (12563, 12623, 12626, 12696, 12708–12716, 20825/20833/20868, 24532);
  "no `Cell[T]` type" sentences stand.
- **§13.5.4.4 + §13.20.4**: the nested scope-path form for group members (R5) with the
  `mixer.parts.2.voice_7` example; **§13.10.5** verify (already bridge-form; 023-53 re-converges
  with it).

### Phase E — Grammar documents

- **GRAMMAR.md**: KindAnnotation production (1267 region) gains the policy-bracket forms
  (`stream[P] T` generic, `stream[Ring[64]] T` concrete) beside the word forms; the type-table
  rows 6216–6220/6228/6238/6278 rewrite (no `Stream[T, P]` type row; the wiring-type family
  described; example respelled); keyword inventory (§2.4) gains `sealed`; trait-declaration
  production gains the optional `sealed` modifier.
- **IR_GRAMMAR.md**: expected NO-OP for streams (the IR already spells streams lowercase with
  policy tokens; verified zero bracket-family mentions). Verification-only pass.

### Phase F — Gates, review, capture

1. gate-count: touched sections dense, headers exact.
2. gate-invariant-2: zero entry-number citations on changed lines.
3. gate-retirement: `Stream\[`, `RingStream`, `GateStream` = 0 in all four docs (no sanctioned
   survivors remain — this gate is now absolute, unlike the old classified-list gate).
4. gate-sealed: `sealed trait` present at the three language sites; the diagnostic name present
   in LOG and SPEC; "never implementable by users" phrasing gone; both-doors rule present.
5. gate-vocabulary: "not a type" survivors all of the bracket-denial shape; two-level split
   present at the surveyed sites; "four `StreamPolicy` members" = 0; "stream value" = 0 in §030;
   stdlib-for-builtin survivors = 0 in stream/policy contexts.
6. gate-riders: 023-53 matches the ruled text verbatim; the nested-path entry present; §13.5.4.4
   carries the group form.
7. gate-regressions: collect-as = 0; `Cell[`/`Signal[`/`Derived[`/`Recurrent[` counts do not
   grow (LOG 0; SPEC only its sanctioned negation/historical mentions); four-tag cell-kind enum
   unchanged; on/off = 0; `keyed_by_on_yielded_group_rejected` still present.
8. gate-xref: every (§) in changed/added entries resolves to an elaborating SPEC heading.
9. Fresh-eyes adversarial review: three scoped blind reviewers (LOG / SPEC / grammar docs), each
   with its file's diff + the rulings ledger + this plan; orchestrator adjudicates, fixes valid,
   re-runs affected gates.
10. Diff capture to scratchpad; final structured report; no commits.

## 5. For execution — surveyed sites (as of 2026-07-13, HEAD after the owner's push; line
numbers WILL drift — locate by content/id, re-grep per file)

> **LOG ids.** Sealing: 010-9 (1080), 010-12/13 (1083/1084); new trait-section entries at tail
> (verify tail count at execution). Stream family: 030-36 (3551), 030-37 (3552), 030-39, 030-41,
> 030-44..46, 030-132, 030-134, 030-141..143, 030-145, 030-149, 030-151, 030-163, 030-177..179,
> 030-258 (3773), 030-261 (3776); 031-27 (3806); 004-121/124/129/133/134/138 (305–322).
> Vocabulary: 016-62 (1922), 016-163 (2023), 016-166 (2026), 016-178 (2038), 016-180 (2040),
> 017-189 (2338), 030-47 (3562), 030-51 (3566). Riders: 023-53 (3070); new 018 entry after
> 018-144 (2622). Section tails verified: 010=45, 016=286, 018=144, 030=261.

> **SPEC regions.** §3.7.1 (2510) + new sealed subsection; §7.2 (5811, passage 5813–5845);
> §10.2 (7673, reference target); §10.9 (8177, package→module conform); §13.2.8 (12563, 12623,
> 12626), §13.2.8.1 (12696, 12708–12716); §13.18.1 (20627); §13.18.3 (20735–20818);
> §13.18.5 (20825, 20833, 20868); §13.18.9 (21395–21504); §13.18.10.3 (21615–21616);
> §13.18.11 (21708–21728); §13.19.3 (22363); §15.4.1 (24532); §2.3.6 (572–579),
> §2.5.3–2.5.7 (965–1149); §13.5.4.4 (15766–15774); §13.20.4; §13.10.5 (verify-only).

> **GRAMMAR.md.** KindAnnotation 1267 (+ reuse sites 3344, 3347, 3686, 3789, 3795, 3855, 6230);
> keyword inventory 225–365; type-table 6216–6220, 6228, 6238, 6278; trait-decl production for
> the `sealed` modifier. **IR_GRAMMAR.md**: verification-only (zero bracket-family mentions;
> stream production at 255 already lowercase).

## 6. Verification greps (run per file after Phase F)

```bash
grep -n 'Stream\[' DECISION_LOG.md          # expect 0
grep -n 'RingStream\|GateStream' DECISION_LOG.md   # expect 0   (same for SPEC.md, GRAMMAR.md)
grep -n 'four .*StreamPolicy\|StreamPolicy members' DECISION_LOG.md  # expect 0
grep -n 'never implementable by users' DECISION_LOG.md SPEC.md      # per file; expect 0
grep -n 'sealed_trait_fulfillment_outside_module' DECISION_LOG.md   # expect definitional hits
grep -cn 'sealed trait' SPEC.md             # expect >=4 (§3.7.x, §7.2, §13.18.3, example)
grep -n 'stream\[P\] T\|stream\[Ring' DECISION_LOG.md               # expect the new spellings
grep -n 'stdlib' DECISION_LOG.md | grep -i 'stream\|alias\|polic'   # expect 0
grep -n '^023-53\.' DECISION_LOG.md         # expect the ruled bridge text
python3 lint_nnm.py DECISION_LOG.md         # numbering/density lint if available; else gate-1
```

## 7. Out of scope, deliberately

- The keyword-class assignment of `sealed` (the keyword-classes phase owns keyword taxonomy).
- The IR behavior-grammar operand/`ret` gap and IR_GRAMMAR's §2/§6 citation-drift sweep
  (pre-existing, flagged, own items).
- The main-removal amendment (parked on its eight confirms; gains the reference-reachability
  closure pin and the unwired-instance ownership note from this review).
- Genuine stdlib content (Vec fulfillments, `alias type byte=u8`) — the language/stdlib
  boundary itself is untouched.
