# SymphonyScript Specification

This document defines SymphonyScript: what a source program may contain, what it means, and how the runtime evaluates it. It is normative. `ARCHITECTURE.md` describes how the implementation realizes it.

## 1. Overview

### 1.1 Programs and vocabularies

SymphonyScript describes graphs: nodes placed inside other nodes, wires between nodes, and values that change while the graph runs. Music graphs (sessions, tracks, clips, notes) and audio graphs (oscillators, filters, mixers) are written the same way.

A program is compiled against exactly one vocabulary. The vocabulary declares every node kind, wire kind, enum, unit and prelude declaration a program may use, together with the rules for combining them (§10). Source code declares no kinds. It places instances of the vocabulary's kinds, sets their attributes, connects them with wires, and binds attributes to values that change at runtime.

Compilation produces a Program (§11): a complete description of instances, wires, inputs and computed values. A host loads the Program into the runtime, writes inputs and commits (§12). Consumers read the Program and the reported changes and interpret them. The meaning of a kind belongs to its consumer, not to the language.

```sym
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

Productions use EBNF: `::=` defines, `|` separates alternatives, `?` `*` `+` mark optional, repeated and one-or-more, `( )` groups, quoted text is literal, and `'a' … 'z'` is a range of characters. `NEWLINE`, `INDENT` and `DEDENT` are layout tokens (§2.2). `Name` is an identifier (§2.4). Every production appears once, in the section of the construct it defines.

Examples are fenced blocks tagged by what they hold: `sym` for a program, `sym-error` for a rejected program, `ebnf` for productions and `text` for anything else. Every `sym` example parses, and compiles when it declares every name it uses; a name it uses without declaring stands for a declaration elsewhere in the same program. Every `sym-error` example is rejected for the reason the text beside it gives. Examples use the example vocabulary (§1.3).

### 1.3 Example vocabulary

| Node kind | Top level | Attributes (the default attribute marked "default") | Accepts |
|---|---|---|---|
| `Session` | yes, at most one; entry kind | `tempo: f64 = 120` (default) | `Track` |
| `Track` | yes | `label: string = ""` | `Piano`, `Synth` |
| `Clip` | yes | `gain: f32 = 1`, `level: f32 = 1` | `Note` |
| `Note` | no | `pitch: i32 = 60` (default), `velocity: f32 = 1`, `brightness: f32 = 0.5`, `length: f64 = 0.25` in beats, `accent: bool = false` with flag `'` | none |
| `Kit` | yes | none | `Sampler` |
| `Sampler` | no | `file: string` (default) | none |
| `Piano` | no | none | none |
| `Synth` | yes | none | `Osc`, `Filter`, `Mixer`, `Output` |
| `Osc` | yes | `freq: f64 = 440` (default), `wave: Wave = Sine`, `rate: f32 = 1` | none |
| `Filter`, `Mixer`, `Output` | yes | `Filter` has `cutoff: f32 = 1000` (default) | none |

| Wire kind | From → to | Attributes (the default attribute marked "default") | Cyclic |
|---|---|---|---|
| `Plays` | `Clip` → `Clip` | `gain: f32 = 1` (default), `transpose: i32 = 0` | yes |
| `Starts` | `Clip` or `Track` → `Clip` | none | yes |
| `Includes` | `Session` → `Track` | none | no |
| `Feeds` | `Osc` or `Filter` → `Filter`, `Mixer` or `Output` | none | no |
| `Modulates` | `Osc` → `Filter` | `depth: f32 = 1` (default) | no |

The enum `Wave` has the variants `Sine`, `Saw` and `Square`. The unit family `beats` has the base unit `beats` and the unit `bars` with scale 4. The unit family `time` has the base unit `s` and the unit `ms` with scale 0.001. Every count not stated above is "any number". The example vocabulary has no prelude.

## 2. Source text

### 2.1 Files

A project is a set of `.sym` files compiled together. All files share one namespace (§6.5); no file imports another. Files are ordered by the byte-wise order of their paths, and items within a file by source order. This order is the program's top-level order.

```ebnf
File    ::= TopItem*
TopItem ::= InputDecl | LetDecl | FnDecl | PatternDecl | TopPlacement
```

### 2.2 Layout

Indentation is significant. Leading whitespace consists of spaces; a tab in leading whitespace is an error. Between tokens, spaces and tabs are whitespace. A line ends at LF or at CRLF. Columns count Unicode scalar values from the start of the line. Blank lines and lines holding only a comment emit no tokens and do not affect layout.

The lexer keeps a stack of indentation columns, starting with column 0. A line indented deeper than the top of the stack pushes its column and emits `INDENT`. A line indented less pops columns, emitting one `DEDENT` per popped column, and must land exactly on a column still on the stack. Every line holding a token ends with `NEWLINE`, including a last line with no line break after it. At end of input the lexer emits a `DEDENT` for every column on the stack above 0.

Inside `( )` and `[ ]`, layout is suspended: newlines are whitespace and no layout tokens are emitted.

An attribute clause (§7.6) continues onto each following line whose first token sits exactly at the column of the clause's first entry, while that column is deeper than the top of the stack and no `:` has closed the clause; `::` closes nothing. The line break before such a line emits no `NEWLINE`, and the line emits no `INDENT` or `DEDENT`. The lexer tracks each clause's first-entry column for this rule, and a line that misses the column only by the display width of wide characters draws a diagnostic that shows the column.

A semicolon outside strings and comments is an error.

### 2.3 Comments

`//` starts a comment that runs to the end of the line. There is no other comment form. Inside a string, `//` is text.

### 2.4 Identifiers

An identifier starts with a character of the Unicode property XID_Start, `_` or `#`, continues with characters of XID_Continue and `#`, and holds at least one character other than `_` and `#`. `#` behaves as a letter, so note names such as `C#4` and `F#3` are identifiers. Identifiers are compared, and stored in paths, in Unicode NFC form, so a name reads the same in every normalization form. A vocabulary's names are ASCII identifiers (§10.1).

By convention, kinds, patterns and enum variants are written in PascalCase, and everything else in snake_case. The compiler does not enforce the convention.

### 2.5 Keywords and reserved tokens

Keywords are never names:

```text
input let fn pattern default
if then else match when for in as
is not and or div to true false
i32 i64 f32 f64 bool
```

The words `from`, `recurrent` and `string` are reserved and are never names. The tokens `^`, `==` and `!=` are reserved and rejected by the lexer; the diagnostic for `==` and `!=` points to `is` and `is not`.

### 2.6 Literals

```ebnf
NumberLit   ::= (Digits | FloatLit) Suffix? | PrefixedInt
PrefixedInt ::= '0x' HexDigits | '0b' BinDigits | '0o' OctDigits
FloatLit    ::= Digits '.' Digits Exponent? | Digits Exponent
Exponent    ::= ('e' | 'E') ('+' | '-')? Digits
Digits      ::= Digit ('_'? Digit)*
HexDigits   ::= HexDigit ('_'? HexDigit)*
BinDigits   ::= BinDigit ('_'? BinDigit)*
OctDigits   ::= OctDigit ('_'? OctDigit)*
Digit       ::= '0' … '9'
HexDigit    ::= Digit | 'a' … 'f' | 'A' … 'F'
BinDigit    ::= '0' | '1'
OctDigit    ::= '0' … '7'
Suffix      ::= TypeSuffix | UnitSuffix
TypeSuffix  ::= 'i32' | 'i64' | 'f32' | 'f64'
UnitSuffix  ::= Name
BoolLit     ::= 'true' | 'false'
StringLit   ::= '"' (Char | Escape | '{' Expr '}')* '"'
Char        ::= any character except '"', '\', '{' and a line break
Escape      ::= '\"' | '\\' | '\n' | '\t' | '\{'
```

