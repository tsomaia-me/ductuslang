use std::collections::BTreeSet;

use symphonyscript::{Code, Compiler, Diagnostic, Severity, Source};
use symphonyscript_core::{Count, Vocabulary};
use symphonyscript_fixtures::{SPEC, vocabulary};

#[derive(Clone, Copy)]
enum Setup {
    Example,
    Prelude,
    Bounded,
}

struct Rule {
    section: &'static str,
    sentence: &'static str,
    code: &'static str,
    setup: Setup,
    program: &'static str,
}

const fn rule(
    section: &'static str,
    sentence: &'static str,
    code: &'static str,
    program: &'static str,
) -> Rule {
    Rule {
        section,
        sentence,
        code,
        setup: Setup::Example,
        program,
    }
}

const fn bounded(
    section: &'static str,
    sentence: &'static str,
    code: &'static str,
    program: &'static str,
) -> Rule {
    Rule {
        section,
        sentence,
        code,
        setup: Setup::Bounded,
        program,
    }
}

const fn prelude(
    section: &'static str,
    sentence: &'static str,
    code: &'static str,
    program: &'static str,
) -> Rule {
    Rule {
        section,
        sentence,
        code,
        setup: Setup::Prelude,
        program,
    }
}

const RULES: &[Rule] = &[
    rule(
        "2.1",
        "File ::= TopItem*",
        "unexpected-character",
        "let a = 1 & 2\n",
    ),
    rule(
        "2.1",
        "TopItem ::= InputDecl | LetDecl | FnDecl | PatternDecl | TopPlacement",
        "top-level-syntax",
        "5\n",
    ),
    rule(
        "2.2",
        "a tab in leading whitespace is an error",
        "tab-indentation",
        "Clip c:\n\tNote\n",
    ),
    rule(
        "2.2",
        "must land exactly on a column still on the stack",
        "dedent-mismatch",
        "Clip c:\n    Note\n  Note\n",
    ),
    rule(
        "2.2",
        "a line that misses the column only by the display width of wide characters is an error whose diagnostic shows the column",
        "clause-column",
        "Clip 音楽 | gain=1\n            level=2\n",
    ),
    rule(
        "2.2",
        "A semicolon outside strings and comments is an error.",
        "semicolon",
        "let a = 1;\n",
    ),
    rule(
        "2.2",
        "A line indented deeper than the top of the stack pushes its column and emits `INDENT`.",
        "unexpected-indent",
        "input a = 1\n    input b = 2\n",
    ),
    rule(
        "2.4",
        "holds at least one character other than `_` and `#`",
        "invalid-identifier",
        "let _ = 1\n",
    ),
    rule(
        "2.5",
        "The words `from`, `recurrent` and `string` are reserved and are never names.",
        "reserved-word",
        "let from = 1\n",
    ),
    rule(
        "2.5",
        "The tokens `^`, `==` and `!=` are reserved and rejected by the lexer",
        "reserved-token",
        "let a = 1 == 2\n",
    ),
    rule(
        "2.6",
        "A single underscore may separate two digits: `1_000_000`. Anywhere else it is an error.",
        "misplaced-underscore",
        "let a = 10_\n",
    ),
    rule(
        "2.6",
        "A float needs digits on both sides of the point: `0.5`, not `.5`.",
        "float-digits",
        "let a = .5\n",
    ),
    rule(
        "2.6",
        "An integer type suffix on a float literal, as in `0.5i32`, is an error.",
        "float-type-suffix",
        "let a = 0.5i32\n",
    ),
    rule(
        "2.6",
        "Any other suffix is an error.",
        "unknown-suffix",
        "let a = 2zz\n",
    ),
    rule(
        "2.6",
        "A prefixed literal takes no suffix, and a letter or digit directly after its digits is an error",
        "prefixed-suffix",
        "let a = 0b12\n",
    ),
    rule(
        "2.6",
        "An integer literal whose magnitude is 2¹²⁷ or more is an error (§4.6).",
        "integer-too-large",
        "let a = 170141183460469231731687303715884105728\n",
    ),
    rule(
        "2.6",
        "A string occupies one line.",
        "unterminated-string",
        "let s = \"abc\n",
    ),
    rule(
        "2.6",
        "Escape ::= '\\\"' | '\\\\' | '\\n' | '\\t' | '\\{'",
        "invalid-escape",
        "let s = \"a\\q\"\n",
    ),
    rule(
        "3.2",
        "Type ::= (ScalarType | Name) ('[' Digits ']')*",
        "type-syntax",
        "input a: i32[x] = [1]\n",
    ),
    rule(
        "3.2",
        "A `Name` in a type is a vocabulary enum.",
        "unknown-type",
        "input a: Shape = 1\n",
    ),
    rule(
        "3.2",
        "A length in a type is at least 1.",
        "zero-length",
        "input a: i32[0] = [1]\n",
    ),
    rule(
        "3.2",
        "Elements may themselves be arrays of equal length",
        "ragged-array",
        "let a = [1, [2]]\n",
    ),
    rule(
        "3.3",
        "No input, cell or runtime value is a string",
        "input-value",
        "input s = \"text\"\n",
    ),
    rule(
        "3.3",
        "a string expression that reads a cell is an error",
        "reactive-string",
        "input x = 0.5\nKit k:\n    Sampler | file=\"{x}\"\n",
    ),
    rule(
        "3.5",
        "They are never stored in an input, a reactive `let` or an attribute.",
        "input-value",
        "input n = c\nClip c:\n    Note\n",
    ),
    rule(
        "3.5",
        "They are never stored in an input, a reactive `let` or an attribute.",
        "type-mismatch",
        "Clip c:\n    Note | velocity=c\n",
    ),
    rule(
        "3.6",
        "A unit value bound to an attribute of a different family, or of no family, is an error.",
        "unit-mismatch",
        "Clip c:\n    Note | length=2s\n",
    ),
    rule(
        "3.6",
        "A unit value bound to an attribute of a different family, or of no family, is an error.",
        "unit-mismatch",
        "Clip c:\n    Note | velocity=2bars\n",
    ),
    rule(
        "3.6",
        "`+`, `-`, `%`, comparisons, `min`, `max`, `clamp`, the first two arguments of `lerp`, array literals, and the arms of `if` and `match` take operands of one family.",
        "mixed-families",
        "Synth s:\n    Osc | freq=2s + 1bars\n",
    ),
    rule(
        "3.6",
        "Where the family value reaches no attribute of the family, as when `/` or `div` makes it a plain number or a comparison a `bool`, the plain value is an error",
        "plain-in-family",
        "input d = 5ms\nClip c:\n    Note when d > 1\n",
    ),
    rule(
        "3.6",
        "Where the family value reaches no attribute of the family, as when `/` or `div` makes it a plain number or a comparison a `bool`, the plain value is an error",
        "plain-in-family",
        "Clip c:\n    Note | length=((2bars + 1) div 1bars)\n",
    ),
    bounded(
        "3.6",
        "and so is a family value that reaches two attributes of the family with different default units",
        "conflicting-units",
        "let d = 2bars + 1\nA one | long=d\nB two | short=d\nRoot r:\n    Item\n    Link: r\n",
    ),
    rule(
        "3.6",
        "Dividing a plain number by a unit value, multiplying two unit values, `**` on a unit value, and a unit value as a range bound are errors.",
        "unit-operation",
        "Clip c:\n    Note | length=2bars * 2bars\n",
    ),
    rule(
        "3.6",
        "Dividing a plain number by a unit value, multiplying two unit values, `**` on a unit value, and a unit value as a range bound are errors.",
        "unit-operation",
        "Clip c:\n    Note | length=(1 / 1bars)\n",
    ),
    rule(
        "3.6",
        "Dividing a plain number by a unit value, multiplying two unit values, `**` on a unit value, and a unit value as a range bound are errors.",
        "unit-operation",
        "Clip c:\n    Note | length=(2bars ** 2)\n",
    ),
    rule(
        "3.6",
        "Dividing a plain number by a unit value, multiplying two unit values, `**` on a unit value, and a unit value as a range bound are errors.",
        "unit-operation",
        "Clip c:\n    for i in 0..2bars:\n        Note\n",
    ),
    bounded(
        "3.6",
        "a plain cell read at two attributes of one family with different default units is an error naming both",
        "conflicting-units",
        "input p = 1.0\nA one | long=p\nB two | short=p\nRoot r:\n    Item\n    Link: r\n",
    ),
    rule(
        "4.1",
        "the value must fit the chosen type",
        "does-not-fit",
        "Clip melody:\n    Note/300000000000\n",
    ),
    rule(
        "4.1",
        "wherever a literal or an integer result is converted to a float type",
        "does-not-fit",
        "Clip c | gain=(16777217 / 2)\n",
    ),
    rule(
        "4.1",
        "where a fixed-type operand gives a literal its type",
        "does-not-fit",
        "Clip c:\n    Note/(5i32 + 3000000000)\n",
    ),
    rule(
        "4.1",
        "otherwise it is an error naming both uses",
        "inconsistent-uses",
        "let q = 2 ** -1\nSynth s:\n    Osc/q\nClip c:\n    Note/q\n",
    ),
    rule(
        "4.2",
        "and any other fixed-type operand is an error",
        "type-mismatch",
        "Clip c:\n    Note | pitch=1i64\n",
    ),
    rule(
        "4.3",
        "With no such type, the expression is an error.",
        "type-mismatch",
        "input b = true\nlet x = if b then 1 else false\n",
    ),
    rule(
        "4.4",
        "A float input used where an integer is expected is an error",
        "float-to-integer",
        "input cutoff = 0.5\nClip c:\n    Note/cutoff\n",
    ),
    rule(
        "4.4",
        "The initial value must fit every use.",
        "does-not-fit",
        "input big = 3000000000\nClip c:\n    Note/big\n",
    ),
    rule(
        "4.4",
        "An integer input whose every use expects a float draws a warning",
        "integer-input",
        "input n = 4\nClip c:\n    Note | velocity=n\n",
    ),
    rule(
        "4.4",
        "A float `let` used where an integer is expected is an error.",
        "float-to-integer",
        "input f = 0.5\nlet g = f * 2\nClip c:\n    Note/g\n",
    ),
    rule(
        "4.6",
        "a fixed-type integer operation that saturates is an error",
        "constant-overflow",
        "let x = 2147483647i32 + 1\nClip c:\n    Note/x\n",
    ),
    rule(
        "4.6",
        "A flexible integer whose magnitude reaches 2¹²⁷, positive or negative, is an error.",
        "integer-overflow",
        "Clip c:\n    Note/(2 ** 200)\n",
    ),
    rule(
        "4.6",
        "A flexible integer whose magnitude reaches 2¹²⁷, positive or negative, is an error.",
        "integer-overflow",
        "Clip c:\n    Note/(-(2 ** 126) * 2 div 2 ** 120)\n",
    ),
    rule(
        "4.6",
        "a value that holds the call and has different shapes at two of its uses is an error naming both",
        "inconsistent-shapes",
        "fn r(x, n): if n < 1 then (if x + 0.2 is 0.3 then [x] else [x, x]) else r(x, n - 1)\nlet v = r(0.1, 3)\nClip c | gain=(v[0])\nOsc o | freq=(v[0])\n",
    ),
    rule(
        "4.6",
        "The bounds of a range in a `for`, and an index known at compile time, are integers that fit `i64`.",
        "range-bound",
        "Clip c:\n    for i in 0..0.5:\n        Note\n",
    ),
    rule(
        "4.6",
        "The bounds of a range in a `for`, and an index known at compile time, are integers that fit `i64`.",
        "index-out-of-range",
        "Clip c:\n    Note/([1, 2][2 ** 70])\n",
    ),
    rule(
        "4.6",
        "an operation they do not allow, such as `div` on a float, is an error naming the call",
        "operand-type",
        "fn f(x): if x < 1 then x div 2 else f(x - 1)\nClip c:\n    Note/(f(0.5))\n",
    ),
    rule(
        "4.6",
        "and yields at most 1,000,000 instances and wires",
        "expansion-limit",
        "Clip c:\n    for i in 0..100001:\n        Note Note Note Note Note Note Note Note Note Note\n",
    ),
    rule(
        "4.6",
        "its expansion runs at most 1,000,000 `for` iterations and pattern uses",
        "expansion-limit",
        "Clip c:\n    for i in 0..1000000000000:\n        for j in 0..0:\n            Note\n",
    ),
    rule(
        "4.6",
        "Exceeding a limit is an error naming the call or the expansion.",
        "expansion-limit",
        "pattern P():\n    for j in 0..0:\n        Note\nClip c:\n    for i in 0..500001:\n        P\n",
    ),
    rule(
        "4.6",
        "inlines at most 1,000,000 calls of other functions",
        "inline-limit",
        "fn h0(x): x\nfn h1(x): h0(x) + h0(x) + h0(x) + h0(x) + h0(x) + h0(x) + h0(x) + h0(x) + h0(x) + h0(x)\nfn h2(x): h1(x) + h1(x) + h1(x) + h1(x) + h1(x) + h1(x) + h1(x) + h1(x) + h1(x) + h1(x)\nfn h3(x): h2(x) + h2(x) + h2(x) + h2(x) + h2(x) + h2(x) + h2(x) + h2(x) + h2(x) + h2(x)\nfn h4(x): h3(x) + h3(x) + h3(x) + h3(x) + h3(x) + h3(x) + h3(x) + h3(x) + h3(x) + h3(x)\nfn h5(x): h4(x) + h4(x) + h4(x) + h4(x) + h4(x) + h4(x) + h4(x) + h4(x) + h4(x) + h4(x)\nfn h6(x): h5(x) + h5(x) + h5(x) + h5(x) + h5(x) + h5(x) + h5(x) + h5(x) + h5(x) + h5(x)\nClip k:\n    Note/(h6(1))\n",
    ),
    rule(
        "4.6",
        "A compilation evaluates at most 1,000,000 distinct recursive calls",
        "recursion-limit",
        "fn f(a, b, c): if a < 1 then 0 else f(a - 1, b + 1, c) + f(a - 1, b, c + 1) + f(a - 1, b, c)\nClip k:\n    Note/(f(200, 0, 0))\n",
    ),
    rule(
        "5.1",
        "Primary ::= NumberLit | BoolLit | StringLit | Name | Name '::' Name",
        "expression-syntax",
        "let a = (1 +)\n",
    ),
    rule(
        "5.1",
        "Comparisons do not chain: `a < b < c` is an error.",
        "chained-comparison",
        "let a = 1 < 2 < 3\n",
    ),
    rule(
        "5.1",
        "An `if` that is an operand of another operator is parenthesized",
        "unparenthesized-if",
        "let a = 1 + if true then 1 else 2\n",
    ),
    rule(
        "5.2",
        "`div` takes integers",
        "operand-type",
        "let a = 1 div 0.5\n",
    ),
    rule(
        "5.3",
        "`and`, `or` and `not` take and yield `bool`.",
        "operand-type",
        "let a = 1 and true\n",
    ),
    rule(
        "5.4",
        "The condition is a `bool`.",
        "condition-type",
        "let a = if 1 then 2 else 3\n",
    ),
    rule(
        "5.4",
        "MatchBlock ::= 'match' Expr ':' NEWLINE INDENT MatchArm+ ElseArm? DEDENT",
        "match-syntax",
        "input b = 1\nlet a = match b\n",
    ),
    rule(
        "5.4",
        "A scrutinee of another type is an error.",
        "match-scrutinee",
        "let l = match 1.5:\n    1: 2\n    else: 1\n",
    ),
    rule(
        "5.4",
        "Arm labels are variant names of the scrutinee's enum, or literals of its type and unit family.",
        "match-label",
        "input w = Wave::Saw\nlet l = match w:\n    Purple: 1\n    else: 2\n",
    ),
    rule(
        "5.4",
        "Arm labels are variant names of the scrutinee's enum, or literals of its type and unit family.",
        "match-label",
        "input n = 1\nlet l = match n:\n    1.5: 1\n    else: 2\n",
    ),
    rule(
        "5.4",
        "Arm labels are variant names of the scrutinee's enum, or literals of its type and unit family.",
        "match-label",
        "input n = 1\nlet l = match n:\n    2bars: 1\n    else: 2\n",
    ),
    rule(
        "5.4",
        "A label that repeats an earlier one is an error.",
        "duplicate-arm",
        "input w = Wave::Saw\nlet l = match w:\n    Sine: 1\n    Sine: 2\n    else: 3\n",
    ),
    rule(
        "5.4",
        "A `match` on an enum or a `bool` covers every value or ends with `else:`",
        "match-incomplete",
        "input w = Wave::Saw\nlet l = match w:\n    Sine: 1\n",
    ),
    rule(
        "5.4",
        "an `else:` after arms that cover every value is an error",
        "redundant-else",
        "input b = true\nlet l = match b:\n    true: 1\n    false: 2\n    else: 3\n",
    ),
    rule(
        "5.4",
        "A `match` on an integer ends with `else:`.",
        "match-incomplete",
        "input n = 1\nlet l = match n:\n    1: 2\n",
    ),
    rule(
        "5.4",
        "`match` is the whole value of a `let`, top-level or local, of a function body or of a wire destination (§8.2), never an operand",
        "match-position",
        "let a = 1 + match 1:\n    else: 2\n",
    ),
    rule(
        "5.5",
        "The index is an integer of either width",
        "index-type",
        "let a = [1, 2][true]\n",
    ),
    rule(
        "5.5",
        "`a[i]` reads element `i`, counting from 0.",
        "not-an-array",
        "let a = 1[0]\n",
    ),
    rule(
        "5.5",
        "A compile-time index outside the array is an error.",
        "index-out-of-range",
        "Clip c:\n    Note/([1, 2, 3][5])\n",
    ),
    rule(
        "5.5",
        "Operators apply to scalars only. `[1, 2] + [3, 4]` is an error.",
        "array-operand",
        "let a = [1, 2] + [3, 4]\n",
    ),
    rule(
        "5.6",
        "A path whose start is chosen at runtime is an error",
        "runtime-path",
        "input i = 0\nlet s = [verse, chorus]\nClip verse | gain=s[i].gain:\n    Note\nClip chorus:\n    Note\n",
    ),
    rule(
        "5.6",
        "Reading an attribute the kind does not declare is an error.",
        "unknown-attribute",
        "Clip c | gain=c.nothing:\n    Note\n",
    ),
    rule(
        "5.6",
        "Reads that form a cycle are an error naming the cycle.",
        "declaration-cycle",
        "Clip a | gain=b.gain:\n    Note\nClip b | gain=a.gain:\n    Note\n",
    ),
    rule(
        "5.7",
        "A call invokes a built-in, a prelude function or a source function (§6.3).",
        "not-callable",
        "let a = 1(2)\n",
    ),
    rule(
        "5.7",
        "mixing them in one call is an error",
        "mixed-arguments",
        "fn f(a, b): a\nlet a = f(1, b=2)\n",
    ),
    rule(
        "5.7",
        "Built-ins take positional arguments only.",
        "named-builtin-arguments",
        "let a = sin(x=1)\n",
    ),
    rule(
        "5.7",
        "`lerp(a, b, t)`",
        "argument-count",
        "let a = lerp(1, 2)\n",
    ),
    rule(
        "5.8",
        "Conversion ::= ('i32' | 'i64' | 'f32' | 'f64') '(' Expr ')'",
        "conversion-syntax",
        "let a = i32 1\n",
    ),
    rule(
        "5.8",
        "Converting a `bool` or an enum value is an error.",
        "conversion-type",
        "let a = f32(true)\n",
    ),
    rule(
        "5.8",
        "Converting a `bool` or an enum value is an error.",
        "conversion-type",
        "let a = f32(Wave::Saw)\n",
    ),
    rule(
        "5.9",
        "`{expr}` inside a literal interpolates a compile-time value",
        "interpolation-type",
        "Kit k:\n    Sampler | file=\"{k}\"\n",
    ),
    rule(
        "6.1",
        "InputDecl ::= 'input' Name (':' Type)? '=' Expr NEWLINE",
        "input-syntax",
        "input = 1\n",
    ),
    rule(
        "6.1",
        "Its initializer is a compile-time value and gives the input its type (§4.4).",
        "reactive-initializer",
        "input a = 1\ninput b = a\n",
    ),
    rule(
        "6.2",
        "LetDecl ::= 'let' Name (':' Type)? '=' (Expr NEWLINE | MatchBlock)",
        "let-syntax",
        "let = 1\n",
    ),
    rule(
        "6.2",
        "A declaration that reaches itself through `let`s, inputs, attribute reads or function bodies is an error naming the cycle",
        "declaration-cycle",
        "let a = b + 1\nlet b = a + 1\n",
    ),
    rule(
        "6.2",
        "A declaration that reaches itself through `let`s, inputs, attribute reads or function bodies is an error naming the cycle",
        "declaration-cycle",
        "fn f(): a\nlet a = f()\n",
    ),
    rule(
        "6.2",
        "A declaration that reaches itself through `let`s, inputs, attribute reads or function bodies is an error naming the cycle",
        "declaration-cycle",
        "input a = b\nlet b = a + 1\n",
    ),
    rule(
        "6.2",
        "A `let` holding node references that reads an input is an error",
        "reactive-node-reference",
        "input i = 0\nlet t = [verse, chorus][i]\nClip verse:\n    Note\nClip chorus:\n    Note\n",
    ),
    rule(
        "6.3",
        "FnDecl ::= 'fn' Name '(' Params? ')' ':' (Expr NEWLINE | NEWLINE INDENT FnBody DEDENT)",
        "fn-syntax",
        "fn (x): x\n",
    ),
    rule(
        "6.3",
        "Two parameters or two local `let`s with one name",
        "duplicate-local",
        "fn f(a, a): a\n",
    ),
    rule(
        "6.3",
        "Two parameters or two local `let`s with one name",
        "duplicate-local",
        "fn f(x):\n    let a = 1\n    let a = 2\n    a\n",
    ),
    rule(
        "6.3",
        "or a local `let` named like a parameter, are an error.",
        "duplicate-local",
        "fn f(x):\n    let x = 1\n    x\n",
    ),
    rule(
        "6.3",
        "A parameter with a default follows every parameter without one",
        "parameter-order",
        "fn f(a = 1, b): a + b\n",
    ),
    rule(
        "6.3",
        "A positional call may omit trailing parameters that have defaults",
        "argument-count",
        "fn f(x): x\nlet a = f(1, 2)\n",
    ),
    rule(
        "6.3",
        "A recursive function whose body reads a cell is an error, and so are a call to one with an argument that reads a cell",
        "reactive-recursion",
        "input i = 1\nfn f(n): if n < 1 then 0 else f(n - 1)\nClip c:\n    Note/(f(i))\n",
    ),
    rule(
        "6.3",
        "A recursive function whose body reads a cell is an error",
        "reactive-recursion",
        "input i = 1\nfn f(n): if n < 1 then i else f(n - 1)\nClip c:\n    Note/(f(3))\n",
    ),
    rule(
        "6.3",
        "evaluation deeper than 1,024 nested calls",
        "recursion-depth",
        "fn f(n): if n < 1 then 0 else f(n - 1) + 1\nClip c:\n    Note/(f(2000))\n",
    ),
    rule(
        "6.4",
        "PatternDecl ::= 'pattern' Name '(' PatternParams? ')' ':' NEWLINE INDENT LocalLet* BodyItem+ DEDENT",
        "pattern-syntax",
        "pattern (x):\n    Note\n",
    ),
    rule(
        "6.4",
        "Two parameters or two `let`s with one name",
        "duplicate-local",
        "pattern P(a, a):\n    Note/a\n",
    ),
    rule(
        "6.4",
        "Two parameters or two `let`s with one name",
        "duplicate-local",
        "pattern P():\n    let a = 1\n    let a = 2\n    Note/a\n",
    ),
    rule(
        "6.4",
        "or a `let` named like a parameter, are an error.",
        "duplicate-local",
        "pattern P(root):\n    let root = 1\n    Note/root\n",
    ),
    rule(
        "6.4",
        "`/expr` supplies the parameter marked `default`, of which there is at most one.",
        "duplicate-default-parameter",
        "pattern P(default a, default b):\n    Note/a\n",
    ),
    rule(
        "6.4",
        "`/expr` on a pattern without a `default` parameter is an error.",
        "no-default-parameter",
        "pattern P(root):\n    Note/root\nClip c:\n    P/1 | root=2\n",
    ),
    rule(
        "6.4",
        "Supplying a parameter twice is an error",
        "duplicate-argument",
        "pattern P(default root):\n    Note/root\nClip c:\n    P/1 | root=2\n",
    ),
    rule(
        "6.4",
        "and so is omitting a parameter that has no default value.",
        "missing-argument",
        "pattern P(root):\n    Note/root\nClip c:\n    P\n",
    ),
    rule(
        "6.4",
        "The attribute clause supplies parameters in every entry form of §7.6",
        "unknown-parameter",
        "pattern P(root = 1):\n    Note/root\nClip c:\n    P | octave=1\n",
    ),
    rule(
        "6.4",
        "A use carries no flags and no body.",
        "use-form",
        "pattern P():\n    Note\nClip c:\n    P'\n",
    ),
    rule(
        "6.4",
        "A use carries no flags and no body.",
        "use-form",
        "pattern P():\n    Note\nClip c:\n    P:\n        Note\n",
    ),
    rule(
        "6.4",
        "A pattern may use other patterns, but no pattern reaches itself through uses.",
        "recursive-pattern",
        "pattern A():\n    Clip:\n        A\n",
    ),
    rule(
        "6.4",
        "Only such uses appear at top level.",
        "top-level-pattern",
        "pattern P():\n    Note\n    Note\nP top\n",
    ),
    rule(
        "6.5",
        "A duplicate is an error naming both declarations",
        "duplicate-name",
        "input a = 1\nlet a = 2\n",
    ),
    rule(
        "6.5",
        "A declared name of any kind, top-level or local, that equals a variant of a vocabulary enum is an error.",
        "variant-name",
        "let Saw = 1\n",
    ),
    rule(
        "6.5",
        "A declared name of any kind, top-level or local, that equals a variant of a vocabulary enum is an error.",
        "variant-name",
        "fn f(x):\n    let Saw = x\n    Saw\n",
    ),
    rule(
        "6.5",
        "A child or wire name equal to a name of its own body or of an enclosing body, or to an attribute of its parent's kind, is an error",
        "child-name-clash",
        "Clip c:\n    Note as dup\n    Note as dup\n",
    ),
    rule(
        "6.5",
        "A child or wire name equal to a name of its own body or of an enclosing body, or to an attribute of its parent's kind, is an error",
        "child-name-clash",
        "Track t:\n    Synth as dup:\n        Osc as dup\n",
    ),
    rule(
        "6.5",
        "A child or wire name equal to a name of its own body or of an enclosing body, or to an attribute of its parent's kind, is an error",
        "child-name-clash",
        "Clip c:\n    Note as gain\n",
    ),
    rule(
        "6.5",
        "also when the two names stand in different arms of one `when` block",
        "child-name-clash",
        "input b = true\nClip c:\n    when b:\n        Note as dup\n    else:\n        Note as dup\n",
    ),
    rule(
        "6.5",
        "a `for` variable equal to a visible name is an error",
        "for-variable-clash",
        "Clip c:\n    for sin in 0..2:\n        Note\n",
    ),
    rule(
        "6.5",
        "A function or pattern that nothing uses has its names resolved, cycles among its `let`s included (§6.2)",
        "declaration-cycle",
        "fn f():\n    let a = b\n    let b = a\n    a\n",
    ),
    rule(
        "6.5",
        "Every other name resolves in scope.",
        "unknown-name",
        "let a = b\n",
    ),
    rule(
        "3.4",
        "Anywhere else it is qualified with the enum's name: `Wave::Saw`.",
        "unknown-name",
        "Clip c | gain=Saw(1)\n",
    ),
    rule(
        "7.1",
        "TopPlacement ::= Kind Flags? 'as'? Name DefaultArg? Gate? Attrs? (NEWLINE | Body)",
        "placement-syntax",
        "Clip\n",
    ),
    rule(
        "7.1",
        "a wire kind at top level is an error",
        "top-level-wire",
        "Plays p: c\n",
    ),
    rule(
        "7.4",
        "An identifier character directly after a flag run is an error",
        "flag-syntax",
        "Clip c:\n    Note'x\n",
    ),
    rule(
        "7.4",
        "The vocabulary assigns flag characters to attributes (§10.1).",
        "unknown-flag",
        "Clip c:\n    Note!\n",
    ),
    rule(
        "7.5",
        "Anything else is parenthesized: `Note/(base + 7)`.",
        "default-arg-syntax",
        "Clip c:\n    Note/a[0]\n",
    ),
    rule(
        "7.5",
        "`/expr` on a kind without a default attribute is an error.",
        "no-default-attribute",
        "Synth s:\n    Mixer/1\n",
    ),
    rule(
        "7.6",
        "AttrEntry ::= Name '=' Expr | Name | '!' Name",
        "attribute-syntax",
        "Clip c:\n    Note | =1\n",
    ),
    rule(
        "7.6",
        "Setting an attribute twice, by any combination of entries, flags and `/expr`, is an error.",
        "duplicate-attribute",
        "Clip c:\n    Note/60 | pitch=61\n",
    ),
    rule(
        "7.6",
        "Setting an attribute the kind does not declare is an error",
        "unknown-attribute",
        "Clip c:\n    Note | loud=1\n",
    ),
    rule(
        "7.6",
        "and so is leaving unset an attribute the vocabulary gives no default (§10.1).",
        "missing-attribute",
        "Kit k:\n    Sampler\n",
    ),
    rule(
        "7.7",
        "A placement with a gate or an attribute clause is parenthesized or written on its own line.",
        "inline-placement",
        "Clip c:\n    Note/60 Note | velocity=1\n",
    ),
    rule(
        "7.8",
        "Bundle ::= '[' (Member+ BundleFor? | BundleFor) ']'",
        "bundle-syntax",
        "Clip c:\n    [Note\n",
    ),
    rule(
        "7.8",
        "A bundle has at least one member after expansion.",
        "empty-bundle",
        "Clip c:\n    [for i in 0..0: Note]\n",
    ),
    rule(
        "7.8",
        "Wires and bundles are not members, and a member has no body, also when a pattern supplies it.",
        "bundle-member",
        "pattern P():\n    Plays: c\nClip c:\n    [P]\n",
    ),
    rule(
        "7.8",
        "Wires and bundles are not members, and a member has no body, also when a pattern supplies it.",
        "bundle-member",
        "pattern P():\n    [Note/62 Note/64]\nClip c:\n    [P]\n",
    ),
    rule(
        "7.8",
        "Wires and bundles are not members, and a member has no body, also when a pattern supplies it.",
        "bundle-member",
        "pattern Inner():\n    Synth:\n        Osc\nTrack t:\n    [Inner]\nSession s:\n    Includes: t\n",
    ),
    rule(
        "7.8",
        "Wires and bundles are not members, and a member has no body, also when a pattern supplies it.",
        "bundle-member",
        "pattern Inner():\n    Synth:\n        Osc\npattern Outer():\n    Inner\nTrack t:\n    [Outer]\nSession s:\n    Includes: t\n",
    ),
    rule(
        "7.8",
        "Wires and bundles are not members, and a member has no body, also when a pattern supplies it.",
        "misplaced-wire",
        "Clip c:\n    [Plays: c]\n",
    ),
    rule(
        "7.9",
        "ForBlock ::= 'for' Name 'in' Iterable ':' (NEWLINE INDENT BodyItem+ DEDENT | InlineChildren NEWLINE)",
        "for-syntax",
        "Clip c:\n    for in 0..2:\n        Note\n",
    ),
    rule(
        "7.9",
        "or an array of scalars, enum values, strings or node references",
        "iterable-type",
        "Clip c:\n    for i in 5:\n        Note\n",
    ),
    rule(
        "7.9",
        "An iterable that reads a cell is an error.",
        "reactive-iterable",
        "input x = 1\nClip c:\n    for i in [x, x]:\n        Note\n",
    ),
    rule(
        "7.9",
        "Placements in a `for` body are anonymous; `as` there is an error.",
        "named-in-for",
        "Clip c:\n    for i in 0..2:\n        Note as n\n",
    ),
    rule(
        "8.1",
        "WirePlacement ::= Kind Flags? ('as' Name)? DefaultArg? Gate? Attrs? ':' Destination",
        "wire-syntax",
        "Clip c:\n    Plays\n",
    ),
    rule(
        "8.2",
        "A destination is a node reference",
        "destination-type",
        "Clip c:\n    Plays: 1\n",
    ),
    rule(
        "8.2",
        "it contains no `to`",
        "misplaced-to",
        "Clip a:\n    Note\nClip c:\n    Plays: to.x\n",
    ),
    rule(
        "8.2",
        "It is never an inline placement or a pattern use written in the destination",
        "not-a-value",
        "pattern V():\n    Note\nClip c:\n    Plays: V\n",
    ),
    bounded(
        "8.3",
        "every candidate has `name` with the same type and unit family",
        "to-candidates",
        "input i = 0\nA one\nB two\nRoot r:\n    Item\n    Link: r\n    Pick | level=to.x: [one, two][i]\n",
    ),
    rule(
        "8.3",
        "`to.a.b`, `to` alone, and `to` written outside a wire placement are errors",
        "misplaced-to",
        "let a = to.gain\n",
    ),
    rule(
        "8.3",
        "`to.a.b`, `to` alone, and `to` written outside a wire placement are errors",
        "misplaced-to",
        "Clip a:\n    Note\nClip c:\n    Plays | gain=to.a.b: a\n",
    ),
    rule(
        "8.3",
        "`to.a.b`, `to` alone, and `to` written outside a wire placement are errors",
        "misplaced-to",
        "Clip c:\n    Plays | gain=to: c\n",
    ),
    rule(
        "8.4",
        "No cycle of wires contains a wire of a kind the vocabulary does not mark cyclic.",
        "wire-cycle",
        "Synth s:\n    Filter as b:\n        Feeds: c\n    Filter as c:\n        Feeds: b\n",
    ),
    rule(
        "9.1",
        "the instance is active only while the expression, a `bool`, is true",
        "condition-type",
        "Clip c:\n    Note when 1\n",
    ),
    rule(
        "9.2",
        "WhenBlock ::= 'when' Expr ':' Block ('else' ':' Block)?",
        "when-syntax",
        "Clip c:\n    when x\n        Note\n",
    ),
    prelude(
        "10.1",
        "It holds `let`, `fn` and `pattern` declarations only",
        "prelude-item",
        "input a = 1\n",
    ),
    prelude(
        "10.1",
        "the prelude fails to parse or to resolve its names",
        "unknown-name",
        "let a = b\n",
    ),
    rule(
        "10.2",
        "Every kind, wire kind, attribute, enum variant and unit exists in the vocabulary.",
        "unknown-kind",
        "Clip c:\n    Foo\n",
    ),
    rule(
        "10.2",
        "Every kind, wire kind, attribute, enum variant and unit exists in the vocabulary.",
        "unknown-variant",
        "let l = Wave::Purple\n",
    ),
    rule(
        "10.2",
        "Every top-level instance's kind may stand at top level",
        "not-top-level",
        "Note n\n",
    ),
    rule(
        "10.2",
        "and the number of each such kind is within its count",
        "count-exceeded",
        "Session a\nSession b\n",
    ),
    bounded(
        "10.2",
        "and the number of each such kind is within its count",
        "count-below-minimum",
        "",
    ),
    rule(
        "10.2",
        "Every child's kind is accepted by its parent",
        "child-not-accepted",
        "Clip c:\n    Osc\n",
    ),
    bounded(
        "10.2",
        "and each parent's children of each kind are within their counts",
        "count-exceeded",
        "Root r:\n    Item\n    Item\n    Item\n    Link: r\n",
    ),
    bounded(
        "10.2",
        "and each parent's children of each kind are within their counts",
        "count-below-minimum",
        "Root r:\n    Link: r\n",
    ),
    rule(
        "10.2",
        "Every wire's source and every candidate destination are kinds its wire kind connects",
        "wire-endpoint",
        "Track t:\n    Starts: t\n",
    ),
    bounded(
        "10.2",
        "and each instance's incoming and outgoing wires of each kind are within its counts",
        "count-exceeded",
        "Root r:\n    Item\n    Link: r\n    Link: r\n",
    ),
    bounded(
        "10.2",
        "and each instance's incoming and outgoing wires of each kind are within its counts",
        "count-below-minimum",
        "Root r:\n    Item\n",
    ),
    rule(
        "10.2",
        "A top-level instance that no entry-kind instance reaches, through children and through wires to any candidate, draws a warning.",
        "unreachable-instance",
        "Session s:\n    Track\nClip lonely:\n    Note\n",
    ),
    rule(
        "10.2",
        "A program with no entry-kind instance draws one warning that says so instead",
        "no-entry-instance",
        "Clip lonely:\n    Note\n",
    ),
    rule(
        "14",
        "No construct takes a function as a value.",
        "not-a-value",
        "let a = sin\n",
    ),
];

