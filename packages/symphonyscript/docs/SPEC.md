# SymphonyScript Specification

This document defines SymphonyScript: what a source program may contain, what it means, and how the runtime evaluates it. It is normative. `ARCHITECTURE.md` describes how the implementation realizes it.

## 1. Overview

### 1.1 Programs and vocabularies

SymphonyScript describes graphs: nodes placed inside other nodes, wires between nodes, and values that change while the graph runs. Music graphs (sessions, tracks, clips, notes) and audio graphs (oscillators, filters, mixers) are written the same way.

A program is compiled against exactly one vocabulary. The vocabulary declares every node kind, wire kind, enum, unit and prelude declaration a program may use, together with the rules for combining them (§10). Source code declares no kinds. It places instances of the vocabulary's kinds, sets their attributes, connects them with wires, and binds attributes to values that change at runtime.

Compilation produces a Program (§11): a complete description of instances, wires, inputs and computed values. A host loads the Program into the runtime, writes inputs and commits (§12). Consumers read the Program and the reported changes and interpret them. The meaning of a kind belongs to its consumer, not to the language.

```
input cutoff = 0.5
let wobble = cutoff * 0.8

Clip intro:
    Note/48 Note/52 Note/55
    [Note/60 Note/64 Note/67]
    Note/62 | velocity=0.7 brightness=wobble
    Plays: verse

Clip verse:
    Note/65
    Plays: intro
```

### 1.2 Notation

Productions use EBNF: `::=` defines, `|` separates alternatives, `?` `*` `+` mark optional, repeated and one-or-more, `( )` groups, quoted text is literal. `NEWLINE`, `INDENT` and `DEDENT` are layout tokens (§2.2). `Name` is an identifier (§2.4). Every production appears once, in the section of the construct it defines.

Every example parses. Examples use the example vocabulary (§1.3). Names an example uses without declaring stand for declarations elsewhere in the same program. An example marked as an error shows a rejected program.

### 1.3 Example vocabulary

| Node kind | Placement | Attributes (default attribute first) | Accepts |
|---|---|---|---|
| `Session` | top level, at most one; entry kind | `tempo: f64 = 120` | `Track` |
| `Track` | top level or in `Session` | `label: string = ""` | `Piano`, `Synth` |
| `Clip` | top level | `gain: f32 = 1`, `level: f32 = 1` | `Note` |
| `Note` | in `Clip` | `pitch: i32 = 60`, `velocity: f32 = 1`, `brightness: f32 = 0.5`, `length: f64 = 0.25` in beats, `accent: bool = false` with flag `'` | none |
| `Kit` | top level | none | `Sampler` |
| `Sampler` | in `Kit` | `file: string` | none |
| `Piano` | in `Track` | none | none |
| `Synth` | top level or in `Track` | none | `Osc`, `Filter`, `Mixer` |
| `Osc` | top level or in `Synth` | `wave: Wave = Sine`, `freq: f64 = 440`, `rate: f32 = 1` | none |
| `Filter`, `Mixer`, `Output` | top level or in `Synth` | `Filter` has `cutoff: f32 = 1000` | none |

| Wire kind | From → to | Attributes (default attribute first) | Cyclic |
|---|---|---|---|
| `Plays` | `Clip` → `Clip` | `gain: f32 = 1`, `transpose: i32 = 0` | yes |
| `Starts` | `Clip` or `Track` → `Clip` | none | yes |
| `Includes` | `Session` → `Track` | none | no |
| `Feeds` | `Osc` or `Filter` → `Filter`, `Mixer` or `Output` | none | no |
| `Modulates` | `Osc` → `Filter` | `depth: f32 = 1` | no |

The enum `Wave` has the variants `Sine`, `Saw` and `Square`. The unit family `beats` has the base unit `beats` and the unit `bars` with scale 4. The unit family `time` has the base unit `s` and the unit `ms` with scale 0.001. Every count not stated above is "any number".

## 2. Source text

### 2.1 Files

A project is a set of `.sym` files compiled together. All files share one namespace (§6.5); no file imports another. Files are ordered by path, and items within a file by source order. This order is the program's top-level order.

```
File    ::= TopItem*
TopItem ::= InputDecl | LetDecl | FnDecl | PatternDecl | TopPlacement
```

### 2.2 Layout

Indentation is significant. Leading whitespace consists of spaces; a tab in leading whitespace is an error. Blank lines and lines holding only a comment do not affect layout.

The lexer keeps a stack of indentation columns, starting with column 0. A line indented deeper than the top of the stack pushes its column and emits `INDENT`. A line indented less pops columns, emitting one `DEDENT` per popped column, and must land exactly on a column still on the stack. Every line ends with `NEWLINE`. At end of input the lexer emits `NEWLINE` and then a `DEDENT` for every open column.

Inside `( )` and `[ ]`, layout is suspended: newlines are whitespace and no layout tokens are emitted.

A line whose first token sits exactly at the column of the first entry of the previous line's attribute clause continues that clause (§7.6). The line break before it emits no `NEWLINE`, and the line emits no `INDENT` or `DEDENT`.

A semicolon is an error anywhere in source.

### 2.3 Comments

`//` starts a comment that runs to the end of the line. There is no other comment form. Inside a string, `//` is text.

### 2.4 Identifiers

