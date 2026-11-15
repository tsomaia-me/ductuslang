# Peer notes

## Settled
- `packages/symphonyscript/docs/STANDARD.md` and `packages/symphonyscript/docs/FINDINGS.md` are settled (reopenable on concrete flaw + recommendation).
- Two binding loop orders (2026-11-15):
  1. Frozen stated tests reach the builder's branch only after its `structure proposal` and `breakdown` are accepted.
  2. `test-author` and the builder's `breakdown` (+ `structure proposal`) work in parallel from the approved `page`.
- Legacy `packages/symphonyscript/CLAUDE.md` and `.claude/docs/CODING.md` (plus `.claude/docs/REVIEWING.md` and 0-byte `.claude/docs/running.md`) are obsolete ("garbage"); root `/CLAUDE.md` and `.claude/agents/*.md` replace them alongside `STANDARD.md` and `FINDINGS.md`.
- `#![forbid(unsafe_code)]` across crates by default; `unsafe` requires explicit owner authorization in an isolated module.
- Three `OWNER_PREFERENCES.md` updates approved and written (approved lessons/preferences commit gate, self-contained open items, quote-block draft-first approval with verbatim replaced/removed lines).

## Drafted on Disk (Awaiting Cold Review & Owner Sign-Off)
- Root `/CLAUDE.md` (note: git index currently tracks uppercase `CLAUDE.MD`; on case-sensitive Linux `/home/cdagent/ductuslang`, needs `git mv` to lowercase `CLAUDE.md` when committing).
- `.claude/agents/builder.md`
- `.claude/agents/test-author.md`
- `.claude/agents/test-reviewer.md`
- `.claude/agents/reviewer.md`
- `.claude/agents/prober.md`
- `.claude/agents/verifier.md`
- `.claude/agents/explorer.md`

## Open for Owner
- Exact quote-block diffs for `.claude/skills/peer/SKILL.md` and `.claude/skills/lesson/SKILL.md` (to align both skills with the newly approved draft-first quote-block rule; held from disk until owner approves exact lines).
- Cleanup confirmation once root `/CLAUDE.md` is approved: removing legacy `packages/symphonyscript/CLAUDE.md` (so Claude Code does not load two `CLAUDE.md` files), renaming tracked `CLAUDE.MD` -> `CLAUDE.md` in git index for Linux case-sensitivity, and updating `.claude/hooks/session.mjs` `MENU_LINE` if `head` is retired.
- Owner is reviewing the 2 remaining `STANDARD.md` / `FINDINGS.md` observations (`STANDARD.md` line 3 "every crate" vs. `tool`/`core` I/O & non-compiler stated-test surfaces; adding explicit `Test Reviewer` row to `FINDINGS.md` to match `.claude/agents/test-reviewer.md`).
