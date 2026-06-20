# Ductus Spec Findings Ledger

**8 open findings remain (2026-06-20).** Originally 61 contradictions, ambiguities, incoherencies, and unsound inferences discovered in SPEC.md during its lossless serialization into DECISION_LOG.md; 53 have since been ruled and applied to SPEC.md + DECISION_LOG.md and struck — 51 across sessions B/C/E/F, plus F017 and F053 struck as **moot no-ops** (vacuous qualifiers, no spec change). The remaining 8 are the genuine design forks tracked as ruling tickets in `BACKLOG.md` §1 (F020, F129/F134, F133, F135, F138, F140), plus F136 riding with F138. Per the production protocol, NONE of these was silently resolved: each finding records the competing readings, the impact, and the contested entry wording (quarantined — withheld from DECISION_LOG.md until ruled on). Every finding was independently verified by at least two agents (extractor + skeleton-first reviewer; three for chunks 10/13/14, which received an additional reconstruction audit; items discovered after delivery — final-QA spot-audits and the syntax-completeness sweep — are marked in-row and were verified by the discovering auditor plus the orchestrator against SPEC.md); per-finding assessments live in the production reports (git history of `.declog-work/report/`).

Ruling protocol: for each finding, decide the intended reading, fix SPEC.md accordingly, then add the now-clean decision(s) to DECISION_LOG.md (next free number, placed by topic). Strike the finding from this ledger when resolved.

Counts by kind (original 61): 30 ambiguities · 24 incoherencies · 6 contradictions · 1 unsound-inference. **Open now (8):** 4 ambiguities (F129, F135, F138, F140) · 3 incoherencies (F020, F133, F136) · 1 contradiction (F134).

## High-priority digest (orchestrator's adjudication of the strongest items)

> **Historical snapshot.** This was the original top-20 adjudication. All but items 18–19 (the IR
> behavior-ID width and the §15.4.5-vs-§15.4.1 text-form forks) have since been ruled and applied;
> the live open set is the 8 findings in the Full ledger below.

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

## Full ledger — 8 open findings (of the original 61; the other 53 were ruled & applied and struck — see git history for their resolutions). Each line carries kind, sections, competing readings, impact, and quarantined wording.

