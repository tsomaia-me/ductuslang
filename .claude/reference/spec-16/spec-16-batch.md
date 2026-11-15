# §16: the edges, as one batch

Nine decisions on what a broken program draws, each with a program, the options, what each option draws and a pick. One worked example comes first, the one the owner asked for. Every outcome below is derived from the rules already settled; nothing was run.

The settled rules these rest on, in short:

- Every fault draws its own message, and one fault draws one message.
- A fault leaves a placeholder with no type and no value; a check that needs a fact the placeholder would give draws nothing.
- A syntax fault's region is the rest of its line and the block indented under that line. Nothing in a region is read, built or reported.
- A fault's line runs between two line ends the layout emits, so the lines a closed bracket continues it over belong to it.
- A closer pairs with the nearest open opener of its shape. An opener that nothing closes is one fault at the opener and suspends no layout.
- Every item a faulty line writes is a placeholder; an item's name that stands before the fault is declared.

In the programs, the numbers at the left are line numbers and are not part of the source.

---

## The worked example: an unclosed `(` on line 3 that a stray `)` on line 500 closes

```text
  1  state gain(1.0)
  2
  3  state trim(0.5
  4  state mix = gain * trim
  5
  6  Rig stage:
  7      Lamp as wash | level=mix
     ...
500      Lamp/0.5)
501
502  Scene dusk:
503      Dims: stage.wash
```

**What happens, step by step.**

1. The `)` on line 500 pairs with the `(` on line 3: it is the nearest open `(`. So the `(` is closed, and by §2.2 layout is suspended from line 3 to line 500: those 498 lines are one line.
2. After `0.5` the next token is `state` on line 4. It continues no production of `Init ::= '(' Expr ')'`. It begins a line, so the fault stands at the end of line 3.
3. The fault's line is lines 3 to 500. Its region is the rest of that line: lines 4 to 500.

**What draws.** One message, at the end of line 3.

**What goes dark.**

- Lines 4 to 500: nothing in them is read, declared, placed or reported.
- `trim` is declared and is a placeholder, because its name stands before the fault.
- `mix`, `stage` and everything else lines 4 to 500 would declare are not declared. By decision 1 below they are placeholders everywhere, so line 503's `stage.wash` draws nothing.
- The stray `)` on line 500 draws nothing. It draws, as a closer with nothing to close, once line 3 is repaired.
- By decision 5 below, minimum counts at top level and incoming-wire minimums are silent.

**The contrast.** Remove the stray `)` from line 500 and the `(` on line 3 is never closed. It is one fault at the `(`, it suspends no layout, and its region is the rest of line 3 alone. Nothing else goes dark. So a second, unrelated bracket fault 497 lines away is what darkens the 497 lines.

**How often.** A later closer takes an earlier opener only when it is unbalanced on its own line group. A balanced `g(b)` on a later line pairs with itself. So this needs two bracket faults in one file at once.

**Decision 0: accept this, or cut the region by indentation?**

1. Accept it. One message, 497 lines dark, in the two-fault case. The message names both ends: where the `(` opens and where it closes.
2. Add a rule for regions only: the region of a fault inside brackets ends before the first later line that starts at or left of the indentation of the line holding the opener; the opener then counts as never closed, and its closer draws as a stray one. Here: two messages (end of line 3, and the `)` on line 500), nothing dark.

Option 2 never refuses a valid program, since it applies only after a fault. Its cost: §2.2 lets a line inside brackets start at any column, so a program that writes continuation lines at or left of the opener's indentation draws extra messages for one fault inside that bracket. It is also one more rule to build, to test and to keep linear.

My pick: 1. The case needs two faults at once, the darkness lifts when either is repaired, and option 2 adds a rule whose results depend on a layout style the SPEC allows.

---

## Decision 1: which names a region turns into placeholders

A region is not read, so what it would have declared is unknown. A name used elsewhere that the region would have declared must not draw "unknown name": that would be a second message for one fault.

SPEC §16.3 (draft): "a name written in it is a placeholder at every use in the program."

Read to the letter this is too wide: a region that writes `Cue` silences every `Cue` in the program.

**The pick, in three parts.** A name a region writes is a placeholder:

- (a) at every use in the body that holds the region and in the bodies nested in it, and in every path that steps into that body;
- (b) for a region at top level, at every use where the name would otherwise resolve to a prelude name;
- (c) anywhere, when nothing outside a region declares it.

