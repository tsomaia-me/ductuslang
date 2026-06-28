# Ductus Design Backlog

Working record of design decisions reached in discussion but **not yet written into
`DECISION_LOG.md` / `SPEC.md`**. This is the bridge between a design conversation and the
decision-of-record: once a `LOCKED` item is amended into the log+spec, it should be removed from
here (or marked LANDED).

It is **not** normative. `DECISION_LOG.md` remains the decision-of-record; where this file and the
log disagree, the log wins until an amendment lands.

**Status legend:** `LOCKED` (agreed, awaiting spec amendment) · `OPEN` (under design, not settled) ·
`DEFERRED` (agreed to handle later) · `DISCARDED` (considered and rejected) · `PARKED` (future-form,
not now).

Entry shape: **Background** (origin, the problem) → **Decision** → **Why** → **Example** →
**Spec impact**.

---

## 1. Triage of the initial 13 proposals

A batch of 13 proposals was reviewed. Several resolved to "no spec change," recorded here so they
are not re-litigated.

### 1.4 `PARKED` — Future-forms list (proposal #3)
- **Background:** A parking lot of "maybe later" forms.
- **Items:** `[=1 total]` enforced-total cardinality (today totals aren't tracked, 017-124);
  per-member visibility on `to`/`from`/`parts`; an instance knowing its own placement-time name;
  union types (the connection `to:` admission rule is forward-compatible with them, 030-68/019-33);
  a reflection API (would widen what bare `Type[Node]` can do, 008-69).
- **Decision:** Parked; not now. Note: "union types" and "reflection" are the same questions as the
  metaprogramming / `Type[…]` threads and should be revisited together if pursued.
- **Spec impact:** None yet.
---

## 6. Provenance — the 13 proposals → outcome

| # | Proposal | Outcome |
|---|---|---|
| 1 | metaprogramming / tuple iteration | DISCARDED (enums cover) |
| 2 | explicit lifetimes | DISCARDED |
| 3 | future-forms list | PARKED |
| 4 | Sync/Async connection markers | DISCARDED |
| 5 | `as`-name on compile-time `for` | LOCKED — its **own** internal-child mechanism (`for … as`), sibling of `repeat … as`; not absorbed into views (§4.12) |
| 6 | trait-bound `for`/parts | absorbed into views (trait selectors) — §4 |
| 7 | per-declaration named parts | became the views model — §4 |
| 8 | placing groups (chords) | became the Bundle concept — §4.5/4.6 |
| 9 | slots | subsumed by named views — §4 |
| 10 | `instant` type | NO-ACTION (already specified) |
| 11 | `@reset_on_reopen` unification | LOCKED — §2 |
| 12 | `Cell` access/return | LANDED — Phase 17 Step 3 + Phase 19a (Cell as type, no special treatment; cross-instance refs via Handle/WeakHandle/Portal/connection) |
| 13 | portals | LANDED — Phases 3, 15, 16 (Portal[T] type + handle/portal/handle! keywords + lens propagation + collapse rules) |

Plus the cross-cutting **instance citizenship reframe** (§3), which arose while discussing #8 and
underpins §4.
