# The owner's rulings on the code standard, 2026-11-13 and 2026-11-14, word for word

Saved by the peer session that held the conversation, so that the standard, the parser's page and the loops are written from the owner's words and not from the notes' short lines. Each message answers the peer's message on the topic named; the settled lines the answers produced are in the notes under Settled. Where a later message changes an earlier one, the later stands. The numbers and letters (lines 1 to 10, 2.1 to 2.8, 3.1 to 3.6, 4.1 to 4.9, 5.1 to 5.7, choices A to L) are those of the notes. Corrected on 2026-11-15: these are messages the owner passed on to the peer; four carry bracketed blanks from a draft (lines 524, 625, 691 and 740), the last unfilled, "[Your answers to its questions go here.]", so this file shows what the owner forwarded, and the owner said on 2026-11-15 not to know the loops, the subagents or the code standard (`owner-rulings-compiler-design.md`, fourth message).

---

# First: topic 1, the architecture principles

Good work. This is grounded, and the point about the middle is right: the
SPEC does rule out a clean pipeline there. The nine are accepted with
three corrections. B and C are yours as picked, each with one note. On A
I choose differently. One rule is added at the end.

The three corrections

1. Principle 5 decides an open §16 edge without saying so. "Nothing
   de-duplicates the message list" stands. But how many messages a faulty
   line draws when a `for`, a pattern or a call reaches it many times is
   not ruled yet. If I rule "one", the fault must be found once, before
   expansion, by something in the data and not by a scan of the list.
   Say in the principle that it and that edge are settled together, and
   put the edge first in the §16 batch.
   Also word the edges of "write-only": ordering the messages for output
   and returning failure when the list is not empty are allowed.

2. Principle 8 has a hole against a ruling of mine. Ids are local to each
   file. An id typed only by its table can be used on another file's
   table, and that is a silent wrong hit, not a miss. Say how the type
   prevents it: the id carries its file, or something else you can show.

3. Determinism is a principle, not a cost item. No result depends on a
   hash map's order, an address, a thread or a clock. Fold it into
   principle 3.

Two smaller wordings

- Principle 1: say that appending is allowed. An entry once written never
  changes; a table may grow.
- Principle 9: the decision is made once, at the first stage whose inputs
  settle it, and the neutral form is named on the page.

Choice A: neither 1, 2 nor 3

Pick 1 gives the builder bodies to fill. That fences the builder; it does
not shape it, and it puts all design on one peer session and one
reviewer. You are right that in a compiler the tables are the design, so
I split it:

- On the page: what crosses a stage's edge, and the shared tables of the
  interleaved middle, each with its one writer. Those are the expensive
  mistakes.
- Inside a stage: the builder's first hand-back is a structure proposal.
  It lists the lasting types, each with its one writer, and the passes.
  A reviewer reads it cold against the principles before any body is
  written. The builder then builds to the accepted proposal.

The scope stays bounded by the page, and the builder does design under
review. The cost is one more review step a work unit. Write this into
the loops when they are revised.

Choice B: pick 1, no catch-all arms on our own enums

Accepted. The cost lands on the large kind enum. Keep it bearable: each
question about a kind is answered once, beside the enum, with every
variant named.

Choice C: pick 1, with one thing to state

Accepted. But say what a lookup by a typed id physically does. Either
every lookup still returns an error to pass up, which is today's 396
sites in new clothes, or the table owner's one accessor indexes directly
under an invariant the module lists. I lean to the second. It touches my
no-panic line, so bring it to me in words, with what the robustness
corpus proves about it.

One rule added: libraries

My usual rule is "before writing custom code, check whether a
well-established, permissive, well-maintained library does the job".
Here it holds at the edges and not at the centre, so it is stated as a
table and not as a habit:

- The core crate takes nothing but `alloc` and `libm`.
- Generic jobs at the edges come from libraries: Unicode tables, hashing,
  big integers, shortest-float printing, the editor protocol, the command
  line, checksums, fetching.
- The compiler's own machinery is written, never borrowed: the lexer, the
  layout, the lossless tree, recovery, the evaluator, inference. A parser
  library would shape the code and bring its own recovery and cost.
