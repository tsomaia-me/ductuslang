# Ductus IR Text Grammar

## 0. Introduction

This document specifies the **text form of the Ductus IR** — the intermediate representation that constitutes the contract between the Ductus frontend (lexer, parser, semantic analyzer) and the runtime/backend (loader, scheduler, behavior interpreter).

**Scope.** The text-form productions for:

- **Module text** — top-level module / type / graph IR (SPEC §15.4.6)
- **Behavior IR** — the per-behavior IR body (SPEC §15.4.4), with all productions defined and no ellipses
- **Lexeme definitions** — `NAME`, `INT`, `FLOAT`, `STRING`, `HexU32`, `PATH`, `BID`, `type_tag`, `PRIM`

**Out of scope.** Runtime semantics live in `SPEC.md` §§13.14 / 14 / 15:

- How behaviors are dispatched
- Snapshot mechanics
- Hot-reload reconciler hooks
- Runtime contract beyond the on-the-wire text shape

The surface (source) grammar that produces input to the IR is in `GRAMMAR.md`; this file does **not** cover surface syntax.

**Relation to SPEC.** This file is a standalone re-formulation of SPEC §15.4 (Compilation Model & IR). It is normative for the IR text shape: a backend implementer should be able to write an IR loader from this document alone, consulting SPEC only for the post-load semantics. Specifically the §5 worked example is covered: mid-path ordinal `PATH` segments, comma-separated operands, `;`-separated intra-block instructions, bare-parameter operands, suffixed `op.PRIM` mnemonics, and the composite `instr_rhs` forms (`call`, `closure.make`, `enum.make`, `array.make`, `record.make`, `raw_alloc`) are all explicit productions in §2–§4.

**Audience.** Runtime and backend implementers — IR loader authors, behavior interpreters, snapshot / reload subsystems.

**How to read this document.** Start with §1 (Notation) and §2 (Lexeme definitions). §3 gives the module-level text grammar; §4 gives the per-behavior IR. §5 walks a concrete worked example. §6 cross-references each production back to SPEC and DECISION_LOG.

## 1. Notation

### EBNF symbols

The EBNF flavor matches `GRAMMAR.md` §1:

| Symbol | Meaning |
|--------|---------|
| `::=` | production / rewrite arrow |
| `|` | alternative |
| `?` | suffix: zero-or-one occurrence (optional) |
| `*` | suffix: zero-or-more occurrences |
| `+` | suffix: one-or-more occurrences |
| `(...)` | grouping |
| `'literal'` | exact literal token |

### Naming conventions

| Convention | Meaning |
|------------|---------|
| `PascalCase` | syntactic nonterminal |
| `UPPER_CASE` | terminal / lexeme |
| `lowercase` | layout marker (rarely used in the IR text form, which is largely free-form) |

### Production-block format

```ebnf
Nonterminal      ::= Alternative1
                   | Alternative2
                   ;  (§<spec-ref>, <log-ref>)

// Disambiguator note (when grammar alone is ambiguous):
// "<short rule explaining how the parser resolves the case>"
```

Each block carries a single source pointer of the form `;  (§N.M, NNN-MM)` citing the most-authoritative SPEC section and DECISION_LOG entry.

### Lexeme conventions

IR text is line-oriented but not layout-sensitive: indentation is purely cosmetic and does not synthesize tokens. Tokens are separated by whitespace; line breaks separate top-level statements. **Within a behavior block** (`behavior B@… { … }`), instructions may be separated either by physical line breaks *or* by an explicit `;` separator — both forms are normative (see `InstrSep` in §4). The two are interchangeable; the `;` form lets a single physical line hold multiple instructions (as the worked example in §5 demonstrates). All `NAME`, `INT`, `FLOAT`, `STRING`, `HexU32`, `PATH`, `BID`, `PRIM`, and `type_tag` lexemes are defined in §2.

## 2. Lexeme definitions

The IR text form's terminals. `NAME`, `INT`, `FLOAT`, and `STRING` are the primitive lexemes (per SPEC §15.4.6); each is pinned to an exact character rule below — none of them is left to "the conventional form". The remaining lexemes — `PATH`, `BID`, `HexU32`, `type_tag`, and `PRIM` — are defined formally further below.

**File intake.** A byte-order mark is stripped at offset 0 and nowhere else; a BOM at any other offset is a lexical error (033-254). Inter-token whitespace is exactly space, tab, and the line terminators: a `\r\n` pair normalizes to a newline before tokenization, and a lone carriage return is a lexical error — outside a lexeme and inside a `STRING` or `QuotedKey` payload alike. Form feed, vertical tab, and non-breaking space are not whitespace: outside string content they are lexical errors.

**No comments.** The text form admits no comment syntax of any kind — no line comment, no block comment, no ignorable-text production. Every byte of a `.duir` file belongs to the grammar (033-246). The `//` lines in this document are EBNF metalanguage, not `.duir` syntax; explanatory prose about a module lives outside the module, beside its witness test.

**Reserved spellings.** Five spellings are reserved language-wide — `null`, `true`, `false`, `nan`, and `inf` (002-39) — and in this text form none of them is ever a `NAME`. A module name, a `type_def` id, a field name, a variant name, a binding name, and a `PATH`'s name segment all reject them: a reader that meets one in any of those positions reports it under its own diagnostic class rather than reading it as an identifier (033-259). Two consequences follow and both are load-bearing. A bare occurrence of one of the five where a `PATH` and a literal both stand — a `wire`'s destination, a `binding`'s right-hand side — is always the literal, so the reader needs no lookahead there (033-258). And a `KeySegment` is untouched: a key whose stringified content is one of the five is DATA, stays legal, and renders single-quoted, because the bare rendering is reserved for the keyword reading (033-243).

**Dot ownership (lexer vs parser).** The lexer never composes a dotted lexeme: it emits `Ident`, `Dot`, and `Int` tokens, and both dotted `PATH`s (`App.count`) and dotted op mnemonics (`add.i32`, `const.i32`, `cast.i64.i32`, `tuple.make`) are composed by the parser (033-253). Position always disambiguates the two: a mnemonic heads an `instr_rhs`, a `PATH` never does. So the "one token or three" question has one answer — three tokens, always, composed above the lexer.

