# Peer notes

Short operational peer notes stating where things stand, immediately relevant things that are settled but not yet implemented or went to the official plan, things being settled and things yet to discuss, kept up-to-date by the current peer and stripped off of obsolete notes on timely manner.

Final designs are the SPEC and the architecture files. What is ruled and not yet reviewed is in the draft files. These notes hold the decisions that stand nowhere else, what is in discussion and what is yet to discuss. Every line is a plain statement. No history, no quoted words.

## Read first

1. `.claude/HANDOVER.md`
2. This file.
3. `packages/symphonyscript/docs/INDEX.md`
4. `packages/symphonyscript/docs/SPEC.md`, whole.
5. `packages/symphonyscript/docs/drafts/MODES.md` and `packages/symphonyscript/docs/drafts/ARCHITECTURE.md`.

After a compaction: the same, without the SPEC unless the work in hand is the language.

## Decisions

- No compiler code is written during the design pass.
- Only settled text goes into the SPEC, after a cold review under heavy scrutiny.
- A specification states what someone can observe or rely on, and states it once. How a thing is built is architecture.
- A change of essence in the SPEC is the owner's to approve, derived or not. In the code standard and the architecture, new essence is the owner's, and essence that derives cleanly and fully from approved essence is the peer's. A mechanical change, one that changes no essence, is the peer's in all three.
- The design of the interfaces between the layers goes to the owner before it is written.
- A question that sets the design's course goes to the owner reasoned and architected, not as a choice of options.
- The work model is approved for testing: a peer, a tester, a builder and a fresh reader, one machine gate, five checks a piece. The first piece of work after the design pass tests it, with its numbers and its quality measured.
- No loops file, agent file or process is approved. None is written, and no agent of the work model starts, without the owner's approval.
- How agents are organised is the peer's to design. The owner approves the files once, as a result, before anything runs.
- `packages/symphonyscript/docs/ARCHITECTURE.md` is the last design. It is being replaced and is not the design.
- `.claude/reference/` is a scratchpad for the design pass. It is cleaned and removed when the pass ends, and what lasts moves into the documents first.
- Ductus is separate work in this repository. The peer does not touch it.

## In discussion

- **What runs a Program on a machine.** The model on the table: SymphonyScript is a synchronous language, of the family of Lustre and of a controller's scan cycle. A running Program is passive: it advances only when the host commits, and time is a cell the host writes (SPEC §12.2). A commit is a run of steps: the `state` writes together as one step, then each pending `stream` or `channel` write as its own step, in arrival order across all cells (§12.2). Memory is one figure, known when the Program compiles, and nothing allocates while it runs (§12.6). Evaluation never fails (§12.4). `past` plays the part of Lustre's `pre`, `on` of a clock, a commit of a scan. Everything asynchronous is kept at the boundary where inputs are taken in, and a cell's word is that boundary's waiting place: a `state` keeps the latest value, a `stream` is a bounded buffer that forgets the oldest write and counts it, a `channel` is a bounded buffer that refuses when full (§12.1). The language is written for producers that do not run in step with the commit, since a count of forgotten writes and a refusal mean something only then. Consequences on the table: the shipped part asks the machine for arithmetic and for memory once, loading taking one block of the size §12.6 gives (to check against §12.6); whether a product runs the interpreted mode is nothing the language grants or forbids. The hand-off is proposed as an interface, with a plain implementation for one context and a lock-free one; the meaning is the same in both, since §12.5 defines determinism over one sequence of writes and commits. Three hand-off questions are open:
  1. Whether a cell's pending buffer is the hand-off between two execution contexts, a thread or an interrupt. If it is not, a host's own queue stands in front of the runtime with a second overflow rule, the count of forgotten writes is no longer what was lost, and the promise of §6.6, that a `channel` forgot no arrival, stops at that queue.
  2. What arrival order across cells (§12.1) means when producers run at once. With one buffer a cell and two producers, a commit can see the later write and not yet the earlier one. The candidates: one producing context; producers that serialise among themselves, since only the commit must not wait; a narrower sentence in §12.1. The analysis of this race is unchecked.
  3. Whether a `state` is written across contexts, or only a `stream` and a `channel`. A `state` written across carries its written marks (§13.2) with it.
