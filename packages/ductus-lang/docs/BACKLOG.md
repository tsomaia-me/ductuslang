# Ductus — Decision & Syntax Backlog

Open items deferred from the spec-findings analysis: genuine design forks, leaned/editorial
fixes awaiting ratification, and syntax questions the spec decides nowhere. Each item is a
self-contained ticket so it can be answered without re-reading the 22k-line `SPEC.md`.

**Source & caveat.** Tickets are mined from the `RULINGS.md` per-finding worksheets and the
`DECISION_LOG_FINDINGS.md` spec-silent appendix. Those worksheets are substance-current for
these (still-live) findings, but their **line numbers have drifted**; section numbers (`§x.y`)
are stable. When acting on a ticket, locate the target by `§` + quoted content, not by any line
number.

**Partition of the 61 live findings** (for orientation; resolved/moot are handled outside this
file): 20 resolved-on-merits · 2 moot (F017, F053) · **11 fork-findings → 10 tickets** (Section
1) · **28 pure lean/editorial** (Section 2). Section 3 (11) + Section 4 (4) are the open syntax
questions; Section 5 is doc-hygiene.

**How to use.** Forks (Section 1) need a real decision from the owner — open them one at a time.
Leans (Section 2) carry a recommendation + the rejected reading; ratify in batches. Sections 3–4
can be answered directly or delegated to a draft-from-legacy-grammar pass.

Legend — tier: `foundational` (reshapes a subsystem) · `cluster-root` (cascades to siblings) ·
`lean` (clear recommendation) · `editorial` (typo / one-defensible-reading).

---

# Section 1 — Design forks (you decide)

11 findings → 10 tickets (F129+F134 are one decision). Six hard forks (no clean default) then
four leaned-reshape forks (a recommendation exists, but it changes a rule, so it needs ratifying).

## 1. [foundational · trait/call] Named-form call: trait param names vs impl-renamed params  (F020)

**Problem.** When a trait method is called by named argument, which parameter names are the
contract — the trait's, or the implementing `fulfill`'s? An impl may freely rename parameters,
so at a generic call site the legal argument names are undetermined.

**Context.** §3.5.1/§3.5.3 say named form is available for trait methods "at every call site,"
matched by parameter name. But §3.3.1 says the parameter name is "always the implementer's
choice": trait `Add` declares `fn add(a: Subject, b: Rhs)`, while `fulfill Add for Vec3` writes
`fn add(left: Vec3, right: Vec3)`. At a generic site `t.add(? : x)` where `t: T` is only known
as `T: Add`, the concrete impl (hence its names) is unresolved — so whether `v1.add(b: v2)` or
`v1.add(right: v2)` is legal is genuinely undefined.

**Potential solutions.**
- (A) Named-form binds to the **trait's** declared names; impl renames are cosmetic/local
  (`v1.add(b: v2)`). Keeps §3.3.1 freedom; names the trait as the contract.
- (B) Named-form binds to the **resolved impl's** names; named form is then unavailable at
  generic `T: Add` sites, available at concrete sites (`v1.add(right: v2)`).
- (C) Require `fulfill` parameter names to **equal** the trait's. Uniform, kills the ambiguity,
  but removes the §3.3.1 "implementer's choice" freedom for trait methods.

**What.** LOG the chosen rule; SPEC §3.5.5/§3.3.1 state the named-form contract for trait
methods and whether `fulfill` must enforce parameter-name equality (a checker-level consequence).

**Why.** Decides whether the checker enforces `fulfill` param-name equality; affects every
named call of a trait method. No clean default — A and C trade against §3.3.1's stated freedom.

**Refs.** F020 · §3.5.1, §3.5.3, §3.3.1 · standalone · OPEN.

-----------

## 2. [foundational · CL-IR] Behavior-ID width is self-contradictory; `BID` lexeme undefined  (F129 / F134)

**Problem.** A behavior's identity is specified two incompatible ways. §14.6.3 calls it a
"stable u32 ID … content-addressed: a stable hash of the canonicalized typed IR" — but a u32
cannot losslessly carry a content hash. The IR text form renders IDs as `B@aa10` / `B@d1`…`B@d5`
(2–4 hex = 8–16 bits, narrower than 32), and the text grammar's
`behavior ::= 'behavior' BID '(' … '}'` uses a `BID` lexeme that is **never given a production**
(verified: `BID` used at SPEC ~§15.4.4 and LOG 033-219, defined nowhere).

**Context.** §14.6.3 (behavior ABI), §15.4.4 (IR text form). DECISION_LOG #3740/#3898/#4007 all
say "content-addressed" and never load-bear a u32 width. §15.4.1 says "runtime binds IDs to
function pointers" — suggesting a handle/index distinct from the hash. Cross-implementation
interop (§15.7.3) and hot-reload matching both key on this ID.

**Potential solutions.**
- (A) The ID is literally u32: the content hash is truncated to 32 bits; the spec must state
  collision handling (compile error, or disambiguation).
- (B) The content address is a wide hash (≥128-bit) and "u32" names a separate runtime handle /
  behavior-table index. Reconciles the DoR, the `B@…` rendering, and the function-pointer role.
  (Lean: B.)

**What.** Decide width semantics. Define the `BID` lexeme grammar in §15.4.4 (`B@` + fixed hex
width). Make every `B@…` example width-consistent. Align §14.6.3 wording. Amend DECISION_LOG
#3740/#3898.

**Why.** Foundational for IR interop and hot-reload identity. Couples with F135/F136/F138
(text-form shape). One decision answers both F129 and F134.

**Refs.** F129 + F134 · §14.6.3, §15.4.4 · DoR #3740/#3898/#4007 · CL-IR · OPEN.

-----------

## 3. [foundational · CL-DROP / CL-GRAMMAR] How does a Drop body release (move out) a field?  (F133)

**Problem.** Drop bodies are said to "move fields out," but the `move` grammar accepts only bare
l-value identifiers and field access never consumes — so `move self.field` is unexpressible. The
release mechanism is unspecified.

**Context.** §14.7.1: a Drop body "may move fields out to release them (partial moves, §14.7.3)";
§14.7.3: drop glue drops "fields left un-moved." But §11.8.5: `move <l-value identifier>` is
legal only as an immediate call-argument sub-expression, and `move v.method()` (dotted) is a
parse error; §11.8.3/§11.3.1: field access reads "without ownership transfer," never consumes.
So the surface `move` cannot express `move self.field`.

**Potential solutions.**
- (a) Extend §11.8.5's `move` grammar to admit field l-values (`move self.field`) in
  partial-move/Drop contexts.
- (b) Define a distinct field-release construct.
- (c) Declare §14.7.1/.3's "move fields out" non-normative — drop glue handles all field
  teardown; bodies cannot selectively release.

**What.** SPEC §11.8.5 (+§14.7.3) per ruling; LOG add. Fixes `move`'s operand grammar AND
whether Drop bodies can selectively release fields.

**Why.** Foundational for ownership + teardown semantics; overlaps CL-GRAMMAR (the `move`
operand production).

**Refs.** F133 · §14.7.1, §14.7.3 vs §11.8.5, §11.8.3, §11.3.1 · CL-DROP/CL-GRAMMAR · OPEN.

-----------

## 4. [cluster-root · CL-IR] Graph-gate encoding: per-instance `gate_parent` vs named `guards` lists  (F135)

**Problem.** The IR encodes gates two ways. The abstract record gates per gated instance
(predicate + `gate_parent` path); the text form materializes named gate entries with `guards`
lists. They are not isomorphic for a shared gate.

