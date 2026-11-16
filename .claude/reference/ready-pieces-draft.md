# The pieces "ready to be recorded", as drafted on 2026-11-16, and why they are not ready

Drafted by designer 2 from the owner's rulings, read cold twice (the limit), not shown to the owner as text, nothing written to its file. The second cold read left findings that are questions for the owner or for the design, listed under each draft. The drafts are kept so that the next attempt starts from them and not from nothing.

## A. The modes' specifications, a first version (three files beside the SPEC)

`MODES.md`:

> # SymphonyScript modes
>
> This document defines what a mode is and what holds in every mode. It is normative. `SPEC.md` names no mode; each mode has a specification of its own, which says only what differs in it.
>
> A mode is when the Program (SPEC §11) meets the runtime. There are two modes, the interpreted and the compiled.
>
> - What compilation produces does not depend on the mode: one Program serves every mode.
> - Every mode runs the Program as SPEC §12 says, bit for bit as §12.5 says, and a mode that offers reload reloads as SPEC §13 says.
> - A mode names no runtime: the same mode can run on different runtimes.
> - The compiled mode is the main mode: what ships is built in it, and every speed promise is stated for it. The interpreted mode is the secondary one, a convenience: the edit loop runs in it, with live reload.

`MODE-INTERPRETED.md`:

> # The interpreted mode
>
> This document defines the interpreted mode of SymphonyScript. It is normative and says only what differs in this mode; what every mode shares is `MODES.md`.
>
> - The Program arrives as data, at load.
> - The mode offers reload (SPEC §13).

`MODE-COMPILED.md`:

> # The compiled mode
>
> This document defines the compiled mode of SymphonyScript. It is normative and says only what differs in this mode; what every mode shares is `MODES.md`.
>
> - The Program is turned into Rust, the generated code, and built together with the runtime.
> - The mode offers no reload: replacing the Program needs a new build.
> - A device carries the runtime and the generated code, and no compiler.

What is open in them:

1. The page `MODES.md` itself is the peer's pick and not ruled; with it the index's line on the modes, in words the owner approved, becomes false, and who approves that update is not said.
2. SPEC sentences presume a Program that is loaded as data and replaced by reload: §1.1 "A host loads the Program into the runtime", §12.3 "Loading a Program evaluates every cell", §12.6 "Loading it allocates every buffer", §14 "A different structure is a different Program, loaded by reload (§13)". "Every mode runs the Program as SPEC §12 says" binds the compiled mode to them before what load is there is settled.
3. The promise that the two modes agree bit for bit is ruled (sixth and tenth entries) and is not SPEC §12.5, which speaks of targets; it needs its own sentence and one home.
4. Main and secondary, what ships and where an edit runs: one relation, not a property of every mode, and by the what/how rule perhaps no specification at all. "A convenience" without the owner's condition (sixth entry) invites the reading the owner refused. Whether the interpreted mode may be what a finished product runs is not ruled.
5. "There are two modes" closes a list the owner left open (a mode compiled to WebAssembly, tenth entry), and `MODE-COMPILED.md` is Rust by its first sentence.
6. "Neither reads the source" (tenth entry) is something a host can rely on and is in none of the three. "Compilation" can be read as the Rust build.
7. "A device carries … no compiler": "device" is not the SPEC's word, "no compiler" does not say which, and the state page had planned the sentence for the architecture.
8. "Arrives as data" can read as a promise of a format, which the owner left open (tenth entry).
9. The names: no pattern yet that also names each mode's architecture document.

## B. The architecture's opening rules

Not shown and not to be asked: every sentence is the owner's own (tenth, eleventh, twentieth and sixth entries), so by the eighth entry it is the peer's to write. Written with the outline of the architecture, where the top document gets its name and place. The text as drafted, after its cold read:

> A contract is defined against an interface, and the implementation is passed in. Three rules ensure that design, and they do not depend on there ever being a second runtime:
>
> 1. The Program is the only hand-off. The compiler ends at the Program in both modes; the interpreter loads it and the generator translates it, and neither reads the source.
> 2. Generated code meets the runtime through a narrow, stated seam.
> 3. Hosts and consumers are written against an interface, and not against the runtime itself.
>
> A contract's exact shape is written once, in code, as interfaces with no implementation inside. Its meaning stays in the specifications.
>
> Every rule of the language is written once. The operations on values and the commit's rules live in one place, and compile-time evaluation, the interpreter and the generated Rust all call them. Equality bit for bit then holds by construction, and a test over every fixture guards the wiring. Tests of running behaviour run in both modes.
>
> The interpreted mode is secondary and no lesser work: it is efficient, optimized, well planned and well architected.
>
> The Program is kept plain data, which keeps it possible to send a Program to a runtime elsewhere; nothing of that is promised.

## C. Lines for the owner's preferences file

The line that would be replaced, since as it stands it sends every SPEC sentence to the owner:

> - Text proposed to the owner for a file — a lesson, a preference, a skill rule, an entry for the message catalogue, or a change of a document that is the owner's to approve among it — is shown in the chat in a quote block of its own, with the exact line it replaces when it refines or widens one and the exact line itself when it removes one, never a bare ID or line number, and is written only after the owner approves it.

Who approves a change:

> - Who approves a change of the SPEC, of the code standard with the testing and reviewing guidelines beside it, and of the architecture's documents: a change of essence, one that changes the design, is the owner's. In the SPEC that holds whether or not the change derives from what the owner approved; in the others the peer approves essence that derives cleanly and fully from what the owner approved. A change that changes no essence is the peer's in all of them: a wording, a gap closed or a contradiction corrected, where the rest of the document already leaves a single reading; where two readings each fit, the change is design and the owner's. The modes' specifications follow the SPEC's rule. Every change, whoever approves it, is read cold before it is applied.

What and how, the fourteenth entry word for word, with no finding against it:

> - A specification is owed for what someone can observe or rely on, and it is said once; how a thing is built is architecture.

What is open in them:

1. In the SPEC, a gap closed where the SPEC is silent: the fifth entry gives gaps closed within the approved design to the peer, the eighth gives every design change of the SPEC, derived or not, to the owner. Where the line between them runs is the owner's to say; the single-reading test is necessary for the peer's approval and may not be sufficient.
2. Not the owner's words: "the testing and reviewing guidelines beside it" (the owner answered "the principles or the architecture"), "The modes' specifications follow the SPEC's rule", "what the owner approved" for the owner's "the prior essence", and the gloss of essence as "changes the design", which is wrong for a coding rule.
3. "Every change is read cold" is the peer's rule; the owner's words are "cold-reviewed under heavy scrutiny" for the SPEC and "reviewed and approved (or rejected)" for the others, and the peer skill exempts mechanical edits from a cold read.
4. In the replaced line, "that is the owner's to approve" can attach to "a document", and the line no longer says by itself that a lesson goes to the owner.
5. A standard that is still a draft has no approved essence to derive from; the state page's sentence that a draft follows the same rule is not in the line.
6. With the line written, the table in `design-state.md` and the notes' pointer to it say the rule a second time.
