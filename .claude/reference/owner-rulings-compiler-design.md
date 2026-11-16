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

---

# Tenth, 2026-11-15: the modes at a high level

Said by the owner in the chat to the peer, the same session, each on the peer's sentence quoted before it.

On "My proposal for what a mode is: when the Program meets the runtime. Interpreted. The Program arrives as data, at load. The runtime walks it and can take a new one, which is reload. Compiled. The Program is turned into Rust and built together with the runtime. Replacing it needs a new build.":

"this is what I had in mind. no argument here."

On where the modes are specified, the owner's words, which lean to the second choice and were not yet confirmed as a ruling when this was saved:

"what I want to decide is whether a.) two modes go in the original SPEC or b.) get SPEC-s of their own. what if we have a third mode in the future - compiled, yes, but to webassembly. (a) means every other mode gets bolted on to the original SPEC, SPEC which stays agnostic and finishes its job at The Program.

that leaves the clear winner: "One Program, two takers. The compiler ends at the Program in both modes. The interpreter loads it and the generator translates it. Neither reads the source." this."

The peer's note, added on 2026-11-16: the owner's "this." rules One Program, two takers, and the choice of specifications of their own was confirmed in the eleventh entry.

On "Every rule is written once. The operations of component 6 and the commit's rules live in one place. Compile-time evaluation, the interpreter and the generated Rust all call them. Equality bit for bit then holds by construction, and a test over every fixture guards the wiring. A device carries the runtime and the generated code, and no compiler. Tests of running behaviour run in both modes.":

"agreed."

On "One runtime or two. [...] My pick is one. How far generated code specialises storage is then measured, not guessed.", a leaning and a question, no ruling:

"for starters I am leaning towards one. question: if we start with one, how difficult would it be to switch to two - one-per-mode - in the future?"

On "Can a Program travel? [...] My pick: promise nothing now, and keep the Program plain data so this stays possible.":

"exactly your pick. the time (and work) will tell, whether it can stay as pure, serializable data."

---

# Eleventh, 2026-11-15: contracts against interfaces, always; the modes get specifications of their own; what the SPEC keeps

Said by the owner in the chat to the peer, the same session, each on the peer's text quoted before it.

On the three things the peer said keep a later switch to two runtimes contained ("The Program is the only hand-off. [...] Generated code meets the runtime through a narrow, stated seam. [...] Hosts and consumers are written against an interface, not against the runtime itself."):

"all three should hold, EVEN IF we were sure we would never want second runtime, why? first, those three ensure the right design: define contract against abstraction (interface), pass the implementation. that is like clean code 101.  and second, however "sure" you are in the present, you can never speak for your future self: define the way your future self would thank you for, especially when it costs nothing because you are not rewriting anything, but are in the very moment of designing the whole thing."

On "I read you as choosing b: modes get specifications of their own, and the SPEC stays free of them. [...] Confirm, and I record it as your ruling.":

"your reading is correct."

On "The SPEC keeps §12 and §13, said once for every mode. It ends at the Program and what a Program means. Each mode's specification says only what differs: when the Program meets the runtime, whether it offers reload, what a host is written against, and what it promises. The architecture says how all of it is built.":

"I am on board with this."

And a question the owner put with it, answered by the peer the same day:

"now I have a question: is "mode" and runtime the same thing?

like whether you interpret or compile (and to what you compile) and how you run The Program - are those the same thing?"

---

# Twelfth, 2026-11-15: mode, runtime and target; the owner's questions on the runtime and on the contracts between layers, no ruling

Said by the owner in the chat to the peer, the same session. On the peer's three definitions ("A mode is the form in which the Program reaches the runtime: as data, or as code. [...] A runtime is what runs a Program the way SPEC §12 says. [...] A target is the machine it all runs on."):

"now the picture is clearer.

And we already know that the SPEC is agnostic of modes.
Now the question: is it also agnostic regarding runtime, or defines it?
IF the answer is no and the SPEC does not define the runtime, and given that there is a possibility of multiple runtimes just like multiple modes - should we have runtime SPEC defined separately from the language spec and mode specs?
and how (and where) do we define  the contracts/interfaces through which each layer talks to each other?

I guess, and correct me here if I'm wrong, the contract of the language is The Program?"

