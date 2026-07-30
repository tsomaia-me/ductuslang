# Ductus Compiler & Runtime — Implementation Strategy

*2026-07-20 · Status: strategy baseline; §2 shape nodded and §4 axes RULED by owner (2026-07-20). NOT an execution plan: each implementation phase still runs the standing protocol (plan → approval → execution → verify). Corpus baseline: commit d719517 (Part 2 closed; Part 3 survey delivered, consent/rulings in flight).*

---

## 1. Premise

Part 3 is the last prose-side audit. After it, the implementation is the audit: the remaining bug classes — soundness of the commit model under real dirty-sets, feasibility of the static checks (reach-edge closure, candidate envelopes, incoming-view arithmetic), composition of the ownership rules — are only discoverable by building.

Two known limits of implementation-as-audit, each with a structural answer:

1. **A compiler audits in dependency order, not severity order.** Built front-to-back, it audits GRAMMAR in week one and §13 — the riskiest part — months later. Answer: runtime-first sequencing (§2.1).
2. **A compiler audits soundness, not ergonomics.** Whether Ductus is pleasant is revealed by writing Ductus programs. Answer: the conformance-corpus track (§2.4) plus continuous program-writing against each milestone.

## 2. Pinned shape (nodded 2026-07-20)

### 2.1 Runtime-first via the normative IR

The IR text form is normative (033-53: the serialization the spec defines is what tests assert against). Therefore the runtime is built and tested against **hand-written `.duir` files before any frontend exists**:

```
graph {
  scope App exposes [] effects [] {
    signal App.tick : i64 init 0
    derived App.doubled : i64 uses B1 inputs [App.tick]
  }
}
```

Drive with `write_signal` + `commit()`; assert the commit loop, value-change gating, freeze/snap, topological order. §13's semantics get audited first, against tiny witnesses, while the frontend is unwritten. Every hand-written `.duir` later becomes a frontend golden test (frontend output ⊇ hand-written expectations).

### 2.2 Walking skeleton over full-fidelity frontend

Milestone 0 of the full pipeline is a brutal slice — signals, deriveds, one node type, one wire, `@root`, the commit loop; no generics, no traits, no streams, no repeat — end to end (lex → parse → check → IR → run). Each subsequent milestone widens one spec chapter and re-audits it end to end. No phase builds full-fidelity breadth at one layer before the next layer exists.

### 2.3 The divergence protocol (pinned BEFORE the first line of code)

When the implementation contradicts the corpus, the fix is a **ruling, never a code workaround**:

1. Implementer files a divergence finding (spec says X; implementation needs/reveals Y), with corpus citations and the failing witness (program or `.duir`).
2. Owner rules.
3. Edit order unchanged: ledger → DECISION_LOG → SPEC (→ GRAMMAR/IR_GRAMMAR) → code.
4. No code merges against un-ruled divergence. Silent workarounds are the failure mode that forks the spec and destroys the audit value; treat one as a systematic error.

Implementation findings are filed exactly like audit findings — same ledger, same discipline.

### 2.4 Conformance corpus from day one

Every Ductus-tagged fence in SPEC.md (the corpus-wide fence-tag convention, 002-35) is harvested by a script into a test corpus, each example annotated with the milestone that claims it. An example that fails to lex/parse/check/run under its claiming milestone = a finding. The corpus grows with every worked example added to the SPEC and every witness program written for a divergence.

**Dual-route equivalence law (from the B ruling):** the `.duir` text form is a fully self-contained alternative route — it carries everything the in-memory handoff carries, behavior bodies included — and MUST produce identical results. CI test: every corpus program runs once via the in-memory handoff and once via emit-text → parse-text → run; the two commit traces must be identical. A divergence is a finding against whichever route lies.

### 2.4a Concurrency model (pinned by the concurrency amendment, 2026-07-20)

No execution-resource constructs exist in the language — concurrency-of-waiting is the effect system; parallelism is invisible backend latitude. Implementation consequences: (1) the runtime owns a worker pool serving the `compute` builtin (runtime-as-reconciler, cancel-on-suspend, latest-wins) and `deferred` cell evaluation (inputs snapshotted at the dispatching commit, publication barrier at dispatch+N, up-to-N-run pipeline); (2) the M0/M1 core stays strictly sequential — the as-if parallel DAG evaluation is a later backend optimization that must prove trace-identity against the sequential oracle (the reference interpreter differential harness extends naturally to this); (3) the `deferred` publish barrier and the compute effect's write-back path reuse the ordinary host-write queue — no new synchronization surface, and completed deferred results surface as staged work through the same queued-write surface (event-driven hosts commit on readiness). Multi-graph bridging is deferred to the host era (M6) per the doctrine trio; value-only materialized crossings mean the bridge needs no shared-memory machinery ever.

