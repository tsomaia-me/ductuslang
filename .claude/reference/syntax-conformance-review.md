# Cold review: the lexer and the parser against SPEC.md, at `6491d802`

Reviewer: one agent on Fable at high effort, given the code under `crates/compiler/src/syntax/`, the SPEC whole and the goal, with no hints. The text below is its report, word for word, saved by the peer session on 2026-11-13 because the reviewer could not write files. Its probes and case files are under `/home/cdagent/cargo-targets/peer-syntax-review/conformance/` (`cases/`, `tools/`, and `packages/symphonyscript/crates/compiler/examples/{probe,ast_probe,depends}.rs`).

---

# Lexer and parser against SPEC.md: cold conformance review

- **Artifact:** `crates/compiler/src/syntax/` at commit `6491d802`, private copy under `/home/cdagent/cargo-targets/peer-syntax-review/conformance/packages/symphonyscript/`.
- **Authority:** `docs/SPEC.md` of the docs worktree, read whole (1,971 lines). The copy inside the archive is byte-identical to it.
- **Vocabulary:** `symphonyscript_fixtures::vocabulary()` unless a line says otherwise.

## Verdict

Not as they stand. The foundations hold: on about 2,700 programs, every layout sentence of §2.2 I could test, every lexical bullet of §2.3 to §2.6, the whole precedence table and JSON mode behaved as the SPEC says, and the tree was lossless and well nested every time, mutants included. But the pair does not accept exactly the language: twelve families of programs the grammar accepts are refused, one of them a SPEC `sym` example, and accessors on five typed views return a wrong or ambiguous part for valid programs. Each finding below was reproduced by a run. I am confident in them, and less confident that the list is complete, because I ran one to a few programs per alternative, not combinations.

## Findings

Each program is in the named file under `cases/`. "The probe" is `crates/compiler/examples/probe.rs`. Line numbers are in `crates/compiler/src/syntax/`. Every finding is verified by run unless it says otherwise.

### A. A valid program refused

**A1. A wire placement without a destination; a SPEC `sym` example fails.**
- **SPEC §8.1:** `WirePlacement ::= Kind Flags? ('as' Name)? DefaultArg? Gate? Attrs? (':' Destination)?`, and "A wire placement omits its destination only as the single placement of a pattern that stands in for it (§6.4)."
- **Program** (fourth `sym` example of §6.4, SPEC line 968):
  ```
  pattern Cap(ceiling = 0.5) -> Drives:
      Drives | limit=ceiling
  ```
- **Code did:** `wire-syntax 2:27 expected ':' and the wire's destination, found the end of the line`. A bare `Triggers` line in a pattern fails the same way.
- **SPEC requires:** no diagnostic. The example is listed in `pending-examples.txt`; of the 50 `sym` examples it is the only one the parser refuses.
- **Code:** `parser/placements.rs:68` (`wire_after_attrs` demands the colon). Cases: `spec_sym`, `suspects1`.

**A2. A use of a pattern that stands in for a wire is read as a node placement.**
- **SPEC §7.1:** "A line whose `Kind` names a wire kind of the vocabulary, or a pattern that stands in for a wire (§6.4), is a wire placement (§8.1)". §6.4: "the use writes it after the colon, as a wire placement does (§8.1)".
- **Programs**, each inside `Scene night:`:
  - `Soft | depth=0.4: stage.wash`
  - `Soft: if flash_on then flash else dusk`
  - `Soft: [night, dawn, noon][look]`
  - `Soft: match look:` with arms
- **Code did**, in the same order:
  - `inline-placement … found '.'`
  - `placement-syntax expected a placement, found 'if'`
  - `bundle-syntax expected ']', found ','`
  - `placement-syntax … found 'match'`
- **Wrong tree when it does parse:** only a destination that is one bare name survives. The SPEC's own `Cap as choke | ceiling=0.3: feed` gives `Placement[Cap as choke Attrs Body[: Block[Members[Placement[feed]]]]]`: an inline child named `feed`, with no `WirePlacement` and no `Destination`.
- **SPEC requires:** a `WirePlacement` with a `Destination` that is any `Expr` or a `MatchBlock`.
- **Code:** `parser/blocks.rs:92` (`body_item` asks only `at_wire_kind`, `parser/mod.rs:546`, which reads the vocabulary). See E1. Cases: `suspects1`.

