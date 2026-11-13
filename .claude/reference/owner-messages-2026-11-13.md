# Two earlier messages of the owner, 2026-11-13, word for word

Saved by the peer session. The owner's last message is in `owner-brief-2026-11-13.md`. Where a later message changes an earlier one, the later stands.

---

# First: the reference machine, the targets, and no choice yet on the parser

The last plan’s targets were never meant for this host. The old process judged them on the second host, wb2, which is much powerful machine. The old notes say this host runs those figures 1.2 to 2.1 times slower and that a time taken here must never be compared with a target.

So I would not pick A, B or C. I would pick a fourth: Re-run the floor on the second host (wb2). The gate’s speed rows will run there anyway, so that is the machine a promise should be stated for. It is one cheap run, which fits the rule you just wrote down.

By the old ratio, lexing and parsing would land at roughly 0.55 to 1.0 µs a call there. That is my arithmetic, not a measurement.

A deeper problem with the targets themselves
The two old targets disagree with each other.

* The project target is 1 µs per syntax node. The front end uses 0.48 of that, so it fits with half to spare.
* The call target is 1.5 µs for about 5 nodes, which is 0.3 µs per node. The front end cannot fit that on any reading.

One of them is wrong, and agents wrote both. I would restate the promise in terms a user feels, for example “a 10,000-line project checks in under N milliseconds on machine M”, and derive the per-node budgets from it. Today that project’s front end takes 31 ms on the slow host. That promise is a product call and it is ours.

Decision 2: repair or rewrite the parser. I would not choose yet.
Your case for repair is real: layout, lexing, precedence, the lossless tree and robustness are proven on 700,000 inputs. But the repair list is long:

* twelve refusal families, five wrong views and two quadratic paths;
* recovery rules that the new error policy changes anyway;
* two constructs that need a structural change;
* no test that states a tree.

That is close to the reasoning that made us stop repairing the engine in place. Two things decide it, and both are cheap:

1. We should settle the thirteen SPEC gaps and the error policy first. They define what the parser must do, so decision 3 comes before decision 2.
2. You should size the repair: each finding mapped to the code it touches, with the lexer and layout counted apart from the parser, the recovery and the views.

My rule of thumb: keep the lexer and layout either way. If the repairs touch more than about half of the rest, rewrite that part against stated trees. The 700,000 inputs stay as the robustness net, so a rewrite re-earns that proof by running them.

Decision 3: agreed, bring the thirteen gaps next as one batch.
Housekeeping

* The same rule now sits in two places. The designer wrote “wait for the cheap measurement” both as a preference of yours and as lesson P-7. That is how the old duplication began. I would keep the lesson and remove the preference, since the rule came from my advice and not from you. Keep the preference only if you want it as your own rule.
* One reviewer wrote its report through the shell after the harness blocked the write. The result is harmless, but agents working around a block is a habit worth stopping. Reviewers should be briefed to return text.
* The bench check it added was the right call. It caught three errors before the figures reached you.

In plain words: the foundations are sound but the parser needs real work, and the speed figures were read against the wrong machine. For example, 1.1 µs a call looks fatal against 1.5 on this host, and may be half that on the machine the target was written for.

---

# Second: the four rulings, and the trial that decides repair or rewrite

The numbers lean towards repair, with the recovery part replaced, but not so fast. and I agree with your pick on all four rulings. I have two conditions and one default I would change.
Speed
My arithmetic held on the right machine: lexing and parsing take 0.54 to 0.58 µs a call and 0.13 µs a node. The parser is not the speed problem, so the speed trigger for reopening it is off the table.
The four rulings

1. Readings that depend on what a name names: A. SPEC already works this way in all three places.
   * §7.1 defines a wire line as one "whose Kind names a wire kind … or a pattern that stands in for a wire".
   * §6.8 says the first bracket list is arguments "after a generic type's name".
   * §5.3 says a name after `is` is a type test "when it names a type".
So A changes no program; it states once what the SPEC already says three times. The cost is that nobody can tell a wire from a node by sight in `Soft: wash`, and neither can a plain syntax highlighter. Only the language server can.
2. Keyword as an attribute name: A. It needs one token of lookahead and does not depend on the expression grammar.
3. A syntax fault's region: A, but know the editor cost. While a header line is broken, its whole body goes dark. That means no errors from inside it, and every name declared in it becomes a placeholder. This happens constantly while you are typing a header. The upside is no flood of false errors at that moment. Moving from A to B later only adds messages, so A is the safe side to start on.
4. Unclosed bracket: A, with one condition. Today's rule is one of the two quadratic paths. Question: How does A stay linear when many brackets are left open, and pin it with a cost test. A bracket is only known to be unclosed at the end of the file, so a careless design re-reads the rest of the file once per bracket.

