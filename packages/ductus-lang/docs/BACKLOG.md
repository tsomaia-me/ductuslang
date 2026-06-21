# Ductus Design Backlog

Working record of design decisions reached in discussion but **not yet written into
`DECISION_LOG.md` / `SPEC.md`**. This is the bridge between a design conversation and the
decision-of-record: once a `LOCKED` item is amended into the log+spec, it should be removed from
here (or marked LANDED).

It is **not** normative. `DECISION_LOG.md` remains the decision-of-record; where this file and the
log disagree, the log wins until an amendment lands.

**Status legend:** `LOCKED` (agreed, awaiting spec amendment) · `OPEN` (under design, not settled) ·
`DEFERRED` (agreed to handle later) · `DISCARDED` (considered and rejected) · `PARKED` (future-form,
not now).

Entry shape: **Background** (origin, the problem) → **Decision** → **Why** → **Example** →
**Spec impact**.

---

## 1. Triage of the initial 13 proposals

A batch of 13 proposals was reviewed. Several resolved to "no spec change," recorded here so they
are not re-litigated.

### 1.1 `DISCARDED` — Explicit lifetimes (proposal #2)
- **Background:** Ductus has implicit borrow-prevention (a borrow can't outlive its root) but no
  explicit lifetime annotations like Rust's `'a`. Question: should it?
- **Decision:** No explicit lifetimes.
- **Why:** Ductus forbids the thing that forces lifetimes in Rust — storing borrows in data. A
  borrow can't be put in a record/tuple/enum/cell and can't be captured by an escaping closure, so
  the only place a borrow's lifetime needs expressing is *function-return rootedness*, which is
  inferred from the body (with the optional `-> T from v` clause as the one explicit knob). Nothing
  else to annotate.
- **Spec impact:** None (confirms existing §11 model).

### 1.2 `DISCARDED` — Sync/Async connection markers (proposal #4)
- **Background:** Proposed two marker traits, `Sync`/`Async`, to tell the runtime whether a
  connection must be processed before traversal continues, or alongside it.
- **Decision:** Not adding them as language markers.
- **Why:** Engagement sequencing is already host-domain: 017-199/200/201 say "what engagement means
  is domain semantics carried by the connection type and interpreted by the host… the language and
  IR carry no sequencing flag." Await-like vs parallel is explicitly the host's call. Language-level
  markers would reverse that. A domain may still define its own such markers.
- **Spec impact:** None.

### 1.3 `DISCARDED` — Heterogeneous-tuple / value-tuple / type-tuple iteration (proposal #1)
- **Background:** Wanted to iterate the components of a tuple, or a list of types.
- **Decision:** Not provided (matches existing 014-63, 012-62).
- **Why:** The enum + fixed-array + `match` pattern covers the real use cases; first-class tuple
  iteration would need per-element polymorphic bodies (variadic-style), which Ductus excludes.
- **Spec impact:** None.

### 1.4 `PARKED` — Future-forms list (proposal #3)
- **Background:** A parking lot of "maybe later" forms.
- **Items:** `[=1 total]` enforced-total cardinality (today totals aren't tracked, 017-124);
  per-member visibility on `to`/`from`/`parts`; an instance knowing its own placement-time name;
  union types (the connection `to:` admission rule is forward-compatible with them, 030-68/019-33);
  a reflection API (would widen what bare `Type[Node]` can do, 008-69).
- **Decision:** Parked; not now. Note: "union types" and "reflection" are the same questions as the
  metaprogramming / `Type[…]` threads and should be revisited together if pursued.
- **Spec impact:** None yet.

### 1.5 `NO-ACTION` — `instant` time type (proposal #10)
- **Background:** Wanted a monotonic time type with `instant - instant = duration`.
- **Decision:** Already specified — 012-93 (`instant`/`duration` primitives) and 012-132
  (`instant - instant -> duration`), full op set 012-128..143. Nothing to do.

### 1.6 `DEFERRED` — Cluster B: `Cell` return/storage gap (#12) + portals (#13)
- **Background:**
  - #12: Can a function read a top-level `Cell` and *return* a `Cell`? Reading is allowed and
    provenance-tracked (025-15/16), but the **ownership category of a `Cell[T]` value** (borrow vs
    storable) and what *returning* one means are underspecified.
  - #13: "Portals" — storable, `Copy`, always-`Option`-on-access references to **owned, non-graph**
    data (records, etc.), to enable self-referential structures (doubly-linked lists). Distinct
    from `Handle` (which is graph-only).
- **Decision:** Deferred to its own discussion. Open framing: portals look mechanically near-identical
  to `Handle` (Copy, Option-on-resolve, address-like) — first question is whether this is just
  *generalizing `Handle`* to non-graph data, or a stdlib slotmap on the `raw_*` intrinsics (033-193),
  rather than a new primitive. Hard parts: liveness for arbitrary owned data (generation stamps),
  and whether portals are ever writable (read-only guts the doubly-linked-list splice case).
- **Spec impact:** TBD — own discussion.

### 1.7 `DEFERRED` — Connections under the views model
- **Background:** The node-side views redesign (§4 below) intentionally left connections out.
- **Decision:** Deferred. Notes captured for later: a node's supplied connections are always
  *outgoing*; `outgoing` already constrains them; whether connection *access* reuses `outgoing` or
  takes its own keyword is open; connection-views would need a direction (incoming vs outgoing),
  since a connection type doesn't encode which side a node is on.
- **Spec impact:** TBD — next discussion.

---

## 2. `LOCKED` — `@reset_on_reopen` unification (origin proposal #11)

- **Background:** Today `@reset_on_reopen` is two disconnected per-kind rules — one for value
  recurrents (016-54: clears self+input history on the gate false→true edge) and one for stream
  consumers (016-57/058: resets cursor to head, drops backlog, releases buffer hold). A
  blast-radius pass confirmed the **recurrent-stream** reopen case is *unspecified*, and that
  `@reset_on_reopen` has **no IR representation at all** (whereas `@reset_on_reload` does, 033-104).
- **Decision:**
  1. State one **governing principle**: *`@reset_on_reopen` discards the state a cell accumulated
     during a gated gap, on the gate false→true (reopen) edge; what counts as gap-state is defined
     per cell kind; on a stateless cell it is a harmless no-op (cf. 016-59).*
  2. Fold 016-54 and 016-57 under it as the per-kind instantiations.
  3. Make the **recurrent-stream** case explicit: on reopen, reset **both** output+input history
     **and** the consumer cursor/backlog — the same union `@reset_on_reload` already states for the
     reload edge (030-251).
  4. The **effect-`desired:` recurrent** case is covered by the general principle (a recurrent in a
     gated `desired:` arm clears its history on reopen); add a §13.19.4 cross-reference.
- **Why:** Two ad-hoc rules don't compose — a recurrent stream is *both* a recurrent and a stream,
  and neither rule alone says what reopen does to it. A single principle ("drop gap-accumulated
  state, per kind") closes that gap and the effect-`desired:` gap for free.
- **Example:**
  ```
  @reset_on_reopen
  recurrent[3] avg: f32 = (avg.previous(0.0) + sample) / 2
  // gated off → reopened: self-history cleared; first post-gap read returns fallbacks.

  @reset_on_reopen
  recurrent[3] stream ring smoothed: f32 = (count + smoothed.previous(0.0)) / 2
  // NEW: reopen resets BOTH output+input history AND cursor/backlog (was unspecified).
  ```
- **Spec impact:** LOG — new governing-principle decision in §13.2.4 (next free §016 id), new
  recurrent-stream-reopen decision (next free §030 id), light cross-refs on 016-54/57, 022-53,
  §13.19.4. SPEC — §13.2.4, §13.18.8 (new "Gating and `@reset_on_reopen`" subsection), §13.18.12,
  §13.9.7, §13.15.5. Reload-safe: 028-10/11 unaffected. (The IR-representation gap is OPEN — §5.)

---

## 3. `LOCKED` — Instance citizenship reframe

- **Background:** The spec asserts node/connection/effect *instances* are "never a value"
  (016-219 → §13.2.10, with parallels 017-139, 031-5). On scrutiny that axiom is misleading: every
  restriction it encodes already follows from general rules, and calling instances "not values"
  doesn't explain what they *are* (you can name them, read their attrs, take a `Handle`, pass them
  as endpoints).
- **Decision (verbatim):** *"Node, connection, and effect instances are first-class citizen values;
  value-semantics (relocate / copy / structural-compare) is an orthogonal property they lack; every
  restriction on them is entailed by the general ownership and borrow rules, not a special case."*
  - First-class citizenship (nameable / passable / returnable) is **orthogonal** to value-semantics.
    Functions are also first-class citizens. A closure is a citizen *with* value-semantics; a
    node/connection/effect instance is a citizen *without* it.
  - Drop "never a value" from 016-219 and parallels; replace with the entailment.
- **Why:** The unstorability/non-relocatability/borrow-only access all derive from: external-state
  immutability (013-8), single ownership (013-9), and the general non-storability of borrows
  (013-52) — plus "external state is created only at declaration sites, never in functions." No
  node-specific axiom is needed. This is a **framing/justification correction with no observable
  rule change**; it's also the foundation the views model (§4) rests on (a view yields borrows of
  instances, and instances are values).
- **Example:**
  ```
  node Mixer:
    view channels: Channel+
  // `channels[0]` is a borrow of a Channel instance. The instance IS a value (first-class);
  // you reach it through a borrow and cannot relocate/copy/store the instance itself.
  let kept: Handle[Channel] = weak channels[0]   // to persist, take a Handle (storable)
  ```
- **Open sub-point:** the *word* "first-class" already has loose uses in the spec (016-163
  "first-class type", 030-2 "first-class reactive cells", 019-2 "first-class entities", 030-254
  "not first-class values"). The framing is locked; the label/disambiguation is OPEN (§5).
- **Spec impact:** LOG — revise 016-219, 017-139, 031-5; add the citizenship/value-semantics
  distinction in §1.3 (one or two new §001 ids — count is OPEN). SPEC — §13.2.10, §5.7
  (opening + §5.7.4), §13.3.6.1, §13.19; minor §3.7.4, §1.3. No IR change (instances already lower
  to `scope`, type-erased: 033-56/063/159). Minimal-edit discipline: most 013/017/019/021 borrow
  decisions are already correct and need at most a cross-reference.

---

## 4. `LOCKED` — Views-only node model (fuses proposals #6, #7, #8, #9)

A re-foundation of how a node declares and accesses its children. Replaces the `parts:`/`parts`
machinery with a single `view` mechanism. Rests on §3 (instances are values; views yield borrows of
them). **Largest change here** — rewrites §13.3.3, §13.4, §13.3.7.2, parts of §13.3.4.

### 4.1 Removed
- **`parts:` clause — removed.** Constraints are expressed through views.
- **`parts` field (catch-all) — removed.** A catch-all is just a `Node`-typed view.
- **Default exposition — removed.** No implicit `expose: parts`. `expose:` is required for any
  structural output. A node may take children purely to read their data and expose nothing —
  access and exposition are independent. (Removes 017-180/181/182.)

### 4.2 Views
- **Decision:** Children are accessed through `view name: <selector> <cardinality>` declarations — a
  new declaration kind parallel to `attr`, living in the body name-scope, names unique. `<selector>`
  is a concrete type, a trait, or a marker (`Node`). Views are **receiver-side queries** over the
  whole supplied set: they overlap, they do not partition.
- **Why:** Names give addressability (which type-keyed access can't, e.g. for groups); trait
  selectors give heterogeneous access; it subsumes both #7 (named declarations) and #9 (slots).
- **Example:**
  ```
  node Mixer:
    view inputs: Channel+
    view main:   Oscillator        // exactly one
    view fx:     Reverb?           // 0 or 1
    derived total: f32 = sum(for c in inputs: c.gain)
  ```

### 4.3 No implicit multiplicity
- **Decision:** Bare selector = **exactly one**. Multiplicity is always explicit: `?` (0..1),
  `+` (1+), `*` (0+), `[=N]`, `[N..M]`. (**Reverses 017-24**, where bare meant `0..` unlimited;
  applies everywhere cardinality appears.)
- **Why:** No silent multiplicity; the access shape is self-documenting from the declaration.
- **Example:**
  ```
  view post:  Post     // exactly one
  view posts: Post+    // 1 or more
  view posts: Post*    // 0 or more
  view post:  Post?    // 0 or 1
  ```

### 4.4 View = homogeneous borrow-window (the "V1" choice)
- **Decision:** A view is a transient borrow-window. Reading through it is direct and zero-ceremony
  (`channels[0].gain`) — no `!`, no Handle resolution. A view is **not storable**; to persist a
  reference you take a `Handle` explicitly (`weak`). **No auto-resolve** mechanism is added.
- **Why:** This is the existing semantics (017-48: part elements are references used in place,
  "storable nowhere"). Choosing it means the common case stays free and we add no implicit-`!`
  semantics. (The alternative — a view *is* an array of `Handle`s, storable but needing
  resolution/auto-resolve — was considered and rejected.)
- **Example:**
  ```
  derived total: f32 = channels[0].gain + channels[1].gain   // direct
  attr favorite: Handle[Channel] = weak channels[0]          // explicit, storable
  ```

### 4.5 Group ≠ View
- **Decision:** A **group** is a *heterogeneous* bundle of co-placed children, written `[...]` at
  placement. It is a distinct kind from a view (which is homogeneous). A group is reached **only**
  via `.<Type>`, which yields a (homogeneous) view; there is no bare positional index into a group.
- **Why:** A view holds one predicate; a group can mix types. Forcing `.<Type>` first means you
  never index a heterogeneous thing, so the "what type does `group[i]` return / union?" problem
  never arises (Ductus has no ad-hoc unions, 030-68).
- **Example:**
  ```
  view bar: [Note+ Rest*]     // a single group: 1+ Note, 0+ Rest
  bar.Note[0].pitch           // .Note → view of Notes in the group, then index
  bar.Rest                    // .Rest → view of Rests in the group
  ```

### 4.6 Bare placement is not a group; group access
- **Decision:** Only an explicit `[...]` is a group; a bare placement is not. Flat views flatten
  through groups (count/see elements); group views see brackets. Single group → `bar.Note[i]`;
  multiple groups → index the group first, `bars[g].Note[i]`.
- **Example:**
  ```
  verse: C4 [F4 G4] E4         // C4, E4 bare; [F4 G4] one group
  view notes:  Note+           // flat → sees C4, F4, G4, E4   (4)
  view chords: [Note+]+        // grouped → sees [F4 G4]        (1)
  chords[0].Note[1]            // 2nd Note of group 0
  ```

### 4.7 Cardinality = conjunction
- **Decision:** Each view's cardinality is a constraint on `count(children matching its predicate)`
  over the whole supply. All views' constraints hold **simultaneously** (conjunction) — there is no
  precedence/resolution. Overlap is fine (a child counted in every matching view). The only
  "conflict" is unsatisfiability, diagnosed by a **cheap subset-edge static check** (along the known
  trait/marker subset lattice) plus **placement-time counting** — no general feasibility solver.
  For groups: **inner cardinality (members-per-group) is part of the match predicate (a filter);
  outer cardinality (group count) is the count constraint.**
- **Why:** Lets you require "5 Drivables, any concrete type" (`view ds: Drivable[=5]`) while keeping
  conflicts (a subset needing more than a superset allows) detectable. Inner-as-filter avoids false
  conflicts and lets distinct group shapes (`[Note[=2]]+` vs `[Note[=3]]+`) coexist.
- **Example:**
  ```
  view drivables: Drivable[=5]   // exactly 5 Drivables (cars, bikes, trains — don't care)
  view cars:      Car[=2]        // 2 of those 5 are Cars (Car ⊆ Drivable) — both hold
  // view all: Node[=3] + view drivables: Drivable[=5]  →  static error (Drivable ⊆ Node, 5 > 3)

  view duos:  [Note[=2]]+        // selects 2-note groups
  view trios: [Note[=3]]+        // selects 3-note groups — disjoint, no conflict
  ```

### 4.8 Accepted universe
- **Decision:** What a node accepts = the **union of its view predicates**. A `Node`/`Connection`
  catch-all view opens it fully. **No views = accept nothing.** (**Reverses 017-16**, where absence
  of `parts:` meant accept-everything.)
- **Why:** Fully explicit — you get exactly the children you declare a view for; "no magic." Pairs
  with the removed default exposition (both are explicit-only).
- **Example:**
  ```
  node A:  view xs: Foo+        // accepts only Foo
  node B:  view all: Node*      // accepts any node (catch-all)
  node C:  // no views          // accepts nothing
  ```

### 4.9 Exposition references views; own children
- **Decision:** `expose:` entries reference **view names** and **placement `as`-names**, in source
  order. `expose:` is also where a node places its **own** internal children, optionally named; those
  names are usable in the body.
- **Example:**
  ```
  node Mixer:
    view inputs: Channel+
    expose:
      inputs                      // expose caller-supplied channels
      Helper as helper            // node's own child, placed + named here
  ```

### 4.10 Reference-variable hoisting
- **Decision:** View names and exposed `as`-names are **hoisted** — visible body-wide regardless of
  textual order. (They name structure, not init-time values; cell *values* still follow init order,
  016-144.) Lock as an explicit rule (clarify in spec if currently silent).
- **Example:**
  ```
  node Mixer:
    derived x: f32 = helper.value     // refers to `helper` declared below — fine, hoisted
    expose:
      Helper as helper
  ```

### 4.11 `dynamic` as a view modifier
- **Decision:** `dynamic` prefixes a view to mark a reactive, runtime-varying collection:
  `dynamic view voices: Voice*`. Consumed by operators / `repeat`, not indexed. Drops the verbose
  `Cell[…]`/`[]` spellings — `dynamic` carries the reactivity; multiplicity is still explicit (§4.3).
- **Example:**
  ```
  dynamic view voices: Voice*
  derived names = voices |> map(fn(v): v.name)    // operators, no [i]
  ```
- **Spec impact (whole of §4):** LOG — rewrite §13.3.3 (parts), §13.4 (parts access), §13.3.7.2
  (default exposition removed); revise 017-16, 017-24; reserved-word changes (free `parts`, 002-5 /
  020-8); add the `view` declaration, group, cardinality-conjunction, accepted-universe, and
  hoisting decisions. SPEC — corresponding sections. Depends on §3.

---

## 5. `OPEN` — not yet resolved (resume here)

- **Dynamic single view** — what `dynamic view x: Post` (single + dynamic) means, or whether only
  collections may be dynamic.
- **Own-emitted children aggregation** — do views see the node's own `expose:`-emitted children
  (e.g. a `for`-emitted bank of 16 oscillators), or only caller-supplied? Old spec kept them apart
  (017-21, 017-58); the views model reopens it.
- **Gated children in views** — gates freeze, don't remove (022-7/9), so views *count* gated-but-
  frozen children and reads return frozen values (unlike `dynamic`/`repeat`, which change the
  count). Believed correct; needs an explicit yes.
- **Body clause order** — respec needed (parts:/incoming:/outgoing: changed; views are cell-like).
- **Survival confirms** — `Type[…]` template slots (016-218+), the `exposition` field (017-183),
  the `effects:` clause, and generic-parameter views (`view items: T`) are believed unaffected;
  confirm rather than assume.
- **`@reset_on_reopen` IR representation** — none exists today (cf. 033-104 for `@reset_on_reload`);
  closing it needs new §033 decision(s) parallel to 033-104. Whether to do it now or track
  separately is OPEN.
- **"first-class" terminology** — the word is overloaded in the spec; the §3 framing is locked, the
  label/disambiguation is not.
- **Connections** (§1.7) and **Cluster B / portals** (§1.6) — full chunks deferred.

---

## 6. Provenance — the 13 proposals → outcome

| # | Proposal | Outcome |
|---|---|---|
| 1 | metaprogramming / tuple iteration | DISCARDED (enums cover) |
| 2 | explicit lifetimes | DISCARDED |
| 3 | future-forms list | PARKED |
| 4 | Sync/Async connection markers | DISCARDED |
| 5 | `as`-name on compile-time `for` | absorbed into views/exposition (`as`-names, hoisting) |
| 6 | trait-bound `for`/parts | absorbed into views (trait selectors) — §4 |
| 7 | per-declaration named parts | became the views model — §4 |
| 8 | placing groups (chords) | became the Group concept — §4.5/4.6 |
| 9 | slots | subsumed by named views — §4 |
| 10 | `instant` type | NO-ACTION (already specified) |
| 11 | `@reset_on_reopen` unification | LOCKED — §2 |
| 12 | `Cell` access/return | DEFERRED (Cluster B) — §1.6 |
| 13 | portals | DEFERRED (Cluster B) — §1.6 |

Plus the cross-cutting **instance citizenship reframe** (§3), which arose while discussing #8 and
underpins §4.