```ebnf
NAME             ::= /* identifier — letters, digits, '_'; not starting with
                        a digit; never one of the five reserved spellings
                        'null', 'true', 'false', 'nan', 'inf' */
                   ;  (§15.4.6, 033-168 / 033-259)

INT              ::= /* optionally signed decimal integer literal — the sign,
                        when present, is a leading '-' INSIDE the lexeme */
                   ;  (§15.4.6, 033-247)

FLOAT            ::= '-'? Digit+ '.' Digit+
                   ;  (§15.4.6, 033-248)

Digit            ::= /* '0'..'9' */
                   ;  (§15.4.6, 033-248)

STRING           ::= '"' (Raw | Escape)* '"'
                   ;  (§15.4.6, 033-249)

Raw              ::= /* any Unicode scalar except '"', '\', and a lone
                        carriage return — raw newlines and raw tabs ARE
                        legal */
                   ;  (§15.4.6, 033-249)

Escape           ::= '\"' | '\\' | '\n' | '\r' | '\t'
                   | '\u{' HexScalar '}'
                   ;  (§15.4.6, 033-249)

HexScalar        ::= /* 1..6 hexadecimal digits [0-9a-fA-F] naming a
                        Unicode scalar */
                   ;  (§15.4.6, 033-249)

// The text form has no unary-minus production and no expression
// machinery: a negative `init`, a negative fold `else`, and a negative
// behavior-body constant are all spelled inside the lexeme —
// `init -5`, `else -1`, `const.i32 -5` (033-247).
//
// FLOAT carries decimal digits on BOTH sides of a mandatory dot and has
// NO exponent form; it is valid wherever a `value` (§3) or a `literal`
// (§4) is admitted — not in behavior literals alone. A float-typed
// position additionally admits the three spellings 'nan', 'inf', and
// '-inf', so every float value has a text form. The canonical emitter
// output (shortest round-trip decimal, never exponent form) always fits
// the FLOAT lexeme (033-248).
//
// **STRING canonical emission.** Raw wherever raw is legal, with four
// exceptions: '"' emits '\"', '\' emits '\\', a carriage return emits
// '\r' (a raw CR would not survive file-level newline normalization),
// and a control scalar other than newline and tab emits '\u{…}';
// newline and tab emit RAW. Every str value is therefore spellable
// (totality) and has exactly one canonical spelling (uniqueness). A
// non-canonical spelling parses but is not what the emitter writes —
// exactly as with non-canonical key quoting (033-249).
//
// A lone carriage return is NOT a Raw scalar: it is a lexical error
// everywhere, inside a STRING or a QuotedKey exactly as outside one. A
// '\r\n' pair inside a STRING or QuotedKey payload normalizes to a
// newline — file-level line-ending normalization cannot be selective —
// so a payload wanting a literal carriage return spells '\r', which is
// exactly its canonical form (033-249).
```

A **PATH** is a cell or instance fully-qualified declaration path: a dot-separated sequence of segments naming the lexical nesting from module root through enclosing instances to the cell or instance name. Segments are lexical identifiers; a `repeat` scope contributes a keyed multiplicity segment — the scope key's stringified `StringifiableKey` form, bare or single-quoted per §15.4.1.1's canonical-quoting rule. Anonymous or duplicated sibling placements append an ordinal suffix `':' INT` (zero-based, declaration-order index among siblings of the same type at the same nesting depth).

```ebnf
PATH             ::= PathSegment ('.' PathSegment)*
                   ;  (§15.4.1.1, 033-130)

PathSegment      ::= NAME (':' INT)?
                   | KeySegment
                   ;  (§15.4.1.1, 033-133 / 033-243)

KeySegment       ::= BareKey | QuotedKey
                   ;  (§15.4.1.1, 033-243)

BareKey          ::= /* a nonempty identifier-shaped run — letters, digits,
                        '_'; not starting with a digit — that is not one of
                        the five reserved spellings. Any other key renders
                        as a QuotedKey. */
                   ;  (§15.4.1.1, 033-243 / 033-259)

QuotedKey        ::= "'" (KeyRaw | KeyEscape)* "'"
                   ;  (§15.4.1.1, 033-243)

KeyRaw           ::= /* any Unicode scalar except "'", '\', and a lone
                        carriage return — raw newlines and raw tabs ARE
                        legal, exactly as in a STRING */
                   ;  (§15.4.1.1, 033-243)

KeyEscape        ::= "\'" | '\\' | '\n' | '\r' | '\t'
                   | '\u{' HexScalar '}'
                   ;  (§15.4.1.1, 033-243)

// The ordinal suffix `:' INT` may attach to ANY path segment, not just
// the tail — e.g. 'App.print:0.text' has the ordinal on the middle
// component naming an anonymous effect instance. See 033-130 / 033-133.
// A KeySegment is a repeat scope key's stringified StringifiableKey
// form (033-243): bare iff it is identifier-shaped — the NAME charset
// exactly, nothing more and nothing less — AND is not one of the five
// reserved spellings; single-quoted otherwise. Quoting is canonical —
// producers emit bare whenever bare is available. A BareKey therefore
// always lexes as NAME and is indistinguishable from a name segment in
// isolation; the graph structure decides which path positions are
// keyed-scope positions, so the loader needs no distinction. A
// KeySegment carries no ordinal suffix. A purely numeric key is NOT
// identifier-shaped and so quotes.
//
// **Key canonical emission (033-243).** A QuotedKey's payload is the
// STRING payload with "'" for '"', so its canonical emission is the STRING
// rule with the same substitution: raw wherever raw is legal, with four
// exceptions — "'" emits "\'", '\' emits '\\', a carriage return emits '\r',
// and a control scalar other than newline and tab emits '\u{…}'; newline and
// tab emit RAW. Every key content is therefore spellable (totality) and has
// exactly one canonical spelling (uniqueness), exactly as every str value
// does. The reserved-spelling exclusion is what makes uniqueness MEANING-safe
// as well: a key spelled 'null' stays quoted, so it round-trips as the key it
// is rather than re-reading as the keyword.
// e.g. 'audio.synth_a.osc_1.frequency', 'App.print:0', 'App.print:0.text',
// and — quoted keys spelled without the surrounding example quotes —
// App.rows.'42'.render_row, App.rows.'us-east.1'.render_row
```

A **BID** is a behavior handle — the behavior's `u32` handle (§14.6.3) rendered in hexadecimal, prefixed by `'B@'`. It is the compact runtime reference; the behavior's wide content-addressed identity is not spelled in the text form.

```ebnf
BID              ::= 'B@' HexU32
                   ;  (§15.4.4, 033-167)

