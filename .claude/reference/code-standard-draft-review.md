# The code standard's draft: its one cold review, and what became of each finding

Saved by the peer session of 2026-11-14. The draft is not in the repo: `/home/cdagent/cargo-targets/peer-code-standard-draft/` holds `STANDARD.md` (parts 1 to 4, the worked example in 3.11), `FINDINGS.md` (part 5) and `check/run.sh`, which takes both forms of the example exactly as the document prints them, compiles them and runs the peer's checks. The reviewer read the first draft; the files there now are the draft after the fixes below. The fixes were checked by run where they are code and were not reviewed again.

## What became of each finding

| Finding | What was done |
|---|---|
| 1. The bad form's fault table says what the code does not do | Fixed. The bad form states its own token convention and prints `leaf`; rows 3, 5, 6, 7, 9 and 10 say what a run shows. `check/run.sh` runs the bad form on the inputs the table names. |
| 2. "Row" has two meanings | Fixed. "Rule list" is the test author's list; "growth row" and "clock row" are written in full; 2.4's 100,000 levels is a gate job. |
| 3. "Lasting type", "table" and "writer" do not hold what is put on them | Words added: table, writer, lasting type, scratch, pass. 1.1 reads writer as one function with the helpers only it calls. The owner's ruling is asked (question 1). |
| 4. 5.3's list of plain breaches does not say whether it is complete | Fixed. The class is any line of parts 1 to 4 whose breach shows by opening the code; the list is examples. |
| 5. 1.6 is not decidable, and the good form does not keep it | Words added: construct, handler, chooser, kind. A chooser's arm is one call or one value. `==` and `!=` on a variant count as a catch-all. The good form is rebuilt as a chooser with four handlers. The owner's ruling is asked (question 2). |
| 6. An id that names no row goes into the tree | Fixed. The token table ends with an `End` row, `Toks::end` is the only way to make a table, and `Toks::read` makes no id past the last row. |
| 7. A fault leaves unreachable nodes in the column | Written as the module's sixth invariant and under "What it does not settle". Not solved: it is SPEC §16's and the parser's page's, items 2 and 3. |
| 8. 3.9 does not cover the comments the good form is made of | 3.9 gains "a meaning a name cannot carry" and a test for restating (a reader of the code alone loses nothing by the comment's removal). The good form's comments were cut to those kinds. The owner's ruling is asked (question 4). |
| 9. Marks do not do what 3.2, 3.3 and the table's catch need | 3.3 says what a home is, adds a spelling for one alternative of a production, says the search is over the source and does not count a case file or the test of 4.2. 3.2 says where a branch's source is found. The good form shows two `// PAGE` marks. The owner's word is asked (question 3). |
| 10. The example's shape does not extend to its neighbours | The example is now a piece cut from one unit, the reader of expressions: `is`, level 1, level 11 and the other primaries are the same unit's, in the same table and on the same stack, and every other nesting construct is a further stack entry. |
| 11. An impossible state in `complete` falls to a default; `FaultCode` is minted in the unit | Fixed. A right-associative row carries its operand level, the decision uses it for every row, and no arm is unreachable. `FaultCode` is marked as a stand-in for catalogue codes. |
| 12. Pairs of lines that cannot both be read literally | Fixed. 2.6 says a row is never set from what the code under test costs; 2.7 says a budget is the page's and kept code holds the time measured before the unit; 2.1 defines "in line with"; 2.3 says a side table is for what few ids have. |
| 13. The counter of 2.5 is under `cfg(test)` and a harness binary cannot see it | Not fixed. It is the gate's design. Open (question 5). |
| 14. The breakdown omits sentences | Fixed. The `??` sentence, the `if` and `for` sentences, §2.5's keywords and §4.1's minus are listed, and the breakdown says its bound: how the operators are written. |
| 15. A judged class with no source | Worded: its source is 3.5's step 4 or the line of part 1 the structure strains. |
| 16. Words never defined | Defined or reworded: core crate, runtime, READY, the pending lists, the plan's line, prober, verifier, corpus, trial programs, amendment, who checks a library. |
| 17. Compile-time arithmetic and the runtime's float functions | Not added. Open (question 6). |
| Nits | Taken: the turn count; the two conditions the table does not show; no "Example B" in the document; the bad form's conventions; `failed` under 1.3; the gate's definition; 4.6 and 4.9 as rules; the Unicode sentence; the runtime bullet. Not taken: constructors (left out, and the document says so); `Vec` with no `alloc` import. |
| From the table's rows | Taken: "node" defined in 2.2; the depth shape named in 5.5; the qualifier kept on "a field written for a callee"; "role" defined in 1.8; a sentence in 2.4 on a type that holds itself through a `Box`. |

## The questions for the owner

1. 1.1's "one function that writes it": the draft reads the writer as one function with the helpers only it calls, so that one append method called from many passes is many writers.
2. 1.6's words: a construct is what a mark names; a handler is one function with the helpers only it calls; a chooser's arm is one call or one value; `==` and `!=` on a variant are catch-alls beside `if let`, `let … else` and `matches!`; refusing by name draws the catalogue's message for the construct.
3. Marks: a third spelling for one alternative of a production; a home is where the mark stands; the test of 4.2 and a case file carry a mark and are not homes; an invariant of the accepted proposal is marked `// PAGE`.
4. Comments: whether the three kinds are the whole list. The draft adds "a meaning a name cannot carry" and the removal test.
5. Counters: 4.4 allows them under `cfg(test)`, and 4.8's harness binary cannot see that. To settle with the gate's design.
6. Float functions at compile time: one sentence in 1.3, that a value computed at compile time uses the functions the runtime uses.
7. The files' names and place, and whether part 5 is its own file.

Waiting on item 3: 1.5 gains its sentence on a fault reached many times when §16's edge is ruled.

## The reviewer's report, word for word

Cold review of `/home/cdagent/cargo-targets/peer-code-standard-draft/STANDARD.md` and `FINDINGS.md`.

Runs, all in a folder of my own under `/home/cdagent/cargo-targets/claude-tmp/`, now deleted; I wrote nothing into the draft folder or the repository:
- The good form, extracted as `run.sh` does (identical to `check/from_doc.rs`), compiles alone with `-D warnings`, and the three checks of `check_tail.rs` pass.
- A recursive-descent parser I wrote straight from §5.1's productions for the slice (a second comparison operator after a comparison being the chain error), compared with the good form on every token sequence up to length 6 over 16 kinds and up to length 8 over 11 kinds: 253,690,466 sequences, 0 differences in tree, stop token, fault kind or fault place.
- The bad form, completed with the functions the document does not print (`leaf`, `binary`, `unary`, `parse_and`, `parse_not`, `parse_fallback`, `parse_add`, `parse_mul`) in the same style, and run on the inputs its fault table names.

### 1. The acceptance table, row by row

**The first engine**
- **Slow by a constant factor.** Prevented: 2.2, 2.3, 2.6, 2.7. Caught: FINDINGS 5.7 (the proposal against parts 1 to 3 and 2.2), 5.3 "a count of 2.2 broken … the count". Less than claimed: 2.2 counts "each node and each expanded item", and "node" is undefined. A lexer has no nodes, so by the letter no count binds a per-token allocation.
- **71 fields, 87 memo maps.** Prevented: 1.1, 1.7, 2.3, and in fact 2.2's hash count more than 2.3 (a memo map keyed by content passes 2.3). Caught: 5.7, 5.1, 5.3 "a lasting type or a writer the proposal does not list". Less than claimed: "lasting type", "table" and "writer" do not carry it (finding 3).
- **Native recursion and derived walks.** Prevented: 2.4, 2.1, 2.5. Caught: the 5.3 plain breach "a self-call on a depth the source sizes"; the prober's 5.5. Less than claimed: 5.5 names "the unit's shapes and its own many-times-many shapes" and never the depth shape. If "derived walks" means `#[derive]`d or compiler-made walks (drop glue, derived `Clone`/`PartialEq`/`Hash`/`Debug` on a type that holds itself), 2.4's "no function calls itself" does not name them and a reviewer reading for a self-call will not see one. I am not sure what the row means by "derived".
- **An internal error on a valid program.** Prevented: 1.8, 1.11, 1.12, 3.1. Caught: 5.3 "a panic or an internal error on an input"; 5.3's last paragraph (a site of 1.11 checked as a claim). Holds; the weak part is not made worse.

**The second engine**
- **253 fields, 1,100 functions.** Prevented: 1.1, 1.7. Caught: 5.7, 5.1. Less than claimed: "one function that writes it" is met by one append method that any number of functions call, as `Tree::push` is called from three functions in the example (finding 3). 4.9 promises nothing.
- **Mode fields.** Prevented: 1.3. Caught: 5.3 plain breach "a field written for a callee to read". Risk the other way: FINDINGS drops 1.3's qualifier ("what holds for the length of a call"). By the shortened words the good form's `scratch.stack` (pushed by `expression`, read by `complete`) is such a field, and that class "owes no second proof".
- **938-line function; a number by its range.** Prevented: 1.6, 1.8, 3.5 step 4. Caught: "a number read by its range" is in 5.3; "a chooser that does a construct's work" is not in 5.3's list (finding 4). Less than claimed: "handler" and "the code that chooses" are undefined, and the example's `expression` does every handler's work inline in the arms of the choosing `match` (finding 5). By the example a 938-line `match` meets 1.6.
- **IR nodes as bare number triples.** Prevented: 1.8, all three clauses. Caught: only if 5.3's plain-breach list is illustrative; none of the three is in it (finding 4).
- **Two quadratic paths.** Prevented: 2.1, 2.5. Caught: 5.5, 2.5 and 5.3's last paragraph. Holds.
- **Dense layout fitted afterwards.** Prevented: 1.7 ("before any body is written"), 2.3, 2.2, 2.6. Caught: 5.7, 5.3. Holds.
- **A rule built once a form and argument position.** Prevented: 1.4, 3.1, 3.2, 3.3. Caught: 3.1 and 5.7 (the breakdown against "the test author's rows"), 5.1 (each rule's home by its mark), 5.3 wrong answer. Less than claimed: "rows" there is not the defined word (finding 2); "a rule with two homes" has no class in 5.3; the mark for a production cannot name an alternative (finding 9).
- **Same message text in four places.** Prevented: 1.4, 3.8. Caught: 5.3 "message words outside the catalogue". Holds.
- **189 fallback sites.** Prevented: 1.6. Caught: 5.3 "a catch-all arm"; the compiler on a new variant. Less than claimed: `==` or `!=` against a variant, and a classifier that returns `Option` followed by `if let Some`, are catch-alls that 1.6 does not name (finding 5). "A fallback path" is not in 5.3's list.
- **Tests that state "the same as the other engine".** Prevented: 4.1, Words "stated test", 4.8. Caught: 5.3 "a builder's test that takes a program or takes a value from a run". Holds.
- **No comments by rule.** Prevented: 3.9, 3.3. Caught: 5.3's last class. Less than claimed: a missing module opening and a home without its mark have no class; 3.9 cannot say whether a `///` summary is a finding (finding 8).

**The lexer and the parser**
- **A recovery rule that fires on valid programs.** Prevented: 3.2, 1.9. Caught: 5.3 "a fate-changing branch with no source", wrong answer. Less than claimed: nothing makes a branch say which source it has, so the reviewer proves a negative over the whole SPEC (finding 9).
- **Typed views reading children by position.** Prevented: 1.8 "a view finds a child by its role". Caught: not in 5.3's list. Less than claimed: "role" is undefined. "The second child of kind Expr" is a role to a builder and is the position bug. The line does not say the role is recorded when the node is built.
- **A lexer deciding from the word before.** Prevented: 1.9. Caught: 5.3 "a decision from a fact the page does not list among the stage's inputs". Less than claimed: the word before is inside the lexer's listed input, the source text. The fault was a fact that does not settle the decision, not a fact outside the inputs. The catch fires only if the page names the neutral form, as "the page not kept".
- **Valid forms refused in constructs nothing exercised.** Prevented: 3.1, 1.6. Caught: 5.7, 5.3. Holds, with the "rows" wording.
- **No test that states a tree.** Prevented: 4.8, 4.5. The catch is out of scope. Holds.
- **Imports from the old engine.** Prevented: 1.2 with 1.10; the opening paragraph. Caught: 5.3 "an import the page does not list". Holds.

### 2. Findings, most severe first

**1. The bad form's fault table states things the printed code does not do.** By run.
- Fault 5: "After the fault the function goes on and builds `(a < b) < c`". It does not. `parse_cmp` has an `if`, no loop: on `a < b < c` it pushes the message, builds `<(a, b)` and returns stopped at the second `<`. The second message, if any, would come from a caller this example does not have.
- Fault 3: "`a < b <= c` is accepted". The function returns `<(a, b)` with no message, stopped at `<=`. That is the same hand-back the good form gives for `a b`, which the good form's own prose calls "the caller's".
- Fault 10: "`(a b)` is accepted". `b` is taken for the closer and the function stops at the real `)`, leaving it to the caller. `(a b` is the input it swallows whole.
- "It passes a first look and most tests of valid programs", and fault 2's `a +`: the bad form reuses `TokKind`, whose `End` says "no table holds it". With such a table every input panics, `a` and `a + b` included, at `parse_pow`'s `self.toks[self.pos]`. With an `End` in the table, `a +` panics only because `leaf()`, which is not printed, steps over `End`.
- Fault 6 omits `parse_pow`'s self-call, marked [7] for another reason: 100,000 `**` overflow a 1 MiB stack as `-` and `(` do.
- Consequence: a reviewer learns attribution from this table, and three of twelve rows fail when repeated. The bad form needs its own token convention stated and the three descriptions rewritten to what the code does.

**2. "Row" has two meanings, and the second is undefined.** By reading. Words: "row: a frozen test of cost (2.6)". 3.1 and FINDINGS 5.7 have the proposal's reviewer compare the breakdown "with the test author's rows". By the Words table that compares a list of forms with cost tests. What the test author's list is, and where the reviewer finds it, is nowhere. Two acceptance rows rest on this step. 3.11 adds a third sense ("the table's row"). 4.5's "stated rules" is likewise undefined.

**3. "Lasting type", "table" and "writer" do not hold what 1.1, 1.7, 5.1 and 5.3 put on them, and the example's own proposal fails them.** By reading.
- Words: a lasting type is one "whose values outlive the function that makes them". Every returned value does. By that, `Node`, `NodeId`, `Fault`, `FaultCode`, `Expr`, even `Under` and `Infix`, are lasting; step 2 lists two. 5.3 makes "a lasting type the proposal does not list" a finding.
- Step 2 calls `Scratch` a lasting type and names no writer and no layout for it. `Scratch::stack` is mutated by `expression`, `complete` and `close`. Either it is not lasting or it breaches "one writer".
- "Table" is never defined, nor is its relation to "lasting type" (1.1 speaks of tables, 1.7 and 5.1 of lasting types).
- "Writer" is shown as an append method called from three functions. Under that reading a 253-field object passes 1.1 by giving each table one `push`. The rows need "the one pass that appends", and no sentence says it.

**4. FINDINGS 5.3's plain-breach list does not say whether it is complete.** By reading. "A finding is one of these", and the class is a colon and a list. If complete, breaches the acceptance table calls "a plain breach" have no class: a chooser doing a handler's work (1.6), a read by arithmetic on a position or a child by position (1.8), an untyped id or one made outside its owner (1.8), packing outside the accessors (1.8), a fallback path (1.6). Nor do: a pass changing a finished result (1.1), reading the message list (1.5), `unwrap` at an impossible state (1.11), a global or environment read (1.3), a branch on `cfg(test)` (4.4), a rule with two homes (3.3), a loop with no total (2.5). If illustrative, say so. A verifier applying 5.4 strictly rejects them.

**5. 1.6 is not decidable as written, and the good form does not keep its last sentence.** By reading.
- "Construct", "handler" and "the code that chooses a handler" are undefined. In `expression` the choosing `match` holds the handlers' work in its arms.
- "Each question about a kind is answered once, beside the enum." The good form answers questions about `Pending`, `Op`, `Starts`, `Continues`, `Stop`, `Under` and `Expect` in `floor`, `complete`, `close` and `expression`, away from those enums; `floor` and `complete` each take `Op`'s two variants apart. The prose applies the sentence to `TokKind` only. Which enums are "kinds" is left to the builder.
- "Refused by its name" names no mechanism (which call, which code, whose catalogue entry), and the example shows no refusal.
- The catch-all list (`if let`, `let … else`, `matches!`) omits `==`/`!=` against a variant, which the bad form uses three times with no fault number, and omits a classifier returning `Option`. The good form derives `PartialEq` on `TokKind` and never uses it.

**6. The good form puts an id that names no row into the lasting tree.** By run. `a +` over a two-token table yields `Fault { at: TokId(2) }` and `next: TokId(2)`. `Toks::read` makes ids by `at.0 + 1`. 1.12's licence to index directly rests on "only the owner makes an id, as the return of the append that makes the row". A later pass that looks up `fault.at` for a span through a direct-indexing accessor panics. The module's invariants speak of `NodeId` only. The standard has no word for a cursor or a past-the-end position as distinct from an id, and the example teaches making one by arithmetic.

**7. After a fault the good form leaves unreachable nodes in the dense column.** By run. `a < b < c` leaves `Leaf(a)`, `Leaf(b)`, then the `Fault` root. `(a + b c)` leaves two leaves and an `Infix`. The module's invariant invites passes in id order ("a node's operands have smaller ids than the node"), and 2.4 pushes every later pass that way. Such a pass meets the orphan leaves and resolves their names, against 1.5 and §16.3 ("no other name on the line is resolved"). The prose defers "what else of its line the tree keeps" to §16 and the page, but the module's invariant list does not say the column may hold nodes under no root. By 3.9 that is a relied-on fact nobody wrote.

**8. 3.9 does not cover the comments the good form is mostly made of, and the prose claim about them is false.** By reading. 3.9 allows three kinds: an invariant relied on, the reason for an odd choice, the SPEC section. The good form has many `///` lines that say what a type, field or function is or does (`/// A name or a number.`, `/// The kind at at and the id after it…`, `/// Completes every pending operator back to the nearest open group…`). "What to see" says "Every other comment states an invariant relied on or the reason for a choice. None restates code." A reviewer can report `Toks::read`'s comment under 5.3's last class, and a builder cannot tell from 3.9 whether a contract comment is required, allowed or a finding. "A choice that looks odd" is the builder's judgement.

**9. Marks do not do what 3.2 and 3.3 and the table's catch need.** By reading.
- The production mark cannot name an alternative. The example's page gives `is`/`is not` (both alternatives of `CmpExpr`) and "every other `Primary`" to other units; they too carry `// SPEC §5.1 CmpExpr` and `// SPEC §5.1 Primary`. "One search … shows when a rule has two" then reports two homes by construction.
- 4.8 puts the mark in every case file, so the search also returns those. 3.3 does not say the search is over source only.
- No `// PAGE` mark appears in the example. The `internal("brackets arrive paired", …)` branch has a page invariant as its source and carries none. 4.2 puts a `// PAGE` mark on an invariant "the accepted proposal lists", which is not the page's, and a page has no "section" for the quoted words to be unique in.
- The good form's fate-changing branches that are not a rule's home (`CloserExpected`, `OperandExpected`, the return at a stop token) carry no source. 3.2 demands one and no line says where it is written.

**10. The example shows 2.4 on the one easy case, and its shape does not extend to the neighbours its page names.** By reasoning; I am less sure how much weight the "no design" disclaimer carries.
- `is` sits at level 5, inside the table whose one home is `complete`. A second unit cannot read it: `a is b or c` needs this unit's stack on both sides. That contradicts "a new operator is a new arm" and 1.4.
- Postfix (level 11) and the other primaries nest expressions in `[ ]`, call arguments and `{ }`. The invariant "`Scratch::stack` is empty when `expression` is entered" forbids a nested call on the same scratch. A second unit calling `expression` for each bracketed position is recursion "through others" on source depth, which is what 2.4 forbids.
- A builder taking the form from the example alone gets "one function per unit with a private stack". A builder that held the page wrong would owe a 3.6 OBJECTION; the example shows none.

**11. 1.11's "where Rust can type it" has no bound, and the good form's "one that is left" is not the only one.** By reading. `complete` relies on "Levels 4 and 9 hold prefix operators only, so these two levels are never equal" and lets the impossible equal case fall to `false`. It is not typed away (it could be), not sent through `internal`, and not among "the four the module opens with". "What to see" says "the one that is left is named". Separately, `internal("brackets arrive paired", at)` and the enum `FaultCode` are minted in the unit: 3.8 says a builder writes no catalogue entry, 5.3 makes "message words outside the catalogue" a plain breach, and the page as cut lists no entries. `Toks` and `internal` are marked as stand-ins; `FaultCode` is not.

**12. Two pairs of lines that cannot both be read literally.** By reading.
- 2.6 "A row states the design's cost, never the day's measurement" against 2.7 "A budget is measured, never guessed" and "Kept code holds the time it was measured at". A clock row's ceiling can only be a measurement.
- 2.1 "work grows in line with the size" against 2.3's default "searched by halving": a halving search per node is n log n in a counter. "In line with" is never defined as proportional. 5.5's 2.4 threshold passes n log n, so the prober and a growth row would disagree.
- Also 2.4 calls the 100,000 levels "a row", 2.6 knows two kinds of row and it is neither, 4.7 calls it a gate job.

**13. 2.5 and 4.4 put a duty on the builder whose other half is nowhere.** By reasoning; uncertain, the rows' mechanics belong to the unwritten loops document. The counter is under `cfg(test)` and is a private field. `cfg(test)` is set only for tests compiled inside the same crate, so 4.8's harness binary cannot see it. Rows are frozen and "no builder edits one", yet a row must read a counter whose name the builder invents. The example says "how a growth row reads the counter is the gate's".

**14. The breakdown claims "Every sentence of the SPEC that speaks of these operators" and omits some.** By reading the SPEC. Nothing in it is stated wrongly. Missing:
- §5.1 "`??` … binds looser than arithmetic and tighter than a comparison" with `port ?? 80 > 1024`, `x ?? base + 1` and `x ?? -1`. The last is the same kind of condition as `2 ** -1`, which the breakdown does list. The code is right on all three.
- §5.1 "An `if` that is an operand of another operator is parenthesized", and §5.10's same for `for`: the unit's `Starts::Nothing` is where that is refused.
- §2.5's keywords standing as member names ("the word `and` is the operator").
- The second sentence of §2.6's minus bullet and §4.1's "a minus applied directly to a literal forms one value".
- "CmpExpr, first alternative" lists four of `CmpOp`'s six items without saying the page took the other two.
- It includes one sentence of types (`-x ?? 1`) and none of §5.2 or §5.3, so the criterion for inclusion is left to the reader. 3.1 says "every sentence that speaks of the construct".

**15. FINDINGS admits a judged class with no source.** By reading. 5.3: "or a structure held poor though no line is plainly broken". 5.2 requires a quoted source, and 3.5 step 5 says "no reviewer prefers one". Either the clause is 3.5 step 4 again or it is the preference 5.4 rejects.

**16. Words used and never defined.** By reading.
- "core crate" and "the runtime" (1.10, 1.12).
- "READY" (4.3).
- "the pending lists" (5.4) and "one PLAN line" (5.6).
- "prober" and "verifier".
- "the corpus" (1.12, 4.7).
- "scratch" in 4.3 (throwaway programs) against Words and 1.1 (a function's values).
- "amendment to the proposal" (3.5 step 2): no act of 3.10 carries it, and 3.6 routes "an accepted proposal found wrong" through OBJECTION.
- In 1.10, "gets a second look for determinism" and "each checked to exist in that form and version" name no one who looks and nothing looked for.

**17. Possible undershoot.** By reasoning; uncertain, and SPEC §12.5 may be held to cover it. No line says compile-time arithmetic uses the same float functions as the runtime. A strong model writes `f64::powf` in a constant folder unasked, and 1.3's list of what no result depends on does not name the platform's float library.

### 3. Nits and preferences

- `complete`'s total comment gives "pushed once and popped once" as the reason, but `close` counts a turn when it pops nothing. The bound still holds: by run the turns never exceed the tokens, over all sequences up to length 7 of 10 kinds. `check_tail.rs` allows `len + 1`, which is not needed.
- "The one condition the table does not show, `2 ** -1`": the breakdown's own NotExpr row is a second (`a + not b`).
- The module opens `//! Example B.`; the document has no example A.
- `Tree`, `Scratch` and `Toks` have no constructor, so the printed module cannot be used from outside itself.
- `Toks::read` answers `End` for any id past the end, not only the first: a default for an impossible state.
- The bad form uses the good form's `Node`, which holds typed ids, while fault 11 says its ids are bare `usize`. It also has no `parse_not` or `parse_fallback`, printed or mentioned.
- The bad form's `failed` is also a 1.3 breach (written by `parse_cmp` for `parse_primary` to read); the table cites 1.5 only.
- 5.3 classes "a fact derived twice" under "slow by a constant factor", owing "the count or the figure". It is shown by the two places, like a plain breach.
- Words' "gate" runs "on every pushed commit"; 4.7's gate jobs carry "when it runs".
- 4.9 and 4.6's second and third sentences state no rule for a builder.
- 1.10's "The Unicode tables reach one, in which identifiers are valid and in the NFC form of paths" does not parse on first reading.
- 1.12's runtime bullet says what the runtime may not do and not what a lookup there does instead.
- The good form uses std's `Vec` with no `alloc` import; fine for an example, but 1.10 is about exactly that.
