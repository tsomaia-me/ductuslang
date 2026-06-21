# Ductus — Decision Backlog (rulings & approvals only)

Open items that need an **owner decision**: genuine design forks, plus syntax/semantic questions
the spec decides nowhere. This file deliberately holds *only* what requires a ruling or your
approval of a strong recommendation.

**Contents: 0 open items.** The backlog is empty — every captured ruling/approval item has been
ruled and applied to `DECISION_LOG.md` + `SPEC.md`. The companion ledger
(`DECISION_LOG_FINDINGS.md`) is likewise at **0 open findings**.

**Resolution history.**

- **Design forks (6)** — F020 (trait named-call binds trait param names), F129/F134 (behavior
  identity hash vs u32 handle), F133 (`move` on field l-value paths), F135 (named gate objects),
  F138 (normative IR text grammar), F140 (observability demoted to implementation-defined) — ruled
  and applied 2026-06-21. See `DECISION_LOG_FINDINGS.md` and git history.
- **Open-syntax questions** — all found already decided in the log (the syntax sweep mis-flagged
  them); marked ✓ RESOLVED in the appendix, with one amendment adding the opt-in `...` rest token
  (006-31/006-32).
- **Open-semantic questions (3)** — ruled and applied 2026-06-21:
  - Multi-segment place-assignment LHS (`r.a.b = x`, `arr[i].field = y`, `t.0.field = z`) with
    left-to-right place resolution — **013-248**, SPEC §11.11.
  - Tuple-component assignment `t.0 = x` through a `mut` tuple binding — **013-249**,
    SPEC §11.11/§11.12.
  - Opt-in borrow-rootedness return annotation `-> T from v` (union `-> T from (v, w)`) —
    **013-246/013-247**, SPEC §11.7.5.

When a new ruling/approval item appears, capture it here (one ticket at a time, with discussion)
using the template below before it is opened.

---

## Ticket template (for future items)

```
## <N>. [<tier>] <Title>

**Problem.** One-paragraph statement of the contradiction/gap/decision.
**Context.** The §§ involved, what the spec/log currently says (quoted), the competing readings.
**Potential solutions.** The options with tradeoffs (A/B/C…).
**What.** The concrete decision/edit needed (log entry + spec section).
**Why.** Blast radius — what depends on it.
**Refs.** §§ · DECISION_LOG entries · status.
```

Legend — tier: `foundational` (reshapes a subsystem) · `cluster-root` (cascades to siblings) ·
`leaned-reshape` (clear recommendation, but it changes a rule, so it needs your nod) ·
`open-syntax` / `open-semantic` (spec-silent).
