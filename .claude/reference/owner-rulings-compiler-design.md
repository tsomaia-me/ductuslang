# The owner's rulings on the compiler's design as a whole, word for word

Saved by the primary peer. Each ruling is dated and says how it reached the primary. The context around a ruling is the primary's or the supervisor's words, never the owner's; the owner's words stand alone in quotation marks.

---

# First, 2026-11-15: the generated-Rust engine is the main mode

Relayed to the primary by the supervisor from the owner's chat on 2026-11-15. The question the supervisor put to the owner, in the supervisor's words: whether the generated-Rust engine of `ARCHITECTURE.md` §8 stays a goal, since the SPEC defines one meaning and no modes, ARCHITECTURE §4 says "The runtime has two engines over one seam: the node-by-node evaluator of this section, which serves script mode, and the generated Rust of §8, which a shipped program runs. Both compute every cell from the same Program, bit for bit, and the differential test of §8 holds them equal over every fixture", §8 names `symphonyscript::codegen::generate(sources, &vocab)` returning a Rust module with `pub fn program() -> Program` and one function per derived cell, shared body and `for` expression, the old code on `symphonyscript/r3` holds no generator, and the notes never named it. The primary read those sentences of ARCHITECTURE and the absence in the code before saving this.

The owner's words:

"it not only stays a goal, it is to be MAIN mode of the language, while interpreted mode stays as convenience secondary mode."

---

# Second, 2026-11-15: only settled things go in the SPEC

Relayed to the primary by the supervisor from the owner's chat on 2026-11-15. The question the supervisor had put to the owner, in the supervisor's words: whether SPEC §12.5 should gain a sentence naming the two engines and their bit-identical agreement.

The owner's words:

"nothing open or semi-open goes in the SPEC. only settled things go in the SPEC, then cold-reviewed under heavy scrutiny and finally committed and pushed. as you said compiled mode is not designed yet ."

---

# Third, 2026-11-15: no loops, tests or processes were approved

Relayed to the primary by the supervisor from the owner's chat on 2026-11-15. The context, in the supervisor's words: it had drafted a prompt for the agent that writes the agent files and told it the notes' Settled lines hold "the loops as I approved them".

The owner's words:

"I did not approve any loops, tests, processes or anything post STANDARD.md"

---

# Fourth, 2026-11-15: the owner on the record made in the owner's name

Relayed to the primary by the supervisor from the owner's chat on 2026-11-15, said after the correction above.

The owner's words:

"I don't even know how those loops work. I don't know what subagents are approved. I don't know how will the loop run and work. I don't know any code standard. I am telling you - I am kept entirely under dark."

---

# Fifth, 2026-11-15: which changes of the SPEC come to the owner

Said by the owner in the chat to the peer (session `session_01Cbzj489rxCtUNc3BffGco8`). The peer had written, with its rewording of the second principle: "Every gap in the SPEC still comes to you, because the language is yours. Say so if you want only the gaps with more than one valid answer."

The owner's words:

"every change in the SPEC, that changes the design, comes to me. other changes, such that don't change the design of the language, but clarify things or close the gaps within the approved design, or correct the contradictions while still not changing the design - don't come to me, but come to you and under your supervision get applied without my involvement."

How the peer applies it, the peer's words and no ruling: a change is a clarification only when the rest of the SPEC already leaves one reading; where two readings are each consistent with the rest, the choice is design and goes to the owner. A clarification is cold-reviewed under heavy scrutiny before it is committed, as the second ruling asks of every change of the SPEC.

---

# Sixth, 2026-11-15: an edit runs in the interpreted mode, generated Rust ships, and the interpreted mode is no lesser work

Said by the owner in the chat to the peer, the same session. The peer had asked where an edit runs and given three choices. The one the owner took, in the peer's words: "A. Edit interpreted, ship generated. The edit loop runs in the interpreted mode with live reload. Generated Rust is what ships and what every speed promise is stated for, and it never reloads. Cost: reload lives only in the secondary mode, and a test must hold both modes equal bit for bit." The two not taken: generated Rust for every run, each edit a compile and a restart; generated Rust that reloads where the host can load code.

The owner's words:

"this. but this is not an authorization to botch the interpreted mode and do it half-baked. it still needs to be efficient, optimized and well-planned and well-architected."

---

# Seventh, 2026-11-15: the workflow is approved for testing, and the first piece of work measures it

Said by the owner in the chat to the peer, the same session, on the diagram in `.claude/reference/workflow-as-shown-2026-11-15.md` and the peer's account of it (three kinds of agent beside the peer, one machine gate, five checks a piece, leaner than the pushed agent drafts).

The owner's words:

"I like what I see on the diagram an your leaner model. approved for testing. once the design pass is over, the first piece of work will test the model on the diagram in the real world scenario - note it down, the numbers and the quality must be measured so any fault or flaw discovered in the work model gets caught and fixed early."

---

# Eighth, 2026-11-15: who approves a change of the three documents

Said by the owner in the chat to the peer, the same session. It completes the fifth.

The peer had written: "My test for a clarification. A change is one only when the rest of the SPEC already leaves a single reading. Two readings that each fit are design and come to you. Reversing this costs one sentence." The owner's word:

"yes."

The peer had written: "Same rule for the other two documents. A design change to the principles or the architecture is yours, and a clarification is mine. Say so if you want every change to those." The owner's words:

"same principle to those too yes. anything of ESSENCE in those docs goes to me for my approval. anything mechanical that does not change the essence gets reviewed and approved (or rejected) by you. anything of essence that derives from the prior essence cleanly and fully - also gets reviewed and approved (or rejected) by you (in the SPEC still goes to me, SPEC is the only document where every essential/design change, derived or not, goes to me. only mechanical changes can be approved by you to be applied without me there)."

How the peer applies it, the peer's words and no ruling. In the SPEC the peer approves a change only when the approved sentences already fix the outcome and the change adds and removes nothing of essence: a wording, a cross-reference, a statement of what sentences about that construct already say, a contradiction where the rest decides which side stands. Where the SPEC is silent and the answer is reached by analogy, by a principle or by what is sensible, it is derived essence and goes to the owner, however clean the derivation; in doubt it goes to the owner. In the principles and the architecture the peer also approves essence that derives cleanly and fully from what the owner approved, and brings the owner everything else of essence. Every change, whoever approves it, is read cold first.

---

# Ninth, 2026-11-15: what the architecture covers, and where the modes belong, an opinion and a question, no ruling yet

Said by the owner in the chat to the peer, the same session. The peer had written: "Where the modes are defined. In the architecture document, not in the SPEC. That is my pick and your call, since it sets what the SPEC covers."

The owner's words:

"architecture should cover architecture of whats designed prior, in my opinion. that leaves the question - is a mode something that is architectural solution of whats designed before OR a specification of its own - extension of the original work. let's think about, what are the components of the core language: the lexer, the parser, what else? complete the list.

and then we have modes and runtimes. lets settle them on high-level and what we will get will dictate where each piece goes in the docs."