**A3. A qualified name as a kind.**
- **SPEC §7.1:** `Kind ::= Name | QualifiedName`. §15.3: "A qualified name stands wherever a name refers to a top-level declaration".
- **Programs:**
  - `gradients::Sweep dusk /0`
  - `Rig stage:` / `    util::Blink as b`
  - `Rig stage: util::Blink util::Blink`
  - `util::Soft: dusk` in a body
- **Code did:** `placement-syntax 1:10 expected the top-level instance's name, found '::'`; in a body, `expected the end of the line, found '::'`.
- **Code:** `parser/placements.rs:24`, `:41`, `:228` bump one `Ident` as the kind. Cases: `suspects1`.

**A4. A generic type with one argument.**
- **SPEC §6.8:** `TypeArgs ::= '[' TypeArg (',' TypeArg)* ']'`, `TypeArg ::= Type | Expr`; "In a type, the first bracket list after a generic type's name is its arguments".
- **Programs and what the code did:**

  | Program | Diagnostic |
  |---|---|
  | `fn f[T](b: Box[T?]) -> i32: 1` | `type-syntax expected ']', found '?'` |
  | `fn f(b: Box[Circle \| Square]) -> i32: 1` | `type-syntax expected ']', found '\|'` |
  | `fn f(b: Box[Pair[i32, bool]]) -> i32: 1` | `conversion-syntax expected '(' after the conversion's type, found ','` |
  | `fn f(b: Box[(i32?)[..4]]) -> i32: 1` | `conversion-syntax … found '?'` |

- **Wrong tree when it does parse:** `Box[T]` and `Box[util::Band]` are accepted as a dimension, `TypeRef[Box [ Size[NameRef[T]] ]]`, not as `TypeArgs`.
- **Code:** `parser/types.rs:248` (`type_args_optional` takes the list for arguments only when its first token is a type keyword or `{`, or the list holds a top-level comma). See E2. Cases: `suspects2`.

**A5. A keyword as an attribute name after the first entry, written bare or with `=>`.**
- **SPEC §2.5:** "A keyword stands as a member name where nothing but a name can stand: after `|` in an attribute clause (§7.6) … so a vocabulary may name an attribute `channel` or `out`". §7.6: `AttrEntry ::= Name '=' Expr | Name '=>' … | Name | '!' Name`.
- **Programs:** `Rig r:` / `    Lamp | level=1 out => c`; `    Lamp | level=1 channel`; the same entry on a continuation line.
- **Code did:** `placement-syntax expected the end of the line, found 'out'` (and `'channel'`). All 46 keywords fail in both forms (92 cases). All 46 pass as a first entry, after `!`, and with `=`.
- **Code:** `parser/mod.rs:891` (`at_attr_entry`: a keyword counts only when `first` or followed by `=`). Cases: `kw_member`, `suspects1`.

**A6. A member named `json` switches the lexer to JSON text; so does a dimension after the type `json`.**
- **SPEC §2.5:** a keyword is a member name "after `.` in a read (§5.6)". §3.2: `Base ::= (… | JsonType | …) Dims?`, `Size ::= Expr`.
- **Programs and what the code did:**

  | Program | Diagnostic |
  |---|---|
  | `state a = r.json["k"]` | `expression-syntax expected an expression, found '"k"'` (the string was lexed as `JsonString`) |
  | `state a = r.json - 0x10` | `json-syntax 'x10' directly follows a number in JSON text` |
  | `state a = r.json - 2bars` | the same for `bars` |
  | `fn f[const N](a: json[N + 0x2]) -> i32: 1` | the same for `x2` |

- `r.json[0]` parses, but the index token has kind `JsonNumber`.
- **Code:** `lexer/mod.rs:964` sets `json_next` on every word `json`, whatever precedes it; `:679` then reads `[`, `"`, digits and `-` as JSON. Cases: `suspects1`.

