# Ductus IR Text Grammar

## 0. Introduction

This document specifies the **text form of the Ductus IR** — the intermediate representation that constitutes the contract between the Ductus frontend (lexer, parser, semantic analyzer) and the runtime/backend (loader, scheduler, behavior interpreter).

**Scope.** The text-form productions for:

- **Module text** — top-level module / type / graph IR (SPEC §15.4.6)
- **Behavior IR** — the per-behavior IR body (SPEC §15.4.4), with all productions defined and no ellipses
- **Lexeme definitions** — `NAME`, `INT`, `STRING`, `HEX`, `PATH`, `BID`, `type_tag`, `PRIM`

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

```
Nonterminal      ::= Alternative1
                   | Alternative2
                   ;  (§<spec-ref>, <log-ref>)

// Disambiguator note (when grammar alone is ambiguous):
// "<short rule explaining how the parser resolves the case>"
```

Each block carries a single source pointer of the form `;  (§N.M, NNN-MM)` citing the most-authoritative SPEC section and DECISION_LOG entry.

### Lexeme conventions

IR text is line-oriented but not layout-sensitive: indentation is purely cosmetic and does not synthesize tokens. Tokens are separated by whitespace; line breaks separate top-level statements. **Within a behavior block** (`behavior B@… { … }`), instructions may be separated either by physical line breaks *or* by an explicit `;` separator — both forms are normative (see `InstrSep` in §4). The two are interchangeable; the `;` form lets a single physical line hold multiple instructions (as the worked example in §5 demonstrates). All `NAME`, `INT`, `FLOAT`, `STRING`, `HEX`, `PATH`, `BID`, `PRIM`, and `type_tag` lexemes are defined in §2.

## 2. Lexeme definitions

The IR text form's terminals. `NAME`, `INT`, `FLOAT`, `STRING`, and `HEX` are the **obvious lexemes** (per SPEC §15.4.6): identifier, decimal integer literal, floating-point literal, double-quoted string literal, and hexadecimal digit sequence respectively. Their concrete syntax is the conventional one; the IR loader is expected to match the standard forms. The remaining lexemes — `PATH`, `BID`, `type_tag`, and `PRIM` — are defined formally below.

```
NAME             ::= /* identifier — letters, digits, '_'; not starting with a digit */
                   ;  (§15.4.6, 033-166)

INT              ::= /* decimal integer literal */
                   ;  (§15.4.6, 033-166)

FLOAT            ::= /* floating-point literal (used in behavior IR literals only) */
                   ;  (§15.4.4, 033-203)

STRING           ::= /* double-quoted string literal */
                   ;  (§15.4.6, 033-166)

HEX              ::= /* one or more hexadecimal digits [0-9a-fA-F] */
                   ;  (§15.4.4, 033-203)
```

A **PATH** is a cell or instance fully-qualified declaration path: a dot-separated sequence of identifiers naming the lexical nesting from module root through enclosing instances to the cell or instance name. Anonymous or duplicated sibling placements append an ordinal suffix `':' INT` (zero-based, declaration-order index among siblings of the same type at the same nesting depth).

```
PATH             ::= PathSegment ('.' PathSegment)*
                   ;  (§15.4.1.1, 033-166)

PathSegment      ::= NAME (':' INT)?
                   ;  (§15.4.1.1, 033-167)

// The ordinal suffix `:' INT` may attach to ANY path segment, not just
// the tail — e.g. 'App.print:0.text' has the ordinal on the middle
// component naming an anonymous effect instance. See 033-167/033-168.
// e.g. 'audio.synth_a.osc_1.frequency', 'App.print:0', 'App.print:0.text'
```

A **BID** is a behavior handle — the behavior's `u32` handle (§14.6.3) rendered in hexadecimal, prefixed by `'B@'`. It is the compact runtime reference; the behavior's wide content-addressed identity is not spelled in the text form.

```
BID              ::= 'B@' HexU32
                   ;  (§15.4.4, 033-203)

HexU32           ::= /* 1..8 hexadecimal digits — encodes a u32 behavior handle */
                   ;  (§14.6.3, 033-203)

// e.g. 'B@d1', 'B@aa10'. The hex run is bounded to ≤ 8 digits
// because the handle is a u32.
```

**PRIM** is the closed set of primitive type tags. These are the type-erased primitives the runtime understands directly (§4.1); aggregates and closures are built on top via `type_tag`.

```
PRIM             ::= 'i8'  | 'i16' | 'i32' | 'i64' | 'i128'
                   | 'u8'  | 'u16' | 'u32' | 'u64' | 'u128'
                   | 'isize' | 'usize'
                   | 'f32' | 'f64'
                   | 'bool' | 'str'
                   ;  (§15.4.6, 033-166)
