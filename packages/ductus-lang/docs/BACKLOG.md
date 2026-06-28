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

### 1.6 `DEFERRED` — Cluster B: `Cell` return/storage (#12) + portals (#13)
Deferred to its own discussion. Full framing and numbered open questions (Q8b.1–5) live in
**§5 (§8b)** — not duplicated here.

---

## 5. #8 — Connections under the views model (`LOCKED`) + Cluster B portals/`Cell` (`OPEN`)

### 5b (§8b) `DEFERRED` — Cluster B: portals (#13) + `Cell` return/storage (#12)
Own discussion (was §1.6). Proposal-form open questions; **no decisions yet.**

**#13 Portals — proposal:** a **storable, `Copy`, always-`Option`-on-access** reference to **owned,
non-graph** data (records, arbitrary values), to enable self-referential structures (e.g.
doubly-linked lists). Distinct from `Handle` (graph entities only, 017-145/146); mechanically
near-identical (Copy, Option-on-resolve, address-like).
- **Q8b.1** — Is a portal just **`Handle` generalized** to non-graph data, or a **stdlib slotmap** on
  the `raw_*` intrinsics (033-193) — rather than a new language primitive?
- **Q8b.2** — Liveness for arbitrary owned data needs **generation stamps** (017-157 analog): who pays
  the cost, and how does the compiler decide which allocations to instrument (only those a portal is
  taken to)?
- **Q8b.3** — Are portals ever **writable**? Read-only portals can't splice a doubly-linked list
  (can't set `prev`/`next` after construction) — gutting the headline use case. If writable, how does
  it reconcile with localized mutability (writing through a portal mutates non-local state)?
- **Q8b.4** — A portal into a **record field dangles when the record moves** (category-B storage,
  013-3). Caught at runtime (generation stamp → `None`) or forbidden at compile time?

**#12 `Cell` return/storage — open:** reading a top-level `Cell` inside a function is allowed and
provenance-tracked (025-15/16), but the **ownership category of a `Cell[T]` value is unpinned.**
- **Q8b.5** — Is `Cell[T]` a **call-scoped borrow** (unstorable) or a **freely-copyable storable
  reference** (like `Handle`)? That decides what *returning* a `Cell[T]` from a function means.
  (Connects to portals — both are "references to non-instance data.")
- **Spec impact:** TBD — own discussion.

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
| 12 | `Cell` access/return | DEFERRED (Cluster B) — §5 (§8b) |
| 13 | portals | DEFERRED (Cluster B) — §5 (§8b) |

Plus the cross-cutting **instance citizenship reframe** (§3), which arose while discussing #8 and
underpins §4.
