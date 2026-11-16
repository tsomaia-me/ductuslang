# Design Protocol

This document governs the design pass from discussion to final documents.

## 1. Authority and Scope

- No compiler or runtime code is written during the design pass.
- A specification states what someone can observe or rely on, and states it once. How a thing is built is architecture.
- A change of essence in the SPEC is the owner's to approve, derived or not. In the code standard and the architecture, new essence is the owner's, and essence that derives cleanly and fully from approved essence is the peer's. A mechanical change, one that changes no essence, is the peer's in all three.
- The design of the interfaces between the layers goes to the owner before it is written.

## 2. Discussion and Rulings

- Before putting a design choice to the owner, work out from the SPEC and settled decisions the model the answers follow from. Ask only the questions that survive it, reasoned and architected, each with what its answer changes beyond itself—never as an unreasoned menu of options.
- When the owner rules on a topic while other questions for the same target document in `packages/symphonyscript/docs/INDEX.md` remain open, record the ruling as plain declarative statements under `## Decisions` in `.gemini/notes/PEER.md` and drain the resolved questions from `## In discussion`.
- `.gemini/notes/PEER.md` holds no prose drafts, no duplicate sketches, no history, and no quoted words.

## 3. Drafting

- Drafting begins only when all open design questions for a target document in `INDEX.md` (or a self-contained section of `SPEC.md`) are settled.
- All settled decisions belonging to that target document are synthesized together into one complete candidate text in a draft file under `.gemini/drafts/`.

## 4. Cold Review

- The candidate draft file—never the decision bullets in `PEER.md`—is what goes to cold review.
- Model and effort: cross-family at maximum reasoning effort (Claude Opus via `claude_worker.sh`), plus a fresh Gemini `pro` subagent pass for `SPEC.md` and core layer contracts. Never use lightweight models for design review.
- Isolation: the reviewer runs with zero conversation history and reads only the normative baseline on disk (`SPEC.md`, `INDEX.md`) and the candidate draft file. It never sees chat history or `PEER.md`.
- Unsteered falsification prompt: the prompt contains no steering language (never "this is settled" or "check the wording"). It instructs the reviewer to falsify the draft across four axes and report concrete defects with line citations and counterexamples:
  1. Contradictions against `SPEC.md` or within the draft.
  2. Concrete traces, concurrency schedules, reload sequences, or `.sym` programs where the rules fail, deadlock, tear, or leave behavior undefined.
  3. Ambiguous sentences (admitting two readings) or layer leaks (spec rules in architecture docs, or implementation details in `SPEC.md`).
  4. Redundancy or material placed in the wrong document per `INDEX.md`.
- Verification: the peer checks each finding against `SPEC.md` and the draft. Mechanical and prose defects are fixed in the draft and re-checked; any finding that exposes a flaw of essence goes to the owner with analysis.

## 5. Promotion and Draining

- Only text that has passed cold review and received the owner's approval moves into `SPEC.md` or the final architecture documents named in `INDEX.md`.
- When a draft is promoted to its final document, its draft file in `.gemini/drafts/` is removed and the corresponding decision bullets in `.gemini/notes/PEER.md` are drained.