- A library on a path that reaches a value gets a second look: a hash
  with a random seed or a float routine that differs by platform breaks
  determinism.
- A builder never adds a dependency. It proposes one with the job it
  does; the page lists the approved ones, each checked to exist in that
  form and version.

Place it where it fits best among the principles and tell me where.

What I want next

1. The nine lines again with these changes, short, for my last look.
   Then record them under Settled, the code standard.
2. Topic 2, cost discipline, in the same form: grounded in the reviews,
   options where a choice is mine, your pick.

Still nothing is written as a rules file, an agent file or a loops file.

---

# Second: topic 1 settled, the no-panic line, topic 2

Topic 1 is settled as you wrote it at the last look. Record it as
settled, without the "last look" mark. The split of the §16 edge and the
Unicode point are both right; keep them.

Choice C: the no-panic line is two rules, not one

- The compiler takes reading 1: no input draws a panic, tested by the
  corpus. Direct indexing through the table owner's one accessor is
  accepted, with the three invariants listed in the module.
- The runtime in the core crate keeps the strict reading: no path can
  panic. That is my hard rule and it does not change; it runs on devices.
  Say both in the standard so no builder carries the compiler's reading
  into core.
- The backstop at the compiler's edge is not a separate question. A panic
  that kills the language server on a keystroke is a product fault.
  Verify where unwinding works for the hosts the compiler ships to (the
  tool, the language server, a build script, WebAssembly) and bring me
  what a panic does in each, with your pick.

Topic 2: accepted in shape

2.1, 2.4, 2.5, 2.6 and 2.7 stand as written. Four fixes:

1. Kept code and 2.2. The lexer allocates about 5,000 times on the large
   fixture, so the rule would fail it on its first run. Rule: the kept
   lexer owes "no allocation a token" after the recovery work unit, not
   before it, and that unit's page carries the counting row.
   The likely cause is `line_stack_snapshot` (lexer/mod.rs:1337), which
   copies the indentation stack once a line for the rewind that the
   two-pass design removes; it is on the sizing's delete list. That is
   read, not measured. Before you write that page, make one counted run
   by call site with the bench that exists, and say what else allocates
   a line once it is gone.
   State the same for every other piece of kept code a count would fail.

2. "Allocations: none" needs its limit said. None a node and none an
   expanded item; a built value allocates in line with its own new
   elements, which is the third size.

3. 2.3 is a default, not a finding. The three layout forms are the last
   design's layout, and that design promised 16 to 53 times and delivered
   1.4 to 16. They stand as the default until the thin slice confirms or
   changes them. Say so in the line.

4. The counters are placed by the builder being checked, and a loop
   without a counter passes its growth row. The prober checks growth on
   its own, by time ratio at doubling sizes, which needs no counter and
   runs on any machine. Write that into the prober's side when topic 5
   comes.

Choice D: pick 1. The cost statement is a comment at the loop, with its
counter. Fix 4 stands beside it.

Choice E: option 2, with one rule and one exception

- The gate already runs the clock rows on every pushed commit. Review
  starts only on a tip the gate marked green. A red tip goes back to the
  builder and spends no round. That removes your reason for pick 1.
- Builders get no shell on wb2; the reference machine stays quiet.
- Exception: a work unit whose job is speed may be given wb2 by its page,
  under the gate's lock, because tuning needs many quick runs.
- The builder still runs the growth rows itself, anywhere.

From your veto list

- Agreed: the structure proposal's review spends no fix round, and an
  accepted proposal found wrong takes the objection route.
- Agreed: the hash's key waits for the slice.
- Agreed: peak bytes a node is a growth row with no ceiling until N.

What I want next

1. Topic 2's lines again with these changes, short, then recorded as
   settled.
2. Topic 3, the method from a SPEC sentence to code, in the same form.

Push your notes commits; main is behind what you work from. Still no
rules file, agent file or loops file.

---

# Third: the backstop, topic 3

The backstop is accepted, topic 2 stands as recorded, and topic 3 is
accepted with three fixes. F, G and H are your picks, each with one
addition. Your notes commits are pushed; the other session did it.

The backstop