- A single underscore may separate two digits: `1_000_000`. Anywhere else it is an error.
- A float needs digits on both sides of the point: `0.5`, not `.5`.
- In a decimal literal, `e` or `E` starts an exponent only when a digit, or a sign and a digit, follows it; otherwise it starts a suffix. `2e3` is 2000.0, and `2eighths` is 2 in a unit named `eighths`.
- `0x`, `0b` or `0o` starts a prefixed literal only when a digit of its base follows; otherwise the `0` is a decimal literal and the letters start a suffix, so `0bars` is 0 bars.
- A suffix follows a decimal literal with no whitespace. A type suffix fixes the literal's type: `5i64`, `0.5f64`. An integer type suffix on a float literal, as in `0.5i32`, is an error. A unit suffix is a unit the vocabulary declares: `200ms`, `2bars` (§3.6). Any other suffix is an error.
- A prefixed literal takes no suffix, and a letter or digit directly after its digits is an error: `0b12` is an error, and `0xFFf32` is the integer 0xFFF32.
- An integer literal whose magnitude is 2¹²⁷ or more is an error (§4.6).
- A minus sign is always an operator (§5.1). The fit check treats a minus applied directly to a literal as one value (§4.1).
- A string occupies one line. `{expr}` inside a string is interpolation (§5.9), and the expression may contain strings of its own. `\{` is a literal brace, and `}` outside an interpolation is an ordinary character.

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

An array has a fixed length of at least 1 and one element type. `[60, 64, 67]` has length 3, and its element type follows §4. Elements may themselves be arrays of equal length: `[[60, 64, 67], [62, 65, 69]]` holds two rows of three, and with `i32` elements its type is written `i32[3][2]`.

An array's length is fixed at compile time. Its contents may change at runtime when its elements are reactive (§5.5).

```ebnf
Type       ::= (ScalarType | Name) ('[' Digits ']')*
ScalarType ::= 'i32' | 'i64' | 'f32' | 'f64' | 'bool'
```

A `Name` in a type is a vocabulary enum. A length in a type is at least 1.

### 3.3 Strings

Strings exist only at compile time. They are written as literals, built with `+` and interpolation (§5.9), and bound to string attributes the vocabulary declares. No input, cell or runtime value is a string, and a string expression that reads a cell is an error.

```sym
Kit drums:
    for i in 0..8:
        Sampler | file="kit/{i}.wav"
```

### 3.4 Enums

The vocabulary declares enums. Every variant is a plain name with no data. Where the expected type is an enum (§4.2), a variant is written bare: `wave=Saw`. Anywhere else it is qualified with the enum's name: `Wave::Saw`.

### 3.5 Node references

A node reference names an instance. A Program's instances are fixed at compile time (§14), so every node reference is a compile-time value. Node references appear only as wire destinations (§8.2), as pattern arguments (§6.4), at the start of attribute paths (§5.6), as elements of a `for` iterable (§7.9) and as the values of compile-time `let`s (§6.2), alone or in arrays. They are never stored in an input, a reactive `let` or an attribute.

### 3.6 Units

The vocabulary declares unit families. Each family has a base unit and further units with a positive scale relative to the base: in the family `time`, `s` is the base and `ms` has scale 0.001.

- A literal with a unit suffix converts to its family's base unit at compile time: `200ms` is 0.2 in `time`. Its value is the literal times the unit's scale, computed exactly and rounded once to the type of its use. It is a flexible value (§4.1): an integer when the literal is an integer and the scale is an integer, as `2bars` is 8, and a float otherwise, as `1000ms` and `1.0s` are.
- An attribute may belong to a family and then has a default unit. A plain number bound to it is in that default unit.
- A unit value bound to an attribute of a different family, or of no family, is an error.
- `+`, `-`, `%`, comparisons, `min`, `max`, `clamp`, the first two arguments of `lerp`, array literals, and the arms of `if` and `match` take operands of one family. A plain value among them, a number or a plain cell, takes that family and is read in the default unit of the attribute the expression feeds: `length=2bars + 1` is 9 beats. Where the expression feeds no attribute of the family, the plain value is an error, and so is an expression that feeds two attributes of the family with different default units.
- `/` and `div` of two values in the same family yield a plain number. Multiplying or dividing a unit value by a plain number keeps the family. Dividing a plain number by a unit value, multiplying two unit values, `**` on a unit value, and a unit value as a range bound are errors.
- Unary minus, conversions, `abs`, `floor`, `ceil` and `round` keep the family of their operand. The other built-ins take plain numbers.
- A value's family comes from its own initializer or expression, never from its uses: `input d = 5ms` and `let e = d * 2` are in `time`, and `input p = 5.0` is plain. A value in a family holds base units, and the host writes such inputs in base units (§12.1). A plain cell bound to an attribute of a family is read in that attribute's default unit, as a plain number is; a plain cell read at two attributes of one family with different default units is an error naming both.

Families never convert into each other. A family whose relation to another depends on runtime values, such as beats and seconds under a changing tempo, stays separate, and the consumer converts.

```sym
Clip intro:
    Note | length=2bars
    Note | length=0.5
```

The first note lasts 8 beats; the second lasts 0.5 beats, the attribute's default unit.

## 4. Type inference

Types come from the places values are used; source states a type only to fix one outright (§4.5).

### 4.1 Fixed and flexible values

A value has a fixed type when something fixes it: an attribute, a suffix, an annotation, `rand`, or an operand of fixed type in the same expression. A cell fixes its own type only where its use expects none (§4.4). Every other compile-time value is flexible: an unsuffixed literal, `pi`, and any compile-time `let`, operation or built-in call built only from flexible values. Each use of a flexible value chooses its type independently, and the value must fit the chosen type.

```sym
let base = 48

Clip melody:
    Note/(base + 7)

Synth lead:
    Osc | freq=(base * 2.5)
```

`base` is `i32` in the pitch and `f64` in `freq`.

The fit check applies to every flexible value once it is computed: an integer fits a float type when its value is exactly representable, and a float fits a float type when it rounds to a finite value. A minus applied directly to a literal forms one value for this check, so `-2147483648` fits `i32`.

```sym-error
Clip melody:
    Note/300000000000
```

This is an error: the value does not fit the pitch's `i32`.

A position that fixes no type, such as a comparison between flexible values, an interpolation or the argument of an integer conversion, gives a flexible value the defaults of §4.5. The argument of a float conversion takes the conversion's type. The operands of a comparison first give each other their types, so in `cutoff > 0.5` the literal takes `cutoff`'s type.

When one flexible value has an integer use and a float use, the float use's value equals the integer use's value converted to the float type; otherwise it is an error naming both uses. `let q = 2 ** -1` is 0 as a pitch and 0.5 as a frequency, so using it as both is an error.

### 4.2 Expected types

