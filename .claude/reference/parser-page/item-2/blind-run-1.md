# The blind run for item 2 (1): the overlap of `Type` and `Expr`, proved by a run, 2026-11-15

Launched by the primary as a subagent, given the SPEC's grammar sections and the goal only, nothing of the notes, the proposal or the colon ruling's reasoning. Its scripts, its `REPORT.md` and its small outputs are in `blind-run-1/` beside this file (the full intersection listings, up to 5 MB, stay out of the repo and are made again by `run.sh`). The brief and the report are pasted whole. The primary read the report only after writing `proposal.md`; what the report changes in the proposal is said at the end.

## The brief

```
Cold task. You are given a grammar and a goal, nothing else about the project, and you work alone.

Rules of the run:
- Change no file under `/home/cdagent/ductuslang`. Write your scripts, their outputs and your report only under <scratch>/type-expr/ (call it SCRATCH below). Write no report file anywhere else, and return your report as text in your final message as well.
- Read nothing under `/home/cdagent/ductuslang/.claude/` or `.memory/`, and no source code of the repository. Read only `/home/cdagent/ductuslang/packages/symphonyscript/docs/SPEC.md`, and of it only these sections: §1.2 (notation), §2.4 to §2.6, §3.2, §3.8, §3.10, §3.11, §5.1, §5.3, §5.5, §5.8, §5.10, §6.7, §6.8, §7.9 (its `LoopHead` and `Iterable` productions and the prose on them only), §15.3 (`QualifiedName`). Nothing else.
- Python 3 is at /usr/bin/python3; rustc and cargo are under ~/.cargo/bin if you prefer Rust. Run Python with `-I`. Nothing heavy: run with `nice -n 19`, and keep the whole run under about twenty minutes of machine time.

The goal. SymphonyScript writes types and expressions with overlapping tokens, and three positions take either a type or an expression: the right operand of `is` (§5.1 `CmpExpr ::= Fallback (CmpOp Fallback | 'is' 'not'? AltType)?` with §5.3's prose), the first bracket list after a name in a type (§6.8: `TypeArgs`, whose `TypeArg ::= Type | Expr`, against §3.2's `Dims`, whose `Size ::= Expr`), and a type argument itself. A parser of one file knows no names, so it must read such text once, left to right, with no backtracking, into one of four answers: type-only, expression-only, both (the text fits either reading) or neither (a syntax fault). Prove by a run what the overlap of `Type` and `Expr` is over short token sequences, and that a single-pass reader can classify it.

Do this, in order:

1. From the productions alone, write three recognisers over abstract token kinds: one for `Type` (§3.2 `Type`, with `FnType`, `Base`, `Dims`, `Size`, `ScalarType`, `StrType`, §3.10 `MapType`, §3.11 `JsonType`, §6.8 `TypeArgs`, §15.3 `QualifiedName`), one for `Expr` (§5.1 `Expr` with everything it reaches: `IfExpr`, the operator levels, `Postfix`, `Primary`, `ArrayLit`, `Args` with `Arg ::= Expr | Lambda` and §6.7's `Lambda`, §3.8 `RecordLit`, §3.10 `MapLit`, §3.11 `JsonLit`, §5.8 `Conversion`, §5.5 `Index` and `Bound`, §5.10 `ForExpr` with §7.9's `LoopHead` and `Iterable`), and one for `AltType` (§5.1). Each accepts exactly the token sequences its productions derive. The alphabet: one kind for each keyword and punctuator the productions name, and one each for Name, Number, String (a string literal with its interpolations is one token here), JsonString, JsonNumber. Keep every token kind that any of these productions distinguishes; merge nothing the grammar tells apart. Beside the productions, apply the prose that fixes a reading by tokens, and quote each sentence you applied: §5.1's sentence on what `{` opens, §3.11's on where the word `json` opens JSON text, §5.3's on the right operand of `is` (the map type and `json` there), §6.8's on the first bracket list after a name. Where the prose depends on what a name names, do not apply it: that is what the overlap is about.

2. Enumerate every token sequence up to a length N you state, over that alphabet, pruned to the viable prefixes of at least one of the three recognisers (a prefix that none can extend to an accepted sequence is dropped). List the intersection Type ∩ Expr and the intersection AltType ∩ Fallback (where `Fallback` is §5.1's production). Group each intersection by shape and give each group one example in source text. Report for each length how many sequences each recogniser accepts and how many lie in each intersection. Choose N as large as runs within the time limit, and say what N you reached.

3. Write a single-pass reader: one left-to-right pass with no backtracking over a token sequence that answers type-only, expression-only, both or neither, for each of four positions: a free sequence; the right operand after `is`; a type argument; the first bracket list after a name in a type. Say exactly what it must remember as it reads (for instance whether a comma list is still open, or whether a brace group may still be a record literal) and where each decision is made. Compare it against the recognisers on every sequence up to N in every position and report every disagreement, or that there is none.

4. Report, as text in your final message and in SCRATCH/REPORT.md: the alphabet; N and the counts by length; each intersection by shape with source examples; the facts the single-pass reader carries and where each decision falls; every disagreement; the prose sentences you applied, quoted with their section; what you did not cover; and the one command that repeats the run. Keep the scripts under SCRATCH so the run can be repeated.
```

## The report, word for word

# Type / Expr overlap in SymphonyScript, by a run

Scripts, outputs and this report are under SCRATCH = `/home/cdagent/cargo-targets/claude-tmp/claude-1001/-home-cdagent-ductuslang/4c9bc397-7612-5302-a9d1-3a1193f8774f/scratchpad/type-expr/` (`REPORT.md` there has the same content). Nothing under `/home/cdagent/ductuslang` was changed; only the permitted SPEC.md sections were read (plus the heading list to locate them). One slip: a `compare.py` smoke run wrote its `compare.txt` to `/tmp/dummy_unused` for 20 seconds; it was deleted at once.

**The one command** (7 min 13 s on this machine, one core, `nice -n 19`; transcript in `out/full_run.log`):

    sh /home/cdagent/cargo-targets/claude-tmp/claude-1001/-home-cdagent-ductuslang/4c9bc397-7612-5302-a9d1-3a1193f8774f/scratchpad/type-expr/run.sh

Files: `grammar.py` (productions as data, EBNF→automaton compiler, pushdown recognisers for `Type`, `Expr`, `AltType`, `Fallback`, `TypeArgs`, `Dim`, a test tokenizer, a token→source renderer), `tests.py` (133 spec examples), `counts.py` (step 2), `inter.py` (intersections to a longer length), `reader.py` (step 3), `compare.py` (reader vs recognisers), `run.sh`, `out/` (`counts.txt`, `intersections.txt`, `intersection_*.txt` with every sequence, `compare.txt`, logs).

## 1. Alphabet — 58 token kinds

    keywords (26):    if then else or and not is without div none to true false null json for in into from u8 i32 i64 f32 f64 bool str
    punctuators (27): | ( ) -> , : [ ] .. ? { } :: ?? + - * / % ** < <= > >= . = ^
    classes (5):      Name Number String JsonString JsonNumber

Nothing the productions tell apart is merged (`true`/`false` vs `null` vs `Number`; `JsonString`/`JsonNumber` vs `String`/`Number`; `..` vs `.`; `::` vs `:`; `??` vs `?`; `**` vs `*`; `->` vs `-`,`>`).

## 2. Recognisers (step 1)

The productions of §3.2, §3.8, §3.10, §3.11, §5.1, §5.5, §5.8, §5.10, §6.7, §6.8, §7.9 (`LoopHead`, `Iterable`), §15.3 are transcribed as EBNF text; each rule compiles to a small automaton; a recogniser is a nondeterministic pushdown machine (configuration = stack of (rule, state) frames; state = closed set of configurations, interned, transitions memoised). It accepts exactly what the productions derive. Six start symbols: `Type`, `Expr`, `AltType`, `Fallback`, `TypeArgs`, `Dim` (one element of `Dims`).

Not literal: `Result` (§6.7's `Lambda … Result? ':' Expr`) is defined in §6.3, not read; taken as `'->' Type` from §6.7's example `(x: i32) -> bool: x > 60`.

### Prose applied, quoted

1. §5.1: "A `{` in an expression opens a map literal (§3.10), after a type name a record literal (§3.8), and after `json` a document literal (§3.11)." — The brace content's grammar is chosen by the token before `{`: after `json` JSON text, after a name / qualified name / bracket list after one, record `Fields`, otherwise `MapLit` (or `MapType` where a type may stand). The productions already say this (no juxtaposition exists in `Expr`); "type name" read by tokens as "name".
2. §3.11: "The word `json` opens JSON text by the tokens beside it alone. Where it does not follow `.`, and `{`, a `[` that `..` does not follow, a JSON string, a JSON number or `-` follows it, that token begins a document literal's JSON text, wherever the word stands, every place a type stands among them: an annotation, the type of a parameter, a result or a field, a type argument, a `match` label and the right operand of `is`. So `json[..64]` is the type with its budget, `json[2]` is the document `[2]`, and a dimension directly on the type is written `(json)[2]`." — The one change to a production: in `Base` and `AltType`, `json` is `'json' ('[' '..' Size ']' Dims?)?` (plus `'?'?` in `Base`) instead of `JsonType Dims?`; `json [ Size ]` is never a type, `json[..64][2]` still is.
3. §5.3: "After `is`, the word `json` reads as §3.11 says, and a brace group followed by `[` is a map type (§3.10) only when `..` follows the bracket." — The `AltType` change above, and `MapType ::= '{' Type ':' Type '}' '[' '..' Size ']'` as written: `{a: b}[0]` after `is` is only a sliced map literal; `{a: b}[..4]` keeps both readings ("only when" is a necessary condition; the productions give the slice too — see §6).
4. §6.8: "In a type, the first bracket list after a generic type's name is its arguments, one for each bracket parameter in order and read by the parameter it fills: a type for a type parameter, and for a value parameter a compile-time integer expression, as a size is (§3.2), the enclosing declaration's value parameters among what it reads. Later bracket lists are dimensions (§3.2), and after any other name every bracket list is a dimension: `Stack[i32, 8][4]` holds four stacks, and `Band[4]` four bands." — Applied as `(Name | QualifiedName) TypeArgs? Dims?`: first list `TypeArgs` or a `Dim`, later lists `Dim`s; "generic" not applied.

Not applied (depends on what a name names): §5.3's "a name is this test when it names a type … and a value compared otherwise"; §5.1's "type name". Not applied as a semantic resolution: §5.3's "`none` is this test" (`x is none` is reported "both").

`tests.py`: 133 examples from the spec text (`i32[3][2]`, `str[..32][4]`, `(i32?)[..4]`, `Stack[i32, 8] {}`, `json {"a": 1}`, `json[2]` vs `json[..64]`, `x is Band[4]`, `-2 ** 2`, `a < b < c` rejected, `count(readings, p: p > 60)`, the §3.10 `hits` rule, both §5.10 `for` expressions, …) all classify as the text says.

## 3. Enumeration (step 2)

**N = 6** for the full enumeration (every sequence of 1–6 tokens over the 58 kinds, pruned to viable prefixes of `Type`/`Expr`/`AltType`: 14,437,612 viable prefixes) and for the reader comparison; **N = 7** for the intersection listings. Counts come from a memoised recursion over (recogniser state tuple, remaining length) — every sequence is covered, each distinct state once per remaining length. Length 6 took 81 s / 46,140 memo entries; each further length costs ~7×, so N=7 for everything would have been ~15 min for counts and the same again for the comparison. The pair-pruned intersection search reached 7 in 163 s; its counts for n ≤ 6 equal the full enumeration's (cross-check).

Sequences of exactly n tokens accepted:

| n | Type | Expr | AltType | Fallback | TypeArgs | Dim | Type∩Expr | AltType∩Fallback | TypeArgs∩Dim | viable (T∪E∪A) |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 10 | 7 | 10 | 7 | 0 | 0 | 2 | 2 | 0 | 22 |
| 2 | 10 | 22 | 0 | 15 | 0 | 0 | 0 | 0 | 0 | 301 |
| 3 | 111 | 910 | 1 | 489 | 7 | 7 | 3 | 1 | 7 | 4,503 |
| 4 | 302 | 4,863 | 64 | 2,160 | 22 | 22 | 14 | 7 | 22 | 62,711 |
| 5 | 1,900 | 107,763 | 249 | 38,843 | 959 | 910 | 61 | 29 | 910 | 927,210 |
| 6 | 14,890 | 800,555 | 7,826 | 261,005 | 5,171 | 4,863 | 1,899 | 939 | 4,863 | 13,442,865 |
| 7 | – | – | – | – | – | – | 11,792 | 5,851 | 107,763 | – |
| Σ1–6 | 17,223 | 914,120 | 8,150 | 302,519 | 6,159 | 5,802 | 1,979 | 978 | 5,802 | 14,437,612 |

(Dim(n) = Expr(n−2) + Expr(n−3); TypeArgs∩Dim(n) = Expr(n−2): it is exactly `[E]`.)

### Type ∩ Expr by shape (to length 7; `…` = contents of a group)

| shape | count ≤7 | by length | example |
|---|---|---|---|
| `Name` | 1 | n1:1 | `a` |
| `none` | 1 | n1:1 | `none` |
| `Name :: Name` | 1 | n3:1 | `a::b` |
| `( … )` | 80 | n3:2 n5:3 n6:14 n7:61 | `(a)`, `((a))`, `(a::b)`, `(a[1])` |
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

The contents of a bracket group is any `Expr` (value argument / `Size` / `Index`); `[..…]` is a capacity dimension against a slice. Type-only brackets: a type that is not an expression (`a[i32]`, `a[b | c]`, `a[b?]`, `a[(b) -> c]`, `a[json[..1]]`) or a comma (`a[b, c]`). Expression-only: `^`, `..` after an item, two bounds (`a[^1]`, `a[1..2]`, `a[1..]`, `a[..]`). Shapes first appearing past length 7, checked one by one with recognisers and reader: `{a: b}[..c]` (8 tokens) both; `{a: b}[..c][2]` both; `a[1][..2]` both; `{a: b}[..c]?` type-only; `{a: b, c: d}[..1]`, `{1: 2}[..1]`, `{a: b}[0]` expression-only; `{a: i32}[..1]` type-only. So Type ∩ Expr = a name, qualified name or `none`, or a parenthesised member of the intersection, followed by any number of `[E]` / `[..E]` groups — plus, from length 8, a one-entry brace group with key and value in the intersection followed by `[..E]` and then such groups.

### AltType ∩ Fallback (right operand of `is`) by shape (to length 7)

`Name` (1), `none` (1), `Name :: Name` (1), `Name [ … ]` (5,802: `x is Band[4]`, `x is levels[0]`), `Name [ .. … ]` (939), `Name :: Name [ … ]` (29), `Name :: Name [ .. … ]` (7), `Name [ … ] [ … ]` (49); beyond 7: `x is {a: b}[..c]`, `x is {a: b}[..c][2]` both. Against Type ∩ Expr it drops `( … )` (AltType has no parentheses) and `none [ … ]` (`AltType`'s `none` takes no Dims; §3.2's `Base` gives `none[4]` as a Type). This is exactly §5.3's list ("`none`, a name, a qualified name and either with an index") plus the brace group with `[..`.

### TypeArgs ∩ Dim (first bracket list after a name)

One shape, `[ … ]` with any `Expr` inside: 7, 22, 910, 4,863, 107,763 at lengths 3–7. `[..E]` dimension-only; a comma or a non-expression type (`[i32]`, `[a | b]`) arguments-only; `[^1]`, `[a..b]` neither.

## 4. Single-pass reader (step 3)

`reader.py`: a deterministic stack machine; each token goes to the top frame, which consumes it or closes (pops, hands its result down, and the same token goes to the frame below); no token is re-read, nothing undone; an END marker closes all frames; the root's `(T, E)` is the answer. Positions: free sequence and type argument = item frame with full `Type` vs full `Expr` (same language pair; a type argument's `,`/`]` belong to the enclosing bracket frame); after `is` = item frame `AltType` vs `Fallback`; first bracket list = a bracket frame allowing `TypeArgs` and `Dim` only.

**What it must remember** (fields of the frames; stack depth = nesting depth):

Item frame: `T`, `E` (each reading still viable); which sub-language each follows (`Type`/`AltType`, `Expr`/`Fallback`), whether a lambda may stand here, what type form may start (full Type incl. FnType; a Base only, after `|`; AltType); where the reading is (expecting an operand; after a complete operand/base; waiting for the Name after `::` or `.`; after `is` (a `not` may follow); after `json`; after a paren group, holding which of its four readings still fit — FnType params / grouped type / paren expression / lambda params; after an FnType or lambda result type; closed after an `if`/`for` expression; closed after a type-only `is` operand where only `and`/`or` may follow); type detail after a base (bare name: `::`, first bracket list, `?`, `|` may follow; qualified name; after the first list: only dimensions; scalar/`none`/`str`; `json` without budget: next bracket must be `[..`; grouped type; map braces still needing `[..Size]`; after `?`: only `|`; after `|`: a base must follow); expression detail after an operand (bare Name: `::`, `{`, lambda `:`, named-arg `=` may follow; qualified name: `{` may follow; a bracket list fitting TypeArgs and Index; one fitting only TypeArgs: only `{` may follow; a scalar keyword: only `(` may follow; plain operand); whether `not` may start the next operand; whether `if`/`for` may (a whole Expr expected); whether a comparison was seen in this CmpExpr; whether nothing is consumed yet; whether the item is exactly one Name.

Bracket frame: allowed forms (type arguments / dimension / budget-only `[..Size]` / index), phase (before first item, after it, after a comma, after `..`, after `^`), the first item's two readings, item count. Paren frame: which of FnType params / grouped type / paren expr / lambda params still fit; phase; last item's readings and bare-name flag. Args frame: positional or named; phase; last argument's reading and bare-name flag. Brace frame: kind decided by the token before `{` (fields / JSON object / map); phase; for a map whether map-type and map-literal readings still fit, and the key's two readings while the value is read. JSON frames: phase; after `json [` a frame holding both readings for exactly one token. Array / if / for: phase only.

**Where each decision falls:** `{` at the brace by the previous token (§5.1); map type vs map literal by the entries (a second entry, trailing comma, non-type key/value kill the type; non-expression kills the literal) and at the token after `}` (type survives only into `[..`). `json` at the token after it (§3.11), the `[..` case at the second token. A bracket list after a name at `]`: `^`/`..` after an item/two bounds → index only; comma → arguments only; leading `..` → dimension or slice; one item → by its readings; later lists dimensions only (§6.8). `::` keeps both, a Name must follow. `?`/`|` kill the expression at once; a second `?` kills the type. `.`, call `(`, every operator, `??`, `and`, `or`, `is` kill the type at once; `(` after a scalar keyword is a conversion. `(`: readings narrowed by items (`:` after a bare name, comma, non-name item), decided at the token after `)`: `->` → FnType or lambda result, `:` → lambda, else group/paren-expr. `is` operand: nested item; if its expression reading survived, the comparison is complete and the expression goes on; if only its type reading survived, only `and`/`or` may follow. End: root `(T, E)`.

**Comparison** (`compare.py`): every sequence of length ≤ 6 in every position; both machines deterministic, so each (reader state, recogniser pair state) is expanded once per remaining depth; where the recognisers are dead the reader is run alone (must stay "neither"), where the reader is dead the recognisers are run alone (must never accept). Covers all 58^n sequences for n ≤ 6:

| position | distinct state pairs | disagreements |
|---|---|---|
| free sequence (Type vs Expr) | 356,488 | 0 |
| after `is` (AltType vs Fallback) | 247,994 | 0 |
| type argument (Type vs Expr) | 356,488 | 0 |
| first bracket list (TypeArgs vs Dim) | 56,799 | 0 |

**No disagreement up to length 6 in any position**, and 0 reader-alone states (the reader dies exactly where both recognisers do). During development the check found one disagreement, `Name ? ?` (`a??`) read as a type — fixed; and one place where the reader outlived the grammar harmlessly (`a((b, 1`: after the comma left only lambda parameters it accepted `1` and died at `)`) — tightened.

## 5. Not covered

§2.5's keyword-as-member-name rule (after `.`, record keys, attribute clauses): not applied, keywords stay keywords (so §3.11's "does not follow `.`" case never arises). §5.10's prose that a one-line `for` expression as operand/argument/start/iterable stands in parentheses: not applied; `Expr` counts include `[for …]`, `f(for …)`; no effect on the overlap. `Result` assumed `'->' Type`; `ForExprBlock`, `match`, layout tokens, attribute clauses, patterns, declarations: outside the named productions. Lexical level: literals are single tokens; `JsonString`/`JsonNumber` vs `String`/`Number` left to a lexer that must itself apply §3.11 (the test tokenizer does, for the examples). Everything semantic. Lengths past N (counts 6, intersections 7, comparison 6); longer shapes checked individually only.

