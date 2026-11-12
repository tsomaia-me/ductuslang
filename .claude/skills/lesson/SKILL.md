---
name: lesson
description: Project-specific conventions Peer follows when writing, widening, or removing a lesson in .memory/LESSONS.md or .memory/lessons/<ROLE>.md.
---

# Lesson Conventions

## When a lesson is written

Write or widen a lesson when a mistake cost rework or the owner had to correct you, and a check would have prevented it, so every future session reads the check before acting. One-off slips with no repeatable mechanism get no lesson.

When the owner gives a lesson directly (`lesson: ...`), rewrite it into the form below while keeping its point, record it, and show the written line so the owner can veto.

## Form

Every lesson is one present-tense sentence under 500 characters:

```
- <ID>: Do X, because Y.
```

- `X` is the concrete action or check to take.
- `Y` is the mechanism that bites when you skip `X` (never a restatement of `X`).
- Never include a date, a ticket ID, a PR number, a person's name, quotation marks, or an incident story. Use the incident only to test whether the general sentence would have prevented it.
- Good: `- B3: Read the area's architecture doc before changing its code, because its invariants are not visible in the files you touch.`
- Good: `- B4: Stage only the files you wrote before pushing the memory checkout, because a blanket add sweeps other sessions' half-written files into your commit.`
- Bad: `- "After the TIC-037 PR sat two days and conflicted, check Mergify's reply."` (incident and ticket ID)
- Bad: `- "Be careful with merges, since conflicts arise in several ways."` (no concrete action, no mechanism)

## Widen over append

Before adding a lesson, check `.claude/LESSONS.md` (shared across all roles) and `.memory/lessons/<ROLE>.md`:

- **Same mechanism and action:** If an existing lesson already covers the same mechanism or should have caught the incident, widen that line in place under its existing ID instead of adding a twin beside it.
- **Shared vs. role-specific:** If the mechanism bites any role that performs the action (or matches lines across more than one role file), merge it into the shared `.memory/LESSONS.md` file and remove the role-specific duplicates.
- **Cap (~20 lessons per file):** When a file crosses 20 lessons, widen overlapping lines into the general check they share and remove the redundant IDs until the file sits at or under 20. If merging would strip the concrete action or mechanism from a line, don't merge; ask the owner which lesson to drop. Never reuse a removed ID: keep a "Last ID:" line at the top of each file and bump it on every add.
- **Removal:** Remove a lesson when the mechanism it names no longer exists.

## How to write it in the repo

Edit the project's shared or role lessons file directly (`.memory/LESSONS.md` and `.memory/lessons/<ROLE>.md`), taking the ID after that file's "Last ID:" and disclosing every written, widened or removed line to the owner for veto.
