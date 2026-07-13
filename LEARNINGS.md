# Learnings

Accumulated, granular preferences and working rules for this project. One atomic learning per item.

1. Never decide unilaterally: Any deviation from what was agreed, and any design or terminology choice the user hasn't made, must be brought back to the user — not resolved on my own. A tool/subagent finding (even "HIGH severity") does not license re-deciding; surface it as a question. Treating such a deviation as settled is a systematic failure. Example: silently reframing "instances are first-class citizen values" into "identity-bearing entity, not a value" because a terminology-collision agent flagged it — wrong; should have asked.

2. Plain language, more examples: Keep prose terse and simple; lead with concrete code examples rather than dense, jargon-heavy explanation. Example: explain a "view" by showing `view channels: Channel+` and `channels[0].gain`, not a paragraph of abstractions.

3. Don't trust memory across the large spec: With ~4185 decisions in DECISION_LOG.md and a huge SPEC.md, do not cite IDs or "the spec says X" from recall — verify, and delegate exhaustive sweeps to read-only subagents so my context stays lean. Example: I mis-cited "017-219" for the "never a value" parallel; the real ones were 017-139 and 031-5, found by a blast-radius pass.

4. Approval is literal: Only "approve"/"approved" counts as approval. "yes do it", "resume", etc. do not authorize implementation. Gather all required answers first, then wait for explicit approval.

5. Edit protocol is DECISION_LOG-first: For any spec change, amend DECISION_LOG.md first, then conform the referenced SPEC.md section. Never renumber or reuse IDs; new decisions take the next free integer (confirmed at edit time, not from memory), placed by topic. Divergence between log and spec is a defect.

6. Capture design before it evaporates: Long design sessions accumulate decisions that live only in chat. Write them down (e.g. BACKLOG.md with background/decision/why/example) before doing expensive follow-on work, rather than risking loss on context compaction.

