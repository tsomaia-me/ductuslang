Drafted by the peer on 2026-11-15 for the work model the owner approved for testing. Not cold-read, not shown to the owner, nothing of it written to its file. Held: its wording names the SPEC, the principles and the architecture as the three documents, and the owner's discussion of where the modes are defined (the ninth entry of `owner-rulings-compiler-design.md`) decides which documents there are. The peer's sorting, by the eighth ruling: A1, A2 and D need the owner's yes, B10 holds one choice for the owner (who approves error messages), and the rest derives from what the owner approved and is the peer's to approve after a cold read.

# Proposed changes of text, batch 1

Each item gives the file, the current text (OLD) and the proposed text (NEW). Nothing here is written to its file yet.

## A. `.claude/reference/code-principles-page.md` (the thirteen principles in plain words, addressed to the owner as "you")

### A1. Principle 1
OLD:
1. **The SPEC decides what the code does.** Code is written from the SPEC's sentence, never from an example, a test or older code. Where the SPEC is silent or says two things, the gap is written down with the sentences quoted and brought to you; work goes on with what it does not block. Refuses: guessing what was meant.

NEW:
1. **The SPEC decides what the code does.** Code is written from the SPEC's sentence, never from an example, a test or older code. Where the SPEC is silent or says two things, the gap is written down with the sentences quoted and goes to the peer. Where the approved sentences already fix the answer, the peer states it in the SPEC, after a cold review. Any other answer is a change of the language's design, however cleanly it follows from the rest, and comes to you. Work goes on with what the gap does not block. Refuses: guessing what was meant.

### A2. Principle 2
OLD:
2. **Design before code.** The design page sets the edges of a piece of work. The builder's plan of its types and passes is reviewed and accepted before any function body is written. Refuses: structure that grows by accident.

NEW:
2. **Design before code.** The SPEC, these principles and the architecture are designed with you before the work starts. The architecture names the parts of the compiler and the runtime and what each takes and passes on. Every piece of work is then designed inside them, without you. The peer writes the piece's design page, and the builder writes its plan of types and passes. Each is read by a reviewer who did not write it, and the peer accepts it before any function body is written. A question about how to build reaches you only when the SPEC, the architecture, these principles and the decisions already written down leave it with more than one valid answer or with no clear winner. The peer decides every other such question and writes the decision down with its reason. Refuses: structure that grows by accident, and work that stops to ask you what is already answered.

### A3. The line under the thirteen
OLD:
**When these do not settle a choice:** the SPEC or a question to you first, then these principles, then cost, then the form easiest to check against its SPEC sentence. Two principles that cannot both be met come to you.

NEW:
**When these do not settle a choice:** the SPEC first, a gap in it handled as the first principle says; then these principles; then cost; then the form easiest to check against its SPEC sentence. Two principles that cannot both be met come to you.

### A4. A new paragraph after it
NEW:
**Who changes the SPEC, these principles and the architecture.** A change of essence is yours. In the SPEC that holds even when the change follows cleanly from what you approved. In these principles and in the architecture, the peer approves a change that follows cleanly and fully from what you approved. A mechanical change, one that leaves the essence as it is, is the peer's in all three. Every change is read cold before it is applied.

## B. `packages/symphonyscript/docs/STANDARD.md`

### B1. Words, the row "page" (only its first sentence changes)
OLD: | page | The unit's design page, approved by the owner. It names what the unit builds and the SPEC sections it serves; [rest unchanged] |
NEW: | page | The unit's design page. The peer writes it from the SPEC, the architecture document and this standard, a reviewer who did not write it reads it cold, and the peer accepts it. It names what the unit builds and the SPEC sections it serves; [rest unchanged] |

### B2. Words, the row "peer"
OLD: | peer | The session that writes pages with the owner. |
NEW: | peer | The session that leads the work. It writes and accepts the pages, accepts a unit's structure proposal and its stated tests, and answers what the SPEC, the architecture document and this standard settle. It brings the owner only what 3.4 and 3.5's last paragraph name. |

### B3. Words, a new row after "page"
NEW: | architecture document | `ARCHITECTURE.md`: the design of the whole, approved by the owner before the work starts. It names the parts of the compiler and the runtime and what each takes and hands on. A page works inside it. It is not part 1 of this standard, which holds the principles of the code's architecture. |

### B4. Words, the row "stated test"
OLD: | stated test | A test that names the SPEC sentence it proves. The test author, who is never the builder, writes it before the build, and it is frozen. One stated test is one case file (4.8). |
NEW: | stated test | A test that names the SPEC sentence it proves. The test author, who is never the builder, writes it. A unit's first tests are written before the build, read cold with the structure proposal (FINDINGS 5.7), and frozen when the peer accepts them. One stated test is one case file (4.8). |

### B5. 1.7, two sentences
OLD: It is read cold against parts 1 to 3 before any body is written. The builder builds to the accepted proposal.
NEW: It is read cold against the page and parts 1 to 3 before any body is written (FINDINGS 5.7), and the peer accepts it. The builder builds to the accepted proposal.

### B6. 1.10, three sentences
OLD: A builder never adds a dependency: it proposes one, with the job it does. The page lists the approved ones. Each was checked, before the page was approved, to exist in that form and version, and one on a path that reaches a value was checked to give the same result on every target and in every run.
NEW: A builder never adds a dependency: it proposes one to the peer, with the job it does. The page lists the ones the peer approved. Each was checked, before the page was accepted, to exist in that form and version, and one on a path that reaches a value was checked to give the same result on every target and in every run.

