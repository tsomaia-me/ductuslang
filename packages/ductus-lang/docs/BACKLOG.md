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

### 1.6 `DEFERRED` — Cluster B: `Cell` return/storage (#12) + portals (#13)
Deferred to its own discussion. Full framing and numbered open questions (Q8b.1–5) live in
**§5 (§8b)** — not duplicated here.

### 1.7 `LOCKED` — Connections under the views model
Model decided this session (individualistic per-declaration `outgoing name: Type` / `incoming name:
Type`, bare = 1, `dynamic ⟹ *`, no groups). Full locked model and the one open follow-up (Q8a.5)
live in **§5 (§8a)** — not duplicated here.

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
  5. **IR representation (closes the former open gap, Q6):** add a `reset_on_reopen` boolean to the
     IR, the way `@reset_on_reload` already has one (033-104) — because `@reset_on_reopen` is
     *runtime-behavioral* (the runtime acts on the reopen edge), not frontend-resolved. **Location
     nuance:** 033-104 is a *stream-entry* field, but `@reset_on_reopen` attaches to **recurrents**
     (016-54) and **stream consumers** (016-57), so its IR slot sits on the **recurrent cell entry**
     (033-82 tuple) and the **consumer/cursor** encoding — *not* the stream entry. **General principle
     to state:** runtime-behavioral annotations get IR fields; compile-time annotations (`@derive`,
     `@flag`, `@literal_suffix`, trait `@default`) are frontend-resolved and never appear in the IR.
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
  §13.9.7, §13.15.5. Reload-safe: 028-10/11 unaffected. IR — new §033 field(s) on the recurrent cell
  entry + consumer/cursor encoding, parallel to 033-104 (Decision 5).

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
- **Resolved (Q7):** define "first-class citizen" **precisely in the new §1.3 decision itself**
  (nameable / passable / returnable; orthogonal to value-semantics). The spec's existing loose uses
  (016-163 "first-class type", 030-2 "first-class reactive cells", 019-2 "first-class entities",
  030-254 "not first-class values") are **left as-is** — **no spec-wide rename**; fix a specific loose
  use only if the blast-radius shows it *actively* contradicts the precise meaning (none expected).
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
- **Resolved (Q1) — a dynamic view is always a list, written `dynamic view name: T*`:** the `*` is
  **required** (the explicit spelling of its inherent 0+ under no-implicit-multiplicity); the bounded
  specifiers `?`/`+`/`[=N]`/`[N..M]` are **forbidden** on `dynamic`. **There is no single dynamic
  view.** This **amends 017-36** (today: `dynamic` excludes *every* cardinality specifier) to:
  `dynamic` takes *exactly* `*`.
- **Example:**
  ```
  dynamic view voices: Voice*                     // ✓ the only dynamic form
  // dynamic view voice:  Voice    ✗  no single dynamic view
  // dynamic view voices: Voice+   ✗  bounded specifier forbidden on dynamic
  derived names = voices |> map(fn(v): v.name)    // operators, no [i]
  ```