The reason for each part is the declaration the region could have held. Inside a body, a child or wire name shadows top-level and prelude names (§6.5: "A child or wire name shadows top-level names and prelude names within its body and the bodies nested in it"). At top level, a package's declaration shadows only a prelude name (§6.5: "a declaration of the package and a name its package file brings in shadow a prelude name"); against any other top-level name it would be a duplicate, which is its own fault. So a region at top level does not bring the wide reading back: it touches prelude names it writes, and names nothing declares.

**(a), a region inside a body.**

```text
1  let wash = 0.5
2
3  Rig stage:
4      when alarm +:
5          Lamp as wash
6      Dims: wash
```

Line 4 lacks an operand; its region is line 5. With (a), `wash` on line 6 is a placeholder: one message. Without (a), `wash` resolves to the `let` on line 1, a number where a node is needed: a second message the user did not earn.

**(c), a name nothing declares.**

```text
1  let base = 48
2      let step = 7
3
4  Clip melody:
5      Note/(base + step)
```

Line 2 is a block indented under a line that opens none: one fault, and line 2 is its region. With (c), `step` on line 5 is a placeholder: one message. Without it, `step` names nothing: two.

**(b), a prelude name at top level.** The same program with line 2 as `fn ratio(x): x * 2`, where the prelude also publishes `ratio`, and a later `ratio(3, 4)`. With (b) the call draws nothing; without it the call is checked against the prelude's `ratio`, which the user's own declaration was meant to replace.

Options: 1. the three parts above; 2. the letter of the draft (every use in the program); 3. only part (c).

My pick: 1. Option 2 silences common names program-wide. Option 3 draws the second message of example (a).

---

## Decision 2: which fault draws when one line holds several

SPEC §16.3 (draft): "The region is not read." After the first fault the rest of the line is its region, so a line draws one syntax message. Which one?

```text
A   Lamp | level=f(1 + , 2
B   Lamp/(f("abc))
```

In A the line holds two faults: no operand before `,`, and a `(` nothing closes. In B the string is not closed, so it swallows both `)`, and both `(` are left open.

Options:

1. The first in the text draws. A: the `(`. B: the first `(`.
2. The first in the text draws, and an opener left open counts last on its line, since it is known only at the line's end. A: the `,`. B: the string.

My pick: 2. In B the open brackets are a consequence of the string, and §16.1 says "nothing is drawn for what another fault left unknown". When the opener is the line's only fault its message stands at the opener, as ruled.

---

## Decision 3: which faults inside one token have a region

SPEC §16.3 (draft): "A fault of §2.2 to §2.6's rules or of a production is a syntax fault." By the letter, an unknown unit suffix unreads the rest of its line and the block under it.

```text
1  Rig stage:
2      Lamp | level=5foo hue=tint
```

`foo` is no unit, and `tint` names nothing.

Options:

1. Every fault of §2.2 to §2.6 has a region. One message; `tint` is in the region.
2. A fault inside one token leaves that token a placeholder value and has no region; the line is read on. Two messages, each a real fault.

Faults inside one token would be: an unknown suffix; an integer type suffix on a float (`0.5i32`); a literal of 2¹²⁷ or more; a misplaced underscore in a number; a letter or digit directly after a prefixed literal (`0b12`); and `==` or `!=`, which read on as `is` and `is not`. Every other fault of §2 keeps its region: a semicolon, a stray `^`, a string not closed, a tab in indentation, a lone CR, a misplaced byte order mark.

My pick: 2. The token's extent is known, so what follows it is read reliably, and "a fault that reads no placeholder draws its message as it does in any program" (§16.2).

---

## Decision 4: a fault on a line that holds no token

A lone CR or a byte order mark can stand in a comment. A comment-only line emits no `NEWLINE` (§2.2), so "the text between two `NEWLINE` tokens" is the code line after it.

```text
1  // gains<CR> for the stage
2  Rig stage:
3      Lamp/0.5
```

Options: 1. the fault's line is line 2, so `stage` and its body go dark; 2. a fault on a line that holds no token draws its message and has no region.

My pick: 2. Nothing on a comment line can make the next line unreadable.

---

## Decision 5: what the counts say

The owner's ruling: silence only the parents and kinds a region or a placeholder could change.

An unread region can only add items. So it can cure a count that is too low, and never one that is too high.

