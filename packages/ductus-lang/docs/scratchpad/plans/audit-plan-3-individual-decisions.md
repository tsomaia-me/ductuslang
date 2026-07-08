# Audit Remediation — Part 3 of 3: Individual decisions

*2026-07-07 · one decision per item. Read AFTER Part 2 — some recommendations assume Part 2's master decisions go as recommended, and say so explicitly where they do.*

Items are ordered by severity. Obvious calls are one-liners; only real tradeoffs get options. The final section holds dismiss-or-pin items: findings where the audit's own verifiers disagreed — each states both arguments and recommends which side holds.

Items I-78 to I-80 were originally drafted as mechanical fixes (Part 1) and moved here because each hides a real choice.

## Decisions

#### I-01 — Which operand of `|>` carries the effect-kind method at interpretation bootstrap

Background: when a node hands itself off to be interpreted, it pipes a node reference into a method that walks that node. The SPEC's prose for this "bootstrap" shape and its own example disagree on which side of `|>` holds the interpreting method.

The question: in `render(song) |> audio_out`, is `render` (left side) the effect-kind method, or is `audio_out` (right side)?

Option A — method on the left, called conventionally:
```
audio = render(song).audio |> audio_out
// render(song) runs the interpreting method, produces an effect;
// audio_out is a plain downstream effect the result pipes into
```

Option B — method on the right (what the current bootstrap prose says):
```
audio = song |> audio_out
// song is a bare node reference; audio_out is the effect-kind
// method whose Subject-typed first parameter accepts the node
```

Recommended: Option A, because the SPEC's own definitions force it. The SPEC declares `render` as an effect-kind method (`effect render(value: Subject):`) and separately says `|>` "may apply an operator or an effect" on its right side. So in the example `render(song)` is the method call that produces an effect, and `audio_out` is a plain effect on the right — exactly what the decision-of-record shows. The competing bootstrap prose contradicts both the SPEC's own definition of `render` and the record. Fix the prose, not the example.

For execution: LOG 017-273 (§13.3.8). SPEC §13.17.7 Case-3 prose (lines ~19931-19953, esp. the RHS-is-the-method claim and the 19944 comment) contradicts SPEC §13.19.7 / §13.3.8 example `render(song).audio |> audio_out` and the `effect render(value: Subject):` declaration (~SPEC:1231) plus the `|>`-applies-operator-or-effect rule (~SPEC:19979). Reconcile Case-3 prose to Option A. Finding F069.

#### I-02 — How many dynamic-source exceptions does provenance have — one or two

One SPEC section says the sole exception is a read through a weak handle; the record and another SPEC section say there are two (weak handle and portal). Recommended: two, because the record and the section that defines dynamic dependency both list weak-handle and portal reads as the two sources, and dropping portal would silently under-track portal re-binds. Fix the one-exception section to list both.

#### I-03 — The "wake gate" construct is named five times in the record but never appears in the SPEC

The record defines it as a real lowering construct — a compiler-synthesized gate on every wire-following target render, with a fixed predicate and a prior-commit read rule — yet the term appears zero times in the SPEC, including in the sections the record points to. Recommended: elaborate the wake gate in the referenced SPEC sections under the same term, because the SPEC must conform to the record and a named load-bearing construct absent from its own cited elaboration is a divergence; keep "wake gate" as the fixed term unless the user wants to rename it.

#### I-04 — Two tiebreaker rules both match a cross-instance same-level pair, and they can disagree

When two cells are at the same level with no dependency between them, the compiler needs one deterministic order.

The question: which rule wins when both apply? The record has two:
```
// Rule A (unconditional over "two same-level DAG nodes"):
//   tiebreaker = source declaration order
// Rule B ("same-level cells across different node instances"):
//   tiebreaker = placement order at construction time
```
A same-level pair living in two different node instances matches both. Source order and placement order can differ — a node type declared earlier in source may be placed later at construction. So the two rules can pick different evaluation traces for the same program.

Options:
- Scope Rule A to same-instance pairs, and let Rule B govern cross-instance pairs. Exactly one rule then applies to any pair.
- Give one rule explicit precedence over the other (e.g. cross-instance placement order overrides).

Recommended: scope Rule A to same-instance and let Rule B own cross-instance, because that is the only reading where each pair matches exactly one rule and the SPEC's own stated goal ("same program, same inputs, same output trace") is preserved. But this is a behavior-observable choice — the user should confirm the split rather than have it inferred.

For execution: LOG 023-33, 023-34 (§13.10.3), SPEC §13.10.3 (~18201-18209). Add explicit scoping: 023-33 intra-instance, 023-34 cross-instance; state in SPEC prose that exactly one applies. Finding F168. User decides.

#### I-05 — A connection derived that reads `to` has no defined value when the connection is frozen from construction

Background: a connection can opt in to surfacing endpoint data by declaring its own derived over an endpoint, e.g. `derived target: WeakHandle[Clip] = handle to`. Separately, a weak-handle destination that resolves to `None` freezes the connection and its body never runs. The cell-value invariant that guarantees every connection derived has a defined value is justified only for deriveds computed against `from` ("`from` always has defined cells").

The question: what does `target` read when `to` was never bound — for instance when `to` resolves through a repeat-view lookup that is empty at construction, so the body never runs and `target` never commits? The invariant covers `from`-based deriveds; the fallback "initial value if the instance has never been active" is undefined here because the body never computed one. `handle to` is uncomputable when `to` is unbound.

Options:
```
// A: give it a defined frozen initial value
derived target: WeakHandle[Clip] = handle to  // reads None/unresolved handle while to unbound

// B: forbid observing a to-reading connection derived before first activation
// (compile/runtime rule: no external read until the connection has been active once)
```

Recommended: Option A — extend the cell-value invariant to cover `to`/`pair`-reading connection deriveds and pin their frozen-from-construction value to an unresolved/`None` weak handle, because the endpoint-derived carve-out is explicitly legal and empty repeat sources are legal, so the language already admits this state and must define what it reads rather than leaving a legal-but-unspecified hole. Option B is defensible if the user prefers to keep the invariant narrow, but it adds a new observation restriction.

For execution: LOG 019-59, 019-60, 019-53 (§13.6.2). SPEC §13.9.7 cell-value invariant (~17781-17787), §13.6.2 endpoint-derived carve-out (~16047-16055). Extend invariant to `to`-dependent connection deriveds with a defined frozen-from-construction value, OR add an observe-before-activation prohibition. Finding F116.

#### I-06 — Scope drop (retire) is never placed inside the six-step commit cycle

Background: a commit runs a fixed six-step sequence (topo-sort, evaluate deriveds, advance recurrents, publish snapshot, clear dirty bits). Separately, when a repeat source loses a key, that scope is dropped and its streams/effects tear down. The record says repeat re-iterates "at commit time" but never says at which of the six steps the drop happens.

The question: where in the commit does scope drop land, relative to derived evaluation, recurrent advancement, and snapshot publish — and how does it order against the gate-close `suspend` hook for an effect that is both retiring and gate-closing? Two consequences hang on this:
```
// 1. Does a dropped scope's last scope_evaluate land in THIS commit's
//    published snapshot (step 5) or the next one?
// 2. For an effect that is gate-closing AND retiring in the same commit:
//    does scope_drop's teardown run before or after the suspend hook?
```
Without an answer, two conformant implementations can publish different snapshots.

