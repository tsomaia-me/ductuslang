# The code principles, as shown to the owner on 2026-11-15

A draft: nothing on this page is settled or approved. The peer sent it to the owner in the chat on 2026-11-15, for the owner to keep, change or strike each principle and add what is missing. It says the code standard, `packages/symphonyscript/docs/STANDARD.md`, in plain words without its rule numbers. Below it: the owner's words on it so far, and one rewording the peer showed in answer.

## The page, word for word as sent

1. **The SPEC decides what the code does.** Code is written from the SPEC's sentence, never from an example, a test or older code. Where the SPEC is silent or says two things, the gap is written down with the sentences quoted and brought to you; work goes on with what it does not block. Refuses: guessing what was meant.
2. **Design before code.** The design page sets the edges of a piece of work. The builder's plan of its types and passes is reviewed and accepted before any function body is written. Refuses: structure that grows by accident.
3. **One rule, one place.** Each rule of the language is implemented once, and that place carries a note naming its SPEC sentence. A list of forms is data with one function over it. Comments say only what the code cannot. Refuses: copies of a rule that drift apart.
4. **Every case is handled by name or refused by name.** No catch-all, no fallback, no default that quietly accepts the unknown. A new case stops the build until someone decides it. Refuses: an `else` that swallows what nobody thought about.
5. **An error is part of the result, never a guess.** A broken piece leaves a marked hole in the data. Nothing invents a value to keep going, and nothing reads a flag or an error list to decide what to do next. Refuses: recovering with a guess.
6. **Data has one writer and a type that says what it is.** Each table is filled by one function and an entry never changes afterwards. An id of one table cannot be used on another. Refuses: one large object every step edits.
7. **Nothing hidden, and one direction.** A function gets what it needs as arguments: no globals, no reading files inside the compiler, no result that depends on timing, threads or hash order. Each part uses only the parts before it. Refuses: ambient state and tangled dependencies.
8. **Decide once, where the facts are.** A decision is made by the first step that has what it needs. Refuses: guess now, patch later.
9. **Cost is designed, not discovered.** Work grows in step with the input, broken input included. Nothing allocates or copies text per item, text is hashed once, and nothing recurses on how deeply the source nests. These are tests that count work and take time, set by the design. Refuses: make it work first, fast later.
10. **It never crashes.** No input makes the compiler panic; impossible states are ruled out by types. The runtime cannot panic at all. Refuses: `unwrap` and hope.
11. **Behaviour is proved by tests the builder did not write.** They come from SPEC sentences, their expected results never come from running the code, and the builder may not change them. Refuses: tests written afterwards to match the code.
12. **Borrow the generic, write the core.** Unicode tables, hashing and float printing come from libraries. The compiler's own machinery is written here, and no builder adds a dependency alone. Refuses: a library to skip the hard part.
13. **A review reports faults that hold up, not taste.** The reviewer first writes down how the code works. A finding names its place, quotes what it breaks, and says what goes wrong. A second reader repeats each finding before the builder sees it. A judgement on structure counts only with a concrete cost. No rule counts lines. Refuses: preferences dressed as findings.

**When these do not settle a choice:** the SPEC or a question to you first, then these principles, then cost, then the form easiest to check against its SPEC sentence. Two principles that cannot both be met come to you.

One thing I do not see among them, which you named as something you care about: a principle on simplicity and readability in its own right. Today the nearest is "easiest to check against its SPEC sentence".

## The owner's words on it so far

On the second principle, word for word:

> 2 - you mean like every unit will go in its own design phase involving me, after already having designed SPEC, STANDARD and the general architecture, principles  and bar?
> or does "design before code" mean something different here? I'd like to know.

The peer answered that on paper today the standard has the owner approve every unit's design page, that this is not what the owner asked for, and showed this rewording of the second principle. The owner has not said yes or no to its words:

> **Design before code.** The architecture, designed once with the owner, says what each part takes and hands on. For each piece of work the builder first writes its plan of types and passes, and a cold reader checks it against the architecture and these principles before any function body is written. The owner sees a piece's plan only when it meets something the architecture and the principles do not settle.

The owner's next message, word for word:

