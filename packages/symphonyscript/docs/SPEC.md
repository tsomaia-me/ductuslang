# SymphonyScript Specification

This document defines SymphonyScript: what a source program may contain, what it means, and how the runtime evaluates it. It is normative. `ARCHITECTURE.md` describes how the implementation realizes it.

## 1. Overview

### 1.1 Programs and vocabularies

SymphonyScript describes graphs: nodes placed inside other nodes, wires between nodes, and values that change while the graph runs. Music graphs (sessions, tracks, clips, notes) and audio graphs (oscillators, filters, mixers) are written the same way.

A program is compiled against exactly one vocabulary. The vocabulary declares every node kind, wire kind, enum, unit and prelude declaration a program may use, together with the rules for combining them (§10). Source code declares no kinds; it declares functions, patterns and record types (§3.8). It places instances of the vocabulary's kinds, sets their attributes, connects them with wires, and binds attributes to cells, values that change at runtime, the ones the host writes among them.

Compilation produces a Program (§11): a complete description of instances, wires and cells. A host loads the Program into the runtime, writes the cells that are its to write and commits (§12). Consumers read the Program and the reported changes and interpret them. The meaning of a kind belongs to its consumer, not to the language.

```sym
state cutoff(0.5)
state wobble = cutoff * 0.8

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

Examples are fenced blocks tagged by what they hold: `sym` for a program, `sym-error` for a rejected program, `sym-package` for a package file (§15.2), `ebnf` for productions and `text` for anything else. Every `sym` example parses, and compiles when it declares every name it uses; a name it uses without declaring stands for a declaration elsewhere in the same program. Every `sym-error` example is rejected for the reason the text beside it gives. `docs/pending-examples.txt` lists the examples of constructs the code does not build yet, and the test requires each of them to fail and every other example to hold. `docs/pending-productions.txt` lists the productions of constructs the code does not build yet, and the test requires each of them to have no parse case and every other production to have one. Examples use the example vocabulary (§1.3).

### 1.3 Example vocabulary

| Node kind | Top level | Attributes (the default attribute marked "default") | Accepts |
|---|---|---|---|
| `Session` | yes, at most one; entry kind | `tempo: f64 = 120` (default) | `Track` |
| `Track` | yes | `label: str = ""`, `title: str[..64] = ""` | `Piano`, `Synth` |
| `Clip` | yes | `gain: f32 = 1`, `level: f32 = 1` | `Note` |
| `Note` | no | `pitch: i32 = 60` (default), `velocity: f32 = 1`, `brightness: f32 = 0.5`, `length: f64 = 0.25` in beats, `accent: bool = false` with flag `'` | none |
| `Kit` | yes | none | `Sampler` |
| `Sampler` | no | `file: str` (default; takes a constant) | none |
| `Piano` | no | none | none |
| `Synth` | yes | none | `Osc`, `Filter`, `Mixer`, `Output` |
| `Osc` | yes | `freq: f64 = 440` (default), `wave: Wave = Sine`, `rate: f32 = 1` | none |
| `Filter`, `Mixer`, `Output` | yes | `Filter` has `cutoff: f32 = 1000` (default) | none |
| `Pad` | yes | `hit: Hit` (default; takes `out channel`, capacity 16), `gain: f32 = 1` | none |

| Wire kind | From → to | Attributes (the default attribute marked "default") | Cyclic |
|---|---|---|---|
| `Plays` | `Clip` → `Clip` | `gain: f32 = 1` (default), `transpose: i32 = 0` | yes |
| `Starts` | `Clip` or `Track` → `Clip` | none | yes |
| `Includes` | `Session` → `Track` | none | no |
| `Feeds` | `Osc` or `Filter` → `Filter`, `Mixer` or `Output` | none | no |
| `Modulates` | `Osc` → `Filter` | `depth: f32 = 1` (default) | no |

The enum `Wave` has the variants `Sine`, `Saw` and `Square`. The unit family `beats` has the base unit `beats` and the unit `bars` with scale 4. The unit family `time` has the base unit `s` and the unit `ms` with scale 0.001. Every count not stated above is "any number". The example vocabulary's prelude declares one record type, `Hit`, with the fields `note: i32` and `velocity: f32`, and nothing else.

## 2. Source text

### 2.1 Files

A project is a package (§15.1): a set of `.sym` files compiled together, found in every folder under its root, the folder holding its package file (§15.2). All files of a package share one namespace (§6.5), and no file imports another; a package takes in and gives out names only through its package file (§15.2). Files are ordered by the byte-wise order of their paths, and items within a file by source order. This order is the program's top-level order. A source file whose path equals a prelude file's path (§10.1) is an error.

```ebnf
File    ::= TopItem*
TopItem ::= CellDecl | LetDecl | FnDecl | PatternDecl | TypeDecl | TopPlacement
```

### 2.2 Layout

Indentation is significant. Leading whitespace consists of spaces; a tab in leading whitespace is an error. Between tokens, spaces and tabs are whitespace. A line ends at LF or at CRLF. Columns count Unicode scalar values from the start of the line. Blank lines and lines holding only a comment emit no tokens and do not affect layout.

The lexer keeps a stack of indentation columns, starting with column 0. A line indented deeper than the top of the stack pushes its column and emits `INDENT`. A line indented less pops columns, emitting one `DEDENT` per popped column, and must land exactly on a column still on the stack. Every line holding a token ends with `NEWLINE`, including a last line with no line break after it. At end of input the lexer emits a `DEDENT` for every column on the stack above 0.

Inside `( )`, `[ ]` and the braces of a record, map or document literal (§3.8, §3.10, §3.11), layout is suspended: newlines are whitespace and no layout tokens are emitted.

An attribute clause (§7.6) continues onto each following line whose first token sits exactly at the column of the clause's first entry, while that column is deeper than the top of the stack and no `:` has closed the clause; `::` closes nothing. The line break before such a line emits no `NEWLINE`, and the line emits no `INDENT` or `DEDENT`. The lexer tracks each clause's first-entry column for this rule, and a line that misses the column only by the display width of wide characters is an error whose diagnostic shows the column.

A semicolon outside strings and comments is an error.

### 2.3 Comments

`//` starts a comment that runs to the end of the line. There is no other comment form. Inside a string and outside its interpolations, `//` is text.

### 2.4 Identifiers

An identifier starts with a character of the Unicode property XID_Start, `_` or `#`, continues with characters of XID_Continue and `#`, and holds at least one character other than `_` and `#`. `#` behaves as a letter, so note names such as `C#4` and `F#3` are identifiers. Identifiers are compared, and stored in paths, in Unicode NFC form, so a name reads the same in every normalization form. A vocabulary's names are ASCII identifiers (§10.1).

By convention, kinds, patterns and enum variants are written in PascalCase, and everything else in snake_case. The compiler does not enforce the convention.

### 2.5 Keywords and reserved tokens

Keywords are never names:

```text
let state stream channel fn pattern type default const cell out children
if then else match when for in into from as on without
is not and or div to true false none
u8 i32 i64 f32 f64 bool str json node wire
require use publish
```

A keyword stands as a member name where nothing but a name can stand: after `|` in an attribute clause (§7.6), after `.` in a read (§5.6), as a key in a record literal and as a field in a `type` body (§3.8), so a vocabulary may name an attribute `channel` or `out` (§10.1). Everywhere else it is a keyword. The words `recurrent` and `string` are reserved and are never names; `null` is a word of JSON text inside a document literal (§3.11) and is never a name. The tokens `==` and `!=` are reserved and rejected by the lexer, and their diagnostic points to `is` and `is not`. `^` stands only at the start of an index or a slice bound (§5.5); anywhere else it is an error.

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
TypeSuffix  ::= 'u8' | 'i32' | 'i64' | 'f32' | 'f64'
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
| `u8` | 8-bit unsigned integers, 0 to 255 |
| `i32` | 32-bit signed integers |
| `i64` | 64-bit signed integers |
| `f32` | 32-bit IEEE 754 floats |
| `f64` | 64-bit IEEE 754 floats |
| `bool` | `true`, `false` |

### 3.2 Arrays

An array has one element type and a length. A fixed array's length is fixed at compile time: `[60, 64, 67]` has length 3, and its element type follows §4. Elements may themselves be arrays, of one fixed length or each with its own live length (§3.7): `[[60, 64, 67], [62, 65, 69]]` holds two rows of three, and with `i32` elements its type is written `i32[3][2]`. A variable array's length changes at runtime, up to a capacity fixed at compile time (§3.7). An array's length and capacity are below 2³¹.

`[]` is the empty array, of length 0; each use gives it its element type, as a flexible value's use does (§4.1). An array's members, `size`, `capacity`, `bytes` and `map(f)`, are §5.7's.

A fixed array's contents may change at runtime when its elements read cells (§5.5).

```ebnf
Type       ::= FnType | Base ('|' Base)*
FnType     ::= '(' (FnParam (',' FnParam)*)? ')' '->' Type
FnParam    ::= (Name ':')? Type
Base       ::= (ScalarType | StrType | MapType | JsonType | (Name | QualifiedName) TypeArgs? | 'none' | '(' Type ')') Dims? '?'?
Dims       ::= ('[' Size ']' | '[' '..' Size ']')+
Size       ::= Expr
ScalarType ::= 'u8' | 'i32' | 'i64' | 'f32' | 'f64' | 'bool'
StrType    ::= 'str' ('[' '..' Size ']')?
```

A `Name` in a type is a vocabulary enum, a record type or an alias (§3.8), a node kind or a wire kind (§3.5) or a type parameter (§6.8); `TypeArgs` are §6.8's, a function type `FnType` is §6.7's, `none`, `|` and `?` are §3.9's, and `( )` groups a sum under dimensions: `(i32?)[..4]` holds optional elements, and `i32[..4]?` is an optional array. A capacity written directly after `str` is the string's (§3.3): `str[..32][4]` holds four strings of up to 32 characters each. A `Size`, a length or a capacity, is any compile-time integer expression (§4.6), value parameters among what it reads (§6.8): `T[..N + 1]`, `T[N * 2]`. A size lies between 1 and 2³¹ − 1, and one outside that range is an error; where the size reads a value parameter, the error is at the call or use that binds it, naming it, with a secondary span on the type. A float size is an error whose help names `div`. A capacity may stand at any level: `i32[3][..10]` holds up to 10 rows of 3, and `i32[..4][..10]` up to 10 rows of up to 4 elements, each row with its own live length.

### 3.3 Strings

A string is a sequence of characters, each one Unicode scalar value. It is written as a literal (§2.6), built with `+`, slices, indexing and interpolation (§5.5, §5.9), compared with `is` (§5.3), matched (§5.4) and bound to string attributes. `s.size` is its number of characters and `s.bytes` its UTF-8 bytes (§5.7).

A string has a capacity, the most characters it holds, fixed at compile time. A string built only from compile-time values is a compile-time string, whose capacity is its length; a string that reads a cell is a runtime string, one cell (§4.4), whose capacity this table gives:

| Value | Capacity |
|---|---|
| a literal | its fixed text, plus for each interpolation the most characters its value takes: 3 for a `u8`, 11 for an `i32`, 20 for an `i64`, 5 for a `bool`, the longest variant name of an enum, a string's capacity |
| `a + b` | the sum of the capacities of `a` and `b` |
| `s[i]` | 1 |
| a slice `s[a..b]` | the most characters it holds for any length of `s` up to its capacity and any value of its bounds that read cells, as a variable array's slice (§3.7) |
| `if` and `match` | the largest capacity among the arms |
| a derived cell | §6.6 |
| a host-fed cell, an annotation, a parameter, a record field or an attribute written `str[..N]` | N |

A string that comes from outside the program states its capacity: a host-fed cell (§6.1), a record field (§3.8) and a native function's parameter or result (§10.1) are written `str[..N]`, and `str` without a capacity there is an error whose help names `str[..N]`; a vocabulary attribute states a capacity or takes compile-time strings only (§10.1). Elsewhere `str` without a capacity takes the value's: `let a: str = "hello"` holds 5. A string's memory is 4 bytes per character of its capacity (§12.6).

A string binds to a string attribute whose capacity is at least its own; a compile-time string binds to an attribute that states no capacity, and a runtime string there is an error. A string where a `u8` array is expected is its `bytes` (§5.7): a native function's `u8[..]` parameter given `"GET"` receives `[71, 69, 84]`. A string converts to bytes nowhere else, and nothing makes a string from bytes.

```sym
state name(""): str[..32]

Track lead | title="lead: {name}":
    Piano

Kit drums:
    for i in 0..8:
        Sampler | file="kit/{i}.wav"
```

`title` holds up to 64 characters and takes the runtime string of 38; `file` states no capacity, so it takes compile-time strings only, and `i` is a compile-time value.

### 3.4 Enums

The vocabulary declares enums. Every variant is a plain name with no data. Where the expected type is an enum (§4.2), a variant is written bare: `wave=Saw`. Anywhere else it is qualified with the enum's name: `Wave::Saw`.

### 3.5 Node references

A reference names an instance of a node kind or of a wire kind. A Program's instances are fixed at compile time (§14), so every reference is a compile-time value. A reference stands wherever a compile-time `let` stands: as the value of a compile-time `let` (§6.2), alone or in an array; as a function's or a pattern's argument, bound to its parameter (§6.3, §6.4); at the start of an attribute path (§5.6); and as an element of a `for` iterable (§7.9); a node reference also stands as a wire destination (§8.2). A node kind or a wire kind in a type (§3.2) is a reference to an instance of that kind, and `Osc[3]` an array of three; `Osc[..3]` is an error, since a variable array is a runtime value (§3.7). References are never stored in a cell, a record or an attribute.

### 3.6 Units

The vocabulary declares unit families. Each family has a base unit and further units with a positive scale relative to the base: in the family `time`, `s` is the base and `ms` has scale 0.001.

- A literal with a unit suffix converts to its family's base unit at compile time: `200ms` is 0.2 in `time`. Its value is the literal times the unit's scale, computed exactly and rounded once to the type of its use. It is a flexible value (§4.1): an integer when the literal is an integer and the scale is an integer, as `2bars` is 8, and a float otherwise, as `1000ms` and `1.0s` are.
- An attribute may belong to a family and then has a default unit. A plain number bound to it is in that default unit.
- A unit value bound to an attribute of a different family, or of no family, is an error.
- `+`, `-`, `%`, `??`, comparisons, `min`, `max`, `clamp`, the first two arguments of `lerp`, array literals, and the arms of `if` and `match` take operands of one family. A plain value among them, a number or a plain cell, takes that family and is read in the default unit of the attribute its family value reaches, as a literal with that unit's suffix would be: `length=2bars + 1` is 9 beats, and where the default unit is `ms`, `1s + 3` is the float 1.003 s. An array literal's family value reaches every attribute that a read of the literal reaches, whole or at any index, so in `let a = [1bars, 2]` read as `length=(a[0])` the `2` is 2 beats. A fixed-type integer read in a unit whose scale is not an integer is the float it widens into: the expected type (§4.2), or with none `f32` for an `i32` and `f64` for an `i64` (§4.3). Where the family value reaches no attribute of the family, as when `/` or `div` makes it a plain number or a comparison a `bool`, the plain value is an error, and so is a family value that reaches two attributes of the family with different default units.
- `/` and `div` of two values in the same family yield a plain number. Multiplying or dividing a unit value by a plain number keeps the family. Dividing a plain number by a unit value, multiplying two unit values, `**` on a unit value, and a unit value as a range bound, an index, a slice bound or a `^` offset (§5.5) are errors.
- Unary minus, conversions, `abs`, `floor`, `ceil`, `round`, `min`, `max`, `clamp` and `lerp` keep the family of their operands, `lerp`'s third argument a plain number; the other `math` functions take plain numbers.
- A value's family comes from its own init or expression, never from its uses: `state d(5ms)` and `state e = d * 2` are in `time`, and `state p(5.0)` is plain. A value in a family holds base units, and the host writes such cells in base units (§12.1). A plain cell bound to an attribute of a family is read in that attribute's default unit, as a plain number is; a plain cell read at two attributes of one family with different default units is an error naming both.

Families never convert into each other. A family whose relation to another depends on runtime values, such as beats and seconds under a changing tempo, stays separate, and the consumer converts.

```sym
Clip intro:
    Note | length=2bars
    Note | length=0.5
```

The first note lasts 8 beats; the second lasts 0.5 beats, the attribute's default unit.

### 3.7 Variable arrays

A variable array holds from 0 up to its capacity of elements, and its live length changes at runtime. Its type writes `..` before the capacity: `i32[..10]` holds up to ten `i32`s. A value is a variable array when its length depends on a cell: a host-fed cell annotated with such a type (§6.1), a join or slice whose length reads a cell (§5.5), an `if` or `match` that chooses at runtime among arms of different lengths, a derived cell that holds one (§6.6), and any value built from these. A compile-time array has a known length and is a fixed array.

| Value | Capacity |
|---|---|
| a fixed array | its length |
| a host-fed cell or an annotation `T[..N]` | N |
| `a + b` | the sum of the capacities of `a` and `b` |
| a slice `xs[a..b]` | the most elements it holds for any live length of `xs` up to its capacity and any value of its bounds that read cells between their minimum and maximum read from their forms (§7.9), a bound with none by its form taking any value: `xs[^8..]` holds at most 8, `xs[1..]` one fewer than `xs`, and `xs[..min(n, 4)]` at most 4 |
| `if` and `match` | the largest capacity among the arms |
| a derived cell | §6.6 |

A variable array is one cell (§4.4), held whole rather than element by element, from its own level down: `i32[..4][3]` is three cells, each a row with its own live length, and `i32[..4][..10]` is one cell. `xs[i].size` on such a row is an `i32` (§5.7), and the capacity of a value at an inner level follows the table at that level. It binds to a variable-array attribute (§10.1) of its element type whose capacity is at least its own, and a fixed array binds to one when its length is at most the capacity; a variable array where a fixed array is expected is an error. An index past its live length reads its last live element (§5.5), and an index into an empty array yields the element type's zero (§3.8).

### 3.8 Declared types: records and aliases

