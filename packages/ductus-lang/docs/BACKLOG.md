# Ductus — Decision Backlog (rulings & approvals only)

Open items that need an **owner decision**: genuine design forks, plus syntax/semantic questions
the spec decides nowhere. This file deliberately holds *only* what requires a ruling or your
approval of a strong recommendation.

Items that only need batch-approval (the 28 lean/editorial fixes) or are pure execution (the
doc-hygiene reconciliation, the moot no-ops) have been **moved into the implementation plan**
(Phase 5 and Part D respectively) and are not duplicated here.

**Source & caveat.** Tickets are mined from the `RULINGS.md` per-finding worksheets and the
`DECISION_LOG_FINDINGS.md` spec-silent appendix. Those worksheets are substance-current for
these (still-live) findings, but their **line numbers have drifted**; section numbers (`§x.y`)
are stable. Locate any target by `§` + quoted content, not by line number.

**Contents (15 items):** Section 1 — design forks (6 hard forks) · Section 2 —
open syntax questions (6) · Section 3 — open semantic questions (3).

**How to use.** Forks (Section 1) need a real decision — open them one at a time, with discussion.
Sections 2–3 can be answered directly or delegated to a draft-from-legacy-grammar pass for your
ratification.

Legend — tier: `foundational` (reshapes a subsystem) · `cluster-root` (cascades to siblings) ·
`leaned-reshape` (clear recommendation, but it changes a rule, so it needs your nod) ·
`open-syntax` / `open-semantic` (spec-silent).

---

# Section 1 — Design forks (you decide)

