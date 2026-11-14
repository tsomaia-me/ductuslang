# SymphonyScript

SymphonyScript is a language for describing graphs that run in real time: nodes placed inside other nodes, wires between nodes, and values that change while the graph runs, within memory fixed when the program compiles and work bounded at every step. A program places instances of node kinds, sets their attributes, connects them with wires, and binds attributes to cells, values the host or the program changes at runtime. What the graph describes, a lighting rig, a control loop or an arrangement of clips, is the vocabulary's business: every kind comes from a vocabulary that a framework author writes in Rust, and source code declares none.

The compiler is written in Rust. It compiles a project against one vocabulary into a Program: plain Rust data that any consumer interprets. A runtime evaluates the Program's reactive values, and synaptic-kernel is one consumer.

## Read first

1. This file.
2. `docs/SPEC.md` §1, then the sections your task touches. The SPEC is normative.
3. `docs/ARCHITECTURE.md`, the sections your task touches.
4. `docs/STANDARD.md`, whole, before you write or review this package's code. Whoever reports on that code also reads `docs/FINDINGS.md`.

Read the SPEC and ARCHITECTURE by their sections, not whole, unless your task spans all of one.

## Scope

Everything this project needs is inside `packages/symphonyscript`. The kernel adapter also uses `packages/synaptic-kernel`; read its `CLAUDE.md`, `ARCHITECTURE.md` and `CONVENTIONS.md` before touching it. This package's code, in every crate, follows `docs/STANDARD.md` and no other rulebook, and no existing code is a model for it. In `symphonyscript-core` the runtime's rules that the first paragraph of `.claude/docs/CODING.md` names stay in force beside it. A reviewer reads `docs/SPEC.md` and `docs/ARCHITECTURE.md` whole. Read nothing else in the repository for this project.

## Hard rules

- **The SPEC decides.** Code follows `docs/SPEC.md`. A gap or a contradiction in the SPEC is never resolved silently in code: its answer lands as a SPEC sentence or a stated test (`docs/STANDARD.md` 3.4), and the code follows it.
- **A builder makes no call on a gap.** It writes a GAP that quotes the sentences, and keeps working on what the gap does not block.
- **Language design belongs to the owner.** A design question states the background, the problem, the options with SymphonyScript examples, and a recommendation with its reason.
- **`symphonyscript-core` builds without `std`.** It uses `alloc` and `libm` and nothing else.
- **`commit` never allocates.** Allocation happens in `Runtime::new` and `reload` only.
- **Evaluation never fails.** No runtime path panics; the runtime returns errors only for rejected host calls.
- **Results are bit-identical on every target.** Float functions come from `libm` everywhere.
- **The compiler performs no file I/O.** Callers pass every source.
- **Every `sym` example in the SPEC parses, and every `sym-error` example is rejected.** A test enforces both.
- **The kernel changes only with the owner's approval,** and every change preserves the kernel's invariants.

## Writing documentation

- Present tense and definitive: "A wire's destination is a named instance." No history, no plans inside reference documents, no "will", "for now", "later", "note that", and no hedges.
- The documents describe SymphonyScript alone. They compare it to no other language.
- The documents name no domain as the language's own: a domain's words appear only in examples, and the examples come from several domains, so no reader takes SymphonyScript for a language of one domain; where a document says what the language is for, it says real-time use within fixed memory and bounded work, never general purpose.
- Each fact appears in one place; other places refer to it by section number. Sections are numbered; individual rules are not, except the steps of an ordered procedure, such as a commit or reload matching.
- A rule comes first, then an example. A reason appears only where the rule would otherwise look arbitrary, in one clause.
- Each construct's grammar sits in its own SPEC section, and each production appears once.
- Tables serve where they read faster than prose: types, precedence, mappings.
