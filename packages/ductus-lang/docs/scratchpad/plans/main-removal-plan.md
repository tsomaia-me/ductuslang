# Main-Removal Amendment Plan — `main` retires, `@root` arrives

*2026-07-11 · Executes the ROOT-DESIGNATION ruling (owner, LOCKED). Companion to the BACKLOG
entry "Main-removal redesign" and the rulings ledger in the master-decisions audit plan.
Status: DRAFT — awaiting owner confirmation of the decisions in section 3, then approval.*

---

## 1. Background — why `main` retires

Today a program marks one top-level placement with the `main` keyword. That placement is the
entry point: the compiler walks outward from it, everything it reaches is the live graph, and
any top-level instance it cannot reach is a compile error (`unreachable_top_level_instance`).
Exactly one `main` is required — zero is an error, two is an error.

That construct predates in-language traversal. It was designed for a world where the host
started the walk and needed a designated node to start from. That world is gone: traversal
and interpretation now happen inside Ductus (the interpretation bootstrap is an ordinary
`effects:` entry — `render(song).audio |> audio_out`). The owner's ruling follows through:
the host stays topology-blind and never names nodes; the runtime is one general-purpose
artifact; which node roots the program is a fact expressed in the language, not a keyword
blessed by the host or a magic placement class.

Two audit decisions forced the issue. The closure-vocabulary decision (D-17 in the audit
ledger) and the closure-membership decision (D-23) both found the entry-point machinery
described in two drifting vocabularies. While ruling on those, the owner issued this broader
redesign instead of patching the drift.

## 2. The ruling

Locked, owner, 2026-07-11 (ROOT-DESIGNATION in the rulings ledger):

- The `main` keyword retires entirely.
- A traversal root is designated in-language by the `@root` directive applied to a top-level
  placement. `@root` joins the fixed, language-provided directive set as an applied directive.
- The host never names nodes; the runtime is one general-purpose artifact; traversal and
  interpretation happen inside Ductus.
- An unreachable top-level instance is DEAD CODE — a lint at most, never a compile error.
  The `unreachable_top_level_instance` error class retires.

Before:

```
main Driver john_doe | expertise_level=10:
  Drives | enhanced_handling=true: some_car
```

After:

```
@root Driver john_doe | expertise_level=10:
  Drives | enhanced_handling=true: some_car
```

The defaults below (multiplicity, lint shape, misapplication, zero roots) are recommended
positions awaiting confirmation — they are not yet part of the locked ruling.

## 3. Decisions to confirm

Each item: the question, the recommended default, one line of why. Nothing in this section
executes until the owner confirms or overrides it.

### 3.1 Multiplicity — may a module carry more than one `@root`?

**Recommend: allowed.** Each `@root` placement roots its own closure; the live graph is the
union of all root closures.

```
@root App app:            // roots app's closure
@root Debugger debug:     // roots a second, possibly overlapping closure
Widget spare              // in neither closure -> dead code (lint), not an error
```

Why: with roots visible in source, the union is statically computable — the compiler keeps
the same static-liveness guarantee it had with a single `main`. Exactly-one was an artifact
of the host needing one start node; nothing in the union model needs it.

### 3.2 Dead code — what happens to an instance outside every root closure?

**Recommend: a named, suppressible lint; the instance is absent from the live graph.**
Proposed name: `dead_top_level_instance`.

```
Widget spare    // lint: dead_top_level_instance — not reachable from any @root
```

Why a NEW name rather than downgrading `unreachable_top_level_instance` to a lint: the
semantics changed (union over many roots, advisory not fatal); reusing the old name would
make old error-class citations silently mean something weaker. Why suppressible: the corpus
already has advisory precedent ("style lint", "dead-arm lint", the unused-binding warning),
and library-ish modules legitimately hold instances for a future root. Suppression spelling
(directive, attribute, or toolchain flag) is a follow-up call — recommend deferring the
suppression mechanism to the toolchain, keeping the language surface clean.

### 3.3 Misapplication — `@root` on something that is not a top-level placement?

**Recommend: compile error.**

```
@root
fn helper() -> i32: 42     // error: @root applies only to a top-level placement
```

Why: every applied directive in the set already has a fixed attachment site; misattachment
is a parse/semantic error today (`@flag` off a flags declaration, `@default` off a trait).
`@root` follows the same law: legal on a top-level placement only — not on nested child
placements, not on declarations, not on values.

### 3.4 Zero roots — is a module with no `@root` legal?

