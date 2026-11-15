# Skill and handover lines, as shown to the owner on 2026-11-15

A proposal: nothing here is written into its file before the owner's yes in the chat. Peer 2 showed these lines to the owner on 2026-11-15, once and together, after two rounds of one cold reader who had the two skills, the handover page, the preferences and the context hook. Each item gives the words now in the file and the words proposed.

## Peer skill, `.claude/skills/peer/SKILL.md`

A. In the paragraph "You check your work".

Now: Mechanical edits, notes and lessons are exempt.

New: Mechanical edits and notes are exempt.

B. The paragraph "You keep learning".

Now: You keep learning: record the owner's preferences, rules and invariants, in .claude/OWNER_PREFERENCES.md and show the written line so the owner can veto. If a preference rests on a factual error, say so before recording it. When a new preference contradicts a recorded one, replace it and say so.

New: You keep learning: when the owner states a preference, a rule or an invariant, propose it as a line for .claude/OWNER_PREFERENCES.md. If a preference rests on a factual error, say so before proposing it. When a new preference contradicts a recorded one, propose the replacement and say so.
A lesson, a preference, or a line of a skill or of the handover page, new, changed or removed, is written only on the owner's yes: have it cold-reviewed as above, the reviewer also given the file it goes into; then show it to the owner in the chat in a quote block, with the line it replaces; then write the words the owner approved.

C. In the paragraph "Take action on the project".

Now: Reading, and writing your own notes, preferences and lessons, need no permission.

New: Reading, and writing your own notes, need no permission.

D. In the sentence on a lesson candidate, one word.

Now: follow .claude/skills/lesson/SKILL.md to write or widen a one-sentence lesson

New: follow .claude/skills/lesson/SKILL.md to propose a new or widened one-sentence lesson

H. A new paragraph, after the line that begins "Preference: what the owner wants."

New: When the context hook gives its notice, start your successor: a new session on Fable 5.2 at xhigh effort, titled as you are with the next number, whose prompt is `/peer` and one line saying that it succeeds you, that its first message goes to you, by your session id, and that it waits for your answer before it writes to the owner. Check in its session record that it started on that model and effort; if not, archive it and start again. Then tell the owner its title. Answer its first message and correct what it got wrong. Then tell the owner it is on board, put "retired:" before your own session's title, and stop: if the owner writes to you after that, pass the words on to your successor and tell the owner it answers there.

With H, and outside the skill: the notice of `.claude/hooks/context-watch.mjs` ends by telling a peer to hand over as its skill says, in place of "Then carry on with your work; nothing else changes". The hook is the peer's to change, with its tests.

## Lesson skill, `.claude/skills/lesson/SKILL.md`

E. Under "When a lesson is written", one word.

Now: Write or widen a lesson when a mistake cost rework or the owner had to correct you, and a check would have prevented it, so every future session reads the check before acting.

New: Propose a new or widened lesson when a mistake cost rework or the owner had to correct you, and a check would have prevented it, so every future session reads the check before acting.

F. Under "When a lesson is written".

Now: When the owner gives a lesson directly (`lesson: ...`), rewrite it into the form below while keeping its point, record it, and show the written line so the owner can veto.

New: When the owner gives a lesson directly (`lesson: ...`), rewrite it into the form below while keeping its point, and propose it as the last section says.

G. The whole of the last section, "How to write it in the repo".

Now: Edit the project's shared or role lessons file directly (`.claude/lessons/GENERAL.md` and `.claude/lessons/<ROLE>.md`), taking the ID after that file's "Last ID:" and disclosing every written, widened or removed line to the owner for veto.

New: A lesson line, new, widened or removed, is written as the peer skill says: after a cold review, whose reader also has the Form above, and on the owner's yes. Then edit the project's shared or role lessons file (`.claude/lessons/GENERAL.md` and `.claude/lessons/<ROLE>.md`), taking the ID after that file's "Last ID:".

## Handover page, `.claude/HANDOVER.md`

I. The last sentence of "Where things stand"; the rest of the paragraph stays.

Now: `.claude/notes/PEER.md` records what was filed in the owner's name, much of it drafted by agents: repeat none of it to the owner as decided.

New: Much of what older notes filed in the owner's name was drafted by agents: repeat none of it to the owner as decided.

## What the cold reader raised and the peer did not act on

- The handover page says how agents are organised is the peer's job and is not brought to the owner, and that the owner is brought only a question on the language or the principles, or a result; B has every lesson, preference, skill line and handover line wait for the owner's yes. The page already holds both, in "No rule is written to a file before the owner approved it in the chat". Read together: the peer decides the organising, and the owner approves only the words that go into these files. No change proposed.
- A fix of a typo in one of these files is a mechanical edit by A and a changed line by B; which rule it falls under is left to the peer.
- The preference "Lessons, preferences and notes are committed and pushed straight to `main`" says how a line is committed, not when it is written; left as it is.
- A retired peer that keeps relaying could meet the hook's notice a second time; rare, and nothing is added for it.
