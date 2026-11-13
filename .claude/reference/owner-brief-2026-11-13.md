# The owner's brief of 2026-11-13 for the three owed items

The owner's message, word for word, saved by the peer session so that the agent files, the design page and the §16 batch are written from the owner's words and not from a paraphrase. Its item 1 and the two deletions are done; the rest is the brief.

---

Good work. Item 1 is approved with one condition. Items 2 and 3 come back
to me once the gaps below are closed; none is large.

1. The SPEC sentences and `json`
- The sentences outside §16 are approved. Commit them once the condition
  below is met.
- The `json` ruling is taken: the word opens JSON text by the tokens
  beside it alone, and a dimension straight after it is written `(json)[2]`.
- Condition: state that rule once, for every place a type stands
  (annotation, parameter, field, type argument, match label, after `is`).
  The draft has it only in §5.3; that sentence then points to the one home.

2. The two-pass bracket plan
- Approved, with one hole to close. "A closer pairs with the nearest
  opener of its shape" needs a search down the stack. Many open `(`
  followed by many stray `]` search the whole stack each time, which is
  quadratic again. Keep a count of open brackets per shape so a stray
  closer is known at once.
- Add that mixed shape to the cost test: many openers of one shape, then
  many stray closers of another.
- Add one example to the §16 batch: an unclosed `(` on line 3 that an
  unrelated stray `)` on line 500 closes. Show what draws and what goes
  dark.

3. The old agent files
- The old ticket-and-PR layout is dead here: no PRs, no tickets, no
  ceremony. The loop does the work and it goes to main.
- Delete the old files under `.claude/agents/` and the head and consultant
  skills before you write anything new. Do not use them as a model; they
  would anchor you wrongly.
- Do not assume `.claude/docs/CODING.md` and `REVIEWING.md` fit. Read them
  and say what you keep, change or drop.

4. The loops: four gaps
- The verifier lost its job. It admits or rejects each finding by stated
  gates before a builder sees it: does the finding quote a SPEC sentence
  or a design-page invariant, and does it reproduce? Counting rules
  passing is a script in the gate, not an agent. With that job the
  verifier runs on Opus at high. The other models stand as you picked.
- A frozen test that is wrong has no path. Add a dispute line: the builder
  quotes the SPEC sentence the test contradicts, it goes to the test
  author, and the round is not spent on it.
- "Shown to fail once" is undefined for kept behaviour. Say who does it
  and at what grain. For whole-tree comparisons one deliberately wrong
  tree proves the harness; breaking the code per test is for tests with
  weaker checks.
- The reviewer's directions and the coding guidelines carry the quality,
  and so far they are named, not shown. Write the agent files as drafts,
  have them cold-reviewed, and bring me their text. No agent starts
  before I approve them. The reviewer is not told the tests passed.
- Also name the mechanism by which the gate refuses a builder commit that
  touches a frozen test.

5. The trial
- The budgets are approved: one builder session and two fix rounds for
  the trial, three fix rounds a unit for the repair.
- The pass lines are approved with these fixes:
  - Record a baseline first. Many stated trees fail on today's parser by
    design. "Every earlier tree still passes" means those that passed at
    the start.
  - The 320-line stop is your measurement. It is not in the builder's
    brief; a builder told a number writes to the number.
  - A round lost to a wrong test or to a gap in the design page is counted
    apart, so a fault of the new process does not read as "rewrite".
- I change one condition of mine. The SPEC has 134 productions and about
  300 alternatives; do not write all the stated trees before the loop has
  run once. First the trees for §7 to §9 (placements, wires, gates), then
  the trial, then the rest before the full repair.
- The design page for what the parser hands on comes first, as you say.
  It must settle the text after the colon before the trial: `Soft:
  Lamp/0.5` reads as a child or as a division, and `Soft: [a, b][i]`
  starts like a bundle.

6. §16
- Narrow one rule. Counts silent across the whole program while any
  region exists contradicts §16.2's own sentence, that a fault which reads
  no placeholder still draws. A child-count fault in an untouched file
  would never show while I type in another. Silence only the parents and
  kinds a region or a placeholder could change.
- "Every item on a faulty line is a placeholder" widens my ruling 3. I
  accept it.
- The other four rules you added stand: the line's extent, the nearest
  opener of its shape, the mis-indented line's region, a tab as one
  column.
- For your pick on which names a region turns into placeholders, add the
  case of a region at top level. There "the bodies that enclose it" is
  the whole program, which brings the broad reading back.
- Bring the rest of §16's edges as one batch with examples, as planned.
  The trial does not wait on them.

What I want back, in this order:
1. The old files deleted, and the SPEC sentences committed.
2. The loops revised, and the agent files' text after its cold review.
3. The design page for what the parser hands on.
4. The §16 batch.

Still on my word only: the old commit loop (pid 1260982) and `minus.txt`.