F020 | [chunk 03] - kind: incoherence | sections: §3.5.1, §3.5.3 vs §3.3.1 | readings: named form is available for trait methods "at every call site" matched "by name", but impls rename non-receiver parameters (trait `Add` declares `fn add(a: Subject, b: Rhs)`; `fulfill Add for Vec3` writes `fn add(left: Vec3, right: Vec3)`) — trait parameter names are the named-form contract vs resolved-impl names govern, breaking named form at generic call sites | impact: legality of `v1.add(b: v2)` vs `v1.add(right: v2)`; whether fulfill matching must enforce parameter-name equality | quarantined: none (syntax-sweep discovery, orchestrator-verified)
F129 | [chunk 24] - kind: ambiguity | sections: §14.6.3 | readings: the behavior ID is literally a 32-bit value (content hash computed/truncated at u32 width, collisions possible) vs the ID is a full-width content-addressed hash and the stated u32 width conflicts with content-addressing | impact: determines ID width, collision exposure, and what the behavior table and hot-reload matching key on | quarantined: "Each behavior is identified by a stable u32 ID assigned at compile time, computed as a 32-bit content hash of the canonicalized typed IR." vs "Each behavior is identified by a stable content-addressed hash of its canonicalized typed IR; the u32 designation names the runtime handle, not the hash width."
F133 | [chunk 24] - kind: incoherence | sections: §14.7.1, §14.7.3 vs §11.8.5, §11.8.3, §11.3.1 | readings: drop bodies "may move fields out" and drop glue drops "fields left un-moved", but `move` accepts only bare l-value identifiers and field access never consumes — `move x.field` is admitted after all vs an unspecified construct vs dead spec text | impact: `move`'s operand grammar; whether `Drop` bodies can release fields; distinct from the ledgered §14.7.2 item | quarantined: none (syntax-sweep discovery, orchestrator-verified)
F134 | [chunk 25] - kind: contradiction | sections: §15.4.4, §14.6.3 | readings: behavior IDs are u32 integers with `B@aa10` a 32-bit hex rendering vs IDs are wider content-addressed hashes rendered abbreviated in the text form | impact: ID width fixes collision and text-form interop semantics across implementations | quarantined: none
F135 | [chunk 25] - kind: ambiguity | sections: §15.4.1, §15.4.5 | readings: gating is encoded per gated instance (compiled predicate plus `gate_parent` instance path) vs the text form uses named gate entries with `guards` lists that guarded entries reference by gate id (`gate App.g0 ... guards [App.print:0]`) | impact: the normative text-form shape for gates differs between readings | quarantined: A graph text-form gate entry carries `pred`, `inputs`, and a `guards` list of guarded instance paths: `gate App.g0 pred B@d4 inputs [App.show] guards [App.print:0]`. (§15.4.5)
F136 | [chunk 25] - kind: incoherence | sections: §15.4.4, §15.4.5 | readings: a desired-builder returns the whole desired record that the runtime scatters into per-field desired cells vs each desired cell's behavior returns that cell's own field-typed value | impact: the worked example types `App.print:0.text` as `str` while its behavior `B@d5` returns `%TextRec`, a mismatch under either strict reading | quarantined: none (the §15.4.4 ABI row "output: the desired record" is serialized as stated)
F138 | [chunk 25] - kind: ambiguity | sections: §15.4, §15.4.1, §15.4.5 | readings: the §15.4.1 abstract record (cell entries with observability/cadence/size fields plus separate dependency-edge lists) determines the normative text form, the §15.4.5 module text being illustrative vs the §15.4.5 text form (cell lines carrying role=/uses/inputs/init/depth inline, omitting observability/size/alignment, with no grammar given for the module/types/graph sections — only behaviors have one, §15.4.4) is itself the normative serialization | impact: §15.4 makes the text form normative and §15.7.3 defines interop against it, yet the graph section's concrete syntax is underdetermined | quarantined: A graph text-form cell line reads `cell App.total : i32 role=recurrent uses B@d2 inputs [App.count] depth 1 init 0`. (§15.4.5)
F140 | [chunk 25] - kind: ambiguity | sections: §15.4.1.4, §15.6.1, §15.7.3 | readings: §15.4.1.4 makes the `cross_thread_snapshot`→`confined` downgrade "implementation-defined" (so compilers A and B may emit different `observability` for the same path-matched cell) and warns the reverse "would silently change concurrency semantics", yet §15.6.1's reload classification (reload-safe / per-instance-restart / full-restart, computed from the diff alone) never lists an observability-class change, while §15.7.3 permits a host swapping one implementation's spec for another's — reading 1: an observability-class change on a carried-over cell is outside the diff's concern and silently honored vs reading 2: it must force a restart class, since silently flipping a cell's concurrency contract across hot reload violates the §15.4.1.4 no-silent-change intent | impact: whether cross-implementation hot reload can silently alter a path-matched cell's concurrency contract; the boundary of the reload-classification's input set | quarantined: none (entries transcribe each section; syntax-sweep discovery, orchestrator-verified against §15.4.1.4/§15.6.1)

## Session 4 discoveries — reactive surface (effects · streams · sinks · operators)

0 findings remain of the 8 that surfaced while ruling F064–F089 (the reactive-structure batch): F141/F142/F144/F145/F147 were applied by the reactive-cell type-model overhaul, and F143/F146/F148 by the StreamPolicy overhaul. All are struck. Provenance differs from F001–F140 (those came from the original SPEC→LOG serialization; these are design discoveries made during adjudication), so they were tracked in their own block and not folded into the line-3 serialization count.


---

## Appendix: Spec-silent syntax questions (SPEC decides these nowhere)

Surfaced by the syntax-completeness sweep (reconstruction test over every §). These are NOT defects in existing text — they are decisions the SPEC never makes, so the log cannot serialize them without inventing language. Each needs a ruling; once ruled, add the decision to DECISION_LOG.md (next free number, by topic) and write the governing prose into SPEC.md. The log is reconstructible up to exactly this boundary.

