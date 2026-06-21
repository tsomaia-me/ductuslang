# Ductus Spec Findings Ledger

**0 open findings remain (2026-06-21).** Originally 61 contradictions, ambiguities, incoherencies, and unsound inferences discovered in SPEC.md during its lossless serialization into DECISION_LOG.md; 53 have since been ruled and applied to SPEC.md + DECISION_LOG.md and struck — 51 across sessions B/C/E/F, plus F017 and F053 struck as **moot no-ops** (vacuous qualifiers, no spec change). The last 8 — the design forks F020, F129/F134, F133, F135, F138, F140 plus F136 riding with F138 — were ruled and applied 2026-06-21 (see the struck ledger below); the ledger is now empty. Per the production protocol, NONE of these was silently resolved: each finding records the competing readings, the impact, and the contested entry wording (quarantined — withheld from DECISION_LOG.md until ruled on). Every finding was independently verified by at least two agents (extractor + skeleton-first reviewer; three for chunks 10/13/14, which received an additional reconstruction audit; items discovered after delivery — final-QA spot-audits and the syntax-completeness sweep — are marked in-row and were verified by the discovering auditor plus the orchestrator against SPEC.md); per-finding assessments live in the production reports (git history of `.declog-work/report/`).

Ruling protocol: for each finding, decide the intended reading, fix SPEC.md accordingly, then add the now-clean decision(s) to DECISION_LOG.md (next free number, placed by topic). Strike the finding from this ledger when resolved.

Counts by kind (original 61): 30 ambiguities · 24 incoherencies · 6 contradictions · 1 unsound-inference. **Open now (0):** the 8 design forks were ruled & applied 2026-06-21.

## High-priority digest (orchestrator's adjudication of the strongest items)

> **Historical snapshot.** This was the original top-20 adjudication. All of it — including items
> 18–19 (the IR behavior-ID width and the §15.4.5-vs-§15.4.1 text-form forks) — has since been ruled
> and applied; the ledger is now empty (2026-06-21).

