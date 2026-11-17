---
name: delegation
description: Routes and invokes subagents for file writes, state updates, codebase reconnaissance, cross-family cold reviews, adversarial probing, and multi-slice orchestration (`head`). Use before calling `invoke_subagent` or `run_command`.
---

# Delegation & Subagent Routing

Never run arbitrary git, build, or shell exploration commands via `run_command`. Use `invoke_subagent` (Gemini workers) and headless `claude -p` via `run_command` (Claude workers) strictly according to the table below.

## 1. Capability Routing

| Need | Target Worker | Channel | Model & Effort |
| :--- | :--- | :--- | :--- |
| Fast symbol, signature, or path lookup | `explorer` | `invoke_subagent` (default) or `claude -p` | Gemini `flash` (or Claude `opus` `low`) |
| Deep codebase trace or primary external spec check | `researcher` | `invoke_subagent` or `claude -p` | Inherited Gemini / Claude `opus` `xhigh` |
| Update `.agents/notes/PEER.md`, lessons, or preferences | `scribe` | `claude -p` (one-shot) | `opus`, `low` |
| Write or amend `SPEC.md`, `ARCHITECTURE.md`, or drafts | `writer` | `claude -p` (one-shot) | `opus`, `medium` (`high` for dense cross-refs) |
| Cold-audit specs/designs authored with `peer` | `reviewer` (+ `prober` if high-risk) | `claude -p` (one-shot, cross-family) | `opus`, `high` (`prober`: `opus`/`fable` `xhigh`) |
| Cross-family audit of code/tests built by Claude `head` | `reviewer` (+ `prober` if high-risk) | `invoke_subagent` (Gemini cross-family) | Inherited Gemini |
| Multi-slice TDD implementation, bugfix, or refactor | `head` | `claude -p` (**persistent session**) | `opus`, `high` |

## 2. Exact CLI Patterns (`run_command`)

Always pass prompts via a single-quoted heredoc (`<< 'EOF' ... EOF`) so quotes/backticks never break bash and `stdin` closes immediately on background tasks.

### One-Shot Workers (`scribe`, `writer`, `reviewer`, `prober`, `researcher`, `explorer`)
```bash
claude -p --agent <role> --model <model> --effort <effort> --dangerously-skip-permissions --no-session-persistence << 'EOF'
<self-contained brief>
EOF

```

### Persistent Multi-Slice Orchestrator (`head`)

1. Ensure `.agents/runs/` is gitignored, and persist the session UUID into `.agents/runs/heads.log` on launch so compaction or rotation never loses the resume handle.
2. Pass `WaitMsBeforeAsync: 5000` on `run_command` (both on launch and on `--resume`) so long runs move cleanly to Antigravity's background task runner without freezing chat.
3. **Working-Tree Exclusivity:** While a `head` run is active in the background, do **not** run `writer` on tracked repository files—concurrent edits in a shared working tree will race with `head`'s `builder` and `tester` passes.

```bash
mkdir -p .agents/runs
SID=$(uuidgen | tr '[:upper:]' '[:lower:]')
printf "%s\t%s\t%s\n" "$(date -u +\%Y-\%m-\%dT\%H:\%M:\%SZ)" "<slice-slug>" "$SID" >> .agents/runs/heads.log
echo "HEAD_SESSION_ID=$SID"
claude -p --agent head --model opus --effort high --session-id "$SID" --dangerously-skip-permissions << 'EOF'
<slice goal, existing authoritative spec/standard paths, scope boundaries, and constraints>
EOF

```

To resume a halted `head` session after resolving an authority conflict (`--dangerously-skip-permissions` and `--effort high` must both be re-passed on resume; check `.agents/runs/heads.log` via `view_file` if `HEAD_SESSION_ID` left context):

```bash
claude -p --resume "<HEAD_SESSION_ID>" --effort high --dangerously-skip-permissions << 'EOF'
<ruling summary, updated spec sections on disk, and instruction to re-read amended sections and resume>
EOF

```

## 3. Non-Negotiable Brief & Audit Hygiene

* **Zero-Context & Existing-Paths Assumption:** Subagents do not see our chat or `.agents/notes/PEER.md`, and halt if handed missing reference paths. Pass only authority documents and paths confirmed to exist on disk.
* **Fresh Cold Instances:** Every non-trivial spec or code change requires a cold review (notes, preferences, and lessons are exempt—never cold-review meta-files). On `invoke_subagent`, always spawn a **fresh** `reviewer` or `prober` instance rather than messaging a warm `Idle` one from an earlier round. Pass **only** target artifact paths and authoritative spec/standard paths—never include your rationale, `head`'s implementation summary, or claims that tests pass.
* **Post-`head` Cross-Family Gate:** When `head` returns `COMPLETED` on a substantive code slice, run Gemini `reviewer` (and Gemini `prober` on concurrency, memory/lifetime, or parser passes) via `invoke_subagent` over the touched files before reporting final completion to the owner.
* **Filter Review Noise:** Fix factual, structural, and invariant defects (up to 2 repair/re-review rounds only when a fix alters a decision); disclose subjective nits briefly to the owner without churning files over them.