On the peer's "So a mode's specification names three things: the form the Program takes, the runtime it runs on":

"does it though? if we are separating runtime from mode, and the same runtime can run for different modes, then it's only natural to ask: why can't same mode run on different runtimes?"

---

# Thirteenth, 2026-11-15: no separate runtime specification, and a mode does not name a runtime

Said by the owner in the chat to the peer, the same session.

On "A separate runtime specification? No. Its content would be §12 and §13 a second time. If a second runtime ever comes, it differs only in how it is built and in what it offers, such as where it runs. That fits a short part of its own, like a mode's.", an agreement and a question, which the peer answered the same day:

"while I agree with this, something tells me modes get SPEC because they are different from runtime, even though SPEC is agnostic of both modes and runtimes - you yourself suggested to have SPEC-s for modes, while you just now suggested that the runtime needs no separate SPEC, so what I wander is what is that something that is telling me all these?"

On "Why can't the same mode run on different runtimes? It can, and my sentence was wrong by your own rule. A mode should not name a runtime.":

"decided."

---

# Fourteenth, 2026-11-15: what a specification is and what architecture is

Said by the owner in the chat to the peer, the same session. On the peer's "The rule underneath is the difference between what and how: A specification is owed for what someone can observe or rely on, and it is said once. How a thing is built is architecture.":

"that is the right distinction."

On the peer's "Of a runtime, the SPEC is silent only about how. What a runtime does is fully written. Sections 12 and 13 are the runtime's specification. They sit inside the SPEC because a cell has no meaning without them.", a question, answered by the peer the same day:

"and there is a good reason for that, right? something tells me that too (about the fact why SPEC is and should be opinionated about what a runtime must do)."

---

# Fifteenth, 2026-11-16: two questions before the handover, no ruling

Said by the owner in the chat to the peer, the same session, while the peer was preparing its handover. The peer's answers are in `design-state.md` under Proposed.

"before I hand off, the stuff we agreed on - is any of it ready to be recorded in the final designs (SPECs, ARCHITECTURE, etc) wherever they belong or still planning ahead is needed?

and second question, for architecture - would you have one giant doc or separate docs per domain (language, modes, runtime - or language+runtime and modes) ?"

---

# Sixteenth, 2026-11-16: each mode gets its own architecture, as it gets its own specification

Said by the owner in the chat to the peer, the same session, on the peer's pick for the architecture's documents, "One for the modes, with a part for each mode, mirroring their specification.":

"here is where I disagree. I think each mode gets its own architecture just ss much each mode gets its own spec. interpreted mode is very different from compiled and peer/builder working on one should not have to read about the other, should it?"

The peer agreed and withdrew both its pick and its earlier default of one document for the modes' specification.

---

# Seventeenth, 2026-11-16: the handover as a rule, an index of the documents, and how a question is answered

Said by the owner in the chat to the peer, the same session, in one message. The first is an order for a rule, the second a question for discussion, the third an order for a preference. Their lines for the skill and the preferences file are written only on the owner's yes to their words.

On the peer's offer to start the successor and check its first read-back against the work:

"I'd prefer that yes, that way you could make sure its grounded: and make it a rule, when context-watch notifies you about coming compaction, take it as a signal to wrap up your work, update the notes (and drain it of all the excess: and set the format please, because peers like to bloat it from time to time), and to write  a handoff note, which human approves and then you launch the new peer and ground it and make sure its grounded and finally retire, set [retired] in front of your session title. as all peers have numbers, for example you are "SymphonyScript designer 1", make it incremental."

On the documents:

"and should we have some INDEX file that lists all docs: SPEC (langauge), mode SPECS, language ARCHITECTURE, architecture files for modes, our first runtime architecture, testing&probing guidelines, coding guidelines, reviewing guidelines, anything else? INDEX file that every peer does read and inside, each domain gets its own short, very concise section and references to the files, so that peer is aware about them and reads them when needed (except SPEC, which every peer reads of course)

thoughts?"

On the handover and on questions:

"don't start any hangover yet until we discuss the questions I raised here. and make it a preference: when my prompt contains a question that needs ongoing dicussion, instead of one-off answer - answering it gets the priority over taking further actions. when it contains simple question that get  one-shot answers and does not require discussions, such as questions about the settled things, state of things, about facts and stuff - then just make sure that you answer them first and only after that continue with what you are doing. in both cases, peer is not supposed to cancel any prior actions thats been already running."

