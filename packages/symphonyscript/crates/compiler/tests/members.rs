mod support;

use symphonyscript::{Code, Compiler};
use symphonyscript_core::{
    Bounded, Count, NativeArg, NativeOut, NativeSignature, NativeType, Ty, Value, Vocabulary,
};
use symphonyscript_fixtures::{Rows, covered};

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

#[derive(Clone, Copy, PartialEq)]
enum Takes {
    Array,
    Scalar,
    Reference,
    Wire,
}

const RECEIVERS: [(&str, Takes); 7] = [
    ("[5, 6]", Takes::Array),
    ("xs", Takes::Array),
    ("q", Takes::Scalar),
    ("b.gain", Takes::Scalar),
    ("b.seq[..2]", Takes::Array),
    ("b.hook", Takes::Reference),
    ("c.back", Takes::Wire),
];

type Function = (
    &'static str,
    &'static str,
    &'static str,
    Option<Takes>,
    &'static str,
    bool,
);

const FUNCTIONS: [Function; 8] = [
    ("at", "xs", "k", Some(Takes::Array), "1", true),
    ("pair_sum", "", "", Some(Takes::Array), "", true),
    ("double", "", "", Some(Takes::Scalar), "", true),
    ("shift", "x", "by", Some(Takes::Scalar), "1", true),
    ("max", "", "", Some(Takes::Scalar), "3", false),
    ("clamp", "", "", Some(Takes::Scalar), "0, 9", false),
    ("pick", "w", "k", None, "1", true),
    ("transpose", "w", "k", None, "1", true),
];

const RESTS: [&str; 3] = ["none", "positional", "named"];

const DECLARATIONS: &str = "state xs([5, 6])\nstate q(2)\nfn at(xs, k = 0): xs[k]\nfn shift(x, by = 1): x + by\nfn pick(w, k = 0): k\nfn transpose(w, k = 0): k\n";

fn program(call: &str) -> String {
    format!(
        "{DECLARATIONS}Clip a:\n    Note/(i32({call}))\nClip b:\n    Note as hook\nClip c:\n    Plays as back: b\n"
    )
}