An identifier starts with a Unicode letter, `_` or `#`, and continues with those characters and Unicode digits. `#` behaves as a letter, so note names such as `C#4` and `F#3` are identifiers.

By convention, kinds, patterns and enum variants are written in PascalCase, and everything else in snake_case. The compiler does not enforce the convention.

### 2.5 Keywords and reserved tokens

Keywords are never names:

```
input let fn pattern default
if then else match when for in as
is not and or div to true false
i32 i64 f32 f64 bool
```

The words `from`, `recurrent` and `string` are reserved and are never names. The tokens `^`, `==` and `!=` are reserved and rejected by the lexer; the diagnostic for `==` and `!=` points to `is` and `is not`.

### 2.6 Literals

```
IntLit     ::= Digits | '0x' HexDigits | '0b' BinDigits | '0o' OctDigits
FloatLit   ::= Digits '.' Digits Exponent? | Digits Exponent
Exponent   ::= ('e' | 'E') ('+' | '-')? Digits
NumberLit  ::= (IntLit | FloatLit) Suffix?
Suffix     ::= TypeSuffix | UnitSuffix
TypeSuffix ::= 'i32' | 'i64' | 'f32' | 'f64'
BoolLit    ::= 'true' | 'false'
StringLit  ::= '"' (Char | Escape | '{' Expr '}')* '"'
Escape     ::= '\"' | '\\' | '\n' | '\t' | '\{'
```

- An underscore may separate two digits: `1_000_000`. Anywhere else it is an error.
- A float needs digits on both sides of the point: `0.5`, not `.5`.
- A suffix follows the digits with no whitespace. A type suffix fixes the literal's type: `5i64`, `0.5f64`. A unit suffix is a unit the vocabulary declares: `200ms`, `2bars` (§3.6). Any other suffix is an error.
- A minus sign directly before a numeric literal forms one negative literal, so `-128` is checked against its target type as a single value.
- A string occupies one line. `{expr}` inside a string is interpolation (§5.9); `\{` is a literal brace.

## 3. Values and types

### 3.1 Scalars

| Type | Values |
|---|---|
| `i32` | 32-bit signed integers |
| `i64` | 64-bit signed integers |
| `f32` | 32-bit IEEE 754 floats |
| `f64` | 64-bit IEEE 754 floats |
| `bool` | `true`, `false` |

### 3.2 Arrays

An array has a fixed length and one element type. `[60, 64, 67]` has type `i32[3]`. Elements may themselves be arrays of equal length: `[[60, 64, 67], [62, 65, 69]]` has type `i32[3][2]`, two rows of three.

An array's length is fixed at compile time. Its contents may change at runtime when its elements are reactive (§5.5).

```
Type       ::= (ScalarType | Name) ('[' IntLit ']')*
ScalarType ::= 'i32' | 'i64' | 'f32' | 'f64' | 'bool'
```

A `Name` in a type is a vocabulary enum.

### 3.3 Strings

Strings exist only at compile time. They are written as literals, built with `+` and interpolation, and bound to string attributes the vocabulary declares. No input, cell or runtime value is a string.

```
Kit drums:
    for i in 0..8:
        Sampler | file="kit/{i}.wav"
```

### 3.4 Enums

The vocabulary declares enums. Every variant is a plain name with no data. Where the expected type is an enum (§4.2), a variant is written bare: `wave=Saw`. Anywhere else it is qualified with the enum's name: `Wave::Saw`.

### 3.5 Node references

A node reference names an instance. Node references appear only as wire destinations (§8.2) and as pattern arguments (§6.4). They are never stored in an input, a `let` or an attribute.

### 3.6 Units

The vocabulary declares unit families. Each family has a base unit and further units with a scale relative to the base: in the family `time`, `s` is the base and `ms` has scale 0.001.

1. A literal with a unit suffix converts to its family's base unit at compile time: `200ms` is 0.2 in `time`.
2. An attribute may belong to a family and then has a default unit. A plain number bound to it is in that default unit.
3. A unit value bound to an attribute of a different family, or of no family, is an error.
4. `+`, `-` and comparisons require both operands in the same family. `/` of two values in the same family yields a plain number. Multiplying or dividing a unit value by a plain number keeps the family. Multiplying two unit values is an error, and so is `**` on a unit value.
5. A `let`, input, function result or pattern argument carries the family of its value, inferred like its type.

Families never convert into each other. A family whose relation to another depends on runtime values, such as beats and seconds under a changing tempo, stays separate, and the consumer converts.

```
Clip intro:
    Note | length=2bars
    Note | length=0.5
```

The first note lasts 8 beats; the second lasts 0.5 beats, the attribute's default unit.

## 4. Type inference

Source code rarely states a type. Types come from the places values are used.

### 4.1 Fixed and flexible values

A value has a fixed type when something fixes it: an attribute, a built-in's result, a suffix, an annotation, or an operand of fixed type in the same expression. Every other compile-time value is flexible: an unsuffixed integer or float literal, and any compile-time `let` built only from such values. Each use of a flexible value chooses its type independently, and the value must fit the chosen type.

```
let base = 48

Clip melody:
    Note/(base + 7)

Synth lead:
    Osc | freq=(base * 2.5)
```

`base` is `i32` in the pitch and `f64` in `freq`.

An integer literal fits a float type when its value is exactly representable. A float literal fits any float type, rounded to the nearest value.