```text
A   Session a | tempo=:
        Track
    Session b

B   let base = 48 +
    Rig stage:
        Lamp
    Session live
```

In A, line 1 lacks a value. `a` is a placeholder, and its kind `Session` stands before the fault. The vocabulary allows one `Session`.

Options:

1. A parent that holds a region or a placeholder item has every count silent. A: nothing about two sessions.
2. A maximum already passed by what was read draws. A minimum is silent wherever an unread item could meet it: for the parent of a region, and for incoming wires everywhere while any region holds lines. A placeholder item whose kind stands before the fault counts as that kind; one whose kind is unknown counts toward nothing. A: "at most one `Session`" draws, a real second fault.

In B the fault's region is empty, so no item is unread: under option 2 every count and warning draws as in any program.

The same test serves the §10.2 warnings: "no entry-kind instance" and "an instance no entry-kind instance reaches" are silent while a region holds lines, since an unread line could hold the instance or the wire.

My pick: 2. It is the ruling applied by direction: what an unread line could change is silent, and what it could not change draws.

---

## Decision 6: which faults draw beside a limit's stop

SPEC §16.2 (draft): "A limit's error that stops the compilation (§4.6) stops it here too, and no fault found after it draws." The SPEC gives no order of finding, so two readers list different survivors.

```text
1  fn grow(n): if n < 1 then [0] else grow(n - 1) + grow(n - 1)
2  let big = grow(40)
3  let other = missing + 1
4  Rig stage | hue=:
```

Line 2 passes the recursive-call limit. Line 3 uses a name nothing declares. Line 4 has a syntax fault.

Options:

1. Only the limit's error draws.
2. Beside the limit's error, the faults that need no evaluation draw: every syntax fault, every name declared more than once, every name that resolves to nothing. No other fault does. Here: three messages.

My pick: 2. Those three kinds are decided by the text and the names alone, so the set is the same whatever order a compiler works in.

---

## Decision 7: a pattern's use with a placeholder argument

SPEC §16.2 (draft): "Structure that a placeholder decides, a `for` whose range bound is one or a use whose argument decides what it places, is not expanded".

```text
1  pattern Arp(default root, steps = 4) -> (Note*):
2      Note/root | velocity="loud"
3      for i in 0..steps:
4          Note/(root + i)
5
6  Clip verse:
7      Arp/48 | steps=glow
```

`glow` names nothing. Line 2 has a fault of its own: a string where `velocity` takes an `f32`.

Options:

1. The whole use is not expanded. One message: `glow`.
2. Only the structure the placeholder decides is not expanded, here the `for` of line 3; the rest of the use is expanded and checked. Two messages: `glow`, and line 2.

My pick: 2. Line 2's fault reads no placeholder, so by §16.2 it draws.

---

## Decision 8: one faulty line reached many times

```text
A   Rig chase:
        for i in 0..8:
            Lamp/(true + i)

B   pattern Dim(default level) -> Lamp:
        Lamp/level

    Rig stage:
        Dim/true
        Dim/0.5
        Dim/"low"
```

In A one line is faulty in all eight iterations. In B the pattern's line is faulty for two of three uses, each time because of the argument.

Options:

1. One message for each time the line is reached. A: eight. B: two.
2. A fault that reads no argument draws once. A fault that reads an argument draws once for each use or call written in the source that supplies it. Iterations of one `for` never multiply a message. A: one. B: two, one at each faulty use.

My pick: 2. In A the user made one mistake. In B the user made two, at lines the messages can point to (§6.4: "A diagnostic points at the use and at the line inside the pattern").

---

## Decision 9: one undeclared name used twice

```text
1  Rig stage:
2      Lamp/glow
3      Lamp/glow
```

Options:

1. One message for each use: two.
2. One message for the name, naming every use, as §6.5 does for a duplicate ("A duplicate is one error naming every declaration of the name"): one.

My pick: 2. Declaring `glow` once repairs every use, so it is one fault, and an editor can still mark each use from the one message.

---

## Decided without a question, for veto

- **The placeholder of a fault of legality is what its diagnostic names.** For `let x = reading + 1`, a `let` that reads a cell (§6.2), `x` is a placeholder and its uses draw nothing. This follows from the settled rule that no type is kept behind a fault. Reversing it means keeping a type behind a fault, which the owner ruled out.