1. **[RULED S2: truncate toward zero]** §4.4.1.2 — `\` is called "truncating" integer division but the spec's own example `-5 \ 2` yields `-3` "(toward negative infinity)", i.e. floor division (truncation gives `-2`).
2. §4.4.3/§4.4.6 vs §4.9.1 — comparison operators attributed to `Ord`, but `Ord` is a methodless umbrella; dispatch targets are `Lt::lt`/`Le::le`/`Gt::gt`/`Ge::ge`. Different inferred generic constraints under each reading.
3. §4.8.2 vs §4.9.2 — `Float` umbrella "requires all" float-op traits, but the canonical §4.9.2 requires-list omits `Log` (two-arg) and the inspection traits (`is_nan` etc.).
4. §4.5/§4.9.4 vs §4.7 — residual references to lossless casts being "trivially equivalent to `as`" although §4.7 defines `as` as not-a-cast (naming only). Apparent residue of an earlier design.
5. **[RULED S2: inner-error model]** §8.4.1 vs §7.9 — the literal `?` desugar `return From::from(failure)` type-checks for `Option` (via `From[()] for Option[T]`) but not for `Result`, whose stated requirement `From[SourceError] for DestError` needs an unstated `Err(...)` re-wrap.
6. **[RULED S2: explicit cast]** §9.3.4 — the `usize`-indexing example (`arr[n]` ✓ "widens to isize") contradicts the stated rule that types whose range exceeds `isize` require an explicit cast; the "same source valid on every platform" claim also conflicts with the platform-dependent cast requirement.
7. §11.3.5 — Rule (P)'s body lists `move x` at a call site among name-only consumes, while the corollary/diagnostic/example make `move` of an alias into an `own` parameter a compile error; §11.8.5 permits `move` only toward `own` parameters, so the success path is unreachable.
8. §11.10.1 vs §11.10.5/§11.9.1 — unconditional Copy-only closure captures vs non-escaping closures "may treat their environment access as alias access": observable legality differs for `fn(): sum(buf)` with non-Copy `buf`.
9. **[RULED S3: `-> own Subject`]** §11.5/§11.3.6 — `Clone`'s declared signature `fn clone(value: Subject) -> Subject` is borrow-default-returning under the spec's own return conventions, contradicting its independent-owned-copy contract (would need `-> own Subject`).
10. §12.3.7 — the fixed-extent paragraph ("a loop over a fixed-extent value unrolls") conflicts with the section's own iff-rule (unroll iff iterable compile-time-known) and the two-admitted-forms list; tuple iteration both "not provided" and in the fixed-extent set.
11. **[RULED S4: open-parts + no `expose:` exposes the caller-supplied children as-is; empty only when nothing is supplied]** §13.3.3 vs §13.3.7.2 — an open-parts node's children are "walked through its exposition" while the same node with no `parts:` and no `expose:` has an EMPTY exposition.
12. **[RULED S5a: gate-close fires no recompute — the phantom forward-reference was removed]** §13.9.7 — the deriveds bullet forward-references a "one-shot effect-desired recompute of the close transition (below)" that the gate-close paragraph explicitly denies ("there is no recompute pass").
13. **[RULED S5a: a hot reload re-opens a pre-live registration window]** §13.14.7 vs §13.15.6 — reconciler registrations after the live transition are rejected, yet a renamed effect type must be registered on a live runtime "before the reload reaches the live state".
14. **[RULED 5b: require ≥1 block; the block-less `log` example was rewritten with a `desired:` block]** §13.19.2 vs §13.19.3 — "at least one of desired:/observed: must be present" vs the `effect log(...)` example with neither block ("pure fire-and-forget consumer").
15. **[RULED 5b: operator named args use `:`]** §13.17.13 vs §3.5.4 — the operator-type examples use `=` named-argument syntax (`apply(source = s, kernel = gain)`) although named value arguments use `name: value`.
16. **[RULED 5b: `|>` RHS may be a terminal sink]** §13.17.7 — "`|>` may only apply operators or effects" vs the same section's Case 3 (RHS is a `Sink[T]`).
17. **[RULED S2: `&` only]** §3.6.1/§2.2.4/§13.17.8 vs §5.1 — trait bounds written both `T: Add + Mul` and `T: Numeric & Ord`; only `&` is ever defined as bound conjunction; `+` in bound position is used but never specified.
18. §14.6.3 (and §15.4.4's `B@aa10` ids) — behavior IDs are both "stable u32" and "content-addressed stable hash"; width vs content unreconciled, `BID` lexeme undefined in the IR grammar.
19. §15.4.5 vs §15.4.1 — the worked example's text form diverges from the field lists (standalone `gate ... guards [...]` entries; cell lines carrying `role=`/`uses`/`inputs`/`depth`; omitted observability/size; `App.print:0.text : str` fed by a behavior returning `%TextRec`; `label |> print` producing no `connection` entry despite the desugar map).
20. **[RULED S4: `stream` is the sixth no-source-write kind; the enumeration now lists all six]** §13.2.7 — the no-source-write rule enumerates five declaration kinds but claims to apply "to all six declaration kinds uniformly" (the §13.2 intro's six include `stream`, omitted from the enumeration).

## Full ledger — 0 open findings (of the original 61; all ruled & applied — see git history for their resolutions).

**All 8 struck — ruled & applied 2026-06-21 (this session):**
- **F020** → trait-method named call binds to the *trait's* parameter names; fulfill names are body-local; no param-name-equality check. (006-33; SPEC §3.5.5/§3.3.1)
- **F129 / F134** → behavior identity = wide content hash; the `u32` is a separate runtime handle (behavior-table index); `BID ::= 'B@' HEX+` renders the handle. (032-115/032-180/033-251; SPEC §14.6.3/§15.4.4)
- **F133** → `move`'s operand may be a field-access l-value path rooted in an owned binding (`move v.field`) — a partial move; method-call operands stay forbidden. (013-245; SPEC §11.8.5)
- **F135** → gating is encoded as first-class named gate objects `{id, pred, inputs, guards, gate_parent}`; gated instances reference their gate by id. (033-252; SPEC §15.4.1)
- **F136** → an effect's desired state is one whole-record `pool_index<%T>` cell (the desired-builder's output) the runtime scatters per-field. (033-255; SPEC §15.4.5)
- **F138** → the text form is the normative serialization; the module/type-table/graph text grammar is written (parallel to the behavior grammar). (033-49/033-240/033-254; SPEC §15.4.6)
- **F140** → observability/cadence cell classification is demoted to an implementation-defined backend concern (28 normative entries retired); the reload-reclassification question dissolves. (033-253; SPEC §15.4.1.2–.4)

## Session 4 discoveries — reactive surface (effects · streams · sinks · operators)

0 findings remain of the 8 that surfaced while ruling F064–F089 (the reactive-structure batch): F141/F142/F144/F145/F147 were applied by the reactive-cell type-model overhaul, and F143/F146/F148 by the StreamPolicy overhaul. All are struck. Provenance differs from F001–F140 (those came from the original SPEC→LOG serialization; these are design discoveries made during adjudication), so they were tracked in their own block and not folded into the line-3 serialization count.


---

## Appendix: Spec-silent syntax questions (SPEC decides these nowhere)

Surfaced by the syntax-completeness sweep (reconstruction test over every §). These are NOT defects in existing text — they are decisions the SPEC never makes, so the log cannot serialize them without inventing language. Each needs a ruling; once ruled, add the decision to DECISION_LOG.md (next free number, by topic) and write the governing prose into SPEC.md. The log is reconstructible up to exactly this boundary.

Status key (added 2026-06-13 after the legacy-grammar pass): **✓** closed and applied to spec+log from the legacy grammar with owner approval; **◐** partially closed (sub-question remains); unmarked = still open.

**Re-audit 2026-06-21:** every open-*syntax* item the sweep raised below was in fact already decided in the log (the sweep mis-flagged them); they are now marked **✓ RESOLVED** with their governing entries, and one (record-pattern rest) was **amended** to add an opt-in `...`. The three remaining open *semantic* questions were themselves ruled and applied 2026-06-21 (multi-segment assignment LHS 013-248; tuple-component assignment 013-249; opt-in borrow-rootedness `-> T from v` 013-246/013-247) — emptying `BACKLOG.md`. This appendix is retained as the discovery record; do not re-litigate ✓ items.


RESOLVED (F038, session 1): there is **no** separate grammar document, and none is planned (DECISION_LOG 001-3/002-27; SPEC "there is no separate grammar document"). The ~23 SPEC sites that once delegated micro-syntax to "grammar §x.y" were **inlined or removed**. The items below are therefore open *syntax decisions the SPEC never makes* — not dangling deferrals to a missing file. (The open-*syntax* questions the sweep raised were all already resolved in the log; see the 2026-06-21 re-audit above. The three remaining open *semantic* items were ruled and applied 2026-06-21 — 013-246/247/248/249 — emptying `BACKLOG.md`.)

### Lexis & layout (lexical/grammatical syntax decisions, inlined into SPEC per F038)
- ✓ **RESOLVED — already specified (mis-flagged as open):** §1.4 fixes the identifier alphabet — a Unicode `Letter` or `_` start (no leading digit), Unicode letters/digits/`_` to continue, and `#` legal as a leading/infix/terminating character. 002-13/002-19, SPEC §1.4.
- ✓ Keyword reservedness per class: reserved-everywhere vs contextual (ledgered F005, ruled & applied).
- ✓ Indentation discipline (unit; tabs vs spaces) and line-continuation outside separated lists.
- ✓ **RULED (2026-06-20):** operator bodies follow the function rule (002-24) — inline single-expression body allowed (`operator double(s): s * 2`), indented block when declarations are present; 029-130, SPEC §13.17.4. (Declaration bodies were already settled: 002-23 record-likes indented-only, 002-24 functions/control-flow inline-or-block.)
- ✓ Leading zeros in decimal literals (`007`): valid or rejected?
- ✓ Float suffix on a based literal: does `0x1_f32` lex as `0x1F32` or as a float-suffixed hex literal?
- ✓ **RULED (2026-06-20):** literal suffixes unified — all appended directly (no underscore); the 14 numeric type names are pre-registered built-in suffixes (`5i32`) alongside the durations, and the underscore is solely a digit separator. 007-19/34/35, 005-209, §3.9.4/§4.3. (The `255_byte` underscore form is gone; alias/domain literals use `byte(255)` or a custom `@literal_suffix`.)