- Accepted: one catch at the compiler's public entry, state dropped
  after it, and a caught panic still counted as red by the gate.
- It belongs on the design page of the compiler's entry, not in the code
  standard: none of the four hosts exists yet. The standard carries one
  sentence: the entry is the one place that catches a panic, and no
  other code does.
- WebAssembly stays on stable, as you picked. Record its limit on that
  page as taken from the docs you cite, not as verified here.

Topic 3: three fixes

1. 3.2 is too literal. A fast path for plain ASCII or a table that grows
   is a branch with no sentence, and it is fine. The rule is about a
   branch that changes what a program is accepted as, what it means or
   what it draws. Say that. A design-page invariant counts as a source
   beside a SPEC sentence.

2. 3.5 undershoots. After the principles and the counts it calls any two
   forms equal and bars a reviewer from preferring one. Nothing in the
   standard yet speaks for simplicity. Add one step before "equal": the
   form a reader can check against its sentence with the least else in
   view wins. A reviewer may raise it with a concrete consequence, as I
   settled for findings on design. Only what is left after that is
   equal.

3. 3.3 needs the mark's exact form. "Searches the code for the section's
   mark" works only if every rule's home carries the same searchable
   mark. The comment rule already asks for the SPEC section a piece of
   code implements. Give its exact spelling in the standard, so one
   search finds a rule's home and shows when it has two.

3.1, 3.4 and 3.6 stand as written. In 3.4 use one name for the line a
builder writes when the SPEC is silent; the loops draft calls it GAP and
this topic calls it QUESTION.

Choice F: pick 1

Accepted. Two additions:
- The builder's breakdown is a list of forms and conditions, each with
  its section, not prose.
- It is not kept as a record. What survives it is the marks in the code.

Choice G: pick 1

Accepted: the page lists each line the kept code does not meet and the
work unit that owes it; the builder fixes what is listed for its unit
and nothing in passing; new functions and types meet every line.

One addition, and it bears on the parser. The sizing counted only the
conformance findings. The kept grammar also breaks lines that are now
settled: it has no comments, it has catch-all arms, it keeps a flag that
silences later faults on a line, it renames token kinds after the fact.
A repaired parser owes all of that later or lives under a lower
standard.

So, once and read-only, before the trial's verdict: count the settled
lines the kept grammar, lexer, tree and views each break, with the
places. No fix and no estimate. That count sits beside the trial's
result when I choose between repair and rewrite.

Choice H: pick 1

Accepted: one catalogue, the page proposes entries, I approve them with
the page, and a builder writes none.

One addition, so approval stays quick: bring me a style line and a
handful of model messages once, before the first page that needs
entries. Each page's entries are then written to that pattern. A stated
test compares a message's code and its place, never its words.

What I want next

1. Topic 3's lines again with these changes, short, then recorded as
   settled, with F, G and H.
2. Topic 4: whether a builder writes tests of its own beside the frozen
   ones, and what keeps them few, real and fast. The comment rule there
   is already settled.

Still no rules file, agent file or loops file.

---

# Fourth: topic 4

Topic 3 is settled as recorded. Topic 4 is accepted with one wording fix
and two additions. Choice I is your pick 1. Your notes commit is pushed.

Topic 4

4.1 to 4.5 stand, with one fix to 4.1. "No expected value from a run or
another implementation" must not forbid a property. A check that two
runs agree, the same answer twice or the same answer with the top-level
items reordered, copies no value and is allowed where the SPEC promises
the agreement. Say so in the line.

Choice I: pick 1. A builder keeps tests of a lasting type's listed
invariant only, with your four bounds. The invariants are the ones the
accepted structure proposal lists, so a reviewer has seen the list
before any test exists.

Addition 1: the suite's speed

You list 293 ignored tests and one of 234 s as evidence and then give no
rule for them. My pain with the last build was the whole suite, not the
builder's tests. Bring lines for it:
- what a builder runs before a commit, its unit's cases and its
  invariant tests, finishes in seconds;
- no test is ignored. A check that is slow by nature, a growth row at
  its largest size, the 100,000 levels, the robustness corpus, is a
  named gate job on a page, not an ignored test;