```
Clip melody:
    Note/300000000000
```

This is an error: the value does not fit the pitch's `i32`.

### 4.2 Expected types

The type a position requires flows into the expression filling it: through operators, `if` and `match` arms, array literals and function arguments, down to the flexible values inside. A fixed-type operand meeting an expected type either widens into it (§4.3) or is an error.

```
input cutoff = 0.5

Clip melody:
    Note | brightness=cutoff * 0.8
```

With `brightness` of type `f32`, `0.8` and `cutoff` are both `f32`.

### 4.3 Widening

These implicit widenings exist, and no others: `i32` to `i64`, `i32` to `f32`, `i32` to `f64`, `i64` to `f64`, `f32` to `f64`. A value never narrows implicitly, and a float never becomes an integer implicitly; conversions are explicit (§5.8).

When an operator's operands have different fixed types, both widen to the smallest type both widen into. With no such type, the expression is an error.

### 4.4 Cell types

An input, and a `let` that reads an input (a reactive `let`), is a cell: it holds one value at runtime, so it has exactly one type. The compiler chooses the narrowest type that widens into every use of the cell. The initializer must fit that type, and uses that no single type satisfies are an error naming both.

```
input cutoff = 0.5

Clip melody:
    Note | brightness=cutoff

Synth lead:
    Osc | freq=cutoff
```

With `brightness` of type `f32` and `freq` of type `f64`, `cutoff` is `f32` and widens at `freq`.

The compiler resolves cell types from uses back to sources. This always terminates, because cells form no cycles (§6.2).

### 4.5 Defaults and annotations

A cell whose uses fix no type defaults to `i32` for an integer initializer, or `i64` when the initializer does not fit `i32`; to `f32` for a float initializer, or `f64` when the initializer lies outside `f32`'s finite range; and to `bool` for a `bool` initializer.

An annotation fixes a type outright: `input fine: f64 = 0`. A suffix does the same for a literal: `0f64`.

### 4.6 Compile-time evaluation

A compile-time value is computed by the rules of §5, in a type fixed before the computation:

- The bounds of a range in a `for` and an index known at compile time are `i64`. A range's loop variable is a flexible integer, so each use chooses its type.
- A call to a recursive function (§6.3) computes in the types its arguments and expected type give, and in the defaults of §4.5 where neither fixes a type.
- Every other compile-time value is computed in the type its use chooses (§4.1).

A value that structure depends on, such as a range bound or a recursive call's result, is computed before the expansion that depends on it.

```
pattern Arp(default root, steps = 4, gap = 4):
    for i in 0..steps:
        Note/(root + i * gap)
```

`steps` is computed as an `i64` to count the iterations, and `i` is an `i32` in each pitch.

## 5. Expressions

### 5.1 Precedence

From loosest to tightest:

| Level | Operators | Associativity |
|---|---|---|
| 1 | `if … then … else …` | right |
| 2 | `or` | left |
| 3 | `and` | left |
| 4 | `not` | prefix |
| 5 | `is`, `is not`, `<`, `<=`, `>`, `>=` | none |
| 6 | `+`, `-` | left |
| 7 | `*`, `/`, `div`, `%` | left |
| 8 | unary `-` | prefix |
| 9 | `**` | right |
| 10 | `.name`, `[index]`, `(arguments)` | left |

```
Expr      ::= IfExpr | OrExpr
IfExpr    ::= 'if' Expr 'then' Expr 'else' Expr
OrExpr    ::= AndExpr ('or' AndExpr)*
AndExpr   ::= NotExpr ('and' NotExpr)*
NotExpr   ::= 'not' NotExpr | CmpExpr
CmpExpr   ::= AddExpr (CmpOp AddExpr)?
CmpOp     ::= 'is' 'not'? | '<' | '<=' | '>' | '>='
AddExpr   ::= MulExpr (('+' | '-') MulExpr)*
MulExpr   ::= UnaryExpr (('*' | '/' | 'div' | '%') UnaryExpr)*
UnaryExpr ::= '-' UnaryExpr | PowExpr
PowExpr   ::= Postfix ('**' UnaryExpr)?
Postfix   ::= Primary ('.' Name | '[' Expr ']' | '(' Args? ')')*
Primary   ::= NumberLit | BoolLit | StringLit | Name | Name '::' Name
            | 'to' | ScalarType '(' Expr ')' | ArrayLit | '(' Expr ')'
ArrayLit  ::= '[' Expr (',' Expr)* ','? ']'
Args      ::= Expr (',' Expr)* | NamedArg (',' NamedArg)*
NamedArg  ::= Name ':' Expr
```

Comparisons do not chain: `a < b < c` is an error. `-2 ** 2` is −4. An `if` that is an operand of another operator is parenthesized: `base + (if high then 12 else 0)`. An `if` that is a whole value, such as a `let`'s value, an attribute value or an argument, needs no parentheses.

Expressions evaluate the same way at compile time and at runtime; §4.6 fixes the types compile-time values compute in.

### 5.2 Arithmetic

`+`, `-` and `*` take numeric operands of one type after widening; `+` also concatenates strings (§5.9). On integers they saturate at the type's bounds: `2147483647 + 1` in `i32` is 2147483647.

