# Two cold reviews of the SPEC §16 draft, 2026-11-13

Each by one agent on Fable at high effort, given the edited SPEC, its writing rules and the rulings the edit must state; each read and ran nothing. Saved word for word by the peer session. The draft they read is at `/home/cdagent/cargo-targets/peer-syntax-review/SPEC.full-draft.md` in its state after both; the first round's fixes and the second round's findings 1, 3, 5, 8, 11, 13, 14 and 15's first point are applied there, the rest are open. In the examples ⏎ is a line break.

---

# Round 1

## Verdict

The edit is not ready to commit. Rulings 2, 4 and 7 are stated faithfully; ruling 1 is widened by a fourth case, ruling 6 loses its tab half, and ruling 3's limit edge says something different from the ruling. The §16.3 example counts three errors by the document's own rules, not two, and §16 leaves at least a dozen programs where two test authors would expect different message counts.

I read the whole edited SPEC and the writing rules, and ran nothing.

## A. Rulings misstated

1. **Ruling 1 widened (§6.5).** "…and a `match` label that is a name is a variant or a type by its scrutinee (§5.4)." The ruling names three places; this is a fourth. It also contradicts its own lead sentence ("only what a name names tells them apart, the name decides"), because here the scrutinee decides. Fix: delete the clause.
2. **Ruling 1 timing dropped (§6.5).** "the name decides, and the text is read as that form" never says the reading is fixed when the name resolves, and says nothing for a name that resolves to nothing (see C1). Fix: "…the reading is fixed when the name resolves; a name that resolves to nothing fixes neither reading, and nothing after it on the line is checked (§16.2)."
3. **Ruling 6, tab half missing (§16.3).** Only the mis-indented line is stated. Nothing says a tab used for indentation is one fault with one message, what column that line has, or what its region is (see C9). Fix: add "A tab in a line's indentation (§2.2) is one fault at the tab; the line takes the column its first token would have with the tab counted as one column."
4. **Ruling 3, "a limit's stop stands" changed (§16.2).** "A limit's error stands as §4.6 says, whatever placeholders the program holds." This reads as "the error is still drawn". The ruling is about the stop, which is the exception to §16.1's "Every fault draws its own message"; without it §16.1 contradicts §4.6 "stops the compilation". Fix: "A limit's error that stops the compilation (§4.6) stops it here too, and no fault found after it draws."
5. **Ruling 5 half implied (§16.3).** "No list of words decides where a bracket ends" is nowhere stated; "A bracket that nothing closes" only implies it (see C6). Fix: "A bracket ends at its closer and nowhere else."
6. **Additions outside the rulings.**
   - §16.3 "A closer that closes nothing is a fault at the closer."
   - §16.3 "unless a declaration outside every region declares it."
   - §16.2 "what it would have yielded is a placeholder in turn."
   - The last two are needed for the rules to work; the first is a new rule the owner did not rule.

## B. Contradictions

1. **§16.3 against §16.2.** "What stands before the fault on its line is read as usual" against "what it names is a placeholder". In `Rig stage | hue=:` either `Rig stage | hue` is read and checked, or `stage` has no type and nothing of it is checked. Fix: "What stands before the fault on its line is parsed; its item is a placeholder whose name is declared (§16.2), and nothing else of the item is checked."
2. **§5.3 "fits both forms".** "A right operand that is `none`, or a name or a qualified name with bracket lists or without, fits both forms… so `x is Pair[i32, bool]` tests a type". `Pair[i32, bool]` fits no `Fallback`, because `i32, bool` is no `Index` (§5.5), so the example is not a case of the rule. Fix: "…may fit both forms" and use `x is Grid[4]` against `x is levels[0]`.
3. **§2.2 against §2.6.** "Inside `( )`, `[ ]` and `{ }`… newlines are whitespace" now covers an interpolation's `{ }`, while §2.6 says "A string occupies one line". Fix: "`{ }` outside a string".
4. **"Bracket" means two things.** §3.11 separates "its braces and brackets"; §2.2's new "a line that begins inside brackets" and §16.3's "A bracket that nothing closes" (whose example is a `(`) mean all three pairs. A reader of §3.11 takes a tab on a line inside `( )` as an error. Fix in §2.2: "a line that begins where layout is suspended"; in §16.3: "An opening `(`, `[` or `{` that nothing closes".
5. **§2.2 against the `Char` production.** "one anywhere else is an error" forbids a byte order mark inside a string or a comment, which `Char` (§2.6) and §2.3 admit. Fix: add "inside a string or a comment included".

