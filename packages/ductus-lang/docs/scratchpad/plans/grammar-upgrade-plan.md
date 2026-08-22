# GRAMMAR.md / IR_GRAMMAR.md Upgrade Plan — post-merge 4f62015

Status: DECISIONS LOCKED — Workflow B execution in progress.
Analysis baseline: merge commit `4f62015` on main (DECISION_LOG.md 4258 entries / 35 sections; SPEC.md ~24 879 lines).
Targets: `packages/ductus-lang/docs/GRAMMAR.md` (~6200 lines), `packages/ductus-lang/docs/IR_GRAMMAR.md` (535 lines), plus DECISION_LOG.md/SPEC.md for the PS conformance phase only.

## 1. Context

Main's "Interpretation RFC Amendment v2" was merged with this branch's session work (raw-string removal, if/else mandatory-else, assoc-`is`, where-tighter-than-`|>`, move-bare-IDENT, record shorthand, IR text-grammar formalization, TraitDecl BNF, column alignment). Main's rulings take precedence on every contested entry. Both grammar files were authored against the PRE-merge LOG/SPEC and are stale. Additionally, the pass-3 grammar remediation resolver (72 pass-2 findings) died on API overload — evidence shows it (or the f89c179 commit) applied a substantial fraction of the fixes before dying, so the backlog is in a "partially applied, unverified" state, not "unapplied".

**Scope ruling (user):** the upgrade covers EVERYTHING wrong or stale in the grammar, not just new LOG/SPEC content.

### Settled design decisions — implement, never re-ask

`where` tighter than `|>` (030-167); assoc-type `is` (D1); move operand bare-IDENT only (013-142, D2); record-pattern shorthand (006-33, D3); `Annotation*` wrapper-only — AnnotatedDecl is the sole attachment site (D4); OperatorType return does not admit `own` (D5, SPEC §13.17.13); `Annotation*` reaches effect-block cells (D6); single-suffix `here::`/`module::` (D7); raw strings DROPPED; main's rulings win wherever main changed a rule.

Verified already landed in GRAMMAR.md (no re-work, only verification): D1 (`AssocTypeDecl ::= 'type' IDENT ('is' TypeExpr)?`; `AssocTypeBinding`; `AssocTypeEqualityPredicate ::= AssocPath 'is' TypeExpr`), D2 (§5.4 move disambiguators, "bare IDENT only … per Phase D"), D3 (record shorthand cited at §4), D4 (TraitDecl carries no inline `Annotation*`), D6 (`AnnotatedDesiredCellDecl`/`AnnotatedObservedCellDecl`), D7 (`AnchorSuffix ::= IDENT`), raw strings absent.

### User-resolved design decisions (2026-08 session; LOCKED)

- **UD-1 (c)**: `collect`, `yield`, `fold`, `yielded` become RESERVED keywords (§2.4 blocks); `by` stays CONTEXTUAL (arm label only, parallel to `desired`/`observed` per 002-28).
- **UD-2 (a)**: kind annotations admitted at DEDICATED positions only — `AnnotationType ::= KindAnnotation | TypeExpr` at FnParam/FnReturn/OperatorParam/OperatorReturn/trait-method params+returns/effect params/observed-contract cells/reactive-decl annotation slots; `KindAnnotation` also admitted inside `GenericArgList` (`Portal[cell T]`). Everything else stays `TypeExpr` (`type R: f: cell i32` is a parse error).
- **UD-3 (a)**: maximal collect admission — `CollectExpr` joins the Expr grammar; `CollectStmt` (`collect as x:`) joins operator/node/effect-desired statement lists; collect body = enclosing body's statement grammar + `YieldStmt`; restrictions post-parse citing 034-4/034-7.
- **UD-4 (b)**: fold members operand is `PostfixExpr`.
- **UD-5 (a)**: fulfill-side effect-kind methods mirror the trait form — `FulfillBodyItem` gains `EffectMethodImpl ::= 'effect' IDENT '(' Params? ')' ':' EffectBody`.
- **UD-6a (yes)**: FLOAT stays in IR `value`; SPEC §15.4.6's `value` production gains FLOAT (LOG 033-212 already correct).
- **UD-6b (yes + conformance)**: `map.make`/`bundle.make`/`slice.make` in the IR op set; LOG 033-211 amended to match 033-165; SPEC §15.4.4 op production + prose conformed.
- **UD-6c (yes + conformance)**: operand admits bare-NAME parameter references; LOG 033-209 amended to `operand ::= '%'NAME | NAME | 'move' '%'NAME | 'move' NAME | literal`; SPEC §15.4.4 conformed (worked example §15.4.5 + 033-156 are normative evidence).
- **R1 (yes)**: SPEC §3.1 TraitDecl BNF + §3.3 FulfillItem BNF amended to include effect-kind methods per §3.1.1.1 / 005-68..70.
- **R2 (yes)**: LOG 033-121 "pairs" → triples with optional provenance marker per 033-64 + SPEC §15.4.1.
- **R3 (yes)**: LOG 030-56..58 bracketed spellings (`Derived[i64]` etc.) → kind-annotation spellings per SPEC §13.2.8.1.

## 2. Change inventory by staleness class

### C1 — Missing surface forms (zero productions today)

**C1a. `collect` / `yield` (SPEC §13.20, LOG 034-1..034-13).** No occurrence of `collect`/`yield` as syntax anywhere in GRAMMAR.md. Required surface:
- Block-expression form `collect:` INDENT body DEDENT — value is a `yielded T` group (034-1; SPEC §13.20.1 example: `derived voices: yielded Voice = collect:`).
- Statement form `collect as x:` — binds the group to a name (034-1).
- `yield <expr>` — one member-cell per yield; accepts ANY expression (034-2, 034-6); `yield` outside a `collect` block is a compile error (034-4).
- Body may contain: bare `yield`; `yield` under `repeat` (key-driven member); `yield` under gated `when`/`given` arms (activation-driven); `yield` under compile-time `if`/`match`/`for` (expanded; SPEC §13.20.1 shows `for v in candidates: yield v`); runtime `if`/`match` around `yield` is a compile error with a mandated diagnostic offering `yield if c: a else: b` (034-7, §13.20.3.1).
- `collect` is not a coroutine (034-5); membership is positional (034-3).
- Consumption restrictions (disambiguator/post-parse notes): no indexing, no structural `for` over `yielded`; consumers are fold, yielded-typed params (fn AND operator), and `repeat` (034-11); Iterable at language level, runtime loops only (034-12); synthesized `.count` member (034-13, §9.3.7 naming rule).