### 2.5 Ergonomics track

Alongside each milestone, real programs are written in the slice it supports (the studio/counter/mixer family from the SPEC's own examples first). Friction findings are filed like any finding; they audit the design's taste, which the compiler cannot.

## 3. Milestone sketch (subject to §4 rulings; granularity indicative)

- **M0 — IR runtime kernel.** Parse IR_GRAMMAR text form; cells (signal/derived) + scopes; `write_signal`/`commit()`; snapshot reads; value-change gating; topological evaluation + tiebreakers. Hand-written `.duir` only.
- **M1 — runtime semantics widening.** Recurrents (history, delay-element law), gates (freeze/snap, effective activation via gate_parent), wires (engagement vs liveness, re-point), streams (ring/gate policies, cursors, observation cells).
- **M2 — walking-skeleton frontend.** The §2.2 slice, end to end, replacing hand-written IR for that slice; golden-test against M0/M1 `.duir` corpus.
- **M3 — static semantics.** Name resolution (scope chain, anchors), visibility/barrel, ownership categories A–D, borrow/storability rules, Handle/WeakHandle/Portal.
- **M4 — the reactive type system.** Kinds/channels, traits + fulfill + markers, generics/monomorphization, endpoint typing, cycle analysis (Circularity, candidate envelopes), incoming-view arithmetic, root closures + liveness lints.
- **M5 — dynamic structure & effects.** repeat/keying, collect/yield/fold, effects + reconciler protocol + suspend/resume, interpretation bootstrap, hot reload (behavior identity, diff, reload plan).
- **M6 — host & CLI.** `ductus run`, watch mode, a minimal native host (clock signal + print/log reconciler), panic traces.

Each milestone = one plan doc, owner approval, execution, verification chain — the standing protocol, unchanged.

## 4. Ruled axes (owner, 2026-07-20)

- **A — Host language: Rust.** Dependency philosophy: minimal/boring; cranelift is sanctioned by the C ruling, LLVM stays out.
- **B — Two-product architecture.** In-memory handoff is the primary route (shared `ductus-ir` crate holding graph-spec + behavior-table types with text parse/emit both directions). The text form is a fully self-contained ALTERNATIVE route: it must carry the entire context the in-memory path carries — behavior bodies included — and produce IDENTICAL results (the dual-route equivalence law, §2.4).
- **C — Behavior execution (respelled 2026-07-20 after the iOS/embedded ruling): cranelift, both modes; AOT is a COMMITTED requirement, not a maybe.** Dev/watch = cranelift-JIT with full hot reload. Release and no-JIT targets (iOS, consoles, locked-down embedded, **and the browser/WASM**) = cranelift-AOT: behaviors precompiled to object code and linked in — no runtime codegen, which is what those platforms actually ban. Cranelift fact: its backend matrix (x86-64, aarch64, s390x, riscv64) does not emit WASM, so the browser tier's behavior-body codegen ladder is: (1) the reference interpreter engine (day one; also powers in-browser dev-mode hot reload with no codegen), (2) a small direct behavior-IR→WASM emitter when web performance matters (WASM is a typed stack machine — no regalloc; the mapping from behavior IR is nearly mechanical; keeps rustc out of web builds), (3) the arm's-length emit-Rust→rustc contingency, shared with the MCU tier, likely never built for web. Invariants unchanged on every tier: graph = data, behaviors = the sole codegen, no program transpilation, rustc never in the dev loop. Host cadence note: the runtime is cadence-agnostic by construction (commit() is host-called; no internal clock) — metronomic hosts (game/audio) and event-driven hosts (web: commit on batch flush + on staged-work-readiness, no beats while quiet) drive one identical core. The spec cooperates: `reload` is an optional negotiated capability outside the mandatory core (§13.14.11), so an AOT runtime that omits it is fully conformant; the reference interpreter is the universal fallback engine (any Rust target, incl. MCU-class outside cranelift's matrix) and can serve on-device reload if ever wanted. LLVM remains a possible future AOT backend on exactly two triggers: MCU-tier codegen needs, or profiled evidence that leaf-expression quality matters — neither forecloses anything built now; its likely shape is emit-Rust-source-for-behavior-bodies + rustc as the arm's-length backend (build-time only), never llvm-sys bindings. Architecture pin: the user's program compiles to graph DATA (walked by the fixed native engine) + behavior machine code (the sole codegen); no transpile-to-Rust, and rustc is never in the user's compile loop. Behaviors are pure params→value functions with two-tier identity (wide content hash for reload matching, per-build u32 handle for dispatch) — corpus-pinned, friendly to every engine listed.
  **Day-one guardrails the AOT commitment imposes:** (1) the behavior-engine seam (table: hash → executable) is the ONLY codegen point in the system — the runtime never synthesizes code anywhere else and stays fully data-driven; (2) the loader binds precompiled behaviors as a first-class path, not a hack around the JIT path; (3) capability advertising is per-target from v0 (dev runtime advertises `reload`; AOT runtime omits it).
- **D — Compilation model: D3.** Whole-program batch, with two insurance policies: content-addressed behavior hashing carries watch-mode latency (behavior-only edits skip graph re-diffing), and every pass stays a pure `(inputs) → outputs` function so a query engine can slot under it later.
- **E1 — Single-threaded deterministic core.** Also corpus law: exactly one driving context may write/evaluate/commit (027-46/47); host threads enqueue.
- **E2 — Static evaluation order** computed at load and after reload; per-commit work is a dirty-set walk over it. Dynamic-dependency events (re-points, mounts) re-establish edges; they do not trigger per-commit DAG rebuilds.
- **E3 — Storage follows the spec's own model literally:** word-sized in-cell values, `pool_index` into per-type pools otherwise, generational slot maps behind Handle/Portal stamps, reserved-capacity index-flip publish, wait-free snapshot reads, producer-side reclamation.
- **F1 — Hand-written lexer** (indent stack + layout-suspension depth, INDENT/DEDENT synthesis, tab-in-leading-whitespace error, `handle!` single token). **F2 — Hand-written recursive-descent parser** (GRAMMAR's lookahead/post-parse notes are its design doc). **F3 — Diagnostic registry enum from day one**: every diagnostic carries a normative class id; conformance tests assert ids, never message strings. **F4 — Legality is a distinct post-parse pass** with its own test corpus (the grammar deliberately over-admits).
- **G — Testing: all five tracks.** Conformance corpus (§2.4), commit-trace tests, semantic property tests, `.duir` round-trip byte-stability, dual-route equivalence (§2.4), and the findings ledger: implementation findings live in `docs/implementation/FINDINGS.md` (numbered-atomic, same discipline), cross-linked into the audit ledger when a finding triggers a ruling.
- **H — Repo layout confirmed:** cargo workspace rooted at `packages/ductus-lang/` with `crates/{ductus-ir, ductus-runtime, ductus-frontend, ductus-cli}` + `corpus/`; docs stay the sibling source of truth; no monorepo constraints exist today.
- **Process:** no work estimates in human time units — scope is stated as deliverables, dependencies, and exit criteria (LEARNINGS #27).

## 4a. Remaining open items

1. **M0 behavior execution — RULED (a)** (owner, 2026-07-20): M0 runs behaviors from a native-closure stub registry (test-harness seam); cranelift lands as its own immediately-following milestone. The reference-interpreter-as-differential-oracle (tree-walk engine as a permanent test oracle, never a product path) stands as proposed unless struck.
2. **M0 slice contents** — rides the M0 plan document.
3. **First host/reconciler** — recommendation stands: CLI host, user-declared clock signal, print/log leaf reconciler, with the worker-thread dispatch seam present from v0 (hooks fire inside blocking `commit()` and must not block it).
4. **Naming** — CLI binary `ductus` (the SPEC already speaks `ductus run`); crate names above are working names.

## 5. Interlock with Part 3

Part 3 rulings land before the milestones that touch them (most affect M3–M5; M0/M1 are exposed mainly to tiebreaker and provenance-exception items — flagged in the survey's ruling queue). If an M0-relevant ruling is still open when M0 planning starts, M0's plan names it as a blocker rather than assuming an answer.