### 4.12 Views capture only caller-supplied children (Q2)
- **Decision:** `view`/`dynamic view` capture **only caller-supplied children**. A node's **own
  (internal) children** are reached by their own mechanisms, never by a view:
  - single internal child → `as <name>` (in `expose:`)
  - multiple static internal children → `for … as <name>` (proposal #5 — its **own** mechanism)
  - dynamic internal children → `repeat … as <name>` (018-60 keyed view)
- **Why:** Preserves the old supply/internal separation (017-21, 017-58) under the new model: views
  are the *caller-facing* surface, the `as`-family is the *internal* surface.
- **Provenance fix:** proposal #5 (`for … as`) is its **own** internal-child mechanism, a sibling of
  `repeat … as` — **not** "absorbed into views." (§6 row #5 corrected.)

### 4.13 Views count gated-but-frozen children (Q3)
- **Decision:** Views **count** gated-but-frozen children, and reads return their **frozen values** —
  because gates *freeze*, they don't remove (022-7/9). Distinct from `dynamic`/`repeat`, which change
  membership.
- **Example:**
  ```
  view chans: Channel+      // a `when`-gated-off Channel still counts here; its reads are frozen
  ```

### 4.14 Resolved follow-ups (Q4, Q5) — tasks/confirms, not open
- **Body clause order (Q4) — RESOLVED:** **free order** among all members (cells, `view`/`dynamic
  view`, `incoming`/`outgoing`, `satisfies`, `when:`), **except** `effects:` comes after the members
  and **`expose:` is always last**. Cell init-order-significance (016-20/144/145) unchanged within the
  free block. Singletons (≤ once): `satisfies`, `when:`, `effects:`, `expose:`, `default attr`.
  Enforce the two bottom anchors (`effects:` then `expose:`); free above — light, strict-where-it-
  gives-shape (confirm enforce-level during spec). Replaces the fixed order 017-162 (which named the
  now-removed clauses).
- **Survival confirms (Q5) — blast-radius, not a design open:** the blast-radius (not memory) confirms
  `Type[…]` template slots (016-218+), the `exposition` field (017-183), the `effects:` clause, and
  generic-parameter views (`view items: T`) survive **untouched**. Confirm during spec work.

### 4.15 Exposition entry catalog (Round 2 — `expose:` respec)
- **Decision:** the valid `expose:` entries under the views model are:
  - **view name** (snake_case) → caller-supplied children of that view, in placement order (replaces
    `parts.X`, 017-164).
  - **connection-view name** (snake_case) → caller-supplied connections of that view, engaged here in
    placement order (replaces the `outgoing.<C>` positioning entry, 017-168).
  - **own placement** (PascalCase): `Helper as h`, `for … as`, `repeat … as` (names hoisted).
  - **self-sourced connection**: `ConnType: dest` (017-167, unchanged).
  - **gate blocks** `when …:` / `given …:`, inline `when` (017-176/177, unchanged).
  - **`@content`** — standalone directive (see §4.16).
  - **error:** a bare incoming-view name as an entry → points at `name[i].from` / `repeat c in name:`
    (restates 017-170/171).
- Carry-overs unchanged: omitted view = no output, not an error (017-191); `.exposition` field
  (017-183); traversal in entry order (017-192).

### 4.16 `@content` — whole-supplied-body exposition directive (Round 2)
- **Decision:** `@content` is a **standalone directive** used as an `expose:` entry (and inside a
  wrapper body). It exposes **everything the node accepts — children + outgoing connections — in the
  caller's original, interleaved order**. Exact port of the old `expose: parts` (017-180), now
  explicit (no default exposition, §4.1).
- **Not a named declaration** (the name would only ever be used in `expose:`). Exposition-only — no
  expression access; typed access goes through views (a heterogeneous bag isn't bare-indexable, §4.5).
- **Acceptance unchanged:** a node accepts ONLY what its `view`/`outgoing` declarations say (§4.8). A
  generic container declares explicit catch-all views (`view rest: Node*`, `outgoing wires:
  Connection*`). **No "content-alone accepts anything" special case** — forced explicit (user decision).
- Covers **children + outgoing connections only**; never incoming (incoming isn't body-supplied).
- **Wrappable:** `Padding as pad:` then `@content` inside tucks the whole body into pad. At most one
  `@content` per `expose:` scope.
- **Spelling `@content`** (over `*` — can't be wrapped — and `$content`); `@` = directive (§7).

- **Spec impact (whole of §4 + the `expose:` respec):** LOG — rewrite §13.3.3 (parts), §13.4 (parts
  access), §13.3.7 incl. §13.3.7.2 (default exposition removed) and the entry catalog (017-160..192);
  revise 017-16, 017-24, 017-162 (clause order), 017-164/166/168/180/181/182; reserved-word changes
  (free `parts`, 002-5 / 020-8); add the `view` declaration, group, cardinality-conjunction,
  accepted-universe, hoisting (§4.10), clause-order (§4.14), exposition-entry (§4.15), and `@content`
  (§4.16) decisions. SPEC — corresponding sections (§13.3.3 / §13.4 / §13.3.7). Depends on §3.

---

## 5. #8 — Connections under the views model (`LOCKED`) + Cluster B portals/`Cell` (`OPEN`)

### 5a (§8a) `LOCKED` — Connections: individualistic per-declaration model
Connections get the **individualistic, per-declaration** treatment — parallel to views, but with their
own keywords and direction semantics. (Was §1.7, deferred; decided this session.)

- **Form:** `outgoing name: Type <card>` (node is `from`) and `incoming name: Type <card>` (node is
  `to`). Direction-keyword, name, type, opt-in cardinality. Replaces the `incoming:`/`outgoing:`
  comma-list clauses (017-81/84). (`view` stays node-only; connections use their own keywords — the
  earlier `view x: incoming SomeConn` candidate is rejected.)
- **bare = 1, no implicit multiplicity — ALL directions** (incoming too). Reverses 017-84 (bare =
  `0..`). Fan-in / multi-origin writes `*` (or `+`/`[N..M]`). Coherence over the one-char convenience.
- **`dynamic` ⟹ `*`:** `dynamic outgoing`/`dynamic incoming` = `*` + runtime membership; inherits §4.11
  (Q1); replaces the 017-36 exclusion.
- **Cardinality semantics differ by direction (preserved):** `outgoing` = **local-supply** bound
  (017-83); `incoming` = **aggregate** reception bound across all sources (caller-placed +
  self-sourced, 017-88/117). bare-1 incoming = "exactly one, from any source, may target me,"
  aggregate-checked.
- **No groups for connections** (Option A; the `group` keyword was rejected — grouping stays
  node-only). Homogeneous "one of {A,B}" models as a single pairs/cartesian connection type
  (019-33/38), not a group.
- **Named access** replaces `outgoing.<C>`/`incoming.<C>` (017-97): body access by view name (`name`,
  `name[i]` under guaranteed min, `for c in name:`); exposition positioning by name (replaces 017-168);
  back-connection `name[i].from`, `repeat c in name:`. Named slots permit two slots of the **same**
  connection type — impossible under type-keyed access.
- **Self-sourced connections unaffected** (017-121) — internal, placed in `expose:` with `ConnType:
  dest` syntax, exempt from the caller-facing signature.
- **Reserved-field cleanup:** `parts`/`incoming`/`outgoing` stop being bulk fields (020-8/020-20);
  `incoming`/`outgoing` survive as direction keywords; `parts` retires for `view`.
  `exposition`/`from`/`to`/`pair`/`subject` stay.

**Resolves:** Q8a.1 (replace clauses — yes), Q8a.2 (direction via keyword — yes), Q8a.3 (inherit the
individualistic form; **diverge** on groups [none] + incoming-cardinality *meaning* [aggregate]),
Q8a.4 (no "incoming supplied" — `outgoing` supplies, `incoming` receives).

**Confirm during spec work (expected outcomes, not open decisions):**
- Connection-view *access* yields transient connection-reference borrows under the same borrow-window
  V1 as node-view access (017-48 analog).
- Bare-vs-array body access mirrors views: bare (=1) = single connection ref; `*`/`+`/`[N..M]` = array.
- Blast-radius bound: connection-*type* bodies are **untouched** — `from`/`to`/`pairs`, connection
  `when:`/attrs/recurrents, and connection traits declaring endpoints (005-54/58, §13.6.1) stay as-is.
  Only the node-side `incoming:`/`outgoing:` clauses change.

**Q8a.5 (engagement positioning of caller-supplied connections) — RESOLVED (Round 2, §4.15/4.16):**
- **The type positions caller-supplied connections; anchoring is dropped.** A caller-supplied
  (outgoing) connection engages only where the type names its **connection-view** in `expose:`, in
  placement order within that view. A connection-view not named in `expose:` is live (presence =
  participation, 017-195/202) but never engaged — same as an unexposed view (017-191).
- **Self-sourced connections unaffected** — placed inline in `expose:` (`ConnType: dest`), engage in
  place (017-106/107/167). The interleaving use case (notes + `Plays`) is served by self-sourced
  connections, so dropping anchoring loses nothing real.
- Grounded: 017-170 ("engagement order belongs to the source's traversal" = the type) + explicit-only
  philosophy (§4.1/§4.8). **Retires** anchoring (021-54/56/59), the `outgoing.<C>` positioning entry
  (017-168), and the bare-`incoming` entry rule (017-170/171 → bare incoming-view-name error).

- **Spec impact (§8a):** LOG — rewrite §13.3.4 (`incoming:`/`outgoing:` clauses → per-declaration
  connection-views), §13.3.4.1 (access → named), parts of §13.3.7 (exposition positioning); revise
  017-81/84/97/168, 020-8/020-20; the 017-36 exclusion folds into the `dynamic ⟹ *` rule. SPEC —
  §13.3.4 / §13.6 cross-refs. Depends on §3 + §4.

### 5b (§8b) `DEFERRED` — Cluster B: portals (#13) + `Cell` return/storage (#12)
Own discussion (was §1.6). Proposal-form open questions; **no decisions yet.**

**#13 Portals — proposal:** a **storable, `Copy`, always-`Option`-on-access** reference to **owned,
non-graph** data (records, arbitrary values), to enable self-referential structures (e.g.
doubly-linked lists). Distinct from `Handle` (graph entities only, 017-145/146); mechanically
near-identical (Copy, Option-on-resolve, address-like).
- **Q8b.1** — Is a portal just **`Handle` generalized** to non-graph data, or a **stdlib slotmap** on
  the `raw_*` intrinsics (033-193) — rather than a new language primitive?
- **Q8b.2** — Liveness for arbitrary owned data needs **generation stamps** (017-157 analog): who pays
  the cost, and how does the compiler decide which allocations to instrument (only those a portal is
  taken to)?
- **Q8b.3** — Are portals ever **writable**? Read-only portals can't splice a doubly-linked list
  (can't set `prev`/`next` after construction) — gutting the headline use case. If writable, how does
  it reconcile with localized mutability (writing through a portal mutates non-local state)?
- **Q8b.4** — A portal into a **record field dangles when the record moves** (category-B storage,
  013-3). Caught at runtime (generation stamp → `None`) or forbidden at compile time?

**#12 `Cell` return/storage — open:** reading a top-level `Cell` inside a function is allowed and
provenance-tracked (025-15/16), but the **ownership category of a `Cell[T]` value is unpinned.**
- **Q8b.5** — Is `Cell[T]` a **call-scoped borrow** (unstorable) or a **freely-copyable storable
  reference** (like `Handle`)? That decides what *returning* a `Cell[T]` from a function means.
  (Connects to portals — both are "references to non-instance data.")
- **Spec impact:** TBD — own discussion.

---

## 6. Provenance — the 13 proposals → outcome

| # | Proposal | Outcome |
|---|---|---|
| 1 | metaprogramming / tuple iteration | DISCARDED (enums cover) |
| 2 | explicit lifetimes | DISCARDED |
| 3 | future-forms list | PARKED |
| 4 | Sync/Async connection markers | DISCARDED |
| 5 | `as`-name on compile-time `for` | LOCKED — its **own** internal-child mechanism (`for … as`), sibling of `repeat … as`; not absorbed into views (§4.12) |
| 6 | trait-bound `for`/parts | absorbed into views (trait selectors) — §4 |
| 7 | per-declaration named parts | became the views model — §4 |
| 8 | placing groups (chords) | became the Group concept — §4.5/4.6 |
| 9 | slots | subsumed by named views — §4 |
| 10 | `instant` type | NO-ACTION (already specified) |
| 11 | `@reset_on_reopen` unification | LOCKED — §2 |
| 12 | `Cell` access/return | DEFERRED (Cluster B) — §5 (§8b) |
| 13 | portals | DEFERRED (Cluster B) — §5 (§8b) |

Plus the cross-cutting **instance citizenship reframe** (§3), which arose while discussing #8 and
underpins §4.

---

## 7. `LOCKED` — `@` directives generalization

- **Background:** `@content` (the whole-body exposition directive, §4.16) raised "what does `@`
  mean?" Today `@` is the annotation prefix (decorating declarations) plus a placement flag character
  (021-119/126). `@content` stands alone, which annotations don't — so the model needs generalizing.
- **Decision — `@<name>` introduces a directive.** A directive is a **language-provided, closed** set
  of compiler instructions. Each directive has a fixed **role**:
  - **applied directive** (everyday word: **annotation**) — attaches to an entity. Current set:
    `@derive`, `@flag`, `@literal_suffix`, `@reset_on_reopen`, `@reset_on_reload`, trait `@default`.
  - **standalone directive** — stands alone as an element. Current set: `@content`.
- **Terminology reframe:** "annotation" → "annotation directive" (the applied role); "annotation"
  stays as everyday shorthand (keep repeated uses simple in a paragraph).
- **Why:** gives `@content` a principled home instead of a lonely one-off; one rule for `@`; the
  family already varies (args / no-args), so naming it is honest. The applied/standalone role split
  keeps the decorator-vs-structural distinction **explicit**, not blurred.
- **Closed set, stated explicitly** so naming the concept doesn't imply extensibility. **User-defined
  directives are OUT OF SCOPE / design-intent only — never spec text** (would be metaprogramming,
  DISCARDED #1; reflection PARKED §1.4; no postponed decision, 001-5).
- **Word is "directive," not "attribute"** (attribute connotes user-extensible metaprogramming).
- **Flag-char carve-out:** `@` is also a placement flag character (021-119); the directive rule
  excludes the flag-after-TypeRef position. Reword 021-126 "annotation prefix" → "directive prefix".
- Role split is **descriptive** (applied vs standalone), not an elaborate engine — ~7 fixed items.
- **Spec impact:** LOG — new §002/§1.4 decision "`@<name>` introduces a directive" + the
  applied/standalone role taxonomy; reword 021-126; light cross-refs on the annotation decisions
  (006 / §3.8 `@derive`, 021-116 `@flag`, 005-206+ `@literal_suffix`, 016-54 `@reset_on_reopen`,
  030-236 `@reset_on_reload`, 005-32/35 `@default`). SPEC — §1.4 (lexical) + annotation sections
  reframed as "annotation directives."