Six hard forks (F129+F134 are one decision), no clean default. (The four leaned-reshape forks —
F008, F029, F035, F132 — and the conditional-`Copy` surface have since been ruled and applied; see
the implementation plan's Part F. They are no longer carried here.)

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

**Linked — F136 (deferred to this ticket).** F136 is resolved on the merits but cannot be applied
until this ticket lands. §15.4.5's worked example types the desired cell `App.print:0.text` as
`str`, yet its behavior `B@d5` returns the whole `%TextRec` record (per the §15.4.4 ABI: the
desired-builder returns "the desired record", DoR #4018); and by cell-type rule 033-70 an
aggregate cell must be a **dynamic-pool-index**, not a record tag. So the correct form is a single
whole-record pool-index desired cell — but the *text-form spelling* of a pool-index cell type is
undefined, which is exactly this ticket's missing graph grammar. **When F138 is decided, also fix
§15.4.5:** make `App.print:0` one whole-record pool-index `effect-desired` cell that the runtime
scatters into per-field desired state, and update the `desired [...]` reference. F136 remains in
the `FINDINGS.md` ledger until then.

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

# Section 2 — Open syntax questions (grammar-class)

6 items the SPEC decides nowhere. The mined legacy grammar likely answers most; each needs a nod
(answer directly, or approve a draft-from-legacy-grammar pass). On ruling: add a DECISION_LOG
entry + write the governing prose into SPEC, then strike from the spec-silent appendix.
(Round-1 bulk decisions of 2026-06-20 ruled & removed five tickets — array construction, enum
turbofish, value-position `dyn` extent, `Type[A & B]`, newtype patterns; see the implementation
plan's round-1 apply-pass.)

## 7. [open-syntax · CL-GRAMMAR] Forced identifier-suffix name set  (appendix)

**Problem.** A forced (identifier-)suffix on a literal — is the name set exactly the primitive type
names, or also `alias type` names (e.g. `255_byte`)?

**Context.** §3.9 (literal suffixes). The sweep flags the forced-suffix name set as undecided.

**Potential solutions.** (A) Primitive type names only. (B) Also `alias type` names.

**What.** A DECISION_LOG entry fixing the forced-suffix name set; a §3.9 note.

**Why.** Determines which `<int>_<name>` forms lex as forced suffixes.

**Refs.** Spec-silent appendix · §3.9 · CL-GRAMMAR · OPEN.

-----------

## 8. [open-syntax · CL-GRAMMAR] String interpolation format specifiers + `\xHH` range  (appendix, ◐)

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

## 9. [open-syntax · CL-GRAMMAR] `with` grammar extent  (appendix)

**Problem.** The `with` override form is shown only as a single override; chaining
(`a with x: 1 with y: 2`), precedence, and use inside a call-argument list are undefined.

**Context.** §6.1.5 (`with`), §6.1.3 (construction). No grammar production defines `with`'s extent.

**Potential solutions.** (A) Left-associative chaining; `with` binds looser than construction;
permitted inside call args with explicit parens. (B) Single-override only.

**What.** DECISION_LOG entry fixing associativity/precedence/positions; a §6.1.5 note.

**Why.** Affects every override site.

**Refs.** Spec-silent appendix · §6.1.5, §6.1.3 · CL-GRAMMAR · OPEN.

-----------

## 10. [open-syntax · CL-GRAMMAR] Inline `observe` delimiting + multi-line arm bodies  (appendix)

**Problem.** How is an inline `observe` delimited as a sub-expression / call argument, and may an
`observe` arm body span multiple lines (only single-expression arms are shown)?

**Context.** §13.2.11 (`observe`). Only single-line arm bodies and top-level `observe` appear.

**Potential solutions.** (A) Inline `observe` requires parentheses; arm bodies may be indented
blocks. (B) `observe` is statement-position only; arms single-expression.

**What.** DECISION_LOG entry + §13.2.11 prose.

**Why.** Whether `observe` composes inside expressions and carries block-bodied arms.

**Refs.** Spec-silent appendix · §13.2.11 · CL-GRAMMAR · OPEN.

-----------

## 11. [open-syntax · CL-GRAMMAR] Inline-after-colon body for operators  (appendix, ◐)

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

## 12. [open-syntax · CL-GRAMMAR] Connection-body surface details  (appendix)

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

# Section 3 — Open semantic questions (grammar won't help)

3 items needing real rulings, not syntax decisions.

## 13. [open-semantic] Multi-segment assignment LHS + desugar order  (appendix)

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

## 14. [open-semantic] Tuple-component assignability through a `mut` binding  (appendix)

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

## 15. [open-semantic · CL-OWNERSHIP] Optional surface form for borrow-rootedness (incl. union)  (reframed from old elaborated-borrow item)

**Problem.** Borrow-rootedness — which input cluster(s) a borrow-return is rooted in, including
the multi-root "union of clusters" — is a compile-time concept with observable effects (which
`move`s/mutations are rejected) but no surface form. It is the only inferred property in the
language with no optional explicit annotation (contrast types: inferred yet always annotatable).

**Context.** §11.7.5: the elaborated `borrow_rooted_in(v)` form is diagnostic-only ("users do not
write this"); rootedness is "inferred from the function body." DECISION_LOG 013-66: multiple
contributing inputs → union of clusters (conservative; locks all). For concrete functions
body-inference is precise. For abstract trait-method / fn-type returns there is no body, so the
system falls back to the conservative union — and a trait author cannot constrain rootedness even
though that is a genuine contract, not a derived fact. (Rust solves this with explicit lifetime
parameters — signature-local, mandatory-when-ambiguous; that syntax is explicitly off the table.)

**Potential solutions.**
- (A, owner-chosen direction) Add a **purely opt-in** rootedness annotation. Default stays
  body-inferred and invisible (unchanged); the annotation is available only to constrain/document
  rootedness — most usefully on abstract returns, the exact spot inference can't reach.
  **Concrete syntax TBD and explicitly NOT lifetime-style** — design deferred to when this item is
  opened.
- (B) Status quo: rootedness stays inexpressible; accept conservative union at the abstract
  boundary.

**What.** A DECISION_LOG entry + §11.7.5 prose introducing the optional annotation and its scope;
the surface syntax is a separate design pass (owner: not Rust-style).

**Why.** Closes a "no privileged thing the user cannot express" gap — the language reasons in
rootedness/union terms the user currently cannot write — without taxing concrete code.

**Refs.** reframed old elaborated-borrow item · §11.7.5, 013-66, 013-48 · CL-OWNERSHIP · OPEN
(owner: add it, opt-in only, body-inference stays; syntax deferred). Subsumes the old
"explicitly-written elaborated borrow signatures" question (F052-adjacent): the elaborated
`borrow_rooted_in(v)` form stays diagnostic-only; this opt-in annotation is the user-writable
surface, if any.
