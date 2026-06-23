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