## C. The example that does not hold

**§16.3, "This is an error twice".** `Rig` declares no attributes (§1.3), so `hue` on `Rig stage` breaks §7.6 ("Setting an attribute the kind does not declare is an error"). That check needs only the kind and the entry's name, both "before the fault… read as usual", and reads no placeholder. A careful reader counts three. Fix: `Scene dusk | hold=:` with `Cue/0.5` and `Cue/ | fade=` under it, and settle B1.

Both §16.2 examples hold as counted (2 and 1).

## D. Programs two readers count differently

1. **Same unknown name used twice:** `Rig stage:⏎ Lamp/glow⏎ Lamp/glow`. Two (each use resolves to nothing) or one ("nothing is drawn for what another fault left unknown"). The duplicate bullet settles uses; nothing does for undeclared names.
2. **One faulty line expanded many times:** `Rig chase:⏎ for i in 0..8:⏎  Lamp/(true + i)`. One or eight. §6.3 and §6.4 check "in place" per use; §16.1 says one fault, one message. The same holds for a faulty function body called three times.
3. **Unknown kind before a colon:** `Scene dusk:⏎ Trigers: night`. One, or two if "under any other kind it is the placement's body" makes `night` a child whose kind names an instance.
4. **Is the unfinished entry set?** `Kit drums:⏎ Sampler | file=`. One, or two with "`file` has no default" (§7.6). Same root as section C.
5. **Fault inside a bracket that closes on a later line at column 0:** `let xs = [1 +,⏎2,⏎3]`. One, or three: lines 2 and 3 are not "indented under", so `2,` is a fresh fault and `]` "closes nothing".
6. **Closer of another kind:** `let x = f(a, [b, c)`. The message sits at `[` (if `)` closes `(`) or at `(`. Also `state gain(1.0⏎state trim(0.5))`: one at the second `state`, or two (the opener and the stray `)`) for a reader who ends a bracket at a keyword.
7. **Name on an earlier line than the fault:** `state band(Band {low: 0.2,⏎ high: })` then `…target=band.low`. "on its line" does not cover `band`: one, or two with "`band` names nothing". The same holds for an `as` name before a faulty clause-continuation line.
8. **Lexical faults:** `Rig stage:⏎ Lamp | level=5foo hue=tint`. If a bad suffix is "a fault of the grammar… (§2)" it has a region and `tint` is silent (one); if it leaves a value placeholder, `tint` draws (two). The same holds for `==`, `;`, `0b12`, a stray `^` and a lone CR.
9. **Tab line:** `Rig stage:⏎<TAB>Lamp/0.5⏎    Lamp/typo`. One if the column-4 line is "indented under" the tab line, two if not.
10. **Mis-indent:** `Rig stage:⏎        Lamp/0.5⏎    Lamp/0.2⏎    Lamp/0.3`. "keep the columns they had" suggests two (line 4 also lands on no column); "one mis-indented line" suggests one. Fix: "the stack is as it was before the line".
11. **`else` after a faulty `when`:** `when alarm_on +:⏎ Valve/0.3⏎else:⏎ Valve/0.5`. One, or two with "`else` without `when`".
12. **Name declared three times:** `let a = 1⏎let a = 2⏎let a = 3`. One or two.
13. **Fit beside a placeholder:** `let base = 48 +⏎Clip melody:⏎ Note/(base + 7.5)`. One, or two if the float in an `i32` position reads no placeholder.
14. **Top-level counts:** `Session a | tempo=:⏎Session b`. The counts bullet names "a parent"; whether "at most one `Session`" is checked is open.
15. **More count cases the bullet does not reach:**
    - a destination's incoming minimum when the only wire to it has a placeholder destination;
    - a parent's minimum when all its children sit in a region ("nothing… placed" leaves no placeholder child);
    - an unexpanded `for`.
16. **Warnings:** `Session live | tempo=:⏎Track lead`. Does "no entry-kind instance" (§10.2) draw? Whether an absent placement is "read" is open.
17. **Scoped names in a region:** a child `as wash` inside a region with a top-level `let wash` outside. `Dims: wash` is silent, or draws against the `let`.
18. **Where the message sits:** "A block indented under a line that opens none is one fault" names no position. Fix: "at its first line".