Status key (added 2026-06-13 after the legacy-grammar pass): **✓** closed and applied to spec+log from the legacy grammar with owner approval; **◐** partially closed (sub-question remains); unmarked = still open.

**The still-open items below are now tracked as proper ruling tickets in `BACKLOG.md`** (Section 2 open-syntax §7–17, Section 3 open-semantic §18–20). This appendix is retained as the discovery record; do not re-litigate ✓ items.


RESOLVED (F038, session 1): there is **no** separate grammar document, and none is planned (DECISION_LOG 001-3/002-27; SPEC "there is no separate grammar document"). The ~23 SPEC sites that once delegated micro-syntax to "grammar §x.y" were **inlined or removed**. The items below are therefore open *syntax decisions the SPEC never makes* — not dangling deferrals to a missing file — and are now carried as ruling tickets in `BACKLOG.md` (open-syntax §7–17, open-semantic §18–20).

### Lexis & layout (lexical/grammatical syntax decisions, inlined into SPEC per F038)
- ◐ Base identifier alphabet, leading-digit rule, Unicode allowance, case-sensitivity (§1.4 legislates only the `#` rule).
- ✓ Keyword reservedness per class: reserved-everywhere vs contextual (ledgered F005, ruled & applied).
- ✓ Indentation discipline (unit; tabs vs spaces) and line-continuation outside separated lists.
- ✓ **RULED (2026-06-20):** operator bodies follow the function rule (002-24) — inline single-expression body allowed (`operator double(s): s * 2`), indented block when declarations are present; 029-130, SPEC §13.17.4. (Declaration bodies were already settled: 002-23 record-likes indented-only, 002-24 functions/control-flow inline-or-block.)
- ✓ Leading zeros in decimal literals (`007`): valid or rejected?
- ✓ Float suffix on a based literal: does `0x1_f32` lex as `0x1F32` or as a float-suffixed hex literal?
- ✓ **RULED (2026-06-20):** literal suffixes unified — all appended directly (no underscore); the 14 numeric type names are pre-registered built-in suffixes (`5i32`) alongside the durations, and the underscore is solely a digit separator. 007-19/34/35, 005-209, §3.9.4/§4.3. (The `255_byte` underscore form is gone; alias/domain literals use `byte(255)` or a custom `@literal_suffix`.)

### Operators & precedence
- Full precedence/associativity table among `+ - * / \ %`, shifts, `& ^ |`, comparisons, `is`/`is not`, the `%`/`|`/`?` cast-policy families, and `..` — SPEC states only fragments ("arithmetic binds tighter"; `|`≈`|>` low/left).
- Range operator `..` precedence vs arithmetic (compound bounds are always parenthesized in examples; bare `0..N + 1` undecided).
- Unary `~` position (prefix vs postfix): no value-level example anywhere (rides with the ledgered `-%`/`-|` unary-position item).

### Strings, tuples, arrays
- ✓ **RULED (2026-06-20):** interpolation `{expr}` formats only via `Display` — no in-brace format-specifier mini-language (use method/stdlib calls); the sole literal-brace escape is `\{` (no `{{`); raw strings never interpolate. 012-149, SPEC §9.1.9.
- ✓ **RULED (2026-06-20):** `\xHH` is restricted to ASCII `0x00`–`0x7F`; `\u{…}` covers all higher Unicode scalars (`\x80`+ or a surrogate is a compile error), keeping the UTF-8/scalar invariants airtight. Multi-line plain/raw strings already settled (§9.1.3). 012-20, SPEC §9.1.3.
- ✓ Char-literal escape set: is `\'` (and `\"`) valid in a char literal? (§9.1.2 example vs §9.1.3 closed list — also ledgered.)
- ✓ **RULED (2026-06-20):** array construction — literal list `[e1,…,eK]`→`T[K]` and empty `[]` (F047) stand; added the comprehension `[for i in 0..N: expr]` (pure map → `T[N]`) and repeat `[for N: v]` (012-147/012-148, §9.3.1). Vec stays stdlib (no `Vec[…]=[]` language form); no `[0..N]` form (`Range` stays lazy, §12.2).
- ✓ Trailing commas in tuples of arity ≥ 2.

