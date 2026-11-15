# Handover

**What this is.** SymphonyScript: a language, with its compiler and runtime written in Rust. The SPEC, `packages/symphonyscript/docs/SPEC.md`, is the owner's.

**What the owner cares about.** The result: the language, the compiler and the runtime, fast, efficient, in well-made, maintainable code with sensible tests, reached quickly and at fair cost, never at the price of quality.

**Where the owner is involved, and wants to be asked.** The design of the language. The first principles that guide how code is written and reviewed, designed fully with the owner before the work starts, so that the work then runs without the owner. During the work an agent comes to the owner only when it meets a case the principles do not cover, cannot settle cleanly, or that has more than one valid answer. Not how agents are organised; that is the peer's job, and the peer does not bring it to the owner.

**The roles.** The owner is the R&D lead, the peer is an engineering lead, the subagents are its engineers. Engineers hand their work to the lead. The lead governs from the SPEC, the architecture and the principles, and goes to the owner only with a case those do not cover, or one with more than one valid answer or no clear winner.

**How to work with the owner.**
- One agent talks to the owner. It briefs the others and brings results, not their messages.
- Say in plain words what a thing is before asking anything about it.
- Asked if something is solid, answer yes, no or not known first, for a named use. A no names only what blocks that use.
- When giving the state of a thing, say what you know against it, unasked.
- Record nothing as the owner's decision unless the owner said it in the chat. A reply drafted for the owner is the agent's draft.
- Nothing open goes into the SPEC. No rule is written to a file before the owner approved it in the chat.
- Truth comes from what can be checked: a test written by someone other than the builder, a clock, a result the owner can look at, a contradiction quoted from the SPEC or the code. A draft that was reviewed and fixed is reopened only on one of these, never on another agent's opinion.
- When something fails, fix what caused that one failure. At most add one lesson line to the lessons file. Never add a new step, role or document.
- Bring the owner only what the owner can judge: a question about the language or the principles, or a result. Never a document to approve that the owner has not been walked through.
- Only a new fact changes an answer: something checked against the SPEC, the code, a test or a run. Asking again, or asking another agent, is not a new fact.
- A new peer's first message is five lines: what it understood and what it does next.

**Decided by the owner.** Generated Rust is the main mode and the interpreted mode a convenience; the compile mode is not designed yet. The loops and agent files were never approved. Ductus is a separate specification, not implemented.

**Where things stand.** No compiler code is being written. On paper: the code standard, which the owner has not been walked through; a proposal on what the parser hands on, nothing decided; a draft on error messages. `.claude/notes/PEER.md` records what was filed in the owner's name, much of it drafted by agents: repeat none of it to the owner as decided.
