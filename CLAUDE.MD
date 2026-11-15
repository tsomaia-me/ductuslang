# SymphonyScript Repository Rules

SymphonyScript is a language for describing graphs that run in real time: nodes placed inside other nodes, wires between nodes, and reactive cells whose values change while the graph runs, within memory fixed at compile time and work bounded at every step. Every node kind, wire kind, enum, and unit comes from a vocabulary written in Rust by a framework author; source code declares none. The Rust compiler compiles a package against one vocabulary into a `Program`, and the Rust runtime evaluates it deterministically.

This file loads into every session and custom subagent. There is no user-level `~/.claude/CLAUDE.md` on the runner; treat this file and the documents it cites as self-contained.

## 1. Posture & Spine

- **Quality is the invariant; speed is only a tool.** Never trade correctness, spec fidelity, or clarity to finish faster or avoid friction.
- **Check claims at the source.** Read the governing sentence in `SPEC.md`, `ARCHITECTURE.md`, `STANDARD.md`, `FINDINGS.md`, the unit's design `page`, or the code before stating a fact or acting on one. Mark anything unread or unrun as unverified.
- **Act only on definitive instruction.** In an interactive session with the owner, take mutating action on code, files, git branches, or external systems only when the owner has made it definitive to do so; agreeing with a plan is not an instruction to execute it.
- **A question is a question; pushback is a claim to check.** `"Why?"`, `"Are you sure?"`, and `"You failed"` are claims, never orders to change course. Check them against the facts first, report what holds, and change nothing merely to end friction.
- **Spine.** Hold a position for its factual reasons; drop it when those reasons fail, and never before. State an objection, risk, or design flaw once with the concrete mechanism and your pick. Once the owner rules after hearing it, execute completely and reopen only on a new fact. Own mistakes in one sentence and continue — no praise, fluff, or repeated apologies.

## 2. Workspace Scope & Order of Truth

- **Active workspace.** Only `packages/symphonyscript` (and `.claude/`) is active work.
  - `packages/ductus-lang` is obsolete: never read, search, or edit it.
  - `packages/synaptic-kernel` is complete and frozen: read its `CLAUDE.md`, `ARCHITECTURE.md`, and `CONVENTIONS.md` only when working on `symphonyscript-kernel`, and modify `synaptic-kernel` only with explicit owner approval while preserving its invariants.
  - Read nothing under `.memory/` or legacy `.claude/docs/CODING.md` and `.claude/docs/REVIEWING.md`.
- **Order of truth.**
  1. `packages/symphonyscript/docs/SPEC.md` — normative language and runtime meaning.
  2. `packages/symphonyscript/docs/ARCHITECTURE.md` — crate boundaries, data structures, and interfaces. (Note: §8 generated-Rust delivery is an unbuilt, unruled seam regarding whether code generation reads `Program` or compiler stages/sources; preserve differential parity between the §4 evaluator and §8 generated Rust across any change touching `Program`, lowering, or evaluation.)
  3. The owner-approved unit design `page` and its `accepted structure proposal`.
  4. `packages/symphonyscript/docs/STANDARD.md` and `packages/symphonyscript/docs/FINDINGS.md` — sole bar for how code is written, tested, reviewed, probed, and verified.
  5. Frozen `stated tests` — subject to `DISPUTE` (`STANDARD.md` §3.10) if a case contradicts `SPEC.md`.
- **No existing code is a model.** Code comes from the `SPEC.md` sentence and `STANDARD.md` alone (`STANDARD.md` §3.1); kept code may hold tracked non-conformances (`STANDARD.md` §3.7) and never legitimizes a pattern or a spec deviation.
- **No silent semantics.** Never resolve a `SPEC.md` gap or contradiction in code. Workers emit `GAP`, `DISPUTE`, or `OBJECTION` (`STANDARD.md` §3.10) and continue unblocked work; orchestrators bring language semantics, product, architecture, dependency (`STANDARD.md` §1.10), diagnostic catalogue (`STANDARD.md` §3.8), `unsafe`, and irreversible choices to the owner with concrete options, SymphonyScript examples where applicable, costs, and a recommended pick.

## 3. Reading Discipline & Hard Boundaries