### Types, records, enums, conversions
- ✓ **RULED (2026-06-20):** explicit generic-enum variant instantiation parameterizes the enum — `Option[i32]::None` — with no separate `::[…]` turbofish (`Enum[args]` is already unambiguous, §9.3.2). 009-120, SPEC §6.2.3.
- ✓ **RULED (2026-06-20):** value-position `dyn` binds a single primary expression (parens for compounds); `dyn a + b` = `(dyn a) + b`. 008-72, SPEC §5.2.5. (Positions settled earlier by F037.)
- Record-intersection chaining `A & B & C` / parenthesized RHS (associativity stated only for bound-position conjunction).
- ✓ **RULED (2026-06-20):** `Type[…]` admits a trait intersection — `Type[Drivable & Insurable]` — via the same `&` conjunction as a bound (§5.1). 008-73, SPEC §5.7.1.
- Named enum payload field defaults (`Rectangle(width: f64 = 1.0)`).
- ✓ Zero-field records / zero-variant enums: declarable? empty-body spelling?
- Record pattern surface: concrete shape, whether every field must be bound, rest-pattern token (none exists anywhere).
- ✓ **RULED (2026-06-20):** a newtype is destructured by a positional pattern `UserId(n)` (mirrors tuple/variant patterns); the `T(value)` extractor coexists. 009-121, SPEC §6.3.2.
- ✓ **RULED (2026-06-20):** `with` is low-precedence, left-associative; chaining `a with x:1 with y:2` is legal (= the comma-list); a comma-list `with` used as a call argument must be parenthesized. 009-122, SPEC §6.1.5.
- Fallible-conversion turbofish example aside, no operator-specific turbofish gap (general rule governs).

### Modules
- ✓ `use … as` import renaming (referenced by §1.4/§4.7 but defined nowhere — also ledgered as an incoherence).
- ✓ Module-targeting `use` (`use root::audio::synth` then `synth::X`) — ledgered F045, ruled (a `use` leaf must be an importable name; module targets are rejected) & applied.
- ✓ Selection-list grammar closure: nested paths/globs inside `use root::(…)`.

### Reactive surface
- ✓ **RULED (C1, 2026-06-20):** the conditional-`Copy` surface is `satisfies Copy where T: Copy` — a `where` clause on the bodyless `satisfies` opt-in, no `fulfill` block — generalized to any methodless marker trait. Applied: DECISION_LOG 005-234/013-244 (and 005-230/§3.3.4 narrowed to method-bearing traits); SPEC §3.2/§11.4.1.
- Multi-segment assignment LHS (`r.a.b = x`, `arr[i].field = y`) and the FieldAssign/IndexAssign desugaring order (only single-segment forms shown).
- Tuple-component assignability through a `mut` binding and its LHS form.
- Explicitly-written elaborated borrow signatures: concrete surface (only "Schematically: `fn f(borrow v: T) -> borrow_rooted_in(v) T`" given).
- ✓ **RULED (2026-06-20):** an inline `observe` (sub-expr / call arg) must be parenthesized; an arm body may be a single expression or an indented block (002-24). 016-277/278, SPEC §13.2.11.1.
- `default:`/catch-all position in `when:` multi-way and `given` blocks (also ledgered — observe requires last; block forms silent).
- ✓ **RULED (2026-06-20):** connection-body member order is free (from/to or pairs each once); generic-connection placement writes type args inline (`Contains[i32]`); `/expr` whitespace is insignificant (`Node/arg` = `Node / arg`); pairs-form `match pair:` must be exhaustive. 019-76/77/78, 021-144, SPEC §13.6/§13.8.5. (Already-settled: body `:` placement 021-12; whitespace-bearing attr parenthesization 021-139.)
