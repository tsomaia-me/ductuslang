# The design, where it stands

Checkpoint of 2026-11-16, written by designer 1 at its handover and kept by designer 2 since. The peer keeps this page current as rulings land, until its content has moved into the documents themselves. It is the peer's summary and no ruling: the owner's words are in `owner-rulings-compiler-design.md`, cited here as [1] to [24] by entry. "Proposed" means the peer said it to the owner in the chat and the owner has not ruled on it.

## The pass

Two engines were built without a design for how code is written, and both came out poor. The owner stopped the build and opened this design pass on 2026-11-13: the compiler and the runtime are designed again from the SPEC, the rules for writing and reviewing code first, and are then built in small pieces. No compiler code is being written. The language itself is designed: SPEC §1 to §15.

## Settled by the owner

The documents:

- **What and how.** A specification is owed for what someone can observe or rely on, and it is said once. How a thing is built is architecture [14]. The test the peer gave with it: can a user or a framework tell the difference; if yes it is specification, if no it is architecture.
- **The SPEC** holds the language, the Program and what a Program means. It keeps §12 and §13, said once for every mode, and names no mode [11]. Only settled things go into it, cold-reviewed under heavy scrutiny first [2].
- **The modes get specifications of their own** [11], and each mode its own architecture document as well: "interpreted mode is very different from compiled and peer/builder working on one should not have to read about the other" [16]. A runtime gets none, on which the owner agrees [13]: §12 and §13 are the runtime's specification already.
- **The architecture** says how all of it is built [11]. That it covers the architecture of what is designed before it is the owner's opinion, given as one [9]. It is several documents [20]: a short top document with the map, the contracts and the rules that bind every part, one for the compiler, one for the runtime, and one for each mode [16].
- **A contract's exact shape** is written once, in code, as interfaces with no implementation inside; its meaning stays in the specifications. The design of the interfaces goes to the owner first [20].
- **The index of the documents** is written, `packages/symphonyscript/docs/INDEX.md`, in the words the owner approved [17] [18] [23], and the handover page names it, with the SPEC, as what a peer reads first [23]. A test beside it checks that every file it names exists. The tester gets a guideline document of its own [18].
- **The reference folder,** `.claude/reference/`, this page among it, is a scratchpad for the design pass and is cleaned and removed when the pass ends [21].
- **Who approves a change** [5] [8]:

| Change | The SPEC | The code standard and the architecture |
|---|---|---|
| New essence | the owner | the owner |
| Essence that derives cleanly and fully from what the owner approved | the owner | the peer |
| Mechanical, no essence changed | the peer | the peer |

  The owner's words say "the principles" for the code standard. What follows in this paragraph is the peer's application and no ruling, but for the quoted test, which the owner answered with yes [8]. A document that is still a draft follows the same rule. The modes' specification is treated as the SPEC is, the peer's default, told to the owner and not ruled. Every change is read cold first, by a fresh agent that wrote none of it. In the SPEC the peer approves only where the approved sentences already fix the outcome: "A change is one only when the rest of the SPEC already leaves a single reading" [8]. An answer reached by analogy or by a principle is derived essence and the owner's. In doubt it is the owner's.

The modes and running:

- Generated Rust is the main mode and the interpreted mode a secondary convenience [1]. The interpreted mode is still held to the full bar: "efficient, optimized and well-planned and well-architected" [6].
- An edit runs in the interpreted mode with live reload. Generated Rust is what ships and what every speed promise is stated for, and it never reloads [6].
- A mode is when the Program meets the runtime. Interpreted: the Program arrives as data, at load; the runtime walks it and can take a new one, which is reload. Compiled: the Program is turned into Rust and built together with the runtime; replacing it needs a new build [10].
- One Program, two takers: the compiler ends at the Program in both modes, the interpreter loads it, the generator translates it, and neither reads the source [10].
- Every rule is written once. The operations on values and the commit's rules live in one place, and compile-time evaluation, the interpreter and the generated Rust all call them. Equality bit for bit holds by construction, and a test over every fixture guards the wiring [10].
- A device carries the runtime and the generated code, and no compiler [10]. Tests of running behaviour run in both modes [10].
- Whether a Program can travel to a runtime elsewhere: nothing is promised, and the Program is kept plain data so that it stays possible [10].
- Three rules hold whether or not a second runtime ever comes, because a contract is defined against an interface and the implementation is passed in [11]: the Program is the only hand-off; generated code meets the runtime through a narrow, stated seam; hosts and consumers are written against an interface and not against the runtime itself.
- A mode does not name a runtime, and the same mode can run on different runtimes [13]. How, in the peer's words and not ruled: a mode names what it needs from a runtime, a runtime states what it offers, and any mode runs on any runtime that offers it.

