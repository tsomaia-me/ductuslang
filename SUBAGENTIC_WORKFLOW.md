# Subagentic Workflow — orchestration pattern

A reusable shape for executing heavy, multi-step work (documentation edits, code refactors, multi-site audits) that cannot fit one orchestrator's context window. Built on the Workflow tool.

## When to use

- Work that spans more sites/files than one context can hold (rule of thumb: more than ~10 distinct edit targets, or any task whose plan + sources exceed ~200K tokens).
- Work with mechanical gates that benefit from independent verification per check.
- Work where adversarial cold-reading after the fact catches drift that mechanical gates miss.
- Work the user has explicitly opted into multi-agent orchestration (`ultracode`, "use a workflow", etc.).

Do NOT use this for:
- One-shot edits.
- Research/exploration tasks (use a single Explore agent).
- Anything where the orchestrator can fit the whole thing in context comfortably.

## The six-phase shape (per batch)

```
                    ┌─────────────────────────────────────────┐
                    │  ORCHESTRATOR (you)                     │
                    │  Holds: which batch, halt state, diff   │
                    │  approval cycle with the user           │
                    └────────────────┬────────────────────────┘
                                     │ Workflow({script: batch})
                                     ▼
    ┌────────────────────────────────────────────────────────────────┐
    │  PHASE 1 — Extract                                             │
    │  ─────────────────                                             │
    │  1 agent: read plan + verification reports, produce concrete    │
    │  site list with verbatim quotes pulled from live disk.          │
    │  Output: [{file, locator, verbatim_old, intent, decision_id}…]  │
    └────────────────┬───────────────────────────────────────────────┘
                     │
                     ▼
    ┌────────────────────────────────────────────────────────────────┐
    │  PHASE 2 — Edit pipeline (NOT barrier-parallel)                 │
    │  ─────────────────                                              │
    │                                                                 │
    │   site_A ── [editor] ── [spot-verify] ── result_A               │
    │   site_B ──── [editor] ──── [spot-verify] ── result_B           │
    │   site_C ────── [editor] ────── [spot-verify] ── result_C       │
    │                                                                 │
    │  Stage 1 (editor): fresh-read target, build old_string from     │
    │    disk, apply Edit, run per-edit gates 1-8 inline.             │
    │  Stage 2 (spot-verify): read 10-line context around edit,       │
    │    confirm intent matches outcome, no adjacent bleed.           │
    │                                                                 │
    │  Sites in the SAME file pipeline sequentially (Edit serialization│
    │   requires it). Sites in DIFFERENT files run in parallel.        │
    │  Halt-token from any stage propagates → workflow returns early. │
    └────────────────┬───────────────────────────────────────────────┘
                     │
                     ▼
    ┌────────────────────────────────────────────────────────────────┐
    │  PHASE 3 — Batch gates (PARALLEL barrier)                       │
    │  ─────────────────                                              │
    │                                                                 │
    │   ┌─[gate-count]─┐  ┌─[gate-grep]─┐   ┌─[gate-xref]─┐           │
    │   │ rule-count   │  │ retired-    │   │ cross-ref +  │           │
    │   │ reconcile    │  │ term sweep  │   │ LOG↔SPEC     │           │
    │   └──────────────┘  └─────────────┘   └──────────────┘           │
    │                                                                 │
    │   ┌─[gate-inv]───┐  ┌─[gate-compat]─┐                            │
    │   │ foundation   │  │ cross-batch    │                            │
    │   │ invariants   │  │ compatibility  │                            │
    │   └──────────────┘  └────────────────┘                            │
    │                                                                 │
    │  Five gates run concurrently. Barrier collects all results.     │
    │  Any halt → workflow returns; orchestrator surfaces to user.    │
    └────────────────┬───────────────────────────────────────────────┘
                     │
                     ▼
    ┌────────────────────────────────────────────────────────────────┐
    │  PHASE 4 — Adversarial cold-read                                │
    │  ─────────────────                                              │
    │  1 strong agent (effort: high): reads the staged diff end-to-end │
    │  against the plan. Reports fabricated quotes, missed sites,     │
    │  drift, contradictions. Independent judgment, not checklist.   │
    └────────────────┬───────────────────────────────────────────────┘
                     │
                     ▼
    ┌────────────────────────────────────────────────────────────────┐
    │  PHASE 5 — E2E review (orchestrator forcing function)           │
    │  ─────────────────                                              │
    │  1 capture agent: runs `git diff HEAD`, retired-term grep,      │
    │  byte counts. Stages everything the orchestrator needs to do    │
    │  one final personal e2e read. The agent gathers; it does NOT    │
    │  judge. The orchestrator MUST read the unified diff personally  │
    │  before saying "ready to commit". Trust-but-verify the          │
    │  adversarial verdict.                                            │
    └────────────────┬───────────────────────────────────────────────┘
                     │
                     ▼
    ┌────────────────────────────────────────────────────────────────┐
    │  PHASE 6 — Surface                                              │
    │  ─────────────────                                              │
    │  Workflow returns a structured summary:                         │
    │    { edits, gates, adversarial, e2eReview, haltReasons,        │
    │      orchestrator_required_step }                              │
    │  Orchestrator reads e2eReview.unifiedDiff personally,           │
    │  shows the summary to user, awaits "approve", commits.         │
    └─────────────────────────────────────────────────────────────────┘
```

