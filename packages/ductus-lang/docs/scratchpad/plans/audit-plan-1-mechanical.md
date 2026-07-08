# Audit Remediation — Part 1 of 3: Mechanical fixes

*2026-07-07 · from the design audit of 2026-07-06 (report + addendum in docs/scratchpad/audits/). Corpus pin 1b0c6b8f2a.*

Every item in this document has exactly one correct fix — no design choice hides inside any of them. You approve this document as ONE batch; agents then execute the edits LOG-first (change the decision log entry, then conform the spec section it cites) and run each item's verification check. Items are grouped so that sections of the decision log are touched together, which keeps the renumbering (the log renumbers entries on insert/remove by design) reviewable.

If you disagree that an item is truly mechanical, strike it and it moves to the individual-decisions document.

#### M-01 · P005 (MED) — One dot-call rule says it never adds an implicit ownership move; another rule adds exactly such a move for a common case, so the "never" is false.

Ductus lets you call a method as `v.m(args)`. There is a general rule that this sugar is purely positional — the receiver becomes the first argument and nothing about ownership changes. To consume the receiver you must write `(move v).m()` explicitly.

But there is a separate, later rule: when `v` is a `mut` binding, the method takes its receiver by `own`, and the method returns `Subject`, then a bare `v.m(args)` quietly rewrites to `v = (move v).m(args)`. That is an implicit ownership move triggered by the dot-call form — precisely what the general rule swears never happens.

```
// General rule: v.m() moves nothing; you must write:
(move v).into_sorted()
// Rebind-sugar rule: for mut/own-receiver/returns-Subject, a bare call auto-moves:
v.push(3)   // desugars to  v = (move v).push(3)
```

The two rules are not in real conflict about behavior — the rebind sugar is intended and correct. The only defect is that the general rule states its "no implicit ownership" claim as absolute and never carves out the one exception. The rebind sugar is the authored, self-consistent behavior; the general rule's wording is simply stale.

**Edit:** LOG-first. Amend the entry that says the dot-call "introduces no implicit ownership rule of its own" (013-130) so the claim is no longer absolute: append a carve-out clause naming the one exception — the receiver-rebind sugar for a `mut` binding whose receiver parameter is `own` on a call returning `Subject` (as defined in 013-223/013-226). Do not touch 013-223/013-226 (they are correct). Then conform the SPEC section on dot-call sugar (§11.8.3) to restate the same carve-out and cross-reference the receiver-rebind-sugar section (§11.11.3). No behavior changes; only 013-130's over-broad wording is narrowed to match the existing sugar.