`/` always yields a float. Integer operands convert to the result's float type: the expected type when one exists, otherwise `f64` if either operand is `i64` or `f64`, otherwise `f32`. `1 / 4` is 0.25.

`div` takes integers and rounds toward negative infinity: `-7 div 2` is −4. `%` takes integers or floats, and its result has the sign of the divisor: `-7 % 2` is 1, `-1 % 7` is 6. Integer `div` or `%` by zero yields 0.

`**` raises to a power. With integer operands it yields an integer of their type, saturating; a negative exponent yields 0, except that base 1 yields 1 and base −1 yields 1 or −1 by the exponent's parity. With a float operand it yields a float of the operands' type after widening. `2 ** 4` is the integer 16. `2 ** (7 / 12)` is the frequency ratio of a fifth, a float because `7 / 12` is one.

Float arithmetic follows IEEE 754, including infinities and NaN.

### 5.3 Comparison and logic

`is` and `is not` compare two scalars, enum values or strings of one type after widening; floats compare by IEEE 754 equality. `<`, `<=`, `>` and `>=` compare numbers. `and`, `or` and `not` take and yield `bool`.

### 5.4 Conditionals

`if c then a else b` yields `a` when `c` is true and `b` otherwise. The condition is a `bool`, and only the chosen branch is evaluated. `else` is mandatory, and `else if` chains:

```
input loud = false
input quiet = false
let gain = if loud then 0.9 else if quiet then 0.3 else 0.6
```

`match` selects a value by an enum, an integer or a `bool`:

```
input wave = Wave::Saw

let level = match wave:
    Sine: 1.0
    Saw: 0.8
    else: 0.6
```

```
MatchBlock ::= 'match' Expr ':' NEWLINE INDENT MatchArm+ ElseArm? DEDENT
MatchArm   ::= (Name | NumberLit | BoolLit) ':' Expr NEWLINE
ElseArm    ::= 'else' ':' Expr NEWLINE
```

- Arm labels are variant names of the scrutinee's enum, or literals of its type.
- A `match` on an enum or a `bool` covers every value or ends with `else:`. A `match` on an integer ends with `else:`.
- The first matching arm wins, and only its value is evaluated. Arm values unify as `if` branches do (§4.2).
- `match` is the whole value of a `let`, a function body or a wire destination (§8.2), never an operand: its arms need layout, and layout is suspended inside parentheses.

### 5.5 Arrays and indexing

`a[i]` reads element `i`, counting from 0. The index is an integer. An index below 0 reads the first element, and an index past the end reads the last. A compile-time index outside the array is an error.

An array literal whose elements read cells is reactive, and each element is bound separately:

```
input a = 1
input b = 2
input i = 0
let c = [a, b, a + b]
```

- `c[1]` with a literal index is the same binding as `b`. A change to `a` does not reach it.
- `c[2]` is a cell of its own, computed from `a` and `b`.
- `c[i]` depends on `i` and on every element.

Operators apply to scalars only. `[1, 2] + [3, 4]` is an error.

### 5.6 Attribute reads

`x.name` reads attribute `name` of the instance or wire `x`. The read is the same binding the attribute holds: a constant, an input, a cell, or the vocabulary default when the source sets none.

```
Clip verse | gain=0.8:
    Note as hook /65 | velocity=0.9

Clip chorus | gain=verse.gain * 1.2:
    Note/72 | velocity=verse.hook.velocity
    Plays as back | transpose=-2: verse

Clip outro | gain=chorus.back.transpose:
    Note/60
```

- A path starts at a name in scope (§6.5) and steps through children, named wires and the named internals of pattern uses: `chorus.back`, `lead.v1.osc`.
- Reading an attribute the kind does not declare is an error.
- Reads that form a cycle are an error naming the cycle.
- A gated-off instance's attributes read as their current values (§9.3).
- Gate state is not readable.

`to.name` reads an attribute of a wire's destination (§8.3).

### 5.7 Calls and built-ins

A call invokes a built-in, a prelude function or a source function (§6.3). Arguments are all positional or all named; mixing them in one call is an error.

| Built-in | Result |
|---|---|
| `abs(x)`, `min(a, b)`, `max(a, b)`, `clamp(x, lo, hi)` | numeric, of the operands' type |
| `floor(x)`, `ceil(x)`, `round(x)` | float; `round` rounds half away from zero |
| `sqrt(x)`, `sin(x)`, `cos(x)`, `tan(x)`, `exp(x)`, `log(x)` | float; `log` is the natural logarithm |
| `lerp(a, b, t)` | float, `a + (b - a) * t` |
| `rand(seed, n)` | `f32` in [0, 1), a fixed function of two integers (§12.5) |
| `pi` | the float constant π |

Integer arguments to float built-ins convert as `/` does. Domain functions, such as converting a note number to a frequency, come from the vocabulary's prelude (§10.1).

### 5.8 Conversions

`i32(x)`, `i64(x)`, `f32(x)` and `f64(x)` convert a number explicitly. Float to integer truncates toward zero. Values outside the target range saturate to its bounds, and NaN becomes 0.

### 5.9 Strings

`+` concatenates strings. `{expr}` inside a literal interpolates a compile-time value: integers in decimal, floats in their shortest exact form, `bool` as `true` or `false`, enum variants by name. A string expression that reads a cell is an error.

## 6. Declarations

### 6.1 Inputs

