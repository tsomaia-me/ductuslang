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

**Contents (3 items):** Section 1 — open semantic questions (3). (The 6 design forks once tracked
here were all ruled and applied on 2026-06-21 — see the `DECISION_LOG_FINDINGS.md` ledger, now 0
open, and git history — and removed; the open-*syntax* questions were likewise found already decided
and removed, one amended to add the opt-in `...` rest token, 006-31/006-32.)

**How to use.** These open-semantic questions need real rulings — grammar won't settle them; open
them one at a time, with discussion.

Legend — tier: `foundational` (reshapes a subsystem) · `cluster-root` (cascades to siblings) ·
`leaned-reshape` (clear recommendation, but it changes a rule, so it needs your nod) ·
`open-syntax` / `open-semantic` (spec-silent).

---

# Section 1 — Open semantic questions (grammar won't help)

3 items needing real rulings, not syntax decisions.

## 1. [open-semantic] Multi-segment assignment LHS + desugar order  (appendix)

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

## 2. [open-semantic] Tuple-component assignability through a `mut` binding  (appendix)

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

## 3. [open-semantic · CL-OWNERSHIP] Optional surface form for borrow-rootedness (incl. union)  (reframed from old elaborated-borrow item)

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