fn double(args: &[NativeArg<'_>], out: &mut NativeOut<'_>) {
    let value = match args.first() {
        Some(NativeArg::Value(Value::I32(value))) => *value,
        Some(NativeArg::Value(_) | NativeArg::Elements(_) | NativeArg::Text(_)) | None => 0,
    };
    out.set(Value::I32(value.saturating_mul(2)));
}

fn pair_sum(args: &[NativeArg<'_>], out: &mut NativeOut<'_>) {
    let mut sum = 0i32;
    if let Some(NativeArg::Elements(values)) = args.first() {
        for value in *values {
            if let Value::I32(value) = value {
                sum = sum.saturating_add(*value);
            }
        }
    }
    out.set(Value::I32(sum));
}

fn dot_vocabulary() -> Vocabulary {
    let mut v = Vocabulary::builder("dots");
    let clip = v.node("Clip");
    let note = v.node("Note");
    let plays = v.wire("Plays");
    v.kind(clip)
        .top_level(Count::Any)
        .children(note, Count::Any)
        .attr::<f32>("gain")
        .default(1.0)
        .attr::<Bounded<i32, 4>>("seq")
        .default(Bounded::from_items([7, 8]).unwrap_or_default());
    v.kind(note).attr::<i32>("pitch").default(60).default_attr();
    v.wire_kind(plays)
        .connects(clip, clip)
        .attr::<i32>("transpose")
        .default(3);
    v.native(
        "pair_sum",
        NativeSignature::new(
            [NativeType::Value(Ty::I32.array(2))],
            NativeType::Value(Ty::I32),
        ),
        pair_sum,
    );
    v.native(
        "double",
        NativeSignature::new([NativeType::Value(Ty::I32)], NativeType::Value(Ty::I32)),
        double,
    );
    v.build().expect("the dot vocabulary builds")
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
    let compiler = Compiler::new(&dot_vocabulary()).expect("the dot vocabulary compiles");
    let codes = |call: &str| -> Vec<Code> {
        support::error_texts(&compiler, &program(call))
            .into_iter()
            .map(|(code, _)| code)
            .collect()
    };
    let mut failures = Vec::new();
    for (receiver_at, (receiver, receives)) in RECEIVERS.iter().enumerate() {
        for (function_at, (function, first, name, takes, listed, bare)) in
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
                    _ => (
                        format!("{receiver}.{function}({name}=1)"),
                        format!("{function}({first}={receiver}, {name}=1)"),
                    ),
                };
                let found = codes(&dot);
                if *function == "transpose" && *receives == Takes::Wire {
                    if found != vec![Code::NotCallable] {
                        failures.push(format!("{dot:?} draws {found:?}, expected [NotCallable]"));
                    }
                    continue;
                }
                let twin = codes(&direct);
                if found != twin {
                    failures.push(format!(
                        "{dot:?} draws {found:?}, and {direct:?} draws {twin:?}"
                    ));
                }
                let fits = takes.is_none_or(|takes| takes == *receives)
                    && !(*function == "pair_sum" && *receiver == "b.seq[..2]")
                    && !(*function == "double" && *receiver == "b.gain")
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

#[test]
fn a_dot_call_on_a_reference_whose_kind_has_no_such_attribute_passes_the_reference_first() {
    let compiler =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    let index_one = "the index 1 is outside the array of length 1".to_string();
    for (text, expected) in [
        (
            "Osc as o\nfn first(w, k): k\nClip c:\n    Note/(o.first(60))\n",
            vec![],
        ),
        (
            "Osc as o\nfn first(w, k): k\nClip c:\n    Note/([0][o.first(1)])\n",
            vec![(Code::IndexOutOfRange, index_one.clone())],
        ),
        (
            "Osc as o\nfn first(w, k): k\nClip c:\n    Note/([0][o.first(k=1)])\n",
            vec![(Code::IndexOutOfRange, index_one.clone())],
        ),
        (
            "Osc as o /330\nfn freq_of(w): w.freq\nClip c:\n    Note/([0][i32(o.freq_of()) - 329])\n",
            vec![(Code::IndexOutOfRange, index_one.clone())],
        ),
        (
            "Clip verse:\n    Note as hook /65\nfn pitch_of(x): x.pitch\nClip c:\n    Note/([0][verse.hook.pitch_of() - 64])\n",
            vec![(Code::IndexOutOfRange, index_one.clone())],
        ),
        (
            "Clip verse:\n    Note\nClip chorus:\n    Plays as back | transpose=3: verse\nfn transpose_of(x): x.transpose\nClip c:\n    Note/([0][chorus.back.transpose_of() - 2])\n",
            vec![(Code::IndexOutOfRange, index_one.clone())],
        ),
        (
            "Osc as o\nClip c:\n    Note/([0][i32(o.abs())])\n",
            vec![(Code::OperandType, "`abs` takes numbers".to_string())],
        ),
        (
            "Osc as o\nfn freq(w, k): k\nClip c:\n    Note/(o.freq(1))\n",
            vec![(
                Code::NotCallable,
                "only a function or a built-in is called".to_string(),
            )],
        ),
        (
            "Clip verse:\n    Note\nClip chorus:\n    Plays as back: verse\nfn transpose(w, k): k\nClip c:\n    Note/(chorus.back.transpose(1))\n",
            vec![(
                Code::NotCallable,
                "only a function or a built-in is called".to_string(),
            )],
        ),
    ] {
        let found: Vec<(Code, String)> = support::errors(compiler.check(&[support::source(text)]))
            .into_iter()
            .map(|diagnostic| (diagnostic.code, diagnostic.summary))
            .collect();
        assert_eq!(found, expected, "{text}");
    }
}

struct EmptyReach {
    items: &'static [&'static str],
    value: &'static str,
    indexed: &'static str,
}

const EMPTY_REACHES: [EmptyReach; 8] = [
    EmptyReach {
        items: &[],
        value: "[verse][{lo}..]{i}.gain",
        indexed: "[verse][{lo}..]{i}",
    },
    EmptyReach {
        items: &["let s = [verse][{lo}..]\n"],
        value: "s{i}.gain",
        indexed: "s{i}",
    },
    EmptyReach {
        items: &["let t = [verse][{lo}..]\n", "let s = t\n"],
        value: "s{i}.gain",
        indexed: "s{i}",
    },
    EmptyReach {
        items: &["let t = [verse]\n", "let s = t[{lo}..]\n"],
        value: "s{i}.gain",
        indexed: "s{i}",
    },
    EmptyReach {
        items: &[],
        value: "[verse][{lo}..][0..]{i}.gain",
        indexed: "[verse][{lo}..][0..]{i}",
    },
    EmptyReach {
        items: &["fn f(c):\n    let s = [c][{lo}..]\n    s{i}.gain\n"],
        value: "f(verse)",
        indexed: "s{i}",
    },
    EmptyReach {
        items: &["fn g(s): s{i}.gain\n"],
        value: "g([verse][{lo}..])",
        indexed: "s{i}",
    },
    EmptyReach {
        items: &[
            "Clip other | gain=3:\n    Note\n",
            "fn h(xs, i, n):\n    let s = xs[i..]\n    if n < 1 then s{i}.gain else h(xs, i, n - 1)\n",
        ],
        value: "h([verse, other], {lo}, 1)",
        indexed: "s{i}",
    },
];

const EMPTY_INDEXES: [(&str, bool); 2] = [("[0]", false), ("[^1]", true)];

const EMPTY_STANDS: [&str; 2] = [
    "Clip k | gain={v}:\n    Note\n",
    "Clip k:\n    Note/(i32({v}))\n",
];

fn empty_element_rows(rows: &Rows) -> Vec<String> {
    let compiler = Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("vocabulary");
    let mut failures = Vec::new();
    for (reach, form) in EMPTY_REACHES.iter().enumerate() {
        for (index, (read, whole)) in EMPTY_INDEXES.iter().enumerate() {
            for (stand, place) in EMPTY_STANDS.iter().enumerate() {
                for (start, empty) in [("3", true), ("0", false)] {
                    let fill = |text: &str| text.replace("{lo}", start).replace("{i}", read);
                    let mut items = vec!["Clip verse | gain=2:\n    Note\n".to_string()];
                    items.extend(form.items.iter().map(|item| fill(item)));
                    items.push(place.replace("{v}", &fill(form.value)));
                    let expected = if empty {
                        let spanned = if *whole {
                            fill(form.indexed)
                        } else {
                            "0".to_string()
                        };
                        vec![(Code::IndexOutOfRange, spanned)]
                    } else {
                        Vec::new()
                    };
                    for (order, reversed) in [false, true].into_iter().enumerate() {
                        if !rows.take(&[reach, index, stand, usize::from(empty), order]) {
                            continue;
                        }
                        let mut ordered = items.clone();
                        if reversed {
                            ordered.reverse();
                        }
                        let text = ordered.concat();
                        let found = support::error_texts(&compiler, &text);
                        if found != expected {
                            failures.push(format!(
                                "338: {text}\n  draws {found:?}\n  expected {expected:?}"
                            ));
                        }
                    }
                }
            }
        }
    }
    failures
}

#[test]
fn an_element_read_of_an_empty_compile_time_array_has_its_element_type_however_the_array_reaches_it()
 {
    let failures = covered(empty_element_rows);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_an_element_read_of_an_empty_compile_time_array_has_its_element_type_however_the_array_reaches_it()
 {
    let failures = empty_element_rows(&Rows::Every);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn a_dot_call_whose_receiver_reads_the_cell_being_declared_resolves_its_member_first_379() {
    let note = "state note(60)\n";
    let read = "Clip c:\n    Note/(recent[0])\n";
    let rows: &[support::Row] = &[
        (
            &[
                note,
                "state recent = (recent.xpast(1, []) + [note])[^8..]\n",
                read,
            ],
            &[
                (Code::DeclarationCycle, "recent"),
                (Code::NotCallable, "recent.xpast"),
            ],
        ),
        (
            &[
                note,
                "state recent([]) = (recent.xpast(1, []) + [note])[^8..]\n",
                read,
            ],
            &[
                (Code::DeclarationCycle, "recent"),
                (Code::NotCallable, "recent.xpast"),
            ],
        ),
        (
            &[
                note,
                "state recent = (recent.past(1, []) + [note])[^8..]\n",
                read,
            ],
            &[],
        ),
        (
            &["let a = a.xpast(1)\n", "Clip c:\n    Note/(a)\n"],
            &[
                (Code::DeclarationCycle, "a"),
                (Code::NotCallable, "a.xpast"),
            ],
        ),
        (
            &[
                "fn twice(x): x * 2\n",
                "let a = a.twice()\n",
                "Clip c:\n    Note/(a)\n",
            ],
            &[(Code::DeclarationCycle, "a")],
        ),
        (
            &[
                note,
                "state recent([]) = (note.xpast(1, []) + [note])[^8..]\n",
                read,
            ],
            &[(Code::NotCallable, "note.xpast")],
        ),
    ];
    support::assert_rows_in_orders(
        support::Orders::ForwardAndReversed,
        &symphonyscript_fixtures::vocabulary(),
        rows,
    );
}
