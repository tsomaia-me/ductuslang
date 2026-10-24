# SymphonyScript

SymphonyScript is a language for describing graphs: music graphs (sessions, tracks, clips, notes) and audio graphs (oscillators, filters, mixers). A program places instances of node kinds, sets their attributes, connects them with wires, and binds attributes to inputs the host changes at runtime. Every kind comes from a vocabulary that a framework author writes in Rust; source code declares none.

The compiler is written in Rust. It compiles a project against one vocabulary into a Program: plain Rust data that any consumer interprets. A runtime evaluates the Program's reactive values, and synaptic-kernel is one consumer.

## Read first

1. This file.
2. `docs/SPEC.md` §1, then the sections your task touches. The SPEC is normative.
3. `docs/ARCHITECTURE.md`, the sections your task touches.
4. `docs/PLAN.md`: § How the work runs, and the current milestone.

Read a document by its sections, not whole, unless your task spans all of it.

## Scope

Everything this project needs is inside `packages/symphonyscript`. The kernel adapter also uses `packages/synaptic-kernel`; read its `CLAUDE.md`, `ARCHITECTURE.md` and `CONVENTIONS.md` before touching it. Implementers also read `.claude/docs/CODING.md` and reviewers `.claude/docs/REVIEWING.md` (`docs/PLAN.md` § How the work runs). Read nothing else in the repository for this project.

## Hard rules

- **The SPEC decides.** Code follows `docs/SPEC.md`. A gap or a contradiction in the documents is never resolved silently in code: it is decided as `docs/PLAN.md` § How the work runs says, the documents change first, and the code after.
- **An implementer makes no call on a gap.** It records every gap in its progress file, keeps working on what the gap does not block, and hands back to the head.
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
- Each fact appears in one place; other places refer to it by section number. Sections are numbered; individual rules are not, except the steps of an ordered procedure, such as a commit or reload matching.
- A rule comes first, then an example. A reason appears only where the rule would otherwise look arbitrary, in one clause.
- Each construct's grammar sits in its own SPEC section, and each production appears once.
- Tables serve where they read faster than prose: types, precedence, mappings.
