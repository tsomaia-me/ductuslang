# Peer notes

Short operational peer notes stating where things stand: decisions that do not yet stand in the final documents or draft files, what is in discussion, and what is yet to discuss. Every line is a plain statement. No history, no quoted words.

## Decisions

### Work model and scope
- The work model is approved for testing: a peer, a tester, a builder and a fresh reader, one machine gate, five checks a piece. The first piece of work after the design pass tests it, with its numbers and its quality measured.
- No loops file, agent file or process is approved. None is written, and no agent of the work model starts, without the owner's approval.
- How agents are organised is the peer's to design. The owner approves the files once, as a result, before anything runs.
- `packages/symphonyscript/docs/ARCHITECTURE.md` is the last design. It is being replaced and is not the design.
- `.claude/reference/` is a scratchpad for the design pass. It is cleaned and removed when the pass ends, and what lasts moves into the documents first.
- Ductus is separate work in this repository. The peer does not touch it.

### Modes and Program
- A mode is when the Program meets the runtime.
- A mode is an open, pluggable mechanism defined against common interfaces (`Program` on the compiler side and the `ModeInstance` seam on the runtime side), never a closed set hardcoded in the compiler or the runtime. Adding a new mode requires writing only that mode without touching the compiler or the runtime.
- The interpreted mode: the Program arrives as data, at load. The runtime walks it and can take a new one, which is reload.
- The compiled mode: the Program is turned into Rust and built together with the runtime. Replacing it needs a new build. WebAssembly (`wasm32`) is a compilation target of Rust, not a third mode.
- One Program, two takers. The compiler ends at the Program in both modes. The interpreter loads it and the generator translates it. Neither reads the source.
- The compiled mode is the main mode. The interpreted mode is the secondary one, a convenience, and it is built to the same bar: efficient, optimized, well planned and well architected.
- The edit loop runs in the interpreted mode, with live reload. Generated Rust is what ships and what every speed promise is stated for, and it never reloads.
- Both modes give the same results, bit for bit.
- A mode names no runtime, and a runtime names no mode. Both modes happen to share one runtime implementation to start with, interacting strictly through abstract interfaces rather than being hard-intertwined.
- Load exists in both modes: the runtime initializes host-fed cells to their inits, evaluates derived cells once in dependency order, computes initial activations and destinations, and initializes its arena buffers before the first commit. Once loaded, host code (taking handles/writers, writing, committing, consuming steps) is written against the same interfaces in both modes, with `reload` exposed only in the interpreted mode.
- Nothing is promised about a Program travelling to a runtime elsewhere. The Program is kept plain data, so that it stays possible.
- The SPEC names no mode. Its §12 and §13 hold for every mode. Each mode has a specification of its own, which says only what differs: when the Program meets the runtime, whether it offers reload, what a host is written against, and what it promises. No separate shared-modes page exists outside `SPEC.md` and the architecture's top document in `INDEX.md`.
- A runtime has no specification of its own. SPEC §12 and §13 are it.
- The seam between a mode and the runtime is `ModeInstance` (`ProgramMeta` + `Evaluator`): the runtime owns the entire commit state machine (SPEC §12.2 step ordering, dirty propagation across dependencies/clocks/cycles, bit-equality checks on `state`, history ring rotation, activation/destination updates, and §12.3 change emission) and calls the mode's `Evaluator` to compute a single cell's rule, gate, destination choice, or shared body.
- Concrete cycle and wall-clock speed budgets apply to the compiled mode and are set after building and measuring the early thin slice through the compiler and runtime.
- Beyond implementing `ModeInstance` for the shared `RuntimeHost` interface, the compiled mode's generated Rust module exports statically typed cell handles/writers (plus the program's record and enum types) and `const` instance, wire, and attribute IDs, turning type mismatches, unknown cell names, and over-capacity fixed arrays into Rust compile-time errors.

### Synchronous runtime and hand-off
- SymphonyScript is a synchronous language: a running Program is passive and advances only when the host commits, and time is a cell the host writes (SPEC §12.2).
- Each host-fed cell's staging buffer is the direct hand-off boundary between a producer context (thread or interrupt) and `commit`; no host queue stands in front of the runtime.
- Every host-fed cell (`state`, `stream`, `channel`) has at most one active writer at a time: a single-context host writes any cell word via `rt.write(handle, ...)` / `rt.write_at(...)`, and a multi-context host takes a unique `Send` `Writer` via `rt.writer(handle)` for any cell word.
- A cell's single writer owns its working copy (`newest_value` and SPEC §13.2 `written` marks), validating and applying whole and addressed writes (SPEC §12.1) locally with no read-back from the commit thread:
  - `state` hands off via a wait-free 3-slot triple buffer (atomic slot swap + dirty bit) and steps together in Step 1 of `commit`.
  - `stream[N]` hands off via a wait-free slot-index ring over `N` pending slots plus 1 private producer slot, popping the oldest pending slot index on overflow and incrementing `dropped` so the producer never touches the slot `commit` is reading.
  - `channel[N]` hands off via a wait-free `N`-slot SPSC ring that rejects immediately when full.
- For cross-cell arrival order (SPEC §12.1, §12.2), a `stream` or `channel` write validates and fills its private slot first, then takes its sequence number and publishes the slot index; `commit` takes a sequence cutoff before scanning cells and steps only published writes below that cutoff in sequence order, leaving post-cutoff writes in their rings for the next commit.
- Behind the hand-off interface, the single-context implementation uses plain counters/indices and the multi-context implementation uses atomics, with identical semantics under SPEC §12.5.
- Both modes share one wait-free runtime design with zero lock or epoch indirection on `write()`. `Handle` (the cell-name token) stays valid across reloads (SPEC §13.2), while `Writer` binds directly to the active Program's cell buffer; `reload()` (interpreted mode only, between commits) requires cross-thread `Writer`s to be quiesced/returned and re-taken after `reload()`.