```
InputDecl ::= 'input' Name (':' Type)? '=' Expr NEWLINE
```

An input is a cell the host writes (§12.1). Its initializer is a compile-time value. An input may be an array; the host then writes single elements or the whole array.

```
input cutoff = 0.5
input steps = [true, false, false, true, false, false, true, false]
```

### 6.2 Lets

```
LetDecl ::= 'let' Name (':' Type)? '=' (Expr NEWLINE | MatchBlock)
```

A `let` names a value. A `let` that reads no input, directly or through other declarations, is a compile-time value (§4.1). A `let` that reads an input is a reactive cell, recomputed when what it reads changes (§12.2). Cells form no cycles; a cycle is an error naming its members.

### 6.3 Functions

```
FnDecl   ::= 'fn' Name '(' Params? ')' ':' (Expr NEWLINE | NEWLINE INDENT FnBody DEDENT)
FnBody   ::= (LocalLet NEWLINE)* (Expr NEWLINE | MatchBlock)
LocalLet ::= 'let' Name '=' Expr
Params   ::= Param (',' Param)*
Param    ::= Name ('=' Expr)?
```

A function computes one value from its parameters.

```
fn db(x): 10 ** (x / 20)

fn scale(degree, root = 60):
    let octave = degree div 7
    root + [0, 2, 4, 5, 7, 9, 11][degree % 7] + 12 * octave
```

- A body is one expression, or local `let`s followed by one expression or `match`.
- Each call to a non-recursive function is replaced by the body with its arguments substituted, and the result is checked in place. Parameter types come from each call's arguments and expected type. A function never appears in the Program.
- A call whose arguments read cells yields a reactive value.
- A positional call may omit trailing parameters that have defaults; a named call may omit any parameter that has a default.
- A function's arguments and result are values, never node references.

A function that reaches itself through calls, directly or through other functions, is recursive. Every call to a recursive function has compile-time arguments, and the compiler evaluates it to a value. A call to a recursive function with an argument that reads a cell is an error, and so is evaluation deeper than 1,024 nested calls; both diagnostics name the chain of calls.

```
fn fib(n): if n < 2 then n else fib(n - 1) + fib(n - 2)

let lengths = [fib(3), fib(4), fib(5), fib(6)]
```

### 6.4 Patterns

```
PatternDecl   ::= 'pattern' Name '(' PatternParams? ')' ':' NEWLINE INDENT BodyItem+ DEDENT
PatternParams ::= PatternParam (',' PatternParam)*
PatternParam  ::= 'default'? Name ('=' Expr)?
```

A pattern is a named, parameterized group of placements.

```
pattern Arp(default root, steps = 4, gap = 4):
    for i in 0..steps:
        Note/(root + i * gap)

Clip verse:
    Arp/48
    Arp/60 | steps=8 gap=3
```

1. Patterns are declared at top level. The body holds any body items (§7.1).
2. A pattern is used with placement syntax. The attribute clause supplies parameters, and `/expr` supplies the parameter marked `default`, of which there is at most one. A parameter without a default must be supplied. A use carries no flags and no body.
3. A use is replaced by the body's placements at the use's position, with the arguments substituted, and the result is checked in place. Parameter types come from each use's arguments. An argument may be a value, a reactive value or a node reference.
4. A pattern body sees top-level names, its parameters and its own names, never the names around its use.
5. Names inside one expansion belong to it. A named use (`Voice as v1`) makes them reachable as `v1.osc`.
6. A pattern may use other patterns, but no pattern reaches itself through uses.
7. The vocabulary checks (§10.2) apply to the expanded placements. A diagnostic points at the use and at the line inside the pattern.
8. When a pattern's body is exactly one placement, a use stands in for that placement and the use's name becomes its name. Only such uses appear at top level.
9. A `when` on a use gates every placement it expands to.

```
pattern Riff(default root):
    Clip:
        Note/root Note/(root + 7) Note/(root + 12)

Riff low /36
Riff high /48
```

A pattern body cannot name the instance that a top-level use creates. A pattern that needs that instance takes it as an argument:

```
pattern Loop(default root, again):
    Clip:
        Note/root Note/(root + 7)
        Plays: again

Loop low /36 | again=low
```

Patterns never appear in the Program.

### 6.5 Names and scope

All top-level names share one namespace across the project: inputs, `let`s, functions, patterns, top-level instances, and the vocabulary's kinds, enums and prelude declarations. A duplicate is an error naming both declarations.

Inside a body, the `as` names of the body's own children and wires are visible regardless of order, as are top-level names. A child name equal to a visible name is an error. A `for` variable is visible in its body and is not equal to a visible name. In a function, parameters and local `let`s shadow top-level names; in a pattern, parameters do. Nothing else shadows.

A bare name in a position whose expected type is an enum resolves to that enum's variant (§3.4). Every other name resolves in scope.

## 7. Placements

### 7.1 Form

A placement creates an instance of a node kind or uses a pattern. Its elements appear in a fixed order.