- how the stated tests are built so that a run does not pay one link a
  file. Measure before you propose; do not guess.
State each as a rule with what it prevents, and where a number is
needed say where it comes from.

Addition 2: the old tests

The kept code carries about 4,200 lines of tests, and several break 4.1:
the recorded digests of `front_end_goldens.rs`, `line_shape.rs` as its
own oracle. One of those digests had to be recorded again today just to
merge the SPEC. Treat them as choice G treats kept code:
- each old test that breaks a line is listed on the page of the unit
  that retires it;
- the page names the regression it caught and the stated test that
  takes over, before it is deleted;
- until then it runs as it is, and nobody adds to it.

From your veto list

- The gate script for the marks: bring it in the loops draft.
- The kept-code count after topic 5, once: agreed. Add the old tests to
  that count.
- The style line and model messages before the first page that needs
  them: agreed.

What I want next

1. Topic 4's lines again with these changes, short, then recorded as
   settled.
2. Topic 5: what a reviewer must show before its findings count, the
   verifier's gate reworded to admit a finding on design and still
   reject a preference, and slow by a constant factor and badly
   structured as classes of finding. Include the prober's growth check
   by time ratio.

Still no rules file, agent file or loops file.

---

# Fifth: the suite, topic 5

Topic 4 is settled. 4.6, 4.7 and 4.8 are accepted as written; the
measurement was worth making. Topic 5 is accepted with four fixes. J, K
and L are your picks, J with one change. Your commit is pushed.

4.9

Decide it on the design page after the slice measures it, as you say.
Put one more fact beside the build time when you do: a crate a stage
makes Rust itself refuse an import of a later stage and a reach past a
stage's public surface. That is lines 2 and 7 kept by the compiler with
no reviewer.

Topic 5: four fixes

1. Choice J, pick 1 with a change. The account gates the findings that
   rest on reading or reasoning. A finding known by a run, an input that
   reproduces a wrong answer, a panic or a growth, counts whatever the
   account says: your own test is that someone else can repeat it, and
   the verifier can. A difference between the account and the accepted
   proposal stays a finding, or the account is withdrawn.

2. A plain breach needs no second proof. As 5.3 reads, "badly
   structured" needs a broken line and also a cost figure, an input or
   a change made hard. A catch-all arm, a field written for a callee, a
   table with two writers: each is repeatable by opening the lines, and
   that is enough. The concrete consequence is owed only where a line
   needs judgement: 3.5's step on the form easiest to check, and any
   claim that a structure is poor though no line is plainly broken.
   My earlier wording asked for a consequence on every design finding;
   this replaces it.

3. Classes missing from 5.3:
   - a panic or an internal error on an input;
   - an answer that depends on order: of top-level items, of a hash
     map, of runs;
   - the page or the accepted proposal not kept: a lasting type or a
     writer the proposal does not list, a file the page does not name.

4. Two gaps:
   - The review of a structure proposal has no code to give an account
     of and nothing to run. Say what its reviewer shows and what a
     finding there needs: the proposal against the page, against the
     lines of topics 1 to 3, and the builder's breakdown against the
     test author's rows.
   - 5.6's record of rejected findings lives until the unit closes and
     no longer. By then each one right though rejected has become a
     change to the standard or one PLAN line. It must not grow into a
     ledger.

5.2 and 5.5 stand. In 5.5 say that each size is large enough to time,
and keep the two measured ratios beside the 2.4 as its source.

Choice K: pick 1. A change made hard is one the SPEC, the page or the
pending lists already hold, with its places counted.

Choice L: pick 1. A cost claim that breaks no count and no row goes to
you as a note with its measurement and spends no builder round.

What I want next

1. Topic 5's lines again with these changes, short, then recorded as
   settled with 4.6 to 4.8.
2. The kept-code count, old tests included, now that the lines are
   settled: read-only, with places, no fix and no estimate.
3. The worked example. Propose two or three constructs it could be,
   each with what it would show, and your pick. It must include an
   explicit-stack walk, as you said. I choose the construct before you
   write it.
4. Then the acceptance table: one row a root cause, the line that
   prevents it and the reviewer step that catches it.

Still no rules file, agent file or loops file.

---

