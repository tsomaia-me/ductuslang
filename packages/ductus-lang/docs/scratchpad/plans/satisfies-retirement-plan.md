# Satisfies-Retirement Amendment Plan — conformance becomes the `fulfill` block

*2026-07-16 · Executes the owner rulings of 2026-07-16 recorded in the audit-plan-2 rulings
ledger: SATISFIES-RETIREMENT (d) full retirement + three refinements (open-by-default with
`sealed type`; bodiless fulfill; keyword `fulfill` kept). Status: rulings LOCKED; plan awaiting
owner read and approval. Survey pin: working tree at HEAD 001f8ec + the uncommitted traits
batch (D-12/D-13); all line numbers WILL drift — locate by content/entry-id, per-file greps.*

---

## 1. Background — why `satisfies` retires

The corpus required conformance in two pieces: `satisfies Trait` (the promise, written in the
type's body) and `fulfill Trait for Type` (the implementation). The promise turned out to be
the wrong load-bearing wall three times over:

1. **It blocks open extension.** A foreign type's body cannot be edited, so any trait with an
   abstract method could never be implemented for a foreign type — the orphan rule's trait-arm
   was dead letter. The old fulfill-without-satisfies waiver was a too-narrow patch over this
   (all-defaults traits only), and it was backwards from the owner's intent besides.
2. **It binds the wrong party.** Any repair that enables extension exempts foreign modules
   from the promise — leaving a rule that binds only the module that needs it least.
3. **The visibility it bought is bought better elsewhere.** `@derive` (written on its own
   line above the type declaration) keeps the common conformances visible at the type;
   documentation tooling lists the rest. Rust and Haskell made the same trade and it held.

**The ruling: conformance IS the `fulfill` block.** One mechanism, one door, local and foreign
alike, gated by the orphan rule and sealing. Only the `satisfies` CLAUSE retires; the English
verb — "a type satisfies a bound", "auto-satisfaction" — survives untouched everywhere.

## 2. The rulings (owner, 2026-07-16 — LOCKED)

**R1 — Full retirement.** The `satisfies` clause is removed from the language: from type,
record, enum, newtype, node, and connection bodies, from the keyword inventory, and from every
rule that references the clause. Conformance to a trait is declared by a `fulfill` block.

**R2 — Open by default; `sealed type` closes.** Types are open to foreign conformance (a
module that owns a trait may fulfill it for a foreign type, per the orphan trait-arm). The
closing marker reuses the sealed keyword, mirror-image of `sealed trait`:

```
sealed trait ColorSpace     // only my module decides who WEARS me
sealed type Song:           // only my module decides what I WEAR
  ...
```

A foreign fulfill for a sealed type is a compile error, diagnostic class
`sealed_type_fulfillment_outside_module` (naming symmetric with the trait-side class).
Composition: sealing a trait restricts by the trait's module; sealing a type restricts by the
type's module; a fulfill must pass both checks.

**R3 — Bodiless fulfill.** `fulfill Trait for Type` with no colon and no body is legal, with
an optional `where` clause. It is: the marker-trait claim form, the conditional-marker carrier,
the nothing-to-override form for defaulted traits, and an optional explicit pin for
auto-satisfied traits (never required — auto-satisfaction needs no fulfill at all):

```
fulfill Copy for Point                       // marker claim
fulfill Copy for Pair[T] where T: Copy       // conditional marker (replaces satisfies…where)
fulfill Renderable for Song                  // defaulted trait, nothing overridden
```

**R4 — The keyword stays `fulfill`.** Its anchor is the trait-as-contract metaphor (abstract
methods, observed contracts, associated types are obligations; the block fulfills them), which
survives retirement. `impl` was explicitly rejected.

**R5 — New coherence rule (required once the promise is gone).** At most one `fulfill` exists
per (trait, type) pair program-wide — the trait's module and the type's module are the only
legal sites (orphan rule), and if both write one, it is a compile error naming both sites.
Conditional fulfills of the same pair with disjoint `where` bounds are a design question the
corpus already governs under conditional-impl rules; the editors conform, not invent.

**R6 — Collision rule re-anchors.** The disjoint-method-names check moves from "the type's
`satisfies` set" to the type's conformance set — every trait the type fulfills (explicitly or
auto-satisfied), the same effective-set the compiler already computes.

