# DECISION_LOG.md — Production Plan (Operating Manual)

**This document is the single source of truth for this task.** It is written to be executable by an agent with NO prior context: every settled requirement, protocol, template, and acceptance criterion is embedded here. If the session is compacted or a new agent takes over, read this document fully, then follow §8 (Resume Protocol).

---

## 0. Context & Goal

`packages/ductus-lang/docs/SPEC.md` (21,818 lines, ~350k tokens) is the sole design document for the Ductus language (GRAMMAR.md does not exist yet). Normative content is interleaved with rationale, worked examples, and verbose prose — too large for agents to consume routinely.

**Goal:** create `packages/ductus-lang/docs/DECISION_LOG.md` — a complete, lossless, atomic, numbered serialization of **every decision the spec makes**: lexical, syntactic, semantic, type-system, ownership, reactive, runtime/host-API, IR, tooling, conformance — plus explicit exclusions ("no X in v1") and implementation-defined/stdlib delegations, which are decisions too. Every topic matters equally: nodes, connections, types, promotion, traits, tuples, reactivity, streams, effects — no topic is second-class.

The log has two roles:
1. **Compressed lossless WHAT** — devoid of "why" unless the why is load-bearing for losslessness.
2. **Index / primary agent entry point** — agents read the log first; reach into SPEC.md (via the § reference) only for rationale/grounding. Future edits land in the log first, then in the referenced spec section.

Expected magnitude: 1,500–3,000+ entries (measured density ~1 decision per 8–12 prose lines over ~13,200 prose lines). **No cap** — completeness dominates count.

## 1. Deliverables

- **Create** `packages/ductus-lang/docs/DECISION_LOG.md` (header + `<!-- BEGIN LOG -->` + sparse `##` topic headers + numbered entries).
- **Edit** `CLAUDE.MD` — append the "Project Map & Reading Protocol" section (exact text in §2.6).
- **Deliver to user**: final entry count, per-topic distribution, and the full **findings ledger** (§3) partitioned genuine/resolved.
- **Commit & push** to branch `claude/affectionate-davinci-tpfmgm` (checkpoints per phase; final commit contains only the two deliverable files, scratch removed).

## 2. Settled Requirements Registry

### 2.1 Entry format
`N. <one terse sentence>: <one minimal example> (§x.y.z)`
- Entire entry on ONE line. Example is inline code, target ≤ 80 chars, never fenced.
- Example present only when applicable (syntax/semantic items). **Reuse the spec's own example, minimized. Never invent syntax not derivable from that subsection** — no example beats a hallucinated one.
- Trailing parenthesized reference: the **deepest applicable numbered subsection**. Spec headings are irregular — top level carries a trailing dot (`## 13.`), depth-4 numbers appear at two heading levels, a `.0` subsection exists (`13.3.0`), and 13 `#####` headings are unnumbered: parse the number token, strip trailing dots, and under an unnumbered heading cite the nearest numbered ancestor. Max depth is 4 components.

**Good entries:**
```
1. Ductus is a general-purpose, statically typed language in which compositional reactive systems are first-class. (§1.3)
112. The `/` operator always produces a `Float` result, even on uniformly-integer operands: `10_i32 / 5_i32` evaluates to `2.0_f64`. (§4.4.1.1)
405. `move` is legal only as an immediate sub-expression of a call argument list: `f(move x)`. (§11.8.5)
```

**Bad entry → mandatory split (the atomicity test):**
```
✗ `/` is mathematical division producing Float and `\` is truncating integer division: `5 / 2` is `2.5`; `5 \ 2` is `2`.
```
Two claims, two examples → split (and the split surfaces a third hidden decision):
```
✓ The `/` operator always produces a `Float` result, even on uniformly-integer operands: `5 / 2` is `2.5`. (§4.4.1.1)
✓ `\` is truncating integer division, defined on `Integer` operands only: `5 \ 2` is `2`. (§4.4.1.2)
✓ `\` applied to `Float` operands is a type error: `5.0 \ 2.0` ✗. (§4.4.1.2)
```
**Rule:** if an entry states two things or needs two examples, split it. Refinements may sit directly below the decision they refine even when their § is later; multiple entries may cite the same §.