# Sixth: the rewrite, example B, the order of what is owed

Topic 5 is settled as recorded, 5.7 included. The count changes a
decision of mine. The worked example is B. The acceptance table stands.
Your commit is pushed.

The parser: rewrite, no trial

The count answers what the trial was to measure, and the design pass is
not over, so its list can only grow. Against the settled lines, the
lexer and the grammar have no opening comment in 31 files and no mark;
204 lines write a message's words in place; 44 places choose among
constructs with a catch-all or a shortcut; the flags and side lists are
woven into a recovery that is replaced anyway; the lexer cuts its tokens
and lexes again. With the twelve refused families on top, nearly every
function there is edited, by reading old code. Repair costs much and
returns little.

So:
- The lexer and the grammar are rewritten to the design. The trial is
  dropped, with its pass and stop lines.
- The old lexer and parser stay in the tree as a reference, a source of
  programs for cases, and the speed to match: 8.3 ms for the large
  fixture on wb2. They are never a yardstick for a tree or a message.
- The old tests go with the code they test, all 4,218 lines, each
  retired by a named unit as the old-tests rule says. A program in one
  may become a case's input. An expected value never does.
- What the old code proved carries over as checks the new code must
  pass: the source reproduced byte for byte, 100,000 levels on a 1 MiB
  stack, the growth rows, and every layout sentence of §2.2 as stated
  tests.
- The robustness check carries over as its recipe, not as stored files:
  the mutations of the recovery review (cut at every character; each
  token deleted, doubled, swapped; the stray insertions) over the valid
  programs, checked for no panic, a whole result and the source
  reproduced. It is a gate job. Its size is set by its run time,
  measured, not by the old figure of 700,000.
- That recipe lives today only under
  /home/cdagent/cargo-targets/peer-syntax-review/recovery/. Note it as
  owed: it moves into the repo when the gate is built, and the folder
  is not cleaned before.
- The tree and the views are decided on the page for what the parser
  hands on: kept, reshaped or rewritten, each with its reason. Say
  there whether the views are written by hand or made from one table.

What I want on the parser's design page, beyond what the notes list:
- the work units of the rewrite in their order, each small enough to
  read whole, each with its budget in rounds;
- the first unit chosen so that it is small and its cost is a fair
  measure for the rest. Tokens, tool calls, rounds and wall time are
  recorded for it;
- no time estimate.

The §16 batch moves up

New recovery code cannot be written before §16's edges are ruled. Bring
the batch before the tests of the first unit that draws a syntax fault,
the edge of a fault reached many times first.

The worked example: B

§5.1's expressions parsed by level. It is the next code to be written,
and it shows a table as data, an explicit stack, a classifier beside the
kind enum, and a fault as a value for `a < b < c`. A is the better
lesson on method, but its types would be invented ahead of the middle's
design and could anchor it. Add or swap in A when the middle is
designed.

The acceptance table

Accepted with its three weak rows as they stand:
- a new stage's speed until N: N closes it;
- two rules meeting on one program: the prober's pass over neighbours,
  each construct inside each construct it may stand in, is a stated
  duty when the prober's file is revised;
- a long function that breaks no line: 3.5's step and the judged
  finding on structure hold it. No cap.

What I want next, in this order

1. Record the rewrite and the dropped trial in the notes, and remove
   what they make obsolete.
2. The standard itself, written from the settled lines alone, with
   example B. One cold review against the acceptance table, then to me.
3. The page for what the parser hands on, with the units.
4. The §16 batch.
5. The loops and the agent files, revised against the standard.

Tell me your context size before you start item 2. If you are near
your cap, bring the notes up to date and stop there.

---

# Seventh: the draft is accepted in shape; the answers to its seven questions; one narrow review

Good work, and the right place to stop. The draft is accepted in shape
and is not approved yet: a few fixes and one narrow review come first.
[Your commit is pushed.]

The preference line

Commit it as written.

The seven questions

1. The writer. Accepted: one function with the helpers only it calls.
   Strike the escape "or a shared table named on the page". A shared
   table has one writer too (1.7). Two passes that fill one table are
   two tables or one pass.