The type a position requires flows into the expression filling it: through operators, `if` and `match` arms, array literals and function arguments, down to the flexible values inside. It stops at the operands of comparisons, at an index, and at the operands of `div` when it is a float type; those take their types from each other or from the defaults of §4.5, and the result then widens into the position. An integer type flows through `div`, so with `input n = 4`, `Note/(n div 2)` reads `n` as an `i32`. A fixed-type operand meeting an expected type widens into it (§4.3), a cell is read in it as §4.4 says, and any other fixed-type operand is an error.

```sym
input cutoff = 0.5

Clip melody:
    Note | brightness=cutoff * 0.8
```

With `brightness` of type `f32`, `0.8` is an `f32`, and `cutoff` is read as one.

The operands of `is` and `is not` give each other their expected type, so in `wave is Saw` the variant resolves in `wave`'s enum. A `let` whose initializer needs an expected type, such as a bare variant, takes it from its uses when they all expect one type.

### 4.3 Widening

These implicit widenings exist, and no others: `i32` to `i64`, `i32` to `f32`, `i32` to `f64`, `i64` to `f64`, `f32` to `f64`. A value never narrows implicitly, and a float never becomes an integer implicitly; conversions are explicit (§5.8). Widening `i32` to `f32` and `i64` to `f64` rounds integers beyond the float's exact range: an `i32` cell holding 16777217 reaches an `f32` attribute as 16777216.

When an operator's operands have different fixed types, both widen to the smallest type both widen into. With no such type, the expression is an error. A flexible float meeting an operand of fixed integer type makes the operation a float one, `f64` for an `i64` operand and `f32` for an `i32` one, and the integer widens into it: with `input n = 4`, `n * 0.5 > 1` computes in `f64`. Built-ins widen their operands as operators do.

### 4.4 Cell types

An input, and a `let` that reads an input (a reactive `let`), is a cell: it holds one value at runtime, so it has exactly one type.

An input's type comes from its initializer alone, never from its uses. Its kind is the initializer's (§4.5): integer, float, `bool` or one particular enum, with the initializer's array shape. An integer input is an `i64` and a float input an `f64`, unless an annotation gives the type (§4.5), and the initializer computes in that type. Each use reads the input in the type the use expects: an integer input saturates into a narrower integer type and converts into a float type, rounding beyond the float's exact range, and a float input rounds into `f32`. Where the use expects no type, the input is read in its own type. A float input used where an integer is expected is an error; a conversion (§5.8) states the rounding. The initial value must fit every use.

```sym
input cutoff = 0.5

Clip melody:
    Note | brightness=cutoff

Synth lead:
    Osc | freq=cutoff * 4000
```

`cutoff` is an `f64` input. It reaches `brightness` rounded to `f32`, and `freq` at full precision.

An integer input whose every use expects a float draws a warning, because its host cannot write a fraction; `1.0` in place of `1` makes it a float input.

A reactive `let` is typed as an input is, except for its width. Its kind comes from its initializer (§4.5); its width is the widest its uses of that kind expect; and each use reads it in the use's type, as each use reads an input. A float `let` used where an integer is expected is an error. Its uses include the operands it feeds in other reactive `let`s' initializers, so expected types flow from attributes back through reactive `let`s to the inputs they read. A reactive `let` whose uses fix no width takes the default of §4.5, and defaults apply to a reader before the `let`s it reads:

```sym
input a = 1
let b = a + 0.5

Clip c:
    Note when b > 1
```

`b` is a float `let` with the default width `f32`, and its initializer reads `a`, an `i64` input, as an `f32`.

An array `let` whose elements read cells is an array of bindings (§5.5), not one cell. An element read with a literal index is its own binding with its own type; an element read through a runtime index is read in that read's type.

The compiler resolves cell types once over the whole program. This always terminates, because cells form no cycles (§6.2).

### 4.5 Defaults and annotations

A cell's kind is its initializer's: `bool`, an enum, integer or float, element by element for an array. A numeric initializer is a float one when its result is a float under §5: when it holds a float literal, `pi`, `/`, a float built-in, `rand`, a float conversion or a float unit value, or reads a float cell or a float compile-time `let`. Every other numeric initializer is an integer one.

A reactive `let` whose uses fix no width takes `f32` when it is a float, and the widest integer type among its operands when it is an integer, so a `let` reading an `i64` input stays `i64`. A flexible value in a position that fixes no type (§4.1) takes `i32`, or `i64` when its value does not fit `i32`, and a flexible float takes `f32`, or `f64` when its value lies outside `f32`'s finite range.

An annotation fixes a type outright: `input fine: f32 = 0` is a float input stored as `f32`, and its uses read it as §4.4 says. A suffix does the same for a literal: `0f64`.

### 4.6 Compile-time evaluation

A compile-time value is computed by the rules of §5, except that flexible integers compute exactly and a fixed-type integer operation that saturates is an error; a conversion saturates as §5.8 says and is no error. A flexible integer whose magnitude reaches 2¹²⁷ is an error. A flexible float computes in the type its use chooses (§4.1).

- The bounds of a range in a `for`, and an index known at compile time, are integers that fit `i64`. A range's loop variable is a flexible integer, so each use chooses its type.
- A call to a recursive function (§6.3) computes flexible integer arguments exactly and every other argument in the type its call gives it: a fixed type, or for a flexible float its expected type or the default of §4.5. Its whole body computes in those types, and an operation they do not allow, such as `div` on a float, is an error naming the call.
- A value that structure depends on, such as a range bound or a recursive call's result, is computed before the expansion that depends on it.
- A compilation evaluates at most 1,000,000 distinct recursive calls, and its expansion yields at most 1,000,000 instances and wires. A call repeats an earlier one, counts once and reuses its value, only when its argument values, argument types and expected type are all equal. Exceeding either limit is an error naming the call or the expansion.

In §6.4's `Arp`, `steps` counts the iterations exactly, and `i` is an `i32` in each pitch. `Note/(2147483647 + 1)` is an error, as `Note/2147483648` is.

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

```ebnf
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
            | 'to' | Conversion | ArrayLit | '(' Expr ')'
ArrayLit  ::= '[' Expr (',' Expr)* ','? ']'
Args      ::= Expr (',' Expr)* | NamedArg (',' NamedArg)*
NamedArg  ::= Name '=' Expr
```

Comparisons do not chain: `a < b < c` is an error. `-2 ** 2` is −4, and `n-1` is `n - 1`. An `if` that is an operand of another operator is parenthesized: `base + (if high then 12 else 0)`. An `if` that is a whole value, such as a `let`'s value, an attribute value or an argument, needs no parentheses.

Expressions evaluate the same way at compile time and at runtime; §4.6 states the differences in compile-time arithmetic.

### 5.2 Arithmetic

`+`, `-` and `*` take numeric operands of one type after widening; `+` also concatenates strings (§5.9). Every integer operation saturates at the type's bounds: `2147483647 + 1` in `i32` is 2147483647, and negating −2147483648 in `i32` yields 2147483647.

`/` always yields a float. Integer operands convert to the result's float type: the expected type when one exists, otherwise `f64` if either operand is `i64` or `f64`, otherwise `f32`. `1 / 4` is 0.25.

