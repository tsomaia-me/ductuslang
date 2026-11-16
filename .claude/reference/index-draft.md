# The index of the documents, a draft

A draft by designer 2, 2026-11-16, for `packages/symphonyscript/docs/INDEX.md`, which does not exist yet. Nothing here is written to that file before the owner's yes. Below it: one line proposed for `.claude/HANDOVER.md`, with the line it replaces. The owner's words that ask for the index are the seventeenth and eighteenth entries of `owner-rulings-compiler-design.md`.

## The page, word for word

# The documents

Start here. This page lists SymphonyScript's documents: what each is for, where it stands, and who reads it when. It repeats nothing a document says, and it holds no state of the work.

Read `SPEC.md` whole before anything else. Read any other document when your work is in its part, as its line says, and not before.

The peer keeps this page. A document gets its line here in the change that creates it, and its line changes when its standing does. A path without a folder is in this folder, `packages/symphonyscript/docs/`; any other path starts at the repository's root.

| Standing | Meaning |
|---|---|
| approved | The owner approved it. It binds. |
| approved for testing | The owner approved it to be tried, not as final. |
| draft | Written, and not approved. |
| not written | It is to be written. Its name and its place are set when it is. |
| proposed | Whether it becomes a document of its own is not ruled. |
| being replaced | The last version, kept until its successor is written. It is not the design. |
| old | Of the old build. It binds nobody. |

## The language

| Document | What it is for | Standing | Who reads it, when |
|---|---|---|---|
| `SPEC.md` | The language, the Program, and what a Program means when it runs. It names no mode. | approved | Everyone, whole, first. |
| The compiler's architecture | How source becomes a Program. | not written, proposed | Whoever works on the compiler. |

## The modes

Each mode has a specification and an architecture of its own, so that work on one mode needs nothing written about the other.

| Document | What it is for | Standing | Who reads it, when |
|---|---|---|---|
| The page common to every mode | What a mode is, and what holds for all of them. | not written, proposed | Whoever works on a mode. |
| The interpreted mode's specification | What a host can rely on in the interpreted mode beyond the SPEC. | not written | Whoever works on the interpreted mode. |
| The interpreted mode's architecture | How the interpreted mode is built. | not written | Whoever works on the interpreted mode. |
| The compiled mode's specification | What a host can rely on in the compiled mode beyond the SPEC. | not written | Whoever works on the compiled mode. |
| The compiled mode's architecture | How the compiled mode is built. | not written | Whoever works on the compiled mode. |

## The runtime

| Document | What it is for | Standing | Who reads it, when |
|---|---|---|---|
| `SPEC.md` §12 and §13 | What a runtime does. They are its specification, and it has no other. | approved | Whoever works on a runtime. |
| The runtime's architecture | How the first runtime is built. | not written, proposed | Whoever works on the runtime. |

## The whole

| Document | What it is for | Standing | Who reads it, when |
|---|---|---|---|
| The architecture's top document | The map of the parts, the contracts between them, and the rules that bind every part. | not written, proposed | Everyone who builds, before the document of their part. |
| `ARCHITECTURE.md` | The last design, of the old build. The SPEC still cites its sections. | being replaced | Nobody, as the design. |

## Writing code, testing it, reviewing it

| Document | What it is for | Standing | Who reads it, when |
|---|---|---|---|
| `STANDARD.md` | How code is written: architecture, cost, the way from a SPEC sentence to code, a builder's own tests. | draft | A builder, whole, before it writes. A reviewer, who cites its lines. |
| `FINDINGS.md` | What may be reported on a piece of work, and what is admitted. | draft | A reviewer, before it reads. A tester, before it attacks a build. |
| The tester's guidelines | How tests are written from the SPEC, and how a build is attacked. | not written | A tester, before it writes. |

## How the work runs

| Document | What it is for | Standing | Who reads it, when |
|---|---|---|---|
| `.claude/reference/workflow-as-shown-2026-11-15.md` | The work model: who does what for a piece of work, and the checks every piece passes. | approved for testing | The peer. |
| `.claude/agents/` and `CLAUDE.MD` | Agent files drafted for an earlier, larger model of the work. | draft | Nobody. Nothing runs on them before the owner approves them. |
| `PLAN.md` | The plan of the old build. | old | Nobody. |

This folder also holds two lists that SPEC §1.2 names, `pending-examples.txt` and `pending-productions.txt`, and `standard-check/`, which checks the standard's worked example and is run by hand after any edit of the standard.

## The line for the handover page

It replaces this line of `.claude/HANDOVER.md`:

> **Read before your first message to the owner.** The SPEC whole and the code standard whole, `packages/symphonyscript/docs/STANDARD.md`. The owner talks with the peer about the language, and a peer that knows it from the notes alone gets it wrong.

With this:

> **Read before your first message to the owner.** The index of the documents, `packages/symphonyscript/docs/INDEX.md`, and the SPEC whole. Every other document is read when the work in hand needs it, as the index says. The owner talks with the peer about the language, and a peer that knows it from the notes alone gets it wrong.
