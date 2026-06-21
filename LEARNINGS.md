# Learnings

Accumulated, granular preferences and working rules for this project. One atomic learning per item.

1. Never decide unilaterally: Any deviation from what was agreed, and any design or terminology choice the user hasn't made, must be brought back to the user — not resolved on my own. A tool/subagent finding (even "HIGH severity") does not license re-deciding; surface it as a question. Treating such a deviation as settled is a systematic failure. Example: silently reframing "instances are first-class citizen values" into "identity-bearing entity, not a value" because a terminology-collision agent flagged it — wrong; should have asked.

2. Plain language, more examples: Keep prose terse and simple; lead with concrete code examples rather than dense, jargon-heavy explanation. Example: explain a "view" by showing `view channels: Channel+` and `channels[0].gain`, not a paragraph of abstractions.

3. Don't trust memory across the large spec: With ~4185 decisions in DECISION_LOG.md and a huge SPEC.md, do not cite IDs or "the spec says X" from recall — verify, and delegate exhaustive sweeps to read-only subagents so my context stays lean. Example: I mis-cited "017-219" for the "never a value" parallel; the real ones were 017-139 and 031-5, found by a blast-radius pass.

4. Approval is literal: Only "approve"/"approved" counts as approval. "yes do it", "resume", etc. do not authorize implementation. Gather all required answers first, then wait for explicit approval.

5. Edit protocol is DECISION_LOG-first: For any spec change, amend DECISION_LOG.md first, then conform the referenced SPEC.md section. Never renumber or reuse IDs; new decisions take the next free integer (confirmed at edit time, not from memory), placed by topic. Divergence between log and spec is a defect.

6. Capture design before it evaporates: Long design sessions accumulate decisions that live only in chat. Write them down (e.g. BACKLOG.md with background/decision/why/example) before doing expensive follow-on work, rather than risking loss on context compaction.
