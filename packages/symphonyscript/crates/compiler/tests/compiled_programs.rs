mod support;

use support::{errors, source};
use symphonyscript::{Code, Compiler};
use symphonyscript_core::LimitOverrides;
use symphonyscript_fixtures::vocabulary;

const HELPERS: [&str; 4] = [
    "fn r(m): if m > 0 then r(m - 1) else [1]\n",
    "fn s(m): if m > 0 then s(m - 1) else 1\n",
    "fn d(m): if m < 1 then 0 else d(m - 1) + 1\n",
    "Session main\n",
];

type Drawn = &'static [(Code, &'static str)];

const KINDS: [(&str, &str, Drawn, Drawn); 16] = [
    (
        "an exact power past 2^127, twice",
        "2 ** 200 - 2 ** 200",
        &[
            (Code::IntegerOverflow, "2 ** 200"),
            (Code::IntegerOverflow, "2 ** 200"),
        ],
        &[
            (Code::IntegerOverflow, "2 ** 200"),
            (Code::IntegerOverflow, "2 ** 200"),
        ],
    ),
    (
        "a shape need refusing an array operand",
        "-[1, 2]",
        &[(Code::ArrayOperand, "-[1, 2]")],
        &[(Code::ArrayOperand, "-[1, 2]")],
    ),
    (
        "a pick past its items",
        "[10, 20, 30][n + 5]",
        &[(Code::IndexOutOfRange, "n + 5")],
        &[(Code::IndexOutOfRange, "0 + 5")],
    ),
    (
        "an element past its array",
        "[[1, 2], [3, 4]][n][7]",
        &[(Code::IndexOutOfRange, "7")],
        &[(Code::IndexOutOfRange, "7")],
    ),
    (
        "a fixed sum that saturates",
        "i32(n) + 2147483647 + 1",
        &[(Code::ConstantOverflow, "i32(n) + 2147483647 + 1")],
        &[(Code::ConstantOverflow, "i32(0) + 2147483647 + 1")],
    ),
    (
        "a negation that saturates",
        "-(i32(n) - 2147483647 - 1)",
        &[(Code::ConstantOverflow, "-(i32(n) - 2147483647 - 1)")],
        &[(Code::ConstantOverflow, "-(i32(0) - 2147483647 - 1)")],
    ),
    (
        "a comparison's operand past i64",
        "(if n < 30000000000000000000 then 60 else 61)",
        &[(Code::DoesNotFit, "30000000000000000000")],
        &[(Code::DoesNotFit, "30000000000000000000")],
    ),
    (
        "a from-end offset past i64",
        "[1, 2, 3][^(n + 30000000000000000000)]",
        &[(Code::IndexOutOfRange, "n + 30000000000000000000")],
        &[(Code::IndexOutOfRange, "0 + 30000000000000000000")],
    ),
    (
        "a deferred product of an array",
        "r(n) * 2",
        &[(Code::ArrayOperand, "r(n) * 2")],
        &[(Code::ArrayOperand, "r(0) * 2")],
    ),
    (
        "a deferred sum of an array and a single value",
        "r(n) + 1",
        &[(Code::ArrayOperand, "r(n) + 1")],
        &[(Code::ArrayOperand, "r(0) + 1")],
    ),
    (
        "a join of a single value",
        "([1] + s(n))[0]",
        &[(Code::ArrayOperand, "[1] + s(n)")],
        &[(Code::ArrayOperand, "[1] + s(0)")],
    ),
    (
        "a slice of a single value",
        "s(n)[0..1][0]",
        &[(Code::NotAnArray, "s(n)")],
        &[(Code::NotAnArray, "s(0)")],
    ),
    (
        "a slice bound past i64",
        "[1, 2, 3][(n + 30000000000000000000)..][0]",
        &[(Code::IndexOutOfRange, "n + 30000000000000000000")],
        &[(Code::IndexOutOfRange, "0 + 30000000000000000000")],
    ),
    (
        "the size of a single value",
        "s(n).size",
        &[(Code::NotAnArray, "s(n)")],
        &[(Code::NotAnArray, "s(0).size")],
    ),
    (
        "the bytes of a single value",
        "s(n).bytes.size",
        &[(Code::OperandType, "s(n)")],
        &[(Code::OperandType, "s(0)")],
    ),
    (
        "a built-in that saturates",
        "abs(i32(n) - 2147483647 - 1)",
        &[(Code::ConstantOverflow, "abs(i32(n) - 2147483647 - 1)")],
        &[(Code::ConstantOverflow, "abs(i32(0) - 2147483647 - 1)")],
    ),
];