**C1b. `fold` (SPEC §13.21, LOG 035-1..035-10).** Required surface:
- `fold <members>:` NEWLINE INDENT `by:` Expr NEWLINE `else:` Expr DEDENT — arms newline-separated, no comma, both mandatory, `else` last (035-2; §13.21.1).
- `by:` takes an `fn(T, T) -> T` combiner expression (035-3); `else:` is the empty-membership result (035-4).
- `fold` is an expression form, not an operator/method (035-1); joins operator-body items and `derived` RHS (035-8; §13.17.4, §13.21.6); a fold in an operator body forces the indented block form (035-8).
- Foldable: `yielded` groups + reactive composites with uniform slot type (035-6); stored-collection-in-a-cell values are not foldable (035-7) — post-parse note.

**C1c. Effect-kind trait methods (SPEC §3.1.1.1, LOG 005-68..005-70, 005-51).** A trait may declare `effect name(params):` methods (first param conventionally `Subject`-typed) with an optional `observed:` contract block listing minimum required output cells (`audio: stream ring[256] Sample`) and optional default bodies. GRAMMAR §7.11 `TraitBodyItem` has no such alternative; §7.12 `FulfillBodyItem ::= Annotation* (FnDecl | AssocTypeBinding)` cannot express a fulfill of an effect-kind method. PS amends SPEC §3.1/§3.3 BNF, so the grammar can cite the amended BNF directly. Waiver rewording ripple: 005-67/005-71 (fulfill-without-satisfies waiver conditions) affects §7.11/§7.12 prose notes; 029-65/029-106 add a third `|>` RHS dispatch case (effect-kind trait method with node-reference LHS) — §5.25 and §8.11 disambiguator notes.

**C1d. Kind-annotation type family (SPEC §13.2.8, §13.2.8.1) — the largest single item.** The merged SPEC ABOLISHED the bracketed cell types: "there is no `Cell[T]` type and no `Cell` trait" (SPEC ~L12586); `Derived[`/`Signal[`/`Recurrent[` have ZERO occurrences in merged SPEC. Replaced by lowercase kind-led annotations usable in parameter positions, return positions, and generic arguments (§13.2.8; e.g. `operator passthrough[T](source: cell T) -> cell T`, `Portal[cell T]`):

| Annotation | Notes (SPEC §13.2.8.1 table) |
|---|---|
| `signal T` | attr also produces `signal T` |
| `derived T` | zero-history |
| `recurrent[N] T` | `N` omitted = `[1]` |
| `stream ring[N] T` / `stream gate[N] T` / erased `stream T` | event cell |
| `recurrent[H] stream ring[N] T` | policy × history |
| `yielded T` / `yielded f32[128]` | membership cell (§13.20.4) |
| `cell T` | umbrella KIND; never bracketed |
| `dynamic view V` | replaces `Cell[DynamicView[WeakHandle[V]]]` |

Kind annotations never carry inline bounds (bounds go in generic-param list / where clause). Storables KEEP brackets: `Type[C]`, `Handle[T]`, `WeakHandle[T]`, `Portal[T]`, `Stream[T, P]` (§13.18.11 policy-as-type; 030-261 aliases `RingStream[T,N]`/`GateStream[T,N]`). GRAMMAR.md has NO kind-annotation type productions and 13 stale `Cell[`/`Derived[`/`Signal[`/`Recurrent[`/`DynamicView[` sites (§5.24 note, §7.6/§7.13 examples, §7.14 region, §11.9 note, Appendix B.1 rows, B.3 `DynamicView[T]`). Admission per UD-2(a).

### C2 — Dead references / productions contradicting merged SPEC

- **`to_ring_stream`** — GRAMMAR §5.23 note: "converted to a stream explicitly first (`signal |> to_ring_stream`)". Removed by the `.changes` redesign; correct form per 030-155/030-90/030-127: `sensor_signal.changes |> filter(...)`. Only one occurrence in either file; `to_gate_stream` has zero. `.changes` itself needs no new production (ordinary member postfix) — prose/disambiguator updates only.
- **`where` precedence contradiction (CRITICAL).** GRAMMAR §5.23 and Appendix A place `where` at "tier 0b — tighter than `with` (tier 0a) and looser than `|>` (tier 1)", and define `WhereFilterExpr ::= PipeExpr ('where' PipeExpr)*` above PipeExpr. Merged LOG 030-167 (settled) and merged SPEC §4.4.7 (now a 16-tier table with `where` at tier 2) require `where` TIGHTER than `|>`. The §5.23 note's example parse "(a where b) |> c" is right but its production and tier prose are wrong; the §5.23 precedence citation "per 030-168, 030-169" is also stale (those are now "composes at any position" and "observe arm-trigger" entries; the precedence entry is 030-167). Fix: expression tower becomes WithExpr → PipeExpr → WhereFilterExpr → OrExpr…; Appendix A re-tiered to match SPEC §4.4.7 (16 tiers + grammar-extension rows for `with`, prefix `dyn`/`move`, bare `T(x)`).
- **SPEC §4.4.7 tier-count drift.** Merged SPEC now has 16 tiers (where inserted at 2) and its multiplicative row includes `/?` — GRAMMAR Appendix A's 15-tier layout, its "derives from SPEC §4.4.7" prose, §5.7's mirror table, and the §5.7/§2.1.1 tier references must be re-derived wholesale.
- **`arr.length`** — GRAMMAR §5.10 slicing sugar note uses `.length` ×2; merged §012 renames to `.count` (per §9.3.7 bare-count naming rule; same rule cited by 034-13). No other `.length` occurrences.
- **Appendix B abolished rows** — `Cell[T]`, `Signal[T]`, `Derived[T]`, `Recurrent[T, N]` (B.1), `DynamicView[T]` (B.3), plus the B.1 footnote "Recurrent variants … parse via §3.2". Keep `Stream[T, P]` + Ring/Gate aliases (030-261). B.1 must be rewritten around the storability razor (§13.2.8.1); add `yielded` and `dynamic view` pointers into the new kind-annotation section instead.
- **IR_GRAMMAR worked example** — `effect print(message: Cell[string])`; merged SPEC §15.4.5 now reads `message: cell string`.

### C3 — Stale source pointers (script-verified)

Script (`check_pointers.py`, reproduced in §6 Gate G4) extracted every `(§N.M…, NNN-MM)` pointer and compared the pointer's SPEC ref with the SPEC section the cited LOG entry itself carries:

- GRAMMAR.md: 271 instances consistent, 173 flagged; IR_GRAMMAR.md: 31 consistent, 28 flagged; **107 distinct (spec, entry) pairs** flagged overall. No pointer cites a nonexistent entry.
- The flags split into two classes needing adjudication:
  1. **True renumbering staleness** — e.g. every IR_GRAMMAR §033 anchor: `033-166` (cited 26×, now "field/index assignment is a trait call") should be ≈033-168/033-169 (module text grammar / kind-led cell line); `033-203` (behavior grammar, cited ~15×) should be 033-205..033-213 (per-production entries now exist: 033-205 behavior, -206 params, -207 block, -208 instr, -209 operand, -210 terminator, -211 op, -212 literal, -213 LABEL); `033-240` (reset_on_reopen dual position) is now 033-215; the `(§14.6.3, 033-203)` HexU32 anchor is now 033-167 (BID). GRAMMAR examples: `(§9.5.6, 007-239)` → 007-238; `(§11.10.6, 013-79)` — 013-79 was DELETED by D2 (now cites §11.4.1 Copy content by coincidence of renumbering; needs the correct §11.10.6 entry); `(§13.9.12, 022-88)` ×7 and `(§13.9.13, 022-106)` ×7 are off-by-one (022-89/022-107 region); 005-30/005-43/005-44/005-122/005-128/005-130, 006-31/32 (§ refs moved to §3.5.7), 009-86/009-115/009-123, 016-75/-94/-240/-241/-256, 017-12/-21/-219/-228, 021-7/-56/-92/-121/-123, 029-24/-65/-93, 030-46/-168, 031-69/-114/-131.
  2. **Benign canonical-§ divergence** — e.g. `(§13.7.7, 002-12)` where LOG 002-12 (Subject, canonical §1.4) is the correct entry cited from a different SPEC context. These must be allowlisted, not "fixed".
- Full machine-generated mismatch list with candidate re-anchors is regenerable by the script; do not hand-enumerate.

### C4 — The 72-finding backlog

Full finding text embedded in the Appendix below (11 critical / 48 warning / 13 nit). **Line numbers in findings are stale — match by production/section name.** Spot-verification showed the dead pass-3 resolver partially applied fixes; dispositions in §4. Summary: 20 verified already-fixed, 4 obsoleted/settled by merge, 6 verified still-open or reshaped, 42 need per-finding verification.

### C5 — Semantic ripples from main's RFC

- **TraitDecl/FulfillItem**: C1c above; also §7.11/§7.12 waiver prose (005-67/005-71), auto-satisfaction incl. effect-kind methods (005-109).
- **IR §15.4.6 productions** (IR_GRAMMAR §3): `cell` gains `('combiner' BID)? ('else' value)? ('members' member_set)?`; `cell_kind` gains `'fold'` (033-80, 035-9); NEW `member_set ::= '[' (member (',' member)*)? ']'`, `member ::= PATH ':' member_driver`, `member_driver ::= 'permanent' | 'keyed-template' | 'gate-guarded'` (033-81, 035-10); `binding` gains `('@' provenance)?` with NEW `provenance ::= 'bound' | 'constant_wrap' | 'bridge'` (033-64; SPEC §15.4.1 parameter_bindings triples). A `member` disambiguator paralleling the existing reopen_set colon note is needed (PATH ordinal `: INT` vs driver `: NAME`).
- **IR §15.4.4 mirror**: merged SPEC reverted to main's compact behavior grammar; PS conforms SPEC §15.4.4 (op enumeration + operand), after which IR_GRAMMAR §4 mirrors it cleanly — re-anchor citations to 033-205..213 and reword the "lifted verbatim … post Phase-B formalization" intro.
- **IR §6 cross-reference table**: every row re-anchored (C3).
- **§012 rewordings**: `.length` → `.count` (C2).
- **`|>` third dispatch case** (029-65/029-106): §5.25 + §8.11 notes.

### C6 — Unknown-unknowns found by analysis

- **Keyword-inventory drift (§2.4)**: `collect`, `yield`, `fold`, `by`, `yielded` appear nowhere; `cell` (umbrella kind, §13.2.8) also absent. Classification per UD-1(c).
- **§2.2 body-shape table**: needs rows for `collect:` and `fold` (always-indented per 035-2/035-8).
- **Appendix C disambiguator catalog**: new entries for yield-outside-collect, runtime-if-under-collect diagnostic, fold-forces-block-form, kind-annotation vs generic-instantiation overlap (`recurrent[N] T` type position vs `recurrent[` decl head), `where`-tier change, `.count` synthesized member.
- **Appendix D**: fold arms (`by:`/`else:`) column-alignment interaction — verify against §D.2 in-block terminal arms.
- **IR `value`/FLOAT wobble**: resolved by UD-6a + PS.
- **LOG 033-121 vs 033-64/SPEC §15.4.1**: resolved by R2 + PS.
- **LOG 033-165 vs SPEC §15.4.4 op enum**: resolved by UD-6b + PS.
- **SPEC §3.1 BNF gap**: resolved by R1 + PS. Merged SPEC §13.18.5 cell umbrella now includes `yielded` (034-10) — Appendix B.1 wording ripple.

Verified NOT stale (no action): StreamDecl / RecurrentDecl / DesiredCellDecl / ObservedCellDecl already match the merged two-axis declaration model (`stream (ring|gate) [N]? name: T? = expr`, `recurrent[H] stream …`); raw strings; assoc-`is`; record shorthand; move restriction; `here::`/`module::` §13.3 side.

## 3. Phased execution plan (Workflow B)

Every phase: one writer subagent (Opus), one reviewer subagent (adversarial on non-trivial phases), resolver on findings. Phases marked ∥ run in parallel within their wave (disjoint file regions). All edits cite merged LOG entry ids / SPEC §s in source pointers. **Do not trust any line number in this plan or in the findings — locate by production name / section heading.**

**Wave -1 (sequential, FIRST):**
- **PS — LOG/SPEC conformance.** DECISION_LOG-first: (1) LOG 033-211 += `map.make`/`bundle.make`/`slice.make` (match 033-165); (2) LOG 033-209 → elaborated operand (bare NAME | move NAME); (3) LOG 033-121 pairs → triples w/ provenance marker (per 033-64); (4) LOG 030-56..58 bracketed → kind-annotation spellings (§13.2.8.1); (5) LOG §005 TraitDecl/FulfillItem BNF entries amended for effect-kind methods if the BNF lives in LOG; then SPEC conforms: §15.4.6 `value` gains FLOAT; §15.4.4 op production/prose gains the three `.make`s; §15.4.4 operand elaborated; §3.1 TraitDecl BNF + §3.3 FulfillItem BNF gain effect-kind items per §3.1.1.1. Amend-in-place only — no renumbering expected. Reviewer verifies LOG invariants, all six touchpoints, and that worked example §15.4.5 parses under §15.4.4's own productions.

**Wave 0 (sequential):**
- **P0 — Backlog verification.** For each of the 72 findings (Appendix): reproduce against current GRAMMAR.md/IR_GRAMMAR.md; classify already-fixed / still-open / obsoleted-by-merge / reshaped-by-merge; for reshaped, restate against merged LOG/SPEC. Output: survivor worklist consumed by P9. No edits. (Reviewer: spot-check 15 classifications.)