**Verify:** `grep -n '013-130' DECISION_LOG.md` shows the entry now references the mut/own/Subject receiver-rebind exception (e.g. contains "except" and "rebind" or a §11.11.3 pointer); 013-223 and 013-226 are byte-unchanged (`git diff` touches only 013-130's line). In SPEC §11.8.3, `grep -n` finds the carve-out sentence and a §11.11.3 xref. Re-read 013-130 and confirm it no longer asserts "no implicit ownership rule" without qualification.

#### M-02 · P006 (LOW) — One rule says `satisfies Trait` with no `fulfill` block is always a compile error; three other rules allow exactly that for marker and umbrella traits.

In Ductus a type declares conformance with `satisfies Trait` and normally must back it with a `fulfill Trait for Type` block. One entry states this as absolute: `satisfies` without a reachable `fulfill` is a compile error, no exceptions.

But three other entries describe legal `satisfies` clauses that involve no `fulfill` at all: a marker trait's `satisfies` is checked only for the trait's existence; a methodless marker like `satisfies Copy where T: Copy` explicitly says "no `fulfill` block is involved"; and pure-requirement umbrella traits are satisfied automatically when their requirements are.

```
satisfies Copy where T: Copy   // legal, no fulfill block — but the absolute rule calls this a compile error
```

**Owner correction (2026-07-07): there is a FOURTH exempt kind** — a trait all of whose methods, direct and inherited, have default bodies. Verification against the corpus: this rule does not exist anywhere as stated. The nearest neighbor is the auto-satisfaction entry (005-109), but it is about satisfaction with no `satisfies` clause at all, and it ranges over "every method it declares" — which, per the method-origin rule (005-30: every trait method has exactly one origin, the trait that originally declared it), excludes inherited methods. So the fourth exception is a new rule and must be added.

**Edit:** LOG-first. Amend the entry stating that `satisfies` without a reachable `fulfill` is a compile error (005-65) to state its exceptions self-containedly: the pairing requirement does not apply to (1) marker traits (per 005-6), (2) methodless marker traits (per 005-72), (3) pure-requirement umbrella traits (per 009-9), or (4) a trait all of whose methods — direct and inherited — have default bodies [owner-added rule]. Add exception (4) as a clause inside the amended 005-65, not as a standalone new entry (avoids Invariant-1 renumbering of section 005). Do not touch 005-6, 005-72, 009-9, 005-109. Then conform the SPEC section on the satisfies/fulfill pairing rule (§3.2) to state the same four exceptions.

**Flag for owner (do not edit):** the new exception (4) speaks of "direct and inherited" methods, while auto-satisfaction (005-109) ranges only over methods a trait *declares* — read with 005-30, a child trait with an inherited undefaulted method could literally auto-satisfy under 005-109. Whether 005-109 should also be re-scoped to "direct and inherited" is a separate design call; surfaced, not resolved.

**Verify:** `grep -n '005-65' DECISION_LOG.md` shows the entry now names four exceptions including "direct and inherited" default-body traits. 005-6, 005-72, 009-9, 005-109 are byte-unchanged in `git diff`. SPEC §3.2 `grep -n` finds the four-exception clause. Re-read 005-65 to confirm it no longer reads as an unqualified "always a compile error."

#### M-03 · P027 (LOW) — Four statements of one rule disagree on scope: three say "any fold in an operator body forces the indented block," one restricts it to a fold that is the body's final expression.

A `fold` in an operator body forces that body into indented-block form — it can't sit inline after the colon. Four places state this rule, and they don't agree on when it fires. One entry (the operator-body-form log entry) scopes it narrowly: only when the fold is the body's *final expression*. The other three — the duplicate log entry that splices `fold` into the operator-body item list, and both spec leaves (the operator-body section and the section that defines where a fold may appear) — scope it broadly: any `fold` written anywhere in an operator body forces the block, regardless of position.

The broad reading is the correct one: a `fold` is never written inline in any position, and the outcomes coincide anyway (a non-final fold body-item already implies a multi-item indented block). So the majority wording is authoritative and the outlier is just narrower.

Recommended: broaden the outlier to match the majority, because three of the four statements plus the underlying "a fold is never inline" fact already say any fold in the body forces the block; narrowing the three to match the one outlier would contradict that fact.

**Edit:** LOG-first. Amend the operator-body-form entry (029-30) so its fold clause reads "any `fold` in the body forces the indented block" (dropping the "whose final expression is" restriction), matching 035-8 and both SPEC leaves. Do not touch 035-8. SPEC §13.17.4 and §13.21.6 already state the broad scope ("A fold in an operator body forces the indented block form") and need no change.

**Verify:** `grep -n '029-30' DECISION_LOG.md` shows the fold clause no longer contains "final expression" and instead says any fold in the body forces the block. `git diff` touches only 029-30 (035-8 unchanged). Cross-check SPEC §13.17.4 (19743-19745) and §13.21.6 (23175-23177) both already read "A fold in an operator body forces the indented block form" (any position) — no "final expression" qualifier remains anywhere. Finding: P027.

#### M-04 · F001 (MED) — The log carves out declared `fn` references as const-eligible; the spec blocks them. Owner ruling (2026-07-07): functions are not const values, at all — the carve-out retires.

The log entry on const-ineligible types (004-82) ends with "a declared `fn` reference is const-eligible and does not disqualify a containing type." The spec's const-ineligible list already blocks "function references or closures with captured runtime state" (SPEC.md:728). The audit read this as the spec drifting from the log; the owner has ruled the opposite: the spec bullet is the correct side, and the log's carve-out is retired.

**Edit:** LOG-first. Amend 004-82 (DECISION_LOG.md:265): replace the trailing carve-out "; a declared `fn` reference is const-eligible and does not disqualify a containing type" so the entry instead includes function references among the ineligible types — e.g. "..., types containing function references or anonymous/capturing closures that hold captured runtime state, and `dyn` trait objects are not `const`-eligible." SPEC §2.4.1.3 line 728 already conforms — leave byte-unchanged. Sweep both docs for any other site asserting fn-reference const-eligibility or showing a `const` holding a function reference in an example; report unexpected hits as findings, do not silently edit them.

**Flag for owner (do not edit):** the corpus distinguishes capturing closures (banned) from the now-banned function references; whether a NON-capturing closure remains const-eligible is not stated anywhere and is not settled by this ruling. Surfaced, not resolved.

**Verify:** `grep -n '^004-82\.' DECISION_LOG.md` shows function references among the ineligible list and no const-eligible carve-out; `sed -n '728p' SPEC.md` is byte-unchanged; `grep -in 'const-eligible' DECISION_LOG.md SPEC.md` shows no site asserting fn-reference eligibility.

#### M-05 · F015 (LOW) — A log entry cites "the methodless-trait waiver" as a named rule, but nothing defines a waiver by that name.

Entry 005-72 leans on "the methodless-trait waiver" as if established, but the only named waiver in scope is the fulfill-without-satisfies waiver (005-67). Entries must be self-contained; the reader cannot pin what the term denotes.

**Recommended:** replace the parenthetical in 005-72 with a self-contained restatement of the auto-satisfaction condition it relies on, because the intended rule (auto-satisfaction of a methodless marker) is stated elsewhere but never named "methodless-trait waiver".

**Edit:** Log-first. Amend DECISION_LOG.md:413 (005-72): replace the phrase "auto-satisfiable under the methodless-trait waiver" with an inline self-contained restatement (a methodless marker trait auto-satisfies without a `fulfill` block). No new term is introduced; no id renumber. Spec §3.2: conform only if it repeats the coined term — grep §3.2 for "methodless-trait waiver" and align if present.
**Verify:** `grep -c 'methodless-trait waiver' DECISION_LOG.md SPEC.md` returns 0 in both.

#### M-06 · F028 (MED) — A log entry says mixed int/float always widens the integer to float, contradicting the entry that forbids i128/u128 widening to any float.

Entry 007-90 is written as a universal with no carve-out. Entry 007-107 says i128/u128 never widen implicitly to any float, so `i128_val + 1.0` must be a compile error, not a silent widen. An implementer reading only 007-90 wrongly accepts it.

**Recommended:** add the i128/u128 exception to 007-90, because the entry must be self-contained (atomic) and 007-107 supplies the carve-out it omits.

**Edit:** Log-first. Amend DECISION_LOG.md:708 (007-90) to state the mixed int/float widening applies except where the float-widening rules forbid it (i128/u128 never widen to float, per 007-107). Spec §4.4.5 already defers to §4.5 and needs no change; confirm §4.4.5/§4.5.2 still exclude i128/u128 after the log edit.
**Verify:** `grep -n '^007-90\.' DECISION_LOG.md` shows the new exception clause naming i128/u128; the entry no longer reads as an unconditional universal.

#### M-07 · F029 (MED) — Spec §4.5 tells the reader to use `as` for narrowing conversions, but the same spec and the log say `as` is not a cast.

Spec §4.5 (line 3372) says non-lossless conversions "require explicit `as`". But §4.4.7 (line 3363) and log 007-146 say `as` is a naming keyword only; explicit conversion uses the call forms `T()`/`T%()`/`T|()`/`T?()`. A reader following §4.5 writes `x as u8`, which the language rejects.

**Recommended:** update spec §4.5 to reference the `T()`/`T%()`/`T|()`/`T?()` call forms instead of `as`, because the log and §4.7 already fixed `as` as naming-only — this is stale wording, a spec-conformance fix with no log change.

**Edit:** No log change (007-145/007-146 correct). Spec §4.5: edit SPEC.md:3371-3372, replace "require explicit `as` (§4.7)" with a reference to the explicit-conversion call forms `T()`/`T%()`/`T|()`/`T?()` per §4.7, keeping the `From`/`Into` clause.
**Verify:** `grep -n 'as' SPEC.md` around line 3372 no longer presents `as` as the narrowing-cast mechanism; the call forms are named instead.

#### M-08 · F030 (LOW) — The spec operator-precedence table omits `delete` from the prefix tier, which the log lists there.

Log 007-74 places `delete` among the prefix operators (`-`/`~`/`handle`/`handle!`/`portal`/`delete`) at tier 13, and 007-239 confirms `delete` uses prefix syntax. The spec's tier-13 row (line 3353) drops `delete`, leaving its binding strength unspecified in the normative table.

**Recommended:** add `delete` to the spec's tier-13 prefix row, because the spec must conform to the log's precedence list — a spec-only fix.

**Edit:** No log change (007-74 correct). Spec §4.4.7: edit SPEC.md:3353 to add `delete` to the tier-13 prefix row alongside `-`, `~`, `handle`, `handle!`, `portal`.
**Verify:** `sed -n '3353p' SPEC.md` includes `delete` in the tier-13 prefix operator list.

#### M-09 · F032 (LOW) — The log's phrase "the shift right operand" is ambiguous: the count operand of any shift, or only the operand of `>>`.

Entry 007-92 says "the shift right operand may be any unsigned integer type narrower than or equal to u32". "Shift right operand" can mean the right-hand (count) operand of any shift, or the operand of the right-shift `>>`. The spec footnote applies the rule to both `<<` and `>>`. The entry should be unambiguous on its own.

**Recommended:** reword 007-92 to "the shift-count (right) operand of `<<` and `>>`", because the spec footnote's scope is both operators and that reading makes the entry self-contained.

**Edit:** Log-first. Amend DECISION_LOG.md:710 (007-92): replace "The shift right operand" with wording naming the shift-count (right-hand) operand of both `<<` and `>>`. Spec §4.4.6 footnote (SPEC.md:3306-3308) already covers both; confirm it still matches after the log edit.
**Verify:** `grep -n '^007-92\.' DECISION_LOG.md` shows the operand named as the count operand of both `<<` and `>>`, with no bare "shift right operand".

#### M-10 · F070 (LOW) — The spec's interpretation-bootstrap example drops the `.audio` projection the log mandates be written explicitly, and its comment calls the projection both explicit and elided.

Log 017-273 mandates projection "written explicitly" and shows `render(song).audio |> audio_out`. Spec §13.17.7 writes `render(song) |> audio_out` (no `.audio`) while the comment says the projection "is explicit (here `render(song).audio`, elided)" — self-contradictory.

**Recommended:** make the spec example write the projection explicitly (`render(song).audio |> audio_out`) and fix the comment, because the log's canonical form requires explicit projection and "explicit yet elided" cannot both hold.

**Edit:** No log change (017-273 correct). Spec §13.17.7: edit SPEC.md:19943 to `audio = render(song).audio |> audio_out`, and edit the comment at SPEC.md:19945-19946 to drop the "elided" claim (projection is written explicitly).
**Verify:** `sed -n '19943p' SPEC.md` shows `render(song).audio`; the comment no longer contains "elided".

#### M-11 · F071 (MED) — A log entry says every checked duration operator returns `Option[duration]`, but checked division of two durations must return `Option[f64]`.

In Ductus, `duration / duration` yields an f64 ratio (log 012-148). So `duration /? duration` must yield `Option[f64]`. Entry 012-159 carves this out. But entry 012-164 flatly says all checked variants `+?`,`-?`,`*?`,`/?`,`%?` return `Option[duration]` — no carve-out. The spec repeats the same split: §9.4.1.2 gets it right, §9.4.1.3 gets it wrong. The same expression `d1 /? d2` gets two different types depending on which rule the implementer reads.

The fix is uniquely determined — the correct return types are already pinned by 012-148 (ratio is f64) and 012-159 (the carve-out). 012-164 and spec §9.4.1.3 are simply the incomplete side and must be brought into line.

```
d1 /? d2   // two durations  -> Option[f64]   (ratio)
d  /? 2.0  // scale by Numeric -> Option[duration]
```

**Recommended:** amend 012-164 and spec §9.4.1.3 to state that `/?` (and `%?`) over two durations returns `Option[f64]`, while `/?` scaling by a Numeric returns `Option[duration]` — because 012-148 fixes the duration/duration result as f64 and 012-159 already encodes exactly this split; 012-164's blanket wording is the defect.

**Edit:** Log-first. Amend DECISION_LOG.md:1359 (012-164): replace the blanket "return `Option[duration]`" with the split matching 012-159 — `/?` and `%?` over two durations return `Option[f64]`; the other checked variants (and Numeric-scaling `/?`/`%?`) return `Option[duration]`. Spec §9.4.1.3: edit SPEC.md:7146-7150 to carry the same carve-out already present at §9.4.1.2 (SPEC.md:7132-7134). 
**Verify:** `grep -n '^012-164\.' DECISION_LOG.md` shows the duration/duration → `Option[f64]` carve-out; SPEC.md:7146-7150 and SPEC.md:7132-7134 both state `Option[f64]` for duration/duration and agree.

#### M-12 · F126 (MED) — The type-level const example reads `Log::type`/`Delay::type`/`T::type`, but the const is declared `kind` and no `type` const exists (`type` is even a reserved keyword).

The §13.2.5.2 node bodies declare `const kind: string = "@action/log"`, yet the type-level access example reads `Log::type`/`Delay::type`/`T::type`. The error also lives in the log: 016-124 shows `Log::type` and 016-125 shows `T::type`. Correct form is `::kind`.

**Recommended:** change all three read sites to `::kind` in both docs, because every declaration names the const `kind` and no `type` const is declared anywhere; `::type` reads a nonexistent, keyword-named const.

**Edit:** Log-first. Amend DECISION_LOG.md:1977 (016-124) `Log::type` → `Log::kind`, and DECISION_LOG.md:1978 (016-125) `T::type` → `T::kind`. Spec §13.2.5.2 conforms: edit SPEC.md:12239-12243, `Log::type` → `Log::kind`, `Delay::type` → `Delay::kind`, `T::type` → `T::kind`.
**Verify:** `grep -c '::type' DECISION_LOG.md SPEC.md` returns 0 in both.

#### M-13 · F130 (LOW) — A rejected-write example writes the Option constructor lowercase `some(...)`, but the constructor is `Some`, which sneaks a second unrelated "undefined identifier" error into an example meant to show only one error.

*Revision 2 (2026-07-08): the audit counted two defective fences; a protocol halt during execution found THREE identical ones (SPEC.md:22306, 22389, 22818). All three are the same defect class; the item now covers all three. Orchestrator ruling: mechanical undercount, proceed — with the per-fence context check below as the safeguard.*

**Edit:** SPEC-only fix; no LOG entry changes (the finding lists no LOG anchor and the constructor casing `Some` is already fixed in the log entry that defines positional enum-variant construction). In SPEC section 13.19: first Read ~8 lines around EACH of the three fences containing `f.response = some(custom_response)`; if any of the three is a deliberate demonstration of an undefined-identifier error (its surrounding prose calls out the lowercase `some` itself as the error being taught), HALT and surface. Otherwise change all three so the constructor is capitalized to `Some(custom_response)`, matching the Option variant casing used everywhere else.
**Verify:** `grep -n 'some(custom_response)' SPEC.md` returns zero lines; `grep -c 'Some(custom_response)' SPEC.md` returns 3.

*Execution record: a fourth occurrence of the same defect existed in the LOG itself (the write-to-observed-cell entry, now 031-55) and was fixed in the same batch — so despite the "SPEC-only" wording above, the executed fix touched both documents. Recorded for the paper trail.*

#### M-14 · F152 (LOW) — The log entry that defines the shape of an observe arm lists on-clause + optional where + colon + expression, but leaves out the optional `as` binder that the spec's full arm grammar includes.
**Edit:** LOG-first. In the log entry that defines an observe arm's structure (the one whose example is `on T3 where C: expr_filtered`), add the optional `as` binder to the stated structure so it reads on-clause + optional `where` filter + optional `as` binder + colon + expression, matching the full arm grammar `on <trigger> [where C] [as <binder>]:` in the spec section it cites (13.2.11.1). No spec change needed: the spec already carries the full grammar; this only brings the log grammar entry up to it.
**Verify:** the observe-arm structure entry (currently 016-243) contains the substring `as` binder in its structural list; the spec grammar line `on <trigger> [where C] [as <binder>]:` is unchanged.

#### M-15 · F169, F191 (LOW) — The reconciler-registration rule is deliberately restated verbatim across seven sites, but one site joins its two clauses with ", and" while the other six (and the spec) use a semicolon — verbatim drift in a canon that is supposed to read identically. (F169 and F191 are the same defect on the same entry; F191 just names a superset of the sibling sites.)
**Edit:** LOG-only, single entry. In the log entry that adds a new effect type by declaring an `effect` (the "§13.1" reconciler-registration entry, currently 015-39), change the clause join from `channels (\`signal\`/\`stream\`), and an interior effect` to `channels (\`signal\`/\`stream\`); an interior effect`, so its punctuation matches the six sibling canon sites (027-80, 031-128, 031-143, 033-124, and the two others) and the spec's own canon sentence in section 13.1, all of which use the semicolon form. No other entry changes; no spec change (spec 13.1 already uses the semicolon).
**Verify:** `grep -n 'signal\`/\`stream\`), and an interior effect' DECISION_LOG.md` returns zero lines; the seven canon sites and SPEC 13.1 all read `(\`signal\`/\`stream\`); an interior effect`.

#### M-16 · F178 (MED) — The spec's reserved-instance-field list appears in two places that disagree: one spot names six fields (adding `incoming` and `outgoing`), the other names four; the log names four in both of its entries.

Background: a node or connection instance has a fixed set of reserved fields that resolve by bare name in expression position. The log settles this at four: `from`, `to`, `pair`, `exposition`. The log uses `incoming:`/`outgoing:` only as placement-clause keywords, never as expression-position reserved fields. The spec section that summarizes the instance-body scope (13.7.1) lists six by adding `incoming` and `outgoing`; the spec section that actually defines the reserved fields (13.7.5) lists four. So the six-item list is the lone outlier: it diverges from the log and self-contradicts the other spec section.

The question is which count is authoritative. The edit protocol makes spec conform to log, and the log is unanimous:

- Option A — four fields (`from`, `to`, `pair`, `exposition`). Matches both log entries and spec section 13.7.5.
- Option B — six fields. Only spec section 13.7.1 says this, and it contradicts a sibling spec section.

**Recommended: Option A (four fields), because** the log is the decision-of-record and both its entries say four, and the sole six-item source already self-contradicts the spec's own reserved-field definition. `incoming`/`outgoing` are placement-clause keywords in the log, not bare-name expression fields, so listing them as reserved instance fields is the defect.
**Edit:** LOG unchanged (already correct at four). SPEC-only: in section 13.7.1, change the reserved-endpoint/structure-field list from `(\`from\`, \`to\`, \`incoming\`, \`outgoing\`, \`pair\`, \`exposition\`)` to `(\`from\`, \`to\`, \`pair\`, \`exposition\`)`, matching section 13.7.5 and the log.
**Verify:** `grep -n 'incoming.*outgoing.*pair.*exposition' SPEC.md` returns zero lines; SPEC 13.7.1 and 13.7.5 both list exactly `from`, `to`, `pair`, `exposition`.

#### M-17 · F179 (LOW) — A log entry (and its spec mirror) cite section 6.2.4 for `match` exhaustiveness, but 6.2.4 covers pattern forms only; exhaustiveness is defined in section 6.2.5. The pointer aims at the wrong section.
**Edit:** LOG-first. In the pairs-form-match exhaustiveness entry (currently 019-46, cited to §13.6.1.3), change the parenthetical cross-reference `as any \`match\` (§6.2.4)` to `(§6.2.5)`, since 6.2.5 is the section titled "Exhaustiveness checking" and 6.2.4 is "Pattern matching" (verified by section headers). Then update the spec mirror in section 13.6.1.3 (the bullet reading "exactly as any `match` (§6.2.4)") to cite `(§6.2.5)` identically.
**Verify:** neither the log entry nor the SPEC 13.6.1.3 mirror cites `(§6.2.4)` for exhaustiveness; both cite `(§6.2.5)`.

#### M-18 · F200 (MED) — A log entry cites "the C15 naming rule," but `C15` is defined nowhere in the log or spec — it is an internal amendment-plan change-set label that leaked into a normative entry, leaving the entry not self-contained.
**Edit:** LOG-first. In the `event_count` running-count entry (currently 030-137, cited to §13.18.9), replace `the \`event_\` prefix applies the C15 naming rule to this specialized tally` with a reference to the actual normative rule the spec attributes it to — the tally-accessor naming rule (the one where a bare `count` is the unified element tally and a prefixed `x_count` names a specialized tally exempt from that unification), which lives at §9.3.7. Concretely, state it self-contained: the `event_` prefix marks this as a specialized tally under the tally-accessor naming rule (§9.3.7), so it is exempt from the bare-`count` element-tally unification. The spec section 13.18.9 already attributes the count-name reservation to "the element-tally rule, §9.3.7," so no spec change is required; if desired, confirm SPEC 13.18.9 wording matches.
**Verify:** `grep -n 'C15' DECISION_LOG.md` and `grep -n 'C15' SPEC.md` both return zero lines; the edited entry cites §9.3.7 / the tally-accessor naming rule.

> Three findings originally drafted here (F181, F202, F203) turned out to hide real choices and were moved to Part 3 as items I-78, I-79, I-80.

#### M-19 · F208 (LOW) — the hot-reload portal entry mislabels the cell-identity criterion as `kind` when the rule it invokes keys on `type`, and `kind` is a separately-defined term.

*Revision 2 (2026-07-08): a protocol halt established that the item's SPEC instruction was unexecutable — 033-113's cited section (§15.4.6, IR grammar) contains no portal-preservation prose because the citation itself is broken (that is finding F207, deferred to Part 2 item D-06), and the genuine portal prose at §13.3.6.3:14430 already reads correctly ("by the same path-based identity rule as cells") and never used "kind". Ruling: the SPEC half is a verified no-op; only the LOG edit applies.*

**Edit:** In the LOG entry for `Portal[T]` preservation across hot reload (033-113), change `kind` to `type` so the criterion matches the same-path-and-type rule it invokes in the same sentence. This edit may already be on disk from the pre-halt attempt — verify it is in place and apply only if missing (idempotent). NO SPEC edit: verify §13.3.6.3's portal prose already reads path-based/type identity and contains no `kind` criterion, and leave §15.4.6 alone (its repoint belongs to Part 2 D-06/F207).
**Verify:** `grep -n '033-113' DECISION_LOG.md` shows the entry now reads `path and type`; `grep -c 'path and kind' DECISION_LOG.md` returns 0; grep of SPEC §13.3.6.3's portal-preservation prose shows no `kind`-as-criterion; `git diff` shows no SPEC hunk from this item.

#### M-20 · F214 (LOW) — the fold-lowering entry frames `fold` as 'introducing/extending' the cell-kind enum, but the LOG is a steady-state set of simultaneously-true decisions and a sibling entry already lists `fold` as a settled enum member.
**Edit:** Restate the LOG entry on IR lowering of a fold (the one saying lowering 'introduces a new cell kind fold, extending the cell kind enum to input | derived | recurrent | fold') in steady-state terms: the fold-kind cell is one of the enum's members `input | derived | recurrent | fold`, and the count of six graph primitives is unchanged. Drop the 'introduces/extending' changelog framing. Then conform the cited SPEC section so its wording is steady-state too. Endpoints already agree (both land on the 4-member enum), so the edit is wording-only.
**Verify:** `grep -n '035-9' DECISION_LOG.md` shows no `introduces`/`extending` verbs; the entry lists the same four-member enum as the sibling cell-kind entry. `grep -c 'introduces a new cell kind' DECISION_LOG.md` on that line returns 0.

#### M-21 · F217 (MED) — the IR entry defining effect `parameter_bindings` calls them 2-tuples `(parameter_name, source_cell_id | value_literal)`, but the sibling entry and the SPEC section it cites define a 3-tuple carrying a load-bearing provenance marker; an implementer following the atomic entry emits the wrong field shape.
**Edit:** Amend the LOG entry that says an effect entry's `parameter_bindings` are `(parameter_name, source_cell_id | value_literal)` pairs so it includes the provenance-marker slot, matching the sibling entry (the effect-position `|>` entry, whose second sentence already states each slot carries a provenance marker: bare cell / static-wrapped constant / reactive-bridged, a tag on the existing slot, not a third case) and the cited IR SPEC section (which spells the triple `(parameter_name, source_cell_id | value_literal, provenance_marker)` with markers `bound`/`constant_wrap`/`bridge`). The SPEC and sibling LOG entry are the correct side and are already conformed; only this outlier entry changes. No SPEC edit needed.
**Verify:** `grep -n '033-121' DECISION_LOG.md` shows the field now names the provenance marker; the tuple shape matches the SPEC IR section's triple. Cross-check: the marker vocabulary (bound/constant_wrap/bridge or equivalent) appears in the amended entry.

#### M-22 · F222 (MED) — the entry enumerating the reconciler's lifecycle hooks lists only creation/update/teardown, but other entries require the reconciler to receive suspend and resume, and two further entries confirm the count is five.
**Edit:** Extend the LOG entry that enumerates reconciler lifecycle hooks (creation, update, teardown) to include `suspend` (on gate-off / effective-activation false) and `resume` (on gate-on / effective-activation true), making it five hooks — matching the entry stating the runtime guarantees suspend on gate-close and resume on gate-open, the entry stating 'five reconciler hooks must be invokable', and the entry enumerating create/teardown/update/suspend/resume. Then conform the cited SPEC section on reconciler lifecycle hooks so its bulleted hook list includes suspend and resume.
**Verify:** `grep -n '031-123' DECISION_LOG.md` shows five hooks including `suspend` and `resume`; the SPEC lifecycle-hooks bullet list enumerates all five. Cross-check the count against the 'five reconciler hooks' entry.

#### M-23 · F224 (MED) — three pre-amendment framing entries still say an effect body consists only of `desired:`/`observed:` blocks, but the composable-effects amendment added top-of-body child-effect placements as a legal third body item, and the SPEC was already conformed to allow them.

Background: the composable-effects amendment made it legal to place child effects at the top of an effect body, before the `desired:` and `observed:` blocks (a graph placement, not a cell expression). The SPEC section on effect-body shape was updated to match. But three older framing entries were left stale.

The stale entries are: the one saying an effect body 'consists only of the desired: and observed: blocks'; the one saying an effect declaration 'consists of two record-shaped blocks'; and the one saying reactive declarations other than the five role-keyword cell forms 'cannot appear inside an effect's body' — a child-effect placement is exactly such a non-role-keyword reactive instantiation, so as written this entry forbids what the amendment permits.

**Edit:** Conform all three stale LOG entries to admit top-of-body child-effect placements as a body item alongside the `desired:`/`observed:` blocks, matching the amendment entry (which permits child effects as top-of-body items before those blocks) and the entry marking such placements private to the effect. Concretely: the 'consists only of' and 'two record-shaped blocks' entries gain the optional top-of-body child-effect placements as a preceding item; the 'cannot appear inside an effect's body' entry is narrowed so it no longer forbids child-effect placements (it still forbids other non-role-keyword reactive declarations such as `attr` or top-level `signal`). The cited SPEC section is already conformed — do not change it; if a re-read shows it still says 'only', conform it too.

**Verify:** `grep -n '031-138\|031-7\.\|031-15\.' DECISION_LOG.md` — none of the three entries still asserts the body is exclusively the two blocks in a way that excludes child-effect placements; each admits (or no longer forbids) top-of-body child effects. Cross-check against the amendment entry permitting the placement and the SPEC effect-body section.

#### M-24 · F225 (LOW) — the gating-placement entry writes 'the effect's desired: block (or an expose: block)', which reads as if an effect owns an `expose:` block, but `expose:` is node-only and never mixes with effects.
**Edit:** In the LOG entry on gating driven by an observed cell (the one saying gating uses `when`/`given` in 'the effect's desired: block (or an expose: block)'), clarify that the `expose:` block belongs to the enclosing node, not the effect — e.g. reword the parenthetical to 'or the enclosing node's `expose:` block'. This is fixed by the entries establishing that effects are never exposition entries and that `expose:` declares topology while `effects:` declares side effects, the two never mixing. Then conform the cited SPEC section so its parallel prose attributes the `expose:` block to the node.
**Verify:** `grep -n '031-52' DECISION_LOG.md` shows the `expose:` block is attributed to the enclosing node, not the effect; the SPEC gating section matches. No remaining phrasing implies an effect owns an `expose:` block.

#### M-25 · F254 (LOW) — the entry stating gates freeze rather than change membership and 'only dynamic/repeat change membership' is written as a language-wide universal, but in the collect/yielded world a gated-off `yield` is absent, so a gate does change `yielded` membership.
**Edit:** Narrow the LOG entry that contrasts view membership with dynamic/repeat (the one saying views count gated-but-frozen children and only dynamic/repeat change membership) to the view/exposition domain — its own cited section is the views/exposition section — so it no longer reads as a universal claim about all constructs. Keep the substance (gated children stay counted in views) but scope the 'only dynamic/repeat change membership' contrast to views, leaving the yielded/fold entries (where a gated-off yield is absent) unaffected. Then conform the cited SPEC section so its wording is scoped to views, not universal.
**Verify:** `grep -n '017-105' DECISION_LOG.md` shows the membership contrast scoped to views/exposition, not stated as language-wide; the entry no longer conflicts with the yielded-membership entry (gated yield absent) or the fold-membership entry. SPEC section prose matches the narrowed scope.

#### M-26 · owner-directed (2026-07-08) — Auto-fulfillment must range over direct AND inherited methods; today's entries say "every method it declares," which excludes inherited ones.

Owner ruling: a trait is automatically fulfilled — no `fulfill` block needed — when ALL of its methods, declared directly or inherited from its required (super)traits, have default bodies. The auto-satisfaction entry (005-109) currently conditions on "every method it declares," and the method-origin rule (005-30) pins inherited methods' origin to the parent trait — so the current text silently ignores inherited undefaulted methods. The complement rule (005-71, `satisfies` mandatory for a trait with any abstract method) must partition against the same boundary, so its "any abstract method" is scoped the same way (entailed by the ruling; disclosed).

**Edit:** LOG-first. Amend 005-109 (DECISION_LOG.md:450): "every method it declares — including every effect-kind method — has a default body" becomes "every method, declared directly or inherited from its required traits — including every effect-kind method — has a default body". Amend 005-71 (DECISION_LOG.md:412): "for a trait with any abstract method (a method lacking a default body)" becomes "for a trait with any abstract method, direct or inherited (a method lacking a default body)". Do NOT touch 005-67 — its "every method it declares" wording awaits a separate owner ruling. Then conform SPEC: the auto-satisfaction prose in §3.3.5 (grep 'automatically satisfied') and the mandatory-satisfies prose in §3.2 take the same direct-or-inherited scoping.
**Verify:** `grep -n '^005-109\.' DECISION_LOG.md` and `grep -n '^005-71\.' DECISION_LOG.md` both contain "inherited"; `grep -n '^005-67\.' DECISION_LOG.md` is byte-unchanged in `git diff`; SPEC §3.3.5 and §3.2 prose carry the scoping; both entries' (§...) refs intact.

#### M-27 · owner-directed (2026-07-08, Ruling 1) — Extend the fulfill-without-satisfies waiver (005-67) to count inherited methods.

Owner ruling: the waiver's condition ranges over direct AND inherited methods, closing the contradiction M-26 opened between 005-67 ("every method it declares") and the re-scoped 005-71 (satisfies mandatory for any abstract method, direct or inherited).

**Edit:** LOG-first. Amend 005-67: "for a trait where every method it declares, including every effect-kind method, has a default body" becomes "for a trait where every method, declared directly or inherited from its required traits — including every effect-kind method — has a default body". Keep the "declares no required cells" clause and the (§3.2) ref untouched. Then conform the SPEC §3.2 waiver prose to the same scoping.
**Verify:** `grep -n '^005-67\.' DECISION_LOG.md` contains "inherited"; the waiver boundary now partitions exactly against 005-71 (no trait class is both waived and satisfies-mandatory); SPEC §3.2 waiver prose matches.

#### M-28 · owner-directed (2026-07-08, Ruling 2) — Const can never reference functions, period; pin the positive rule: a const is a static, compile-time serializable and inline-able value.

Owner ruling: (a) no function-like value in `const` — function references AND closures, capturing or not; the "captured runtime state" qualifier no longer carves an opening for non-capturing closures. (b) The positive characterization: a `const` holds only a static (never a cell), compile-time serializable, inline-able value — records, arrays, tuples, integers, strings, booleans, and peers.

**Edit:** LOG-first. (a) Amend 004-82 so the closure clause reads "types containing function references or closures (capturing or not)" — the runtime-state qualifier goes. (b) Positive rule: grep the §2.4.1.3 eligibility cluster (004-78..004-84 region) and 013-27 for an existing positive eligibility statement. If one exists, amend it to carry the owner's characterization verbatim in substance ("a `const` holds a static, compile-time serializable and inline-able value"). If none exists, insert a NEW entry adjacent to 004-82 in the §2.4.1.3 run stating it — then renumber section 004 densely per Invariant 1 and update the section header's "— N Rules" count. Derive the eligible enumeration from what the corpus already grants (integers, floats, `bool`, `char`, `string` per 013-27, `duration`/`instant` if corpus-eligible, records/tuples/arrays/enums composed of eligible types) — do NOT drop a corpus-granted type silently; flag anything arguably not serializable/inline-able. Then conform SPEC §2.4.1.3 (the eligibility prose and the ineligible-list bullet, currently ~line 728).
**Verify:** `grep -n '^004-82\.' DECISION_LOG.md` shows the unqualified closure ban; grep the §2.4.1.3 cluster shows the positive rule; if an entry was inserted, section 004's header count equals its actual entry count and numbering is dense; SPEC §2.4.1.3 conforms; `grep -in 'captured runtime state' DECISION_LOG.md` returns no const-eligibility carve-out reading.

#### M-29 · owner-directed (2026-07-08, Ruling 3) — Carve the element-tally rule out of §9.3.7 ("Slices") into a new §9.3.8, and repoint its citers.

Owner ruling: the tally prose (bare `.count` = uniform element tally; `x_count` = specialized tally, exempt) lives inside §9.3.7 whose title promises slices. Give it its own home.

**Edit:** SPEC-first here is structural, LOG conforms after: (a) in SPEC, move the element-tally prose out of §9.3.7 into a new heading `#### 9.3.8 The element tally` placed immediately after §9.3.7's remaining (slices-only) content — purely additive, no existing § renumbers. (b) Grep the LOG for every entry citing §9.3.7; classify each: slices-content citers keep §9.3.7; tally-rule citers (the unified `.count` entry, the `event_count` specialized-tally entry, and any peers) repoint to §9.3.8. (c) Grep SPEC for internal references attributing the tally rule to §9.3.7 (e.g. the stream chapter's "element-tally rule, §9.3.7") and repoint to §9.3.8.
**Verify:** `grep -n '^#### 9.3.8' SPEC.md` exists with the tally prose under it; §9.3.7 contains only slices content; every LOG citer of §9.3.7/§9.3.8 lands on matching content (spot-read each); no dangling refs (`grep -c '§9.3.8' DECISION_LOG.md` ≥ 2).

#### M-30 · owner-directed (2026-07-08, Ruling 4) — Repoint the umbrella-trait citation from the records section to where pure-requirement traits are defined.

Owner ruling: repoint both sites. 009-9 cites §6.1.1 ("Declaration", records — wrong); pure-requirement traits are defined in the auto-satisfaction section (§3.3.5, which reserves the term). The batch-added §3.2 exception text copied the bad parenthetical.

**Edit:** LOG-first. Amend 009-9's trailing ref from (§6.1.1) to the section that defines pure-requirement umbrella traits — confirm by content that this is §3.3.5 (the auto-satisfaction text around SPEC line ~1969) before writing; if the definition's actual heading differs, use that one and say so. Then fix the SPEC §3.2 exception text's "(§6.1.1)" parenthetical to the same target.
**Verify:** `grep -n '^009-9\.' DECISION_LOG.md` cites the confirmed section; `grep -c '(§6.1.1)' SPEC.md` shows the §3.2 exception text no longer carries it; following both refs lands on pure-requirement trait content.