**A7. Layout suspension ends at a line that starts with `let`, `state`, `stream`, `channel`, `fn` or `pattern` at or left of the enclosing indentation.**
- **SPEC §2.2:** "Inside `( )`, `[ ]` and the braces of a record, map or document literal (§3.8, §3.10, §3.11), layout is suspended: newlines are whitespace and no layout tokens are emitted."
- **Programs:**
  ```
  let r = Config {
  state: 1,
  channel: 2
  }
  ```
  ```
  pattern P(
  state s,
  stream t
  ) -> Lamp:
      Lamp
  ```
  ```
  Rig r:
      (Lamp | level=1
      channel=2)
  ```
- **Code did:** the first two programs give four diagnostics each and the third gives two (`record-syntax expected a field's name, found 'state'`, `cell-syntax …`, `unexpected-character '}'`). The first two parse clean once the continuation lines are indented deeper.
- **Code:** `lexer/mod.rs:463` and `:1312`–`:1335` (`ends_region_here`, `starts_with_item_keyword`): a recovery rule that also fires in valid programs. Cases: `suspects2`.

**A8. An unparenthesized atom that is a qualified name with a path.**
- **SPEC §7.5:** `Atom ::= … | (Name | QualifiedName) ('.' Name)* | …`. §6.6: `Clock ::= 'on' (Atom | …)`.
- **Programs:** `Rig r:` / `    Lamp/util::base.low`; `state q5 on lib::c.d = 1`.
- **Code did:** `placement-syntax expected the end of the line, found '.'`; `cell-syntax expected '(' and the init, or '=' and the rule, found '.'`. The `=>` path and the JSON path take the same form correctly.
- **Code:** `parser/placements.rs:344` (the `::` branch of `atom` reads no `.Name`). Cases: `suspects1`, `g_decls`.

**A9. A record literal naming its type by a qualified name with arguments.**
- **SPEC §3.8:** `RecordLit ::= (Name | QualifiedName) TypeArgs? '{' Fields '}'`.
- **Program:** `let s = util::Stack[i32, 8] {}`.
- **Code did:** `conversion-syntax 1:24 expected '(' after the conversion's type, found ','` (read as an index of `util::Stack`).
- **Code:** `parser/decls.rs:104` (`at_record_literal`: the `::` branch looks for `{` directly after the name). Cases: `suspects1`.

**A10. The right operand of `is`.**
- **SPEC §5.1:** `CmpExpr ::= Fallback (CmpOp Fallback | 'is' 'not'? AltType)?`, `AltType ::= 'none' | (… | (Name | QualifiedName) TypeArgs?) Dims?`.
- **Programs:** `state t = x is Pair[i32, bool]`; `let v = x is {"a": 1}["a"]`.
- **Code did:** `conversion-syntax … found ','` (the bracket list was parsed as an index, as §5.3 says for a name; see E3); `type-syntax expected a type, found '"'` (a brace group followed by `[` is taken for a map type).
- **Code:** `parser/expressions.rs:170`–`:177`, `:216`. The same `Pair[i32, bool]` is accepted as a match label (`parser/blocks.rs:308`). Cases: `suspects2`, `context`.

**A11. A top-level placement whose kind is named `input`.**
- **SPEC §7.1:** `TopPlacement ::= Kind Flags? 'as'? Name …`; `input` is no keyword and no reserved word (§2.5).
- **Program:**
  ```
  pattern input() -> Rig:
      Rig:
          Lamp

  input stage
  ```
- **Code did:** `top-level-syntax 5:1 expected a declaration or a placement, found 'input'`, with the help "write `state <name>(<init>)`". `input as stage` parses.
- **Code:** `parser/items.rs:38`–`:49`. Cases: `suspects1`.

**A12. Two narrower refusals.**
- `Rig r:` / `    Lamp | a=v is json b=2`: `json b` is taken for a document literal (`parser/expressions.rs:200`).
- `let a: Band[{1: 2}.size] = x`: a `{` first in a bracket list forces type arguments (`parser/types.rs:253`).
- Cases: `package`.

### B. An invalid program accepted