HexU32           ::= /* 1..8 hexadecimal digits [0-9a-fA-F] — encodes a u32
                        behavior handle */
                   ;  (§14.6.3, 033-167)

// e.g. 'B@d1', 'B@aa10'. The hex run is bounded to ≤ 8 digits
// because the handle is a u32.
//
// The prefix is CASE-SENSITIVE — exactly 'B@'. Lowercase 'b@d1' opens no
// handle: it lexes as the ordinary tokens Ident('b'), At, Ident('d1')
// (033-254).
//
// BOUNDARY RULE: the hex run must end at a NON-identifier byte. A
// NONEMPTY run that continues into identifier characters is not a
// handle at all — 'B@d1x' is Ident('B'), At, Ident('d1x'), not a
// malformed BID — which is what keeps an '@'-provenance clause after a
// path segment named 'B' spellable: 'App.B@bound' lexes Ident, Dot,
// Ident, At, Ident (033-254). So: 1..8 hex digits ending at a
// non-identifier byte is a BID; MORE than 8 hex digits ending at a
// non-identifier byte is a malformed BID, never a fallback to ordinary
// tokens; and NO hex digit at all ('B@', 'B@ d1', 'B@xyz') is a
// malformed BID too — the boundary rule speaks only of a run that
// exists.
```

**PRIM** is the closed set of primitive type tags. These are the type-erased primitives the runtime understands directly (§4.1); aggregates and closures are built on top via `type_tag`.

```ebnf
PRIM             ::= 'i8'  | 'i16' | 'i32' | 'i64' | 'i128'
                   | 'u8'  | 'u16' | 'u32' | 'u64' | 'u128'
                   | 'isize' | 'usize'
                   | 'f32' | 'f64'
                   | 'bool' | 'str'
                   ;  (§15.4.6, 033-43)
```

**`type_tag`** is the unified type-tag nonterminal used throughout both the module and behavior grammars. It encodes a primitive, a named record/enum/tuple layout from the module's `types` section (`'%'NAME`), a pool-indexed aggregate (`pool_index<%TypeId>`), a `handle<T>`/`portal<T>` reference pair (inline `(slot_path, generation)` representation, §15.4.1), a tuple structural type, a fixed-size array, or a closure environment type.

```ebnf
type_tag         ::= PRIM
                   | '%' NAME
                   | 'pool_index' '<' '%' NAME '>'
                   | 'yielded' '<' type_tag '>'
                   | 'handle' '<' type_tag '>'
                   | 'portal' '<' type_tag '>'
                   | '(' type_tag (',' type_tag)* ')'
                   | '[' type_tag ';' INT ']'
                   | 'closure' '<' '(' (type_tag (',' type_tag)*)? ')' '->' type_tag '>'
                   ;  (§15.4.6, 033-43 / 033-79)

// An aggregate-valued cell — a record, enum, or tuple — is typed
// 'pool_index<%TypeId>' (§14.3.3), never an inline '%TypeId'.
// 'yielded<T>' is the ABI-only membership-descriptor type (§15.4.1,
// 033-79): it types synthesized membership-descriptor cells and
// 'yielded T' parameters, and is never a user-facing value type.
//
// NESTING BOUND (033-255). A type_tag nests at most 64 levels deep.
// Each bracketed constructor is ONE level: a 'pool_index', 'yielded',
// 'handle', or 'portal' wrapper, a tuple position, an array element,
// and a 'closure' parameter or return. A tag that exceeds the bound is
// a grammar error surfaced at the token that exceeds it, under its own
// diagnostic class. The bound is format law, not implementation
// latitude: every conforming reader accepts every tag within it and
// rejects every tag beyond it, identically.
```

## 3. Module text grammar

The normative text form of the module-level IR (§15.4.1's abstract data model). Lifted verbatim from SPEC §15.4.6. `NAME`, `INT`, `FLOAT`, and `STRING` are the lexemes pinned in §2; `PATH` is the cell/instance path (§15.4.1.1); `BID` is the behavior handle (§15.4.4); `type_tag` is the type-erased graph tag (§15.4.3).

**Entry termination (normative).** In the `types`, `graph`, and `behaviors` sections a NEWLINE terminates an entry — one `type_def`, one `entry`, one `behavior` per line (033-250). Consecutive NEWLINEs collapse, so blank lines between entries are insignificant. No other entry separator is admitted at these levels; the enclosing section's closing `}` also terminates a final entry that shares its line, as in the one-line `types { %Request = … align 8 }` of §5. An entry carrying a **braced body** — a `scope`, a `behavior` — is terminated by the NEWLINE after its closing `}` and may span physical lines between its braces, where the body's own rules govern: inside a behavior block a NEWLINE is an instruction separator, not an entry terminator, and `;` is admitted there and nowhere else (`InstrSep`, §4). The terminator is what makes a brace-less entry's optional trailing clauses unambiguous — a `cell` line's trailing `gate PATH` clause cannot swallow the `gate` entry on the next line. The NEWLINE is omitted from the productions below only to keep every line readable; it is normative wherever this paragraph names it, exactly as if each `type_def`, `entry`, and `behavior` were written `… NEWLINE`.

```ebnf
module           ::= 'module' NAME '{' types_section graph_section behaviors_section '}'
                   ;  (§15.4.6, 033-168)

types_section    ::= 'types' '{' type_def* '}'
                   ;  (§15.4.6, 033-168)

type_def         ::= '%' NAME '=' layout 'size' INT 'align' INT
                   ;  (§15.4.6, 033-168)

layout           ::= 'record' '{' field_list '}'
                   | 'enum'   '{' variant (',' variant)* '}'
                   | 'tuple'  '(' type_tag (',' type_tag)* ')'
                   ;  (§15.4.6, 033-168)