`div` takes integers and rounds toward negative infinity: `-7 div 2` is −4, and `-2147483648 div -1` in `i32` is 2147483647. `%` takes integers or floats, and its result has the sign of the divisor: `-7 % 2` is 1, `-1 % 7` is 6, and an integer `x % -1` is 0. Integer `div` or `%` by zero yields 0. On floats, `a % b` is `fmod(a, b)`, plus `b` when that result is neither zero nor NaN and its sign differs from `b`'s; the sum can round to `b` itself, so `-1e-20 % 1.0` is 1.0. A zero float result takes the divisor's sign, so `-2.0 % 1.0` is 0.0.

`**` raises to a power. With integer operands it yields an integer of their type, saturating; a negative exponent yields 0, except that base 1 yields 1 and base −1 yields 1 or −1 by the exponent's parity, and `0 ** 0` is 1. With a float operand it yields a float of the operands' type after widening. In an integer position `2 ** 4` is the integer 16. `2 ** (7 / 12)` is the frequency ratio of a fifth, a float because `7 / 12` is one.

Float arithmetic follows IEEE 754, including infinities and NaN: a float divided by zero is an infinity or NaN, and a float `%` by zero is NaN.

### 5.3 Comparison and logic

`is` and `is not` compare two scalars, enum values or strings of one type after widening; floats compare by IEEE 754 equality. `<`, `<=`, `>` and `>=` compare numbers. `and`, `or` and `not` take and yield `bool`.

### 5.4 Conditionals

`if c then a else b` yields `a` when `c` is true and `b` otherwise. The condition is a `bool`. Both branches are type-checked, and only the chosen branch is evaluated; at compile time only the chosen branch is checked for fit and index range. `else` is mandatory, and `else if` chains:

```sym
input loud = false
input quiet = false
let gain = if loud then 0.9 else if quiet then 0.3 else 0.6
```

`match` selects a value by an enum, an integer or a `bool`:

```sym
input wave = Wave::Saw

let level = match wave:
    Sine: 1.0
    Saw: 0.8
    else: 0.6
```

```ebnf
MatchBlock ::= 'match' Expr ':' NEWLINE INDENT MatchArm+ ElseArm? DEDENT
MatchArm   ::= (Name | '-'? NumberLit | BoolLit) ':' Expr NEWLINE
ElseArm    ::= 'else' ':' Expr NEWLINE
```

- A scrutinee of another type is an error. Arm labels are variant names of the scrutinee's enum, or literals of its type and unit family. A label that repeats an earlier one is an error.
- A `match` on an enum or a `bool` covers every value or ends with `else:`, and an `else:` after arms that cover every value is an error. A `match` on an integer ends with `else:`.
- The first matching arm wins. Arms are checked and evaluated as `if` branches are, and arm values unify as `if` branches do (§4.2).
- `match` is the whole value of a `let`, top-level or local, of a function body or of a wire destination (§8.2), never an operand: its arms need layout, and layout is suspended inside parentheses.

### 5.5 Arrays and indexing

`a[i]` reads element `i`, counting from 0. The index is an integer of either width, and a flexible index takes the defaults of §4.5. An index below 0 reads the first element, and an index past the end reads the last. A compile-time index outside the array is an error.

An array literal whose elements read cells is reactive, and each element is bound separately:

```sym
input a = 1
input b = 2
input i = 0
let c = [a, b, a + b]
```

- `c[1]` with a literal index is the same binding as `b`. A change to `a` does not reach it.
- `c[2]` is a cell of its own, computed from `a` and `b`.
- `c[i]` depends on `i` and on every element.

An array-valued expression that reads cells is evaluated element by element. With `let row = grid[i]`, `row[1]` is `grid[i][1]`, and `if flag then [60, 64] else [62, 65]` chooses each element separately.

Operators apply to scalars only. `[1, 2] + [3, 4]` is an error.

### 5.6 Attribute reads

`x.name` reads attribute `name` of the instance or wire `x`. The read is the same binding the attribute holds: a constant, an input, a cell, or the vocabulary default when the source sets none.

- A path starts at a name in scope (§6.5), a node-reference parameter or `let` included, or at a node reference chosen at compile time by an index or an `if`: `sections[0].gain`. It steps through children, named wires and the names inside named pattern uses: `chorus.back`, `lead.v1.osc`.
- A path whose start is chosen at runtime is an error; `to.name` reads an attribute of a wire's destination, which may be chosen at runtime (§8.3).
- Reading an attribute the kind does not declare is an error.
- Reads that form a cycle are an error naming the cycle.
- Reads ignore gates, and activation is not readable (§9.3).

```sym
Clip verse | gain=0.8:
    Note as hook /65 | velocity=0.9

Clip chorus | gain=verse.gain * 1.2:
    Note/72 | velocity=verse.hook.velocity
    Plays as back | transpose=-2: verse

Clip outro | gain=chorus.back.transpose:
    Note/60
```

### 5.7 Calls and built-ins

A call invokes a built-in, a prelude function or a source function (§6.3). Arguments are all positional or all named, as in `scale(3, 48)` or `scale(degree=3, root=48)`; mixing them in one call is an error. Built-ins take positional arguments only. Built-in names are top-level names (§6.5).

| Built-in | Result |
|---|---|
| `abs(x)`, `min(a, b)`, `max(a, b)`, `clamp(x, lo, hi)` | numeric, of the operands' type |
| `floor(x)`, `ceil(x)`, `round(x)` | float; `round` rounds half away from zero |
| `sqrt(x)`, `sin(x)`, `cos(x)`, `tan(x)`, `exp(x)`, `log(x)` | float; `log` is the natural logarithm |
| `lerp(a, b, t)` | float, `a + (b - a) * t` |
| `rand(seed, n)` | `f32` in [0, 1), a fixed function of two integers (§12.5) |
| `pi` | the float constant π |

- A call whose arguments are all flexible is flexible (§4.1), and `pi` is flexible. `rand` always yields `f32`.
- Integer arguments to float built-ins convert as `/` does.
- `abs` saturates on integers: `abs` of −2147483648 in `i32` is 2147483647.
- `clamp(x, lo, hi)` is `min(max(x, lo), hi)`, so with `lo` above `hi` it yields `hi`.
- `min` and `max` with one NaN operand yield the other operand. `min(-0.0, 0.0)` is −0.0, and `max(-0.0, 0.0)` is 0.0.

Domain functions, such as converting a note number to a frequency, come from the vocabulary's prelude (§10.1).

### 5.8 Conversions

```ebnf
Conversion ::= ('i32' | 'i64' | 'f32' | 'f64') '(' Expr ')'
```

`i32(x)`, `i64(x)`, `f32(x)` and `f64(x)` convert a number explicitly. Float to integer truncates toward zero. Values outside the target range saturate to its bounds, and NaN becomes 0. Converting a `bool` or an enum value is an error.

### 5.9 Strings

`+` concatenates strings. `{expr}` inside a literal interpolates a compile-time value:

- an integer in decimal;
- a float as the shortest decimal that reads back as the same value, without an exponent and with `.0` when it has no fraction: `2.0`, `0.1`, `-0.0`; infinities and NaN as `inf`, `-inf` and `nan`;
- a `bool` as `true` or `false`, an enum variant by name, and a string as its text;
- a unit value as its value in the family's base unit.

## 6. Declarations

### 6.1 Inputs

```ebnf
InputDecl ::= 'input' Name (':' Type)? '=' Expr NEWLINE
```