**B1. `?` after an `AltType`.**
- **SPEC §5.1:** `AltType ::= 'none' | (…) Dims?`, with no `?`.
- **Programs:** `state t = x is i32?`; a match arm `i32?: 1`.
- **Code did:** no diagnostic.
- **Code:** `parser/types.rs:210` (`base_tail`, shared with `Base`). Cases: `suspects2`.

**B2. Placements and attribute entries with no whitespace between them.**
- **SPEC §7.7:** "Placements on one line are separated by whitespace". §7.6: "entries are separated by whitespace".
- **Programs**, in a body: `Lamp(Lamp)`, `(Lamp)(Lamp)`, `[Lamp][Lamp]`, `Lamp/(1)Lamp`, `Lamp/"a"Lamp`, `Lamp | a="x"b=2`, `Lamp | strobe!strobe`, `Lamp | a=(1)b=2`.
- **Code did:** no diagnostic; two placements or two entries. The productions do not express whitespace, so only the prose refuses these.
- **Code:** `parser/placements.rs:173`, `:412`. Cases: `adjacent`.

### C. A wrong tree or a wrong accessor for a valid program

**C1. `TypeRef::lengths()` and `TypeRef::capacity()`** (`ast/type_ref.rs:42`, `:47`).
- **SPEC §3.2:** "`i32[3][..10]` holds up to 10 rows of 3, and `i32[..4][..10]` up to 10 rows of up to 4 elements".
- **Output of `ast_probe`** (cases `ast`, `ast2`):

  | Type | `lengths()` | `capacity()` | Wrong because |
  |---|---|---|---|
  | `i32[3][..10]` | `[3]` | `10` | same answer as the next row |
  | `i32[..10][3]` | `[3]` | `10` | a different type, same answer |
  | `i32[..4][..10]` | `[10]` | `4` | the second capacity is reported as a length |
  | `T[..N][3]` | `[]` | `3` | `3` is a length; the capacity is `N` |
  | `T[N][..8]` | `[]` | `8` | the length `N` is absent |
  | `json[2]` | `[]` | none | the `2` was lexed as `JsonNumber` (A6) |
  | `json[2][3]` | `[3]` | none | the first dimension is lost |

- The accessors read bare `Number` tokens only and keep no order; a size that is not plain digits is a `Size` node they do not see.
- `base()` and `enum_name()` return nothing for `lib::Band[2]?`.

**C2. `Lambda::params()`** (`ast/lambda.rs:8`).
- **SPEC §6.7:** `Lambda ::= Name ':' Expr | '(' LambdaParams? ')' Result? ':' Expr`.
- `x: 1` and `(): 1` both return `None`. No accessor returns the bare parameter or the result type.

**C3. `ArgList::positional()` and `NamedArg::value()`** (`ast/arg_list.rs:22`, `ast/named_arg.rs:25`).
- **SPEC §5.1:** `Arg ::= Expr | Lambda`.
- For `f((p: i32) -> bool: p < 48, xs)`, `positional()` is `[xs]` and `lambdas()` is the lambda: the argument positions are lost.
- For `f(keep=p: p, xs=ys)`, `NamedArg::value()` of `keep` is `None`.

**C4. `AttrEntry`** (`ast/attr_entry.rs:29`).
- **SPEC §7.6** separates `Name '=' Expr` from `Name '=>' path`.
- `hit => hit` and `gain=gain` both answer `is_negated=false`, a name and a value. Nothing tells `=>` from `=`.

**C5. `PlacementEntry::count()`** (`ast/placement_entry.rs:11`).
- **SPEC §6.4:** `('+' | '*' | '{' Count '}')?`.
- `Lamp+`, `Lamp*` and `Lamp` all return `None`; only the brace form returns a node. No accessor returns the entry's name.

**C6. Tree shapes that differ from the production**, each seen in a run:
- A match label that is a bare name is `NameRef`, not `AltType` (`parser/blocks.rs:325`). SPEC §5.4: `MatchArm ::= (AltType | '-'? NumberLit | BoolLit | StringLit) ':' …`.
- A size of plain digits is a bare token in `TypeRef`; any other size is a `Size` node (`parser/types.rs:228`). `i32[16]` and `i32[0x10]` have different shapes.
- A qualified name in an expression, an atom, a `=>` path or a JSON path is a `VariantRef`; in a type it is a `QualifiedName`.
- The wire-pattern use of A2 and the dimension reading of `Box[T]` of A4.

