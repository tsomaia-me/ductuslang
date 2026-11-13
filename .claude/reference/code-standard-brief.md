# The code standard: the brief for the conversation with the owner

Written by the supervising peer session on 2026-11-13 from what the owner settled that day. It is the agenda of owed item 1 in the notes. It is not the standard and no part of it is a rule yet.

## What the standard is

- The owner's bar for every line of the compiler's code, as the SPEC is the owner's bar for the language. The owner sets it; the peer brings options, reasons through them and says which it prefers and why.
- Self-contained: a builder learns how to write good code here from the standard alone, never from existing code, and no rule leaves the builder to judge what is good.
- Four parts: the architecture principles, the cost discipline, the method from a SPEC sentence to code, and one worked example inside it.
- A rule earns its place only if a strong model would get it wrong without it. General Rust hygiene a strong model applies unasked stays out, because it buries the rules that matter. The standard must not undershoot either: the acceptance table below is the guard.
- It shapes the builder and does not only fence it. A builder's scope is bounded by its design page; inside that scope its bar is high.
- No cap on a stand-in for quality: no limit on a function's or a file's length, or on a count of parameters or fields. A budget on the product's own promise (work that grows in line with the input, allocations a node, a time on wb2) is the requirement itself and stays as a test.

## How the conversation runs

- One topic a message, in the order below. For each: the candidates, what each prevents and costs, the peer's pick.
- What the owner settles goes into the notes under Settled, one line a principle.
- No file of the standard, no rules file, no agent file and no loops file is written until the owner says the five topics are settled. The draft is then written from the settled lines alone.
- Nothing of the old `CODING.md`, the old `REVIEWING.md` or the two refused draft sets is a base or a model.

## The topics

### 1. Architecture principles

The supervising peer's first candidates, which the owner liked as a start. Open the conversation with them: which to strike, change or add, and options on the contested ones.

1. A stage is a function. It reads what earlier stages built and returns its own result, which nothing later changes. Prevents the one object every stage writes.
2. Dependencies point one way. A stage uses only earlier stages' results. Today the parser imports from the old engine.
3. No ambient state. What changes a function's behaviour is passed to it. Prevents the mode switches set in 85 places.
4. One rule, one implementation. A SPEC rule lives in one place that every form of it goes through. Prevents the unit bug, where `min` and `+` took different paths and the engine disagreed with itself.
5. A fault is a value. A placeholder travels as data, and no stage asks a global "was there an error".
6. Every construct is handled or refused by name. No fallback path and no silent default arm.
7. A module's public surface is what its design page names; everything else is private. Prevents 1,100 functions hung on one object.

### 2. Cost discipline

To settle:
- The cost model a builder holds while writing: what work a syntax node and an expanded item may cost, and what is counted (passes over the input, allocations, hash operations, copies of text).
- The data layout: typed ids, dense tables in an order the source fixes, interning, and what is never keyed by a node or an address.
- What a builder owes at hand-back: the cost of each loop over something the source sizes, and whether it runs its unit's speed rows itself before READY.
- Where a stage's budget comes from (the thin slice) and how a builder reads it.

### 3. The method from a SPEC sentence to code

To settle:
- What a builder does with a sentence before it writes: its conditions, and every form it covers.
- How a builder decides where the standard is silent: the criteria, in their order.
- What a builder does when the page looks wrong: it objects with a better design; it neither builds what it holds wrong nor changes the page in code.

### 4. Comments and tests in code

Settled already:
- A comment states what the code cannot: an invariant it relies on, the reason for a choice that looks odd, the SPEC section it implements. It never restates code and never tells history.
- A module opens with a few lines: its job, what it takes and hands on, its invariants.
- A comment that restates code is a finding, and so is a relied-on invariant nobody wrote down.

To settle:
- Whether a builder writes tests of its own beside the frozen stated ones, and what keeps them few, real and fast.

### 5. What a reviewer may report

Settled already:
- A finding on design is admitted when it cites a principle of the standard and shows a concrete consequence: a cost figure, an input, or a change the structure makes hard.

To settle:
- What a reviewer must show before its findings count: how the unit works, in its own words.
- The verifier's gate, reworded so that it admits such a finding and still rejects a preference.
- That slow by a constant factor and badly structured are classes of finding.

## After the five topics

- The worked example: the owner chooses the construct; the peer drafts it, good form and bad.
- The acceptance table: one row a root cause, with the sentence of the standard that prevents it and the reviewer step that catches it. An empty cell is a gap in the standard.
- One cold review of the draft against the table, then the owner approves.

## The root causes the table must cover

From `compiler-review.md`, `syntax-conformance-review.md` and `syntax-recovery-review.md`; read those for each row's detail.

The first engine:
- slow by a constant factor: 22.8 µs an inlined call, 30.6 µs an expanded item;
- one engine object of 71 fields with about 87 memo maps;
- native recursion and derived walks over source depth: 39 s on a 100,000-deep literal;
- an internal error on a valid program.

The second engine:
- one object of 253 fields that every stage changes, with 1,100 functions on it;
- mode fields that change what other methods do;
- one function of 938 lines, and one number that means two things by its range;
- IR nodes read as bare number triples;
- two paths whose work grows with the square of the input;
- the dense layout fitted on afterwards: 79 hash maps first, 18 allocations an inlined call pinned by its tests;
- a valid program refused and a wrong one accepted, by a rule built once a form and argument position;
- the same message text written in four places;
- 189 sites that decline a program to a fallback;
- tests that state "the same as the other engine" and no expected result;
- no comments by rule, so each session relearned the code.

The lexer and the parser:
- a recovery rule that fires on valid programs;
- typed views that answer wrong, reading children by position;
- a lexer that decides from the word before, a fact it does not have;
- valid forms refused in constructs nothing downstream exercised;
- no test that states a tree;
- imports from the old engine.