- **Read whole what governs your task, and nothing more.** Every file read stays in context on later turns. Follow each role file's reading list; when reading by section, read `SPEC.md` §1 plus every touched and cross-referenced section whole rather than skimming fragments.
- **Crate and safety invariants (`packages/symphonyscript`):**
  - `#![forbid(unsafe_code)]` across every crate by default; `unsafe` requires explicit owner approval inside an isolated module with written safety invariants and verification.
  - No unauthorized dependencies (`STANDARD.md` §1.10).
  - `symphonyscript-core` builds `#![no_std]` using only `alloc` and `libm`. `commit` and reload `install` never allocate (allocation occurs only in `Runtime::new` and reload `prepare`). Runtime evaluation is total and panic-free, returning errors only on rejected host calls (`SPEC.md` §12.1, §12.4, §13.1). `producer` thread hand-off is lock-free and wait-free (`Release`/`Acquire` SPSC, never blocking the commit path), and step/tick tracking uses integers converted to float units only at boundaries.
  - `symphonyscript` (compiler) performs no file, environment, or clock I/O (`STANDARD.md` §1.3), recurses on no source- or option-sized depth (`STANDARD.md` §2.4), and uses `libm` for compile-time float operations so compile-time and every target's runtime stay bit-identical (`SPEC.md` §12.5).
  - Every `sym` block in `SPEC.md` parses and every `sym-error` block is rejected, enforced against `docs/pending-examples.txt` and `docs/pending-productions.txt` (`SPEC.md` §1.2).
- **Surgical scope.** Touch only files within your assigned scope. Never edit `SPEC.md`, `ARCHITECTURE.md`, `STANDARD.md`, `FINDINGS.md`, `docs/pending-*.txt`, frozen `stated tests`, or `growth`/`clock` rows unless your brief explicitly assigns them.

## 4. Unit Loop Invariants

1. **Parallel pre-body start:** From an owner-approved unit `page`, `test-author` (building the `rule list` and `stated tests` on a `tests/` branch) and `builder` Phase 1 (writing the `structure proposal` and `breakdown` on a `unit/` branch, `STANDARD.md` §1.7, §3.1) run in parallel.
2. **Pre-body gates & test isolation:** `test-reviewer` checks the stated tests cold before freeze; `reviewer` audits the `structure proposal` and checks the `breakdown` against the `rule list` (`FINDINGS.md` §5.7). **Frozen stated tests reach the `builder`'s branch only after its `structure proposal` and `breakdown` are accepted.**
3. **Post-READY gates:** On `READY`, `reviewer` (`FINDINGS.md` §5.1–5.3) and `prober` (`FINDINGS.md` §5.5) audit the unit cold; `verifier` (`FINDINGS.md` §5.4) gates every finding (`ADMIT` / `REJECT` / `ROUTE`) before any finding reaches `builder`; any admitted wrong-answer finding lacking a stated case goes to `test-author` before `builder` fixes it (`STANDARD.md` §4.3).

## 5. Host, Cargo & Git Hygiene

- **Secrets:** Never read `.env*` or key files, and never print secrets or tokens.
- **Git & worktrees:** Work in the worktree your brief names using `git -C <path>` and absolute paths. Remove only worktrees you created. Stage files by explicit path (`git add -- <path>`); never run `git add -A`, `git add .`, `git commit -a`, `git stash`, `git reset`, rebase, or force-push.
- **Builds & runs on the runner:** Call cargo by its full path (`/home/cdagent/.cargo/bin/cargo` on the Linux runner; `cargo` locally if absent), under `nice -n 19` with `-j 6` on shared hosts (avoiding heavy shared-host runs 18:00Z–02:00Z). Use an isolated target directory per task (`CARGO_PROFILE_DEV_OPT_LEVEL=0` for fast edit/probe builds; workspace `dev` profile as written for timing runs and 1 MiB stack checks). Delete your task's scratch folder and temporary target directory before handing back.
- **Process & check discipline:** Judge every command by its own exit code with nothing chained after it. Confirm filtered test runs executed the intended non-zero test count. Run long checks in the background with no outer timeout and wait on completion notice or PID. Kill processes by PID only, never `pkill -f`. Compare wall-clock targets only on the reference machine `wb2`; growth ratios across doubling sizes on one quiet host are comparable, raw times across hosts are not.
- **Subagent reports:** Subagents return their report as plain markdown text in their final message and write no report files to disk.

## 6. Writing Reference Documentation

- Write in definitive present tense ("A wire's destination is a named instance") — no history, plans, `"will"`, `"for now"`, `"later"`, `"note that"`, or hedges.
- Describe SymphonyScript alone; compare it to no other language.
- Keep language documents domain-neutral: state that SymphonyScript describes real-time graphs within fixed memory and bounded work (never "general purpose" and never bound to one domain), using examples across multiple domains.
- State each fact in one place and cite its section number elsewhere. Put the rule first, then the example, and state a reason in one clause only where the rule would otherwise look arbitrary. Keep each construct's EBNF production once, in its own `SPEC.md` section.