### D. Layout (§2.2)

A7 is the only sentence I found broken. The rest were verified by run and behave as the SPEC says:
- the stack, and one `DEDENT` per popped column;
- a dedent that lands on no stacked column (`dedent-mismatch`);
- `NEWLINE` on a last line with no line break, and the `DEDENT`s at end of input;
- a tab in leading whitespace (`tab-indentation`), and tabs between tokens;
- CRLF, in comments and strings too;
- columns in scalar values: a clause continued after `日本`, after `Cafe` + U+0301, and after U+20000;
- blank and comment-only lines at any indentation;
- suspension in `( )`, `[ ]`, and record, map and document braces;
- the clause continuation at exactly the first entry's column, across comment lines and a multi-line bracket, closed by `:`, not by `::` or by a `:` inside brackets;
- the wide-character line (`clause-column`, "its first entry is at column 18 … show at display column 20");
- the semicolon: an error also inside an interpolation; text inside a string, a JSON string and a comment.

### E. SPEC gaps and contradictions

- **E1.** §7.1 makes a line a wire placement when its kind is "a pattern that stands in for a wire". The pattern may be in another file, the prelude or a library, and §8.2 destinations are not `InlineChildren`. The SPEC gives no reading a parser of one file can produce.
- **E2.** §6.8: whether a type's first bracket list is arguments or dimensions depends on whether the name is generic, which no production states and one file cannot know. `Box[T]` is `TypeArgs` or `Dims` by the same tokens.
- **E3.** §5.3 says a right operand of `is` that is "a name or a qualified name, with any bracketed suffix or without, parses as a `Fallback`"; §5.1's `AltType` holds `Name TypeArgs?`. `x is Pair[i32, bool]` is an `AltType` and cannot be a `Fallback`. For a match label the SPEC states no such rule, and a bare name is both an `AltType` and a variant.
- **E4.** §5.1 has `Expr ::= IfExpr | ForExpr | OrExpr`, so the grammar accepts a `for` as any `Expr`; §5.10 requires parentheses outside four positions. The code follows §5.10: a `for` as a lambda body, an `if` branch, a match arm, the body of another `for`, or the last line of a block form is refused (verified by run).
- **E5.** §8.1's production lets any wire omit its destination; the prose allows it only in a stand-in pattern. The parser refuses both (A1).
- **E6.** §2.5 says "after `|` in an attribute clause"; whether that covers entries after the first is not said (A5). `Lamp | level=x or=1` has two derivations; the code reads a keyword followed by `=` as a name (`parser/expressions.rs:656`), which the SPEC does not state.
- **E7.** After `is`, and in a type, `json[2]` is both the type `json` with a dimension and a document literal `json [2]`. The code reads `x is json[2]` as the literal and `a: json[2]` as the type (verified by run). `x is {…}[…]` has the same double reading (A10).
- **E8.** §2.2 does not say whether a tab is an error in the leading whitespace of a blank line, a comment-only line, or a continuation line inside brackets. The code accepts all three (verified by run).
- **E9.** §2.2 names "the braces of a record, map or document literal"; the braces of a map type `{K: V}[..N]` and of a count `{2+}` are not named. The code suspends layout in every brace.
- **E10.** §3.11 `JsonChar` excludes "a control character" without a range; the code refuses below U+0020 only (read, not run above U+001F). A byte order mark and a lone CR outside a string are not mentioned; both are `unexpected-character` (verified by run).
- **E11.** §2.6 does not say whether `5f64` is "an integer literal" for the 2¹²⁷ limit. The code applies the limit to every literal without a point or an exponent, whatever its suffix (verified by run).
- **E12.** §15.2's `PackageFile` and §2.1's `File` are separate grammars, but there is one `parse` entry with no mode. `require`, `use`, `publish` and a top-level record literal parse in any file, and declarations parse in a package file; the SPEC's errors for these fall to a later stage.
- **E13.** The `sym-error` of §7.9 ("a range binds one name") is decidable from the tokens, but `LoopHead` admits two names over any `Iterable`; the parser accepts it. The `sym-error` of §6.7 (`state g(x: x + 1)`) is refused by the grammar itself (`Init ::= '(' Expr ')'`) and by the parser, with a reason of position.