```

**`type_tag`** is the unified type-tag nonterminal used throughout both the module and behavior grammars. It encodes a primitive, a named record/enum/tuple layout from the module's `types` section (`'%'NAME`), a pool-indexed aggregate (`pool_index<%TypeId>`), a tuple structural type, a fixed-size array, or a closure environment type.

```
type_tag         ::= PRIM
                   | '%' NAME
                   | 'pool_index' '<' '%' NAME '>'
                   | 'yielded' '<' type_tag '>'
                   | '(' type_tag (',' type_tag)* ')'
                   | '[' type_tag ';' INT ']'
                   | 'closure' '<' '(' (type_tag (',' type_tag)*)? ')' '->' type_tag '>'
                   ;  (§15.4.6, 033-166)

// An aggregate-valued cell — a record, enum, or tuple — is typed
// 'pool_index<%TypeId>' (§14.3.3), never an inline '%TypeId'.
// 'yielded<T>' is the ABI-only membership-descriptor type (§15.4.1,
// 033-79): it types synthesized membership-descriptor cells and
// 'yielded T' parameters, and is never a user-facing value type.
```

## 3. Module text grammar

The normative text form of the module-level IR (§15.4.1's abstract data model). Lifted verbatim from SPEC §15.4.6. `NAME`, `INT`, `FLOAT`, `STRING`, and `HEX` are the obvious lexemes from §2; `PATH` is the cell/instance path (§15.4.1.1); `BID` is the behavior handle (§15.4.4); `type_tag` is the type-erased graph tag (§15.4.3).

```
module           ::= 'module' NAME '{' types_section graph_section behaviors_section '}'
                   ;  (§15.4.6, 033-166)

types_section    ::= 'types' '{' type_def* '}'
                   ;  (§15.4.6, 033-166)

type_def         ::= '%' NAME '=' layout 'size' INT 'align' INT
                   ;  (§15.4.6, 033-166)

layout           ::= 'record' '{' field_list '}'
                   | 'enum'   '{' variant (',' variant)* '}'
                   | 'tuple'  '(' type_tag (',' type_tag)* ')'
                   ;  (§15.4.6, 033-166)

field_list       ::= (NAME ':' type_tag (',' NAME ':' type_tag)*)?
                   ;  (§15.4.6, 033-166)

variant          ::= '#' NAME ('(' type_tag (',' type_tag)* ')')?
                   ;  (§15.4.6, 033-166)

graph_section    ::= 'graph' '{' scope+ '}'
                   ;  (§15.4.6, 033-166)

scope            ::= 'scope' PATH 'exposes' path_set 'effects' path_set
                     ('reset_on_reopen' reopen_set)? '{' entry* '}'
                   ;  (§15.4.6, 033-166)

// 'reset_on_reopen' on a scope carries a reopen_set of (consumer : stream)
// pairs; this is one of three grammar positions for the keyword — see also
// the bare flag on the 'cell' production and the bare flag on the 'stream'
// production. All three spellings are normative (033-240).

entry            ::= cell | gate | connection | effect | stream
                   ;  (§15.4.6, 033-166)

cell             ::= cell_kind PATH ':' type_tag
                     ('uses' BID)? ('inputs' path_set)? ('depth' INT)?
                     ('reset_on_reopen')? ('init' value)? ('gate' PATH)?
                     ('combiner' BID)? ('else' value)? ('members' member_set)?
                   ;  (§15.4.6, 033-166)

// 'reset_on_reopen' on a cell is a bare flag (no payload) — one of three
// grammar positions for the keyword (033-240): the bare flag here on
// 'cell', the bare flag on 'stream', and the 'reopen_set' payload on
// 'scope'. 'depth' applies only to
// 'recurrent' cells; 'uses' / 'inputs' apply to 'derived' / 'recurrent';
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

cell_kind        ::= 'input' | 'derived' | 'recurrent' | 'fold'
                   ;  (§15.4.6, 033-166)

// The four cell-kind tags are closed (033-80): 'fold' is a cell KIND,
// not a seventh graph primitive, and no 'yielded' tag exists — a
// standalone 'yielded' group lowers to a membership-descriptor
// 'derived' cell (above), and a folded group is absorbed into its
// fold cell's 'members' set.

member_set       ::= '[' (member (',' member)*)? ']'
                   ;  (§15.4.6, 033-81)

