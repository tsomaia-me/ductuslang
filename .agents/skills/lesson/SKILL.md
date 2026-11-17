---
name: lesson
description: Proposes, widens, promotes, or removes lessons in .agents/lessons/GENERAL.md and .agents/lessons/<ROLE>.md. Use whenever a mistake caused rework, the owner corrects you, a lesson mechanism becomes obsolete, or the owner states `lesson: ...`.
---

# Lesson Conventions

## Trigger
Propose a new or widened lesson when a mistake caused rework or required owner correction and a concrete check prevents the repeatable mechanism. One-off slips with no repeatable mechanism get no lesson.
- **Direct owner input (`lesson: ...`):** Rewrite the owner's raw phrasing into the required form below while keeping its exact point, then propose it through the approval gate.
- **Obsolete mechanism:** Propose removing an existing lesson when architectural or tooling changes eliminate the mechanism it guards against.

## Form
One present-tense sentence under 500 characters:
`- <ID>: Do X, because Y.`
- `X`: Concrete action or check to perform before acting.
- `Y`: Specific failure mechanism that bites when `X` is skipped (never a restatement of `X`).
- Zero incident stories, dates, ticket/PR numbers, person names, or quotation marks.
- Good: `- B3: Read the area's architecture doc before changing its code, because its invariants are not visible in the files you touch.`
- Bad: `- "After the TIC-037 PR conflicted, check Mergify's reply."` (incident story and ticket ID)
- Bad: `- "Be careful with merges, since conflicts arise in several ways."` (no concrete action or mechanism)

## Widen Over Append
Read `.agents/lessons/GENERAL.md` and the target `.agents/lessons/<ROLE>.md` (if present) before proposing:
1. **Widen in place:** If an existing lesson already covers the same mechanism or should have caught the mistake, widen that line under its existing `<ID>` instead of adding a duplicate.
2. **Promote shared checks:** If the mechanism bites any role performing the action, place or merge it into `.agents/lessons/GENERAL.md` and check `.agents/lessons/*.md` to strip role-specific duplicates.
3. **20-lesson ceiling & prefixed monotonic IDs:** Keep each file at or under 20 lessons by merging overlapping checks (if merging loses concrete signal, ask the owner which `<ID>` to drop). Keep `Last ID: <PREFIX><N>` at the top of each file (e.g., `G` for `GENERAL`, `P` for `PEER`, `B` for `BUILDER`—matching the file's existing prefix, or `<PREFIX>0` when creating a new file), bump it on every add, and never reuse retired IDs.

## Approval Gate (No Cold Review)
Never cold-review lesson edits. Resolve the concrete `<ID>` first, then show the proposed line in a quote block in chat alongside the target file and any existing line(s) it widens, replaces, or removes. Only after the owner says yes, follow the `delegation` skill to pass `scribe` the exact target file path(s), updated `Last ID:` value, and verbatim approved edit.
