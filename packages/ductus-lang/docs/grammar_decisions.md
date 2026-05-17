**Final decisions for GRAMMAR.md:**

**Notation:** ISO EBNF (ISO/IEC 14977).

- `=` for productions, `;` to terminate
- `,` for concatenation
- `|` for alternatives
- `[ X ]` for optional
- `{ X }` for zero-or-more repetition
- `( X )` for grouping
- `'literal'` or `"literal"` for terminals
- `(* comment *)` for comments
- `? prose ?` for special sequences requiring English explanation

**File structure:** single `GRAMMAR.md` with sections:

- §1 Notation (defines the meta-syntax)
- §2 Lexical grammar (tokens, whitespace, comments, identifiers, literals, INDENT/DEDENT emission rules)
- §3 Syntactic grammar (productions)
- §4 Operator precedence and associativity (encoded via grammar stratification)
- §5 Context-sensitive disambiguation (prose for cases the EBNF can't capture cleanly)

**Layering:** fine-grained productions — per-construct categories (`MatchExpr`, `IfExpr`, `BinaryExpr`, `MethodCall`,
etc.), not coarse umbrellas.

**INDENT/DEDENT:** explicit token classes emitted by the lexer; consumed in syntactic productions like any other
terminal.

**Operator precedence:** grammar stratification (Expr → OrExpr → AndExpr → ... → AtomExpr), each level encoding one
precedence rank.

**Cross-references:** GRAMMAR.md and SPEC.md are coupled; grammar productions cite SPEC sections for semantics; SPEC
sections cite GRAMMAR productions for syntax.

**Context-sensitivity:** §5 covers cases the formal grammar can't fully describe — literal-suffix tokenization (`100ms`
vs `100 ms`), flag-adjacency to TypeRef, `/expr` form, the underscore-suffix vs identifier-suffix split, INDENT/DEDENT
emission specifics.

**Format:** ISO EBNF + prose hybrid throughout. Productions are formal ISO EBNF; surrounding prose explains intent,
rationale, and context-sensitive bits.