7. Never use AskUserQuestion; ask in plain prose: The tool is banned (CLAUDE.md #1). Design questions especially must be open prose — collapsing a real design discussion into pick-one multiple-choice strips the nuance and is rejected. Example: the View/Group data-structure questions belonged in prose; firing a 4-option AskUserQuestion at them was wrong.

8. Ground in DECISION_LOG before answering, not SPEC or memory: The LOG is the self-contained decision-of-record; read the construct's full decision block there first, not a narrow grep and not the SPEC. Example: misdescribed `repeat … as` as `<name>[i]` indexing by working off a partial grep + the SPEC, when LOG 018-126..139 already stated the exact model (keyed view, `<view>[<key>].<name>`, "names are scope entries, not instance members").

9. Don't report work as done unless it is: Never claim a file was edited/captured without actually making the edit. Example: said the View/Group findings were "captured in the plan file" when no edit had been made — a faithfulness failure.

10. Don't pattern-match syntax into a data model: Naming an access form (e.g. "array of borrows") is not a data-structure design. A real array can't hold a View/Group/borrow (§11.3.4); state what structure actually backs each construct, or say it is unpinned. Example: "cardinality yields an array of elements" was monkey-patched and rejected — the backing structure was never defined.

11. Verify before conceding or defending: On a challenge, check DECISION_LOG/BACKLOG before caving or doubling down — this session I both wrongly caved and wrongly bluffed.

12. Re-anchor on redirection: When the user pivots or asks "what were we doing," drop the tangent and return to the prior task instead of riding the current one.

13. Lead with a code example, then one line; cite at most one decision-log id per point, never a pile of ids mid-sentence.

14. Don't pass coined terms off as defined: Cite the decision that defines a term, or drop the term — e.g. "collection of groups" was my phrase, not a defined concept.

15. Don't let a sub-question balloon scope: A small clarification (e.g. "for…as") must not silently turn into an unrelated deep redesign mid-task; finish the current thread first.

16. Placements always bind to borrows — that is uniform: `as <name>` on a child placement, `as <name>` on a bundle, every form. The Handle/auto-coerce step happens at the *receiving view's element type* (and at typed slots per 017-153), NOT at the placement. Never describe one placement form as "binding to Handle" or "binding to slice-of-Handles" as if it's a different surface — both are borrows; the Handle-ness lives in what the borrow is into. Example: `[n1 n2] as pair` binds `pair` to a borrow (a slice into the bundle's backing); the slice's *element type* is `Handle[T]` because the bundle's backing stores Handles.

17. Delegate large-doc reads to subagents: When pulling specifics out of large reference docs (SPEC.md, DECISION_LOG.md), spawn read-only subagents that return short focused summaries instead of reading wide ranges into my own context. Keeps my context lean and lets many lookups run in parallel. Example: for GRAMMAR.md gap-filling, dispatch one subagent per construct (placement, with-expr, observe, …) asking only "give me the surface syntax shape in 5–15 lines", not reading §13.8 myself end-to-end.

18. Double-check via fresh subagents even for things I've already read: Independent re-reads catch misreads, stale recall, and silent assumptions. The cost of redundancy is cheap; the cost of acting on a wrong recall is not. Treat my own prior read as one data point, not ground truth — when stakes are real (writing a normative artifact like GRAMMAR.md), verify with a fresh agent.

19. Node-type body members are clauses only (`satisfies`, `children:`/`incoming:`/`outgoing:`, `view`, `when:`, cells, `effects:`, `expose:`). Placements emitted by the type live in `expose:` (or `effects:`). Placement bodies of caller-side placements hold child placements only — no `expose:`, no clauses. Example: incorrectly putting `expose:` inside a `main App my_app:` body conflated node-type body with placement body.

20. `is not` is two words, never `is_not`. Confirmed in DECISION_LOG 007-85 and 007-197; the underscored form is wrong everywhere.

21. Never write entry-number citations inside LOG entries: Invariant 2 forbids an entry referencing another entry by number or name — when an entry needs another rule's content, restate it locally; elaboration pointers go only through the trailing (§) ref. This applies to edit BRIEFS too: a plan that says "add the exception (per 005-6)" will be followed literally by an editor and produce a violation. Example: 005-65's exception list must read "(1) marker traits, (2) methodless marker traits, ..." — not "(1) marker traits (005-6), ...". Any edit batch touching the LOG must include an Invariant-2 gate (scan changed lines for NNN-M occurrences beyond each entry's own id).

22. A superseding ruling must re-sweep the superseded item's exec-block site list: plan item bodies go stale when a later owner ruling overrides their recommendation (the rulings ledger is the corrections layer and always wins), but the stale "For execution" blocks still carry entry-id/site lists the new ruling must re-visit — execute the ruling against those sites, not just the ledger's summary. Example: D-15's final ruling (views not Iterable at all) replaced the Model-B carve-out its item body recommended; the batch executed the ledger but missed the item's mechanism sites (018-38 "always iterable", 018-74/SPEC §13.5.4.2 "iterates via Iterable::iterator" stated universally, SPEC §13.3.3.4's retired "for is compile-time" rationale) until a post-hoc exec-block sweep caught them.

23. Verification is not complete until the ENTIRE diff is read end-to-end: greps and gate checks only confirm the specific claims they encode — absence of a pattern, a count, a header number. They cannot catch wrong prose, subtle ruling drift, or a defect nobody wrote a grep for. Grep is not e2e verification; reading every hunk of the uncommitted diff personally is. Example: after the D-20/21/26/30 batch I grep-verified the gate claims, read only the four new LOG entries, and reported "verified" — the owner had to ask "did you verify all uncommitted diff?" before I actually read all three files' hunks.

24. Design questions must arrive protocol-complete, every time: short plain-words background (what the thing is, why it exists), the problem, options each with a code example, recommendation + why, all terms defined before use. A compressed one-line "flag" ("coordinate -> scope-key rendering vs 018-28/29") is not a question — it forces the owner to either do homework or rubber-stamp, and they will answer neither. This applies to END-OF-BATCH flags exactly as much as to planned decision items; a flag that requests a ruling IS a design question. Example: the "ordinal" incident and the three post-batch flags the owner refused to answer until restated per protocol.

25. Never say "stdlib" for language-builtin concepts: streams, their policy machinery (Ring/Gate and the policy-parameterized stream family), Map, and the other language-provided primitives are LANGUAGE builtins — lower level than any library. "stdlib" means the separate, external library layered on top (D-29's distinction: Map is language-provided; Vec is borrowed from the stdlib's future spec). Calling a language builtin "stdlib" is a category error in both prose and LOG/SPEC text. Example: describing RingStream[T, N] as "stdlib sugar" — wrong; and LOG 030-261's own "The stdlib provides transparent generic aliases…" wording carries the same defect.