An input is a cell the host writes (§12.1). Its initializer is a compile-time value and gives the input its type (§4.4). An input may be an array; the host then writes single elements or the whole array.

```sym
input cutoff = 0.5
input steps = [true, false, false, true, false, false, true, false]
```

### 6.2 Lets

```ebnf
LetDecl ::= 'let' Name (':' Type)? '=' (Expr NEWLINE | MatchBlock)
```

A `let` names a value. A `let` that reads no input, directly or through other declarations, is a compile-time value (§4.1). A `let` that reads an input is reactive: a cell, or for an array an array of bindings (§4.4), recomputed when what it reads changes (§12.2). A declaration that reaches itself through `let`s, inputs, attribute reads or function bodies is an error naming the cycle, as in `let a = b + 1` with `let b = a + 1`; a function reaching itself through calls follows §6.3.

A compile-time `let` may hold a node reference or an array of them. It stands for its value wherever it is used and never reaches the Program. A `let` holding node references that reads an input is an error; a choice among instances at runtime is written in the destination itself (§8.2).

```sym
input section = 0

let sections = [verse, chorus, bridge]

Clip intro:
    for s in sections:
        Starts: s

Clip outro:
    Plays: sections[section]
```

### 6.3 Functions

```ebnf
FnDecl   ::= 'fn' Name '(' Params? ')' ':' (Expr NEWLINE | NEWLINE INDENT FnBody DEDENT)
FnBody   ::= LocalLet* (Expr NEWLINE | MatchBlock)
LocalLet ::= 'let' Name '=' (Expr NEWLINE | MatchBlock)
Params   ::= Param (',' Param)*
Param    ::= Name ('=' Expr)?
```

A function computes one value from its parameters.

- A body is one expression, or local `let`s followed by one expression or `match`. Two local `let`s with one name, or a local `let` named like a parameter, are an error.
- A call to a non-recursive function evaluates the body in the function's own scope, with each parameter bound to its argument, and the argument evaluated in the caller's scope. The compiler inlines the call and checks the result in place. Parameter types come from each call's arguments and expected type. A function never appears in the Program.
- A call whose arguments read cells yields a reactive value.
- A parameter with a default follows every parameter without one, and its default may read earlier parameters. A positional call may omit trailing parameters that have defaults; a named call may omit any parameter that has a default.

```sym
fn db(x): 10 ** (x / 20)

fn scale(degree, root = 60):
    let octave = degree div 7
    root + [0, 2, 4, 5, 7, 9, 11][degree % 7] + 12 * octave
```

A function that reaches itself through calls, directly or through other functions, is recursive. Every call to a recursive function has compile-time arguments, and the compiler evaluates it to a value (§4.6). A recursive function whose body reads a cell is an error, and so are a call to one with an argument that reads a cell and evaluation deeper than 1,024 nested calls; these diagnostics name the chain of calls.

```sym
fn fib(n): if n < 2 then n else fib(n - 1) + fib(n - 2)

let lengths = [fib(3), fib(4), fib(5), fib(6)]
```

### 6.4 Patterns

```ebnf
PatternDecl   ::= 'pattern' Name '(' PatternParams? ')' ':' NEWLINE INDENT LocalLet* BodyItem+ DEDENT
PatternParams ::= PatternParam (',' PatternParam)*
PatternParam  ::= 'default'? Name ('=' Expr)?
```

A pattern is a named, parameterized group of placements.

- Patterns are declared at top level. The body holds `let` lines first and then any body items (§7.1). Each expansion has its own `let`s, visible throughout the body. Two `let`s with one name, or a `let` named like a parameter, are an error.
- A pattern is used with placement syntax. The attribute clause supplies parameters in every entry form of §7.6, so `name` and `!name` supply a `bool` parameter, and `/expr` supplies the parameter marked `default`, of which there is at most one. `/expr` on a pattern without a `default` parameter is an error. Supplying a parameter twice is an error, and so is omitting a parameter that has no default value. A use carries no flags and no body.
- A use is replaced by the body's placements at the use's position, with each parameter bound to its argument as in a function call (§6.3), and the result is checked in place. Parameter types come from each use's arguments; an argument that needs an expected type, such as a bare variant in `Voice | shape=Saw`, takes it from the parameter's uses in the body. An argument may be a value, a reactive value, a node reference or an array of node references.
- A wire placed directly in the body has as its source the instance whose body holds the use.
- A pattern body sees top-level names, its parameters, its `let`s and its own names (§6.5), never the names around its use. A top-level name written in the body resolves as usual, but the body has no name for the instance its own use creates.
- Names inside an anonymous use are unreachable and never clash with names outside it. A named use (`Voice as v1`) makes them reachable through its name: `v1.osc`.
- A pattern may use other patterns, but no pattern reaches itself through uses.
- The vocabulary checks (§10.2) apply to the expanded placements. A diagnostic points at the use and at the line inside the pattern.
- A pattern whose body is a single node placement, with or without a body of its own, or a single use of such a pattern, stands in for that placement: a use creates that one instance, and the use's name becomes its name, replacing any `as` name the placement has. Only such uses appear at top level.
- A `when` on a use gates every placement it expands to.

```sym
pattern Arp(default root, steps = 4, gap = 4):
    for i in 0..steps:
        Note/(root + i * gap)

Clip verse:
    Arp/48
    Arp/60 | steps=8 gap=3
```

```sym
pattern Riff(default root):
    Clip:
        Note/root Note/(root + 7) Note/(root + 12)

Riff low /36
Riff high /48
```

A pattern that needs the instance its use creates takes it as an argument:

```sym
pattern Loop(default root, again):
    Clip:
        Note/root Note/(root + 7)
        Plays: again

Loop low /36 | again=low
```

Patterns never appear in the Program.

### 6.5 Names and scope

All top-level names share one namespace across the project: inputs, `let`s, functions, patterns, top-level instances, the built-ins (§5.7), and the vocabulary's kinds, enums and prelude declarations. A duplicate is an error naming both declarations; a source declaration shadows a prelude declaration of its name (§10.1). Top-level declarations refer to each other regardless of order. A declared name of any kind, top-level or local, that equals a variant of a vocabulary enum is an error.

Inside a body, the `as` names of that body's children and wires are visible regardless of order, and so are those of every enclosing body up to a pattern's body, and the top-level names. A child or wire name shadows top-level names, built-ins and prelude names within its body and the bodies nested in it. A child or wire name equal to a name of its own body or of an enclosing body, or to an attribute of its parent's kind, is an error, also when the two names stand in different arms of one `when` block. A `for` variable is visible in its body, and a `for` variable equal to a visible name is an error. In a function, parameters and local `let`s shadow top-level names; in a pattern, parameters and the body's `let`s do.

A function or pattern that nothing uses has its names resolved and no types checked.

A bare name that is a variant of the enum its position expects resolves to that variant (§3.4). Every other name resolves in scope.

## 7. Placements

### 7.1 Form

A placement creates an instance of a node kind or uses a pattern. Its elements appear in a fixed order.