### Operators & precedence
- ✓ **RESOLVED — already specified (mis-flagged as open):** §4.4.7 gives the full 15-tier operator precedence/associativity table (`|>`; `or`; `and`; `not`; bitwise `|`/`^`/`&`; `..`; comparison; shifts; additive; multiplicative; prefix; postfix; `::`). 007-73/007-70.
- ✓ **RESOLVED — already specified:** §4.4.7 — `..` (tier 8) binds looser than arithmetic, so `0..n + 1` is `0..(n + 1)`. 007-73.
- ✓ **RESOLVED — already specified:** §4.4.7 tier 13 — unary `-`/`~`/`weak` (and the policy negations `-%`/`-|`/`-?`) are prefix, right-associative. 007-73.

### Strings, tuples, arrays
- ✓ **RULED (2026-06-20):** interpolation `{expr}` formats only via `Display` — no in-brace format-specifier mini-language (use method/stdlib calls); the sole literal-brace escape is `\{` (no `{{`); raw strings never interpolate. 012-149, SPEC §9.1.9.
- ✓ **RULED (2026-06-20):** `\xHH` is restricted to ASCII `0x00`–`0x7F`; `\u{…}` covers all higher Unicode scalars (`\x80`+ or a surrogate is a compile error), keeping the UTF-8/scalar invariants airtight. Multi-line plain/raw strings already settled (§9.1.3). 012-20, SPEC §9.1.3.
- ✓ Char-literal escape set: is `\'` (and `\"`) valid in a char literal? (§9.1.2 example vs §9.1.3 closed list — also ledgered.)
- ✓ **RULED (2026-06-20):** array construction — literal list `[e1,…,eK]`→`T[K]` and empty `[]` (F047) stand; added the comprehension `[for i in 0..N: expr]` (pure map → `T[N]`) and repeat `[for N: v]` (012-147/012-148, §9.3.1). Vec stays stdlib (no `Vec[…]=[]` language form); no `[0..N]` form (`Range` stays lazy, §12.2).
- ✓ Trailing commas in tuples of arity ≥ 2.

