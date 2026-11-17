---
name: lesson
description: Conventions for proposing, widening, or pruning lessons in .agents/lessons/GENERAL.md and .agents/lessons/<ROLE>.md.
---

# Lesson Conventions

## Trigger
Propose a lesson only when a mistake caused rework or required owner correction and a concrete check prevents the repeatable mechanism (or when the owner states `lesson: ...`). One-off slips with no general mechanism get no lesson.

## Form
One present-tense sentence under 500 characters:
`- <ID>: Do X, because Y.`
- `X`: Concrete check or action to take before acting.
- `Y`: Specific failure mechanism that bites if `X` is skipped (never a restatement of `X`).
- Zero incident stories, dates, ticket/PR numbers, names, or quotation marks.

## Widen Over Append
Read `.agents/lessons/GENERAL.md` and the target `.agents/lessons/<ROLE>.md` (if it exists) before proposing:
1. **Widen in place:** If an existing lesson covers the same mechanism or should have caught the mistake, widen that existing `<ID>` in place instead of adding a twin.
2. **Promote shared checks:** If the mechanism bites multiple roles (`builder`, `tester`, `reviewer`, `peer`), merge it into `GENERAL.md` and remove role-specific duplicates.
3. **20-lesson ceiling & monotonic IDs:** Keep each file at or under 20 lessons by merging overlapping checks (if merging loses concrete signal, ask the owner which ID to drop). Maintain `Last ID: <N>` at the top of each file (initializing at `Last ID: 0` if creating a new file) and never reuse retired IDs.

## Approval Gate (No Cold Review)
Never cold-review lesson lines. Present the proposed sentence (and any line it widens or replaces) in a quote block in chat. Only after the owner says yes, load `.agents/skills/delegation/SKILL.md` (if not already in view) and delegate the exact file update to `scribe`.