## E. Writing rules

1. **Each fact in one place.** The new §6.5 paragraph restates §7.1 (wire line), §6.8 (first bracket list), §5.3 and §5.4. Fix: keep the general sentence and cite the four sections without restating them.
2. **Undefined terms.** "Fault", "message" and "draws" appear only in §16; everywhere else the document says "error" and "diagnostic". §16.1 should define them: "A fault is one breach of a rule this document calls an error; its message is that error's diagnostic." D1 and D2 depend on this.
3. **§16.2 bullet 3** "a range bound or an argument that decides what a use places among it" does not parse. Fix: "Structure that a placeholder decides, a `for` whose range bound is one or a use whose argument decides what it places, is not expanded…".
4. Nothing else found: tense, rule before example, no other language and no history all hold in the new text.

## Not checked

- `ARCHITECTURE.md`.
- `docs/pending-examples.txt` and `docs/pending-productions.txt`.
- The example test and the compiler's actual output for any program above.
- Whether the rulings as given match the owner's original words.

---

# Round 2

## Verdict

The edit states all fourteen points and nothing foreign to them; point 9 is weakened and point 14 widened. All three new `sym-error` examples are rejected for the reasons and counts their text gives, and every cited section exists and supports its citation. It is not yet enough to test from: the programs below give two careful readers different counts or different silent sets, mostly where two rules meet on one line.

## Findings, most serious first

1. **§16.3, "A fault of §2's rules or of a production is a syntax fault."** The text never says at which token a production fault stands, and "its line" depends on it. In `Rig stage:` followed by `Scene dusk:` at column 0 with a block, the token that fits no production is `Scene`. One reader puts the fault on line 1 (`stage` a placeholder, `dusk` sound); another on line 2 (`dusk` undeclared, its block unread). Fix: "A syntax fault stands at the first token that continues no production; when that token is a layout token or begins a line, it stands at the end of the line before."

2. **§16.3, "a name written in it is a placeholder at every use in the program."** Point 9's clause "whether or not something outside the region declares it" is missing, and "a name written" is undefined. In the section's own second example the region writes `Cue` and `fade`. By the letter every `Cue` in the program is then a placeholder, so `Cue/(true)` in another scene is silent; a second reader takes only names the region would declare. Fix: say which — e.g. "a name the region would declare (a declaration's, an `as` name, a parameter) is a placeholder at every use in the program, whether or not a declaration outside the region has it" — or state outright that kinds, attributes and member names count.

3. **§16.3, "A tab in a line's leading whitespace is one fault at the tab, and counts as one column."** With the stack at [0, 4], a middle line `\tLamp` breaches the tab rule and, at column 1, "lands on no column of the stack". The same double breach arises on a clause's continuation line: a tab, or §2.2's wide-character miss, plus the fourth bullet's "block indented under a line that opens none". One message or two; ruling 6 says one. Fix: "A line that draws a tab fault or §2.2's column fault draws no other indentation fault."

4. **§16.3, "The region is not read", against "An opener left open … is a fault at the opener."** Take `Lamp/(f("abc))` with the string unterminated: the string swallows both `)`. Readers get one message at the string, one at the first `(` (whose region covers the string), or three. The text does not say whether a closer inside a region still closes an opener before the fault, as in `f(1 + , 2)`. Fix: "Brackets are matched before any region is taken, a region's closers included; of the syntax faults on one line the first in the text draws and the rest lie in its region."

5. **§16.3, "The item its line writes … is a placeholder. … nothing else of the item is checked."** Two gaps:
   - In `Lamp/(glow) | hue=`, `glow` stands before the fault. §6.5 separates "names resolved" from "checked", so one reader draws `glow` (two messages) and one does not.
   - In `Valve/0 Valve/(glow) (Controller/0.4 | gain=)` the line writes three items; "the item" does not say whether the first two are placed and checked.
   - Fix: "Every item its line writes is a placeholder; no name on the line is resolved, and nothing of the line draws but the fault."