## 6. Observations worth a ruling

* `x is {a: b}[..4]` (and `…[..4][2]`) is both a map type with its capacity and a sliced map literal; §5.3 gives `..` as the condition for the map type without excluding the literal. If "only when" means "exactly when", the text should rule the slice out there.
* §3.2's `Base` gives `none` dimensions (`none[4]`, `none[..4]` are types); §5.1's `AltType` does not — `none[1]` is "both" free and expression-only after `is`.
* By the productions alone a `TypeArg` and a `Size` may be any expression, so `a["s"]`, `a[b ?? 1]`, `a[if b then 1 else 2]`, `a[json 1]` are types as well as expressions: the overlap is the whole of `Expr` inside one-item bracket lists after a name.

## What the report changes in the proposal

The run agrees with the proposal's section (2) for the first bracket list after a name in a type (the overlap is exactly one item that is an expression), for the right operand of `is` (a name or qualified name with bracket lists of one expression each, `..` before it allowed, and `{a: b}[..c]` left to the batch's wording of §5.3), and for a type argument (the same, with `none`, the parenthesised forms with and without dimensions, and the one-entry brace group with `[..N]`); and with section (5)'s decision points (`{` by the token before it; a bracket list at its closer, with `{` after the first list making a record literal; a parenthesis at the token after its closer, `->` or `:`; a brace group after `is` at the two tokens after it; `json` at the next token, and `[..` at the second). Its single-pass reader is a stack machine with one frame kind a construct and no backtracking, with no disagreement against the recognisers over every sequence of up to six tokens in every position; whether the Rust form the one-home ruling asks for follows from it is the structure proposal's to show. Two things it adds: `none[4]` is a type (§3.2) and not an `AltType` (§5.1), so after `is` it is the value compare alone while elsewhere it is neutral, an asymmetry of the SPEC's own productions for the owner; and `x is none` fits both forms by the productions, which §5.3 settles by its sentence, so the parser hands it on as the test. The proposal's section (2) and its question 7 were amended for the first; nothing else in it changed.
