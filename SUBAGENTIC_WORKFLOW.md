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

## The five-phase shape (per batch)

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
    │  PHASE 5 — Surface                                              │
    │  ─────────────────                                              │
    │  Workflow returns a structured summary:                         │
    │    { edits, gates, adversarial, totalDiff, haltReasons }       │
    │  Orchestrator shows it to user, awaits "approve", commits.     │
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

### Phase 5 — Surface

**Goal:** return a structured summary to the orchestrator for user-approval handoff.

Workflow returns:
```
{
  edits: [{site_id, status, diff}…],
  gates: {count, grep, xref, inv, compat},
  adversarial: {findings, verdict},
  totalDiff: <unified-diff string>,
  haltReasons: [strings]   // empty if clean
}
```

Orchestrator shows it to the user, awaits explicit "approve", then commits + pushes outside the workflow.

## Outer loop (between workflows)

```
foreach batch in plan.batches:
  1. result = Workflow({script: batch.script})
  2. surface result to user
  3. if result.haltReasons.length > 0:
       do NOT commit; await user direction
  4. else:
       await user "approve"
       on approve:
         git add <touched files>
         git commit -m "<batch message>"
         git push -u origin <branch>
  5. next batch
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

For a 30-site batch: ~500K-700K total. For a 7-site batch: ~150-250K. Plan to budget the largest batch + 50% headroom.

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

```javascript
export const meta = {
  name: 'batch-foo',
  description: 'Project Foo batch X',
  phases: [
    {title: 'Extract'}, {title: 'Edit'}, {title: 'Batch gates'},
    {title: 'Adversarial'}, {title: 'Surface'},
  ],
}

// PHASE 1
phase('Extract')
const sites = await agent(EXTRACT_PROMPT, {schema: SITE_LIST_SCHEMA})
if (sites.halt) return {halt: true, phase: 'Extract', reason: sites.halt_reason}

// PHASE 2
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

// PHASE 3
phase('Batch gates')
const gates = await parallel([
  () => agent(GATE_COUNT_PROMPT, {phase: 'Batch gates', schema: GATE_SCHEMA}),
  () => agent(GATE_GREP_PROMPT, {phase: 'Batch gates', schema: GATE_SCHEMA}),
  () => agent(GATE_XREF_PROMPT, {phase: 'Batch gates', schema: GATE_SCHEMA}),
  () => agent(GATE_INV_PROMPT, {phase: 'Batch gates', schema: GATE_SCHEMA}),
  () => agent(GATE_COMPAT_PROMPT, {phase: 'Batch gates', schema: GATE_SCHEMA}),
])
const gateHalts = gates.filter(g => g?.halt)
if (gateHalts.length > 0) return {halt: true, phase: 'Batch gates', reasons: gateHalts}

// PHASE 4
phase('Adversarial')
const adversarial = await agent(ADVERSARIAL_PROMPT,
                                 {schema: ADVERSARIAL_SCHEMA, effort: 'high'})

// PHASE 5
return {
  edits: editResults,
  gates,
  adversarial,
  totalDiff: editResults.map(e => e.diff).join('\n\n'),
  haltReasons: adversarial.halt ? [adversarial.halt_reason] : [],
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
