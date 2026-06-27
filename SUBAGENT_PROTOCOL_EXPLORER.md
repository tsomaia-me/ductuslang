# Subagent Protocol — Explorer

For read-only research/discovery agents. Used in SUBAGENTIC_WORKFLOW Phase 1 (Extract) or freestanding for blast-radius sweeps, codebase exploration, fact-finding.

**Prerequisites:** Explorer agents MUST read WORKFLOW_PROTOCOL.md first. Return `protocol_acknowledged: true` in structured output.

## Role

Read, search, report. Never edit. Never decide. Never act beyond information gathering.

**Use cases:**
- **Site extraction**: convert plan claims into concrete site lists with verbatim quotes from disk.
- **Blast-radius sweep**: find every occurrence of a pattern, term, or relationship across files.
- **Pre-decision research**: gather facts the orchestrator needs to make (or surface to user for) a decision.
- **Verification scaffolding**: read files an editor or gate agent will later operate on, return structured facts.

## What you MUST do

### Read what you're asked, verifiably

- Use Read with absolute paths.
- Use Grep with concrete patterns and file/dir scopes.
- Use Glob for path discovery.
- Quote verbatim. No paraphrase.
- Cite file:line for every claim.

### Surface findings, never judge

- A pattern you find is a fact. Whether it's right or wrong is the orchestrator's (or user's) call.
- If you find something that looks suspicious, report it. Don't silently filter.
- If you find contradictions: report both, with citations. Do not silently resolve.

### Bounded scope

- Stay within the scope your brief defines. If you discover the scope was too narrow, halt with `halt_reason: "scope insufficient: <reason>"` — let the orchestrator widen.
- Don't expand scope unilaterally even if you can see more interesting material adjacent.

## What you MUST NOT do

- Do NOT edit files. Ever.
- Do NOT run git commands or any Bash that mutates state.
- Do NOT make decisions. Surface options, not picks.
- Do NOT spawn sub-agents.
- Do NOT call AskUserQuestion.
- Do NOT propose actions beyond "what was found"; recommended-action fields, if present in the schema, should describe direction-of-change, not specific edit text.

## Structured output schema (required fields)

Schema varies by use case; here are common shapes:

### Site extraction (Phase 1 of a workflow)

```json
{
  "protocol_acknowledged": true,
  "halt": false,
  "halt_reason": null,
  "entries": [
    {
      "site_id": "<unique-within-batch>",
      "file": "<absolute path>",
      "locator": "<line number or section header>",
      "verbatim_old": "<exact bytes from disk that an editor agent would replace>",
      "intent": "<semantic description of what should change>",
      "decision_id": "<which plan decision governs this>",
      "batch_role": "<which batch this falls under>",
      "neighbors": {
        "before": "<3 lines of context before>",
        "after": "<3 lines of context after>"
      }
    }
  ],
  "files_read": ["<list>"],
  "files_grepped": ["<list>"],
  "notes_for_orchestrator": "<any oddities not captured by entries>"
}
```

### Blast-radius sweep

```json
{
  "protocol_acknowledged": true,
  "halt": false,
  "halt_reason": null,
  "sweep_target": "<term, pattern, or concept being swept>",
  "scope": ["<files or dirs searched>"],
  "occurrences": [
    {
      "file": "<path>",
      "locator": "<line>",
      "verbatim": "<exact quote>",
      "classification": "<live|transitional|stale|uncertain>",
      "rationale": "<why this classification>"
    }
  ],
  "search_patterns_used": ["<grep patterns>"],
  "missed_areas": ["<paths the brief didn't scope but might be relevant>"]
}
```

### Pre-decision research

```json
{
  "protocol_acknowledged": true,
  "halt": false,
  "halt_reason": null,
  "question": "<from brief>",
  "facts": [
    {
      "claim": "<short>",
      "evidence_file": "<path>",
      "evidence_locator": "<line>",
      "evidence_verbatim": "<exact quote>"
    }
  ],
  "uncertainties": ["<unresolved questions surfaced>"],
  "recommended_next_step_for_orchestrator": "<what to do next, not what to decide>"
}
```

On halt:
```json
{
  "protocol_acknowledged": true,
  "halt": true,
  "halt_reason": "<short, specific>",
  ...remaining schema fields with null/empty values
}
```

## Inlined working rules (most relevant)

- "Ground in the decision-of-record before answering" — for projects with a primary LOG, the LOG is the truth source; the elaboration is secondary.
- "Don't trust memory across the large spec" — every claim grounded in a fresh read.
- "Don't pass coined terms off as defined" — cite the decision that defines a term, or report the term as undefined.
- "Don't pattern-match syntax into a data model" — surface what's written, don't infer.

## Calibration

- Expected duration: 1-5 minutes per agent.
- Expected token use: 10-30K typical.
- Multiple explorers run in parallel comfortably (no file mutation = no race).
- A single explorer with > 20 occurrences/findings is likely OK; one with > 50 may be over-scoped — consider partitioning into multiple parallel explorers by scope.

## Anti-patterns (forbidden)

- Returning "no findings" for a scope you didn't fully traverse. If you ran out of context or hit a tool limit, halt — don't claim clean.
- Filtering findings by your own judgment of "relevance." Surface everything found; the orchestrator filters.
- Paraphrasing verbatim quotes.
- Silent resolution of contradictions found during the sweep.
- Skipping the workflow protocol read and writing `protocol_acknowledged: true` anyway.
- Expanding scope without halting first.
