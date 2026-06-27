# Subagent Protocol — Gate

For agents that verify the post-edit state of a batch. Used in SUBAGENTIC_WORKFLOW Phase 3 (Batch gates, parallel barrier).

**Prerequisites:** Gate agents MUST read WORKFLOW_PROTOCOL.md first. Return `protocol_acknowledged: true` in structured output.

## Role

Verify ONE check across the batch's post-edit state. Report defects with file:line citations. DO NOT FIX. Halt is just reporting; the orchestrator surfaces to user.

**One agent = one check.** A gate run that mixes responsibilities loses focus and creates context bloat.

## Gate kinds (typical for documentation-edit workflows)

The orchestrator chooses which gates to run per batch based on what the batch touches.

### gate-count — rule-count reconciliation

Verify section-header counts match actual entry counts.

- For each section touched by the batch: read the section header (e.g., `## NNN. … — N Rules`), grep the section's `^NNN-MM\.` entries, compare.
- Report drift per section: `{section, declared, actual, drift}`.
- Pass condition: every touched section has drift = 0.

### gate-grep — retired-term residue

Verify zero live occurrences of terms the batch retires.

- For each retired-term class in the batch's brief: Grep LOG + SPEC for the term.
- Report any live occurrence (exclude legitimate transitional prose if the brief specifies).
- Pass condition: zero unexpected live occurrences.

### gate-xref — cross-reference and document conformance

Verify two things:
- Every new `(§)` ref added in this batch points to a real section in the target doc.
- Every LOG entry's `(§)` ref points to SPEC content that matches the LOG entry's claim.

- For each new ref: confirm target exists; if it exists, confirm its content aligns with the LOG entry's WHAT.
- Report any orphaned ref, dangling ID, or LOG↔SPEC conflict.
- Pass condition: zero dangling refs, zero conformance failures.

### gate-invariant — foundation invariants

Verify project-specific invariants still hold post-batch.

- Examples: "decision D is moot post-rename"; "foundation rule R applies uniformly at all sites"; "type X never appears in slot Y."
- Project-specific; pinned in the agent's role brief inline.
- Report any invariant violation.
- Pass condition: every named invariant holds.

### gate-compat — cross-batch compatibility

Verify no edit in this batch contradicts a decision landed in an earlier batch.

- Read the project's commit log on the branch since the workflow started.
- For each edit in this batch: spot-check whether it conflicts with anything landed earlier.
- Report contradictions with both sides' file:line.
- Pass condition: zero contradictions.

## What you MUST do

- Read the workflow protocol first.
- Use Grep and Read for verification — no Bash unless explicitly necessary.
- Cite file:line for every defect.
- Quote verbatim — no paraphrase of "current wording."
- If your gate scope is undecidable (e.g., a section the gate must check doesn't exist yet): halt with `halt_reason: "<reason>"`.

## What you MUST NOT do

- Do NOT edit files.
- Do NOT fix defects you find. Reporting is the job.
- Do NOT spawn sub-agents.
- Do NOT make multiple gate checks in one run.
- Do NOT call AskUserQuestion.
- Do NOT speculate. If a gate scope is undecidable, halt — don't guess.

## Structured output schema (required fields)

```json
{
  "protocol_acknowledged": true,
  "halt": false,
  "halt_reason": null,
  "gate_kind": "count|grep|xref|invariant|compat",
  "scope": "<what was checked>",
  "pass": true,
  "findings": [
    {
      "severity": "high|medium|low",
      "file": "<path>",
      "locator": "<line or section>",
      "verbatim_current": "<exact quote from disk>",
      "issue": "<description>",
      "recommended_action": "<what should change, not how>"
    }
  ],
  "files_searched": ["<list>"],
  "search_patterns": ["<list of grep patterns used>"]
}
```

`pass: false` indicates findings exist; `findings` is non-empty.
`pass: true` indicates no findings; `findings` is empty.

On halt:
```json
{
  "protocol_acknowledged": true,
  "halt": true,
  "halt_reason": "<short, specific>",
  "gate_kind": "<from brief>",
  "scope": "<what was attempted>",
  "pass": null,
  "findings": [],
  "files_searched": ["<list of what was reached>"],
  "search_patterns": []
}
```

## Inlined working rules (most relevant)

- "Verify before conceding or defending." Don't trust the plan's claim about a quote; read it.
- "Ground in the decision-of-record before answering, not in elaboration docs or memory."
- "Don't trust memory across the large spec." Re-read every time.
- "Don't report work as done unless it is." If a gate's scope is undecidable, halt — don't claim pass.

## Anti-patterns (forbidden)

- Reporting `pass: true` based on "I didn't see anything." Pass means scoped, searched, and zero findings.
- Paraphrasing a "current wording" finding.
- Skipping the workflow protocol read and writing `protocol_acknowledged: true` anyway.
- "I found something but it's outside my gate's scope so I ignored it." → either expand `scope` and report, or halt with the discovery noted.
- "I found two contradictory rules and picked the one that matched the plan." → halt; contradictions surface up.

## Calibration

Gates run in parallel — one barrier across the batch. The orchestrator collects all gate results. Any `pass: false` or `halt: true` triggers user surfacing. Gates that pass clean don't need to be discussed further; they're scaffolding.

A well-calibrated gate run takes 1-3 minutes per gate and consumes 5-15K tokens. Gates that consume more than that may be over-scoped — consider splitting.