2. 1.6's words. All five accepted, one sharpened. `==` and `!=` on a
   variant count as a catch-all where they choose among constructs. A
   grammar position that requires one exact token is not a choice among
   constructs. Name the one allowed form for it in 1.6 and show it in
   the example, where `)` is expected.

3. Marks. Accepted, all three spellings, and `// PAGE` for an invariant
   of the accepted proposal. The spelling was shown to me in topic 3.

4. Comments. Both additions accepted. Word the fourth kind as "a
   meaning neither a name nor a type can carry", so 1.8 is tried first.

5. Counters. Settle it with the gate's design, as you say. Record it as
   open in the notes, with the note that 4.4's wording changes with it.

6. Float functions. Add the sentence to 1.3.

7. Names and place. Accepted: `STANDARD.md` and `FINDINGS.md` in
   `packages/symphonyscript/docs/`, two files. The old files are deleted
   and the two CLAUDE files fixed in the commit that lands the approved
   standard, not before. For each old file, say first in one line what
   it holds that the standard does not, or that nothing is lost.

The veto list

- Numbering: accepted. Change the acceptance table's bare numbers to
  match.
- The example as a piece cut from one unit, without `is` and `is not`:
  accepted, since the breakdown says what is left out.
- The table as the arms of two functions beside the kind enum:
  accepted. An array would state the operators twice.
- The four additions:
  - No `as` cast between integer types: not as written. The good form
    keeps the clause by storing ids as `usize`, eight bytes an id.
    Either name the one home of the conversion, in the base layer, and
    show a four-byte id, or drop the clause until the base layer is
    designed. Tell me which.
  - No lasting type that holds itself through a `Box`: keep.
  - A buffer comes in empty and goes back empty: keep, worded as the
    one exception to "scratch dies in its function".
  - `unwrap`, `expect`, `unreachable!`, `panic!` and assertions named
    in 1.11: keep.
- Left out, 2.2's sentences on the lexer, the grammar and the IR:
  agreed, they are a page's facts.
- Left out, 1.5's sentence on a fault reached many times: agreed for
  the file, but the notes must owe it. When that §16 edge is ruled the
  sentence goes into 1.5 in the same change.

The two findings not fixed

Accepted as they stand. The fault half of the example is provisional:
record in the notes that it is checked again after items 2 and 3. A
faulty expression that leaves nodes under no root does not fit "the
source reproduced byte for byte", and the parser's page must answer it.

One more review

The good form broke the standard in four places and was then rebuilt
whole. Your 24.8 million sequences prove what it does, not that it
keeps the standard's own lines. So one narrow cold review: the rebuilt
good form against parts 1 to 4, and nothing else. The reviewer gets the
two files and no word from you on what changed. That is the last
review of this draft.

The lesson

Write P-9 with its reason tightened: "Run every behaviour a document
claims of code it prints, the bad example as well as the good, before
writing the claim, because a behaviour read off code and never run can
be wrong, and the document then teaches a false rule."

What I want next

1. The answers above worked into both files, and the example compiled
   as edition 2024, which is the workspace's.
2. The narrow review, then its findings and what became of each.
3. Both files brought to me for approval, with the old files' lines.
4. Then item 2. Questions 1 to 3 are answered, so a design page can
   name its writers, constructs and marks.

Tell me your context size before you start item 2. If you are near
your cap, bring the notes up to date and stop there.

---

# Eighth: both files are approved; the three words; the old files; the landing

Both files are approved, with the three answers below worked in at
landing. Good work on the second pass. [Your commits are pushed.]

The three words

1. 1.12's first invariant. Accepted, a little wider than you wrote it,
   so a stored range of rows is covered and the next unit does not ask
   again: "Only the owner makes an id, and only for a row that exists:
   as the return of the append that makes the row, as the id after one
   it is given, or as one of a range of rows it stored. Never past the
   last row."

2. The glob sentence in 1.6. Accepted: a variant in a pattern is
   written with its enum's path, never through a glob import.

3. The expectation stays in the example.

The `as` clause

Dropped until the base layer is designed, as you say. The notes owe
the clause, the four-byte id and one more thing to that page: what the
compiler does when a table would pass 2³² rows. No limit covers source
size today.