## 3. Execution defaults (disclosed; veto any)

- **`satisfies` is freed as an ordinary identifier, silently** — the `main` precedent: no
  reservation, no migration note in normative text.
- **The `satisfies-flag` codegen artifact keeps its name.** SPEC names an internal
  compiler-emitted marker "satisfies-flag" at two sites; it records that a type *satisfies* a
  trait — the surviving VERB sense, not the retired clause. Keep, with a one-line note at
  first use that the name is verb-derived. (Veto → rename to conformance-flag corpus-wide.)
- **@derive rewording**: "implicitly declares `satisfies`" becomes "establishes the
  conformance directly" (derive output = language-provided fulfills).
- **Auto-satisfaction re-expression**: rules phrased as "no `satisfies` clause needed" are
  vacuous once no clause exists; they re-express as "conformance holds with no `fulfill`
  block" (the concept and the D-12 precise observed-contract rule survive unchanged).

## 4. Phased execution — LOG first

Discipline as always: locate by content; verbatim old_strings; entries atomic (no
entry-number citations, in entries or briefs); dense numbering, tail-append for new entries;
class mandates are floors; the CLAUSE/VERB distinction rules every site — backticked
`` `satisfies` `` is the clause, unbackticked prose "satisfies/satisfy/satisfied" is the verb
and survives. Per-file greps only.

### Phase A — LOG section 005 (the core; 20 clause-entries + 3 mixed + new rules)

- **Replace in place (the 8 PURE entries)**: 005-63 (conformance = fulfill block, local or
  foreign, orphan + sealing gate it); 005-65 (auto-satisfaction summary: zero-obligation traits
  need no fulfill; marker traits claim via bodiless fulfill); 005-66 (RETIRED content →
  single-fulfill coherence rule R5); 005-67 (waiver text → bodiless-fulfill form R3);
  005-71 (mandatory-satisfies → what REQUIRES a non-empty fulfill: any abstract method,
  undefaulted associated type, or uncovered observed contract); 005-72 (conditional marker →
  bodiless fulfill + where); 005-107 (conditional-impl pairing → unconditional conformance is
  the fulfill itself; condition carried on the fulfill's where); 005-110 (optional explicit
  pin: a bodiless fulfill of an auto-satisfied trait is legal, useful as evolution guard).
- **Reword (12)**: 005-6, 005-44, 005-73 (R6 re-anchor), 005-75, 005-78, 005-84, 005-109
  (drop the waiver reference; keep the precise observed-contract rule), 005-112, 005-176,
  005-180, 005-189, 005-239 (sealing one door: the bare-satisfies clause of the both-doors
  rule is deleted; fulfill claims — bodiless included — are the only door).
