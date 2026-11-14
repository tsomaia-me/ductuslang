# The code standard's acceptance table

Written by the peer session on 2026-11-14 from the lines the owner settled (the notes, Settled, the code standard). One row a root cause of `code-standard-brief.md`; the lines that prevent it; who catches it and how. It is the test the draft of the standard is reviewed against, and it is not the standard. A row marked weak is a place where the lines prevent less than the cause needs.

## The first engine

| Root cause | The lines that prevent it | Who catches it, and how |
|---|---|---|
| Slow by a constant factor: 22.8 µs an inlined call, 30.6 µs an expanded item | 2.2's counts; 2.3's default layout; 2.6 and 2.7's rows | The proposal's reviewer reads the passes and layout against 2.2 and 2.3. The reviewer reports a broken count with the count. The gate's clock rows. Weak until N: a new stage has no clock ceiling |
| One object of 71 fields with about 87 memo maps | 1 (one writer a table); 7 (the page and the proposal list every lasting type); 2.3 (a map's key is content) | The proposal's reviewer checks each lasting type has one writer. The reviewer's account lists each lasting type with its writer; one the proposal does not list is "the proposal not kept" |
| Native recursion and derived walks over source depth: 39 s on a 100,000-deep literal | 2.4; 2.1; 2.5 | The gate job of 100,000 levels on 1 MiB. The prober's time ratio on the depth shape. A self-call on a depth the source sizes is a plain breach |
| An internal error on a valid program | 8 and the rule on impossible states (typed away, the rest named and listed); 3.1 (every position a form stands in) | The prober: a panic or an internal error on an input. The corpus job: none of either. The reviewer checks each listed invariant site as a claim. Weak: no line makes a builder look for two rules meeting; the prober is the catch |

## The second engine

| Root cause | The lines that prevent it | Who catches it, and how |
|---|---|---|
| One object of 253 fields that every stage changes, with 1,100 functions on it | 1; 7 (what crosses an edge on the page, the rest in the accepted proposal) | The proposal's reviewer, before any body exists. The reviewer's account against the proposal. A crate a stage would add the compiler (4.9, to be measured) |
| Mode fields that change what other methods do | 3 (what holds for a call is an immutable argument) | The reviewer: a field written for a callee to read is a plain breach, shown by the two places |
| One function of 938 lines, and one number that means two things by its range | 6 (one handler a construct; the chooser does none of their work); 8; 3.5's step on the form easiest to check | The reviewer: a chooser that does a construct's work, or a number read by its range, is a plain breach; the rest is 3.5's step with a concrete consequence. Weak: a long function that is neither a chooser nor breaks a line rests on that step alone |
| IR nodes read as bare number triples | 8 (typed ids, no read by arithmetic on a position, packing behind the owner's accessors) | The reviewer: a plain breach |
| Two paths whose work grows with the square of the input | 2.1; 2.5 (a loop states its total) | The prober's time ratio at four doubling sizes over its own many-times-many shapes. The growth rows. The reviewer checks each 2.5 comment as a claim |
| The dense layout fitted on afterwards: 79 hash maps first, 18 allocations an inlined call pinned by its tests | 7 (the tables are named before any body); 2.3; 2.2; 2.6 (a row states the design's cost, never the day's) | The proposal's reviewer reads the layout against 2.2 and 2.3. The reviewer reports a broken count |
| A valid program refused and a wrong one accepted, by a rule built once a form and argument position | 4 (a sentence's list of forms is data, one function over it); 3.1; 3.2; 3.3 | The proposal's reviewer compares the builder's breakdown with the test author's rows. One search for the mark shows a rule with two homes. The prober: a wrong answer with its input |
| The same message text written in four places | 4 (a message has one constructor); the one catalogue | The reviewer: message words outside the catalogue are a plain breach |
| 189 sites that decline a program to a fallback | 6 (no fallback path; handled or refused by name; no catch-all arm) | The reviewer: a plain breach. The compiler itself, when a variant is added |
| Tests that state "the same as the other engine" and no expected result | 4.1; the settled test rules (a test names its sentence; no value copied from the implementation) | The test reviewer checks every value against its sentence. The reviewer: a builder's test that takes a program or a value from a run is a plain breach |
| No comments by rule, so each session relearned the code | The comment rule; a module's opening lines; 3.3's marks | The reviewer: a relied-on invariant nobody wrote, a comment that restates code. The gate's script for the marks |

## The lexer and the parser

| Root cause | The lines that prevent it | Who catches it, and how |
|---|---|---|
| A recovery rule that fires on valid programs | 3.2 (a branch that changes a program's fate has a source); 9 | The reviewer: a fate-changing branch with no source. The prober: a valid program refused |
| Typed views that answer wrong, reading children by position | 8 (a view finds a child by role) | The reviewer: a plain breach. A stated tree a grammar alternative, on the test side |
| A lexer that decides from the word before, a fact it does not have | 9 (decided once, at the first stage whose listed inputs settle it; the neutral form named on the page) | The reviewer: a decision from a fact the page does not list among the stage's inputs is a plain breach |
| Valid forms refused in constructs nothing downstream exercised | 3.1 (each alternative, each item of a list, each position); 6 (no catch-all arm) | The proposal's reviewer compares the breakdown with the rows. A stated tree a grammar alternative. The prober |
| No test that states a tree | The settled test rules; 4.8 (a stated test is a case file with its sentence's mark) | The gate counts progress in rules passing. The test reviewer |
| Imports from the old engine | 2 (earlier stages, the base layer, the libraries of line 10; the page lists them) | The reviewer: a plain breach by reading the imports. A crate a stage would add the compiler |

## The rows marked weak

- A new stage's speed until N: only the counts of 2.2 and the growth rows hold it.
- Two rules meeting on one program: nothing but the prober and the corpus looks for it.
- A long function that breaks no line: only 3.5's step on the form easiest to check speaks to it.
