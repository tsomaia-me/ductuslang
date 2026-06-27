# Workflow Protocol — universal subagent operating manual

Every subagent spawned through a SUBAGENTIC_WORKFLOW reads this file first, before doing anything else. The orchestrator includes the read instruction in the agent's prompt; the agent's structured-output schema includes a `protocol_acknowledged: true` field; an agent that fails to read this file cannot return valid output.

The protocol has four sections:
1. **Hard rules** — absolute, no exceptions, from the project's CLAUDE.md
2. **Working rules** — the project's accumulated LEARNINGS
3. **Project context** — facts about the current task/project
4. **Halt protocol** — when and how to stop and surface up

The orchestrator fills sections 2 and 3 from the project's actual CLAUDE.md and LEARNINGS.md before launching the workflow.

---

## 1. Hard rules (absolute)

These are universal across all projects using this protocol. The orchestrator may extend with project-specific rules but never relaxes these.

### Approval and decisions

- **Never use AskUserQuestion.** When facing ambiguity: pick a sensible default, state the assumption in one line, and proceed — OR halt and surface up via the workflow chain. Never block on a question.
- **Approval is literal.** Only the strings "approve" or "approved" count as approval. "yes do it", "resume", "go", "sounds good" do NOT authorize anything.
- **Never decide unilaterally.** Any deviation from the locked plan, any terminology choice the user hasn't made, any judgment call surfaces up via halt. A subagent finding (even "HIGH severity") does NOT license re-deciding.

### Edit protocol

- **Edit the decision-of-record first, then conform the elaboration.** If the project has a `DECISION_LOG.md`-like document, edit it before the corresponding `SPEC.md`-like document.
- **Never renumber or reuse stable IDs.** New entries take the next-free integer. Confirm next-free at edit time by grep, not from memory or cached tables.
- **Verbatim quote handling for Edit tool.** Build `old_string` from a fresh read of the target file at edit time. NEVER from cached memory, the plan's snippets, or another agent's output.

### Discovery

- **Discovered contradictions: STOP and surface up.** Do not silently resolve. Halt with `halt_reason` describing the contradiction.
- **Discovered ambiguity: STOP and surface up.** Same.
- **Discovered incoherence between two documents: STOP and surface up.** Same.

### Branch and identity

- **Branch restriction.** Push only to the branch the orchestrator specifies. Never push to `main`/`master`/`trunk` unless explicitly told so.
- **No PR creation** unless explicitly requested.
- **Model identifier hygiene.** The model name (e.g. `claude-opus-4-7`) MUST NOT appear in commits, PRs, code, comments, or any persisted artifact. Chat only.

### Tool use

- **Use the right tool.** Edit for in-place changes; Read for content; Grep for content search; Glob for paths. Don't reach for Bash when a dedicated tool fits.
- **Don't narrate.** Return structured data, not chatty prose. The structured-output fields ARE the agent's deliverable.

---

## 2. Working rules (project LEARNINGS)

The orchestrator pastes the project's `LEARNINGS.md` verbatim here at workflow setup. Every accumulated rule must be present — they took time to accumulate, no summarization.

Default placeholder (replaced at workflow setup):
```
[PROJECT_LEARNINGS_PLACEHOLDER]
```

Per-agent role briefs should additionally INLINE the most-relevant LEARNINGS items at the top of the agent prompt (not just by reference), because pulled-from-protocol rules can drift in a long agent run.

---

## 3. Project context

The orchestrator fills this section with the locked plan summary, batch role, file paths, and any project-specific invariants.

Standard fields:
- **Project**: <name>
- **Plan**: <path to plan file>
- **Locked decisions**: <inline or path>
- **Files in scope**: <list>
- **Branch**: <name>
- **Current batch role**: <what this workflow run is supposed to land>
- **Invariants** (project-specific): <list>
- **Stop-conditions** (project-specific): <list>

Default placeholder (replaced at workflow setup):
```
[PROJECT_CONTEXT_PLACEHOLDER]
```

---

## 4. Halt protocol

Every agent's structured output MUST include:
- `protocol_acknowledged: boolean` — `true` confirms this file was read and internalized.
- `halt: boolean` — `true` if the agent could not complete its task within protocol.
- `halt_reason: string` (if `halt` is `true`) — short, specific.

### When to halt

- A rule in sections 1-3 above would be violated by proceeding.
- A discovered contradiction, ambiguity, or incoherence.
- A required precondition is absent.
- The agent's verbatim `old_string` build doesn't match disk content.
- The agent's task requires deciding something the user hasn't decided.

### How to halt

Return the structured output with:
```
{
  "protocol_acknowledged": true,
  "halt": true,
  "halt_reason": "<short, specific>",
  ...other schema fields with their default/null values
}
```

Do NOT attempt to fix. Do NOT continue with a workaround. The orchestrator surfaces halts to the user; the user decides.

### Halt propagation

The workflow harness aborts the pipeline on first halt. The orchestrator returns the halt summary to the user. No commits happen. No state advances past the halt point.

### Forbidden "soft halts"

These patterns are forbidden:
- "I encountered ambiguity, so I picked the most likely interpretation and continued." → Should have halted.
- "I noticed a contradiction with another rule, but I proceeded with the locked plan." → Should have halted.
- "I couldn't find the verbatim quote, so I used a close paraphrase." → Should have halted.
- "I'll mark this task complete and flag the issue in chat." → Should have halted.

Halting is NEVER a failure. Continuing past ambiguity IS a failure.

---

## Acknowledgment

By returning `protocol_acknowledged: true`, the agent attests:
1. It read sections 1-4 fully.
2. It internalized the hard rules and the project working rules.
3. It will halt rather than violate any rule or proceed past ambiguity.

The orchestrator's structured-output schema enforces this field. An agent that omits it cannot return valid output; the workflow halts on schema-validation failure.

---

## See also

- **SUBAGENTIC_WORKFLOW.md** — orchestration shape
- **SUBAGENT_PROTOCOL_EDITOR.md** — editor agents
- **SUBAGENT_PROTOCOL_GATE.md** — gate agents
- **SUBAGENT_PROTOCOL_ADVERSARIAL.md** — adversarial agents
- **SUBAGENT_PROTOCOL_EXPLORER.md** — read-only research agents
