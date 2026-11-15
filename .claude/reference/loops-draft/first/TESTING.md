# Stated tests

What a stated test is and what makes one worth keeping. The test author writes to it and the test reviewer fails a case against it.

## Where they live

- `packages/symphonyscript/crates/compiler/tests/stated/` holds the rules files, the cases and the harness. `tests/stated-pending.txt` beside it lists the id of every case the code does not pass yet.
- The harness requires every listed case to fail and every other case to pass.
- A rules file holds one row a rule: its id, its place (`SPEC §7.5`, a production's name and alternative, or the page and its invariant), and its words quoted exactly.
- A case holds its id, its rule's id, a program and the expected value.

## Rules

1. **A case proves a rule.** Every case names one rule, and every rule is a SPEC sentence, a grammar alternative or a design-page invariant quoted word for word. A case that proves no quoted words is removed.
2. **The expected value comes from the rule's words.** Derive it by hand from the sentence and the page, and write it before any run. Never take it from a run of the code, from another implementation, from an older test or from a golden file, because a value copied back passes whatever the code does. A program may come from anywhere, an older test or a corpus included; its expected value may not.
3. **A run never corrects a value.** When a run disagrees with a stated value, read the sentence again. The value changes only when the sentence, read again, gives another one, and the change is reported with the words that decide it.
4. **A sentence that does not decide is a GAP.** Where the rule's words leave the expected value open or two sentences give two values, write no case: report the two readings, each with the program that tells them apart.
5. **Real behaviour, once.** A case proves something a user of the language would notice breaking, at the stage that owns the rule and nowhere else. No case for what the type system or a library guarantees, and no second case of a behaviour another case proves, because each one costs every later run and every later reader and proves nothing more.
6. **A rule's cases are its boundaries and its distinct outcomes**, in one table. A row between two boundaries fails only when they do, and is removed.
7. **Every condition has its row.** Each condition in a rule has a case that fails when that condition alone is dropped; a case another condition already decides proves nothing about it.
8. **Every refusal has its neighbour.** A rule that refuses has the refused program and the nearest accepted one, because a refusal alone is passed by code that refuses everything.
9. **Every sibling form.** When a rule covers several forms, the three bracket shapes, the cell words, the operators of one level, each form has a case or the rules file says why one is left out.
10. **Whole values.** A case compares the whole tree, or the whole list of diagnostics, each with its code and its span. Never a count, a search for one item or a test that something succeeded.
11. **Through the surface the page names.** A case calls what a caller of the stage calls and reads what the page says its tests observe, so a change inside the stage that keeps its behaviour keeps the case passing.
12. **Cost counts work.** A case that states a cost counts tokens, nodes or visits at four doubling sizes, never the clock.
13. **Shown to fail once.** A listed case fails by the harness. A harness that compares whole values holds one deliberately wrong case that must fail. A case of kept behaviour that cannot compare a whole value is shown red once against a scratch copy of the code with the behaviour broken, and the report names the case, the break and the failure.

## Defects

**Expected value from the code.** A value no sentence yields, an internal name, an order the SPEC does not state, came from a run.\
**Bad**
```text
program:  Soft: wash
expected: (Line (Placement (Kind Soft) (InlineChildren (Member (Kind wash)))))
rule:     "the parser reads a line with a colon as a placement with inline children"
```
**Good**
```text
program:  Soft: wash
expected: the neutral line the page defines, its head `Soft` and its tail the tokens `wash`
rule:     SPEC §6.5 "the reading is fixed when the name resolves: a body line with a colon"
```

**Golden accepted unread.** A stored output accepted from a run pins today's defect as the specification.\
**Bad**
```rust
if env::var_os("BLESS").is_some() { fs::write(&golden, &out)?; }
```
**Good**
```rust
assert_eq!(printed(tree), STATED_TREE);
```

**Case that cannot fail.** An assertion behind a match or a guard passes when the code breaks.\
**Bad**
```rust
if let Ok(tree) = parse(SOURCE) { assert_eq!(printed(tree), STATED_TREE); }
```
**Good**
```rust
assert_eq!(parse(SOURCE).map(printed), Ok(STATED_TREE.to_owned()));
```

**Name that promises more.** The id claims the behaviour; the case proves a call returned.\
**Bad**
```text
case wire_keeps_its_position: the program parses without a diagnostic
```
**Good**
```text
case wire_keeps_its_position: the body's tree holds `Cue`, `Triggers`, `Cue` in that order
```

**One behaviour from many angles.** Ten cases of the trivial path stand in for the one rule that is real logic.\
**Bad**
```text
lexes_plus, lexes_minus, lexes_star, lexes_slash
```
**Good**
```text
product_binds_tighter_than_sum: `1 + 2 * 3` is the sum of `1` and the product
```

**A clause no case isolates.** A rule with two conditions and one case that breaks both proves neither.\
**Bad**
```text
"a single underscore may separate two digits": `1__0_` is refused
```
**Good**
```text
`1_0` is accepted, `1__0` is refused, `1_` is refused, `_1` is a name
```

**A value changed to match a run.** A case whose history shows its value moved to what the code gives, with no sentence beside the move.\
**Bad**
```text
changed after run: case 7.5-3, the span now ends at the closing parenthesis
```
**Good**
```text
changed after run: case 7.5-3, span end 14 to 15: §7.7 "a parenthesized `/expr` ends at its closing parenthesis", which I had read as excluding it
```