**Wave 1 (∥, after P0):**
- **P1 — Kind-annotation type grammar** (UD-2a). New §3.15 production family: `KindAnnotation ::= 'cell' TypeExpr | 'signal' TypeExpr | 'derived' TypeExpr | 'recurrent' Depth? TypeExpr | 'recurrent' Depth? 'stream' PolicyHead? TypeExpr | 'stream' PolicyHead? TypeExpr | 'yielded' TypeExpr | 'dynamic' 'view' TypeExpr`; `AnnotationType ::= KindAnnotation | TypeExpr`; admission per UD-2(a). Update: §3.2 kind-of-T disambiguator, §3.3 FnType params/returns, §3.4 OperatorType, §7.6 FnParam/FnReturn, §7.13 OperatorParam + `Cell[f32]`→`cell f32` example text, §5.24 note, §11.9 note, Appendix B rewrite (B.1 storability razor; delete abolished rows; keep `Stream[T,P]`, `Handle`, `WeakHandle`, `Portal`, `Type[C]`; delete B.3 `DynamicView[T]`; fix B.5 cite §13.2.10 per V72), Appendix C overlap disambiguators. Sources: SPEC §13.2.8/§13.2.8.1, §13.18.3/§13.18.11, 030-261. Reviewer stresses V58/V59 interactions + `recurrent[N]` type-vs-decl ambiguity.
- **P4 — `where` precedence restructure.** Tower: `WithExpr → PipeExpr → WhereFilterExpr → OrExpr`; rewrite §5.23 (production, tier prose, citation → 030-167, `to_ring_stream` → `.changes` per 030-155), §5.25 PipeExpr operand tier, §5.7 mirror table, Appendix A full re-derivation against merged SPEC §4.4.7 (16 tiers; extension rows qualified per V23). Sources: 030-166..168, SPEC §4.4.7, §13.18.10.4.

**Wave 2 (∥, after P1 — collect/fold need `yielded T` types):**
- **P2 — collect/yield** (UD-1c, UD-3a). New §5.28 CollectExpr + CollectStmt (`collect as`), YieldStmt (+ note: `yield if c: a else: b` is just `yield` + §5.18 IfExpr), §2.4 keyword additions (all four reserved + `by` contextual note), §2.2 body-shape rows, Appendix C/D entries. Post-parse notes: 034-4, 034-7, 034-11/034-12. Sources: SPEC §13.20 complete, 034-1..13.
- **P3 — fold** (UD-1c, UD-4b). New §5.29 FoldExpr: `'fold' PostfixExpr ':' NEWLINE INDENT 'by' ':' Expr NEWLINE 'else' ':' Expr DEDENT`; add to OperatorBodyItem (§7.13) + derived-RHS admission + forced block form (035-8); reconcile OperatorBodyItem with merged §13.17.4 (V41). CHECK: the combiner-lambda syntax in SPEC §13.21 examples (`(a, b) => a + b`) — if it is a surface form the grammar lacks, flag as critical. Sources: SPEC §13.21 complete, 035-1..10, §13.17.4.
- **P5 — Effect-kind trait methods** (UD-5a). §7.11: `EffectMethodSig ::= 'effect' IDENT '(' TraitFnParamList? ')' (':' EffectMethodBody)?` with `observed:` contract sub-grammar reusing §7.14 shapes (kind-annotation types from P1); §7.12: `EffectMethodImpl` per UD-5(a); prose notes 005-67/68/69/71/109; §5.25/§8.11 third-dispatch notes (029-65/029-106). Cites PS-amended SPEC §3.1/§3.3 BNF directly. Sources: SPEC §3.1.1.1, §3.2, §13.17.7 case 3.

**Wave 3 (∥, after Wave 2):**
- **P6 — Dead-reference and prose sweep.** `.length`→`.count`; residual `to_ring_stream` must be zero; renamed-member audit against merged §012; Appendix B leftovers; general grep pass (`Cell[`, raw strings, old constructor names).
- **P7 — IR_GRAMMAR.md upgrade.** §3: cell fold clauses + member_set/member/member_driver + provenance on binding (+ member colon disambiguator; FLOAT in value per UD-6a); §4: re-anchor to 033-205..213, reword intro, op enumeration per PS-amended 033-211, operand per PS-amended 033-209; §5 worked example: `cell string` + token-for-token vs merged SPEC §15.4.5; §6 cross-reference table full re-anchor; §0/§1 prose citations. Sources: SPEC §15.4.1/§15.4.4/§15.4.5/§15.4.6; 033-60..102, 033-119..125, 033-130..133, 033-155..169, 033-205..215, 033-64, 035-9/10.
- **P9 — Survivor-finding remediation.** Fix every P0 still-open/reshaped finding (expected: V8 ReservedInstanceField desired/observed gap; V60 NamedBundlePlacement tail; V63 SelfDelimitingPlacement bundle forms; V31/V43 Decl-union duplication; V41 handled in P3; V70/V71 §5.1 Path restriction per D7; plus P0 discoveries).

**Wave 4 (sequential, last):**
- **P8 — Source-pointer adjudication.** Run the §6/G4 script; for each flagged pair: adjudicate true-stale vs benign; re-anchor true-stale pointers (script emits candidates; resolver confirms by reading entry text); benign pairs → allowlist `docs/scratchpad/plans/pointer-allowlist.txt` (one-line justification each). Covers pointers newly introduced by PS/P1-P7. Includes the D2 casualty `(§11.10.6, 013-79)`. Reviewer double-checks a 20% sample.
- **PV — Verification gates** (§6) + final lens reviews.

Dependencies: PS first; P0 → P9; P1 → {P2, P3, P5}; P4 independent; P6/P7/P9 after wave 2; P8 last-but-one; PV last.

## 4. 72-finding disposition table (pre-verification hypotheses — P0 confirms)

Legend: FIXED = verified fixed in current text (P0 confirms & closes); VERIFY = likely fixed, must reproduce (P0); OPEN = verified still present (P9); RESHAPED = merge changed the premise (P0 restates, P9 fixes); OBSOLETE = merge/settled decision voids it (P0 closes with citation).