### 2.2 Inclusion / exclusion / losslessness test
- **Include:** all normative content; explicit exclusions ("no X in v1" — likeliest silent omissions, sweep for them); implementation-defined and stdlib delegations; defaults; reserved sets; normative diagnostic *classes* (not wording); normative cost models (e.g., required O(1) bounds).
- **Strip:** rationale/"why", trade-off discussion, worked examples, non-normative notes, diagnostic wording.
- **Losslessness test:** omitting a clause changes what a conforming implementation must do → it is WHAT, keep it; otherwise it is WHY, strip it. A "why" is kept only when losslessness requires it.
- **Tables:** capture the governing rule when the spec states one; enumerate rows only when irreducible (e.g., §4.5.1 reduces to its stated principles; §13.18.9's operator catalog is largely irreducible).

### 2.3 Ordering model
- The log is ordered **generic → specific**, NOT in spec order. Topic-contiguous: each topic's decisions stay together, internally generic→specific; topics sequenced by depth of their entry point. **Gradient ranks specificity, never importance.**
- **Sparse topic headers**: ~25–35 one-line non-normative `##` headings (initial outline in §4).
- §1.3 philosophy statements live once, in the leading Identity tier — never duplicated as per-topic leads.
- Prefix property: reading decisions 1..k yields a coherent partial model at any k.

### 2.4 Numbering invariants
- IDs are **stable identities**: never renumbered, never reused. New decision = next free integer, *placed* anywhere by topic (position ≠ identity). Amendments edit in place keeping the number. Revocations delete the line; the number is retired (gaps allowed).
- At initial assembly, numbers are assigned 1..N in final **file** order (i.e., after arrangement), so number order coincides with the gradient on day one only; thereafter position carries the gradient.

### 2.5 DECISION_LOG.md header (exact text)
```
# Ductus Decision Log

Decision-of-record for the Ductus language. One atomic decision per numbered entry: `N. <sentence>: <example> (§ref into SPEC.md)`.
This log is the lossless WHAT; SPEC.md is the normative elaboration holding the WHY — follow an entry's (§) reference for rationale and full nuance. SPEC.md must conform to this log; divergence between the two is a defect.
Ordering: generic → specific, topic-contiguous (## headers); reading any prefix yields a coherent partial model. Position carries this gradient; numbers are stable identities — never renumbered, never reused. New decisions take the next free integer and are placed by topic; amendments keep their number; revoked numbers are retired (gaps allowed).
Edit protocol: change this log FIRST, then update the referenced SPEC.md section to conform.
If you discover a contradiction, ambiguity, or incoherence in either document: stop and disclose; never silently resolve it.

<!-- BEGIN LOG -->
```

### 2.6 CLAUDE.MD addition (exact text, appended at end of file)
```
# Project Map & Reading Protocol (Ductus)

- `packages/ductus-lang/docs/DECISION_LOG.md` — the decision-of-record: every language decision, atomic, numbered, ordered generic→specific. Read this FIRST for WHAT Ductus is and does.
- `packages/ductus-lang/docs/SPEC.md` — the normative elaboration: rationale, full nuance, worked examples. Reach it via a decision's (§x.y.z) reference, for WHY.
- Edit protocol: change DECISION_LOG.md first (amend in place; new decisions take the next free number, placed by topic; never renumber or reuse IDs), then update the referenced SPEC section to conform. Divergence between the two is a defect.
- If you discover a contradiction, ambiguity, or incoherence in either document: stop and disclose; do not silently resolve.
```

## 3. Findings Protocol (NON-NEGOTIABLE)

The log serializes decisions; it must **never launder defects**. If extraction surfaces a **contradiction** (§a says X, §b says ¬X), an **incoherence** (rules that cannot compose), an **ambiguity** (two defensible readings with different semantics), or an **unsound inference** in SPEC.md:

1. **Never silently resolve.** Nobody picks a reading and serializes it as a clean decision. Flag liberally — a false positive costs minutes; a silently normalized defect poisons the artifact. Unsure → flag, never drop.
2. **Every agent output carries a `FINDINGS:` block** — each item: kind (contradiction | ambiguity | incoherence | unsound-inference); the §§ involved; the competing readings; impact. If none: the explicit attestation `FINDINGS: none`.
3. **Quarantine:** entries affected by a finding are withheld from the log and parked inside the finding with proposed wording for each reading.
4. **Severity:** a foundation-level finding (would invalidate many downstream entries) → orchestrator HALTS the run and surfaces it to the user immediately. All others → aggregate and disclose in full at the end (pre-authorized by user).
5. **Adjudication (Phase D):** orchestrator checks every finding against the spec and partitions the ledger: **(a) genuine — needs user ruling**, quarantined entries attached; **(b) resolved as non-defect**, with the resolving citation. Both partitions are delivered — nothing is hidden in either direction.
6. The shipped log contains **clean decisions only**; quarantined material waits for user rulings.

Calibration example (ambiguity-candidate flavor, not a contradiction): §4.4.3's table attributes `<`/`<=`/`>`/`>=` to `Ord`, while §4.9.1 defines `Ord` as a methodless umbrella whose dispatch targets are `Lt::lt`/`Le::le`/`Gt::gt`/`Ge::ge` — flag and adjudicate, don't normalize.

## 4. Topic Outline (initial; finalized as `outline.md` in Phase C2)

Order below = file order. C2 may refine topic membership within constraints: topic-contiguous, generic→specific, every move documented in `outline.md`, permutation check (§7) must pass.

1. **Identity & Philosophy** — §1.1–§1.3 (what Ductus is; spec governance; the pillars)
2. **Source Form & Lexical Rules** — §1.4 (keywords lowercase; no `;`; `#` in identifiers; comments; naming conventions)
3. **Modules, Packages & Visibility** — §10
4. **Type System Core** — §2 (placeholders; inference; monomorphization; compile-time eval; const-generics)
5. **Traits** — §3.1–§3.4, §3.6–§3.9 (declarations; satisfies/fulfill; dispatch/UFCS; hierarchies; coherence; @derive; @literal_suffix)
6. **Calls & Argument Forms** — §3.5
7. **Numerics** — §4
8. **Type Composition: `&`, `dyn`, `Type[…]`** — §5
9. **Records, Enums, Newtypes** — §6
10. **Conversions** — §7
11. **Error Handling** — §8
12. **Strings, Tuples, Arrays, Time** — §9
13. **Ownership & Mutability** (incl. closures) — §11
14. **Iteration & Loops** — §12
15. **Reactive System** (layered sub-topics, each its own `##` header):
    Reactive Model & Principles §13.1 · Reactive Declarations (cells) §13.2 · Nodes & Parts §13.3–§13.4 · Keyed Scopes & `repeat` §13.5 · Connections §13.6 · Name Resolution in Bodies §13.7 · Placement §13.8 · Conditional Activation (gates) §13.9 · Evaluation & Commit §13.10 · Cycles §13.11 · Reactivity Boundary & Cell Storage §13.12 · Reactive Errors §13.13 · Runtime Interface §13.14 · Hot Reload §13.15–§13.16 · Operators §13.17 · Streams §13.18 · Effects §13.19
16. **Implementation Model** — §14
17. **Compilation Model & IR** — §15

## 5. Pipeline

Scratch lives in **`packages/ductus-lang/docs/.declog-work/`** (in-repo for survivability; checkpoint-committed per phase; removed in the final commit). Subdirs: `draft/`, `skeleton/`, `reviewed/`, `report/`, plus `PLAN.md` (copy of this document), `STATE.md` (§8), `manifest.tsv`, `outline.md`, `assemble_lint.py`, `lint.txt`.

### Phase 0 — Kickoff (orchestrator)
1. `wc -l SPEC.md` must equal **21,818** and record SPEC.md's git blob hash in `STATE.md`. If either differs, SPEC changed: regenerate all chunk boundaries from the heading grep before proceeding.
2. Extract the global heading set: number-token parse of `^#{2,6} <number>` headings, trailing dots stripped, `.0` accepted.
3. Write `manifest.tsv` (chunk id, start line, end line, owned §s — single source of truth for prompts and the script), `assemble_lint.py` (§7), copy this plan to `PLAN.md`, initialize `STATE.md`. Checkpoint-commit.

### Phase A — Drafting (25 agents, waves of 5, parallel within a wave)
Each drafter is a general-purpose agent whose prompt embeds, verbatim from this plan: §2.1–§2.2 (format registry with the good/bad examples), §3 (findings protocol), plus:
- Its chunk id NN, exact line range, owned-§ list with line numbers.
- Exact `offset/limit` Read pairs (≤1,400 lines each) **and the rule: verify each Read's last line number equals the expected end; issue a continuation Read otherwise** (the Read tool truncates silently at ~25k tokens).
- Every entry's ref MUST be an owned §.
- Output `.declog-work/draft/NN.md`: placeholder numbering `NN-1…NN-K` (zero-padded id, dense from 1); intentional out-of-order refinement lines carry `<!-- ooo -->`; then `COVERAGE:` footer (every owned subsection → entry count); then `FINDINGS:` block (or `FINDINGS: none`); then sentinel `<!-- END NN n=K -->`.
- If nearing output limits: stop at a subsection boundary, emit `HANDOFF: §x.y` — orchestrator spawns a continuation agent. Never silently compress the tail.
- Write only your own output file; never `rm`; never modify SPEC.md.
Draft files are in spec order (arrangement happens in Phase C).

### Gate A→B (orchestrator)
Run per-file lint (§7, C1 checks) on each draft before dispatching its reviewer; malformed drafts go back to a fix-up agent. Reviewer waves pipeline behind drafter waves. Update `STATE.md` per chunk; checkpoint-commit per wave.

### Phase B — Review (25 fresh agents; skeleton-first, anti-rubber-stamp)
Reviewer prompt embeds the same registry + findings protocol, plus this mandatory sequence:
1. **Skeleton first:** read the spec range (same Read-verification rule) and write `.declog-work/skeleton/NN.md` — per owned subsection, 3–6-word keyphrases for every decision found — **BEFORE opening the draft** (audit trail; makes rubber-stamping structurally impossible).
2. Read `draft/NN.md`. Reconcile both directions: every skeleton keyphrase unmatched in the draft → add an entry; every draft entry → check atomicity (two claims or two examples → split), deepest-owned-§ ref, example traceable to that subsection's text, one-line regex shape.
3. Write `reviewed/NN.md` (re-densified `NN-1…NN-K`, `ooo` annotations, COVERAGE footer, FINDINGS block, sentinel) and `report/NN.md`: strict pipe table `§ | skeleton_n | draft_n | final_n | adds | splits | removals`; any `final_n = 0` row must carry `no-normative-content: <reason>`; **every removal individually justified** (never delete silently); `UNRESOLVED:` list (unsure → flag, never drop); `FINDINGS:` (independently re-check drafter findings and add own; quarantined entries live in the report, not in `reviewed/NN.md`).
4. Same HANDOFF escape hatch.

### Phase C — Arrangement + assembly + lint (`assemble_lint.py`; idempotent; nonzero exit on error)
- **C1 per-file** (also the A→B gate): see §7.
- **C2 arrangement:** orchestrator authors `outline.md` — final topic outline (§4) plus the `§-prefix → topic` mapping with a documented exception list. Script permutes the spec-ordered corpus into outline order and inserts `##` topic headers. **Hard check: pure permutation — identical entry multiset before/after (headers excluded).**
- **C3 global lint + renumber** (after arrangement, so day-one IDs ascend in file order): see §7.

### Phase D — Orchestrator final QA
Requires spec familiarity: the original orchestrator holds the full spec in context; a **resuming agent instead reads targeted spec slices** steered by lint stats, reports, and findings (full re-read only if budget allows).
1. Script green before reading anything; read the assembled log per-topic.
2. **Adjudicate every FINDINGS item** (§3.5); build the partitioned ledger.
3. Work lint outputs: coverage gaps; duplicate warnings; density outliers (targeted spec re-read for ±2σ chunks).
4. Audit every reviewer removal and every UNRESOLVED item.
5. Spot-audit ~10 random subsections against the spec; sweep per major section for explicit-exclusion coverage ("no X in v1").
6. Verify topic ordering and `ooo` placements read sensibly; after any edit re-run renumber+lint (header-aware mode).
7. Prepend header (§2.5); write `packages/ductus-lang/docs/DECISION_LOG.md`; append §2.6 to `CLAUDE.MD`.

### Phase E — Delivery
1. Final lint pass green; `wc -l` consistent with N + headers.
2. Final commit: add the two deliverables, `git rm -r` `.declog-work/`; `git status` then shows a clean tree. Push to `claude/affectionate-davinci-tpfmgm` (retry up to 4× with backoff on network failure only).
3. Report to user: entry count, per-topic distribution, full findings ledger (genuine + resolved), and any HANDOFF/restart events.

## 6. Chunk Manifest (25 chunks; boundaries verified to land on heading lines; valid for SPEC.md @ 21,818 lines)

| # | Sections | Lines | # | Sections | Lines |
|---|----------|-------|---|----------|-------|
| 01 | §1–§2 | 1–1096 | 14 | §13.4–§13.5 | 13055–13842 |
| 02 | §3.1–§3.4 | 1098–2016 | 15 | §13.6–§13.7 | 13843–14376 |
| 03 | §3.5–§3.9 | 2017–2678 | 16 | §13.8 | 14377–15322 |
| 04 | §4 | 2680–3862 | 17 | §13.9 | 15323–15992 |
| 05 | §5–§6 | 3864–5123 | 18 | §13.10–§13.13 | 15993–16742 |
| 06 | §7–§8 | 5125–5861 | 19 | §13.14–§13.16 | 16743–17438 |
| 07 | §9–§10 | 5863–7143 | 20 | §13.17 | 17439–18090 |
| 08 | §11.1–§11.8 | 7145–8213 | 21 | §13.18.1–§13.18.8 | 18091–18746 |
| 09 | §11.9–§11.14 | 8214–8860 | 22 | §13.18.9–§13.18.16 | 18747–19548 |
| 10 | §12 | 8863–10257 | 23 | §13.19 | 19549–20506 |
| 11 | §13.0–§13.2.4 | 10260–11085 | 24 | §14 | 20510–21142 |
| 12 | §13.2.5–§13.2.11 | 11086–11918 | 25 | §15 | 21146–21818 |
| 13 | §13.3 | 11919–13054 | | | |

Gap lines between chunks are blanks/`---` rules only (verified — nothing lost). Chunk 11 is the densest; 21/22 split §13.18; §13.18.9 is a 178-line operator catalog (expect irreducible enumeration).

## 7. Lint Specification (`assemble_lint.py`)

**C1 per-file checks:** file exists/non-empty; sentinel `<!-- END NN n=K -->` present and K == counted entries (truncation detector); every entry line matches `^NN-(\d+)\. \S.* \(§\d+(\.\d+){0,3}\)\s*(<!-- ooo -->)?$` with NN == filename id; placeholder indices dense 1..K; no bare `^\d+\. ` lines, no code fences, no tabs; every § ref ∈ global heading set AND ∈ chunk's owned set; § order nondecreasing except `ooo`-annotated lines; `COVERAGE:` and `FINDINGS:` blocks present. Warnings: entry > 300 chars; atomicity smells (`; `, ` and also `).

**C2 check:** arrangement is a pure permutation — identical entry multiset before/after (headers excluded).

**C3 global checks:** exactly the manifest's chunk set consumed, nothing extra; comments/sentinels/blanks stripped; renumber 1..N; final shape `^\d+\. .+ \(§\d+(\.\d+){0,3}\)$` with sequential IDs (`##` headers exempt); normalized-text duplicate scan (lowercase, example and ref stripped) → warning list; **subsection coverage net: every numbered § in the heading set is cited ≥1×, OR covered by a justified `no-normative-content` report row, OR quarantined in a finding — anything else is an error** (the mechanical approximation of losslessness); per-chunk density stats with ±2σ outlier flags. Header-aware mode (skip lines above `<!-- BEGIN LOG -->`) for post-Phase-D re-runs. Output `lint.txt`; nonzero exit on any error.

## 8. State Tracking & Resume Protocol

`STATE.md` (in `.declog-work/`, updated after every wave and phase, checkpoint-committed) records: SPEC.md blob hash + line count; current phase; per-chunk status table (`drafted | gated | reviewed | -`); HANDOFF continuations spawned; halts; open findings count; next action.

**A new/resumed agent must:**
1. Read `.declog-work/PLAN.md` (this document) fully — it contains every requirement; do not improvise format or protocol.
2. Read `STATE.md`; verify `.declog-work/` contents match it (files present per status table; per-file lint green for chunks marked gated/reviewed).
3. Verify SPEC.md hash/line count still match `STATE.md`; on mismatch, STOP and disclose to the user (line ranges are stale).
4. Continue at the recorded next action. For Phase D without full-spec context: adjudicate via targeted spec-slice reads (each finding and lint flag carries its §§).
5. The findings protocol (§3) binds resumed agents identically — including the immediate-halt rule.

## 9. Verification & Acceptance Criteria

- `assemble_lint.py` exits clean: format, sequential numbering 1..N, all § refs resolve, ownership respected, sentinels/counts consistent, permutation check passed, subsection coverage net satisfied.
- Every reviewer report accounted for: no unjustified zero-decision subsection, no unjustified removal, no open UNRESOLVED, FINDINGS block present in every report.
- Findings ledger delivered in full, partitioned genuine/resolved; no defect serialized into the log; every quarantined entry traceable to its finding.
- Spot-audits find no missing normative content; sentinel decisions present and correctly referenced (e.g., `;` is a lex error §1.4; `/` always produces Float §4.4.1.1; recurrents advance in lockstep §13.2.4.1; gates freeze rather than unmount §13.9.7).
- Final tree contains exactly: new `DECISION_LOG.md`, edited `CLAUDE.MD`; scratch removed; branch pushed.


---

## Addendum A — Operational learnings & continuation procedure (added mid-run)

**Division of labor:** PLAN.md (this file) = the protocol, immutable mid-run except addenda. STATE.md = the live tracker (chunk table, repair notes, recovery TODO). Trust STATE for position, PLAN for rules.

**Session-limit failure mode (observed).** Subagents can be killed mid-flight by usage limits ("You've hit your session limit"); their final replies are lost but their FILES may be complete — the agents write outputs before replying. Recovery loop, per affected chunk:
1. Check disk: output file exists AND `assemble_lint.py gate` passes (the sentinel is the truncation detector) → ACCEPT the work; the lost reply does not matter.
2. File missing or gate-failing → relaunch that role from scratch (Write overwrites stale partials; a fresh reviewer writes its own skeleton — never reuse a dead reviewer's skeleton for a fresh review).
3. reviewed/NN.md green but report/NN.md missing → launch a REPORT-RECONSTRUCTION agent: reads the spec range + draft/NN.md + reviewed/NN.md, independently diffs draft→reviewed, verifies every delta against the spec text (flagging unjustified removals as UNRESOLVED), regenerates the per-§ tally, and writes the standard report. This substitutes for the lost audit because the reconstructor re-verifies the deltas itself.

**Quarantine-violation patterns (3 repairs to date — see STATE Phase D notes).** Watch for: (a) finding recorded but contested entries left serialized in the list; (b) invented in-list annotations (the ONLY legal trailing comment on an entry is ` <!-- ooo -->`); (c) silent resolution by omitting one reading entirely. Repair recipe: move the contested entry text verbatim into the finding's `quarantined:` field, delete the in-list lines, renumber dense, fix COVERAGE counts and sentinel n, re-gate.

**Launch discipline:** ≤ ~10 concurrent agents; reviewers pipeline behind drafters; gate every file on arrival BEFORE dispatching its reviewer; checkpoint-commit (+push) after every gate/launch turn.

**Resume-safe remaining-work algorithm:** for each STATE chunk row — drafted `-` → launch drafter; drafted `x`, gated `-` → gate; gated `x`, reviewed `-` → launch reviewer; all `x` → nothing. Clear the Recovery TODO list first. When all 25 chunks are reviewed AND all 25 reports exist: Phase C (`python3 assemble_lint.py assemble`; fix any coverage-net errors via targeted agents), then Phase D (PLAN §5: findings adjudication, dedup, density outliers, spot audits), then Phase E (header §2.5, CLAUDE.MD §2.6, final commit removing .declog-work, push).

## Addendum B — Prompt construction for relaunched agents

Build prompts per PLAN §5 Phase A (drafters) / Phase B (reviewers), embedding verbatim: §2.1–§2.2 (format registry with the good/bad worked examples), §3 (findings protocol). Per-chunk specifics: chunk id NN; line range and ≤800-line Read slices with the verify-last-line rule; owned-§ list from `manifest.tsv` (`awk -F'\t' '$1=="NN" {print $4}' manifest.tsv`); for reviewers additionally the drafter findings list (read from draft/NN.md's FINDINGS block) and the skeleton-first sequence. Two clauses are MANDATORY verbatim in every prompt: the quarantine form (contested entries OUT of the list, wording inside the finding; `quarantined: none` only when no entry affected) and the only-ooo-comment rule. Output-file layouts: PLAN §5. Final replies must be short summaries (counts, findings, handoff) — never entry dumps.