Options:
- Anchor the repeat iterate/diff/scope_obtain/scope_drop/scope_evaluate sequence to specific numbered steps of the commit cycle, and state its order against the gate-close suspend hook and stream teardown.
- Leave it to a named owning section (repeat's section) that cross-references the commit steps.

Recommended: anchor the sequence to numbered commit steps and designate one owning section, because the ambiguity is behavior-observable (differing published snapshots) and only an explicit step assignment closes it. This should be reconciled with the master decision on yielded/fold IR scheduling (K-YIELDFOLD-IR) and the reconciler hook order (K-HOOK-ORDER) so the suspend-vs-teardown seam agrees across all three.

For execution: LOG 023-13/023-14/023-29/023-30/023-31 (§13.10.2 six steps), 018-94 (§13.5.4.6 commit-time recompute), 018-78, 022-63 (gate-close suspend at commit boundary). No SPEC text places scope_* at a numbered step. Anchor repeat's scope diff to specific §13.10.2 steps; designate owner (§018 or §023). Assuming K-YIELDFOLD-IR and K-HOOK-ORDER go as recommended. Finding F112.

#### I-07 — Is a gated-off static feeder of a dynamic view a frozen-present member or absent

Background: a static placement can feed a `dynamic` view, and placement-level `when` can gate it off. Gating freezes propagation but never removes existence — for static views, a gated-off child stays a frozen-present member. But a dynamic view is precisely the kind whose membership changes. The SPEC enumerates exactly four gate read-paths; two are membership paths — frozen static-view membership and activation-driven fold membership — and neither covers a gated-off static feeder of a dynamic view.

The question: does the gated-off static feeder stay a member of the dynamic view, or drop out?
```
// A: existence-based (like static views)
posts.count      // counts the gated-off feeder
repeat p in posts  // mounts a scope for it

// B: activation-based (like folds)
posts.count      // does NOT count it
repeat p in posts  // no scope for it
```
Both readings survive the current text and produce different concrete behavior.

Recommended: Option A (frozen-present), because the record's general rule is that gates "freeze propagation rather than remove existence," explicitly contrasting gating with dynamic/repeat membership changes — the feeder is gated (frozen), not removed from the source, so existence should hold and `.count`/`repeat` should see it. But this is a real gap the text does not force; the user should confirm A vs B since it changes `.count` and the repeat scope set.

For execution: LOG 017-80, 017-105, 017-198 (§13.3.3.4, §13.9.7). SPEC four gate read-paths (~17800-17816, paths 3 and 4); add a fifth/clarifying rule for dynamic-view membership of a gated-off static feeder, stating `.count` and `repeat` behavior. Finding F084. User decides A vs B.

#### I-08 — What a self-conditional gate reading a same-instance recurrent resolves to at startup

Background: a gate like `when: counter > 5`, where `counter` is a same-instance recurrent, is self-referential. The rule for such gates is: read the recurrent's prior-commit value (one-commit delay). But at startup there is no prior commit — and the startup rule evaluates `when` in the same topological pass as everything else, letting it read freshly-computed startup values.

The question: at startup, does the gate read `counter`'s just-computed startup value, or a nonexistent prior-commit value?
```
when: counter > 5   // counter = same-instance recurrent
// Reading A: reads counter's fresh startup value (same-pass topo order)
// Reading B: reads prior-commit value — but none exists at startup
```
Reading A contradicts the one-commit-delay rule; Reading B is undefined because no startup fallback for a bare recurrent read in a gate is specified. The two give different initial gate states, so different programs (instance begins active vs inactive). The reopen rule is settled (such a gate cannot reopen once closed) but says nothing about the initial state.

Options:
- Startup uses the same-pass just-computed value; the one-commit-delay rule applies only from the first real commit onward.
- Startup uses a defined fallback (e.g. treat the absent prior commit as the recurrent's initial value, which equals its just-computed startup value here).

Recommended: the just-computed startup value, because the startup rule already establishes initial gate state within the same topological pass and explicitly allows an initial-value expression to read any reactive cell freshly, regardless of kind — carving startup out of the one-commit-delay rule keeps it consistent with how every other startup read works. Note both options converge to the same value here (the recurrent's initial value), so the practical recommendation is stable; the user should confirm the wording that reconciles the delay rule with the startup pass.

For execution: LOG 022-37 (§13.9.6 prior-commit rule), 016-144/016-145/016-146 (§13.2.6 startup topo pass), 016-139 (.previous/.past fallbacks only). SPEC §13.9.6 (~17629-17653, covers reopen not initial state), §13.2.6 (~12303-12307). State that a self-gate's bare recurrent read at startup resolves to the just-computed startup value; carve startup out of the one-commit-delay rule. Finding F065. User decides.

#### I-09 — What `here::` reaches is spelled two opposite ways

Inside a node or connection body, names resolve through three nested scopes, innermost first: your local `let`/`for` bindings, then the instance body scope, then the module top level. The `here::` anchor lets you jump past the innermost layer to reach a specific one. The problem is the docs disagree on which layer it lands on.

One rule says `here::x` resolves `x` "bypassing inner local bindings" — it skips your `let`/`for` vars and anchors the instance body scope. The spec elaboration agrees and names it "scope 2." But another rule, and two spec lines, call `here` "the current (innermost) scope" — and the innermost scope is exactly the local bindings the first rule says to skip. One spec bullet even manages to contradict itself in a single sentence: it calls `here` the innermost scope, then says `here::x` bypasses shadowing from inner blocks — if `here` were already innermost there would be no inner block left to bypass.

This changes results. Take:

```
attr gain = 0.5
derived x: f32 = ( let gain = 2.0; here::gain )
```

Under "bypass local bindings," `here::gain` skips the `let` and reads the attr: `0.5`. Under "innermost scope," `here::gain` reads the `let`: `2.0`. Same code, two answers.

The question is which scope `here::` is meant to anchor:

- **Body scope (skip `let`/`for`).** `here::` means "the member's own scope, ignoring any block-local shadowing." This is what the detailed spec section already commits to.
- **Truly innermost scope.** `here::` means the nearest enclosing scope including `let`/`for`, which makes `here::x` nearly the same as bare `x` and gives it little purpose.

Recommended: anchor `here::` at the body scope (skip local bindings), and fix the "innermost" wording as the drifted side. The detailed spec section already commits to the body-scope reading, and "innermost" makes `here::` almost redundant with a bare name. But this is a real design choice with different observable behavior, so confirm the intent before amending rather than silently picking.

For execution: LOG 020-11 (body-scope reading, keep) vs 020-34 + SPEC §13.7.1 line 16163 + §13.7.7 lines 16383-16385 ("innermost", drifted); ground truth of scope chain is 020-6; SPEC §13.7.2 lines 16225-16229 anchors scope 2. Fix: drop/replace "innermost" wording toward "current body/enclosing scope"; resolve the self-contradictory §13.7.7 bullet. Finding P017.

#### I-10 — A fixed-size array cell whose elements are dynamic-size has no layout rule

Two things are both legal. First, a reactive cell can hold any value type, and `string[10]` (a 10-element array of strings) is a valid array type — so `signal words: string[10]` is a legal cell. Second, a rule says a fixed-extent array cell `T[N]` has compile-time-known layout: every element's offset in the cell is fixed at compile time, and a `for` over it unrolls to reads at those known offsets.

Those collide for the array-of-dynamic case. A `string` is dynamic-size — its bytes live in a pool, and the cell holds an index, not the characters. So `string[10]` is ten dynamic-size things. The "every element offset is known" claim assumes each element has a fixed compile-time width, which a `string` does not. The docs give a layout rule for a whole-cell dynamic type (the cell holds one pool index) and for arrays of fixed-size elements, but nothing states how an array of dynamic-size elements is laid out. An implementer has no rule to lower `for x in words.value():` over a `string[10]` cell.

The question is what such a cell lays out as:

- **Each element is its own fixed-size pool index.** The cell stores ten word-size indices; offsets over that index array are compile-time-known, and each read resolves its index into the pool. This keeps the "known offsets" claim true, just over indices instead of values.

```
signal words: string[10]   // cell = [idx0, idx1, ... idx9], each a word; string bytes in pool
```

- **Disallow dynamic-size element types in fixed-extent array cells.** `string[N]` / `Vec[T][N]` are simply rejected as cell types. Simpler rule, but removes an otherwise-constructible type.

Recommended: store each element as a fixed-size pool index (option 1), and qualify the offset rule to say "compile-time-known offset over the element slots, which for dynamic-size element types are pool indices." This preserves the unroll guarantee and matches how the whole-cell dynamic case already works (a dynamic value becomes a word-size index). But the storage model is a design decision, so surface both options rather than picking unilaterally.

For execution: LOG 025-58/025-59 (unconditional known-offset + unroll claim) vs 025-29 (string/Vec are index-based dynamic-size); 025-22 accepts any value type; SPEC §9.3.1 line 6730 makes string[10] legal; SPEC §14.3.3 lines 23405+ covers whole-cell dynamic only. Fix: either qualify 025-58 + add an array-of-dynamic layout rule (per-element fixed-size pool index) or disallow dynamic-size element types as fixed-extent array cell elements. Distinct from F221 (bracket spelling only). Finding P018.

#### I-11 — A desired-block output stream is given two conflicting default capacities

An effect's `desired:` block can declare an event-output stream. It is written as a full `stream` declaration with a `= source`, fed internally from the effect's own parameters or observed cells. The rule for it says two things at once: its shape "matches a regular stream declaration," and its omitted capacity "defaults to 1024".

Those two clauses disagree, because a regular stream declaration does not default omitted capacity to a flat 1024. The regular rule is source-derived: if the source is a single stream or a known-capacity stream chain, the default is that source's capacity; if the source is a reactive expression over streams/signals, it is the sum of input capacities. The flat-1024 default is explicitly scoped to "all other cases" — the residual, like a bare `.changes` conversion with no capacity context.

A desired-block stream always has a source, and that source is normally a stream: effect stream parameters carry a capacity (e.g. `GateStream[Message, 256]`), and observed cells are known-capacity streams. So the normal case falls under the source-derived branch, not the residual. Concretely:

```
effect WsClient(outbound_src: GateStream[Message, 256]):
  desired:
    stream ring outbound: Message = outbound_src
```

The source-derived rule gives this capacity `256`. The desired-block rule gives it `1024`. An implementer cannot honor both, and downstream buffer/pool sizing diverges.

The question is which default the desired-block stream takes:

- **Inherit the regular source-derived default.** Drop the standalone "defaulting to 1024"; the omitted capacity follows the source, matching the "matches a regular stream declaration" claim it already makes.
- **Override with a flat 1024.** Keep 1024, but then drop the "matches a regular stream declaration" equivalence for the capacity axis, since it no longer holds.

Recommended: inherit the source-derived default and drop the standalone 1024. The rule already asserts equivalence to a regular stream declaration, and its sources are exactly the known-capacity cases the source-derived branch was written for — a flat 1024 would silently over-allocate a 256-cap source. But this is a genuine either/or the corpus does not settle, so surface the 256/sum-vs-1024 mismatch to the user before amending.

For execution: LOG 031-32 + SPEC §13.19.4 line 22150 (1024 + "matches §13.18.2") vs 030-21/030-22 (source-derived) and 030-25 (1024 scoped to residual); 031-27 (effect stream params known-capacity), 031-31/031-33 (always has a source, fed internally). Both LOG-internal (031-32 vs 030-21/22) and LOG-SPEC. Fix: drop standalone 1024 and inherit source-derived, OR keep 1024 and drop the §13.18.2 equivalence for capacity. Finding P019.

#### I-12 — The three membership-driver tags are spelled two ways in the LOG with no stated equivalence

The three membership-driver tags are spelled two different ways in the LOG — `{permanent, keyed-template, gate-guarded}` in the IR entries versus `{permanent, key-driven, activation-driven}` in the surface entry — and no LOG entry states they are the same three drivers (the spec bridges them, the LOG does not). Recommended: add a self-contained equivalence to the LOG (key-driven = keyed-template, activation-driven = gate-guarded), because the LOG is the decision-of-record and a LOG-only reader currently sees two unlinked vocabularies; the adjudicator should not pick which name is canonical, only that the identity is stated.

For execution: LOG 034-3 (surface: permanent/key-driven/activation-driven) vs 033-81 + 035-10 (IR: permanent/keyed-template/gate-guarded); bridge exists only at SPEC §13.20.3 lines 22997-22999. Fix: add a LOG equivalence statement (or unify to one vocabulary). Finding F053. Interacts with K-FOLD-ORDER (member-order term) but is a distinct driver-tag naming issue.

#### I-13 — The fold-cost rule cites "loop cost rules" that state no O() bound

The fold-cost rule claims kinship with "the loop and collection cost rules," but only the collection rules state an O() bound (Map, Vec); the loop machinery states no normative O() cost — it speaks only qualitatively (no heap alloc, no per-iteration call cost). So the "loop cost rules" half of the cited family has no LOG referent. Recommended: repoint the citation to name only the collection cost rules that actually state O() bounds, because per the citation policy a reference should point at a section that owns the claim, and no loop O() rule exists to sit beside; the fold's own O(log n) bound is self-contained regardless.

For execution: LOG 035-5 (cites "loop and collection cost rules"); collection O() rules exist (012-108 Map, 025-41 Vec); 014-149 is qualitative only, no O(). Fix: drop the "loop" half of the family claim, keep "collection cost rules." Finding F216. Falls under K-CITE-POLICY (repoint-by-default).

#### I-14 — The duplicate-scope-key warning has no named diagnostic class

The duplicate-scope-key rule mandates a runtime "non-fatal warning" but, unlike its sibling diagnostics, assigns it no named diagnostic class — and it fires only when two elements happen to derive the same key at runtime, so no static conformance test can target the warning obligation as written. Recommended: assign it a named diagnostic class parallel to its siblings and state the warning's delivery channel, because every neighboring diagnostic already carries a targetable class name and without one tooling cannot check that exactly this warning fired; the first-wins-and-drop behavior stays testable either way.

For execution: LOG 018-81 (non-fatal warning, no class) vs siblings 018-142 (`unstable_positional_iteration_on_unordered_source`), 018-143 (`bundle_in_repeat_rejected`), 031-141; SPEC §13.5.4.2 lines 15411-15415. Fix: add a normative diagnostic class name + delivery channel to 018-81. Finding F087.

#### I-15 — Two rules use the undefined bracket form `[N..M]`

Two normative rules — the `dynamic`-view forbid list and the connection-view cardinality rule — use the bracket form `[N..M]`, but that spelling is not one of the four defined bracket forms (`[=N]`, `[N..=M]`, `[N..]`, `[..=M]`); `[N..M]` is a half-open, exclusive-upper form the language does not otherwise define. Recommended: replace `[N..M]` in those two rules with the defined inclusive form `[N..=M]`, because the four forms are fixed exactly by the defining entries and no exclusive-upper form exists elsewhere, so `[N..M]` is a typo for the inclusive spelling rather than a new form to introduce.

For execution: defined forms LOG 017-32/017-33/017-34/017-35; offending uses 017-40 (forbid list) and 017-109 (connection cardinality), mirrored at SPEC §13.3.3.1 line 13359. Fix: change `[N..M]` → `[N..=M]` in 017-40, 017-109, and the SPEC mirror. Finding F161.

#### I-16 — The endpoint-derived surface: the LOG closes it to `WeakHandle`, the SPEC adds `Portal`

The LOG names `WeakHandle` as the sole sanctioned endpoint-derived surface and closes the set ("No other surface for endpoint data exists"), but the spec elaboration widens the carrier to `WeakHandle`/`Portal` — the LOG neither mentions nor sanctions `Portal` as an endpoint surface. Recommended: surface to the user whether `Portal` endpoint surfaces are intended; if yes, add `Portal` to the LOG entry, if no, drop it from the spec — because the LOG explicitly closes the set to `WeakHandle` only, so the spec's addition of `Portal` is an unsanctioned widening that only the designer can resolve.

For execution: LOG 019-59 (WeakHandle only, set closed) vs SPEC §13.6.2 lines 16058-16060 (WeakHandle/Portal). Fix: align the carrier set (add Portal to 019-59 or drop from SPEC). Finding F182. Relates to K-HANDLE-FREEZE (Handle/WeakHandle split) but concerns Portal-as-endpoint-surface, which that key does not settle.

#### I-17 — The borrow-storage slot list omits the reactive cell

The log entry that lists where a node/connection/effect borrow may NOT be stored names four slots — record field, tuple component, enum payload, indexed slot — but omits the reactive cell, even though the very SPEC section it cites lists the cell first. Other log entries do forbid storing a borrow in a cell, so this is not a soundness hole; it is a narrow-list defect in one closed enumeration. Recommended: add "reactive cell" to that log entry's slot list so it matches its cited SPEC section, and confirm the citation target (the storable-alias section forwards to the canonical borrow-restriction section — point at whichever actually owns the enumeration). Because a closed atomic enumeration that omits a member its own cited section includes is a divergence a reader taking the log as authoritative would get wrong.

#### I-18 — The IR type-tag vocabulary is stated closed but extended later, and `Handle[T]` is never enumerated

Background: the IR has a fixed vocabulary of type tags — the shapes a serialized type can take. The log states this vocabulary once, then later entries add to it.

The problem: one log entry says the vocabulary IS a specific seven-family list (primitives, str, tuples, arrays, %TypeIds, pool_index, closure) using closed "is" phrasing. Two later entries then say slice types and Portal "join the IR type vocabulary." A closed set cannot have members join it. On top of that, Handle[T]'s representation is referenced as if it were a vocabulary member, but no entry ever enumerates Handle at all. An implementer building a type-tag parser from the closed entry rejects Portal, slice, Handle, Map, and Bundle tags.

Option A — keep one closed enumeration, fold everything in:
```
IR type vocabulary = primitives, str, tuple, array, %TypeId, pool_index,
                     closure, slice, Portal, Handle[T]   // single closed list
```
One authoritative list; every tag has a home. But it fights the current text, which was clearly written to extend incrementally per topic section.

Option B — make the base entry explicitly open, later entries extend it:
```
base vocabulary = primitives, str, tuple, array, %TypeId, pool_index, closure
  ... slice types extend the vocabulary   (borrow types)
  ... Portal[T] extends the vocabulary
  ... Handle[T] extends the vocabulary     // add this — currently missing
```
Matches how the entries already read; only requires softening the "is" copula and adding the missing Handle entry.

Recommended: Option B — reword the base entry from "the vocabulary IS X" to "the vocabulary starts with X and is extended by later entries," and add the missing Handle[T] enumerating entry. Because the corpus already extends the vocabulary additively in two live entries; the only actual defect is the false "closed" copula plus Handle never being enumerated. Fixing the copula and adding Handle is the smaller, evidence-aligned change, and it does not force renumbering or relocating the slice/Portal entries.

For execution: LOG 033-43 (reword closed copula → extensible), 033-47/033-48 (already-extending, keep), add Handle[T] enumerating entry; SPEC §15.4 type_tag grammar (24529-24533) must add slice/Portal/Handle/Map/Bundle tags to conform. Finding F240. Note K-STREAM-IR-TEXT (stream production must serialize) and K-HANDLE-FREEZE both touch this grammar — sequence after them.

#### I-19 — Portal-typed and array-typed cells have no legal cell type tag

Background: a cell entry in the IR carries a `type` tag saying what the cell holds. One log entry pins the legal set of those tags.

The problem: the cell-type-tag entry allows exactly three families — primitives, string-pool-index, dynamic-pool-index. But another entry mandates that a Portal-typed cell is stored inline as a (slot_path, generation) pair — that is neither a primitive nor a pool index, so it has no legal tag. Array-typed cells hit the same wall: the vocabulary admits arrays, the pooling entry pools only record/enum/tuple, so an array cell is neither primitive nor pooled and has no tag. The SPEC's own type_tag grammar also lacks a Portal/Handle tag, so the gap is on both sides.

Option A — widen the cell-type-tag enumeration to cover every cell-storable type:
```
cell.type = primitive | string_pool_index | dynamic_pool_index
          | (slot_path, generation)   // Portal, stored inline
          | array_tag                 // [T;N] inline
```
Every storable type gets exactly one tag. Requires editing the tag entry and the grammar together.

Option B — force Portal and array cells through the pool (no inline):
This contradicts the entry that explicitly says Portal cells are Copy and stored inline, so it would require reversing a deliberate representation choice. Rejected on that ground.

Recommended: Option A — widen the cell-type-tag enumeration so Portal (inline (slot_path, generation)) and inline arrays each get an admissible tag, and add the matching grammar productions. Because the inline-Portal-cell rule and the array vocabulary are both live, deliberate decisions; the narrow tag entry is the outlier, and the fix is to make it cover what the corpus already requires cells to hold.

For execution: LOG 033-79 (widen cell type-tag set to admit Portal (slot_path,generation) + array), reconcile with 033-48 (Portal inline), 033-83 (aggregate pooling), 033-43 (vocabulary); SPEC §15.4.1 grammar (24529-24533) add Portal/Handle + confirm array tag. Finding F239. Coupled to F240 (same grammar) — resolve together.

#### I-20 — `bundle[g]`'s index-dependent slice length cannot come from a single `Index[isize]` Output

Background: a bundle is indexed with `bundle[g]` to get one row (a slice of handles). This goes through the Index trait, whose result type (`Output`) is fixed per (type, key) pair.

The problem: the log says `bundle[g]` returns `Handle[T][..N]` when the index is compile-time-known (fixed-length slice) and `Handle[T][..]` when the index is runtime (unknown-length slice). But the SPEC section the log cites for this pins the bundle's `Index[isize]` Output to `Handle[T][..]` only, and states Output is a function of (Self, Key) — one Output per (Bundle[T], isize). A single-Output trait cannot yield a length that depends on whether the index is a constant. So the trait mechanism as written cannot express what the log asserts.

Option A — drop the compile-time-length claim; `bundle[g]` always returns `Handle[T][..]`:
```
bundle[g] : Handle[T][..]   // always runtime-length, even for constant g
```
Makes the trait consistent. Loses the static-length information for constant indices — a real capability loss if downstream code relies on `[..N]`.

Option B — take row access OUT of plain Index trait fulfillment, give it its own access path that can vary the length:
```
bundle[g]  ->  Handle[T][..N]  for compile-time g
bundle[g]  ->  Handle[T][..]   for runtime g   // not a single Index[isize] Output
```
Keeps the index-dependent length. Requires the SPEC section to describe bundle row access as a special form, not a straight Index[isize] instance.

Recommended: Option B — reconcile by making the cited section express bundle row access as a dedicated form whose result length tracks whether the index is constant, rather than a single fixed Index[isize] Output. Because the log states the varying-length behavior in two separate entries and another SPEC section already elaborates the `[..N]` variant; the defect is that the cited Index section never reconciles it. Aligning the cited section to the behavior the rest of the corpus already commits to is the smaller change than deleting a stated capability. Flag to the user if they would rather drop `[..N]` for trait simplicity.

For execution: LOG 017-90, 017-99, 017-101 (varying row-slice length) vs SPEC §4.9.5 (pins Index[isize] Output=Handle[T][..] only, Output=(Self,K)); §13.3.3.5 (13687,13711,13782) already elaborates [..N]. Reconcile §4.9.5 to express constant-index [..N] as a distinct access form. Finding F186.

#### I-21 — `bundle_in_repeat_rejected` is promised by the LOG but elaborated nowhere in the SPEC

A log entry defines a normative, tooling-targetable diagnostic class named `bundle_in_repeat_rejected` and a two-way rejection (no bundle bracket inside a repeat body, no repeat inside a bundle bracket), citing a section that does not elaborate it — that class name appears zero times in the SPEC, and the only related prose lives in a different section and covers only one direction. Recommended: repoint the log entry's citation to the section that actually discusses bundle brackets and repeat, then extend that section to state both rejection directions and to introduce the `bundle_in_repeat_rejected` class name by name. Because a normative diagnostic class the log promises must be elaborated somewhere, and per the citation policy the default fix is to repoint to the owning section rather than duplicate the rule into the cited-but-silent one.

#### I-22 — Whether a standalone view's cardinality is enforced against the caller

Background: acceptance entries say what children a caller may supply (the caller-facing contract). A standalone `view` is a receiver-side selection over children already accepted — the log says it "never widens acceptance." But a view carries a cardinality (e.g. `Drivable+` means at least one; a bare view means exactly one).

The problem: put a standalone view `view drivables: Drivable+` (min 1) over `all: Node*` (min 0). If the view's `+` is enforced against the caller, then supplying zero children is rejected — that tightens the caller's minimum from 0 to 1, which is widening the caller's obligation and contradicts "never widens acceptance." If instead the cardinality is inert on a standalone view, no rule says so, and the exactly-one default plus the `+` example become meaningless. An implementer cannot tell whether supplying zero Drivables compiles.

Option A — standalone-view cardinality is selection-only, never enforced against the caller:
```
view drivables: Drivable+   // + describes the selected subset, does NOT reject zero at the call site
```
The caller contract stays exactly the acceptance entries; the view just names/filters. Consistent with "never widens acceptance." Cost: the cardinality on a standalone view is documentation of the expected shape, not a check — the `+`/exactly-one specifier does no placement-time work.

Option B — standalone-view cardinality IS enforced, and "never widens acceptance" is narrowed:
```
view drivables: Drivable+   // rejects a placement supplying zero Drivables
```
The view can tighten the caller's minimum. Requires rewriting "never widens acceptance" to carve out standalone-view cardinality, and reconciling with the generic rule that "each view's cardinality constrains the count of supplied children."

Recommended: Option A — pin that a standalone view's cardinality is selection-side only and is not enforced against the caller, and state it explicitly in a log entry. Because "never widens acceptance" is the stated, load-bearing role of the standalone view (it exists precisely to be receiver-side and not touch the caller contract); making cardinality enforceable there would invert that role. The generic "cardinality constrains supplied children" rule should be scoped to acceptance-entry views, not standalone ones. Surface to the user, since this does void the caller-facing meaning of a standalone view's specifier.

For execution: LOG 017-19 ("never widens acceptance"), 017-28 (bare view = exactly one), 017-9; add a log entry pinning standalone-view cardinality as selection-only/inert against the caller; conform SPEC §13.3.3 / §13.3.3.1, and scope SPEC 13335 ("each view's cardinality constrains supplied children") to acceptance-entry views. Cross-check 021-41 (cardinality enforced at placement) applies to acceptance entries only. Finding F160.

#### I-23 — A trait declaring both an effect-kind method and a required cell is satisfiable by no kind

Background: traits are kind-specific by what they declare. A trait with an effect-kind method can be satisfied only by an effect. A trait with a required cell (attr/const) can be satisfied only by a node or connection.

The problem: nothing stops a trait from declaring BOTH an effect-kind method AND a required cell. Such a trait is satisfiable by no kind at all — effects can't have the required cell, nodes/connections can't have the effect method. No rule in the trait section forbids the combination or resolves it, and no legal boundary (standard-library-defined or implementation-defined) covers it.

Option A — reject at declaration:
```
trait T:
  effect run(self: Subject): ...   // effect-kind method -> effect-only
  attr x: i32                      // required cell -> node/connection-only
// ERROR at trait declaration: no kind can satisfy both
```
The compiler flags the trait itself as unsatisfiable-by-construction. Clean; the error lands where the mistake is, not at some far-off satisfies site.

Option B — leave it legal but permanently unsatisfiable:
The trait declares fine; every `satisfies` for it fails. This is strictly worse — the real error (impossible trait) is reported far from its cause, once per attempted satisfier, with a confusing per-kind message.

Recommended: Option A — add a rule making a trait that declares both an effect-kind method and a required node/connection cell a declaration-site error. Because the two existing kind-gating rules are individually sound; their intersection is empty, and the only useful place to report an empty-intersection trait is where it is declared, not at every doomed satisfier.

Scope note: this item owns ONLY the declaration-site-error rule. Do not touch the satisfies-waiver's "declares no required cells" wording here — that rewrite is owned and executed solely by the master key that scopes what "required cell" means in the waiver (node/connection members only; observed cells do not block the waiver). Two independent rewrites of the same waiver sentence is exactly the defect to avoid.

For execution: add a LOG entry near 005-51/005-50/005-59/005-61 forbidding effect-kind-method + required-cell in one trait as a declaration-site error; conform SPEC §3.1.7. Do NOT edit 005-67's "declares no required cells" wording — that half is owned and executed solely by master key K-OBSERVED-REQCELLS. Finding F013 (declaration-site-error half only).

#### I-24 — Is an effect-kind method's Subject-typed first parameter mandatory or conventional

The log states as a hard requirement that an effect-kind method's first parameter IS Subject-typed, but the SPEC says it is only "conventionally" Subject-typed and that trait methods may have any parameter list — and the SPEC is internally split, since its dispatch section relies on the Subject-typed first parameter as a mandatory fact. This gives an implementer opposite enforcement rules. Recommended: make it mandatory — an effect-kind method's first parameter must be Subject-typed — and change both the softening SPEC clause and the "any parameter list" clause to say so for effect-kind methods specifically. Because effect dispatch already treats the Subject-typed first parameter as load-bearing (the effect is dispatched on the Subject type), so the convention is in fact relied upon as a rule; the "any parameter list" freedom belongs to ordinary value trait methods, not effect-kind ones. Surface the modality choice to the user, since it is the log-vs-SPEC adjudication.

#### I-25 — `slice` / `byte_slice` with reversed bounds is undefined

The two string-slice methods list exactly two ways to fail: a byte index that lands mid-character, and a position out of range. A call like `s.slice(5, 2)` — both indices valid but start past end — is neither. Nothing says whether it returns the empty string or aborts the process, so an implementer has no rule to follow and two programs would behave differently.

Recommended: return the empty string when start > end (both in range), because the range operator already treats `start >= end` as empty, and matching that keeps one consistent rule for reversed spans across the language instead of a surprise process abort.

For execution: LOG 012-31, 012-32, 012-35; SPEC §9.1.7 (lines 6466-6468). Add a `start > end` = empty-string rule for both methods and reflect it in 012-35. Finding F073.

#### I-26 — Cloning a Map does not state it keeps insertion order

Map iteration, Display, and repeat all yield entries in insertion order — that order is an observable language guarantee. Clone is compiler-derived, but neither the log nor the spec says the clone keeps the source's order; only Eq and Hash are pinned as order-insensitive. So a clone could rehash into a different order and `for (k,v) in m` would diverge from `for (k,v) in m.clone()`.

Recommended: pin that derived Clone on Map preserves the source's insertion order, because insertion order is already normative for every other Map observation and a silent reorder on clone would be an invisible behavior change the corpus otherwise forbids.

For execution: LOG 012-115 (also 012-107, 012-112); SPEC §9.5.13 (lines 7465-7469), §9.5. State clone-preserves-insertion-order. Finding F072.

#### I-27 — What type `.count` returns — `isize` or `usize`

**Background.** Ductus reports the number of elements in a collection through one accessor, `.count`, used on arrays, slices, bundles, Map, Vec, sets, and views. The log fixes the array length and index type at `isize` and states flatly there is no `usize` length type. It also forbids an implicit crossing between same-width signed and unsigned integers.

**The problem.** The spec binds `.count` results to `usize` in its own examples:

```
let n: usize = arr.count     // spec example
```

If `.count` returns `isize` (matching the length type the log pins), this example is ill-typed — it needs an explicit `usize(...)` cast, because signed-to-unsigned is never implicit. If instead `.count` really returns `usize`, then the log's "there is no usize length type" is violated. The two documents cannot both be right, and neither one ever states `.count`'s return type as a normative rule — the type only shows up inside examples.

**Options.**
- `.count` returns `isize`. Matches the pinned length type and `char_count` (which already returns `isize`); the spec examples become the thing to fix.

```
let n: isize = arr.count      // no cast needed
```

- `.count` returns `usize`. Matches the spec examples as written but contradicts the log's "no usize length type" and would make count a special unsigned island.

**Recommended: `.count` returns `isize`, and fix the spec examples**, because the log already commits the whole length/index/tally family to `isize` (array length, index, and `char_count` all signed) and the only counter-evidence is three unpinned spec example lines, not a normative rule. This is a real owner-facing call — it changes the type of a pervasive accessor — so flag it rather than treat it as pure cleanup.

For execution: LOG 012-91 (pin return type here), 012-124, 012-30, 007-7, 012-133; SPEC §9.3.7, §9.3.3, §9.3.4, examples at 6987-6988 and 13587. Add a normative `.count -> isize` statement to 012-91/§9.3.7, then rewrite spec examples to `isize` or add explicit casts. Surface the isize-vs-usize choice to the owner. Finding F054.

#### I-28 — Associated-type projection is spelled both `.` and `::`

**Background.** Ductus projects an associated type out of a trait or type parameter — e.g. getting the `Item` type out of an iterator.

**The problem.** Chapter 12 writes this projection two ways. One rule mandates dot member-access for associated types, giving `T.Iter.Item`. Another rule, defining the loop variable's type, writes the same projection with `::`:

```
Iter::Item          // where Iter = Iterable::Iter    (one rule)
T.Iter.Item                                            // the other rule
```

Trait-item path references like `Iterator::next` and `Iterable::iterator` are a defensible separate use of `::` — those are items, not type projections. But `Iter::Item` and `Iterable::Iter` are associated-*type* projections, exactly the case the dot rule says must use `.`. An implementer building the grammar cannot tell whether `Iter::Item` and `Iter.Item` are one production or two.

**Options.**
- One spelling: `.` for all associated-type projection. Rewrite the `::` uses. Matches the general dot-notation rule and the spec's own `Iter.Source` spelling.

```
Iterable.Iter.Item
```

- Admit both, with a disambiguation rule. Amend the dot rule to permit `Trait::Assoc` as an equivalent spelling. Keeps existing text but adds a second production for one construct.

**Recommended: one spelling, `.` member-access**, and rewrite the `::` projections, because the dot rule is the one stated as the general mandate, the spec already writes Iterable's own associated type with a dot (`Iter.Source`), and keeping two spellings for a single grammar production is exactly the ambiguity to remove. Confirm this reads consistently with the trait-item `::` path uses, which stay untouched.

For execution: LOG 014-42 (rewrite to dot), 014-157 (canonical rule); SPEC §12.3.3 (lines 10158-10160), §12.12.3, §3.1.2 (1282-1291). Keep 014-26/014-104 `::` path refs unchanged. Finding F045.

#### I-29 — The consuming enumeration omits the indexed slot

**Background.** "Consuming" is the move-semantics term: an operation consumes a value when it takes ownership and the source name goes dead. One log entry defines what consuming *is* as a closed list.

**The problem.** The definition opens with "Consuming comprises" (a closed enumeration) and lists its structural storage sinks as record field, tuple component, or enum payload — no indexed slot. But three other entries in the same chapter treat indexed slots as consuming owned storage: one puts indexed assignment in the category that "consumes the RHS," one forbids a borrow alias in an indexed slot (only meaningful if indexed slots hold owned values), and one lists indexed slots among owned storage sites.

```
arr[i] = v      // definition says: does NOT consume v (v stays live)
arr[i] = v      // the other three entries say: consumes v (v dies)
```

An implementer following the definition leaves `v` live after `arr[i] = v`; the other rules kill it. Same operation, opposite answers.

**Options.**
- Add "indexed slot" to the consuming enumeration. Makes the definition list the same four structural slots the storage-site rules list. `arr[i] = v` consumes `v`.
- Reword "comprises" as a non-exhaustive "includes." Turns the definition into an open list, so omission stops meaning exclusion. Note the spec's parallel sentence already uses "includes" and also omits indexed slot.

**Recommended: add "indexed slot" to the consuming enumeration**, because three separate entries — including the category rule that explicitly names indexed assignment as consuming the RHS — all agree indexed slots hold owned values, so the definition is the outlier and should list all four structural slots rather than stay open and silent. This changes whether `arr[i] = v` kills its RHS name, so surface the wording choice (add-slot vs open-list) to the owner rather than pick it silently.

For execution: LOG 013-42 (add indexed slot), 013-3, 013-52, 013-72; SPEC §11.3.1 (line 8315), §11.1, §11.3.4, §11.3.6. Align the consuming enumeration and storage-site enumerations on the same four slots. Owner-facing wording call. Finding F042.

#### I-30 — Duration suffixes: reserved "in the same scope" vs "in any scope"

One log entry says the built-in duration suffixes may not be re-registered for another type "in the same scope," which permits a different module to register `ms` for its own type. But its own cited spec section, plus another log entry, say re-registration is forbidden in "any scope" — reserved everywhere. The two readings give different compiler behavior for `@literal_suffix("ms", ...)` in a module that does not import the duration built-ins.

Recommended: forbid re-registration in *any* scope (fix the outlier to match), because two of three sources — including the spec section the outlier itself cites and the entry that makes duration suffixes globally visible — say reservation is global, so the "same scope" entry is the one out of step. This is a real behavior change, so surface the same-scope-vs-any-scope wording to the owner rather than resolve it unilaterally.

For execution: LOG 005-236 (the outlier, "same scope"), 005-224, 005-238; SPEC §3.9.4 (line 2825), §3.9.1, §3.9.5. State the reservation scope identically across 005-224/005-236 and match §3.9.4 "any scope." Owner-facing. Finding F035.

#### I-31 — "Interpretation context" switches `match` lowering but is never defined

**Background.** Two log entries branch how `match` compiles on whether it is "in interpretation context." In that context a `match` on a reactive scrutinee lowers to given-style build-all-arms-and-freeze, and a `match` over exposition entries becomes a static unroll plus a mount-time tag. Outside it, `match` is a plain value selector.

**The problem.** "Interpretation context" is the switch that flips this lowering, but neither document ever defines it. There is no rule stating which syntactic positions or evaluation phases count as one. The nearby definition of "interpretation" (walking `.exposition`) does not say which contexts qualify. Two readers can disagree on whether a given `match` is in interpretation context and get different compiled output — build-all-arms versus single-arm.

**Recommended: define "interpretation context" as the exposition-walk (interpretation) phase** — the set of positions and phases reached when the runtime walks a node's `.exposition`, tied to the already-defined interpretation pass — because the master decision on interpretation-context `match` keeps a `match`-selects / `given`-freezes split and deletes the outlier "builds all arms and freezes" clause. So the term still needs a definition to scope where `match` static-unrolls, but the freezing half of these two entries is being removed anyway. This item just supplies the missing definition of the phase those entries branch on.

This is not a standalone edit. It is folded into the interpretation-context `match` master decision as the answer to that decision's own open residual — whether to (a) define "interpretation context" or (b) replace the phrase with a plain description. This item picks (a) and defines it once. Do not also write the definition separately: the master decision already edits these same entries and deletes the build-all-arms clause, so a second independent edit would either re-add the term the master might remove or write the definition twice.

For execution: fold into master key K-MATCH-INTERP as the proposed option-(a) answer to its "interpretation context" residual. Do NOT execute as a standalone edit. Execute once against post-deletion LOG 009-90 (after K-MATCH-INTERP deletes 009-89's build-all-arms-and-freeze clause): add a definitional statement of "interpretation context" (which positions/phases qualify, tied to the `.exposition` walk), then confirm 009-89/009-90 against it. Conform the SPEC anchors named in K-MATCH-INTERP's verification greps: SPEC §6.2.5, §13.3.7.7, §13.19. Finding F233.

#### I-32 — The reload sequence never opens the reconciler registration window it promises

**Background.** A running Ductus program can hot-reload. New or renamed effect types may appear in the reload, and before the reload goes live the host must register a reconciler for any such type — the log calls this the reload's "pre-live phase."

**The problem.** The reload procedure is a closed 10-step atomic sequence: acquire the reload lock, drain, diff, allocate, drop, update the behavior table, recompute deriveds, commit, release the lock. New/renamed effect types become known only at the diff step; the reload goes live at the commit step; steps in between run under the lock, which pauses host writes. No step hands control back to the host to call `register_reconciler`. Registration while live is rejected, and queued host requests apply only after the lock releases — after go-live, too late. And unlike startup, which refuses to go live when a required reconciler is missing, there is no reload-abort-on-missing-reconciler rule. So the "pre-live phase" names a window the sequence never actually opens.

**Options.**
- Add a host-registration step to the sequence. After the diff identifies new/renamed effect types needing reconcilers and before the reloaded commit, yield to the host to register them; add a reject-on-missing rule mirroring startup.

```
step 4: diff  ->  step 4b: host registers reconcilers for new/renamed types  ->  ... commit
```

- Restate where in the existing steps registration already happens. If the intent is that the host registers via some pre-diff call, say so explicitly and reconcile with the "rejected once live" rule.

**Recommended: add an explicit host-registration window plus a reject-on-missing rule**, because the log already asserts a pre-live registration phase and requires new-type instances with host-written observed channels to have a reconciler before go-live, yet the sequence physically leaves no point to place that call — so the fix is to open the window the contract already promises, mirroring the startup abort-on-missing behavior. This is a timing-contract gap in a mandatory-core verb; direction only, exact step placement is an owner call.

For execution: LOG 028-13..028-24 (reload sequence, esp. 028-15 lock, 028-23 commit, 028-24 release), 027-67, 028-62, 027-80/027-81; SPEC §13.15.3 (19322-19355), §13.15.6 (19509-19512), §13.14.7 (19048-19052). Add a post-diff/pre-commit host-registration step and a reject-on-missing-reconciler rule. Relates to master key K-HOOK-ORDER (reconciler hook/commit model) but is a distinct sequence gap. Finding F090.

#### I-33 — The spec's postfix precedence tier drops `?.`, `?[]`, `?()`

The operator-precedence table has two lists of postfix operators that disagree. The log's postfix tier includes the optional-chaining forms `?.`, `?[]`, and `?()` (optional field access, optional index, optional call). The spec's precedence table at the same tier lists `?`, `.`, `[]`, `()` and the cast-call forms, but drops those three optional forms. The log is the decision-of-record and the spec must conform.

Recommended: add `?.`, `?[]`, `?()` to the spec's postfix tier so it matches the log, because the log enumerates them explicitly as postfix operators and the spec table is the list an implementer parses precedence from — leaving them out makes the optional forms look like they don't exist at that tier.

#### I-34 — A letter glued directly onto a flag run has no lexing rule

Background: a placed type can carry a run of single-character flags glued to its name with no space, like `Pin'` or `Component?*`. A flag is one of a fixed set of non-letter characters (`'`, `!`, `?`, `*`, `+`, `^`, `~`, `@`, `$`). The bare instance-name form puts whitespace after the flag run: `Pin' p1`.

The question: what happens when an identifier character is glued directly onto a flag run with no whitespace? Every example in the corpus separates the flag run from what follows by a space or a slot character. The zero-separator case is undefined.

```
Pin'!x     // '!' ends the flag run, then 'x' is glued on with no space
```

Options:

- Lex error: a letter immediately after a flag run with no whitespace is rejected. `Pin'!x` fails to tokenize.
- Start the next element: the letter begins the instance name (or whatever inline element follows), so `Pin'!x` reads as type `Pin'!` then name `x`.

Recommended: make the glued identifier char a lex error, because the log already says a bare type including its flag run is self-delimiting AND the only shown way to attach an instance name is with intervening whitespace (`Pin' p1`). Silently letting `x` start the name would create a second, whitespace-free spelling of instance placement that no rule sanctions and that reads badly. A hard error with a diagnostic ("insert a space") is the safe, non-committal default; if the user later wants glued names, that is a deliberate additive decision, not a lexer accident.

#### I-35 — How the lexer scans a string-interpolation hole is undefined

Background: layout (indentation-sensitivity) is suspended inside string literals so strings can span lines. Strings also interpolate arbitrary expressions inside `{...}` holes — method calls, arithmetic, field access, and even nested string literals.

The question: how does the lexer scan an interpolation hole? Nothing says whether it tracks bracket/brace/string nesting through the hole or just runs to the first `}`.

```
"total {items[compute("a")]}"   // hole contains [ ( and a nested "a" string
```

Here the hole has a `[`, a `(`, and an inner `"a"`. Two implementers can disagree:

- Flat scan: the hole ends at the first `}`, and inner content is treated as raw text. This breaks the moment an expression contains a `}` or a nested string with its own `}`.
- Balanced scan: the lexer re-enters full lexing inside the hole, tracking nested strings, brackets, and braces, and the hole closes on the matching `}`. The inner `"a"` is a real nested string literal with its own escape rules.

Recommended: balanced scan that re-enters full lexing, because the corpus already says interpolation expressions are arbitrary expressions including method calls and field access — which by definition contain brackets and can contain nested strings. A flat first-`}` scan cannot tokenize the corpus's own interpolation examples once they nest, so balanced is the only reading consistent with what interpolation already promises. Also state that layout stays suspended inside the hole (it is still inside the string literal).

#### I-36 — A loop `else:` at arm indentation is ambiguous with a new `when`/`given` arm

Background: `when:` and `given` blocks are lists of arms, and every line at the arm indentation is read as a new arm. A `for`/`while` loop can carry an `else:` clause, and the rule says that `else:` sits at the loop head's indentation, dedented from the loop body.

The question: what happens when a loop is the entire body of a `when`/`given` arm, so the loop head sits at the arm indentation? Then the loop's `else:` — which must dedent to the loop head — lands at the arm indentation too, where every line is supposed to be a new arm.

```
when:
  cond:
    for x in xs:
      ...
    else:        // at arm indent — loop's else, or a new arm header?
      ...
```

The `else:` is ambiguous: it could be the loop's `else` clause, or the start of a sibling arm. The token spelling doesn't save us here because the collision is about indentation position, not the keyword.

Options:

- Depth rule: a loop `else:` is only recognized when the loop head is nested strictly deeper than the enclosing arm/block. A loop that is itself the whole arm body cannot have a dedented `else:` at arm indent; it must be indented one level.
- Require the loop body to be indented one level below arm position whenever it carries an `else:`, forcing the `else:` off the arm line.

Recommended: the depth rule (loop `else:` recognized only when the loop head is strictly deeper than the enclosing arm indentation), because it is the minimal disambiguation and keeps the existing "every line at arm indent is an arm" invariant intact. The alternative bakes a formatting mandate into the parser for one narrow case. Note: this assumes the loop-else placement rule stays dedent-based; if the user prefers to re-spell loop-else the way the fallback arms were re-spelled (`otherwise:`/`default:`), that is a bigger call for them.

#### I-37 — `yield if` is ambiguous between conditional-value form and a separate statement

The corpus sanctions `yield if c: a else: b` as the conditional-value form inside a `collect`. But `yield <expr>` followed by a structural `if` block also exists, so `yield if ...` is surface-ambiguous: is the `if` the yielded value, or a separate `if` statement after a bare yield? The two readings are not lexically distinguished, and structural `if` gating a yield is exactly what the conditional-value form was introduced to forbid.

Recommended: state that an `if` or `match` immediately following `yield` on the same line is always parsed as the conditional-value expression (the yield's operand), never as a separate statement, because the corpus explicitly offers `yield if c: a else: b` as the sanctioned form and says `if`/`match` never gate structure — so the same-line `if` after `yield` can only be the value form. Add an example contrasting it with a block `if` under `collect`.

#### I-38 — The referential-transparency floor never says read reactive cells count as inputs

The normative floor says every user-defined `fn` is referentially transparent — same inputs give same outputs — and bounds its effects to the declared return value, which reads as "inputs = declared parameters." But a later rule lets a `fn` body directly read a module-level signal, so `shifted(base_value)` returns different results across commits for the same `base_value` when the signal changed. The provenance machinery reconciles this by folding the read cells into the caller's provenance and re-running the fn per emission, but the floor never says "inputs" includes transitively-read reactive cells, so a reader of the floor alone can't tell whether such a fn is conformant.

Recommended: tighten the referential-transparency statement to define "inputs" as all reactive cells in the call's provenance (declared arguments plus directly/transitively read cells), because the provenance rule already treats the read signal as an input by folding it into provenance and re-running — the floor just needs to name that so the two layers agree. This is a definitional edit to the philosophy layer; surface the exact wording to the user rather than redefining "inputs" unilaterally.

#### I-39 — The non-yielded-element drop rule names `break` but omits enclosing function return

The drop-insertion decision for non-yielded iterator elements says the drop is inserted "at the point of `break`," but the spec section it cites covers two early-exit paths that leave elements un-yielded: `break` AND an enclosing function return. The log entry names only `break`, understating the section it points into. The function-return case is arguably covered by the separate end-of-scope / end-of-function drop rules, but this entry presents itself as the rule for non-yielded iterator elements and silently drops one of its section's two exit paths.

Recommended: broaden the entry to name both exit paths (break and enclosing function return) per its cited section, because the citation is load-bearing and an entry should not state less than the section it points into; if the end-of-function drop rule genuinely subsumes the return case, instead repoint the citation so the entry isn't advertising coverage it doesn't carry. Surface which of the two to the user.

#### I-40 — Division is written `\` instead of `/` in two SPEC places

The spec's list of admitted compile-time operations for const-generic arguments writes division as a backslash `\` instead of `/`. `\` is not a Ductus operator — it only appears as an escape character inside char/string literals — and the division operator is `/`. Fresh reads show the same wrong glyph in a second place: the operator-precedence table's multiplicative tier lists `*`, `/`, `\`, `%`, where the log's multiplicative tier has no `\`. So there are two occurrences of this typo, not the one the audit flagged.

Recommended: replace `\` with `/` in both the const-generic operations prose and the precedence-table multiplicative tier, because `/` is the language's division operator per the log and `\` denotes no operator — both are pure spec typos with no log backing to reconcile.

#### I-41 — The `in` membership operator has no precedence tier

Background: Ductus has a precedence table that ranks every operator so the parser knows how to group an expression. `in` is a real operator — `k in s` tests membership — but it is missing from that table. One decision even justifies how `not k in s` parses by claiming `not` "binds looser than comparison," which quietly assumes `in` sits at the comparison tier. Nothing in the table actually places it there.

The question: where does `in` bind, and how does an expression that mixes `in` with a range parse?

This matters because ranges are a natural thing to test membership against:

```
x in 0..n
```

Without a tier for `in`, this has two readings:

```
x in (0..n)     // membership in the range 0..n  — almost certainly intended
(x in 0) .. n   // range from the bool (x in 0) up to n — nonsense
```

Options:

- Place `in` just below the range tier (looser than `..`), so `x in 0..n` groups as `x in (0..n)`. This is the reading every real program wants.
- Place `in` at the comparison tier alongside `<`, `is`, etc. This is what the existing justification text already assumes, but it makes `x in 0..n` parse as `(x in 0)..n`, which is broken.
- Mark `in`'s precedence implementation-defined. Legal per the corpus's own "only std-delegation or implementation-defined are valid stopping points" rule, but it pushes a program-visible parse decision onto implementers for no reason.

Recommended: place `in` below the range tier so `x in 0..n` reads as membership in the range, because the range is the natural membership subject and the alternative produces a meaningless parse. Then fix the "binds looser than comparison" justification text, which asserts a tier that will no longer be true. Note the same missing-tier problem recurs in the map-membership decision, so fixing one without the other leaves the hole half-open.

For execution: LOG 007-231 (repair justification), 012-104 (same gap, map `in`); add `in` row to SPEC §4.4.7 precedence table (between tier 8 `..` and tier 9 comparison), reconcile SPEC §4.9.5. Finding F037.

#### I-42 — The placement-position flag-run rule over-reaches

The rule says any non-letter character right after a TypeRef opens a "flag run," but `/` (a `/expr` placement argument) and `|` (an attribute clause) also follow a TypeRef legally and are not flags — so `Drives/0.8`, which the corpus elsewhere calls valid, mis-lexes as an invalid flag run.

Recommended: narrow the rule so the flag-run opener must be one of the nine actual flag characters, not any non-letter character, because those nine are the only characters that ever legally open a flag run — every other non-letter that reaches a TypeRef (`/`, `|`, `:`, `(`) belongs to its own slot.

For execution: LOG 021-119 (narrow "non-letter character" to the 021-113 flag set), reconcile SPEC §13.8.8.4; cross-check 021-76 (`/expr`), 021-97 (attribute clause). Finding F105.

#### I-43 — Nested inline `if`/`else` has no association rule (the dangling-else problem)

Background: an `if`/`else` arm may be a single inline expression, and that expression may itself be an `if`. Since a value-`if` need not carry an `else`, an inner `if` with no `else` sitting inside an outer `if` leaves an `else:` that could attach to either one. No rule anywhere says which.

The question: when an `else:` could bind to an inner or an outer inline `if`, which one claims it?

The ambiguity is behavior-changing:

```
let z = if a: if b: x else: y
```

Two valid parses, different results when `a` is true and `b` is false:

```
if a: (if b: x else: y)     // else binds inner → z = y
if a: (if b: x) else: y     // else binds outer → z = the inner if's value
```

Options:

- Bind `else` to the nearest unmatched `if` (the classic C/Rust rule). Predictable, familiar, keeps the terse inline form usable.
- Forbid a bare un-parenthesized inline `if` as an inline arm — force parentheses or a block. Removes the ambiguity by construction but costs some of the inline form's convenience.

Recommended: bind `else` to the nearest unmatched `if`, because the corpus is otherwise meticulous about association (it pins move-parenthesization, `with` associativity, `/expr` atomicity), and nearest-match is the universally expected default that keeps the inline arm form intact. The alternative — banning bare inline `if` — is a heavier surface restriction to close a hole a one-line association rule already closes.

For execution: add a decision fixing dangling-`else` to nearest-unmatched-`if` (topically near LOG 002-26); elaborate in SPEC §1.4. Finding F100.

#### I-44 — Import name-collisions are ruled only for glob-vs-glob

Two other collision cases have no rule.

Background: one decision says two glob imports that bring the same name into a scope are a compile error. But a single `use` may mix a selective (named) import and a glob, and imports land in the same scope as local declarations. So two other collisions can happen, and nothing covers them.

The question: what happens for these two uncovered cases?

```
use a::(Foo, b::*)   // selective Foo AND a glob that also brings Foo
// case (a): error? or does the explicit Foo win?

use a::*             // glob brings Foo
signal Foo = ...     // same file declares Foo locally
// case (b): error? or does the local shadow the glob?
```

Options per case:

- Case (a), selective-vs-glob: make the explicit named import win (a glob is a wildcard; an explicit name is more specific), OR make it the same compile error as glob-vs-glob.
- Case (b), glob-vs-local: make the local declaration shadow the glob (standard "explicit local beats wildcard import"), OR make it a collision error.

Recommended: explicit-import-wins for case (a) and local-shadows-glob for case (b), because both follow the same principle the glob-vs-glob rule already implies — a wildcard is the weakest binding, and anything named explicitly (an import selection or a local declaration) is more specific and should win rather than error. Keep the hard error only for the genuinely symmetric glob-vs-glob case, where neither side is more specific. This is a recommendation, not a settled fact — the corpus states neither outcome, so confirm the specificity-wins framing before pinning it.

For execution: add 003-series rules for selective-vs-glob and glob-vs-local collisions (near LOG 003-46/003-47/003-48), elaborate in SPEC §10.4.1; cross-check §13.7 name resolution. Finding F005.

#### I-45 — A LOG decision points at a SPEC section that never elaborates it, and a behavior-determining default goes unspecified

Background: the LOG's use-sites decision lists, among the places a type gets pinned, a consuming `stream` declaration that fixes a `.changes` policy and capacity — and it adds the rule that if no such site exists, the policy defaults to `ring[1024]`. It cites the type-inference use-sites section. That SPEC section lists five ordinary use sites and stops; it never mentions `.changes`, the stream materialization, or the `ring[1024]` default. The default changes real program behavior and is elaborated nowhere.

The question: where does the `.changes` / `ring[1024]` material live, and how do we fix the broken pointer?

Options:

- Add the stream/`.changes` use site and the `ring[1024]` default into the cited use-sites section, matching the LOG.
- Repoint the LOG citation to whatever section actually elaborates `.changes` materialization and stream capacity, and elaborate the default there.

Recommended: repoint rather than duplicate, following the corpus's default citation policy (move the reference to the section that already owns the material). The `.changes` policy and stream capacity are a streams-chapter concern, not a type-inference-use-sites concern; cramming stream buffering into the use-sites list would scatter one topic across two homes. If no section currently owns the `ring[1024]` default, add it to the streams section that defines `.changes`, not to the use-sites list. Confirm which section elaborates `.changes` before repointing.

For execution: LOG 004-4 — repoint (SPEC §2.1.2) to the `.changes`/stream-capacity section, or add the use site + `ring[1024]` default to SPEC §2.1.2. Finding F002. (Citation-repoint-vs-extend follows master key K-CITE-POLICY.)

#### I-46 — The Map key-bounds decision omits `duration` and `instant`, but the SPEC lists them as valid keys

The LOG's §9.5 authority enumerates only integers/bool/char/string; the SPEC also lists `duration` and `instant` and spells out every integer width. A separate decision already grants `duration` and `instant` a `Hash` implementation, so the SPEC and that decision agree — the key-bounds decision is just the stale, under-specified side.

Recommended: bring the key-bounds decision up to the SPEC's fuller list (all integer widths, plus `duration` and `instant`), because those two types already auto-implement `Hash` elsewhere in the corpus, so `Map[instant, V]` is legal by that rule and the key-bounds decision should license it too. This lands the same key set the master widening decision reaches on other grounds.

This is not a standalone edit. The master widening decision already reconciles this same enumeration against the widened StringifiableKey and the SPEC list, and its open residual proposes defining StringifiableKey and the Map key bound as ONE canonical set. So this fix must run inside that reconciliation and produce a single canonical list that both the Map key bound and StringifiableKey reference — not a second independent edit to the same enumeration, which would risk two subtly different "canonical" lists.

For execution: execute as part of master key K-REPEAT-KEYING's 012-101 reconciliation — do NOT execute standalone. Extend the qualifying key set to match SPEC §9.5.3 (i8..i128, u8..u128, isize, usize, bool, char, string, duration, instant), and produce ONE canonical list referenced by both the Map key bound (LOG 012-101) and StringifiableKey (per the recommended "same set" answer to K-REPEAT-KEYING's F229 residual). Cross-check 007-236 (duration/instant Hash). Finding F020.

#### I-47 — Composite-map slot paths are spelled two different ways in the two documents

Background: a reactive composite built over a literal-key map exposes each slot at a path. The LOG spells that path with brackets; the SPEC spells it with a dot. Same construct, two different program-visible syntaxes — an implementer building the path grammar cannot satisfy both.

The question: which surface addresses a composite-map slot?

```
// LOG (012-111, 012-120), twice:
binding['a']
// SPEC §9.5.12:
binding.a
```

Options:

- Bracket form `binding['<key>']`. Parallels array-slot paths `binding[<index>]`, and map keys are values (strings, ints), so bracket-with-a-key-value is the natural map-indexing surface.
- Dot form `binding.<key>`. Parallels record-field paths, which is the analogy the SPEC leans on; but it only reads naturally when the key looks like an identifier and gets awkward for non-identifier keys.

Recommended: the bracket form, because the LOG (the decision-of-record, which the SPEC must conform to) pins it twice and explicitly ties it to the array-slot parallel `binding[<index>]`; map keys are values, and a dot path implies field access, which a map slot is not. This is the standard LOG-wins reconciliation, but it is a program-visible syntax choice, so confirm the direction before editing the SPEC.

For execution: reconcile SPEC §9.5.12 (dot `<binding>.<key>`) to the LOG bracket form `<binding>['<key>']` per LOG 012-111, 012-120. Finding F019.

#### I-48 — The repeat comprehension `[for N: v]` duplicates `v` N times but pins neither a Clone bound, an evaluation count, nor the N=0 case

Background: `[for N: v]` produces an N-element array of `v`. The corpus's ownership rules require an explicit `.clone()` at every duplication site — but this form is a duplication site with no explicit clone, and three things are left open. The SPEC's own examples show the tension: `[for 256: 0]` duplicates a `Copy` zero (fine), while `[for n: origin]` duplicates a `Point`, whose Clone status is never stated.

The question: what does `[for N: v]` require and how many times does it evaluate `v`?

Three unpinned points:

```
[for 3: origin]       // (1) must Point satisfy Clone? if v is owned non-Clone, N copies are impossible
[for 3: next_id()]    // (2) evaluate next_id() once then clone, or call it 3 times? side-effect-visible
[for 0: v]            // (3) is v evaluated at all when N is 0?
```

Options for evaluation semantics:

- Evaluate `v` exactly once, then require `T: Clone` and clone it N−1 times; N=0 evaluates `v` never (or, alternatively, once-and-discard for its type). Matches the array-of-identical-values mental model and keeps the explicit-clone rule honest by making the form require Clone.
- Evaluate `v` N times (re-run the expression each slot). Lets `[for 3: next_id()]` produce three distinct ids and needs no Clone bound, but breaks the "N copies of v" wording, which says copies, not fresh evaluations.

Recommended: evaluate `v` once and clone N−1 times, require `T: Clone` (`Copy` types clone trivially), and evaluate `v` zero times when N=0, because the decision's own words say "N copies of v" — copies, not N independent evaluations — and the copy model is what reconciles this form with the explicit-clone-per-duplication-site rule instead of contradicting it. Programs wanting N fresh side-effecting values should use the binding form `[for i in 0..N: next_id()]`, which already exists for exactly that.

For execution: LOG 012-82 / SPEC §9.3.1 — state `T: Clone` required, `v` evaluated once then cloned N−1 times, N=0 evaluates `v` zero times; reconcile with the explicit-`.clone()`-per-site rule at SPEC §11 (line ~8738). Finding F074.

#### I-49 — Value recurrents and reload: does history survive a body edit?

A recurrent accumulates history. `recurrent[8] f32 = avg(.past)` remembers its last 8 committed values, and you can read them through `.past` and `.previous`.

Hot reload keeps a cell alive across a code change when its path and type still match. The reload rules spell out what happens to a *derived* whose body text changed (recompute from current inputs) and to *recurrent-streams* (their own capacity rules). But for a plain value recurrent at module or node scope, nothing says what happens to its accumulated history when you edit its expression body.

The question: you ship a new build where `recurrent[8] f32 = avg(.past)` becomes `recurrent[8] f32 = max(.past)`. Same path, same type, changed body. Does the 8-slot history buffer carry over, or reset empty?

Option A — preserve. The cell is the same cell (same path, same type), so keep its history; the next commit reads the old `.past` under the new formula.

Option B — reset. A changed body is new behavior with a new content-addressed id, so wipe the history and start accumulating fresh.

Recommended: **Option A (preserve)**, because the reload identity rule already says a same-path-same-type cell *is* the same cell and its value is preserved, and history is part of that cell's state. Wiping it would make a recurrent's history quietly vanish on any formula tweak, which is the surprising outcome. This parallels how a derived keeps its value when its body is unchanged; the only added claim is that a recurrent's buffer is preserved by identity, not by body-text sameness. Either way the rule must be written down; today it is inferable at best.

For execution: F109. Add rule under LOG §028 (near 028-21/028-22) and SPEC §13.15.3 step 8 (or §13.2.4): a value `recurrent[N]` at module/node scope keeps its accumulated `.past`/`.previous` history across reload when path and type match, even if the expression body changed. Reconcile with 028-6 (same-path-same-type identity), 028-30 (initial-text change forces operator restart), and the recurrent-stream rules at §13.18.14.

#### I-50 — Reload and late reconciler writebacks: what happens to a write aimed at a torn-down instance?

A reconciler hook is not allowed to block, so long work goes to a worker thread and writes its result back later through the runtime interface. Reload pauses host writes and lets the in-flight commit drain — but a worker-thread writeback is neither of those. It arrives after reload, through the write channel, aimed at an effect instance.

Worse case: that effect got a per-instance restart during reload (its param or cell type changed), so the host received a teardown for the old instance. Now a stale writeback lands, keyed to an instance id that no longer exists.

The question: what does the reload sequence do with an outstanding async reconciler writeback, and specifically with one aimed at an instance reload already tore down?

Option A — quiesce. Await outstanding reconciler work before reload proceeds, so no stale writeback can land. Clean, but reload now waits on arbitrary host worker threads it does not control.

Option B — queue and apply. Treat the writeback like any paused host write and apply it to the new state. Simple, but a write keyed to a torn-down instance id has no valid target; it would either error or misapply to the fresh instance sharing that path.

Option C — accept but validate. Queue writebacks, then reject any whose target instance id was torn down; apply the rest. Late writes to surviving instances still land; stale ones are dropped with a defined outcome.

Recommended: **Option C**, because the corpus already queues host requests during reload and applies them to the new state, so blanket-quiescing (A) contradicts that model, and blindly applying (B) is the actual soundness hole — the write-signal path today does no validity check on a torn-down instance id. A defined rejection for stale-instance writes closes the gap without making reload block on host threads. This is an engineering call, not a corpus-settled one; flag it.

For execution: F110. Specify in SPEC §13.15.3 (and a LOG §028 entry near 028-15/028-16/028-38) how reload treats an outstanding async reconciler writeback: queue like a host write, then reject any `write_signal`/`push_stream` whose target instance id was torn down by per-instance restart (028-38); apply the rest to new state. Cross-ref 027-45/027-46 (reconciler writes are observed-cell writes via write_signal on the driving context) and 028-24 (queued writes applied to new state).

#### I-51 — A trap during reload-time first evaluation has no rule

The rule that a trap during first evaluation (signal initializer, first recurrent commit, initial derived evaluation) aborts the process is bound to startup, but hot reload adds cells whose first evaluation runs outside startup, and no reload rule says what a trap there does. Recommended: broaden the abort rule to cover reload-time first evaluations too, because reload runs the identical first-recurrent-commit / initial-derived-evaluation step under the reload lock, and leaving its trap fate unstated forces the implementer to guess between abort and rolling back the reload. Owner decides whether to widen the existing rule or add a distinct reload-time rule (a reload rollback is defensible since a mid-reload trap is not a fatal startup); flag the choice.

For execution: F262. Either broaden 016-153 (§13.2.6) to name reload-time first evaluations, or add a reload-time trap rule under LOG §028 / SPEC §13.15.3 (reload step 5). Cross-ref 028-8 (added cell allocated+initialized), 028-18, 028-3 (compile-failure rejection only). Shares root gap with F109's sibling primary finding.

#### I-52 — `Map + Map` merge order: new right keys "in left order" vs "in right order"

Two LOG entries restate the same `Map + Map` merge and disagree on where new right-operand keys land: one says the merged-in right keys go 'in left order', the other says 'in right order'. Since Map iteration is insertion-ordered and observable through `for`, Display, Debug, and `repeat`, this is behavior-changing. Recommended: fix the 'in left order' entry to say the new right keys are appended in the right operand's order, because the SPEC merge section and the other LOG entry both already say 'in the right operand's order' — the 'left order' phrasing is the lone drifted restatement. Surface the edit rather than applying it unilaterally.

For execution: P001. Correct 012-106 ('in left order') to 'in the right operand's order' to match 012-121 and SPEC §9.5 (SPEC.md:7371-7375). Keep the update-keeps-position clause (collision key holds its left position, takes right value) — that part already agrees.

#### I-53 — How many structural carve-outs does the nominal type system have?

Ductus is nominally typed with a few named exceptions — types compared by shape instead of by name. Four LOG entries give three different counts of what those exceptions are.

- One says exactly two: `Tuples and closure types are the only two structural-typing carve-outs`.
- One says two a different way: closure types are `the second structural carve-out after tuples`.
- One says three: `Tuples, closure types, and trait-constraint intersections are explicit structural exceptions`.
- One adds a fourth form: `Map[K, V] is structurally typed by K and V; nominal Map types do not exist`.

The 'only two' entry is a closed count, and the other two entries falsify it. A reader building the typing rule from 'only two' would wrongly conclude trait-constraint intersections and Map are nominal.

The question is which forms are truly structural, then make every entry state the same set. This is not pure wording drift — the entries disagree on substance, so the owner must pick the authoritative set.

Recommended: **treat all four as structural (tuples, closure types, trait-constraint intersections, Map[K,V]) and drop every 'only two' / 'second carve-out' closed count**, because each of the extra two is independently asserted structural for a concrete reason — a trait-constraint intersection has no nominal declaration, and Map is explicitly shape-typed with no nominal Map type existing. Nothing in the corpus argues those two are nominal; only the stale closed counts imply it. If the owner instead wants a narrower canonical set, that has to be a deliberate call, since it would make Map or intersections nominal — flag it.

For execution: P011. Reconcile 001-30 (§1.3, =3), 012-46 (§9.2, 'only two'), 013-187 (§11.10.6, 'second after tuples'), 012-100 (§9.5, Map structural). Pick the authoritative closed set; restate all four to it and align SPEC §1.3, §9.2, §11.10.6, §9.5. User decides the set; do not resolve unilaterally.

#### I-54 — Is string backing-storage sharing optional or mandatory?

Strings are Copy — `let t = s` gives you an independent copy. The question is whether the runtime is *required* to share the underlying bytes behind the scenes.

Three entries disagree:

- `The runtime may share immutable backing storage between string values; sharing is an implementation detail invisible to the user.` (optional, invisible)
- `string realizes its Copy semantics through refcounted shared backing ... without copying bytes.` (mandatory mechanism)
- `String duplication is constant-time regardless of string length.` (observable O(1) guarantee)

These collide. Constant-time duplication of an arbitrary-length string is only possible if the bytes are shared. So a conforming implementation that took 'may share' literally and copied bytes would break the O(1) guarantee. The same sharing is called optional-and-invisible in one place and mandatory-and-observable in another.

The question: is shared backing a required, user-observable guarantee, or an optional optimization?

Option A — make sharing normative. Drop the 'may share' / 'invisible' framing; refcounted shared backing is the mechanism, and O(1) duplication is a real guarantee users can rely on.

Option B — keep sharing optional. Then weaken the duplication claim: duplication is O(1) *when* the implementation shares backing, not unconditionally.

Recommended: **Option A (sharing is normative and observable)**, because two of the three entries already commit to it concretely — refcounted shared backing as the Copy mechanism and constant-time duplication regardless of length — and only the oldest entry hedges. Weakening the guarantee (B) would remove a performance property the language otherwise promises. The fix is to align the hedging entry to the guarantee, not the reverse. User decides whether the O(1) guarantee is intended to be binding; flag it.

For execution: P012. Reconcile 012-37 (§9.1.8, 'may share'/'invisible') with 013-102 (§11.6, refcounted shared backing) and 013-104 (§11.6, O(1) duplication). Recommended: drop 012-37's 'may'/'invisible' so shared backing is normative; align SPEC §9.1.8 and §11.6. User decides if sharing is mandatory-observable.

#### I-55 — Are enum-variant imports an exception to the absolute-path rule?

Every `use` path is supposed to be absolute, starting from a path base. A path base is `root`, a manifest dependency name, or `std` — never a bare type name.

But the variant-import forms are written rooted at a bare type name:

```
use Result::(Ok, Err)
use Direction::*
```

`Result` and `Direction` are type names, not path bases. Under the absolute-path rule these would have to be fully rooted, like `use root::...::Result::(Ok, Err)`. So an implementer cannot tell whether `use Result::(Ok, Err)` is legal as written or must be rooted. Nothing carves variant imports out of the rule.

The question: is a bare-type-rooted variant `use` a real exception, or must variant imports be fully rooted like every other path?

Option A — carve out an exception. `Type::(...)` and `Type::*` are a special variant-import form that names an in-scope type, not a path base. The short forms stay legal; the absolute-path rule gets an explicit exception.

Option B — require full rooting. Variant imports obey the same rule as everything else; the examples must be rewritten as `use root::...::Result::(Ok, Err)`.

Recommended: **Option A (explicit exception)**, because the short forms `use Result::(Ok, Err)` and `use Direction::*` appear repeatedly as the intended surface syntax, and forcing full rooting would make routine variant imports verbose for no stated benefit. But this is genuinely underdetermined — the absolute-path rule is stated with an unqualified 'all' and 'no relative form', so the owner has to decide the variant form is a deliberate exception rather than let the two rules silently conflict. Flag it.

For execution: P015. Reconcile 003-24 (§10.2.3, all use paths absolute from a path base) and SPEC §10.4 (SPEC.md:7742-7744) with 009-70/71/72 (§6.2.3, bare-type-rooted variant imports) and SPEC §6.2.3 (SPEC.md:5222-5224). Path bases per 003-22 = root/dependency-name/std. Recommended: declare variant/enum-rooted `use` an explicit exception in 003-24 and SPEC §10.4/§10.2.3. User picks exception vs full-rooting; do not self-decide.

#### I-56 — What is the exact host write surface — two channels, and which two?

The host writes into the reactive graph through a fixed set of channels. Two LOG entries both give exhaustive lists, and the lists differ:

- `Host-driven writes occur only through runtime.write_signal and runtime.transaction.`
- `The host-program write surface comprises exactly two channels: runtime.write_signal ... and runtime.push_stream ... There is no third channel.`

They conflict two ways. First, `push_stream` is genuinely a host write channel — stream cells are host-written through it — so the first entry's 'only', which omits it, is wrong. Second, `transaction` is not a peer write channel; the SPEC shows it as a batching wrapper that contains `tx.write_signal` calls. The SPEC's own signal section scopes its `{write_signal, transaction}` list with the qualifier 'for signal cells' — a scope the first entry dropped, turning a signal-scoped list into an unqualified 'only' that collides with the whole-surface enumeration.

The question: is `transaction` a write channel or a batching wrapper, and does the canonical two-channel surface name `push_stream`?

Option A — align to {write_signal, push_stream}. Two channels: signals via write_signal, streams via push_stream; transaction is a batching wrapper around write_signal, not a channel.

Option B — keep {write_signal, transaction} but re-scope it. Restore the dropped 'for signal cells' qualifier so the first entry stops reading as the whole surface, and let the stream channel be stated separately.

Recommended: **Option A**, because push_stream demonstrably writes stream cells (the stream write path is defined in the runtime interface), so any exhaustive surface list that omits it is factually incomplete, and transaction is shown as a wrapper that issues write_signal calls rather than a distinct value-write channel. Rewriting the first entry to the {write_signal, push_stream} surface, with transaction reclassified as a batcher, makes both entries state the same two channels. User confirms which entry carries the intended scope before amending.

For execution: P016. Reconcile 016-157 (§13.2.7, 'only write_signal + transaction') with 016-285 (§13.14.2, 'exactly two: write_signal + push_stream, no third'). SPEC §13.2.7 (SPEC.md:12355-12362) scopes its list 'for signal cells'; §13.14.5 (19006-19011) shows transaction wrapping tx.write_signal; §13.14.8 (19111-19145) defines push_stream as the stream write path. Recommended: rewrite 016-157 to the {write_signal, push_stream} surface, transaction = batching wrapper. User decides which entry states intended scope.

#### I-57 — Structural self-emission (a node exposing its own type) never terminates and nothing rejects it

**Background.** A node can place child nodes. Some children the caller supplies; others the node's own `expose:` block emits at instantiation — including a compile-time `for` loop that stamps out children. The spec says node trees always terminate because "each placement is an explicit user act" — the compiler walks a finite tree the caller built.

**The problem.** That termination argument only covers children the *caller* hands in. It does not cover a node that emits *its own type*. Nothing stops this:

```
node T:
  expose:
    T          // every T contains a T contains a T ...
```

or the loop form:

```
node T:
  expose:
    for i in 0..1: T
```

Both are well-formed under every rule that exists. The type-emitted-`for` rule only requires the *iterable* be compile-time-known; it never constrains the emitted element *type*. Materializing one of these puts a `T` inside every `T`, so elaboration never terminates. There is no acyclicity or self-emission guard anywhere in either document, and the language says the only legal ways to stop are delegating to the standard library or declaring behavior implementation-defined — neither is declared here. So an implementer literally cannot decide whether to reject this, and a naive compiler hangs.

**The question.** How should structural (containment) self-recursion be handled?

**Options.**

- **Reject it.** Make a node type that emits its own type — directly or through a chain of other node types — a compile error with a named diagnostic class. Analogous to Rust rejecting an infinitely-sized recursive struct without indirection.
- **Scope the termination claim.** Leave self-emission legal in principle but narrow the "placements always terminate" claim to caller-supplied recursion only, and separately declare what structural self-recursion does (e.g. implementation-defined depth limit). This keeps a door open for indirection-guarded recursion but needs a real mechanism, which the corpus does not have.

**Recommended: reject it — make structural self-emission a compile error of a named diagnostic class.** The corpus offers no indirection primitive (no boxing/handle-placement that would bound the recursion), so option two would require inventing one; and the whole point of the finite-closure carve-out that makes compile-time interpretation safe is that it *presupposes* a finite containment closure. Rejecting self-emission is what makes that presupposition true. The reject rule must catch transitive cycles (T emits U emits T), not just direct self-emission.

For execution: F083. LOG 017-27, 017-54, 017-56, 017-63; SPEC §13.3.3 (self-recursive-placements-terminate, ~SPEC:13307-13309), §13.3.3.3 (type-emitted-for), §2.3.3, and the finite-closure carve-out at SPEC:520-527. Add a containment-acyclicity rule + diagnostic class; scope the termination claim to acceptance recursion.

#### I-58 — Do `derived` and value-`recurrent` parse in `observed:` as computed outputs

**Background.** An effect has two blocks: `desired:` (what the effect wants the world to be, driven by expressions) and `observed:` (what the host reports back). Interior effects can also compute their own outputs and lift them into `observed:` without the host writing them.

**The problem.** The corpus accepts and rejects the same declaration. One set of rules says `observed:` may hold only `signal` and `stream` — host-written cells — and that `recurrent` is allowed only in `desired:`. Another set says `observed:` may *also* hold `derived` and value-`recurrent` cells as computed outputs. Both the log and the spec contradict themselves, section to section:

```
effect Foo:
  observed:
    recurrent x = ...   // one rule: reject (not signal/stream)
                        // other rule: accept (computed output)
```

A parser cannot both parse this and diagnose it.

**The question.** Do `derived` and value-`recurrent` cells parse in `observed:` as computed outputs, or not?

**Options.**

- **Widen the restrictive side.** Admit `derived` and value-`recurrent` in `observed:` as computed outputs; keep `recurrent[N] stream` forbidden there. Fix the two-keyword enumeration and its spec section.
- **Withdraw the computed-output feature.** Delete the computed-output rules and the spec section that admits them, leaving `observed:` host-written only.

**Recommended: widen the restrictive side.** Three log entries plus a whole spec section define computed outputs as a real feature — interior effects lifting child outputs into `observed:` with no host write. Only two survivor entries and one spec paragraph carry the two-keyword restriction. Withdrawing the feature deletes the interior-effect surface; the restriction is the stale side. Keep the one genuine carve-out both sides already share: `recurrent[N] stream` stays forbidden in `observed:`.

For execution: F088. LOG 031-14, 031-15 (restrictive) vs 031-49, 031-146, 031-152 (admit computed outputs). SPEC §13.19.2 (SPEC:21934-21938, "no other declaration kinds permitted") vs §13.19.5 (SPEC:22244-22246). Widen 031-14/031-15 and §13.19.2 to admit `derived` + value-`recurrent` computed outputs; also fix the diagnostic hint at SPEC:22870-22874. Distinct from master key K-OBSERVED-REQCELLS (that governs waiver/auto-satisfaction, not which kinds parse).

#### I-59 — A dropped repeat scope's cursor hold on an outer gate stream is never released

**Background.** A gate stream buffers events and holds each one until every consumer has read it (slowest-cursor retention). A frozen consumer pins the buffer — producers get rejected until it catches up. A `repeat` over a Map creates one scope per key; when a key is removed, that scope is dropped (`scope_drop`).

**The problem.** Put those together: a node inside a repeat body consumes a gate stream that lives *outside* the repeat (a shared upstream buffer). Its cursor pins that shared buffer. Now the Map key is removed and the scope is dropped. Nothing says what happens to that cursor's buffer hold. Section 030 defines cursor lifecycle for exactly three cases — normal advance, gate freeze/resume, and code reload — but not for a consumer killed by `scope_drop`. Two readings diverge:

```
// key removed -> scope dropped
// reading (a): cursor removed from retention set -> buffer released
// reading (b): hold persists -> shared buffer pinned forever, producers rejected
```

Reading (b) is a permanent back-pressure leak on a stream that outlives the dropped scope. The one release rule that exists (`@reset_on_reopen`) fires on a gate *reopen* edge — a dropped scope never reopens, so it cannot even be the remedy. The rule "a stream is freed when its scope dies" only covers a stream *declared* in the dying scope, not a consumer's hold on an outer stream.

**The question.** When a repeat scope is dropped, what happens to a cursor its consumer held on an *outer* gate stream — is the slowest-cursor hold released?

**Options.**

- **Release on scope_drop.** Treat `scope_drop` as removing the consumer's cursor from the outer stream's retention set, same as the reload rule already does for a removed consumer. No leak.
- **Leave undefined / pin forever.** Rejected on its face — a permanent buffer pin from a routine key removal is a soundness/liveness hole no user could work around.

**Recommended: release on scope_drop.** The reload path already drops a removed consumer's cursor; a runtime key removal is the same event class and should behave the same. The effect-teardown rule already tears down effects "with the element key" on scope drop — the cursor/hold release should ride the same teardown. Anything else permanently jams the shared stream.

For execution: F111. LOG 030-13, 030-188, 030-193, 030-196, 018-78, 018-8; also reload-scoped 030-221, 028-49. SPEC §13.18.12, §13.18.1, §13.5.4.2 (scope_drop, SPEC:15145), §13.5.1, effect-teardown 018-110. Add a rule (in §030) releasing the consumer's cursor + slowest-cursor buffer hold on scope_drop, distinct from reopenable gate-freeze.

#### I-60 — The wire-format identity contract never defines the repeat `.<key>.` path segment

**Background.** Every reactive cell has an identity — a fully-qualified path used for hot reload and for the on-wire cell ID. The identity section defines that path as lexical declared names, with one extra device for duplicate anonymous siblings: a positional ordinal `:N`. A `repeat` creates one scope per key, and the repeat rules say a per-key cell's path is `<enclosing>.<key>.<field>`.

**The problem.** The repeat rules route per-key cell identity *through* the identity section and say the keyed path "follows" the wire-format section. But those sections define only lexical names plus the `:N` ordinal — they never define a `.<key>.` path segment, and they declare themselves the complete mechanism ("the two are the same mechanism," "§15.4.1.1 specifies the wire format"). An implementer applying the wire-format section verbatim produces `:N`-ordinal IDs for repeat siblings, not key-based IDs:

```
// repeat over Map with keys "a","b"
// repeat rules want:   grid.a.frequency   grid.b.frequency
// wire-format section verbatim gives: grid:0.frequency  grid:1.frequency
```

Two careful readers get different on-wire IDs for the same cells. The verifier flagged a severity nuance: the repeat section does self-supply "the key value serves as the path component," and the `:N` rule is textually gated on "anonymous or duplicated" siblings — which keyed scopes are not — so an implementer *can* reconcile by treating keyed scopes as a specialization. That makes it a specification gap in the wire-format section rather than a strict unsatisfiable contradiction.

**The question.** Is `.<key>.` a first-class multiplicity mechanism in the identity/wire-format contract, and which section owns its definition?

**Options.**

- **Extend the wire-format section.** Define `.<key>.` as a multiplicity device alongside `:N`, state that repeat-materialized siblings use the key (not the ordinal), and specify how the key is wire-encoded. One section becomes the authority.
- **Stop routing repeat identity through that section.** Have the repeat rules own and fully define the keyed path themselves, and drop the "follows §15.4.1.1" claim so two sections no longer both claim to be the whole mechanism.

**Recommended: extend the wire-format section to define `.<key>.` as a first-class multiplicity device, and make it the single authority.** Cell IDs are a serialization contract — one section must own the complete wire format, or two implementations disagree on bytes. The repeat section already leans on that section; the fix is to make the section actually cover the keyed case (encoding included) rather than letting repeat self-supply a device the authority does not know about. Also downgrade the severity from a soundness hole to a specification gap per the verifier's finding.

For execution: F218 (re-grade HIGH->MED per charity check). LOG 018-25, 018-26, 028-4, 028-5. SPEC §13.5.3 (SPEC:15206-15221, self-supplies key-as-path-component ~15220), §13.15.2 (SPEC:19289-19300), §15.4.1.1 (SPEC:24213-24232). Add `.<key>.` to §15.4.1.1 as a multiplicity mechanism with wire encoding; state keyed scopes use key not `:N`.

#### I-61 — The gate-open snap has no legal ordering inside the frozen commit DAG

**Background.** A gate freezes a subtree while its `when` predicate is false. When the predicate flips false->true, the frozen deriveds must "snap" — re-evaluate against current upstream state. The rules say this snap happens *in the same commit* that flips the predicate. Meanwhile the commit engine has a fixed shape: step 1 computes the dirty set and no new dirty bits may be added after that; step 2 topologically sorts a per-commit DAG built from exactly the dirty deriveds plus dirtied recurrents.

**The problem.** While a subtree is gated off, its input writes are suppressed — gated edges contribute no dirty propagation to those output-affecting deriveds. So when the gate opens, the frozen subtree's deriveds carry no dirty bit this commit (unless their inputs happened to change now too). That means step 1 does not mark them, step 2's DAG does not contain them, and the "no new dirty bits after step 1" rule forbids adding them mid-commit. Yet the snap rule demands they re-evaluate in topological order *this* commit. The gated-off subtree is even explicitly excluded from the DAG. There is no rule authorizing a mid-commit expansion of the sorted DAG to pull in the newly-unfrozen subtree, and no implementation-defined escape is declared. The implementer has no legal ordering to run the snap.

**The question.** How does the false->true gate flip get the newly-unfrozen deriveds evaluated?

**Options.**

- **Inject into the current commit.** Make the flip add the gated subtree's output-affecting deriveds to *this* commit's evaluation set, and specify their position in the topological order relative to the flipping predicate (which is itself a derived, known only after it evaluates) and to recurrent advance.
- **Relax the DAG-freeze rules.** Explicitly permit gate-transition-induced nodes as a sanctioned exception to "no new dirty bits after step 1."
- **Move the snap to the next commit.** Schedule the re-evaluation for the following commit — matching how reload predicates and connection re-points already defer — and rewrite the same-commit rule.

**Recommended: inject into the current commit (option one), with an explicit gate-transition carve-out to the DAG-freeze rules.** This keeps the snap in the same commit, which is what the effect-hook ordering resolution already depends on. That resolution pins the effect's `resume` hook to fire before its first `update` *at a single commit boundary* — and it can only do that because the gate-open snap dirties the effect's desired cells inside the flipping commit, making resume and update co-eligible at one boundary. Deferring the snap to the next commit removes that premise: the desired cells would go dirty a commit later, `update` would fire a boundary after `resume`, and the pinned same-boundary rule would address a scenario that no longer happens — plus gate-open reconciliation would silently gain a commit of latency. Same-commit injection preserves the premise. Its cost is that you must write a full mid-commit-injection ordering spec: where the injected deriveds sit relative to the flipping predicate (known only after it evaluates in step 3) and relative to recurrent advance. That spec is the price of same-commit consistency, and it is bounded — the carve-out admits exactly the gated subtree's output-affecting deriveds.

**If the user picks next-commit instead.** The effect-hook ordering resolution must be restated to match: `resume` fires at the flip boundary, the first `update` at the next boundary. Ship the two edits together — the gate-snap timing and the hook-ordering text are one decision, not two.

For execution: F064, F249 (resume-before-update depends on same-commit desired-cell snap), K-HOOK-ORDER. LOG 022-58, 022-59 (same-commit mandate), 023-12, 023-13, 023-14 (DAG freeze), 022-74 (gated edges no dirty prop), 022-12 (predicate is derived), 031-108 (resume hook), 031-124/125 (update enumeration). SPEC §13.9.7 (SPEC:17718-17724), §13.9.8 (gated subtree excluded, ~17826), §13.10.2 (SPEC:18138-18148), §13.11.3, §13.19.14 (hook firing). Direction: add the gate-transition carve-out to 023-12/023-14 and its ordering spec to §13.10.2/§13.11.3; keep 022-59 same-commit, preserving K-HOOK-ORDER's F249 premise. If next-commit is chosen instead: rewrite 022-59, reconcile 022-58, AND restate F249's resume/update ordering (resume at flip boundary, update at next) — ship together.

#### I-62 — A shift count at or beyond the operand's bit width is undefined

**Background.** Shift operators `<<` and `>>` dispatch through the `Shl`/`Shr` traits and produce the left operand's type. The shift *count*'s type is pinned (unsigned, up to u32). Every arithmetic operation is promised four overflow variants (default/wrapping/saturating/checked).

**The problem.** Nothing defines what a shift does when the count is at or beyond the operand's bit width — `1u8 << 8`, or `1u8 << 300`. That is the classic trap/mask/UB decision every low-level language must make, and shifts have *none* of the four overflow variants. The count *type* is pinned; the count *value* being out of range is unaddressed. A grep for shift + overflow/trap/mask/width/implementation-defined finds nothing. No legal boundary (std delegation or implementation-defined) is declared, so the implementer is blocked:

```
1u8 << 8    // trap? mask count to 0 (8 mod 8)? produce 0? compile error?
```

**The question.** What does a shift by count >= bit-width do?

**Options.**

- **Mask the count modulo width.** `1u8 << 8` uses `8 mod 8 == 0`. Matches x86 hardware and Rust's wrapping-shift family. Cheap, never traps, but surprising for `<< 300`.
- **Trap.** Out-of-range count traps at runtime (and is a compile error when constant-known), consistent with the language's default-traps-on-overflow stance for arithmetic.
- **Declare implementation-defined.** Explicitly punt, naming it an implementation-defined boundary.

**Recommended: trap on out-of-range shift count (and reject it at compile time when the count is a known constant).** The language already makes the *default* arithmetic operator trap on overflow rather than silently produce a masked/wrapped result; an out-of-range shift is the same class of programmer error and should fail loudly by default, not silently mask. That leaves room to add wrapping/checked shift variants later that mask or return Option, mirroring the existing overflow-variant scheme the shift operators are currently missing. Masking-by-default would be the one place the language silently swallows an out-of-range operand, breaking its own trap-first stance.

For execution: F027. LOG 007-64, 007-72, 007-92, 007-116 (four-variant promise), 007-117 (trap-on-overflow), 007-193. SPEC §4.4.2 (SPEC:3206-3211), §4.4.6, §4.6; Shl/Shr defs ~SPEC:3892-3895. Add a rule fixing out-of-range shift-count behavior to trap (const -> compile error); consider defining shift overflow variants.

#### I-63 — The constant-overflow rule rejects wrapping/checked expressions whose defined results are in range

The wrapping operator `+%` is defined to always produce an in-range modular result (`200u8 +% 100u8` is `44u8`, `255u8 +% 1` is `0u8`), but the compile-time-constant-overflow rule declares that very expression a compile error "because 300 doesn't fit u8," applied "regardless of operator variant." The two are jointly unsatisfiable: an implementer cannot both compute the defined `44u8` and reject the program. The same over-reach breaks the checked operator `+?`, which returns `Some/None` and never overflows, yet would be rejected as overflow. The rule checks the *pre-wrap* mathematical value instead of the operator's *defined* result. Recommended: restrict the constant-overflow rule to the default (trapping) variant only — or restate it to check each operator's defined result against the type — so wrapping and checked constant expressions that yield in-range results are accepted, because those operators by definition never overflow.

For execution: F026. LOG 007-120 (wrapping = modular in-range), 007-132 (`+?` returns Option), 007-137 (const overflow = error "regardless of variant"). SPEC §4.6.2, §4.6.5 (SPEC:3584-3585, 3589-3592). Scope 007-137/§4.6.5 to the trapping variant or check the defined result, not the pre-wrap value.

#### I-64 — Blanket Copy-storability collides with the closure-in-cell ban

One rule says any `Copy` value "can be stored in cells... like any other `Copy` value" with no exception; another makes a closure `Copy`; a third categorically forbids a closure as the value type of a `signal`, `attr`, `recurrent`, or `derived` cell. Composed, the first two license storing a closure in a cell that the third forbids — jointly unsatisfiable as written. Notably the spec is already more careful: it scopes the "storable like any other Copy value" claim to the Handle/WeakHandle/Portal carriers, not all Copy values, so the log's blanket wording also diverges from the spec. Recommended: qualify the general storability rule so its "any other Copy value" does not blanket-cover closures — align it to the spec's narrower carrier-scoped wording — leaving the closure prohibition as the authoritative rule, because the closure ban is a deliberate specific rule and the general claim asserts a false universal the spec itself does not make.

For execution: F047. LOG 013-153 (blanket Copy-storable), 013-192 (closure is Copy), 013-199 + 025-55 (closure not a cell value type). SPEC §11.9.1 (SPEC:9281-9287, carrier-scoped), §11.10.6. Qualify 013-153 to exclude closure Copy values, matching §11.9.1's narrower scope.

#### I-65 — The general error section covers `derived` only; the reactive-error section covers more constructs

Ductus has two places that describe what happens when a reactive computation hits an error. One is the general error-handling section; the other is a later reactive-error section that landed with a recent amendment. They describe the same subject — traps propagating, and value-track errors flowing through Result/Option — but they cover different sets of constructs.

The question: the general section frames these rules for `derived` only. The later section covers a wider set. Should the general section stay narrow, or match the wider set?

What the general section says:

```
// derived-only framing
A trap inside a derived expression's computation propagates as a normal trap.
A derived whose expression is Result[T,E] or Option[T] produces a reactive value of that type.
```

What the later reactive-error section says:

```
// broader construct list
A derived OR recurrent expression that traps aborts the process;
the same holds for a fold form's by: combiner and a collect/yield member expression.
```

So the narrower entries are not wrong — a trap in a `derived` really does propagate. But a reader scanning the general section sees only `derived` and could think recurrent, fold, and collect/yield traps are unaddressed. The two surfaces disagree on coverage, which is exactly the kind of drift the edit protocol warns against.

Two ways to fix it:

Option A — broaden the general section to name the same construct list (derived + recurrent + fold `by:` + collect/yield). Keeps the rule visible in the general section but duplicates the construct list in two places, so both must be kept in sync forever.

Option B — make the general section defer to the reactive-error section and drop the derived-only wording. Single owner, no duplication, no drift risk.

Recommended: Option B (defer), because the reactive-error section already states the broader rule completely and correctly, and the general section's derived-only wording only exists to be a partial restatement. One owner removes the drift permanently instead of creating a second copy that has to be maintained. Under the master citation policy (repoint-by-default), the general section should point at the section that already elaborates the claim rather than re-owning it.

For execution: F025. Reconcile LOG 011-79/011-80 and SPEC §8.9 (lines ~6301-6304) with LOG 026-2/026-6 and SPEC §13.13.1/§13.13.2. Recommended: repoint §8.9 to §13.13 and drop the derived-only duplication in 011-79/011-80; if instead broadening, add recurrent + fold `by:` + collect/yield to both entries and §8.9.

#### I-66 — What `?` returns early from inside a reactive expression body

The `?` operator is defined as "return early from the enclosing function." But a recent amendment lets `?` appear inside a reactive expression body — a `derived` or `recurrent` whose declared type is Result or Option. A reactive body is not a function, so the definition never says what `?` returns early *from* there.

```
// the ? definition (function-shaped)
expr?  =>  match Try::branch(expr):
             Continue(v): v
             Break(f):    return Err(From::convert(f))  // "in a Result-returning FUNCTION"

// but this is now legal, and there is no function here:
derived total: i32? = compute()?   // ? returns early from... what?
```

The corpus does resolve this, just not in the error-handling rules: a reactive body compiles to a behavior — a pure function the runtime invokes. So the derived's declared Result/Option type plays the role of the function's return type. But an implementer reading only the error-handling entries can't see that; the bridge lives in a different section.

The question is only *where* to state the bridge, not what it is.

Option A — state it in the reactive-error rule: "a reactive expression body with declared type Result/Option acts as the Result/Option-returning function for `?`'s early-return."

Option B — add a cross-reference from the reactive-error rule to the entry that establishes body-as-behavior-function.

Recommended: Option A (state it inline in the reactive-error rule), because the body-as-function fact is load-bearing exactly at the `?`-in-body site and readers hit it there; a one-line statement removes the need to leave the section, and a bare cross-reference makes them chase it. The behavior is already settled — this only makes it visible where it is used.

For execution: F024. Anchor LOG 011-46 (§8.4.1), 026-7 (§13.13.2), 026-6; SPEC §8.4.1 (lines ~6138-6140), §13.13.2 (line ~18811). The body-as-behavior-function fact is in 033-145. Recommended: add to 026-7 (or 011-46) that a Result/Option-typed reactive body plays the function-return role for `?`.

#### I-67 — `byte_len` violates the `x_count` tally-naming rule it is cited under

The tally-naming rule says specialized tallies are spelled `x_count` and cites `byte_len` and `char_count` on `string` as examples — but `byte_len` uses `_len`, not `_count`, and its sibling `char_count` follows the pattern. The rule contradicts its own example, and `byte_len` is the only `_len` name in the corpus.

Recommended: rename `byte_len` to `byte_count`, because the rule it is cited under says specialized tallies are spelled `x_count`, its direct sibling on the same type is `char_count`, and every other prefixed tally (char_count, pending_count, event_count) uses `_count` — the lone `_len` is the outlier. (Alternative if the user prefers: keep `byte_len` and amend the rule to accept `_len` as a valid prefixed-tally spelling.)

For execution: F076. LOG 012-92 (§9.3.7), 012-29 (§9.1.6, `byte_len`), 012-30 (§9.1.6, `char_count`); SPEC §9.3.7 (lines ~6991-6995), §9.1. Recommended: rename `byte_len` -> `byte_count` in 012-29 and §9.1 mirror.

#### I-68 — A bare `own` parameter as placement-attr RHS: implicit consume vs required `move`

Two rules collide at one site: assigning a bare `own`-typed parameter as the value RHS of a placement attribute. The placement-specific rule says a value-typed RHS at a placement attr is category B — implicit consume, no `move`. The general rule says function parameters flowing into reactive declarations follow category-A `own`/`move` — source must write `move`. So valid source either omits or requires `move` at that exact site, depending on which rule wins.

Recommended: the placement-specific rule wins (category B, no `move`), and scope the general parameter rule to the argument/initializer positions it means, explicitly excluding the placement-attr value-RHS site. The more-specific rule governs, and the placement rule already type-directs the category B/C split at that assignment; the general rule's "like any other category A consumption" was never scoped to override it.

For execution: F048. LOG 013-248 (§11.14, placement category B/C), 013-249 (§11.14, params into reactive), 013-3 (§11.1, category B implicit consume); SPEC §11.14. Recommended: scope 013-249 to argument/initializer positions, exclude the placement-attr value-RHS site governed by 013-248/013-3.

#### I-69 — Bare `Index[Range]` names an undefined trait instance

The built-in `Index` fulfillments entry writes the array/slice range-index instance as bare `Index[Range]`, while its sibling and the SPEC both write `Index[Range[isize]]`. Since the same sibling makes the key type part of trait identity, `Index[Range]` and `Index[Range[isize]]` are literally different trait instances — so the bare form is either an under-specified restatement or names an undefined instance.

Recommended: change the bare `Index[Range]` to `Index[Range[isize]]`, because the sibling entry makes `K` identity-bearing and both it and the SPEC already use the fully-parameterized spelling; the bare form has no defined instance behind it.

For execution: F039. LOG 007-230 (§4.9.5, uses bare `Index[Range]`), 007-229 (§4.9.5, `K` part of trait identity, uses `Index[Range[isize]]`); SPEC §4.9.5 (lines ~4200-4201, `Index[Range[isize]]`). Recommended: fix 007-230 to `Index[Range[isize]]`.

#### I-70 — A pairs-form match rule references an undefined `default:` arm

A pairs-form match rule says the match may "carry a `default:` arm" — but the match sections define the catch-all only as `_:` or a bare identifier. There is no `default:` arm anywhere in the language, so the term has no defined meaning at the point it is used.

Recommended: replace `default:` with the defined catch-all spelling (`_:` or a bare identifier), because that is what the match sections actually define and no separate `default:` construct exists. (If a distinct `default:` arm is genuinely wanted, that is a new match construct the user must decide to define in §6.2.x — surface it.)

For execution: F180. LOG 019-46 (§13.6.1.3); SPEC §6.2.4/§6.2.5 (catch-all defined as `_:` / bare identifier, lines ~5314-5315, 5330-5331), pairs-form mirror line ~15996. Recommended: replace `default:` with `_:` in 019-46 and the SPEC mirror.

#### I-71 — A move-position rule is misplaced inside the Copy-trait run

> **SETTLED (2026-07-08, no decision remains):** the grammar-session work merged from `main` (commit f89c179) removed the misplaced rule from the Copy-trait run and replaced it with new entries in the move-rules run (now 013-141/013-142). This item is closed; do not execute.

A move-position rule (§11.8.5, about the `move` keyword's l-value operand) sits wedged between two Copy-trait entries (§11.4). The log orders entries by topic, so this interrupts the Copy-trait run — a reader scanning the Copy block hits an unrelated move-syntax rule. It is a placement smell from an insert, not a semantic bug.

Recommended: relocate the move-l-value rule to sit with the other §11.8.5 move-position entries, but surface it first rather than moving unilaterally, because the log uses dense positional numbering and any relocation renumbers surrounding ids — which the edit protocol forbids doing without the user.

For execution: F043. LOG 013-79 (§11.8.5 move rule, misplaced), between 013-78 (§11.4) and 013-80 (§11.4.1); other §11.8.5 move entries at 013-135..013-142. Recommended: relocate 013-79 into the §11.8.5 cluster; surface because relocation renumbers (Invariant 1).

#### I-72 — "Compile-time evaluable" vs "compile-time-known" are never equated

Two phrasings describe one concept: the compile-time criterion is written as "compile-time evaluable" in one entry (and one other) but as "compile-time-known" everywhere else, including the entry that gates `yield` under `if`/`match`. The criteria substantively match — both negate on signals and external input — but the two phrasings are never explicitly equated, so a reader has to assume they are synonyms.

Recommended: unify on "compile-time-known," because that is the phrasing used by the defining cluster and the load-bearing `yield` legality gate, and a single term removes the guess; the outlier "compile-time evaluable" appears in only two entries. (If the user prefers keeping both, add an explicit one-line equivalence statement instead.)

For execution: F009. LOG 001-28 (§1.3, "compile-time evaluable"), 004-100 (also "evaluable"; was 004-99 before the M-28 insert renumbered section 004), vs 034-7 (§13.20, yield gate), 004-64..004-92 (§2.4.1/2.4.2), 034-7 ("compile-time-known"). Recommended: reword 001-28/004-100 to "compile-time-known".

#### I-73 — The general glob-collision rule delegates to enum-variant-only material

The rule that two glob imports colliding on any name is a compile error is a general rule about any importable name. But the SPEC section that elaborates it hands the reader off to the section titled "Variant construction and resolution," whose collision discussion is framed entirely around enum variants (`use Direction::*` vs `use Heading::*`). So a reader chasing the general rule lands on narrower, enum-only material. The LOG-to-SPEC pointer itself is fine; the defect is one SPEC section over-narrowly delegating to another.

Recommended: give the module/use-path section a self-contained statement of the parentheses convention and the general glob-collision rule, instead of delegating to the enum-variant section, because the rule governs any importable name and enum variants are just one instance of it. (This is a repoint-style citation fix under the citation-policy master key.)

For execution: LOG 003-45, 003-47 (unchanged); rewrite SPEC §10.4.1 to state the parentheses convention and general glob-collision rule directly rather than "Per §6.2.3", or generalize §6.2.3's framing to name enum variants as one instance. Finding F006. Belongs under K-CITE-POLICY.

#### I-74 — Does `private` on an effect mean anything at the host boundary

Background: in Ductus, `private` on a declaration is a compile-time, reference-site check — you cannot name a private thing from another module. A leaf effect whose `observed:` block declares host-written channels needs a host reconciler, and the host registers that reconciler keyed by the effect's declared type name. That type name is carried verbatim in the IR as `effect_type_name`, and type erasure at the runtime boundary drops records/traits/generics but never the effect type name.

The question: does `private` on an effect mean anything at the host boundary? A module-private leaf effect is legal, and it still forces the host to see and register its identity string:

```
// module A
private effect internal_health_check():
  observed:
    signal status: Health = Health::Unknown
// host, outside every Ductus module:
runtime.register_reconciler("internal_health_check", ...)  // private name is host-visible
```

So `private` hides the effect from other Ductus modules but does not hide it from the host. The docs never say whether that is intended. It is not a contradiction — the cross-module ban and the host exposure are jointly satisfiable — but nothing states the boundary rule either way.

Options:
- (a) Affirm that visibility is a Ductus-source-only, reference-site property that deliberately does not survive lowering; a private effect's type name is host-observable by construction. Add a one-line note next to the private-effect-reachability rule so the reader is not misled.
- (b) Decide private effect identities must be hidden from the host, and add a name-mangling or omission mechanism in the IR.

Recommended: (a), affirm and document, because the whole IR data model encodes no visibility field, the host binding is already implementation-defined, and the requirement that a reconciler-needing effect surface its name is fully normative — there is no existing hook to hide it, so (b) would be new machinery for a boundary the language never claimed to police. This is a user decision; (b) is only worth it if effect-name secrecy is a real goal.

For execution: LOG 031-77 (module-private effects not reachable from other modules), 031-119 (host registers reconciler keyed by effect type name), 027-68/033-120 (`effect_type_name` = declared type name), 033-141 (type erasure keeps the effect type name), 003-6 (visibility is reference-site). SPEC §13.19.10, §13.19.14, §13.14.7, §15.4.1, §15.4.3. Fix: add a visibility-does-not-cross-the-host-boundary note at 031 §13.19.10 and/or 027 §13.14.7. Finding F120 (LOW, gap).

#### I-75 — Where an effect's reconciliation failure lands on the value track

Background: Ductus has a uniform two-track failure model — errors land on a value track or a trap track — and the reactive system is required to use that same model. Separately, every effect is declared to be a "reconciliation contract" with a failure mode, and a reconciler is forbidden from panicking (so failures cannot go on the trap track).

The question: where does an effect's reconciliation failure land on the value track? The docs route reconciler errors to the program through the effect's `observed:` cells, via an error signal:

```
effect fetch():
  observed:
    signal error: Option[E] = None   // failure lands here
```

But no rule requires an effect to declare such an error cell. The SPEC even softens it to "typically." So an effect that declares only a non-error observed signal can fail reconciliation with no rule-guaranteed landing site — not on the trap track (reconcilers can't panic) and not guaranteed on the value track. That breaks the promise that the two-track model applies uniformly to every effect.

Options:
- (a) State that reporting reconciliation failure via an observed cell is a convention the author opts into — the failure track is author-chosen, not language-guaranteed.
- (b) Make an observed error channel obligatory for every reconciler-registered effect, so the uniform two-track guarantee holds for all of them.

Recommended: (b), require an error channel for reconciler-registered effects, because the language commits in words to a uniform two-track model, and (a) would carve a permanent hole in it — an effect could silently have an unreportable failure mode. (b) makes the guarantee real; the cost is one mandatory `observed:` cell on leaf effects. If that mandate is judged too heavy, (a) is the honest fallback, but it must be stated, not left implicit. User decides.

For execution: LOG 031-129 (reconciler errors surface through observed cells), 031-158 (every effect is a reconciliation contract with a failure mode), 011-78 (reactive system uses the same two-track model), 031-130 (reconciler must not panic). SPEC §13.19.14 (currently "typically a signal error"). Fix: add a 031 rule making the observed error cell either author-opt-in-and-so-labeled, or mandatory for reconciler-registered effects. Finding F226 (LOW, gap).

#### I-76 — "Attrs are written only at placement time" vs the reactive-fed attr bridge

One attr rule says attrs are written only at placement time, stated as an absolute closed rule. Another says a placement attr whose RHS references reactive cells builds an implicit derived bridge so the attr reactively tracks those sources after placement — meaning its value changes on later commits, not only at placement. A charitable reading reconciles them (the bridge is bound once at placement; later updates are the bridge's, not new writes to the attr), which is why this is low, but the absolute "only at placement time" wording is stronger than the reactive-tracking rule supports and can mislead an implementer about attr update timing.

Recommended: qualify the "only at placement time" rule so it does not deny post-placement reactive tracking — distinguish the placement-time bind of the value/bridge from the ongoing reactive updates that flow through it — because the reactive-fed attr rule plainly makes an attr's observed value change after placement, and the two restatements must agree on when an attr's value changes.

For execution: LOG 016-15 ("Attrs are written only at placement time", §13.2.2) vs 021-16 (reactive-fed attr tracks sources via implicit derived bridge). Qualify 016-15. SPEC §13.2.2, §13.8.2.1. Finding P004 (LOW, contradiction).

#### I-77 — A trait import silently recaptures existing bare calls to a same-named free function

Background: Ductus promises methods and free functions are interchangeable forms of the same operation — `f(x)` and `x.f()` mean the same call. Separately, when a call name matches both a trait-impl method and an in-scope free function, the trait impl wins; the free function is shadowed and reachable only by path qualification.

The question: those two rules combine into a silent-capture footgun. A module defines a free function and calls it by bare name everywhere:

```
fn render(s: Song) -> Frames { ... }
render(song)   // calls the free function

use SomeRenderTrait   // brings a trait whose `render` method Song fulfills
render(song)   // now silently resolves to the TRAIT method instead
```

Adding one `use` redirects every existing bare `render(song)` / `song.render()` to a different operation, with no diagnostic. It is action-at-a-distance triggered by an import, and it tensions the "same operation" promise — the operation silently swapped. This is an intended consequence of the trait-wins priority rule, not a contradiction, but the docs do not flag the hazard. Notably, the language already rejects a different silent-shadow class (a body-member name shadowing a module-level value is a hard error), so permitting this one silently is inconsistent.

Options:
- (a) Leave the priority rule as is, but document the capture hazard at the trait-vs-free-function resolution rule so authors know an import can redirect bare calls.
- (b) Warn or error at the call site (or at import) when a trait-impl candidate collides with an unrelated same-named in-scope free function, forcing the author to disambiguate.

Recommended: (b), warn at the collision, because the language already treats one class of silent cross-boundary shadowing as an error, so silently redirecting existing bare calls on import is the inconsistent case; a diagnostic turns action-at-a-distance into an explicit choice. If (b) is judged too aggressive (it can fire on benign coincidental name matches), fall back to (a) and at minimum document the hazard — silence is the one option that should be off the table. Design call; user decides.

For execution: LOG 001-33 (methods/free functions interchangeable, same operation), 005-120 (resolution prioritizes trait impls over free functions), 005-124 (trait-impl candidate wins; free function shadowed to path-only), 005-121 (candidates collected for every reachable trait). SPEC §3.4.1 step 4. Contrast: SPEC 16293-16298 makes body-vs-module bare-name shadowing a hard error. Fix: add a warn/error at 005-124 / §3.4.1, or at minimum a documented hazard note. Finding P030 (LOW, design_smell).

#### I-78 — The 'nearest scope wins' rule and the body-vs-module ambiguity error resolve the same name oppositely

**Background.** Name resolution walks outward: local bindings, then the instance body scope, then the module top level. Two record entries state the rule unconditionally — a bare name binds to the nearest enclosing scope that declares it. A third entry says a name declared in *both* the body scope and the module top level is a compile error. A fourth limits that error to exactly the body-vs-module collision and lets local bindings shadow normally.

**The problem.** For a name declared in both body and module, the two rules give opposite answers:

```
signal gain: f32 = 1.0        // module level
node Channel:
  attr gain: f32 = 0.5        // member
  derived a: f32 = gain       // nearest-wins entries: the member (0.5)
                              // ambiguity entry: compile error
```

Only by also reading the fourth entry does a reader reconcile them; the nearest-wins entries do not self-limit, and record entries must stand alone. The spec's scope-chain prose has the same trap in one phrase: it says a bare module-level reference "not shadowed by a member" resolves to the module — but a member never shadows a module name; that exact case is the error. The corrective rewrite therefore hides a real wording choice: how to carve the collision out of the nearest-wins entries.

**Options.**
- Exception clause. Keep nearest-wins as the stated rule; append the carve-out to both entries.

```
// a bare name binds to the nearest enclosing scope that declares it,
// EXCEPT a body+module co-declaration, which is an ambiguity error
```

- Restate by tier. Replace nearest-wins with two rules: locals shadow everything normally; between body and module there is no "nearer" — the name must live in exactly one, else anchor or error.

```
// locals shadow normally; body vs module: unique, or here::/module::
```

**Recommended: the exception clause**, because the record's own reconciling entry is already exception-shaped ("the ambiguity error applies only to the body-vs-module collision"), and nearest-wins stays literally true in every other case — locals over body, locals over module, and any name declared in only one of body/module. Scoping the rule down to local-binding shadowing (the restatement) would leave the plain single-declaration cases formally ungoverned and rewrites two entries' core wording to fix one carved case. Either way, also fix the spec's "not shadowed by a member" phrase — silent shadowing across that boundary is exactly what the error forbids. Wording is user-visible teaching material, so confirm the carve-out phrasing with the user.

For execution: LOG 020-3, 020-10 (add the carve-out; carve per 020-17, 020-19); SPEC §13.7.1 (16179-16224; "not shadowed by a member" phrase ~16197-16199), §13.7.4 (16275-16309, stays as is). Finding F181. User confirms wording.

#### I-79 — An operator-internal cell type change is claimed by two reload rules with opposite remedies

**Background.** Hot reload classifies source edits by remedy. One record entry says operator-specific changes — signature changes and internal cell type changes — need a per-instance restart: the affected operator instances are recreated. The very next entry says "all other changes," explicitly naming cell type changes (handled as remove-old-cell + add-new-cell), are reload-safe and need no restart.

**The problem.** Change the type of a cell declared inside an operator body. Both entries facially claim it:

```
operator smooth(x: f32) -> f32:
  recurrent acc: f32 = 0.0      // edit: f32 -> f64
// per-instance entry: restart every smooth instance (all state reset)
// all-other entry: remove acc, add acc, instance keeps running
```

The "all other changes" entry never excludes operator-internal cells, and the cell-identity rule it cites covers them — a cell's identity path includes the instance name, so an operator-internal cell has a perfectly legal remove+add path. The word "internal" appears only in the per-instance entry. The two remedies are observably different: a restart resets all of the instance's internal state; remove+add resets one cell and keeps the rest running. The spec's reload-constraints section does pin it — "internal cell type changes within an operator body" sits under per-instance restart — but the record entries alone admit both readings, and nothing in them fixes the precedence. The author must pick which rule governs.

**Options.**
- Per-instance restart governs. Scope the "all other changes" entry to cells outside operator bodies; the operator entry owns internal cells.

```
// 'cell type changes (remove + add)' — outside operator bodies only;
// operator-internal cell type change => per-instance restart
```

- Remove+add governs everywhere. Every cell type change, operator-internal included, is reload-safe remove+add; delete the internal-cell clause from the operator entry and from the spec's constraints list.

```
// any cell type change = remove old + add new; never a restart
```

**Recommended: per-instance restart governs — scope the "all other changes" entry to exclude operator-internal cells**, because the spec's constraints section already lists operator-internal cell type changes under per-instance restart explicitly, and the whole operator-reload model is instance-granular: even changing the text of an internal cell's initial expression restarts the instance, so a full type change surviving as a surgical remove+add would be an odd carve-under. The other reading requires deleting normative spec text; this one only makes an already-stated split self-contained in the record. The two readings produce different runtime behavior for the same edit, so the user picks — do not resolve unilaterally.

For execution: LOG 032-169 (add "outside operator bodies" exclusion or explicit precedence vs 032-168), 032-168, 032-166, 032-163; SPEC §13.15.4 (19364-19407; operator-internal line ~19377 already states Option 1), §14.8.3 (23790-23819), §13.15.2 (19289-19321; path scheme covers operator-internal cells), §13.17.10 (operator reload rules). Finding F202. User decides which rule governs.

#### I-80 — Reload-unsafe changes 'fall into two classes,' but a third outcome — reject — exists

One record entry says reload-unsafe changes fall into exactly two classes: those requiring full-runtime restart and those needing only per-instance restart. Two entries later, the implementation may instead reject an unsafe change outright — the runtime keeps running the old version — and reject-versus-restart is implementation-defined. So an unsafe change's actual outcome is one of three, not two, and a reader taking "two classes" as the complete taxonomy is wrong. Reconciling the count has two distinct, equally legal framings:

- Two classes, scoped axis. Keep the count at two by saying what it counts: the classes classify the restart a change *would need if applied*; rejection is a separate, cross-cutting disposition either class may receive.

```
// class (property of the change):  full-restart | per-instance
// disposition (impl-defined):      reject | apply that restart
```

- Three outcomes. Fold reject into the taxonomy: an unsafe change's disposition is one of reject, full-runtime restart, per-instance restart.

```
// disposition: reject | full-restart | per-instance
```

**Recommended: the two-axis framing — keep "two classes" and scope it to the required-restart axis**, because reject is not a property of the change: it is implementation-defined, so no feature of a change selects it, and both spec sections already keep the axes apart — the constraints section says implementations "either reject the reload or schedule the appropriate restart" while "the runtime diagnoses which class of change occurred," and the reload chapter repeats the same split. Folding reject in would invent a third class that nothing about a change determines. Runtime behavior is identical under both framings — this is a taxonomy-wording call — but the audit flags both as legal, so surface it and let the user pick.

For execution: LOG 032-166 (reword: two classes = required restart scope; either may instead be rejected per 032-170/171), 032-170, 032-171; SPEC §14.8.3 (23799-23818), §13.15.4 closing paragraph (~19403-19407). Finding F203. User picks framing.

## Dismiss-or-pin (verifier splits)

#### DP-01 — Fold `else:` vs loop `else:` two-token collision

**What was claimed.** A fold's `else:` arm takes an expression, and a loop is an expression anywhere. So you could write a loop that carries its own `else:` clause as the fold arm, producing two `else:` tokens on effectively the same line — and nothing tells the parser which `else:` belongs to the loop and which is an illegal second fold arm.

**The counter-argument.** The charity verifier refuted this by showing the witness cannot be written at all. A runtime loop is a header plus an indented body block, and the rule for its `else:` says the clause is "written at the loop head's indentation, dedented from the body rather than nested inside it." That dedent is exactly the boundary that separates the loop's own `else:` from the fold arm's `else:`. The only *inline* bracketed `for` form is the compile-time array comprehension `[for i in 0..N: <expr>]`, and the spec states outright it has "**no** `if`-filter form" and no `else:` arm. So there is no inline loop-carrying-an-else construct to jam into a fold arm in the first place — wherever a loop-with-else legally appears, its indentation rule creates the disambiguating boundary. As the verifier put it, the selecting rule the finding claims is missing is the very rule the finding cites.

**Recommended: dismiss, because** on a fresh read the grammar has no one-line loop-with-else form. The runtime loop's `else:` must sit at loop-head indentation (a dedent boundary), and the only inline `for` (the comprehension) is explicitly else-less. The two-`else:` collision the finding needs cannot be constructed. Note: the genuine inline-`if`/`else:` collision is a separate, already-recorded finding.

#### DP-02 — Does `move` accept a field-access operand like `move self.handle`

> **SETTLED (2026-07-08, no decision remains):** the grammar-session work merged from `main` (commit f89c179) ruled it — the `move` operand is restricted to a bare identifier; a field-access path is a compile error (now 013-142). The finding this item carried (F046, plausible) is dismissed on that ruling. Do not execute.

**What was claimed.** Two log entries restrict `move`'s operand to a bare identifier and declare any `move` on a dotted expression a parse error. But another log entry and the spec both explicitly allow `move self.handle` — a partial move of a field. Build the parser from the two narrow entries and you reject `close(move self.handle)`, which the spec accepts.

**The counter-argument.** The charity verifier tried to dissolve this but *sustained* it. The spec's `move` grammar admits "a binding identifier or a field-access path rooted in an owned binding (`self.handle`, `rec.a.b`)" and shows `close(move self.handle)` as valid; it forbids only method-*call* operands, stating plainly "a field l-value `move v.field` IS allowed." The dedicated log entry agrees: the operand "may be a field-access l-value path rooted in an owned binding... not only a bare identifier," with only method calls forbidden. But the two narrow entries, read standalone as the log's atomicity rule demands, contradict that: one spells the operand as an "l-value identifier" (bare only), and the other says "`move` attached to a dotted expression is a parse error" — its example is a method call, but the rule *text* sweeps in `move self.handle`, which is also a move attached to a dotted expression. No text forces a narrow reading of "dotted expression" that carves out field l-values.

**Recommended: pin it, because** this is a real internal contradiction, not a wording nicety. Three entries govern the same operand grammar and two of them, read alone, reject a program the third and the spec accept. Because log entries must stand on their own, the fix is to widen the two narrow entries: state the `move` operand admits a field-access l-value path rooted in an owned binding, and narrow the "dotted expression is a parse error" rule to method-call (non-l-value) operands only.

#### DP-03 — Is the impl-visibility "same module" row a new acceptance gate

**What was claimed.** The log defines an impl's visibility as `min(trait, type)`, callable wherever both are visible. For a private trait in module A and a private type in module B of the same package, that min is private and the "where both visible" set is empty — so the log makes the impl legal but callable nowhere. The spec's visibility table adds a row reading "only if both declared in same module," which the finding reads as a *new acceptance gate* that rejects the cross-module case the log accepts.

**The counter-argument.** The charity verifier refuted this. The spec table's column is titled "Impl visibility," and every surrounding sentence frames each row as a *reachability outcome*, not an acceptance precondition: "An implementation is callable wherever both the trait and the type are visible" and "if a caller can't name both the trait and the type, the implementation is unreachable from that caller's site." So "only if both declared in same module" is just the reachability of private∩private under the min rule — a private trait is visible only in its module, a private type only in its module, so both are jointly visible only when co-located. That IS the log's model, not a new gate. And the orphan-rule section (also log-backed) explicitly calls the cross-module private/private fulfill "rare but valid" — accepted, dead-but-legal.

**Recommended: dismiss, because** on a fresh read the spec row states *where the impl is reachable*, not whether it is accepted, and the neighboring "valid" verdict for the cross-module case matches the log's legal-but-unreachable model exactly. The acceptance-gate reading the finding needs is not forced by any text. (If the wording of that one table cell is felt to invite the misreading, it could be softened to "reachable only where both are visible (i.e. same module)" — but that is cosmetic, not a divergence.)

#### DP-04 — Does reload regenerate every portal's slot generation stamp

**What was claimed.** One log entry says a portal survives hot reload only if its target slot's generation stamp is *preserved*, and a stamp mismatch resolves the portal to `None`. Another entry says on reload the runtime *regenerates* the slot stamp. If the stamp is reissued even for a surviving slot, then every portal's stored generation stops matching, so by the first rule every portal resolves to `None` after any reload — contradicting the promise that portals to surviving slots survive.

**The counter-argument.** The charity verifier refuted this on the exact wording. The regenerating entry reads in full: "relocation or removal invalidates the portal, and the runtime regenerates the slot stamp **at the new path**." The regeneration clause is grammatically coordinated with "relocation or removal" and scoped by "at the new path." A surviving, same-path slot has no new path, so regeneration does not apply to it — it keeps its generation and its portals preserve. That is exactly the first entry's rule, which increments the generation only on "slot relocation or removal." The spec's hot-reload paragraph agrees: survivors preserve, relocation/removal increments. The finding's premise — that the stamp is regenerated *unconditionally* at every reload — misreads the "at the new path" scoping.

**Recommended: dismiss, because** read as one sentence, the regeneration is conditioned on relocation/removal, not on reload in general, and a surviving slot keeps its stamp — no jointly-unsatisfiable pair exists. (Separately, the entry cites a spec section that does not actually elaborate portal reload; that broken pointer is a distinct confirmed finding and is where any repair effort belongs.)

#### DP-05 — Does `.exposition` return two entry kinds or five

**What was claimed.** The `.exposition` field's element type is defined as a CLOSED sum of exactly five entry kinds: Node, Connection, Bundle, DynamicView, Gated. But two other log entries restate what `.exposition` returns as only "node and connection entries" — enumerating two of the five. An implementer modeling the public element type from the restatement builds a 2-variant type; from the definition, a 5-variant type. The same drift is mirrored in the spec (one section says "node and connection entries alike," another says "closed sum of five entry kinds").

**The counter-argument.** The charity verifier's dissolving reading is that "node and connection" are broad categories and the five are fine variants — i.e. the restatement is loose shorthand in entries that are really about *placement ordering* (where a connection entry sits relative to node entries), not a full enumeration of the variant set. On that reading the closed-five definition is authoritative and the restatements do not contradict it.

**Recommended: pin it, because** the shorthand reading does not survive the log's atomicity rule. The entry that *defines what `.exposition` returns* names exactly "node and connection entries" — read standalone, as log entries must be, it yields a 2-variant list and hides Bundle/DynamicView/Gated from anyone walking the public field. That under-determines the exposed variant surface for external readers, which is behavior-relevant. The fix is cheap and one-directional: make the two restating entries (and the matching spec sentence) say the returned list carries all five entry kinds, or state explicitly why the public field is restricted to node/connection — do not leave two enumerations standing.

#### DP-06 — Does a behavior body itself read and write cells at the ABI boundary

**What was claimed.** Two sections restate what a behavior *is* at the ABI boundary and disagree on who touches cells. One entry says behaviors "read inputs from cells, compute, and write outputs." Another set says a behavior is "a function of its declared inputs only, with no ambient state and no I/O," that I/O is "never of behaviors," gives a pure `(params) -> <type>` signature, and puts the `input_cell_ids`/`output_cell_id` on the *runtime table* — i.e. the runtime feeds and drains cells, the behavior is a pure params-to-value function. An implementer building the ABI cannot tell which model holds.

**The counter-argument.** The charity verifier softened this to a wording drift: the "reads from cells / writes outputs" entry also self-labels behaviors "pure transformations," so its cell phrasing can be read as loose ABI-dataflow description (the runtime maps cells to/from the behavior) rather than a claim that the behavior *body* performs cell I/O.

**Recommended: pin it, because** the "pure" label does not dissolve a literal I/O attribution. One entry says I/O is "never of behaviors" — absolute — while the other says the behavior itself reads inputs *from cells* and writes outputs. Reading a reactive cell is exactly the ambient-state access the pure-behavior entries forbid; "pure transformation" and "reads from cells" cannot both hold of the behavior body. This is load-bearing at the ABI: glitch-freedom, determinism, and thread-safety all depend on the runtime (not the behavior) doing the cell I/O, which is what the signature and behavior-table entries encode. The fix is to restate the drifting entry so the behavior is a pure params-to-return-value function and the runtime performs the cell read and cell write — matching the table and the purity entries.

#### DP-07 — Circularity: does satisfying it do anything beyond opting into a cycle?

A connection type can carry a `Circularity` marker. There are two claims about what that marker does. One entry says: satisfying `Circularity` has *no effect beyond* letting the connection take part in a legal cycle — nothing else. Another entry, in a different section, says the compiler's wake-gate machinery settles a cycle by a one-commit-delay fixpoint *only when* a `Circularity` connection guards it. That second claim ties real resolution behavior to the marker. So the marker does more than opt-in.

The dismiss case (the charity argument): a nearby entry says *which* connection types satisfy `Circularity` is domain-defined, and the spec says the marker's *semantic effect* is domain-defined — typically a temporal break so a cycle can't loop instantly. Read that way, the fixpoint behavior is a property of the connection type's domain meaning, not an effect the marker itself confers, so 'no effect beyond opt-in' can stand.

That rescue does not fully hold. The load-bearing entry does not say 'domain-defined temporal break enables the fixpoint' — it says the fixpoint resolves *only when guarded by a Circularity connection*. It ties settlement to the marker, not to a separate domain property. So on the page, one entry says the marker's only job is cycle admission and another makes it a precondition of correct fixpoint resolution. A reader building the wake-gate resolver from the first entry would not know the marker gates settlement.

Recommended: pin it (soft), because the absolute 'no effect beyond opting into cycle participation' collides with an entry that makes the marker a precondition of fixpoint resolution; qualify the absolute one so it does not deny the temporal-break/fixpoint role. Note this overlaps a confirmed audit finding on the same entry — settle them together.

For execution: LOG 019-78 vs 024-26; SPEC §13.6.5, §13.11.5. Reconcile: qualify 019-78 ('no effect beyond cycle participation and the resulting temporal-break/fixpoint semantics') or soften 024-26's 'only when guarded by a Circularity connection'. Coordinate with audit F170 (anchors 024-26/024-20/022-37). Finding P009.

#### DP-08 — No specified way to call a closure stored in a field

Ductus lets you store a closure in a record field or a `Vec` slot (a `Vec[dyn fn(Event) -> ()]`). Now write `r.handler()` where `handler` is a closure-typed field. The spec has two rules that both claim this shape and disagree.

Rule one (field-vs-method disambiguation): the compiler tells a field read from a method call 'by the syntax following the dot,' and the method-call operator is 'a dot followed by a function name and call syntax.' `r.handler()` is exactly that shape, so it routes to method dispatch. Method dispatch then searches for a trait method or free function named `handler` on `r`'s type, and its final step is 'unknown-method error' if none exists. There is no step that checks whether `handler` is a closure-typed field.

```
r.handler()   // dispatch: look for method/free-fn `handler` on r -> not found -> error
```

Rule two (precedence): the precedence table makes `.` and `()` independent left-associative postfix operators, which parses the same text as read-the-field-then-call-the-closure.

```
r.handler()   // (r.handler)() : read field, then invoke the closure -> works
```

The only closure-invocation rule says a closure is 'invoked with ordinary call syntax on its binding' — a binding *name*, not a field or index result. So a closure sitting in a field or slot has no stated invocation form at all, and `r.handler()` has two specified parses that give opposite results (unknown-method error vs. successful call).

Recommended: pin it, because storing closures in fields/slots is explicitly permitted while no rule says how to call one, and the disambiguation rule's premise ('the syntax after the dot decides') is false for callable fields — `r.method()` and `r.closure_field()` are identical after the dot. State which parse wins (field-read-then-call, or require `(r.handler)(args)`) and give the invocation form for a closure held in a field/index slot.

For execution: LOG 009-21 (disambiguate by syntax after dot), 013-178 (closure invoked on its binding — binding only), 013-195/013-200 (closures in fields/Vec slots); SPEC §6.1.4, §3.4.1 (8-step dispatch, step 8 = unknown-method error), precedence tier 14. Finding P028 (evidence note: P028's own block mislabels a 013-200 quote at line 1600, which is 013-214; the substance survives on the other verified anchors). Add a callable-field rule to §6.1.4/§3.4.1 and extend 013-178 to field/index results.

#### DP-09 — Can a record field hold a `Type[C]` value?

Ductus has type values — `Type[Drivable]` and friends — that are compile-time data. Two adjacent spec sub-sections talk about where they can appear, and the finding reads them as fighting.

The 'value position only' section lists where a `Type[…]` can appear: attr, parameter, return value, let/const binding, generic value argument. Record field is not in that list. The 'storage' section, three entries later, says a type value 'may be stored in persistent slots (record fields, attrs, cells).' The finding treats the first list as a closed whitelist that excludes record fields, so a record field is both forbidden and allowed.

The dismiss case (the charity argument): the 'value position only' section is not enumerating a whitelist — it is drawing one axis, value position *versus bound position*. Its own words: `Type[…]` 'appears only in value position ... It never appears in bound position,' and 'in bound position ... no `Type[…]` is needed or permitted.' The em-dash list illustrates value positions; the contrast being drawn is value-vs-bound, not field-vs-not-field. A record field is uncontroversially a value position — it holds a compile-time value, not a generic bound. The 'storage' section is a separate axis (can this value be persisted?) and answers yes for record fields. The two are consistent.

On a fresh read the charity reading is forced by the section's own 'never in bound position' framing. Nothing says the list is exhaustive of value positions; treating it as a closed whitelist is the finding's move, not the text's.

Recommended: dismiss, because the 'value position only' section scopes 'only' to value-vs-bound and a record field is a value position — the storage section's explicit record-field permission is consistent, not contradictory. (If desired, a one-line clarification that the list is illustrative, not exhaustive, would remove the trap — but it is not a defect.)

For execution: LOG 008-62 (value-position list), 008-71 (storage in record fields/attrs/cells), 008-65 (existential desugar to generic param); SPEC §5.7.2 (value-vs-bound), §5.7.4 (storage). Finding P029 — verdict dismiss; the two passages are consistent under the value-vs-bound reading.

#### DP-10 — Is 'exactly three materialization boundaries' really exhaustive?

A reactive composite stays 'live' (its fields are cells that keep updating) across most of the language. One entry says it collapses to a plain concrete value at *exactly three* boundaries: non-reactive collection storage, FFI/host handoff, and serialization. Another entry says a plain `let` of a `with` result yields a concrete value by reading the reactive RHS cells at evaluation time. The finding calls that a fourth reactive-to-concrete path the 'exactly three' wrongly excludes.

The dismiss case (the charity argument): materialization means a *live* composite collapsing to concrete. A plain `let x = A with f: sig` was never a live composite — the spec says it 'produces a concrete value per the standard §6.1.5 semantics,' i.e. it is born concrete, `sig` read once at binding time. Nothing is being materialized because nothing was ever live. So the plain-let-of-with path is outside the enumeration by construction, not a missing fourth member.

That reading holds on the page: the spec text ties the plain-let case to ordinary let semantics, not to the materialization-boundary list. The only real friction is the word 'exactly,' which invites a reader to treat the three as covering every reactive-to-concrete transition and be surprised that a plain `let` of a `with` also produces concrete.

Recommended: dismiss, because the plain-let-of-with result is born concrete and was never a live composite, so it is not a materialization boundary — the 'exactly three' set is about live composites collapsing. Optionally scope the word 'exactly' to already-live composites so the wording does not overclaim; that is a wording nicety, not a contradiction.

For execution: LOG 016-209 ('exactly three' boundaries), 016-217 (plain-let-of-with born concrete), 016-205/016-216; SPEC §13.2.9.7, §13.2.9.8. Finding P010 — verdict dismiss; optional: scope 016-209/§13.2.9.7 'exactly three' to already-live composites.

#### DP-11 — Does the record `@derive` set of six exclude `Copy` and `Circularity`?

One entry says `@derive` on a record 'generates structural implementations for a fixed trait set' of six (Eq, Ord, Hash, Clone, Display, Debug); other entries make `@derive(Copy)` and `@derive(Circularity)` legal on records too, so the finding reads the six as an over-narrow whitelist. But read the scope: the six-entry is about `@derive` that *generates a structural implementation*, and the same section says `@derive(Copy)` 'generates no method body' and `@derive(Circularity)` has 'no structural check, no method body.' Copy and Circularity generate nothing, so they fall outside 'generates structural implementations' by that entry's own wording — not in contradiction to it. Recommended: dismiss, because the six-trait entry is scoped to implementation-generating derivations and the marker derivations that generate no body are outside that scope by construction. Optionally add one clarifying clause that the six are the *implementation-generating* set, not the full `@derive`-eligible set. For execution: LOG 009-43 (six-trait 'fixed set'), 005-194/005-196/005-197 (marker/Copy/Circularity @derive-eligibility); SPEC §6.1.8, §3.8.1. Finding P013 — verdict dismiss.

#### DP-12 — Does a static effect argument lower as a constant cell or an inline literal

**What was claimed.** An effect argument's IR binding has two conflicting representations. One rule says a static argument (like a literal passed to an effect) becomes a synthesized "degenerate constant cell" — the binding slot holds a pointer to that cell. Another rule, for the same effect `parameter_bindings`, says a slot may hold a plain `value_literal` directly. So the same source (`x |> print` with a static `x`) would lower two different ways: a new constant cell plus a cell-id binding, versus an inline literal and no new cell. That difference matters for hot-reload diffing (add/remove a cell vs. change a literal) and cell-id stability.

**The verifier's counter.** "They govern DIFFERENT parameter kinds... a static expression passed to a `cell T` parameter is ALWAYS constant-wrapped into a synthesized constant cell referenced by source_cell_id, tagged `constant_wrap`, never inlined as a value_literal. The `value_literal` case is for value (non-`cell`) parameters, which are evaluated and snapshotted." So one source never has two IR shapes — it depends on whether the target parameter is a cell parameter or a plain value parameter.

The fresh read backs the verifier. The spec's argument-materialization rule and the IR-format section both say the provenance marker (`bound` / `constant_wrap` / `bridge`) is metadata riding on top of a `source_cell_id | value_literal` binding — not a third case — and that a static expression to a cell parameter is constant-wrapped to a `source_cell_id`, while a plain value parameter takes the `value_literal`. The two log entries the finding pairs are not two readings of one slot: one lists the three provenance markers, the other lists the two underlying binding shapes; the spec ties them together consistently.

Recommended: dismiss, because the spec text the finding overlooked already fixes which shape applies (cell-parameter static arg = constant_wrap/source_cell_id; value-parameter arg = value_literal), so no single source lowers two ways and there is no behavior-changing ambiguity to pin.

For execution: F241 (main report). LOG 033-64 (provenance-marker entry), 033-121 (source_cell_id | value_literal pairs). SPEC §13.17.3 uniform cell-argument rule (SPEC.md:19662-19687) and §15.4.1 parameter_bindings IR (SPEC.md:24167-24176). Verdict: dismiss.

#### DP-13 — Can a runtime `match` yield a `Type[C]` without breaking monomorphization

**What was claimed.** A runtime `match` may yield a type value (a `Type[C]`). But two other rules require type values to be resolved at compile time and each supply site to lock to one concrete type (`ListView | item=PostCard` picks `~F = PostCard`). If a `match` over a runtime scrutinee genuinely yields *different* concrete types, the supply site can't lock to one concrete type at compile time, and the placed type carries runtime information the "never erased behind a vtable" rule forbids. If instead all arms must be the same compile-time type, the runtime `match` is pointless. Nothing pins which reading holds.

**The verifier's counter (which sustained the finding).** "The two SPEC passages actually conflict on the SAME case: §5.7.3 says `Selecting among different node structures at runtime is given, not a type value`, but two different Type[Drivable]-satisfying types ARE different node structures, so §5.7.3 routes them to `given` while §13.2.10 routes `one of several types` to match->Type[C]. Neither passage dissolves the tension; both reinforce it."

The fresh read confirms the tension is real. The spec says `match` yielding a `Type[C]` "chooses *one* type, which the receiving node then places once" and "Use `match`->`Type[C]` when exactly one of several types should ever be placed." That describes the observable behavior (one type gets placed) but never says how a single supply-site slot holds a runtime-chosen concrete type while still monomorphizing to one fixed `~F` at compile time. The two mechanisms — compile-time monomorphization to one concrete `~F`, and runtime selection among structurally different types — cannot both hold at one site without a runtime type tag, which the "never erased" rule denies. The spec deepens the conflict instead of resolving it.

Recommended: pin it, because the corpus states the desired behavior but leaves the mechanism contradictory — an implementer cannot tell whether to reject multi-type-arm `match`->`Type[C]`, collapse arms to one compile-time type, or carry a forbidden runtime tag. The user must decide whether such a `match` requires all arms to share one concrete type, or genuinely varies at runtime (and if so, how it reconciles with per-supply-site monomorphization and no-erasure).

For execution: F041 (main report). LOG 008-64 (type value resolved at compile time, never vtable-erased), 008-66 (each supply site monomorphizes to one ~F), 008-69 (runtime match may yield Type[C]). SPEC §5.7.3 (SPEC.md:4701-4704) and §13.2.10 (SPEC.md:12890-12897). Verdict: pin. Surface as a choice, do not resolve unilaterally.

#### DP-14 — Do repeat-materialized scopes fit the five `.exposition` entry kinds

**What was claimed.** The `.exposition` list of a node is a closed sum of exactly five entry kinds (Node, Connection, Bundle, DynamicView, Gated). A `repeat` is allowed at `expose:` level over a value collection (e.g. `Signal[Vec[Post]]`), and it materializes internal scope children whose bodies can hold both nodes and connections. Those scopes aren't caller-supplied dynamic-view members, and the DynamicView walk is nodes-only (`for m in v: interpret_node(m)`), so — the finding argues — none of the five kinds represents them, leaving a completeness hole for an in-language interpreter.

**The verifier's counter.** "A `repeat`'s scope placements are FLATTENED into individual entries in the flat exposition list, each already a `Node`/`Connection`/`Bundle`/`Gated` entry... The node/connection types inside a written repeat body are statically known, so they slot into existing `Node`/`Connection` entries whose runtime variation is only mount/dismount. The closed sum therefore DOES represent them; no sixth kind is needed."

The fresh read backs the verifier. One rule says iteration entries "emit type-internal structure in place... a `repeat` mounts its keyed scopes there"; another says "Each scope's placements join the enclosing structural sequence at the `repeat`'s written position, in source order"; and the spec says a repeat-materialized connection "mounts and dismounts with its keyed scope, appearing in `.exposition` only while mounted" — i.e. it is an individual Connection entry, not a grouped variant. DynamicView is a separate construct for caller-supplied dynamic child *supply* (WeakHandle members), which repeat scopes are not. So a `repeat` body's statically-known nodes and connections each become ordinary Node/Connection entries; only their mount state varies.

Recommended: dismiss, because the corpus explicitly flattens repeat-scope placements into individual Node/Connection/Bundle/Gated entries in the exposition list, so the five-kind sum stays exhaustive and no sixth variant is missing. The finding wrongly assumed repeat scopes must map to DynamicView.

For execution: F056 (main report). LOG 017-219 (closed five-kind sum), 017-234 (iteration entries emit in place), 018-56 (scope placements join at written position), 018-104 (repeat in expose:). SPEC §13.3.7.4 DynamicView walk (SPEC.md:14774,14791) vs. repeat-connection flattening (SPEC.md:14700-14701). Verdict: dismiss.

#### DP-15 — What entry field a bundle `as`-name inside a repeat contributes

**What was claimed.** A bundle placement named `[...] as buf` inside a `repeat ... as` body has no defined entry-field type: the rule that mints one `<view>::entry` field per `as` name types fields as a singular `WeakHandle[T]`, but a bundle `as`-name binds an unstorable row slice, so what field (if any) the bundle name contributes — and whether it is cross-scope addressable — is unspecified.

**The verifier's counter.** "018-143 explicitly makes bundle-in-repeat ILLEGAL: 'a `repeat` body cannot contain bundle brackets', a parse-time error (class bundle_in_repeat_rejected). With the construct rejected, there is no legal case for the WeakHandle[T] field-type rule to fail to cover."

The fresh read confirms this outright: the log entry says bundle brackets are not legal in a repeat body, the compiler emits `bundle_in_repeat_rejected` at parse time, and the two only become legal when un-nested. The finding's premise — that a named bundle inside a repeat is a legal construct — is false.

Recommended: dismiss, because the construct the finding depends on (a bundle bracket inside a repeat body) is a parse-time error, so there is no legal case whose entry-field type could be undefined.

For execution: F117 (main report). LOG 018-143 (bundle_in_repeat_rejected, §13.5.4). Dissolves 018-130 / 017-99 concern. Verdict: dismiss.

#### DP-16 — Scope drop on an already-gated-off subtree: who owns the teardown sequence

**What was claimed.** Consider a node that is already gated-off (frozen, its effects already sent `suspend`, its stream consumer in freeze-and-backlog) at the moment its `repeat` key is removed, dropping the scope. The finding says freeze and retire live in disjoint sections with disjoint vocabularies, and no rule states that scope-drop supersedes an in-progress freeze or specifies the teardown sequence — e.g. whether an already-suspended effect gets a second teardown, or how a frozen stream consumer transitions to the scope-drop path. "Does any section own the composite answer?" resolves to no.

**The verifier's counter.** "§13.14.9 explicitly owns exactly this seam: a suspended instance whose scope then dies receives `teardown` directly (not a `resume` first) — teardown subsumes the release suspend already performed... This answers all three effect sub-questions: no double release, no resume-before-drop, scope-death supersedes freeze. The stream-consumer sub-seam is covered by 028-49 ('A removed consumer's cursor is dropped')."

The fresh read backs the verifier. The spec says plainly: a suspended instance whose scope dies gets `teardown` directly, teardown subsumes the already-fired suspend, the reconciler must tolerate teardown while suspended, and the runtime never delivers resume to an instance leaving scope. That is the integrating rule the finding claimed was absent — it sequences suspend-then-teardown and confirms no double teardown. For streams, a removed consumer's cursor is dropped regardless of the freeze-and-backlog state. Both seams have explicit owning text.

Recommended: dismiss, because the ordering rule for scope-death overlapping suspension already resolves the effect seam (teardown subsumes suspend, no resume first) and the removed-consumer cursor-drop rule resolves the stream seam — the finding missed the section that owns the composite. The residual "which section formally owns it" is a locality nit, not a behavior gap.

For execution: F113 (main report). LOG 022-8/022-9 (gate freeze/no unmount), 022-63 (suspend at gate-close), 030-191 (freeze-and-backlog), 028-49 (removed consumer cursor dropped). SPEC §13.14.9 scope-death-overlaps-suspend ordering (SPEC.md:19169-19174). Verdict: dismiss.

#### DP-17 — Can the fold header's `<members>` slot carry a colon-bearing expression

**What was claimed.** The fold header is `fold <members>:`, and `<members>` sits before a colon. If `<members>` is a general expression position, an inline conditional `if c: a else: b` fits there — and then `fold if c: a else: b:` parses two ways: (1) members = `if c: a else: b`, the trailing `:` opens the fold block; (2) the first `:` opens the fold block, and `a else: b:` becomes garbage. The inline `else:` also collides with the fold's own mandatory `else:` arm. `observe` requires parenthesization for exactly this open-extent problem, but the fold header has no such rule.

**The verifier's counter (which sustained the finding).** "The fold `<members>` slot is a value-producing expression position: 035-6 admits 'reactive composites with a uniform slot type' (not restricted to a bare identifier)... and 002-26 permits an inline `if c: a else: b` as a single-expression value... No parenthesization/self-delimiting rule covers the fold header."

The fresh read weakens the sustain's premise. The scope section says `fold` folds over **membership structures** — a `yielded` group or a reactive composite — and explicitly says a stored value held in a cell is **not** foldable ("`fold` is over membership, not over the contents of a stored container"). An inline `if c: a else: b` yields a *value*, not a membership structure, and the grammar prose says the header "names the `<members>`" — wording that points to a reference, not an arbitrary colon-bearing expression. So the finding's specific witness (an inline conditional producing values in the members slot) would not type-check as foldable, and it is not clearly admissible grammatically. But the grammar never states a delimiter for `<members>` nor explicitly forbids a colon-bearing expression form, so the boundary is genuinely underspecified even though the likely intent is "name/path to a group or composite only."

Recommended: pin it, but narrowly — not the full parenthesization rule the finding proposed. State that `<members>` is a reference (name or path) to a membership structure, not an arbitrary expression, which closes the colon-collision cheaply. The corpus underdetermines whether the slot admits expression forms; on engineering grounds a self-delimiting reference-only slot is the clean fix and matches the "membership structures, not stored values" restriction already in the scope section.

For execution: F101 (main report). LOG 035-2 (fold block shape, mandatory else last), 035-6 (ranges over yielded/reactive composites), 002-26 (inline if/else as single expression). SPEC §13.21.1 grammar (SPEC.md:23100-23109), §13.21.4 scope/foldable (SPEC.md:23143-23155). Compare 016-249 (observe parenthesization). Verdict: pin (narrow: restrict members slot to a reference, do not require full parenthesization).