### Types, records, enums, conversions
- ✓ **RULED (2026-06-20):** explicit generic-enum variant instantiation parameterizes the enum — `Option[i32]::None` — with no separate `::[…]` turbofish (`Enum[args]` is already unambiguous, §9.3.2). 009-120, SPEC §6.2.3.
- ✓ **RULED (2026-06-20):** value-position `dyn` binds a single primary expression (parens for compounds); `dyn a + b` = `(dyn a) + b`. 008-72, SPEC §5.2.5. (Positions settled earlier by F037.)
- ✓ **RESOLVED — already specified:** §5.1 — `&` conjunction is commutative and associative, so `A & B & C` ≡ `(A & B) & C`, grouping-independent. (008-73 admits `Type[A & B]` by the same rule.)
- ✓ **RULED (2026-06-20):** `Type[…]` admits a trait intersection — `Type[Drivable & Insurable]` — via the same `&` conjunction as a bound (§5.1). 008-73, SPEC §5.7.1.
- ✓ **RESOLVED — already specified:** variant payload fields **do** take defaults (`Rectangle(width: f64 = 1.0, height: f64 = 1.0)`); construction may omit a defaulted field (`Rectangle(width: 10.0)`). 009-55/56/57, SPEC §6.2.1.
- ✓ Zero-field records / zero-variant enums: declarable? empty-body spelling?
- ✓ **AMENDED (2026-06-21):** record patterns were already specified (named call-site form, exhaustive, `field: _` to drop — 006-29/006-31, §3.5.7); owner added an **opt-in trailing `...` rest token** (non-binding, three dots, ≠ the `..` range op) that elides unlisted fields/components, applied uniformly to record, tuple, and variant-payload patterns. Amends 006-31; adds 006-32; SPEC §3.5.7/§9.2.2/§6.2.4.
- ✓ **RULED (2026-06-20):** a newtype is destructured by a positional pattern `UserId(n)` (mirrors tuple/variant patterns); the `T(value)` extractor coexists. 009-121, SPEC §6.3.2.
- ✓ **RULED (2026-06-20):** `with` is low-precedence, left-associative; chaining `a with x:1 with y:2` is legal (= the comma-list); a comma-list `with` used as a call argument must be parenthesized. 009-122, SPEC §6.1.5.
- Fallible-conversion turbofish example aside, no operator-specific turbofish gap (general rule governs).