```ebnf
TopPlacement ::= Kind Flags? 'as'? Name DefaultArg? Gate? Attrs? (NEWLINE | Body)
Placement    ::= Kind Flags? ('as' Name)? DefaultArg? Gate? Attrs?
Kind         ::= Name
Body         ::= ':' (NEWLINE INDENT BodyItem+ DEDENT | InlineChildren NEWLINE)
BodyItem     ::= Line | WirePlacement | ForBlock | WhenBlock
Line         ::= Placement (NEWLINE | Body) | '(' Placement ')' NEWLINE | Bundle NEWLINE
               | SelfDelimited SelfDelimited+ NEWLINE
```

`DefaultArg` is §7.5's, `Gate` §9.1's and `Attrs` §7.6's. A line whose `Kind` names a wire kind of the vocabulary is a wire placement (§8.1), and a wire kind at top level is an error. Whitespace around `/` is free. In a body, `Note hook/60` reads as two placements, and its diagnostic suggests `as`.

### 7.2 Top-level placements

A top-level placement is named: `Clip verse` or `Clip as verse`. Top-level instances are independent; they relate to each other only through wires and attribute reads (§5.6). The vocabulary states which kinds may stand at top level and how many of each (§10.2).

### 7.3 Children and containment

A placement inside another's body is its child. The parent owns the child, and the child is reachable only through its parent: `verse.hook`. A child is anonymous or named with `as`.

- An instance has exactly one owner and is placed once. Other instances reach it only through wires and attribute reads.
- The vocabulary states which kinds each kind accepts as children and how many of each (§10.2).
- A body is an ordered sequence. Children, bundles and wires keep their source order in the Program (§11).
- Any kind may have a body. The body of a kind that accepts no children holds only wires.

```sym
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

```ebnf
Flags    ::= FlagChar+
FlagChar ::= "'" | '!' | '?' | '*' | '+' | '~' | '@' | '$'
```

`Note'/65` sets the attribute flagged `'`. An identifier character directly after a flag run is an error; a space separates them.

### 7.5 Default attribute

```ebnf
DefaultArg ::= '/' (Atom | '(' Expr ')')
Atom       ::= '-'? NumberLit | BoolLit | StringLit | Name ('.' Name)* | Name '::' Name | 'to' '.' Name
```

`/expr` sets the kind's default attribute, which the vocabulary designates (§10.1): `Note/60` sets the pitch. Unparenthesized, the value is a single atom: a literal, optionally negative, a name, a dotted path, a qualified variant or `to.name`. Anything else is parenthesized: `Note/(base + 7)`. `/expr` on a kind without a default attribute is an error.

### 7.6 Attribute clause

```ebnf
Attrs     ::= '|' AttrEntry+
AttrEntry ::= Name '=' Expr | Name | '!' Name
```

A single `|` opens the attribute clause, its first entry follows the `|` on the same line, and entries are separated by whitespace.

- `name=expr` sets an attribute.
- `name` sets a `bool` attribute to `true`; `!name` sets it to `false`.
- Setting an attribute twice, by any combination of entries, flags and `/expr`, is an error.
- Setting an attribute the kind does not declare is an error, and so is leaving unset an attribute the vocabulary gives no default (§10.1).

The clause continues on following lines that start at its first entry's column (§2.2), without another `|`:

```sym
input cutoff = 0.5
let wobble = cutoff * 0.8

Clip melody:
    Note/62 | velocity=0.7
              brightness=wobble
              accent
```

An attribute bound to a constant is static. An attribute bound to an expression that reads a cell tracks it (§12.2).

### 7.7 Several placements on one line

```ebnf
SelfDelimited  ::= Member | Bundle
Member         ::= Kind Flags? ('as' Name)? DefaultArg? | '(' Placement ')'
InlineChildren ::= SelfDelimited+
```

Placements on one line are separated by whitespace, and each ends without looking at the next: a kind with optional flags, `as` name and `/expr`, where a parenthesized `/expr` ends at its closing parenthesis. A placement with a gate or an attribute clause is parenthesized or written on its own line. A placement with a body owns its line, and its body may follow the colon on the same line:

```sym
Clip fill: Note/60 Note/62 (Note/64 | velocity=0.5)
```

### 7.8 Bundles

```ebnf
Bundle    ::= '[' (Member+ BundleFor? | BundleFor) ']'
BundleFor ::= 'for' Name 'in' Iterable ':' Member+
```

A bundle is one entry of its parent's sequence whose members are simultaneous: a chord.

- A bundle has at least one member after expansion. Members are node placements, which may be gated and named, and uses of patterns that expand only to node placements, whose placements become members. Wires and bundles are not members, and a member has no body, also when a pattern supplies it.
- Members are children of the bundle's parent.
- Layout is suspended inside the brackets, so members are self-delimited (§7.7). A `for` inside a bundle extends to the closing bracket.

```sym
Clip pad:
    [Note/60 Note/64 Note/67]
    [for i in 0..3: Note/(62 + i * 4)]
```

### 7.9 Compile-time `for`

```ebnf
ForBlock ::= 'for' Name 'in' Iterable ':' (NEWLINE INDENT BodyItem+ DEDENT | InlineChildren NEWLINE)
Iterable ::= Expr '..' Expr | Expr
```

A `for` repeats its body once for each value of a compile-time iterable: a range `a..b`, which counts from `a` up to but excluding `b` and is empty when `b` is not above `a`, or an array of scalars, enum values, strings or node references. The loop variable is a compile-time value. An iterable that reads a cell is an error. Placements in a `for` body are anonymous; `as` there is an error.

```sym
input steps = [true, false, false, true, false, false, true, false]
input pitches = [36, 36, 43, 36, 41, 36, 43, 48]

Clip seq:
    for i in 0..8:
        Note/(pitches[i]) when steps[i]
```

## 8. Wires

### 8.1 Wire placements

```ebnf
WirePlacement ::= Kind Flags? ('as' Name)? DefaultArg? Gate? Attrs? ':' Destination
```

A wire connects its source to its destination and carries attributes. Its source is the instance whose body it is written in. A wire may appear anywhere in that body's sequence, and its position is kept (§7.3); what the position means is the consumer's.

```sym
Clip intro:
    Note/48 Note/52
    Starts: drums
    Note/55
    Plays | transpose=5: verse
```

Flags, `/expr` and the attribute clause work for wires as for nodes (§7.4–§7.6). The vocabulary states which kinds each wire kind connects (§10.2).

### 8.2 Destinations

```ebnf
Destination ::= Expr NEWLINE | MatchBlock
```

A destination is a node reference: a name or a path, an `if` choosing between node references, an array of node references indexed by a value, or a `match` yielding node references, which may start on the wire's line: `Plays: match section:`.

- A destination is always one of a fixed set of candidates written in the source, directly or through compile-time values such as `let`s, `for` variables and pattern parameters. A destination that reads cells chooses among them at runtime. Candidates are listed left to right as written, those of nested choices included, and one index chooses among them, clamped as an array index is (§5.5). A candidate written twice counts once in the checks of §8.4 and §10.2.
- Every candidate is checked against the wire kind's endpoints (§10.2).
- A wire never resolves to nothing. A gate disconnects it: `Plays when looping: verse`.
- A destination names an instance placed elsewhere. It is never an inline placement or a pattern use written in the destination, and it contains no `to`.

```sym
input section = 0
input fill_on = false

Clip verse:
    Note/65 Note/67
    Plays: [chorus, bridge, outro][section]

Clip chorus:
    Note/72
    Plays: if fill_on then fill else verse