field_list       ::= (NAME ':' type_tag (',' NAME ':' type_tag)*)?
                   ;  (§15.4.6, 033-168)

variant          ::= '#' NAME ('(' type_tag (',' type_tag)* ')')?
                   ;  (§15.4.6, 033-168)

graph_section    ::= 'graph' '{' scope* '}'
                   ;  (§15.4.6, 033-168 / 033-256 / 033-260)

// The arity is `scope*`: an empty `graph {}` is structurally legal — a module
// of pure types — and any minimum-content requirement is a load-time semantic
// rule rather than a grammar one (033-256).

// **Entry order is semantic (normative).** The sequence of scopes in the
// section, and of entries in each scope's body, is the carrier of the
// composed evaluation-order tiebreak (instance placement order, then
// intra-instance declaration order — SPEC §13.10.3): a conforming emitter
// preserves it, a conforming loader reads entry rank as the same-level
// tiebreak, and reordering entries is a semantic change (033-260). Blank
// lines are insignificant; entry order is not.

scope            ::= 'scope' PATH 'exposes' path_set 'effects' path_set
                     ('reset_on_reopen' reopen_set)? '{' entry* '}'
                   ;  (§15.4.6, 033-168 / 033-75)

// 'reset_on_reopen' on a scope carries a reopen_set of (consumer : stream)
// pairs; this is one of three grammar positions for the keyword — see also
// the bare flag on the 'cell' production and the bare flag on the 'stream'
// production. All three spellings are normative (033-215).

entry            ::= cell | gate | wire | effect | stream
                   ;  (§15.4.6, 033-168)

cell             ::= cell_kind PATH ':' type_tag
                     ('uses' BID)? ('inputs' path_set)? ('depth' INT)?
                     ('reset_on_reopen')? ('init' value)? ('gate' PATH)?
                     ('combiner' BID)? ('else' value)? ('members' member_set)?
                   ;  (§15.4.6, 033-169 / 033-215 / 033-81)

// 'reset_on_reopen' on a cell is a bare flag (no payload) — one of three
// grammar positions for the keyword (033-215): the bare flag here on
// 'cell', the bare flag on 'stream', and the 'reopen_set' payload on
// 'scope'. 'depth' applies to
// 'recurrent' cells (history depth) and 'deferred' cells (latency
// depth in commits, 033-245); 'uses' / 'inputs' apply to 'derived' /
// 'recurrent' / 'deferred';
// 'init' applies to 'input' and 'recurrent'. 'combiner' and 'else'
// apply only to 'fold' cells (033-81): 'combiner' names the 'by:'
// combiner's behavior handle and 'else' the empty-membership result.
// 'members' applies to 'fold' cells and to membership-descriptor
// cells — a membership-descriptor cell (the lowering of a standalone
// 'yielded' group) renders as a 'derived' line typed 'yielded<T>'
// carrying a 'members' set and no 'uses' / 'inputs' (033-169). The
// clauses must appear
// in the order listed in the production (`uses`, `inputs`, `depth`,
// `reset_on_reopen`, `init`, `gate`, `combiner`, `else`, `members`);
// each clause is optional (subject
// to the cell-kind constraints above) and appears at most once.

cell_kind        ::= 'input' | 'derived' | 'recurrent' | 'deferred' | 'fold'
                   ;  (§15.4.6, 033-80)

// The five cell-kind tags are closed (033-80): 'fold' is a cell KIND,
// not a seventh graph primitive, and no 'yielded' tag exists — a
// standalone 'yielded' group lowers to a membership-descriptor
// 'derived' cell (above), and a folded group is absorbed into its
// fold cell's 'members' set.

member_set       ::= '[' (member (',' member)*)? ']'
                   ;  (§15.4.6, 033-81)

member           ::= PATH ':' member_driver
                   ;  (§15.4.6, 033-81)

member_driver    ::= 'permanent' | 'keyed_template' | 'gate_guarded'
                   ;  (§15.4.6, 033-81)

// A 'members' set lists the member edges in member order (walk order
// for a membership-descriptor cell), each PATH naming the member cell
// and each tag naming its membership driver — 'permanent' (a static
// position), 'keyed_template' (a repeat-driven position, present per
// key), or 'gate_guarded' (a gated-arm position, present iff its gate
// is effectively active). The three tags are snake_case keywords in the
// NAME charset; no driver tag contains a hyphen (033-81).
//
// **Boundary ambiguity note.** A 'member' pair separator is ':' and a
// PATH segment may itself end with a "(':' INT)?" ordinal suffix. The
// disambiguator matches reopen_set's: after a colon, an INT is the
// left-hand PATH's ordinal segment; a NAME-led token that is one of
// the three driver keywords closes the pair.

gate             ::= 'gate' PATH 'pred' BID 'inputs' path_set 'guards' path_set
                     ('gate_parent' PATH)?
                   ;  (§15.4.6, 033-168 / 033-65)

wire             ::= 'wire' PATH 'from' PATH 'to' (PATH | 'null')
                     ('type' type_tag)? ('attrs' binding_set)? ('gate' PATH)?
                   ;  (§15.4.6, 033-168 / 033-86 / 033-87 / 033-88 / 033-89 / 033-258)

// **Keyword over path, by reservation.** The 'to' clause admits both a
// PATH and the word 'null', and the word is the literal outright: 'null'
// is a reserved spelling, so no NAME segment may be 'null' and no PATH
// can begin with one (033-259). No lookahead is needed and nothing is
// shadowed (033-258). 'null.x' is therefore not a path: the reader takes
// 'null' as the complete destination and the '.' that follows it is
// simply the next token, unexpected where an entry has ended. (The
// reserved-spelling diagnostic belongs to the NAME positions; here the
// word is not in a name position at all.) A key spelled 'null' remains
// reachable: it is a KeySegment and renders quoted. The same rule governs
// the right-hand side of a 'binding' — see the 'value' production.

effect           ::= 'effect' PATH 'reconciler' STRING 'params' binding_set
                     ('desired' path_set)? ('observed' path_set)? ('gate' PATH)?
                   ;  (§15.4.6, 033-168 / 033-119 / 033-120 / 033-121 / 033-122 / 033-123)

