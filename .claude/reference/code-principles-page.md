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

The peer put that picture into `.claude/HANDOVER.md` under "The roles", in words the owner approved with "go ahead." The owner has said nothing else on the thirteen principles.
