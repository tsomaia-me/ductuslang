mod support;

use symphonyscript::{Code, Compiler};
use symphonyscript_fixtures::{Rows, covered, native_vocabulary};

const ARRAYS: [(&str, &str, Option<u32>, u32); 6] = [
    ("", "[1, 2, 3]", Some(3), 3),
    ("let a = [4, 5, 6, 7]\n", "a", Some(4), 4),
    ("state a([1, 2])\n", "a", Some(2), 2),
    ("state a([]): i32[..8]\n", "a", None, 8),
    ("let a = [[1, 2], [3, 4], [5, 6]]\n", "a", Some(3), 3),
    ("let a = [1][1..]\n", "a", Some(0), 0),
];

const MEMBERS: [&str; 2] = ["size", "capacity"];

#[derive(Clone, Copy, PartialEq)]
enum Needs {
    Anything,
    CompileTime,
    Flexible,
}

const POSITIONS: [(&str, Needs); 8] = [
    (
        "Clip c:\n    Note/([60, 61, 62, 63, 64, 65, 66, 67, 68][{}])\n",
        Needs::Anything,
    ),
    (
        "let s = [60, 61, 62, 63, 64, 65, 66, 67, 68][..{}]\nClip c:\n    Note/(s.size * 0)\n",
        Needs::Anything,
    ),
    (
        "Clip c:\n    for i in 0..{}:\n        Note/i\n",
        Needs::CompileTime,
    ),
    ("Clip c:\n    Note/({})\n", Needs::Anything),
    ("Clip c:\n    Note when {} > 1\n", Needs::Anything),
    ("let v = {}\nClip c:\n    Note/v\n", Needs::Anything),
    ("let v: u8 = {}\nClip c:\n    Note/v\n", Needs::Flexible),
    (
        "let v: f32 = {}\nClip c:\n    Note | velocity=v\n",
        Needs::Anything,
    ),
];

fn member_rows(rows: &Rows) -> Vec<String> {
    let mut failures = Vec::new();
    for (array_at, (decl, array, size, capacity)) in ARRAYS.iter().enumerate() {
        for (member_at, member) in MEMBERS.iter().enumerate() {
            for (position_at, (position, needs)) in POSITIONS.iter().enumerate() {
                if !rows.take(&[array_at, member_at, position_at]) {
                    continue;
                }
                let value = format!("{array}.{member}");
                let text = format!("{decl}{}", position.replacen("{}", &value, 1));
                let known = if *member == "size" {
                    *size
                } else {
                    Some(*capacity)
                };
                let flexible = known.is_some();
                let expected = match needs {
                    Needs::CompileTime if known.is_none() => vec![Code::ReactiveIterable],
                    Needs::Flexible if !flexible => vec![Code::TypeMismatch],
                    Needs::Anything | Needs::CompileTime | Needs::Flexible => Vec::new(),
                };
                let found: Vec<Code> = support::check_codes(&text)
                    .iter()
                    .map(|(code, ..)| *code)
                    .collect();
                if found != expected {
                    failures.push(format!("{text:?} draws {found:?}, expected {expected:?}"));
                }
                let Some(known) = known.filter(|known| *known > 0) else {
                    continue;
                };
                let probe = format!(
                    "{decl}Clip c:\n    Note/([{}][{value}])\n",
                    vec!["60"; known as usize].join(", ")
                );
                let summary = format!("the index {known} is outside the array of length {known}");
                let drawn: Vec<(Code, String)> = support::checked(&probe)
                    .into_iter()
                    .map(|diagnostic| (diagnostic.code, diagnostic.summary))
                    .collect();
                if drawn != vec![(Code::IndexOutOfRange, summary.clone())] {
                    failures.push(format!("{probe:?} draws {drawn:?}, expected {summary:?}"));
                }
            }
        }
    }
    failures
}