```

### 8.3 `to`

Inside a wire placement's `/expr`, gate and attribute clause, `to.name` reads attribute `name` of the wire's destination.

- With a fixed destination, `to.name` is that instance's attribute.
- With a destination chosen at runtime, every candidate has `name` with the same type and unit family, and `to.name` makes the same choice among the candidates' attributes, in the gate as in the attribute clause.
- `to` takes exactly one name. `to.a.b`, `to` alone, and `to` written outside a wire placement are errors; a function body is outside every wire placement.

```sym
Clip verse:
    Plays | gain=to.level: [chorus, bridge][section]
```

### 8.4 Cycles

No cycle of wires contains a wire of a kind the vocabulary does not mark cyclic. Every candidate of a destination chosen at runtime counts as an edge in this check.

## 9. Gates

### 9.1 Inline `when`

```ebnf
Gate ::= 'when' Expr
```

`when expr` on a node, a wire or a pattern use gates it: the instance is active only while the expression, a `bool`, is true.

```sym
Clip verse:
    Note/60
    Note/62 when fill_on
    Plays when looping: chorus
```

### 9.2 `when` blocks

```ebnf
WhenBlock ::= 'when' Expr ':' Block ('else' ':' Block)?
            | 'when' ':' NEWLINE INDENT GuardArm+ ('else' ':' Block)? DEDENT
GuardArm  ::= Expr ':' Block
Block     ::= NEWLINE INDENT BodyItem+ DEDENT | InlineChildren NEWLINE
```

A `when` block gates groups of body items. In the first form, the first block is active while the condition holds and the `else` block otherwise; the `else` aligns with its `when`. In the second form, the first arm whose guard holds is active, and `else`, the last arm, is active when none holds. Every arm's placements exist at all times (§10.2); only their activation varies.

```sym
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

### 9.3 Activation

An instance or wire is active when its own gate, the conditions of the `when` block arms around it, and the gates of all its ancestors are open. An arm's condition is that its guard holds and no earlier guard of its block does; an `else` arm's is that no guard of its block holds. A wire's ancestors include its source. Activation is runtime state (§12.3). It is not readable from source, and it stops no value from updating. What an inactive instance means, such as a silent note that keeps its time or a bypassed effect, is the consumer's.

## 10. The vocabulary

### 10.1 What a vocabulary declares

| Entry | Declares |
|---|---|
| Node kind | name; attributes; whether it may stand at top level, and how many; accepted child kinds, and how many of each; how many incoming and outgoing wires of each wire kind it takes; whether it is an entry kind |
| Wire kind | name; allowed source and destination kinds; attributes; whether it may form cycles |
| Attribute | name; type; default; unit family and default unit; flag character; whether it is its kind's default attribute |
| Enum | name; variants |
| Unit family | name; base unit; units and their scales |
| Prelude | SymphonyScript source: functions, patterns and named values such as note names |

A count is exactly *n*, at least *n*, at most *n*, between *n* and *m*, or any number. A child kind absent from a kind's accepted list is not accepted. A wire kind connects only the source and destination kinds it lists; a node kind bounds how many wires of each kind it takes, and takes any number where it states no count. Each kind has at most one default attribute, and its flag characters are distinct.

A vocabulary is invalid, and no program compiles against it, when:

- a name it declares is not an ASCII identifier (§2.4), or is a keyword or reserved word (§2.5), a built-in name (§5.7) or a variant of one of its enums;
- two of its declarations share a name, or a kind declares two attributes of one name;
- a kind has two default attributes, two of its attributes share a flag character, or a flag character is outside the set of §7.4 or flags an attribute that is not `bool`;
- a count's minimum exceeds its maximum;
- a declaration refers to a kind of another vocabulary;
- an enum has no variants or more than 65,536, or an enum attribute's default is not one of its variants;
- a unit's scale is not a positive finite number, a unit name repeats across families, an attribute's default unit is not in its family, or a unit name begins with `x` and a hexadecimal digit, `b` and a binary digit, `o` and an octal digit, or `e` or `E` and a digit or sign (§2.6);
- an integer attribute belongs to a family with a unit whose scale is not an integer;
- the prelude fails to parse or to resolve its names.

The prelude is ordinary source compiled with the program. It holds `let`, `fn` and `pattern` declarations only, and its declarations share the top-level namespace (§6.5). It is parsed and resolved once per vocabulary; its declarations are type-checked where programs use them, and an error found there is a program error located in the prelude. A source declaration with a prelude declaration's name shadows it in the program's source. Prelude bodies see only prelude declarations, the vocabulary's names and the built-ins, so source declarations never change them.

### 10.2 Checks against the vocabulary

The compiler checks the expanded program (§6.4) against the vocabulary:

- Every kind, wire kind, attribute, enum variant and unit exists in the vocabulary.
- Every top-level instance's kind may stand at top level, and the number of each such kind is within its count.
- Every child's kind is accepted by its parent, and each parent's children of each kind are within their counts.
- Every wire's source and every candidate destination are kinds its wire kind connects, and each instance's incoming and outgoing wires of each kind are within its counts.
- A wire whose destination is chosen at runtime counts toward the maximum of every candidate and toward the minimum of none.
- Gated placements and wires count as ungated ones do, because a gate varies activation and never structure (§9.2): `when a: Output` followed by `else: Output` holds two `Output`s, and a gated wire counts toward its destination's minimum.
- Every attribute value has the attribute's type and unit family.
- Wires form cycles only as §8.4 allows.

A top-level instance that no entry-kind instance reaches, through children and through wires to any candidate, draws a warning. A program with no entry-kind instance draws one warning that says so instead, and a vocabulary with no entry kind draws none.

## 11. The Program

Compilation produces a Program. It contains:

- the identity of the vocabulary it was compiled against;
- every instance: its kind, path, attributes and gate, and its sequence of children, bundles and wires in source order;
- every wire: its kind, path, source, destination candidates and choice, attributes and gate;
- every input: its name, type, unit family and initial value, in top-level order;
- every reactive cell that something reads, in dependency order;
- the top-level instances in top-level order (§2.1).

An item's gate combines its own `when` with the conditions of the `when` block arms around it (§9.3). Instances are listed in preorder: each top-level instance in top-level order, each followed by the children and bundle members of its sequence in sequence order, each of those followed in turn by its own. Wires are listed in the order that walk meets them.

Every attribute of every instance and wire is present, with vocabulary defaults filled in. Values are in their family's base unit. Functions and patterns are expanded, and no other source construct remains.

### 11.1 Paths

Every instance and wire has a path, with segments joined by `.`:

- A top-level instance's path is its name.
- A named child or wire extends its parent's path with its name: `verse.hook`.
- An anonymous child or wire extends it with its kind and its position among the parent's anonymous entries of that kind, counted from 0 in sequence order, bundle members included: `verse.Note:2`.
- A named pattern use contributes its name, and an anonymous use contributes the pattern's name and its position among the parent's anonymous uses of that pattern: `verse.Arp:0.Note:2`. The items a use expands to take their paths under its segment and do not count among the parent's entries, so in `Clip verse: Note/40 Arp/48 Note/41` the last note is `verse.Note:1`.
- A use that stands in for a single placement (§6.4) gives that placement the use's path.

## 12. Runtime semantics

### 12.1 Writes