member           ::= PATH ':' member_driver
                   ;  (§15.4.6, 033-81)

member_driver    ::= 'permanent' | 'keyed-template' | 'gate-guarded'
                   ;  (§15.4.6, 033-81)

// A 'members' set lists the member edges in member order (walk order
// for a membership-descriptor cell), each PATH naming the member cell
// and each tag naming its membership driver — 'permanent' (a static
// position), 'keyed-template' (a repeat-driven position, present per
// key), or 'gate-guarded' (a gated-arm position, present iff its gate
// is effectively active).
//
// **Boundary ambiguity note.** A 'member' pair separator is ':' and a
// PATH segment may itself end with a "(':' INT)?" ordinal suffix. The
// disambiguator matches reopen_set's: after a colon, an INT is the
// left-hand PATH's ordinal segment; a NAME-led token that is one of
// the three driver keywords closes the pair.

gate             ::= 'gate' PATH 'pred' BID 'inputs' path_set 'guards' path_set
                     ('gate_parent' PATH)?
                   ;  (§15.4.6, 033-166)

connection       ::= 'connection' PATH 'from' PATH 'to' (PATH | 'null')
                     ('type' type_tag)? ('attrs' binding_set)? ('gate' PATH)?
                   ;  (§15.4.6, 033-166)

effect           ::= 'effect' PATH 'reconciler' STRING 'params' binding_set
                     ('desired' path_set)? ('observed' path_set)? ('gate' PATH)?
                   ;  (§15.4.6, 033-166)

stream           ::= 'stream' PATH ':' type_tag 'policy' ('ring' | 'gate') 'capacity' INT
                     ('source_deps' path_set)? ('observes' path_set)?
                     ('reset_on_reload')? ('reset_on_reopen')?
                     ('history' INT)? ('lookback' lookback_map)?
                   ;  (§15.4.6, 033-106)

// A 'stream' line serializes the stream primitive (§15.4.1's Stream
// cells) with its ten data-model fields: PATH is the stream's id;
// type_tag the element_type; 'policy' is 'ring' or 'gate'; 'capacity'
// the integer capacity; 'source_deps' the source_dependencies; 'observes'
// the six synthesized observation-cell ids (pending_count, pressure,
// is_full, dropped_total, rejected_total, last_overflow_at); the two
// bare flags 'reset_on_reload' and 'reset_on_reopen' mark the matching
// @-annotations ('reset_on_reopen' here is the third grammar position of
// the keyword — a bare flag on 'stream', alongside the bare flag on
// 'cell' and the reopen_set on 'scope', 033-240); 'history' is
// output_history_size (present only on a recurrent[N] stream); and
// 'lookback' is the input_lookback_map (input cell id → max lookback k).

behaviors_section ::= 'behaviors' '{' behavior* '}'
                   ;  (§15.4.6, 033-166)

// 'behavior' production defined in §4 (per SPEC §15.4.4).
// **Module-resolvability constraint.** An empty `behaviors {}` is
// syntactically well-formed but renders any non-empty `graph_section`
// unresolvable — every `derived`/`recurrent` cell's `uses BID` and
// every `gate`'s `pred BID` reference a `behavior` by handle. The
// match is a load-time semantic check (post-parse), not a grammar
// rule; a module with `derived … uses B@xx` and no defining
// `behavior B@xx { … }` is rejected at module load.

path_set         ::= '[' (PATH (',' PATH)*)? ']'
                   ;  (§15.4.6, 033-166)

binding_set      ::= '[' (binding (',' binding)*)? ']'
                   ;  (§15.4.6, 033-166)

reopen_set       ::= '[' (PATH ':' PATH (',' PATH ':' PATH)*)? ']'
                   ;  (§15.4.6, 033-166)

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

binding          ::= NAME ':' (PATH | value)
                   ;  (§15.4.6, 033-166)

value            ::= INT | FLOAT | 'true' | 'false' | STRING
                   ;  (§15.4.6, 033-166)

// value is a compile-time literal (the placement_binding_kind is either
// 'static' (value literal) or 'reactive' (PATH referencing source cells);
// per §13.8.4 / §15.4.1).
```

## 4. Behavior IR text grammar

The text grammar for the per-behavior body — a pure, typed, SSA-with-block-arguments compute that the runtime invokes via `BID`. Lifted verbatim from SPEC §15.4.4 (post Phase-B formalization: every nonterminal below has a defining production; `type_tag` is used uniformly — the bare name `type` does not appear; `terminator`, `op`, `literal`, and `LABEL` are explicit productions).

```
behavior         ::= 'behavior' BID '(' params ')' '->' type_tag '{' block+ '}'
                   ;  (§15.4.4, 033-203)