---

# Eighteenth, 2026-11-16: yes to the index, the handover rule, the notes' format and the preference on questions

Said by the owner in the chat to the peer, the same session, on the peer's message that gave its thoughts on the index (one job, lasting documents only, a section a domain, every peer and agent starts from it, the tester gets a guideline document of its own), the draft of the peer skill's handover paragraph, the notes' format as set, and the draft of the preference on questions, the last two lines marked as not yet cold-read:

"yes to all"

The two lines were cold-read after that, and their words changed in small ways; the owner's yes to the changed words is the nineteenth entry.

The part of that message on the index, the peer's words that the owner's yes covers, word for word:

> ## The index: yes
>
> - **One job.** It says which documents exist, what each is for in one line, its standing, and who reads it when. It never restates a document.
> - **Lasting documents only.** State stays in the notes and your words stay in the rulings file.
> - **Every peer and every agent starts from it.** The SPEC is the one document everyone reads whole.
> - **It takes the list of documents out of the notes,** which is where most of the bloat sits.
> - **A small test can check that every file it names exists.**
>
> Its sections, by domain:
>
> - **Language:** the SPEC, and the compiler's architecture.
> - **Modes:** the page on what every mode is, then a specification and an architecture for each mode.
> - **Runtime:** its architecture. Its specification is SPEC §12 and §13.
> - **The whole:** the architecture's top document, with the map and the contracts.
> - **Writing, testing, reviewing:** see the gap below.
> - **The work:** the work model and the agent files.
>
> The gap is in your own list. Coding guidelines exist as the code standard, and reviewing guidelines as the findings file. Testing and probing guidelines have no document: their rules are spread over those two. The work model has three kinds of agent, so I would give the tester its own document. A tester then does not read the builder's rules.

---

# Nineteenth, 2026-11-16: yes to the two lines' words and to the handoff note

Said by the owner in the chat to the peer, the same session, on the final words of the peer skill's handover paragraph and of the preference on questions, shown in quote blocks after their cold read, and on designer 1's handoff note with the successor's prompt:

"yes."

---

# Twentieth, 2026-11-16: where the contracts' exact shape is written, and the architecture's documents

Said by the owner in the chat to the peer, designer 2 (session `session_01Cbzj47fDGft1eyp2vyvysD`), each on the peer's sentence quoted before it. Both sentences are from designer 2's first message, which put the two unanswered things to the owner again, each with its pick.

The first stood under "The four contracts, and where their exact shape is written", after a line that listed the four: the vocabulary (a framework and the language), the Program (the language and whatever runs or translates it), the rule seam (a mode and a runtime), and writes, commits and changes (a runtime and a framework). On "their meaning stays in the specifications, and their exact shape is written once, in code, as interfaces with no implementation inside. That design comes to you first.":

"this."

On "a short top document with the map, the contracts and the rules that bind every part, plus one for the compiler, one for the runtime, and one for each mode, which you ruled.":

"this."

The peer's note, no ruling: the owner quoted the sentence on where the shape is written and not the line that lists the four contracts. The list is settled with the design of the interfaces, which comes to the owner first; in the same message the peer had said it holds the count open at one place, what the compiler hands a tool, faults and their positions.

---

# Twenty-first, 2026-11-16: what the reference folder is

Said by the owner in the chat to the peer, designer 1, of a file under `.claude/reference/`:

"a reference file, a scratchpad mfor agents during this design work, that should be completely cleaned and removed when the design pass is finished"

---

# Twenty-second, 2026-11-16: yes to the handover rule's words and to the successor

Said by the owner in the chat to the peer, designer 1, on its message that showed the two changes to the peer skill's handover paragraph in quote blocks, after their second cold read, and that repeated its verdict on its successor, "SymphonyScript designer 2", session `session_01Cbzj49BnKWbDEJwU7egSfJ`: that it holds, the questions put to it and its answers having been given to the owner in the messages before:

"yes and yes"

The peer's reading: the first yes is to the rule's words, the second to the successor. The same message asked the owner to push a branch to `main`, which is not a thing a yes answers.