fn bounded_vocabulary() -> Vocabulary {
    let mut v = Vocabulary::builder("bounded");
    let beats = v.unit_family("beats").base("beats").unit("bars", 4.0);
    let root = v.node("Root");
    let item = v.node("Item");
    let first = v.node("A");
    let second = v.node("B");
    let link = v.wire("Link");
    let pick = v.wire("Pick");
    v.kind(root)
        .top_level(Count::Exactly(1))
        .entry()
        .children(item, Count::Between(1, 2))
        .outgoing(link, Count::AtMost(1))
        .incoming(link, Count::AtLeast(1));
    v.kind(item);
    v.kind(first)
        .top_level(Count::Any)
        .attr::<i32>("x")
        .default(0)
        .attr::<f64>("long")
        .default(0.0)
        .family(&beats, "bars");
    v.kind(second)
        .top_level(Count::Any)
        .attr::<f32>("x")
        .default(0.0)
        .attr::<f64>("short")
        .default(0.0)
        .family(&beats, "beats");
    v.wire_kind(link).connects(root, root).cyclic();
    v.wire_kind(pick)
        .connects(root, first)
        .connects(root, second)
        .attr::<f32>("level")
        .default(1.0);
    v.build().expect("the bounded vocabulary builds")
}

fn with_prelude(text: &str) -> Vocabulary {
    let mut v = Vocabulary::builder("preluded");
    let clip = v.node("Clip");
    v.kind(clip).top_level(Count::Any);
    v.prelude("prelude.sym", text);
    v.build().expect("the vocabulary builds")
}