6. **§16.2, "not drawn for a program that holds a placeholder placement or wire, a region (§16.3) or structure that is not expanded."** Three things are open:
   - "Placeholder placement" is undefined. In §16.2's own example, is the `Lamp` with placeholder attributes one? That decides whether the no-entry-kind warning draws.
   - A fault at a line's end, as in `let base = 48 +`, has an empty region: does the program "hold a region"?
   - Naming only §10.2's counts implies §6.4's share counts and `->` contract and §7.8's "at least one member" still draw when a region removed the line they count.
   - Fix: "…for a program with a syntax fault, a placement or wire whose kind is a placeholder, or structure that is not expanded; no count of any rule is drawn there."

7. **§16.2, "stops it here too, and no fault found after it draws."** The document gives no order of finding, so the surviving messages cannot be listed (a syntax fault in a later file, a type fault elsewhere). "Here too" and "found after" are also process words. Fix: name the set, e.g. "every syntax fault and every name that resolves to nothing draws; no other fault does."

8. **§16.3, "Its region also holds the lines after it at its column, with the blocks under them, and the stack after the region is as it was before the line."** With the stack at [0, 4, 8], a line at column 6 is followed by a line at column 8. One reader takes it as a block under the faulty line (unread); another as a return to the column-8 block (read, and its own faults draw). Fix: "…holds every following line indented at or deeper than its column, up to the first line indented less."

9. **§16.3, the same first sentence, is too wide and has a hole.**
   - Too wide: §2.1's prelude-path clash is "a fault of §2's rules" and has no line. §2.6's undeclared unit suffix and the 2¹²⁷ literal become syntax faults that unread a block; say whether that is meant.
   - Hole: a byte order mark or lone CR on a comment-only line. That line emits no `NEWLINE`, so "the text between two `NEWLINE` tokens" is the following code line, whose item and block go unread; a second reader gives the comment no region.
   - Fix: cite §2.2–§2.6, and add "a fault on a line that holds no token has no region."

10. **§16.2, "a use whose argument decides what it places, is not expanded."** For `Arp/48 | steps=glow`, is the whole use unexpanded, or only the inner `for`? That decides whether a fault elsewhere in the body that reads no placeholder draws. A `for` over a placeholder array is not listed. Fix: "A `for` whose iterable or bound is a placeholder is not expanded; the rest of a use that holds it is."

11. **§6.5, "A name that resolves to nothing fixes neither reading, and what the reading would decide draws nothing."** Point 14 says "a kind name"; the edit covers all three places (widened; harmless, but the owner's to approve). It is also not countable: in `Soft: Lamp/(glow)` with `Soft` unknown, `glow` is a name under both readings — one message or two? Fix: "…and nothing after the colon, in the bracket list or in the right operand is resolved or checked."

12. **§16.2, "A fault leaves a placeholder where the faulty name, type or value would stand."** For a fault of legality the placeholder is not identified. `let x = reading + 1` (a `let` reading a cell) then `Note | accent=x`: is `x` a placeholder (one message), or known and mistyped (two)? Fix: "The expression or declaration a fault's diagnostic names is the placeholder."

13. **§6.5 against §16.2.** §6.5 says "A duplicate is an error naming both declarations"; §16.2 says "one message, which names every declaration". With three declarations §6.5 reads as two errors. Fix §6.5 to "naming every declaration of the name" and let §16.2 only cite it.

14. **§2.2, "Inside `( )`, `[ ]` and, outside a string, `{ }`".** The qualifier binds to braces only, so by contrast a `(` in an interpolation suspends layout, against §2.6's "A string occupies one line". Fix: "Outside a string, inside `( )`, `[ ]` and `{ }` …". Likewise the BOM is an error "inside a string or a comment included" while the CR sentence is silent on both.

15. **Writing rules.**
   - §5.3 "may fit both forms" is a hedge; write "fits both forms".
   - §16.3's first bullet restates the region rule ("so its region is the rest of its line and the block indented under it").
   - Closer matching, the tab's column and the stack after a mis-indent are layout facts and belong in §2.2, with §16.3 citing them.
   - No history, comparison with another language or rule-after-example found.

## Not checked

- Nothing was run: not the example test, not `docs/pending-examples.txt` or `pending-productions.txt`.
- `ARCHITECTURE.md` was not read, so §7's stage order (relevant to finding 7) is unverified.
- The two cases the brief leaves out were not judged.
- The examples were checked by reading the rules, not by a parser.