The defaults
Each cites its SPEC basis and they read correctly. I would change one:

* Byte order mark. The default refuses it "as today". Some Windows editors write one at the start of every file, so a user would get an error on a file that looks empty of faults. I would skip a leading mark silently and refuse it anywhere else. Your SPEC does not mention it, so this is yours to rule.

One thing to confirm
The "placeholder rule's edges" all rest on the earlier choice that a broken thing silences whatever reads it. If we have not ruled that in words yet, we should rule it now, because seven defaults stand on it.
Repair or rewrite
By the sizing:

* Grammar: 27 to 33% touched. Repair.
* Tree and views: 4%. Repair.
* Recovery: 67% touched if replaced, and mostly deletion (about 715 lines out, 200 to 360 in). Replace it to rulings 3 and 4.
* Lexer and layout: about half of 823 lines. It is small either way.

That is under my half-way rule for everything except recovery. Two caveats: the sizing is an upper bound from reading, with nothing run, and 2,963 lines of tests encode today's recovery and get rewritten.

But, when I said "not so fast" - I meant that - there is also another angle: I measured the repair by lines to be written, and that misses another point: for an agent, reading unfamiliar code well enough to change it safely is the expensive and risky half, and my line counts say nothing about it.

What this angle adds in favor of rewriting:

* Reading to modify is where agents go wrong. The parser carries unwritten rules: the tree must reproduce the source byte for byte, recovery must stay linear, and deep nesting must not grow the stack. An agent patching one function can break a rule it never saw. Writing fresh against a design page and tests has no such hidden rules.
* A repair is a poor first trial of the new process. The new rules are built for “write to a design, tests first”. A repair unit mixes in archaeology, and there is no design page for code the old process wrote.
* The end state differs. After a repair the parser is of mixed origin, with 2,963 lines of old tests rewritten. After a rewrite every line was written under the new rules.


What still favours repair

* The size is readable. The old engine failed partly because nobody could read it whole: 39,000 lines and one object with 253 fields. The parts in question here are 3,600 lines of grammar and 800 of lexer. A builder reads those whole in well under a tenth of its room.
* The hard properties are proven and executable. Layout, precedence, the lossless tree and 700,000 broken inputs without a crash all exist as runnable checks. A repair that breaks one is caught at once. A rewrite must earn each again, and the last rebuild’s estimate was wrong by 2.5 times.
* The parser is written in an unusual style. It avoids recursion so that deep nesting cannot overflow the stack. That is hard to write fresh and get right.
* The defects are missing pieces, not rotten structure. They cluster in constructs nothing downstream ever used. But nobody has yet judged whether the code is clean to modify, only whether it behaves.


Cost and time

* Common to both paths: the stated-tree tests come first, and recovery is replaced.
* Repair: about 750 to 1,300 lines to write, and about 6,600 lines to read closely.
* Rewrite of the grammar: about 3,600 lines to write, plus re-earning the proven properties.
* Time: I cannot give you an honest figure for either. A guess here would be the mistake the last peer made.


Where that leaves my position
I no longer say “repair” on the strength of the sizing. The question is whether agents can repair this code safely, and it can be measured cheaply before deciding:

1. One bounded trial. One builder takes the hardest structural fix, the neutral shape for a line like `Soft: wash`, with its stated tests written first. It works on a throwaway branch under a fixed budget of one session.
2. Read the outcome.
   * If it lands cleanly, with the proven checks still green, repair is viable and the trial’s cost predicts the rest.
   * If it sprawls or breaks the proven checks, rewrite the grammar to the design.
3. Either way, keep the tree and views (4% touched) and the small lexer, unless the design pass changes what the parser must hand on. That design question is still open and could tip this by itself.


My two conditions for starting the repair (both - the trial and the full-scale repair if we later decide that):

1. Tests first, by a different author: one stated tree per grammar alternative before any code changes. This is the first real work unit under the new rules, so it is also their first trial.
2. The loops and the agent definitions must exist before it starts. I raised this last time and it is not in this report. The repair also needs its own budget in rounds, with a stop that comes to you.


Housekeeping
The two new lessons, P-8 and G-14, are sound, and the duplicate preference is gone. Four lessons were added in one day; each is real, but that pace is worth watching.
In plain words: keep the parser, fix its grammar gaps, and swap its error recovery for the simpler rule. For example, a typo in `Rig stage | hue=:` would show one error and hide the block under it until fixed, where today it sometimes shows nothing and sometimes one error per line.