### B7. 3.4
OLD: **3.4 The SPEC is never completed in code.** Where the SPEC is silent or contradicts itself, the builder writes a GAP that quotes the sentences. The answer lands as a SPEC sentence or a stated test. The round is counted apart.
NEW: **3.4 The SPEC is never completed in code.** Where the SPEC is silent or contradicts itself, the builder writes a GAP that quotes the sentences, and it goes to the peer. The peer answers it where the SPEC's approved sentences already fix the answer, and takes every other one to the owner. The answer lands as a SPEC sentence or a stated test. The round is counted apart.

### B8. 3.5, its last line
OLD: Two lines of the standard that cannot both be met go to the owner through the peer.
NEW: Beyond a GAP (3.4), a question goes to the owner, through the peer, only when the SPEC, the architecture document, this standard and the decisions already written down leave it with more than one valid answer or with no clear winner. Two lines of the standard that cannot both be met are such a question. The peer decides every other one and writes the decision, with its reason, into the document it settles: the unit's page, or, where it binds more than one unit, the architecture document or this standard.

### B9. 3.6, one sentence added at its end
OLD (the paragraph's end): An accepted proposal found wrong goes the same way.
NEW: An accepted proposal found wrong goes the same way. The peer decides an OBJECTION, by 3.5's last paragraph.

### B10. 3.8
OLD: **3.8 Messages.** There is one catalogue. A page proposes entries, the owner approves them with the page, and a builder writes none. A stated test compares a message's code and its place, never its words.
NEW: **3.8 Messages.** There is one catalogue. Its style, with a handful of model messages, is approved by the owner once. A page proposes entries in that style, the peer accepts them with the page, and a builder writes none. A stated test compares a message's code and its place, never its words.

## C. `packages/symphonyscript/docs/FINDINGS.md`

### C1. The opening and its table
OLD:
This document says what may be reported on a unit of the SymphonyScript compiler, and what is admitted. It has four readers:

| Reader | What it does |
|---|---|
| the reviewer of a diff | Reads a unit's code cold, on READY. |
| the reviewer of a structure proposal | Reads a proposal cold, before any body is written (5.7). |
| the prober | Writes programs around the unit's rules and runs them. |
| the verifier | Admits or rejects each finding before a builder sees it (5.4). |

NEW:
This document says what may be reported on a unit of the SymphonyScript compiler, and what is admitted. Its readers:

| Reader | What it does |
|---|---|
| the reviewer of a page | Reads a unit's page cold against the SPEC, the architecture document and the standard, before the peer accepts it. |
| the reviewer of a structure proposal | Reads a proposal and the unit's stated tests cold, before any body is written (5.7). |
| the reviewer of a diff | Reads a unit's code cold, on READY. |
| the test author, on the built unit | Writes programs around the unit's rules and runs them (5.5). |
| the verifier | Repeats each finding, and admits or rejects it, before a builder sees it (5.4). |

A reviewer and a verifier are each a fresh agent that wrote nothing of what it reads.

### C2. 5.5, its title and first words
OLD: **5.5 The prober's check of growth.** The prober checks growth on its own, with no counter: by time ratio at four doubling sizes, [rest unchanged]
NEW: **5.5 The test author's check of growth.** On the built unit the test author checks growth on its own, with no counter: by time ratio at four doubling sizes, [rest unchanged]

### C3. 5.7
OLD: **5.7 The review of a structure proposal** has no code and nothing to run. Its reviewer writes, from the proposal alone, what the unit will build: the passes in order, and each lasting type with its writer and its layout. It then checks the proposal against the page: what crosses the edge, the shared tables, the listed inputs. It checks it against the lines of parts 1 to 3 and the counts of 2.2. It checks the builder's breakdown against the test author's rule list. A finding there names the proposal's line and quotes what it breaks: the page's line, the standard's line or the SPEC sentence; or it names the form or the condition that one list holds and the other does not. The verifier admits it by reading.
NEW: **5.7 The review of a structure proposal** has no code and nothing to run. Its reviewer writes, from the proposal alone, what the unit will build: the passes in order, and each lasting type with its writer and its layout. It then checks the proposal against the page: what crosses the edge, the shared tables, the listed inputs. It checks it against the lines of parts 1 to 3 and the counts of 2.2. It checks the builder's breakdown against the test author's rule list. It works out each stated test's expected result from the sentence the test names, before it reads the stated one. A finding there names the proposal's line and quotes what it breaks: the page's line, the standard's line or the SPEC sentence; or it names the form or the condition that one list holds and the other does not; or it names the test whose expected result its sentence does not give. The verifier admits it by reading.

## D. `.claude/OWNER_PREFERENCES.md`, its last line (one line becomes two)

OLD:
- Text proposed for a file — a lesson, a preference, a SPEC sentence, a skill rule, or an entry for the message catalogue among it — is shown to the owner in the chat in a quote block of its own, with the exact line it replaces when it refines or widens one and the exact line itself when it removes one, never a bare ID or line number, and is written only after the owner approves it.

NEW:
- A lesson, a preference, a skill line, a handover line, and any change of the SPEC, the code standard or the architecture that is the owner's to approve, is shown to the owner in the chat in a quote block of its own, with the exact line it replaces when it refines or widens one and the exact line itself when it removes one, never a bare ID or line number, and is written only after the owner approves it.
- Who approves a change of the SPEC, the code standard and the architecture: a change of essence is the owner's, and in the SPEC that holds even when it derives cleanly from what is approved; in the code standard and the architecture the peer approves essence that derives cleanly and fully from what the owner approved; a mechanical change, one that leaves the essence as it is, is the peer's in all three; every change is cold-reviewed before it is applied.