### Modules
- ✓ `use … as` import renaming (referenced by §1.4/§4.7 but defined nowhere — also ledgered as an incoherence).
- ✓ Module-targeting `use` (`use root::audio::synth` then `synth::X`) — ledgered F045, ruled (a `use` leaf must be an importable name; module targets are rejected) & applied.
- ✓ Selection-list grammar closure: nested paths/globs inside `use root::(…)`.

### Reactive surface
- ✓ **RULED (C1, 2026-06-20):** the conditional-`Copy` surface is `satisfies Copy where T: Copy` — a `where` clause on the bodyless `satisfies` opt-in, no `fulfill` block — generalized to any methodless marker trait. Applied: DECISION_LOG 005-234/013-244 (and 005-230/§3.3.4 narrowed to method-bearing traits); SPEC §3.2/§11.4.1.
- ✓ **RULED (2026-06-21):** a place-assignment LHS may be a multi-segment path (`r.a.b = x`, `arr[i].field = y`, `t.0.field = z`); the place path resolves left-to-right (root, then each segment, index exprs in written order), the RHS is evaluated, then the innermost assignment runs — every projection an in-place place (013-209), no alias. 013-248, SPEC §11.11.
- ✓ **RULED (2026-06-21):** a tuple component is assignable in place through a `mut` tuple binding via the positional LHS `t.0 = x` (mirrors record-field assignment 013-224; not `Copy`-restricted; composes with multi-segment paths). 013-249, SPEC §11.11/§11.12.
- ✓ **RULED (2026-06-21):** the opt-in borrow-rootedness surface is the return annotation `-> T from v` (union `-> T from (v, w)`) — names the contributing input(s) directly via the `from` keyword, not lifetime-style; default `-> T` stays inferred; a compiler-verified assertion on concrete functions, the rootedness contract on abstract returns (013-66). The diagnostic-only elaborated `borrow_rooted_in(v)` form stays compiler-internal. 013-246/013-247, SPEC §11.7.5.
- ✓ **RULED (2026-06-20):** an inline `observe` (sub-expr / call arg) must be parenthesized; an arm body may be a single expression or an indented block (002-24). 016-277/278, SPEC §13.2.11.1.
- ✓ **RESOLVED — already specified (the "block forms silent" note was stale):** 022-119 — in a `when:` or `given` block the fallback arm (`otherwise:` / `default:`) must be the last arm; a non-last fallback is a compile error. Observe's `default:`-last is 016-253. SPEC §13.9.12 / §13.2.11.5.
- ✓ **RULED (2026-06-20):** connection-body member order is free (from/to or pairs each once); generic-connection placement writes type args inline (`Contains[i32]`); `/expr` whitespace is insignificant (`Node/arg` = `Node / arg`); pairs-form `match pair:` must be exhaustive. 019-76/77/78, 021-144, SPEC §13.6/§13.8.5. (Already-settled: body `:` placement 021-12; whitespace-bearing attr parenthesization 021-139.)
