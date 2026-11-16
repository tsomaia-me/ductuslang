# The three pieces that are ready for their documents, a draft

A draft by designer 2, 2026-11-16. Nothing here is written to its file before the owner's yes: each piece is new essence in its document and is the owner's to approve. Every sentence is meant to say only what the owner ruled or agreed, in `owner-rulings-compiler-design.md`; the entry behind it is given in square brackets after each block, and the brackets are not part of the text. Left out on purpose, because none is ruled: the rule seam, the targets, the speed figures, and what a host is written against.

The file names are the peer's and are not ruled.

## Piece 1: the modes' specifications

### `packages/symphonyscript/docs/MODES.md`, a new file, whole

# SymphonyScript modes

This document defines what a mode is and what holds in every mode. It is normative. `SPEC.md` defines the language, the Program and what a Program means, and it names no mode. Each mode has a specification of its own, which says only what that mode adds: `MODE-INTERPRETED.md` and `MODE-COMPILED.md`.

## 1. What a mode is

A mode is when the Program (SPEC §11) meets the runtime.

- In the interpreted mode the Program arrives as data, at load.
- In the compiled mode the Program is turned into Rust and built together with the runtime.

## 2. What holds in every mode

- **The Program is the only hand-off.** Compilation ends at the Program in every mode. What takes the Program, to load it or to translate it, never reads the source.
- **The meaning is the SPEC's.** SPEC §12 holds in every mode, and SPEC §13 in every mode that offers reload. A mode's specification says whether it does.
- **Every mode gives the same results.** The same Program with the same sequence of writes and commits produces bit-identical values in every mode, as it does on every target (SPEC §12.5).
- **A mode names no runtime.** The same mode can run on different runtimes.

[§1: tenth. §2, in order: tenth and eleventh; eleventh; tenth and sixth; thirteenth.]

### `packages/symphonyscript/docs/MODE-INTERPRETED.md`, a new file, whole

# The interpreted mode

This document defines SymphonyScript's interpreted mode. It is normative, and it says only what this mode adds to `SPEC.md` and `MODES.md`.

## 1. The Program arrives as data

In the interpreted mode the Program arrives as data, at load.

## 2. Reload

The interpreted mode offers reload: a running Program is replaced by a new one, as SPEC §13 says.

## 3. What it is for

An edit runs in the interpreted mode, with live reload. It is the secondary mode, a convenience; what ships is the compiled mode.

[§1 and §2: tenth. §3: sixth and first.]

### `packages/symphonyscript/docs/MODE-COMPILED.md`, a new file, whole

# The compiled mode

This document defines SymphonyScript's compiled mode. It is normative, and it says only what this mode adds to `SPEC.md` and `MODES.md`.

## 1. The Program is built in

In the compiled mode the Program is turned into Rust and built together with the runtime.

## 2. No reload

The compiled mode never reloads. Replacing the Program needs a new build.

## 3. What it is for

The compiled mode is the main mode. It is what ships, and every speed promise is stated for it.

## 4. What a device carries

A device carries the runtime and the generated code, and no compiler.

[§1 and §2: tenth and sixth. §3: first and sixth. §4: tenth.]

## Piece 2: the architecture's opening rules

For the top document of the architecture. Where that document is, and whether it is a document of its own, waits for the owner's word on the architecture's documents.

## Rules that bind every part

1. **The Program is the only hand-off.** The compiler ends at the Program. Whatever runs a Program or translates it takes the Program and never reads the source.
2. **Generated code meets the runtime through a narrow, stated seam.**
3. **Hosts and consumers are written against an interface,** and not against the runtime itself.
4. **Every rule is written once.** The operations on values and the commit's rules live in one place, and compile-time evaluation, the interpreter and the generated Rust all call them. Equality bit for bit between the modes then holds by construction, and a test over every fixture guards the wiring.
5. **A device carries the runtime and the generated code, and no compiler.**
6. **Tests of running behaviour run in both modes.**

Rules 1 to 3 hold whether or not a second runtime ever comes, because a contract is defined against an interface and the implementation is passed in.

[1 to 3 and the closing sentence: eleventh, and tenth for the second sentence of 1. 4 to 6: tenth.]

## Piece 3: two lines for `.claude/OWNER_PREFERENCES.md`

Both are new lines; neither replaces one.

> Who approves a change of a document: in the SPEC, the owner approves every change of essence, derived or not, and the peer approves only a mechanical change, one that changes no essence, which it is only when the rest of the SPEC already leaves a single reading; in the code standard and the architecture, the owner approves new essence, and the peer approves or rejects a mechanical change and essence that derives cleanly and fully from what the owner approved; every change, whoever approves it, is read cold first by a fresh agent that wrote none of it.

> A specification is owed for what someone can observe or rely on, and it is said once; how a thing is built is architecture.

[The first: fifth, eighth and second. The second: fourteenth.]