**Recommend: legal — that is a library.** Compilation succeeds; there is simply no live
graph to build. The runtime, handed a rootless module, declines to interpret it gracefully
(a clean "nothing to run" condition, not a crash and not a compile error). The `ductus run`
CLI surfaces that condition to the user.

```
// signals.duc — a library module: nodes, traits, fns. No @root. Compiles clean.
```

Why: the old `no_entry_point` error made every compilation unit pretend to be a program.
Packages meant for import were always legal in spirit; this makes them legal in letter. The
zero-root lint story: no lint fires (all instances in a rootless module are library surface,
not dead code) — confirm this reading.

### 3.5 Layout — `@root` sits inline on the placement line

**Recommend: inline prefix, as the ruling's own example spells it — with a disclosed
carve-out.** The directive law today says applied directives sit on their own line directly
above the declaration they modify. The locked example `@root App app:` is inline. So either:

```
@root App app:            // (a) inline prefix — the ruling's spelling  [recommended]
```
```
@root
App app:                  // (b) own-line, per the existing applied-directive layout law
```

Recommend (a), amending the directive layout law with a one-line carve-out: "`@root` is
written inline as a placement-line prefix." The lexer already handles this: `@` opens a
directive in declaration position and is a flag character only inside a placement flag run
(path-adjacent, no whitespace) — line-initial `@root` followed by whitespace and a type name
is unambiguous. The grammar's directive-versus-flag disambiguation note gets reworded to say
so. If the owner prefers zero carve-outs, (b) also works and needs no layout amendment —
but then the ruling's example must be respelled.

### 3.6 Order with visibility — `@root public` or `public @root`?

**Recommend: `@root` first.**

```
@root public Driver root_driver     // recommended: directive precedes visibility
public @root Driver root_driver     // alternative: @root takes main's old slot
```

Why: directives elsewhere decorate the whole declaration from outside (they precede
everything, including visibility, in the annotated-declaration form). Putting `@root` first
keeps that reading. The alternative (visibility first) is minimal grammar churn — `@root`
would occupy exactly the slot `main` vacates. Either is workable; pick one.

### 3.7 IR — does `@root` survive into the compiled module?

**Recommend: no — `@root` is compile-time-only.** The compiler computes the union closure,
prunes dead instances, and emits only the live graph. The runtime initializes every cell in
the loaded graph in topological order; it never looks for a root. This matches the ruling
directly (general-purpose runtime, topology-blind host) and it dissolves a latent defect:
today's startup text says "locate the `main` placement" but the IR module grammar carries no
entry-point field at all — the runtime could never have located it from the IR. Under this
recommendation nothing needs adding to the IR grammar.

### 3.8 Freed word — `main` becomes an ordinary identifier

**Recommend: yes, silently.** With the keyword gone, `main` is a legal instance, fn, or
cell name like any other. No reservation, no migration note in normative text. One example
consequence: the panic-trace example that shows a frame named `main` becomes legal-but-
confusing; the plan respells it (Phase D) to avoid implying a special function.

## 4. Phased execution plan — LOG first

Ordering rule (edit protocol): the decision log changes first, then the spec conforms, then
the grammar documents. Each phase ends with a diff review before the next begins.

### Phase 0 — Sequencing with in-flight work (no edits)

Two in-flight audit executions touch the same text this plan rewrites:

- The closure-membership conform (D-23) edits the spec's reachability paragraph and startup
  step from three closure members to five — the exact paragraphs Phase D rewrites.
- The closure-vocabulary unification (D-17, Option 1) picks a canonical closure name and
  defines "containment closure" — its canonical vocabulary must be root-based ("closure of a
  root"), not entry-point-based, or it is dead on arrival.

Decide the order before editing: either land D-17/D-23 first and let this plan rewrite on
top (safe, some double-editing of the same lines), or fold their closure-membership and
vocabulary outcomes into this plan's rewrites (one pass, but this plan then carries their
verification burden). **Recommend folding in**: the five-member closure enumeration and the
root-based canonical vocabulary are written once, here, in the new entries. Confirm.

### Phase A — LOG, lexical layer (section 002: source form and lexical rules)

Two in-place rewrites. No entries added or removed, so no renumbering and no section-header
count change.

- The declaration-keyword entry drops `main` from its list.
- The directive-taxonomy entry adds `@root` to the applied-directive list.