params           ::= (('own')? NAME ':' type_tag) (',' ('own')? NAME ':' type_tag)*
                   ;  (§15.4.4, 033-203)

// A param without 'own' is a borrow (read, not consumed); 'own' marks
// it as consumed. The entry block's parameters are the behavior's
// parameters; a branch supplies its successor block's arguments
// (this replaces phi).

block            ::= LABEL ('(' params ')')? ':' ( instr InstrSep )* terminator
                   ;  (§15.4.4, 033-203)

InstrSep         ::= ';' | NEWLINE                            // intra-block separator
                   ;  (§15.4.4, 033-203)

// Multiple instructions on one line are separated by `;`; one
// instruction per line uses an implicit NEWLINE. The two are
// interchangeable per the worked example (§5). NEWLINE here is a
// physical line terminator (the IR text form is line-oriented, but
// `;` overrides the line shape).

instr            ::= '%' NAME '=' instr_rhs (':' type_tag)?  // typed-result form
                   | instr_rhs                                // void/effect-result form
                   ;  (§15.4.4, 033-203)

instr_rhs        ::= op_application
                   | call_rhs                                 // 'call' f(args), 'call.dyn' obj #method(args)
                   | closure_rhs                              // 'closure.make' BID 'captures' '{' ... '}'
                   | enum_make_rhs                            // 'enum.make' %T #V(p)
                   | array_make_rhs                           // 'array.make' '[' operand_list? ']'
                   | record_make_rhs                          // 'record.make' %T '{' field_inits? '}'
                   | raw_alloc_rhs                            // 'raw_alloc' '<' type_tag '>' '(' operand ')'
                   ;  (§15.4.4, 033-203)

op_application   ::= op ( operand (',' operand)* )?           // comma-separated operands
                   ;  (§15.4.4, 033-203)

call_rhs         ::= 'call' callee '(' operand_list? ')'
                   | 'call.dyn' '%' NAME '#' NAME '(' operand_list? ')'
                   ;  (§15.4.4, 033-203)

callee           ::= NAME                                    // direct call by behavior NAME (frontend-resolved)
                   | BID                                     // direct call by BID
                   ;  (§15.4.4, 033-203)

closure_rhs      ::= 'closure.make' BID 'captures' '{' capture_list? '}'
                   ;  (§15.4.4, 033-203)

capture_list     ::= NAME ':' operand (',' NAME ':' operand)*
                   ;  (§15.4.4, 033-203)

enum_make_rhs    ::= 'enum.make' '%' NAME '#' NAME ( '(' operand_list? ')' )?
                   ;  (§15.4.4, 033-203)

array_make_rhs   ::= 'array.make' '[' operand_list? ']'
                   ;  (§15.4.4, 033-203)

record_make_rhs  ::= 'record.make' '%' NAME '{' field_init_list? '}'
                   ;  (§15.4.4, 033-203)

field_init_list  ::= NAME ':' operand (',' NAME ':' operand)*
                   ;  (§15.4.4, 033-203)

raw_alloc_rhs    ::= 'raw_alloc' '<' type_tag '>' '(' operand ')'
                   ;  (§15.4.4, 033-203)

operand_list     ::= operand (',' operand)*
                   ;  (§15.4.4, 033-203)

operand          ::= '%' NAME                                // SSA result reference
                   | NAME                                    // bare param name (entry-block parameter)
                   | 'move' '%' NAME
                   | 'move' NAME                             // move of a bare param
                   | literal
                   ;  (§15.4.4, 033-203)

terminator       ::= 'br' LABEL paren_args?
                   | 'cond_br' '%' NAME ',' LABEL paren_args? ',' LABEL paren_args?
                   | 'switch' '%' NAME switch_table
                   | 'ret' '%' NAME
                   | 'trap' STRING
                   ;  (§15.4.4, 033-203)

// Every block ends in exactly one terminator. 'match' lowers to
// 'enum.tag' + 'switch'. 'trap' is the only non-'ret' exit
// (§13.13.1).

paren_args       ::= '(' operand (',' operand)* ')'
                   ;  (§15.4.4, 033-203)

switch_table     ::= '[' switch_case (',' switch_case)* ',' 'default' ':' LABEL ']'
                   ;  (§15.4.4, 033-203)

switch_case      ::= INT ':' LABEL
                   ;  (§15.4.4, 033-203)

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
                   ;  (§15.4.4, 033-203)