stream           ::= 'stream' PATH ':' type_tag 'policy' ('ring' | 'gate') 'capacity' INT
                     ('source_deps' path_set)? ('observes' path_set)?
                     ('reset_on_reload')? ('reset_on_reopen')?
                     ('history' INT)? ('lookback' lookback_map)?
                   ;  (§15.4.6, 033-106 / 033-107 / 033-108 / 033-109 / 033-110 / 033-111 / 033-112 / 033-114 / 033-115 / 033-117)

// A 'stream' line serializes the stream primitive (§15.4.1's Stream
// cells) with its ten data-model fields: PATH is the stream's id;
// type_tag the element_type; 'policy' is 'ring' or 'gate'; 'capacity'
// the integer capacity; 'source_deps' the source_dependencies; 'observes'
// the six synthesized observation-cell ids (pending_count, pressure,
// is_full, dropped_total, rejected_total, last_overflow_at); the two
// bare flags 'reset_on_reload' and 'reset_on_reopen' mark the matching
// @-annotations ('reset_on_reopen' here is the third grammar position of
// the keyword — a bare flag on 'stream', alongside the bare flag on
// 'cell' and the reopen_set on 'scope', 033-215); 'history' is
// output_history_size (present only on a recurrent[N] stream); and
// 'lookback' is the input_lookback_map (input cell id → max lookback k).

behaviors_section ::= 'behaviors' '{' behavior* '}'
                   ;  (§15.4.6, 033-168)

// 'behavior' production defined in §4 (per SPEC §15.4.4).
// **Module-resolvability constraint.** An empty `behaviors {}` is
// syntactically well-formed but renders any non-empty `graph_section`
// unresolvable — every `derived`/`recurrent`/`deferred` cell's `uses BID` and
// every `gate`'s `pred BID` reference a `behavior` by handle. The
// match is a load-time semantic check (post-parse), not a grammar
// rule; a module with `derived … uses B@xx` and no defining
// `behavior B@xx { … }` is rejected at module load.

path_set         ::= '[' (PATH (',' PATH)*)? ']'
                   ;  (§15.4.6, 033-168)

binding_set      ::= '[' (binding (',' binding)*)? ']'
                   ;  (§15.4.6, 033-168)

reopen_set       ::= '[' (PATH ':' PATH (',' PATH ':' PATH)*)? ']'
                   ;  (§15.4.6, 033-75)

// reopen_set elements are (consumer_id : stream_id) pairs naming the
// consuming operator/derived instance and the consumed stream whose
// cursor resets to head on the scope's gate reopen edge.
//
// **Boundary ambiguity note.** Each `PATH` may itself end with a
// `(':' INT)?` ordinal suffix, and a `PATH ':' PATH` pair separator
// is also `:`. The parser commits to the pair separator when the
// token following the colon begins a fresh `PATH` (a NAME). When the
// token following the colon is an INT, it is the ordinal segment of
// the left-hand `PATH`. Per-segment ordinals always attach to a
// NAME-led segment (`PathSegment`); the pair separator's RHS always
// starts with a NAME — so the disambiguator is "next token is INT →
// ordinal; NAME → pair separator".

lookback_map     ::= '[' (PATH ':' INT (',' PATH ':' INT)*)? ']'
                   ;  (§15.4.6, 033-117)

// lookback_map elements are (input_cell_id : INT) pairs — an input cell
// read via `.past(k, ...)` in a recurrent stream's body mapped to its
// maximum lookback k, driving per-input history allocation. Empty for a
// non-recurrent stream and for a recurrent stream whose body calls no
// `.past` on inputs (033-118).
//
// **Scope of the colon-ordinal disambiguator.** The "next token is INT →
// ordinal; NAME → pair separator" rule stated under reopen_set applies
// to reopen_set ONLY. It cannot apply here, where BOTH sides of a pair's
// colon-INT are integers: in a lookback_map entry the LAST ':' INT of
// the entry is the lookback value, and any earlier ':' INT belongs to
// the key PATH. An ordinal-suffixed lookback key therefore stays
// expressible — 'App.print:0 : 3' is the cell 'App.print:0' at
// lookback 3.

binding          ::= NAME ':' (PATH | value) ('@' provenance)?
                   ;  (§15.4.6, 033-252)

provenance       ::= 'bound' | 'constant_wrap' | 'bridge'
                   ;  (§15.4.6, 033-252)

// The '@'-suffixed provenance marker on an effect's 'params' binding (or
// a wire's 'attrs' binding) records how the cell argument was
// materialized under the uniform cell-argument rule (§13.17.3). It is
// part of the normative text form — the '@' token belongs to the text
// form's inventory — and is optional metadata on the binding, never a
// distinct binding case (033-252).
// e.g. 'params [message: App.label@bound]'.

value            ::= INT | FLOAT | 'true' | 'false' | STRING
                   | 'nan' | 'inf' | '-inf'
                   ;  (§15.4.6, 033-168 / 033-212 / 033-248 / 033-257 / 033-258)

// **Keyword over path, by reservation.** The right-hand side of a 'binding'
// is the one value position that also admits a PATH, and there the words
// 'true', 'false', 'nan', and 'inf' are the literals outright — the same
// reservation the wire destination's 'null' rests on (033-259). No lookahead
// decides it and nothing is shadowed: 'flag: true' binds the literal, and
// 'flag: true.state' is not a path — the reader takes 'true' as the whole
// right-hand side and the '.' after it is an unexpected token (033-258).
// Where a value stands ALONE — a cell's 'init', a fold cell's 'else' — no
// path is admitted, so the question never arises.

