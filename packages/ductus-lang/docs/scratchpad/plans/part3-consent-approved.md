# Part 3 — Approved Consent Batch (owner: "all approved", 2026-07-20)

*Execution authority for the mechanical batch. 35 items + 2 riders. Source of per-item evidence: part3-survey-report.md (§2 table, §4 one-liners, §7 residue, §8 finds). Item backgrounds: audit-plan-3-individual-decisions.md — WARNING: that doc carries systematic id drift; every brief re-anchors by CONTENT against the current corpus, never by the audit doc's ids/line numbers.*

## Items (all approved; defaults marked ⚑ approved as stated)

1. I-02 — §13.12.1 "the one exception" → enumerate both dynamic sources (WeakHandle + Portal), conforming to the two-source law (023-46 family).
2. I-03 — Elaborate the wake gate into the SPEC sections its five LOG entries cite, under the same term. RIDER: fix 017-272's mistargeted §13.3.8 citation (re-anchor by content).
3. I-09 — Three drifted "innermost" labels for `here::` (020-34 + two SPEC sites) → committed body-scope wording.
4. I-13 — 035-5 (+ SPEC mirror): drop the "loop" half of the cost-family citation; keep collection-cost.
5. I-15 — `[N..M]` → `[N..=M]` at 017-40, 017-109, SPEC mirror.
6. I-17 — Borrow-unstorable slot lists conformed both ways: LOG 001-25 gains "reactive cell"; SPEC counterpart gains "indexed slot" (full five-way list both docs).
7. I-21 — 018-143: repoint citation to §13.3.3.5; state both rejection directions; name `bundle_in_repeat_rejected`.
8. I-33 — Add `?.` `?[]` `?()` to SPEC's postfix precedence tier per 007-74.
9. I-39 — 032-137 (+ SPEC list item): name both loop-exit paths (break AND enclosing function return).
10. I-42 — Narrow flag-opener prose "any non-letter character" → the nine flag chars, at LOG + SPEC + GRAMMAR comment.
11. I-45 — Repoint 004-4's citation to the section owning `->changes` materialization (§13.18.9); verify the target's own §4.4 pointer while there.
12. I-52 — 012-105 "in left order" → "in the right operand's order" (conform to 012-120 + SPEC).
13. I-69 — 007-230 `Index[Range]` → `Index[Range[isize]]`.
14. I-72 — Unify "compile-time evaluable" → "compile-time-known": LOG 001-28, 004-100, 016-116, 016-150 + ~7 SPEC lines (fresh-grep for stragglers).
15. R-5 — Repoint the 15 stale GRAMMAR cites of 005-30 to each production's governing decision (per-production research by content).
16. I-14 — Assign the duplicate-scope-key warning (018-81) a named diagnostic class + delivery channel, parallel to sibling lints.
17. I-26 — Pin: derived Clone on Map preserves insertion order (012-114 area + SPEC §9.5.13).
18. I-37 — State in prose: same-line `if`/`match` after `yield` is always the yield's operand.
19. I-58 — Widen restrictive observed:-block sites (031-14/15 + SPEC role list + scoped "recurrent only in desired:" + hint) to admit derived/value-recurrent computed outputs; host-fed `recurrent[N] stream` stays forbidden. RIDER: conform the two contradictory diagnostic hints (§13.19.5 vs §13.19.16) to the widened law.
20. I-59 — New 030 rule: scope_drop releases a consumer's cursor + slowest-cursor hold on an OUTER gate stream (parallel to reload's 028-49); coordinate with effect-teardown 018-110 prose.
21. I-66 — Pin in 026-7/§13.13.2: a Result/Option-typed reactive body plays the function-return role for `?` early return.
22. I-73 — Make SPEC §10.4.1 self-contained (parentheses convention + general glob-collision rule stated in place).
23. I-76 — Qualify 016-15's "written only at placement time" per the landed reactive-bridge model (016-285).
24. DP-05 — `.exposition` restating entries (017-247 + SPEC mirror) enumerate all five entry kinds.
25. DP-06 — Restate 032-108 + SPEC §14.6.1: behavior = pure params→value function; the RUNTIME performs cell I/O.
26. R-3 — Mirror SPEC's Portal[WeakHandle[T]] two-step presence check into the LOG (new entry, 017 family).
27. R-4 — Conform LOG 033-207/208/209 AND SPEC §15.4.4 to normative IR_GRAMMAR (InstrSep, instr_rhs, bare-NAME/move-NAME operands); verify IR_GRAMMAR's "lifted verbatim" note becomes true post-conform (fix it if not).
28. R-2 — Respell bare-noun "kind" → channel vocabulary at 016-167, 029-124, 030-48, 030-49 (only where the applied annotation is meant; "kind" as the keyword-level term stays).
29. I-20 — Conform SPEC §4.9.5 to the `Handle[T][..N]` variant; respell 017-101's index-basis to the landed row-length basis.
30. I-35 — Home the interpolation-hole balanced-scan rule normatively (SPEC §9.1.9 + LOG 002 family entry); GRAMMAR keeps its comment as mirror.
31. I-70 — Pairs-form match fallback `default:` → `_:` in 019-46 + SPEC (entry anchors "as any match §6.2.5"). ⚑ approved.
32. N-3 — Single-home the duplicated stream-reload rules: §13.18.14 (streams chapter) owns; §13.15.5 reduces to summary + pointer. ⚑ approved default.
33. N-8 — Verify §2.5 rejected-ops row "Integer `/`, `%`" vs its truncation rationale (`\` is the truncating operator); correct label or rationale; NO-OP if verification clears it — report either way.
34. N-9 — Add a note at the precedence section: postfix `->` crossing is position-disambiguated, deliberately outside the tier table. ⚑ approved default (documents status quo).
35. N-11 — Scope "exactly three materialization boundaries": three = consumption-into-storage; group auto-materialization is a READ path per 034-14, not a fourth boundary. ⚑ approved default.

## Standing rules
LOG first, SPEC conforms; new entries take the next free number in their topic section (headers update their "— K Rules" counts); never renumber. Invariant-2: no entry-id citations inside LOG entry text or editor briefs (§-refs only). Same-file editors sequential. /usr/bin/grep, /usr/bin/sed for all evidence (rtk hook corrupts output). No commits. Gates + three blind reviewers + full-diff capture before the final report.
