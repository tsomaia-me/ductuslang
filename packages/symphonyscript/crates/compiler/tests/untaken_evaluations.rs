mod support;

use std::collections::BTreeSet;

use support::error_texts;
use symphonyscript::{Code, Compiler};
use symphonyscript_core::LimitOverrides;
use symphonyscript_fixtures::{Rows, covered, vocabulary};

const INSTANCES: &str = "Clip verse | gain=2:\n    Note as tone\nClip other | gain=3:\n    Note\nClip tail | gain=5:\n    Note\n";

const HELPERS: &str = "fn pk(xs, p, m): if m < 1 then xs[p] else pk(xs, p, m - 1)\nfn dp(x, m): if m < 1 then x else dp(x, m - 1)\nfn rg(xs, p, m): if m < 1 then [xs[..p], xs[..1]][0][0] else rg(xs, p, m - 1)\n";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Site {
    RecursiveBody,
    TopLevel,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Guard {
    False,
    Deep,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Form {
    Direct,
    Helper,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Fault {
    Outside,
    Fit,
    Overflow,
    Depth,
    Ragged,
}

const SITES: [Site; 2] = [Site::RecursiveBody, Site::TopLevel];
const GUARDS: [Guard; 2] = [Guard::False, Guard::Deep];
const FORMS: [Form; 2] = [Form::Direct, Form::Helper];
const FAULTS: [Fault; 5] = [
    Fault::Outside,
    Fault::Fit,
    Fault::Overflow,
    Fault::Depth,
    Fault::Ragged,
];

impl Fault {
    fn code(self) -> Code {
        match self {
            Fault::Outside => Code::IndexOutOfRange,
            Fault::Fit => Code::DoesNotFit,
            Fault::Overflow => Code::IntegerOverflow,
            Fault::Depth => Code::RecursionDepth,
            Fault::Ragged => Code::RaggedArray,
        }
    }

    fn start(self, form: Form, xs: &str, i: &str) -> Option<String> {
        let index = match self {
            Fault::Outside => "5".to_string(),
            Fault::Fit => format!("u8({i} + 300) - 300"),
            Fault::Overflow => format!("{i} + 2 ** 126 * 4"),
            Fault::Depth | Fault::Ragged => format!("{i} % 1"),
        };
        Some(match (self, form) {
            (Fault::Ragged, Form::Direct) => return None,
            (Fault::Ragged, Form::Helper) => format!("rg({xs}, 2, 1)"),
            (Fault::Depth, Form::Direct) => format!("{xs}[dp({index}, 2000)]"),
            (Fault::Depth, Form::Helper) => format!("pk({xs}, {index}, 2000)"),
            (Fault::Outside | Fault::Fit | Fault::Overflow, Form::Direct) => {
                format!("{xs}[{index}]")
            }
            (Fault::Outside | Fault::Fit | Fault::Overflow, Form::Helper) => {
                format!("pk({xs}, {index}, 1)")
            }
        })
    }
}

impl Guard {
    fn text(self, i: &str) -> String {
        match self {
            Guard::False => "false".to_string(),
            Guard::Deep => format!("{i} > 100"),
        }
    }
}

fn ordered(items: Vec<String>, reversed: bool) -> String {
    let mut items = items;
    if reversed {
        items.reverse();
    }
    format!("Session main\n{}", items.concat())
}

fn untaken_program(site: Site, guard: &str, start: &str, reversed: bool) -> String {
    let reader = match site {
        Site::RecursiveBody => vec![
            format!(
                "fn h(xs, i): if i >= xs.size then 0 else (if {guard} then i32(({start}).gain) else 0) + i32(xs[i].gain) + h(xs, i + 1)\n"
            ),
            "Clip k:\n    Note/([60][h([verse, other], 0) - 5])\n".to_string(),
        ],
        Site::TopLevel => vec![format!(
            "Clip k:\n    Note/(if {guard} then i32(({start}).gain) else 60)\n"
        )],
    };
    let mut items = vec![INSTANCES.to_string(), HELPERS.to_string()];
    items.extend(reader);
    ordered(items, reversed)
}

fn codes(compiler: &Compiler, text: &str) -> BTreeSet<Code> {
    error_texts(compiler, text)
        .into_iter()
        .map(|(code, _)| code)
        .collect()
}

fn untaken_start_rows(rows: &Rows) -> (Vec<String>, usize) {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    let mut programs = 0;
    for (s, site) in SITES.iter().enumerate() {
        let (xs, i) = match site {
            Site::RecursiveBody => ("xs", "i"),
            Site::TopLevel => ("[verse, other]", "0"),
        };
        for (g, guard) in GUARDS.iter().enumerate() {
            for (f, form) in FORMS.iter().enumerate() {
                if *site == Site::TopLevel && *form == Form::Direct {
                    continue;
                }
                for (k, fault) in FAULTS.iter().enumerate() {
                    let Some(start) = fault.start(*form, xs, i) else {
                        continue;
                    };
                    for (o, reversed) in [false, true].into_iter().enumerate() {
                        if !rows.take(&[s, g, f, k, o]) {
                            continue;
                        }
                        programs += 1;
                        let text = untaken_program(*site, &guard.text(i), &start, reversed);
                        let twin = untaken_program(*site, "true", &start, reversed);
                        let found = codes(&compiler, &text);
                        let twin_found = codes(&compiler, &twin);
                        let limited: BTreeSet<Code> = match fault {
                            Fault::Depth => BTreeSet::from([Code::RecursionDepth]),
                            Fault::Outside | Fault::Fit | Fault::Overflow | Fault::Ragged => {
                                BTreeSet::new()
                            }
                        };
                        if found != limited || !twin_found.contains(&fault.code()) {
                            failures.push(format!(
                                "{text}\n  draws {found:?}, its taken twin {twin_found:?}\n"
                            ));
                        }
                    }
                }
            }
        }
    }
    (failures, programs)
}

fn assert_none_fail((failures, programs): (Vec<String>, usize), expected: usize) {
    assert!(
        failures.is_empty(),
        "{} of {programs} programs fail:\n{}",
        failures.len(),
        failures.iter().take(6).cloned().collect::<String>()
    );
    assert_eq!(programs, expected);
}

#[test]
fn an_untaken_starts_evaluation_draws_no_value_fault_in_any_frame_it_opens_and_its_taken_twin_draws_it()
 {
    assert_none_fail(covered(untaken_start_rows), 12);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_an_untaken_starts_evaluation_draws_no_value_fault_in_any_frame_it_opens_and_its_taken_twin_draws_it()
 {
    assert_none_fail(untaken_start_rows(&Rows::Every), 56);
}

fn limited(calls: u64) -> Compiler {
    Compiler::with_limits(
        &vocabulary(),
        LimitOverrides {
            recursive_calls: Some(calls),
            recursive_calls_warning: Some(calls),
            ..LimitOverrides::default()
        },
    )
    .expect("the vocabulary compiles")
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Counted {
    SharedKeys,
    QuietTree,
}

fn counted_program(counted: Counted, guard: &str, reversed: bool) -> String {
    let items = match counted {
        Counted::SharedKeys => vec![
            INSTANCES.to_string(),
            "fn pk(xs, p, m): if m < 1 then xs[p % 2] else pk(xs, p, m - 1)\n".to_string(),
            format!(
                "fn h(xs, i, o): if i < 1 then 0 else (if {guard} then i32(pk(xs, i + o, 20).gain) else 0) + h(xs, i - 1, o)\n"
            ),
            "fn t(xs, n, o): if n < 1 then 0 else i32(pk(xs, n + o, 20).gain) + t(xs, n - 1, o)\n"
                .to_string(),
            "Clip a | gain=h([verse, other], 20, 0):\n    Note\n".to_string(),
            "Clip a2 | gain=h([verse, other], 20, 1000):\n    Note\n".to_string(),
            "Clip b | gain=t([verse, other], 20, 0):\n    Note\n".to_string(),
            "Clip b2 | gain=t([verse, other], 20, 1000):\n    Note\n".to_string(),
        ],
        Counted::QuietTree => vec![
            INSTANCES.to_string(),
            "fn t(a, b): if b < 1 then 0 else t(a * 2, b - 1) + t(a * 2 + 1, b - 1)\n".to_string(),
            "fn f(n): if n < 1 then 0 else f(n - 1) + 1\n".to_string(),
            format!(
                "fn h(ys, i): if i < 1 then 0 else (if {guard} then i32((ys[t(i + 1, 10) % 2 + 1]).gain) else 0) + h(ys, i - 1)\n"
            ),
            "Clip k:\n    Note/([60][h([verse, other, tail], 2)])\n".to_string(),
            "Clip z:\n    Note/([60][f(3) - 3])\n".to_string(),
        ],
    };
    ordered(items, reversed)
}

fn counted_rows(rows: &Rows) -> (Vec<String>, usize) {
    let mut failures = Vec::new();
    let mut programs = 0;
    let cases = [
        (Counted::SharedKeys, 1_000, false),
        (Counted::SharedKeys, 900, true),
        (Counted::QuietTree, 8_000, false),
        (Counted::QuietTree, 5_000, true),
    ];
    for (c, (counted, calls, draws)) in cases.into_iter().enumerate() {
        let compiler = limited(calls);
        for (g, guard) in ["false", "i > 100"].into_iter().enumerate() {
            for (o, reversed) in [false, true].into_iter().enumerate() {
                if !rows.take(&[c, g, o]) {
                    continue;
                }
                programs += 1;
                let text = counted_program(counted, guard, reversed);
                let found = codes(&compiler, &text);
                if found.contains(&Code::RecursionLimit) != draws
                    || found.iter().any(|code| *code != Code::RecursionLimit)
                {
                    failures.push(format!(
                        "{text}\n  with {calls} calls draws {found:?}, recursion-limit expected {draws}\n"
                    ));
                }
            }
        }
    }
    (failures, programs)
}

#[test]
fn each_distinct_call_counts_once_whichever_evaluation_reaches_it_and_the_limit_draws_in_every_order()
 {
    assert_none_fail(covered(counted_rows), 8);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_each_distinct_call_counts_once_whichever_evaluation_reaches_it_and_the_limit_draws_in_every_order()
 {
    assert_none_fail(counted_rows(&Rows::Every), 16);
}

fn shallow() -> Compiler {
    Compiler::with_limits(
        &vocabulary(),
        LimitOverrides {
            nesting_depth: Some(40),
            nesting_depth_warning: Some(40),
            ..LimitOverrides::default()
        },
    )
    .expect("the vocabulary compiles")
}

fn chain_program(levels: usize, chain: usize, guard: &str, reversed: bool) -> String {
    ordered(
        vec![
            INSTANCES.to_string(),
            "fn pk(xs, p, m): if m < 1 then xs[p] else pk(xs, p, m - 1)\n".to_string(),
            format!(
                "fn h(xs, n): if n < 1 then 0 else 1 + (if {guard} then i32(pk(xs, 0, {chain} + n % 1).gain) else 0) + h(xs, n - 1)\n"
            ),
            format!("Clip k:\n    Note/([60][h([verse, other], {levels}) - {levels}])\n"),
        ],
        reversed,
    )
}

fn chain_rows(rows: &Rows) -> (Vec<String>, usize) {
    let compiler = shallow();
    let mut failures = Vec::new();
    let mut programs = 0;
    let cases = [(5, 60, true, true), (38, 3, false, true)];
    for (c, (levels, chain, passes, twin_passes)) in cases.into_iter().enumerate() {
        for (g, guard) in ["false", "n > 100"].into_iter().enumerate() {
            for (o, reversed) in [false, true].into_iter().enumerate() {
                if !rows.take(&[c, g, o]) {
                    continue;
                }
                programs += 1;
                let text = chain_program(levels, chain, guard, reversed);
                let found = codes(&compiler, &text);
                let expected: BTreeSet<Code> = if passes {
                    BTreeSet::from([Code::RecursionDepth])
                } else {
                    BTreeSet::new()
                };
                let twin = chain_program(levels, chain, "true", reversed);
                let twin_found = codes(&compiler, &twin);
                let twin_draws = twin_passes == twin_found.contains(&Code::RecursionDepth);
                if found != expected || !twin_draws {
                    failures.push(format!(
                        "{text}\n  draws {found:?}, its taken twin {twin_found:?}\n"
                    ));
                }
            }
        }
    }
    (failures, programs)
}

#[test]
fn an_untaken_evaluation_counts_its_depth_from_its_own_start_and_draws_recursion_depth_when_its_chain_passes()
 {
    assert_none_fail(covered(chain_rows), 5);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_an_untaken_evaluation_counts_its_depth_from_its_own_start_and_draws_recursion_depth_when_its_chain_passes()
 {
    assert_none_fail(chain_rows(&Rows::Every), 8);
}

fn open_key_program(intermediaries: usize, guard: &str, order: usize) -> String {
    let callee = |argument: &str| match intermediaries {
        0 => format!("b(ys, {argument})"),
        _ => format!("kk(ys, {argument})"),
    };
    let mut functions = vec![
        format!(
            "fn a(ys, n): if n < 1 then 0 else a(ys, n - 1) + 1 + (if {guard} then i32((ys[0 * {}]).gain) else 0)\n",
            callee("n")
        ),
        format!(
            "fn b(ys, n): if n < 1 then 1 else b(ys, n - 1) + 1 + (if {guard} then i32((ys[({} + 1) % 2]).gain) else 0)\n",
            callee("n + 1")
        ),
    ];
    match intermediaries {
        0 => {}
        1 => functions.push("fn kk(ys, m): b(ys, m)\n".to_string()),
        _ => functions.push("fn kk(ys, m): k2(ys, m)\nfn k2(ys, m): b(ys, m)\n".to_string()),
    }
    let ca = "Clip ca:\n    Note/([60][a([verse, verse.tone], 1) - 1])\n".to_string();
    let cb = "Clip cb:\n    Note/([60][b([verse, verse.tone], 1) - 2])\n".to_string();
    let (first, second) = if order.is_multiple_of(2) {
        (ca, cb)
    } else {
        (cb, ca)
    };
    let mut items = vec![INSTANCES.to_string()];
    if order < 2 {
        items.extend(functions);
        items.extend([first, second]);
    } else {
        items.extend([first, second]);
        items.extend(functions);
    }
    ordered(items, false)
}

fn open_key_rows(rows: &Rows) -> (Vec<String>, usize) {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    let mut programs = 0;
    for intermediaries in 0..3 {
        for (g, guard) in ["false", "n > 100"].into_iter().enumerate() {
            for order in 0..4 {
                if !rows.take(&[intermediaries, g, order]) {
                    continue;
                }
                programs += 1;
                let text = open_key_program(intermediaries, guard, order);
                let found = codes(&compiler, &text);
                if !found.is_empty() {
                    failures.push(format!("{text}\n  draws {found:?}\n"));
                }
            }
        }
    }
    (failures, programs)
}

#[test]
fn a_value_computed_through_a_key_open_at_the_frame_is_never_reused_there_in_any_order() {
    assert_none_fail(covered(open_key_rows), 12);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_value_computed_through_a_key_open_at_the_frame_is_never_reused_there_in_any_order()
{
    assert_none_fail(open_key_rows(&Rows::Every), 24);
}

const SITE_PAIRS: [([&str; 2], [&str; 2]); 4] = [
    (["verse", "other"], ["other", "verse"]),
    (["verse", "other"], ["other", "tail"]),
    (["verse", "tail"], ["other", "verse"]),
    (["verse", "other"], ["verse", "other"]),
];

fn gain_of(instance: &str) -> i32 {
    match instance {
        "verse" => 2,
        "other" => 3,
        _ => 5,
    }
}

fn candidate_rows(rows: &Rows) -> (Vec<String>, usize) {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    let mut programs = 0;
    for (p, (first, second)) in SITE_PAIRS.iter().enumerate() {
        for (r, recursive) in [true, false].into_iter().enumerate() {
            for (c, chooser) in ["xs[0]", "xs[m % 2]"].into_iter().enumerate() {
                for (o, reversed) in [false, true].into_iter().enumerate() {
                    if !rows.take(&[p, r, c, o]) {
                        continue;
                    }
                    programs += 1;
                    let g = if recursive {
                        format!("fn g(xs, m): if m < 1 then {chooser} else g(xs, m - 1)\n")
                    } else {
                        format!("fn g(xs, m): {}\n", chooser.replace('m', "0"))
                    };
                    let site = |name: &str, array: &[&str; 2]| {
                        format!(
                            "Clip {name}:\n    Note/([60][h([{}], 3) - {}])\n",
                            array.join(", "),
                            3 * gain_of(array[0]) - 1
                        )
                    };
                    let text = ordered(
                        vec![
                            INSTANCES.to_string(),
                            g,
                            "fn h(xs, n): if n < 1 then 0 else i32(g(xs, n).gain) + h(xs, n - 1)\n"
                                .to_string(),
                            site("k", first),
                            site("k2", second),
                        ],
                        reversed,
                    );
                    let found: Vec<Code> = error_texts(&compiler, &text)
                        .into_iter()
                        .map(|(code, _)| code)
                        .collect();
                    if found != [Code::IndexOutOfRange; 2] {
                        failures.push(format!("{text}\n  draws {found:?}\n"));
                    }
                }
            }
        }
    }
    (failures, programs)
}

#[test]
fn a_read_whose_evaluation_chooses_a_candidate_no_arm_holds_instantiates_its_arm_and_draws_nothing()
{
    assert_none_fail(covered(candidate_rows), 9);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_read_whose_evaluation_chooses_a_candidate_no_arm_holds_instantiates_its_arm_and_draws_nothing()
 {
    assert_none_fail(candidate_rows(&Rows::Every), 32);
}

fn growing_rows(rows: &Rows) -> (Vec<String>, usize) {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    let mut programs = 0;
    let outers: [&[&str]; 2] = [&["verse"], &["verse", "other"]];
    for (t, taken) in [false, true].into_iter().enumerate() {
        for (l, outer) in outers.iter().enumerate() {
            for (o, reversed) in [false, true].into_iter().enumerate() {
                if !rows.take(&[t, l, o]) {
                    continue;
                }
                programs += 1;
                let read = "i32((ys[h(xs + [other], ys, 9) % 2]).gain)";
                let arm = if taken {
                    format!("(if i > 0 then 0 else {read})")
                } else {
                    format!("(if false then {read} else 0)")
                };
                let sum: i32 = outer.iter().map(|name| gain_of(name)).sum();
                let value = if taken { sum + 2 } else { sum };
                let text = ordered(
                    vec![
                        INSTANCES.to_string(),
                        format!(
                            "fn h(xs, ys, i): if i >= xs.size then 0 else i32(xs[i].gain) + {arm} + h(xs, ys, i + 1)\n"
                        ),
                        format!(
                            "Clip k:\n    Note/([60][h([{}], [verse, other], 0) - {value} + 1])\n",
                            outer.join(", ")
                        ),
                    ],
                    reversed,
                );
                let started = std::time::Instant::now();
                let found = codes(&compiler, &text);
                let elapsed = started.elapsed();
                if found != BTreeSet::from([Code::IndexOutOfRange])
                    || elapsed > std::time::Duration::from_secs(5)
                {
                    failures.push(format!("{text}\n  draws {found:?} in {elapsed:?}\n"));
                }
            }
        }
    }
    (failures, programs)
}

#[test]
fn a_self_read_path_start_whose_array_argument_grows_finishes_within_the_limits_at_its_value() {
    assert_none_fail(covered(growing_rows), 5);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_self_read_path_start_whose_array_argument_grows_finishes_within_the_limits_at_its_value()
 {
    assert_none_fail(growing_rows(&Rows::Every), 8);
}

#[test]
fn a_recursive_call_that_reaches_its_own_key_draws_recursion_depth_in_either_order() {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    for function in ["fn f(k): f(k)\n", "fn f(k, j): f(k, j)\n"] {
        let call = if function.contains('j') {
            "f(1, 2)"
        } else {
            "f(1)"
        };
        for reversed in [false, true] {
            let text = ordered(
                vec![
                    function.to_string(),
                    format!("Clip k:\n    Note/({call})\n"),
                ],
                reversed,
            );
            assert_eq!(
                error_texts(&compiler, &text),
                vec![(
                    Code::RecursionDepth,
                    call.replace('1', "k").replace('2', "j")
                )],
                "{text}"
            );
        }
    }
}

struct Ledger {
    entry: u32,
    text: String,
    twin: Option<(String, Code)>,
    drawn: BTreeSet<Code>,
}

fn ledger() -> Vec<Ledger> {
    let body = |guard: &str| {
        format!(
            "fn deep(x, m): if m < 1 then x else deep(x, m - 1)\nfn h(xs, n): if n < 1 then 0 else h(xs, n - 1) + (if {guard} then i32(xs[deep(0, 100000)].gain) else 0)\nClip k:\n    Note/([60][h([verse, other], 3)])\n"
        )
    };
    let guarded = |guard: &str| {
        format!(
            "fn h(xs, n): if n < 1 then 0 else (if xs.size > {guard} then i32(xs[2].gain) else i32(xs[0].gain)) + h(xs, n - 1)\nClip k:\n    Note/([60][h([verse, other], 2) - 4])\n"
        )
    };
    let ragged = |guard: &str| {
        format!(
            "fn pk(xs, p, m): if m < 1 then [xs[..p], xs[..1]][0][0] else pk(xs, p, m - 1)\nfn total(xs, i): if i >= xs.size then 0 else i32(xs[i].gain) + total(xs, i + 1) + (if {guard} then i32(pk(xs, i + 2, 1).gain) else 0)\nClip k:\n    Note/([60][total([verse, other], 0) - 5])\n"
        )
    };
    let top = |guard: &str| {
        format!(
            "fn pk(xs, p, m): if m < 1 then xs[p] else pk(xs, p, m - 1)\nClip k:\n    Note/(if {guard} then i32(pk([verse, other], 5, 1).gain) else 60)\n"
        )
    };
    vec![
        Ledger {
            entry: 357,
            text: body("false"),
            twin: Some((body("true"), Code::RecursionDepth)),
            drawn: BTreeSet::from([Code::RecursionDepth]),
        },
        Ledger {
            entry: 357,
            text: guarded("2"),
            twin: Some((guarded("1"), Code::IndexOutOfRange)),
            drawn: BTreeSet::new(),
        },
        Ledger {
            entry: 361,
            text: ragged("false"),
            twin: Some((ragged("true"), Code::RaggedArray)),
            drawn: BTreeSet::new(),
        },
        Ledger {
            entry: 362,
            text: top("false"),
            twin: Some((top("true"), Code::IndexOutOfRange)),
            drawn: BTreeSet::new(),
        },
        Ledger {
            entry: 364,
            text: "fn g(xs, m): if m < 1 then xs[0] else g(xs, m - 1)\nfn h(xs, n): if n < 1 then 0 else i32(g(xs, n).gain) + h(xs, n - 1)\nClip k | gain=h([verse, other], 3):\n    Note\nClip k2 | gain=h([other, verse], 3):\n    Note\n".to_string(),
            twin: None,
            drawn: BTreeSet::new(),
        },
        Ledger {
            entry: 366,
            text: open_key_program(1, "false", 0).replacen("Session main\n", "", 1).replacen(INSTANCES, "", 1),
            twin: None,
            drawn: BTreeSet::new(),
        },
        Ledger {
            entry: 368,
            text: "fn h(xs, ys, i): if i >= xs.size then 0 else i32(xs[i].gain) + (if false then i32((ys[h(xs + [other], ys, 9) % 2]).gain) else 0) + h(xs, ys, i + 1)\nClip k:\n    Note/([60][h([verse, other], [verse, other], 0) - 5])\n".to_string(),
            twin: None,
            drawn: BTreeSet::new(),
        },
    ]
}

fn items(text: &str) -> Vec<String> {
    let mut items: Vec<String> = Vec::new();
    for line in text.lines() {
        match items.last_mut() {
            Some(last) if line.starts_with(' ') => {
                last.push_str(line);
                last.push('\n');
            }
            Some(_) | None => items.push(format!("{line}\n")),
        }
    }
    items
}

#[test]
fn the_ledger_programs_draw_what_their_rules_give_in_either_order_and_their_taken_twins_draw_their_fault()
 {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    let mut programs = 0;
    for row in ledger() {
        for reversed in [false, true] {
            programs += 1;
            let mut all = items(INSTANCES);
            all.extend(items(&row.text));
            let text = ordered(all, reversed);
            let found = codes(&compiler, &text);
            if found != row.drawn {
                failures.push(format!("{}: {text}\n  draws {found:?}\n", row.entry));
            }
            if let Some((twin, code)) = &row.twin {
                let mut all = items(INSTANCES);
                all.extend(items(twin));
                let twin = ordered(all, reversed);
                let twin_found = codes(&compiler, &twin);
                if !twin_found.contains(code) {
                    failures.push(format!("{}: {twin}\n  draws {twin_found:?}\n", row.entry));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert_eq!(programs, 14);
}

const VALUE_DERIVED: [Code; 7] = [
    Code::IndexOutOfRange,
    Code::DoesNotFit,
    Code::IntegerOverflow,
    Code::ConstantOverflow,
    Code::LengthLimit,
    Code::RaggedArray,
    Code::ReactiveRecursion,
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TopCall {
    LackingAtPitch,
    LackingAtAttribute,
    Cycle,
    LackingBesideValueFault,
    ChosenCell,
}

const TOP_CALLS: [TopCall; 5] = [
    TopCall::LackingAtPitch,
    TopCall::LackingAtAttribute,
    TopCall::Cycle,
    TopCall::LackingBesideValueFault,
    TopCall::ChosenCell,
];

impl TopCall {
    fn items(self, guard: &str) -> Vec<String> {
        let plain = "fn h(xs, i, n): if n < 1 then i32(xs[i].gain) else h(xs, i, n - 1)\n";
        let faulty =
            "fn h(xs, i, n): if n < 1 then i32(xs[i].gain) + [1][i] else h(xs, i, n - 1)\n";
        let (function, reader) = match self {
            TopCall::LackingAtPitch => (
                plain,
                format!(
                    "Clip k:\n    Note/((if {guard} then h([verse, verse.tone], 1, 1) else 60))\n"
                ),
            ),
            TopCall::LackingAtAttribute => (
                plain,
                format!(
                    "Clip k | gain=(if {guard} then h([verse, verse.tone], 1, 2) else 1):\n    Note\n"
                ),
            ),
            TopCall::Cycle => (
                plain,
                format!("Clip k | gain=(if {guard} then h([verse, k], 1, 1) else 1):\n    Note\n"),
            ),
            TopCall::LackingBesideValueFault => (
                faulty,
                format!(
                    "Clip k:\n    Note/((if {guard} then h([verse, verse.tone], 1, 1) else 60))\n"
                ),
            ),
            TopCall::ChosenCell => (
                plain,
                format!("Clip k:\n    Note/((if {guard} then h([verse, o2], 1, 1) else 60))\n"),
            ),
        };
        let mut items = vec![INSTANCES.to_string(), function.to_string(), reader];
        if self == TopCall::ChosenCell {
            items.push("state gv(0.5)\nClip o2 | gain=gv:\n    Note\n".to_string());
        }
        items
    }

    fn drawn(self) -> Code {
        match self {
            TopCall::LackingAtPitch
            | TopCall::LackingAtAttribute
            | TopCall::LackingBesideValueFault => Code::UnknownAttribute,
            TopCall::Cycle => Code::DeclarationCycle,
            TopCall::ChosenCell => Code::ReactiveRecursion,
        }
    }
}

fn top_call_rows(rows: &Rows) -> (Vec<String>, usize) {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    let mut programs = 0;
    for (c, call) in TOP_CALLS.iter().enumerate() {
        for (g, guard) in ["false", "1 > 2"].into_iter().enumerate() {
            for (o, reversed) in [false, true].into_iter().enumerate() {
                if !rows.take(&[c, g, o]) {
                    continue;
                }
                programs += 1;
                let text = ordered(call.items(guard), reversed);
                let twin = ordered(call.items("true"), reversed);
                let found: Vec<(Code, String)> = error_texts(&compiler, &text)
                    .into_iter()
                    .map(|(code, spanned)| (code, spanned.replace(guard, "true")))
                    .collect();
                let twin_found = error_texts(&compiler, &twin);
                let expected: Vec<(Code, String)> = twin_found
                    .iter()
                    .filter(|(code, _)| !VALUE_DERIVED.contains(code))
                    .cloned()
                    .collect();
                let twin_draws = twin_found.iter().any(|(code, _)| *code == call.drawn())
                    && (*call != TopCall::LackingBesideValueFault
                        || twin_found
                            .iter()
                            .any(|(code, _)| *code == Code::IndexOutOfRange));
                if found != expected || !twin_draws {
                    failures.push(format!(
                        "352: {text}\n  draws {found:?}, its taken twin {twin_found:?}\n"
                    ));
                }
            }
        }
    }
    (failures, programs)
}

#[test]
fn a_recursive_call_in_an_untaken_arm_at_top_level_is_checked_against_its_chosen_references_and_draws_no_value_fault()
 {
    assert_none_fail(covered(top_call_rows), 10);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_recursive_call_in_an_untaken_arm_at_top_level_is_checked_against_its_chosen_references_and_draws_no_value_fault()
 {
    assert_none_fail(top_call_rows(&Rows::Every), 20);
}

const ARM_POSITIONS: [(&str, &str, &str, Code); 8] = [
    ("direct", "", "[1, 2][i]", Code::IndexOutOfRange),
    ("path", "", "i32(xs[i].gain)", Code::IndexOutOfRange),
    (
        "recursive argument",
        "",
        "i32(gs(xs[i], 2))",
        Code::IndexOutOfRange,
    ),
    (
        "inlined argument",
        "",
        "i32(nr(xs[i]))",
        Code::IndexOutOfRange,
    ),
    (
        "local let path",
        "    let e = (xs[i]).gain\n",
        "i32(e)",
        Code::IndexOutOfRange,
    ),
    (
        "local let index",
        "    let e = [1, 2][i]\n",
        "e",
        Code::IndexOutOfRange,
    ),
    ("fit", "", "[1, 2][u8(i + 300) - 300]", Code::DoesNotFit),
    (
        "local let fit",
        "    let e = [1, 2][u8(i + 300) - 300]\n",
        "e",
        Code::DoesNotFit,
    ),
];

fn arm_program(recursive: bool, (lets, arm): (&str, &str), guard: &str, reversed: bool) -> String {
    let (function, reader) = if recursive {
        (
            format!(
                "fn h(xs, i, n):\n{lets}    if n < 1 then (if {guard} then {arm} else 0) else h(xs, i, n - 1)\n"
            ),
            "Clip k:\n    Note/([60][h([verse, other], 5, 2)])\n",
        )
    } else {
        (
            format!("fn h(xs, i):\n{lets}    if {guard} then {arm} else 0\n"),
            "Clip k:\n    Note/([60][h([verse, other], 5)])\n",
        )
    };
    ordered(
        vec![
            INSTANCES.to_string(),
            "fn gs(c, n): if n < 1 then c.gain else gs(c, n - 1)\nfn nr(c): c.gain\n".to_string(),
            function,
            reader.to_string(),
        ],
        reversed,
    )
}

fn arm_position_rows(rows: &Rows) -> (Vec<String>, usize) {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    let mut programs = 0;
    for (p, (name, lets, arm, code)) in ARM_POSITIONS.iter().enumerate() {
        for (r, recursive) in [true, false].into_iter().enumerate() {
            for (o, reversed) in [false, true].into_iter().enumerate() {
                if !rows.take(&[p, r, o]) {
                    continue;
                }
                programs += 1;
                let text = arm_program(recursive, (lets, arm), "false", reversed);
                let twin = arm_program(recursive, (lets, arm), "true", reversed);
                let found = codes(&compiler, &text);
                let twin_found = codes(&compiler, &twin);
                if !found.is_empty() || twin_found != BTreeSet::from([*code]) {
                    failures.push(format!(
                        "346 {name}: {text}\n  draws {found:?}, its taken twin {twin_found:?}\n"
                    ));
                }
            }
        }
    }
    (failures, programs)
}

#[test]
fn an_untaken_arm_of_a_compile_time_choice_draws_no_index_range_or_fit_fault_at_any_position_in_a_body_or_its_twin()
 {
    assert_none_fail(covered(arm_position_rows), 16);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_an_untaken_arm_of_a_compile_time_choice_draws_no_index_range_or_fit_fault_at_any_position_in_a_body_or_its_twin()
 {
    assert_none_fail(arm_position_rows(&Rows::Every), 32);
}

#[test]
fn a_local_let_read_in_a_taken_arm_and_an_untaken_one_draws_its_fault_once_in_either_order() {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    for recursive in [true, false] {
        for reversed in [false, true] {
            let text = arm_program(
                recursive,
                ("    let e = (xs[i]).gain\n", "i32(e) + i32(e)"),
                "false",
                reversed,
            )
            .replace("else 0", "else i32(e)");
            let found: Vec<Code> = error_texts(&compiler, &text)
                .into_iter()
                .map(|(code, _)| code)
                .collect();
            assert_eq!(found, vec![Code::IndexOutOfRange], "346: {text}");
        }
    }
}
