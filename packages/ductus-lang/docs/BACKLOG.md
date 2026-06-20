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

**Contents (25 items):** Section 1 — design forks (10: 6 hard + 4 leaned-reshape) · Section 2 —
open syntax questions (11) · Section 3 — open semantic questions (4).

**How to use.** Forks (Section 1) need a real decision — open them one at a time, with discussion.
Sections 2–3 can be answered directly or delegated to a draft-from-legacy-grammar pass for your
ratification.

Legend — tier: `foundational` (reshapes a subsystem) · `cluster-root` (cascades to siblings) ·
`leaned-reshape` (clear recommendation, but it changes a rule, so it needs your nod) ·
`open-syntax` / `open-semantic` (spec-silent).

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

# Section 2 — Open syntax questions (grammar-class)

11 items the SPEC decides nowhere. The mined legacy grammar likely answers most; each needs a nod
(answer directly, or approve a draft-from-legacy-grammar pass). On ruling: add a DECISION_LOG
entry + write the governing prose into SPEC, then strike from the spec-silent appendix.

## 11. [open-syntax · CL-GRAMMAR] Forced identifier-suffix name set  (appendix)

**Problem.** A forced (identifier-)suffix on a literal — is the name set exactly the primitive type
names, or also `alias type` names (e.g. `255_byte`)?

**Context.** §3.9 (literal suffixes). The sweep flags the forced-suffix name set as undecided.

**Potential solutions.** (A) Primitive type names only. (B) Also `alias type` names.

**What.** A DECISION_LOG entry fixing the forced-suffix name set; a §3.9 note.

**Why.** Determines which `<int>_<name>` forms lex as forced suffixes.

**Refs.** Spec-silent appendix · §3.9 · CL-GRAMMAR · OPEN.

-----------

## 12. [open-syntax · CL-GRAMMAR] String interpolation format specifiers + `\xHH` range  (appendix, ◐)

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

## 13. [open-syntax · CL-GRAMMAR] Array repeat-count form + `Vec[…] = []` initializer  (appendix)

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

## 14. [open-syntax · CL-GRAMMAR] Turbofish for enum variants  (appendix)

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

## 15. [open-syntax · CL-GRAMMAR] Value-position `dyn` operand extent/precedence  (appendix)

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

## 16. [open-syntax · CL-GRAMMAR] `Type[…]` with a conjunction constraint  (appendix)

**Problem.** May a `Type[…]` meta-type carry a conjunction constraint (`Type[Drivable & Insurable]`)?

**Context.** §5.7 (`Type[…]`), §5.1 (`&` intersection). Conjunction in `Type[…]` position is
unspecified.

**Potential solutions.** (A) Permit `Type[A & B]` (intersection constraint). (B) Single-trait only.

**What.** DECISION_LOG entry + §5.7 prose.

**Why.** Expressiveness of compile-time type-value constraints.

**Refs.** Spec-silent appendix · §5.7, §5.1 · CL-GRAMMAR · OPEN.

-----------

## 17. [open-syntax · CL-GRAMMAR] Newtype constructor pattern vs `T(value)` sole eliminator  (appendix)

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

## 18. [open-syntax · CL-GRAMMAR] `with` grammar extent  (appendix)

**Problem.** The `with` override form is shown only as a single override; chaining
(`a with x: 1 with y: 2`), precedence, and use inside a call-argument list are undefined.

**Context.** §6.1.5 (`with`), §6.1.3 (construction). No grammar production defines `with`'s extent.

**Potential solutions.** (A) Left-associative chaining; `with` binds looser than construction;
permitted inside call args with explicit parens. (B) Single-override only.

**What.** DECISION_LOG entry fixing associativity/precedence/positions; a §6.1.5 note.

**Why.** Affects every override site.

**Refs.** Spec-silent appendix · §6.1.5, §6.1.3 · CL-GRAMMAR · OPEN.

-----------

## 19. [open-syntax · CL-GRAMMAR] Inline `observe` delimiting + multi-line arm bodies  (appendix)

**Problem.** How is an inline `observe` delimited as a sub-expression / call argument, and may an
`observe` arm body span multiple lines (only single-expression arms are shown)?

**Context.** §13.2.11 (`observe`). Only single-line arm bodies and top-level `observe` appear.

**Potential solutions.** (A) Inline `observe` requires parentheses; arm bodies may be indented
blocks. (B) `observe` is statement-position only; arms single-expression.

**What.** DECISION_LOG entry + §13.2.11 prose.

**Why.** Whether `observe` composes inside expressions and carries block-bodied arms.

**Refs.** Spec-silent appendix · §13.2.11 · CL-GRAMMAR · OPEN.

-----------

## 20. [open-syntax · CL-GRAMMAR] Inline-after-colon body for operators  (appendix, ◐)

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

## 21. [open-syntax · CL-GRAMMAR] Connection-body surface details  (appendix)

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

4 items needing real rulings, not syntax decisions.

## 22. [open-semantic] Conditional `Copy` impl surface a generic type writes  (appendix)

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

## 23. [open-semantic] Multi-segment assignment LHS + desugar order  (appendix)

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

## 24. [open-semantic] Tuple-component assignability through a `mut` binding  (appendix)

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

## 25. [open-semantic] Explicitly-written elaborated borrow signatures  (appendix)

**Problem.** The spec gives only a schematic for elaborated borrow signatures
(`fn f(borrow v: T) -> borrow_rooted_in(v) T`); the concrete writable surface is undefined.

**Context.** §11.x (borrow elaboration). Only "Schematically: …" is given; whether/how a user writes
such a signature is open.

**Potential solutions.** (A) Define a concrete surface for elaborated borrow signatures. (B) They
are spec-internal notation only; never written in source (borrow is always implicit).

**What.** DECISION_LOG entry + §11 prose (or an explicit "not surface-writable" rule).

**Why.** Whether borrow rooting is ever user-written; ties to F052 (`&T` is not surface).

**Refs.** Spec-silent appendix · §11 (adjacent F052) · OPEN.