| ID | Sev | Disposition | Note |
|----|-----|-------------|------|
| V1 | crit | FIXED | `panic` no longer in §2.4 keyword list; PanicExpr marked documentation-only overlay |
| V2 | crit | FIXED | `fulfill` now clause-keyword only, with explanatory note |
| V3 | warn | VERIFY | declaration-keyword block now dual-cites 003-50/007-11/009-102 — confirm complete |
| V4 | warn | VERIFY | ownership block now cites 001-18/004-47/008-7 — confirm |
| V5 | crit | OBSOLETE | `/?` legitimized by merged 007-133..136 + SPEC §4.4.7 multiplicative row; verify checked-group placement |
| V6 | warn | FIXED | now `IntBuiltinSuffix`, "integer-typed" |
| V7 | warn | VERIFY | §2.1.1 §4.4.7 qualifier |
| V8 | warn | OPEN | `ReservedInstanceField ::= 'from'\|'to'\|'pair'\|'exposition'` still omits desired/observed — reconcile with 002-28 |
| V9-V12 | nit/warn | VERIFY | format/lexer notes; V11 likely fixed (KW_HANDLE_BANG declared) |
| V13 | crit | FIXED | AssocTypeEqualityPredicate now `AssocPath 'is' TypeExpr` with restriction note |
| V14-V18 | warn | VERIFY | type/pattern notes |
| V19 | warn | OBSOLETE | settled D5: OperatorType has no `own` return — grammar already correct; add §13.17.13 citation |
| V20, V21 | nit | VERIFY | |
| V22 | crit | FIXED | §5.7 names the WithExpr/WhereFilterExpr/PipeExpr tower; superseded by P4 re-tiering anyway |
| V23 | warn | RESHAPED | Appendix A wording rewritten wholesale in P4 |
| V24 | warn | RESHAPED | D2 landed; verify the "parse error" claims match production-level guards |
| V25-V30 | warn/nit | VERIFY | |
| V31 | crit | VERIFY | ConstStmt/Decl-union — region changed; reproduce |
| V32 | crit | FIXED (partial) | TraitDecl inline head removed; P0 must check NodeDecl/ConnectionDecl/Signal/Derived/Recurrent/StreamDecl heads too (D4) |
| V33-V36 | warn | VERIFY | pointer re-cites — fold into P8 |
| V37-V43 | warn/nit | VERIFY | V41 handled in P3 against merged §13.17.4 |
| V44 | crit | VERIFY | WrapperPlacement moved; check element order vs 021-123 (renumber-check the entry too) |
| V45-V49 | warn | FIXED (sampled) | PlacementAs/RepeatKeyedBy/ForAsClause eliminated; P0 confirms V46/V49 |
| V50-V56 | warn/nit | VERIFY | |
| V57 | crit | FIXED | Placement union now includes ConnectionPlacement |
| V58, V59 | crit | VERIFY | CompileTimeFor greediness; TypeRef-`[`-vs-bundle disambiguation — must hold under the new kind-annotation grammar too (P1 reviewer) |
| V60 | warn | OPEN | `NamedBundlePlacement ::= BundlePlacement 'as' IDENT` still bare |
| V61, V62 | warn | VERIFY | |
| V63 | warn | OPEN | SelfDelimitingPlacement still excludes bundle forms |
| V64-V68 | warn/nit | VERIFY | V64 is a pointer re-cite → P8 |
| V69 | warn | FIXED | DirectiveArg no longer carries the `StringLit ',' IDENT` union |
| V70 | warn | OBSOLETE→OPEN(inverted) | D7 settles single-suffix: §13.3 is CORRECT; the fix moves to §5.1 Path (V71's site) |
| V71 | warn | OPEN | §5.1 PathSegment admits bare/chained `here`/`module` — post-parse note or production restriction per D7 |
| V72 | nit | VERIFY | Appendix B cite — subsumed by P1 Appendix B rewrite |

## 5. User-decision items

All resolved — see "User-resolved design decisions (LOCKED)" in §1. Historical option analyses live in the session transcript.

## 6. Verification gates (Workflow B exit criteria)

- **G1 Coverage**: every LOG entry in §034 (13) and §035 (10) plus 005-67..71, 005-109, 029-65, 029-106, 030-155/-166/-167/-168, 030-261, 033-64, 033-80..82, 033-205..215, and SPEC §13.2.8/§13.2.8.1/§13.20/§13.21/§3.1.1.1/§4.4.7/§15.4.6 maps to at least one production or disambiguator note (checklist audit by a fresh-context subagent).
- **G2 Translatability**: fresh-context parser-sketch test over stress constructs: `derived total: f32 = fold voices:\n  by: (a, b) => a + b\n  else: 0.0`; `collect as active:` with repeat-nested and when-gated yields; `yield if is_active: voice else: muted`; runtime-if-yield (must be parseable, semantically diagnosed); `operator passthrough[T](source: cell T) -> cell T: source`; `Portal[cell T]`; `stream gate[256] db_writes = pending_writes`; `events where pred |> op` (must parse `(events where pred) |> op`); `sensor.changes |> skip_first`; trait with `effect render(value: Subject):` + `observed:` contract; IR module with a `fold` cell (`combiner B@x else 0.0 members [App.v:0 : keyed-template]`) and a provenance-marked binding (`params [message: App.label @bridge]`).
- **G3 Adversarial completeness**: critic hunts undefined nonterminals, unreachable productions, keyword-inventory vs production mismatches, §5.7/Appendix A tier consistency, §2.2 body-shape completeness for the new forms.
- **G4 Pointer integrity (standing gate)**: re-run the pointer checker (embedded below): zero flagged pairs outside the committed allowlist; allowlist entries each carry a one-line justification.

```python
#!/usr/bin/env python3
# check_pointers.py — (SPEC-ref, LOG-entry) consistency for grammar source pointers.
import re, collections
DOCS = 'packages/ductus-lang/docs/'
log = {}; by_sec = collections.defaultdict(list)
for line in open(DOCS + 'DECISION_LOG.md'):
    m = re.match(r'^(\d{3})-(\d+)\.\s+(.*)$', line)
    if m:
        eid = f"{m.group(1)}-{m.group(2)}"; text = m.group(3)
        specs = re.findall(r'§([\d.]+\d)', text[-120:])
        log[eid] = specs; by_sec[m.group(1)].append((eid, specs))
ptr = re.compile(r'\(\s*§([\d.]+\d)[^)]*?,\s*(\d{3}-\d+)')
for fname in ('GRAMMAR.md', 'IR_GRAMMAR.md'):
    for n, line in enumerate(open(DOCS + fname), 1):
        for spec, eid in ptr.findall(line):
            if eid not in log:
                print(f"{fname}:{n} MISSING (§{spec}, {eid})"); continue
            es = log[eid]
            if es and not any(s.startswith(spec) or spec.startswith(s) for s in es):
                cands = [e for e, sp in by_sec[eid[:3]]
                         if any(s.startswith(spec) or spec.startswith(s) for s in sp)]
                print(f"{fname}:{n} FLAG (§{spec}, {eid}) LOG cites §{'/'.join(es)};"
                      f" candidates: {','.join(cands[:6]) or 'none'}")
```
Flags are *candidates for adjudication*, not automatic errors: a pointer may legitimately cite a SPEC section different from the LOG entry's canonical home (e.g. `(§13.7.7, 002-12)` for `Subject`). Adjudicated-benign pairs go in the allowlist; the gate then filters them.
- **G5 Lens reviews** (2): (i) new-constructs lens (collect/yield/fold/kind-annotations/effect-kind methods) over GRAMMAR.md; (ii) IR lens over IR_GRAMMAR.md vs merged SPEC §15.4.
- **G6 Backlog closure**: every V1-V72 has a recorded terminal disposition; survivors show the fixing diff hunk.
- **PS-gate**: LOG dense-numbering invariants for touched sections; all six conformance touchpoints re-verified; SPEC §15.4.5 worked example parses under SPEC §15.4.4's own productions; no LOG-to-LOG cross references.

## 7. Open risks

- **R4** The 42 VERIFY findings may include additional still-open items; P0 sizes P9 — budget flexibly.
- **R5** Pointer adjudication (107 pairs) is judgment work; the allowlist prevents silent regressions but its first population needs care (reviewer double-checks a 20% sample).
- **R6** Kind-annotation grammar may interact with existing disambiguators (V58/V59 bundle-`[` adjacency; `recurrent[N]` type-position vs decl-head) — P1's reviewer must stress these.
- **R7** The fold combiner-lambda syntax (`(a, b) => a + b`) may be a new closure surface form the grammar lacks entirely — P3 must check and flag.
(R1/R2/R3 from the analysis were resolved by the user: fixed in PS, no longer risks.)

## 8. Out of scope

- Any LOG/SPEC change beyond the six PS touchpoints.
- Reference parser implementation.
- Raw strings (stay removed).
- Runtime semantics beyond what disambiguator notes require.
- The `backends/` docs and `lint_nnm.py`.

## Appendix — The 72-finding backlog, full text (pass-2 e2e list, embedded for clean-slate execution)

Line numbers below reference the pre-merge 6044-line GRAMMAR.md — locate by production/section name, not line.

- **V1** [critical] (GRAMMAR.md:296; GRAMMAR.md:2763; GRAMMAR.md:2777; refs LOG 002-11, LOG 011-23, LOG 011-24): `panic` listed as operator-context keyword (line 296) contradicts LOG 002-11 (omits panic) and LOG 011-23/24 (panic is prelude builtin). Also creates self-contradiction with §5.27 note at line 2777 saying panic is NOT a keyword. PanicExpr production uses quoted-keyword form.
- **V2** [critical] (GRAMMAR.md:233; GRAMMAR.md:252; refs LOG 002-3, LOG 002-4): `fulfill` listed twice — in Declaration keywords (line 233) and Clause keywords (line 252). LOG 002-3 omits fulfill; LOG 002-4 lists it once as a clause keyword. Declaration placement contradicts LOG.
- **V3** [warning] (GRAMMAR.md:227; refs LOG 002-3): Declaration-keyword block (line 227) cites (002-3) but adds `use`, `wraps`, `fulfill`, `alias` not present in LOG 002-3's enumeration. Citation is misleading; either anchor to additional LOG entries (003-50, 007-*) or note divergence.
- **V4** [warning] (GRAMMAR.md:242; refs LOG 002-11, LOG 001-18): `Ownership/coercion keywords (002-11)` at line 242 cites wrong LOG entry — 002-11 is operator-context keywords. own/move come from 001-18; dyn from 004-47/008-7.
- **V5** [critical] (GRAMMAR.md:617): §2.12 operator table line 617 includes `/?` alongside `\?` in the saturating-q group, but wrapping (615) and saturating (616) groups use only `\` for division. `/?` appears spurious or undocumented (disambiguator at 630–632 confirms `\` is integer division).
- **V6** [warning] (GRAMMAR.md:535; GRAMMAR.md:552; refs SPEC §4.3.2): `SuffixedIntLit ::= IntLit BuiltinSuffix` where BuiltinSuffix includes f32/f64 — so `5f32` is a SuffixedIntLit token, contradicting SPEC §4.3.2 (literal with f-suffix is float). Should be renamed or split.
- **V7** [warning] (GRAMMAR.md:153): §2.1.1 line 153 references `precedence table of §4.4.7` without qualifying as SPEC; GRAMMAR §4 is Patterns and the precedence mirror lives at §5.7/Appendix A. Reader is misdirected.
- **V8** [warning] (GRAMMAR.md:253; GRAMMAR.md:328; refs LOG 002-28): §2.4 declares `desired`/`observed` as clause keywords (line 253) and LOG 002-28 includes them as field-like-position keywords, but ReservedInstanceField production (line 328) lists only `from|to|pair|exposition`. Production misses desired/observed.
- **V9** [nit] (GRAMMAR.md:315): §2.4 lines 309–316 wraps a code fence with no real production, with a trailing source-pointer comment line, violating the format convention that source pointers follow a production block.
- **V10** [nit] (GRAMMAR.md:609): §2.12 lines 609,611,636,648 use SPEC-section cite `§13.8.8` for placement-flag char without qualifying as SPEC; GRAMMAR's internal flag-run production is §11.6.
- **V11** [warning] (GRAMMAR.md:651): `KW_HANDLE_BANG` symbolic name referenced in §2.12 disambiguator (line 651) but never declared as a terminal anywhere — the rest of the doc uses spelling-quoting.
- **V12** [warning] (GRAMMAR.md:495): §2.9 string-interpolation lexer mode enumerates STR_END / STR_TEXT / STR_ESCAPE / STR_INTERP_OPEN / STR_INTERP_CLOSE but no paired STR_BEGIN — opening `"` produces no token; needs explicit note.
- **V13** [critical] (GRAMMAR.md:715; GRAMMAR.md:1135): AssocTypeEqualityPredicate `TypeExpr '.' IDENT '=' TypeExpr` is unparseable as written: TypeExpr greedily absorbs `.IDENT` via TypeAssocAccess postfix, so `T.Item = i32` has its `.Item` consumed by TypeExpr leaving no `.IDENT` for the predicate. Need restriction or disambiguator.
- **V14** [warning] (GRAMMAR.md:987): DynTrait does not admit `'(' TypeExpr ')'`/single-trait-in-parens nor parenthesised FnType — IntersectionType inside parens requires 2+ operands via `( '&' PostfixType )+`.
- **V15** [warning] (GRAMMAR.md:703; GRAMMAR.md:1032): TypeExpr alternation DynType|IntersectionType|PostfixType needs explicit ordering/lookahead note: IntersectionType prefix-overlaps PostfixType (PostfixType is its first operand).
- **V16** [warning] (GRAMMAR.md:1375): RecordPattern admits empty `Foo()` (RecordFieldList?) for non-empty records though semantics require exhaustive binding — needs explicit syntactic-vs-semantic note.
- **V17** [warning] (GRAMMAR.md:1283): VariantPattern admits empty `Foo()` for payload-bearing variants with no disambiguator addressing rejection.
- **V18** [warning] (GRAMMAR.md:1489): Refutability table single-variant carve-out for VariantPattern not clearly backed by `(§6.2.4, 006-31)` source pointer — verify or move to a SPEC-cited footnote.
- **V19** [warning] (GRAMMAR.md:884): OperatorType allows `-> TypeExpr` but not `-> own TypeExpr` while FnType allows `own`; SPEC §13.17 does not appear to forbid `own` on operator return — verify.
- **V20** [nit] (GRAMMAR.md:711): TypePostfix comment claims `[…]` covers slice but production only admits GenericArgList; slice goes through a separate SliceType. Comment is misleading.
- **V21** [nit] (GRAMMAR.md:706): TypeExpr source pointer `(§5, 005-44)` is uncharacteristically broad (no subsection); other pointers use N.M.K granularity.
- **V22** [critical] (GRAMMAR.md:1954; GRAMMAR.md:1968; GRAMMAR.md:1969): §5.7 precedence table (lines 1954–) omits tier 0a (with), tier 0b (where), and prefix `dyn`/`move` at tier 13, and bare `T(x)` cast at tier 14, all present in Appendix A. Multiple lens hits collapsed: this is one defect — §5.7 table is stale vs Appendix A.
- **V23** [warning] (GRAMMAR.md:5841; refs SPEC §4.4.7): Appendix A header (line 5841) claims table is `lifted verbatim from SPEC §4.4.7` but additions (0a/0b/dyn/move/T(x)) contradict 'verbatim' — wording should be qualified.
- **V24** [warning] (GRAMMAR.md:1769; GRAMMAR.md:1801; refs LOG 013-135..142, 013-79): `move` is unguarded as a PrefixOp alternative; restriction to call-arg/paren-receiver positions is post-parse-only despite disambiguator claiming it is a parse error.
- **V25** [warning] (GRAMMAR.md:1672): CastPolicySuffix is admitted as any Postfix alternative — over-admits e.g. `(f())%(g)` though §5.6 prose requires type-name head. Needs grammar restriction or explicit post-parse note.
- **V26** [warning] (GRAMMAR.md:2071): Slicing commit-on-`..` rule (§5.10 lines 2071–2079) is prose-only — not surfaced in Appendix C disambiguator catalog.
- **V27** [warning] (GRAMMAR.md:2107): DeleteTarget production has no constraint that its root is an l-value path — e.g. `delete {a:1}[a]` parses; needs disambiguator/post-parse check note.
- **V28** [warning] (GRAMMAR.md:2170; GRAMMAR.md:2612; GRAMMAR.md:2680): §5.13 / §5.23 / §5.25 prose blocks before productions are narrative not `//`-prefixed disambiguators, breaking format convention.
- **V29** [warning] (GRAMMAR.md:2138): §5.11 commentary line 2138 about `delete (...)` is imprecise — actual rule is that trailing `[Expr]` must sit outside any paren wrap. Worth clarifying.
- **V30** [nit] (GRAMMAR.md:2515): WhenSimple's WhenOtherwise-after-DEDENT placement relies on column-alignment layout (Appendix D §D.2) but no inline cross-ref is given.
- **V31** [critical] (GRAMMAR.md:5676; GRAMMAR.md:2828; GRAMMAR.md:3022): ConstStmt listed in Decl union (line 5676) but ConstStmt is body-only (§6.1) with no Visibility? slot; module-level const uses TopLevelConstDecl. Admits annotated `const` at module level via AnnotatedDecl wrapper, conflicting with §7's TopLevelDecl.
- **V32** [critical] (GRAMMAR.md:3507; GRAMMAR.md:3831; GRAMMAR.md:3939; GRAMMAR.md:4509): Inline `Annotation*` head on TraitDecl/NodeDecl/ConnectionDecl (and SignalDecl/DerivedDecl/RecurrentDecl/StreamDecl) duplicates AnnotatedDecl wrapper, while other decls (RecordDecl, EnumDecl, NewtypeDecl, FnDecl, OperatorDecl, EffectDecl, AliasTypeDecl) lack the inline head — inconsistent and produces duplicate annotation parse path.
- **V33** [warning] (GRAMMAR.md:3213; refs LOG 013-117, LOG 013-126): FnParam source pointer cites LOG 013-126 (call-site default semantics) but the param `own` modifier rule is LOG 013-117.
- **V34** [warning] (GRAMMAR.md:3216; refs LOG 013-121): FnReturn source pointer cites 013-126; the `-> own T` rule is LOG 013-121.
- **V35** [warning] (GRAMMAR.md:2973; refs LOG 013-30): ReturnStmt cites (§11.3.6, 013-30) but LOG 013-30 is about always-initialized bindings, unrelated to return.
- **V36** [warning] (GRAMMAR.md:3019; refs LOG 003-34, LOG 004-73): TopLevelConstDecl source pointer is (§2.4.1.1, 003-34) but the const RHS rule is LOG 004-73; recommend dual-cite for const-semantics + visibility-attach.
- **V37** [warning] (GRAMMAR.md:2872): LoopElseClause uses `ColonBody` while other §6 forms (ForStmt, WhileStmt) use `Body` — same shape, inconsistent naming.
- **V38** [warning] (GRAMMAR.md:3225): FnBody (line 3225) duplicates Body shape plus adds `NEWLINE` for abstract trait methods — but FnBody is also used by FnDecl (§7.6) where the NEWLINE alt is parse-only-rejected post-parse. Recommend splitting.
- **V39** [warning] (GRAMMAR.md:3546; GRAMMAR.md:3640): MethodSig (TraitDecl) and FulfillBodyItem (FulfillItem) both reuse FnBody; rejection of NEWLINE alt in fulfill is post-parse. Cross-references between §3.1.3 default body and §7.6/§7.7 should be explicit.
- **V40** [warning] (GRAMMAR.md:3334): RecordBody `'=' IntersectionType` alt has no disambiguator note explaining selection between record-intersection RHS and `:`-introduced body, especially vs `alias type X = A & B`.
- **V41** [warning] (GRAMMAR.md:3666): OperatorBodyItem admits `let` but not `const` (and not AssignStmt) — exclusion appears unintentional; verify §13.17.4.
- **V42** [nit] (GRAMMAR.md:3047): Visibility production's `'private'` lacks ConstructorVis? asymmetry call-out inside the production block.
- **V43** [nit] (GRAMMAR.md:5672): TopLevelDecl lists ModuleReactiveDecl AND the four reactive decls individually — double-listing; pick one.
- **V44** [critical] (GRAMMAR.md:4333; refs LOG 021-123): WrapperPlacement element order (line 4333) swaps AttrClause/WhenClause vs §11.1 TypeRefPlacement and SPEC §13.8.9 (021-123) normative order: WhenClause before AttrClause.
- **V45** [warning] (GRAMMAR.md:4338; GRAMMAR.md:5054): PlacementAs duplicates NameClause (`'as' IDENT`); should reuse canonical NameClause.
- **V46** [warning] (GRAMMAR.md:4345): ExposeRepeatEntry/EffectsRepeatEntry inline `('at' IDENT)?` and locally redeclare RepeatKeyedBy instead of reusing canonical §11.9 RepeatIndex/RepeatKeyed.
- **V47** [warning] (GRAMMAR.md:4353): ExposeForEntry/EffectsForEntry use local ForAsClause instead of canonical NameClause.
- **V48** [warning] (GRAMMAR.md:4359; GRAMMAR.md:5399): §8.10 RepeatKeyedBy duplicates §11.9 RepeatKeyed — two nonterminals same surface.
- **V49** [warning] (GRAMMAR.md:4308; GRAMMAR.md:4452): ExposeWhenBlock inlines a simple-when shape rather than delegating to WhenBlockExpr (as EffectsWhenBlock does) — inconsistent abstraction.
- **V50** [warning] (GRAMMAR.md:4624): FromMultiDecl/ToMultiDecl admit trailing `','?` undocumented; inconsistent with other comma-list productions (e.g. SatisfiesClause).
- **V51** [warning] (GRAMMAR.md:4683): §9.4 PairEntry `TypeExpr '->' TypeExpr` is ambiguous against FnType `'->'`; needs disambiguator.
- **V52** [warning] (GRAMMAR.md:3953): §8.1 NodeBodyMember lists EffectsClause before ExposeClause but §9.1 disambiguator says expose comes last — listing order risks misreading.
- **V53** [warning] (GRAMMAR.md:4297): §8.10 ExposeEntry alternatives (ViewNameEntry/NamedChildEntry/ConnectionPlacement/WrapperPlacement) ambiguous from leading IDENT; disambiguation deferred to post-parse name resolution without production-level capture.
- **V54** [nit] (GRAMMAR.md:4781): §9.6 source pointer carries two SPEC sections `(§13.9.2, §13.6.1.1, 022-3)` — atypical vs §0 convention.
- **V55** [nit] (GRAMMAR.md:4252): §8.9 DefaultAttrDecl admits unlimited occurrences though semantics restrict at-most-one per node body — consistent with other deferred-to-semantics patterns but worth cross-ref.
- **V56** [nit] (GRAMMAR.md:4113): §8.5 StandaloneView admits BundleViewSelector though views are 2D-only co-placement constructs — needs restriction or post-parse note.
- **V57** [critical] (GRAMMAR.md:5043; GRAMMAR.md:5152; GRAMMAR.md:5181): ConnectionPlacement is unreachable through §11.1 Placement union (which only lists TypeRefPlacement/NamedBundlePlacement/BundlePlacement). BodyIntro's BlockBody/InlineBody do not admit a bare NodeRef. `Drives: some_car` (021-81) cannot parse via productions as written.
- **V58** [critical] (GRAMMAR.md:4908; refs LOG 017-96): CompileTimeFor inline form `'for' Pattern 'in' Expr ':' BundleMember (BundleMember)*` is greedy/ambiguous inside `[...]` (layout suspended); LOG 017-96 says 'one child placement per iteration' (singular).
- **V59** [critical] (GRAMMAR.md:5051; GRAMMAR.md:4897): TypeRef `'[' GenericArgList ']'` vs BundlePlacement `[` adjacency has no disambiguation rule (flag-run adjacency rule is for non-letter; no analogous rule for `[`).
- **V60** [warning] (GRAMMAR.md:4940): NamedBundlePlacement = BundlePlacement 'as' IDENT — admits nothing after IDENT, contradicting §11.1 prose that bundle's `as` slot sits before when modifier (implying when/attr/body follow).
- **V61** [warning] (GRAMMAR.md:5230; GRAMMAR.md:5243): AttrClause multi-line continuation form has no production expression — only prose; reader cannot derive from grammar.
- **V62** [warning] (GRAMMAR.md:5324): DefaultArgExpr AtomicExpr `Literal | IDENT | TypePath` — IDENT is subsumed by single-segment TypePath; redundant.
- **V63** [warning] (GRAMMAR.md:5472): SelfDelimitingPlacement omits bare bundle `[a b]` and named-bundle `[a b] as pair` — `[...]` brackets are inherently self-delimiting but the production requires `(...)` wrap.
- **V64** [warning] (GRAMMAR.md:5185; refs LOG 021-93): ConnectionDestBody cites (§13.8.5.1, 021-93) but LOG 021-93 is filed under §13.8.6 per DECISION_LOG. Cite drift.
- **V65** [warning] (GRAMMAR.md:5281): FlagChar set includes `*` and `+` which collide with Cardinality sigils (§8.7 / §10.1) but disambiguator only calls out `?`/`@`/`'`/`!`.
- **V66** [warning] (GRAMMAR.md:4994): §10.5 example uses parenthesised `when (silent_break is true)` per 021-127, but SPEC §13.3.3.5 example uses unparenthesised `when silent_break: true` — note doesn't flag the SPEC example as non-conformant.
- **V67** [nit] (GRAMMAR.md:4966): §10.4 source-pointer `;  (§13.3.3.5, 017-101)` sits orphaned outside a production block (only commented examples).
- **V68** [nit] (GRAMMAR.md:5354): §11.8 has no production block, only commentary referring to §11.1 — breaks per-subsection format rhythm.
- **V69** [warning] (GRAMMAR.md:5550): DirectiveArg union includes `StringLit ',' IDENT` — inner comma collides with outer DirectiveArgs `(',' DirectiveArg)*` comma split, producing ambiguous parse. LiteralSuffixDirective in §12.1 spells the rule correctly without union.
- **V70** [warning] (GRAMMAR.md:5770): §13.3 NamespaceAnchorPath admits only single AnchorSuffix segment, but §5.1 Path admits chained `::` segments after `here`/`module` (e.g. `here::SomeMod::x`). Either restrict per SPEC or generalize to chained.
- **V71** [warning] (GRAMMAR.md:1621): §5.1 PathSegment admits `'here'`/`'module'` allowing bare `here`/`module` Path (zero `::` repetitions) — contradicts §13.3 'may not appear bare'. Needs post-parse note on Path production.
- **V72** [nit] (GRAMMAR.md:5927): Appendix B B.5 `Type[T]` cites SPEC §5.7 but §13.2.10 is the meta-type home.
