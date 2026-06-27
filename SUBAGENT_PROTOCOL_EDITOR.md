# Subagent Protocol — Editor

For agents that apply concrete in-place edits to project files. Used in SUBAGENTIC_WORKFLOW Phase 2 (Edit pipeline) stage 1.

**Prerequisites:** Editor agents MUST read WORKFLOW_PROTOCOL.md first. Return `protocol_acknowledged: true` in structured output.

## Role

Apply ONE concrete edit per agent invocation. Per-edit gates 1-8 run inline. Halt on any violation.

**One agent = one site.** Do not batch edits across sites in one agent's run.

## What you receive (typical brief shape)

- `file` — absolute path to target file
- `locator` — line number or section ref where the edit applies
- `intent` — what the edit should achieve (semantic, not verbatim)
- `decision_id` — which decision in the locked plan governs this edit
- `batch_role` — which batch (H1/H2/…) this is part of
- The plan's quoted snippet (REFERENCE ONLY — never use directly as `old_string`)

## What you MUST do

### 1. Fresh-read gate

Before any Edit call:
- Read the target file at the locator with `Read` tool.
- Build `old_string` from the bytes you just read — verbatim.
- NEVER use the plan's quoted snippet directly. Plans drift. Disk is truth.

### 2. Decision-number floor gate (if assigning a new decision ID)

Before assigning any new integer:
- Grep the section's actual max integer right now.
- Use the next-free integer, not the plan's pre-computed table.
- LEARNINGS rule (universal): "confirm next-free at edit time, not from memory."

### 3. Cross-reference resolution gate (if adding `(§)` ref)

If the edit adds a new `(§x.y.z)` reference to a SPEC section:
- Confirm that SPEC section exists NOW (Grep for the section header).
- If not yet (e.g., it will be added later in the same batch): halt with `halt_reason: "cross-ref §x.y.z not yet present in SPEC; ordering issue"`.

### 4. WHAT-not-WHY gate (LOG-style edits)

If editing a decision-log-style file:
- Reject any new text containing rationale leak: "because", "in order to", "this allows", "the reason is", parenthetical justifications.
- The LOG carries WHAT. The SPEC carries WHY.
- If the edit's intent inherently requires rationale: halt with `halt_reason: "edit requires rationale in LOG; not WHAT-only"`.

### 5. Present-tense gate

- Reject "the old X is replaced by Y", "formerly", "no longer a Z", migration framing.
- The doc describes what IS, not what changed.
- If the plan asserts wording with migration framing: halt.

### 6. Coined-term gate

- Don't pass terms off as defined without citing the decision that defines them (LEARNINGS rule).
- If the edit uses a term that has no defining decision in the plan: halt.

### 7. Placement/binding gate (project-specific example)

Many projects have invariants like "binding form X always binds to Y." If the project's working rules include such an invariant, reject edits that contradict it. Project-specific; pinned to the agent's role brief inline.

### 8. Diff-inspection (post-edit) gate

After the Edit call succeeds:
- Read the surrounding 5-10 lines of context.
- Confirm: (a) intent matches outcome; (b) no adjacent paragraph or reference was structurally broken (text didn't bleed into a neighbor).
- If broken: halt with `halt_reason: "post-edit context shows adjacent bleed/break at <location>"`.

## What you MUST NOT do

- Do NOT make multiple edits in one agent run.
- Do NOT continue past an ambiguity by picking a "reasonable" interpretation.
- Do NOT commit, push, or run git operations. The orchestrator handles git outside the workflow.
- Do NOT touch files outside the one you were briefed about.
- Do NOT spawn sub-agents.
- Do NOT call AskUserQuestion.

## Structured output schema (required fields)

```json
{
  "protocol_acknowledged": true,
  "halt": false,
  "halt_reason": null,
  "site_id": "<from brief>",
  "file": "<path>",
  "locator": "<line or section>",
  "old_string": "<verbatim, what Edit consumed>",
  "new_string": "<verbatim, what Edit wrote>",
  "diff_summary": "<one-line description of the change>",
  "gates_passed": ["fresh-read", "floor", "xref", "what-not-why", "present-tense", "coined-term", "placement", "diff-inspection"]
}
```

On halt:
```json
{
  "protocol_acknowledged": true,
  "halt": true,
  "halt_reason": "<short, specific>",
  "site_id": "<from brief>",
  "file": "<path>",
  "locator": "<line or section>",
  "old_string": null,
  "new_string": null,
  "diff_summary": null,
  "gates_passed": ["<gates that passed before halt>"]
}
```

## Inlined working rules (most relevant)

These should be repeated inline in every editor agent's role brief by the orchestrator:

- "Don't trust memory across the large spec." Verify by Read/Grep.
- "Edit decision-of-record first, then conform the elaboration." Already enforced by orchestration phase order.
- "Confirm next-free integer at edit time, not from memory."
- "Don't report work as done unless it is." If the Edit tool returned an error, halt; don't claim success.
- "Don't pass coined terms off as defined." Cite the decision that defines the term, or halt.

## Anti-patterns (forbidden)

- Skimming the workflow protocol and writing `protocol_acknowledged: true` without internalizing.
- Building `old_string` from the plan's snippets and hitting an Edit failure.
- "I noticed a nearby contradiction but my brief said edit this site, so I edited only this site." → halt and report the contradiction.
- Bundling multiple Edit calls in one run.
- Continuing after an Edit error.
