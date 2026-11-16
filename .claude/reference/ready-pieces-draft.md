# The three pieces that were called ready for their documents, second draft

By designer 2, 2026-11-16. Nothing here is written to its file before the owner's yes: each piece is new essence in its document and is the owner's to approve. The entries of `owner-rulings-compiler-design.md` behind a block stand in square brackets after it and are no part of the text. The first draft is in git, commit `c886ac1b`.

## What the cold read found, 2026-11-16

Two fresh readers on Opus, neither given the peer's reasoning. One held every sentence against the owner's words. The other read as someone who writes a host, as a builder of one mode, and as an agent that knows only the SPEC.

- **Piece 1, the modes' specifications, is not ready.** It rests on four things the owner has not ruled; they are listed under Piece 1 and go to the owner as questions before any text.
- **Piece 2, the architecture's opening rules,** stands after its fixes: a rule's scope, two rulings that were missing, one ambiguous word.
- **Piece 3, the preference lines,** stands after its fixes: one line an entry, the owner's own terms kept, and one line already in the file that the rulings contradict.

Findings not taken, with the reason: that "turned into Rust" says how and not what (a framework's build takes the generated Rust in, so its author relies on it); that rule 3 belongs in a specification (the owner ruled it as one of three rules of the design, eleventh entry); that the top document's rules should say the design of the interfaces goes to the owner first (a document does not say who approves it).

## Piece 1: the modes' specifications, held

Open, and the owner's:

1. **Whether a page common to both modes exists.** Ruled: each mode gets a specification of its own (eleventh, sixteenth). Not ruled: a third page for what the two share. Designer 1 showed it to the owner in a table on 2026-11-16 and got no answer.
2. **What loading is in the compiled mode.** The ruled sentences say the interpreted mode's Program "arrives as data, at load" and the compiled mode's is "built together with the runtime" (tenth). SPEC §12.3 and §12.6 give every Program a load, which evaluates every cell and allocates every buffer, and §12 is "said once for every mode" (eleventh). Read cold, the two ruled sentences say the compiled mode has no load.
3. **Reload in the SPEC.** §13 says what a reload is, and §14 says "A different structure is a different Program, loaded by reload (§13)". The compiled mode never reloads (sixth). A reader of the SPEC alone takes reload as always there. A sentence of the SPEC is the owner's.
4. **Who approves a change of a mode's specification.** Designer 1 told the owner it follows the SPEC's rule, as a default for a veto. The owner's words in the eighth entry, "SPEC is the only document where every essential/design change, derived or not, goes to me", were said before the modes had specifications.

Decided by the peer for the text, to be seen in it: no sentence of rank or of use stands in a specification ("the main mode", "a convenience", "an edit runs in"), since none is a promise and "a convenience" reads as a weaker promise of results; the rank goes into the architecture's rules as the bar both modes are held to (Piece 2, rule 9). "Both modes" and not "every mode", since the owner's words say both. "Every speed figure" and not "every speed promise", since SPEC §1.1 already promises bounded work in every mode. "A mode's specification names no runtime", and no sentence that a mode can run on several, which reads as a promise of a second runtime. The hand-off is said in the architecture alone. What each mode's file says is not said again in the common page.

The text as it would then read, a sketch and no proposal until the four are answered:

- The common page: what a mode is, "when the Program (SPEC §11) meets a runtime: at load, as data, or at build, as Rust"; what a runtime and a target are, in the SPEC's sense; that the SPEC holds in both modes, the host loading, writing and committing and a consumer reading as §12 says; that both modes give bit-identical values (§12.5); that a mode's specification names no runtime.
- The interpreted mode: the host hands the runtime a Program as data when it loads it; it offers reload, as SPEC §13 says.
- The compiled mode: the Program is turned into Rust and built together with the runtime, and is fixed at build; the host loads it at its start as §12 says; it never reloads, and a different Program needs a new build; every speed figure is stated for this mode; a device carries the runtime and the generated code, and no compiler.

## Piece 2: the architecture's opening rules

For the top document of the architecture, ruled in the twentieth entry: "a short top document with the map, the contracts and the rules that bind every part". The peer's choice, not ruled: the top document keeps the name `packages/symphonyscript/docs/ARCHITECTURE.md`, which the SPEC cites, and the previous design stays in that file below the new head until its sections are replaced.

The head replaces this opening paragraph of `ARCHITECTURE.md`:

> This document describes how the implementation realizes `SPEC.md`. The SPEC defines what programs mean; this document defines the crates, data structures and interfaces that compute it. A conflict between the two is a defect the owner resolves.

With this, placed between the title and `## 1. Crates`:

This is the top document of the architecture. It holds the map of the parts, the contracts between them and the rules that bind every part. The compiler, the runtime and each mode have an architecture document of their own, and `INDEX.md` lists them. The specifications say what is promised, and the architecture says how it is built. A conflict between the two is a defect the owner resolves.

The architecture is being designed again. Of the new design this document holds the rules that bind every part, and nothing else yet. Sections 1 to 10 below them are the previous design, kept until their successors are written. They are not the design. The SPEC still cites some of them, and each citation moves when its section is replaced.

## Rules that bind every part

1. **The Program is the only hand-off.** The compiler ends at the Program. The interpreter loads it and the generator translates it, and neither reads the source.
2. **Generated code meets the runtime through a narrow, stated seam.**
3. **Hosts and consumers are written against an interface,** and not against the runtime itself.
4. **A contract's exact shape is written once, in code,** as interfaces with no implementation inside. Its meaning stays in the specifications.
5. **Every rule of the language is written once.** The operations on values and the commit's rules live in one place, and compile-time evaluation, the interpreter and the generated Rust all call them. Equality bit for bit between the two modes then holds by construction, and a test over every fixture guards the wiring.
6. **Tests of running behaviour run in both modes.**
7. **A device carries no compiler.** What ships to it is the runtime and the generated code.
8. **The Program is kept plain data,** so that a Program that travels to a runtime elsewhere stays possible. Nothing is promised about that.
9. **Both modes are held to one bar.** Generated Rust is the main mode and the interpreted mode the secondary one, and the interpreted mode is no lesser work: it is efficient, optimized, well planned and well architected.

Rules 1 to 3 hold whether or not a second runtime ever comes, because a contract is defined against an interface and the implementation is passed in.

[The first paragraph: twentieth, sixteenth, fourteenth; its last sentence is kept from the old opening. The second paragraph is the peer's. 1 to 3 and the closing sentence: eleventh, and tenth for the second and third sentences of 1. 4: twentieth. 5 to 7: tenth, and sixth for "what ships". 8: tenth. 9: first and sixth.]

## Piece 3: lines for `.claude/OWNER_PREFERENCES.md`

Four new lines, and one line already there that the rulings contradict.

> In the SPEC, every change of essence, derived or not, is approved by the owner. The peer reviews and approves or rejects only a change that changes no essence: a mechanical one, or a clarification, which it is only when the rest of the SPEC already leaves a single reading.

> In the code standard and the architecture, new essence is approved by the owner. A mechanical change, and essence that derives cleanly and fully from what the owner approved, is reviewed and approved or rejected by the peer.

> Only settled things go into the SPEC, nothing open or half open, and a change of it is cold-reviewed under heavy scrutiny before it is committed and pushed.

> A specification is owed for what someone can observe or rely on, and it is said once; how a thing is built is architecture.

[In order: fifth and eighth; eighth; second; fourteenth.]

The line already in the file says a SPEC sentence is written only after the owner approves it, which the fifth and eighth entries changed for a change of no essence. It replaces:

> Text proposed for a file — a lesson, a preference, a SPEC sentence, a skill rule, or an entry for the message catalogue among it — is shown to the owner in the chat in a quote block of its own, with the exact line it replaces when it refines or widens one and the exact line itself when it removes one, never a bare ID or line number, and is written only after the owner approves it.

With:

> Text proposed for a file — a lesson, a preference, a SPEC sentence that is the owner's to approve, a skill rule, or an entry for the message catalogue among it — is shown to the owner in the chat in a quote block of its own, with the exact line it replaces when it refines or widens one and the exact line itself when it removes one, never a bare ID or line number, and is written only after the owner approves it.

Waits on the fourth question of Piece 1: whether the first line says "In the SPEC and in a mode's specification".