#[test]
fn an_arrays_size_and_capacity_are_known_at_compile_time_where_the_spec_says_and_typed_as_it_says_in_every_position()
 {
    let failures = covered(member_rows);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_an_arrays_size_and_capacity_are_known_at_compile_time_where_the_spec_says_and_typed_as_it_says_in_every_position()
 {
    let failures = member_rows(&Rows::Every);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

const RECEIVERS: [(&str, bool); 5] = [
    ("[5, 6]", true),
    ("xs", true),
    ("q", false),
    ("b.gain", false),
    ("b.seq[..2]", true),
];

const FUNCTIONS: [(&str, &str, bool, &str, bool); 5] = [
    ("at", "k", true, "1", true),
    ("pair_sum", "", true, "", true),
    ("shift", "by", false, "1", true),
    ("max", "", false, "3", false),
    ("clamp", "", false, "0, 9", false),
];

const RESTS: [&str; 3] = ["none", "positional", "named"];

const DECLARATIONS: &str =
    "state xs([5, 6])\nstate q(2)\nfn at(xs, k = 0): xs[k]\nfn shift(x, by = 1): x + by\n";

fn program(call: &str) -> String {
    format!("{DECLARATIONS}Clip a:\n    Note/(i32({call}))\nClip b:\n    Note\n")
}

fn call_pair(receiver: &str, function: &str, listed: &str) -> (String, String) {
    if listed.is_empty() {
        (
            format!("{receiver}.{function}()"),
            format!("{function}({receiver})"),
        )
    } else {
        (
            format!("{receiver}.{function}({listed})"),
            format!("{function}({receiver}, {listed})"),
        )
    }
}

fn dot_rows(rows: &Rows) -> Vec<String> {
    let compiler = Compiler::new(&native_vocabulary()).expect("the native vocabulary compiles");
    let codes = |call: &str| -> Vec<Code> {
        support::error_texts(&compiler, &program(call))
            .into_iter()
            .map(|(code, _)| code)
            .collect()
    };
    let mut failures = Vec::new();
    for (receiver_at, (receiver, array)) in RECEIVERS.iter().enumerate() {
        for (function_at, (function, name, takes_array, listed, bare)) in
            FUNCTIONS.iter().enumerate()
        {
            for (rest_at, rest) in RESTS.iter().enumerate() {
                if !rows.take(&[receiver_at, function_at, rest_at]) {
                    continue;
                }
                let (dot, direct) = match *rest {
                    "none" => call_pair(receiver, function, ""),
                    "positional" => call_pair(receiver, function, listed),
                    _ if name.is_empty() => (
                        format!("{receiver}.{function}(a=1)"),
                        format!("{function}(x={receiver}, a=1)"),
                    ),
                    _ => {
                        let first = if *takes_array { "xs" } else { "x" };
                        (
                            format!("{receiver}.{function}({name}=1)"),
                            format!("{function}({first}={receiver}, {name}=1)"),
                        )
                    }
                };
                let found = codes(&dot);
                let twin = codes(&direct);
                if found != twin {
                    failures.push(format!(
                        "{dot:?} draws {found:?}, and {direct:?} draws {twin:?}"
                    ));
                }
                let fits = array == takes_array
                    && !(*function == "pair_sum" && *receiver == "b.seq[..2]")
                    && match *rest {
                        "none" => *bare,
                        "positional" => !listed.is_empty(),
                        _ => !name.is_empty(),
                    };
                if fits && !found.is_empty() {
                    failures.push(format!("{dot:?} draws {found:?}"));
                }
            }
        }
    }
    failures
}

#[test]
fn a_dot_call_is_the_call_with_the_receiver_first_for_a_source_a_native_and_a_math_function() {
    let failures = covered(dot_rows);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_dot_call_is_the_call_with_the_receiver_first_for_a_source_a_native_and_a_math_function()
 {
    let failures = dot_rows(&Rows::Every);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn the_rows_of_the_brief_hold() {
    for (text, expected) in [
        (
            "Clip c:\n    Note/([1, 2, 3][[1, 2, 3].size])\n",
            vec![(Code::IndexOutOfRange, "[1, 2, 3].size")],
        ),
        (
            "state xs([]): i32[..8]\nlet b: u8 = xs.size\nClip c:\n    Note/b\n",
            vec![(Code::TypeMismatch, "xs.size")],
        ),
        (
            "state xs([]): i32[..8]\nlet b: i32 = xs.size\nClip c:\n    Note/b\n",
            vec![],
        ),
        (
            "state xs([]): i32[..8]\nlet b: u8 = xs.capacity\nlet d: f32 = xs.capacity\nClip c:\n    Note/b | velocity=d\n",
            vec![],
        ),
        (
            "state xs([]): i32[..8]\nClip c:\n    Note/([1, 2, 3, 4, 5, 6, 7, 8, 9][xs.capacity])\n    Note/([1, 2, 3, 4, 5, 6, 7, 8][xs.capacity])\n",
            vec![(Code::IndexOutOfRange, "xs.capacity")],
        ),
        (
            "state notes([60, 64])\nfn f(x): x + 1\nClip c:\n    Note/(notes.map(f)[0])\n",
            vec![(Code::UnbuiltConstruct, "notes.map(f)")],
        ),
        (
            "state q(1)\nClip c:\n    Note/(q.dropped)\n",
            vec![(Code::UnbuiltConstruct, "q.dropped")],
        ),
        (
            "Clip c:\n    Note/(i32(math::sin(0.5) + sin(0.5) + math::pi))\n",
            vec![],
        ),
        (
            "Clip c:\n    Note/(lib::x + lib::f(1))\n",
            vec![
                (Code::UnbuiltConstruct, "lib::x"),
                (Code::UnbuiltConstruct, "lib::f"),
            ],
        ),
        (
            "Clip c:\n    Note/(i32(math::sin))\n",
            vec![(Code::NotAValue, "math::sin")],
        ),
        (
            "state q(1)\nClip c:\n    Note/(q.nothing(1))\n",
            vec![(Code::NotCallable, "q.nothing")],
        ),
        (
            "state xs([1, 2])\nClip c:\n    Note/(xs.size(1))\n",
            vec![(Code::NotCallable, "xs.size")],
        ),
        (
            "Clip c:\n    Note/(\"abc\".size)\n",
            vec![(Code::UnbuiltConstruct, "\"abc\".size")],
        ),
    ] {
        let mut expected: Vec<(Code, String)> = expected
            .into_iter()
            .map(|(code, spanned)| (code, spanned.to_string()))
            .collect();
        expected.sort();
        let compiler =
            Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
        assert_eq!(support::error_texts(&compiler, text), expected, "{text}");
    }
}

#[test]
fn the_old_calls_name_the_members_that_replace_them() {
    for (text, call, help) in [
        (
            "state xs([1, 2])\nClip c:\n    Note/(len(xs))\n",
            "len",
            "write `x.size`",
        ),
        (
            "state xs([1, 2])\nClip c:\n    Note/(bytes(xs)[0])\n",
            "bytes",
            "write `x.bytes`",
        ),
    ] {
        let (start, end) = support::at(text, call, 0);
        let drawn: Vec<_> = support::checked(text)
            .into_iter()
            .map(|diagnostic| {
                (
                    diagnostic.code,
                    diagnostic.span.range.start,
                    diagnostic.span.range.end,
                    diagnostic.help,
                )
            })
            .collect();
        assert_eq!(
            drawn,
            vec![(Code::UnknownName, start, end, Some(help.to_string()))],
            "{text}"
        );
    }
}

#[test]
fn the_examples_the_spec_writes_with_size_pass_check() {
    for text in [
        "state held([]): i32[..10]\nstate rises = for i in 1..held.size into n from 0: if held[i] > held[i - 1] then n + 1 else n\nClip c:\n    Note/rises\n",
        "state held([]): i32[..10]\nstate high = for x in held into kept from []:\n    if x > 60 then kept + [x] else kept\nClip c:\n    Note/(high.size)\n",
        "state steps([true, false, false, true, false, false, true, false])\nstate pitches([36, 36, 43, 36, 41, 36, 43, 48])\nClip seq:\n    for i in 0..8:\n        Note/(pitches[i]) when steps[i]\n",
    ] {
        assert_eq!(support::check_codes(text), vec![], "{text}");
    }
}
