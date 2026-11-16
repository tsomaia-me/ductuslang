# The documents of SymphonyScript

Every peer and every agent starts from this page. It lists the documents by domain: what each is for, its standing, and who reads it. Everyone reads the SPEC whole; every other document is read when the work in hand needs it. A document that is not written yet has no file name here.

## The language

- [`SPEC.md`](SPEC.md): the language, the Program and what a Program means. The owner's, and normative.
- The compiler's architecture: how the compiler is built. Not written. For whoever works on the compiler.

## The modes

- Each mode, the interpreted and the compiled, gets a specification and an architecture of its own. None is written. Whoever works on one mode reads its two documents and need not read the other mode's.

## The runtime

- Its specification is §12 and §13 of [`SPEC.md`](SPEC.md).
- The runtime's architecture: how the runtime is built. Not written. For whoever works on the runtime.

## The whole

- The architecture's top document: the map of the parts, the contracts between them and the rules that bind every part. Not written. For whoever designs or builds any part.
- [`ARCHITECTURE.md`](ARCHITECTURE.md): the last design, which the architecture documents named on this page will replace. It is not the current design. The SPEC still cites its sections, and it is read for those.

## Writing, testing and reviewing code

- [`STANDARD.md`](STANDARD.md): the code standard, how code is written. A draft. For whoever writes or reviews code.
- The tester's guidelines: how code is tested and probed. Not written; until they are, their rules are in `STANDARD.md` and `FINDINGS.md`. For whoever tests.
- [`FINDINGS.md`](FINDINGS.md): what may be reported on a piece of code, and what is admitted. A draft. For whoever reviews or tests code.

## The work

- The work model: who does what on a piece of work, and the checks the piece passes. Approved for testing. Not written.
- The agent files: what each kind of agent reads at its start. None is approved.