### F. Tests of the module

No test under `src/syntax/` states a tree for a production, a layout token sequence, or a precedence. The module's 14 non-ignored tests pass in the copy (`cargo test -p symphonyscript --lib -- syntax::`: 14 passed, 0 failed, 9 ignored stress tests, exit 0).

| Sort | Files | What they check |
|---|---|---|
| Follows from the SPEC | `parser/tests/valid_items.rs` | every non-pending `sym` example, the four fixture projects and 13 written items parse with no diagnostic; absence of diagnostics only |
| Follows from the SPEC | `ast/tests.rs` | one test: a hand-built tree read through a handful of accessors, with a name returned in NFC |
| Hand-written, but about recovery the SPEC does not define | `parser/tests/fault_catalogue.rs`, `parser/tests/single_faults.rs`, `parser/tests/repeated_faults.rs` (ignored) | one fault gives one diagnostic at a marked span |
| A model of the implementation used as its own oracle | `parser/tests/line_shape.rs`, `parser/tests/generated_file.rs` | `line_shape.rs` restates the recovery rules of `starts_line.rs` and `ends_region_here` (the six keywords of A7 among them); `generated_file.rs` computes the expected diagnostics from it |
| Output copied from the implementation | `front_end_goldens.rs` | digests of the HIR and placeholders of the stage after the parser, compared with recorded files |
| Cost and memory, nothing of the language | `lexer/tests.rs`, `cost_tests.rs`, `parser/tests/kind_reads.rs` | work and storage bounds |

Outside the module, `crates/compiler/tests/productions.rs` holds one hand-written tree per production, not per alternative. Its `Kind` case has no qualified name, its `TypeArgs` cases start with a type keyword, and its `WirePlacement` case has a destination. I read only its head and those cases.

### G. Anything else

- **What a parse result depends on.** The text and the vocabulary. `Source.path` is copied into diagnostic spans and changes nothing else. Over 2,623 programs, parsing twice gave equal results every time, and another path changed no tree and no diagnostic.
- **Vocabulary facts read:**
  - unit names (`lexer/mod.rs:1094`, a suffix);
  - wire kind names (`parser/mod.rs:546`, the choice of `WirePlacement` and two diagnostics);
  - node kind names (`parser/starts_line.rs:59`, recovery only).
  - Against an empty vocabulary, 196 of the 2,623 trees changed.
- **Hash order.** `Parser.groups` is a `HashMap` with a fixed hasher; it is read by key, written and cleared, never iterated. Diagnostics are sorted by a total key. No output depends on hash order (read; run only as the repeat parse).
- **Imports from `semantics`:**
  - `counted_walk::CountedWalk` (lexer, tree builder, parser, `node_view`, `child_pool`; the counters exist only under `cfg(test)`);
  - `analysis::fast_hash::FastMap` (`parser/mod.rs:60`);
  - `reporting::order` (`parser/mod.rs:145`);
  - in tests, `front_end_goldens.rs` also imports `semantics::ir`.
- **`lex` against the tokens of `parse`.** On every clean parse they are equal, except that a keyword in a member position is renamed to `Ident` in the tree. On 8 of 852 mutants (all with diagnostics) the tree's tokens differ structurally from `lex`'s, because the parser re-lexes during recovery.
- **Column base.** `dedent-mismatch` reports columns from 0 ("indentation, 4 … at 0"); `clause-column` reports from 1.
- **Missing accessors:** `RecordLit` (type name, arguments); `TypeRef` (qualified name, arguments, `Size` nodes, the nested `StrType`, `JsonType`, `MapType`, `ParenType`); `MapType` (capacity); `Use`, `UseName`, `Require`, `PublishName`, `Count`.
- **Host rule slip.** One probe run of a few milliseconds was started without the `taskset` prefix by mistake; every other program and every cargo command carried it.

