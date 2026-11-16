## Peer notes

## Read first, in this order, before your first message

1. `.claude/HANDOVER.md`.
2. This file.
3. The SPEC whole, `packages/symphonyscript/docs/SPEC.md`. Its §16 is an unapproved draft that sits as an uncommitted change of that file in the shared checkout.
4. `.claude/reference/owner-rulings-compiler-design.md`: the owner's rulings, word for word, fourteen entries.
5. `.claude/reference/design-state.md`: the design as it stands, what is settled, what is proposed, where the conversation stopped and what comes next.

Nothing else is read before the first message. Every other file is read only when the item in hand needs it, by the list at the end, and secondary reading goes to subagents, never at the cost of what the design needs in view (the owner's prompt to the third peer, 2026-11-15). The handover page asks for the code standard whole as well; the owner's later prompt narrowed that, and whether the page's words change is the owner's. After a compaction: the same list, without the SPEC unless the item in hand is the language.

## Where the work stands

- **Decided by the owner.** Generated Rust is the main mode and the interpreted mode a convenience; the compile mode is not designed yet. The loops and agent files were never approved. Ductus is a separate specification, not implemented.
- No compiler code has been written and none is being written. A design pass is under way; `design-state.md` says what it is and where it stands. What the owner ruled is settled; everything else is an unproven draft: the code standard and the findings file, the §16 draft, the loops and agent files, the old architecture document.
- `packages/symphonyscript/docs/ARCHITECTURE.md` is the last design and is being replaced. It is not the current design, and nothing in it is given to the owner as the design.
- In hand with the owner: the layers and their contracts. The owner has ruled the modes at a high level and the cut of the documents. Not answered by the owner: the peer's four contracts between the layers and where their exact shape is defined. The owner leans to one runtime for a start, no ruling.
- Held, at the owner's pace: the batch of reworded lines for the work model, `.claude/reference/standard-batch-1-draft.md`, drafted, not cold-read, not shown. Before it is shown it takes in what was ruled after it was drafted: the documents are the specifications, the code standard and the architecture, not three documents; and a candidate principle from the owner's reason for the three rules, "a contract is defined against an interface, and the implementation is passed in". Its head says which lines need the owner's yes.

## Who and how

- One peer talks to the owner and is the one writer of these notes. Until its handover on 2026-11-16 that was the third peer, session `session_01Cbzj489rxCtUNc3BffGco8`, titled "SymphonyScript design writer". Its successor is started on Fable 5.2 at xhigh as the peer skill says. Earlier peers' sessions are gone.
- What runs: nothing. No agent runs between turns; reading and checking go to short-lived subagents, each ended when its piece is done.
- How the owner wants to be answered, from the owner's words in this pass: a question is answered before any work that has not started; plain words, short, the answer first; text for a file in a quote block beside the line it replaces; a ruling saved word for word in the rulings file as it lands; what the peer decided alone listed at the end for a veto. The owner asked two peers where the design stands and was answered from the old architecture document, without the pass; `design-state.md` is the answer to that question.
- Rules that bind every session: nothing is recorded as the owner's decision unless the owner said it in the chat; who approves a change of the SPEC, the code standard and the architecture is the table in `design-state.md`, and every change is read cold first; a lesson, a preference, a skill line or a handover line is shown in a quote block and written on the owner's yes; no agent file or loops file is written without the owner's approval, and no agent starts before it; commits to `main` are staged by path and pushed unforced; the shared checkout at `/home/cdagent/ductuslang` is never stashed or switched, and its `SPEC.md` carries the §16 draft as an uncommitted change that is never staged; a conflict between two notes or documents is settled by checking the fact, and by the owner where no check settles it.

## Owed, in order

1. The owner's order to peer 1 of 2026-11-15, half done: "delete the build output and make sure there are mechanisms that keep it drained when obsolete". The output is deleted. No mechanism is written; the third peer told the owner it waits until the files that run builds are settled, since nothing builds before then.
2. The files an agent loads at its start. `.claude/hooks/session.mjs` is to print `.claude/HANDOVER.md` first when the file exists, with tests, and to tell a peer to drain these notes past a set size; its menu line still names the head and the builder of an older harness. One test of `.claude/hooks/` fails and failed before this pass, `settings.test.mjs`, which expects a guard of that harness; the other 49 pass. The root rules file is tracked as `CLAUDE.MD` and does not load on `wb`, where Claude Code looks for `CLAUDE.md`; the owner was told and has not said to rename it.
3. Before any execution starts, by the owner's word of 2026-11-15: the peer of that time has the pushed agent files, the root `CLAUDE.MD` and `.claude/agents/`, reviewed properly. A cold reader's digest of them is a start: `.claude/reference/pipeline-drafts-digest-2026-11-15.md`.
4. The first piece of work, once the design pass is over (the seventh ruling): it tests the work model in a real case, with its numbers and its quality measured; what is measured is named before it starts. The piece picked so far is the number reader of SPEC §2.6 in a throwaway crate; the owner is owed one line, that it is the number reader and not the expression reader of §5.1 that peer 1 announced, which the standard's example already holds.

## Waiting for the owner's yes, none written, not to be pushed

- A preference, shown twice: "A question from the owner is answered before any work that has not started, except the reading or fetching the answer needs; what already runs keeps running." Its cold reader's one gap: an answer that needs something run is neither reading nor fetching.
- A widening of lesson P-4, and a preference that the repo holds everything a session needs; their text is at the end of `.claude/reference/open-design-questions.md`.
- Whether the §16 draft leaves the shared checkout's working tree (advised yes; `.claude/reference/spec-16/spec-16-draft.patch` equals it byte for byte). `minus.txt` at the repository's root.
- One finding on the standard that the owner was reviewing: its line 3 binds every crate, while 1.3 and 4.8 are written for the compiler's stages, so the tool's file reading and the runtime's tests do not fit. The other finding of that pair, a test reviewer missing from the findings file, is answered by the work model: the review before code reads the tests.

## Where to find things, only when an item needs them

- The code standard: `packages/symphonyscript/docs/STANDARD.md` and `FINDINGS.md`. Its thirteen principles in plain words, with the owner's words on the second and the rewording: `.claude/reference/code-principles-page.md`.
- The work model's diagram: `.claude/reference/workflow-as-shown-2026-11-15.md`.
- Faults: `.claude/reference/spec-16/`. The parser's hand-on: `.claude/reference/parser-page/item-2/proposal.md`. Other open questions of the design: `.claude/reference/open-design-questions.md`.
- What `rustc` costs on generated code, a stand-in measurement: `.claude/reference/compile-mode/rustc-probe.md`.
- The peer's earlier loops draft, not approved: `.claude/reference/loops-draft/first/`.
- The old code, a source of cases and never a yardstick: branch `symphonyscript/r3`, on `wb` at `/home/cdagent/cargo-targets/r3/wt/`; its reviews are under `.claude/reference/`.
- The owner's earlier words: the other `owner-*.md` files under `.claude/reference/`. Much of what older notes filed in the owner's name was drafted by agents and is repeated to the owner as decided nowhere; that older record is `git show e2557cc3:.claude/notes/PEER.md`.
- Transcripts, on `wb` under `/home/cdagent/.claude/projects/-home-cdagent-ductuslang/`: the third peer's, `494ae3bd-cbaa-5c23-8e7b-04e7fe71874c.jsonl`; peer 1's, `021dac70-57a2-5e77-a945-779fcf2cd322.jsonl`, with the owner's own words of 2026-11-15, 13:11 to 14:02 UTC, on what the owner wants.

## The host

`wb` is this machine (its `hostname` prints `tornike`) and is slow; `wb2` (`ssh wb2`) is the reference machine. A build runs cargo by its full path, `nice -n 19`, 6 jobs. `/tmp` on `wb` is `noexec`. `grep` on `wb` is `ugrep`, where a pattern such as `'^\+\+\+'` is a syntax error and `awk` is the way round. Sixteen worktrees of older work remain beside the shared checkout; a peer removes only a worktree it made.