```ebnf
TypeDecl  ::= 'type' Name GenericParams? (':' NEWLINE INDENT FieldDecl+ DEDENT | '=' Type NEWLINE)
FieldDecl ::= Name ':' Type NEWLINE
RecordLit ::= (Name | QualifiedName) TypeArgs? '{' Fields '}'
Fields    ::= (Name ':' Expr (',' Name ':' Expr)* ','?)?
```

A `type` declaration names a type at top level, in the program's source or in the vocabulary's prelude (§10.1), and shares the top-level namespace (§6.5). The block form declares a record; `type Name = Type` declares an alias, a name that is its type everywhere it is written, a sum (§3.9) included: `type Shape = Circle | Square`, `type Id = i64`. An alias of itself, directly or through other aliases, is an error. A `type` declaration may take bracket parameters (§6.8).

A record is a value with named fields, each of its own type. A field is a scalar, a string with a capacity (§3.3), an enum, a record, a sum of these (§3.9), a map (§3.10), a document with a budget (§3.11) or an array of these; a field of a kind type is an error (§3.5), two fields of one name are an error, and a record type that reaches itself through its fields is an error, since its size would be unbounded. A field's name is scoped to its type and clashes with no other name; a keyword may be one (§2.5), and `past`, `stepped` and `dropped` may not, because a record can be held in a cell whose members they are (§5.7).

A record literal names its type and lists fields by name, in any order: `Chord {root: 60, spread: 4}`. A field it leaves out holds its zero, so `Chord {}` is the record with every field at its zero. A type's zero is `0`, `0.0`, `false`, the enum's first variant, the empty string, an array of its elements' zeros for a fixed array and the empty array for a variable one, a record of zeros, `none` for a sum that has it and otherwise the zero of its first alternative, the empty map, and `null` for a document. The literal carries its type, so it stands wherever a value of that type stands; a literal naming a generic type writes the type's arguments or binds them from its fields (§6.8). `r.name` reads a field (§5.7). Records are not compared, interpolated or converted; `is` on records is an error.

A record whose fields read cells is a value of bindings, field by field, as a fixed array is (§4.4, §5.5): each leaf field is its own binding, and a variable-array field is one cell. A host-fed record cell is written per leaf field or whole (§12.1).

```sym
type Chord:
    root: i32
    spread: i32

state chord(Chord {root: 60, spread: 4})

Clip c:
    Note/(chord.root) Note/(chord.root + chord.spread)
```

### 3.9 Sums and `none`

A sum type `A | B` holds a value of one of its alternatives, which are distinct types among scalars, strings, enums, records, aliases, arrays, maps, documents and `none`; `A | A` is an error. `none` is a type with one value, written `none`, and `T?` is `T | none`; `T??` is an error. A sum is stored as a tag and the slots of its largest alternative.

- A value of an alternative widens into the sum (§4.3), and a sum into a sum that holds every alternative of it; nothing narrows implicitly, so a `T?` where a `T` is expected is an error whose help names `??`.
- `match` reads a sum (§5.4). `x is none`, `x is not none` and `x is Circle`, with an alternative's type name, are `bool` tests (§5.3), and `x ?? y` yields `x`'s value, or `y` when `x` is `none` (§5.3).
- A record literal names its alternative: `Square {side: 1}` where a `Shape` is expected is a `Square`.
- A sum stands in every value type position: a field, a cell, a `let`, a parameter, a result, an array element. It is never an attribute's type, and never holds a node reference.
- Two sums compare with `is` when every alternative is a scalar, a string or an enum: equal when both are `none`, or both hold the same alternative with equal values. A sum with a record alternative is not compared.

```sym
type Circle:
    r: f32
type Square:
    side: f32
type Shape = Circle | Square

state port(none): i32?
state shape(Square {side: 1}): Shape

state bound = port ?? 80
state area = match shape:
    Circle: pi * shape.r * shape.r
    Square: shape.side * shape.side
```

`Square {side: 1}` is a `Square`; in the `Circle` arm `shape` is a `Circle`.

```sym-error
state port(none): i32?

Synth s:
    Osc | freq=port
```

This is an error: `port` may be `none`, and `freq` holds one `f64`; `port ?? 440` reads it with a fallback.

### 3.10 Maps

```ebnf
MapType ::= '{' Type ':' Type '}' '[' '..' Size ']'
MapLit  ::= '{' (Entry (',' Entry)* ','?)? '}'
Entry   ::= Expr ':' Expr
```

A map holds up to its capacity of entries, each a key of the key type and a value of the value type, in the order the entries were added. A key type is a scalar, a string with a capacity or an enum; a value type is any value type but a node reference. The type `{K: V}[..N]` holds up to N entries. A map literal whose keys are compile-time values and whose entries read cells is a value of bindings, entry by entry, as a fixed array is (§4.4, §5.5), `m["k"]` with a compile-time key being that entry's binding, and any other map value is one cell (§4.4), held whole.

- A map literal is written in braces and lists entries `key: value`, each key an expression read in the key type (§4.2), a string literal, a number or a bare variant among them; two keys that are equal compile-time values are an error, and `{}` is the empty map. A literal takes the map type its position expects; where none is expected it takes its key and value types from its entries, as an array literal takes its element type, with its entry count as its capacity, and `{}` there is an error whose help says to name the type.
- `m[k]` yields the entry's value as a `V?` (§3.9), `none` when the key is absent. `m + n` joins two maps, `n`'s entries winning on a shared key and its new keys following `m`'s, with the capacity the sum of the two; `m without k` and `m without ks`, `ks` an array of keys, yield `m` less those entries, with `m`'s capacity; `m[a..b]` slices a map by its entry order as an array is sliced (§5.5), so `(m + n)[^64..]` keeps the newest 64 entries. `m.size`, `m.capacity`, `m.keys` and `m.values` are its members (§5.7).
- A map compares with nothing and interpolates nowhere; a key compares as `is` compares its type. A map built only from compile-time values is a compile-time value, and a placement `for` iterates it (§7.9).
- A host-fed map cell is written whole or one entry by key (§12.1), and a derived cell may hold a map (§6.6).

```sym
state headers({}): {str[..32]: str[..256]}[..32]
state req(Request {})

state host = headers["host"] ?? "localhost"
state trimmed = headers without ["cookie", "set-cookie"]
state with_host = headers + {"host": host}
state hits({}): {str[..64]: i32}[..64] = (hits.past(1) + {req.path: (hits.past(1)[req.path] ?? 0) + 1})[^64..]
```

`hits` counts requests per path and keeps the newest 64 paths.

### 3.11 Documents

```ebnf
JsonType   ::= 'json' ('[' '..' Size ']')?
JsonLit    ::= 'json' JsonValue
JsonValue  ::= JsonObject | JsonArray | JsonString | '-'? JsonNumber | 'true' | 'false' | 'null'
             | (Name | QualifiedName) ('.' Name)* | 'to' ('.' Name)+ | '(' Expr ')'
JsonObject ::= '{' (JsonString ':' JsonValue (',' JsonString ':' JsonValue)*)? '}'
JsonArray  ::= '[' (JsonValue (',' JsonValue)*)? ']'
JsonString ::= '"' (JsonChar | JsonEscape)* '"'
JsonChar   ::= any character except '"', '\' and a control character
JsonEscape ::= '\"' | '\\' | '\/' | '\b' | '\f' | '\n' | '\r' | '\t' | '\u' HexDigit HexDigit HexDigit HexDigit
JsonNumber ::= ('0' | '1' … '9' Digit*) ('.' Digit+)? (('e' | 'E') ('+' | '-')? Digit+)?
```

A document is a JSON value: an object, an array, a string, a number, `true`, `false` or `null`, with the values under it. `json[..N]` holds a document whose text is at most N bytes, and reserves N bytes of text and `(N + 1) div 2` nodes. A document's parts are untyped, so a document holds a shape the program does not declare.

- A document literal is the keyword `json` followed by JSON text, `json {"a": 1}`, `json [1, "b"]`, `json "text"`, `json 12.5`, `json true`, `json null`, and is written no other way. Inside it, strings, numbers, keys and the words `true`, `false` and `null` are JSON's, so a document is pasted unchanged: a string has JSON's escapes, `\uXXXX` naming a character and a surrogate pair one character, and no interpolation; a number has no prefix, separator or suffix; `null` stands nowhere else in a program (§2.5). A name, a path or a qualified variant stands for its value, and `( )` holds any expression, whose strings are the language's: `json {"count": count, "msg": ("hi {name}"), "echo": items}`; a value there widens into the document. Layout is suspended inside its braces and brackets (§2.2). A literal that reads cells is rebuilt at the commits that change them (§12.2); its budget is the sum of its parts' widest text, as a string's capacity counts its interpolations (§3.3).
- A value where a document is expected (§4.2) widens into one (§4.3): a scalar or a string as itself, an enum variant as its name, `none` as `null`, a record as an object by its fields, an array as an array, a map as an object, a sum as its value, and a view spliced in. Nothing narrows a document implicitly. A document is never edited: a program reads parts out through typed reads and builds new documents from values.
- A cell or a record field of document type states its budget. Every other document value derives its budget: a view from its document, a parameter written `json` from its argument, a literal from its parts; `let a: json = json {}` says the same twice.
- A view is a `json` value naming a part of a document: `v["key"]` an object's entry, `v[i]` an array's element, with `i` an integer of either width, `v.keys` and `v.values` an object's keys and values as array views. An absent key, an index into a value that is not an array or past the array's end, and a `null` node are the `Null` value, so a chain of reads never fails. `v.size` is an array's length or an object's entry count, an `i32`, and 0 for anything else. A document interpolates nowhere and is not ordered.
- `v.kind` is the node's kind, a value of the built-in enum `Json`: `Object`, `Array`, `String`, `Number`, `Bool` or `Null` (§5.7).
- `is` and `is not` compare two documents (§5.3), a language value on either side widening into a document first. Two documents are equal when they are the same JSON value: the same kind, numbers numerically (two integer nodes exactly, any other pair as `f64`), strings character by character, arrays element by element, objects by their keys in any order, each with equal values. So `v is none` is true when `v` is the `Null` value, `v is "bob"` when `v` is that string, `v is [1, 2]` when `v` is that array, and `v is json {"a": 1}` when `v` is an object holding exactly that entry.
- A typed read: a `json` value where a type `T` other than a document is expected (§4.2) reads as `T?`, the value when the node fits `T` and `none` otherwise. A number fits an integer type when it is a whole number in the type's range, and a float type when it rounds to a finite value in it (§4.1); a string fits `str[..K]` when it has at most K characters, and an enum when its text names a variant; `true` and `false` fit `bool`; an array fits `T[..M]` when it has at most M elements and each fits `T`, and `T[n]` when it has exactly n; an object fits a record when every field it names fits the field's type, an absent field taking its zero and a key the record lacks being ignored, and fits a map when its entries fit the map's key and value types within its capacity; a value fits a sum when it fits an alternative, the first in the sum's order; a document fits `json[..M]` when its text is at most M bytes, and that read copies it. A read never cuts a value. `??` supplies the fallback (§5.3).
- The host writes a document cell as text (§12.1): a write past the budget, or one that is not valid JSON in valid UTF-8, is refused with the capacity or the byte position in the error, and the cell keeps its value. An object's repeated key keeps its last value in the first key's position; a number is an integer when its text has no fraction or exponent and fits `i64`, otherwise an `f64`, past its range an infinity, which fits no float type.

```sym
state body(json {}): json[..65536]

state items = body["items"]
state count = items.size
state qty = items[0]["qty"] ?? 1
state name: str[..64] = body["customer"]["name"] ?? "anonymous"
state order: Order = body["order"] ?? Order {}
state is_list = items.kind is Array
state anonymous = body["customer"]["name"] is none
state reply = json {"ok": true, "count": count, "echo": items, "order": order}
```

`items` is a view, two integers; `reply` is rebuilt at each commit that changes `body`.

```sym-error
state body(json {}): json[..4096]

state small: json[..256] = body["items"]
```

This is an error: `body["items"]` may not fit `json[..256]`; `?? json {}` supplies a fallback, and `json[..256]?` keeps the `none`.

## 4. Type inference

Types come from the places values are used; source states a type only to fix one outright (§4.5).

### 4.1 Fixed and flexible values

A value has a fixed type when something fixes it: an attribute, a suffix, an annotation, `rand`, or an operand of fixed type in the same expression. A cell fixes its own type only where its use expects none (§4.4). Every other compile-time value is flexible: an unsuffixed literal, `pi`, and any compile-time `let`, operation or `math` call built only from flexible values. Each use of a flexible value chooses its type independently, and the value must fit the chosen type.

```sym
let base = 48

Clip melody:
    Note/(base + 7)

Synth lead:
    Osc | freq=(base * 2.5)
```

`base` is `i32` in the pitch and `f64` in `freq`.

The fit check applies to a flexible value where it takes a type: at each use, where a fixed-type operand gives a literal its type, and wherever a literal or an integer result is converted to a float type. An integer fits a float type when its value is exactly representable, and a float fits a float type when it rounds to a finite value. Other results inside one flexible expression are not checked: integers compute exactly (§4.6), and floats follow §5.2 in the chosen type, so `Note/(2147483647 * 2 div 2)` fits and `Osc | freq=(1.0 / (1e300 * 1e300))` is 0.0. A minus applied directly to a literal forms one value for this check, so `-2147483648` fits `i32`.

```sym-error
Clip melody:
    Note/300000000000
```

This is an error: the value does not fit the pitch's `i32`.

A position that fixes no type, such as a comparison between flexible values, an interpolation or the argument of an integer conversion, gives a flexible value the defaults of §4.5. The argument of a float conversion takes the conversion's type. The operands of a comparison first give each other their types, so in `cutoff > 0.5` the literal takes `cutoff`'s type.

When one flexible value has an integer use and a float use, the float use's value, before §3.6 reads it in a unit, equals the integer use's value converted to the float type; otherwise it is an error naming both uses. `let q = 2 ** -1` is 0 as a pitch and 0.5 as a frequency, so using it as both is an error.

### 4.2 Expected types

The type a position requires flows into the expression filling it: through operators, `if` and `match` arms, array literals and function arguments, down to the flexible values inside. It stops at the operands of comparisons, at an index, and at the operands of `div` when it is a float type; those take their types from each other or from the defaults of §4.5, and the result then widens into the position. An integer type flows through `div`, so with `state n(4)`, `Note/(n div 2)` reads `n` as an `i32`. A fixed-type operand meeting an expected type widens into it (§4.3) before the operation, so `Note/(u8(255) + 4)` computes in `i32` and is 259; a cell is read in it as §4.4 says, and any other fixed-type operand is an error. An annotation (§4.5) is the type its init's position requires, so `state w(Saw): Wave` takes `Saw` in `Wave`. An expected type that is a sum (§3.9) gives an expression the alternative it matches as its expected type, a bare variant `Wave` from `Wave?`, a flexible literal `i64` from `i64?` and a string literal `str[..32]` from `str[..32]?`, the first alternative in the sum's order where several match, as §3.11's typed read chooses, and the value widens into the sum (§4.3).

```sym
state cutoff(0.5)

Clip melody:
    Note | brightness=cutoff * 0.8
```

With `brightness` of type `f32`, `0.8` is an `f32`, and `cutoff` is read as one.

The operands of `is` and `is not` give each other their expected type, so in `wave is Saw` the variant resolves in `wave`'s enum. A `let` or a derived cell whose value needs an expected type, such as a bare variant, takes it from those of its uses that expect a type, when they all expect one, and a use that expects no type reads it in that type; a bare variant is no flexible value (§4.1). A parameter's annotation is the type its argument is read in, and a result annotation the body's expected type (§6.3, §6.4).

### 4.3 Widening

These implicit widenings exist, and no others: `u8` to `i32`, `i64`, `f32` and `f64`, `i32` to `i64`, `i32` to `f32`, `i32` to `f64`, `i64` to `f64`, `f32` to `f64`, a value into a sum that holds its type, and a sum into a sum that holds every alternative of it (§3.9). A value never narrows implicitly, and a float never becomes an integer implicitly; conversions are explicit (§5.8). Widening `i32` to `f32` and `i64` to `f64` rounds integers beyond the float's exact range: an `i32` cell holding 16777217 reaches an `f32` attribute as 16777216.

When an operator's operands have different fixed types, both widen to the smallest type both widen into. With no such type, the expression is an error. A flexible float meeting an operand of fixed integer type makes the operation a float one, `f64` for an `i64` operand and `f32` for an `i32` one, and the integer widens into it: with `state n(4)`, `n * 0.5 > 1` computes in `f64`. A `u8` operand counts as an `i32` one there, `u8` widening exactly into `f32`. `math` functions widen their operands as operators do.

### 4.4 Cell types

A cell holds one value at runtime, so it has exactly one type.

A host-fed cell (§6.1) is typed from its init alone, never from its uses; a string cell states its capacity (§3.3) and a record init names its type (§3.8). Its kind is the init's (§4.5): integer, float, `bool` or one particular enum, with the init's array shape. An array cell has one kind for all its elements, a float one when any element is a float one (§3.2). An integer cell is an `i64` and a float cell an `f64`, unless an annotation gives the type (§4.5), and the init computes in that type. Each use reads the cell in the type the use expects: an integer cell saturates into a narrower integer type and converts into a float type, rounding beyond the float's exact range, and a float cell rounds into `f32`. Where the use expects no type, the cell is read in its own type. A float cell used where an integer is expected is an error; a conversion (§5.8) states the rounding. The init must fit every use.

```sym
state cutoff(0.5)

Clip melody:
    Note | brightness=cutoff

Synth lead:
    Osc | freq=cutoff * 4000
```

`cutoff` is an `f64` cell. It reaches `brightness` rounded to `f32`, and `freq` at full precision.

An integer host-fed cell whose every use expects a float draws a warning, because its host cannot write a fraction; `1.0` in place of `1` makes it a float cell.

