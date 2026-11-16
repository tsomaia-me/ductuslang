## Peer notes

The format of this file, set on 2026-11-16 at the owner's word (the seventeenth entry of the rulings file): the seven headings below, in this order, and no others; under 12 KB; state only, which is what runs, what is owed, what is open and who. The design lives in `design-state.md` and then in the documents, the owner's words in the rulings file, and nothing is kept in both. A line goes when it stops being relevant, and history goes nowhere but git.

## Read first, in this order, before your first message

1. `.claude/HANDOVER.md`.
2. This file.
3. The SPEC whole, `packages/symphonyscript/docs/SPEC.md`. Its §16 is an unapproved draft that sits as an uncommitted change of that file in the shared checkout.
4. `.claude/reference/owner-rulings-compiler-design.md`: the owner's words, one for one, twenty-one entries.
5. `.claude/reference/design-state.md`: the design as it stands.

Nothing else is read before the first message. Every other file is read only when the item in hand needs it, and secondary reading goes to subagents. After a compaction: the same list, without the SPEC unless the item in hand is the language.

## Where the work stands

- No compiler code is written and none is being written. A design pass is under way; `design-state.md` says what is settled, what is proposed and what comes next.
- What the owner ruled is settled. Everything else is an unproven draft: the code standard and the findings file, the §16 draft, the loops and agent files, and `packages/symphonyscript/docs/ARCHITECTURE.md`, which is the last design, is being replaced and is never given to the owner as the design.
- Ductus is the owner's other work in this repository and not the peer's to touch.
- In hand with the owner: the layers and their contracts. Not answered by the owner: the list of the four contracts.
- The next piece owed: the index of the documents, agreed by the owner in the seventeenth and eighteenth entries. It stands beside the SPEC and takes the list of documents out of these notes.
- The handover is open. Designer 1 is the peer and the writer of these notes until it tells the owner that its successor is verified; the successor then rewrites this line and the first under Who and how.
- Held, at the owner's pace: the batch of reworded lines, `.claude/reference/standard-batch-1-draft.md`, not cold-read, not shown. Before it is shown it takes in what was ruled after it was drafted.

## Who and how

- One peer talks to the owner and is the one writer of these notes. "SymphonyScript designer 1" hands over on 2026-11-16 to "SymphonyScript designer 2", session `session_01Cbzj49BnKWbDEJwU7egSfJ`, on Fable 5.2 at xhigh. A retired peer's title starts with "[retired]". In the rulings file, "the peer" up to the nineteenth entry is designer 1, and the designer 2 of the twentieth entry was an earlier session of that title.
- What runs: nothing. No agent runs between turns; reading and checking go to short-lived subagents.
- Rules that bind every session and stand in no other file: nothing is recorded as the owner's decision unless the owner said it in the chat, and a ruling is saved word for word in the rulings file as it lands; who approves a change of the SPEC, the code standard and the architecture is the table in `design-state.md`; no agent file or loops file is written without the owner's approval, and no agent of the work model starts before it; how agents are organised is the peer's to design, and the owner approves the files once, as a result, before anything runs; the shared checkout at `/home/cdagent/ductuslang` is never stashed or switched, and its `SPEC.md` carries the §16 draft as an uncommitted change that is never staged.

## Owed, in order

The design conversation leads, in the order `design-state.md` gives under Next and at the owner's pace. This list is taken when that conversation pauses or an item blocks it. Items 1 and 2 are the peer's to do, cold-read, with no approval of the owner needed. Items 3 and 4 wait for the end of the design pass.

1. The owner's order of 2026-11-15, half done: "delete the build output and make sure there are mechanisms that keep it drained when obsolete". The output is deleted. The mechanism waits until the files that run builds are settled; the owner was told.
2. The files an agent loads at its start. `.claude/hooks/session.mjs` is to print `.claude/HANDOVER.md` first when the file exists, with tests, and to tell a peer to drain these notes past a set size; its menu line names the head and the builder of an older harness; the context hook's notice asks for the files to re-read "at the end" of the notes, while the format has them first. One test of `.claude/hooks/` fails and failed before this pass, `settings.test.mjs`; the other 49 pass. The root rules file is tracked as `CLAUDE.MD` and does not load on `wb`, where Claude Code looks for `CLAUDE.md`; the owner was told and has not said to rename it.
3. Before any execution starts, by the owner's word of 2026-11-15: the pushed agent files, the root `CLAUDE.MD` and `.claude/agents/`, are reviewed properly. A cold reader's digest of them: `.claude/reference/pipeline-drafts-digest-2026-11-15.md`.
4. The first piece of work, once the design pass is over (the seventh entry): it tests the work model in a real case, with its numbers and its quality measured; what is measured is named before it starts. The piece picked so far is the number reader of SPEC §2.6 in a throwaway crate; the owner is owed one line, that it is the number reader and not the expression reader of §5.1 announced earlier, which the standard's example already holds.

## Waiting for the owner's yes, none written, not to be pushed

- The lines in `.claude/reference/lines-waiting-for-yes.md`.
- A widening of lesson P-4 and a preference that the repo holds everything a session needs; their text is at the end of `.claude/reference/open-design-questions.md`.
- Whether the §16 draft leaves the shared checkout's working tree (advised yes; `.claude/reference/spec-16/spec-16-draft.patch` equals it byte for byte). Whether `minus.txt` goes, an untracked file of older work at the repository's root.
- Whether the handover page's reading line still asks for the code standard whole, which the owner's prompts to the designers narrowed to the skill and the SPEC.
- One finding on the standard: its line 3 binds every crate, while 1.3 and 4.8 are written for the compiler's stages, so the tool's file reading and the runtime's tests do not fit.

## Where to find things, only when an item needs them

- `.claude/reference/` is a scratchpad for this design pass, cleaned and removed when the pass ends (the twenty-first entry); what lasts moves into the documents before then.
- The code standard: `packages/symphonyscript/docs/STANDARD.md` and `FINDINGS.md`. Its thirteen principles in plain words, with the owner's words on the second: `.claude/reference/code-principles-page.md`.
- The work model's diagram: `.claude/reference/workflow-as-shown-2026-11-15.md`.
- Faults: `.claude/reference/spec-16/`. The parser's hand-on: `.claude/reference/parser-page/item-2/proposal.md`. Other open questions of the design: `.claude/reference/open-design-questions.md`.
- What `rustc` costs on generated code, a stand-in measurement: `.claude/reference/compile-mode/rustc-probe.md`.
- The old code, a source of cases and never a yardstick: branch `symphonyscript/r3`, on `wb` at `/home/cdagent/cargo-targets/r3/wt/`.

## The host

`wb` is this machine (its `hostname` prints `tornike`) and is slow; `wb2` (`ssh wb2`) is the reference machine. A build runs cargo by its full path, `nice -n 19`, 6 jobs. `/tmp` on `wb` is `noexec`. `grep` on `wb` is `ugrep`; `awk` is the way round its pattern errors. Worktrees of older work remain beside the shared checkout; a peer removes only a worktree it made.