The narrow review

Accepted as answered. One finding is bigger than its row. The unfixed
half of F2: a group is handled where it is pushed, where it is read and
where it is closed. 1.6 asks one handler a construct, 2.4 forces an
explicit stack, and under a stack every nested construct splits that
way. The old parser's 121 task kinds came from this. Record it as the
first thing the parser's page settles: how a construct that spans push,
resume and close keeps one home. When it is settled the standard says
it once, in 1.6.

The old files

Your pick, with a fence:
- Delete the three package files at landing: `IMPLEMENTING.md`,
  `REVIEWING.md` and `SMELLS.md` under `packages/symphonyscript/docs/`.
- `SMELLS.md` is not lost: move it to `.claude/reference/` as a source
  of cases for the middle. A fault in it may become a case's input,
  never an expected value.
- The habit from the package's `REVIEWING.md`, generate a finding's
  whole class before reporting it, goes on the list for the loops.
- Keep the two files under `.claude/docs/` until the runtime's rules
  have a home. Fix them at the source: `CODING.md` opens with one
  sentence that the compiler's code follows `STANDARD.md` and that the
  standard wins where they differ, comments and tests named. The root
  `CLAUDE.MD` says the same. Add "a home for the runtime's rules" to
  the notes' list of what is not yet designed.

Landing

You are near your cap, so do not land it yourself. Record these rulings
in the notes, word for word in the rulings file, and stop. The next
session's first act is the landing commit: the three answers worked
in, both files moved to `packages/symphonyscript/docs/`, the three
package files gone, `SMELLS.md` moved, the two fences written, the
package's `CLAUDE.md` fixed. Then item 2, starting with the F2
question.

---

# Ninth: the landing's review; the four rulings on the fences, 1.12, the core crate and the plan

The review was worth running. Your two fixes stand, and all four
rulings go your way. [Your three commits are pushed.]

1. `CODING.md` and the compiler's code

It does not apply at all. Change both fences: for the code of this
package `STANDARD.md` is the only rulebook, and no line of `CODING.md`
binds it or is a finding's source. "Wins where they differ" was too
weak.

2. 1.12's first invariant

Drop the list of ways and keep the condition: "Only the owner makes an
id, and only for a row that exists." A table never shrinks, so every
such id stays in range. `first`, the id after a given one and a stored
range all fit, and no fifth way needs a ruling.

Then fix one thing in the example: `read` hands back the id it was
given at the last row to mean "no more". That is one value with two
meanings (1.8). It returns nothing there.

3. The core crate

The standard binds every crate of the package, core among them. In
core, 1.12's strict rule applies: no path can panic. The runtime's own
rules that live only in `CODING.md`, the commit path that never
allocates, lock-free hand-off and integer time, stay in force for core
beside the standard until they have a home. Say this in both fences,
so no agent on core reads itself out of either file.

4. `PLAN.md`

A one-line fence now: its section on how the work runs is replaced and
binds no agent, and the plan decides nothing over the standard. The
file itself stays; a finding may still become one PLAN line. Its
rewrite belongs with the loops, item 4.

What you disclosed

- The example's overstatements: correct them in the same change as
  ruling 2, not later. Each sentence of "What to see" is either shown
  by a run or cut, "allocates nothing" by a counted run. That is P-9.
- Item 4's points, which roles the package file's "Read first" covers
  and where a GAP is sent: add them to the notes' list for the loops.
- `SMELLS.md`'s head: say in one line what "the middle" is.

What I want next

1. These rulings worked in, the example's checks run again from the
   document, and one commit.
2. Then item 2. [Your answers to its questions go here.]

Tell me your context size first. If you are near your cap, record the
rulings in the notes and stop there.

---

# Tenth: one home is ruled; the two sentences; the check moves into the repo; the example's last review; the runtime's notes

Good. The rulings are in as I gave them. Your commits are not pushed yet; push them.
Item 2, question 1: (a) and (b) are accepted

- (a) Only a production that can hold itself takes a stack; every other
  production is a plain function that reads its parts in its
  production's order. Make it mechanical: a production holds itself
  when it lies on a cycle of the grammar's productions. The page lists
  each of the 134 productions with its form, so its reviewer can check
  the classification.