// **Canonical digit spelling (033-257).** A typed value is not converted when
// it is read — the loader owns its type — so canonicalization here is TEXTUAL
// and value-preserving under every type the position may later take. An INT
// strips leading zeros to at least one digit, and '-0' canonicalizes to '0'. A
// FLOAT keeps its sign ('-0.0' is a value of its own), strips integer-part
// leading zeros to at least one digit and fraction trailing zeros to at least
// one digit — '007' → '7', '1.50' → '1.5', '00.0' → '0.0'. 'nan', 'inf', and
// '-inf' are canonical as written. A non-canonical spelling parses but is not
// what the emitter writes, exactly as with STRING content and key quoting.
//
// value is a compile-time literal (the placement_binding_kind is either
// 'static' (value literal) or 'reactive' (PATH referencing source cells);
// per §13.8.4 / §15.4.1). The three alternatives 'nan', 'inf', and
// '-inf' are TYPE-GATED: they are admitted only where the position's
// type is a float ('f32'/'f64'), and rejected elsewhere by the
// load-time type check, not by the grammar (033-248).
```

## 4. Behavior IR text grammar

The text grammar for the per-behavior body — a pure, typed, SSA-with-block-arguments compute that the runtime invokes via `BID`. This block is normative for the behavior-IR text grammar; SPEC §15.4.4 carries a prose-style mirror that conforms to it (post Phase-B formalization: every nonterminal below has a defining production; `type_tag` is used uniformly — the bare name `type` does not appear; `terminator`, `op`, `literal`, and `LABEL` are explicit productions).

```ebnf
behavior         ::= 'behavior' BID '(' params ')' '->' type_tag '{' block+ '}'
                   ;  (§15.4.4, 033-205)

params           ::= (('own')? NAME ':' type_tag) (',' ('own')? NAME ':' type_tag)*
                   ;  (§15.4.4, 033-206 / 033-203)

// A param without 'own' is a borrow (read, not consumed); 'own' marks
// it as consumed. The entry block's parameters are the behavior's
// parameters; a branch supplies its successor block's arguments
// (this replaces phi).

block            ::= LABEL ('(' params ')')? ':' ( instr InstrSep )* terminator
                   ;  (§15.4.4, 033-207)

InstrSep         ::= ';' | NEWLINE                            // intra-block separator
                   ;  (§15.4.4, 033-207)

// Multiple instructions on one line are separated by `;`; one
// instruction per line uses an implicit NEWLINE. The two are
// interchangeable per the worked example (§5). NEWLINE here is a
// physical line terminator (the IR text form is line-oriented, but
// `;` overrides the line shape).

instr            ::= '%' (NAME | INT) '=' instr_rhs (':' type_tag)?
                                                              // typed-result form
                   | instr_rhs                                // void/effect-result form
                   ;  (§15.4.4, 033-208 / 033-251)

// An SSA temporary is named '%' (NAME | INT): digit-named temporaries
// like '%0', '%1', '%42' are first-class alongside identifier-named
// ones, and the form holds at EVERY SSA reference position — the
// instruction result here, 'operand', a 'move' operand, 'call.dyn''s
// receiver, and a 'cond_br' or 'switch' scrutinee (033-251). The '%'
// prefixing a TYPE id ('%TypeId' in 'type_tag', 'enum.make',
// 'record.make') is a different, unrelated use of the sigil and always
// takes a NAME.

instr_rhs        ::= op_application
                   | call_rhs                                 // 'call' f(args), 'call.dyn' obj #method(args)
                   | closure_rhs                              // 'closure.make' BID 'captures' '{' ... '}'
                   | enum_make_rhs                            // 'enum.make' %T #V(p)
                   | array_make_rhs                           // 'array.make' '[' operand_list? ']'
                   | record_make_rhs                          // 'record.make' %T '{' field_inits? '}'
                   | raw_alloc_rhs                            // 'raw_alloc' '<' type_tag '>' '(' operand ')'
                   ;  (§15.4.4, 033-211)

op_application   ::= op ( operand (',' operand)* )?           // comma-separated operands
                   ;  (§15.4.4, 033-211)

call_rhs         ::= 'call' callee '(' operand_list? ')'
                   | 'call.dyn' '%' (NAME | INT) '#' NAME '(' operand_list? ')'
                   ;  (§15.4.4, 033-211 / 033-251)

callee           ::= NAME                                    // direct call by behavior NAME (frontend-resolved)
                   | BID                                     // direct call by BID
                   ;  (§15.4.4, 033-211)

closure_rhs      ::= 'closure.make' BID 'captures' '{' capture_list? '}'
                   ;  (§15.4.4, 033-211)

capture_list     ::= NAME ':' operand (',' NAME ':' operand)*
                   ;  (§15.4.4, 033-211)

enum_make_rhs    ::= 'enum.make' '%' NAME '#' NAME ( '(' operand_list? ')' )?
                   ;  (§15.4.4, 033-211)

array_make_rhs   ::= 'array.make' '[' operand_list? ']'
                   ;  (§15.4.4, 033-211)

record_make_rhs  ::= 'record.make' '%' NAME '{' field_init_list? '}'
                   ;  (§15.4.4, 033-211)

field_init_list  ::= NAME ':' operand (',' NAME ':' operand)*
                   ;  (§15.4.4, 033-211)

raw_alloc_rhs    ::= 'raw_alloc' '<' type_tag '>' '(' operand ')'
                   ;  (§15.4.4, 033-211)

operand_list     ::= operand (',' operand)*
                   ;  (§15.4.4, 033-211)

operand          ::= '%' (NAME | INT)                        // SSA result reference
                   | NAME                                    // bare param name (entry-block parameter)
                   | 'move' '%' (NAME | INT)
                   | 'move' NAME                             // move of a bare param
                   | literal
                   ;  (§15.4.4, 033-209 / 033-251)

terminator       ::= 'br' LABEL paren_args?
                   | 'cond_br' '%' (NAME | INT) ',' LABEL paren_args? ',' LABEL paren_args?
                   | 'switch' '%' (NAME | INT) switch_table
                   | 'ret' operand
                   | 'trap' STRING
                   ;  (§15.4.4, 033-210 / 033-251)

// Every block ends in exactly one terminator. 'match' lowers to
// 'enum.tag' + 'switch'. 'trap' is the only non-'ret' exit
// (§13.13.1).

paren_args       ::= '(' operand (',' operand)* ')'
                   ;  (§15.4.4, 033-183 / 033-184)

switch_table     ::= '[' switch_case (',' switch_case)* ',' 'default' ':' LABEL ']'
                   ;  (§15.4.4, 033-185)

switch_case      ::= INT ':' LABEL
                   ;  (§15.4.4, 033-185)