ArithOp          ::= 'add' | 'sub' | 'mul' | 'div' | 'rem' | 'neg'
                   ;  (§15.4.4, 033-203)

LogicOp          ::= 'and' | 'or' | 'xor' | 'shl' | 'shr' | 'not'
                   ;  (§15.4.4, 033-203)

CompareOp        ::= 'eq' | 'ne' | 'lt' | 'le' | 'gt' | 'ge'
                   ;  (§15.4.4, 033-203)

// All arithmetic / logical / comparison ops are PRIM-suffixed at the
// instruction site (`add.i32`, `not.bool`); a bare mnemonic with no
// PRIM suffix is a parse error. `const.PRIM` and `cast.SRC.DST` are
// the two distinct dotted forms with explicit suffix counts. The
// composite RHS forms `call`, `call.dyn`, `closure.make`,
// `enum.make`, `array.make`, `record.make`, and `raw_alloc` use
// dedicated `instr_rhs` alternatives (above) rather than the bare
// `op` enumeration here.

literal          ::= INT | FLOAT | 'true' | 'false' | STRING
                   ;  (§15.4.4, 033-203)

// 'literal' matches the same lexemes as the module grammar's 'value',
// with FLOAT additionally permitted for floating-point constants.

LABEL            ::= 'bb' INT
                   ;  (§15.4.4, 033-203)

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
// vs `'%' NAME` / op keyword) disambiguates.
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

```
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
      effect App.print:0  reconciler "print"  params [message: App.label]
                          desired [App.print:0.text, App.print:0.request]  gate App.g0
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
| `NAME` | §15.4.6 | 033-166 |
| `INT` | §15.4.6 | 033-166 |
| `FLOAT` | §15.4.4 | 033-203 |
| `STRING` | §15.4.6 | 033-166 |
| `HEX` | §15.4.4 | 033-203 |
| `PATH` | §15.4.1.1 | 033-166 / 033-167 / 033-168 (mid-path ordinal for anonymous effect instances; see worked example `App.print:0.text`) |
| `BID` | §15.4.4 | 033-203 |
| `PRIM` | §15.4.6 | 033-166 |
| `type_tag` | §15.4.6 | 033-166 / 033-79 |

### §3 — Module text grammar

| Production | SPEC § | LOG |
|------------|--------|-----|
| `module` | §15.4.6 | 033-166 |
| `types_section` | §15.4.6 | 033-166 |
| `type_def` | §15.4.6 | 033-166 |
| `layout` | §15.4.6 | 033-166 |
| `field_list` | §15.4.6 | 033-166 |
| `variant` | §15.4.6 | 033-166 |
| `graph_section` | §15.4.6 | 033-166 |
| `scope` | §15.4.6 | 033-166 / 033-240 |
| `entry` | §15.4.6 | 033-166 |
| `cell` | §15.4.6 | 033-166 / 033-240 / 033-81 |
| `cell_kind` | §15.4.6 | 033-166 / 033-80 |
| `member_set` | §15.4.6 | 033-81 |
| `member` | §15.4.6 | 033-81 |
| `member_driver` | §15.4.6 | 033-81 |
| `gate` | §15.4.6 | 033-166 |
| `connection` | §15.4.6 | 033-166 |
| `effect` | §15.4.6 | 033-166 |
| `stream` | §15.4.6 | 033-106 |
| `behaviors_section` | §15.4.6 | 033-166 |
| `path_set` | §15.4.6 | 033-166 |
| `binding_set` | §15.4.6 | 033-166 |
| `reopen_set` | §15.4.6 | 033-166 |
| `lookback_map` | §15.4.6 | 033-117 |
| `binding` | §15.4.6 | 033-166 |
| `value` | §15.4.6 | 033-166 |

### §4 — Behavior IR text grammar

| Production | SPEC § | LOG |
|------------|--------|-----|
| `behavior` | §15.4.4 | 033-203 |
| `params` | §15.4.4 | 033-203 |
| `block` | §15.4.4 | 033-203 |
| `instr` | §15.4.4 | 033-203 |
| `operand` | §15.4.4 | 033-203 |
| `terminator` | §15.4.4 | 033-203 |
| `paren_args` | §15.4.4 | 033-203 |
| `switch_table` | §15.4.4 | 033-203 |
| `switch_case` | §15.4.4 | 033-203 |
| `op` | §15.4.4 | 033-203 |
| `literal` | §15.4.4 | 033-203 |
| `LABEL` | §15.4.4 | 033-203 |