Consequence to word carefully: keywords are reserved in every position, so removing `main`
un-reserves it (see the freed-word item above). The keyword entry needs no replacement text
— `@root` is a directive, not a keyword, and lands in the directive entry instead.

Renumbering impact: **none.**

### Phase B — LOG, placement layer (section 021: placement)

The heart of the amendment. Strategy: **rewrite in place — zero renumbering.** The retiring
entries are rewritten to carry the new rules rather than deleted, so no entry moves, the
section header count stays at 142, and no external citation of a 021 id goes stale.

- The three placement-form entries that mention the optional `main` prefix drop it (plain
  placement form only).
- The entry-point designation entry becomes the `@root` designation entry: form, inline
  layout, attachment restricted to top-level placements (misapplication = compile error,
  per item 3.3).
- The exactly-one-`main` entry becomes the multiplicity entry: any number of `@root`
  placements including zero; zero = library module; `no_entry_point` and
  `multiple_entry_points` retire (per items 3.1, 3.4).
- The unreachable-instance error entry becomes the dead-code entry: outside every root
  closure = dead code, named suppressible lint, absent from the live graph;
  `unreachable_top_level_instance` retires (per item 3.2).
- The transitive-closure entry becomes the per-root closure entry: each root's closure
  (five members, folding in the closure-membership ruling if Phase 0 confirms), live graph
  = union over all roots, statically computable.

Invariant check: each rewritten entry must stay atomic and self-contained (no entry-number
citations inside entries). The multiplicity entry restates "root closure" locally rather
than pointing at the closure entry.

Renumbering impact: **none** (pure in-place rewrites).

### Phase C — LOG, runtime and tooling layers (sections 027 and 032)