op               ::= ArithOp '.' PRIM                          // e.g. add.i32, mul.i64
                   | LogicOp '.' PRIM                          // e.g. and.bool, xor.u8
                   | CompareOp '.' PRIM                        // e.g. eq.i32, lt.f64
                   | 'const.' PRIM                             // const-of-primitive
                   | 'cast.' PRIM '.' PRIM                     // cast.i64.i32 (source.target)
                   | 'tuple.make'  | 'tuple.get'
                   | 'field.get'
                   | 'enum.tag'    | 'enum.payload'
                   | 'call.closure'
                   | 'clone' | 'drop'
                   | 'raw_free' | 'raw_read' | 'raw_write'
                   ;  (§15.4.4, 033-211)

ArithOp          ::= 'add' | 'sub' | 'mul' | 'div' | 'rem' | 'neg'
                   ;  (§15.4.4, 033-211)

LogicOp          ::= 'and' | 'or' | 'xor' | 'shl' | 'shr' | 'not'
                   ;  (§15.4.4, 033-211)

CompareOp        ::= 'eq' | 'ne' | 'lt' | 'le' | 'gt' | 'ge'
                   ;  (§15.4.4, 033-211)

// All arithmetic / logical / comparison ops are PRIM-suffixed at the
// instruction site (`add.i32`, `not.bool`); a bare mnemonic with no
// PRIM suffix is a parse error. `const.PRIM` and `cast.SRC.DST` are
// the two distinct dotted forms with explicit suffix counts. The
// composite RHS forms `call`, `call.dyn`, `closure.make`,
// `enum.make`, `array.make`, `record.make`, and `raw_alloc` use
// dedicated `instr_rhs` alternatives (above) rather than the bare
// `op` enumeration here.

literal          ::= INT | FLOAT | 'true' | 'false' | STRING
                   | 'nan' | 'inf' | '-inf'
                   ;  (§15.4.4, 033-212 / 033-248)

// 'literal' matches exactly the same lexemes as the module grammar's
// 'value' — the two productions are identical, FLOAT included; 'value'
// does not omit FLOAT, as an earlier note implied. The three float
// spellings 'nan', 'inf', and '-inf' are type-gated in both: admitted
// only in a float-typed position, rejected elsewhere by the load-time
// type check rather than by the grammar (033-248). Signs live inside
// the INT and FLOAT lexemes, so 'const.i32 -5' is one operand, not a
// negation (033-247).

LABEL            ::= 'bb' INT
                   ;  (§15.4.4, 033-213)

// e.g. 'bb0', 'bb1', 'bb42'.
//
// **`:` discrimination between block header and `instr` type tag.**
// The `:` after a `LABEL` (with optional `params`) opens the block's
// instruction list; the `:` inside an `instr` introduces the result's
// `type_tag`. They are unambiguous because a block opener appears in
// the production `block ::= LABEL (params)? ':' …` at the start of a
// statement (after a NEWLINE / EOF / `}`), while an `instr`'s `:`
// follows an `instr_rhs` (after operands, never at statement start).
// Implementers should not need lookahead: the leading token (`LABEL`
// vs `'%' (NAME | INT)` / op keyword) disambiguates.
```

## 5. Worked example

The following example brings the type table, graph IR, and behavior IR together. Lifted verbatim from SPEC §15.4.5. The source (post-`effects:`-clause, §13.3.8):

```
type Request:
  text: string

effect print(message: cell string):
  desired:
    derived text: string = message
    derived request: Request = Request(text: message)

node App:
  attr count: i32 = 0
  attr show: bool = true
  derived doubled: i32 = count * 2
  recurrent total: i32 = total.past(1, 0) + count
  derived label: string = "value is {doubled}"
  effects:
    when show:
      label |> print
```

lowers to:

```ductus-ir
module App {
  types { %Request = record { text: str }  size 8 align 8 }

  graph {
    scope App  exposes []  effects [App.print:0] {
      input     App.count   : i32  init 0
      input     App.show    : bool init true
      derived   App.doubled : i32  uses B@d1 inputs [App.count]
      recurrent App.total   : i32  uses B@d2 inputs [App.count] depth 1 init 0
      derived   App.label   : str  uses B@d3 inputs [App.doubled]

      gate App.g0  pred B@d4  inputs [App.show]  guards [App.print:0]

      derived   App.print:0.text    : str                  uses B@d5 inputs [App.label]
      derived   App.print:0.request : pool_index<%Request> uses B@d6 inputs [App.label]
      effect App.print:0  reconciler "print"  params [message: App.label]  desired [App.print:0.text, App.print:0.request]  gate App.g0
    }
  }