- (b) A construct the stack splits is one type, the entry it leaves on
  the stack, with one `impl` block that holds open, resume and close,
  its mark above the block. The stack's enum has one variant a
  construct, never one a place to resume at.
- The three alternatives are refused: one handler a resume point is the
  old form; rows of data under one interpreter is a parser library by
  another name (1.10); recursion under a depth limit reopens 2.4.
- One condition: the first unit carries a clock row beside the old
  parser's 128 ns a node on wb2, so this form's cost is known before
  the rest is built on it.
- The standard gains its sentence in 1.6 and in 2.4, and the example's
  group becomes a type with its block.

Item 2, question 3: two expectations

Accepted, your pick. A run of expectations ends in a fault drawn by
name, never in a default that accepts. Say that in 1.6.

Item 2, question 2: the two sentences

Accepted as worded, with two word fixes.
- 1.6: "What it remembers between two entries is held in the type's
  own fields."
- 2.4: say what a machine is in the same sentence: one loop with its
  own stack. The standard uses the word nowhere else.

Item 2, question 4: the changed example's review

Yes. It is the one ruled under "The example's reviews" below: one
narrow cold review, then the example is frozen.

Your new questions

2. The check moves into the repo, in a folder beside the standard. It
   is run by hand after any edit of the standard and becomes a gate job
   when the gate is built.
3. Reword the first sentence to the package's code in every crate.
4. Fix items 3 and 4 in the same edit as the group: a narrower type so
   no impossible code can be named (1.11), and one constructor for
   `OperandExpected` (1.4). Items 1, 2 and 5 are answered by the ruling
   above, the counters' open point and the gate's mark script.

The example's reviews

It has had four, and the form in the repo has been read by none. After
the edit above, one cold review of the good form against parts 1 to 4,
then it is frozen. A later finding becomes a line in the notes, fixed
when the example is next touched.

Order

You are at 529,000 of 700,000. Do the example's edit, its check and
its one review in this session, then bring the notes up to date, give
me the handoff prompt and stop. The page itself, the colon question
first, is the next session's.

For the notes, under "a home for the runtime's rules": checked against
the SPEC on 2026-11-15. No allocation on commit is already §1.1 and
§12.6. Lock-free hand-off is not in the SPEC (no threads; §12.1 has
only "the runtime never waiting") and its "refuses when full" fits a
channel only, not a stream or a state. Integer time has no subject:
§12.2 gives the runtime no clock, and time values are seconds as
floats. The owner decides at the runtime's design whether the third is
kept at all.

For the notes, under "Not yet designed": the runtime's build design
(buffers, the commit, the host's hand-off of writes, reload), owed
before M3 is built. A sketch of what the runtime needs from the Program
is owed earlier: before the open question "what the compiler hands the
runtime" is settled, so that the Program's shape is not fixed by the
compiler's side alone.

---

# Eleventh, 2026-11-15: two orders on findings against the standard, relayed

Relayed to the primary by the supervisor from the owner's chat on 2026-11-15. The context is the supervisor's, in its words: the owner brought it three findings another reviewer made on `STANDARD.md` and `FINDINGS.md`; it checked each against the standard's text and put its reading to the owner; on two of them the owner ordered a record, and the third, that no approved document states the test reviewer's check, is still a question with the owner. The primary checked the three quoted phrases of the standard against its text before saving this.

(a) The finding: line 3 of the standard binds "every crate", while 4.8 defines a stated test as "the program, the expected tree or the expected codes and places", a compiler's shape, and says nothing of a runtime's or a tool's case file. The supervisor's proposal: one line on the list of what the standard owes, that 4.8's case format for a non-compiler unit is its page's, settled with the runtime's design. The owner's words:

"add it"

(b) The finding: 3.1's "before it opens its tests" is a behavioural rule while the frozen tests sit in the builder's checkout. The supervisor's proposal: the loops and the gate enforce it, the frozen stated tests reaching the builder's branch only after the structure proposal and the breakdown are accepted, and the test author and the builder's breakdown running in parallel from the approved page. The owner's words:

"do it"