A derived cell (§6.6) is typed as a host-fed cell is, except for its width. Its kind comes from its rule (§4.5); its width is the widest its uses of that kind expect; and each use reads it in the use's type, as each use reads a host-fed cell. A float derived cell used where an integer is expected is an error. Its uses include the operands it feeds in other derived cells' rules, so expected types flow from attributes back through derived cells to the host-fed cells they read. A derived cell whose uses fix no width takes the default of §4.5, and defaults apply to a reader before the cells it reads:

```sym
state a(1)
state b = a + 0.5

Clip c:
    Note when b > 1
```

`b` is a float cell with the default width `f32`, and its rule reads `a`, an `i64` cell, as an `f32`.

A derived cell whose rule is a fixed array literal whose elements read cells is an array of bindings (§5.5), not one cell, and one whose rule is a record literal a record of bindings (§3.8); a variable array (§3.7), a runtime string (§3.3), a derived cell that holds one (§6.6) and a `for` expression that reads a cell (§5.10) are each one cell. An element read with an index known at compile time, such as a literal, a compile-time `let` or a `for` variable, is its own binding with its own type; an element read through a runtime index is read in that read's type.

The compiler resolves cell types once over the whole program. This always terminates, because present reads form no cycles (§6.2), and cells whose history reads reach one another settle their widths together as §6.6 settles their kinds.

### 4.5 Defaults and annotations

A cell's kind is its init's or its rule's: `bool`, an enum, integer or float, element by element for an array. A numeric init or rule is a float one when its result is a float under §5: when it holds a float literal, `pi`, `/`, a float `math` function, `rand`, a float conversion or a float unit value, or reads a float cell or a float `let`, or calls a recursive function whose body is a float one for the call's argument kinds. Every other numeric init or rule is an integer one.

A derived cell whose uses fix no width takes `f32` when it is a float, and the widest integer type among its operands when it is an integer, an operand that is a derived cell counting with the widest integer type among its own operands, found the same way, so a derived cell reading an `i64` cell, directly or through other derived cells, stays `i64`. A flexible value in a position that fixes no type (§4.1) takes `i32`, or `i64` when its value does not fit `i32`, and a flexible float takes `f32`, or `f64` when its value lies outside `f32`'s finite range.

An annotation fixes a type outright: `state fine(0): f32` is a float cell stored as `f32`, and its uses read it as §4.4 says. A suffix does the same for a literal: `0f64`. An annotation on a function or pattern parameter, a local `let`, a `for` variable, a `for` expression's accumulator, a lambda's parameter or a result fixes its type the same way (§5.10, §6.3, §6.4, §6.7, §7.9), and a node kind or a wire kind there names a reference (§3.5). Annotations are per binder: in a function, a pattern or a lambda, one parameter may be annotated and another inferred, as in `fn shift(x, by: i32): x + by`.

### 4.6 Compile-time evaluation

A compile-time value is computed by the rules of §5, except that a flexible integer computes exactly and a fixed-type integer operation that saturates is an error; a conversion saturates as §5.8 says and is no error. A flexible integer whose magnitude reaches 2¹²⁷, positive or negative, is an error. A float type that flows into a flexible value (§4.2) makes it a float of that type from its literals up, so `2 ** -1` is 0.5 where a frequency expects it (§4.1) and `16777216 + 1` is 16777216 where an `f32` does; a flexible float computes in the type its use chooses.

- The bounds of a range in a `for`, and an index or a slice bound known at compile time, are integers that fit `i64`. A range's loop variable is a flexible integer, so each use chooses its type.
- A call to a recursive function (§6.3) computes flexible integer arguments exactly and every other argument in the type its call gives it: a fixed type, or for a flexible float its expected type or the default of §4.5. Its whole body computes in those types, and an operation they do not allow, such as `div` on a float, is an error naming the call. Its result may be an array, with the shape of the value the call yields in the type its use expects; a value that holds the call and has different shapes at two of its uses is an error naming both, as in §4.1. A cell whose shape comes from such a result alone takes the shape the call yields with the defaults of §4.5, and a use that yields another shape is an error naming both.
- A value that structure depends on, such as a range bound or a recursive call's result, is computed before the expansion that depends on it.
- A compilation evaluates at most 1,000,000 distinct recursive calls and inlines at most 100,000 calls of other functions, a call counting each time the body holding it is inlined or expanded, and once in a recursive function's body or its parameters' defaults, which are evaluated rather than inlined, and its expansion runs at most 1,000,000 `for` iterations and pattern uses, the iterations of a compile-time `for` expression (§5.10) counting among them, and yields at most 1,000,000 instances and wires. A recursive call repeats an earlier one, counts once and reuses its value, only when its argument values, argument types and families, expected type and the default unit its plain values are read in (§3.6) are all equal, a function reference passed as an argument (§6.7), directly or through a `let`, counting by its declaration, a lambda's being its source position, and the values of the names its body reads. Exceeding a limit is an error naming the call or the expansion. Each number here, §6.3's nesting depth and §12.6's memory limit are default limits that a package's `options` (§15.2) or a single compilation may set otherwise (ARCHITECTURE §7, §9).
- Every limit that draws an error has a warning value beside it, whose default is half the limit's default and which a package's `options` or a single compilation sets as it sets the limit. A count that passes the warning value within the limit draws a warning naming what the limit's error names: the call, the expansion, or for the memory limit the Program; a count that passes the limit draws the error, and one that passes both at once draws the error alone. The reload search bound (§13.3) draws no error and has no warning.

In §6.4's `Arp`, `steps` counts the iterations exactly, and `i` is an `i32` in each pitch. `Note/(2147483647 + 1)` is an error, as `Note/2147483648` is.

## 5. Expressions

### 5.1 Precedence

From loosest to tightest:

| Level | Operators | Associativity |
|---|---|---|
| 1 | `if … then … else …`, `for … in … into … from …: …` | right |
| 2 | `or` | left |
| 3 | `and` | left |
| 4 | `not` | prefix |
| 5 | `is`, `is not`, `<`, `<=`, `>`, `>=` | none |
| 6 | `??` | right |
| 7 | `+`, `-`, `without` | left |
| 8 | `*`, `/`, `div`, `%` | left |
| 9 | unary `-` | prefix |
| 10 | `**` | right |
| 11 | `.name`, `[index]`, `(arguments)` | left |

```ebnf
Expr      ::= IfExpr | ForExpr | OrExpr
IfExpr    ::= 'if' Expr 'then' Expr 'else' Expr
OrExpr    ::= AndExpr ('or' AndExpr)*
AndExpr   ::= NotExpr ('and' NotExpr)*
NotExpr   ::= 'not' NotExpr | CmpExpr
CmpExpr   ::= Fallback (CmpOp Fallback | 'is' 'not'? AltType)?
CmpOp     ::= 'is' 'not'? | '<' | '<=' | '>' | '>='
AltType   ::= 'none' | (ScalarType | StrType | MapType | JsonType | (Name | QualifiedName) TypeArgs?) Dims?
Fallback  ::= AddExpr ('??' Fallback)?
AddExpr   ::= MulExpr (('+' | '-' | 'without') MulExpr)*
MulExpr   ::= UnaryExpr (('*' | '/' | 'div' | '%') UnaryExpr)*
UnaryExpr ::= '-' UnaryExpr | PowExpr
PowExpr   ::= Postfix ('**' UnaryExpr)?
Postfix   ::= Primary ('.' Name | '[' Index ']' | '(' Args? ')')*
Primary   ::= NumberLit | BoolLit | StringLit | RecordLit | MapLit | JsonLit | 'none' | Name | QualifiedName
            | 'to' | Conversion | ArrayLit | '(' Expr ')'
ArrayLit  ::= '[' (Expr (',' Expr)* ','?)? ']'
Args      ::= Arg (',' Arg)* | NamedArg (',' NamedArg)*
Arg       ::= Expr | Lambda
NamedArg  ::= Name '=' Arg
```

Comparisons do not chain: `a < b < c` is an error. `-2 ** 2` is −4, and `n-1` is `n - 1`. `??` is the fallback on a `T?` (§5.3), and binds looser than arithmetic and tighter than a comparison, so `port ?? 80 > 1024` is `(port ?? 80) > 1024`, `x ?? base + 1` falls back to `base + 1`, `x ?? -1` needs no parentheses, and `-x ?? 1` is an error, since unary minus takes no `T?`; `(x ?? 0) + 1` adds to the resolved value. An `if` that is an operand of another operator is parenthesized: `base + (if high then 12 else 0)`. An `if` that is a whole value, such as a `let`'s value, an attribute value or an argument, needs no parentheses. A `{` in an expression opens a map literal (§3.10), after a type name a record literal (§3.8), and after `json` a document literal (§3.11).

Expressions evaluate the same way at compile time and at runtime; §4.6 states the differences in compile-time arithmetic.

### 5.2 Arithmetic

`+`, `-` and `*` take numeric operands of one type after widening; `+` also concatenates strings (§5.9) and joins arrays (§5.5). Every integer operation saturates at the type's bounds: `2147483647 + 1` in `i32` is 2147483647, negating −2147483648 in `i32` yields 2147483647, and in `u8`, `3 - 5` is 0.

`/` always yields a float. Integer operands convert to the result's float type: the expected type when one exists, otherwise `f64` if either operand is `i64` or `f64`, otherwise `f32`. `1 / 4` is 0.25.

`div` takes integers and rounds toward negative infinity: `-7 div 2` is −4, and `-2147483648 div -1` in `i32` is 2147483647. `%` takes integers or floats, and its result has the sign of the divisor: `-7 % 2` is 1, `-1 % 7` is 6, and an integer `x % -1` is 0. Integer `div` or `%` by zero yields 0. On floats, `a % b` is `fmod(a, b)`, plus `b` when that result is neither zero nor NaN and its sign differs from `b`'s; the sum can round to `b` itself, so `-1e-20 % 1.0` is 1.0. A zero float result takes the divisor's sign, so `-2.0 % 1.0` is 0.0.

`**` raises to a power. With integer operands it yields an integer of their type, saturating; a negative exponent yields 0, except that base 1 yields 1 and base −1 yields 1 or −1 by the exponent's parity, and `0 ** 0` is 1. With a float operand it yields a float of the operands' type after widening. In an integer position `2 ** 4` is the integer 16. `2 ** (7 / 12)` is the frequency ratio of a fifth, a float because `7 / 12` is one.

Float arithmetic follows IEEE 754, including infinities and NaN: a float divided by zero is an infinity or NaN, and a float `%` by zero is NaN.

### 5.3 Comparison and logic

`is` and `is not` compare two scalars, enum values or strings of one type after widening; floats compare by IEEE 754 equality, and strings character by character. Two documents, or a document and a value that widens into one, compare by JSON value (§3.11). A record, a map or an array there is otherwise an error. `<`, `<=`, `>` and `>=` compare numbers. `and`, `or` and `not` take and yield `bool`.

With a type on the right, an `AltType` (§5.1), the form an alternative of a sum takes, `is` and `is not` test a sum (§3.9): `x is none` is true when `x` holds no value, and `x is Circle` or `x is i32`, with an alternative's type, when `x` holds that alternative. A right operand that is `none`, a name or a qualified name, with any bracketed suffix or without, parses as a `Fallback`: `none` is this test, or on a document the `Null` test (§3.11), and a name is this test when it names a type (§6.5); no narrowing follows. `x ?? y`, with a `T?` as its left operand, is the fallback: it yields `x`'s value when `x` is not `none` and `y` otherwise, `y` a `T` or a `T?` and the result the same, a `bool?` included, so `flag ?? true` is `flag`'s value whenever `flag` holds one. The type a `??` expression's position expects (§4.2) flows into both operands, `T` into the right and `T?` into the left, so `state id: i64 = body["id"] ?? 1` reads the document as an `i64?`; where the position expects none and the left operand is a document (§3.11), the right operand's type, a flexible one taking the defaults of §4.5, is the expected type of the left, which makes it a typed read as `T?`. `or` takes `bool` operands only, and a `T?` beside it is an error whose help names `??`.

### 5.4 Conditionals

`if c then a else b` yields `a` when `c` is true and `b` otherwise. The condition is a `bool`. Both branches are type-checked, and only the chosen branch is evaluated; at compile time only the chosen branch is checked for fit and index range. `else` is mandatory, and `else if` chains:

```sym
state loud(false)
state quiet(false)
state gain = if loud then 0.9 else if quiet then 0.3 else 0.6
```

`match` selects a value by an enum, an integer, a `bool`, a string or a sum (§3.9):

```sym
state wave(Wave::Saw)

state level = match wave:
    Sine: 1.0
    Saw: 0.8
    else: 0.6
```

```ebnf
MatchBlock ::= 'match' Expr ':' NEWLINE INDENT MatchArm+ ElseArm? DEDENT
MatchArm   ::= (AltType | '-'? NumberLit | BoolLit | StringLit) ':' Expr NEWLINE
ElseArm    ::= 'else' ':' Expr NEWLINE
```

- A scrutinee of another type is an error. Arm labels are variant names of the scrutinee's enum, or literals of its type and unit family, a string label being a compile-time string. A label that repeats an earlier one is an error.
- A `match` on an enum or a `bool` covers every value or ends with `else:`, and an `else:` after arms that cover every value is an error. A `match` on an integer or a string ends with `else:`.
- On a sum, a label is an alternative's type, an `AltType` (§5.1), `none` and `i32` among them; a label that is a name is a variant or a type by the scrutinee's type. Inside the arm the scrutinee, when it is a name, reads as that alternative; in `else` it reads as the alternatives no arm labels, and as that one type when one is left. A `match` on a sum covers every alternative or ends with `else:`, and a label that is no alternative of it is an error.
- The first matching arm wins. Arms are checked and evaluated as `if` branches are, and arm values unify as `if` branches do (§4.2).
- `match` is the whole value of a `let`, top-level or local, of a derived cell's rule, of a function body or of a wire destination (§8.2), never an operand: its arms need layout, and layout is suspended inside parentheses.

### 5.5 Arrays and indexing

```ebnf
Index ::= Bound | Bound? '..' Bound?
Bound ::= '^'? Expr
```

`a[i]` reads element `i`, counting from 0. The index is an integer of either width, and a flexible index takes the defaults of §4.5. An index below 0 reads the first element, and an index past the live length reads the last live element. A compile-time index outside a fixed array is an error.

`^k` at the start of an index or a slice bound of `xs` stands for `xs.size - k`, counting from the live end: with `xs = [10, 20, 30, 40, 50]`, `xs[^1]` is 50 and `xs[^2..]` is `[40, 50]`.

`xs[a..b]` is the slice of `xs` from element `a` up to but excluding element `b`; `xs[a..]` runs to the end and `xs[..b]` starts at 0. Each bound is clamped to between 0 and the live length, and a slice whose upper bound is not above its lower one is empty, so a slice never fails: `xs[1..^1]` drops the first and the last element, and `xs[..i] + xs[i + 1..]` removes element `i`. A slice of a fixed array whose bounds are known at compile time is a fixed array; any other slice is a variable array (§3.7).

A string is indexed and sliced as the array of its characters (§3.3): `s[i]` is the one-character string at `i`, clamped as an index is, and the empty string when `s` is empty; `s[a..b]` is a string; `^k` counts characters from the end. `"a日b"[1..]` is `"日b"`, and `"a日b"[^1]` is `"b"`.

An array literal whose elements read cells is an array of bindings, each element bound separately:

```sym
state a(1)
state b(2)
state i(0)
state c = [a, b, a + b]
```

- `c[1]`, with an index known at compile time, is the same binding as `b`. A change to `a` does not reach it.
- `c[2]` is a cell of its own, computed from `a` and `b`.
- `c[i]` depends on `i` and on every element.

A fixed-array expression that reads cells is evaluated element by element; a variable array is evaluated whole (§3.7). With `state row = grid[i]`, `row[1]` is `grid[i][1]`, and `if flag then [60, 64] else [62, 65]` chooses each element separately.

`a + b` with two arrays joins them, the elements of `a` followed by those of `b`. Their elements unify as an array literal's do (§4.2), and their rows have one shape. The join of two fixed arrays is a fixed array, `[1, 2] + [3]` being `[1, 2, 3]`; any other join is a variable array. Every other operator takes scalars only, and `+` of an array and a scalar is an error.

```sym
state held([]): i32[..10]
state step(0)
state note(60)

state played = held + [note]

Clip arp:
    Note/(played[step % played.size])
```

### 5.6 Attribute reads

`x.name` reads attribute `name` of the instance or wire `x`. The read is the same binding the attribute holds: a constant, a cell, or the vocabulary default when the source sets none.

- A path starts at a name in scope (§6.5), a parameter or a `let` holding a reference (§3.5) included, or at a reference chosen at compile time by an index, an `if`, a `match` or a call: `sections[0].gain`. It steps through children, named wires and the names inside named pattern uses: `chorus.back`, `lead.v1.osc`.
- A path whose start is chosen at runtime is an error; `to.path` reads an attribute along a path from a wire's destination, which may be chosen at runtime (§8.3).
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

### 5.7 Calls, members and the `math` package

A call invokes a prelude function, native functions included (§10.1), a source function (§6.3), a parameter bound to a function reference (§6.7), a `let` holding a function reference (§6.7), or a function of the `math` package. Arguments are all positional or all named, as in `scale(3, 48)` or `scale(degree=3, root=48)`; mixing them in one call is an error. Native and `math` functions take positional arguments only.

`x.f(a, b)` is `f(x, a, b)` for any function in scope: the receiver fills the first parameter, and the parenthesized list fills the rest by position, or names any of the rest where the function's parameters have names, a native or `math` function's having none. `x.name` and `x.name(…)` resolve to a member of `x`'s own type first, an attribute of an instance or a wire (§5.6), a field of a record (§3.8) or a member of a built-in type, and to a function in scope only when the type has no such member.

Built-in values have members, resolved as a record's fields are (§3.8); no member is a top-level name (§6.5), so a kind's attribute or a record's field named `size` or `keys` is its own:

| value | members |
|---|---|
| an array | `size`, the live length, on a fixed array a flexible integer known at compile time and otherwise an `i32`; `capacity`, a compile-time value, a fixed array's its length (§3.7); `bytes`, a `u8` array: a `u8` array itself, and any other integer array element by element as `u8(x)` converts; `map(f)`, the array of `f(x)` for each element `x`, in order, of the array's length and capacity and of `f`'s result type, `f` a function reference or a lambda of one parameter (§6.7), evaluated as a `for` expression over the array is (§5.10), its element capacities those of one application of `f` |
| a string | `size`, its number of characters, a compile-time string's a flexible integer known at compile time and a runtime string's an `i32`; `capacity` (§3.3); `bytes`, its UTF-8 bytes, a fixed array of them for a compile-time string and a variable array of four times the string's capacity for a runtime string |
| a map | `size`, its number of entries, on a compile-time map a flexible integer known at compile time and otherwise an `i32`; `capacity`; `keys` and `values`, the keys and the values in entry order, fixed arrays on a compile-time map and otherwise variable arrays of the map's capacity (§3.10) |
| a document | `size`, an array's length or an object's entry count, an `i32`, and 0 for anything else; `capacity`, its budget; `keys` and `values`, an object's keys and values as array views; `kind`, a value of the built-in enum `Json`: `Object`, `Array`, `String`, `Number`, `Bool` or `Null` (§3.11) |
| a cell | `past(k)` and `past(k, fallback)` (§6.6); `stepped`, a `bool`, whether the cell stepped at the current step (§6.6); on a host-fed `stream` or `channel`, `dropped`, an `i32`, at a step of the cell the number of writes forgotten since its step before, 0 when none was and always 0 on a channel (§12.1) |

The `math` package is a library every build holds, required with no `require` item (§15.3): its names are reachable as `math::sin` always, and a package file's `use math` (§15.4), which `sym new` writes, brings them in unqualified, so a package that drops the line frees the names.

| `math` name | Result |
|---|---|
| `abs(x)`, `min(a, b)`, `max(a, b)`, `clamp(x, lo, hi)` | numeric, of the operands' type |
| `floor(x)`, `ceil(x)`, `round(x)` | float; `round` rounds half away from zero |
| `sqrt(x)`, `sin(x)`, `cos(x)`, `tan(x)`, `exp(x)`, `log(x)` | float; `log` is the natural logarithm |
| `lerp(a, b, t)` | float, `a + (b - a) * t` |
| `rand(seed, n)` | `f32` in [0, 1), a fixed function of two integers (§12.5) |
| `pi` | the float constant π |

- A call whose arguments are all flexible is flexible (§4.1), and `pi` is flexible. `rand` always yields `f32`.
- Integer arguments to float functions convert as `/` does.
- `abs` saturates on integers: `abs` of −2147483648 in `i32` is 2147483647.
- `clamp(x, lo, hi)` is `min(max(x, lo), hi)`, so with `lo` above `hi` it yields `hi`.
- `min` and `max` with one NaN operand yield the other operand. `min(-0.0, 0.0)` is −0.0, and `max(-0.0, 0.0)` is 0.0.

Domain functions, such as converting a note number to a frequency, come from the vocabulary's prelude (§10.1).

### 5.8 Conversions

```ebnf
Conversion ::= ('u8' | 'i32' | 'i64' | 'f32' | 'f64') '(' Expr ')'
```

`u8(x)`, `i32(x)`, `i64(x)`, `f32(x)` and `f64(x)` convert a number explicitly. Float to integer truncates toward zero; a value outside an integer type's range saturates to its bounds, and NaN becomes 0. A float converted to `f32` rounds to the nearest `f32` (§5.2), so a value beyond its range becomes an infinity. Converting a `bool` or an enum value is an error.

### 5.9 Strings

`+` concatenates strings. `{expr}` inside a literal interpolates a value:

- an integer in decimal;
- a float as the shortest decimal that reads back as the same value, without an exponent and with `.0` when it has no fraction: `2.0`, `0.1`, `-0.0`; infinities and NaN as `inf`, `-inf` and `nan`;
- a `bool` as `true` or `false`, an enum variant by name, and a string as its text;
- a unit value as its value in the family's base unit.

An interpolation of a value that reads a cell makes the string a runtime string (§3.3) and takes an integer, a `bool`, an enum variant or a string; any other value there is an error, since a float is formatted at compile time only; a native function that formats a float at runtime is the framework's (§10.1).

### 5.10 The `for` expression

```ebnf
ForExpr      ::= ForExprHead Expr
ForExprBlock ::= ForExprHead NEWLINE INDENT LocalLet* Expr NEWLINE DEDENT
ForExprHead  ::= LoopHead 'into' Name (':' Type)? 'from' Expr ':'
```

A `for` expression folds an iterable (§7.9) into one value. The name after `into` is its accumulator, which starts as the value after `from`. The body runs once per element of the iterable, in order, with the loop names (§7.9) and the accumulator in scope, and the value of each run is the accumulator of the next. The expression's value is the last accumulator, or the start when the iterable is empty.

- The accumulator's type unifies the start and the body as `if` arms do (§4.2), and an annotation fixes it (§4.5); a capacity found by the next rule beyond an annotated one is an error naming both.
- Every capacity or budget in the accumulator's type, an array's at every level, a string's, a map's and a document's, is found by applying the body's capacity or budget rule (§3.3, §3.7, §3.10, §3.11) to the start's once per iteration the iterable can run, stopping when it no longer grows. An array or a map runs at most its capacity of iterations, and a range at most the count its bounds allow (§7.9).
- The block form is the whole value of a `let`, a local `let`, a derived cell's rule or a function body, never an operand, as `match` is (§5.4); its local `let`s are visible throughout the block, as a function body's are (§6.3). The one-line form stands without parentheses only as the whole value of a `let`, a local `let`, a derived cell's rule or a function body; anywhere else, as an operand, an argument, an attribute value, a gate, a scrutinee, a start or an iterable, it stands in parentheses.
- A `for` expression whose iterable, start and body read no cell is a compile-time value (§4.6). Any other is one cell (§4.4), evaluated whole each time it runs (§12.2).
- Its loop names, its accumulator and its local `let`s follow §6.5's name rules.
- The body is inlined once, however many times it runs, and counts once toward §4.6's inline limit.

```sym
state held([]): i32[..10]

state high = for x in held into kept from []:
    let keep = x > 60
    if keep then kept + [x] else kept

state total = 1 + (for i, x in held into sum from 0: sum + i * x)
```

`high` keeps the held notes above 60 in their order and holds at most 10, since each run adds at most one element to a start of none; `total` weighs each note by its position.

```sym-error
state held([]): i32[..10]

state total = 1 + for x in held into sum from 0: sum + x
```

This is an error: a `for` expression that is an operand stands in parentheses.

```sym-error
fn count(xs): xs.size

state held([]): i32[..10]

state high = count(for x in held into kept from []: if x > 60 then kept + [x] else kept)
```

This is an error: a `for` expression that is an argument stands in parentheses, although an `if` there needs none (§5.1).

## 6. Declarations

### 6.1 Cells

```ebnf
CellDecl ::= CellWord ('[' Digits ']')? Name Init? (':' Type)? Clock? ('=' (Expr NEWLINE | MatchBlock | ForExprBlock) | NEWLINE)
CellWord ::= 'state' | 'stream' | 'channel'
Init     ::= '(' Expr ')'
```

A cell is a value that changes while the graph runs. Its word says what a step of it is: a `state` steps when its value changes, a `stream` and a `channel` step on every arrival or run. Its form says who feeds it: a declaration with an init in parentheses and no rule is host-fed, a cell the host writes (§12.1); a declaration with a rule after `=` is derived, computed by the program (§6.6), with or without an init. A declaration with neither is an error. Cells are declared at top level and in pattern bodies (§6.4). `Clock` is §6.6's and stands on a derived cell only.

A host-fed `state` keeps the last value written and steps when a write changes it, an equal write being nothing. A host-fed `stream` and a host-fed `channel` step on every write, equal or not, and keep every write since the last commit as a pending step, up to a capacity: a `stream` forgets the oldest pending write when the capacity is full, and a `channel` refuses the write until a commit drains (§12.1). `[N]` after `stream` or `channel` states the capacity; absent, it is the largest capacity an attribute the cell is bound to states (§10.1), and 1 when it is bound to none. `[N]` on a `state` or on a derived cell is an error, and `[N]` below the capacity a bound attribute states is an error naming the attribute and its capacity; a larger `[N]` is always allowed. A cell's history depth is never declared (§6.6).

The init is a compile-time value and gives a host-fed cell its type (§4.4); an annotation follows it. At load a host-fed cell holds its init. A cell may hold an array, a string, a record, a sum, a map or a document, never a node reference or a function; a string cell states its capacity (§3.3), a record init names its type (§3.8), a document cell states its budget (§3.11), and a cell annotated with a variable array type (§3.7) takes an init no longer than its capacity: `state held([]): i32[..10]`. The host writes a host-fed cell by its handle, by name (§12.1), whether or not an attribute is bound to it (§7.6). `stream tick(none)` is a cell that carries nothing, a clock the host steps; a cell of type `none` is legal.

```sym
state cutoff(0.5)
state steps([true, false, false, true, false, false, true, false])
state name(""): str[..32]
state hit(Hit {})
stream mic(0.0)
channel[16] notes(Hit {})
```

### 6.2 Lets

```ebnf
LetDecl ::= 'let' Name (':' Type)? '=' (Expr NEWLINE | Lambda NEWLINE | MatchBlock | ForExprBlock)
```

A `let` names a compile-time value (§4.1). A top-level `let` or a pattern's `let` that reads a cell, directly or through other declarations, is an error whose help names `state`, except a `let` holding a function reference (§6.7), whose body runs only at a use; a function's local `let` follows its parameters (§6.3). An `if` or a `match` whose condition or scrutinee is a compile-time value reads the cells of the arm it takes alone, so `let x = if false then note else 1` reads no cell and is a compile-time value; a cell named in a recursive function's body counts in every arm (§6.3). A declaration that reaches itself through `let`s, cells, attribute reads, function bodies or the lambda a `let` holds is an error naming the cycle, as in `let a = b + 1` with `let b = a + 1`, and in `let f = x: f(x)`, whose recursion is written with `fn`; a function reaching itself through calls follows §6.3.

A `let` may hold a reference (§3.5) or an array of them, or a function reference (§6.7). It stands for its value wherever it is used and never reaches the Program. Every `let`, top-level or local, a function's local `let` and a pattern's `let` among them (§6.3, §6.4), holds a function reference as §6.7 says. A choice among instances at runtime is written in the destination itself (§8.2).

```sym
state section(0)

let sections = [verse, chorus, bridge]

Clip intro:
    for s in sections:
        Starts: s

Clip outro:
    Plays: sections[section]
```

### 6.3 Functions

```ebnf
FnDecl   ::= 'fn' Name GenericParams? '(' Params? ')' Result? ':' (Expr NEWLINE | NEWLINE INDENT FnBody DEDENT)
Result   ::= '->' Type
FnBody   ::= LocalLet* (Expr NEWLINE | MatchBlock | ForExprBlock)
LocalLet ::= 'let' Name (':' Type)? '=' (Expr NEWLINE | Lambda NEWLINE | MatchBlock | ForExprBlock)
Params   ::= Param (',' Param)*
Param    ::= 'cell'? Name (':' Type)? ('=' Expr)?
```

A function computes one value from its parameters.

- A parameter marked `cell` is bound to the argument's cell itself, of any kind, or to a compile-time argument's value, an argument that is an expression reading cells binding it to the anonymous `state` the expression is (§6.6), so the body reads its history, and a call with such a parameter is always inlined. A parameter without it is bound to the argument's value at each run, the argument evaluated as part of the caller's rule and no cell of its own, such a function may be a shared body, and a history read, `stepped` or `dropped` of it is an error whose help names `cell`. Any other marker of §6.4 on a function parameter is an error.

- A body is one expression, or local `let`s followed by one expression or `match`, the local `let`s visible throughout the body. A local `let` follows the arguments it reads: bound to compile-time values it is a compile-time value, and reading a parameter bound to a value that reads cells it is such a value too. Two parameters or two local `let`s with one name, or a local `let` named like a parameter, are an error.
- A call to a non-recursive function evaluates the body in the function's own scope, with each parameter bound to its argument, and the argument evaluated in the caller's scope. The compiler checks the result in place, as the body inlined at the call. A parameter is typed as a `let` holding its argument would be: bound to a compile-time argument it is a compile-time value, flexible when the argument is (§4.1), and bound to an argument that reads cells it is typed as a derived cell is (§4.4), each use reading it in the use's type.
- Whether a call to a non-recursive function is inlined at its use or invokes the function's body stored once in the Program as a shared body (§11) is the compiler's choice by cost, and changes no value; a function with a `cell` parameter is inlined at every call. A call whose arguments are all compile-time values is evaluated by the compiler either way.
- A parameter's annotation fixes its type (§4.5): a compile-time argument fits it and one that reads cells is read in it, as a cell is read at a use (§4.4). A parameter annotated with a node kind or a wire kind takes a reference (§3.5), and a parameter bound to a reference reads attributes through it as a path does (§5.6). A result annotation is the body's expected type (§4.2). A local `let`'s annotation is its initializer's expected type.
- A call whose arguments read cells yields a value that reads them.
- A parameter with a default follows every parameter without one, and its default may read earlier parameters. A positional call may omit trailing parameters that have defaults; a named call may omit any parameter that has a default. A call that supplies more arguments than the function has parameters, names a parameter the function lacks, or names one twice is an error.

```sym
fn db(x): 10 ** (x / 20)

fn scale(degree: i32, root: i32 = 60) -> i32:
    let octave: i32 = degree div 7
    root + [0, 2, 4, 5, 7, 9, 11][degree % 7] + 12 * octave
```

A function that reaches itself through calls, directly or through other functions, is recursive. A call to a recursive function reads a cell when one of its arguments does, or when the body of its function, or of a function it calls, names a cell anywhere, an arm the evaluation never takes included. A path in the body that starts at a reference the evaluation chooses (§5.6) reads, in each evaluation, the binding of the reference that evaluation chooses: the call reads a cell through it only when that binding is a cell, and an attribute the chosen reference's kind lacks, or a cycle through it (§5.6), is an error for the chosen reference alone, never for a reference the evaluation does not choose. A call that reads no cell is evaluated by the compiler to a value (§4.6), and evaluation deeper than the nesting limit, 1,024 nested calls by default (§4.6), is an error. A call to a recursive function that reads a cell is an error whose help says to write a `for` expression (§5.10). These diagnostics name the chain of calls.

```sym
fn fib(n): if n < 2 then n else fib(n - 1) + fib(n - 2)

let lengths = [fib(3), fib(4), fib(5), fib(6)]
```

```sym
state held([]): i32[..10]

state lowest = for x in held into low from 127: min(low, x)

Clip c:
    Note/lowest
```

`lowest` is the lowest held note, or 127 while none is held.

```sym-error
state n(5)

fn fib(k): if k < 2 then k else fib(k - 1) + fib(k - 2)

Clip c:
    Note/(fib(n))
```

This is an error: `fib` is recursive, and its argument reads the cell `n`; the help says to write a `for` expression, as `lowest` is written.

### 6.4 Patterns

```ebnf
PatternDecl    ::= 'pattern' Name GenericParams? '(' PatternParams? ')' ('->' PlacementType)? ':' NEWLINE INDENT PatternItem* BodyItem+ DEDENT
PatternParams  ::= PatternParam (',' PatternParam)*
PatternParam   ::= 'default'? Marker? Name (':' Type)? ('=' Expr)?
                 | 'children' Name ':' PlacementType
Marker         ::= 'const' | 'cell' | 'state' | 'stream' | 'channel' | 'out' ('state' | 'stream' | 'channel')?
PatternItem    ::= CellDecl | LocalLet
PlacementType  ::= PlacementEntry | '(' PlacementEntry (',' PlacementEntry)* ')'
PlacementEntry ::= (Name | QualifiedName | 'node' | 'wire' | '[' PlacementEntry ']') ('+' | '*' | '{' Count '}')?
Count          ::= Digits ('+' | '..' Digits)? | '..' Digits
```

A pattern is a named declaration of placements with parameters and a type. A use places them as one node, one wire or a group, with cells of its own, and the pattern's name stands in a `children` type and in the paths of what a use places (§11.1).

- Patterns are declared at top level. The body holds its declaration lines first, cells and `let`s in any order, then any body items (§7.1). Each expansion has its own cells and `let`s, visible throughout the body. A cell of a use is an ordinary cell of the Program (§11), named by the use's path (§11.1): `Voice as low` with `state gain(1.0)` in the pattern yields the cell `low.gain` under the use's parent, one per use, and one per iteration inside a `for`, and a host-fed one is the host's handle. A cell's init is a compile-time value that may read the pattern's compile-time parameters; a derived cell's rule may read the pattern's cells, parameters and `let`s. Two parameters or two declarations with one name, or a declaration named like a parameter, are an error.
- A pattern is used with placement syntax, or with wire syntax when it stands in for a wire. The attribute clause supplies parameters in every entry form of §7.6, so `name` and `!name` supply a `bool` parameter, and `/expr` supplies the parameter marked `default`, of which there is at most one and which the clause may supply by name instead. `/expr` on a pattern without a `default` parameter is an error. Supplying a parameter twice is an error, and so is omitting a parameter that has no default value. A use carries no flags, and a body only for `children` parameters.
- A use is replaced by the body's placements at the use's position, with each parameter bound to its argument as in a function call (§6.3), and the result is checked in place. Parameters are typed as a function's are (§6.3); a parameter's annotation fixes its type (§4.5), and a node kind or a wire kind there names a reference, `Osc[3]` three of them. An argument that needs an expected type, such as a bare variant in `Voice | shape=Saw`, takes it from the parameter's annotation or from the parameter's uses in the body. An argument may be a value, a cell or a value that reads cells, a reference (§3.5) or an array of references.
- A parameter may carry a marker, which states what it accepts; the author writes it, and the compiler infers none from the body. A kind word accepts the cells of that kind, host-fed or derived, and a compile-time value, which widens into it; `out` before a kind word accepts a host-fed cell of exactly that kind, and `out` alone a host-fed cell of any kind. An argument the marker refuses is an error at the use naming the parameter and the marker; where the argument is an expression over a `stream` or a `channel` and the marker asks for one, the help says to declare a derived `stream` or `channel` and pass it by name (§6.6). `default` marks the parameter `/expr` supplies. The words stand in the grammar's order, and a marked parameter's default value is one its marker accepts. A history read, `stepped` or `dropped` of a parameter (§6.6) needs a marker, `const` excluded; on an unmarked parameter it is an error whose help names `cell`. An argument that is an expression reading cells binds a parameter marked `cell` or `state` to the anonymous `state` it is (§6.6), and an unmarked parameter to the expression itself, as a function's unmarked parameter is bound (§6.3).