fn diagnostics(rule: &Rule) -> Vec<Diagnostic> {
    let source = Source {
        path: "rule.sym".into(),
        text: rule.program.into(),
    };
    match rule.setup {
        Setup::Example => Compiler::new(&vocabulary())
            .expect("the example vocabulary compiles")
            .check(&[source]),
        Setup::Bounded => Compiler::new(&bounded_vocabulary())
            .expect("the bounded vocabulary compiles")
            .check(&[source]),
        Setup::Prelude => match Compiler::new(&with_prelude(rule.program)) {
            Ok(_) => Vec::new(),
            Err(failed) => failed.diagnostics,
        },
    }
}

fn normalized(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn section_text(section: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for line in SPEC.lines() {
        if let Some(heading) = line
            .strip_prefix("## ")
            .or_else(|| line.strip_prefix("### "))
        {
            let number = heading
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_end_matches('.');
            inside = number == section;
            continue;
        }
        if inside {
            out.push_str(line);
            out.push('\n');
        }
    }
    normalized(&out)
}

#[test]
fn a_count_an_unknown_placement_or_destination_could_change_draws_no_diagnostic() {
    let rows = [
        bounded(
            "10.2",
            "",
            "unknown-kind",
            "Root r:\n    Itme\n    Link: r\n",
        ),
        bounded(
            "10.2",
            "",
            "unknown-name",
            "Root r:\n    Item\n    Link: nosuch\n",
        ),
        bounded(
            "10.2",
            "",
            "unknown-kind",
            "Rot r:\n    Item\n    Link: r\n",
        ),
        bounded(
            "10.2",
            "",
            "unknown-name",
            "Root r:\n    for i in 0..nothing:\n        Item\n    Link: r\n",
        ),
        rule("10.2", "", "unknown-kind", "Clip c:\n    [Nte/60]\n"),
        rule(
            "10.2",
            "",
            "unknown-kind",
            "Session s:\n    Track\nClip lonely:\n    Nte:\n        Starts: lonely\n",
        ),
        bounded(
            "10.2",
            "",
            "unknown-kind",
            "pattern Back():\n    Link: r\nRoot r:\n    Item\n    Bak\n",
        ),
        rule(
            "10.2",
            "",
            "unknown-kind",
            "pattern Feed():\n    Includes: t\nSession s:\n    Fed\nTrack t:\n    Piano\n",
        ),
        bounded(
            "10.2",
            "",
            "unknown-kind",
            "Root r:\n    Item\n    Lnk: r\n",
        ),
    ];
    let mut failures = Vec::new();
    for rule in &rows {
        let found: Vec<&str> = diagnostics(rule)
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect();
        if found != [rule.code] {
            failures.push(format!("{:?} gives {found:?}", rule.program));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_rule_sentence_stands_in_its_section_of_the_spec() {
    for rule in RULES {
        let text = section_text(rule.section);
        assert!(
            text.contains(&normalized(rule.sentence)),
            "§{} does not hold {:?}",
            rule.section,
            rule.sentence
        );
    }
}

#[test]
fn every_code_appears_in_the_table_and_every_table_code_exists() {
    let names: BTreeSet<&str> = Code::ALL.iter().map(|code| code.as_str()).collect();
    let table: BTreeSet<&str> = RULES.iter().map(|rule| rule.code).collect();
    let missing: Vec<&&str> = names.difference(&table).collect();
    let unknown: Vec<&&str> = table.difference(&names).collect();
    assert!(
        missing.is_empty(),
        "codes missing from the table: {missing:?}"
    );
    assert!(
        unknown.is_empty(),
        "table codes that do not exist: {unknown:?}"
    );
    for code in Code::ALL {
        assert!(
            RULES
                .iter()
                .any(|rule| rule.code == code.as_str() && rule.section == code.section()),
            "{code}: no rule of §{} maps to it",
            code.section()
        );
    }
}

#[test]
fn every_rule_rejects_its_program_with_its_code() {
    let mut failures = Vec::new();
    for rule in RULES {
        let found = diagnostics(rule);
        if !found
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == rule.code)
        {
            failures.push(format!(
                "§{} {:?}: {:?} gives {:?}",
                rule.section,
                rule.code,
                rule.program,
                found
                    .iter()
                    .map(|diagnostic| diagnostic.code.as_str())
                    .collect::<Vec<_>>()
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_rule_program_holds_one_fault_and_draws_one_diagnostic_for_it() {
    let mut failures = Vec::new();
    for rule in RULES {
        let found = diagnostics(rule);
        let errors: Vec<&str> = found
            .iter()
            .filter(|diagnostic| diagnostic.severity() == Severity::Error)
            .map(|diagnostic| diagnostic.code.as_str())
            .collect();
        let warning = Code::ALL
            .iter()
            .any(|code| code.as_str() == rule.code && code.severity() == Severity::Warning);
        let expected = if warning {
            let of_code = found
                .iter()
                .filter(|diagnostic| diagnostic.code.as_str() == rule.code)
                .count();
            errors.is_empty() && of_code == 1
        } else {
            errors == [rule.code]
        };
        if !expected {
            failures.push(format!(
                "§{} {:?}: {:?} gives {:?}",
                rule.section,
                rule.code,
                rule.program,
                found
                    .iter()
                    .map(|diagnostic| diagnostic.code.as_str())
                    .collect::<Vec<_>>()
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
