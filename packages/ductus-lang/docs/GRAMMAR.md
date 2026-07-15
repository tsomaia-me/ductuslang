# Ductus Surface Grammar

## 0. Introduction

This document specifies the **surface grammar** of the Ductus language: the lexical and syntactic rules that govern the translation of a UTF-8 source-text stream into an Abstract Syntax Tree (AST).

**Scope.** Surface syntax only. This file covers stages 1–2 of a standard compiler front-end:

1. **Lexical analysis** — character stream to token stream, with layout pre-processing (INDENT/DEDENT synthesis) and string-interpolation lexer-mode switching.
2. **Syntactic analysis** — token stream to AST, with attached disambiguator notes for cases where context-free grammar alone is ambiguous.

**Out of scope.** Everything post-parse lives in `SPEC.md`:

- Name resolution (which symbol does a path refer to)
- Type checking, type inference, value-fits checks
- Trait coherence, orphan rules
- Ownership and borrow checking
- Reactive provenance, dependency tracking
- Const-evaluation, monomorphization
- Hot-reload mechanics, IR generation, runtime contract

**Companion document.** The IR text-form grammar (the contract between frontend and runtime) is `IR_GRAMMAR.md`; this file does **not** cover the IR.

**Non-normative status.** This document is **non-normative**. It is a *derived reformulation* of rules stated normatively in the SPEC; on any conflict the SPEC **prevails**, and any divergence between this document and the SPEC is a **defect** in this document (to be repaired here, not in the SPEC). This contrasts with `IR_GRAMMAR.md`, which *is* normative for the IR text form.

**Source documents.** This grammar is a reformulation of rules already established normatively in:

- `SPEC.md` — the language specification (sections referenced as `§N.M`)
- `DECISION_LOG.md` — the per-decision log (entries referenced as `NNN-MM`)

The grammar is a re-formulation, not a re-derivation. Readers needing the rationale for a rule follow the source pointer to SPEC / LOG.

**How to read this document.** Start with §1 (Notation). §2 gives the lexical layer (tokens). §§3–13 give the syntactic layer (productions over tokens). Appendices A–D collect cross-cutting tables and disambiguator catalogs.

## 1. Notation

### EBNF symbols

Productions use a conventional EBNF flavor:

| Symbol | Meaning |
|--------|---------|
| `::=` | production / rewrite arrow |
| `|` | alternative (one alternative per line in multi-alternative blocks) |
| `?` | suffix: zero-or-one occurrence (optional) |
| `*` | suffix: zero-or-more occurrences |
| `+` | suffix: one-or-more occurrences |
| `(...)` | grouping |
| `'literal'` | exact literal token (e.g., `'if'`, `'->'`, `','`) |

### Naming conventions

| Convention | Meaning | Example |
|------------|---------|---------|
| `PascalCase` | syntactic nonterminal | `IfExpr`, `TypeExpr` |
| `UPPER_CASE` | terminal / lexeme | `IDENT`, `INT_LIT`, `STRING_LIT`, `INDENT` |
| `lowercase` | layout marker | `newline`, `indent` |

### Production-block format

```
Nonterminal      ::= Alternative1
                   | Alternative2
                   | Alternative3
                   ;  (§<spec-ref>, <log-ref>)

// Disambiguator note (attached when grammar alone is ambiguous):
// "<short rule explaining how the parser resolves the case>"
```

### Source pointers

Each production block ends with a single source pointer of the form `;  (§N.M, NNN-MM)` citing the most-authoritative SPEC section and DECISION_LOG entry underlying the rule. When multiple LOG entries underpin one production, the pointer picks the canonical (usually first or most-semantic) entry. One pointer per block; no piles.

### Disambiguator notes

When context-free grammar alone cannot resolve a parse, the production block is followed by one or more `//`-prefixed lines stating the resolution rule in parser-implementable terms. Disambiguator notes are **not** prose explanations; if a rule needs more than two lines, it is a semantic concern and belongs in SPEC.

### Cross-references within this document

Cross-references between productions use the nonterminal name (e.g., "see `TypeExpr`"). No section numbers appear in cross-references, to avoid drift if sections are renumbered.

## 2. Lexical grammar

The lexer translates a UTF-8 character stream into a token stream consumed by the parser. The lexer is responsible for: scanning lexemes, recognising keywords, classifying literals (including suffixed-literal joining for built-in suffixes), running the layout pre-processor that synthesises `INDENT` / `DEDENT` / `NEWLINE` tokens, switching modes inside string interpolations, and rejecting forbidden / reserved tokens.

Source encoding is UTF-8. The lexer reads Unicode scalar values; the term *character* below refers to a Unicode scalar value.

### 2.1 Layout pre-processor (INDENT/DEDENT synthesis)

Ductus is a layout-sensitive language: indentation opens and closes blocks. The lexer produces synthetic `INDENT`, `DEDENT`, and `NEWLINE` tokens from the leading whitespace of each *logical line* using an indent stack. This subsection specifies the algorithm normatively.

**Inputs.** A character stream split into physical lines on `'\n'`. A *logical line* is the sequence of characters from the start of a physical line to a `'\n'` that occurs at *bracket-nesting depth zero* — where bracket depth is incremented by `(`, `[`, `{` and decremented by their closers (see §2.9 for the string-interpolation case). Newlines inside `(...)`, `[...]`, `{...}` (the latter only inside an interpolation expression — see §2.9) are not logical-line terminators; they are whitespace.

**State.** A stack `S` of nonnegative integers, the *indent stack*, initialised to `[0]`. The top of `S`, written `top(S)`, is the current block's indent column. A bracket-depth counter `D`, initialised to `0`, persisted across all physical lines for the entire input (it is *not* reset at logical-line boundaries — it only reaches zero again when every opener has been matched by its closer).

**Logical-line assembly (driver).** Before applying the algorithm below, the lexer assembles each logical line `L` as follows: starting at the next unread physical line, scan characters and update `D` for every `(`, `)`, `[`, `]`, `{`, `}` encountered *outside* string-literal and `//`-comment regions — see §§2.5, 2.8, 2.9. The driver must execute the STR/EXPR mode automaton of §2.9 while scanning, because interpolated `\{...}` expressions inside strings may themselves contain nested `(`/`[`/`{` and further nested string literals; brackets *inside* an interpolation expression participate in the same `D` counter (so newlines inside such an expression are also absorbed when `D > 0`), but brackets inside a `StrChunk` (non-interpolated string text) do not. On reaching a `'\n'`: if `D > 0`, concatenate the following physical line onto `L` (the `'\n'` becomes whitespace inside `L`) and continue scanning; if `D == 0`, the `'\n'` terminates `L`. The assembled `L` — possibly spanning many physical lines — is then handed to step 1. The driver maintains `D` across logical-line iterations as well, so that no INDENT/DEDENT/NEWLINE is ever emitted while inside brackets.

**Output.** A merged token stream: the lexer's content tokens interleaved with synthetic `INDENT`, `DEDENT`, `NEWLINE`, and `EOF` tokens.

**Algorithm (normative).**

```
on EOF:
  emit NEWLINE                   # close last open logical line, if any
  while top(S) > 0:
    pop S; emit DEDENT
  emit EOF
  halt

for each logical line L:
  # 1. Layout-irrelevant line filter.
  if L is blank (only whitespace) or contains only a // comment:
    skip L (no tokens emitted)             # 002-24
    continue

  # 2. Compute indent column.
  let W = leading whitespace of L
  if W contains a tab character:
    LEX ERROR: "tab in leading whitespace"   # 002-22
  let col = length(W)                        # ASCII spaces only

  # 3. Compare against indent stack.
  if col > top(S):
    push col onto S
    emit INDENT
  else:
    while col < top(S):
      pop S; emit DEDENT
    if col != top(S):
      LEX ERROR: "indent does not match any enclosing block"

  # 4. Tokenize content of L (everything after W up to the line-terminator).
  scan L's content into tokens (per §§2.3–2.14); emit them in order.
  # By construction of the driver, L is already a *complete* logical line:
  # any physical-line newlines whose enclosing bracket depth D was > 0 were
  # absorbed as whitespace by the assembly step, and L terminates only at a
  # newline where D == 0. (See §2.9 for the string-interpolation case.)

  emit NEWLINE
```

;  (§1.4, 002-17)

// `INDENT`, `DEDENT`, `NEWLINE` are synthetic tokens — they never appear in source.
// `NEWLINE` separates statements within the same block; the parser may treat it as
//  equivalent to a comma in list-bearing contexts where the grammar admits both (002-18).
// Mixed tabs/spaces in non-leading whitespace are not regulated by the layout rule.

### 2.1.1 Inline-body Expr termination (normative addendum)

The `':' InlineExpr` form of a body (see §2.2, §5.17, §5.18, §5.19,
§7.6, §7.13, §11.1) terminates the inline `Expr` at the **lowest-
precedence expression boundary** — whichever of the following comes
first under the precedence table of §4.4.7:

- a `NEWLINE` at layout-active bracket depth zero (`D == 0`);
- the enclosing layout-suspending closer `')'`, `']'`, or `'}'`
  (looked ahead, not consumed) when inside `(...)` / `[...]` /
  string-interpolation `{...}`;
- a `','` at the enclosing bracket level that terminates a list element
  or argument;
- a `':'` introducing the next body in a chained construct (e.g. an
  `IfArmBody` followed by `'else'`).

This rule lets inline bodies appear inside layout-suspended contexts
where SPEC examples elide a trailing NEWLINE — e.g.
`items |> map(fn(p): p.title)` — without breaking the same
production's use at top-line bracket depth zero.

;  (§1.4, 002-25)

// All inline-body productions of this document (`Body`, `BlockExpr`,
//  `IfArmBody`, `MatchArm`, `FnBody`, `ClosureLit`, `OperatorBody`,
//  `LoopElseClause`, `BodyIntro`) treat their inline-form `Expr`'s
//  extent as governed by this rule.

### 2.2 Body shape rule (always-indented vs may-be-inline)

The colon `:` opens a body. Two body shapes exist; which is admissible depends on the *owning construct*, not on the lexer:

```
Body              ::= ':' INDENT BlockBody DEDENT
                    | ':' InlineExpr
                    ;  (§1.4, 002-25)

// "always-indented" constructs (only the first alternative is legal):
//   trait, type, enum, node, connection bodies (002-25).
// "may-be-inline" constructs (either alternative is legal):
//   fn bodies, if/else arms, match arms (002-26).
// The lexer emits the same token sequence for both — the parser dispatches
//  on the owning construct's category. INDENT/DEDENT only appear after
//  the colon when the next non-skipped logical line is at a greater indent.
```

### 2.3 Identifiers and `#`

```
IDENT             ::= IdentStart IdentCont*
                    ;  (§1.4, 002-21)

IdentStart        ::= UnicodeLetter
                    | '_'
                    | '#'
                    ;  (§1.4, 002-15)

IdentCont         ::= IdentStart
                    | UnicodeDigit
                    ;  (§1.4, 002-21)

UnicodeLetter     ::= any character with Unicode `Letter` property
                    ;  (§1.4, 002-21)

UnicodeDigit      ::= any character with Unicode `Decimal_Number` property
                    ;  (§1.4, 002-21)
```

// `#` behaves as a letter in every identifier position — leading, infix, terminating.
//  Examples: `#default`, `audio#main`, `note#`. (002-15)
// An identifier may not begin with a digit; `2nd` is invalid. (002-21)
// Longest-match: the lexer extends an `IDENT` to the longest valid character run.
// After scanning, the lexer compares the resulting lexeme against the keyword
//  inventory (§2.4) and reclassifies as the corresponding `KW_*` token if matched.

### 2.4 Keyword inventory

Keywords are reserved in every position; no keyword spelling may be used as an ordinary identifier (002-27). All keyword spellings are lowercase (002-2). The complete inventory follows.

**Declaration keywords** (002-3; `use` per 003-50, `wraps` per 007-11, `alias` per 009-102, `sealed` per 005-241):

```
'node'  'connection'  'trait'  'type'  'fn'  'operator'  'effect'
'signal'  'attr'  'recurrent'  'derived'  'stream'  'yielded'  'view'
'const'  'let'  'mut'  'repeat'  'main'  'collect'
'use'  'wraps'  'alias'  'sealed'
```

// `sealed` is a trait-declaration modifier (`sealed trait …`, §3.7.6,
//  005-239): it restricts fulfillment claims to the trait's declaring
//  module. It is a reserved word in every position (005-241, 002-27).
//  Its listing in this box is inventory only — the keyword-class
//  assignment is deferred to the keyword-class taxonomy (005-241); it
//  is not thereby fixed as a declaration keyword.

// `yielded` is the seventh declaration keyword (016-1); it heads a
//  body-only named group declaration (`yielded <name>: <MemberType> =
//  collect:`, §8.9.1). `collect` heads the collect block expression —
//  written standalone or as the right-hand side of a `yielded`
//  declaration (002-3, 034-1). `yield` (control-flow block below) is
//  legal only directly under a `collect` (002-6, 034-4).

// `fulfill` is a *clause keyword only* (per LOG 002-4) — see the
//  clause-keyword block below. It is *not* a declaration head: a
//  fulfill body is introduced by the `fulfill` clause attached to a
//  `trait` head (§7.12).

**Visibility keywords** (002-3):

```
'public'  'shared'  'private'
```

**Ownership / coercion keywords** (`own`/`move` per 001-18; `dyn` per 004-47, 008-7):

```
'own'  'move'  'dyn'
```

**Clause keywords** (002-4):

```
'children'  'incoming'  'outgoing'  'expose'  'when'
'satisfies'  'fulfill'  'default'  'otherwise'
'from'  'to'  'pairs'  'on'  'where'  'desired'  'observed'
'ring'  'gate'  'keyed'  'at'  'dynamic'
```

// `from`, `to` also serve as reserved instance-field names — see the
//  `ReservedInstanceField` production below. `incoming` / `outgoing` head
//  node-body acceptance clauses; they are clause keywords only (002-5).

**Control-flow keywords** (002-6):

```
'if'  'else'  'match'  'for'  'in'  'while'  'break'  'continue'  'return'  'yield'
```

**Scope-anchor namespace keywords** (002-7):

```
'here'  'module'  'root'  'std'
```

// `std` and `root` are scope-anchor segment heads (per 003-22 / 002-7):
//  `root::` denotes the project root namespace and `std::` denotes the
//  stdlib root. A bare path-base name matching a manifest-declared
//  external dependency is admitted via the same `PathSegment` IDENT
//  alternative (post-parse resolution).

**Instance-value keyword** (002-8):

```
'subject'
```

**Naming/alias keyword** (002-9, 002-10):

```
'as'
```

// `as` is *not* a cast operator (002-10); explicit conversion uses `T(value)` (§5.6).

**Operator-context keywords** (002-11):

```
'is'  'and'  'or'  'not'  'handle'  'handle!'  'portal'  'delete'
```

// `panic` is *not* a keyword — it is an ordinary prelude function
//  `panic(msg) -> never` (per LOG 011-23 / 011-24). The spelling lexes
//  as `IDENT` and parses through `Path` + `CallSuffix` (§5.27 documents
//  the shape; the production is documentation-only).

// **`where` role overloading.** `where` appears (a) as a declaration
//  clause after a signature head (§3.13), (b) as the binary
//  stream-filter operator `A where C` (§5.23, tier 0b), and (c) as a
//  per-arm filter inside an observe `on`-clause (§5.20). The lexer
//  emits a single keyword token; syntactic position disambiguates.

// `handle!` is a single lexer token (002-11). The two-character sequence
//  `handle` immediately followed by `!` is recognised as one keyword;
//  there is no separate `handle` `!` parse.

```
// Keyword: the lexeme set is the union of all spellings in the boxes
// above (declaration / visibility / ownership / clause / control-flow /
// scope-anchor / instance-value / naming-alias / operator-context).
// There is no separate concrete production — the lexer matches each
// spelling individually after the longest-match IDENT scan (§2.3).
//                                                 ;  (§1.4, 002-3)
```

// The sole reserved capitalised type identifier is `Subject` (002-12). It is a
//  type alias, not a keyword — it is lexed as an `IDENT` and resolved per §13.1.
// Header-continuation keywords `else` and `else if` attach to their owning `if`
//  by column-alignment, not by indentation depth (002-30); see Appendix D.

**Reserved instance-field names** — spellings that the lexer emits as ordinary
`IDENT` tokens but which the parser / resolver treats as compiler-assigned
field names in connection / node body contexts:

```
ReservedInstanceField ::= 'from' | 'to' | 'pair' | 'exposition'
                        | 'desired' | 'observed'
                        ;  (§1.4, 002-5, 002-28)
// Repeated as a free-standing production in §13.4 with the same RHS;
// the duplication is intentional — readers reach the production from
// either §2 (lexical) or §13.4 (reserved-identifier reference).
// `desired` and `observed` are simultaneously clause keywords (heading
//  `desired:` / `observed:` blocks inside an `effect` body, §7.14) and
//  reserved instance-field names (per 002-28).
```

// `from` and `to` are simultaneously clause keywords (§2.4 clause-keyword
//  block) and reserved instance-field names — the role is positional:
//  header position → clause keyword; body-reference position → field name
//  (002-5).
// `pair` and `exposition` are *not* clause keywords; they exist only as
//  reserved instance-field names with compiler-assigned meaning in
//  connection / node body contexts (002-5, 002-28).
// `incoming` and `outgoing` are clause keywords only — they head node-body
//  acceptance clauses whose named entries join the instance-body namespace
//  shared with cells, views, and placement `as`-names (002-5). They are
//  *not* reserved instance-field names.
// User code may not declare a field with any of the spellings listed in
//  `ReservedInstanceField`.

### 2.5 Comments

```
LineComment       ::= '//' ( any character except '\n' )* ( '\n' | EOF )
                    ;  (§1.4, 002-19)
```

// `//` starts a comment that runs to end-of-line. There is no block-comment
//  form (002-19). A line containing only a comment (possibly after leading
//  whitespace) is layout-irrelevant per §2.1 step 1 (002-24).
// A `//` appearing inside a string literal is text, not a comment.

### 2.6 Integer literals

```
IntLit            ::= DecIntLit
                    | HexIntLit
                    | OctIntLit
                    | BinIntLit
                    ;  (§4.3.1, 007-17)

DecIntLit         ::= Digit ( Digit | '_' Digit )*
                    ;  (§4.3.1, 007-18)

HexIntLit         ::= '0x' HexDigit ( HexDigit | '_' HexDigit )*
                    ;  (§4.3.1, 007-18)

OctIntLit         ::= '0o' OctDigit ( OctDigit | '_' OctDigit )*
                    ;  (§4.3.1, 007-18)

BinIntLit         ::= '0b' BinDigit ( BinDigit | '_' BinDigit )*
                    ;  (§4.3.1, 007-18)

Digit             ::= '0'..'9'
HexDigit          ::= '0'..'9' | 'a'..'f' | 'A'..'F'
OctDigit          ::= '0'..'7'
BinDigit          ::= '0' | '1'
```

// Underscore is a *visual separator only* and must appear strictly between
//  two digits: `0x_FF` and `1_` are lex errors (§4.3.1).
// Leading zeros in a `DecIntLit` carry no octal meaning: `007` is the
//  integer `7`. Octal requires the explicit `0o` prefix (§4.3.1).
// An `IntLit` may carry a numeric type or duration suffix per §2.10; the
//  lexer joins the digit run and a recognised built-in suffix into a single
//  `SuffixedIntLit` token. For user-defined suffixes the lexer emits the
//  digit run as an `IntLit` and the suffix as an `IDENT`; see §2.10.

### 2.7 Float literals (incl. formal Exponent production)

```
FloatLit          ::= DecIntLit '.' DecIntLit Exponent?
                    | DecIntLit Exponent
                    ;  (§4.3.2, 007-25)

Exponent          ::= ( 'e' | 'E' ) ( '+' | '-' )? Digit+
                    ;  (§4.3.2, 007-25)
```

// A digit is required on each side of the decimal point — leading-dot forms
//  like `.5` are not permitted; write `0.5` (§4.3.2).
// At least one digit is required after the optional exponent sign;
//  `2.5e` (no exponent digits) is a lex error (§4.3.2).
// Float suffixes (e.g., `f32`, `f64`) attach only to a decimal literal.
//  Hex / oct / bin literals admit no float suffix: under longest-match,
//  `0x1_f32` is the hex integer `0x1F32`, not a float (§4.3.2).
// A bare `1` is `IntLit`; `1.0`, `1e5`, `1f32` are float literals.

### 2.8 Bool, char, byte literals

```
BoolLit           ::= 'true' | 'false'
                    ;  (§4.3.4, 007-37)

CharLit           ::= "'" CharContent "'"
                    ;  (§9.1.2, 012-11)

ByteLit           ::= "b'" ByteContent "'"                  // single token; see disambiguator
                    ;  (§4.3.4, 007-40)

CharContent       ::= any single Unicode scalar value, except '\'', '\\', '\n'
                    | EscapeSeq                              // `\{` is admissible per EscapeSeq below but a no-op inside CharLit (no interpolation)
                    ;  (§9.1.2, 012-13)

ByteContent       ::= any single ASCII character in 0x00..0x7F, except '\'', '\\', '\n'
                    | ByteEscapeSeq
                    ;  (§4.3.4, 007-40)

EscapeSeq         ::= '\\n' | '\\r' | '\\t' | '\\0'
                    | '\\\\' | '\\"' | "\\'" | '\\{'         // `\{` suppresses string interpolation (§2.9)
                    | '\\x' HexDigit HexDigit                // ASCII byte 0x00..0x7F
                    | UnicodeEscape                          // \u{H[H[H[H[H[H]]]]]} per below
                    ;  (§9.1.3, 012-20)

UnicodeEscape     ::= '\\u{' HexDigit ( HexDigit ( HexDigit ( HexDigit ( HexDigit HexDigit? )? )? )? )? '}'
                    ;  (§9.1.3, 012-20)

ByteEscapeSeq     ::= '\\n' | '\\r' | '\\t' | '\\0'
                    | '\\\\' | '\\"' | "\\'"
                    | '\\x' HexDigit HexDigit
                    ;  (§4.3.4, 007-40)
```

// `true` / `false` are reserved spellings; the lexer reclassifies them from
//  `IDENT` (§9.1.1). They are not numeric and do not participate in numeric
//  trait dispatch.
// A `CharLit` contains exactly one Unicode scalar; multi-character literals
//  are a lex error (§9.1.2). The `\{` escape is *not* admitted in `CharLit`
//  — it is an interpolation escape that only appears in string literals.
// `\xHH` in any context here denotes a single byte in the ASCII range
//  `0x00`–`0x7F` (one-byte UTF-8 scalar). `\x80`-or-higher is a lex error
//  (§9.1.3).
// `\u{HHHHHH}` accepts 1–6 hex digits; surrogate code points
//  `\u{D800}`–`\u{DFFF}` and values above `\u{10FFFF}` are lex errors
//  (§9.1.3, §9.1.2).
// `b'...'` denotes a `u8` byte literal; its content is restricted to ASCII
//  (§4.3.4).
// **`ByteLit` vs `IDENT` starting `b`** (per §4.3.4). The two-character
//  prefix `b'` is recognised as the byte-literal opener only when an
//  ASCII content character and a closing `'` follow under longest-match.
//  An ordinary identifier such as `bb` is lexed as `IDENT` because the
//  character immediately after the leading `b` is not `'`. An identifier
//  cannot begin with `b'` because `'` is not an `IdentCont` character.

### 2.9 String literals + interpolation lexer mode

String literal lexing requires a small mode-switching machine: inside a string the lexer is in **STR** mode, where most characters become string content; an unescaped `{` enters **EXPR** mode (an interpolated expression); the matching `}` returns to **STR** mode.

```
StringLit         ::= '"' StrChunk* '"'
                    ;  (§9.1.3, 012-19)

StrChunk          ::= StrText                               // literal content
                    | EscapeSeq                              // \n, \t, \xHH, \u{...}, etc.
                    | '\\{'                                  // literal '{' (interpolation escape)
                    | '{' InterpExpr '}'                     // interpolated expression
                    ;  (§9.1.3, 012-19)

StrText           ::= any sequence of characters except '"', '\\', '{'
                    ;  (§9.1.3, 012-22)
```

**Lexer mode automaton (normative).**

```
state STR (inside a string, depth d ≥ 1):
  on '"'   : emit STR_END, exit STR, return to outer mode
  on '\\{' : emit STR_TEXT "{", stay in STR
  on '{'   : emit STR_INTERP_OPEN, push outer mode, enter EXPR (interp depth d+1)
  on '\\<escape>' : recognise EscapeSeq per §2.8, emit STR_ESCAPE
  on '\n'  : append to STR_TEXT (literal newline permitted, §9.1.3)
  on EOF   : LEX ERROR "unterminated string"
  otherwise: append to STR_TEXT

state EXPR (inside an interpolation, paired with STR depth d):
  run normal lexer rules (§§2.3–2.14)
  track bracket depth as usual: '(', '[', '{' push; ')', ']', '}' pop
  on '}' at bracket depth zero of this interpolation:
    emit STR_INTERP_CLOSE, exit EXPR, return to STR at depth d
```

;  (§9.1.9, 012-42)

// `\{` is the *sole* literal-brace escape (§9.1.9). There is no `{{` / `}}`
//  doubling form. A `\{` in `StrChunk` emits a literal `{` and suppresses
//  interpolation at that position.
// Interpolation expressions are full Ductus expressions (§5.12). They may
//  contain string literals (which re-enter STR mode) and thus may nest
//  to arbitrary depth via the mode-stack; the grammar imposes no special
//  limit. The parser-side production for an interpolation expression is
//  in §5.12; this section only specifies the lexer-mode transition.
// A `}` that arises from an inner-bracket close inside the interp expression
//  does *not* exit STR — only a `}` at this interpolation's bracket depth
//  zero closes the interpolation.
// Layout is suspended inside a `StringLit` (002-23): newlines inside the
//  string are content, not logical-line terminators. The bracket-depth
//  rule of §2.1 treats `"..."` as an opaque region.
// There is no format-specifier mini-language: `{expr}` always formats via
//  `Display` (§9.1.9). Width / precision are produced by method calls
//  inside the expression.

### 2.10 Numeric suffix lexing (built-in lexer-resolved + user-defined raw)

A literal suffix is the suffix name appended directly to the literal with no separator. The lexer's treatment depends on whether the suffix is *built-in* or *user-defined*.

```
SuffixedIntLit    ::= IntLit IntBuiltinSuffix           // single token; integer-typed
                    ;  (§4.3.3, 007-31)

SuffixedFloatLit  ::= FloatLit FloatBuiltinSuffix       // single token; float-typed
                    ;  (§4.3.3, 007-31)

IntBuiltinSuffix  ::= IntegerTypeSuffix
                    | DurationSuffix
                    ;  (§4.3.3, 007-36)

FloatBuiltinSuffix ::= FloatTypeSuffix
                    | DurationSuffix
                    ;  (§4.3.3, 007-36)

IntegerTypeSuffix ::= 'i8'  | 'i16' | 'i32' | 'i64' | 'i128'
                    | 'u8'  | 'u16' | 'u32' | 'u64' | 'u128'
                    | 'isize' | 'usize'
                    ;  (§4.3.3, 007-36)

FloatTypeSuffix   ::= 'f32' | 'f64'
                    ;  (§4.3.3, 007-36)

// `NumericTypeSuffix` is the *union* of `IntegerTypeSuffix` and
//  `FloatTypeSuffix` and exists only as a documentation alias — it
//  is not referenced from any production:
NumericTypeSuffix ::= IntegerTypeSuffix | FloatTypeSuffix
                    ;  (§4.3.3, 007-36)

DurationSuffix    ::= 'ns' | 'us' | 'μs' | 'ms' | 's' | 'min' | 'h' | 'd'
                    ;  (§4.3.3, 007-33)
```

// Built-in suffixes are *lexer-resolved*: the lexer emits a single
//  `SuffixedIntLit` / `SuffixedFloatLit` token covering `<NumberLiteral><suffix>`
//  with no whitespace between (§4.3.3). The numeric-type suffix pins the
//  literal's type (§4.3.1, §4.3.2); a duration suffix runs the corresponding
//  compile-time constructor (§4.3.3).
// A `FloatLit` may carry a `NumericTypeSuffix` only when that suffix is
//  `f32` or `f64`. Integer numeric-type suffixes on a float literal are a
//  lex / type error per §4.3.2.
// `NumericTypeSuffix` and `DurationSuffix` only attach to *decimal* float
//  literals; under longest-match they are absorbed as digits of hex
//  literals — `0x1_f32` is the hex integer `0x1F32`, not a float (§4.3.2).
// User-defined `@literal_suffix` suffixes are *not* lexer-joined: the lexer
//  emits the digit run as `IntLit` or `FloatLit` and the suffix as a
//  separate `IDENT`. The parser joins them into a single suffixed-literal
//  expression node and resolves against the registered constructor (§3.9).

### 2.11 Negative-literal token rule (`-N` as one signed literal)

```
SignedIntLit      ::= '-' ( IntLit | SuffixedIntLit )       // see disambiguator
                    ;  (§2.4.5, 004-100)

SignedFloatLit    ::= '-' ( FloatLit | SuffixedFloatLit )
                    ;  (§2.4.5, 004-100)
```

// `-N` is parsed as a *single* signed integer literal token for the purpose
//  of type-checking against signed integer target types: `let x: i8 = -5`
//  checks `-5` against `i8`'s range, not unary minus applied to `5`
//  (which would first form a positive `5`, possibly out-of-range for the
//  target). (004-100)
// This applies only at literal sites — in any context where an `IntLit` or
//  `SuffixedIntLit` immediately follows a leading `-`, the parser unifies
//  the two into a `SignedIntLit` and applies value-fits checking against the
//  signed target type. Outside literal contexts, `-` is the unary-minus
//  operator (§5.4).
// The rule does *not* apply to runtime values: unary `-` on an unsigned
//  runtime value remains a type error (§4.4.1.2).

### 2.12 Operator and punctuation tokens

The lexer recognises the following operator and punctuation tokens. Longest-match applies — `<<` is a single token, `<` `<` two tokens; `->`, `=>`, `..`, `::`, `?.`, `?[`, `?(`, `|>`, `<=`, `>=`, `<<`, `>>`, `\` (single backslash, integer division), etc. are each one token.

```
PunctOrOp         ::= '(' | ')' | '[' | ']' | '{' | '}'
                    | ',' | ':' | '.' | '..'
                    | '->' | '=>' | '::'
                    | '@' | '#' | '$'                    // # outside identifiers reserved; $ placement-flag char (§13.8.8)
                    | '?' | '?.' | '?[' | '?('
                    | '!'                                // standalone attribute-false / flag char
                    | '|' | '|>'
                    | '&' | '^' | '~'
                    | '+' | '-' | '*' | '/' | '\\' | '%'
                    | '+%' | '-%' | '*%' | '\\%' | '%%'
                    | '+|' | '-|' | '*|' | '\\|' | '%|'
                    | '+?' | '-?' | '*?' | '\\?' | '%?'
                    | '<' | '<=' | '>' | '>='
                    | '<<' | '>>'
                    | '='
                    ;  (§4.4, 007-57)
```

// Single-character `&` and `|` are reused at the type level (`&` for trait /
//  record intersection; `|` for the placement attribute clause's leader) and
//  at the value level as bitwise operators. The parser resolves by position
//  (§4.4.2); the lexer emits the same token in all cases.
// `|>` is a distinct token from `|` and is *not* a bitwise operator. It is
//  the operator-application token (§13.17); see §5.25.
// The backslash `\` is the integer-division operator at the value level
//  (§4.4.1.2) and the escape character inside string and char literals
//  (§9.1.3); position disambiguates.
// `?` is the postfix Try operator (§5.2), the cast-policy marker
//  `T?(value)` (§5.6), the leader of optional-chaining tokens `?.` / `?[` /
//  `?(` (§5.3), and a flag character in placement flag runs (§13.8.8); the
//  parser resolves by position.
// **Lexer split for cast-policy `T?(`** (per §5.6). When `?` follows a
//  type-name `PathSegment` (the previous token is a name in path position
//  that lexically *could* be a type name) and is immediately followed by
//  `(`, the lexer emits two tokens — `?` then `(` — rather than the
//  merged `?(`. Optional-chaining `?(` is emitted only when the preceding
//  token completes a value-typed `PostfixExpr` head. The longest-match
//  rule is overridden in this single case by the look-back to the
//  preceding token's lexical category. The same look-back applies to
//  `?.` and `?[` (i.e. `T?.foo` and `T?[X]` are not currently used by
//  any production but the lexer splits the same way for consistency).
// `!` outside the `handle!` keyword is the inline attribute-false marker
//  (§11.5) and a flag character in placement flag runs (§13.8.8); the
//  lexer emits `!` as a standalone punctuation token. The two-character
//  sequence `handle!` is lexed as a single `KW_HANDLE_BANG` keyword
//  (002-11).
// `@` is the directive prefix in every non-placement position
//  (§1.4, applied directives `@derive` / `@literal_suffix` / `@flag` /
//  `@reset_on_reopen` / `@reset_on_reload` / `@default`, and standalone
//  `@content`); inside a placement flag run (§13.8.8) it is a flag
//  character instead. The lexer emits `@` as a standalone punctuation
//  token in all cases; the parser resolves the role by position
//  (021-120).
// `==`, `!=`, `>>>` are *not* in the table — see §2.13.

### 2.13 Reserved-but-unused tokens (`==`, `!=`, `>>>`) — lexer rejects

```
ReservedToken     ::= '==' | '!=' | '>>>'
                    ;  (§4.4.4, 007-86)
```

// `==` and `!=` are not equality operators; equality is written `a is b`
//  and `a is not b` (the latter desugars to `not (a is b)` per 007-197).
//  The two-character sequences `==` and `!=` are reserved against future
//  use; the lexer recognises them and emits a *lex error* directing the
//  user to `is` / `is not` (007-86, §4.4.4).
// `>>>` does not exist; `>>` is a single operator whose behaviour depends
//  on the signedness of the left operand's type (§4.4.2). The
//  three-character sequence `>>>` is reserved against future use and
//  rejected by the lexer (007-73, §4.4.2).
// The lexer's rejection produces a diagnostic, not a token — the rule is
//  enforced at lex time so the parser never sees these forms.

### 2.14 Forbidden tokens (`;`) — lex error

```
ForbiddenToken    ::= ';'
                    ;  (§1.4, 002-16)
```

// `;` is not a token in the grammar and may never appear in Ductus source —
//  not as a statement terminator, not as a list separator, not in generic
//  parameter lists (002-16, §1.4). A `;` in Ductus source is a lex error.
// Every separated list uses comma, newline, or both (002-18). Statements
//  within a block are separated by `NEWLINE` (§2.1).
// This rule governs Ductus source only; embedded host-driver pseudocode
//  and host-language comparison snippets shown in SPEC follow their own
//  language's rules.

## 3. Type expressions

Productions for the surface syntax of type expressions: primaries, instantiations, function and operator types, tuples, arrays, slices, `dyn`, intersections, generic parameter lists, `where` clauses, turbofish.

`TypeExpr` is the entry-point nonterminal for a type expression in any type-position context (binding annotation, parameter type, return type, field type, generic argument, etc.). It is built from a `TypePrimary` extended with the postfix `[…]` instantiation / array / slice forms, with the prefix `dyn` form for trait objects, and with `&` for intersections in the positions where intersection is admitted.

```
TypeExpr          ::= DynType
                    | IntersectionType
                    | PostfixType
                    ;  (§5, 005-44)

PostfixType       ::= TypePrimary TypePostfix*
                    ;  (§9.3.2, 005-44)

TypePostfix       ::= '[' GenericArgList ']'                  // generic-inst | array | slice
                    | TypeAssocAccess                          // associated-type projection T.Item
                    ;  (§9.3.2, 005-44)

TypeAssocAccess   ::= '.' IDENT                                // navigates a trait's associated type
                    ;  (§5.6, 005-128)
```

// **Associated-type access (per §5.6, 005-128).** A `TypeExpr` may
//  carry `.IDENT` chains in type position to navigate associated
//  types — e.g. `P.Item`, `T.Iter.Item`. The leading head `P`/`T` is
//  typically a generic parameter; resolution against the trait's
//  associated-type bindings is a post-parse semantic step. The same
//  `.` token serves field projection in expression position (§5.2);
//  the type-vs-value context discriminates.

// `TypeExpr` is a single uniform nonterminal even though several distinct
//  type-system constructs (records, traits, enums, primitives, tuples, etc.)
//  share its surface. The kind of a given `TypeExpr` is determined post-parse
//  by name resolution against the symbol table; the parser does not branch
//  on kind. The intersection / dyn / tuple shape constraints (§5.5, §5.2.1)
//  are semantic and reported after parsing.
// The `[…]` postfix is a *single* surface production that covers three
//  semantically distinct cases — generic instantiation, array type, slice
//  type — disambiguated by the kind of the head `TypePrimary` and the
//  shape of the argument list, per §3.2 / §3.6 / §3.7 below.

### 3.1 Type primary (path, primitive, wildcard `_`)

```
TypePrimary       ::= TypePath
                    | PrimitiveTypeName
                    | TypeWildcard
                    | TupleType
                    | FnType
                    | OperatorType
                    | '(' TypeExpr ')'
                    ;  (§1.4, 002-1)

TypePath          ::= TypePathSegment ( '::' TypePathSegment )*
                    ;  (§10.2.3, 029-24)

TypePathSegment   ::= IDENT
                    | 'here'
                    | 'module'
                    | 'root'
                    | 'std'
                    ;  (§10.2.3, 002-7)

// `Subject` (capitalised) is reachable via the `IDENT` alternative —
//  it is lexed as an ordinary identifier per 002-12. A bare leading
//  segment matching a manifest-declared external dependency name is
//  also admitted via the `IDENT` alternative (per 003-22, resolved
//  post-parse against the manifest).

PrimitiveTypeName ::= 'i8' | 'i16' | 'i32' | 'i64' | 'i128' | 'isize'
                    | 'u8' | 'u16' | 'u32' | 'u64' | 'u128' | 'usize'
                    | 'f32' | 'f64'
                    | 'bool' | 'char' | 'string' | 'never'
                    ;  (§4.1, 007-1)

TypeWildcard      ::= '_'
                    ;  (§2.2.6, 004-32)
```

// `Subject` is a reserved capitalised identifier (002-12); it lexes as an
//  ordinary `IDENT` and the parser recognises the spelling in type-path
//  position. It is *not* a keyword (002-12, §13.1 of this document).
// `here` and `module` are scope-anchor namespace keywords (002-7); in
//  *path* position they introduce a name-resolution anchor (§13.3 of this
//  document). Their use as the leading segment of a `TypePath` is the
//  grammar admission; the resolver enforces context legality (§13.7.3).
// `root` and the bare-name external-dependency form (§10.2.3) are
//  path-base anchors and appear only as the leading segment of an
//  absolute `TypePath` in a `use` statement context (§7.4) or anywhere
//  an absolute type path is admitted.
// `TypeWildcard` `_` is the inference placeholder (per §2.2.6, 004-32).
//  It is admissible in any nested `TypeExpr` position — generic argument,
//  tuple component, function parameter / return — but **not** as the
//  whole annotation of a binding lacking an RHS source of context, per
//  §2.1.4 (a post-parse rule, not a syntactic rule).
// The placeholder keywords `numeric`, `integer`, `float`, `signed`,
//  `unsigned` (§1.4 line 120) are lowercase identifier spellings that
//  the parser accepts as `IDENT`; their role as built-in trait-like
//  placeholders is resolved at name-resolution time.
// The pattern wildcard `_` of §4.1 and the type wildcard `_` share a
//  spelling and are disambiguated by syntactic position (per §2.2.6,
//  004-32 final paragraph).

### 3.2 Generic instantiation `T[args]` (incl. kind-of-T disambiguator)

```
GenericArgList    ::= PositionalArgList
                    | NamedArgList
                    ;  (§2.2.5, 004-31)

PositionalArgList ::= GenericArg ( ',' GenericArg )* ','?
                    ;  (§2.2.5, 004-26)

NamedArgList      ::= NamedGenericArg ( ',' NamedGenericArg )* ','?
                    ;  (§2.2.5, 004-31)

GenericArg        ::= TypeExpr
                    | ConstGenericArg
                    | '_'                                       // wildcard hole
                    ;  (§2.2.5, 004-28)

NamedGenericArg   ::= IDENT '=' ( TypeExpr | ConstGenericArg )
                    ;  (§2.2.5, 004-31)

ConstGenericArg   ::= Expr                                      // restricted post-parse
                    ;  (§2.5.2, 004-44)
```

// **Kind-of-T disambiguator (per §9.3.2).** The surface form
//  `T '[' GenericArgList ']'` is a single production. Its meaning is
//  resolved by the *kind of `T`* after name resolution:
//   - `T` resolves to a generic type / trait / type alias whose definition
//     declares one or more generic parameters → generic instantiation
//     (e.g. `Vec[i32]`, `Map[string, T]`).
//   - `T` is a primitive (PrimitiveTypeName) or a non-generic nominal
//     type, and the argument list contains exactly one `ConstGenericArg`
//     → array type `T[N]` (§3.6).
//   - `T` is likewise but the argument list begins with `..` →
//     slice type `T[..N]` or `T[..]` (§3.7).
//  The discrimination is by the kind of `T`, not by the kind of the
//  argument: a primitive's name is always an array-type constructor; a
//  generic type's name is always a generic-instantiation site. The
//  parser does not need to know which; it builds the same AST node and
//  the resolver attaches the semantic interpretation.
// Named and positional forms never mix in one list (004-31): the parser
//  selects `PositionalArgList` if the first argument lacks an `'=' '`
//  after its head identifier, `NamedArgList` otherwise; a later argument
//  in the other form is a parse error.
// Wildcard `_` is permitted only in positional form (004-28); a named
//  wildcard would have no parameter to skip.
// Trailing comma is admitted in both forms.
// `ConstGenericArg` is an `Expr` at the surface; the const-evaluability
//  and shape restrictions of §2.5.2 / §2.5.3 are post-parse semantic
//  checks, not grammar.

### 3.3 Function/closure type `fn(P, ...) -> R`

```
FnType            ::= 'fn' '(' FnTypeParamList? ')' ( '->' 'own'? TypeExpr )?
                    ;  (§11.10.6, 013-79)

FnTypeParamList   ::= FnTypeParam ( ',' FnTypeParam )* ','?
                    ;  (§11.10.6, 013-79)

FnTypeParam       ::= 'own'? TypeExpr
                    ;  (§11.10.6, 013-79)
```

// The structural closure / function type. Ownership-convention markers
//  (`own` before a parameter type, `own` before the return type) are part
//  of the type identity per §11.10.6: `fn(T) -> R` and `fn(own T) -> own R`
//  are distinct types (022-119).
// The `fn` head is what keeps `(P)` unambiguous (§11.10.6): a bare
//  `(T1, T2)` is a tuple type, `fn(T1, T2) -> R` is a two-parameter function
//  type, and `fn((T1, T2)) -> R` is a one-parameter function taking a
//  tuple. The parser cannot mis-attribute these because `fn` is a keyword
//  prefix.
// The `-> R` clause is optional at the surface — a `fn(...)` with no
//  arrow denotes a function returning unit `()` (parallel to a `fn` decl
//  with the arrow omitted, §9.2.3).
// `fn` types are object-safe by construction (§5.2.4 final paragraph);
//  `dyn fn(P) -> R` is always well-formed and is a `DynType` whose
//  inner is an `FnType` per §3.8 below.

### 3.4 Operator structural type `operator(P, ...) -> U`

```
OperatorType      ::= 'operator' '(' OperatorTypeParamList? ')' '->' TypeExpr
                    ;  (§13.17.13, 029-65)

OperatorTypeParamList ::= TypeExpr ( ',' TypeExpr )* ','?
                    ;  (§13.17.13, 029-65)
```

// Distinct production from `FnType` per §13.17.13. The leading keyword
//  `operator` is what discriminates the two at the parser level.
// The `-> U` clause is *not* optional on `OperatorType` (unlike `FnType`):
//  an operator always declares a return per §13.17.5 (029-93).
// **No `'own'` on operator return (per Phase D, §13.17 / LOG 029-11).**
//  Unlike `FnType` whose return slot admits `'->' 'own'? TypeExpr`,
//  `OperatorType` rejects `'own'` on the return: an operator's output
//  is always wrapped to a reactive cell (the cell wrapping is the
//  operator's identity per §13.17), so there is nothing to transfer.
//  An `'own'` token after the arrow is a parse error here.
// `OperatorType` carries reactive structure (§13.17.13) — semantic
//  restrictions (an operator-typed parameter may appear only in an
//  `operator` declaration's signature, not in a `fn`; §13.17.13) are
//  post-parse and not enforced by the grammar.
// Ownership-convention markers are not part of an operator-type
//  parameter (operators take only cell-bound and value parameters per
//  §13.17.3, never `own` parameters).

### 3.5 Tuple types (incl. unit `()` and 1-tuple `(T,)`)

```
TupleType         ::= '(' ')'                                   // unit
                    | '(' TypeExpr ',' ')'                      // 1-tuple
                    | '(' TypeExpr ( ',' TypeExpr )+ ','? ')'   // n-tuple, n >= 2
                    ;  (§9.2, 012-58)
```

// **Trailing-comma rule (per §9.2.4, 012-58).** The single-component
//  alternative `'(' TypeExpr ',' ')'` *requires* the trailing comma. The
//  comma is what discriminates the 1-tuple type `(T,)` from a
//  parenthesised type `(T)`, the latter being a `TypePrimary`
//  alternative of §3.1 with no tuple meaning. Without the comma the
//  parser takes the `'(' TypeExpr ')'` alternative and `T` retains its
//  identity.
// **Tuple-type vs parenthesised-type lookahead (per §9.2.4).** Inside
//  a type-position `'('`, the parser scans the inner `TypeExpr` and
//  then peeks: a `','` selects `TupleType`; a `')'` selects
//  `TypePrimary`'s `'(' TypeExpr ')'` alternative. No backtracking
//  beyond the single peek is required.
// The n-tuple alternative (n ≥ 2) admits an *optional* trailing comma:
//  `(T1, T2)` and `(T1, T2,)` denote the same type (012-59).
// Tuple types are structural (§9.2): the parser builds the same AST
//  whether the tuple appears at a binding annotation, parameter type, or
//  generic-argument position. No declaration site exists for a tuple
//  type.

### 3.6 Array types `T[N]`

The array type surface `T[N]` is one *interpretation* of the
`TypePrimary '[' GenericArgList ']'` production from §3.2 — selected by
the kind of `T` and the shape of the argument list. The arrangement is
made explicit here for documentation; it does not introduce a new
parser branch.

```
ArrayType         ::= TypePrimary '[' ConstGenericArg ']'       // see §3.2
                    ;  (§9.3.1, 009-115)
```

// Array typing applies (per §9.3.2, 009-129) when the head `T` resolves
//  to a primitive, a non-generic nominal type, a generic *parameter*, or
//  a tuple-typed expression — i.e. *not* a generic type / trait /
//  type-alias-of-generic — and the bracketed list contains exactly one
//  argument that is a `ConstGenericArg`.
// `T[N]` parses left-to-right as a sequence of `TypePostfix` (§3) —
//  `T[N][M]` is an M-element array of `T[N]` (§9.3.1).
// `T[0]` is a valid type (§9.3.1).
// The const-generic-expression restrictions on `N` (§2.5.2 / §2.5.3 —
//  must be compile-time-known non-negative integer of type `isize`) are
//  post-parse and not enforced by the grammar.

### 3.7 Slice types `T[..N]` / `T[..]`

The slice type surface `T[..N]` / `T[..]` is the *third* interpretation
of the `TypePrimary '[' … ']'` production from §3.2 — selected when the
bracket content begins with `..`.

```
SliceType         ::= TypePrimary '[' '..' ConstGenericArg? ']'
                    ;  (§9.3.7, 009-115)
```

// `T[..N]` is a slice of compile-time-known length `N` (§9.3.7).
// `T[..]`  is a slice whose length is runtime data (§9.3.7).
// The `..` here is the *type-level* slice marker, lexically the same
//  token as the value-level range operator (§5.8) but disambiguated by
//  position (the parser is in a type-expression context immediately
//  after `'['` of a `TypePostfix`, so it selects the slice branch when
//  the next token is `..`).
// Slice typing applies (per §9.3.2, 009-129) on the same head-`T` kind
//  side as array typing of §3.6 — a primitive, non-generic nominal type,
//  generic parameter, or tuple-typed head.
// Borrow semantics, widening to `T[..]` at parameter positions, and
//  read-only-ness are post-parse rules (§9.3.7) and not part of the
//  grammar.

### 3.8 `dyn` trait objects (parens-required for intersection)

```
DynType           ::= 'dyn' DynTrait
                    ;  (§5.2.1, 005-44)

DynTrait          ::= TypePrimary
                    | '(' IntersectionType ')'
                    | FnType
                    ;  (§5.2.1, 005-44)
```

// **Parens-required-for-intersection rule (per §5.2.1, 005-44).** A
//  `dyn` applied to a trait intersection *must* parenthesise the
//  intersection: `dyn (Drivable & Insurable)`. The bare form
//  `dyn Drivable & Insurable` parses (per the grammar) as
//  `(dyn Drivable) & Insurable` — `dyn` binds only its immediate primary,
//  and the surrounding `&` continues at the `IntersectionType` level.
//  The post-parse semantic check rejects the bare form (a `dyn`
//  trait-object is not in the intersection domain, §5.5); the parens
//  force the intended grouping.
// `dyn` accepts an `FnType` directly with no parens (`dyn fn(i32) -> i32`)
//  per §11.10.6 — closure types are always object-safe (§5.2.4 final
//  paragraph).
// `dyn` may also appear in *value* position as a coercion prefix
//  (§5.2.5); the value-position form is §5.4 of this document and is
//  syntactically distinct (it prefixes a primary *expression*, not a
//  type).

### 3.9 `Type[…]` meta-type (just generic instantiation; no special grammar)

`Type[C]` is a *value-position* type whose value is a *type* satisfying
the constraint `C` (per §5.7). It has **no dedicated grammar production**
— it parses as a `PostfixType` whose head is the identifier `Type`
and whose `[…]` argument is a `TypeExpr` (typically a trait, a concrete
type, or a `&`-intersection of traits per §5.7.1). The identifier
`Type` is an ordinary `IDENT`; no keyword status, no special parser
branch.

```
// Type[C] : no production; parses as PostfixType per §3.2.
//                                                 ;  (§5.7, 005-90)
```

// See Appendix B for the language-provided type vocabulary that follows
//  this rule. The position-only restriction (value position; never bound
//  position) of §5.7.2 is a semantic rule, not a grammar rule.

### 3.10 Trait intersection `&` in bounds

```
IntersectionType  ::= PostfixType ( '&' PostfixType )+
                    ;  (§5.1, 005-43)
```

// In a *bound* position — a generic parameter's bound (`T: A & B`,
//  §3.12) or a `where`-clause predicate (§3.13) — `IntersectionType`
//  expresses **trait conjunction**: the resolved operands are required
//  to be traits, and the conjunction constrains the bound parameter to
//  satisfy all of them simultaneously (§5.1, 005-43).
// `&` is left-associative and admits arbitrary arity; commutativity and
//  associativity are semantic equivalences over the resulting
//  constraint set (§5.1).
// `&` is the same surface token as bitwise-AND at the value level
//  (§5.7 of this document) and as the leader of certain placement
//  positions; the parser resolves by position (it sits inside a
//  `TypeExpr` here).
// The trait-vs-record kind of the operands and the cross-kind
//  rejection rules of §5.5 are post-parse semantic checks; the grammar
//  admits any `PostfixType & PostfixType`.

### 3.11 Record intersection `&` in type def

`RecordIntersectionType` is the same surface production as
`IntersectionType` of §3.10 — both lift to
`PostfixType ( '&' PostfixType )+`. The grammar does *not* repeat the
production here; the RHS of a `type Name = …` record-intersection
declaration (§7.8) and the RHS of an `alias type` (§7.5) both delegate
to `IntersectionType` for the surface and rely on a post-parse semantic
check to distinguish the two positions:

  - In a *bound* position (§3.10) the operands must resolve to traits;
    the result is a constraint conjunction (§5.1).
  - On the RHS of a `type Name = …` declaration the operands must
    resolve to records; the result is a new nominal record type
    combining the fields of both operands (§5.3, 005-50).

`alias type X = A & B` is grammatically the same intersection in the
alias RHS; the legality of record-intersection under `alias type`
(rejected, §5.4) is a post-parse semantic check.

### 3.12 Generic parameter list (with const-generics + defaults)

```
GenericParamList  ::= '[' GenericParam ( ',' GenericParam )* ','? ']'
                    ;  (§2.5, 004-44)

GenericParam      ::= TypeParam
                    | ConstGenericParam
                    ;  (§2.5, 004-44)

TypeParam         ::= IDENT ( ':' TraitBound )? ( '=' TypeExpr )?
                    ;  (§3.1.6, 005-30)

ConstGenericParam ::= 'const' IDENT ':' TypeExpr ( '=' ConstGenericArg )?
                    ;  (§2.5, 004-44)

TraitBound        ::= PostfixType ( '&' PostfixType )*       // one or more trait operands
                    ;  (§5.1, 005-43)
```

// `TraitBound` admits a single trait `T: Numeric` as well as
//  intersections `T: Numeric & Display`. The `*` repetition (not `+`)
//  ensures the single-operand case is parsed without requiring a `&`.

// **Interleaved type and const-generic parameters (per §2.5).** Type
//  parameters and `const`-prefixed const-generic parameters appear in
//  one comma-separated list and may be freely interleaved in declaration
//  order. The `const` keyword on a parameter is what distinguishes a
//  const-generic from a type parameter (per §2.5 lead paragraph). The
//  list separator is always a comma per §1.4 (002-16); a semicolon is a
//  lex error.
// **Defaults referencing earlier parameters (per §2.3.6, 004-58 / §2.5.7,
//  004-46).** A `TypeParam` default (`= TypeExpr`) and a
//  `ConstGenericParam` default (`= ConstGenericArg`) may reference any
//  *earlier* parameter of the same list (forward references are a
//  post-parse semantic error per §2.3.6 final paragraph). The grammar
//  admits any well-formed `TypeExpr` / `ConstGenericArg` in the default
//  position; the DAG / no-forward-reference rule is checked after
//  parsing.
// **Trait-bound conjunction.** The `: TraitBound` form on a `TypeParam`
//  uses `IntersectionType` of §3.10 — `T: A & B` admits an arbitrary-arity
//  conjunction.
// **Const-generic parameter type.** The post-parse semantic restriction
//  to integer or `bool` types (§2.5.1) is *not* enforced by the grammar;
//  the parser accepts any `TypeExpr` after the `':'`.

### 3.13 `where` clauses

```
WhereClause       ::= 'where' WherePredicate ( ',' WherePredicate )* ','?
                    ;  (§3.3.4, 005-122)

WherePredicate    ::= TraitBoundPredicate
                    | ValueBoundPredicate
                    | AssocTypeEqualityPredicate
                    ;  (§3.3.4, 005-122)

TraitBoundPredicate    ::= TypeExpr ':' TraitBound
                    ;  (§3.3.4, 005-122)

ValueBoundPredicate    ::= Expr                                 // boolean predicate
                    ;  (§2.5.6, 004-49)

AssocTypeEqualityPredicate ::= AssocPath 'is' TypeExpr
                    ;  (§3.1.2, 005-19)

AssocPath         ::= TypePath ( '.' IDENT )+
                    ;  (§3.1.2, 005-19)
```

// **`AssocPath` requires `+1` `.IDENT` tail (per §3.1.2).** The LHS
//  of an `AssocTypeEqualityPredicate` is restricted to an `AssocPath`
//  — a `TypePath` followed by at least one `'.' IDENT` step — so the
//  predicate-position parse does not collide with the `TypePostfix`
//  `TypeAssocAccess` fall-through (§3). A bare `TypeExpr is TypeExpr`
//  has no `.IDENT` step on the LHS and so cannot be misread as an
//  associated-type equality.
// **`is` (not `=`) for assoc-type equality (per Phase D, 030-N).** The
//  predicate operator is the keyword `is`, parallel to the value-level
//  equality `is` (§5.7). This frees `=` for generic-parameter defaults
//  and named-generic-argument introduction inside the same generic
//  bracket without ambiguity. See also §7.12 `FulfillItem` which uses
//  `is` for associated-type bindings.
// **Mixed predicate kinds (per §3.3.4, 005-122).** A single `where`
//  clause may hold any mix of trait bounds (`T: Numeric`), value bounds
//  (`N >= 1`, `N % 2 is 0`), and associated-type equality predicates
//  (`I.Item is i32`). The parser admits each alternative independently
//  and the comma separator is uniform.
// **Predicate-form discrimination (parser priority).** The three
//  alternatives are distinguished by their syntactic shape; the parser
//  tries them in this order to avoid ambiguity:
//   1. `AssocTypeEqualityPredicate` — `AssocPath 'is' TypeExpr`. The
//      parser commits when it sees `<head> . IDENT ... is` in a where-
//      predicate position. The mandatory `.IDENT` tail distinguishes
//      from a `ValueBoundPredicate` that uses `is` at the value level.
//   2. `TraitBoundPredicate` — `TypeExpr ':' TraitBound`. The parser
//      commits on `<head> :` (the `:` immediately after the head
//      distinguishes from a value-bound expression).
//   3. `ValueBoundPredicate` — any other boolean `Expr` over in-scope
//      const-generic parameters and compile-time-known values; the
//      allowable operator set is §2.5.6 and is enforced post-parse.
//  The strict priority order forecloses the otherwise-ambiguous case of
//  an `Expr` that begins with a `TypeExpr` lookalike.
// **Predicate evaluation timing** (`TraitBoundPredicate` checked at
//  resolution; `ValueBoundPredicate` checked at instantiation;
//  `AssocTypeEqualityPredicate` checked at resolution) is post-parse
//  semantic.
// A `where` clause attaches to a generic-bearing declaration (`fn`,
//  `operator`, `type`, `trait`, `fulfill`, `node`, `connection`) and
//  appears between the signature head and the body's `':'`; the precise
//  attachment grammar is given with each declaration kind in §7 / §8 / §9.

### 3.14 Turbofish `::[args]` (positional + named forms)

```
Turbofish         ::= '::' '[' GenericArgList ']'
                    ;  (§2.2.5, 004-26)
```

// **All-positional vs all-named, never mixed (per §2.2.5, 004-31).** A
//  turbofish `GenericArgList` is exactly the same nonterminal as a
//  type-position generic instantiation (§3.2): either `PositionalArgList`
//  (with optional `_` wildcards per 004-28 and trailing omission per
//  004-27) or `NamedArgList`. The two never mix in one list. The parser
//  discriminates on the first argument: an arg whose shape is
//  `IDENT '=' …` selects `NamedArgList`; any other shape selects
//  `PositionalArgList`.
// **Named uses `=`, not `:` (per §2.2.5).** Named generic arguments are
//  `IDENT '=' (TypeExpr | ConstGenericArg)` — distinct from named *value*
//  arguments which use `IDENT ':' Expr` (§5.5 of this document). The
//  reason is that inside a bracketed generic list `:` already separates a
//  parameter from its type (`const N: usize`), so reusing it for the
//  named-argument separator would clash.
// **The `::` is what disambiguates from indexing (per §2.2.5).** Without
//  the `::` prefix, `foo[T](args)` is ambiguous between "index `foo` with
//  `T`, then call" and "call generic `foo` with type argument `T`". The
//  `::` forces path-navigation mode, where the immediately following
//  `[…]` is unambiguously a generic-argument list.
// A `Turbofish` attaches in two places:
//   - On a *path segment* — `Trait::method::[T]`, `From::[i32]::convert`
//     — where the segment immediately preceding the turbofish names a
//     generic item being instantiated. The shape `Path '::' '[' … ']'`
//     is admissible at any segment, not only the final one (per §3.4.1.1,
//     005-128).
//   - On a *method call* — `x.f::[T]()`. The `::` immediately after the
//     method name (and before the `(` of the value-argument list) marks
//     a generic-argument list for the method, not a trait
//     disambiguator (per §3.4.1.1, 005-130).

### 3.15 Kind annotations (`cell` / `signal` / `derived` / `recurrent` / `stream` / `yielded` / `dynamic view`)

A **kind annotation** designates a reactive binding form. It works on
two levels. The kind *keyword* alone (`cell`, `stream`, `derived`, …)
is a **kind** and a keyword — not a type. An *applied* annotation
(`stream T`, `stream[P] T`, `cell T`) IS a type: a member of the type
system in a distinct class, the **wiring types** — unstorable by
nature, expressing wiring rather than a value, and never itself a value
type (§13.2.8.1). Kind brackets *admit* parameters: const-generic
arguments (`recurrent[N]`, capacities — `ConstGenericArg` per §3.2)
and, on `stream`, a policy-type argument bounded by `StreamPolicy`
(`stream[P]`, `stream[Ring[64]]`). That is the legal direction; the
banned direction — a wiring type nested inside a value-type constructor
— never occurs. A kind is legal only in the *outermost* annotation slot
of a declaration, a parameter, or a return — never nested inside a type
constructor (§13.2.8, 016-180). The value type `T` that follows a kind
keyword is an ordinary `TypeExpr`.

```
KindAnnotation    ::= 'cell' TypeExpr
                    | 'signal' TypeExpr
                    | 'derived' TypeExpr
                    | 'recurrent' '[' ConstGenericArg ']' TypeExpr
                    | 'stream' '[' TypeExpr ']' TypeExpr
                    | 'stream' ( 'ring' | 'gate' ) '[' ConstGenericArg ']' TypeExpr
                    | 'stream' TypeExpr
                    | 'recurrent' '[' ConstGenericArg ']' 'stream'
                          ( ( 'ring' | 'gate' ) '[' ConstGenericArg ']'
                            | '[' TypeExpr ']' )? TypeExpr
                    | 'yielded' TypeExpr
                    | 'dynamic' 'view' TypeExpr
                    ;  (§13.2.8, 016-180)
```

// **Outermost-only; never inside a type constructor (per §13.2.8,
//  016-180).** A `KindAnnotation` is admissible ONLY as the whole
//  annotation of a declaration binding, a parameter, or a return type
//  — the positions wired below. It is never a `GenericArg`, a
//  container element, a tuple component, or any other nested
//  `TypeExpr` position; the grammar deliberately does NOT add
//  `KindAnnotation` as a `TypeExpr` alternative, so each admitting
//  site names it explicitly. The sole exception is `Portal[cell T]`
//  (Appendix B.2 / §3.9): there the bracket argument is a cell
//  *designation* (which cell's identity the portal carries), not a
//  nested value type — and it is handled by the
//  `Portal` type, not by this production. Consequently `Vec[cell T]`,
//  `Map[K, signal V]`, and the like are ill-formed.
// **Wired into (per §13.2.8, 016-180).** The parameter/return
//  annotation slots that admit a kind carry `( KindAnnotation |
//  TypeExpr )`: `FnParam` / `FnReturn` (§7.6), `TraitFnParam` (§7.11),
//  `OperatorParam` and the `OperatorDecl` return (§7.13), `EffectParam`
//  (§7.14). Reactive *declaration* forms (`signal` / `derived` /
//  `recurrent` / `stream` at §7.15, `yielded` at §8.9.1) spell their
//  own kind head inline and do not route through this production.
// **Value cells and the `cell` umbrella (per §13.2.8, 016-178).**
//  `signal T`, `derived T`, and `recurrent[N] T` are the value-cell
//  kinds and `cell T` is the umbrella spanning exactly those (`attr`
//  annotates as `signal T`). The bracketed `[N]` on `recurrent` is a
//  history depth; the declaration form (§7.15) may omit it (defaulting
//  to `[1]`), whereas this annotation form names it explicitly.
// **Stream kinds (per §13.18.3).** The policy-generic spelling is
//  `stream[P] T` with `P: StreamPolicy`; the bracket holds a policy
//  *type* (a generic `P` or a concrete `Ring[64]` / `Gate[64]`) — not a
//  `ConstGenericArg`. Concrete instantiation substitutes the policy
//  (`stream[Ring[64]] f32`), legal in generic and concrete positions
//  alike. The word forms `stream ring[N] T` / `stream gate[N] T` are the
//  idiomatic sugar for `stream[Ring[N]] T` / `stream[Gate[N]] T`;
//  `stream T` is the policy-erased form. A recurrent stream
//  (`recurrent[N] stream …`) adds the orthogonal output-history-depth
//  axis (`recurrent[N]`, not a policy) over any of these stream
//  spellings — spelled with either the word form or the `[P]` policy
//  bracket. Stream kinds sit outside the `cell` umbrella.
// **Group kind (per §13.20.4, 034-10).** `yielded T` is the ordered,
//  membership-varying group kind, outside the `cell` umbrella; its
//  declaration form is the §8.9.1 `YieldedDecl` (`yielded <name>:
//  <MemberType> = collect:`).
// **Dynamic view (per §13.3.3.4, 017-192).** `dynamic view T` is the
//  runtime-varying view kind; it is the lowercase-kind spelling that
//  supersedes any wrapped reactive-view-cell type form (§13.2.8.1).
// **No inline bounds on a kind (per §13.2.8).** A bound on the value
//  type is written in the generic parameter list or a `where` clause
//  (`operator f[T: Numeric](x: cell T)`), never inside the kind
//  annotation.

## 4. Patterns

Productions for the surface syntax of patterns used in `let`, `for`, `match`, `repeat`-bind, and variant/tuple/record destructuring contexts.

`Pattern` is the entry-point nonterminal for any pattern-position context (binding LHS, match arm head, for-loop iteration variable, repeat-bind, variant payload sub-pattern, tuple component sub-pattern, record field sub-pattern). Patterns are built compositionally: a `Pattern` is one of the primary forms below, optionally a *binding* (identifier), or a destructuring shape (variant / tuple / record / newtype) whose sub-positions are themselves `Pattern`s — yielding arbitrary nesting (§4.6).

```
Pattern           ::= PatternPrimary
                    | VariantPattern
                    | TuplePattern
                    | RecordPattern
                    | NewtypePattern
                    ;  (§3.5.7, 006-31)
```

// The five alternatives are not parser-disambiguated by a single leading
//  token; the parser looks ahead past the pattern's head identifier (if any)
//  to the *first non-identifier token* in pattern position:
//   - `_` → `PatternPrimary` (wildcard).
//   - bare `IDENT` not followed by `(` / `{` → `PatternPrimary` (binding).
//   - `IDENT '(' …` (the `IDENT` resolves post-parse to a variant /
//     newtype name) → `VariantPattern` or `NewtypePattern`.
//   - `IDENT '{' …` is *not* used: records destructure via `IDENT '(' … ')'`
//     with named sub-patterns (§4.4). Braces are *not* a record-destructure
//     surface (002-25 keeps `{…}` for map literals / interpolation only).
//   - `'(' …` (no leading IDENT) → `TuplePattern`.
//  The variant-vs-newtype kind distinction is post-parse (resolution against
//  the symbol table); both share the surface `IDENT '(' SubPatterns ')'`.
//  See §4.7 for the refutable / irrefutable context constraint.

### 4.1 Pattern primary (wildcard, catch-all, literal, binding)

```
PatternPrimary    ::= Wildcard
                    | Binding
                    | LiteralPattern
                    ;  (§6.2.5, 009-88)

// The unit pattern `()` parses via `TuplePattern`'s `'(' ')'` alternative
//  (§4.3); a single production avoids duplication.

Wildcard          ::= '_'
                    ;  (§3.5.7, 006-31)

Binding           ::= IDENT
                    ;  (§6.2.5, 009-88)

LiteralPattern    ::= IntLit | SignedIntLit | SuffixedIntLit
                    | FloatLit | SuffixedFloatLit
                    | BoolLit | CharLit | ByteLit | StringLit
                    ;  (§6.2.4, 009-86)
```

// **Wildcard `_`** matches any value, binds nothing. It is the universal
//  filler in compound patterns (`field: _`, `(x, _, z)`) and the canonical
//  match catch-all (`_:` arm; 009-88).
// **Binding** is a bare identifier in pattern position. It is irrefutable —
//  matches any value of the scrutinee's type and binds the value to the
//  identifier. A bare identifier as a match arm head also acts as a
//  catch-all (009-88): `name:` covers every remaining scrutinee value and
//  binds it to `name`.
// **LiteralPattern** matches scrutinees equal to the literal's value, by the
//  scrutinee type's `is`-equivalence (§6.2.4). It is refutable; admissible
//  only in match-arm positions (§4.7).
// The wildcard `_` and the *type-position* wildcard of §3.1 share a
//  spelling and are disambiguated by syntactic position (per 004-32).
// The unit pattern `()` matches the unit value (§9.2.3); it is the
//  zero-component tuple-pattern form, refutable only against a non-unit
//  type (which is a type error rather than a runtime miss).
// `LiteralPattern` does not include `Path` literals (e.g. `Constants::FOO`)
//  at the surface — post-parse, an identifier resolving to a constant value
//  may be allowed in pattern position per §6.2.4, but that is a name-resolution
//  rule, not a grammar production.

### 4.2 Variant patterns (positional / named / trailing `...`)

```
VariantPattern    ::= VariantHead ( '(' VariantPayload? ')' )?  // parens optional for unit variants
                    ;  (§6.2.4, 006-32)

VariantHead       ::= TypePath                                  // resolves to enum variant
                    ;  (§6.2.4, 006-31)

VariantPayload    ::= PositionalPayload
                    | NamedPayload
                    ;  (§3.5.7, 006-32)

PositionalPayload ::= Pattern ( ',' Pattern )* ( ',' RestToken )? ','?
                    ;  (§3.5.7, 006-32)

NamedPayload      ::= NamedFieldPattern ( ',' NamedFieldPattern )* ( ',' RestToken )? ','?
                    ;  (§3.5.7, 006-32)

NamedFieldPattern ::= IDENT ( ':' Pattern )?                   // shorthand: `field` ≡ `field: field` (per 006-33)
                    ;  (§6.2.4, 006-33)

RestToken         ::= '...'
                    ;  (§3.5.7, 006-32)
```

// **Positional vs named, never mixed (per §6.2.4, 006-32).** The two
//  payload forms parallel variant construction (§6.2.1.1). The parser
//  discriminates on the first payload sub-element: an element whose shape
//  is `IDENT ':' Pattern` selects `NamedPayload`; any other shape
//  (including a bare `IDENT` binding) selects `PositionalPayload`. Mixing
//  the two forms within one variant payload is a parse error
//  (e.g. `Rectangle(width, height: h)`).
// **Trailing `...` rest (per 006-32).** The `...` rest token is three
//  dots, distinct lexically from the `..` range operator (§5.8). It is
//  permitted *only* as the final element of the payload — after the last
//  comma-separated sub-pattern. It binds nothing and elides every payload
//  field not explicitly listed. Without `...`, a variant pattern is
//  exhaustive over its payload components (006-31): every payload field
//  must be named (in `NamedPayload`) or every payload position must have a
//  sub-pattern (in `PositionalPayload`).
// **Named form admissibility** (per §6.2.4). `NamedPayload` is well-formed
//  only when the variant was declared with named payload fields; a
//  positionally-declared variant (e.g. `Some(T)`) admits only
//  `PositionalPayload`. This is a post-parse semantic check; the parser
//  admits the surface in either case.
// **Catch-all arms** at the match level (§5.19) use `PatternPrimary` —
//  `Wildcard` or `Binding` — not a `VariantPattern`. A bare lowercase
//  `IDENT` (e.g. `name`) in pattern position is a *binding*; a bare
//  capitalised path that resolves post-parse to a unit-variant
//  constructor (e.g. `None`, `North` in `enum Direction: North | …`)
//  is a `VariantPattern` whose `( VariantPayload )?` is omitted, per
//  the SPEC §8.2.1 example `None: panic(...)`. The capitalisation /
//  resolution-against-variant-table check is post-parse; the grammar
//  admits both interpretations via the optional paren tail.

### 4.3 Tuple patterns (positional, trailing `...`)

```
TuplePattern      ::= '(' ')'                                   // unit pattern (see §4.1)
                    | '(' Pattern ',' ')'                       // 1-tuple
                    | '(' Pattern ( ',' Pattern )+ ( ',' RestToken )? ','? ')'
                    ;  (§9.2.2, 006-32)
```

// **Always positional (per §9.2.2, §3.5.7).** Tuples have no field names,
//  so there is no named tuple-pattern form. Sub-positions are bound by
//  ordering against the tuple's components.
// **1-tuple trailing-comma rule (per §9.2.4).** The single-component
//  alternative `'(' Pattern ',' ')'` *requires* the trailing comma —
//  parallel to the 1-tuple type production of §3.5 and the 1-tuple
//  value-literal production. Without the comma the surface
//  `'(' Pattern ')'` matches *no* `Pattern` production (neither
//  `PatternPrimary` of §4.1, which admits only `'(' ')'`, nor
//  `TuplePattern` of this section, whose 1-tuple alternative requires
//  the comma) and is therefore a parse error. Patterns differ from
//  types in this regard: §3.5 types admit a grouping form `'(' TypeExpr ')'`
//  via `TypePrimary` (§3.1), but patterns have no analogous grouping
//  production — grouping a single pattern in parens is not part of the
//  surface.
// **Trailing `...` rest (per 006-32).** Permitted only as the final
//  element. `(first, ...)` binds `first` and elides the remaining
//  components. Without `...`, a tuple pattern is exhaustive over the
//  tuple's components (§9.2.2).
// **Nesting.** Tuple patterns nest to arbitrary depth via the recursive
//  `Pattern` sub-positions (§4.6): `((a, b), c)` destructures a
//  `((T1, T2), T3)`. Each inner `(...)` here is a tuple pattern of two
//  or more components (admitted by the n-tuple alternative), not a
//  grouping form — bare `( Pattern )` with a single sub-pattern and no
//  trailing comma has no admitting production (see the 1-tuple
//  trailing-comma rule above) and is a parse error.

### 4.4 Record patterns (named, trailing `...`)

```
RecordPattern     ::= RecordHead '(' RecordPatternFieldList? ')'
                    ;  (§6.2.4, 006-31)

RecordHead        ::= TypePath                                  // resolves to a record type
                    ;  (§6.2.4, 006-31)

RecordPatternFieldList ::= RecordPatternField ( ',' RecordPatternField )* ( ',' RestToken )? ','?
                    ;  (§3.5.7, 006-33)

RecordPatternField ::= IDENT ( ':' Pattern )?                   // shorthand: `field` ≡ `field: field`
                    ;  (§3.5.7, 006-33)
```

// **Always named (per §3.5.7).** Records have field names; record patterns
//  are exhaustive over those fields by default. Each entry is a
//  `RecordPatternField` admitting either the long form `IDENT ':' Pattern`
//  or the shorthand bare `IDENT` (the shorthand `field` is equivalent
//  to `field: field`, binding the field's value to a local of the same
//  name). There is no positional record-pattern form.
// **Shorthand vs long form (per Phase D, 006-33).** Inside a
//  `RecordPattern` (and likewise inside a named-variant payload via
//  `NamedFieldPattern` of §4.2 which accepts the same shorthand by
//  cross-reference), `BigRec(a, b: renamed_b, c, ...)` is equivalent
//  to `BigRec(a: a, b: renamed_b, c: c, ...)`. The shorthand binds
//  each elided field's value to a local with the same identifier;
//  the long form rebinds, supplies a sub-pattern, or wildcards
//  (`field: _` to ignore). The shorthand cannot be combined with a
//  sub-pattern other than the implicit bare-identifier binding.
// **Exhaustiveness applies to both forms.** A `RecordPattern` must
//  enumerate every field of the resolved record (using shorthand or
//  long form) unless terminated by `...` rest (per 006-32).
// **Exhaustive by default (per 006-31).** A record pattern must bind
//  *every* field. A field whose value is not needed is bound to the
//  wildcard (`field: _`). To opt out of exhaustiveness, append the
//  trailing `...` rest token, which elides every unlisted field
//  (per 006-32): `Point(x: px, ...)` binds `x` and discards `y` (and any
//  further fields).
// **Surface shares `IDENT '(' … ')'` with `VariantPattern` (§4.2).** The
//  parser admits the same surface for both kinds; the discrimination
//  (record vs enum-variant) is by name-resolution of `RecordHead` /
//  `VariantHead` against the symbol table. This is a post-parse semantic
//  decision; the parser builds a uniform `Pattern` node and the resolver
//  attaches the kind.
// Records use *parens* `( … )` in pattern position uniformly with
//  construction (§3.5.3) — braces `{ … }` are reserved for map literals
//  and string-interpolation expression delimiters, never record
//  destructuring.

### 4.5 Newtype patterns

```
NewtypePattern    ::= NewtypeHead '(' Pattern ')'
                    ;  (§6.3.2, 006-31)

NewtypeHead       ::= TypePath                                  // resolves to a newtype
                    ;  (§6.3.2, 006-31)
```

// **Always positional, exactly one sub-pattern (per §6.3.2).** A newtype
//  wraps exactly one value; the pattern surface is `Newtype(binding)`
//  with a single `Pattern` sub-position. No named form, no multi-element
//  form, no trailing `...` (there is nothing to elide). A
//  zero-sub-pattern or multi-sub-pattern form is a parse error.
// **Surface shares `IDENT '(' … ')'` with `VariantPattern` (§4.2) and
//  `RecordPattern` (§4.4).** The parser admits the same surface for all
//  three; the kind distinction (variant / record / newtype) is by
//  name-resolution of `NewtypeHead`. The arity check (exactly one
//  sub-pattern) is enforced post-parse against the resolved kind.
// **Reads opposite of `T(value)` extraction (per §6.3.2).** In *pattern*
//  position the head names the *newtype* (`Email(addr)`, `UserId(n)`); in
//  *expression* position `T(value)` for the wrapped type `T` extracts the
//  underlying value. The two coexist and read oppositely; they never
//  collide at the grammar level because they sit in disjoint positions.

### 4.6 Nested patterns

Patterns nest to arbitrary depth. Every sub-position of a `VariantPattern`,
`TuplePattern`, `RecordPattern`, and `NewtypePattern` is itself a full
`Pattern` and may take any of the §4.1–§4.5 forms.

```
NestedPattern     ::= Pattern                                   // applies in every sub-position
                    ;  (§9.2.2, 009-86)
```

// No depth limit is imposed by the grammar; the parser admits arbitrary
//  composition. Example shapes:
//   - `Some((x, y))`             — variant payload is a tuple pattern.
//   - `Ok(Email(addr))`          — variant payload is a newtype pattern.
//   - `((a, b), c)`              — tuple component is a tuple pattern.
//   - `Point(x: 0, y: py, ...)`  — record field bound to a literal pattern.
//   - `Rect(width: w, height: _)`— record field bound to a wildcard.
//  Each sub-position independently chooses its `Pattern` alternative; the
//  refutability of the whole is the disjunction of the refutability of its
//  sub-patterns (see §4.7).

### 4.7 Refutable vs irrefutable contexts

The grammar admits the same `Pattern` nonterminal in every pattern
position. Whether a given pattern is *refutable* (can fail to match) or
*irrefutable* (always matches) is a property of its shape, not its
position. Pattern positions split into two context categories per the
constraint they impose on the pattern's refutability:

**Irrefutable-only contexts (refutable pattern is a compile error).**

- `let <Pattern> = <expr>` binding LHS (§6.1).
- `mut <Pattern> = <expr>` binding LHS (§6.1).
- `const <Pattern> = <expr>` binding LHS (§6.1).
- `for <Pattern> in <iterable>:` iteration variable (§6.2; per §12.12.1
  the iteration variable must always bind successfully — 014-152).
- `repeat <Pattern> in <source>:` bind position (§13.5.4.1; per 018-43
  the bind accepts the same destructuring grammar as the for-loop's
  iteration variable and is therefore likewise irrefutable).
- Function / closure / operator parameter patterns (§7.6, §7.7, §7.13).

**Refutable-permitting contexts (any pattern admissible).**

- `match <expr>:` arm head (§5.19).
- `given <expr>:` arm head (§5.22; shares match arm shape per 009-91).

**Refutability table (a pattern's refutability follows from its shape).**

| Pattern form              | Refutable? |
|---------------------------|------------|
| `Wildcard` (`_`)          | irrefutable |
| `Binding` (bare `IDENT`)  | irrefutable |
| `LiteralPattern`          | refutable |
| `TuplePattern`            | irrefutable iff every sub-pattern is irrefutable |
| `RecordPattern`           | irrefutable iff every named sub-pattern is irrefutable |
| `NewtypePattern`          | irrefutable iff its single sub-pattern is irrefutable |
| `VariantPattern`          | refutable (the variant tag may not match), except when the scrutinee's enum has exactly one variant and every sub-pattern is irrefutable |

;  (§6.2.4, 006-31)

// This subsection is a **normative note**, not a grammar production —
//  refutability is a property of the resolved pattern, and the
//  irrefutable-only-context constraint is a post-parse semantic check
//  (parallel to the typing checks of §3.x). The grammar admits any
//  `Pattern` in every position; the parser does not branch on
//  refutability.
// The compile-error diagnostic for a refutable pattern in an
//  irrefutable-only context is mandated by §12.12.1 for `for`, by §6.1 for
//  bindings, and follows from the iteration-variable rule of §13.5.4.1 for
//  `repeat`-bind. The catch-all arm of §5.19 (009-88) is the canonical
//  irrefutable-in-refutable-position form.

## 5. Expressions

Productions for the surface syntax of expressions: primaries, postfix and prefix forms, calls, casts, binary operators (with the full precedence table), ranges, slicing, optional chaining, with, control-flow expressions, reactive expressions, place-assignment LHS, panic.

`Expr` is the entry-point nonterminal for an expression in any value-position context. The grammar is layered: binary operators (§5.7) sit above `UnaryExpr` (the prefix-operator layer, §5.4), which sits above `PostfixExpr` (the postfix-operator layer, §5.2), which sits above `PrimaryExpr` (the atom layer, §5.1). Special forms — `with` (§5.13), `if` (§5.18), `match` (§5.19), `observe` (§5.20), `when` block (§5.21), `given` block (§5.22), pipe `|>` (§5.25) — are productions at the binary or top layer, sequenced into `Expr` per their precedence in Appendix A.

```
Expr              ::= AssignExpr
                    | WithExpr
                    ;  (§4.4.7, 007-74)

AssignExpr        ::= PlaceLhs '=' Expr                          // see §5.26
                    ;  (§11.11, 013-204)

WithExpr          ::= WhereFilterExpr ( WithSuffix )*            // see §5.13
                    ;  (§6.1.5, 009-23)

WhereFilterExpr   ::= PipeExpr ( 'where' PipeExpr )*             // see §5.23
                    ;  (§13.18.10, 030-169)

PipeExpr          ::= OrExpr ( '|>' PipeRhs )*                   // see §5.25; tier 1
                    ;  (§13.17.7, 029-65)

OrExpr            ::= AndExpr ( 'or' AndExpr )*                  // tier 2
                    ;  (§4.4.7, 007-74)

AndExpr           ::= BitOrExpr ( 'and' BitOrExpr )*             // tier 3
                    ;  (§4.4.7, 007-74)

BitOrExpr         ::= BitXorExpr ( '|' BitXorExpr )*             // tier 5
                    ;  (§4.4.2, 007-73)

BitXorExpr        ::= BitAndExpr ( '^' BitAndExpr )*             // tier 6
                    ;  (§4.4.2, 007-73)

BitAndExpr        ::= RangeExprTier ( '&' RangeExprTier )*       // tier 7
                    ;  (§4.4.2, 007-73)

RangeExprTier     ::= CompareExpr ( '..' CompareExpr )?          // tier 8, non-associative (single application)
                    ;  (§4.4.7, 007-74)

CompareExpr       ::= ShiftExpr ( CompareOp ShiftExpr )?         // tier 9, non-associative (single application)
                    ;  (§4.4.7, 007-74)

CompareOp         ::= EqualityOp | OrderingOp
                    ;  (§4.4.7, 007-74)

ShiftExpr         ::= AdditiveExpr ( ShiftOp AdditiveExpr )*     // tier 10
                    ;  (§4.4.7, 007-74)

AdditiveExpr      ::= MultiplicativeExpr ( AdditiveOp MultiplicativeExpr )*  // tier 11
                    ;  (§4.4.7, 007-74)

MultiplicativeExpr ::= UnaryExpr ( MultiplicativeOp UnaryExpr )* // tier 12
                    ;  (§4.4.7, 007-74)
```

// `AssignExpr` is the place-assignment form `LHS '=' RHS`; admitted only
//  in statement position (§6.1) — a bare `Expr` that is an `AssignExpr` is a
//  parse error in value position. The grammar carries the alternative at
//  this level because place-assignment shares the LHS path syntax with
//  postfix access. The parser admits `=` only when the LHS is a valid
//  place-assignment LHS per §5.26 and the surrounding context is a
//  statement, not a value-yielding sub-expression. The statement-form
//  spelling `AssignStmt` (§5.26) is `AssignExpr` admitted at a `BlockItem`
//  position — the productions denote the same surface shape.
// `WithExpr` wraps `WhereFilterExpr` and admits the low-precedence
//  `with` update of §5.13 as a postfix at the top of the operator tower
//  (looser than every binary operator including `|>`). Multiple `with`
//  applications chain left-associatively per §6.1.5.
// `WhereFilterExpr` is the binary stream-filter `A where C` of §5.23;
//  per DECISION_LOG 030-168 it is left-associative, and per 030-169 it
//  binds tighter than `|>`. So `a where b |> c` parses as
//  `(a where b) |> c`. The `where` operator sits *between* `|>` (tier 1,
//  loosest) and the logical operators (tiers 2–3). See the §5.23 block
//  for the disambiguator against declaration-level `where` clauses.
// `PipeExpr` enforces tier 1 left-associativity per 029-72. `OrExpr` /
//  `AndExpr` realize tiers 2 and 3 of Appendix A. The per-tier nonterminal
//  decomposition (rather than a single flat `BinaryExpr`) makes the
//  precedence directly mechanical for a recursive-descent implementer.
// `RangeExprTier` and `CompareExpr` use single-application
//  (`x ( op y )?`) rather than `*` repetition because tiers 8 and 9 are
//  non-associative per §4.4.7. `a..b..c` and `a < b < c` both fail at
//  parse time at the second operator.

### 5.1 Primary expressions (literal, ident, path, parenthesized)

```
PrimaryExpr       ::= Literal
                    | Path
                    | ParenExpr
                    | TupleExpr                                // §5.16
                    | ArrayExpr                                // §5.14
                    | MapExpr                                  // §5.15
                    | BlockExpr                                // §5.17
                    | StringInterpExpr                         // §5.12
                    | IfExpr                                   // §5.18
                    | MatchExpr                                // §5.19
                    | ObserveExpr                              // §5.20
                    | WhenBlockExpr                            // §5.21
                    | GivenBlockExpr                           // §5.22
                    | DeleteExpr                               // §5.11
                    ;  (§4.4, 007-74)

Literal           ::= IntLit | SignedIntLit | SuffixedIntLit
                    | FloatLit | SuffixedFloatLit
                    | BoolLit | CharLit | ByteLit | StringLit
                    | SuffixedUserLit
                    ;  (§4.3, 007-17)

SuffixedUserLit   ::= ( IntLit | FloatLit ) IDENT              // user-defined @literal_suffix
                    ;  (§4.3.3, 007-31)

Path              ::= PathSegment ( '::' PathSegment )* Turbofish?
                    ;  (§3.4.1, 005-128)

PathSegment       ::= IDENT
                    | 'here'
                    | 'module'
                    | 'root'
                    | 'std'
                    | 'subject'
                    ;  (§10.2.3, 002-7)

// `Subject` (capitalised) is the sole reserved capitalised *type*
//  identifier (002-12); it is lexed as an ordinary `IDENT` and is
//  reachable here via the `IDENT` alternative. The lowercase `subject`
//  is a *keyword* (002-8) and gets its own alternative because the
//  lexer must reclassify it before it reaches the parser.
// **`here` / `module` are single-suffix anchors (per Phase D, D7;
//  §13.7.2 / §13.7.3).** When a `Path` begins with `here` or
//  `module`, exactly one `::` segment may follow — the chained form
//  `here::a::b` is rejected post-parse (the grammar's `Path` admits
//  the shape, but the `NamespaceAnchorPath` of §13.3 is the
//  authoritative production for the bounded form). A bare `here` /
//  `module` (no `::` tail) is also rejected.

ParenExpr         ::= '(' Expr ')'
                    ;  (§4.4, 007-74)
```

// `Literal` covers every form produced by the lexical literal productions
//  of §2.6–§2.9. `SignedIntLit` admission here is the parser-level
//  consequence of the §2.11 unification rule.
// `SuffixedUserLit` is the parser-side join for user-defined
//  `@literal_suffix` constructors (§2.10). The lexer emits two adjacent
//  tokens — a numeric literal and an `IDENT` — and the parser merges them
//  into one expression node iff the suffix name resolves to a registered
//  literal-suffix constructor (post-parse semantic check).
// `Path` shares its segment vocabulary with `TypePath` (§3.1) and admits a
//  trailing `Turbofish` (§3.14) for generic-argument supply at the final
//  path segment or at any non-terminal segment (the latter for trait
//  disambiguation per §3.4.1.1, 005-128).
// `ParenExpr` is grouping only; `(x)` and `x` produce the same AST aside
//  from source position. A one-element tuple is the distinct form
//  `(x,)` — see §5.16.
// `subject` (lowercase) is the instance-value keyword (§2.4, 002-8); it
//  appears in `PrimaryExpr` position via its `Path` segment.

### 5.2 Postfix forms (`.field`, `.NUMERIC` for tuples, `[idx]`, `(args)`, `?`)

```
PostfixExpr       ::= PrimaryExpr Postfix*
                    ;  (§4.4.7, 007-74)

Postfix           ::= FieldAccess
                    | MethodPart                               // FieldAccess + Turbofish + CallSuffix
                    | TupleIndex
                    | IndexAccess
                    | CallSuffix
                    | TryPostfix
                    | OptChainField                            // §5.3
                    | OptChainIndex                            // §5.3
                    | OptChainCall                             // §5.3
                    | CastPolicySuffix                         // §5.6
                    ;  (§4.4.7, 007-74)

FieldAccess       ::= '.' IDENT                                // bare field projection; no turbofish
                    ;  (§6.1.4, 009-20)

MethodPart        ::= '.' IDENT Turbofish CallSuffix           // method invocation with explicit generic args
                    ;  (§3.14, 005-130)

TupleIndex        ::= '.' DecIntLit
                    ;  (§9.2.1, 012-47)

IndexAccess       ::= '[' IndexArg ( ',' IndexArg )* ']'
                    ;  (§9.3.7, 012-84)

IndexArg          ::= Expr
                    | RangeIndex                               // §5.10
                    | FromEndIndex                             // §5.9
                    ;  (§9.3.7, 012-84)

CallSuffix        ::= '(' CallArgList? ')'
                    ;  (§3.4, 013-130)

TryPostfix        ::= '?'
                    ;  (§8.4, 011-40)
```

// `FieldAccess` and `TupleIndex` share the `.` token; the parser
//  discriminates on the token following `.` — `IDENT` → `FieldAccess`,
//  `DecIntLit` → `TupleIndex` (per §9.2.3). A tuple index is a *decimal*
//  integer literal only — hex/oct/bin literals are not admitted in tuple
//  position (a `0x` prefix following `.` is a parse error).
// `FieldAccess` is bare projection. The method-call form `x.f::[T](...)`
//  with explicit generic arguments parses through `MethodPart`, which
//  fuses `'.' IDENT Turbofish CallSuffix` so a turbofish without a
//  trailing call is rejected at parse time rather than post-parse
//  (per §3.14, 005-130). An ordinary method call `x.f(args)` (no
//  explicit generics) is `FieldAccess` followed by `CallSuffix` — both
//  are `Postfix` alternatives and chain freely.
// `CallSuffix` covers both free-function and method-call sites; the
//  receiver / first argument is the preceding `PostfixExpr` head when
//  reached via method-call form (per §3.4, 013-130 — uniform call syntax).
// `TryPostfix` `?` desugars per §8.4. The same `?` token is the prefix of
//  optional-chaining `?.` / `?[` / `?(` of §5.3 — discrimination is by the
//  token following `?`: another postfix-trigger token (`.`, `[`, `(`)
//  selects optional-chaining; anything else selects `TryPostfix`. The
//  lexer emits `?.` / `?[` / `?(` as single tokens per §2.12 to keep this
//  discrimination unambiguous.
// `?` is also the lexeme of the cast-policy marker `T?(x)` (§5.6) and a
//  flag character in placement flag runs (§13.8.8); the parser resolves by
//  position. In `PostfixExpr` position, `?` is `TryPostfix` (or the leader
//  of an optional-chaining token); the cast-policy reading applies only
//  when the head is a type name immediately followed by `?(` per §5.6.
// All `Postfix` forms bind at tier 14 of Appendix A — they are
//  left-associative and chain freely: `x.f().g[i].h`.

### 5.3 Optional chaining (`?.`, `?[]`, `?()`)

```
OptChainField     ::= '?.' IDENT                               // bare field projection
                    | '?.' IDENT Turbofish CallSuffix          // method invocation with explicit generics
                    ;  (§4.4.7, 007-74)

OptChainIndex     ::= '?[' IndexArg ( ',' IndexArg )* ']'
                    ;  (§4.4.7, 007-74)

OptChainCall      ::= '?(' CallArgList? ')'
                    ;  (§4.4.7, 007-74)
```

// `?.`, `?[`, `?(` are single tokens emitted by the lexer (§2.12). They
//  attach as `Postfix` alternatives at tier 14 of Appendix A — the same
//  precedence as their non-optional counterparts `.`, `[`, `(`. The
//  optional forms short-circuit on a `None` / `Err` receiver, propagating
//  the empty case through the remaining chain.
// Chaining is free: `x?.f?.g[i]?.h()` is well-formed. Mixing optional and
//  non-optional postfix forms in the same chain is admissible — each step
//  is independent at the grammar level; the semantic interpretation of
//  the optional steps is post-parse (§5.2 / §5.3 of SPEC).
// The post-parse type requirement that the receiver of an `?.` / `?[` /
//  `?(` step be an `Option`- or `Result`-shaped (`Try`-implementing) value
//  is a semantic check, not a grammar restriction.

### 5.4 Prefix forms (`-`, `~`, `not`, `handle`, `handle!`, `portal`, `dyn`, `move`)

```
UnaryExpr         ::= PrefixOp UnaryExpr
                    | PostfixExpr
                    ;  (§4.4.7, 007-74)

PrefixOp          ::= '-'                                      // unary minus (tier 13)
                    | '~'                                      // bitwise not  (tier 13)
                    | 'not'                                    // logical not  (tier 4)
                    | 'handle'                                 // handle borrow (tier 13)
                    | 'handle!'                                // strong handle (tier 13)
                    | 'portal'                                 // portal handle (tier 13)
                    | 'dyn'                                    // dyn value-coercion (tier 13)
                    | 'move'                                   // move (call-arg position only)
                    | ArithPolicyPrefixOp                      // -%, -|, -? (tier 13)
                    ;  (§4.4.7, 007-74)

ArithPolicyPrefixOp ::= '-%' | '-|' | '-?'
                    ;  (§4.6, 007-79)
```

// **Prefix operator precedence.** All prefixes except `not` bind at tier
//  13 (the unary-prefix tier of Appendix A) — they associate right and
//  bind tighter than every binary operator except postfix and path
//  resolution. `not` binds at tier 4 (looser than comparison) and so
//  negates a whole comparison: `not a is b` parses as `not (a is b)` per
//  the precedence table (007-74, 007-77).
// **`-` (unary minus) vs `SignedIntLit`.** The unary-minus prefix and the
//  §2.11 negative-literal rule are not in conflict: in a context where the
//  next token after `-` is an `IntLit` / `SuffixedIntLit` and the unified
//  `-N` token is being produced for value-fits checking, the lexer-level
//  rule produces one `SignedIntLit`; in any other context, `-` is the
//  unary-minus prefix here. The two interpretations always agree on the
//  runtime value (negation of the literal); the §2.11 rule exists only to
//  preserve the literal's value-fits guarantee against the target type
//  (per 004-100).
// **`move` is forbidden outside call-arg positions (per §11.8.5,
//  013-135..142).** `move <ident>` is admissible only as an
//  immediate sub-expression of a `CallArg` (§5.5), or as the immediate
//  contents of a `ParenExpr` whose enclosing context is the head of a
//  postfix chain (a call receiver). The latter carve-out permits
//  `(move v).method()` per SPEC §11.8.3 / §11.8.5 line 9185 — the
//  paren-wrapped `move v` then serves as the receiver of a method
//  call. A `move` appearing in binding-RHS (`let y = move x`), return
//  (`return move x`), or any other non-receiver position is a parse
//  error.
// **`move` operand is a bare `IDENT` only (per Phase D, SPEC §11.8.5).**
//  Per Model 4, partial-move via field-access path is no longer
//  admitted; record decomposition is canonical via pattern
//  destructuring (§4.4 record-pattern shorthand). A method-call form
//  `move x.f()` is a parse error (use `(move x).f()`); a field-access
//  form `move r.field` is also a parse error (use record-pattern
//  destructuring `let Rec(field: f, ...) = r` followed by `f`).
// **`move` operand vs place-l-value asymmetry.** `MoveArg` admits a
//  bare `IDENT` only (Model 4, Phase D), while `PlaceLhs` of §5.26
//  admits `.field` / `.DecIntLit` / `[Expr]` projections. The
//  asymmetry is intentional — move transfers a whole binding; place-
//  assignment writes to a single storage slot reachable through the
//  binding.
// **`dyn` value-position vs type-position (per §3.8).** `dyn` in
//  `UnaryExpr` position is the value-coercion prefix of §5.2.5;
//  `dyn` in type-position is the trait-object marker of §3.8 — the
//  surrounding production category disambiguates.
// **`handle` / `handle!` / `portal` arity.** These prefixes take a single
//  `UnaryExpr` operand (the value being borrowed / strong-borrowed /
//  portal-projected per §13.3.6.2 / §13.6.2). The lexer emits `handle!`
//  as a single token per §2.4 (002-11); the surface `handle !` (with a
//  space) is two tokens and is a parse error in this position.
// **Arithmetic-policy unary forms.** `-%`, `-|`, `-?` are the
//  wrapping / saturating / checked variants of unary minus per §4.6.2 /
//  §4.6.3 / §4.6.4; they bind at the same prefix tier as `-`.

### 5.5 Call expressions

```
CallArgList       ::= PositionalCallArgs
                    | NamedCallArgs
                    ;  (§3.5, 006-1)

PositionalCallArgs ::= CallArg ( ',' CallArg )* ','?
                    ;  (§3.5, 006-4)

NamedCallArgs     ::= NamedCallArg ( ',' NamedCallArg )* ','?
                    ;  (§3.5, 006-2)

CallArg           ::= Expr
                    | MoveArg
                    ;  (§11.8.5, 013-135)

NamedCallArg      ::= IDENT ':' ( Expr | MoveArg )
                    ;  (§3.5, 006-2)

MoveArg           ::= 'move' IDENT
                    ;  (§11.8.5, 013-135)
```

// **Uniform call syntax (per §3.4).** `x.f(args)` and `f(x, args)`
//  produce the same AST shape post-parse: the receiver in the method
//  form is the first positional argument in the canonical free form
//  (013-130). The dot-call surface adds no implicit ownership rule of its
//  own.
// **Positional vs named, never mixed (per §3.5, 006-6).** A single call's
//  argument list is either *all positional* or *all named*. The parser
//  discriminates on the first argument: an argument whose shape is
//  `IDENT ':' ...` selects `NamedCallArgs`; any other shape selects
//  `PositionalCallArgs`. A later argument in the other form is a parse
//  error (`f(x, y: 2)` rejected). Trailing comma is admitted in either
//  form.
// **Named uses `:`, not `=`.** Named *value* arguments use `IDENT ':' Expr`
//  (the `:` separator parallels record-field construction, §6.1.3). This
//  is distinct from named *generic* arguments, which use `IDENT '=' ...`
//  per §3.2 / §3.14 — the type-level list reserves `:` for `const N: T`
//  parameter declarations and so reuses `=` for named arguments
//  (per 004-31).
// **`move` in call-arg position only.** `MoveArg` is admissible *only* as
//  a `CallArg` or the value half of a `NamedCallArg`. Per Phase D
//  (Model 4) the operand is a bare `IDENT` — partial-move via a
//  field-access path is no longer admitted; record decomposition is
//  expressed via record-pattern destructuring (§4.4). A method call is
//  not an l-value (`move x.f()` ✗ — use `(move v).method()` per §11.8.3).

### 5.6 Cast forms `T(x)`, `T%(x)`, `T|(x)`, `T?(x)`

```
CastExpr          ::= TypePath CastPolicyMarker? '(' Expr ')'  // documentation-only overlay; canonical path is Path + CastPolicySuffix? + CallSuffix via PostfixExpr (§5.1, §5.2)
                    ;  (§4.7.1, 007-91)

CastPolicyMarker  ::= '%' | '|' | '?'
                    ;  (§4.7.1, 007-91)

CastPolicySuffix  ::= CastPolicyMarker '(' Expr ')'            // tail per §5.2
                    ;  (§4.7.1, 007-91)
```

// **`CastExpr` is documentation-only.** The named production is not
//  referenced from any other production; the canonical parse path for
//  `T(x)` / `T%(x)` / `T|(x)` / `T?(x)` is a `Path` (§5.1) head with a
//  `CastPolicySuffix?` followed by a `CallSuffix` — both as `Postfix`
//  alternatives of §5.2 attached to the `Path`. The named `CastExpr`
//  production is retained here as a documentation shape so a reader can
//  see the cast form in one place; a parser implementer wires it
//  through `PostfixExpr` alone.

// **`T(x)` shares surface with construction and call (per §4.7.5,
//  007-91).** `T(value)` is admissible at three semantic sites — numeric
//  conversion / newtype extraction / record construction — disambiguated
//  post-parse by the kind of `T`. The parser builds the same AST node
//  (a `CallSuffix` attached to a `Path` PostfixExpr) and the resolver
//  attaches the semantic interpretation. The cast-policy variants are the
//  only form where the parser branches at the grammar level: a `%`,
//  `|`, or `?` token immediately following a type name and immediately
//  preceding `(` is the cast-policy marker per §4.7.1; the parser
//  consumes the marker into the cast head.
// **Policy marker placement (per §4.7.1).** The policy marker `%` / `|` /
//  `?` attaches to the *target type*, immediately before the argument
//  list — `u8%(x)`, *not* `u8(%x)` or `u8(x)%`. The disambiguation works
//  because a type name is never a value operand: the lexical sequence
//  `<type-name> '%' '(' ...` cannot be parsed as modulo, the sequence
//  `<type-name> '|' '(' ...` cannot be bitwise-OR (which requires two
//  expression operands), and `<type-name> '?' '(' ...` cannot be the Try
//  postfix (which would need an expression head and no `(` follow).
// **Cast-policy suffix at the postfix tier (per Appendix A).** The
//  cast-policy forms `T%(x)` / `T|(x)` / `T?(x)` bind at tier 14, the
//  same tier as a call suffix. The reason for the separate `Postfix`
//  alternative `CastPolicySuffix` is that the policy marker may attach
//  directly to any `PostfixExpr` whose head is a type — the form lifts
//  uniformly with the postfix chain.
// **`T?(value)` is *not* the optional-chaining call `?(`.** The two
//  distinct tokens `?` then `(` of `T?(value)` differ from the single
//  token `?(` of optional-chaining call (per §2.12). The lexer emits
//  `?(` as a single token only when no whitespace separates them and the
//  `?` is in postfix-trigger position immediately after a value-typed
//  expression; in `T?(x)` the `?` follows a type-name `PathSegment` and
//  is lexed as the separate cast-policy marker.

### 5.7 Binary expressions (full precedence table — lifted from §4.4.7)

The per-tier nonterminals that realize the binary operator tower (`OrExpr`, `AndExpr`, `BitOrExpr`, `BitXorExpr`, `BitAndExpr`, `RangeExprTier`, `CompareExpr`, `ShiftExpr`, `AdditiveExpr`, `MultiplicativeExpr`) are defined at the top of §5 immediately under the `Expr` production. They encode the precedence and associativity of Appendix A directly — a recursive-descent parser walks the tower without consulting the table. The operator-token productions below define the per-tier operator vocabulary.

```
OrOp              ::= 'or'                                                 ;  (§4.4.7, 007-74)
AndOp             ::= 'and'                                                ;  (§4.4.7, 007-74)
BitOrOp           ::= '|'                                                  ;  (§4.4.2, 007-73)
BitXorOp          ::= '^'                                                  ;  (§4.4.2, 007-73)
BitAndOp          ::= '&'                                                  ;  (§4.4.2, 007-73)
RangeOp           ::= '..'                                                 ;  (§4.4.7, 007-74)
EqualityOp        ::= 'is' | 'is' 'not'                                    ;  (§4.4.4, 007-197)
OrderingOp        ::= '<' | '<=' | '>' | '>='                              ;  (§4.4.3, 007-74)
ShiftOp           ::= '<<' | '>>'                                          ;  (§4.4.2, 007-74)
AdditiveOp        ::= '+' | '-' | '+%' | '-%' | '+|' | '-|' | '+?' | '-?'  ;  (§4.6, 007-79)
MultiplicativeOp  ::= '*' | '/' | '\\' | '%'
                    | '*%' | '\\%' | '%%'
                    | '*|' | '\\|' | '%|'
                    | '*?' | '\\?' | '%?'                                  ;  (§4.6, 007-79)
```

**Operator precedence table.** The canonical, complete precedence
table — including tiers 0a (`with`), 0b (`where`), the prefix `dyn` /
`move` admissions at tier 13, and the bare `T(x)` cast at tier 14 —
is **Appendix A** of this document. Appendix A annotates each tier
with the §5 production that realises it. The per-tier nonterminal
tower at the top of §5 (`OrExpr` … `MultiplicativeExpr`) plus
`WithExpr`, `WhereFilterExpr`, `PipeExpr`, and `UnaryExpr` implements
that table directly; a recursive-descent implementer follows the
tower without further consultation.

;  (§4.4.7, 007-74, 007-78)

// **`is not` is a greedy two-token compound (per §4.4.4, 007-197).** When
//  the parser is in *infix-completion* position (i.e., an `EqualityOp` is
//  expected after a `UnaryExpr` head) and sees the token sequence
//  `'is' 'not'`, it consumes both tokens as a single `EqualityOp`
//  meaning "is not". `a is not b` desugars to `not (a is b)` per 007-197;
//  the grammar treats it as one operator at tier 9. Outside the
//  infix-completion position, `not` is parsed as the prefix operator of
//  tier 4.
// **`not` is *prefix only*, never infix.** The §4.4.7 entry "tier 4: `not`
//  (prefix)" lifts to the `UnaryExpr` alternative of §5.4 with a
//  precedence-aware parser that applies prefix `not` at tier 4 — bound to
//  the entire comparison that follows.
// **Comparison does not chain (per §4.4.3).** Tier 9 is *non-associative*:
//  `a < b < c` does parse (the grammar admits the production via two
//  tier-9 applications) but the type system rejects it because the
//  intermediate `<` produces a `bool` that does not order against the
//  third operand. This is a post-parse semantic check, surfaced as a
//  type error rather than a parse error.
// **Range `..` does not chain (per §4.4.7).** Tier 8 is *non-associative*:
//  `a..b..c` is rejected by the parser because the production admits
//  only one `RangeOp` application at the head and `..` does not
//  re-enter at its own tier.
// **Bitwise vs logical precedence (per §4.4.7 final note).** Bitwise
//  operators (`&`, `|`, `^`) bind *tighter than the logical operators*
//  but *looser than comparison* — the C convention. `a & b is c` parses
//  as `a & (b is c)`. Parenthesize when the other grouping is meant.
// **`|` is bitwise OR, `|>` is operator application.** The lexer emits
//  `|>` as a single token per §2.12, so the discrimination is at the
//  lexer level; `|>` binds at tier 1 (§5.25), `|` at tier 5.
// **Cast-policy forms are postfix, not infix.** Despite their `%` / `|` /
//  `?` lexeme overlap with arithmetic-policy operators, the call-like
//  forms `T%(x)` / `T|(x)` / `T?(x)` (§5.6) bind at the postfix tier 14;
//  the parser's discrimination is the `<type-name> <marker> '('` shape
//  preceding the marker.
// **`as` is not in the precedence table.** `as` is a naming / aliasing
//  keyword (per §2.4, 002-9), not a value operator; explicit conversion
//  uses the call forms `T()` / `T%()` / `T|()` / `T?()` at the postfix
//  tier.

### 5.8 Range `a..b` (non-associative)

```
RangeExpr         ::= Expr '..' Expr                          // documentation-only overlay; canonical path is BinaryExpr via RangeOp at tier 8
                    ;  (§4.4.7, 007-74)
```

// **Documentation-only overlay.** `RangeExpr` is not referenced from any
//  other production; the canonical parse path for `a..b` is `BinaryExpr`
//  with `BinaryOp ::= RangeOp` at tier 8 (§5.7). The named production is
//  retained here as a documentation shape for the reader; a parser
//  implementer wires range through the precedence machinery of §5.7
//  alone.
// **Non-associative (per §4.4.7).** `a..b..c` is a parse error because
//  tier 8 admits at most one `..` application at its tier. To form a
//  range whose endpoint is itself a range, parenthesize.
// **Binds looser than arithmetic (per §4.4.7).** `0..n + 1` parses as
//  `0..(n + 1)` because tier 11 (additive) binds tighter than tier 8.
// **Open-ended forms admissible only inside `[…]` slice context (per
//  §9.3.7 / §5.10).** A bare `let r = k..` is a parse error; `k..` is
//  admitted only inside the bracket of an `IndexAccess` per §5.10. The
//  closed form `a..b` is admissible in any expression position.
// **Range vs slice-type `..` (per §3.7).** The same `..` token is the
//  type-level slice marker; position disambiguates — `..` inside a type
//  context is the slice marker, `..` inside an expression context is the
//  range operator.

### 5.9 `^k` from-end indexing

```
FromEndIndex      ::= '^' UnaryExpr                            // inside IndexAccess only
                    ;  (§9.3.7, 012-87)
```

// **`^` is a from-end prefix only inside `[…]` (per §9.3.5).** The
//  `FromEndIndex` form is admissible only as an `IndexArg` (§5.2) — the
//  bracket of a slice or index access — or directly after `..` within
//  such a bracket. Outside `[…]`, `^` is the bitwise-XOR operator of
//  tier 6 (§5.7).
// **Combines with `RangeIndex`.** `arr[^2..]`, `arr[..^1]`, and
//  `arr[^3..^1]` are admissible (per §9.3.5); each end of the range
//  may independently be a plain `Expr` or a `FromEndIndex` form.
// **The operand is a `UnaryExpr`, not a bare literal (per §9.3.5).**
//  `arr[^(n+1)]` is admissible — the operand may be a computed
//  expression — bound by the parser at the unary tier so that
//  `arr[^a + b]` parses as `arr[(^a) + b]` (i.e., `^` binds tighter
//  than `+`). For the alternate grouping, parenthesize.

### 5.10 Slicing (`arr[2..5]`, open-ended `[..n]` / `[k..]` / `[..]`)

```
RangeIndex        ::= IndexHead? '..' IndexHead?              // inside IndexAccess only
                    ;  (§9.3.7, 012-86)

IndexHead         ::= Expr | FromEndIndex
                    ;  (§9.3.7, 012-86)
```

// **`IndexArg` commit-on-`..` rule.** Inside an `IndexAccess` bracket,
//  the parser tentatively scans an `IndexHead` (or sees the empty
//  case). On encountering a `..` token in this position the parser
//  *commits to `RangeIndex`* and continues with the optional trailing
//  `IndexHead`; otherwise it backtracks (or, equivalently, commits to
//  `Expr` if the head was non-empty, or `FromEndIndex` if the head
//  was `^...`). This commit-on-`..` rule removes the ambiguity between
//  `IndexArg ::= Expr` and `IndexArg ::= RangeIndex` for an LL(k)
//  implementer.

// **Open-ended forms admissible *only* inside slice-index `[…]` (per
//  §9.3.7, §9.3.5 "Open-ended ranges").** The three forms `[..n]`,
//  `[k..]`, `[..]` are sugar for `[0..n]`, `[k..arr.length]`, and
//  `[0..arr.length]` respectively. The grammar admits the partial
//  forms only as an `IndexArg` inside `IndexAccess` (§5.2) — a bare
//  expression `let r = k..` or `let r = ..` outside `[…]` is a parse
//  error per the §5.8 closed-form-only rule.
// **Both ends `FromEndIndex`-eligible.** Either or both endpoints may be
//  a `FromEndIndex` form, e.g. `arr[^3..^1]` (per §9.3.5).
// **Single `RangeIndex` per `IndexArg`.** Multiple `,`-separated
//  `IndexArg` entries are admissible (the `IndexAccess` production has
//  the comma-list form), each independently a `RangeIndex`, plain
//  `Expr`, or `FromEndIndex`. Multidimensional indexing is a sequence of
//  `IndexArg`s in one bracket.
// **`RangeIndex` differs from `RangeExpr` (§5.8) only at the open-ended
//  positions.** In a non-slice context `a..b` is the closed-form
//  `RangeExpr` of §5.8; in a slice-index context `a..b` parses via the
//  same shape but is admitted under `RangeIndex` to permit the
//  optional `IndexHead?` endpoints.

### 5.11 Delete `delete m[k]` (mandatory keyed index target)

```
DeleteExpr        ::= 'delete' DeleteTarget
                    ;  (§9.5.6, 007-239)

DeleteTarget      ::= PrimaryExpr DeleteSegment* DeleteKeyTail
                    ;  (§9.5.6, 007-239)

DeleteSegment     ::= FieldAccess
                    | TupleIndex
                    | CallSuffix
                    | IndexAccess                              // upstream multi-key reads admitted
                    ;  (§9.5.6, 007-239)

DeleteKeyTail     ::= '[' Expr ']'                             // mandatory single-key trailing index
                    ;  (§9.5.6, 007-239)
```

// **Mandatory keyed-target rule (per 007-239, §4.9.5).** The `delete`
//  keyword is a prefix-operator surface that *requires* a keyed-indexing
//  target. A bare `delete <expr>` (without a trailing `'[' Expr ']'`) is
//  a parse error. The grammar guarantees that every successful `delete`
//  parse targets a `Deletable[K]` dispatch site — never a bare binding.
// **The keyed-target shape is a postfix `IndexAccess` with exactly one
//  `IndexArg` that is a plain `Expr`.** `DeleteTarget` is structured so
//  the *trailing* index `DeleteKeyTail` is consumed by `DeleteExpr`
//  (not by upstream `DeleteSegment*` postfix steps): only a single-key
//  index is admissible at the tail — multi-key, slice, and from-end
//  forms are parse errors at this position (per 007-239). Earlier
//  `IndexAccess` segments in the prefix path may carry multi-key reads
//  (they merely navigate to the container being mutated).
// **`delete` is a top-level expression form, not a `Postfix` of
//  another expression.** Its result type is `()` (the deletion is a
//  side effect on the keyed container); `delete` does not chain with
//  postfix forms — `(delete m[k]).foo` is a parse error.
// **`delete` is a keyword (§2.4, 002-11).** The two-token sequence
//  `delete (...)` without a bracket index is rejected at parse time
//  with the same diagnostic class as `delete <expr>` (no keyed target).

### 5.12 String interpolation expression (`{expr}` inside string literal)

```
StringInterpExpr  ::= StringLit                                // see §2.9 for lexer-mode rules
                    ;  (§9.1.9, 012-42)

InterpExpr        ::= Expr                                     // inside a STR_INTERP_OPEN…STR_INTERP_CLOSE bracket pair
                    ;  (§9.1.9, 012-42)
```

// **`{expr}` is a full `Expr` (per §9.1.9).** The lexer's STR/EXPR mode
//  automaton (§2.9) emits `STR_INTERP_OPEN` and `STR_INTERP_CLOSE`
//  around the interpolation; the parser consumes a full `Expr`
//  between them. Interpolation expressions may themselves contain
//  string literals (the lexer re-enters STR mode), so interpolation
//  nests to arbitrary depth via the mode stack.
// **No format-specifier mini-language (per §9.1.9).** `{expr}` always
//  formats via the `Display` trait; width / precision are produced by
//  method calls inside the `Expr` (e.g., `{value.to_string_padded(8)}`).
//  The grammar admits no `:format` tail inside the braces.
// **`\{` is the literal-brace escape (per §9.1.9, 012-42).** Inside a
//  string literal, `\{` emits a literal `{` and suppresses interpolation
//  at that position. There is no `{{` / `}}` doubling form.
// **A `}` at a deeper bracket depth inside the interpolation does
//  *not* close the interpolation.** Only a `}` at the interpolation's
//  bracket depth zero closes it — see the §2.9 lexer-mode automaton.

### 5.13 `with` expression (single-line + multi-line forms)

`WithExpr` is defined at the top of §5 (above `WhereFilterExpr`) as
`WhereFilterExpr ( WithSuffix )*` — `with` is the loosest postfix update
and binds *outside* every binary operator (per §6.1.5). The `WithSuffix`
forms below are the surface inline and block variants.

```
WithSuffix        ::= WithInline
                    | WithBlock
                    ;  (§6.1.5, 009-26)

WithInline        ::= 'with' WithItem ( ',' WithItem )*
                    ;  (§6.1.5, 009-24)

WithBlock         ::= 'with' ( MergeSource ( ',' MergeSource )* )? ':' INDENT WithBlockBody DEDENT
                    ;  (§6.1.5, 009-25)

WithBlockBody     ::= NamedFieldOverride ( NEWLINE NamedFieldOverride )* NEWLINE?
                    ;  (§6.1.5, 009-25)

WithItem          ::= NamedFieldOverride
                    | MergeSource
                    ;  (§6.1.5, 009-23)

NamedFieldOverride ::= IDENT ':' Expr
                    ;  (§6.1.5, 009-23)

MergeSource       ::= Expr
                    ;  (§6.1.5, 009-29)
```

// **Two surface forms only (per §6.1.5).** A single `with` carries
//  either an inline comma-separated tail (`base with name: "new"`,
//  `base with other1, other2`) *or* a colon-introduced indented body
//  (`base with: ↵ name: "new"`). Mixing the two forms within one
//  `with` is a parse error per the §6.1.5 "These are the only two
//  surface forms" rule. The block form may also carry a same-line
//  merge-source list ahead of the `:` (`base with other1, other2:`).
// **Chaining is permitted and left-associative (per §6.1.5).** `base
//  with x: 1 with y: 2` parses as `(base with x: 1) with y: 2`. The
//  grammar admits chaining naturally via `WithSuffix` being a `Postfix`
//  alternative attached to a `PostfixExpr` head.
// **Low-precedence postfix.** `with` binds looser than construction and
//  the arithmetic / call / `|>` / `where` operators (per §6.1.5
//  "Associativity, precedence, and nesting" and Appendix A tier 0a),
//  so the *base* and each override value are parsed as complete
//  operator-tower expressions before the update applies. Implemented as
//  `WithExpr ::= WhereFilterExpr ( WithSuffix )*` at the top of §5.
// **Block-form body holds field overrides only (per §6.1.5).** Merge
//  sources appear *only* in the head before `:` (e.g. `base with src1,
//  src2:`); the indented body is restricted to `NamedFieldOverride`
//  entries. A merge-source inside the body is a parse error.
// **`WithItem` discrimination.** Each item is either a *named field
//  override* `IDENT ':' Expr` or a *merge source* `Expr`. The parser
//  discriminates on the first two tokens of the item: an `IDENT`
//  immediately followed by `:` selects `NamedFieldOverride`; any other
//  shape selects `MergeSource`.
// **`with` as a call argument (per §6.1.5).** A `with` carrying a
//  comma-list at the inline form must be parenthesised when used as a
//  call argument so its commas are not read as further arguments:
//  `g((r with x: 1, y: 2))`. The grammar does not add a special
//  production for this; the standard `ParenExpr` (§5.1) handles it,
//  and the comma-as-arg-separator rule of §5.5 disambiguates.

### 5.14 Array literals + comprehensions + repeat form

```
ArrayExpr         ::= ArrayLiteral
                    | ArrayComprehension
                    | ArrayRepeat
                    ;  (§9.3.1, 012-78)

ArrayLiteral      ::= '[' ( Expr ( ',' Expr )* ','? )? ']'
                    ;  (§9.3.1, 012-78)

ArrayComprehension ::= '[' 'for' Pattern 'in' Expr ':' Expr ']'
                    ;  (§9.3.1, 012-80)

ArrayRepeat       ::= '[' 'for' Expr ':' Expr ']'
                    ;  (§9.3.1, 012-81)
```

// **Three forms (per §9.3.1).** A bracketed expression is either an
//  ordinary element list `[e1, e2, ...]`, an array comprehension
//  `[for i in iterable: body]`, or a repeat form `[for N: v]`. The
//  parser discriminates on the second token: `'for'` selects one of
//  the comprehension forms; anything else (including `']'` for the
//  empty `[]`) selects `ArrayLiteral`.
// **Comprehension vs repeat discrimination (per §9.3.1).** Inside the
//  `[for ...]` head, the parser discriminates on the token following
//  the head expression: an `'in'` keyword selects `ArrayComprehension`
//  (a binding-and-iterable form); anything else selects `ArrayRepeat`
//  (a bare compile-time-count form with no binding). The
//  comprehension's `Pattern` follows the irrefutable-context rules of
//  §4.7.
// **Empty `[]` requires type annotation (per §9.3.1).** A bare empty
//  `[]` is grammatically admissible as `ArrayLiteral`; its element
//  type must be inferred from context (e.g., a binding annotation
//  `let xs: i32[0] = []`). The annotation requirement is a post-parse
//  semantic check, not a grammar rule.
// **No `if`-filter (per §9.3.1).** The comprehension head admits no
//  filter clause — an array's length is part of its type and must be
//  compile-time known. Filtered or dynamically-sized construction
//  uses stdlib `Vec` (§9.3.6) instead. The grammar does not admit any
//  `if` / `where` tail inside an `ArrayComprehension`.
// **Compile-time-known iterable / count.** The `Expr` in
//  `ArrayComprehension` (the iterable) and `ArrayRepeat` (the count
//  `N`) must be compile-time evaluable — a post-parse semantic check,
//  not a grammar restriction.
// **Inside `[...]` layout is suspended (per §2.1).** Newlines inside
//  the bracket are treated as whitespace; multi-line array literals
//  are a single logical expression.

### 5.15 Map literals (incl. empty `{}` requires type annotation)

```
MapExpr           ::= '{' ( MapEntry ( ',' MapEntry )* ','? )? '}'
                    ;  (§9.5.1, 012-97)

MapEntry          ::= ColonEntry
                    | BracketKeyEntry
                    ;  (§9.5.1, 012-97)

ColonEntry        ::= Expr ':' Expr
                    ;  (§9.5.1, 012-97)

BracketKeyEntry   ::= '[' Expr ']' ':' Expr                    // const-keyed composite per §9.5.12
                    ;  (§9.5.12, 012-118)
```

// **Two entry forms (per §9.5.1, §9.5.12).** The colon form
//  `<key>: <value>` is the ordinary map-entry shape. The bracket-colon
//  form `['<key>']: <value>` is equivalent to the colon form for
//  string and other const-evaluable keys and parallels the
//  const-indexed-array access form; both produce identical
//  compile-time-known slot paths for const-key composite literals
//  (§9.5.12).
// **Empty `{}` requires type annotation (per §9.5.1).** A bare empty
//  `{}` is grammatically admissible as `MapExpr`; its element types
//  (`K`, `V`) must be inferred from context (e.g., `let empty:
//  Map[string, i32] = {}`). The annotation requirement is a post-parse
//  semantic check.
// **`{...}` is a map literal here, *not* a record-construction surface
//  or a block.** Record construction uses `Type(name: value)` parens
//  per §6.1.3 (no braces); blocks live in `BlockExpr` (§5.17). The
//  same `{` token is also the leader of an interpolation expression
//  inside a string literal (§2.9, §5.12) — the lexer's STR-mode
//  discrimination handles that case.
// **Duplicate keys are a compile error (per §9.5.1).** Post-parse
//  semantic check; the grammar admits arbitrary key expressions in
//  the entry list.
// **Inside `{...}` layout is suspended (per §2.1).** Newlines inside
//  the braces are treated as whitespace; multi-line map literals are
//  a single logical expression.

### 5.16 Tuple literals (trailing comma rule)

```
TupleExpr         ::= '(' ')'                                  // unit value
                    | '(' Expr ',' ')'                         // 1-tuple
                    | '(' Expr ( ',' Expr )+ ','? ')'          // n-tuple, n >= 2
                    ;  (§9.2.4, 012-58)
```

// **Trailing-comma rule (per §9.2.4, 012-58).** The single-component
//  alternative `'(' Expr ',' ')'` *requires* the trailing comma. The
//  comma is what discriminates the 1-tuple value `(x,)` from a
//  parenthesised expression `(x)`, the latter being the `ParenExpr`
//  alternative of §5.1 with no tuple meaning. Without the comma the
//  parser takes `ParenExpr`.
// **Unit `()` is the zero-component form (per §9.2.3).** It denotes
//  the unique value of the unit type; the parser admits the same
//  surface for both the value and the type (§3.5) — position
//  disambiguates.
// **N-tuple optional trailing comma (per §9.2.4, 012-59).** `(a, b)` and
//  `(a, b,)` denote the same value.
// **Tuple field access uses `.0` / `.1` / ... per §5.2 `TupleIndex`.**

### 5.17 Block expressions

```
BlockExpr         ::= ColonBody
                    ;  (§1.4, 002-25)

ColonBody         ::= ':' INDENT BlockBody DEDENT              // colon-introduced indented block
                    | ':' InlineExpr                           // colon-introduced inline
                    ;  (§1.4, 002-25)

BlockBody         ::= BlockItem ( NEWLINE BlockItem )* NEWLINE?
                    ;  (§1.4, 002-25)

BlockItem         ::= LetStmt | MutStmt | ConstStmt            // §6.1
                    | AssignStmt                               // §5.26 + §6.1
                    | ForStmt | WhileStmt | BreakStmt          // §6.2–§6.4
                    | ContinueStmt | ReturnStmt
                    | Expr                                     // expression statement
                    ;  (§1.4, 002-26)

InlineExpr        ::= Expr
                    ;  (§1.4, 002-26)
```

// **Body-shape inheritance from §2.2.** A `BlockExpr` follows the
//  general body-shape rule of §2.2: the colon `:` opens either an
//  indented `BlockBody` or a same-line `InlineExpr` terminated by
//  NEWLINE. The owning construct decides which alternative is admissible
//  per the body-shape categories of §2.2 (always-indented vs
//  may-be-inline); a `BlockExpr` itself accepts both alternatives.
// **The block's value is the last `BlockItem` when it is an `Expr`.**
//  Post-parse semantic rule; the grammar does not distinguish a
//  "tail-expression" position from intermediate items.
// **Bare `BlockExpr` is *not* admissible as a `PrimaryExpr` outside
//  contexts that name a block.** A free-standing `:`-introduced block in
//  the middle of an expression position is a parse error; blocks attach
//  to constructs (fn body, `if`/`else`, `match` arm, `with`, etc.) that
//  syntactically introduce the `:`.

### 5.18 If/else-if/else expression (mandatory final else)

```
IfExpr            ::= 'if' Expr ':' IfArmBody ElseIfClause* ElseClause
                    ;  (§6.4, 009-123)

ElseIfClause      ::= 'else' 'if' Expr ':' IfArmBody
                    ;  (§6.4, 009-123)

ElseClause        ::= 'else' ':' IfArmBody
                    ;  (§6.4, 009-125)

IfArmBody         ::= InlineExpr
                    | INDENT BlockBody DEDENT
                    ;  (§6.4, 009-123)
```

// **Mandatory final `else` (per §6.4, 009-125).** An `IfExpr` *must*
//  terminate with an `ElseClause`. An `if` chain without a closing
//  `else` is a parse error. Because an `if` produces a value (it is
//  an expression form), every path must yield a defined value; a
//  unit-typed chain still requires `else: ()` (or its block
//  equivalent) per 009-125.
// **`else if` is two tokens (per §6.4).** The keyword sequence
//  `'else' 'if'` is a literal extension of the chain to any depth, not
//  nesting sugar for `else: if ...`. The parser admits any number of
//  `ElseIfClause`s before the mandatory `ElseClause`.
// **Inline vs block forms (per §6.4 "Inline form" / "Block form").**
//  Each arm body uses the body-shape rule of §2.2 — an inline
//  expression after `:` or an indented block. The two forms may mix
//  within one chain (one arm inline, another a block); the grammar
//  admits each arm independently.
// **Column-alignment of `else` / `else if` (per Appendix D, 002-30).**
//  In the block form, `if`, `else if`, and `else` heads align at the
//  same column. The column-alignment rule is the layout-time
//  discrimination of `else if` chains from independent nested `if`s.
// **`else:` on a loop is unrelated (per §6.4).** The `else:` clause
//  on a `for` / `while` loop (§6.4 here, §12.6.1 in SPEC) reuses the
//  `else` keyword for loop-natural-completion — it is *not* part of an
//  `if` chain. The owning-construct context disambiguates.

### 5.19 Match expression

```
MatchExpr         ::= 'match' Expr ':' INDENT MatchArm+ DEDENT
                    ;  (§6.2.4, 009-86)

MatchArm          ::= Pattern ':' IfArmBody
                    ;  (§6.2.4, 009-86)
```

// **One scrutinee, indented arm list (per §6.2.4).** A `match`
//  evaluates its scrutinee `Expr`, selects one arm by `Pattern` match,
//  and yields that arm's body value. The arm list is always indented
//  under the `match` head — the always-indented body shape of §2.2.
// **Arm body uses the inline-or-block rule of §2.2.** Each arm's
//  body is either a single inline expression following the `:` or an
//  indented block — same shape as an `if` arm (§5.18) and a function
//  body (§7.6).
// **Pattern is any `Pattern` from §4 (refutable-permitting context
//  per §4.7).** Variant, tuple, record, newtype, literal, wildcard,
//  and binding (catch-all) patterns are all admissible. A bare
//  identifier in arm-head position acts as a catch-all (per 009-88).
// **Exhaustiveness (per §6.2.5).** A `match` must be exhaustive over
//  the scrutinee's type; a non-exhaustive `match` is a *compile error*,
//  not a runtime trap (per §6.2.5, §8.1). This is a post-parse
//  semantic check; the grammar admits any arm list.
// **Arms are tried in declaration order; first match wins (per
//  §6.2.4).** Semantic rule.
// **`match` returns a value (per §6.2.5).** Distinct from `given`
//  (§5.22) which gates structure; the two share arm shape but differ
//  in operation.

### 5.20 Reactive `observe` expression (`on` / `where` / `default` arms)

```
ObserveExpr       ::= 'observe' ':' INDENT OnArm+ DefaultArm? DEDENT
                    ;  (§13.2.11, 016-240)

OnArm             ::= 'on' OnTarget ObserveArmWhereFilter? ':' IfArmBody
                    ;  (§13.2.11.1, 016-241)

OnTarget          ::= Path                                     // single trigger cell
                    | '(' Path ( ',' Path )+ ','? ')'          // multi-cell trigger set, n >= 2
                    ;  (§13.2.11.4, 016-256)

ObserveArmWhereFilter ::= 'where' Expr                         // boolean predicate per §13.18.10
                    ;  (§13.2.11.1, 016-241)

DefaultArm        ::= 'default' ':' IfArmBody                  // must be last (encoded in ObserveExpr)
                    ;  (§13.2.11.5, 016-242)
```

// **Arm shape (per §13.2.11.1).** Each arm is either an `on`-clause
//  arm with one or more trigger cells (optional `where` filter), or a
//  `default:` arm with no trigger. Arms are indented under the
//  `observe:` head — always-indented body shape (§2.2).
// **Multi-cell trigger sets are parenthesised (per §13.2.11.4).** A
//  multi-cell `on` clause uses a parenthesised comma list
//  `on (T1, T2, T3):`. The parens are mandatory for n ≥ 2; a single
//  trigger cell uses the bare `Path` form `on T1:` with no parens.
// **`where` filter is per-arm (per §13.2.11.1, §13.18.10).** The
//  optional `where Expr` tail filters arm-selection candidates by a
//  boolean predicate. Distinct from the binary `A where C` stream
//  filter of §5.23 — here `where` attaches to an `on` clause as part
//  of an observe arm; in §5.23 it is a binary operator over an
//  expression.
// **`default:` must be the last arm (per §13.2.11.5).** A `default:`
//  arm appearing before any `on` arm is a compile error (post-parse
//  semantic check). It supplies the observe's value before the first
//  activating trigger fires.
// **Arm body uses the inline-or-block rule of §2.2 (per §13.2.11.1).**
//  Each arm body is either a single inline expression or an indented
//  block ending in a final expression — same shape as `match` /
//  function body.
// **Parenthesisation as a sub-expression (per §13.2.11.1).** Used as
//  a sub-expression or call argument, `observe` must be parenthesised
//  (`f((observe: ...))`) because its `observe:`-introduced block is
//  otherwise open-ended; the parentheses fix where it ends, the same
//  self-delimiting requirement that governs `when` / `/expr` operands
//  in placements (§13.8.10).

### 5.21 `when` block expression (simple + multi-way)

```
WhenBlockExpr     ::= WhenSimple
                    | WhenMultiWay
                    ;  (§13.9.12, 022-88)

WhenSimple        ::= 'when' Expr ':' INDENT WhenBody DEDENT WhenOtherwise?
                    ;  (§13.9.12, 022-93)

WhenOtherwise     ::= 'otherwise' ':' INDENT WhenBody DEDENT
                    ;  (§13.9.12, 022-93)

WhenMultiWay      ::= 'when' ':' INDENT GuardArm+ OtherwiseGuardArm? DEDENT
                    ;  (§13.9.12, 022-98)

GuardArm          ::= Expr ':' IfArmBody
                    ;  (§13.9.12, 022-98)

OtherwiseGuardArm ::= 'otherwise' ':' IfArmBody                // must be last
                    ;  (§13.9.12, 022-101)

WhenBody          ::= BlockBody                                // context-dependent contents
                    ;  (§13.9.12, 022-89)
```

// **Two forms (per §13.9.12).** The *simple* form `when cond:`
//  introduces a then-body, optionally followed by a sibling
//  `otherwise:` block (the `WhenOtherwise` clause sits at the same
//  column as `when` per Appendix D). The *multi-way* form `when:`
//  (no head condition) introduces a list of guard arms whose body
//  shape parallels `match`.
// **Simple-form discrimination from multi-way.** The parser looks at
//  the token following `when`: a colon `:` selects `WhenMultiWay`;
//  any other token (the start of an `Expr`) selects `WhenSimple`.
// **`otherwise:` must be last in multi-way form (per §13.9.12).** A
//  non-last `otherwise:` is a compile error; the grammar admits it
//  syntactically and the parser surfaces the position rule as a
//  post-parse check.
// **`when` block lives in structural contexts only (per §13.9.12).**
//  `WhenBlockExpr` is admissible in `expose:` clauses, node bodies,
//  placement bodies, `effects:` clauses, and effect `desired:` blocks
//  — *not* as an inline modifier or in arbitrary value position. The
//  enclosing construct controls admissibility; the grammar does not
//  restrict the position.
// **Arm body contents are context-dependent.** A `WhenBody` may hold
//  placements, effect entries, or desired-cell declarations
//  depending on its enclosing context (per §13.9.12). The grammar
//  reuses `BlockBody` here; the parser delegates content
//  admissibility to the enclosing construct.
// **`when:` block guards are boolean expressions, not patterns.** A
//  `GuardArm`'s head is an `Expr` evaluated for truthiness; this is
//  the structural distinction from `given` (§5.22), whose arm heads
//  are *patterns* over a scrutinee. Per §13.9.12, do not use
//  `when x is Variant` guards for discriminant selection — use
//  `given` (§5.22) for exhaustiveness checking.

### 5.22 `given` block expression (variant arms)

```
GivenBlockExpr    ::= 'given' Expr ':' INDENT GivenArm+ DefaultGivenArm? DEDENT
                    ;  (§13.9.13, 022-106)

GivenArm          ::= Pattern ':' IfArmBody
                    ;  (§13.9.13, 022-108)

DefaultGivenArm   ::= 'default' ':' IfArmBody                  // must be last
                    ;  (§13.9.13, 022-119)
```

// **Header and arms (per §13.9.13).** `given <scrutinee>:` introduces
//  a list of variant-pattern arms whose shape is *exactly* `MatchArm`
//  (per §5.19): bare `Pattern: body`, no per-arm keyword. The
//  dedicated `given` header is what disambiguates an arm such as
//  `Realtime: RealtimeChain` from a connection placement
//  `Name: dest` (per §13.3.7.1) — inside a `given` block every line
//  at the arm indent is an arm.
// **Exhaustive over the scrutinee's variants (per §13.9.13).** A
//  `given` is a *closed* selector; non-exhaustive coverage is a
//  compile error (parallel to `match`, §6.2.5). The grammar admits
//  any arm list; exhaustiveness is post-parse.
// **`default:` arm is the explicit catch-all (per §13.9.13).** When
//  present, `default:` must be the *last* arm; a non-last `default:`
//  is a compile error. It suppresses the exhaustiveness obligation —
//  the author opts out knowingly, exactly as a catch-all does in
//  value `match` (§6.2.4).
// **Live payload binding (per §13.9.13).** Arms bind variant
//  payloads exactly as `match` patterns do (`Active(session):`
//  binds `session`); the binding is a *live reactive projection* of
//  the scrutinee's current payload, not a snapshot. The pattern
//  surface is `Pattern` (§4); the live-projection semantics is
//  post-parse.
// **Distinct from value `match` (per §13.9.13).** `match` returns a
//  value and discards unselected arms; `given` builds every arm's
//  subtree and freezes the inactive ones (Model B gate semantics,
//  §13.9.7). The same arm shape supports both operations; the
//  enclosing form selects the operation.
// **`given` block lives in structural contexts (per §13.9.13).**
//  Same admissibility as `WhenBlockExpr` (§5.21): `expose:` clauses,
//  node bodies, placement bodies, `effects:` clauses, effect
//  `desired:` blocks.

### 5.23 `A where C` stream-filter binary expression

The binary form is defined at the top of §5 as
`WhereFilterExpr ::= PipeExpr ( 'where' PipeExpr )*`. The two
operands and any chained applications are full `PipeExpr` heads.

// **Binary form, distinct from declaration `where` clauses.** The
//  binary `A where C` here is a stream-filter expression (§13.18.10):
//  `A` is a stream of element type `T`, `C` is a boolean predicate
//  evaluated per LHS event, and the output is a stream of `T`
//  emitting events of `A` whose `C` is true. This is *not* the
//  declaration-level `where` clause of §3.13 (which carries trait
//  bounds and predicates on a generic signature) — the disambiguator
//  is the syntactic context: declaration-level `where` follows a
//  signature head and admits `WherePredicate` lists; the binary form
//  here is an `PipeExpr 'where' PipeExpr` shape and admits a
//  `PipeExpr` (no `with` postfix) on each side.
// **Precedence and associativity (per 030-168, 030-169).** `where` is
//  left-associative (030-168) and sits at tier 0b of Appendix A —
//  tighter than `with` (tier 0a) and looser than `|>` (tier 1). So
//  `a where b |> c` parses as `(a where b) |> c` (NOT
//  `a where (b |> c)`) — the `where` operator binds its right
//  operand at the `PipeExpr` tier, so the trailing `|> c` cannot
//  re-enter at the `where` level.
// **LHS must be a stream (per §13.18.10).** A signal must be
//  converted to a stream explicitly first (`signal |> to_ring_stream`).
//  This is a post-parse semantic check.
// **Reference to the LHS event inside `C` (per §13.18.10.2).** The
//  LHS stream's *name* (e.g. `clicks` in `clicks where clicks.x > 100`)
//  refers to the current event; other reactive cells reference their
//  current committed values. The grammar treats the LHS name like
//  any other `Path` in `C`; the live-event projection semantics is
//  post-parse.

### 5.24 `.previous(fb)` / `.past(k, fb)` accessor forms

```
PreviousCall      ::= '.' 'previous' '(' Expr ')'              // sugar for .past(1, fb)
                    ;  (§13.2.4.3, 016-75)

PastCall          ::= '.' 'past' '(' Expr ',' Expr ')'         // k, fallback
                    ;  (§13.2.4.3, 016-50)
```

// **Syntactically ordinary method calls (per §13.2.4.3).** The
//  surface `.previous(fb)` and `.past(k, fb)` parse as `FieldAccess`
//  (with `IDENT` = `previous` / `past`) followed by `CallSuffix`
//  (§5.2) — i.e., the same shape as any other method call. They
//  appear here as a documentation production to record the
//  semantic-level grammar of the recurrent self/input-history
//  accessors; the parser does not branch on the method name.
// **`k` is compile-time-known (per §13.2.4.3).** The first argument
//  to `.past(k, fb)` must be a compile-time-known positive `usize` —
//  a literal, a `const`, or a const-generic parameter. Runtime or
//  reactive values are rejected. This is a post-parse semantic
//  check, not a grammar restriction; the grammar admits any `Expr`.
// **`fallback` is an expression of the accessed cell's value type
//  (per §13.2.4.3).** Returned when fewer than `k` commits have
//  happened. Post-parse type check.
// **Each call is independent (per §13.2.4.3).** Multiple calls on
//  the same cell with different fallbacks are independent; each
//  returns its own fallback when no history exists. The grammar does
//  not de-duplicate calls.
// **Method-name spellings `previous` / `past` are reserved by the
//  reactive subsystem.** They are ordinary `IDENT` lexemes at the
//  parse level; the resolver attaches the recurrent-history semantics
//  per §13.2.4.3 (a post-parse rule, not a grammar rule).

### 5.25 Operator-pipe `|>` (operator/effect application)

`PipeExpr` is defined at the top of §5 as
`OrExpr ( '|>' PipeRhs )*`. The RHS form is defined here.

```
PipeRhs           ::= Path CallSuffix?                         // operator or effect call
                    ;  (§13.17.7, 029-66)
```

// **Low-precedence, left-associative (per §13.17.7).** `|>` binds at
//  tier 1 of Appendix A — looser than every other operator. `a + b
//  |> op` parses as `(a + b) |> op`; `a |> op1 |> op2` parses as
//  `(a |> op1) |> op2`.
// **LHS must be a reactive cell type or a statically convertible
//  value (per §13.17.7).** A static value (literal, `const`,
//  compile-time constant expression) is wrapped as a degenerate
//  `derived T` cell automatically. This is a post-parse semantic
//  check; the grammar admits any `BinaryExpr` on the LHS.
// **RHS must be an operator call or an effect call (per §13.17.7).**
//  Using `|>` with a `fn` is a compile error — diagnostic class:
//  "`|>` requires an operator or effect on the right-hand side." The
//  grammar admits any `Path CallSuffix?` shape; the operator-vs-fn
//  discrimination is post-parse via name resolution.
// **`|>` is a distinct lexeme from `|` (per §2.12, §4.4.2).** The
//  lexer emits `|>` as a single token; bitwise `|` (tier 5) and `|>`
//  (tier 1) never collide at the parse level.
// **Bare RHS without argument list.** When the RHS operator/effect
//  takes only the implicit pipe target (its first positional
//  parameter is the LHS), the `CallSuffix?` may be omitted —
//  `source |> peak_detector` per the §13.17.5 example. The grammar
//  admits both shapes.

### 5.26 Place-assignment LHS path (`r.a.b = x`, `arr[i].field = y`)

```
AssignStmt        ::= AssignExpr                               // statement form per §6.1; AssignExpr defined at top of §5
                    ;  (§11.11, 013-204)

PlaceLhs          ::= IDENT PlaceSegment*
                    ;  (§11.11, 013-204)

PlaceSegment      ::= '.' IDENT                                // field projection
                    | '.' DecIntLit                            // tuple-component projection
                    | '[' Expr ']'                             // single-key index projection
                    ;  (§11.11, 013-204)
```

// **Multi-segment paths (per §11.11, 013-204).** A `PlaceLhs` is a
//  binding identifier optionally followed by any number of `.field`
//  / `.NUMERIC` / `[expr]` projections in any order. Examples:
//  `r.a.b = x`, `arr[i].field = y`, `t.0.field = z`. The path is
//  resolved left-to-right (root binding, then each segment, with any
//  index expressions evaluated in their written order per 013-204);
//  the compiler then evaluates the RHS and performs the *innermost*
//  assignment.
// **Statement-only position (per §11.11).** `AssignStmt` is
//  admissible only as a `BlockItem` (§5.17) — never as a
//  value-yielding sub-expression. `Expr`'s top-level production
//  carries both `AssignExpr` and `PipeExpr`, but `AssignExpr` is
//  promoted only in statement contexts.
// **Root must be a `mut` binding (per §11.11).** Place-assignment
//  through a `let` or `const` binding is a compile error; this is a
//  post-parse semantic check tied to the binding's declaration.
// **No alias materialised along the path (per 013-204).** Every
//  intermediate projection is itself an in-place place into storage
//  the `mut` binding owns; no borrow-equivalent alias is created
//  anywhere along the chain. This is a post-parse semantic property.
// **RHS is implicitly consumed (per §11.11).** The right-hand side
//  of an indexed / field / whole-value reassignment is consumed into
//  the storage slot. Consumption is *implicit* — the user does not
//  write `move` for the RHS; the storage assignment is structurally
//  a transfer of ownership into a slot (per §11.11 final paragraph).
// **Single-segment whole-value reassignment.** `IDENT '=' Expr` with
//  no `PlaceSegment` is the whole-value form (per §11.11.1) and is
//  admissible by the same production.
// **Index target admits exactly one `Expr` (per §11.11).** Multi-key
//  index assignment is not part of the surface here; only single-key
//  index projections are admitted in `PlaceSegment`'s
//  `'[' Expr ']'` alternative. (Multi-key read access is admitted in
//  `IndexAccess` of §5.2 for ordinary indexing.)

### 5.27 `panic(msg)` form

```
PanicExpr         ::= 'panic' '(' Expr ')'                    // documentation-only overlay; canonical path is Path + CallSuffix via PostfixExpr (§5.1, §5.2)
                    ;  (§8.2.1, 011-13)
```

// **`PanicExpr` is documentation-only.** Like `CastExpr` (§5.6) and
//  `RangeExpr` (§5.8), the named production is *not* referenced from
//  any other production — including `PrimaryExpr`. `panic` is an
//  ordinary `IDENT` (per §2.4 it is not a keyword), so the canonical
//  parse path for `panic(message)` is a `Path` (`PrimaryExpr`) head
//  with a `CallSuffix` (§5.2). The production is retained here as a
//  documentation shape so a reader can locate `panic` in one place;
//  a parser implementer wires it through `PostfixExpr` alone.

// **`panic` is an ordinary prelude function (per §8.2.1).** The
//  spelling `panic` is an `IDENT` (it is *not* a keyword per §2.4);
//  the grammar lists it here as a named production because of its
//  special return type and trap behaviour, but the parser does not
//  branch on the name — `PanicExpr` parses via the standard
//  `Path` + `CallSuffix` shape of §5.1 / §5.2.
// **Signature (per §8.2.1).** `fn panic(message: string) -> never`.
//  The `never` return type allows `panic` to appear anywhere a value
//  of any type is expected; this is a post-parse type-system
//  property (the bottom-type unification rule of §8.2.2), not a
//  grammar rule.
// **Trap behaviour is post-parse (per §8.2.3).** The grammar makes
//  no commitment about `panic`'s runtime behaviour; it is a normal
//  call at the surface and the §8.2 / §8.3 trap-track machinery is
//  attached at semantic-analysis time.

## 6. Statements and control flow

Productions for binding statements and loop / return control flow. The
authoritative statement-shape nonterminal is `BlockItem` (§5.17), which
enumerates every form admissible inside a block body. The individual
production blocks below define each form.

```
BlockItem         ::= LetStmt | MutStmt | ConstStmt            // §6.1
                    | AssignStmt                               // §5.26
                    | ForStmt | WhileStmt | BreakStmt          // §6.2–§6.4
                    | ContinueStmt | ReturnStmt
                    | Expr                                     // expression statement
                    ;  (§1.4, 002-26)
```

// A bare `Expr` is admissible as a block item; its value is discarded
//  unless it is the trailing expression of a block (see §5.17 of this
//  document, the `BlockExpr` production). `NEWLINE` separates items
//  within a block.
// Each statement alternative below is a distinct, syntactically
//  discriminable form: `let` / `mut` / `const` lead with their keyword;
//  `for` / `while` lead with their loop keyword; `break` / `continue` /
//  `return` lead with their control-flow keyword; an `AssignStmt` is
//  recognised by a `PlaceLhs` head followed by `=`; anything else parses
//  as the `Expr` alternative.

### 6.1 `let` / `mut` / `const` bindings (incl. shadowing)

```
LetStmt           ::= 'let' Pattern ( ':' TypeExpr )? '=' Expr
                    ;  (§11.2, 013-30)

MutStmt           ::= 'mut' Pattern ( ':' TypeExpr )? '=' Expr
                    ;  (§11.2, 013-30)

ConstStmt         ::= 'const' IDENT ( ':' TypeExpr )? '=' Expr
                    ;  (§2.4.1.1, 004-73)
```

// **Always-initialised (per §11.2, 013-30).** Every binding carries an
//  initial value at its declaration; there is no uninitialised `let` /
//  `mut` / `const` form. The `'=' Expr` is required for all three
//  alternatives.
// **`mut` is function-body-only (per §11.2, 013-30).** `MutStmt` is a
//  parse-time admissible form in any `Stmt` position, but the
//  surrounding-context check enforced by the parser / resolver rejects a
//  `MutStmt` whose owning block is not a function body (or a nested block
//  scope inside one): `mut` is forbidden at module top level, inside
//  type / trait / node / connection bodies, and on function parameters.
//  This is a *post-parse* check in the sense that the same token sequence
//  can be parsed in either context; the surrounding-construct kind decides
//  legality (§11.2).
// **`ConstStmt` RHS is compile-time-only (per §2.4.1.2, 004-75).** A
//  `ConstStmt`'s RHS must be compile-time-evaluable; reactive / signal /
//  derived / external-input expressions are rejected post-parse. The
//  grammar admits any `Expr` here; the const-evaluability rule is a
//  semantic check, not a syntactic one.
// **Shadowing (per §11.2.1, 013-31 / 013-32).** Either form may shadow a
//  prior binding of the same name in the same scope, with the prior
//  binding inaccessible by that name from the shadow point forward. A
//  `let` may shadow a `mut` and vice versa; the new binding's mutability
//  is governed solely by its own declaration form. There is no separate
//  grammar production for shadowing — it is the ordinary `LetStmt` /
//  `MutStmt` form recurring in scope.
// **LHS is a `Pattern` for `let` / `mut`.** The LHS uses the `Pattern`
//  nonterminal of §4 and must be irrefutable per §4.7. A destructuring
//  bind like `let (a, b) = pair` is the same `LetStmt` production with a
//  `TuplePattern` LHS. The `ConstStmt` LHS is the bare identifier form
//  only; const bindings do not destructure.
// **Optional type annotation (per §1.4, 002-1).** The `':' TypeExpr`
//  annotation is admitted on all three forms. When omitted, the type is
//  inferred from the RHS per §2.

### 6.2 `for` loop (incl. `for own`, iteration-variable pattern)

```
ForStmt           ::= 'for' 'own'? Pattern 'in' Expr Body LoopElseClause?
                    ;  (§12.3, 014-23)

LoopElseClause    ::= 'else' ColonBody                          // body shape shared with §5.17 BlockExpr
                    ;  (§12.6.1, 014-87)
```

// **Default vs `for own` form (per §12.3, 014-23 / 014-24).** The
//  optional `'own'` modifier before the iteration pattern selects the
//  consuming form: `for own x in v:` consumes `v` at loop entry. Without
//  `'own'`, the default borrow-equivalent form applies — `v` survives the
//  loop. The two forms share one production; the `'own'` token is the
//  surface discriminator.
// **`for mut` is a parse error (per §12.3.2, 014-40).** The grammar
//  *deliberately* does not admit `'mut'` in the iteration-pattern
//  position; the parser rejects `for mut x in iterable:` as a syntactic
//  error. The iteration variable is bound by the loop construct, not by
//  user declaration, and follows the same rule as the `mut`-on-parameter
//  prohibition (§11.7.2). A mutable per-iteration value is obtained by
//  rebinding inside the body — `mut local = x` (014-41).
// **Iteration variable is a `Pattern` (per §12.12.1).** The position
//  after `'for' 'own'?` admits any `Pattern` of §4; the pattern
//  destructures each yielded value. The pattern must be *irrefutable*
//  per §4.7 — iteration cannot fail to match — and the irrefutability
//  rule is enforced post-parse. To filter elements the body uses
//  `continue`; there is no `for pattern if guard in iterable:` inline-filter
//  form (§12.12.1).
// **`Body` is the standard `':' …` body (per §2.2).** A `for` loop body
//  is a may-be-inline body — `for i in 0..N: process(i)` and the indented
//  block form are both admissible (002-26). See `Body` in §2.2.
// **`LoopElseClause` dedented to the loop head's column (per §12.6.1,
//  014-90).** The optional `else:` clause is written at the loop head's
//  indentation, *dedented from the body* rather than nested inside it.
//  The layout pre-processor of §2.1 emits the `DEDENT` ending the loop
//  body before the `'else'` token, and Appendix D's column-alignment rule
//  attaches the `'else'` to the owning `for` by column match. The
//  `LoopElseClause` is shared with `WhileStmt` (§6.3).

### 6.3 `while` loop

```
WhileStmt         ::= 'while' Expr Body LoopElseClause?         // LoopElseClause defined in §6.2
                    ;  (§12.4, 014-73)
```

// **Boolean condition (per §12.4.1, 014-74).** The condition `Expr` must
//  produce a value of type `bool`. This is a semantic check, not a
//  grammar restriction; the parser admits any `Expr` in the condition
//  position.
// **No separate `loop` keyword (per §12.4.2, 014-76).** The loop-forever
//  idiom is `while true:`; there is no dedicated `loop` keyword or
//  production. The grammar does not need a special form.
// **Body shape (per §2.2).** `Body` is the same may-be-inline body
//  nonterminal as `ForStmt`'s body. An inline single-statement form
//  `while cond: do_step()` is admissible, as is the indented block form.
// **`LoopElseClause` (per §12.6.1).** The `else:` clause production is
//  shared with `ForStmt`; its evaluation rule (fires when the condition
//  becomes false, *not* on `break` or function-return exit) is semantic
//  (§12.6.1) and not enforced by the grammar.

### 6.4 `break` / `continue` / `break <value>` / loop `else:` clause

```
BreakStmt         ::= 'break' Expr?
                    ;  (§12.5, 014-78)

ContinueStmt      ::= 'continue'
                    ;  (§12.5, 014-79)
```

// **`break` with optional value (per §12.5.1, 014-80).** The optional
//  trailing `Expr` is the `break value` form; the bare `'break'` form is
//  equivalent to `break ()` (014-81). The grammar admits both with one
//  alternation. Whether a particular loop's `break` carries a value is
//  uniform within the body per §12.5.1 (mixing bare `break` and
//  `break value` with a non-unit value is a *type* error, not a parse
//  error).
// **`continue` carries no value (per §12.5.2, 014-83).** `ContinueStmt`
//  takes no operand at the grammar level; `continue` is a control-flow
//  statement only.
// **Innermost-loop targeting (per §12.5.3, 014-84).** Both statements
//  always target the innermost enclosing loop; there is no label syntax
//  in v1. The grammar offers no production for a labelled break /
//  continue.
// **Outside-a-loop is a parse error (per §12.5.4, 014-86).** A
//  `BreakStmt` or `ContinueStmt` whose nearest enclosing statement
//  context is not a `ForStmt` or `WhileStmt` body (transitively, through
//  block expressions) is rejected at parse time. This is the only
//  surrounding-context check enforced *during* parsing in §6.
// **Loop expression typing collapse (per §12.6.2).** The shape of the
//  loop expression's value type — `()` / `Option[T]` / `T` / `never` —
//  depends on the combination of `break value` sites in the body and the
//  presence of a `LoopElseClause`, with `never`-unification collapsing
//  the natural-completion arm when it is provably unreachable
//  (§12.6.2 / §12.6.4). This is a *post-parse* semantic determination;
//  the grammar does not enumerate the cases. The grammar admits any
//  `BreakStmt` / `ContinueStmt` placement inside a loop body and any
//  combination of `break` / `break value` / `else:`-clause shapes; the
//  type checker decides the result type.

### 6.5 `return`

```
ReturnStmt        ::= 'return' Expr?
                    ;  (§11.3.6, 013-30)
```

// **Optional value.** The trailing `Expr` is the returned value; the
//  bare `'return'` form returns `()` (unit). One alternation covers both
//  shapes.
// **Function-body-only (semantic).** A `ReturnStmt` is legal only inside
//  a function body (free `fn`, method body in a `fulfill` block, or
//  closure body). The surrounding-construct check is enforced post-parse
//  per §11.3.6 and §11.10 — the grammar admits a `ReturnStmt` anywhere a
//  `Stmt` is admitted; the resolver rejects the form in non-function
//  contexts.
// **Ownership transfer (semantic).** Whether `return r` transfers
//  ownership or propagates a borrow-equivalent alias depends on whether
//  `r` is a real-owner local and on the function's signature (`-> T`
//  vs `-> own T`); the rule is §11.3.6. The grammar admits any `Expr`
//  in the return position; the ownership analysis runs post-parse.

## 7. Top-level declarations

Productions for module-level declarations: visibility, use, alias, free functions, closures, records, newtypes, enums, traits, fulfill blocks, reactive operators, effects, module-level reactive declarations, top-level placements.

A top-level declaration is one of the alternatives below. The
`TopLevelDecl` entry-point gathers them; a Ductus source file is a
sequence of `TopLevelDecl`s separated by `NEWLINE` per §2.1.

```
TopLevelDecl      ::= AnnotatedDecl                            // see §12.3
                    | UseStmt
                    | AliasTypeDecl
                    | FnDecl
                    | RecordDecl
                    | NewtypeDecl
                    | EnumDecl
                    | TraitDecl
                    | FulfillItem
                    | OperatorDecl
                    | EffectDecl
                    | ModuleReactiveDecl
                    | NodeDecl
                    | ConnectionDecl
                    | TopLevelConstDecl
                    | TopLevelPlacement
                    ;  (§10.3, 003-34)

TopLevelConstDecl ::= Visibility? 'const' IDENT ( ':' TypeExpr )? '=' Expr
                    ;  (§2.4.1.1, 003-34)
```

// `TopLevelConstDecl` carries the optional `Visibility?` prefix that
//  `ConstStmt` (the function-body form, §6.1) lacks. Body-level `const`
//  inside a function uses `ConstStmt`; module-level `const` uses
//  `TopLevelConstDecl`. The two share their RHS shape; the wrapping
//  difference is the admissibility of a visibility prefix.

// `NodeDecl` is specified in §8; `ConnectionDecl` is specified in §9.
//  `ConstStmt` is the `Stmt`-form of §6.1 — a `const` declaration is
//  admissible both inside a function body and at module top level
//  (003-34). `ModuleReactiveDecl` covers the four module-level reactive
//  declaration forms enumerated in §7.15.
// Each `TopLevelDecl` alternative is distinguished by its leading
//  visibility prefix (optional, §7.1) followed by its keyword head
//  (`use`, `alias`, `fn`, `type`, `enum`, `trait`, `fulfill`, `operator`,
//  `effect`, `signal`/`derived`/`recurrent`/`stream`, `node`,
//  `connection`, `const`) or — for `TopLevelPlacement` — by the
//  optional `'main'` keyword followed by a `TypeRef` head. The parser
//  selects the alternative on the lookahead after any visibility
//  prefix.

### 7.1 Visibility prefix (public / shared / private)

```
Visibility        ::= 'public' ConstructorVis?
                    | 'shared' ConstructorVis?
                    | 'private'
                    ;  (§10.1, 003-1)
```

// `shared` is the default — absence of a `Visibility` prefix means
//  `shared` (003-3). `private` and `public` are explicit; `shared`
//  may also be written explicitly. There is no `pub` keyword
//  (003-3).
// The `Visibility` prefix is admissible on every named top-level
//  declaration enumerated in §10.3 — records, enums, newtypes,
//  alias types, traits, free fns, operators, effects, consts,
//  module-level reactive declarations, node / connection types, and
//  top-level placements (003-34). `fulfill` blocks do *not* carry a
//  visibility prefix; their reachability is derived from the trait's
//  and the type's joint visibility (§10.3 final bullet).
// `use` statements do *not* carry a visibility prefix (003-39); a
//  `use` controls how the current file refers to other names, not
//  how other files refer to the current file.
// The `ConstructorVis` suffix is admitted only on `'public'` and
//  `'shared'`; see §7.2.

### 7.2 Constructor visibility `public(private)`

```
ConstructorVis    ::= '(' ConstructorVisInner ')'
                    ;  (§10.5, 003-1)

ConstructorVisInner ::= 'public' | 'shared' | 'private'
                    ;  (§10.5, 003-1)
```

// **Inner-≤-outer rule (per §10.5, post-parse).** The inner specifier
//  may never be more permissive than the outer: `private(public)` and
//  `shared(public)` are rejected post-parse. The grammar admits any
//  outer/inner combination; the cap is a semantic check.
// `ConstructorVis` attaches *only* to record and newtype `type`
//  declarations (§7.8, §7.9) — the constructor-bearing nominal forms.
//  Enums, traits, effects, operators, alias types, and free fns do not
//  admit a `ConstructorVis` suffix; the parser rejects the `(…)` form
//  on those declarations as a parse error (the outer keyword would
//  bind a parenthesised-expression-shaped following construct that
//  has no production at the start of a declaration head).
// Omission of `ConstructorVis` defaults the constructor's visibility
//  to match the type's visibility (§10.5 lead paragraph).

### 7.3 Field visibility

```
FieldVisibility   ::= 'public' | 'shared' | 'private'
                    ;  (§10.7, 003-1)
```

// `FieldVisibility` is the prefix on a record field declaration
//  (§7.8). It is *independent* of the enclosing record's type
//  visibility and of the constructor's visibility (SPEC §6.1.6). The
//  grammar admits the same three spellings as `Visibility`, with one
//  semantic restriction: a field's visibility may not exceed the
//  enclosing type's visibility (post-parse check per §10.7).
// `FieldVisibility` does not admit a `ConstructorVis`-style suffix —
//  fields have no constructor. The form is the bare keyword.
// Field visibility on a newtype's wrapped value is not a concept;
//  newtypes have no `FieldDecl`s. The constructor visibility per §7.2
//  controls reachability of the newtype's construction.

### 7.4 `use` statements (path bases, selection lists incl. nesting, glob, aliases)

```
UseStmt           ::= 'use' UsePath
                    ;  (§10.4, 003-35)

UsePath           ::= UsePathBase ( '::' UsePathSegment )*
                    ;  (§10.4, 003-35)

UsePathBase       ::= 'root'                                    // current package
                    | 'std'                                     // standard library
                    | IDENT                                     // external dependency
                    ;  (§10.2.3, 003-1)

UsePathSegment    ::= IDENT UseAlias?                           // single name (leaf or intermediate)
                    | '*'                                       // glob (terminal only)
                    | '(' UseSelectionList ')'                  // selection list (terminal only)
                    ;  (§10.4.1, 003-48)

UseSelectionList  ::= UseSelectionItem ( ',' UseSelectionItem )* ','?
                    ;  (§10.4.1, 003-48)

UseSelectionItem  ::= IDENT ( '::' UsePathSegment )* UseAlias?  // nested sub-path (with own alias)
                    | '*'                                       // nested glob
                    ;  (§10.4.1, 003-48)

UseAlias          ::= 'as' IDENT
                    ;  (§10.4, 003-44)
```

// **All `use` paths are absolute (per §10.2.3).** Every `UsePath`
//  starts at a `UsePathBase` — `'root'`, `'std'`, or a bare-IDENT
//  external-dependency name. There is no relative-path / current-module
//  form for imports (003-35); the `'here'` and `'module'` namespace
//  anchors of §13.3 are not admitted in `UsePath`. The parser rejects
//  a `UsePath` whose first segment is anything other than the three
//  alternatives of `UsePathBase`.
// **Selection lists use parentheses (per §10.4.1, 003-45).** The
//  `(item, item, ...)` form is the selection-list terminator. A
//  `UseSelectionItem` may itself be a multi-segment path
//  `IDENT '::' UsePathSegment*`, may carry its own `UseAlias`, may be
//  a nested selection list `'(' UseSelectionList ')'` (since the inner
//  `UsePathSegment` admits the form recursively), and may be a `'*'`
//  glob (003-48). Selection lists nest to arbitrary depth:
//  `use root::a::(b, c::(d, e))` parses (003-48).
// **Glob `*` is terminal (per §10.4.1, 003-46).** A `'*'` appears only
//  as the *last* segment of a path or selection item — it imports every
//  visible name from that scope. The parser rejects a `'*'` followed
//  by `::` or any further segment.
// **`UseAlias` attaches to a single-name leaf (per §10.4).** The
//  `'as' IDENT` suffix is admissible on the final IDENT segment of a
//  path or selection item, renaming the imported name (003-44). It is
//  not admissible after `'*'` (a glob renames no single thing).
// **No `Visibility` prefix on `UseStmt` (per §10.4, 003-39).** `use`
//  has no visibility modifier. The grammar `UseStmt` production has
//  no leading `Visibility?`; the parser rejects any visibility keyword
//  preceding `'use'` as a syntax error.
// **File-scope-only (per §10.4.3).** A `UseStmt` is admissible only at
//  module top level — never inside a function body, node body, or
//  other inner scope. The grammar admits `UseStmt` as a
//  `TopLevelDecl` alternative; the surrounding-context check rejects a
//  `'use'` inside a `Stmt` or `NodeBodyClause` post-parse.

### 7.5 `alias type` declarations

```
AliasTypeDecl     ::= Visibility? 'alias' 'type' IDENT GenericParamList? '=' TypeExpr
                    ;  (§4.2, 003-34)
```

// `'alias type'` is a two-keyword sequence (not a compound). The
//  parser requires both keywords in order; either alone produces a
//  parse error (`'type'` alone heads a `RecordDecl` / `NewtypeDecl`,
//  §7.8 / §7.9).
// **Transparent alias (per §4.2).** `alias type X = T` makes `X` an
//  alternative spelling for `T`; no new nominal identity is created.
//  This contrasts with `type X: wraps T` (§7.9), which creates a
//  new identity. The semantic distinction is post-parse; the parser
//  distinguishes the two productions by the `'alias'` prefix.
// **No body (per §4.2).** An `AliasTypeDecl` has no body — it is a
//  single-line declaration; the `'=' TypeExpr` is the entirety of
//  the right-hand side. There is no `':'` after the RHS.
// `GenericParamList?` admits the standard generic-parameter shape
//  of §3.12, so generic aliases (`alias type byte = u8`,
//  `alias type Pair[T] = (T, T)`) are well-formed.
// `Visibility?` is admitted per §10.4.2 (003-34); alias types follow
//  the standard visibility rules for named declarations.

### 7.6 Free function declarations (generics, defaults, `own`, `where`, `-> T from v`)

```
FnDecl            ::= Visibility? 'fn' IDENT GenericParamList? '(' FnParamList? ')'
                      FnReturnWithFrom? WhereClause? FnBody
                    ;  (§11.7, 003-34)

FnReturnWithFrom  ::= FnReturn FnFromClause?                  // from-clause requires return clause
                    ;  (§11.7.5, 013-124)

FnParamList       ::= FnParam ( ',' FnParam )* ','?
                    ;  (§3.5, 006-7)

FnParam           ::= 'own'? IDENT ':' ( KindAnnotation | TypeExpr ) ( '=' Expr )?  // kind slot per §3.15
                    ;  (§11.7, 013-126)

FnReturn          ::= '->' 'own'? ( KindAnnotation | TypeExpr )                      // kind slot per §3.15
                    ;  (§11.3.6, 013-126)

FnFromClause      ::= 'from' FnFromRoots
                    ;  (§11.7.5, 013-124)

FnFromRoots       ::= IDENT
                    | '(' IDENT ( ',' IDENT )* ','? ')'
                    ;  (§11.7.5, 013-124)

FnBody            ::= ':' INDENT BlockBody DEDENT
                    | ':' InlineExpr
                    | NEWLINE                                   // abstract: trait method signature only
                    ;  (§1.4, 002-26)
```

// **Inline-vs-block body (per §2.2, 002-26).** A `FnBody` is a
//  may-be-inline body — both `fn f(x): x + 1` and the indented block
//  form are admissible. The third alternative — `NEWLINE` with no
//  preceding `':'` — is the *abstract* form admissible only in trait
//  method signatures (§7.11); a free-`fn` `FnDecl` whose body is
//  absent is a parse error in `TopLevelDecl` position.
// **`own` opt-in per parameter (per §11.7.4, 013-126).** The optional
//  `'own'` modifier before the parameter name selects the consuming
//  convention; without it the parameter is default
//  (borrow-equivalent). The `'own'` is part of the function's
//  contract (013-126).
// **`own` opt-in on return (per §11.3.6, 013-126).** The optional
//  `'own'` before the return `TypeExpr` selects the anchored
//  (independent-owner) form; without it the return is the default
//  borrow-rooted form. `'own'` here is part of the type identity, not
//  a separate annotation.
// **Default-parameter values (per §3.5.4).** A `FnParam` may carry an
//  `'=' Expr` default. The defaulted-before-non-defaulted ordering
//  rule of §3.5.4 is a *post-parse* check — the parser admits any
//  order; the resolver rejects an out-of-order placement.
// **Parameters are immutable (per §11.7.2).** A `FnParam` does not
//  admit `'mut'` — `mut buf: T` in parameter position is a parse
//  error; the grammar simply does not include `'mut'` as a parameter
//  modifier. To mutate a parameter's value the body uses a `mut`-local
//  rebind (§11.7.3), not a `mut`-parameter declaration.
// **Named-vs-positional discipline (per §3.5, 006-7).** The
//  no-mixing rule between named and positional arguments at call
//  sites is post-parse; the `FnParam` declaration itself is
//  uniform (`IDENT ':' TypeExpr` with no positional vs named
//  marker at the *declaration*).
// **`FnFromClause` (per §11.7.5, 013-124).** The optional
//  `'from' FnFromRoots` annotation makes the return's
//  borrow-rootedness explicit by naming the contributing input
//  binding(s). A single bare IDENT (`from v`) and a parenthesised
//  comma-separated union (`from (a, b)`) are the two surface
//  forms. The clause is purely optional (013-124); a default `-> T`
//  with no `from` leaves rootedness body-inferred. The `'from'`
//  keyword here is the same lexeme as the clause keyword used in
//  `connection` declarations (§2.4) — disambiguated by position
//  (immediately following `FnReturn`).
// **`WhereClause?` follows `FnFromClause?` (per §11.7.5, 013-124).**
//  When both are present the order is fixed: `-> T from v where T: Clone`.
//  The parser admits each independently; the combined order is
//  the only legal sequence.
// **No `fn` declarations inside record / enum / newtype bodies (per
//  §6.1.9, 009-46).** A `FnDecl` is admissible at module top level,
//  inside a `FulfillBody` (§7.12), or inside a `TraitBody`
//  (§7.11). It is not admissible inside a `RecordDecl`, `EnumDecl`,
//  or `NewtypeDecl` body — those bodies hold field / variant
//  declarations only. The exclusion is a parse-position rule, not a
//  feature of `FnDecl` itself.

### 7.7 Closure literals (anonymous fn expressions; type-position closure types live in §3.3)

```
ClosureLit        ::= 'fn' '(' ClosureParamList? ')' FnReturn? FnBody
                    ;  (§11.10, 022-118)

ClosureParamList  ::= ClosureParam ( ',' ClosureParam )* ','?
                    ;  (§11.10, 022-118)

ClosureParam      ::= 'own'? IDENT ( ':' TypeExpr )?
                    ;  (§11.10, 022-118)
```

// **Anonymous (per §11.10, 022-118).** A closure literal omits the
//  function name — the `'fn'` keyword is immediately followed by the
//  parameter list `'(' … ')'`. The lookahead `'(' ` after `'fn'` is
//  what distinguishes a `ClosureLit` from a named `FnDecl` (where the
//  IDENT precedes the `'('`).
// **Closure literals are expressions (per §5).** `ClosureLit` is an
//  `Expr` form, not a top-level declaration; it is listed here in §7
//  to keep all `fn`-headed constructs together. The closure's *type*
//  is the structural `FnType` of §3.3 (`fn(P) -> R`); the closure
//  *literal* is a value expression of that type.
// **Parameter / return types inferable (per §11.10).** The
//  `':' TypeExpr` on a `ClosureParam` and the entire `FnReturn` are
//  optional in closure literals when inferable from the expected
//  closure type at the use site (§11.10 lead paragraph). The grammar
//  admits the omission — both `':' TypeExpr` and the whole `FnReturn`
//  may be absent — and inference fills them post-parse.
// **`own` on parameters (per §11.10).** The `'own'` opt-in convention
//  applies identically to `ClosureParam` as to `FnParam`; closures
//  inhabit `fn(own T) -> R` types when their parameters consume.
// **No name → no visibility (per §11.10).** A `ClosureLit` carries no
//  visibility prefix and no `WhereClause` / `FnFromClause` — closures
//  are not named declarations. The grammar reflects this: no leading
//  `Visibility?`, no trailing `WhereClause?` / `FnFromClause?`.
// **No `mut` on closure parameters.** Parallel to §7.6; the
//  `ClosureParam` production does not include `'mut'`.
// **Captures-must-be-`Copy` rule (per §11.10.1).** A closure body
//  may reference let-bound names from the enclosing scope; the
//  capture set is the minimal subvalue set the body reads
//  (§11.10.2). The `Copy`-only restriction on captured values is a
//  semantic check, not a grammar rule.

### 7.8 Record (`type Name: <fields>`) declarations

```
RecordDecl        ::= Visibility? 'type' IDENT GenericParamList? WhereClause? RecordBody
                    ;  (§6.1, 009-1)

RecordBody        ::= ':' INDENT RecordBodyItem+ DEDENT
                    | '=' IntersectionType                      // record-intersection RHS form per §5.3, 005-50
                    | NEWLINE                                   // zero-field marker
                    ;  (§6.1.1, 009-1)

RecordBodyItem    ::= SatisfiesClause
                    | FieldDecl
                    ;  (§6.1.1, 009-1)

SatisfiesClause   ::= 'satisfies' TypePath ( ',' TypePath )* ','?
                    ;  (§3.2, 017-12)

FieldDecl         ::= FieldVisibility? IDENT ':' TypeExpr
                    ;  (§6.1.1, 009-1)
```

// **Zero-field marker form (per §6.1.1).** `type Marker` with no
//  body is admissible — it produces a zero-field nominal type. The
//  `NEWLINE` alternative of `RecordBody` covers this case.
// **`SatisfiesClause` placement (per §6.1.1).** A `RecordBody` may
//  hold at most one `SatisfiesClause`, conventionally at the top.
//  The grammar admits the clause in any `RecordBodyItem` position;
//  the at-most-one constraint is a post-parse check.
// **No field defaults (per §6.1.2).** `FieldDecl` does not admit an
//  `'=' Expr` default — every field must be supplied at every
//  construction site. The production carries no default position.
//  This contrasts with `FnParam` (§7.6) and enum-variant payload
//  fields (§7.10), which do take defaults.
// **No `fn` declarations (per §6.1.9, 009-46).** A `RecordBodyItem`
//  is `SatisfiesClause` or `FieldDecl` only — there is no `FnDecl`
//  alternative. Behaviour on records is delivered through free
//  functions or `fulfill`-block methods, never inline.
// **Discrimination from `NewtypeDecl` (per §6.3.1).** The discriminating
//  surface token is the presence of a `'wraps'` clause inside the
//  body. A `RecordDecl` and a `NewtypeDecl` share the same header
//  (`Visibility? 'type' IDENT GenericParamList? WhereClause?` and
//  the body's leading `':'` token); the parser commits to one or
//  the other based on whether the first body item is `'wraps'` or
//  a `FieldDecl` / `SatisfiesClause`. The `NewtypeDecl` production
//  is in §7.9.
// **`WhereClause?`** is admitted before the `RecordBody`'s `':'`
//  for generic records carrying constraints (§3.13).
// **`@derive(...)` directive (per §3.8).** The `@derive` annotation
//  is a directive (§12.1) attached to the line above the
//  `RecordDecl` — it is not a part of the `RecordBody`. The
//  directive surface is uniform across record / enum / newtype
//  forms.

### 7.9 Newtype declarations (`wraps`)

```
NewtypeDecl       ::= Visibility? 'type' IDENT GenericParamList? WhereClause? NewtypeBody
                    ;  (§6.3, 009-1)

NewtypeBody       ::= ':' INDENT NewtypeBodyItem+ DEDENT
                    ;  (§6.3.1, 009-1)

NewtypeBodyItem   ::= WrapsClause
                    | SatisfiesClause
                    ;  (§6.3.1, 009-1)

WrapsClause       ::= 'wraps' TypeExpr
                    ;  (§6.3.1, 009-1)
```

// **Discriminating token (per §6.3.1).** A `NewtypeBody` contains
//  *exactly one* `WrapsClause` and zero or more `SatisfiesClause`s.
//  The presence of a `'wraps'` keyword in the body distinguishes
//  `NewtypeDecl` from `RecordDecl` (§7.8). The parser inspects the
//  body's first non-clause-keyword item; if it is `'wraps'`, the
//  declaration is a `NewtypeDecl`. A body that mixes `'wraps'` with
//  `FieldDecl`s is a post-parse semantic error (§6.3.1).
// **No `FieldDecl` (per §6.3.1).** A `NewtypeBodyItem` does not admit
//  `FieldDecl` — newtypes wrap one underlying value via `WrapsClause`,
//  not a set of fields.
// **Constructor is positional with one argument (per §6.3.2).** The
//  newtype's constructor surface is `TypeName '(' value ')'` —
//  parsed as an ordinary `CallExpr` (§5.5). There is no separate
//  constructor production; the parser cannot syntactically
//  distinguish a newtype construction from a function call or a
//  cast (§5.6). The discrimination is post-parse via name
//  resolution.
// **`ConstructorVis` admissible (per §10.5, §6.3.4).** The
//  `Visibility?` head of a `NewtypeDecl` admits the `ConstructorVis`
//  suffix per §7.2 — `public(private) type Email: wraps string` is
//  the smart-constructor pattern.

### 7.10 Enum declarations

```
EnumDecl          ::= Visibility? 'enum' IDENT GenericParamList? WhereClause? EnumBody
                    ;  (§6.2, 009-1)

EnumBody          ::= ':' INDENT EnumBodyItem+ DEDENT
                    | NEWLINE                                   // uninhabited / zero-variant
                    ;  (§6.2.1, 009-1)

EnumBodyItem      ::= SatisfiesClause
                    | VariantDecl
                    ;  (§6.2.1, 009-1)

VariantDecl       ::= IDENT VariantDeclPayload?
                    ;  (§6.2.1, 009-1)

VariantDeclPayload ::= '(' VariantPayloadList? ')'
                    ;  (§6.2.1, 009-1)

VariantPayloadList ::= PositionalPayloadList
                    | NamedPayloadList
                    ;  (§6.2.1, 009-1)

PositionalPayloadList ::= PositionalPayloadField ( ',' PositionalPayloadField )* ','?
                    ;  (§6.2.1, 009-1)

PositionalPayloadField ::= TypeExpr ( '=' Expr )?
                    ;  (§6.2.1, 009-1)

NamedPayloadList  ::= NamedPayloadField ( ',' NamedPayloadField )* ','?
                    ;  (§6.2.1, 009-1)

NamedPayloadField ::= IDENT ':' TypeExpr ( '=' Expr )?
                    ;  (§6.2.1, 009-1)
```

// **Zero-variant uninhabited form (per §6.2.1).** `enum Bottom` with
//  no body is admissible; the `NEWLINE` alternative of `EnumBody`
//  covers this case. A zero-variant enum has no values and serves as
//  a type-level bottom marker.
// **Variant payload form selection (per §6.2.1).** Within a single
//  `VariantDeclPayload`, the parser selects `PositionalPayloadList`
//  or `NamedPayloadList` based on whether the first payload component
//  has the shape `IDENT ':' TypeExpr` (named) or `TypeExpr` alone
//  (positional). Mixing within one variant declaration is a parse
//  error: a later component of the other form in the same payload
//  list does not match the selected alternative's grammar.
//  Different variants of the same enum may use different forms
//  independently.
// **Variant payload defaults (per §6.2.1).** Both
//  `PositionalPayloadField` and `NamedPayloadField` admit `'=' Expr`
//  defaults, applied when the field is omitted at construction —
//  exactly as `FnParam` defaults.
// **Variant unit form (per §6.2.1).** A `VariantDecl` with no
//  `VariantDeclPayload` (`North`, `None`) is a unit variant — the
//  optional `VariantDeclPayload` is absent.
// **`VariantDeclPayload` vs `VariantPayload` (per §4.2).** The
//  declaration-side payload nonterminal is `VariantDeclPayload`
//  (this section, includes the surrounding parens and lists
//  `TypeExpr` field declarations). The pattern-side `VariantPayload`
//  (§4.2) is distinct — it lists sub-`Pattern`s without surrounding
//  parens. Renamed in this section to avoid a nonterminal collision.
// **No per-variant visibility (per §6.2.6, §10.6).** Variants share
//  the enum's visibility. A `VariantDecl` does not admit a leading
//  `Visibility` prefix; the production carries none.
// **No `FieldDecl` outside variants (per §6.2).** An `EnumBody`'s
//  top-level items are `SatisfiesClause` and `VariantDecl` only —
//  fields belong to variant payloads.

### 7.11 Trait declarations (TraitDecl BNF — already in SPEC §3.1, mirror exactly)

The trait-declaration grammar is fixed in SPEC §3.1 and is mirrored
here. Productions are reproduced with the same shape as SPEC §3.1's
BNF; lexical sub-forms cross-reference productions elsewhere in this
document. Two presentational refinements apply: (1) `RequiredCell`'s
kind alternatives are factored into a separate `RequiredCellKind`
production (SPEC §3.1 inlines them) — the split is presentational only,
not semantic; (2) `MethodSig`'s return and body re-use the `FnReturn`
and `FnBody` productions from §7.6, which embed the `'own'?` return
qualifier (§11.3.6, supported by trait methods per §11.7.4) and the
abstract-body `NEWLINE` alternative (semantically equivalent to SPEC
§3.1's outer `(':' FnBody)?` optionality). No `FnFromClause` or
`WhereClause` appears on the trait `MethodSig` itself — both are
omitted, matching SPEC §3.1's BNF text.

```
TraitDecl         ::= Visibility? 'sealed'? 'trait' IDENT GenericParamList? TraitBody
                    ;  (§3.1, 005-30)
// Directive decoration attaches via the §12.3 AnnotatedDecl wrapper
//  (Phase D, D4) — TraitDecl itself carries no inline Annotation* head.
// The optional `sealed` modifier (§3.7.6, 005-239) restricts
//  fulfillment claims — both `fulfill Trait for Type` blocks and bare
//  `satisfies Trait` marker claims — to the trait's declaring module;
//  a claim outside that module is diagnostic
//  `sealed_trait_fulfillment_outside_module`.

TraitBody         ::= NEWLINE INDENT TraitBodyItem+ DEDENT
                    | NEWLINE
                    ;  (§3.1, 005-30)

TraitBodyItem     ::= Annotation*
                      ( RequiresClause
                      | AssocTypeDecl
                      | RequiredCell
                      | EndpointDecl
                      | MethodSig )
                    ;  (§3.1, 005-30)

// Reserved-but-undefined: a doc-comment form is not yet specified
//  (§2.5 line-comment-only rule). When SPEC adopts one, attach the
//  optional slot here.

RequiresClause    ::= 'requires' TypePath ( ',' TypePath )*
                    ;  (§3.1.4, 005-30)

AssocTypeDecl     ::= 'type' IDENT ( 'is' TypeExpr )?
                    ;  (§3.1.2, 005-19)

RequiredCell      ::= RequiredCellKind IDENT ':' TypeExpr ( '=' Expr )?
                    ;  (§3.1.7, 005-30)

RequiredCellKind  ::= 'attr'
                    | 'const'
                    | 'derived'
                    | 'recurrent' ( '[' ConstGenericArg ']' )?
                    | 'stream' ( 'ring' | 'gate' ) '[' ConstGenericArg ']'
                    | 'stream' '[' TypeExpr ']'      // bracket-policy head (mirrors KindAnnotation §3.15); same wiring type as the word form (§13.18.3)
                    ;  (§3.1.7, 005-30)

EndpointDecl      ::= ( 'from' | 'to' ) ':' TypeExpr
                    ;  (§3.1.7, 005-30)

MethodSig         ::= 'fn' IDENT GenericParamList? '(' TraitFnParamList? ')'
                      FnReturn? FnBody
                    ;  (§3.1.1, 005-30)

TraitFnParamList  ::= TraitFnParam ( ',' TraitFnParam )* ','?
                    ;  (§3.1.1, 005-30)

TraitFnParam      ::= 'own'? IDENT ':' ( KindAnnotation | TypeExpr )  // kind slot per §3.15; no default-value `'=' Expr` per SPEC §3.1
                    ;  (§3.1.1, 005-30)
```

// **TraitBody two-alternative shape (per §3.1).** An empty trait
//  body (`trait Marker`) is the second alternative — a single
//  `NEWLINE` with no `INDENT` / body items. The marker-trait form
//  yields a methodless trait per §3.1 (003-1 final paragraph).
// **`Annotation*` covers `@default(T)` (per §3.1.5).** The trait's
//  default-concrete-type directive `@default(<TypePath>)` attaches at
//  the `Annotation*` position before the `'trait'` keyword. The
//  grammar treats it as one annotation in the `Annotation*` run
//  (see §12.1).
// **`MethodSig` mirrors SPEC §3.1 (per §3.1.1, 005-30).** The trait
//  `MethodSig` mirrors SPEC §3.1's BNF with the same `FnReturn` /
//  `FnBody` factoring used by `FnDecl` in §7.6: `FnReturn` embeds the
//  `'own'?` return qualifier (§11.3.6, admitted in trait methods per
//  §11.7.4 — see SPEC's `fn clone(value: Subject) -> own Subject`
//  example), and `FnBody`'s `NEWLINE` alternative is the abstract-method
//  form (semantically equivalent to SPEC §3.1's outer `(':' FnBody)?`
//  optionality — a body that is absent rather than a body whose head is
//  optional). No `FnFromClause` and no `WhereClause` appear on the
//  trait `MethodSig` — both are absent from SPEC §3.1's BNF text and
//  absent here. A present `FnBody` (block or inline) is the
//  default-body form per §3.1.3. If trait methods later need a `where`
//  clause or `from` clause, the enhancement must be upstreamed to SPEC
//  §3.1 first and then mirrored here — this grammar does not deviate.
// **`RequiredCellKind` mirrors node-body cell forms (per §3.1.7).**
//  The `recurrent[N]` and `stream <policy>[N]` bracketed forms are
//  the same lexical shape as the corresponding node-body
//  declarations of §8.9. A trait declaring any `RequiredCell` (or
//  `EndpointDecl`) is a kind-specific trait per §3.1.7 — the
//  kind-specificity is a semantic property, not a grammar one.
// **`EndpointDecl`'s `'from'` / `'to'` keywords (per §3.1.7).** The
//  same `'from'` / `'to'` clause keywords used in connection
//  declarations (§9) appear here as endpoint-requirement
//  introducers. Their position inside a `TraitBodyItem` is what the
//  parser uses to recognise the `EndpointDecl` alternative.
// **`AssocTypeDecl` re-uses the `'type'` keyword (per §3.1.2).** The
//  `'type'` keyword that heads a `RecordDecl` / `NewtypeDecl` at
//  module level appears inside a `TraitBody` as an associated-type
//  declarator. Position discriminates.

### 7.12 Fulfill blocks (FulfillItem BNF — already in SPEC §3.3, mirror exactly)

The fulfill-block grammar is fixed in SPEC §3.3 and is mirrored
here verbatim.

```
FulfillItem       ::= 'fulfill' TypeExpr 'for' TypeExpr WhereClause? FulfillBody
                    ;  (§3.3, 005-30)

FulfillBody       ::= NEWLINE INDENT FulfillBodyItem+ DEDENT
                    ;  (§3.3, 005-30)

FulfillBodyItem   ::= Annotation* ( FnDecl | AssocTypeBinding )
                    ;  (§3.3, 005-30)

// Reserved-but-undefined: a doc-comment form is not yet specified
//  (§2.5 line-comment-only rule). When SPEC adopts one, attach the
//  optional slot here.

AssocTypeBinding  ::= 'type' IDENT 'is' TypeExpr NEWLINE
                    ;  (§3.3.2, 005-93)
```

// **No visibility prefix on `FulfillItem` (per §10.3).** A `fulfill`
//  block carries no separate visibility specifier — reachability is
//  derived from the trait's and the type's joint visibility (§10.3
//  final bullet). The grammar has no `Visibility?` head on
//  `FulfillItem`.
// **`FulfillBodyItem`'s `FnDecl` (per §3.3).** The `FnDecl` here is
//  the same production as the free-function `FnDecl` of §7.6,
//  including all its options (generics, defaults, `'own'`,
//  `FnFromClause`, `WhereClause`, inline-or-block body). The
//  function lives in the (Trait, Type)-scoped namespace, not the
//  module's free-function namespace; this is a semantic property
//  (§3.3 lead paragraphs).
// **`WhereClause?` on `FulfillItem` (per §3.3.4).** The `WhereClause`
//  attached at the `FulfillItem`-header level expresses conditional
//  implementations (`fulfill Display for Result[T, E] where T: Display, E: Display`).
//  The clause syntax is `WhereClause` from §3.13; the conditions
//  are checked at every call site post-parse.
// **`AssocTypeBinding` shape (per §3.3.2; Phase D).** An
//  `AssocTypeBinding` is the implementation-side binder for a trait's
//  `AssocTypeDecl`. The binder uses the keyword `is` (parallel to the
//  where-clause `AssocTypeEqualityPredicate` of §3.13) — never `=`.
//  The `'is' TypeExpr` is mandatory here (unlike `AssocTypeDecl`,
//  where the initial-value clause is optional and represents a
//  trait-level default).
// **`MethodSig`-without-body vs `FnDecl`-with-body (per §3.3).** A
//  `FulfillBodyItem` requires a *concrete* body on each `FnDecl` —
//  the abstract `FnBody` `NEWLINE` form is not legal at a fulfill
//  site. The grammar admits the abstract form via `FnBody`, but
//  semantically a `FulfillBodyItem`'s `FnDecl` must supply a body
//  (abstract methods inherit the trait's default body per §3.1.3 by
//  omission — the method does not appear in the `FulfillBody` at all,
//  rather than appearing without a body).

### 7.13 Reactive `operator` declarations

```
OperatorDecl      ::= Visibility? 'operator' IDENT GenericParamList?
                      '(' OperatorParamList? ')' '->' ( KindAnnotation | TypeExpr ) WhereClause? OperatorBody  // kind slot per §3.15
                    ;  (§13.17.2, 029-93)

OperatorParamList ::= OperatorParam ( ',' OperatorParam )* ','?
                    ;  (§13.17.3, 029-93)

OperatorParam     ::= IDENT ':' ( KindAnnotation | TypeExpr ) ( '=' Expr )?  // kind slot per §3.15
                    ;  (§13.17.3, 029-93)

OperatorBody      ::= ':' INDENT OperatorBodyItem+ DEDENT
                    | ':' InlineExpr
                    ;  (§13.17.4, 029-93)

OperatorBodyItem  ::= ModuleReactiveDecl                        // recurrent / derived / stream
                    | LetStmt
                    | Expr                                      // including final-expression position
                    ;  (§13.17.4, 029-93)
```

// **No `attr` or `mut` in operator bodies (per §13.17.4).** Neither
//  `attr` (covered by the rejection note below) nor a `MutStmt` is
//  admitted: `OperatorBodyItem` does not list `MutStmt`. Mutation
//  state belongs to reactive cells (`recurrent`, `stream`) not
//  local mutable bindings.

// **Mandatory return arrow (per §13.17.2, 029-93).** Unlike `FnDecl`,
//  `OperatorDecl` requires `'->' TypeExpr` — an operator always
//  declares a return type. The grammar makes the `'->'` part of the
//  declaration head, not optional.
// **Inline-vs-block body (per §13.17.4).** An `OperatorBody` whose
//  only content is the final expression may be written inline after
//  the colon (`operator double(source: cell f32) -> derived f32: source * 2`);
//  a body holding any `recurrent`, `derived`, `stream`, or `let`
//  declarations uses the indented block form.
// **No `own` on operator parameters (per §13.17.3).** Operators take
//  cell-bound (`cell T`) or value (`T`) parameters; ownership-consume
//  conventions are not part of an operator's parameter shape. The
//  `OperatorParam` production does not admit `'own'`. The default-on
//  value parameters and disallowed-on-`cell T`-parameters semantic
//  restriction (§13.17.3) is a post-parse check.
// **No `attr` declarations in body (per §13.17.4).** The
//  `OperatorBodyItem` production admits `ModuleReactiveDecl` — which
//  covers `signal`, `derived`, `recurrent`, and `stream` (§7.15) — but
//  the `'attr'` form is rejected post-parse inside an `OperatorBody`.
//  Per-instance configuration is expressed via parameters, not
//  internal `attr`s.
// **Final-expression-is-output (per §13.17.4).** The trailing
//  `ExprStmt` of an `OperatorBody`'s block — or the inline-form
//  `InlineExpr` — is the operator's output expression. The grammar
//  treats it as a `ExprStmt` like any other; the
//  output-from-final-expression rule is semantic.
// **Operator structural type production lives in §3.4** as
//  `OperatorType`. The `OperatorDecl` here is the declaration form;
//  the type form is the parameterised structural type appearing in
//  signatures.

### 7.14 Effect declarations (with `desired:` / `observed:` block sub-grammar)

```
EffectDecl        ::= Visibility? 'effect' IDENT GenericParamList?
                      '(' EffectParamList? ')' WhereClause? EffectBody
                    ;  (§13.19.2, 029-93)

EffectParamList   ::= EffectParam ( ',' EffectParam )* ','?
                    ;  (§13.19.3, 029-93)

EffectParam       ::= IDENT ':' ( KindAnnotation | TypeExpr ) ( '=' Expr )?  // kind slot per §3.15
                    ;  (§13.19.3, 029-93)

EffectBody        ::= ':' INDENT EffectBlock+ DEDENT
                    ;  (§13.19.2, 029-93)

EffectBlock       ::= DesiredBlock
                    | ObservedBlock
                    ;  (§13.19.2, 029-93)

DesiredBlock      ::= 'desired' ':' INDENT AnnotatedDesiredCellDecl+ DEDENT
                    ;  (§13.19.4, 029-93)

ObservedBlock     ::= 'observed' ':' INDENT AnnotatedObservedCellDecl+ DEDENT
                    ;  (§13.19.5, 029-93)

AnnotatedDesiredCellDecl ::= Annotation* DesiredCellDecl       // per Phase D (D6): cells admit directive decoration
                    ;  (§13.19.4, 030-N)

AnnotatedObservedCellDecl ::= Annotation* ObservedCellDecl
                    ;  (§13.19.5, 030-N)

DesiredCellDecl   ::= 'derived' IDENT ( ':' TypeExpr )? '=' Expr
                    | 'recurrent' RecurrentDepth? RecurrentBind ( ':' TypeExpr )? '=' Expr
                    | 'stream' ( 'ring' | 'gate' ) ( '[' ConstGenericArg ']' )? IDENT ( ':' TypeExpr )? '=' Expr
                    | 'stream' '[' TypeExpr ']' IDENT ( ':' TypeExpr )? '=' Expr
                    | 'recurrent' RecurrentDepth? 'stream' ( 'ring' | 'gate' )
                          ( '[' ConstGenericArg ']' )? IDENT ( ':' TypeExpr )? '=' Expr
                    | 'recurrent' RecurrentDepth? 'stream' '[' TypeExpr ']'
                          IDENT ( ':' TypeExpr )? '=' Expr
                          // bracket-policy declaration heads (mirror KindAnnotation §3.15); each spells the same wiring type as its word-form sibling — the word form is the idiomatic sugar (§13.18.3)
                    | WhenBlockDecl                              // cell-bearing variant of §5.21 (per §13.9.12)
                    | GivenBlockDecl                             // cell-bearing variant of §5.22 (per §13.9.13)
                    ;  (§13.19.4, 029-93)

ObservedCellDecl  ::= 'signal' IDENT ':' TypeExpr '=' Expr
                    | 'stream' ( 'ring' | 'gate' ) ( '[' ConstGenericArg ']' )? IDENT ':' TypeExpr
                    | 'stream' '[' TypeExpr ']' IDENT ':' TypeExpr
                      // bracket-policy declaration head (mirrors KindAnnotation §3.15); same wiring type as the word form — the word form is the idiomatic sugar (§13.18.3)
                    ;  (§13.19.5, 029-93)

WhenBlockDecl     ::= 'when' Expr ':' INDENT DesiredCellDecl+ DEDENT OtherwiseDeclArm?
                    | 'when' ':' INDENT WhenDeclArm+ OtherwiseDeclArm? DEDENT
                    ;  (§13.19.4, 022-88)

WhenDeclArm       ::= Expr ':' INDENT DesiredCellDecl+ DEDENT
                    ;  (§13.9.12, 022-88)

OtherwiseDeclArm  ::= 'otherwise' ':' INDENT DesiredCellDecl+ DEDENT
                    ;  (§13.9.12, 022-88)

GivenBlockDecl    ::= 'given' Expr ':' INDENT GivenDeclArm+ DefaultDeclArm? DEDENT
                    ;  (§13.19.4, 022-106)

GivenDeclArm      ::= Pattern ':' INDENT DesiredCellDecl+ DEDENT
                    ;  (§13.9.13, 022-106)

DefaultDeclArm    ::= 'default' ':' INDENT DesiredCellDecl+ DEDENT
                    ;  (§13.9.13, 022-106)
```

// **`WhenBlockDecl` / `GivenBlockDecl` (cell-bearing variants).** The
//  `WhenBlockDecl` and `GivenBlockDecl` productions parallel the
//  expression-form `WhenBlockExpr` (§5.21) and `GivenBlockExpr` (§5.22),
//  but each arm body holds `DesiredCellDecl+` rather than an expression.
//  Used inside `desired:` to select per-arm cells (per §13.19.4 / SPEC
//  §13.9.12, §13.9.13). The four flavors — `WhenBlockExpr` (value
//  expression), `WhenBlockDecl` (desired cells), `ExposeWhenBlock`
//  (expose entries, §8.10), `EffectsWhenBlock` (effect entries, §8.11)
//  — and their `given` analogues are kept separate because each arm-body
//  grammar differs.

// **Effect-body block-only shape (per §13.19.2).** The `EffectBody`
//  contains *only* `DesiredBlock` and `ObservedBlock` entries — no
//  bare cell declarations directly in the effect body, no other
//  clauses, no nested `EffectDecl`. At least one of the two blocks
//  must be present; both may appear in either order (canonical is
//  `desired:` first). The `+` on `EffectBlock` admits one-or-more;
//  the at-least-one-of-each-kind constraint is a post-parse check.
// **`recurrent` allowed in `desired:` only (per §13.19.4 /
//  §13.19.5).** `DesiredCellDecl` admits `recurrent` and
//  `recurrent[N] stream`; `ObservedCellDecl` admits *neither* — a
//  host-fed observed cell has no expression body for `.past` to
//  read. The grammar enforces this by branching: the `ObservedCellDecl`
//  production has no `recurrent` alternative.
// **`stream` `= source` forbidden in `observed:` (per §13.19.5).**
//  `ObservedCellDecl`'s `stream` alternative has *no* `'=' Expr`
//  source — the host's reconciler pushes events via the runtime API
//  (§13.14.8). The grammar omits the `'=' Expr` tail on this branch
//  alone; an `observed: stream ring[1024] x: T = ...` is therefore a
//  parse error. `desired:` streams, conversely, *require* a
//  `'=' Expr` source.
// **`WhenBlockDecl` / `GivenBlockDecl` admissible in `desired:` only
//  (per §13.19.4 final paragraph, §13.19.5 third paragraph).** A
//  `desired:` block may also contain `when` / `given` selection
//  blocks whose arms hold further `DesiredCellDecl`s; the
//  block-decl forms reuse the productions of §5.21 / §5.22 at the
//  cell-declaration level. `observed:` blocks reject these — see
//  §13.19.5's `repeat` / `when` / `given` exclusion (post-parse
//  semantic check; the grammar does not enumerate them as
//  alternatives in `ObservedCellDecl`).
// **`'desired'` and `'observed'` are reserved as cell names (per
//  §13.19.6).** The spellings cannot appear as `IDENT` in
//  `DesiredCellDecl` / `ObservedCellDecl` cell-name positions; this
//  is a post-parse semantic check, not a grammar exclusion.
// **Cells admit directive decoration (per Phase D, D6).** The
//  `AnnotatedDesiredCellDecl` / `AnnotatedObservedCellDecl` wrappers
//  attach an `Annotation*` run ahead of each effect-block cell.
//  Examples: a `desired:` `recurrent` carrying `@reset_on_reopen`
//  (§13.2.4); a `desired:` `stream` carrying `@reset_on_reload`
//  (§13.19.11, §13.15.5). The wrapper is the sole attachment site
//  here — the §12.3 `AnnotatedDecl` wrapper does not reach inside an
//  effect block's `desired:` / `observed:` body because cell decls
//  are not module-level `Decl` alternatives.
// **No `own` on effect parameters (per §13.19.3).** Effect
//  parameters are cell-bound or value-typed; the `EffectParam`
//  production does not admit `'own'`. Defaults on `cell T`
//  parameters are not allowed in v1 (post-parse).
// **Type and constructor share name (per §13.19.8).** The `IDENT`
//  on `EffectDecl` serves both as the effect's type name (used in
//  signatures, bounds) and as the constructor (used in pipe chains
//  and function-call form). The grammar makes no syntactic
//  distinction; both roles parse against the same name.

### 7.15 Module-level reactive declarations (`signal`, `derived`, `recurrent`, `stream`)

```
ModuleReactiveDecl ::= Visibility? SignalDecl
                    | Visibility? DerivedDecl
                    | Visibility? RecurrentDecl
                    | Visibility? StreamDecl
                    ;  (§13.2, 003-31)

SignalDecl        ::= 'signal' IDENT ':' TypeExpr '=' Expr
                    ;  (§13.2.1, 003-31)

DerivedDecl       ::= 'derived' IDENT ( ':' TypeExpr )? '=' Expr
                    ;  (§13.2.3, 016-32)

RecurrentDecl     ::= 'recurrent' RecurrentDepth? RecurrentBind ( ':' TypeExpr )? '=' Expr
                    | 'recurrent' RecurrentDepth? 'stream' ( 'ring' | 'gate' )
                          ( '[' ConstGenericArg ']' )? IDENT ( ':' TypeExpr )? '=' Expr
                    | 'recurrent' RecurrentDepth? 'stream' '[' TypeExpr ']'
                          IDENT ( ':' TypeExpr )? '=' Expr
                          // bracket-policy declaration head (mirrors KindAnnotation §3.15); both spell the same wiring type — the word form is the idiomatic sugar (§13.18.3)
                    ;  (§13.2.4, 016-49)

RecurrentDepth    ::= '[' ConstGenericArg ']'
                    ;  (§13.2.4, 016-49)

RecurrentBind     ::= IDENT
                    | '(' IDENT ( ',' IDENT )+ ','? ')'        // tuple-coupled recurrent (§13.2.4.6)
                    ;  (§13.2.4.6, 016-94)

StreamDecl        ::= 'stream' ( 'ring' | 'gate' ) ( '[' ConstGenericArg ']' )?
                      IDENT ( ':' TypeExpr )? '=' Expr
                    | 'stream' '[' TypeExpr ']'
                      IDENT ( ':' TypeExpr )? '=' Expr
                      // bracket-policy declaration head (mirrors KindAnnotation §3.15); both spell the same wiring type — the word form is the idiomatic sugar (§13.18.3)
                    ;  (§13.18.2, 003-31)
```

// Directive decoration on any reactive cell declaration attaches via
//  the §12.3 `AnnotatedDecl` wrapper (Phase D, D4) at module scope, or
//  the local `AnnotatedDesiredCellDecl` / `AnnotatedObservedCellDecl`
//  wrappers (§7.14) at effect-block scope, or implicitly inside a node
//  body via §8.9's body decoration. The decl productions themselves
//  carry no inline `Annotation*` head.

// **Type annotation optional on `derived`, `recurrent`, `stream`
//  (per SPEC §13.2.3 / LOG 016-32; SPEC §13.2.4 / LOG 016-49;
//  SPEC §13.18.2).** The `':' TypeExpr` is optional in all three
//  forms; when omitted, the type is inferred from the RHS.
//  Canonical un-annotated example (SPEC §13.18.2):
//  `stream gate[256] db_writes = pending_writes`.
// **Tuple-coupled recurrent form (per SPEC §13.2.4.6 / LOG 016-94..102).**
//  `RecurrentBind` admits a parenthesized identifier tuple binding
//  multiple co-evolving recurrent cells from one RHS expression:
//  `recurrent (mean, variance): (f32, f32) = kalman_step(...)`.
//  When the annotation is present, its `TypeExpr` MUST be a
//  `TupleType` whose arity matches the bind's arity — a post-parse
//  semantic check; the grammar admits any `TypeExpr` here.

// **Visibility prefix (per §10.3, 003-31).** Module-level `signal`,
//  `derived`, and `recurrent` declarations accept visibility specifiers
//  on the declaration line (003-31). Module-level `stream`
//  declarations follow the same rule (the §10 enumeration covers
//  reactive declarations as a class).
// **`signal` / `derived` / `recurrent` `'=' Expr` is mandatory (per
//  §13.2.1, §13.2.3, §13.2.4).** Every reactive declaration carries
//  an initial-value / body expression; there is no uninitialised
//  reactive cell. The `'=' Expr` is required.
// **`recurrent[N]` history depth (per §13.2.4).** The optional
//  `'[' ConstGenericArg ']'` after `'recurrent'` declares the
//  output-history capacity; absent it defaults per §13.2.4. The
//  bracketed form is a const-generic argument (§3.2 production
//  `ConstGenericArg`), not a `GenericArgList` — a single
//  compile-time-known integer.
// **`stream` policy is mandatory (per §13.18.2).** A stream declaration
//  must name its policy — there is no policy-erased (`stream T`)
//  declaration form. The policy may be spelled either as the `'ring'` /
//  `'gate'` word form or as the `'[' TypeExpr ']'` bracket-policy form
//  (`stream[Ring[64]]`, §13.18.3); the two spell the same wiring type.
//  On the word form, the optional `'[' ConstGenericArg ']'` is the
//  buffer capacity, defaulting to 1024 (§13.18.2) when absent.
// **`'=' Expr` source is mandatory at module level (per §13.18.2).**
//  SPEC §13.18.2's declaration BNF (`stream policy[capacity]? name:
//  Type? = source`) has no optionality marker on `= source`, and every
//  module-level example in SPEC §13.18.2 carries a `= source`. A
//  `StreamDecl` at module level therefore requires `'=' Expr`. The
//  only sourceless `stream` admitted in SPEC is the `observed:` form
//  (§13.19.5) — host-fed via `runtime.push_stream` (§13.14.8) — which
//  has its own dedicated `ObservedCellDecl` production in §7.14 and
//  does not flow through `StreamDecl`.
// **`recurrent[N] stream` compound form (per §13.18.8).** A recurrent
//  stream is declared by stacking `'recurrent' ( '[' N ']' )?` before
//  the standard `'stream' …` form. Both bracketed forms are
//  independently optional and convey distinct semantics: the
//  outer `recurrent[N]` is the output history depth, the inner
//  `stream <policy>[M]` is the buffer policy and capacity.
// **No `attr` at module level (per §13.2.2).** `attr` declarations
//  appear only inside node and connection bodies and trait
//  `RequiredCell` declarations — never at module top level. The
//  `ModuleReactiveDecl` production has no `'attr'` alternative.
// **`ModuleReactiveDecl` reused in operator and effect bodies.**
//  Per §7.13 (`OperatorBodyItem`) and §7.14 (`DesiredCellDecl`
//  shapes), the `signal` / `derived` / `recurrent` / `stream`
//  productions are the same shape inside operator bodies and
//  effect `desired:` blocks as at module top level; the
//  surrounding-construct kind restricts which alternatives are
//  semantically legal.

### 7.16 Module-level placements (top-level placement, incl. `main` prefix)

The `TopLevelPlacement` production is defined canonically in §11.2
alongside the rest of the placement grammar. See §11.2 for the
production block and disambiguator notes; this section is retained as
a navigational stub.

See §11.2 for the canonical production; this section carries no
production block of its own.

// **No bare top-level connection placement (per §13.8 / §9).**
//  `TopLevelPlacement` instantiates node types only (the production
//  references `TypeRef` resolving to a node type); connection
//  placements live inside placement bodies as a `BlockBody`
//  item, not at module top level. The connection-vs-node distinction
//  is a post-parse semantic check; the parser cannot syntactically
//  tell the two apart from the placement surface alone.

## 8. Node declarations

Productions for `node` declarations: body skeleton, satisfies, children/incoming/outgoing acceptance clauses, standalone views, dynamic marker, cardinality forms, type-level when, body cells, expose, effects.

### 8.1 Node body skeleton (clauses only; no bare placements)

```
NodeDecl          ::= Visibility? 'node' IDENT GenericParamList?
                      WhereClause? NodeBody
                    ;  (§13.3.1, 017-8)
// Directive decoration attaches via the §12.3 `AnnotatedDecl` wrapper
//  (Phase D, D4) — `NodeDecl` carries no inline `Annotation*` head.

NodeBody          ::= ':' INDENT NodeBodyMember+ DEDENT
                    ;  (§13.3.1, 017-8)

NodeBodyMember    ::= SatisfiesClause                            // §8.2
                    | ChildrenClause                             // §8.3
                    | IncomingClause                             // §8.4
                    | OutgoingClause                             // §8.4
                    | StandaloneView                             // §8.5
                    | TypeLevelWhenClause                        // §8.8
                    | NodeBodyCellDecl                           // §8.9
                    | EffectsClause                              // §8.11
                    | ExposeClause                               // §8.10
                    ;  (§13.3.1, 017-9)
```

// **Clauses only — no bare placements (per §13.3.1, LEARNINGS #19).**
//  A `NodeBodyMember` is exclusively a clause or cell declaration;
//  bare node / connection placements are *not* admissible as direct
//  body members. Placements the type itself emits live inside the
//  `expose:` clause (as `for`/`repeat`/wrapper entries — §8.10) or
//  inside the `effects:` clause (as effect entries — §8.11); no
//  placement appears at the body's top level. The grammar enforces
//  this by enumerating only the clause-and-cell alternatives above.
// **Free order with positional constraints on `effects:` /
//  `expose:` (per §13.3.7 lead paragraph).** Members appear in *free
//  order* — the `+` repetition admits any sequence — with two
//  semantic ordering constraints checked post-parse: `effects:`
//  comes after the members; `expose:` comes last. The grammar admits
//  any order; the post-parse pass surfaces the position rule.
// **At-most-once clauses (per §13.3.7 lead paragraph).**
//  `SatisfiesClause`, `ChildrenClause`, `IncomingClause`,
//  `OutgoingClause`, `TypeLevelWhenClause`, `EffectsClause`, and
//  `ExposeClause` may each appear at most once per node body. The
//  grammar's `+` admits multiple; the at-most-once rule is a
//  post-parse semantic check.
// **`GenericParamList?` and `WhereClause?` (per §13.3.5).** A node
//  type may carry standard generic parameters (`[T, const N: usize]`)
//  and a `where` clause; both reuse the productions of §3.12 / §3.13.

### 8.2 `satisfies` clause

```
SatisfiesClause   ::= 'satisfies' TypePath ( ',' TypePath )*
                    ;  (§13.3.2, 005-63)
```

// **Trait-name list (per §13.3.2).** A `satisfies` clause lists one
//  or more trait `TypePath`s the node conforms to. Each path is a
//  trait `TypePath` per §3.1 — the surface accepts generic trait
//  instantiations (`satisfies Add[i64]`, `satisfies Display`); the
//  defaulted-type-parameter sugar (`satisfies Add` for
//  `satisfies Add[Subject]`) is a name-resolution rule per §3.1.6.1
//  and not a grammar one.
// **No `fn` declarations in a node body (per §13.3.2 / §13.3.6).**
//  Trait methods are implemented in separate `fulfill` blocks
//  (§7.12); the node body itself contains no `FnDecl`. The grammar
//  reflects this — `NodeBodyMember` has no `FnDecl` alternative.
// **At most one `SatisfiesClause` per body (per §13.3.1).** A second
//  `satisfies` clause is a post-parse semantic error; merge trait
//  lists into the single clause.

### 8.3 `children:` acceptance clause (named vs unnamed entries)

```
ChildrenClause    ::= 'children' ':' INDENT AcceptanceEntry+ DEDENT
                    ;  (§13.3.3, 017-21)

AcceptanceEntry   ::= NamedAcceptance
                    | UnnamedAcceptance
                    ;  (§13.3.3, 017-21)

NamedAcceptance   ::= 'dynamic'? IDENT ':' Selector Cardinality?
                    ;  (§13.3.3, 017-21)

UnnamedAcceptance ::= Selector Cardinality?
                    ;  (§13.3.3, 017-21)

Selector          ::= TypeExpr                                   // node-typed; trait or marker also legal
                    | BundleViewSelector                             // see §10.1
                    ;  (§13.3.3, 017-21)
```

// **Two entry forms — named vs unnamed (per §13.3.3, 017-21).**
//  A `NamedAcceptance` (`<name>: <selector> <card>`) joins the
//  body-scope namespace as a *selection view* over the children it
//  accepts (§13.3.3.2). An `UnnamedAcceptance`
//  (`<selector> <card>` — no leading `<name>:`) is *accept-only*:
//  it widens caller-facing acceptance and feeds the `@content`
//  directive (§12.2) but produces no body-side selection binding.
//  The two forms are syntactically distinguished by the presence or
//  absence of `IDENT ':'` ahead of the selector.
// **Layout under the clause header (per §13.3.3 final bullet).**
//  Each named entry occupies its own line under the clause header.
//  Multiple unnamed entries *may* share a line, space-separated; a
//  mixed line (named entries on the same line as anything else) is
//  not admitted. The grammar admits the one-entry-per-line shape via
//  `INDENT … DEDENT`; the multiple-unnamed-on-one-line allowance is
//  a layout-pre-processor permission tied to homogeneous-kind lines.
// **`dynamic` marker (per §13.3.3.1, §8.6).** The optional
//  `'dynamic'` prefix marks the entry as a dynamic-supply view
//  (membership varies at runtime). When present, the cardinality
//  *must* be the bare `'*'` sigil — see §8.6. Per SPEC §13.3.3.1
//  `dynamic` is admissible **only on `NamedAcceptance`** inside a
//  `children:` clause (a dynamic supply requires a body-side name
//  binding); `UnnamedAcceptance` does not admit the prefix.
//  Connection-view acceptance (§8.4) admits `dynamic` on both
//  named and unnamed entries per SPEC §13.3.4.
// **No `children:` clause means no children accepted (per
//  §13.3.3).** A node body lacking the clause accepts no children
//  whatsoever; a `Node` catch-all entry opens the clause fully.
//  This is a presence-rather-than-default rule — there is no
//  implicit catch-all.
// **Selector kinds (per §13.3.3).** `Selector` admits any
//  node-typed `TypeExpr` (concrete node type, trait whose
//  `requires` includes `Node`, or the `Node` marker), and the
//  `BundleViewSelector` form `[...]` of §10.1. The kind constraint
//  (node-typed only — connection-typed selectors are rejected) is
//  a post-parse semantic check; the grammar admits any `TypeExpr`.

### 8.4 `incoming:` / `outgoing:` connection-view acceptance clauses

```
IncomingClause    ::= 'incoming' ':' INDENT ConnAcceptanceEntry+ DEDENT
                    ;  (§13.3.4, 017-21)

OutgoingClause    ::= 'outgoing' ':' INDENT ConnAcceptanceEntry+ DEDENT
                    ;  (§13.3.4, 017-21)

ConnAcceptanceEntry ::= ConnNamedAcceptance
                      | ConnUnnamedAcceptance
                    ;  (§13.3.4, 017-21)

ConnNamedAcceptance ::= 'dynamic'? IDENT ':' TypeExpr Cardinality?
                    ;  (§13.3.4, 017-21)

ConnUnnamedAcceptance ::= 'dynamic'? TypeExpr Cardinality?
                    ;  (§13.3.4, 017-21)
```

// **Same two-form pattern as `ChildrenClause` (per §13.3.4, 017-21).**
//  Each acceptance entry is either named (joining the body-scope
//  namespace as a connection-view binding per §13.3.4.1) or unnamed
//  (accept-only — widens the caller-facing connection contract but
//  produces no body binding). The two forms parallel
//  `NamedAcceptance` / `UnnamedAcceptance` in §8.3.
// **Connection-typed selectors only (per §13.3.4).** The `TypeExpr`
//  in a `ConnAcceptanceEntry` must resolve to a connection type, a
//  trait whose `requires` includes `Connection`, or the
//  `Connection` marker. The kind check is post-parse.
// **`incoming:` vs `outgoing:` (per §13.3.4).** The two clauses
//  have identical surface shape; the keyword selects which endpoint
//  the node plays. An `outgoing:` entry bounds connections the
//  caller may originate *from* the node; an `incoming:` entry
//  bounds connections directed *at* the node (aggregated across
//  sources). The semantic difference is post-parse — the grammar's
//  productions are parallel.
// **`dynamic` marker (per §13.3.4 final paragraph, §8.6).** The
//  `'dynamic'` prefix applies to connection-view entries on the
//  same terms as `ChildrenClause` entries: it marks runtime-varying
//  membership and demands the bare `'*'` cardinality (see §8.6).
//  Either clause's entries may be marked `dynamic`.
// **`Selector` does *not* appear here — only `TypeExpr`.** A
//  connection-view's contract is a single connection type
//  (concrete or trait); `BundleViewSelector` is not admissible (bundles
//  bundle *nodes*, not connections — §10.4). The grammar restricts
//  this branch accordingly.

### 8.5 Standalone `view` declarations

```
StandaloneView    ::= 'dynamic'? 'view' IDENT ':' Selector Cardinality?
                    ;  (§13.3.3, 017-19)
```

// **Receiver-side projection — never widens acceptance (per
//  §13.3.3 "Selection views", 017-19).** A `StandaloneView` provides
//  an *additional* selection over children the node *already accepts*
//  (via its `ChildrenClause`); it does not widen the caller-facing
//  acceptance contract. A `view` whose selector is not narrowed by
//  any `ChildrenClause` entry resolves to zero matches at every site,
//  which is a post-parse semantic concern, not a parse one.
// **Distinct from a `NamedAcceptance` (per §13.3.3).** A
//  `NamedAcceptance` (entry inside `ChildrenClause` — §8.3) is *both*
//  an acceptance widening *and* a selection binding; a
//  `StandaloneView` is *only* a selection binding. The two are
//  syntactically distinguished by position: a `StandaloneView`
//  begins with `'view'` (or `'dynamic' 'view'`) and is a top-level
//  `NodeBodyMember`, never inside a clause's indented block.
// **`dynamic view` (per §13.3.3.4).** A standalone `view` may also
//  be `dynamic`; the same `'*'`-only rule of §8.6 applies. The
//  grammar admits the marker uniformly with §8.3 / §8.4.
// **Cardinality optional but conventional (per §13.3.3.1).** A
//  cardinality specifier is admitted; absence means the default
//  exactly-one (see §8.7). For projection views — the common
//  standalone-view use case — the specifier typically matches the
//  underlying acceptance entry's bound.

### 8.6 `dynamic` marker (requires `*`)

```
DynamicMarker     ::= 'dynamic'                                  // attaches to acceptance entry or view
                    ;  (§13.3.3.1, 017-40)
```

// **The `dynamic` keyword is a prefix on three forms (per §13.3.3.1):**
//  - on a `NamedAcceptance` / `UnnamedAcceptance` inside
//    `ChildrenClause` (§8.3);
//  - on a `ConnNamedAcceptance` / `ConnUnnamedAcceptance` inside
//    `IncomingClause` / `OutgoingClause` (§8.4);
//  - on a `StandaloneView` (§8.5).
//  It is not a standalone clause and does not appear elsewhere.
// **`dynamic` requires `'*'` — bounded specifiers forbidden (per
//  §13.3.3.1, 017-40).** When the `'dynamic'` marker is present, the
//  entry's `Cardinality` *must* be the bare `'*'` sigil. The
//  bounded specifiers (`?`, `+`, `[=N]`, `[N..=M]`, `[N..]`,
//  `[..=M]`) and the implicit exactly-one (no specifier) are
//  rejected post-parse with a diagnostic naming the marker. A
//  runtime-varying set guarantees no minimum and admits no checked
//  maximum, and runtime cardinality checks do not exist in the
//  language.
// **Single-element dynamic forbidden (per §13.3.3.1).**
//  `dynamic voice: Voice` (no sigil, defaults to exactly-one) and
//  `dynamic voices: Voice+` are both compile errors — the marker
//  excludes any bound other than `*`.

### 8.7 Cardinality forms (bare, `?`, `+`, `*`, `[=N]`, `[N..=M]`, `[N..]`, `[..=M]`)

```
Cardinality       ::= '?'                                        // 0..=1
                    | '+'                                        // 1..
                    | '*'                                        // 0..
                    | '[' CardinalityBracket ']'                 // bracketed range
                    ;  (§13.3.3.1, 017-21)

CardinalityBracket ::= '=' ConstGenericArg                       // [=N]      — exactly N
                    | ConstGenericArg '..=' ConstGenericArg      // [N..=M]   — N to M inclusive
                    | ConstGenericArg '..'                       // [N..]     — at least N, unbounded above
                    | '..=' ConstGenericArg                      // [..=M]    — 0 to M inclusive
                    ;  (§13.3.3.1, 017-21)
```

// **Bare = exactly one (per §13.3.3.1).** Absence of any
//  `Cardinality` specifier means *exactly one*; multiplicity is
//  always explicit. The grammar reflects this by making
//  `Cardinality?` optional at every site (§8.3, §8.4, §8.5); the
//  default at the absence is the implicit `=1` cardinality.
// **Sigil forms attach with no whitespace (per §13.3.3.1).**
//  `'?'`, `'+'`, and `'*'` lex as their own tokens but parse only
//  in direct adjacency to the selector — `Reverb?`, `Channel+`,
//  `Voice*`. Intervening whitespace is rejected by the layout
//  pre-processor of §2.1 (sigil position is post-selector,
//  pre-newline; the parser sees a single contiguous run).
// **Bracket forms may have an optional space before the bracket
//  (per §13.3.3.1).** `Filter[=1]` and `Filter [=1]` are both
//  admissible — the lexer admits the optional space before `'['`
//  in cardinality position. The bracket open-delimiter rule of
//  §2.1 / §10.5 still applies inside.
// **Exactly one cardinality specifier per view (per §13.3.3.1).**
//  Sigil *or* bracket, not both. The grammar admits a single
//  `Cardinality?` at each acceptance / view site; a duplicate
//  specifier is a post-parse semantic error.
// **`[=N]` is a distinct form (per §13.3.3.1).** The leading `'='`
//  inside `[…]` selects the "exactly N" alternative — distinct
//  from `[N..=N]` (which the grammar admits, but `[=N]` is the
//  canonical form). The `ConstGenericArg` of §3.2 supplies the
//  integer expression.

### 8.8 Type-level `when:` clause

```
TypeLevelWhenClause ::= 'when' ':' Expr
                    ;  (§13.9.2, 022-3)
```

// **Single predicate (per §13.9.2).** A node or connection body may
//  declare *one* `when:` predicate as a body member; it is the
//  type-level activation gate inherited by every placement. Multiple
//  `when:` clauses in one body are a post-parse semantic error.
// **Colon form distinguishes from inline `when` modifier (per
//  §13.9.2, §13.9.3).** The type-level form uses *colon* —
//  `when: <expr>` — consistent with other body fields (`from:`,
//  `to:`, `attr name:`). The placement-level modifier (§11.8) uses
//  *no colon* — `when <expr>` — consistent with modifier-style
//  clauses. The two surfaces never collide because they appear in
//  different positions (body-member vs placement-tail).
// **Predicate `Expr` (per §13.9.4).** The `Expr` must produce
//  `bool` (post-parse type check) and is a reactive expression: it
//  joins the enclosing instance's provenance and re-evaluates when
//  its references change. The grammar admits any `Expr`.
// **Traits cannot carry `when:` (per §13.9.2).** A `TraitDecl`
//  (§7.11) body does not admit `TypeLevelWhenClause` — gates are
//  per-type structural metadata, not behavior. The `TraitBodyItem`
//  production of §7.11 has no `'when'` alternative; a `when:`
//  clause inside a trait body is a parse error.

### 8.9 Node body cells (attr / default attr / derived / recurrent / stream / const)

```
NodeBodyCellDecl  ::= AnnotatedNodeBodyCellDecl                  // wrapper attaches Annotation* per Phase D, D4
                    ;  (§13.3.1, 017-21)

AnnotatedNodeBodyCellDecl ::= Annotation* ( AttrDecl
                                          | DefaultAttrDecl
                                          | DerivedDecl                  // see §7.15
                                          | RecurrentDecl                // see §7.15
                                          | StreamDecl                   // see §7.15
                                          | TopLevelConstDecl            // module-style const inside node body
                                          | YieldedDecl )                // see §8.9.1 (named yielded group)
                    ;  (§13.3.1, 017-21)

AttrDecl          ::= 'attr' IDENT ':' TypeExpr ( '=' Expr )?
                    ;  (§13.2.2, 017-21)

DefaultAttrDecl   ::= 'default' 'attr' IDENT ':' TypeExpr ( '=' Expr )?
                    ;  (§13.2.2.1, 017-21)
```

// **Annotation attaches via wrapper (Phase D, D4).** The decl
//  productions `AttrDecl` / `DefaultAttrDecl` / `DerivedDecl` /
//  `RecurrentDecl` / `StreamDecl` / `TopLevelConstDecl` carry no
//  inline `Annotation*` head; the `AnnotatedNodeBodyCellDecl`
//  wrapper attaches them uniformly inside a node body.

// **Reuses module-level reactive productions (per §7.15).**
//  `DerivedDecl`, `RecurrentDecl`, `StreamDecl`, and `ConstStmt`
//  are the same productions used at module top level and inside
//  operator bodies; the surrounding-construct kind (node body)
//  restricts which alternatives are semantically legal. All four
//  are admissible inside a node body.
// **`AttrDecl` introduced here (per §13.2.2).** Unlike the
//  module-level reactive set, `attr` is a node-and-connection-body
//  declaration; it has no module-level form (see §7.15 disambiguator
//  on "no `attr` at module level"). The default-value clause
//  `'=' Expr` is optional; when absent, the caller must supply the
//  value at placement time (semantic, per §13.2.2).
// **`DefaultAttrDecl` (per §13.2.2.1).** A node body admits *at most
//  one* `default attr` declaration — the positional default attr
//  for that node type. The grammar admits the `'default' 'attr'`
//  two-token form as a distinct production; the at-most-one rule is
//  a post-parse semantic check. The two-token sequence is not a
//  compound lexeme; `'default'` here is the keyword from the
//  reserved set.
// **`recurrent[N]` and `recurrent[N] stream <policy>[M]` (per
//  §13.2.4, §13.18.8).** Both bracketed forms of `RecurrentDecl`
//  (§7.15) are admissible inside a node body — per-instance history
//  cells and per-instance recurrent streams. The grammar shape
//  matches §7.15 exactly.
// **`StreamDecl` policy mandatory (per §13.18.2).** A node-body
//  `stream` declaration follows §7.15's `StreamDecl` shape: the
//  policy is mandatory — spelled as the `'ring'` / `'gate'` word
//  form or the `'[' TypeExpr ']'` bracket-policy form (§13.18.3),
//  both the same wiring type — the bracketed capacity on the word
//  form is optional (defaults to 1024 per §13.18.2), and the
//  `'=' Expr` source is optional only for host-fed streams
//  (post-parse semantic).
// **`ConstStmt` is `const` from §6.1 (per §13.3.1).** A node body
//  may declare per-type compile-time constants. The
//  compile-time-RHS rule of §6.1 applies; reactive / signal RHS
//  is rejected post-parse.

### 8.9.1 `yielded` group declarations (collect body)

A `yielded` declaration binds a *named* membership-varying group of
cells (§13.20.4). Its right-hand side is a `collect:` block whose
indented body contributes member-cells via `yield`. `yielded` is a
**body-only** declaration — it is wired into node bodies (§8.1) and
connection bodies (§9.1); it is *not* a module-top-level declaration
(§7.15 has no `yielded` alternative). The declaration form is the only
way to *name* a group (034-1).

```
YieldedDecl       ::= 'yielded' IDENT ':' TypeExpr '=' 'collect' ':' INDENT CollectBody DEDENT
                    ;  (§13.20.4, 034-1)

CollectBody       ::= CollectItem+
                    ;  (§13.20.1, 034-1)

CollectItem       ::= YieldStmt                                 // permanent member
                    | YieldRepeat                               // key-driven members
                    | YieldWhenBlock                            // activation-driven members
                    | YieldGivenBlock                           // activation-driven members
                    ;  (§13.20.3, 034-3)

YieldStmt         ::= 'yield' Expr
                    ;  (§13.20.2, 034-2)

YieldRepeat       ::= 'repeat' RepeatBind RepeatIndex? 'in' Expr RepeatKeyed? ':'
                      INDENT CollectBody DEDENT                  // RepeatBind / RepeatIndex / RepeatKeyed: §11.9
                    ;  (§13.20.3, §13.5.4.1)

YieldWhenBlock    ::= 'when' Expr ':' INDENT CollectBody DEDENT YieldOtherwiseArm?
                    | 'when' ':' INDENT YieldGuardArm+ YieldOtherwiseArm? DEDENT
                    ;  (§13.20.3, §13.9.12)

YieldGuardArm     ::= Expr ':' INDENT CollectBody DEDENT
                    ;  (§13.9.12, 022-98)

YieldOtherwiseArm ::= 'otherwise' ':' INDENT CollectBody DEDENT
                    ;  (§13.9.12, 022-93)

YieldGivenBlock   ::= 'given' Expr ':' INDENT YieldGivenArm+ YieldDefaultArm? DEDENT
                    ;  (§13.20.3, §13.9.13)

YieldGivenArm     ::= Pattern ':' INDENT CollectBody DEDENT
                    ;  (§13.9.13, 022-108)

YieldDefaultArm   ::= 'default' ':' INDENT CollectBody DEDENT
                    ;  (§13.9.13, 022-119)
```

// **Body-only; the retired `collect … as` statement form is gone (per
//  §13.20.1, 034-1).** A group is named ONLY by the `YieldedDecl` form
//  `yielded <name>: <MemberType> = collect:`. There is no
//  `collect:` … `as <name>:` naming statement. The anonymous
//  `collect:` block-expression form (a
//  `yielded T` value consumed in place — e.g. a `fold` members operand
//  §13.21, or a `yielded T`-typed argument §13.20.4.1) is an
//  *expression*, not a declaration; it is admitted wherever a value of
//  its kind is accepted. That anonymous expression form is part of the
//  wider `collect` / `fold` expression grammar this document does not
//  yet reformulate (gap).
// **`yield` is legal only inside a `collect` body (per §13.20.2,
//  034-4).** `YieldStmt` appears only within a `CollectBody` — directly
//  (permanent member), inside a `YieldRepeat` (one key-driven member
//  per live repetition key, §13.5.4), or inside a gated arm
//  (`YieldWhenBlock` / `YieldGivenBlock`; one activation-driven member
//  per effectively-active arm, §13.9.7). A `yield` anywhere else is a
//  compile error; `yield` is not enumerated as a general `BlockItem`.
// **Reuses the placement / gate arm nonterminals.** `RepeatBind`,
//  `RepeatIndex`, and `RepeatKeyed` are the §11.9 forms; the `when` /
//  `given` arm shapes parallel the cell-bearing `WhenBlockDecl` /
//  `GivenBlockDecl` of §7.14, but each arm body holds a `CollectBody`
//  (yields) rather than `DesiredCellDecl+`.
// **`yield` under a value `if` / `match` (per §13.20.3.1, 034-7).** SPEC
//  admits a `yield` nested under a *compile-time-known* value `if` /
//  `match` inside `collect` (the conditional is expanded); a runtime
//  condition is a compile error. This document enumerates only the
//  three membership drivers above; the compile-time `if` / `match`
//  expansion form belongs to the wider `collect` expression grammar
//  not yet reformulated here (gap).
// **`MemberType` may be any `TypeExpr` (per §13.2.8.1).** `yielded
//  Voice`, `yielded f32`, and `yielded f32[128]` are all well-formed;
//  the `TypeExpr` after the `:` is the member value type. The
//  outermost `yielded` head is a kind (§3.15); it does not route
//  through `KindAnnotation` because the declaration spells the kind
//  keyword inline.

### 8.10 `expose:` clause entries (view-name, named-child, wrapper, connection, `@content`, type-internal `for`/`repeat`, `when`/`given` blocks)

```
ExposeClause      ::= 'expose' ':' INDENT ExposeEntry+ DEDENT
                    ;  (§13.3.7, 017-21)

ExposeEntry       ::= ViewNameEntry
                    | NamedChildEntry
                    | WrapperPlacement
                    | ConnectionPlacement                        // see §11.4
                    | StandaloneDirective                        // see §12 head; @content per §12.2
                    | ExposeForEntry
                    | ExposeRepeatEntry
                    | ExposeWhenBlock                            // expose-entry arms; see §5.21
                    | ExposeGivenBlock                           // expose-entry arms; see §5.22
                    ;  (§13.3.7.1, 017-21)

ExposeWhenBlock   ::= 'when' Expr ':' INDENT ExposeEntry+ DEDENT ExposeOtherwiseArm?
                    | 'when' ':' INDENT ExposeWhenArm+ ExposeOtherwiseArm? DEDENT
                    ;  (§13.9.12, 022-88)

ExposeWhenArm     ::= Expr ':' INDENT ExposeEntry+ DEDENT
                    ;  (§13.9.12, 022-88)

ExposeOtherwiseArm ::= 'otherwise' ':' INDENT ExposeEntry+ DEDENT
                    ;  (§13.9.12, 022-88)

ExposeGivenBlock  ::= 'given' Expr ':' INDENT ExposeGivenArm+ ExposeDefaultArm? DEDENT
                    ;  (§13.9.13, 022-106)

ExposeGivenArm    ::= Pattern ':' INDENT ExposeEntry+ DEDENT
                    ;  (§13.9.13, 022-106)

ExposeDefaultArm  ::= 'default' ':' INDENT ExposeEntry+ DEDENT
                    ;  (§13.9.13, 022-106)

ViewNameEntry     ::= IDENT                                      // a declared view name
                    ;  (§13.3.7.1, 017-21)

NamedChildEntry   ::= IDENT                                      // a placement-named child
                    ;  (§13.3.7.1, 017-21)

WrapperPlacement  ::= TypeRef FlagsRun? NameClause?
                      DefaultArgPart? WhenClause? AttrClause?
                      BodyIntro?
                    ;  (§13.3.7.1, 021-123)
// **Element order matches §11.1 TypeRefPlacement (per LOG 021-123 /
//  SPEC §13.8.9).** `WhenClause` precedes `AttrClause`; the alias
//  `PlacementAs` is removed in favor of canonical `NameClause` of
//  §11 (`NameClause ::= 'as' IDENT`).

ExposeForEntry    ::= 'for' Pattern 'in' Expr NameClause? ':'
                      INDENT ExposeEntry+ DEDENT               // body holds further expose entries
                    ;  (§13.3.3.3, 017-219)

ExposeRepeatEntry ::= 'repeat' RepeatBind RepeatIndex? 'in' Expr
                      NameClause? RepeatKeyed? ':'
                      INDENT ExposeEntry+ DEDENT               // body holds further expose entries
                    ;  (§13.5.4.1, 018-43)

// RepeatBind / RepeatIndex / RepeatKeyed: see §11.9 for canonical
//  productions (per SPEC §13.5.4.1 / LOG 018-36 / 018-43).
```

// **`ViewNameEntry` vs `NamedChildEntry` resolved post-parse (per
//  §13.3.7.1).** Both are bare `IDENT`s; the kind distinction
//  follows from what the identifier resolves to in the body
//  namespace — a declared view (named acceptance entry of §8.3 /
//  §8.4 or `StandaloneView` of §8.5) or a placement-named child
//  introduced by an `as` binding earlier in the type. The grammar
//  unifies the two as the same surface; the name-resolution layer
//  picks the kind.
// **Entry-kind disambiguation for `Name: …` shapes (per §13.3.7.1
//  "Entry-kind disambiguation").** A `WrapperPlacement` headed by
//  a node-typed `TypeRef` followed by `':' BodyIntro` is a
//  wrapper placement (the body is a list of further exposition
//  entries); a `ConnectionPlacement` headed by a connection-typed
//  `TypeRef` followed by `':' <destination>` is a connection
//  placement (the body is the destination reference — see §11.4).
//  Both share the `Name: …` surface; the kind follows from the
//  name's resolution. A `Type[…]`-typed binding follows its
//  constraint's kind.
// **`@content` is the standalone-directive entry (per §13.3.7.2,
//  §12.2).** A `ContentDirective` entry is the `@content` form;
//  at most one per `ExposeClause` (post-parse). It exposes
//  caller-supplied children and outgoing connections in caller
//  order. The full directive production is in §12.2.
// **`ExposeForEntry` = type-emitted children via compile-time `for`
//  (per §13.3.3.3, §13.3.7.1 "Iteration entries").** The body is a
//  list of `ExposeEntry`s holding further nested placements; the
//  loop unrolls per §12.3.7. The optional `'as' IDENT` hoists inner
//  `as`-named placements into a `[<name>::entry; N]` array binding
//  (§11.10). The placement-position `for ... as <name>` is the
//  separately-named `ForAsPlacement` of §11.10 — same surface, but
//  `ForAsPlacement` admits a generic `BlockBody` (for use inside a
//  placement body), while `ExposeForEntry` admits only further
//  `ExposeEntry`s (for use inside an `expose:` clause).
// **`ExposeRepeatEntry` = `repeat`-mounted scopes (per §13.5.4.1).**
//  A `repeat` entry mounts one keyed scope per element of `<source>`;
//  the body is a list of `ExposeEntry`s. The placement-position
//  variant is `RepeatPlacement` of §11.9 — identical clause shape,
//  but admits a generic `BlockBody` rather than the
//  `ExposeEntry+` restriction here. The clause order is fixed per
//  §13.5.4.1: `<bind>`, optional `at <index>`, `in <source>`,
//  optional `as <view>` (§11.9), optional `keyed by <key-expr>`.
// **`WhenBlockExpr` and `GivenBlockExpr` as entries (per §13.3.7.1
//  "Conditional exposition").** The block forms of §5.21 / §5.22
//  are admissible as exposition entries; each arm's body is itself
//  a list of `ExposeEntry`s. Arm-position discrimination (variant
//  labels in `given`, boolean guards in `when:`) keeps the arm-head
//  shape distinct from a `ConnectionPlacement` `Name: dest` line —
//  per §13.3.7.1 final paragraph.
// **Bare `incoming` connection-view name forbidden as an entry (per
//  §13.3.7.1).** Naming an `IncomingClause` view in `expose:` is a
//  post-parse semantic error — engagement order belongs to the
//  source's traversal, not the destination's. The grammar admits
//  the bare `IDENT` surface uniformly; the rejection happens at
//  name resolution.
// **Effects are not exposition (per §13.3.7.5 final paragraph,
//  §13.3.8).** No effect-instantiation form (`name = src |> eff`)
//  appears in `ExposeEntry`; effects live exclusively in the
//  `effects:` clause (§8.11). The grammar enforces the partition by
//  enumerating only the entry kinds above.

### 8.11 `effects:` clause entries

```
EffectsClause     ::= 'effects' ':' INDENT EffectEntry+ DEDENT
                    ;  (§13.3.8, 031-131)

EffectEntry       ::= NamedEffectEntry
                    | AnonEffectEntry
                    | EffectsForEntry
                    | EffectsRepeatEntry
                    | EffectsWhenBlock
                    | EffectsGivenBlock
                    ;  (§13.3.8, 031-114)

NamedEffectEntry  ::= IDENT '=' Expr                             // RHS is a `|>` pipe chain
                    ;  (§13.3.8, 031-69)

AnonEffectEntry   ::= Expr                                       // bare `<source> |> <effect>`
                    ;  (§13.3.8, 031-114)

EffectsForEntry   ::= 'for' Pattern 'in' Expr NameClause? ':'
                      INDENT EffectEntry+ DEDENT
                    ;  (§13.3.8, 022-3)

EffectsRepeatEntry ::= 'repeat' RepeatBind RepeatIndex? 'in' Expr
                      NameClause? RepeatKeyed? ':'
                      INDENT EffectEntry+ DEDENT
                    ;  (§13.5.4.1, 022-3)

EffectsWhenBlock  ::= WhenBlockExpr                              // arm bodies hold `EffectEntry`s
                    ;  (§13.9.12, 022-88)

EffectsGivenBlock ::= GivenBlockExpr                             // arm bodies hold `EffectEntry`s
                    ;  (§13.9.13, 022-106)
```

// **Sole effect-instantiation site (per §13.3.8, §13.19.15).** An
//  effect may be instantiated *only* inside an `EffectsClause`;
//  module level, operator bodies, connection bodies, function
//  bodies, and the `ExposeClause` are all rejected hosts. The
//  grammar enforces partitioning by *not* admitting `EffectEntry`
//  forms in any other clause's production.
// **Named vs anonymous entry (per §13.3.8).** A `NamedEffectEntry`
//  (`<name> '=' <expr>`) binds the effect instance value into the
//  body namespace; an `AnonEffectEntry` (bare `<expr>`) runs for
//  side effect only and binds nothing. The two are syntactically
//  distinguished by the presence of `IDENT '='` at the head of the
//  line.
// **RHS must be a `|>` pipe chain (per §13.3.8, §13.17.7).** The
//  expression on either entry's RHS is, in practice, a `PipeExpr`
//  of §5.25 ending with an operator or effect call (`<source> |>
//  <op> |> <effect>`). The grammar admits any `Expr`; the
//  pipe-chain shape and the operator-or-effect terminator are
//  post-parse semantic checks per §13.17.7.
// **Arm headers `':'` vs named entries `'='` never collide (per
//  §13.3.8 "`when` / `given` / `repeat`").** A `WhenSimple` or
//  `GivenArm` header uses `':'` (`Variant:`, `when cond:`); a
//  `NamedEffectEntry` uses `'='` (`name = src |> eff`). The two
//  surfaces are syntactically disjoint at the head of a line, so
//  the parser distinguishes arm-position lines from named-effect
//  lines unambiguously.
// **`EffectsForEntry` and `EffectsRepeatEntry` (per §13.3.8,
//  §13.5.4.1).** Both control-flow forms admit effect-entry bodies
//  — one effect per iteration. Their head clauses mirror the
//  `ExposeClause` counterparts of §8.10 exactly; the body
//  contents differ in entry-kind only.
// **`when` / `given` arm bodies hold effect entries (per §13.3.8
//  "`when` / `given` / `repeat`" final paragraph).** Inside an
//  `EffectsClause`, the arm body of a `WhenBlockExpr` or
//  `GivenBlockExpr` is itself a list of `EffectEntry`s — not
//  exposition entries, not statements. The grammar reuses the
//  block productions of §5.21 / §5.22; the enclosing clause
//  controls arm-body admissibility.
// **No effect parameters; no effect children (per §13.3.8
//  "Local-only").** A node defines its own effects; effects are
//  never supplied from outside. The grammar provides no
//  effect-acceptance clause and no effect-parameter form on any
//  declaration head.

## 9. Connection declarations

Productions for `connection` declarations: shell, single / cartesian / pairs forms, body references, when, circularity.

### 9.1 Common shell

```
ConnectionDecl    ::= Visibility? 'connection' IDENT GenericParamList?
                      WhereClause? ConnectionBody
                    ;  (§13.6.1, 017-21)
// Directive decoration attaches via the §12.3 `AnnotatedDecl` wrapper
//  (Phase D, D4) — `ConnectionDecl` carries no inline `Annotation*` head.

ConnectionBody    ::= ':' INDENT ConnectionBodyMember+ DEDENT
                    ;  (§13.6.1, 017-21)

ConnectionBodyMember ::= SatisfiesClause                         // §8.2
                    | FromDecl                                   // §9.2
                    | ToDecl                                     // §9.2
                    | FromMultiDecl                              // §9.3
                    | ToMultiDecl                                // §9.3
                    | PairsClause                                // §9.4
                    | TypeLevelWhenClause                        // §8.8 / §9.6
                    | ConnectionBodyCellDecl                     // §9.1 (cells)
                    ;  (§13.6.1, 017-21)

ConnectionBodyCellDecl ::= AttrDecl                              // §8.9
                    | DefaultAttrDecl                            // §8.9
                    | DerivedDecl                                // §7.15
                    | RecurrentDecl                              // §7.15
                    | StreamDecl                                 // §7.15
                    | ConstStmt                                  // §6.1
                    | YieldedDecl                                // §8.9.1 (named yielded group)
                    ;  (§13.6.1.1, 017-21)
```

// **Connection body is a clauses-and-cells set (per §13.6.1).** A
//  `ConnectionBodyMember` is exclusively a clause, an endpoint
//  declaration, or a cell declaration. Bare placements, `fn`
//  declarations, `repeat` declarations, and effect entries are *not*
//  admissible as direct members — per §13.6.4. The grammar enforces
//  this by enumerating only the alternatives above.
// **Free-order body with cardinality constraints (per §13.6.1 lead
//  paragraph).** Body members may appear in any order; the layout
//  shown in SPEC examples is conventional, not mandated. The only
//  structural constraints are cardinality: exactly one `from:` /
//  `to:` pair (`FromDecl` or `FromMultiDecl`, plus `ToDecl` or
//  `ToMultiDecl`) *or* exactly one `PairsClause` appears in the
//  body; `SatisfiesClause`, `TypeLevelWhenClause`, and
//  `DefaultAttrDecl` each appear at most once. The grammar's `+`
//  admits multiples and admits the endpoint members independently
//  (so `from:` and `to:` may be separated by other clauses); the
//  at-most-once, exactly-once, and form-exclusivity rules are
//  post-parse semantic checks.
// **Exactly one endpoint form (per §13.6.1).** A connection uses
//  *one* of the three endpoint forms — single, cartesian, or pairs.
//  Mixing forms (e.g. `pairs:` alongside `from:`+`to:`) is a
//  compile-time error. The grammar admits the endpoint-member
//  alternatives independently (`FromDecl`, `ToDecl`, `FromMultiDecl`,
//  `ToMultiDecl`, `PairsClause`); the exclusivity, the
//  exactly-one-`from`-and-one-`to` cardinality, and the
//  single-vs-cartesian classification (see §9.2 / §9.3
//  disambiguators) are post-parse.
// **`SatisfiesClause` reuses §8.2 (per §13.6.5).** The trait list
//  carries language-defined marker traits like `Circularity` (§9.7)
//  as well as domain traits. Same production, same shape.
// **Cell forms mirror node body cells (per §13.6.1.1, §13.3.1).**
//  `AttrDecl`, `DefaultAttrDecl`, `DerivedDecl`, `RecurrentDecl`,
//  `StreamDecl`, and `ConstStmt` are the same productions used in
//  node bodies (§8.9) and at module top level (§7.15). A connection
//  body admits *at most one* `DefaultAttrDecl` per §13.2.2.1 — the
//  positional default attr — checked post-parse.
// **No `fn` / `repeat` / `effects:` (per §13.6.4).** A connection
//  body never contains `FnDecl`, `RepeatDecl` (no `expose:` either,
//  see below), or `EffectsClause`. Behavior on connections lives in
//  free functions (UFCS) and `fulfill` blocks. Dynamic-scope
//  structure belongs in node `expose:` blocks or placement bodies.
// **No `expose:` clause (per §13.6.1, §13.6.4).** A connection
//  surfaces its endpoints only body-internally (§13.6.2 / §9.5) and
//  has no exposition entries. The grammar admits no `ExposeClause`
//  alternative in `ConnectionBodyMember`.
// **No `children:` / `incoming:` / `outgoing:` (per §13.6 vs §13.3).**
//  Acceptance clauses are node-only; a connection neither admits
//  children nor accepts inbound or outbound wiring beyond its two
//  endpoints. The grammar enumerates no acceptance-clause
//  alternative here.

### 9.2 Single form (`from: T` / `to: U`)

```
FromDecl          ::= 'from' ':' TypeExpr
                    ;  (§13.6.1.1, 017-21)

ToDecl            ::= 'to' ':' TypeExpr
                    ;  (§13.6.1.1, 017-21)
```

// **Each of `from:` / `to:` required, exactly once (per §13.6.1.1
//  / §13.6.1 lead paragraph).** A single-form connection must
//  declare both a `FromDecl` and a `ToDecl`, each appearing
//  *exactly once* in the body. `FromDecl` and `ToDecl` are
//  admitted as independent `ConnectionBodyMember` alternatives
//  (§9.1), so they may appear in either order and may be
//  separated by other body members (e.g. `satisfies`, `attr`,
//  `when`); the exactly-once-each rule is a post-parse semantic
//  check across the whole `ConnectionBody`, not enforced by the
//  local production.
// **`TypeExpr` is a node type or trait (per §13.6.1.1).** Each
//  endpoint's `TypeExpr` resolves to a node type — concrete or, via
//  a trait `TypePath`, an open trait-typed slot per §3.1.6. The
//  grammar admits any `TypeExpr`; the node-or-trait restriction is
//  post-parse.
// **No commas in single form (per §13.6.1.1 vs §13.6.1.2).** A
//  single-form `from:` / `to:` carries *one* type — no comma list.
//  A `from:` / `to:` line with a comma-separated list is the
//  cartesian form of §9.3, not the single form.
// **No `match` required in single-form body (per §13.6.2 first
//  bullet).** With one `from` type and one `to` type, body
//  expressions reference `from` and `to` directly at their declared
//  types — no `(from, to)` match arm is needed. Cells in the body
//  use `from.<field>` / `to.<field>` per §9.5.

### 9.3 Cartesian form (multiple types + body's `match (from, to):`)

```
FromMultiDecl     ::= 'from' ':' TypeExpr ( ',' TypeExpr )+ ','?
                    ;  (§13.6.1.2, 017-21)

ToMultiDecl       ::= 'to' ':' TypeExpr ( ',' TypeExpr )+ ','?
                    ;  (§13.6.1.2, 017-21)
```

// **Cartesian = comma-listed types on at least one endpoint (per
//  §13.6.1.2).** The cartesian form is selected when `from:` or
//  `to:` carries a comma-separated `TypeExpr` list of length ≥ 2.
//  Either or both endpoints may be multi-typed; the mixed cases
//  (single on one side, multi on the other) are still cartesian.
//  `FromMultiDecl` and `ToMultiDecl` are admitted as independent
//  `ConnectionBodyMember` alternatives (§9.1) alongside the
//  single-arity `FromDecl` / `ToDecl`, so any combination —
//  `FromMultiDecl` + `ToMultiDecl`, `FromMultiDecl` + `ToDecl`,
//  or `FromDecl` + `ToMultiDecl` — is admitted by the body.
// **Single-vs-cartesian disambiguation is local per `from:` /
//  `to:` line, classification is post-parse over the pair (per
//  §13.6.1.1 vs §13.6.1.2).** At each `from:` / `to:` line the
//  grammar disambiguates `FromDecl` from `FromMultiDecl` (resp.
//  `ToDecl` from `ToMultiDecl`) by the presence of `','` after
//  the first `TypeExpr` — purely local lookahead. The overall
//  *form* of the connection (single vs. cartesian) is then a
//  post-parse classification over the pair of endpoint members:
//  the form is **single** iff both endpoints parsed as the
//  single-arity production (`FromDecl` + `ToDecl`), and
//  **cartesian** otherwise (any combination involving at least one
//  `FromMultiDecl` or `ToMultiDecl`). This deferral is analogous to
//  the form-exclusivity check called out in §9.1.
// **Match-on-`(from, to)` is REQUIRED in body cells (per §13.6.1.2,
//  §13.6.2 second bullet).** The body is specialized per
//  from×to combination; cells reading endpoint state typically
//  do so via a `match (from, to):` `MatchExpr` (§5.19) on the tuple
//  formed from `from` and `to`. The required form of the match
//  itself is admitted by the existing `MatchExpr` production — no
//  new grammar rule is introduced here. The *requirement* that
//  every cross-type body expression be covered by a `(from, to)`
//  match arm is a post-parse semantic check.
// **Exhaustiveness over the cartesian product (per §13.6.1.2,
//  §13.6.2).** A `match (from, to):` in the body must be
//  *exhaustive* over the full cartesian product of the declared
//  endpoint types — every from-type × to-type combination admitted
//  by the declaration must have a corresponding arm (or be covered
//  by a `default:` arm per §6.2.4). The grammar admits any
//  `MatchExpr`; exhaustiveness is a post-parse semantic check.
// **Concretely-typed `from` / `to` within each arm (per §13.6.1.2
//  final paragraph, §13.6.2 second bullet).** Inside each
//  combination's arm, `from` and `to` are bound to their concrete
//  per-combination types. The grammar surface is the standard
//  variant-pattern shape of §4.2 — `(Person(p), Vehicle(v)): …` —
//  applied to the tuple. No new pattern form is introduced.

### 9.4 Pairs form (`pairs:` listing + body's `match pair:`)

```
PairsClause       ::= 'pairs' ':' INDENT PairEntry+ DEDENT
                    ;  (§13.6.1.3, 017-21)

PairEntry         ::= TypeExpr '->' TypeExpr
                    ;  (§13.6.1.3, 017-21)
```

// **Pairs form lists explicit `From -> To` rows (per §13.6.1.3).**
//  Each `PairEntry` declares one admissible
//  `(<from-type>, <to-type>)` placement combination. The arrow
//  token is the existing `'->'` lexeme used elsewhere for function
//  return types (§3.3) and pattern-arm separators; here it
//  separates the two endpoint type expressions in a pair row.
// **One entry per line under the clause header (per §13.6.1.3
//  example layout).** The `INDENT ... DEDENT` block holds one
//  `PairEntry` per line; the layout pre-processor of §2.1 produces
//  the line boundaries.
// **Pairs are unique; duplicates rejected post-parse (per
//  §13.6.1.3 "Rules for pairs form").** Listing the same
//  `From -> To` twice is a compile-time error. The grammar admits
//  the `+` repetition without uniqueness; the uniqueness check is
//  post-parse.
// **Asymmetric counts allowed (per §13.6.1.3).** The number of
//  distinct from-types and to-types across the pair list need not
//  match; pair uniqueness, not type count, is the constraint. The
//  grammar imposes no count relation between the two sides.
// **Match-on-`pair` is REQUIRED in body cells (per §13.6.1.3,
//  §13.6.2 third bullet).** In pairs form, `from` and `to` are
//  *not* independently accessible; endpoint pairs are extracted via
//  a `match pair:` `MatchExpr` (§5.19) on the reserved
//  `pair` name (§9.5). The required form is the existing
//  `MatchExpr` production — no new grammar rule is introduced.
//  The requirement that endpoint reads go through `match pair:` is
//  a post-parse semantic rule.
// **Exhaustiveness over declared pairs (per §13.6.1.3 last
//  bullet, §13.6.2).** A `match pair:` must be *exhaustive* over
//  the declared `pairs:` list (or carry a `default:` arm, per
//  §6.2.4) — exactly as the cartesian form's match exhaustiveness
//  (§9.3). The grammar admits any `MatchExpr`; the exhaustiveness
//  check is post-parse.
// **Body is uniform across pairs (per §13.6.1.3 third bullet).**
//  All `attr` / `derived` declarations in a pairs-form body apply
//  uniformly to every declared pair; the body cannot vary its
//  *shape* by pair. When per-pair shape is needed, declare a
//  separate connection type per pair. The grammar enforces this by
//  not admitting any per-pair body-shape selector — pairs are
//  selected only inside `match pair:` arms.

### 9.5 `from` / `to` / `pair` body references (instance-field reserved names)

```
// No new production: `from`, `to`, and `pair` are reserved
// instance-field names resolved by name resolution inside a
// connection body. Their surface in expressions is the standard
// `IDENT` of §5.1 (primary expression), with member access via
// `from.<field>` / `to.<field>` per the postfix forms of §5.2.
//                                                  ;  (§13.6.2, 002-5)
```

// **Three reserved instance-field names (per §13.6.2, 002-5).**
//  Inside a connection body, the bare identifiers `from`, `to`, and
//  `pair` are *reserved instance-field names* — the
//  contextual-keyword set of §13.4 (per DECISION_LOG 002-5). They
//  resolve at parse-shape time to endpoint-reference values
//  without explicit declaration in the body; the grammar treats
//  them as standard `IDENT` occurrences at the primary-expression
//  position of §5.1.
// **Form-conditioned availability (per §13.6.2).** Which names are
//  bound depends on the declared endpoint form:
//   - **Single form** (§9.2): `from` and `to` are bound; `pair` is
//     *not* bound.
//   - **Cartesian form** (§9.3): `from` and `to` are bound (the
//     body specializes per combination via `match (from, to):`);
//     `pair` is *not* bound.
//   - **Pairs form** (§9.4): only `pair` is bound. `from` and `to`
//     are *not independently accessible* — they must be extracted
//     from `pair` via `match pair:` patterns per §9.4. Bare `from`
//     or `to` in a pairs-form body is a post-parse name-resolution
//     error.
// **Postfix access via `.` (per §13.6.2 first bullet, §5.2).**
//  Endpoint attrs and deriveds are read with the standard
//  postfix-`.` form of §5.2: `from.expertise_level`, `to.top_speed`.
//  The grammar introduces no special accessor; reuses the
//  primary-and-postfix shape.
// **Body-internal scope only (per §13.6.2 final paragraph).**
//  `from`, `to`, and `pair` are visible *only inside the
//  connection type's own body*. No external `some_conn.to` access
//  is admitted — the grammar's `PathExpr` / postfix forms simply
//  do not bind these names outside the connection body's scope.
//  This is a name-resolution rule, not a separate grammar form.
// **Reactive re-evaluation on `to` re-point (per §13.6.2).** The
//  `to` binding tracks the destination supplied at placement time;
//  reads of `to.*` re-evaluate when the destination re-points.
//  This is a reactive-provenance rule (§13.10.5 / §13.12.1), not a
//  grammar one; the grammar admits the bare `to` / postfix access
//  identically regardless of dynamism.

### 9.6 Connection-declaration-level `when:` clause

```
// Reuses §8.8: `TypeLevelWhenClause ::= 'when' ':' Expr`
//                                                  ;  (§13.9.2, §13.6.1.1, 022-3)
// Reuse-note source pointer (per §0 conventions). The pointer carries
//  two SPEC sections: §13.9.2 is the primary normative source for the
//  `when:` clause shape; §13.6.1.1 is the reuse-origin (where a
//  connection body admits this clause). This is a documented reuse
//  note, not a second source pointer on a new production.
```

// **Reuses node-body `when:` shape (per §13.9.2, §13.6.1.1).** A
//  connection body admits the same `TypeLevelWhenClause` as a node
//  body (§8.8). The clause form is `'when' ':' Expr` — colon-form,
//  consistent with other body fields (`from:`, `to:`, `attr name:`)
//  and distinct from the placement-level inline `when` modifier of
//  §11.8 (no colon).
// **At most one per connection body (per §13.9.2).** A second
//  `when:` clause in the same `ConnectionBody` is a post-parse
//  semantic error; the grammar admits multiple via the `+` body
//  repetition, with the at-most-once rule enforced semantically.
// **Same gate-and-freeze semantics as on a node (per §13.9.7,
//  §13.6.2).** When the predicate evaluates to false, the
//  connection *freezes*: cells retain their last committed value
//  and the body does not re-evaluate. The freeze condition combines
//  with the unresolved-destination freeze of §13.6.2 (a `WeakHandle`
//  destination resolving to `None`); these two are the only
//  switches on a connection's reactive liveness. The grammar
//  admits the `Expr` unrestricted; the bool-typed reactive-predicate
//  rule is post-parse.
// **Predicate is a reactive `Expr` (per §13.9.4).** The predicate
//  joins the connection's provenance and re-evaluates when its
//  references change. Endpoint references inside the predicate
//  follow the same form-conditioned availability rules of §9.5
//  (`from` / `to` in single / cartesian forms; `pair` in pairs
//  form). The grammar admits any `Expr`; the bool-result rule is
//  post-parse.

### 9.7 `Circularity` marker note

```
// No new production: `Circularity` is a language-defined marker
// trait (§3.7.4) named in a `SatisfiesClause` (§8.2) of a
// `ConnectionDecl`. The grammar surface is the standard `TypePath`
// of §3.1 in the satisfies list — no special trait-name form.
//                                                  ;  (§13.6.5, 019-73)
```

// **Surface = `satisfies Circularity` (per §13.6.5).** A
//  connection type opts into participation in topology cycles by
//  listing the `Circularity` marker trait in its `SatisfiesClause`.
//  No special grammar form attaches; the `TypePath` is the same
//  as for any other trait name (per §3.1 / §8.2). The parser does
//  not special-case `Circularity` — name resolution does.
// **Marker trait — no method bodies (per §13.6.5, §3.7.4).**
//  `Circularity` is a language-defined marker trait with no
//  methods; `satisfies Circularity` requires no accompanying
//  `fulfill` block. The grammar enforces nothing here — the
//  empty-method-set fact follows from the trait's language-defined
//  declaration.
// **Static cycle rule is post-parse (per §13.6.5).** The
//  compiler enforces that every topology cycle in the
//  construction-time node graph traverses at least one connection
//  satisfying `Circularity`; cycles consisting only of
//  non-`Circularity` connections are compile-time errors. This is
//  a graph-analysis rule across many declarations, not a
//  per-declaration grammar form, and lives entirely outside the
//  scope of the grammar.

## 10. Bundles

Productions for bundle types (as **view selectors**) and bundle
**placements** (the surface form `[...]` written at a placement site). A
bundle is the bracketed co-placement form (§13.3.3.5): an explicit
`[...]` at a placement site **is** a bundle; a bare placement is **not**
— bundles always carry the bracket as part of the surface (017-94).
Bundles are 2D only — exactly one nesting level (017-101).

### 10.1 Bundle as view selector `[T]+`, `[T[=N]]+`

A *bundle view selector* names a bundle of placed children rather than
the placed children themselves. Inside the brackets sits a view-style
selector with its own inner cardinality; outside the brackets sits the
**outer** cardinality (bundle count). Inner cardinality is part of the
match predicate (a filter), outer cardinality is the count constraint
(017-93).

```
BundleViewSelector ::= '[' BundleViewInner ']' Cardinality?    // bare outer = exactly one
                    ;  (§13.3.3.5, 017-94)

BundleViewInner   ::= TypeExpr Cardinality
                    | TypeExpr                                  // bare = exactly one
                    ;  (§13.3.3.5, 017-93)

// Cardinality is defined in §8.7. The default — no sigil, no bracket —
//  means exactly one (013-… / 017-…); a member of the bundle written
//  without a specifier is one placement (per §13.3.3.1).
// Examples (per §13.3.3.5):
//   [Note[=2]]+        — 1+ bundles, each containing exactly 2 Notes
//   [Note]+            — 1+ single-element bundles
//   [Drivable[2..4]]+  — 1+ bundles, each 2..4 Drivables
//   [Note[=2]]+ and [Note[=3]]+ select disjoint bundle sets (017-93).
// The selector matches only *bracketed* co-placements; flat (bare)
//  placements are never matched by a bundle selector and are seen by
//  flat (non-bundle) selectors of the same element type (017-… —
//  "flat views flatten; bundle views see brackets", §13.3.3.5).
// Nested-nested bundle selectors `[[T]]` are rejected — bundles are
//  2D only (017-101); see §10.4.
```

### 10.2 Bundle placement `[n1 n2]`

A *bundle placement* is the bracketed co-placement at a placement site.
The brackets are part of the surface and survive in the structural
output (§13.3.3.5). Only an explicit `[...]` is a bundle — a bare
placement is **not** a bundle (017-94).

```
BundlePlacement   ::= '[' BundleMember ( BundleMember )* ']'   // members are self-delimiting under suspended layout (§10.5)
                    | '[' ']'                                  // empty bundle (§13.3.3.5)
                    ;  (§13.3.3.5, 017-94)

BundleMember      ::= BundleInnerPlacement                      // node placement; no indented-block body inside [...]
                    | CompileTimeFor                            // `for ... in <ct-iter>: <member>`
                    ;  (§13.3.3.5, 017-96)

BundleInnerPlacement ::= TypeRef FlagsRun? NameClause? DefaultArgPart? WhenClause? AttrClause?
                    ;  (§13.3.3.5, 017-98)

CompileTimeFor    ::= 'for' Pattern 'in' Expr ':' BundleMember ( BundleMember )*   // inline form admits one or more members
                    ;  (§13.3.3.5, 017-96)

// Iteration variable is a `Pattern` (per SPEC §12.12.1, 014-150..152);
//  the pattern must be irrefutable — a post-parse check.
```

// **Contents allowed inside `[...]` (per 017-96).** Node placements
//  (the common case) — including gated node placements, where the
//  `when` modifier (§11.1's `WhenClause?` slot) freezes membership
//  but does not remove the placement from the bundle; and a
//  compile-time `for` unrolling into one or more members per
//  iteration. Whether the body is recognised as compile-time is
//  post-parse (§12.3.7 / §13.5.4), not a grammar rule.
// **Forbidden inside `[...]` (per 017-96 / §13.3.3.5).** `repeat` is
//  rejected (dynamic bundles use the reactive `Cell` form, §13.3.3.5);
//  connection placements are rejected (bundling is node-only). The
//  rejections are post-parse — the grammar admits the same brackets and
//  the resolver / type-checker reports the diagnostic at the inner
//  construct's site.
// **Empty bundle.** `[]` is a legal `BundlePlacement` (017-95 /
//  §13.3.3.5); its element count is zero. The type is `Bundle[T]` for
//  any `T` the context infers (semantic — post-parse).

### 10.3 `as <name>` row-slice binding

A bundle placement may be named with the `as` marker, exactly as any
other placement (§11). The bound name is the **row-slice** (a borrow of
the row), not a bare `Handle[T]` (017-97). The bind position is the
same `NameClause` slot as for non-bundle placements per §11.1.

```
NamedBundlePlacement ::= BundlePlacement 'as' IDENT
                    ;  (§13.3.3.5, 017-97)
```

// Per 017-97 / §13.3.3.5, `[n1 n2] as pair` binds `pair` to a borrow
//  of the row-slice form — `Handle[T][..N]` when the row's inner
//  cardinality is statically known; `Handle[T][..]` when only the
//  runtime length is known. The bind is always a borrow (placement
//  bindings are always borrows, §13.8.x). The element type is
//  `Handle[T]` because the bundle backing stores Handles; `as <name>`
//  never binds to a bare `Handle[T]` (017-97).
// In the inline-element ordering of §11.1, the bundle placement's
//  `as` slot sits where any other placement's `as` slot sits — after
//  the surface form, before any `when` modifier. (The bundle has no
//  `TypeRef` / `FlagsRun` / `/Expr` of its own; those clauses belong
//  to the *members* inside `[...]`.)
// `as` in nested (bundle-member) position has the §11 rule: an
//  individual member may carry its own `as` to name itself, since
//  nested placements may be anonymous (§11.3).

### 10.4 2D-only constraint

Bundles are exactly one nesting level (017-101). The surface form
`[[T]]` — a bundle whose member is itself a bundle selector — is
**rejected**. Deeper structure uses nested node bodies.

```
//  Rejected at the bundle-view selector level (per 017-101):
//    [[Note]]+                   ✗
//    [[Note[=2]]+]+              ✗
//  Rejected at the bundle placement level (per 017-101 / §13.3.3.5):
//    [[A B] [C D]]               ✗   (inner brackets at member position)
```

;  (§13.3.3.5, 017-101)

// The rule is enforced at the bundle-recognition layer: any `[...]`
//  whose immediate (non-`for`-unrolled) member is itself a `[...]`
//  bundle surface is a compile error. The grammar in §10.2's
//  `BundleMember` admits only `Placement | CompileTimeFor` — neither
//  of those reduces to a `BundlePlacement`, so the 2D-only rule is
//  captured by the production itself (no separate post-parse check
//  needed for nested-direct cases). The diagnostic identifies the
//  inner `[`.

### 10.5 Open-delimiter layout (layout suspended inside `[…]`)

Inside `[...]`, layout is **suspended** (017-98 / §13.3.3.5) — parallel
to the layout-suspension rule for `(...)` and string literals (§2.1,
002-23). Newlines and indentation inside the brackets are whitespace,
not logical-line terminators; members may span multiple physical lines
and carry attrs, children, and `when`-gating.

```
[
  Note | pitch=60 duration=0.25
  Note | pitch=64 duration=0.25
  Rest                           when (silent_break is true)
]
```

// **Predicate parenthesisation note (per 021-127).** The original
// SPEC example uses `when silent_break: true`, but that form makes
// the `:` ambiguous between predicate-tail and body-introducer.
// Per 021-127 a `when` predicate carrying an unparenthesised `:`
// MUST be parenthesised. The example above uses the
// `is`-comparison form to avoid the `:` entirely.

;  (§13.3.3.5, 017-98)

// This is the open-delimiter rule of §2.1's bracket-depth counter
//  applied to `[`/`]` brackets (per the layout pre-processor algorithm,
//  §2.1). The `[` increments the depth counter; until the matching `]`,
//  physical-line newlines are absorbed as whitespace and no
//  INDENT/DEDENT/NEWLINE tokens are synthesised. Same-line whitespace
//  is the only separator between bundle members.
// The members inside `[...]` follow the same per-line layout rules
//  the lexer normally enforces — including same-line multi-member
//  separation by whitespace (parallel to §11.11) — but the rules for
//  *line termination* are suspended: each member ends when the
//  parser-side construct ends, not when a physical-line newline
//  appears.

## 11. Placements

Productions for placement syntax: the inline element order (§13.8.9),
the top-level / child / connection placement variants (§13.8.1 /
§13.8.3 / §13.8.4), the attribute clause (§13.8.7), the flag run
(§13.8.8), the `/expr` default-attr targeting (§13.8.5), the inline
`when` modifier (§13.9), the `repeat … as` and `for … as` view
bindings (§13.5.4.9 / §13.3.3.3), and the same-line multi-placement
self-delimiting rule (§13.8.10).

### 11.1 Inline element order: `TypeRef [Flags] [as Name] [/Expr] [when Pred] [| AttrClause] [: Body]`

A placement's inline elements have a **fixed order** (§13.8.9). Every
element after `TypeRef` is optional; when present, each appears at
most once and in the order shown. Quoting the SPEC line verbatim
(§13.8.9):

> `TypeRef [FlagsRun]? [NameClause (`as` Name)]? [DefaultArgPart (`/Expr`)]? [WhenClause (`when` Pred)]? [AttrClause]? [BodyIntro (`:` Body)]?`

```
Placement         ::= TypeRefPlacement
                    | NamedBundlePlacement                      // see §10.3
                    | BundlePlacement                           // see §10.2
                    | ConnectionPlacement                       // see §11.4
                    ;  (§13.8.9, 021-123)

TypeRefPlacement  ::= TypeRef FlagsRun? NameClause? DefaultArgPart? WhenClause? AttrClause? BodyIntro?
                    ;  (§13.8.9, 021-123)

TypeRef           ::= TypePath ( '[' GenericArgList ']' )?
                    ;  (§13.8.9, 021-123)

NameClause        ::= 'as' IDENT
                    ;  (§13.8.9, 021-7)

DefaultArgPart    ::= '/' DefaultArgExpr
                    ;  (§13.8.5, 021-72)

WhenClause        ::= 'when' Expr
                    ;  (§13.8.9, 021-124)

BodyIntro         ::= ':' INDENT PlacementBody DEDENT
                    | ':' InlineBody
                    ;  (§13.8.9, 021-92)

PlacementBody     ::= PlacementBodyItem ( NEWLINE PlacementBodyItem )* NEWLINE?
                    ;  (§13.8.9, 021-92)

PlacementBodyItem ::= Placement                                 // nested child / connection / bundle placement
                    | BlockItem                                 // ordinary statement-shape (let / for / while / Expr …)
                    ;  (§13.8.9, 021-92)

InlineBody        ::= SelfDelimitingPlacement ( SelfDelimitingPlacement )*
                    ;  (§13.8.10, 021-137)
```

// **`PlacementBody` reaches nested placements (per §13.8.9, §11.2 /
//  §11.3 / §11.4).** Inside the indented body of a `TypeRefPlacement`
//  or `TopLevelPlacement`, the parser admits any `Placement` (nested
//  child / connection / bundle / wrapper) as a body item in addition
//  to ordinary `BlockItem` statement shapes. The discrimination
//  between `Placement` and `BlockItem` is by the head token: a
//  `TypeRef` head (a `TypePath` that names a node / connection /
//  bundle type) selects `Placement`; `let` / `mut` / `const` / `for`
//  / `while` / `return` / `break` / `continue` and bare-expression
//  shapes select `BlockItem`. This wires the §11 placement productions
//  reachably into every body that accepts them.

// **Order is normative (per §13.8.9, 021-123).** Re-ordering elements
//  (e.g., `as` after `|`) is a parse error. The order is the same for
//  node and connection placements.
// **Flags adjacency (per §13.8.8.4, 021-119).** The `FlagsRun`, when
//  present, sits **adjacent to `TypeRef`** with no intervening
//  whitespace — see §11.6.
// **`as` optionality differs by placement context (per 021-7 /
//  021-35).** At top-level (§11.2), `as` is **optional** — the bare
//  declaration form `TypeName instance_name` carries the name
//  positionally. In nested placements (§11.3), `as` is **required**
//  to name the placement, since nested placements may be anonymous.
// **`/Expr` requires a declared `default attr` on the placed type**
//  (021-74) — post-parse semantic check; the grammar admits any
//  `Placement` syntactically.
// **`WhenClause` slot (per 021-125).** When `/Expr` is absent, the
//  `when` clause slots immediately after whichever preceding element
//  is present. The fixed-order grammar above admits this naturally —
//  every preceding element is optional and the clause sequence is
//  prefix-free.
// **`when` predicate may not contain an unparenthesized `:` (per
//  021-127).** A predicate whose tokens include a bare `:` collides
//  with the body-introducer `:` of `BodyIntro`; such a predicate must
//  be parenthesized. Common predicates are flat boolean expressions
//  and need no parens.
// **Inline `when` is the single-placement modifier (per 021-126).**
//  `when` blocks (§5.21) and `given` blocks (§5.22) are not
//  inline-element modifiers; they appear as standalone entries at
//  `expose:` / body level and do not participate in this ordering.

### 11.2 Top-level placement (incl. `main` prefix, optional `as`)

A top-level placement creates a named instance of a node type at
module scope. The optional `main` prefix designates the program's
entry point (021-138).

```
TopLevelPlacement ::= Visibility? 'main'? TypeRef TopLevelName
                      FlagsRun? DefaultArgPart? WhenClause? AttrClause? BodyIntro?
                    ;  (§13.8.1, 021-6)

TopLevelName      ::= IDENT                                     // bare declaration form
                    | 'as' IDENT                                 // explicit `as` form
                    ;  (§13.8.1, 021-7)
```

// **Visibility prefix (per §10.3, 003-33).** A top-level placement
//  accepts a visibility specifier governing the cross-module
//  reachability of the instance name. The `Visibility?` head precedes
//  the optional `'main'` keyword; the order is fixed
//  (e.g., `public main Driver root_driver`).
// **Top-level name is mandatory (per 021-6); `as` is optional (per
//  021-7).** The bare form `Driver john_doe` and the explicit form
//  `Driver as john_doe` have identical meaning. By convention,
//  top-level placements omit `as`. The `as` marker is required only
//  in nested positions (§11.3).
// **`main` prefix (per 021-138 / §13.8.1).** The `main` keyword
//  prefixes a single top-level placement to mark it as the entry
//  point. Exactly one `main` placement is required per program
//  (021-139); zero is `no_entry_point`, two-or-more is
//  `multiple_entry_points` — semantic, post-parse.
// **Element order matches §11.1 with the name in the first optional
//  slot (positional).** The `TopLevelName` slot is the top-level
//  equivalent of `NameClause` in §11.1; subsequent clauses follow the
//  same fixed order as §11.1: `FlagsRun`, `DefaultArgPart` (`/Expr`),
//  `WhenClause` (`when`), `AttrClause` (`|`), `BodyIntro` (`:`).
// **Instance names are unique within the declaring module (per
//  021-8)** — semantic, post-parse.
// **Body items reuse `BlockBody` (per §13.8).** Within the
//  `BodyIntro`'s indented body (a `BlockBody`), nested placements
//  follow §11 placement productions; they do not carry their own
//  `Visibility` prefix (visibility is for declarations, not for
//  nested placement bodies).

### 11.3 Child placement (`as` required for naming)

A child placement names a node type admitted by one of the parent's
views (021-34). The child placement may be anonymous (bare
`TypeRef`); to give it a placement-time name, `as` is **required**
(021-35).

```
ChildPlacement    ::= Placement                                 // see §11.1
                    ;  (§13.8.3, 021-33)
```

// The `ChildPlacement` reduces directly to the §11.1 `Placement`
//  production — every clause of §11.1 is admissible on a child
//  placement, including the `NameClause` (where `as` is required to
//  name the child).
// **Anonymous child (per 021-36).** Bare `Pin` is a valid unnamed
//  child placement; no `as` marker, no name.
// **Named child (per 021-35).** `Pin as out1` is the named form;
//  the `as` is required because bare `Pin out1` would be ambiguous
//  between one named child and two anonymous ones.
// **Children body content (per 021-33 / §13.8.3).** A node
//  placement's `BodyIntro:` body contains zero or more child
//  placements (child nodes and connections). No attribute settings
//  appear in the body; they live on the placement's main line via
//  the attribute clause (§11.5) or aligned multi-line continuation
//  (§11.5).

### 11.4 Connection placement (body = destination reference)

A connection placement creates a directional connection from a source
instance to a destination instance (021-51). The source is the
immediately enclosing instance, determined positionally (021-52); the
destination is supplied in the placement body as a **node reference**
(021-56).

```
ConnectionPlacement ::= TypeRef FlagsRun? NameClause? DefaultArgPart?
                        WhenClause? AttrClause? ConnectionDestBody
                    ;  (§13.8.4, 021-51)

ConnectionDestBody ::= ':' INDENT NodeRef DEDENT
                    | ':' NodeRef                              // inline; NEWLINE termination follows §2.1.1 addendum
                    ;  (§13.8.6, 021-93)

NodeRef           ::= Expr                                      // see disambiguator
                    ;  (§13.8.6, 021-56)
```

// **Same inline-element surface as the generic `Placement` (per
//  §11.1).** A `ConnectionPlacement` reuses the §11.1 clause sequence
//  (`FlagsRun`, `NameClause`, `DefaultArgPart`, `WhenClause`,
//  `AttrClause`) verbatim; only the body differs — the generic
//  `Placement`'s `BodyIntro` admits child placements
//  (`SelfDelimitingPlacement+` or `BlockBody`), whereas
//  `ConnectionDestBody` holds a single node-yielding `NodeRef`
//  (021-93). The discrimination between a `Placement` body and a
//  `ConnectionDestBody` is **by placement kind**, which is post-parse —
//  the resolver classifies the `TypeRef` as a node type (then the body
//  holds children, parsed as `BodyIntro`) or a connection type (then
//  the body holds the destination, parsed as `ConnectionDestBody`).
//  The parser uses the same surface up to `':' `; the body production
//  is selected by what follows.
// **NodeRef shape (per 021-56).** A bare identifier naming an
//  instance in scope, any expression yielding a node reference
//  (possibly reactive), or a `WeakHandle[N]` (read as `Option[&N]`
//  per §13.3.6.2). The grammar admits any `Expr`; the
//  reference-yielding constraint is post-parse (021-79).
// **No child placements, attr settings, or multi-value bodies (per
//  021-93).** A connection-placement body that contains anything
//  other than exactly one `NodeRef` is a compile error — semantic,
//  post-parse.
// **`from` / `to` do not appear in connection placement bodies (per
//  021-94 / 021-95).** They are reserved as endpoint slots inside
//  connection type bodies (§9.2); using them as attr names on a
//  connection is a compile error.

### 11.5 Attribute clause `| name=value name !name` (incl. multi-line continuation)

An attribute clause follows the `TypeRef` (and optional flags,
instance name, `/expr`, and `when` modifier) on the same line,
introduced by **exactly one leading `|`** (021-97). After the leading
`|`, attributes are whitespace-separated; intermediate `|` characters
between attributes are not permitted (021-98).

```
AttrClause        ::= '|' AttrEntry ( AttrEntry )*
                    ;  (§13.8.7, 021-97)

AttrEntry         ::= IDENT '=' Expr                            // name=value
                    | IDENT                                      // boolean true (bare)
                    | '!' IDENT                                  // boolean false
                    ;  (§13.8.7, 021-99)
```

// **One leading `|` only (per 021-97 / 021-98).** A second `|` between
//  attributes is a parse error. The leading `|` is the **sole**
//  separator of the attribute clause from the elements preceding it;
//  whitespace alone separates attribute entries thereafter.
// **Multi-line continuation (per 021-11).** When the attribute clause
//  extends across multiple physical lines, the continuation lines
//  carry **no further `|`** and are aligned exactly to the column of
//  the first attribute on the placement's main line. This is a layout
//  rule managed by §2.1's layout pre-processor in conjunction with
//  the parser-side recognition of `AttrClause` continuation: a
//  continuation line at the first-attribute column with no leading
//  `|` is consumed as continued `AttrEntry`s until the column
//  changes.
//
//   Driver john_doe | expertise_level=10
//                     risk_tolerance=0.8
//                     license_class="full"
//
// **Duplicate-set is a compile error (per 021-… / §13.8.7).** Setting
//  the same attribute twice on one placement — whether via two
//  inline entries, two continuation entries, or one inline + one
//  flag (§11.6, 021-121) — is a compile error. Semantic, post-parse.
// **`name=value` admits reactive expressions (per 021-16 /
//  §13.8.2.1).** The grammar admits any `Expr` after `=`; whether the
//  RHS is a reactive binding (category C) or a value binding
//  (category B) is type-directed (post-parse) and requires no
//  syntactic marker (021-22).
// **`!name` and bare `name` set literal booleans (per 021-28).** No
//  reactive binding applies; both require the attr to be of type
//  `bool` (semantic).

### 11.6 Flag run (flag char set + `@flag('c')` declaration cross-ref)

A *flag* is a single non-letter character appearing **adjacent to**
the placed type's `TypeRef` (no intervening whitespace), aliasing a
boolean attribute (021-119). One or more flags form a **flag run**
written directly after the `TypeRef`.

```
FlagsRun          ::= FlagChar+
                    ;  (§13.8.8, 021-119)

FlagChar          ::= "'" | '!' | '?' | '*' | '+' | '^' | '~' | '@' | '$'
                    ;  (§13.8.8.1, 021-113)
```

// **Adjacency-to-TypeRef is the disambiguator (per §13.8.8.4 /
//  021-119).** A non-letter character immediately following the
//  `TypeRef` path with no intervening whitespace is a flag-run
//  opener. Any of the flag characters is admitted only at this
//  position. The same characters in non-placement positions are
//  parsed as their other-context tokens — `'` opens a `CharLit`
//  (§2.8); `?` is postfix Try / cast-policy / optional-chaining /
//  cardinality marker (§5.2, §5.3, §5.6, §8.7); `@` is the
//  directive prefix (§12); `!` is the inline attribute-false marker
//  (§11.5). The grammar resolves by position (021-120).
// **Flag character set (per 021-113 / §13.8.8.1).** Exactly the
//  nine listed characters. `#` is excluded (it is a valid identifier
//  character per §2.3, 002-15).
// **Declaration cross-reference (per 021-110 / §13.8.8).** Each
//  flag character is declared on a boolean `attr` via the
//  `@flag('c')` directive (§12.1); the `c` is a `CharLit` (§2.8)
//  giving the flag character. Per 021-115 the flag-character set
//  must be unique within a type's effective attribute surface
//  (semantic).
// **Flag run is set-true-only (per 021-117 / 021-118).** Each flag
//  in the run sets its aliased attr to `true`. There is no
//  flag form for setting `false`; the inline `!name` form (§11.5)
//  is used instead.
// **Two-mechanism duplicate (per 021-121).** A boolean attr set via
//  both a flag and an inline `name` / `!name` / `name=value` on the
//  same placement is a compile error (semantic, same diagnostic
//  class as duplicate-set).

### 11.7 `/expr` default-attr targeting (atomic vs parenthesized rule)

`/expr` is the positional argument of a placement, targeting the
placed type's `default attr` (021-73). Whitespace around the `/` is
insignificant (021-76).

```
DefaultArgExpr    ::= AtomicExpr
                    | '(' Expr ')'
                    ;  (§13.8.5, 021-75)

AtomicExpr        ::= Literal
                    | IDENT
                    | TypePath                                  // single-segment or multi-segment path
                    ;  (§13.8.5, 021-75)
```

// **Atomic-vs-parenthesized rule (per §13.8.5, 021-75 / 021-77).** An
//  unparenthesized `/expr` is **restricted to a single atom** — a
//  literal, identifier, or path (`C/4`, `Filter/cutoff_default`). A
//  compound expression must be parenthesized: `C/(base * 2)`. Without
//  the restriction, an open expression could greedily swallow the
//  next placement in same-line multi-placement (§11.11) — `C/x - G`
//  would be ambiguous between two placements and one subtraction.
//  The restriction is what keeps the placement self-delimiting (per
//  §13.8.10), not the adjacency of `/` to its operand.
// **Whitespace around `/` is insignificant (per 021-76).**
//  `Drives/0.8`, `Drives /0.8`, and `Drives / 0.8` are equivalent.
// **`/expr` requires a declared `default attr` (per 021-74).** Using
//  `/expr` on a type without one is a compile error — semantic,
//  post-parse.
// **Connection placements use `/expr` for `default attr`; the
//  destination remains in the body (per 021-78 / §13.8.5.1).**
//  Neither `/expr` nor the attribute clause targets the destination.

### 11.8 Inline `when` modifier

An inline `when` clause gates the placement (021-124). It sits in the
fixed `WhenClause` slot of §11.1 — after the optional `/Expr` and
before the `AttrClause` / `BodyIntro`.

```
// WhenClause defined in §11.1 (`'when' Expr`); not redefined here.
// (See §11.1 for the production block embedding `WhenClause`.)
```

// **Predicate is a boolean expression in placement scope (per
//  021-124).** The grammar admits any `Expr`; the `bool`-typedness is
//  post-parse.
// **Predicate evaluation scope is the enclosing source instance for
//  connection placements (per 021-60), not the connection's own
//  scope.** To gate on the connection's own attrs, use a type-level
//  `when:` clause inside the connection type's body (per 021-61).
//  Both rules are semantic.
// **Unparenthesized `:` in the predicate must be parenthesized (per
//  021-127).** A predicate containing a bare `:` collides with the
//  body-introducer `:`; parenthesize the predicate to disambiguate.
// **Same-line multi-placement requires parenthesization (per
//  021-133).** A placement carrying an inline `when` predicate
//  contains an open expression and is **not self-delimiting**; if it
//  shares a line with sibling placements it must be parenthesized
//  (see §11.11).

### 11.9 `repeat … as <view>` (keyed view binding)

The `repeat` construct may carry an `as <view>` clause that hoists
its named placements out of the per-iteration body and binds them
collectively to `<view>` (§13.5.4.9). The placement-level clause
order is fixed (§13.5.4.1).

```
RepeatPlacement   ::= 'repeat' RepeatBind RepeatIndex? 'in' Expr RepeatViewName? RepeatKeyed? ':'
                      ( INDENT BlockBody DEDENT
                      | InlineBody )                            // per §13.8.3, 021-137: inline same-line children admissible
                    ;  (§13.5.4.1, 018-36)

RepeatBind        ::= IDENT
                    | TuplePattern                              // see §4.3
                    ;  (§13.5.4.1, 018-36)

RepeatIndex       ::= 'at' IDENT
                    ;  (§13.5.4.1, 018-36)

RepeatViewName    ::= 'as' IDENT
                    ;  (§13.5.4.9, 018-130)

RepeatKeyed       ::= 'keyed' 'by' Expr
                    ;  (§13.5.4.1, 018-36)
```

// **Clause order (per §13.5.4.1).** `<bind>`, then optional
//  `at <index>`, then `in <source>`, then optional `as <view>`, then
//  optional `keyed by <key-expr>`. The fixed order is normative.
// **`as <view>` binds a compiler-minted `cell Map[Key, <view>::entry]`
//  in the body-scope namespace (per §13.5.4.9).** The `<view>::entry`
//  type is synthetic, path-derived, with one field per named
//  (`as <name>`) placement inside the repeat body. Field types are
//  `WeakHandle[T]` (per §13.5.4.9, since repeat-keyed scopes can
//  dismount between commits).
// **`as`-names inside the body must be unique within the `repeat`
//  body across nesting (semantic, §13.5.4.9).** Anonymous placements
//  inside the body are unaddressable through `<view>` (post-parse).
// **`RepeatBind` accepts a tuple-destructuring pattern (per
//  §13.5.4.1 / §12.12.1).** The same destructuring grammar as the
//  for-loop iteration variable (`Pattern`, §4); the bind is
//  *move-promoted* (semantic, §13.5.4.1) — a grammar-irrelevant
//  ownership transformation.

### 11.10 `for … as <name>` (static view binding)

A compile-time `for` in a node body or placement body may carry an
`as <name>` clause that hoists its loop-scoped named placements and
binds them collectively to `<name>` in the enclosing scope (§13.3.3.3).

```
ForAsPlacement    ::= 'for' Pattern 'in' Expr ForAsName? ':' INDENT BlockBody DEDENT
                    ;  (§13.3.3.3, 017-68)

ForAsName         ::= 'as' IDENT
                    ;  (§13.3.3.3, 017-68)

// Iteration variable is a `Pattern` (per SPEC §12.12.1, 014-150..152);
//  irrefutability is a post-parse check.
```

// **Clause order (per §13.3.3.3).** The `as <name>` slot follows the
//  `in <iterable>`, in the same position as `RepeatViewName` follows
//  `in <source>` in `RepeatPlacement` (§11.9). The clause is optional;
//  when omitted, the `for` is the unmarked compile-time-`for` of
//  §6.2 / §12.3.7.
// **`<name>` binds `[<name>::entry; N]`** — a fixed-extent array of
//  compiler-minted `<name>::entry` records (per §13.3.3.3). Field
//  types are `Handle[T]` (the statically-placed type form), since the
//  for-loop is compile-time unrolled and every iteration's placement
//  is statically placed. The `entry` record's fields are named after
//  the `as <name>` placements inside the loop body.
// **Iterable must be compile-time-known (semantic, §13.3.3.3 /
//  §12.3.7).** A runtime iterable is a compile error pointing at
//  the iterable, enforced by §13.1's static-graph rule.
// **`for … as` is the static counterpart to `repeat … as` (per
//  §13.3.3.3 closing paragraph).** Array vs map, positional vs
//  keyed, static vs Cell-dynamic — the entry-field type tracks the
//  difference (`Handle[T]` here vs `WeakHandle[T]` there).
// **Same-line multi-placement.** A `for … as` (like `for` generally)
//  introduces an indented body via `:` and therefore owns the rest
//  of its line per §11.11 / 021-136.

### 11.11 Same-line multi-placement self-delimiting rule

Multiple placements may appear on a single line, separated by
**whitespace** — no comma separator, no semicolon (021-128). Per
021-129, each placement on a shared line must be **self-delimiting**:
its end must be determinable without lookahead into the next
placement.

```
SameLinePlacement ::= SelfDelimitingPlacement ( SelfDelimitingPlacement )+
                    ;  (§13.8.10, 021-128)

SelfDelimitingPlacement ::= BareTypeRef FlagsRun? NameClause? ( '/' AtomicExpr )?
                          | '(' Placement ')'                          // 021-133, 021-135
                          ;  (§13.8.10, 021-129)

// The first alternative subsumes the three forms enumerated in
// 021-130 (`C`/`G'`), 021-131 (`/expr`), and 021-132 (`as Name`) into
// one production. The `FlagsRun?`, `NameClause?`, and `/AtomicExpr?`
// are each independently optional. The trailing alternative is the
// parenthesised escape for any non-self-delimiting placement.

BareTypeRef       ::= TypeRef                                    // see §11.1
                    ;  (§13.8.10, 021-130)
```

// **Self-delimiting forms (per 021-130 / 021-131 / 021-132).**
//  - A bare type, including any flag run: `C`, `G'`, `Pin'!` (021-130).
//  - A single-atom `/expr`: `C/4`, `Filter/cutoff_default` (021-131).
//  - An `as` name, which consumes exactly one identifier:
//    `G as a  rest  C` (021-132).
// **Not self-delimiting — must be parenthesized or on its own line
//  (per 021-133).** A placement carrying an **open expression** —
//  a `when` predicate, an attribute clause (`|`), or a compound
//  (non-atomic) `/expr` — has an unbounded right edge. Such a
//  placement must be parenthesized to share its line with siblings,
//  or written on its own line.
//
//    (Sensor as s1 | gain=0.5) (Sensor as s2 | gain=0.7)   ✓
//    Sensor as s1 | gain=0.5                               ✓ (own line)
//
// **No diagnostic mandated for the silent mis-parse (per 021-134).**
//  An unparenthesized open-expression placement that silently
//  reparses as a different construct under the greedy grammar is the
//  user's responsibility to disambiguate; the surface diagnostic comes
//  from whatever downstream type / arity error the misread form
//  produces. The parenthesize-or-newline rule above is the prevention.
// **`as` naming never *requires* parens (per 021-135).** Naming with
//  `as` is parser-safe unparenthesized (it binds exactly one
//  identifier); parenthesizing a named placement is a readability
//  convention only.
// **`:`-bearing placement owns its line (per 021-136 / 021-137).** A
//  placement that introduces its own children body via `:` cannot
//  share its line with sibling placements; the body owns the rest of
//  the line and the indented block that follows. A `:`-bearing
//  placement *may* carry **inline children** on its own line —
//  `SomePart: Child1 Child2 Child3` — as long as no sibling placement
//  shares the line (021-137). To combine `:`-bearing placements with
//  same-line siblings, use multi-line layout.

## 12. Directives

A *directive* is introduced by the `@` sigil and is drawn from a fixed,
language-provided set; there are no user-defined directives (§1.4). A
directive is either **applied** — attached to a declaration to modify it —
or **standalone** — a construct in its own right. In placement position,
`@` is instead a flag-run character (§11.6, §13.8.8.4); the
disambiguation is positional.

```
Directive         ::= AppliedDirective
                    | StandaloneDirective
                    ;  (§1.4, 002-13)

AppliedDirective  ::= '@' DirectiveName ( '(' DirectiveArgs? ')' )?
                    ;  (§1.4, 002-14)

DirectiveName     ::= 'derive'
                    | 'literal_suffix'
                    | 'flag'
                    | 'default'
                    | 'reset_on_reopen'
                    | 'reset_on_reload'
                    ;  (§1.4, 002-14)

DirectiveArgs     ::= DirectiveArg ( ',' DirectiveArg )*
                    ;  (§1.4, 002-14)

DirectiveArg      ::= TypePath                                  // @derive operands
                    | TypeExpr                                  // @default's operand admits intersections (§12.1)
                    | StringLit ',' IDENT                       // @literal_suffix
                    | CharLit                                   // @flag
                    ;  (§1.4, 002-14)

StandaloneDirective ::= '@' 'content'
                    ;  (§13.3.7.2, 017-228)
```

// **Directive set is closed (per §1.4).** The six applied names
//  (`derive`, `literal_suffix`, `flag`, `default`, `reset_on_reopen`,
//  `reset_on_reload`) plus the one standalone name (`content`) form
//  the complete inventory. Any other identifier following `@` in
//  directive position is a parse error. Users cannot register new
//  directives.
// **Directive vs flag-run disambiguation (per §13.8.8.4).** `@` opens
//  a directive only in declaration / annotation context. In placement
//  context — immediately adjacent to a `TypeRef` path with no
//  intervening whitespace (§11.6) — `@` is a flag character. The
//  parser resolves by position.

### 12.1 Applied directives

Applied directives sit on their own line directly above the
declaration they modify (§12.3). The applied set, with declaration
sites:

```
Annotation            ::= AppliedDirective
                        ;  (§3.1, 002-13)
// Alias used by `TraitBodyItem`, `FulfillBodyItem`, `NodeDecl`,
//  `ConnectionDecl`, and other productions mirroring SPEC BNF that
//  spells the slot `Annotation`.

DeriveDirective       ::= '@' 'derive' '(' TypePath ( ',' TypePath )* ')'
                        ;  (§3.8, 005-188)

LiteralSuffixDirective ::= '@' 'literal_suffix' '(' StringLit ',' IDENT ')'
                        ;  (§3.9, 005-209)

FlagDirective         ::= '@' 'flag' '(' CharLit ')'
                        ;  (§13.8.8, 021-110)

DefaultDirective      ::= '@' 'default' '(' TypeExpr ')'
                        ;  (§3.1.5, 005-32)

ResetOnReopenDirective ::= '@' 'reset_on_reopen'
                        ;  (§13.9.7, 022-66)

ResetOnReloadDirective ::= '@' 'reset_on_reload'
                        ;  (§13.18.14, 030-46)
```

// **`@derive` attaches to type declarations (per §3.8).** Applied
//  above a `type` / `enum` / newtype declaration; the `TypePath`
//  arguments are the traits to derive. The derivable trait set is
//  fixed (§3.8.1) and the check is semantic (post-parse).
// **`@literal_suffix` attaches to type declarations (per §3.9).** The
//  `StringLit` is the suffix name (an identifier per §3.9.1, but
//  given as a string literal); the `IDENT` is the unqualified name
//  of the constructor function in scope.
// **`@flag` attaches to boolean `attr` declarations (per §13.8.8).**
//  The `CharLit` carries the flag character (drawn from `FlagChar`,
//  §11.6 / §13.8.8.1). Non-boolean attrs carrying `@flag` is a
//  compile error — semantic, post-parse.
// **`@default(T)` attaches to `trait` declarations (per §3.1.5).**
//  Declares the trait's default concrete type for the defaulting
//  mechanism (§3.6.2). The argument is a `TypeExpr` that itself
//  satisfies the trait (semantic, post-parse).
// **`@reset_on_reopen` attaches to `recurrent` declarations (per
//  §13.2.4).** Zero-arg form; opts the recurrent into discarding its
//  self-history on gate false→true reactivation (§13.9.7). Semantic
//  effect is post-parse.
// **`@reset_on_reload` attaches to `stream` declarations (per
//  §13.18.14).** Zero-arg form; opts the stream out of cross-reload
//  buffer preservation. Semantic effect is post-parse.

### 12.2 Standalone directive `@content`

`@content` is the sole standalone directive and appears **only as an
entry inside an `expose:` clause** (§13.3.7.2). It uses the single
canonical `StandaloneDirective` production (§12 head). The
`ExposeEntry` alternative cross-references this directive via
`StandaloneDirective` rather than a separately-named production.

```
// ContentDirective: alias for StandaloneDirective per §12 head; no
//  separate production. ExposeEntry references StandaloneDirective.
//                                                  ;  (§13.3.7.2, 017-228)
```

// **Exposure-context-only (per §13.3.7.2).** `@content` is admitted
//  only as an `expose:` entry — including as the body of a wrapper
//  placement entry inside `expose:`. Outside that context it is a
//  parse error.
// **At most one per `expose:` scope (per §13.3.7.2).** Multiple
//  `@content` entries in the same `expose:` (or in nested wrapper
//  bodies sharing one scope) is a compile error — semantic, post-parse.
// **No expression access (per §13.3.7.2).** `@content` is not a
//  named declaration; it produces no binding, has no `.` form, and
//  is not referenced by name elsewhere in the grammar.

### 12.3 Line-attachment rule

An applied directive sits on its **own line, directly above** the
declaration it modifies, aligned to the declaration's indentation
column. Multiple applied directives stack vertically, each on its own
line, in source order, all above the declaration.

```
AnnotatedDecl     ::= AppliedDirective+ NEWLINE Decl
                    | Decl
                    ;  (§1.4, 002-14)

Decl              ::= TopLevelDecl
                    | NodeBodyDecl
                    | ConnectionBodyDecl
                    | EffectBodyDecl                          // wrapped via AnnotatedDesiredCellDecl / AnnotatedObservedCellDecl (§7.14)
                    | TraitBodyItem                           // §7.11
                    | FulfillBodyItem                         // §7.12
                    ;  (§1.4, 002-14)

TopLevelDecl      ::= FnDecl
                    | RecordDecl
                    | EnumDecl
                    | NewtypeDecl
                    | AliasTypeDecl
                    | TraitDecl
                    | FulfillItem
                    | OperatorDecl
                    | EffectDecl
                    | TopLevelConstDecl
                    | UseStmt
                    | NodeDecl
                    | ConnectionDecl
                    | ModuleReactiveDecl                      // signal / derived / recurrent / stream at module scope
                    | TopLevelPlacement
                    ;  (§1.4, 002-14)

NodeBodyDecl      ::= AttrDecl                                // §8.9
                    | DefaultAttrDecl                         // §8.9
                    | DerivedDecl                             // §8.9 / §7.15
                    | RecurrentDecl                           // §8.9 / §7.15
                    | StreamDecl                              // §8.9 / §7.15
                    | TopLevelConstDecl                       // §8.9 admits module-style const inside node body
                    ;  (§1.4, 002-14)

ConnectionBodyDecl ::= AttrDecl                               // §9 inherits §8.9 cell shapes
                    | DefaultAttrDecl
                    | DerivedDecl
                    | RecurrentDecl
                    | StreamDecl
                    ;  (§1.4, 002-14)

EffectBodyDecl    ::= DesiredCellDecl                         // §7.14 (wrapped form is AnnotatedDesiredCellDecl)
                    | ObservedCellDecl                        // §7.14 (wrapped form is AnnotatedObservedCellDecl)
                    ;  (§1.4, 002-14)
```

// **Wrapper-only attachment site (per Phase D, D4).** `AnnotatedDecl`
//  is the *sole* directive-attachment production. Individual decl
//  productions (`FnDecl`, `RecordDecl`, `NodeDecl`, `ConnectionDecl`,
//  `TraitDecl`, etc.) do **not** carry an inline `Annotation*` head —
//  the directive-decoration parse path goes through this wrapper
//  uniformly. Where a decl form is reachable only inside a body (e.g.
//  `DesiredCellDecl` inside an effect's `desired:` block), the local
//  wrapper production (e.g. `AnnotatedDesiredCellDecl` of §7.14)
//  attaches the same `Annotation*` run in that scope.
// **`Decl` enumerates the directive-decoratable declaration heads.**
//  `AnnotatedDecl` attaches zero or more `AppliedDirective`s to any
//  `Decl`. `Decl` is the meta-name for the union; the resolver applies
//  each directive's own attach-site constraint (e.g., `@derive` on
//  type-like declarations, `@flag` on boolean `attr`s) post-parse.
// **`ConstStmt` is body-only.** Module-level const uses
//  `TopLevelConstDecl` (which admits `Visibility?`). `ConstStmt`
//  (§6.1) is reachable inside a `BlockBody` (statements within a
//  function / operator / block body) and is *not* a `Decl`
//  alternative; it carries no visibility prefix and is not
//  directive-decoratable.

// **Own-line placement (per §1.4 / §3.8 / §3.9 / §13.8.8).** A
//  directive may not share its line with the declaration it modifies
//  (no `@derive(Eq) type Point:` form). The line terminator after the
//  directive line is structural; it does not introduce an indent.
// **Stacking order is source order (per §3.8.1 / §3.9.1).** Multiple
//  `@derive(...)` directives may stack; multiple `@literal_suffix(...)`
//  directives may stack — each on its own line above the type. The
//  semantic interpretation is per directive (e.g., union of derived
//  traits, set of registered suffixes).
// **Column alignment (per §1.4 layout rules).** The directive line is
//  written at the declaration's own indentation column, not deeper.
//  Continuation across multiple physical lines uses ordinary
//  paren/bracket layout-suspension (§2.1) within the directive's
//  argument list.

## 13. Reserved identifiers and namespaces

This section documents identifier names with special grammatical or
contextual meaning. None of these names may be (re-)declared as
user-defined identifiers (§1.4). They appear in the grammar wherever
their respective constructs are referenced; this section gathers them
in one place for the reader.

### 13.1 `Subject`

`Subject` is the **sole reserved capitalized type identifier** (002-12).
It names the *type-level alias* for the implementing/subject type and
is usable only in type positions inside `trait` declarations and
`fulfill` blocks (§3.1.1, §13.7.7).

```
SubjectTypeRef    ::= 'Subject'
                    ;  (§13.7.7, 002-12)
```

// **Type-position only (per §13.7.7).** `Subject` is a type alias, not
//  a value. It appears wherever the grammar admits a `TypeExpr` inside
//  a `TraitDecl` body (§7.11) or a `FulfillItem` body (§7.12), naming
//  the subject type of the current trait/impl. Using it in value
//  position is a parse error.
// **Capitalized by the type-naming convention (per 002-12).** Because
//  it is a type alias rather than a keyword, it follows the
//  capitalize-types convention; it does not fall under the
//  lowercase-keyword rule of §2.4.
// **No user redefinition (per §1.4).** A user declaration introducing
//  a type named `Subject` is a parse/declaration error.

### 13.2 `subject`

`subject` is the **instance value**, available in expression position
inside a node or connection body. It denotes the whole instance
currently being declared, suitable for passing to a function that
takes the instance type (`total_output(subject)`).

```
SubjectValueRef   ::= 'subject'
                    ;  (§13.7.7, 002-8)
```

// **Value position only (per §13.7.7).** `subject` is a value, not a
//  namespace; it has no `::` form. It appears wherever the grammar
//  admits an `IDENT` in expression position, but only inside a node
//  or connection body (semantic check, post-parse).
// **No implicit receiver (per §13.7.7).** `some_method(subject)` and
//  `subject.some_method()` are the same call written two ways; the
//  dot is sugar, not a receiver binding. The grammar does not
//  special-case `subject.` — it parses as ordinary member access on a
//  value.
// **No user redefinition (per §1.4).** A user declaration introducing
//  a binding named `subject` is a parse/declaration error.

### 13.3 `here::` and `module::` namespace anchors

`here` and `module` are reserved **namespace** identifiers usable
**only** as the left side of `::` (§13.7.2, §13.7.3). They name *which
scope* to resolve a name in.

```
NamespaceAnchorPath ::= 'here'   '::' AnchorSuffix
                      | 'module' '::' AnchorSuffix
                      ;  (§13.7.2, 002-7)

AnchorSuffix      ::= IDENT
                    | ReservedInstanceField                    // reserved instance-field names per §13.7.5 (see §13.4)
                    ;  (§13.7.5, 002-5)
```

// **`::` form only (per §13.7.7).** `here` and `module` are
//  namespaces, not values; they have no `.` form and may not appear
//  bare. `here::x` and `module::x` are the only admissible forms;
//  the `PathSegment` admission of `'here'`/`'module'` in §5.1 covers
//  *only* the leading segment of a `Path` whose head is immediately
//  followed by `::`. A bare `here` / `module` Path (no `::` tail) is
//  a parse error per the §13.7.2 / §13.7.3 single-suffix rule.
// **Single-suffix rule (per Phase D, D7; §13.7.2 / §13.7.3).**
//  `NamespaceAnchorPath` takes exactly one `AnchorSuffix` segment —
//  not a chain. `here::SomeMod::x` is not admitted; reach `SomeMod`
//  via the anchor's single resolved name and then continue with
//  ordinary path navigation through that bound identifier if needed.
// **Scope semantics (per §13.7.2 / §13.7.3).** `here::x` resolves in
//  the current (innermost) scope — inside a node/connection body, the
//  instance body scope. `module::x` resolves in the enclosing
//  module's top-level scope. Both bypass collision-disambiguation
//  rules of §13.7.4 (semantic, post-parse).
// **No user redefinition (per §1.4).** A user declaration introducing
//  a binding named `here` or `module` is a parse/declaration error.

### 13.4 Contextual instance-field reserved names

The names **`pair`**, **`exposition`**, **`from`**, **`to`**,
**`desired`**, and **`observed`** are reserved (002-5, §13.7.5). Each
serves two co-existing grammatical roles, distinguished by syntactic
position:

| Name         | Declaration-position role                            | Expression-position role                                 | Source       |
|--------------|------------------------------------------------------|----------------------------------------------------------|--------------|
| `from`       | endpoint clause head on connection types (§9.2)      | instance field of a connection (the `from` endpoint)     | §13.6, §13.7.5 |
| `to`         | endpoint clause head on connection types (§9.2)      | instance field of a connection (the `to` endpoint)       | §13.6, §13.7.5 |
| `pair`       | pairs-form body matcher head (§9.4)                  | instance field on pairs-form connection instances        | §13.6.1.3, §13.7.5 |
| `exposition` | (no declaration-position role)                       | instance field of any node — the exposed list (§13.3.7.3) | §13.3.7.3, §13.7.5 |
| `desired`    | effect clause head (`desired:` sub-block, §7.14)     | (sub-block name; not an expression-position reference)   | §13.19, 002-5 |
| `observed`   | effect clause head (`observed:` sub-block, §7.14)    | (sub-block name; not an expression-position reference)   | §13.19, 002-5 |

```
ReservedInstanceField ::= 'pair' | 'exposition' | 'from' | 'to'
                        | 'desired' | 'observed'
                        ;  (§13.7.5, 002-5, 002-28)
```

// **Positional disambiguation (per §13.7.5).** In statement / clause
//  position (`from:`, `to:`, `pairs:` then `match pair:`, `desired:`,
//  `observed:`) the name is the clause/keyword. In expression-operand
//  position (`from.expertise_level`, `to.top_speed`, `pair.x`,
//  `instance.exposition`) the same name is the instance field. The
//  parser resolves by syntactic position; no collision with user
//  names is possible (per 002-5 / §1.4 — these are reserved and
//  cannot be declared).
// **Six-of-six per 002-28.** `pair`, `exposition`, `from`, `to`,
//  `desired`, `observed` are all reserved as field-like names per
//  LOG 002-28. `desired` and `observed` arise principally as clause
//  heads inside `effect` bodies (§7.14); reserving them as instance
//  fields keeps the spellings unavailable for user-declared cells
//  elsewhere in the instance namespace (per 002-5 / §1.4).
// **`here::`-anchored equivalents are admissible (per §13.7.5).**
//  `here::from`, `here::to`, `here::pair`, `here::exposition` resolve
//  the same instance fields explicitly.
// **`exposition` is read-only (per §13.3.7.3).** Mutation is not
//  syntactically distinguished here; the field's read-only nature is
//  a semantic rule (post-parse).
// **`incoming` / `outgoing` are not in this set (per 002-5 /
//  §13.3.4).** They are clause heads on node bodies, not instance
//  fields — they do not appear as expression-position references and
//  are documented with `Node` declarations (§8.4) rather than here.
// **`in` is not in this set (per §13.7.5).** `in` is a `for`-loop
//  separator and the `Contains` membership operator; it does not
//  carry an instance-field role.

## Appendix A. Operator precedence table

The precedence table below derives from SPEC §4.4.7, with grammar-
specific extensions: tiers **0a (`with`)** and **0b (`where`)** are
the looser-than-`|>` postfix-update and stream-filter binders
(SPEC §6.1.5 / §13.18.10; LOG 030-168 / 030-169); the prefix tier
13 enumerates `dyn` and `move` alongside the SPEC-listed prefixes;
and tier 14 enumerates the bare `T(x)` call/cast surface. Operators
are listed loosest-binding (top) to tightest-binding (bottom);
operators on one row share precedence. Annotations on the right
map each tier to the production(s) in §5 that realize it.

| Tier | Operators                                                       | Associativity   | Realized by                                  |
|------|-----------------------------------------------------------------|-----------------|----------------------------------------------|
| 0a   | `with` (postfix update)                                         | left            | `WithExpr`/`WithSuffix` (§5.13)              |
| 0b   | `where` (stream-filter binary)                                  | left            | `WhereFilterExpr` (§5.23, 030-168/030-169)   |
| 1    | `\|>` (operator / effect application)                           | left            | `PipeExpr` (§5.25)                           |
| 2    | `or`                                                            | left            | `OrExpr` (§5.7)                              |
| 3    | `and`                                                           | left            | `AndExpr` (§5.7)                             |
| 4    | `not` (prefix)                                                  | right           | `NotPrefix` of `PrefixOp` (§5.4)             |
| 5    | `\|` (bitwise or)                                               | left            | `BitOrExpr` (§5.7)                           |
| 6    | `^` (bitwise xor)                                               | left            | `BitXorExpr` (§5.7)                          |
| 7    | `&` (bitwise and)                                               | left            | `BitAndExpr` (§5.7)                          |
| 8    | `..` (range)                                                    | non-associative | `RangeExprTier` (§5.7)                       |
| 9    | `is`, `is not`, `<`, `<=`, `>`, `>=`                            | non-associative | `CompareExpr` (§5.7)                         |
| 10   | `<<`, `>>` (shifts)                                             | left            | `ShiftExpr` (§5.7)                           |
| 11   | `+`, `-`                                                        | left            | `AdditiveExpr` (§5.7)                        |
| 12   | `*`, `/`, `\`, `%`                                              | left            | `MultiplicativeExpr` (§5.7)                  |
| 13   | `-`, `~`, `handle`, `handle!`, `portal`, `dyn`, `move` (prefix) | right           | `UnaryExpr` / `PrefixOp` (§5.4)              |
| 14   | `?`, `.`, `[]`, `()`, `T(x)`, and `T%()`/`T\|()`/`T?()` casts   | left            | `PostfixExpr` (§5.2), `CastPolicySuffix` (§5.6) |
| 15   | `::`                                                            | left            | `Path` (§5.1)                                |

Notes (verbatim from §4.4.7):

- `|>` is the loosest-binding operator; every other operator binds tighter, so `a + b |> op` is `(a + b) |> op`.
- Bitwise operators bind tighter than the logical operators (`and`/`or`/`not`) but looser than comparison — the C convention — so `a & b is c` parses as `a & (b is c)`; parenthesize when the other grouping is meant.
- `not` binds looser than comparison and negates the whole comparison: `not a is b` is `not (a is b)`.
- `..` binds looser than arithmetic, so `0..n + 1` is `0..(n + 1)`.
- Comparison does not chain: `a < b < c` is rejected (§4.4.3).
- Each arithmetic policy variant — wrapping `…%`, saturating `…|`, checked `…?` — binds at its base operator's tier: `+%`/`+|`/`+?` are additive, `*%`/`*|`/`*?`/`\?`/`%?` multiplicative, and unary `-%`/`-|`/`-?` prefix.
- `as` is **not** in this table: it is a naming keyword, not a value operator (§4.7); explicit conversion uses the call forms `T()`/`T%()`/`T|()`/`T?()`, which bind at the postfix tier.
- The cast-policy forms `u8%(x)`/`u8|(x)`/`u8?(x)` are call-like (the `(` disambiguates, §4.7.1), binding at the postfix tier, not as infix operators.
- Type-level `&` (intersection, §5.1) and `dyn` binding (§5.2.1) are governed separately from this value-expression table.

## Appendix B. Language-provided type vocabulary

The names below denote **language-provided types**. They have **no
special grammar productions**: every occurrence parses as an ordinary
generic instantiation `TypePath '[' GenericArgs ']'` per §3.2. The
parser does not special-case them; they participate in the same
`TypeExpr` productions as user-defined generic types. Their semantics
— what they mean, how they behave at runtime, what operations they
admit — are defined in SPEC.

### B.1 Stream policy types

| Type      | One-line description                                                                                                          | SPEC §   |
|-----------|------------------------------------------------------------------------------------------------------------------------------|----------|
| `Ring[N]` | Language-provided const-generic marker type fulfilling the sealed trait `StreamPolicy`: bounded ring-buffer policy of capacity `N`. | §13.18.3 |
| `Gate[N]` | Language-provided const-generic marker type fulfilling the sealed trait `StreamPolicy`: bounded gate-buffer policy of capacity `N`. | §13.18.3 |

`StreamPolicy` is a **sealed trait**, not a type — it names no row
here; the only two types fulfilling it are the marker types above
(§3.7.6, §13.18.3). `Ring[N]` and `Gate[N]` parse as ordinary generic
instantiations `TypePath '[' GenericArgs ']'` per §3.2; the parser does
not special-case them.

There are no stream bracket types and no stream alias types: a stream
is wiring, not a value. The stream annotation is the wiring type
`stream[P] T` (a `KindAnnotation`, §3.15) with word-form sugar
`stream ring[N] T` / `stream gate[N] T` for `stream[Ring[N]] T` /
`stream[Gate[N]] T`. A recurrent stream carries the orthogonal
history-depth axis (`recurrent[H] stream …`), which is not a policy and
mints no type of its own (§13.18.3).

**No reactive-cell or reactive-stream *types* — the binding forms are
lowercase KINDS.** There is no `Cell[T]`, `Signal[T]`, `Derived[T]`, or
`Recurrent[T, N]` type, and no bracket stream type of any spelling
(single-argument or policy-carrying). The reactive binding forms are
lowercase **kinds** written in annotation position — see the
`KindAnnotation` production (§3.15). Two levels: the keyword alone
(`cell`, `stream`, `signal`, …) is a **kind**; the applied annotation
is a **wiring type** — a type-system member, unstorable, never a value
type (§13.2.8.1). The wiring types are the value-cell umbrella `cell T`
(spanning the value cells `signal T`, `attr`-as-`signal T`, `derived
T`, `recurrent[N] T`), the stream kind class (erased `stream T`, the
policy-generic `stream[P] T`, its word-form sugar `stream ring[N] T` /
`stream gate[N] T`, and the history-bearing `recurrent[N] stream …`),
the group kind class (`yielded T`), and `dynamic view T` (§13.3.3.4). A
wiring type never appears inside a value-type constructor; the sole
exception is `Portal[cell T]`, whose bracket carries a cell
designation, not a nested value type (§13.2.8, 016-180).

### B.2 Graph references

| Type            | One-line description                                                                          | SPEC §       |
|-----------------|-----------------------------------------------------------------------------------------------|--------------|
| `Handle[T]`     | Statically-placed graph-entity reference; storable in cells (the lexer-merged `handle!` form). | §13.3.6.2   |
| `WeakHandle[T]` | Dynamically-placed graph-entity reference (the `handle` prefix form).                          | §13.3.6.2   |
| `Portal[T]`     | Non-graph slot reference (the `portal` prefix form).                                          | §13.3.6.3   |

### B.3 Iteration

| Type             | One-line description                                                                          | SPEC §       |
|------------------|-----------------------------------------------------------------------------------------------|--------------|
| `DynamicView[T]` | A runtime-varying view of zero-or-more `T`-typed children selected by a dynamic predicate.    | §13.3.3.4   |

### B.4 Compound

| Type        | One-line description                                                                          | SPEC §       |
|-------------|-----------------------------------------------------------------------------------------------|--------------|
| `Bundle[T]` | An ordered collection of `T`-typed children placed and accepted as a single 2-D row group.    | §13.3.3.5   |

### B.5 Meta

| Type      | One-line description                                                                          | SPEC §       |
|-----------|-----------------------------------------------------------------------------------------------|--------------|
| `Type[T]` | The compile-time meta-type carrying the type `T` as a value.                                  | §5.7        |

### B.6 Stdlib collections

| Type           | One-line description                                                                          | SPEC §       |
|----------------|-----------------------------------------------------------------------------------------------|--------------|
| `Vec[T]`       | Growable ordered sequence of `T`.                                                             | §9.6        |
| `Map[K, V]`    | Keyed associative container.                                                                  | §9.5        |
| `HashSet[T]`   | Unordered set of unique `T`.                                                                  | §9.6        |
| `Option[T]`    | Optional value (`Some(T)` / `None`).                                                          | §8.3        |
| `Result[T, E]` | Fallible result (`Ok(T)` / `Err(E)`).                                                         | §8.3        |
| `Range[T]`     | A range of `T` values (the construct produced by `a..b`, §5.8).                              | §12.2       |

**Reminder.** None of these names is special-cased by the grammar.
`Vec[i32]`, `Handle[Driver]`, `Ring[64]`, `Option[T]`, and a
user-defined `MyContainer[i32]` all parse identically: a `TypePath`
followed by `'[' GenericArgs ']'` (§3.2). The grammar admits these as
generic instantiations of identifiers in scope; whether they resolve
to a language-provided type or a user-defined one is a name-resolution
concern (semantic, post-parse).

## Appendix C. Disambiguator catalog

Index of context-sensitive rules referenced from individual production
blocks. Each entry summarises the rule and points to the section that
states it normatively.

| Case                                                  | Rule (short form)                                                                                                              | Defined in                  |
|-------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------|-----------------------------|
| `T[args]` — array vs generic instantiation            | Resolved by *kind* of `T`: a primitive scalar type bracket → array; an identifier denoting a generic → instantiation.            | §3.2 / §9.3.2               |
| `is` / `not` — greedy `is not` compound               | In infix-completion position (after a `CompareExpr` operand expecting a comparator), the two-token sequence `is not` is parsed as one operator. | §5.7 / 007-197 / §4.4.4 |
| `@` — directive prefix vs flag character              | In declaration/annotation context, `@` opens a directive (§12). Adjacent to a `TypeRef` in placement position (no whitespace), `@` is a flag char (§11.6). | §13.8.8.4 / §12          |
| `'` — char literal vs flag character vs identifier trailer | In expression context, `'` opens a `CharLit` (§2.8). In placement context, adjacent to a `TypeRef`, it is a flag char (§11.6). Otherwise, it is forbidden in identifiers (§2.3 admits only `#` as the non-letter identifier char). | §2.8 / §11.6 / §13.8.8.4 |
| `?` — Try postfix / flag / cast policy / optional chaining | In expression context, postfix `?` is Try (§5.2). After a `TypeRef` it forms the checked cast `T?(x)` (§5.6). After `.`/`[`/`(` it is optional chaining (§5.3). In placement context adjacent to `TypeRef`, it is a flag char (§11.6). | §5.2 / §5.3 / §5.6 / §11.6 |
| `!` — inline attribute-false / `handle!` lexer-merged / flag | In an attribute clause, leading `!name` is the attribute-false form (§11.5). The two-character sequence `handle!` is a single lexer token (§2.4). In placement context adjacent to `TypeRef`, `!` is a flag char (§11.6). | §11.5 / §2.4 / §11.6     |
| `(...)` — tuple vs parenthesized expr vs call args    | Disambiguated by surrounding production: in `Expr` position, `(...)` is grouping (one element) or tuple (zero or ≥2 elements; or a 1-tuple if trailing comma — §5.16). After a callable, `(args)` is a call-arg list (§5.5). In type position, see §3.5. | §5.16 / §5.5 / §3.5     |
| `[...]` — array literal / slice / generic args / map literal | Disambiguated by surrounding production: in `Expr` position, `[...]` is array literal or comprehension (§5.14); after an indexable, it is `[idx]` postfix (§5.2) or slicing (§5.10); after a `TypeRef`, it is the generic-args list (§3.2). Map literals use `{...}` (next row), not `[...]`. | §5.14 / §5.10 / §3.2     |
| `{...}` — map literal / interpolation expr / block     | In `Expr` position, `{...}` is a map literal (§5.15) when its element list is non-empty key-value pairs, or a `Block` when its body is a sequence of statements/expression (§5.17). Inside a string-literal lexer mode (§2.9), `{expr}` is the interpolation form. Record construction does **not** use braces. | §5.15 / §5.17 / §2.9     |
| Same-line multi-placement                              | A placement is self-delimiting when every clause is bounded (bare type / flags / `as` name / atomic `/expr`). Open expressions (`when`, `\|`, compound `/expr`) require parens or own line. A `:`-bearing placement owns its line. | §11.11 / §13.8.10        |
| `else` / `else if` column alignment                    | `else` and `else if` heads attach to their owning `if` by column alignment, not by indentation depth. See Appendix D.            | Appendix D / 002-30         |

## Appendix D. Layout column-alignment rule

The layout-time rule for **header continuations** is column-alignment,
distinct from the indent-depth rule that governs body openers. This
appendix consolidates the rule cross-referenced from §1.4 (lex layer),
§5.18 (if/else expression), §5.19 (match), §5.21 (`when` block), §5.22
(`given` block), and §5.20 (`observe` expression).

### D.1 Header continuations align to the owning header's column

The header-continuation keywords **`else`** and **`else if`** attach
to their owning `if` header **by column-alignment**: they sit at the
`if`'s indentation column, never deeper inside the prior arm body
(002-30, §1.4, §5.18).

```
// ✓ aligned to owning `if` column:
if cond:
  ...
else if other:
  ...
else:
  ...

// ✗ misaligned — `else` sits inside the prior arm body:
if cond:
  ...
  else:
    ...
```

This is what discriminates an `else if` chain from an independent
nested `if`: the column relationship to the opener, not the indent of
the body. The layout pre-processor (§2.1) does not synthesize an
`INDENT` for an `else`/`else if` head; the parser sees the
continuation token at the owning header's column and binds it to the
open `if`.

### D.2 In-block terminal arms sit at arm indent

In-block terminal arms — **`otherwise:`** in a `when` block (§5.21,
§13.9.12), **`default:`** in a `given` block (§5.22, §13.9.13) or
`observe` expression (§5.20, §13.2.11), and the catch-all arm of
`match` (§5.19, §6.2.4) — are arms of their own block. They sit at
**arm indent** like every other arm, governed by their construct's
arm-layout rules. They are **not** header continuations and do **not**
follow the column-alignment rule of §D.1.

```
// `otherwise:` is at arm indent — the same column as the other arms:
when:
  cond1: ...
  cond2: ...
  otherwise: ...

// `default:` in `given` — at arm indent:
given v:
  Some(x): ...
  None:    ...
  default: ...

// `_` catch-all in `match` — at arm indent:
match v:
  Some(x): ...
  None:    ...
  _:       ...
```

### D.3 Cross-reference summary

| Construct                  | Continuation keyword(s)  | Layout rule                            |
|----------------------------|--------------------------|-----------------------------------------|
| `if` / `else if` / `else`  | `else`, `else if`        | Column-align to owning `if` (§D.1)       |
| `when` block               | `otherwise:`             | Arm indent (§D.2)                       |
| `given` block              | `default:`               | Arm indent (§D.2)                       |
| `observe` expression       | `default:`               | Arm indent (§D.2)                       |
| `match` expression         | `_:` (catch-all)         | Arm indent (§D.2)                       |