The host writes inputs by handle. A write stages a value until the next commit and has no other effect. Writes accumulate between commits, and the last write to each element wins.

A write takes a value of the input's kind (§4.4): `I32` or `I64` for an integer input, `F32` or `F64` for a float input, `Bool` for a `bool` input, and a variant of its enum for an enum input. The runtime converts it to the input's type, a float rounding and an integer saturating. A write is rejected with an error when its value is of another kind, when its element index is outside the array, when its enum value belongs to another enum or names no variant, or when a whole-array write has the wrong length. A scalar input takes single writes, and an array input takes single-element and whole-array writes; any other form of write is rejected. An accepted write marks the elements it writes as written by the host (§13.2), whether or not their values change. Elements are numbered row by row: element `k` of an `i32[3][2]` input is row `k div 3`, position `k % 3`, and change reports number array attributes the same way (§12.3). An input in a unit family is written in the family's base unit (§3.6). A written NaN is stored as the canonical NaN (§12.5).

### 12.2 Commit

A commit applies staged writes and brings every cell up to date:

1. Each input whose staged value differs bit for bit from its current value changes.
2. Each cell that reads a changed value is evaluated, in dependency order, at most once per commit.
3. A cell whose new value is bit-for-bit equal to its previous value is unchanged, and its readers are not evaluated because of it.
4. Activation (§9.3) and destination choices (§8.2) are recomputed.

After a commit, every value the runtime holds reflects the same set of inputs, and no read from the runtime sees part of a commit. A consumer that copies values elsewhere states what its own readers see.

Every cell is a pure function of the inputs, and no cell carries state from one commit to the next. Time-dependent behaviour comes from inputs the host writes, such as `bar`, `beat` and `time`, combined with pure functions and `rand`.

### 12.3 Changes

A commit reports what changed, in the terms consumers use:

- an instance attribute, with the element index for an array attribute;
- a wire attribute, with the element index for an array attribute;
- an instance's activation;
- a wire's activation;
- a wire's chosen destination.

Each is reported only when its value differs bit for bit from its value before the commit, and a chosen destination only when the chosen instance differs. Instance changes come first and wire changes after them, each in Program order (§11); for one item, attribute changes come in the order the vocabulary declares the attributes, element by element, then its activation, then a wire's chosen destination. Loading a Program evaluates every cell, activation and destination choice, and the first commit reports changes relative to that loaded state.

### 12.4 Evaluation never fails

Evaluation has no runtime errors: every operation yields a value (§5.2, §5.5, §5.7, §5.8). The only runtime errors are rejected host calls: a write §12.1 rejects, or a write through a handle whose input no longer exists (§13.2).

### 12.5 Determinism

The same Program with the same sequence of writes and commits produces bit-identical values on every target. Built-in float functions use one implementation on every target, and a call in `f32` uses that implementation's `f32` function. Every NaN that an input, a cell, an attribute binding or a folded constant holds is the canonical quiet NaN: `0x7FC00000` for `f32` and `0x7FF8000000000000` for `f64`.

`rand(seed, n)` widens both arguments to `i64`, takes their bits as unsigned 64-bit integers, and computes in 64-bit unsigned arithmetic that wraps, where `>>` shifts right with zeros shifted in:

```text
x = (seed << 32) + n
z = x + 0x9E3779B97F4A7C15
z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9
z = (z ^ (z >> 27)) * 0x94D049BB133111EB
z = z ^ (z >> 31)
rand = (z >> 40) / 2^24
```

`rand(0, 0)` is 14819496 / 2²⁴, and `rand(1, 2)` is 11759674 / 2²⁴.

## 13. Reload

A reload replaces a running Program with a newly compiled one.

### 13.1 Acceptance

A reload is all-or-nothing and happens between commits. A Program compiled against a different vocabulary is refused, and the running Program continues. Source that fails to compile produces no Program, so the running Program continues.

### 13.2 Inputs

Each input of the new Program takes its value by the first of these rules that applies; a scalar input is one element:

1. An input with no old input of its name takes its initial value.
2. An input whose kind changed (§4.4), or whose shape changed other than in its outermost length, takes its initial value. An enum input moved to another enum changes kind, and a scalar input that becomes an array changes shape.
3. Otherwise each element both lengths share keeps its old value, converted to the new type as a write is (§12.1), when the host has written it, and takes the new initial value when the host has not; elements beyond the old length take the new initial values.

Whether the host has written an element is tracked per element, and a kept element stays written. A removed input is dropped, and a handle to it rejects writes. A handle stays valid across reloads while an input of its name exists, and becomes valid again when a reload brings one back. Writes staged before the reload stay staged against the new Program by input name and apply at the next commit, which marks their elements written; a staged write that the new input rejects (§12.1), or whose input is gone, is dropped and listed in the report (§13.4).

### 13.3 Matching

Each instance and wire of the new Program is matched with at most one of the old, of the same kind. Matching starts with the top-level instances and descends into each matched pair's sequence. At each level:

1. Named items match by their path below their matched parent. An item whose path matches one of a different kind does not match.
2. The parent's anonymous entries, bundles among them, match by a common subsequence of their keys. An entry's key is its kind, `bundle` for a bundle, with its attribute values, a wire's fixed destination, the names of its named descendants and, for a bundle, its members' keys. An attribute's value in a key is the bits of its constant, or a mark that it is reactive; a string compares by its text and a destination by its path. After the common prefix and suffix are matched, the subsequence is the one Myers' difference algorithm finds, preferring a removal to an addition where both cost the same.
3. In each gap, between two consecutive matches of step 2 or before the first or after the last, the i-th unmatched entry of a kind on the old side pairs with the i-th unmatched entry of that kind on the new side, and they match with changed attributes.
4. A matched bundle's members match within it by steps 1 to 3.

Items under an unmatched parent, named members of an unmatched bundle among them, and every other unmatched item, are removed or added.

### 13.4 Report

A reload reports, for instances, bundles and wires, the matched pairs (old and new), additions, removals, and the parents whose sequence order changed, and it lists the staged writes it dropped (§13.2). The top level counts as a parent, and a sequence's order changed when matched items changed their relative order. It then reports the attribute, activation and destination changes of matched items in the form a commit uses (§12.3); a destination compares by path. Cells are recomputed from the new Program (§12.2).

## 14. Not part of SymphonyScript

SymphonyScript has none of the following:

- **Declared kinds or types.** Node kinds, wire kinds, enums and units come from the vocabulary (§10). There are no records, tuples, maps, or enums with data (§3).
- **Mutation and loops.** Bindings never change. The only loop is the compile-time `for` (§7.9), patterns do not recurse (§6.4), and functions recurse only at compile time (§6.3).
- **State across commits** (§12.2).
- **Varying structure.** Gates vary activation (§9), and destinations vary among fixed candidates (§8.2). A different structure is a different Program, loaded by reload (§13).
- **Runtime strings** (§3.3) and **node references held at runtime** (§3.5).
- **Closures.** No construct takes a function as a value.
- **Modules.** There are no imports, exports or visibility levels; one namespace spans the project (§6.5).
- **Metadata attributes.** No construct attaches metadata to a declaration.
- **Bitwise operators.** `^` is reserved (§2.5).
- **Readable gate state** (§9.3).
- **Aborting evaluation** (§12.4).
