# The design, where it stands

Checkpoint of 2026-11-16, written by the third peer at its handover. The peer keeps this page current as rulings land, until its content has moved into the documents themselves. It is the peer's summary and no ruling: the owner's words are in `owner-rulings-compiler-design.md`, cited here as [1] to [14] by entry. "Proposed" means the peer said it to the owner in the chat and the owner has not ruled on it.

## The pass

Two engines were built without a design for how code is written, and both came out poor. The owner stopped the build and opened this design pass on 2026-11-13: the compiler and the runtime are designed again from the SPEC, the rules for writing and reviewing code first, and are then built in small pieces. No compiler code is being written. The language itself is designed: SPEC §1 to §15.

## Settled by the owner

The documents:

- **What and how.** A specification is owed for what someone can observe or rely on, and it is said once. How a thing is built is architecture [14]. The test the peer gave with it: can a user or a framework tell the difference; if yes it is specification, if no it is architecture.
- **The SPEC** holds the language, the Program and what a Program means. It keeps §12 and §13, said once for every mode, and names no mode [11]. Only settled things go into it, cold-reviewed under heavy scrutiny first [2].
- **The modes get specifications of their own** [11]. A runtime gets none: §12 and §13 are the runtime's specification already [13].
- **The architecture** covers the architecture of what is designed before it [9], and says how all of it is built [11].
- **Who approves a change** [5] [8]:

| Change | The SPEC | The code standard and the architecture |
|---|---|---|
| New essence | the owner | the owner |
| Essence that derives cleanly and fully from what the owner approved | the owner | the peer |
| Mechanical, no essence changed | the peer | the peer |

  Every change is read cold first. In the SPEC the peer approves only where the approved sentences already fix the outcome: "A change is one only when the rest of the SPEC already leaves a single reading" [8]. An answer reached by analogy or by a principle is derived essence and the owner's. In doubt it is the owner's.

The modes and running:

- Generated Rust is the main mode and the interpreted mode a secondary convenience [1]. The interpreted mode is still held to the full bar: "efficient, optimized and well-planned and well-architected" [6].
- An edit runs in the interpreted mode with live reload. Generated Rust is what ships and what every speed promise is stated for, and it never reloads [6].
- A mode is when the Program meets the runtime. Interpreted: the Program arrives as data, at load; the runtime walks it and can take a new one, which is reload. Compiled: the Program is turned into Rust and built together with the runtime; replacing it needs a new build [10].
- One Program, two takers: the compiler ends at the Program in both modes, the interpreter loads it, the generator translates it, and neither reads the source [10].
- Every rule is written once. The operations on values and the commit's rules live in one place, and compile-time evaluation, the interpreter and the generated Rust all call them. Equality bit for bit holds by construction, and a test over every fixture guards the wiring [10].
- A device carries the runtime and the generated code, and no compiler [10]. Tests of running behaviour run in both modes [10].
- Whether a Program can travel to a runtime elsewhere: nothing is promised, and the Program is kept plain data so that it stays possible [10].
- Three rules hold whether or not a second runtime ever comes, because a contract is defined against an interface and the implementation is passed in [11]: the Program is the only hand-off; generated code meets the runtime through a narrow, stated seam; hosts and consumers are written against an interface and not against the runtime itself.
- A mode does not name a runtime. It names what it needs from one, a runtime states what it offers, and any mode runs on any runtime that offers it [13].

The work:

- The work model on the diagram, `workflow-as-shown-2026-11-15.md`, is approved for testing. The first piece of work after the design pass tests it, with its numbers and its quality measured [7].
- The second principle as reworded "sounds right-ish"; its words have no clean yes (`code-principles-page.md`, at its end).
- Nothing else of loops, tests, agent files or processes is approved [3].

## Proposed by the peer, not ruled

- **Three words.** A mode is the form in which the Program reaches the runtime, data or code. A runtime is what runs a Program the way SPEC §12 says: it holds the values, takes the writes, commits and reports the changes. A target is the machine, the SPEC's own word. The owner's answer: "now the picture is clearer" [12].
- **One runtime, not one a mode.** The owner leans to one "for starters" [10]. Both modes then share the commit, and only how a rule's value is computed differs. How far generated code specialises storage is measured at the thin slice. SPEC §12.6 gives a Program one memory figure for every target, so a second runtime could gain speed and not memory without a change of the SPEC.
- **Four contracts between the layers:**

| Between | Contract | Its meaning |
|---|---|---|
| A framework and the language | The vocabulary | SPEC §10 |
| The language and whatever runs or translates it | The Program, with its meaning | SPEC §11 and §12 |
| A mode and a runtime | The rule seam: "compute this rule's value" | New; it belongs with the modes' specification |
| A runtime and a framework | Writes, commits, changes | SPEC §12 and §13 |

  The runtime owns the values, the commit and the reports and knows nothing of modes. A mode puts something behind the seam: an interpreter that walks the Program, or generated Rust.
- **Where a contract's exact shape is defined:** in code, once, as a set of interfaces with no implementation inside, which every layer depends on and none owns. Its design is the first part of the architecture and goes to the owner as essence. The other way, a written interface specification beside the code, has every signature twice.
- **The modes' specification:** one document, a shared part and a part for each mode. A mode's part states its form, what it needs from a runtime, what it offers (reload or not), its targets and its promises. It follows the SPEC's rule for changes.
- **WebAssembly:** Rust builds for it as a target, so the compiled and the interpreted mode can both reach it. A third mode is needed only if WebAssembly is generated without Rust.
- **The documents as they stand:** the SPEC; the modes' specification; the code standard with the findings file; the architecture, the interfaces first.
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

Components 4 to 9 interleave: the SPEC checks types at each use and makes structure wait on values. Beside the core stand the interpreter, the generator, and the tool.

## Where the conversation stopped

The owner's last ruling was the distinction of what and how [14]. The last thing the peer answered was why the SPEC says what a runtime must do: a program's meaning is what it does over time; the language's promises are promises about running; one right answer is what lets runtimes be swapped and modes be held equal. Unanswered by the owner: the four contracts and where their shape is defined.

## Next, in the order the peer proposes

1. The owner's word on the four contracts and on where their shape lives.
2. An outline of the modes' specification for the owner: the shared part (what a mode is, the seam's meaning, that every mode gives the same results) and the two modes' parts.
3. An outline of the architecture: the interfaces first, then the fifteen components, the interpreter and the generator, on one runtime, under the three rules. `ARCHITECTURE.md` is the last design and is replaced section by section; its §2, §4, §6 and §8 are the one statement so far of the vocabulary's declaration, the runtime's interface, the consumer and the generated mode, and are read as that and no more.
4. What is left of the compile mode: how "the same results" is held, the speed figure after the thin slice, what a host gets beyond the shared interface in the compiled mode.
5. Faults, SPEC §16: about ten open edges, and the parser waits on them (`spec-16/spec-16-batch.md`).
6. What the parser hands on: seven questions (`parser-page/item-2/proposal.md`).
7. The small language questions in `open-design-questions.md`.
8. The held batch of reworded lines, `standard-batch-1-draft.md`, at the owner's pace.

## Owed to the SPEC when its turn comes, each a change of essence for the owner

- Its first paragraph names `ARCHITECTURE.md` as the one document beside it; a modes' specification may need naming there.
- It cites `ARCHITECTURE.md` by section in seven sentences, which move as the architecture is rewritten.
- §13 says what a reload is and never that every host offers one. Whether it needs a sentence is checked when the modes' specification says which mode offers reload.
- §12.5 may gain a sentence that results are the same however a Program is evaluated, once the compile mode is settled.
