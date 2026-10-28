use std::collections::BTreeSet;

use symphonyscript::{Code, Compiler, Diagnostic, Severity, Source};
use symphonyscript_core::Limits;
use symphonyscript_fixtures::{
    SPEC, bounded_vocabulary, native_vocabulary, prelude_vocabulary, vocabulary,
};

#[derive(Clone, Copy)]
enum Setup {
    Example,
    Prelude,
    Bounded,
    Natives,
    AtPreludePath,
    LowLimits,
}

struct Rule {
    section: &'static str,
    code: &'static str,
    setup: Setup,
    program: &'static str,
}

const fn rule(section: &'static str, code: &'static str, program: &'static str) -> Rule {
    Rule {
        section,
        code,
        setup: Setup::Example,
        program,
    }
}

const fn bounded(section: &'static str, code: &'static str, program: &'static str) -> Rule {
    Rule {
        section,
        code,
        setup: Setup::Bounded,
        program,
    }
}

const fn natives(section: &'static str, code: &'static str, program: &'static str) -> Rule {
    Rule {
        section,
        code,
        setup: Setup::Natives,
        program,
    }
}

const fn at_prelude_path(section: &'static str, code: &'static str, program: &'static str) -> Rule {
    Rule {
        section,
        code,
        setup: Setup::AtPreludePath,
        program,
    }
}

const fn low_limits(section: &'static str, code: &'static str, program: &'static str) -> Rule {
    Rule {
        section,
        code,
        setup: Setup::LowLimits,
        program,
    }
}

const fn prelude(section: &'static str, code: &'static str, program: &'static str) -> Rule {
    Rule {
        section,
        code,
        setup: Setup::Prelude,
        program,
    }
}