## Coverage

Programs are the cases in the named files under `cases/`. Counts per area are approximate to within a few where one file serves several areas; file totals are exact.

| Area | SPEC | Programs | File |
|---|---|---|---|
| SPEC examples: `sym`, `sym-package`, `sym-error` | all | 50, 2, 19 | `spec_*` |
| File, top items | §2.1 | 9 | `g_decls` |
| Layout | §2.2 | 64 | `layout`, `layout2`, `misc2` |
| Comments, identifiers, NFC | §2.3, §2.4 | 14 | `lexical`, `ast2` |
| Each keyword as a name | §2.5 | 46 | `kw_name` |
| Each keyword and reserved word in member positions | §2.5 | 735 | `kw_member` |
| Keywords in 16 other positions | §2.5 | 192 | `kw_elsewhere` |
| Reserved words, `null`, `==`, `!=`, `^` | §2.5 | 18 | `lexical` |
| Number and string literals | §2.6 | 46 | `lexical`, `esc`, `esc2` |
| Types, generics | §3.2, §6.8 | 40 | `g_expr`, `g_decls`, `suspects2` |
| `type`, record literal | §3.8 | 18 | `g_decls`, `g_expr` |
| Map type and literal | §3.10 | 10 | `g_expr`, `layout` |
| Documents, JSON mode | §3.11 | 94 | `json`, `misc`, `misc2`, `suspects1` |
| Precedence and associativity | §5.1 | 53 | `precedence` |
| Primary, postfix, arguments, conversion | §5.1, §5.5, §5.8 | 35 | `g_expr` |
| `match` | §5.4 | 24 | `g_place`, `suspects2` |
| `for` expression | §5.10 | 29 | `g_expr`, `suspects2`, `context` |
| Cells, clocks | §6.1, §6.6 | 26 | `g_decls` |
| `let`, lambdas | §6.2, §6.7 | 19 | `g_decls`, `g_expr` |
| Functions | §6.3 | 21 | `g_decls` |
| Patterns, placement types | §6.4 | 24 | `g_decls` |
| Placements, lines, bodies | §7.1–§7.3, §7.7 | 33 | `g_place` |
| Flags, default attribute, attribute clause | §7.4–§7.6 | 51 | `g_place`, `adjacent` |
| Bundles, placement `for` | §7.8, §7.9 | 23 | `g_place` |
| Wires, `to` | §8 | 23 | `g_place`, `suspects1`, `context` |
| `when` | §9 | 13 | `g_place` |
| Package file | §15.2–§15.5 | 20 | `package` |
| Context-dependent readings | §5.1, §5.3, §6.8, §7.1 | 33 | `context` |
| Typed views, 85 node kinds probed | `syntax/ast/` | 16 | `ast`, `ast2` |
| Lossless and nesting check on mutants of the examples | tree | 852 | `mutants` |
| Deep nesting (20,000 brackets, 3,000 blocks) | tree | 5 | `deep` |
| Dependence: repeat, path, empty vocabulary | item 9 | 2,623 × 4 parses | every file |

The byte-for-byte, one-token-per-byte and nesting checks ran on every program above, valid or not, with no fault.

## Not covered

- **Other vocabularies.** Only the example and the empty vocabulary were used. A5 was shown with attribute names the example vocabulary does not declare, which the parser does not consult.
- **Combinations of alternatives.** Each alternative was run once or a few times, usually alone.
- **§4 and §10 to §14.** They hold no `ebnf`; nothing was run for them.
- **`ARCHITECTURE.md`.** Not read; nothing here is judged against it.
- **Diagnostic wording and recovery quality**, beyond what a finding quotes.
- **Crate-level tests** under `crates/compiler/tests/`, beyond the head of `productions.rs` and the test names of `lexer.rs`.
- **The ignored stress tests** of the module were not run.
- **Unicode breadth.** A handful of scalars only: no check of every XID boundary, of normalization forms other than one NFD name, or of the display-width table.