The work:

- The work model on the diagram, `workflow-as-shown-2026-11-15.md`, is approved for testing [7]. Beside the peer it has a tester, a builder and a fresh reader, and one machine gate. Each piece passes five checks: its page read cold, a review before code of the plan and the tests, the gate, the code read cold together with the tester's attack on the build, and a second reader who repeats every fault before the builder sees it. The first piece of work after the design pass tests the model, with its numbers and its quality measured [7].
- Nothing else of loops, tests, agent files or processes is approved [3].

## Proposed by the peer, not ruled

- **The second principle, reworded.** The owner's answer was "in a nutshell, sounds right-ish" (kept in `code-principles-page.md`, not in the rulings file), no clean yes to its words. The first two principles are reworded again in the held batch (`code-principles-page.md` at its end, `standard-batch-1-draft.md`).
- **Three words.** A mode is the form in which the Program reaches the runtime, data or code; this and the settled sentence [10] say one thing, since the form is fixed at the moment the Program meets the runtime. A runtime is what runs a Program the way SPEC §12 says: it holds the values, takes the writes, commits and reports the changes. A target is the machine, the SPEC's own word. The owner's answer: "now the picture is clearer" [12].
- **One runtime, not one a mode.** The owner leans to one "for starters" [10]. Both modes then share the commit, and only how a rule's value is computed differs. How far generated code specialises storage is measured at the thin slice, a first narrow path through the whole compiler that is built early to measure before budgets are set and is itself not yet designed. SPEC §12.6 gives a Program one memory figure for every target, so a second runtime could gain speed and not memory without a change of the SPEC.
- **Four contracts between the layers:**

| Between | Contract | Its meaning |
|---|---|---|
| A framework and the language | The vocabulary | SPEC §10 |
| The language and whatever runs or translates it | The Program, with its meaning | SPEC §11 and §12 |
| A mode and a runtime | The rule seam: "compute this rule's value" | New; it belongs with the modes' specification |
| A runtime and a framework | Writes, commits, changes | SPEC §12 and §13 |

  The runtime owns the values, the commit and the reports and knows nothing of modes. A mode puts something behind the seam: an interpreter that walks the Program, or generated Rust. The list is not ruled: the owner's "this." was to where the shape is written and not to the list [20]. It is settled with the design of the interfaces, and one place is held open in it: what the compiler hands a tool, faults and their positions.