### Layer contracts and shared foundation
- Beneath the compiler, the modes, and the runtime sits `ops`, a pure `no_std`, zero-allocation value and math kernel implementing SPEC §5.2 saturation and float arithmetic, §12.5 canonical NaN and `rand`, §5.7 `math` functions, §12.2 bit-equality, and §12.1 write conversions. Compile-time evaluation, every mode's evaluator, and the runtime call `ops` so bit-for-bit equality holds by construction.
- Seven contracts cross between the layers, with their meanings in the specifications and their shapes written as pure data or trait interfaces with no implementation inside:
  1. Framework to Compiler (`Vocabulary`, SPEC §10.1): plain `const`/`static` data structs (`Vocabulary`, `NodeKindDef`, `WireKindDef`, `AttrDef`, `EnumDef`, `UnitFamilyDef`, `PreludeDef`, `NativeSig`) with `const fn` constructors/helpers on item structs and no separate builder type. Separated from callable native functions so compiled device binaries never link vocabulary schemas or compiler types.
  2. Evaluator (compile-time and runtime) to Framework (`NativeFunctions`, SPEC §10.1): callable trait for pure, total, allocation-free Rust functions writing into a caller-supplied `ValueSlot` and invoking passed lambdas via a `LambdaCaller` callback context.
  3. Compiler to any Mode or Tool (`Program`, SPEC §11): plain, target-independent data holding vocabulary identity, canonical memory and max-work figures (§12.6), host-fed cells, dependency-ordered derived cells with clocks/cycle groups/lowered rules, shared bodies, instances, and wires.
  4. Tool/Driver to and from Compiler (`SourceProvider` and `CompilationOutput`, SPEC §4.6, §12.6, §15, §16): resolved package files and `.sym` sources in (keeping filesystem and `git` I/O outside the pure compiler core); `Option<Program>`, structured faults/warnings with spans, and memory/work figures out.
  5. Mode and Runtime (`ModeInstance`: `ProgramMeta` + `Evaluator`): structural cell/instance/wire metadata without ASTs (`ProgramMeta`) plus rule/gate/choice evaluation into a runtime-supplied slot (`Evaluator`) reading current/history values, `stepped`, and `dropped` from `EvalContext`.
  6. Host to Runtime (`RuntimeHost`, `Writer`, and `Reloadable`, SPEC §12.1, §12.2, §13.1, §13.2): `load`, `handle`, single-context `write`/`write_at`, `writer` (returning a `Send` `Writer` for any cell word), `commit`/`commit_steps`, and extension trait `Reloadable` for modes supporting live reload.
  7. Runtime to Consumer (`Consumer`, `Step`, and `ReloadReport`, SPEC §12.2, §12.3, §13.4): per-step change records in §12.3 order with read access to end-of-step values, plus reload diff reports.

## In discussion

- **The outline of the architecture documents (`INDEX.md`):**
  1. The architecture's top document (map of parts, the seven contracts, and the cross-cutting rules).
  2. The runtime's architecture (arena layout, wait-free hand-off rings/triple-buffers, commit/dirty engine, consumer delivery, reload).
  3. The interpreted mode's specification and architecture, and the compiled mode's specification and architecture.
  4. The compiler's architecture (packages and files, lexer, parser, names and scope, types and inference, values and `ops`, compile-time evaluation, expansion, cells and dependency/cycle order, vocabulary checks, faults, the Program).

## Yet to discuss

- Faults, SPEC §16: a draft and about ten open edges, in `.claude/reference/spec-16/`. The parser waits on them.
- What the parser hands on: seven questions, in `.claude/reference/parser-page/item-2/proposal.md`.
- Small language questions: `.claude/reference/open-design-questions.md`.
- The code standard, `STANDARD.md` with `FINDINGS.md`: a draft. A batch of reworded lines is held in `.claude/reference/standard-batch-1-draft.md`. Its line 3 binds every crate while its 1.3 and 4.8 fit only the compiler's stages. Three rules of the runtime have no home: a commit path that never allocates, lock-free hand-off between threads, integer time (`.claude/docs/CODING.md`).
- What the SPEC needs once the modes and the architecture are final: its first paragraph names `ARCHITECTURE.md`; seven sentences cite its sections, and some lean on them (§13.5, §15.2); §12.5 and the agreement of the modes; §13 and which mode offers reload.
- The first piece of work that tests the work model. Proposed: the number reader of SPEC §2.6 in a throwaway crate. What it measures is named before it starts.

## Where things are

- The record of the owner's words, kept word for word and added to as they land, is `.claude/reference/owner-rulings-compiler-design.md`, with two older files beside it, `owner-rulings-code-standard.md` and `owner-rulings-parser-page.md`. It is a record to check a statement against, not reading for a start.
- The work model's diagram: `.claude/reference/workflow-as-shown-2026-11-15.md`. The code standard's principles in plain words: `.claude/reference/code-principles-page.md`.
- What `rustc` costs on generated code, a stand-in measurement: `.claude/reference/compile-mode/rustc-probe.md`.
- The old code, a source of cases and no yardstick: branch `symphonyscript/r3`, on `wb` at `/home/cdagent/cargo-targets/r3/wt/`.