| marker | accepts |
|---|---|
| none | anything: a compile-time value, a reference, a cell |
| `const` | a compile-time value, a reference among them |
| `cell` | any cell, or a compile-time value a cell can hold; never a node, wire or function reference |
| `state` | a `state`, or a compile-time value; never a `stream` or a `channel`, whose equal arrivals it would collapse |
| `stream` | a `stream` or a `channel`, a channel being a stream that forgets nothing, or a compile-time value; never a `state` |
| `channel` | a `channel`, or a compile-time value |
| `out` | a host-fed cell of any kind; never a compile-time value or a derived cell |
| `out state`, `out stream`, `out channel` | a host-fed cell of that kind and no other, because the writer's code depends on the buffer behind it |
- A placement type lists entries with counts. An entry is a node kind, a wire kind, `node` for any node kind, `wire` for any wire kind, a pattern's name, meaning a use of that pattern, which stands only in a `children` parameter's type, or a bundle `[…]` of one node entry (§7.8). A count follows it: none for exactly one, `+` one or more, `*` zero or more, `{3}` exactly three, `{2+}` two or more, `{..4}` at most four, `{2..4}` two to four, both included. A single entry is written bare; a list is parenthesized.
- A pattern's type is what a use places. A pattern whose body is a single node or wire placement, or a single use of such a pattern, stands in for that placement: its type is that kind, written bare; a use creates that one instance or wire, and the use's name becomes its name, replacing any `as` name the placement has. Only node stand-ins appear at top level. A stand-in wire placement writes no destination: the use writes it after the colon, as a wire placement does (§8.1), and `to` in the pattern's wire reads it (§8.3); a destination written in the pattern is an error. Every other pattern is a group, whose type is parenthesized and lists what the body places, one entry per node kind, wire kind and bundle shape in order of first placement, with the tightest count the body fixes: a placement written once counts one, inside a `when` arm too (§9.2); a bundle counts one bundle entry of its members' kind and count; a `for` whose count the declaration fixes counts its iterations, and one whose count depends on a parameter counts zero or more; a nested use counts its pattern's entries, and a `children` parameter its declared type; `(Clip)` is a group placing one `Clip` that does not stand in. A type written after `->` is a contract: the body places exactly the entries it names, each count inside the declared one, or the declaration is an error naming the placement that breaks it; a type the declaration leaves out is inferred.
- A parameter marked `children` takes placements: the use's body, in ordinary body syntax. A pattern may declare several, naming disjoint entries, or the declaration is an error; a wire pattern declares none; `children` stands with no other modifier. The use's one body is partitioned: each item goes to the parameter whose type names the pattern it uses, by the pattern's name and before any expansion; else, when it is a bundle, to the parameter whose type has a bundle entry its members fit; else to the parameter naming its kind, `node` and `wire` naming every kind; a `when` block's items are partitioned one by one, each carrying the block's condition as its gate (§9.2); each share keeps its relative order. An item no parameter takes is an error at the use, and so is a share outside its parameter's counts. A use with no body supplies nothing, which fits only counts that allow zero; a body on a use of a pattern without a `children` parameter is an error. The parameter's name written as a line in the pattern body, alone or with a gate, places its share there, once, never inside a `for`: as children of the instance whose body holds the line, at that point of its sequence (§7.3), or at the pattern body's top level, where its type joins the pattern's; the gate gates them. Written as a bundle's member, `[notes]`, it makes one bundle of its share, whose type then holds node entries only and takes no bundle (§7.8). Written as a line in the body of a nested use, it forwards its share to that use, whose own `children` parameters partition it as they partition any use's body. The use's body resolves its names where the use is written, as an argument does (§6.3); its `as` names are reachable through the use's name; a wire among its items has as its source the instance whose body holds the line.
- A wire placed directly in the body has as its source the instance whose body holds the use.
- A pattern body sees top-level names, its parameters, its declarations and its own names (§6.5), never the names around its use. A top-level name written in the body resolves as usual, but the body has no name for the instance its own use creates.
- Names inside an anonymous use are unreachable and never clash with names outside it. A named use (`Voice as v1`) makes them reachable through its name: `v1.osc`.
- A pattern may use other patterns, but no pattern reaches itself through uses.
- The vocabulary checks (§10.2) apply to the expanded placements. A diagnostic points at the use and at the line inside the pattern.
- A `when` on a use gates every placement it expands to.

```sym
pattern Arp(default root, steps = 4, gap = 4) -> (Note*):
    for i in 0..steps:
        Note/(root + i * gap)

Clip verse:
    Arp/48
    Arp/60 | steps=8 gap=3
```

```sym
pattern Riff(default root: i32) -> Clip:
    Clip:
        Note/root Note/(root + 7) Note/(root + 12)

Riff low /36
Riff high /48
```

A pattern that needs the instance its use creates takes it as an argument:

```sym
pattern Loop(default root, again: Clip):
    Clip:
        Note/root Note/(root + 7)
        Plays: again

Loop low /36 | again=low
```

A pattern that stands in for a wire is used as a wire:

```sym
pattern Up(semis = 12) -> Plays:
    Plays | transpose=semis

Clip verse:
    Note/60
    Up as octave | semis=7: chorus
```

`octave` is a `Plays` wire from `verse` to `chorus`, and `verse.octave.transpose` is 7. A pattern's host-fed cells are the host's way into each use:

```sym
pattern Voice(default root: i32, const steps = 4) -> (Note*, Starts):
    state gain(1.0)
    for i in 0..steps:
        Note/(root + i) | velocity=gain
    Starts: intro

Clip verse:
    Voice as low /48
    Voice/60 | steps=8
```

The Program holds the cells `verse.low.gain` and `verse.Voice:0.gain`. An `out` parameter takes a host-fed cell the program declares, which the body binds with `=>` (§7.6), and `children` parameters take the use's body, split by what each names:

```sym
pattern Drum(default out channel hit: Hit, gain: f32 = 1.0) -> Pad:
    Pad | hit => hit gain=gain

pattern Section(default root: i32, children notes: Note*, children links: Plays*) -> Clip:
    Clip | gain=0.8:
        Note/root
        notes
        Plays: intro
        links

channel kick_hit(Hit {})

Drum kick /kick_hit | gain=0.9

Section bass /36:
    Note/43 | velocity=0.6
    Plays: verse
```

`bass` is the `Clip` with the notes 36 and 43, a `Plays` wire to `intro` and one to `verse`, both from `bass`; a body with an `Osc` is an error at the use. A bundle entry expects a chord, and a parameter's name inside brackets makes one:

```sym
pattern Stab(default root: i32, children chords: [Note+]*) -> Clip:
    Clip:
        Note/root
        chords

pattern Chord(children notes: Note{2+}) -> ([Note{2+}]):
    [notes]

Stab hit /48:
    [Note/52 Note/55 Note/59]
    [Note/50 Note/53]

Clip pad:
    Chord: Note/60 Note/64 Note/67
```

Patterns never appear in the Program.

### 6.5 Names and scope

All top-level names of a package share one namespace across its files: cells, `let`s, functions, patterns, record types, top-level instances, the names its package file brings in (§15.4), and, in a package written for a vocabulary, the vocabulary's kinds and enums and the names its prelude publishes (§15.6). A duplicate is an error naming both declarations; a declaration of the package and a name its package file brings in shadow a prelude name (§15.6). Top-level declarations refer to each other regardless of order. A declared name of any kind, top-level or local, that equals a variant of a vocabulary enum is an error.

Inside a body, the `as` names of that body's children and wires are visible regardless of order, and so are those of every enclosing body up to a pattern's body, and the top-level names. A child or wire name shadows top-level names and prelude names within its body and the bodies nested in it. A child or wire name equal to a name of its own body or of an enclosing body, or to an attribute of its parent's kind, is an error, also when the two names stand in different arms of one `when` block. A child or wire name equal to a pattern parameter, a pattern declaration or a `for` variable visible where it is declared is an error, and so is a pattern declaration equal to a child or wire name of its body. A `for` variable is visible in its body, and a `for` variable equal to a visible name is an error; so is a `for` expression's loop name, its accumulator or its block form's local `let` (§5.10), or a lambda's parameter (§6.7), equal to a visible name. The names of one loop head (§7.9), and a `for` expression's accumulator and its block form's local `let`s, are distinct names, as a function's parameters and local `let`s are (§6.3). A bracket name (§6.8) equal to a top-level name is an error. In a function, parameters and local `let`s shadow top-level names; in a pattern, parameters and the body's declarations do. A record's field names are scoped to its type (§3.8).

A function or pattern that nothing uses has its names resolved, cycles among its `let`s included (§6.2), and no types checked. So do a parameter default that no call takes and the body of a placement `for` that runs no iteration (§7.9), since nothing expands them. A compile-time `let` that nothing uses is never computed, so no error of its value, such as an index out of range or a value past 2¹²⁷, is reported. In all of these, resolving the names draws its errors, a name, path or attribute that resolves to nothing or to what its position cannot hold, and nothing else is checked or computed, in the bodies the code reaches, recursive or not, as in the code itself.

A bare name that is a variant of the enum its position expects resolves to that variant (§3.4). Every other name resolves in scope.

### 6.6 Derived cells

```ebnf
Clock ::= 'on' (Atom | '(' Expr (',' Expr)* ')')
```

A derived cell is computed by its rule, the expression after `=`. It runs when a cell it reads steps, in the present or through a history read, and steps as its word says (§6.1): a `state` when the run changed its value, a `stream` at every run, and a `channel` at every run, a `channel` being a `stream` whose every clock carries every arrival, as below. A run reads each cell at its current value. With a clock, written after `on`, the cell runs only when a listed cell steps, whatever that cell's word, and reads every other cell as a parameter at that run; a single clock that is an atom (§7.5) is written bare, and anything else is parenthesized; an entry that is an expression is the anonymous `state` it stands for (below). A listed entry that never steps, a compile-time value among them, is an error, and so is a derived cell with no clock that reads no cell but its own history, since nothing runs it. The cells of one cycle of reads, present or history reads, directly or through other cells, run together: an unclocked member runs at every step at which a cell read by any member of the cycle steps, a clocked member's clocks among them, and a clocked member runs at its clocks alone; so a cycle runs the same whatever its order, and a member ordered before another whose step it reads through history runs at that step. An unclocked cycle that reads no cell outside itself is the same error as a cell that reads only its own history. A cell's own word converts at its boundary, so a `state` reading a stream turns arrivals into values and a `stream` reading a state turns changes into arrivals, and no conversion is written.

```sym
state a(1)
stream b(2)
state c1 = a + b
state c2 on b = a + b
state c3 on a = a + b
state c4 on (a, b) = a + b
```

`c1` runs at `a`'s changes and `b`'s arrivals; `c2` at `b`'s arrivals only, reading `a` as a parameter; `c3` at `a`'s changes only; `c4` is `c1` written out.

At load, a derived cell without an init is evaluated once and that value is its first. A derived cell with an init holds the init at load and runs its rule from the first step of what it reads; the init is a compile-time value that fits the cell's type, which its rule gives (§4.4), and a start taken from another cell is written in the rule.

`x.past(k)` reads a value from the history of the cell `x`, and `x.past(k, fallback)` the same with a fallback; `k` is a positive integer known at compile time. A cell's history is the values its steps replaced, the newest first, behind its current value, and a cell that steps moves the value it replaced into it at that moment (§12.2). For a reader that lies on no cycle of reads with `x`, `x.past(1)` is the value before `x`'s current one, the value `x`'s newest step replaced, and `x.past(k)` the value its k-th newest step replaced, whether or not `x` steps at this step; such a reader runs after `x` (§12.2), so at a step at which `x` steps it reads the value `x` held as the step began, and at a step at which `x` does not step it reads the value before `x`'s current one. In `x`'s own rule, and in the rule of a cell that lies with `x` on one cycle of reads, present or history reads, directly or through other cells, `x.past(k)` counts from the value being computed: `x.past(1)` is the value `x` held as the step began and `x.past(k)` the value replaced by the (k − 1)-th newest of the steps `x` took before this one, so the order in which the cells of such a cycle run does not matter. Where the history holds no such value, `x.past(k)` yields the value `x` held at load and `x.past(k, fallback)` the fallback: for a reader on no cycle with `x`, while `x` has stepped fewer than k times; in `x`'s own rule or on a cycle with `x`, for `x.past(1)` only at load while `x` has no value yet, and for a deeper read while `x` had stepped fewer than k − 1 times before this step. Every cell has a history, as deep as the deepest `past` read of it in the program, and a history read stands wherever an expression reads a cell. A history read never reads a value computed at the current step, so a cell that reads its own history, or cells that read one another's, form no cycle; a cell that reads its own present value does form one (§6.2). A derived cell whose rule reads its own past, directly or through other cells' pasts, has a value at load when it or a cell on that cycle has an init, or a read on the cycle a fallback; with neither the cycle is an error whose help names the init and points at the earliest history read on the cycle in top-level order (§2.1). At load the derived cells are evaluated in dependency order (§11), and in such a cycle the history read that carries a fallback, lies in the rule of a cell with an init or reads a cell with an init is the one left out of the order, so its reader comes first, the earliest such read in top-level order (§2.1) where there are several; a reload orders a cycle with a cell that is not kept the same way (§13.5). A compile-time value never steps, so on a parameter bound to one `x.past(k)` is the value itself and `x.stepped` is false. `x is not x.past(1)` tells whether `x`'s newest step changed its value, not whether `x` stepped at this step; that is `x.stepped`, a `bool` true at a step at which `x` stepped, read by the cells that run at that step, and false at every other step, at load and in a reload's evaluation (§13.5), so a cell with several clocks tells which of them fired, as `hits` does below. The comparison that decides a `state`'s step is §12.2's, over bits, so a step from `-0.0` to `0.0` is one that `is` does not see, and a canonical NaN (§12.5) written over a NaN is no step.

```sym
state gate(false)
state level(0.0)
state note(60)

state held(0.0) on gate = if gate then level else held.past(1)
state previous = note.past(1, -1)
state recent([]) = (recent.past(1) + [note])[^8..]
```

`held` runs at `gate`'s steps only: when `gate` turns true it takes `level` as it is then, and when `gate` turns false it keeps what it held, because `level` is no clock of it. `previous` is the note before the current one, and −1 until `note` has stepped once. `recent` holds the last eight notes, the oldest falling off, starting from its init; `(recent.past(1) + [note])[..8]` keeps the first eight instead.

```sym
channel hit(Hit {})
state bar(0)

state hits(0) on (hit, bar) = if bar.stepped then 0 else hits.past(1) + 1
```

`hits` counts the arrivals of `hit` since load or since `bar` last changed: it holds 0 at load, runs at every arrival of `hit` and at every change of `bar`, reads `hit` only as a clock, and `bar.stepped` tells it which clock fired. Its init is its start; without one, `hits.past(1, -1) + 1` reads the fallback once, at load, and starts the same.

A derived `channel` is a derived stream the compiler checks: each of its clocks, or each cell it reads when it has none, is a `channel`, host-fed or derived, or a cell that carries no arrivals, a `state` with no `stream` or `channel` upstream of it. A `stream`, or a `state` that reads a stream or a channel, among them is an error naming that cell, whose help says to write `stream` or to clock on the channel alone. So a `channel` forgot no arrival between the host and its reader, which is what the word promises on a parameter (§6.4) and an attribute (§10.1).

- Derived cells are declared at top level and in pattern bodies (§6.4).
- An expression that reads a cell and stands where a cell is bound, an attribute value (§7.6), a gate (§9.1), a wire's choice (§8.2), a clock entry, or the argument of a parameter marked `cell` or `state` (§6.3, §6.4), is an anonymous derived `state`: it runs when what it reads steps and steps when its value changes, so equal consecutive arrivals of a `stream` or a `channel` it reads collapse into one change. The argument of a parameter without a marker is bound as the expression itself, part of the rules that read it and no cell of its own (§6.3, §6.4). A cell of any word stands in those positions by its name (§11.1), a path or a qualified name among them, as itself, with no anonymous `state` around it; an expression over a `stream` or a `channel` that must keep every arrival is declared as a derived `stream` or `channel` and bound by its name, and a marker or an attribute that asks for a `stream` or a `channel` refuses the anonymous `state` with that help (§6.4, §10.2).
- A derived cell is typed as §4.4 says; its own history reads count as their fallbacks, or as its init, for its kind and are no uses of it. Each group of cells whose reads reach one another, present or history reads, settles its cells' kinds and widths together: each history read of a cell of the group first counts as its fallback or init, or as nothing when it has neither, then takes that cell's kind, and this repeats until no cell's kind or width changes; a cell of the group whose kind or array shape is still open then takes its init's, and the reads of it take that in turn. A derived cell may hold a string (§3.3), a record (§3.8), a sum (§3.9), a map (§3.10) or a document (§3.11), never a node reference.
- Whether derived cells hold fixed arrays is settled together over each group of cells whose reads reach one another, present or history reads, a cell alone being a group of one. Every cell of the group is first taken to hold a fixed array, each history read of it yielding a fixed array of that read's fallback's or the cell's init's length, or of the cell's own length when it has neither; a cell whose rule then yields no fixed array, or a fixed array of a length other than that of the fallbacks and the init of the reads of it, holds a variable array, and each history read of it then yields a variable array; this repeats until no cell changes, and the cells left hold fixed arrays. A history read of a cell outside the group has that cell's type. A derived cell that holds a variable array has the least capacity for which its rule's capacity is at most its own, each history read of a cell having that cell's capacity or its fallback's, whichever is larger; the cells of each group take their capacities together. A cell with no such capacity is an error, whose diagnostic shows the slice that bounds it.
- A derived cell that holds a string takes its capacity as one that holds a variable array does, in characters; one that holds a document states its budget (§3.11).