- **Two things designer 2 holds open in that table,** told to designer 1 at the handover and to the owner only in designer 1's report. The seam's grain: the tenth entry fixes that the rules live in one place and that all three call them, and it rules out a generator that carries a rule of the commit; who calls whom at the seam, and how much one crossing covers, it does not say. "Compute this rule's value", the runtime calling the mode one rule at a time, is the first candidate and the one the thin slice builds and measures on the reference machine. The other form that stays inside the entry has generated code go through the cells in the Program's own order (SPEC §11) and call the shared code for every decision: whether a cell runs, and the settling of its value. Whether the owner's "agreed." admits that form is the owner's to say, with the interfaces and with the number. And native functions (SPEC §10.1): their meaning is in the first row, but they are also called while the Program runs, where there is no compiler, and they call a lambda's body back, so the first row's pair is too narrow or they need a row.
- **What is common to every mode** needs one home, since it is said once: what a mode is, that the Program is the only hand-off, that every mode gives the same results, that a mode names no runtime, and the rule seam's meaning. The peer's pick, told to the owner on 2026-11-16: one short specification page for that, beside one specification for each mode. A mode's specification states its form, what it needs from a runtime, what it offers (reload or not), its targets and its promises. All of them follow the SPEC's rule for changes.
- **WebAssembly:** Rust builds for it as a target, so the compiled and the interpreted mode can both reach it. A third mode is needed only if WebAssembly is generated without Rust.
- **The documents as they stand.** The cut is ruled [11] [13]; this list is the peer's summary of it: the SPEC; a specification for each mode; the code standard with the findings file; the architecture's documents, the interfaces first; a guideline document for the tester; the index.
- **How the architecture's documents hang together,** the peer's words beside the ruling [20]: they are cut where the contracts are; the top document can keep the name `ARCHITECTURE.md` that the SPEC cites; a part's document points to the top one and never into another part's inside. The reasons given: a contract is a wall, agents read whole documents, and change stays local.
- **What is ready to be written into its document;** the owner asked on 2026-11-16 [15]. Designer 1 named three pieces. Designer 2 drafted them and had them read cold twice, and they are less ready than they looked; `ready-pieces-draft.md` holds the drafts and what is open in each. A first version of the modes' specifications cannot be written from the rulings alone: the page on what every mode shares is not ruled, the SPEC's sentences on load and reload are not squared with the compiled mode, the promise that the modes agree bit for bit has no home, and whether main and secondary are specification at all is open. The line on who approves a change needs the owner's word on where, in the SPEC, a gap stops being the peer's, and it comes with a reworded line of the preferences file, which it would otherwise contradict. The sentence on what and how [14] is ready as it stands. The architecture's opening rules are the owner's own sentences and the peer's to write, with the outline. The peer drafts all of these: who drafts is how the work is organised, which the handover page leaves to the peer, and the owner approves their words.
- **A running Program has no life of its own,** designer 2's model, given to the owner on 2026-11-16 after [24] and not answered. From the SPEC: a Program advances only when the host commits, and time is a cell the host writes (§12.2); a commit is a run of steps (§12.2); memory is one figure known when the Program compiles (§12.6); evaluation never fails (§12.4). So the language is a synchronous one, of the family of Lustre and of a controller's scan cycle, in which everything asynchronous is kept at the boundary where inputs are taken in. The cell's word is that boundary's waiting place: a `state` keeps the latest value, a `stream` is a bounded buffer that forgets the oldest write and counts it, a `channel` one that refuses when full (§12.1). What designer 2 named as open: whether the cell's pending buffer is the hand-off between two execution contexts, a thread or an interrupt, without which the promise of §6.6 that a channel forgot no arrival stops at a queue the language cannot see; what "Writes to different cells keep their arrival order relative to one another" (§12.1) means when producers run at once, where designer 2 sees a race that nobody has checked; whether a `state` is written across contexts, its written marks (§13.2) with it. Proposed with it: the hand-off is an interface, with a plain implementation for one context and a lock-free one; the shipped part asks the machine for arithmetic and for memory once, loading taking one block of the size §12.6 gives, to be checked against that section; whether a product runs the interpreted mode is nothing the language grants or forbids. The last design's answers, from an extraction of `ARCHITECTURE.md` that the peer has not checked line by line: its §1 has the core `no_std` with `alloc`; its §4 one `Send` producer for each stream and channel over a single-producer ring, the runtime `Send` and not `Sync`, states written on the runtime's thread; its §8 the generator called from a framework's build script, and `Runtime::new(program(), &vocabulary)` in the compiled mode too.
- **Seven crossings where an interface is written,** a sketch designer 2 showed the owner on 2026-11-16, not for approval and not answered: a framework to the compiler, the vocabulary with native functions for calls at compile time (§10); the compiler to the interpreter and the generator, the Program (§11); the compiler to a tool, faults with positions and the memory and work figures (§12.6), which waits on §16; a mode and the runtime, the seam; a host to the runtime, load, handles, writes, commit and reload (§12.1, §12.2, §13); the runtime to a consumer, the values, each step's changes and the reload report (§12.3, §13.4); whatever computes a rule to a framework's native functions while the Program runs (§10.1). With it designer 2 proposed two things, and the owner answered neither: load exists in both modes and the same host code runs both; the generator is a library that a framework's own build calls. Both are proposals and no decision: the first is essence of the architecture, which is the owner's.
- **What the first piece measures:** rounds, tokens, tool calls and wall time for each agent and each check; for quality, the faults a fresh audit finds in the merged piece against the SPEC and the standard, with the check that should have caught each. Named before the piece starts.

## The core language, fifteen components

As the peer listed them to the owner on 2026-11-15, each from the SPEC. The owner did not object and did not rule.

