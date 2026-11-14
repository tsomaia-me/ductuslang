# Findings

This document says what may be reported on a unit of the SymphonyScript compiler, and what is admitted. It has four readers:

| Reader | What it does |
|---|---|
| the reviewer of a diff | Reads a unit's code cold, on READY. |
| the reviewer of a structure proposal | Reads a proposal cold, before any body is written (5.7). |
| the prober | Writes programs around the unit's rules and runs them. |
| the verifier | Admits or rejects each finding before a builder sees it (5.4). |

Each reads `STANDARD.md` first: a finding cites its lines by number, and its words are used here as it defines them. A finding that is not admitted does not reach the builder.

## 5. What a reviewer may report

**5.1 The account comes first.** Before its findings by reading or by reasoning count, the reviewer writes from the code alone how the unit works: what comes in, each pass in order, each lasting type with its writer, each rule's home by its mark. A finding by reading or by reasoning that lies outside that account, or against it, is rejected. A finding known by a run, an input that reproduces a wrong answer, a panic or a growth, counts whatever the account says. A difference between the account and the accepted proposal is a finding, or the account is withdrawn.

The reviewer reads a builder's tests as code and takes nothing from them as evidence (4.5).

**5.2 A finding's form.** Four parts:

1. its place;
2. its source, quoted: a SPEC sentence, an invariant of the page or a line of the standard;
3. its consequence;
4. how it is known: by a run, by reading or by reasoning.

**5.3 The classes.** A finding is one of these.

| Class | What it owes |
|---|---|
| A wrong answer | An input and the sentence. |
| A panic or an internal error on an input | The input. |
| An answer that depends on order: of top-level items, of a hash map, of runs | The two runs. |
| A growth fault (2.1) | The ratio or the counter. |
| Slow by a constant factor: a count of 2.2 broken, a fact derived twice or a clock row red | The count, the two places or the figure. |
| A plain breach of a line: any line of parts 1 to 4 whose breach shows by opening the code | The lines. Anyone repeats it by opening them, and it owes no second proof. |
| The page or the accepted proposal not kept: a lasting type or a writer the proposal does not list, a file the page does not name | The place, and the page's or the proposal's line. |
| A judged finding on structure: 3.5's step on the form easiest to check, or a structure held poor though no line is plainly broken | Its source is 3.5's step 4 or the line of part 1 the structure strains. It owes a concrete consequence: a cost figure, an input or a change made hard. |
| A fate-changing branch with no source, or a condition with no branch (3.2) | The branch, or the sentence that has none. |
| A comment that restates code, or a relied-on invariant nobody wrote (3.9) | The place. |

Plain breaches, as examples and not as the whole list: a catch-all arm, in any of 1.6's spellings, a glob-imported variant among them; a chooser that does a handler's work; a fallback path; a field written for a callee to read in place of an argument; a table with two writers; a finished result changed; the message list read to decide something; a self-call on a depth the source sizes; a number read by its range; a child found by position; an id made outside its owner; an `unwrap` at an impossible state; message words outside the catalogue; an import the page does not list; a decision from a fact the page does not list among the stage's inputs; a rule with two homes, or a home with no mark; a loop that states no total; a module with no opening; a builder's test that takes a program or takes a value from a run.

A decision made at a stage whose listed inputs do not settle it (1.9) is shown by a run: two inputs the stage cannot tell apart that need different answers.

A claim the code makes is checked as a claim: a loop's stated total (2.5), an invariant a module lists, a site of 1.11. One that does not hold is a finding of the class its failure causes.

**5.4 What the verifier admits.** The verifier admits a finding when its source is there and says what the finding claims, and when the verifier repeats the finding itself: the lines open as claimed; the input reproduces; the count or the ratio measures again; the change made hard is one the SPEC, the page or the pending lists of SPEC §1.2 already hold, with the places it would touch counted. It rejects everything else, a preference among them.

**5.5 The prober's check of growth.** The prober checks growth on its own, with no counter: by time ratio at four doubling sizes, each large enough to time, over the unit's shapes, its depth shape among them, and its own many-times-many shapes, on any machine, taking the smallest of five runs. A ratio above 2.4 at any doubling is a finding, reported with its shape. 2.4 lies between the largest ratio measured on a linear shape, 2.14, and the smallest measured on a superlinear one, 2.70. Slow creep below 2.4 is the growth rows' job (2.6).

**5.6 A rejected finding** is never shown to the builder. It is kept in the unit's record, with the reason it was rejected, until the unit closes and no longer. By then each finding that was right though rejected has become a change to the standard or one line of the plan of work owed. The record never grows into a ledger.

**5.7 The review of a structure proposal** has no code and nothing to run. Its reviewer writes, from the proposal alone, what the unit will build: the passes in order, and each lasting type with its writer and its layout. It then checks the proposal against the page: what crosses the edge, the shared tables, the listed inputs. It checks it against the lines of parts 1 to 3 and the counts of 2.2. It checks the builder's breakdown against the test author's rule list. A finding there names the proposal's line and quotes what it breaks: the page's line, the standard's line or the SPEC sentence; or it names the form or the condition that one list holds and the other does not. The verifier admits it by reading.

**5.8 A cost claim that breaks no count and no row** goes to the peer as a note, with its measurement, and spends no builder round.