**Context.** §15.4.1 abstract record: a `when`-gate is "per gated instance" — predicate behavior
ID + input cell IDs + `gate_parent` (nearest enclosing gated instance). §15.4.5 worked example:
`gate App.g0 pred B@d4 inputs [App.show] guards [App.print:0]` (a named gate with a guards list)
and the guarded `effect App.print:0 ... gate App.g0` references it back. A `when` block guarding
N instances = one named gate with N guards (text form) vs N instances each pointing at a
`gate_parent` (abstract record).

**Potential solutions.**
- (A) Per-gated-instance: each instance carries its own predicate + `gate_parent`; no standalone
  gate entry.
- (B) Named gate entries with `guards` lists referenced by guarded instances.

**What.** Reconcile §15.4.1 and §15.4.5 — either add a normative gate entry (`id`,`pred`,
`inputs`,`guards`) to the §15.4.1 record and relate it to `gate_parent`, or restate §15.4.5 in
per-instance form. LOG amend the §15.4.5 gate entry.

**Why.** Root of the §15.4 text-form-vs-field-lists divergence. The §15.4.5 example currently
mixes both encodings. Couples with F138.

**Refs.** F135 · §15.4.1, §15.4.5 · CL-IR · OPEN.

-----------

## 5. [foundational · CL-IR / CL-GRAMMAR] Which IR serialization is normative; the graph grammar is missing  (F138)

**Problem.** Both the §15.4.1 abstract record and the §15.4.5 text form are declared normative,
yet they carry different fields — and only `behavior` has a text grammar, so the module/types/
graph sections have no defined concrete syntax, leaving interop undefined.

**Context.** DoR #3851: "The IR's text form IS normative … what tests assert against." §15.7.1.3:
"Produces an IR conforming to the abstract data model of §15.4.1. Its text form is normative."
Divergence: §15.4.1 cells carry observability/cadence/size/alignment + separate dependency-edge
lists; §15.4.5 cell lines carry `role=`/`uses`/`inputs`/`depth`/`init` inline and OMIT
observability/size/align, with no separate edge lists. Only §15.4.4 (`behavior`) has a text
grammar; module/types/graph sections have none. §15.7.3 defines interop against the text form.

**Potential solutions.**
- (A) The §15.4.1 abstract record is the normative source; the text form is an illustrative
  rendering whose field omissions are mere brevity.
- (B) The §15.4.5 text form is the normative serialization, requiring full field fidelity.