fn drawn(compiler: &Compiler, text: &str) -> Vec<(Code, String)> {
    errors(compiler.check(&[source(text)]))
        .iter()
        .map(|diagnostic| {
            let range = &diagnostic.span.range;
            let spanned = text
                .get(range.start as usize..range.end as usize)
                .unwrap_or_default();
            (diagnostic.code, spanned.to_string())
        })
        .collect()
}

fn in_both_orders(items: &[String]) -> [String; 2] {
    let reversed: Vec<String> = items.iter().rev().cloned().collect();
    [items.concat(), reversed.concat()]
}

fn expected(rows: Drawn) -> Vec<(Code, String)> {
    rows.iter()
        .map(|(code, text)| (*code, (*text).to_string()))
        .collect()
}

#[test]
fn every_instruction_that_draws_a_diagnostic_draws_it_at_the_span_and_in_the_order_the_evaluator_drew_in_either_order_300()
 {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    for (name, expression, in_frame, at_top) in KINDS {
        let body = format!("fn g(n): if n > 0 then g(n - 1) else {expression}\n");
        let top = at_zero(expression);
        for (items, rows) in [
            (
                vec![body, "Clip c:\n    Note/(g(2))\n".to_string()],
                in_frame,
            ),
            (vec![format!("Clip c:\n    Note/({top})\n")], at_top),
        ] {
            let mut items = items;
            items.extend(HELPERS.iter().map(|helper| (*helper).to_string()));
            for text in in_both_orders(&items) {
                let found = drawn(&compiler, &text);
                if found != expected(rows) {
                    failures.push(format!("{name}:\n{text}  found {found:?}"));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

const SUMS: [&str; 2] = [
    "fn f(a, b, c): if a < 1 then 0 else f(a - 1, b + 1, c) + f(a - 1, b, c + 1) + f(a - 1, b, c)\n",
    "fn f(a, b, c): if a < 1 then 0 else f(a - 1, b, c) + f(a - 1, b, c + 1) + f(a - 1, b + 1, c)\n",
];

#[test]
fn the_call_that_passes_the_recursive_call_limit_is_the_call_the_evaluator_reached_in_either_order_of_the_operands_and_the_items_300()
 {
    let mut failures = Vec::new();
    for (limit, sites) in [
        (10, ["f(a - 1, b, c)", "f(a - 1, b + 1, c)"]),
        (50, ["f(a - 1, b + 1, c)", "f(a - 1, b, c)"]),
        (1_000, ["f(a - 1, b + 1, c)", "f(a - 1, b, c)"]),
    ] {
        let limits = LimitOverrides {
            recursive_calls: Some(limit),
            recursive_calls_warning: Some(limit),
            ..LimitOverrides::default()
        };
        let compiler =
            Compiler::with_limits(&vocabulary(), limits).expect("the vocabulary compiles");
        for (sum, site) in SUMS.iter().zip(sites) {
            let items = vec![
                (*sum).to_string(),
                "Clip k:\n    Note/(f(20, 0, 0))\n".to_string(),
                "Session main\n".to_string(),
            ];
            for text in in_both_orders(&items) {
                let found = drawn(&compiler, &text);
                if found != [(Code::RecursionLimit, site.to_string())] {
                    failures.push(format!("{limit}: {text}  found {found:?}"));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

const CHOSEN: &str = "fn i0(z): [1.5]\nfn r1(n): if n > 0 then r1(n - 1) else [1]\nfn rc(m):\n    let ch = if m is 0 then r1(1) else i0(0)\n";

#[test]
fn an_array_through_a_conversion_is_converted_leaf_by_leaf_and_a_refused_one_draws_at_its_holder_in_either_order_300()
 {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    for (tail, rows) in [
        (
            "[10, 20][i32(ch[0] * 4.0)]",
            &[(Code::IndexOutOfRange, "i32(ch[0] * 4.0)")][..],
        ),
        ("-(ch)", &[(Code::ArrayOperand, "-(ch)")][..]),
        ("abs(ch)", &[(Code::ArrayOperand, "abs(ch)")][..]),
    ] {
        let items = vec![
            format!("{CHOSEN}    if m > 0 then rc(m - 1) else {tail}\n"),
            "Clip c | gain=rc(1):\n    Note\n".to_string(),
            "Session main\n".to_string(),
        ];
        for text in in_both_orders(&items) {
            let found = drawn(&compiler, &text);
            if found != expected(rows) {
                failures.push(format!("{text}  found {found:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn at_zero(expression: &str) -> String {
    let characters: Vec<char> = expression.chars().collect();
    let word = |at: Option<&char>| at.is_some_and(|character| character.is_alphanumeric());
    characters
        .iter()
        .enumerate()
        .map(|(at, character)| {
            let alone = *character == 'n'
                && !word(at.checked_sub(1).and_then(|before| characters.get(before)))
                && !word(characters.get(at + 1));
            if alone { '0' } else { *character }
        })
        .collect()
}