```
TopPlacement ::= Kind Flags? 'as'? Name DefaultArg? Gate? Attrs? (NEWLINE | Body)
Placement    ::= Kind Flags? ('as' Name)? DefaultArg? Gate? Attrs?
Kind         ::= Name
DefaultArg   ::= '/' (Atom | '(' Expr ')')
Atom         ::= NumberLit | BoolLit | StringLit | Name ('.' Name)*
Gate         ::= 'when' Expr
Attrs        ::= '|' AttrEntry+
AttrEntry    ::= Name '=' Expr | Name | '!' Name
Body         ::= ':' (NEWLINE INDENT BodyItem+ DEDENT | InlineChildren NEWLINE)
BodyItem     ::= Line | WirePlacement | ForBlock | WhenBlock
Line         ::= Placement (NEWLINE | Body) | Bundle NEWLINE | SelfDelimited SelfDelimited+ NEWLINE
```

A line whose `Kind` names a wire kind of the vocabulary is a wire placement (§8.1). Whitespace around `/` is free.

### 7.2 Top-level placements

A top-level placement is named: `Clip verse` or `Clip as verse`. Top-level instances are independent; they relate to each other only through wires. The vocabulary states which kinds may stand at top level and how many of each (§10.2).

### 7.3 Children and containment

A placement inside another's body is its child. The parent owns the child, and the child is reachable only through its parent: `verse.hook`. A child is anonymous or named with `as`.

1. An instance has exactly one owner and is placed once. Another instance relates to it only by wire.
2. The vocabulary states which kinds each kind accepts as children and how many of each (§10.2).
3. A body is an ordered sequence. Children, bundles and wires keep their source order in the Program (§11).

```
Session main | tempo=120:
    Track as bass:
        Synth
        Starts: groove
    Includes: keys

Track keys:
    Piano
    Starts: intro
```

### 7.4 Flags

A flag is a single character directly after the kind, with no whitespace, that sets a `bool` attribute to `true`. The vocabulary assigns flag characters to attributes (§10.1).

```
Flags    ::= FlagChar+
FlagChar ::= "'" | '!' | '?' | '*' | '+' | '~' | '@' | '$'
```

`Note'/65` sets the attribute flagged `'`. An identifier character directly after a flag run is an error; a space separates them.

### 7.5 Default attribute

`/expr` sets the kind's default attribute, which the vocabulary designates (§10.1): `Note/60` sets the pitch. Unparenthesized, the value is a single atom: a literal, a name or a dotted path. Anything else is parenthesized: `Note/(base + 7)`. `/expr` on a kind without a default attribute is an error.

### 7.6 Attribute clause

A single `|` opens the attribute clause, and entries are separated by whitespace.

- `name=expr` sets an attribute.
- `name` sets a `bool` attribute to `true`; `!name` sets it to `false`.
- Setting an attribute twice, by any combination of entries and flags, is an error.
- Setting an attribute the kind does not declare is an error, and so is leaving unset an attribute the vocabulary gives no default (§10.1).

The clause continues on following lines that start at its first entry's column, without another `|`:

```
input cutoff = 0.5
let wobble = cutoff * 0.8

Clip melody:
    Note/62 | velocity=0.7
              brightness=wobble
              accent
```

An attribute bound to a constant is static. An attribute bound to an expression that reads a cell tracks it (§12.2).

### 7.7 Several placements on one line

```
SelfDelimited  ::= Kind Flags? ('as' Name)? ('/' Atom)? | '(' Placement ')' | Bundle
InlineChildren ::= SelfDelimited+
```

Placements on one line are separated by whitespace, and each must end without looking at the next: a kind with optional flags, `as` name and atomic `/expr`. A placement with a gate, an attribute clause or a parenthesized `/expr` is parenthesized or written on its own line. A placement with a body owns its line, and its body may follow the colon on the same line:

```
Clip fill: Note/60 Note/62 (Note/64 | velocity=0.5)
```

### 7.8 Bundles

```
Bundle    ::= '[' SelfDelimited* BundleFor? ']'
BundleFor ::= 'for' Name 'in' Iterable ':' SelfDelimited+
```

A bundle is one entry of its parent's sequence whose members are simultaneous: a chord.

```
Clip pad:
    [Note/60 Note/64 Note/67]
    [for i in 0..3: Note/(62 + i * 4)]
```

- A bundle has at least one member. Members are node placements, which may be gated and named. Wires and bundles are not members.
- Members are children of the bundle's parent.
- Layout is suspended inside the brackets, so members are self-delimited (§7.7). A `for` inside a bundle extends to the closing bracket.

### 7.9 Compile-time `for`

```
ForBlock ::= 'for' Name 'in' Iterable ':' (NEWLINE INDENT BodyItem+ DEDENT | InlineChildren NEWLINE)
Iterable ::= Expr '..' Expr | Expr
```

A `for` repeats its body once for each value of a compile-time iterable: a range `a..b`, which counts from `a` up to but excluding `b`, or an array. The loop variable is a compile-time value. An iterable that reads a cell is an error. Placements in a `for` body are anonymous; `as` there is an error. `for` appears in bodies, not at top level.

```
input steps = [true, false, false, true, false, false, true, false]
input pitches = [36, 36, 43, 36, 41, 36, 43, 48]

Clip seq:
    for i in 0..8:
        Note/(pitches[i]) when steps[i]
```

## 8. Wires

### 8.1 Wire placements

```
WirePlacement ::= Kind Flags? ('as' Name)? DefaultArg? Gate? Attrs? ':' Destination
Destination   ::= Expr NEWLINE | NEWLINE INDENT MatchBlock DEDENT
```