> I don't know what I am about to say if it even makes sense, but imagine you are CTO or R&D lead and you have multiple engineering teams, each having several engineers within, working on a project. Now imagine that you designed specification for the project and the scope, designed architecture and general guidelines that everyone should follow and govern writing and reviewing code from. That you designed standards and processes to ensure everything flows smoothly. but then, imagine that every single developer, every single day hands to you a technical document of its scope, then a code to review, that they could have handed to their immediate engineering lead instead who would govern from the set spec, standard and guidelines and would ask the R&D lead (you) only for those genuine cases where the spec and the documents genuinely don't cover it and the question is of nature that has either multiple valid answers or no clear winner based on all the given context.
> 
> Resembles something? well, that R&D lead in those scenarios is me and the engineering team leads are orchestrator agents (peers) and engineers are subagents.

The peer put that picture into `.claude/HANDOVER.md` under "The roles", in words the owner approved with "go ahead." The owner has said nothing else on the thirteen principles. The owner took "The roles" out of the handover page again on 2026-11-15 (commit 2e72fd7a), so the picture now stands only in the message above.

## The second principle, reworded by the third peer and shown to the owner on 2026-11-15

A draft: the owner has not said yes or no to its words.

What the owner wrote on it into the peer's notes, in the owner's own hand (commit 2e72fd7a), word for word:

> Code standard is has been drafted, but owner dislikes the second principle and wants it changed in a way that pre-established first principles, guidelines and standards govern the execution pipeline without interruption (only exceptions would be the genuine novel cases that cannot be judged by the peer based on prior context and docs - either because it has several valid answers or no clear winner. Those primarily would be cases which neither SPEC, nor the STANDARD coverd.)

And the owner's words to peer 1 the same day, 13:35 UTC, word for word:

> where I want to be involved is DESIGNING those first principles guiding the code writing and reviewing process fully so that the work goes as seamlessly as possible and only involving me DURING execution if agents truly hit some uncovered path that cannot be derived from the first principles cleanly or have more than one valid answer.

The rewording as shown, cold-read once on Opus with its findings worked in:

> 2. **Design before code.** The SPEC, these principles and the architecture are designed with you before the work starts. The architecture names the parts of the compiler and the runtime and what each takes and passes on. Every piece of work is then designed inside them, without you. The peer writes the piece's design page, and the builder writes its plan of types and passes. Each is read by a reviewer who did not write it, and the peer accepts it before any function body is written. A question of design reaches you only when the SPEC, the architecture, these principles and the decisions already written down leave it with more than one valid answer or with no clear winner. The peer decides every other question and writes the decision down with its reason. None of the three documents changes without you. Refuses: structure that grows by accident, and work that stops to ask you what is already answered.

Said to the owner with it: the first principle is left as it is, so every gap in the SPEC still comes to the owner, the language being the owner's; and the handover page's sentence, "a case the principles do not cover, cannot settle cleanly, or that has more than one valid answer", read word by word sends every uncovered case to the owner, which is wider than this text.

What the cold reader found that the standard's sentences must then say, for the change of `STANDARD.md` and `FINDINGS.md` that follows the owner's yes:

- Sentences that put the owner into each unit: `STANDARD.md` line 16 (the page, "approved by the owner"), line 37 (the peer "writes pages with the owner"), line 59 ("before the page was approved", so the peer then approves a library), line 122 (two lines that cannot both be met go to the owner, with no condition), line 128 (the owner approves catalogue entries with the page).
- What the principle relies on and neither document holds: the architecture as a document (the standard uses the word only for its own part 1); a cold reader of a page (`FINDINGS.md` lists four readers and none reads a page); who accepts a page; who accepts a builder's plan (1.7 says only "the accepted proposal"); who answers a GAP and who decides an OBJECTION (3.4 and 3.6 name nobody); where the peer writes a decision and its reason; the condition for asking the owner; where a page's budgets come from.
- On the page itself: the line under the thirteen, "the SPEC or a question to you first", and "Two principles that cannot both be met come to you", carry no condition.
- `ARCHITECTURE.md` line 3, "A conflict between the two is a defect the owner resolves", carries no condition either; the file is being replaced.

The owner's answer to that rewording, 2026-11-15, word for word:

> 2 - in a nutshell, sounds right-ish. to make sure, draw me a simple textual mermaid diagram of the workflow (including me). I just need to make sure that its as efficient as possible while ensuring the best possible quality as well. I need to make sure there are not too much moving pieces basically.

The diagram the peer showed in answer: `.claude/reference/workflow-as-shown-2026-11-15.md`. On gaps in the SPEC the owner ruled otherwise than the peer had left it: `.claude/reference/owner-rulings-compiler-design.md`, fifth. So the first principle's "brought to you" and the second's "None of the three documents changes without you" are both to be reworded to that ruling.