- The startup-initialization entry (which initializes "the entry-point's transitive-closure
  reactive cells") is rewritten: initialization covers every cell in the live graph — the
  union of root closures — in topological order.
- The startup-traversal entry (traversal "beginning at the program's `main` entry-point";
  unreachable instances "caught at compile time by class `unreachable_top_level_instance`")
  is rewritten: the loaded graph is already the live union; the runtime is root-blind; dead
  instances were pruned at compile time and surfaced by the lint.
- The manifest entry ("manifest file specifying the entry point") is conformed to say root
  module, matching the spec's project-layout section — its "entry point" wording was already
  drifted; this removes the last entry-point mention in the tooling layer.
- If the owner confirms graceful-decline (item 3.4), one NEW entry is appended at the end of
  the runtime-interface section: the runtime, given a module whose live graph is empty,
  reports "nothing to run" and exits cleanly.

Renumbering impact: **none for the rewrites; the one new entry appends at the section tail**
(next free number, no later entries to shift). Section header count for the runtime section
increments by one if the new entry lands.

### Phase D — SPEC conformance

Rewrites, in document order:

- The lexical chapter: drop `main` from the keyword list; add `@root` to the applied-
  directive list with a pointer to the placement chapter; extend the directive-versus-flag
  positional note for the inline placement prefix (item 3.5).
- The panic-trace example naming a `main` frame: respell the frame name (item 3.8).
- The placement chapter's "Entry-point designation" block becomes "Root designation":
  `@root` form, multiplicity, per-root closure (five members if Phase 0 folds), union
  liveness, dead-code lint. The three retiring error classes come out; the lint goes in.
  This is the D-23 collision zone — coordinate per Phase 0.
- The runtime-lifecycle startup step ("Locate the entry-point node instance...") becomes:
  initialize every cell in the loaded (already-pruned) graph in topological order. This also
  dissolves the latent locate-from-IR gap (item 3.7).
- The interpretation-bootstrap passage: confirm its "interpretation root" wording stands on
  its own now that "entry-point" is gone; reword its one entry-point-adjacent sentence if
  the D-17 fold lands here.
- The project-layout section: the manifest still names the root module; the sentence saying
  the entry-point is "declared separately in source via the `main` keyword" is rewritten to
  name `@root`.

Left alone on purpose (incidental English, not the keyword): "the placement's main line"
(attribute-layout prose, several sites), `main.duc` example filenames, `audio#main`
identifier examples, "reactive entry points" describing signals, "main thread" in the
backends notes. These are whitelisted in the verification grep.

### Phase E — GRAMMAR.md conformance

Caveat: the grammar document's normative status is still an open audit question (D-31).
Precedent from the cell-spelling ruling (D-01) is that grammar conformance runs as its own
pass regardless. This phase proceeds on that precedent; flag if the owner wants it gated.

- Keyword inventory: drop `'main'` from the declaration-keyword block.
- Top-level declaration disambiguation note: placements no longer begin with an optional
  `main`; the parser cue becomes the inline `@root` prefix or a bare type head.
- The top-level placement production: `'main'?` is replaced according to items 3.5/3.6
  (recommended: an optional inline `RootDirective` before the visibility prefix). The
  production's comment block (entry-point, exactly-one, error classes) is rewritten; the
  navigational stub heading retitles.
- The directive chapter: `'root'` joins the closed `DirectiveName` list; the "six applied
  names" count becomes seven; the own-line layout law gains the inline carve-out; the
  directive-versus-flag note is reworded (line-initial `@name` + whitespace + type head =
  directive; path-adjacent `@` in a flag run = flag character).
- Citation repair: grammar comments cite LOG ids for the entry-point rules; since Phase B
  rewrites in place, those ids still exist but now say different things — every cited id in
  the touched comment blocks is re-verified against its rewritten text.

### Phase F — IR_GRAMMAR.md (expected no-op)

The IR grammar contains no `main` and no entry-point field; its `entry` production is the
behavior-block entry (cell/gate/connection/effect), unrelated. If item 3.7 is confirmed
(no root marker in IR), this phase is a verification-only pass. If the owner instead wants
roots encoded in IR, this phase gains a module-header field and the plan must be re-approved
on that point.

### Phase G — Self-review, cold review, verification

- Run the LOG structure linter (numbering density, section prefixes, spec-ref resolution).
- Run the verification greps (section 6) against the whitelist.
- Cross-reference every rewritten entry against this plan and the rulings ledger.
- Cold reviewer pass over the diff, per the definition-of-done protocol.

## 5. For execution

Surveyed ids and line numbers, per phase. Line numbers are as of the survey (2026-07-11,
working tree at ca84f1c + uncommitted doc edits); re-grep before editing — they will drift.

> **Phase A — DECISION_LOG.md, section 002:**
> 002-3 (L74): declaration-keyword list — remove `` `main` ``. Do not touch its
> `collect as x:` clause here (already retired by the separate COLLECT-AS ruling; that
> amendment owns the edit — coordinate to avoid a merge collision on the same line).
> 002-14 (L85): applied-directive list — add `@root` alongside `@derive`,
> `@literal_suffix`, `@flag`, `@reset_on_reopen`, `@reset_on_reload`, `@default`.
> Untouched neighbors that mention main-as-identifier only: 002-15 (L86), 002-21 (L92)
> (`audio#main` examples — keep).

> **Phase B — DECISION_LOG.md, section 021 (header L2746, "142 Rules" — count unchanged):**
> In-place rewrites: 021-3 (L2750), 021-4 (L2751), 021-5 (L2752) — drop the `main` prefix
> from the placement forms; 021-138 (L2885) → @root designation + attachment rule;
> 021-139 (L2886) → multiplicity/zero-roots (retire `no_entry_point`,
> `multiple_entry_points`); 021-140 (L2887) → dead-code lint `dead_top_level_instance`
> (retire `unreachable_top_level_instance`); 021-141 (L2888) → per-root five-member closure
> + union live graph (fold D-23 membership + D-17 root-based vocabulary if Phase 0 confirms).
> Do-not-touch (incidental "main line" = first line of the placement): 021-9 (L2756),
> 021-11 (L2758).

> **Phase C — DECISION_LOG.md, sections 027 and 032:**
> 027-18 (L3204) → live-graph initialization wording; 027-120 (L3306) → root-blind runtime
> wording, lint reference. Optional NEW entry 027-121 appended (graceful decline on empty
> live graph) — increments header count at L3185 from 120 to 121.
> 032-37 (L3976) → "root module" manifest wording.
> Compatible closure-vocabulary entries reviewed but expected untouched (D-17 owns them):
> 015-16 (L1832), 017-271 (L2420), 024-17 (L3092), 004-45 (L229), 013-48 (L1441).

> **Phase D — SPEC.md:**
> L128 (§1.4 keyword list): remove `` `main` ``. L152–159 (§1.4 directives): add `@root`;
> extend the placement-position `@` note. L6181 (§4-region panic-trace example): respell
> the `main` frame. §13.8.1 block L16619–16649: full rewrite (designation, multiplicity,
> reachability, error→lint) — D-23 collision zone L16636–16643. §13.14.1 startup step 3
> L19073–19093: rewrite. §13.17.7 Case 3 L20105–20124: review/reword entry-point-adjacent
> wording. §14.2.3 L23558–23566: manifest/root-module passage — rewrite the `main` sentence
> at L23563–23566. Compatible interpretation-closure sites reviewed, not rewritten here:
> L524 (§13.19 carve-out), L15004 (participation report).
> Do-not-touch whitelist: L162 (`audio#main`), L7684/L7721/L7728/L7731 (`main.duc`
> filenames), L11729 ("reactive entry points"), L16674, L16829, L17216, L17233, L17266
> ("placement's main line").

> **Phase E — GRAMMAR.md:**
> L232 (keyword inventory): remove `'main'`. L3092–3094 (TopLevelDecl disambiguation
> comment): rewrite. §7.16 stub L3994–4000: retitle. §11.2 L5205–5240: production
> `TopLevelPlacement ::= Visibility? 'main'? …` → per items 3.5/3.6; rewrite comments
> L5224–5235 (retire the exactly-one/error-class note at L5231–5235); retitle heading
> L5205. §12 L5625–5673: `DirectiveName` list gains `'root'` (L5642–5648); "six applied
> names" comment → seven (L5663–5667); directive-vs-flag notes L671–677 and L5669–5673:
> reword for the inline prefix. §12.1 L5678+: add a `RootDirective` production with the
> layout carve-out. Citation re-verify in touched blocks: 021-138 cites at L5211, L5231;
> 021-139 cite at L5234.
> Do-not-touch: L217 (`audio#main`), L5273, L5351 ("main line").

> **Phase F — IR_GRAMMAR.md:** no `main` occurrences (verified); `entry` production
> L173/L181 is behavior-IR, unrelated. Expected zero edits under item 3.7.

## 6. Verification greps

Run after Phase G. All paths relative to `packages/ductus-lang/docs/`. The rtk grep proxy
truncates multi-file output — run each file separately.

```bash
# 1. The keyword is gone. Only whitelisted incidental hits may remain.
grep -n '\bmain\b' DECISION_LOG.md   # expect: only 002-15/002-21 (audio#main),
                                     #         021-9/021-11 ("main line")
grep -n '\bmain\b' SPEC.md           # expect: only audio#main, main.duc filenames,
                                     #         "main line" x5, "entry points" (signals)
grep -n '\bmain\b' GRAMMAR.md        # expect: only audio#main (L217-region), "main line" x2
grep -n '\bmain\b' IR_GRAMMAR.md     # expect: 0
grep -n "'main'" GRAMMAR.md          # expect: 0 (no keyword-quoted form survives)

# 2. The retired error classes are gone everywhere.
grep -n 'no_entry_point\|multiple_entry_points\|unreachable_top_level_instance' \
    DECISION_LOG.md SPEC.md GRAMMAR.md IR_GRAMMAR.md   # expect: 0 in each

# 3. The new surface exists.
grep -cn '@root' DECISION_LOG.md     # expect: >=5 (002-14, 021-3/4 forms, 021-138..141)
grep -cn '@root' SPEC.md             # expect: >=4 (§1.4, §13.8.1, §13.14.1, §14.2.3)
grep -n "'root'" GRAMMAR.md          # expect: DirectiveName list + §11.2 production
grep -n 'dead_top_level_instance' DECISION_LOG.md SPEC.md   # expect: definitional hits

# 4. Entry-point vocabulary retired (incidental "entry points" for signals excepted).
grep -n 'entry-point\|entry point' DECISION_LOG.md   # expect: 0
grep -n 'entry-point' SPEC.md                        # expect: 0

# 5. Structure intact.
python3 lint_nnm.py DECISION_LOG.md SPEC.md          # expect: clean
grep -n '^## 021\.\|^## 027\.' DECISION_LOG.md       # counts: 142; 120 (121 if new entry)
```

## 7. Open items for the owner (summary)

1. Confirm defaults 3.1–3.8 (multiplicity, lint name/shape, misapplication error, zero
   roots + graceful decline, inline layout carve-out, visibility order, no root marker in
   IR, `main` freed as identifier).
2. Confirm Phase 0 sequencing: fold the D-17 vocabulary and D-23 five-member membership
   into these rewrites, or land them first.
3. Note the directive-layout carve-out (3.5): the locked example `@root App app:` is the
   first inline applied directive; the own-line law gets a stated exception.
4. Note D-31 (grammar-doc status) is still open; Phase E proceeds on the D-01 precedent
   unless gated.