A wire connects its source to its destination and carries attributes. Its source is the instance whose body it is written in. A wire may appear anywhere in that body's sequence, and its position is kept (§7.3); what the position means is the consumer's.

```
Clip intro:
    Note/48 Note/52
    Starts: drums
    Note/55
    Plays | transpose=5: verse
```

Flags, `/expr` and the attribute clause work for wires as for nodes (§7.4–§7.6). The vocabulary states which kinds each wire kind connects (§10.2).

### 8.2 Destinations

A destination is a node reference: a name or a path, an `if` choosing between node references, an array of node references indexed by a value, or a `match` yielding node references.

```
input section = 0
input fill_on = false

Clip verse:
    Note/65 Note/67
    Plays: [chorus, bridge, outro][section]

Clip chorus:
    Note/72
    Plays: if fill_on then fill else verse
```

1. A destination is always one of a fixed set of candidates written in the source. A destination that reads cells chooses among them at runtime.
2. Every candidate is checked against the wire kind's endpoints (§10.2).
3. A wire never resolves to nothing. A gate disconnects it: `Plays when looping: verse`.
4. A destination names an instance placed elsewhere. It is never an inline placement and never a pattern use.

### 8.3 `to`

Inside a wire placement's `/expr`, gate and attribute clause, `to.name` reads attribute `name` of the wire's destination.

```
Clip verse:
    Plays | gain=to.level: [chorus, bridge][section]
```

- With a fixed destination, `to.name` is that instance's attribute.
- With a destination chosen at runtime, every candidate has `name` with the same type and unit family, and `to.name` makes the same choice among the candidates' attributes.
- `to` alone, and `to` outside a wire placement, is an error.

### 8.4 Cycles

Wires form cycles only through wire kinds the vocabulary marks cyclic. Every candidate of a destination chosen at runtime counts as an edge in this check.

## 9. Gates

### 9.1 Inline `when`

`when expr` on a node, a wire or a pattern use gates it: the instance is active only while the expression, a `bool`, is true.

```
Clip verse:
    Note/60
    Note/62 when fill_on
    Plays when looping: chorus
```

### 9.2 `when` blocks

```
WhenBlock ::= 'when' Expr ':' Block ('else' ':' Block)?
            | 'when' ':' NEWLINE INDENT GuardArm+ ('else' ':' Block)? DEDENT
GuardArm  ::= Expr ':' Block
Block     ::= NEWLINE INDENT BodyItem+ DEDENT | InlineChildren NEWLINE
```

A `when` block gates groups of body items. In the first form, the first block is active while the condition holds and the `else` block otherwise; the `else` aligns with its `when`. In the second form, the first arm whose guard holds is active, and `else`, the last arm, is active when none holds.

```
Clip verse:
    Note/60
    when fill_on:
        Note/62 Note/64
    else:
        Note/62
    when:
        section is 0: Note/67
        section is 1: Note/69
        else: Note/71
```

Every arm's placements exist at all times and count toward the vocabulary's counts (§10.2). Only their activation varies.

### 9.3 Activation

An instance or wire is active when its own gate and the gates of all its ancestors are open. A wire's ancestors include its source. Activation is runtime state (§12.3). It is not readable from source, and it stops no value from updating. What an inactive instance means, such as a silent note that keeps its time or a bypassed effect, is the consumer's.

## 10. The vocabulary

### 10.1 What a vocabulary declares

| Entry | Declares |
|---|---|
| Node kind | name; attributes; whether it may stand at top level, and how many; accepted child kinds, and how many of each; accepted incoming and outgoing wires per wire kind, and how many; whether it is an entry kind |
| Wire kind | name; allowed source and destination kinds; attributes; whether it may form cycles |
| Attribute | name; type; default; unit family and default unit; flag character; whether it is its kind's default attribute |
| Enum | name; variants |
| Unit family | name; base unit; units and their scales |
| Prelude | SymphonyScript source: functions, patterns and named values such as note names |

A count is exactly *n*, at least *n*, at most *n*, between *n* and *m*, or any number. A kind or wire kind absent from a list is not accepted. Each kind has at most one default attribute, and its flag characters are distinct. Every name a vocabulary declares is an identifier (§2.4) and not a keyword or reserved word (§2.5).

The prelude is ordinary source compiled with the program. It holds `let`, `fn` and `pattern` declarations only, and its declarations share the top-level namespace (§6.5).

### 10.2 Checks against the vocabulary

The compiler checks the expanded program (§6.4) against the vocabulary:

1. Every kind, wire kind, attribute, enum variant and unit exists in the vocabulary.
2. Every top-level instance's kind may stand at top level, and the number of each such kind is within its count.
3. Every child's kind is accepted by its parent, and each parent's children of each kind are within their counts.
4. Every wire's source and every candidate destination are kinds the wire kind connects, and each instance's incoming and outgoing wires of each kind are within their counts.
5. A wire whose destination is chosen at runtime counts toward the maximum of every candidate and toward the minimum of none. The program is valid only if it is valid for every value the choice can take.
6. Every attribute value has the attribute's type and unit family.
7. Cycles pass only through cyclic wire kinds (§8.4).

A top-level instance that no entry-kind instance reaches, through children and through wires to any candidate, draws a warning.

## 11. The Program

Compilation produces a Program. It contains:

- the identity of the vocabulary it was compiled against;
- every instance: its kind, path, attributes and gate, and its sequence of children, bundles and wires in source order;
- every wire: its kind, path, source, destination candidates and choice, attributes and gate;
- every input: its name, type and initial value;
- every reactive cell, in dependency order;
- the top-level instances in top-level order (§2.1).

Every attribute of every instance and wire is present, with vocabulary defaults filled in. Values are in their family's base unit. Functions and patterns are expanded, and no other source construct remains.

### 11.1 Paths

Every instance and wire has a path, with segments joined by `.`:

- A top-level instance's path is its name.
- A named child or wire extends its parent's path with its name: `verse.hook`.
- An anonymous child or wire extends it with its kind and its position among the parent's anonymous entries of that kind, counted from 0 in sequence order, bundle members included: `verse.Note:2`.
- A named pattern use contributes its name, and an anonymous use contributes the pattern's name and position: `verse.Arp:0.Note:2`. A use that stands in for a single placement (§6.4, rule 8) gives that placement the use's path.

## 12. Runtime semantics

### 12.1 Writes

The host writes inputs by handle. A write stages a value until the next commit and has no other effect. A value of the wrong type is rejected with an error. An array input accepts single-element and whole-array writes. Writes accumulate between commits, and the last write to each element wins.

### 12.2 Commit

A commit applies staged writes and brings every cell up to date:

1. Each input whose staged value differs from its current value changes.
2. Each cell that reads a changed value is evaluated, in dependency order, at most once per commit.
3. A cell whose new value is bit-for-bit equal to its previous value is unchanged, and its readers are not evaluated because of it.
4. Activation (§9.3) and destination choices (§8.2) are recomputed.

After a commit, every value reflects the same set of inputs. No reader sees part of a commit.

### 12.3 Changes

A commit reports what changed, in the terms consumers use:

- an instance attribute, with the element index for an array attribute;
- a wire attribute, with the element index for an array attribute;
- an instance's activation;
- a wire's activation;
- a wire's chosen destination.

Each is reported only when its value differs from its value before the commit. Loading a Program evaluates every cell, activation and destination choice, and the first commit reports changes relative to that loaded state.

### 12.4 Evaluation never fails

Evaluation has no runtime errors. Integer arithmetic saturates (§5.2), division and remainder by zero yield 0, indexes clamp (§5.5), conversions saturate (§5.8), and floats follow IEEE 754. The only runtime errors are rejected host calls: a write of the wrong type, or a write through a handle whose input no longer exists (§13.2).

### 12.5 Determinism

The same Program with the same sequence of writes and commits produces bit-identical values on every target. Built-in float functions use one implementation on every target. `rand(seed, n)` forms the 64-bit value `seed * 4294967296 + n` with wrapping arithmetic, applies one SplitMix64 step, and divides the top 24 bits of the result by 2²⁴.

## 13. Reload

A reload replaces a running Program with a newly compiled one.

### 13.1 Acceptance

A reload is all-or-nothing and happens between commits. A Program compiled against a different vocabulary is refused, and the running Program continues. Source that fails to compile produces no Program, so the running Program continues.

### 13.2 Inputs

- An input with the same name and type keeps its value.
- An input the host has never written follows its new initializer. Once the host has written an input, the host's value stays.
- An input whose type changed takes its new initial value.
- An array input whose length changed keeps the elements both lengths share and takes the rest from the new initializer.
- A removed input is dropped, and a handle to it rejects writes.
- A handle stays valid across reloads as long as an input of its name exists.
- Writes staged before the reload apply to the new Program by input name and are dropped when their input is gone.

### 13.3 Matching

Each instance and wire of the new Program is matched with at most one of the old:

1. Named instances and wires match by path.
2. Within each parent's sequence, anonymous entries match by a longest common subsequence over their kinds and constant attribute values; ties favour earlier positions.
3. Anonymous entries left unmatched between the same matched neighbours pair up in order when their kinds agree, and match with changed attributes.
4. Everything else is removed or added.

### 13.4 Report

A reload reports, for instances and wires, the matched pairs (old and new), additions, removals, and the parents whose sequence order changed. It then reports the attribute, activation and destination changes of matched items in the form a commit uses (§12.3). Cells are recomputed from the new Program; no cell carries state.

## 14. Not part of SymphonyScript

SymphonyScript has none of the following:

- **Declared kinds or types.** Node kinds, wire kinds, enums and units come from the vocabulary. There are no records, tuples, maps, or enums with data.
- **Mutation and loops.** Bindings never change. The only loop is the compile-time `for`. Patterns do not recurse, and functions recurse only at compile time (§6.3).
- **State across commits.** Every cell is a pure function of inputs. Time-dependent behaviour comes from inputs the host writes, such as `bar`, `beat` and `time`, combined with pure functions and `rand`.
- **Varying structure.** The instances and wires of a Program are fixed. Gates vary activation, and destinations vary among fixed candidates. A different structure is a different Program, loaded by reload.
- **Runtime strings and node references.** Strings exist only at compile time. Node references exist only as destinations and pattern arguments.
- **Closures.** No construct takes a function as a value.
- **Modules.** There are no imports, exports or visibility levels; one namespace spans the project.
- **Annotations.** No construct attaches metadata to a declaration.
- **Bitwise operators.**
- **Readable gate state.**
- **Aborting evaluation.** No evaluation stops the program.