```sym-error
state note(60)

state recent = recent.past(1, []) + [note]
```

This is an error: `recent` grows by one element at every step, so no capacity holds it; `[^8..]` keeps the newest eight.

### 6.7 Lambdas and function references

```ebnf
Lambda       ::= Name ':' Expr | '(' LambdaParams? ')' Result? ':' Expr
LambdaParams ::= LambdaParam (',' LambdaParam)*
LambdaParam  ::= Name (':' Type)?
```

A lambda is a function written in place: `x: x * 2`, `(a, b): a + b`, `(x: i32) -> bool: x > 60`. A single parameter stands without parentheses only when it has no annotation, and each parameter is one name; a lambda never destructures its arguments.

A function reference is a compile-time value that names one declaration: a source function, a native function, a `math` function, or a lambda. It is that declaration, not a copy and not its name: its parameters with their annotations, its result, its bracket parameters, its body, and for a lambda the scope it is written in. Two references from one declaration are one function. A function's name and a lambda, written where a function reference stands, are function references.

- A function reference stands in exactly these positions: the whole value of a compile-time `let`, top-level or local (§6.2, §6.3, §6.4); an argument where the receiving parameter has a function type or is called in the body; and the callee of a call (§5.7). Anywhere else, as an operand, an array element, an `if` or `match` arm, an attribute value, a cell's init or rule, a record field, an interpolation or an `is` operand, it is an error whose summary says a function is not a value.
- A use substitutes the declaration. With `let baz = bar`, `baz(1.0, inc)` is `bar(1.0, inc)`, inlined and checked at that call as §6.3 says, `bar`'s annotated parameters fixed and its unannotated ones typed per call, and `count(held, baz)` is `count(held, bar)`. A recursive or a native function is called as §6.3 and §10.1 say, and a call through a reference is a call of the function for §6.3's recursion rule: `fn bar(n): if n < 1 then 0 else baz(n - 1)` with `let baz = bar` makes `bar` recursive, evaluated at compile time.
- Where the body of the function that receives a reference calls its parameter, the call inlines the referenced body with its parameters bound to the call's arguments, as §6.3 inlines a call. Where the receiving parameter has a function type, the type flows into the function's parameters and result as their expected types (§4.2), and an annotation the function carries agrees with them or is an error; where it has none, the function's parameters are typed by those calls' arguments as §6.3 types a call. A native function that receives a reference calls it as §10.1 says. `each(xs, abs)` passes `math`'s `abs`, and `sort_by(notes, before)` with `fn before(a, b): a.pitch < b.pitch` passes `before`.
- A lambda is a lexical closure with zero storage: it reads the names in scope where it is written and holds no value of its own. A `let` holding a lambda holds the lambda expression. A use inlines its body with its parameters bound to the call's arguments, and the body reads the names in scope where the lambda is written, at the `let`, never at the use.
- A function reference has no type of its own; a use types it as a use types an unannotated function (§6.3). A `let` holding one may carry a function type, `let inc: (f64) -> f64 = x: x + 1`, which fixes the lambda's parameter and result types (§4.5); on a `let` holding a function, the annotation agrees with the function's own annotations or is an error, and fixes its unannotated parameters and result for uses through that `let`.
- A `let` holding a function reference is a compile-time value whatever the body reads, because the body runs only at a use; a cell read in the body is read by that use, as an argument's read is (§6.3). The `let` is never a cell and never reaches the Program. A `let` whose lambda names the `let` itself is §6.2's cycle error.
- A function type `(T1, T2) -> R` (§3.2) is the type of a function that takes parameters of the types `T1` and `T2` and yields an `R`. Its parameter and result types are value types, never function types. It stands only as the type of a function's or a native function's parameter and of a compile-time `let` holding a function reference, and on any other `let`, a cell, an attribute, a record field or an array element it is an error.
- A parameter of a function type may carry a name, `(x: f64) -> f64`. The name labels the position for the reader and lets the body call the parameter by it, `f(x=seed)`, as named arguments name a source function's parameters (§5.7); the referenced function's own parameter names are its own and need not match.
- A lambda's annotations fix its parameters' and its result's types as a function's do (§4.5).
- The number of parameters a lambda takes comes from the receiving parameter's function type, or from the calls of that parameter in the body. A lambda or a function that takes another number, or calls of one parameter that pass different numbers of arguments, are an error.
- A parameter bound to a function reference, written as the argument of a call in the body, passes the reference on to that call.

```sym
fn count(xs, keep: (i32) -> bool):
    for x in xs into n from 0: if keep(x) then n + 1 else n

state held([]): i32[..10]
state threshold(60)

state high = count(held, p: p > 60)
state low = count(held, (p: i32) -> bool: p < 48)
state above = count(held, p: p > threshold)

fn is_high(p): p > 60

state named = count(held, is_high)

fn count_high(xs):
    let keep = p: p > 60
    count(xs, keep)

state local = count_high(held)
```

`above` reads `threshold` where its lambda is written, so it follows the cell. `is_high` takes `p` as an `i32` and yields a `bool` from `count`'s function type, so `named` equals `high`, and so does `local`, whose local `let` holds the same lambda.

A compile-time `let` holds a function reference, and a function type may name its parameters:

```sym
fn bar(seed, f: (x: f64) -> f64): f(x=seed) * 2

let inc = x: x + 1
let twice = bar
let a = bar(1.0, inc)
let b = twice(1.0, inc)
let c = inc(2.0)
```

`a` and `b` are 4.0, and `c` is 3.0. `bar` calls `f` by the name `x` its function type gives, a name `inc`'s own parameter need not share; `twice` is `bar`.

```sym
let g = 2
let dbl = x: x * g

fn f(x):
    let g = 3
    dbl(x)

let ten = f(5)
```

`ten` is 10: `dbl`'s body reads the `g` in scope where the lambda is written, never the local `g` of `f`.

```sym-error
state g(x: x + 1)
```

This is an error: a function reference stands only in the positions above, and a cell holds no function.

```sym-error
fn bar(n): n + 1

let baz = bar
let same = baz is bar

Clip c:
    Note when same
```

This is an error: `baz` and `bar` are function references, and a function is not a value, so it is no operand of `is`.

```sym-error
let f = x: f(x)

Clip c:
    Note/(f(60))
```

This is an error: `f` reaches itself through the lambda it holds (§6.2); recursion is written with `fn`.

```sym-error
fn apply(f: (i32) -> i32, x: i32) -> i32: f(x)

Clip c:
    Note/(apply((a, b): a + b, 60))
```

This is an error: `f` takes one argument, and the lambda takes two.

### 6.8 Generics

```ebnf
GenericParams ::= '[' GenericParam (',' GenericParam)* ']'
GenericParam  ::= 'const'? Name (':' Type)?
TypeArgs      ::= '[' TypeArg (',' TypeArg)* ']'
TypeArg       ::= Type | Expr
```

A function, a pattern, a `type` declaration and a native function's signature (§10.1) may take bracket parameters after their name: `fn name[T, const N](…)`, `pattern Name[K, const N](…)`, `type Name[A, const N] = …`. A bracket name without `const` is a type parameter, standing for a type. A bracket name with `const` is a value parameter, a compile-time integer that stands in sizes (§3.2) and wherever a compile-time `let` or a parameter bound to a compile-time argument may stand: arithmetic, comparisons, logic, `if` and `when`, indices, range bounds, interpolation, array literals, the default of a parameter after it, calls and conversions, with every operation they allow, computed exactly as §4.6 computes compile-time integers; `/` on it yields a float (§5.2). A value parameter is an integer because it is bound only from the sizes in parameter types, and a size is an integer. A bracket name is visible throughout its declaration.

- A call to a function or a native function, and a use of a pattern, binds every bracket parameter from its arguments where the bracket parameter stands bare in a parameter's type, as a size, as a type or as a type argument, and writes no bracket arguments. A size that is an expression in a parameter's type is checked against the argument's size once the bracket parameters it reads are bound, as a second mention of one parameter is. A bracket parameter that stands bare in none of its declaration's parameter types is an error whose help says to write the bare name in a parameter's type, since binding reads a size from an argument and solves no equation.
- Two mentions of one bracket parameter unify (§4.2), and arguments that bind it differently are an error at the call or use.
- A type parameter bound from a flexible argument (§4.1) takes the defaults of §4.5, and a value parameter without a constraint is a flexible integer.
- A function's or a pattern's type parameter may also bind to a node kind or a wire kind, from an argument that is a reference (§3.5); for a pattern, §10.2's checks run on each use's expansion against the kind it binds, and a constraint written on such a parameter is an error, those checks being its check.
- A bracket parameter's annotation is its constraint. On a type parameter it is a type the parameter must bind to, a sum listing the types it may bind to, one of them at each call or use, or an alias naming that set; on a value parameter it is the integer type of its value. A bracket parameter without a constraint binds to whatever its arguments give, and the body is checked per call or use with that binding.
- In a type, the first bracket list after a generic type's name is its arguments, one for each bracket parameter in order and read by the parameter it fills: a type for a type parameter, and for a value parameter a compile-time integer expression, as a size is (§3.2), the enclosing declaration's value parameters among what it reads. Later bracket lists are dimensions (§3.2), and after any other name every bracket list is a dimension: `Stack[i32, 8][4]` holds four stacks, and `Chord[4]` four chords.
- A record literal (§3.8) names its generic type with its arguments, `Stack[i32, 8] {}`, or without them, `Pair {left: 1i32, right: true}`, and then binds the type's parameters from its fields as a call binds them from its arguments: two fields that bind one parameter differently are an error naming both, and a parameter that no field binds is an error whose help says to write the type's arguments. Anywhere else, a generic type named without its arguments is an error whose help names its parameters.
- An instantiated record's shape is its fields' shapes, so a reload keeps it as it keeps any record (§13.2, §13.5).

```sym
type Stack[T, const N]:
    items: T[..N]
    top: i32

fn push[T, const N](s: Stack[T, N], x: T) -> Stack[T, N]:
    Stack[T, N] {items: (s.items + [x])[..N], top: s.top + 1}

fn first[T, const N: i32](xs: T[N]) -> T: xs[0]

fn mid[T: f32 | f64](a: T, b: T) -> T: (a + b) / 2

state note(60): i32

let pitches: i32[3] = [60, 64, 67]
let start = Stack[i32, 8] {}
let big = Stack[i32, 4 * 2] {}
let pushed = push(start, note)
let centre = mid(0.25f32, 0.75f32)

Clip c:
    Note/(first(pitches))
```

`push` binds `T` to `i32` and `N` to 8 from `start`, and `x: T` reads `note` as that `i32`; `first` binds `N` to 3 from `pitches`; `big` is a `Stack[i32, 8]`.

```sym
fn padded[const N](xs: i32[N]) -> i32[N + 1]: xs + [0]

fn mean[const N](xs: f32[N]) -> f32: (for x in xs into total from 0.0: total + x) / N

let notes = padded([60, 64, 67])
let average = mean([0.25f32, 0.5f32, 0.75f32])
```

`padded` binds `N` to 3 and yields an `i32[4]`; in `mean`, `N` is the integer 3, and `/` yields the `f32` the result expects.

```sym-error
fn shorter[const N](xs: i32[N]) -> i32[N - 1]: xs[1..]

Clip c:
    Note/(shorter([60]).size)
```

This is an error: the call binds `N` to 1, so the size `N - 1` is 0; the diagnostic names the call and points at the type.

```sym-error
fn half[const N](xs: i32[N]) -> i32[N / 2]: xs[..N div 2]

Clip c:
    Note/(half([60, 64, 67, 72])[0])
```

This is an error: the size `N / 2` is a float; the help names `div`, as the body's `N div 2` writes it.

```sym
type Pair[A, B]:
    left: A
    right: B

let p = Pair {left: 1i32, right: true}
```

`p` is a `Pair[i32, bool]`, its parameters bound from its fields.

```sym-error
fn same[T](a: T, b: T) -> T: a

Clip c:
    Note/(same(60i32, true))
```

This is an error: `T` binds to `i32` from `a` and to `bool` from `b`.

```sym-error
fn tail[const N](xs: i32[N + 1]) -> i32[N]: xs[1..]

Clip c:
    Note/(tail([60, 64, 67])[0])
```

This is an error: `N` stands bare in no parameter's type, only in `N + 1`; the help says to write it bare, as `padded` above takes `xs: i32[N]`.

```sym-error
type Pair[A, B]:
    left: A
    right: B

let p = Pair {left: 1i32}

Clip c:
    Note/(p.left)
```

This is an error: no field of the literal binds `B`; the help says to write the type's arguments, as `Pair[i32, bool] {left: 1i32}`.

## 7. Placements

### 7.1 Form

A placement creates an instance of a node kind or uses a pattern. Its elements appear in a fixed order.

```ebnf
TopPlacement ::= Kind Flags? 'as'? Name DefaultArg? Gate? Attrs? (NEWLINE | Body)
Placement    ::= Kind Flags? ('as' Name)? DefaultArg? Gate? Attrs?
Kind         ::= Name | QualifiedName
Body         ::= ':' (NEWLINE INDENT BodyItem+ DEDENT | InlineChildren NEWLINE)
BodyItem     ::= Line | WirePlacement | ForBlock | WhenBlock
Line         ::= Placement (NEWLINE | Body) | '(' Placement ')' NEWLINE | Bundle NEWLINE
               | SelfDelimited SelfDelimited+ NEWLINE
```

`DefaultArg` is §7.5's, `Gate` §9.1's and `Attrs` §7.6's. A line whose `Kind` names a wire kind of the vocabulary, or a pattern that stands in for a wire (§6.4), is a wire placement (§8.1), and a wire at top level is an error. Whitespace around `/` is free. In a body, `Note hook/60` reads as two placements, and its diagnostic suggests `as`.

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
Atom       ::= '-'? NumberLit | BoolLit | StringLit | (Name | QualifiedName) ('.' Name)* | 'to' ('.' Name)+
```

`/expr` sets the kind's default attribute, which the vocabulary designates (§10.1): `Note/60` sets the pitch. Unparenthesized, the value is a single atom: a literal, optionally negative, a name, a dotted path, a qualified variant or `to.name`. Anything else is parenthesized: `Note/(base + 7)`. `/expr` on a kind without a default attribute is an error.

### 7.6 Attribute clause

```ebnf
Attrs     ::= '|' AttrEntry+
AttrEntry ::= Name '=' Expr | Name '=>' (Name | QualifiedName) ('.' Name)* | Name | '!' Name
```

A single `|` opens the attribute clause, its first entry follows the `|` on the same line, and entries are separated by whitespace.

- `name=expr` sets an attribute.
- `name => cell` binds an attribute that takes a host-fed cell (§10.1) to one: `cell` names a host-fed cell of the kind the attribute takes, by its name, by the path of a named pattern use's cell (`low.gain`, §6.4), by a library's qualified name (§15.3) or through a pattern parameter bound to one, and the framework writes the cell through the attribute (§12.1). `=` on such an attribute, and `=>` on an attribute that takes a value or a constant, are errors.
- `name` sets a `bool` attribute to `true`; `!name` sets it to `false`.
- Setting an attribute twice, by any combination of entries, flags and `/expr`, is an error.
- Setting an attribute the kind does not declare is an error, and so is leaving unset an attribute the vocabulary gives no default (§10.1).

The clause continues on following lines that start at its first entry's column (§2.2), without another `|`:

```sym
state cutoff(0.5)
state wobble = cutoff * 0.8

Clip melody:
    Note/62 | velocity=0.7
              brightness=wobble
              accent
```

An attribute bound to a constant is static. An attribute bound to a cell steps with it, and one bound to an expression that reads cells is bound to the anonymous `state` the expression is (§6.6), reported at its changes (§12.3).

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
BundleFor ::= LoopHead ':' Member+
```

A bundle is one entry of its parent's sequence whose members are simultaneous: a chord.

- A bundle has at least one member after expansion. Members are node placements, which may be gated and named, uses of patterns that expand only to node placements, whose placements become members, and a `children` parameter's name, whose share becomes members (§6.4). Wires and bundles are not members, and a member has no body, also when a pattern supplies it.
- Members are children of the bundle's parent.
- Layout is suspended inside the brackets, so members are self-delimited (§7.7). A `for` inside a bundle takes the loop head of §7.9, one name or two, and extends to the closing bracket.

```sym
Clip pad:
    [Note/60 Note/64 Note/67]
    [for i in 0..3: Note/(62 + i * 4)]
    [for i, p in [60, 64, 67]: (Note/p | velocity=1.0 - i * 0.2)]
```

### 7.9 Compile-time `for`

```ebnf
ForBlock ::= LoopHead ':' (NEWLINE INDENT BodyItem+ DEDENT | InlineChildren NEWLINE)
LoopHead ::= 'for' Name (':' Type)? (',' Name (':' Type)?)? 'in' Iterable
Iterable ::= Expr '..' Expr | Expr
```

A placement `for` repeats its body once for each element of its iterable. The iterable of a placement `for` and of a `for` expression (§5.10) is one of these, and nothing else:

- an array, fixed or variable, compile-time or runtime, element by element in order;
- a map (§3.10), entry by entry in insertion order;
- a range `a..b`, which counts from `a` up to but excluding `b` and is empty when `b` is not above `a`.

A bound of a range or of a slice (§3.7) has a minimum and a maximum read from its form: a literal and a compile-time value are their own minimum and maximum; `a.size` of an array, a string or a map lies between 0 and `a`'s capacity (§5.7), and a document's `size` has no maximum by its form; `min(x, y)` has the smaller of its operands' minimums and the smaller of their maximums, and `max(x, y)` the larger of each; `x + y` has the sum of its operands' minimums and the sum of their maximums, and `x - y` has `x`'s minimum less `y`'s maximum and `x`'s maximum less `y`'s minimum. An operand that reads a cell and has none on a side by these forms is unbounded on that side, and these rules carry an unbounded side as they carry a number: `min(n, 16)` with a cell `n` has the maximum 16 and no minimum, and `16 - n` has neither. A range's iteration count is at most its upper bound's maximum less its lower bound's minimum. An upper bound of a range that reads a cell and has no maximum, or a lower bound that reads a cell and has no minimum, is an error.

