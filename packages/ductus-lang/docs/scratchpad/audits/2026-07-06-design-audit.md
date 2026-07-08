# Ductus Design Audit — 2026-07-06

**Verdict: RED** — The amendment landed asymmetrically: the SPEC's new cell-kind model and the new yielded/fold constructs are contradicted or unsupported by the decision-of-record, producing 23 implementer-blocking defects — the docs cannot be relied on as-is until the seam is reconciled.

- Corpus pin: `1b0c6b8f2a4e2723514dbe2722f2e77281854386` (DECISION_LOG.md 658926 B, SPEC.md 1105530 B)
- Raw findings: 262 → unique defect clusters: 242 → CONFIRMED 223 · PLAUSIBLE 14 · UNVERIFIED 1 · REFUTED 4 · observations 10

## Executive summary

223 confirmed findings (23 HIGH), concentrated in the amendment seam. Verdict is RED because two problems are systemic, not local.

First: the cell model is stated two incompatible ways. SPEC 13.2.8 says there is no Cell[T] type — cells are lowercase kinds (cell T, signal T, derived T). But the LOG, the document you declared the lossless decision-of-record, still types dozens of entries across sections 013, 016, 017, 025, and 029 as Cell[T], Signal[T], Portal[Cell[T]], Cell[DynamicView[...]] (F194, F195, F146, F156, F221, F183). Section 016 contradicts itself internally — adjacent entries use both models (F147, F062) — and gives three different counts of its own declaration kinds (F153). The Cell-as-KIND amendment landed in SPEC and was never conformed in the LOG. An implementer reading the LOG builds a type system the SPEC explicitly denies.

Second: yielded and fold are unimplementable as written. A standalone yielded group has no lowering to the six IR primitives and no IR cell kind (F093, F213); repeat's source rule does not admit a yielded group even though 034-11 requires repeat over one (F094, F097, F164); fold cells have no route into the per-commit dirty set or DAG (F123, F247); the fold's value at a gate-close commit is under-determined (F114); and the machinery the new sections import — "on/off bits", "walk order" — is defined nowhere (F211, F212).

Third tier, still HIGH: effect-kind method dispatch collects effect methods as ordinary call candidates while other rules forbid ordinary calls on them (F033, F067, F068), and the effect observed: block both admits and rejects computed outputs (F088). In the structure layer, `keyed by <index>` is rejected by its own StringifiableKey bound (F163); six legal Map key types have no legal repeat keying at all (F229); LOG and SPEC swap Handle and WeakHandle for freeze/re-point behavior (F228, F177, F172); dynamic view satisfies Iterable yet `for` over it is forbidden (F255); structural self-recursion is unbounded with no declared error (F083); and the live-graph closure differs between LOG and SPEC (F227). StreamPolicy is "four members" in one entry and exactly two everywhere else (F199); a gate stream's buffer hold leaks permanently when a keyed scope drops (F111); the gate-open snap has no legal ordering inside the commit (F064). The keyword floor is broken: given, cell, observe, own, requires, enum, wraps, yielded and others are used as keywords but absent from every closed keyword set (F251, F136–F141), so a lexer built from the docs mis-lexes the corpus's own examples.

Behind these sit contained MED/LOW clusters: hot-reload identity contradictions (F218, F205, F220), missing reconciler lifecycle windows (F089, F090, F188), core numerics contradictions (F026, F027, F028), stale examples using forbidden syntax (F121, F122, F126), and a long tail of broken LOG-to-SPEC pointers, duplicated canon, and undefined load-bearing terms (containment closure, interpretation context, wake gate).

14 findings are PLAUSIBLE: verifiers sustained three (F046, F041, F101) and effectively dissolved the rest, but per charter they are retained for your judgment. Nothing in this report decides anything; every direction-of-change is a question for you.

## How to read this report

The findings body below this summary is assembled mechanically from full records; each finding carries its LOG anchors, SPEC section refs, severity per the chartered rubric (HIGH = implementer-blocking, MED = behavior-changing ambiguity or substantive divergence, LOW = drift/smell), and verbatim evidence. Read themes in order — the first three contain nearly all HIGHs and describe the amendment seam; the last three are hygiene tails you can batch. PLAUSIBLE findings include the verifier's crux verbatim: several are argued refuted by the verifier itself — read the crux before acting on the finding. A few near-duplicate confirmations survived dedup (F057/F086/F162; F190/F260 with F092; F094/F097/F164; F171/F211) and are grouped inside one theme each. Every finding is direction-of-change only; no fix is decided.

## Findings by theme

### Cell model split: LOG bracket types vs SPEC kinds

The Cell-as-KIND amendment landed in SPEC 13.2.8 but the LOG was not conformed: sections 013/016/017/025/029 still speak Cell[T]/Signal[T]/Portal[Cell[T]] bracket types the SPEC flatly denies, 016 self-contradicts internally and miscounts its own declaration kinds, and cell-storage rules (closures, borrow slots, stream reads) conflict at the edges. This is the systemic divergence driving the RED verdict.

#### F153 — Section 016 gives three mutually inconsistent counts of the declaration kinds: seven (016-1), six (016-162), and an enumerated five (016-156).

- **Severity/Category/Verdict:** HIGH / contradiction / CONFIRMED
- **Anchors:** LOG 016-1, 016-162, 016-156 · SPEC § 13.2, 13.2.7
- **Why it is a defect:** 016-1 fixes the total number of declaration kinds at exactly seven (signal, attr, recurrent, derived, const, stream, yielded). 016-162 asserts the no-source-write rule covers 'all six declaration kinds', and the immediately preceding 016-156 enumerates only five kinds it applies to (signal, attr, recurrent, derived, const). An implementer building the assignment-rejection check cannot reconcile 'six' with a fixed universe of seven and an explicit list of five; the set the rule ranges over is undetermined (does it include stream? yielded? both? neither?). These are jointly unsatisfiable as written.
- **Direction of change:** Reconcile the cardinal used in 016-162 with the fixed taxonomy of 016-1 and the enumeration in 016-156; make the count and the member list agree across all three entries. Surface to user which count is authoritative.
- **Evidence check:** pass — §016 gives three inconsistent counts of declaration kinds: 016-1 fixes seven, 016-162 says 'six', 016-156 enumerates five; the set the no-source-write rule ranges over is undetermined (does it include stream/yielded?), leaving the assignment-rejection check unsatisfiable as written.
- **Charity check:** sustain — Three counts of the assignment-rejection universe cannot be reconciled. 016-1 (L1854): 'exactly seven reactive declaration kinds' (signal, attr, recurrent, derived, const, stream, yielded). 016-156 (L2009), same §13.2.7 as its antecedent, enumerates FIVE by name: 'signal, attr, recurrent, derived, or const'. 016-162 (L2015): the no-source-write rule 'applies uniformly to all six declaration kinds'. Five vs six directly conflict on the SAME rule (both §13.2.7, both no-source-write): an implementer cannot make the rejection check cover exactly-five AND exactly-six. No charitable reading closes it — if the sixth is stream, 016-156's list is an undercount; neither reconciles with 016-1's seven. This also spills into LOG-SPEC divergence (SPEC:11349, 11550, 12373 all say SIX kinds and omit 'yielded'; only LOG 016-1 says seven), but the sustained defect is the LOG-internal count contradiction. HIGH: jointly unsatisfiable as written.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1854-1854`
    > 016-1. There are exactly seven reactive declaration kinds — `signal`, `attr`, `recurrent`, `derived`, `const`, `stream`, and `yielded` (introduced by `collect:`/`yield`) — distinguished by who controls the value and how it changes
  - `packages/ductus-lang/docs/DECISION_LOG.md:2009-2009`
    > 016-156. Ductus source has no syntactic form for assigning to a signal, attr, recurrent, derived, or const after declaration; source expressions only read them. (§13.2.7)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2015-2015`
    > 016-162. The no-source-level-write rule applies uniformly to all six declaration kinds. (§13.2.7)

#### F195 — LOG 017-215 defines `Portal[Cell[T]]` resolving to `Option[&Cell[T]]` (a real `&Cell[T]` reference type); SPEC §13.3.6.3 uses `Portal[cell T]` resolving to `Option[&cell T]`, and §13.2.8 says no `Cell[T]` type exists.

- **Severity/Category/Verdict:** HIGH / divergence / CONFIRMED
- **Anchors:** LOG 017-215, 016-179 · SPEC § 13.3.6.3, 13.2.8
- **Why it is a defect:** 016-179 itself is self-undercutting: it states 'a cell is refused as a value' yet writes the sanctioned form as `Portal[Cell[T]]` using the bracket type it just said cells cannot have. 017-215 goes further and posits `&Cell[T]` — a reference to a `Cell[T]` type. SPEC spells both as lowercase `Portal[cell T]` / `Option[&cell T]`. An implementer cannot build both a `&Cell[T]` type-reference (LOG) and a `&cell T` kind-reference (SPEC) for the same construct. Direct LOG-SPEC divergence plus internal incoherence in 016-179.
- **Direction of change:** Pick one spelling for the portal-of-cell carrier and the reference form (`Portal[Cell[T]]`/`&Cell[T]` vs `Portal[cell T]`/`&cell T`), then align 017-215, 016-179 and SPEC §13.3.6.3 to it. User decision required.
- **Evidence check:** pass — 017-215 defines 'Portal[Cell[T]]' resolving to 'Option[&Cell[T]]' — a reference to a Cell[T] type the SPEC (§13.2.8) says does not exist; SPEC §13.3.6.3 spells it 'Portal[cell T]'/'Option[&cell T]'. Plus 016-179 is internally incoherent ('cell refused as value' yet writes bracket 'Portal[Cell[T]]'). Implementer cannot build both a &Cell[T] type-ref and a &cell T kind-ref for the same construct — HIGH.
- **Charity check:** refile_divergence — The finding is filed as a HIGH internal-contradiction (016-179 self-undercutting + 017-215 positing `&Cell[T]`). The passages that speak to `Cell[T]`'s existence -- SPEC.md:12441 'there is no `Cell[T]` type', SPEC.md:12509 `Cell[DynamicView...]` retired, SPEC.md:12512 lowercase `Portal[cell T]` -- do not dissolve the finding; they CONFLICT with the finding's quoted LOG passages (016-179 at DECISION_LOG.md:2032 and 017-215 at DECISION_LOG.md:2356, both using `Cell[T]`/`Portal[Cell[T]]`/`&Cell[T]`). So per the category rule this mutates into a LOG-SPEC divergence, not an internal-only contradiction. It is also not HIGH-unsatisfiable: the two spellings denote the same intended construct (12509 shows `Cell[...]` is a retired spelling of the kind), so each is constructible in isolation -- an implementer following SPEC builds `&cell T`, following LOG builds `&Cell[T]`; the rules are not jointly unsatisfiable, they merely disagree in vocabulary. Refiled as a (MED-severity) divergence. Note 016-180 (DECISION_LOG.md:2033) keeps `Cell[T]` as live LOG generic-abstraction vocabulary, deepening rather than dissolving the divergence. | SPEC.md:12441 > all cell kinds; there is no `Cell[T]` type and no `Cell` trait to fulfill. AND SPEC.md:12509 > | Dynamic view | `dynamic view V` | replaces `Cell[DynamicView[WeakHandle[V]]]` (§13.3.3.4) | AND SPEC.md:12512 > | Storable — non-graph slot | `Portal[T]` | identity-as-data; `Portal[cell T]` is the sanctioned cell-identity carrier | -- ALL CONFLICT WITH -- DECISION_LOG.md:2032 > 016-179. ... a cell is refused as a value, and `Portal[Cell[T]]` is the sanctioned identity-as-data form. AND DECISION_LOG.md:2356 > 017-215. ... `Portal[Cell[T]]` is well-formed — the portal resolves to `Option[&Cell[T]]` ...
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2356-2356`
    > 017-215. `Cell[T]` and `Portal[T]` are orthogonal: `Cell[T]` is a reactive reference whose read declares a dependency edge into the enclosing expression's provenance, while `Portal[T]` is an inert window whose read does not. `Portal[Cell[T]]` is well-formed — the portal resolves to `Option[&Cell[T]]` and the inner cell read remains reactive by the compiler's implicit cell-value auto-deref. (§13.3.6.3)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2032-2032`
    > 016-179. The storability razor: storable values keep bracket types (`Type[C]`, `Handle[T]`, `WeakHandle[T]`, `Portal[T]`), while binding machinery is spelled as lowercase kinds; a cell is refused as a value, and `Portal[Cell[T]]` is the sanctioned identity-as-data form. (§13.2.8)
  - `packages/ductus-lang/docs/SPEC.md:14412-14414`
    > inert window whose read does not. `Portal[cell T]` is well-formed and is
    > the sanctioned identity-as-data carrier for a cell: the portal resolves
    > to `Option[&cell T]`, and the inner cell read remains

#### F194 — Section 016 uses bracketed type `Cell[T]` and lowercase KIND `cell T` for the same subject; SPEC §13.2.8 flatly bans the `Cell[T]` type, so the bracket-form LOG entries diverge from SPEC.

- **Severity/Category/Verdict:** HIGH / contradiction / CONFIRMED
- **Anchors:** LOG 016-62, 016-172, 016-175, 016-178, 016-179, 016-283 · SPEC § 13.2.8
- **Why it is a defect:** 016-178 (and 016-62) declare that a cell is named ONLY by a lowercase kind annotation `cell T`, with no bracket type; but 016-172/016-175/016-283 in the SAME section annotate parameters as `Cell[T]` (bracket type). An implementer reading section 016 cannot decide whether the operator/function parameter annotation is `cell T` or `Cell[T]` — the two forms are mutually exclusive per the section's own KIND-vs-storable razor (016-179). SPEC §13.2.8 resolves it against the bracket form ('there is no `Cell[T]` type'), making every bracketed `Cell[T]` LOG entry a LOG-SPEC divergence (defect per edit protocol).
- **Direction of change:** Decide (user call) whether the annotation is the lowercase KIND `cell T` or a bracket type `Cell[T]`; then make the LOG entries in 016 internally consistent and conform them to whichever SPEC keeps. Do not resolve unilaterally.
- **Evidence check:** pass — Within LOG section 016, 016-178/016-62/016-179 mandate lowercase-only `cell T` (no bracket type) while 016-172/175/283 annotate params as bracket `Cell[T]`; the section's own KIND-vs-storable razor makes these mutually exclusive, and SPEC §13.2.8 bans `Cell[T]` outright.
- **Charity check:** sustain — HIGH intra-section contradiction confirmed. Within §016: 016-172/016-175/016-283 (DECISION_LOG.md:2025,2028,2136) annotate parameters as bracket Cell[T], while 016-62/016-178/016-179 (DECISION_LOG.md:1915,2031,2032) declare a cell is named ONLY by a lowercase kind annotation with no bracket type and 'refused as a value'. The section's own storability razor (016-179) makes the two forms mutually exclusive, so an implementer building §016 cannot decide the parameter surface form. SPEC:12439-12441 resolves it against the bracket ('there is no Cell[T] type and no Cell trait to fulfill'), and SPEC:20545-20549 explicitly labels the bracket 'type-vs-trait' framing as the older incorrect one — confirming the bracket-form LOG entries are the stranded/divergent ones. No corpus text rescues the bracket form. Sustain at HIGH: an implementer-blocking, mutually-unsatisfiable pair inside one section with a confirming LOG-SPEC divergence.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2025-2025`
    > 016-172. An operator value parameter is typed `Cell[T]`, binding to any reactive value cell at instantiation and allocating internal state tied to that cell. (§13.2.8)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2028-2028`
    > 016-175. A function parameter declared `s: Cell[T]` receives the cell reference itself. (§13.2.8)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2031-2031`
    > 016-178. The kind taxonomy has a single home, organized as values / graph entities / cells: storable values are designated directly, or through `Handle` or `Portal`; a cell is named by a lowercase kind annotation — `signal T`, `derived T`, `recurrent[N] T`, `stream ring[N] T`, `stream gate[N] T`, `recurrent[N] stream …`, erased `stream T`, `yielded T`, `cell T`, or `dynamic view X`; static views get no kind form; `const` needs no row; and effects, nodes, and connections annotate by their type name. (§13.2.8)
  - `packages/ductus-lang/docs/SPEC.md:12439-12441`
    > Membership is a KIND relation, not a subtype or trait relation: `signal`, `derived`, `recurrent`, `stream`, and `yielded` are
    > all cell kinds; there is no `Cell[T]` type and no `Cell` trait to fulfill.

#### F047 — LOG 013-153 states any `Copy` value can be stored in a cell 'like any other Copy value', but a closure is `Copy` (013-192) yet 013-199 forbids closures as cell value types — jointly unsatisfiable as stated.

- **Severity/Category/Verdict:** MED / contradiction / CONFIRMED
- **Anchors:** LOG 013-153, 013-192, 013-199 · SPEC § 11.9.1, 11.10.6
- **Why it is a defect:** 013-153 gives an unqualified rule: Copy values can be stored in cells 'like any other Copy value'. 013-192 makes closures Copy. Composed, they license storing a closure in a reactive cell. 013-199 (and 025-55) categorically forbid it. An implementer cannot satisfy both the general storability rule and the closure prohibition without 013-153 carrying an exception it does not state. The atomic-self-containment invariant is also strained: 013-153's blanket 'like any other Copy value' is false for closures.
- **Direction of change:** Qualify 013-153's 'Copy values can be stored in cells' so it does not blanket-cover closure Copy values, or have 013-199 be restated as the recognized exception. Surface the tension; do not pick the carve-out wording unilaterally.
- **Evidence check:** pass — 013-153 gives an unqualified rule that any Copy value can be stored in a cell, 013-192 makes closures Copy, yet 013-199 forbids closures as cell value types — the general storability rule and the closure prohibition cannot both hold without an exception 013-153 does not state.
- **Charity check:** sustain — Confirmed jointly-unsatisfiable LOG triple. 013-153 (DECISION_LOG.md:1539) states unqualified: '`Copy` values ... can be stored in cells, fields, payloads, or tuples like any other `Copy` value.' 013-192 (DECISION_LOG.md:1578): 'A closure value is itself `Copy`.' 013-199 (DECISION_LOG.md:1585): 'A closure cannot be the value type of a `signal`, `attr`, `recurrent`, or `derived` cell.' Composed, 013-153+013-192 license storing a closure in a cell; 013-199 forbids it. No LOG text restricts 013-153's universal 'like any other Copy value' to exclude closures. Notably the SPEC is MORE careful: SPEC.md:9281-9287 scopes the same 'storable in a cell like any other Copy value' claim specifically to Handle/WeakHandle/Portal carriers, NOT all Copy values — so 013-153's blanket wording also DIVERGES from SPEC §11.9.1 (which never generalizes to closures). Either way the LOG-internal contradiction the finding asserts is real and quoted-text-forced. SUSTAIN. The specific-over-general precedence an implementer might apply is not licensed by Invariant-2 self-containment, and 013-153 asserts a false universal.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1539-1539`
    > 013-153. `Handle[T]` is `Copy`; `Copy` values aren't aliases under §11.9 — they can be stored in cells, fields, payloads, or tuples like any other `Copy` value. (§11.9.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1578-1578`
    > 013-192. A closure value is itself `Copy`: read, passed, and duplicated freely, never consumed. (§11.10.6)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1585-1585`
    > 013-199. A closure cannot be the value type of a `signal`, `attr`, `recurrent`, or `derived` cell. (§11.10.6)

#### F008 — 001-25 enumerates borrow-unstorable slots as record field / tuple component / enum payload / indexed slot, omitting the reactive cell that its own cited SPEC §13.3.6.1 explicitly includes.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 001-25 · SPEC § 13.3.6.1, 11.9.1
- **Why it is a defect:** SPEC must conform to LOG, but here the LOG's own closed enumeration is narrower than the cited SPEC section. 001-25 lists 'indexed slot' (a category-B compound site) but not 'cell'; §13.3.6.1 (the section 001-25 cites) lists 'cell' first. §11.9.1 treats cell-storage as a separate category D from compound storage category B. A reader taking 001-25 as the authoritative WHAT would conclude a borrow may be written into a reactive cell, which §13.3.6.1/§11.9.1 forbid. Also a citation-aptness concern: 001-25 cites §13.3.6.1, but the canonical borrow-restriction enumeration lives in §11.9.1; §13.3.6.1 forwards to §11.9.1.
- **Direction of change:** Surface to user: reconcile 001-25's slot enumeration with §13.3.6.1/§11.9.1 — either add the cell/reactive-cell case to 001-25 or confirm 'indexed slot' was intended to subsume it; also confirm the §13.3.6.1 vs §11.9.1 citation target. Do not edit.
- **Evidence check:** pass — 001-25's closed borrow-unstorable list omits 'reactive cell', which its own cited SPEC §13.3.6.1/§11.9.1 explicitly forbids. A reader treating LOG as authoritative WHAT concludes a borrow may be written to a cell. Confirmed on disk.
- **Charity check:** sustain — Charitable hunt found the LOG DOES forbid storing a borrow in a reactive cell — 013-53 (DECISION_LOG.md:1439 'A borrow-equivalent alias cannot be written into a reactive cell'), 013-148 (DECISION_LOG.md:1534), and 019-20 (DECISION_LOG.md:2636 'a borrow may not be stored in a cell'). So this is NOT a soundness hole. BUT the finding is filed as a divergence, and that survives: 001-25 (DECISION_LOG.md:55) is a closed atomic enumeration — 'a borrow cannot be stored in a record field, tuple component, enum payload, or indexed slot (§13.3.6.1)' — and the SPEC section it explicitly cites, §13.3.6.1 (SPEC.md:14135), lists 'cell' first: 'A borrow may not be put in a cell, record field, enum payload, or tuple.' The LOG entry's own list omits the cell that its cited SPEC section includes. That the LOG covers the cell case in OTHER entries (013-53/019-20) does not dissolve the divergence — it proves 001-25's list is defectively narrow relative to both its cited section and the LOG's own other entries. Under LOG Invariant 2 (atomic, self-contained restatement), 001-25 restated an enumeration that is incomplete. Citation is apt (§13.3.6.1 is the right section and forwards to §11.9.1 per SPEC.md:14135-14139), so the defect is the omitted list member, not a wrong pointer. Substantive LOG-SPEC divergence in a closed enumeration = MED.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:55-55`
    > 001-25. Node, connection, and effect instances are first-class citizens; a binding to one is a borrow, and a borrow cannot be stored in a record field, tuple component, enum payload, or indexed slot (§13.3.6.1). (§1.3)
  - `packages/ductus-lang/docs/SPEC.md:14135-14136`
    > - **A node reference cannot be stored.** A borrow may not be put in a cell,
    >   record field, enum payload, or tuple (§11.9.1). The storable, non-owning way
  - `packages/ductus-lang/docs/SPEC.md:9249-9256`
    > - **Stored in a record field, enum variant payload, or tuple component
    >   (category B).** Compound types contain owned values, never aliases
    >   (§11.11). To store a value derived from an alias, declare the
    >   function return as `-> own T` (anchoring per §11.3.6) or
    >   explicitly `.clone()` to produce a real owner.
    > - **Stored in a reactive cell (category D).** `signal.write`,
    >   `stream.emit`, attr reassignment, and recurrent advance require a real
    >   owner for the value flowing into the cell (§11.3.4).

#### F243 — 016-172 says a Cell[T] operator value parameter binds only value cells, but 016-283 and 030-48 say Cell[T]/cell T admits any cell including streams (excluded only at the read site).

- **Severity/Category/Verdict:** MED / contradiction / CONFIRMED
- **Anchors:** LOG 016-172, 016-283, 030-48 · SPEC § 13.2.8, 13.18.5
- **Why it is a defect:** The two vocabularies collide on the same subject: what a `Cell[T]`/`cell T` PARAMETER admits. 016-172 (cell speak) restricts binding to value cells, so an implementer rejects a stream at the SIGNATURE. 016-283 and 030-48 (stream speak) admit any cell incl. stream at the signature and reject a stream only at the READ site. These give different concrete compile outcomes for `operator f(source: Cell[T])` passed a stream: signature-reject vs signature-accept. SPEC 12508 sides with the umbrella (spans all), leaving 016-172 as the outlier. Under LOG Invariant 2 each entry is self-contained; 016-172 as written is jointly unsatisfiable with 016-283/030-48.
- **Direction of change:** Reconcile 016-172 to the umbrella-at-signature / excluded-at-read-site model of 016-283 and 030-48 (or narrow all three to one consistent rule); user decides which side is normative — do not resolve unilaterally.
- **Evidence check:** pass — 016-172 binds a `Cell[T]` operator value param to value cells only (signature rejects a stream); 016-283 and 030-48 admit any cell incl. stream at the signature, excluding streams only at the read site. Different compile outcomes for the same signature.
- **Charity check:** sustain — Jointly-unsatisfiable pair stands. 016-172 (DECISION_LOG.md:2025) 'An operator value parameter is typed Cell[T], binding to any reactive VALUE cell' restricts the bind set to value cells (a stream is rejected at the signature). 016-283 (DECISION_LOG.md:2136) 'a Stream[T] is excluded at the read site rather than by the signature' and 030-48 (DECISION_LOG.md:3554) 'works with any value cell or stream' admit a stream at the signature. Different concrete compile outcome for a stream passed to operator f(source: Cell[T]): signature-reject vs signature-accept-then-read-reject. I hunted for text making 016-172's 'value cell' mean 'any cell'; none exists. SPEC:12415-12418 forces the read-site reading ('binding to any value cell... A stream T has no current value, so it is excluded at the read site, not by the annotation'), siding with 016-283/030-48 and leaving 016-172 the outlier — this confirms rather than dissolves the intra-LOG contradiction.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2025-2025`
    > 016-172. An operator value parameter is typed `Cell[T]`, binding to any reactive value cell at instantiation and allocating internal state tied to that cell. (§13.2.8)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2136-2136`
    > 016-283. Value-reading operator and function parameters are typed `Cell[T]` (the umbrella); a `Stream[T]` is excluded at the read site rather than by the signature. (§13.2.8)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3554-3554`
    > 030-48. `Cell[T]` types signatures accepting any kind of reactive cell: `operator monitor[T, C: Cell[T]](source: C) -> Derived[bool]` works with any value cell or stream. (§13.18.5)
  - `packages/ductus-lang/docs/SPEC.md:12508-12508`
    > | Cell umbrella (KIND) | `cell T` | spans all of the above; not a type, not a trait |

#### F221 — LOG 013-10, 025-13, and 025-59 spell the reactive-cell type/parameter as bracketed `Cell[T]`, but the referenced SPEC (§13.2.8) states "there is no `Cell[T]` type" and §13.12.2 uses the un-bracketed kind `cell T`.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 013-10, 025-13, 025-59 · SPEC § 13.2.8, 13.12.2, 13.12.4
- **Why it is a defect:** SPEC must conform to LOG and divergence between LOG and its referenced SPEC section is a defect per the project edit protocol. Here the LOG carries a spelling the SPEC explicitly says does not exist as a type: 025-13's ref (§13.12.2) and 013-10's neighbourhood both render the construct as the lowercase kind `cell T`, while SPEC §13.2.8 flatly denies any `Cell[T]` type. An implementer reading the LOG would look for a bracketed `Cell[T]` type/trait the SPEC forbids. This is the same subject (cell-typed parameter) described with two incompatible surface forms across the cross-pair — the disjoint-vocabulary hazard the pairing targets.
- **Direction of change:** Reconcile the LOG entries' surface spelling with the SPEC's `cell T` kind notation (or, if the bracketed form is intended, reconcile the SPEC) — surface the choice to the user rather than resolving unilaterally, since it is a terminology decision.
- **Evidence check:** pass — Three LOG entries (013-10, 025-13, 025-59) spell the reactive-cell parameter/type as bracketed `Cell[T]`, but their referenced SPEC sections state no `Cell[T]` type exists and consistently use kind `cell T` — a substantive LOG-SPEC vocabulary divergence for one construct.
- **Charity check:** sustain — Same root divergence as F197, broader scope, independently confirmed. LOG uses bracket `Cell[T]` in 25 entries (grep-confirmed: 013-10, 016-172/175/179/180, 017-215, 025-13, 025-59, 029-2/12/18/20/22/31/41/45/69/122/124, 030-48, 031-19/25, etc.). SPEC.md:12441 flatly denies any `Cell[T]` type; SPEC §13.12.2 at SPEC.md:18542 writes the identical cell-parameter construct as `fn some_fn(s: cell T)` (LOG 025-13/DECISION_LOG.md:3110 writes it `fn some_fn(s: Cell[T])`). 025-59 (DECISION_LOG.md:3156) uses `Cell[f32[64]]` where SPEC §13.12.4 uses bracket-free `T[N]` fixed-array cell types (SPEC.md:18674). An implementer reading the LOG hunts for a bracketed `Cell[T]` type/trait the SPEC forbids. Note the LOG is internally split too (016-179/016-180 deliberately keep `Cell[T]`; 016-177/016-178 use `cell T`) — the bracket-form is not a stray typo but a systematically stale spelling. SUSTAIN.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1396-1396`
    > 013-10. Exception: `Cell[T]` parameters and reactive composite bindings name reactive cells, so multiple live aliases to the same cell may coexist without violating single ownership. (§11.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3110-3110`
    > 025-13. A parameter declared with a cell type (`Cell[T]`) binds to the cell reference itself rather than to its current value: `fn some_fn(s: Cell[T])`. (§13.12.2)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3156-3156`
    > 025-59. A `for` iterating a fixed-extent cell's value compile-time-unrolls to element reads at known offsets — no runtime loop counter, bounds check, or per-iteration dispatch: `for x in buf.value():` over `Cell[f32[64]]`. (§13.12.4)
  - `packages/ductus-lang/docs/SPEC.md:12439-12441`
    > Membership is a KIND relation, not a subtype or
    > trait relation: `signal`, `derived`, `recurrent`, `stream`, and `yielded` are
    > all cell kinds; there is no `Cell[T]` type and no `Cell` trait to fulfill.
  - `packages/ductus-lang/docs/SPEC.md:12423-12427`
    > - **Function parameters** — `fn` may accept a value by type `T` or a cell by
    >   kind `cell T`. The compiler distinguishes call-site semantics by the
    >   function's declared signature: a `fn(x: T)` parameter receives the cell's
    >   current value (with reactive transparency per §13.12.2); a `fn(s: cell T)`
    >   parameter receives the cell reference.

#### F198 — 016-246/016-270/016-283 type observe values and read-site parameters as bracket `Cell[T]`; SPEC §13.2.8 and 016-178 require the lowercase KIND `cell T` — same LOG-vs-SPEC bracket/kind split across the observe and generics sub-sections.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 016-246, 016-270, 016-283, 016-180 · SPEC § 13.2.8, 13.2.11.1
- **Why it is a defect:** The SPEC's own worked example for the identical construct that 016-180 documents writes `source: cell T` (lowercase kind); the LOG writes `source: Cell[T]` (bracket). 016-246 and 016-283 likewise use bracket `Cell[T]` where §13.2.8 forbids the bracket type. This is the same systemic bracket/kind divergence spread across the observe (§13.2.11) and generics paragraphs; each is an independent LOG-SPEC mismatch an implementer would hit at parse/annotation time.
- **Direction of change:** Batch-conform the bracket-`Cell[T]` occurrences in 016 (016-63,172,175,179,180,246,270,283) to the chosen canonical spelling; these move together with the 017/018 view entries. User picks canonical form; do not decide unilaterally.
- **Evidence check:** pass — 016-246 (observe value) and 016-180/016-283 (read-site params) use bracket `Cell[T]`; SPEC §13.2.8 and its worked passthrough example require lowercase kind `cell T`. Same systemic bracket/kind LOG-SPEC divergence across observe and generics paragraphs.
- **Charity check:** sustain — Same systemic bracket/kind LOG-SPEC split, confirmed across observe and generics. 016-246 (DECISION_LOG.md:2099) 'observe expression's value is a Cell[T]' and 016-270 (DECISION_LOG.md:2123) 'anywhere a Cell[T] value is valid' vs SPEC:13042 'An observe expression produces a cell T'. 016-180 (DECISION_LOG.md:2033) 'source: Cell[T]' vs SPEC:12466-12467 'source: cell T'. 016-283 vs SPEC:12441 'there is no Cell[T] type'. Charitable search for a passage authorizing the bracket spelling of observe values or generic params found none; SPEC uniformly uses the lowercase kind and conflicts with each cited LOG passage, confirming the divergence.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2099-2099`
    > 016-246. An observe expression's value is a `Cell[T]` whose concrete reactive type (a value cell or `Stream[T]`) is determined by the use context. (§13.2.11.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2136-2136`
    > 016-283. Value-reading operator and function parameters are typed `Cell[T]` (the umbrella); a `Stream[T]` is excluded at the read site rather than by the signature. (§13.2.8)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2033-2033`
    > 016-180. Generic functions and operators abstract over the value type via `Cell[T]`: `operator passthrough[T](source: Cell[T]) -> Cell[T]`. (§13.2.8)
  - `packages/ductus-lang/docs/SPEC.md:12466-12468`
    > operator passthrough[T](source: cell T) -> cell T:
    >   source

#### F197 — 013-10 grants the multiple-live-aliases ownership exception to `Cell[T]` parameters by bracket type; SPEC's aliasing/no-`Cell[T]`-type basis is the lowercase `cell T` kind, so the exception's carrier is spelled a nonexistent type.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 013-10 · SPEC § 13.2.8
- **Why it is a defect:** 013-10 (ownership exception) and 016-177 (read-only cell parameter) describe the SAME 'cell received as a parameter' construct with clashing spellings — `Cell[T]` (bracket, 013-10) vs `cell T` (kind, 016-177) — while SPEC §13.2.8 asserts the bracket type does not exist. The ownership carve-out is thus anchored to a spelling the normative SPEC forbids; a reader cannot tell whether the single-ownership exception attaches to a type `Cell[T]` or a kind `cell T`.
- **Direction of change:** Align 013-10's carrier spelling with 016-177 / SPEC §13.2.8 once the global `Cell[T]`-vs-`cell T` decision is made; cross-references §13.2.9 composite aliasing already use kind vocabulary.
- **Evidence check:** pass — 013-10 anchors the multiple-live-aliases ownership exception to a bracketed `Cell[T]` parameter spelling that the referenced SPEC §13.2.8 explicitly denies exists as a type, while the same construct is spelled kind `cell T` in 016-177 and SPEC §13.12.2.
- **Charity check:** sustain — Confirmed LOG-SPEC divergence. SPEC.md:12441 states verbatim 'there is no `Cell[T]` type and no `Cell` trait to fulfill' and the SPEC uses lowercase kind `cell T` throughout §13.2.8 (e.g. SPEC:12426 `fn(s: cell T)`, SPEC:12464-12467 `operator passthrough[T](source: cell T)`, taxonomy row SPEC:12508-12512 `cell T`/`Portal[cell T]`). LOG 013-10 (DECISION_LOG.md:1396) spells the ownership-exception carrier as bracket `Cell[T]`, while its sibling 016-177 (DECISION_LOG.md:2030) spells the same 'cell received as a parameter' construct as kind `cell T`. So the LOG carries both spellings for one construct AND the bracketed one names a type the referenced SPEC §13.2.8 explicitly denies. Divergence per project edit protocol; SUSTAIN. No normative text forces `Cell[T]` to mean something legal distinct from the forbidden type — the SPEC's generics example uses `cell T` in the exact 016-180 slot.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1396-1396`
    > 013-10. Exception: `Cell[T]` parameters and reactive composite bindings name reactive cells, so multiple live aliases to the same cell may coexist without violating single ownership. (§11.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2030-2030`
    > 016-177. A `cell T` received as a parameter is read-only; no source-level form writes through a cell reference. (§13.2.8)
  - `packages/ductus-lang/docs/SPEC.md:12441-12441`
    > all cell kinds; there is no `Cell[T]` type and no `Cell` trait to fulfill.

#### F196 — 016-63 mints internal bindings as bracket types `Cell[Map[K,V]]` and `Cell[DynamicView[T]]`; SPEC §13.2.8/taxonomy says these bracket forms are 'replaced' by lowercase `cell Map[…]` and `dynamic view V`, and no `Cell[T]` type exists.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 016-63 · SPEC § 13.2.8
- **Why it is a defect:** The SPEC and LOG describe the identical compiler-minted bindings with contradictory spellings: LOG 016-63 = `Cell[Map[K,V]]` / `Cell[DynamicView[T]]` (bracket type); SPEC's parallel sentence = `cell Map[…]` / `dynamic view V` (lowercase kind), and the SPEC taxonomy explicitly says the dynamic-view bracket form is 'replaced'. LOG-SPEC divergence on the same paragraph; the LOG kept the pre-amendment bracket spelling the amendment retired.
- **Direction of change:** Conform 016-63's spellings to the retained SPEC forms (or vice versa per the user's chosen direction); note LOG 017-53/017-71/017-193 and 018-129/131/133/141 repeat `Cell[DynamicView[…]]` / `Cell[Map[…]]` and would move with it.
- **Evidence check:** pass — 016-63 mints internal bindings as bracket `Cell[Map[K,V]]` and `Cell[DynamicView[T]]`; the parallel SPEC §13.2.8 sentence writes lowercase `cell Map[…]` / `dynamic view V` and the taxonomy row states `dynamic view V` 'replaces `Cell[DynamicView[WeakHandle[V]]]`'. LOG kept the retired bracket spelling.
- **Charity check:** sustain — LOG-SPEC divergence on compiler-minted internal bindings confirmed. 016-63 (DECISION_LOG.md:1916) mints 'the Cell[Map[K, V]] of a repeat … as view' and 'the Cell[DynamicView[T]] of a dynamic view' (bracket). The parallel SPEC sentence SPEC:12436-12438 writes the identical bindings as 'cell Map[…] for repeat … as views' and 'dynamic view V for dynamic views' (lowercase kind), and SPEC:12509 says 'dynamic view V replaces Cell[DynamicView[WeakHandle[V]]]'. I checked whether SPEC keeps Cell[Map[...]] or Cell[DynamicView[...]] as a live spelling anywhere: grep shows Cell[ appears in SPEC only at 12441 (negation), 12509/13640 ('replaces'/'spelling', retired), and 23152 (Cell[Vec[T]]-shaped descriptive prose) — none legitimizes the minted-binding bracket form. The SPEC passage conflicts with the LOG passage, confirming the divergence; the LOG kept the pre-amendment bracket spelling the amendment retired.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1916-1916`
    > 016-63. Cell-typed bindings typically appear in parameter positions, return positions, and compiler-minted internal bindings such as the `Cell[Map[K, V]]` of a `repeat … as` view or the `Cell[DynamicView[T]]` of a dynamic view. (§13.2.8)
  - `packages/ductus-lang/docs/SPEC.md:12436-12441`
    > **`cell T` is the umbrella KIND.** Cell-kind bindings typically appear in
    > parameter positions, return positions, and compiler-minted internal bindings
    > (`cell Map[…]` for `repeat … as` views per §13.5.4.9, `dynamic view V` for
    > dynamic views per §13.3.3.4). Membership is a KIND relation, not a subtype or
    > trait relation: `signal`, `derived`, `recurrent`, `stream`, and `yielded` are
    > all cell kinds; there is no `Cell[T]` type and no `Cell` trait to fulfill.
  - `packages/ductus-lang/docs/SPEC.md:12509-12509`
    > | Dynamic view | `dynamic view V` | replaces `Cell[DynamicView[WeakHandle[V]]]` (§13.3.3.4) |

#### F187 — 017-215 spells the reactive reference as the bracket type `Cell[T]` and `Portal[Cell[T]]`, but its cited SPEC §13.3.6.3 uses `cell T`/`Portal[cell T]`, and SPEC §13.2.8 states there is no `Cell[T]` type.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 017-215 · SPEC § 13.3.6.3
- **Why it is a defect:** 017-215 uses `Cell[T]` and `Portal[Cell[T]]` as bracket types, but the SPEC section it cites (§13.3.6.3) uses the lowercase kind spelling `cell T`/`Portal[cell T]`, and SPEC §13.2.8 line 12441 explicitly states 'there is no `Cell[T]` type'. The LOG therefore names a type-form the SPEC says does not exist, diverging from its own cited elaboration.
- **Direction of change:** Align 017-215's spelling with the authoritative cell spelling (`cell T` per SPEC, or resolve the razor tension with 016-179 which also uses `Cell[T]`); bring the `Cell[T]`-vs-`cell T` decision to the user, do not resolve unilaterally.
- **Evidence check:** pass — 017-215 spells the reactive reference as bracket type 'Cell[T]'/'Portal[Cell[T]]', but its cited SPEC §13.3.6.3 uses lowercase kind 'cell T'/'Portal[cell T]' and SPEC §13.2.8 says no 'Cell[T]' type exists; LOG names a type-form the SPEC says does not exist — substantive LOG-SPEC divergence.
- **Charity check:** sustain — 017-215 (DECISION_LOG.md:2356) spells the reactive reference with the bracket type `Cell[T]`, `Portal[Cell[T]]`, and `&Cell[T]` (via `Option[&Cell[T]]`). Its cited SPEC §13.3.6.3 (SPEC.md:14410-14417) uses the lowercase kind spelling `cell T`, `Portal[cell T]`, `Option[&cell T]`; and SPEC §13.2.8 (SPEC.md:12441) states outright 'there is no `Cell[T]` type and no `Cell` trait to fulfill', reinforced by the kind table at SPEC.md:12508/12512 (`cell T` 'not a type, not a trait'; `Portal[cell T]` is the sanctioned carrier). The LOG therefore names a type-form the cited SPEC says does not exist -- substantive LOG-SPEC divergence. The finding already IS the divergence; nothing dissolves it.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2356-2356`
    > 017-215. `Cell[T]` and `Portal[T]` are orthogonal: `Cell[T]` is a reactive reference whose read declares a dependency edge into the enclosing expression's provenance, while `Portal[T]` is an inert window whose read does not. `Portal[Cell[T]]` is well-formed — the portal resolves to `Option[&Cell[T]]` and the inner cell read remains reactive by the compiler's implicit cell-value auto-deref. (§13.3.6.3)
  - `packages/ductus-lang/docs/SPEC.md:14410-14414`
    > **Reactivity.** `cell T` (§13.2.8) and `Portal[T]` are orthogonal axes.
    > A `cell T` is a reactive reference whose read declares a dependency edge into the enclosing expression's provenance; `Portal[T]` is an
    > inert window whose read does not. `Portal[cell T]` is well-formed and is
    > the sanctioned identity-as-data carrier for a cell: the portal resolves
    > to `Option[&cell T]`
  - `packages/ductus-lang/docs/SPEC.md:12441-12441`
    > all cell kinds; there is no `Cell[T]` type and no `Cell` trait to fulfill.

#### F184 — Within section 017, 017-192 uses the new kind spelling `dynamic view T` while adjacent 017-193/017-194 use the retired `Cell[DynamicView[WeakHandle[T]]]` / `DynamicView[T]` / `Cell` spelling for the same construct.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 017-192, 017-193, 017-194 · SPEC § 13.3.3.4
- **Why it is a defect:** Two adjacent in-scope entries describing the same dynamic-view construct use mutually inconsistent type vocabulary: 017-192 treats `dynamic view T` as the kind (no `Cell`/`DynamicView` type constructors), while 017-194 speaks of a live `DynamicView[T]` type and a 'containing `Cell`'. An implementer cannot tell whether `DynamicView[T]` and `Cell[...]` are real type constructors or retired spellings.
- **Direction of change:** Make the terminology uniform across 017-192..194 (and 017-23/53/71/125/193) once the authoritative spelling is chosen; surface the choice to the user rather than deciding it.
- **Evidence check:** pass — Within section 017, 017-192 spells the dynamic-view construct as kind 'dynamic view T' (no type constructors) while adjacent 017-193/017-194 use retired 'Cell[DynamicView[WeakHandle[T]]]'/'DynamicView[T]'/'containing Cell'; an implementer cannot tell whether DynamicView[T]/Cell[...] are real type constructors or retired spellings.
- **Charity check:** refile_divergence — The finding frames this as an intra-section-017 contradiction (017-192 `dynamic view T` vs 017-193/017-194 `Cell[DynamicView[WeakHandle[T]]]`/`DynamicView[T]`/`Cell`). The corpus does contain a passage that speaks to which spelling is current -- SPEC.md:12509 declares `dynamic view V` 'replaces `Cell[DynamicView[WeakHandle[V]]]`' -- but that passage CONFIRMS both spellings denote the same construct while marking the LOG's bracket spelling RETIRED. Since SPEC declares retired exactly the vocabulary LOG 017-193/017-194 still use as live, the dissolving passage conflicts with the finding's quoted LOG passages. Per the category rule this is a LOG-SPEC divergence (the decision-of-record carries a spelling the SPEC declares retired), not a mere intra-017 contradiction and not refutable. The implementer-visible defect stands, refiled as divergence. | SPEC.md:12509 > | Dynamic view | `dynamic view V` | replaces `Cell[DynamicView[WeakHandle[V]]]` (§13.3.3.4) | -- CONFLICTS WITH -- DECISION_LOG.md:2334 > 017-193. Iteration narrowing for `Cell[DynamicView[WeakHandle[T]]]`. ... and DECISION_LOG.md:2335 > 017-194. `DynamicView[T]` reactivity surface: the containing `Cell` is what declares the dependency edge.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2333-2333`
    > 017-192. The `dynamic view T` kind is a language-level non-array iterator-shaped kind with `Item = WeakHandle[T]`.
  - `packages/ductus-lang/docs/DECISION_LOG.md:2335-2335`
    > 017-194. `DynamicView[T]` reactivity surface: the containing `Cell` is what declares the dependency edge.

#### F183 — Multiple in-scope 017 entries spell the dynamic-view cell as `Cell[DynamicView[WeakHandle[T]]]`, a spelling SPEC §13.3.3.4 explicitly says it retired and replaced with the kind `dynamic view T`.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 017-23, 017-53, 017-71, 017-125, 017-193, 017-194 · SPEC § 13.3.3.4
- **Why it is a defect:** SPEC conforms to LOG, and the SPEC section these entries cite explicitly labels `Cell[DynamicView[WeakHandle[T]]]` as 'the old spelling' that `dynamic view T` replaced. The LOG (decision-of-record) still carries the retired spelling in six entries, so LOG and its cited SPEC section disagree on the current name of the same construct.
- **Direction of change:** Reconcile the spelling between LOG and SPEC for the dynamic-view cell kind; decide which spelling is authoritative and make both documents use it consistently (do not resolve unilaterally).
- **Evidence check:** pass — LOG (decision-of-record) carries the retired cell spelling `Cell[DynamicView[WeakHandle[T]]]` in four+ entries while its own cited SPEC §13.3.3.4 explicitly labels that spelling retired/replaced by kind `dynamic view T` — LOG and cited SPEC disagree on the current name of the same construct.
- **Charity check:** sustain — Confirmed genuine LOG-SPEC divergence. SPEC.md:13638-13640 states the dynamic-view cell is the cell kind `dynamic view T` and says it 'replaces the old spelling Cell[DynamicView[WeakHandle[T]]]'. Six LOG entries still carry the retired spelling: 017-23 (2164), 017-53 (2194), 017-71 (2212), 017-125 (2266), 017-193 (2334), 017-194 (2335). All cite §13.3.3.x, the same SPEC subtree where the retirement is stated. LOG is the decision-of-record; SPEC must conform, yet SPEC's own retirement text conflicts with the LOG entries on the current name of the same construct. Hunted for any note reinstating or dual-blessing the old spelling — none found; SPEC's single occurrence (13640) is the retirement itself. Direction of change: LOG entries carry the stale name; not deciding which name is canonical (that is the user's call), only reporting the two documents disagree. | none found; SPEC.md:13638-13640 verbatim: 'The cell for a dynamic view is a language-level **cell kind** `dynamic view T` (D0-8; it replaces the old spelling `Cell[DynamicView[WeakHandle[T]]]`).' This reinforces, not dissolves, the finding.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2194-2194`
    > 017-53. A `dynamic` view is a reactive cell — `Cell[DynamicView[WeakHandle[T]]]` — not an array; none of the static array forms apply. (§13.3.3.2)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2334-2334`
    > 017-193. Iteration narrowing for `Cell[DynamicView[WeakHandle[T]]]`. When the cell is iterated in a context that proves the iterated element is reachable
  - `packages/ductus-lang/docs/SPEC.md:13638-13640`
    > **The `dynamic view T` kind.** The cell for a dynamic view is a
    > language-level **cell kind** `dynamic view T` (D0-8; it replaces the old
    > spelling `Cell[DynamicView[WeakHandle[T]]]`).

#### F062 — Within LOG §016 the same value cells are described twice under two incompatible identity models — lowercase KINDS (016-163/164/166/178/179) and bracket TYPES (016-167/171/172/279-283) — in the same section under the same §13.2.8 anchor.

- **Severity/Category/Verdict:** MED / design_smell / CONFIRMED
- **Anchors:** LOG 016-163, 016-166, 016-178, 016-179, 016-167, 016-171, 016-172 · SPEC § 13.2.8
- **Why it is a defect:** 016-179 itself states the 'storability razor' that binding machinery (cells) is spelled as lowercase kinds and only storables keep bracket types — yet 016-168/169/170/171/172 declare exactly those cells as bracket types Signal[T]/Derived[T]/Recurrent[T,N]/Cell[T], and 016-179 even writes `Portal[Cell[T]]` (uppercase) while SPEC §13.2.8 writes `Portal[cell T]`. Violates 001-4 (single authoritative source for syntax) and the LOG's atomic/self-contained invariant: two co-resident entries hand an implementer contradictory annotation syntax for the same construct. Concrete witness: for `operator passthrough[T](source: ???)`, 016-172/016-180 say `Cell[T]` while 016-178 and SPEC say `cell T`.
- **Direction of change:** Pick one identity model across all of LOG §016 (kinds vs bracket types) to match the chosen SPEC model, and delete/rewrite the entries carrying the other model; surface to user which model is authoritative.
- **Evidence check:** pass — Within §016 under one §13.2.8 anchor the same value cells are described under two incompatible identity models — lowercase KINDS (016-163/164/166/178/179) and bracket TYPES (016-167/171/172/279-283) — handing an implementer contradictory annotation syntax for the same construct in co-resident atomic entries.
- **Charity check:** sustain — LOG-internal contradiction: §016 co-hosts two incompatible identity models under the same §13.2.8 anchor. The lowercase-KIND entries — 016-163 (L2016), 016-166 (L2019), 016-178 (L2031 'a cell is named by a lowercase kind annotation — signal T, derived T, recurrent[N] T ...'), 016-179 (L2032, the 'storability razor': 'binding machinery is spelled as lowercase kinds') — install cells-as-kinds and 016-179 even says cells are NOT storable bracket values. Yet the bracket-TYPE entries in the same section — 016-167 ('Signal[T] is a first-class type'), 016-171, 016-172 ('operator value parameter is typed Cell[T]'), 016-180 ('source: Cell[T]'), 016-279/280/281/282/283 — spell the SAME cells as first-class bracket types, and 016-179 itself writes 'Portal[Cell[T]]'. Concrete witness: operator passthrough[T] is typed Cell[T] (016-180/283) vs cell T (016-178, SPEC:12467). Two co-resident entries hand an implementer contradictory annotation syntax for one construct — violates 001-4 (single authoritative source). My dissolving search found only SPEC text that BACKS the lowercase-kind side (SPEC:12441 'there is no Cell[T] type'), reinforcing rather than dissolving. Sustain. Overlaps F061 (that one is LOG-vs-SPEC; this one is LOG-internal).
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2016-2016`
    > 016-163. `cell T` is the umbrella KIND over all reactive cells, streams included; the value cells among them are those declared `signal T`, `derived T`, and `recurrent[N] T`, whose value type is `T`. (§13.2.8)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2031-2032`
    > 016-178. The kind taxonomy has a single home, organized as values / graph entities / cells: storable values are designated directly, or through `Handle` or `Portal`; a cell is named by a lowercase kind annotation — `signal T`, `derived T`, `recurrent[N] T`, `stream ring[N] T`, `stream gate[N] T`, `recurrent[N] stream …`, erased `stream T`, `yielded T`, `cell T`, or `dynamic view X`; static views get no kind form; `const` needs no row; and effects, nodes, and connections annotate by their type name. (§13.2.8)
    > 016-179. The storability razor: storable values keep bracket types (`Type[C]`, `Handle[T]`, `WeakHandle[T]`, `Portal[T]`), while binding machinery is spelled as lowercase kinds; a cell is refused as a value, and `Portal[Cell[T]]` is the sanctioned identity-as-data form. (§13.2.8)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2021-2025`
    > 016-168. `signal X = init` declares a host-writable `Signal[T]`, written via `runtime.write_signal`. (§13.2.8)
    > 016-169. `derived X = expr` declares a `Derived[T]` whose value the runtime keeps consistent with its inputs. (§13.2.8)
    > 016-170. `recurrent[N]? X: T = expression` declares a `Recurrent[T, N]` with self-history via `.previous(fallback)` and `.past(k, fallback)`. (§13.2.8)
    > 016-171. The keyword `signal` names a host-writable `Signal[T]` (written via `runtime.write_signal`); `attr` is also a `Signal[T]` but is placement-written only, never host-writable post-construction; `derived` and `recurrent` are the distinct types `Derived[T]` and `Recurrent[T, N]`. (§13.2.8)

#### F061 — LOG §016 spells value cells as bracket TYPES (Cell[T]/Signal[T]/Derived[T]/Recurrent[T,N]) while SPEC §13.2.8 retired them for lowercase KINDS and states 'there is no Cell[T] type'.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 016-167, 016-171, 016-172, 016-175, 016-266, 016-279, 016-280, 016-281, 016-282, 016-283, 025-13 · SPEC § 13.2.8, 13.12.2
- **Why it is a defect:** Learning 5 and the project edit protocol make LOG-SPEC divergence a defect: the SPEC (the conform target of these entries' §13.2.8 refs) explicitly abolished the bracket-type family for value cells and installed lowercase kinds, yet the referencing LOG entries still name Cell[T]/Signal[T]/Derived[T]/Recurrent[T,N] as first-class types. An implementer reading the LOG for the 'WHAT' gets a type family the SPEC says does not exist. This is the core surface an implementer builds the type system from.
- **Direction of change:** Reconcile LOG §016 value-cell entries with SPEC §13.2.8's kind model (lowercase kinds; no Cell[T]/Signal[T]/Derived[T]/Recurrent[T,N] types), or, if the bracket types are intended, restore them in SPEC — surface to user which model is authoritative before editing.
- **Evidence check:** pass — LOG §016 names value cells as bracket types Cell[T]/Signal[T]/Derived[T]/Recurrent[T,N] (016-167/171/172/175/279-283) while their conform-target SPEC §13.2.8 explicitly retired that family — 'there is no Cell[T] type' — for lowercase KINDS, so an implementer reading the LOG for the WHAT gets a type family the SPEC says does not exist.
- **Charity check:** sustain — Substantive LOG-SPEC divergence at the type-system surface. The referenced target SPEC §13.2.8 explicitly abolished the bracket-type family for value cells: SPEC:12380 'cell is a KIND, not a type or a trait'; SPEC:12408-12411 'signal names the host-writable value cell ... derived and recurrent produce the distinct KINDS derived T and recurrent[N] T'; SPEC:12467 'operator passthrough[T](source: cell T) -> cell T'; SPEC:12441 'there is no `Cell[T]` type and no `Cell` trait to fulfill.' The referencing LOG entries still name them as first-class TYPES: 016-167 (L2020) 'Signal[T] is a first-class type'; 016-171 (L2024) 'signal names a host-writable Signal[T] ... derived and recurrent are the distinct types Derived[T] and Recurrent[T, N]'; 016-172 (L2025) 'typed Cell[T]'; 016-279/280/281 (L2132-2134) 'Derived[T]/Recurrent[T,N] is the type produced by ...'; 016-283 (L2136) 'typed Cell[T]'. An implementer reading the LOG for the WHAT gets a type family the SPEC says does not exist. Per Learning 5 / the edit protocol, LOG-SPEC divergence is a defect. Sustain.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2020-2020`
    > 016-167. `Signal[T]` is a first-class type usable in parameter positions, return types, and generic arguments. (§13.2.8)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2025-2025`
    > 016-172. An operator value parameter is typed `Cell[T]`, binding to any reactive value cell at instantiation and allocating internal state tied to that cell. (§13.2.8)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2132-2134`
    > 016-279. `Derived[T]` is the type produced by a `derived` declaration: a reactive value computation with no self-history. (§13.2.8)
    > 016-280. `Recurrent[T, N]` is the type produced by a `recurrent[N]` declaration: a reactive value computation carrying `N` steps of self-history. (§13.2.8)
    > 016-281. `Derived[T]` is the degenerate zero-history case of `Recurrent[T, N]` (`N = 0`). (§13.2.8)
  - `packages/ductus-lang/docs/SPEC.md:12441-12441`
    > all cell kinds; there is no `Cell[T]` type and no `Cell` trait to fulfill.
  - `packages/ductus-lang/docs/SPEC.md:12377-12384`
    > The value cells carry a current value of type `T` and come in three annotation
    > kinds: `signal T`, `derived T`, and `recurrent[N] T`. `cell` is a **KIND**, not
    > a type or a trait

#### F059 — The LOG spells the cell umbrella as a bracket type `Cell[T]` throughout, but SPEC §13.2.8 says no such type exists and uses lowercase `cell T` — same construct, contradictory surface syntax.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 016-62, 016-63, 016-172, 016-175, 016-179, 016-180, 025-13 · SPEC § 1.1, 1.3, 13.2.8
- **Why it is a defect:** The LOG is the decision-of-record and SPEC must conform to it (project edit protocol; 001-4 makes the spec the single authoritative source for syntax). Here the two documents give directly opposed surface syntax for the identical construct: LOG 016-63/016-172/016-175/016-179/016-180/025-13 write the cell umbrella and cell-typed parameters as the bracket type `Cell[T]` (and `Portal[Cell[T]]`, `Cell[Map[K,V]]`, `Cell[DynamicView[T]]`), while SPEC §13.2.8 states verbatim 'there is no `Cell[T]` type', writes the same signatures as lowercase `cell T`, and its taxonomy table says `dynamic view V` REPLACES `Cell[DynamicView[WeakHandle[V]]]`. Internally the LOG is also split: 016-62 says `cell T` is 'not a type', yet 025-13 calls `Cell[T]` 'a cell type'. An implementer building the lexer/parser from the LOG reserves and emits `Cell[T]` as a bracketed generic type; one building from the SPEC emits `cell T` and reserves NO `Cell` type. The two toolchains reject each other's source. This is a substantive LOG-SPEC divergence (a defect per the project's stated rule) and bears on 001-9's 'every value has a concrete type' story: whether the cell umbrella is a first-class bracket type or a non-storable lowercase kind is left contradictory.
- **Direction of change:** Pick ONE surface spelling 
for the cell umbrella and cell-typed bindings across BOTH documents (either the lowercase `cell T` kind the SPEC insists on, or the bracket `Cell[T]` type the LOG uses), then conform the other document; this is a naming/type-model decision for the user to make, not for me to resolve. Whichever is chosen, reconcile 016-62 ('not a type') with 025-13 ('a cell type').
- **Evidence check:** pass — LOG spells the cell umbrella and cell-typed params as bracket `Cell[T]` throughout (016-63/172/179/180, 025-13) while SPEC §13.2.8 states 'there is no `Cell[T]` type' and uses lowercase `cell T`; LOG is also internally split (016-62 'not a type' vs 025-13 'a cell type').
- **Charity check:** sustain — Systemic LOG-SPEC surface-syntax divergence confirmed, plus an intra-LOG split the finder flagged. LOG spells the umbrella and cell-typed bindings as bracket Cell[T] (016-63/016-172/016-175/016-179/016-180 at DECISION_LOG.md:1916,2025,2028,2032,2033, and 025-13 at :3110 'a cell type (Cell[T])'), including Portal[Cell[T]] (016-179) and Cell[DynamicView[T]] (016-63). SPEC:12439-12441 states 'there is no Cell[T] type', writes signatures as 'cell T' (SPEC:12467), and SPEC:12509 says 'dynamic view V replaces Cell[DynamicView[WeakHandle[V]]]'; SPEC:12512 gives Portal[cell T] (not Portal[Cell[T]]). The finding's noted intra-LOG tension (016-62 'not a type' vs 025-13 'a cell type Cell[T]') is real. I searched for any LOG/SPEC note equating the two spellings; none exists. SPEC text conflicts with the LOG passages, confirming the divergence rather than dissolving it.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1915-1916`
    > 016-62. `cell T` is the umbrella KIND — a lowercase kind annotation, not a type or a trait — over every reactive cell, whose membership is all signals, deriveds, recurrents, streams, and yielded groups. (§13.2.8)
    > 016-63. Cell-typed bindings typically appear in parameter positions, return positions, and compiler-minted internal bindings such as the `Cell[Map[K, V]]` of a `repeat … as` view or the `Cell[DynamicView[T]]` of a dynamic view. (§13.2.8)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2025-2033`
    > 016-172. An operator value parameter is typed `Cell[T]`, binding to any reactive value cell at instantiation and allocating internal state tied to that cell. (§13.2.8)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2032-2033`
    > 016-179. The storability razor: storable values keep bracket types (`Type[C]`, `Handle[T]`, `WeakHandle[T]`, `Portal[T]`), while binding machinery is spelled as lowercase kinds; a cell is refused as a value, and `Portal[Cell[T]]` is the sanctioned identity-as-data form. (§13.2.8)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3110-3110`
    > 025-13. A parameter declared with a cell type (`Cell[T]`) binds to the cell reference itself rather than to its current value: `fn some_fn(s: Cell[T])`. (§13.12.2)
  - `packages/ductus-lang/docs/SPEC.md:12439-12441`
    > trait relation: `signal`, `derived`, `recurrent`, `stream`, and `yielded` are
    > all cell kinds; there is no `Cell[T]` type and no `Cell` trait to fulfill.
  - `packages/ductus-lang/docs/SPEC.md:12466-12467`
    > operator passthrough[T](source: cell T) -> cell T:
  - `packages/ductus-lang/docs/SPEC.md:12509-12512`
    > | Dynamic view | `dynamic view V` | replaces `Cell[DynamicView[WeakHandle[V]]]` (§13.3.3.4) |

#### F157 — 029-45 cites §13.2.8 for a 'Cell[T]≅T relationship', but §13.2.8 contains neither the ≅ symbol nor any Cell[T] spelling; the ≅ wrap language lives in 13.17.5 and is written cell T≅T.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 029-45 · SPEC § 13.2.8, 13.17.5
- **Why it is a defect:** Citation-aptness break plus notation divergence. 029-45 attributes a 'Cell[T]≅T' relationship to §13.2.8, but §13.2.8 (read fully, lines 12377-12521) never uses the ≅ symbol and never spells Cell[T]; the wrap-isomorphism claim actually appears in SPEC 13.17.5 (line 19767) written as cell T≅T with the produced kind derived f32. The entry's pointer to §13.2.8 for the ≅ relationship is therefore inapt, and the Cell[T]/Derived[f32] spellings contradict the kind system in both cited/relevant SPEC sections.
- **Direction of change:** Surface to user: 029-45 should point at the section that actually states the wrap relationship and adopt the agreed notation. Do not edit unilaterally.
- **Evidence check:** pass — 029-45 attributes a 'Cell[T]≅T relationship' to §13.2.8, but §13.2.8 contains neither the ≅ symbol nor any Cell[T] spelling; the wrap-isomorphism language actually appears in §13.17.5 written 'cell T≅T' producing 'derived f32' — an inapt citation plus a Cell[T]/Derived[f32] notation divergence.
- **Charity check:** sustain — 029-45 attributes a '`Cell[T]`≅`T` relationship (§13.2.8)' to SPEC 13.2.8, but 13.2.8 (read fully, 12377-12446) contains ZERO occurrences of the ≅ symbol (grep over 12377-12521: count 0) and never spells Cell[T]. The wrap-isomorphism language actually appears at SPEC line 19767, written as 'the `cell T`≅`T` wrap (§13.2.8), so a `-> f32` return is carried as `derived f32`' — lowercase kinds, not Cell[T]/Derived[f32]. So 029-45's pointer to 13.2.8 for the ≅ relationship is inapt AND the Cell[T]/Derived[f32] spelling contradicts the kind system. (Minor: the finding says ≅ 'lives in 13.17.5'; the single ≅ occurrence at 19767 is in the tail of 13.17.4 just above the 13.17.5 header at 19776, and 029-45 itself refs §13.17.5 — does not affect the core sustain.) Citation-aptness break plus notation divergence.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:3424-3424`
    > 029-45. An operator's value-typed output is implicitly wrapped into a reactive cell via the `Cell[T]`≅`T` relationship (§13.2.8): a `-> f32` return is the reactive output's value type, carried as `Derived[f32]`, never a non-reactive snapshot. (§13.17.5)
  - `packages/ductus-lang/docs/SPEC.md:12436-12441`
    > **`cell T` is the umbrella KIND.** Cell-kind bindings typically appear in
    > parameter positions, return positions, and compiler-minted internal bindings
    > (`cell Map[…]` for `repeat … as` views per §13.5.4.9, `dynamic view V` for
    > dynamic views per §13.3.3.4). Membership is a KIND relation, not a subtype or
    > trait relation: `signal`, `derived`, `recurrent`, `stream`, and `yielded` are
    > all cell kinds; there is no `Cell[T]` type and no `Cell` trait to fulfill.
  - `packages/ductus-lang/docs/SPEC.md:19767-19768`
    > exposes that cell as the operator instance's output — the `cell T`≅`T`
    > wrap (§13.2.8), so a `-> f32` return is carried as `derived f32`, never a

#### F156 — LOG 029 pervasively uses bracket cell-TYPES (Cell[T], Signal[T], Derived[T], Recurrent[T,N], Stream[T]) but its cited SPEC 13.2.8 abolishes them: 'there is no Cell[T] type', cells are KINDs written cell T / signal T / derived T / recurrent[N] T.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 029-2, 029-6, 029-11, 029-12, 029-18, 029-20, 029-22, 029-31, 029-41, 029-45, 029-69, 029-75, 029-112, 029-122, 029-124 · SPEC § 13.2.8, 13.17
- **Why it is a defect:** Per the edit protocol SPEC must conform to LOG; here the two are in direct notational contradiction over the reactive-cell type system. SPEC 13.2.8 and SPEC 13.17 have migrated to lowercase KIND annotations (cell T, signal T, derived T, recurrent[N] T, stream T) and explicitly state no Cell[T] type/trait exists, while LOG 029 still declares operator params/returns as bracket types Cell[T]/Derived[T]/Signal[T]/Recurrent[T,N]/Stream[T]. An implementer reading the decision-of-record would build a Cell[T] type the SPEC forbids. One side must be brought into conformance; which side is the authority is a decision for the user (LOG is nominally decision-of-record, but SPEC 13.2.8/13.17 and LOG 016-178/016-179 storability-razor entries encode the newer kind system).
- **Direction of change:** Surface to user: decide whether the kind-notation (cell T/signal T/derived T/recurrent[N] T/stream T, no Cell[T] type) is the adopted system; if so, LOG 029 entries must be reworded to kind notation to conform to their cited SPEC 13.2.8/13.17. Do not resolve unilaterally.
- **Evidence check:** pass — LOG 029 pervasively declares operator params/returns as bracket cell-TYPES (Cell[T], Derived[T], Signal[T], Recurrent[T,N], Stream[T]) while its cited SPEC 13.2.8 abolishes them — cells are KINDs (cell T / signal T / derived T / recurrent[N] T) and 'there is no Cell[T] type' — a direct notational divergence over the reactive-cell type system.
- **Charity check:** sustain — Substantive LOG-SPEC notational divergence over the entire reactive-cell type system, confirmed both directions. LOG 029 pervasively declares operator params/returns with BRACKET cell-types across ~16 entries: 029-2 'Cell[T] inputs', 029-6 'operator double(source: Cell[f32]) -> Derived[f32]', 029-11 'Signal[T]/Derived[T]/Recurrent[T, N]/Stream[T]', 029-12/18/20/22/31/41/45/69/70/75/112/122/124 all using Cell[T]/Derived[T]/Signal[T]/Stream[T]. SPEC 13.2.8 (12440-12441) explicitly abolishes them: 'there is no `Cell[T]` type and no `Cell` trait to fulfill'; cells are KINDs written cell T / signal T / derived T / recurrent[N] T (12380-12384). SPEC 13.17 (the section 029 cites) contains ZERO bracket cell-types and 65 lowercase kind annotations (grep of lines 19600-20140; sample 'operator example(s: cell f32) -> derived f32'). An implementer reading the decision-of-record would build a Cell[T] type the SPEC forbids. Which side is authority is a user decision. MED divergence sustained.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:3391-3391`
    > 029-12. An operator parameter may be any value: cell-bound `name: Cell[T]`, value `name: T`, or function- or operator-typed (carried structurally, §13.17.13). (§13.17.3)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3503-3503`
    > 029-124. Ruling A: a value-reading operator or function parameter is typed `Cell[T]` (the umbrella), and an operator's computed value output is a `Derived[T]`; a `Stream[T]` has no current value and is excluded at the read site, not by the signature. (§13.17.3)
  - `packages/ductus-lang/docs/SPEC.md:12440-12441`
    > trait relation: `signal`, `derived`, `recurrent`, `stream`, and `yielded` are
    > all cell kinds; there is no `Cell[T]` type and no `Cell` trait to fulfill.
  - `packages/ductus-lang/docs/SPEC.md:12380-12384`
    > kinds: `signal T`, `derived T`, and `recurrent[N] T`. `cell` is a **KIND**, not
    > a type or a trait: `cell T` is the umbrella designator spanning *all* reactive
    > cells — these value kinds plus the event kind `stream T` and the membership kind
    > `yielded T` (§13.20.4). `cell` is never written with brackets; it appears only
  - `packages/ductus-lang/docs/SPEC.md:12415-12418`
    > - **Operator parameters** (§13.17) — a value-reading operator parameter is
    >   annotated `cell T`, binding to any value cell at instantiation and allocating
    >   internal state tied to it. A `stream T` has no current value, so it is
    >   excluded at the read site, not by the annotation.

#### F155 — 016-1 heads all seven kinds as 'reactive declaration kinds' and includes `const`, but 016-4/016-115/016-118 state const is not reactive and occupies no reactive cell.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 016-1, 016-4, 016-115, 016-118 · SPEC § 13.2, 13.2.5, 13.2.5.1
- **Why it is a defect:** 016-1 classifies `const` as one of the 'reactive declaration kinds' and says the taxonomy is 'distinguished by who controls the value and how it changes'. But 016-115 says const is not reactive and 016-118 says it occupies no reactive cell and never participates in dirty propagation — a const's value never changes. Calling const a 'reactive declaration kind' contradicts its own defining entries. This is a category error inside the taxonomy header, not just wording: a reader deciding whether `const` participates in reactive-cell machinery gets 'yes' from 016-1's label and 'no' from 016-115/118.
- **Direction of change:** Re-scope the 016-1 header so `const` is not asserted to be reactive (e.g. separate the reactive kinds from the compile-time const), keeping consistency with 016-4/016-115/016-118. Surface to user.
- **Evidence check:** pass — 016-1 classifies const among the 'reactive declaration kinds' while 016-115 states const is not reactive and 016-118 states it occupies no reactive cell and never participates in dirty propagation; a reader asking whether const touches reactive-cell machinery gets 'yes' from the header label and 'no' from its own defining entries.
- **Charity check:** refile_divergence — The finding is real but its category is wrong: it is not a logical_flaw dissolvable by a forced reading — it is a divergence between two LOG passages. 016-1 (L1854) literally applies the adjective 'reactive' to a list that INCLUDES const: 'exactly seven reactive declaration kinds — signal, attr, recurrent, derived, const, stream, yielded'. 016-115 (L1968) says the opposite at face value: 'A const is not reactive'; 016-118 (L1971): 'A const occupies no reactive cell and does not participate in dirty propagation'. Reconciling them requires reinterpreting 016-1's 'reactive declaration kinds' as a mere system-grouping header rather than a per-kind claim — an inference NOT forced by 016-1's own words. Per the category-conditional rule, when the dissolving passage (016-115/118: const not reactive, no cell) conflicts with the finding's quoted passage (016-1: const IS a reactive declaration kind), the verdict is refile_divergence, not refute. Both operational entries agree const has no cell, so no concrete behavior diverges (MED severity ceiling), but the header wording literally contradicts 016-115. | DECISION_LOG.md:1968 '016-115. A `const` is not reactive and not per-instance: its value is identical for every instance of the declaring type and fixed at compile time. (§13.2.5)' AND DECISION_LOG.md:1971 '016-118. A const occupies no reactive cell and does not participate in dirty propagation. (§13.2.5.1)' — these conflict at face value with the finding's passage DECISION_LOG.md:1854 '016-1. There are exactly seven reactive declaration kinds — `signal`, `attr`, `recurrent`, `derived`, `const`, `stream`, and `yielded`'.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1854-1854`
    > 016-1. There are exactly seven reactive declaration kinds — `signal`, `attr`, `recurrent`, `derived`, `const`, `stream`, and `yielded`
  - `packages/ductus-lang/docs/DECISION_LOG.md:1968-1968`
    > 016-115. A `const` is not reactive and not per-instance: its value is identical for every instance of the declaring type and fixed at compile time. (§13.2.5)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1971-1971`
    > 016-118. A const occupies no reactive cell and does not participate in dirty propagation. (§13.2.5.1)

#### F154 — The `cell T` umbrella KIND membership diverges between entries: 016-62 includes 'yielded groups'; 016-166 lists membership as signal/derived/recurrent/stream and omits yielded entirely.

- **Severity/Category/Verdict:** MED / contradiction / CONFIRMED
- **Anchors:** LOG 016-62, 016-166, 016-163 · SPEC § 13.2.8
- **Why it is a defect:** Three entries in the same section define the membership of the single `cell T` umbrella KIND and disagree on whether `yielded` groups are members. 016-62 explicitly enumerates 'yielded groups' as members; 016-166 gives an apparently-exhaustive list ('those declared signal T, derived T, recurrent[N] T, and every stream cell') that omits yielded; 016-163 omits it too. Since 016-1's annotation row lists `yielded T` as a cell annotation and 016-62 includes it, whether `x: cell T` accepts a yielded group is a behavior-changing question a type-checker must answer, and the LOG gives both answers.
- **Direction of change:** Make the three umbrella-membership entries state one consistent member set (decide yielded's membership) or collapse the redundant restatements into one authoritative entry. Escalate the yielded-membership call to the user.
- **Evidence check:** pass — 016-62 lists yielded groups as members of the `cell T` umbrella KIND; 016-166 and 016-163 give apparently-exhaustive membership lists omitting yielded. A type-checker gets two answers on whether `x: cell T` accepts a yielded group.
- **Charity check:** sustain — Intra-LOG membership contradiction on yielded stands. 016-62 (DECISION_LOG.md:1915) enumerates 'all signals, deriveds, recurrents, streams, and yielded groups' as cell-T members. 016-166 (DECISION_LOG.md:2019) gives an apparently-closed list 'those declared signal T, derived T, recurrent[N] T, and every stream cell' with no 'including'/'such as' hedge, omitting yielded; 016-163 (DECISION_LOG.md:2016) similarly omits it. Since 016-1 (DECISION_LOG.md:1854) makes yielded T a first-class cell kind, whether x: cell T accepts a yielded group is a behavior-changing type-checker question the LOG answers both ways. The dissolving candidate SPEC:20535 ('yielded T groups likewise participate as cell kinds') sides with 016-62 but does not remove the LOG-internal disagreement between 016-62 and 016-166; per the category rule SPEC agreeing with one horn confirms the other horn (016-166) is the defective one — sustain.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1915-1915`
    > 016-62. `cell T` is the umbrella KIND — a lowercase kind annotation, not a type or a trait — over every reactive cell, whose membership is all signals, deriveds, recurrents, streams, and yielded groups. (§13.2.8)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2019-2019`
    > 016-166. `cell T` is the umbrella KIND over all reactive cells: those declared `signal T`, `derived T`, `recurrent[N] T`, and every `stream` cell. (§13.2.8)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2016-2016`
    > 016-163. `cell T` is the umbrella KIND over all reactive cells, streams included; the value cells among them are those declared `signal T`, `derived T`, and `recurrent[N] T`, whose value type is `T`. (§13.2.8)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1854-1854`
    > `stream ring[N] T`, `yielded T`, `cell T`) — and the two rows are not the same list. (§13.2)

#### F148 — 016-164 says an attr annotates as the `signal T` kind, but 016-171 says an attr 'is also a `Signal[T]`' bracket type — the same fact stated two incompatible ways in adjacent entries.

- **Severity/Category/Verdict:** MED / contradiction / CONFIRMED
- **Anchors:** LOG 016-164, 016-171 · SPEC § 13.2.8
- **Why it is a defect:** Both entries describe the annotation form of `attr`. 016-164 uses the lowercase kind `signal T`; 016-171 uses the bracket type `Signal[T]`. The referenced SPEC §13.2.8 (L12409) uses `signal T`. An implementer reading only the LOG cannot tell which spelling is normative for an attr's annotation. This is the same regime split as the prior finding but with both sides inside a single fact, making it a direct atomic contradiction.
- **Direction of change:** Pick one annotation spelling for attr and make 016-164 and 016-171 agree with it and with SPEC.
- **Evidence check:** pass — 016-164 spells an attr's annotation as the lowercase kind 'signal T' while adjacent 016-171 spells it as bracket type 'Signal[T]'; the same fact is stated two incompatible ways and an implementer reading only the LOG cannot tell which spelling is normative (SPEC §13.2.8 uses the lowercase form).
- **Charity check:** sustain — Direct atomic contradiction between two entries stating the same fact — attr's annotation form. 016-164 (L2017): 'An attr annotates as the `signal T` kind' (lowercase kind). 016-171 (L2024): 'attr is also a `Signal[T]`' (bracket type). The SPEC ref target §13.2.8 uses the kind form: SPEC:12393 'attr X: T = default — a `signal T`' and SPEC:12409 'an attr is also a `signal T`'. 016-178/179 establish that lowercase kinds and bracket storable types are DISTINCT regimes and SPEC:12441 states 'there is no Cell[T] type', so `signal T` and `Signal[T]` cannot be the same thing spelled two ways. An implementer reading only the LOG cannot tell which spelling is normative for an attr. This is a narrower instance of the F062/F061 regime split but stands alone as a single-fact atomic contradiction. No dissolving passage found — SPEC backs `signal T`. Sustain.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2017-2017`
    > 016-164. An `attr` annotates as the `signal T` kind: it is a placement-written signal whose value is supplied by the placing parent (§13.8.2), reference-passable as a read-only `signal T` with no host write API after construction. (§13.2.8)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2024-2024`
    > 016-171. The keyword `signal` names a host-writable `Signal[T]` (written via `runtime.write_signal`); `attr` is also a `Signal[T]` but is placement-written only, never host-writable post-construction; `derived` and `recurrent` are the distinct types `Derived[T]` and `Recurrent[T, N]`. (§13.2.8)

#### F147 — Within the §13.2.8 LOG block, one set of entries declares Signal[T]/Derived[T]/Recurrent[T,N] as first-class bracket types while adjacent entries declare cell/signal are KINDS and 'a cell is refused as a value' — mutually incompatible.

- **Severity/Category/Verdict:** MED / contradiction / CONFIRMED
- **Anchors:** LOG 016-167, 016-171, 016-179, 016-178, 016-166 · SPEC § 13.2.8
- **Why it is a defect:** 016-167 asserts `Signal[T]` IS a first-class type and 016-171 says `signal`/`attr`/`derived`/`recurrent` name the bracket types `Signal[T]`/`Derived[T]`/`Recurrent[T,N]`. 016-178/016-179 assert the opposite regime: cells are named by lowercase kind annotations, are NOT storable values, and 'a cell is refused as a value.' These cannot both hold — either `Signal[T]` is a first-class storable type or a cell is refused as a value. Two careful readers get incompatible type systems. SPEC §13.2.8 sides with 016-178/016-179, leaving 016-167/016-171 stranded.
- **Direction of change:** Decide (with the user) whether cells are lowercase kinds (retire the bracket-type entries) or first-class bracket types (retire the kind entries), then make the whole §13.2.8 LOG block internally consistent and matching SPEC.
- **Evidence check:** pass — 016-167/016-171 declare Signal[T]/Derived[T]/Recurrent[T,N] first-class storable bracket types; 016-178/016-179 declare cells lowercase KINDs refused as values. Both in §13.2.8; an implementer cannot honor both surface regimes.
- **Charity check:** sustain — Intra-LOG contradiction stands. 016-167 (DECISION_LOG.md:2020) 'Signal[T] is a first-class type' and 016-171 (DECISION_LOG.md:2024) name Signal[T]/Derived[T]/Recurrent[T,N] as the types of signal/attr/derived/recurrent, while 016-178 (DECISION_LOG.md:2031) says 'a cell is named by a lowercase kind annotation' and 016-179 (DECISION_LOG.md:2032) says 'a cell is refused as a value'. These are jointly unsatisfiable: either Signal[T] is a first-class storable type or a cell is refused as a value. I searched the whole corpus for a reconciliation note equating the bracket and kind forms (grep for legacy/synonym/older near cell) and found none in the LOG. The nearest dissolving text is SPEC:12408-12411, which replaces the bracket value-cell types with lowercase kinds ('signal names the host-writable value cell... derived and recurrent produce the distinct kinds derived T and recurrent[N] T') and SPEC:12441 'there is no Cell[T] type' — but that text CONFIRMS the kind side and conflicts with 016-167/016-171's LOG passage, so it cannot refute; per the category rule it reinforces the sustain (SPEC strands 016-167/016-171). No forced reading eliminates the alternative inside the LOG.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2020-2020`
    > 016-167. `Signal[T]` is a first-class type usable in parameter positions, return types, and generic arguments. (§13.2.8)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2024-2024`
    > 016-171. The keyword `signal` names a host-writable `Signal[T]` (written via `runtime.write_signal`); `attr` is also a `Signal[T]` but is placement-written only, never host-writable post-construction; `derived` and `recurrent` are the distinct types `Derived[T]` and `Recurrent[T, N]`. (§13.2.8)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2032-2032`
    > 016-179. The storability razor: storable values keep bracket types (`Type[C]`, `Handle[T]`, `WeakHandle[T]`, `Portal[T]`), while binding machinery is spelled as lowercase kinds; a cell is refused as a value, and `Portal[Cell[T]]` is the sanctioned identity-as-data form. (§13.2.8)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2031-2031`
    > 016-178. The kind taxonomy has a single home, organized as values / graph entities / cells: storable values are designated directly, or through `Handle` or `Portal`; a cell is named by a lowercase kind annotation — `signal T`, `derived T`, `recurrent[N] T`, `stream ring[N] T`, `stream gate[N] T`, `recurrent[N] stream …`, erased `stream T`, `yielded T`, `cell T`, or `dynamic view X`; static views get no kind form; `const` needs no row; and effects, nodes, and connections annotate by their type name. (§13.2.8)

#### F146 — SPEC §13.2.8 flatly states there is no Cell[T] type, but in-scope LOG entries repeatedly type parameters/returns as bracketed Cell[T]; the cited section contradicts them.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 016-172, 016-175, 016-180, 016-246, 016-270, 016-283 · SPEC § 13.2.8
- **Why it is a defect:** SPEC must conform to LOG, yet the LOG entries and their cited SPEC section describe two mutually exclusive surface forms for the same construct: a bracketed generic type `Cell[T]` (LOG) vs a lowercase kind annotation `cell T` that SPEC declares is NOT a type at all. An implementer cannot write both spellings; this is a substantive LOG-SPEC divergence across the whole §13.2.8 operator/function-parameter surface, exactly the Phase-17 landing zone flagged in the risk note (SPEC updated to the kind regime, these LOG entries not propagated).
- **Direction of change:** Reconcile the bracket-vs-kind regime for §13.2.8 in one direction and make LOG and SPEC agree; surface to the user which regime is authoritative rather than picking one.
- **Evidence check:** pass — LOG §13.2.8 types params/returns as bracket `Cell[T]` (016-172/175/180/283); SPEC §13.2.8 states 'there is no `Cell[T]` type' and writes the same passthrough operator as `source: cell T`. SPEC must conform to LOG; divergence is a defect.
- **Charity check:** sustain — LOG-SPEC divergence confirmed, not dissolved. 016-172/016-175/016-180/016-283 (DECISION_LOG.md:2025,2028,2033,2136) type parameters/returns as bracket Cell[T]; the cited section SPEC:12441 states verbatim 'there is no Cell[T] type and no Cell trait to fulfill' and SPEC:12467 writes the identical passthrough signature as 'operator passthrough[T](source: cell T) -> cell T'. Charitable hunt: I checked whether SPEC defines Cell[T] as a legal synonym anywhere — grep found only SPEC:12441 (negation), 12509/13640 ('replaces'/'spelling', i.e. retired), and 23152 (descriptive prose). No normative text legitimizes the bracket surface form the LOG uses. The SPEC passage directly conflicts with the LOG passages, so it confirms the divergence.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2025-2025`
    > 016-172. An operator value parameter is typed `Cell[T]`, binding to any reactive value cell at instantiation and allocating internal state tied to that cell. (§13.2.8)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2028-2028`
    > 016-175. A function parameter declared `s: Cell[T]` receives the cell reference itself. (§13.2.8)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2033-2033`
    > 016-180. Generic functions and operators abstract over the value type via `Cell[T]`: `operator passthrough[T](source: Cell[T]) -> Cell[T]`. (§13.2.8)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2136-2136`
    > 016-283. Value-reading operator and function parameters are typed `Cell[T]` (the umbrella); a `Stream[T]` is excluded at the read site rather than by the signature. (§13.2.8)
  - `packages/ductus-lang/docs/SPEC.md:12441-12441`
    > all cell kinds; there is no `Cell[T]` type and no `Cell` trait to fulfill.
  - `packages/ductus-lang/docs/SPEC.md:12467-12467`
    > operator passthrough[T](source: cell T) -> cell T:

#### F049 — 013-199 enumerates signal/attr/recurrent/derived as forbidden closure-cell kinds but omits `stream`, while 025-55 forbids closures as any reactive cell type.

- **Severity/Category/Verdict:** LOW / gap / CONFIRMED
- **Anchors:** LOG 013-199 · SPEC § 11.10.6, 13.12.4
- **Why it is a defect:** 013-199's enumeration is narrower than 025-55's blanket rule: it names four cell kinds and leaves `stream` (a reactive cell kind per 016-1) unaddressed. An implementer relying on 013-199 alone might accept a `stream` of closures. Not a hard contradiction (025-55 covers it) but the atomic entry is incomplete relative to the actual cell-kind taxonomy.
- **Direction of change:** Either broaden 013-199 to say 'any reactive cell kind' (aligning with 025-55) or explicitly note streams are covered elsewhere. Surface to user.
- **Evidence check:** pass — 013-199 enumerates four forbidden closure-cell kinds omitting stream; 025-55 forbids closures as any reactive cell type.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1585-1585`
    > 013-199. A closure cannot be the value type of a `signal`, `attr`, `recurrent`, or `derived` cell. (§11.10.6)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3152-3152`
    > 025-55. Functions and closures are not permitted as reactive cell types in v1. (§13.12.4)

#### F245 — 016-64's umbrella rule states a cell-name in a reactive expression resolves to an auto-deref'd cell-pool value read, but for a stream cell-name a bare read is a compile error (030-246), not a value read.

- **Severity/Category/Verdict:** LOW / divergence / CONFIRMED
- **Anchors:** LOG 016-64, 030-246 · SPEC § 13.17.3.1, 13.18.16
- **Why it is a defect:** 016-64 says 'A cell-name' (the umbrella per 016-62/016-166 includes streams) resolves to 'an auto-deref'd cell-pool read' — i.e. a scalar current-value read at the name's position. That delivery is the VALUE-cell semantics; SPEC 12449-12456 illustrates it only with value cells (`s + 1`). For a stream cell-name the delivery is NOT a value read: reading a bare stream as a value is a compile error (030-246), and a bare stream name in an observe arm body 'never denotes an event value' (SPEC 12933-12934). So the umbrella rule 016-64 states only the value-read delivery and, taken over its full umbrella scope, mischaracterizes what a dependency edge from a stream delivers (event participation / error, not an auto-deref'd value). Answers the director's seed: a stream does NOT have 'a value at commit' the way 016-64's read model assumes.
- **Direction of change:** Scope 016-64 to value cells, or add the stream carve-out (bare stream name is not a value read; it participates as an event source or is an error per 030-246), so the umbrella rule does not assert a scalar value read for stream cell-names. User decides wording; do not resolve unilaterally.
- **Evidence check:** pass — 016-64 says a cell-name resolves to an auto-deref'd value read, but for a stream cell-name a bare value read is a compile error (030-246), not a value read.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1917-1917`
    > 016-64. A cell-name in a reactive expression is a compile-time identifier, not a value or a borrow: the compiler resolves it into a provenance entry for the enclosing expression and an auto-deref'd cell-pool read inserted at the name's position. (§13.17.3.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3752-3752`
    > 030-246. Reading a bare stream as a value is a normative error class: `derived latest: Event = events` ✗ when `events` is a stream. (§13.18.16)
  - `packages/ductus-lang/docs/SPEC.md:12449-12456`
    > **Cell-names in reactive expressions are compile-time identifiers, not
    > values or borrows.** When a cell-name appears in a reactive expression — `s + 1`
    > where `s` is a `signal T`, `posts_view.get(k)?.avatar`, `subject.attr_name`,
    > etc. — the compiler resolves the name into two things: (a) a provenance entry
    > for the enclosing expression's dependency set, and (b) an auto-deref'd
    > cell-pool read inserted at the name's position (per §13.17.3.1). No
    > `cell T` value is materialized at the name's position; no borrow object
    > is constructed.

#### F066 — The `cell T` umbrella KIND is defined three times in section 016 with three different membership lists (016-62 includes yielded groups and omits attrs; 016-163 and 016-166 omit yielded groups), leaving the KIND's exact membership under-determined.

- **Severity/Category/Verdict:** LOW / divergence / CONFIRMED
- **Anchors:** LOG 016-62, 016-163, 016-166 · SPEC § 13.2.8
- **Why it is a defect:** Three atomic rules (each must be self-contained per LOG Invariant 2) redefine the same `cell T` KIND with non-identical membership: 016-62 lists 'signals, deriveds, recurrents, streams, and yielded groups' (attrs absent, yielded present); 016-163 and 016-166 list signal/derived/recurrent/stream (yielded absent, attr only reachable via 016-164's 'attr annotates as signal T'). An implementer cannot tell from the LOG alone whether a `yielded T` group and an `attr` are members of `cell T`. This is terminology/membership drift without a concrete program-behavior witness, so LOW, but it is a genuine divergence among restatements that should collapse to one authoritative membership list.
- **Direction of change:** Collapse the three restatements to a single authoritative membership statement for `cell T` (explicitly including or excluding attr and yielded groups), and make the other occurrences reference the same membership without contradicting it. Decision on the canonical membership belongs to the user.
- **Evidence check:** pass — cell T umbrella KIND defined three times in §016 with non-identical membership (yielded/attr in/out), leaving membership under-determined.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1915-1915`
    > 016-62. `cell T` is the umbrella KIND — a lowercase kind annotation, not a type or a trait — over every reactive cell, whose membership is all signals, deriveds, recurrents, streams, and yielded groups. (§13.2.8)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2016-2019`
    > 016-163. `cell T` is the umbrella KIND over all reactive cells, streams included; the value cells among them are those declared `signal T`, `derived T`, and `recurrent[N] T`, whose value type is `T`. (§13.2.8)
    > 016-164. An `attr` annotates as the `signal T` kind: it is a placement-written signal whose value is supplied by the placing parent (§13.8.2), reference-passable as a read-only `signal T` with no host write API after construction. (§13.2.8)
    > 016-165. Streams are reactive cells but are not value cells (not `Signal[T]`/`Derived[T]`/`Recurrent[T, N]`). (§13.2.8)
    > 016-166. `cell T` is the umbrella KIND over all reactive cells: those declared `signal T`, `derived T`, `recurrent[N] T`, and every `stream` cell. (§13.2.8)

#### F159 — SPEC taxonomy table spells the sanctioned cell-identity carrier as `Portal[cell T]` (lowercase, unbracketed) but the LOG consistently spells it `Portal[Cell[T]]` (capital-C, bracketed).

- **Severity/Category/Verdict:** LOW / divergence / CONFIRMED
- **Anchors:** LOG 016-179, 017-215 · SPEC § 13.2.8.1
- **Why it is a defect:** The storability razor (016-179) hinges on `Cell[T]` being a storable BRACKET type, not the lowercase `cell` kind (which per 016-179 is refused as a value). Writing `Portal[cell T]` puts the lowercase reactive-machinery kind inside `Portal[...]`, contradicting the very rule this table's Rationale restates two lines below ('a designator that names something storable keeps brackets'). LOG 016-179 and 017-215 both fix the spelling as `Portal[Cell[T]]`. This is a LOG-SPEC notation divergence, not spec-only-normative content: the underlying carrier is backed.
- **Direction of change:** Reconcile the SPEC table cell to the LOG spelling `Portal[Cell[T]]` (or, if the lowercase form is intended, raise the notation choice to the user — do not resolve unilaterally, per LEARNINGS 1/14).
- **Evidence check:** pass — SPEC table spells carrier Portal[cell T] (lowercase) but LOG spells Portal[Cell[T]] (capital-C bracket).
- **Evidence:**
  - `packages/ductus-lang/docs/SPEC.md:12512-12512`
    > | Storable — non-graph slot | `Portal[T]` | identity-as-data; `Portal[cell T]` is the sanctioned cell-identity carrier |
  - `packages/ductus-lang/docs/DECISION_LOG.md:2032-2032`
    > 016-179. The storability razor: storable values keep bracket types (`Type[C]`, `Handle[T]`, `WeakHandle[T]`, `Portal[T]`), while binding machinery is spelled as lowercase kinds; a cell is refused as a value, and `Portal[Cell[T]]` is the sanctioned identity-as-data form. (§13.2.8)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2356-2356`
    > 017-215. `Cell[T]` and `Portal[T]` are orthogonal: `Cell[T]` is a reactive reference whose read declares a dependency edge into the enclosing expression's provenance, while `Portal[T]` is an inert window whose read does not. `Portal[Cell[T]]` is well-formed — the portal resolves to `Option[&Cell[T]]` and the inner cell read remains reactive by the compiler's implicit cell-value auto-deref. (§13.3.6.3)

#### F158 — 029-11 and 029-69 spell the recurrent cell type Recurrent[T, N] (both params bracketed, T-then-N order), but SPEC 13.2.8 writes it recurrent[N] T (only N bracketed, kind keyword) — a distinct notational divergence within the same system.

- **Severity/Category/Verdict:** LOW / divergence / CONFIRMED
- **Anchors:** LOG 029-11, 029-69 · SPEC § 13.2.8
- **Why it is a defect:** Even setting aside the Cell[T]-vs-kind question, the recurrent designator's spelling differs: LOG writes the storable-style Recurrent[T, N] with the history depth as a type parameter after T; SPEC 13.2.8 writes the kind recurrent[N] T with N as a bracketed count before the value type. These are not interchangeable notations and would confuse an implementer about where N sits. Folds into the same conformance decision as the primary finding but is called out separately because the fix touches ordering/placement, not just casing.
- **Direction of change:** Surface to user as part of the notation-conformance decision; align LOG's recurrent spelling to the SPEC kind form (or vice versa) once the authority is chosen. Do not resolve unilaterally.
- **Evidence check:** pass — LOG spells the recurrent type Recurrent[T, N]; SPEC 13.2.8 spells it recurrent[N] T — divergent notation.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:3390-3390`
    > 029-11. An operator's declared return type may be any type — a value type, a record/tuple (plain or with cell fields), or an explicit `Cell` type (`Signal[T]`/`Derived[T]`/`Recurrent[T, N]`/`Stream[T]`); the output is exposed to callers as a reactive cell. (§13.17.2)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3448-3448`
    > 029-69. The `|>` LHS must be an expression of a reactive cell type (`Signal[T]`, `Derived[T]`, `Recurrent[T, N]`, `Stream[T]`, or other `Cell[T]`) or convertible to one. (§13.17.7)
  - `packages/ductus-lang/docs/SPEC.md:12410-12411`
    > only and host-unreachable post-construction. `derived` and `recurrent` produce
    > the distinct kinds `derived T` and `recurrent[N] T`.

#### F152 — 016-243 states the observe arm grammar as on-clause + optional where + colon + expression, omitting the `as` binder that SPEC §13.2.11.1 makes part of the full arm grammar.

- **Severity/Category/Verdict:** LOW / divergence / CONFIRMED
- **Anchors:** LOG 016-243 · SPEC § 13.2.11.1
- **Why it is a defect:** 016-243 is the atomic entry defining the arm's structure; it lists only on-clause, where, and expression. SPEC §13.2.11.1 defines the full arm grammar as `on <trigger> [where C] [as <binder>]:`, with a whole paragraph of `as` binder semantics. The structural entry cited to that section is incomplete relative to it. 016-269 does mention the `as` binder, so a LOG-only reader is not fully deprived, hence LOW; but the grammar-defining entry diverges from the SPEC grammar it points to.
- **Direction of change:** Either add the optional `as` binder to 016-243's arm-structure statement or confirm the binder is intentionally carried solely by 016-269, aligning with SPEC §13.2.11.1.
- **Evidence check:** pass — 016-243 omits the 'as' binder that SPEC §13.2.11.1 makes part of the full observe-arm grammar.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2096-2096`
    > 016-243. An observe arm is an `on` clause listing one or more trigger cells, an optional `where` filter, and a colon followed by the arm expression: `on T3 where C: expr_filtered`. (§13.2.11.1)
  - `packages/ductus-lang/docs/SPEC.md:12922-12922`
    >   `on <trigger> [where C] [as <binder>]:`.

#### F151 — 016-179 gives the identity carrier as `Portal[Cell[T]]`, but SPEC §13.2.8.1's storability table gives it as `Portal[cell T]`.

- **Severity/Category/Verdict:** LOW / divergence / CONFIRMED
- **Anchors:** LOG 016-179 · SPEC § 13.2.8
- **Why it is a defect:** 016-179's own sentence says 'binding machinery is spelled as lowercase kinds' yet in the same sentence writes the carrier as `Portal[Cell[T]]` (bracket, capital C). SPEC writes it `Portal[cell T]` (lowercase kind inside the Portal). The entry contradicts itself and diverges from the cited SPEC — this is the seed-question adjacency form (Portal[Cell[T]]) re-admitting the bracketed cell spelling the entry itself forbids.
- **Direction of change:** Make 016-179 write the carrier consistently with its own 'lowercase kinds' rule and with SPEC (`Portal[cell T]`), pending the user's regime decision.
- **Evidence check:** pass — 016-179 writes carrier as Portal[Cell[T]] while SPEC §13.2.8.1 table gives Portal[cell T].
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2032-2032`
    > 016-179. The storability razor: storable values keep bracket types (`Type[C]`, `Handle[T]`, `WeakHandle[T]`, `Portal[T]`), while binding machinery is spelled as lowercase kinds; a cell is refused as a value, and `Portal[Cell[T]]` is the sanctioned identity-as-data form. (§13.2.8)
  - `packages/ductus-lang/docs/SPEC.md:12512-12512`
    > | Storable — non-graph slot | `Portal[T]` | identity-as-data; `Portal[cell T]` is the sanctioned cell-identity carrier |

#### F150 — 016-165 and 016-283 use bracketed Stream[T]/Signal[T]/Derived[T]/Recurrent[T,N], but SPEC §13.2.8 uses lowercase `stream T` and denies any bracket Cell/Stream type family.

- **Severity/Category/Verdict:** LOW / divergence / CONFIRMED
- **Anchors:** LOG 016-165, 016-283 · SPEC § 13.2.8
- **Why it is a defect:** 016-283 spells the excluded event type as `Stream[T]`; SPEC §13.2.8 (L12417-12418) spells it `stream T`. 016-165 spells the value-cell family as `Signal[T]`/`Derived[T]`/`Recurrent[T, N]`; SPEC uses `signal T`/`derived T`/`recurrent[N] T`. Same bracket-vs-kind divergence, definitional restatement severity.
- **Direction of change:** Align stream/value-cell spelling in these entries with the chosen regime and with SPEC §13.2.8.
- **Evidence check:** pass — 016-165/283 use bracketed Stream[T]/Signal[T]/etc but SPEC §13.2.8 uses lowercase 'stream T' kind.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2018-2018`
    > 016-165. Streams are reactive cells but are not value cells (not `Signal[T]`/`Derived[T]`/`Recurrent[T, N]`). (§13.2.8)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2136-2136`
    > 016-283. Value-reading operator and function parameters are typed `Cell[T]` (the umbrella); a `Stream[T]` is excluded at the read site rather than by the signature. (§13.2.8)
  - `packages/ductus-lang/docs/SPEC.md:12418-12418`
    >   excluded at the read site, not by the annotation. A `stream T` has no current value, so it is

#### F149 — 016-279/280/281/282 name the produced construct as bracket types Derived[T]/Recurrent[T,N], but the cited SPEC §13.2.8 names them lowercase kinds `derived T`/`recurrent[N] T`.

- **Severity/Category/Verdict:** LOW / divergence / CONFIRMED
- **Anchors:** LOG 016-279, 016-280, 016-281, 016-282 · SPEC § 13.2.8
- **Why it is a defect:** 016-282 says the produced construct is `Derived[T]`; the cited SPEC line (12406-12407) says it 'always produces a `derived T`'. 016-281 says `Derived[T]` is the zero-history case of `Recurrent[T, N]`; SPEC 12403 states this as `derived T`. Bracket-type vs kind spelling divergence on the same claims. Lower severity than the parameter/return findings because these are definitional restatements, but SPEC must still conform to LOG (or the reverse decided).
- **Direction of change:** Align the produced-construct spelling in these entries with the chosen regime and with SPEC §13.2.8.
- **Evidence check:** pass — 016-279..282 name Derived[T]/Recurrent[T,N] but cited SPEC §13.2.8 names lowercase kinds derived T/recurrent[N] T.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2132-2132`
    > 016-279. `Derived[T]` is the type produced by a `derived` declaration: a reactive value computation with no self-history. (§13.2.8)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2135-2135`
    > 016-282. A reactive value expression combining one or more reactive cells always produces a `Derived[T]`. (§13.2.8)
  - `packages/ductus-lang/docs/SPEC.md:12403-12403`
    >   `derived T` is the degenerate zero-history case (`N = 0`).
  - `packages/ductus-lang/docs/SPEC.md:12406-12406`
    > A reactive value expression combining one or more reactive cells always

### yielded/fold: no lowering, no dirty route, undefined imports

The two new constructs (sections 034/035) have no IR representation, no dirty-set/DAG scheduling, a repeat-source rule that rejects them, and load-bearing imported terms (on/off bits, walk order, runtime-loop context) defined nowhere. Includes the IR type-vocabulary contradictions in 033 they expose.

#### F114 — The fold's value during the commit in which a gated arm deactivates is jointly under-determined: membership-leave dirties consumers this commit, yet gate-close is defined to add no in-commit recompute.

- **Severity/Category/Verdict:** HIGH / contradiction / CONFIRMED
- **Anchors:** LOG 034-10, 035-5, 022-59, 022-62 · SPEC § 13.20.4, 13.21.5, 13.9.8, 13.9.7
- **Why it is a defect:** Witness: an operator holds `collect as parts:` with three `when`-gated arms each `yield`ing one activation-driven member (034-3), consumed by `fold parts: by: (x,y)=>x+y  else: 0.0` (035-8 forces block form). At a commit, arm b's gate predicate (a derived bool, 022-12) flips true->false. Two rules give different concrete fold values THIS commit, and both are mandatory. Path A (034-10, 033-82, 035-5): a member LEAVE 'propagates dirt to consumers just as a value change does' and is a costed 'leave' event, so the fold-kind cell is in the dirty set and recomputes this commit WITHOUT the departed member — fold value = a+c. Path B (SPEC 13.9.8 'Gate-close adds no DAG work'; 022-59 makes only gate-OPEN an in-commit event and explicitly contrasts close): gate-close schedules no recompute this commit, so the fold does not re-run and still reflects the member — fold value = a+b+c (and would only drop next commit, if ever, since nothing else re-triggers it). The membership-driver of the leave is precisely the gate that 13.9.8 says produces no DAG work, so the leave-dirties-consumers rule and the gate-close-no-work rule cannot both hold for this witness. An implementer cannot satisfy both; the commit-of-departure fold value is unspecified between two named answers.
- **Direction of change:** Decide and state whether a gate-guarded member LEAVE is a first-class dirty root that recomputes its fold within the deactivating commit (making gate-close symmetric with the gate-open snap for fold consumers), or whether the fold reflects the departed member until the next commit; then reconcile 034-10/033-82/035-5 with the SPEC 13.9.8 'gate-close adds no DAG work' clause and the 022-59 open/close asymmetry so a single answer for the deactivating-commit fold value is derivable.
- **Evidence check:** pass — For a fold over gate-gated yields, at the commit a gate flips true→false: 034-10/035-5 make the member-leave dirty consumers and recompute the fold this commit (value=a+c), while SPEC 13.9.8 'Gate-close adds no DAG work' + 022-59 (only gate-open is an in-commit event) leave the fold un-recomputed (value=a+b+c). Both are mandatory; commit-of-departure value is unspecified between two named answers.
- **Charity check:** sustain — Confirmed jointly unsatisfiable. On the commit where a gated arm's predicate flips true→false, two mandatory rules give different concrete fold values. Path A: 034-10 (LOG 4361) 'a change in membership ... propagates dirt to its consumers just as a value change does' + 033-82 + 035-5 (LOG 4372, O(log n) per 'leave') → the fold-kind cell recomputes THIS commit without the departed member. Path B: SPEC 17833-17834 'Gate-close adds no DAG work' + 022-59 (LOG 2943, only gate-OPEN is scheduled in the same commit, explicitly contrasting close) → no recompute this commit, fold still reflects the member. The leave's membership-driver is precisely the gate-close that §13.9.8 says produces no DAG work, so the leave-dirties-consumers rule and the gate-close-no-work rule cannot both hold for this witness. SPEC 17805-17816 (activation-driven fold membership: 'present iff its arm is effectively active') defines the toggle but does not resolve the timing. Commit-of-departure fold value is unspecified between two named answers; no dissolving text found.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:4361-4361`
    > 034-10. `yielded` is a distinct cell-KIND, and a change in membership of a `yielded` group propagates dirt to its consumers just as a value change does. (§13.20)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4372-4372`
    > 035-5. The fold cost is normative, in the family of the loop and collection cost rules: O(log n) combines per member value-change, join, or leave, over a deterministic combine tree fixed by member order. (§13.21)
  - `packages/ductus-lang/docs/SPEC.md:17832-17834`
    > the gate-open transition is itself a propagation event scheduled within
    > the commit that flips the predicate (the open snap, §13.9.7). Gate-close
    > adds no DAG work; its only consequence is that the runtime fires `suspend`
  - `packages/ductus-lang/docs/DECISION_LOG.md:2943-2943`
    > 022-59. The gate-open re-evaluation snap is scheduled within the **same commit** that flips the predicate false→true — not the next commit; this contrasts with a reload predicate (next commit) and a connection re-point (next commit). (§13.9.7)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4194-4194`
    > 033-82. A `fold`-kind cell's surface is unchanged: its result lives in a `derived` declaration and consumers see `derived T`; membership changes on its member edges propagate dirt exactly as member value changes do. (§15.4.1)

#### F123 — A fold-kind cell fed by repeat-driven members has no specified route into the per-commit dirty set/DAG, so a same-commit membership change may never recompute the fold.

- **Severity/Category/Verdict:** HIGH / gap / CONFIRMED
- **Anchors:** LOG 034-10, 035-9, 035-10, 023-14, 023-10 · SPEC § 13.20.4, 13.21.7, 13.10.2, 13.11.3, 15.4.1
- **Why it is a defect:** Scenario: inserting two keys into `items` dirties the signal. The `count = items.count` derived is a plain derived transitively dependent on `items`, so 023-10 marks it dirty and it updates. The fold consuming a `collect{ repeat (k,v) in items: yield v }` gains two `keyed-template` members (034-3, SPEC 13.20.3). 034-10 asserts this membership change 'propagates dirt to its consumers,' but the commit dirty-set (023-8/9/10) and the per-commit DAG node set (023-14, SPEC 18140-18143) are defined ONLY over writable roots, derived cells, and recurrents. A fold-kind cell is none of these (035-9). The IR provides exactly one dirty-propagation edge list — 'Derived dependency edges … used by the runtime for dirty-set propagation' (SPEC 24086-88) — plus a recurrent one; there is NO fold-dependency-edge list and no rule feeding the fold_payload member edges (SPEC 24060-69) into step-1 dirty computation. So an implementer has no legal mechanism to make the fold recompute in the same commit the membership changed. This is an implementer-blocking gap with no standard-library or implementation-defined boundary offered (per 001-6). The count-derived's value-change story and the fold's membership-change story therefore do NOT provably converge in one commit: one is fully specified, the other dead-ends.
- **Direction of change:** Specify how a yielded/membership change and the resulting fold-kind cell enter the per-commit dirty set and DAG — either add a fold-dependency-edge list analogous to the derived/recurrent lists and name what dirties a fold (source-signal dirt, scope mount/unmount, or member value-dirt), or state that a fold-kind cell is treated as a derived for dirty-propagation and topological ordering.
- **Evidence check:** pass — A fold-kind cell fed by repeat-driven (keyed-template) members has no specified route into the per-commit dirty set/DAG: 023-14 + SPEC 18140-43 restrict DAG nodes to deriveds/recurrents, and the only dirty-propagation edge lists (SPEC 24086-91) are derived/recurrent, with no fold-member-edge feed — so 034-10's same-commit membership dirt has no legal mechanism, no 001-6 boundary offered.
- **Charity check:** sustain — Confirmed with the IR structure fresh-read. The IR provides exactly two dirty-propagation edge lists: 'Derived dependency edges' (SPEC 24086-24088, used by the runtime for dirty-set propagation) and 'Recurrent dependency edges' (SPEC 24090). There is NO fold-dependency-edge list; the fold's members live in `fold_payload` (SPEC 24060-24069) with no rule feeding those member edges into step-1 dirty computation (SPEC 18127-18139). The per-commit DAG node set (023-14, SPEC 18140-18148) is defined only over writable roots, deriveds, and recurrents; a fold-kind cell (035-9) is none of these. So a fold fed by repeat-driven (keyed-template) members that gains members in a commit has no specified route into the dirty set/DAG, and no legal mechanism makes it recompute the same commit. The count-derived's value-change story is fully specified; the fold's membership-change story dead-ends. Implementer-blocking gap, no 001-6 boundary. (Same underlying hole as F246/F114 viewed via repeat-driven membership; independently valid.)
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:4361-4361`
    > 034-10. `yielded` is a distinct cell-KIND, and a change in membership of a `yielded` group propagates dirt to its consumers just as a value change does. (§13.20)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4376-4376`
    > 035-9. IR lowering of a fold introduces a new cell kind `fold`, extending the cell kind enum to `input | derived | recurrent | fold`, while the count of six graph primitives is unchanged. (§13.21)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3022-3022`
    > 023-14. The per-commit DAG's nodes are the dirty derived expressions plus each recurrent whose expression became dirty this commit. (§13.10.2)
  - `packages/ductus-lang/docs/SPEC.md:24086-24088`
    > **Derived dependency edges.** A list of `(derived_cell_id,
    > [input_cell_ids])` pairs. Used by the runtime for dirty-set
    > propagation and topological evaluation ordering.
  - `packages/ductus-lang/docs/SPEC.md:18140-18143`
    > 2. **Compute evaluation order.** Topologically sort the per-commit
    >    DAG (§13.11.3). Nodes in the DAG are:
    >     - Dirty derived expressions.
    >     - Each recurrent whose expression became dirty this commit.

#### F093 — A standalone `yielded` group (consumed by `repeat` or a yielded-typed parameter, not by `fold`) has no stated lowering to any of the six graph primitives and no IR type.

- **Severity/Category/Verdict:** HIGH / gap / CONFIRMED
- **Anchors:** LOG 033-58, 033-59, 033-80, 034-9, 034-11, 035-9 · SPEC § 15.4.1, 13.20, 13.21
- **Why it is a defect:** 016-1 and 034-10 make `yielded` one of seven surface cell-KINDs. 033-58 claims ALL surface reactive constructs desugar into the six primitives, but 033-59 lowers only signal/attr/derived/recurrent/const to `cell` and omits `yielded`; the IR cell-kind enum (033-80, 035-9) is exactly `input|derived|recurrent|fold` with no `yielded` kind. The ONLY IR home for yielded membership is member edges absorbed inside a `fold`-kind cell's payload (035-10). But 034-11 lists two other legal consumers — `repeat` over the group and yielded-typed operator/function parameters — where the group is nameable (`collect as x`, 034-1) and passed as a value. No rule says what IR object those standalone yielded groups lower to, and 033-43's IR type vocabulary contains no `yielded T` type, so a `yielded T` parameter has no expressible ABI signature. 034-9's 'compile-time wire set plus runtime on/off bits' is mechanism, not a mapping to one of the six primitives, and is not marked as a 001-6 legal boundary (stdlib-delegation or implementation-defined). Per the rubric this is an implementer-blocking gap with no legal boundary: a compiler cannot emit IR for `repeat p in some_yielded_group` or for a function taking `yielded T`.
- **Direction of change:** State the IR lowering for a standalone (non-fold-consumed) yielded group — either add an explicit IR representation/type for it or reduce every consumer (repeat, yielded-typed param) to an already-specified primitive/type — or mark it an explicit 001-6 boundary.
- **Evidence check:** pass — Standalone `yielded` group (via repeat / yielded-typed param) has no stated lowering to any of the six primitives and no IR type; 034-9's 'wire set plus on/off bits' is mechanism, not a mapping to a primitive, and no 001-6 legal boundary is declared.
- **Charity check:** sustain — Corpus-wide grep confirms no rule lowers a standalone `yielded` group (consumed by `repeat` or a yielded-typed parameter) to any of the six primitives, and no IR type exists for `yielded T`. 033-59 (LOG 4171) omits `yielded` from the cell-lowering list; the IR cell-kind enum is `input|derived|recurrent|fold` (SPEC 24512, 035-9) with no `yielded` kind; the `type_tag` grammar (SPEC 24529-24533) has no `yielded T` production, so a `yielded T` parameter has no ABI signature. The only `yielded`-lowering text is 034-9's 'compile-time wire set plus runtime on/off bits' — mechanism, not one of the six primitives, and not marked as a 001-6 legal boundary. SPEC 20535 ('yielded T groups likewise participate as cell kinds') is surface type-annotation umbrella framing (§13.2.8 kind), NOT an IR lowering, and does not conflict with the finding. The fold consumer routes through fold_payload member edges, but 034-11's other two consumers (repeat-over-group, yielded-typed params) have no IR home. Charitable search produced no dissolving normative text; implementer-blocking gap with no legal boundary.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:4170-4171`
    > 033-58. All surface reactive constructs desugar into the six graph primitives. (§15.4.1)
    > 033-59. Surface `signal`, `attr`, `derived`, `recurrent`, and `const` all lower to `cell`. (§15.4.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4192-4192`
    > 033-80. A cell ent
ry carries a `kind` — `input` (a stored, externally-written cell), `derived`, `recurrent`, or `fold` — classifying it; the kind leads the cell's text-form declaration. (§15.4.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4360-4362`
    > 034-9. A `yielded` group stores nothing: it is realized as a compile-time wire set plus runtime on/off bits, reusing the gate on/off-bit machinery, and holds no backing collection of its own. (§13.20)
    > 034-10. `yielded` is a distinct cell-KIND, and a change in membership of a `yielded` group propagates dirt to its consumers just as a value change does. (§13.20)
    > 034-11. A `yielded` group is consumed only by the fold form, by yielded-typed parameters of both operators and functions, and by `repeat` over it; it supports no indexing, and a structural compile-time `for` over a `yielded` group is a compile error. (§13.20)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4376-4377`
    > 035-9. IR lowering of a fold introduces a new cell kind `fold`, extending the cell kind enum to `input | derived | recurrent | fold`, while the count of six graph primitives is unchanged. (§13.21)
    > 035-10. A fold-kind cell carries a combiner behavior id, an else value, and member edges in member order, each edge tagged with its membership driver — permanent, keyed-template, or gate-guarded — while its surface remains a `derived T` declaration seen by consumers. (§13.21)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4155-4155`
    > 033-43. The IR type vocabulary is the §4.1 primitives plus `str`, tuples `(T,…)`, arrays `[T;N]`, record/enum `%TypeId`s, `pool_index<%PoolId>`, and `closure<(T,…)->R>`. (§15.4)

#### F051 — A `yielded` group `stores nothing` (034-9), yet ch12's default-form `Iterable` dispatch requires a Source threaded on every `next` and roots each item in the source's cluster; there is no backing collection to be the Source or the cluster root.

- **Severity/Category/Verdict:** MED / logical_flaw / CONFIRMED
- **Anchors:** LOG 034-12, 034-9, 014-28, 014-29, 014-121 · SPEC § 13.20.4.2, 12.3.1, 12.7, 12.8
- **Why it is a defect:** ch12's `Iterable`/`Iterator` machinery offers exactly two shapes: self-contained (`Source = ()`, all state internal) or source-bearing (`Source = TheCollection`, cursor reads through a backing collection, items rooted in that collection's cluster, §12.7, 014-29). A `yielded` group has no backing collection (034-9), so it cannot be source-bearing; and its members are external cells whose values/membership change, so it is not self-contained state either. 034-12 asserts it fulfills `Iterable` without specifying which pattern the language-defined fulfillment uses, what `Iter.Source` is (the `where Iter.Source = Subject` constraint would bind Source to the `yielded` group, which stores nothing), or what cluster the borrow-equivalent `Item` (014-29) is rooted in. The contract's own required slots have no legal filling here.
- **Direction of change:** Specify the concrete `Iterator`/`Source` shape of the language-defined `yielded`-`Iterable` fulfillment: what `Iter.Source` is, whether items are Copy/borrow-equivalent and rooted in what, given 034-9's no-backing-collection stance. Reconcile with §12.7's two-pattern taxonomy or state this is a third, language-only pattern.
- **Evidence check:** pass — 034-9 says a yielded group stores nothing/has no backing collection, but ch12 Iterable dispatch requires a Source threaded on every next and roots each Item in the source's cluster (014-28/29, SPEC §12.7); neither of ch12's two Source shapes has a legal filling, and 034-12 does not resolve which is used.
- **Charity check:** sustain — ch12's Iterable machinery offers exactly two Source shapes: self-contained `type Source = ()` (all state internal) or source-bearing `type Source = TheCollection` (cursor reads through a backing collection) — SPEC §12.7 (SPEC 10737-10745), and the Iterable trait binds `where Iter.Source = Subject` (SPEC 10893-10896/10900-10903), i.e. Source is the iterable value itself. A yielded group 'stores nothing … holds no backing collection' (034-9, LOG 4360), so it cannot be source-bearing; its members are external membership-varying cells, so it is not self-contained internal state either. 014-29 (LOG 1668) roots each default-form Item 'in the source's cluster' — but a stores-nothing group has no cluster to root in. 034-12 asserts the language-defined fulfillment exists without specifying which pattern it uses or what Iter.Source/the item-root cluster is. Neither §13.20.4.2 (SPEC 23082-23090) nor the cited precedent §13.5.4.2 (Keyed/StringifiableKey are key-derivation traits, not Source-binding fulfillments) fills these slots. Sustained: the contract's required slots have no legal filling for a stores-nothing group, and no normative text supplies one. | No dissolving passage found. SPEC:10893-10896 (Iterable trait) 'fn iterator(value: Subject) -> Iter where Iter.Source = Subject' binds Source to the iterable value; SPEC:10741-10745 requires a source-bearing iterator to hold 'TheCollection … supplied on each `next` call.' LOG:4360 (034-9) 'A `yielded` group stores nothing … holds no backing collection of its own' leaves both Source shapes unfillable, and SPEC:23082-23090 (§13.20.4.2) does not state which pattern the language-defined fulfillment uses or what Iter.Source is.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:4360-4360`
    > 034-9. A `yielded` group stores nothing: it is realized as a compile-time wire set plus runtime on/off bits, reusing the gate on/off-bit machinery, and holds no backing collection of its own. (§13.20)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1667-1668`
    > 014-28. Each default-form iteration step calls `Iterator::next(iter, v)`, threading the source through every call and returning `(Option[Item], NewIter)`. (§12.3.1)
    > 014-29. A default-form yielded `Item` is borrow-equivalent rooted in the source's cluster. (§12.3.1)
  - `packages/ductus-lang/docs/SPEC.md:10741-10745`
    > - **Source-bearing iterators** (Vec, Map, user-defined
    >   collections) — the iterator holds only a cursor / position; the
    >   collection is the source, supplied on each `next` call. `type Source
    >   = TheCollection`; each `next` call receives the collection under
    >   borrow-equivalent convention and reads through it.
  - `packages/ductus-lang/docs/SPEC.md:10893-10896`
    > trait Iterable:
    >   type Iter: Iterator
    >   fn iterator(value: Subject) -> Iter
    >   where Iter.Source = Subject

#### F050 — 034-12 makes a membership-varying `yielded` group fulfill ch12 `Iterable`, but ch12 freezes the source (no mutation) and evaluates it once at loop entry; what a runtime `for` sees when a member joins/leaves mid-loop is unspecified.

- **Severity/Category/Verdict:** MED / gap / CONFIRMED
- **Anchors:** LOG 034-12, 034-10, 014-25, 014-55 · SPEC § 13.20.4.2, 12.8.2, 12.3.1, 13.20.4
- **Why it is a defect:** ch12's `Iterable` contract was authored for sources whose contents are frozen for the loop's duration: the source is evaluated once at entry (014-25), held live, and may not be moved or mutated while the iterator lives (§12.8.2). A `yielded` group is, by design, membership-varying at runtime (034-10). 034-12 imports the frozen-source `Iterable` contract into exactly the context ch12 excluded, and neither §12 nor §13.20.4.2 states what values a runtime `for` walking a `yielded` group observes when a member joins or leaves between two `next` calls. An implementer has no rule for the mid-iteration membership-change case. ch12 §12 (lines 9925-11342) never mentions membership-varying sources at all.
- **Direction of change:** Add a rule specifying the snapshot/live-membership semantics of a runtime `for` over a `yielded` group (e.g. membership is snapshotted at loop entry per 014-25, or explicitly live), and reconcile with the source-freeze invariant of §12.8.2.
- **Evidence check:** pass — 034-12 imports ch12's Iterable contract (source evaluated once at entry, no mutation while iterator live — 014-25, SPEC §12.8.2) onto a membership-varying yielded group (034-10), but neither ch12 nor §13.20.4.2 specifies what a runtime for observes when a member joins/leaves mid-iteration.
- **Charity check:** sustain — 034-12 (LOG 4363) imports the ch12 Iterable contract onto a membership-varying yielded group (034-10). ch12's contract is frozen-source: 014-25 (LOG 1664) 'source expression is evaluated exactly once, at loop entry'; SPEC §12.8.2 (SPEC 10920-10923) 'the cluster's iteration alias rooted in the source is live for the loop's duration. The source-mutation invariants of §11.9.2 apply … (no move, no mutation of the source while the iterator is live).' A yielded group is membership-varying at runtime by design (034-10, LOG 4361). Neither §12.8 nor §13.20.4.2 (SPEC 23082-23090, which only states Item=member-values-in-walk-order and runtime-loop-only availability) specifies what a runtime `for` observes when a member joins/leaves between two `next` calls. §13.5.4.2 (SPEC 15386-15425) operates on a committed snapshot and does not cover mid-iteration membership change of a yielded group. Sustained: no rule for the mid-loop membership-change case, and the imported frozen-source contract conflicts with the membership-varying design. | No dissolving passage found. SPEC:10920-10923 (§12.8.2) 'The source-mutation invariants of §11.9.2 apply for that same span (no move, no mutation of the source while the iterator is live)' is the frozen-source contract; SPEC:23082-23090 (§13.20.4.2) states only 'its `Item` is the `T` member values in walk order … available in runtime-loop contexts only' and never addresses a member joining/leaving mid-iteration. The frozen-source rule conflicts with 034-10's membership-varying design rather than resolving the finding.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:4363-4363`
    > 034-12. `yielded T` fulfills `Iterable` at the language level, authored as a language-defined fulfillment in the style of `Keyed` and `StringifiableKey`, with `Item` = the `T` member values in walk order, usable only in runtime-loop contexts such as function, derived, and operator bodies, and it is not a dynamic-view consumer. (§13.20)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4361-4361`
    > 034-10. `yielded` is a distinct cell-KIND, and a change in membership of a `yielded` group propagates dirt to its consumers just as a value change does. (§13.20)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1664-1664`
    > 014-25. The iteration source expression is evaluated exactly once, at loop entry. (§12.3.1)
  - `packages/ductus-lang/docs/SPEC.md:10921-10923`
    > the cluster's iteration alias rooted in the source is live for the
    > loop's duration. The source-mutation invariants of §11.9.2 apply for
    > that same span (no move, no mutation of the source while the iterator
    > is live).

#### F217 — 033-121 says effect parameter_bindings are 2-tuples (parameter_name, source_cell_id|value_literal), but 033-64 and the cited SPEC 15.4.1 require a third provenance-marker slot, so an implementer following 033-121 emits the wrong field shape.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 033-121, 033-64 · SPEC § 15.4.1
- **Why it is a defect:** LOG entries are atomic and self-contained (Invariant 2); an implementer reading 033-121 in isolation would encode parameter_bindings as 2-tuples with no provenance marker. Both a sibling LOG entry (033-64) and the SPEC section 033-121 itself cites (§15.4.1) define the field as a 3-tuple carrying the provenance marker. SPEC must conform to LOG and LOG entries must not contradict each other; here 033-121 is the outlier that contradicts both. Concrete behavior split: static-RHS placements must skip implicit-derived allocation and reactive-RHS must instantiate the bridge (033-236), which requires the marker be present in the field 033-121 describes.
- **Direction of change:** Reconcile 033-121 so its stated field shape includes the provenance marker consistent with 033-64 and SPEC §15.4.1; surface to the user rather than resolving unilaterally which of the two framings is canonical.
- **Evidence check:** pass — 033-121 defines effect parameter_bindings as 2-tuples, but sibling 033-64 and the cited SPEC §15.4.1 require a third provenance-marker slot; an implementer following the atomic, self-contained 033-121 in isolation emits the wrong field shape, and the marker is load-bearing for static-vs-reactive RHS handling.
- **Charity check:** sustain — Confirmed. 033-121 says parameter_bindings are '(parameter_name, source_cell_id | value_literal)' pairs (2-tuple). Its own cited SPEC §15.4.1 (24167-24176) defines them as '(parameter_name, source_cell_id | value_literal, provenance_marker)' triples, and sibling 033-64 says 'Each parameter_bindings slot carries a provenance marker'. 033-121 is the lone outlier contradicting both its cited SPEC and a sibling LOG entry. The provenance marker is load-bearing (033-236: static-RHS skips implicit-derived allocation, reactive-RHS instantiates the bridge). An implementer following 033-121 in isolation emits the wrong field shape (2-tuple, no marker). Dissolving-text hunt found only text that confirms the 3-tuple. Sustained MED divergence.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:4233-4233`
    > 033-121. An effect entry's `parameter_bindings` are `(parameter_name, source_cell_id | value_literal)` pairs. (§15.4.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4176-4176`
    > Each `parameter_bindings` slot carries a provenance marker recording whether its argument is a bare cell, a static expression wrapped as a degenerate constant cell, or a reactive expression bridged through an implicit derived; the marker is a tag on the existing slot, not a third binding case.
  - `packages/ductus-lang/docs/SPEC.md:24167-24176`
    > - `parameter_bindings`: list of `(parameter_name, source_cell_id |
    >   value_literal, provenance_marker)` triples. The `provenance_marker`
    >   records how the cell argument was materialized under the uniform
    >   cell-argument rule (§13.17.3): `bound` (a bare cell bound directly),
    >   `constant_wrap` (a static expression wrapped as a degenerate constant
    >   cell), or `bridge` (a reactive expression whose implicit `derived`
    >   bridge cell — synthesized at the call site — is the `source_cell_id`).
    >   The marker is metadata on the existing binding, **not** a third binding
    >   case: `source_cell_id | value_literal` still carries the binding
    >   itself; the marker only annotates its origin.

#### F248 — 033-169's kind-led text-form rule enumerates only input/derived/recurrent, omitting the `fold` kind that 033-80 defines and SPEC §15.4.6 grammar includes.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 033-169, 033-80 · SPEC § 15.4.6, 15.4.1
- **Why it is a defect:** 033-169 is the LOG rule for the kind-led IR cell text form and lists exactly `input`/`derived`/`recurrent`. But 033-80 (same section) defines the kind enum as four values including `fold`, and SPEC §15.4.6 — the section 033-169 cites — spells the grammar as `cell_kind ::= 'input' | 'derived' | 'recurrent' | 'fold'` (line 24512) and the prose lists all four (24540-24544). So 033-169 is internally inconsistent with 033-80 and diverges from the SPEC it points at. Per the project's edit protocol the LOG is the decision-of-record and SPEC must conform; here the LOG rule (033-169) is the stale/incomplete side, omitting `fold`. Divergence between LOG and SPEC is defined as a defect.
- **Direction of change:** Bring 033-169 into line with 033-80 and SPEC §15.4.6 by including the `fold` kind (and its combiner/else/members serialization) in the kind-led text-form enumeration; confirm the LOG-first direction with user before editing.
- **Evidence check:** pass — 033-169's kind-led cell text-form rule enumerates only input/derived/recurrent, omitting the fold kind that sibling 033-80 defines and the cited SPEC §15.4.6 grammar and prose include — an internal LOG inconsistency plus LOG-SPEC divergence.
- **Charity check:** sustain — Confirmed. 033-169 enumerates the kind-led cell text form as 'input/derived/recurrent' — three kinds. Its cited SPEC §15.4.6 grammar (line 24512) spells cell_kind ::= 'input' | 'derived' | 'recurrent' | 'fold' and the prose (24540-24549) lists all four and elaborates the fold cell's serialization. Sibling 033-80 defines the kind enum as four values including fold. So 033-169 is stale/incomplete, diverging from both its cited SPEC and sibling 033-80. LEARNINGS-5/edit-protocol: LOG is decision-of-record and SPEC conforms, so 033-169 (LOG) is the side needing conformance — but divergence itself is the defect. No text reconciles the omission. Sustained MED divergence.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:4281-4281`
    > 033-169. A graph cell's text-form line is kind-led — `input`/`derived`/`recurrent <path> : <type>` — parallel to `behavior`/`gate`/`effect`; a derived or recurrent cell renders `uses BID` and `inputs [...]` (a recurrent adds `depth`) inline, serializing its behavior-table association and dependency edges, which remain the runtime's propagation structure. (§15.4.6)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4192-4192`
    > 033-80. A cell entry carries a `kind` — `input` (a stored, externally-written cell), `derived`, `recurrent`, or `fold` — classifying it; the kind leads the cell's text-form declaration. (§15.4.1)
  - `packages/ductus-lang/docs/SPEC.md:24512-24512`
    > cell_kind     ::= 'input' | 'derived' | 'recurrent' | 'fold'
  - `packages/ductus-lang/docs/SPEC.md:24540-24544`
    > `type_tag` does not repeat them. A cell is **kind-led** — `input` (a stored
    > attr/signal), `derived`, `recurrent`, or `fold` leads the line, parallel to
    > `behavior`/`gate`/`effect`; a derived or recurrent cell's `uses` names its
    > behavior handle and `inputs` its input cells, `recurrent` adds `depth`, and a
    > stored or recurrent cell an `init`. A `fold` cell (the lowering of the

#### F247 — 033 makes `fold` a distinct cell kind, but 023-14's per-commit DAG node enumeration lists only dirty deriveds and recurrents, leaving no slot for a fold cell to be scheduled.

- **Severity/Category/Verdict:** MED / gap / CONFIRMED
- **Anchors:** LOG 033-80, 023-14, 023-13 · SPEC § 15.4.1, 13.10.2
- **Why it is a defect:** 033-80 defines four cell kinds, with `fold` distinct from `derived` and `recurrent`. 023-14 enumerates the per-commit DAG's nodes as exactly "dirty derived expressions plus each recurrent whose expression became dirty." A `fold`-kind cell is neither, so 023-14 places it nowhere in the evaluation-order DAG, yet 033-82 requires the fold's result to be recomputed (it "propagates dirt"). 033-82's escape hatch — the fold's "result lives in a `derived` declaration" — collides with 033-80, which explicitly gives the IR cell a `fold` kind separate from `derived`. So at the surface a fold reads as a derived (would fit 023-14), but at the IR level it is a distinct kind the runtime must schedule (does not fit 023-14). An implementer cannot tell from 023 whether to treat a fold as a derived DAG node or as an unlisted new node type. Two careful readings yield different commit-loop scheduling.
- **Direction of change:** Decide whether fold cells are scheduled as derived DAG nodes (align 033-80's surface claim) or need an explicit fold-node clause in 023-14; reconcile 023's DAG-node enumeration with 033-80's four-kind enum. Surface to user.
- **Evidence check:** pass — fold is a distinct IR cell kind (033-80) but 023-14's per-commit DAG node set is only dirty deriveds + recurrents, giving a fold cell no scheduling slot; 033-82's 'result lives in a derived' collides with 033-80's separate fold kind.
- **Charity check:** sustain — Confirmed unresolved. 033-80 (LOG 4192) and SPEC 24512 define `fold` as a cell kind distinct from `derived` and `recurrent`. 023-14 (LOG 3022) and SPEC 18140-18148 enumerate the per-commit DAG's nodes as exactly dirty derived expressions plus triggered recurrents — a fold-kind cell is neither, so it is placed nowhere, yet 033-82 requires the fold to recompute on membership change. 033-82's escape hatch ('its result lives in a derived declaration') collides with 033-80's explicit `fold` kind separate from `derived`. Charitable search of §13.10.2 (SPEC 18122-18189) and the fold IR section (SPEC 23183-23195, 24044-24069) found no rule reconciling whether the runtime schedules a fold as a derived DAG node or an unlisted node type. Two careful readings yield different commit-loop scheduling. Gap with no covering normative text or legal boundary.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:4192-4192`
    > 033-80. A cell entry carries a `kind` — `input` (a stored, externally-written cell), `derived`, `recurrent`, or `fold` — classifying it; the kind leads the cell's text-form declaration. (§15.4.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3022-3022`
    > 023-14. The per-commit DAG's nodes are the dirty derived expressions plus each recurrent whose expression became dirty this commit. (§13.10.2)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3021-3021`
    > 023-13. Commit step 2 computes evaluation order by topologically sorting the per-commit DAG. (§13.10.2)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4194-4194`
    > 033-82. A `fold`-kind cell's surface is unchanged: its result lives in a `derived` declaration and consumers see `derived T`; membership changes on its member edges propagate dirt exactly as member value changes do. (§15.4.1)

#### F246 — IR mandates fold-cell membership changes propagate dirt, but section 023's dirty-set algorithm has no rule that produces dirt from a membership (edge-set) change.

- **Severity/Category/Verdict:** MED / gap / CONFIRMED
- **Anchors:** LOG 033-82, 033-81, 023-8, 023-9, 023-10, 023-11, 023-19 · SPEC § 15.4.1, 13.10.2, 13.9
- **Why it is a defect:** 033-82 asserts a dirty-propagation behavior for fold cells in IR vocabulary ("membership changes propagate dirt"). Section 023 is the authoritative commit/dirty algorithm. Its dirty-production rules (023-8: writable cell staged-vs-committed value differs; 023-9: reactive expression re-evaluates when a referenced cell is dirty, operationalized as value change; 023-10/-11: propagation to transitively-dependent deriveds and triggered recurrents) all key off a VALUE change of a referenced cell. A fold member's membership change is not a value change of a referenced cell — it is a change in the SET of contributing edges. For a `gate-guarded` member (033-81), the member joins/leaves on a gate transition; 023-19 treats a gate-false edge only as non-propagation, never as a dirty trigger. Section 023's model (which SPEC §13.10.2 mirrors, lines 18127-18139, with no mention of fold or membership) therefore has no step that marks a fold cell dirty when a member joins or leaves. An implementer building the commit loop from 023 cannot satisfy 033-82. The two vocabularies (023 'dirty from value change' vs 033 'membership propagates dirt') hide the same subject and do not compose.
- **Direction of change:** Decide whether 023's dirty rules should gain an explicit membership-change dirty trigger for fold cells (paralleling 023-53's membership-cell dependency edge for dynamic views), or whether 033-82 overreaches; surface to user — do not resolve unilaterally.
- **Evidence check:** pass — IR (033-82) mandates fold membership changes propagate dirt, but section 023's dirty-production rules all derive from a referenced cell's value change; no rule produces dirt from a membership/edge-set change, so an implementer building the commit loop from 023 cannot satisfy 033-82.
- **Charity check:** sustain — Confirmed. 033-82 (LOG 4194) asserts fold membership changes 'propagate dirt exactly as member value changes do,' but section 023's dirty-production rules all key off a referenced cell's VALUE change: 023-8 (writable staged≠committed), 023-9 (value-change operationalized as current≠previous), 023-10/11 (propagation to dependent deriveds / triggered recurrents). A membership (edge-set) change is not a value change of a referenced cell. 023-19 (LOG 3027) treats a gate-false edge only as non-propagation, never a dirty trigger. Fresh read of SPEC §13.10.2 (18127-18148) confirms no fold or membership step in the dirty-set computation. An implementer building the commit loop from 023 cannot satisfy 033-82. Gap with no covering text; the '023 dirty-from-value-change' and '033 membership-propagates-dirt' vocabularies do not compose.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:4194-4194`
    > 033-82. A `fold`-kind cell's surface is unchanged: its result lives in a `derived` declaration and consumers see `derived T`; membership changes on its member edges propagate dirt exactly as member value changes do. (§15.4.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4193-4193`
    > 033-81. A `fold`-kind cell entry carries a combiner behavior id (a `fn(T, T) -> T` handle), an else value (the empty-membership result), and member edges listed in member order; each member edge is tagged with its membership driver — `permanent`, `keyed-template`, or `gate-guarded`. (§15.4.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3017-3018`
    > 023-9. A reactive expression (derived, recurrent, or `observe` arm trigger set) re-evaluates when a referenced cell is dirty; value-change semantics are operationalized as current-commit value ≠ previous-commit value. (§13.10.2)
    > 023-10. Dirty propagation extends to all derived cells transitively dependent on dirty roots. (§13.10.2)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3027-3027`
    > 023-19. A dependency edge whose gate predicate evaluates false does not propagate to destination outputs. (§13.10.2)

#### F240 — 033-43 states the IR type vocabulary as a closed enumeration, but 033-47 and 033-48 each add to it ('join the IR type vocabulary') and Handle[T] is used yet never enumerated.

- **Severity/Category/Verdict:** MED / contradiction / CONFIRMED
- **Anchors:** LOG 033-43, 033-47, 033-48, 033-49, 033-50 · SPEC § 15.4
- **Why it is a defect:** 033-43 uses the closed copula 'The IR type vocabulary IS [list]', naming exactly seven families. 033-47 and 033-48 then say slice and Portal 'join the IR type vocabulary', contradicting the closed form. Furthermore 033-48 and 033-50 reference `Handle[T]`'s representation as if it is a vocabulary member, but Handle[T] appears in no enumerating entry at all. An implementer building an IR type-tag parser from 033-43 alone rejects Portal, slice, Handle, Map, and Bundle tags; the vocabulary is de facto open while asserted closed.
- **Direction of change:** Decide whether 033-43 is the single closed enumeration (and fold slice/Portal/Handle/Map/Bundle into it) or an open base extended by later entries, then make the wording consistent; also enumerate Handle[T] somewhere. Surface as a question rather than picking.
- **Evidence check:** pass — 033-43 states the IR type vocabulary with a closed copula naming exactly seven families, but 033-47 and 033-48 each add to it ('join the IR type vocabulary') and Handle[T] is referenced as a member yet never enumerated; a type-tag parser built from 033-43 alone rejects Portal, slice, Handle, Map, and Bundle tags while the vocabulary is de facto open.
- **Charity check:** sustain — Confirmed. 033-43 uses a closed copula — 'The IR type vocabulary IS the §4.1 primitives plus str, tuples, arrays, %TypeIds, pool_index, and closure' — naming exactly seven families and matching the SPEC §15.4 type_tag grammar (24529-24533), which also omits slice/Portal/Handle/Map/Bundle tags. 033-47 ('Slice types ... join the IR type vocabulary') and 033-48 ('Portal[T] values join the IR type vocabulary') additively extend it — a closed set cannot have members join it, a contradiction an implementer cannot satisfy both sides of. Additionally 033-48 and 033-50 reference Handle[T]'s representation as a vocabulary member, yet grepping all 033-* entries and the §15.4.6 grammar shows Handle[T] is never enumerated as an IR type — a used-but-undefined gap with no covering boundary (001-6). No charitable scoping ('033-43 = cell-storable core') holds, because 033-47 explicitly calls slices non-cell-storable borrow types that nonetheless 'join the IR type vocabulary'. An implementer building a type_tag parser from 033-43 alone rejects valid IR. Sustained HIGH.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:4155-4155`
    > 033-43. The IR type vocabulary is the §4.1 primitives plus `str`, tuples `(T,…)`, arrays `[T;N]`, record/enum `%TypeId`s, `pool_index<%PoolId>`, and `closure<(T,…)->R>`. (§15.4)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4159-4159`
    > 033-47. Slice types `T[..N]` (compile-time-known length) and `T[..]` (runtime length) join the IR type vocabulary as borrow types — represented at the ABI as `(pointer, length)` pairs — used in behavior parameter and return positions only; as borrow-equivalent aliases containing borrows, slices are not cell-storable. (§15.4.1, §15.4.4)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4160-4160`
    > 033-48. `Portal[T]` values join the IR type vocabulary as `(slot_path, generation)` pairs, paralleling `Handle[T]`'s representation. Portal-typed cells are `Copy` and stored inline; resolution compares the stamp in the portal slot table, and a mismatched stamp resolves to `None`. (§15.4.1, §15.4.4)

#### F239 — 033-79 limits a cell's type tag to §4.1 primitives plus the two pool-index types, but 033-48 mandates inline (non-pooled, non-primitive) Portal cells and 033-43 admits inline array/tuple cells with no valid tag under 033-79.

- **Severity/Category/Verdict:** MED / contradiction / CONFIRMED
- **Anchors:** LOG 033-79, 033-48, 033-83, 033-43 · SPEC § 15.4.1
- **Why it is a defect:** 033-79 is the authoritative statement of a cell entry's `type` field and enumerates exactly three tag families (§4.1 primitives, string-pool-index, dynamic-pool-index). A Portal-typed cell is stored inline as a `(slot_path, generation)` pair per 033-48 — not a §4.1 primitive and not a pool index — so it has no legal `type` tag under 033-79. Likewise 033-83 pools only record/enum/tuple; an array-typed cell (`[T;N]`, admitted by the vocabulary in 033-43) is neither a primitive nor pooled, so it too has no admissible tag. An implementer serializing a cell entry cannot satisfy both 033-79 and 033-48/033-43 for Portal or array cells.
- **Direction of change:** Reconcile the cell `type`-tag enumeration in 033-79 with the storable-cell types actually required by 033-48 (Portal), 033-83, and the vocabulary in 033-43, so every cell-storable type has exactly one admissible tag; do not resolve unilaterally — surface which entry is authoritative.
- **Evidence check:** pass — 033-79 restricts a cell's type tag to primitives + two pool-index types, but 033-48 mandates inline non-primitive/non-pool Portal cells and 033-43+033-83 leave array cells with no admissible tag.
- **Charity check:** sustain — Sustained on the Portal arm, strengthened. Charitable search for a Portal/Handle cell type_tag found NONE: the authoritative IR grammar `type_tag` production (SPEC 24529-24533) is `PRIM | '%'NAME | 'pool_index'<'%'NAME> | tuple | array | closure`, where `PRIM ::= i8..usize|f32|f64|bool|str` — no `(slot_path,generation)`/Portal/Handle tag anywhere. Yet 033-48 (LOG 4160) and 017-206 (LOG 2347) mandate that Portal-typed values are `Copy`, stored inline in any cell. An implementer serializing a Portal cell entry has NO legal `type_tag`. Both 033-79 (primitives + two pool-index types) AND the SPEC grammar fail to admit Portal — the contradiction is unsatisfiable regardless of which cell-type authority is used. Secondary: the array arm is partially relieved because the SPEC grammar (24530) admits `[type_tag;INT]` array tags, but that grammar itself conflicts with 033-79's narrower enumeration; the array conflict does not dissolve the primary Portal contradiction, so overall verdict is sustain, not refute or refile.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:4191-4191`
    > 033-79. A cell entry's `type` is a primitive type tag per §4.1, extended with the string-pool-index and dynamic-pool-index types. (§15.4.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4160-4160`
    > 033-48. `Portal[T]` values join the IR type vocabulary as `(slot_path, generation)` pairs, paralleling `Handle[T]`'s representation. Portal-typed cells are `Copy` and stored inline; resolution compares the stamp in the portal slot table, and a mismatched stamp resolves to `None`. (§15.4.1, §15.4.4)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4195-4195`
    > 033-83. An aggregate-valued graph cell — a record, enum, or tuple — is typed `pool_index<%TypeId>` (its layout living in the `types` table), never an inline `%TypeId`. (§15.4.1)

#### F213 — 034-10 declares `yielded` a distinct cell-KIND, but the IR cell-kind enum (033-80, 035-9) is input|derived|recurrent|fold with no `yielded` variant.

- **Severity/Category/Verdict:** MED / gap / CONFIRMED
- **Anchors:** LOG 034-10, 035-9 · SPEC § 13.20, 13.21, 15.4.1
- **Why it is a defect:** 034-10 asserts `yielded` is a distinct cell-KIND whose membership changes propagate dirt like value changes — i.e. it is a graph cell. But the authoritative IR cell-`kind` enum appears twice (033-80 steady-state and 035-9 after fold is added) and both enumerate exactly input|derived|recurrent|fold with no `yielded` member. An implementer building the IR from the LOG has no enum variant to encode a yielded-kind cell. SPEC 23046 patches this by calling yielded 'a member of the cell umbrella', but that umbrella note is not in the LOG and does not supply an enum tag. Either yielded is a real IR cell kind (then the enum must list it) or it is not (then 034-10's 'distinct cell-KIND' overclaims).
- **Direction of change:** Reconcile: decide whether `yielded` is an IR cell-kind enum member. If yes, extend the 033-80/035-9 enum to include it; if no, soften 034-10 so 'distinct cell-KIND' does not imply an IR enum variant that the enum decisions omit.
- **Evidence check:** pass — 034-10 calls yielded a distinct cell-KIND, but the two authoritative IR cell-kind enums (033-80, 035-9) are exactly input|derived|recurrent|fold with no yielded variant, leaving an implementer no enum tag to encode a yielded-kind cell.
- **Charity check:** sustain — The IR cell-`kind` enum is authoritatively `input | derived | recurrent | fold` in both 033-80 (LOG 4192) and after fold is added in 035-9 (LOG 4376), with no `yielded` tag. 034-10 (LOG 4361) calls `yielded` 'a distinct cell-KIND.' The charitable dissolvers I found do NOT settle it: (a) SPEC §13.18.5 'cell kind umbrella' (SPEC 20513-20535) and 016-1/016-62/016-178 establish `yielded T` as a member of the SURFACE lowercase-kind-annotation umbrella (signal/derived/recurrent/stream/yielded/cell T) — a different taxonomy than the IR encoding `kind` tag of 033-80; membership in the surface umbrella supplies no IR enum variant. (b) 034-9 says a yielded group 'stores nothing … holds no backing collection,' suggesting it may not be an IR cell entry at all — but then 034-10's 'distinct cell-KIND' overclaims, exactly the finding's disjunction. No LOG text reconciles 'distinct cell-KIND' with an IR enum that omits it. Sustained: an implementer building the IR from the LOG has no enum variant for a yielded-kind cell, and no rule states yielded is intentionally absent because it is never an IR cell entry. | No dissolving passage found. LOG:4192 (033-80) 'A cell entry carries a `kind` — `input` …, `derived`, `recurrent`, or `fold`' and LOG:4376 (035-9) 'extending the cell kind enum to `input | derived | recurrent | fold`' both omit yielded. SPEC:20535 ('`yielded T` groups … likewise participate as cell kinds') is the surface-annotation umbrella §13.18.5, not the IR `kind` tag; it does not add an IR enum variant and is silent on whether a yielded group is ever an IR cell entry.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:4361-4361`
    > 034-10. `yielded` is a distinct cell-KIND, and a change in membership of a `yielded` group propagates dirt to its consumers just as a value change does. (§13.20)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4376-4376`
    > 035-9. IR lowering of a fold introduces a new cell kind `fold`, extending the cell kind enum to `input | derived | recurrent | fold`, while the count of six graph primitives is unchanged. (§13.21)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4192-4192`
    > 033-80. A cell entry carries a `kind` — `input` (a stored, externally-written cell), `derived`, `recurrent`, or `fold` — classifying it; the kind leads the cell's text-form declaration. (§15.4.1)

#### F212 — 'walk order' is load-bearing in 034-8/034-12 (and drives 035-5's determinism) but is never defined in the LOG; only the SPEC defines it.

- **Severity/Category/Verdict:** MED / undefined_term / CONFIRMED
- **Anchors:** LOG 034-8, 034-12, 035-5 · SPEC § 13.20, 13.21
- **Why it is a defect:** 'Walk order' is the observable member/iteration order of a yielded group and the order fixing 035-5's deterministic combine tree. Its only normative definition ('lexical/structural order of their yield positions, with repeat members interleaved in key order') lives in SPEC 23053-23055; the LOG never states it. Section 017's related concept is spelled 'structural (entry) order' / 'traversal order' (017-264, 017-218), not 'walk order', so a reader cannot even confirm they are the same order. Per the LOG-is-self-contained invariant, a load-bearing order term used across three entries must be pinned in the LOG, and its identity with 017's order made explicit.
- **Direction of change:** Add a LOG decision defining 'walk order' (yield-position lexical/structural order with repeat members in key order) and state whether it equals section 017's structural/entry order; then have 034-8/034-12/035-5 lean on that single definition.
- **Evidence check:** pass — 'walk order' is load-bearing across 034-8/034-12 and underpins 035-5's determinism, but its only normative definition lives in SPEC 23053-23055; the LOG never defines it, and section 017's 'structural (entry) order' is never stated to be the same order, violating LOG self-containment.
- **Charity check:** sustain — 'walk order' is load-bearing across 034-8 (LOG 4359 'ordered, walk-order … group'), 034-12 (LOG 4363 'Item = the T member values in walk order'), and drives 035-5's deterministic combine tree fixed by member order. I grepped both documents: 'walk order'/'walk-order' appears in the LOG ONLY at 4359 and 4363 (both 034 entries, neither defining it); its sole normative definition lives in SPEC 23053-23055 'Members are in walk order — the lexical/structural order of their `yield` positions, with `repeat` members interleaved in key order at their position.' The LOG never defines the term. Section 017's analogous concept is spelled 'structural (entry) order'/'traversal order', not 'walk order', so the LOG does not assert the two orders are identical. Per the LOG-is-self-contained invariant, a load-bearing order term used across three entries must be pinned in the LOG. Sustained: definition exists only in SPEC. | No dissolving passage found in the LOG. A whole-document grep returned 'walk order'/'walk-order' in the LOG only at LOG:4359 (034-8) and LOG:4363 (034-12), neither defining it; the sole definition is SPEC:23053-23055 'Members are in walk order — the lexical/structural order of their `yield` positions, with `repeat` members interleaved in key order at their position' — which is SPEC, not LOG, and the LOG-is-self-contained invariant requires the term be pinned in the LOG.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:4359-4359`
    > 034-8. A `yielded T` is the ordered, walk-order, membership-varying group of cells that a `collect` produces. (§13.20)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4363-4363`
    > 034-12. `yielded T` fulfills `Iterable` at the language level, authored as a language-defined fulfillment in the style of `Keyed` and `StringifiableKey`, with `Item` = the `T` member values in walk order, usable only in runtime-loop contexts such as function, derived, and operator bodies, and it is not a dynamic-view consumer. (§13.20)
  - `packages/ductus-lang/docs/SPEC.md:23053-23055`
    > - **Ordered.** Members are in walk order — the lexical/structural order
    >   of their `yield` positions, with `repeat` members interleaved in key
    >   order at their position.

#### F211 — 034-9 imports 'gate on/off-bit machinery' but no rule in section 022 (Gates) ever defines on/off bits; the imported term has no home.

- **Severity/Category/Verdict:** MED / undefined_term / CONFIRMED
- **Anchors:** LOG 034-9 · SPEC § 13.20
- **Why it is a defect:** The risk brief requires every imported term to resolve to a real matching definition. 034-9 explicitly reuses 'the gate on/off-bit machinery' as its runtime realization, but a full scan of section 022 (lines 2883-3377) finds no rule naming on/off bits, activation bits, or a liveness-bit representation; gates are specified in terms of 'effectively active', 'open', 'freeze', and 'suspend propagation'. An implementer cannot find the machinery 034-9 says it reuses. The LOG is meant to be the self-contained decision-of-record (Invariant 2: entries atomic and self-contained), so the term must have a home the reader can reach.
- **Direction of change:** Either add/point to a section-022 decision that names and defines the on/off-bit (liveness-bit) representation gates use, or restate 034-9's realization in terms of the vocabulary 022 actually defines (effectively-active state per position).
- **Evidence check:** pass — 034-9 says it reuses 'the gate on/off-bit machinery,' but a full grep of the LOG shows the term 'on/off bits' occurs only in 034-9; section 022 (Gates) never defines any on/off-bit representation, so the imported term is undefined in the decision-of-record.
- **Charity check:** sustain — 034-9 says a `yielded` group is 'realized as a compile-time wire set plus runtime on/off bits, reusing the gate on/off-bit machinery' (DECISION_LOG.md:4360). A full scan of section 022 (Gates, lines 2883-3005) finds NO rule defining on/off bits, activation bits, or any bit-level liveness representation. Gates are specified entirely as frozen/open/close/propagation/'effectively active' (e.g. 022-40 'frozen-when-gated, snap on activation'; 022-41 'evaluates gate state at edge level'; 022-67 'effectively active iff its own gate and every ancestor gate ... are open'). Corpus-wide grep for 'on/off bit' returns exactly two hits: 034-9 itself, and SPEC §13.20.4 line 23051 which is the yielded-group elaboration DOWNSTREAM of 034-9, not a gate definition. So the imported term 'the gate on/off-bit machinery' has no home the reader can reach; per LOG Invariant 2 (atomic, self-contained) an implementer cannot resolve what machinery is being reused. No dissolving definition exists. Sustain.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:4360-4360`
    > 034-9. A `yielded` group stores nothing: it is realized as a compile-time wire set plus runtime on/off bits, reusing the gate on/off-bit machinery, and holds no backing collection of its own. (§13.20)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2951-2951`
    > 022-67. An instance is effectively active iff its own gate and every ancestor gate on the path from the root are open. (§13.9.7)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2973-2973`
    > 022-73. The reactive dependency graph is built independent of gate state; gates suspend propagation at runtime and never remove edges from the static graph. (§13.9.8)

#### F171 — Section 022 (the gate layer) never defines the 'on/off-bit machinery' that 034-9 claims to reuse from it; the only definition of 'on/off bits' lives in the yielded section itself.

- **Severity/Category/Verdict:** MED / gap / CONFIRMED
- **Anchors:** LOG 034-9, 022-40, 022-67, 022-72 · SPEC § 13.9.7, 13.20.4
- **Why it is a defect:** 034-9 asserts a reuse-of relationship: the yielded group's runtime membership representation IS the pre-existing 'gate on/off-bit machinery.' For that reuse claim to be grounded, the gate layer (section 022 / SPEC §13.9.7) must normatively establish a named on/off-bit representation of activation state. It does not: section 022 describes activation only as freeze / effectively-active / edge-level gate-state evaluation (022-40, 022-41, 022-67) and never introduces any 'on/off bit' primitive; the string 'bit' does not occur in the gate section or §13.9.7 in that sense. The sole definition of 'on/off bits' is in §13.20.4 (the yielded section itself, SPEC 23051), so 034-9 points back at machinery that only exists forward, in the very section citing it. The pointer contract is inverted: the reused mechanism is not homed where the reuse claims it lives. An implementer reading 034-9 to find the shared bit representation in the gate layer finds nothing there. The RISK NOTE flagged exactly this: the on/off-bit machinery reused by 034-9 should be normatively homed in 022, and it is not.
- **Direction of change:** Either home an explicit named on/off / activation-bit representation in section 022 (and §13.9.7) that both the gate freeze machinery and 034-9 can point at, or rephrase 034-9 so its 'reuse' claim references the mechanism where it is actually defined rather than asserting a gate-layer origin the gate layer does not provide. Decision belongs to the owner; do not resolve unilaterally.
- **Evidence check:** pass — 034-9 claims to reuse 'the gate on/off-bit machinery,' but section 022 (§13.9.7) never defines any on/off-bit primitive (only freeze / effectively-active / edge-level gate-state); the only definition of on/off bits lives forward in the yielded section itself (SPEC 23051), so the reuse pointer is not homed where it points.
- **Charity check:** sustain — 034-9 (LOG 4360) asserts a reuse relationship: the yielded group 'reus[es] the gate on/off-bit machinery.' For that reuse to be grounded, the gate layer (section 022 / §13.9.7) must normatively establish a named on/off-bit representation. It does not. I grepped both documents for 'on/off bit'/'on-bit'/'off-bit': the ONLY occurrence anywhere is SPEC 23051 (the yielded section itself, §13.20.4). Section 022's activation model is freeze / effectively-active / suspend-resume (022-67 LOG 2951, 022-68 LOG 2952, 022-72 LOG 2956) and SPEC §13.9.7 (SPEC 17663-17670 freeze triggers) — no 'bit' primitive. 022-72 provides a SEMANTIC anchor (a gated contribution 'joins a fold as an activation-driven member … present exactly while its position is effectively active'), but that is membership semantics, not a named bit mechanism. Sustained: 034-9 points at machinery whose only definition lives forward in the very section citing it; the 'on/off-bit' named mechanism is not homed in the gate layer. | No dissolving passage found homing the mechanism in the gate layer. A whole-document grep for 'on/off bit' returns exactly one occurrence — SPEC:23051 (§13.20.4, the yielded section itself) 'a compile-time wire set … plus runtime on/off bits.' Section 022 / §13.9.7 describes only freeze/effectively-active/suspend (LOG:2951 022-67 'effectively active iff its own gate and every ancestor gate on the path from the root are open'; SPEC:17663-17670 freeze triggers) and never introduces an on/off-bit primitive, so 034-9's 'reusing the gate on/off-bit machinery' points at machinery not defined in the gate layer.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:4360-4360`
    > 034-9. A `yielded` group stores nothing: it is realized as a compile-time wire set plus runtime on/off bits, reusing the gate on/off-bit machinery, and holds no backing collection of its own. (§13.20)
  - `packages/ductus-lang/docs/SPEC.md:23049-23052`
    > - **Stores nothing.** A `yielded` group holds no buffer of values: it is
    >   a compile-time wire set (which positions can contribute) plus runtime
    >   on/off bits (which are currently live). The member *values* live in the
    >   member cells themselves; the group is only the membership structure.
  - `packages/ductus-lang/docs/SPEC.md:17663-17670`
    > **Freeze triggers.** A construct freezes under exactly two conditions: a
    > gate evaluating false (this section), and — for a connection — a dynamic
    > destination resolving to `None` (§13.6.2). The second follows Model B
    > identically: the unresolved state behaves as gate-false (the freeze
    > semantics below), and resolution returning `Some` behaves as the
    > false→true flip (the snap-on-activation semantics below, including its
    > propagation event). Nothing else freezes a construct; in particular,
    > traversal position never does (§13.3.7.6).

#### F167 — Section 024's reactive-dependency-graph construction (walk derived+recurrent bodies; signal/attr/derived/recurrent reads contribute edges) never names the fold cell kind, so a fold-mediated instantaneous cycle has no covering detection rule.

- **Severity/Category/Verdict:** MED / gap / CONFIRMED
- **Anchors:** LOG 024-2, 024-4, 024-6 · SPEC § 13.11.1, 13.11.2, 13.21.6, 13.21.7
- **Why it is a defect:** Fold is a distinct cell kind (033-80, 035-9) whose combiner (`by:`) and member edges read cells, and a fold sits on a `derived` RHS (035-8, 13.21.6). Section 024 is the normative reactive-cycle-detection site, but its graph-construction rules (024-2, 024-4) enumerate only `derived`/`recurrent` bodies and `signal/attr/derived/recurrent` reads — fold-form member/combiner reads and fold-kind reads are not named. Sections 034/035 (fold/yield, the recent amendment) contain zero cycle rules. An implementer building the dependency graph strictly per 024-2/024-4 would not walk fold combiner/member reads, so a `derived total = fold m: by:(+) else:0` whose member is a derived that reads `total` back forms an instantaneous cycle that the enumerated rules do not detect. Two readings: (a) treat fold-as-derived (033-82 'consumers see derived T') and it is caught; (b) follow 024-2/024-4 literally and it is not. Per 001-6 there is no legal std/impl-defined boundary here — cycle rejection is a compile-time soundness gate.
- **Direction of change:** Decide (do not resolve here) whether fold-form member/combiner reads and fold-kind reads must be enumerated in 024-2/024-4 and whether 024-6's derived-to-derived prohibition subsumes fold; surface to user as a coverage question for section 024 vs the fold amendment.
- **Evidence check:** pass — Does 024-2/024-4's graph-construction enumeration cover fold-kind cells? It names only derived/recurrent bodies and signal/attr/derived/recurrent reads; SPEC 23185 makes fold a distinct 4th cell kind. Evidence supports the un-covered-cycle-detection gap.
- **Charity check:** sustain — Fold is a distinct IR cell kind, not a derived cell: 035-9 / 033-80 (DECISION_LOG.md:4376,4192) enumerate the kind enum as `input | derived | recurrent | fold`, and SPEC §13.21.7 (SPEC.md:23185-23188) confirms fold 'lowers to a new cell kind, fold'. The reactive-dependency-graph construction that feeds cycle detection (024-2/024-4, DECISION_LOG.md:3069,3071; SPEC §13.11.1, SPEC.md:18314-18319) walks ONLY 'every derived body and every recurrent expression body' and records edges for 'Signal, attr, derived, and recurrent reads' — fold combiner/member reads and reads OF a fold-kind cell are not named. The charitable dissolver, 033-82 'consumers see derived T' (DECISION_LOG.md:4194), is a SURFACE-TYPE statement, and 033-82's clause 'membership changes on its member edges propagate dirt exactly as member value changes do' is about runtime DIRT propagation, not the compile-time cycle graph. Neither text forces the cycle-graph walker to traverse fold cells. Witness stands: `derived total = fold m:...` whose member `m` (a derived) reads `total` back — 024-2 walks m's body and records total->m, but never walks the fold body to record m->total, so 024-6's derived-to-derived check never sees the cycle. Two readings survive (fold-as-derived caught vs. 024-2/024-4 literal not caught); the inference-required-ness is the finding. SUSTAIN.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:3069-3069`
    > 024-2. The compiler builds the reactive dependency graph by walking every `derived` body and every recurrent expression body, recording the set of reactive cells each reads. (§13.11.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3071-3071`
    > 024-4. Signal, attr, derived, and recurrent reads all contribute reactive-dependency edges. (§13.11.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3073-3073`
    > 024-6. A cycle consisting only of derived-to-derived edges is a compile error (instantaneous cycle): derived `a.x` reading `b.y` while `b.y` reads `a.x` is rejected. (§13.11.2)
  - `packages/ductus-lang/docs/SPEC.md:23185-23188`
    > A `fold` expression lowers to a **new cell kind**, `fold`, in the graph IR
    > (§15.4.1). The IR cell-`kind` enum gains `fold` — `input | derived |
    > recurrent | fold`. **`fold` is a cell kind, not a seventh graph
    > primitive**: the graph's six-primitives count (§15.4.1) is **unchanged**.

#### F164 — 034-11 mandates `repeat` over a `yielded` group, but 018's source-admission rule (018-37) enumerates only `Signal[I]` or a `dynamic` collection cell, admitting neither a yielded group nor its cell-kind.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 018-37, 018-41, 018-3 · SPEC § 13.5.4.1, 13.20
- **Why it is a defect:** 034-11 states a `yielded` group 'is consumed only by the fold form, by yielded-typed parameters ... and by `repeat` over it', and SPEC §13.20 corroborates ('**`repeat` over it** (§13.5.4) — materializes one scope per live member'). But a `yielded T` is a distinct cell-KIND (034-10) that stores nothing (034-9); it is not a `Signal[I]` and is not a `dynamic` collection cell. 018-37 (and SPEC §13.5.4.1) give a closed, gate-worded enumeration of legal `repeat` sources — reinforced by 018-41 explicitly excluding `Stream[T]` — that admits neither the yielded group nor its kind. An implementer reading only 018 would reject `repeat over_yielded`, contradicting 034-11/§13.20. The source-admission rule and the yielded-consumption rule diverge on whether a yielded group is a legal source.
- **Direction of change:** 018-37 (and its SPEC §13.5.4.1 source list) needs a decision on admitting `yielded T` as a `repeat` source to match 034-11/§13.20; whether yielded is folded into 'Signal[I]', added as a third source category, or 034-11 is narrowed is a design call to surface.
- **Evidence check:** pass — 018-37's closed repeat-source enumeration (Signal[I] or dynamic collection cell) admits neither a yielded group nor its cell-kind, but 034-11 mandates repeat over a yielded group; an implementer reading only 018 rejects the exact program 034 requires.
- **Charity check:** sustain — 034-11 and SPEC §13.20.4.1 (line 23069) both name '`repeat` over it' as a legal consumer of a yielded group, but 018-37 and SPEC §13.5.4.1 (line 15265) both give a CLOSED source enumeration — 'a reactive collection cell: Signal[I] for some I: Iterable, or a dynamic collection cell' — that admits neither a yielded group nor its cell-kind. A yielded group is a distinct cell-KIND that stores nothing (034-9/034-10), not a Signal[I] and not a dynamic collection cell; 034-12's Iterable fulfillment is scoped to 'runtime-loop contexts only' (a runtime for), not repeat. Grep confirms no passage anywhere characterizes a yielded group as a Signal[I] or reactive collection cell, so the source-admission rule cannot be charitably read to include it. Divergence is real on both LOG (034-11 vs 018-37) and SPEC (§13.20.4.1 vs §13.5.4.1) sides.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2507-2507`
    > 018-37. A `repeat` source is a reactive collection cell: `Signal[I]` for some `I: Iterable`, or a `dynamic` collection cell (a `dynamic` node view or connection-view). (§13.5.4.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2511-2511`
    > 018-41. `Stream[T]` is not a valid `repeat` source. (§13.5.4.1)
  - `packages/ductus-lang/docs/SPEC.md:15265-15268`
    > **`<source>`** is a reactive collection cell: `signal I` for some
    >   `I: Iterable` (§12.8), or a **`dynamic` collection cell** of the
    >   enclosing node (a `dynamic` view or connection-view,
    >   §13.3.3.4), whose value is a language-provided keyed, ordered

#### F124 — 'Propagates dirt as a value change does' is unoperationalized for membership: no dirty root, no dirty-bit rule, and no DAG-node rule is given for a membership change.

- **Severity/Category/Verdict:** MED / undefined_term / CONFIRMED
- **Anchors:** LOG 034-10, 023-8, 023-9, 023-12 · SPEC § 13.20.4, 13.10.2
- **Why it is a defect:** A value change has a concrete dirty root: a writable cell whose staged value differs (023-8), from which propagation flows to deriveds/recurrents. A membership change has no analogue defined. 034-10/SPEC 23047 say membership 'propagates dirt … just as a value change does,' but never identify what the dirty ROOT of a membership change is (the source signal `items`? the `repeat`'s dynamic scope? the yielded group cell itself?), nor how that root's dirtiness is detected during step-1's writable-cell scan, nor how it reaches the yielded/fold consumer given 023-12 forbids adding dirty bits after step 1. Two careful readings — 'the source signal's dirt is the root' vs 'the yielded group is itself a dirty root discovered separately' — yield different DAG contents and different evaluation orders when membership and value both change in one commit.
- **Direction of change:** Define the dirty root and detection point for a membership change (tie it to the source signal's dirtiness and the repeat re-key diff of 018-76), and state where in the step-1 dirty computation it is registered so it respects the no-new-dirty-bits-after-step-1 rule.
- **Evidence check:** pass — Membership-change dirt is asserted to propagate 'just as a value change does' but no dirty ROOT, dirty-bit rule, or DAG-node rule is defined for it, while 023-8 defines dirt only for writable cells and 023-12 forbids new dirty bits after step 1.
- **Charity check:** sustain — The dirt-set mechanism is defined only for writable cells (023-8: 'a writable cell (signal, attr) is dirty iff its staged value differs') with transitive propagation to deriveds (023-10) and triggered recurrents (023-11), and 023-12 forbids adding new dirty bits after step 1. Nothing in the LOG or in SPEC §13.10.2 identifies the dirty ROOT of a membership change on a yielded group, nor how step-1's writable-cell scan discovers it. 034-10/033-82 (SPEC 23047) merely assert membership 'propagates dirt just as a value change does' without operationalizing which cell is the root. I searched all 'propagate dirt'/'membership change' occurrences (SPEC 23047; LOG 033-82, 034-10) and the entire 023 dirt-set block (LOG 3009-3027) — no membership-dirty-root rule exists. The two plausible readings the finding names (source signal's dirt is the root vs the yielded group is a separately-discovered dirty root) both remain open, and 023-12 actively obstructs any 'discovered later' reading. Sustained: the propagation claim has no dirty-root/dirty-bit operationalization for membership. | No dissolving passage found. Searched LOG 3009-3027 (§13.10.2 dirt-set block: 023-8 LOG:3016 'a writable cell (signal, attr) is dirty iff its staged value differs fr
om its previously-committed value'; 023-10 LOG:3018 propagation to deriveds; 023-11 LOG:3019 recurrents; 023-12 LOG:3020 'no new dirty bits are added') and SPEC:23047 'membership changes propagate dirt to consumers' — none identifies the dirty root or detection path for a membership change.
- **Evidence:**
  - `packages/ductus-lang/docs/SPEC.md:23044-23047`
    > `yielded T` is the **kind** of the group a `collect` produces: an
    > **ordered** (walk-order), membership-varying group of cells carrying
    > values of type `T`. It is a cell-KIND (a member of the `cell` umbrella,
    > §13.18.5): membership changes propagate dirt to consumers.
  - `packages/ductus-lang/docs/DECISION_LOG.md:3016-3016`
    > 023-8. Commit step 1 computes the dirty set: a writable cell (signal, attr) is dirty iff its staged value differs from its previously-committed value. (§13.10.2)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3020-3020`
    > 023-12. After dirty-set computation, no new dirty bits are added during the rest of the commit cycle. (§13.10.2)

#### F097 — 034-11 permits `repeat` over a `yielded` group, but 018-37's repeat-source rule admits only `Signal[Iterable]` or a dynamic collection cell, and 034-12 says a yielded group is neither.

- **Severity/Category/Verdict:** MED / contradiction / CONFIRMED
- **Anchors:** LOG 034-11, 018-37, 034-12 · SPEC § 13.5.4.1, 13.20
- **Why it is a defect:** 018-37 restricts a repeat source to `Signal[I: Iterable]` or a dynamic view/connection-view. 034-12 states a `yielded` group is explicitly NOT a dynamic view, and its Iterable fulfillment is "usable only in runtime-loop contexts" — whereas `repeat` is a structural (compile-time graph) construct, and a structural `for` over a yielded group is a compile error (034-11, 014-71). SPEC §13.5.4 (lines 15271-15279) lists the admitted repeat sources and does not include `yielded`. So 034-11's "`repeat` over it" has no matching source-type admission and is contradicted by 018-37. An implementer cannot both reject non-(Signal[Iterable]/dynamic-view) repeat sources and accept a yielded group.
- **Direction of change:** Either extend 018-37 to admit a `yielded` group as a repeat source (and reconcile with 034-12's runtime-only Iterable restriction), or retract `repeat` from 034-11's consumer list; decide and align LOG + SPEC §13.5.4/§13.20.
- **Evidence check:** pass — 034-11 permits repeat over a yielded group, but 018-37 admits only Signal[Iterable] or a dynamic collection cell as a repeat source and 034-12 states a yielded group is neither — an implementer cannot both enforce 018-37's source restriction and accept a yielded repeat source.
- **Charity check:** sustain — 034-11 (LOG 4362) grants 'repeat over it [a yielded group]' and SPEC §13.20.4.1 (SPEC 23069) lists 'repeat over it (§13.5.4)' as a yielded consumer. But the repeat SOURCE-admission rule admits only two source shapes and yielded is neither: 018-37 (LOG 2507) 'A `repeat` source is a reactive collection cell: `Signal[I]` for some `I: Iterable`, or a `dynamic` collection cell'; SPEC §13.5.4.1 (SPEC 15265-15279) restates this and does NOT list yielded. 034-12 (LOG 4363) states a yielded group 'is not a dynamic-view consumer' and its Iterable fulfillment is 'usable only in runtime-loop contexts,' whereas repeat is structural (a structural for over yielded is a compile error, 034-11). SPEC §13.5.4.2 (SPEC 15388-15395) confirms repeat operates on the signal's committed snapshot via Iterable::iterator — which a stores-nothing yielded group (034-9) is not. Sustained: 034-11's grant has no matching source-type admission and is contradicted by 018-37; also a SPEC-internal divergence (SPEC 23069 lists a consumer SPEC 15265-15279 does not admit as a source). | No dissolving passage found; the candidate dissolver conflicts with the finding. LOG:2507 (018-37) 'A `repeat` source is a reactive collection cell: `Signal[I]` for some `I: Iterable`, or a `dynamic` collection cell' and SPEC:15265-15279 ('`signal I` for some `I: Iterable` … or a `dynamic` collection cell') admit only those two shapes — neither is a yielded group per 034-12 (LOG:4363 'it is not a dynamic-view consumer'). SPEC:23069 ('repeat over it') grants the consumer the source rule does not admit; the two passages conflict rather than dissolve the finding.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:4362-4362`
    > 034-11. A `yielded` group is consumed only by the fold form, by yielded-typed parameters of both operators and functions, and by `repeat` over it; it supports no indexing, and a structural compile-time `for` over a `yielded` group is a compile error. (§13.20)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2507-2507`
    > 018-37. A `repeat` source is a reactive collection cell: `Signal[I]` for some `I: Iterable`, or a `dynamic` collection cell (a `dynamic` node view or connection-view). (§13.5.4.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4363-4363`
    > 034-12. `yielded T` fulfills `Iterable` at the language level, authored as a language-defined fulfillment in the style of `Keyed` and `StringifiableKey`, with `Item` = the `T` member values in walk order, usable only in runtime-loop contexts such as function, derived, and operator bodies, and it is not a dynamic-view consumer. (§13.20)

#### F096 — A `yielded T` operator/function parameter is a permitted consumer (034-11/12) but the behavior ABI and IR type vocabulary have no representation for passing a yielded group across the behavior boundary.

- **Severity/Category/Verdict:** MED / gap / CONFIRMED
- **Anchors:** LOG 034-11, 034-12, 033-43, 033-199 · SPEC § 13.20, 15.4, 15.4.4
- **Why it is a defect:** 033-43's IR type vocabulary lists no yielded-group type; 033-47/48/49/50 extend the vocabulary for slices, Portal, Map, Bundle but never for `yielded`. Behavior ABI inputs are cells whose types come from that vocabulary (033-156, 033-199). A `yielded T` group is not a single cell and not in the vocabulary, so an implementer cannot write the calling convention for `fn f(g: yielded Voice)` or an operator with a `yielded T` port. 033-67 lowers an operator to "a scope with ports" but no rule says a port can carry a yielded group. This is a lowering-totality gap for a consumer path the LOG explicitly declares legal, with no legal boundary declared.
- **Direction of change:** Add an IR representation for a `yielded T` value in parameter/port position (e.g. how the membership wire-set is presented to a behavior/operator body), or declare the ABI form implementation-defined.
- **Evidence check:** pass — A yielded-T operator/function parameter is a declared-legal consumer, but the IR type vocabulary (033-43) contains no yielded-group type and a yielded group is not a single cell, so no calling convention/port representation exists to pass it across the behavior ABI, with no legal boundary declared.
- **Charity check:** sustain — 034-11/034-12 (LOG 4362-4363) and SPEC §13.20.4.1 (SPEC 23067-23068) explicitly permit `yielded T` as a parameter type on operators AND functions, with 'the body walks the members in walk order.' Behavior parameters are typed from the IR type vocabulary — 033-156 (LOG 4268) 'A behavior parameter `p: T`' where T comes from 033-43 (LOG 4155): §4.1 primitives plus str/tuples/arrays/%TypeIds/pool_index/closure. 033-47..50 (LOG 4159-4162) extend that vocabulary for slices, Portal, Map, Bundle — never for yielded. I searched SPEC for yielded+ABI/param/IR/lower: only SPEC 23067 (the surface permission) exists; nothing in §15.4/§15.4.4 gives a calling-convention representation for passing a membership-varying group across the behavior boundary, and no legal boundary (001-6 std-delegation or implementation-defined) is declared. Sustained: a lowering-totality gap for a consumer path the LOG declares legal. | No dissolving passage found. LOG:4155 (033-43) enumerates the IR type vocabulary with no yielded-group type; LOG:4159-4162 (033-47..50) extend it only for slices/Portal/Map/Bundle; LOG:4268 (033-156) types behavior params from that vocabulary. SPEC yielded-consumer text (SPEC:23067 'Yielded-typed parameters — `yielded T` may be a parameter type on operators and on functions') states the permission but supplies no ABI/IR representation and declares no legal boundary.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:4362-4362`
    > 034-11. A `yielded` group is consumed only by the fold form, by yielded-typed parameters of both operators and functions, and by `repeat` over it; it supports no indexing, and a structural compile-time `for` over a `yielded` group is a compile error. (§13.20)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4155-4155`
    > 033-43. The IR type vocabulary is the §4.1 primitives plus `str`, tuples `(T,…)`, arrays `[T;N]`, record/enum `%TypeId`s, `pool_index<%PoolId>`, and `closure<(T,…)->R>`. (§15.4)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4363-4363`
    > 034-12. `yielded T` fulfills `Iterable` at the language level, authored as a language-defined fulfillment in the style of `Keyed` and `StringifiableKey`, with `Item` = the `T` member values in walk order, usable only in runtime-loop contexts such as function, derived, and operator bodies, and it is not a dynamic-view consumer. (§13.20)

#### F095 — The IR/lowering section (033) never states what graph primitive or IR entry a `yielded` group becomes, though 033-58 claims every surface reactive construct desugars into the six primitives.

- **Severity/Category/Verdict:** MED / gap / CONFIRMED
- **Anchors:** LOG 033-58, 034-8, 034-9, 034-10 · SPEC § 15.4.1, 13.20
- **Why it is a defect:** 033-59..033-71 enumerate the lowering of every other surface reactive construct (signal/attr/derived/recurrent/const->cell, node/placements->scope, repeat->dynamic scope, when/given->gate, connections/|>->connection, operator->scope, stream->stream, effect->effect, Bundle->scope). `collect`/`yield`/`yielded` appear nowhere in section 033. 034-9 gives a mechanism ("compile-time wire set plus runtime on/off bits") but a wire set is not one of the six primitives (033-56) and no rule maps it to one. Only the fold consumer path (035-9/10) is lowered. An implementer lowering a named `yielded` group (`collect as x`) has no IR entry to emit for the group itself, its membership structure, or its dirt-propagation edges. This is an implementer-blocking gap with no declared legal boundary (no std-delegation or implementation-defined marker per 001-6).
- **Direction of change:** State the IR representation of a standalone `yielded` group (its membership wire-set and on/off bits) as an entry in one of the six primitives, or declare it implementation-defined; reconcile with 033-58's blanket desugar claim.
- **Evidence check:** pass — Section 033 never states what graph primitive or IR entry a `yielded` group becomes for the non-fold path, though 033-58 claims every surface reactive construct desugars into the six primitives.
- **Charity check:** sustain — Narrower MED sibling of F093, independently valid. Section 033 (LOG 4110-4260, SPEC §15.4.1) never states what graph primitive or IR entry a `yielded` group becomes. 033-59 through 033-71 enumerate lowering for every OTHER surface reactive construct but `collect`/`yield`/`yielded` appear nowhere in 033. 034-9's wire-set-plus-on/off-bits is a mechanism, not one of the six primitives (033-56) and no rule maps it to one; SPEC 20535's 'participate as cell kinds' is surface-annotation framing, not IR. Only the fold consumer (035-9/10, fold_payload) is lowered. A named `yielded` group (`collect as x`) has no IR entry for the group itself, its membership structure, or its dirt-propagation edges. No std-delegation or implementation-defined marker (001-6). No dissolving text found.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:4170-4170`
    > 033-58. All surface reactive constructs desugar into the six graph primitives. (§15.4.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4360-4360`
    > 034-9. A `yielded` group stores nothing: it is realized as a compile-time wire set plus runtime on/off bits, reusing the gate on/off-bit machinery, and holds no backing collection of its own. (§13.20)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4361-4361`
    > 034-10. `yielded` is a distinct cell-KIND, and a change in membership of a `yielded` group propagates dirt to its consumers just as a value change does. (§13.20)

#### F094 — `repeat` over a `yielded` group is declared legal, but the `repeat`-source rule admits only `Signal[I]` or a dynamic collection cell, and a yielded group is neither (it stores nothing).

- **Severity/Category/Verdict:** MED / contradiction / CONFIRMED
- **Anchors:** LOG 018-37, 034-9, 034-11, 034-12 · SPEC § 13.5.4.1, 13.20
- **Why it is a defect:** 034-11 lists `repeat` over a yielded group as a legal consumer. But 018-37 exhaustively defines a `repeat` source as either a `Signal[I]` cell or a `dynamic` collection cell. A yielded group is neither: 034-9 says it stores nothing and holds no backing collection, so it is not a collection cell; it is a membership structure, not a `Signal[I]`. 034-12 further restricts yielded's `Iterable` fulfillment to 'runtime-loop contexts such as function, derived, and operator bodies' and says a structural `for` over it is a compile error — yet `repeat` is a structural dynamic-scope construct (033-61: `repeat` lowers to a dynamic `scope`), not a runtime loop. So two rules cannot both hold: either yielded is admissible as a `repeat` source (needing 018-37 to be widened and yielded's Iterable to reach structural repeat) or it is not (contradicting 034-11's consumer list). An implementer cannot satisfy both.
- **Direction of change:** Reconcile 034-11 with 018-37 and 034-12: either widen the `repeat`-source rule to admit yielded groups (and state the key/iteration model for a store-nothing membership structure) or remove `repeat` from yielded's consumer list.
- **Evidence check:** pass — 034-11 lists `repeat` over a yielded group as legal, but 018-37 enumerates repeat sources as only `Signal[I]` or a dynamic collection cell — and 034-9 says yielded stores nothing (no collection cell), while 034-12 confines yielded's Iterable to runtime-loop contexts and declares it 'not a dynamic-view consumer'. repeat is a structural dynamic-scope consumer, so it cannot consume yielded under 018-37/034-12 yet must under 034-11. An implementer cannot satisfy both. Real contradiction.
- **Charity check:** unverified — missing from verifier output
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2507-2507`
    > 018-37. A `repeat` source is a reactive collection cell: `Signal[I]` for some `I: Iterable`, or a `dynamic` collection cell (a `dynamic` node view or connection-view). (§13.5.4.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4362-4363`
    > 034-11. A `yielded` group is consumed only by the fold form, by yielded-typed parameters of both operators and functions, and by `repeat` over it; it supports no indexing, and a structural compile-time `for` over a `yielded` group is a compile error. (§13.20)
    > 034-12. `yielded T` fulfills `Iterable` at the language level, authored as a language-defined fulfillment in the style of `Keyed` and `StringifiableKey`, with `Item` = the `T` member values in walk order, usable only in runtime-loop contexts such as function, derived, and operator bodies, and it is not a dynamic-view consumer. (§13.20)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4360-4360`
    > 034-9. A `yielded` group stores nothing: it is realized as a compile-time wire set plus runtime on/off bits, reusing the gate on/off-bit machinery, and holds no backing collection of its own. (§13.20)

#### F241 — 033-64 lowers a static effect argument to 'a degenerate constant cell' (a source_cell_id), while 033-121 says a parameter binding may be a value_literal, giving two readings of how a static |> argument is represented.

- **Severity/Category/Verdict:** MED / ambiguity / PLAUSIBLE
- **Anchors:** LOG 033-64, 033-121 · SPEC § 15.4.1
- **Why it is a defect:** For the same slot type, 033-64 says a static-expression argument is 'wrapped as a degenerate constant cell' — i.e. the slot holds a `source_cell_id` pointing at a synthesized cell. 033-121 says a slot may hold a `value_literal` directly. Reading A: static args always become constant cells, so `value_literal` is dead / never emitted for effect params. Reading B: static args are inlined as `value_literal`, so no degenerate constant cell is synthesized. The two entries produce different IR for the same source (`x |> print` with static `x`): a constant-cell entry plus a `source_cell_id` binding, versus an inline `value_literal` and no synthesized cell. This is behavior-changing for hot-reload diffing (a cell add/remove vs a literal change) and for cell-ID stability.
- **Direction of change:** Reconcile 033-64's 'degenerate constant cell' path with 033-121's `value_literal` alternative: state authoritatively whether a static effect argument is emitted as a constant cell or an inline literal (and if both are legal, when each applies). Cross-shard: 033-121 is outside this shard's ownership (033-1..118).
- **Evidence check:** pass — A static effect argument: 033-64 says it lowers to a degenerate constant cell (source_cell_id); 033-121 admits a value_literal binding — two readings yield different IR (cell add/remove vs literal change) for the same source.
- **Charity check:** refute — The finding's premise — 033-64's degenerate-constant-cell and 033-121's value_literal are two readings of the SAME slot type — is false. They govern DIFFERENT parameter kinds. The uniform cell-argument rule (SPEC 19662-19685, echoed by 034-6) fixes that a static expression passed to a `cell T` parameter is ALWAYS constant-wrapped into a synthesized constant cell referenced by source_cell_id, tagged `constant_wrap` (SPEC 24167-24176) — never inlined as a value_literal. The `value_literal` case is for value (non-`cell`) parameters, which are evaluated and snapshotted (SPEC 19686; example `rate: f32 = 0.1` snapshotted at instantiation, SPEC 19632-19634, 19646). So a `cell`-param static arg = source_cell_id/constant_wrap; a value-param arg = value_literal. One source does not have two IR representations. The dissolving text (SPEC 19671-19686, 24167-24176) is consistent with both 033-64 and 033-121 — no conflict, so refute, not refile_divergence. | SPEC.md:19671-19675: "**A static expression is constant-wrapped.** A literal, `const`, or\ncompile-time-constant expression is wrapped as a degenerate constant\ncell (compile-time-fixed `signal T`). Cost: one cell per literal at\nthe call site (effectively zero — folded by the compiler when\npossible)." AND SPEC.md:19686-19687: "- Values passed to `T` (value, non-`cell`) parameters are evaluated and\nsnapshotted, as above." AND SPEC.md:24167-24176 (parameter_bindings): "list of `(parameter_name, source_cell_id |\nvalue_literal, provenance_marker)` triples. The `provenance_marker`\nrecords how the cell argument was materialized ... `constant_wrap` (a static expression wrapped as a degenerate constant\ncell) ... The marker is metadata on the existing binding, **not** a third binding\ncase"
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:4176-4176`
    > Each `parameter_bindings` slot carries a provenance marker recording whether its argument is a bare cell, a static expression wrapped as a degenerate constant cell, or a reactive expression bridged through an implicit derived; the marker is a tag on the existing slot, not a third binding case. (§15.4.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4233-4233`
    > 033-121. An effect entry's `parameter_bindings` are `(parameter_name, source_cell_id | value_literal)` pairs. (§15.4.1)

#### F044 — ch12 asserts `yielded T` is runtime-loop-iterable via Iterable, but every ch12 Iterable rule requires a source-bearing cursor over a live, unmutated backing collection; no ch12 rule covers a stores-nothing, membership-varying source or a mid-iteration membership change.

- **Severity/Category/Verdict:** MED / gap / PLAUSIBLE
- **Anchors:** LOG 014-65, 014-71, 014-117, 014-120, 014-161 · SPEC § 12.3.7, 12.7.4, 12.8, 12.13
- **Why it is a defect:** ch12's Iterable contract is entirely source-bearing: the iterator holds only a cursor (014-117) and reads elements from a backing collection `source` passed borrow-equivalently every `next` call, with `Iter.Source = Subject` (014-120/121) and the source held live and forbidden to mutate for the loop's span (SPEC 12.8.2). ch12 names `yielded T` as a runtime-loop-iterable source (014-65, 014-71) but supplies no rule saying WHAT `Source` a yielded group presents (034-9 says it holds no backing collection) nor what happens when the group's membership changes during a loop pass (034-10 makes membership change a first-class propagating event). 014-161 addresses only reactive value updates, not membership changes of the iterated collection. An implementer walking a live yielded group with a runtime `for` has no rule to satisfy the source-bearing cursor contract and no defined behavior for member join/leave mid-iteration, and neither std-delegation nor implementation-defined behavior is stated as the boundary (001-6).
- **Direction of change:** Add a ch12 rule (or explicit cross-reference into ch13's yielded semantics) pinning what Source a `yielded T` presents to the Iterable contract and defining mid-iteration membership-change behavior for a runtime `for` over a yielded group, or state the boundary as implementation-defined/std-delegated per 001-6.
- **Evidence check:** pass — ch12 declares `yielded T` runtime-loop-iterable via a source-bearing Iterable contract (cursor over a live, unmutated backing collection with `Iter.Source = Subject`), but supplies no ch12 rule for what Source a stores-nothing yielded group presents nor for member join/leave during a loop pass, and 014-161 addresses only reactive value updates — leaving mid-iteration membership change undefined with no stated std/implementation-defined boundary.
- **Charity check:** refute — The mid-iteration-membership sub-gap is covered by explicit ch12 text the finder read too narrowly, and the Source-mechanics sub-gap is covered by an explicit language-defined-fulfillment boundary. (1) Membership change during a pass: 034-10 (DECISION_LOG.md:4361) states a membership change 'propagates dirt to its consumers JUST AS A VALUE CHANGE does' — collapsing the value-vs-membership distinction the finding leans on. SPEC §12.13 (SPEC.md:11295-11299) then explicitly covers a reactive iterated collection in a derived body: 'The collection or range being iterated in a derived body may itself be a reactive value. Each time the reactive value updates, the derived re-evaluates and the loop re-runs,' and 'reactive updates do not interrupt an in-progress loop iteration.' A yielded group is a reactive cell-kind (034-10), so a mid-pass member join/leave dirties the enclosing derived and re-runs the loop on the next cycle; the in-progress pass is atomic. This is explicit normative text, not inference. (2) Source mechanics for a stores-nothing group: SPEC §13.20.4.2 (SPEC.md:23084-23090) declares yielded's Iterable a 'language-defined fulfillment' available in runtime-loop contexts only — the Iter/Source implementation is language-provided (a legal boundary), and Iter.Source=Subject is forced by the Iterable contract (§12.8/014-121). Consistency check: §12.8.2's 'no mutation while the iterator is live' governs within a single pass; §12.13 governs cross-pass reactive re-run — no conflict, so no refile. REFUTE.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1704-1704`
    > 014-65. Loops over variable-extent types run at runtime: `Vec[T]`, `SmallVec[T, N]`, `RingBuf[T, N]`, `String`/`s.chars()`, `Map[K, V]`, `yielded T`, and ranges with runtime bounds. A `yielded T` group is runtime-loop iterable only. (§12.3.7)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1756-1756`
    > 014-117. An iterator cannot hold a borrow-equivalent reference to its collection; a source-bearing iterator carries only a cursor. (§12.7.4)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1759-1760`
    > 014-120. The stdlib `Iterable` trait declares `type Iter: Iterator` and `fn iterator(value: Subject) -> Iter` with `where Iter.Source = Subject`. (§12.8)
    > 014-121. The `where Iter.Source = Subject` constraint binds the iterator's source type to the iterable itself. (§12.8)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1800-1800`
    > 014-161. A `while` condition may depend on reactive values, but reactive updates do not interrupt an in-progress loop iteration. (§12.13)
  - `packages/ductus-lang/docs/SPEC.md:10916-10923`
    > #### 12.8.2 Iterator lifetime
    > 
    > The `iterator` method takes its parameter by default convention. The
    > returned iterator's lifetime is bounded by the for-loop expression:
    > the cluster's iteration alias rooted in the source is live for the
    > loop's duration. The source-mutation invariants of §11.9.2 apply for
    > that same span (no move, no mutation of the source while the iterator
    > is live).

#### F053 — The 3-way fold membership driver has two unreconciled vocabularies inside the LOG: {permanent, keyed-template, gate-guarded} vs {permanent, key-driven, activation-driven}, with no LOG entry stating they are the same three drivers.

- **Severity/Category/Verdict:** LOW / vague_term / CONFIRMED
- **Anchors:** LOG 033-81, 035-10, 034-3, 022-72 · SPEC § 13.20.3, 15.4.1
- **Why it is a defect:** The membership driver is a defined 3-way tag. SPEC §13.20.3 (line 22997-22999) explicitly bridges the two vocabularies by saying the surface drivers (permanent/key-driven/activation-driven) are 'the same three drivers the IR records' (keyed-template/gate-guarded). But no LOG entry makes this identity statement. Per LOG Invariant 2 entries are atomic and self-contained, and per LEARNING 8 the LOG is the self-contained decision-of-record. An implementer grounding in the LOG alone reads key-driven (034-3) and keyed-template (033-81) as two labels with no stated equivalence. This is terminology drift, not a soundness hole: the SPEC reconciles it, and the mapping (key-driven=keyed-template, activation-driven=gate-guarded) is inferable from position (repeat / gated arm). LOW.
- **Direction of change:** Surface to user: decide whether the LOG should carry one canonical driver vocabulary, or add a self-contained equivalence statement so a LOG-only reader sees key-driven=keyed-template and activation-driven=gate-guarded. Adjudicator does not pick the naming.
- **Evidence check:** pass — Two membership-driver vocabularies unreconciled within the LOG.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:4193-4193`
    > 033-81. A `fold`-kind cell entry carries a combiner behavior id (a `fn(T, T) -> T` handle), an else value (the empty-membership result), and member edges listed in member order; each member edge is tagged with its membership driver — `permanent`, `keyed-template`, or `gate-guarded`. (§15.4.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4354-4354`
    > 034-3. Membership is positional: a `yield` at a static position is a permanent member; a `yield` inside a `repeat` is a key-driven member, present once per repetition key; a `yield` inside a gated arm is an activation-driven member, present exactly when that arm is effectively active. (§13.20)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4377-4377`
    > 035-10. A fold-kind cell carries a combiner behavior id, an else value, and member edges in member order, each edge tagged with its membership driver — permanent, keyed-template, or gate-guarded — while its surface remains a `derived T` declaration seen by consumers. (§13.21)
  - `packages/ductus-lang/docs/SPEC.md:22997-22999`
    > A `yield`'s **membership driver** is determined by its lexical position
    > inside the `collect` block — the same three drivers the IR records
    > (§15.4.1):

#### F052 — 034-12 restricts the `yielded`-`Iterable` fulfillment to `runtime-loop contexts`, a term defined nowhere in DECISION_LOG or SPEC; ch12's own axis is `compile-time-unrolled` vs `runtime`, not `runtime-loop context`.

- **Severity/Category/Verdict:** LOW / undefined_term / CONFIRMED
- **Anchors:** LOG 034-12, 014-58, 014-65, 014-70 · SPEC § 13.20.4.2, 12.3.7
- **Why it is a defect:** 014-12 through 014-72 classify loops on a single axis: compile-time-unrolled (iterable is compile-time known) vs runtime (014-58, 014-70). ch12 has no notion of a `runtime-loop context` as a place. The string `runtime-loop context(s)` appears nowhere in either document except 034-12 and its SPEC mirror (§13.20.4.2, line 23087). The restriction 034-12 states is real (a structural compile-time `for` over `yielded` is already a compile error per 034-11), but it is phrased with an undefined term rather than pointing at 014-58/014-71's existing compile-time-vs-runtime machinery. An implementer must guess whether `runtime-loop context` means anything beyond `the loop is a runtime loop`.
- **Direction of change:** Either define `runtime-loop context` in ch12 terms, or restate 034-12/§13.20.4.2 using the existing compile-time-unrolled-vs-runtime vocabulary (014-58, 014-71) so no new undefined term is introduced.
- **Evidence check:** pass — 'runtime-loop context' term undefined; not the ch12 compile-time-vs-runtime axis.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:4363-4363`
    > usable only in runtime-loop contexts such as function, derived, and operator bodies, and it is not a dynamic-view consumer. (§13.20)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1697-1697`
    > 014-58. A `for` loop is compile-time-unrolled iff its iterable is compile-time known per §2.4.1; otherwise it runs at runtime. (§12.3.7)
  - `packages/ductus-lang/docs/SPEC.md:23087-23090`
    > the `T` member values in walk order. This fulfillment is available in **runtime-loop
    > contexts only** — `fn`, `derived`, and `operator` bodies where a runtime
    > `for` walks the live members — never in compile-time structural `for`
    > (above).

#### F254 — 017-105 states as a universal that only `dynamic`/`repeat` change membership and gates never do, but 034-3/022-72 make a gate change `yielded` membership (a gated-off yield is absent).

- **Severity/Category/Verdict:** LOW / contradiction / CONFIRMED
- **Anchors:** LOG 017-105, 034-3, 034-10 · SPEC § 13.3.3, 13.20
- **Why it is a defect:** 017-105 asserts a language-wide generalization — gates freeze rather than change membership; only `dynamic`/`repeat` change membership. In the collect/yielded world that generalization is false: a `yield` inside a gated arm is present iff effectively active and ABSENT when gated off (034-3, 022-72), so a gate — not `dynamic`/`repeat` — drives a `yielded` membership change (034-10 treats that as a membership change propagating dirt). The claim is scoped to views but written as universal contrast, so it reads as untrue when set beside §13.20. No behavioral hole: views and `yielded` are disjoint constructs with no shared counter, so no program computes a wrong count. It is terminology divergence, not an unsatisfiable rule pair.
- **Direction of change:** Narrow 017-105's contrast to the view/exposition domain (gated CHILDREN stay counted), or acknowledge that gates DO change membership for `yielded` groups, so the 'only dynamic/repeat change membership' generalization does not hold language-wide.
- **Evidence check:** pass — 017-105 says only dynamic/repeat change membership and gates never do, but 034-3/022-72 make a gate change yielded/fold membership.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2246-2246`
    > 017-105. Views count gated-but-frozen children and reads return their frozen values — gates freeze propagation rather than remove existence — distinct from `dynamic`/`repeat`, which change membership. (§13.3.3)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4354-4354`
    > 034-3. Membership is positional: a `yield` at a static position is a permanent member; a `yield` inside a `repeat` is a key-driven member, present once per repetition key; a `yield` inside a gated arm is an activation-driven member, present exactly when that arm is effectively active. (§13.20)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2956-2956`
    > 022-72. A value contribution yielded from a gated position joins a fold as an activation-driven member: it is present in the fold exactly while its position is effectively active — its own gate and every ancestor gate open — and absent otherwise; this is a distinct gate read-path from a frozen direct read, which returns a held value rather than dropping from membership. (§13.9.7)

#### F242 — 033-80 introduces the `fold` cell kind but no entry in section 033's lowering table (033-59) states what surface form produces a fold-kind cell, leaving the kind's origin unstated within the IR section.

- **Severity/Category/Verdict:** LOW / gap / CONFIRMED
- **Anchors:** LOG 033-80, 033-59, 033-82 · SPEC § 15.4.1
- **Why it is a defect:** 033-59's surface-to-cell lowering table names five surface constructs (signal/attr/derived/recurrent/const) and maps them to the `cell` primitive; none is stated to yield `kind=fold`. 033-80 lists `fold` as a possible kind and 033-82 says its surface 'lives in a `derived` declaration', but no in-section rule states the trigger 'a `derived` whose RHS is a `fold` expression carries kind=fold'. The connecting rule (035-9/035-10) is in section 035, outside this IR section. An implementer reading section 033 alone knows the `fold` kind exists but not how to assign it. Minor because section 035 does resolve it elsewhere in the LOG.
- **Direction of change:** Add (or point to) a rule in the 033-59 lowering vicinity stating which surface form yields a fold-kind cell, so section 033's kind enumeration is self-contained; do not decide the exact wording unilaterally.
- **Evidence check:** pass — No in-section rule states which surface form produces a fold-kind cell.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:4192-4192`
    > 033-80. A cell entry carries a `kind` — `input` (a stored, externally-written cell), `derived`, `recurrent`, or `fold` — classifying it; the kind leads the cell's text-form declaration. (§15.4.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4171-4171`
    > 033-59. Surface `signal`, `attr`, `derived`, `recurrent`, and `const` all lower to `cell`. (§15.4.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4194-4194`
    > 033-82. A `fold`-kind cell's surface is unchanged: its result lives in a `derived` declaration and consumers see `derived T`; membership changes on its member edges propagate dirt exactly as member value changes do. (§15.4.1)

#### F216 — 035-5 claims membership in 'the family of the loop and collection cost rules', but the LOG has collection cost rules while no loop cost rule states an O() bound to be in a family with.

- **Severity/Category/Verdict:** LOW / divergence / CONFIRMED
- **Anchors:** LOG 035-5 · SPEC § 13.21
- **Why it is a defect:** Citation-aptness: 035-5 asserts kinship with 'the loop and collection cost rules'. Collection cost rules exist as normative O() statements (012-108 Map O(1)/O(n); 025-41 Vec O(log32 n)). But the loop machinery in section 014/§12 carries no normative O() cost rule — 014-149 speaks only qualitatively (no heap alloc, no per-iteration call cost). So the 'loop ... cost rules' half of the cited family has no LOG referent, making the family claim partly unanchored. Low impact because the O(log n) bound in 035-5 is self-contained.
- **Direction of change:** Either point 035-5 only at the cost rules that actually state O() bounds (the collection cost rules), or add/identify the loop cost rule it means to sit beside.
- **Evidence check:** pass — 'loop cost rules' family half has no normative O() LOG referent.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:4372-4372`
    > 035-5. The fold cost is normative, in the family of the loop and collection cost rules: O(log n) combines per member value-change, join, or leave, over a deterministic combine tree fixed by member order. (§13.21)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1788-1788`
    > 014-149. The required loop code has no heap allocation, no iterator-object lifecycle overhead, and no per-iteration function-call cost. (§12.11)

#### F215 — Three distinct order terms — 034-8 'walk order', 035-5/035-10/033-81 'member order', 035-6 'declared order' — are used for the fold/yielded ordering with no statement that they coincide.

- **Severity/Category/Verdict:** LOW / vague_term / CONFIRMED
- **Anchors:** LOG 034-8, 035-5, 035-6, 035-10 · SPEC § 13.20, 13.21
- **Why it is a defect:** The combine-tree determinism in 035-5 hinges on 'member order'. For a yielded-group fold the member order is 034-8's 'walk order'; for a composite fold 035-6 calls it 'declared order'; 035-10/033-81 call it 'member order' generically. Nothing states walk order = member order = declared order, so the single normative order that fixes fold determinism is spelled three ways. A reader cannot confirm the tree is deterministic across membership churn without knowing which order 'member order' resolves to in the yielded case.
- **Direction of change:** Pick one order term for the fold/yielded member ordering and define the others in terms of it (e.g. member order := walk order for yielded groups, := declared slot order for composites), so 035-5's determinism has a single referent.
- **Evidence check:** pass — Three ordering vocabularies for fold determinism with no stated equivalence.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:4372-4372`
    > 035-5. The fold cost is normative, in the family of the loop and collection cost rules: O(log n) combines per member value-change, join, or leave, over a deterministic combine tree fixed by member order. (§13.21)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4373-4373`
    > 035-6. A fold ranges over `yielded` groups and over reactive composites with a uniform slot type; its members are the slots in declared order, and zero slots yields the `else:` result. (§13.21)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4359-4359`
    > 034-8. A `yielded T` is the ordered, walk-order, membership-varying group of cells that a `collect` produces. (§13.20)

#### F214 — 035-9 frames `fold` as a 'new cell kind' it 'introduces/extends' the enum with, yet 033-80 already lists `fold` as a present enum member — steady-state vs. introduction tension.

- **Severity/Category/Verdict:** LOW / divergence / CONFIRMED
- **Anchors:** LOG 035-9, 033-80 · SPEC § 13.21, 15.4.1
- **Why it is a defect:** The LOG is a flat set of simultaneously-true decisions, not a changelog. 033-80 already presents the enum as containing `fold` as a settled member, while 035-9 speaks of 'introduces a new cell kind fold, extending the enum'. Both entries co-exist at the same pin, so the 'new/introduces/extending' framing reads as a stale-amendment artifact describing a transition that, in the current LOG, has no before-state. Endpoints agree (both land on the 4-member enum), so no implementer is blocked, but the narrative wording contradicts the decision-of-record's steady-state stance.
- **Direction of change:** Restate 035-9 in steady-state terms (the fold-kind cell is one of the enum's members input|derived|recurrent|fold) rather than as an introduction/extension of a prior enum.
- **Evidence check:** pass — 035-9's 'introduces/extends' framing conflicts with 033-80 presenting fold as settled.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:4376-4376`
    > 035-9. IR lowering of a fold introduces a new cell kind `fold`, extending the cell kind enum to `input | derived | recurrent | fold`, while the count of six graph primitives is unchanged. (§13.21)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4192-4192`
    > 033-80. A cell entry carries a `kind` — `input` (a stored, externally-written cell), `derived`, `recurrent`, or `fold` — classifying it; the kind leads the cell's text-form declaration. (§15.4.1)

#### F099 — The synthesized `.count` member of a yielded group (034-13) has no IR cell entry stated, unlike stream observation cells which are enumerated in the IR (033-111).

- **Severity/Category/Verdict:** LOW / gap / CONFIRMED
- **Anchors:** LOG 034-13, 033-111 · SPEC § 13.20, 15.4.1
- **Why it is a defect:** Streams enumerate their synthesized observation cells (including `pending_count`) as named IR fields (033-111), and those cells are declared reactive state (032-86). A yielded group's `.count` is likewise reactive (it changes as membership does, per SPEC §13.20.4) and must be dirty-tracked, but no IR rule states it is a cell entry, where it lives, or how consumers form a dependency edge on it. An implementer wiring a consumer that reads `group.count` has no IR anchor. LOW because it is a narrow sub-case of the broader yielded-lowering gap (finding 1).
- **Direction of change:** State that a yielded group's `.count` is a synthesized cell (kind and id), parallel to the stream observation-cell enumeration in 033-111.
- **Evidence check:** pass — Synthesized .count has no IR cell anchor unlike enumerated stream observation cells.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:4364-4364`
    > 034-13. A `yielded` group has a synthesized `count` member giving its element tally, following the naming rule that a bare `count` names an element tally. (§13.20)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4223-4223`
    > 033-111. A stream entry's `observation_cell_ids` name the synthesized observation cells: `pending_count`, `pressure`, `is_full`, `dropped_total`, `rejected_total`, `last_overflow_at`. (§15.4.1)

#### F098 — `repeat` over a yielded group has no keying/lowering rule; 033-61/033-74 key a dynamic scope by `keyed_by`, but a yielded group's membership drivers (permanent, keyed-template, gate-guarded) do not all supply a key.

- **Severity/Category/Verdict:** LOW / gap / CONFIRMED
- **Anchors:** LOG 034-11, 033-61, 033-74, 034-3 · SPEC § 15.4.1, 13.20, 13.5.4.1
- **Why it is a defect:** If `repeat` over a yielded group is legal (034-11), its lowering is a dynamic scope keyed by `keyed_by` (033-61/74). But a yielded group's members are driven by permanent/keyed-template/gate-guarded drivers (035-10, 034-3); permanent and gate-guarded members carry no repetition key. No rule states what identity keys the per-member scopes when repeating a yielded group, so the diff/mount/unmount identity for repeat-over-yielded is undefined. Dependent on the resolution of the 034-11-vs-018-37 contradiction; LOW because it is downstream of that decision.
- **Direction of change:** If repeat-over-yielded is retained, state the keying identity for a yielded group's members under `keyed_by` (e.g. walk-order position or member-cell path); otherwise this resolves with the contradiction above.
- **Evidence check:** pass — repeat-over-yielded scope identity/keying undefined for non-key-driven members.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:4173-4173`
    > 033-61. `repeat` lowers to a dynamic `scope`. (§15.4.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4186-4186`
    > 033-74. A `dynamic` scope additionally carries a `keyed_by` identity for `repeat`. (§15.4.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4354-4354`
    > 034-3. Membership is positional: a `yield` at a static position is a permanent member; a `yield` inside a `repeat` is a key-driven member, present once per repetition key; a `yield` inside a gated arm is an activation-driven member, present exactly when that arm is effectively active. (§13.20)

### Structure layer: unsatisfiable repeat keying, Handle/WeakHandle swap, views

Six HIGHs: keyed-by-index rejected by its own type bound, legal Map keys with no keying path, LOG-vs-SPEC swap of which handle freezes, dynamic view both Iterable and un-for-able, unbounded structural recursion, and two different live-graph closures. Plus empty-bundle ambiguity, connection-body divergences, and interpretation-context match conflicts.

#### F083 — No rule bounds structural (containment) self-recursion: a node whose expose: emits its own type (type-emitted for, direct placement, or wrapper) instantiates unboundedly, yet nothing declares it a compile error.

- **Severity/Category/Verdict:** HIGH / gap / CONFIRMED
- **Anchors:** LOG 017-27, 017-56, 017-63 · SPEC § 13.3.3, 13.3.3.3, 2.3.3
- **Why it is a defect:** 017-27's termination claim is justified in SPEC 13.3.3 only for acceptance-based recursion (caller supplies finite children, so the placement tree is finite). But 017-56/017-63 let a node type EMIT children of its own type at instantiation with no caller act to bound them. `node T: expose: T` (or a type-emitted `for i in 0..1: T`) makes every instantiation of T place another T -> infinite containment closure -> elaboration does not terminate. 004-45's interpretation-closure-is-finite argument PRESUPPOSES finite containment; it does not establish it. Grep found no infinite-size/recursive-type/cyclic-containment guard anywhere in either doc. An implementer cannot decide whether to reject, and if it does not, the compiler hangs — per 001-6 the only legal boundaries are std-delegation or implementation-defined, neither declared here.
- **Direction of change:** Either add a rule making structural self-emission (a node type-emitting or wrapper-placing its own type, directly or transitively) a compile error of a named diagnostic class, or scope 017-27's termination claim to acceptance-only recursion and declare structural self-recursion's status.
- **Evidence check:** pass — 017-56/017-63 let a node's expose: emit children of its own type at every instantiation with no caller act to bound them, but the sole termination argument (SPEC 13307-13309) covers only caller-supplied acceptance recursion; nothing bounds structural self-emission or declares it an error, so elaboration may not terminate.
- **Charity check:** sustain — Confirmed gap; no legal boundary. 017-54 (LOG 2195) 'A node type may emit child instances directly via a compile-time for written in its expose: block' + 017-56 (2197) 'materialized by every instance at instantiation' make `node T: expose: for i in 0..1: T` well-formed by every stated rule — the type-emitted-for section (SPEC 13410-13427) constrains only that the iterable be compile-time-known, never the emitted element TYPE. Materializing this emits a T inside every T -> infinite instantiation at elaboration. The termination text (SPEC 13307-13309) justifies 017-27 ONLY for caller-supplied/acceptance placements ('each placement is an explicit user act — the compiler walks finite placement trees'); type-emitted self-emission is not a caller act. 004-45's carve-out (SPEC 520-528) asserts the interpretation closure is finite because it is 'the containment closure union wire-candidate envelopes' — it PRESUPPOSES a finite containment closure, does not establish it. I grepped both docs for any containment-acyclicity / well-founded / self-emission / finite-containment guard: none exists; the only acyclicity rule is 003-54 (cross-module reference graph, LOG 157), which does not govern structural containment. Per 001-6 the only legal boundaries are std-delegation or implementation-defined; neither is declared for this case, so an implementer cannot decide to reject and the compiler can hang. | none found; SPEC.md:13307-13309 'Self-recursive placements terminate because each placement is an explicit user act — the compiler walks finite placement trees, not infinite type recursions' covers only caller/acceptance placements, and SPEC.md:520-528 finite-closure argument presupposes rather than proves finite containment. No guard on type-emitted self-emission exists in either document.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2168-2168`
    > 017-27. Self-recursive placements terminate. (§13.3.3)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2197-2197`
    > 017-56. A type-emitted `for` is compile-time-unrolled; its placements become children of the type itself, materialized by every instance at instantiation: `for i in 0..VOICE_COUNT: Oscillator | freq=base_freq(i)`. (§13.3.3.3)
  - `packages/ductus-lang/docs/SPEC.md:13307-13309`
    > Self-recursive placements terminate because each placement is an
    > explicit user act — the compiler walks finite placement trees, not
    > infinite type recursions.
  - `packages/ductus-lang/docs/SPEC.md:520-527`
    > **Carve-out — compile-time interpretation expansion.** Compile-time
    > expansion of an in-language interpretation walk (§13.3.7.7) is exempt from
    > this ban. When an interpreter recurses over `.exposition`, the recursion is
    > bounded by the **finite interpretation closure** — the static set of
    > instances an interpretation root can reach (containment closure union
    > wire-candidate envelopes, §13.19). Because that closure is finite and
    > statically known, the expansion produces a finite instantiation set and
    > terminates

#### F255 — The `dynamic view T` kind is declared to satisfy `Iterable`/`IntoIterable`, yet `for` iteration over it is forbidden; satisfying those traits auto-enables `for`, so an implementer cannot honor both rules.

- **Severity/Category/Verdict:** HIGH / contradiction / CONFIRMED
- **Anchors:** LOG 017-192, 017-81, 014-125, 014-135, 014-2, 014-3 · SPEC § 13.3.3.4, 12.8.3, 12.9.4, 12.1
- **Why it is a defect:** 014-2/014-3/014-125/014-135 make `for`/`for own` dispatch through `Iterable`/`IntoIterable` automatically for any type that satisfies them, with no built-in per-type iteration and no stdlib privilege (014-141). 017-192 asserts the `dynamic view T` kind satisfies BOTH traits. 017-81/017-82/017-83 then forbid `for`, indexing, and key-addressing on that same kind. There is no stated carve-out that a type may satisfy `Iterable`/`IntoIterable` yet reject surface `for`; per 001-6 the only legal ways to stop short are stdlib-delegation or implementation-defined behavior, neither of which is invoked here. An implementer given `for x in dynamic_view:` cannot simultaneously (a) dispatch it via the satisfied `Iterable` impl per 014-125 and (b) reject it per 017-81. The intended reconciliation (only operators/`repeat` iterate, internally) is never written as a rule.
- **Direction of change:** Either state that the `dynamic view T` kind does NOT satisfy the user-surface `Iterable`/`IntoIterable` dispatch (it is consumed only by operators/`repeat`), or carve out an explicit exception to 014-125/014-135 for language-level kinds that satisfy the traits internally but block surface `for`. Bring the choice to the user; do not resolve unilaterally.
- **Evidence check:** pass — 017-192 declares the 'dynamic view T' kind satisfies Iterable/IntoIterable; 014-125/014-135 make satisfying those traits auto-enable 'for'/'for own' with no carve-out; 017-81 forbids 'for' on that same kind. Given 'for x in dynamic_view:', an implementer cannot both dispatch via the satisfied impl (014-125) and reject per 017-81 — jointly unsatisfiable, HIGH.
- **Charity check:** sustain — The dynamic-view kind is stated to satisfy `Iterable` AND `IntoIterable` (017-192 at DECISION_LOG.md:2333; SPEC.md:13641), while 014-2/014-125/014-135 make `for`/`for own` dispatch automatically through those traits and 014-3 forbids any type-specific built-in iteration and 014-141 grants no stdlib privilege. 017-81 (DECISION_LOG.md:2222) then forbids `for` on the same kind. There is NO stated general rule that a type may satisfy `Iterable`/`IntoIterable` yet reject surface `for`, and 001-6's only legal boundaries (std-delegation, implementation-defined) are not invoked for the `for`-rejection (only storage is implementation-defined). The one candidate reconciliation, SPEC.md:13615-13616 ('`for` cannot iterate a dynamic view (`for` is compile-time, §12.3.7, and the set is not a compile-time fact)'), does NOT dissolve the contradiction: §12.3.7 (SPEC.md:10315-10320) says a `for` is compile-time-unrolled ONLY iff its iterable is compile-time known and OTHERWISE runs at runtime per §12.3.1, and runtime `for` dispatches precisely through the satisfied `Iterable` impl (SPEC.md:10158-10161). So 13615's premise is false for runtime `for`, and it is not a carve-out. Given `for x in dynamic_view:`, 014-125 forces dispatch through the satisfied Iterable impl and 017-81 forces rejection -- jointly unsatisfiable. Sustain; the SPEC's own attempted reconciliation is itself inconsistent with §12.3.7/§12.3.1.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2333-2333`
    > 017-192. The `dynamic view T` kind is a language-level non-array iterator-shaped kind with `Item = WeakHandle[T]`. Its storage is implementation-defined: no surface index operator, but it exposes `.count` as the bare element tally. It satisfies `Iterable` and `IntoIterable`, and produces `WeakHandle[T]` elements on iteration.
  - `packages/ductus-lang/docs/DECISION_LOG.md:2222-2222`
    > 017-81. `for` cannot iterate a dynamic view. (§13.3.3.4)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1764-1764`
    > 014-125. A type implements `Iterable` by declaring `type Iter` and an `iterator` method; `for x in d:` then dispatches to the implementation automatically. (§12.8.3)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1774-1774`
    > 014-135. A type implements `IntoIterable` by declaring `type Iter` and a `consuming_iterator` method; `for own x in d:` then dispatches automatically, consuming `d`. (§12.9.4)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1642-1642`
    > 014-3. All iteration dispatches through the trait protocol; no type has built-in iteration logic specific to it. (§12.1)
  - `packages/ductus-lang/docs/SPEC.md:13641-13643`
    > `Item = WeakHandle[T]`, satisfies `Iterable` and `IntoIterable`, exposes
    > a reactive `.count` element tally but **no index operator** on its own —
    > storage is implementation-defined. A `dynamic view T` is consume-only:

#### F228 — SPEC §13.8.5.1 attributes the Option[&N] read and freeze-on-None to `Handle[N]`, but LOG 021-82/019-52/019-53 and SPEC §13.3.6.2 assign Option[&N]/freeze exclusively to `WeakHandle[N]` and make `Handle[N]` auto-deref to &N and never freeze.

- **Severity/Category/Verdict:** HIGH / contradiction / CONFIRMED
- **Anchors:** LOG 021-82, 021-83, 019-52, 019-53 · SPEC § 13.8.5.1, 13.3.6.2, 13.6.2
- **Why it is a defect:** The §13.8.5.1 placement elaboration (021 pair) directly contradicts the §13.6.2 connection semantics (019 pair) and §13.3.6.2 on which designation reads as Option[&N] and which freezes. §13.8.5.1 says `Handle[N]` reads Option[&N] and freezes on None; but 019-52/53, 021-82/83, and §13.3.6.2 (14227-14231) all say `Handle[N]` auto-derefs to &N and NEVER freezes, and it is `WeakHandle[N]` that reads Option[&N] and freezes on None. An implementer reading §13.8.5.1 would build a freezing/resolving Handle destination; reading the other sections would build a Handle that unconditionally auto-derefs and a separate WeakHandle that freezes. The two readings produce different runtime liveness for the same destination designation. This is the exact seed thread (does a wire-following derived follow the borrow or the placed instance; do connection-endpoint and placement-borrow lifetimes compose): SPEC §13.8.5.1 mislabels the storable persist/re-point designation, collapsing the Handle/WeakHandle static/dynamic split that 019 and 021 otherwise agree on.
- **Direction of change:** Correct SPEC §13.8.5.1 (lines ~16959-16963) to name `WeakHandle[N]` (not `Handle[N]`) as the persist/re-point destination that reads Option[&N] and freezes on None, matching LOG 021-82/83, 019-52/53 and SPEC §13.3.6.2; or surface to the user if the intent was to broaden Handle. Do not resolve unilaterally.
- **Evidence check:** pass — SPEC §13.8.5.1 attributes Option[&N]-read and freeze-on-None to Handle[N]; LOG 019-52/53, 021-82/83 and SPEC §13.3.6.2 assign those exclusively to WeakHandle[N] and make Handle[N] auto-deref and never freeze. Jointly unsatisfiable for an implementer building the destination carrier.
- **Charity check:** sustain — SPEC §13.8.5.1 (16961-16963) attributes Option[&N] read + freeze-on-None to `Handle[N]`, but SPEC §13.3.6.2 (14227-14233) itself states `Handle[T]` auto-derefs to &T directly and never freezes, while `WeakHandle[T]` reads Option[&T]; and SPEC §13.6.2 mirror (LOG 019-52/019-53) makes a `Handle[N]` destination 'unconditionally active', never freezing. So the two SPEC sections contradict on which designation freezes. Confirmed verbatim. An implementer following §13.8.5.1 builds a resolving/freezing Handle destination; following §13.3.6.2/§13.6.2 builds a Handle that unconditionally auto-derefs and a distinct WeakHandle that freezes — different runtime liveness for the same destination. Not dissolvable: the 'Handle' umbrella (017-182) only covers the bare unbracketed word, not `Handle[N]`. HIGH: an implementer cannot satisfy both sides.
- **Evidence:**
  - `packages/ductus-lang/docs/SPEC.md:16959-16963`
    > A borrow cannot be stored in a cell (§11.9.1), so a destination that must persist or re-point across time is
    > supplied as a `Handle[N]` (§13.3.6.2) — the storable designation, whose read
    > is `Option[&N]`. The connection points at the contained node while that
    > resolution is `Some` and **freezes** while it is `None` (§13.9.7).
  - `packages/ductus-lang/docs/DECISION_LOG.md:2821-2822`
    > 021-82. A connection endpoint is a borrow; a destination that must persist or re-point across time is supplied as a `WeakHandle[N]`, whose read is `Option[&N]`. (§13.8.5.1)
    > 021-83. A `WeakHandle` destination points at the contained node while the resolution is `Some` and the connection freezes while it is `None`. (§13.8.5.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2668-2669`
    > 019-52. A `WeakHandle[N]` destination (read type `Option[&N]`) is unwrapped at the boundary: while it resolves to `Some`, the connection is active and the body sees the contained node as `to`. A `Handle[N]` destination (statically placed) auto-derefs to `&N` directly — its statical-placement assertion makes resolution unnecessary, so the connection is unconditionally active. (§13.6.2)
    > 019-53. While a `WeakHandle[N]` destination resolves to `None`, the connection freezes and its body does not run at all. A statically-placed `Handle[N]` destination never freezes. (§13.6.2)
  - `packages/ductus-lang/docs/SPEC.md:14227-14231`
    > In a read or `&T`-expected position, a **`Handle[T]`** (statically
    >   placed) **auto-derefs to `&T` directly** — its statical-placement
    >   assertion makes resolution unnecessary: `channels[0].gain` is a direct
    >   read.
    > - In a read or `Option[&T]`-expected position, a **`WeakHandle[T]`**

#### F227 — LOG says the entry-point closure includes effect-argument node refs and wire-candidate envelopes; SPEC's two closure passages list only subtree+connection-destinations+Handle-reachable, so a wire-only or effect-arg-only reachable instance is live per LOG but an unreachable-instance compile error per SPEC.

- **Severity/Category/Verdict:** HIGH / contradiction / CONFIRMED
- **Anchors:** LOG 021-140, 021-141 · SPEC § 13.8.1, 13.14.1
- **Why it is a defect:** This is a cross-pair impossibility with a concrete witness. Section 019 wire-following endpoint deriveds (019-59) and dynamic destinations (019-49/10) traverse the wire-candidate envelope, and 021-140/141 make both of those envelope members live/initi
alized. But SPEC §13.8.1 and §13.14.1 both define the closure with only three members, omitting effect-argument borrows and wire-candidate envelopes. Consider a top-level instance reachable ONLY as one type in a dynamic destination's candidate envelope (or ONLY as an effect argument): LOG says it is initialized and live; both SPEC passages classify it as `unreachable_top_level_instance` (a compile error). An implementer cannot satisfy both — the same program is legal under LOG and rejected under SPEC. SPEC must conform to LOG (edit protocol), so both SPEC closure enumerations are the divergent side.
- **Direction of change:** Bring SPEC §13.8.1 and §13.14.1 closure enumerations into conformance with LOG 021-140/141 by adding the two missing members (effect-argument node references reached as borrows, and connections' wire-candidate envelopes); or surface to the user whether the omission was an intended narrowing of the closure. Do not resolve unilaterally.
- **Evidence check:** pass — The two SPEC closure omissions (effect-arg refs, wire-candidate envelopes) are not merely a smaller set but jointly unsatisfiable: a wire-only or effect-arg-only reachable top-level instance is live per LOG 021-140/141 but a compile error (unreachable_top_level_instance) per SPEC §13.8.1/§13.14.1 — the same program cannot satisfy both.
- **Charity check:** sustain — Same closure divergence as F173, escalated to HIGH with a witness. LOG 021-140/141 make effect-argument node refs and wire-candidate envelopes closure members (live/initialized); both SPEC passages (§13.8.1, §13.14.1) omit them, and §13.8.1 (line 16466) classifies any top-level instance not in the closure as compile error unreachable_top_level_instance. Witness holds on the effect-argument half: 021-140 says such refs are 'reached as borrows, not cell stores', so a node passed ONLY as an effect argument is in none of SPEC's three sets → live per LOG, compile error per SPEC — the same program legal one way and rejected the other. The wire-candidate half of the witness is weaker (024-21: a dynamic reference resolves only to an 'already-placed' node; 024-17: the envelope is a type-level over-approximation), but the effect-argument witness alone sustains the HIGH classification. Sustain.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2879-2880`
    > 021-140. A top-level node instance that is not the entry-point and is not reachable from the entry-point's transitive closure is a compile error of class `unreachable_top_level_instance`. The transitive closure comprises the entry-point's own subtree, its connection destinations, its `Handle`/`WeakHandle`-reachable module-level instances, node references bound as effect arguments (reached as borrows, not cell stores), and the wire-candidate envelopes of its connections. (§13.8.1)
    > 021-141. The entry-point's transitive closure — the entry-point's own subtree, its connection destinations, its `Handle`/`WeakHandle`-reachable module-level instances, node references bound as effect arguments (reached as borrows), and its connections' wire-candidate envelopes — defines the live graph
  - `packages/ductus-lang/docs/SPEC.md:16457-16464`
    > The closure includes: (a) the
    > `main` instance's own child subtree (placements declared in its
    > body, recursively); (b) the destinations of every connection
    > placement within that subtree (§13.8.4); and (c) every module-level
    > top-level instance reachable through a `Handle` or `WeakHandle`
    > (§13.3.6.2) held by any cell in the closure. The transitive closure
    > is the program's *initialization scope*: only reachable instances
    > are instantiated and wired into the runtime graph.
  - `packages/ductus-lang/docs/SPEC.md:18893-18896`
    > Locate the entry-point node instance (the `main` placement,
    >    §13.8.1) and compute its **transitive closure**: the entry-point's
    >    own subtree plus everything reachable through connections and
    >    through module-level `Handle`/`WeakHandle` references.

#### F229 — Six legal Map key types (usize, isize, i128, u128, and per SPEC duration/instant) satisfy Map's key bound but are excluded from StringifiableKey, so a repeat over such a Map has no legal keying path — a compile error with no workaround.

- **Severity/Category/Verdict:** HIGH / unsatisfiable / CONFIRMED
- **Anchors:** LOG 012-101, 018-63, 018-64, 018-42, 018-44 · SPEC § 9.5.3, 13.5.4.1
- **Why it is a defect:** Section 012 (§9.5) admits usize, isize, i128, u128 (and per SPEC 9.5.3 also duration and instant) as valid Map key types. Section 018 (§13.5.4.1) restricts every repeat scope key to StringifiableKey = i8-i64, u8-u64, bool, char, string, which excludes all six. A repeat over Map[K,V] must key by the map key (the element is a (K,V) tuple which is not itself a StringifiableKey and fulfills neither Keyed nor stringifiable-element paths, so paths 3/4 fail; path 2 applies only to dynamic sources). The only usable path is explicit `keyed by k`, but 018-63 requires that result to be a StringifiableKey, which K=usize/isize/i128/u128/duration/instant is not. There is no widening or coercion into StringifiableKey (the bound is a fixed enumerated set; signed/unsigned crossings need explicit casts per 007-133, and i128/duration/instant have no lossless StringifiableKey target). So these Maps are legal to construct but impossible to repeat over. This directly contradicts 018-42, which prescribes projecting a stream into a Map[K,V] and repeating over it as the canonical stream-to-repeat path. An implementer cannot honor both 'Map[usize,V] is a legal Map' and 'every repeat source with derivable keys can drive repeat'.
- **Direction of change:** Reconcile the two key-type sets: either widen StringifiableKey (018-64 / SPEC 13.5.4.1) to cover the full integer range and any other Map-eligible key types intended to drive repeat, or narrow the Map-repeat guarantee / 018-42's prescription so it does not promise every legal Map can drive repeat. Requires a user decision on which set is authoritative; do not resolve unilaterally.
- **Evidence check:** pass — usize/isize/i128/u128 (and per SPEC duration/instant) are legal Map keys per §9.5 but excluded from StringifiableKey (018-64), and none of the four repeat key-derivation paths (018-62/65/68/71) apply to a Map[K,V] with such K, so 018-72 forces a compile error — yet 018-42 prescribes Map projection as the canonical stream-to-repeat path. Jointly unsatisfiable.
- **Charity check:** sustain — Verified the full key-derivation precedence in both docs. LOG 018-62/63/64 (lines 2532-2534) and SPEC 15350-15353: explicit `keyed by` result MUST be StringifiableKey = i8-i64,u8-u64,bool,char,string. I confirmed via grep of all 8 SPEC StringifiableKey mentions that there is NO widening/coercion rule broadening this fixed enumerated set (15352 restates it verbatim). Path 3 Keyed (018-68) applies to the element type; a Map element is a (K,V) tuple, no Keyed. Path 4 (018-71) requires the element itself be StringifiableKey; a tuple is not. So for Map[usize/isize/i128/u128,V] (and per SPEC 9.5.3 duration/instant) the only path is explicit `keyed by k`, and k's type is not in StringifiableKey — no lossless cast target exists (007-133 forbids implicit signed<->unsigned crossing; i128/duration/instant have no StringifiableKey target). 012-101 admits usize/isize/i128/u128 as legal Map keys and 018-42 (line 2512) prescribes 'project a stream into a Map[K,V] and repeat over it' as the canonical path. An implementer cannot satisfy both 'Map[usize,V] is legal' and 'a repeat over it has a legal keying path'. HIGH: implementer-blocking, no legal boundary (001-6). Sustained.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1296-1296`
    > 012-101. Keys must satisfy `K: Eq + Hash`. Built-in numerics (integers), `bool`, `char`, and `string` qualify; floats (`f32`/`f64`) do not, so `Map[f32, V]` is a compile error at the `Hash` bound. (§9.5)
  - `packages/ductus-lang/docs/SPEC.md:7311-7318`
    > Keys must satisfy `K: Eq + Hash`. The Ductus primitive types that
    > qualify are:
    > 
    > - All integer types (`i8`..`i128`, `u8`..`u128`, `isize`, `usize`).
    > - `bool`.
    > - `char`.
    > - `string`.
    > - `duration` and `instant`.
  - `packages/ductus-lang/docs/DECISION_LOG.md:2533-2534`
    > 018-63. An explicit `keyed by` result must be a `StringifiableKey`, the language-defined key bound. (§13.5.4.1)
    > 018-64. `StringifiableKey` comprises `i8`–`i64`, `u8`–`u64`, `bool`, `char`, and `string`. (§13.5.4.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2512-2512`
    > 018-42. Driving `repeat` from a stream requires first projecting the stream into a collection-valued cell (e.g. folding events into a `Vec` or `Map[K, V]`) and repeating over that. (§13.5.4.1)
  - `packages/ductus-lang/docs/SPEC.md:15478-15490`
    > **`Map` source with destructuring bind** — `Map[K, V]`
    > iterates as `Iterable` yielding `(K, V)` pairs. Move-promotion
    > (§13.5.4.1) gives the bind an owned `(K, V)`, so ordinary tuple
    > destructuring (§12.12.1) binds owned `sid` and `info`; `keyed by`
    > names the map key as the scope key:
    > 
    > ```
    > node SessionPanel:
    >   attr sessions: Map[SessionId, SessionInfo] = {}
    >   expose:
    >     repeat (sid, info) in sessions keyed by sid:
    >       SessionRow | id=sid info=info
    > ```

#### F163 — The prescribed remedy for positional identity, `keyed by <index>`, is rejected by its own type bound: `<index>` is `usize` but `usize` is not a `StringifiableKey`.

- **Severity/Category/Verdict:** HIGH / contradiction / CONFIRMED
- **Anchors:** LOG 018-58, 018-63, 018-64, 018-127, 018-142 · SPEC § 13.5.4.1, 13.5.4.8, 13.5.4
- **Why it is a defect:** 018-127 and 018-142 tell the user the way to get positional identity is to write `keyed by <index>`. The `at` index is typed `usize` (018-58). 018-63 requires a `keyed by` result to be a `StringifiableKey`, and 018-64 enumerates the `StringifiableKey` members as `i8`–`i64`, `u8`–`u64`, `bool`, `char`, `string` — `usize` (and `isize`) are absent from that list, and no other in-scope or upstream entry adds them. An implementer cannot satisfy both sides: the entry that prescribes `keyed by <index>` and the entry that bounds `keyed by` on `StringifiableKey` jointly reject the exact program the docs instruct the user to write. Grep confirms no `usize`/`isize` StringifiableKey rule anywhere in the LOG.
- **Direction of change:** Reconcile the `StringifiableKey` membership (018-64) with the prescribed positional-identity remedy (018-127/018-142): either the width list must be decided to include the index's type, or the remedy must be restated to require an explicit conversion — this is a user decision, surface it rather than resolve it.
- **Evidence check:** pass — 018-127 prescribes 'keyed by <index>' as the sole path to positional identity; the at-index is usize (018-58); 018-63 requires the keyed-by result be a StringifiableKey; 018-64 omits usize/isize — the docs instruct the user to write a program the type bound rejects.
- **Charity check:** sustain — The prescribed remedy for positional identity is 'keyed by <index>' (018-127; SPEC line 15656-15657 'must opt in explicitly by writing keyed by <index>'), and the at-index is typed usize (018-58; SPEC line 15340). 018-63 requires a keyed-by result to be a StringifiableKey and 018-64/SPEC line 15352 enumerate that set as i8–i64, u8–u64, bool, char, string — usize and isize are absent, and grep confirms no upstream rule adds them. Searched for a coercion escape: the widening table (SPEC 3382) widens narrower types TO usize, not usize TO u64 (usize→u64 is platform-dependent, not implicit); and 018-63 is a type-membership test on the result type of keyed by, with no target-type slot to coerce toward. Both sides jointly reject the exact program the docs instruct the user to write. Confirmed contradiction.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2528-2528`
    > 018-58. `at <index>` binds a bare identifier to each element's 0-based enumeration index (`usize`) in iterator order. (§13.5.4.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2533-2534`
    > 018-63. An explicit `keyed by` result must be a `StringifiableKey`, the language-defined key bound. (§13.5.4.1)
    > 018-64. `StringifiableKey` comprises `i8`–`i64`, `u8`–`u64`, `bool`, `char`, and `string`. (§13.5.4.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2597-2597`
    > 018-127. Positional identity requires explicit opt-in by writing `keyed by <index>`; there is no implicit path to index keying. (§13.5.4.8)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2612-2612`
    > for stable positional identity, an explicit `keyed by <key-expr>` is the prescribed remedy.

#### F018 — Within the LOG, 009-89 and 009-90 give conflicting operational models for interpretation-context `match`: 009-89 says it builds-all-and-freezes; 009-90 says it static-unrolls/tags-and-discards and that live-subtree keeping is given's job, not match's.

- **Severity/Category/Verdict:** MED / contradiction / CONFIRMED
- **Anchors:** LOG 009-89, 009-90 · SPEC § 6.2.5, 13.9.13
- **Why it is a defect:** Both entries govern `match` in interpretation context. 009-89 says such a match 'lowers to given semantics (builds all arms and freezes unselected ones)'. 009-90 says live-subtree selection 'remains given, not match', and that an interpretation-context match compiles to a static unroll / mount-time tag — a select-one-discard-rest model, not a build-all-freeze model. An implementer reading 009-89 would keep all arms live-and-frozen; reading 009-90 would build only the selected subtree. The two cannot both hold.
- **Direction of change:** Have the user decide the single intended lowering for interpretation-context match, then restate it identically (atomic, self-contained) in whichever of 009-89/009-90 survive. Do not self-decide.
- **Evidence check:** pass — 009-89 (build-all-arms-and-freeze) vs 009-90 (static unroll / single mount-time tag, live-keeping is given's job not match's) give incompatible operational models for interpretation-context match within the LOG.
- **Charity check:** sustain — 009-89 says an interpretation-context match on a reactive scrutinee lowers to given semantics = builds all arms and freezes unselected. 009-90 says an interpretation-context match over `.exposition` entries compiles to a static unroll / single mount-time tag with 'live-subtree selection remains `given`, not `match`' — a select/tag/discard model where match does NOT keep arms live-frozen. An exposition match is itself a match on a reactive scrutinee (exposition carries DynamicView/Gated reactive kinds, §13.3.7.7), so 009-89's unqualified rule overlaps 009-90's domain and prescribes the opposite lowering; no text carves exposition out of 009-89. The contradiction is reinforced by 034-7 (DECISION_LOG.md:4358) 'since `if`/`match` never gate structure' and §13.9.13 (SPEC.md:18029-18031) assigning build-all-freeze to `given`, making 009-89 the outlier. Sustain.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1026-1026`
    > 009-89. `match` is a value selector: it evaluates the scrutinee, selects one arm, evaluates that arm to a value, and discards the rest; in interpretation context a `match` on a reactive scrutinee lowers to `given` semantics (it builds all arms and freezes unselected ones), but the value-selection meaning is unchanged. (§6.2.5)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1027-1027`
    > 009-90. Selecting which node/connection subtree is exposed and kept live is the role of `given` (§13.9.13), not `match`; a `match` over `.exposition` entries in interpretation context compiles to a static unroll for static entries and a single mount-time tag over the closed candidate envelope for dynamic elements, while live-subtree selection remains `given`. (§6.2.5)

#### F017 — 009-89 says an interpretation-context `match` on a reactive scrutinee 'lowers to given semantics (builds all arms and freezes unselected)', but its cited §6.2.5 says match discards unselected arms and building/freezing all is given's role, not match's.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 009-89 · SPEC § 6.2.5
- **Why it is a defect:** SPEC must conform to LOG, and the cited §6.2.5 must elaborate the LOG entry's claim. 009-89 asserts a reactive-scrutinee match 'builds all arms and freezes unselected ones' (given semantics). The cited §6.2.5 states the exact opposite operation for match ('discards the rest') and explicitly assigns build-all-and-freeze to `given`, not match. The two documents describe different runtime behavior for the same construct; an implementer cannot tell whether an interpretation-context match freezes or discards non-selected arms.
- **Direction of change:** Reconcile the parenthetical in 009-89 with §6.2.5 — decide (with the user) whether interpretation-context match freezes-all (given semantics) or discards, then make LOG and SPEC state one story. Do not self-resolve.
- **Evidence check:** pass — 009-89 says interpretation-context match builds-all-and-freezes (given semantics); its cited §6.2.5 says match discards and only given freezes, and never elaborates the interpretation-context lowering — LOG-SPEC divergence.
- **Charity check:** sustain — 009-89 asserts that in interpretation context a reactive-scrutinee `match` 'lowers to `given` semantics (it builds all arms and freezes unselected ones)'. Its cited §6.2.5 (SPEC.md:5335-5343) describes match ONLY as 'selects one arm … and discards the rest' and assigns build-all-and-freeze to `given` ('the structure-level counterpart that builds all arms and freezes the unselected ones … they differ in operation (discard vs. freeze)'). §6.2.5 never mentions the interpretation-context reactive lowering at all, so it fails to elaborate the LOG entry's own claim — a LOG→SPEC pointer/divergence defect. Charitable search for the lowering in §6.2.5 found nothing. Sustain.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1026-1026`
    > 009-89. `match` is a value selector: it evaluates the scrutinee, selects one arm, evaluates that arm to a value, and discards the rest; in interpretation context a `match` on a reactive scrutinee lowers to `given` semantics (it builds all arms and freezes unselected ones), but the value-selection meaning is unchanged. (§6.2.5)
  - `packages/ductus-lang/docs/SPEC.md:5335-5343`
    > `match` is a **value** selector: it evaluates its scrutinee, selects one
    > arm, evaluates that arm to a value, and discards the rest. It is used
    > everywhere a value is produced — function bodies and reactive `derived`/
    > `recurrent` expressions alike. It is *not* used to gate reactive
    > *structure*: selecting which node/connection subtree is exposed and kept
    > live is the role of the `given` block (§13.9.13), the structure-level
    > counterpart that builds all arms and freezes the unselected ones rather
    > than discarding them. The two share arm shape and this exhaustiveness
    > rule; they differ in operation (discard vs. freeze).
  - `packages/ductus-lang/docs/SPEC.md:18029-18031`
    > arms; `given` builds all arms and freezes the inactive ones (Model B,

#### F186 — 017-90 (citing §4.9.5 for the `Index` trait) says `bundle[g]` returns `Handle[T][..N]` or `Handle[T][..]`, but §4.9.5 pins the Bundle `Index[isize]` `Output` to `Handle[T][..]` only.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 017-90, 017-99, 017-101 · SPEC § 4.9.5
- **Why it is a defect:** 017-90/99/101 claim the row-slice result type varies (`Handle[T][..N]` when the outer index `g` is compile-time-known, `Handle[T][..]` when runtime). But §4.9.5, the section 017-90 explicitly cites for bundle indexing, fixes `Index[isize]` `Output = Handle[T][..]` and states the `Output` is determined by `(Self, K)` — one Output per `(Bundle[T], isize)`. The trait as specified cannot produce a compile-time-length `[..N]` result for a constant index, so LOG and cited SPEC diverge and the trait mechanism cannot express the LOG's index-dependent output.
- **Direction of change:** Reconcile: either §4.9.5 must express the compile-time-length row-slice output the LOG asserts (e.g. a separate access path outside the `Index[isize]` fulfillment), or the LOG must drop the `Handle[T][..N]` compile-time-length claim for `bundle[g]`. Surface to the user.
- **Evidence check:** pass — 017-90 cites §4.9.5 for a variable index-result type (`[..N]` or `[..]`), but §4.9.5 pins Bundle `Index[isize]` Output to `Handle[T][..]` only and makes Output a function of `(Self,K)` — the trait cannot express the LOG's constant-index compile-time-length result. Real LOG-SPEC divergence.
- **Charity check:** sustain — §4.9.5 (the section 017-90 cites for the Index trait) pins Bundle Index[isize] Output = Handle[T][..] ONLY (SPEC:4203-4204) and states Output is determined by (Self,K) (SPEC:4194-4196) — one Output per (Bundle[T],isize). But 017-90/017-99/017-101 and §13.3.3.5 (SPEC:13687-13690) require the row-slice result to vary: Handle[T][..N] for statically-known row length, Handle[T][..] otherwise. The (Self,K)-determined single-Output trait mechanism as written in §4.9.5 cannot express a compile-time-length-dependent output, so §4.9.5 diverges from both the LOG and §13.3.3.5. Hunted the corpus: the [..N] variant is elaborated at §13.3.3.5 (13687, 13711, 13782) but §4.9.5 itself — the cited section — never reconciles it and omits the [..N] case entirely. Real LOG↔cited-SPEC divergence; sustains.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2231-2231`
    > 017-90. Bundle access goes through the `Index` trait (§4.9.5): `bundle[g]` returns a row slice (`Handle[T][..N]` or `Handle[T][..]`), `bundle[g][i]` returns `Handle[T]`, and `bundle.count` returns the row count.
  - `packages/ductus-lang/docs/SPEC.md:4203-4204`
    > - **Bundles `Bundle[T]`** (§13.3.3.5): `Index[isize]` →
    >   `Output = Handle[T][..]` (a row slice).
  - `packages/ductus-lang/docs/SPEC.md:4194-4196`
    > trait identity, so a type fulfilling both `Index[isize]` (for
    > element access) and `Index[Range[isize]]` (for range slicing) carries
    > two distinct trait instances. The `Output` associated type is
    > determined by `(Self, K)`.
  - `packages/ductus-lang/docs/DECISION_LOG.md:2242-2242`
    > 017-101. A whole `Bundle[T]` value (e.g. `chords`) is storable. A row `chords[g]` is a slice — `Handle[T][..N]` for compile-time `g`, `Handle[T][..]` for runtime `g` — a borrow that follows the ordinary borrow rules and is not itself storable.

#### F178 — SPEC §13.7.1 enumerates six reserved instance-body fields (adds `incoming`,`outgoing`) but LOG 020-8/020-20 and SPEC §13.7.5 enumerate exactly four (`from`,`to`,`pair`,`exposition`).

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 020-8, 020-20 · SPEC § 13.7.1, 13.7.5
- **Why it is a defect:** The LOG's atomic reserved-field set is exactly four (`from`,`to`,`pair`,`exposition`); it never lists `incoming`/`outgoing` as reserved instance fields (it uses `incoming:`/`outgoing:` only as clause keywords in 020-8). SPEC 13.7.1 adds `incoming` and `outgoing` to the reserved-field list, diverging from the LOG and self-contradicting SPEC 13.7.5 (line 16312), which lists only four. The 13.7.1 example (`incoming shows: ShowsCount` then `shows[0]`) reads the user-declared view name `shows`, not the keyword `incoming` as a field, so it does not support the six-item list.
- **Direction of change:** Bring SPEC 13.7.1's reserved-field enumeration into conformance with LOG 020-8/020-20 and SPEC 13.7.5 (four fields); confirm with the user whether `incoming`/`outgoing` are meant to be reserved expression-position fields at all before changing the LOG.
- **Evidence check:** pass — SPEC §13.7.1 enumerates six reserved instance-body fields (adds incoming, outgoing) while LOG 020-8/020-20 and SPEC §13.7.5 enumerate exactly four (from, to, pair, exposition) — a LOG-SPEC and SPEC-internal divergence.
- **Charity check:** sustain — SPEC §13.7.1 (line 16188-16190) enumerates SIX reserved endpoint/structure fields — from, to, incoming, outgoing, pair, exposition — and cross-references §13.7.5. But §13.7.5 (line 16312-16314) enumerates exactly FOUR — from, to, pair, exposition — and LOG 020-8/020-20 also enumerate exactly four. Grep found no passage listing incoming/outgoing as reserved instance-body fields resolving by bare name in expression position; they are placement-clause keywords (020-8's children:/incoming:/outgoing: clauses) and connection-view specifiers. SPEC §13.7.1's six-item list is the divergent side, self-contradicting §13.7.5 and diverging from the LOG. Sustain as divergence.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2706-2706`
    > 020-8. The instance body scope comprises cells (`const`/`attr`/`recurrent`/`derived`/`stream`/`yielded`), `collect as x` bindings, `yielded` groups, `view` declarations, named acceptance entries inside `children:`/`incoming:`/`outgoing:` clauses, placement `as`-names, and the reserved fields `from`, `to`, `pair`, and `exposition`. (§13.7.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2718-2718`
    > 020-20. The reserved fields `from`, `to`, `pair`, and `exposition` are instance-body-scope members resolving by bare name in expression position: `from.expertise_level`. (§13.7.5)
  - `packages/ductus-lang/docs/SPEC.md:16186-16190`
    > 2. **The instance body scope** — the node's or connection's members:
    >    `attr`, `recurrent`, `derived`, `stream` cells; `view` and
    >    connection-view declarations; placement `as`-names; and the reserved
    >    endpoint/structure fields (`from`, `to`, `incoming`, `outgoing`, `pair`,
    >    `exposition` — §13.7.5).
  - `packages/ductus-lang/docs/SPEC.md:16312-16314`
    > The reserved fields of a node or connection instance — `from`, `to`
    > (connection endpoints, §13.6), `pair` (§13.6.1.3), and `exposition`
    > (§13.3.7) — occupy

#### F165 — 018-143 defines a normative diagnostic class `bundle_in_repeat_rejected` and a bidirectional rejection rule citing §13.5.4, but §13.5.4 does not elaborate it and the only related SPEC text (§13.3.3.5) covers one direction and omits the class name.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 018-143 · SPEC § 13.5.4, 13.3.3.5
- **Why it is a defect:** 018-143 cites (§13.5.4) and makes two normative claims: (1) a bidirectional rejection (bundle-in-repeat AND repeat-in-bundle), and (2) a targetable normative diagnostic class name `bundle_in_repeat_rejected`. SPEC must conform to LOG. But (a) the class name `bundle_in_repeat_rejected` appears zero times in SPEC.md (grep confirmed; the only normative diagnostic class present in the repeat section is `unstable_positional_iteration_on_unordered_source`), and (b) the only related prose lives in §13.3.3.5 (Bundles), not §13.5.4, and it covers only one direction ('Forbidden inside a bundle bracket: `repeat`') — it never states the reverse (a bundle bracket inside a `repeat` body being rejected) and never names the class. The cited section (§13.5.4) does not actually elaborate the claim, so the LOG→SPEC pointer contract is broken and a normative diagnostic-class name is unelaborated.
- **Direction of change:** Either §13.5.4 must be extended to elaborate 018-143 (both rejection directions and the `bundle_in_repeat_rejected` class name), or 018-143's citation must point at the section that actually elaborates it (§13.3.3.5) — and that section must be brought into conformance (add the reverse direction and the class name). Which citation/section is authoritative is an edit decision to surface.
- **Evidence check:** pass — 018-143 defines a normative, tooling-targetable diagnostic class `bundle_in_repeat_rejected` and a bidirectional rejection, citing §13.5.4. The class name appears zero times in SPEC; the only related prose is one-directional and lives in §13.3.3.5, not the cited §13.5.4. LOG→SPEC pointer broken; normative class unelaborated. Real divergence.
- **Charity check:** sustain — 018-143 cites (§13.5.4) and makes two normative claims: a bidirectional bundle/repeat rejection and a targetable diagnostic class `bundle_in_repeat_rejected`. Verified in the corpus: (1) `bundle_in_repeat_rejected` appears ZERO times in SPEC.md (grep -c = 0); (2) the entire §13.5.4 range (SPEC lines 15227-15790, all of §13.5.4.1–.10) contains ZERO occurrences of 'bundle' or 'bracket' (awk scan empty), so the cited section does not elaborate the rule at all; (3) the only related prose lives in §13.3.3.5 (SPEC:13756-13760) and covers ONE direction only ('Forbidden inside a bundle bracket: `repeat`'), never the reverse (bundle bracket inside a repeat body) and never the class name. LOG→SPEC pointer contract broken + normative diagnostic-class name unelaborated. Sustains (MED divergence).
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2613-2613`
    > 018-143. Bundle brackets `[…]` are not legal in a `repeat`-rejected context: a `repeat` body cannot contain bundle brackets, and a bundle bracket cannot contain a `repeat` (runtime structure inside a static pre-tied bracket contradicts the static-bundle nature). The error class is `bundle_in_repeat_rejected` (a normative diagnostic class name targetable by user tooling); the compiler emits it at parse time at the offending site. The same restriction lifts at the boundary: a bundle bracket *outside* a `repeat` body, or a `repeat` *outside* a bundle bracket, are both legal. (§13.5.4)
  - `packages/ductus-lang/docs/SPEC.md:15756-15759`
    > diagnostic** — class **`unstable_positional_iteration_on_unordered_source`**
  - `packages/ductus-lang/docs/SPEC.md:13756-13760`
    > Forbidden inside a bundle bracket:
    > 
    > - `repeat` — a runtime-varying membership inside a static pre-tied
    >   bracket contradicts the bundle nature. Dynamic bundles use the
    >   reactive `cell` form below.

#### F086 — Empty bundle `[]` is described as both 'zero elements' and 'a single zero-length row', leaving `[].count` (defined as row count) ambiguous between 0 and 1 with no offset-table layout pinned.

- **Severity/Category/Verdict:** MED / parse_ambiguity / CONFIRMED
- **Anchors:** LOG 017-89, 017-90 · SPEC § 13.3.3.5
- **Why it is a defect:** 017-90/SPEC 13692 define `bundle.count` as the ROW count. 017-89/SPEC 13680 describe the empty bundle's offset table as 'length 1 holding a single zero-length row'. Under a standard CSR layout, offsets length = rows+1, so length-1 means 0 rows and `[].count == 0`; but the prose 'a single zero-length row' reads as 1 row, so `[].count == 1`. The offset-table layout (length=rows vs length=rows+1) is never pinned (grep found no such rule), so the two readings both survive. A program branching on `bundle.count == 0` for an empty bundle behaves differently under the two readings. Also collides with the 012-91 'element tally' unification, since for a non-empty bundle `.count` is rows, not elements.
- **Direction of change:** Pin the empty bundle's `.count` explicitly (0 or 1) and state the offset-table length invariant (rows vs rows+1) so row-count is unambiguous; note bundle `.count` counts rows, an intentional exception to the element-tally unification.
- **Evidence check:** pass — Same as F057: empty bundle `[]` described as both 'zero elements' and 'a single zero-length row' with `.count` defined as row count and no offset-table length convention pinned, leaving `[].count` ambiguous between 0 and 1; a program branching on `bundle.count == 0` behaves differently under the two readings.
- **Charity check:** sustain — Confirmed — same empty-bundle ambiguity as F057, verified independently. `[]` is 'zero elements' (SPEC 13679) yet its offset table is 'length 1 holding a single zero-length row' (LOG 017-89 line 2230; SPEC 13680). Under standard CSR (offsets length = rows+1) length-1 -> 0 rows -> `[].count == 0`; the prose 'a single zero-length row' -> 1 row -> `[].count == 1`. The offset-table layout is never pinned (grep confirms no such rule). SEPARATELY, F086's secondary observation is itself a real divergence I verified: 012-91 (LOG 1286) declares `.count` the 'unified element-tally accessor across ... bundles ... it reports the number of elements', but 017-90/SPEC 13692 define `bundle.count` as the ROW count — for a non-empty bundle rows != elements, so 012-91 and 017-90 disagree on what `bundle.count` returns. This adjacent LOG-internal divergence (012-91 vs 017-90) is not the framed defect but is genuine; flagged in notes. Sustained on the framed empty-bundle 0-vs-1 ambiguity. | none found; DECISION_LOG.md:2230 (017-89, 'offset table of length 1 holding a single zero-length row') and SPEC.md:13692 ('bundle.count returns the row count') together leave `[].count` under-determined between 0 and 1, with no offset-table layout rule pinning the relation in either document. Additionally DECISION_LOG.md:1286 (012-91, '.count ... reports the number of elements') conflicts with 017-90's row-count definition — a separate divergence, not a dissolver.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2230-2230`
    > 017-89. A `Bundle[T]` is a homogeneous co-placement of children written `[...]` at placement — a distinct kind from a view; only an explicit `[...]` constructs a bundle, a bare placement is not. The empty bundle literal `[]` is allowed and produces a `Bundle[T]` value for any inferable `T`, with an offset table of length 1 holding a single zero-length row. (§13.3.3.5)
  - `packages/ductus-lang/docs/SPEC.md:13678-13680`
    > An **empty bundle literal `[]`** is legal and has type
    > `Bundle[T]` for any `T` the context can infer. The bundle has zero
    > elements; its offset table has length 1 (a single zero-length row).
  - `packages/ductus-lang/docs/SPEC.md:13692-13693`
    > - `bundle.count` returns the row count; `bundle[g].count` returns the
    >   row's element count.

#### F057 — Empty bundle `[]` is described as both zero elements/zero rows and 'a single zero-length row', leaving `[].count` (defined as row count) and `for row in []` under-determined between 0 and 1.

- **Severity/Category/Verdict:** MED / parse_ambiguity / CONFIRMED
- **Anchors:** LOG 017-89, 017-90 · SPEC § 13.3.3.5
- **Why it is a defect:** Under the stated storage model an offsets table 'delimits each row', so a table delimiting R rows has length R+1; a length-1 offsets table therefore delimits ZERO rows, consistent with 'zero elements'. But both LOG 017-89 and SPEC gloss the same length-1 table as 'a single zero-length row'. Since `bundle.count` is defined as the row count (017-90) and `for row in bundle` iterates once per row (SPEC 13701), the empty bundle yields two contradictory concrete behaviors: `[].count == 0` and zero loop iterations (row interpretation) versus `[].count == 1` and one iteration binding an empty row (single-zero-length-row interpretation). A program branching on `bundle.count == 0` or iterating a possibly-empty bundle behaves differently under the two readings.
- **Direction of change:** Reconcile the empty-bundle description with the offsets-table encoding: decide whether the empty bundle has zero rows or one zero-length row, then make 017-89/017-90 and the SPEC storage+count+iteration text agree on `[].count` and the `for row in []` iteration count.
- **Evidence check:** pass — Empty bundle `[]` is glossed as both 'zero elements' (offsets 'delimits each row' => length-1 table delimits 0 rows) and 'a single zero-length row' (1 row); with `.count` defined as the row count and no offset-table length convention pinned, `[].count` and `for row in []` are under-determined between 0 and 1, changing program behavior.
- **Charity check:** sustain — Confirmed behavior-changing ambiguity. `bundle.count` is defined as the ROW count (SPEC 13692; example 13674 'chords.count // number of bundles'; LOG 017-90). The empty bundle `[]` is described BOTH as 'zero elements' (SPEC 13679) and as having an offset table 'of length 1 (a single zero-length row)' (SPEC 13680, 13715; LOG 017-89). SPEC 13710 says the offsets table 'delimits each row' — CSR-boundary semantics, under which a length-1 table delimits ZERO rows, giving `[].count == 0` and zero `for row in []` iterations. But 'a single zero-length row' reads as ONE row, giving `[].count == 1` and one loop iteration binding an empty row. I grepped both docs for any rule pinning the offsets-length-to-row-count relation (length=rows vs rows+1) — none exists. The lowering rule (SPEC 24039 'the bundle's row count becomes the grouping scope's child count') propagates the ambiguity rather than resolving it. A program branching on `bundle.count == 0` or iterating a possibly-empty bundle behaves differently under the two readings. Note: F057 and F086 are near-duplicates of the same defect (same anchors 017-89/017-90); F057 additionally raises the `for row in []` iteration count. | none found; SPEC.md:13680 ('its offset table has length 1 (a single zero-length row)') and SPEC.md:13679 ('The bundle has zero elements') plus SPEC.md:13710 ('an offsets table that delimits each row') are jointly under-determined for `[].count`, defined as row count at SPEC.md:13692. No rule pins offsets-length-to-row-count anywhere in either document.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2230-2230`
    > 017-89. A `Bundle[T]` is a homogeneous co-placement of children written `[...]` at placement — a distinct kind from a view; only an explicit `[...]` constructs a bundle, a bare placement is not. The empty bundle literal `[]` is allowed and produces a `Bundle[T]` value for any inferable `T`, with an offset table of length 1 holding a single zero-length row.
  - `packages/ductus-lang/docs/DECISION_LOG.md:2231-2231`
    > 017-90. Bundle access goes through the `Index` trait (§4.9.5): `bundle[g]` returns a row slice (`Handle[T][..N]` or `Handle[T][..]`), `bundle[g][i]` returns `Handle[T]`, and `bundle.count` returns the row count. Indexing follows the index-to-min-cardinality rule (`viewname[i]` legal iff `i < min_cardinality`) at each level — outer `[g]` to-min produces the slice, inner `[i]` to-min into it.
  - `packages/ductus-lang/docs/SPEC.md:13678-13680`
    > An **empty bundle literal `[]`** is legal and has type
    > `Bundle[T]` for any `T` the context can infer. The bundle has zero
    > elements; its offset table has length 1 (a single zero-length row).
  - `packages/ductus-lang/docs/SPEC.md:13708-13715`
    > All bundle forms yield the same external type `Bundle[T]` and share a
    > uniform homogeneous slice-backed storage model: a flat `Handle[T]`
    > backing buffer plus an offsets table that delimits each row.
    > User code always sees a `Handle[T][..N]` or `Handle[T][..]` slice from
    > `bundle[g]`; single-element rows are length-1 slices, uniform with the
    > other forms (no collapse to a bare `Handle[T]`). The empty bundle
    > literal `[]` is a legal `Bundle[T]` for any inferable `T`; its offsets
    > table has length 1 (one zero-length row).
  - `packages/ductus-lang/docs/SPEC.md:13692-13693`
    > - `bundle.count` returns the row count; `bundle[g].count` returns the
    >   row's element count.
  - `packages/ductus-lang/docs/SPEC.md:13710-13710`
    > backing buffer plus an offsets table that delimits each row.
  - `packages/ductus-lang/docs/SPEC.md:13701-13703`
    > for chord in chords:                  // unrolls to one body copy per bundle
    >   for note in chord:                  // unrolls per element of the row
    >     process(note)

#### F160 — Standalone `view` is said to 'never widen acceptance', yet a standalone view carries cardinality (e.g. `Drivable+`, min 1) whose enforcement would widen the caller's minimum obligation — behavior undefined.

- **Severity/Category/Verdict:** MED / ambiguity / CONFIRMED
- **Anchors:** LOG 017-9, 017-19, 017-28 · SPEC § 13.3.1, 13.3.3, 13.3.3.1
- **Why it is a defect:** Two careful readings yield different program behavior. Reading A: the standalone view's `+` (or the bare = exactly-one of 017-28) is enforced against the caller, so `view drivables: Drivable+` over `children: all: Node*` requires ≥1 Drivable and rejects zero — that is widening the caller's minimum, contradicting 'never widens acceptance'. Reading B: the cardinality is inert on a standalone view, but then no rule says so, and 017-28's exactly-one default plus the `+` example are meaningless. An implementer cannot tell whether supplying zero Drivables compiles.
- **Direction of change:** Pin whether a standalone `view` declaration's cardinality is enforced (and if so reconcile with 'never widens acceptance') or is selection-only/inert; state it in a LOG entry and conform SPEC §13.3.3.
- **Evidence check:** pass — A standalone `view drivables: Drivable+` (min 1, per 017-28) over `all: Node*` (permits zero) either enforces its minimum against the caller — widening the caller's minimum obligation, contradicting 017-19's 'never widens acceptance' — or is inert, which no rule states and which voids the view cardinality; whether supplying zero Drivables compiles is undefined.
- **Charity check:** sustain — Confirmed behavior-changing ambiguity. SPEC 13284-13291 shows `view drivables: Drivable+` over `children: all: Node*` and says the standalone view 'never widens acceptance, only narrows the selection'. The `+` means min-1 (1..). Reading A: the generic Conjunction rule (SPEC 13335 'Each view's cardinality constrains the count of supplied children') and 021-41 (LOG 2780, 'Cardinality declared in the parent's views is enforced at placement') apply to the standalone view too, so `Drivable+` requires >=1 Drivable and REJECTS a caller supplying zero — tightening the caller's minimum beyond `Node*` (min 0), clashing with 'never widens acceptance'. Reading B: standalone-view cardinality is inert (selection-only), but then 017-28's 'exactly one' default and the `+` example carry no caller-facing meaning, and no rule states the inertness. I hunted both docs for any explicit statement that a STANDALONE view (vs a named acceptance entry) does NOT enforce its cardinality against the caller — none found; 'purely receiver-side'/'narrows the selection' (SPEC 13290-13291) does not say whether the cardinality specifier participates in placement-time checking. An implementer cannot tell whether supplying zero Drivables compiles. No forced-by-quoted-text reading eliminates the alternative; inference is required. | none found; SPEC.md:13335 states generically 'Each view's cardinality constrains the count of supplied children' (pulling toward enforcement of the standalone view's + and thus toward changing the caller's minimum) while SPEC.md:13290-13291 says the standalone view 'never widens acceptance, only narrows the selection' — no passage states whether a standalone (non-acceptance-entry) view's cardinality is enforced against the caller, leaving both readings live.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2160-2160`
    > 017-19. Acceptance entries place no instances themselves; they are the node's caller-facing contract, bounding only what a caller supplies. A separately declared `view` (when present) is a receiver-side selection over already-accepted children — it never widens acceptance. (§13.3.3)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2169-2169`
    > 017-28. A view selector with no cardinality specifier means **exactly one**; multiplicity is always explicit. (§13.3.3.1)
  - `packages/ductus-lang/docs/SPEC.md:13286-13291`
    >     all: Node*                       // accepts any node
    >   view drivables: Drivable+          // projection — selects the Drivable subset
    > ```
    > 
    > The standalone `view` form is purely receiver-side: it never widens
    > acceptance, only narrows the selection.

#### F085 — The self-sourced-connection worked example types `wheel` as static `Handle[Drivable]` yet requires `dynamic incoming` and claims membership varies — contradicting the rule that only `WeakHandle` dynamizes membership; LOG 017-139 uses `WeakHandle`.

- **Severity/Category/Verdict:** MED / stale_example / CONFIRMED
- **Anchors:** LOG 017-139, 017-141 · SPEC § 13.3.4.2
- **Why it is a defect:** 017-141 (and SPEC 14008-14012) tie dynamic incoming membership to a `WeakHandle` destination; a static `Handle[T]` attr 'does not freeze — its referent is fixed', so membership is a static fact and the target needs a STATIC incoming view. The example declares `attr wheel: Handle[Drivable]` (static) yet gives `Axle` a `dynamic incoming` view and comments 'membership varies with the handles', which only WeakHandle does. LOG 017-139, the decision-of-record for this form, spells the attr `WeakHandle[Drivable]`. The SPEC example diverges from the LOG and from its own adjacent rule; an implementer copying it would wrongly require `dynamic incoming` for a static Handle.
- **Direction of change:** Reconcile the example with 017-139/017-141: either type `wheel` as `WeakHandle[Drivable]` (keeping `dynamic incoming` and the 'membership varies' comment), or keep `Handle[Drivable]` and change `Axle` to a static `incoming` view with a corrected comment.
- **Evidence check:** pass — The self-sourced-connection example uses static 'Handle[Drivable]' yet requires 'dynamic incoming' and claims membership varies — but adjacent SPEC text says static Handle does not freeze (membership is a static fact), and LOG 017-139/141 (decision-of-record) spell it 'WeakHandle'; only WeakHandle dynamizes membership. Example diverges from LOG and from its own adjacent rule.
- **Charity check:** sustain — The worked example at SPEC.md:14019-14033 declares `attr wheel: Handle[Drivable]` (the STATIC placement form per 017-176/017-182) yet gives Axle `dynamic incoming drives: Drives*` with comment 'membership varies with the handles' and annotates `wheel` 'configured per instance at placement'. Both signatures describe the WeakHandle scenario. The adjacent normative prose (SPEC.md:14008-14012) ties varying membership + `dynamic incoming` to a `WeakHandle[T]` attr and states a static `Handle[T]` attr 'does not freeze -- its referent is fixed for the handle's lifetime', i.e. produces static per-instance membership needing NO `dynamic incoming`. LOG 017-139, the decision-of-record, spells the attr `WeakHandle[Drivable]`. The SPEC example thus contradicts both LOG 017-139 and its own adjacent rule; an implementer copying it would wrongly require `dynamic incoming` for a static Handle. Charitable rescue (two Driver placements = varying membership) fails: each individual Driver's `Handle` is fixed, so per-instance membership is static -- the 'varies at runtime' semantics are WeakHandle's. Substantive LOG-SPEC divergence.
- **Evidence:**
  - `packages/ductus-lang/docs/SPEC.md:14019-14028`
    > node Axle:
    >   satisfies Drivable
    >   dynamic incoming drives: Drives*    // membership varies with the handles — §13.3.4
    > 
    > node Driver:
    >   attr wheel: Handle[Drivable]        // configured per instance at placement
    >   view pedals: Pedal+
    >   expose:
    >     pedals
    >     Drives: wheel                     // self-sourced; engages after the pedals
  - `packages/ductus-lang/docs/SPEC.md:14008-14012`
    > admits must declare a `dynamic incoming` connection-view of that type
    > (§13.3.4). A statically-placed `Handle[T]` attr destination does not
    > freeze — its referent is fixed for the handle's lifetime.
  - `packages/ductus-lang/docs/DECISION_LOG.md:2280-2282`
    > 017-139. A `WeakHandle`-typed attr destination gives per-instance parameterized wiring, configured at placement: `attr wheel: WeakHandle[Drivable]` with entry `Drives: wheel`. (§13.3.4.2)
    > 017-140. A self-sourced connection through a `WeakHandle` freezes while the handle resolves to `None`. (§13.3.4.2)
    > 017-141. With a `WeakHandle` destination, membership at the destination is not a static fact: every node type the handle's type admits must declare a `dynamic incoming` connection-view of that type. (§13.3.4.2)

#### F173 — SPEC 13.8.1 and 13.14.1 transitive-closure enumerations omit two closure members (effect-argument-bound references; connections' wire-candidate envelopes) that LOG 021-140/141 normatively require.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 021-140, 021-141 · SPEC § 13.8.1, 13.14.1
- **Why it is a defect:** LOG 021-140/141 define the closure with five members including 'node references bound as effect arguments (reached as borrows, not cell stores)' and 'the wire-candidate envelopes of its connections'. Both SPEC enumerations (13.8.1 (a)-(c) and 13.14.1 step 3) list only subtree, connection destinations, and Handle/WeakHandle-reachable instances. An implementer following SPEC would compute a strictly smaller closure than LOG mandates: an instance reachable only as an effect argument, or only via a connection's wire-candidate envelope, would be wrongly flagged `unreachable_top_level_instance` (021-140) or wrongly excluded from the live graph (021-141). This is behavior-changing divergence.
- **Direction of change:** Add the effect-argument-bound-reference and wire-candidate-envelope closure members to SPEC 13.8.1 and 13.14.1 to conform to LOG 021-140/141, or surface the mismatch to the user if the LOG side is the one to be revisited.
- **Evidence check:** pass — Both SPEC transitive-closure enumerations (§13.8.1 and §13.14.1) omit two closure members — effect-argument-bound references and connections' wire-candidate envelopes — that LOG 021-140/141 normatively require; SPEC computes a strictly smaller closure than LOG mandates.
- **Charity check:** sustain — LOG 021-140 and 021-141 both define the entry-point transitive closure with FIVE members, explicitly including 'node references bound as effect arguments (reached as borrows, not cell stores)' and 'the wire-candidate envelopes of its connections'. Both SPEC closure enumerations list only THREE: §13.8.1 (line 16457-16464) '(a) subtree; (b) connection destinations; (c) Handle/WeakHandle-reachable module-level instances', and §13.14.1 (line 18894-18896) 'subtree plus everything reachable through connections and through module-level Handle/WeakHandle references'. Grep for effect-argument / wire-candidate in a closure context found only SPEC line 525 — the SEPARATE finite-interpretation closure (§13.3.7.7/§13.19), not the entry-point transitive closure. Even reading §13.8.1's 'includes' as non-exhaustive leaves the two members unspecified in SPEC while line 16466 makes the closure operative for the unreachable_top_level_instance error. Substantive LOG-SPEC divergence; SPEC is the divergent side per edit protocol. Sustain.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2879-2879`
    > 021-140. A top-level node instance that is not the entry-point and is not reachable from the entry-point's transitive closure is a compile error of class `unreachable_top_level_instance`. The transitive closure comprises the entry-point's own subtree, its connection destinations, its `Handle`/`WeakHandle`-reachable module-level instances, node references bound as effect arguments (reached as borrows, not cell stores), and the wire-candidate envelopes of its connections. (§13.8.1)
  - `packages/ductus-lang/docs/SPEC.md:16456-16464`
    > *reachable* top-level instances. The closure includes: (a) the
    > `main` instance's own child subtree (placements declared in its
    > body, recursively); (b) the destinations of every connection
    > placement within that subtree (§13.8.4); and (c) every module-level
    > top-level instance reachable through a `Handle` or `WeakHandle`
    > (§13.3.6.2) held by any cell in the closure. The transitive closure
    > is the program's *initialization scope*: only reachable instances
    > are instantiated and wired into the runtime graph.
  - `packages/ductus-lang/docs/SPEC.md:18893-18896`
    > 3. Locate the entry-point node instance (the `main` placement,
    >    §13.8.1) and compute its **transitive closure**: the entry-point's
    >    own subtree plus everything reachable through connections and
    >    through module-level `Handle`/`WeakHandle` references. Initialize

#### F177 — LOG 019-49 says a `Handle` destination's `to` follows whichever node it currently resolves to (re-points), but 019-52/019-17 and 017-182 say a static `Handle[N]` is statically placed and never re-points; SPEC 13.6.2 says `WeakHandle`, not `Handle`.

- **Severity/Category/Verdict:** MED / contradiction / CONFIRMED
- **Anchors:** LOG 019-49, 019-52, 019-17 · SPEC § 13.6.2
- **Why it is a defect:** Per 017-182, `Handle[T]` is the statically-placed form whose referent is fixed for the handle's lifetime; only `WeakHandle[T]` re-points. 019-49's atomic wording says a bare `Handle` destination's `to` 'follows whichever node it currently resolves to', which under the static-form reading contradicts 019-52 ('resolution unnecessary', 'unconditionally active') and 019-17 ('only a dynamic destination's target may move'). The SPEC elaboration it cites writes `WeakHandle` in exactly this sentence, so the LOG entry also diverges from its own cited SPEC section. An implementer cannot both hold `to` fixed and let it follow a re-pointing target for a static Handle.
- **Direction of change:** Reconcile the destination term in 019-49 with 019-52/017-182 and its cited SPEC sentence so that only re-pointing destination kinds are named as ones `to` follows; surface to the user which term (WeakHandle vs Handle) is intended rather than picking one.
- **Evidence check:** pass — 019-49 lets a static Handle destination's 'to' follow a re-pointing target, contradicting 019-52 (Handle unconditionally active, resolution unnecessary) and 019-17 (only dynamic destinations move); the cited SPEC section says WeakHandle, not Handle, so the LOG entry also diverges from its own referenced elaboration.
- **Charity check:** sustain — 017-182 defines Handle[T] as the statically-placed form 'whose referent is statically placed in the graph for the handle's lifetime' and only WeakHandle[T] may 're-point'; 019-52 says a Handle[N] destination's 'resolution unnecessary ... unconditionally active'; 019-53 says a statically-placed Handle[N] 'never freezes'; 019-17 says 'only a dynamic destination's target may move'. 019-49 nonetheless says a 'Handle' destination's to 'follows whichever node it currently resolves to' (re-points) — directly contradicting all four. The SPEC sentence it cites (§13.6.2, line 16024) writes 'WeakHandle', not 'Handle', in exactly this position, so 019-49 also diverges from its own cited SPEC. No passage says a static Handle re-points; nothing dissolves the defect. Both the internal contradiction and the LOG-SPEC divergence are real; sustain as filed.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2665-2665`
    > 019-49. When the destination is a reactive selection or a `Handle`, `to` follows whichever node it currently resolves to. (§13.6.2)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2668-2668`
    > 019-52. A `WeakHandle[N]` destination (read type `Option[&N]`) is unwrapped at the boundary: while it resolves to `Some`, the connection is active and the body sees the contained node as `to`. A `Handle[N]` destination (statically placed) auto-derefs to `&N` directly — its statical-placement assertion makes resolution unnecessary, so the connection is unconditionally active. (§13.6.2)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2633-2633`
    > 019-17. Every statically placed connection exists from construction; only a dynamic destination's target may move. (§13.6.0)
  - `packages/ductus-lang/docs/SPEC.md:16023-16025`
    > reactive selection or a `WeakHandle` (§13.8.5.1), `to` follows whichever node it
    > currently resolves to, and the body's reads of `to.*` re-evaluate when it
    > re-points — a *dynamic dependency* on the current target (§13.10.5, §13.12.1).

#### F172 — SPEC pins the connection destination as a `Handle[N]` that reads `Option[&N]` and freezes on `None`, but LOG 021-56/82/83 pin `WeakHandle[N]`; a `Handle[T]` never resolves `None`.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 021-56, 021-82, 021-83 · SPEC § 13.8.4, 13.8.5.1
- **Why it is a defect:** SPEC must conform to LOG. LOG 021-56/82/83 name the freeze-capable, Option-reading destination carrier `WeakHandle[N]`; SPEC names it `Handle[N]`. Per 017-182/186 a `Handle[T]` is the statically-placed carrier whose referent is placed for the handle's lifetime and needs no Option elimination (reads `&T`, never `Option[&T]`); only `WeakHandle[T]` reads `Option[&T]` and can resolve `None`. SPEC's `Handle[N]` reading `Option[&N]` and freezing on `None` is both a LOG divergence and internally incoherent with the Handle contract, so an implementer following SPEC would type the destination read wrong and admit a freeze state a `Handle` cannot enter.
- **Direction of change:** Reconcile SPEC 13.8.4 and 13.8.5.1 with LOG 021-56/82/83 on which carrier (Handle vs WeakHandle) is the storable, Option-reading, freeze-capable connection destination; surface to user rather than deciding which side is canon.
- **Evidence check:** pass — SPEC pins connection destination as Handle[N] reading Option[&N] and freezing on None; LOG 021-56/82/83 pin WeakHandle[N]. A Handle[T] auto-derefs to &T (017-186) and never reads Option — SPEC divergence plus internal incoherence with the Handle contract.
- **Charity check:** sustain — SPEC §13.8.4 (16808-16811) and §13.8.5.1 (16961-16963) name the Option[&N]-reading, freeze-on-None destination carrier `Handle[N]` (bracketed, i.e. the statically-placed form per 017-182). LOG 021-56/82/83 name it `WeakHandle[N]`. Divergence confirmed verbatim on both sides. The charitable escape — that bare unbracketed 'Handle' is an umbrella over both types per 017-182 — does NOT apply: SPEC wrote `Handle[N]` with the type parameter in backticks, which 017-179/017-186 fix unambiguously as the static form that auto-derefs to &N and never reads Option[&N]. So SPEC pins the wrong carrier. SPEC must conform to LOG; this is a substantive LOG-SPEC divergence. Note the same SPEC line-range also anchors F228; F172 is the LOG-vs-SPEC divergence view, F228 the SPEC-internal-contradiction view — both hold on the identical mislabel.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2795-2795`
    > 021-56. A connection's destination is supplied in the placement's body as a node reference: a bare identifier, any (possibly reactive) expression yielding a node reference, or a `WeakHandle[N]` read as `Option[&N]`. (§13.8.4)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2821-2822`
    > 021-82. A connection endpoint is a borrow; a destination that must persist or re-point across time is supplied as a `WeakHandle[N]`, whose read is `Option[&N]`. (§13.8.5.1)
    > 021-83. A `WeakHandle` destination points at the contained node while the resolution is `Some` and the connection freezes while it is `None`. (§13.8.5.1)
  - `packages/ductus-lang/docs/SPEC.md:16808-16811`
    > or a `Handle[N]` (read as `Option[&N]`,
    > §13.3.6.2) selecting one of
    > the candidate nodes (the connection freezes while that selection resolves to
    > `None`, §13.9.7).
  - `packages/ductus-lang/docs/SPEC.md:16959-16963`
    > supplied as a `Handle[N]` (§13.3.6.2) — the storable designation, whose read
    > is `Option[&N]`. The connection points at the contained node while that
    > resolution is `Some` and **freezes** while it is `None` (§13.9.7).
  - `packages/ductus-lang/docs/DECISION_LOG.md:2327-2327`
    > 017-186. A `WeakHandle[T]` is eliminated th
rough the ordinary `Option` eliminators `match` and `?`: `let s = target?.speed`. A `Handle[T]` needs no elimination. (§13.3.6.2)

#### F256 — SPEC justifies forbidding `for` over a dynamic view by claiming `for` is compile-time, but §12.3.7 says `for` runs at runtime for runtime iterables (e.g. Vec[T]); the rationale contradicts the language's own loop model.

- **Severity/Category/Verdict:** MED / logical_flaw / CONFIRMED
- **Anchors:** LOG 014-58, 014-65, 014-70 · SPEC § 13.3.3.4, 12.3.7
- **Why it is a defect:** The SPEC's parenthetical reason for the `for`-ban — '`for` is compile-time, §12.3.7, and the set is not a compile-time fact' — is false on the spec's own terms. 014-58 and 014-65 (SPEC §12.3.7) state `for` runs at RUNTIME over variable-extent runtime iterables such as `Vec[T]` and `Map[K, V]`. A dynamic view is exactly such a runtime-varying collection. So 'not a compile-time fact' is not a valid reason to reject `for`; by the same criterion `for x in some_vec:` would also be illegal, which it is not. The real reasons (consume-only membership, `WeakHandle` items, no stable positional identity) are stated elsewhere, but the compile-time rationale is self-contradictory and will mislead an implementer or future editor about why the ban exists.
- **Direction of change:** Replace the compile-time-based justification with the actual reason the ban rests on (consume-only membership / WeakHandle items / operator-and-repeat-only consumption), consistent with §12.3.7's runtime-`for` rule.
- **Evidence check:** pass — SPEC §13.3.3.4 justifies forbidding `for` over a dynamic view by asserting `for` is compile-time, but §12.3.7 (014-58/014-65) states `for` runs at runtime for runtime-varying iterables like `Vec[T]` and `Map[K,V]`, so the stated rationale contradicts the language's own loop model.
- **Charity check:** sustain — Confirmed self-contradictory rationale. SPEC.md:13615-13616 justifies the dynamic-view `for`-ban with '(`for` is compile-time, §12.3.7, and the set is not a compile-time fact)'. But §12.3.7 (SPEC.md:10317-10318 and 014-58/DECISION_LOG.md:1697) says `for` is compile-time-unrolled ONLY iff its iterable is compile-time known, 'otherwise it runs at runtime'; 014-65 (DECISION_LOG.md:1704) lists `Vec[T]`, `Map[K,V]`, `yielded T` as runtime `for` iterables. So '`for` is compile-time' is false as a blanket premise, and by its own criterion `for x in some_vec:` would be illegal too — which it is not. Worse, the same dynamic-view kind 'satisfies `Iterable` and `IntoIterable`' (SPEC.md:13641), so a runtime `for` is mechanically well-typed; the real ban reasons (consume-only, WeakHandle items, no positional identity) are stated elsewhere in the same passage. The parenthetical rationale contradicts the very section it cites. Charitable check: no reading makes '`for` is compile-time' consistent with §12.3.7's runtime-`for` rule. SUSTAIN (note: the concrete behavior — the ban — is unambiguous, so by the rubric this is closer to LOW/logical_flaw than MED; the defect is a false, misleading rationale, not a behavior ambiguity).
- **Evidence:**
  - `packages/ductus-lang/docs/SPEC.md:13614-13617`
    > Beyond the two consumption paths and the `.count` element tally, no
    > other access exists: `for` cannot iterate a dynamic view (`for` is
    > compile-time, §12.3.7, and the set is not a compile-time fact);
    > `viewname[i]` is rejected (a positional index into a keyed, changing set is
  - `packages/ductus-lang/docs/SPEC.md:10317-10318`
    > A `for` loop is **compile-time-unrolled iff its iterable is compile-time
    > known** (§2.4.1). Otherwise it runs at runtime per §12.3.1.
  - `packages/ductus-lang/docs/DECISION_LOG.md:1697-1697`
    > 014-58. A `for` loop is compile-time-unrolled iff its iterable is compile-time known per §2.4.1; otherwise it runs at runtime. (§12.3.7)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1704-1704`
    > 014-65. Loops over variable-extent types run at runtime: `Vec[T]`, `SmallVec[T, N]`, `RingBuf[T, N]`, `String`/`s.chars()`, `Map[K, V]`, `yielded T`, and ranges with runtime bounds. A `yielded T` group is runtime-loop iterable only. (§12.3.7)

#### F041 — 008-69 allows a runtime `match` to yield a `Type[C]`, but 008-64/008-66 require type values to be compile-time-resolved and each slot monomorphized to one concrete type; the two readings of 'runtime selection' give different program behavior and nothing in scope pins which holds.

- **Severity/Category/Verdict:** MED / ambiguity / PLAUSIBLE
- **Anchors:** LOG 008-64, 008-66, 008-69 · SPEC § 5.7.3, 13.2.10
- **Why it is a defect:** Two careful readings yield different concrete behavior. Reading A: the `match` arms must all resolve to the SAME compile-time-known concrete type, so 008-66 monomorphization (one `~F` per supply site) survives and 008-64 (compile-time resolution, no runtime representation) holds — but then a runtime `match` over a runtime scrutinee is degenerate/pointless. Reading B: the `match` genuinely yields DIFFERENT concrete types depending on a runtime scrutinee — the useful case implied by matching on a runtime value — but then the enclosing slot cannot be monomorphized to a single `~F` (008-66) and the type value carries information not resolvable at compile time (008-64). No rule in scope, nor §5.7.3/§13.2.10, states whether the arms of such a `match` must be a single concrete type or may differ, so an implementer cannot tell whether to reject multi-type-arm `match`→Type[C] or to carry a runtime type tag (which 008-64 appears to forbid).
- **Direction of change:** Pin whether a `match` yielding `Type[C]` may have arms of differing concrete types (and if so, how that reconciles with per-supply-site monomorphization in 008-66 and no-runtime-representation in 008-64), or restrict it to arms of one concrete type. Surface the choice to the user rather than resolving it.
- **Evidence check:** partial — 008-64 (compile-time-resolved, never vtable-erased) + 008-66 (each slot monomorphized to one ~F) vs 008-69/SPEC:12890 (a runtime match may yield different-typed Type[C] values) — but SPEC:12890-12897 does specify the observable behavior ('chooses one type, places once'), so the genuine gap is the monomorphization/erasure mechanism, not two divergent program behaviors as claimed.
- **Charity check:** sustain — Behavior-affecting ambiguity confirmed; no text forces one reading, and the SPEC deepens rather than dissolves it. 008-69/§13.2.10 line 12895-12896 says `Use match->Type[C] when exactly one of several types should ever be placed` (implying arms yield DIFFERENT types selected at runtime — Reading B), while 008-64 (`resolved at compile time, never erased behind a vtable`) and 008-66 (each supply site monomorphizes to one ~F) require a single compile-time-known type (Reading A). Under Reading B the winner is runtime-chosen among structurally-different node types, so the supply site cannot monomorphize to one ~F and the placed type is not compile-time-resolved — requiring a runtime type tag that 008-64 forbids and that §5.7.3 line 4703 says is unnecessary (`why type values never require erasure`). The two SPEC passages actually conflict on the SAME case: §5.7.3 (4701-4703) says `Selecting among different node structures at runtime is given, not a type value`, but two different Type[Drivable]-satisfying types ARE different node structures, so §5.7.3 routes them to `given` while §13.2.10 routes `one of several types` to match->Type[C]. Neither passage dissolves the 008-64/66-vs-008-69 tension; both reinforce it. No normative text pins whether match->Type[C] arms must collapse to one compile-time type (degenerate) or may differ (requires forbidden erasure). MED sustained.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:924-924`
    > 008-64. A type value is resolved at compile time and is never erased behind a vtable. (§5.7.3)
  - `packages/ductus-lang/docs/DECISION_LOG.md:926-926`
    > 008-66. Each supply site picks a concrete type and monomorphizes the enclosing type for it: `ListView | item=PostCard` infers `~F = PostCard`. (§5.7.3)
  - `packages/ductus-lang/docs/DECISION_LOG.md:929-929`
    > 008-69. Runtime selection among type values of the same slot type is allowed: a `match` may yield a `Type[C]`. (§5.7.3)
  - `packages/ductus-lang/docs/SPEC.md:12890-12892`
    > A `Type[…]` value is a *value*, so a value conditional may select among
    > type values of the same slot type: `match scrutinee: …` yielding a
    > `Type[C]` chooses *one* type, which the receiving node then places once.

#### F056 — The .exposition CLOSED sum of exactly five entry kinds has no variant for an expose:-level `repeat` over a value collection, yet such repeats are admitted and produce internal scope children.

- **Severity/Category/Verdict:** MED / gap / PLAUSIBLE
- **Anchors:** LOG 017-219, 018-104, 017-262 · SPEC § 13.3.7, 13.5.4.7, 13.3.7.7
- **Why it is a defect:** The DynamicView variant is defined narrowly as a §13.3.3.4 membership view whose members are WeakHandle[T] entity references drawn from caller-supplied dynamic children; its walk is `for m in v: interpret_node(m)` (nodes only). But 018-104 admits a bare `repeat p in items:` in `expose:` where the source may be a value collection (018-37 allows `Signal[I]` for `I: Iterable`, e.g. `Signal[Vec[Post]]`), materializing internal scope children whose bodies may contain arbitrary node AND connection placements (018-55). Those scopes are not caller-supplied dynamic-view members and are not a flat WeakHandle[T] member list, so no one of the five kinds represents them. Because 017-262 makes exposition-walking interpretation expressible in-language over this closed sum, a conforming in-language interpreter that exhaustively matches the five variants cannot encounter or interpret these entries — an implementer-visible completeness hole against the 001-5/001-6 no-postponed-decision, explicit-boundary-only commitments.
- **Direction of change:** Pin how an expose:-level value-collection `repeat` maps into the closed entry sum (either define/extend a variant that carries repeat-materialized scope bodies, or restrict expose:-level `repeat` to dynamic-view-feeding forms), so the five-kind sum remains exhaustive for everything expose: can produce.
- **Evidence check:** pass — The closed five-kind exposition sum has no variant for an expose-level 'repeat' over a value collection (018-37 admits Signal[Vec[T]]): 018-104 materializes internal scope children whose bodies may hold connections (018-55), but the DynamicView variant's walk is nodes-only ('for m in v: interpret_node(m)'), so an exhaustive in-language interpreter cannot represent/walk these connection-bearing repeat scopes — a completeness gap. Gap is inferential, not a flat textual contradiction, but the nodes-only walk corroborates it.
- **Charity check:** refute — The finding assumes a value-collection expose-level `repeat` (018-104; source `Signal[Vec[Post]]` per 018-37) needs its own `.exposition` variant or maps to `DynamicView`, and that none of the five kinds represents it. The corpus forces the opposite reading: a `repeat`'s scope placements are FLATTENED into individual entries in the flat exposition list, each already a `Node`/`Connection`/`Bundle`/`Gated` entry. 017-234 states iteration entries 'emit type-internal structure in place ... a `repeat` mounts its keyed scopes there'; 018-56 states 'Each scope's placements join the enclosing structural sequence at the `repeat`'s written position, in source order'; and SPEC.md:14700-14701 states a repeat-materialized connection 'mounts and dismounts with its keyed scope, appearing in `.exposition` only while mounted' -- i.e. an individual connection ENTRY, not a grouped variant. `DynamicView` (SPEC.md:14774/14791, walk `for m in v: interpret_node(m)`) is the distinct construct for dynamic child SUPPLY (WeakHandle[T] members over a closed candidate envelope), which repeat scopes are not and need not be. The node/connection types inside a written repeat body are statically known, so they slot into existing `Node`/`Connection` entries whose runtime variation is only mount/dismount. The closed sum (017-219) therefore DOES represent them; no sixth kind is needed. The dissolving passages are consistent with the finding's quoted passages (017-219 closed sum, 018-104, SPEC DynamicView walk) -- no conflict -- so this is a clean refute, not a divergence. | DECISION_LOG.md:2375 > 017-234. Iteration entries emit type-internal structure in place: a compile-time `for` unrolls at its written position; a `repeat` mounts its keyed scopes there. AND DECISION_LOG.md:2525 > 018-56. Each scope's placements join the enclosing structural sequence at the `repeat`'s written position, in source order. AND SPEC.md:14700-14701 > connection placement (§13.3.4, §13.5.4) is the exception — it mounts and dismounts with its keyed scope, appearing in `.exposition` only while mounted.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2360-2360`
    > 017-219. `.exposition` is a typed, language-owned CLOSED sum of exactly five entry kinds: `Node`, `Connection`, `Bundle`, `DynamicView`, and `Gated`. User code can never add a sixth kind; the sum is defined by the language, not the stdlib. (§13.3.7)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2574-2574`
    > 018-104. `repeat` is admitted in `expose:` blocks: scopes become internal children of every instance of the enclosing node, materialized at instantiation, tracking the source, at the `repeat`'s written position in the exposition. (§13.5.4.7)
  - `packages/ductus-lang/docs/SPEC.md:14774-14774`
    > | `DynamicView` | a runtime-varying membership view (§13.3.3.4) |
  - `packages/ductus-lang/docs/SPEC.md:14791-14791`
    > DynamicView(v):     for m in v: interpret_node(m)

#### F117 — A bundle `as buf` name inside a `repeat … as` body has no defined entry-field type: the minting rule types fields as singular `WeakHandle[T]`, but a bundle name binds an unstorable slice.

- **Severity/Category/Verdict:** MED / gap / PLAUSIBLE
- **Anchors:** LOG 018-130, 017-99, 002-9 · SPEC § 13.5.4.9, 13.3.3.5
- **Why it is a defect:** Scenario needs a wire path entering a bundle placement bound `as buf` inside a repeat. A repeat body may contain node placements (018-55), and a bundle `[...] as buf` is a node co-placement (017-89). 018-130 mints one `<view>::entry` field per `as <name>` placement, typed singular `WeakHandle[T]`, so outside-scope access works. But a bundle `as`-name binds a slice `Handle[T][..N]` (017-99), not a single node reference, and 018-130 gives no rule for the field a bundle name contributes: is it omitted, a `WeakHandle[T][..N]` slice (which SPEC 13.3.3.5 line 13801-13803 says is unstorable, so it cannot be a record field), or per-element fields? Whether a bundle name is even addressable through the repeat view is therefore unspecified — which determines whether a wire can follow into a repeat-scoped bundle at all. The construct (bundle inside repeat, named) is legal; its cross-scope surface is a gap.
- **Direction of change:** Specify what field, if any, a bundle `as`-name contributes to `<view>::entry` for a `repeat … as` body — either exclude bundle names from entry fields (and state cross-scope bundle access is unavailable), or define a storable per-bundle representation — and state the same for `for … as` (017-67).
- **Evidence check:** pass — 018-130 types every repeat `as <name>` entry field as singular `WeakHandle[T]`, but a bundle `as`-name (017-99) binds an unstorable row slice (SPEC 13799-13803). No rule defines the ::entry field a bundle name contributes — so whether a bundle inside a repeat is cross-scope addressable at all is unspecified. Real implementer-blocking gap for a legal construct.
- **Charity check:** refute — The finding's premise — 'a bundle [...] as buf inside a repeat body is a legal construct whose entry-field type is undefined' — is false at the LOG level. 018-143 explicitly makes bundle-in-repeat ILLEGAL: 'a `repeat` body cannot contain bundle brackets', a parse-time error (class bundle_in_repeat_rejected). With the construct rejected, there is no legal case for §13.5.4.9's WeakHandle[T] field-type rule to fail to cover. Consistency check: 018-143 (dissolver) does not conflict with the finding's cited passages — 018-130 (WeakHandle[T] field), 017-99 (bundle as-name slice), 002-9 (as-keyword catalog), SPEC:13799-13803 (slices unstorable) none assert bundle-in-repeat is legal; 002-9 merely catalogs uses of `as`, not that they nest. The legality premise is refuted cleanly. | DECISION_LOG.md:2613 — "018-143. Bundle brackets `[…]` are not legal in a `repeat`-rejected context: a `repeat` body cannot contain bundle brackets, and a bundle bracket cannot contain a `repeat` ... The error class is `bundle_in_repeat_rejected` ... the compiler emits it at parse time at the offending site."
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2600-2600`
    > 018-130. `<view>::entry` is a compiler-minted nominal record per `repeat … as` site; its fields are named after the `as <name>` placements inside the repeat body. The record is path-derived and synthetic — not nameable in user code as a parameter type. The field type is `WeakHandle[T]`. The placement binding `<name>` itself stays a borrow. (§13.5.4.9)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2240-2240`
    > 017-99. `as`-naming a bundle: `[n1 n2] as pair` binds `pair` to a borrow of the row slice form — `Handle[T][..N]` (compile-time row length) or `Handle[T][..]` (runtime length), the same type as the receiving view's row. The slice element type is `Handle[T]` (the bundle backing stores `Handle[T]`); `as <name>` never binds to a bare `Handle[T]`. (§13.3.3.5)
  - `packages/ductus-lang/docs/DECISION_LOG.md:80-80`
    > 002-9. `as` is the naming/alias keyword, used for placement names, import aliases, `for … as` view names, `repeat … as` view names, bundle `as`-names, `collect as x` membership bindings, and the observe as-binding `on <trigger> [where C] as <binder>` that names the post-filter event value. (§1.4)
  - `packages/ductus-lang/docs/SPEC.md:13799-13803`
    > A `Bundle[T]` is storable: its fields are all `Copy` (Handles are Copy;
    > `usize` offset tables are Copy). The bundle value sits in
    > cells, fields, records, and attrs. Row slices, however, are borrows
    > (§11.9) and follow the ordinary borrow rules — `bundle[g]` cannot be
    > stored, only the whole bundle.

#### F181 — 020-3/020-10 state name resolution as 'nearest enclosing scope wins' unconditionally, while 020-17 makes a body-vs-module name collision a compile error — conflicting resolutions for that case.

- **Severity/Category/Verdict:** LOW / ambiguity / CONFIRMED
- **Anchors:** LOG 020-3, 020-10, 020-17, 020-19 · SPEC § 13.7.1, 13.7.4
- **Why it is a defect:** 020-3 and 020-10 state 'nearest scope wins' as a total rule with no carve-out. For a name declared in both the instance body scope (scope 2) and the module scope (scope 3), 'nearest wins' resolves to the body member; but 020-17 says that exact collision is a compile error. Two careful readings of the atomic entries yield different behavior (silent body resolution vs. compile error). Only by integrating 020-19 (which limits the error to body-vs-module and permits local-binding shadowing) does the reader reconcile them; the 'nearest wins' entries do not self-limit to the local-binding case.
- **Direction of change:** Tighten 020-3/020-10 so the 'nearest wins' rule is scoped to the cases where it holds (local bindings shadowing outer scopes), leaving the body-vs-module collision governed by 020-17; confirm the intended scoping with the user.
- **Evidence check:** pass — 'Nearest wins' (020-3/020-10) and 'collision is compile error' (020-17) give conflicting resolutions for the body-vs-module case.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2701-2701`
    > 020-3. There is no receiver concept: a bare name binds to the nearest enclosing scope that declares it. (§13.7)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2708-2708`
    > 020-10. A bare name resolves to the nearest scope in the chain that declares it: `local_gain * master_gain` reads a member and a module cell anchor-free. (§13.7.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2715-2715`
    > 020-17. A bare name declared in both the instance body scope and the module top-level scope is ambiguous — a compile error: `derived a: f32 = gain` ✗. (§13.7.4)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2717-2717`
    > 020-19. Local bindings (`let`, `for`-vars) shadow outer scopes normally; the ambiguity error applies only to the body-vs-module collision. (§13.7.4)

#### F166 — 018-83 says a `Map[K, V]` source 'is diffed by key identity' with 'scopes are keyed', implying an automatic Map-key derivation path that the key-derivation precedence (018-61..72) does not provide.

- **Severity/Category/Verdict:** LOW / ambiguity / CONFIRMED
- **Anchors:** LOG 018-83, 018-44, 018-64, 018-67, 018-72 · SPEC § 13.5.4.2, 13.5.4.1
- **Why it is a defect:** 018-83's phrasing that a `Map` source 'is likewise diffed by key identity' and 'scopes are keyed, not positional' reads as if the Map's own key `K` automatically becomes the scope key. But the element yielded is `(K, V)` (018-44), a tuple — which is not a `StringifiableKey` (018-64 excludes tuples), is not `Keyed` unless separately fulfilled, and the carried-key path is restricted to `dynamic` namespace sources (018-67), not a plain `Map` signal. Under the strict precedence (018-61..72), `repeat (k,v) in map_signal` with no `keyed by` would fail at 018-72. Two readings: (A) 018-83 merely describes behavior once `keyed by k` is written — Map has no auto-key path; (B) Map's `K` is an implicit scope key (an unstated fifth derivation path). SPEC §13.5.4.1 (line 15481-15482, example `keyed by sid`) resolves in favor of reading A, so no divergent runtime behavior is currently witnessed — but the LOG entry alone is misleading and could induce an implementer to add a nonexistent Map auto-key path.
- **Direction of change:** Consider tightening 018-83's wording so it does not imply an automatic Map-key scope derivation, or add an explicit entry stating Map requires `keyed by` (matching SPEC §13.5.4.1); this is a clarity decision to surface, not a unilateral rewrite.
- **Evidence check:** pass — 018-83's Map phrasing implies an auto Map-key derivation path the precedence rules do not provide.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2553-2553`
    > 018-83. The unordered iterable `HashSet[T]` is diffed by key identity; its iteration order is whatever the underlying iterator emits and does not affect scope identity. `Map[K, V]` iterates in insertion order and is likewise diffed by key identity; its stable iteration order does not affect scope identity, since scopes are keyed, not positional. (§13.5.4.2)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2514-2514`
    > 018-44. `Map[K, V]`'s iterator yields `(K, V)` pairs. (§13.5.4.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2537-2537`
    > 018-67. The carried-key path applies only to `dynamic` namespace sources. (§13.5.4.1)
  - `packages/ductus-lang/docs/SPEC.md:15478-15482`
    > **`Map` source with destructuring bind** — `Map[K, V]`
    > iterates as `Iterable` yielding `(K, V)` pairs. Move-promotion
    > (§13.5.4.1) gives the bind an owned `(K, V)`, so ordinary tuple
    > destructuring (§12.12.1) binds owned `sid` and `info`; `keyed by`
    > names the map key as the scope key:

#### F087 — The duplicate-scope-key rule mandates a runtime 'non-fatal warning' but assigns no diagnostic class, unlike its sibling rules, and the trigger is runtime-data-dependent, so no compile-time conformance test is constructible for it.

- **Severity/Category/Verdict:** LOW / unfalsifiable / CONFIRMED
- **Anchors:** LOG 018-81 · SPEC § 13.5.4.2
- **Why it is a defect:** Neighboring diagnostics (018-142, 018-143, 031-141) all carry named normative diagnostic classes so tooling can target them; 018-81's 'non-fatal warning' has none. It is also a RUNTIME warning fired only when two elements happen to derive the same key at some commit, which depends on runtime data. A conformance test for 'the runtime raises exactly one non-fatal warning here' cannot be written as a static/deterministic check without a class handle and a defined delivery channel. This is minor (behavior of first-wins-and-drop IS testable) but the warning obligation itself is not conformance-checkable as written.
- **Direction of change:** Assign 018-81 a named diagnostic class (parallel to 018-142/018-143) and specify the warning's delivery channel so the obligation is targetable and testable.
- **Evidence check:** pass — 018-81's runtime non-fatal warning has no diagnostic class handle unlike its siblings, so it is not conformance-checkable.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2551-2551`
    > 018-81. Two distinct elements deriving the same scope key in one evaluation: the first in iterator order wins the scope and bind, later duplicates are dropped, and the runtime raises a non-fatal warning; it never traps. (§13.5.4.2)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2612-2612`
    > 018-142. `at <index>` on an unordered `repeat` source (`HashSet[T]`, and other unordered iterables) emits a **normative diagnostic** of class `unstable_positional_iteration_on_unordered_source`.
  - `packages/ductus-lang/docs/SPEC.md:15411-15415`
    > **Duplicate keys.** If two distinct elements in one evaluation derive
    > the same key, the first in iterator order wins the scope and the
    > `<bind>`; later duplicates are dropped. The runtime raises a non-fatal
    > warning — a non-unique `keyed by` derivation is almost always a bug —
    > and continues; a duplicate key never halts the program.
  - `packages/ductus-lang/docs/DECISION_LOG.md:2613-2613`
    > 018-143. Bundle brackets `[…]` are not legal in a `repeat`-rejected context ... The error class is `bundle_in_repeat_rejected` (a normative diagnostic class name targetable by user tooling)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3910-3910`
    > 031-141. A normative diagnostic class covers a cell name duplicated across `desired:` and `observed:`. (§13.19.16)

#### F162 — The empty bundle `[]` is stated to have 'zero elements' yet an offset table of length 1, so `bundle.count` (defined as row count) yields 1 for an empty bundle — surprising and unaddressed.

- **Severity/Category/Verdict:** LOW / design_smell / CONFIRMED
- **Anchors:** LOG 017-89, 017-90 · SPEC § 13.3.3.5
- **Why it is a defect:** 017-90 defines `bundle.count` as the row count, and an offset table of length 1 means one row. So for the empty bundle `[]`, `.count == 1` while it holds zero elements. Nothing states this, and it clashes with the natural expectation that an empty literal has count 0. Consistent between LOG and SPEC (not a divergence), but a latent surprise for any code that treats `.count == 0` as emptiness. Note: also unclear what `[].count` (row count 1) vs `[][0].count` (element count 0) mean together.
- **Direction of change:** State explicitly what `[].count` evaluates to and reconcile the 'zero elements' framing with the length-1 offset-table model (either count reports rows and this edge case is documented, or the empty bundle is modeled as zero rows).
- **Evidence check:** pass — Empty bundle [] has zero elements but row count 1, so .count==1 clashes with emptiness expectation.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2230-2230`
    > 017-89. A `Bundle[T]` is a homogeneous co-placement of children written `[...]` at placement — a distinct kind from a view; only an explicit `[...]` constructs a bundle, a bare placement is not. The empty bundle literal `[]` is allowed and produces a `Bundle[T]` value for any inferable `T`, with an offset table of length 1 holding a single zero-length row.
  - `packages/ductus-lang/docs/DECISION_LOG.md:2231-2231`
    > 017-90. Bundle access goes through the `Index` trait (§4.9.5): `bundle[g]` returns a row slice (`Handle[T][..N]` or `Handle[T][..]`), `bundle[g][i]` returns `Handle[T]`, and `bundle.count` returns the row count. Indexing follows the index-to-min-cardinality rule (`viewname[i]` legal iff `i < min_cardinality`) at each level — outer `[g]` to-min produces the slice, inner `[i]` to-min into it. (§13.3.3.5)
  - `packages/ductus-lang/docs/SPEC.md:13678-13680`
    > An **empty bundle literal `[]`** is legal and has type
    > `Bundle[T]` for any `T` the context can infer. The bundle has zero
    > elements; its offset table has length 1 (a single zero-length row).
  - `packages/ductus-lang/docs/SPEC.md:13692-13693`
    > - `bundle.count` returns the row count; `bundle[g].count` returns the
    >   row's element count.

#### F161 — Normative dynamic-forbid and connection-view rules use the bracket form `[N..M]`, which is not one of the four defined bracket forms (`[=N]`, `[N..=M]`, `[N..]`, `[..=M]`).

- **Severity/Category/Verdict:** LOW / undefined_term / CONFIRMED
- **Anchors:** LOG 017-40, 017-109, 017-33 · SPEC § 13.3.3.1, 13.3.4
- **Why it is a defect:** The set of legal bracket forms is defined exactly (017-32..35) as `[=N]`, `[N..=M]`, `[N..]`, `[..=M]`. `[N..M]` (half-open, exclusive upper) is not among them. 017-40 and 017-109 reference `[N..M]` as if it were a canonical spelling. SPEC §13.3.3.1 (line 13359) mirrors the same `[N..M]`, so it is consistent LOG↔SPEC but internally inconsistent with the defining entries. Reader cannot tell whether `[N..M]` denotes the inclusive `[N..=M]` or a distinct exclusive form the language does not otherwise support.
- **Direction of change:** Make the forbid/example lists in 017-40 and 017-109 (and SPEC §13.3.3.1) use one of the four defined bracket spellings, or add a defined `[N..M]` form; do not leave an undefined bracket spelling in normative text.
- **Evidence check:** pass — `[N..M]` (exclusive-upper half-open) is used in 017-40/017-109 but is not one of the four defined bracket forms.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2174-2174`
    > 017-33. The bracket form `[N..=M]` means between N and M, inclusive on both ends. (§13.3.3.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2181-2181`
    > 017-40. A `dynamic` view takes exactly the `*` specifier (its inherent zero-or-more); the bounded specifiers `?`/`+`/`[=N]`/`[N..M]` are forbidden on `dynamic`, and there is no single dynamic view: `children:` with `dynamic voices: Voice+` ✗. (§13.3.3.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2250-2250`
    > 017-109. Connection-view cardinality uses the same specifiers as views (§13.3.3.1) with the same meaning — **bare = exactly one** in every direction (incoming included), multiplicity always explicit (`*`/`+`/`[N..M]`); fan-in or multi-origin is written `*`. (§13.3.4)

#### F182 — 019-59 defines the endpoint-derived surface as a `WeakHandle` only ('No other surface... exists'), but SPEC 13.6.2 broadens it to `WeakHandle`/`Portal`.

- **Severity/Category/Verdict:** LOW / divergence / CONFIRMED
- **Anchors:** LOG 019-59 · SPEC § 13.6.2
- **Why it is a defect:** The LOG entry names the endpoint-derived carrier as `WeakHandle` and closes the surface set ('No other surface for endpoint data exists'). The SPEC elaboration widens the carrier to `WeakHandle`/`Portal`. The LOG neither mentions nor sanctions `Portal` as an endpoint surface; the SPEC introduces it, a substantive-if-minor LOG-SPEC divergence about which handle kinds may surface an endpoint.
- **Direction of change:** Align the carrier set: either add `Portal` to 019-59 or drop it from SPEC 13.6.2; surface to the user whether `Portal` endpoint surfaces are intended before amending the LOG.
- **Evidence check:** pass — LOG names WeakHandle only and closes the set; SPEC widens the endpoint carrier to WeakHandle/Portal.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2675-2675`
    > 019-59. The endpoint-derived form is the sole sanctioned endpoint surface: a connection type opts in, per type, by declaring a derived over an endpoint — `derived target: WeakHandle[Clip] = handle to`. No other surface for endpoint data exists. (§13.6.2)
  - `packages/ductus-lang/docs/SPEC.md:16058-16060`
    > This is the sanctioned way for an interpreter to reach through a
    > connection ("wire-following", §13.19): the connection publishes a
    > `WeakHandle`/`Portal` of its own, rather than exposing `to` directly. The

#### F176 — 021-57 says a `Handle` destination makes membership a runtime fact, while sibling entries 021-56/82/83/84 name `WeakHandle` as the re-pointing/freeze-capable carrier — terminology drift on which carrier is dynamic.

- **Severity/Category/Verdict:** LOW / divergence / CONFIRMED
- **Anchors:** LOG 021-57, 021-56, 021-82, 021-83, 021-84 · SPEC § 13.8.4, 13.8.5.1
- **Why it is a defect:** Within scope, 021-56/82/83 name `WeakHandle[N]` as the destination carrier that persists, re-points, and freezes on `None`, consistent with 017-182 (WeakHandle is the dynamically-placed/re-pointable form). 021-57 instead pairs 'reactive or `Handle`' for the same runtime-membership trigger. A `Handle[T]` per 017-182 is statically placed for its lifetime and does not dismount/re-point, so 'Handle destination makes membership a runtime fact' is at odds with the WeakHandle framing in its own section's sibling entries. This is terminology drift with no concrete program witness (the reactive-selection driver is common to both), hence LOW, but it muddies which carrier the dynamic-incoming-view obligation attaches to.
- **Direction of change:** Align the destination-carrier term across 021-56/57/82/83/84 (Handle vs WeakHandle) so a single carrier name is used for the re-pointable/runtime-membership destination; surface to user rather than picking a term unilaterally.
- **Evidence check:** pass — 021-57 pairs runtime membership with `Handle` while sibling entries and 017-182 make WeakHandle the re-pointing/dynamic carrier.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2796-2796`
    > 021-57. A reactive or `Handle` destination makes membership at the destination a runtime fact: every node type in the reference's candidate envelope must declare a `dynamic incoming` connection-view of that type. (§13.8.4)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2823-2823`
    > 021-84. A destination that varies at runtime makes the connection's `to` endpoint dynamic: the connection re-points among existing nodes as the reference changes. (§13.8.5.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2322-2323`
    > 017-181. ... `Handle[T]` and `WeakHandle[T]` contain no borrows and are the storable carriers. (§13.3.6.2)
    > 017-182. `Handle` names two distinct types: `Handle[T]`, the *statically-placed* type form whose referent is statically placed in the graph for the handle's lifetime, and `WeakHandle[T]`, the *dynamically-placed* type form whose referent may be dismounted or re-pointed at runtime. ... (§13.3.6.2)

#### F257 — `for own` over a `dynamic view T` is left unspecified: IntoIterable is declared satisfied and its contract moves owned elements out of a consumed source, but the view is consume-only membership whose items are WeakHandle[T] with nothing to move.

- **Severity/Category/Verdict:** LOW / gap / CONFIRMED
- **Anchors:** LOG 017-192, 014-135, 014-127, 014-130, 014-119 · SPEC § 13.3.3.4, 12.9, 12.9.2, 12.7.4
- **Why it is a defect:** 017-192 says the `dynamic view T` kind satisfies `IntoIterable`, whose whole point (014-127, 014-130, 014-135) is `for own x in d:` — consume the source binding and move owned elements out of it. But 017-192/017-73 also say the view is consume-only, is not stored bare, lives only as a reactive cell, and its `Item` is `WeakHandle[T]` (a non-owning reference), not an owned element. There is no owned storage to `move` out of, and the dynamic view cannot be the `own value` consumed at loop entry (it cannot be bound/stored/returned). What `for own` means on a dynamic view — whether it is legal, what it consumes, what it yields — is nowhere stated. This is a stretched-contract smell: `IntoIterable` is asserted for a kind that structurally cannot fulfil the consuming-move contract the trait defines. This is subsumed by finding 1 if that contradiction is resolved by dropping trait satisfaction, but stands independently if the kind keeps `IntoIterable`.
- **Direction of change:** Decide and state whether `dynamic view T` satisfies `IntoIterable` at all; if it does, specify what `for own`/`consuming_iterator` do given `Item = WeakHandle[T]` and consume-only membership, or drop the `IntoIterable` claim. Surface to the user.
- **Evidence check:** pass — 'for own' over a dynamic view T is unspecified: IntoIterable is declared satisfied but Item is non-owning WeakHandle[T] with no owned storage to move and the view cannot be the consumed own value.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2333-2333`
    > 017-192. The `dynamic view T` kind is a language-level non-array iterator-shaped kind with `Item = WeakHandle[T]`. Its storage is implementation-defined: no surface index operator, but it exposes `.count` as the bare element tally. It satisfies `Iterable` and `IntoIterable`, and produces `WeakHandle[T]` elements on iteration. It is consume-only: it is not stored bare; it lives only as a reactive cell in the `dynamic view T` kind, and the cell is what is bound or stored.
  - `packages/ductus-lang/docs/DECISION_LOG.md:1766-1766`
    > 014-127. `IntoIterable::consuming_iterator` declares its parameter `own`: the source is consumed at the call and the returned iterator owns the source's storage. (§12.9)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1769-1769`
    > 014-130. Under `IntoIterable`, each `next` call on a non-`Copy` `Item` physically moves one element out of the iterator's internal storage. (§12.9.2)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1758-1758`
    > 014-119. An iterator reached through `Iterable` dispatch (source-bearing, default form) may not declare `type Item = own T`; owned items are exclusive to the consuming `IntoIterable` path (`Source = ()`, `for own`). (§12.7.4)

### Effect-kind trait methods: dispatch and satisfaction contradictions

Effect-kind methods are simultaneously callable and forbidden as ordinary calls, dispatch has no exclusion step, the observed: block admits and rejects the same forms, and the satisfies/fulfill waivers gate on inconsistent conditions between LOG and SPEC.

#### F088 — 031-14/15 admit only signal/stream in `observed:` and forbid all else, while 031-49/146/152 (and the SPEC diagnostic hint's own contradiction) admit derived/value-recurrent computed outputs there — accept and reject the same forms.

- **Severity/Category/Verdict:** HIGH / contradiction / CONFIRMED (x2 independent reports)
- **Anchors:** LOG 031-14, 031-15, 031-49, 031-152, 031-146 · SPEC § 13.19.2, 13.19.5
- **Why it is a defect:** 031-14 enumerates the legal observed-block role keywords as exactly `signal`/`stream`; 031-15 says nothing outside 'the five role-keyword cell forms' (derived/recurrent/stream in desired + signal/stream in observed) may appear in an effect body — jointly forbidding `derived` and value-`recurrent` in `observed:`. But 031-49, 031-146, and 031-152 explicitly admit them as computed outputs. SPEC repeats the contradiction verbatim: 13.19.2 says 'No other declaration kinds are permitted' and '`recurrent` is allowed only in `desired:`', while 13.19.5 says '`observed:` block may also declare `derived` cells and value-`recurrent` cells'. A parser/type-checker implementer must decide whether `derived x = …` or `recurrent x = …` inside `observed:` parses or is a diagnostic; the two halves demand opposite answers. This is a load-bearing v1 feature (interior effects lift child outputs into `observed:` with no host write), so it cannot be resolved by picking the restrictive half without deleting the interior-effect surface.
- **Direction of change:** Reconcile the observed-block role-keyword enumeration: either 031-14/031-15 and SPEC 13.19.2 must widen to admit `derived` and value-`recurrent` as computed outputs in `observed:`, or the computed-output feature (031-49/031-146/031-152, SPEC 13.19.5) must be withdrawn. Direction only; the user decides which side is authoritative.
- **Evidence check:** pass — 031-14/15 (and SPEC 13.19.2 diagnostic hint) forbid any declaration in observed: other than signal/stream and state recurrent is allowed only in desired:, while 031-49/146/152 and SPEC 13.19.5 admit derived and value-recurrent as computed outputs in observed:; a parser/type-checker cannot both reject and accept 'recurrent x = ...' inside observed:.
- **Charity check:** sustain — Live jointly-unsatisfiable contradiction confirmed on fresh read. 031-14/15 (and SPEC 13.19.2 lines 21937-21938: 'No other declaration kinds are permitted inside the blocks. recurrent is allowed only in desired:') forbid derived and value-recurrent in observed:. 031-49/146/152 (and SPEC 13.19.5 lines 22244-22246: 'An observed: block may also declare derived cells and value-recurrent cells') admit exactly those. SPEC contradicts itself section-to-section; LOG contradicts itself entry-to-entry. A parser/type-checker implementer must decide whether 'derived x =' / 'recurrent x =' inside observed: parses or is a diagnostic — the two halves demand opposite answers, and the interior-effect surface (lifted computed outputs, no host write) is load-bearing so the restrictive half cannot simply win. No carve-out in 13.19.2 for the computed-output lift. Sustained HIGH.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:3783-3784`
    > 031-14. Every cell declaration in an effect block carries an explicit role keyword: `derived`, `recurrent`, or `stream` in `desired:`, `signal` or `stream` in `observed:`. (§13.19.2)
    > 031-15. Reactive declarations other than the five role-keyword cell forms (e.g. `attr`, top-level `signal`) cannot appear inside an effect's body. (§13.19.2)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3921-3921`
    > 031-152. A value `recurrent` is allowed in both an effect's `desired:` and `observed:` blocks; a `recurrent[N] stream` is allowed in `desired:` but forbidden in `observed:`. (§13.19.4)
  - `packages/ductus-lang/docs/SPEC.md:21934-21938`
    > - `signal` (in `observed:`) — host-written value cell (§13.19.5).
    > - `stream` (in `observed:`) — host-written event sequence (§13.19.5).
    > 
    > No other declaration kinds are permitted inside the blocks. `recurrent`
    > is allowed only in `desired:` (a host-fed `observed:` cell has no
  - `packages/ductus-lang/docs/SPEC.md:22244-22246`
    > **Computed outputs (`derived` / `recurrent`).** An `observed:` block
    > may also declare `derived` cells and value-`recurrent` cells that the
    > effect computes itself — from its own `desired:` cells, its parameters,
  - `packages/ductus-lang/docs/DECISION_LOG.md:3818-3818`
    > 031-49. A value `recurrent` (and a `derived`) is valid in `observed:` blocks as a computed output; a `recurrent[N] stream` is not valid in `observed:` blocks. (§13.19.5)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3915-3915`
    > 031-146. A normative diagnostic class covers a non-role-keyword declaration inside an effect's blocks, and `recurrent[N] stream` in `observed:`; a value `recurrent` and a `derived` are allowed in `observed:` as computed outputs, and `recurrent` is allowed in `desired:`. (§13.19.16)
  - `packages/ductus-lang/docs/SPEC.md:22870-22874`
    >   hint: effect blocks accept only `derived`, `recurrent`, `stream` (in
    >         `desired:`) and `signal`, `stream` (in `observed:`). `recurrent`
    >         in `observed:` is rejected (host-fed cells have no expression
    >         body); for state that wraps multiple effects, use a wrapping
    >         operator (§13.17).

#### F068 — The dispatch algorithm §3.4.1 collects effect-kind methods as trait-impl candidates (they are in the effective method set) but has no step to exclude or error on them, so `song.render()` resolves to an effect-kind method as an ordinary call with no defined behavior.

- **Severity/Category/Verdict:** HIGH / gap / CONFIRMED
- **Anchors:** LOG 005-78, 005-79, 005-121, 005-120 · SPEC § 3.4.1, 3.1.1.1
- **Why it is a defect:** Step 1 collects a candidate for every trait method named f in scope; 005-78 and SPEC 1248-1249 put effect-kind methods into that same effective set 'exactly as value methods' and say they 'resolve by the same rules as any other method.' So for `song.render()` where render is an effect-kind method and no value method exists, step 4 resolves to the effect-kind method — an ordinary call SPEC 1250 forbids. The 8-step algorithm has no branch that detects an effect-kind candidate and emits a use-in-wrong-context diagnostic. No error path and no legal boundary is declared for this reachable input.
- **Direction of change:** Add an explicit step/rule to §3.4.1 stating what the resolver does when the winning candidate is an effect-kind method (either admit it, or reject with a specific diagnostic). Decision belongs to user.
- **Evidence check:** pass — §3.4.1's 8-step dispatch collects effect-kind methods as ordinary trait-impl candidates (005-78: exactly as value methods) with no branch to exclude or diagnose them, so x.f() where f is only an effect-kind method resolves to it as an ordinary call — which §3.1.1.1 forbids — with no error path or legal boundary.
- **Charity check:** sustain — Confirmed gap. The §3.4.1 algorithm step 1 (SPEC L2038-2042; 005-121) collects a trait-impl candidate 'for each trait T ... such that x's type fulfills T and T declares a method named f' — effect-kind methods are declared methods and 005-78 (L2038 vicinity) puts them in the effective set 'exactly as value methods do'. The 8 steps (L2038-2078) contain NO branch that detects an effect-kind candidate and emits a use-in-wrong-context diagnostic; step 4 would resolve song.render() to it. Corpus grep found exactly one restraining passage: §3.1.1.1 L1250-1253 ('invoked as an effect ... not as an ordinary function call'). Per gap standard this is not explicit normative text covering the resolution case: it states how effect-kind methods ARE invoked (intent) but supplies no algorithm step, no dropped-candidate rule, and no diagnostic for a written x.f(); it actually CONFLICTS with 005-78's 'exactly as value methods do' rather than resolving it. Inference/'obviously intended' is a sustain. Boundary check: no std-delegation or implementation-defined declaration covers this reachable input. Sustain.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:419-419`
    > 005-78. Effective method-set computation begins from an empty set and, for each trait the type fulfills — directly-satisfied (via `satisfies`/`fulfill`) and auto-satisfied (§3.3.5) — computes its closure under the `requires` relation: the trait plus every trait reachable through any chain of `requires` clauses. Effect-kind methods enter the effective method set exactly as value methods do. (§3.2.1)
  - `packages/ductus-lang/docs/SPEC.md:2038-2042`
    > 1. **Trait-impl search.** For each trait `T` reachable in the current scope
    >    (imported or accessible by path) such that `x`'s type fulfills `T` and `T`
    >    declares a method named `f`, collect the trait-impl candidate
    >    `T::f(x, ...)`. The function bodies live inside the corresponding `fulfill
    >    T for X` blocks.
  - `packages/ductus-lang/docs/SPEC.md:1245-1249`
    > - **Collision namespace.** Effect-method names join the ordinary
    >   method-name collision namespace — a trait's effect-method name collides
    >   with an `fn` method of the same name under the disjoint-names rule
    >   (§3.2.1), and effective-set entries sharing a name resolve by the same
    >   rules as any other method.

#### F067 — Effect-kind trait methods are simultaneously declared callable in the three ordinary call forms (LOG 005-114 + 017-273 `render(song)`) and forbidden as ordinary calls (SPEC 3.1.1.1), so a dispatcher cannot satisfy both.

- **Severity/Category/Verdict:** HIGH / contradiction / CONFIRMED
- **Anchors:** LOG 005-114, 005-68, 017-273 · SPEC § 3.1.1.1, 3.4
- **Why it is a defect:** 005-68 makes effect-kind methods trait methods; 005-114 says a trait method (no carve-out) is callable in three ordinary forms, and 017-273's own example `render(song).audio` uses the conventional call form `render(song)`. SPEC 3.1.1.1 flatly says effect-kind methods are NOT invoked as an ordinary function call. The two cannot both hold: either `render(song)` is a legal conventional call (LOG) or it is illegal (SPEC). An implementer writing name resolution has no rule to decide, and there is no legal boundary (001-6) declared.
- **Direction of change:** Decide and state uniformly whether effect-kind trait methods are callable in the three ordinary forms; make LOG 005-114/017-273 and SPEC 3.1.1.1 agree. Surface to user — do not resolve unilaterally.
- **Evidence check:** pass — 005-114/005-78 make an effect-kind trait method callable in ordinary call forms (and 017-273's own `render(song)` uses one), but SPEC §3.1.1.1 forbids ordinary invocation — jointly unsatisfiable for a dispatcher, no legal boundary per 001-6.
- **Charity check:** sustain — Core contradiction holds: LOG 005-114 states without carve-out that 'a trait method is callable in three equivalent forms' (person.display() / display(person) / Display::display(person)); 005-68 makes effect-kind methods trait methods; 005-78/005-84 place effect-kind methods into the effective method set 'exactly as value methods do'; SPEC §3.1.1.1 (1250-1253) forbids invoking an effect-kind method 'as an ordinary function call'. No LOG entry carries §3.1.1.1's restriction — 005-51 only constrains which KIND may satisfy the trait, not the call form — so an implementer has no rule to exclude effect-kind methods from 005-114's three forms. CAVEAT: the finding's 017-273 witness `render(song).audio |> audio_out` is MISCLASSIFIED as 'the conventional call form' — it sits in an `effects:` clause and IS the effect-invocation form §3.1.1.1 explicitly permits, so 017-273 alone does not contradict §3.1.1.1. The contradiction survives on the 005-114 (uncarved three-form) vs §3.1.1.1 half; the finder should drop or reframe the 017-273 citation.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:455-455`
    > 005-114. Under uniform function call syntax, a trait method is callable in three equivalent forms: `person.display()`, `display(person)`, `Display::display(person)`. (§3.4)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2414-2414`
    > 017-273. Interpretation bootstraps as an effect call in an `effects:` clause — `render(song).audio |> audio_out` — with projection written explicitly; there is no new top-level construct. (§13.3.8)
  - `packages/ductus-lang/docs/SPEC.md:1250-1253`
    > - **Effect-specific.** An effect-kind trait method is effect-specific: it
    >   is invoked as an effect (in `effects:`, bootstrapped via `|>`, §13.19),
    >   not as an ordinary function call. A trait carrying effect-kind methods
    >   is not thereby a general-purpose value trait.

#### F033 — Bare-name/method-call dispatch (005-120/121, §3.4.1) collects ANY trait method named f including effect-kind methods, but §3.1.1.1 forbids invoking effect-kind methods as ordinary f(x)/x.f() calls — an implementer has no rule to exclude them.

- **Severity/Category/Verdict:** HIGH / divergence / CONFIRMED
- **Anchors:** LOG 005-120, 005-121 · SPEC § 3.4.1, 3.1.1.1
- **Why it is a defect:** An effect-kind method declared with `effect render(value: Subject)` is a method named `render`. Under 005-121 / §3.4.1 step 1, a bare-name call `render(x)` or `x.render()` on a receiver whose type fulfills the trait MUST collect `T::render` as a candidate and (step 4) resolve to it as an ordinary call. But §3.1.1.1 states effect-kind methods are invoked only as effects via `|>`, never as ordinary function calls. The dispatch entries 005-120..137 and §3.4.1 contain zero mention of effect-kind methods (confirmed: no `effect` token in DECISION_LOG.md L461-478 nor in SPEC §3.4 L2000-2145). An implementer cannot satisfy both: either bare-name dispatch admits effect methods (violating §3.1.1.1) or it must exclude them (a rule that exists nowhere). This is a soundness/behavior contradiction with no legal boundary per 001-6.
- **Direction of change:** Surface to user: the dispatch resolution decisions (005-120/121, §3.4.1) need an explicit statement of whether effect-kind methods participate in bare-name/method-call resolution; if they must not (per §3.1.1.1), the exclusion has to be stated in the LOG and §3.4.1, not left implicit.
- **Evidence check:** pass — Bare-name/method-call dispatch (005-121/§3.4.1) collects any trait method named f including effect-kind methods, but §3.1.1.1 forbids ordinary f(x)/x.f() on effect methods; exclusion rule exists nowhere, so the two are unsatisfiable together.
- **Charity check:** sustain — Cleanest statement of the effect-kind contradiction, correctly cited. SPEC §3.4.1 step 1 (2038-2042) collects a candidate for EVERY trait T where 'T declares a method named f' with zero effect-kind exclusion — verbatim matching LOG 005-121 — and step 4 (2057-2064) resolves the winner as an ordinary call. An effect-kind method `effect render(...)` is a method named `render`, so bare-name `render(x)`/`x.render()` MUST collect and resolve it. SPEC §3.1.1.1 (1250-1253) forbids exactly that. Fresh read confirms neither SPEC §3.4/§3.4.1 (2000-2143) nor LOG 005-120..137 mentions effect-kind methods at all; no exclusion rule and no legal boundary (001-6). An implementer cannot satisfy both the dispatch algorithm and the §3.1.1.1 restriction. CONFIRMED soundness/behavior contradiction. HIGH.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:461-462`
    > 005-120. Resolution of a bare-name call `f(x)` or method-call `x.f()` prioritizes trait implementations over free functions. (§3.4.1)
    > 005-121. The trait-impl search collects a candidate `T::f` for every trait `T` reachable in the current scope (imported or accessible by path) such that the receiver's type fulfills `T` and `T` declares a method named `f`. (§3.4.1)
  - `packages/ductus-lang/docs/SPEC.md:2038-2042`
    > 1. **Trait-impl search.** For each trait `T` reachable in the current scope
    >    (imported or accessible by path) such that `x`'s type fulfills `T` and `T`
    >    declares a method named `f`, collect the trait-impl candidate
    >    `T::f(x, ...)`. The function bodies live inside the corresponding `fulfill
    >    T for X` blocks.
  - `packages/ductus-lang/docs/SPEC.md:1250-1253`
    > - **Effect-specific.** An effect-kind trait method is effect-specific: it
    >   is invoked as an effect (in `effects:`, bootstrapped via `|>`, §13.19),
    >   not as an ordinary function call. A trait carrying effect-kind methods
    >   is not thereby a general-purpose value trait.

#### F237 — SPEC blocks the fulfill-without-satisfies waiver when an effect-kind method carries an observed: contract, but the LOG waiver text gates only on node/connection 'required cells' and never says an observed: contract blocks it.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 005-67, 005-69, 005-71, 005-49 · SPEC § 3.2, 3.1.1.1, 3.3.5, 3.1.7
- **Why it is a defect:** The LOG defines 'required cell' (005-49, §3.1.7) exclusively as node/connection attr/const/derived/recurrent/stream/endpoint members. 005-67 and 005-71 gate the waiver on 'required cells' in that sense and never state that an effect-kind method's observed: contract (005-69, a distinct §3.1.1 notion of 'required output cells') blocks the waiver. SPEC 1976-1977 folds observed: contract cells INTO the waiver-blocking 'required cells' set. So for a trait whose only effect-kind method has a default body AND an observed: contract block, SPEC says the waiver does NOT apply (satisfies mandatory) while the LOG's literal text says it DOES apply. An implementer following the LOG and one following SPEC produce different compile-time acceptance for the same program. SPEC conforms to LOG is required; this is a LOG-SPEC divergence.
- **Direction of change:** Reconcile the two documents by deciding, at the LOG level, whether an effect-kind method's observed: contract block counts as a waiver-blocking 'required cell'; then make 005-67/005-71 (and the parallel auto-satisfaction rule 005-109) state that explicitly if yes, or make SPEC drop the observed:-contract clause if no. This is a user decision, not to be resolved here.
- **Evidence check:** pass — For a trait whose only effect-kind method has a default body plus an observed: contract, LOG's literal 'required cell' definition (005-49) admits the fulfill-without-satisfies waiver while SPEC §3.3.5 blocks it, yielding different compile-time acceptance.
- **Charity check:** sustain — Confirmed LOG-SPEC divergence (same underlying divergence as F010, program-behavior framing). LOG waiver text 005-67 and 005-71 gate the fulfill-without-satisfies waiver on 'required cells' (defined 005-49/§3.1.7 as node/connection members only) and never state that an effect-kind method's observed: contract (005-69, §3.1.1) blocks the waiver. SPEC L1974-1978 makes observed: contract cells block the waiver. Same trait (one defaulted effect-kind method with an observed: block) → LOG-follower accepts fulfill-without-satisfies, SPEC-follower requires satisfies. Different compile-time acceptance. SPEC-conforms-to-LOG is required; sustain.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:408-408`
    > 005-67. The satisfies/fulfill pairing requirement is waived — a trait may be fulfilled without a `satisfies` clause — for a trait where every method it declares, including every effect-kind method, has a default body and that declares no required cells. (§3.2)
  - `packages/ductus-lang/docs/DECISION_LOG.md:412-412`
    > 005-71. The `satisfies` clause remains mandatory for a trait with any abstract method (a method lacking a default body) and for a trait declaring any required cell; the fulfill-without-satisfies waiver never applies to such traits. (§3.2)
  - `packages/ductus-lang/docs/DECISION_LOG.md:390-390`
    > 005-49. Traits may declare required members that implementing types must provide: `attr`, `const`, `derived`, `recurrent`, and `stream` declarations, and — for connection types — `from` and `to` endpoints. (§3.1.7)
  - `packages/ductus-lang/docs/DECISION_LOG.md:410-410`
    > 005-69. An effect-kind trait method may declare an `observed:` contract block listing the required output cells it exposes; the contract is a MINIMUM — a fulfill may expose more cells than the contract lists. (§3.1.1)
  - `packages/ductus-lang/docs/SPEC.md:1974-1978`
    > effect-kind methods (§3.1.1.1): a trait in which **every** method —
    > including every effect-kind method — has a default body **and** which
    > declares **no required cells** (no `observed:` contract cells that a
    > fulfillment must supply, and no required node/connection members per
    > §3.1.7) permits a `fulfill` block **without** a matching `satisfies`

#### F014 — The satisfies-waiver (005-67) requires 'no required cells' but the stronger auto-satisfaction rule (005-109), which drops both satisfies and fulfill, imposes no such cell condition, leaving effect-kind traits with defaulted methods plus observed cells governed inconsistently.

- **Severity/Category/Verdict:** MED / ambiguity / CONFIRMED
- **Anchors:** LOG 005-67, 005-109, 005-71 · SPEC § 3.3.5
- **Why it is a defect:** 005-109 (auto-sat
isfaction: drops BOTH satisfies and fulfill) lists conditions but omits any 'no required cells' clause. 005-67 (weaker waiver: drops only satisfies, keeps fulfill) explicitly requires 'no required cells'. Take a trait with one defaulted effect-kind method carrying an observed: contract cell: under 005-109 it can auto-satisfy (all methods defaulted, no undefaulted assoc type) with neither satisfies nor fulfill; under 005-67 you may NOT drop satisfies if you write a fulfill (has required/observed cells). The two rules yield opposite conclusions about whether such a trait needs a written satisfies clause, depending on whether a fulfill is present. Whether observed cells count for 005-109 is unstated. Two readings produce different accept/reject behavior for the same trait.
- **Direction of change:** Align the cell condition across the two waivers: state explicitly whether auto-satisfaction (005-109) is also blocked by required/observed cells, so the two rules classify effect-kind traits with observed contracts consistently.
- **Evidence check:** pass — 005-109 (drops both satisfies and fulfill) imposes no 'no required cells' condition while 005-67 (drops only satisfies) explicitly requires it, so a trait with one defaulted effect-kind method carrying an observed cell is governed to opposite conclusions depending on whether a fulfill is present.
- **Charity check:** sustain — Confirmed inconsistency. 005-109 (auto-satisfaction, drops both satisfies and fulfill; SPEC L1944-1954) lists conditions and carries NO 'no required cells' clause; 005-67/005-71 (weaker waiver, keeps fulfill) explicitly require 'no required cells'. Grep for any auto-satisfaction restriction to no-required-cell / kind-agnostic traits found none. For a trait with a defaulted effect-kind method plus an observed:/attr cell the two rules give opposite answers on whether satisfies is needed, and whether observed cells count for 005-109 is unstated. Note: this finding shares premises with F010/F013 (whether observed cells are 'required cells', and whether a defaulted-method + required-cell trait can exist); the inconsistency is real regardless but is entangled with those two. When torn, sustain.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:408-408`
    > 005-67. The satisfies/fulfill pairing requirement is waived — a trait may be fulfilled without a `satisfies` clause — for a trait where every method it declares, including every effect-kind method, has a default body and that declares no required cells. (§3.2)
  - `packages/ductus-lang/docs/DECISION_LOG.md:450-450`
    > 005-109. A trait is automatically satisfied when all of its `requires` are satisfied, every method it declares — including every effect-kind method — has a default body, and every associated type it declares has a default; a trait with an undefaulted associated type requires explicit `satisfies` plus `fulfill`. (§3.3.5)
  - `packages/ductus-lang/docs/SPEC.md:1944-1954`
    > A trait is **automatically satisfied** — no explicit `satisfies` clause on
    > the type and no `fulfill` block for the trait itself are needed — when all of
    > the following hold: all of its `requires` are satisfied, *every method it
    > declares has a default body* (§3.1.3, i.e. it has no *abstract* method — a
    > method with no default body), *and every associated type it declares has a
    > default* (§3.3.2).

#### F013 — A trait declaring both an effect-kind method (effect-only per 005-51) and a required cell (node/connection-only per 005-50/005-59) is satisfiable by no single kind, and no in-scope rule forbids declaring both.

- **Severity/Category/Verdict:** MED / gap / CONFIRMED
- **Anchors:** LOG 005-51, 005-50, 005-59, 005-61, 005-67 · SPEC § 3.1.7, 3.3.5
- **Why it is a defect:** 005-51 pins an effect-kind-method trait to effect kinds only. 005-50/005-59/005-61 pin a required-cell trait to node/connection kinds only (and 005-59 bars records/enums/newtypes/primitives). A trait declaring BOTH an effect-kind method and a required attr/const is therefore satisfiable by no kind at all — jointly unsatisfiable. No entry in 005-1..119 forbids this combination or resolves the conflict, and there is no legal boundary (std-delegation or implementation-defined) covering it. Meanwhile 005-67's waiver phrasing ('every effect-kind method ... and that declares no required cells') implies effect traits with required cells are contemplated, deepening the ambiguity about whether such a trait is legal-but-unsatisfiable or should be rejected at declaration.
- **Direction of change:** Decide and state whether a trait mixing an effect-kind method with a required node/connection cell is a declaration-site error, and add the rule; if observed: cells are what 005-67 means, disambiguate that separately from node/connection required cells.
- **Evidence check:** pass — A trait declaring both an effect-kind method and a required cell is pinned to effect-only AND node/connection-only simultaneously by 005-51 vs 005-50/59, satisfiable by no kind; no in-scope rule forbids or resolves the combination and no legal boundary covers it.
- **Charity check:** sustain — Confirmed gap. §3.1.1.1 (SPEC L1250-1253) pins an effect-kind-method trait to effect kinds ('effect-specific', 'not thereby a general-purpose value trait'); §3.1.7 gating (SPEC L1556-1572) and 005-50/005-59/005-61 pin a required-cell trait to node/connection kinds and bar records/enums/newtypes/primitives. A trait declaring BOTH is satisfiable by no kind. Corpus-wide grep for any rule forbidding the combination ('may not declare both', 'single kind', 'jointly unsatisfiable', etc.) across SPEC.md and DECISION_LOG.md returned nothing on point. No legal boundary (001-6 std-delegation or implementation-defined) covers it. Per gap standard, no explicit normative text covers the case; sustain.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:392-392`
    > 005-51. A trait that declares an effect-kind method is effect-specific and may be satisfied only by an effect kind; kind-agnosticism holds only for traits with no effect-kind methods — a trait declaring only value methods and associated types is kind-agnostic and may be satisfied by any kind. (§3.1.7)
  - `packages/ductus-lang/docs/DECISION_LOG.md:391-391`
    > 005-50. A trait that declares any required cell or endpoint is kind-specific: a node trait, or a connection trait when it declares endpoints. (§3.1.7)
  - `packages/ductus-lang/docs/DECISION_LOG.md:400-402`
    > 005-59. A trait declaring any required cell or endpoint cannot be satisfied by a record, enum, newtype, or primitive; the compiler rejects the `satisfies`. (§3.1.7)
    > 005-60. A trait declaring a `from` or `to` endpoint cannot be satisfied by a node; the compiler rejects the `satisfies`. (§3.1.7)
    > 005-61. A trait with required cells but no endpoints is satisfiable by either nodes or connections. (§3.1.7)

#### F011 — 005-68 asserts an effect-kind method's first parameter 'is Subject-typed' as a hard requirement, but SPEC says it is only 'conventionally' Subject-typed and that trait methods may have any parameter list.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 005-68, 005-10 · SPEC § 3.1.1.1, 3.1.1
- **Why it is a defect:** LOG states as normative fact that the first parameter is Subject-typed; SPEC states it as convention ('conventionally') and elsewhere says trait methods may have any parameter list including no Subject. Two careful readings give different compiler behavior: one enforces first-param==Subject on effect-kind methods and rejects otherwise; the other permits any parameter list. SPEC must conform to LOG, so this is a behavior-changing LOG-SPEC divergence requiring adjudication of which is the rule.
- **Direction of change:** Decide whether first-param==Subject is mandatory or conventional for effect-kind methods and make LOG and SPEC state the same modality.
- **Evidence check:** pass — 005-68 states the effect-kind method first parameter 'is Subject-typed' as a hard requirement while SPEC states it is only 'conventionally' Subject-typed and that trait methods may have any parameter list, giving an implementer opposite enforcement rules.
- **Charity check:** sustain — Confirmed divergence, and SPEC is internally inconsistent too. 005-68 states as fact 'whose first parameter is Subject-typed'. SPEC §3.1.1.1 L1238 softens to 'conventionally Subject-typed', and §3.1.1 L1207-1211 says trait methods 'may have any parameter list — including no Subject parameter at all'. Meanwhile SPEC §13.19 Case-3 (L19931-19938) relies on the effect-kind method's 'Subject-typed first parameter' as a hard dispatch fact ('dispatched on the Subject type'; 'binds to the method's Subject-typed first parameter'). So SPEC says both 'conventionally' (§3.1.1.1) and treats it as mandatory (§13.19). The finding's cited SPEC quotes ('conventionally', 'any parameter list') are exact. LOG hard requirement vs SPEC §3.1.1.1 convention is a behavior-changing divergence; sustain.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:409-409`
    > 005-68. A trait may declare an effect-kind method with the form `effect name(params):` whose first parameter is `Subject`-typed; an effect-kind method declares an interpretation obligation that a fulfilling type discharges. (§3.1.1)
  - `packages/ductus-lang/docs/SPEC.md:1236-1238`
    > - An effect-kind method's body is an effect body (§13.19): it may place
    >   child effects and declare `desired:` / `observed:` blocks. Its first
    >   parameter is conventionally `Subject`-typed (the interpreted value).
  - `packages/ductus-lang/docs/SPEC.md:1207-1211`
    > Trait method signatures name their receiver parameter explicitly; there is
    > no implicit receiver. The first parameter's type is conventionally `Subject`
    > for methods that operate on instances, but trait methods may have any parameter
    > list — including no `Subject` parameter at all (for "associated functions" like
    > constructors).

#### F010 — 005-67 states the fulfill-without-satisfies waiver requires 'no required cells', but 'required cell' is defined elsewhere in the section only as node/connection members, while SPEC silently folds effect-method observed: cells into the term.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 005-67, 005-49, 005-50, 005-59, 005-69 · SPEC § 3.3.5, 3.1.7, 3.2
- **Why it is a defect:** Within LOG scope 'required cell' is defined only by 005-49/005-50 as attr/const/derived/recurrent/stream declarations (reactive node/connection structure). SPEC §3.3.5 broadens 'required cells' to also include an effect method's observed: contract cells. Nothing in the LOG restates this broadening, so 005-67 read on LOG terms alone excludes observed cells, but SPEC includes them. An implementer of the waiver gate gets two different answers for a trait whose only 'cell' is an observed: contract cell. This is a substantive LOG-SPEC divergence and, per Invariant 2, 005-67 is not self-contained about which sense of 'required cell' it means.
- **Direction of change:** Reconcile the term: either the LOG must restate that observed: contract cells count as 'required cells' for the 005-67 waiver, or SPEC must stop folding observed cells into 'required cells'; the two documents must state one meaning.
- **Evidence check:** pass — LOG scope defines 'required cell' (005-49) as node/connection attr/const/etc. only, but SPEC §3.3.5 folds effect-method observed: contract cells into the waiver-blocking 'required cells' set; 005-67 read on LOG terms alone yields a different waiver result than SPEC for a trait whose only cell is an observed cell.
- **Charity check:** sustain — Confirmed LOG-SPEC divergence. LOG defines 'required cell' only via 005-49 (§3.1.7) as attr/const/derived/recurrent/stream + from/to endpoints — node/connection members. SPEC L1974-1978 explicitly folds 'observed: contract cells that a fulfillment must supply' into the waiver-blocking 'required cells' set. Grep of all 005-* entries and every LOG 'observed'+waiver combination shows NO LOG entry restates this broadening; 005-69 calls observed cells 'required output cells', a distinct §3.1.1 notion never tied to the 005-67 waiver gate. On LOG terms 005-67 excludes observed cells from 'required cells'; SPEC includes them. Substantive divergence per Invariant-2 (005-67 not self-contained about which sense). Sustain.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:408-408`
    > 005-67. The satisfies/fulfill pairing requirement is waived — a trait may be fulfilled without a `satisfies` clause — for a trait where every method it declares, including every effect-kind method, has a default body and that declares no required cells. (§3.2)
  - `packages/ductus-lang/docs/DECISION_LOG.md:390-391`
    > 005-49. Traits may declare required members that implementing types must provide: `attr`, `const`, `derived`, `recurrent`, and `stream` declarations, and — for connection types — `from` and `to` endpoints. (§3.1.7)
    > 005-50. A trait that declares any required cell or endpoint is kind-specific: a node trait, or a connection trait when it declares endpoints. (§3.1.7)
  - `packages/ductus-lang/docs/SPEC.md:1976-1979`
    > declares **no required cells** (no `observed:` contract cells that a
    >   fulfillment must supply, and no required node/connection members per
    >   §3.1.7) permits a `fulfill` block **without** a matching `satisfies`
    >   clause.

#### F069 — SPEC Case-3 prose says the effect-kind method is the `|>` RHS with a node-reference LHS, but its own example and LOG 017-273 put the effect-kind method on the LHS as a conventional call `render(song)` with the RHS being a different effect/operator.

- **Severity/Category/Verdict:** MED / contradiction / CONFIRMED
- **Anchors:** LOG 017-273 · SPEC § 13.17.7, 13.3.8
- **Why it is a defect:** The Case-3 prose (19931, 19949 'the RHS is a trait method', 19953 'effect-kind-trait-method RHS') requires the effect-kind method to be the RHS of `|>` with the node reference as LHS. The example line 19943 `render(song) |> audio_out` has `render` (the effect-kind method) as a conventional call on the LHS and `audio_out` (a separate effect/operator) as the RHS; the comment 19944 even reclassifies `render(song)` as 'a node reference' that is 'piped into an effect-kind trait method,' implying audio_out is the method. LOG 017-273 confirms `render` is called conventionally and `audio_out` is downstream. The prose and the example disagree on which operand of `|>` is the effect-kind method; an implementer building `|>` dispatch cannot tell which Case-3 shape to match.
- **Direction of change:** Reconcile SPEC §13.17.7 Case-3 prose with its example and LOG 017-273: state unambiguously whether the effect-kind method is invoked conventionally (LHS) or as the `|>` RHS. User decides the canonical form.
- **Evidence check:** pass — SPEC Case-3 prose fixes the effect-kind trait method as the `|>` RHS with a bare node-reference LHS, but its own example and LOG 017-273 write `render(song).audio |> audio_out` where the interpretation method `render` is a conventional call on the LHS and the RHS is a separate downstream effect — an implementer building `|>` dispatch cannot tell which operand carries the effect-kind method.
- **Charity check:** sustain — Confirmed contradiction, and it is stronger than the finder stated. SPEC Case-3 prose (19931-19953) requires: LHS is a node reference, RHS is an effect-kind trait method whose Subject-typed first parameter accepts that node. But `render` IS an effect-kind trait method: SPEC 1231 declares `effect render(value: Subject):` and 1251 says such methods are 'invoked as an effect (in effects:, bootstrapped via |>)'. So in the example `render(song) |> audio_out` (19943), `render(song)` is the effect-kind method CALLED on the LHS producing an effect instance, and `audio_out` (RHS) is a plain downstream effect — matching the general |> rule (SPEC 19979: '|> may apply an operator or an effect') and LOG 017-273 (2414) `render(song).audio |> audio_out`. This directly contradicts the Case-3 prose, which would force `audio_out` (RHS) to be the Subject-parameter trait method and `render(song)` (LHS) to be a bare node reference. The SPEC comment at 19944 ('render(song): a node reference ... piped into an effect-kind trait method') tries to rescue the prose but is itself wrong given 1231. I checked whether `audio_out` is defined anywhere as an effect-kind trait method to salvage the prose — it is not (only occurrence is 19943); all other |> examples (14855-14881) put a plain effect on the RHS. An implementer building |> dispatch cannot tell which operand is the effect-kind method. | none found; SPEC.md:1231 declares `effect render(value: Subject):` and SPEC.md:19979 states '**|>** applicability: |> may apply an operator or an effect' — both force `render` (LHS) to be the effect-kind method and `audio_out` (RHS) a plain effect, contradicting the Case-3 prose at SPEC.md:19931-19953 that puts the effect-kind method on the RHS. The two SPEC passages conflict; the prose loses to its own definitions and to LOG 017-273.
- **Evidence:**
  - `packages/ductus-lang/docs/SPEC.md:19931-19939`
    > **Case 3: LHS is a node reference and RHS is an effect-kind trait
    > method** whose first (Subject-typed) parameter accepts that node
    > (§3.1.1.1, effect-kind trait methods). This is the *interpretation
    > bootstrap* form: an interpretation root is expressed by piping a node
    > reference into an effect-kind trait method that walks the node's
    > `.exposition` (§13.3.7). The method is dispatched on the Subject type
    > of the node reference; the LHS node reference binds to the method's
    > Subject-typed first parameter. The result is the effect instance
    > value the method evaluates to, accessed per §13.19.7:
  - `packages/ductus-lang/docs/SPEC.md:19943-19946`
    >   audio = render(song) |> audio_out
    >   // render(song): a node reference of Subject type Song is piped into
    >   // an effect-kind trait method that interprets it. Projection of the
    >   // walked result is explicit (here `render(song).audio`, elided).
  - `packages/ductus-lang/docs/DECISION_LOG.md:2414-2414`
    > 017-273. Interpretation bootstraps as an effect call in an `effects:` clause — `render(song).audio |> audio_out` — with projection written explicitly; there is no new top-level construct. (§13.3.8)

#### F070 — SPEC's interpretation-bootstrap example elides the `.audio` projection that LOG 017-273 mandates be written explicitly, and the SPEC comment simultaneously calls the projection both required-and-elided.

- **Severity/Category/Verdict:** LOW / stale_example / CONFIRMED
- **Anchors:** LOG 017-273 · SPEC § 13.17.7
- **Why it is a defect:** LOG 017-273 requires projection 'written explicitly' and shows `render(song).audio |> audio_out`. The SPEC example writes `render(song) |> audio_out` (no `.audio`) while its comment says projection 'is explicit (here render(song).audio, elided)' — self-contradictory (explicit yet elided) and divergent from the LOG's normative example, which per the project's LOG-first edit protocol is a defect.
- **Direction of change:** Make the SPEC §13.17.7 example match LOG 017-273 by writing the projection explicitly, or drop the 'explicit' claim; align with the LOG's canonical form.
- **Evidence check:** pass — SPEC bootstrap example drops the .audio projection the LOG mandates be explicit; SPEC comment calls it both explicit and elided.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2414-2414`
    > 017-273. Interpretation bootstraps as an effect call in an `effects:` clause — `render(song).audio |> audio_out` — with projection written explicitly; there is no new top-level construct. (§13.3.8)
  - `packages/ductus-lang/docs/SPEC.md:19943-19946`
    >   audio = render(song) |> audio_out
    >   // render(song): a node reference of Subject type Song is piped into
    >   // an effect-kind trait method that interprets it. Projection of the
    >   // walked result is explicit (here `render(song).audio`, elided).

#### F015 — 005-72 invokes 'the methodless-trait waiver' as a named mechanism, but no entry in scope defines a waiver by that name, leaving the load-bearing term unpinned at point of use.

- **Severity/Category/Verdict:** LOW / undefined_term / CONFIRMED
- **Anchors:** LOG 005-72 · SPEC § 3.2
- **Why it is a defect:** 005-72 names 'the methodless-trait waiver' as if it were an established term, but the phrase appears nowhere else in the section (the named waiver at 005-67 is the 'fulfill-without-satisfies waiver'; auto-satisfaction is 005-109-112, never called 'methodless-trait waiver'). Per Invariant 2 each entry must be self-contained; a reader cannot pin which rule 'the methodless-trait waiver' denotes without cross-referencing unnamed rules. Low severity because the intended referent (auto-satisfaction of methodless markers) is inferable.
- **Direction of change:** Either define the term 'methodless-trait waiver' where it is first used, or replace the phrase in 005-72 with a self-contained restatement of the auto-satisfaction condition it relies on.
- **Evidence check:** pass — 005-72 invokes 'the methodless-trait waiver' as a named mechanism but no in-scope entry defines a waiver by that name (005-67 is the fulfill-without-satisfies waiver), leaving the term unpinned at point of use.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:413-413`
    > 005-72. A `satisfies` clause for a methodless marker trait (auto-satisfiable under the methodless-trait waiver) may carry a `where` clause that makes conformance conditional on the type's parameters: `satisfies Copy where T: Copy`. The type satisfies the marker for exactly those instantiations whose parameters meet the bounds, and for no others; no `fulfill` block is involved. (§3.2)
  - `packages/ductus-lang/docs/DECISION_LOG.md:408-408`
    > 005-67. The satisfies/fulfill pairing requirement is waived — a trait may be fulfilled without a `satisfies` clause — for a trait where every method it declares, including every effect-kind method, has a default body and that declares no required cells. (§3.2)

#### F070 — SPEC's interpretation-bootstrap example elides the `.audio` projection that LOG 017-273 mandates be written explicitly, and the SPEC comment simultaneously calls the projection both required-and-elided.

- **Severity/Category/Verdict:** LOW / stale_example / CONFIRMED
- **Anchors:** LOG 017-273 · SPEC § 13.17.7
- **Why it is a defect:** LOG 017-273 requires projection 'written explicitly' and shows `render(song).audio |> audio_out`. The SPEC example writes `render(song) |> audio_out` (no `.audio`) while its comment says projection 'is explicit (here render(song).audio, elided)' — self-contradictory (explicit yet elided) and divergent from the LOG's normative example, which per the project's LOG-first edit protocol is a defect.
- **Direction of change:** Make the SPEC §13.17.7 example match LOG 017-273 by writing the projection explicitly, or drop the 'explicit' claim; align with the LOG's canonical form.
- **Evidence check:** pass — SPEC bootstrap example drops the .audio projection the LOG mandates be explicit; SPEC comment calls it both explicit and elided.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2414-2414`
    > 017-273. Interpretation bootstraps as an effect call in an `effects:` clause — `render(song).audio |> audio_out` — with projection written explicitly; there is no new top-level construct. (§13.3.8)
  - `packages/ductus-lang/docs/SPEC.md:19943-19946`
    >   audio = render(song) |> audio_out
    >   // render(song): a node reference of Subject type Song is piped into
    >   // an effect-kind trait method that interprets it. Projection of the
    >   // walked result is explicit (here `render(song).audio`, elided).

#### F238 — The same LOG-vs-SPEC gap recurs for auto-satisfaction: 005-109 auto-satisfies a trait whose effect-kind method has a default body without regard to any observed: contract, yet SPEC's waiver reasoning treats observed: contract cells as requiring explicit fulfillment.

- **Severity/Category/Verdict:** LOW / divergence / CONFIRMED
- **Anchors:** LOG 005-109, 005-69 · SPEC § 3.3.5, 3.1.1.1
- **Why it is a defect:** 005-109 (auto-satisfaction) lists only three gates: requires satisfied, every method (incl. effect-kind) defaulted, every associated type defaulted. It says nothing about an effect-kind method's observed: contract. Under 005-109's literal text, a trait with one effect-kind method that has a default body and an observed: contract auto-satisfies. But SPEC (1976-1978) treats observed: contract cells as cells 'a fulfillment must supply', which for the waiver blocks fulfill-without-satisfies; the same logic implies auto-satisfaction of such a trait would leave the contract cells unfulfilled. The LOG gives no rule closing this for auto-satisfaction. This is the auto-satisfaction sibling of the waiver divergence and hinges on the same undecided question.
- **Direction of change:** Once the waiver question (finding 1) is decided, mirror the decision into 005-109 so auto-satisfaction and the waiver treat effect-kind observed: contracts consistently.
- **Evidence check:** pass — 005-109 auto-satisfies a trait whose effect-kind method has a default body without regard to observed: contract, yet SPEC treats observed: contract cells as requiring fulfillment; LOG gives no rule closing this for auto-satisfaction.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:450-450`
    > 005-109. A trait is automatically satisfied when all of its `requires` are satisfied, every method it declares — including every effect-kind method — has a default body, and every associated type it declares has a default; a trait with an undefaulted associated type requires explicit `satisfies` plus `fulfill`. (§3.3.5)
  - `packages/ductus-lang/docs/DECISION_LOG.md:410-410`
    > 005-69. An effect-kind trait method may declare an `observed:` contract block listing the required output cells it exposes; the contract is a MINIMUM — a fulfill may expose more cells than the contract lists. (§3.1.1)
  - `packages/ductus-lang/docs/SPEC.md:1976-1978`
    > declares **no required cells** (no `observed:` contract cells that a
    > fulfillment must supply, and no required node/connection members per
    > §3.1.7) permits a `fulfill` block **without** a matching `satisfies`

#### F015 — 005-72 invokes 'the methodless-trait waiver' as a named mechanism, but no entry in scope defines a waiver by that name, leaving the load-bearing term unpinned at point of use.

- **Severity/Category/Verdict:** LOW / undefined_term / CONFIRMED
- **Anchors:** LOG 005-72 · SPEC § 3.2
- **Why it is a defect:** 005-72 names 'the methodless-trait waiver' as if it were an established term, but the phrase appears nowhere else in the section (the named waiver at 005-67 is the 'fulfill-without-satisfies waiver'; auto-satisfaction is 005-109-112, never called 'methodless-trait waiver'). Per Invariant 2 each entry must be self-contained; a reader cannot pin which rule 'the methodless-trait waiver' denotes without cross-referencing unnamed rules. Low severity because the intended referent (auto-satisfaction of methodless markers) is inferable.
- **Direction of change:** Either define the term 'methodless-trait waiver' where it is first used, or replace the phrase in 005-72 with a self-contained restatement of the auto-satisfaction condition it relies on.
- **Evidence check:** pass — 005-72 invokes 'the methodless-trait waiver' as a named mechanism but no in-scope entry defines a waiver by that name (005-67 is the fulfill-without-satisfies waiver), leaving the term unpinned at point of use.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:413-413`
    > 005-72. A `satisfies` clause for a methodless marker trait (auto-satisfiable under the methodless-trait waiver) may carry a `where` clause that makes conformance conditional on the type's parameters: `satisfies Copy where T: Copy`. The type satisfies the marker for exactly those instantiations whose parameters meet the bounds, and for no others; no `fulfill` block is involved. (§3.2)
  - `packages/ductus-lang/docs/DECISION_LOG.md:408-408`
    > 005-67. The satisfies/fulfill pairing requirement is waived — a trait may be fulfilled without a `satisfies` clause — for a trait where every method it declares, including every effect-kind method, has a default body and that declares no required cells. (§3.2)

#### F070 — SPEC's interpretation-bootstrap example elides the `.audio` projection that LOG 017-273 mandates be written explicitly, and the SPEC comment simultaneously calls the projection both required-and-elided.

- **Severity/Category/Verdict:** LOW / stale_example / CONFIRMED
- **Anchors:** LOG 017-273 · SPEC § 13.17.7
- **Why it is a defect:** LOG 017-273 requires projection 'written explicitly' and shows `render(song).audio |> audio_out`. The SPEC example writes `render(song) |> audio_out` (no `.audio`) while its comment says projection 'is explicit (here render(song).audio, elided)' — self-contradictory (explicit yet elided) and divergent from the LOG's normative example, which per the project's LOG-first edit protocol is a defect.
- **Direction of change:** Make the SPEC §13.17.7 example match LOG 017-273 by writing the projection explicitly, or drop the 'explicit' claim; align with the LOG's canonical form.
- **Evidence check:** pass — SPEC bootstrap example drops the .audio projection the LOG mandates be explicit; SPEC comment calls it both explicit and elided.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2414-2414`
    > 017-273. Interpretation bootstraps as an effect call in an `effects:` clause — `render(song).audio |> audio_out` — with projection written explicitly; there is no new top-level construct. (§13.3.8)
  - `packages/ductus-lang/docs/SPEC.md:19943-19946`
    >   audio = render(song) |> audio_out
    >   // render(song): a node reference of Subject type Song is piped into
    >   // an effect-kind trait method that interprets it. Projection of the
    >   // walked result is explicit (here `render(song).audio`, elided).

### Streams: policy contradiction and lifecycle leaks

StreamPolicy is four members in one entry, exactly two everywhere else; a gate stream's slowest-cursor hold leaks permanently on scope drop; phantom RecurrentRing/GateStream aliases; take(n) cursor fate and stream completion semantics unstated; a leaked amendment-plan label (C15) in a normative entry.

#### F111 — A gate stream consumer inside a repeat scope whose key is removed (scope_drop) has no rule releasing the slowest-cursor buffer hold; the only release mechanism (@reset_on_reopen) is keyed to a reopen edge a dropped scope never reaches.

- **Severity/Category/Verdict:** HIGH / gap / CONFIRMED
- **Anchors:** LOG 030-13, 030-188, 030-193, 030-196, 018-78, 018-8 · SPEC § 13.18.12, 13.18.1, 13.5.4.2, 13.5.1
- **Why it is a defect:** Scenario: a repeat over a Map places a node under a gated arm; the node consumes an outer gate stream (an upstream shared buffer, not one declared inside the retiring scope). The node's cursor pins that gate buffer via slowest-cursor retention (030-188/030-193). When the Map key is removed, 018-78 drops the scope. Section 030 specifies cursor lifecycle for exactly three situations: normal advance (030-183), gate-freeze/resume (030-191..196), and reload add/remove (030-219..221). It says nothing about a consumer retired by scope_drop. Two readings diverge with opposite program behavior: (a) the retired cursor is removed from the retention set, releasing the hold; (b) the hold persists (the buffer stays pinned, producers stay rejected — a leaked back-pressure / effectively-leaked buffer slot for the shared stream's lifetime). Nothing forbids the construct (a gate stream consumed from a repeat body is legal), and 030-13 'freed when that scope dies' governs only a stream DECLARED in the dying scope, not the retiring consumer's cursor/hold on an outer stream. The only hold-release rule, 030-196, requires a gate REOPEN edge; a dropped scope never reopens, so @reset_on_reopen cannot even be the intended remedy. This is an implementer-blocking gap with no legal boundary (001-6).
- **Direction of change:** Add a rule (in 030, mirroring the effect-teardown rule 018-110 'suspend, resume, and tear down with the element key') specifying what happens to a stream consumer's cursor — and, for gate consumers, the slowest-cursor buffer hold / back-pressure — when the consumer's enclosing repeat scope is retired via scope_drop (distinct from a reopenable gate-freeze).
- **Evidence check:** pass — A gate-stream consumer inside a repeat scope removed by scope_drop has no rule releasing its slowest-cursor buffer hold on an outer shared stream; the only release mechanism (@reset_on_reopen) is keyed to a reopen edge the dropped scope never reaches.
- **Charity check:** sustain — Genuine implementer-blocking gap; the charitable hunt found only reload-scoped rules that require inference to extend, which is a SUSTAIN for a gap. Every 'removed consumer's cursor is dropped' rule is explicitly RELOAD-scoped: LOG 028-49 (§13.15.5), LOG 030-221 'A consumer removed by a RELOAD has its cursor dropped' (DECISION_LOG.md:3727, §13.18.14), SPEC 19455 and 21620 both under 'Cursor identity across RELOAD.' F111's scenario is a runtime scope_drop from repeat-key removal (018-78), not a code reload — no rule covers it. The runtime scope_drop path (§13.5.1, SPEC:15145) invokes Drop only on the scope's STATE cells (attrs+recurrents, §13.5.2), while a consumer's cursor is 'part of the consumer's per-instance state' (SPEC:21526) whose release on scope_drop is nowhere stated. 030-13 'freed when that scope dies' governs a stream DECLARED in the dying scope, not an outer shared gate stream's cursor held by a retiring consumer. The only explicit gate-hold-release mechanism is @reset_on_reopen (030-196, SPEC:21500-21504), keyed to a gate REOPEN edge that a dropped scope never reaches. Two readings (hold released vs. buffer permanently pinned / producers permanently rejected) diverge in observable behavior with no covering normative text and no legal 001-6 boundary. Sustained HIGH.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:3694-3694`
    > 030-188. Gate streams are lossless per consumer: retention is slowest-cursor-driven, and no event is evicted until every consumer has observed it. (§13.18.12)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3699-3699`
    > 030-193. A frozen gate consumer pins the buffer via slowest-cursor retention, so the buffer fills and producers are rejected (`rejected_total`, `is_full`) for the duration of the freeze. (§13.18.12)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3702-3702`
    > 030-196. On a gate consumer, `@reset_on_reopen` additionally releases the buffer hold during the freeze, so the frozen consumer neither pins the buffer nor back-pressures producers. (§13.18.12)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3519-3519`
    > 030-13. A stream's lifetime is scope-bound: it lives as long as its declaration's enclosing scope and is freed when that scope dies. (§13.18.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2548-2548`
    > 018-78. Keys in `old − new` are dropped: `scope_drop(key)` releases the per-key cells; this diff applies uniformly to derived keys and to carried keys arriving from a `dynamic` view or a `repeat`-expression source. (§13.5.4.2)
  - `packages/ductus-lang/docs/SPEC.md:21500-21504`
    > The `@reset_on_reopen` decorator (§13.2.4) opts the consumer out of
    > backlog: on resume its cursor skips to the current head (discarding gap
    > events), and — for a **gate** consumer — it additionally *releases the
    > buffer hold during the freeze*, so a reset-annotated gate consumer does
    > not pin the buffer or back-pressure producers while frozen.

#### F199 — 030-37 says StreamPolicy has 'four members', but 030-253/254/257/258 and SPEC §13.18.3 fix it at exactly two (Ring[N], Gate[N]) with recurrence as an orthogonal history axis — a two-member vs four-member sealed trait an implementer cannot build both of.

- **Severity/Category/Verdict:** HIGH / contradiction / CONFIRMED (x5 independent reports)
- **Anchors:** LOG 030-253, 030-254, 030-257, 030-260 · SPEC § 13.18.3
- **Why it is a defect:** 030-37 asserts four StreamPolicy members. 030-253/254/257/260 and SPEC §13.18.3 assert exactly two (Ring[N], Gate[N]) and explicitly state RecurrentRing/RecurrentGate are no longer members. An implementer cannot satisfy both a two-member sealed trait and a four-member one; the runtime policy enum has a single fixed cardinality. 030-37 is a survivor of the amendment that collapsed four policies to two axes and was not updated.
- **Direction of change:** Reconcile 030-37 with the two-axis model so it states exactly two policy members (Ring[N], Gate[N]), not four. Surface to user as the four-vs-two contradiction; do not self-resolve.
- **Evidence check:** pass — 030-37 says StreamPolicy has four members while 030-253/254/257/258 and SPEC 13.18.3 fix it at exactly two (Ring[N], Gate[N]) with recurrence as an orthogonal history axis — an implementer cannot build both a two-member and a four-member sealed trait.
- **Charity check:** sustain — Jointly-unsatisfiable cardinality for a sealed trait; a clean HIGH contradiction with no rescuing text. LOG 030-37 (DECISION_LOG.md:3543): 'Every stream value has a concrete policy P (one of the FOUR StreamPolicy members) at runtime.' LOG 030-254 (3760) and 030-257 (3763): members are 'exactly Ring[N] and Gate[N]' — TWO. SPEC §13.18.3 sides with two and is emphatic: SPEC:20433-20435 'The *only* two sealed StreamPolicy members' and SPEC:20452 'There is **no** RecurrentRing / RecurrentGate StreamPolicy member,' with the fulfill block (SPEC:20443-20449) listing exactly Ring and Gate. A sealed trait's member set has a single fixed cardinality; an implementer cannot build both a 4-way and a 2-way discriminant. 030-37 is stale wording surviving the two-axis amendment (030-253/254 explicitly retire the recurrent members). Both a LOG-internal contradiction and a LOG-SPEC divergence. No charitable reading reconciles 'four' with 'exactly two.' Sustained HIGH.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:3543-3543`
    > 030-37. Every stream value has a concrete policy `P` (one of the four `StreamPolicy` members) at runtime. (§13.18.3)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3760-3760`
    > 030-254. `Ring[N]` / `Gate[N]` are the only stream policies; a recurrent stream is an ordinary `ring`/`gate` stream whose positive self-history depth is an orthogonal stream parameter, not a distinct policy. (§13.18.3)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3763-3763`
    > 030-257. `StreamPolicy` is a sealed (language-closed) trait, like `Into`/`TryInto`; its members are exactly `Ring[N]` and `Gate[N]`, and users cannot add policies. (§13.18.3)
  - `packages/ductus-lang/docs/SPEC.md:20433-20435`
    > - **Buffer discipline** — `Ring[N]` or `Gate[N]`. The *only* two
    >   sealed `StreamPolicy` members. `Ring` overwrites the oldest
    >   unconsumed event when full; `Gate` rejects the write when full.
  - `packages/ductus-lang/docs/DECISION_LOG.md:3543-3543`
    > 030-37. Every stream value has a concrete policy `P` (one of the four `StreamPolicy` members) at runtime. (§13.18.3)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3760-3760`
    > 030-254. `Ring[N]` / `Gate[N]` are the only stream policies; a recurrent stream is an ordinary `ring`/`gate` stream whose positive self-history depth is an orthogonal stream parameter, not a distinct policy. (§13.18.3)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3763-3763`
    > 030-257. `StreamPolicy` is a sealed (language-closed) trait, like `Into`/`TryInto`; its members are exactly `Ring[N]` and `Gate[N]`, and users cannot add policies. (§13.18.3)
  - `packages/ductus-lang/docs/SPEC.md:20452-20454`
    > There is **no** `RecurrentRing` / `RecurrentGate` StreamPolicy member.
    > A recurrent stream declaration (§13.18.8) is a `Ring[N]` or `Gate[N]`
    > policy stream carrying a non-zero history depth `H` — the two axes
  - `packages/ductus-lang/docs/DECISION_LOG.md:3543-3543`
    > 030-37. Every stream value has a concrete policy `P` (one of the four `StreamPolicy` members) at runtime. (§13.18.3)
  - `packages/ductus-lang/docs/SPEC.md:20430-20435`
    > The policy is modelled on **two orthogonal
    > axes** rather than as a flat enumeration of four members:
    > 
    > - **Buffer discipline** — `Ring[N]` or `Gate[N]`. The *only* two
    >   sealed `StreamPolicy` members. `Ring` overwrites the oldest
    >   unconsumed event when full; `Gate` rejects the write when full.
  - `packages/ductus-lang/docs/SPEC.md:20452-20452`
    > There is **no** `RecurrentRing` / `RecurrentGate` StreamPolicy member.
  - `packages/ductus-lang/docs/DECISION_LOG.md:3543-3543`
    > 030-37. Every stream value has a concrete policy `P` (one of the four `StreamPolicy` members) at runtime. (§13.18.3)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3763-3763`
    > 030-257. `StreamPolicy` is a sealed (language-closed) trait, like `Into`/`TryInto`; its members are exactly `Ring[N]` and `Gate[N]`, and users cannot add policies. (§13.18.3)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3760-3760`
    > 030-254. `Ring[N]` / `Gate[N]` are the only stream policies; a recurrent stream is an ordinary `ring`/`gate` stream whose positive self-history depth is an orthogonal stream parameter, not a distinct policy. (§13.18.3)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3759-3759`
    > 030-253. A stream policy is modeled on two independent axes: the buffer policy — `Ring[N]` or `Gate[N]` — and a self-history depth parameter that defaults to `0`; a positive history depth carries the `H`-deep self-history as a separate allocation, so `RecurrentRing`/`RecurrentGate` are no longer distinct policy members. (§13.18.3)
  - `packages/ductus-lang/docs/SPEC.md:20430-20435`
    > The policy is modelled on **two orthogonal
    > axes** rather than as a flat enumeration of four members:
    > 
    > - **Buffer discipline** — `Ring[N]` or `Gate[N]`. The *only* two
    >   sealed `StreamPolicy` members. `Ring` overwrites the oldest
    >   unconsumed event when full; `Gate` rejects the write when full.
  - `packages/ductus-lang/docs/DECISION_LOG.md:3543-3543`
    > 030-37. Every stream value has a concrete policy `P` (one of the four `StreamPolicy` members) at runtime. (§13.18.3)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3760-3760`
    > 030-254. `Ring[N]` / `Gate[N]` are the only stream policies; a recurrent stream is an ordinary `ring`/`gate` stream whose positive self-history depth is an orthogonal stream parameter, not a distinct policy. (§13.18.3)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3763-3763`
    > 030-257. `StreamPolicy` is a sealed (language-closed) trait, like `Into`/`TryInto`; its members are exactly `Ring[N]` and `Gate[N]`, and users cannot add policies. (§13.18.3)
  - `packages/ductus-lang/docs/SPEC.md:20452-20457`
    > There is **no** `RecurrentRing` / `RecurrentGate` StreamPolicy member.
    > A recurrent stream declaration (§13.18.8) is a `Ring[N]` or `Gate[N]`
    > policy stream carrying a non-zero history depth `H` — the two axes
    > combine (`recurrent[H] stream ring[N] T` in kind form, §13.2.8),
    > rather than selecting a fourth sealed policy tag. When `H` is omitted
    > it defaults to 0 and the stream is an ordinary non-recurrent stream.
  - `packages/ductus-lang/docs/SPEC.md:20443-20449`
    > trait StreamPolicy                                   // sealed (language-closed)
    > 
    > type Ring[const N: usize]                            // ring policy, buffer capacity N
    > type Gate[const N: usize]                            // gate policy, buffer capacity N
    > 
    > fulfill StreamPolicy for Ring[N]
    > fulfill StreamPolicy for Gate[N]

#### F200 — Entry 030-137 cites a 'C15 naming rule' that is defined nowhere in DECISION_LOG.md or SPEC.md; C15 is an internal amendment-plan change-set label that leaked into a normative entry.

- **Severity/Category/Verdict:** MED / undefined_term / CONFIRMED
- **Anchors:** LOG 030-137 · SPEC § 13.18.9
- **Why it is a defect:** 'C15' appears in no LOG entry and no SPEC section — only in scratchpad/plans/amendment-plan.md as a change-set marker. The entry is not self-contained: a reader cannot resolve 'the C15 naming rule'. The cited SPEC §13.18.9 attributes the count-name reservation to the element-tally rule §9.3.7, not to any 'C15' rule, so the LOG entry both dangles and diverges from its own cited elaboration.
- **Direction of change:** Replace the 'C15 naming rule' reference with the actual normative rule the SPEC uses (the bare-`count` element-tally reservation, §9.3.7), stated in self-contained atomic form. Surface the leaked change-set label to user.
- **Evidence check:** pass — 030-137 cites a 'C15 naming rule' defined nowhere in LOG or SPEC (an internal amendment-plan change-set label), so the entry is not self-contained and diverges from its cited SPEC section, which attributes the reservation to §9.3.7.
- **Charity check:** sustain — Undefined-term + LOG-SPEC divergence, both confirmed. grep of the full corpus shows 'C15' appears in EXACTLY one place: LOG 030-137 (DECISION_LOG.md:3643). It occurs in no SPEC section and no other LOG entry, so 'the C15 naming rule' is unresolvable — the entry is not self-contained (violates LOG Invariant 2). The actual rule is LOG 012-92 'The tally-accessor naming rule' (§9.3.7). 030-137's own cited elaboration SPEC §13.18.9 (SPEC:21111-21112) attributes the count-name reservation to 'the element-tally rule, §9.3.7' — NOT to any 'C15' rule — so the LOG entry both dangles and diverges from the section it points into. 'C15' is a change-set label that leaked into a normative entry. No passage anywhere defines a 'C15 naming rule.' Sustained MED.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:3643-3643`
    > 030-137. `event_count[T](source: stream T) -> derived i64` is the running count of observed events, starting at `0`; the `event_` prefix applies the C15 naming rule to this specialized tally. (§13.18.9)
  - `packages/ductus-lang/docs/SPEC.md:21111-21112`
    > The stream projection formerly named `count` is `event_count` (the
    > bare `count` name is reserved for the element-tally rule, §9.3.7); the

#### F128 — 029-124's 'excluded at the read site' phrase has no concrete read-site rule or diagnostic for the operator-body case; the only stream-read-as-value diagnostic is for a bare stream in a derived binding, not a cell-T param read.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 029-124, 016-283, 030-50 · SPEC § 13.18.5, 13.17.12, 13.18.7.6
- **Why it is a defect:** 029-124 and §13.18.5 (line 20555-20562) assert a stream is 'excluded at the read site' inside a value-reading operator, but never define WHERE that read site is or WHAT diagnostic fires for an operator body. The one concrete 'stream read as a value' diagnostic (SPEC 21743-21752) targets `derived latest: Event = events` — a bare stream reference directly in a value binding, NOT `source * rate` inside an operator body where `source: cell f32` is bound to a stream by the umbrella. §13.17.12's operator diagnostics (line 20140) enumerate five classes and none covers this. So the normative claim 'excluded at the read site' is unfalsifiable for the operator case: there is no operable rule an implementer can point to.
- **Direction of change:** Pin the read-site exclusion to a specific mechanism/diagnostic that applies inside operator bodies (auto-deref rejecting a stream at a T-expecting position, or a §13.17.12 diagnostic class), and cross-reference it from 029-124 / §13.18.5.
- **Evidence check:** pass — The normative 'a stream is excluded at the read site' (029-124 / §13.18.5) is unfalsifiable for the operator-body case: the only stream-read-as-value diagnostic targets a bare stream in a derived binding, and §13.17.12's enumerated operator diagnostics do not cover reading a stream-bound cell-T parameter as a value inside an operator body.
- **Charity check:** refile_divergence — Filed as a gap ('no operable rule for the read-site exclusion in an operator body'), but there ARE two operable rules and they CONFLICT, so it mutates into a divergence. SPEC 13.18.5 (20555-20557) — the finding's own quoted passage — says a stream-bound value-reading param 'has no current value, so it is excluded at the read site rather than by the annotation', i.e. reading it in a value position is rejected. SPEC 13.18.7.1 (20624-20628) says the OPPOSITE for the same situation: a stream in a reactive expression 'contributes its events. The surrounding expression is re-evaluated once per event, producing one output event per input event' — so `source * rate` with `source` a stream is a VALID stream-producing expression, not an excluded read. SPEC 13.18.2 (20370-20374) confirms `stream_a * scalar` is a legal stream source. Per the refile rule: because the dissolving passage (13.18.7.1) contradicts the finding's quoted passage (13.18.5) on whether a stream in a value-arithmetic expression is excluded or consumed-as-events, the verdict is refile_divergence, not refute. (The finding also missed SPEC 21735's 'stream-valued expression to signal binding' diagnostic, but that catches the RETURN/binding, not a 'read site', and does not resolve the 13.18.5-vs-13.18.7.1 conflict.) | packages/ductus-lang/docs/SPEC.md:20624-20628 (dissolving/conflicting passage) "A reactive input to an expression participates uniformly: - **A stream** contributes its events. The surrounding expression is re-evaluated once per event, producing one output event per input event." ; CONFLICTS WITH the finding's passage packages/ductus-lang/docs/SPEC.md:20555-20557 "A value-reading operator parameter is annotated `cell T` (§13.2.8); a `stream T` has no current value, so it is excluded at the read site rather than by the annotation."
- **Evidence:**
  - `packages/ductus-lang/docs/SPEC.md:20555-20562`
    > **Use in generic signatures.** A value-reading operator parameter is
    > annotated `cell T` (§13.2.8); a `stream T` has no current value, so it
    > is excluded at the read site rather than by the annotation. Operators
    > returning a value cell declare the concrete kind through their return
    > annotation — typically `derived T` for a computed output, `stream T`
    > for a stream, or `cell T` only for a genuine passthrough. Concrete
    > kinds carry more information and are preferred unless the operator
    > genuinely produces a polymorphic output.
  - `packages/ductus-lang/docs/SPEC.md:21743-21752`
    > **Stream read as a value (no expression context):**
    > 
    > ```
    > error: cannot read a `stream T` as a value
    >   --> derived latest: Event = events
    >                               ^^^^^^ this is a stream, not a value cell
    >   hint: streams have no current value. Project to a signal via
    >         `to_signal`, or fold the stream:
    >         `derived latest = events |> to_signal(default_event)`
  - `packages/ductus-lang/docs/SPEC.md:20140-20143`
    > #### 13.17.12 Diagnostics
    > 
    > Normative diagnostic classes for operator usage:

#### F082 — After a take(n) consumer completes on a gate stream, whether its cursor keeps advancing (releasing gate retention) or freezes at n (permanently back-pressuring producers) is unspecified.

- **Severity/Category/Verdict:** MED / gap / CONFIRMED
- **Anchors:** LOG 030-134, 030-188, 030-189 · SPEC § 13.18.9, 13.18.12
- **Why it is a defect:** A `take(n)` consumer of a gate stream stops emitting after n events (030-134). Gate retention is slowest-cursor-driven and a producer is rejected once the buffer is full of events unobserved by the slowest cursor (030-188/189). Whether a completed take() consumer's cursor keeps advancing (draining/releasing its hold) or freezes at its n-th position is not stated anywhere in sections 030/13.18. The two readings diverge in observable behavior: a frozen take-cursor pins the gate buffer and permanently rejects all producer pushes (rejected_total climbs forever, is_full stuck true); an advancing take-cursor releases the buffer. This must be decided before implementing take over gate streams. The freeze-and-backlog rules (030-191, SPEC 13.18.12) cover a gated-off subtree, not a logically-completed consumer.
- **Direction of change:** State the cursor behavior of a completed subset consumer (take, and analogously skip/filter that drop events) on a gate stream — specifically whether a completed take() cursor continues to advance to release slowest-cursor retention. Do not pick a default; surface to user.
- **Evidence check:** pass — After a take(n) consumer completes on a gate stream, whether its cursor keeps advancing (releasing retention) or freezes at n (permanently rejecting producers) is unspecified, yielding divergent observable rejected_total/is_full.
- **Charity check:** sustain — No dissolving text; behavior-changing gap. LOG 030-134 and SPEC §13.18.9 (SPEC:21077-21078) specify only the OUTPUT stream after take(n) ('the output stream is complete after n events / emits no more'). Neither §13.18.9 nor §13.18.12 (consumer cursors) specifies what happens to the completed consumer's cursor on the SOURCE stream: whether it keeps advancing (draining gate retention, releasing the hold) or freezes at position n (permanently pinning the gate buffer so rejected_total climbs forever and is_full stays true, per 030-188/030-189). The freeze-and-backlog rules (SPEC:21484-21498) cover a gate-OFF gated subtree, not a logically-completed consumer — different trigger. The two readings diverge in observable producer behavior (permanently blocked vs. released). Charitable hunt: 'output stream is complete' describes output emission, not source-cursor retention, so it does not dissolve the gap. Sustained MED.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:3640-3640`
    > 030-134. `take[T, P: StreamPolicy](source: Stream[T, P], n: i32) -> Stream[T, P]` emits the first `n` events, after which the output stream is complete and emits no more. (§13.18.9)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3694-3694`
    > 030-188. Gate streams are lossless per consumer: retention is slowest-cursor-driven, and no event is evicted until every consumer has observed it. (§13.18.12)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3695-3695`
    > 030-189. When a gate buffer holds `capacity` events unobserved by the slowest cursor, further producer pushes are rejected — `rejected_total` increments and `is_full` is true. (§13.18.12)

#### F193 — 030-116 (and 030-36 listing 'recurrent') assert named alias types RecurrentRingStream/RecurrentGateStream exist, but 030-260 and SPEC §13.18.3/§13.18.8.6 state no such aliases exist — a recurrent stream is just a Ring/Gate stream with history depth.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED (x3 independent reports)
- **Anchors:** LOG 030-116, 030-36 · SPEC § 13.18.8.6, 13.18.3
- **Why it is a defect:** 030-116 cites §13.18.8.6 as its elaboration, but that section says the recurrent stream produces a Ring[B]/Gate[B] stream with history depth H and names NO alias types; §13.18.3 explicitly states 'There are no RecurrentRing / RecurrentGate aliases'. grep of SPEC.md finds zero occurrences of RecurrentRingStream/RecurrentGateStream. So the LOG asserts stdlib alias types that the SPEC affirmatively denies exist. 030-36's parenthetical '(likewise GateStream/recurrent)' commits the same error, implying a recurrent alias parallel to RingStream/GateStream. Pointer contract broken: the cited section elaborates the opposite. An implementer would look for RecurrentRingStream[T,B,H] in the stdlib alias set and find nothing.
- **Direction of change:** Decide (owner call) whether recurrent-stream aliases exist. If they do not (matching current SPEC and the two-axis amendment), 030-116 and 030-36's parenthetical should stop naming RecurrentRingStream/RecurrentGateStream and 'recurrent' as aliases. If they should exist, the SPEC denial in §13.18.3/§13.18.8.6 must change. Surface, do not resolve.
- **Evidence check:** pass — 030-116 asserts named alias types RecurrentRingStream/RecurrentGateStream exist, but its cited SPEC 13.18.8.6/13.18.3 affirmatively deny any recurrent alias — a recurrent stream is just a Ring/Gate stream with history depth.
- **Charity check:** sustain — Genuine LOG-SPEC divergence AND LOG-internal divergence, no dissolving text exists. LOG 030-116 (DECISION_LOG.md:3622) names alias types 'RecurrentRingStream[T, B, H]' and 'RecurrentGateStream[T, B, H]' as things a recurrent stream produces. SPEC §13.18.8.6 (SPEC:20949-20951) says the same construct produces a plain 'Ring[B]- or Gate[B]-policy stream carrying history depth H' with NO such alias, and SPEC §13.18.3 (SPEC:20468-20471) states affirmatively 'There are no RecurrentRing / RecurrentGate aliases.' LOG 030-260 (DECISION_LOG.md:3766) itself enumerates only RingStream/GateStream and denies recurrent aliases, so 030-116 also contradicts a sibling LOG entry. grep of SPEC.md returns zero occurrences of RecurrentRingStream/RecurrentGateStream. The charitable hunt for a passage defining these aliases as real found none. Behavior-visible: a programmer following 030-116 writes an undefined type name and fails to compile. Sustained as MED divergence.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:3622-3622`
    > 030-116. A `recurrent[N] stream` produces a `RecurrentRingStream[T, B, H]` or `RecurrentGateStream[T, B, H]` — sugar aliases naming a positive history depth on its `ring`/`gate` policy; its special semantics are contained in its own body, and it is usable as a `Stream[T]` downstream, to which operators apply normally: `avg |> map(fn(x): x * 2)`. (§13.18.8.6)
  - `packages/ductus-lang/docs/SPEC.md:20468-20471`
    > `RingStream[T, N]` *is* `Stream[T, Ring[N]]` at every use site; these
    > alias spellings survive as sugar. There are no `RecurrentRing` /
    > `RecurrentGate` aliases — a recurrent stream is spelled with the
    > history-depth axis on a `Ring`/`Gate` policy stream, not with a
    > distinct alias.
  - `packages/ductus-lang/docs/SPEC.md:20949-20951`
    > A `recurrent[N] stream` declaration produces a `Ring[B]`- or
    > `Gate[B]`-policy stream carrying history depth `H` (per its policy and
    > history axes, §13.18.3), usable as an erased `stream T` for downstream
    > consumers.
  - `packages/ductus-lang/docs/DECISION_LOG.md:3622-3622`
    > 030-116. A `recurrent[N] stream` produces a `RecurrentRingStream[T, B, H]` or `RecurrentGateStream[T, B, H]` — sugar aliases naming a positive history depth on its `ring`/`gate` policy; its special semantics are contained in its own body, and it is usable as a `Stream[T]` downstream, to which operators apply normally: `avg |> map(fn(x): x * 2)`. (§13.18.8.6)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3542-3542`
    > 030-36. A stream type is `Stream[T, P]` where `P: StreamPolicy`; `RingStream[T, N]` is a transparent alias for `Stream[T, Ring[N]]` (likewise `GateStream`/recurrent). (§13.18.3)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3766-3766`
    > 030-260. The stdlib provides transparent generic aliases as sugar over the two-axis model for the common stream spellings: `RingStream[T, N]` = `Stream[T, Ring[N]]`, `GateStream[T, N]` = `Stream[T, Gate[N]]`, and the recurrent forms name a positive history depth on the same `Ring`/`Gate` policy. (§13.18.3)
  - `packages/ductus-lang/docs/SPEC.md:20468-20471`
    > alias spellings survive as sugar. There are no `RecurrentRing` /
    > `RecurrentGate` aliases — a recurrent stream is spelled with the
    > history-depth axis on a `Ring`/`Gate` policy stream, not with a
    > distinct alias.
  - `packages/ductus-lang/docs/DECISION_LOG.md:3622-3622`
    > 030-116. A `recurrent[N] stream` produces a `RecurrentRingStream[T, B, H]` or `RecurrentGateStream[T, B, H]` — sugar aliases naming a positive history depth on its `ring`/`gate` policy; its special semantics are contained in its own body, and it is usable as a `Stream[T]` downstream, to which operators apply normally: `avg |> map(fn(x): x * 2)`. (§13.18.8.6)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3542-3542`
    > 030-36. A stream type is `Stream[T, P]` where `P: StreamPolicy`; `RingStream[T, N]` is a transparent alias for `Stream[T, Ring[N]]` (likewise `GateStream`/recurrent). (§13.18.3)
  - `packages/ductus-lang/docs/SPEC.md:20949-20952`
    > A `recurrent[N] stream` declaration produces a `Ring[B]`- or
    > `Gate[B]`-policy stream carrying history depth `H` (per its policy and
    > history axes, §13.18.3), usable as an erased `stream T` for downstream
    > consumers.
  - `packages/ductus-lang/docs/SPEC.md:20467-20471`
    > `RingStream[T, N]` *is* `Stream[T, Ring[N]]` at every use site; these
    > alias spellings survive as sugar. There are no `RecurrentRing` /
    > `RecurrentGate` aliases — a recurrent stream is spelled with the
    > history-depth axis on a `Ring`/`Gate` policy stream, not with a
    > distinct alias.

#### F129 — The sampling moment for a stream flowing into a value-reading operator is unspecified: §13.17 (operator, single-commit) and §13.18.7 (stream expressions, per-event, stream-declaration context) neither covers the derived-returning operator body case.

- **Severity/Category/Verdict:** MED / gap / PLAUSIBLE
- **Anchors:** LOG 029-98, 030-64, 030-82, 030-65 · SPEC § 13.17.11, 13.18.7.1, 13.18.7.3
- **Why it is a defect:** The prober's scenario asks where the sampling moment for each case is specified (13.17 vs 13.18.7). For a stream bound to `smooth(source: cell f32) -> derived f32`: §13.17's model (029-98) says operator internals evaluate within a single commit and read cell params by current value — but a stream has no current value, so this model does not tell you WHEN an event is consumed. §13.18.7.1's per-event model (SPEC 20622) would specify it, but that model governs stream EXPRESSIONS whose surrounding declaration must be `stream`/`recurrent[N] stream` (030-82); `smooth` returns `derived f32`, a value-cell (signal) context, so §13.18.7 does not apply. Neither leaf section specifies the sampling moment for the stream-into-value-reading-operator case, leaving the observable event-consumption cadence undefined.
- **Direction of change:** If the owner decides case (b) is legal, state which section governs its sampling moment and the per-event vs per-commit cadence; if illegal, this gap is subsumed by rejecting the construct.
- **Evidence check:** pass — For a stream bound to a derived-returning value-reading operator, the operator single-commit model (13.17) reads a current value a stream lacks, and the per-event model (13.18.7) governs only stream-typed declarations, so neither pins the event-consumption cadence.
- **Charity check:** refute — F129 presupposes the stream-into-value-reading-operator construct is legal and asks only where its sampling moment is specified. But the same read-site exclusion that dissolves F127 makes the construct a compile error: a stream bound to 'source: cell f32' and read in the operator body ('source * rate') is 'excluded at the read site' (SPEC 20556-20557, 12417-12418, 029-124) and rejected by the general diagnostic 'cannot read a stream T as a value' (SPEC 21746). Since the value-reading operator body cannot legally read the stream, no legal program reaches a point where an event-consumption cadence must be defined — there is no sampling-moment question to specify. The gap is vacuous. The finder's own scenario is specifically the value-reading case (smooth returns derived f32); the erased-stream/§13.18.7 per-event model is correctly noted as not applying, but that non-applicability does not create a gap because the construct is rejected upstream. Dissolving passages consistent with cited 030-82/029-98 (they govern legal stream/value contexts, not the rejected read). | packages/ductus-lang/docs/SPEC.md:20555-20557 — 'A value-reading operator parameter is\nannotated `cell T` (§13.2.8); a `stream T` has no current value, so it\nis excluded at the read site rather than by the annotation.' AND packages/ductus-lang/docs/SPEC.md:21746-21749 — 'error: cannot read a `stream T` as a value ... streams have no current value.'
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:3477-3477`
    > 029-98. Operators do not cross commit boundaries; all internal evaluation happens within a single commit. (§13.17.11)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3588-3588`
    > 030-82. A reactive expression containing one or more streams has stream result kind, and its surrounding declaration must be `stream` or `recurrent[N] stream`. (§13.18.7.3)
  - `packages/ductus-lang/docs/SPEC.md:20622-20637`
    > ##### 13.18.7.1 Expression evaluation model
    > 
    > A reactive input to an express
ion participates uniformly:
    > 
    > - **A stream** contributes its events. The surrounding expression
    >   is re-evaluated once per event, producing one output event per
    >   input event.
    > - **A signal** is *sampled*, not converted: at each event produced by
    >   the expression's driving stream(s), the signal's current value is
    >   read. A signal does not itself produce events — there is no implicit
    >   `to_stream`. To make a signal an event source, convert it explicitly
    >   via its synthesized `.changes` member (§13.18.9).
    > 
    > The expression is recomputed whenever *any* of its reactive inputs
    > emit. The output stream emits the freshly-computed value as its
    > next event.

#### F201 — Entry 030-134 introduces stream 'complete' as a terminal state but no rule in scope defines what completion means for downstream consumers, observation cells, or merge inputs.

- **Severity/Category/Verdict:** LOW / gap / CONFIRMED
- **Anchors:** LOG 030-134 · SPEC § 13.18.9
- **Why it is a defect:** 'complete' is load-bearing (it governs whether the stream can ever emit again) but is used only here; no rule in section 030 defines a stream completion state or its downstream effects — whether a completed input to merge (030-143) ends that arm, whether a consumer cursor behaves differently, whether observation cells (pending_count, is_full) reflect completion. No legal boundary (std-delegation or implementation-defined) is stated. The practical effect ('emits no more') is clear, so severity is limited, but the consumer/observation-cell interaction is unpinned.
- **Direction of change:** Either add a rule pinning stream-completion semantics (its effect on consumers, observation cells, and combining operators) or explicitly mark those interactions implementation-defined. Surface to user for the decision.
- **Evidence check:** pass — 'complete' load-bearing but consumer/merge/observation-cell interaction unpinned in section 030.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:3640-3640`
    > 030-134. `take[T, P: StreamPolicy](source: Stream[T, P], n: i32) -> Stream[T, P]` emits the first `n` events, after which the output stream is complete and emits no more. (§13.18.9)
  - `packages/ductus-lang/docs/SPEC.md:21076-21078`
    > operator take[T, P: StreamPolicy](source: Stream[T, P], n: i32) -> Stream[T, P]:
    >   // emits the first `n` events observed from source, then emits no
    >   // more (the output stream is complete after n events)

### Effects, reconcilers, and hot reload seams

PLACEHOLDER

#### F088 — 031-14/15 admit only signal/stream in `observed:` and forbid all else, while 031-49/146/152 (and the SPEC diagnostic hint's own contradiction) admit derived/value-recurrent computed outputs there — accept and reject the same forms.

- **Severity/Category/Verdict:** HIGH / contradiction / CONFIRMED (x2 independent reports)
- **Anchors:** LOG 031-14, 031-15, 031-49, 031-152, 031-146 · SPEC § 13.19.2, 13.19.5
- **Why it is a defect:** 031-14 enumerates the legal observed-block role keywords as exactly `signal`/`stream`; 031-15 says nothing outside 'the five role-keyword cell forms' (derived/recurrent/stream in desired + signal/stream in observed) may appear in an effect body — jointly forbidding `derived` and value-`recurrent` in `observed:`. But 031-49, 031-146, and 031-152 explicitly admit them as computed outputs. SPEC repeats the contradiction verbatim: 13.19.2 says 'No other declaration kinds are permitted' and '`recurrent` is allowed only in `desired:`', while 13.19.5 says '`observed:` block may also declare `derived` cells and value-`recurrent` cells'. A parser/type-checker implementer must decide whether `derived x = …` or `recurrent x = …` inside `observed:` parses or is a diagnostic; the two halves demand opposite answers. This is a load-bearing v1 feature (interior effects lift child outputs into `observed:` with no host write), so it cannot be resolved by picking the restrictive half without deleting the interior-effect surface.
- **Direction of change:** Reconcile the observed-block role-keyword enumeration: either 031-14/031-15 and SPEC 13.19.2 must widen to admit `derived` and value-`recurrent` as computed outputs in `observed:`, or the computed-output feature (031-49/031-146/031-152, SPEC 13.19.5) must be withdrawn. Direction only; the user decides which side is authoritative.
- **Evidence check:** pass — 031-14/15 (and SPEC 13.19.2 diagnostic hint) forbid any declaration in observed: other than signal/stream and state recurrent is allowed only in desired:, while 031-49/146/152 and SPEC 13.19.5 admit derived and value-recurrent as computed outputs in observed:; a parser/type-checker cannot both reject and accept 'recurrent x = ...' inside observed:.
- **Charity check:** sustain — Live jointly-unsatisfiable contradiction confirmed on fresh read. 031-14/15 (and SPEC 13.19.2 lines 21937-21938: 'No other declaration kinds are permitted inside the blocks. recurrent is allowed only in desired:') forbid derived and value-recurrent in observed:. 031-49/146/152 (and SPEC 13.19.5 lines 22244-22246: 'An observed: block may also declare derived cells and value-recurrent cells') admit exactly those. SPEC contradicts itself section-to-section; LOG contradicts itself entry-to-entry. A parser/type-checker implementer must decide whether 'derived x =' / 'recurrent x =' inside observed: parses or is a diagnostic — the two halves demand opposite answers, and the interior-effect surface (lifted computed outputs, no host write) is load-bearing so the restrictive half cannot simply win. No carve-out in 13.19.2 for the computed-output lift. Sustained HIGH.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:3783-3784`
    > 031-14. Every cell declaration in an effect block carries an explicit role keyword: `derived`, `recurrent`, or `stream` in `desired:`, `signal` or `stream` in `observed:`. (§13.19.2)
    > 031-15. Reactive declarations other than the five role-keyword cell forms (e.g. `attr`, top-level `signal`) cannot appear inside an effect's body. (§13.19.2)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3921-3921`
    > 031-152. A value `recurrent` is allowed in both an effect's `desired:` and `observed:` blocks; a `recurrent[N] stream` is allowed in `desired:` but forbidden in `observed:`. (§13.19.4)
  - `packages/ductus-lang/docs/SPEC.md:21934-21938`
    > - `signal` (in `observed:`) — host-written value cell (§13.19.5).
    > - `stream` (in `observed:`) — host-written event sequence (§13.19.5).
    > 
    > No other declaration kinds are permitted inside the blocks. `recurrent`
    > is allowed only in `desired:` (a host-fed `observed:` cell has no
  - `packages/ductus-lang/docs/SPEC.md:22244-22246`
    > **Computed outputs (`derived` / `recurrent`).** An `observed:` block
    > may also declare `derived` cells and value-`recurrent` cells that the
    > effect computes itself — from its own `desired:` cells, its parameters,
  - `packages/ductus-lang/docs/DECISION_LOG.md:3818-3818`
    > 031-49. A value `recurrent` (and a `derived`) is valid in `observed:` blocks as a computed output; a `recurrent[N] stream` is not valid in `observed:` blocks. (§13.19.5)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3915-3915`
    > 031-146. A normative diagnostic class covers a non-role-keyword declaration inside an effect's blocks, and `recurrent[N] stream` in `observed:`; a value `recurrent` and a `derived` are allowed in `observed:` as computed outputs, and `recurrent` is allowed in `desired:`. (§13.19.16)
  - `packages/ductus-lang/docs/SPEC.md:22870-22874`
    >   hint: effect blocks accept only `derived`, `recurrent`, `stream` (in
    >         `desired:`) and `signal`, `stream` (in `observed:`). `recurrent`
    >         in `observed:` is rejected (host-fed cells have no expression
    >         body); for state that wraps multiple effects, use a wrapping
    >         operator (§13.17).

#### F218 — Repeat-scope cells are declared to follow 13.15.2's identity rules, yet 13.15.2/15.4.1.1 define identity only by lexical name plus positional ordinal :N and never define the .<key>. path segment 018 depends on.

- **Severity/Category/Verdict:** HIGH / contradiction / CONFIRMED
- **Anchors:** LOG 018-25, 018-26, 028-4, 028-5 · SPEC § 13.5.3, 13.15.2, 15.4.1.1
- **Why it is a defect:** 018-25 routes repeat per-key state-cell identity through 13.15.2, and 13.5.3 says the `<enclosing>.<key>.<template_field>` path `follows §15.4.1.1`. But 13.15.2/15.4.1.1 (028-4, 028-5) is normatively closed: identity = lexical declared names, and the ONLY multiplicity mechanism is the positional ordinal `:N` (declaration-order index among same-type siblings). Neither 13.15.2 nor 15.4.1.1 defines a `.<key>.` path segment, and both assert `the two are the same mechanism`. An implementer applying 15.4.1.1 verbatim would materialize repeat siblings with `:N` ordinal identity, not key identity. The `.<key>.` component 018 requires has no basis in the section it cites. Two careful readings yield different concrete on-wire cell IDs for the same cells; that is a soundness hole in the identity contract, not a wording nit.
- **Direction of change:** Either 15.4.1.1/13.15.2 must be extended to define a keyed path segment as a first-class multiplicity mechanism alongside `:N` (and state which applies to repeat-materialized siblings), or 018 must stop claiming its keyed path `follows §15.4.1.1`. Decide and make one section the authority; do not leave both claiming to be `the same mechanism`.
- **Evidence check:** pass — 018-25/26 route per-key repeat cell identity through §13.15.2/§15.4.1.1, but those sections (028-4/5) are normatively closed — identity = lexical names + positional `:N` ordinal, 'the two are the same mechanism' — and never define the `.<key>.` path segment. An implementer applying §15.4.1.1 verbatim produces `:N`-keyed, not key-keyed, cell IDs. Real identity-contract hole.
- **Charity check:** sustain — 018-25/018-26 and §13.5.3 (SPEC:15206-15221) / §13.5.4.4 (SPEC:15518-15525) assert the repeat scope-cell path <enclosing>.<key>.<cell> and route it 'per §13.15.2's cell-identity rules' and 'follows §15.4.1.1'. But §13.15.2 (SPEC:19289-19300) and §15.4.1.1 (SPEC:24213-24232) — the delegated-to sections — define ONLY a lexical declaration path plus a `:N` positional-ordinal for anonymous/duplicated siblings; neither defines, mentions, or wire-encodes a `.<key>.` segment or acknowledges keyed multiplicity, and §15.4.1.1 declares itself the complete wire format ('§15.4.1.1 specifies the wire format', 'the two are the same mechanism'). So §13.5.3 needs a wire device §15.4.1.1 does not provide — a real divergence in the citation contract. Note: the finding tags HIGH (jointly-unsatisfiable); I sustain the divergence but it is not strictly unsatisfiable — §13.5.3 self-supplies 'the key value serves as the path component' (SPEC:15220) and §15.4.1.1's `:N` is textually gated on 'anonymous or duplicated', which keyed scopes are not, so an implementer can reconcile by specialization. That makes it a MED divergence/gap (§15.4.1.1 fails to specify how `.<key>.` is wire-encoded or that keyed scopes are a multiplicity source), not a HIGH soundness hole. Sustains at MED; severity re-grade surfaced, not decided.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2495-2496`
    > 018-25. When state-shape is non-empty, per-key state cells participate in hot reload per §13.15.2's cell-identity rules. (§13.5.3)
    > 018-26. A scope cell's path is `<enclosing_path>.<key>.<template_field>`. (§13.5.3)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3305-3306`
    > 028-4. Reactive cells are identified across reloads by fully-qualified declaration path — module path, instance name, cell name: `audio.synth_a.osc_1.frequency`. Interpreter-placed effect instances are identified the same way, with their paths rooted at the interpretation site and mirroring the node-path scheme. (§13.15.2)
    > 028-5. Anonymous or duplicated sibling placements get an ordinal suffix `:N` — the zero-based declaration-order index among same-type siblings at the same nesting depth; the `:N` ordinal rule applies equally to interpreter-placed effects. (§13.15.2)
  - `packages/ductus-lang/docs/SPEC.md:24215-24232`
    > A cell ID is the cell's fully-qualified declaration path: a
    > dot-separated sequence of identifiers naming the lexical nesting
    > from module root through enclosing instances to the cell name.
    > 
    > Example: `audio.synth_a.osc_1.frequency` — module `audio`,
    > top-level instance `synth_a`, nested child `osc_1`, attr
    > `frequency`.
    > 
    > The path is derived deterministically from source: nesting plus
    > declared instance/cell names. The syntax is identical to that of
    > §13.15.2 (cell identity across reloads); the two are the same
    > mechanism: §13.15.2 specifies hot-reload identity in source-level
    > terms, §15.4.1.1 specifies the wire format.
    > 
    > For anonymous or duplicated sibling placements (rare; the language
    > encourages explicit naming per §13.8), the compiler appends an
    > ordinal suffix `:N` where N is the declaration-order index among
    > siblings of the same type at the same nesting depth (zero-based).
  - `packages/ductus-lang/docs/SPEC.md:15209-15214`
    > hot reload per §13.15.2's cell-identity rules. The cell path
    > follows §15.4.1.1:
    > 
    > ```
    > <enclosing_path>.<key>.<template_field>
    > ```

### Commit engine: gate ordering and scheduling

The gate-open snap has no legal ordering inside the commit's own rules; startup and self-conditional gate evaluation conflict; two deterministic tiebreakers can disagree; scope_drop is never placed in the commit cycle; the wake-gate construct exists only in the LOG.

#### F064 — Gate-open snap requires same-commit topological re-evaluation of frozen deriveds, but the per-commit DAG omits them and no new dirty bits may be added after step 1 — the implementer has no legal ordering to run the snap.

- **Severity/Category/Verdict:** HIGH / contradiction / CONFIRMED
- **Anchors:** LOG 022-58, 022-59, 023-12, 023-13, 023-14, 022-74 · SPEC § 13.9.7, 13.9.8, 13.10.2
- **Why it is a defect:** The `when` predicate is itself a derived (022-12); its new value (true) is only known after it is evaluated in step 3 (topological order). A gate that flips because its OWN provenance changed does not imply the frozen subtree's downstream output-affecting deriveds were dirtied this commit — during the gap their input writes were suppressed (022-74: gated edges contribute no dirty propagation to destination output-affecting cells), so they carry no dirty bit at this commit unless their inputs also happened to change now. Therefore step 1 does not mark them dirty, step 2's DAG (023-14) does not contain them, and 023-12 forbids adding them mid-commit. Yet 022-58/022-59 mandate they re-evaluate in topological order within THIS commit. An implementer building the commit machine has no rule authorizing a mid-commit expansion of the topologically-sorted DAG to include the newly-unfrozen subtree, nor is any legal boundary (001-6: std-delegation or implementation-defined) declared for the snap's scheduling. The two-track error/eval model gives no escape hatch here.
- **Direction of change:** Add a rule making the false→true gate flip inject the gated subtree's output-affecting deriveds into the current commit's evaluation set and specifying their position in the topological order relative to the flipping predicate and to recurrent advancement (step 4); or relax 023-12/023-14 to admit gate-transition-induced re-evaluation nodes; or move the snap to the next commit and reconcile 022-59. Decision belongs to the user.
- **Evidence check:** pass — Gate-open snap (022-58/59) requires same-commit topological re-evaluation of frozen deriveds; but 022-74 keeps gated edges from dirtying those output-affecting cells, so 023-12/14 exclude them from the DAG and forbid adding dirty bits mid-commit. No legal ordering exists to run the snap.
- **Charity check:** sustain — 022-58/59 (SPEC §13.9.7, 17718-17724) mandate that on a false→true flip the frozen deriveds re-evaluate in topological order within the SAME commit. But §13.9.8 (17826-17827) explicitly EXCLUDES a gated-off subtree from the per-commit DAG ('A subtree that is gated off is excluded from the DAG'); 022-74 gives its edges no dirty propagation to output-affecting cells; 023-12 (18138-18139) forbids adding dirty bits after step 1; §13.11.3 (18389-18390) fixes the DAG at step 2. No section supplies the mechanism to inject the newly-unfrozen deriveds into the already-sorted DAG within this commit — the 'delegating note' (17830) is circular (§13.10.2→§13.9.7→§13.9.8→§13.10.2), and no legal boundary (001-6: std-delegation / implementation-defined) is declared for the snap scheduling (§13.9.11's implementation-defined clause covers only diagnostic wording). An implementer cannot satisfy both the same-commit snap mandate and the DAG-freeze rules. HIGH implementer-blocking gap sustains.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2942-2943`
    > 022-58. A false-to-true predicate flip is a propagation event: the frozen deriveds re-evaluate against current upstream state in topological order. (§13.9.7)
    > 022-59. The gate-open re-evaluation snap is scheduled within the **same commit** that flips the predicate false→true — not the next commit; this contrasts with a reload predicate (next commit) and a connection re-point (next commit). (§13.9.7)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3020-3022`
    > 023-12. After dirty-set computation, no new dirty bits are added during the rest of the commit cycle. (§13.10.2)
    > 023-13. Commit step 2 computes evaluation order by topologically sorting the per-commit DAG. (§13.10.2)
    > 023-14. The per-commit DAG's nodes are the dirty derived expressions plus each recurrent whose expression became dirty this commit. (§13.10.2)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2958-2958`
    > 022-74. In per-commit DAG construction, gated edges contribute no dirty propagation to destination output-affecting cells but do contribute to input cells and `when`-predicate provenance. (§13.9.8)
  - `packages/ductus-lang/docs/SPEC.md:18138-18148`
    > commit. No new dirty bits are added during the rest of the
    >    commit cycle.
    > 2. **Compute evaluation order.** Topologically sort the per-commit
    >    DAG (§13.11.3). Nodes in the DAG are:
    >     - Dirty derived expressions.
    >     - Each recurrent whose expression became dirty this commit.

#### F063 — LOG 025-6 (→SPEC §13.12.1) says provenance has TWO dynamic-source exceptions (WeakHandle and Portal), but SPEC §13.12.1 states 'The one exception is a read through WeakHandle[T] resolution' — Portal omitted.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 025-6, 023-46 · SPEC § 13.12.1, 13.10.5
- **Why it is a defect:** LOG 025-6 must conform to its referenced SPEC §13.12.1, but they disagree on the count of dynamic-dependency exceptions (two vs one). SPEC is also internally inconsistent: §13.10.5 says 'two sources' (matching LOG 023-46), §13.12.1 says 'the one exception'. An implementer building provenance tracking from §13.12.1 would not treat Portal re-binds as a provenance-identity change, silently under-tracking Portal dynamic dependencies.
- **Direction of change:** Make SPEC §13.12.1 list both WeakHandle and Portal as provenance exceptions to match LOG 025-6 and SPEC §13.10.5, or, if §13.12.1's single-exception scoping is intended, amend LOG 025-6 to match — surface which is authoritative.
- **Evidence check:** pass — LOG 025-6 must conform to SPEC 13.12.1. LOG says two dynamic-source exceptions (WeakHandle+Portal); SPEC 18501 says exactly one (WeakHandle). Verified verbatim on both sides; genuine LOG-SPEC divergence and SPEC self-inconsistency (18295 'two' vs 18501 'one').
- **Charity check:** sustain — LOG 025-6 (DECISION_LOG.md:3103) states 'the two exceptions are the dynamic-dependency sources — a read through WeakHandle[T] resolution and a read through Portal[T] resolution'. Its referenced SPEC section §13.12.1 (SPEC.md:18501-18502) states 'The one exception is a read through WeakHandle[T] resolution' — Portal is omitted entirely from the exception count. This is a substantive LOG->SPEC divergence (025-6 must conform to §13.12.1 per the edit protocol; they disagree on the count and on whether Portal is an exception). It is compounded by an internal SPEC divergence: §13.10.5 (SPEC.md:18295-18297) says WeakHandle and Portal 'are the two sources of dynamic dependency', contradicting §13.12.1's 'one exception'. No passage in scope reconciles one-vs-two. An implementer building provenance tracking from §13.12.1 would not treat Portal re-binds as a provenance-identity change, under-tracking Portal dynamic dependencies. SUSTAIN.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:3103-3103`
    > 025-6. A provenance set is normally static; the two exceptions are the dynamic-dependency sources — a read through `WeakHandle[T]` resolution and a read through `Portal[T]` resolution — where the identity of the depended-on cells changes on re-point, mount, dismount, or slot rebind. A statically-placed `Handle[T]` reads as `&T` directly and contributes its referent's cells to the provenance like any static reference. (§13.12.1)
  - `packages/ductus-lang/docs/SPEC.md:18501-18510`
    > A provenance set is normally *static* — fixed for the expression. The one
    > exception is a read through **`WeakHandle[T]` resolution** (§13.3.6.2,
    > §13.10.5): the *identity* of the depended-on cells changes when the
    > resolution re-points, mounts, or dismounts. A statically-placed
    > `Handle[T]` carries no such exception — its referent is fixed, so its
    > read contributes its referent's cells to the provenance like any static
    > reference. The canonical dynamic case is a connection body reading `to.*`
    > against a reactive destination (§13.6.2); a node-body read through a
    > `repeat`-view handle (§13.5.4.9) is the same mechanism, and an operator
    > over a dynamic namespace cell (§13.3.3.4) is its collective form.
  - `packages/ductus-lang/docs/SPEC.md:18295-18298`
    > `WeakHandle[T]` resolution (for graph entities) and `Portal[T]`
    > resolution (for non-graph slots) — individually or in this collective
    > form — are the **two** sources of dynamic dependency in the language

#### F230 — 'wake gate'/'compiler-synthesized wake gate' is a first-class construct in the LOG but the term appears zero times anywhere in SPEC.md, including its own referenced sections.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 017-269, 017-272, 022-37, 033-65 · SPEC § 13.3.8, 13.9.6, 15.4.1
- **Why it is a defect:** The LOG defines wake gate as a load-bearing lowering construct (a compiler-synthesized gate on every wire-following target render, with a specific predicate and prior-commit read rule) carried by four decisions referencing SPEC 13.3.8, 13.9.6, and 15.4.1. A full-text grep of SPEC.md for 'wake gate'/'wake-gate' returns zero hits; none of the referenced SPEC sections surface the mechanism under this term. SPEC must conform to LOG; a named construct present in LOG and absent from its own referenced SPEC elaboration is a substantive LOG-SPEC divergence, not mere style.
- **Direction of change:** Bring the referenced SPEC sections into conformance with the LOG by elaborating the wake-gate construct under a stable term, or surface to the user whether 'wake gate' should be the fixed term.
- **Evidence check:** pass — 'wake gate'/'compiler-synthesized wake gate' is a first-class lowering construct carried by 5 LOG decisions (017-269, 017-272, 022-37, 024-26, 033-65) referencing SPEC §13.3.8/§13.9.6/§15.4.1, yet the term appears zero times anywhere in SPEC.md including those referenced sections.
- **Charity check:** sustain — Confirmed. 'wake gate'/'wake-gate' appears in LOG four times as a load-bearing construct: 017-269 (2410), 017-272 (2413), 022-37 (2921), 033-65 (4177), plus 024-26 (3093). A full grep of SPEC.md for 'wake gate'/'wake-gate' returns ZERO hits. I hunted for the mechanism under other terms (synthesized gate, 'wire currently resolves', 'mounted yet inactive', wire-following target render) — SPEC has 'wire-following' (16059) but never surfaces the compiler-synthesized-per-target-render wake gate under any name. I read the referenced SPEC sections directly: §13.3.8 (14832), §13.9.6 self-conditional gates (17625-17653), and §15.4.1 gate objects (24104-24122). §15.4.1 elaborates only `when`-gates and block-selector arms; §13.9.6 elaborates cyclic self-conditional gates by committed values but never the wake gate. LOG 033-65 explicitly says wake gates 'lower to these same gate objects' citing §15.4.1, yet §15.4.1 never mentions them. A named LOG construct absent from its own referenced SPEC elaboration is a substantive LOG-SPEC divergence, not style. | none found; SPEC.md:24104-24122 (§15.4.1 gate objects, the section LOG 033-65 cites for wake-gate lowering) enumerates only '**when**-gates' and 'Block selectors ... lower to one gate per arm' — the wake gate is never named or described there.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2413-2413`
    > 017-272. Wires activate, never instantiate: each target render carries a compiler-synthesized wake gate whose predicate is "some live wire currently resolves to me OR my containment parent is active," lowering to existing gate objects. The wake gate's reads of incoming connection-views are compiler-internal. (§13.3.8)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4177-4177`
    > Compiler-synthesized wake gates (one per wire-following target render, whose predicate is that some live wire currently resolves to it or its containment parent is active) and the per-arm `gate_parent` guarding of structural output produced while walking a `Gated` entry's arm both lower to these same gate objects — no new IR construct is introduced for either. (§15.4.1)
  - `packages/ductus-lang/docs/SPEC.md:14832-14840`
    > #### 13.3.8 Effects (the `effects:` clause)
    > 
    > The `effects:` clause is the node's **side-effect zone**: the place
    > where the node declares the effects (§13.19) it runs against the
    > outside world. It is the **sole** site at which an effect may be
    > instantiated — module level, operator bodies, connection bodies, and
    > function bodies are all illegal hosts (§13.19.15). Reading a node's
    > `effects:` clause therefore tells you exactly what that node does to
    > the outside world; there is nowhere else to look.

#### F168 — Two same-level DAG nodes in different node instances match BOTH tiebreaker rules — 023-33 (source declaration order) and 023-34 (placement order) — which can disagree, yielding two different deterministic evaluation traces.

- **Severity/Category/Verdict:** MED / ambiguity / CONFIRMED
- **Anchors:** LOG 023-33, 023-34 · SPEC § 13.10.3
- **Why it is a defect:** 023-33 is stated unconditionally over 'two same-level DAG nodes' with tiebreaker = source declaration order. 023-34 covers 'same-level cells across different node instances' with tiebreaker = placement order. A pair of same-level cells in two different node instances satisfies both predicates. Source declaration order and construction-time placement order can differ (a node type declared earlier in source may be placed later). Neither entry scopes 023-33 to same-instance, so Reading A applies 023-33 globally (source order) and Reading B applies 023-34 for the cross-instance case (placement order); the two produce different evaluation traces. The spec itself flags the trace as observable ('same program, same inputs, same output trace'), so the divergence is behavior-observable, not cosmetic. The SPEC prose does not state which rule takes precedence when both apply.
- **Direction of change:** Surface to user: 023-33 and 023-34 need an explicit precedence/scoping (e.g. 023-33 scoped to intra-instance, 023-34 governing cross-instance) so exactly one tiebreaker applies to any pair. Do not pick the resolution unilaterally.
- **Evidence check:** pass — 023-33 (source declaration order, unconditional over 'two same-level DAG nodes') and 023-34 (placement order, cross-instance) both match a cross-instance same-level pair; the orders can differ, yielding two different deterministic evaluation traces. Neither entry nor SPEC states which rule wins.
- **Charity check:** sustain — 023-33 (SPEC §13.10.3, 18201-18206) states the tiebreaker over 'two same-level DAG nodes' unconditionally as source-declaration order; 023-34 (18208-18209) states it for 'cells across different node instances' as construction-time placement order. A same-level pair in two different node instances matches BOTH predicates literally, and source-declaration order and placement order CAN differ (a type declared earlier may be placed later). Neither entry scopes 023-33 to same-instance, and grep finds NO precedence statement ('takes precedence/overrides/more specific') in §13.10.3 or the LOG. The specific-over-general resolution is an inference the text does not force — per the ambiguity standard, inference-required-ness is the finding. The spec's own goal for the tiebreaker is 'same program, same inputs, same output trace' (18206), so the unpinned precedence lets two conformant compilers produce different observable traces, defeating that guarantee. MED ambiguity sustains.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:3041-3042`
    > 023-33. When two same-level DAG nodes have no dependency between them, the tiebreaker is deterministic source declaration order: the cell declared earlier evaluates first. (§13.10.3)
    > 023-34. For same-level cells across different node instances, the tiebreaker is placement order at construction time. (§13.10.3)
  - `packages/ductus-lang/docs/SPEC.md:18201-18209`
    > When two nodes are at the same level (neither depends on the
    > other), the compiler chooses a deterministic tiebreaker:
    > **source declaration order**. The cell declared earlier in source
    > evaluates first. Since the two are not dependency-related, the
    > choice does not affect correctness — but determinism matters for
    > reproducibility (same program, same inputs, same output trace).
    > 
    > For cells across different node instances at the same level, the
    > placement order at construction time is the tiebreaker.

#### F116 — The cell-value invariant guarantees defined connection-derived reads only via `from`; an endpoint-derived reading `to` on a connection frozen from construction (empty repeat key set) has no defined initial value.

- **Severity/Category/Verdict:** MED / gap / CONFIRMED
- **Anchors:** LOG 019-59, 019-60, 019-53 · SPEC § 13.9.7, 13.6.2
- **Why it is a defect:** Scenario: a self-sourced connection whose `to` resolves through a repeat-view lookup (`view.get(k)?...`) that is `None` because the repeat key set is empty at construction. Per 019-53 the connection freezes and its body never runs, so the endpoint-derived `target = handle to` never commits. 019-60 says an outside observer reads the derived's 'last committed value', but here there is none. SPEC 13.9.7's cell-value invariant justifies defined connection-derived reads ONLY via `from` (line 17782-17783: 'compute against `from` which always has defined cells'), so a `to`-reading derived is outside the enumerated guarantee. Line 17786-17787 promises 'the initial value if the instance has never been active', but no rule defines the initial value of a derived whose sole input `to` is unbound. An implementer cannot compute `handle to` when `to` is unbound and has no rule for what `target` reads instead. The construct is legal (019-59 sanctions the endpoint-derived; empty repeat sources are legal) but its behavior at frozen-from-construction is unspecified.
- **Direction of change:** Specify the initial/frozen value of an endpoint-derived (a connection derived reading `to`/`pair`) when the connection is frozen from construction and has never bound `to`; extend the 13.9.7 cell-value invariant to cover `to`-dependent connection deriveds, or forbid observing such a derived before first activation.
- **Evidence check:** pass — A legal self-sourced connection whose 'to' resolves via an empty repeat-view lookup freezes from construction, so 'derived target = handle to' never commits; the cell-value invariant guarantees defined reads only via 'from', and no rule defines the initial value of a to-reading derived whose sole input is unbound — behavior unspecified.
- **Charity check:** sustain — The §13.9.7 cell-value invariant (SPEC 17781-17783) justifies defined connection-derived reads by 'All connection-level deriveds compute against from which always has defined cells' — from ONLY. The endpoint-derived carve-out (019-59) declares 'derived target: WeakHandle[Clip] = handle to', reading to, an endpoint outside that enumerated guarantee. In the witness scenario, a WeakHandle destination sourced from a repeat-view lookup that is None at construction freezes the connection from construction and 'the body does not run at all' (019-53; SPEC 16032), so target never commits and has no last-committed value. Line 17786-17787 falls back to 'the initial value if the instance has never been active,' but no rule defines that initial value: 016-145's 'just-computed initial values' covers when-gated instances whose bodies DO compute once, not a freeze-from-unresolved-destination where the body never runs. Searched startup/init rules and endpoint-derived text; found no covering normative text and no legal boundary (001-6). Genuine gap: handle to is uncomputable when to is unbound and the initial value is unspecified.
- **Evidence:**
  - `packages/ductus-lang/docs/SPEC.md:17781-17787`
    > - All deriveds compute against always-defined inputs.
    > - All connection-level deriveds compute against `from` which
    >   always has defined cells.
    > 
    > On a gated node or connection, reads return frozen values: the
    > last committed value during an active period, or the initial value
    > if the instance has never been active.
  - `packages/ductus-lang/docs/SPEC.md:16047-16055`
    > **Endpoint-derived carve-out (opt-in).** A connection MAY *opt in* to
    > surfacing endpoint data by declaring its **own derived** whose value is a
    > storable handle to an endpoint:
    > 
    > ```
    > connection Plays:
    >   from: Verse
    >   to: Clip
    >   derived target: WeakHandle[Clip] = handle to      // opt-in endpoint surface
  - `packages/ductus-lang/docs/DECISION_LOG.md:2669-2669`
    > 019-53. While a `WeakHandle[N]` destination resolves to `None`, the connection freezes and its body does not run at all. A statically-placed `Handle[N]` destination never freezes. (§13.6.2)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2676-2676`
    > 019-60. An outside observer reads a connection's own cells (attrs, deriveds), which hold their last committed value even while the connection is frozen. (§13.6.2)

#### F112 — No section places scope_drop (or the stream/effect teardown it triggers) within the commit cycle; 023's six commit steps never mention scope add/drop, and 018 never anchors scope_drop to a commit phase, leaving retire-vs-teardown-vs-suspend ordering unspecified.

- **Severity/Category/Verdict:** MED / gap / CONFIRMED
- **Anchors:** LOG 023-13, 023-20, 023-29, 023-30, 018-78, 018-94 · SPEC § 13.10.2, 13.5.4.2, 13.5.4.6
- **Why it is a defect:** The scenario asks 'in what commit order' the instance retires, the gate reports inactive, and pending emissions resolve. 023 enumerates a fixed six-step commit (steps 1-6, 023-8..023-31) but none of the steps is where scope_obtain/scope_drop occurs — the phrase scope_drop never appears in section 023, and section 018 (checked: no occurrence of 'commit' step-anchoring in 018-1..018-143) says only that repeat recomputes at commit time (018-94) without stating at which of the six steps the diff-and-drop runs relative to derived evaluation (step 3), recurrent advancement (step 4), or snapshot publish (step 5). Consequence: an implementer cannot determine whether a dropped scope's last scope_evaluate contributes to this commit's snapshot, nor the ordering of scope_drop against the gate-close suspend hooks (022-63, fired 'at the commit boundary') for effects that are simultaneously retired. Two implementers can order these differently and produce different committed snapshots.
- **Direction of change:** Anchor repeat's iterate/diff/scope_obtain/scope_drop/scope_evaluate sequence to specific numbered steps of the §13.10.2 commit cycle, and state its ordering relative to the gate-close suspend hook (022-63) and stream teardown, in either 018 or 023 (and designate which section owns it).
- **Evidence check:** pass — scope_drop (and the stream/effect teardown it triggers) is never anchored to a commit step: 023's six steps never mention scope add/drop, and 018 never anchors scope_drop to a commit phase. Retire-vs-teardown-vs-suspend ordering relative to derived eval / recurrent advance / snapshot publish is unspecified.
- **Charity check:** sustain — The `repeat` scope diff procedure (§13.5.4.2 / SPEC 15400-15409) issues scope_drop/scope_obtain/scope_evaluate, but 018-32/74/94 anchor it only to 'per commit' / 'commit-time-recompute', never to one of the six numbered commit steps (023-8..023-31). Grep confirms the six-step commit cycle (§13.10.2, 18122-18189) never mentions any scope_* operation, and no text places the scope diff at a numbered step ('scope_*.*step' returns nothing). Consequence: an implementer cannot tell whether a dropped scope's last scope_evaluate lands in this commit's published snapshot (step 5) or the next, nor the order of scope_drop against the gate-close `suspend` hook (022-63, 'at the commit boundary') for an effect that is simultaneously retiring and gate-closing. 018-47 ('scope_evaluate runs against the current committed snapshot, read-only') deepens rather than dissolves the ambiguity. Two conformant implementations can produce different committed snapshots. MED gap sustains.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:3021-3022`
    > 023-13. Commit step 2 computes evaluation order by topologically sorting the per-commit DAG. (§13.10.2)
    > 023-14. The per-commit DAG's nodes are the dirty derived expressions plus each recurrent whose expression became dirty this commit. (§13.10.2)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3037-3039`
    > 023-29. Commit step 4 writes the values computed during evaluation into the recurrent cells; after this step, recurrent reads return the newly-advanced values. (§13.10.2)
    > 023-30. Commit step 5 atomically publishes the just-committed values as the visible snapshot, so observers' subsequent reads return the new state. (§13.10.2)
    > 023-31. Commit step 6 clears the dirty bits. (§13.10.2)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2564-2564`
    > 018-94. `repeat` follows the commit-time-recompute model: a dirty source is re-iterated and re-keyed in full. (§13.5.4.6)

#### F084 — Whether a gated-off static placement feeding a dynamic view is a frozen-present member or absent is unspecified: the four gate read-paths cover static-view and fold membership but omit dynamic-view membership.

- **Severity/Category/Verdict:** MED / gap / CONFIRMED
- **Anchors:** LOG 017-80, 017-105, 017-198 · SPEC § 13.3.3.4, 13.9.7
- **Why it is a defect:** 017-80 permits a static placement (which 021-59 can gate with placement-level `when`) to feed a `dynamic` view. Gating freezes but never removes existence (017-105). The SPEC's exhaustive four-path enumeration of how a gate is observed names ONLY static-view membership (path 3, frozen-present) and fold membership (path 4, activation-absent). It does not state which applies to a gated-off static feeder of a DYNAMIC view. Two careful readings diverge: (a) existence-based, so it stays a member and `posts.count` counts it and `repeat p in posts` mounts a scope for it; (b) activation-based, so it drops from membership. This changes concrete program behavior (`.count`, repeat scope set).
- **Direction of change:** Add a rule fixing dynamic-view membership under gating: state whether a gated-off static feeder remains a frozen-present member of the dynamic view (parallel to static views) or is absent (parallel to folds), and how it affects `.count` and `repeat`.
- **Evidence check:** pass — 017-80 permits a static placement (gateable via placement-level `when`) to feed a `dynamic` view, but the SPEC's exhaustive four gate read-paths cover only static-view membership (frozen-present) and fold membership (activation-absent), leaving a gated-off static feeder of a dynamic view unspecified between frozen-present and absent — changing concrete `.count` and repeat-scope behavior.
- **Charity check:** sustain — Confirmed gap. 017-80 (LOG 2221) permits an explicitly-written (static) placement to feed a `dynamic` view; placement-level `when` can gate it. 017-105 (2246) says gating 'freeze[s] propagation rather than remove existence — distinct from dynamic/repeat, which change membership', so a gated static placement is a frozen-present member. But a dynamic view is precisely the membership-changing kind. The SPEC's exhaustive four-read-path enumeration (17789-17816) names ONLY static-view membership (path 3: frozen-present) and fold/collect membership (path 4: activation-driven absent); it is silent on dynamic-view membership of a gated-off static feeder. I read the dynamic-view access rules (SPEC 13614-13622: '.count ... tracking membership like any other read') and 13600-13612 (placement-site key) and the collect/yield permanent-member rule (22979-22983) — none states whether the gated static feeder counts. Two readings both survive: (a) 017-105 governs -> frozen-present, `.count` counts it, `repeat` mounts a scope; (b) activation-driven like path 4 -> absent, dropped from `.count` and repeat. This changes concrete program behavior. No explicit covering text and no 001-6 legal boundary; inference is required, which is a sustain under the gap standard. | none found; SPEC.md:17800-17816 enumerates exactly four gate read-paths and the only membership paths are path 3 'Frozen static-view membership' and path 4 'Activation-driven fold membership' — neither covers a gated-off static placement feeding a dynamic view. LOG 017-105 (frozen existence) and the dynamic-view membership-changing nature pull in opposite directions with no reconciling rule.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2246-2246`
    > 017-105. Views count gated-but-frozen children and reads return their frozen values — gates freeze propagation rather than remove existence — distinct from `dynamic`/`repeat`, which change membership. (§13.3.3)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2221-2221`
    > 017-80. An explicitly written (static) placement feeding a `dynamic` view carries a stable, path-derived placement-site key. (§13.3.3.4)
  - `packages/ductus-lang/docs/SPEC.md:17800-17816`
    > 3. **Frozen static-view membership.** A static view's members include
    >    gated-off elements as **frozen** members: they remain present in the
    >    view's membership (membership is static), and a read of a frozen
    >    member returns its frozen value. Gating changes the value-state, not
    >    the membership.
    > 4. **Activation-driven fold membership (new).** When a value
    >    contribution flows into a `fold` or `collect` group from inside a
    >    gated arm, its membership is **activation-driven**: the member is
    >    present in the group iff its arm is *effectively active*

#### F065 — A self-conditional gate reading a same-instance recurrent has two conflicting evaluation rules at startup: 022-37 says read prior-commit values, but startup has no prior commit and 016-146 evaluates the gate same-pass in topological order.

- **Severity/Category/Verdict:** MED / ambiguity / CONFIRMED
- **Anchors:** LOG 022-37, 016-144, 016-146, 016-139 · SPEC § 13.9.6, 13.2.6
- **Why it is a defect:** Consider `when: counter > 5` where `counter` is a same-instance recurrent. Per 022-37 the self-referential gate reads `counter`'s PRIOR-commit value (one-commit delay). At startup there is no prior commit (016-139/12292 only defines `.previous`/`.past` fallbacks; a BARE recurrent read is not covered). Two careful readings diverge on the initial gate state: (a) the gate reads the recurrent's freshly-computed startup value (same-pass topological read per 016-146), or (b) the gate reads a nonexistent prior-commit value and must fall back to some undefined default. Reading (a) contradicts 022-37's one-commit-delay semantics; reading (b) is undefined because no startup fallback for a bare recurrent read in a gate is specified. The two readings yield different initial gate states and therefore different program behavior (whether the instance begins active). Note 022-38 says such a self-gate cannot REOPEN once closed, but says nothing about the INITIAL state at startup, which is the case at issue.
- **Direction of change:** State explicitly what a self-conditional gate's bare read of a same-instance recurrent (or any cell subject to the one-commit-delay rule) resolves to during the startup pass — e.g. the just-computed startup value, or a defined startup-fallback — and reconcile 022-37's prior-commit rule with the startup topological-order rule of 016-144/016-146. Decision belongs to the user.
- **Evidence check:** pass — A self-conditional gate reading a same-instance recurrent: 022-37 says read prior-commit value, but startup has no prior commit while 016-144/146 evaluate the gate same-pass in topological order. Two readings yield different initial gate state (active vs inactive).
- **Charity check:** sustain — For `when: counter > 5` with `counter` a same-instance recurrent: 022-37 / SPEC §13.9.6 (17629-17631) say the cyclically self-referential gate reads the recurrent's PRIOR-committed values. But 016-144 / SPEC (12303-12307) evaluate `when` in the same startup topological pass, and 016-146 lets it read the recurrent's freshly-computed startup value. At startup there is NO prior commit. 016-139 only supplies fallbacks for `.previous`/`.past`, not for a bare recurrent read. Grep confirms no text carves out the startup case for a self-gate; §13.9.6 (17646-17653) addresses only REOPEN, not the INITIAL state. 016-147 does not fire (no init-time cycle: the recurrent's init expression need not read the gate). So the program is legal but two careful readings yield different initial gate state (active vs inactive) → different behavior. Sustains as behavior-changing ambiguity / startup gap.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2921-2921`
    > 022-37. A cyclically self-referential gate predicate evaluates against the gated cells' previously-committed values from the prior commit; a compiler-synthesized wake gate inherits this rule, reading prior-commit values so a wire-following cycle resolves as a one-commit-delay fixpoint. (§13.9.6)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1997-1999`
    > 016-144. `when` predicates are evaluated within the same topological order as other cells, establishing each instance's initial gate state at startup. (§13.2.6)
    > 016-145. An instance whose `when` evaluates false at the end of startup begins inactive, its other cells holding their just-computed initial values. (§13.2.6)
    > 016-146. An initial-value expression may read any reactive cell visible in scope, regardless of declaration kind; the topological sort resolves ordering with no artificial kind-order constraint. (§13.2.6)
  - `packages/ductus-lang/docs/SPEC.md:12303-12307`
    > - **`when` predicates** (§13.9) are evaluated alongside deriveds
    >   in the topological order. Each instance's initial gate state is
    >   established here. An instance whose `when` evaluates to false at
    >   the end of startup begins inactive, with its other cells holding
    >   their just-computed initial values per Model B (§13.9.7).

#### F113 — The composite retire+gate+stream+order question has no owning section; 018 owns retire, 022 owns freeze, 030 owns stream policy, 023 owns commit order, and no rule integrates a node that is simultaneously gated-off and scope-dropped.

- **Severity/Category/Verdict:** MED / gap / PLAUSIBLE
- **Anchors:** LOG 018-35, 018-78, 022-8, 022-63, 030-191 · SPEC § 13.5.4, 13.9, 13.9.7, 13.18.12
- **Why it is a defect:** In the scenario the node is already gated-off (frozen, 022-8) at the moment its Map key is removed. Freeze and retire are governed by disjoint sections with disjoint vocabularies: 022 says a gated instance is only frozen and 'resume[s] when the gate reopens' (022-8) and never unmounts (022-9); 018-78 unconditionally drops the scope on key removal. No rule states that scope_drop supersedes an in-progress freeze, nor what teardown a frozen (already-suspended, per 022-63) subtree receives when retired — e.g. whether an effect that already got `suspend` at gate-close now gets a second teardown/`drop` at scope_drop, or whether a stream consumer that is currently in freeze-and-backlog (030-191) transitions to the (unspecified, see finding 1) scope-drop path. The scenario's own question 'Does any section own the composite answer?' resolves to NO: the answer must be reassembled from 018 (retire), 022 (freeze/suspend), 030 (stream cursor), and 023 (order), and the seams between them are unspecified.
- **Direction of change:** Decide and record which section owns the retire-while-gated composite (candidate: 018, since retire is repeat's concern), and add a rule stating that scope_drop supersedes any concurrent gate-freeze and specifying the teardown sequence (effect drop vs the already-fired suspend; stream cursor/hold release) for a frozen subtree that is retired in the same commit.
- **Evidence check:** pass — A node gated-off (frozen, 022-8, never unmounted per 022-9, effects already suspended per 022-63, stream in freeze-and-backlog per 030-191) whose repeat key is then removed (scope drop, 018) hits disjoint sections with disjoint vocabularies and no integrating rule for the seam — e.g. double teardown of an already-suspended effect, or a frozen stream consumer's transition to scope-drop. Real integration gap; the composite answer is unowned.
- **Charity check:** refute — The finding claims 'no section owns the composite answer' for an already-gated-off (frozen/suspended) instance whose scope is then dropped. §13.14.9 explicitly owns exactly this seam: SPEC:19169-19174 — 'Ordering when scope death overlaps a suspended state: a suspended instance whose scope then dies receives `teardown` directly (not a `resume` first) — teardown subsumes the release suspend already performed, and the reconciler must tolerate `teardown` arriving while suspended. The runtime never delivers `resume` to an instance that is leaving scope.' This answers all three effect sub-questions: no double release (teardown subsumes suspend), no resume-before-drop, scope-death supersedes freeze. The stream-consumer sub-seam is covered by 028-49 ('A removed consumer's cursor is dropped'): scope death removes the consumer, dropping its cursor regardless of the 030-191 freeze-and-backlog state (freeze = cursor paused; removal = cursor dropped). Consistency: the dissolver INTEGRATES the finding's cited passages (022-63 suspend at gate-close, then 018-78 scope_drop → teardown) rather than conflicting with them; 022-8/022-9 govern the gate axis only, scope-drop is the separate removal axis the integrating rule sequences. Refuted. | SPEC.md:19169-19174 — "**Ordering when scope death overlaps a suspended state:** a suspended instance whose scope then dies receives `teardown` directly (not a `resume` first) — teardown subsumes the release suspend already performed, and the reconciler must tolerate `teardown` arriving while suspended. The runtime never delivers `resume` to an instance that is leaving scope."
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2505-2505`
    > 018-35. Conditional-activation gates never add or remove instances; they only freeze an unconditionally-constructed instance while it is inactive. (§13.5.4)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2892-2893`
    > 022-8. A gated-off instance is frozen: its cells hold their values and resume when the gate reopens. (§13.9)
    > 022-9. Gates never unmount or reconstruct an instance. (§13.9)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2947-2947`
    > 022-63. At gate-close the runtime fires the `suspend` reconciler hook for every effect in the newly-frozen subtree, at the commit boundary. (§13.9.7)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3697-3697`
    > 030-191. When a consumer's enclosing subtree is gated off, its cursor stops advancing; the default behavior is freeze-and-backlog. (§13.18.12)

#### F170 — 024-26 claims wire-following (wake-gate) cycles resolve by the one-commit-delay fixpoint 'only when guarded by a Circularity connection', but 022-37 says a wake gate reads prior-commit values unconditionally, so the delay resolution is not conditional on Circularity.

- **Severity/Category/Verdict:** LOW / logical_flaw / CONFIRMED
- **Anchors:** LOG 024-26, 024-20 · SPEC § 13.11.5
- **Why it is a defect:** 022-37 makes the one-commit-delay fixpoint an unconditional consequence of a wake gate reading prior-commit values — no Circularity precondition. 024-26 (in scope) states the delay-fixpoint resolution occurs 'only when guarded by a `Circularity` connection', conflating runtime read behavior (always prior-commit, per 022-37) with the compile-time legality gate (needs Circularity, per 024-20). A wire-following cycle lacking Circularity is a compile error (024-20), so it never runs — but that is a legality fact, not a change in how the wake gate resolves. The literal 'only when' claim is falsifiable against 022-37. Not implementer-blocking because 024-20 governs legality and 024-26 self-labels as 'guidance'; recorded as wording/logic drift.
- **Direction of change:** Surface to user: 024-26's 'resolve ... only when guarded by a Circularity connection' should separate the always-on prior-commit read behavior (022-37) from the Circularity legality gate (024-20); do not rewrite unilaterally.
- **Evidence check:** pass — 024-26 says delay-fixpoint resolution is conditional on Circularity, but 022-37 makes prior-commit read unconditional.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:3093-3093`
    > 024-26. Connection types that imply simultaneous source-destination activation should not satisfy `Circularity`; this guidance is load-bearing because compiler-synthesized wake gates form cycles that resolve by the prior-commit, one-commit-delay fixpoint only when guarded by a `Circularity` connection. (§13.11.5)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2921-2921`
    > 022-37. A cyclically self-referential gate predicate evaluates against the gated cells' previously-committed values from the prior commit; a compiler-synthesized wake gate inherits this rule, reading prior-commit values so a wire-following cycle resolves as a one-commit-delay fixpoint. (§13.9.6)

### Lexical floor: missing keywords and parse ambiguities

The closed keyword sets omit given/cell/observe/own/requires/enum/wraps/yielded and more; the precedence table omits in and delete; dangling-else, flag-run lexing, interpolation nesting, and fold-header parses are undefined or ambiguous.

#### F251 — The normative keyword enumeration in §002 (and SPEC §1.4) omits reserved words the rest of the corpus uses — given, observe, effects, requires, own, move, dyn, cell, yielded — so a lexer built from the floor would not reserve them.

- **Severity/Category/Verdict:** HIGH / gap / CONFIRMED
- **Anchors:** LOG 002-3, 002-4, 002-6, 002-11, 002-27 · SPEC § 1.4
- **Why it is a defect:** §002 is the language's lexical authority and each entry is a closed enumeration ('The declaration keywords are …', 'The clause keywords are …', 'The control-flow keywords are …', 'The operator-context keywords are …'); SPEC §1.4 restates the same closed set. 002-27 makes every keyword globally reserved and forbids any declaration introducing an identifier of a keyword's spelling. But `given` (022-107, heads a selector block), `observe` (016-247, heads an expression block), `effects` (017-274 `effects:` clause head), `requires` (005-25, called 'the `requires` keyword'), `own` and `move` (001-18/005-12, called keywords, surface syntax `fn f(own x)`/`f(move x)`), `dyn` (004-52), and the annotation-kinds `cell`/`yielded` (016-1) are all used as reserved surface tokens yet appear in NONE of the enumerations in either document. An implementer building the reserved-word table strictly from the normative floor (§002 / §1.4) would treat these as ordinary identifiers, so `given mode:`, `observe:`, `effects:`, `requires Person`, `fn f(own x)` would fail to parse as intended and a user could legally declare `let given = 3`, directly contradicting 002-27. No legal boundary (001-6) is declared for this gap. Grep confirmed none of these tokens appears in any 'keywords are' enumeration; the answer does not live in another §002 entry (002-13..29 introduce none of them).
- **Direction of change:** Close the §002 keyword enumerations (and the parallel SPEC §1.4 list) over every reserved token the corpus uses: add the missing block/clause/expression heads and modifiers to their appropriate keyword class, or explicitly reclassify each as a non-keyword contextual token — but 002-27 currently forbids contextual keywords, so that reclassification would itself need a decision. Surface to user; do not pick a class unilaterally.
- **Evidence check:** pass — The normative keyword floor (§002 / SPEC §1.4) omits reserved words used elsewhere as surface tokens (given, observe, requires, cell, yielded verified; own/move/dyn/effects asserted). A lexer from the floor wouldn't reserve them; 'let given = 3' would be legal, contradicting 002-27. No legal boundary (001-6) declared. Core claim confirmed on disk; severity HIGH as an implementer-blocking lexical-coverage gap.
- **Charity check:** sustain — Exhaustively grepped every keyword enumeration in both docs. None of given, observe, effects, requires, own, move, dyn, cell, yielded appears in §002's closed enumerations (002-3 DECISION_LOG.md:74, 002-4 DECISION_LOG.md:75, 002-6 DECISION_LOG.md:77, 002-11 DECISION_LOG.md:82) nor in SPEC §1.4's parallel closed set (SPEC.md:126-140). Yet all are used as reserved surface tokens: `given` heads a selector block (022-6, 022-109), `observe` heads an expression block (016-247 DECISION_LOG.md:2100), `effects:` is a node-body clause head (017-9 DECISION_LOG.md:2150), `requires` is 'the requires keyword' (005-25 DECISION_LOG.md:366), `own`/`move` are called keywords (013-117 DECISION_LOG.md:1503, 013-79), `dyn` is a value/type keyword (008-33 DECISION_LOG.md:893), `cell`/`yielded` are annotation-kinds (016-1 DECISION_LOG.md:1854). 002-27 (DECISION_LOG.md:98) forbids declaring an identifier of a keyword's spelling — but an implementer building the reserved table strictly from §002/§1.4 never adds these, so `let given = 3`, `fn f(own x)`, `observe:`, `effects:`, `requires Person` misbehave. Partial charitable offset: 020-23 (DECISION_LOG.md:2721) reserves cell/yielded as cell-names only (instance-body scope), not globally, and covers none of given/observe/effects/requires/own/move/dyn. SPEC §13.2.8 (SPEC.md:20517) and §11.7.4 (SPEC.m
d:8957) describe own/cell as keywords but that is elaboration, not a §1.4 enumeration entry. No legal boundary (001-6). HIGH implementer-blocking lexical-coverage gap on the normative floor.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:74-74`
    > 002-3. The declaration keywords are `node`, `connection`, `trait`, `type`, `fn`, `operator`, `effect`, `signal`, `attr`, `recurrent`, `derived`, `stream`, `view`, `const`, `let`, `mut`, `repeat`, `main`, `collect`; `collect` heads the collect block expression and its `collect as x:` statement form. `fold` is a keyword too, but heads an expression form (the fold form) rather than a declaration. (§1.4)
  - `packages/ductus-lang/docs/DECISION_LOG.md:75-75`
    > 002-4. The clause keywords are `children`, `incoming`, `outgoing`, `expose`, `when`, `satisfies`, `fulfill`, `default`, `otherwise`, `from`, `to`, `pairs`, `on`, `where`, `desired`, `observed`, `ring`, `gate`, `keyed`, `at`, `dynamic` (the supply-mode marker), `by`, `else`; `by` and `else` head the two arms of the fold form (`by:` the combiner, `else:` the empty-membership result). No clause keywords are removed. (§1.4)
  - `packages/ductus-lang/docs/DECISION_LOG.md:98-98`
    > 002-27. Every keyword is reserved in every position and can never be an ordinary identifier; no keyword class is contextual. (§1.4)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2991-2991`
    > 022-107. `given` gates among constructed arms by the current variant of a sum-typed scrutinee: `given mode:` with arm `Realtime: RealtimeChain`. (§13.9.13)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2100-2100`
    > 016-247. An `observe` expression is written as `observe:` followed by an indented block of `on`-clause arms and an optional `default:` arm. (§13.2.11.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:366-366`
    > 005-25. A trait declares super-trait requirements with the `requires` keyword: `requires Person`. (§3.1.4)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1854-1854`
    > the declaration keywords that introduce a cell and the lowercase kind annotations that name a cell in an annotation position (`signal T`, `derived T`, `recurrent[N] T`, `stream ring[N] T`, `yielded T`, `cell T`)
  - `packages/ductus-lang/docs/SPEC.md:135-136`
    > the clause keywords above), all control-flow keywords (`if`, `else`,
    > `match`, `for`, `in`, `while`, `break`, `continue`, `return`), the

#### F037 — 007-231 justifies `not (k in s)` by 'not binds looser than comparison', but the `in` membership operator has no precedence tier anywhere; §4.4.7's table omits it, leaving `x in 0..n` unparseable.

- **Severity/Category/Verdict:** MED / gap / CONFIRMED
- **Anchors:** LOG 007-231 · SPEC § 4.9.5, 4.4.7
- **Why it is a defect:** 007-231 (and its §4.4.7-anchored justification) treats `in` as a comparison-tier operator ('not binds looser than comparison'), but `in` appears nowhere in the §4.4.7 precedence table (SPEC 3339-3355) and no decision assigns it a tier. Contrast 007-239/§4.9.5, which explicitly places `delete` ('same precedence tier as the other operator-context keywords'). The gap is behavior-affecting: `x in 0..n` — since ranges (tier 8) are a natural `Contains` subject, an implementer cannot tell whether this parses `x in (0..n)` (membership in a range) or `(x in 0)..n`. Per 001-6 the only legal stopping points are std-delegation or implementation-defined; neither is invoked, so `in`'s precedence is an unfilled hole. The 'since not binds looser than comparison' clause is also a non-sequitur: it presumes the very tier placement that is missing.
- **Direction of change:** Add `in` (membership) to the §4.4.7 precedence table at a defined tier and state a corresponding decision, OR explicitly mark `in`'s precedence implementation-defined. Note the same unpinned-tier issue recurs verbatim in out-of-scope 012-104. Surface to user; do not pick the tier unilaterally.
- **Evidence check:** pass — The 'in' membership operator has no tier in the §4.4.7 precedence table nor any decision assigning one, so x in 0..n cannot be deterministically parsed as x in (0..n) vs (x in 0)..n; the 'binds looser than comparison' justification presumes a tier placement that does not exist.
- **Charity check:** sustain — Gap confirmed; no explicit normative text assigns `in` a precedence tier. The §4.4.7 table (SPEC 3339-3355) has no `in` row; tier 9 lists only `is`, `is not`, `<`, `<=`, `>`, `>=`. 007-74's precedence enumeration omits `in`. §4.9.5 (SPEC 4220-4224) only states `not binds looser than the membership operator` — placing `in` relative to `not` (tier 4) but never against `..` (tier 8) or comparison (tier 9). By contrast `delete` (SPEC 4241-4244) explicitly gets `the same precedence tier as the other operator-context keywords` — the authors know how to assign a tier and did not for `in`. So `x in 0..n` is genuinely unparseable: `x in (0..n)` vs `(x in 0)..n` is undetermined. 007-231's `since not binds looser than comparison` presumes a comparison-tier placement that no normative text supplies. Per 001-6 neither std-delegation nor implementation-defined is invoked. MED sustained.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:849-849`
    > 007-231. The `Contains[K]` trait dispatches the `in` membership operator: `k in s` desugars to `Contains::contains(s, k)`. The trait declares `fn contains(s: Subject, k: K) -> bool`. `not k in s` parses as `not (k in s)` since `not` binds looser than comparison. (§4.9.5)
  - `packages/ductus-lang/docs/SPEC.md:3339-3355`
    > | Tier | Operators                                                 | Associativity   |
    > |------|-----------------------------------------------------------|-----------------|
    > | 1    | `\|>` (operator / effect application)                     | left            |
    > | 2    | `or`                                                      | left            |
    > | 3    | `and`                                                     | left            |
    > | 4    | `not` (prefix)                                            | right           |
    > | 5    | `\|` (bitwise or)                                         | left            |
    > | 6    | `^` (bitwise xor)                                         | left            |
    > | 7    | `&` (bitwise and)                                         | left            |
    > | 8    | `..` (range)                                              | non-associative |
    > | 9    | `is`, `is not`, `<`, `<=`, `>`, `>=`                      | non-associative |
    > | 10   | `<<`, `>>` (shifts)                                       | left            |
    > | 11   | `+`, `-`                                                  | left            |
    > | 12   | `*`, `/`, `\`, `%`                                        | left            |
    > | 13   | `-`, `~`, `handle`, `handle!`, `portal` (prefix)          | right           |
    > | 14   | `?`, `.`, `[]`, `()`, and `T%()`/`T\|()`/`T?()` casts     | left           |
    > | 15   | `::`                                                      | left            |

#### F105 — 021-119 says ANY non-letter char immediately after a TypeRef opens a flag run, but `/` (the /expr slot) and `|` (attribute clause) validly follow a TypeRef and are not flags — `Drives/0.8` would mis-lex as an invalid flag run.

- **Severity/Category/Verdict:** MED / parse_ambiguity / CONFIRMED
- **Anchors:** LOG 021-119, 021-76, 021-97, 021-113 · SPEC § 13.8.8.4, 13.8.5, 13.8.7
- **Why it is a defect:** Witness fragment `Drives/0.8` (valid per 021-76). Lexing per 021-119: `Drives` is the TypeRef; `/` is a non-letter char immediately following it with no whitespace, so it is a flag-run opener; `/` is not in the flag set (021-113), so the flag run is invalid → error. But 021-76 says `Drives/0.8` is a valid /expr placement. The same character sequence is both a valid /expr and an invalid flag run. `|` (attribute clause, 021-97) has the identical problem: `Sensor|gain=0.5` has `|` immediately after TypeRef. 021-119 is stated over 'non-letter character' but the only non-letter chars that may legally open a flag run are the nine flag-set chars of 021-113; `/`, `|` (and `:` body-introducer, `(` for a parenthesized placement) are non-letter and reach a TypeRef legally without being flags. An implementer taking 021-119 literally cannot also satisfy 021-76/021-97.
- **Direction of change:** Narrow 021-119 / SPEC §13.8.8.4 so the flag-run opener is a member of the 021-113 flag set, not any non-letter character; or explicitly carve out `/`, `|`, `:`, `(` as reaching the TypeRef in their own slots.
- **Evidence check:** pass — 021-119 (repeated in SPEC §13.8.8.4) says ANY non-letter char immediately after a TypeRef opens a flag run, but '/' (021-76 /expr) and '|' (021-97 attribute clause) validly follow a TypeRef and are not in the 021-113 flag set — 'Drives/0.8' is both a valid /expr and, read literally, an invalid flag run.
- **Charity check:** sustain — 021-119 / SPEC §13.8.8.4 (line 17198-17200) state the flag-run-opener rule over 'a non-letter character' with no restriction to the flag set. / validly follows a TypeRef with no whitespace (021-76: 'Drives/0.8') and | introduces the attribute clause (021-97), both non-letter chars — so the literal rule makes them flag-run openers, then 021-113's nine-char flag set rejects them → error, contradicting 021-76/021-97. 021-120's token-doubling disambiguation lists only flag-set chars (', ?, !, @), not / or |. 021-123 gives the fixed inline order (TypeRef, flags, as, /Expr, when, attr clause) but states no lexical exclusion of /|| from the flag-run rule; SPEC line 157-158 confirms the positional rule fires (for @, which IS in the set). No quoted normative text scopes 'non-letter character' to the flag set, so the over-reaching reading is unrescued. Sustain as filed (MED).
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2858-2858`
    > 021-119. In placement position, a non-letter character immediately following the TypeRef path with no intervening whitespace is a flag-run opener. (§13.8.8.4)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2814-2814`
    > 021-76. Whitespace around the `/` in a `/expr` placement argument is insignificant: `Drives/0.8`, `Drives /0.8`, and `Drives / 0.8` are equivalent; the single-atom restriction on unparenthesized `/expr`, not adjacency, keeps a space-separated placement self-delimiting. (§13.8.5)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2852-2852`
    > 021-113. The permitted flag characters are exactly `'`, `!`, `?`, `*`, `+`, `^`, `~`, `@`, `$`. (§13.8.8.1)
  - `packages/ductus-lang/docs/SPEC.md:17198-17201`
    > Disambiguation is positional: in placement position, a non-letter
    > character immediately following the `TypeRef` path (no intervening
    > whitespace) is a flag-run opener. In any other position
    > (expression context, annotation context, etc.) it is the operator.

#### F100 — Nested inline if/else (dangling-else) has no association rule: `if a: if b: x else: y` parses two ways.

- **Severity/Category/Verdict:** MED / parse_ambiguity / CONFIRMED
- **Anchors:** LOG 002-26 · SPEC § 1.4
- **Why it is a defect:** Because an if-arm may be a single inline expression that is itself an if without an else, the program `let z = if a: if b: x else: y` has two structurally valid parses under the stated prose: the `else:` may bind to the inner `if b:` (yielding `if a: (if b: x else: y)`) or to the outer `if a:` (yielding `if a: (if b: x) else: y`). No rule in 002-26 or its SPEC elaboration states nearest-enclosing (or any) association for a dangling `else`. The two parses produce different values when `a` is true and `b` is false: parse 1 yields `y`, parse 2 yields the inner if's implicit value. This is a behavior-changing ambiguity with no disambiguation rule anywhere in the LOG.
- **Direction of change:** Add a decision fixing dangling-`else` association for inline `if`/`else` (e.g. bind to nearest unmatched `if`) or forbid an un-parenthesized bare inline `if` as an inline arm.
- **Evidence check:** pass — Inline if-arm may itself be an if without else, so 'if a: if b: x else: y' has two valid parses (else binds inner vs outer) with different runtime values when a true/b false. No association rule in 002-26 or SPEC §1.4. Verified absent.
- **Charity check:** sustain — Hunted the whole corpus for any dangling-else disambiguation and found none. 002-26 (DECISION_LOG.md:97) and SPEC §1.4 (SPEC.md:186) both allow an if/else arm to be a single inline expression that is itself an if, but neither states a nearest-enclosing (or any) association rule for a dangling else, and I found no rule requiring an else on a value if (searched exhaustiveness/else-optionality: 014-4/014-87 cover only loop-else; no if-else-mandatory rule). The corpus is meticulous about association elsewhere — 008-11 (dyn vs &), 008-37 (dyn binding), 009-35 (with left-assoc), 013-131/139 (move parenthesization), 021-75/77 (/expr atom) all give explicit precedence/association — which makes the SILENCE on nested inline if/else a genuine behavior-changing gap, not an oversight the reader can fill by a forced reading. For `let z = if a: if b: x else: y` two structurally valid parses yield different values when a=true,b=false. No normative text forces one parse. Per the ambiguity standard, the existence of a plausible second reading with no disambiguating rule IS the defect.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:97-97`
    > 002-26. A function body and `if`/`else` and `match` arms may be a single inline expression after the `:` or an indented block: `fn double(x: i32) -> i32: x * 2`. (§1.4)
  - `packages/ductus-lang/docs/SPEC.md:186-186`
    > A function body and the arms of `if`/`else` and `match` may be written either as an indented block or as a single expression inline after the `:` (`fn double(x: i32) -> i32: x * 2`; `if a > 0: a else: -a`).

#### F252 — LOG §002 lists `collect`, `fold`, and `yield` as keywords but SPEC §1.4's parallel keyword enumeration omits all three — a LOG-SPEC divergence in the normative keyword set.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 002-3, 002-6 · SPEC § 1.4
- **Why it is a defect:** LOG 002-3 explicitly lists `collect` as a declaration keyword and names `fold` a keyword; LOG 002-6 lists `yield` as a control-flow keyword. SPEC §1.4 is the conforming elaboration of the same lexical rules (both carry (§1.4)), and it presents the SAME closed enumeration — 'all declaration keywords (…)' and 'all control-flow keywords (…)' — but its declaration list ends at `main` (no `collect`, no `fold`) and its control-flow list ends at `return` (no `yield`). Per the log's Invariant/edit protocol SPEC must conform to LOG; divergence is a defect. A reader who trusts SPEC §1.4 as the keyword authority gets a strictly smaller reserved set than LOG mandates. (This is a distinct, narrower defect than the joint gap above: here the tokens ARE in LOG but dropped from SPEC.)
- **Direction of change:** Amend SPEC §1.4 to include `collect` and `fold` in the declaration/expression keyword listing and `yield` in the control-flow keyword listing, conforming to LOG 002-3 and 002-6. Editing LOG-first is unnecessary since LOG already has them; conform SPEC. Confirm the LOG side is the intended set before editing.
- **Evidence check:** pass — LOG 002-3 lists collect/fold and 002-6 lists yield as keywords; SPEC §1.4's parallel closed enumeration omits all three. SPEC must conform to LOG; a reader trusting SPEC §1.4 gets a strictly smaller reserved set. Distinct from F251 (here tokens ARE in LOG, dropped from SPEC). Confirmed.
- **Charity check:** sustain — Direct LOG-vs-SPEC comparison confirms the divergence. LOG 002-3 (DECISION_LOG.md:74) explicitly ends its declaration-keyword list with '... main, collect' and names 'fold' a keyword; LOG 002-6 (DECISION_LOG.md:77) ends its control-flow list with '... return, yield'. SPEC §1.4 presents the SAME closed enumeration form ('all declaration keywords (...)' SPEC.md:126-128, 'all control-flow keywords (...)' SPEC.md:135-136) but its declaration list ends at 'main' (no collect, no fold) and its control-flow list ends at 'return' (no yield). Both carry (§1.4); SPEC must conform to LOG (edit protocol / Invariant 3). A reader trusting SPEC §1.4 as the keyword authority gets a strictly smaller reserved set than LOG mandates. This is distinct from F251 (there the tokens are absent from BOTH docs); here the three tokens ARE in the LOG but dropped from the SPEC restatement — a substantive LOG-SPEC divergence, MED.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:74-74`
    > 002-3. The declaration keywords are `node`, `connection`, `trait`, `type`, `fn`, `operator`, `effect`, `signal`, `attr`, `recurrent`, `derived`, `stream`, `view`, `const`, `let`, `mut`, `repeat`, `main`, `collect`; `collect` heads the collect block expression and its `collect as x:` statement form. `fold` is a keyword too, but heads an expression form (the fold form) rather than a declaration. (§1.4)
  - `packages/ductus-lang/docs/DECISION_LOG.md:77-77`
    > 002-6. The control-flow keywords are `if`, `else`, `match`, `for`, `in`, `while`, `break`, `continue`, `return`, `yield`; `yield` is legal only directly under a `collect` and contributes one member-cell.
  - `packages/ductus-lang/docs/SPEC.md:126-128`
    > This includes all declaration keywords (`node`, `connection`, `trait`,
    > `type`, `fn`, `operator`, `effect`, `signal`, `attr`, `recurrent`,
    > `derived`, `stream`, `view`, `const`, `let`, `mut`, `repeat`, `main`), all clause
  - `packages/ductus-lang/docs/SPEC.md:135-136`
    > the clause keywords above), all control-flow keywords (`if`, `else`,
    > `match`, `for`, `in`, `while`, `break`, `continue`, `return`), the

#### F060 — SPEC calls `cell` a reserved lowercase keyword, but section 002's keyword enumerations never list `cell`, so a lexer built from the LOG will not reserve it.

- **Severity/Category/Verdict:** MED / gap / CONFIRMED
- **Anchors:** LOG 002-3, 002-4, 002-6, 002-27, 016-1, 016-62, 016-178 · SPEC § 1.1, 1.4, 13.2.8
- **Why it is a defect:** 001-3 commits that 'Lexical and syntactic structure is specified directly in this specification and decision log', and section 002's keyword lists (002-3 declaration, 002-4 clause, 002-6 control-flow) read as the exhaustive keyword inventory, reinforced by 002-27's 'every keyword is reserved in every position'. SPEC §13.2.8 (line 20517) states `cell` is 'a lowercase keyword kind', peer to `signal`/`derived`/`recurrent`/`stream` — all of which ARE listed in 002-3. `cell` is listed in none of 002-3/002-4/002-6, and unlike the others it has no declaration-keyword form introducing it (it appears only as the `cell T` annotation), so nothing in section 002 reserves the word. An implementer building the lexer from the LOG's keyword tables will treat `cell` as an ordinary identifier: `let cell = 5` compiles, and a `cell T` annotation then collides with a user binding named `cell`. The kind-taxonomy decisions (016-1, 016-62, 016-178) rely on `cell` being a reserved annotation keyword, but the lexical section that must reserve it is silent. This is an implementer-blocking lexical-coverage gap between the two documents (and internally, between section 002 and section 016).
- **Direction of change:** Decide whether `cell` is a reserved keyword (as SPEC 20517 and 016-62/016-178 assume) and, if so, add it to the appropriate keyword list in section 002 (and conform §1.4); if it is instead a non-reserved annotation form, correct SPEC 20517's 'lowercase keyword kind' wording. This is a keyword-inventory decision to bring to the user, not to resolve unilaterally.
- **Evidence check:** pass — SPEC §13.18.5 calls 'cell' a lowercase reserved keyword-kind (peer to signal/derived/recurrent/stream, all of which ARE in 002-3), but §002 lists never reserve 'cell'. Lexer from LOG treats it as identifier; 'let cell = 5' compiles, colliding with 'cell T' annotations. Confirmed.
- **Charity check:** sustain — Charitable hunt found a partial reservation: 020-23 (DECISION_LOG.md:2721) says 'Reserved words cannot be declared as cell names; the reserved set includes collect, yield, fold, by, cell, and yielded' — so `cell` IS named reserved somewhere in the LOG. BUT this does NOT dissolve the gap: 020-23 is scoped to cell-NAMES in the instance body namespace (§13.7.5), which is narrower than 002-27's global-position reservation and does not add `cell` to the §002 lexer keyword table. The §002 keyword enumerations (002-3/4/6, DECISION_LOG.md:74/75/77) — which read as the exhaustive keyword inventory reinforced by 002-27 — never list `cell`, and SPEC §1.4 (SPEC.md:125-144) never lists it, while SPEC §13.2.8 (SPEC.md:20517) calls it 'a lowercase keyword kind ... exactly like signal, derived, recurrent, and stream' — all four of which ARE in 002-3. `cell` alone has no declaration-keyword form and appears in no §002 enumeration, so an implementer building the lexer from §002/§1.4 treats it as an ordinary identifier: `let cell = 5` then collides with a `cell T` annotation. 016-62 (DECISION_LOG.md:1915) relies on `cell` being a reserved annotation keyword; §002 is silent. Implementer-blocking lexical gap between §002 and §016; 020-23's cell-name-scoped reservation is not a lexer-level reservation. No legal boundary declared.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:74-74`
    > 002-3. The declaration keywords are `node`, `connection`, `trait`, `type`, `fn`, `operator`, `effect`, `signal`, `attr`, `recurrent`, `derived`, `stream`, `view`, `const`, `let`, `mut`, `repeat`, `main`, `collect`; `collect` heads the collect block expression and its `collect as x:` statement form. `fold` is a keyword too, but heads an expression form (the fold form) rather than a declaration. (§1.4)
  - `packages/ductus-lang/docs/DECISION_LOG.md:98-98`
    > 002-27. Every keyword is reserved in every position and can never be an ordinary identifier; no keyword class is contextual. (§1.4)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1915-1915`
    > 016-62. `cell T` is the umbrella KIND — a lowercase kind annotation, not a type or a trait — over every reactive cell, whose membership is all signals, deriveds, recurrents, streams, and yielded groups. (§13.2.8)
  - `packages/ductus-lang/docs/SPEC.md:20515-20518`
    > `cell T` is the umbrella **KIND** over the reactive cell forms that
    > carry values of type `T`. It is not a trait and not a value type
    > (§13.2.8): `cell` is a lowercase keyword kind occupying an annotation
    > position, exactly like `signal`, `derived`, `recurrent`, and `stream`.

#### F007 — `given` is used as a structure-gating block-header keyword across the LOG but is never enumerated as a keyword in section 002 or SPEC §1.4, while 002-27 declares every keyword reserved.

- **Severity/Category/Verdict:** MED / gap / CONFIRMED
- **Anchors:** LOG 002-3, 002-4, 002-6, 002-27, 022-6 · SPEC § 1.4
- **Why it is a defect:** 002-2/002-27 make the keyword enumeration normative and exhaustive, yet `given` — a distinct block-header form that gates structure (022-6, and offered as a diagnostic remedy in 034-7) with its own `default:` fallback arm — appears in no keyword list in 002-3, 002-4, or 002-6, and SPEC §1.4 (lines 115-193) never mentions `given`. An implementer building the lexer/keyword table from section 002 would not reserve `given`, so `given` could be used as an ordinary snake_case identifier, contradicting its keyword role. `otherwise`/`default` fallback spellings are listed but the block heads `when`(listed as clause keyword) and `given`(unlisted) are treated asymmetrically.
- **Direction of change:** Surface to user: decide whether `given` should be added to the control-flow (or clause) keyword enumeration in 002 and SPEC §1.4, or whether it is intentionally excluded; do not resolve unilaterally.
- **Evidence check:** pass — 'given' heads a structure-gating block yet is never enumerated/reserved as a keyword in §002 lists or SPEC §1.4, so a lexer built from the floor would not reserve it — usable as identifier, contradicting 002-27. Confirmed; minor imprecision that 002-6 does mention the word 'given' in passing.
- **Charity check:** sustain — Grepped every 'keywords are' enumeration and every reserved-word statement. `given` appears in NO keyword enumeration in §002 (002-3 declaration DECISION_LOG.md:74, 002-4 clause DECISION_LOG.md:75, 002-6 control-flow DECISION_LOG.md:77, 002-11 operator-context DECISION_LOG.md:82) and NO SPEC §1.4 keyword list (SPEC.md:125-144). Yet `given` heads a structure-gating block (022-6 DECISION_LOG.md:2890 'Exactly four surfaces gate structure ... and the given block'; 022-109 DECISION_LOG.md:2993) with its own default: arm (002-6). 002-27 (DECISION_LOG.md:98) makes reservation global and forbids any declaration of a keyword's spelling, but an implementer building the reserved-word table from §002's enumerations would not reserve `given`, so `let given = 3` compiles — contradicting its keyword role. I checked whether 020-23 (DECISION_LOG.md:2721, reserved-cell-name set) rescues it: it does not list `given` and is scoped to cell-names in the instance body, not the global lexer table. No legal boundary (001-6) declared. Gap sustained.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:77-77`
    > 002-6. The control-flow keywords are `if`, `else`, `match`, `for`, `in`, `while`, `break`, `continue`, `return`, `yield`; `yield` is legal only directly under a `collect` and contributes one member-cell. The keyword `else` has exactly two senses: the loop-`else` arm and the fold-form empty-membership `else:` arm; the fallback arms of `when:` and `given` blocks are spelled `otherwise:` and `default:`, so no third `else` sense exists. (§1.4)
  - `packages/ductus-lang/docs/DECISION_LOG.md:74-74`
    > 002-3. The declaration keywords are `node`, `connection`, `trait`, `type`, `fn`, `operator`, `effect`, `signal`, `attr`, `recurrent`, `derived`, `stream`, `view`, `const`, `let`, `mut`, `repeat`, `main`, `collect`; `collect` heads the collect block expression and its `collect as x:` statement form. `fold` is a keyword too, but heads an expression form (the fold form) rather than a declaration. (§1.4)
  - `packages/ductus-lang/docs/DECISION_LOG.md:98-98`
    > 002-27. Every keyword is reserved in every position and can never be an ordinary identifier; no keyword class is contextual. (§1.4)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2890-2890`
    > 022-6. Exactly four surfaces gate structure: type-level `when:`, the placement modifier `when`, the `when` block, and the `given` block. (§13.9)
  - `packages/ductus-lang/docs/SPEC.md:125-139`
    > **Keywords are always lowercase.** No keyword has a capitalized form.
    > This includes all declaration keywords (`node`, `connection`, `trait`,
    > `type`, `fn`, `operator`, `effect`, `signal`, `attr`, `recurrent`,
    > `derived`, `stream`, `view`, `const`, `let`, `mut`, `repeat`, `main`), all clause
    > keywords (`incoming`, `outgoing`, `expose`, `when`,
    > `satisfies`, `fulfill`, `default`, `otherwise`, `from`, `to`, `pairs`, `on`,
    > `where`, `desired`, `observed`, `ring`, `gate`, `keyed`, `at`,
    > `dynamic` — the supply-mode marker, §13.3.3.1), the reserved
    > instance-field names (`pair`, `exposition` — §13.7.5; the
    > remaining fields `from`, `to`, `incoming`, `outgoing` double as
    > the clause keywords above), all control-flow keywords (`if`, `else`,
    > `match`, `for`, `in`, `while`, `break`, `continue`, `return`), the

#### F141 — The block-heading token `observe` (016-68) heads an observe block for recurrent triggers but is absent from all four closed keyword sets.

- **Severity/Category/Verdict:** MED / invariant_violation / CONFIRMED
- **Anchors:** LOG 016-68, 002-3, 002-4 · SPEC § 1.4, 13.2.4
- **Why it is a defect:** `observe` heads a block expression (parallel to `collect`, which IS listed in 002-3 as an expression-block head). `observe` is absent from every closed set. Its arms use `on`/`where`/`default` clauses (all reserved), so `observe` must itself be reserved; a reserved-word table built from the closed sets omits it, contradicting 002-27.
- **Direction of change:** Add `observe` to a closed keyword set (declaration/expression-head), or reconcile its keyword status.
- **Evidence check:** pass — 'observe' is a block-heading reserved token per 015-2/016-68/016-247 but appears in none of the four closed keyword sets (002-3/002-4/002-6/002-7-8), so a reserved-word table built from those sets omits it, contradicting 002-27's every-keyword-reserved rule.
- **Charity check:** sustain — `observe` heads a block expression (015-2 names it a reactive expression form; 016-247 writes it as `observe:` with reserved `on`/`where`/`default` arms), so it must be a reserved keyword. Grep of the closed keyword sets (LOG 002-3..002-12 and SPEC §1.4 keyword enumeration at SPEC.md:118-142) shows the literal token `observe` (backticked) is enumerated in NONE of them, while parallel block-heads `collect`/`fold`/`yield` ARE listed in 002-3/002-6. No entry anywhere declares `observe` a keyword (grep for `\`observe\`.*keyword|reserved.*\`observe\`` returns nothing). 002-27 requires every keyword reserved in every position with no contextual keywords, so a lexer built from the enumerated closed sets would fail to reserve `observe`, permitting `let observe = ...` that then collides with the `observe:` block head. Genuine invariant/completeness gap; nothing dissolves it.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1921-1921`
    > 016-68. Explicit triggers are written by making the recurrent's expression an `observe` block. (§13.2.4)
  - `packages/ductus-lang/docs/DECISION_LOG.md:74-74`
    > 002-3. The declaration keywords are `node`, `connection`, `trait`, `type`, `fn`, `operator`, `effect`, `signal`, `attr`, `recurrent`, `derived`, `stream`, `view`, `const`, `let`, `mut`, `repeat`, `main`, `collect`; `collect` heads the collect block expression and its `collect as x:` statement form. `fold` is a keyword too, but heads an expression form (the fold form) rather than a declaration. (§1.4)

#### F140 — The token `wraps` heads a newtype clause (009-103) but is absent from the clause keyword set and all other closed sets.

- **Severity/Category/Verdict:** MED / invariant_violation / CONFIRMED
- **Anchors:** LOG 009-103, 002-4 · SPEC § 1.4, 6.3.1
- **Why it is a defect:** `wraps` heads a body clause naming the underlying type of a newtype — a fixed language construct, not a user field. It is absent from the clause keyword set 002-4 and every other closed set, so the reserved-word table omits it and `wraps` could be an ordinary identifier, violating 002-27.
- **Direction of change:** Add `wraps` to the clause keyword set 002-4, or reconcile its status.
- **Evidence check:** pass — `wraps` heads a newtype body clause (009-103) but is in no clause/declaration keyword set, so a reserved-word table built from the closed sets would not reserve it — tension with 002-27.
- **Charity check:** sustain — `wraps` heads a newtype body clause (009-103 `type UserId:` with `wraps i64`; 009-105 'A `wraps` body may include at most one `satisfies` clause'), so it must be reserved for `wraps i64` to parse as a clause rather than identifier-then-type. But it is absent from the clause keyword set 002-4 (whose closing 'No clause keywords are removed' signals a complete set) and from every other closed set; the SPEC §1.4 clause-keyword enumeration (SPEC.md:129-132) also omits `wraps`. 002-27 only reserves things that ARE keywords; nothing makes a construct-heading token a keyword absent enumeration. No dissolving rule found: grep of both docs shows `wraps` never declared keyword/reserved. Sustain.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1040-1040`
    > 009-103. A newtype is declared with `type` and a body containing a `wraps` clause naming the underlying type: `type UserId:` with `wraps i64`. (§6.3.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:75-75`
    > 002-4. The clause keywords are `children`, `incoming`, `outgoing`, `expose`, `when`, `satisfies`, `fulfill`, `default`, `otherwise`, `from`, `to`, `pairs`, `on`, `where`, `desired`, `observed`, `ring`, `gate`, `keyed`, `at`, `dynamic` (the supply-mode marker), `by`, `else`; `by` and `else` head the two arms of the fold form (`by:` the combiner, `else:` the empty-membership result). No clause keywords are removed. (§1.4)

#### F139 — The token `requires` is explicitly called a keyword/clause (005-25) heading super-trait clauses, but is absent from all four closed sets.

- **Severity/Category/Verdict:** MED / invariant_violation / CONFIRMED
- **Anchors:** LOG 005-25, 002-4 · SPEC § 1.4, 3.1.4
- **Why it is a defect:** 005-25 calls `requires` 'the requires keyword' heading a trait-body clause, sibling to `satisfies`/`fulfill` which ARE listed in 002-4. `requires` is omitted from the clause keyword set and every other set, so a reserved-word table built from the closed sets would not reserve it, contradicting 002-27.
- **Direction of change:** Add `requires` to the clause keyword set 002-4, or reconcile.
- **Evidence check:** pass — 005-25 calls `requires` a keyword heading super-trait clauses (sibling to satisfies/fulfill which ARE in 002-4), but it is absent from all four closed keyword sets; contradicts 002-27.
- **Charity check:** sustain — `requires` is explicitly a keyword — LOG 005-25 'the `requires` keyword: `requires Person`', SPEC 1330 'Requirements are declared with the `requires` keyword' — yet it appears in none of §002's four enumerated keyword-class sets nor in SPEC §1.4's enumeration (SPEC 125-142). Its siblings `satisfies`/`fulfill` ARE listed in 002-4, but `requires` is not, and no other group carries it. A reserved-word table built from the closed sets would not reserve `requires`, contradicting 002-27. No dissolving passage: the enumerations are the normative keyword listing (002-2) and `requires` belongs to no listed class. Same completeness/invariant gap as F136. MED.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:366-366`
    > 005-25. A trait declares super-trait requirements with the `requires` keyword: `requires Person`. (§3.1.4)
  - `packages/ductus-lang/docs/DECISION_LOG.md:75-75`
    > 002-4. The clause keywords are `children`, `incoming`, `outgoing`, `expose`, `when`, `satisfies`, `fulfill`, `default`, `otherwise`, `from`, `to`, `pairs`, `on`, `where`, `desired`, `observed`, `ring`, `gate`, `keyed`, `at`, `dynamic` (the supply-mode marker), `by`, `else`; `by` and `else` head the two arms of the fold form (`by:` the combiner, `else:` the empty-membership result). No clause keywords are removed. (§1.4)

#### F138 — The block-heading token `given` is a first-class selector construct (`given <scrutinee>:`, 022-109) but is absent from all four closed keyword sets; it appears only in prose.

- **Severity/Category/Verdict:** MED / invariant_violation / CONFIRMED
- **Anchors:** LOG 022-109, 002-6, 002-4 · SPEC § 1.4, 13.9.13
- **Why it is a defect:** `given` heads a block construct with its own colon-body, structurally parallel to `when` and `match` (both keywords). 002-6 references '`given` blocks' in prose but never enumerates `given` as a keyword in any closed set. Per 002-27 it must be reserved in every position; a reserved-word table built from the closed sets omits it, so `given` could be parsed as an identifier, breaking 022-109.
- **Direction of change:** Add `given` to a closed keyword set (control-flow or a selector-head set), or reconcile its keyword status.
- **Evidence check:** pass — `given` heads a colon-body block construct structurally parallel to when/match but appears in no closed keyword set (002-4/002-6), only in prose. A reserved-word table built from the closed sets omits it, admitting `given` as an identifier and breaking 022-109.
- **Charity check:** sustain — `given` heads a first-class block construct (022-109; also 009-92, 017-239) structurally parallel to `when` (a clause keyword, 002-4) and `match` (a control-flow keyword, 002-6). Yet `given` appears in NONE of the four closed keyword sets: 002-3 (declaration), 002-4 (clause), 002-6 (control-flow), 002-11 (operator-context). Grep across the whole corpus finds `given` only in prose ('`when:` and `given` blocks', 002-6) and in construct-definition entries, never in any keyword/reserved enumeration. 002-27 requires every keyword reserved in every position and no keyword class contextual; a reserved-word table built from the closed sets omits `given`, so it could be parsed as an identifier, breaking 022-109. No dissolving text exists. Sustains as an invariant/completeness gap in the closed keyword sets.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2993-2993`
    > 022-109. `given <scrutinee>:` introduces variant-pattern arms written exactly as value-`match` arms: bare `Pattern: body` with no per-arm keyword. (§13.9.13)
  - `packages/ductus-lang/docs/DECISION_LOG.md:77-77`
    > 002-6. The control-flow keywords are `if`, `else`, `match`, `for`, `in`, `while`, `break`, `continue`, `return`, `yield`; `yield` is legal only directly under a `collect` and contributes one member-cell. The keyword `else` has exactly two senses: the loop-`else` arm and the fold-form empty-membership `else:` arm; the fallback arms of `when:` and `given` blocks are spelled `otherwise:` and `default:`, so no third `else` sense exists. (§1.4)

#### F137 — The token `enum` is explicitly called a keyword (009-48) and heads a declaration body, but is absent from the declaration keyword set 002-3 and all other closed sets.

- **Severity/Category/Verdict:** MED / invariant_violation / CONFIRMED
- **Anchors:** LOG 009-48, 002-3 · SPEC § 1.4, 6.2.1
- **Why it is a defect:** 009-48 calls `enum` 'the enum keyword' and 002-25 lists `enum` among body-heading declaration forms alongside trait/type/node/connection — all of which ARE in 002-3. But 002-3's declaration keyword enumeration omits `enum`. A reserved-word table built from the closed sets would not reserve `enum`, contradicting its keyword status and letting `enum` be used as an ordinary identifier in violation of 002-27.
- **Direction of change:** Add `enum` to the declaration keyword set 002-3, or reconcile the enumeration with 009-48/002-25.
- **Evidence check:** pass — 009-48 calls `enum` a keyword and 002-25 groups it with trait/type/node/connection (all in 002-3), but 002-3's declaration-keyword enumeration omits `enum`, so the reserved-word coverage misses it — tension with 002-27.
- **Charity check:** sustain — 009-48 explicitly calls it 'the `enum` keyword' and 002-25 groups `enum` bodies with trait/type/node/connection (all of which ARE in the 002-3 declaration-keyword set), yet 002-3's definite-article enumeration 'The declaration keywords are …' omits `enum`. The SPEC §1.4 declaration-keyword list (SPEC.md:126-128) also omits `enum` (grep-confirmed 0 hits for enum in that list). So one entry names `enum` a keyword while the closed set that should reserve it excludes it — internal contradiction, and a reserved-word table built from the closed sets would not reserve `enum`, contradicting 002-27. No dissolving text. Sustain.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:985-985`
    > 009-48. An enum is declared with the `enum` keyword and a body of variant declarations: `enum Option[T]:` with variants `Some(T)` and `None`. (§6.2.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:96-96`
    > 002-25. A `trait`/`type`/`enum`/`node`/`connection` body is always an indented block, never written inline after the `:`. (§1.4)

#### F136 — The token `own` is explicitly called a keyword (005-12/005-13) but is absent from all four closed keyword sets in 002-3/002-4/002-6/002-11.

- **Severity/Category/Verdict:** MED / invariant_violation / CONFIRMED
- **Anchors:** LOG 005-12, 002-3, 002-11 · SPEC § 1.4, 3.1.1
- **Why it is a defect:** 002-2 declares the keyword enumeration normative; 002-27 says every keyword is reserved and no keyword class is contextual. `own` is called 'the own keyword' yet appears in none of the four enumerated closed sets. Either the sets are incomplete (invariant violation) or `own` is not reserved (contradicting 005-12). An implementer building the reserved-word table from 002-3/4/6/11 would omit `own`.
- **Direction of change:** Add `own` to the appropriate closed set (or add an explicit keyword class for parameter-mode keywords), or reconcile 005-12's 'keyword' claim with the enumeration.
- **Evidence check:** pass — 005-12 calls `own` a keyword but it is absent from all four closed keyword sets (002-3/4/6/11); a reserved-word table built from the sets omits it, contradicting 002-27.
- **Charity check:** sustain — `own` is explicitly a keyword — LOG 005-12 'the `own` keyword', SPEC 8966 '`own` is grammatically a keyword in parameter-declaration position' — yet it is enumerated in NONE of §002's keyword-class lists (002-3 declaration, 002-4 clause, 002-6 control-flow, 002-11 operator-context) NOR in SPEC §1.4's parallel enumeration (SPEC 125-142). No listed class contains it and no catch-all group exists, so a reserved-word table built from the enumerations omits `own`, contradicting 002-27 'every keyword is reserved'. Charitable hunt found the enumerations are class-partitioned rather than a single closed 'these are ALL keywords' list — but that does not dissolve the defect, it widens it: `public`/`private`/`shared`/`root` (003-3/003-8, also called keywords) are likewise absent, so the enumerated classes are demonstrably incomplete relative to tokens the docs themselves call keywords. Completeness/invariant gap sustained. MED.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:353-353`
    > 005-12. A trait method signature may declare any parameter with the `own` keyword; the `own` declaration is part of the trait's contract and implementations cannot strengthen or weaken it. (§3.1.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:74-74`
    > 002-3. The declaration keywords are `node`, `connection`, `trait`, `type`, `fn`, `operator`, `effect`, `signal`, `attr`, `recurrent`, `derived`, `stream`, `view`, `const`, `let`, `mut`, `repeat`, `main`, `collect`; `collect` heads the collect block expression and its `collect as x:` statement form. `fold` is a keyword too, but heads an expression form (the fold form) rather than a declaration. (§1.4)

#### F101 — Fold header `fold <members>:` admits an inline-conditional members expression whose `:`/`else:` collide with the fold's own arm colons and `else:` arm.

- **Severity/Category/Verdict:** MED / parse_ambiguity / PLAUSIBLE
- **Anchors:** LOG 035-2, 035-6, 002-26 · SPEC § 13.21, 1.4
- **Why it is a defect:** The `<members>` slot is an expression position (035-6 allows a reactive-composite expression), and 002-26 permits an inline `if c: a else: b` as an expression. So `fold if c: a else: b:` has two valid parses: (1) the members expression is `if c: a else: b`, the trailing `:` opens the fold block; (2) the first `:` opens the fold block, making members = `if c`, then `a else: b:` becomes garbage. The inline conditional's own `else:` also collides with the fold form's mandatory `else:` arm keyword. Observe deliberately requires parenthesization for exactly this open-ended-extent problem (016-249), but the fold `<members>` slot has no such parenthesization requirement, leaving the boundary undecidable.
- **Direction of change:** State how `<members>` is delimited from the fold's opening `:` — e.g. require parenthesization of any colon-bearing members expression, mirroring the observe rule (016-249).
- **Evidence check:** partial — Whether the fold <members> slot is a general expression position (admitting an inline 'if c: a else: b' whose colons collide with the fold arm colons) — the finding infers 'admits' from 035-6's 'reactive composite,' but neither 035-6 nor §13.21.4 states the slot accepts arbitrary inline conditional expressions.
- **Charity check:** sustain — The fold `<members>` slot is a value-producing expression position: 035-6 admits 'reactive composites with a uniform slot type' (not restricted to a bare identifier — the SPEC's word 'names the <members>' at line 23108 is descriptive prose, no normative restriction to identifiers), and 002-26 permits an inline `if c: a else: b` as a single-expression value (confirmed usable in value position, SPEC:6008 `let x: i32 = if condition: 5 else: unreachable()`). No parenthesization/self-delimiting rule covers the fold header: 016-249 scopes its requirement to `observe`, and §13.8.10 (SPEC:17319-17335) scopes its self-delimiting-or-parenthesize rule to whitespace-separated placements only — neither reaches `fold <members>:`. So `fold if c: a else: b:` has two live readings (members = `if c: a else: b` with trailing `:` opening the block, vs first `:` opening the block), and the inline conditional's own `else:` collides with the fold form's mandatory `else:` arm (035-2). No normative text forces one reading. The existence of the second reading IS the defect. Sustain.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:4369-4369`
    > 035-2. A fold has the block shape `fold <members>:` followed by a `by: <expr>` arm and an `else: <expr>` arm, with newline-separated arms, no comma between them, both arms mandatory, and `else` written last. (§13.21)
  - `packages/ductus-lang/docs/SPEC.md:23103-23106`
    > fold <members>:
    >   by: <fn(T, T) -> T expression>
    >   else: <T expression>
  - `packages/ductus-lang/docs/DECISION_LOG.md:97-97`
    > 002-26. A function body and `if`/`else` and `match` arms may be a single inline expression after the `:` or an indented block: `fn double(x: i32) -> i32: x * 2`. (§1.4)

#### F102 — Fold `else:` arm expression may be an inline loop-with-`else`, producing two `else:` tokens the parser cannot assign between loop-else and fold-arm.

- **Severity/Category/Verdict:** MED / parse_ambiguity / PLAUSIBLE
- **Anchors:** LOG 035-2, 014-4, 014-87, 014-90 · SPEC § 13.21, 12.6.1
- **Why it is a defect:** The fold `else:` arm takes a `<T expression>` (SPEC 13.21.1) and a loop is an expression in every position (014-4/014-5). Writing `else: for x in seed: acc else: 0.0` as the fold arm produces a second `else:` token. 014-90 fixes a loop-`else` at the loop head's indentation via dedent, but here the loop is inline after the fold `else:`, so no dedent boundary exists to tell whether the trailing `else:` is the inline loop's `else` clause or an illegal second fold arm. Because 035-2 forbids a second fold `else:` arm, one parse is a compile error and the other is a valid two-else loop; the two readings diverge and no rule selects between them.
- **Direction of change:** Forbid a bare (un-parenthesized) loop expression carrying an `else:` clause as an inline fold-arm expression, or require parenthesization of any inline `else:`-bearing expression in an arm slot.
- **Evidence check:** partial — Whether the fold else: arm admits an inline loop carrying its own else: clause (creating two else: tokens) — inferred from 014-4/014-5 loop-is-expression, but the 014-4 citation line is wrong (1743 vs actual 1643) and no rule states an inline loop-with-else is legal in the fold else: arm.
- **Charity check:** refute — The specific witness requires an INLINE loop-with-else written after the fold `else:` on one line (`else: for x in seed: acc else: 0.0`), and the finding's own argument depends on 'no dedent boundary exists'. But the corpus provides no inline (non-bracketed) form for a runtime `for`/`while` loop carrying a body and an `else:` clause: a runtime loop is a header + indented body block, and 014-90 mandates its `else:` be written 'at the loop head's indentation, dedented from the body'. The only bracketed inline `for`-expression is the compile-time array comprehension `[for i in 0..N: <expr>]`, which is self-delimited by brackets and explicitly has NO `else:`/filter form (SPEC:6750-6759). So a loop-with-else cannot be placed inline after the fold `else:` at all; wherever it legally appears (indented block form) 014-90's loop-head-indentation rule creates exactly the dedent boundary that distinguishes the loop's own `else:` from the fold arm's `else:`. 014-90 is the selecting rule the finding claims is absent — and it is cited by the finding itself, so it is consistent with the finding's quoted passages (035-2, 014-4), not in conflict — hence refute, not refile_divergence. Note: the inline-if `else:` collision that DOES construct is already captured by F101. | DECISION_LOG.md:1729 — "014-90. A loop's `else:` clause is written at the loop head's indentation, dedented from the body rather than nested inside it. (§12.6.1)"; and SPEC.md:6756-6759 — "The body is a pure 1:1 map — there is **no** `if`-filter form, because a filter would make the element count not compile-time-known" (the only inline bracketed `for`-expression carries no `else:` arm, so no inline loop-with-else form exists to produce the second `else:` token).
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:4369-4369`
    > 035-2. A fold has the block shape `fold <members>:` followed by a `by: <expr>` arm and an `else: <expr>` arm, with newline-separated arms, no comma between them, both arms mandatory, and `else` written last. (§13.21)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1743-1743`
    > 014-4. `for` and `while` loops are expressions whose value is determined by the body's `break` expressions and an optional `else:` clause. (§12.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1729-1729`
    > 014-90. A loop's `else:` clause is written at the loop head's indentation, dedented from the body rather than nested inside it. (§12.6.1)

#### F031 — 007-74 postfix list and SPEC tier 14 postfix list disagree: LOG lists `?.`/`?[]`/`?()`, SPEC omits them and instead lists cast forms.

- **Severity/Category/Verdict:** LOW / divergence / CONFIRMED
- **Anchors:** LOG 007-74 · SPEC § 4.4.7
- **Why it is a defect:** 007-74's postfix tier enumerates `?`, `?.`, `?[]`, `?()`, `.`, `[]`, `()`. SPEC §4.4.7 tier 14 lists `?`, `.`, `[]`, `()`, and the cast call forms, but omits the optional-chaining/optional-index/optional-call postfix forms `?.`/`?[]`/`?()`. The two normative precedence enumerations diverge on which postfix operators exist at this tier; SPEC must conform to LOG.
- **Direction of change:** Reconcile the postfix enumerations: add `?.`/`?[]`/`?()` to SPEC tier 14 (or, if the cast-call additions are the newer decision, amend 007-74 in the LOG first). Decision owner confirms which enumeration is current.
- **Evidence check:** pass — 007-74 postfix tier lists ?./?[]/?() but SPEC tier-14 omits them and instead lists cast forms; the two normative enumerations diverge on which postfix operators exist.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:692-692`
    > prefix `-`/`~`/`handle`/`handle!`/`portal`/`delete`; postfix `?`/`?.`/`?[]`/`?()`/`.`/`[]`/`()`; `::`. (§4.4.7)
  - `packages/ductus-lang/docs/SPEC.md:3354-3354`
    > | 14   | `?`, `.`, `[]`, `()`, and `T%()`/`T\|()`/`T?()` casts     | left           |

#### F030 — SPEC operator-precedence table (tier 13 prefix) omits `delete`, which 007-74 and 007-239 list as a prefix operator.

- **Severity/Category/Verdict:** LOW / divergence / CONFIRMED
- **Anchors:** LOG 007-74 · SPEC § 4.4.7
- **Why it is a defect:** 007-74 lists `delete` among the prefix operators at the negation/prefix precedence tier, and 007-239 confirms `delete` uses prefix-operator syntax. The SPEC §4.4.7 precedence table tier 13 omits `delete` entirely. SPEC must conform to LOG; the omission leaves `delete`'s binding strength unspecified in the normative table an implementer parses from.
- **Direction of change:** Add `delete` to SPEC §4.4.7 tier 13 (prefix) to match 007-74. SPEC-conformance fix; no LOG change.
- **Evidence check:** pass — SPEC operator-precedence tier-13 prefix row omits 'delete' which 007-74 lists as a prefix operator, leaving delete's binding strength unspecified in the normative table.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:692-692`
    > 007-74. Operator precedence, loosest to tightest: `|>`; `or`; `and`; `not`; bitwise `|`/`^`/`&`; `..`; comparison; shifts `<<`/`>>`; additive; multiplicative; prefix `-`/`~`/`handle`/`handle!`/`portal`/`delete`; postfix `?`/`?.`/`?[]`/`?()`/`.`/`[]`/`()`; `::`. (§4.4.7)
  - `packages/ductus-lang/docs/SPEC.md:3353-3353`
    > | 13   | `-`, `~`, `handle`, `handle!`, `portal` (prefix)          | right           |

#### F107 — No rule specifies how a flag run terminates against an immediately-following identifier char with no whitespace, so `Pin'!x` has no defined tokenization.

- **Severity/Category/Verdict:** LOW / gap / CONFIRMED
- **Anchors:** LOG 021-107, 021-109, 021-119, 021-130 · SPEC § 13.8.8, 13.8.8.4, 13.8.10
- **Why it is a defect:** Witness: `Pin'!x`. `'` and `!` are flags (021-113) forming a run after TypeRef `Pin`. A flag run is a maximal run of flag chars; `x` is a letter so it ends the run — but `x` is immediately adjacent with no whitespace. The bare instance-name form uses whitespace (`Pin' p1`, 021-107). Whether a letter glued to a flag run (a) starts the instance name, (b) is a lex error, or (c) was meant to be part of the type is unspecified. All examples separate the flag run from what follows by whitespace or a slot char; the zero-separator letter case is undefined.
- **Direction of change:** State the flag-run termination rule explicitly (maximal run of flag-set chars) and specify whether an immediately-adjacent identifier-start char after a flag run is a lex error or begins the next inline element.
- **Evidence check:** pass — No rule defines tokenization of a flag run immediately followed by an identifier char with no whitespace (e.g. Pin'!x).
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2846-2846`
    > 021-107. A flag is a single non-letter character adjacent to the placed type's TypeRef with no intervening whitespace, aliasing a boolean attribute of that type: `Pin' p1`. (§13.8.8)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2848-2848`
    > 021-109. Multiple flags may appear as a run after the TypeRef: `Component?* c1`. (§13.8.8)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2869-2869`
    > 021-130. A bare type, including any flag run, is self-delimiting: `C`, `G'`, `Pin'!`. (§13.8.8)

#### F106 — Layout is suspended 'inside string literals', but interpolation embeds arbitrary expressions containing brackets/nested strings inside the literal, and no rule says whether delimiter/string nesting is tracked through `{expr}`.

- **Severity/Category/Verdict:** LOW / gap / CONFIRMED
- **Anchors:** LOG 002-23, 012-42 · SPEC § 1.4, 9.1.9
- **Why it is a defect:** Witness: `"total {items[compute(\"a\")]}"`. The interpolation holds `[`, `(`, and a nested `".."` string. 002-23/§1.4 say layout suspends 'inside string literals' but say nothing about how the lexer scans an interpolation hole: whether the inner `"a"` is a nested string literal (with its own escape/layout rules), whether a `{` closes on the first `}` or tracks brace nesting, and whether a stray `}` inside the inner expression ends interpolation. The escape `\{` (012-44) covers literal braces but not the balanced-hole scan. This is under-specified rather than contradictory, but two implementers can tokenize a multi-line interpolated string differently (first-`}` vs balanced-`}`).
- **Direction of change:** State whether interpolation `{...}` scans as a balanced-brace hole re-entering full lexing (nested strings, brackets, braces tracked) or as a flat first-`}`-terminated hole; specify layout behavior inside the hole.
- **Evidence check:** pass — Layout suspends 'inside string literals' but interpolation embeds arbitrary expressions with nested brackets/strings; no rule says whether delimiter/string nesting is tracked through {expr}.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:94-94`
    > 002-23. Layout is suspended inside `(...)`/`[...]` and string literals, letting those span lines freely. (§1.4)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1237-1237`
    > 012-42. Interpolation expressions are arbitrary expressions (method calls, arithmetic, field access), not just bare identifiers: `"value: {amount * tax_rate}"`. (§9.1.9)
  - `packages/ductus-lang/docs/SPEC.md:184-184`
    > Inside paired delimiters — `(...)` and `[...]` — and inside string literals, layout is suspended, so multi-line argument lists, generic lists, tuples, and string contents may span lines freely.
  - `packages/ductus-lang/docs/SPEC.md:6510-6512`
    > Interpolation expressions are arbitrary expressions, including method
    > calls, arithmetic, and field access. They are not limited to bare
    > identifiers.

#### F104 — `when:`/`given` fallback (`otherwise:`/`default:`) and loop `else:` can dedent to the same indentation inside an arm body, with no attachment rule.

- **Severity/Category/Verdict:** LOW / parse_ambiguity / CONFIRMED
- **Anchors:** LOG 022-102, 014-90, 002-6 · SPEC § 13.9.12, 12.6.1, 1.4
- **Why it is a defect:** 002-6 separates the tokens (`else` for loop/fold, `otherwise:`/`default:` for when/given) which prevents a keyword collision — good. But the LOG still gives no indentation-attachment rule for a loop `else:` that dedents inside a `when`/`given` arm body: a `for` loop whose `else:` dedents to the arm indentation is not distinguished from the start of a sibling arm. 022-110 says every line at the arm indent is an arm, so a loop-`else:` dedented to arm indent is read as an arm header, not the loop's else. Because 014-90 mandates the loop `else:` sit at the loop head's indentation (which, for a loop that is itself the whole arm body, equals arm indent), the two readings collide. This is a witness that the dedent-based loop-`else` placement rule is under-specified relative to arm-position rules.
- **Direction of change:** State that a loop `else:` is only recognized when the loop head is nested strictly deeper than the enclosing arm/block indentation, or require loop bodies inside when/given arms to be parenthesized/indented one level below arm position.
- **Evidence check:** pass — No indentation-attachment rule distinguishes a loop else: dedented to arm indent from the start of a sibling when/given arm.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2986-2986`
    > 022-102. In a `when:` or `given` block the fallback arm — `otherwise:` for `when:`, `default:` for `given:` — must be the last arm; a non-last fallback is a compile error. (§13.9.12)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1729-1729`
    > 014-90. A loop's `else:` clause is written at the loop head's indentation, dedented from the body rather than nested inside it. (§12.6.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:77-77`
    > 002-6. The control-flow keywords are `if`, `else`, `match`, `for`, `in`, `while`, `break`, `continue`, `return`, `yield`; `yield` is legal only directly under a `collect` and contributes one member-cell. The keyword `else` has exactly two senses: the loop-`else` arm and the fold-form empty-membership `else:` arm; the fallback arms of `when:` and `
given` blocks are spelled `otherwise:` and `default:`, so no third `else` sense exists. (§1.4)

#### F103 — `yield if c: a else: b` conditional-value form is un-parenthesized; its inline `if/else` colons are indistinguishable from a `yield`-then-`if`-block parse.

- **Severity/Category/Verdict:** LOW / parse_ambiguity / CONFIRMED
- **Anchors:** LOG 034-7, 034-2, 002-26 · SPEC § 13.20, 1.4
- **Why it is a defect:** 034-7 promotes `yield if c: a else: b` as the sanctioned conditional-value form, but `yield <expr>` (034-2) followed by a value `if` also parses as `yield`-of-nothing followed by a separate structural `if` block — precisely the runtime-`if`-gating construct 034-7 forbids. Distinguishing the two readings requires the parser to already know whether the trailing `if` is a member-value expression or a structural conditional, which is the very question 034-7 resolves only semantically (compile-time-known vs runtime). The surface `yield if c: a else: b` and `yield` + `if c: (yield a) ...` are not lexically disambiguated; no rule states the `if` after `yield` is always the conditional-value operator rather than a fresh statement.
- **Direction of change:** State that `if`/`match` immediately following `yield` on the same line is always parsed as the conditional-value expression (the `yield` operand), never as a separate statement; add an example distinguishing it from a block `if` under `collect`.
- **Evidence check:** pass — 'yield if c: a else: b' not lexically disambiguated from yield + structural if.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:4358-4358`
    > 034-7. A `yield` placed under a value `if` or `match` inside a `collect` is legal only when the condition or scrutinee is compile-time-known, in which case the conditional is expanded rather than gating structure; when the condition or scrutinee is reactive or runtime-valued it is a compile error whose diagnostic offers `when:`/`given` arms for membership switching or a conditional value yield of the form `yield if c: a else: b`, since `if`/`match` never gate structure. (§13.20)
  - `packages/ductus-lang/docs/SPEC.md:23039-23039`
    >         yield a conditional value: `yield if is_active: voice else: muted`.

### Core language: numerics, collections, ownership, stale examples

Contained defects in the pre-amendment core: const wrapping arithmetic contradiction, over-width shifts undefined, mixed-type widening conflicts, usize/isize count divergence, move-operand rules, plus SPEC examples using forbidden syntax and fences that mis-lex under the language's own rules.

#### F027 — No in-scope rule defines shift behavior when the shift count >= the left operand's bit width (e.g. `1u8 << 8`); the count TYPE is pinned but out-of-range amounts are not.

- **Severity/Category/Verdict:** HIGH / gap / CONFIRMED
- **Anchors:** LOG 007-64, 007-72, 007-92, 007-116, 007-193 · SPEC § 4.4.2, 4.4.6, 4.6
- **Why it is a defect:** 007-116 promises every arithmetic operation has four overflow variants, but shifts have none and the shift-count-overflow case (count equal to or greater than the left operand's bit width, e.g. `1u8 << 8` or `1u8 << 300`) is a classic trap/mask/UB decision in every low-level language. Nothing in the scoped numerics section (007-62 through 007-73, 007-92, 007-193) or §4.4.2/§4.6 states whether such a shift traps, masks the count modulo the width, produces zero, or is a compile error. Per 001-6 the only legal stopping points are std-library delegation or explicit implementation-defined behavior; neither is stated, so an implementer is blocked with no legal boundary.
- **Direction of change:** Add a rule fixing the out-of-range-shift-count behavior (trap, mask-modulo-width, or explicit implementation-defined) and, if applicable, whether default `<<` traps when significant bits are shifted out. Decision owner must choose the policy.
- **Evidence check:** pass — No in-scope rule defines what 1u8 << 8 (shift count >= left-operand bit width) does — trap, mask, zero, or compile error — and neither std-delegation nor implementation-defined behavior is invoked as a legal boundary, leaving the implementer blocked.
- **Charity check:** sustain — Gap confirmed; no rule defines shift behavior when count >= left operand bit width. The Shl/Shr trait defs (SPEC 3892-3895) declare only `n: u32` with no out-of-range semantics. §4.4.2 (SPEC 3206-3211) and §4.4.6 footnote (3306-3308) and 007-92/007-193 pin the count TYPE (u32-convertible) but say nothing about the count VALUE exceeding the width. A full grep for shift + (overflow|trap|mask|modul|width|implementation-defined|undefined) returns nothing in either doc. 007-116 promises four overflow variants for `every arithmetic operation` but shifts have none, and shift-count-out-of-range is not a result overflow, so 007-117's trap-on-overflow does not cover it either. Per 001-6 no legal boundary (std-delegation or implementation-defined) is invoked. `1u8 << 8` is an unfilled hole. HIGH sustained.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:682-682`
    > 007-64. `<<` and `>>` dispatch via `Shl` and `Shr` and produce the same type as the left operand. (§4.4.2)
  - `packages/ductus-lang/docs/DECISION_LOG.md:710-710`
    > 007-92. The shift right operand may be any unsigned integer type narrower than or equal to `u32` (implicitly widened); other types require an explicit cast. (§4.4.6)
  - `packages/ductus-lang/docs/SPEC.md:3206-3211`
    > The right-shift operator `>>` is a single operator whose behavior depends
    > on the signedness of the left operand's type: signed types shift
    > arithmetically (sign-extending); unsigned types shift logically (zero-
    > extending). The compiler dispatches on the type via the `Shr` trait impl.
    > No separate `>>>` operator exists. `>>` has no other meaning at the value
    > level.

#### F026 — 007-137 declares wrapping/checked compile-time constant arithmetic a compile error, contradicting 007-120's definition that wrapping always yields an in-range modular result.

- **Severity/Category/Verdict:** HIGH / contradiction / CONFIRMED
- **Anchors:** LOG 007-120, 007-137 · SPEC § 4.6.2, 4.6.5
- **Why it is a defect:** Per 007-120 the wrapping operator `+%` is defined to produce a modular result that always fits the destination type: `200u8 +% 100u8` is `44u8` (300 mod 256), which fits u8. There is no overflow. 007-137 nonetheless declares this same expression a compile error 'because 300 doesn't fit u8' and applies 'regardless of operator variant'. An implementer cannot both compute the defined wrapping result (44u8) and reject the program. The same over-reach hits `+?` (which returns None/Some, never overflows) and 007-120's own example `255u8 +% 1` (a compile-time-constant expression per 001-28) which 007-120 says 'is 0u8' but 007-137 would reject. The two rules are jointly unsatisfiable for the wrapping and checked variants.
- **Direction of change:** Restrict 007-137 / §4.6.5 to the trapping (default) variant, or restate it to check the operator's DEFINED result against the type rather than the pre-wrap mathematical value; reconcile with 007-120 so wrapping/checked constant expressions that produce in-range results are accepted. Decision owner must choose.
- **Evidence check:** pass — 007-120 defines wrapping as always yielding an in-range modular result; 007-137/SPEC:3585 declares the same wrapping expression a compile error 'still doesn't fit' — an implementer cannot both compute 44u8 and reject the program.
- **Charity check:** sustain — Could not dissolve. SPEC §4.6.5 forces exactly the reading the finding attacks. Line 3585 rejects `const x: u8 = 200u8 +% 100u8` with the rationale `still doesn't fit` — the fitting check provably runs on the PRE-wrap value (300), not the wrapping result (44u8) that 007-120 defines. Line 3589-3592 says `This applies to +%, +|, +?` and that `the compile-time analysis happens before the runtime semantics of each variant matters`. For the checked variant this is unsatisfiable: 007-132 defines `+?` to return Option[T] (Some/None, never an overflow), so `200u8 +? 100u8` yields None:Option[u8], an in-range well-defined value, yet §4.6.5 forbids const-folding it as overflow. 007-120's own definitional example `255u8 +% 1 is 0u8` is a compile-time constant (001-28) whose inferred type is u8; 007-137's fitting check on the pre-wrap value 256 rejects it, so the value assertion `is 0u8` cannot hold. No charitable reading reconciles a defined-modular-result operator (007-120) and a checked-returns-Option operator (007-132) with a pre-wrap-fitting rejection that explicitly covers `+%`/`+|`/`+?`. HIGH sustained.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:738-738`
    > 007-120. Wrapping operators `+%`, `-%`, `*%`, `\%`, `%%`, and unary `-%` perform modular two's-complement arithmetic: `255u8 +% 1 is 0u8`. (§4.6.2)
  - `packages/ductus-lang/docs/DECISION_LOG.md:755-755`
    > 007-137. Compile-time constant overflow is a compile error regardless of operator variant: `const x: u8 = 200u8 +% 100u8` ✗. (§4.6.5)
  - `packages/ductus-lang/docs/SPEC.md:3584-3585`
    > const x: u8 = 200u8 + 100u8                 // compile error: 300 doesn't fit u8
    > const x: u8 = 200u8 +% 100u8                // compile error: still doesn't fit

#### F258 — Philosophy states absolutely 'There is no shared mutable state' (001-22), yet the runtime it governs is defined as a mutable store of host-written cells (027-3/027-6); no 001 entry scopes the claim to source-level values.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 001-22, 027-3, 027-6 · SPEC § 1.3, 13.14
- **Why it is a defect:** 001-22 is unqualified. Section 001's only relief is 001-15 ('Time-varying external behavior is expressed through the reactive system, not through assignment'), which governs HOW time-varying state is expressed, not whether the reactive store is shared mutable state. A reader taking 001-22 literally against 027-3 (a mutable store the host writes) gets two irreconcilable readings: (a) 001-22 is absolute, so the reactive store violates it; (b) 001-22 is implicitly scoped to source-level bindings, so no conflict. The docs never state the scope. §13.2.1 (SPEC 11564) confirms the intended reading — signals are 'writable' cells with no source assignment — but that carve-out lives only in SPEC, not in any 001 entry that would bound 001-22.
- **Direction of change:** Consider qualifying 001-22 to scope 'shared mutable state' to source-observable state, or add an atomic 001 entry naming the reactive-cell store / host boundary as the sanctioned mutable channel, so the philosophy's absolute does not collide with the runtime interface it governs.
- **Evidence check:** pass — 001-22 states 'no shared mutable state' unqualified while 027-3/027-6 define a mutable reactive store; no 001 entry scopes 001-22 to source-level bindings. Verified: neither 001-15 nor any 001 entry bounds it; the carve-out is SPEC-only (§13.2.1). Two irreconcilable readings for a literal reader.
- **Charity check:** refile_divergence — The dissolving text I hunted for (a LOG entry scoping 001-22) exists as 032-125, but it CONFLICTS with 001-22 rather than dissolving it, so per the category rule this refiles as a LOG-internal divergence, not a refute. 001-22 (DECISION_LOG.md:52) states absolutely 'There is no shared mutable state.' 032-125 (DECISION_LOG.md:4055) states 'no shared mutable state exists outside reactive cells, which the runtime coordinates' — an explicit carve-out that ASSERTS shared mutable state exists (inside reactive cells the runtime coordinates). The two entries make jointly contradictory claims about whether shared mutable state exists at all: 001-22 says none exists; 032-125 says some exists (in coordinated reactive cells). The SPEC mirrors both unreconciled — §1.3 (SPEC.md:65) restates the absolute 'no shared mutable state' and §14.6.4 (SPEC.md:23654) restates the scoped version. This is stronger than the filed MED ambiguity: it is a concrete LOG-vs-LOG divergence between 001-22 and 032-125, with neither entry acknowledging the other. Note the finding's premise 'no 001 entry scopes the claim' is technically correct — the scoping lives in §14 (032-125), not §1.3 — but the scoping's existence does not fix 001-22; it contradicts it. | DECISION_LOG.md:52 `001-22. There is no shared mutable state. (§1.3)` CONFLICTS WITH DECISION_LOG.md:4055 `032-125. Behaviors are thread-safe by construction: no shared mutable state exists outside reactive cells, which the runtime coordinates. (§14.6.4)` — the second entry asserts shared mutable state exists inside runtime-coordinated reactive cells, directly negating the first entry's absolute 'there is no shared mutable state.'
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:52-52`
    > 001-22. There is no shared mutable state. (§1.3)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3181-3181`
    > 027-3. A runtime is a transactional store of reactive cells with snapshot isolation. (§13.14)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3184-3184`
    > 027-6. The core verbs `write`/`push` stage external input into the runtime. (§13.14)
  - `packages/ductus-lang/docs/SPEC.md:11564-11567`
    > A `signal` declares a writable reactive cell. The initial value is
    > supplied at the declaration. After construction, the value is written
    > only through the host API (§13.14.2); Ductus source has no
    > syntactic form for assigning to a signal.

#### F126 — §13.2.5.2 example reads a const named `type` (Log::type / Delay::type / T::type) but the node bodies declare `const kind`, and no `type` const exists.

- **Severity/Category/Verdict:** MED / stale_example / CONFIRMED
- **Anchors:** LOG — · SPEC § 13.2.5.2
- **Why it is a defect:** The same subsection defines type-level const access as `<TypeName>::<const>` and its own node bodies declare the const as `kind`. The example uses `Log::type`, `Delay::type`, and `T::type`, reading a const named `type` that is never declared. An implementer following the example literally reads a nonexistent const; the correct form is `Log::kind` / `Delay::kind` / `T::kind`. Internal contradiction between the example and its surrounding section.
- **Direction of change:** Reconcile the const name between the node declarations and the type-level access example (use one consistent name, `kind`, in all three access sites) so the example reads the const that is actually declared.
- **Evidence check:** pass — The §13.2.5.2 example reads Log::type/Delay::type/T::type, but the section's own node bodies declare the const as 'kind' and no 'type' const exists — a self-contained internal contradiction; correct form is Log::kind/Delay::kind/T::kind.
- **Charity check:** sustain — Confirmed internal contradiction, and it is broader than the finding states — it also lives in the LOG. The §13.2.5.2 node bodies declare the const as `kind` with value `"@action/log"`/`"@action/delay"` (SPEC:12193, 12198), but the type-level access example reads `Log::type`, `Delay::type`, `T::type` (SPEC:12239-12243), reading a const named `type` that is never declared — correct form is `Log::kind`/`Delay::kind`/`T::kind`. I searched for a synthesized/implicit `type` const on node types: none exists; the corpus never declares `const type`, and `type` is a reserved body keyword (002-25), so `Log::type` reads a keyword-named nonexistent const. I checked whether any passage dissolves it: instead 016-124 ('`<TypeName>::<const>`: `Log::type`') and 016-125 ('`T::type` inside `fn tag_for[T: Action]`') in the LOG carry the SAME error and point their (§13.2.5.2) ref at this exact subsection, while the LOG's own const example uniformly names the const `kind` (005-55 and 016-114: `const kind: string = "@action/log"`). The LOG and SPEC AGREE (both write `::type`, both declare `kind`), so this is not a LOG↔SPEC divergence (not refile_divergence) — it is a shared internal contradiction between the example's read (`::type`) and every declaration (`kind`). The finding's stated defect holds. Sustain.
- **Evidence:**
  - `packages/ductus-lang/docs/SPEC.md:12191-12199`
    > node Log:
    >   satisfies Action
    >   const kind: string = "@action/log"
    >   default attr message: string
    > 
    > node Delay:
    >   satisfies Action
    >   const kind: string = "@action/delay"
    >   default attr time: duration
  - `packages/ductus-lang/docs/SPEC.md:12232-12233`
    > - **Type-level (`<TypeName>::<const>`)** — direct type-level access
    >   without an instance. Useful for compile-time discriminators and
  - `packages/ductus-lang/docs/SPEC.md:12239-12243`
    > const ACTION_LOG_TAG: string = Log::type        // "@action/log"
    > const ACTION_DELAY_TAG: string = Delay::type    // "@action/delay"
    > 
    > fn tag_for[T: Action](_: T) -> string:
    >   T::type           // type-level read; no instance needed at runtime

#### F122 — SPEC example `mut v = Vec::new()` uses the forbidden type-qualified free-function call form (LOG 011-55, §8.7).

- **Severity/Category/Verdict:** MED / stale_example / CONFIRMED
- **Anchors:** LOG 011-55 · SPEC § 11.11.2
- **Why it is a defect:** Same defect class as the §11.10.6 case: `Vec::new()` is a type-qualified constructor call that LOG 011-55 forbids and that SPEC's §9.4 prose calls unsanctioned. This is a separate fence under §11.11.2's in-place-mutation example, so it needs its own correction.
- **Direction of change:** Replace `Vec::new()` with the sanctioned module-path or method construction surface per §8.7 / LOG 011-55, or surface to the user the intended Vec constructor form.
- **Evidence check:** pass — SPEC §11.11.2 in-place-mutation example writes `mut v = Vec::new()`, same forbidden type-qualified free-function form (011-55); separate fence from F121 needing its own fix.
- **Charity check:** sustain — SPEC.md:9741 `mut v = Vec::new()` is the same forbidden type-qualified constructor form as F121, in a separate §11.11.2 in-place-mutation fence, needing its own correction. Same dissolving check applied and failed: 011-55 (DECISION_LOG.md:1166) forbids the type-qualified free-function form; 005-129 covers only instance/trait-method type-side dispatch, not a zero-receiver `new()` constructor; SPEC's own §8.7 and §9.4.1.4 prose call `Type::fn()` unsanctioned and require a module path. Sustain.
- **Evidence:**
  - `packages/ductus-lang/docs/SPEC.md:9741-9741`
    > mut v = Vec::new()
  - `packages/ductus-lang/docs/DECISION_LOG.md:1166-1166`
    > 011-55. No type-qualified free-function form exists: `Option::unwrap(option)` ✗. (§8.7)

#### F121 — SPEC example writes `Vec::new()`, a type-qualified free-function call the language forbids (LOG 011-55, §8.7); SPEC's own prose elsewhere calls this syntax unsanctioned.

- **Severity/Category/Verdict:** MED / stale_example / CONFIRMED
- **Anchors:** LOG 011-55 · SPEC § 11.10.6
- **Why it is a defect:** `Vec::new()` is a type-qualified free-function (constructor) call. LOG 011-55 says no such form exists; SPEC's own §9.4.1.4/§9.4.2.2 prose explicitly declares `Type::fn(x)` unsanctioned and requires a module path (`std::vec::new(...)`) or method form. The example demonstrates syntax the document elsewhere forbids, so an implementer copying it emits illegal source.
- **Direction of change:** Rewrite the example initializer to a sanctioned surface form (module-path free function or method form) consistent with §8.7 / LOG 011-55, or flag to the user which constructor surface Vec is meant to use.
- **Evidence check:** pass — SPEC §11.10.6 example writes `Vec::new()`, a type-qualified free-function call form that 011-55 forbids and SPEC's own §8.7/§9.4 prose calls unsanctioned — stale/illegal example.
- **Charity check:** sustain — SPEC.md:9605 `Vec[dyn fn(Event) -> ()] = Vec::new()` is a type-qualified constructor call (`Type::fn()`, no receiver). 011-55 (DECISION_LOG.md:1166) 'No type-qualified free-function form exists' and §8.7 (SPEC.md:6271-6273) 'There is no `Option::unwrap(option)` (type-qualified) form: free functions live in modules … not associated with types' forbid exactly this shape; §9.4.1.4/§9.4.2.2 (SPEC.md:7167-7169, 7224-7226) declare `Type::fn(x)` unsanctioned and prescribe the module path (`std::vec::…`). Charitable check: 005-129 (`Type::f(x)` type-side dispatch) does NOT save it — that is instance/trait-method dispatch on a receiver, whereas `new()` is a zero-receiver constructor, the very pattern §9.4.1.4 calls unsanctioned; §8.3.6/§9.3.6 route Vec construction through stdlib module paths (`std::vec::Vec`, SPEC.md:7644). Stale example. Sustain.
- **Evidence:**
  - `packages/ductus-lang/docs/SPEC.md:9605-9605`
    > mut handlers: Vec[dyn fn(Event) -> ()] = Vec::new()   // heterogeneous, erased
  - `packages/ductus-lang/docs/DECISION_LOG.md:1166-1166`
    > 011-55. No type-qualified free-function form exists: `Option::unwrap(option)` ✗. (§8.7)
  - `packages/ductus-lang/docs/SPEC.md:7223-7226`
    > The `instant::now` rendering above is illustrative of a stdlib constructor;
    > it is **not** a sanctioned type-qualified call syntax (Ductus has no
    > `Type::fn(x)` free-function namespace, §8.7). The real surface form is a
    > module path (e.g. `std::instant::now()`, §10.2.3) or a method form.

#### F005 — Name-collision on import is ruled only for glob-vs-glob (003-47); glob-vs-selective-import and glob-vs-local-declaration collisions are uncovered by any in-scope rule, with no legal boundary.

- **Severity/Category/Verdict:** MED / gap / CONFIRMED
- **Anchors:** LOG 003-46, 003-47, 003-48 · SPEC § 10.4.1
- **Why it is a defect:** 003-47 (and its §10.4.1 elaboration) only names the glob-vs-glob collision case. 003-48 permits mixing a selective named import and a glob in one `use`. An implementer must decide what happens when (a) a named/selective import brings `Foo` and a separate glob also brings `Foo`, or (b) a glob brings `Foo` that collides with a same-file local declaration named `Foo`. No 003 entry in scope covers either, and neither std-delegation nor implementation-defined behavior is stated as the boundary (per 001-6 the only legal stopping points). Result: undefined acceptance/shadowing behavior for a case any real importer hits.
- **Direction of change:** Add a 003 rule fixing precedence/collision behavior for named-vs-glob imports and for glob-vs-local-declaration shadowing, then elaborate it in §10.4.1; or explicitly mark the boundary if it is meant to be std/impl-defined.
- **Evidence check:** pass — 003-47 rules only glob-vs-glob collisions; 003-48 enables glob-vs-selective mixing, and glob-vs-local-declaration collisions exist, yet no in-scope 003 entry covers either and neither std-delegation nor implementation-defined boundary (001-6) is stated. Undefined acceptance/shadowing for a common importer case. In-scope evidence confirms the absence.
- **Charity check:** sustain — Hunted for any rule covering the two named collision cases. Found adjacent rules that do NOT reach them: 003-47 (DECISION_LOG.md:150) covers only glob-vs-glob; 009-74 (DECISION_LOG.md:1011) covers only two-enum-variant globs; SPEC §6.2.3 (SPEC.md:5238-5252) is stated for 'two enums'/'two glob imports'; 005-127 (DECISION_LOG.md:468) resolves cross-module conflicts at import time but is scoped to free-function names and names no outcome for selective-vs-glob. For case (a) selective import Foo + glob Foo: no rule states whether it errors or the explicit import wins — the corpus resolves many precedences carefully (005-124 impl-vs-free-fn) but leaves this one unstated. For case (b) glob Foo + same-file local declaration Foo: §13.7 name resolution (SPEC.md:16154 'nearest enclosing scope', §13.7.1 SPEC.md:16179 lists module top-level scope) never ranks a glob-imported name against a locally-declared name in the same module scope; standard 'local shadows glob' is a plausible inference but NO normative text forces it. Per the gap standard, inference/'obviously intended' is a SUSTAIN. 003-48 (DECISION_LOG.md:151) permits mixing selective + glob in one use, making case (a) reachable. Neither std-delegation nor implementation-defined behavior is declared as the boundary (001-6). MED gap.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:150-150`
    > 003-47. Two glob imports bringing colliding names into the same scope are a compile error at the `use` site that introduces the second collision. (§10.4.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:151-151`
    > 003-48. A `use` selection-list item may be a multi-segment path, carry its own `as` alias, or be a `*` glob: `use root::audio::(synth::Oscillator, Filter as Filt, fx::*)`. (§10.4.1)
  - `packages/ductus-lang/docs/SPEC.md:7777-7779`
    > Glob imports are subject to the import-time conflict rules per §6.2.3:
    > two glob imports that bring colliding names into the same scope produce
    > a compile error at the `use` site that introduces the second collision.

#### F002 — LOG 004-4 specifies a stream/.changes use site and a ring[1024] default policy; the cited SPEC 2.1.2 use-site list omits both entirely.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 004-4 · SPEC § 2.1.2
- **Why it is a defect:** SPEC must conform to LOG. LOG 004-4's cited target is §2.1.2, but §2.1.2 does not elaborate the stream/.changes use site, the '.changes materializes as declared stream backed by one buffer' rule, nor the 'absent any such site defaults to ring[1024]' default — a behavior-determining default that has no elaboration anywhere in chapter 2 (grep of 194-1147 for '.changes' returns nothing). The pointer contract is broken and a defaulting rule that changes concrete program behavior is unspecified in the section that is supposed to carry it.
- **Direction of change:** Either add the stream/.changes use site and the ring[1024] default to SPEC §2.1.2 to match 004-4, or repoint 004-4's (SECTION-REF) to the section that actually elaborates the stream .changes materialization and default policy.
- **Evidence check:** pass — LOG 004-4's cited SPEC §2.1.2 use-site list omits the stream/.changes use site and the ring[1024] default, a behavior-determining default with no elaboration; broken pointer + unspecified default.
- **Charity check:** sustain — LOG 004-4 cites §2.1.2 as its elaboration target and specifies (a) a consuming `stream` declaration as a use site pinning a `.changes` policy/capacity, (b) '.changes materializes as the declared stream, backed by one buffer', and (c) the behavior-determining default 'absent any such site the policy defaults to ring[1024]'. Fresh grep of the entire §2.1.2 region (SPEC 194-1200) returns zero hits for '.changes', 'ring[1024]', or the stream-consuming use-site text; the §2.1.2 use-site list (SPEC 228-236) enumerates five ordinary use sites and stops. The LOG→SPEC pointer is broken and a defaulting rule that changes concrete program behavior is elaborated nowhere in the cited section. No dissolving passage found in chapter 2.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:187-187`
    > 004-4. Use sites include explicit type annotations, arguments to concretely-typed parameters, operands paired with concretely-typed operands, assignments to concretely-typed fields, returns from concretely-typed functions, and a consuming `stream` declaration that pins a `.changes` policy and capacity — the `.changes` member materializes as the declared stream, backed by one buffer; a parameter or return type likewise pins it; absent any such site the policy defaults to `ring[1024]`. (§2.1.2)
  - `packages/ductus-lang/docs/SPEC.md:228-236`
    > type. Use sites include:
    > 
    > - An explicit type annotation (`let y: i32 = x`). The annotation resolves
    >   only a *bare* placeholder-bearing right-hand side; against a *compound* RHS
    >   it is checked, not flowed inward (see the forward-only rule below).
    > - An argument to a function parameter with a concrete type.
    > - An operand to an operator whose other operand is concretely typed.
    > - A field assignment in a record where the field type is concrete.
    > - A return value of a function with a concrete return type.

#### F001 — SPEC 2.4.1.3 lists 'function references' as const-ineligible, but LOG 004-82 says a declared fn reference IS const-eligible and does not disqualify a containing type — opposite behavior.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 004-82 · SPEC § 2.4.1.3
- **Why it is a defect:** LOG (decision-of-record) explicitly carves out that a declared `fn` reference is const-eligible and does not disqualify a containing type. SPEC contradicts this by naming 'function references' in the const-ineligible list. An implementer reading SPEC rejects a `const` record holding a `fn` reference; reading LOG accepts it. Two opposite concrete behaviors from the two documents SPEC is required to conform to.
- **Direction of change:** SPEC must conform to LOG: the ineligibility bullet should scope to anonymous/capturing closures that hold captured runtime state, and must not list a declared `fn` reference as disqualifying.
- **Evidence check:** pass — LOG 004-82 makes a declared fn reference const-eligible; SPEC §2.4.1.3 lists 'function references' as const-ineligible — opposite acceptance of a const record holding a fn reference.
- **Charity check:** sustain — LOG 004-82 explicitly carves out 'a declared `fn` reference is const-eligible and does not disqualify a containing type.' SPEC §2.4.1.3 has no counterpart carve-out; its const-ineligible bullet (SPEC 728) reads 'Types containing function references or closures with captured runtime state.' The modifier 'with captured runtime state' has ambiguous attachment: distributing it over both nouns reconciles with LOG, but the bullet sits in a flat category list ('Signal-bearing or reactive types.', '`dyn` trait objects.') whose other items are unqualified category heads, so the most natural reading takes 'function references' as its own const-ineligible category — rejecting a `const` record holding a plain `fn` reference that LOG accepts. Neither reading is forced, and LOG's explicit carve-out has no clear SPEC expression; an implementer reading the SPEC bullet as a category head diverges from LOG. Sustained as a LOG-SPEC divergence: the SPEC bullet fails to encode 004-82's carve-out unambiguously.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:265-265`
    > 004-82. Heap-allocated stdlib collections (`Vec`), the language-level `Map[K, V]`, signal-bearing or reactive types, types containing anonymous or capturing closures that hold captured runtime state, and `dyn` trait objects are not `const`-eligible; a declared `fn` reference is const-eligible and does not disqualify a containing type. (§2.4.1.3)
  - `packages/ductus-lang/docs/SPEC.md:728-728`
    > - Types containing function references or closures with captured runtime state.

#### F020 — LOG 012-101 (the §9.5 key-bounds authority) omits `duration` and `instant` from valid Map key types, but SPEC 9.5.3 lists them as qualifying keys.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 012-101 · SPEC § 9.5.3
- **Why it is a defect:** SPEC must conform to LOG. SPEC enumerates `duration` and `instant` as valid map keys; LOG 012-101 enumerates only integers/bool/char/string. SPEC also broadens the integer wording to include u128/i128/usize/isize which 012-101's parenthetical `(integers)` leaves vague. 007-236 backs the SPEC (duration/instant auto-implement Hash), so the SPEC and 007 agree and 012-101 is the under-specified side — a program using `Map[instant, V]` is legal per SPEC/007 but not licensed by the §9.5 decision-of-record.
- **Direction of change:** Bring 012-101 into agreement with SPEC 9.5.3 and 007-236 on the qualifying key set (integer-width coverage plus duration/instant); owner decides the canonical list.
- **Evidence check:** pass — SPEC 9.5.3 enumerates duration/instant (and all integer widths) as valid Map keys; LOG 012-101 (the §9.5 decision-of-record) enumerates only 'integers/bool/char/string'. SPEC must conform to LOG; the LOG side is under-specified and does not license Map[instant,V].
- **Charity check:** sustain — LOG 012-101 (§9.5 key-bounds authority, line 1296) enumerates only 'Built-in numerics (integers), bool, char, and string' as valid Map key types. SPEC 9.5.3 (lines 7311-7318) additionally lists 'duration and instant' and expands integers to explicitly include i128/u128/isize/usize. I hunted the whole corpus for a §9.5 LOG entry admitting duration/instant as keys: none exists (012-98..012-121 cover Map, none add them). 007-236 backing Hash for duration/instant does not license the §9.5 key set — that is exactly the divergence: SPEC/007 admit them, the §9.5 decision-of-record does not. SPEC must conform to LOG; the LOG side is under-specified. Sustained as MED divergence.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1296-1296`
    > 012-101. Keys must satisfy `K: Eq + Hash`. Built-in numerics (integers), `bool`, `char`, and `string` qualify; floats (`f32`/`f64`) do not, so `Map[f32, V]` is a compile error at the `Hash` bound. (§9.5)
  - `packages/ductus-lang/docs/SPEC.md:7311-7318`
    > Keys must satisfy `K: Eq + Hash`. The Ductus primitive types that
    > qualify are:
    > 
    > - All integer types (`i8`..`i128`, `u8`..`u128`, `isize`, `usize`).
    > - `bool`.
    > - `char`.
    > - `string`.
    > - `duration` and `instant`.

#### F019 — LOG 012-111 and 012-120 mandate composite-map slot paths spelled with brackets `<binding>['<key>']`, but SPEC 9.5.12 spells them with a dot `<binding>.<key>`.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 012-111, 012-120 · SPEC § 9.5.12
- **Why it is a defect:** SPEC must conform to LOG. LOG pins the concrete surface used to address a composite map slot as the bracket form `<binding>['<key>']` (twice, in 012-111 and 012-120). SPEC 9.5.12 states the dot form `<binding>.<key>`. An implementer building the reactive-composite path grammar cannot satisfy both; the two docs prescribe different program-visible syntax for the same construct.
- **Direction of change:** Reconcile the composite-map slot-path syntax between LOG (bracket `['<key>']`) and SPEC 9.5.12 (dot `.<key>`) so both name the same surface; this is a design/terminology decision for the owner, not an auto-fix.
- **Evidence check:** pass — LOG pins composite-map slot paths as bracket form `<binding>['<key>']` (twice: 012-111, 012-120); SPEC 9.5.12 spells them dot form `<binding>.<key>`. Program-visible syntax divergence; SPEC must conform to LOG.
- **Charity check:** sustain — LOG 012-111 (line 1306) and 012-120 (line 1315) both spell the const-keyed composite map slot path as the bracket form `<binding>['<key>']`. SPEC 9.5.12 (line 7451) spells the same path as the dot form `<binding>.<key>` ('parallel to record fields'). Same construct (reactive composite over a literal-key map), two different program-visible path syntaxes. I found no reconciling text — SPEC never offers the bracket form and LOG never offers the dot form for this construct. SPEC must conform to LOG; divergence in program-visible surface syntax. Sustained as MED.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1306-1306`
    > 012-111. A reactive composite over a map literal exists only for **literal-key maps**: `{ 'a': sig, 'b': 5 }` in a reactive declaration (`derived`/`attr`/`recurrent`) is a per-slot reactive composite, like a tuple or fixed-array literal mixing reactive and static elements — each slot independently reactive, with composite paths under `<binding>['<key>']` (the bracket-colon form for const-keyed map paths, paralleling `<binding>[<index>]` for arrays).
  - `packages/ductus-lang/docs/DECISION_LOG.md:1315-1315`
    > 012-120. Bracket-colon syntax `{ ['<key>']: <value>, … }` is the const-keyed alternative to colon syntax `{ '<key>': <value>, … }` for reactive composite map literals with compile-time-known keys; both forms produce identical compile-time-known slot paths under `<binding>['<key>']`. (§9.5)
  - `packages/ductus-lang/docs/SPEC.md:7451-7453`
    > The map's reactive-composite paths are `<binding>.<key>` (parallel to
    > record fields, §13.2.9.4). Keys must be compile-time-known for the
    > slot path to be static.

#### F074 — Repeat-form comprehension [for N: v] 'produces N copies of v' without stating a Clone bound or evaluation count, conflicting with the explicit-clone-at-every-duplication-site rule.

- **Severity/Category/Verdict:** MED / gap / CONFIRMED
- **Anchors:** LOG 012-82 · SPEC § 9.3.1
- **Why it is a defect:** The repeat form silently duplicates `v` N times. Three things are unpinned: (1) whether `v`'s type must satisfy Clone — if `v` is an owned non-Clone value, N copies are impossible; (2) whether `v` is evaluated once then cloned, or evaluated N times (side-effect-visible for `[for 3: next_id()]`); (3) N=0 — is `v` evaluated at all. This implicit N-way duplication also tensions with SPEC line 8738 requiring an explicit `.clone()` at every duplication site — the repeat form is a duplication site with no explicit clone. An implementer cannot write the typecheck or codegen without deciding all three, and no legal boundary is declared.
- **Direction of change:** Specify in 012-82/§9.3.1: whether `[for N: v]` requires `T: Clone`, whether `v` is evaluated exactly once (then cloned N-1 times) or N times, and the N=0 evaluation rule — and reconcile with the explicit-`.clone()`-per-duplication-site rule.
- **Evidence check:** pass — The repeat comprehension [for N: v] silently duplicates v N times without stating a Clone bound, an evaluation count (once-then-clone vs N evaluations, side-effect-visible), or N=0 behavior, and it tensions with the rule (SPEC §... @8738) that every duplication site needs an explicit .clone(); an implementer cannot write the typecheck/codegen without deciding these.
- **Charity check:** sustain — 012-82 (line 1277) / SPEC §9.3.1 (lines 6766-6773): `[for N: v]` 'produces N copies of v'. Three things unpinned: (1) whether v's type must satisfy Clone — the SPEC example `[for n: origin]` copies a Point n times with no Clone bound stated; (2) evaluate-once-then-copy vs evaluate-N-times (side-effect count for `[for 3: next_id()]`); (3) N=0 evaluation. SPEC line 8738 requires an explicit `.clone()` at every duplication site, yet this repeat form is a silent N-way duplication site with no explicit clone — a direct tension. I found no rule resolving evaluation count or the Clone bound and no legal boundary. Gap sustained as MED.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1277-1277`
    > 012-82. The array comprehension's degenerate form `[for N: v]` — a bare compile-time-known count `N`, with no binding and no `in` — produces `N` copies of `v`, of type `T[N]`. (§9.3.1)
  - `packages/ductus-lang/docs/SPEC.md:6766-6768`
    > **Repeat form.** The degenerate comprehension `[for N: v]` — a bare
    > compile-time count with no binding and no `in` — produces `N` copies of `v`,
    > of type `T[N]`:
  - `packages/ductus-lang/docs/SPEC.md:8738-8738`
    > `Clone` requires an explicit `.clone()` call at every duplication site.

#### F073 — s.slice(start,end) and s.byte_slice(start,end) behavior when start > end (both in range) is undefined: not a listed trap cause, not declared empty, not implementation-defined.

- **Severity/Category/Verdict:** MED / gap / CONFIRMED
- **Anchors:** LOG 012-31, 012-32, 012-35 · SPEC § 9.1.7
- **Why it is a defect:** 012-35 and SPEC §9.1.7 enumerate exactly two invalid-boundary causes: mid-codepoint byte index, and out-of-range position. A call like `s.slice(5, 2)` with both indices in range is neither — it is a reversed/empty span. The corpus never says whether this returns the empty string or traps. The range operator's `start >= end` = empty rule (SPEC line 10004) governs `..` range values, not these `isize`-argument methods, and `slice` takes two positional args, not a range. An implementer must choose empty-vs-trap with no rule to consult; the two choices give different program behavior (empty string vs process abort).
- **Direction of change:** Add a rule under §9.1.7 fixing `start > end` behavior for both `slice` and `byte_slice` — either 'yields the empty string' (paralleling the range `start >= end` rule) or 'traps as an invalid boundary' — and reflect it in 012-31/012-32/012-35.
- **Evidence check:** pass — s.slice(start,end)/byte_slice with start>end and both in range is a reversed span — not a mid-codepoint index and not out-of-range — so it matches neither enumerated trap cause (012-35 / SPEC §9.1.7) and is not declared empty or implementation-defined; empty-vs-trap is an undecided, behavior-changing gap.
- **Charity check:** sustain — 012-35 (line 1230) and SPEC §9.1.7 (lines 6466-6468) enumerate exactly two invalid-boundary trap causes: mid-codepoint byte index and out-of-range position. A call `s.slice(5,2)` with both indices in range is neither — a reversed/empty span. I checked the slice method definitions (SPEC 6455-6468) for any reversed-order handling: none. The range operator `start>=end`=empty rule governs `..` range values, not these two positional isize-argument methods. So behavior is undefined: empty string vs process abort — different program behavior with no rule to consult and no legal boundary declared. Gap sustained as MED.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1226-1226`
    > 012-31. `s.slice(start: isize, end: isize) -> string` slices at validated character positions with cost O(end). (§9.1.7)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1230-1230`
    > 012-35. Invalid slice boundaries — a mid-codepoint byte index or an out-of-range position — trap at runtime. (§9.1.7)
  - `packages/ductus-lang/docs/SPEC.md:6466-6468`
    > Both methods return a new string value. Invalid boundaries
    > (mid-codepoint byte index, out-of-range positions) trap at runtime per
    > §4.6.1's trap-on-error philosophy.

#### F072 — Map Clone is derived but nothing states it preserves insertion order, though insertion order is a language-level observable (iteration/Display); clone iteration order is undefined.

- **Severity/Category/Verdict:** MED / gap / CONFIRMED
- **Anchors:** LOG 012-115, 012-107, 012-112 · SPEC § 9.5.13, 9.5
- **Why it is a defect:** Insertion order is a load-bearing language-level guarantee for iteration, Display, Debug, and `repeat` over a Map. Clone is derived, but neither 012-115 nor SPEC §9.5.13 states whether the clone preserves the source's insertion order. Eq/Hash are explicitly declared order-insensitive, which by omission leaves Clone unpinned. An implementer could produce a clone whose backing table rehashes into a different iteration order, so `for (k,v) in m` and `for (k,v) in m.clone()` (or their Display output) diverge in element order — an observable, behavior-changing difference the corpus otherwise forbids. No legal boundary (std-delegation or implementation-defined) is declared for clone order.
- **Direction of change:** State in 012-115/§9.5.13 that derived Clone on Map preserves the source's insertion order, so the clone's iteration/Display order equals the original's.
- **Evidence check:** pass — Map insertion order is a language observable (iteration/Display/repeat), but neither 012-115 nor SPEC §9.5.13 states Clone preserves the source's insertion order; a rehashing clone could yield divergent iteration order the corpus otherwise forbids, with no std-delegation or implementation-defined boundary declared.
- **Charity check:** sustain — Insertion order is a language-level observable: 012-107 (line 1302), 012-112 (line 1307), and Display/iteration/repeat all depend on it. Clone is derived (012-115, SPEC §9.5.13 line 7465). I grepped both docs for any statement that Map Clone preserves insertion order — none exists. 012-115 and §9.5.13 pin ONLY Eq/Hash as order-insensitive (line 7467-7469); by omission Clone's iteration order is unpinned, so `for (k,v) in m` vs `for (k,v) in m.clone()` (and their Display) could diverge in element order — an observable difference the corpus otherwise forbids. No legal boundary (std-delegation/implementation-defined) is declared for clone order. Gap sustained as MED.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1302-1302`
    > 012-107. Map iteration is **insertion-ordered** at the language level — keys and `(K, V)` pairs are yielded in the order their keys were first inserted, following JS map semantics: updating an existing key's value keeps that key's position, and deleting then reinserting a key appends it at the end. `HashSet[T]` is unaffected and stays unordered; only `Map` carries the insertion-order commitment. (§9.5)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1310-1310`
    > 012-115. The compiler provides `Eq`, `Hash`, `Clone`, `Display`, and `Debug` for `Map[K, V]` structurally when `K` and `V` satisfy the requested trait. `Ord` is NOT derived; any program demanding `Ord` on a `Map` is a compile error at the bound. (§9.5)
  - `packages/ductus-lang/docs/SPEC.md:7465-7469`
    > The compiler derives `Eq`, `Hash`, `Clone`, `Display`, and `Debug`
    > structurally for `Map[K, V]` whenever `K` and `V` satisfy the requested
    > trait. `Eq` and `Hash` are insensitive to insertion order: two maps are
    > equal iff they hold the same key→value set, and their hash agrees
    > accordingly.
  - `packages/ductus-lang/docs/DECISION_LOG.md:1307-1307`
    > 012-112. `Map[K, V]` implements `Iterable` and `IntoIterable`, yielding `(K, V)` pairs in insertion order — the order the keys were first inserted, with updates keeping position and delete-then-reinsert appending — the same insertion-order commitment the language makes for all Map iteration. (§9.5)

#### F071 — 012-164 says all five checked duration variants (+?,-?,*?,/?,%?) return Option[duration], but duration /? duration must return Option[f64]; contradicts 012-159 and 012-148.

- **Severity/Category/Verdict:** MED / contradiction / CONFIRMED
- **Anchors:** LOG 012-164, 012-159, 012-148 · SPEC § 9.4.1.2, 9.4.1.3
- **Why it is a defect:** 012-164 (and SPEC 9.4.1.3 line 7147-7148) flatly assign Option[duration] to `/?`, but `duration / duration` yields f64 (012-148), so `duration /? duration` must yield Option[f64]. 012-159 and SPEC line 7133 carve this out; 012-164 and SPEC line 7147 do not. Within SPEC, line 7133 and line 7147 directly disagree on the `/?` return type. An implementer coding the checked-division return type from 012-164/§9.4.1.3 alone would type `d1 /? d2` as Option[duration], which is wrong. Two careful readings yield different program types for the same expression.
- **Direction of change:** Reconcile 012-164 and SPEC §9.4.1.3 with 012-159/012-148: state that `/?` over two durations returns Option[f64] while `/?` scaling by a Numeric returns Option[duration]; make the two SPEC sentences agree.
- **Evidence check:** pass — 012-164 (and SPEC §9.4.1.3 @7147) blanket-assign Option[duration] to all checked duration variants incl '/?', but duration/duration yields f64 (012-148), so duration /? duration must yield Option[f64] per 012-159 / SPEC @7133 — the same expression gets two different types depending on which rule the implementer reads.
- **Charity check:** sustain — Direct contradiction confirmed on disk. LOG 012-164 (line 1359) flatly: checked variants +?,-?,*?,/?,%? 'return Option[duration]' — no carve-out. But 012-148 (line 1343) says `duration / duration` yields f64, and 012-159 (line 1354) carves `/?`/`%?` to Option[f64] for duration/duration. Within SPEC the same split exists: line 7147-7148 (§9.4.1.3 overflow) says all checked variants incl `/?` return Option[duration]; line 7133 (§9.4.1.2) says Option[f64] for duration/duration. An implementer coding the `/?` return type from 012-164/§9.4.1.3 alone types `d1 /? d2` as Option[duration], which is wrong for the duration/duration case. Two careful readings yield different program types for the same expression. MED behavior-changing contradiction sustained (both intra-LOG and intra-SPEC).
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1359-1359`
    > 012-164. Checked duration variants `+?`, `-?`, `*?`, `/?`, `%?` return `Option[duration]`. (§9.4.1.3)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1354-1354`
    > 012-159. Checked variants `/?` and `%?` on durations return `Option[duration]`, or `Option[f64]` for `duration / duration`. (§9.4.1.2)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1343-1343`
    > 012-148. `duration / duration` yields the ratio as `f64`. (§9.4.1.2)
  - `packages/ductus-lang/docs/SPEC.md:7146-7150`
    > Default arithmetic operators trap on overflow per §4.6.1: a `duration`
    > result that does not fit i64 nanoseconds aborts the process. Checked
    > variants (`+?`, `-?`, `*?`, `/?`, `%?`) per §4.6.4 return
    > `Option[duration]` and are recommended where saturation or failure
    > recovery is needed.
  - `packages/ductus-lang/docs/SPEC.md:7132-7134`
    > Use checked variants (`/?`, `%?`) per §4.6.4 where division-by-zero
    > recovery is needed; they return `Option[duration]` (or `Option[f64]`
    > for `duration / duration`).
  - `packages/ductus-lang/docs/DECISION_LOG.md:1373-1373`
    > 012-179. `instant * Numeric` and `instant / Numeric` are not defined. (§9.4.2.1)

#### F054 — SPEC types unified `.count` as `usize`, but the LOG commits array length/tally to `isize` and forbids implicit signed<->unsigned crossing, so SPEC's own `let n: usize = arr.count` is ill-typed.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 012-91, 012-124 · SPEC § 9.3.3, 9.3.7
- **Why it is a defect:** The LOG fixes the array length/index type at `isize` and states there is no `usize` length type (012-124, echoed by 007-7). The unified element-tally `.count` (012-91) is the length accessor, and `char_count` returns `isize` (012-30). The SPEC nonetheless binds `.count` results to `usize` in three examples. Because 012-133/007-102 forbid an implicit same-width signed↔unsigned crossing, if `.count` is `isize` per the LOG then `let n: usize = arr.count` needs an explicit `usize(...)` cast and the SPEC example is ill-typed; if instead `.count` is `usize` then 012-124 (`no usize length type`) is violated. The two documents cannot both be right, and the SPEC never states which type `.count` returns as a normative rule.
- **Direction of change:** Pin the element-tally `.count` return type in the LOG (candidate: `isize`, matching 012-124/012-30/007-7), then make every SPEC `.count` example use that type or an explicit cast; surface the isize-vs-usize choice to the owner rather than resolving it.
- **Evidence check:** pass — LOG fixes array length/tally type at isize with no usize length type (012-124) and forbids implicit signed↔unsigned crossing (012-133), yet SPEC binds .count results to usize in examples (@6987, @13587) with no cast — either the SPEC examples are ill-typed or 012-124 is violated, and SPEC never normatively pins .count's return type.
- **Charity check:** sustain — LOG 012-124 (line 1319) fixes array length type at isize with 'there is no usize length type'; 012-91 (line 1286) makes `.count` the unified element-tally accessor (the length accessor); char_count returns isize. 012-133 (line 1328) forbids implicit same-width signed<->unsigned crossing. SPEC binds `.count` results to usize in three examples (lines 6987, 6988, 13587) and — I verified via grep — NEVER states `.count`'s return type as a normative rule. If `.count` is isize per LOG, `let n: usize = arr.count` needs an explicit usize() cast and the SPEC examples are ill-typed; if `.count` is usize, 012-124 ('no usize length type') is violated. The two docs cannot both be right. MED divergence sustained.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1319-1319`
    > 012-124. The array length type is `isize` — signed and platform-sized; there is no `usize` length type. (§9.3.3)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1286-1286`
    > 012-91. `.count` is the unified element-tally accessor across arrays, slices, bundles, `Map[K, V]`, `Vec[T]`, `HashSet[T]`, `yielded T`, and dynamic views — it reports the number of elements — compile-time-known where the extent is static (`T[N]`, `T[..N]`) and runtime where it is not (`T[..]`, `Map`, `Vec`, `HashSet`, `yielded`, dynamic views). (§9.3.7)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1328-1328`
    > 012-133. A `usize` array index requires an explicit cast to `isize` (`arr[isize(n)]`); a same-width signed↔unsigned crossing is never implicit. (§9.3.4)
  - `packages/ductus-lang/docs/SPEC.md:6987-6988`
    > let n: usize = arr.count           // compile-time-known for T[N], T[..N]
    > let m: usize = dyn_slice.count     // runtime for T[..]
  - `packages/ductus-lang/docs/SPEC.md:13587-13587`
    >   derived n: usize = items.count                  // element tally (§13.2.8.1)

#### F045 — Associated-type projection notation is written two ways in ch12: 014-42 uses `::` (`Iter::Item`, `Iterable::Iter`) while 014-157 mandates `.` member-access for associated types (`T.Iter.Item`).

- **Severity/Category/Verdict:** MED / contradiction / CONFIRMED
- **Anchors:** LOG 014-42, 014-157 · SPEC § 12.3.3, 12.12.3
- **Why it is a defect:** 014-157 fixes the notation for projecting an associated type as `.` member-access (`T.Iter.Item`). 014-42 projects the same associated types with `::` (`Iter::Item`, and `Iter` = `Iterable::Iter` / `IntoIterable::Iter`). `Iterator::next` / `Iterable::iterator` (014-26/104) are trait-item/path references, a defensible separate use of `::`; but `Iter::Item` and `Iterable::Iter` in 014-42 are associated-TYPE projections, exactly the case 014-157 says must be written with `.`. Two spellings for one construct: an implementer building the grammar cannot tell whether `Iter::Item` and `Iter.Item` are the same production or two different ones, and the two in-scope entries disagree on which is canonical.
- **Direction of change:** Reconcile 014-42 to the associated-type projection notation mandated by 014-157 (or, if `::` for associated-type projection is intended to remain valid, amend 014-157 to admit both and state the disambiguation) so ch12 uses one spelling for associated-type projection.
- **Evidence check:** pass — 014-42 writes associated-type projections with `::` (`Iter::Item`, `Iterable::Iter`) while 014-157 mandates `.` member-access for associated types (`T.Iter.Item`), leaving the canonical projection notation ambiguous within ch12.
- **Charity check:** sustain — Confirmed notation inconsistency for associated-type projection, present in BOTH docs. Canonical rule: SPEC §3.1.2 (SPEC.md:1282-1291) and 014-157 (DECISION_LOG.md:1796) mandate `.` member-access for associated-type projection/constraints — `P.Item`, `T.Iter.Item`, and even Iterable's own `Iter.Source` is written with a dot (SPEC.md:1291). Yet 014-42 (DECISION_LOG.md:1681) and SPEC §12.3.3 (SPEC.md:10158-10160) project the SAME associated types with `::` — `Iter::Item`, `Iterable::Iter`, `IntoIterable::Iter`. `Iterator::next`/`Iterable::iterator` (014-26/104) are defensibly separate trait-item PATH references, but `Iter::Item`/`Iterable::Iter` are associated-TYPE projections — exactly the case §3.1.2/014-157 say must be `.`. No normative rule establishes `Trait::Assoc` as a distinct legal production for type projection; §3.1.2 writes Iterable's associated type as `Iter.Source` with a dot, contradicting `Iterable::Iter`. Two grammar spellings for one construct. Not a LOG-SPEC divergence (both spellings sit in both docs) but a genuine internal inconsistency 014-42 vs 014-157. SUSTAIN.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1681-1681`
    > 014-42. The iteration variable's type is `Iter::Item`, where `Iter` is `Iterable::Iter` under the default form and `IntoIterable::Iter` under `for own`. (§12.3.3)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1796-1796`
    > 014-157. Associated-type constraints use `.` member-access notation: `fn total[T: Iterable](source: T) -> T.Iter.Item`. (§12.12.3)

#### F042 — 013-42 defines 'consuming' as an exhaustive enumeration that omits indexed-slot storage, contradicting 013-3, 013-52, and 013-72 which all treat indexed assignment/slots as consuming owned storage.

- **Severity/Category/Verdict:** MED / contradiction / CONFIRMED
- **Anchors:** LOG 013-42, 013-3, 013-52, 013-72 · SPEC § 11.3.1, 11.1, 11.3.4, 11.3.6
- **Why it is a defect:** 013-42 opens with 'Consuming comprises' — a closed enumeration of what consuming is. Its list of storage sinks is 'record field, tuple component, or enum payload' — three slots, no indexed slot. But 013-3 says indexed assignment consumes the RHS, 013-52 says a borrow alias cannot be stored in an indexed slot (only true if indexed slots hold owned values), and 013-72 lists 'indexed slots' among the owned storage sites. An implementer taking 013-42 as the definition of consuming would conclude `arr[i] = v` does NOT consume v (leaving v's name live), while 013-3 requires it to consume v. Two entries in the same section give opposite answers for the same operation.
- **Direction of change:** Reconcile so the consuming enumeration and the storage-site enumerations list the same four structural slots (record field, tuple component, enum payload, indexed slot); decide whether 013-42's list should gain 'indexed slot' or be reworded as non-exhaustive — surface to user, do not self-resolve.
- **Evidence check:** pass — 013-42 defines consuming as a closed enumeration whose storage sinks are record field / tuple component / enum payload / reactive cell — omitting indexed-slot storage that 013-3, 013-52, and 013-72 all treat as consuming owned storage, so an implementer following 013-42's definition would not consume the RHS of `arr[i] = v`.
- **Charity check:** sustain — Confirmed LOG-internal contradiction. 013-42 (DECISION_LOG.md:1428) opens 'Consuming COMPRISES' (exhaustive) and its structural-storage sinks are exactly 'record field, tuple component, or enum payload' — indexed slot omitted. 013-3 (DECISION_LOG.md:1389) puts 'indexed assignment' in category B which 'consumes the RHS into the storage slot.' 013-52 (DECISION_LOG.md:1438) forbids a borrow alias in 'an indexed slot' (only meaningful if indexed slots hold owned/consumed values), and 013-72 (DECISION_LOG.md:1458) lists 'indexed slots' among owned storage sites. So 013-42's exhaustive 'comprises' excludes `arr[i] = v` from consuming, while 013-3/52/72 require it to consume. An implementer using 013-42 as the consuming definition leaves the RHS name live after `arr[i] = v`; 013-3 kills it. Charitable check: 'comprises' is standardly exhaustive; SPEC §11.3.1 (SPEC.md:8315) hedges with 'includes' (open list) and ALSO omits indexed slot, so the SPEC is not self-contradictory — but the LOG's 'comprises' is, and its own category-B sub-list drops indexed assignment that 013-3 explicitly names. No text forces indexed slots into 013-42. SUSTAIN as a LOG-internal contradiction.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1428-1428`
    > 013-42. Consuming comprises passing to an `own` parameter with explicit call-site `move`, returning a real-owner local, storing into a record field, tuple component, or enum payload (implicit move), and writing into a reactive cell (implicit move): `consume(move r)`. (§11.3.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1389-1389`
    > 013-3. Category B (structural storage: record construction, indexed assignment, field assignment via `mut`, whole-value reassignment of `mut`, attr initialization with a value RHS at placement) consumes the RHS
 into the storage slot implicitly, with no `move` keyword. (§11.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1438-1438`
    > 013-52. A borrow-equivalent alias cannot be stored in a record field, tuple component, enum payload, or indexed slot. (§11.3.4)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1458-1458`
    > 013-72. Only storage sites (record fields, indexed slots, enum payloads, reactive cells) default to owned; every transient binding site defaults to borrow-equivalent. (§11.3.6)

#### F035 — 005-236 says duration suffixes may not be re-registered `in the same scope`, but 005-224 and SPEC §3.9.4 forbid re-registration in ANY scope — two readings permit vs forbid a different-scope re-registration.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 005-236, 005-224 · SPEC § 3.9.4, 3.9.1, 3.9.5
- **Why it is a defect:** Reading A (005-236): duration suffixes are reserved only `in the same scope` — a module in a different scope from the built-in could register `ms`/`s` for its own type. Reading B (005-224 `cannot be registered` unqualified; SPEC §3.9.4 `re-registered for another type in any scope`; §3.9.5 / 005-238 make duration suffixes globally visible): reservation is global, so `ms` can never be registered for another type anywhere. These yield different concrete compiler behavior for the identical program `@literal_suffix("ms", from_ms)` written in a module that does not import the duration built-ins. The SPEC section 005-236 cites (§3.9.4) itself uses `any scope`, so 005-236's `same scope` also diverges from its own cited SPEC text.
- **Direction of change:** Surface to user: the duration-suffix reservation scope must be stated identically in 005-224 and 005-236 (and matched to §3.9.4 `any scope`). Do not pick `same scope` vs `any scope` unilaterally.
- **Evidence check:** pass — 005-236 scopes duration-suffix reservation to 'the same scope' while 005-224 (unqualified) and SPEC §3.9.4 say 'any scope'; the two readings permit vs forbid a different-scope re-registration, differing program behavior, and 005-236 contradicts its own cited SPEC section.
- **Charity check:** refile_divergence — Real LOG-SPEC textual divergence: LOG 005-236 says duration suffixes 'may not be re-registered for another type in the same scope' while its OWN cited section SPEC §3.9.4 (line 2825) says 'in any scope'. Read atomically (LOG Invariant 2), 005-236's 'same scope' permits re-registering `ms` for another type in a DIFFERENT scope; SPEC 'any scope' (and 005-224 'cannot be registered' unqualified) forbids it everywhere. The candidate dissolver — 005-238/SPEC 2832 'duration suffixes are globally visible' — would force the forbid-everywhere behavior only by COMPOSING 005-236 with 005-238 (violating atomicity), and even then it conflicts in wording with 005-236 'same scope' and its cited SPEC §3.9.4 'any scope'. Because the passage that could dissolve the ambiguity conflicts in wording with the finding's quoted 005-236, this refiles as a divergence, not a refutation. LOG carries the weaker constraint than the SPEC section it points to. MED. | DECISION_LOG.md:577 '005-236. Built-in duration suffixes are reserved and may not be re-registered for another type in the same scope. (§3.9.4)' CONFLICTS WITH its cited section SPEC.md:2825 'Neither family may be re-registered for another type in any scope.' — 'same scope' (LOG) vs 'any scope' (SPEC) is a wording divergence in a normative constraint; 005-224 (DECISION_LOG.md:565) 'cannot be registered; doing so is a compile error' and 005-238 (DECISION_LOG.md:579) 'globally visible' both side with SPEC, leaving 005-236 the outlier.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:565-565`
    > 005-224. Reserved suffixes — every built-in numeric type name (`i8`, `i16`, `i32`, `i64`, `i128`, `u8`, `u16`, `u32`, `u64`, `u128`, `isize`, `usize`, `f32`, `f64`) and the built-in `duration` suffixes — cannot be registered; doing so is a compile error. (§3.9.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:577-578`
    > 005-236. Built-in duration suffixes are reserved and may not be re-registered for another type in the same scope. (§3.9.4)
    > 005-237. `@literal_suffix` registrations follow normal name-visibility rules: visible in the defining module and to importers. (§3.9.5)
  - `packages/ductus-lang/docs/SPEC.md:2825-2825`
    > Neither family may be re-registered for another type in any scope.

#### F029 — SPEC §4.5 says out-of-range conversions 'require explicit `as`', contradicting 007-146 and SPEC §4.7 which state `as` is not a cast but a naming keyword.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 007-145, 007-146 · SPEC § 4.5, 4.7
- **Why it is a defect:** SPEC must conform to LOG. 007-146 (and SPEC §4.7 line 3363) state `as` is NOT a cast — explicit conversion uses `T(value)` call syntax (007-145). But SPEC §4.5 line 3371-3372 says non-lossless conversions 'require explicit `as` (§4.7)', using `as` as the cast keyword. This is stale wording from before the `as`-is-naming-only decision; it contradicts both the LOG and §4.7 within the same document. A reader following §4.5 would write `x as u8`, which the language rejects.
- **Direction of change:** Update SPEC §4.5 to reference the `T()`/`T%()`/`T|()`/`T?()` explicit-cast call forms (per 007-145) instead of `as`. No LOG change needed; this is a SPEC-conformance fix.
- **Evidence check:** pass — SPEC §4.5 line 3372 tells a reader to 'require explicit as' for narrowing conversions, but 007-146 and SPEC §4.7 line 3363 say as is not a cast and explicit conversion uses T(value) call syntax — following §4.5 produces a rejected program.
- **Charity check:** sustain — Confirmed intra-document contradiction, no dissolving text. SPEC §4.5 line 3371-3372 says non-lossless conversions `require explicit as (§4.7)`, using `as` as the cast keyword. But the SAME document at §4.4.7 line 3363 says `as is not in this table: it is a naming keyword, not a value operator (§4.7); explicit conversion uses the call forms T()/T%()/T|()/T?()`, and 007-146 says `The as keyword is not a cast; it is reserved for naming`. §4.5's wording is stale from before the as-is-naming decision. A reader following §4.5 writes `x as u8`, which the language rejects. SPEC diverges from LOG (007-145/007-146) and from itself. MED sustained.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:764-764`
    > 007-146. The `as` keyword is not a cast; it is reserved for naming — placement names and import aliases. (§4.7)
  - `packages/ductus-lang/docs/SPEC.md:3371-3372`
    > precision-losing — require explicit `as` (§4.7) or `From`/`Into` (§7).
  - `packages/ductus-lang/docs/SPEC.md:3363-3363`
    > - `as` is **not** in this table: it is a naming keyword, not a value operator (§4.7); explicit conversion uses the call forms `T()`/`T%()`/`T|()`/`T?()`, which bind at the postfix tier.

#### F028 — 007-90 states unconditionally that a mixed int/float operand widens to float, contradicting 007-107 which forbids i128/u128 implicit widening to any float.

- **Severity/Category/Verdict:** MED / contradiction / CONFIRMED
- **Anchors:** LOG 007-90, 007-107, 007-115 · SPEC § 4.4.5, 4.5.2, 4.5.6
- **Why it is a defect:** 007-90 is written as a universal ('the integer widens implicitly to the float type') with no carve-out. For `i128_val + 1.0f64`, 007-90 says the i128 widens to f64, but 007-107 says i128 never widens implicitly to any float, so the operands cannot reach a common type implicitly and the expression must be a compile error requiring an explicit cast. Invariant 2 requires each entry be atomic and self-contained; 007-90 as written is not self-contained and directly contradicts 007-107. An implementer reading only 007-90 would wrongly accept `i128 + 1.0`.
- **Direction of change:** Amend 007-90 to state the exception (mixed int/float widening applies except where §4.5 forbids it — i128/u128, per 007-107), making it self-consistent with 007-107. Decision owner confirms wording.
- **Evidence check:** pass — 007-90 unconditionally widens the integer operand to float in mixed int/float expressions, but 007-107 forbids i128/u128 from ever widening implicitly to any float; an implementer reading only 007-90 wrongly accepts i128 + 1.0.
- **Charity check:** sustain — Sustained as a LOG self-containment/contradiction defect, though the SPEC resolves it. 007-90 is written as an unconditional universal (`the integer widens implicitly to the float type`) with no carve-out, and per Invariant 2 must be atomic and self-contained. 007-107 (`i128/u128 never widen implicitly to any float`) directly contradicts it for `i128_val + 1.0`. An implementer reading only 007-90 wrongly accepts `i128 + 1.0`. The SPEC does reconcile this — §4.4.5 (SPEC 3263-3268) subordinates itself: `Full widening rules ... are specified in §4.5`, and §4.5.2 (line 3412) excludes i128/u128, with line 3116-3117 confirming `i128/u128 operands: implicit widening is not permitted by §4.5`. But that deferral lives in the SPEC, not in LOG entry 007-90, and the LOG is the self-contained decision-of-record (LEARNING 8). 007-90 over-claims and omits the carve-out 007-107 supplies; no text within 007-90 makes the two entries jointly satisfiable as written. The two entries are reconcilable only because 007-107 is more specific — a divergence/atomicity defect, not an implementer-blocking hole. MED sustained.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:708-708`
    > 007-90. In an expression mixing integer and float operands, the integer widens implicitly to the float type before the operation. (§4.4.5)
  - `packages/ductus-lang/docs/DECISION_LOG.md:725-725`
    > 007-107. `i128`/`u128` never widen implicitly to any float type. (§4.5.2)
  - `packages/ductus-lang/docs/SPEC.md:3459-3462`
    > *`) with mixed-kind operands, the compiler applies the appropriate
    > widening from §4.5.1–§4.5.4 to bring operands to a common type, then
    > dispatches the operator's trait method on that type. For `/` specifically,
    > the operator's always-float result triggers integer-to-float widening even

#### F046 — LOG 013-135/013-139 restrict `move` to a bare identifier and forbid all dotted operands, but §11.8.5 (and 013-79) allow `move` on a field-access l-value (partial move).

- **Severity/Category/Verdict:** MED / divergence / PLAUSIBLE
- **Anchors:** LOG 013-135, 013-139 · SPEC § 11.8.5
- **Why it is a defect:** An implementer building the parser from LOG 013-135/013-139 alone rejects `close(move self.handle)` as a parse error, because 013-135's operand is an `l-value identifier` (bare) and 013-139 declares any dotted `move` operand a parse error with no field-path carve-out. The cited §11.8.5 and LOG 013-79 both explicitly permit the field-l-value partial move. LOG-SPEC divergence (SPEC must conform to LOG, yet here the LOG entries are the narrower/wrong ones) plus a LOG-internal contradiction between 013-135/139 and 013-79. Programs valid per SPEC are rejected per these LOG entries.
- **Direction of change:** Reconcile 013-135 and 013-139 with 013-79 and §11.8.5: the `move` operand grammar must admit a field-access l-value path rooted in an owned binding, and 013-139's 'dotted expression is a parse error' must be narrowed to method-call (non-l-value) operands only. Surface to user which document is authoritative rather than resolving unilaterally.
- **Evidence check:** partial — 013-135 restricts the `move` operand to a bare 'l-value identifier', while the referenced SPEC §11.8.5 and LOG 013-79 both permit a field-access l-value partial move (`move self.handle`), making 013-135 reject SPEC-legal programs; the finding's inclusion of 013-139 as also forbidding this is overstated since 013-139 targets method-call operands, which SPEC likewise forbids.
- **Charity check:** sustain — Confirmed LOG-internal contradiction plus LOG-SPEC divergence. SPEC §11.8.5 (SPEC.md:9164-9166) defines `move`'s operand as 'a binding identifier or a field-access path rooted in an owned binding (`self.handle`, `rec.a.b`)' and SPEC.md:9174 shows `close(move self.handle)` ✓; SPEC.md:9183-9185 forbids only method-CALL operands, stating '(a field l-value `move v.field` IS allowed)'. LOG 013-79 (DECISION_LOG.md:1465) agrees: field-access l-value partial move is legal. BUT 013-135 (DECISION_LOG.md:1521) spells the operand 'l-value identifier' (bare only), and 013-139 (DECISION_LOG.md:1525) says '`move` attached to a dotted expression is a parse error' — its example is a method call, but the rule TEXT sweeps in `move self.handle`, which is a `move` attached to a dotted expression. Per LOG Invariant 2 (atomic/self-contained, no cross-reference), 013-135/139 must stand alone; standing alone they reject `close(move self.handle)`, which SPEC §11.8.5 and 013-79 accept. No normative text forces a narrow reading of 013-139's 'dotted expression' that carves out field l-values — 013-139 carries no such carve-out. SUSTAIN. Programs valid per SPEC are rejected per 013-135/139.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1521-1521`
    > 013-135. `move <l-value identifier>` is legal only as an immediate sub-expression of a function-call argument list: `f(move x)`, `g(a, move b, c)`. (§11.8.5)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1525-1525`
    > 013-139. `move` attached to a dotted expression is a parse error; prefix parenthesization is required: `move v.method()` ✗, `(move v).method()` ✓. (§11.8.5)
  - `packages/ductus-lang/docs/SPEC.md:9164-9167`
    > **Grammar.** `move <l-value>` — where `<l-value>` is a binding identifier
    > or a field-access path rooted in an owned binding (`self.handle`,
    > `rec.a.b`) — is legal only as an immediate sub-expression of a
    > function-call argument list:
  - `packages/ductus-lang/docs/SPEC.md:9183-9185`
    > move v.method()                    // ✗ a method *call* is not an l-value
    >                                    //   (a field l-value `move v.field` IS
    >                                    //   allowed); for a moved receiver write
  - `packages/ductus-lang/docs/DECISION_LOG.md:1465-1465`
    > 013-79. The `move` keyword's operand may be a field-access l-value path rooted in an owned binding — `move value.handle`, `move rec.a.b` — not only a bare identifier; this performs a partial move (§14.7.3), consuming that field while the rest of the binding stays live

#### F004 — SPEC §10.8 adds a 'only if both declared in same module' gate for a private-trait/private-type fulfill that no 003 entry carries, and it conflicts with the pure min-visibility model of 003-72.

- **Severity/Category/Verdict:** MED / divergence / PLAUSIBLE
- **Anchors:** LOG 003-72, 003-77 · SPEC § 10.8, 10.10
- **Why it is a defect:** 003-72 defines impl visibility purely as min(trait,type) with reachability = 'wherever both are visible.' For a private trait in module A and a private type in module B (different modules of the same package, orphan-legal per 003-73/74), the min is private and the set 'where both are visible' is empty, so the pure LOG model makes the impl legal-but-callable-nowhere. SPEC §10.8 instead injects a normative precondition — 'only if both declared in same module' — that appears in no 003 entry. Under LOG the cross-module private-private fulfill is accepted (dead but legal); under SPEC it is gated on same-module co-location. Two careful readings give different program acceptance, and SPEC carries a normative rule with no 003 backing (LOG-first violation).
- **Direction of change:** Decide whether a cross-module private-private fulfill is legal-but-unreachable or rejected; encode that decision as a new/amended 003 entry, then conform §10.8's table row to it so LOG and SPEC state the same rule.
- **Evidence check:** pass — SPEC §10.8 private/private table row adds a same-module gate absent from LOG 003-72's pure min-visibility model; LOG-first divergence, program-acceptance differs.
- **Charity check:** refute — SPEC §10.8 table column is titled 'Impl visibility' and every surrounding sentence frames the row as a reachability outcome, not an acceptance precondition. 'only if both declared in same module' is exactly the reachability of private∩private under min-visibility: a private trait is visible only in its module, a private type only in its module, so both are jointly visible only when co-located — which is 003-72's model, not a new gate. The acceptance-gate reading the finding requires is not forced by any text; the forced reading is reachability, and §10.9 (LOG 003-74) explicitly declares such a cross-module private/private fulfill VALID (accepted, dead-but-legal), matching the pure LOG model. No divergence. | SPEC.md:8001-8011 table header '| Trait visibility | Type visibility | Impl visibility |' with prose 'An implementation is callable wherever both the trait and the type are visible' (7996-7997) and 'if a caller can't name both the trait and the type, the implementation is unreachable from that caller's site' (8009-8011); SPEC.md:8031-8034 'A `private` trait or type still counts as "in the current package" for orphan-rule purposes. The combination — a `fulfill` block for a private trait and a foreign type, with the implementation accessible only inside the declaring module — is rare but valid.' — 'valid' = accepted, so §10.8 states reachability not an acceptance gate, consistent with 003-72.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:175-175`
    > 003-72. An implementation's effective visibility is `min(trait_visibility, type_visibility)` under `private < shared < public`: it is callable wherever both the trait and the type are visible. (§10.8)
  - `packages/ductus-lang/docs/SPEC.md:8007-8007`
    > | `private`        | `private`       | only if both declared in same module   |
  - `packages/ductus-lang/docs/SPEC.md:8009-8012`
    > The intersection rule reflects the practical observation: if a caller
    > can't name both the trait and the type, the implementation is
    > unreachable from that caller's site regardless of any separate
    > visibility specifier on the `fulfill` block.

#### F259 — 001-11 lists 'signals' among external state that is 'always immutable', but §13.2.1 defines a signal as 'a writable reactive cell' and 027-6/027-41 have the host write signals — same subject, opposite adjectives across the two sections.

- **Severity/Category/Verdict:** LOW / vague_term / CONFIRMED
- **Anchors:** LOG 001-11, 027-6 · SPEC § 1.3, 13.2.1, 13.14
- **Why it is a defect:** 'immutable' in 001-11 means source-non-assignable (no `mut`, no assignment syntax), which §13.2.1 confirms ('Ductus source has no syntactic form for assigning to a signal'). But the word 'immutable' collides head-on with §13.2.1's 'writable reactive cell' and 027-6's host `write`. The two sections use opposite adjectives for the identical subject (a signal cell), with the reconciling scope ('immutable from source, writable from host') never stated in either LOG entry. A reader cross-referencing 001-11 and 027-6 sees a naked contradiction resolvable only by inferring the source-vs-host scope from SPEC.
- **Direction of change:** Consider narrowing 001-11's 'immutable' for signals to 'source-immutable / not source-assignable', so the philosophy word matches §13.2.1's 'writable reactive cell' and the host-write verbs in section 027.
- **Evidence check:** pass — 001-11 calls signals 'always immutable' while §13.2.1 calls a signal 'a writable reactive cell' and 027-6 has the host write signals; reconciling scope (source-immutable vs host-writable) not stated in either LOG entry.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:41-41`
    > 001-11. External state — module-level declarations, type definitions, signals, function parameters, and record fields as a property of types — is always immutable. (§1.3)
  - `packages/ductus-lang/docs/SPEC.md:11564-11566`
    > A `signal` declares a writable reactive cell. The initial value is
    > supplied at the declaration. After construction, the value is written
    > only through the host API (§13.14.2)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3184-3184`
    > 027-6. The core verbs `write`/`push` stage external input into the runtime. (§13.14)

#### F253 — 001-26 promises a user-defined `fn` returns the same output for the same inputs, but 025-15 lets a `fn` read module-level signals so its result varies over time for fixed declared arguments; 'inputs' is left undefined w.r.t. transitively-read cells.

- **Severity/Category/Verdict:** LOW / ambiguity / CONFIRMED
- **Anchors:** LOG 001-26, 001-28 · SPEC § 1.3
- **Why it is a defect:** 001-26 states referential transparency as 'same inputs produce the same outputs' from the caller's perspective, and its wording bounds observable effects by 'the declared return value' — implying the declared parameters are the inputs. 025-15 explicitly permits a `fn` body to directly read a module-level signal (`global_offset`), so two calls `shifted(base_value)` with identical `base_value` return different values across commits when the signal changed. The provenance machinery (025-9/10/15) reconciles this by folding the read cells into the CALLER's provenance and re-running the fn per emission, but 001-26 never says 'inputs' includes transitively-read reactive cells, so a reader of the normative floor (§001) cannot tell whether such a fn is conformant. This is a definitional ambiguity in the philosophy layer, largely resolved once §025 is read; hence LOW, not a hard contradiction. Noted here because §001 is the in-scope 'normative floor' and the over-promise smell is exactly the assigned panel.
- **Direction of change:** Consider tightening 001-26 to define 'inputs' as including all reactive cells in the call's provenance (per §025), or to state that a fn reading module-level reactive state remains referentially transparent only relative to that provenance. Surface to user; do not redefine 'inputs' unilaterally.
- **Evidence check:** pass — 001-26 promises 'same inputs produce the same outputs' but 025-15 lets a fn read a module-level signal so results vary for fixed declared args; 'inputs' undefined w.r.t. transitively-read cells.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:56-56`
    > 001-26. From the caller's perspective every user-defined function is referentially transparent: same inputs produce the same outputs with no externally observable side effects beyond the declared return value. (§1.3)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3112-3112`
    > 025-15. A function whose body directly reads a reactive cell passes that provenance to its return value, so callers gain the directly-read cells: `shifted(base_value)` reading signal `global_offset` yields provenance `{base_value, global_offset}`. (§13.12.2.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3109-3109`
    > 025-10. When a signal argument changes, the containing reactive expression re-evaluates and re-runs the function with the new concrete values; the function never observes the signal itself. (§13.12.2)

#### F204 — 032-137 fixes the non-yielded-element drop point 'at the point of break', but its cited §12.9.3 also covers the enclosing-function-return exit path, which 032-137 does not name; the atomic entry under-states its own cited section.

- **Severity/Category/Verdict:** LOW / divergence / CONFIRMED
- **Anchors:** LOG 032-137 · SPEC § 12.9.3, 14.7.2
- **Why it is a defect:** The cited §12.9.3 specifies two early-exit paths that leave iterator elements un-yielded: `break` AND an enclosing function return. 032-137 names only `break`. The function-return case is arguably reachable via 032-134 (end of lexical scope) / 032-136 (end of function for un-returned locals), but 032-137 presents itself as the rule for non-yielded iterator elements and silently drops one of the two exit paths its own cited section enumerates. This is a narrow LOG-vs-cited-SPEC divergence (LOG entry states less than the section it points into), not a soundness hole.
- **Direction of change:** Either broaden 032-137 to include the enclosing-function-return exit path per §12.9.3, or confirm 032-136/032-134 fully subsume it and adjust the citation — surface to user.
- **Evidence check:** pass — 032-137 omits the function-return early-exit path its cited section enumerates.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:4067-4067`
    > 032-137. A drop call is inserted at the point of `break` for non-yielded iterator elements (§12.9.3). (§14.7.2)
  - `packages/ductus-lang/docs/SPEC.md:11032-11036`
    > If the loop exits via `break` (or via an enclosing function return)
    > before exhausting the iterator, elements at positions not yet yielded
    > remain inside the iterator's internal storage. When the iterator is
    > dropped (at loop exit), the remaining elements are dropped per their
    > `Drop` semantics, and the underlying buffer is released.

#### F145 — Bare IR-module and IR-grammar fences (24437, 24493) use ';' as an IR statement separator and grammar terminal; unmarked, they would lex-error against the Ductus lexer (002-16).

- **Severity/Category/Verdict:** LOW / stale_example / CONFIRMED
- **Anchors:** LOG 002-16 · SPEC § 15.4.5, 15.4.6
- **Why it is a defect:** These fences are the Ductus IR text form and its EBNF grammar, not Ductus surface source; ';' is legitimate there. But the info string is empty, so nothing distinguishes them from Ductus source for a mechanical lexer.
- **Direction of change:** Tag the IR/grammar fences with a distinct non-Ductus info string so they are skipped by Ductus-source lex checks.
- **Evidence check:** pass — Bare IR-module and IR-grammar fences (24437/24493) use ';' as IR separator/grammar terminal; unmarked they lex-error against the Ductus lexer per 002-16.
- **Evidence:**
  - `SPEC.md:24459-24459`
    >     bb0: %0 = const.i32 2 ; %1 = mul.i32 count, %0 ; ret %1 }
  - `SPEC.md:24530-24530`
    >                 | '(' type_tag (',' type_tag)* ')' | '[' type_tag ';' INT ']'

#### F144 — Bare fences containing compiler-diagnostic 'hint:' prose (1118, 17905, 21402) include ';' inside English sentences; as untagged fences they are not valid Ductus source.

- **Severity/Category/Verdict:** LOW / stale_example / CONFIRMED
- **Anchors:** LOG 002-16 · SPEC § —
- **Why it is a defect:** These fences are compiler diagnostic output (error:/hint:), not Ductus source, but carry no info string; the ';' is English punctuation. A mechanical lexer over bare fences would flag it. Purely a tagging/classification gap, not a program-behavior defect.
- **Direction of change:** Tag diagnostic-output fences with a non-Ductus info string so they are excluded from Ductus lex checks.
- **Evidence check:** pass — Bare fences of compiler-diagnostic 'hint:' prose (1118/17905/21402) contain ';' English punctuation; as untagged fences not valid Ductus source per 002-16.
- **Evidence:**
  - `SPEC.md:1126-1126`
    >   hint: the bound `K <= N` requires K ≤ 16 here; supply a smaller K or a
  - `SPEC.md:17910-17910`
    >   hint: at most one `when:` per type; combine predicates with `and`/`or`
  - `SPEC.md:21406-21406`
    >   hint: `persist` requires a gate stream because lost writes would be incorrect;

#### F143 — Bare fence at 11523 and 19008 contain host-runtime loop/transaction pseudocode with ';' terminators; unmarked, they lex-error as Ductus (002-16).

- **Severity/Category/Verdict:** LOW / stale_example / CONFIRMED
- **Anchors:** LOG 002-16 · SPEC § 13.10.4
- **Why it is a defect:** Same class: empty info string, host-side pseudocode with ';' and '{}' that is not valid Ductus source per 002-16.
- **Direction of change:** Mark as non-Ductus (info string) or convert to prose so the ';'-terminated host code is not mis-classified as Ductus.
- **Evidence check:** pass — Bare fences at 11523/19008 hold host loop/transaction pseudocode with ';' terminators; unmarked they lex-error as Ductus per 002-16.
- **Evidence:**
  - `SPEC.md:11524-11526`
    > loop {
    >   runtime.write_signal(tick_id, next_tick_value);   // accumulate dirty bits
    >   runtime.commit();                                 // evaluate + publish snapshot
  - `SPEC.md:19010-19011`
    >   tx.write_signal(a_id, new_a);
    >   tx.write_signal(b_id, new_b);

#### F142 — Bare (untagged) fences at 18111 and 18221 contain host-runtime pseudocode with trailing ';' and Rust closure/brace syntax; fed to the Ductus lexer this is a lex error (002-16).

- **Severity/Category/Verdict:** LOW / stale_example / CONFIRMED
- **Anchors:** LOG 002-16 · SPEC § 13.10.1, 13.10.4
- **Why it is a defect:** Info string is empty, so nothing marks these as non-Ductus. The content is host-API pseudocode (statement-terminating ';', '|tx| {}' closure, '{}' blocks) that violates 002-16 if read as Ductus. A fence consumer cannot distinguish it from Ductus source.
- **Direction of change:** Tag these fences with a non-Ductus info string (or otherwise mark them as host/pseudocode) so they are excluded from Ductus lexing.
- **Evidence check:** pass — Bare untagged fences at 18111/18221 hold host-runtime pseudocode (';' terminators, Rust closure/brace) that would lex-error if read as Ductus per 002-16.
- **Evidence:**
  - `SPEC.md:18111-18115`
    > ```
    > // Outside or inside a transaction, identical semantics:
    > runtime.write_signal(x_id, 1);   // staged value now 1
    > runtime.write_signal(x_id, 0);   // staged value back to 0
    > runtime.commit();                // x's value equals previous commit — no dirty bit
  - `SPEC.md:18221-18225`
    > ```
    > runtime.transaction(|tx| {
    >   tx.write_signal(a_id, new_a);
    >   tx.write_signal(b_id, new_b);
    > });

#### F130 — Effect-write error-example fences write the Option constructor lowercase `some(custom_response)`, but the variant is `Some`; the example introduces a second unrelated undefined-identifier error.

- **Severity/Category/Verdict:** LOW / stale_example / CONFIRMED
- **Anchors:** LOG — · SPEC § 13.19.5, 13.19.7
- **Why it is a defect:** The illustrative rejected-write uses `some(...)` where the Option variant constructor is `Some` (LOG 006-13; 007-132 shows `Some(result)`). The example intends to show only the assign-to-observed-cell error, but `some` as written is itself a second, unrelated undefined-identifier error, muddying the diagnostic. No program behavior is affected because the whole snippet is a rejected example; hence LOW, category stale_example.
- **Direction of change:** Capitalize the constructor to match the language's Option variant casing used elsewhere.
- **Evidence check:** pass — Example fences write lowercase some(), introducing a second unrelated undefined-identifier error.
- **Evidence:**
  - `packages/ductus-lang/docs/SPEC.md:22295-22295`
    >   --> f.response = some(custom_response)
  - `packages/ductus-lang/docs/SPEC.md:22378-22378`
    >   --> f.response = some(custom_response)
  - `packages/ductus-lang/docs/DECISION_LOG.md:595-595`
    > 006-13. Positionally-declared enum variants accept only positional argument form: `Some(T)`. (§3.5.3)

#### F003 — SPEC 2.5.2 writes the admitted division operator as a backslash `\` instead of `/`; LOG 004-114 and 004-123 use `/`, and `\` is not a Ductus operator.

- **Severity/Category/Verdict:** LOW / stale_example / CONFIRMED
- **Anchors:** LOG 004-114 · SPEC § 2.5.2
- **Why it is a defect:** The division operator in Ductus is `/` (LOG 004-123: 'Integer `/` and `%`'). SPEC §2.5.2 renders the admitted division operator as a backslash `\`, which denotes no operator in the language. It is a normative list of admitted operations, so the wrong glyph is a citation/elaboration defect against LOG 004-114's 'division and modulo', not mere prose.
- **Direction of change:** Correct the glyph in SPEC §2.5.2 from `\` to `/` to match the language's division operator per LOG 004-114/004-123.
- **Evidence check:** pass — SPEC §2.5.2 writes the division operator as backslash '\' instead of '/'; LOG 004-114/004-123 use '/', and '\' denotes no Ductus operator.
- **Evidence:**
  - `packages/ductus-lang/docs/SPEC.md:929-930`
    > including `\` and `%`, comparisons, conditionals, and calls to pure
    > functions:
  - `packages/ductus-lang/docs/DECISION_LOG.md:297-297`
    > 004-114. Concrete const-generic arguments admit the full compile-time vocabulary — integer arithmetic including division and modulo, comparisons, conditionals, and calls to pure functions: `fn recent(h: i32[fib(10) + 1])`. (§2.5.2)

#### F025 — 011-79/80 and §8.9 frame reactive trap/value-track rules for `derived` only, while 026-2/026-6 broaden to recurrent/fold/collect/yield — §8.9 under-covers relative to §13.13.

- **Severity/Category/Verdict:** LOW / divergence / CONFIRMED
- **Anchors:** LOG 011-79, 011-80, 026-2, 026-6 · SPEC § 8.9, 13.13.1, 13.13.2
- **Why it is a defect:** 011-79/011-80 (and their §8.9 elaboration, SPEC lines 6301-6304) frame the reactive trap-propagation and value-track rules exclusively around `derived`. 026-2/026-6 (§13.13.1/2) cover the same subject matter for the broader set of reactive behaviors — recurrent expressions, a fold form's `by:` combiner, and `collect`/`yield` member expressions. §8.9 therefore under-covers relative to §13.13, and 011's two derived-only entries read as if recurrent/fold/collect traps were unaddressed. Not a contradiction (the narrower statements are true), and §13.13/026 fill the broader case, so no implementer-blocking gap — but the two error-handling surfaces (§8.9 and §13.13) are inconsistent in coverage, which is the kind of LOG-SPEC divergence/redundancy the edit protocol warns against.
- **Direction of change:** Reconcile the two reactive-error surfaces: either broaden 011-79/011-80/§8.9 to match 026-2/026-6's construct list (derived + recurrent + fold by: + collect/yield), or make §8.9 defer entirely to §13.13 and drop the derived-only duplication so the two do not drift. User decides.
- **Evidence check:** pass — §8.9/011 frame reactive trap/value-track rules for derived only; 026/§13.13 broaden the same subject to recurrent/fold/collect/yield — §8.9 under-covers.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1190-1191`
    > 011-79. A trap inside a `derived` expression's computation propagates as a normal trap; the reactive system does not catch traps. (§8.9)
    > 011-80. A `derived` whose expression has type `Result[T, E]` or `Option[T]` produces a reactive value of that type, consumed with standard `match` or `?`. (§8.9)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3167-3167`
    > 026-2. A derived or recurrent expression that traps during evaluation — arithmetic overflow under default operators, division by zero, out-of-range array index, or explicit `panic` — aborts the process; the same holds for a trap raised while evaluating a fold form's `by:` combiner or a `collect`/`yield` member expression. (§13.13.1)
  - `packages/ductus-lang/docs/SPEC.md:6301-6304`
    > model. A trap inside a `derived` expression's
    > computation propagates as a normal trap — the reactive system does not
    > catch traps. A `derived` declaration whose expression has type
    > `Result[T, E]` or `Option[T]` produces a reactive value of that type;

#### F024 — `?` desugars to early-return from the 'enclosing function', but 026-7 requires `?` inside derived/recurrent bodies (not functions); no rule bridges body-to-function for the desugaring.

- **Severity/Category/Verdict:** LOW / gap / CONFIRMED
- **Anchors:** LOG 011-46, 026-7, 026-6 · SPEC § 8.4.1, 13.13.2
- **Why it is a defect:** 011-46 and §8.4.1 define `?`'s Break branch strictly as 'return early ... in a Result-returning function / Option-returning function' — the desugaring's early-exit target is 'the enclosing function'. 026-7 (and SPEC §13.13.2 line 18811) require `?` to propagate value-track errors inside reactive expression bodies (a Result/Option-typed derived or recurrent per 026-6). A derived/recurrent expression body is not a function and 011-46 never says what `?`'s early-return does there — what does it return early FROM, and does the derived's declared Result/Option type play the role of the 'function return type'? The corpus does resolve this elsewhere (a derived body compiles to a behavior, a pure function the runtime invokes — 033-145), but that bridging is not stated in any error-handling rule in scope, so an implementer reading 011/026 cannot determine `?`'s in-body semantics without leaving the error-handling section.
- **Direction of change:** State, in the reactive error-handling decisions (026) or in 011-46/§8.4.1, that a reactive expression body with declared type Result[T,E]/Option[T] plays the role of the Result/Option-returning function for `?`'s early-return desugaring; or add a §ref from 026-7 to the rule that establishes the body-as-behavior-function equivalence. User decides.
- **Evidence check:** pass — ? desugaring targets 'enclosing function' but 026-7 applies it inside reactive expression bodies which are not functions; no error-handling rule bridges body-to-function.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1157-1157`
    > 011-46. `expr?` desugars to a `match` on `Try::branch(expr)`: `Continue(value)` yields `value`; `Break(failure)` returns early — `Err(From::convert(failure))` in a `Result`-returning function (converting the inner error value) or `None` in an `Option`-returning function. (§8.4.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3172-3172`
    > 026-7. Downstream reactive expressions propagate value-track errors through `?` or `match`. (§13.13.2)
  - `packages/ductus-lang/docs/SPEC.md:6138-6140`
    > The `?` operator desugars to a `match` on the trait method's result,
    > with the failure branch returning from the enclosing function and
    > applying `From`-conversion to bridge failure types:

#### F021 — 012-129 declares the growable vector's name out of spec, yet 012-91 uses `Vec[T]` normatively and commits a language-level `.count` accessor on it.

- **Severity/Category/Verdict:** LOW / contradiction / CONFIRMED
- **Anchors:** LOG 012-91, 012-129 · SPEC § 9.3.7, 9.3.6
- **Why it is a defect:** 012-129 says the vector's name is outside the spec; 012-91 pins the name `Vec[T]` and makes a normative language-level commitment (a `.count` accessor unifying across it). If `Vec`'s name is truly outside the spec, the language cannot normatively commit `.count` on that named type; if `.count` on it is a language commitment, the name is inside the spec. The two entries disagree on whether `Vec[T]` is a spec-owned surface.
- **Direction of change:** Decide whether the vector name/surface is language-owned or stdlib-owned, then make 012-91's `.count` unification either name it or reference it as a stdlib collection fulfilling the accessor; owner call.
- **Evidence check:** pass — 012-129 puts Vec's name out of spec while 012-91 uses Vec[T] normatively with a language-level .count accessor.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1286-1286`
    > 012-91. `.count` is the unified element-tally accessor across arrays, slices, bundles, `Map[K, V]`, `Vec[T]`, `HashSet[T]`, `yielded T`, and dynamic views — it reports the number of elements — compile-time-known where the extent is static (`T[N]`, `T[..N]`) and runtime where it is not (`T[..]`, `Map`, `Vec`, `HashSet`, `yielded`, dynamic views). (§9.3.7)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1324-1324`
    > 012-129. The dynamic growable vector is a stdlib concern; only fixed-size arrays receive dedicated language syntax, and the vector's name and syntax are outside the specification. (§9.3.6)

#### F076 — 012-92's naming rule states prefixed tallies are `x_count` and 'never bare count', yet its own example `byte_len` uses the `_len` suffix while sibling `char_count` follows the pattern — the rule contradicts its own exemplar.

- **Severity/Category/Verdict:** LOW / design_smell / CONFIRMED (x3 independent reports)
- **Anchors:** LOG 012-92, 012-29, 012-30 · SPEC § 9.3.7, 9.1.6
- **Why it is a defect:** 012-92 formalizes the naming pattern as 'a prefixed `x_count`' and then, in the same sentence, cites `byte_len` as an example of that pattern — but `byte_len` uses `_len`, not `_count`. Its direct sibling on the same type is `char_count`, and every other prefixed tally in the corpus (char_count, pending_count, event_count) uses `_count`. `byte_len` is the lone `_len` name, making the tally surface inconsistent with the rule that describes it. Concrete witness: two methods on the same `string`, `byte_len` vs `char_count`, differ in suffix for no stated reason. Terminology drift only — no behavior changes — hence LOW.
- **Direction of change:** Decide one suffix for prefixed tallies (surface to the user, not resolve unilaterally): either rename `byte_len`→`byte_count` for consistency with `char_count` and the `x_count` pattern 012-92 states, or amend 012-92 to acknowledge `_len` as an accepted prefixed-tally spelling.
- **Evidence check:** pass — Rule generalizes prefixed tallies as x_count but its own example byte_len uses _len; rule does not describe its own exemplar.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1287-1287`
    > 012-92. The tally-accessor naming rule: a bare `count` names the element tally that unifies across every element-bearing type, while a prefixed `x_count` names a specialized tally that is exempt from that unification — `byte_len` and `char_count` on `string` report byte and Unicode-scalar counts rather than an element tally, and stream metrics such as `pending_count` report queue occupancy rather than an element tally. A prefixed tally is never spelled bare `count`.
  - `packages/ductus-lang/docs/DECISION_LOG.md:1287-1287`
    > 012-92. The tally-accessor naming rule: a bare `count` names the element tally that unifies across every element-bearing type, while a prefixed `x_count` names a specialized tally that is exempt from that unification — `byte_len` and `char_count` on `string` report byte and Unicode-scalar counts rather than an element tally, and stream metrics such as `pending_count` report queue occupancy rather than an element tally. A prefixed tally is never spelled bare `count`. (§9.3.7)
  - `packages/ductus-lang/docs/SPEC.md:6991-6995`
    > The accessor is the full word `count`, not `len()` or `length`. This is
    > the uniform element-tally name shared by arrays, slices, bundles, `Map`,
    > `Vec`, `HashSet`, `yielded` groups, and dynamic views (§13.3.3.4).
    > Strings are exempt — they use `byte_len` / `char_count` (§9.1) — and
    > stream metrics keep their specialized `pending_count`-style names.
  - `packages/ductus-lang/docs/DECISION_LOG.md:1224-1225`
    > 012-29. `s.byte_len() -> isize` returns the length in bytes in O(1). (§9.1.6)
    > 012-30. `s.char_count() -> isize` returns the number of Unicode scalars in O(n).
  - `packages/ductus-lang/docs/DECISION_LOG.md:1287-1287`
    > 012-92. The tally-accessor naming rule: a bare `count` names the element tally that unifies across every element-bearing type, while a prefixed `x_count` names a specialized tally that is exempt from that unification — `byte_len` and `char_count` on `string` report byte and Unicode-scalar counts rather than an element tally, and stream metrics such as `pending_count` report queue occupancy rather than an element tally. A prefixed tally is never spelled bare `count`. (§9.3.7)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1224-1224`
    > 012-29. `s.byte_len() -> isize` returns the length in bytes in O(1). (§9.1.6)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1225-1225`
    > 012-30. `s.char_count() -> isize` returns the number of Unicode scalars in O(n). (§9.1.6)

#### F048 — Whether a bare parameter binding used as a placement attr's value-typed RHS needs `move` is ambiguous: 013-248 makes it implicit category B, 013-249 says params into reactive follow category-A `own`/`move`.

- **Severity/Category/Verdict:** LOW / ambiguity / CONFIRMED
- **Anchors:** LOG 013-248, 013-249, 013-3 · SPEC § 11.14
- **Why it is a defect:** Reading A: an `own`-typed function parameter passed as a value RHS at a placement attribute is 'a function parameter flowing into a reactive declaration' → 013-249 → category A → source must write `move`. Reading B: placement-attribute assignment with a value-typed RHS is category B (013-248, 013-3) → implicit consume, no `move`. The two readings differ in whether valid source omits or requires `move` at that exact site. The more-specific 013-248 likely governs, but 013-249's 'like any other category A consumption' is not scoped to exclude the placement-attr-value site, leaving the surface syntax under-pinned.
- **Direction of change:** Scope 013-249 to the argument/initializer positions it means (category A) and explicitly exclude the placement-attr value-RHS site already governed by 013-248/013-3, or state which rule wins at that intersection. Surface to user.
- **Evidence check:** pass — Whether a bare own param used as placement-attr value RHS needs move is ambiguous: category-B implicit (013-248/3) vs category-A move (013-249).
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1634-1634`
    > 013-248. The category-B/C distinction at placement attribute assignment is type-directed: a reactive-typed RHS produces wiring; a value-typed RHS is consumed into the attr's slot. Category D applies to recurrent advance, not placement. (§11.14)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1635-1635`
    > 013-249. Function parameters flowing into reactive declarations follow `own`/`move` like any other category A consumption. (§11.14)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1389-1389`
    > 013-3. Category B (structural storage: record construction, indexed assignment, field assignment via `mut`, whole-value reassignment of `mut`, attr initialization with a value RHS at placement) consumes the RHS into the storage slot implicitly, with no `move` keyword. (§11.1)

#### F039 — 007-230 writes the array range-index instance as bare `Index[Range]`, while sibling 007-229 and SPEC §4.9.5 write `Index[Range[isize]]`.

- **Severity/Category/Verdict:** LOW / divergence / CONFIRMED
- **Anchors:** LOG 007-230, 007-229 · SPEC § 4.9.5
- **Why it is a defect:** Since 007-229 makes `K` part of trait identity, `Index[Range]` and `Index[Range[isize]]` would be different trait instances; the bare form in 007-230 is either an under-specified restatement of the SPEC's `Index[Range[isize]]` or names a distinct (undefined) instance. SPEC §4.9.5 (4200-4201) and sibling 007-229 both use the fully-parameterized `Index[Range[isize]]`. Low severity: reader can infer intent, no soundness impact, but it is terminology drift on an identity-bearing type parameter.
- **Direction of change:** Make 007-230's array/slice range-index instance name consistent with 007-229 and SPEC (`Index[Range[isize]]`), or confirm bare `Range` is the intended canonical spelling everywhere. Surface to user.
- **Evidence check:** pass — 007-230 writes 'Index[Range]' while sibling 007-229 and SPEC §4.9.5 write 'Index[Range[isize]]'; since 007-229 makes K part of trait identity these would be distinct instances.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:848-848`
    > 007-230. Built-in `Index` fulfillments: arrays `T[N]` and slices `T[..N]`/`T[..]` fulfill `Index[isize]` (Output = T) and `Index[Range]` (Output = slice); Bundles (§13.3.3.5) fulfill `Index[isize]` returning a row slice; `Map[K, V]` fulfills `Index[K]` returning `V`. (§4.9.5)
  - `packages/ductus-lang/docs/DECISION_LOG.md:847-847`
    > 007-229. `Index[K]` is part of trait identity (the type parameter `K` distinguishes trait instances) — `Index[isize]` and `Index[Range[isize]]` are distinct trait instances on the same type. (§4.9.5)
  - `packages/ductus-lang/docs/SPEC.md:4200-4201`
    > - **Arrays `T[N]`**: `Index[isize]` → `Output = T`;
    >   `Index[Range[isize]]` → `Output = T[..]` (slice).

#### F032 — 007-92's phrase 'the shift right operand' is ambiguous between the count of `>>` only vs the count operand of any shift; SPEC generalizes to both.

- **Severity/Category/Verdict:** LOW / ambiguity / CONFIRMED
- **Anchors:** LOG 007-92 · SPEC § 4.4.6
- **Why it is a defect:** Reading A: 'shift right operand' = the count operand of a shift, applying to both `<<` and `>>` (the SPEC footnote's reading, which attaches to both `<<` and `>>` rows). Reading B: 'the operand of the right-shift `>>` operator', restricting the u32-count rule to `>>` and leaving `<<`'s count-operand type unstated in this entry. The two readings differ on whether 007-92 governs `<<`. Per Invariant 2 the entry should be self-contained and unambiguous.
- **Direction of change:** Reword 007-92 to say 'the shift-count (right) operand of `<<` and `>>`' so it unambiguously covers both shift operators, matching the SPEC footnote's scope.
- **Evidence check:** pass — 007-92's phrase 'the shift right operand' is ambiguous between the count of '>>' only vs the count operand of any shift; SPEC footnote generalizes to both '<<' and '>>'.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:710-710`
    > 007-92. The shift right operand may be any unsigned integer type narrower than or equal to `u32` (implicitly widened); other types require an explicit cast. (§4.4.6)
  - `packages/ductus-lang/docs/SPEC.md:3306-3308`
    > ¹ The right operand may be any unsigned integer type narrower than or
    > equal to u32 (implicit widening per §4.5.1); other types require an
    > explicit cast.

### Doc hygiene: broken pointers, duplicated canon, undefined terms

LOG entries citing SPEC sections that lack the claimed content, the reconciler-registration canon drifting verbatim across its own declared-identical sites, near-duplicate adjacent entries, and load-bearing terms (containment closure, interpretation context/root, wire-candidate envelope) used normatively but never defined.

#### F233 — 'interpretation context' governs how match/exposition lowering behaves but is never defined; two decisions branch behavior on being 'in interpretation context' with no criterion for when a context is one.

- **Severity/Category/Verdict:** MED / undefined_term / CONFIRMED
- **Anchors:** LOG 009-89, 009-90 · SPEC § 6.2.5
- **Why it is a defect:** 009-89 and 009-90 make the compilation of `match` (value-select vs build-all-arms-and-freeze; static unroll vs mount-time tag) conditional on being 'in interpretation context'. Grepping both docs yields no definitional statement of 'interpretation context' — no rule states which syntactic positions or evaluation phases constitute one. Two careful readers could disagree on whether a given `match` is 'in interpretation context', producing different lowering (build-all-arms vs single-arm) — a behavior-changing ambiguity. Per LEARNINGS 14, the term is used but not defined.
- **Direction of change:** Add a definitional statement fixing what 'interpretation context' is (which positions/phases qualify), then confirm it against 009-89/009-90.
- **Evidence check:** pass — 009-89/009-90 make match lowering (value-select/build-all-freeze; static-unroll/mount-tag) conditional on 'in interpretation context', a term with no cited definitional entry — behavior-changing ambiguity over which matches qualify.
- **Charity check:** sustain — 'interpretation context' is the branch condition that flips match/exposition lowering in 009-89 and 009-90, but the phrase never appears in SPEC.md (grep: 0 hits for 'interpretation context'/'interpretation-context') and appears in the LOG only inside 009-89/009-90 themselves — never with a definition. §13.3.7.7 and §13.19 define 'interpretation' as walking `.exposition`, but state no criterion for WHICH syntactic positions or evaluation phases constitute an 'interpretation context'. Two readers can disagree on whether a given match 'is in interpretation context', yielding different lowering (build-all-arms vs single-arm). Per the undefined_term/ambiguity standard, an undefined behavior-branching condition is the defect; no forced criterion exists. Sustain.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1026-1026`
    > 009-89. `match` is a value selector: it evaluates the scrutinee, selects one arm, evaluates that arm to a value, and discards the rest; in interpretation context a `match` on a reactive scrutinee lowers to `given` semantics (it builds all arms and freezes unselected ones), but the value-selection meaning is unchanged. (§6.2.5)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1027-1027`
    > 009-90. Selecting which node/connection subtree is exposed and kept live is the role of `given` (§13.9.13), not `match`; a `match` over `.exposition` entries in interpretation context compiles to a static unroll for static entries and a single mount-time tag over the closed candidate envelope for dynamic elements, while live-subtree selection remains `given`. (§6.2.5)

#### F232 — 'wire-candidate envelope' is used across five LOG entries and cited to SPEC 13.19 but is never defined as distinct from the defined 'candidate envelope' (017-119).

- **Severity/Category/Verdict:** MED / vague_term / CONFIRMED
- **Anchors:** LOG 015-16, 017-271, 021-140, 021-141, 024-17 · SPEC § 13.3.4, 13.11.5, 13.19
- **Why it is a defect:** 'candidate envelope' has a precise definition (017-119). The compound 'wire-candidate envelope' (used in 015-16, 017-271, 021-140, 021-141, 024-17 and SPEC:525) is never defined. Two readings are open: (a) it is exactly the candidate envelope of a connection/wire endpoint (024-17 links 'candidate edges' and 'the same wire-candidate envelopes'), or (b) a distinct construct. The concreteness matters: the interpretation closure and unreachability class `unreachable_top_level_instance` (021-140) are computed from this set, so whether it is the per-017-119 envelope or something wider changes which top-level instances are legal. No definitional statement resolves it.
- **Direction of change:** State whether 'wire-candidate envelope' is definitionally the candidate envelope (017-119) applied to a connection's wire endpoint, or a distinct set; pick one canonical term. Surface the naming choice to the user.
- **Evidence check:** pass — 'wire-candidate envelope' is used across multiple LOG entries and cited to SPEC §13.19 but never defined as distinct from the defined 'candidate envelope' (017-119); whether it equals the per-017-119 envelope or a wider set changes which top-level instances are legal via unreachability analysis.
- **Charity check:** sustain — 'wire-candidate envelope' appears in 5 LOG entries (1825, 2412, 2879, 2880, 3084) and SPEC:525, but grep for any 'wire-candidate envelope is/are' definitional statement returns nothing — the 'wire-' compound is never introduced. 017-119 (L2260) defines only 'candidate envelope'. The charitable reading is that a wire-candidate envelope = 017-119's candidate envelope applied to a connection's dynamic destination (024-17 L3084 links 'candidate edges ... the same wire-candidate envelopes'), but that equivalence is nowhere stated; it is a reader inference. Per the vague_term standard, existence of a plausible second reading (a distinct construct) plus the inference-required-ness of reading (a) IS the defect. The set feeds the interpretation closure and the unreachable_top_level_instance class (021-140), so whether it is exactly 017-119's envelope or wider changes which top-level instances are legal. Sustain.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2260-2260`
    > 017-119. The candidate envelope is: for a dynamic destination, every type the reference's static type admits; for a repeat-materialized connection, its destination's type. (§13.3.4)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3084-3084`
    > 024-17. A connection whose destination is a dynamic reference contributes candidate edges to each node type the reference's static type admits, analyzed over node types as a sound over-approximation of their instances; the same wire-candidate envelopes also feed the interpretation closure, which is the static set formed by taking the containment closure in union with those envelopes. (§13.11.5)
  - `packages/ductus-lang/docs/SPEC.md:525-525`
    > wire-candidate envelopes, §13.19). Because that closure is finite and

#### F231 — 'containment closure' is the base of the interpretation/interpretation closure and live graph but is never given a definitional statement in either document.

- **Severity/Category/Verdict:** MED / undefined_term / CONFIRMED
- **Anchors:** LOG 015-16, 017-271, 024-17 · SPEC § 13.1, 13.3.8, 13.11.5
- **Why it is a defect:** 'containment closure' carries the interpretation closure (017-271), the live graph (021-141 uses the equivalent enumerated set), and topology analysis (015-16). Grepping both docs for a definitional statement of 'containment closure' returns only these usages plus its parenthetical use at SPEC:524; there is no sentence stating what the containment closure IS (which instances it comprises: subtree only? subtree plus Handle-reachable? the enumeration in 021-140/141 is attached to 'transitive closure', not to 'containment closure'). Per LEARNINGS 14, usage is not definition. An implementer cannot compute the interpretation closure without a definition of its base set.
- **Direction of change:** Add a definitional statement of 'containment closure' (or reconcile it with the enumerated 'transitive closure' of 021-140/141) and confirm which term is canonical.
- **Evidence check:** pass — 'containment closure' is the base set of the interpretation closure yet is never given a definitional statement (subtree only? subtree + Handle-reachable?) in either document; the enumeration in 021-140/141 attaches to 'transitive closure', not 'containment closure'.
- **Charity check:** sustain — 'containment closure' is used as the base of the interpretation closure (017-271, SPEC:524), the live graph (021-140/141), and topology analysis (015-16/024-17), but grep of both docs finds NO definitional statement of what set it comprises. All 3 LOG uses (1825, 2412, 3084) and the sole SPEC use (524) are usages, not definitions. The enumeration that DOES exist (SPEC §13.8.1 L16457-16464 and 021-140/141) is attached to 'transitive closure' and lists three-to-five members (subtree, connection destinations, Handle-reachable, effect-arg borrows, wire-candidate envelopes); nothing forces 'containment closure' to equal any specific one of these (e.g. the bare subtree vs subtree+destinations+Handle-reachable). Per the gap standard, refute requires explicit normative text defining the set — none exists. An implementer cannot compute the interpretation closure without knowing its base. Sustain. (Side note not part of this finding: SPEC:525 cites §13.19 for this closure but §13.19 is Effects, and 021-141 enumerates five members while SPEC §13.8.1 enumerates three — separate divergences.)
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2412-2412`
    > 017-271. An interpretation closure is the static set of instances an interpretation root can reach: the containment closure UNION the wire-candidate envelopes, both static sets. One render is mounted per `(root, instance)` pair at startup, at stable paths. (§13.3.8)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1825-1825`
    > the candidate topology's wire-candidate envelopes join the interpretation closure, so that closure is the containment closure of an interpretation root unioned with those static wire-candidate envelopes. (§13.1)
  - `packages/ductus-lang/docs/SPEC.md:523-525`
    > bounded by the **finite interpretation closure** — the static set of
    > instances an interpretation root can reach (containment closure union
    > wire-candidate envelopes, §13.19).

#### F185 — 017-190's single cite §8.4.1 is broken for its optional-chaining half: §8.4.1 elaborates only standalone Try-propagation of `?`, not `expr?.field`/`?[i]`/`?()` short-circuiting (which lives under §13.3.x).

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED (x2 independent reports)
- **Anchors:** LOG 017-190 · SPEC § 8.4.1
- **Why it is a defect:** The LOG entry's single cite (§8.4.1) is a pointer contract: the reader expects §8.4.1 to elaborate the whole entry. §8.4.1 covers only Try-propagation `expr?`; the optional-chaining split that is the majority of 017-190's content is elaborated elsewhere (§13.3.6.2, SPEC lines 14237, 14321-14324), leaving §8.4.1 non-elaborating for that half. Pointer contract is broken.
- **Direction of change:** Point the optional-chaining half of 017-190 to the section that actually elaborates it, or extend §8.4.1 to cover optional chaining; decide with the user which.
- **Evidence check:** pass — 017-190's sole cite §8.4.1 is a broken pointer for its optional-chaining half: §8.4.1 desugars only standalone/terminal 'expr?' Try-propagation; the '?.field'/'?[i]'/'?()' short-circuit-to-None semantics are elaborated under §13.3.x, so a reader following the cite finds no elaboration of that half.
- **Charity check:** sustain — Same defect as F023 confirmed independently: 017-190 (DECISION_LOG.md:2331) carries the sole cite (§8.4.1); SPEC §8.4.1 (SPEC.md:6136-6167) covers only `expr?` Try-propagation desugaring; the optional-chaining `expr?.field`/`?[i]`/`?()` semantics that dominate 017-190 are elaborated under §13.3.6.2 (SPEC.md:14321-14324, 'not the terminal Try-propagation `?` of §8.4'). The LOG entry's pointer lands on a section that does not elaborate the majority of the entry. Non-dissolving.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2331-2331`
    > 017-190. Postfix `?` splits by position. Standalone or terminal `expr?` is Try-propagation (the `Try::branch` desugaring that returns early on `Break` and yields the inner value on `Continue`). Optional chaining `expr?.field`, `expr?[i]`, `expr?()`: if `expr` is `None`, the chain short-circuits to `None`; if `expr` is `Some(v)`, the chain continues with `v`. The chain's result type is `Option[final-type]`. (§8.4.1)
  - `packages/ductus-lang/docs/SPEC.md:6136-6167`
    > #### 8.4.1 Desugaring
    > 
    > The `?` operator desugars to a `match` on the trait method's result,
    > with the failure branch returning from the enclosing function and
    > applying `From`-conversion to bridge failure types:
  - `packages/ductus-lang/docs/SPEC.md:14237-14237`
    > the job of optional chaining `?.`, `?[]`, `?()`: the
  - `packages/ductus-lang/docs/DECISION_LOG.md:2331-2331`
    > 017-190. Postfix `?` splits by position. Standalone or terminal `expr?` is Try-propagation (the `Try::branch` desugaring that returns early on `Break` and yields the inner value on `Continue`). Optional chaining `expr?.field`, `expr?[i]`, `expr?()`: if `expr` is `None`, the chain short-circuits to `None`; if `expr` is `Some(v)`, the chain continues with `v`. The chain's result type is `Option[final-type]`. (§8.4.1)
  - `packages/ductus-lang/docs/SPEC.md:6136-6155`
    > #### 8.4.1 Desugaring
    > 
    > The `?` operator desugars to a `match` on the trait method's result,
    > with the failure branch returning from the enclosing function and
    > applying `From`-conversion to bridge failure types:
    > 
    > ```
    > expr?
    > ```
    > 
    > desugars to:
    > 
    > ```
    > match Try::branch(expr):
    >   Continue(value): value
    >   Break(failure):

#### F040 — 008-57 invokes the 'storability razor' and cites §5.7, but §5.7 never states that razor; its rationale (brackets vs lowercase kinds) lives at §13.2.8 / §13.2.8.1, where the sibling razor entry 016-179 correctly points.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 008-57 · SPEC § 5.7, 13.2.8.1
- **Why it is a defect:** The LOG-to-SPEC pointer contract requires the cited section to elaborate the entry's normative claim. 008-57's load-bearing claim is a razor claim (brackets-vs-lowercase-kind spelling, comparison to Handle/WeakHandle/Portal), but §5.7 contains no razor content and never mentions lowercase kind keywords or the storable/binding-machinery split. The razor is defined and rationalized at §13.2.8 / §13.2.8.1, which is exactly where the sibling razor entry 016-179 points. A reader following 008-57's (§5.7) to verify the razor claim finds no elaboration of it.
- **Direction of change:** Re-point 008-57's citation toward the section that actually states the storability razor (the §13.2.8 family, matching 016-179), or add the razor's spelling rationale into §5.7 so the cited section elaborates the claim. Do not decide which; surface to the user.
- **Evidence check:** pass — 008-57's load-bearing razor claim cites §5.7, but §5.7 contains no razor content; the storability razor is elaborated at §13.2.8 (SPEC 12515-12517), where the sibling entry 016-179 correctly points.
- **Charity check:** sustain — Pointer-contract break confirmed. §5.7 spans SPEC 4631-4736 (all subsections 5.7.1-5.7.5); none states the storability razor. §5.7.4 Storage (4706-4713) says a type value `may be stored` but never states the brackets-vs-lowercase-kind razor or compares against Handle/WeakHandle/Portal spelling. The ONLY `storability razor` text in the entire SPEC is at line 12515 (§13.2.8.1), whose table (12510-12513) lists `Type[C]`, `Handle[T]`, `WeakHandle[T]`, `Portal[T]` as storable bracket forms — exactly 008-57's claim. The sibling razor entry 016-179 correctly cites §13.2.8. A reader following 008-57's (§5.7) to verify the razor claim finds no elaboration of it. MED sustained.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:917-917`
    > 008-57. `Type[C]` keeps its bracket spelling under the storability razor: it is a storable compile-time-only value, not reactive binding machinery, so it is written with brackets like the other storable designators (`Handle[T]`, `WeakHandle[T]`, `Portal[T]`) and is never respelled as a lowercase kind keyword. (§5.7)
  - `packages/ductus-lang/docs/SPEC.md:4631-4638`
    > ### 5.7 The `Type[…]` meta-type
    > 
    > `Type[C]` is the type of a **type value** — a value whose value is a
    > *type* satisfying the constraint `C`. Where `dyn` (§5.2) lifts a trait to
    > value position as an *instance* (an erased, vtable-dispatched value of some
    > implementing type), `Type[…]` lifts a constraint to value position as the
    > *type itself*. The two are complementary: `dyn Drivable` is "some Drivable
    > value"; `Type[Drivable]` is "some Drivable *type*."
  - `packages/ductus-lang/docs/SPEC.md:12515-12517`
    > Rationale (the **storability razor**): a designator that names something
    > storable keeps brackets; a designator that names reactive binding
    > machinery is a lowercase kind keyword.
  - `packages/ductus-lang/docs/DECISION_LOG.md:2032-2032`
    > 016-179. The storability razor: storable values keep bracket types (`Type[C]`, `Handle[T]`, `WeakHandle[T]`, `Portal[T]`), while binding machinery is spelled as lowercase kinds; a cell is refused as a value, and `Portal[Cell[T]]` is the sanctioned identity-as-data form. (§13.2.8)

#### F036 — Entries 007-234/007-235 cite §4.9.4 for integer/bool/string Hash auto-impl, but §4.9.4 never mentions Hash; the Hash conformance is elaborated only in §4.9.5.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 007-234, 007-235 · SPEC § 4.9.4, 4.9.5
- **Why it is a defect:** The pointer contract (a LOG entry's (SECTION-REF) must land on the SPEC text that elaborates it) is broken. §4.9.4's integer auto-impl list (SPEC 4126-4140) and float list (4141-4166) do not contain `Hash`; a grep of §4.9.4 for `Hash` returns nothing. The claim is true but the elaboration lives in §4.9.5's Hash conformance table (SPEC 4263-4270). The asymmetry is self-evident within the same block: sibling entry 007-237 (float non-Hash) correctly cites §4.9.5, and 007-236 (duration/instant Hash) correctly cites §9.4, while 007-234/235 point at a section that says nothing about Hash.
- **Direction of change:** Re-point 007-234 and 007-235 at the section that actually elaborates built-in Hash conformance (the §4.9.5 Hash conformance table), matching sibling 007-237; OR add the Hash auto-impl to §4.9.4's built-in trait lists so the existing cite lands. Surface to user which document is authoritative here.
- **Evidence check:** pass — 007-234/007-235 cite §4.9.4 for the integer/bool/string Hash auto-impl, but §4.9.4's auto-impl lists never mention Hash; the elaboration lives only in §4.9.5's Hash conformance table (SPEC 4263-4270), where the sibling non-Hash entry correctly points.
- **Charity check:** sustain — Pointer-contract break confirmed. §4.9.4 runs SPEC 4119-4177; its integer auto-impl list (4128-4140) enumerates Add/Sub/.../IntPow and explicitly does NOT include Hash. The `Built-in Hash conformance` subsection is at SPEC 4257, under §4.9.5 (which starts at 4178), not §4.9.4. 007-234/007-235 cite (§4.9.4) but the Hash elaboration lives only in §4.9.5. The asymmetry is self-evident: sibling 007-237 (float non-Hash) correctly cites §4.9.5 and 007-236 (duration/instant) correctly cites §9.4, while 007-234/235 point at a section that never mentions Hash. MED sustained.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:852-853`
    > 007-234. All integer types (`i8`–`i128`, `u8`–`u128`, `isize`, `usize`) auto-implement `Hash`, joining their existing auto-implemented trait conformance list. (§4.9.4)
    > 007-235. `bool` and `string` auto-implement `Hash` (joining `char`, which already satisfies `Hash`). (§4.9.4)
  - `packages/ductus-lang/docs/SPEC.md:4126-4140`
    > - **All integer types** auto-implement: `Add`, `Sub`, `Mul`, `Rem`,
    >   `IntDiv`, `BitAnd`, `BitOr`, `BitXor`, `BitNot`, `Shl`, `Shr`; the
    >   wrapping variants `WrappingAdd`, `WrappingSub`, `WrappingMul`,
    >   `WrappingIntDiv`, `WrappingRem`; the saturating variants
    >   `SaturatingAdd`, `SaturatingSub`, `SaturatingMul`, `SaturatingIntDiv`,
    >   `SaturatingRem`; the checked variants `CheckedAdd`, `CheckedSub`,
    >   `CheckedMul`, `CheckedIntDiv`, `CheckedRem` (note: not `CheckedDiv`,
    >   which is float-only since `/` widens integers to float per §4.4.1.1);
    >   the cast traits `WrappingAs`, `SaturatingAs`, `CheckedAs`; `Zero`,
    >   `One`, `Abs`, `Min`, `Max`, `Ord`, `Eq`, `IntPow`; and (for signed
    >   integer types) `Neg`, `WrappingNeg`, `SaturatingNeg`, `CheckedNeg`.
    >   They satisfy `Integer`, `Numeric`, and `Signed` or `Unsigned`
    >   accordingly.
  - `packages/ductus-lang/docs/SPEC.md:4263-4270`
    > | Type                                         | `Hash` | Rationale                          |
    > |----------------------------------------------|:-----:|------------------------------------|
    > | `i8`–`i128`, `u8`–`u128`, `isize`, `usize`   | yes   | Trivial integer hash                |
    > | `bool`                                       | yes   | Two-value hash                     |
    > | `char`                                       | yes   | Unicode scalar value               |
    > | `string`                                     | yes   | Byte-content hash                  |
    > | `duration`, `instant`                        | yes   | i64-backed (§9.4); integer hash    |
    > | `f32`, `f64`                                 | **no**| NaN ≠ NaN violates `Eq → Hash`     |
  - `packages/ductus-lang/docs/DECISION_LOG.md:855-855`
    > 007-237. Float types (`f32`, `f64`) do NOT implement `Hash`. Any expression demanding `Hash` on a float type is a compile error at the bound. (§4.9.5)

#### F034 — 006-27 asserts the `.exposition` entry-match typed-bound carve-out `Variant(name: Bound)` and cites §3.5.7, but §3.5.7 says nothing about exposition or typed bounds; the actual elaboration lives at §13.3.7.7.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 006-27 · SPEC § 3.5.7, 13.3.7.7
- **Why it is a defect:** 006 carries exactly one SPEC ref per entry (Invariant 3, LOG-to-SPEC). 006-27 makes a substantive normative claim — the exposition-only typed-bound pattern `Variant(name: Bound)` — and points it at §3.5.7. But §3.5.7 (SPEC L2336-2348) covers only positional/named pattern forms, exhaustiveness, and the `...` rest token; it contains no `exposition`, `Bound`, or typed-bound text (verified). The claim's real elaboration is at §13.3.7.7 (L14798, `**The entry-match-only bounded pattern `Variant(name: Bound)`.**`). The pointer contract is broken: a reader following 006-27's (§3.5.7) reference will not find the rule it names.
- **Direction of change:** Surface to user: 006-27's SPEC reference should point at the section that actually elaborates the exposition typed-bound carve-out (§13.3.7.7), or §3.5.7 should be extended to state/point to it. Do not re-decide which section owns the rule.
- **Evidence check:** pass — 006-27's exposition-only typed-bound carve-out `Variant(name: Bound)` is cited to §3.5.7, but §3.5.7 contains no exposition/Bound/typed-bound text; the carve-out is elaborated only at §13.3.7.7, breaking the single-leaf-citation pointer contract for that clause.
- **Charity check:** sustain — Confirmed broken pointer for the load-bearing claim. 006-27's substantive carve-out — the exposition/entry-match-only typed-bound pattern Variant(name: Bound) where Bound is a trait or concrete type — is elaborated at §13.3.7.7 (SPEC L14798, 'The entry-match-only bounded pattern Variant(name: Bound)'), a distant top-level §13 section, NOT the cited §3.5.7. §3.5.7 (L2336-2348) covers only positional/named pattern forms, exhaustiveness, and the ... rest token; grep confirms zero 'exposition'/'Bound'/typed-bound text there (the one 'bound' hit is 'bound to the wildcard'). Unlike F012 (where the cited section physically contains the target subsection), §13.3.7.7 is not nested under §3.5.7, so the reader following (§3.5.7) cannot reach the carve-out. §3.5.7 does carry only 006-27's opening positional/named-parallel-to-construction fragment. The dissolving check fails: no passage at §3.5.7 covers the carve-out, and the covering passage sits in a section the ref does not point to. Sustain. | packages/ductus-lang/docs/SPEC.md:2336-2348 — "#### 3.5.7 Argument forms in patterns ... Variant patterns may be positional or named, parallel to variant construction; mixing within one pattern is a compile error. Record patterns are always named; tuple patterns are always positional. Patterns are exhaustive by default..." — contains NO exposition/typed-bound text; the carve-out lives instead at SPEC.md:14798 "**The entry-match-only bounded pattern `Variant(name: Bound)`.**" under §13.3.7.7. The two passages are consistent (§3.5.7 simply omits the carve-out), so this confirms rather than dissolves the finding.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:609-609`
    > 006-27. Variant patterns may be positional or named, parallel to variant construction; as an entry-match-only carve-out, inside an `.exposition` entry match (and only there) a variant payload may bind a typed bound `Variant(name: Bound)` where `Bound` is a trait or concrete type, which is not a general pattern shape and is unavailable at ordinary variant-pattern sites. (§3.5.7)
  - `packages/ductus-lang/docs/SPEC.md:2339-2344`
    > or named, parallel to variant construction; mixing within one pattern is
    > a compile error. Record patterns are always named; tuple patterns are
    > always positional.
    > 
    > Patterns are exhaustive by default: a record pattern must bind every field, and a tuple or variant-payload pattern every component, in their respective named or positional forms.
  - `packages/ductus-lang/docs/SPEC.md:14761-14761`
    > ##### 13.3.7.7 The entry sum: walking `.exposition`

#### F016 — 009-91 asserts the trait-headed record-match ban and cites §6.2.4, but §6.2.4 never states the ban; the ban lives only at §13.3.7.7 which points back to §6.2.4 — broken pointer contract.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 009-91 · SPEC § 6.2.4, 13.3.7.7
- **Why it is a defect:** Refs flow LOG→SPEC and the cited section must elaborate the claim (pointer contract). 009-91's cite (§6.2.4) does not contain the trait-headed-record-match ban nor any mention of the `Variant(name: Bound)` payload-bound restriction; the ban is elaborated only at §13.3.7.7, which itself cites §6.2.4 as the authority. An implementer following 009-91's pointer to §6.2.4 finds nothing about the ban.
- **Direction of change:** Either move/add the trait-headed record-match ban text into §6.2.4 so the cite is apt, or repoint 009-91's SPEC-ref to the section that actually elaborates it (§13.3.7.7). Bring the choice to the user; do not self-decide.
- **Evidence check:** pass — 009-91 cites §6.2.4 as the elaboration of the trait-headed-record-match ban, but §6.2.4 (Pattern matching) never states it; the ban lives only at §13.3.7.7 which points back to §6.2.4 — circular/broken pointer.
- **Charity check:** sustain — 009-91 asserts the trait-headed record-match ban and the `Variant(name: Bound)` payload-bound restriction, citing §6.2.4. Fresh read of §6.2.4 (SPEC.md:5254-5316) covers positional/named/nested/wildcard patterns only — grep of that region for 'trait-headed|ban|Bound|exposition' returns nothing about the ban. The ban is elaborated only at §13.3.7.7 (SPEC.md:14808-14810), which itself cites §6.2.4 as the authority ('trait-headed record match stays banned, §6.2.4'). Following 009-91's pointer to §6.2.4 finds no ban — broken LOG→SPEC pointer contract; the cited section fails to carry the claim. Sustain.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1028-1028`
    > 009-91. A trait-headed `match` over a record value is a compile error: a bound in match-payload position — `Variant(name: Bound)` where `Bound` is a trait or concrete type — is legal only in a `match` over `.exposition` entries, never over ordinary records. (§6.2.4)
  - `packages/ductus-lang/docs/SPEC.md:5254-5316`
    > #### 6.2.4 Pattern matching
    > 
    > The `match` expression is the canonical way to consume an enum value. Each arm specifies a pattern and an
    > expression:
  - `packages/ductus-lang/docs/SPEC.md:14808-14810`
    > - The bound binder `name: Bound` is scoped to entry matches; it is not a
    >   general pattern shape and does not appear in ordinary `match` over
    >   records (trait-headed record match stays banned, §6.2.4).

#### F012 — 005-68/005-69/005-70 cite §3.1.1, but §3.1.1 covers only ordinary methods; the effect-kind form, observed: contract, and projection they describe are elaborated exclusively in §3.1.1.1, breaking the leaf-citation pointer contract.

- **Severity/Category/Verdict:** MED / divergence / PLAUSIBLE
- **Anchors:** LOG 005-68, 005-69, 005-70 · SPEC § 3.1.1, 3.1.1.1
- **Why it is a defect:** The section convention (visible at 005-44 citing §3.1.6.1 and 005-73+ citing §3.2.1) is to cite the leaf subsection that actually elaborates the claim. §3.1.1 as read (lines 1192-1221) elaborates only fn-methods and never mentions the effect keyword, observed: contracts, interpretation obligations, or consumer projection. That content lives solely in §3.1.1.1. A reader following 005-68/69/70's pointer lands on a section that does not elaborate them; the pointer contract (LOG-to-SPEC citation aptness) is broken.
- **Direction of change:** Repoint 005-68, 005-69, and 005-70 at §3.1.1.1, the subsection that actually elaborates effect-kind methods.
- **Evidence check:** pass — 005-68/69/70 cite §3.1.1 but §3.1.1 covers only fn-methods; the effect-kind form, observed: contract, and consumer projection they assert are elaborated exclusively in §3.1.1.1, so the LOG-to-SPEC pointer lands on a section that does not elaborate the claim.
- **Charity check:** refute — The cited section §3.1.1 CONTAINS the elaboration. §3.1.1.1 'Effect-kind trait methods' (SPEC L1223) is nested strictly inside §3.1.1 'Method signatures' (L1192), before §3.1.2 (L1255) — verified by heading grep. The effect-keyword form, observed: contract, MINIMUM semantics, and consumer projection are all at L1223-1253, within §3.1.1's span. A reader following (§3.1.1) reads that span and reaches the material. Moreover the LOG's actual convention on this topic is uniform §3.1.1: every fn-method entry (005-7..005-14) AND every effect-kind entry (005-68/69/70) cites §3.1.1; NO LOG entry cites §3.1.1.1 at all. So §3.1.1 is the section's chosen umbrella ref, not a mis-pointer. Invariant 3 mandates one section-ref, not leaf granularity; the 'leaf-citation pointer contract' the finding invokes is an inferred convention contradicted by the LOG's own uniform usage. Dissolving fact (nesting + uniform citation) is consistent with the finding's passage, so refute not refile. | packages/ductus-lang/docs/SPEC.md:1223 — "##### 3.1.1.1 Effect-kind trait methods" (nested under §3.1.1 at SPEC.md:1192 "#### 3.1.1 Method signatures", before §3.1.2 at SPEC.md:1255 "#### 3.1.2 Associated types"); and DECISION_LOG.md:409-411 all cite "(§3.1.1)" exactly as DECISION_LOG.md:348-355 (the fn-method entries 005-7..005-14) do — the LOG uses §3.1.1 as the umbrella ref for all method material and never cites §3.1.1.1.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:409-411`
    > 005-68. A trait may declare an effect-kind method with the form `effect name(params):` whose first parameter is `Subject`-typed; an effect-kind method declares an interpretation obligation that a fulfilling type discharges. (§3.1.1)
    > 005-69. An effect-kind trait method may declare an `observed:` contract block listing the required output cells it exposes; the contract is a MINIMUM — a fulfill may expose more cells than the contract lists. (§3.1.1)
    > 005-70. A consumer projecting through an effect-kind trait sees only the cells named in the trait's `observed:` contract, never the additional cells a fulfill may expose. (§3.1.1)
  - `packages/ductus-lang/docs/SPEC.md:1223-1234`
    > ##### 3.1.1.1 Effect-kind trait methods
    > 
    > A trait may declare **effect-kind methods** — methods introduced with the
    > `effect` keyword rather than `fn` — so that interpretation logic can be
    > written against a trait instead of a concrete type:
    > 
    > ```
    > trait Renderable:
    >   effect render(value: Subject):
    >     observed:
    >       audio: stream ring[256] Sample     // contract: minimum required output cells
    > ```
  - `packages/ductus-lang/docs/SPEC.md:1192-1205`
    > #### 3.1.1 Method signatures
    > 
    > Trait methods are declared with the `fn` keyword inside the trait body. The
    > signatures use `Subject` (capitalized, the type-level identifier) to refer to the
    > implementing type:
    > 
    > ```
    > trait Eq:
    >   fn eq(a: Subject, b: Subject) -> bool
    > ```
    > 
    > `Subject` is a type-level placeholder bound during implementation: in a `fulfill
    > Eq for i32` block, `Subject` resolves to `i32`, so the method's signature becomes
    > `fn eq(a: i32, b: i32) -> bool`.
  - `packages/ductus-lang/docs/DECISION_LOG.md:385-385`
    > 005-44. A bare reference to a generic trait with defaulted type parameters resolves to the instance with all defaults applied, uniformly in `requires`, trait bounds, `satisfies`, and `fulfill` positions: `T: Add` is sugar for `T: Add[Subject]`. (§3.1.6.1)

#### F260 — 027-80 and 027-81 are two adjacent LOG entries stating the SAME iff reconciler-registration rule with duplicated lead clauses and differing trailing failure descriptions — a self-containment/atomicity redundancy under LOG Invariant 2.

- **Severity/Category/Verdict:** LOW / design_smell / CONFIRMED
- **Anchors:** LOG 027-80, 027-81 · SPEC § 13.14.7
- **Why it is a defect:** Both entries open with the identical iff clause verbatim, then 027-80 handles the generic-instantiation case and 027-81 handles the general case. The two are consistent ('refuse the live transition' == 'does not enter the live state'), so this is not a contradiction, but the duplicated lead sentence and split failure-behavior across two entries reads as a copy-paste artifact rather than two atomic decisions. It risks future divergence if only one is edited.
- **Direction of change:** Consider merging into a single atomic entry (or removing the duplicated lead clause from one), keeping both the generic-instantiation and general failure behaviors, so the iff rule is stated once.
- **Evidence check:** pass — Same 027-80/027-81 duplicated-lead redundancy under LOG Invariant 2 self-containment/atomicity.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:3258-3258`
    > 027-80. Reconciler registration is required if and only if the effect's `observed:` block declares host-written channels (`signal`/`stream`); an interior effect whose contract is fulfilled entirely by child effects requires no reconciler. Generic-effect instantiations whose `observed:` block declares host-written channels but lack a registered reconciler are detected at startup and cause the runtime to refuse the live transition. (§13.14.7)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3259-3259`
    > 027-81. Reconciler registration is required if and only if the effect's `observed:` block declares host-written channels (`signal`/`stream`); an interior effect whose contract is fulfilled entirely by child effects requires no reconciler. If the program declares an effect type whose `observed:` block has host-written channels and no reconciler is registered, startup fails with a diagnostic naming the effect type and the runtime does not enter the live state. (§13.14.7)

#### F234 — 'interpretation root' is the reachability base for the interpretation closure but is only described by its bootstrap form (usage), never given a definitional statement, and its relation to 'entry-point' is left implicit.

- **Severity/Category/Verdict:** LOW / undefined_term / CONFIRMED
- **Anchors:** LOG 015-16, 017-271, 021-141 · SPEC § 13.1, 13.3.8, 13.8.1
- **Why it is a defect:** The interpretation closure (017-271) and per-root render mounting (021-141) hinge on 'interpretation root', yet the only near-definition (SPEC:19934) states how a root is *expressed* ('is expressed by piping a node reference...') — a construction form, not a statement of what the root IS. Meanwhile 021-140/141 speak of the singular 'entry-point' as the reachability base for the live graph, while 017-271/021-141 speak of one-or-more 'interpretation root(s)'. Whether interpretation root == entry-point, or a root is any node bootstrapped in an `effects:` clause (so multiple roots exist under one entry-point), is not stated. Per LEARNINGS 14 form-of-expression is not definition.
- **Direction of change:** Add a definitional statement of 'interpretation root' and state its relationship to 'entry-point' (equal, or many-roots-under-one-entry-point). Surface to the user.
- **Evidence check:** pass — 'interpretation root' only described by bootstrap/usage form, never defined; its relation to singular 'entry-point' left implicit.
- **Evidence:**
  - `packages/ductus-lang/docs/SPEC.md:19933-19936`
    > method** whose first (Subject-typed) parameter accepts that node
    > (§3.1.1.1, effect-kind trait methods). This is the *interpretation
    > bootstrap* form: an interpretation root is expressed by piping a node
    > reference into an effect-kind trait method that walks the node's
  - `packages/ductus-lang/docs/DECISION_LOG.md:2880-2880`
    > each interpretation root's renders mount once at startup at stable paths within this closure. (§13.8.1, §13.14.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2412-2412`
    > 017-271. An interpretation closure is the static set of instances an interpretation root can reach: the containment closure UNION the wire-candidate envelopes, both static sets. One render is mounted per `(root, instance)` pair at startup, at stable paths. (§13.3.8)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2879-2879`
    > 021-140. A top-level node instance that is not the entry-point and is not reachable from the entry-point's transitive closure is a compile error of class `unreachable_top_level_instance`.

#### F191 — The reconciler-registration canon sentence is verbatim-inconsistent across its six+ canon sites: 015-39 joins the two clauses with ', and' while the 027/031/033 sites use a semicolon, drifting the 'single source' wording.

- **Severity/Category/Verdict:** LOW / divergence / CONFIRMED
- **Anchors:** LOG 015-39, 027-80, 031-128, 033-124 · SPEC § 13.1, 13.14.7, 13.19.14, 15.4.1
- **Why it is a defect:** Invariant 2 requires each canon restatement to be self-contained; when a rule is deliberately restated verbatim across many sites, drift in the restatement erodes the 'one canonical sentence' contract. 015-39 uses '...channels (`signal`/`stream`), and an interior effect...' (comma+and) whereas 027-80/031-128/031-143/031-157/033-124 use '...channels (`signal`/`stream`); an interior effect...' (semicolon). This is terminology/wording drift, not a behavior change, but it undermines the intent that these be identical canon statements. NOTE for orchestrator: SPEC 13.1 L11471-11475 also carries the sentence with the semicolon form, so 015-39 is the odd one out.
- **Direction of change:** Normalize the shared canon sentence to one exact punctuation/wording across all sites (015-39 and the 027/031/033 sites), so the deliberately-restated invariant reads identically everywhere.
- **Evidence check:** pass — Reconciler-registration canon verbatim-inconsistent: 015-39 uses ', and' vs semicolon across 027/031/033 sites.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1848-1848`
    > 015-39. A new effect type is added by declaring an `effect`; reconciler registration is required if and only if the effect's `observed:` block declares host-written channels (`signal`/`stream`), and an interior effect whose contract is fulfilled entirely by child effects requires no reconciler. (§13.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3258-3258`
    > 027-80. Reconciler registration is required if and only if the effect's `observed:` block declares host-written channels (`signal`/`stream`); an interior effect whose contract is fulfilled entirely by child effects requires no reconciler. Generic-effect instantiations whose `observed:` block declares host-written channels but lack a registered reconciler are detected at startup and cause the runtime to refuse the live transition. (§13.14.7)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3897-3897`
    > 031-128. Reconciler registration is required if and only if the effect's `observed:` block declares host-written channels (`signal`/`stream`); an interior effect whose contract is fulfilled entirely by child effects requires no reconciler.
  - `packages/ductus-lang/docs/DECISION_LOG.md:4236-4236`
    > 033-124. Reconciler dependencies are `(effect_type_name, [concrete_type_parameters])` pairs the host must register via `runtime.register_reconciler` before the runtime can enter the live state. Reconciler registration is required if and only if the effect's `observed:` block declares host-written channels (`signal`/`stream`); an interior effect whose contract is fulfilled entirely by child effects requires no reconciler and contributes no reconciler dependency.

#### F190 — 027-80 and 027-81 are near-duplicate entries in the same section: identical first sentence stating the iff-rule, differing only in a second clause, violating the atomic/self-contained-one-decision intent.

- **Severity/Category/Verdict:** LOW / design_smell / CONFIRMED
- **Anchors:** LOG 027-80, 027-81 · SPEC § 13.14.7
- **Why it is a defect:** Both entries repeat the identical iff registration rule (first sentence) and then attach a startup-refusal clause — 027-80 phrased for generic-effect instantiations, 027-81 for effect types generally (the generic case is a subcase). They do not contradict, but two consecutive same-section entries carrying the same primary decision is a redundancy/atomicity smell: a reader cannot tell whether 027-80 states a distinct obligation from 027-81 or merely re-states it with a narrower example. SPEC 13.14.7 elaborates both (Generic effects L19094-19100; Unregistered effect types L19102-19109), so citation is apt; the defect is LOG-internal redundancy.
- **Direction of change:** Decide whether the generic-instantiation refusal is a distinct decision or a subcase of the general refusal; if a subcase, fold it into one entry so the section carries the iff-rule and its startup-diagnostic consequence once.
- **Evidence check:** pass — 027-80 and 027-81 duplicate the iff lead sentence and split the failure clause across two adjacent entries.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:3258-3258`
    > 027-80. Reconciler registration is required if and only if the effect's `observed:` block declares host-written channels (`signal`/`stream`); an interior effect whose contract is fulfilled entirely by child effects requires no reconciler. Generic-effect instantiations whose `observed:` block declares host-written channels but lack a registered reconciler are detected at startup and cause the runtime to refuse the live transition. (§13.14.7)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3259-3259`
    > 027-81. Reconciler registration is required if and only if the effect's `observed:` block declares host-written channels (`signal`/`stream`); an interior effect whose contract is fulfilled entirely by child effects requires no reconciler. If the program declares an effect type whose `observed:` block has host-written channels and no reconciler is registered, startup fails with a diagnostic naming the effect type and the runtime does not enter the live state. (§13.14.7)

#### F180 — 019-46 references a `default:` arm for `match`, but the match sections define the catch-all only as `_:` or a bare identifier — `default:` is an undefined match construct at point of use.

- **Severity/Category/Verdict:** LOW / undefined_term / CONFIRMED
- **Anchors:** LOG 019-46 · SPEC § 6.2.4, 6.2.5
- **Why it is a defect:** 019-46 says a pairs-form match may 'carry a `default:` arm', invoking `default:` as if it were the established match catch-all form. But the cited match sections (§6.2.4, §6.2.5) define the catch-all exclusively as `_:` or a bare identifier — no `default:` arm exists in the language's `match`. The load-bearing term `default:` has no pinned meaning at point of use, and the SPEC pairs-form mirror (line 15996) repeats it.
- **Direction of change:** Replace `default:` with the defined catch-all spelling (`_:` or bare identifier) in 019-46 and the SPEC mirror, or, if a distinct `default:` arm is genuinely intended, surface to the user that a new match construct would need defining in §6.2.x.
- **Evidence check:** pass — 019-46 invokes a `default:` match arm that the match sections never define.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2662-2662`
    > 019-46. A pairs-form `match pair:` must be exhaustive over the declared pairs (or carry a `default:` arm), as any `match` (§6.2.4) and as the cartesian form requires (§13.6.1.2). (§13.6.1.3)
  - `packages/ductus-lang/docs/SPEC.md:5330-5331`
    > A catch-all arm (`_:` or a bare identifier) covers all remaining variants
    > and makes the match trivially exhaustive. Users may opt into this when
  - `packages/ductus-lang/docs/SPEC.md:5314-5315`
    > Wildcard patterns (`_`) match without binding. Catch-all patterns (a bare
    > identifier with no constructor) match any value and bind it.

#### F179 — 019-46 (and its SPEC mirror) cite §6.2.4 for `match` exhaustiveness, but §6.2.4 covers pattern forms only; exhaustiveness is defined in §6.2.5 — the pointer contract is broken.

- **Severity/Category/Verdict:** LOW / divergence / CONFIRMED
- **Anchors:** LOG 019-46 · SPEC § 6.2.4, 6.2.5, 13.6.1.3
- **Why it is a defect:** 019-46 anchors the exhaustiveness requirement to §6.2.4, but §6.2.4 (Pattern matching) elaborates only pattern forms, nesting, wildcards, and catch-all — it says nothing about exhaustiveness. Exhaustiveness lives in §6.2.5 (Exhaustiveness checking). The LOG->SPEC pointer points to a section that does not elaborate the cited claim; the SPEC pairs-form text (line 15996) repeats the same misdirected `(§6.2.4)`.
- **Direction of change:** Retarget the exhaustiveness citation in 019-46 (and the SPEC 13.6.1.3 mirror) to §6.2.5 rather than §6.2.4.
- **Evidence check:** pass — 019-46 anchors exhaustiveness to §6.2.4, but §6.2.4 is pattern forms; exhaustiveness is §6.2.5.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2662-2662`
    > 019-46. A pairs-form `match pair:` must be exhaustive over the declared pairs (or carry a `default:` arm), as any `match` (§6.2.4) and as the cartesian form requires (§13.6.1.2). (§13.6.1.3)
  - `packages/ductus-lang/docs/SPEC.md:5254-5257`
    > #### 6.2.4 Pattern matching
    > 
    > The `match` expression is the canonical way to consume an enum value. Each arm specifies a pattern and an
    > expression:
  - `packages/ductus-lang/docs/SPEC.md:5317-5322`
    > #### 6.2.5 Exhaustiveness checking
    > 
    > A `match` expression must be exhaustive: every possible variant of the
    > matched enum (and every combination, for compound matches) must be covered
    > by some arm. The compiler verifies exhaustiveness at compile time. A
    > non-exhaustive match is a compile error identifying which variants are

#### F175 — 021-141 attributes 'each interpretation root's renders mount once at startup at stable paths' to §13.14.1, which contains no interpretation-root/render/stable-path language.

- **Severity/Category/Verdict:** LOW / divergence / CONFIRMED
- **Anchors:** LOG 021-141 · SPEC § 13.14.1
- **Why it is a defect:** Citation-aptness: 021-141 cites §13.14.1 for the claim that 'each interpretation root's renders mount once at startup at stable paths'. §13.14.1 (Lifecycle) describes cell allocation, closure init, commit, and teardown but nowhere mentions interpretation roots, renders, or stable paths. The cited section does not elaborate the pointed-at claim; the pointer contract is broken for that clause.
- **Direction of change:** Re-point the interpretation-root/render/stable-path clause of 021-141 to the section that actually defines interpretation-root render mounting, or add that content to §13.14.1; surface to user.
- **Evidence check:** pass — 021-141 cites §13.14.1 for interpretation-root/render/stable-path claim that §13.14.1 does not elaborate.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2880-2880`
    > 021-141. The entry-point's transitive closure ... defines the live graph: cells are allocated, instances are mounted, connections are engaged, and effects are reconciled for the entry-point and every reachable instance — and only those; each interpretation root's renders mount once at startup at stable paths within this closure. (§13.8.1, §13.14.1)
  - `packages/ductus-lang/docs/SPEC.md:18885-18896`
    > #### 13.14.1 Lifecycle
    > 
    > The runtime's lifecycle proceeds in phases:
    > 
    > **Startup:**
    > 
    > 1. Load the IR (per §15.4).
    > 2. Allocate cell storage (§14.3).
    > 3. Locate the entry-point node instance (the `main` placement,
    >    §13.8.1) and compute its **transitive closure**: the entry-point's
    >    own subtree plus everything reachable through connections and
    >    through module-level `Handle`/`WeakHandle` references.

#### F174 — 021-16's second sentence generalizes the implicit-derived-bridge to operator arguments, effect arguments, and a `|>` LHS and cites §13.8.2.1, which elaborates only the attr-RHS case.

- **Severity/Category/Verdict:** LOW / divergence / CONFIRMED
- **Anchors:** LOG 021-16 · SPEC § 13.8.2.1
- **Why it is a defect:** Citation-aptness (pointer contract): the entry's first sentence (attr-RHS bridge) is elaborated by §13.8.2.1, but the second sentence generalizes the mechanism to 'operator arguments, effect arguments, and a `|>` LHS' and cites the same §13.8.2.1, which discusses only the attribute-RHS case and never mentions operator arguments, effect arguments, or `|>` LHS. The pointer promises elaboration the section does not deliver for the generalized claim.
- **Direction of change:** Either extend §13.8.2.1 to cover the operator/effect/`|>`-LHS generalization or re-point that half of 021-16 to the section that actually elaborates it; surface to user.
- **Evidence check:** pass — 021-16's generalized bridge (operator/effect args, |> LHS) cites §13.8.2.1 which only elaborates the attr-RHS case.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2755-2755`
    > 021-16. A placement attr RHS that references reactive cells (signals, attrs, recurrents, deriveds) creates an implicit derived bridging the source cells to the target attr, so the attr reactively tracks the sources: `Display as d1 | label=format(c1.count)`. The same implicit-derived-bridge mechanism applies to any reactive expression occupying a cell position beyond the attr RHS — operator arguments, effect arguments, and a `|>` LHS — where a reactive-provenance expression synthesizes an implicit derived bridging its source cells to that position. (§13.8.2.1)
  - `packages/ductus-lang/docs/SPEC.md:16521-16542`
    > ##### 13.8.2.1 Reactive vs. compile-time placement values
    > 
    > The right-hand side of an attribute setting at placement may be:
    > 
    > - A **compile-time / value expression** ... — a literal, a `const`
    >   reference ... whose provenance contains no reactive cell. ...
    > - A **reactive expression** — references reactive cells (signals,
    >   attrs, recurrents, deriveds) visible at the placement scope ... The placement creates
    >   an implicit `derived` bridging the source cells to the target
    >   attr ...

#### F169 — 015-39 states the reconciler-registration canon with ', and' joining the two clauses, while the six other canon sites use a semicolon; verbatim drift within a self-declared identical-restatement canon.

- **Severity/Category/Verdict:** LOW / divergence / CONFIRMED
- **Anchors:** LOG 015-39, 027-80, 027-81, 031-128, 031-143, 031-157, 033-124 · SPEC § 13.1
- **Why it is a defect:** The condition ('required if and only if the effect's `observed:` block declares host-written channels (`signal`/`stream`)') and the interior-effect exemption are semantically identical across all seven sites, so there is no behavioral divergence. But 015-39 joins the two clauses with ', and' where 027-80/027-81/031-128/031-143/031-157/033-124 use '; '. Since Invariant 2 requires each entry to restate rather than reference, drift in a restated canon is a low-grade maintenance defect: a future edit touching one form and not the others could silently split the canon. Flagging as terminology/style drift only.
- **Direction of change:** Surface to user whether the seven canon sites should be normalized to one verbatim clause form; do not edit.
- **Evidence check:** pass — 015-39 joins the reconciler-registration canon clauses with ', and' while the other six sites use a semicolon.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1848-1848`
    > 015-39. A new effect type is added by declaring an `effect`; reconciler registration is required if and only if the effect's `observed:` block declares host-written channels (`signal`/`stream`), and an interior effect whose contract is fulfilled entirely by child effects requires no reconciler. (§13.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3258-3258`
    > 027-80. Reconciler registration is required if and only if the effect's `observed:` block declares host-written channels (`signal`/`stream`); an interior effect whose contract is fulfilled entirely by child effects requires no reconciler. Generic-effect instantiations whose `observed:` block declares host-written channels but lack a registered reconciler are detected at startup and cause the runtime to refuse the live transition. (§13.14.7)

#### F135 — Weaker overlap: 'node/connection type carried as a value via Type[…]' restated in 005-185, 016-223, 017-167, 019-21, 031-6 — near-duplicate facts, each with a thin per-construct rider.

- **Severity/Category/Verdict:** LOW / design_smell / CONFIRMED
- **Anchors:** LOG 005-185, 016-223, 017-167, 019-21, 031-6 · SPEC § 3.7.4, 13.2.10, 13.3.6.1, 13.6.0, 13.19
- **Why it is a defect:** 005-185 already states the general fact for node/connection/effect types. 016-223, 017-167, 019-21, 031-6 each restate the same 'type carried as value via Type[…]' fact for a subset, with a small construct-specific rider. The general entry 005-185 makes the per-construct repetitions partly redundant. Lower confidence than the other families because each restatement carries a genuinely distinct rider (attr slot mechanism, storable designation vs Handle, template slot, effect). They agree, so redundancy not contradiction.
- **Direction of change:** Consider whether 005-185's general statement subsumes the bare 'carried via Type[…]' clause in the per-construct entries; if so, trim those entries to only their distinct riders.
- **Evidence check:** pass — 'node/connection/effect type carried as value via Type[…]' restated in 005-185, 016-223, 017-167, 019-21, 031-6 — near-duplicate facts each with a thin per-construct rider.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:526-526`
    > 005-185. A node/connection/effect type in value position is carried by the `Type[…]` meta-type. (§3.7.4)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2076-2076`
    > 016-223. A node or connection type is carried as a value via the `Type[…]` meta-type, the mechanism for an attr template slot deferring which kind a receiver places. (§13.2.10)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3775-3775`
    > 031-6. An effect type (as opposed to an instance) is carried as a value via `Type[…]`. (§13.19)

#### F134 — Entry-match bounded-pattern carve-out restated three times: 006-27, 009-91, 017-221 all say 'Variant(name: Bound)' is legal only in .exposition/entry matches, banned over records.

- **Severity/Category/Verdict:** LOW / design_smell / CONFIRMED
- **Anchors:** LOG 006-27, 009-91, 017-221 · SPEC § 3.5.7, 6.2.4, 13.3.7
- **Why it is a defect:** The same carve-out — bounded pattern Variant(name: Bound) permitted only in .exposition/entry matches, banned as a general pattern shape / over records — is stated three times in three sections. The rule is atomic and needed once; the three copies are decision decay. They agree, so redundancy not contradiction.
- **Direction of change:** Consolidate the carve-out into one authoritative entry and drop the restatements in the other sections, or reduce the duplicates to cross-topic pointers that do not re-encode the full rule.
- **Evidence check:** pass — Entry-match bounded-pattern carve-out 'Variant(name: Bound)' restated in 006-27, 009-91, 017-221 — legal only in .exposition/entry matches, banned over records; redundancy, they agree.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:609-609`
    > 006-27. Variant patterns may be positional or named, parallel to variant construction; as an entry-match-only carve-out, inside an `.exposition` entry match (and only there) a variant payload may bind a typed bound `Variant(name: Bound)` where `Bound` is a trait or concrete type, which is not a general pattern shape and is unavailable at ordinary variant-pattern sites. (§3.5.7)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1028-1028`
    > 009-91. A trait-headed `match` over a record value is a compile error: a bound in match-payload position — `Variant(name: Bound)` where `Bound` is a trait or concrete type — is legal only in a `match` over `.exposition` entries, never over ordinary records. (§6.2.4)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2362-2362`
    > 017-221. Entry matches admit a bounded pattern `Variant(name: Bound)`, where `Bound` is a trait or a concrete type. This is a carve-out scoped to entry matches only, not a general pattern shape; trait-headed `match` over records stays banned. (§13.3.7)

#### F133 — Node/connection/effect 'held only by borrow, never placed in cells or records' restated across 016-224, 017-170, 031-5 (and enumerated form in 001-25).

- **Severity/Category/Verdict:** LOW / design_smell / CONFIRMED
- **Anchors:** LOG 016-224, 017-170, 031-5 · SPEC § 13.2.10, 13.3.6.1, 13.19
- **Why it is a defect:** The core 'held by borrow, never placed in cells or records' rule appears three times, each nominally scoped to a construct (node/connection, connection/effect, effect) but with heavy overlap — 016-224 covers node+connection, 017-170 re-covers connection+effect, 031-5 re-covers effect. The overlapping coverage is restatement the atomicity design does not require. They agree, so redundancy not contradiction. Note each copy also carries a small construct-specific rider (Type[…] stand-in; effects: clause), which is the weak justification for keeping some separation.
- **Direction of change:** State the ownership rule once for all graph-member instances and let per-construct entries carry only their genuinely distinct riders, dropping the repeated 'held by borrow, never in cells or records' clause.
- **Evidence check:** pass — 'held only by borrow, never placed in cells or records' restated across 016-224/017-170/031-5 with overlapping coverage.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2077-2077`
    > 016-224. Node and connection instances are first-class citizens held only by borrow, never placed in cells or records; the storable stand-in for *which kind* to place is the type value `Type[…]`. (§13.2.10)
  - `packages/ductus-lang/docs/DECISION_LOG.md:2311-2311`
    > 017-170. Connection and effect instances obey the same ownership rule as nodes: brought in only by the language's placement/instantiation syntax and held only by borrow, never placed in cells or records; their types travel as values via `Type[…]`. (§13.3.6.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3774-3774`
    > 031-5. Like nodes and connections, an effect instance is a graph member: instantiated only in a node's `effects:` clause and held by borrow, never placed in cells or records. (§13.19)

#### F132 — Borrow-not-storable slot list stated verbatim twice: 001-25 and 013-52 both enumerate 'record field, tuple component, enum payload, or indexed slot'.

- **Severity/Category/Verdict:** LOW / design_smell / CONFIRMED
- **Anchors:** LOG 001-25, 013-52 · SPEC § 1.3, 11.3.4
- **Why it is a defect:** The trailing clause of 001-25 and the whole of 013-52 assert the identical prohibition with the identical four-slot enumeration. One decision restated in two sections. They agree (no contradiction), so this is decay/redundancy, not a soundness issue.
- **Direction of change:** Keep the storage-prohibition rule in one location and have the other reference the concept without re-enumerating the slot list, or scope 013-52 to the borrow-equivalent-alias specific case only.
- **Evidence check:** pass — Same borrow-not-storable prohibition with identical four-slot enumeration restated in 001-25 and 013-52 — redundancy, they agree.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:55-55`
    > 001-25. Node, connection, and effect instances are first-class citizens; a binding to one is a borrow, and a borrow cannot be stored in a record field, tuple component, enum payload, or indexed slot (§13.3.6.1). (§1.3)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1438-1438`
    > 013-52. A borrow-equivalent alias cannot be stored in a record field, tuple component, enum payload, or indexed slot. (§11.3.4)

#### F131 — Grammar-authority rule restated three times (001-3, 001-4, 002-29) plus partial 001-1: 'single authoritative source / no separate grammar document' — atomicity did not require restatement.

- **Severity/Category/Verdict:** LOW / design_smell / CONFIRMED
- **Anchors:** LOG 001-3, 001-4, 002-29 · SPEC § 1.1, 1.4
- **Why it is a defect:** 001-3 and 002-29 assert verbatim the same 'no separate grammar document' fact; 001-4 asserts 'single authoritative source for syntax and semantics'; 001-1 asserts 'authoritative source' for a different member list. Three (arguably four) entries encode one decision. The LOG's atomicity design means the rule need be stated once; the copies are decision decay across §1.1 and §1.4. They agree, so no contradiction.
- **Direction of change:** Consolidate the grammar-authority assertion into a single entry and remove the redundant restatements, or scope each to a genuinely distinct facet.
- **Evidence check:** pass — 'no separate grammar document / single authoritative source' restated across 001-3, 001-4, 002-29 (and partial 001-1) — redundancy, they agree.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:33-34`
    > 001-3. Lexical and syntactic structure is specified directly in this specification and decision log; there is no separate grammar document. (§1.1)
    > 001-4. This specification is the single authoritative source for the language's syntax and semantics. (§1.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:100-100`
    > 002-29. The language's syntax is specified directly in this log and SPEC.md; there is no separate grammar document and no normative content is delegated to one. (§1.4)
  - `packages/ductus-lang/docs/DECISION_LOG.md:31-31`
    > 001-1. The specification is the authoritative source for the type system, evaluation model, and runtime semantics. (§1.1)

#### F092 — The reconciler-required-iff conditional is restated in full at seven LOG sites plus two SPEC sites; even while consistent it is a maintenance smell (⚛A) — a single edit to the rule must be mirrored across nine places.

- **Severity/Category/Verdict:** LOW / design_smell / CONFIRMED
- **Anchors:** LOG 027-80, 027-81, 031-119, 031-128, 031-143, 031-157, 033-124 · SPEC § 13.14.7, 13.19.14
- **Why it is a defect:** The clause 'Reconciler registration is required if and only if the effect's `observed:` block declares host-written channels (`signal`/`stream`); an interior effect whose contract is fulfilled entirely by child effects requires no reconciler' appears verbatim in 027-80, 027-81, 031-128, 031-143, 031-157 and in near-verbatim form in 015-39, 031-119, 033-124 — plus SPEC 13.14.7 and 13.19.14. This is the ⚛A smell: a normative condition restated at ~9 sites. Under LOG Invariant 2 (atomic, self-contained entries) restatement is required, so it is not itself a rule violation — but it is a real maintenance hazard: any refinement of the condition (e.g. a third host-written channel kind, or a change to the interior-effect carve-out) must be edited identically in every site or the LOG silently diverges. Additionally 027-80 and 027-81 are near-duplicates of each other (both encode the startup-refuse-on-missing-reconciler behavior; 027-80 framed for generic effects, 027-81 for the general case) — candidate redundancy within a single section.
- **Direction of change:** Not an implementer blocker; surface as a maintenance-risk observation. If desired, consider whether 027-80 and 027-81 can be merged (they encode the same startup-refuse outcome) without violating atomicity, and note the 9-site restatement as a known consistency-maintenance cost. No content decision implied — user's call.
- **Evidence check:** pass — The reconciler-required-iff clause is restated verbatim across many sites, a maintenance smell if edited non-uniformly.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:3258-3259`
    > 027-80. Reconciler registration is required if and only if the effect's `observed:` block declares host-written channels (`signal`/`stream`); an interior effect whose contract is fulfilled entirely by child effects requires no reconciler. Generic-effect instantiations whose `observed:` block declares host-written channels but lack a registered reconciler are detected at startup and cause the runtime to refuse the live transition. (§13.14.7)
    > 027-81. Reconciler registration is required if and only if the effect's `observed:` block declares host-written channels (`signal`/`stream`); an interior effect whose contract is fulfilled entirely by child effects requires no reconciler. If the program declares an effect type whose `observed:` block has host-written channels and no reconciler is registered for it, startup fails with a diagnostic naming the effect type and the runtime does not enter the live state. (§13.14.7)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3897-3897`
    > 031-128. Reconciler registration is required if and only if the effect's `observed:` block declares host-written channels (`signal`/`stream`); an interior effect whose contract is fulfilled entirely by child effects requires no reconciler. An effect type that requires registration but appears in the graph with no registered reconciler triggers a startup diagnostic, and the runtime refuses to enter the live state. (§13.19.14)

#### F043 — 013-79 (a §11.8.5 move-position rule) is wedged among §11.4 Copy-trait entries (between 013-78 and 013-80), breaking by-topic ordering.

- **Severity/Category/Verdict:** LOW / design_smell / CONFIRMED
- **Anchors:** LOG 013-79, 013-78, 013-80 · SPEC § 11.8.5, 11.4, 11.4.1
- **Why it is a defect:** The LOG orders entries by topic (generic→specific). 013-78 and 013-80 both carry §11.4/§11.4.1 (Copy trait). 013-79 carries §11.8.5 and concerns the move keyword's l-value operand — a topic elaborated by the §11.8.5 cluster at 013-135..013-142. Wedged at position 79 it interrupts the Copy-trait run, so a reader scanning the §11.4 block hits an unrelated move-syntax rule. This is an ordering/placement smell from an insert, not a semantic contradiction.
- **Direction of change:** Consider relocating the move-l-value-operand rule to sit with the other §11.8.5 move-position entries; because dense positional numbering (Invariant 1) means relocation renumbers, surface to user rather than moving unilaterally.
- **Evidence check:** pass — 013-79 (§11.8.5 move rule) wedged between §11.4 Copy entries, breaking by-topic ordering.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:1464-1464`
    > 013-78. A generic type conditionally opts into a methodless marker like `Copy` via a `where` clause on its `satisfies` clause: `type Pair[T]:` with `satisfies Copy where T: Copy` makes `Pair[T]` `Copy` exactly for instantiations where `T: Copy` — the same conditional pattern the stdlib uses for `Range[T]` (§11.4.1). No `fulfill` block is written, `Copy` being methodless. (§11.4)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1465-1465`
    > 013-79. The `move` keyword's operand may be a field-access l-value path rooted in an owned binding — `move value.handle`, `move rec.a.b` — not only a bare identifier; this performs a partial move (§14.7.3), consuming that field while the rest of the binding stays live, and is the sole exception to field access reading without ownership transfer (§11.8.3). The root must be owned (an `own` parameter or owning local), and a method-call operand stays forbidden (`move x.f()` ✗ — a call is not an l-value). (§11.8.5)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1466-1466`
    > 013-80. All primitive numeric types (`i8`–`i128`, `u8`–`u128`, `isize`, `usize`, `f32`, `f64`), `bool`, and `char` automatically implement `Copy`. (§11.4.1)

#### F009 — 001-28 states the compile-time criterion as 'compile-time evaluable' while the defined term used by 034-7 and 004-64..004-91 is 'compile-time-known'; two coexisting phrasings for one concept.

- **Severity/Category/Verdict:** LOW / vague_term / CONFIRMED
- **Anchors:** LOG 001-28, 034-7, 004-64, 004-90 · SPEC § 1.3, 
2.4.1, 2.4.2, 13.20
- **Why it is a defect:** 001-28 uses 'compile-time evaluable' (also in 004-99), while the term defined and used elsewhere (004-64..004-91, 034-7) is 'compile-time-known'. Substantively the criteria match — 004-90/091 negate on signals/external I/O, matching 001-28's 'not involving a signal or external input' — so this is not behavior-changing, but the two phrasings are never explicitly equated, leaving a reader to assume they are synonyms. Load-bearing because 034-7's legality gate for `yield` under `if`/`match` depends on the criterion being identical to 001-28's.
- **Direction of change:** Surface to user: decide whether to unify on one term ('compile-time-known') or add an explicit equivalence statement; do not resolve unilaterally.
- **Evidence check:** pass — 001-28 says 'compile-time evaluable', 034-7/004-64..91 say 'compile-time-known'; two phrasings for one concept never explicitly equated.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:58-58`
    > 001-28. Any expression not involving a signal or external input is compile-time evaluable. (§1.3)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4358-4358`
    > 034-7. A `yield` placed under a value `if` or `match` inside a `collect` is legal only when the condition or scrutinee is compile-time-known, in which case the conditional is expanded rather than gating structure; when the condition or scrutinee is reactive or runtime-valued it is a compile error whose diagnostic offers `when:`/`given` arms for membership switching or a conditional value yield of the form `yield if c: a else: b`, since `if`/`match` never gate structure. (§13.20)
  - `packages/ductus-lang/docs/DECISION_LOG.md:273-273`
    > 004-90. Expressions involving signals or any value derived from a signal are not compile-time known. (§2.4.2)

#### F006 — SPEC §10.4.1 grounds the general (all-names) glob-collision rule on §6.2.3, whose actual scope is enum-variant imports only, weakening the elaboration pointer.

- **Severity/Category/Verdict:** LOW / divergence / CONFIRMED
- **Anchors:** LOG 003-45, 003-47 · SPEC § 10.4.1
- **Why it is a defect:** 003-45/47 are general use-path rules (any importable name), cited to §10.4.1. But §10.4.1 delegates both the parentheses convention and the glob-collision rule to §6.2.3, and §6.2.3 is titled 'Variant construction and resolution' and its collision discussion is framed around enum variants (`use Direction::*` vs `use Heading::*`). The general import-collision rule therefore rests on an enum-specific section, so the reader chasing the SPEC cross-reference lands on narrower material than the LOG rule asserts. LOG-to-SPEC pointer (§10.4.1) is intact, so this is a SPEC-internal over-narrow cross-reference, not a broken LOG pointer — hence LOW.
- **Direction of change:** Give §10.4.1 a self-contained statement of the parentheses convention and the general glob-collision rule (not delegated to §6.2.3), or make §6.2.3 explicitly the general rule that enum variants are one instance of.
- **Evidence check:** pass — §10.4.1 grounds the general glob-collision rule on §6.2.3 whose scope is enum-variant imports only, weakening the elaboration pointer for general use-path rules 003-45/47.
- **Evidence:**
  - `packages/ductus-lang/docs/SPEC.md:7769-7770`
    > Per §6.2.3, selection lists on `use` paths use parentheses; a glob
    > imports every visible name from the source:
  - `packages/ductus-lang/docs/SPEC.md:5184-5184`
    > #### 6.2.3 Variant construction and resolution
  - `packages/ductus-lang/docs/SPEC.md:5219-5220`
    > Unqualified variant names are not available by default. To bring variants
    > into scope unqualified, the user explicitly imports them via `use`:

### Unthemed (Director did not assign; listed for completeness)

#### F224 — 031-138 (and 031-7/031-15) say an effect body consists only of desired:/observed: blocks, but 031-154/156 permit top-of-body child-effect placements as a third body item.

- **Severity/Category/Verdict:** MED / contradiction / CONFIRMED
- **Anchors:** LOG 031-138, 031-154, 031-156, 031-7, 031-15 · SPEC § 13.19.15, 13.19.3.1
- **Why it is a defect:** The composable-effects amendment added top-of-body child-effect placements (031-154/155/156, 031-134 revised, SPEC 13.19.3.1). But three pre-amendment framing entries were not conformed: 031-138 ('consists only of the desired: and observed: blocks'), 031-7 ('consists of two record-shaped blocks'), and 031-15 ('Reactive declarations other than the five role-keyword cell forms ... cannot appear inside an effect's body'). A child-effect placement is a reactive instantiation appearing in the effect body that is neither a role-keyword cell nor a desired/observed block, so 031-15 and 031-138 as written forbid exactly what 031-154 permits. SPEC 13.19.15 (lines 22760-22766) was conformed ('optional top-of-body child-effect placements ... followed by the desired: and observed: blocks'), leaving these LOG entries stale relative to their own SPEC.
- **Direction of change:** Conform 031-138, 031-7, and 031-15 to admit top-of-body child-effect placements as a body item alongside the desired:/observed: blocks, matching 031-154/156 and SPEC 13.19.15.
- **Evidence check:** pass — 031-154/156 permit top-of-body child-effect placements as a third body item and SPEC 13.19.15 was conformed to allow them, but pre-amendment framing entries 031-138/031-7/031-15 still assert the body consists ONLY of desired:/observed: blocks and forbid non-role-keyword declarations, forbidding exactly what 031-154 permits.
- **Charity check:** sustain — The composable-effects amendment added top-of-body child-effect placements as a legitimate third body item (031-154/156; SPEC 13.19.15 lines 22760-22766 conformed: body 'consists only of optional top-of-body child-effect placements ... followed by the desired: and observed: blocks'). But 031-138 ('body consists only of the desired: and observed: blocks'), 031-7 ('two record-shaped blocks'), and 031-15 (nothing but the five role-keyword cell forms) were left stale — as written they forbid the child placements 031-154 permits and SPEC 13.19.15 admits. No reconciling qualifier in these three entries. Sustained as MED contradiction/LOG-SPEC divergence.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:3907-3907`
    > 031-138. An effect's body consists only of the `desired:` and `observed:` blocks containing role-keyword cell declarations. (§13.19.15)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3923-3923`
    > 031-154. An effect body may place child effects as top-of-body items before its `desired:` and `observed:` blocks; such a placement is a graph placement, not a cell expression: `x = source |> child_effect`. (§13.19.15)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3776-3776`
    > 031-7. An effect declaration consists of two record-shaped blocks, `desired:` and `observed:`. (§13.19.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3784-3784`
    > 031-15. Reactive declarations other than the five role-keyword cell forms (e.g. `attr`, top-level `signal`) cannot appear inside an effect's body. (§13.19.2)

#### F202 — 032-169 classifies 'cell type changes (remove+add)' as reload-safe/no-restart while 032-168 classifies operator-internal cell type changes as per-instance-restart (reload-unsafe per 032-166); the two overlap because neither entry pins 'internal' vs top-level cell type change.

- **Severity/Category/Verdict:** MED / ambiguity / CONFIRMED
- **Anchors:** LOG 032-166, 032-168, 032-169 · SPEC § 14.8.3
- **Why it is a defect:** Reading 1: 032-169's bare 'cell type changes' means all cells, so a cell type change is always reload-safe (remove+add) — an operator-internal cell type change needs no restart. Reading 2: 032-168 governs operator-internal cell type changes (per-instance restart, hence reload-unsafe per 032-166) and 032-169 governs only top-level cells. The two readings yield different runtime behavior for the same edit (changing the type of a cell declared inside an operator body): silent remove+add vs a per-instance restart of every instance of that operator. 032-169 says 'All other changes' but does not exclude operator-internal cells, and the word 'internal' that would disambiguate appears only in 032-168, not in 032-169. An implementer cannot tell which remedy applies to an operator-internal cell type change from these atomic entries.
- **Direction of change:** Disambiguate 032-169's 'cell type changes' scope to explicitly exclude operator-internal cells already governed by 032-168 (or state the intended precedence between the two entries) — surface to user, do not resolve unilaterally.
- **Evidence check:** pass — For an operator-internal cell type change, 032-168 (per-instance restart, hence reload-unsafe) and 032-169 (cell type changes are reload-safe, no restart) overlap; neither atomic entry pins 'internal' vs top-level, so the concrete remedy is undetermined.
- **Charity check:** sustain — Confirmed the overlap is live in both LOG and SPEC. 032-168 routes 'internal cell type changes per §13.17.10' to per-instance restart (reload-unsafe per 032-166); 032-169's unqualified 'cell type changes (remove + add per §13.15.2)' routes to reload-safe/no-restart. Checked whether the §13.15.2 cite forces a top-level-only reading of 032-169: it does NOT — §13.15.2 (SPEC 19291-19314) identifies cells by fully-qualified declaration path including 'instance name', so it covers operator-internal cells too. The disambiguation lives only in §13.15.4 (SPEC 19377, 'Internal cell type changes within an operator body' → per-instance), a finer split absent from the atomic entries 032-168/169 themselves. Per the ambiguity standard, a reader must infer the resolution by consulting §13.15.4 — inference-required is a sustain, and Invariant 2 requires the atomic entries to be self-contained. The dissolving §13.15.4 text refines rather than conflicts with 032-169, so refile is not warranted; the LOG-level ambiguity stands. Sustained MED ambiguity.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:4098-4099`
    > 032-168. Operator-specific changes (operator signature changes, internal cell type changes per §13.17.10) need only per-instance restart: only the affected operator instances are recreated. (§14.8.3)
    > 032-169. All other changes — cell removal (compile-gate-verified unreferenced), cell type changes (remove + add per §13.15.2), connection topology changes (remove + add per §13.15.2) — are reload-safe and need no restart. (§14.8.3)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4096-4096`
    > 032-166. Reload-unsafe changes fall into two classes per §13.15.4: those requiring full-runtime restart and those needing only per-instance restart. (§14.8.3)

#### F089 — Startup sequence (027-16..027-28 / SPEC 13.14.1) never fires the reconciler `create` hook for the initial cohort of effect instances, yet `update` needs the create-produced instance state.

- **Severity/Category/Verdict:** MED / gap / CONFIRMED
- **Anchors:** LOG 027-16, 027-28, 027-69, 027-70, 027-96 · SPEC § 13.14.1, 13.14.7, 13.14.9
- **Why it is a defect:** The startup steps 027-16 through 027-28 (SPEC 13.14.1 steps 1-4) enumerate: load IR, allocate storage, initialize cells + evaluate when-predicates, first commit. None of them fires `create` for the effect instances that are live at startup. 027-96 places all reconciler hooks 'at the commit boundary … after the commit publishes its snapshot', which would push the initial `create` to after the first commit — but the runtime is only 'live' after the first commit (027-29), so there is no defined moment at which the initial cohort's `create` runs and produces the instance state that every subsequent `update` (027-70) requires. An implementer cannot tell whether initial effects are created during construction (before first commit) or at the first commit boundary, and observers reading effect `observed:` cells right after go-live may see a state that no reconciler has yet initialized. No other section supplies the initial-create step (grep of DECISION_LOG/SPEC for initial/first-commit create found none).
- **Direction of change:** Add a startup rule pinning when the initial cohort of effect instances receives `create` (e.g. as part of construction before the first commit, or as the first commit-boundary hook pass), so instance state exists before any `update`. Direction only.
- **Evidence check:** pass — Is there a defined moment for the initial cohort's create hook? Startup steps enumerate no create; 027-96 pushes hooks past the first commit but 027-29 says runtime is live only after it — no window is named for initial create. Evidence supports an implementer-blocking sequencing gap.
- **Charity check:** sustain — The startup steps 027-16..027-28 (DECISION_LOG.md:3194-3206; SPEC §13.14.1 steps 1-4, SPEC.md:18918-18919) enumerate load IR, allocate storage, initialize cells + evaluate when-predicates, and first commit — none fires `create` for the effect instances live at startup. The only candidate dissolver requires stitching three rules: 027-69 'create fires when a new effect instance enters the live graph' (DECISION_LOG.md:3247), 027-29 'live only after the first commit completes' (DECISION_LOG.md:3207), and 027-96 'hooks fire after the commit publishes its snapshot' (DECISION_LOG.md:3274). That stitch is INFERENCE, and 027-69's 'a NEW effect instance enters the live graph' is itself ambiguous as to whether the initial startup population counts as 'new instances entering' or only steady-state late-arriving instances. No rule explicitly names an initial-create step, so per the gap standard (inference/obviously-intended = SUSTAIN) the gap survives; the inference-required-ness is the finding. SUSTAIN.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:3206-3207`
    > 027-28. The final startup step performs the first commit, publishing the initial snapshot; observers' subsequent `acquire_snapshot` calls return real data. (§13.14.1)
    > 027-29. The runtime is "constructing" until the first commit and "live" only after it completes. (§13.14.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3247-3248`
    > 027-69. The *create* hook fires when a new effect instance enters the live graph; it receives the instance ID, current parameter values, and initial `desired:` values, and returns opaque reconciler-side instance state. (§13.14.7)
    > 027-70. The *update* hook fires when any parameter or `desired:` cell of an existing instance becomes dirty; it receives the instance ID, the instance state, and current parameter/desired values. (§13.14.7)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3274-3274`
    > 027-96. Reconciler hooks fire at the commit boundary: after the commit publishes its snapshot and before the next commit begins. (§13.14.9)

#### F090 — The 10-step reload sequence (028-13..028-24 / SPEC 13.15.3) exposes no pre-live window in which the host can call `register_reconciler`, yet 027-67 and 028-62 require registering a reconciler for a new/renamed effect type before the reload goes live.

- **Severity/Category/Verdict:** MED / gap / CONFIRMED
- **Anchors:** LOG 028-13, 028-23, 028-24, 028-62, 027-67 · SPEC § 13.15.3, 13.15.6, 13.14.7
- **Why it is a defect:** 027-67 and SPEC 13.19.16/13.15.6 assert a 'hot reload's pre-live phase before the reload reaches the live state' during which the host must register a reconciler for any newly-appearing or renamed effect type (028-62: new-type instances constructed fresh; those with host-written observed channels require a reconciler per 027-80/81). But the reload procedure 028-13 through 028-24 is a closed 10-step atomic sequence: acquire lock (step 2/028-15), drain commit, diff, allocate, drop, update behavior table, recompute deriveds, commit (step 9/028-23), release lock (step 10/028-24). No step yields control to the host to call `register_reconciler`, and steps 2-9 run under the reload lock which pauses host writes. An implementer has nowhere to place the host's registration call, and no rule says the reload aborts (like startup does) when a required reconciler is missing after the diff. The bridge phrase 'pre-live phase' names a window the sequence never opens.
- **Direction of change:** Either add an explicit host-registration window to the reload sequence (a step, after the diff identifies new/renamed effect types requiring reconcilers and before the reloaded commit, where the host registers them) and a reject-on-missing rule mirroring startup; or restate where in steps 1-10 the pre-live registration occurs. Direction only.
- **Evidence check:** pass — Does the reload sequence (028-13..028-24) open the pre-live host window 027-67/028-62/SPEC 19511 require for register_reconciler? Steps run under the reload lock (028-15..028-23) with no yield to host; step 10 releases only after commit goes live. Evidence supports a named-but-never-opened window gap.
- **Charity check:** sustain — The reload sequence §13.15.3 (SPEC.md:19322-19355) / 028-13..028-24 (DECISION_LOG.md:3314-3325) is a closed sequence performed 'atomically on the runtime's driving context'. New/renamed effect types become known only at step 4 (diff computed), and go-live occurs at step 9 (commit); steps 2-9 run under the reload lock (step 2 acquires it, step 10 releases). No step yields control to the host to call register_reconciler between step 4 and step 9. Registration before the reload(diff) call is rejected: 027-67 (DECISION_LOG.md:3245) / §13.14.7 (SPEC.md:19048-19052) reject registrations 'once live and outside a reload window', and the runtime is live/steady-state before reload(diff) is called. 028-62 (DECISION_LOG.md:3363) + 027-80/81 make a renamed/new effect type with host-written observed channels require a reconciler before go-live (§13.15.6, SPEC.md:19510-19512). Unlike startup — which explicitly refuses the live transition on a missing reconciler (027-80/027-81) — there is NO reload-abort-on-missing-reconciler rule (grep found none), and no callback-during-reload mechanism, and queued host requests apply only at step 10 (after go-live), too late. The bridge phrase 'pre-live phase' (027-67, §13.14.7, §13.15.6) names a window the sequence never opens. Implementer-blocking gap in a mandatory-core verb's timing contract, no legal boundary. SUSTAIN.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:3245-3245`
    > 027-67. `register_reconciler` must be called during a pre-live window — initial boot before the runtime goes live, or a hot reload's pre-live phase before the reload reaches the live state; registrations once live and outside a reload window are rejected. (§13.14.7)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3323-3325`
    > 028-23. Reload step 9 commits the reloaded state. (§13.15.3)
    > 028-24. Reload step 10 releases the reload lock, resumes signal/attr writes, and applies queued writes to the new state. (§13.15.3)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3363-3363`
    > 028-62. Renaming an effect type is removal of the old type plus addition of a new one: old instances are torn down and new-type instances are constructed fresh. (§13.15.6)
  - `packages/ductus-lang/docs/SPEC.md:19509-19512`
    > reconciler for the new effect type via `runtime.register_reconciler`
    >     before the reload reaches the live state.

#### F091 — No rule orders `create` before `update` when an effect instance both enters the live graph and has dirty desired/param cells in the same commit; `update` consumes the instance state that only `create` produces.

- **Severity/Category/Verdict:** MED / gap / CONFIRMED
- **Anchors:** LOG 027-69, 027-70, 027-91 · SPEC § 13.14.7, 13.14.9
- **Why it is a defect:** 027-91 lists `create` ('on scope entry') and `update` ('on dirty parameters/`desired:` during a commit') as independent triggers. When a `repeat` in an `effects:` clause adds a new element (or at the first commit for the initial cohort), the new instance both enters the live graph AND has dirty desired/parameter cells in that same commit. 031-124 enumerates 'effect instances whose parameters or desired cells became dirty during that commit' for update — which includes the just-created instance. But 027-70 says `update` 'receives … the instance state', and 027-69 says the instance state is what `create` returns. If `update` runs before `create` for such an instance, there is no instance state to pass — an unsatisfiable invocation. The corpus never states that `create` precedes `update` for a newly-live-and-dirty instance in the same commit. An implementer must invent the ordering.
- **Direction of change:** Add a rule guaranteeing that for an instance newly entering the live graph in a commit, `create` fires before any `update` for that instance in the same commit boundary (or that a newly-created instance is excluded from that commit's update pass). Direction only.
- **Evidence check:** pass — Is create ordered before update for an instance that enters live AND has dirty desired/param in the same commit? 031-124's enumeration includes the just-created instance; 027-70 update needs create-returned state. No rule states create precedes update here, so update-first is an unsatisfiable invocation. Evidence supports the gap.
- **Charity check:** sustain — No rule orders `create` before `update` when an instance BOTH newly enters the live graph AND has dirty desired/param cells in the same commit. 027-69 (DECISION_LOG.md:3247) makes create return the opaque instance state; 027-70 (DECISION_LOG.md:3248) makes update RECEIVE that instance state — so update-before-create is an unsatisfiable invocation (no state to pass). The candidate dissolver is the word 'existing' in 027-70 ('when any parameter or desired: cell of an EXISTING instance becomes dirty'), but 'existing' is a trigger qualifier, not a normative ORDERING rule, and it is in direct tension with 031-124 (DECISION_LOG.md:3893) 'After commit, the runtime enumerates effect instances whose parameters or desired cells became dirty during that commit' — which carries NO existing-qualifier and would enumerate a just-created-and-dirty instance for update. Since no explicit text orders create before update for the overlap case and inference is required, the gap survives. SUSTAIN.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:3269-3269`
    > 027-91. Hooks align with the instance lifecycle: `create` on scope entry, `teardown` on scope exit, `update` on dirty parameters/`desired:` during a commit, `suspend` on gate-off, `resume` on gate-on. (§13.14.9)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3893-3893`
    > 031-124. After commit, the runtime enumerates effect instances whose parameters or desired cells became dirty during that commit. (§13.19.14)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3247-3248`
    > 027-69. The *create* hook fires when a new effect instance enters the live graph; it receives the instance ID, current parameter values, and initial `desired:` values, and returns opaque reconciler-side instance state. (§13.14.7)
    > 027-70. The *update* hook fires when any parameter or `desired:` cell of an existing instance becomes dirty; it receives the instance ID, the instance state, and current parameter/desired values. (§13.14.7)

#### F108 — The reload-sequence step-9 commit ("Commit the reloaded state") never states whether it fires the reconciler update hook, though every ordinary commit is defined to fire reconcilers against dirtied effect instances.

- **Severity/Category/Verdict:** MED / gap / CONFIRMED
- **Anchors:** LOG 028-23, 027-32, 031-124, 031-125 · SPEC § 13.15.3, 13.14.9, 13.19.14, 13.16
- **Why it is a defect:** Reload step 8 dirties effect desired cells: it recomputes changed deriveds (028-21) and initializes added desired cells (028-8/028-18/031-82). The step-9 commit is described as an ordinary commit. The universal commit rule (027-32, 031-124/125, §13.16 map) says every commit fires reconciler hooks and enumerates instances dirtied during that commit. Nothing in §13.15.3 confirms OR suppresses that tail for the reload commit. Two careful readings give different concrete behavior: reading A fires update against every effect instance whose desired cells the reload dirtied (a reconciliation immediately on reload); reading B treats the reload commit as internal and defers update to the next host-driven commit. An implementer cannot decide which, and program-visible side effects differ (whether a reconciler re-aligns external reality at the reload instant).
- **Direction of change:** State in §13.15.3 / 028-23 whether the step-9 reload commit runs the ordinary reconciler-firing tail (enumerate dirtied effect instances and fire update), or explicitly defers reconciler update to the first post-reload host commit.
- **Evidence check:** pass — Reload step-9 commit (028-23) is described as an ordinary commit, yet the universal rule that every commit fires reconciler hooks against dirtied effect instances (027-32/031-124/125) is neither confirmed nor suppressed for the reload commit — two readings give different program-visible reconciliation timing.
- **Charity check:** sustain — Reload step 8 dirties effect desired cells (028-21 recomputes changed deriveds; 028-18 initializes added cells). The step-9 commit (028-23) is stated only as 'Commit the reloaded state' with no mention of reconciler firing. The universal-commit-fires-reconcilers rule 027-32 is explicitly qualified 'In steady state, runtime.commit() ... fires reconciler hooks'; the reload commit is inside the reload lock and is NOT steady state, so 027-32 does not automatically extend to it. 031-124/125 key off 'that commit' generically but are not tied to the reload commit. Grep for 'reconcil'/'hook'/'fires' across the entire reload-sequence region (SPEC 19320-19520) returned zero matches. Two careful readings survive: (A) the atomic reload commit fires update against every effect instance the reload dirtied; (B) it is internal and defers to the next host-driven commit. Program-visible side effects differ. Behavior-changing ambiguity; the steady-state qualifier on 027-32 blocks a forced reading.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:3324-3324`
    > 028-23. Reload step 9 commits the reloaded state. (§13.15.3)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3210-3210`
    > 027-32. In steady state, `runtime.commit()` settles dirty cells, advances recurrents, publishes a new snapshot for observers, and then fires reconciler hooks (which observe the just-published snapshot, §13.14.9). (§13.14.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3893-3894`
    > 031-124. After commit, the runtime enumerates effect instances whose parameters or desired cells became dirty during that commit. (§13.19.14)
    > 031-125. For each such instance, the runtime invokes the registered reconciler with the instance ID; the reconciler reads the new desired state and reconciles. (§13.19.14)
  - `packages/ductus-lang/docs/SPEC.md:19353-19353`
    > 9. Commit the reloaded state.

#### F109 — Whether a module/node value recurrent preserves its accumulated .previous/.past history when its expression body changes across reload is unspecified; only deriveds, operator recurrents, and recurrent-streams are covered.

- **Severity/Category/Verdict:** MED / gap / CONFIRMED
- **Anchors:** LOG 028-6, 028-21, 028-22, 028-30, 016-51, 016-54 · SPEC § 13.15.3, 13.15.2, 13.2.4, 13.18.14
- **Why it is a defect:** The scenario's node holds a value recurrent whose accumulated .past history should persist across the reload boundary. Reload step 8 (028-21/§13.15.3 step 8) speaks ONLY of deriveds; recurrent-stream history has its own explicit rules (§13.18.14, LOG 028-42..050); operator-body recurrents force per-instance restart on a body/initial text change (028-29/028-30). But for a plain value `recurrent[N] T` at module or node scope, no rule states what happens when its expression body changes across reload: is it preserved by identity (028-6 same-path-same-type, keeping .past history) or does the changed behavior reset/re-initialize the history? 016-54 ("holds its last committed value... resumes from pre-gap history") governs gating, not reload. An implementer has no rule to decide whether the recurrent's history survives a body edit, and the two outcomes (preserve vs. reset history) produce different subsequent recurrent values.
- **Direction of change:** Add a rule (LOG §028 / SPEC §13.15.3 step 8 or §13.2.4) covering value-recurrent history under reload: whether a changed recurrent expression body preserves or resets accumulated self/input history when path and type match, paralleling the derived rule and the recurrent-stream rules.
- **Evidence check:** pass — For a plain module/node-scope value recurrent[N] whose expression body changes across reload, no rule states whether accumulated .past/.previous history is preserved (via 028-6 same-path-same-type) or reset — step 8 (028-21) covers only deriveds, and streams/operator-recurrents have their own rules.
- **Charity check:** sustain — No rule states whether a plain value recurrent[N] T at module/node scope preserves its accumulated .previous/.past history when its EXPRESSION BODY TEXT changes across reload. Reload step 8 (028-21/028-22, SPEC 19348-19352) is scoped exclusively to DERIVEDS ('for each derived whose behavior body changed'). 028-6 preserves value by same-path-same-type but is silent on whether a changed behavior (new content-addressed ID, step 7) resets the history buffer. Recurrent-STREAM history has dedicated capacity/type rules (SPEC 13.18.14, 21641-21658) which the finding correctly excludes. 016-54 is gate-driven (13.2.4 reopen), not reload. SPEC 13.17.10 (20048-20050) asserts a recurrent body change is 'reload-safe' but never states the history outcome on a body-text change. Grep confirmed no covering text. Two outcomes (preserve vs. reset .past) yield different subsequent recurrent values -> behavior-changing gap.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:3322-3323`
    > 028-21. Reload step 8 recomputes the value of each derived whose behavior body changed (different content-addressed ID) from current inputs. (§13.15.3)
    > 028-22. A derived whose body is unchanged keeps its value across reload. (§13.15.3)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3307-3307`
    > 028-6. A cell with the same fully-qualified path and same type in old and new source is the same cell; its value is preserved across reload. (§13.15.2)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3331-3331`
    > 028-30. Any change to the text of a cell's `= initial` expression requires restart of the affected operator instances. (§13.15.4)
  - `packages/ductus-lang/docs/SPEC.md:19348-19352`
    > 8. Run a re-initialization evaluation pass: for each derived
    >    whose behavior body changed (different content-addressed ID
    >    from old to new), recompute its initial value from current
    >    inputs. For deriveds whose body is unchanged, the value
    >    persists.

#### F110 — The reload sequence pauses host writes and drains in-flight commits but says nothing about outstanding async reconciler work (worker-thread writebacks) landing after reload, including against a per-instance-restarted effect whose instance was torn down.

- **Severity/Category/Verdict:** MED / gap / CONFIRMED
- **Anchors:** LOG 028-15, 028-16, 028-35, 028-38, 031-100, 027-46 · SPEC § 13.15.3, 13.14.9, 13.15.6, 13.15.4
- **Why it is a defect:** The scenario's "effect mid-reconciliation when reload lands" is only reachable via the §13.14.9 pattern where an update hook dispatched long-running work to a worker thread that is still outstanding when reload(diff) is called (a synchronous hook and reload cannot overlap on the single driving context). Reload steps 2-3 (028-15/16) pause host signal/attr writes and drain the in-flight commit, but the outstanding reconciler writeback is neither a host signal/attr write (it is a reconciler observed-cell write via write_signal, 027-45) nor part of the drained commit. Nothing says whether such a late writeback is queued, dropped, or applied — and if the effect underwent per-instance restart (028-35 param/cell-type change → 028-38 teardown), the old instance ID the worker thread will write against has been torn down, with no rule stating whether that stale write is rejected, silently dropped, or misapplied to the fresh instance sharing the path/ID. This is a soundness-adjacent gap: a legal construct (long-running reconciler + reload) with unspecified behavior at the boundary.
- **Direction of change:** Specify in §13.15.3 how the reload sequence treats outstanding asynchronous reconciler writebacks: whether they are quiesced/awaited before step 4, queued like host writes and applied to the new state, or rejected — and specifically what happens to a write_signal/push_stream keyed to an instance ID that per-instance restart (028-38) has torn down.
- **Evidence check:** pass — Reload's pause-writes + drain-commit sequence (028-15/16) does not cover an outstanding worker-thread reconciler writeback (a write_signal observed-cell write per 027-45, not a host write), so its behavior against a per-instance-restarted/torn-down effect (028-38) is unspecified.
- **Charity check:** sustain — The reload write-pause (028-15) queues host requests, and a worker-thread writeback is a queued host request (SPEC 18977-18978: 'Other threads write indirectly by enqueueing requests for that context to apply'); step 10 (028-24) applies queued writes 'to the new state'. But nothing addresses a queued write whose target effect instance was per-instance-restarted (028-38 teardown). runtime.write_signal (SPEC 18946-18974) specifies no validity check for a torn-down instance_id, and no rule states whether the stale write is rejected, dropped, or misapplied to the fresh instance sharing the path/ID. Grep for stale/torn-down/queued-write handling across the reload and runtime-interface regions returned nothing. Genuine MED gap; no legal boundary (001-6) declared.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:3316-3317`
    > 028-15. Reload step 2 acquires a reload lock and pauses acceptance of new signal writes; host requests queue. (§13.15.3)
    > 028-16. Reload step 3 lets any in-flight commit complete, ensuring a between-commits state. (§13.15.3)
  - `packages/ductus-lang/docs/SPEC.md:19178-19182`
    > Hooks fire at the commit boundary, after the commit publishes its snapshot
    > (§13.10.2) and before the next commit begins. They run on the runtime's
    > driving context; the reconciler implementation must not block long-running
    > operations there (long operations should be dispatched to host-managed
    > worker threads, with results written back via the interface on completion).
  - `packages/ductus-lang/docs/DECISION_LOG.md:3339-3339`
    > 028-38. On per-instance restart of effect instances, the host's reconciler receives a teardown call for the affected instances. (§13.15.4)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3225-3225`
    > 027-46. Both `write_signal` arities must be called from the runtime's driving context — the single context permitted to write, evaluate, and commit. (§13.14.2)

#### F188 — commit fires suspend/resume 'for effects whose activation changed' during commit, yet the same hooks are declared to fire only AFTER the commit publishes its snapshot — the implementer cannot satisfy both.

- **Severity/Category/Verdict:** MED / contradiction / CONFIRMED
- **Anchors:** LOG 027-50, 027-91, 027-96, 027-32 · SPEC § 13.14.4, 13.14.9
- **Why it is a defect:** 027-91 classes suspend/resume as reconciler hooks, and 027-96/027-32 say ALL reconciler hooks fire after the snapshot is published. But 027-50 says commit itself fires suspend/resume, and commit settles-then-publishes (027-49/027-51), placing suspend/resume before publication. So suspend/resume are asserted both to fire before publish (as part of commit) and after publish (as hooks). The snapshot a suspend/resume observes differs between the two readings, so this is behavior-changing, not cosmetic. SPEC mirrors the split: 13.14.4 L18992 fires suspend/resume then publishes; 13.14.9 L19178 says hooks fire after publish.
- **Direction of change:** Pin whether suspend/resume are commit-internal (fire before publication) or post-publication reconciler hooks, and make 027-32, 027-50, 027-91, 027-96 agree on one ordering; conform SPEC 13.14.4 and 13.14.9 to match.
- **Evidence check:** pass — When do suspend/resume observe — pre-publish or post-publish? 027-50/SPEC 18992 place them within commit before publication; 027-91/027-96/027-32/SPEC 19178 class them as hooks firing after publication. The snapshot observed differs, making it behavior-changing. Evidence confirms the jointly-unsatisfiable placement.
- **Charity check:** sustain — Jointly unsatisfiable ordering for suspend/resume. §13.14.4 (SPEC.md:18990-18993) and 027-50 (DECISION_LOG.md:3228) place suspend/resume BEFORE publish: the commit '...advances recurrents, fires suspend/resume for effects whose effective activation changed, and publishes a new consistent snapshot'. But 027-91 (DECISION_LOG.md:3269) classes suspend/resume as reconciler hooks ('suspend on gate-off, resume on gate-on'), and 027-96 (DECISION_LOG.md:3274) / SPEC §13.14.9 (SPEC.md:19178) declare ALL reconciler hooks fire AFTER publish ('after the commit publishes its snapshot'). So suspend/resume are asserted to fire both before publish (as part of the settle-then-publish commit body) and after publish (as reconciler hooks). The snapshot a suspend/resume observes differs between the two placements, so this is behavior-changing, not cosmetic. No passage resolves it. SUSTAIN.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:3228-3228`
    > 027-50. `commit` fires `suspend`/`resume` for effects whose effective activation changed. (§13.14.4)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3269-3269`
    > 027-91. Hooks align with the instance lifecycle: `create` on scope entry, `teardown` on scope exit, `update` on dirty parameters/`desired:` during a commit, `suspend` on gate-off, `resume` on gate-on. (§13.14.9)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3274-3274`
    > 027-96. Reconciler hooks fire at the commit boundary: after the commit publishes its snapshot and before the next commit begins. (§13.14.9)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3210-3210`
    > 027-32. In steady state, `runtime.commit()` settles dirty cells, advances recurrents, publishes a new snapshot for observers, and then fires reconciler hooks (which observe the just-published snapshot, §13.14.9). (§13.14.1)

#### F189 — Whether reconciler hooks run inside the runtime.commit() call (so commit blocks on them) or in a host-driven window after commit() returns is left ambiguous, and the two readings give different blocking behavior.

- **Severity/Category/Verdict:** MED / ambiguity / PLAUSIBLE
- **Anchors:** LOG 027-32, 027-52, 027-96 · SPEC § 13.14.4, 13.14.9
- **Why it is a defect:** Reading A: hooks are part of runtime.commit() (027-32 says commit '...and then fires reconciler hooks'), so per 027-52 commit does not return until hooks finish — commit() blocks on host reconciler code. Reading B: 027-96's 'after the commit publishes ... and before the next commit begins' describes a host-driven inter-commit window, so commit() returns before hooks run and the host drives hook execution separately. The two readings differ in whether a caller of commit() is blocked by (possibly slow) reconciler hooks, and in what the effect's observed writes are visible to before the next commit. No entry in scope resolves which holds. This is not covered by a legal boundary (001-6) because the calling contract of a mandatory-core verb is fully within the normative interface, not implementation-defined.
- **Direction of change:** State explicitly whether hook invocation is inside the commit() call (commit blocks until hooks return) or a separate host-invoked phase after commit() returns, and align 027-32, 027-52, 027-96 and SPEC 13.14.4/13.14.9.
- **Evidence check:** pass — Does runtime.commit() block on reconciler hooks? 027-32+027-52 read as hooks-inside-commit (blocks); 027-96's 'before the next commit begins' reads as a separate host-driven window (returns first). Two careful readings yield different blocking behavior; evidence supports the ambiguity.
- **Charity check:** refute — The alternative reading (commit() returns before hooks run; a separate host-driven window fires them) is NOT forced by any normative text. Reading A — hooks run inside runtime.commit(), so commit() blocks on them — is forced: 027-32 (DECISION_LOG.md:3210) attributes hook-firing to the verb itself, 'runtime.commit() settles..., publishes..., and then fires reconciler hooks', reinforced verbatim by SPEC §13.14.1 (SPEC.md:18926-18928) 'Host calls runtime.commit() to settle dirty cells, advance recurrents, publish a new snapshot..., and then fire reconciler hooks'; combined with 027-52 'commit is synchronous... and returns when the commit completes'. 027-96's 'before the next commit begins' is a temporal boundary that does NOT state commit() has already returned, so it does not force Reading B. The dissolving text (027-32, §13.14.1 L18926-18928) is consistent with the finding's own cited 027-32/027-52/027-96, so this is a clean refute, not a divergence. | SPEC.md:18926-18928 verbatim: '- Host calls `runtime.commit()` to settle dirty cells, advance / recurrents, publish a new snapshot for observers, and then fire / reconciler hooks (which observe the just-published snapshot, §13.14.9).' — the runtime.commit() call itself fires the hooks; combined with DECISION_LOG.md:3230 (027-52) '`commit` is synchronous, runs on the driving context, and returns when the commit completes.' this forces the single reading that commit() runs the hooks before returning and therefore blocks on them; no text places hook execution in a post-return host-driven window.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:3210-3210`
    > 027-32. In steady state, `runtime.commit()` settles dirty cells, advances recurrents, publishes a new snapshot for observers, and then fires reconciler hooks (which observe the just-published snapshot, §13.14.9). (§13.14.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3230-3230`
    > 027-52. `commit` is synchronous, runs on the driving context, and returns when the commit completes. (§13.14.4)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3274-3274`
    > 027-96. Reconciler hooks fire at the commit boundary: after the commit publishes its snapshot and before the next commit begins. (§13.14.9)

#### F205 — Effect-instance reload identity is defined two incompatible ways: 028-4/028-5 use declaration-order path with a positional `:N` ordinal, while 028-51 tolerates positional moves.

- **Severity/Category/Verdict:** MED / contradiction / CONFIRMED
- **Anchors:** LOG 028-4, 028-5, 028-51 · SPEC § 13.15.2, 13.15.6
- **Why it is a defect:** Both rules govern the identity of the same objects (effect instances) across reload. For an anonymous/duplicated effect placement, reordering siblings changes its zero-based declaration-order `:N` ordinal (028-5), which changes its path (028-4) and therefore its identity. But 028-51 says effect identity tolerates positional moves within the same scope, i.e. a reorder preserves identity. An implementer diffing a reload with a reordered anonymous effect cannot satisfy both: 028-4/5 say drop-old + add-new (teardown/create), 028-51 says preserve the instance. The two produce opposite reconciler-hook sequences and opposite state preservation.
- **Direction of change:** Reconcile the effect-instance reload-identity scheme to a single rule: decide whether effect instances key by declaration-order path (`:N` positional, reorder breaks identity) or by (enclosing scope, type name, argument bindings) with positional-move tolerance, and make 028-4/5 and 028-51 agree; surface the choice to the user rather than resolving it here.
- **Evidence check:** pass — Effect-instance reload identity is defined two incompatible ways for an anonymous/duplicated placement: 028-4/5's positional :N ordinal makes a sibling reorder change identity (teardown/create), while 028-51 tolerates positional moves (preserve), yielding opposite reconciler-hook sequences.
- **Charity check:** sustain — Two rules govern the identity of the SAME objects (effect instances) across reload and give opposite results on reorder of an anonymous/duplicated placement. 028-4 (SPEC 13.15.2, 19289-19314): interpreter-placed effect INSTANCES are identified by fully-qualified declaration path; 028-5: anonymous/duplicated siblings get a zero-based declaration-order ordinal ':N', applying 'equally to interpreter-placed effects'; 028-6: different path = removal+addition. Reordering same-type siblings changes ':N' -> changes path -> teardown/create. But 028-51 (SPEC 13.15.6, 19463-19468): 'Effect instance identity across reloads follows the operator identity rule ... tolerating positional moves within the same scope' -> reorder preserves the instance. The operator-identity tie-breaker in SPEC 13.17.10 (20087-20092) matches indistinguishable identical calls by syntactic order and treats an inserted third call as fresh while the existing two 'preserve state'; it does NOT re-index on reorder the way the ':N' positional-path scheme does, so it does not reconcile the two. An implementer diffing a reordered anonymous effect cannot satisfy both: opposite reconciler-hook sequences and opposite state preservation.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:3305-3306`
    > 028-4. Reactive cells are identified across reloads by fully-qualified declaration path — module path, instance name, cell name: `audio.synth_a.osc_1.frequency`. Interpreter-placed effect instances are identified the same way, with their paths rooted at the interpretation site and mirroring the node-path scheme. (§13.15.2)
    > 028-5. Anonymous or duplicated sibling placements get an ordinal suffix `:N` — the zero-based declaration-order index among same-type siblings at the same nesting depth; the `:N` ordinal rule applies equally to interpreter-placed effects. (§13.15.2)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3352-3352`
    > 028-51. Effect instance identity across reloads follows the operator identity rule: enclosing scope, effect type name, and argument bindings, tolerating positional moves within the same scope. (§13.15.6)

#### F206 — 028-12 says a surviving slot's portal generation is preserved across reload; 033-113 says the runtime regenerates the slot stamp at reload — under 028-12's own rule a regenerated stamp resolves the portal to None.

- **Severity/Category/Verdict:** MED / contradiction / PLAUSIBLE
- **Anchors:** LOG 028-12, 033-113 · SPEC § 13.3.6.3, 15.4.6
- **Why it is a defect:** 028-12 conditions portal preservation on the slot's generation being *preserved*, and states a stamp mismatch resolves the portal to None. 033-113 says on reload the runtime *regenerates* the slot stamp. If the stamp is regenerated (issued anew) even for a surviving, non-relocated slot, then the portal's stored generation no longer matches the slot's current generation, so by 028-12 every portal resolves to None after any reload — contradicting 028-12's premise that portals to preserved slots survive. An implementer cannot both regenerate stamps unconditionally (033-113) and preserve matching generations for survivors (028-12).
- **Direction of change:** Pin one behavior: either a surviving slot keeps its generation stamp across reload (only relocation/removal bumps it, per 028-12/13.3.6.3), or the stamp is regenerated (033-113) — and if regenerated, specify how a live portal is re-validated so it does not spuriously resolve to None. Bring the divergence to the user.
- **Evidence check:** partial — Does 033-113 regenerate the stamp for surviving slots (contradicting 028-12) or only for relocated/removed slots (consistent)? The quoted 'regenerates the slot stamp at the new path' scopes regeneration to relocation, so the contradiction as framed (unconditional regen breaking all survivors) is weaker than claimed — a residual terminology tension, not a demonstrated jointly-unsatisfiable rule.
- **Charity check:** refute — The contradiction dissolves under a reading forced by the phrase 'at the new path' in 033-113. 028-12 (DECISION_LOG.md:3313) preserves a portal iff the slot's identity AND generation are preserved, and increments the generation only on 'slot relocation or removal'. 033-113 does NOT say the runtime regenerates the stamp for a surviving, non-relocated slot; read in full, it conditions regeneration on relocation/removal: 'relocation or removal invalidates the portal, and the runtime regenerates the slot stamp AT THE NEW PATH'. A surviving slot (path+kind match per §13.15.2) has no 'new path', so regeneration does not apply to it; it keeps its generation and its portals preserve — exactly 028-12's rule. SPEC §13.3.6.3 (SPEC.md:14419-14426) agrees: survivors preserve, 'slot relocation or removal increments the generation'. The finding's premise that 033-113 regenerates the stamp unconditionally at reload misreads the 'at the new path' scoping. The dissolving text is consistent with 028-12, so this is a clean refute, not a divergence. | DECISION_LOG.md:4225 verbatim: '033-113. Across hot reload, a `Portal[T]` preserves iff its target slot's path and kind match the §13.15.2 cell-identity rule; relocation or removal invalidates the portal, and the runtime regenerates the slot stamp at the new path, parallel to the same-path-and-type cell-value preservation rule. (§15.4.6)' — the clause 'and the runtime regenerates the slot stamp AT THE NEW PATH' is grammatically scoped to the preceding 'relocation or removal', so a surviving same-path slot's stamp is NOT regenerated, matching 028-12's rule that only relocation/removal increments the generation; no unconditional per-reload regeneration is stated.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:3313-3313`
    > 028-12. A `Portal[T]` preserves across hot reload only when its targeted slot's identity AND generation are preserved across the reload. The portal carries a `(slot_path, generation)` pair; on the next read after reload, the runtime compares the stamped generation to the slot's current generation, and a mismatch resolves the portal to `None`. Slot relocation or removal increments the generation (or removes the slot entirely), invalidating portals to that slot. (§13.3.6.3)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4225-4225`
    > 033-113. Across hot reload, a `Portal[T]` preserves iff its target slot's path and kind match the §13.15.2 cell-identity rule; relocation or removal invalidates the portal, and the runtime regenerates the slot stamp at the new path, parallel to the same-path-and-type cell-value preservation rule. (§15.4.6)

#### F207 — 033-113 cites §15.4.6 as its SPEC elaboration, but §15.4.6 (IR module text grammar) contains no Portal, slot-path, or generation content.

- **Severity/Category/Verdict:** MED / divergence / CONFIRMED
- **Anchors:** LOG 033-113 · SPEC § 15.4.6
- **Why it is a defect:** The pointer contract requires the cited SPEC section to elaborate the LOG claim. §15.4.6 (SPEC 24485-24561) is the IR module text grammar; it defines the `type_tag` production and cell/gate/connection/effect entries but never mentions `Portal`, `(slot_path, generation)`, portal reload, or the generation stamp. The word Portal does not occur in §15.4.6. The reader following (§15.4.6) from 033-113 finds no elaboration of portal reload preservation.
- **Direction of change:** Repoint 033-113 to a SPEC section that actually elaborates portal reload preservation (e.g. §13.3.6.3, which carries the Hot reload paragraph), or add the portal reload elaboration to §15.4.6; do not silently pick — flag to the user.
- **Evidence check:** pass — 033-113 cites §15.4.6 as its SPEC elaboration, but §15.4.6 is the IR module text grammar and contains no Portal, slot-path, generation, or stamp content — the pointer resolves to a section that does not elaborate the claim.
- **Charity check:** sustain — Confirmed by fresh read: §15.4.6 (SPEC 24485-24561) is the IR module text grammar; the string 'Portal' does not occur anywhere in it (grepped the 24485-24562 range: zero hits), and it contains no slot-path/generation/portal-reload content. 033-113's claim about Portal[T] preservation across hot reload has its actual elaboration at SPEC 14407/14419 (a different section), not §15.4.6. The reader following (§15.4.6) from 033-113 finds no elaboration of the claim. The dissolving-text hunt turned up the real portal-reload passage but it is NOT the cited section, so the pointer defect stands. Sustained MED divergence (broken LOG→SPEC pointer).
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:4225-4225`
    > 033-113. Across hot reload, a `Portal[T]` preserves iff its target slot's path and kind match the §13.15.2 cell-identity rule; relocation or removal invalidates the portal, and the runtime regenerates the slot stamp at the new path, parallel to the same-path-and-type cell-value preservation rule. (§15.4.6)
  - `packages/ductus-lang/docs/SPEC.md:24529-24531`
    > type_tag      ::= PRIM | '%'NAME | 'pool_index' '<' '%'NAME '>'
    >                 | '(' type_tag (',' type_tag)* ')' | '[' type_tag ';' INT ']'
    >                 | 'closure' '<' '(' (type_tag (',' type_tag)*)? ')' '->' type_tag '>'

#### F220 — Repeat-in-effects materializes N same-type effect instances keyed by element key, but 028-51 identifies effect instances by argument bindings tolerating positional moves — two different identity contracts for the same instances.

- **Severity/Category/Verdict:** MED / contradiction / CONFIRMED
- **Anchors:** LOG 018-109, 018-110, 028-51 · SPEC § 13.5.4.3, 13.5.4.7, 13.15.6
- **Why it is a defect:** For `repeat row in current_rows keyed by row.id: render_row(data: row)`, 018/13.5.4.3 says there is one `render_row` instance per row, identified by `row.id`, cells at `<node>.<row.id>.<cell>`. But 028-51 says effect-instance identity across reload = `enclosing scope, effect type name, and argument bindings, tolerating positional moves within the same scope`. All N `render_row` instances share enclosing scope and type name; their only distinguisher under 028-51 is the argument binding value (`data: row`) — not the key `row.id`. If two rows carry equal `data` payloads with distinct ids, 028-51 collapses/confuses them while 018 keeps them distinct by id; and 028-51's `tolerating positional moves` is a positional-identity notion at odds with 018's key identity. Two readings yield different reload-time instance matching (state preserved vs torn down/reassigned).
- **Direction of change:** Reconcile 028-51 with 018-110: for repeat-materialized effect instances, identity should be the element KEY, not argument-binding equality. State that the key path (13.5.3) governs effect-instance identity when the effect is repeat-materialized, superseding 028-51's argument-binding+positional rule in that case.
- **Evidence check:** pass — Repeat-in-effects (018-109/110) gives N same-type effect instances identity by element key; 028-51 gives effect-instance identity by enclosing-scope + type-name + argument-bindings tolerating positional moves. For N sibling render_row instances sharing scope+type, only the arg binding distinguishes them under 028-51 — a positional/value notion at odds with 018's key identity. Different reload matching. Real contradiction.
- **Charity check:** sustain — §13.5.4.3 (SPEC:15436-15443) and §13.5.4.7 (SPEC:15596-15599) fix repeat-in-effects instance identity by element KEY (cells at <node>.<row.id>.<cell>; 'tear down with the element key'). §13.15.6 (SPEC:19463-19468) uniformly fixes effect-instance identity as (enclosing scope, effect type name, argument bindings) with tolerance for POSITIONAL moves, and makes NO carve-out for repeat-generated effects. Two rows with equal `data` payloads but distinct ids collide under 028-51's argument-bindings identity yet stay distinct under 018's key identity; positional-move tolerance directly contradicts key identity. Hunted §13.15.6, §13.5.4.7, §13.19.8 — no text folds the key into 'argument bindings' for repeat effects, no exception clause. Two identity contracts for the same instances → different reload matching. Sustains (MED).
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2579-2580`
    > 018-109. `repeat` is admitted in `effects:` clauses: each scope materializes one effect-bearing instance per element. (§13.5.4.7)
    > 018-110. Per-element effects from an `effects:`-clause `repeat` suspend, resume, and tear down with the element key. (§13.5.4.7)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3352-3352`
    > 028-51. Effect instance identity across reloads follows the operator identity rule: enclosing scope, effect type name, and argument bindings, tolerating positional moves within the same scope. (§13.15.6)
  - `packages/ductus-lang/docs/SPEC.md:15436-15443`
    >     repeat row in current_rows keyed by row.id:
    >       render_row(data: row)
    > ```
    > 
    > The runtime's reconciler diffs the key set and materializes / drops one
    > `render_row` effect instance per row; per-element effects suspend, resume,
    > and tear down with the key (§13.5.4.7). Each instance's cells live at path
    > `<node>.<row.id>.<cell>` per §13.5.3.

#### F222 — 031-123 enumerates the reconciler's lifecycle hooks as only creation/update/teardown, omitting suspend/resume that 031-107/108/111 require the reconciler to handle.

- **Severity/Category/Verdict:** MED / contradiction / CONFIRMED
- **Anchors:** LOG 031-123, 031-107, 031-108, 031-111, 031-42 · SPEC § 13.19.14, 13.19.12
- **Why it is a defect:** 031-123 (and SPEC 13.19.14) present the reconciler's hook set as an exhaustive three-item list, but 031-107/108/111/42 assert suspend and resume are reconciler hooks the reconciler must respond to. An implementer building the reconciler interface from 031-123 gets a 3-hook interface with no channel to deliver the suspend/resume signals 031-111 guarantees. 027-78 ('five reconciler hooks') and 027-91 (create/teardown/update/suspend/resume) corroborate the correct count of five, out of cited scope but confirming 031-123 undercounts.
- **Direction of change:** Reconcile the hook enumeration in 031-123/SPEC 13.19.14 to include suspend and resume (five hooks), or make explicit that the list is non-exhaustive and points to the suspend/resume hooks in 13.19.12.
- **Evidence check:** pass — 031-123 presents the reconciler hook set as an exhaustive three-item list (creation/update/teardown) while 031-107/108/111 and 027-78/91 require five, including suspend/resume; an implementer building the interface from 031-123 has no channel for the guaranteed suspend/resume signals.
- **Charity check:** sustain — 031-123 (SPEC 13.19.14, lines 22691-22693) presents the reconciler hook set with an exhaustive copula 'A reconciler exposes lifecycle hooks: creation, update, teardown' — no 'e.g.'/'among others' qualifier — yet 031-107/108/111 require the reconciler to receive suspend (gate-close) and resume (gate-open). 027-78 ('five reconciler hooks must be invokable') and 027-91 (create/teardown/update/suspend/resume) confirm the count is five. No dissolving text found: nothing scopes 031-123's list to a 'host-write' subset or otherwise re-admits suspend/resume. An implementer building the reconciler interface from 031-123 gets a 3-hook interface with no channel for the suspend/resume signals 031-111 
guarantees. Sustained as MED contradiction/divergence.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:3892-3892`
    > 031-123. A reconciler exposes lifecycle hooks: instance creation (effect appears in the live graph), update (parameters or desired cells change), and teardown (instance leaves scope). (§13.19.14)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3876-3876`
    > 031-107. Gating off an effect's enclosing subtree delivers the reconciler's `suspend` hook: the resource is released, instance state preserved. (§13.19.12)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3880-3880`
    > 031-111. The runtime guarantees the `suspend` signal on gate-close and `resume` on gate-open. (§13.19.12)
  - `packages/ductus-lang/docs/SPEC.md:22691-22693`
    > - **Lifecycle hooks**: instance creation (when the effect appears
    >   in the live graph), update (when parameters or desired cells
    >   change), and teardown (when the instance leaves scope).

#### F249 — At the gate-open commit boundary neither LOG nor SPEC pins whether the effect's `resume` hook fires before or after its first `update`; a reconciler could reconcile new desired state against a not-yet-reacquired resource.

- **Severity/Category/Verdict:** MED / gap / CONFIRMED
- **Anchors:** LOG 022-59, 031-108, 031-124 · SPEC § 13.9.7, 13.14.7, 13.14.9, 13.19.14
- **Why it is a defect:** On a false→true gate flip, 022-59 puts the desired-cell snap INSIDE the flipping commit, so the effect's desired cells become dirty in that commit. Both `resume` (031-108/SPEC 19082) and `update` (031-124/125) then fire at that single commit boundary (SPEC 19178 fires all hooks there). The desired cells are dirty, so `update` is eligible in the same boundary as `resume`. No rule states resume-before-update. If `update` runs first, the reconciler reconciles the freshly-snapped desired state against a resource that `resume` has not yet re-acquired — a surprising, unspecified interleaving where the two section models (022 activation-snap timing, 031 hook set) compose without a defined order. This is the only place the two vocabularies fail to fully bridge; elsewhere they are deliberately unified.
- **Direction of change:** Pin, in either 031 (§13.19.14) or §13.14.9, the relative order of `resume` and `update` for an instance whose gate opens and whose desired cells snap in the same commit (e.g. resume strictly precedes any same-boundary update), or state that the two are coalesced. Surface to user; do not decide the order unilaterally.
- **Evidence check:** pass — On gate-open, 022-59 snaps desired cells dirty inside the flipping commit; both resume (031-108/SPEC 19082) and update (031-124/125) fire at that boundary (SPEC 19178). No rule states resume-before-update, so update could reconcile new desired state against a not-yet-reacquired resource.
- **Charity check:** sustain — On a false→true gate flip, 022-59 puts the desired-cell snap INSIDE the flipping commit, so the effect's desired cells become dirty this commit. 031-124 (unconditional: 'enumerates effect instances whose parameters or desired cells became dirty during that commit') then fires `update` for this instance at the commit boundary (SPEC 19178). `resume` also fires at that same boundary on effective-activation-true (031-108/111; SPEC 19082, 19541-19543). Grep for any resume/update ordering rule ('before/after/then/order' near suspend/resume) returns nothing. No carve-out exempts a just-resumed instance from the dirty enumeration. So if `update` runs first, the reconciler reconciles freshly-snapped desired state against a resource `resume` has not yet re-acquired — an unspecified, surprising interleaving. No dissolving text; sustains as MED gap in the composition of §13.9 activation-snap timing with §13.19.14 hook firing.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2943-2943`
    > 022-59. The gate-open re-evaluation snap is scheduled within the **same commit** that flips the predicate false→true — not the next commit; this contrasts with a reload predicate (next commit) and a connection re-point (next commit). (§13.9.7)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3893-3894`
    > 031-124. After commit, the runtime enumerates effect instances whose parameters or desired cells became dirty during that commit. (§13.19.14)
    > 031-125. For each such instance, the runtime invokes the registered reconciler with the instance ID; the reconciler reads the new desired state and reconciles. (§13.19.14)
  - `packages/ductus-lang/docs/SPEC.md:19178-19179`
    > Hooks fire at the commit boundary, after the commit publishes its snapshot
    > (§13.10.2) and before the next commit begins.
  - `packages/ductus-lang/docs/SPEC.md:19082-19085`
    > - A *resume* hook invoked when the enclosing subtree is gated back on
    >   (effective activation goes true). Receives the instance ID and the
    >   preserved reconciler-side instance state. Re-acquires the external
    >   resource from the preserved state.

#### F262 — 016-153 enumerates 'first recurrent commit' as a startup abort case, but reload can add a recurrent (028-8) whose first evaluation is not covered by that startup-scoped rule, leaving its first-commit trap fate unstated.

- **Severity/Category/Verdict:** MED / gap / CONFIRMED
- **Anchors:** LOG 016-153, 028-8, 028-18 · SPEC § 13.2.6, 13.15.2, 13.15.3
- **Why it is a defect:** 016-153 names 'first recurrent commit' and 'initial derived evaluation' as abort cases but binds them to §13.2.6 (startup). Hot reload adds cells that undergo their own first-recurrent-commit / initial-derived-evaluation during reload step 5, outside startup. The reload rules (028) describe how added cells are initialized but never say a trap in that first evaluation aborts, nor whether the startup no-recovery rule extends to it. This is the same underlying gap as the primary finding, isolated to the added-cell first-evaluation sub-case; kept separate because an implementer could plausibly read 016-153 as startup-only and thus find added-cell init entirely unspecified for traps.
- **Direction of change:** Either broaden 016-153's scope to explicitly include reload-time first evaluations, or add a distinct reload-time rule; do not leave the added-cell first-evaluation trap fate to inference. Owner decides which.
- **Evidence check:** pass — 016-153 binds the first-recurrent-commit / initial-derived-evaluation abort rule to startup (§13.2.6), but a cell added during hot reload (028-8) undergoes its own first evaluation outside startup, and no reload rule states whether a trap there aborts, leaving added-cell first-commit trap fate unstated.
- **Charity check:** sustain — The startup abort rule is textually startup-scoped and does not reach reload-time first evaluation. 016-153 (L2006): 'A trap during any initial evaluation (signal initializer, attr default, first recurrent commit, initial derived evaluation) aborts the process; startup traps have no recovery path. (§13.2.6)'; SPEC:12336-12340 confirms 'There is no recovery path for traps encountered during startup.' Reload steps 5 and 8 (028-8/028-18; SPEC:19338-19339 'allocate cell storage and initialize per the new source', L19348-19351 'recompute its initial value from current inputs') run their own first-recurrent-commit / initial-derived evaluation OUTSIDE startup, with the reload lock held mid-atomic-reload. grep of all of §028 and SPEC §13.15.x for trap|abort|rollback|revert finds NOTHING covering a trap during that reload-time evaluation. 028-3 covers only compile-failure rejection, not a runtime init trap. No legal boundary (001-6) is declared. The fate is genuinely unstated: abort (per a generalized 016-153) vs roll back the reload (per 028-3's spirit). Per the gap standard, refute needs explicit covering text; none exists. Sustain.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:2006-2006`
    > 016-153. A trap during any initial evaluation (signal initializer, attr default, first recurrent commit, initial derived evaluation) aborts the process; startup traps have no recovery path. (§13.2.6)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3309-3309`
    > 028-8. A cell present only in the new source is an addition: a new cell is allocated and initialized per the new source's declared initial value or default. (§13.15.2)

#### F225 — 031-52 phrases gating placement as 'the effect's desired: block (or an expose: block)', implying an effect owns an expose: block, but expose: is node-only and never mixes with effects (017-259/274).

- **Severity/Category/Verdict:** LOW / ambiguity / CONFIRMED
- **Anchors:** LOG 031-52 · SPEC § 13.19.5
- **Why it is a defect:** 031-52's parenthetical '(or an `expose:` block)' sits inside a clause about 'the effect's desired: block', reading as if an effect may carry an expose: block. But 017-259 ('Effects are never exposition entries') and 017-274 ('expose: declares topology; effects: declares side effects; the two never mix') establish expose: as node-only, and an effect never has one. Reading A: the expose: block is the enclosing node's (legal). Reading B: the effect has an expose: block (illegal per 017-274). The atomic entry does not disambiguate whose expose: block it means; SPEC 13.19.5 repeats the same conflation.
- **Direction of change:** Clarify in 031-52 (and SPEC 13.19.5) that the expose: block is the enclosing node's, not the effect's, so the reader does not infer effects carry expose: blocks.
- **Evidence check:** pass — 031-52 reads as if an effect owns an expose: block; ownership undisambiguated.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:3821-3821`
    > 031-52. Gating driven by an observed cell uses `when`/`given` in the effect's `desired:` block (or an `expose:` block); per-element materialization is a node's `effects:`-clause `repeat`, not an effect block. (§13.19.5)
  - `packages/ductus-lang/docs/SPEC.md:22277-22280`
    > reactive-structure declarations. To materialize per-element scopes from
    > an observed cell, place the `repeat` in an `expose:` block or the same effect's
    > `desired:` block, consuming the observed cell as the source; to gate
    > structure on an observed value, do the same with a `when`/`given` block.

#### F203 — 032-166 says reload-unsafe changes fall into exactly two classes (full-runtime restart, per-instance restart), but 032-170/171 add 'reject' as a third implementation-defined disposition for the same unsafe changes, making the two-classes taxonomy incomplete.

- **Severity/Category/Verdict:** LOW / contradiction / CONFIRMED
- **Anchors:** LOG 032-166, 032-170, 032-171 · SPEC § 14.8.3
- **Why it is a defect:** 032-166 states the taxonomy of unsafe changes is 'two classes,' both restart-based. 032-170/171 permit a third disposition — outright rejection (keep running old version) — chosen implementation-defined. An unsafe change's actual disposition is one of {reject, full-restart, per-instance-restart}, three outcomes, not the two 032-166 asserts. The 'two classes' framing describes only the restart sub-cases and omits reject, so a reader taking 032-166 as the complete taxonomy is wrong.
- **Direction of change:** Reconcile the count in 032-166 with the reject option in 032-170/171 (either frame 032-166 as classifying only the restart remedy, or fold reject into the taxonomy) — surface to user.
- **Evidence check:** pass — 'Two classes' taxonomy omits reject, making it incomplete against 032-170/171.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:4096-4096`
    > 032-166. Reload-unsafe changes fall into two classes per §13.15.4: those requiring full-runtime restart and those needing only per-instance restart. (§14.8.3)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4100-4101`
    > 032-170. The implementation diagnoses unsafe changes at reload time and either rejects them — the runtime keeps running the old version — or cleanly applies the appropriate restart, full-runtime or per-instance per §13.15.4. (§14.8.3)
    > 032-171. Whether an unsafe change is rejected or handled by restart is implementation-defined. (§14.8.3)

#### F120 — Source-level `private` on a leaf effect does not survive lowering: the IR carries the effect's declared type name and the host must register a reconciler by that string, so the host observes a private effect's identity; the docs never state whether visibility is meant to bind at the host boundary.

- **Severity/Category/Verdict:** LOW / gap / CONFIRMED
- **Anchors:** LOG 031-77, 031-119, 027-68, 033-120, 033-141 · SPEC § 13.19.10, 13.19.14, 13.14.7, 15.4.1, 15.4.3
- **Why it is a defect:** Visibility is a purely compile-time, reference-site check (003-6). A module-private leaf effect (its `observed:` block declares host-written channels) is legal — it is instantiated in a same-module node's `effects:` clause (031-131, 003-18) — and therefore REQUIRES a host reconciler (031-119). Reconciler registration is keyed by the effect's declared type name (027-68, 033-120), which the IR carries verbatim as `effect_type_name`; §15.4 (the entire IR data model) encodes NO visibility field, and type erasure (033-141) drops only records/traits/generics, never the effect type name. So the host — code outside every Ductus module — necessarily observes and registers-by the private effect's identity string. 031-77 scopes non-reachability to 'other modules' (a Ductus-source concept); the host is not a module, so this is not a strict contradiction and both sides are jointly satisfiable (reject cross-module Ductus refs AND expose the name to the host). The gap: the docs never state whether `private` on an effect is meant to carry any meaning at the host boundary. Under 001-6, the only legal boundaries are stdlib delegation or implementation-defined behavior; the host BINDING is implementation-defined (027-2, 027-77), but the REQUIREMENT that a private leaf effect's name be surfaced to and claimed by the host is fully normative, not a deferred decision. No implementer is blocked (behavior is specified), and no rule ever claimed visibility constrains the host — hence LOW, a design-smell/documentation gap rather than a soundness hole. Witness sketch: module A declares `private effect internal_health_check(): observed: signal status: Health = Health::Unknown`; a same-module node hosts it in `effects:`; module B's public wrapper node references only B's public effect. At runtime the host must call `runtime.register_reconciler("internal_health_check", ...)` — the private name is host-visible.
- **Direction of change:** Decide and state explicitly (in 031 §13.19.10 and/or 027 §13.14.7) whether effect visibility is intended to bind at the host/reconciler boundary at all. Either (a) affirm that visibility is a Ductus-source-only reference-site property that deliberately does not survive lowering — private effects' type names are host-observable by construction — and note it beside 031-77 so the reader is not misled; or (b) if private effects' identities are meant to be hidden from the host, add the missing rule and a name-mangling/omission mechanism in the IR. This is a user decision, not to be resolved by the audit.
- **Evidence check:** pass — Docs never state whether `private` on a leaf effect binds at the host registration boundary.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:3846-3846`
    > 031-77. Module-private effects are not reachable from other modules; public effects may be re-exported. (§13.19.10)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3888-3888`
    > 031-119. The host registers a reconciler via the host API, keyed by the effect's type name, if and only if the effect's `observed:` block declares host-written channels (`signal`/`stream`); an interior effect whose contract is fulfilled entirely by child effects requires no reconciler. (§13.19.14)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3246-3246`
    > 027-68. The `effect_type_name` argument is a string matching the name of an `effect` declaration in the loaded source. (§13.14.7)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4232-4232`
    > 033-120. An effect entry's `effect_type_name` is the effect's declared type name, used to dispatch to the host's reconciler. (§15.4.1)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4253-4253`
    > 033-141. The specification is type-erased at the runtime boundary: it carries primitive type tags and cell layouts but no record definitions, trait conformances, or generic parameters. (§15.4.3)
  - `packages/ductus-lang/docs/DECISION_LOG.md:109-109`
    > 003-6. A reference to a declaration from outside its visibility reach is a compile error at the reference site. (§10.1)
  - `packages/ductus-lang/docs/SPEC.md:24293-24299`
    > #### 15.4.3 Type erasure at the runtime boundary
    > 
    > The specification is type-erased at the runtime boundary. It contains
    > primitive type tags and cell layouts, but **not** Ductus's full type
    > system — no record definitions, trait conformances, or generic
    > parameters. These are compile-time artifacts of the frontend, fully
    > resolved before the specification is emitted.

#### F208 — 033-113 says portal preservation keys on the slot's path and `kind`, but the §13.15.2 rule it invokes keys on path and `type`; `kind` is a different, separately-defined term in the corpus.

- **Severity/Category/Verdict:** LOW / divergence / CONFIRMED
- **Anchors:** LOG 033-113 · SPEC § 13.15.2
- **Why it is a defect:** 033-113 characterizes the §13.15.2 cell-identity rule as matching on `path and kind`, and in the same sentence calls it the `same-path-and-type` rule — using both `kind` and `type` for the criterion. §13.15.2 keys strictly on `type`. Meanwhile `kind` is a load-bearing term elsewhere: 033-80 defines a cell IR `kind` as input/derived/recurrent/fold, and 016-178 uses `kind` for the lowercase kind-annotation taxonomy. Calling the §13.15.2 criterion `kind` mislabels it and risks an implementer reading the wrong equality (kind-tag vs full type).
- **Direction of change:** Use `type` (matching §13.15.2) uniformly in 033-113 instead of `kind`, or state explicitly which equality (full type vs IR kind-tag) governs portal-slot preservation; surface to the user.
- **Evidence check:** pass — 033-113 labels the §13.15.2 type-keyed rule as keying on 'kind'.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:4225-4225`
    > 033-113. Across hot reload, a `Portal[T]` preserves iff its target slot's path and kind match the §13.15.2 cell-identity rule; relocation or removal invalidates the portal, and the runtime regenerates the slot stamp at the new path, parallel to the same-path-and-type cell-value preservation rule. (§15.4.6)
  - `packages/ductus-lang/docs/SPEC.md:19313-19314`
    > When a cell exists in both but with different type, it is treated
    > as removal of the old + addition of the new.

#### F209 — 028-4 asserts interpreter-placed effect instances are identified by declaration path the same as cells, but the cited §13.15.2 elaborates only cells and never mentions effect instances.

- **Severity/Category/Verdict:** LOW / divergence / CONFIRMED
- **Anchors:** LOG 028-4, 028-5 · SPEC § 13.15.2
- **Why it is a defect:** 028-4's second sentence (interpreter-placed effect instances identified the same way, paths rooted at the interpretation site) has no counterpart in the cited §13.15.2, which speaks only of reactive cells. The pointer promises elaboration of the effect-instance-by-path claim and does not deliver it; combined with 028-51's contrasting §13.15.6 rule, an implementer has no single authoritative elaboration for effect-instance reload identity.
- **Direction of change:** Either add the interpreter-placed-effect-instance identity elaboration to §13.15.2 (the cited section) or repoint 028-4's effect-instance clause; and reconcile with 028-51/§13.15.6 (see the contradiction finding). Flag to the user.
- **Evidence check:** pass — 028-4 asserts effect instances identified by path per §13.15.2, but §13.15.2 elaborates only cells.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:3305-3305`
    > 028-4. Reactive cells are identified across reloads by fully-qualified declaration path — module path, instance name, cell name: `audio.synth_a.osc_1.frequency`. Interpreter-placed effect instances are identified the same way, with their paths rooted at the interpretation site and mirroring the node-path scheme. (§13.15.2)
  - `packages/ductus-lang/docs/SPEC.md:19291-19295`
    > Reactive cells are identified across reloads by their *fully-
    > qualified declaration path*: the dotted sequence of module path,
    > instance name, and attribute/recurrent/signal/derived name. For
    > example, `audio.synth_a.osc_1.frequency`. The wire-format syntax for
    > declaration paths is specified in §15.4.1.1.

#### F226 — Effect failure ('reconciliation contract') is routed to the value track only via an observed error cell that no 031 rule requires an effect to declare, so a leaf effect with no error cell has a reconciliation-failure mode with no guaranteed 011 track.

- **Severity/Category/Verdict:** LOW / gap / CONFIRMED
- **Anchors:** LOG 031-129, 031-158 · SPEC § 13.19.14
- **Why it is a defect:** 011-78 requires the two-track model to apply uniformly, and 031-158 declares every effect a reconciliation contract with a failure mode. But 031-129 states reconciler errors surface 'through the observed cells' via an error signal, while no 031 rule (031-7 through 031-9, 031-14, 031-43) mandates that an effect declare such an error cell. The SPEC even softens this to 'typically'. An effect that declares only a non-error observed signal therefore has a reconciliation-failure outcome (031-1 desired alignment fails) with no rule-guaranteed value-track landing site, leaving the failure neither on the trap track (031-130 forbids reconciler panic) nor guaranteed on the value track. This is a coherence gap between the 'contract has a failure mode' framing and the two-track guarantee, not a hard contradiction: the effect author can add an error cell.
- **Direction of change:** Either state (as a 031 rule) that reporting reconciliation failure via an observed cell is a convention the author must opt into (making it explicit the failure track is author-chosen, not language-guaranteed), or make an observed error channel obligatory for reconciler-registered effects so 011-78's uniform two-track guarantee is satisfiable for every effect. Direction only; the user decides.
- **Evidence check:** pass — Effect reconciliation-failure has no rule-guaranteed value-track landing site absent a declared error cell.
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:3898-3898`
    > 031-129. Reconciler errors are reported to the program through the effect's `observed:` cells: `signal error: Option[E] = None`. (§13.19.14)
  - `packages/ductus-lang/docs/DECISION_LOG.md:3927-3927`
    > 031-158. Every effect is a reconciliation contract, fulfilled by a host reconciler when it is a leaf effect, by child effects when it is an interior effect, or by both. (§13.19)
  - `packages/ductus-lang/docs/DECISION_LOG.md:1189-1189`
    > 011-78. The reactive system uses the same two-track failure model as the rest of the language. (§8.9)
  - `packages/ductus-lang/docs/SPEC.md:22725-22728`
    > **Reconciler error reporting.** Reconciler errors (network failure,
    > resource exhaustion, etc.) are reported to the program through the
    > effect's `observed:` cells (typically a `signal error: Option[E] =
    > None` cell).

## Known-open handling

Workers were briefed with a known-open register — Phase 19a Cell-storage-prohibition over-reach residue, Phase 19b subscribe-to-dependency-edge terminology, the C12 to_ring/gate_stream deletions, the rejected interpret/combineLatest/Monoid designs, and the adjudicated-legitimate 029-74 'fn RHS' wording difference — and instructed not to re-flag those items. Zero findings were tagged known-open, so nothing in this report duplicates the register and nothing beyond that briefing was suppressed. One deliberate exception: the BACKLOG's 'Cell as a type with no special treatment' tension against the amendment's Cell-as-KIND umbrella was treated as a live design question, not a known-open item — it was probed intentionally (shards S03/S16/S17, cross-pair 004x016) and its fallout is reported openly as theme 1 rather than filtered, because which side wins is a decision only you can make.

## UNVERIFIED findings (verifier died or omitted; NOT dropped — treat as unreviewed leads)

#### F115 — 034-9 claims a `yielded` group reuses 'the gate on/off-bit machinery', but neither section 022 (Gates) nor 033 (IR) defines gate state as on/off bits — they define predicate gate objects composed via gate_parent.

- **Severity/Category/Verdict:** LOW / divergence / UNVERIFIED
- **Anchors:** LOG 034-9, 033-65, 033-81 · SPEC § 13.20, 15.4.1
- **Why it is a defect:** 034-9 names an 'on/off-bit machinery' belonging to gates and asserts the `yielded` group reuses it. But the gate mechanism actually specified in 022 and 033 is predicate-based gate objects (033-65: `{id, pred, guards, gate_parent}`) whose effective activation is composed by walking gate_parent (033-66) and evaluated at edge level each commit (022-41). No 'on/off bit' array is defined anywhere in 022 or 033 as a gate primitive; 033-81/§15.4.1 wires a gate-guarded fold member edge to the gate's effective activation (a predicate composition), not to a stored on/off bit. So 034-9's 'reusing the gate on/off-bit machinery' points at a mechanism with no referent in the sections it claims to reuse, leaving it ambiguous whether a `yielded` group's runtime liveness is (a) a fresh bit-vector the group stores or (b) just re-reads of the gate objects' effective activation. Under (b), 034-9's 'runtime on/off bits' storage claim is inaccurate since the liveness is derived, not stored; the group 'stores nothing' clause and 'runtime on/off bits' clause sit uneasily together.
- **Direction of change:** Either define the 'gate on/off-bit machinery' term in section 022/033 that 034-9 reuses, or restate 034-9 in the vocabulary 022/033 actually use (gate objects / effective-activation composed via gate_parent), and clarify whether a `yielded` group stores its own runtime liveness bits or derives member liveness by reading the guarding gates' effective activation.
- **Evidence check:** unverified — missing from output
- **Evidence:**
  - `packages/ductus-lang/docs/DECISION_LOG.md:4360-4360`
    > 034-9. A `yielded` group stores nothing: it is realized as a compile-time wire set plus runtime on/off bits, reusing the gate on/off-bit machinery, and holds no backing collection of its own. (§13.20)
  - `packages/ductus-lang/docs/DECISION_LOG.md:4177-4177`
    > 033-65. Gating is encoded in the graph IR as first-class gate objects — `{id, pred (behavior handle + input cell IDs), guards (gated-instance paths), gate_parent (enclosing gate id or null)}` — and each gated cell, connection, or effect references its gate by `id`; nesting is gate→gate via `gate_parent`, which the runtime walks to compose effective activation.
  - `packages/ductus-lang/docs/DECISION_LOG.md:4193-4193`
    > 033-81. A `fold`-kind cell entry carries a combiner behavior id (a `fn(T, T) -> T` handle), an else value (the empty-membership result), and member edges listed in member order; each member edge is tagged with its membership driver — `permanent`, `keyed-template`, or `gate-guarded`. (§15.4.1)

## Coverage

What this audit read: all 35 LOG sections, each exactly once, via 33 disjoint shards (seven giant sections split at entry midpoints); every SPEC subsection cited by an in-scope LOG entry, read as whole leaf subsections located by heading grep. On top of shard coverage: 20 cross-pair passes on adjacent-restatement seams, 12 end-to-end witness scenarios, 8 code-fence sweeps over the full SPEC, dedicated parser, terminology, implementer, smell, lossless, and gate passes, then normalization, verification, and dedup. Pin held for the whole run (HEAD 1b0c6b8, re-verified at synthesis). 4 findings were refuted and dropped; 10 observations were inadmissible smells and not carried; duplicate reports were merged into the x-counts on survivors. What this audit cannot claim: (1) SPEC subsections cited by NO LOG entry got no dedicated reader — the planned orphan sweep was never scheduled, so uncited SPEC ranges are attested unread, not clean. (2) The planned mechanical residual grep (to_ring, gate_stream, subscribe, interpret, combineLatest, Monoid, signal-as-noun) never ran; Phase-19a/19b and C12 deletion residue is covered only incidentally. (3) Three lanes died and were not re-run: pair-029x035 (operators x fold seam — partially covered by shard-S26 and S33 but not as a pair), smell-behavior-and-data, and dedup-2 (hence the residual near-duplicates noted in the reading guide). (4) F115 remains UNVERIFIED — no verifier reached it; treat it as an unadjudicated claim. (5) The planned dedicated six-site collation of the reconciler-registration canon was folded into shard evidence (F169/F191) rather than run as its own micro-agent. (6) The amendment plan document itself was out of scope as evidence; findings rest on LOG+SPEC only.

### Coverage matrix

| Mechanism | Coverage level | Status |
|---|---|---|
| Claims index (all ~4185 entries normalized; subject-collision adjudication) | exhaustive at subject-collision fidelity | all 12 ok |
| Shard deep audits (35 sections + cited SPEC leaf ranges) | exhaustive per-entry read | all 33 ok |
| Cross-pair semantic hunts | sampled (15 of 595 pairs) | 14 ok / 1 dead-or-halted (coverage hole) |
| Terminology (mechanical inventory + adjudication) | heuristic | all 3 ok |
| Design-smell panel (001-grounded admissibility) | sampled by cluster | 8 ok / 1 dead-or-halted (coverage hole) |
| Implementer completeness + unfalsifiability | sampled by cluster | all 9 ok |
| Parser adversaries | heuristic | all 2 ok |
| Witness probes (constructive, 3+ section composition) | sampled (10 scenarios) | all 10 ok |
| Example conformance (all SPEC fences) | exhaustive mechanical charter | all 8 ok |
| Reverse losslessness (churn + orphan zones) | sampled (churn+orphan zones only) | all 1 ok |
| Mechanical gates (8 invariants) | exhaustive mechanical | all 8 ok |
| Deepening rounds | Director-directed | all 0 ok |
| Adversarial verification (dual-lens HIGH/MED; evidence-only LOW) | per-cluster | all 47 ok |

### Disclaimed (this audit does NOT claim)

- Semantic cross-section completeness beyond subject collisions and the sampled pairs.
- Parse-ambiguity completeness (heuristic adversaries, not a grammar proof).
- Joint-satisfiability completeness (witness probes sample composition space).

### Agent failures/halts

- pair-029x035 (Sweeps & probes): dead
- smell-behavior-and-data (Sweeps & probes): dead
- dedup-2 (Triage): dead
- director-triage (Triage): dead
- completeness-critic (Deepening): dead

## Appendix A — Observations (inadmissible smells; unverified opinions, kept for honesty)

- **F038** [shard] SPEC-internal tension: §4.8.3 (cited by 007-180) says FloatPow takes an 'any-numeric exponent', but §4.9.1's trait signature declares `fn pow(base: Subject, exp: Subject)`, forcing a float exponent. — Two SPEC sections disagree on the FloatPow exponent type. §4.8.3 (the section 007-180 cites, and which 007-180 matches verbatim) permits any-numeric exponent with integer→float promotion; §4.9.1's declared signature makes `exp: Subject`, i.e. the same float type, admitting no integer exponent without prior promotion. This is a SPEC-internal inconsistency, not a LOG-SPEC divergence for 007-180 (who
- **F058** [smell] 017-96 lists `Bundle[T]` among 'language-level compound types alongside string, tuples, arrays, slices, and maps' but 017-89 also calls it 'a distinct kind from a view' and D0-family kinds; the taxonomy label 'compound type' vs 'kind' is applied inconsistently to the same construct. — Terminology drift: 017-96 files Bundle[T] under 'language-level compound types' (with string/tuples/arrays/slices/maps), while the reactive-structure sections elsewhere describe `dynamic view T` as a language 'cell kind' (D0-8) and 017-89 calls Bundle 'a distinct kind'. No behavior turns on this, so it is observation-grade, but the mixed 'compound type' / 'kind' vocabulary for graph-shape construc
- **F075** [implementer] 035-5 asserts the fold cost is 'in the family of the loop and collection cost rules', but no normative loop cost rule exists anywhere in the corpus. — 035-5 invokes 'the family of the loop and collection cost rules' as if a normative loop-cost rule exists. Collection cost rules do exist (012-108, 025-40, 025-41..43, 032-65/67). But grepping the whole LOG for loop/repeat/for normative complexity bounds turns up only 018-32 (which describes `scope_evaluate` per active key per commit as a mechanism, not a stated Big-O bound) and 018-103/014-149 (wh
- **F079** [smell] The reconciler-registration biconditional is copy-restated verbatim at 8 LOG sites across 4 sections plus multiple SPEC paragraphs; a future edit to one site risks silent divergence. — The ⚛A smell named in the role brief. Under LOG Invariant 2 (atomic, self-contained entries that cannot reference each other by number), cross-section restatement of the shared rule is INVARIANT-MANDATED, not a defect. It is classified observation, not design_smell, because the admissibility rule requires a 001-x commitment or a Ductus-program footgun and neither exists for a documentation-DRY con
- **F118** [witness] Scenario premise 'operator body raises a recoverable error' has no Ductus construct: 'recoverable' is value-track, produced as an Err/None output value, never raised/unwound. — The rules jointly and unambiguously answer the scenario, so this is not a soundness defect. Recorded as an observation because the scenario's four candidate outcomes each have a definite answer under the rules: (a) the gated arm does NOT deactivate — arms flip only on when/given predicate change (022-95/022-98), never on event content; (b) the stream DOES carry the error — as a value-track Err/Non
- **F119** [witness] Section 030 (260 rules on streams) never restates what a value-track (Err/None) stream event means downstream, deferring silently to 011/026 rather than carrying a self-contained restatement. — Not a defect: 030 events are 'typed events' (030-4), so an event whose type is Result[T,E] is fully governed by the two-track model (011/026) with no stream-specific carve-out needed. The stream layer is correctly orthogonal to error semantics. Flagged only as an observation that a reader scanning 030 alone finds no mention of error-typed events; the answer lives in 026-6/026-7, which is the corre
- **F125** [witness] Scenario's 'both feed one fold' is structurally unrealizable: a fold takes one members group (yielded or uniform composite); a scalar count-derived is neither and cannot co-member. — Not a document defect — a scenario-framing clarification for the orchestrator. The probe asks whether the repeat feed and the count-derived feed 'agree when both feed one fold.' Per 035-6 a single fold has exactly one `<members>` operand, either a `yielded` group or a uniform-slot reactive composite. The repeat-driven `yield`s form a `yielded` group (valid fold input); the `count = items.count` de
- **F210** [shard] 025-22 states reactive cells accept any value type, but 025-54/025-55 forbid dyn trait objects, functions, and closures as cell types in v1 — the general claim is stated without its carve-out. — 025-22 says cells accept `any value type`; 025-54 and 025-55 carve out dyn trait objects, functions, and closures for v1. Read literally and atomically (Invariant 2 requires each entry to be self-contained), 025-22 over-states the rule: a reader taking 025-22 alone would conclude a closure-typed cell is legal, which 025-55 forbids. Not a hard contradiction because the exclusions are explicit elsew
- **F235** [terminology] 'candidate topology' is used once with only an inline gloss, never a definitional statement, though the gloss largely self-describes it. — 'candidate topology' appears exactly once (015-16), glossed inline as 'every node type a `to` could resolve to'. The gloss is close to a definition but is embedded in a longer sentence rather than stated as a standalone atomic decision, and the term is not restated where 'wire-candidate envelopes' derive from it. Low-severity terminology/atomicity smell rather than a soundness issue; the inline gl
- **F250** [cross-pair] Cross-pair conclusion: the 022 activation vocabulary and the 031 reconciler/suspend-resume vocabulary are consistently bridged, not contradictory; the director's 'gate cease-to-exist vs effect teardown' hypothesis does not hold. — Not a defect. Recorded so the orchestrator knows the seam was checked and cleared: the gate side never says children 'cease to exist' (022-7 gates propagation not existence; SPEC 17414 'Gates never unmount'), and gate-off maps to `suspend` with instance state preserved (031-104/107), never `teardown` (031-106 'Only scope death causes effect instance teardown'). SPEC 19542-19543 explicitly names th

## Appendix B — Refuted findings (raised and killed; reasons given)

- **F127** A stream piped into a value-reading operator (cell T param) is legal at the pipe and dispatches identically to a value cell, but no rule specifies its behavior and no diagnostic rejects it — a gap. — refuted: tiebreak: The gap claim ('no rule specifies its behavior and no diagnostic rejects it') is dissolved. The read-site exclusion is stated normatively in the operator-parameter context itself: SPEC 13.18.5 (20556-20557) 'a value-reading operator parameter is annotated cell T ... a stream T has no current value, so it is excluded at the read site rather than by the annotation'; SPEC 12417-12418 (operator-parameter bullet) says the same; LOG 029-124 restates it. When smooth's body reads source as a value (verbatim SPEC 19585: 'state.previous(source) + (source - state.previous(source)) * rate'; 19586 'default: source'), that arithmetic operand expects T. SPEC 13.17.3.1 (19696-19715) applies auto-deref ONLY to value cells (signal/derived/recurrent[N]) at 'arithmetic operands ...'; a stream is not a value cell, so it does not deref -- and the exclusion rule rejects it. The normative diagnostic class 'cannot read a stream T as a value' (SPEC 21746-21749) is the enforcing diagnostic. Auto-deref covers the legal case (value cell -> value); the exclusion covers the illegal case (stream -> rejected) -- complementary, not gapped. Behavior specified = compile error; diagnostic exists. Finding's cited diagnostic (SPEC 20173-20182, plain value f32 -> cell T) is a different class and does not conflict. All dissolving passages agree with 029-124 -> not refile_divergence.
- **F219** Anonymous same-type child instances placed by a repeat body (e.g. Voice, UserCard) are identified by key under 018 but would be identified by positional ordinal :N under 028-5, so a source reorder silently swaps their state across reload. — refuted: tiebreak: The contradiction requires the repeat-placed anonymous children to be identified by a positional/iteration-order :N under 028-5 while keyed under 018. But 028-5 / SPEC 13.15.2 (19297-19300) / SPEC 15.4.1.1 (24229-24232) all define :N as the DECLARATION-order index among siblings of the same type at the same nesting depth (zero-based). A repeat body contains exactly ONE RowComponent/Voice/UserCard *declaration*; runtime multiplicity is data-driven. So :N is trivially :0 for that single declaration and never varies with source element order -- it discriminates declarations, not runtime elements. Per-element identity is the <key> path segment: SPEC 13.5.3 (15206-15221) states the repeat-managed cell path is <enclosing_path>.<key>.<template_field> and 'The key value serves as the path component'; SPEC 13.5.3 (15191-15194) explicitly routes repeat-placed children through the keyed .<key>. path; SPEC 15461-15462 confirms 'Each Voice scope's state persists across commits for the same voice_id.' The two schemes address orthogonal axes (declarations vs runtime elements) and compose without conflict; a source reorder cannot swap state because :N is not an element-order index. The finding's own claim ('RowComponent:0, RowComponent:1... by iteration order') contradicts 028-5's verbatim 'declaration-order' wording. Dissolving passages are consistent with the finding's cited 018-88/93 -> not refile_divergence.
- **F261** No rule specifies what happens when a user-code evaluation traps during the hot-reload re-initialization pass (reload steps 5 and 8); two established precedents point in opposite directions (process-abort vs transactional rollback). — refuted: tiebreak: The gap claim rests on 016-153 being startup-scoped (true) with no rule for reload-time traps. But the finding missed the UNSCOPED universal trap rules that cover any behavior evaluation. SPEC 13.13.1 (18799-18802): 'The runtime does not isolate traps within behavior invocations. There is no ... continuation past a trap. A trap is a bug, and bugs end the program.' SPEC 13.13.3 (18845-18848) / 026-9 (LOG:3174): 'A behavior that traps aborts the process, same as a free function or function-body trap.' Neither carries steady-state scoping. Reload step 8 (028-21) recomputes derived *behaviors* (028-72 classifies derived as behaviors); step 5 (028-18) runs initializers -- both are behavior/initial evaluations the abort rule covers. The finding's competing rule 028-3 is scoped to COMPILATION failure ('If reload compilation fails...') which 028-14 (LOG:3315) confirms completes host-side BEFORE reload(diff) is called; it is a different phase and never governs a runtime trap. Section 028 contains NO transactional-rollback rule for the runtime reload sequence on a runtime trap. So there is one covered outcome (abort), not two competing precedents. Abort rule and 028-3 govern distinct phases and are consistent -> not refile_divergence. Gap dissolved by explicit unscoped normative text.
- **F236** 029-80 asserts public operators 'may be re-exported', but 003-49 restricts re-export to alias-type and wrapper-fn forms, neither of which can name/host an operator, so no mechanism exists to satisfy the claim. — refuted: tiebreak: As filed, the finding is a CONTRADICTION requiring 003-49 to be a closed/exhaustive enumeration ('restricts re-export to alias-type and wrapper-fn forms') that forbids what 029-80 requires. The verbatim text defeats that premise: 003-49 (LOG:152) says re-export 'requires an explicit re-declaration SUCH AS a public alias type or a wrapper function' -- 'such as' is exemplary, not closed. SPEC 10.4.2 (7785-7787) confirms: 'write an explicit re-declaration rather than a re-exporting use. Common forms:' -- 'Common forms' is explicitly non-exhaustive. So 003-49 imposes a general requirement with two examples and does NOT forbid an operator re-export. 029-80 and 003-49 are therefore not jointly unsatisfiable -- there is no rule an implementer 'cannot satisfy both sides of.' Dissolving passages are the finding's own quotes read correctly; consistent with 029-80 -> not refile_divergence. NOTE for orchestrator: a distinct GAP question survives -- no operator re-export/alias construct is defined anywhere in the LOG (grep confirms only 'public alias type' for types and wrapper fn; no 'alias operator' form; 029-112/113 permit operator values only as params/attrs/returns, not top-level re-exposure). That reframed gap ('does any construct actually host/re-export an operator?') is a separate finding, not the contradiction F236 filed, and is not adjudicated here.

## Appendix C — Method

One Fable Director (charter → triage → deepening → synthesis, memo-threaded), Opus 4.8 worker fleets, deterministic script plumbing (buckets, ledger, retry, assembly). Read-only throughout; all evidence quoted verbatim from the pinned corpus. Verification: dual-lens (evidence + category-conditional charity) per HIGH/MED cluster, evidence-only for LOW, tiebreak on HIGH splits. Agents: 171. Output tokens spent (all fleets): ~3088k.
