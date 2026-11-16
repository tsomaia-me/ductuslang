# The index of the documents, a draft

A draft by designer 2, 2026-11-16, for `packages/symphonyscript/docs/INDEX.md`, which does not exist yet. Nothing here is written to that file before the owner's yes. Below it: one line proposed for `.claude/HANDOVER.md`, with the line it replaces. The owner's words that ask for the index are the seventeenth and eighteenth entries of `owner-rulings-compiler-design.md`.

## The page, word for word

# The documents

Start here. This page lists SymphonyScript's documents: what each is for, where it stands, and who reads it when. It repeats nothing a document says, and it holds no state of the work.

After this page, read `SPEC.md` whole. Read any other document when your work is in its part, as its line says, and not before.

The peer keeps this page. A document gets its line here in the change that creates it, and its line changes when its standing does. The peer's own files, its notes and the record of the owner's words among them, are not listed: the peer's skill leads to them. The design page of one piece of work is not listed either. A path without a folder is in this folder, `packages/symphonyscript/docs/`; any other path starts at the repository's root.

| Standing | Meaning |
|---|---|
| approved | The owner approved it. It binds. |
| approved for testing | The owner approved it for a trial, not as final. |
| draft | Written, and not approved. It can still change, and no work starts on it before the owner approves it. |
| not written | No file yet. |
| proposed | Whether it becomes a document of its own is not ruled. |
| being replaced | It holds the last version until its successor is written. That version is not the design. |
| old | Of the old build. Nobody works from it. |

## The language

| Document | What it is for | Standing | Who reads it, when |
|---|---|---|---|
| `SPEC.md` | The language, the Program, and what a Program means when it runs. It names no mode. | approved | Everyone, whole, right after this page. |
| The compiler's architecture | How source becomes a Program. | not written | Whoever works on the compiler. |

## The modes

Each mode has a specification and an architecture of its own.

| Document | What it is for | Standing | Who reads it, when |
|---|---|---|---|
| The page common to every mode | What a mode is, and what holds for all of them. | not written, proposed | Whoever works on a mode. |
| The interpreted mode's specification | What the interpreted mode adds to the SPEC that someone can observe or rely on. | not written | Whoever works on the interpreted mode. |
| The interpreted mode's architecture | How the interpreted mode is built. | not written | Whoever works on the interpreted mode. |
| The compiled mode's specification | What the compiled mode adds to the SPEC that someone can observe or rely on. | not written | Whoever works on the compiled mode. |
| The compiled mode's architecture | How the compiled mode is built. | not written | Whoever works on the compiled mode. |

## The runtime

| Document | What it is for | Standing | Who reads it, when |
|---|---|---|---|
| `SPEC.md` §12 and §13 | What a runtime does. They are its specification, and it has no other. | approved | Whoever works on a runtime. |
| The runtime's architecture | How the first runtime is built. | not written | Whoever works on the runtime. |

## The whole

| Document | What it is for | Standing | Who reads it, when |
|---|---|---|---|
| `ARCHITECTURE.md` | The architecture's top document: the map of the parts, the contracts between them, and the rules that bind every part. Until that is written it holds the last design, of the old build. | being replaced | Whoever the SPEC sends to one of its sections, for that section. Nobody takes the last design as the design. |

## Writing code, testing it, reviewing it

| Document | What it is for | Standing | Who reads it, when |
|---|---|---|---|
| `STANDARD.md` | How code is written: architecture, cost, the way from a SPEC sentence to code, a builder's own tests. | draft | A builder, whole, before it writes. A reviewer and a tester, before `FINDINGS.md`. |
| `FINDINGS.md` | What may be reported on a piece of work, and what is admitted. | draft | A reviewer, before it reads. A tester, before it attacks a build. |
| The tester's guidelines | How tests are written from the SPEC, and how a build is attacked. | not written | A tester, before it writes. |
| `.claude/docs/CODING.md` and `.claude/docs/REVIEWING.md` | The repository's guidelines from before the standard. For this package's code the standard took their place; `CODING.md` names three rules for the runtime that have no other home yet. | old | Whoever designs the runtime's architecture, for those three rules. |

## How the work runs

| Document | What it is for | Standing | Who reads it, when |
|---|---|---|---|
| `.claude/reference/workflow-as-shown-2026-11-15.md` | The work model: who does what for a piece of work, and the checks every piece passes. | approved for testing | The peer. |
| `.claude/agents/`, and `CLAUDE.MD` at the repository's root | Agent files and repository rules drafted for an earlier, larger model of the work. | draft | Nobody works from them: nothing runs on them before the owner approves them. |
| `PLAN.md` | The plan of the old build. Its first paragraph fences it, and keeps it as the place for a line of work owed. | old | Nobody, as a plan. |

This folder also holds two lists that SPEC §1.2 names, `pending-examples.txt` and `pending-productions.txt`, and `standard-check/`, which checks the standard's worked example and is run by hand after any edit of the standard.

## The line for the handover page

It replaces this line of `.claude/HANDOVER.md`:

> **Read before your first message to the owner.** The SPEC whole and the code standard whole, `packages/symphonyscript/docs/STANDARD.md`. The owner talks with the peer about the language, and a peer that knows it from the notes alone gets it wrong.

With this:

> **Read before your first message to the owner.** The index of the documents, `packages/symphonyscript/docs/INDEX.md`, and the SPEC whole. Every other document the index lists is read when the work in hand needs it, as its line there says. The owner talks with the peer about the language, and a peer that knows it from the notes alone gets it wrong.

## The cold read, 2026-11-16

One fresh reader on Opus, given the draft and its goal. Taken: the meanings of "not written", "draft" and "old"; the readers of `ARCHITECTURE.md`, `PLAN.md`, the agent files, the standard and the findings file; the modes' rows widened from "a host" to "someone"; a row for `.claude/docs/CODING.md` and `REVIEWING.md`; that the peer's own files and a piece's design page are not listed; the root's `CLAUDE.MD` named by its place; the handover line's "the index lists". Folded in beside it: the owner's ruling of the same day on the architecture's documents (the twentieth entry). Checked and kept: the standard and the findings file stand as drafts, by the corrected head of `owner-rulings-code-standard.md` and the fourth entry of the compiler-design rulings. Not taken: a caveat on the SPEC's §16, which sits only in the shared checkout's working tree and is no part of the committed file.
