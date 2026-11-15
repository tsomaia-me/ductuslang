# Type / Expr overlap in SymphonyScript, by a run

Cold task: only SPEC.md §1.2, §2.4–2.6, §3.2, §3.8, §3.10, §3.11, §5.1, §5.3,
§5.5, §5.8, §5.10, §6.7, §6.8, §7.9 (`LoopHead`, `Iterable` and their prose),
§15.3 (`QualifiedName`) were read. No repository source, nothing under
`.claude/` or `.memory/`. Nothing under the repository was changed.

Everything lives in this directory (SCRATCH = `…/scratchpad/type-expr/`):

| file | what |
|---|---|
| `grammar.py` | the productions as data (near-literal EBNF), an EBNF→automaton compiler, the pushdown recognisers for `Type`, `Expr`, `AltType`, `Fallback`, `TypeArgs`, `Dim`; a test tokenizer and a renderer of token kinds to source text |
| `tests.py` | 133 source examples from the spec text (positive and negative) against the recognisers |
| `counts.py` | step 2: every sequence up to N over the alphabet, pruned to viable prefixes; counts per length; intersections by shape |
| `inter.py` | step 2 again for the three intersections only, to a longer length (pair-pruned) |
| `reader.py` | step 3: the single-pass reader, four positions |
| `compare.py` | step 3: the reader against the recognisers on every sequence up to N, four positions |
| `run.sh` | the one command |
| `out/` | every output: `counts.txt`, `intersections.txt`, `intersection_*.txt` (the sequences themselves), `compare.txt`, `run.log`, `full_run.log` |

The one command that repeats the run (7 min 13 s on this machine, one core, `nice -n 19`; `out/full_run.log` is its transcript):

    sh /home/cdagent/cargo-targets/claude-tmp/claude-1001/-home-cdagent-ductuslang/4c9bc397-7612-5302-a9d1-3a1193f8774f/scratchpad/type-expr/run.sh

(`sh run.sh 6 7` is the default: N=6 for the full enumeration and the reader
comparison, N=7 for the intersection listing.)

## 1. The alphabet

58 token kinds, one for each keyword and punctuator the productions name and
one each for `Name`, `Number`, `String` (a string literal with its
interpolations is one token), `JsonString`, `JsonNumber`:

    keywords (26):    if then else or and not is without div none to true false null
                      json for in into from u8 i32 i64 f32 f64 bool str
    punctuators (27): | ( ) -> , : [ ] .. ? { } :: ?? + - * / % ** < <= > >= . = ^
    classes (5):      Name Number String JsonString JsonNumber

Nothing the productions tell apart is merged: `true`/`false` are kept apart
from `null` and from `Number`; `JsonString`/`JsonNumber` apart from
`String`/`Number`; `..` apart from `.`; `::` from `:`; `??` from `?`; `**`
from `*`; `->` from `-` and `>`.

## 2. The recognisers (step 1)

`grammar.py` holds the productions of §3.2 (`Type FnType FnParam Base Dims
Size ScalarType StrType`), §3.10 (`MapType MapLit Entry`), §3.11 (`JsonType
JsonLit JsonValue JsonObject JsonArray`), §6.8 (`TypeArgs TypeArg`), §15.3
(`QualifiedName`), §5.1 (`Expr IfExpr OrExpr AndExpr NotExpr CmpExpr CmpOp
AltType Fallback AddExpr MulExpr UnaryExpr PowExpr Postfix Primary ArrayLit
Args Arg NamedArg`), §6.7 (`Lambda LambdaParams LambdaParam`), §3.8
(`RecordLit Fields`), §5.8 (`Conversion`), §5.5 (`Index Bound`), §5.10
(`ForExpr ForExprHead`), §7.9 (`LoopHead Iterable`), transcribed as text and
compiled: each rule's right-hand side becomes a small automaton over
terminals and nonterminals; a recogniser is a nondeterministic pushdown
machine over those automata (a configuration is a stack of (rule, state)
frames; a state of the recogniser is the closed set of configurations). It
accepts exactly the token sequences the productions derive. Six start
symbols give six recognisers: `Type`, `Expr`, `AltType`, `Fallback`,
`TypeArgs`, `Dim` (`Dim` is one element of `Dims`: `'[' Size ']' | '[' '..'
Size ']'`).

Two things are not literal transcription:

* `Result`, used by §6.7's `Lambda ::= … '(' LambdaParams? ')' Result? ':'
  Expr`, is defined in §6.3, which was not read. It is taken as `'->' Type`
  from §6.7's example `(x: i32) -> bool: x > 60`.
* The one prose rule that changes a production (below): `json` followed by
  `[` is a type only when `..` follows the bracket.

### The prose applied, quoted

Each sentence fixes a reading by tokens alone; none depends on what a name
names.

1. §5.1: "A `{` in an expression opens a map literal (§3.10), after a type
   name a record literal (§3.8), and after `json` a document literal
   (§3.11)." — Applied as: the brace group's content grammar is chosen by the
   token before `{`: after `json` JSON text (`JsonObject`), after a name, a
   qualified name or a bracket list after one, record `Fields`, otherwise
   `MapLit` (or, where a type may stand, `MapType`). The productions already
   say this (no juxtaposition exists in `Expr`, so `Name {` is only
   `RecordLit`); "after a type name" was read by tokens as "after a name",
   since whether the name names a type is what the overlap is about.
2. §3.11: "The word `json` opens JSON text by the tokens beside it alone.
   Where it does not follow `.`, and `{`, a `[` that `..` does not follow, a
   JSON string, a JSON number or `-` follows it, that token begins a document
   literal's JSON text, wherever the word stands, every place a type stands
   among them: an annotation, the type of a parameter, a result or a field, a
   type argument, a `match` label and the right operand of `is`. So
   `json[..64]` is the type with its budget, `json[2]` is the document `[2]`,
   and a dimension directly on the type is written `(json)[2]`." — Applied as
   the one change to the productions: in `Base` and in `AltType`, `json` is
   `'json' ('[' '..' Size ']' Dims?)?` (plus `'?'?` in `Base`) instead of
   `JsonType Dims?`, so `json [ Size ]` is never a type; `json[..64][2]`
   stays one. (The `.` case does not arise: no production puts `json` after
   `.`; see "not covered".)
3. §5.3: "After `is`, the word `json` reads as §3.11 says, and a brace group
   followed by `[` is a map type (§3.10) only when `..` follows the bracket."
   — Applied as: the `AltType` change of item 2, and `MapType ::= '{' Type
   ':' Type '}' '[' '..' Size ']'` taken as written, so `{a: b}[0]` on the
   right of `is` is only a sliced map literal; `{a: b}[..4]` keeps both
   readings, since "only when" is a necessary condition and the productions
   give the map-literal slice too (see §6 below).
4. §6.8: "In a type, the first bracket list after a generic type's name is
   its arguments, one for each bracket parameter in order and read by the
   parameter it fills: a type for a type parameter, and for a value
   parameter a compile-time integer expression, as a size is (§3.2), the
   enclosing declaration's value parameters among what it reads. Later
   bracket lists are dimensions (§3.2), and after any other name every
   bracket list is a dimension: `Stack[i32, 8][4]` holds four stacks, and
   `Band[4]` four bands." — Applied as `(Name | QualifiedName) TypeArgs?
   Dims?`: the first list after a name is `TypeArgs` (one or more `Type |
   Expr`) or a `Dim`; every later list is a `Dim`. Whether the name is
   generic is not applied, so `Band[4]` keeps both the argument and the
   dimension reading inside the type reading (both are "type").

Not applied, because it depends on what a name names: §5.3's "a name is this
test when it names a type … and a value compared otherwise", and §5.1's "type
name". Not applied either, as a semantic resolution rather than a syntax
rule: §5.3's "`none` is this test" — `x is none` is reported as "both",
which that sentence then resolves to the sum test.

`tests.py`: 133 examples from the spec's text (`i32[3][2]`, `str[..32][4]`,
`(i32?)[..4]`, `Stack[i32, 8] {}`, `json {"a": 1}`, `json[2]` vs
`json[..64]`, `x is Band[4]`, `-2 ** 2`, `a < b < c` rejected, `count(readings,
p: p > 60)`, `(x: i32) -> bool: x > 60` as an argument, the §3.10 `hits`
rule, both `for` expressions of §5.10, …) all classify as the text says.

## 3. The enumeration (step 2)

N reached: **6** for the full enumeration (every sequence of 1–6 tokens over
the 58 kinds, pruned to viable prefixes of `Type`, `Expr` or `AltType`;
14,437,612 viable prefixes in all) and for the reader comparison; **7** for
the intersection listings. The three recognisers' states are interned and
their transitions memoised, so a prefix's whole future depends only on its
state tuple, and the counts come from a recursion over (state tuple,
remaining length) that expands each distinct tuple once per remaining length:
every sequence is covered, none is written out except those in the
intersections (`out/intersection_*.txt`). Length 6 took 81 s and 46,140 memo
entries; each further length costs about seven times the last, so N=7 for
everything would have been about 15 minutes for the counts alone and the
comparison the same again — past the limit. The pair-pruned intersection
search (`inter.py`, both recognisers of a pair kept alive) reached length 7
in 163 s; its counts for lengths ≤ 6 equal the full enumeration's, a
cross-check between the two searches.

Sequences of exactly n tokens accepted (from `out/counts.txt`; n = 7
intersections from `out/intersections.txt`):

| n | Type | Expr | AltType | Fallback | TypeArgs | Dim | Type∩Expr | AltType∩Fallback | TypeArgs∩Dim | viable prefixes (T∪E∪A) |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 10 | 7 | 10 | 7 | 0 | 0 | 2 | 2 | 0 | 22 |
| 2 | 10 | 22 | 0 | 15 | 0 | 0 | 0 | 0 | 0 | 301 |
| 3 | 111 | 910 | 1 | 489 | 7 | 7 | 3 | 1 | 7 | 4,503 |
| 4 | 302 | 4,863 | 64 | 2,160 | 22 | 22 | 14 | 7 | 22 | 62,711 |
| 5 | 1,900 | 107,763 | 249 | 38,843 | 959 | 910 | 61 | 29 | 910 | 927,210 |
| 6 | 14,890 | 800,555 | 7,826 | 261,005 | 5,171 | 4,863 | 1,899 | 939 | 4,863 | 13,442,865 |
| 7 | – | – | – | – | – | – | 11,792 | 5,851 | 107,763 | – |
| Σ(1–6) | 17,223 | 914,120 | 8,150 | 302,519 | 6,159 | 5,802 | 1,979 | 978 | 5,802 | 14,437,612 |

(`Dim` at length n is `[E]` or `[..E]` with E an `Expr`, so Dim(n) = Expr(n−2)
+ Expr(n−3); `TypeArgs∩Dim`(n) = Expr(n−2): the whole of it is `[E]`.)

### Type ∩ Expr, by shape (to length 7; `…` stands for the contents of a group)

| shape | count ≤7 | by length | example |
|---|---|---|---|
| `Name` | 1 | n1:1 | `a` |
| `none` | 1 | n1:1 | `none` |
| `Name :: Name` | 1 | n3:1 | `a::b` |
| `( … )` | 80 | n3:2 n5:3 n6:14 n7:61 | `(a)`, `(none)`, `((a))`, `(a::b)`, `(a[1])` |
| `Name [ … ]` | 5,802 | n4:7 n5:22 n6:910 n7:4863 | `a[b]`, `Band[4]`, `a["s"]`, `a[b + 1]`, `a[if b then 1 else 2]` |
| `none [ … ]` | 5,802 | n4:7 … n7:4863 | `none[a]` |
| `Name [ .. … ]` | 939 | n5:7 n6:22 n7:910 | `a[..b]`, `T[..N + 1]` |
| `none [ .. … ]` | 939 | n5:7 n6:22 n7:910 | `none[..a]` |
| `( … ) [ … ]` | 58 | n6:14 n7:44 | `(a)[b]` |
| `Name :: Name [ … ]` | 29 | n6:7 n7:22 | `a::b[c]` |
| `( … ) [ .. … ]` | 14 | n7:14 | `(a)[..b]` |
| `Name :: Name [ .. … ]` | 7 | n7:7 | `a::b[..c]` |
| `Name [ … ] [ … ]` | 49 | n7:49 | `a[b][c]` |
| `none [ … ] [ … ]` | 49 | n7:49 | `none[a][b]` |

The contents `…` of a bracket group is any `Expr` (the `TypeArgs` reading
takes it as a value argument, the `Dims` reading as a `Size`, the `Expr`
reading as an `Index`); `[..…]` is a capacity dimension against a slice. A
bracket group whose contents is a type but not an expression (`a[i32]`,
`a[b | c]`, `a[b?]`, `a[(b) -> c]`, `a[json[..1]]`) is type-only; one with a
comma (`a[b, c]`) is type-only; one with `^`, with `..` after an item or with
two bounds (`a[^1]`, `a[1..2]`, `a[1..]`, `a[..]`) is expression-only. The
contents of `( … )` is itself in the intersection. Shapes that first appear
past length 7, checked one by one with the recognisers and the reader:

| sequence | Type | Expr | AltType | Fallback | reading |
|---|---|---|---|---|---|
| `{a: b}[..c]` (8 tokens) | yes | yes | yes | yes | map type `{K: V}[..N]` against a sliced map literal |
| `{a: b}[..c][2]` (11) | yes | yes | yes | yes | the same with a dimension against an index |
| `a[1][..2]` (8) | yes | yes | yes | yes | argument list then capacity dimension, against index then slice |
| `{a: b}[..c]?` (9) | yes | no | no | no | type-only: `?` |
| `{a: b, c: d}[..1]`, `{1: 2}[..1]` | no | yes | no | yes | two entries, or a non-type key: literal only |
| `{a: i32}[..1]` | yes | no | yes | no | a type as a value: type only |
| `{a: b}[0]` | no | yes | no | yes | §5.3: no `..` after the bracket |

So Type ∩ Expr, as far as length 7 shows and the productions' structure
explains, is: a name, a qualified name or `none`, or a parenthesised member of
the intersection, followed by any number of bracket groups each holding one
expression (`[E]`) or a leading `..` and one expression (`[..E]`) — plus,
from length 8, a one-entry brace group whose key and value are both in the
intersection, followed by `[..E]` and then such bracket groups.

### AltType ∩ Fallback (the right operand of `is`), by shape (to length 7)

| shape | count ≤7 | by length | example |
|---|---|---|---|
| `Name` | 1 | n1:1 | `x is a` |
| `none` | 1 | n1:1 | `x is none` |
| `Name :: Name` | 1 | n3:1 | `x is a::b` |
| `Name [ … ]` | 5,802 | n4:7 n5:22 n6:910 n7:4863 | `x is Band[4]`, `x is levels[0]` |
| `Name [ .. … ]` | 939 | n5:7 n6:22 n7:910 | `x is a[..b]` |
| `Name :: Name [ … ]` | 29 | n6:7 n7:22 | `x is a::b[c]` |
| `Name :: Name [ .. … ]` | 7 | n7:7 | `x is a::b[..c]` |
| `Name [ … ] [ … ]` | 49 | n7:49 | `x is a[b][c]` |

Beyond length 7: `x is {a: b}[..c]` (8 tokens) and `x is {a: b}[..c][2]` are in
both. Against Type ∩ Expr, the `is` operand drops `( … )` (AltType has no
parentheses) and `none [ … ]` (`AltType ::= 'none' | … Dims?` gives `none` no
dimensions, while §3.2's `Base` does: `none[4]` is a Type). This is exactly
the set §5.3's sentence lists ("`none`, a name, a qualified name and either
with an index") plus the brace group with `[..`, which §5.3's "only when"
sentence admits as a map type without excluding the sliced map literal.

### TypeArgs ∩ Dim (the first bracket list after a name in a type)

One shape: `[ … ]` with `…` any `Expr` — 7, 22, 910, 4,863, 107,763 sequences
at lengths 3–7, i.e. Expr(n−2). `[..E]` is dimension-only; a list with a
comma or with a type that is not an expression (`[i32]`, `[a | b]`) is
arguments-only; `[^1]`, `[a..b]` are neither.

## 4. The single-pass reader (step 3)

`reader.py` is a deterministic stack machine: one left-to-right pass, each
token handed to the top frame, which consumes it or closes (pops, hands its
result to the frame below, and that frame sees the same token). No token is
ever re-read; nothing is undone. At the end a marker closes every frame and
the root's result is the answer: type-only, expression-only, both or neither.
The four positions differ only in the root frame:

* free sequence and type argument: an item frame with both readings open,
  full `Type` and full `Expr` (the two positions are the same language pair;
  a type argument's terminators `,` and `]` belong to the enclosing bracket
  frame);
* right operand after `is` (`not`): an item frame with `AltType` against
  `Fallback` (no `(…)`, `|`, `?`, `FnType`, no `none[…]` on the type side; no
  `if`/`for`/`not`/`and`/`or`/comparison/`is` on the expression side);
* first bracket list after a name in a type: a bracket frame allowing
  `TypeArgs` and `Dim`, nothing else (no index), answering arguments-only,
  dimension-only, both or neither.

### What it must remember

Every fact is a field of a frame on the stack; the stack depth is the
nesting depth of brackets, parens, braces and `if`/`for` heads.

Item frame (one type-and/or-expression item, also used for every nested
expression, size, type argument, annotation, lambda body, `is` operand):

* `T`, `E`: the type reading / the expression reading is still viable.
* which sub-language each reading follows (`Type` or `AltType`; `Expr` or
  `Fallback`), whether a lambda may stand here (argument position), and what
  type form may start (full `Type` including `FnType`; a `Base` only, after
  `|`; `AltType`).
* where the reading is: expecting an operand; after a complete operand or
  base; waiting for the `Name` after `::` or `.`; after `is` (a `not` may
  follow); after `json` (JSON text or a type continuation may follow); after
  a paren group, holding which of its four readings still fit (FnType
  parameters, grouped type, parenthesised expression, lambda parameters);
  after an FnType/lambda result type (a `:` would make it a lambda); complete
  and closed (an `if` or `for` expression ended: nothing may follow); complete
  after a type-only `is` operand (only `and`/`or` may follow).
* type detail after a base: a bare name (`::`, a first bracket list, `?`, `|`
  may follow); a qualified name (no `::`); after the first bracket list (only
  dimensions); a scalar, `none`, `str`; `json` without a budget (the next
  bracket must be `[..`); a grouped type; map braces still needing `[..Size]`;
  after `?` (only `|`); after `|` (a base must follow).
* expression detail after an operand: a bare `Name` (so `::`, `{`, a lambda's
  `:` or a named argument's `=` may follow); a qualified name (`{` may
  follow); a bracket list that fits `TypeArgs` and `Index` (`{` may follow,
  and so may anything else); a bracket list that fits only `TypeArgs` (only
  `{` may follow, for a record literal); a scalar keyword (only `(` may
  follow, for a conversion); a plain operand.
* whether `not` may start the next operand (at the start of an `Expr` and
  after `and`, `or`, `not`); whether `if`/`for` may (a whole `Expr` is
  expected: not after `not`, an operator or `is`); whether a comparison
  operator has been seen in the current `CmpExpr` (comparisons do not
  chain); whether nothing has been consumed yet; whether the item so far is
  exactly one `Name`.

Bracket frame (`[` after an operand or base): which forms the context allows
(type arguments; a dimension; a budget `[..Size]` only, after `json` or map
braces; an index); the phase (before the first item; after it; after a
comma, so arguments only; after `..`; after `^`); the first item's two
readings; the number of items so far.

Paren frame (`(` at operand start): which of FnType parameters / grouped type
/ parenthesised expression / lambda parameters still fit; the phase (first
item, after an item, after `Name :`, after a comma); the last item's two
readings and whether it was a bare name.

Args frame (call): positional or named mode once decided; phase; the last
argument's reading and whether it was a bare name (for `=`).

Brace frame: its kind, decided by the token before `{` (record fields after
a name, qualified name or bracket list; JSON object after `json`; map
otherwise); phase; for a map: whether the map-type reading and the
map-literal reading still fit, and the key's two readings while the value is
read.

JSON frames: the phase of a JSON value, object or array; after `json [`, a
frame holding both readings for exactly one token.

Array, `if`, `for` frames: the phase only (which part is being read).

### Where each decision falls

* `{`: at the brace, by the token before it (§5.1): `json` → JSON object;
  a name, qualified name or bracket list after one → record fields;
  otherwise → map. Between map type and map literal: a second entry, a
  trailing comma, or a key or value that is not a type kills the type
  reading; a key or value that is not an expression kills the literal
  reading; at the token after `}`, the type reading survives only into
  `[..` (checked when `..` follows `[`), the literal reading into anything
  an operand may take (§5.3).
* `json`: at the token after it (§3.11): `[` then `..` → type budget (decided
  at the second token after `json`); `[` then anything else → JSON array;
  `{`, a JSON string or number, `-`, `true`/`false`/`null`, a name, `to`,
  `(` → document literal; `?`, `|` or a closer → the type `json`.
* a bracket list after a name: at `]`, from what the list held: `^`, `..`
  after an item, or two bounds → index only; a comma → arguments only; a
  leading `..` → dimension or slice; one item → arguments/dimension as far as
  the item is a type or an expression, index as far as it is an expression.
  Later lists after a name: dimensions only on the type side (§6.8).
* `::`: both readings survive, a `Name` must follow; a third name kills both.
* `?` and `|`: the expression reading dies at once; a second `?` kills the
  type reading; after `?` only `|` continues it.
* `.`, `(` as a call, every arithmetic or comparison operator, `??`, `and`,
  `or`, `is`: the type reading dies at once; `(` after a scalar keyword is a
  conversion.
* `(`: the four readings of the group are narrowed as its items arrive (`:`
  after a bare name → parameter annotation; a comma → parameter list only; a
  non-name item → no lambda parameters); at `)` the next token decides: `->`
  → function type or lambda result (then `:` → lambda); `:` → lambda; else
  grouped type and/or parenthesised expression.
* the right operand of `is`: a nested item frame in `AltType`/`Fallback`
  mode; when it closes, if its expression reading survived the comparison is
  complete and the outer expression goes on (one comparison per `CmpExpr`);
  if only its type reading survived, only `and`/`or` may follow.
* the end: the root item's `(T, E)`.

### Comparison with the recognisers

`compare.py` walks every token sequence of length ≤ 6 for each of the four
positions. Both machines are deterministic, so the pair (reader state,
recogniser state pair) fixes every future; each pair is expanded once per
remaining depth. Where both recognisers of a position are dead, the reader
is run alone to make sure it keeps answering neither; where the reader is
dead but a recogniser alive, the recognisers are run alone to make sure
neither ever accepts. That covers all 58^n sequences for n ≤ 6 in every
position (`out/compare.txt`):

| position | distinct (reader, recogniser) pairs | disagreements |
|---|---|---|
| free sequence (Type vs Expr) | 356,488 | 0 |
| after `is` (AltType vs Fallback) | 247,994 | 0 |
| type argument (Type vs Expr) | 356,488 | 0 |
| first bracket list after a name (TypeArgs vs Dim) | 56,799 | 0 |

**No disagreement up to length 6 in any position.** During development the
check found one: `a??` as `Name ? ?` was read as a type (a second `?` after a
`?`); fixed. It also showed one place where the reader outlived the grammar
without ever answering wrongly: in a call's argument `a((b, 1`, after the
comma had left only lambda parameters, the reader accepted `1` as an item and
died at `)`, where the grammar dies at `1`; tightened so the reader now dies
at the same token; the final run reports 0 reader-alone states in every
position, i.e. the reader is dead exactly where both recognisers are.

## 5. Not covered

* §2.5's rule that a keyword stands as a member name after `.`, as a record
  literal key and in attribute clauses: not applied; `Name` stays `Name` and
  keywords stay keywords everywhere, so `a.json`, `Band {in: 1}` are not in
  the languages here. (The §3.11 "where it does not follow `.`" case never
  arises for that reason.)
* §5.10's prose that a one-line `for` expression stands in parentheses
  wherever it is an operand, an argument, a start or an iterable: not
  applied; the productions admit `[for …]`, `f(for …)`, and the counts of
  `Expr` include them. `for` starts no type, so the overlap is unaffected.
* `Result` (§6.3) assumed as `'->' Type`; `ForExprBlock`, `match`, layout
  tokens (NEWLINE/INDENT/DEDENT), attribute clauses, patterns, declarations:
  outside the productions named.
* The lexical level: literals are single tokens; telling `JsonString` /
  `JsonNumber` from `String` / `Number` is left to a lexer that itself must
  apply §3.11's rule (the test tokenizer in `grammar.py` does, for the spec
  examples only).
* Everything semantic: what a name names, generic arity, `none` as the sum
  test (§5.3), comparisons on records, sizes' ranges.
* Lengths past N: counts to 6, intersections to 7, comparison to 6; the
  longer shapes above were checked one by one, not enumerated.

## 6. Observations worth a ruling

* `x is {a: b}[..4]` (and `{a: b}[..4][2]`) is both a map type with its
  capacity and a sliced map literal; §5.3's sentence gives `..` as the
  condition for the map type without excluding the literal. If "only when" is
  meant as "exactly when", the sliced-literal reading on the right of `is`
  should be ruled out in the text.
* §3.2's `Base` gives `none` dimensions (`none[4]`, `none[..4]` are types),
  §5.1's `AltType` does not; so `none[1]` is "both" as a free sequence and
  expression-only after `is`.
* By the productions alone, a `TypeArg` may be any expression and a `Size`
  too, so `a["s"]`, `a[b ?? 1]`, `a[if b then 1 else 2]`, `a[json 1]` are
  types as well as expressions; the overlap is the whole of `Expr` inside
  one-item bracket lists after a name.