const RULES: &[Rule] = &[
    rule("2.1", "unexpected-character", "let a = 1 & 2\n"),
    rule("2.1", "top-level-syntax", "5\n"),
    at_prelude_path("2.1", "prelude-path", "Clip c:\n    Note/60\n"),
    rule("2.2", "tab-indentation", "Clip c:\n\tNote\n"),
    rule("2.2", "dedent-mismatch", "Clip c:\n    Note\n  Note\n"),
    rule(
        "2.2",
        "clause-column",
        "Clip 音楽 | gain=1\n            level=2\n",
    ),
    rule("2.2", "semicolon", "let a = 1;\n"),
    rule("2.2", "unexpected-indent", "input a = 1\n    input b = 2\n"),
    rule("2.4", "invalid-identifier", "let _ = 1\n"),
    rule("2.5", "reserved-word", "let from = 1\n"),
    rule("2.5", "reserved-token", "let a = 1 == 2\n"),
    rule("2.5", "misplaced-caret", "let a = 2 ^ 3\n"),
    rule("2.6", "misplaced-underscore", "let a = 10_\n"),
    rule("2.6", "float-digits", "let a = .5\n"),
    rule("2.6", "float-type-suffix", "let a = 0.5i32\n"),
    rule("2.6", "unknown-suffix", "let a = 2zz\n"),
    rule("2.6", "prefixed-suffix", "let a = 0b12\n"),
    rule(
        "2.6",
        "integer-too-large",
        "let a = 170141183460469231731687303715884105728\n",
    ),
    rule("2.6", "unterminated-string", "let s = \"abc\n"),
    rule("2.6", "invalid-escape", "let s = \"a\\q\"\n"),
    rule("3.2", "type-syntax", "input a: i32[x] = [1]\n"),
    rule("3.2", "unknown-type", "input a: Shape = 1\n"),
    rule("3.2", "zero-length", "input a: i32[0] = [1]\n"),
    rule(
        "3.2",
        "ragged-array",
        "let a = [1, [2]]\nClip c:\n    Note/a\n",
    ),
    rule("3.2", "length-limit", "input a: i32[..2147483648] = []\n"),
    rule("3.2", "length-limit", "input a: i32[2147483648] = 0\n"),
    rule("3.2", "zero-length", "input a: i32[..0] = []\n"),
    rule("3.2", "type-syntax", "input a: i32[..2][3] = []\n"),
    rule("3.3", "input-value", "input s = \"text\"\n"),
    rule(
        "3.3",
        "reactive-string",
        "input x = 0.5\nKit k:\n    Sampler | file=\"{x}\"\n",
    ),
    rule("3.5", "input-value", "input n = c\nClip c:\n    Note\n"),
    rule("3.5", "type-mismatch", "Clip c:\n    Note | velocity=c\n"),
    rule("3.6", "unit-mismatch", "Clip c:\n    Note | length=2s\n"),
    rule(
        "3.6",
        "unit-mismatch",
        "Clip c:\n    Note | velocity=2bars\n",
    ),
    rule(
        "3.6",
        "mixed-families",
        "Synth s:\n    Osc | freq=2s + 1bars\n",
    ),
    rule(
        "3.6",
        "plain-in-family",
        "input d = 5ms\nClip c:\n    Note when d > 1\n",
    ),
    rule(
        "3.6",
        "plain-in-family",
        "Clip c:\n    Note | length=((2bars + 1) div 1bars)\n",
    ),
    bounded(
        "3.6",
        "conflicting-units",
        "let d = 2bars + 1\nA one | long=d\nB two | short=d\nRoot r:\n    Item\n    Link: r\n",
    ),
    rule(
        "3.6",
        "unit-operation",
        "Clip c:\n    Note | length=2bars * 2bars\n",
    ),
    rule(
        "3.6",
        "unit-operation",
        "Clip c:\n    Note | length=(1 / 1bars)\n",
    ),
    rule(
        "3.6",
        "unit-operation",
        "Clip c:\n    Note | length=(2bars ** 2)\n",
    ),
    rule(
        "3.6",
        "unit-operation",
        "Clip c:\n    for i in 0..2bars:\n        Note\n",
    ),
    bounded(
        "3.6",
        "conflicting-units",
        "input p = 1.0\nA one | long=p\nB two | short=p\nRoot r:\n    Item\n    Link: r\n",
    ),
    rule(
        "4.1",
        "does-not-fit",
        "Clip melody:\n    Note/300000000000\n",
    ),
    rule("4.1", "does-not-fit", "Clip c | gain=(16777217 / 2)\n"),
    rule(
        "4.1",
        "does-not-fit",
        "Clip c:\n    Note/(5i32 + 3000000000)\n",
    ),
    rule(
        "4.1",
        "inconsistent-uses",
        "let q = 2 ** -1\nSynth s:\n    Osc/q\nClip c:\n    Note/q\n",
    ),
    rule("4.2", "type-mismatch", "Clip c:\n    Note | pitch=1i64\n"),
    rule(
        "4.3",
        "type-mismatch",
        "input b = true\nlet x = if b then 1 else false\n",
    ),
    rule(
        "4.4",
        "float-to-integer",
        "input cutoff = 0.5\nClip c:\n    Note/cutoff\n",
    ),
    rule(
        "4.4",
        "does-not-fit",
        "input big = 3000000000\nClip c:\n    Note/big\n",
    ),
    rule(
        "4.4",
        "integer-input",
        "input n = 4\nClip c:\n    Note | velocity=n\n",
    ),
    rule(
        "4.4",
        "float-to-integer",
        "input f = 0.5\nlet g = f * 2\nClip c:\n    Note/g\n",
    ),
    rule(
        "4.6",
        "constant-overflow",
        "let x = 2147483647i32 + 1\nClip c:\n    Note/x\n",
    ),
    rule("4.6", "integer-overflow", "Clip c:\n    Note/(2 ** 200)\n"),
    rule(
        "4.6",
        "integer-overflow",
        "Clip c:\n    Note/(-(2 ** 126) * 2 div 2 ** 120)\n",
    ),
    rule(
        "4.6",
        "inconsistent-shapes",
        "fn r(x, n): if n < 1 then (if x + 0.2 is 0.3 then [x] else [x, x]) else r(x, n - 1)\nlet v = r(0.1, 3)\nClip c | gain=(v[0])\nOsc o | freq=(v[0])\n",
    ),
    rule(
        "4.6",
        "inconsistent-shapes",
        "fn r(x, n): if n < 1 then (if x + 0.2 is 0.3 then [x] else [x, x]) else r(x, n - 1)\ninput f = true\nlet w = if f then r(0.1, 3) else r(0.1, 3)\nOsc o | freq=(w[0])\n",
    ),
    rule(
        "4.6",
        "range-bound",
        "Clip c:\n    for i in 0..0.5:\n        Note\n",
    ),
    rule(
        "4.6",
        "index-out-of-range",
        "Clip c:\n    Note/([1, 2][2 ** 70])\n",
    ),
    rule(
        "4.6",
        "operand-type",
        "fn f(x): if x < 1 then x div 2 else f(x - 1)\nClip c:\n    Note/(f(0.5))\n",
    ),
    rule(
        "4.6",
        "expansion-limit",
        "Clip c:\n    for i in 0..100001:\n        Note Note Note Note Note Note Note Note Note Note\n",
    ),
    rule(
        "4.6",
        "expansion-limit",
        "Clip c:\n    for i in 0..1000000000000:\n        for j in 0..0:\n            Note\n",
    ),
    rule(
        "4.6",
        "expansion-limit",
        "pattern P():\n    for j in 0..0:\n        Note\nClip c:\n    for i in 0..500001:\n        P\n",
    ),
    rule(
        "4.6",
        "inline-limit",
        "fn h0(x): x\nfn h1(x): h0(x) + h0(x) + h0(x) + h0(x) + h0(x) + h0(x) + h0(x) + h0(x) + h0(x) + h0(x)\nfn h2(x): h1(x) + h1(x) + h1(x) + h1(x) + h1(x) + h1(x) + h1(x) + h1(x) + h1(x) + h1(x)\nfn h3(x): h2(x) + h2(x) + h2(x) + h2(x) + h2(x) + h2(x) + h2(x) + h2(x) + h2(x) + h2(x)\nfn h4(x): h3(x) + h3(x) + h3(x) + h3(x) + h3(x) + h3(x) + h3(x) + h3(x) + h3(x) + h3(x)\nfn h5(x): h4(x) + h4(x) + h4(x) + h4(x) + h4(x) + h4(x) + h4(x) + h4(x) + h4(x) + h4(x)\nfn h6(x): h5(x) + h5(x) + h5(x) + h5(x) + h5(x) + h5(x) + h5(x) + h5(x) + h5(x) + h5(x)\nClip k:\n    Note/(h6(1))\n",
    ),
    low_limits(
        "4.6",
        "recursion-limit",
        "fn f(a, b, c): if a < 1 then 0 else f(a - 1, b + 1, c) + f(a - 1, b, c + 1) + f(a - 1, b, c)\nClip k:\n    Note/(f(200, 0, 0))\n",
    ),
    rule("5.1", "expression-syntax", "let a = (1 +)\n"),
    rule("5.1", "chained-comparison", "let a = 1 < 2 < 3\n"),
    rule(
        "5.1",
        "unparenthesized-if",
        "let a = 1 + if true then 1 else 2\n",
    ),
    rule(
        "5.2",
        "operand-type",
        "let a = 1 div 0.5\nClip c:\n    Note/a\n",
    ),
    rule(
        "5.3",
        "operand-type",
        "let a = 1 and 2\nClip c:\n    Note when a\n",
    ),
    rule(
        "5.4",
        "condition-type",
        "let a = if 1 then 2 else 3\nClip c:\n    Note/a\n",
    ),
    rule("5.4", "match-syntax", "input b = 1\nlet a = match b\n"),
    rule(
        "5.4",
        "match-scrutinee",
        "let l = match 1.5:\n    1: 2\n    else: 1\nClip c:\n    Note/l\n",
    ),
    rule(
        "5.4",
        "match-label",
        "input w = Wave::Saw\nlet l = match w:\n    Purple: 1\n    else: 2\n",
    ),
    rule(
        "5.4",
        "match-label",
        "input n = 1\nlet l = match n:\n    1.5: 1\n    else: 2\n",
    ),
    rule(
        "5.4",
        "match-label",
        "input n = 1\nlet l = match n:\n    2bars: 1\n    else: 2\n",
    ),
    rule(
        "5.4",
        "duplicate-arm",
        "input w = Wave::Saw\nlet l = match w:\n    Sine: 1\n    Sine: 2\n    else: 3\n",
    ),
    rule(
        "5.4",
        "match-incomplete",
        "input w = Wave::Saw\nlet l = match w:\n    Sine: 1\n",
    ),
    rule(
        "5.4",
        "redundant-else",
        "input b = true\nlet l = match b:\n    true: 1\n    false: 2\n    else: 3\n",
    ),
    rule(
        "5.4",
        "match-incomplete",
        "input n = 1\nlet l = match n:\n    1: 2\n",
    ),
    rule(
        "5.4",
        "match-position",
        "let a = 1 + match 1:\n    else: 2\n",
    ),
    rule(
        "5.5",
        "index-type",
        "let a = [1, 2][true]\nClip c:\n    Note/a\n",
    ),
    rule("5.5", "not-an-array", "let a = 1[0]\nClip c:\n    Note/a\n"),
    rule(
        "5.5",
        "index-out-of-range",
        "Clip c:\n    Note/([1, 2, 3][5])\n",
    ),
    rule(
        "5.5",
        "array-operand",
        "let a = [1, 2] + 3\nClip c:\n    Note/a\n",
    ),
    rule(
        "5.6",
        "runtime-path",
        "input i = 0\nlet s = [verse, chorus]\nClip verse | gain=s[i].gain:\n    Note\nClip chorus:\n    Note\n",
    ),
    rule(
        "5.6",
        "unknown-attribute",
        "Clip c | gain=c.nothing:\n    Note\n",
    ),
    rule(
        "5.6",
        "declaration-cycle",
        "Clip a | gain=b.gain:\n    Note\nClip b | gain=a.gain:\n    Note\n",
    ),
    rule("5.7", "not-callable", "let a = 1(2)\n"),
    rule(
        "5.7",
        "mixed-arguments",
        "fn f(a, b): a\nlet a = f(1, b=2)\n",
    ),
    rule(
        "5.7",
        "named-builtin-arguments",
        "let a = sin(x=1)\nClip c:\n    Note/a\n",
    ),
    rule(
        "5.7",
        "argument-count",
        "let a = lerp(1, 2)\nClip c:\n    Note/a\n",
    ),
    rule("5.8", "conversion-syntax", "let a = i32 1\n"),
    rule(
        "5.8",
        "conversion-type",
        "let a = i32(true)\nClip c:\n    Note/a\n",
    ),
    rule(
        "5.8",
        "conversion-type",
        "let a = i32(Wave::Saw)\nClip c:\n    Note/a\n",
    ),
    rule(
        "5.9",
        "interpolation-type",
        "Kit k:\n    Sampler | file=\"{k}\"\n",
    ),
    rule("6.1", "input-syntax", "input = 1\n"),
    rule("6.1", "reactive-initializer", "input a = 1\ninput b = a\n"),
    rule("6.2", "let-syntax", "let = 1\n"),
    rule("6.2", "declaration-cycle", "let a = b + 1\nlet b = a + 1\n"),
    rule("6.2", "declaration-cycle", "fn f(): a\nlet a = f()\n"),
    rule("6.2", "declaration-cycle", "input a = b\nlet b = a + 1\n"),
    rule(
        "6.2",
        "declaration-cycle",
        "fn g(x): x\nfn f():\n    let a = g(a)\n    a\nClip c:\n    Note/(f())\n",
    ),
    rule(
        "6.2",
        "reactive-node-reference",
        "input i = 0\nlet t = [verse, chorus][i]\nClip verse:\n    Note\nClip chorus:\n    Note\n",
    ),
    rule("6.3", "fn-syntax", "fn (x): x\n"),
    rule("6.3", "duplicate-local", "fn f(a, a): a\n"),
    rule(
        "6.3",
        "duplicate-local",
        "fn f(x):\n    let a = 1\n    let a = 2\n    a\n",
    ),
    rule("6.3", "duplicate-local", "fn f(x):\n    let x = 1\n    x\n"),
    rule("6.3", "parameter-order", "fn f(a = 1, b): a + b\n"),
    rule(
        "6.3",
        "missing-argument",
        "fn f(a, b): a\nlet x = f(1)\nClip c:\n    Note/x\n",
    ),
    rule(
        "6.3",
        "missing-argument",
        "fn f(a, b): a\nlet x = f(a=1)\nClip c:\n    Note/x\n",
    ),
    rule(
        "6.3",
        "argument-count",
        "fn f(x): x\nlet a = f(1, 2)\nClip c:\n    Note/a\n",
    ),
    rule("6.3", "unknown-parameter", "fn f(x): x\nlet a = f(y=1)\n"),
    rule(
        "6.3",
        "duplicate-argument",
        "fn f(x): x\nlet a = f(x=1, x=2)\nClip c:\n    Note/a\n",
    ),
    rule(
        "6.3",
        "reactive-recursion",
        "input i = 1\nfn f(n): if n < 1 then 0 else f(n - 1)\nClip c:\n    Note/(f(i))\n",
    ),
    rule(
        "6.3",
        "reactive-recursion",
        "input n = 1\nfn a(x) bounded 3 else 0: b(x)\nfn b(x): if x > 0 then a(x - 1) else 0\nClip c:\n    Note/(a(n))\n",
    ),
    rule(
        "6.3",
        "reactive-recursion",
        "input i = 1\nfn f(n): if n < 1 then i else f(n - 1)\nClip c:\n    Note/(f(3))\n",
    ),
    rule(
        "6.3",
        "recursion-depth",
        "fn f(n): if n < 1 then 0 else f(n - 1) + 1\nClip c:\n    Note/(f(2000))\n",
    ),
    rule("6.4", "pattern-syntax", "pattern (x):\n    Note\n"),
    rule("6.4", "duplicate-local", "pattern P(a, a):\n    Note/a\n"),
    rule(
        "6.4",
        "duplicate-local",
        "pattern P():\n    let a = 1\n    let a = 2\n    Note/a\n",
    ),
    rule(
        "6.4",
        "duplicate-local",
        "pattern P(root):\n    let root = 1\n    Note/root\n",
    ),
    rule(
        "6.4",
        "duplicate-default-parameter",
        "pattern P(default a, default b):\n    Note/a\n",
    ),
    rule(
        "6.4",
        "no-default-parameter",
        "pattern P(root):\n    Note/root\nClip c:\n    P/1 | root=2\n",
    ),
    rule(
        "6.4",
        "duplicate-argument",
        "pattern P(default root):\n    Note/root\nClip c:\n    P/1 | root=2\n",
    ),
    rule(
        "6.4",
        "missing-argument",
        "pattern P(root):\n    Note/root\nClip c:\n    P\n",
    ),
    rule(
        "6.4",
        "unknown-parameter",
        "pattern P(root = 1):\n    Note/root\nClip c:\n    P | octave=1\n",
    ),
    rule(
        "6.4",
        "use-form",
        "pattern P():\n    Note\nClip c:\n    P'\n",
    ),
    rule(
        "6.4",
        "use-form",
        "pattern P():\n    Note\nClip c:\n    P:\n        Note\n",
    ),
    rule(
        "6.4",
        "recursive-pattern",
        "pattern A():\n    Clip:\n        A\n",
    ),
    rule(
        "6.4",
        "top-level-pattern",
        "pattern P():\n    Note\n    Note\nP top\n",
    ),
    rule("6.5", "duplicate-name", "input a = 1\nlet a = 2\n"),
    rule("6.5", "variant-name", "let Saw = 1\n"),
    rule(
        "6.5",
        "variant-name",
        "fn f(x):\n    let Saw = x\n    Saw\n",
    ),
    rule(
        "6.5",
        "child-name-clash",
        "Clip c:\n    Note as dup\n    Note as dup\n",
    ),
    rule(
        "6.5",
        "child-name-clash",
        "Track t:\n    Synth as dup:\n        Osc as dup\n",
    ),
    rule("6.5", "child-name-clash", "Clip c:\n    Note as gain\n"),
    rule(
        "6.5",
        "child-name-clash",
        "input b = true\nClip c:\n    when b:\n        Note as dup\n    else:\n        Note as dup\n",
    ),
    rule(
        "6.5",
        "for-variable-clash",
        "Clip c:\n    for sin in 0..2:\n        Note\n",
    ),
    rule(
        "6.5",
        "declaration-cycle",
        "fn f():\n    let a = b\n    let b = a\n    a\n",
    ),
    rule("6.5", "unknown-name", "let a = b\n"),
    rule("3.4", "unknown-name", "Clip c | gain=Saw(1)\n"),
    rule(
        "3.7",
        "variable-array",
        "input a: i32[..4] = []\nlet b: i32[2] = a\n",
    ),
    natives(
        "3.7",
        "capacity-exceeded",
        "input held: i32[..10] = []\nClip c | sequence=held:\n    Note\n",
    ),
    natives(
        "3.7",
        "capacity-exceeded",
        "Clip c | sequence=[1, 2, 3, 4, 5, 6, 7, 8, 9]:\n    Note\n",
    ),
    rule(
        "3.7",
        "capacity-exceeded",
        "input a: i32[..4] = []\nlet b: i32[..2] = a\n",
    ),
    rule(
        "3.7",
        "capacity-exceeded",
        "input a: i32[..2] = [1, 2, 3]\n",
    ),
    rule(
        "4.6",
        "index-out-of-range",
        "Clip c:\n    Note/([1, 2][2 ** 70..][0])\n",
    ),
    natives(
        "5.7",
        "named-builtin-arguments",
        "let a = checksum(text=\"x\")\nClip c:\n    Note/a\n",
    ),
    rule("6.3", "needless-bound", "fn f(x) bounded 3 else 0: x\n"),
    rule("6.3", "fn-syntax", "fn f(x) bounded else 0: f(x)\n"),
    rule("6.3", "fn-syntax", "fn f(x) bounded 0 else 0: f(x)\n"),
    rule(
        "6.5",
        "child-name-clash",
        "pattern P(default n):\n    Note as n\n    Note\nClip c:\n    P/1\n",
    ),
    rule(
        "6.5",
        "child-name-clash",
        "Track t:\n    for i in 0..2:\n        Synth:\n            Osc as i\n",
    ),
    rule(
        "6.5",
        "child-name-clash",
        "pattern P():\n    let k = 1\n    Note as k\n    Note\nClip c:\n    P\n",
    ),
    rule("6.6", "state-syntax", "state = 1\n"),
    rule("6.6", "expression-syntax", "input a = 1\nstate s = a{}\n"),
    rule("6.6", "declaration-cycle", "state s = s + 1\n"),
    rule(
        "6.6",
        "state-syntax",
        "Clip c:\n    state x = 1\n    Note\n",
    ),
    rule("6.6", "misplaced-history", "input a = 1\nlet b = a{1}\n"),
    rule(
        "6.6",
        "misplaced-history",
        "input a = 1\nfn f(x): a{1} + x\nlet b = f(1)\n",
    ),
    rule("6.6", "history-offset", "input a = 1\nstate s = a{0}\n"),
    rule("6.6", "history-offset", "input a = 1\nstate s = a{a}\n"),
    rule("6.6", "history-target", "let a = 1\nstate s = a{1, 0}\n"),
    rule("6.6", "history-fallback", "state s = s{1} + 1\n"),
    rule(
        "6.6",
        "type-mismatch",
        "input a = 1\nstate s = a{1, true}\n",
    ),
    rule("6.6", "history-depth", "input a = 1\nstate[1] s = a{2}\n"),
    rule("6.6", "state-value", "state s = \"x\"\n"),
    rule("6.6", "state-value", "Clip c:\n    Note\nstate s = c\n"),
    rule(
        "6.6",
        "unbounded-state",
        "input note = 60\nstate recent = recent{1, []} + [note]\n",
    ),
    rule("7.1", "placement-syntax", "Clip\n"),
    rule("7.1", "top-level-wire", "Plays p: c\n"),
    rule("7.4", "flag-syntax", "Clip c:\n    Note'x\n"),
    rule("7.4", "unknown-flag", "Clip c:\n    Note!\n"),
    rule("7.5", "default-arg-syntax", "Clip c:\n    Note/a[0]\n"),
    rule("7.5", "no-default-attribute", "Synth s:\n    Mixer/1\n"),
    rule("7.6", "attribute-syntax", "Clip c:\n    Note | =1\n"),
    rule(
        "7.6",
        "duplicate-attribute",
        "Clip c:\n    Note/60 | pitch=61\n",
    ),
    rule("7.6", "unknown-attribute", "Clip c:\n    Note | loud=1\n"),
    rule("7.6", "missing-attribute", "Kit k:\n    Sampler\n"),
    rule(
        "7.7",
        "inline-placement",
        "Clip c:\n    Note/60 Note | velocity=1\n",
    ),
    rule("7.8", "bundle-syntax", "Clip c:\n    [Note\n"),
    rule(
        "7.8",
        "empty-bundle",
        "Clip c:\n    [for i in 0..0: Note]\n",
    ),
    rule(
        "7.8",
        "bundle-member",
        "pattern P():\n    Plays: c\nClip c:\n    [P]\n",
    ),
    rule(
        "7.8",
        "bundle-member",
        "pattern P():\n    [Note/62 Note/64]\nClip c:\n    [P]\n",
    ),
    rule(
        "7.8",
        "bundle-member",
        "pattern Inner():\n    Synth:\n        Osc\nTrack t:\n    [Inner]\nSession s:\n    Includes: t\n",
    ),
    rule(
        "7.8",
        "bundle-member",
        "pattern Inner():\n    Synth:\n        Osc\npattern Outer():\n    Inner\nTrack t:\n    [Outer]\nSession s:\n    Includes: t\n",
    ),
    rule("7.8", "misplaced-wire", "Clip c:\n    [Plays: c]\n"),
    rule(
        "7.9",
        "for-syntax",
        "Clip c:\n    for in 0..2:\n        Note\n",
    ),
    rule(
        "7.9",
        "iterable-type",
        "Clip c:\n    for i in 5:\n        Note\n",
    ),
    rule(
        "7.9",
        "reactive-iterable",
        "input x = 1\nClip c:\n    for i in [x, x]:\n        Note\n",
    ),
    rule(
        "7.9",
        "named-in-for",
        "Clip c:\n    for i in 0..2:\n        Note as n\n",
    ),
    rule("8.1", "wire-syntax", "Clip c:\n    Plays\n"),
    rule("8.2", "destination-type", "Clip c:\n    Plays: 1\n"),
    rule(
        "8.2",
        "misplaced-to",
        "Clip a:\n    Note\nClip c:\n    Plays: to.x\n",
    ),
    rule(
        "8.2",
        "not-a-value",
        "pattern V():\n    Note\nClip c:\n    Plays: V\n",
    ),
    bounded(
        "8.3",
        "to-candidates",
        "input i = 0\nA one\nB two\nRoot r:\n    Item\n    Link: r\n    Pick | level=to.x: [one, two][i]\n",
    ),
    rule("8.3", "misplaced-to", "let a = to.gain\n"),
    rule(
        "8.3",
        "misplaced-to",
        "Clip a:\n    Note as n\nClip c:\n    Plays | gain=to.n: a\n",
    ),
    rule("8.3", "misplaced-to", "Clip c:\n    Plays | gain=to: c\n"),
    rule(
        "8.3",
        "misplaced-to",
        "Clip a:\n    Note\n    Plays as w: a\nClip c:\n    Plays | gain=to.w: a\n",
    ),
    rule(
        "8.4",
        "wire-cycle",
        "Synth s:\n    Filter as b:\n        Feeds: c\n    Filter as c:\n        Feeds: b\n",
    ),
    rule("9.1", "condition-type", "Clip c:\n    Note when 1\n"),
    rule("9.2", "when-syntax", "Clip c:\n    when x\n        Note\n"),
    prelude("10.1", "prelude-item", "input a = 1\n"),
    prelude("10.1", "unknown-name", "let a = b\n"),
    natives(
        "10.1",
        "compile-time-native",
        "input n = 0\nClip c:\n    Note/(tag(\"abc\", n))\n",
    ),
    rule("10.2", "unknown-kind", "Clip c:\n    Foo\n"),
    rule("10.2", "unknown-variant", "let l = Wave::Purple\n"),
    rule("10.2", "not-top-level", "Note n\n"),
    rule("10.2", "count-exceeded", "Session a\nSession b\n"),
    bounded("10.2", "count-below-minimum", ""),
    rule("10.2", "child-not-accepted", "Clip c:\n    Osc\n"),
    bounded(
        "10.2",
        "count-exceeded",
        "Root r:\n    Item\n    Item\n    Item\n    Link: r\n",
    ),
    bounded("10.2", "count-below-minimum", "Root r:\n    Link: r\n"),
    rule("10.2", "wire-endpoint", "Track t:\n    Starts: t\n"),
    bounded(
        "10.2",
        "count-exceeded",
        "Root r:\n    Item\n    Link: r\n    Link: r\n",
    ),
    bounded("10.2", "count-below-minimum", "Root r:\n    Item\n"),
    rule(
        "10.2",
        "unreachable-instance",
        "Session s:\n    Track\nClip lonely:\n    Note\n",
    ),
    rule("10.2", "no-entry-instance", "Clip lonely:\n    Note\n"),
    rule("14", "not-a-value", "let a = sin\n"),
];

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
        Setup::Natives => Compiler::new(&native_vocabulary())
            .expect("the native vocabulary compiles")
            .check(&[source]),
        Setup::LowLimits => Compiler::with_limits(
            &vocabulary(),
            Limits {
                recursive_calls: 1_000,
                ..Limits::default()
            },
        )
        .expect("the example vocabulary compiles")
        .check(&[source]),
        Setup::AtPreludePath => Compiler::new(&prelude_vocabulary(""))
            .expect("the prelude vocabulary compiles")
            .check(&[Source {
                path: "prelude.sym".into(),
                text: rule.program.into(),
            }]),
        Setup::Prelude => match Compiler::new(&prelude_vocabulary(rule.program)) {
            Ok(_) => Vec::new(),
            Err(failed) => failed.diagnostics,
        },
    }
}