- **The contracts between the layers.** On the table, seven crossings: a framework to the compiler (the vocabulary, SPEC §10); the compiler to the interpreter and the generator (the Program, §11); the compiler to a tool (faults, positions, the memory and work figures); a mode and the runtime (the seam); a host to the runtime (load, handles, writes, commit, reload); the runtime to a consumer (values, changes, the reload report); what computes a rule to a framework's native functions while the Program runs (§10.1).
- **The seam between a mode and the runtime.** Open: one rule's value a crossing with the runtime calling, or generated code going through the cells in the Program's order and calling the shared code for every decision. A generator that carries a rule of the commit is ruled out. It is measured before it is fixed.
- **One runtime for both modes to start with.** A leaning, no decision.
- **WebAssembly.** A target of the compiled mode, or a third mode.
- **Load in the compiled mode.** SPEC §1.1, §12.3, §12.6 and §14 speak of a Program loaded as data and replaced by reload. Proposal: load exists in both modes and one host code runs both.
- **A page on what every mode shares,** beside the modes' specifications.
- **Where, in the SPEC, a gap stops being the peer's to close.**
- **Lines that wait for the owner's yes:** `.claude/reference/lines-waiting-for-yes.md`. A lesson on working out the model before a design choice goes to the owner; a widening of lesson P-4; a preference that the repo holds everything a session needs.

## Yet to discuss

- The outline of the architecture: the interfaces first; then the compiler's parts (packages and files, lexer, parser, names and scope, types and inference, values and their operations, compile-time evaluation, expansion, cells and their order, the vocabulary's checks, faults, the Program); the runtime, changes and consumers, reload; the interpreter, the generator, the tool.
- The rest of the compiled mode: how the same results are held, the speed figure, what a host gets beyond the shared interface. A thin slice through the whole compiler is built early to measure before budgets are set. It is not designed.
- Faults, SPEC §16: a draft and about ten open edges, in `.claude/reference/spec-16/`. The parser waits on them.
- What the parser hands on: seven questions, in `.claude/reference/parser-page/item-2/proposal.md`.
- Small language questions: `.claude/reference/open-design-questions.md`.
- The code standard, `STANDARD.md` with `FINDINGS.md`: a draft. A batch of reworded lines is held in `.claude/reference/standard-batch-1-draft.md`. Its line 3 binds every crate while its 1.3 and 4.8 fit only the compiler's stages. Three rules of the runtime have no home: a commit path that never allocates, lock-free hand-off between threads, integer time (`.claude/docs/CODING.md`).
- What the SPEC needs once the modes and the architecture are final: its first paragraph names `ARCHITECTURE.md`; seven sentences cite its sections, and some lean on them (§13.5, §15.2); §12.5 and the agreement of the modes; §13 and which mode offers reload.
- The first piece of work that tests the work model. Proposed: the number reader of SPEC §2.6 in a throwaway crate. What it measures is named before it starts.

## Owed by the peer

- A mechanism that keeps build output drained.
- The session hooks: `session.mjs` prints the handover page first and tells a peer to drain these notes past a set size; its menu line names roles of an older harness; `settings.test.mjs` fails.
- The agent files in `.claude/agents/` and the root `CLAUDE.MD` are reviewed before any execution. `CLAUDE.MD` does not load under that name, and renaming it is the owner's.
- After a change of the index, or when a document it names moves: `node --test packages/symphonyscript/docs/index-check.test.mjs`. Nothing runs it by itself.

## Where things are

- The peer is "SymphonyScript designer 2", session `session_01Cbzj49BnKWbDEJwU7egSfJ`. It is the one writer of these notes. Nothing runs between turns; reading and checking go to short-lived subagents started from no agent file.
- The record of the owner's words, kept word for word and added to as they land, is `.claude/reference/owner-rulings-compiler-design.md`, with two older files beside it, `owner-rulings-code-standard.md` and `owner-rulings-parser-page.md`. It is a record to check a statement against, not reading for a start.
- The work model's diagram: `.claude/reference/workflow-as-shown-2026-11-15.md`. The code standard's principles in plain words: `.claude/reference/code-principles-page.md`.
- What `rustc` costs on generated code, a stand-in measurement: `.claude/reference/compile-mode/rustc-probe.md`.
- The old code, a source of cases and no yardstick: branch `symphonyscript/r3`, on `wb` at `/home/cdagent/cargo-targets/r3/wt/`.

## The host

`wb` is this machine (its `hostname` prints `tornike`) and is slow; `wb2` (`ssh wb2`) is the reference machine. A build runs cargo by its full path, `nice -n 19`, 6 jobs. `/tmp` on `wb` is `noexec`. `grep` on `wb` is `ugrep`; `awk` is the way round its pattern errors. The shared checkout at `/home/cdagent/ductuslang` is used by other sessions and by the owner; it is not stashed or switched, and nothing unapproved is left lying in its working tree. Worktrees of older work remain beside it; a peer removes only a worktree it made.