Either way: a text grammar for the module/types/graph/cell sections **must** be written
(paralleling §15.4.4's behavior grammar).

**What.** SPEC §15.4 add the normative graph-section text grammar; reconcile which fields the
text form carries (observability/size/align must appear if the text form is the interop wire).
LOG amend #3851.

**Why.** Foundational for IR interop; §15.7.3 is undefined without it. Couples with F135/F136/F137
and the (already-resolved) grammar-inlining decision F038.

**Refs.** F138 · §15.4, §15.4.1, §15.4.5, §15.4.4 · DoR #3851 · CL-IR/CL-GRAMMAR · OPEN.

-----------

## 6. [foundational · CL-IR] Can cross-impl hot reload silently flip a cell's observability class?  (F140)

**Problem.** A carried-over cell's observability class can differ between the old and new spec
across a hot reload (or a cross-implementation swap), but the reload classifier never looks at
observability — while §15.4.1.4 forbids silently changing a cell's concurrency contract.

**Context.** §15.4.1.4: the `cross_thread_snapshot`→`confined` downgrade is "implementation-
defined" (compilers A and B may emit different observability for the same path-matched cell), and
the compiler "may NOT upgrade observability — a `confined` cell becoming `cross_thread_snapshot`
would silently change concurrency semantics." §15.6.1: reload classification (reload-safe /
per-instance-restart / full-restart) is "computed from the diff alone" and never lists an
observability-class change. §15.7.3 permits cross-impl mixing (compiler A's spec → runtime B).

**Potential solutions.**
- (1) An observability-class change on a carried-over cell is outside the diff's input set and
  silently honored (observability recomputed per build, not part of cell identity).
- (2) It is a reload-classification input that must force a per-instance/full restart when it
  would flip the concurrency contract (honoring §15.4.1.4's no-silent-change intent across reload
  and cross-impl swaps).

**What.** SPEC §15.6.1 add observability-class change to the diff's input set with a
classification (recommend: downgrade snapshot→confined reload-safe; any contract-widening change
or cross-thread-observed mismatch forces per-instance restart). §15.4.1.4 cross-reference §15.6.1.

**Why.** Sets the boundary of §15.6.1's input set and whether cross-impl hot reload can silently
alter a cell's concurrency contract.

**Refs.** F140 · §15.4.1.4, §15.6.1, §15.7.3 · CL-IR · OPEN.

-----------

## 7. [leaned-reshape · CL-UMBRELLA] Extend the dispatch-collision check to auto-satisfied traits  (F008)

**Problem.** The dispatch-determinism guarantee rests on a collision check over the written
`satisfies` set, but auto-satisfied traits never appear in any `satisfies` set — so two in-scope
auto-satisfied traits with a same-named default-bodied method could yield two parent-trait
candidates, which the guarantee says is impossible.

**Context.** §3.4.1 step 1 keys on "x's type **fulfills** T" (includes auto-satisfied traits) and
asserts determinism "because §3.2.1's parent-trait collision rule guarantees …" — but §3.2.1
checks the `satisfies` **set** only. Auto-satisfied traits (§3.3.5) are in no `satisfies` set.
Narrow bite: auto-satisfied traits have no abstract methods, so a collision needs two distinct
auto-satisfied traits each declaring a same-named *default-bodied* method, both in scope on one type.

**Potential solutions.**
- (B1, lean) Extend the collision check to the **effective** fulfilled-trait set (directly
  satisfied + auto-satisfied + their `requires` closures). One uniform rule; may reject some
  today-legal type/scope combinations; reshapes the determinism proof.
- (B2) Narrower rule: default-bodied methods on auto-satisfiable traits may not share a name with
  another in-scope auto-satisfiable trait's method.

**What.** LOG amend the collision check to range over the effective fulfilled-trait set; SPEC
§3.2.1 + §3.4.1 restate the guarantee and the §3.4.1 determinism justification.

**Why.** Reshapes the dispatch-determinism proof; ratify because B1 can reject existing code.

**Refs.** F008 · §3.2.1, §3.4.1, §3.3.5 · CL-UMBRELLA · OPEN, lean B1.

-----------

## 8. [leaned-reshape · CL-BOUNDS] Implicit widening to `isize`/`usize` from wider sources  (F029)

**Problem.** §4.5.1 lists `isize`/`usize` widening targets only in the `i8`/`u8` source rows, but
the prose "same-signedness widening is implicit" reads as general — and `isize` is platform-width,
so `i64 → isize` is not lossless on a 32-bit target.

**Context.** §4.5.1 table has no row for `i16`/`i32`/`i64` → `isize`; §4.1: `isize` is 32- or
64-bit per platform; §4.5 invariant: implicit widening "fires only when provably lossless."

**Potential solutions.**
- (A, lean) Treat the table as the authoritative whitelist: implicit widening to `isize`/`usize`
  fires only from sources provably ≤ the minimum pointer width (`i8`/`i16`→`isize`,
  `u8`/`u16`→`usize`, assuming a ≥16-bit floor); `i32`/`i64`/`u32`/`u64`→`isize`/`usize` require
  an explicit cast.
- (Owner-fork) Fix a minimum `isize` width and allow `i16`→`isize` implicitly (or not).

**What.** LOG amend; SPEC §4.5.1 add explicit `i16`/`u16` rows (✓ if ≥16-bit floor) and
`i32`+/`i64`+ rows (✗ explicit cast).

**Why.** Portability of index/size code; ties to F041/F042 (already ruled: same-width signed↔
unsigned needs an explicit cast).

**Refs.** F029 · §4.5.1 · CL-BOUNDS · OPEN, lean A (owner confirms the min-pointer-width floor).

-----------

## 9. [leaned-reshape · CL-KEYWORD] Bare variant patterns resolve against the scrutinee enum  (F035)

**Problem.** Every variant reference is path-qualified by default (unqualified needs `use`), yet
the spec's canonical `match` examples write bare variant patterns with no imports — so either an
unstated exemption exists, or the examples are wrong.

**Context.** §6.2.3: variant references are path-qualified by default; unqualified requires `use`.
But §6.2.4/§6.2.1.1 write `Circle(radius):`, `Rectangle(w, h):`; §6.3.1 writes bare `Ok(Email(s))`;
§8.3.1 writes bare `Some`/`Ok` — none importing. Only `panic` is documented prelude, so the
prelude theory fails for user enums like `Circle`.

**Potential solutions.**
- (A, lean) Pattern arms (and constructions with a known expected enum type) resolve variant
  names against that enum without `use`; path-qualification is required only at *open* value
  positions where no enum is fixed by context. (Standard ML/Rust model.)
- (B) The rule is universal and every canonical example must add the missing `use` imports.

**What.** LOG add the pattern/expected-type resolution exemption; SPEC §6.2.3 add a paragraph
reconciling it with the qualify-by-default rule.

**Why.** Validates ~all `match` examples spec-wide at once (cluster-root). Adjacent to the IR/
grammar gaps but is a name-resolution decision.

**Refs.** F035 · §6.2.3, §6.2.4, §6.2.1.1, §6.3.1, §8.3.1 · CL-KEYWORD · OPEN, lean A.

-----------

## 10. [leaned-reshape · CL-DECLKINDS] Does `ductus check` run monomorphization?  (F132)

**Problem.** `ductus check` "runs the frontend," but the parenthetical lists exactly four passes,
omitting the fifth (monomorphization) — so whether monomorphization-stage diagnostics surface in
`check`/LSP is undetermined.

**Context.** §14.1.1 lists 5 frontend passes (5 = monomorphization). §14.2.1: "`ductus check` —
runs the frontend (lexing, parsing, type checking, ownership checking, reactive analysis) without
invoking either backend" — the parenthetical is exactly passes 1–4. Stakes: deferred const-generic
bound checks (§2.5.6) arise only during monomorphization.

**Potential solutions.**
- (B, lean) `check` runs passes 1–4 only (frontend minus monomorphization); LSP gets fast
  incremental checking; monomorphization-only diagnostics surface at `build`/`run`. Owner must
  confirm that diagnostic-coverage tradeoff is acceptable.
- (A) `check` runs all five passes, including monomorphization (slower, fuller diagnostics).

**What.** SPEC §14.2.1 state explicitly which passes `check` runs and where monomorphization-only
diagnostics surface; LOG add.

**Why.** Determines LSP/editor diagnostic coverage; B is a deliberate tradeoff the owner should
sign off on.

**Refs.** F132 · §14.2.1, §14.1.1 · CL-DECLKINDS · OPEN, lean B (confirm tradeoff).

-----------

# Section 2 — Pure lean / editorial (ratify in batches)

28 items. Each carries a recommendation and the rejected reading. The DECISION_LOG already takes
no contrary position; these refine SPEC prose (a few add a small log entry).

## 11. [lean · CL-INITLET-adjacent] Annotation context for a compound placeholder RHS  (F001)

**Problem.** Does a binding annotation flow *into* a compound RHS expression's sub-expressions, or
only resolve a bare placeholder RHS?

**Context.** §2.1.2 lists "an explicit type annotation (`let y: i32 = x`)" as a use site that
resolves the RHS placeholder, but a later §2.1.2 paragraph says a compound RHS "resolves first
from its own context … the binding's annotation status does not provide context to the
expression." So `let y: i64 = x * 2` (x a placeholder) is ambiguous: resolve `*` at i64 from the
annotation, or default internally (i32) then check?

**Potential solutions.** (A, lean) The annotation resolves only a *bare* placeholder RHS; a
compound RHS resolves from its own operand context/defaulting, after which the annotation is
checked against the result (never flows inward). (B) The annotation flows inward to sub-expressions.

**What.** LOG add the bare-vs-compound rule; SPEC §2.1.2 cross-reference the use-site bullet to the
forward-only paragraph.

**Why.** Determines inferred types of annotated compound bindings; soundness of "same value
resolves differently at two sites."

**Refs.** F001 · §2.1.2 · OPEN, lean A.

-----------

## 12. [editorial · CL-DECLKINDS-adjacent] "Two binding forms" count: let/const vs let/mut  (F004)

**Problem.** §2.4.1.1's unqualified "The language has two binding forms" (let/const) contradicts
the three-form reality (let, mut, const).

**Context.** §2.4.1.1 lists `let`/`const`, omitting `mut`; §11.2 says "two binding forms **for
runtime values**" (let/mut) and treats `const` separately. The decision log (202/1252) carries
§11.2's qualified framing, not the bare count.

**Potential solutions.** (B, lean) Editorial: qualify §2.4.1.1 to the let-vs-const contrast (or
state three forms total). (A) Leave it (loose wording).

**What.** SPEC §2.4.1.1 qualify the sentence; LOG optional clarifying entry (entries already correct).

**Why.** Removes a self-contradiction; no semantic change.

**Refs.** F004 · §2.4.1.1 vs §11.2 · OPEN, lean B (editorial).

-----------

## 13. [lean · standalone] `usize` const-generic size param vs `isize` array length  (F006)

**Problem.** A `usize`-typed symbolic size parameter is used as an array length whose type is
`isize` — is that well-formed?

**Context.** §2.4.4/§2.5.2 declare `const N: usize` then use `data: T[N]`; §9.3.3 fixes array
length type as `isize`; §4.6.5: `let arr: i32[v]` is a compile error if the value doesn't fit
`isize`. §9.3.4 already establishes any integer index widens to `isize`.

**Potential solutions.** (A, lean) Well-formed: the symbolic `usize N` bridges to `isize` by
value-fits checking at instantiation (a `usize` value exceeding `isize::MAX` is then a compile
error). `usize` is the natural domain for a capacity. (B) Force size params to `isize`.

**What.** LOG add the bridge rule; SPEC §2.5.2 (or §9.3.3) one sentence.

**Why.** Declaration-time well-formedness of `T[N]` with symbolic `usize N`.

**Refs.** F006 · §2.5.2, §9.3.3, §4.6.5 · OPEN, lean A.

-----------

## 14. [cluster-root · CL-UMBRELLA] Undefaulted associated type vs auto-satisfaction  (F007)

**Problem.** Auto-satisfaction is gated only on satisfied `requires` + all-default-bodied methods
— silent on associated types — so a trait with an undefaulted associated type but no abstract
methods would auto-satisfy with the associated type left **unbound**.

**Context.** §3.3.5 states the auto-satisfaction condition without mentioning associated types;
§3.1.2 shows undefaulted associated types are real (`type Item` with no default); §3.3.2:
"an associated type without a default must be bound explicitly in the `fulfill` block" — but
auto-satisfaction skips the `fulfill` block.

**Potential solutions.** (B, lean) Auto-satisfaction additionally requires every associated type
to have a default (or be absent); an undefaulted associated type forces explicit `satisfies`+
`fulfill`. (A) Leave it (incoherent — nothing binds the type).

**What.** LOG amend entry 388; SPEC §3.3.5 add "and every associated type has a default."

**Why.** Root of F009 (classification); interacts with F008.

**Refs.** F007 · §3.2, §3.3.5 · CL-UMBRELLA · OPEN, lean B.

-----------

## 15. [editorial · CL-UMBRELLA] "Pure-requirement trait" definition mismatch  (F009)

**Problem.** §3.2 equates "traits with no methods" with pure-requirement traits, but §3.3.5
defines pure-requirement as no methods **and** no associated types.

**Context.** §3.2: "traits with no methods (pure-requirement traits, §3.3.5)"; §3.3.5: "declares
no methods and no associated types." The log (entry 387) already carries §3.3.5's stricter
definition.

**Potential solutions.** (B, lean) Editorial: tighten §3.2's parenthetical to "traits with no
abstract methods," reserving "pure-requirement" for §3.3.5's sense.

**What.** SPEC §3.2 change the parenthetical; LOG none.

**Why.** Classification consistency; resolving F007 makes the distinction moot for auto-satisfaction.

**Refs.** F009 · §3.2, §3.3.5 · CL-UMBRELLA · OPEN, lean B (editorial).

-----------

## 16. [lean · standalone] Conditional `fulfill` vs the satisfies/fulfill pairing rule  (F012)

**Problem.** A conditional `fulfill … where …` requires a paired `satisfies` clause, but no
conditional-`satisfies` syntax exists — so what must a generic type's body contain?

**Context.** §3.2 requires `satisfies`↔`fulfill` pairing for method-bearing traits; §3.3.4 shows
`fulfill Display for Result[T,E] where T: Display, E: Display` with no `satisfies` line in the
snippet. No conditional-`satisfies` surface form exists.

**Potential solutions.** (A, lean) The body carries a plain unconditional `satisfies Display`; the
`where`-clause on the `fulfill` is the sole carrier of the availability condition. Pairing is
satisfied; no new syntax needed. (B) Invent conditional-satisfies. (C) Waive pairing.

**What.** LOG add the rule; SPEC §3.3.4 show the `satisfies Display` line or state the pairing.

**Why.** Whether §3.3.4's canonical example compiles.

**Refs.** F012 · §3.2, §3.3.4 · OPEN, lean A.

-----------

## 17. [lean · standalone] `@literal_suffix` on a bare type alias: error or advice?  (F014)

**Problem.** Is registering a literal suffix on a bare type alias a compile error, or merely
ineffective advice?

**Context.** §3.9: "Note: the registered type must be a distinct nominal type … a bare type alias
… defeats the purpose; use a newtype." The phrasing ("Note:", "defeats the purpose") is advisory;
nothing says "compile error" (contrast §3.9.1/§3.9.2, which say it explicitly).

**Potential solutions.** (B, lean) It compiles but yields no nominal distinction (the suffix's
values are the underlying type). (A) Stricter: alias registration is a compile error (needs a new
normative sentence).

**What.** LOG add the legal-but-ineffective rule; SPEC §3.9 leave the Note advisory, optionally
append "(this is not a compile error)."

**Why.** Whether `@literal_suffix` on `type Frequency = i64` compiles.

**Refs.** F014 · §3.9 · OPEN, lean B.

-----------

## 18. [lean · CL-GRAMMAR] Digit-initial custom suffixes create lexer ambiguity  (F015)

**Problem.** Suffixes are "one or more identifier-continue characters" (which includes digits), so
a digit-initial suffix makes `<NumericLiteral><suffix>` tokenization ambiguous (`4402k`: is it
`440`·`2k` or `4402`·`k`?).

**Context.** §3.9.1 admits digit-first suffixes (`years_2k` example); §3.9.3 gives no
maximal-munch/boundary rule. §1.4 forbids digit-initial *identifiers*, but a registered suffix is
governed by the looser identifier-continue rule.

**Potential solutions.** (B, lean) Forbid digit-initial suffixes — a suffix must begin with an
identifier-*start* character. Costs nothing real (`years_2k` still works; only a literal `2k`
suffix is disallowed) and makes tokenization unambiguous. (A) Allow them (lexer undefined).

**What.** LOG amend entry 495; SPEC §3.9.1 add the identifier-start constraint.

**Why.** Lexer determinism for custom suffixes.

**Refs.** F015 · §3.9.1, §3.9.3 · CL-GRAMMAR · OPEN, lean B.

-----------

## 19. [editorial · standalone] Inferred type of an integer literal under a custom suffix  (F016)

**Problem.** A §3.9 comment types `440hz` as an "i64 literal," but the `Numeric`-bound constructor
defaults the literal to `i32`.

**Context.** `from_hz[N: Numeric](n: N)`; `Numeric` carries `@default(i32)`. With no other context,
`440` resolves to `i32`, not `i64`. The wrapped type `Frequency` wraps `i64`, but the literal
*argument* is what the comment annotates.

**Potential solutions.** (B, lean) Editorial: the literal is `i32` (Numeric default), widened to
`i64` inside the constructor; fix the misleading comment.

**What.** SPEC §3.9 fix the comment; LOG none.

**Why.** Corrects a misleading comment; no rule change.

**Refs.** F016 · §3.9, §3.6.1 · OPEN, lean B (editorial).

-----------

## 20. [editorial · standalone] Single-argument calls: §3.5.1 "both forms" vs §3.5.3 constraints  (F018)

**Problem.** §3.5.1's "both forms valid for any single-argument call … no special rule restricts
single-argument calls" reads as permitting `Newtype(value: x)` and positional single-field record
construction, contradicting §3.5.3's per-callable constraints.

**Context.** §3.5.1's example `square` is a *function* (both forms allowed). §3.5.3 sets per-callable
rules: records always named, tuples/newtypes always positional. The "no special rule" sentence is
about *arity*, not per-callable form.

**Potential solutions.** (B, lean) §3.5.1 denies only an *arity*-based restriction; §3.5.3's
per-callable constraints still govern (`Newtype(value: x)` is illegal; positional single-field
record construction is illegal).

**What.** SPEC §3.5.1 narrow "no special rule" to "no *arity*-based rule"; LOG none.

**Why.** Legality of named newtype / positional single-field-record construction.

**Refs.** F018 · §3.5.1, §3.5.3 · OPEN, lean B (editorial).

-----------

## 21. [editorial · CL-GRAMMAR] `<NumericLiteral>` vs `<NumberLiteral>` nonterminal naming  (F019)

**Problem.** §3.9 names the suffixed-token nonterminal `<NumericLiteral>`; §3.9.3 writes
`<NumberLiteral>`. No two distinct nonterminals are defined.

**Context.** Pure naming inconsistency; the log (entry 492) uses `<NumericLiteral>`.

**Potential solutions.** (A, lean) Standardize on `<NumericLiteral>`; change §3.9.3.

**What.** SPEC §3.9.3 `<NumberLiteral>` → `<NumericLiteral>`; LOG none.

**Why.** Grammar cross-reference consistency.

**Refs.** F019 · §3.9, §3.9.3 · CL-GRAMMAR · OPEN, lean A (editorial).

-----------

## 22. [lean · standalone] `@literal_suffix` decorates the type vs the constructor  (F021)

**Problem.** `@literal_suffix` appears above both the type declaration (§3.9) and the constructor
function (§3.9.2) — which is the attachment site?

**Context.** §3.9.1: "Multiple `@literal_suffix` annotations may decorate one type." §3.9.2 places
it above `fn from_hz…`. §3.9.2's return-type rule ("returns the annotated type") presumes a known
attachment site.

**Potential solutions.** (A, lean) It attaches to the **type** declaration; the §3.9.2 placement is
presentational shorthand co-locating it with the named constructor. (B) It decorates the constructor.

**What.** LOG add the attachment rule; SPEC §3.9.2 move the example onto the type or add a note.

**Why.** Defines "the annotated type" in §3.9.2; low stakes (same registration either way).

**Refs.** F021 · §3.9.1, §3.9.2 · OPEN, lean A.

-----------

## 23. [lean · CL-UMBRELLA] `Float` umbrella requires-list omits `Log` and the inspection traits  (F024)

**Problem.** §4.8.2 says `Float` "requires all" float-only ops, but §4.9.2's `Float` requires-list
omits two-argument `Log` and the inspection traits (`is_nan`/`is_infinite`/`is_finite`/`is_normal`).

**Context.** Generic `T: Float` code cannot call `log(base, x)` or `is_nan` without extra bounds,
contradicting §4.8.2. The ops are auto-implemented for `f32`/`f64` regardless (§4.9.4).

**Potential solutions.** (A, lean) Add `Log` + the four inspection traits to `Float`'s requires-list.
Low design stakes (float-only by definition).

**What.** LOG amend the `Float`-requires entry; SPEC §4.9.2 insert `Log` and the four inspection
traits (and their `trait` decls into §4.9.1's elided "…and so on").

**Why.** Lets `T: Float` code use logs/inspection without extra bounds.

**Refs.** F024 · §4.8.2, §4.9.2 · CL-UMBRELLA · OPEN, lean A.

-----------

## 24. [lean · standalone] `/` listed among trapping operators is integer-scoped  (F027)

**Problem.** §4.6.1 lists `/` among default operators that "trap on overflow in all modes," but `/`
always produces Float, and float overflow yields signed infinity without trapping.

**Context.** §4.4.1.1: `/` always Float; §4.6.6/§4.6.2: float overflow → ±Infinity, no trap. So
listing `/` under trap-on-overflow is misleading (not behavior-changing).

**Potential solutions.** (B, lean) The trap rule is integer-scoped (`+`,`-`,`*`,`\`,`%`, unary `-`);
`/` is governed by §4.6.6.

**What.** LOG amend entry 659; SPEC §4.6.1 qualify the operator list (drop `/` or footnote §4.6.6).

**Why.** Removes a misleading inclusion; ties to F028.

**Refs.** F027 · §4.6.1, §4.6.6, §4.6.2 · OPEN, lean B.

-----------

## 25. [lean · standalone] `5 / 0`: trap vs `f64` Infinity  (F028)

**Problem.** §4.6.7 says "integer division by zero traps," naming `/` — but `/` widens to float
first, so `5 / 0` is `5.0 / 0.0` = `+Infinity`.

**Context.** §4.4.1.1: `/` always Float; §4.6.4's `/?` already presumes `5/0` is a float Infinity.
The log (entry 685) already dropped `/` and says only "Integer division by zero traps."

**Potential solutions.** (B, lean) `5 / 0` = `f64::INFINITY`; only `\` and `%` by zero trap.

**What.** LOG amend entry 685; SPEC §4.6.7 scope the zero-trap to `\`/`%`, note `/` by zero →
±Infinity.

**Why.** Result of `5 / 0`; forced by the always-float rule.

**Refs.** F028 · §4.6.7, §4.4.1.1, §4.6.4 · OPEN, lean B.

-----------

## 26. [editorial · standalone] `let x: Drivable` error phase: parse vs post-resolution  (F030)

**Problem.** §5.2.2 calls a bare trait at value position a "parse error," but the trigger
("`Drivable` is a trait, not a type") is decidable only after name resolution.

**Context.** A grammar cannot know `Drivable` is a trait; the `dyn`-required diagnostic belongs to
the resolver.

**Potential solutions.** (B, lean) Reclassify as a compile-time (post-name-resolution) error.

**What.** LOG amend entries 781/782; SPEC §5.2.2 replace "parse error" with "compile-time error
(after name resolution)."

**Why.** Correct diagnostic class.

**Refs.** F030 · §5.2.2 · OPEN, lean B (editorial).

-----------

## 27. [lean · standalone] Record-intersection `@derive` when both operands implement the trait  (F031)

**Problem.** `@derive(Trait)` on `type T = A & B` delegates to an operand's `fulfill`, but
"when both … would equally apply, derivation is ambiguous" is undefined, and no rule composes two
impls.

**Context.** §5.3.2 (record-intersection derive), §3.8. The ambiguity clause is in SPEC prose, not
serialized. Example: `@derive(Display, Hash)` on `type InsuredCar = Car & Insured` when both
implement.

**Potential solutions.** (A, lean) Exactly one operand implements ⇒ delegate; both ⇒ always
ambiguous (compile error), user writes the impl. Drop "would equally apply." (B) Define a
composition test (no semantics exist for merging impls).

**What.** LOG add rule A; SPEC §5.3.2 replace the vague qualifier.

**Why.** Whether a common derive pattern compiles or errors.

**Refs.** F031 · §5.3.2, §3.8 · OPEN, lean A.

-----------

## 28. [editorial · standalone] `with`-override conversions cite §6.1.3, which states none  (F033)

**Problem.** §6.1.5 says override values are "subject to the same widening and conversion rules as
direct construction per §6.1.3," but §6.1.3 states no widening/conversion rules.

**Context.** The log (entry 872) already keeps only "Override values must be type-compatible." The
defect is the dangling citation; implicit widening lives in §4.5, explicit conversions in §4.7.

**Potential solutions.** (A, lean) The citation is misdirected; redirect to §4.5 (implicit
widening) / §4.7 (explicit `T(x)`).

**What.** SPEC §6.1.5 change the citation; LOG none.

**Why.** Which conversions a `with` override accepts.

**Refs.** F033 · §6.1.5, §6.1.3, §4.5, §4.7 · OPEN, lean A (editorial).

-----------

## 29. [lean · standalone] Scope of the "at most one `satisfies` clause" rule  (F036)

**Problem.** §6.1.1 says a record body has "at most one" `satisfies` clause, but §6.3.1 says a
newtype body "may include `satisfies` clauses" (plural).

**Context.** §6.3.1's own example shows exactly one `satisfies`; §6.2.2 gives enums a singular
clause. The plural is the lone divergence.

**Potential solutions.** (A, lean) One `satisfies` clause per body for all nominal kinds (multiple
traits comma-separated on that line); `@derive`-implied conformances are separate.

**What.** LOG amend entry 939; SPEC §6.3.1 change "clauses" to "one clause (listing one or more
traits)."

**Why.** Whether two `satisfies` lines in one body compile.

**Refs.** F036 · §6.1.1, §6.3.1, §6.2.2 · OPEN, lean A.

-----------

## 30. [editorial · standalone] "`match` and `?` are the complete surface" vs `!` and stdlib methods  (F040)

**Problem.** §8.3.1 calls `match` + `?` "the complete surface for consuming `Option` and `Result`,"
but the postfix `!` operator (§8.4.2) and stdlib methods (§8.7) also consume them.

**Context.** `!` is a language operator that post-dates the sentence; in context the claim is about
*minimal pattern/short-circuit sugar* ("no `if let`"), not a normative exclusion.

**Potential solutions.** (A, lean) Soften the wording: `match` and `?` are the *primary* forms;
`!` and stdlib methods also consume.

**What.** LOG amend entry 1040; SPEC §8.3.1 reword "complete surface" → "primary … forms."

**Why.** Whether §8.3.1 normatively excludes further consumers.

**Refs.** F040 · §8.3.1, §8.4.2, §8.7 · OPEN, lean A (editorial).

-----------

## 31. [lean · CL-DECLKINDS] Module-targeting `use`: legal or not?  (F045)

**Problem.** A `use` whose path leaf is a module — legal or rejected?

**Context.** §10.4: `use` "imports a name from a module." §10.4.1 shows `use root::audio::(synth::
Oscillator, …)` reaching *through* sub-modules to a name. The quarantined `✗` example
(`use root::audio::effects`, "effects is not a module") is consistent with "leaf must be a name."

**Potential solutions.** (B, lean) The leaf must be an importable declaration name; intermediate
segments may be modules; a `use` targeting a module is a compile error (`synth::Oscillator`
qualification still works without importing `synth`).

**What.** LOG add the leaf-must-be-a-name rule; SPEC §10.4 one sentence.

**Why.** Legality of `use root::audio::synth`.

**Refs.** F045 · §10.2, §10.4, §10.4.1 · CL-DECLKINDS · OPEN, lean B. (Also a Section-3 dup —
resolve here.)

-----------

## 32. [editorial · CL-ASCAST] `duration::`/`instant::` type-qualified call form  (F046)

**Problem.** `duration::from_nanos(n)` / `instant::now()` look like a type-qualified free-function
call, which §8.7 explicitly forbids.

**Context.** §8.7: "There is no `Option::unwrap(option)` (type-qualified) form." §10.2.3's closed
`PathBase` set has no primitive-type base. So these are illustrative names for stdlib constructors.

**Potential solutions.** (A, lean) They are illustrative; the real surface is a module path
(`std::duration::from_nanos`) or a method. Rewrite §9.4.1.4/§9.4.2.2 accordingly.

**What.** LOG amend #1215/#1230 (illustrative, cite §8.7); SPEC §9.4.1.4/§9.4.2.2 rewrite in
module-path/method form.

**Why.** Avoids implying a banned syntax.

**Refs.** F046 · §9.4.1.4, §9.4.2.2, §8.7, §10.2.3 · CL-ASCAST · OPEN, lean A (editorial).

-----------

## 33. [lean · CL-LOOPFORMS] `for own` over a Copy-element aggregate: source usable after?  (F061)

**Problem.** `for own` over a Copy-*element* but non-Copy *aggregate* (e.g. `f32[1024]`) — is the
source usable after the loop?

**Context.** §12.3.3 calls the case "functionally indistinguishable from the default form" (under
which the source survives), yet the example comments "buf is consumed … cannot be used after." But
arrays are not Copy (§11.6.1), and `for own` dispatches `consuming_iterator(move buf)`, consuming
the array.

**Potential solutions.** (A, lean) `for own` over a non-Copy aggregate consumes the source; it is
unusable after. The indistinguishability holds only when the *source type itself* is Copy (e.g.
`Range[T]`).

**What.** LOG add the rule; SPEC §12.3.3 scope the indistinguishability claim.

**Why.** Source liveness after `for own`.

**Refs.** F061 · §12.3.3 · CL-LOOPFORMS · OPEN, lean A.

-----------

## 34. [editorial · standalone] Footnote over-permits >16-byte direct multi-cell spanning  (F099)

**Problem.** §13.12.4's footnote ("larger values span multiple consecutive cells … or use pool
storage") reads as permitting unbounded direct multi-cell spanning, contradicting the body rule.

**Context.** Body: direct storage only ≤ atomic word; two-cell direct is a backend choice for
9–16 bytes only; otherwise pool. The cell-fit table sends a 40-byte record to pool.

**Potential solutions.** (A, lean) Bound direct multi-cell to the 9–16-byte two-cell option;
everything larger is pool.

**What.** SPEC §13.12.4 reword the footnote; LOG none.

**Why.** Whether a >16-byte type may be stored directly across cells.

**Refs.** F099 · §13.12.4 · OPEN, lean A (editorial).

-----------

## 35. [editorial · standalone] Wrong internal cross-ref "§13.10.1" for dirty propagation  (F101)

**Problem.** §13.12.1 cites "the dirty-bit propagation in §13.10.1," but propagation is defined in
§13.10.2 step 1.

**Context.** §13.10.1 (lazy writes) states no propagation rule. No conformance divergence — both
describe the same commit-time model.

**Potential solutions.** (A, lean) Fix the pointer to §13.10.2 step 1.

**What.** SPEC §13.12.1 change the cross-reference; LOG none.

**Why.** Correct internal reference.

**Refs.** F101 · §13.12.1, §13.10.1, §13.10.2 · OPEN, lean A (editorial).

-----------

## 36. [lean · CL-DROP] Is the overwritten-assignment-target's previous-value drop specified?  (F130)

**Problem.** §14.7.2 says the compiler "inserts a drop call at the point of consumption (move into
a function parameter or assignment)," but a move into a parameter inserts no drop (the callee owns
it) — the rule conflates two things.

**Context.** "The moved-out source's drop slot is empty thereafter" = transfer of responsibility
(no drop runs on the moved value). At an *assignment* `x = new`, the *overwritten previous value*
of `x` is dropped there. The four drop points don't cleanly name the overwritten-previous-value drop.

**Potential solutions.** (B, lean, with carve-out) Split the bullet: (a) a move transfers drop
responsibility (no call at the move site); (b) an assignment drops the binding's previous value at
the assignment point.

**What.** SPEC §14.7.2 split the bullet; LOG amend the §14.7.2 entry.

**Why.** Specifies the overwritten-target drop; removes the move/assignment conflation.

**Refs.** F130 · §14.7.2 · CL-DROP · OPEN, lean B.

-----------

## 37. [lean · CL-DROP] Are recurrent/stream cells dropped on instance removal?  (F131)

**Problem.** §14.7.5 says only `attr` and `derived` cells receive Drop on removal — leaving
recurrent and stream-metadata teardown unspecified (resource-leak exposure).

**Context.** §14.4 lists recurrent/stream as instance reactive state; §14.8.1 step 3b says "for
each removed cell: invoke drop per §14.7" (unqualified). So "attr and derived" is under-inclusive.

**Potential solutions.** (B, lean) All of a removed instance's cells (attr, recurrent, derived,
stream metadata) are dropped per their Drop impls.

**What.** SPEC §14.7.5 change "attr and derived" → "all of its cells …"; LOG add the rule.

**Why.** Closes a leak under the literal reading.

**Refs.** F131 · §14.7.5, §14.4, §14.8.1 · CL-DROP · OPEN, lean B.

-----------

## 38. [lean · CL-IR] Does an effect-position `|>` emit a `connection` entry?  (F139)

**Problem.** The §15.4.1 desugar map says "connection/`|>` → `connection`" (unqualified), but the
worked example lowers an effect-position `|>` to `parameter_bindings` with no connection entry.

**Context.** §15.4.5 lowers `label |> print` to `effect App.print:0 … params [message: App.label]`
and emits `scope App exposes [] effects [App.print:0]` — `exposes` empty, no connection entry.

**Potential solutions.** (B, lean) An effect-position `|>` lowers to the effect's
`parameter_bindings`, not a `connection`; only topological `|>`/connections produce `connection`
entries.

**What.** SPEC §15.4.1 qualify the desugar map; LOG add the rule.

**Why.** Affects graph contents, `exposes` traversal, and hot-reload diffing.

**Refs.** F139 · §15.4.1, §15.4.5 · CL-IR · OPEN, lean B.

-----------

# Section 3 — Open syntax questions (grammar-class)

11 items the SPEC decides nowhere. The mined legacy grammar likely answers most; each needs a nod
(answer directly, or approve a draft-from-legacy-grammar pass). On ruling: add a DECISION_LOG
entry + write the governing prose into SPEC, then strike from the spec-silent appendix.

## 39. [open-syntax · CL-GRAMMAR] Forced identifier-suffix name set  (appendix)

**Problem.** A forced (identifier-)suffix on a literal — is the name set exactly the primitive type
names, or also `alias type` names (e.g. `255_byte`)?

**Context.** §3.9 (literal suffixes). The sweep flags the forced-suffix name set as undecided.

**Potential solutions.** (A) Primitive type names only. (B) Also `alias type` names.

**What.** A DECISION_LOG entry fixing the forced-suffix name set; a §3.9 note.

**Why.** Determines which `<int>_<name>` forms lex as forced suffixes.

**Refs.** Spec-silent appendix · §3.9 · CL-GRAMMAR · OPEN.

-----------

## 40. [open-syntax · CL-GRAMMAR] String interpolation format specifiers + `\xHH` range  (appendix, ◐)

**Problem.** Two string sub-grammar residuals: (1) format specifiers inside `{…}` interpolations
are undefined; (2) the admissible `\xHH` byte-vs-scalar range vs the UTF-8/scalar invariants is
unsettled.

**Context.** §9.1 (string literals). Marked partial (◐) in the appendix — interpolation and escape
basics are settled; these two are not.

**Potential solutions.** Define a format-spec mini-grammar inside `{…}` (or disallow it); fix
`\xHH` to a scalar-valid range (or allow raw bytes only in a byte-string form).

**What.** DECISION_LOG entries + §9.1 prose for both.

**Why.** Interpolation/escape determinism; affects every formatted string.

**Refs.** Spec-silent appendix (◐) · §9.1 · CL-GRAMMAR · OPEN.

-----------

## 41. [open-syntax · CL-GRAMMAR] Array repeat-count form + `Vec[…] = []` initializer  (appendix)

**Problem.** Array *literal* `[e1,…,eK]`→`T[K]` and empty `[]` are settled (F047), but a
repeat-count form (`[e; N]`) and the `Vec[…] = []` cell-initializer are not.

**Context.** §9.3 (arrays), §13 examples use `attr posts: Vec[PostData] = []`. F047 defined the
list/empty forms; the repeat-count form and `Vec` literal sugar were left open.

**Potential solutions.** (A) Define `[e; N]` (repeat `e` N times → `T[N]`); decide whether `[]` for
a `Vec` cell is language sugar or stdlib. (B) No repeat form; `Vec` literal stays stdlib.

**What.** DECISION_LOG entry + §9.3 "Array literals" addition (or stdlib cross-ref).

**Why.** Every fixed-array and Vec-cell initializer example depends on it.

**Refs.** Spec-silent appendix · §9.3 (extends F047) · CL-GRAMMAR · OPEN.

-----------

## 42. [open-syntax · CL-GRAMMAR] Turbofish for enum variants  (appendix)

**Problem.** Where does `::[…]` attach on `Enum::Variant`, and how is a unit variant explicitly
instantiated with no inference context (`Option::None`)?

**Context.** §6.2 (enums), §5.7/turbofish. The variant turbofish attachment + unit-variant explicit
instantiation are undefined.

**Potential solutions.** (A) `Enum::[T]::Variant` (type args on the enum). (B) `Enum::Variant::[T]`
(on the variant). Decide unit-variant explicit form.

**What.** DECISION_LOG entry + §6.2 prose.

**Why.** Explicit instantiation of generic enums in no-context positions.

**Refs.** Spec-silent appendix · §6.2 · CL-GRAMMAR · OPEN.

-----------

## 43. [open-syntax · CL-GRAMMAR] Value-position `dyn` operand extent/precedence  (appendix)

**Problem.** F037 settled the admitted *positions* of value-position `dyn`, but the operand
extent/precedence (how much of the following expression `dyn` binds) is open.

**Context.** §5.2.5. The "mirrors `move`" hint was ruled rhetorical (F037), so it fixes no operand
grammar.

**Potential solutions.** (A) `dyn` binds a single primary expression (parenthesize for more).
(B) Binds to a precedence tier (define which).

**What.** DECISION_LOG entry + §5.2.5 operand-extent note.

**Why.** Parse of `dyn expr` in argument/let-RHS/collection positions.

**Refs.** Spec-silent appendix · §5.2.5 (extends F037) · CL-GRAMMAR · OPEN.

-----------

## 44. [open-syntax · CL-GRAMMAR] `Type[…]` with a conjunction constraint  (appendix)

**Problem.** May a `Type[…]` meta-type carry a conjunction constraint (`Type[Drivable & Insurable]`)?

**Context.** §5.7 (`Type[…]`), §5.1 (`&` intersection). Conjunction in `Type[…]` position is
unspecified.

**Potential solutions.** (A) Permit `Type[A & B]` (intersection constraint). (B) Single-trait only.

**What.** DECISION_LOG entry + §5.7 prose.

**Why.** Expressiveness of compile-time type-value constraints.

**Refs.** Spec-silent appendix · §5.7, §5.1 · CL-GRAMMAR · OPEN.

-----------

## 45. [open-syntax · CL-GRAMMAR] Newtype constructor pattern vs `T(value)` sole eliminator  (appendix)

**Problem.** Is there a newtype destructuring *pattern* (`UserId(n)` in a `match`), or is `T(value)`
extraction the sole eliminator?

**Context.** §6.3 (newtypes). Construction/extraction share `T(value)` (scalar-only extraction,
per F034), but a *pattern* form for binding the wrapped value is undefined.

**Potential solutions.** (A) Admit `UserId(n)` patterns (mirror tuple/enum patterns). (B) No
pattern; extraction via `T(value)` / accessor only.

**What.** DECISION_LOG entry + §6.3 prose.

**Why.** Whether newtypes are destructurable in `match`.

**Refs.** Spec-silent appendix · §6.3 (adjacent F034) · CL-GRAMMAR · OPEN.

-----------

## 46. [open-syntax · CL-GRAMMAR] `with` grammar extent  (appendix)

**Problem.** The `with` override form is shown only as a single override; chaining
(`a with x: 1 with y: 2`), precedence, and use inside a call-argument list are undefined.

**Context.** §6.1.5 (`with`), §6.1.3 (construction). No grammar production defines `with`'s extent.

**Potential solutions.** (A) Left-associative chaining; `with` binds looser than construction;
permitted inside call args with explicit parens. (B) Single-override only.

**What.** DECISION_LOG entry fixing associativity/precedence/positions; a §6.1.5 note.

**Why.** Affects every override site.

**Refs.** Spec-silent appendix · §6.1.5, §6.1.3 · CL-GRAMMAR · OPEN.

-----------

## 47. [open-syntax · CL-GRAMMAR] Inline `observe` delimiting + multi-line arm bodies  (appendix)

**Problem.** How is an inline `observe` delimited as a sub-expression / call argument, and may an
`observe` arm body span multiple lines (only single-expression arms are shown)?

**Context.** §13.2.11 (`observe`). Only single-line arm bodies and top-level `observe` appear.

**Potential solutions.** (A) Inline `observe` requires parentheses; arm bodies may be indented
blocks. (B) `observe` is statement-position only; arms single-expression.

**What.** DECISION_LOG entry + §13.2.11 prose.

**Why.** Whether `observe` composes inside expressions and carries block-bodied arms.

**Refs.** Spec-silent appendix · §13.2.11 · CL-GRAMMAR · OPEN.

-----------

## 48. [open-syntax · CL-GRAMMAR] Inline-after-colon body for operators  (appendix, ◐)

**Problem.** Declarations were normalized to multi-line bodies, but whether an operator may carry an
inline single-member body after the colon (e.g. `operator gain[…]: …`) is unresolved.

**Context.** §13.17 (operators), §13.17.2 (operator bodies). The inline-after-colon question was
settled for declarations but left open for operators (◐).

**Potential solutions.** (A) Inline single-member operator bodies are legal (compact form). (B)
Operator bodies must be indented blocks.

**What.** DECISION_LOG entry + §13.17.2 prose.

**Why.** Permits/forbids the compact operator form spec-wide.

**Refs.** Spec-silent appendix (◐) · §13.17.2 · CL-GRAMMAR · OPEN.

-----------

## 49. [open-syntax · CL-GRAMMAR] Connection-body surface details  (appendix)

**Problem.** Several connection-body surface points are undefined: clause-ordering of
`from:`/`to:`/`pairs:` vs members; explicit type-arg surface when placing a generic connection;
whitespace significance around `/` in the `/expr` slot; `:` placement when a multi-line attr
continuation meets a child body; whether an unparenthesized whitespace-bearing attr value is an
error; exhaustiveness of the pairs-form `match pair:`.

**Context.** §13.6 (connections), §13.8 (placement). These are the residual connection-surface
items from the syntax sweep.

**Potential solutions.** Settle each from the legacy grammar (clause order free vs fixed; explicit
type-arg surface; `/expr` whitespace rule; multi-line-attr `:` rule; parenthesization requirement;
pairs-form exhaustiveness).

**What.** DECISION_LOG entries + §13.6/§13.8 prose for each sub-point.

**Why.** Connection placement is core to the reactive surface; each example is currently
unverifiable on these points.

**Refs.** Spec-silent appendix · §13.6, §13.8 · CL-GRAMMAR · OPEN (multi-part).

-----------

# Section 4 — Open semantic questions (grammar won't help)

4 items needing real rulings, not syntax decisions.

## 50. [open-semantic] Conditional `Copy` impl surface a generic type writes  (appendix)

**Problem.** How does a generic type conditionally implement `Copy` — `fulfill Copy for G[T] where
T: Copy`, and may the body be empty?

**Context.** §3.3.4's `where`-form is specified only for method-bearing traits; `Copy` is a marker
(methodless) trait, so the conditional-impl surface for it is undefined.

**Potential solutions.** (A) Permit `fulfill Copy for G[T] where T: Copy` with an empty body. (B)
`Copy` is auto-derived from field Copy-ness only; no manual conditional impl.

**What.** DECISION_LOG entry + §3.3.4/§11.12 prose.

**Why.** Whether generic containers can be conditionally Copy.

**Refs.** Spec-silent appendix · §3.3.4, §11.12 · OPEN.

-----------

## 51. [open-semantic] Multi-segment assignment LHS + desugar order  (appendix)

**Problem.** Only single-segment place assignments are shown (`r.field = v`, `arr[i] = v`).
Multi-segment LHS (`r.a.b = x`, `arr[i].field = y`) and the FieldAssign/IndexAssign desugaring
order are undefined.

**Context.** §11.11 (place assignment) defines fields (#1431) and indices (#1432) only.

**Potential solutions.** (A) Admit multi-segment LHS with a defined left-to-right desugar order.
(B) Single-segment only; deeper updates via re-construction.

**What.** DECISION_LOG entry + §11.11 prose (and the desugar order).

**Why.** Whether nested in-place updates are expressible.

**Refs.** Spec-silent appendix · §11.11 · OPEN.

-----------

## 52. [open-semantic] Tuple-component assignability through a `mut` binding  (appendix)

**Problem.** May a tuple component be assigned through a `mut` binding, and what is its LHS form
(`t.0 = x`)?

**Context.** §11.11 (place assignment), §11.12. Record fields are assignable; tuple components are
not addressed.

**Potential solutions.** (A) Admit `t.0 = x` through a `mut` tuple binding. (B) Tuples are
immutable-component; whole-value reassignment only.

**What.** DECISION_LOG entry + §11.11/§11.12 prose.

**Why.** Whether tuples support in-place component update.

**Refs.** Spec-silent appendix · §11.11, §11.12 · OPEN.

-----------

## 53. [open-semantic] Explicitly-written elaborated borrow signatures  (appendix)

**Problem.** The spec gives only a schematic for elaborated borrow signatures
(`fn f(borrow v: T) -> borrow_rooted_in(v) T`); the concrete writable surface is undefined.

**Context.** §11.x (borrow elaboration). Only "Schematically: …" is given; whether/how a user writes
such a signature is open.

**Potential solutions.** (A) Define a concrete surface for elaborated borrow signatures. (B) They
are spec-internal notation only; never written in source (borrow is always implicit).

**What.** DECISION_LOG entry + §11 prose (or an explicit "not surface-writable" rule).

**Why.** Whether borrow rooting is ever user-written; ties to F052 (`&T` is not surface).

**Refs.** Spec-silent appendix · §11 (adjacent F052) · OPEN.

-----------

# Section 5 — Doc hygiene

## 54. [doc-hygiene] Reconcile `RULINGS.md` worksheet-vs-applied-log skew + `FINDINGS.md` staleness  (DH-1)

**Problem.** `RULINGS.md` has two layers that disagree: the per-finding worksheets are the original
triage (their OPEN counts sum to the "86 OPEN" snapshot), while the "Rulings log (applied)" records
later sessions that struck findings to the current 61. So worksheet verdicts are current only for
the 61 live findings, and their line numbers have **drifted** (confirmed: F129's cited
"§14.6.3 line 21042" now lands in the string-pool section). `FINDINGS.md` likewise carries stale
GRAMMAR.md/worksheet annotations (handled in part by ticket Part D of the immediate plan).

**Context.** Anyone reading the worksheets or `FINDINGS.md` at face value is misled about what is
open and where it lives.

**Potential solutions.** A reconciliation pass: re-audit the 61 live findings against current SPEC,
refresh `§` line refs (or drop line numbers in favor of quoted content), and clearly mark struck
findings as struck in the worksheets.

**What.** Edit `RULINGS.md` (and any residual `FINDINGS.md` annotations) to reflect the applied
sessions and current SPEC.

**Why.** Prevents future work from re-litigating already-ruled findings or chasing drifted line
numbers.

**Refs.** DH-1 · `RULINGS.md`, `DECISION_LOG_FINDINGS.md` · OPEN.

-----------

## Moot (no action — recorded for completeness)

- **F017** (§3.9.1/§3.9.4 reserved-suffix prohibition: unconditional vs same-scope) — moot;
  duration suffixes are globally visible (§3.9.5), so the "same scope" qualifier never narrows
  anything. Optional editorial tidy only.
- **F053** (§11.10.2 "regardless of root Copy" qualifier) — moot; §11.12 forbids a Copy compound
  with a non-Copy subvalue, so the qualifier is harmlessly vacuous. Optional trim only.