1. Packages and files: which files make a build, in what order, what each package takes in and gives out (§2.1, §15).
2. Lexer: characters to tokens, layout, literals (§2.2 to §2.6).
3. Parser: tokens to a tree of every construct (the productions of §3 to §9).
4. Names and scope (§6.5).
5. Types and inference: the type of every value and cell, units, capacities, generics (§3, §4, §6.8).
6. Values and their operations: arithmetic, strings, arrays, maps, documents, conversions (§3, §5); the same at compile time and at run time.
7. Compile-time evaluation and its limits (§4.6).
8. Expansion: patterns, the placement `for`, inlining, paths (§6.3, §6.4, §7.9).
9. Cells and their order: steps, clocks, histories, which cells run together (§6.1, §6.6).
10. The vocabulary and its checks (§10).
11. Faults and messages (the §16 draft).
12. The Program (§11).
13. The runtime: load, the host's writes, the commit, memory fixed at load (§12).
14. Changes and consumers (§12.3).
15. Reload (§13).

Components 4 to 9 interleave: the SPEC checks types at each use and makes structure wait on values. Beside the core stand the interpreter, the generator, and the tool: the `sym` command with its fetching of packages, its watch loop and the language server.

## Where the conversation stopped

The owner's last ruling on the design is where a contract's exact shape is written and how the architecture's documents are cut [20]. Since then the owner approved the index and the handover page's reading line [23]. On 2026-11-16 designer 2 put three questions to the owner as options, and the owner answered that each needs proper reasoning and architecting [24]. Designer 2 withdrew them as posed and gave the model that stands under Proposed. Unanswered by the owner: that model; the list of the contracts; one runtime as a ruling; whether WebAssembly is a target of the compiled mode and no third mode; where, in the SPEC, a gap stops being the peer's; the page on what every mode shares; the sketch of seven crossings and the two proposals made with it; the seam's grain and where native functions stand among the contracts; the lesson P-16, a widening of lesson P-4 and a preference, whose words `lines-waiting-for-yes.md` holds or points to.

## Next, in the order the peer proposes

1. The three questions of [24], reasoned and architected before they are put: the hand-off between execution contexts, with the SPEC sentences it touches and the last design's scheme examined; what the shipped part asks of the machine; where the interpreted mode runs. Each is read cold before it is shown.
2. What of the agreed things goes into its document [15]: the drafts and what is open in each are in `ready-pieces-draft.md`, and the page on what every mode shares is the peer's pick and not ruled; the index gains its line when the owner rules on it [23].
3. An outline of the architecture: the interfaces first, then the fifteen components, the interpreter and the generator, on one runtime if the owner rules it [10], under the three rules. `ARCHITECTURE.md` is the last design and is replaced section by section; its §2, §4, §6 and §8 are the one statement so far of the vocabulary's declaration, the runtime's interface, the consumer and the generated mode, and are read as that and no more. With them go SPEC §10 to §13, where the contracts' meaning sits, and part 1 of `STANDARD.md`, whose lines the interfaces must meet.
4. What is left of the compile mode: how "the same results" is held, the speed figure after the thin slice, what a host gets beyond the shared interface in the compiled mode.
5. Faults, SPEC §16: about ten open edges, and the parser waits on them (`spec-16/spec-16-batch.md`). Designer 2 would take them beside item 3 and not after it, since the place held open in the contracts, what the compiler hands a tool, cannot be closed without them; said before reading the batch.
6. What the parser hands on: seven questions (`parser-page/item-2/proposal.md`).
7. The small language questions in `open-design-questions.md`.
8. The held batch of reworded lines, `standard-batch-1-draft.md`, at the owner's pace.

## Owed to the SPEC when its turn comes, each a change of essence for the owner

- Its first paragraph names `ARCHITECTURE.md` as the one document beside it; a modes' specification may need naming there.
- It cites `ARCHITECTURE.md` by section in seven sentences, which move as the architecture is rewritten. Some lean on it for what they mean, such as the names of the options in §15.2 (ARCHITECTURE §9) and the lowered expression of §13.5 (ARCHITECTURE §3), so whether a cited section of the last design can be relied on is open until they move.
- It speaks of a Program that a host loads as data and that a reload replaces: §1.1 "A host loads the Program into the runtime", §12.3 "Loading a Program evaluates every cell", §12.6 "Loading it allocates every buffer", §14 "A different structure is a different Program, loaded by reload (§13)". What load is in the compiled mode is not ruled, and these sentences need words that fit every mode once it is.
- §13 says what a reload is and never that every host offers one. Whether it needs a sentence is checked when the modes' specification says which mode offers reload.
- §12.5 may gain a sentence that results are the same however a Program is evaluated, once the compile mode is settled.