  behaviors {
    behavior B@d1 (count: i32) -> i32 {
    bb0: %0 = const.i32 2 ; %1 = mul.i32 count, %0 ; ret %1 }

    behavior B@d2 (count: i32, past1: i32) -> i32 {
    bb0: %0 = add.i32 past1, count ; ret %0 }

    behavior B@d3 (doubled: i32) -> str {
    bb0: %0 = const.str "value is " ; %1 = call int_to_str(doubled) : str
         %2 = call str_concat(move %0, move %1) : str ; ret %2 }

    behavior B@d4 (show: bool) -> bool { bb0: ret show }

    behavior B@d5 (message: str) -> str { bb0: %0 = clone message ; ret %0 }

    behavior B@d6 (message: str) -> %Request {
    bb0: %0 = record.make %Request { text: message } ; ret %0 }
  }
}
```

The effect sits in `App`'s `effects` set, not `exposes` (effects are not topology, §13.3.8); the gate `App.g0` guards it (gate-off ⇒ freeze + `suspend`, §13.9.7); `App.print:0` uses the ordinal path form for an anonymous instance (§15.4.1.1). Each declaration in the `desired:` block is its own graph cell — `App.print:0.text` and `App.print:0.request`, each with its own behavior and dependency edges — and the effect's `desired` list names them (§15.4.1's `desired_cell_ids`). Program reads of desired fields attach dependency edges to these per-declaration cells; the whole-record desired view the reconciler consumes is assembled by the runtime from their current values and is not a graph cell. Both desired behaviors are pure functions of `message` (bound to `App.label`), so the desired computation has no whole-effect activation input — effect activation is expressed solely through the suspend/resume model (§13.19.12) via the gate. An intra-desired `when`/`given` block, had the example used one, would lower its arm cells behind their own gate, whose predicate is an activation input for arm selection, distinct from the effect's suspend/resume gate.

## 6. Cross-reference table

Each production in §3 and §4 mapped to its source SPEC section and DECISION_LOG entry. The LOG entry cited is the most-authoritative one for that production block; SPEC sections are the normative prose.

### §2 — Lexeme definitions

| Production | SPEC § | LOG |
|------------|--------|-----|
| `NAME` | §15.4.6 | 033-168 / 033-259 (never one of the five reserved spellings) |
| `INT` | §15.4.6 | 033-168 / 033-247 (sign inside the lexeme) |
| `FLOAT` | §15.4.6 | 033-248 (`-? digits '.' digits`, no exponent; `nan`/`inf`/`-inf` spellings) |
| `STRING` | §15.4.6 | 033-249 (raw scalars incl. newline/tab; six escapes; canonical emission) |
| `Digit` | §15.4.6 | 033-248 |
| `Raw` | §15.4.6 | 033-249 |
| `Escape` | §15.4.6 | 033-249 |
| `HexScalar` | §15.4.6 | 033-249 |
| `PATH` | §15.4.1.1 | 033-130 / 033-133 / 033-243 / 033-259 (mid-path ordinal for anonymous effect instances, see worked example `App.print:0.text`; keyed-scope multiplicity segment, bare iff identifier-shaped and not one of the five reserved spellings, single-quoted otherwise) |
| `PathSegment` | §15.4.1.1 | 033-133 / 033-243 / 033-259 (a name segment is never a reserved spelling) |
| `KeySegment` | §15.4.1.1 | 033-243 / 033-259 (a key spelling one of the five is data, and quotes) |
| `BareKey` | §15.4.1.1 | 033-243 (identifier charset exactly) / 033-259 (and not a reserved spelling) |
| `QuotedKey` | §15.4.1.1 | 033-243 (STRING payload grammar with `'` as delimiter; same canonical emission) |
| `KeyRaw` | §15.4.1.1 | 033-243 |
| `KeyEscape` | §15.4.1.1 | 033-243 |
| `BID` | §15.4.4 | 033-167 / 033-254 (case-sensitive `B@` prefix; hex-run boundary rule) |
| `HexU32` | §14.6.3 | 033-167 |
| `PRIM` | §15.4.6 | 033-43 |
| `type_tag` | §15.4.6 | 033-43 / 033-79 / 033-255 (nesting bounded at 64 levels, one per bracketed constructor) |

The lexical laws that constrain no single production — **file intake** (033-254), **no comments** (033-246), **reserved spellings** (002-39 / 033-259), and **lexer/parser dot ownership** (033-253) — are stated in §2's preamble.

### §3 — Module text grammar

| Production | SPEC § | LOG |
|------------|--------|-----|
| `module` | §15.4.6 | 033-168 |
| `types_section` | §15.4.6 | 033-168 |
| `type_def` | §15.4.6 | 033-168 |
| `layout` | §15.4.6 | 033-168 |
| `field_list` | §15.4.6 | 033-168 |
| `variant` | §15.4.6 | 033-168 |
| `graph_section` | §15.4.6 | 033-168 / 033-256 (`scope*` — an empty graph is structurally legal) / 033-260 (entry order is semantic — the evaluation-order tiebreak's carrier) |
| `scope` | §15.4.6 | 033-168 / 033-75 |
| `entry` | §15.4.6 | 033-168 |
| `cell` | §15.4.6 | 033-169 / 033-215 / 033-81 / 033-245 |
| `cell_kind` | §15.4.6 | 033-80 |
| `member_set` | §15.4.6 | 033-81 |
| `member` | §15.4.6 | 033-81 |
| `member_driver` | §15.4.6 | 033-81 |
| `gate` | §15.4.6 | 033-168 / 033-65 |
| `wire` | §15.4.6 | 033-168 / 033-86 / 033-87 / 033-88 / 033-89 / 033-258 (a bare `null` destination is the literal, by reservation) |
| `effect` | §15.4.6 | 033-168 / 033-119 / 033-120 / 033-121 / 033-122 / 033-123 |
| `stream` | §15.4.6 | 033-106 / 033-107 / 033-108 / 033-109 / 033-110 / 033-111 / 033-112 / 033-114 / 033-115 / 033-117 |
| `behaviors_section` | §15.4.6 | 033-168 |
| `path_set` | §15.4.6 | 033-168 |
| `binding_set` | §15.4.6 | 033-168 |
| `reopen_set` | §15.4.6 | 033-75 |
| `lookback_map` | §15.4.6 | 033-117 (last `':' INT` of an entry is the lookback value) |
| `binding` | §15.4.6 | 033-168 / 033-252 (optional `'@' provenance` clause) |
| `provenance` | §15.4.6 | 033-252 |
| `value` | §15.4.6 | 033-168 / 033-212 / 033-248 / 033-257 (canonical digit spelling, normalized textually) / 033-258 (a value word in a `binding`'s right-hand side is the literal, by reservation) |

Entry termination in the `types`/`graph`/`behaviors` sections is NEWLINE (033-250), stated normatively in §3's preamble rather than repeated on every production.

### §4 — Behavior IR text grammar

| Production | SPEC § | LOG |
|------------|--------|-----|
| `behavior` | §15.4.4 | 033-205 |
| `params` | §15.4.4 | 033-206 / 033-203 |
| `block` | §15.4.4 | 033-207 |
| `instr` | §15.4.4 | 033-208 / 033-251 (`'%' (NAME \| INT)`) |
| `operand` | §15.4.4 | 033-209 / 033-251 |
| `terminator` | §15.4.4 | 033-210 / 033-251 |
| `paren_args` | §15.4.4 | 033-183 / 033-184 |
| `switch_table` | §15.4.4 | 033-185 |
| `switch_case` | §15.4.4 | 033-185 |
| `op` | §15.4.4 | 033-211 / 033-253 (dotted mnemonics composed by the parser) |
| `literal` | §15.4.4 | 033-212 / 033-248 |
| `LABEL` | §15.4.4 | 033-213 |
