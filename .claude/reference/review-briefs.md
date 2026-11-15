# The two fixed briefs of the design work

Used as they stand. Only the parts in angle brackets are filled in, and neither names who wrote the artifact or what anyone prefers.

## The gate review of a page

Launched by the owner or the supervisor, never by the page's author, once before the owner approves a page. Its report goes to the owner whole.

```
Cold review, read-only. Change no file and write no report file; return your
report as text in your final message. Read nothing under `.memory/`.

The artifact: <path of the page, and the commit>. It is a design page for
<what the unit builds>. Its goal: <the goal, in one or two sentences>.

Read first, whole: `packages/symphonyscript/docs/SPEC.md` (with
`.claude/reference/spec-16/spec-16-draft.patch` laid over it in a worktree
of your own, unstaged), `packages/symphonyscript/docs/STANDARD.md` and
`FINDINGS.md`, and `.claude/notes/PEER.md` for what is ruled.

Check:
a) Every claim the page makes about the SPEC, the standard or the code:
   quote the sentence or the line beside it, and say where the page says
   more than its source.
b) Every count and every cost figure: rebuild it yourself from the source
   it was taken over. Do not run the script that produced it.
c) Whether the page reopens anything ruled, or decides as a default
   something that has two valid answers.
d) What the page must hold by the notes' line "A design page carries", and
   what it leaves out.
e) For every construct the page handles: one program the page's design
   reads wrongly, if you can build one. Run what can be run.

Report: findings most severe first, each with its place, the quoted source,
why it is wrong or risky and how you know it (ran, read or reasoned); then
nits and preferences apart; then one paragraph on what you checked, what
you ran and what you did not.
```

## The blind second's prompt

Gemini alone answers as the blind second: the owner starts it as a fresh session from the same commit as the primary, on a branch cut for it and deleted after the comparison, and closes it after its answer, because its tool compacted after one question. It sees nothing of the primary's answer. Fable at Max is an extra run only on the owner's order, with the same prompt.

```
You are the owner's peer for SymphonyScript's compiler design. Read
`.claude/skills/peer/SKILL.md` and follow it as your role, then
`.claude/skills/lesson/SKILL.md`, `.claude/OWNER_PREFERENCES.md`, both
lessons files, the root `CLAUDE.MD` and `packages/symphonyscript/CLAUDE.md`,
then `.claude/notes/PEER.md` whole and the reading order at its top.

Work only in a worktree of your own on the branch <branch>
(`git worktree add <dir> origin/<branch>`). Never stash, switch or edit
another checkout. Lay `.claude/reference/spec-16/spec-16-draft.patch` over
the SPEC in your worktree, unstaged, before you read the SPEC whole. Read
nothing under `.memory/`, and of ARCHITECTURE.md only §3 to §6. Commit
nothing to `main` and push nothing to it; your notes go to <branch>.

The question: <the question, as the supervisor worded it and the owner
confirmed it, with no option named>.

Bring it to me with the options, their costs and wins, and your pick.
Rebuild every count you rely on from its source; do not run again the
script that produced it. Quote the SPEC sentence beside each claim you make
about the language. Decide nothing that has two valid answers: put it to me.
If your tool compacts your context before you answer, say so at the top of
your answer.
```
