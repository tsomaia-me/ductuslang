# Supervisor notes

Kept by the supervising Claude session, beside `.claude/notes/PEER.md`, which the primary peer alone writes. This file holds only what the supervisor needs and the peer notes do not carry.

## Now

- The supervisor of 2026-11-15 (transcript `da25adeb`, named "SymphonyScript supervisor") is at its cap and stops. A fresh supervisor takes over from `PEER.md`, line "How the design work runs", and from this file. The other supervisor of that day (transcript `f24028f1`, "SymphonyScript designer 2 ⑂") has stopped.
- The job: the owner pastes a peer's message whole; the supervisor answers with its reasoning, a short list of key points (every numbered or lettered item said in one sentence), and a draft reply with no blank and no bracket in it (P-10). It checks each claim that carries weight at its source (P-11 to P-15) and says where a claim differs from what the check returned.
- State of the repo when these notes were written: `main` and origin agree at `36edfe56`; the shared checkout is on `main` with the §16 draft laid over `SPEC.md`, unstaged; the branch `sub` is deleted; no stash is left.
- The colon question is ruled (`PEER.md`, Settled; the owner's words in `.claude/reference/owner-rulings-parser-page.md`). The reply for Gemini was given to the owner as one text: designer 2's draft with three lines added (the corrected count confirmed by two independent recounts, 743, 134 and 0; "Your veto list: both items stand"; item 5 naming the four callers of `.claude/reference/syntax-recovery-review.md`, finding F1). The owner sent it. The Gemini session read two files and its tool, Antigravity, began compacting.

## Open, put to the owner, not ruled

- Whether the roles swap. The supervisor's recommendation: a Claude session on wb is the primary and the one writer of the notes, and Gemini answers as the blind second in a fresh session each time, because the writer needs a long memory and the blind second needs none. The owner's answer so far: about 250,000 tokens is too low for design work, and he asks first for a workaround that keeps a Gemini session longer. That answer is the next step owed.
- Facts behind it, from forum posts and not from an official page, so unverified: Antigravity compacts on its own at about 250,000 to 256,000 tokens whatever the model's window, and the app and the command line have no setting for it; its Python SDK is said to expose `compaction_threshold`. Sources: `https://discuss.ai.google.dev/t/1m-advertised-context-vs-a-256k-compaction-clamp-in-the-shipped-binaries-plus-quota-and-agy-p-policy-questions-desktop-2-29-0-cli-1-5-1/198430`, `https://discuss.ai.google.dev/t/req-remove-antigravity-automatic-compact-let-us-do-it-instead/187572`.
- The score line in `PEER.md` says "Fable at Max, the control". The control's transcript (`688d522d`) records the model `claude-fable-5-1` and the effort `xhigh` on all 54 of its replies. Owed: the primary corrects the line. No result from Max exists.
- The fresh supervisor's effort level: xhigh recommended; what the owner launched is not known here.
- Texts given to the owner and held only in transcript `da25adeb`: the prompt for a Claude primary if the roles swap; the one changed line of the supervisor's prompt ("The primary peer alone writes the notes on main; the notes say who it is."); a short kickoff for a fresh Gemini session if they do not swap.

## Where things are

- A session's model, effort and context size are in the usage fields of its transcript under `/home/cdagent/.claude/projects/-home-cdagent-ductuslang*/`. Read them before comparing two answers.
- The fixed brief for the gate review and the prompt for a blind second: `.claude/reference/review-briefs.md`.

## Read again after a compaction, in this order

1. The peer skill's first step: `.claude/OWNER_PREFERENCES.md`, `.claude/lessons/GENERAL.md`, `.claude/lessons/PEER.md`.
2. `.claude/notes/PEER.md` whole.
3. This file.
4. `packages/symphonyscript/docs/SPEC.md` whole, with the §16 draft laid over it, and `packages/symphonyscript/docs/STANDARD.md`.
5. `.claude/reference/owner-rulings-parser-page.md` and `.claude/reference/review-briefs.md`.