A loop head binds one name or two. With one name, it is the element over an array, the key over a map and the value over a range. With two names, the first is the element's position and the second the element: over an array the index and the element, the index a flexible integer as a range's value is (§4.6), and over a map the key and the value. A range binds one name, and two are an error. An annotation fixes its name's type (§4.5).

A placement `for`'s iterable is a compile-time value, and its loop names are compile-time values; an iterable there that reads a cell is an error. Placements in a `for` body are anonymous; `as` there is an error.

```sym
state steps([true, false, false, true, false, false, true, false])
state pitches([36, 36, 43, 36, 41, 36, 43, 48])

Clip seq:
    for i in 0..8:
        Note/(pitches[i]) when steps[i]

let chord = [60, 64, 67]

Clip strum:
    for i, p in chord:
        Note/p | velocity=1.0 - i * 0.2
```

In `strum`, `i` is each note's position and `p` its pitch.

```sym
state held([]): i32[..10]
state counts({}): {str[..16]: i32}[..8]

state rises = for i in 1..held.size into n from 0: if held[i] > held[i - 1] then n + 1 else n
state total = for k, v in counts into sum from 0: sum + v
state names = for k in counts into all from "": all + k
```

`rises` runs at most 9 times, the maximum 10 of `held.size` less the minimum 1. One name over `counts` binds its key, so `names` joins the keys and holds at most 128 characters, 16 for each of at most 8 iterations.

```sym-error
Clip c:
    for i, k in 0..4:
        Note/(60 + k)
```

This is an error: a range binds one name.

```sym-error
state n(4)

state total = for i in 0..n into sum from 0: sum + i

Clip c:
    Note/total
```

This is an error: the bound `n` reads a cell, and its form gives it no maximum; `0..min(n, 16)` runs at most 16 times.

## 8. Wires

### 8.1 Wire placements

```ebnf
WirePlacement ::= Kind Flags? ('as' Name)? DefaultArg? Gate? Attrs? (':' Destination)?
```

A wire connects its source to its destination and carries attributes. Its source is the instance whose body it is written in. A wire may appear anywhere in that body's sequence, and its position is kept (§7.3); what the position means is the consumer's. A wire placement omits its destination only as the single placement of a pattern that stands in for it (§6.4).

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

- A destination is always one of a fixed set of candidates written in the source, directly or through compile-time values such as `let`s, `for` variables and pattern parameters. A destination that reads cells chooses among them at runtime. Candidates are listed left to right as written, those of nested choices included, and one index, an anonymous `state` (§6.6), chooses among them, clamped as an array index is (§5.5). A candidate written twice counts once in the checks of §8.4 and §10.2.
- Every candidate is checked against the wire kind's endpoints (§10.2).
- A wire never resolves to nothing. A gate disconnects it: `Plays when looping: verse`.
- A destination names an instance placed elsewhere. It is never an inline placement or a pattern use written in the destination, and it contains no `to`.

```sym
state section(0)
state fill_on(false)

Clip verse:
    Note/65 Note/67
    Plays: [chorus, bridge, outro][section]

Clip chorus:
    Note/72
    Plays: if fill_on then fill else verse
```

### 8.3 `to`

Inside a wire placement's `/expr`, gate and attribute clause, `to` starts a path at the wire's destination (§5.6): `to.name` reads attribute `name` of the destination, and `to.hook.velocity` reads attribute `velocity` of its child named `hook`. The path steps through named children, named wires and the names inside named pattern uses, and ends at an attribute.

- With a fixed destination, `to.path` reads that instance's path.
- With a destination chosen at runtime, every candidate has the path, ending at an attribute of the same type and unit family, and `to.path` makes the same choice among the candidates' attributes, in the gate as in the attribute clause.
- `to` alone, a `to` path that ends at an instance or a wire, and `to` written outside a wire placement are errors; a function body is outside every wire placement.

```sym
state section(0)

Clip verse:
    Plays | gain=to.hook.velocity: [chorus, bridge][section]

Clip chorus:
    Note as hook /72 | velocity=0.9

Clip bridge:
    Note as hook /67 | velocity=0.6
```

### 8.4 Cycles

No cycle of wires contains a wire of a kind the vocabulary does not mark cyclic. Every candidate of a destination chosen at runtime counts as an edge in this check.

## 9. Gates

### 9.1 Inline `when`

```ebnf
Gate ::= 'when' Expr
```

`when expr` on a node, a wire or a pattern use gates it: the instance is active only while the expression, a `bool` and an anonymous `state` when it reads cells (§6.6), is true.

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
| Attribute | name; type; default; unit family and default unit; flag character; whether it is its kind's default attribute; the binding it takes, by §6.4's marker words: any value, `const`, `cell`, `state`, `stream`, `channel`, or `out` with a kind or alone, a host-fed cell the framework writes, with the least capacity its writer needs for a `stream` or a `channel` |
| Enum | name; variants |
| Unit family | name; base unit; units and their scales |
| Identity | the vocabulary's name and its version (§15.8) |
| Prelude | functions, patterns, record types and named values such as note names, in SymphonyScript source, and native functions, implemented in Rust |

A count is exactly *n*, at least *n*, at most *n*, between *n* and *m*, or any number. A child kind absent from a kind's accepted list is not accepted. A wire kind connects only the source and destination kinds it lists; a node kind bounds how many wires of each kind it takes, and takes any number where it states no count. Each kind has at most one default attribute, and its flag characters are distinct. An attribute takes any value unless it states otherwise. What it takes is written with §6.4's marker words and accepts the same sets: `const` a compile-time value, `cell`, `state`, `stream` or `channel` a cell of that kind or a compile-time value, as the marker widens, and `out` with a kind or alone a host-fed cell the framework writes. An attribute that takes a host-fed cell has no default, and a program binds it with `=>` (§7.6, §10.2). One that takes a `stream` or a `channel` states the least pending capacity its writer needs, and a cell with a smaller capacity is refused at the binding (§6.1). An attribute that takes a constant is bound to a compile-time value. A record attribute names a record type the prelude declares, or takes a record of any type, whose type the bound value supplies; a string attribute states a capacity (§3.3) or takes compile-time strings only. A map attribute names its key and value types and its capacity (§3.10); a document attribute names its budget, or takes any document, whose budget the bound value supplies (§3.11). A sum is never an attribute's type (§3.9).

A vocabulary is invalid, and no program compiles against it, when:

- a name it declares is not an ASCII identifier (§2.4), or is a variant of one of its enums, or is a reserved word (§2.5), or, for anything but an attribute, is a keyword (§2.5), an attribute's name standing only in member positions;
- two of its node kinds, wire kinds, enums and unit families share a name, a node kind or wire kind declares two attributes of one name, or an enum declares two variants of one name;
- a kind has two default attributes, two of its attributes share a flag character, or a flag character is outside the set of §7.4 or flags an attribute that is not `bool`;
- a count's minimum exceeds its maximum;
- a declaration refers to a kind of another vocabulary;
- an enum has no variants or more than 65,536, or an enum attribute's default is not one of its variants;
- an attribute that takes a host-fed cell has a default or states a capacity of 0, a record attribute names a type the prelude does not declare, or a string, map or document attribute states a capacity or budget of 0;
- an array attribute, a native parameter or a native result has a length or capacity of 0 at any level, or of 2³¹ or more (§3.2);
- a unit's scale is not a positive finite number, a unit name repeats, within a family or across families, an attribute's default unit is not in its family, or a unit name begins with `_`, or with `x` and a hexadecimal digit, `b` and a binary digit, `o` and an octal digit, or `e` or `E` and a digit or sign (§2.6);
- an integer attribute belongs to a family with a unit whose scale is not an integer;
- a native function's signature holds a type outside §3, or a variable-array result whose capacity it does not state;
- the prelude fails to parse or to resolve its names;
- its version is not a version (§15.7).

The prelude's source is ordinary source compiled with the program, and the prelude is a library, the framework's, that reaches programs as §15.6 says. It holds cell, `let`, `fn`, `pattern` and `type` declarations only, beside its package file (§15.2), and its declarations and native functions share its own namespace (§6.5). A prelude cell is the framework's: the framework states its type, its capacity and its init, as `state path(""): str[..2048]`. A project never changes a prelude cell; a framework that leaves a value to the project takes it as an `out` parameter (§6.4). It is parsed and resolved once per vocabulary; its declarations are type-checked where programs use them, and an error found there is a program error located in the prelude. A program's declaration with a prelude name shadows it in the program's package (§15.6). Prelude bodies see only prelude declarations, the vocabulary's names and the `math` package, so source declarations never change them.

A native function is a prelude function the vocabulary implements in Rust. Its declaration gives its name and its signature: the bracket parameters it takes (§6.8), and the type of each parameter and of its result, each a type of §3, where an array parameter may leave its capacity open, written `i32[..]`, and a variable-array result states its capacity as a number or as the capacity of one array parameter; the array result of a native function that runs only at compile time is a fixed array of the length it yields. Its parameters and result take plain values (§3.6). A parameter may be a function type (§6.7), whose argument is a function reference. A call reads each argument in its parameter's type as a fixed-type position does (§4.2, §4.3). A call whose arguments are all compile-time values, a function reference among them when its body reads no cell, is evaluated by the compiler, and any other call runs at runtime; a string parameter is written `str[..N]` or, open, `str[..]`, and a string result states its capacity as an array result does (§3.3). A native function calls a function reference it receives with argument values of the function type's parameter types, and each call runs the referenced body with its parameters bound to them and yields its value: the compiler runs it when it evaluates the call, and the runtime when it runs the call, the native function writing its result into the buffer the runtime provides. A lambda whose body calls a native function with a lambda of its own nests that call in the outer one; the depth of this nesting is fixed by the expressions and counts toward the nesting limit (§6.3), which the compiler checks, so a call at runtime never passes it. A native function is pure and total: it yields the same value for the same arguments on every target, allocates nothing and never fails. The vocabulary's author upholds this, a native function that takes a lambda included, so how many times it calls the lambda is its author's; the compiler and the runtime rely on it.

### 10.2 Checks against the vocabulary

The compiler checks the expanded program (§6.4) against the vocabulary:

- Every kind, wire kind, attribute, enum variant and unit exists in the vocabulary.
- Every top-level instance's kind may stand at top level, and the number of each such kind is within its count.
- Every child's kind is accepted by its parent, and each parent's children of each kind are within their counts.
- Every wire's source and every candidate destination are kinds its wire kind connects, and each instance's incoming and outgoing wires of each kind are within its counts.
- A wire whose destination is chosen at runtime counts toward the maximum of every candidate and toward the minimum of none.
- Gated placements and wires count as ungated ones do, because a gate varies activation and never structure (§9.2): `when a: Output` followed by `else: Output` holds two `Output`s, and a gated wire counts toward its destination's minimum.
- Every attribute value has the attribute's type and unit family.
- Every attribute that takes a host-fed cell is bound with `=>` to a host-fed cell of the kind it names, directly or through a pattern parameter bound to one, whose capacity is at least the one the attribute states; one cell may be bound to any number of attributes, and a framework that writes per instance sees the shared binding in the Program (§11) and decides for itself. Every attribute that takes a cell of a kind without `out` is bound to what that kind's marker accepts (§6.4), and every attribute that takes a constant to a compile-time value. A record attribute is bound to a value of its type, or, where it takes any record type, to a record.
- Wires form cycles only as §8.4 allows.

```sym
channel hit(Hit {})

Pad kick | hit => hit
```

```sym-error
Pad kick | hit=Hit {}
```

This is an error: `hit` takes a host-fed cell, bound with `=>`, and `Hit {}` is a constant.

A top-level instance that no entry-kind instance reaches, through children and through wires to any candidate, draws a warning. A program with no entry-kind instance draws one warning that says so instead, and a vocabulary with no entry kind draws none.

## 11. The Program

Compilation produces a Program. It contains:

- the identity of the vocabulary it was compiled against;
- every instance: its kind, path, attributes and gate, and its sequence of children, bundles and wires in source order;
- every wire: its kind, path, source, destination candidates and choice, attributes and gate;
- every record type a cell or an attribute holds, with its fields, and every compile-time document (§3.11);
- every host-fed cell: its word, name, type, unit family, init and, for a `stream` or a `channel`, its capacity, in top-level order, a pattern's cells at their use's position (§6.4);
- every derived cell that something reads, with its word, in dependency order;
- every function body the compiler stores once, a shared body (§6.3), with the cells that invoke it through call nodes carrying their arguments;
- the top-level instances in top-level order (§2.1).

An item's gate combines its own `when` with the conditions of the `when` block arms around it (§9.3). Instances are listed in preorder: each top-level instance in top-level order, each followed by the children and bundle members of its sequence in sequence order, each of those followed in turn by its own. Wires are listed in the order that walk meets them.

Every attribute of every instance and wire is present, with vocabulary defaults filled in. Values are in their family's base unit. Patterns are expanded, a function is inlined or stored once as a shared body, and no other source construct remains.

### 11.1 Paths

Every instance and wire has a path, with segments joined by `.`:

- A top-level instance's path is its name.
- A named child or wire extends its parent's path with its name: `verse.hook`.
- An anonymous child or wire extends it with its kind and its position among the parent's anonymous entries of that kind, counted from 0 in sequence order, bundle members included: `verse.Note:2`.
- A named pattern use contributes its name, and an anonymous use contributes the pattern's name and its position among the parent's anonymous uses of that pattern: `verse.Arp:0.Note:2`. The items a use expands to take their paths under its segment and do not count among the parent's entries, so in `Clip verse: Note/40 Arp/48 Note/41` the last note is `verse.Note:1`.
- A use that stands in for a single placement (§6.4) gives that placement the use's path.
- A cell declared in a pattern has the path of its use followed by its name: `verse.low.gain`.
- An anonymous `state` (§6.6) is named by what binds it: for an attribute value, the item's path and the attribute's name, with the element's index or the field's path appended where the attribute is bound to an array or a record of bindings (§4.4); for a gate, one binding per item (§11), the item's path and `when`; for a wire's choice, the wire's path and `to`; for a clock entry, the cell's path and the entry's position; for an argument, the path of the item or cell whose body or rule holds the call or use, the call's or use's position in it and the parameter's name. A reload keeps it by that name (§13.5).
- A top-level cell of a library is named by its package's identity (§15.1), `::` and its name: `prelude::path`, `github.com/acme/drums::swing`.

## 12. Runtime semantics

### 12.1 Writes

The host writes a host-fed cell by its handle, which it takes by the cell's name (§11.1). A write stages a value until the next commit and has no other effect. One write writes a whole value, and one addressed write writes a part of it: an element of an array, a leaf field of a record, a row of a variable array at an inner level (§3.7), or an entry of a map by key. A write is one write whatever it holds, taken whole or rejected whole with its reason.

A write takes a value of the cell's kind (§4.4): `U8`, `I32` or `I64` for an integer cell, `F32` or `F64` for a float cell, `Bool` for a `bool` cell, and a variant of its enum for an enum cell. The runtime converts it to the cell's type, a float rounding and an integer saturating. A write is rejected with an error when its value is of another kind, when its element index is outside the array, when its enum value belongs to another enum or names no variant, or when a whole-array write has the wrong length. A variable array cell takes whole-array writes of any length up to its capacity, which set its live length, and single-element writes within its live length; a longer write is rejected with an error naming the capacity, and the cell keeps its value. An accepted write to a variable array marks the whole array written. A string cell takes a whole-string write of valid UTF-8 up to its capacity of characters, which sets its length; a longer or invalid write is rejected with an error naming the capacity, and the cell keeps its value. A record cell takes writes of a single leaf field, numbered in declaration order depth first, an array field's elements numbered as an array cell's are, and whole-record writes; a write to a field follows the rules of a cell of the field's type. A sum cell (§3.9) takes `None` or a value tagged with one of its alternatives, and a write whose alternative the sum lacks is rejected. An element of a variable array at an inner level (§3.7) is addressed by its row and its position in the row, and a whole-row write sets the row's live length. A map cell takes a whole-map write within its capacity, and a write of one entry by key, which adds the entry or replaces its value; a write past the capacity is rejected naming it. A document cell takes a whole-document write as text, refused past its budget or when it is not valid JSON in valid UTF-8, with the capacity or the byte position in the error (§3.11). Any other form of write is rejected. An accepted write marks the elements it writes as written by the host (§13.2), whether or not their values change. Elements are numbered row by row: element `k` of an `i32[3][2]` cell is row `k div 3`, position `k % 3`. A cell in a unit family is written in the family's base unit (§3.6). A written NaN is stored as the canonical NaN (§12.5).

What a cell keeps between commits follows its word (§6.1). A `state` keeps its last write to each element. A `stream` and a `channel` keep every accepted write since the last commit as a pending step, in arrival order, up to the cell's capacity, an addressed write making one pending step whose value is the cell's newest value, pending or committed, with the part replaced. When the capacity is full, a `stream` forgets its oldest pending write; the writes forgotten between two steps of the stream are counted on the later step, where `s.dropped` is their number (§5.7), so every forgotten write counts at exactly one step; a `channel` rejects the write with an error naming the capacity, and what becomes of the value is the host's, the runtime never waiting. Writes to different cells keep their arrival order relative to one another.

### 12.2 Commit

A commit applies the staged writes and brings every cell up to date, in steps:

1. The `state` writes come first, together, as one step: each host-fed `state` whose staged value differs from its current value steps, and an equal one is unchanged. Then each pending write of every `stream` and `channel` is one step of its cell, taken one at a time in arrival order across all cells, by one sequence number per write.
2. At each step, a derived cell without a clock runs when a cell it reads, in the present or through a history read, stepped, or when a cell read by any member of a cycle of reads it lies on stepped, and a derived cell with a clock runs when a listed cell stepped (§6.6); the runs go in dependency order, a history read ordering the reader after the cell except where that would close a cycle (§6.6), each cell at most once per step.
3. A `state` whose run yields a value equal to its previous value is unchanged, and its readers do not run because of it. A `stream` and a `channel` step at every run, and their readers run.
4. A cell that steps moves the value it replaced into its history, behind its new value; the cells after it in the order read that history, and the cells on a cycle of reads with it read its value and history as the step began (§6.6).
5. Activation (§9.3) and destination choices (§8.2) are recomputed at each step.

Two values are equal here when their live parts hold the same bits: a scalar its bits, the canonical NaN (§12.5) equal to itself; a variable array or a string its live length and then its live elements or characters; a map its entries; a sum its tag and the value of its alternative; a record and a fixed array part by part; a document its text. Unused capacity is never compared.

At the end of each step every value the runtime holds reflects the same writes, and the consumer (ARCHITECTURE §6) reads the Program's values and the step's changes then, before the next step begins; no read sees part of a step. A consumer that copies values elsewhere states what its own readers see.

A cell carries its value and its history from one commit to the next (§6.6); a derived cell's value is a function of the values and histories of the cells it reads and of its own history. Time-dependent behaviour comes from cells the host writes, such as `bar`, `beat` and `time`, combined with pure functions, histories and `rand`.

### 12.3 Changes

A commit reports what changed, in the terms consumers use:

- an instance attribute;
- a wire attribute;
- an instance's activation;
- a wire's activation;
- a wire's chosen destination.

An attribute bound to a cell is reported once per step of its binding, and the consumer reads the value whole, a variable array with its live length; an attribute whose binding is an array or a record of bindings (§4.4) is reported once per step of any of them. A `state` steps only when its value changes (§12.2), so an attribute bound to one is reported at each change, a value that returns within one commit to what it was before it being reported at each step it changed; one bound to a `stream` or a `channel` is reported at every step of it, equal values included. Activation and a chosen destination are reported at each step at which they changed. The consumer receives each step's records at the end of that step (§12.2), so a value a later step overwrites is read before it is; within one step, instance changes come first and wire changes after them, each in Program order (§11), and for one item attribute changes come in the order the vocabulary declares the attributes, then its activation, then a wire's chosen destination. Loading a Program evaluates every cell, activation and destination choice, and the first commit reports changes relative to that loaded state. A map attribute reports one change when any entry differs, and a document attribute one change when its text differs.

### 12.4 Evaluation never fails

Evaluation has no runtime errors: every operation yields a value (§5.2, §5.5, §5.7, §5.8). The only runtime errors are rejected host calls: a write §12.1 rejects, a `channel` write past its capacity among them, or a write through a handle whose cell no longer exists (§13.2).

### 12.5 Determinism

The same Program with the same sequence of writes and commits produces bit-identical values on every target. Built-in float functions use one implementation on every target, and a call in `f32` uses that implementation's `f32` function. Every NaN that a cell, an attribute binding or a folded constant holds is the canonical quiet NaN: `0x7FC00000` for `f32` and `0x7FF8000000000000` for `f64`.

`math`'s `rand(seed, n)` widens both arguments to `i64`, takes their bits as unsigned 64-bit integers, and computes in 64-bit unsigned arithmetic that wraps, where `>>` shifts right with zeros shifted in:

```text
x = (seed << 32) + n
z = x + 0x9E3779B97F4A7C15
z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9
z = (z ^ (z >> 27)) * 0x94D049BB133111EB
z = z ^ (z >> 31)
rand = (z >> 40) / 2^24
```

`rand(0, 0)` is 14819496 / 2²⁴, and `rand(1, 2)` is 11759674 / 2²⁴.

### 12.6 Memory

A Program needs a fixed amount of memory, known when it compiles. Loading it allocates every buffer the runtime uses, and nothing allocates while it runs: variable arrays, joins, slices and strings hold their capacities (§3.3, §3.7), a string 4 bytes per character, and each cell's ring holds its history depth (§6.6) plus, for a host-fed `stream` or `channel`, its capacity of pending writes (§6.1). A shared body's buffers are allocated at load as every other buffer is, one set per shared body, sized for its deepest invocation (§11). The memory is what loading allocates, the same on every target; the Program is the host's data and is not counted. The compiler reports the memory in bytes, and a program whose memory exceeds the memory limit, 1,073,741,824 bytes by default (§4.6), is an error. Beside the memory it reports a commit's most work: the runs of derived cells a commit can take, every `stream` and `channel` contributing its capacity of steps and the `state` writes one step together, each step the derived cells that run for it (§12.2).

## 13. Reload

A reload replaces a running Program with a newly compiled one.

### 13.1 Acceptance

A reload is all-or-nothing and happens between commits. A Program compiled against a different vocabulary is refused, and the running Program continues. Source that fails to compile produces no Program, so the running Program continues.

### 13.2 Host-fed cells

Each host-fed cell of the new Program takes its value by the first of these rules that applies; a scalar cell is one element:

1. A cell with no old host-fed cell of its name and word takes its init.
2. A cell whose kind changed (§4.4), or whose shape changed other than in its outermost length, takes its init. An enum cell moved to another enum changes kind, and a scalar cell that becomes an array changes shape.
3. Otherwise each element both lengths share keeps its old value, converted to the new type as a write is (§12.1), when the host has written it, and takes the new init when the host has not; elements beyond the old length take the new inits.

A variable array cell that the host has written keeps its live length, cut to its new capacity, and its live elements, converted to the new type as a write is; a change between a fixed and a variable array is a change of shape. A string cell that the host has written keeps its characters, cut to its new capacity. A record cell keeps each leaf field that exists in both Programs with its kind and shape, by the field's path, as rule 3 keeps an element. A sum cell keeps its value when the new sum holds the old value's alternative with its kind and shape, and takes its init otherwise; a change between `T` and `T?` is a change of shape. A variable array at an inner level keeps each row as a variable array is kept. A map cell keeps the entries whose key and value fit the new types, in entry order, cut to the new capacity. A document cell keeps its document when the new budget holds its text, and takes its init otherwise. Whether the host has written an element is tracked per element, and a kept element stays written. A removed cell is dropped, and a handle to it rejects writes. A handle stays valid across reloads while a host-fed cell of its name exists, and becomes valid again when a reload brings one back. Writes staged before the reload, a `stream`'s or a `channel`'s pending writes among them, stay staged against the new Program by cell name and apply at the next commit, which marks their elements written; a staged write that the new cell rejects (§12.1), one past a new capacity among them, or whose cell is gone, is dropped and listed in the report (§13.4).

### 13.3 Matching

Each instance and wire of the new Program is matched with at most one of the old, of the same kind. Matching starts with the top-level instances and descends into each matched pair's sequence. At each level:

1. Named items match by their path below their matched parent. An item whose path matches one of a different kind does not match.
2. The parent's anonymous entries, bundles among them, match by a common subsequence of their keys. An entry's key is its kind, `bundle` for a bundle, with its attribute values, a wire's fixed destination, the names of its named descendants and, for a bundle, its members' keys. An attribute's value in a key is its constant, compared as §12.2 compares values, or a mark that it reads a cell; a string compares by its text and a destination by its path. After the common prefix and suffix are matched, the subsequence is the one Myers' difference algorithm finds, preferring a removal to an addition where both cost the same. The search runs up to an edit distance of 1,000 per sequence by default, a limit the host may set otherwise (ARCHITECTURE §5); past it, the unmatched middle is one gap for step 3.
3. In each gap, between two consecutive matches of step 2 or before the first or after the last, the i-th unmatched entry of a kind on the old side pairs with the i-th unmatched entry of that kind on the new side, and they match with changed attributes.
4. A matched bundle's members match within it by steps 1 to 3.

Items under an unmatched parent, named members of an unmatched bundle among them, and every other unmatched item, are removed or added.

### 13.4 Report

A reload reports, for instances, bundles and wires, the matched pairs (old and new), additions, removals, and the parents whose sequence order changed, and it lists the staged writes it dropped (§13.2). The top level counts as a parent, and a sequence's order changed when matched items changed their relative order. It then reports the attribute, activation and destination changes of matched items in the form a commit uses (§12.3); a destination compares by path. Cells other than kept derived cells (§13.5) are recomputed from the new Program (§12.2).

### 13.5 Derived cells

A derived cell of the new Program keeps its value and its history when the old Program has a derived cell of its name, word, kind and shape, a variable array's capacity apart, its value then cut to the new capacity. Its history keeps the records of history reads whose cell keeps its name, word, origin, kind and shape, up to their new depths; a history read that is new, or deeper than before, yields its fallback, or the cell's value at load, until the cell has stepped as often as its offset. The reload goes through the derived cells, kept or not, in dependency order. A cell that is not kept takes its first value as at load (§6.6). A kept cell is evaluated once when its lowered expression differs from the old Program's (ARCHITECTURE §3) or when a cell it reads is not kept or changed its value during the reload, a host-fed cell under §13.2 or a derived cell under this rule, its value after the reload compared with its value before; its history reads read its kept history, the reload reports the change (§13.4), and the evaluation is no step, so its history is unchanged and `stepped` is false in it. A kept cell with the same expression and unchanged reads is not evaluated by the reload and steps at the next commit that steps what it reads.

## 14. Not part of SymphonyScript

SymphonyScript has none of the following:

- **Declared kinds.** Node kinds, wire kinds, enums and units come from the vocabulary (§10). A record type, an alias and a sum (§3.8, §3.9) declare a shape and no kind; there are no tuples, and no enums with data.
- **Mutation and unbounded loops.** Bindings never change, and a cell changes only by taking a step (§6.1). The loops are the placement `for`, which runs at compile time (§7.9), and the `for` expression, which runs at most as often as its iterable allows (§5.10); patterns do not recurse (§6.4), and a function never recurses at runtime (§6.3).
- **Varying structure.** Gates vary activation (§9), and destinations vary among fixed candidates (§8.2). A different structure is a different Program, loaded by reload (§13).
- **References held at runtime** (§3.5).
- **Functions as values.** No function is a value at runtime, and no cell or record holds one. A function's body is inlined or stored once as a shared body (§11), and a lambda is inlined or evaluated by the native function or the shared body that receives it (§6.7, §10.1); nothing passes or holds a function while the Program runs.
- **Metadata attributes.** No construct attaches metadata to a declaration.
- **Bitwise operators.** `^` only counts from an array's end (§5.5).
- **Readable gate state** (§9.3).
- **Aborting evaluation** (§12.4).
- **Structural sharing.** Every cell holds its own value in its own buffer (§12.6); a derived array, map or document is a value of its own, bounded by its capacity, whatever buffer the implementation computes it in (ARCHITECTURE §4).
- **Allocation while running** (§12.6).

## 15. Packages

### 15.1 Packages

A package is a set of `.sym` files compiled together: every `.sym` file in every folder under its root, the folder holding its package file (§15.2). A project is a package; a library is a package that another package requires (§15.3); a vocabulary's prelude is a library (§15.6). A package's files share its namespace (§6.5), and a name it declares is visible outside it only when it publishes the name (§15.5).

A package's identity is its source (§15.3) without its version: `github.com/acme/drums`. A local library's identity is its path from the project's root, `../util`, and the prelude's is `prelude`. A build holds one package of each identity.

The build's top-level order (§2.1) runs through the prelude, then the libraries in the byte-wise order of their identities, then the project, each package in its own file order.

### 15.2 The package file

```ebnf
PackageFile ::= PackageItem*
PackageItem ::= PackageDecl | Require | Use | Publish
PackageDecl ::= RecordLit NEWLINE
```

A package's package file is `package.sym`, and the folder holding it is the package's root. It holds these items only, in any order, and no declaration. A `package.sym` in any other folder of the package, and a `Package`, `require`, `use` or `publish` item in any other file, is an error naming the package file.

The `Package` item is a record literal (§3.8) of the built-in record type `Package`, the one record literal that stands as an item, its type name marking the item; a literal of another type there, and a declaration named `Package` anywhere (§6.5), are errors. It states the package's identity, and its fields are compile-time values, none of which reaches the Program:

| Field | Holds |
|---|---|
| `name` | the package's name, an identifier (§2.4) |
| `version` | its version (§15.7) |
| `license` | its license, an SPDX license expression |
| `authors` | an array of strings, each naming a person: `"Ada Lovelace <ada@example.com>"` |
| `contributors` | an array of strings, as `authors` |
| `vocabulary` | the vocabulary it is written for (§15.8) |
| `options` | the build's settings, a record combined across the build's packages as below |

A library's package file holds a `Package` item with a `name` and a `version`, and a project's holds one with its `vocabulary` (§15.8). Two `Package` items are an error.

The `options` field holds the limits of §4.6, §6.3, §12.6 and §13.3, each limit that draws an error with its warning value beside it, and the sharing threshold, by the names ARCHITECTURE §9 gives them. A limit on a compilation's work, the recursive calls, inlined calls, expansion steps and expanded items of §4.6 and the nesting depth of §6.3, takes in a build the highest value any package of the build states, and a library whose value is above the starting package's, stated or default, draws a warning at the starting package's `Package` item naming the library, the limit and both values. The memory limit, the reload search bound and the sharing threshold take the starting package's value alone, and a library's is read only in a build the library starts. A warning value combines as its limit does.

```sym-package
Package {
    name: "scales",
    version: "2.4.1",
    license: "MIT",
    authors: ["Ada Lovelace <ada@example.com>"],
}

require "github.com/acme/util@1.2.0"
use util as u

publish major, minor
```

### 15.3 Requiring a package

```ebnf
Require       ::= 'require' StringLit ('as' Name)? NEWLINE
QualifiedName ::= Name '::' Name
```

A `require` item names a library by its source, a string literal without interpolation:

- `"<host>/<path>@<version>"` is the release `<version>` of the git repository at `https://<host>/<path>`: the commit its tag `v<version>` names;
- `"<host>/<path>@<commit>"` is that repository at the commit named by 7 to 40 hexadecimal digits;
- `"./<path>"` and `"../<path>"` are the library in that folder, relative to the requiring package's root.

A required library is reached by the `name` its `Package` item states, or by the name `as` gives: `require "github.com/other/scales@1.0.0" as oscales`. Two required libraries of one name, and a required library named like a declaration of the package or `prelude`, are an error whose help names `as`.

A `require` item makes every name the library publishes reachable as a qualified name in every file of the package: `scales::major`. A qualified name stands wherever a name refers to a top-level declaration (§6.5); one that the library does not publish is an error naming the library.

A commit and a local path stand only in the project's package file and in the package files of libraries the project reaches through local paths. In a library fetched by its source, either is an error.

The `math` package (§5.7) is required by every package with no `require` item, as the prelude is (§15.6): its identity is `math`, its names are reachable as `math::<name>`, and `use math` brings them in (§15.4).

### 15.4 Using names

```ebnf
Use     ::= 'use' Name (':' UseName (',' UseName)* | 'as' Name)? NEWLINE
UseName ::= Name ('as' Name)?
```

A `use` item brings a required library's published names into the package's namespace (§6.5):

- `use drums` brings every name `drums` publishes;
- `use drums: Kit, Snare` brings those two, and `use drums: Kit as Set, Snare` brings `Kit` as `Set`;
- `use drums as d` brings no name, and `d::Kit` reaches `drums::Kit`.

A `use` of a name that no `require` item of the package names is an error. A name a `use` item brings that equals another name of the package's namespace, other than a prelude name (§15.6), is an error naming both, whose help names a narrower `use` or `as`, unless both name one declaration: a library's declaration that another library republishes (§15.5) is one name.

### 15.5 Publishing

```ebnf
Publish     ::= 'publish' PublishName (',' PublishName)* NEWLINE
PublishName ::= (Name | QualifiedName) ('as' Name)?
```

A library publishes the names its `publish` items list, and only those; every other declaration of it is visible only inside it. A published name names a top-level declaration of the library, a name a `use` item brings, or a qualified name of a library it requires, and the last two republish that library's declaration. `as` publishes it under another name. A name published twice, a published name that names nothing, and a `publish` item in a project are errors.

```sym-package
use scales: major

publish Drum as Kick, Kit, Hit
publish major
publish scales::minor as Minor
```

### 15.6 The prelude

The prelude of a vocabulary (§10.1) is a library that every package written for that vocabulary requires without a `require` item: the project always, and a library whose `Package` item states that vocabulary (§15.8). In such a package, every name the prelude publishes is reachable unqualified and as `prelude::<name>`. A declaration of the package and a name a `use` item brings shadow a prelude name of their spelling in that package, and `prelude::<name>` still reaches it. The prelude's package file lists what it publishes, and a prelude declaration it does not publish is visible only inside the prelude.

### 15.7 Versions

A version is three decimal integers without signs or leading zeros, joined by `.`: `MAJOR.MINOR.PATCH`, as `2.4.1`. Versions compare by their major, then their minor, then their patch.

A build holds one version of each library: the highest version any package of the build requires of it. Two requirements of one library with different majors are an error naming each requiring package and the version it requires. A library raises its major when it removes or changes a name it publishes, and its minor when it publishes a new one.

A commit the project requires replaces every version the build requires of that library. The newest release tag in the commit's history gives the commit's major, and a major other than that of the build's other requirements of the library is the error above. A fetched release whose `Package` item states another version than its tag names is an error naming both.

### 15.8 A package's vocabulary

A vocabulary has a name and a version (§10.1, §15.7). A package's `Package` item states the vocabulary it is written for as `"<name>@<version>"`, the lowest version it accepts within that version's major: `vocabulary: "drums@2.1.0"`. A project's states the vocabulary of the framework that runs it (ARCHITECTURE §9). A library that states none uses no kind, enum, unit, attribute or prelude name. A library written for a vocabulary of another name than the project's, of another major, or of a higher version, is an error naming the library and both vocabularies, and so is a project whose framework's vocabulary differs that way from the one it states.

A vocabulary raises its major when it removes or changes a kind, an attribute, an enum variant, a unit or a name its prelude publishes, and its minor when it adds one.