## Phase responsibilities

### Phase 1 — Extract

**Goal:** produce a concrete, schema-validated site list. No edits.

- 1 agent, narrow context: plan section for this batch + matching per-batch verification report.
- Output schema requires verbatim `old_string` pulled from disk at extract time — NOT from the plan's quoted snippets. The plan is allowed to drift; disk is the source of truth.
- Schema fields: `file`, `locator` (line or section), `verbatim_old`, `intent` (what should change), `decision_id`, `batch_role`.

### Phase 2 — Edit pipeline

**Goal:** apply each site's edit + per-edit gates 1-8 in isolation.

- Use `pipeline()`, NOT `parallel()` between stages. Each site flows edit → verify independently.
- Stage 1 (editor agent) per site:
  - Fresh-read target file at locator
  - Build `old_string` from current disk content (NEVER from cached memory or plan quote)
  - Apply Edit
  - Run per-edit gates 1-8 inline (see SUBAGENT_PROTOCOL_EDITOR.md)
  - Returns `{site_id, status, diff}` or `{halt: true, reason}`
- Stage 2 (spot-verify agent) per site:
  - Read 10-line context window around the edited region
  - Confirm intent matches outcome
  - Confirm no adjacent-paragraph bleed or reference breakage
  - Returns `{site_id, verified, reason}` or halt

**Concurrency rule:** sites in the SAME file pipeline sequentially. Sites in DIFFERENT files run in parallel. Reason: Edit changes file state; same-file parallel edits race the verbatim-old check.

**Halt propagation:** any stage failing → workflow exits to phase 5 immediately, skipping later phases. Orchestrator surfaces to user.

### Phase 3 — Batch gates (parallel barrier)

**Goal:** check the batch as a whole after all sites landed.

Five gates run concurrently:

| Gate | Role |
|------|------|
| gate-count | rule-count headers (`## NNN. — N Rules`) match actual `^NNN-MM.` count for touched sections |
| gate-grep | retired-term residue across the relevant files; zero live occurrences |
| gate-xref | every new `(§)` ref added points to a real SPEC section; LOG↔SPEC conformance |
| gate-inv | foundation invariants still hold (project-specific; e.g., specific decision-relationships hold) |
| gate-compat | no edit in this batch contradicts a decision landed in an earlier batch |

Each gate is a fresh agent reading only what its scope requires. Barrier collects all; any halt → workflow returns.

### Phase 4 — Adversarial cold-read

**Goal:** an independent skeptic verifies the batch against intent. Catches what mechanical gates miss.

- 1 strong agent, `effort: high`.
- Reads the staged diff end-to-end against the plan.
- Reports: fabricated quotes, missed sites, contradictions, drift from plan intent.
- Skeptical stance — does NOT trust the orchestrator's own summary.