- **Mixed (3, C-list)**: 005-26 ("where `satisfies` is declared" → at the fulfill site;
  "satisfies set" → conformance set; verb rule intact); 005-59, 005-60 ("the compiler rejects
  the `satisfies`" → rejects the fulfill).
- **NEW entries at the 005 tail**: `sealed type` (R2: open-by-default, the marker, the
  diagnostic, the mirror relation restated locally, both-checks composition); the bodiless
  fulfill form (R3, grammar-level statement + where permitted); single-fulfill coherence (R5)
  if 005-66's replacement doesn't carry it whole.
- Header count updated; renumber only if any entry is genuinely deleted (prefer replacement).

### Phase B — LOG other sections

- 001-31, 002-4 (keyword list drops `satisfies`; freed silently per default).
- 009-8, 009-9, 009-66, 009-105, 009-118 (record/enum/newtype satisfies clauses retired —
  bodies no longer carry conformance items; conformance via fulfill), 009-117 (@derive
  rewording).
- 010-9 (one door), 010-13 (moot phrase dropped).
- 013-77, and **013-78 with care** — the conditional-Copy carrier inverts: today "the
  `satisfies` clause alone carries the condition, no fulfill is written"; post-amendment the
  bodiless `fulfill Copy for X where …` is the sole carrier. Semantic rework, not token swap.
- 017-9, 017-226 (node-body member lists drop the clause), 017-12 (node conformance = fulfill:
  `fulfill Drivable for MyNode`; required cells stay declared in the node body, the fulfill
  claims and the compiler checks the body supplies them).
- 019-75 (Circularity opt-in → bodiless fulfill).

### Phase C — SPEC conformance

- **§3.2 rewritten wholesale** (retitled "Conformance Declarations (`fulfill`)"): conformance
  = fulfill; bodiless form; single-fulfill coherence; foreign conformance + sealed-type gate;
  the waiver subsection DELETED. §3.2.1 collision text re-anchored (R6). §3.3.4 conditional
  impls (condition on the fulfill's where; marker case = bodiless). §3.3.5 auto-satisfaction
  re-expressed per the default (D-12 precise rule intact). §3.7.4 markers (bodiless claim).
  **§3.7.6 sealed**: one door; ADD the `sealed type` mirror passage with the diagnostic and a
  worked example; the "second door" example line replaced.
- §1 keyword list drops `satisfies`; overview line 104-region reworded.
- §6.1.1 / §6.2.2 / §6.3.1 (record/enum/newtype satisfies-clause passages → conformance via
  fulfill; body-grammar text drops the clause item). §6.3.3 @derive rewording. §7.2 moot
  phrase. §11.4 Copy (the conditional-carrier inversion, mirror of 013-78).
- **§13.3.2 retitled** (node "satisfies clause" section → node conformance via fulfill);
  §13.3.7 ordering lists drop the clause.
- **Worked examples corpus-wide** (the survey's code-block list, ~14 sites): `satisfies X`
  lines in node/connection/record skeletons become fulfill blocks after the declaration, or
  are dropped where the example doesn't need conformance. Editors keep examples compiling
  under the new law, minimal diffs.
- `satisfies-flag` sites (2802/2841 region): the keep-note per the default.

### Phase D — GRAMMAR.md

- Both `SatisfiesClause` productions DELETED (§7.8 record-form, §8.2 node-form — their
  trailing-comma divergence dies with them); the five referencing body-item productions
  (RecordBodyItem, NewtypeBodyItem, EnumBodyItem, NodeBodyMember, ConnectionBodyMember) drop
  the alternative; §9.7 Circularity comment repointed to the bodiless fulfill.
- **FulfillItem/FulfillBody gain the bodiless alternative in BOTH copies** (GRAMMAR §7.12 and
  SPEC §3.3 are mirrored verbatim — keep lockstep):
  `FulfillBody ::= ( NEWLINE INDENT FulfillBodyItem+ DEDENT ) | NEWLINE` with `WhereClause?`
  reachable in the bodiless form.
- Keyword inventory drops `'satisfies'`. TypeDecl/type-declaration production gains the
  optional `'sealed'` modifier (mirror of TraitDecl's) + diagnostic comment; TraitDecl's
  sealed comment loses the bare-satisfies door.
- IR_GRAMMAR.md: verified zero occurrences — no-op pass.

### Phase E — Gates, review, capture

1. gate-clause-residue (the razor): backticked `` `satisfies` `` = 0 in DECISION_LOG, SPEC,
   GRAMMAR; `'satisfies'` (quoted keyword) = 0 in GRAMMAR; `SatisfiesClause` = 0 everywhere.
   Every surviving unbackticked "satisfies/satisfy/satisfied" hit is verb prose — the reviewer
   spot-classifies a sample.
2. gate-sealed-type: the mirror passage + diagnostic present in LOG and SPEC; both-doors
   phrasing = 0 ("bare `satisfies`" = 0); trait-side sealing text intact minus its second door.
3. gate-bodiless: the FulfillBody alternative present in BOTH copies, byte-mirrored; the
   conditional-marker examples (`fulfill Copy for Pair[T] where T: Copy`) present at the
   Copy sites.
4. gate-coherence: the single-fulfill rule present (LOG + SPEC); collision rule re-anchored
   (grep "satisfies set" = 0).
5. gate-counts/invariant-2/xref: as always, on the raw diff.
6. gate-regressions: stream/cell bracket families still 0; collect-as 0; 'policy keyword' 0;
   the D-12 auto-satisfaction precise rule survives (grep "does the default body cover the
   contract" — 1 hit in LOG, wrapped hit in SPEC).
7. THREE scoped blind reviewers (LOG / SPEC / GRAMMAR diffs — the batch is too large for one),
   adjudication, fix passes, gate re-runs; diff capture to the scratchpad; final report.
   No commits.

## 5. For execution — survey site lists (2026-07-16; re-grep before editing)

> **LOG (36 A-sites + 3 C)**: 001-31; 002-4; 005-6, 44, 63, 65, 66, 67, 71, 72, 73, 75, 78,
> 84, 107, 109, 110, 112, 176, 180, 189, 239 (+C: 005-26, 59, 60); 009-8, 9, 66, 105, 117,
> 118; 010-9, 13; 013-77, 78; 017-9, 12, 226; 019-75. Headers: 005=241, 009=128, 010=45,
> 013=250, 017=326, 019=79.
> **SPEC (86 A-sites + 6 C)**: the survey's region lists — §1 (104, 133); §3.1 (1200–1579);
> §3.2 block (1611–1661); §3.2.1 (1668–1783, 2136); §3.3.4 (1974–1981); §3.3.5 (1996–2057);
> §3.7.4 (2656–2717); §3.7.6 (2740–2756); §6.1.1 (4939–4950); §6.2.2 (5325–5337); §6.3.1
> (5607–5643); §6.3.3 (5750–5797); §7.2 (5960); §11.4 (8888–8923); worked examples (12494,
> 12499, 13166, 13488, 13516, 14375, 14818, 16284, 16533, 17279, 17285); §13.3.2 (13524–13537);
> §13.3.7 (14836–14838); C: 1376, 1377, 1599, 1603, 2802, 2841 (satisfies-flag).
> **GRAMMAR (32 A-sites)**: keyword list 273; SatisfiesClause 3496 + 4188 and referencing
> items 3492, 3544–3545, 3584, 4149, 4812; FulfillItem/Body 3762–3770 (+ SPEC mirror
> 1834–1836); TraitDecl comment 3667; §9.7 comments 5113–5131; §8 prose 4135–4204; remaining
> comment sites per survey. **IR_GRAMMAR: zero.**

## 6. Verification greps (per file, after Phase E)

```bash
grep -c '`satisfies`' DECISION_LOG.md      # expect 0   (same for SPEC.md, GRAMMAR.md)
grep -c "'satisfies'" GRAMMAR.md           # expect 0
grep -c 'SatisfiesClause' GRAMMAR.md       # expect 0   (and SPEC.md)
grep -n 'satisfies set' DECISION_LOG.md    # expect 0
grep -n 'bare .satisfies' DECISION_LOG.md SPEC.md GRAMMAR.md   # per file; expect 0
grep -n 'sealed type' DECISION_LOG.md      # expect the new mirror entries
grep -n 'sealed_type_fulfillment_outside_module' DECISION_LOG.md SPEC.md  # per file; defined
grep -n 'fulfill Copy for' DECISION_LOG.md SPEC.md   # the conditional-marker carriers
grep -c 'does the default body cover the contract' DECISION_LOG.md   # expect 1 (D-12 intact)
```

## 7. Out of scope, deliberately

- The keyword-class assignment of `sealed`'s type-side use and the freed `satisfies`
  identifier (the keyword-classes phase, D-05, owns taxonomy).
- The node-trait required-cells checking machinery (unchanged: cells declared in node bodies;
  the fulfill claims; the compiler checks coverage — only the claim's spelling moved).
- Everything queued behind this amendment: structure batch (D-09/14/16/24-verify), streams
  (D-19 + observe-per-event), policies (D-05 residuals ride it), main-removal (parked on its
  confirms), Part 3 re-anchor.