fn spec_has_heading(section: &str) -> bool {
    let subsection = format!("### {section} ");
    let top = format!("## {section}.");
    SPEC.lines()
        .any(|line| line.starts_with(&subsection) || line.starts_with(&top))
}

#[test]
fn a_count_an_unknown_placement_or_destination_could_change_draws_no_diagnostic() {
    let rows = [
        bounded("10.2", "unknown-kind", "Root r:\n    Itme\n    Link: r\n"),
        bounded(
            "10.2",
            "unknown-name",
            "Root r:\n    Item\n    Link: nosuch\n",
        ),
        bounded("10.2", "unknown-kind", "Rot r:\n    Item\n    Link: r\n"),
        bounded(
            "10.2",
            "unknown-name",
            "Root r:\n    for i in 0..nothing:\n        Item\n    Link: r\n",
        ),
        rule("10.2", "unknown-kind", "Clip c:\n    [Nte/60]\n"),
        rule(
            "10.2",
            "unknown-kind",
            "Session s:\n    Track\nClip lonely:\n    Nte:\n        Starts: lonely\n",
        ),
        bounded(
            "10.2",
            "unknown-kind",
            "pattern Back():\n    Link: r\nRoot r:\n    Item\n    Bak\n",
        ),
        rule(
            "10.2",
            "unknown-kind",
            "pattern Feed():\n    Includes: t\nSession s:\n    Fed\nTrack t:\n    Piano\n",
        ),
        bounded("10.2", "unknown-kind", "Root r:\n    Item\n    Lnk: r\n"),
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
fn every_rule_section_has_a_heading_in_the_spec() {
    for rule in RULES {
        assert!(
            spec_has_heading(rule.section),
            "{}: the SPEC has no heading for §{}",
            rule.code,
            rule.section
        );
    }
}

#[test]
fn every_code_appears_in_the_table_and_every_table_code_exists() {
    let names: BTreeSet<&str> = Code::ALL
        .iter()
        .filter(|code| **code != Code::InternalError)
        .map(|code| code.as_str())
        .collect();
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
        let Some(section) = code.section() else {
            continue;
        };
        assert!(
            RULES
                .iter()
                .any(|rule| rule.code == code.as_str() && rule.section == section),
            "{code}: no rule of §{section} maps to it",
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

#[test]
fn every_diagnostic_of_a_rule_program_has_a_summary_of_one_to_three_lines() {
    let mut failures = Vec::new();
    for rule in RULES {
        for diagnostic in diagnostics(rule) {
            let lines = diagnostic.summary.lines().count();
            let blank = diagnostic
                .summary
                .lines()
                .any(|line| line.trim().is_empty());
            let help_lines = diagnostic
                .help
                .as_deref()
                .map_or(1, |help| help.lines().count());
            let labelled = diagnostic
                .related
                .iter()
                .all(|related| !related.label.trim().is_empty());
            if !(1..=3).contains(&lines) || blank || help_lines != 1 || !labelled {
                failures.push(format!("{:?}: {diagnostic:?}", rule.program));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