See SUBAGENT_PROTOCOL_ADVERSARIAL.md.

### Phase 5 — E2E review (orchestrator forcing function)

**Goal:** stage everything the orchestrator needs to do one final personal end-to-end read of the staged diff.

- 1 capture agent. Mechanical: runs `git diff HEAD --stat`, captures full `git diff HEAD`, greps each retired term listed in the plan, captures post-edit byte counts.
- The agent does NOT judge correctness. It assembles evidence.
- The orchestrator MUST personally read `e2eReview.unifiedDiff` before declaring the batch ready to commit. The adversarial verdict (Phase 4) is the agent's judgment, not the orchestrator's. Trust-but-verify.
- This phase is a forcing function: the workflow returns an `orchestrator_required_step` string that names the obligation explicitly so the orchestrator cannot skip it without noticing.

Why this exists: a GREEN adversarial verdict is necessary but not sufficient. Subagents miss things the orchestrator can catch with a 30-second read of a 200-line diff. The cost is one extra agent run + a few seconds of orchestrator attention; the payoff is catching the one defect the cold-read agent didn't.

### Phase 6 — Surface

**Goal:** return a structured summary to the orchestrator for user-approval handoff.

Workflow returns:
```
{
  edits: [{site_id, status, diff}…],
  gates: {count, grep, xref, inv, compat},
  adversarial: {findings, verdict},
  e2eReview: {findings: [{kind, content}…]},   // contains the unified diff
  orchestrator_required_step: "Read e2eReview.findings[].content end-to-end…",
  haltReasons: [strings]   // empty if clean
}
```

Orchestrator reads `e2eReview.unifiedDiff` personally, shows the summary to the user, awaits explicit "approve", then commits + pushes outside the workflow.

## Outer loop (between workflows)

```
foreach batch in plan.batches:
  1. result = Workflow({script: batch.script})
  2. READ result.e2eReview.findings[].content (the unified diff) end-to-end personally
     — do NOT delegate; this is the forcing function from Phase 5
  3. surface result to user (adversarial verdict + your own e2e read findings)
  4. if result.haltReasons.length > 0 OR your e2e read found issues:
       do NOT commit; await user direction
  5. else:
       await user "approve"
       on approve:
         git add <touched files>
         git commit -m "<batch message>"
         git push -u origin <branch>
  6. next batch
```

Orchestrator holds: which batch is next, last user directive, high-level halt state. Nothing batch-internal.

## Token budget guidance

Per batch (rough order of magnitude):

| Phase | Cost |
|-------|------|
| Extract | ~10-20K |
| Edit pipeline | ~10-15K per site × N sites |
| Batch gates (5 parallel) | ~40-80K |
| Adversarial | ~60-120K (high effort) |
| E2E review (capture) | ~5-10K (mechanical) |

For a 30-site batch: ~500K-700K total. For a 7-site batch: ~150-250K. Plan to budget the largest batch + 50% headroom.

The orchestrator's own e2e diff read costs orchestrator-context tokens (proportional to diff size). A 500-line diff costs ~3-5K. Budget for it in the parent loop.

## Choice rationale: pipeline vs barrier

**Default to `pipeline()`.** Each item flows through all stages independently — wall-clock = slowest single chain, not sum of slowest-per-stage. A 30-site pipeline finishes about as fast as a 1-site pipeline.

**Use `parallel()` barrier only when stage N genuinely needs ALL of stage N-1's results together.** Examples: dedup across all findings before verification, early-exit if total count is zero. Phase 3 (batch gates) IS a barrier because the gates run on the post-edit state of the whole batch.

## Schemas

All structured-output fields end the agent's run as raw data, not chatty prose. Validate at the tool-call layer (JSON Schema in the `schema` option of `agent()`). Required fields per role are specified in the per-kind protocol files.

Universal fields every agent MUST return:
- `protocol_acknowledged: boolean` — true if the agent read the workflow protocol.
- `halt: boolean` — true if the agent could not complete within protocol.
- `halt_reason: string` (if halt is true).

## Example skeleton

A full, fill-in-the-blanks reusable template lives at the repo root: **`SUBAGENTIC_WORKFLOW.template.js`**. Copy it, fill in the four CUSTOMIZE blocks (meta, hard-rules, learnings, project-context) and the six SCOPE constants, and run it via `Workflow({scriptPath: '<copy-path>'})`.

Skeleton sketch (the template has the full version):

```javascript
export const meta = {
  name: 'batch-foo',
  description: 'Project Foo batch X',
  phases: [
    {title: 'Extract'}, {title: 'Edit'}, {title: 'Batch gates'},
    {title: 'Adversarial'}, {title: 'E2E review'}, {title: 'Surface'},
  ],
}

// PHASE 1 — Extract
phase('Extract')
const sites = await agent(EXTRACT_PROMPT, {schema: SITE_LIST_SCHEMA})
if (sites.halt) return {halt: true, phase: 'Extract', reason: sites.halt_reason}

// PHASE 2 — Edit pipeline
phase('Edit')
const editResults = await pipeline(
  sites.entries,
  s => agent(`Edit ${s.file}:${s.locator}. ${EDITOR_BRIEF(s)}`,
            {phase: 'Edit', schema: EDIT_SCHEMA}),
  (e, s) => agent(`Spot-verify ${s.file}:${s.locator}. ${VERIFY_BRIEF(s, e)}`,
                  {phase: 'Edit', schema: VERIFY_SCHEMA})
)
const editHalts = editResults.filter(r => r?.halt)
if (editHalts.length > 0) return {halt: true, phase: 'Edit', reasons: editHalts}

// PHASE 3 — Batch gates (parallel barrier)
phase('Batch gates')
const gates = await parallel([
  () => agent(GATE_COUNT_PROMPT,    {phase: 'Batch gates', schema: GATE_SCHEMA}),
  () => agent(GATE_GREP_PROMPT,     {phase: 'Batch gates', schema: GATE_SCHEMA}),
  () => agent(GATE_XREF_PROMPT,     {phase: 'Batch gates', schema: GATE_SCHEMA}),
  () => agent(GATE_INVARIANT_PROMPT,{phase: 'Batch gates', schema: GATE_SCHEMA}),
  () => agent(GATE_COMPAT_PROMPT,   {phase: 'Batch gates', schema: GATE_SCHEMA}),
])

// PHASE 4 — Adversarial cold-read
phase('Adversarial')
const adversarial = await agent(ADVERSARIAL_PROMPT,
                                 {schema: ADVERSARIAL_SCHEMA, effort: 'high'})

// PHASE 5 — E2E review capture (orchestrator forcing function)
phase('E2E review')
const e2eReview = await agent(E2E_CAPTURE_PROMPT,
                              {phase: 'E2E review', schema: GATE_SCHEMA})

// PHASE 6 — Surface
return {
  edits: editResults,
  gates,
  adversarial,
  e2eReview,
  orchestrator_required_step:
    'Read e2eReview.findings[].content (the unified diff) end-to-end personally before approving.',
  haltReasons: [
    ...(adversarial.halt ? [adversarial.halt_reason] : []),
    ...(adversarial.verdict === 'RED' ? [adversarial.verdict_one_line] : []),
  ],
}
```

## Protocol files (must read together)

- **WORKFLOW_PROTOCOL.md** — universal protocol; every agent reads first.
- **SUBAGENT_PROTOCOL_EDITOR.md** — editor agents (Phase 2 stage 1).
- **SUBAGENT_PROTOCOL_GATE.md** — gate agents (Phase 3).
- **SUBAGENT_PROTOCOL_ADVERSARIAL.md** — adversarial agents (Phase 4).
- **SUBAGENT_PROTOCOL_EXPLORER.md** — read-only research agents (Phase 1 or freestanding).

## Versioning

When the project's working rules (LEARNINGS) or operating protocols (CLAUDE.md) update, update WORKFLOW_PROTOCOL.md in the SAME step. Next workflow run picks up the changes. No stale-protocol drift across batches.
